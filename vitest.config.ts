import { defineConfig } from 'vitest/config';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  test: {
    retry: 0,
    reporters: process.env.GITHUB_ACTIONS ? ['dot', 'github-actions'] : ['dot'],
    maxWorkers: 1,
    disableConsoleIntercept: true,
    projects: [
      {
        test: {
          testTimeout: 30_000,
          hookTimeout: 30_000,
          include: ['bitcoin/nodejs/ts/__test__/**/*.test.ts'],
        },
        plugins: [wasm()],
      },
      {
        test: {
          testTimeout: 120_000,
          hookTimeout: 120_000,
          retry: 2,
          include: ['localchain/__test__/**/*.test.ts'],
        },
      },
      {
        test: {
          testTimeout: 120_000,
          hookTimeout: 120_000,
          retry: 0,
          maxConcurrency: 1,
          include: ['client/nodejs/src/__test__/**/*.test.ts'],
        },
      },
    ],
  },
});
