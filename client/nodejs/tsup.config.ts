import { defineConfig } from 'tsup';
import { promises as fs } from 'node:fs';
import { resolve } from 'node:path';

const contractArtifacts = [
  'artifacts/contracts/ArgonToken.sol/ArgonToken.json',
  'artifacts/contracts/ArgonotToken.sol/ArgonotToken.json',
  'artifacts/contracts/MintingGateway.sol/MintingGateway.json',
  'artifacts/contracts/ProxyArtifacts.sol/ProxyAdmin.json',
  'artifacts/contracts/ProxyArtifacts.sol/TransparentUpgradeableProxy.json',
] as const;

async function copyContractArtifacts(outDir: 'lib' | 'browser') {
  const contractsRoot = resolve('..', '..', 'chains', 'ethereum', 'contracts');

  for (const artifactPath of contractArtifacts) {
    const outputPath = resolve(outDir, artifactPath);
    await fs.mkdir(resolve(outputPath, '..'), { recursive: true });
    await fs.copyFile(resolve(contractsRoot, artifactPath), outputPath);
  }
}

export default defineConfig([
  {
    entry: ['src/index.ts'],
    dts: {
      compilerOptions: {
        rootDir: '.',
      },
      resolve: ['@argonprotocol/ethereum-contracts'],
    },
    external: ['@polkadot/types/lookup'],
    noExternal: ['@argonprotocol/ethereum-contracts'],
    format: ['esm', 'cjs'],
    clean: true,
    splitting: true,
    outDir: 'lib',
    platform: 'node',
    target: 'node20',
    sourcemap: true,
    skipNodeModulesBundle: true,
    shims: true,
    onSuccess: async () => {
      await copyContractArtifacts('lib');
    },
  },

  {
    entry: ['src/index.ts'], // only the library API
    format: ['esm'], // browser-friendly
    outDir: 'browser',
    external: ['@polkadot/types/lookup'],
    dts: {
      compilerOptions: {
        rootDir: '.',
      },
      resolve: ['@argonprotocol/ethereum-contracts'],
    },
    noExternal: ['@argonprotocol/ethereum-contracts'],
    platform: 'browser',
    target: 'es2020',
    sourcemap: true,
    clean: true,
    splitting: false,
    treeshake: true,
    onSuccess: async () => {
      await copyContractArtifacts('browser');
    },
  },
]);
