import { convertFixedU128ToBigNumber, ExtrinsicError, JsonExt } from './utils';
import { AccountId } from '@polkadot/types/interfaces/runtime';
import { Compact, u128 } from '@polkadot/types-codec';
import { ArgonClient } from './index';
import { ApiDecoration } from '@polkadot/api/types';

export enum SeatReductionReason {
  InsufficientFunds = 'InsufficientFunds',
  MaxBidTooLow = 'MaxBidTooLow',
  MaxBudgetTooLow = 'MaxBudgetTooLow',
}

export interface IBidHistoryEntry {
  cohortStartingFrameId: number;
  blockNumber: number;
  tick: number;
  // changes to the bid list
  bidChanges: {
    address: string;
    bidAmount: bigint;
    // null if no longer in the list
    bidPosition: number | null;
    // null if newly added
    prevPosition: number | null;
    // prev bid amount
    prevBidAmount?: bigint;
  }[];
  // activity by the bot
  myBidsPlaced?: {
    bids: number;
    bidPerSeat: bigint;
    txFeePlusTip: bigint;
    successfulBids: number;
    failureReason?: ExtrinsicError;
  };
  // my current winning number of bids
  winningSeats: number;
  // the max seats that are still in play
  maxSeatsInPlay: number;
  // did we reduce the max seats in play?
  maxSeatsReductionReason?: SeatReductionReason;
}

export class CohortBidderHistory {
  public bidHistory: IBidHistoryEntry[] = [];

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
  };
  private lastBids: { address: string; bid: bigint }[] = [];
  private readonly myAddresses: Set<string> = new Set();
  private maxSeatsInPlay: number = 0;

  constructor(
    readonly cohortStartingFrameId: number,
    readonly subaccounts: {
      index: number;
      isRebid: boolean;
      address: string;
    }[],
  ) {
    this.maxSeatsInPlay = this.subaccounts.length;
    this.subaccounts.forEach(x => {
      this.myAddresses.add(x.address);
    });
  }

  public async init(client: ArgonClient) {
    if (!this.stats.argonotsPerSeat) {
      const startingStats = await CohortBidderHistory.getStartingData(client);
      Object.assign(this.stats, startingStats);
    }
  }

  public maybeReducingSeats(
    maxSeats: number,
    reason: SeatReductionReason,
    historyEntry: IBidHistoryEntry,
  ): void {
    if (this.maxSeatsInPlay > maxSeats) {
      historyEntry.maxSeatsReductionReason = reason;
    }
    this.maxSeatsInPlay = maxSeats;
    historyEntry.maxSeatsInPlay = maxSeats;
  }

  public trackChange(
    next: { accountId: AccountId; bid: u128 | Compact<u128> }[],
    blockNumber: number,
    tick: number,
    isLastEntry = false,
  ): IBidHistoryEntry {
    let winningBids = 0;
    let totalArgonsBid = 0n;
    const nextEntrants: { address: string; bid: bigint }[] = [];
    for (const x of next) {
      const bid = x.bid.toBigInt();
      const address = x.accountId.toHuman();
      nextEntrants.push({ address, bid });
      if (this.myAddresses.has(address)) {
        winningBids++;
        totalArgonsBid += bid;
      }
    }
    this.stats.seatsWon = winningBids;
    this.stats.totalArgonsBid = totalArgonsBid;
    this.stats.lastBlockNumber = Math.max(blockNumber, this.stats.lastBlockNumber);

    const historyEntry: IBidHistoryEntry = {
      cohortStartingFrameId: this.cohortStartingFrameId,
      blockNumber,
      tick,
      bidChanges: [],
      winningSeats: winningBids,
      maxSeatsInPlay: this.maxSeatsInPlay,
    };
    const hasDiffs = JsonExt.stringify(nextEntrants) !== JsonExt.stringify(this.lastBids);

    if (!isLastEntry || hasDiffs) {
      this.bidHistory.unshift(historyEntry);
    }
    if (hasDiffs) {
      nextEntrants.forEach(({ address, bid }, i) => {
        const prevBidIndex = this.lastBids.findIndex(y => y.address === address);
        const entry: any = {
          address,
          bidAmount: bid,
          bidPosition: i,
          prevPosition: prevBidIndex === -1 ? null : prevBidIndex,
        };
        if (prevBidIndex !== -1) {
          const prevBidAmount = this.lastBids[prevBidIndex].bid;
          if (prevBidAmount !== bid) {
            entry.prevBidAmount = prevBidAmount;
          }
        }
        historyEntry.bidChanges.push(entry);
      });

      this.lastBids.forEach(({ address, bid }, i) => {
        const nextBid = nextEntrants.some(y => y.address === address);
        if (!nextBid) {
          historyEntry.bidChanges.push({
            address,
            bidAmount: bid,
            bidPosition: null,
            prevPosition: i,
          });
        }
      });
      this.lastBids = nextEntrants;
    }

    return historyEntry;
  }

  public onBidResult(
    historyEntry: IBidHistoryEntry,
    param: {
      txFeePlusTip: bigint;
      bidPerSeat: bigint;
      blockNumber: number | undefined;
      bidsAttempted: number;
      successfulBids: number;
      bidError?: ExtrinsicError;
    },
  ) {
    const { txFeePlusTip, bidPerSeat, bidsAttempted, successfulBids, blockNumber, bidError } =
      param;
    this.stats.fees += txFeePlusTip;
    this.stats.bidsAttempted += bidsAttempted;
    if (bidPerSeat > this.stats.maxBidPerSeat) {
      this.stats.maxBidPerSeat = bidPerSeat;
    }
    if (blockNumber !== undefined) {
      this.stats.lastBlockNumber = Math.max(blockNumber, this.stats.lastBlockNumber);
    }

    if (historyEntry.myBidsPlaced) {
      historyEntry.myBidsPlaced!.failureReason = bidError;
      historyEntry.myBidsPlaced!.successfulBids = successfulBids;
      historyEntry.myBidsPlaced!.txFeePlusTip = txFeePlusTip;
    }
  }

  public static async getStartingData(
    api: ApiDecoration<'promise'>,
  ): Promise<
    Pick<
      CohortBidderHistory['stats'],
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
}
