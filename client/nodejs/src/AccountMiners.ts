import { Accountset } from './Accountset';
import { Header } from '@polkadot/types/interfaces/runtime';
import { GenericEvent } from '@polkadot/types';
import { BlockWatch } from './BlockWatch';
import { MiningRotations } from './MiningRotations';
import { createNanoEvents } from 'nanoevents';

export class AccountMiners {
  public events = createNanoEvents<{
    mined: (
      header: Header,
      earnings: {
        author: string;
        argons: bigint;
        argonots: bigint;
        cohortId: number;
        rotation: number;
      },
    ) => void;
    minted: (
      header: Header,
      minted: {
        accountId: string;
        argons: bigint;
        cohortId: number;
        rotation: number;
      },
    ) => void;
  }>();

  public miningRotations: MiningRotations;

  private trackedAccountsByAddress: {
    [address: string]: { cohortId: number; subaccountIndex: number };
  } = {};

  constructor(
    private accountset: Accountset,
    registeredMiners: {
      cohortId: number;
      address: string;
      subaccountIndex: number;
    }[],
    private options: { shouldLog: boolean } = { shouldLog: false },
  ) {
    this.miningRotations = new MiningRotations();
    for (const seat of registeredMiners) {
      this.trackedAccountsByAddress[seat.address] = {
        cohortId: seat.cohortId,
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
    const rotation = await this.miningRotations.getForTick(client, tick);
    let newMiners: { cohortId: number; addresses: string[] } | undefined;
    const dataByCohort: {
      rotation: number;
      [cohortId: number]: {
        argonsMinted: bigint;
        argonsMined: bigint;
        argonotsMined: bigint;
      };
    } = { rotation };
    for (const event of events) {
      if (client.events.miningSlot.NewMiners.is(event)) {
        newMiners = {
          cohortId: event.data.cohortId.toNumber(),
          addresses: event.data.newMiners.map(x => x.accountId.toHuman()),
        };
      }
      if (client.events.blockRewards.RewardCreated.is(event)) {
        const { rewards } = event.data;
        for (const reward of rewards) {
          const { argons, ownership } = reward;

          const entry = this.trackedAccountsByAddress[author];
          if (entry) {
            dataByCohort[entry.cohortId] ??= {
              argonsMinted: 0n,
              argonsMined: 0n,
              argonotsMined: 0n,
            };
            dataByCohort[entry.cohortId].argonotsMined += ownership.toBigInt();
            dataByCohort[entry.cohortId].argonsMined += argons.toBigInt();
            this.events.emit('mined', header, {
              author,
              argons: argons.toBigInt(),
              argonots: ownership.toBigInt(),
              cohortId: entry.cohortId,
              rotation,
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
            const { cohortId } = info;
            dataByCohort[cohortId] ??= {
              argonsMinted: 0n,
              argonsMined: 0n,
              argonotsMined: 0n,
            };
            dataByCohort[cohortId].argonsMinted += amountPerMiner;
            this.events.emit('minted', header, {
              accountId: address,
              argons: amountPerMiner,
              cohortId,
              rotation,
            });
          }
        }
      }
    }
    if (newMiners) {
      this.newCohortMiners(newMiners.cohortId, newMiners.addresses);
    }
    return dataByCohort;
  }

  private newCohortMiners(cohortId: number, addresses: string[]) {
    for (const [address, info] of Object.entries(
      this.trackedAccountsByAddress,
    )) {
      if (info.cohortId === cohortId - 10) {
        delete this.trackedAccountsByAddress[address];
      }
    }
    for (const address of addresses) {
      const entry = this.accountset.subAccountsByAddress[address];
      if (entry) {
        this.trackedAccountsByAddress[address] = {
          cohortId,
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
    const registered = seats.filter(x => x.cohortId !== undefined);
    return new AccountMiners(accountset, registered as any, {
      shouldLog: options.shouldLog ?? false,
    });
  }
}
