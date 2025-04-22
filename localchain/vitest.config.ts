import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    testTimeout: 120_000,
    hookTimeout: 120_000,
    retry: 2,
    disableConsoleIntercept: true,
    reporters: process.env.GITHUB_ACTIONS
      ? ['dot', 'github-actions', 'junit']
      : ['dot'],
    outputFile: {
      junit: 'vitest-results.xml',
    },
  },
});
