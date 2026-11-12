use frame_support::{CloneNoBound, DebugNoBound, EqNoBound, PartialEqNoBound};
use pallet_prelude::*;

use super::Config;

/// Complete replacement configuration, shared by a standalone account or linked account group.
#[derive(
	CloneNoBound,
	DebugNoBound,
	EqNoBound,
	PartialEqNoBound,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct Policy<T: Config> {
	/// Strictly sorted, unique accounts approving exits and sensitive authority changes.
	pub signers: BoundedVec<T::AccountId, T::MaxSigners>,
	/// Required distinct signatures, from one through the number of signers inclusive.
	pub quorum: u16,
	/// Offline backup, separate from the protected group and the approval signers.
	pub recovery_account: T::AccountId,
	/// Waiting period in ticks for recovery, backup replacement or changes to this delay.
	/// Zero explicitly disables the cancellation window. Changes wait the current delay.
	pub recovery_delay: Tick,
}

/// One accepted replacement policy awaiting its cancellation window, never an arbitrary call.
#[derive(
	CloneNoBound,
	DebugNoBound,
	EqNoBound,
	PartialEqNoBound,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct PendingPolicyUpdate<T: Config> {
	/// Complete policy accepted by the main account and the relevant current authority.
	pub policy: Policy<T>,
	/// Consumed group authorization nonce identifying this particular proposal.
	pub nonce: u64,
	/// Hash of the policy under which this change was accepted, including its delay.
	pub previous_policy_hash: [u8; 32],
	/// Inclusive tick at which anyone may complete the change if it has not been cancelled.
	pub execute_after_tick: Tick,
	/// Additional native-token reserve for this bounded pending record, in base units.
	pub deposit: u128,
}

/// Durable configuration and its native-token storage reserve, owned by the default account.
#[derive(
	CloneNoBound,
	DebugNoBound,
	EqNoBound,
	PartialEqNoBound,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct Registration<T: Config> {
	/// Approved configuration shared by the default account and its linked members.
	pub policy: Policy<T>,
	/// Actual reserved deposit in native-token base units; not part of the signing policy.
	pub deposit: u128,
}

/// Immutable replay bounds carried in every signed approval payload.
#[derive(
	CloneNoBound,
	DebugNoBound,
	EqNoBound,
	PartialEqNoBound,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct Authorization<T: Config> {
	/// Expected default account; linking cannot redirect a signature to another group.
	pub group: T::AccountId,
	/// Exact next group authorization nonce, independent of native transaction nonces.
	pub nonce: u64,
	/// Inclusive last block at which these signatures may authorize the operation.
	pub expires_at: BlockNumberFor<T>,
	/// Blake2-256 of SCALE Option<Policy>; None is used only for initial enrollment.
	pub policy_hash: [u8; 32],
}

/// An off-chain signature over the canonical full security call with an empty approvals vector.
#[derive(
	CloneNoBound,
	DebugNoBound,
	EqNoBound,
	PartialEqNoBound,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct Approval<T: Config> {
	/// Policy signer identity authenticated by this proof; device identities cannot submit
	/// transactions.
	pub signer: T::AccountId,
	/// Account or P256 proof over the canonical security call in the selected metadata-hash mode.
	pub signature: T::Signature,
	/// Include the runtime's actual metadata hash in native signing (e.g. Ledger clear signing).
	/// The app cannot substitute a different hash; unsupported runtimes reject this mode.
	pub with_metadata_hash: bool,
}

/// Effective identity for one subtree; wrapper-derived origins cannot forge the first factor.
#[derive(CloneNoBound)]
pub struct AuthorizationContext<T: Config> {
	/// Effective signed origin after native proxy, utility or multisig transformations.
	pub account: T::AccountId,
	/// Group whose quorum the caller must verify before dispatch. Inspection only evaluates
	/// requirements; constructing this context does not establish any signing authority.
	pub authorized_group: Option<T::AccountId>,
	/// False for a pending multisig: inspect potential effects, but do not require execution
	/// approval.
	pub executes: bool,
}

/// Runtime rules for native calls; AccountSecurity's management APIs enforce their own authority.
#[derive(Debug, PartialEq, Eq)]
pub enum CallRequirement<AccountId> {
	RequireApproval(AccountId),
	/// Linking cannot discard existing signing policies or recovery authorities.
	RequireAccountLinkApproval([AccountId; 3]),
	/// Internal movement still participates in the mixed-authority batch guard.
	CheckProtectionForBatch(AccountId),
	ChangesAuthorization,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ApprovalStatus {
	Unprotected,
	Approved,
	Missing,
}

#[derive(Default)]
pub(super) struct AuthorizationInspection {
	pub(super) missing_approval: bool,
	pub(super) visited_calls: u32,
	pub(super) leaf_calls: u32,
	pub(super) changes_authority: bool,
	pub(super) touches_protection: bool,
}
