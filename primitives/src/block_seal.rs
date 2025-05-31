use crate::{Balance, tick::Tick};
use codec::{Codec, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{CloneNoBound, EqNoBound, Parameter, PartialEqNoBound};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_application_crypto::AppCrypto;
use sp_core::{
	H256, OpaquePeerId,
	crypto::{CryptoTypeId, KeyTypeId},
};
use sp_debug_derive::RuntimeDebug;
use sp_runtime::traits::{Block, OpaqueKeys};

pub const BLOCK_SEAL_KEY_TYPE: KeyTypeId = KeyTypeId(*b"seal");

// sr25519 signatures are non-deterministic, so we use ed25519 for deterministic signatures since
// these are part of the nonce hash
pub mod app {
	use sp_application_crypto::{KeyTypeId, app_crypto, ed25519};

	app_crypto!(ed25519, KeyTypeId(*b"seal"));
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

pub type FrameId = u64;

#[derive(
	PartialEqNoBound,
	EqNoBound,
	CloneNoBound,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Deserialize,
	Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct MiningRegistration<AccountId, Balance, Keys>
where
	AccountId: Parameter + DecodeWithMemTracking,
	Balance: Parameter + MaxEncodedLen + DecodeWithMemTracking,
	Keys: OpaqueKeys + Parameter + DecodeWithMemTracking,
{
	/// The account id the miner will operate as
	pub account_id: AccountId,
	/// The account that bids and argonots come from
	pub external_funding_account: Option<AccountId>,
	/// How much was bid for the mining slot
	#[codec(compact)]
	pub bid: Balance,
	/// The argonots put on hold to run a mining seat
	#[codec(compact)]
	pub argonots: Balance,
	/// The signing keys for the miner
	pub authority_keys: Keys,
	/// Which frame the miner started in
	#[codec(compact)]
	pub starting_frame_id: FrameId,
	/// When the bid was placed
	#[codec(compact)]
	pub bid_at_tick: Tick,
}

impl<A: Parameter, B: Parameter + MaxEncodedLen, K: OpaqueKeys + Parameter>
	MiningRegistration<A, B, K>
{
	pub fn rewards_account(&self) -> A {
		self.external_funding_account.clone().unwrap_or(self.account_id.clone())
	}
}

#[derive(
	Clone,
	Serialize,
	Deserialize,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Eq,
	PartialEq,
	RuntimeDebug,
	Default,
	TypeInfo,
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

#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	Serialize,
	Deserialize,
)]
pub struct PeerId(pub OpaquePeerId);

impl MaxEncodedLen for PeerId {
	fn max_encoded_len() -> usize {
		<[u8; 64]>::max_encoded_len()
	}
}

/// The starting frame id for a miner and the index of the miner in that frame
pub type MinerIndex = (FrameId, u32);

#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct MiningAuthority<AuthorityId, AccountId> {
	pub authority_index: MinerIndex,
	pub authority_id: AuthorityId,
	pub account_id: AccountId,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct MiningBidStats {
	pub bids_count: u32,
	pub bid_amount_min: Balance,
	pub bid_amount_max: Balance,
	pub bid_amount_sum: Balance,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
	RuntimeDebug,
)]
pub struct BlockPayout<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + MaxEncodedLen,
{
	pub account_id: AccountId,
	#[codec(compact)]
	pub ownership: Balance,
	#[codec(compact)]
	pub argons: Balance,
	pub reward_type: BlockRewardType,
	pub block_seal_authority: Option<BlockSealAuthorityId>,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
	RuntimeDebug,
)]
pub enum BlockRewardType {
	Miner,
	Voter,
	ProfitShare,
}
