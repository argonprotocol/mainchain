import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

import { Interface, getAddress } from 'ethers';
import { getClient } from '@argonprotocol/mainchain';
import { contractsRoot, repoRoot } from './src/hardhat.js';
import { parseArgs } from './src/cli.js';
import { deployBuyAndTransfer } from './src/purchase.js';
import {
  collectVaultOperatorsByEffectiveCouncilSigner,
  deriveRuntimeCouncilSnapshot,
} from './src/council.js';

const ERC1967_ADMIN_SLOT = '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103';
const MIN_BOOTSTRAP_COUNCIL_BONDS = 5_000;

const args = parseArgs(process.argv.slice(2));
const argonRpcUrl = getRequiredArg('argon-rpc-url', 'ARGON_RPC_URL');
const adminSafe = getRequiredAddress('admin-safe', 'ADMIN_SAFE_ADDRESS');
const guardianSafe = getRequiredAddress('guardian-safe', 'GUARDIAN_SAFE_ADDRESS');

async function main() {
  const argonClient = await getClient(argonRpcUrl);
  process.chdir(contractsRoot);
  const { network } = await import('hardhat');
  const connection = await network.create(args.network);
  try {
    const registeredCouncilMembers = [
      ...(await collectVaultOperatorsByEffectiveCouncilSigner(argonClient)).values(),
    ];
    const initialCouncilMembers = (
      await Promise.all(
        registeredCouncilMembers.map(async member => {
          const bondState = await argonClient.query.treasury.bondLotsByVault(member.vaultId);
          const activeBonds =
            bondState.regularBonds.toNumber() + bondState.flexibleBonds.toNumber();
          return activeBonds > MIN_BOOTSTRAP_COUNCIL_BONDS ? member : undefined;
        }),
      )
    ).filter(member => member !== undefined);
    if (!initialCouncilMembers.length) {
      throw new Error(
        'No vault operators with a pre-registered effective Ethereum council signer and more than 5,000 treasury bonds were found on the target Argon runtime',
      );
    }

    const initialCouncil = await deriveRuntimeCouncilSnapshot(
      argonClient,
      initialCouncilMembers.map(member => member.accountId),
    );
    const { ethers } = connection;
    const networkName = connection.networkName;
    const outputDir = resolve(
      repoRoot,
      args['output-dir'] ?? `chains/ethereum/deploy/${networkName}`,
    );
    const [deployer] = await ethers.getSigners();
    const connectedNetwork = await ethers.provider.getNetwork();
    const sortedCouncilSigners = initialCouncil.members.map(({ signer }) => signer);
    const sortedCouncilWeights = initialCouncil.members.map(({ weight }) => weight);

    let previousSigner = ethers.ZeroAddress;
    for (let index = 0; index < initialCouncil.members.length; index += 1) {
      const { signer, weight } = initialCouncil.members[index];
      if (
        signer === ethers.ZeroAddress ||
        weight === 0n ||
        signer.toLowerCase() === previousSigner.toLowerCase()
      ) {
        throw new Error(
          `Initial council entry ${index} must have a unique non-zero signer and non-zero weight`,
        );
      }

      previousSigner = signer;
    }

    const councilMemberCount = BigInt(sortedCouncilSigners.length);
    const councilTotalWeight = initialCouncil.totalWeight;
    const epochMicrogonsPerArgonot = initialCouncil.epochMicrogonsPerArgonot;
    const councilHash = ethers.keccak256(
      ethers.AbiCoder.defaultAbiCoder().encode(
        ['address[]', 'uint256[]', 'uint128'],
        [sortedCouncilSigners, sortedCouncilWeights, epochMicrogonsPerArgonot],
      ),
    );

    const gatewayImplementationFactory = await ethers.getContractFactory('MintingGateway');
    const gatewayBootstrapImplementation = await gatewayImplementationFactory.deploy(
      ethers.ZeroAddress,
      ethers.ZeroAddress,
      ethers.ZeroAddress,
    );
    await gatewayBootstrapImplementation.waitForDeployment();

    const initializeData = gatewayImplementationFactory.interface.encodeFunctionData('initialize', [
      adminSafe,
      guardianSafe,
      councilHash,
      councilMemberCount,
      councilTotalWeight,
      epochMicrogonsPerArgonot,
    ]);

    const gatewayProxyFactory = await ethers.getContractFactory('TransparentUpgradeableProxy');
    const gatewayProxy = await gatewayProxyFactory.deploy(
      await gatewayBootstrapImplementation.getAddress(),
      adminSafe,
      initializeData,
    );
    await gatewayProxy.waitForDeployment();

    const gatewayProxyAddress = await gatewayProxy.getAddress();
    const proxyAdminAddress = await getProxyAdminAddress(ethers, gatewayProxyAddress);

    const argonFactory = await ethers.getContractFactory('ArgonToken');
    const argonToken = await argonFactory.deploy(gatewayProxyAddress);
    await argonToken.waitForDeployment();

    const argonotFactory = await ethers.getContractFactory('ArgonotToken');
    const argonotToken = await argonotFactory.deploy(gatewayProxyAddress);
    await argonotToken.waitForDeployment();

    let purchase;
    if (args['purchase-config']) {
      purchase = await deployBuyAndTransfer(
        ethers,
        gatewayProxyAddress,
        await argonToken.getAddress(),
        resolve(repoRoot, args['purchase-config']),
      );
    }
    const swapContractAddress = purchase?.address ?? ethers.ZeroAddress;

    const gatewayFinalImplementation = await gatewayImplementationFactory.deploy(
      await argonToken.getAddress(),
      await argonotToken.getAddress(),
      swapContractAddress,
    );
    await gatewayFinalImplementation.waitForDeployment();

    const proxyAdminInterface = new Interface([
      'function upgradeAndCall(address proxy, address implementation, bytes data)',
    ]);
    const safeTransactions = [
      {
        description:
          'Upgrade MintingGateway proxy to final implementation with fixed canonical tokens',
        target: proxyAdminAddress,
        value: '0',
        data: proxyAdminInterface.encodeFunctionData('upgradeAndCall', [
          gatewayProxyAddress,
          await gatewayFinalImplementation.getAddress(),
          '0x',
        ]),
      },
    ];

    const manifest = {
      generatedAt: new Date().toISOString(),
      deployer: deployer.address,
      network: {
        name: connection.networkName,
        chainId: connectedNetwork.chainId.toString(),
      },
      adminSafe,
      guardianSafe,
      contracts: {
        argonBuyAndTransfer: purchase,
        argonToken: buildContractRecord(await argonToken.getAddress(), argonToken, [
          gatewayProxyAddress,
        ]),
        argonotToken: buildContractRecord(await argonotToken.getAddress(), argonotToken, [
          gatewayProxyAddress,
        ]),
        mintingGatewayImplementationBootstrap: buildContractRecord(
          await gatewayBootstrapImplementation.getAddress(),
          gatewayBootstrapImplementation,
          [ethers.ZeroAddress, ethers.ZeroAddress, ethers.ZeroAddress],
        ),
        mintingGatewayImplementationFinal: buildContractRecord(
          await gatewayFinalImplementation.getAddress(),
          gatewayFinalImplementation,
          [await argonToken.getAddress(), await argonotToken.getAddress(), swapContractAddress],
        ),
        mintingGatewayProxy: buildContractRecord(gatewayProxyAddress, gatewayProxy, [
          await gatewayBootstrapImplementation.getAddress(),
          adminSafe,
          initializeData,
        ]),
        mintingGatewayProxyAdmin: {
          address: proxyAdminAddress,
          owner: adminSafe,
          deploymentTxHash: gatewayProxy.deploymentTransaction()?.hash ?? null,
        },
        mintingGateway: buildContractRecord(gatewayProxyAddress, gatewayProxy, [
          await gatewayBootstrapImplementation.getAddress(),
          adminSafe,
          initializeData,
        ]),
      },
      initialCouncil: {
        councilHash,
        memberCount: councilMemberCount.toString(),
        totalWeight: councilTotalWeight.toString(),
        epochMicrogonsPerArgonot: epochMicrogonsPerArgonot.toString(),
        accounts: initialCouncil.members.map(({ accountId }) => accountId),
        signers: sortedCouncilSigners,
        weights: sortedCouncilWeights.map(a => a.toString()),
        weightSource: 'runtimeCouncilSnapshot',
      },
      runtimeBootstrap: {
        externalChain: {
          chainId: connectedNetwork.chainId.toString(),
          gatewayAddress: gatewayProxyAddress,
        },
        externalChainAssets: [
          buildRuntimeAssetRecord('ARGN', await argonToken.getAddress(), gatewayProxyAddress),
          buildRuntimeAssetRecord('ARGNOT', await argonotToken.getAddress(), gatewayProxyAddress),
        ],
      },
      safeTransactions,
      notes: [],
    };

    await mkdir(outputDir, { recursive: true });
    await writeFile(
      join(outputDir, 'deployment-manifest.json'),
      `${JSON.stringify(manifest, null, 2)}\n`,
    );
  } finally {
    await argonClient.disconnect().catch(() => undefined);
    await connection.close();
  }
}

function buildContractRecord(
  address: string,
  contract: { deploymentTransaction(): { hash: string } | null },
  constructorArgs: unknown[],
) {
  return {
    address,
    constructorArgs,
    deploymentTxHash: contract.deploymentTransaction()?.hash ?? null,
  };
}

function buildRuntimeAssetRecord(symbol: string, tokenAddress: string, gatewayAddress: string) {
  return {
    symbol,
    tokenAddress,
    gatewayAddress,
    decimals: 18,
    runtimeDecimals: 6,
    enabled: true,
  };
}

async function getProxyAdminAddress(
  ethers: {
    provider: {
      getStorage(address: string, position: string): Promise<string>;
    };
  },
  proxyAddress: string,
) {
  const raw = await ethers.provider.getStorage(proxyAddress, ERC1967_ADMIN_SLOT);
  return getAddress(`0x${raw.slice(-40)}`);
}

function getRequiredAddress(argKey: string, envKey: string) {
  const raw = args[argKey] ?? process.env[envKey];
  if (!raw) {
    throw new Error(`Missing required ${argKey} argument or ${envKey} environment variable`);
  }

  return getAddress(raw);
}

function getRequiredArg(argKey: string, envKey: string) {
  const raw = args[argKey] ?? process.env[envKey];
  if (!raw) {
    throw new Error(`Missing required ${argKey} argument or ${envKey} environment variable`);
  }

  return raw.trim();
}

void main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
