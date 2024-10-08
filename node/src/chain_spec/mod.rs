use std::time::Duration;

use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

use argon_node_runtime::{opaque::SessionKeys, AccountId, Balance, Signature};
use argon_primitives::{
	bitcoin::{BitcoinNetwork, Satoshis},
	block_seal::{MiningRegistration, MiningSlotConfig, RewardDestination},
	block_vote::VoteMinimum,
	notary::GenesisNotary,
	tick::{Tick, Ticker},
	BlockNumber, BlockSealAuthorityId, ComputeDifficulty,
};

mod development;
mod local_testnet;
mod testnet;

pub use development::development_config;
pub use local_testnet::local_testnet_config;
pub use testnet::testnet_config;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
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

#[allow(clippy::too_many_arguments)]
/// Configure initial storage state for FRAME modules.
pub(crate) fn testnet_genesis(
	initial_authorities: Vec<(AccountId, SessionKeys)>,
	root_key: AccountId,
	bitcoin_network: BitcoinNetwork,
	bitcoin_tip_operator: AccountId,
	price_index_operator: AccountId,
	endowed_accounts: Vec<(AccountId, Balance)>,
	initial_vote_minimum: VoteMinimum,
	initial_difficulty: ComputeDifficulty,
	tick_millis: u64,
	initial_notaries: Vec<GenesisNotary<AccountId>>,
	channel_hold_expiration_ticks: Tick,
	mining_config: MiningSlotConfig<BlockNumber>,
	minimum_bitcoin_bond_satoshis: Satoshis,
) -> serde_json::Value {
	let authority_zero = initial_authorities[0].clone();
	let ticker = Ticker::start(Duration::from_millis(tick_millis), channel_hold_expiration_ticks);
	let miner_zero = MiningRegistration::<AccountId, Balance, SessionKeys> {
		account_id: authority_zero.0.clone(),
		bond_id: None,
		reward_destination: RewardDestination::Owner,
		bond_amount: 0u32.into(),
		ownership_tokens: 0u32.into(),
		reward_sharing: None,
		authority_keys: authority_zero.1,
	};

	serde_json::json!({
		"balances": {
			"balances": endowed_accounts,
		},
		"bonds": {
			"minimumBitcoinBondSatoshis": minimum_bitcoin_bond_satoshis
		},
		"priceIndex": {
			"operator": Some(price_index_operator),
		},
		"bitcoinUtxos": {
			"tipOracleOperator": Some(bitcoin_tip_operator),
			"network": bitcoin_network,
		},
		"miningSlot":  {
			"minerZero": Some(miner_zero),
			"miningConfig": mining_config,
		},
		"sudo":  {
			// Assign network admin rights.
			"key": Some(root_key),
		},
		"ticks":  {
			"ticker": ticker,
		},
		"blockSealSpec":  {
			"initialVoteMinimum": initial_vote_minimum,
			"initialComputeDifficulty": initial_difficulty,
		},
		"notaries": {
			"list": initial_notaries,
		},
		"grandpa":  {
			"authorities": initial_authorities.iter().map(|(_, keys)| (keys.grandpa.clone(), 1)).collect::<Vec<_>>(),
		},
	})
}
