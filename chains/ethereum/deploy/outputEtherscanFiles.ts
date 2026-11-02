import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { AbiCoder, Interface } from 'ethers';
import { contractsRoot, repoRoot } from './src/hardhat.js';
import { getRequiredArg, parseArgs } from './src/cli.js';

const contractNames = {
  mintingGatewayImplementationBootstrap: 'MintingGateway',
  mintingGatewayImplementationFinal: 'MintingGateway',
  argonBuyAndTransfer: 'ArgonBuyAndTransfer',
  stabilizationPoolHook: 'StabilizationPoolHook',
  stabilizationPoolSubscriber: 'StabilizationPoolSubscriber',
  uniswapV4PriceQuoter: 'UniswapV4PriceQuoter',
  argonToken: 'ArgonToken',
  argonotToken: 'ArgonotToken',
  mintingGatewayProxy: 'TransparentUpgradeableProxy',
};

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const manifestPath = resolve(repoRoot, getRequiredArg(args, 'deployment-manifest'));
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as {
    contracts: Record<string, { address: string; constructorArgs: unknown[]; initCode?: string }>;
  };
  const outputDir = dirname(manifestPath);
  process.chdir(contractsRoot);
  const { artifacts } = await import('hardhat');
  await mkdir(outputDir, { recursive: true });

  for (const [key, name] of Object.entries(contractNames)) {
    const contract = manifest.contracts[key];
    if (!contract) continue;
    const artifact = await artifacts.readArtifact(name);
    const constructor = new Interface(artifact.abi).deploy;
    if (constructor.inputs.length !== contract.constructorArgs.length) {
      throw new Error(
        `${key} uses a different constructor revision. Export historical deployments from their original Git revision.`,
      );
    }
    const buildInfoPath =
      artifact.buildInfoId && (await artifacts.getBuildInfoPath(artifact.buildInfoId));
    if (!buildInfoPath)
      throw new Error(`Missing build info for ${name}; compile the contracts first`);
    const buildInfo = JSON.parse(await readFile(buildInfoPath, 'utf8')) as { input: unknown };
    const encoded = AbiCoder.defaultAbiCoder().encode(constructor.inputs, contract.constructorArgs);
    if (contract.initCode && contract.initCode !== `${artifact.bytecode}${encoded.slice(2)}`) {
      throw new Error(
        `${key} bytecode differs from its prepared deployment; use the original Git revision`,
      );
    }
    await writeFile(
      join(outputDir, `${key}.standard-input.json`),
      `${JSON.stringify(buildInfo.input, null, 2)}\n`,
    );
    await writeFile(join(outputDir, `${key}.constructor-args.txt`), `${encoded.slice(2)}\n`);
  }
}

void main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
