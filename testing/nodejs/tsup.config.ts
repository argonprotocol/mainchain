import { defineConfig } from 'tsup';
import { promises as fs } from 'node:fs';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: {
    compilerOptions: {
      paths: {
        '@argonprotocol/mainchain': ['../../client/nodejs/src/index.ts'],
        '@polkadot/api/augment': ['../../client/nodejs/src/interfaces/augment-api.ts'],
        '@polkadot/types/lookup': ['../../client/nodejs/src/interfaces/types-lookup.ts'],
        '@polkadot/types/augment': ['../../client/nodejs/src/interfaces/augment-types.ts'],
        '@argonprotocol/mainchain/interfaces/*': [
          '../../client/nodejs/src/interfaces/*/index.ts',
          '../../client/nodejs/src/interfaces/*.ts',
        ],
      },
      module: 'esnext',
      resolveJsonModule: true,
      rootDir: '../..',
    },
  },
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
    await fs.copyFile('../../dev.docker-compose.yml', 'lib/dev.docker-compose.yml');
  },
});
