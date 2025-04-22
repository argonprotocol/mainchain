import type { Accountset } from './Accountset';
import {
  ArgonClient,
  ArgonPrimitivesBlockSealMiningRegistration,
  convertFixedU128ToBigNumber,
} from './index';
import { formatArgons } from './utils';
import { Bool, Compact, u128, u32, u64, Vec } from '@polkadot/types-codec';
import { AccountId } from '@polkadot/types/interfaces/runtime';
import { ApiDecoration } from '@polkadot/api/types';

export class CohortBidder {
  public get client(): Promise<ArgonClient> {
    return this.accountset.client;
  }

  public stats = {
    // number of seats won
    seats: 0,
    // sum of argons bid in successful bids
    totalArgonsBid: 0n,
    // total number of bids placed (includes 1 per seat)
    bids: 0,
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
    lastBlock: 0,
  };

  private unsubscribe?: () => void;
  private pendingRequest: Promise<any> | undefined;
  private retryTimeout?: NodeJS.Timeout;
  private isStopped = false;
  private needsRebid = false;
  private lastBidTime = 0;

  private millisPerTick?: number;

  private readonly allAddresses = new Set<string>();

  constructor(
    public accountset: Accountset,
    public cohortId: number,
    public subaccounts: { index: number; isRebid: boolean; address: string }[],
    public options: {
      minBid: bigint;
      maxBid: bigint;
      maxBalance: bigint;
      bidIncrement: bigint;
      bidDelay: number;
    },
  ) {
    this.subaccounts.forEach(x => {
      this.allAddresses.add(x.address);
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
    const [nextCohort, nextCohortId, blockNumber] = await new Promise<
      [Vec<ArgonPrimitivesBlockSealMiningRegistration>, u64, u32]
    >(async resolve => {
      // wait for bidding to be complete
      const unsub = await client.queryMulti<
        [Vec<ArgonPrimitivesBlockSealMiningRegistration>, u64, Bool, u32]
      >(
        [
          client.query.miningSlot.nextSlotCohort as any,
          client.query.miningSlot.nextCohortId as any,
          client.query.miningSlot.isNextSlotBiddingOpen as any,
          client.query.system.number as any,
        ],
        ([nextCohort, nextCohortId, isBiddingStillOpen, blockNumber]) => {
          if (
            nextCohortId.toNumber() !== this.cohortId ||
            isBiddingStillOpen.isFalse
          ) {
            unsub();
            resolve([nextCohort, nextCohortId, blockNumber]);
          }
        },
      );
    });
    // wait for any pending request to finish updating stats
    void (await this.pendingRequest);

    if (nextCohortId.toNumber() === this.cohortId) {
      console.log('Bidder updating stats with bid queue');
      this.updateStats(nextCohort);
    } else {
      const bestBlock = await client.rpc.chain.getBlockHash();
      const api = await client.at(bestBlock);
      const wonIndices = await api.query.miningSlot.accountIndexLookup
        .multi([...this.allAddresses])
        .then(x => x.filter(x => x.isSome).map(x => x.value));
      const wonSeats = await api.query.miningSlot.activeMinersByIndex
        .multi(wonIndices)
        .then(x =>
          x
            .filter(
              x => x.isSome && x.value.cohortId.toNumber() === this.cohortId,
            )
            .map(x => x.value),
        );

      console.log('Bidder updating stats with finalized cohort');

      this.updateStats(wonSeats);
    }
    this.stats.lastBlock = Math.max(
      blockNumber.toNumber(),
      this.stats.lastBlock,
    );
    return this.stats;
  }

  public static async getStartingData(
    api: ApiDecoration<'promise'>,
  ): Promise<
    Pick<
      CohortBidder['stats'],
      'argonotUsdPrice' | 'argonotsPerSeat' | 'cohortArgonsPerBlock'
    >
  > {
    const argonotPrice = await api.query.priceIndex.current();
    let argonotUsdPrice = 0;
    if (argonotPrice.isSome) {
      argonotUsdPrice = convertFixedU128ToBigNumber(
        argonotPrice.unwrap().argonotUsdPrice.toBigInt(),
      ).toNumber();
    }

    const argonotsPerSeat = await api.query.miningSlot
      .argonotsPerMiningSeat()
      .then(x => x.toBigInt());
    const cohortArgonsPerBlock = await api.query.blockRewards
      .argonsPerBlock()
      .then(x => x.toBigInt());
    return { argonotsPerSeat, argonotUsdPrice, cohortArgonsPerBlock };
  }

  public async start() {
    console.log(`Starting cohort ${this.cohortId} bidder`, {
      maxBid: formatArgons(this.options.maxBid),
      minBid: formatArgons(this.options.minBid),
      bidIncrement: formatArgons(this.options.bidIncrement),
      maxBalance: formatArgons(this.options.maxBalance),
      bidDelay: this.options.bidDelay,
      subaccounts: this.subaccounts,
    });

    const client = await this.client;
    if (!this.stats.argonotsPerSeat) {
      const startingStats = await CohortBidder.getStartingData(client);
      Object.assign(this.stats, startingStats);
    }

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

  private updateStats(
    next: Iterable<{ accountId: AccountId; bid: u128 | Compact<u128> }>,
  ) {
    let seats = 0;
    let totalArgonsBid = 0n;
    for (const x of next) {
      if (this.allAddresses.has(x.accountId.toHuman())) {
        seats++;
        totalArgonsBid += x.bid.toBigInt();
      }
    }
    this.stats.seats = seats;
    this.stats.totalArgonsBid = totalArgonsBid;
  }

  private async checkSeats(next: ArgonPrimitivesBlockSealMiningRegistration[]) {
    if (this.isStopped || this.pendingRequest) return;

    const ticksSinceLastBid = Math.floor(
      (Date.now() - this.lastBidTime) / this.millisPerTick!,
    );
    if (ticksSinceLastBid < this.options.bidDelay) {
      return;
    }
    console.log(
      'Checking bids for cohort',
      this.cohortId,
      this.subaccounts.map(x => x.index),
    );
    this.updateStats(next);

    this.needsRebid = this.subaccounts.some(
      x => !next.some(y => y.accountId.toHuman() === x.address),
    );
    if (!this.needsRebid) return;

    const lowestBid = next.at(-1)?.bid.toBigInt() ?? -this.options.bidIncrement;
    const MIN_INCREMENT = 10_000n;
    // 1. determine next bid based on current bids and settings
    let nextBid = lowestBid + this.options.bidIncrement;
    if (nextBid < this.options.minBid) {
      nextBid = this.options.minBid;
    }
    if (nextBid > this.options.maxBid) {
      nextBid = this.options.maxBid;
      if (nextBid - lowestBid < MIN_INCREMENT) {
        console.log(
          `Can't make any more bids for ${this.cohortId} with given constraints.`,
          {
            lowestCurrentBid: formatArgons(lowestBid),
            nextAttemptedBid: formatArgons(nextBid),
            maxBid: formatArgons(this.options.maxBid),
          },
        );
        return;
      }
    }
    if (nextBid < lowestBid) {
      console.log(
        `Can't bid ${formatArgons(nextBid)}. Current lowest bid is ${formatArgons(
          lowestBid,
        )}.`,
      );
      return;
    }

    // 2. how many seats fit into this budget?
    const seatsInBudget =
      nextBid === 0n
        ? this.subaccounts.length
        : Number(this.options.maxBalance / nextBid);
    if (seatsInBudget <= 0) {
      console.log(
        `Can't afford any seats at ${formatArgons(nextBid)}. Would exceed our max balance of ${formatArgons(this.options.maxBalance)}.`,
      );
      return;
    }
    // 3. if we have more seats than we can afford, we need to remove some
    if (this.subaccounts.length > seatsInBudget) {
      const toKeep: CohortBidder['subaccounts'] = [];

      // first add rebids
      for (const account of this.subaccounts) {
        if (toKeep.length >= seatsInBudget) break;
        if (account.isRebid) {
          toKeep.push(account);
        }
      }
      // if we still have space, add non-rebids
      for (const account of this.subaccounts) {
        if (toKeep.length >= seatsInBudget) break;
        if (!account.isRebid) {
          toKeep.push(account);
        }
      }

      const removedIndices = this.subaccounts
        .filter(x => !toKeep.some(y => y.index === x.index))
        .map(x => x.index);
      this.subaccounts = toKeep;
      console.log('Had to remove some subaccounts to fit in budget:', {
        removedIndices,
        seatsInBudget,
        budget: formatArgons(this.options.maxBalance),
      });
    }
    this.pendingRequest = this.bid(
      nextBid,
      this.subaccounts.map(x => x.index),
    );
    this.needsRebid = false;
  }

  private async bid(bidPerSeat: bigint, subaccountRange: number[]) {
    if (!subaccountRange.length) return;
    const prevLastBidTime = this.lastBidTime;
    try {
      this.lastBidTime = Date.now();
      const result = await this.accountset.createMiningBids({
        subaccountRange,
        bidAmount: bidPerSeat,
        sendRewardsToSeed: true,
      });
      if (result.blockHash) {
        const client = await this.client;
        const api = await client.at(result.blockHash);
        this.stats.lastBlock = await api.query.system
          .number()
          .then(x => x.toNumber());
      }
      this.stats.fees += result.finalFee ?? 0n;
      this.stats.bids += subaccountRange.length;
      if (bidPerSeat > this.stats.maxBidPerSeat) {
        this.stats.maxBidPerSeat = bidPerSeat;
      }

      console.log('Done creating bids for cohort', this.cohortId);
      if (result.bidError) throw result.bidError;
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
