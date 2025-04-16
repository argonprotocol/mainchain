import { Accountset, type ArgonClient, type GenericEvent } from './index';
import type TypedEventEmitter from 'typed-emitter';
import EventEmitter from 'node:events';
import type { Header, SignedBlock } from '@polkadot/types/interfaces';
import { eventDataToJson, formatArgons } from './utils';

export type BlockWatchEvents = {
  'vaults-updated': (blockHash: Uint8Array, vaultIds: Set<number>) => void;
  'bitcoin-verified': (
    blockHash: Uint8Array,
    lockedBitcoin: { utxoId: number; vaultId: number; amount: bigint },
  ) => void;
  'mining-bid': (
    blockHash: Uint8Array,
    bid: { amount: bigint; accountId: string },
  ) => void;
  'mining-bid-ousted': (
    blockHash: Uint8Array,
    bid: {
      preservedArgonotHold: boolean;
      accountId: string;
    },
  ) => void;
  'accountset-authored': (
    blockHash: Uint8Array,
    earnings: {
      author: string;
      argons: bigint;
      argonots: bigint;
      cohortId: number;
    },
  ) => void;
  'accountset-minted': (
    blockHash: Uint8Array,
    minted: {
      accountId: string;
      argons: bigint;
      cohortId: number;
    },
  ) => void;
  event: (blockHash: Uint8Array, event: GenericEvent) => void;
};

export class BlockWatch {
  public readonly events =
    new EventEmitter() as TypedEventEmitter<BlockWatchEvents>;
  public readonly obligationsById: {
    [obligationId: number]: {
      vaultId: number;
      amount: bigint;
      utxoId?: number;
    };
  } = {};
  public readonly obligationIdByUtxoId: {
    [utxoId: number]: number;
  } = {};
  private unsubscribe?: () => void;

  private trackedAccountsByAddress: {
    [address: string]: { cohortId: number; subaccountIndex: number };
  } = {};

  constructor(
    private readonly mainchain: Promise<ArgonClient>,
    private monitorAccounts?: Accountset,
    private shouldLog = true,
  ) {}

  public stop() {
    if (this.unsubscribe) {
      this.unsubscribe();
      this.unsubscribe = undefined;
    }
  }

  public async start() {
    if (this.monitorAccounts) {
      const seats = await this.monitorAccounts.miningSeats();
      for (const seat of seats) {
        if (seat.cohortId === undefined) continue;
        this.trackedAccountsByAddress[seat.address] = {
          cohortId: seat.cohortId,
          subaccountIndex: seat.subaccountIndex,
        };
      }
    }
    await this.watchBlocks();
  }

  private newCohortMiners(cohortId: number, addresses: string[]) {
    if (!this.monitorAccounts) return;

    for (const [address, info] of Object.entries(
      this.trackedAccountsByAddress,
    )) {
      if (info.cohortId === cohortId - 10) {
        delete this.trackedAccountsByAddress[address];
      }
    }
    for (const address of addresses) {
      const entry = this.monitorAccounts.subAccountsByAddress[address];
      if (entry) {
        console.log('new miners has subaccount', address);
        this.trackedAccountsByAddress[address] = {
          cohortId,
          subaccountIndex: entry.index,
        };
      }
    }
  }

  private async watchBlocks() {
    const client = await this.mainchain;
    this.unsubscribe = await client.rpc.chain.subscribeNewHeads(
      async header => {
        try {
          await this.processBlock(header);
        } catch (e) {
          console.error('Error processing block', e);
        }
      },
    );
  }

  private async processBlock(header: Header) {
    const client = await this.mainchain;

    if (this.shouldLog) {
      console.log(`-------------------------------------
BLOCK #${header.number}, ${header.hash.toHuman()}`);
    }
    const blockHash = header.hash;
    const api = await client.at(blockHash);
    const isBlockVote = await api.query.blockSeal.isBlockFromVoteSeal();
    if (!isBlockVote) {
      console.warn('> Compute reactivated!');
    }
    const events = await api.query.system.events();
    const reloadVaults = new Set<number>();
    let block: SignedBlock | undefined = undefined;

    const author =
      header.digest.logs
        .map(x => {
          if (x.isPreRuntime) {
            const [engineId, data] = x.asPreRuntime;
            if (engineId.toString() === 'pow_') {
              return client.createType('AccountId32', data).toHuman();
            }
          }
          return undefined;
        })
        .find(Boolean) ?? '';

    const voteAuthor = this.trackedAccountsByAddress[author];
    if (voteAuthor && this.shouldLog) {
      console.log(
        '> Our vote author',
        this.monitorAccounts!.accountRegistry.getName(author),
      );
    }

    for (const { event, phase } of events) {
      const data = eventDataToJson(event);
      if (data.vaultId) {
        const vaultId = data.vaultId as number;
        reloadVaults.add(vaultId);
      }

      let logEvent = false;

      if (event.section === 'liquidityPools') {
        if (client.events.liquidityPools.BidPoolDistributed.is(event)) {
          const { bidPoolBurned, bidPoolDistributed } = event.data;
          data.burned = formatArgons(bidPoolBurned.toBigInt());
          data.distributed = formatArgons(bidPoolDistributed.toBigInt());
          logEvent = true;
        } else if (
          client.events.liquidityPools.NextBidPoolCapitalLocked.is(event)
        ) {
          const { totalActivatedCapital } = event.data;
          data.totalActivatedCapital = formatArgons(
            totalActivatedCapital.toBigInt(),
          );
          logEvent = true;
        }
      } else if (event.section === 'bitcoinLocks') {
        if (client.events.bitcoinLocks.BitcoinLockCreated.is(event)) {
          const { lockPrice, utxoId, accountId, vaultId } = event.data;
          this.obligationsById[utxoId.toNumber()] = {
            vaultId: vaultId.toNumber(),
            amount: lockPrice.toBigInt(),
          };
          data.lockPrice = formatArgons(lockPrice.toBigInt());
          data.accountId = accountId.toHuman();
          reloadVaults.add(vaultId.toNumber());
        }
        logEvent = true;
      } else if (event.section === 'mint') {
        logEvent = true;
        if (client.events.mint.MiningMint.is(event)) {
          const { amount } = event.data;
          data.amount = formatArgons(amount.toBigInt());
          if (this.monitorAccounts) {
            const activeMiners = await client.query.miningSlot
              .activeMinersCount()
              .then(x => x.toBigInt());
            const amountPerMiner = amount.toBigInt() / activeMiners;
            if (amountPerMiner > 0n) {
              for (const [address, info] of Object.entries(
                this.trackedAccountsByAddress,
              )) {
                const { cohortId } = info;
                this.events.emit('accountset-minted', blockHash, {
                  accountId: address,
                  argons: amountPerMiner,
                  cohortId,
                });
              }
            }
          }
        }
      } else if (event.section === 'miningSlot') {
        logEvent = true;
        if (client.events.miningSlot.SlotBidderAdded.is(event)) {
          data.amount = formatArgons(event.data.bidAmount.toBigInt());
          this.events.emit('mining-bid', blockHash, {
            amount: event.data.bidAmount.toBigInt(),
            accountId: event.data.accountId.toString(),
          });
        } else if (client.events.miningSlot.SlotBidderDropped.is(event)) {
          this.events.emit('mining-bid-ousted', blockHash, {
            accountId: event.data.accountId.toString(),
            preservedArgonotHold: event.data.preservedArgonotHold.toPrimitive(),
          });
        }
        if (client.events.miningSlot.NewMiners.is(event)) {
          this.newCohortMiners(
            event.data.cohortId.toNumber(),
            event.data.newMiners.map(x => x.accountId.toHuman()),
          );
        }
      } else if (event.section === 'bitcoinUtxos') {
        logEvent = true;
        if (client.events.bitcoinUtxos.UtxoVerified.is(event)) {
          const { utxoId } = event.data;
          const details = await this.getBitcoinLockDetails(
            utxoId.toNumber(),
            blockHash,
          );
          this.events.emit('bitcoin-verified', blockHash, {
            utxoId: utxoId.toNumber(),
            vaultId: details.vaultId,
            amount: details.amount,
          });

          data.amount = formatArgons(details.amount);
          reloadVaults.add(details.vaultId);
        }
      } else if (event.section === 'blockRewards') {
        if (client.events.blockRewards.RewardCreated.is(event)) {
          const { rewards } = event.data;
          for (const reward of rewards) {
            const { argons, ownership } = reward;

            const entry = this.trackedAccountsByAddress[author];
            if (entry) {
              this.events.emit('accountset-authored', blockHash, {
                author,
                argons: argons.toBigInt(),
                argonots: ownership.toBigInt(),
                cohortId: entry.cohortId,
              });
            }
          }
        }
      } else if (event.section === 'system') {
        if (client.events.system.ExtrinsicFailed.is(event)) {
          const { dispatchError } = event.data;
          if (dispatchError.isModule) {
            const decoded = api.registry.findMetaError(dispatchError.asModule);
            const { name, section } = decoded;
            block ??= await client.rpc.chain.getBlock(header.hash);
            const extrinsicIndex = phase.asApplyExtrinsic.toNumber();
            const ext = block!.block.extrinsics[extrinsicIndex];
            // translate dispatchInfo into readable tx
            if (this.shouldLog) {
              console.log(
                `> [Failed Tx] ${section}.${name} -> ${ext.method.section}.${ext.method.method} (nonce=${ext.nonce})`,
                (ext.toHuman() as any)?.method?.args,
              );
            }
          } else {
            // Other, CannotLookup, BadOrigin, no extra info
            if (this.shouldLog) {
              console.log(`x [Failed Tx] ${dispatchError.toJSON()}`);
            }
          }
        }
      }
      if (this.shouldLog && logEvent) {
        console.log(`> ${event.section}.${event.method}`, data);
      }
      this.events.emit('event', blockHash, event);
    }
    if (reloadVaults.size)
      this.events.emit('vaults-updated', blockHash, reloadVaults);
  }

  private async getBitcoinLockDetails(
    utxoId: number,
    blockHash: Uint8Array,
  ): Promise<{ vaultId: number; amount: bigint; utxoId?: number }> {
    const client = await this.mainchain;
    const api = await client.at(blockHash);
    let obligationId = this.obligationIdByUtxoId[utxoId];
    if (!obligationId) {
      const lock = await api.query.bitcoinLocks.locksByUtxoId(utxoId);
      obligationId = lock.value.obligationId.toNumber();
      this.obligationIdByUtxoId[utxoId] = obligationId;
      this.obligationsById[obligationId] = {
        vaultId: lock.value.vaultId.toNumber(),
        amount: lock.value.lockPrice.toBigInt(),
      };
    }
    return this.obligationsById[obligationId];
  }
}
