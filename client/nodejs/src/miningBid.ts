import type { ApiPromise } from '@polkadot/api';
import { blake2AsU8a, decodeAddress, randomAsU8a } from '@polkadot/util-crypto';
import { hexToU8a, stringToU8a, u8aConcat, u8aToHex } from '@polkadot/util';

export interface MiningBidRange {
  targetFrameId: bigint;
  fundingAccount: string;
  miningAccount?: string;
  lowBid: bigint;
  highBid: bigint;
  /** SCALE-encoded session keys, in runtime field order. */
  keys: Uint8Array;
  salt?: Uint8Array;
}

/** Create a v1 commitment. Keep the returned salt private until the following block. */
export function createMiningBidCommitment(api: ApiPromise, range: MiningBidRange) {
  const salt = range.salt ?? randomAsU8a(256);
  if (salt.length !== 32) throw new Error('Mining bid salt must contain 32 bytes');
  if (range.lowBid < 0n || range.highBid < range.lowBid || range.highBid >= 1n << 128n) {
    throw new Error('Invalid mining bid range');
  }
  if (range.lowBid % 10_000n !== 0n || range.highBid % 10_000n !== 0n) {
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
    api.registry.createType('u128', range.lowBid.toString()).toU8a(),
    api.registry.createType('u128', range.highBid.toString()).toU8a(),
    range.keys,
    salt,
  );
  return {
    commitmentHash: u8aToHex(blake2AsU8a(encoded, 256)),
    salt: u8aToHex(salt),
  };
}
