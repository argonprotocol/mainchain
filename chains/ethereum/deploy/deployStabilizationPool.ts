import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { keccak256, toUtf8Bytes } from 'ethers';
import { getClient } from '@argonprotocol/mainchain';
import { getRequiredArg, parseArgs, stringifyJson } from './src/cli.js';
import { contractsRoot, repoRoot } from './src/hardhat.js';
import {
  prepareStabilizationPool,
  publishStabilizationPool,
  type StabilizationPoolConfig,
  type StabilizationPoolDeployment,
} from './src/stabilization.js';

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const networkName = getRequiredArg(args, 'network');
  const configPath = resolve(repoRoot, getRequiredArg(args, 'pool-config'));
  const outputPath = resolve(repoRoot, getRequiredArg(args, 'output-file'));
  const config = JSON.parse(await readFile(configPath, 'utf8')) as StabilizationPoolConfig;

  process.chdir(contractsRoot);
  const { network } = await import('hardhat');
  const publishing = args.publish === 'true';
  const connection = await network.create({
    network: networkName,
    override: publishing ? {} : { accounts: [] },
  });
  try {
    const plan = await prepareStabilizationPool(connection.ethers, config);
    const planHash = keccak256(toUtf8Bytes(stringifyJson(plan)));
    let transactions: Record<string, string> = {};
    try {
      const previous = JSON.parse(await readFile(outputPath, 'utf8'));
      if (previous.planHash !== planHash) {
        throw new Error(
          'Existing manifest describes a different deployment; select a new output file',
        );
      }
      transactions = previous.transactions;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    }

    const poolConfig = {
      positionManager: plan.pool.positionManager,
      subscriber: plan.contracts.stabilizationPoolSubscriber.address,
      key: plan.pool.key,
    };
    let registrationCalls;
    if (args['argon-rpc-url']) {
      const client = await getClient(args['argon-rpc-url']);
      try {
        const registerPool = client.tx.stabilizerPools?.registerPool;
        if (!registerPool)
          throw new Error('Argon runtime does not expose stabilizerPools.registerPool');
        const call = registerPool(plan.pool.poolManager, poolConfig);
        registrationCalls = {
          callHex: call.method.toHex(),
          sudoCallHex: client.tx.sudo.sudo(call).method.toHex(),
        };
      } finally {
        await client.disconnect();
      }
    }

    const runtimeRegistration = {
      pallet: 'stabilizerPools',
      method: 'registerPool',
      args: [plan.pool.poolManager, poolConfig],
      ...registrationCalls,
    };

    const deployment: StabilizationPoolDeployment = { ...plan, transactions };
    async function saveProgress() {
      const manifest = { ...deployment, planHash, runtimeRegistration };
      await mkdir(dirname(outputPath), { recursive: true });
      await writeFile(`${outputPath}.tmp`, `${stringifyJson(manifest)}\n`);
      await rename(`${outputPath}.tmp`, outputPath);
    }

    // Persist the complete bundle before the first transaction; receipt writes are atomic.
    await saveProgress();
    if (publishing) {
      await publishStabilizationPool(connection.ethers, deployment, saveProgress);
      console.log(`Published stabilization pool ${plan.pool.poolId}; manifest: ${outputPath}`);
    } else {
      console.log(`Prepared stabilization pool ${plan.pool.poolId}; manifest: ${outputPath}`);
    }
  } finally {
    await connection.close();
  }
}

void main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
