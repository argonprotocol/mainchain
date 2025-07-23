import { Address, NETWORK, OutScript, TEST_NETWORK } from '@scure/btc-signer';
import { bech32 } from '@scure/base';
import { HDKey, Versions } from '@scure/bip32';
import { hexToU8a, u8aToHex } from '@argonprotocol/mainchain';
import * as secp256k1 from '@noble/secp256k1';
import { BTC_NETWORK } from '@scure/btc-signer/utils';
import { BitcoinNetwork } from './wasm/bitcoin_bindings';

export { HDKey, BitcoinNetwork };

export function getBip32Version(network?: BitcoinNetwork): Versions | undefined {
  if (!network) {
    return undefined;
  }

  if (network === BitcoinNetwork.Testnet || network === BitcoinNetwork.Signet) {
    return {
      private: 0x04358394, // tprv
      public: 0x043587cf, // tpub
    };
  }
  if (network === BitcoinNetwork.Regtest) {
    return {
      private: 0x04358394, // rprv
      public: 0x043587cf, // rpub
    };
  }
  // If the network is Bitcoin, we return undefined to use the default BIP32 version
  return undefined;
}

export function getChildXpriv(
  bip39Seed: Uint8Array,
  hdPath: string,
  network?: BitcoinNetwork,
): HDKey {
  const root = HDKey.fromMasterSeed(bip39Seed, getBip32Version(network));
  return root.derive(hdPath);
}

export function getXpubFromXpriv(xpriv: HDKey): string {
  return xpriv.publicExtendedKey;
}

export function getCompressedPubkey(pubkey: string | Uint8Array): Uint8Array {
  const pubkeyUint8Array = keyToU8a(pubkey);

  // If already compressed (33 bytes), return as-is
  if (pubkeyUint8Array.length === 33) {
    return pubkeyUint8Array;
  }

  // Use @noble/secp256k1 to properly compress the point
  const point = secp256k1.Point.fromHex(pubkeyUint8Array);
  return point.toRawBytes(true); // true = compressed
}

export function getBech32Prefix(network: BitcoinNetwork): string {
  return getScureNetwork(network).bech32;
}

export function p2wshScriptHexToAddress(scriptPubKeyHex: string, network: BitcoinNetwork): string {
  const script = hexToU8a(scriptPubKeyHex);
  if (Buffer.byteLength(script) !== 34 || script[0] !== 0x00 || script[1] !== 0x20) {
    throw new Error('Invalid P2WSH scriptPubKey');
  }

  const witnessProgram = script.slice(2);
  const version = 0; // P2WSH uses version 0
  const prefix = getBech32Prefix(network);
  return bech32.encode(prefix, [version, ...bech32.toWords(witnessProgram)]);
}

export function addressBytesHex(address: string, network: BitcoinNetwork): string {
  if (address.startsWith('0x')) {
    return address;
  }
  const bech32Prefix = getBech32Prefix(network);
  // if the address is all hex, we assume it's a script hash
  if (
    /^[0-9a-fA-F]+$/.test(address) &&
    !address.startsWith('bc') &&
    !address.startsWith(bech32Prefix)
  ) {
    return `0x${address}`;
  }
  const btcNetwork = getScureNetwork(network);
  const decoded = Address(btcNetwork).decode(address);
  const out = OutScript.encode(decoded);
  return u8aToHex(out);
}

export function keyToU8a(pubkey: string | Uint8Array): Uint8Array {
  return typeof pubkey === 'string' ? hexToU8a(pubkey) : pubkey;
}

export function getScureNetwork(network: BitcoinNetwork): BTC_NETWORK {
  if (network === BitcoinNetwork.Bitcoin) {
    return NETWORK;
  } else if (network === BitcoinNetwork.Testnet || network === BitcoinNetwork.Signet) {
    return TEST_NETWORK;
  } else {
    return {
      bech32: 'bcrt',
      pubKeyHash: 0x6f,
      scriptHash: 0xc4,
      wif: 0xef,
    };
  }
}
