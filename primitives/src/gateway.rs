use crate::Moment;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
/// Source chains supported by the crosschain gateway.
pub enum SourceChain {
	/// Gateway activity stream backed by Ethereum execution state.
	Ethereum,
}

/// Monotonic position of one proven gateway activity for a source chain.
pub type GatewayActivityNonce = u64;
/// Highest council approval queue nonce the gateway has already absorbed into source-chain
/// state.
pub type ArgonApprovalsNonce = u64;

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Default,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
/// Latest gateway snapshot that Argon has proven and accepted for one source chain.
pub struct GatewayState<Balance> {
	/// Timestamp of the latest source-chain block whose gateway activity is fully synced,
	/// in Unix milliseconds.
	pub last_synced_source_block_time: Option<Moment>,
	/// Most recent gateway activity Argon has applied for this chain.
	#[codec(compact)]
	pub gateway_activity_nonce: GatewayActivityNonce,
	/// Most recent council approval queue item the gateway has already incorporated.
	#[codec(compact)]
	pub argon_approvals_nonce: ArgonApprovalsNonce,
	/// Gateway-reported amount of Argon that should still be backed by funds parked on Argon.
	pub argon_circulation: Balance,
	/// Gateway-reported amount of Argonot that should still be backed by funds parked on
	/// Argon.
	pub argonot_circulation: Balance,
}
