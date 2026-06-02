import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { AbiCoder } from 'ethers';
import { contractsRoot, deployRoot } from './src/hardhat.js';

const abiCoder = AbiCoder.defaultAbiCoder();

const fileSpecs = [
  {
    contractKey: 'mintingGatewayImplementationBootstrap',
    contractName: 'project/contracts/MintingGateway.sol:MintingGateway',
    constructorTypes: ['address', 'address'],
  },
  {
    contractKey: 'mintingGatewayImplementationFinal',
    contractName: 'project/contracts/MintingGateway.sol:MintingGateway',
    constructorTypes: ['address', 'address'],
  },
  {
    contractKey: 'argonToken',
    contractName: 'project/contracts/ArgonToken.sol:ArgonToken',
    constructorTypes: ['address'],
  },
  {
    contractKey: 'argonotToken',
    contractName: 'project/contracts/ArgonotToken.sol:ArgonotToken',
    constructorTypes: ['address'],
  },
  {
    contractKey: 'mintingGatewayProxy',
    contractName: 'project/contracts/ProxyArtifacts.sol:TransparentUpgradeableProxy',
    constructorTypes: ['address', 'address', 'bytes'],
  },
] as const;

async function main() {
  const buildInfoDir = join(contractsRoot, 'artifacts/build-info');
  const buildInfos = await Promise.all(
    (await readdir(buildInfoDir))
      .filter(file => file.endsWith('.json') && !file.endsWith('.output.json'))
      .map(async file => {
        const buildInfoPath = join(buildInfoDir, file);
        const outputPath = join(buildInfoDir, file.replace(/\.json$/, '.output.json'));
        const buildInfo = await readJson(buildInfoPath);
        const output = buildInfo.output ? buildInfo : await readJson(outputPath);

        return {
          input: buildInfo.input,
          output: output.output,
        };
      }),
  );
  const buildInfoByContractName: Record<string, BuildInfo | undefined> = {};

  for (const spec of fileSpecs) {
    if (buildInfoByContractName[spec.contractName]) continue;

    const [sourceName, contractName] = spec.contractName.split(':');
    const buildInfo = buildInfos.find(
      info => sourceName && contractName && info.output?.contracts?.[sourceName]?.[contractName],
    );

    if (!buildInfo) {
      throw new Error(`Missing build-info for ${spec.contractName}`);
    }

    buildInfoByContractName[spec.contractName] = buildInfo;
  }

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
      const buildInfo = buildInfoByContractName[spec.contractName];
      if (!contract?.address || !contract.deploymentTxHash) {
        throw new Error(`Missing deployment data for ${dirent.name}:${spec.contractKey}`);
      }
      if (!buildInfo) {
        throw new Error(`Missing build-info for ${spec.contractName}`);
      }

      const standardInputFile = join(outputDir, `${spec.contractKey}.standard-input.json`);
      const constructorArgsFile = join(outputDir, `${spec.contractKey}.constructor-args.txt`);
      const constructorArgsHex = abiCoder
        .encode([...spec.constructorTypes], contract.constructorArgs ?? [])
        .slice(2);

      await writeFile(
        standardInputFile,
        `${JSON.stringify(buildInfo.input, null, 2)}\n`,
      );
      await writeFile(constructorArgsFile, `${constructorArgsHex}\n`);
    }
  }
}

void main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});

type BuildInfo = {
  input: unknown;
  output?: {
    contracts?: Record<string, Record<string, unknown>>;
  };
};

async function readJson(path: string) {
  return JSON.parse(await readFile(path, 'utf8')) as BuildInfo & {
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
