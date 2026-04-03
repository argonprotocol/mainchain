import { BitcoinLock, waitForLoad } from '../index';
import { expect, it } from 'vitest';

it('exports bitcoin lock helpers', async () => {
  await waitForLoad();
  expect(BitcoinLock).toBeTruthy();
});
