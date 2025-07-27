import { defineConfig } from 'tsup';
import { wasmLoader } from 'esbuild-plugin-wasm';
import * as trace_events from 'node:trace_events';

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
    esbuildPlugins: [
      wasmLoader({
        mode: 'deferred',
      }),
    ],
  },

  // Browser build with polyfills
  {
    entry: ['ts/index.ts'],
    format: ['esm'],
    outDir: 'browser',
    dts: true,
    platform: 'browser',
    target: 'esnext',
    sourcemap: true,
    clean: true,
    splitting: false,
    treeshake: true,
    esbuildPlugins: [
      wasmLoader({
        mode: 'embedded',
      }),
    ],
  },
]);
