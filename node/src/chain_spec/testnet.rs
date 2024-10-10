use crate::chain_spec::{get_account_id_from_seed, get_from_seed, testnet_genesis, ChainSpec};
use argon_node_runtime::{opaque::SessionKeys, WASM_BINARY};
use argon_primitives::{
	bitcoin::BitcoinNetwork,
	block_seal::MiningSlotConfig,
	notary::{GenesisNotary, NotaryPublic},
	AccountId, Chain, ComputeDifficulty, ADDRESS_PREFIX,
};
use codec::Decode;
use sc_service::{ChainType, Properties};
use sp_core::sr25519;
use std::str::FromStr;

pub fn testnet_config() -> Result<ChainSpec, String> {
	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "ARGON".into());
	properties.insert("tokenDecimals".into(), 3.into());
	properties.insert("ss58Format".into(), ADDRESS_PREFIX.into());

	const HASHES_PER_SECOND: u64 = 200;
	const TICK_MILLIS: u64 = 60_000;
	let sudo_account = AccountId::from_str("5EZxgPRoQDYW72ceBKTd8AcSPizNL38cVjBzDGTqbeHUPfRx")?;
	let bitcoin_oracle = get_account_id_from_seed::<sr25519::Public>("Dave");
	let price_oracle = get_account_id_from_seed::<sr25519::Public>("Eve");

	let notary_account = get_account_id_from_seed::<sr25519::Public>("Ferdie");
	let notary_public = get_from_seed::<NotaryPublic>("Ferdie//notary");

	let miner_zero = AccountId::from_str("5GBdjj5vi4W1jDW5iNwAju8tiBvxhMdHeAEPmuP1ZY6ZPQCH");
	let miner_zero_keys = SessionKeys::decode(
		&mut &hex::decode("1e69c7672dfb67dc19abfce302caf4c60cc5cb21f39538f749efdc6a28feaba695d5d04b29524e535278e511ac0ec98e4bb76b08a9a6874c5c481f338d727e60")
			.map_err(|e| format!("Error processing miner zero authority key hex {:?}", e))?[..]
	).map_err(|e| format!("Error decoding miner zero authority keys {:?}", e))?;

	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Wasm not available".to_string())?,
		None,
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
		// Initial BlockSeal authorities
		vec![(miner_zero, miner_zero_keys)],
		// Sudo account
		sudo_account.clone(),
		BitcoinNetwork::Signet,
		// Bitcoin utxo tip operator
		bitcoin_oracle.clone(),
		// Price index operator
		price_oracle.clone(),
		// Pre-funded accounts
		vec![
			// funds for the sudo account
			(sudo_account, 10_000),
			// basic funds so an oracle can submit a price
			(bitcoin_oracle, 1_000),
			// oracle funds
			(price_oracle, 1_000),
		],
		500,
		((TICK_MILLIS / 2) * HASHES_PER_SECOND / 1_000) as ComputeDifficulty,
		TICK_MILLIS,
		vec![GenesisNotary {
			account_id: notary_account,
			public: notary_public,
			hosts: vec!["wss://notary1.testnet.argonprotocol.org".to_string().into()],
			name: "Argon Foundation".into(),
		}],
		60,
		MiningSlotConfig {
			blocks_before_bid_end_for_vrf_close: 200,
			blocks_between_slots: 1440,
			slot_bidding_start_block: 0,
		},
		5000, // 0.00006 avail in testnet faucet, need 1k for fees
	))
	.build())
}
