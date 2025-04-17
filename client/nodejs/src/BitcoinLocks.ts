import {
  Accountset,
  type ArgonClient,
  type KeyringPair,
  VaultMonitor,
} from './index';
import { formatArgons } from './utils';
import { Vault } from './Vault';

const SATS_PER_BTC = 100_000_000n;

export class BitcoinLocks {
  constructor(readonly client: Promise<ArgonClient>) {}

  public async getMarketRate(satoshis: bigint): Promise<bigint> {
    const client = await this.client;
    const sats = client.createType('U64', satoshis.toString());
    const marketRate = await client.rpc.state.call(
      'BitcoinApis_market_rate',
      sats.toHex(true),
    );
    const rate = client.createType('Option<U128>', marketRate);
    if (!rate.isSome) {
      throw new Error('Market rate not available');
    }
    return rate.value.toBigInt();
  }

  public async buildBitcoinLockTx(args: {
    vaultId: number;
    keypair: KeyringPair;
    amount: bigint;
    bitcoinXpub: string;
    tip?: bigint;
    reducedBalanceBy?: bigint;
  }) {
    const { vaultId, keypair, bitcoinXpub, tip } = args;
    let amount = args.amount;
    const marketRatePerBitcoin = await this.getMarketRate(100_000_000n);

    const client = await this.client;
    const account = await client.query.system.account(keypair.address);
    const freeBalance = account.data.free.toBigInt();
    let availableBalance = freeBalance;
    if (args.reducedBalanceBy) {
      availableBalance -= args.reducedBalanceBy;
    }

    /**
     * If 1_000_000 microgons are available, and the market rate is 100 microgons per satoshi, then
     * 1_000_000 / 100 = 10_000 satoshis needed
     */
    // Add wiggle room for fluctuating price
    const satoshisNeeded =
      (amount * SATS_PER_BTC) / marketRatePerBitcoin - 500n;

    const tx = client.tx.bitcoinLocks.initialize(
      vaultId,
      satoshisNeeded,
      bitcoinXpub,
    );
    const existentialDeposit =
      client.consts.balances.existentialDeposit.toBigInt();
    const finalTip = tip ?? 0n;
    const fees = await tx.paymentInfo(keypair.address, { tip });
    const txFee = fees.partialFee.toBigInt();
    const tickDuration = (
      await client.query.ticks.genesisTicker()
    ).tickDurationMillis.toNumber();
    const rawVault = await client.query.vaults.vaultsById(vaultId);
    const vault = new Vault(vaultId, rawVault.unwrap(), tickDuration);
    const btcFee = vault.calculateBitcoinFee(amount);
    const totalCharge = txFee + finalTip + btcFee;
    if (amount + totalCharge + existentialDeposit > availableBalance) {
      throw new Error('Insufficient balance to lock bitcoins');
    }
    console.log(
      `Locking ${satoshisNeeded} satoshis in vault ${vaultId} with market rate of ${formatArgons(marketRatePerBitcoin)}/btc. Xpub: ${bitcoinXpub}`,
    );
    return { tx, txFee, btcFee, satoshis: satoshisNeeded, freeBalance };
  }

  public static async waitForSpace(
    accountset: Accountset,
    options: {
      argonAmount: bigint;
      bitcoinXpub: string;
      maxLockFee?: bigint;
      tip?: bigint;
    },
  ): Promise<{
    satoshis: bigint;
    argons: bigint;
    vaultId: number;
    txFee: bigint;
    btcFee: bigint;
    utxoId: number;
    finalizedPromise: Promise<Uint8Array>;
  }> {
    const { argonAmount, bitcoinXpub, maxLockFee, tip = 0n } = options;
    const vaults = new VaultMonitor(accountset, {
      bitcoinSpaceAvailable: argonAmount,
    });

    return new Promise(async (resolve, reject) => {
      vaults.events.on('bitcoin-space-above', async (vaultId, amount) => {
        const vault = vaults.vaultsById[vaultId];
        const fee = vault.calculateBitcoinFee(amount);
        console.log(
          `Vault ${vaultId} has ${formatArgons(amount)} argons available for bitcoin. Lock fee is ${formatArgons(fee)}`,
        );
        if (maxLockFee !== undefined && fee > maxLockFee) {
          console.log(
            `Skipping vault ${vaultId} due to high lock fee: ${formatArgons(maxLockFee)}`,
          );
          return;
        }

        try {
          const bitcoinLock = new BitcoinLocks(accountset.client);
          const { tx, satoshis, btcFee, txFee } =
            await bitcoinLock.buildBitcoinLockTx({
              vaultId,
              keypair: accountset.txSubmitterPair,
              amount: argonAmount,
              bitcoinXpub,
              tip,
            });
          const result = await accountset
            .tx(tx)
            .then(x => x.submit({ waitForBlock: true, tip }));

          const client = await accountset.client;
          const utxoId = result.events
            .find(x => client.events.bitcoinLocks.BitcoinLockCreated.is(x))
            ?.data.utxoId?.toNumber();
          if (!utxoId) {
            throw new Error('Failed to find UTXO ID');
          }

          resolve({
            satoshis,
            argons: argonAmount,
            vaultId,
            btcFee,
            txFee,
            finalizedPromise: result.finalizedPromise,
            utxoId,
          });
        } catch (err) {
          console.error('Error submitting bitcoin lock tx:', err);
          reject(err);
        } finally {
          vaults.stop();
        }
      });
      await vaults.monitor();
    });
  }
}
