import type { ApiPromise } from '@polkadot/api';
import { blake2AsU8a, decodeAddress, randomAsU8a } from '@polkadot/util-crypto';
import { hexToU8a, stringToU8a, u8aConcat, u8aToHex } from '@polkadot/util';

type RuntimeMiningBid = ApiPromise['tx']['miningSlot']['bid'];
type MiningBidArgs = Parameters<RuntimeMiningBid>;
type OptionalMiningBid = RuntimeMiningBid &
  ((
    bid: MiningBidArgs[0],
    keys: MiningBidArgs[1],
    miningAccountId: MiningBidArgs[2],
    targetFrameId?: MiningBidArgs[3],
    maxHash?: MiningBidArgs[4],
  ) => ReturnType<RuntimeMiningBid>);

export type MiningBidClient = ApiPromise & {
  readonly tx: ApiPromise['tx'] & {
    miningSlot: ApiPromise['tx']['miningSlot'] & { bid: OptionalMiningBid };
  };
};

const clientsWithBidDefaults = new WeakSet<ApiPromise>();
const bidsWithDefaults = new WeakSet<RuntimeMiningBid>();

/** Enable synchronous trailing defaults on SDK clients, including after metadata refreshes. */
export function withMiningBidDefaults(api: ApiPromise): MiningBidClient {
  const decorate = () => {
    const mining = api.tx.miningSlot;
    const bid = mining?.bid;
    if (!bid || bidsWithDefaults.has(bid)) return;
    const fields = bid.meta.fields;
    // Leave older runtime calls unchanged rather than encoding them using a newer schema.
    if (
      fields.length !== 5 ||
      fields[3].name.unwrapOrDefault().toString() !== 'target_frame_id' ||
      api.registry.lookup.getTypeDef(fields[3].type).type !== 'Option<u64>' ||
      fields[4].name.unwrapOrDefault().toString() !== 'max_hash'
    )
      return;

    const wrapped = new Proxy(bid, {
      apply(target, thisArg, args: unknown[]) {
        // Preserve the underlying argument-count errors for malformed calls.
        if (args.length < 3 || args.length > 5) return Reflect.apply(target, thisArg, args);
        const [amount, keys, miner, frame = null, maxHash = null] = args;
        return Reflect.apply(target, thisArg, [amount, keys, miner, frame, maxHash]);
      },
    });
    bidsWithDefaults.add(wrapped);
    Object.defineProperty(mining, 'bid', {
      configurable: true,
      enumerable: true,
      writable: true,
      value: wrapped,
    });
  };

  if (!clientsWithBidDefaults.has(api)) {
    api.on('decorated', decorate);
    clientsWithBidDefaults.add(api);
  }
  decorate();
  return api as MiningBidClient;
}

export interface MiningBidRange {
  targetFrameId: bigint;
  fundingAccount: string;
  miningAccount?: string;
  bid: bigint;
  max: bigint;
  /** SCALE-encoded session keys, in runtime field order. */
  keys: Uint8Array;
  salt?: Uint8Array;
}

/** Create a v1 commitment. Keep the returned salt private until calling bidMax in blocks B+1 through B+5. */
export function createMiningBidCommitment(api: ApiPromise, range: MiningBidRange) {
  const salt = range.salt ?? randomAsU8a(32);
  if (salt.length !== 32) throw new Error('Mining bid salt must contain 32 bytes');
  if (range.bid < 0n || range.max < range.bid || range.max >= 1n << 128n) {
    throw new Error('Invalid mining bid range');
  }
  if (range.bid % 10_000n !== 0n || range.max % 10_000n !== 0n) {
    throw new Error('Mining bids must use 10,000 microgon increments');
  }
  const funding = decodeAddress(range.fundingAccount);
  const miner = decodeAddress(range.miningAccount ?? range.fundingAccount);
  const encoded = u8aConcat(
    stringToU8a('argon-mining-bid-v1'),
    hexToU8a(api.genesisHash.toHex()),
    api.registry.createType('u64', range.targetFrameId.toString()).toU8a(),
    funding,
    miner,
    api.registry.createType('u128', range.bid.toString()).toU8a(),
    api.registry.createType('u128', range.max.toString()).toU8a(),
    range.keys,
    salt,
  );
  return {
    bid: range.bid,
    maxHash: u8aToHex(blake2AsU8a(encoded, 256)),
    salt: u8aToHex(salt),
  };
}

/** Inclusive reveal window, derived from runtime configuration and the actual inclusion block. */
export function miningBidRevealWindow(api: ApiPromise, submissionBlock: bigint) {
  const blocks = api.consts.miningSlot.bidRevealBlocks.toBigInt();
  return { firstBlock: submissionBlock + 1n, lastBlock: submissionBlock + blocks };
}
