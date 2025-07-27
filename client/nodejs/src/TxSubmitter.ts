import type { ISubmittableResult } from '@polkadot/types/types/extrinsic';
import {
  ArgonClient,
  dispatchErrorToExtrinsicError,
  ExtrinsicError,
  GenericEvent,
  KeyringPair,
  waitForLoad,
} from './index';
import type { SubmittableExtrinsic } from '@polkadot/api/promise/types';
import type { SignerOptions } from '@polkadot/api/types';
import { getConfig } from './config';

export function logExtrinsicResult(result: ISubmittableResult) {
  if (getConfig().debug) {
    const json = result.status.toJSON() as any;
    const status = Object.keys(json)[0];
    console.debug('Transaction update: "%s"', status, json[status]);
  }
}

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

  public async submit(
    options: Partial<SignerOptions> & {
      logResults?: boolean;
      waitForBlock?: boolean;
      useLatestNonce?: boolean;
      txProgressCallback?: ITxProgressCallback;
    } = {},
  ): Promise<TxResult> {
    const { logResults, waitForBlock, useLatestNonce, ...apiOptions } = options;
    await waitForLoad();
    const result = new TxResult(this.client, logResults);
    result.txProgressCallback = options.txProgressCallback;
    let toHuman = (this.tx.toHuman() as any).method as any;
    let txString = [];
    let api = formatCall(toHuman);
    const args: any[] = [];
    if (api === 'proxy.proxy') {
      toHuman = toHuman.args.call;
      txString.push('Proxy');
      api = formatCall(toHuman);
    }
    if (api.startsWith('utility.batch')) {
      const calls = toHuman.args.calls.map(formatCall).join(', ');
      txString.push(`Batch[${calls}]`);
    } else {
      txString.push(api);
      args.push(toHuman.args);
    }
    args.unshift(txString.join('->'));
    if (useLatestNonce && !apiOptions.nonce) {
      apiOptions.nonce = await this.client.rpc.system.accountNextIndex(this.pair.address);
    }

    console.log('Submitting transaction from %s:', this.pair.address, ...args);
    await this.tx.signAndSend(this.pair, apiOptions, result.onResult.bind(result));
    if (waitForBlock) {
      await result.inBlockPromise;
    }
    return result;
  }
}

function formatCall(call: any): string {
  return `${call.section}.${call.method}`;
}

export type ITxProgressCallback = (progressToInBlock: number, result: TxResult) => void;
export class TxResult {
  public inBlockPromise: Promise<Uint8Array>;
  public finalizedPromise: Promise<Uint8Array>;
  public status?: ISubmittableResult['status'];
  public readonly events: GenericEvent[] = [];

  /**
   * The index of the batch that was interrupted, if any.
   */
  public batchInterruptedIndex?: number;
  public includedInBlock?: Uint8Array;
  /**
   * The final fee paid for the transaction, including the fee tip.
   */
  public finalFee?: bigint;
  /**
   * The fee tip paid for the transaction.
   */
  public finalFeeTip?: bigint;

  public txProgressCallback?: ITxProgressCallback;

  private inBlockResolve!: (blockHash: Uint8Array) => void;
  private inBlockReject!: (error: ExtrinsicError) => void;
  private finalizedResolve!: (blockHash: Uint8Array) => void;
  private finalizedReject!: (error: ExtrinsicError) => void;

  constructor(
    private readonly client: ArgonClient,
    private shouldLog: boolean = false,
  ) {
    this.inBlockPromise = new Promise((resolve, reject) => {
      this.inBlockResolve = resolve;
      this.inBlockReject = reject;
    });
    this.finalizedPromise = new Promise((resolve, reject) => {
      this.finalizedResolve = resolve;
      this.finalizedReject = reject;
    });
    // drown unhandled
    this.inBlockPromise.catch(() => {});
    this.finalizedPromise.catch(() => {});
  }

  public onResult(result: ISubmittableResult) {
    this.status = result.status;
    if (this.shouldLog) {
      logExtrinsicResult(result);
    }
    const { events, status, dispatchError, isFinalized } = result;
    if (status.isInBlock) {
      this.includedInBlock = new Uint8Array(status.asInBlock);
      let encounteredError = dispatchError;
      let batchErrorIndex: number | undefined;
      for (const event of events) {
        this.events.push(event.event);
        if (this.client.events.utility.BatchInterrupted.is(event.event)) {
          batchErrorIndex = event.event.data[0].toNumber();
          this.batchInterruptedIndex = batchErrorIndex;
          encounteredError = event.event.data[1] as any;
        }
        if (this.client.events.transactionPayment.TransactionFeePaid.is(event.event)) {
          const [_who, actualFee, tip] = event.event.data;
          this.finalFee = actualFee.toBigInt();
          this.finalFeeTip = tip.toBigInt();
        }
      }

      if (encounteredError) {
        const error = dispatchErrorToExtrinsicError(this.client, encounteredError, batchErrorIndex);
        this.reject(error);
      } else {
        this.inBlockResolve(new Uint8Array(status.asInBlock));
      }
    }
    if (isFinalized) {
      this.finalizedResolve(status.asFinalized);
    }
    if (this.txProgressCallback) {
      let percent = 0;
      if (result.status.isBroadcast) {
        percent = 50;
      } else if (result.status.isInBlock) {
        percent = 100;
      }
      this.txProgressCallback(percent, this);
    }
  }

  private reject(error: ExtrinsicError) {
    this.inBlockReject(error);
    this.finalizedReject(error);
  }
}
