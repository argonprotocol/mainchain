import {
  type ArgonClient,
  getClient,
  type IGlobalOptions,
  Keyring,
  keyringFromCli,
  type KeyringPair,
} from './index';
import { dispatchErrorToString, formatArgons } from './utils';
import { logExtrinsicResult, TxSubmitter } from './TxSubmitter';
import { AccountRegistry } from './AccountRegistry';
import type { Command } from '@commander-js/extra-typings';
import * as process from 'node:process';
import { SubmittableExtrinsic } from '@polkadot/api/promise/types';
import { AccountMiners } from './AccountMiners';
import { ApiDecoration } from '@polkadot/api/types';

export type SubaccountRange = readonly number[];

export type IAddressNames = Map<string, string>;

export interface ISubaccountMiner {
  address: string;
  seat?: number;
  bidAmount?: bigint;
  subaccountIndex: number;
  cohortId?: number;
  isLastDay: boolean;
}

export class Accountset {
  public txSubmitterPair: KeyringPair;
  public isProxy = false;
  public seedAddress: string;
  public subAccountsByAddress: {
    [address: string]: { index: number; pair?: KeyringPair };
  } = {};
  public accountRegistry: AccountRegistry;
  public readonly client: Promise<ArgonClient>;

  public get addresses(): string[] {
    return [this.seedAddress, ...Object.keys(this.subAccountsByAddress)];
  }

  public get namedAccounts(): IAddressNames {
    return this.accountRegistry.namedAccounts;
  }

  private readonly sessionKeyMnemonic: string | undefined;

  constructor(
    options: {
      client: Promise<ArgonClient>;
      accountRegistry?: AccountRegistry;
      subaccountRange?: SubaccountRange;
      sessionKeyMnemonic?: string;
      name?: string;
    } & (
      | { seedAccount: KeyringPair }
      | {
          seedAddress: string;
          isProxy: true;
          txSubmitter: KeyringPair;
        }
    ),
  ) {
    if ('seedAccount' in options) {
      this.txSubmitterPair = options.seedAccount;
      this.seedAddress = options.seedAccount.address;
      this.isProxy = false;
    } else {
      this.isProxy = options.isProxy;
      this.txSubmitterPair = options.txSubmitter;
      this.seedAddress = options.seedAddress;
    }
    this.sessionKeyMnemonic = options.sessionKeyMnemonic;
    this.accountRegistry =
      options.accountRegistry ?? AccountRegistry.factory(options.name);
    this.client = options.client;
    const defaultRange = options.subaccountRange ?? getDefaultSubaccountRange();
    this.accountRegistry.register(
      this.seedAddress,
      `${this.accountRegistry.me}//seed`,
    );
    for (const i of defaultRange) {
      const pair = this.txSubmitterPair.derive(`//${i}`);
      this.subAccountsByAddress[pair.address] = { pair, index: i };
      this.accountRegistry.register(
        pair.address,
        `${this.accountRegistry.me}//${i}`,
      );
    }
  }

  public async balance(blockHash?: Uint8Array): Promise<bigint> {
    const client = await this.client;
    const api = blockHash ? await client.at(blockHash) : client;
    const accountData = await api.query.system.account(this.seedAddress);

    return accountData.data.free.toBigInt();
  }

  public async totalArgonsAt(
    blockHash?: Uint8Array,
  ): Promise<{ address: string; amount: bigint; index: number }[]> {
    const client = await this.client;
    const api = blockHash ? await client.at(blockHash) : client;
    const addresses = this.addresses;
    const results = await api.query.system.account.multi(addresses);
    return results.map((account, i) => {
      const address = addresses[i];
      return {
        address,
        amount: account.data.free.toBigInt(),
        index: this.subAccountsByAddress[address]?.index ?? Number.NaN,
      };
    });
  }

  public async totalArgonotsAt(
    blockHash?: Uint8Array,
  ): Promise<{ address: string; amount: bigint; index: number }[]> {
    const client = await this.client;
    const api = blockHash ? await client.at(blockHash) : client;
    const addresses = this.addresses;
    const results = await api.query.ownership.account.multi(addresses);
    return results.map((account, i) => {
      const address = addresses[i];
      return {
        address,
        amount: account.free.toBigInt(),
        index: this.subAccountsByAddress[address]?.index ?? Number.NaN,
      };
    });
  }

  public async getAvailableMinerAccounts(
    maxSeats: number,
  ): Promise<{ index: number; isRebid: boolean; address: string }[]> {
    const miningSeats = await this.miningSeats();
    const subaccountRange = [];
    for (const seat of miningSeats) {
      if (seat.hasWinningBid) {
        continue;
      }
      if (seat.isLastDay || seat.seat === undefined) {
        subaccountRange.push({
          index: seat.subaccountIndex,
          isRebid: seat.seat !== undefined,
          address: seat.address,
        });
        if (subaccountRange.length >= maxSeats) {
          break;
        }
      }
    }
    return subaccountRange;
  }

  public async loadRegisteredMiners(
    api: ApiDecoration<'promise'>,
  ): Promise<ISubaccountMiner[]> {
    const addresses = Object.keys(this.subAccountsByAddress);
    const rawIndices =
      await api.query.miningSlot.accountIndexLookup.multi(addresses);
    const addressToMiningIndex: { [address: string]: number } = {};
    for (let i = 0; i < addresses.length; i++) {
      const address = addresses[i];
      if (rawIndices[i].isNone) continue;
      addressToMiningIndex[address] = rawIndices[i].value.toNumber();
    }
    const indices = Object.values(addressToMiningIndex).filter(
      x => x !== undefined,
    ) as number[];

    const indexRegistrations = indices.length
      ? await api.query.miningSlot.activeMinersByIndex.multi(indices)
      : [];
    const registrationBySeatIndex: {
      [seatIndex: string]: { cohortId: number; bidAmount: bigint };
    } = {};

    for (let i = 0; i < indices.length; i++) {
      const seatIndex = indices[i];
      const registration = indexRegistrations[i];
      if (registration?.isSome) {
        registrationBySeatIndex[seatIndex] = {
          cohortId: registration.value.cohortId.toNumber(),
          bidAmount: registration.value.bid.toBigInt(),
        };
      }
    }
    const nextCohortId = await api.query.miningSlot.nextCohortId();

    return addresses.map(address => {
      const seat = addressToMiningIndex[address];
      const registration = registrationBySeatIndex[seat];
      let isLastDay = false;
      if (registration?.cohortId) {
        isLastDay = nextCohortId.toNumber() - registration?.cohortId === 10;
      }
      return {
        ...registration,
        address,
        seat,
        isLastDay,
        subaccountIndex:
          this.subAccountsByAddress[address]?.index ?? Number.NaN,
      };
    });
  }

  public async miningSeats(blockHash?: Uint8Array): Promise<
    (ISubaccountMiner & {
      hasWinningBid: boolean;
    })[]
  > {
    const client = await this.client;
    const api = blockHash ? await client.at(blockHash) : client;
    const miners = await this.loadRegisteredMiners(api);

    const nextCohort = await api.query.miningSlot.nextSlotCohort();

    return miners.map(miner => {
      const bid = nextCohort.find(x => x.accountId.toHuman() === miner.address);
      return {
        ...miner,
        hasWinningBid: !!bid,
        bidAmount: bid?.bid.toBigInt() ?? miner.bidAmount,
      };
    });
  }

  public async bids(
    blockHash?: Uint8Array,
  ): Promise<
    { address: string; bidPlace?: number; index: number; bidAmount: bigint }[]
  > {
    const client = await this.client;
    const api = blockHash ? await client.at(blockHash) : client;
    const addresses = Object.keys(this.subAccountsByAddress);
    const nextCohort = await api.query.miningSlot.nextSlotCohort();

    const registrationsByAddress = Object.fromEntries(
      nextCohort.map((x, i) => [x.accountId.toHuman(), { ...x, index: i }]),
    );

    return addresses.map(address => {
      const entry = registrationsByAddress[address];

      return {
        address,
        bidPlace: entry?.index,
        bidAmount: entry?.bid?.toBigInt(),
        index: this.subAccountsByAddress[address]?.index ?? Number.NaN,
      };
    });
  }

  public async consolidate(
    subaccounts?: SubaccountRange,
  ): Promise<{ index: number; inBlock?: string; failedError?: Error }[]> {
    const client = await this.client;
    const accounts = this.getAccountsInRange(subaccounts);
    const results: { index: number; inBlock?: string; failedError?: Error }[] =
      [];
    await Promise.allSettled(
      accounts.map(({ pair, index }) => {
        if (!pair) {
          results.push({
            index,
            failedError: new Error(`No keypair for //${index}`),
          });
          return Promise.resolve();
        }
        return new Promise<void>(resolve => {
          client.tx.utility
            .batchAll([
              client.tx.balances.transferAll(this.seedAddress, true),
              client.tx.ownership.transferAll(this.seedAddress, true),
            ])
            .signAndSend(pair, cb => {
              logExtrinsicResult(cb);
              if (cb.dispatchError) {
                const error = dispatchErrorToString(client, cb.dispatchError);

                results.push({
                  index,
                  failedError: new Error(
                    `Error consolidating //${index}: ${error}`,
                  ),
                });
                resolve();
              }
              if (cb.isInBlock) {
                results.push({ index, inBlock: cb.status.asInBlock.toHex() });
                resolve();
              }
            })
            .catch(e => {
              results.push({ index, failedError: e });
              resolve();
            });
        });
      }),
    );
    return results;
  }

  public status(opts: {
    argons: Awaited<ReturnType<Accountset['totalArgonsAt']>>;
    argonots: Awaited<ReturnType<Accountset['totalArgonotsAt']>>;
    seats: Awaited<ReturnType<Accountset['miningSeats']>>;
    bids: Awaited<ReturnType<Accountset['bids']>>;
    accountSubset?: ReturnType<Accountset['getAccountsInRange']>;
  }): IAccountStatus[] {
    const { argons, argonots, accountSubset, bids, seats } = opts;
    const accounts: IAccountStatus[] = [
      {
        index: 'main',
        address: this.seedAddress,
        argons: formatArgons(
          argons.find(x => x.address === this.seedAddress)?.amount ?? 0n,
        ),
        argonots: formatArgons(
          argonots.find(x => x.address === this.seedAddress)?.amount ?? 0n,
        ),
      },
    ];
    for (const [address, { index }] of Object.entries(
      this.subAccountsByAddress,
    )) {
      const argonAmount = argons.find(x => x.address === address)?.amount ?? 0n;
      const argonotAmount =
        argonots.find(x => x.address === address)?.amount ?? 0n;
      const bid = bids.find(x => x.address === address);
      const seat = seats.find(x => x.address === address)?.seat;
      const entry: IAccountStatus = {
        index: ` //${index}`,
        address,
        argons: formatArgons(argonAmount),
        argonots: formatArgons(argonotAmount),
        seat,
        bidPlace: bid?.bidPlace,
        bidAmount: bid?.bidAmount ?? 0n,
      };
      if (accountSubset) {
        entry.isWorkingOn = accountSubset.some(x => x.address === address);
      }
      accounts.push(entry);
    }
    return accounts;
  }

  public async registerKeys(url: string) {
    const client = await getClient(url.replace('ws:', 'http:'));
    const keys = this.keys();
    for (const [name, key] of Object.entries(keys)) {
      console.log('Registering key', name, key.publicKey);
      const result = await client.rpc.author.insertKey(
        name,
        key.privateKey,
        key.publicKey,
      );
      // verify keys
      const saved = await client.rpc.author.hasKey(key.publicKey, name);
      if (!saved) {
        console.error('Failed to register key', name, key.publicKey);
        throw new Error(`Failed to register ${name} key ${key.publicKey}`);
      }
      console.log(`Registered ${name} key`, result.toHuman());
    }
    await client.disconnect();
  }

  public keys(keysVersion?: number): {
    gran: { privateKey: string; publicKey: string; rawPublicKey: Uint8Array };
    seal: { privateKey: string; publicKey: string; rawPublicKey: Uint8Array };
  } {
    let version = keysVersion ?? 0;
    if (process.env.KEYS_VERSION) {
      version = parseInt(process.env.KEYS_VERSION) ?? 0;
    }
    const seedMnemonic = this.sessionKeyMnemonic ?? process.env.KEYS_MNEMONIC;
    if (!seedMnemonic) {
      throw new Error(
        'KEYS_MNEMONIC environment variable not set. Cannot derive keys.',
      );
    }
    const blockSealKey = `${seedMnemonic}//block-seal//${version}`;
    const granKey = `${seedMnemonic}//grandpa//${version}`;
    const blockSealAccount = new Keyring().createFromUri(blockSealKey, {
      type: 'ed25519',
    });
    const grandpaAccount = new Keyring().createFromUri(granKey, {
      type: 'ed25519',
    });
    return {
      seal: {
        privateKey: blockSealKey,
        publicKey: `0x${Buffer.from(blockSealAccount.publicKey).toString('hex')}`,
        rawPublicKey: blockSealAccount.publicKey,
      },
      gran: {
        privateKey: granKey,
        publicKey: `0x${Buffer.from(grandpaAccount.publicKey).toString('hex')}`,
        rawPublicKey: grandpaAccount.publicKey,
      },
    };
  }

  public async tx(tx: SubmittableExtrinsic): Promise<TxSubmitter> {
    const client = await this.client;
    return new TxSubmitter(client, tx, this.txSubmitterPair);
  }

  /**
   * Create a mining bid. This will create a bid for each account in the given range from the seed account as funding.
   */
  public async createMiningBids(options: {
    subaccountRange?: SubaccountRange;
    bidAmount: bigint;
    tip?: bigint;
    sendRewardsToSeed?: boolean;
  }): Promise<{
    finalFee?: bigint;
    blockHash?: Uint8Array;
    bidError?: Error;
    successfulBids?: number;
  }> {
    const accounts = this.getAccountsInRange(options.subaccountRange);
    const client = await this.client;

    let tip = options.tip ?? 0n;

    const batch = client.tx.utility.batch(
      accounts.map(x => {
        const keys = this.keys();
        const rewards = options.sendRewardsToSeed
          ? { Account: this.seedAddress }
          : { Owner: null };
        return client.tx.miningSlot.bid(
          options.bidAmount,
          rewards,
          {
            grandpa: keys.gran.rawPublicKey,
            blockSealAuthority: keys.seal.rawPublicKey,
          },
          x.address,
        );
      }),
    );

    let tx = batch;
    if (this.isProxy) {
      tx = client.tx.proxy.proxy(this.seedAddress, 'MiningBid', batch);
    }
    const submitter = new TxSubmitter(client, tx, this.txSubmitterPair);
    const txFee = await submitter.feeEstimate(tip);

    let minBalance = options.bidAmount * BigInt(accounts.length);
    let totalFees = tip + 1n + txFee;
    const seedBalance = await client.query.system
      .account(this.seedAddress)
      .then(x => x.data.free.toBigInt());
    if (!this.isProxy) {
      minBalance += totalFees;
    }
    if (seedBalance < minBalance) {
      throw new Error(
        `Insufficient balance to create mining bids. Seed account has ${formatArgons(
          seedBalance,
        )} but needs ${formatArgons(minBalance)}`,
      );
    }
    if (this.isProxy) {
      const { canAfford, availableBalance } = await submitter.canAfford({
        tip,
      });
      if (!canAfford) {
        throw new Error(
          `Insufficient balance to pay proxy fees. Proxy account has ${formatArgons(
            availableBalance,
          )} but needs ${formatArgons(totalFees)}`,
        );
      }
    }

    console.log('Creating bids', {
      perSeatBid: options.bidAmount,
      subaccounts: options.subaccountRange,
      txFee,
    });

    const txResult = await submitter.submit({
      tip,
      useLatestNonce: true,
    });

    const bidError = await txResult.inBlockPromise
      .then(() => undefined)
      .catch((x: Error) => x);
    return {
      finalFee: txResult.finalFee,
      bidError,
      blockHash: txResult.includedInBlock,
      successfulBids:
        txResult.batchInterruptedIndex !== undefined
          ? txResult.batchInterruptedIndex
          : accounts.length,
    };
  }

  public getAccountsInRange(range?: SubaccountRange): IAccountAndKey[] {
    const entries = new Set(range ?? getDefaultSubaccountRange());
    return Object.entries(this.subAccountsByAddress)
      .filter(([_, account]) => {
        return entries.has(account.index);
      })
      .map(([address, { pair, index }]) => ({ pair, index, address }));
  }

  public async watchBlocks(shouldLog: boolean = false): Promise<AccountMiners> {
    const accountMiners = await AccountMiners.loadAt(this, { shouldLog });
    await accountMiners.watch();
    return accountMiners;
  }

  public static async fromCli(program: Command, proxyForAddress?: string) {
    const parentOptions =
      program.parent?.optsWithGlobals() as unknown as IGlobalOptions;
    const keypair = await keyringFromCli(parentOptions);

    const client = getClient(parentOptions.mainchainUrl);
    if (proxyForAddress) {
      return new Accountset({
        client,
        isProxy: true,
        seedAddress: proxyForAddress,
        txSubmitter: keypair,
      });
    } else {
      return new Accountset({
        seedAccount: keypair,
        client,
      });
    }
  }
}

export function getDefaultSubaccountRange(): number[] {
  return parseSubaccountRange(process.env.SUBACCOUNT_RANGE ?? '0-9')!;
}

export function parseSubaccountRange(range?: string): number[] | undefined {
  if (!range) {
    return undefined;
  }
  const indices = [];
  for (const entry of range.split(',')) {
    if (entry.includes('-')) {
      const [start, end] = entry.split('-').map(x => parseInt(x, 10));
      for (let i = start; i <= end; i++) {
        indices.push(i);
      }
      continue;
    }

    const record = parseInt(entry.trim(), 10);
    if (Number.isNaN(record) || !Number.isInteger(record)) {
      throw new Error(`Invalid range entry: ${entry}`);
    }
    if (Number.isInteger(record)) {
      indices.push(record);
    }
  }
  return indices;
}

export type IAccountAndKey = {
  pair?: KeyringPair;
  index: number;
  address: string;
};

interface IAccountStatus {
  index: string;
  address: string;
  argons: string;
  argonots: string;
  seat?: number;
  bidPlace?: number;
  bidAmount?: bigint;
  isWorkingOn?: boolean;
}
