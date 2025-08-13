import { defineConfig } from 'tsup';
import { promises as fs } from 'node:fs';

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
    await fs.copyFile('../../docker-compose.yml', 'lib/docker-compose.yml');
  },
});
