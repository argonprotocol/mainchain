export enum BitcoinNetwork {
  Bitcoin,
  Testnet,
  Regtest,
  Signet,
}

export function createCosignPubkey(
  vaultPubkeyHex: string,
  vaultClaimPubkeyHex: string,
  ownerPubkeyHex: string,
  vaultClaimHeight: bigint | number,
  openClaimHeight: bigint | number,
  createdAtHeight: bigint | number,
  bitcoinNetwork: BitcoinNetwork,
): string;

export function calculateFee(
  vaultPubkeyHex: string,
  vaultClaimPubkeyHex: string,
  ownerPubkeyHex: string,
  vaultClaimHeight: bigint | number,
  openClaimHeight: bigint | number,
  createdAtHeight: bigint | number,
  bitcoinNetwork: BitcoinNetwork,
  feeRateSatsPerVb: bigint | number,
  toScriptPubkey: string,
): bigint;

export function signPsbtDerived(
  psbtHex: string,
  xprivB58: string,
  xprivHdPath: string,
  finalize: boolean,
): string;

export function signPsbt(
  psbtHex: string,
  bitcoinNetwork: BitcoinNetwork,
  privateKeyHex: string,
  finalize: boolean,
): string;

export function getCosignPsbt(
  txid: string,
  vout: number,
  satoshis: bigint | number,
  vaultPubkeyHex: string,
  vaultClaimPubkeyHex: string,
  ownerPubkeyHex: string,
  vaultClaimHeight: bigint | number,
  openClaimHeight: bigint | number,
  createdAtHeight: bigint | number,
  bitcoinNetwork: BitcoinNetwork,
  toScriptPubkeyHex: string,
  bitcoinNetworkFee: bigint | number,
): string;
