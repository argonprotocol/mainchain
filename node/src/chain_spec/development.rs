use sc_service::{ChainType, Properties};
use sp_core::sr25519;
use std::env;

use crate::chain_spec::{
	authority_keys_from_seed, get_account_id_from_seed, testnet_genesis, ChainSpec, GenesisSettings,
};
use argon_canary_runtime::WASM_BINARY;
use argon_primitives::{
	bitcoin::{BitcoinNetwork, SATOSHIS_PER_BITCOIN},
	block_seal::MiningSlotConfig,
	tick::Ticker,
	Chain, ComputeDifficulty, ADDRESS_PREFIX, ARGON_TOKEN_SYMBOL, TOKEN_DECIMALS,
};

pub fn development_config() -> Result<ChainSpec, String> {
	let mut properties = Properties::new();
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("tokenSymbol".into(), ARGON_TOKEN_SYMBOL.into());
	properties.insert("ss58Format".into(), ADDRESS_PREFIX.into());

	let hashes_per_second: u64 = if env::var("CI").is_ok() { 100 } else { 200 };
	const TICK_MILLIS: u64 = 2000;

	let ticker = Ticker::new(TICK_MILLIS, 2);
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name(&Chain::Devnet.to_string())
	.with_id("argon-dev")
	.with_chain_type(ChainType::Development)
	.with_properties(properties)
	.with_genesis_config_patch(testnet_genesis(GenesisSettings {
		// You have to have an authority to start the chain
		founding_grandpas: vec![(authority_keys_from_seed("Alice").grandpa, 1)],
		sudo_key: get_account_id_from_seed::<sr25519::Public>("Alice"),
		bitcoin_network: BitcoinNetwork::Regtest,
		bitcoin_tip_operator: get_account_id_from_seed::<sr25519::Public>("Dave"),
		price_index_operator: get_account_id_from_seed::<sr25519::Public>("Eve"),
		endowed_accounts: vec![
			(get_account_id_from_seed::<sr25519::Public>("Alice"), 100_000_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), 100_000_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Ferdie"), 100_000_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Eve"), 100_000_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Dave"), 100_000_000_000),
		],
		ticker,
		initial_vote_minimum: 1_000,
		initial_difficulty: (TICK_MILLIS * hashes_per_second / 1_000) as ComputeDifficulty,
		initial_notaries: vec![], // No notaries
		mining_config: MiningSlotConfig {
			blocks_before_bid_end_for_vrf_close: 1,
			blocks_between_slots: 4,
			slot_bidding_start_after_ticks: 4,
		},
		minimum_bitcoin_bond_satoshis: SATOSHIS_PER_BITCOIN / 1_000,
		hyperbridge_token_admin: get_account_id_from_seed::<sr25519::Public>("Alice"),
	}))
	.build())
}
