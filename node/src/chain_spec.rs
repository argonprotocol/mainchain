use std::env;
use std::time::Duration;

use sc_service::{ChainType, Properties};
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

use ulx_node_runtime::{
	opaque::SessionKeys, AccountId, Balance, RuntimeGenesisConfig, Signature, WASM_BINARY,
};
use ulx_primitives::{
	block_seal::{MiningRegistration, RewardDestination},
	block_vote::VoteMinimum,
	notary::{GenesisNotary, NotaryPublic},
	tick::{Ticker, TICK_MILLIS},
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

	const HASHES_PER_SECOND: u64 = 100_000;

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
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		],
		500,
		(TICK_MILLIS * HASHES_PER_SECOND / 1_000) as ComputeDifficulty,
		TICK_MILLIS,
		vec![], // No notaries
	))
	.build())
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let mut properties = Properties::new();
	properties.insert("tokenDecimals".into(), 3.into());

	let notary_host =
		env::var("ULX_LOCAL_TESTNET_NOTARY_URL").unwrap_or("ws://127.0.0.1:9925".to_string()).into();

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
		(TICK_MILLIS * 1_000_000 / 1_000) as ComputeDifficulty,
		TICK_MILLIS,
		vec![GenesisNotary {
			account_id: get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			public: get_from_seed::<NotaryPublic>("Ferdie//notary"),
			hosts: vec![notary_host],
		}],
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
	initial_notaries: Vec<GenesisNotary<AccountId>>,
) -> serde_json::Value {
	let authority_zero = initial_authorities[0].clone();
	let ticker = Ticker::start(Duration::from_millis(tick_millis));
	let miner_zero = MiningRegistration::<AccountId, BondId, Balance> {
		account_id: authority_zero.0.clone(),
		bond_id: None,
		reward_destination: RewardDestination::Owner,
		bond_amount: 0u32.into(),
		ownership_tokens: 0u32.into(),
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
		"notaries": {
			"list": initial_notaries,
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
