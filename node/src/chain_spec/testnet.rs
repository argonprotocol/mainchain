use sc_service::{ChainType, Properties};
use sp_core::sr25519;

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

pub fn testnet_config() -> Result<ChainSpec, String> {
	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "ARGON".into());
	properties.insert("tokenDecimals".into(), 3.into());
	properties.insert("ss58Format".into(), ADDRESS_PREFIX.into());

	const HASHES_PER_SECOND: u64 = 200;
	const TICK_MILLIS: u64 = 60_000;
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Wasm not available".to_string())?,
		None,
	)
	.with_name("Argon Testnet")
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
		vec![(
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			authority_keys_from_seed("Alice"),
		)],
		// Sudo account
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		BitcoinNetwork::Testnet,
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
		((TICK_MILLIS / 2) * HASHES_PER_SECOND / 1_000) as ComputeDifficulty,
		TICK_MILLIS,
		vec![GenesisNotary {
			account_id: get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			public: get_from_seed::<NotaryPublic>("Ferdie//notary"),
			hosts: vec!["wss://notary1.testnet.argonprotocol.org".to_string().into()],
			name: "Argon Foundation".into(),
		}],
		60,
		MiningSlotConfig {
			blocks_before_bid_end_for_vrf_close: 200,
			blocks_between_slots: 1440,
			slot_bidding_start_block: 14400,
		},
		5000, // 0.00006 avail in testnet faucet, need 1k for fees
	))
	.build())
}
