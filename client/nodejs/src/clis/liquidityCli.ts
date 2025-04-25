import { Command } from '@commander-js/extra-typings';
import { BidPool } from '../BidPool';
import { VaultMonitor } from '../VaultMonitor';
import { formatArgons } from '../utils';
import { accountsetFromCli } from './index';

export default function liquidityCli() {
  const program = new Command('liquidity-pools').description(
    'Monitor or bond to liquidity pools',
  );
  program
    .command('list', { isDefault: true })
    .description('Show or watch the vault bid pool rewards')
    .action(async () => {
      const accountset = await accountsetFromCli(program);
      const bidPool = new BidPool(
        accountset.client,
        accountset.txSubmitterPair,
      );
      await bidPool.watch();
    });

  program
    .command('bond')
    .description('Bond argons to a liquidity pool')
    .requiredOption('-v, --vault-id <id>', 'The vault id to use', parseInt)
    .requiredOption(
      '-a, --argons <amount>',
      'The number of argons to set the vault to',
      parseFloat,
    )
    .option(
      '--tip <amount>',
      'The tip to include with the transaction',
      parseFloat,
    )
    .action(async ({ tip, argons, vaultId }) => {
      const accountset = await accountsetFromCli(program);
      const resolvedTip = tip ? BigInt(tip * 1e6) : 0n;

      const microgons = BigInt(argons * 1e6);
      const bidPool = new BidPool(
        accountset.client,
        accountset.txSubmitterPair,
      );
      await bidPool.bondArgons(vaultId, microgons, { tip: resolvedTip });
      console.log('Bonded argons to liquidity pool bond');
      process.exit();
    });

  program
    .command('wait-for-space')
    .description(
      'Add bonded argons to a liquidity pool when the market rate is favorable',
    )
    .requiredOption(
      '--max-argons <amount>',
      'Max daily argons to use per slot',
      parseFloat,
    )
    .option(
      '--min-pct-sharing <percent>',
      'The minimum profit sharing percent to allow',
      parseInt,
      100,
    )
    .option(
      '--tip <amount>',
      'The tip to include with the transaction',
      parseFloat,
    )
    .action(async ({ maxArgons, minPctSharing, tip }) => {
      const maxAmountPerSlot = BigInt(maxArgons * 1e6);

      const accountset = await accountsetFromCli(program);
      const vaults = new VaultMonitor(
        accountset,
        {
          liquidityPoolSpaceAvailable: 1_000_000n,
        },
        { shouldLog: false },
      );
      const bidPool = new BidPool(
        accountset.client,
        accountset.txSubmitterPair,
      );
      const resolvedTip = tip ? BigInt(tip * 1e6) : 0n;
      console.log('Waiting for liquidity pool space...');

      vaults.events.on(
        'liquidity-pool-space-above',
        async (vaultId, amount) => {
          const vault = vaults.vaultsById[vaultId];
          if (
            vault.terms.liquidityPoolProfitSharing.times(100).toNumber() <
            minPctSharing
          ) {
            console.info(
              `Skipping vault ${vaultId} due to lower profit sharing than ${minPctSharing}%`,
            );
            return;
          }
          let amountToAdd = amount;
          if (amountToAdd > maxAmountPerSlot) {
            amountToAdd = maxAmountPerSlot;
          }
          await bidPool.bondArgons(vaultId, amountToAdd, { tip: resolvedTip });
          console.log('Bonding argons to vault liquidity pool', {
            vaultId,
            amount: formatArgons(amountToAdd),
          });
        },
      );
      await vaults.monitor();
    });
  return program;
}
