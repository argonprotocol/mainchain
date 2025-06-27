import { defineConfig } from 'tsup';

export default defineConfig({
  entry: ['ts/index.ts'],
  dts: true,
  format: 'esm',
  target: 'esnext',
  clean: true,
  outDir: 'lib',
  skipNodeModulesBundle: true,
  shims: false,
  loader: {
    '.wasm': 'file',
  },
});
