import { Vault } from '../Vault';
import type { PriceIndex } from '../PriceIndex';
import { MICROGONS_PER_ARGON } from '../utils';
import { TypeRegistry } from '@polkadot/types';
import type { PalletTreasuryVaultBondState } from '@polkadot/types/lookup';
import { describe, expect, it } from 'vitest';

const MICROGONS_PER_ARGON_BIGINT = BigInt(MICROGONS_PER_ARGON);

describe('Vault.availableBondSpace', () => {
  function vaultWithSecuritizedSatoshis(securitizedSatoshis: number): Vault {
    const vault = Object.create(Vault.prototype) as Vault;
    vault.securitizedSatoshis = securitizedSatoshis;
    vault.backfillSecuritizationLocked = 0n;
    vault.backfillSecuritizedSatoshis = 0;
    vault.backfillSecuritizationReserved = 0n;
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

    expect(vault.availableBondSpace(priceIndex, [], true)).toStrictEqual(
      105n * MICROGONS_PER_ARGON_BIGINT,
    );
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

    expect(vault.availableBondSpace(priceIndex, [{ activeBonds: 11 }])).toStrictEqual(0n);
  });

  it('returns zero when the vault has no securitized satoshis', () => {
    const vault = vaultWithSecuritizedSatoshis(0);
    const priceIndex = priceIndexForCapacity(100n * MICROGONS_PER_ARGON_BIGINT);

    expect(vault.availableBondSpace(priceIndex)).toStrictEqual(0n);
  });

  it('subtracts bonds in lots and reserved backfill bonds', () => {
    const vault = vaultWithSecuritizedSatoshis(1);
    const priceIndex = priceIndexForCapacity(10n * MICROGONS_PER_ARGON_BIGINT);
    const registry = new TypeRegistry();
    registry.register({
      PalletTreasuryBondLotSummary: {
        bondLotId: 'Compact<u64>',
        bonds: 'Compact<u32>',
      },
      PalletTreasuryVaultBondState: {
        bondLots: 'Vec<PalletTreasuryBondLotSummary>',
        backfillBonds: 'Compact<u32>',
        backfillBondsReserved: 'Compact<u32>',
      },
    });
    const bondState = registry.createType('PalletTreasuryVaultBondState', {
      bondLots: [{ bondLotId: 1, bonds: 3 }],
      backfillBonds: 10,
      backfillBondsReserved: 2,
    });

    expect(vault.availableBondSpace(priceIndex, bondState, true)).toStrictEqual(
      5n * MICROGONS_PER_ARGON_BIGINT,
    );
  });
});

describe('Vault.availableSecuritization', () => {
  it('subtracts backfill reservations from public capacity without giving them to the operator', () => {
    const vault = Object.create(Vault.prototype) as Vault;
    vault.securitization = 100n;
    vault.securitizationLocked = 120n;
    vault.backfillSecuritizationLocked = 60n;
    vault.backfillSecuritizationReserved = 10n;
    vault.operatorAccountId = 'operator';

    expect(vault.availableSecuritization('joiner')).toStrictEqual(30n);
    expect(vault.availableSecuritization('operator')).toStrictEqual(0n);
  });
});

describe('Vault.effectiveSecuritizedSatoshis', () => {
  it('preserves backfill bitcoin capacity until replacement bitcoin confirms', () => {
    const vault = Object.create(Vault.prototype) as Vault;
    vault.securitization = 100n;
    vault.securitizationLocked = 120n;
    vault.securitizationPendingActivation = 20n;
    vault.securitizedSatoshis = 100;
    vault.backfillSecuritizationLocked = 100n;
    vault.backfillSecuritizedSatoshis = 100;

    expect(vault.effectiveSecuritizedSatoshis()).toStrictEqual(100n);

    vault.securitizationPendingActivation = 0n;
    vault.securitizedSatoshis = 120;

    expect(vault.effectiveSecuritizedSatoshis()).toStrictEqual(100n);
  });
});
