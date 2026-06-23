#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use argon_primitives::{EthereumAccountStorageProof, EthereumLog};
pub use pallet::*;
pub use weights::{prove_gateway_activity_with_providers, WeightInfo, WithProviderWeights};

use pallet_prelude::*;
use polkadot_sdk::sp_runtime::{traits::ConstU32, BoundedVec};

pub(crate) const GATEWAY_ACTIVITY_STORAGE_SLOT_COUNT: u32 = 2;
type GatewayActivityStorageSlots = ConstU32<GATEWAY_ACTIVITY_STORAGE_SLOT_COUNT>;

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
	DebugNoBound,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxActivityLogs))]
pub struct GatewayActivityProof<MaxActivityLogs: Get<u32>> {
	#[codec(compact)]
	pub locator_index: u64,
	pub storage_proof: EthereumAccountStorageProof<GatewayActivityStorageSlots>,
	pub activity_logs: BoundedVec<EthereumLog, MaxActivityLogs>,
}

mod approval_queue;
mod evm;
mod gateway_activity;
mod minting_authority;
mod transfer_out;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod hyperbridge_migration;
mod weights;
pub use transfer_out::{
	MintingAuthorityTransferReservation, PendingCollateralizationRequest, TransferOutOfArgon,
	TransferOutState,
};

#[frame_support::pallet]
pub mod pallet {
	use super::{gateway_activity::GatewayActivityApplyError, *};
	use alloc::vec::Vec;
	use argon_primitives::{
		block_seal::FrameId,
		ethereum::{EthereumBlockNumber, MAX_ETHEREUM_HEADER_CHAIN_LEN},
		vault::BitcoinVaultProvider,
		CallTxPoolKeyProvider, CallTxValidityProvider, CollectBlockerProvider,
		EthereumVerifyProvider, OperationalAccountsHook, PriceProvider, TickProvider,
		TreasuryPoolProvider, UniswapTransferProvider, MICROGONS_PER_ARGON,
	};
	use frame_support::{
		dispatch::Pays,
		storage::with_storage_layer,
		traits::fungible::{InspectHold, Mutate, MutateHold},
	};
	use polkadot_sdk::{
		frame_support::traits::IsSubType,
		frame_system::{ensure_root, ensure_signed},
		sp_core::ecdsa::KeccakSignature,
		sp_crypto_hashing::blake2_256,
		sp_runtime::{transaction_validity::InvalidTransaction, BoundedBTreeMap},
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	pub(super) const WEI_PER_ETH: u128 = 1_000_000_000_000_000_000;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config:
		polkadot_sdk::frame_system::Config<
		AccountId = argon_primitives::AccountId,
		RuntimeEvent: From<Event<Self>>,
	>
	{
		/// Balance type used for inbound payouts and recent-transfer tracking.
		type Balance: AtLeast32BitUnsigned
			+ Member
			+ codec::FullCodec
			+ codec::HasCompact
			+ Copy
			+ MaybeSerializeDeserialize
			+ DecodeWithMemTracking
			+ core::fmt::Debug
			+ Default
			+ From<u128>
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// Canonical Ethereum burn-accounting account representing funds moved to Ethereum.
		#[pallet::constant]
		type EthereumBurnAccount: Get<Self::AccountId>;

		/// Native Argon currency implementation
		type NativeCurrency: Mutate<Self::AccountId, Balance = Self::Balance>
			+ MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ InspectHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>;

		/// Ownership-token currency implementation
		type OwnershipCurrency: Mutate<Self::AccountId, Balance = Self::Balance>
			+ MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ InspectHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Inspect<Self::AccountId, Balance = Self::Balance>;

		/// Runtime hold reason used for transfer-out minting authority tips.
		type RuntimeHoldReason: From<HoldReason>;

		/// Ethereum proof verifier for receipt and header-chain validation.
		type EthereumVerifier: EthereumVerifyProvider;

		/// Existing operational-accounts hook for qualifying inbound Argon transfers.
		type OperationalAccountsHook: OperationalAccountsHook<Self::AccountId, Self::Balance>;

		/// Shared vault provider used to resolve local Minting Authority ownership.
		type VaultProvider: BitcoinVaultProvider<
			AccountId = Self::AccountId,
			Balance = Self::Balance,
		>;

		/// Treasury bond provider used to resolve live committed Argon bond collateral for Minting
		/// Authorities.
		type TreasuryPoolProvider: TreasuryPoolProvider<Self::AccountId, Balance = Self::Balance>;

		/// Shared price provider used to snapshot the council-managed Argonot floor.
		type PriceProvider: argon_primitives::PriceProvider<Self::Balance>;

		/// Runtime frame provider used for collect-due alignment on queued council work.
		type CurrentFrameId: Get<FrameId>;

		/// Runtime tick provider used for recent-transfer retention checks.
		type CurrentTick: Get<Tick>;

		/// Runtime tick provider used to convert verified Ethereum header timestamps into local
		/// tick age.
		type TickProvider: TickProvider<Self::Block>;

		/// Retention window, in ticks, for recent Argon transfer evidence used by operational
		/// accounts.
		#[pallet::constant]
		type RecentTransferRetentionTicks: Get<Tick>;

		/// Maximum number of ordered gateway activities that may share one proven gateway block.
		#[pallet::constant]
		type MaxActivitiesPerGatewayProof: Get<u32>;

		/// Maximum number of members the active Global Issuance Council may carry.
		#[pallet::constant]
		type MaxCouncilMembers: Get<u32>;

		/// Maximum number of contiguous queue approvals one council member may submit in one call.
		#[pallet::constant]
		type MaxQueueApprovalsPerCall: Get<u32>;

		/// Ethereum-block window added to the latest verified block when opening a
		/// transfer out.
		#[pallet::constant]
		type TransferOutValidityEthereumBlocks: Get<EthereumBlockNumber>;

		/// Maximum age, in ticks, of the verified Ethereum execution anchor used to open a
		/// transfer out.
		#[pallet::constant]
		type MaxVerifiedExecutionBlockAgeTicks: Get<Tick>;

		/// Minting authority tip rate applied to transfer-out requests in basis points.
		#[pallet::constant]
		type TransferOutMintingAuthorityTipBasisPoints: Get<u32>;

		/// Minimum normalized collateral increment accepted for one transfer unless the increment
		/// completes the remaining uncovered amount.
		#[pallet::constant]
		type MinTransferCollateralIncrement: Get<Self::Balance>;

		/// Default minimum normalized microgon value required to register a Minting Authority on
		/// one destination chain.
		#[pallet::constant]
		type DefaultMinimumMintingAuthorityMicrogonValue: Get<Self::Balance>;

		/// Maximum number of non-terminal transfer-out requests tracked for one destination chain.
		#[pallet::constant]
		type MaxPendingTransferOutsPerDestinationChain: Get<u32>;

		/// Minimum remaining frame commitment required when reading vault committed collateral
		/// capacity for council weighting and Minting Authority collateral.
		#[pallet::constant]
		type CouncilRotationFrames: Get<FrameId>;

		/// Weight implementation for pallet calls and hooks.
		type WeightInfo: WeightInfo;
	}

	#[pallet::extra_constants]
	impl<T: Config> Pallet<T> {
		/// Maximum execution headers carried in one receipt proof's target-to-anchor chain.
		pub fn max_proof_execution_header_depth() -> u32 {
			MAX_ETHEREUM_HEADER_CHAIN_LEN
		}
	}

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
	/// Concrete source chains supported by this pallet.
	pub enum SourceChain {
		/// Gateway activity stream backed by Ethereum execution state.
		Ethereum,
	}

	/// Monotonic position of one proven gateway activity for a source chain.
	pub type GatewayActivityNonce = u64;
	/// Highest council approval queue nonce the gateway has already absorbed into source-chain
	/// state.
	pub type ArgonApprovalsNonce = u64;
	/// Monotonic position of one council approval item in the Argon-side queue.
	pub type CouncilApprovalQueueNonce = u64;
	/// Per-sender nonce/counter of a TransferOutOfArgon used to keep otherwise identical
	/// transfer-out requests distinct.
	pub type TransferOutRequestNonce = u64;

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
	)]
	/// Source-chain config accepted by this deployment.
	pub enum ChainConfig {
		/// Configuration shared by EVM-family gateway deployments such as Ethereum and Base.
		Evm {
			/// Chain id used when verifying gateway approvals and transfer signatures.
			#[codec(compact)]
			chain_id: u64,
			/// Gateway contract address for this source chain.
			gateway: H160,
			/// Argon token contract address on this source chain.
			argon_token: H160,
			/// Argonot token contract address on this source chain.
			argonot_token: H160,
		},
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Copy,
		Clone,
		PartialEq,
		Eq,
		Debug,
		TypeInfo,
		MaxEncodedLen,
	)]
	/// Asset kinds this pallet can move across the gateway.
	pub enum AssetKind {
		/// The Argon balance tracked in microgons.
		Argon,
		/// The Argonot balance tracked in micronots.
		Argonot,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		DebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	/// Latest gateway snapshot that Argon has proven and accepted for one source chain.
	pub struct GatewayState<T: Config> {
		/// Most recent gateway activity Argon has applied for this chain.
		#[codec(compact)]
		pub gateway_activity_nonce: GatewayActivityNonce,
		/// Most recent council approval queue item the gateway has already incorporated.
		#[codec(compact)]
		pub argon_approvals_nonce: ArgonApprovalsNonce,
		/// Gateway-reported amount of Argon that should still be backed by funds parked on Argon.
		pub argon_circulation: T::Balance,
		/// Gateway-reported amount of Argonot that should still be backed by funds parked on
		/// Argon.
		pub argonot_circulation: T::Balance,
	}

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, DebugNoBound, TypeInfo,
	)]
	#[scale_info(skip_type_params(T))]
	/// One inbound transfer observed on a source-chain gateway and settled to an Argon account.
	pub struct TransferToArgonActivity<T: Config> {
		/// Position of this transfer in the gateway activity stream for the source chain.
		#[codec(compact)]
		pub gateway_activity_nonce: GatewayActivityNonce,
		/// Source-chain account that sent the transfer.
		pub from: H160,
		/// Asset the recipient receives on Argon.
		pub asset: AssetKind,
		/// Recipient account on Argon.
		pub to: T::AccountId,
		/// Amount transferred for this activity.
		#[codec(compact)]
		pub amount: T::Balance,
	}

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
	/// Reason Argon stopped applying further gateway activities for one source chain.
	pub enum GatewaySyncPauseReason {
		/// Paused explicitly by governance or root action.
		Manual,
		/// A recognized gateway event could not be decoded into the expected payload.
		MalformedGatewayActivity,
		/// A transfer referenced a token that is not configured for this source chain.
		UnsupportedToken,
		/// A proven authority activity referenced a signer Argon does not know locally.
		MintingAuthorityNotFound,
		/// A proven authority activity did not match the authority's current lifecycle stage.
		UnexpectedMintingAuthorityState,
		/// A proven authority activity disagreed with the local authority record.
		MintingAuthorityMismatch,
		/// The relayer repayment pricing needed to absorb a proven activation is missing or
		/// invalid.
		MissingMintingAuthorityActivationRepaymentPricing,
		/// A proven activation could not reconcile the local reimbursement hold it expected to
		/// settle.
		MintingAuthorityActivationRepaymentMismatch,
		/// A proven gateway activity referenced a council snapshot Argon no longer retained.
		GlobalIssuanceCouncilNotFound,
		/// The gateway's reported circulation or collateral no longer matched Argon's expectation.
		GatewayStateDrift,
	}

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
	/// Recorded stop point for a paused gateway so operators can see what landed safely and what
	/// still needs investigation.
	pub struct GatewaySyncPause {
		/// Last gateway activity nonce Argon is confident was applied correctly.
		#[codec(compact)]
		pub last_good_gateway_activity_nonce: GatewayActivityNonce,
		/// First gateway activity nonce Argon refused to advance past.
		#[codec(compact)]
		pub failed_gateway_activity_nonce: GatewayActivityNonce,
		/// Operator-facing explanation for why forward sync is blocked.
		pub reason: GatewaySyncPauseReason,
	}

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
	/// Lifecycle state of one locally tracked Minting Authority.
	pub enum MintingAuthorityState {
		/// Registered locally and approved by council, but not yet confirmed by proven gateway
		/// activity.
		PendingActivation,
		/// Confirmed active and able to reserve collateral for outbound transfers.
		Active,
		/// Marked for retirement and waiting for proven gateway deactivation.
		Deactivating,
	}

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
	#[pallet::composite_enum]
	/// Balance holds created by this pallet.
	pub enum HoldReason {
		/// Tip held from a transfer-out request until finalization or cancel resolves it.
		TransferOutMintingAuthorityTip,
		/// Estimated relayer reimbursement held until a Minting Authority activation is proven.
		MintingAuthorityActivationRepayment,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		DebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	/// One signer-weighted member record in the active Global Issuance Council snapshot.
	pub struct GlobalIssuanceCouncilMember<T: Config> {
		/// Argon account that owns the vault weight behind this seat.
		pub account_id: T::AccountId,
		/// Signer this seat uses when approving gateway updates on the destination chain.
		pub signer: H160,
		/// Voting power this member contributes to quorum on this council snapshot.
		pub weight: T::Balance,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		DebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	/// Frozen council snapshot used to verify queue approvals and outbound transfer collateral
	/// math.
	pub struct GlobalIssuanceCouncil<T: Config> {
		/// Price floor used to translate committed Argonots into council voting weight.
		pub epoch_microgons_per_argonot: T::Balance,
		/// Signer-keyed council membership at the moment this snapshot became canonical.
		pub members: BoundedBTreeMap<H160, GlobalIssuanceCouncilMember<T>, T::MaxCouncilMembers>,
		/// Total voting power available for quorum decisions against this snapshot.
		pub total_weight: T::Balance,
	}

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
	/// Object being approved by the council queue.
	pub enum CouncilApprovalTargetId {
		/// Activation of the minting authority identified by this destination-chain signing key.
		MintingAuthorityActivation(H160),
		/// Deactivation of the minting authority identified by this destination-chain signing key.
		MintingAuthorityDeactivation(H160),
		/// Rotation to the council snapshot identified by this hash.
		GlobalIssuanceCouncilRotation(H256),
	}

	#[derive(
		Encode, Decode, DecodeWithMemTracking, CloneNoBound, DebugNoBound, TypeInfo, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	/// One ordered council work item that must gather signatures before the destination-chain
	/// gateway can accept it.
	pub struct CouncilApprovalQueueEntry<T: Config> {
		/// Council snapshot whose weights and signers govern this approval item.
		pub approving_council_hash: H256,
		/// Runtime object the council is authorizing.
		pub target: CouncilApprovalTargetId,
		/// Hash of the concrete payload Ethereum will later verify for this target.
		pub target_payload_hash: H256,
		/// First frame where missing signatures should start blocking vault collect.
		#[codec(compact)]
		pub due_frame_id: FrameId,
		/// Previous approval hash in the queue chain so Ethereum can enforce ordering.
		pub previous_approval_hash: H256,
		/// Hash council members actually sign for this queue item.
		pub approval_hash: H256,
		/// Weight already accumulated from recorded signatures.
		pub approved_total_weight: T::Balance,
		/// Signatures already attached to this queue item, keyed by council signer.
		pub signatures: BoundedBTreeMap<H160, KeccakSignature, T::MaxCouncilMembers>,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		DebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	/// Argon-side view of one Minting Authority, including gateway-proven collateral and local
	/// tentative outbound reservations.
	pub struct MintingAuthority<T: Config> {
		/// Vault operator that owns this authority's backing on Argon.
		pub account_id: T::AccountId,
		/// Destination chain where this authority signs and underwrites issuance.
		pub destination_chain: SourceChain,
		/// Signer identity this authority uses on the destination chain.
		pub destination_signing_key: H160,
		/// Where this authority sits in the activation / active / retirement flow.
		pub state: MintingAuthorityState,
		/// Microgon collateral still available according to the last proven gateway state.
		pub gateway_remaining_microgon_collateral: T::Balance,
		/// Micronot collateral still available according to the last proven gateway state.
		pub gateway_remaining_micronot_collateral: T::Balance,
		/// Microgon collateral currently tentatively reserved for outbound transfers not yet
		/// proven back.
		pub pending_reserved_microgon_collateral: T::Balance,
		/// Micronot collateral currently tentatively reserved for outbound transfers not yet
		/// proven back.
		pub pending_reserved_micronot_collateral: T::Balance,
		/// Live outbound transfers this authority is tentatively backing, oldest first so newer
		/// reservations can be invalidated from the tail when the gateway settles differently than
		/// Argon expected.
		pub active_pending_transfer_ids:
			BoundedVec<H256, <T as Config>::MaxPendingTransferOutsPerDestinationChain>,
		/// Queue item that must be incorporated by the gateway before this authority can complete
		/// activation.
		pub activation_approval_queue_nonce: CouncilApprovalQueueNonce,
		/// Flat activation reimbursement quote snapshotted when this authority entered the queue.
		#[codec(compact)]
		pub activation_base_repayment_quote: T::Balance,
		/// Total signature-side activation reimbursement quote snapshotted when this authority
		/// entered the queue.
		#[codec(compact)]
		pub activation_signature_repayment_quote: T::Balance,
		/// Queue item carrying the council-approved Ethereum deactivation while the local state is
		/// already in `Deactivating`.
		pub deactivation_approval_queue_nonce: Option<CouncilApprovalQueueNonce>,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		DebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	/// Per-destination-chain pricing inputs used to repay relayers who batch Minting Authority
	/// activations onto Ethereum.
	pub struct MintingAuthorityActivationRepaymentPricing<T: Config> {
		/// Reviewed EVM gas units attributed to one activated authority in the batch payload.
		#[codec(compact)]
		pub activation_gas_cost: u128,
		/// Reviewed EVM gas units attributed to one supplied council signature in the activation
		/// tranche's shared signed updates.
		#[codec(compact)]
		pub signature_gas_cost: u128,
		/// Estimated wei paid per unit of Ethereum gas.
		#[codec(compact)]
		pub estimated_wei_per_gas: u128,
		/// Estimated Argon-denominated value of one ETH, expressed in microgons.
		pub estimated_microgons_per_eth: T::Balance,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		DebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	/// Argon and Argonot circulation components tracked for one source chain.
	pub struct SourceChainCirculation<T: Config> {
		/// Argon-denominated circulation component.
		pub argon_circulation: T::Balance,
		/// Argonot-denominated circulation component.
		pub argonot_circulation: T::Balance,
	}

	impl<T: Config> Default for SourceChainCirculation<T> {
		fn default() -> Self {
			Self {
				argon_circulation: T::Balance::default(),
				argonot_circulation: T::Balance::default(),
			}
		}
	}

	#[pallet::storage]
	/// Config accepted for each supported source chain.
	pub type ChainConfigBySourceChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, ChainConfig, OptionQuery>;

	#[pallet::storage]
	/// Latest proven gateway activity snapshot for each source chain.
	pub type GatewayStateBySourceChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, GatewayState<T>, OptionQuery>;

	#[pallet::storage]
	/// Latest accepted gateway locator hash for each source chain.
	pub type LastAcceptedLocatorHashBySourceChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, H256, ValueQuery>;

	#[pallet::storage]
	/// Pause state recorded when a canonical gateway activity cannot be applied safely.
	pub type GatewaySyncPauseBySourceChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, GatewaySyncPause, OptionQuery>;

	#[pallet::storage]
	/// Count of still-retained qualifying Argon transfers for each local account.
	pub type RecentArgonTransfersByAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	#[pallet::storage]
	/// Latest council approval queue nonce signed by this account for one destination chain.
	pub type CouncilApprovalCursorByDestinationChainAndAccountId<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		SourceChain,
		Blake2_128Concat,
		T::AccountId,
		CouncilApprovalQueueNonce,
		OptionQuery,
	>;

	#[pallet::storage]
	/// Accounts whose recent-transfer evidence expires at a given tick.
	#[pallet::unbounded]
	pub type InboundTransfersExpiringAt<T: Config> =
		StorageMap<_, Twox64Concat, Tick, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	/// Latest tick whose recent-transfer expiration bucket was cleaned up.
	pub type LastTransferExpiryCleanupTick<T: Config> = StorageValue<_, Tick, ValueQuery>;

	#[pallet::storage]
	/// The registered council signer for each account on each destination chain.
	pub type CouncilSignerByDestinationChainAndAccountId<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		SourceChain,
		Blake2_128Concat,
		T::AccountId,
		H160,
		OptionQuery,
	>;

	#[pallet::storage]
	/// A staged council signer rotation for the next council activation on each destination chain.
	pub type PendingCouncilSignerByDestinationChainAndAccountId<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		SourceChain,
		Blake2_128Concat,
		T::AccountId,
		H160,
		OptionQuery,
	>;

	#[pallet::storage]
	/// The active Global Issuance Council hash in force for each destination chain.
	pub type ActiveGlobalIssuanceCouncilByDestinationChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, H256, OptionQuery>;

	#[pallet::storage]
	/// Conservative transfer-out conversion rate currently quoted for each destination chain.
	pub type CurrentTransferOutMicrogonsPerArgonotByDestinationChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, T::Balance, OptionQuery>;

	#[pallet::storage]
	/// Immediately previous transfer-out conversion rate still accepted while the floor ratchets
	/// upward across council transitions.
	pub type PreviousTransferOutMicrogonsPerArgonotByDestinationChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, T::Balance, OptionQuery>;

	#[pallet::storage]
	/// Historical Global Issuance Council snapshots keyed by their signer-ordered council hash.
	pub type GlobalIssuanceCouncilByHash<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, GlobalIssuanceCouncil<T>, OptionQuery>;

	#[pallet::storage]
	/// The latest queued council hash that should govern the next queue entry on each destination
	/// chain.
	pub type LatestQueuedCouncilHashByDestinationChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, H256, OptionQuery>;

	#[pallet::storage]
	/// The conservative transfer-out quote floor that the next newly opened transfer should
	/// snapshot on each destination chain.
	pub type TransferOutQuoteMicrogonsPerArgonotByDestinationChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, T::Balance, OptionQuery>;

	#[pallet::storage]
	/// The next outbound approval-queue nonce to assign on each destination chain.
	pub type NextCouncilApprovalQueueNonceByDestinationChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, CouncilApprovalQueueNonce, ValueQuery>;

	#[pallet::storage]
	/// The ordered outbound council approval queue for each destination chain.
	pub type CouncilApprovalQueueByDestinationChainAndNonce<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		SourceChain,
		Twox64Concat,
		CouncilApprovalQueueNonce,
		CouncilApprovalQueueEntry<T>,
		OptionQuery,
	>;

	#[pallet::storage]
	/// Local Minting Authority records keyed by their destination signing key.
	pub type MintingAuthoritiesBySigner<T: Config> =
		StorageMap<_, Blake2_128Concat, H160, MintingAuthority<T>, OptionQuery>;

	#[pallet::type_value]
	pub fn DefaultMinimumMintingAuthorityValue<T: Config>() -> T::Balance {
		T::DefaultMinimumMintingAuthorityMicrogonValue::get()
	}

	#[pallet::storage]
	/// Per-chain override for the minimum normalized microgon value required to register a
	/// Minting Authority.
	pub type MinimumMintingAuthorityValueByDestinationChain<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		SourceChain,
		T::Balance,
		ValueQuery,
		DefaultMinimumMintingAuthorityValue<T>,
	>;

	#[pallet::storage]
	/// Pricing inputs used to repay relayers for Minting Authority activation batches on each
	/// destination chain.
	pub type MintingAuthorityActivationRepaymentPricingByDestinationChain<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		SourceChain,
		MintingAuthorityActivationRepaymentPricing<T>,
		OptionQuery,
	>;

	#[pallet::storage]
	/// Next outbound transfer nonce for each sending account.
	pub type NextTransferOutNonceBySendingAccountId<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, TransferOutRequestNonce, ValueQuery>;

	#[pallet::storage]
	/// Outbound transfers by transfer id.
	pub type TransferOutById<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, TransferOutOfArgon<T>, OptionQuery>;

	#[pallet::storage]
	/// Collateralization requests still open on each destination chain.
	pub type PendingCollateralizationRequestsByChain<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		SourceChain,
		BoundedVec<
			PendingCollateralizationRequest<T>,
			<T as Config>::MaxPendingTransferOutsPerDestinationChain,
		>,
		ValueQuery,
	>;

	#[pallet::storage]
	/// Number of non-terminal transfer-out requests on each destination chain.
	pub type NonTerminalTransferOutCountByDestinationChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, u32, ValueQuery>;

	#[pallet::storage]
	/// Pending outbound principal by destination chain.
	pub type PendingTransferOutCirculationByDestinationChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, SourceChainCirculation<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A `TransferToArgonStarted` activity was proven and settled locally.
		TransferToArgonSettled { source_chain: SourceChain, transfer: TransferToArgonActivity<T> },
		/// Root force-set the active Global Issuance Council for a destination chain.
		GlobalIssuanceCouncilForced { destination_chain: SourceChain, council_hash: H256 },
		/// An account registered its council signer for one destination chain.
		CouncilSignerRegistered {
			destination_chain: SourceChain,
			account_id: T::AccountId,
			signer: H160,
		},
		/// An account queued a replacement council signer for the next council update.
		CouncilSignerRotationQueued {
			destination_chain: SourceChain,
			account_id: T::AccountId,
			signer: H160,
		},
		/// An operator account registered a Minting Authority and queued it for council approval.
		MintingAuthorityRegistered {
			destination_chain: SourceChain,
			destination_signing_key: H160,
			account_id: T::AccountId,
			approval_queue_nonce: CouncilApprovalQueueNonce,
		},
		/// An operator queued the council-approved Ethereum deactivation for a Minting Authority.
		MintingAuthorityDeactivationQueued {
			destination_chain: SourceChain,
			destination_signing_key: H160,
			approval_queue_nonce: CouncilApprovalQueueNonce,
		},
		/// Root updated the minimum normalized microgon value required to register a Minting
		/// Authority on one destination chain.
		MinimumMintingAuthorityValueSet {
			destination_chain: SourceChain,
			minimum_value: T::Balance,
		},
		/// Root updated the pricing inputs used to repay relayers for activation batches on one
		/// destination chain.
		MintingAuthorityActivationRepaymentPricingSet { destination_chain: SourceChain },
		/// A council member recorded approval for a queued council update entry.
		QueueEntryApprovalRecorded {
			destination_chain: SourceChain,
			target: CouncilApprovalTargetId,
			approval_queue_nonce: CouncilApprovalQueueNonce,
		},
		/// The queued council update entry reached local quorum.
		QueueEntryApprovalReady {
			destination_chain: SourceChain,
			target: CouncilApprovalTargetId,
			approval_queue_nonce: CouncilApprovalQueueNonce,
		},
		/// A proven Ethereum activation filled the pending local activation fields.
		MintingAuthorityActivationFinalized {
			source_chain: SourceChain,
			destination_signing_key: H160,
		},
		/// A proven activation paid or released the held relayer reimbursement and made the
		/// authority usable.
		MintingAuthorityActivationCompleted {
			destination_chain: SourceChain,
			destination_signing_key: H160,
			relayer_argon_account_id: T::AccountId,
			repayment_amount: T::Balance,
		},
		/// A proven Ethereum deactivation released collateral and removed the local authority
		/// record.
		MintingAuthorityDeactivationFinalized {
			source_chain: SourceChain,
			destination_signing_key: H160,
		},
		/// Gateway proof application paused one source chain at a specific canonical activity.
		GatewaySyncPaused { source_chain: SourceChain, pause: GatewaySyncPause },
		/// Root manually unpaused one source chain.
		GatewayUnpaused { source_chain: SourceChain },
		/// Root explicitly set the continuity hash needed to validate the next gateway locator.
		LastAcceptedLocatorHashSet { source_chain: SourceChain, locator_hash: H256 },
		/// The stored gateway-state snapshot advanced after a proven contiguous batch.
		GatewayStateAdvanced { source_chain: SourceChain, gateway_state: GatewayState<T> },
		/// A transfer out was opened.
		TransferOutStarted {
			destination_chain: SourceChain,
			transfer_id: H256,
			account_id: T::AccountId,
			asset: AssetKind,
			amount: T::Balance,
			minting_authority_tip: T::Balance,
		},
		/// A minting authority updated transfer collateral.
		TransferCollateralized {
			transfer_id: H256,
			destination_signing_key: H160,
			microgon_collateral: T::Balance,
			micronot_collateral: T::Balance,
		},
		/// A transfer is ready for finalization.
		TransferOutReady { transfer_id: H256 },
		/// A transfer was finalized on the source chain.
		TransferOutFinalized { source_chain: SourceChain, transfer_id: H256 },
		/// A transfer was canceled on the source chain.
		TransferOutCanceled { source_chain: SourceChain, transfer_id: H256 },
		/// A pending collateral reservation was invalidated.
		TransferCollateralInvalidated { transfer_id: H256, destination_signing_key: H160 },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The Ethereum event topics or payload do not match `TransferToArgonStarted`.
		InvalidTransferToArgonActivity,
		/// At least one gateway activity log must be supplied with the proof.
		NoGatewayActivitiesProvided,
		/// The Ethereum verifier rejected the supplied proof.
		InvalidProof,
		/// The next gateway proof needs a previously accepted locator hash that has not been
		/// bootstrapped yet.
		MissingLastAcceptedLocatorHash,
		/// The continuity hash may only be bootstrapped once for a source chain.
		LastAcceptedLocatorHashAlreadySet,
		/// The source chain is not configured for inbound claims.
		UnsupportedSource,
		/// The gateway does not match the configured gateway address.
		UnsupportedGateway,
		/// The token is not supported under the configured gateway.
		UnsupportedToken,
		/// The caller's expected already-proven gateway activity nonce is stale or incorrect.
		UnexpectedPreviousGatewayActivityNonce,
		/// The proven gateway activity nonce is not the next contiguous nonce.
		UnexpectedGatewayActivityNonce,
		/// The configured source-chain shape is incomplete or malformed.
		InvalidChainConfig,
		/// The burn account lacks enough balance for the payout.
		InsufficientLiquidity,
		/// The supplied Minting Authority signing key does not exist locally.
		MintingAuthorityNotFound,
		/// The local Minting Authority did not match the proven Ethereum activity.
		MintingAuthorityMismatch,
		/// The local Minting Authority was not in the expected lifecycle state.
		UnexpectedMintingAuthorityState,
		/// The local owner vault could not be resolved.
		UnknownOwnerVault,
		/// The supplied Minting Authority signing key is invalid.
		InvalidMintingAuthoritySigningKey,
		/// The supplied Minting Authority signing key already has a live local authority record.
		MintingAuthorityAlreadyRegistered,
		/// The provided Minting Authority signer proof did not match the recovered Ethereum
		/// signer.
		InvalidMintingAuthoritySigningKeyProof,
		/// The supplied Minting Authority collateral is invalid.
		InvalidMintingAuthorityCollateral,
		/// The active Global Issuance Council has not been seeded for this destination chain.
		GlobalIssuanceCouncilNotFound,
		/// The supplied Global Issuance Council is empty or internally inconsistent.
		InvalidGlobalIssuanceCouncil,
		/// The supplied Global Issuance Council contains the same signer more than once.
		DuplicateGlobalIssuanceCouncilSigner,
		/// The supplied Global Issuance Council contains the same account more than once.
		DuplicateGlobalIssuanceCouncilAccount,
		/// The provided council signer proof did not match the recovered Ethereum signer.
		InvalidCouncilSignerProof,
		/// The origin is not an active Global Issuance Council member for the destination chain.
		GlobalIssuanceCouncilMemberNotFound,
		/// The account has not registered a council signer for this destination chain.
		CouncilSignerNotRegistered,
		/// The Minting Authority approval queue entry does not exist.
		CouncilApprovalQueueEntryNotFound,
		/// This council member already approved the queued Minting Authority.
		CouncilApprovalAlreadyRecorded,
		/// The force-set cut point was behind already confirmed queue progress.
		InvalidForceSetAfterNonce,
		/// The force-set cut would discard a queue entry that already has local quorum.
		CannotForceSetQuorumApprovedQueueEntry,
		/// The provided council approval signature did not match the expected Ethereum signer.
		InvalidCouncilApprovalSignature,
		/// At least one council approval signature must be supplied.
		NoCouncilApprovalSignaturesProvided,
		/// The operator does not have enough remaining committed microgon collateral for this
		/// Minting Authority.
		InsufficientCommittedMicrogonCollateral,
		/// The operator does not have enough remaining committed Argonot collateral for this
		/// Minting Authority.
		InsufficientCommittedArgonotCollateral,
		/// Reserved legacy error for invalid signer-keyed deactivation signatures.
		InvalidMintingAuthorityDeactivationSignature,
		/// The council-managed Argonot floor could not be determined from current pricing.
		InvalidMicrogonsPerArgonot,
		/// The supplied Minting Authority collateral is below the configured per-chain minimum
		/// normalized microgon value.
		MintingAuthorityCollateralBelowMinimum,
		/// The configured activation repayment pricing is internally invalid.
		InvalidMintingAuthorityActivationRepaymentPricing,
		/// The configured activation repayment pricing is missing for this source chain.
		MissingMintingAuthorityActivationRepaymentPricing,
		/// The Ethereum event topics or payload do not match a supported gateway activity.
		InvalidGatewayActivity,
		/// This source chain is paused due to gateway sync misalignment and needs operator
		/// recovery.
		GatewaySyncPaused,
		/// The transfer-out amount must be nonzero.
		InvalidTransferOutAmount,
		/// The transfer-out recipient must be nonzero for the destination chain.
		InvalidTransferOutRecipient,
		/// No verifier-backed Ethereum execution block is available to anchor a transfer-out
		/// expiry window.
		MissingVerifiedExecutionBlock,
		/// The latest verifier-backed Ethereum execution block is too old to safely open a new
		/// transfer out.
		StaleVerifiedExecutionBlock,
		/// The outbound transfer record does not exist.
		TransferOutNotFound,
		/// The outbound transfer cannot accept more collateral because it is already fully
		/// covered.
		TransferOutAlreadyReady,
		/// The outbound transfer request is already expired on the latest verified Ethereum block.
		TransferOutExpired,
		/// The provided transfer collateral row is invalid for this transfer asset.
		InvalidTransferCollateral,
		/// The provided transfer collateral row did not advance the signer's local reservation.
		InvalidTransferCollateralUpdate,
		/// The transfer collateral signature did not match the authority signing key.
		InvalidTransferCollateralSignature,
		/// The authority does not have enough remaining gateway collateral for this transfer row.
		InsufficientMintingAuthorityCollateral,
		/// The collateral increment is below the configured minimum and does not complete the
		/// transfer.
		TransferCollateralIncrementTooSmall,
		/// The destination chain already tracks the maximum number of non-terminal transfer-out
		/// requests.
		TooManyPendingTransferOuts,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let mut weight = Weight::zero();
			if hyperbridge_migration::needs_initialization::<T>() {
				weight = weight
					.saturating_add(hyperbridge_migration::initialize_crosschain_transfer::<T>());
			}

			let current_tick = T::CurrentTick::get();
			let last_cleanup_tick = LastTransferExpiryCleanupTick::<T>::get();
			let first_tick_to_cleanup = if last_cleanup_tick == 0 {
				current_tick
			} else {
				last_cleanup_tick.saturating_add(1)
			};
			let mut expiring_len = 0u32;

			for tick in first_tick_to_cleanup..=current_tick {
				let expiring = InboundTransfersExpiringAt::<T>::take(tick);
				expiring_len = expiring_len.saturating_add(expiring.len() as u32);

				for account_id in expiring {
					Self::decrement_recent_argon_transfer(&account_id);
				}
			}

			LastTransferExpiryCleanupTick::<T>::put(current_tick);
			weight.saturating_add(T::WeightInfo::on_initialize_cleanup(expiring_len))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_chain_config())]
		pub fn set_chain_config(
			origin: OriginFor<T>,
			source_chain: SourceChain,
			config: ChainConfig,
		) -> DispatchResult {
			ensure_root(origin)?;
			match config {
				ChainConfig::Evm { chain_id, gateway, argon_token, argonot_token } => {
					ensure!(
						chain_id != 0 &&
							gateway != H160::zero() &&
							argon_token != H160::zero() &&
							argonot_token != H160::zero(),
						Error::<T>::InvalidChainConfig,
					);

					if let Some(previous_config) = ChainConfigBySourceChain::<T>::get(source_chain)
					{
						match previous_config {
							ChainConfig::Evm {
								chain_id: prev_chain_id,
								argon_token: prev_argon_token,
								argonot_token: prev_argonot_token,
								..
							} => {
								ensure!(
									chain_id == prev_chain_id &&
										argon_token == prev_argon_token &&
										argonot_token == prev_argonot_token,
									Error::<T>::InvalidChainConfig,
								);
							},
						}
					}
				},
			};

			Self::ensure_burn_account_unreapable(&Self::burn_account(source_chain));
			ChainConfigBySourceChain::<T>::insert(source_chain, config);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::force_set_global_issuance_council())]
		pub fn force_set_global_issuance_council(
			origin: OriginFor<T>,
			destination_chain: SourceChain,
			#[pallet::compact] after_nonce: CouncilApprovalQueueNonce,
			member_account_ids: BoundedVec<T::AccountId, T::MaxCouncilMembers>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(!member_account_ids.is_empty(), Error::<T>::InvalidGlobalIssuanceCouncil);
			let last_synced_nonce = GatewayStateBySourceChain::<T>::get(destination_chain)
				.map_or(0, |state| state.argon_approvals_nonce);
			ensure!(after_nonce >= last_synced_nonce, Error::<T>::InvalidForceSetAfterNonce);

			let epoch_microgons_per_argonot =
				T::PriceProvider::get_lowest_microgons_per_argonot(T::CouncilRotationFrames::get())
					.filter(|price| *price != T::Balance::default())
					.ok_or(Error::<T>::InvalidMicrogonsPerArgonot)?;
			let mut total_weight = T::Balance::default();
			let mut council_members = BoundedBTreeMap::new();
			let mut seen_accounts = BoundedVec::<T::AccountId, T::MaxCouncilMembers>::new();
			let mut promoted_signers = Vec::new();

			for member_account_id in member_account_ids {
				ensure!(
					!seen_accounts.contains(&member_account_id),
					Error::<T>::DuplicateGlobalIssuanceCouncilAccount,
				);
				let committed_microgon_collateral = T::VaultProvider::get_committed_securitization(
					&member_account_id,
					T::CouncilRotationFrames::get(),
				)
				.ok_or(Error::<T>::UnknownOwnerVault)?;
				let committed_argonots =
					T::VaultProvider::get_committed_argonots(&member_account_id)
						.ok_or(Error::<T>::UnknownOwnerVault)?;
				let epoch_microgons_per_argonot: u128 = epoch_microgons_per_argonot.into();
				let argonot_weight = committed_argonots
					.into()
					.saturating_mul(epoch_microgons_per_argonot)
					.saturating_div(MICROGONS_PER_ARGON)
					.into();
				let member_weight = committed_microgon_collateral.saturating_add(argonot_weight);
				let pending_signer = PendingCouncilSignerByDestinationChainAndAccountId::<T>::get(
					destination_chain,
					&member_account_id,
				);
				let signer = pending_signer
					.or_else(|| {
						CouncilSignerByDestinationChainAndAccountId::<T>::get(
							destination_chain,
							&member_account_id,
						)
					})
					.ok_or(Error::<T>::CouncilSignerNotRegistered)?;
				ensure!(
					member_weight != T::Balance::default(),
					Error::<T>::InvalidGlobalIssuanceCouncil,
				);
				ensure!(
					!council_members.contains_key(&signer),
					Error::<T>::DuplicateGlobalIssuanceCouncilSigner,
				);

				seen_accounts
					.try_push(member_account_id.clone())
					.map_err(|_| Error::<T>::InvalidGlobalIssuanceCouncil)?;
				let _ = council_members
					.try_insert(
						signer,
						GlobalIssuanceCouncilMember::<T> {
							account_id: member_account_id.clone(),
							signer,
							weight: member_weight,
						},
					)
					.map_err(|_| Error::<T>::InvalidGlobalIssuanceCouncil)?;
				total_weight = total_weight.saturating_add(member_weight);

				if pending_signer.is_some() {
					promoted_signers.push((member_account_id.clone(), signer));
				}
			}

			ensure!(
				total_weight != T::Balance::default(),
				Error::<T>::InvalidGlobalIssuanceCouncil,
			);
			let council_hash =
				Self::hash_global_issuance_council(&council_members, epoch_microgons_per_argonot);
			let council = GlobalIssuanceCouncil::<T> {
				epoch_microgons_per_argonot,
				members: council_members,
				total_weight,
			};

			for (account_id, signer) in promoted_signers {
				CouncilSignerByDestinationChainAndAccountId::<T>::insert(
					destination_chain,
					&account_id,
					signer,
				);
				PendingCouncilSignerByDestinationChainAndAccountId::<T>::remove(
					destination_chain,
					&account_id,
				);
			}

			let previous_council =
				ActiveGlobalIssuanceCouncilByDestinationChain::<T>::get(destination_chain)
					.and_then(GlobalIssuanceCouncilByHash::<T>::get);
			let previous_council_for_cursor_reset =
				if after_nonce == last_synced_nonce { previous_council.as_ref() } else { None };
			Self::reset_council_approval_cursor(
				destination_chain,
				after_nonce,
				previous_council_for_cursor_reset,
				&council,
			)?;
			GlobalIssuanceCouncilByHash::<T>::insert(council_hash, council);
			let prunable_council_hash =
				ActiveGlobalIssuanceCouncilByDestinationChain::<T>::get(destination_chain);
			ActiveGlobalIssuanceCouncilByDestinationChain::<T>::insert(
				destination_chain,
				council_hash,
			);
			if CurrentTransferOutMicrogonsPerArgonotByDestinationChain::<T>::get(destination_chain)
				.filter(|rate| *rate != T::Balance::default())
				.is_none()
			{
				// Force-sets repair council membership. Only proved gateway rotations advance the
				// live transfer-out floor once one has already been established.
				CurrentTransferOutMicrogonsPerArgonotByDestinationChain::<T>::insert(
					destination_chain,
					epoch_microgons_per_argonot,
				);
			}
			Self::rebase_unresolved_queue_entries(destination_chain, after_nonce, council_hash)?;
			Self::refresh_destination_chain_queue_tracking(destination_chain)?;
			if let Some(prunable_council_hash) = prunable_council_hash {
				Self::prune_global_issuance_council_if_unreferenced(
					destination_chain,
					prunable_council_hash,
				);
			}
			Self::deposit_event(Event::GlobalIssuanceCouncilForced {
				destination_chain,
				council_hash,
			});
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::pause_gateway())]
		pub fn pause_gateway(
			origin: OriginFor<T>,
			source_chain: SourceChain,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let last_good_gateway_activity_nonce =
				GatewayStateBySourceChain::<T>::get(source_chain)
					.map_or(0, |state| state.gateway_activity_nonce);
			Self::pause_source_chain(
				source_chain,
				last_good_gateway_activity_nonce,
				last_good_gateway_activity_nonce.saturating_add(1),
				GatewaySyncPauseReason::Manual,
			);

			Ok(Pays::No.into())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::unpause_gateway())]
		pub fn unpause_gateway(origin: OriginFor<T>, source_chain: SourceChain) -> DispatchResult {
			ensure_root(origin)?;

			if GatewaySyncPauseBySourceChain::<T>::take(source_chain).is_some() {
				Self::deposit_event(Event::GatewayUnpaused { source_chain });
			}

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_minimum_minting_authority_value(
			origin: OriginFor<T>,
			destination_chain: SourceChain,
			#[pallet::compact] minimum_value: T::Balance,
		) -> DispatchResult {
			ensure_root(origin)?;
			MinimumMintingAuthorityValueByDestinationChain::<T>::insert(
				destination_chain,
				minimum_value,
			);
			Self::deposit_event(Event::MinimumMintingAuthorityValueSet {
				destination_chain,
				minimum_value,
			});
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_minting_authority_activation_repayment_pricing(
			origin: OriginFor<T>,
			destination_chain: SourceChain,
			pricing: MintingAuthorityActivationRepaymentPricing<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				pricing.activation_gas_cost > 0 &&
					pricing.signature_gas_cost > 0 &&
					pricing.estimated_wei_per_gas > 0 &&
					pricing.estimated_microgons_per_eth > T::Balance::default(),
				Error::<T>::InvalidMintingAuthorityActivationRepaymentPricing,
			);
			ensure!(
				ChainConfigBySourceChain::<T>::contains_key(destination_chain),
				Error::<T>::UnsupportedSource,
			);
			MintingAuthorityActivationRepaymentPricingByDestinationChain::<T>::insert(
				destination_chain,
				pricing,
			);
			Self::deposit_event(Event::MintingAuthorityActivationRepaymentPricingSet {
				destination_chain,
			});
			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::register_council_signer())]
		pub fn register_council_signer(
			origin: OriginFor<T>,
			destination_chain: SourceChain,
			signing_key: H160,
			signature: KeccakSignature,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			T::VaultProvider::get_vault_id(&account_id).ok_or(Error::<T>::UnknownOwnerVault)?;

			ensure!(
				Self::recover_evm_personal_signer(
					&Self::council_signer_registration_message(destination_chain, &account_id),
					&signature,
				) == Some(signing_key),
				Error::<T>::InvalidCouncilSignerProof,
			);

			if CouncilSignerByDestinationChainAndAccountId::<T>::contains_key(
				destination_chain,
				&account_id,
			) {
				PendingCouncilSignerByDestinationChainAndAccountId::<T>::insert(
					destination_chain,
					&account_id,
					signing_key,
				);
				Self::deposit_event(Event::CouncilSignerRotationQueued {
					destination_chain,
					account_id,
					signer: signing_key,
				});
			} else {
				CouncilSignerByDestinationChainAndAccountId::<T>::insert(
					destination_chain,
					&account_id,
					signing_key,
				);
				Self::deposit_event(Event::CouncilSignerRegistered {
					destination_chain,
					account_id,
					signer: signing_key,
				});
			}
			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::approve_queue_entries(signatures.len().max(1) as u32))]
		pub fn approve_queue_entries(
			origin: OriginFor<T>,
			destination_chain: SourceChain,
			signatures: BoundedVec<KeccakSignature, T::MaxQueueApprovalsPerCall>,
		) -> DispatchResultWithPostInfo {
			let council_member_account_id = ensure_signed(origin)?;
			ensure!(!signatures.is_empty(), Error::<T>::NoCouncilApprovalSignaturesProvided,);
			Self::ensure_source_chain_not_paused(destination_chain)?;
			let mut approval_queue_nonce = Self::next_council_approval_queue_nonce_for_account(
				destination_chain,
				&council_member_account_id,
			)
			.ok_or(Error::<T>::GlobalIssuanceCouncilMemberNotFound)?;

			for signature in signatures {
				let mut approval_became_ready = false;
				let mut approved_target = None;

				CouncilApprovalQueueByDestinationChainAndNonce::<T>::try_mutate(
					destination_chain,
					approval_queue_nonce,
					|entry| -> DispatchResult {
						let entry =
							entry.as_mut().ok_or(Error::<T>::CouncilApprovalQueueEntryNotFound)?;
						let approving_council =
							GlobalIssuanceCouncilByHash::<T>::get(entry.approving_council_hash)
								.ok_or(Error::<T>::GlobalIssuanceCouncilNotFound)?;
						let council_member = approving_council
							.members
							.values()
							.find(|member| member.account_id == council_member_account_id)
							.cloned()
							.ok_or(Error::<T>::GlobalIssuanceCouncilMemberNotFound)?;
						approved_target = Some(entry.target);

						let recovered_signer =
							Self::recover_evm_message_signer(entry.approval_hash, &signature)
								.ok_or(Error::<T>::InvalidCouncilApprovalSignature)?;
						ensure!(
							recovered_signer == council_member.signer,
							Error::<T>::InvalidCouncilApprovalSignature,
						);

						ensure!(
							!entry.signatures.contains_key(&council_member.signer),
							Error::<T>::CouncilApprovalAlreadyRecorded,
						);

						let was_ready =
							Self::global_issuance_council_has_quorum(&approving_council, entry);
						let _ = entry
							.signatures
							.try_insert(council_member.signer, signature)
							.map_err(|_| Error::<T>::InvalidGlobalIssuanceCouncil)?;
						entry.approved_total_weight =
							entry.approved_total_weight.saturating_add(council_member.weight);
						approval_became_ready = !was_ready &&
							Self::global_issuance_council_has_quorum(&approving_council, entry);
						Ok(())
					},
				)?;
				CouncilApprovalCursorByDestinationChainAndAccountId::<T>::insert(
					destination_chain,
					&council_member_account_id,
					approval_queue_nonce,
				);

				let approved_target =
					approved_target.ok_or(Error::<T>::CouncilApprovalQueueEntryNotFound)?;
				Self::deposit_event(Event::QueueEntryApprovalRecorded {
					destination_chain,
					target: approved_target,
					approval_queue_nonce,
				});
				if approval_became_ready {
					Self::deposit_event(Event::QueueEntryApprovalReady {
						destination_chain,
						target: approved_target,
						approval_queue_nonce,
					});
				}
				approval_queue_nonce = approval_queue_nonce.saturating_add(1);
			}
			Ok(Pays::No.into())
		}

		#[pallet::call_index(8)]
		#[pallet::weight({
			let activities = proof.activity_logs.len() as u32;
			prove_gateway_activity_with_providers::<T>(activities)
				.saturating_add(
					T::OperationalAccountsHook::uniswap_transfer_confirmed_weight()
						.saturating_mul(activities as u64)
				)
		})]
		pub fn prove_gateway_activity(
			origin: OriginFor<T>,
			source_chain: SourceChain,
			#[pallet::compact] previous_gateway_activity_nonce: GatewayActivityNonce,
			proof: GatewayActivityProof<<T as Config>::MaxActivitiesPerGatewayProof>,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;
			Self::ensure_source_chain_not_paused(source_chain)?;
			ensure!(!proof.activity_logs.is_empty(), Error::<T>::NoGatewayActivitiesProvided);
			let current_gateway_state = GatewayStateBySourceChain::<T>::get(source_chain)
				.unwrap_or(GatewayState::<T> {
					gateway_activity_nonce: 0,
					argon_approvals_nonce: 0,
					argon_circulation: T::Balance::default(),
					argonot_circulation: T::Balance::default(),
				});

			ensure!(
				previous_gateway_activity_nonce == current_gateway_state.gateway_activity_nonce,
				Error::<T>::UnexpectedPreviousGatewayActivityNonce,
			);
			let activity_root_seed = if previous_gateway_activity_nonce == 0 {
				H256::zero()
			} else {
				let locator_hash = LastAcceptedLocatorHashBySourceChain::<T>::get(source_chain);
				ensure!(locator_hash != H256::zero(), Error::<T>::MissingLastAcceptedLocatorHash,);
				locator_hash
			};
			let (_, gateway) = Self::evm_gateway_signature_domain(source_chain)?;
			T::EthereumVerifier::verify_account_storage_proof(gateway, &proof.storage_proof)
				.map_err(|_| Error::<T>::InvalidProof)?;
			let (locator_hash, decoded_activities) = Self::validate_gateway_activity_proof(
				source_chain,
				previous_gateway_activity_nonce,
				activity_root_seed,
				&proof,
			)
			.map_err(|_| Error::<T>::InvalidProof)?;

			let latest_gateway_state = match with_storage_layer(|| {
				let mut expected_gateway_activity_nonce = previous_gateway_activity_nonce;
				let mut latest_gateway_state = None;

				for decoded_activity in decoded_activities {
					let gateway_state = Self::apply_decoded_gateway_activity(
						source_chain,
						expected_gateway_activity_nonce,
						decoded_activity,
					)?;
					expected_gateway_activity_nonce = gateway_state.gateway_activity_nonce;
					latest_gateway_state = Some(gateway_state);
				}

				latest_gateway_state.ok_or(GatewayActivityApplyError::Reject(
					Error::<T>::NoGatewayActivitiesProvided.into(),
				))
			}) {
				Ok(gateway_state) => gateway_state,
				Err(GatewayActivityApplyError::Pause { failed_gateway_activity_nonce, reason }) => {
					Self::pause_source_chain(
						source_chain,
						previous_gateway_activity_nonce,
						failed_gateway_activity_nonce,
						reason,
					);
					return Ok(Pays::No.into());
				},
				Err(GatewayActivityApplyError::Reject(error)) => return Err(error.into()),
			};

			if latest_gateway_state.argon_approvals_nonce >
				current_gateway_state.argon_approvals_nonce
			{
				let last_local_queue_nonce =
					NextCouncilApprovalQueueNonceByDestinationChain::<T>::get(source_chain);
				let previous_retained_queue_nonce =
					current_gateway_state.argon_approvals_nonce.min(last_local_queue_nonce);
				let retained_queue_nonce =
					latest_gateway_state.argon_approvals_nonce.min(last_local_queue_nonce);

				for queue_nonce in previous_retained_queue_nonce.max(1)..retained_queue_nonce {
					CouncilApprovalQueueByDestinationChainAndNonce::<T>::remove(
						source_chain,
						queue_nonce,
					);
				}
			}
			GatewayStateBySourceChain::<T>::insert(source_chain, latest_gateway_state.clone());
			LastAcceptedLocatorHashBySourceChain::<T>::insert(source_chain, locator_hash);
			Self::refresh_destination_chain_queue_tracking(source_chain)?;
			Self::deposit_event(Event::GatewayStateAdvanced {
				source_chain,
				gateway_state: latest_gateway_state,
			});

			Ok(Pays::No.into())
		}

		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::register_minting_authority())]
		pub fn register_minting_authority(
			origin: OriginFor<T>,
			destination_chain: SourceChain,
			destination_signing_key: H160,
			signature: KeccakSignature,
			#[pallet::compact] microgon_collateral: T::Balance,
			#[pallet::compact] micronot_collateral: T::Balance,
		) -> DispatchResult {
			let vault_operator_account_id = ensure_signed(origin)?;
			Self::do_register_minting_authority(
				vault_operator_account_id,
				destination_chain,
				destination_signing_key,
				signature,
				microgon_collateral,
				micronot_collateral,
			)
		}

		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::deactivate_minting_authority())]
		pub fn deactivate_minting_authority(
			origin: OriginFor<T>,
			destination_signing_key: H160,
		) -> DispatchResultWithPostInfo {
			let account_id = ensure_signed(origin)?;
			Self::do_deactivate_minting_authority(account_id, destination_signing_key)
		}

		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::transfer_out())]
		pub fn transfer_out(
			origin: OriginFor<T>,
			destination_chain: SourceChain,
			asset: AssetKind,
			destination_account: H160,
			#[pallet::compact] amount: T::Balance,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			Self::ensure_source_chain_not_paused(destination_chain)?;
			Self::do_transfer_out(account_id, destination_chain, asset, destination_account, amount)
		}

		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::collateralize_transfer())]
		pub fn collateralize_transfer(
			origin: OriginFor<T>,
			transfer_id: H256,
			signature: KeccakSignature,
			#[pallet::compact] microgon_collateral: T::Balance,
			#[pallet::compact] micronot_collateral: T::Balance,
		) -> DispatchResultWithPostInfo {
			let account_id = ensure_signed(origin)?;
			Self::do_collateralize_transfer(
				account_id,
				transfer_id,
				signature,
				microgon_collateral,
				micronot_collateral,
			)?;
			Ok(Pays::No.into())
		}

		#[pallet::call_index(13)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn set_last_accepted_locator_hash(
			origin: OriginFor<T>,
			source_chain: SourceChain,
			locator_hash: H256,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				LastAcceptedLocatorHashBySourceChain::<T>::get(source_chain) == H256::zero(),
				Error::<T>::LastAcceptedLocatorHashAlreadySet,
			);
			LastAcceptedLocatorHashBySourceChain::<T>::insert(source_chain, locator_hash);
			Self::deposit_event(Event::LastAcceptedLocatorHashSet { source_chain, locator_hash });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn burn_account(source_chain: SourceChain) -> T::AccountId {
			match source_chain {
				SourceChain::Ethereum => T::EthereumBurnAccount::get(),
			}
		}

		pub(crate) fn pending_transfer_out_circulation(
			source_chain: SourceChain,
		) -> SourceChainCirculation<T> {
			PendingTransferOutCirculationByDestinationChain::<T>::get(source_chain)
		}

		pub(crate) fn ensure_source_chain_not_paused(source_chain: SourceChain) -> DispatchResult {
			ensure!(
				!GatewaySyncPauseBySourceChain::<T>::contains_key(source_chain),
				Error::<T>::GatewaySyncPaused,
			);
			Ok(())
		}

		pub(crate) fn pause_source_chain(
			source_chain: SourceChain,
			last_good_gateway_activity_nonce: GatewayActivityNonce,
			failed_gateway_activity_nonce: GatewayActivityNonce,
			reason: GatewaySyncPauseReason,
		) {
			let pause = GatewaySyncPause {
				last_good_gateway_activity_nonce,
				failed_gateway_activity_nonce,
				reason,
			};
			GatewaySyncPauseBySourceChain::<T>::insert(source_chain, pause);
			Self::deposit_event(Event::GatewaySyncPaused { source_chain, pause });
		}

		pub(crate) fn mint_to<C: Mutate<T::AccountId, Balance = T::Balance> + 'static>(
			source_chain: SourceChain,
			amount: T::Balance,
			to: &T::AccountId,
		) -> DispatchResult {
			let burn_account = Self::burn_account(source_chain);
			if amount == 0u128.into() {
				return Ok(());
			}
			ensure!(
				C::reducible_balance(&burn_account, Preservation::Expendable, Fortitude::Force,) >=
					amount,
				Error::<T>::InsufficientLiquidity,
			);

			let _ = C::burn_from(
				&burn_account,
				amount,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)?;
			let _ = C::mint_into(to, amount)?;

			Ok(())
		}

		fn ensure_burn_account_unreapable(account_id: &T::AccountId) {
			let providers = frame_system::Pallet::<T>::providers(account_id);
			for _ in providers..2 {
				frame_system::Pallet::<T>::inc_providers(account_id);
			}
		}

		pub(crate) fn retain_recent_argon_transfer(account_id: &T::AccountId) {
			RecentArgonTransfersByAccount::<T>::mutate(account_id, |count| {
				*count = count.saturating_add(1);
			});

			let expires_at =
				T::CurrentTick::get().saturating_add(T::RecentTransferRetentionTicks::get());
			InboundTransfersExpiringAt::<T>::mutate(expires_at, |accounts| {
				accounts.push(account_id.clone());
			});
		}

		fn decrement_recent_argon_transfer(account_id: &T::AccountId) {
			RecentArgonTransfersByAccount::<T>::mutate_exists(account_id, |count| {
				let Some(existing) = count.as_mut() else {
					return;
				};

				if *existing <= 1 {
					*count = None;
				} else {
					*existing = existing.saturating_sub(1);
				}
			});
		}
	}
	impl<T: Config> UniswapTransferProvider<T::AccountId> for Pallet<T> {
		type Weights = weights::ProviderWeightAdapter<T>;

		fn is_crosschain_activated() -> bool {
			ChainConfigBySourceChain::<T>::contains_key(SourceChain::Ethereum) &&
				!GatewaySyncPauseBySourceChain::<T>::contains_key(SourceChain::Ethereum)
		}

		fn has_recent_argon_transfer(account_id: &T::AccountId) -> bool {
			RecentArgonTransfersByAccount::<T>::get(account_id) > 0
		}
	}

	type RuntimeCallOf<T> = <T as frame_system::Config>::RuntimeCall;

	impl<T: Config> CallTxPoolKeyProvider<RuntimeCallOf<T>, T::AccountId> for Pallet<T>
	where
		RuntimeCallOf<T>: IsSubType<Call<T>>,
	{
		fn key_for(call: &RuntimeCallOf<T>, _signer: Option<&T::AccountId>) -> Option<Vec<u8>> {
			let call = <RuntimeCallOf<T> as IsSubType<Call<T>>>::is_sub_type(call)?;

			match call {
				Call::prove_gateway_activity { source_chain, proof, .. } => Some(
					(
						b"crosschain_transfer:prove".as_slice(),
						source_chain,
						proof.using_encoded(blake2_256),
					)
						.using_encoded(blake2_256)
						.to_vec(),
				),
				_ => None,
			}
		}
	}

	impl<T: Config> CallTxValidityProvider<RuntimeCallOf<T>, T::AccountId> for Pallet<T>
	where
		RuntimeCallOf<T>: IsSubType<Call<T>>,
	{
		fn validate(
			call: &RuntimeCallOf<T>,
			_signer: Option<&T::AccountId>,
		) -> Result<(), TransactionValidityError> {
			let Some(call) = <RuntimeCallOf<T> as IsSubType<Call<T>>>::is_sub_type(call) else {
				return Ok(());
			};

			if let Call::prove_gateway_activity {
				source_chain,
				previous_gateway_activity_nonce,
				..
			} = call
			{
				if GatewaySyncPauseBySourceChain::<T>::contains_key(source_chain) {
					return Err(TransactionValidityError::Invalid(InvalidTransaction::Stale));
				}

				let current_nonce = GatewayStateBySourceChain::<T>::get(source_chain)
					.map(|state| state.gateway_activity_nonce)
					.unwrap_or_default();

				if previous_gateway_activity_nonce < &current_nonce {
					return Err(TransactionValidityError::Invalid(InvalidTransaction::Stale));
				}
			}

			Ok(())
		}
	}

	impl<T: Config> CollectBlockerProvider<T::AccountId> for Pallet<T> {
		type Weights = super::weights::ProviderWeightAdapter<T>;

		fn has_overdue_collect_blocker(account_id: &T::AccountId) -> bool {
			let Some(next_due_nonce) = Self::next_council_approval_queue_nonce_for_account(
				SourceChain::Ethereum,
				account_id,
			) else {
				return false;
			};
			let Some(entry) = CouncilApprovalQueueByDestinationChainAndNonce::<T>::get(
				SourceChain::Ethereum,
				next_due_nonce,
			) else {
				return false;
			};

			entry.due_frame_id <= T::CurrentFrameId::get()
		}
	}
}
