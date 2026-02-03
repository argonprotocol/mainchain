import { HDKey } from '@scure/bip32';
import { p2pkh, p2sh, p2wpkh, p2wsh, Transaction } from '@scure/btc-signer';
import {
  ArgonPrimitivesBitcoinBitcoinNetwork,
  hexToU8a,
  IBitcoinLock,
  IReleaseRequest,
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
import { addressBytesHex, getChildXpriv, getScureNetwork, keyToU8a } from './KeysHelper';

export class CosignScript {
  constructor(
    readonly lock: IBitcoinLock,
    private network: BitcoinNetwork,
  ) {}

  public getFundingPsbt(): Uint8Array {
    const { lock, network } = this;
    const tx = new Transaction();
    tx.addOutput({
      script: keyToU8a(lock.p2wshScriptHashHex),
      amount: lock.utxoSatoshis ?? lock.satoshis,
    });
    return tx.toPSBT(0);
  }

  public calculateFee(feeRatePerSatVb: bigint, toScriptPubkey: string): bigint {
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
      toScriptPubkey,
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
    utxoRef: { txid: string; vout: number };
    releaseRequest: IReleaseRequest;
    // Optional override for orphaned UTXOs that don't match the lock's expected amount.
    utxoSatoshis?: bigint;
  }) {
    const { lock, network } = this;
    const { releaseRequest, utxoRef, utxoSatoshis } = args;

    releaseRequest.toScriptPubkey = addressBytesHex(releaseRequest.toScriptPubkey, network);

    const psbtStr = getCosignPsbt(
      utxoRef.txid,
      utxoRef.vout,
      utxoSatoshis ?? lock.utxoSatoshis ?? lock.satoshis,
      lock.vaultPubkey,
      lock.vaultClaimPubkey,
      lock.ownerPubkey,
      BigInt(lock.vaultClaimHeight),
      BigInt(lock.openClaimHeight),
      BigInt(lock.createdAtHeight),
      network,
      releaseRequest.toScriptPubkey,
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
  public vaultCosignPsbt(psbt: Transaction, lock: IBitcoinLock, vaultXpriv: HDKey): Transaction {
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
    releaseRequest: IReleaseRequest;
    vaultCosignature: Uint8Array;
    utxoRef: { txid: string; vout: number };
    ownerXpriv: HDKey;
    ownerXprivChildHdPath?: string;
    addTx?: string;
  }): Transaction {
    const { lock } = this;
    const psbt = this.getCosignPsbt(args);
    const { addTx, vaultCosignature, ownerXpriv, ownerXprivChildHdPath } = args;

    // add the vault signature to the PSBT
    psbt.updateInput(0, {
      partialSig: [[keyToU8a(lock.vaultPubkey), vaultCosignature]],
    });
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

    if (addTx) {
      const addTxBytes = hexToU8a(addTx);
      const tx = Transaction.fromPSBT(addTxBytes);
      for (let i = 0; i < tx.outputsLength; i++) {
        const output = tx.getOutput(i);
        const network = getScureNetwork(this.network);
        const scripts = [
          p2wpkh(ownerPubkey, network).script,
          p2wsh(p2wpkh(ownerPubkey, network), network).script,
          p2sh(p2pkh(ownerPubkey, network), network).script,
          p2pkh(ownerPubkey, network).script,
        ];

        if (scripts.some(x => x && output.script && u8aEq(output.script, x))) {
          psbt.addInput({
            txid: tx.id,
            index: i,
            witnessUtxo: {
              script: output.script!,
              amount: output.amount!,
            },
          });
        }
      }
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
