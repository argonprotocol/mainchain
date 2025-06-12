import { Keyring, KeyringPair, mnemonicGenerate } from './index';

export function keyringFromSuri(
  suri: string,
  cryptoType: 'sr25519' | 'ed25519' = 'sr25519',
): KeyringPair {
  return new Keyring({ type: cryptoType }).createFromUri(suri);
}

export function createKeyringPair(opts: { cryptoType?: 'sr25519' | 'ed25519' }): KeyringPair {
  const { cryptoType } = opts;
  const seed = mnemonicGenerate();
  return keyringFromSuri(seed, cryptoType);
}
