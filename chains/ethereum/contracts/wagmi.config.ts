import { defineConfig } from '@wagmi/cli';
import { hardhat } from '@wagmi/cli/plugins';

export default defineConfig({
  out: 'generated.ts',
  plugins: [
    hardhat({
      project: '.',
    }),
  ],
});
