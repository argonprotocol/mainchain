use argon_primitives::{
	bitcoin::{BitcoinNetwork, Satoshis},
	block_seal::MiningSlotConfig,
	block_vote::VoteMinimum,
	notary::GenesisNotary,
	tick::Ticker,
	AccountId, Balance, BlockSealAuthorityId, ComputeDifficulty, Signature,
};
use argon_runtime::{
	BalancesConfig, BitcoinUtxosConfig, BlockSealSpecConfig, BondsConfig, ChainTransferConfig,
	GrandpaConfig, MiningSlotConfig as MiningSlotPalletConfig, NotariesConfig, PriceIndexConfig,
	RuntimeGenesisConfig, SessionKeys, SudoConfig, TicksConfig,
};
use sp_consensus_grandpa::{AuthorityId as GrandpaId, AuthorityWeight};
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

mod development;
mod local_testnet;
mod mainnet;
mod testnet;

pub use development::development_config;
pub use local_testnet::local_testnet_config;
pub use mainnet::mainnet_config;
pub use testnet::testnet_config;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec;

/// Generate a crypto pair from seed.
pub(crate) fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate a BlockSeal authority key.
pub(crate) fn authority_keys_from_seed(s: &str) -> SessionKeys {
	let (block_seal_authority, grandpa) =
		(get_from_seed::<BlockSealAuthorityId>(s), get_from_seed::<GrandpaId>(s));
	SessionKeys { block_seal_authority, grandpa }
}
/// Generate an account ID from seed.
pub(crate) fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub struct GenesisSettings {
	pub founding_grandpas: Vec<(GrandpaId, AuthorityWeight)>,
	pub sudo_key: AccountId,
	pub bitcoin_network: BitcoinNetwork,
	pub bitcoin_tip_operator: AccountId,
	pub price_index_operator: AccountId,
	pub endowed_accounts: Vec<(AccountId, Balance)>,
	pub initial_vote_minimum: VoteMinimum,
	pub initial_difficulty: ComputeDifficulty,
	pub ticker: Ticker,
	pub initial_notaries: Vec<GenesisNotary<AccountId>>,
	pub mining_config: MiningSlotConfig,
	pub minimum_bitcoin_bond_satoshis: Satoshis,
	pub hyperbridge_token_admin: AccountId,
}

#[allow(clippy::too_many_arguments)]
/// Configure initial storage state for FRAME modules.
pub(crate) fn testnet_genesis(
	GenesisSettings {
		founding_grandpas,
		sudo_key,
		bitcoin_network,
		bitcoin_tip_operator,
		price_index_operator,
		endowed_accounts,
		initial_vote_minimum,
		initial_difficulty,
		ticker,
		initial_notaries,
		mining_config,
		minimum_bitcoin_bond_satoshis,
		hyperbridge_token_admin,
	}: GenesisSettings,
) -> serde_json::Value {
	let config = RuntimeGenesisConfig {
		balances: BalancesConfig { balances: endowed_accounts },
		bonds: BondsConfig { minimum_bitcoin_bond_satoshis, ..Default::default() },
		price_index: PriceIndexConfig { operator: Some(price_index_operator) },
		bitcoin_utxos: BitcoinUtxosConfig {
			tip_oracle_operator: Some(bitcoin_tip_operator),
			network: bitcoin_network,
		},
		mining_slot: MiningSlotPalletConfig { mining_config, ..Default::default() },
		sudo: SudoConfig { key: Some(sudo_key) },
		ticks: TicksConfig { ticker, ..Default::default() },
		block_seal_spec: BlockSealSpecConfig {
			initial_vote_minimum,
			initial_compute_difficulty: initial_difficulty,
			..Default::default()
		},
		notaries: NotariesConfig { list: initial_notaries },
		grandpa: GrandpaConfig { authorities: founding_grandpas, ..Default::default() },
		chain_transfer: ChainTransferConfig {
			hyperbridge_token_admin: Some(hyperbridge_token_admin),
			..Default::default()
		},
		..Default::default()
	};

	serde_json::to_value(config).expect("Could not build genesis config.")
}
