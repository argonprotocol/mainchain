use polkadot_sdk::*;
use sc_service::{ChainType, Properties};
use sp_core::sr25519;
use std::env;

use crate::chain_spec::{
	ChainSpec, GenesisSettings, authority_keys_from_seed, get_account_id_from_seed, get_from_seed,
	testnet_genesis,
};
use argon_canary_runtime::WASM_BINARY;
use argon_primitives::{
	ADDRESS_PREFIX, ARGON_TOKEN_SYMBOL, Chain, ComputeDifficulty, TOKEN_DECIMALS,
	bitcoin::BitcoinNetwork,
	block_seal::MiningSlotConfig,
	notary::{GenesisNotary, NotaryPublic},
	tick::Ticker,
};

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), ARGON_TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), ADDRESS_PREFIX.into());

	let notary_host = env::var("ARGON_LOCAL_TESTNET_NOTARY_URL")
		.unwrap_or("ws://127.0.0.1:9925".to_string())
		.into();
	const HASHES_PER_SECOND: u64 = 1_000;
	const TICK_MILLIS: u64 = 10_000;
	let ticker = Ticker::new(TICK_MILLIS, 2);

	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name(&Chain::LocalTestnet.to_string())
	.with_id("argon-local")
	.with_chain_type(ChainType::Local)
	.with_properties(properties)
	.with_genesis_config_patch(testnet_genesis(GenesisSettings {
		// You have to have an authority to start the chain
		founding_grandpas: vec![(authority_keys_from_seed("Alice").grandpa, 10)],
		sudo_key: get_account_id_from_seed::<sr25519::Public>("Alice"),
		bitcoin_network: BitcoinNetwork::Regtest,
		bitcoin_tip_operator: get_account_id_from_seed::<sr25519::Public>("Dave"),
		price_index_operator: get_account_id_from_seed::<sr25519::Public>("Eve"),
		endowed_accounts: vec![
			(get_account_id_from_seed::<sr25519::Public>("Alice"), 100_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), 100_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Charlie"), 100_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Dave"), 100_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Eve"), 100_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Ferdie"), 100_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Ferdie//notary"), 100_000_000),
		],
		initial_vote_minimum: 1_000,
		initial_difficulty: (TICK_MILLIS * HASHES_PER_SECOND / 1_000) as ComputeDifficulty,
		ticker,
		initial_notaries: vec![GenesisNotary {
			account_id: get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			public: get_from_seed::<NotaryPublic>("Ferdie//notary"),
			hosts: vec![notary_host],
			name: "FerdieStamp".into(),
		}],
		mining_config: MiningSlotConfig {
			ticks_before_bid_end_for_vrf_close: 5,
			ticks_between_slots: 10,
			slot_bidding_start_after_ticks: 10,
		},
		minimum_bitcoin_lock_satoshis: 100,
		hyperbridge_token_admin: get_account_id_from_seed::<sr25519::Public>("Alice"),
	}))
	.build())
}
