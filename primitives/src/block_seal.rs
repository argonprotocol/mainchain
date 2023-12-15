use codec::{Decode, Encode};
use frame_support::{CloneNoBound, EqNoBound, Parameter, PartialEqNoBound};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_application_crypto::ByteArray;
use sp_core::{crypto::KeyTypeId, ConstU32, MaxEncodedLen, OpaquePeerId, U256};
use sp_runtime::{BoundedVec, RuntimeDebug};

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

impl BlockSealAuthorityId {
	pub fn to_u256(&self) -> U256 {
		let mut bytes = [0u8; 32];
		bytes.copy_from_slice(&self.as_slice());
		U256::from_big_endian(&bytes)
	}
}

pub type MaxMinerRpcHosts = ConstU32<4>;

#[derive(
	PartialEqNoBound, EqNoBound, CloneNoBound, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxHosts))]
#[derive(Deserialize, Serialize)]
pub struct MiningRegistration<
	AccountId: Parameter,
	BondId: Parameter,
	Balance: Parameter + MaxEncodedLen,
> {
	pub account_id: AccountId,
	pub peer_id: PeerId,
	pub rpc_hosts: BoundedVec<Host, MaxMinerRpcHosts>,
	pub reward_destination: RewardDestination<AccountId>,
	pub bond_id: Option<BondId>,
	#[codec(compact)]
	pub bond_amount: Balance,
	#[codec(compact)]
	pub ownership_tokens: Balance,
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
)]
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

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
pub struct PeerId(pub OpaquePeerId);

impl MaxEncodedLen for PeerId {
	fn max_encoded_len() -> usize {
		<[u8; 64]>::max_encoded_len()
	}
}

pub type AuthorityIndex = u16;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct MiningAuthority<AuthorityId, AccountId> {
	#[codec(compact)]
	pub authority_index: AuthorityIndex,
	pub authority_id: AuthorityId,
	pub account_id: AccountId,
	pub peer_id: PeerId,
	pub rpc_hosts: BoundedVec<Host, MaxMinerRpcHosts>,
}

#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Deserialize,
	Serialize,
)]
pub struct Host {
	#[codec(compact)]
	/// ip encoded as u32 big endian (eg, from octets)
	pub ip: u32,
	#[codec(compact)]
	pub port: u16,
	pub is_secure: bool,
}

impl Host {
	#[cfg(feature = "std")]
	pub fn get_url(&self) -> String {
		format!(
			"{}://{}:{}",
			if self.is_secure { "wss" } else { "ws" },
			std::net::Ipv4Addr::from(self.ip),
			self.port
		)
	}
}
