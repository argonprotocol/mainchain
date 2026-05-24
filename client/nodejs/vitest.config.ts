import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  resolve: {
    alias: {
      '@argonprotocol/mainchain': fileURLToPath(new URL('./src/index.ts', import.meta.url)),
      '@argonprotocol/testing': fileURLToPath(
        new URL('../../testing/nodejs/src/index.ts', import.meta.url),
      ),
    },
  },
  test: {
    include: ['src/__test__/**/*.test.ts'],
  },
});
