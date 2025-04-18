import { createKeyringPair, keyringFromFile } from '../keyringUtils';
import * as fs from 'node:fs';
import { expect, it } from 'vitest';
import * as Path from 'node:path';

it('can import and export a keyring', async () => {
  const tmpFile = Path.resolve(fs.mkdtempSync('key'), 'keyring.json');
  const keyring = await createKeyringPair({
    filePath: tmpFile,
    passphrase: 'test',
    cryptoType: 'sr25519',
  });

  const address = keyring.address;
  keyring.lock();
  expect(keyring.isLocked).toBe(true);
  keyring.unlock('test');
  expect(keyring.isLocked).toBe(false);

  expect(fs.existsSync(tmpFile)).toBe(true);

  const decoded = await keyringFromFile({
    filePath: tmpFile,
    passphrase: 'test',
  });
  expect(decoded.address).toBe(address);
});
