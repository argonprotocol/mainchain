import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, relative, resolve } from 'node:path';

import { Interface, getAddress } from 'ethers';
import { getOptionalArg, getRequiredArg, parseArgs, stringifyJson } from './src/cli.js';
import { deployBuyAndTransfer } from './src/purchase.js';
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

    const implementationSlot = '0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc';
    const previousImplementationAddress = getAddress(
      `0x${(await ethers.provider.getStorage(gatewayProxyAddress, implementationSlot)).slice(-40)}`,
    );

    let purchase;
    const purchaseConfigPath = getOptionalArg(args, 'purchase-config');
    if (purchaseConfigPath) {
      if (getOptionalArg(args, 'swap-contract'))
        throw new Error('Choose --purchase-config or --swap-contract');
      purchase = await deployBuyAndTransfer(
        ethers,
        gatewayProxyAddress,
        argonTokenAddress,
        resolve(repoRoot, purchaseConfigPath),
      );
    }
    const swapContractAddress =
      purchase?.address ?? getAddress(getRequiredArg(args, 'swap-contract'));
    if (swapContractAddress !== ethers.ZeroAddress) {
      const swap = await ethers.getContractAt('ArgonBuyAndTransfer', swapContractAddress);
      if (
        (await swap.getFunction('gateway')()) !== gatewayProxyAddress ||
        (await swap.getFunction('argon')()) !== argonTokenAddress
      ) {
        throw new Error('Purchase contract does not match the gateway and canonical ARGN');
      }
    }

    const gatewayFactory = await ethers.getContractFactory('MintingGateway');
    const gatewayImplementation = await gatewayFactory.deploy(
      argonTokenAddress,
      argonotTokenAddress,
      swapContractAddress,
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
        argonBuyAndTransfer: purchase,
        mintingGatewayImplementationFinal: {
          address: nextImplementationAddress,
          constructorArgs: [argonTokenAddress, argonotTokenAddress, swapContractAddress],
          deploymentTxHash: gatewayImplementation.deploymentTransaction()?.hash ?? null,
        },
      },
      safeTransactions: [
        {
          description: 'Upgrade MintingGateway with the explicitly selected purchase authority',
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
