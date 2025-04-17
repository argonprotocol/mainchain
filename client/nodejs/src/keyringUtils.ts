import {
  Keyring,
  KeyringPair,
  mnemonicGenerate,
  IGlobalOptions,
} from './index';
import { promises } from 'node:fs';
import os from 'node:os';

const { readFile, writeFile } = promises;

export async function keyringFromCli(
  opts: IGlobalOptions,
): Promise<KeyringPair> {
  if (opts.accountSuri) {
    return keyringFromSuri(opts.accountSuri!);
  }
  if (opts.accountFilePath) {
    return keyringFromFile({
      filePath: opts.accountFilePath,
      passphrase: opts.accountPassphrase,
    });
  }
  throw new Error(
    'No ACCOUNT account loaded (either ACCOUNT_SURI or ACCOUNT_JSON_PATH required)',
  );
}

export function keyringFromSuri(suri: string): KeyringPair {
  return new Keyring({ type: 'sr25519' }).createFromUri(suri);
}

export async function keyringFromFile(opts: {
  filePath: string;
  passphrase?: string;
}): Promise<KeyringPair> {
  if (!opts.filePath) {
    throw new Error(
      'No ACCOUNT account loaded (either ACCOUNT_SURI or ACCOUNT_JSON_PATH required)',
    );
  }
  const path = opts.filePath.replace('~', os.homedir());
  const json = JSON.parse(await readFile(path, 'utf-8'));
  const mainAccount = new Keyring().createFromJson(json);
  mainAccount.decodePkcs8(opts.passphrase);
  return mainAccount;
}

export async function createKeyringPair(opts: {
  filePath?: string;
  passphrase?: string;
  cryptoType?: 'sr25519' | 'ed25519';
}): Promise<KeyringPair> {
  const { filePath, passphrase, cryptoType } = opts;
  const type = cryptoType ?? 'sr25519';
  const seed = mnemonicGenerate();
  const keyring = new Keyring().createFromUri(seed, { type });
  if (filePath) {
    const json = keyring.toJson(passphrase);
    await writeFile(filePath, JSON.stringify(json, null, 2));
  }
  return keyring;
}
