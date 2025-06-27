import { BIP32Interface } from 'bip32';
import { networks, payments, Psbt, Transaction } from 'bitcoinjs-lib';
import {
  ArgonPrimitivesBitcoinBitcoinNetwork,
  IBitcoinLock,
  IReleaseRequest,
} from '@argonprotocol/mainchain';
import {
  BitcoinNetwork,
  calculateFee,
  createCosignPubkey,
  getCosignPsbt,
  signPsbt,
  signPsbtDerived,
} from './wasm/bitcoin_bindings.js';
import { addressBytesHex, keyToBuffer } from './KeysHelper';

export default class CosignScript {
  private readonly network: networks.Network;
  constructor(
    readonly lock: IBitcoinLock,
    network: networks.Network | ArgonPrimitivesBitcoinBitcoinNetwork,
  ) {
    if (
      network === networks.bitcoin ||
      network === networks.testnet ||
      network === networks.regtest
    ) {
      this.network = network as networks.Network;
    } else {
      this.network = CosignScript.getBitcoinJsNetwork(
        network as ArgonPrimitivesBitcoinBitcoinNetwork,
      );
    }
  }

  public getFundingPsbt(): Psbt {
    const { lock, network } = this;
    return new Psbt({ network }).addOutput({
      script: keyToBuffer(lock.p2wshScriptHashHex),
      value: Number(lock.satoshis),
    });
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
      toBitcoinNetwork(network),
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
      toBitcoinNetwork(network),
    );
  }

  public getCosignPsbt(args: {
    utxoRef: { txid: string; vout: number };
    releaseRequest: IReleaseRequest;
  }) {
    const { lock, network } = this;
    const { releaseRequest, utxoRef } = args;

    releaseRequest.toScriptPubkey = addressBytesHex(releaseRequest.toScriptPubkey, network);

    const psbtStr = getCosignPsbt(
      utxoRef.txid,
      utxoRef.vout,
      lock.satoshis,
      lock.vaultPubkey,
      lock.vaultClaimPubkey,
      lock.ownerPubkey,
      BigInt(lock.vaultClaimHeight),
      BigInt(lock.openClaimHeight),
      BigInt(lock.createdAtHeight),
      toBitcoinNetwork(network),
      releaseRequest.toScriptPubkey,
      releaseRequest.bitcoinNetworkFee,
    );
    return this.psbtFromHex(psbtStr);
  }

  psbtFromHex(psbtHex: string): Psbt {
    const psbt = Psbt.fromHex(psbtHex.replace(/^0x(.+)/, '$1'), { network: this.network });
    if (psbt.data.inputs.length === 0) {
      throw new Error('PSBT has no inputs');
    }
    if (psbt.data.outputs.length === 0) {
      throw new Error('PSBT has no outputs');
    }
    return psbt;
  }

  /**
   * Cosigns the PSBT with the vault xpub.
   * @param psbt - The PSBT to cosign.
   * @param lock - The Bitcoin lock containing the vault information.
   * @param vaultXpriv - The vault's extended private key of which the xpub was used to create the vault.
   */
  public vaultCosignPsbt(psbt: Psbt, lock: IBitcoinLock, vaultXpriv: BIP32Interface): Psbt {
    const parentFingerprint = Buffer.from(lock.vaultXpubSources.parentFingerprint);
    if (!parentFingerprint.equals(vaultXpriv.fingerprint)) {
      throw new Error(
        `Vault xpub fingerprint ${parentFingerprint.toString('hex')} does not match the vault xpriv fingerprint ${vaultXpriv.fingerprint.toString('hex')}`,
      );
    }

    const childPath = `${lock.vaultXpubSources.cosignHdIndex}`;
    const pubkey = vaultXpriv.derivePath(childPath).publicKey;
    const vaultPubkey = keyToBuffer(lock.vaultPubkey);
    if (!vaultPubkey.equals(pubkey)) {
      throw new Error(
        `Vault pubkey ${vaultPubkey.toString('hex')} does not match the derived pubkey ${pubkey.toString('hex')} using path ${childPath}`,
      );
    }
    const signedPsbt = signPsbtDerived(psbt.toHex(), vaultXpriv.toBase58(), childPath, false);
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
    ownerXpriv: BIP32Interface;
    ownerXprivChildHdPath?: string;
    addTx?: string;
  }): Transaction {
    const { lock } = this;
    const psbt = this.getCosignPsbt(args);
    const { addTx, vaultCosignature, ownerXpriv, ownerXprivChildHdPath } = args;

    // add the vault signature to the PSBT
    psbt.updateInput(0, {
      partialSig: [
        {
          pubkey: keyToBuffer(lock.vaultPubkey),
          signature: Buffer.from(vaultCosignature),
        },
      ],
    });
    const derivePubkey = ownerXpriv.publicKey;
    const ownerPubkey = keyToBuffer(lock.ownerPubkey);
    if (!ownerPubkey.equals(derivePubkey)) {
      throw new Error(
        `Owner pubkey ${ownerPubkey.toString('hex')} does not match the derived pubkey ${derivePubkey.toString('hex')}`,
      );
    }

    if (addTx) {
      const tx = Transaction.fromHex(addTx.replace(/^0x(.+)/, '$1'));
      for (let i = 0; i < tx.outs.length; i++) {
        const output = tx.outs[i];
        const scripts = [
          payments.p2wpkh({ pubkey: ownerPubkey }).output,
          payments.p2sh({ redeem: payments.p2wpkh({ pubkey: ownerPubkey }) }).output,
          payments.p2pkh({ pubkey: ownerPubkey }).output,
        ];

        if (scripts.some(x => x && output.script.equals(x))) {
          psbt.addInput({
            hash: tx.getId(),
            index: i,
            witnessUtxo: {
              script: output.script,
              value: output.value,
            },
          });
        }
      }
    }

    const signedPsbt = ownerXprivChildHdPath
      ? signPsbtDerived(psbt.toHex(), ownerXpriv.toBase58(), ownerXprivChildHdPath, true)
      : signPsbt(
          psbt.toHex(),
          toBitcoinNetwork(this.network),
          ownerXpriv.privateKey!.toString('hex'),
          true,
        );

    const finalPsbt = this.psbtFromHex(signedPsbt);

    return finalPsbt.extractTransaction();
  }

  public static getBitcoinJsNetwork(network: ArgonPrimitivesBitcoinBitcoinNetwork) {
    if (network.isBitcoin) return networks.bitcoin;
    if (network.isTestnet || network.isSignet) return networks.testnet;
    if (network.isRegtest) return networks.regtest;
    throw new Error('Unsupported network: ' + network);
  }
}

function toBitcoinNetwork(network: networks.Network): BitcoinNetwork {
  if (network === networks.bitcoin) {
    return BitcoinNetwork.Bitcoin;
  } else if (network === networks.testnet) {
    return BitcoinNetwork.Testnet;
  } else if (network === networks.regtest) {
    return BitcoinNetwork.Regtest;
  }
  throw new Error('Unsupported network: ' + network);
}
