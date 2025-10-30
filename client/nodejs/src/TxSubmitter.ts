import { ArgonClient, KeyringPair, waitForLoad } from './index';
import type { SubmittableExtrinsic } from '@polkadot/api/promise/types';
import type { SignerOptions } from '@polkadot/api/types';
import { ITxProgressCallback, TxResult } from './TxResult';

export type ISubmittableOptions = Partial<SignerOptions> & {
  tip?: bigint;
  logResults?: boolean;
  useLatestNonce?: boolean;
  txProgressCallback?: ITxProgressCallback;
  disableAutomaticTxTracking?: boolean;
};

export class TxSubmitter {
  constructor(
    public readonly client: ArgonClient,
    public tx: SubmittableExtrinsic,
    public pair: KeyringPair,
  ) {}

  public async feeEstimate(tip?: bigint): Promise<bigint> {
    const { partialFee } = await this.tx.paymentInfo(this.pair, { tip });
    return partialFee.toBigInt();
  }

  public async canAfford(
    options: {
      tip?: bigint;
      unavailableBalance?: bigint;
      includeExistentialDeposit?: boolean;
    } = {},
  ): Promise<{ canAfford: boolean; availableBalance: bigint; txFee: bigint }> {
    const { tip, unavailableBalance } = options;
    const account = await this.client.query.system.account(this.pair.address);
    let availableBalance = account.data.free.toBigInt();
    const userBalance = availableBalance;
    if (unavailableBalance) {
      availableBalance -= unavailableBalance;
    }
    const existentialDeposit = options.includeExistentialDeposit
      ? this.client.consts.balances.existentialDeposit.toBigInt()
      : 0n;
    const fees = await this.feeEstimate(tip);
    const totalCharge = fees + (tip ?? 0n);
    const canAfford = availableBalance >= totalCharge + existentialDeposit;
    return { canAfford, availableBalance: userBalance, txFee: fees };
  }

  public async submit(options: ISubmittableOptions = {}): Promise<TxResult> {
    const { useLatestNonce, ...apiOptions } = options;
    await waitForLoad();
    const blockHeight = await this.client.rpc.chain.getHeader().then(h => h.number.toNumber());
    if (options.logResults) {
      this.logRequest();
    }
    if (useLatestNonce && !apiOptions.nonce) {
      apiOptions.nonce = await this.client.rpc.system.accountNextIndex(this.pair.address);
    }

    const signedTx = await this.tx.signAsync(this.pair, apiOptions);
    const txHash = signedTx.hash.toHex();
    const result = new TxResult(this.client, {
      signedHash: txHash,
      method: signedTx.method.toHuman(),
      accountAddress: this.pair.address,
      submittedTime: new Date(),
      submittedAtBlockNumber: blockHeight,
    });
    if (options.disableAutomaticTxTracking !== true) {
      await signedTx.send(result.onSubscriptionResult.bind(result));
    } else {
      try {
        await signedTx.send();
        result.isBroadcast = true;
      } catch (error) {
        result.submissionError = error as Error;
      }
    }
    return result;
  }

  private logRequest() {
    let toHuman = (this.tx.toHuman() as any).method;
    const txString = [];
    let api = formatCall(toHuman);
    const args: any[] = [];
    if (api === 'proxy.proxy') {
      toHuman = toHuman.args.call;
      txString.push('Proxy');
      api = formatCall(toHuman);
    }
    if (api.startsWith('utility.batch')) {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-call
      const calls = toHuman.args.calls.map(formatCall).join(', ');
      txString.push(`Batch[${calls}]`);
    } else {
      txString.push(api);
      args.push(toHuman.args);
    }
    args.unshift(txString.join('->'));
    console.log('Submitting transaction from %s:', this.pair.address, ...args);
  }
}

function formatCall(call: any): string {
  return `${call.section}.${call.method}`;
}
