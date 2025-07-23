import { defineConfig } from 'tsup';

export default defineConfig([
  {
    entry: ['src/index.ts', 'src/cli.ts', 'src/clis/index.ts'],
    dts: true,
    external: ['@polkadot/types/lookup', '@commander-js/extra-typings'],
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
    dts: true,
    platform: 'browser',
    target: 'es2020',
    sourcemap: true,
    clean: true,
    splitting: false,
    treeshake: true,
  },
]);
