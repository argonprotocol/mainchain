import { Command } from '@commander-js/extra-typings';
import { VaultMonitor } from '../VaultMonitor';
import { BitcoinLocks } from '../BitcoinLocks';
import { formatArgons, MICROGONS_PER_ARGON } from '../utils';
import { accountsetFromCli } from './index';

export default function bitcoinCli() {
  const program = new Command('bitcoin').description('Wait for bitcoin space');

  program
    .command('watch')
    .requiredOption(
      '-a, --argons <argons>',
      'Alert when bitcoin space exceeds this amount',
      parseFloat,
    )
    .description('Watch for bitcoin space available')
    .action(async ({ argons }) => {
      const accountset = await accountsetFromCli(program);
      const bot = new VaultMonitor(accountset, {
        bitcoinSpaceAvailable: argons ? BigInt(argons * MICROGONS_PER_ARGON) : 1n,
      });
      bot.events.on('bitcoin-space-above', async (vaultId, amount) => {
        const vault = bot.vaultsById[vaultId];
        const fee = vault.calculateBitcoinFee(amount);
        const ratio = (100n * fee) / amount;
        console.log(
          `Vault ${vaultId} has ${formatArgons(amount)} argons available for bitcoin. Fee ratio is ${ratio}%`,
        );
      });
      await bot.monitor();
    });

  program
    .command('wait-for-space')
    .description('Lock bitcoin when available at a given rate')
    .requiredOption(
      '-a, --argons <amount>',
      'Bitcoin argons needed. NOTE: your account must have enough to cover fees + tip after this amount.',
      parseFloat,
    )
    .requiredOption('--bitcoin-xpub <xpub>', 'The xpub key to use for bitcoin locking')
    .option('--max-lock-fee <argons>', "The max lock fee you're willing to pay", parseFloat)
    .option('--tip <amount>', 'The tip to include with the transaction', parseFloat, 0.0)
    .action(async ({ argons, bitcoinXpub, maxLockFee, tip }) => {
      const amountToLock = BigInt(argons * MICROGONS_PER_ARGON);

      const accountset = await accountsetFromCli(program);
      await BitcoinLocks.waitForSpace(accountset, {
        argonAmount: amountToLock,
        bitcoinXpub,
        maxLockFee: maxLockFee !== undefined ? BigInt(maxLockFee * MICROGONS_PER_ARGON) : undefined,
        tip: BigInt(tip * MICROGONS_PER_ARGON),
      }).then(({ vaultId, satoshis, txFee, btcFee }) => {
        console.log(
          `Locked ${satoshis} satoshis in vault ${vaultId}. Tx fee=${formatArgons(
            txFee,
          )}, Lock fee=${formatArgons(btcFee)}.`,
        );
        process.exit(0);
      });
    });

  return program;
}
