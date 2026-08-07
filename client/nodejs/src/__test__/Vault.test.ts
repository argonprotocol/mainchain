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
    vault.flexibleSecuritizationLocked = 0n;
    vault.flexibleSecuritizedSatoshis = 0;
    vault.reservedSecuritizationSpace = 0n;
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

  it('subtracts regular bonds and reserved bond space without counting flexible bonds twice', () => {
    const vault = vaultWithSecuritizedSatoshis(1);
    const priceIndex = priceIndexForCapacity(10n * MICROGONS_PER_ARGON_BIGINT);
    const registry = new TypeRegistry();
    registry.register({
      PalletTreasuryBondLotSummary: {
        bondLotId: 'Compact<u64>',
        bonds: 'Compact<u32>',
      },
      PalletTreasuryVaultBondState: {
        regularBondLots: 'Vec<PalletTreasuryBondLotSummary>',
        flexibleBonds: 'Compact<u32>',
        reservedBondSpace: 'Compact<u32>',
      },
    });
    const bondState = registry.createType('PalletTreasuryVaultBondState', {
      regularBondLots: [{ bondLotId: 1, bonds: 3 }],
      flexibleBonds: 10,
      reservedBondSpace: 2,
    });

    expect(vault.availableBondSpace(priceIndex, bondState, true)).toStrictEqual(
      5n * MICROGONS_PER_ARGON_BIGINT,
    );
  });

  it('returns zero when only flexible bonds remain and bitcoin capacity is zero', () => {
    const vault = vaultWithSecuritizedSatoshis(0);
    const priceIndex = priceIndexForCapacity(0n);
    const registry = new TypeRegistry();
    registry.register({
      PalletTreasuryBondLotSummary: {
        bondLotId: 'Compact<u64>',
        bonds: 'Compact<u32>',
      },
      PalletTreasuryVaultBondState: {
        regularBondLots: 'Vec<PalletTreasuryBondLotSummary>',
        flexibleBonds: 'Compact<u32>',
        reservedBondSpace: 'Compact<u32>',
      },
    });
    const bondState = registry.createType('PalletTreasuryVaultBondState', {
      regularBondLots: [{ bondLotId: 1, bonds: 3 }],
      flexibleBonds: 10,
      reservedBondSpace: 2,
    });

    expect(vault.availableBondSpace(priceIndex, bondState, true)).toStrictEqual(0n);
  });

  it('reads reserved bond capacity from the previous runtime codec', () => {
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

describe('Vault.availableSecuritizationSpace', () => {
  it('subtracts capacity reservations from operator capacity', () => {
    const vault = Object.create(Vault.prototype) as Vault;
    vault.securitization = 100n;
    vault.securitizationLocked = 80n;
    vault.flexibleSecuritizationLocked = 40n;
    vault.reservedSecuritizationSpace = 50n;
    vault.operatorAccountId = 'operator';

    expect(vault.availableSecuritizationSpace('joiner')).toStrictEqual(10n);
    expect(vault.availableSecuritizationSpace('operator')).toStrictEqual(10n);
  });

  it('subtracts capacity reservations from public capacity without giving them to the operator', () => {
    const vault = Object.create(Vault.prototype) as Vault;
    vault.securitization = 100n;
    vault.securitizationLocked = 120n;
    vault.flexibleSecuritizationLocked = 60n;
    vault.reservedSecuritizationSpace = 10n;
    vault.operatorAccountId = 'operator';

    expect(vault.availableSecuritizationSpace('joiner')).toStrictEqual(30n);
    expect(vault.availableSecuritizationSpace('operator')).toStrictEqual(0n);
  });
});

describe('Vault runtime codec compatibility', () => {
  it('loads previous runtime fields into the new public names', () => {
    const registry = new TypeRegistry();
    registry.register({
      PreviousVault: {
        operatorAccountId: 'AccountId32',
        delegateAccountId: 'Option<AccountId32>',
        securitization: 'Compact<u128>',
        securitizationTarget: 'Compact<u128>',
        securitizationLocked: 'Compact<u128>',
        backfillSecuritizationLocked: 'Compact<u128>',
        backfillSecuritizationReserved: 'Compact<u128>',
        securitizationPendingActivation: 'Compact<u128>',
        lockedSatoshis: 'Compact<u64>',
        securitizedSatoshis: 'Compact<u64>',
        backfillSecuritizedSatoshis: 'Compact<u64>',
        securitizationReleaseSchedule: 'BTreeMap<u64, u128>',
        securitizationRatio: 'Compact<u128>',
        isClosed: 'bool',
        terms: 'PreviousVaultTerms',
        pendingTerms: 'Option<(u64, PreviousVaultTerms)>',
        openedTick: 'Compact<u64>',
      },
      PreviousVaultTerms: {
        bitcoinAnnualPercentRate: 'Compact<u128>',
        bitcoinBaseFee: 'Compact<u128>',
        treasuryProfitSharing: 'Compact<u32>',
        treasuryBonusProfitSharing: 'Compact<u32>',
      },
    });
    const codec = registry.createType('PreviousVault', {
      operatorAccountId: `0x${'01'.repeat(32)}`,
      delegateAccountId: null,
      securitization: 100,
      securitizationTarget: 100,
      securitizationLocked: 70,
      backfillSecuritizationLocked: 40,
      backfillSecuritizationReserved: 20,
      securitizationPendingActivation: 0,
      lockedSatoshis: 50,
      securitizedSatoshis: 50,
      backfillSecuritizedSatoshis: 30,
      securitizationReleaseSchedule: {},
      securitizationRatio: '1000000000000000000',
      isClosed: false,
      terms: {
        bitcoinAnnualPercentRate: 0,
        bitcoinBaseFee: 0,
        treasuryProfitSharing: 0,
        treasuryBonusProfitSharing: 0,
      },
      pendingTerms: null,
      openedTick: 1,
    });

    const vault = new Vault(1, codec as never, 1_000);

    expect(vault.flexibleSecuritizationLocked).toStrictEqual(40n);
    expect(vault.reservedSecuritizationSpace).toStrictEqual(20n);
    expect(vault.flexibleSecuritizedSatoshis).toStrictEqual(30);
  });
});

describe('Vault.effectiveSecuritizedSatoshis', () => {
  it('preserves flexible bitcoin capacity until replacement bitcoin confirms', () => {
    const vault = Object.create(Vault.prototype) as Vault;
    vault.securitization = 100n;
    vault.securitizationLocked = 120n;
    vault.securitizationPendingActivation = 20n;
    vault.securitizedSatoshis = 100;
    vault.flexibleSecuritizationLocked = 100n;
    vault.flexibleSecuritizedSatoshis = 100;

    expect(vault.effectiveSecuritizedSatoshis()).toStrictEqual(100n);

    vault.securitizationPendingActivation = 0n;
    vault.securitizedSatoshis = 120;

    expect(vault.effectiveSecuritizedSatoshis()).toStrictEqual(100n);
  });
});
