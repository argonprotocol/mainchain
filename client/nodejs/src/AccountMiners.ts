import { Accountset } from './Accountset';
import { Header } from '@polkadot/types/interfaces/runtime';
import { GenericEvent } from '@polkadot/types';
import { BlockWatch } from './BlockWatch';
import { FrameCalculator } from './FrameCalculator';
import { createNanoEvents } from 'nanoevents';

export class AccountMiners {
  public events = createNanoEvents<{
    mined: (
      header: Header,
      earnings: {
        author: string;
        argons: bigint;
        argonots: bigint;
        forCohortWithStartingFrameId: number;
        duringFrameId: number;
      },
    ) => void;
    minted: (
      header: Header,
      minted: {
        accountId: string;
        argons: bigint;
        forCohortWithStartingFrameId: number;
        duringFrameId: number;
      },
    ) => void;
  }>();

  public frameCalculator: FrameCalculator;

  private trackedAccountsByAddress: {
    [address: string]: {
      startingFrameId: number;
      subaccountIndex: number;
    };
  } = {};

  constructor(
    private accountset: Accountset,
    registeredMiners: {
      startingFrameId: number;
      address: string;
      subaccountIndex: number;
    }[],
    private options: { shouldLog: boolean } = { shouldLog: false },
  ) {
    this.frameCalculator = new FrameCalculator();
    for (const seat of registeredMiners) {
      this.trackedAccountsByAddress[seat.address] = {
        startingFrameId: seat.startingFrameId,
        subaccountIndex: seat.subaccountIndex,
      };
    }
  }

  public async watch(): Promise<BlockWatch> {
    const blockWatch = new BlockWatch(this.accountset.client, {
      shouldLog: this.options.shouldLog,
    });
    blockWatch.events.on('block', this.onBlock.bind(this));
    await blockWatch.start();
    return blockWatch;
  }

  public async onBlock(
    header: Header,
    digests: { author: string; tick: number },
    events: GenericEvent[],
  ) {
    const { author, tick } = digests;
    if (author) {
      const voteAuthor = this.trackedAccountsByAddress[author];
      if (voteAuthor && this.options.shouldLog) {
        console.log(
          '> Our vote author',
          this.accountset.accountRegistry.getName(author),
        );
      }
    } else {
      console.warn('> No vote author found');
    }
    const client = await this.accountset.client;
    const currentFrameId = await this.frameCalculator.getForTick(client, tick);
    let newMiners: { frameId: number; addresses: string[] } | undefined;
    const dataByCohort: {
      duringFrameId: number;
      [cohortStartingFrameId: number]: {
        argonsMinted: bigint;
        argonsMined: bigint;
        argonotsMined: bigint;
      };
    } = { duringFrameId: currentFrameId };
    for (const event of events) {
      if (client.events.miningSlot.NewMiners.is(event)) {
        newMiners = {
          frameId: event.data.frameId.toNumber(),
          addresses: event.data.newMiners.map(x => x.accountId.toHuman()),
        };
      }
      if (client.events.blockRewards.RewardCreated.is(event)) {
        const { rewards } = event.data;
        for (const reward of rewards) {
          const { argons, ownership } = reward;

          const entry = this.trackedAccountsByAddress[author];
          if (entry) {
            dataByCohort[entry.startingFrameId] ??= {
              argonsMinted: 0n,
              argonsMined: 0n,
              argonotsMined: 0n,
            };
            dataByCohort[entry.startingFrameId].argonotsMined +=
              ownership.toBigInt();
            dataByCohort[entry.startingFrameId].argonsMined +=
              argons.toBigInt();
            this.events.emit('mined', header, {
              author,
              argons: argons.toBigInt(),
              argonots: ownership.toBigInt(),
              forCohortWithStartingFrameId: entry.startingFrameId,
              duringFrameId: currentFrameId,
            });
          }
        }
      }
      if (client.events.mint.MiningMint.is(event)) {
        const { perMiner } = event.data;
        const amountPerMiner = perMiner.toBigInt();
        if (amountPerMiner > 0n) {
          for (const [address, info] of Object.entries(
            this.trackedAccountsByAddress,
          )) {
            const { startingFrameId } = info;
            dataByCohort[startingFrameId] ??= {
              argonsMinted: 0n,
              argonsMined: 0n,
              argonotsMined: 0n,
            };
            dataByCohort[startingFrameId].argonsMinted += amountPerMiner;
            this.events.emit('minted', header, {
              accountId: address,
              argons: amountPerMiner,
              forCohortWithStartingFrameId: startingFrameId,
              duringFrameId: currentFrameId,
            });
          }
        }
      }
    }
    if (newMiners) {
      this.newCohortMiners(newMiners.frameId, newMiners.addresses);
    }
    return dataByCohort;
  }

  private newCohortMiners(frameId: number, addresses: string[]) {
    for (const [address, info] of Object.entries(
      this.trackedAccountsByAddress,
    )) {
      if (info.startingFrameId === frameId - 10) {
        delete this.trackedAccountsByAddress[address];
      }
    }
    for (const address of addresses) {
      const entry = this.accountset.subAccountsByAddress[address];
      if (entry) {
        this.trackedAccountsByAddress[address] = {
          startingFrameId: frameId,
          subaccountIndex: entry.index,
        };
      }
    }
  }

  public static async loadAt(
    accountset: Accountset,
    options: {
      blockHash?: Uint8Array;
      shouldLog?: boolean;
    } = {},
  ) {
    const seats = await accountset.miningSeats(options.blockHash);
    const registered = seats.filter(x => x.seat !== undefined);
    return new AccountMiners(accountset, registered as any, {
      shouldLog: options.shouldLog ?? false,
    });
  }
}
