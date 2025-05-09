import type { Accountset } from './Accountset';
import {
  ArgonClient,
  ArgonPrimitivesBlockSealMiningRegistration,
  ExtrinsicError,
} from './index';
import { formatArgons } from './utils';
import { Bool, u64, Vec } from '@polkadot/types-codec';
import {
  CohortBidderHistory,
  IBidHistoryEntry,
  SeatReductionReason,
} from './CohortBidderHistory';

export class CohortBidder {
  public get client(): Promise<ArgonClient> {
    return this.accountset.client;
  }

  public get stats(): CohortBidderHistory['stats'] {
    return this.history.stats;
  }

  public get bidHistory(): CohortBidderHistory['bidHistory'] {
    return this.history.bidHistory;
  }

  private unsubscribe?: () => void;
  private pendingRequest: Promise<any> | undefined;
  private retryTimeout?: NodeJS.Timeout;
  private isStopped = false;
  private needsRebid = false;
  private lastBidTime = 0;
  private history: CohortBidderHistory;

  private millisPerTick?: number;

  private readonly myAddresses = new Set<string>();

  constructor(
    public accountset: Accountset,
    public cohortId: number,
    public subaccounts: { index: number; isRebid: boolean; address: string }[],
    public options: {
      minBid: bigint;
      maxBid: bigint;
      maxBudget: bigint;
      bidIncrement: bigint;
      bidDelay: number;
      tipPerTransaction?: bigint;
    },
  ) {
    this.history = new CohortBidderHistory(cohortId, subaccounts);
    this.subaccounts.forEach(x => {
      this.myAddresses.add(x.address);
    });
  }

  public async stop(): Promise<CohortBidder['stats']> {
    if (this.isStopped) return this.stats;
    this.isStopped = true;
    console.log('Stopping bidder for cohort', this.cohortId);
    clearTimeout(this.retryTimeout);
    if (this.unsubscribe) {
      this.unsubscribe();
    }
    const client = await this.client;
    const [nextCohortId, isBiddingOpen] = await client.queryMulti<[u64, Bool]>([
      client.query.miningSlot.nextCohortId as any,
      client.query.miningSlot.isNextSlotBiddingOpen,
    ]);
    if (nextCohortId.toNumber() === this.cohortId && isBiddingOpen.isTrue) {
      console.log('Bidding is still open, waiting for it to close');
      await new Promise<void>(async resolve => {
        const unsub = await client.query.miningSlot.isNextSlotBiddingOpen(
          isOpen => {
            if (isOpen.isFalse) {
              unsub();
              resolve();
            }
          },
        );
      });
    }
    // wait for any pending request to finish updating stats
    void (await this.pendingRequest);

    // go back to last block with this cohort
    let header = await client.rpc.chain.getHeader();
    while (true) {
      const api = await client.at(header.hash);
      const cohortId = await api.query.miningSlot.nextCohortId();
      if (cohortId.toNumber() === this.cohortId) {
        break;
      }
      header = await client.rpc.chain.getHeader(header.parentHash);
    }
    const api = await client.at(header.hash);
    const tick = await api.query.ticks.currentTick().then(x => x.toNumber());
    const cohort = await api.query.miningSlot.nextSlotCohort();

    this.history.trackChange(cohort, header.number.toNumber(), tick, true);
    console.log('Bidder stopped', {
      cohortId: this.cohortId,
      blockNumber: header.number.toNumber(),
      tick,
      cohort: cohort.map(x => ({
        address: x.accountId.toHuman(),
        bid: x.bid.toBigInt(),
      })),
    });

    return this.stats;
  }

  public async start() {
    console.log(`Starting cohort ${this.cohortId} bidder`, {
      maxBid: formatArgons(this.options.maxBid),
      minBid: formatArgons(this.options.minBid),
      bidIncrement: formatArgons(this.options.bidIncrement),
      maxBudget: formatArgons(this.options.maxBudget),
      bidDelay: this.options.bidDelay,
      subaccounts: this.subaccounts,
    });

    const client = await this.client;
    await this.history.init(client);
    this.millisPerTick ??= await client.query.ticks
      .genesisTicker()
      .then(x => x.tickDurationMillis.toNumber());

    this.unsubscribe = await client.queryMulti<
      [Vec<ArgonPrimitivesBlockSealMiningRegistration>, u64]
    >(
      [
        client.query.miningSlot.nextSlotCohort as any,
        client.query.miningSlot.nextCohortId as any,
      ],
      async ([next, nextCohortId]) => {
        if (nextCohortId.toNumber() === this.cohortId) {
          await this.checkSeats(next);
        }
      },
    );
  }

  private async checkSeats(next: ArgonPrimitivesBlockSealMiningRegistration[]) {
    if (this.isStopped) return;
    clearTimeout(this.retryTimeout);

    const client = await this.client;
    const bestBlock = await client.rpc.chain.getBlockHash();
    const api = await client.at(bestBlock);
    const blockNumber = await api.query.system.number().then(x => x.toNumber());
    if (this.bidHistory[0]?.blockNumber >= blockNumber) {
      return;
    }
    const tick = await api.query.ticks.currentTick().then(x => x.toNumber());
    const historyEntry = this.history.trackChange(next, blockNumber, tick);

    if (this.pendingRequest) return;

    const ticksSinceLastBid = Math.floor(
      (Date.now() - this.lastBidTime) / this.millisPerTick!,
    );
    if (ticksSinceLastBid < this.options.bidDelay) {
      this.retryTimeout = setTimeout(
        () => void this.checkCurrentSeats(),
        this.millisPerTick!,
      );
      return;
    }
    console.log(
      'Checking bids for cohort',
      this.cohortId,
      this.subaccounts.map(x => x.index),
    );

    const winningBids = historyEntry.winningSeats;
    this.needsRebid = winningBids < this.subaccounts.length;
    if (!this.needsRebid) return;

    const winningAddresses = new Set(next.map(x => x.accountId.toHuman()));
    let lowestBid = -this.options.bidIncrement;
    if (next.length) {
      for (let i = next.length - 1; i >= 0; i--) {
        // find the lowest bid that is not us
        if (!this.myAddresses.has(next[i].accountId.toHuman())) {
          lowestBid = next.at(i)!.bid.toBigInt();
          break;
        }
      }
    }
    const MIN_INCREMENT = 10_000n;

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
      sendRewardsToSeed: true,
    });
    let availableBalanceForBids = await api.query.system
      .account(this.accountset.txSubmitterPair.address)
      .then(x => x.data.free.toBigInt());

    // add our current balance used to the budget
    for (const bid of next) {
      if (this.myAddresses.has(bid.accountId.toHuman())) {
        availableBalanceForBids += bid.bid.toBigInt();
      }
    }
    const tip = this.options.tipPerTransaction ?? 0n;
    const feeEstimate = await fakeTx.feeEstimate(tip);
    const feePlusTip = feeEstimate + tip;

    let budgetForSeats = this.options.maxBudget - feePlusTip;
    if (budgetForSeats > availableBalanceForBids) {
      budgetForSeats = availableBalanceForBids - feePlusTip;
    }
    if (nextBid < lowestBid) {
      console.log(
        `Can't bid ${formatArgons(nextBid)}. Current lowest bid is ${formatArgons(
          lowestBid,
        )}.`,
      );
      this.history.maybeReducingSeats(
        winningBids,
        SeatReductionReason.MaxBidTooLow,
        historyEntry,
      );
      return;
    }

    if (nextBid - lowestBid < MIN_INCREMENT) {
      console.log(
        `Can't make any more bids for ${this.cohortId} with given constraints.`,
        {
          lowestCurrentBid: formatArgons(lowestBid),
          nextAttemptedBid: formatArgons(nextBid),
          maxBid: formatArgons(this.options.maxBid),
        },
      );
      this.history.maybeReducingSeats(
        winningBids,
        SeatReductionReason.MaxBidTooLow,
        historyEntry,
      );
      return;
    }

    const seatsInBudget =
      nextBid === 0n
        ? this.subaccounts.length
        : Number(budgetForSeats / nextBid);

    let accountsToUse = [...this.subaccounts];
    // 3. if we have more seats than we can afford, we need to remove some
    if (accountsToUse.length > seatsInBudget) {
      const reason =
        availableBalanceForBids - feePlusTip < nextBid * BigInt(seatsInBudget)
          ? SeatReductionReason.InsufficientFunds
          : SeatReductionReason.MaxBudgetTooLow;
      this.history.maybeReducingSeats(seatsInBudget, reason, historyEntry);
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
    if (accountsToUse.length > winningBids) {
      historyEntry.myBidsPlaced = {
        bids: accountsToUse.length,
        bidPerSeat: nextBid,
        txFeePlusTip: feePlusTip,
        successfulBids: 0,
      };
      this.pendingRequest = this.bid(nextBid, accountsToUse, historyEntry);
    } else if (historyEntry.bidChanges.length === 0) {
      this.history.bidHistory.shift();
    }
    this.needsRebid = false;
  }

  private async bid(
    bidPerSeat: bigint,
    subaccounts: { address: string }[],
    historyEntry: IBidHistoryEntry,
  ) {
    const prevLastBidTime = this.lastBidTime;
    try {
      this.lastBidTime = Date.now();
      const submitter = await this.accountset.createMiningBidTx({
        subaccounts,
        bidAmount: bidPerSeat,
        sendRewardsToSeed: true,
      });
      const tip = this.options.tipPerTransaction ?? 0n;
      const txResult = await submitter.submit({
        tip,
        useLatestNonce: true,
      });

      const bidError = await txResult.inBlockPromise
        .then(() => undefined)
        .catch((x: ExtrinsicError) => x);
      let blockNumber: number | undefined;
      if (txResult.includedInBlock) {
        const client = await this.client;
        const api = await client.at(txResult.includedInBlock);
        blockNumber = await api.query.system.number().then(x => x.toNumber());
      }

      const successfulBids =
        txResult.batchInterruptedIndex ?? subaccounts.length;
      this.history.onBidResult(historyEntry, {
        blockNumber,
        successfulBids,
        bidPerSeat,
        txFeePlusTip: txResult.finalFee ?? 0n,
        bidsAttempted: subaccounts.length,
        bidError,
      });

      console.log('Done creating bids for cohort', {
        successfulBids,
        bidPerSeat,
        blockNumber,
      });
      if (bidError) throw bidError;
    } catch (err) {
      this.lastBidTime = prevLastBidTime;
      console.error(`Error bidding for cohort ${this.cohortId}:`, err);
      clearTimeout(this.retryTimeout);
      this.retryTimeout = setTimeout(() => void this.checkCurrentSeats(), 1000);
    } finally {
      this.pendingRequest = undefined;
    }

    if (this.needsRebid) {
      this.needsRebid = false;
      await this.checkCurrentSeats();
    }
  }

  private async checkCurrentSeats() {
    const client = await this.client;
    const next = await client.query.miningSlot.nextSlotCohort();
    await this.checkSeats(next);
  }
}
