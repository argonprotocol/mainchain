import { BitcoinLock } from '../BitcoinLock';
import { PriceIndex } from '../PriceIndex';
import BigNumber from 'bignumber.js';
import { describe, expect, it, vi } from 'vitest';

vi.mock('../TxSubmitter', () => ({
  TxSubmitter: class {},
}));

vi.mock('../TxResult', () => ({
  TxResult: class {},
}));

it('exports bitcoin lock helpers', () => {
  expect(BitcoinLock).toBeTruthy();
});

describe('BitcoinLock.calculateUnlockAmount', () => {
  function priceIndex(args: {
    btcUsdPrice: number;
    argonUsdPrice: number;
    argonUsdTargetPrice: number;
  }): PriceIndex {
    const index = new PriceIndex();
    index.btcUsdPrice = new BigNumber(args.btcUsdPrice);
    index.argonUsdPrice = new BigNumber(args.argonUsdPrice);
    index.argonUsdTargetPrice = new BigNumber(args.argonUsdTargetPrice);
    return index;
  }

  it('uses the argon USD price when calculating the base redemption rate', async () => {
    const index = priceIndex({
      btcUsdPrice: 60_000,
      argonUsdPrice: 1.2,
      argonUsdTargetPrice: 1.2,
    });

    expect(
      BitcoinLock.calculateUnlockAmount(index, { lockedTargetPrice: 60_000n, satoshis: 100n }),
    ).toStrictEqual(50_000n);
  });

  it('uses the argon target price through the redemption multiplier', async () => {
    const index = priceIndex({
      btcUsdPrice: 1,
      argonUsdPrice: 0.8,
      argonUsdTargetPrice: 1,
    });

    expect(
      BitcoinLock.calculateUnlockAmount(index, {
        lockedTargetPrice: 1_250_000n,
        satoshis: 100_000_000n,
      }),
    ).toStrictEqual(1_054_800n);
  });
});
