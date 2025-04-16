import { createKeyringPair, keyringFromFile } from '../keyringUtils';
import fs from 'node:fs';
import { expect, it } from 'vitest';

it('can import and export a keyring', async () => {
  const keyring = await createKeyringPair({
    filePath: '/tmp/keyring.json',
    passphrase: 'test',
    cryptoType: 'sr25519',
  });

  const address = keyring.address;
  keyring.lock();
  expect(keyring.isLocked).toBe(true);
  keyring.unlock('test');
  expect(keyring.isLocked).toBe(false);

  expect(fs.existsSync('/tmp/keyring.json')).toBe(true);

  const decoded = await keyringFromFile({
    filePath: '/tmp/keyring.json',
    passphrase: 'test',
  });
  expect(decoded.address).toBe(address);
});
