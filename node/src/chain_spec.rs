use std::{net::Ipv4Addr, time::Duration};

use sc_service::{ChainType, Properties};
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{sr25519, OpaquePeerId, Pair, Public};
use sp_runtime::{
	bounded_vec,
	traits::{IdentifyAccount, Verify},
};

use ulx_node_runtime::{
	opaque::SessionKeys, AccountId, Balance, RuntimeGenesisConfig, Signature, WASM_BINARY,
};
use ulx_primitives::{
	block_seal::{Host, MiningRegistration, PeerId, RewardDestination},
	block_vote::VoteMinimum,
	tick::Ticker,
	BlockSealAuthorityId, BondId, ComputeDifficulty,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

fn session_keys(block_seal_authority: BlockSealAuthorityId, grandpa: GrandpaId) -> SessionKeys {
	SessionKeys { block_seal_authority, grandpa }
}
/// Generate a BlockSeal authority key.
pub fn authority_keys_from_seed(s: &str) -> (BlockSealAuthorityId, GrandpaId) {
	(get_from_seed::<BlockSealAuthorityId>(s), get_from_seed::<GrandpaId>(s))
}
/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub fn development_config() -> Result<ChainSpec, String> {
	let mut properties = Properties::new();
	properties.insert("tokenDecimals".into(), 3.into());

	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Development")
	.with_id("dev")
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
		// Pre-funded accounts
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
		],
		500,
		10_000,
		1_000,
	))
	.build())
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let mut properties = Properties::new();
	properties.insert("tokenDecimals".into(), 3.into());

	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_properties(properties)
	.with_genesis_config(testnet_genesis(
		// Initial BlockSeal authorities
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				authority_keys_from_seed("Alice"),
			),
			// (
			// 	get_account_id_from_seed::<sr25519::Public>("Bob"),
			// 	authority_keys_from_seed("Bob"),
			// ),
			// (
			// 	get_account_id_from_seed::<sr25519::Public>("Dave"),
			// 	authority_keys_from_seed("Dave"),
			// ),
		],
		// Sudo account
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		// Pre-funded accounts
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		],
		500,
		100_000_000,
		60_000,
	))
	.build())
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	initial_authorities: Vec<(AccountId, (BlockSealAuthorityId, GrandpaId))>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	initial_vote_minimum: VoteMinimum,
	initial_difficulty: ComputeDifficulty,
	tick_millis: u64,
) -> serde_json::Value {
	let authority_zero = initial_authorities[0].clone();
	let ticker = Ticker::start(Duration::from_millis(tick_millis));
	let miner_zero = MiningRegistration::<AccountId, BondId, Balance> {
		account_id: authority_zero.0.clone(),
		rpc_hosts: bounded_vec![Host {
			ip: Ipv4Addr::new(127, 0, 0, 1).into(),
			port: 9944,
			is_secure: false
		},],
		bond_id: None,
		reward_destination: RewardDestination::Owner,
		bond_amount: 0u32.into(),
		ownership_tokens: 0u32.into(),
		peer_id: PeerId(OpaquePeerId::new([0u8; 64].to_vec())),
	};

	serde_json::json!({
		"argonBalances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, 10_000)).collect::<Vec<_>>(),
		},
		"ulixeeBalances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, 10_000)).collect::<Vec<_>>(),
		},
		"miningSlot":  {
			"minerZero": Some(miner_zero),
		},
		"sudo":  {
			// Assign network admin rights.
			"key": Some(root_key),
		},
		"ticks":  {
			"tickDurationMillis": tick_millis,
			"genesisUtcTime": ticker.genesis_utc_time,
		},
		"blockSealSpec":  {
			"initialVoteMinimum": initial_vote_minimum,
			"initialComputeDifficulty": initial_difficulty,
		},
		"session":  {
			"keys": initial_authorities
				.iter()
				.map(|(id, (block_seal_id, grandpa_id))| {
					(
						id.clone(),
						id.clone(),
						session_keys(block_seal_id.clone(), grandpa_id.clone()),
					)
				})
				.collect::<Vec<_>>(),
		},
	})
}
