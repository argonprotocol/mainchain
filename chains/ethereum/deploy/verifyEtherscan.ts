import { resolve } from 'node:path';
import { contractsRoot, repoRoot } from './src/hardhat.js';
import { getRequiredArg, parseArgs } from './src/cli.js';
import { verifyEtherscanContracts } from './src/etherscan.js';

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const apiKey = process.env.ETHERSCAN_API_KEY?.trim();
  if (!apiKey) throw new Error('Set ETHERSCAN_API_KEY; no wallet signing key is needed');
  process.chdir(contractsRoot);
  const report = await verifyEtherscanContracts(
    resolve(repoRoot, getRequiredArg(args, 'deployment-manifest')),
    apiKey,
  );
  if (Object.values(report.contracts).some(record => record.status !== 'verified')) {
    process.exitCode = 1;
  }
}

void main().catch(error => {
  let message = String(error);
  const apiKey = process.env.ETHERSCAN_API_KEY;
  if (apiKey) message = message.replaceAll(apiKey, '<REDACTED>');
  console.error(message);
  process.exitCode = 1;
});
