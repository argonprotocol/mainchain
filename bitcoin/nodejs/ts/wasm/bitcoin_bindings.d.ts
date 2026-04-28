/* tslint:disable */
/* eslint-disable */

export enum BitcoinNetwork {
    /**
     * Mainnet Bitcoin.
     */
    Bitcoin = 0,
    /**
     * Bitcoin's testnet network.
     */
    Testnet = 1,
    /**
     * Bitcoin's signet network
     */
    Signet = 2,
    /**
     * Bitcoin's regtest network.
     */
    Regtest = 3,
}

export function calculateFee(vault_pubkey_hex: string, vault_claim_pubkey_hex: string, owner_pubkey_hex: string, vault_claim_height: bigint, open_claim_height: bigint, created_at_height: bigint, bitcoin_network: BitcoinNetwork, fee_rate_sats_per_vb: bigint, to_script_pubkey: string): bigint;

export function createCosignPubkey(vault_pubkey_hex: string, vault_claim_pubkey_hex: string, owner_pubkey_hex: string, vault_claim_height: bigint, open_claim_height: bigint, created_at_height: bigint, bitcoin_network: BitcoinNetwork): string;

export function getCosignPsbt(txid: string, vout: number, satoshis: bigint, vault_pubkey_hex: string, vault_claim_pubkey_hex: string, owner_pubkey_hex: string, vault_claim_height: bigint, open_claim_height: bigint, created_at_height: bigint, bitcoin_network: BitcoinNetwork, to_script_pubkey_hex: string, bitcoin_network_fee: bigint): string;

export function signPsbt(psbt_hex: string, bitcoin_network: BitcoinNetwork, private_key_hex: string, finalize: boolean): string;

export function signPsbtDerived(psbt_hex: string, xpriv_b58: string, xpriv_hd_path: string, finalize: boolean): string;
