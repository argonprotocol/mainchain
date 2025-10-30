import { ArgonClient, GenericEvent, SpRuntimeDispatchError } from './index';
import { dispatchErrorToExtrinsicError, ExtrinsicError } from './utils';
import type { ISubmittableResult } from '@polkadot/types/types/extrinsic';
import { DispatchError } from '@polkadot/types/interfaces';

export type ITxProgressCallback = (progressToInBlock: number, result?: TxResult) => void;

export class TxResult {
  #isBroadcast = false;
  #submissionError?: Error;

  set isBroadcast(value: boolean) {
    this.#isBroadcast = value;
    this.updateProgress();
  }

  get isBroadcast(): boolean {
    return this.#isBroadcast;
  }

  set submissionError(value: Error) {
    if (value) {
      this.#submissionError = value;
      this.finalizedReject(value);
      this.inBlockReject(value);
      this.updateProgress();
    }
  }

  get submissionError(): Error | undefined {
    return this.#submissionError;
  }

  public waitForFinalizedBlock: Promise<Uint8Array>;
  public waitForInFirstBlock: Promise<Uint8Array>;
  public events: GenericEvent[] = [];

  public extrinsicError: ExtrinsicError | Error | undefined;
  public extrinsicIndex: number | undefined;

  public txProgressCallback?: ITxProgressCallback;
  /**
   * The index of the batch that was interrupted, if any.
   */
  public batchInterruptedIndex?: number;
  public blockHash?: Uint8Array;
  public blockNumber?: number;
  /**
   * The final fee paid for the transaction, including the fee tip.
   */
  public finalFee?: bigint;
  /**
   * The fee tip paid for the transaction.
   */
  public finalFeeTip?: bigint;

  public txProgress = 0;
  public isFinalized = false;

  protected finalizedResolve!: (block: Uint8Array) => void;
  protected finalizedReject!: (error: ExtrinsicError | Error) => void;
  protected inBlockResolve!: (block: Uint8Array) => void;
  protected inBlockReject!: (error: ExtrinsicError | Error) => void;

  constructor(
    protected readonly client: ArgonClient,
    public extrinsic: {
      signedHash: string;
      method: any;
      submittedTime: Date;
      submittedAtBlockNumber: number;
      accountAddress: string;
    },
  ) {
    this.waitForFinalizedBlock = new Promise((resolve, reject) => {
      this.finalizedResolve = resolve;
      this.finalizedReject = reject;
    });
    this.waitForInFirstBlock = new Promise((resolve, reject) => {
      this.inBlockResolve = resolve;
      this.inBlockReject = reject;
    });
    // drown reject
    this.waitForFinalizedBlock.catch(() => null);
    this.waitForInFirstBlock.catch(() => null);
  }

  public async setSeenInBlock(block: {
    blockHash: Uint8Array;
    blockNumber?: number;
    extrinsicIndex: number;
    events: GenericEvent[];
  }): Promise<void> {
    const { blockHash, blockNumber, events } = block;
    if (blockHash !== this.blockHash) {
      this.parseEvents(events);
      this.blockHash = blockHash;
      this.blockNumber =
        blockNumber ??
        (await this.client.rpc.chain.getHeader(blockHash).then(h => h.number.toNumber()));
      this.extrinsicIndex = block.extrinsicIndex;
      this.updateProgress();
      if (this.extrinsicError) {
        this.inBlockReject(this.extrinsicError);
      } else {
        this.inBlockResolve(blockHash);
      }
    }
  }

  public setFinalized() {
    this.isFinalized = true;
    this.updateProgress();

    let error = this.extrinsicError ?? this.submissionError;
    if (!error && !this.blockHash) {
      error = new Error('Cannot finalize transaction before it is included in a block');
    }

    if (error) {
      this.finalizedReject(error);
      this.inBlockReject(error);
    } else {
      this.finalizedResolve(this.blockHash!);
      this.inBlockResolve(this.blockHash!);
    }
  }

  public onSubscriptionResult(result: ISubmittableResult) {
    const { events, status, isFinalized, txIndex } = result;
    const extrinsicEvents = events.map(x => x.event);

    if (status.isBroadcast) {
      this.isBroadcast = true;
      if (result.internalError) this.submissionError = result.internalError;
    }
    if (status.isInBlock) {
      void this.setSeenInBlock({
        blockHash: Uint8Array.from(status.asInBlock),
        events: extrinsicEvents,
        extrinsicIndex: txIndex!,
      });
    }
    if (isFinalized) {
      this.setFinalized();
    }
  }

  private updateProgress() {
    if (this.isFinalized || this.submissionError) {
      this.txProgress = 100;
    } else if (this.blockNumber) {
      const elapsedBlocks = this.blockNumber - this.extrinsic.submittedAtBlockNumber;
      const FINALIZATION_BLOCKS = 5;
      const remainingPercent = Math.max(0, FINALIZATION_BLOCKS - elapsedBlocks) * 20;
      const percent = 100 - remainingPercent;
      this.txProgress = Math.min(percent, 99);
    } else if (this.extrinsic.submittedAtBlockNumber) {
      this.txProgress = 10;
    }
    this.txProgressCallback?.(this.txProgress);
  }

  private parseEvents(events: GenericEvent[]) {
    let encounteredError: SpRuntimeDispatchError | undefined;
    for (const event of events) {
      if (this.client.events.system.ExtrinsicFailed.is(event)) {
        const { dispatchError } = event.data;
        encounteredError ??= dispatchError;
      }
      if (this.client.events.utility.BatchInterrupted.is(event)) {
        const { index, error } = event.data;
        this.batchInterruptedIndex = index.toNumber();
        encounteredError = error;
      }
      if (this.client.events.transactionPayment.TransactionFeePaid.is(event)) {
        const { actualFee, tip } = event.data;
        this.finalFee = actualFee.toBigInt();
        this.finalFeeTip = tip.toBigInt();
      }
    }
    if (encounteredError) {
      this.extrinsicError = dispatchErrorToExtrinsicError(
        this.client,
        encounteredError as unknown as DispatchError,
        this.batchInterruptedIndex,
        this.finalFee,
      );
    } else {
      this.extrinsicError = undefined;
    }
    this.events = events;
  }
}
