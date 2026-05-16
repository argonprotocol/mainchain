import { BitcoinLock } from '../BitcoinLock';
import type { IBitcoinLock } from '../BitcoinLock';
import { PriceIndex } from '../PriceIndex';
import BigNumber from 'bignumber.js';
import { describe, expect, it, vi } from 'vitest';

const canAffordMock = vi.hoisted(() =>
  vi.fn(async () => ({ canAfford: true, availableBalance: 0n, txFee: 0n })),
);

vi.mock('../TxSubmitter', () => ({
  TxSubmitter: class {
    canAfford = canAffordMock;
  },
}));

vi.mock('../TxResult', () => ({
  TxResult: class {},
}));

it('exports bitcoin lock helpers', () => {
  expect(BitcoinLock).toBeTruthy();
});

describe('BitcoinLock.calculateRedemptionAmount', () => {
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

    expect(BitcoinLock.calculateRedemptionAmountFromSatoshis(index, 100n, 60_000n)).toStrictEqual(
      50_000n,
    );
  });

  it('uses the argon target price through the redemption multiplier', async () => {
    const index = priceIndex({
      btcUsdPrice: 1,
      argonUsdPrice: 0.8,
      argonUsdTargetPrice: 1,
    });

    expect(
      BitcoinLock.calculateRedemptionAmountFromSatoshis(index, 100_000_000n, 1_250_000n),
    ).toStrictEqual(1_054_800n);
  });
});

describe('BitcoinLock.createInitializeTx', () => {
  function priceIndex(): PriceIndex {
    const index = new PriceIndex();
    index.btcUsdPrice = new BigNumber(60_000);
    index.argonUsdPrice = new BigNumber(1);
    index.argonUsdTargetPrice = new BigNumber(1);
    return index;
  }

  function client() {
    return {
      consts: { balances: { existentialDeposit: { toBigInt: () => 0n } } },
      tx: {
        bitcoinLocks: {
          initialize: vi.fn(() => ({})),
        },
      },
    };
  }

  it('uses a requested target BTC rate when estimating the initialization security fee', async () => {
    canAffordMock.mockClear();
    const vault = {
      vaultId: 1,
      operatorAccountId: 'vault-owner',
      calculateBitcoinFee: vi.fn(amount => amount / 10n),
    };

    await BitcoinLock.createInitializeTx({
      client: client() as any,
      vault: vault as any,
      priceIndex: priceIndex(),
      ownerBitcoinPubkey: new Uint8Array(33),
      satoshis: 50_000_000n,
      txSigner: { address: 'lock-owner', signer: {} as any },
      microgonsAtTargetPerBtc: 2_000_000n,
    });

    expect(vault.calculateBitcoinFee).toHaveBeenCalledWith(1_000_000n);
    expect(canAffordMock).toHaveBeenCalledWith({
      tip: 0n,
      unavailableBalance: 100_000n,
      includeExistentialDeposit: true,
    });
  });
});

describe('BitcoinLock.calculateRatchetingCosts', () => {
  function ratchetPriceIndex(currentTargetPrice: bigint): PriceIndex {
    const index = new PriceIndex();
    index.argonUsdPrice = new BigNumber(1);
    index.argonUsdTargetPrice = new BigNumber(1);
    vi.spyOn(index, 'getSatoshiPriceInTargetMicrogons').mockReturnValue(currentTargetPrice);
    return index;
  }

  function bitcoinTipClient(blockHeight: number) {
    return {
      query: {
        bitcoinUtxos: {
          confirmedBitcoinBlockTip: vi.fn(async () => ({
            unwrap: () => ({ blockHeight: { toNumber: () => blockHeight } }),
          })),
        },
      },
    };
  }

  function feeVault(baseFee: bigint, percentageFee: bigint) {
    return {
      terms: { bitcoinBaseFee: baseFee },
      calculateBitcoinFee: vi.fn(() => baseFee + percentageFee),
    };
  }

  it('floors the prorated up-ratchet fee to match runtime fixed-point math', async () => {
    const lock = new BitcoinLock({
      createdAtHeight: 10,
      vaultClaimHeight: 13,
      lockedTargetPrice: 1_000n,
      satoshis: 1n,
    } as IBitcoinLock);

    const costs = await lock.calculateRatchetingCosts(
      bitcoinTipClient(11) as any,
      ratchetPriceIndex(2_000n),
      feeVault(10n, 1_000n) as any,
    );

    expect(costs.ratchetingFee).toStrictEqual(676n);
  });

  it('uses a requested target BTC rate when estimating an up-ratchet fee', async () => {
    const lock = new BitcoinLock({
      createdAtHeight: 10,
      vaultClaimHeight: 13,
      lockedTargetPrice: 1_000n,
      satoshis: 50_000_000n,
    } as IBitcoinLock);
    const vault = feeVault(10n, 1_000n);

    const costs = await lock.calculateRatchetingCosts(
      bitcoinTipClient(11) as any,
      ratchetPriceIndex(10_000n),
      vault as any,
      4_000n,
    );

    expect(vault.calculateBitcoinFee).toHaveBeenCalledWith(1_000n);
    expect(costs.ratchetingFee).toStrictEqual(676n);
    expect(costs.burnAmount).toStrictEqual(0n);
  });

  it('uses a requested target BTC rate when estimating a down-ratchet burn', async () => {
    const lock = new BitcoinLock({
      createdAtHeight: 10,
      vaultClaimHeight: 13,
      lockedTargetPrice: 3_000n,
      satoshis: 50_000_000n,
    } as IBitcoinLock);

    const costs = await lock.calculateRatchetingCosts(
      bitcoinTipClient(11) as any,
      ratchetPriceIndex(10_000n),
      feeVault(10n, 1_000n) as any,
      4_000n,
    );

    expect(costs.ratchetingFee).toStrictEqual(10n);
    expect(costs.burnAmount).toStrictEqual(2_000n);
  });

  it('handles a zero-length lock term without dividing by zero', async () => {
    const lock = new BitcoinLock({
      createdAtHeight: 10,
      vaultClaimHeight: 10,
      lockedTargetPrice: 1_000n,
      satoshis: 1n,
    } as IBitcoinLock);

    const costs = await lock.calculateRatchetingCosts(
      bitcoinTipClient(10) as any,
      ratchetPriceIndex(2_000n),
      feeVault(10n, 1_000n) as any,
    );

    expect(costs.ratchetingFee).toStrictEqual(1_010n);
  });
});
