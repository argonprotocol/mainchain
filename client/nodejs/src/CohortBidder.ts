import type { Accountset } from './Accountset';
import {
  ArgonClient,
  type ArgonPrimitivesBlockSealMiningRegistration,
  ExtrinsicError,
} from './index';
import { formatArgons } from './utils';
import { Bool, u32, u64, Vec } from '@polkadot/types-codec';

interface IBidDetail {
  address: string;
  bidMicrogons: bigint;
  bidAtTick: number;
}

export class CohortBidder {
  public get clientPromise(): Promise<ArgonClient> {
    return this.accountset.client;
  }

  public txFees = 0n;
  public bidsAttempted = 0;
  public winningBids: IBidDetail[] = [];

  public readonly myAddresses = new Set<string>();

  public readonly currentBids: {
    atBlockNumber: number;
    atTick: number;
    mostRecentBidTick: number;
    bids: IBidDetail[];
  } = {
    bids: [],
    mostRecentBidTick: 0,
    atTick: 0,
    atBlockNumber: 0,
  };
  private unsubscribe?: () => void;

  private pendingRequest: Promise<any> | undefined;
  private isStopped = false;
  private millisPerTick?: number;
  private minIncrement = 10_000n;

  private nextCohortSize?: number;

  private lastBidTick: number = 0;

  private evaluateInterval?: NodeJS.Timeout;

  constructor(
    public accountset: Accountset,
    public cohortStartingFrameId: number,
    public subaccounts: { index: number; isRebid: boolean; address: string }[],
    public options: {
      minBid: bigint;
      maxBid: bigint;
      maxBudget: bigint;
      bidIncrement: bigint;
      bidDelay: number;
      tipPerTransaction?: bigint;
    },
    public callbacks?: {
      onBidsUpdated?(args: { bids: IBidDetail[]; atBlockNumber: number; tick: number }): void;
      onBidParamsAdjusted?(args: {
        tick: number;
        blockNumber: number;
        maxSeats: number;
        winningBidCount: number;
        reason: 'max-bid-too-low' | 'insufficient-balance' | 'max-budget-too-low';
        availableBalanceForBids: bigint;
      }): void;
      onBidsSubmitted?(args: {
        tick: number;
        blockNumber: number;
        microgonsPerSeat: bigint;
        txFeePlusTip: bigint;
        submittedCount: number;
      }): void;
      onBidsRejected?(args: {
        tick: number;
        blockNumber: number;
        microgonsPerSeat: bigint;
        submittedCount: number;
        rejectedCount: number;
        bidError: ExtrinsicError;
      }): void;
    },
  ) {
    this.subaccounts.forEach(x => {
      this.myAddresses.add(x.address);
    });
  }

  public async start() {
    console.log(`Starting cohort ${this.cohortStartingFrameId} bidder`, {
      maxBid: formatArgons(this.options.maxBid),
      minBid: formatArgons(this.options.minBid),
      bidIncrement: formatArgons(this.options.bidIncrement),
      maxBudget: formatArgons(this.options.maxBudget),
      bidDelay: this.options.bidDelay,
      subaccounts: this.subaccounts,
    });

    const client = await this.clientPromise;

    this.minIncrement = client.consts.miningSlot.bidIncrements.toBigInt();

    this.nextCohortSize = await client.query.miningSlot.nextCohortSize().then(x => x.toNumber());
    if (this.subaccounts.length > this.nextCohortSize) {
      console.info(
        `Cohort size ${this.nextCohortSize} is less than provided subaccounts ${this.subaccounts.length}.`,
      );
      this.subaccounts.length = this.nextCohortSize;
    }
    this.millisPerTick = await client.query.ticks
      .genesisTicker()
      .then(x => x.tickDurationMillis.toNumber());

    let didStart = false;
    this.unsubscribe = await client.queryMulti<
      [Vec<ArgonPrimitivesBlockSealMiningRegistration>, u64, u64, u32]
    >(
      [
        client.query.miningSlot.bidsForNextSlotCohort as any,
        client.query.miningSlot.nextFrameId as any,
        client.query.ticks.currentTick as any,
        client.query.system.number as any,
      ],
      async ([rawBids, nextFrameId, currentTick, blockNumber]) => {
        if (nextFrameId.toNumber() === this.cohortStartingFrameId) {
          this.updateBidList(rawBids, blockNumber.toNumber(), currentTick.toNumber());
          if (!didStart) {
            didStart = true;
            // reset schedule to the tick changes
            this.scheduleEvaluation();
            void this.checkWinningBids();
          }
        }
      },
    );
  }

  public async stop(): Promise<CohortBidder['winningBids']> {
    if (this.isStopped) return this.winningBids;
    this.isStopped = true;
    clearInterval(this.evaluateInterval);
    console.log('Stopping bidder for cohort', this.cohortStartingFrameId);
    if (this.unsubscribe) {
      this.unsubscribe();
    }
    const client = await this.clientPromise;
    const [nextFrameId, isBiddingOpen] = await client.queryMulti<[u64, Bool]>([
      client.query.miningSlot.nextFrameId as any,
      client.query.miningSlot.isNextSlotBiddingOpen,
    ]);
    if (nextFrameId.toNumber() === this.cohortStartingFrameId && isBiddingOpen.isTrue) {
      console.log('Bidding is still open, waiting for it to close');
      await new Promise<void>(async resolve => {
        const unsub = await client.query.miningSlot.isNextSlotBiddingOpen(isOpen => {
          if (isOpen.isFalse) {
            unsub();
            resolve();
          }
        });
      });
    }
    // wait for any pending request to finish updating stats
    void (await this.pendingRequest);

    const currentFrameId = await client.query.miningSlot.nextFrameId();
    let blockNumber: number;
    // go back to last block with this cohort
    if (currentFrameId.toNumber() > this.cohortStartingFrameId) {
      blockNumber =
        (await client.query.miningSlot.frameStartBlockNumbers().then(x => x[0]?.toNumber())) - 1;
    } else {
      blockNumber = await client.query.system.number().then(x => x.toNumber());
    }

    const blockHash = await client.rpc.chain.getBlockHash(blockNumber);
    const api = await client.at(blockHash);
    const rawBids = await api.query.miningSlot.bidsForNextSlotCohort();
    const currentTick = await api.query.ticks.currentTick().then(x => x.toNumber());
    this.updateBidList(rawBids, blockNumber, currentTick);

    console.log('Bidder stopped', {
      cohortStartingFrameId: this.cohortStartingFrameId,
      blockNumber,
      winningBids: this.winningBids,
    });

    return this.winningBids;
  }

  private async checkWinningBids() {
    if (this.isStopped) return;

    // don't process two bids at the same time
    if (this.pendingRequest) {
      console.log('Current bid is still in progress, skipping this check');
      return;
    }

    // if we submitted a bid more recently than the max bid tick, hold off
    if (this.currentBids.mostRecentBidTick < this.lastBidTick) {
      console.log(`Waiting for bids more recent than our last attempt.`, {
        ownAttemptedBidTick: this.lastBidTick,
        liveBidsTick: this.currentBids.mostRecentBidTick,
      });
      return;
    }
    const bids = [...this.currentBids.bids];
    const bidsAtTick = this.currentBids.atTick;
    const blockNumber = this.currentBids.atBlockNumber;
    const winningBids = bids.filter(x => this.myAddresses.has(x.address));
    if (winningBids.length >= this.subaccounts.length) {
      console.log(`No updates needed. Winning all remaining seats (${winningBids.length}).`);
      return;
    }

    console.log(
      `Checking bids for cohort ${this.cohortStartingFrameId}, Still trying for seats: ${this.subaccounts.length}`,
    );

    const winningAddresses = new Set(winningBids.map(x => x.address));
    let lowestBid: bigint;
    let myAllocatedBids = 0n;
    for (const bid of bids) {
      lowestBid ??= bid.bidMicrogons;
      if (this.myAddresses.has(bid.address)) {
        myAllocatedBids += bid.bidMicrogons;
      }
      // don't compete against own bids
      else {
        if (bid.bidMicrogons < lowestBid) {
          lowestBid = bid.bidMicrogons;
        }
      }
    }
    lowestBid ??= -this.options.bidIncrement;

    // 1. determine next bid based on current bids and settings
    let nextBid = lowestBid + this.options.bidIncrement;
    if (nextBid < this.options.minBid) {
      nextBid = this.options.minBid;
    }
    if (nextBid > this.options.maxBid) {
      nextBid = this.options.maxBid;
    }

    const fakeTx = await this.accountset.createMiningBidTx({
      subaccounts: this.subaccounts,
      bidAmount: nextBid,
    });
    let availableBalanceForBids = await this.accountset.submitterBalance();
    availableBalanceForBids += myAllocatedBids;

    const tip = this.options.tipPerTransaction ?? 0n;
    const feeEstimate = await fakeTx.feeEstimate(tip);
    const estimatedFeePlusTip = feeEstimate + tip;

    let budgetForSeats = this.options.maxBudget - estimatedFeePlusTip;
    if (budgetForSeats > availableBalanceForBids) {
      budgetForSeats = availableBalanceForBids - estimatedFeePlusTip;
    }
    if (nextBid < lowestBid) {
      console.log(
        `Next bid within parameters is ${formatArgons(nextBid)}, but it's not enough. Current lowest bid is ${formatArgons(lowestBid)}.`,
      );
      this.safeRecordParamsAdjusted({
        tick: bidsAtTick,
        blockNumber,
        maxSeats: 0,
        winningBidCount: winningBids.length,
        reason: 'max-bid-too-low',
        availableBalanceForBids,
      });
      return;
    }

    if (nextBid - lowestBid < Number(this.minIncrement)) {
      console.log(
        `Can't make any more bids for ${this.cohortStartingFrameId} with given constraints (next bid below min increment).`,
        {
          lowestCurrentBid: formatArgons(lowestBid),
          nextAttemptedBid: formatArgons(nextBid),
          maxBid: formatArgons(this.options.maxBid),
        },
      );
      this.safeRecordParamsAdjusted({
        tick: bidsAtTick,
        blockNumber,
        maxSeats: 0,
        winningBidCount: winningBids.length,
        reason: 'max-bid-too-low',
        availableBalanceForBids,
      });
      return;
    }

    const seatsInBudget =
      nextBid === 0n ? this.subaccounts.length : Number(budgetForSeats / nextBid);

    let accountsToUse = [...this.subaccounts];
    // 3. if we have more seats than we can afford, we need to remove some
    if (accountsToUse.length > seatsInBudget) {
      this.safeRecordParamsAdjusted({
        tick: bidsAtTick,
        blockNumber,
        maxSeats: this.subaccounts.length,
        winningBidCount: winningBids.length,
        reason:
          availableBalanceForBids - estimatedFeePlusTip < nextBid * BigInt(seatsInBudget)
            ? 'insufficient-balance'
            : 'max-budget-too-low',
        availableBalanceForBids,
      });
      // Sort accounts by winning bids first, then rebids, then by index
      accountsToUse.sort((a, b) => {
        const isWinningA = winningAddresses.has(a.address);
        const isWinningB = winningAddresses.has(b.address);
        if (isWinningA && !isWinningB) return -1;
        if (!isWinningA && isWinningB) return 1;

        if (a.isRebid && !b.isRebid) return -1;
        if (!a.isRebid && b.isRebid) return 1;
        return a.index - b.index;
      });
      // only keep the number of accounts we can afford
      accountsToUse.length = seatsInBudget;
    }
    if (accountsToUse.length > winningBids.length) {
      this.pendingRequest = this.submitBids(nextBid, accountsToUse);
    }
  }

  private async submitBids(microgonsPerSeat: bigint, subaccounts: { address: string }[]) {
    try {
      this.bidsAttempted += subaccounts.length;
      const submitter = await this.accountset.createMiningBidTx({
        subaccounts,
        bidAmount: microgonsPerSeat,
      });
      const tip = this.options.tipPerTransaction ?? 0n;
      const txResult = await submitter.submit({
        tip,
        useLatestNonce: true,
      });

      const bidError = await txResult.inBlockPromise
        .then(() => undefined)
        .catch((x: ExtrinsicError) => x);

      const client = await this.clientPromise;
      let api = txResult.includedInBlock ? await client.at(txResult.includedInBlock) : client;

      this.lastBidTick = await api.query.ticks.currentTick().then(x => x.toNumber());
      const blockNumber = await api.query.system.number().then(x => x.toNumber());
      const bidAtTick = this.lastBidTick;

      try {
        this.callbacks?.onBidsSubmitted?.({
          tick: bidAtTick,
          blockNumber,
          microgonsPerSeat,
          txFeePlusTip: txResult.finalFee ?? 0n,
          submittedCount: subaccounts.length,
        });
      } catch (error) {
        console.error('Error in onBidsSubmitted callback:', error);
      }

      const successfulBids = txResult.batchInterruptedIndex ?? subaccounts.length;

      this.txFees += txResult.finalFee ?? 0n;

      console.log('Result of bids for cohort', {
        successfulBids,
        bidsPlaced: subaccounts.length,
        bidPerSeat: formatArgons(microgonsPerSeat),
        bidAtTick,
      });

      if (bidError) {
        try {
          this.callbacks?.onBidsRejected?.({
            tick: bidAtTick,
            blockNumber,
            microgonsPerSeat,
            submittedCount: subaccounts.length,
            rejectedCount: subaccounts.length - successfulBids,
            bidError,
          });
        } catch (error) {
          console.error('Error in onBidsRejected callback:', error);
        }
        throw bidError;
      }
    } catch (err) {
      console.error(`Error bidding for cohort ${this.cohortStartingFrameId}:`, err);
    } finally {
      this.pendingRequest = undefined;
      // always delay after submitting
      this.scheduleEvaluation();
    }
  }

  private scheduleEvaluation() {
    if (this.isStopped) return;
    const millisPerTick = this.millisPerTick!;
    const delayTicks = Math.max(this.options.bidDelay, 1);
    const delay = delayTicks * millisPerTick;

    if (this.evaluateInterval) clearInterval(this.evaluateInterval);
    console.log(`Scheduling next evaluation in ${delay}ms`);
    this.evaluateInterval = setInterval(() => this.checkWinningBids().catch(console.error), delay);
  }

  private updateBidList(
    rawBids: Vec<ArgonPrimitivesBlockSealMiningRegistration>,
    blockNumber: number,
    tick: number,
  ) {
    try {
      let mostRecentBidTick = 0;
      let hasDiffs = this.currentBids.bids.length !== rawBids.length;
      const bids = [];
      for (let i = 0; i < rawBids.length; i += 1) {
        const rawBid = rawBids[i];
        const bidAtTick = rawBid.bidAtTick.toNumber();
        if (bidAtTick > mostRecentBidTick) {
          mostRecentBidTick = bidAtTick;
        }
        const address = rawBid.accountId.toHuman();
        const bidMicrogons = rawBid.bid.toBigInt();
        if (!hasDiffs) {
          const existing = this.currentBids.bids[i];
          hasDiffs = existing?.address !== address || existing?.bidMicrogons !== bidMicrogons;
        }
        bids.push({
          address,
          bidMicrogons,
          bidAtTick,
        });
      }

      if (blockNumber > this.currentBids.atBlockNumber && hasDiffs) {
        this.currentBids.bids = bids;
        this.currentBids.mostRecentBidTick = mostRecentBidTick;
        this.currentBids.atTick = tick;
        this.currentBids.atBlockNumber = blockNumber;
        this.winningBids = bids.filter(x => this.myAddresses.has(x.address));
        console.log('Now winning bids:', this.winningBids.length);
        if (this.callbacks?.onBidsUpdated) {
          this.callbacks.onBidsUpdated({
            bids: this.winningBids,
            atBlockNumber: blockNumber,
            tick: mostRecentBidTick,
          });
        }
      }
    } catch (err) {
      console.error('Error processing updated bids list:', err);
    }
  }

  private safeRecordParamsAdjusted(args: {
    tick: number;
    blockNumber: number;
    winningBidCount: number;
    maxSeats: number;
    reason: 'max-bid-too-low' | 'insufficient-balance' | 'max-budget-too-low';
    availableBalanceForBids: bigint;
  }) {
    try {
      this.callbacks?.onBidParamsAdjusted?.(args);
    } catch (err) {
      console.error('Error in onBidParamsAdjusted callback:', err);
    }
  }
}
