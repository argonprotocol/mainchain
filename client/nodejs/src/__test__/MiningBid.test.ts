import { describe, expect, it } from 'vitest';
import { TypeRegistry } from '@polkadot/types';
import { hexToU8a } from '@polkadot/util';
import type { ApiPromise } from '@polkadot/api';
import { createMiningBidCommitment, miningBidRevealWindow } from '../miningBid';

const api = {
  genesisHash: { toHex: () => '0x' + '00'.repeat(32) },
  registry: new TypeRegistry(),
  consts: { miningSlot: { bidRevealBlocks: { toBigInt: () => 10n } } },
} as unknown as ApiPromise;
const range = {
  targetFrameId: 2n,
  fundingAccount: '0x' + '01'.repeat(32),
  miningAccount: '0x' + '02'.repeat(32),
  bid: 90_000_000n,
  max: 150_000_000n,
  keys: new Uint8Array(64).fill(3),
  salt: new Uint8Array(32).fill(4),
};
describe('mining bid commitments', () => {
  it('returns the public minimum and inclusive runtime reveal window', () => {
    expect(createMiningBidCommitment(api, range).bid).toBe(range.bid);
    expect(miningBidRevealWindow(api, 100n)).toEqual({ firstBlock: 101n, lastBlock: 110n });
  });
  it('generates a usable 32-byte salt when omitted', () => {
    const { salt: _salt, ...withoutSalt } = range;
    const result = createMiningBidCommitment(api, withoutSalt);
    expect(result.salt).toMatch(/^0x[0-9a-f]{64}$/);
    expect(createMiningBidCommitment(api, { ...withoutSalt, salt: hexToU8a(result.salt) })).toEqual(
      result,
    );
  });
  it('matches the Rust SCALE vector', () => {
    expect(createMiningBidCommitment(api, range).maxHash).toBe(
      '0x94ce81995f77608e7930dfc85a9d135b119196912afcec0e5be00ae8d08f8736',
    );
  });
  it('binds the frame, accounts, keys, and both bounds', () => {
    const original = createMiningBidCommitment(api, range).maxHash;
    for (const change of [
      { targetFrameId: 3n },
      { fundingAccount: range.miningAccount },
      { miningAccount: range.fundingAccount },
      { bid: 90_010_000n },
      { max: 150_010_000n },
      { keys: new Uint8Array(64) },
    ])
      expect(createMiningBidCommitment(api, { ...range, ...change }).maxHash).not.toBe(original);
  });
  it('rejects malformed ranges and salts', () => {
    for (const change of [
      { bid: -1n },
      { bid: 160_000_000n },
      { max: 1n << 128n },
      { bid: 1n },
      { salt: new Uint8Array(31) },
    ]) {
      expect(() => createMiningBidCommitment(api, { ...range, ...change })).toThrow();
    }
  });
});
