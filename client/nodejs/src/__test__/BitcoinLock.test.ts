import { BitcoinLock, waitForLoad } from '../index';
import { expect, it } from 'vitest';
import { Keyring } from '@polkadot/api';
import { mnemonicGenerate } from '@polkadot/util-crypto';

it('can generate a coupon proof', async () => {
  await waitForLoad();
  const argonKeyring = new Keyring({ type: 'sr25519' }).addFromUri('//Alice');
  const couponProofKeypair = new Keyring({ type: 'sr25519' }).addFromMnemonic(mnemonicGenerate());
  const result = BitcoinLock.createCouponProof({
    couponProofKeypair,
    argonKeyring,
  });
  expect(result.public).toBeTruthy();
  expect(result.signature).toBeTruthy();
});
