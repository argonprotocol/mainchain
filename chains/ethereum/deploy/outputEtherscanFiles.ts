import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { AbiCoder } from 'ethers';
import { contractsRoot, deployRoot } from './src/hardhat.js';

const abiCoder = AbiCoder.defaultAbiCoder();
const buildInfoFiles = {
  gateway: 'solc-0_8_24-99a644eb68baa9f4a7d2f893d75a5ed27a4b1833.json',
  main: 'solc-0_8_24-c97fc17823f46ca2bbf09a524bd4ff9df2054ce8.json',
} as const;

const fileSpecs = [
  {
    contractKey: 'mintingGatewayImplementationBootstrap',
    contractName: 'project/contracts/MintingGateway.sol:MintingGateway',
    buildInfoKey: 'gateway',
    constructorTypes: ['address', 'address'],
  },
  {
    contractKey: 'mintingGatewayImplementationFinal',
    contractName: 'project/contracts/MintingGateway.sol:MintingGateway',
    buildInfoKey: 'gateway',
    constructorTypes: ['address', 'address'],
  },
  {
    contractKey: 'argonToken',
    contractName: 'project/contracts/ArgonToken.sol:ArgonToken',
    buildInfoKey: 'main',
    constructorTypes: ['address'],
  },
  {
    contractKey: 'argonotToken',
    contractName: 'project/contracts/ArgonotToken.sol:ArgonotToken',
    buildInfoKey: 'main',
    constructorTypes: ['address'],
  },
  {
    contractKey: 'mintingGatewayProxy',
    contractName: 'project/contracts/ProxyArtifacts.sol:TransparentUpgradeableProxy',
    buildInfoKey: 'main',
    constructorTypes: ['address', 'address', 'bytes'],
  },
] as const;

async function main() {
  const buildInfo = {
    gateway: await readJson(join(contractsRoot, `artifacts/build-info/${buildInfoFiles.gateway}`)),
    main: await readJson(join(contractsRoot, `artifacts/build-info/${buildInfoFiles.main}`)),
  };

  for (const dirent of await readdir(deployRoot, { withFileTypes: true })) {
    if (!dirent.isDirectory()) continue;

    const outputDir = join(deployRoot, dirent.name);
    const manifestPath = join(outputDir, 'deployment-manifest.json');

    let manifest: {
      contracts?: Record<
        string,
        {
          address?: string;
          constructorArgs?: unknown[];
          deploymentTxHash?: string | null;
        }
      >;
    };

    try {
      manifest = await readJson(manifestPath);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === 'ENOENT') continue;
      throw error;
    }

    const contracts = manifest.contracts ?? {};
    await mkdir(outputDir, { recursive: true });

    for (const spec of fileSpecs) {
      const contract = contracts[spec.contractKey];
      if (!contract?.address || !contract.deploymentTxHash) {
        throw new Error(`Missing deployment data for ${dirent.name}:${spec.contractKey}`);
      }

      const standardInputFile = join(outputDir, `${spec.contractKey}.standard-input.json`);
      const constructorArgsFile = join(outputDir, `${spec.contractKey}.constructor-args.txt`);
      const constructorArgsHex = abiCoder
        .encode(spec.constructorTypes as string[], contract.constructorArgs ?? [])
        .slice(2);

      await writeFile(
        standardInputFile,
        `${JSON.stringify(buildInfo[spec.buildInfoKey].input, null, 2)}\n`,
      );
      await writeFile(constructorArgsFile, `${constructorArgsHex}\n`);
    }
  }
}

void main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});

async function readJson(path: string) {
  return JSON.parse(await readFile(path, 'utf8')) as {
    input: unknown;
    solcLongVersion?: string;
    contracts?: Record<
      string,
      {
        address?: string;
        constructorArgs?: unknown[];
        deploymentTxHash?: string | null;
      }
    >;
  };
}
