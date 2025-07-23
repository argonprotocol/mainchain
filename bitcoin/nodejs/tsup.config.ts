import { defineConfig } from 'tsup';

export default defineConfig([
  // Node.js build
  {
    entry: ['ts/index.ts'],
    dts: true,
    format: 'esm',
    target: 'esnext',
    clean: true,
    outDir: 'lib',
    platform: 'node',
    skipNodeModulesBundle: true,
    shims: false,
    loader: {
      '.wasm': 'file',
    },
  },

  // Browser build with polyfills
  {
    entry: ['ts/index.ts'],
    format: ['esm'],
    outDir: 'browser',
    dts: true,
    platform: 'browser',
    target: 'es2020',
    sourcemap: true,
    clean: true,
    splitting: false,
    treeshake: true,
    loader: {
      '.wasm': 'file',
    },
  },
]);
