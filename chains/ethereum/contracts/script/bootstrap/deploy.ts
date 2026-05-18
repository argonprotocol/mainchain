import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Interface, getAddress } from 'ethers';
import { network } from 'hardhat';

const contractsRoot = resolve(fileURLToPath(new URL('../..', import.meta.url)));
const repoRoot = resolve(contractsRoot, '..', '..', '..');
const ERC1967_ADMIN_SLOT = '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103';

const args = parseArgs(process.argv.slice(2));
const adminSafe = getRequiredAddress('admin-safe', 'ADMIN_SAFE_ADDRESS');
const guardianSafe = getRequiredAddress('guardian-safe', 'GUARDIAN_SAFE_ADDRESS');
const councilSigners = getRequiredAddressList('council-signers', 'COUNCIL_SIGNERS');
const councilWeights = getRequiredBigIntList('council-weights', 'COUNCIL_WEIGHTS');
const microgonsPerArgonot = getRequiredBigInt(
  'microgons-per-argonot',
  'MICROGONS_PER_ARGONOT',
);

async function main() {
  const connection = await network.create(args.network);
  try {
    const { ethers } = connection;
    const outputDir = resolve(
      repoRoot,
      args['output-dir'] ??
        `chains/ethereum/contracts/deployments/${connection.networkName}/bootstrap`,
    );

    const [deployer] = await ethers.getSigners();
    const connectedNetwork = await ethers.provider.getNetwork();

    if (councilSigners.length !== councilWeights.length || councilSigners.length === 0) {
      throw new Error('COUNCIL_SIGNERS and COUNCIL_WEIGHTS must be non-empty and have the same length');
    }
    const initialCouncil = councilSigners
      .map((signer, index) => ({ signer, weight: councilWeights[index]! }))
      .sort((left, right) => left.signer.toLowerCase().localeCompare(right.signer.toLowerCase()));
    const sortedCouncilSigners = initialCouncil.map(({ signer }) => signer);
    const sortedCouncilWeights = initialCouncil.map(({ weight }) => weight);

    let previousSigner = ethers.ZeroAddress;
    for (let index = 0; index < initialCouncil.length; index += 1) {
      const { signer, weight } = initialCouncil[index]!;
      if (
        signer === ethers.ZeroAddress ||
        weight === 0n ||
        signer.toLowerCase() === previousSigner.toLowerCase()
      ) {
        throw new Error(`Initial council entry ${index} must have a unique non-zero signer and non-zero weight`);
      }

      previousSigner = signer;
    }

    const councilMemberCount = BigInt(sortedCouncilSigners.length);
    const councilTotalWeight = sortedCouncilWeights.reduce((sum, weight) => sum + weight, 0n);
    const councilHash = ethers.keccak256(
      ethers.AbiCoder.defaultAbiCoder().encode(
        ['address[]', 'uint256[]'],
        [sortedCouncilSigners, sortedCouncilWeights],
      ),
    );

    const gatewayImplementationFactory = await ethers.getContractFactory('MintingGatewayV2');
    const gatewayBootstrapImplementation = await gatewayImplementationFactory.deploy(
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
      microgonsPerArgonot,
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

    const gatewayFinalImplementation = await gatewayImplementationFactory.deploy(
      await argonToken.getAddress(),
      await argonotToken.getAddress(),
    );
    await gatewayFinalImplementation.waitForDeployment();

    const proxyAdminInterface = new Interface([
      'function upgradeAndCall(address proxy, address implementation, bytes data)',
    ]);

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
        argonToken: buildContractRecord(await argonToken.getAddress(), argonToken, [
          gatewayProxyAddress,
        ]),
        argonotToken: buildContractRecord(await argonotToken.getAddress(), argonotToken, [
          gatewayProxyAddress,
        ]),
        mintingGatewayImplementationBootstrap: buildContractRecord(
          await gatewayBootstrapImplementation.getAddress(),
          gatewayBootstrapImplementation,
          [ethers.ZeroAddress, ethers.ZeroAddress],
        ),
        mintingGatewayImplementationFinal: buildContractRecord(
          await gatewayFinalImplementation.getAddress(),
          gatewayFinalImplementation,
          [await argonToken.getAddress(), await argonotToken.getAddress()],
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
        microgonsPerArgonot: microgonsPerArgonot.toString(),
        signers: sortedCouncilSigners,
        weights: sortedCouncilWeights.map((a) => a.toString()),
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
        notes: [
          'This branch does not yet include the runtime-side pallet_crosschain_transfer migration.',
          'Use these seeded Ethereum addresses as the manifest inputs once the runtime-side state exists.',
        ],
      },
      safeTransactions: [
        {
          description:
          'Upgrade MintingGatewayV2 proxy to final implementation with fixed canonical tokens',
          target: proxyAdminAddress,
          value: '0',
          data: proxyAdminInterface.encodeFunctionData('upgradeAndCall', [
            gatewayProxyAddress,
            await gatewayFinalImplementation.getAddress(),
            '0x',
          ]),
        },
      ],
      notes: [
        'Canonical tokens are deployed with the MintingGateway proxy address fixed in their constructors.',
        'The proxy is first deployed against a bootstrap MintingGatewayV2 implementation with zero canonical token immutables.',
        'That proxy initialization stores the first council summary.',
        'Token-bearing gateway entrypoints reject until the proxy is upgraded to the final implementation with canonical token addresses configured.',
        'After the token contracts exist, the admin Safe upgrades the proxy through ProxyAdmin to the final MintingGatewayV2 implementation that has the Argon and Argonot token addresses baked in as immutables.',
        'MintingGateway business ownership starts under the admin Safe.',
        'The guardian Safe can pause immediately, while unpause remains on the admin Safe owner path.',
        'The proxy admin is initially owned by the admin Safe.',
        'A later hardening step can transfer ProxyAdmin ownership to a TimelockController once the governance flow is ready.',
        'The migration balances still need a later admin Safe migrate(...) call on the final implementation.',
        'Long-term upgrades should happen through the stable gateway proxy address, not by rotating token trust.',
      ],
    };

    await mkdir(outputDir, { recursive: true });
    await writeFile(
      join(outputDir, 'deployment-manifest.json'),
      `${JSON.stringify(manifest, null, 2)}\n`,
    );
  } finally {
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
  ethers: Awaited<ReturnType<typeof network.create>>['ethers'],
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

function getRequiredAddressList(argKey: string, envKey: string) {
  const raw = args[argKey] ?? process.env[envKey];
  if (!raw) {
    throw new Error(`Missing required ${argKey} argument or ${envKey} environment variable`);
  }

  return raw
    .split(',')
    .map((a) => a.trim())
    .filter(Boolean)
    .map((a) => getAddress(a));
}

function getRequiredBigIntList(argKey: string, envKey: string) {
  const raw = args[argKey] ?? process.env[envKey];
  if (!raw) {
    throw new Error(`Missing required ${argKey} argument or ${envKey} environment variable`);
  }

  return raw
    .split(',')
    .map((a) => a.trim())
    .filter(Boolean)
    .map((a) => BigInt(a));
}

function getRequiredBigInt(argKey: string, envKey: string) {
  const raw = args[argKey] ?? process.env[envKey];
  if (!raw) {
    throw new Error(`Missing required ${argKey} argument or ${envKey} environment variable`);
  }

  return BigInt(raw);
}

function parseArgs(argv: string[]) {
  const parsed: Record<string, string> = {};

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith('--')) continue;

    const key = arg.slice(2);
    const next = argv[index + 1];

    if (!next || next.startsWith('--')) {
      parsed[key] = 'true';
      continue;
    }

    parsed[key] = next;
    index += 1;
  }

  return parsed;
}

void main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
