#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use sp_core::{
	crypto::{AccountId32, KeyTypeId},
	ConstU32, MaxEncodedLen, OpaquePeerId, U256,
};
use sp_runtime::{BoundedVec, RuntimeDebug};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

pub use digests::{Difficulty, ProofOfWorkType, UlxPreDigest, UlxSeal, AUTHOR_ID, ULX_ENGINE_ID};

pub mod digests;
pub mod inherents;

pub const BLOCK_SEAL_KEY_TYPE: KeyTypeId = KeyTypeId(*b"ulx_");

// sr25519 signatures are non deterministic, so we use ed25519 for deterministic signatures since
// these are part of the nonce hash
pub mod app {
	use sp_application_crypto::{app_crypto, ed25519};

	app_crypto!(ed25519, sp_core::crypto::KeyTypeId(*b"ulx_"));
}

sp_application_crypto::with_pair! {
	pub type BlockSealAuthorityPair = app::Pair;
}
pub type BlockSealAuthoritySignature = app::Signature;
pub type BlockSealAuthorityId = app::Public;

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct BlockProof {
	pub tax_proof_id: u32,
	pub tax_amount: u128,
	pub seal_stampers: Vec<SealStamper>,
	pub author_id: AccountId32,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SealNonceHashMessage<Hash> {
	pub tax_proof_id: u32,
	pub tax_amount: u128,
	pub parent_hash: Hash,
	pub seal_stampers: Vec<SealStamper>,
}
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SealerSignatureMessage<Hash, AuthorityId> {
	pub tax_proof_id: u32,
	pub tax_amount: u128,
	pub parent_hash: Hash,
	pub author_id: AccountId32,
	pub seal_stampers: Vec<AuthorityId>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct SealStamper {
	pub authority_idx: u16,
	pub signature: Option<BoundedVec<u8, ConstU32<64>>>,
}

sp_api::decl_runtime_apis! {
	/// This runtime api allows people to query the current authority set
	pub trait UlxConsensusApi {
		fn next_work() -> NextWork;
		fn calculate_easing(tax_amount: u128, validators: u8) -> Difficulty;
	}

	pub trait AuthorityApis {
		fn authorities() -> Vec<BlockSealAuthorityId>;
		fn authorities_by_index() -> BTreeMap<u16, BlockSealAuthorityId>;
		fn active_authorities() -> u16;
		fn xor_closest_validators(hash: Vec<u8>) -> Vec<AuthorityDistance<BlockSealAuthorityId>>;
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct NextWork {
	pub work_type: ProofOfWorkType,
	pub difficulty: Difficulty,
	pub closest_x_authorities_required: u32,
	pub min_seal_signers: u32,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ValidatorRegistration<AccountId> {
	pub account_id: AccountId,
	pub peer_id: PeerId,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct PeerId(pub OpaquePeerId);

impl MaxEncodedLen for PeerId {
	fn max_encoded_len() -> usize {
		<[u8; 64]>::max_encoded_len()
	}
}

pub trait AuthorityProvider<AuthorityId, AccountId> {
	fn authorities() -> Vec<AuthorityId>;
	fn authorities_by_index() -> BTreeMap<u16, AuthorityId>;
	fn authority_count() -> u16;
	fn get_authority(author: AccountId) -> Option<AuthorityId>;
	fn find_xor_closest_authorities(hash: U256, closest: u8)
		-> Vec<AuthorityDistance<AuthorityId>>;
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct AuthorityDistance<AuthorityId> {
	pub authority_index: u16,
	pub authority_id: AuthorityId,
	pub peer_id: PeerId,
	pub distance: U256,
}
