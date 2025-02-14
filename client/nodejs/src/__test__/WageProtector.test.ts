import { WageProtector, waitForLoad } from '../index';

it('adjusts the price of a wage for inflation', async () => {
  await waitForLoad();
  const wageProtector = new WageProtector({
    argonUsdTargetPrice: 1_010_000_000_000_000_000n,
    argonUsdPrice: 1_000_000_000_000_000_000n,
    tick: 1n,
    finalizedBlock: Buffer.from([]),
  });

  // if price of argon is below the target price, we have argon inflation, which means we need to increase the wage
  // to keep the value of the wage stable. It will take 1.01 argon to buy the same amount of goods.
  expect(wageProtector.getProtectedWage(2500n)).toBe(BigInt(2500 * 1.01));
});

it('adjusts the price of a wage for deflation', async () => {
  await waitForLoad();
  const wageProtector = new WageProtector({
    argonUsdTargetPrice: 1_000_000_000_000_000_000n,
    argonUsdPrice: 1_010_000_000_000_000_000n,
    tick: 1n,
    finalizedBlock: Buffer.from([]),
  });

  // if price of argon is below the target price, we have argon deflation, which means we need to decrease the wages to
  // match the argon's market value.
  expect(wageProtector.getProtectedWage(2500n)).toBe(BigInt(2500 * 0.99));
});
