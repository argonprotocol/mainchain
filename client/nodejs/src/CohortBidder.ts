import type { Accountset } from './Accountset';
import {
  ArgonClient,
  type ArgonPrimitivesBlockSealMiningRegistration,
  ExtrinsicError,
} from './index';
import { formatArgons } from './utils';
import { Bool, Compact, u128, u64, Vec } from '@polkadot/types-codec';
import { AccountId } from '@polkadot/types/interfaces/runtime';

export class CohortBidder {
  public get client(): Promise<ArgonClient> {
    return this.accountset.client;
  }

  public txFees = 0n;
  public winningBids: { address: string; bid: bigint }[] = [];

  private unsubscribe?: () => void;
  private pendingRequest: Promise<any> | undefined;
  private retryTimeout?: NodeJS.Timeout;
  private isStopped = false;
  private needsRebid = false;
  private lastBidTime = 0;

  private millisPerTick?: number;
  private minIncrement = 10_000n;
  private nextCohortSize?: number;
  private lastBidBlockNumber: number = 0;

  private readonly myAddresses = new Set<string>();

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

    const client = await this.client;

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

    this.unsubscribe = await client.queryMulti<
      [Vec<ArgonPrimitivesBlockSealMiningRegistration>, u64]
    >(
      [
        client.query.miningSlot.bidsForNextSlotCohort as any,
        client.query.miningSlot.nextFrameId as any,
      ],
      async ([bids, nextFrameId]) => {
        if (nextFrameId.toNumber() === this.cohortStartingFrameId) {
          await this.checkWinningBids(bids);
        }
      },
    );
  }

  public async stop(): Promise<CohortBidder['winningBids']> {
    if (this.isStopped) return this.winningBids;
    this.isStopped = true;
    console.log('Stopping bidder for cohort', this.cohortStartingFrameId);
    clearTimeout(this.retryTimeout);
    if (this.unsubscribe) {
      this.unsubscribe();
    }
    const client = await this.client;
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

    // go back to last block with this cohort
    let header = await client.rpc.chain.getHeader();
    let api = await client.at(header.hash);
    while (true) {
      const cohortStartingFrameId = await api.query.miningSlot.nextFrameId();
      if (cohortStartingFrameId.toNumber() === this.cohortStartingFrameId) {
        break;
      }
      header = await client.rpc.chain.getHeader(header.parentHash);
      api = await client.at(header.hash);
    }
    const bids = await api.query.miningSlot.bidsForNextSlotCohort();

    this.updateSeatsWon(bids);
    console.log('Bidder stopped', {
      cohortStartingFrameId: this.cohortStartingFrameId,
      blockNumber: header.number.toNumber(),
      bids: this.winningBids,
    });

    return this.winningBids;
  }

  private async checkWinningBids(bids: ArgonPrimitivesBlockSealMiningRegistration[]) {
    if (this.isStopped) return;
    clearTimeout(this.retryTimeout);

    this.updateSeatsWon(bids);
    const winningBids = this.winningBids.length;
    this.needsRebid = winningBids < this.subaccounts.length;

    const client = await this.client;
    const bestBlock = await client.rpc.chain.getBlockHash();
    const api = await client.at(bestBlock);
    const blockNumber = await api.query.system.number().then(x => x.toNumber());

    if (this.lastBidBlockNumber >= blockNumber) return;
    if (this.pendingRequest) return;

    const ticksSinceLastBid = Math.floor((Date.now() - this.lastBidTime) / this.millisPerTick!);
    if (ticksSinceLastBid < this.options.bidDelay && this.needsRebid) {
      this.retryTimeout = setTimeout(() => void this.checkCurrentSeats(), this.millisPerTick!);
      return;
    }
    if (!this.needsRebid) return;

    console.log(
      'Checking bids for cohort',
      this.cohortStartingFrameId,
      this.subaccounts.map(x => x.index),
    );

    const winningAddresses = new Set(bids.map(x => x.accountId.toHuman()));
    let lowestBid = -this.options.bidIncrement;
    if (bids.length) {
      for (let i = bids.length - 1; i >= 0; i--) {
        // find the lowest bid that is not us
        if (!this.myAddresses.has(bids[i].accountId.toHuman())) {
          lowestBid = bids.at(i)!.bid.toBigInt();
          break;
        }
      }
    }

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
    let availableBalanceForBids = await api.query.system
      .account(this.accountset.txSubmitterPair.address)
      .then(x => x.data.free.toBigInt());

    // add our current balance used to the budget
    for (const bid of bids) {
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
        `Can't bid ${formatArgons(nextBid)}. Current lowest bid is ${formatArgons(lowestBid)}.`,
      );
      return;
    }

    if (nextBid - lowestBid < Number(this.minIncrement)) {
      console.log(
        `Can't make any more bids for ${this.cohortStartingFrameId} with given constraints.`,
        {
          lowestCurrentBid: formatArgons(lowestBid),
          nextAttemptedBid: formatArgons(nextBid),
          maxBid: formatArgons(this.options.maxBid),
        },
      );
      return;
    }

    const seatsInBudget =
      nextBid === 0n ? this.subaccounts.length : Number(budgetForSeats / nextBid);

    let accountsToUse = [...this.subaccounts];
    // 3. if we have more seats than we can afford, we need to remove some
    if (accountsToUse.length > seatsInBudget) {
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
      this.pendingRequest = this.bid(nextBid, accountsToUse);
    }
    this.needsRebid = false;
  }

  private async bid(bidPerSeat: bigint, subaccounts: { address: string }[]) {
    const prevLastBidTime = this.lastBidTime;
    try {
      this.lastBidTime = Date.now();
      const submitter = await this.accountset.createMiningBidTx({
        subaccounts,
        bidAmount: bidPerSeat,
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

      const successfulBids = txResult.batchInterruptedIndex ?? subaccounts.length;

      this.txFees += txResult.finalFee ?? 0n;
      if (blockNumber !== undefined) {
        this.lastBidBlockNumber = Math.max(blockNumber, this.lastBidBlockNumber);
      }

      console.log('Done creating bids for cohort', {
        successfulBids,
        bidPerSeat,
        blockNumber,
      });
      if (bidError) throw bidError;
    } catch (err) {
      this.lastBidTime = prevLastBidTime;
      console.error(`Error bidding for cohort ${this.cohortStartingFrameId}:`, err);
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

  private updateSeatsWon(next: { accountId: AccountId; bid: u128 | Compact<u128> }[]): void {
    this.winningBids.length = 0;
    for (const x of next) {
      const bid = x.bid.toBigInt();
      const address = x.accountId.toHuman();
      if (this.myAddresses.has(address)) {
        this.winningBids.push({ address, bid });
      }
    }
  }

  private async checkCurrentSeats() {
    const client = await this.client;
    const bids = await client.query.miningSlot.bidsForNextSlotCohort();
    await this.checkWinningBids(bids);
  }
}
