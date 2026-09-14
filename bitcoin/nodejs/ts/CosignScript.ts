import { HDKey } from '@scure/bip32';
import { Transaction } from '@scure/btc-signer';
import {
  ArgonPrimitivesBitcoinBitcoinNetwork,
  hexToU8a,
  u8aEq,
  u8aToHex,
} from '@argonprotocol/mainchain';
import {
  BitcoinNetwork,
  calculateFee,
  createCosignPubkey,
  getCosignPsbt,
  signPsbt,
  signPsbtDerived,
} from './wasm/bitcoin_bindings.js';
import { addressBytesHex, keyToU8a } from './KeysHelper';

export type ICosignScriptLock = {
  createdAtHeight: number;
  fundedSatoshis: bigint;
  openClaimHeight: number;
  ownerPubkey: string;
  p2wshScriptHashHex: string;
  securitizedSatoshis: bigint;
  vaultClaimHeight: number;
  vaultClaimPubkey: string;
  vaultPubkey: string;
  vaultXpubSources: {
    parentFingerprint: Uint8Array;
    cosignHdIndex: number;
  };
};

export type IBitcoinReleaseRequest = {
  bitcoinNetworkFee: bigint;
  destinationSatoshis: bigint;
  changeSatoshis: bigint;
  toScriptPubkey: string;
};

export class CosignScript {
  constructor(
    readonly lock: ICosignScriptLock,
    private network: BitcoinNetwork,
  ) {}

  public getFundingPsbt(): Uint8Array {
    const { lock } = this;
    const tx = new Transaction();
    tx.addOutput({
      script: keyToU8a(lock.p2wshScriptHashHex),
      amount: lock.securitizedSatoshis,
    });
    return tx.toPSBT(0);
  }

  public calculateFee(
    feeRatePerSatVb: bigint,
    inputCount: number,
    toScriptPubkey: string,
    hasChange: boolean,
  ): bigint {
    toScriptPubkey = addressBytesHex(toScriptPubkey, this.network);
    const { lock, network } = this;
    return calculateFee(
      lock.vaultPubkey,
      lock.vaultClaimPubkey,
      lock.ownerPubkey,
      BigInt(lock.vaultClaimHeight),
      BigInt(lock.openClaimHeight),
      BigInt(lock.createdAtHeight),
      network,
      feeRatePerSatVb,
      inputCount,
      toScriptPubkey,
      hasChange,
    );
  }

  public calculateScriptPubkey(): string {
    const { lock, network } = this;
    return createCosignPubkey(
      lock.vaultPubkey,
      lock.vaultClaimPubkey,
      lock.ownerPubkey,
      BigInt(lock.vaultClaimHeight),
      BigInt(lock.openClaimHeight),
      BigInt(lock.createdAtHeight),
      network,
    );
  }

  public getCosignPsbt(args: {
    utxos: { utxoRef: { txid: string; vout: number }; satoshis: bigint }[];
    releaseRequest: IBitcoinReleaseRequest;
  }) {
    const { lock, network } = this;
    const { releaseRequest, utxos } = args;

    const toScriptPubkey = addressBytesHex(releaseRequest.toScriptPubkey, network);

    const psbtStr = getCosignPsbt(
      utxos.map(x => ({
        txid: x.utxoRef.txid,
        vout: x.utxoRef.vout,
        satoshis: x.satoshis,
      })),
      lock.vaultPubkey,
      lock.vaultClaimPubkey,
      lock.ownerPubkey,
      BigInt(lock.vaultClaimHeight),
      BigInt(lock.openClaimHeight),
      BigInt(lock.createdAtHeight),
      network,
      toScriptPubkey,
      releaseRequest.destinationSatoshis,
      releaseRequest.changeSatoshis,
      releaseRequest.bitcoinNetworkFee,
    );
    return this.psbtFromHex(psbtStr);
  }

  psbtFromHex(psbtHex: string): Transaction {
    const psbtBytes = hexToU8a(psbtHex);
    const tx = Transaction.fromPSBT(psbtBytes);
    if (tx.inputsLength === 0) {
      throw new Error('PSBT has no inputs');
    }
    if (tx.outputsLength === 0) {
      throw new Error('PSBT has no outputs');
    }
    return tx;
  }

  /**
   * Cosigns the PSBT with the vault xpub.
   * @param psbt - The PSBT to cosign.
   * @param lock - The Bitcoin lock containing the vault information.
   * @param vaultXpriv - The vault's extended private key of which the xpub was used to create the vault.
   */
  public vaultCosignPsbt(
    psbt: Transaction,
    lock: ICosignScriptLock,
    vaultXpriv: HDKey,
  ): Transaction {
    const parentFingerprint = lock.vaultXpubSources.parentFingerprint;
    const vaultFingerprint = vaultXpriv.identifier?.slice(0, 4);
    if (!vaultFingerprint) {
      throw new Error('Could not get vault fingerprint from HDKey');
    }
    if (!u8aEq(parentFingerprint, vaultFingerprint)) {
      throw new Error(
        `Vault xpub fingerprint ${u8aToHex(parentFingerprint)} does not match the vault xpriv fingerprint ${u8aToHex(vaultFingerprint)}`,
      );
    }

    const childPath = `${lock.vaultXpubSources.cosignHdIndex}`;
    const pubkey = vaultXpriv.deriveChild(lock.vaultXpubSources.cosignHdIndex).publicKey;
    if (!pubkey) {
      throw new Error(`Failed to derive public key for path ${childPath}`);
    }
    const vaultPubkey = keyToU8a(lock.vaultPubkey);
    if (!u8aEq(vaultPubkey, pubkey)) {
      throw new Error(
        `Vault pubkey ${u8aToHex(vaultPubkey)} does not match the derived pubkey ${u8aToHex(pubkey)} using path ${childPath}`,
      );
    }
    const signedPsbt = signPsbtDerived(
      u8aToHex(psbt.toPSBT()),
      vaultXpriv.privateExtendedKey,
      childPath,
      false,
    );
    psbt = this.psbtFromHex(signedPsbt);

    return psbt;
  }

  /**
   * Cosigns the transaction.
   */
  public cosignAndGenerateTx(args: {
    releaseRequest: IBitcoinReleaseRequest;
    vaultCosignatures: Uint8Array[];
    utxos: { utxoRef: { txid: string; vout: number }; satoshis: bigint }[];
    ownerXpriv: HDKey;
    ownerXprivChildHdPath?: string;
  }): Transaction {
    const { lock } = this;
    const psbt = this.getCosignPsbt(args);
    const { vaultCosignatures, ownerXpriv, ownerXprivChildHdPath } = args;

    if (vaultCosignatures.length !== psbt.inputsLength) {
      throw new Error('Vault signature count does not match the Bitcoin lock input count');
    }
    for (let i = 0; i < vaultCosignatures.length; i++) {
      psbt.updateInput(i, {
        partialSig: [[keyToU8a(lock.vaultPubkey), vaultCosignatures[i]]],
      });
    }
    const derivePubkey = ownerXprivChildHdPath
      ? ownerXpriv.derive(ownerXprivChildHdPath).publicKey
      : ownerXpriv.publicKey;
    if (!derivePubkey) {
      throw new Error('Failed to derive owner public key');
    }
    const ownerPubkey = keyToU8a(lock.ownerPubkey);
    if (!u8aEq(ownerPubkey, derivePubkey)) {
      throw new Error(
        `Owner pubkey ${u8aToHex(ownerPubkey)} does not match the derived pubkey ${u8aToHex(derivePubkey)}`,
      );
    }

    const psbtBytes = u8aToHex(psbt.toPSBT());
    const signedPsbt = ownerXprivChildHdPath
      ? signPsbtDerived(psbtBytes, ownerXpriv.privateExtendedKey, ownerXprivChildHdPath, true)
      : signPsbt(psbtBytes, this.network, u8aToHex(ownerXpriv.privateKey, undefined, false), true);
    return this.psbtFromHex(signedPsbt);
  }
}

export function getBitcoinNetworkFromApi(
  network: ArgonPrimitivesBitcoinBitcoinNetwork,
): BitcoinNetwork {
  if (network.isBitcoin) {
    return BitcoinNetwork.Bitcoin;
  } else if (network.isTestnet) {
    return BitcoinNetwork.Testnet;
  } else if (network.isRegtest) {
    return BitcoinNetwork.Regtest;
  } else if (network.isSignet) {
    return BitcoinNetwork.Signet;
  }
  throw new Error('Unsupported network: ' + network.toString());
}
