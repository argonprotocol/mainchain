import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

import { Interface, getAddress } from 'ethers';
import { getClient } from '@argonprotocol/mainchain';
import { contractsRoot, repoRoot } from './src/hardhat.js';
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

type MigrationDistribution = {
  sourceBatches: string[];
  recipientCount: number;
  totalAmountRaw: string;
  recipients: string[];
  amounts: string[];
};

type DeploymentMigrationBundle = {
  copiedFrom: {
    repo: string;
    migrationFiles: Array<{
      path: string;
      sha256: string;
    }>;
    finalBalanceSourceHashes: Array<{
      path: string;
      sha256: string;
    }>;
    poolAllocationHash: {
      path: string;
      sha256: string;
    };
  };
  argonMigration: MigrationDistribution;
  argonotMigration: MigrationDistribution;
};

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
          const acceptedBondLots = await argonClient.query.treasury.bondLotsByVault(member.vaultId);
          const activeBonds = acceptedBondLots.reduce(
            (sum, bondLot) => sum + bondLot.bonds.toNumber(),
            0,
          );
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
    const migrationBundlePath = resolve(
      repoRoot,
      `chains/ethereum/deploy/${networkName}/migration/migrate-bundle.json`,
    );
    const migrationBundle = await loadMigrationBundle(migrationBundlePath);

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

    const gatewayFinalImplementation = await gatewayImplementationFactory.deploy(
      await argonToken.getAddress(),
      await argonotToken.getAddress(),
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

    if (migrationBundle) {
      safeTransactions.push({
        description: 'Run MintingGateway migrate(...) from the checked-in final mainnet bundle',
        target: gatewayProxyAddress,
        value: '0',
        data: gatewayImplementationFactory.interface.encodeFunctionData('migrate', [
          {
            recipients: migrationBundle.argonMigration.recipients,
            amounts: migrationBundle.argonMigration.amounts,
          },
          {
            recipients: migrationBundle.argonotMigration.recipients,
            amounts: migrationBundle.argonotMigration.amounts,
          },
        ]),
      });
    }

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
        notes: [
          'Use these seeded Ethereum addresses as the manifest inputs for bootstrap:prepare-runtime-setup.',
          'The Argon-side legacy balance and refund migration now lives in pallet_crosschain_transfer.',
        ],
      },
      migration: migrationBundle && {
        bundlePath: `chains/ethereum/deploy/${networkName}/migration/migrate-bundle.json`,
        sourceRepo: migrationBundle.copiedFrom.repo,
        migrationFileHashes: migrationBundle.copiedFrom.migrationFiles,
        finalBalanceSourceHashes: migrationBundle.copiedFrom.finalBalanceSourceHashes,
        poolAllocationHash: migrationBundle.copiedFrom.poolAllocationHash,
        argon: {
          sourceBatches: migrationBundle.argonMigration.sourceBatches,
          recipientCount: migrationBundle.argonMigration.recipientCount,
          totalAmountRaw: migrationBundle.argonMigration.totalAmountRaw,
        },
        argonot: {
          sourceBatches: migrationBundle.argonotMigration.sourceBatches,
          recipientCount: migrationBundle.argonotMigration.recipientCount,
          totalAmountRaw: migrationBundle.argonotMigration.totalAmountRaw,
        },
      },
      safeTransactions,
      notes: [
        'Canonical tokens are deployed with the MintingGateway proxy address fixed in their constructors.',
        'The proxy is first deployed against a bootstrap MintingGateway implementation with zero canonical token immutables.',
        'That proxy initialization stores the first council summary derived from the target Argon runtime council inputs.',
        'Token-bearing gateway entrypoints reject until the proxy is upgraded to the final implementation with canonical token addresses configured.',
        'After the token contracts exist, the admin Safe upgrades the proxy through ProxyAdmin to the final MintingGateway implementation that has the Argon and Argonot token addresses baked in as immutables.',
        'MintingGateway business ownership starts under the admin Safe.',
        'The guardian Safe can pause immediately, while unpause remains on the admin Safe owner path.',
        'The proxy admin is initially owned by the admin Safe.',
        'A later hardening step can transfer ProxyAdmin ownership to a TimelockController once the governance flow is ready.',
        migrationBundle
          ? 'The manifest already includes the admin Safe migrate(...) call derived from the checked-in final migration bundle for this network.'
          : 'No checked-in migration bundle was found for this network, so migrate(...) still needs to be prepared separately.',
        'Long-term upgrades should happen through the stable gateway proxy address, not by rotating token trust.',
      ],
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

async function loadMigrationBundle(path: string): Promise<DeploymentMigrationBundle | null> {
  try {
    const raw = await readFile(path, 'utf8');
    const bundle = JSON.parse(raw) as DeploymentMigrationBundle;

    validateMigrationDistribution('Argon', bundle.argonMigration);
    validateMigrationDistribution('Argonot', bundle.argonotMigration);

    return bundle;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return null;
    }

    throw error;
  }
}

function validateMigrationDistribution(label: string, distribution: MigrationDistribution) {
  if (distribution.recipients.length !== distribution.amounts.length) {
    throw new Error(
      `${label} migration bundle has ${distribution.recipients.length} recipients but ${distribution.amounts.length} amounts`,
    );
  }

  if (distribution.recipientCount !== distribution.recipients.length) {
    throw new Error(
      `${label} migration bundle recipientCount ${distribution.recipientCount} does not match ${distribution.recipients.length} recipients`,
    );
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
