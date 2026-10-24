import { describe, expect, it } from 'vitest';
import { TypeRegistry } from '@polkadot/types';
import { hexToU8a } from '@polkadot/util';
import type { ApiPromise } from '@polkadot/api';
import { createMiningBidCommitment } from '../miningBid';

const api = {
  genesisHash: { toHex: () => '0x' + '00'.repeat(32) },
  registry: new TypeRegistry(),
} as unknown as ApiPromise;
const range = {
  targetFrameId: 2n,
  fundingAccount: '0x' + '01'.repeat(32),
  miningAccount: '0x' + '02'.repeat(32),
  lowBid: 90_000_000n,
  highBid: 150_000_000n,
  keys: new Uint8Array(64).fill(3),
  salt: new Uint8Array(32).fill(4),
};
describe('mining bid commitments', () => {
  it('generates a usable 32-byte salt when omitted', () => {
    const { salt: _salt, ...withoutSalt } = range;
    const result = createMiningBidCommitment(api, withoutSalt);
    expect(result.salt).toMatch(/^0x[0-9a-f]{64}$/);
    expect(createMiningBidCommitment(api, { ...withoutSalt, salt: hexToU8a(result.salt) })).toEqual(
      result,
    );
  });
  it('matches the Rust SCALE vector', () => {
    expect(createMiningBidCommitment(api, range).commitmentHash).toBe(
      '0x94ce81995f77608e7930dfc85a9d135b119196912afcec0e5be00ae8d08f8736',
    );
  });
  it('binds the frame, accounts, keys, and both bounds', () => {
    const original = createMiningBidCommitment(api, range).commitmentHash;
    for (const change of [
      { targetFrameId: 3n },
      { fundingAccount: range.miningAccount },
      { miningAccount: range.fundingAccount },
      { lowBid: 90_010_000n },
      { highBid: 150_010_000n },
      { keys: new Uint8Array(64) },
    ])
      expect(createMiningBidCommitment(api, { ...range, ...change }).commitmentHash).not.toBe(
        original,
      );
  });
  it('rejects malformed ranges and salts', () => {
    for (const change of [
      { lowBid: -1n },
      { lowBid: 160_000_000n },
      { highBid: 1n << 128n },
      { lowBid: 1n },
      { salt: new Uint8Array(31) },
    ]) {
      expect(() => createMiningBidCommitment(api, { ...range, ...change })).toThrow();
    }
  });
});
