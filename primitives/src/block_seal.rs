use crate::{tick::Tick, Balance, ObligationId};
use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::{CloneNoBound, EqNoBound, Parameter, PartialEqNoBound};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_application_crypto::AppCrypto;
use sp_arithmetic::FixedU128;
use sp_core::{
	crypto::{CryptoTypeId, KeyTypeId},
	OpaquePeerId, H256,
};
use sp_debug_derive::RuntimeDebug;
use sp_runtime::traits::{Block, OpaqueKeys};

pub const BLOCK_SEAL_KEY_TYPE: KeyTypeId = KeyTypeId(*b"seal");

// sr25519 signatures are non-deterministic, so we use ed25519 for deterministic signatures since
// these are part of the nonce hash
pub mod app {
	use sp_application_crypto::{app_crypto, ed25519};

	app_crypto!(ed25519, sp_core::crypto::KeyTypeId(*b"seal"));
}

sp_application_crypto::with_pair! {
	pub type BlockSealAuthorityPair = app::Pair;
}
pub type BlockSealAuthoritySignature = app::Signature;
pub type BlockSealAuthorityId = app::Public;
pub const BLOCK_SEAL_CRYPTO_ID: CryptoTypeId = <app::Public as AppCrypto>::CRYPTO_ID;

pub type ComputeDifficulty = u128;

#[derive(
	PartialEqNoBound,
	EqNoBound,
	CloneNoBound,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Deserialize,
	Serialize,
)]
pub struct ComputePuzzle<B: Block> {
	pub difficulty: ComputeDifficulty,
	/// The block hash to load the randomx vm from. None means use genesis (which isn't available
	/// in block 0... so we have this odd setup)
	pub randomx_key_block: Option<B::Hash>,
}

impl<B: Block> ComputePuzzle<B> {
	pub fn get_key_block(&self, genesis_hash: B::Hash) -> H256 {
		H256::from_slice(self.randomx_key_block.unwrap_or(genesis_hash).as_ref())
	}
}

pub type CohortId = u64;

#[derive(
	PartialEqNoBound, EqNoBound, CloneNoBound, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxHosts))]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MiningRegistration<
	AccountId: Parameter,
	Balance: Parameter + MaxEncodedLen,
	Keys: OpaqueKeys + Parameter,
> {
	pub account_id: AccountId,
	pub reward_destination: RewardDestination<AccountId>,
	pub obligation_id: Option<ObligationId>,
	#[codec(compact)]
	pub bonded_argons: Balance,
	#[codec(compact)]
	pub argonots: Balance,
	pub reward_sharing: Option<RewardSharing<AccountId>>,
	pub authority_keys: Keys,
	#[codec(compact)]
	pub cohort_id: CohortId,
}

#[derive(
	Clone, Serialize, Deserialize, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo,
)]
#[serde(rename_all = "camelCase")]
pub struct MiningSlotConfig {
	/// How many blocks before the end of a slot can the bid close
	#[codec(compact)]
	pub ticks_before_bid_end_for_vrf_close: Tick,
	/// How many ticks transpire between slots
	#[codec(compact)]
	pub ticks_between_slots: Tick,
	/// The tick when bidding will start (eg, Slot "1")
	#[codec(compact)]
	pub slot_bidding_start_after_ticks: Tick,
}

/// An struct to define a reward sharing split with another account
#[derive(
	PartialEq,
	Eq,
	Copy,
	Clone,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Deserialize,
	Serialize,
)]
pub struct RewardSharing<AccountId> {
	pub account_id: AccountId,
	#[codec(compact)]
	pub percent_take: FixedU128,
}

/// A destination account for validator rewards
#[derive(
	PartialEq,
	Eq,
	Copy,
	Clone,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Deserialize,
	Serialize,
	Default,
)]
pub enum RewardDestination<AccountId> {
	#[default]
	Owner,
	/// Pay into a specified account.
	Account(AccountId),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
pub struct PeerId(pub OpaquePeerId);

impl MaxEncodedLen for PeerId {
	fn max_encoded_len() -> usize {
		<[u8; 64]>::max_encoded_len()
	}
}

pub type MinerIndex = u32;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct MiningAuthority<AuthorityId, AccountId> {
	#[codec(compact)]
	pub authority_index: MinerIndex,
	pub authority_id: AuthorityId,
	pub account_id: AccountId,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Encode, Decode, TypeInfo)]
pub struct MiningBidStats {
	pub bids_count: u32,
	pub bid_amount_min: Balance,
	pub bid_amount_max: Balance,
	pub bid_amount_sum: Balance,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
pub struct BlockPayout<AccountId: Codec, Balance: Codec + MaxEncodedLen> {
	pub account_id: AccountId,
	#[codec(compact)]
	pub ownership: Balance,
	#[codec(compact)]
	pub argons: Balance,
	pub reward_type: BlockRewardType,
	pub block_seal_authority: Option<BlockSealAuthorityId>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
pub enum BlockRewardType {
	Miner,
	Voter,
	ProfitShare,
}
