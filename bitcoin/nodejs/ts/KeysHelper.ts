import { address, networks } from 'bitcoinjs-lib';
import BIP32Factory, { BIP32API, BIP32Interface } from 'bip32';
import * as ecc from 'tiny-secp256k1';
import * as bip39 from 'bip39';

export function getBip32Factory(): BIP32API {
  return BIP32Factory(ecc);
}

export function getChildXpriv(
  bip39Seed: Buffer,
  hdPath: string,
  network?: networks.Network,
): BIP32Interface {
  const root = BIP32Factory(ecc).fromSeed(bip39Seed, network);
  return root.derivePath(hdPath);
}
export function getBip39Seed(mnemonic: string, passphrase?: string): Buffer {
  return bip39.mnemonicToSeedSync(mnemonic, passphrase);
}

export function getXpubFromXpriv(xpriv: BIP32Interface): string {
  return xpriv.neutered().toBase58();
}

export function getCompressedPubkey(pubkey: string | Buffer): Buffer {
  const pubkeyBuffer = keyToBuffer(pubkey);
  if (ecc.isPointCompressed(pubkeyBuffer)) {
    return pubkeyBuffer;
  }
  return Buffer.from(ecc.pointCompress(pubkeyBuffer, true));
}

export function stripLeadingHexPrefix(hex: string): string {
  if (hex.startsWith('0x')) {
    return hex.slice(2);
  }
  return hex;
}

export function addressBytesHex(addressString: string, network: networks.Network): string {
  if (addressString.startsWith('0x')) {
    return addressString;
  }

  console.log('Converting address to bytes:', addressString, network);

  // if the address is all hex, we assume it's a script hash
  if (/^[0-9a-fA-F]+$/.test(addressString) && !addressString.startsWith('bc')) {
    return `0x${addressString}`;
  }
  const scriptbuf = address.toOutputScript(addressString, network);
  return `0x${scriptbuf.toString('hex')}`;
}

export function keyToBuffer(key: string | Buffer): Buffer {
  if (typeof key === 'string') {
    key = stripLeadingHexPrefix(key);
    return Buffer.from(key, 'hex');
  }
  return Buffer.from(key);
}
