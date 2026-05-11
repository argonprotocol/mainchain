import { defineConfig } from 'tsup';

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
  },
]);
