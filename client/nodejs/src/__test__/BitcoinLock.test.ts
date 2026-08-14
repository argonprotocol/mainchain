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

describe('BitcoinLock.satoshisRequiredForRedemptionAmount', () => {
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

  it('returns the minimum sats needed when argon is at target', () => {
    const index = priceIndex({
      btcUsdPrice: 60_000,
      argonUsdPrice: 1,
      argonUsdTargetPrice: 1,
    });

    const redemptionAmount = 2_000_000_000n;
    const satoshis = BitcoinLock.satoshisRequiredForRedemptionAmount(index, redemptionAmount);

    expect(
      BitcoinLock.calculateRedemptionAmountFromSatoshis(index, satoshis),
    ).toBeGreaterThanOrEqual(redemptionAmount);
    expect(BitcoinLock.calculateRedemptionAmountFromSatoshis(index, satoshis - 1n)).toBeLessThan(
      redemptionAmount,
    );
  });

  it('reverses the redemption multiplier when argon is below target', () => {
    const index = priceIndex({
      btcUsdPrice: 1,
      argonUsdPrice: 0.8,
      argonUsdTargetPrice: 1,
    });

    const redemptionAmount = 1_054_800n;
    const satoshis = BitcoinLock.satoshisRequiredForRedemptionAmount(index, redemptionAmount);

    expect(satoshis).toStrictEqual(100_000_000n);
    expect(
      BitcoinLock.calculateRedemptionAmountFromSatoshis(index, satoshis),
    ).toBeGreaterThanOrEqual(redemptionAmount);
    expect(BitcoinLock.calculateRedemptionAmountFromSatoshis(index, satoshis - 1n)).toBeLessThan(
      redemptionAmount,
    );
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

  it('detects whether the connected runtime still exposes initializeFor', () => {
    const currentClient = client();
    const legacyClient = client();
    Object.assign(legacyClient.tx.bitcoinLocks, { initializeFor: vi.fn(() => ({})) });

    expect(BitcoinLock.supportsInitializeFor(currentClient as any)).toBe(false);
    expect(BitcoinLock.supportsInitializeFor(legacyClient as any)).toBe(true);
  });

  it('routes delegated initialization through initializeFor on a legacy runtime', async () => {
    canAffordMock.mockClear();
    const testClient = client();
    const initializeFor = vi.fn(() => ({}));
    Object.assign(testClient.tx.bitcoinLocks, { initializeFor });
    const vault = {
      vaultId: 1,
      operatorAccountId: 'vault-owner',
      calculateBitcoinFee: vi.fn(() => 100_000n),
    };

    const result = await BitcoinLock.createInitializeTx({
      client: testClient as any,
      vault: vault as any,
      priceIndex: priceIndex(),
      ownerBitcoinPubkey: new Uint8Array(33),
      satoshis: 50_000_000n,
      txSigner: { address: 'vault-delegate', signer: {} as any },
      initializeForAccountId: 'lock-owner',
      securitizationSpaceToUnreserve: 20_000n,
    });

    expect(initializeFor).toHaveBeenCalledWith(
      'lock-owner',
      1,
      50_000_000n,
      new Uint8Array(33),
      { V1: { microgonsAtTargetPerBtc: null } },
      20_000n,
    );
    expect(testClient.tx.bitcoinLocks.initialize).not.toHaveBeenCalled();
    expect(result.securityFee).toBe(0n);
  });

  it('rejects conflicting or unsupported initialization options', async () => {
    const currentClient = client();
    const legacyClient = client();
    Object.assign(legacyClient.tx.bitcoinLocks, { initializeFor: vi.fn(() => ({})) });
    const vault = {
      vaultId: 1,
      operatorAccountId: 'vault-owner',
      calculateBitcoinFee: vi.fn(() => 100_000n),
    };
    const args = {
      vault: vault as any,
      priceIndex: priceIndex(),
      ownerBitcoinPubkey: new Uint8Array(33),
      satoshis: 50_000_000n,
      txSigner: { address: 'lock-owner', signer: {} as any },
    };
    const feeCoupon = {
      vaultId: 1,
      genesisHash: '0x00',
      beneficiary: 'lock-owner',
      requestedSatoshis: 50_000_000n,
      microgonsAtTargetPerBtc: null,
      feeDiscount: 40_000n,
      securitizationSpaceToUnreserve: 0n,
      expiresAtFrame: 10n,
      nonce: 1n,
      signature: {} as any,
    };

    await expect(
      BitcoinLock.createInitializeTx({
        ...args,
        client: legacyClient as any,
        feeCoupon,
        initializeForAccountId: 'lock-owner',
      }),
    ).rejects.toThrow('Cannot provide both initializeForAccountId and feeCoupon');

    await expect(
      BitcoinLock.createInitializeTx({
        ...args,
        client: legacyClient as any,
        feeCoupon,
      }),
    ).rejects.toThrow('The connected runtime does not support Bitcoin lock fee coupons');

    await expect(
      BitcoinLock.createInitializeTx({
        ...args,
        client: currentClient as any,
        initializeForAccountId: 'lock-owner',
      }),
    ).rejects.toThrow('The connected runtime no longer supports initializeFor');
  });

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

  it('submits a signed fee coupon and estimates only the remaining security fee', async () => {
    canAffordMock.mockClear();
    const testClient = client();
    const vault = {
      vaultId: 1,
      operatorAccountId: 'vault-owner',
      calculateBitcoinFee: vi.fn(() => 100_000n),
    };
    const feeCoupon = {
      vaultId: 1,
      genesisHash: '0x00',
      beneficiary: 'lock-owner',
      requestedSatoshis: 50_000_000n,
      microgonsAtTargetPerBtc: null,
      feeDiscount: 40_000n,
      securitizationSpaceToUnreserve: 0n,
      expiresAtFrame: 10n,
      nonce: 1n,
      signature: {} as any,
    };

    await BitcoinLock.createInitializeTx({
      client: testClient as any,
      vault: vault as any,
      priceIndex: priceIndex(),
      ownerBitcoinPubkey: new Uint8Array(33),
      satoshis: 50_000_000n,
      txSigner: { address: 'lock-owner', signer: {} as any },
      feeCoupon,
    });

    expect(testClient.tx.bitcoinLocks.initialize).toHaveBeenCalledWith(
      1,
      50_000_000n,
      new Uint8Array(33),
      {
        V2: {
          microgonsAtTargetPerBtc: null,
          feeCoupon,
        },
      },
    );
    expect(canAffordMock).toHaveBeenCalledWith({
      tip: 0n,
      unavailableBalance: 60_000n,
      includeExistentialDeposit: true,
    });
  });

  it('caps the estimated fee discount at the full security fee', async () => {
    canAffordMock.mockClear();
    const vault = {
      vaultId: 1,
      operatorAccountId: 'vault-owner',
      calculateBitcoinFee: vi.fn(() => 100_000n),
    };

    await BitcoinLock.createInitializeTx({
      client: client() as any,
      vault: vault as any,
      priceIndex: priceIndex(),
      ownerBitcoinPubkey: new Uint8Array(33),
      satoshis: 50_000_000n,
      txSigner: { address: 'lock-owner', signer: {} as any },
      feeCoupon: {
        vaultId: 1,
        genesisHash: '0x00',
        beneficiary: 'lock-owner',
        requestedSatoshis: 50_000_000n,
        microgonsAtTargetPerBtc: null,
        feeDiscount: 200_000n,
        securitizationSpaceToUnreserve: 0n,
        expiresAtFrame: 10n,
        nonce: 1n,
        signature: {} as any,
      },
    });

    expect(canAffordMock).toHaveBeenCalledWith({
      tip: 0n,
      unavailableBalance: 0n,
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
