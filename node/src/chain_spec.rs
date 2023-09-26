use sc_service::{ChainType, Properties};
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::AccountId32, sr25519, ByteArray, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

use ulx_node_runtime::{
	opaque::SessionKeys, AccountId, ArgonBalancesConfig, BlockSealConfig, DifficultyConfig,
	GrandpaConfig, RuntimeGenesisConfig, SessionConfig, Signature, SudoConfig, SystemConfig,
	UlixeeBalancesConfig, WASM_BINARY,
};
use ulx_primitives::BlockSealAuthorityId;

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

pub fn get_genesis_dummy_validator() -> (AccountId, BlockSealAuthorityId, GrandpaId) {
	let account_id: AccountId = AccountId32::from([1u8; 32]).into();
	let validator_id = BlockSealAuthorityId::from_slice(&[1; 32]).unwrap();
	let grandpa_id = GrandpaId::from_slice(&[1; 32]).unwrap();
	(account_id, validator_id, grandpa_id)
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
				// Initial PoA authorities
				vec![],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
				true,
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
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						authority_keys_from_seed("Bob"),
					),
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
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				],
				true,
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
	mut initial_authorities: Vec<(AccountId, (BlockSealAuthorityId, GrandpaId))>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> RuntimeGenesisConfig {
	let (dummy_validator_account, block_seal_id, grandpa_id) = get_genesis_dummy_validator();
	initial_authorities.push((dummy_validator_account, (block_seal_id, grandpa_id)));

	RuntimeGenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		argon_balances: ArgonBalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 10)).collect(),
		},
		ulixee_balances: UlixeeBalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 10)).collect(),
		},
		grandpa: GrandpaConfig { authorities: vec![], ..Default::default() },
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.0.clone(), session_keys(x.1 .0.clone(), x.1 .1.clone())))
				.collect(),
		},
		block_seal: BlockSealConfig {
			min_seal_signers: 8,
			closest_xor_authorities_required: 10,
			..Default::default()
		},
		difficulty: DifficultyConfig { initial_difficulty: 10_000, ..Default::default() },
		transaction_payment: Default::default(),
	}
}
