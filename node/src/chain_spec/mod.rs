use std::time::Duration;

use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

use argon_node_runtime::{opaque::SessionKeys, AccountId, Balance, Signature};
use argon_primitives::{
	bitcoin::BitcoinNetwork,
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

pub(crate) fn session_keys(
	block_seal_authority: BlockSealAuthorityId,
	grandpa: GrandpaId,
) -> SessionKeys {
	SessionKeys { block_seal_authority, grandpa }
}
/// Generate a BlockSeal authority key.
pub(crate) fn authority_keys_from_seed(s: &str) -> (BlockSealAuthorityId, GrandpaId) {
	(get_from_seed::<BlockSealAuthorityId>(s), get_from_seed::<GrandpaId>(s))
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
	initial_authorities: Vec<(AccountId, (BlockSealAuthorityId, GrandpaId))>,
	root_key: AccountId,
	bitcoin_network: BitcoinNetwork,
	bitcoin_tip_operator: AccountId,
	price_index_operator: AccountId,
	endowed_accounts: Vec<AccountId>,
	initial_vote_minimum: VoteMinimum,
	initial_difficulty: ComputeDifficulty,
	tick_millis: u64,
	initial_notaries: Vec<GenesisNotary<AccountId>>,
	escrow_expiration_ticks: Tick,
	mining_config: MiningSlotConfig<BlockNumber>,
) -> serde_json::Value {
	let authority_zero = initial_authorities[0].clone();
	let ticker = Ticker::start(Duration::from_millis(tick_millis), escrow_expiration_ticks);
	let miner_zero = MiningRegistration::<AccountId, Balance> {
		account_id: authority_zero.0.clone(),
		bond_id: None,
		reward_destination: RewardDestination::Owner,
		bond_amount: 0u32.into(),
		ownership_tokens: 0u32.into(),
		reward_sharing: None,
	};

	serde_json::json!({
		"argonBalances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, 100_000_000)).collect::<Vec<_>>(),
		},
		"shareBalances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, 10_000)).collect::<Vec<_>>(),
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
