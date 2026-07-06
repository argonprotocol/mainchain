import { ArgonClient, GenericEvent, SpRuntimeDispatchError, u8aEq } from './index';
import {
  dispatchErrorToExtrinsicError,
  ExtrinsicError,
  TxSubmissionError,
  TxSubmissionErrorCode,
} from './utils';
import type { ISubmittableResult } from '@polkadot/types/types/extrinsic';
import { DispatchError } from '@polkadot/types/interfaces';

export type ITxProgressCallback = (progressToInBlock: number, result?: TxResult) => void;

type IBlockInclusion = {
  blockHash: Uint8Array;
  blockNumber?: number;
  extrinsicIndex: number;
  events: GenericEvent[];
};

type IPendingInBlock = Omit<IBlockInclusion, 'blockNumber'>;

export class TxResult {
  #isBroadcast = false;
  #submissionError?: Error;
  #pendingInBlock?: IPendingInBlock;

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
      nonce: number;
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

  public async setSeenInBlock(block: IBlockInclusion): Promise<void> {
    if (block.blockNumber === undefined) {
      this.#pendingInBlock = {
        blockHash: block.blockHash,
        extrinsicIndex: block.extrinsicIndex,
        events: block.events,
      };
      return;
    }

    if (
      this.blockHash &&
      this.blockNumber === block.blockNumber &&
      this.extrinsicIndex === block.extrinsicIndex &&
      u8aEq(this.blockHash, block.blockHash)
    ) {
      this.#pendingInBlock = undefined;
      return;
    }

    this.#pendingInBlock = undefined;
    this.parseEvents(block.events);
    this.blockHash = block.blockHash;
    this.blockNumber = block.blockNumber;
    this.extrinsicIndex = block.extrinsicIndex;
    this.updateProgress();
    if (this.extrinsicError) {
      this.inBlockReject(this.extrinsicError);
    } else {
      this.inBlockResolve(block.blockHash);
    }
  }

  public async setFinalized() {
    const pendingInBlock = this.#pendingInBlock;
    if (pendingInBlock) {
      await this.publishSeenInBlock(pendingInBlock);
    } else if (this.blockHash && this.blockNumber === undefined) {
      if (this.extrinsicIndex === undefined) {
        throw new Error('Cannot finalize transaction before extrinsic index is known');
      }

      await this.publishSeenInBlock({
        blockHash: this.blockHash,
        extrinsicIndex: this.extrinsicIndex,
        events: this.events,
      });
    }

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
      try {
        const pendingInBlock = createPendingInBlock(
          Uint8Array.from(status.asInBlock),
          txIndex,
          extrinsicEvents,
        );
        this.#pendingInBlock = pendingInBlock;

        void this.publishSeenInBlock(pendingInBlock).catch(error => {
          if (!isMissingBlockHeaderError(error)) {
            this.submissionError = error as Error;
          }
        });
      } catch (error) {
        this.submissionError = error as Error;
      }
    }
    if (status.isUsurped) {
      this.#pendingInBlock = undefined;
      this.submissionError = new TxSubmissionError(
        TxSubmissionErrorCode.Usurped,
        `Transaction was usurped by ${status.asUsurped.toHex()}.`,
      );
    }
    if (status.isDropped) {
      this.#pendingInBlock = undefined;
      this.submissionError = new TxSubmissionError(
        TxSubmissionErrorCode.Dropped,
        'Transaction was dropped before it was included in a block.',
      );
    }
    if (status.isInvalid) {
      this.#pendingInBlock = undefined;
      this.submissionError = new TxSubmissionError(
        TxSubmissionErrorCode.Invalid,
        'Transaction was rejected as invalid by the node.',
      );
    }
    if (isFinalized) {
      try {
        this.#pendingInBlock = createPendingInBlock(
          Uint8Array.from(status.asFinalized),
          txIndex ?? this.#pendingInBlock?.extrinsicIndex ?? this.extrinsicIndex,
          extrinsicEvents.length ? extrinsicEvents : this.events,
        );
      } catch (error) {
        this.submissionError = error as Error;
        return;
      }

      void this.setFinalized().catch(error => {
        this.submissionError = error as Error;
      });
    }
  }

  private async publishSeenInBlock(block: IPendingInBlock) {
    const pendingInBlock = this.#pendingInBlock;
    if (
      !pendingInBlock ||
      pendingInBlock.extrinsicIndex !== block.extrinsicIndex ||
      !u8aEq(pendingInBlock.blockHash, block.blockHash)
    ) {
      return;
    }

    const blockNumber = await this.client.rpc.chain.getHeader(block.blockHash).then(h => h.number.toNumber());

    const currentPendingInBlock = this.#pendingInBlock;
    if (
      !currentPendingInBlock ||
      currentPendingInBlock.extrinsicIndex !== block.extrinsicIndex ||
      !u8aEq(currentPendingInBlock.blockHash, block.blockHash)
    ) {
      return;
    }

    await this.setSeenInBlock({
      ...block,
      blockNumber,
    });
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

function createPendingInBlock(
  blockHash: Uint8Array,
  extrinsicIndex: number | undefined,
  events: GenericEvent[],
): IPendingInBlock {
  if (extrinsicIndex === undefined) {
    throw new Error('Cannot publish transaction block state before extrinsic index is known');
  }

  return {
    blockHash,
    extrinsicIndex,
    events,
  };
}

function isMissingBlockHeaderError(error: unknown) {
  return String(error).includes('Unable to retrieve header and parent from supplied hash');
}
