use crate::chain_spec::{testnet_genesis, ChainSpec, GenesisSettings};
use argon_canary_runtime::{SessionKeys, WASM_BINARY};
use argon_primitives::{
	bitcoin::BitcoinNetwork,
	block_seal::MiningSlotConfig,
	notary::{GenesisNotary, NotaryPublic},
	AccountId, Chain, ComputeDifficulty, ADDRESS_PREFIX, ARGON_TOKEN_SYMBOL, TOKEN_DECIMALS,
};
use codec::Decode;
use sc_service::{ChainType, Properties};
use std::{fmt::format, str::FromStr};

pub fn testnet_config() -> Result<ChainSpec, String> {
	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), ARGON_TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), ADDRESS_PREFIX.into());

	const HASHES_PER_SECOND: u64 = 200;
	const TICK_MILLIS: u64 = 60_000;

	let sudo_account = AccountId::from_str("5EZxgPRoQDYW72ceBKTd8AcSPizNL38cVjBzDGTqbeHUPfRx")?;
	let bitcoin_oracle = AccountId::from_str("5GZGFLKPxKegnjudkiy32gU6JmiKFgt6cJ35udQpSjnPu8PY")?;
	let price_oracle = AccountId::from_str("5Gp8fDqBvgVj3BepUCeyGHEeguy1Jmeb2gfwcFvG8snV4icd")?;
	let token_admin = sudo_account.clone();

	let notary_account = AccountId::from_str("5CFiHEZUFSqwEeiSqJwfxjp4wZWxom73y5EjVsrAw3GwQuWh")?;
	let notary_public = NotaryPublic::from_str("5EX7HEsDt3nn3rsLttPiApADgFvVWmUpWdjU1UL3qgbNLnJ8")
		.map_err(|e| format!("Error parsing notary public {:?}", e))?;

	let miner_zero_keys = SessionKeys::decode(
		&mut &hex::decode("1e69c7672dfb67dc19abfce302caf4c60cc5cb21f39538f749efdc6a28feaba695d5d04b29524e535278e511ac0ec98e4bb76b08a9a6874c5c481f338d727e60")
			.map_err(|e| format!("Error processing miner zero authority key hex {:?}", e))?[..]
	).map_err(|e| format!("Error decoding miner zero authority keys {:?}", e))?;

	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Wasm not available".to_string())?,
		None
	)
	.with_name(&Chain::Testnet.to_string())
	.with_id("argon-testnet")
	.with_protocol_id("targon")
	.with_chain_type(ChainType::Custom("Testnet".into()))
	.with_properties(properties)
	.with_boot_nodes(vec![
		"/ip4/206.189.181.146/tcp/30333/p2p/12D3KooWHY6UgabjbYJ8xZN4xae3tPHhx6BdwtB6Fh9VjkmErkCF".parse().map_err(|e| format!("Unable to parse multiaddr {e:?}"))?,
		"/ip4/206.189.181.146/tcp/30333/ws/p2p/12D3KooWHY6UgabjbYJ8xZN4xae3tPHhx6BdwtB6Fh9VjkmErkCF".parse().map_err(|e| format!("Unable to parse multiaddr {e:?}"))?,
		"/dns/bootnode0.testnet.argonprotocol.org/tcp/30333/p2p/12D3KooWHY6UgabjbYJ8xZN4xae3tPHhx6BdwtB6Fh9VjkmErkCF".parse().map_err(|e| format!("Unable to parse multiaddr {e:?}"))?,
		"/dns/bootnode0.testnet.argonprotocol.org/tcp/30333/ws/p2p/12D3KooWHY6UgabjbYJ8xZN4xae3tPHhx6BdwtB6Fh9VjkmErkCF".parse().map_err(|e| format!("Unable to parse multiaddr {e:?}"))?
	])
	.with_genesis_config_patch(testnet_genesis(
		GenesisSettings {
			// You have to have an authority to start the chain
			founding_grandpas: vec![(miner_zero_keys.grandpa, 10)],
			sudo_key: sudo_account.clone(),
			bitcoin_network: BitcoinNetwork::Signet,
			bitcoin_tip_operator: bitcoin_oracle.clone(),
			price_index_operator: price_oracle.clone(),
			endowed_accounts: vec![
				// funds for the sudo account
				(sudo_account, 10_000_000),
				// basic funds so an oracle can submit a price
				(bitcoin_oracle, 1_000_000),
				// oracle funds
				(price_oracle, 1_000_000),
			],
			initial_vote_minimum: 1_000,
			initial_difficulty: (TICK_MILLIS * HASHES_PER_SECOND / 1_000) as ComputeDifficulty,
			tick_millis: TICK_MILLIS,
			initial_notaries: vec![GenesisNotary {
				account_id: notary_account,
				public: notary_public,
				hosts: vec!["wss://notary1.testnet.argonprotocol.org".to_string().into()],
				name: "Argon Foundation".into(),
			}],
			channel_hold_expiration_ticks: 60,
			mining_config:
			MiningSlotConfig {
				blocks_before_bid_end_for_vrf_close: 200,
				blocks_between_slots: 1440,
				slot_bidding_start_block: 0,
			},
			minimum_bitcoin_bond_satoshis: 5_000,
			hyperbridge_token_admin: token_admin,
		}
	))
	.build())
}
