import type { Accountset } from './Accountset';
import {
  ArgonClient,
  type ArgonPrimitivesBlockSealMiningRegistration,
  convertFixedU128ToBigNumber,
  ExtrinsicError,
} from './index';
import { formatArgons } from './utils';
import { Bool, Compact, u128, u64, Vec } from '@polkadot/types-codec';
import { AccountId } from '@polkadot/types/interfaces/runtime';

export class CohortBidder {
  public get client(): Promise<ArgonClient> {
    return this.accountset.client;
  }

  public stats = {
    // number of seats won
    seatsWon: 0,
    // sum of argons bid in successful bids
    totalArgonsBid: 0n,
    // total number of bids placed (includes 1 per seat)
    bidsAttempted: 0,
    // fees including the tip
    fees: 0n,
    // Max bid per seat
    maxBidPerSeat: 0n,
    // The cost in argonots of each seat
    argonotsPerSeat: 0n,
    // The argonot price in USD for cost basis
    argonotUsdPrice: 0.0,
    // The cohort expected argons per block
    cohortArgonsPerBlock: 0n,
    // The last block that bids are synced to
    lastBlockNumber: 0,
    // next cohort size
    nextCohortSize: 0,
  };

  private unsubscribe?: () => void;
  private pendingRequest: Promise<any> | undefined;
  private retryTimeout?: NodeJS.Timeout;
  private isStopped = false;
  private needsRebid = false;
  private lastBidTime = 0;

  private millisPerTick?: number;

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
    const argonotPrice = await client.query.priceIndex.current();
    if (argonotPrice.isSome) {
      this.stats.argonotUsdPrice = convertFixedU128ToBigNumber(
        argonotPrice.unwrap().argonotUsdPrice.toBigInt(),
      ).toNumber();
    }

    this.stats.nextCohortSize = await client.query.miningSlot
      .nextCohortSize()
      .then(x => x.toNumber());
    this.stats.argonotsPerSeat = await client.query.miningSlot
      .argonotsPerMiningSeat()
      .then(x => x.toBigInt());
    this.stats.cohortArgonsPerBlock = await client.query.blockRewards
      .argonsPerBlock()
      .then(x => x.toBigInt());
    if (this.subaccounts.length > this.stats.nextCohortSize) {
      console.info(
        `Cohort size ${this.stats.nextCohortSize} is less than provided subaccounts ${this.subaccounts.length}.`,
      );
      this.subaccounts.length = this.stats.nextCohortSize;
    }
    this.millisPerTick ??= await client.query.ticks
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

  public async stop(): Promise<CohortBidder['stats']> {
    if (this.isStopped) return this.stats;
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
    while (true) {
      const api = await client.at(header.hash);
      const cohortStartingFrameId = await api.query.miningSlot.nextFrameId();
      if (cohortStartingFrameId.toNumber() === this.cohortStartingFrameId) {
        break;
      }
      header = await client.rpc.chain.getHeader(header.parentHash);
    }
    const api = await client.at(header.hash);
    const tick = await api.query.ticks.currentTick().then(x => x.toNumber());
    const bids = await api.query.miningSlot.bidsForNextSlotCohort();

    this.updateSeatsWon(bids, header.number.toNumber());
    console.log('Bidder stopped', {
      cohortStartingFrameId: this.cohortStartingFrameId,
      blockNumber: header.number.toNumber(),
      tick,
      bids: bids.map(x => ({
        address: x.accountId.toHuman(),
        bid: x.bid.toBigInt(),
      })),
    });

    return this.stats;
  }

  private async checkWinningBids(bids: ArgonPrimitivesBlockSealMiningRegistration[]) {
    if (this.isStopped) return;
    clearTimeout(this.retryTimeout);

    const client = await this.client;
    const bestBlock = await client.rpc.chain.getBlockHash();
    const api = await client.at(bestBlock);
    const blockNumber = await api.query.system.number().then(x => x.toNumber());
    if (this.stats.lastBlockNumber >= blockNumber) {
      return;
    }
    this.updateSeatsWon(bids, blockNumber);

    if (this.pendingRequest) return;

    const ticksSinceLastBid = Math.floor((Date.now() - this.lastBidTime) / this.millisPerTick!);
    if (ticksSinceLastBid < this.options.bidDelay) {
      this.retryTimeout = setTimeout(() => void this.checkCurrentSeats(), this.millisPerTick!);
      return;
    }
    console.log(
      'Checking bids for cohort',
      this.cohortStartingFrameId,
      this.subaccounts.map(x => x.index),
    );

    const winningBids = this.stats.seatsWon;
    this.needsRebid = winningBids < this.subaccounts.length;
    if (!this.needsRebid) return;

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

    if (nextBid - lowestBid < MIN_INCREMENT) {
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

      this.stats.fees += txResult.finalFee ?? 0n;
      this.stats.bidsAttempted += subaccounts.length;
      if (bidPerSeat > this.stats.maxBidPerSeat) {
        this.stats.maxBidPerSeat = bidPerSeat;
      }
      if (blockNumber !== undefined) {
        this.stats.lastBlockNumber = Math.max(blockNumber, this.stats.lastBlockNumber);
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

  private updateSeatsWon(
    next: { accountId: AccountId; bid: u128 | Compact<u128> }[],
    blockNumber: number,
  ): void {
    let winningBids = 0;
    let totalArgonsBid = 0n;
    for (const x of next) {
      const bid = x.bid.toBigInt();
      const address = x.accountId.toHuman();
      if (this.myAddresses.has(address)) {
        winningBids++;
        totalArgonsBid += bid;
      }
    }
    this.stats.seatsWon = winningBids;
    this.stats.totalArgonsBid = totalArgonsBid;
    this.stats.lastBlockNumber = Math.max(blockNumber, this.stats.lastBlockNumber);
  }

  private async checkCurrentSeats() {
    const client = await this.client;
    const bids = await client.query.miningSlot.bidsForNextSlotCohort();
    await this.checkWinningBids(bids);
  }
}
