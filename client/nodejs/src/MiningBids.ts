import {
  type ArgonClient,
  type ArgonPrimitivesBlockSealMiningRegistration,
  formatArgons,
  type Bool,
  type u64,
} from './index';
import { printTable } from 'console-table-printer';
import type { IAddressNames } from './Accountset';

export class MiningBids {
  public nextCohort: {
    accountId: string;
    isOurs: string;
    bidAmount: bigint;
  }[] = [];

  constructor(
    readonly client: Promise<ArgonClient>,
    private shouldLog = true,
  ) {}

  public async maxCohortSize(): Promise<number> {
    const client = await this.client;
    return client.consts.miningSlot.maxCohortSize.toNumber();
  }

  public async onCohortChange(options: {
    onBiddingStart?: (cohortStartingFrameId: number) => Promise<void>;
    onBiddingEnd?: (cohortStartingFrameId: number) => Promise<void>;
  }): Promise<{ unsubscribe: () => void }> {
    const { onBiddingStart, onBiddingEnd } = options;
    const client = await this.client;
    let openCohortStartingFrameId = 0;
    const unsubscribe = await client.queryMulti<[Bool, u64]>(
      [
        client.query.miningSlot.isNextSlotBiddingOpen as any,
        client.query.miningSlot.nextFrameId as any,
      ],
      async ([isBiddingOpen, rawNextCohortStartingFrameId]) => {
        const nextFrameId = rawNextCohortStartingFrameId.toNumber();

        if (isBiddingOpen.isTrue) {
          if (openCohortStartingFrameId !== 0) {
            await onBiddingEnd?.(openCohortStartingFrameId);
          }
          openCohortStartingFrameId = nextFrameId;
          await onBiddingStart?.(nextFrameId);
        } else {
          await onBiddingEnd?.(nextFrameId);
          openCohortStartingFrameId = 0;
        }
      },
    );
    return { unsubscribe };
  }

  public async watch(
    accountNames: IAddressNames,
    blockHash?: Uint8Array,
    printFn?: (blockNumber: number) => void,
  ): Promise<{ unsubscribe: () => void }> {
    const client = await this.client;
    const api = blockHash ? await client.at(blockHash) : client;
    const unsubscribe = await api.query.miningSlot.bidsForNextSlotCohort(async next => {
      this.nextCohort = await Promise.all(next.map(x => this.toBid(accountNames, x)));
      if (!this.shouldLog) return;
      console.clear();
      const block = await client.query.system.number();
      if (!printFn) {
        console.log('At block', block.toNumber());
        this.print();
      } else {
        printFn(block.toNumber());
      }
    });
    return { unsubscribe };
  }

  public async loadAt(accountNames: IAddressNames, blockHash?: Uint8Array): Promise<void> {
    const client = await this.client;
    const api = blockHash ? await client.at(blockHash) : client;
    const nextCohort = await api.query.miningSlot.bidsForNextSlotCohort();
    this.nextCohort = await Promise.all(nextCohort.map(x => this.toBid(accountNames, x)));
  }

  private async toBid(
    accountNames: IAddressNames,
    bid: ArgonPrimitivesBlockSealMiningRegistration,
  ): Promise<MiningBids['nextCohort'][0]> {
    return {
      accountId: bid.accountId.toString(),
      isOurs: accountNames.get(bid.accountId.toString()) ?? 'n',
      bidAmount: bid.bid.toBigInt(),
    };
  }

  public print() {
    const bids = this.nextCohort.map(bid => {
      return {
        account: bid.accountId,
        isOurs: bid.isOurs,
        bidAmount: formatArgons(bid.bidAmount),
      };
    });
    if (bids.length) {
      console.log('\n\nMining Bids:');
      printTable(bids);
    }
  }
}
