use crate::chain_spec::{ChainSpec, GenesisSettings, build_genesis_config};
use argon_primitives::{
	ADDRESS_PREFIX, ARGON_TOKEN_SYMBOL, AccountId, Chain, ComputeDifficulty, TOKEN_DECIMALS,
	bitcoin::BitcoinNetwork,
	block_seal::MiningSlotConfig,
	notary::{GenesisNotary, NotaryPublic},
	tick::Ticker,
};
use argon_runtime::WASM_BINARY;
use core::str::FromStr;
use polkadot_sdk::*;
use sc_network::config::MultiaddrWithPeerId;
use sc_service::{ChainType, Properties};
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{ByteArray, hexdisplay::AsBytesRef};

pub fn mainnet_config() -> Result<ChainSpec, String> {
	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), ARGON_TOKEN_SYMBOL.into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
	properties.insert("ss58Format".into(), ADDRESS_PREFIX.into());

	let sudo_account = AccountId::from_str("5FYxsYvmbTfWWsfGh9oYowAvEYvcGg1MBJG6g7NfmuMwZVN1")?;
	let bitcoin_oracle = AccountId::from_str("5HQz9hkfF27nZ7hA4e1yuFNhmK2bp8Y8S7hSwZwU6hPHaY5Y")?;
	let price_oracle = AccountId::from_str("5EUuhQGn1zDStAcfmJG8r9Sahj5mD7Va4Nxt64oeScjCvcKP")?;
	let token_admin = sudo_account.clone();

	let notary_account = AccountId::from_str("5EqLq7FdVERJVdzM2AjxQ3v2yP4BjU9tPsKGxtcao8Vq92J8")?;
	let notary_public = NotaryPublic::from_str("5HENL2mzQABFLQaf8fWygpmxn1oHDSzssvQzd1BkWsJETstN")
		.map_err(|e| format!("Error parsing notary public {:?}", e))?;

	let founding_grandpas: Vec<GrandpaId> = vec![
		"962abf1be4e94bb80e6488a2af551c529571fdd1d972b5c7e311d7507f0882ec",
		"45a74d33ead0b5ff58607fc60556cf1b291d4c503254ae07f17b3d54f8c5c27f",
		"bd3d984a74e9d74d26e13331047d3837fda8541af985d7c83da43a8744371003",
		"803c5c3c4059380a8603f785a093c227a8a2f4a7437c466f1f7233a6881400e6",
	]
	.into_iter()
	.map(grandpa_public)
	.collect::<Result<Vec<_>, String>>()?;

	let rpc_bootnode_0 = get_boot_addrs(
		"152.42.154.162",
		Some("bootnode0.argon.network"),
		"12D3KooWCHuCSkTdcPRKAEauTVuf581KMPG3k8brsZ98GhUULhzE",
		true,
	)?;
	let rpc_bootnode_1 = get_boot_addrs(
		"138.197.54.164",
		Some("bootnode1.argon.network"),
		"12D3KooWKB1Rz3yYcEjLEWMqe7Ds9jmSE5UXBRQwp7NzhbFo9V3A",
		true,
	)?;
	let bootnode_grandpa_0 = get_boot_addrs(
		"64.23.233.88",
		None,
		"12D3KooWK6hoK8bWbmF9u6Wwy6GS6L8k4ATLwkKXUgEFi73zQHgm",
		false,
	)?;
	let bootnode_grandpa_1 = get_boot_addrs(
		"206.189.93.82",
		None,
		"12D3KooWNTCMP7GxZ9P6PW6k4Wmpok5fy4LBMxLwDadKBh6Q2XTZ",
		false,
	)?;

	const HASHES_PER_SECOND: u64 = 4000;
	const TICK_MILLIS: u64 = 60_000;

	Ok(ChainSpec::builder(WASM_BINARY.ok_or_else(|| "Wasm not available".to_string())?, None)
		.with_name(&Chain::Mainnet.to_string())
		.with_id("argon")
		.with_protocol_id("argon")
		.with_chain_type(ChainType::Live)
		.with_properties(properties)
		.with_boot_nodes(
			[rpc_bootnode_0, rpc_bootnode_1, bootnode_grandpa_0, bootnode_grandpa_1].concat(),
		)
		.with_genesis_config_patch(build_genesis_config(GenesisSettings {
			// You have to have an authority to start the chain
			founding_grandpas: founding_grandpas.into_iter().map(|a| (a, 1)).collect::<Vec<_>>(),
			sudo_key: sudo_account.clone(),
			bitcoin_network: BitcoinNetwork::Bitcoin,
			bitcoin_tip_operator: bitcoin_oracle.clone(),
			price_index_operator: price_oracle.clone(),
			initial_vote_minimum: 1_000,
			endowed_accounts: vec![],
			initial_difficulty: (TICK_MILLIS * HASHES_PER_SECOND / 1_000) as ComputeDifficulty,
			ticker: Ticker::new(TICK_MILLIS, 60),
			initial_notaries: vec![GenesisNotary {
				account_id: notary_account,
				public: notary_public,
				hosts: vec!["wss://notary1.argon.network".to_string().into()],
				name: "Argon Foundation".into(),
			}],
			mining_config: MiningSlotConfig {
				ticks_before_bid_end_for_vrf_close: 200,
				ticks_between_slots: 1_440,
				slot_bidding_start_after_ticks: 14_400 - 1_440, // start at day 9
			},
			minimum_bitcoin_lock_satoshis: 1_000,
			hyperbridge_token_admin: token_admin,
		}))
		.build())
}

fn grandpa_public(hex: &str) -> Result<GrandpaId, String> {
	let bytes =
		hex::decode(hex).map_err(|e| format!("Error decoding testnet grandpa key {:?}", e))?;
	GrandpaId::from_slice(bytes.as_bytes_ref())
		.map_err(|e| format!("Error decoding testnet grandpa key {:?}", e))
}

fn get_boot_addrs(
	ip: &str,
	dns_name: Option<&str>,
	peer_id: &str,
	has_secure: bool,
) -> Result<Vec<MultiaddrWithPeerId>, String> {
	let mut addrs: Vec<MultiaddrWithPeerId> = vec![
		format!("/ip4/{ip}/tcp/30333/p2p/{peer_id}")
			.parse()
			.map_err(|e| format!("Unable to parse multiaddr {e:?}"))?,
		format!("/ip4/{ip}/tcp/30333/ws/p2p/{peer_id}")
			.parse()
			.map_err(|e| format!("Unable to parse multiaddr {e:?}"))?,
	];
	if let Some(dns_name) = dns_name {
		addrs.push(
			format!("/dns/{dns_name}/tcp/30333/p2p/{peer_id}")
				.parse()
				.map_err(|e| format!("Unable to parse multiaddr {e:?}"))?,
		);
		addrs.push(
			format!("/dns/{dns_name}/tcp/30333/ws/p2p/{peer_id}")
				.parse()
				.map_err(|e| format!("Unable to parse multiaddr {e:?}"))?,
		);
		if has_secure {
			addrs.push(
				format!("/dns/{dns_name}/tcp/30333/wss/p2p/{peer_id}")
					.parse()
					.map_err(|e| format!("Unable to parse multiaddr {e:?}"))?,
			);
		}
	}
	Ok(addrs)
}
