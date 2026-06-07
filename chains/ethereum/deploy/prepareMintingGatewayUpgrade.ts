import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, relative, resolve } from 'node:path';

import { Interface, getAddress } from 'ethers';
import { getOptionalArg, parseArgs, stringifyJson } from './src/cli.js';
import { contractsRoot, repoRoot } from './src/hardhat.js';

const args = parseArgs(process.argv.slice(2));
const networkName = getOptionalArg(args, 'network') ?? 'testnet';
const manifestPath = resolve(
  repoRoot,
  getOptionalArg(args, 'deployment-manifest') ??
    `chains/ethereum/deploy/${networkName}/deployment-manifest.json`,
);
const outputPath = resolve(
  repoRoot,
  getOptionalArg(args, 'output-file') ??
    `chains/ethereum/deploy/${networkName}/minting-gateway-upgrade-manifest.json`,
);

async function main() {
  process.chdir(contractsRoot);
  const manifest = await readJson(manifestPath);
  const gatewayProxyAddress = getRequiredContractAddress(manifest, 'mintingGateway');
  const proxyAdminAddress = getRequiredContractAddress(manifest, 'mintingGatewayProxyAdmin');
  const argonTokenAddress = getRequiredContractAddress(manifest, 'argonToken');
  const argonotTokenAddress = getRequiredContractAddress(manifest, 'argonotToken');
  const previousImplementationAddress =
    manifest.contracts?.mintingGatewayImplementationFinal?.address ?? null;
  const { network } = await import('hardhat');
  const connection = await network.create(networkName);

  try {
    const { ethers } = connection;
    const [deployer] = await ethers.getSigners();
    const connectedNetwork = await ethers.provider.getNetwork();

    if (
      manifest.network?.chainId &&
      BigInt(manifest.network.chainId) !== connectedNetwork.chainId
    ) {
      throw new Error(
        `Deployment manifest chainId ${manifest.network.chainId} does not match connected chainId ${connectedNetwork.chainId}`,
      );
    }

    const gatewayFactory = await ethers.getContractFactory('MintingGateway');
    const gatewayImplementation = await gatewayFactory.deploy(
      argonTokenAddress,
      argonotTokenAddress,
    );
    await gatewayImplementation.waitForDeployment();

    const proxyAdminInterface = new Interface([
      'function upgradeAndCall(address proxy, address implementation, bytes data)',
    ]);
    const nextImplementationAddress = await gatewayImplementation.getAddress();
    const upgradeManifest = {
      generatedAt: new Date().toISOString(),
      network: {
        name: connection.networkName,
        chainId: connectedNetwork.chainId.toString(),
      },
      deployer: deployer.address,
      sourceManifestPath: relative(repoRoot, manifestPath),
      target: {
        gatewayProxyAddress,
        proxyAdminAddress,
        argonTokenAddress,
        argonotTokenAddress,
        previousImplementationAddress,
      },
      contracts: {
        mintingGatewayImplementationFinal: {
          address: nextImplementationAddress,
          constructorArgs: [argonTokenAddress, argonotTokenAddress],
          deploymentTxHash: gatewayImplementation.deploymentTransaction()?.hash ?? null,
        },
      },
      safeTransactions: [
        {
          description: 'Upgrade MintingGateway proxy to the activity-root implementation',
          target: proxyAdminAddress,
          value: '0',
          data: proxyAdminInterface.encodeFunctionData('upgradeAndCall', [
            gatewayProxyAddress,
            nextImplementationAddress,
            '0x',
          ]),
        },
      ],
      notes: [],
    };

    await mkdir(dirname(outputPath), { recursive: true });
    await writeFile(outputPath, `${stringifyJson(upgradeManifest)}\n`);
  } finally {
    await connection.close();
  }
}

void main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});

type DeploymentManifest = {
  network?: {
    name?: string;
    chainId?: string;
  };
  contracts?: Record<
    string,
    {
      address?: string;
    }
  >;
};

function getRequiredContractAddress(manifest: DeploymentManifest, contractKey: string) {
  const address = manifest.contracts?.[contractKey]?.address;
  if (!address) {
    throw new Error(`Deployment manifest is missing contracts.${contractKey}.address`);
  }

  return getAddress(address);
}

async function readJson(path: string) {
  return JSON.parse(await readFile(path, 'utf8')) as DeploymentManifest;
}
