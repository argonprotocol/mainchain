import { expect, it } from 'vitest';
import { allocateFlexibleBondEarnings } from '../scripts/backfillFlexibleBondMetrics';

const SCALE = 10n ** 18n;

it('attributes only each historical bond share from the flexible treasury slice', () => {
  const earnings = allocateFlexibleBondEarnings(1_000n, SCALE / 2n, [
    { bondLotId: 1, bonds: 10n, sharingPercent: 100_000n, bonusPercent: 0n },
    { bondLotId: 2, bonds: 10n, sharingPercent: 200_000n, bonusPercent: 0n },
  ]);

  expect(earnings.get(1)).toBe(25n);
  expect(earnings.get(2)).toBe(50n);
  expect([...earnings.values()].reduce((total, amount) => total + amount, 0n)).toBe(75n);
});

it('includes the lot bonus in its historical bond share', () => {
  const earnings = allocateFlexibleBondEarnings(1_000n, SCALE / 2n, [
    { bondLotId: 1, bonds: 10n, sharingPercent: 100_000n, bonusPercent: 50_000n },
  ]);

  expect(earnings.get(1)).toBe(75n);
});

it('leaves profit-share rounding with the vault instead of giving it to another bond', () => {
  const earnings = allocateFlexibleBondEarnings(5n, SCALE, [
    { bondLotId: 2, bonds: 1n, sharingPercent: 500_000n, bonusPercent: 0n },
    { bondLotId: 1, bonds: 1n, sharingPercent: 500_000n, bonusPercent: 0n },
  ]);

  expect(earnings.get(1)).toBe(1n);
  expect(earnings.get(2)).toBe(1n);
});

it('preserves deterministic gross dust assignment before applying profit sharing', () => {
  const earnings = allocateFlexibleBondEarnings(5n, SCALE, [
    { bondLotId: 2, bonds: 1n, sharingPercent: 1_000_000n, bonusPercent: 0n },
    { bondLotId: 1, bonds: 1n, sharingPercent: 1_000_000n, bonusPercent: 0n },
  ]);

  expect(earnings.get(1)).toBe(3n);
  expect(earnings.get(2)).toBe(2n);
});

it("does not redistribute a zero-share lot's operator income to other bond lots", () => {
  const earnings = allocateFlexibleBondEarnings(1_000n, SCALE, [
    { bondLotId: 1, bonds: 1n, sharingPercent: 0n, bonusPercent: 0n },
    { bondLotId: 2, bonds: 1n, sharingPercent: 1_000_000n, bonusPercent: 0n },
  ]);

  expect(earnings.get(1)).toBe(0n);
  expect(earnings.get(2)).toBe(500n);
});
