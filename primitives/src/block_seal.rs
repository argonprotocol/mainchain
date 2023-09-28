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

pub const SEAL_NONCE_PREFIX: [u8; 14] = *b"ulx_block_seal";

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SealNonceHashMessage<Hash> {
	pub prefix: [u8; 14],
	pub tax_proof_id: u32,
	pub tax_amount: u128,
	pub parent_hash: Hash,
	pub seal_stampers: Vec<SealStamper>,
}

pub const SEALER_SIGNATURE_PREFIX: [u8; 14] = *b"ulx_sealer_sig";

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SealerSignatureMessage<Hash, AuthorityId> {
	pub prefix: [u8; 14],
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
	pub trait AuthorityApis {
		fn authorities() -> Vec<BlockSealAuthorityId>;
		fn authorities_by_index() -> BTreeMap<u16, BlockSealAuthorityId>;
		fn active_authorities() -> u16;
		fn xor_closest_validators(hash: Vec<u8>) -> Vec<AuthorityDistance<BlockSealAuthorityId>>;
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ValidatorRegistration<AccountId, BondId, Balance: MaxEncodedLen> {
	pub account_id: AccountId,
	pub peer_id: PeerId,
	#[codec(compact)]
	pub rpc_ip: u32,
	#[codec(compact)]
	pub rpc_port: u16,
	pub reward_destination: RewardDestination<AccountId>,
	pub bond_id: Option<BondId>,
	#[codec(compact)]
	pub bond_amount: Balance,
	#[codec(compact)]
	pub ownership_tokens: Balance,
}

/// A destination account for validator rewards
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum RewardDestination<AccountId> {
	Owner,
	/// Pay into a specified account.
	Account(AccountId),
}

impl<AccountId> Default for RewardDestination<AccountId> {
	fn default() -> Self {
		RewardDestination::Owner
	}
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
	fn is_active(authority_id: &AuthorityId) -> bool;
	fn get_authority(author: AccountId) -> Option<AuthorityId>;
	fn find_xor_closest_authorities(hash: U256, closest: u8)
		-> Vec<AuthorityDistance<AuthorityId>>;
}

pub trait HistoricalBlockSealersLookup<BlockNumber, AuthorityId> {
	/// Returns block seal validators for the given block number that are still active.
	fn get_active_block_sealers_of(block_number: BlockNumber) -> Vec<AuthorityId>;
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct AuthorityDistance<AuthorityId> {
	#[codec(compact)]
	pub authority_index: u16,
	pub authority_id: AuthorityId,
	pub peer_id: PeerId,
	pub distance: U256,
	#[codec(compact)]
	pub rpc_ip: u32,
	#[codec(compact)]
	pub rpc_port: u16,
}
