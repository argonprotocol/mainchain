use sc_service::{ChainType, Properties};
use sp_core::sr25519;

use crate::chain_spec::{
	authority_keys_from_seed, get_account_id_from_seed, testnet_genesis, ChainSpec,
};
use argon_node_runtime::WASM_BINARY;
use argon_primitives::{
	bitcoin::{BitcoinNetwork, SATOSHIS_PER_BITCOIN},
	block_seal::MiningSlotConfig,
	Chain, ComputeDifficulty, ADDRESS_PREFIX,
};

pub fn development_config() -> Result<ChainSpec, String> {
	let mut properties = Properties::new();
	properties.insert("tokenDecimals".into(), 3.into());
	properties.insert("tokenSymbol".into(), "ARGON".into());
	properties.insert("ss58Format".into(), ADDRESS_PREFIX.into());

	const HASHES_PER_SECOND: u64 = 10;
	const TICK_MILLIS: u64 = 2000;

	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name(&Chain::Devnet.to_string())
	.with_id("argon-dev")
	.with_chain_type(ChainType::Development)
	.with_properties(properties)
	.with_genesis_config_patch(testnet_genesis(
		// You have to have an authority to start the chain
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
			(get_account_id_from_seed::<sr25519::Public>("Alice"), 100_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), 100_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Ferdie"), 100_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Eve"), 100_000_000),
			(get_account_id_from_seed::<sr25519::Public>("Dave"), 100_000_000),
		],
		500,
		(TICK_MILLIS * HASHES_PER_SECOND / 1_000) as ComputeDifficulty,
		TICK_MILLIS,
		vec![], // No notaries
		2,
		MiningSlotConfig {
			blocks_before_bid_end_for_vrf_close: 1,
			blocks_between_slots: 4,
			slot_bidding_start_block: 4,
		},
		SATOSHIS_PER_BITCOIN / 10,
	))
	.build())
}
