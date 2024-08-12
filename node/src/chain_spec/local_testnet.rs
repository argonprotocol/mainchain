use sc_service::{ChainType, Properties};
use sp_core::sr25519;
use std::env;

use crate::chain_spec::{
	authority_keys_from_seed, get_account_id_from_seed, get_from_seed, testnet_genesis, ChainSpec,
};
use argon_node_runtime::WASM_BINARY;
use argon_primitives::{
	bitcoin::BitcoinNetwork,
	block_seal::MiningSlotConfig,
	notary::{GenesisNotary, NotaryPublic},
	ComputeDifficulty, ADDRESS_PREFIX,
};

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "ARGON".into());
	properties.insert("tokenDecimals".into(), 3.into());
	properties.insert("ss58Format".into(), ADDRESS_PREFIX.into());

	let notary_host = env::var("ARGON_LOCAL_TESTNET_NOTARY_URL")
		.unwrap_or("ws://127.0.0.1:9925".to_string())
		.into();
	const HASHES_PER_SECOND: u64 = 1_000;
	const TICK_MILLIS: u64 = 2000;
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_properties(properties)
	.with_genesis_config_patch(testnet_genesis(
		// Initial BlockSeal authorities
		vec![(
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			authority_keys_from_seed("Alice"),
		)],
		// Sudo account
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		BitcoinNetwork::Regtest,
		// Bitcoin utxo tip operator
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		// Price index operator
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		// Pre-funded accounts
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//notary"),
		],
		500,
		(TICK_MILLIS * HASHES_PER_SECOND / 1_000) as ComputeDifficulty,
		TICK_MILLIS,
		vec![GenesisNotary {
			account_id: get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			public: get_from_seed::<NotaryPublic>("Ferdie//notary"),
			hosts: vec![notary_host],
			name: "FerdieStamp".into(),
		}],
		2,
		MiningSlotConfig {
			blocks_before_bid_end_for_vrf_close: 1,
			blocks_between_slots: 4,
			slot_bidding_start_block: 4,
		},
	))
	.build())
}
