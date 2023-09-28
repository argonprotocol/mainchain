use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::{CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use sp_core::{ed25519, RuntimeDebug, H256};
use sp_std::fmt::Debug;

pub type NotaryId = u32;

pub type NotaryPublic = ed25519::Public;
pub type NotarySignature = ed25519::Signature;

pub trait NotaryProvider {
	fn verify_signature(notary_id: NotaryId, message: &H256, signature: &NotarySignature) -> bool;
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct NotaryMeta {
	pub public: NotaryPublic,
	/// Notary ip encoded as u32 big endian (eg, from octets)
	#[codec(compact)]
	pub ip: u32,
	#[codec(compact)]
	pub port: u16,
}

#[derive(
	CloneNoBound, PartialEqNoBound, Eq, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
)]
pub struct NotaryRecord<
	AccountId: Codec + MaxEncodedLen + Clone + PartialEq + Eq + Debug,
	BlockNumber: Codec + MaxEncodedLen + Clone + PartialEq + Eq + Debug,
> {
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub operator_account_id: AccountId,
	#[codec(compact)]
	pub activated_block: BlockNumber,

	#[codec(compact)]
	pub meta_updated_block: BlockNumber,
	pub meta: NotaryMeta,
}
