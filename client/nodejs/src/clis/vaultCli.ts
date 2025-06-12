import { Command } from '@commander-js/extra-typings';
import { VaultMonitor } from '../VaultMonitor';
import { TxSubmitter } from '../TxSubmitter';
import { Vault } from '../Vault';
import { BitcoinLocks } from '../BitcoinLocks';
import { accountsetFromCli } from './index';
import { MICROGONS_PER_ARGON } from '../utils';

export default function vaultCli() {
  const program = new Command('vaults').description('Monitor vaults and manage securitization');

  program
    .command('list', { isDefault: true })
    .description('Show current state of vaults')
    .action(async () => {
      const accountset = await accountsetFromCli(program);
      const vaults = new VaultMonitor(accountset, undefined, {
        vaultOnlyWatchMode: true,
      });
      await vaults.monitor(true);
      process.exit(0);
    });

  program
    .command('modify-securitization')
    .description('Change the vault securitization ratio')
    .requiredOption('-v, --vault-id <id>', 'The vault id to use', parseInt)
    .requiredOption(
      '-a, --argons <amount>',
      'The number of argons to set as securitization',
      parseFloat,
    )
    .option('--ratio <ratio>', 'The new securitization ratio', parseFloat)
    .option('--tip <amount>', 'The tip to include with the transaction', parseFloat)
    .action(async ({ tip, argons, vaultId, ratio }) => {
      const accountset = await accountsetFromCli(program);
      const client = await accountset.client;
      const resolvedTip = tip ? BigInt(tip * MICROGONS_PER_ARGON) : 0n;
      const microgons = BigInt(argons * MICROGONS_PER_ARGON);

      const rawVault = (await client.query.vaults.vaultsById(vaultId)).unwrap();
      if (rawVault.operatorAccountId.toHuman() !== accountset.seedAddress) {
        console.error('Vault does not belong to this account');
        process.exit(1);
      }
      const existingFunds = rawVault.securitization.toBigInt();
      const additionalFunds = microgons > existingFunds ? microgons - existingFunds : 0n;
      const tx = client.tx.vaults.modifyFunding(
        vaultId,
        microgons,
        ratio !== undefined
          ? BigNumber(ratio).times(BigNumber(2).pow(64)).toFixed(0)
          : rawVault.securitizationRatio.toBigInt(),
      );
      const submit = new TxSubmitter(client, tx, accountset.txSubmitterPair);
      const canAfford = await submit.canAfford({
        tip: resolvedTip,
        unavailableBalance: additionalFunds,
      });
      if (!canAfford.canAfford) {
        console.warn('Insufficient balance to modify vault securitization', {
          ...canAfford,
          addedSecuritization: additionalFunds,
        });
        process.exit(1);
      }
      try {
        const result = await submit.submit({ tip: resolvedTip });
        await result.inBlockPromise;
        console.log('Vault securitization modified');
        process.exit();
      } catch (error) {
        console.error('Error modifying vault securitization', error);
        process.exit(1);
      }
    });

  program
    .command('make-bitcoin-space')
    .description('Make bitcoin space in a vault and lock it immediately in the same tx.')
    .requiredOption('-v, --vault-id <id>', 'The vault id to use', parseInt)
    .requiredOption('-a, --argons <amount>', 'The number of argons to add', parseFloat)
    .requiredOption('--bitcoin-pubkey <pubkey>', 'The pubkey to use for the bitcoin lock')
    .option('--tip <amount>', 'The tip to include with the transaction', parseFloat)
    .action(async ({ tip, argons, vaultId, bitcoinPubkey }) => {
      let pubkey = bitcoinPubkey;
      if (!bitcoinPubkey.startsWith('0x')) {
        pubkey = `0x${bitcoinPubkey}`;
      }
      if (pubkey.length !== 68) {
        throw new Error('Bitcoin pubkey must be 66 characters (add 0x in front optionally)');
      }
      const accountset = await accountsetFromCli(program);
      const client = await accountset.client;
      const resolvedTip = tip ? BigInt(tip * MICROGONS_PER_ARGON) : 0n;
      const microgons = BigInt(argons * MICROGONS_PER_ARGON);
      const bitcoinLocks = new BitcoinLocks(Promise.resolve(client));
      const existentialDeposit = client.consts.balances.existentialDeposit.toBigInt();
      const tickDuration = (await client.query.ticks.genesisTicker()).tickDurationMillis.toNumber();

      const rawVault = (await client.query.vaults.vaultsById(vaultId)).unwrap();
      if (rawVault.operatorAccountId.toHuman() !== accountset.seedAddress) {
        console.error('Vault does not belong to this account');
        process.exit(1);
      }
      const vaultModifyTx = client.tx.vaults.modifyFunding(
        vaultId,
        microgons,
        rawVault.securitizationRatio.toBigInt(),
      );
      const vaultTxFee = (
        await vaultModifyTx.paymentInfo(accountset.txSubmitterPair)
      ).partialFee.toBigInt();
      const vault = new Vault(vaultId, rawVault, tickDuration);

      const argonsNeeded = microgons - vault.securitization;
      const argonsAvailable = microgons - vault.availableBitcoinSpace();

      const account = await client.query.system.account(accountset.seedAddress);
      const freeBalance = account.data.free.toBigInt();
      const {
        tx: lockTx,
        btcFee,
        txFee,
      } = await bitcoinLocks.buildBitcoinLockTx({
        vaultId,
        keypair: accountset.txSubmitterPair,
        amount: argonsAvailable,
        bitcoinXpub: pubkey,
        tip: resolvedTip,
        reducedBalanceBy: argonsNeeded + vaultTxFee + resolvedTip,
      });
      if (
        argonsNeeded + txFee + vaultTxFee + resolvedTip + btcFee + existentialDeposit >
        freeBalance
      ) {
        console.warn('Insufficient balance to add bitcoin space and use bitcoins', {
          freeBalance,
          txFee,
          vaultTxFee,
          btcFee,
          argonsAvailable,
          vaultMicrogons: microgons,
          existentialDeposit,
          neededBalanceAboveED: argonsNeeded + txFee + resolvedTip + btcFee + vaultTxFee,
        });
        process.exit(1);
      }
      console.log('Adding bitcoin space and locking bitcoins...', {
        newArgonsAvailable: argonsAvailable,
        txFee,
        vaultTxFee,
        btcFee,
        resolvedTip,
      });

      const txSubmitter = new TxSubmitter(
        client,
        client.tx.utility.batchAll([vaultModifyTx, lockTx]),
        accountset.txSubmitterPair,
      );
      const result = await txSubmitter.submit({ tip: resolvedTip });
      try {
        await result.inBlockPromise;
        console.log('Bitcoin space done');
      } catch (error) {
        console.error('Error using bitcoin space', error);
        process.exit(1);
      }
    });
  return program;
}
