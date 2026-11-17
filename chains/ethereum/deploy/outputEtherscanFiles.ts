import { resolve } from 'node:path';
import { contractsRoot, repoRoot } from './src/hardhat.js';
import { getRequiredArg, parseArgs } from './src/cli.js';
import { outputEtherscanFiles } from './src/etherscan.js';

const args = parseArgs(process.argv.slice(2));
process.chdir(contractsRoot);
void outputEtherscanFiles(resolve(repoRoot, getRequiredArg(args, 'deployment-manifest'))).catch(
  error => {
    console.error(error);
    process.exitCode = 1;
  },
);
