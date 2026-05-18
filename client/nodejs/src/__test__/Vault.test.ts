import { Vault } from '../Vault';
import type { PriceIndex } from '../PriceIndex';
import { MICROGONS_PER_ARGON } from '../utils';
import { describe, expect, it } from 'vitest';

const MICROGONS_PER_ARGON_BIGINT = BigInt(MICROGONS_PER_ARGON);

describe('Vault.availableBondSpace', () => {
  function vaultWithSecuritizedSatoshis(securitizedSatoshis: number): Vault {
    const vault = Object.create(Vault.prototype) as Vault;
    vault.securitizedSatoshis = securitizedSatoshis;
    return vault;
  }

  function priceIndexForCapacity(totalBondCapacityMicrogons: bigint): PriceIndex {
    return {
      getSatoshiPriceInTargetMicrogons: () => totalBondCapacityMicrogons,
    } as unknown as PriceIndex;
  }

  it('returns one-tenth of securitized bitcoin value as whole-bond microgons by default', () => {
    const vault = vaultWithSecuritizedSatoshis(1);
    const priceIndex = priceIndexForCapacity(105n * MICROGONS_PER_ARGON_BIGINT);

    expect(vault.availableBondSpace(priceIndex)).toStrictEqual(10n * MICROGONS_PER_ARGON_BIGINT);
  });

  it('returns full securitized bitcoin value capacity when full frame capacity is enabled', () => {
    const vault = vaultWithSecuritizedSatoshis(1);
    const priceIndex = priceIndexForCapacity(105n * MICROGONS_PER_ARGON_BIGINT);

    expect(vault.availableBondSpace(priceIndex, [], true)).toStrictEqual(105n * MICROGONS_PER_ARGON_BIGINT);
  });

  it('subtracts active bond lots from next-frame capacity', () => {
    const vault = vaultWithSecuritizedSatoshis(1);
    const priceIndex = priceIndexForCapacity(100n * MICROGONS_PER_ARGON_BIGINT);

    expect(
      vault.availableBondSpace(priceIndex, [{ activeBonds: 3 }, { activeBonds: 4 }]),
    ).toStrictEqual(3n * MICROGONS_PER_ARGON_BIGINT);
  });

  it('does not return negative capacity when active bond lots exceed capacity', () => {
    const vault = vaultWithSecuritizedSatoshis(1);
    const priceIndex = priceIndexForCapacity(100n * MICROGONS_PER_ARGON_BIGINT);

    expect(
      vault.availableBondSpace(priceIndex, [{ activeBonds: 11 }]),
    ).toStrictEqual(0n);
  });

  it('returns zero when the vault has no securitized satoshis', () => {
    const vault = vaultWithSecuritizedSatoshis(0);
    const priceIndex = priceIndexForCapacity(100n * MICROGONS_PER_ARGON_BIGINT);

    expect(vault.availableBondSpace(priceIndex)).toStrictEqual(0n);
  });
});
