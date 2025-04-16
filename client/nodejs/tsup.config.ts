import { defineConfig } from 'tsup';

export default defineConfig({
  entry: [
    'src/index.ts',
    'src/test-utils/index.ts',
    'src/cli.ts',
    'src/clis/index.ts',
  ],
  dts: true,
  external: ['@polkadot/types/lookup', '@commander-js/extra-typings'],
  format: ['esm', 'cjs'],
  clean: true,
  splitting: false,
  outDir: 'lib',
  platform: 'node',
  target: 'node20',
  sourcemap: true,
  skipNodeModulesBundle: true,
  shims: true,
});
