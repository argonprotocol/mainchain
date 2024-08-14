use codec::{Codec, Decode, Encode, HasCompact, MaxEncodedLen};
use frame_support::{CloneNoBound, EqNoBound, Parameter, PartialEqNoBound};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_application_crypto::AppCrypto;
use sp_arithmetic::FixedU128;
use sp_core::{
	crypto::{CryptoTypeId, KeyTypeId},
	OpaquePeerId,
};
use sp_debug_derive::RuntimeDebug;

use crate::BondId;

pub const BLOCK_SEAL_KEY_TYPE: KeyTypeId = KeyTypeId(*b"seal");

// sr25519 signatures are non deterministic, so we use ed25519 for deterministic signatures since
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

#[derive(
	PartialEqNoBound, EqNoBound, CloneNoBound, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxHosts))]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MiningRegistration<AccountId: Parameter, Balance: Parameter + MaxEncodedLen> {
	pub account_id: AccountId,
	pub reward_destination: RewardDestination<AccountId>,
	pub bond_id: Option<BondId>,
	#[codec(compact)]
	pub bond_amount: Balance,
	#[codec(compact)]
	pub ownership_tokens: Balance,
	pub reward_sharing: Option<RewardSharing<AccountId>>,
}

#[derive(
	Clone, Serialize, Deserialize, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo,
)]
#[serde(rename_all = "camelCase")]
pub struct MiningSlotConfig<BlockNumber: Codec + HasCompact> {
	/// How many blocks before the end of a slot can the bid close
	#[codec(compact)]
	pub blocks_before_bid_end_for_vrf_close: BlockNumber,
	/// How many blocks transpire between slots
	#[codec(compact)]
	pub blocks_between_slots: BlockNumber,
	/// The block number when bidding will start (eg, Slot "1")
	#[codec(compact)]
	pub slot_bidding_start_block: BlockNumber,
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
pub struct BlockPayout<AccountId: Codec, Balance: Codec + MaxEncodedLen> {
	pub account_id: AccountId,
	#[codec(compact)]
	pub shares: Balance,
	#[codec(compact)]
	pub argons: Balance,
}
