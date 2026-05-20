import { defineConfig } from 'tsup';
import { promises as fs } from 'node:fs';
import { resolve } from 'node:path';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: true,
  format: ['esm', 'cjs'],
  clean: true,
  splitting: false,
  outDir: 'lib',
  platform: 'node',
  target: 'node20',
  sourcemap: true,
  skipNodeModulesBundle: true,
  shims: true,
  onSuccess: async () => {
    const contractArtifacts = [
      ['artifacts/contracts/ArgonToken.sol/ArgonToken.json', 'ArgonToken.json'],
      ['artifacts/contracts/ArgonotToken.sol/ArgonotToken.json', 'ArgonotToken.json'],
      ['artifacts/contracts/MintingGatewayV2.sol/MintingGatewayV2.json', 'MintingGatewayV2.json'],
      ['artifacts/contracts/ProxyArtifacts.sol/ProxyAdmin.json', 'ProxyAdmin.json'],
      [
        'artifacts/contracts/ProxyArtifacts.sol/TransparentUpgradeableProxy.json',
        'TransparentUpgradeableProxy.json',
      ],
    ] as const;

    const contractsRoot = resolve('..', '..', 'chains', 'ethereum', 'contracts');
    const outputRoot = resolve('lib', 'ethereum-contracts');

    await fs.mkdir(outputRoot, { recursive: true });
    await fs.copyFile('../../dev.docker-compose.yml', 'lib/dev.docker-compose.yml');

    for (const [sourcePath, outputName] of contractArtifacts) {
      await fs.copyFile(resolve(contractsRoot, sourcePath), resolve(outputRoot, outputName));
    }
  },
});
