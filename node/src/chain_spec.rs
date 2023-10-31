use std::net::Ipv4Addr;

use sc_service::{ChainType, Properties};
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{bounded_vec, sr25519, OpaquePeerId, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

use ulx_node_runtime::{
	opaque::SessionKeys, AccountId, ArgonBalancesConfig, BlockSealConfig, DifficultyConfig,
	GrandpaConfig, MiningSlotsConfig, RuntimeGenesisConfig, SessionConfig, Signature, SudoConfig,
	SystemConfig, UlixeeBalancesConfig, WASM_BINARY,
};
use ulx_primitives::{
	block_seal::{Host, MiningRegistration, PeerId, RewardDestination},
	BlockSealAuthorityId,
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
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenDecimals".into(), 3.into());

	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
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
				10_000,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		// Properties
		Some(properties),
		// Extensions
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenDecimals".into(), 3.into());

	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
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
				100_000_000,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		Some(properties),
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AccountId, (BlockSealAuthorityId, GrandpaId))>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	initial_difficulty: u128,
) -> RuntimeGenesisConfig {
	let authority_zero = initial_authorities[0].clone();

	RuntimeGenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		argon_balances: ArgonBalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, 10_000)).collect(),
		},
		ulixee_balances: UlixeeBalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, 10_000)).collect(),
		},
		mining_slots: MiningSlotsConfig {
			miner_zero: Some(MiningRegistration {
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
			}),
			..Default::default()
		},
		grandpa: GrandpaConfig { authorities: vec![], ..Default::default() },
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|(id, (block_seal_id, grandpa_id))| {
					(
						id.clone(),
						id.clone(),
						session_keys(block_seal_id.clone(), grandpa_id.clone()),
					)
				})
				.collect(),
		},
		block_seal: BlockSealConfig {
			min_seal_signers: 1,
			closest_xor_authorities_required: 5,
			authority_count_starting_tax_seal: 2,
			..Default::default()
		},
		difficulty: DifficultyConfig { initial_difficulty, ..Default::default() },
		transaction_payment: Default::default(),
		tx_pause: Default::default(),
	}
}
