#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::zero_prefixed_literal)]

extern crate alloc;
extern crate core;

use alloc::vec::Vec;
use codec::Encode;
use pallet_prelude::*;
use polkadot_sdk::sp_runtime::traits::{IdentifyAccount, Verify};

use argon_bitcoin::CosignReleaser;
use argon_primitives::{
	bitcoin::{
		BitcoinNetwork, BitcoinSignature, CompressedBitcoinPubkey, Satoshis, UtxoId,
		SATOSHIS_PER_BITCOIN,
	},
	providers::{BitcoinFissionLockError, BitcoinFissionLockProvider, BitcoinFissionsProvider},
};
pub use pallet::*;
pub use weights::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;

pub mod migrations;
#[cfg(test)]
mod tests;
pub mod weights;

/// The Bitcoin Locks pallet creates and manages receive addresses backed by vault securitization.
/// A Lock records the amount of Bitcoin covered by securitization independently from the amount
/// that eventually funds its receive address.
///
/// ** Vaults: **
/// Vaults are managed in the vault pallet, but determine the amount of funding eligible for
/// locking.
///
/// ** Bitcoin Locks: **
///
/// The first output detected for an unfunded Lock becomes its funding UTXO; later outputs to the
/// same address remain recoverable orphans. Pending securitization may expire before funding
/// arrives without invalidating the receive address. Funded satoshis allocated to active Fission
/// positions prevent the Lock from being released.
///
/// _Bitcoin multisig/ownership_
/// A bitcoin holder retains ownership of their UTXO via a pubkey script that is pre-agreed by the
/// vault user and the bitcoin holder. The vault's hashed public key can be obtained in this pallet,
/// and will be combined with a hashed pubkey provided by the user. The pre-agreed script will be
/// such that both signatures are required to release the bitcoin before 370 days of blocks. After
/// 370 days, only the Vault's signature will be required to release the bitcoin for 30 days. After
/// 400 days, either party will be able to release.
///
/// NOTE: the lock will end on day 365, which gives a 5-day grace period for a bitcoin owner to buy
/// back their bitcoin before the vault can claim it.
///
/// _Releasing a Bitcoin_
/// A bitcoin owner will pre-create a transaction to release their UTXO and submit the sighash to
/// this pallet. The vault operator has 10 days to publish a counter signature along with the public
/// key. If the vault operator fails to do so, they will lose their ownership tokens and all
/// underlying Bitcoin locks. A user will be made whole via a governance vote.
///
/// _Penalties_
/// 1. If a UTXO is found to have moved before a lock expiration via the bitcoin network, the vault
///    will be penalized by the amount of the UTXOs' current value.
/// 2. If a vault operator fails to counter-sign a transaction within 10 days, they will lose their
///    ownership tokens and the market value of underlying Bitcoin locks.
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use argon_bitcoin::{Amount, CosignReleaser, CosignScriptArgs, ReleaseStep};
	use argon_primitives::{
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinScriptPubkey, BitcoinSignature,
			CompressedBitcoinPubkey, Satoshis, UtxoId, UtxoRef, XPubChildNumber, XPubFingerprint,
			SATOSHIS_PER_BITCOIN,
		},
		vault::{
			BitcoinResecuritization, BitcoinSecuritization, BitcoinVaultProvider, LockExtension,
			ReserveSecuritizationRequest, VaultError,
		},
		BitcoinFissionsProvider, BitcoinUtxoEvents, BitcoinUtxoTracker, PriceProvider, VaultId,
	};
	use codec::HasCompact;
	use core::iter::Sum;
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(11);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		/// Currency used only to settle release holds created before storage version 11.
		type Currency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ DecodeWithMemTracking
			+ core::fmt::Debug
			+ Default
			+ From<u128>
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen
			+ Sum
			+ HasCompact;

		/// Runtime hold reason containing the retired Bitcoin Lock release hold variant.
		type RuntimeHoldReason: From<HoldReason>;

		type FissionsProvider: BitcoinFissionsProvider<Self::AccountId, Self::Balance>;

		/// Utxo tracker for bitcoin
		type BitcoinUtxoTracker: BitcoinUtxoTracker;

		type PriceProvider: PriceProvider<Self::Balance>;

		type BitcoinSignatureVerifier: BitcoinVerifier<Self>;

		type FeeCouponSigner: Parameter + IdentifyAccount<AccountId = Self::AccountId>;

		type FeeCouponSignature: Parameter + Verify<Signer = Self::FeeCouponSigner>;

		/// Bitcoin time provider
		type BitcoinBlockHeightChange: Get<(BitcoinHeight, BitcoinHeight)>;

		type GetBitcoinNetwork: Get<BitcoinNetwork>;

		type VaultProvider: BitcoinVaultProvider<
			AccountId = Self::AccountId,
			Balance = Self::Balance,
		>;

		/// Argon tick per day
		#[pallet::constant]
		type ArgonTicksPerDay: Get<Tick>;

		/// Maximum releasing utxos at a time
		#[pallet::constant]
		type MaxConcurrentlyReleasingLocks: Get<u32>;

		/// The number of bitcoin blocks a bitcoin is locked for
		#[pallet::constant]
		type LockDurationBlocks: Get<BitcoinHeight>;

		/// Max bitcoin blocks up to which the first observed output may fund the Lock.
		#[pallet::constant]
		type MaxPendingConfirmationBlocks: Get<BitcoinHeight>;

		/// The bitcoin blocks after a BitcoinLock expires which the vault will be allowed to claim
		/// a bitcoin
		#[pallet::constant]
		type LockReclamationBlocks: Get<BitcoinHeight>;

		/// Number of frames a vault has to counter-sign a bitcoin release
		#[pallet::constant]
		type LockReleaseCosignDeadlineFrames: Get<FrameId>;
		/// Number of frames orphaned UTXO release entries are retained after a lock lifecycle
		/// transition before being cleaned up.
		#[pallet::constant]
		type OrphanedUtxoReleaseExpiryFrames: Get<FrameId>;

		/// Getter for the current frame id
		type CurrentFrameId: Get<FrameId>;

		/// Indicates if a new frame has started in the current block
		type DidStartNewFrame: Get<bool>;

		/// Pallet storage requires bounds, so we have to set a maximum number that can expire in a
		/// single block
		#[pallet::constant]
		type MaxConcurrentlyExpiringLocks: Get<u32>;

		/// Number of ticks per bitcoin block
		#[pallet::constant]
		type TicksPerBitcoinBlock: Get<Tick>;

		/// Max allowed tick-age of microgon-per-btc prices
		#[pallet::constant]
		type MaxBtcPriceTickAge: Get<u32>;

		/// Gets the current tick
		type CurrentTick: Get<Tick>;
	}

	/// Retained until release requests created before storage version 11 finish.
	#[pallet::composite_enum]
	pub enum HoldReason {
		ReleaseBitcoinLock,
	}

	#[pallet::storage]
	pub type NextUtxoId<T: Config> = StorageValue<_, UtxoId, OptionQuery>;

	/// Stores bitcoin utxos that have requested to be released
	#[pallet::storage]
	pub type LocksByUtxoId<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, LockedBitcoin<T>, OptionQuery>;

	/// The utxo funding this lock. In the current runtime, this is just the first utxo received
	#[pallet::storage]
	pub type UtxoIdToFundingUtxoRef<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, UtxoRef, OptionQuery>;

	/// Index of active UTXO IDs per vault
	#[pallet::storage]
	pub type UtxoIdsByVaultId<T: Config> =
		StorageDoubleMap<_, Twox64Concat, VaultId, Twox64Concat, UtxoId, (), OptionQuery>;

	/// Index of active UTXO IDs per owner account.
	#[pallet::storage]
	pub type UtxoIdsByOwnerAccount<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, UtxoId, (), OptionQuery>;

	/// Stores the block number where a release was cosigned by the vault.
	#[pallet::storage]
	pub type LockReleaseCosignHeightById<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, BlockNumberFor<T>, OptionQuery>;

	/// Stores bitcoin locks that have requested to be released
	#[pallet::storage]
	pub type LockReleaseRequestsByUtxoId<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, LockReleaseRequest<T::Balance>, OptionQuery>;

	/// Release amounts identified by the version 10 to 11 migration.
	///
	/// Current release requests do not create currency holds. Each migrated entry is removed when
	/// its in-flight release request terminates.
	#[pallet::storage]
	pub type MigratedReleaseHoldByUtxoId<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, T::Balance, OptionQuery>;

	/// Mismatched utxos that were sent with invalid amounts to a locked bitcoin
	#[pallet::storage]
	pub type OrphanedUtxosByAccount<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		UtxoRef,
		OrphanedUtxo<BlockNumberFor<T>>,
		OptionQuery,
	>;

	/// The minimum number of satoshis accepted in one watched funding UTXO.
	#[pallet::storage]
	pub type MinimumSatoshis<T: Config> = StorageValue<_, Satoshis, ValueQuery>;

	/// Utxos that have been requested to be cosigned for releasing
	#[pallet::storage]
	pub type LockCosignDueByFrame<T: Config> = StorageMap<
		_,
		Twox64Concat,
		FrameId,
		BoundedBTreeSet<UtxoId, T::MaxConcurrentlyReleasingLocks>,
		ValueQuery,
	>;

	/// Expiration of bitcoin locks by bitcoin height. Funds are burned since the user did not
	/// unlock it. Bitcoin will go to vault
	#[pallet::storage]
	pub type LockExpirationsByBitcoinHeight<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinHeight,
		BoundedBTreeSet<UtxoId, T::MaxConcurrentlyExpiringLocks>,
		ValueQuery,
	>;

	/// Unfunded locks whose pending securitization reservation expires at the indexed height.
	/// Expiry releases the reservation but leaves the lock address watched for late orphan output.
	#[pallet::storage]
	pub type LocksPendingFundingByBitcoinHeight<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinHeight,
		BoundedBTreeSet<UtxoId, T::MaxConcurrentlyExpiringLocks>,
		ValueQuery,
	>;

	/// Highest Bitcoin UTXO sync height processed for pending-funding expirations.
	#[pallet::storage]
	pub type LastPendingFundingExpirationHeight<T: Config> =
		StorageValue<_, BitcoinHeight, OptionQuery>;

	/// Expiration of orphaned utxo refs by user account
	#[pallet::storage]
	pub type OrphanedUtxoExpirationByFrame<T: Config> = StorageMap<
		_,
		Twox64Concat,
		FrameId,
		BoundedBTreeSet<
			(<T as frame_system::Config>::AccountId, UtxoRef),
			T::MaxConcurrentlyExpiringLocks,
		>,
		ValueQuery,
	>;

	/// Recent target-normalized microgon values per BTC and their observed ticks.
	#[pallet::storage]
	#[pallet::storage_prefix = "MicrogonPerBtcHistory"]
	pub type MicrogonsAtTargetPerBtcHistory<T: Config> =
		StorageValue<_, BoundedVec<(Tick, T::Balance), T::MaxBtcPriceTickAge>, ValueQuery>;

	#[pallet::storage]
	pub type LastFeeCouponNonceByVaultAndAccount<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		VaultId,
		Blake2_128Concat,
		T::AccountId,
		u64,
		OptionQuery,
	>;

	#[derive(Decode, Encode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct LockedBitcoin<T: Config> {
		#[codec(compact)]
		pub vault_id: VaultId,
		/// Satoshis covered by this Lock's securitization.
		#[codec(compact)]
		pub securitized_satoshis: Satoshis,
		/// Target-normalized microgon value per BTC used for this Lock's securitization.
		#[codec(compact)]
		pub microgons_at_target_per_btc: T::Balance,
		/// Microgon coverage purchased after applying the redemption curve.
		#[codec(compact)]
		pub securitization_coverage_microgons: T::Balance,
		/// Tick of the price-history entry used for the current securitization.
		#[codec(compact)]
		pub securitization_tick: Tick,
		/// Satoshis attached to this Lock by confirmed funding outputs. Zero means unfunded.
		#[codec(compact)]
		pub funded_satoshis: Satoshis,
		/// Funded satoshis currently split into active Fissions.
		#[codec(compact)]
		pub fissioned_satoshis: Satoshis,
		/// The owner account
		pub owner_account: T::AccountId,
		/// The guaranteed securitization ratio for this lock
		pub securitization_ratio: FixedU128,
		/// Sum of all lock fees (initial plus any resecuritization)
		#[codec(compact)]
		pub security_fees: T::Balance,
		/// Fees covered by the vault for this lock
		#[codec(compact)]
		pub coupon_paid_fees: T::Balance,
		/// The vault pubkey used in the cosign script to lock (and unlock) the bitcoin
		pub vault_pubkey: CompressedBitcoinPubkey,
		/// The vault pubkey used to claim the bitcoin after the lock expiration
		pub vault_claim_pubkey: CompressedBitcoinPubkey,
		/// The vault xpub sources. First is the cosign number, second is the claim number
		pub vault_xpub_sources: (XPubFingerprint, XPubChildNumber, XPubChildNumber),
		/// The bitcoin pubkey provided by the owner of the bitcoin lock that will be needed to
		/// spend the bitcoin (owner side of cosign)
		pub owner_pubkey: CompressedBitcoinPubkey,
		/// The height where the vault has exclusive rights to claim the bitcoin
		#[codec(compact)]
		pub vault_claim_height: BitcoinHeight,
		/// The height where either owner or vault can claim the bitcoin
		#[codec(compact)]
		pub open_claim_height: BitcoinHeight,
		/// The bitcoin height when this lock was created
		#[codec(compact)]
		pub created_at_height: BitcoinHeight,
		/// The bitcoin height when the most recent funding window expires.
		#[codec(compact)]
		pub funding_expiration_height: BitcoinHeight,
		/// The script pubkey where funds are sent to fund this bitcoin lock
		pub utxo_script_pubkey: BitcoinCosignScriptPubkey,
		/// Whether this operator-owned lock may be displaced by outside vault capital.
		pub is_flexible: bool,
		/// Funds used by this bitcoin that will need to be held for extended periods when released
		/// back to the vault
		pub fund_hold_extensions: BoundedBTreeMap<BitcoinHeight, T::Balance, ConstU32<366>>,
		/// The argon block when this lock was created
		#[codec(compact)]
		pub created_at_argon_block: BlockNumberFor<T>,
	}

	impl<T: Config> LockedBitcoin<T> {
		pub fn get_lock_extension(&self) -> LockExtension<T::Balance> {
			LockExtension {
				extended_expiration_funds: self.fund_hold_extensions.clone(),
				lock_expiration: self.vault_claim_height,
			}
		}
		pub fn is_funded(&self) -> bool {
			self.funded_satoshis > 0
		}

		pub fn btc_value_in_microgons(&self) -> T::Balance {
			self.get_securitization().btc_value_in_microgons()
		}

		pub fn get_securitization(&self) -> BitcoinSecuritization<T::Balance> {
			BitcoinSecuritization {
				securitized_satoshis: self.securitized_satoshis,
				microgons_at_target_per_btc: self.microgons_at_target_per_btc,
				securitization_coverage_microgons: self.securitization_coverage_microgons,
				securitization_ratio: self.securitization_ratio,
			}
		}
	}

	#[derive(
		Decode, Encode, CloneNoBound, PartialEqNoBound, EqNoBound, Debug, TypeInfo, MaxEncodedLen,
	)]
	pub struct LockReleaseRequest<
		Balance: Clone + Eq + PartialEq + TypeInfo + Codec + MaxEncodedLen,
	> {
		/// The utxo id this request is related to
		#[codec(compact)]
		pub utxo_id: UtxoId,
		/// The vault id this request is related to
		#[codec(compact)]
		pub vault_id: VaultId,
		/// The network fee to take out of the bitcoin being released
		#[codec(compact)]
		pub bitcoin_network_fee: Satoshis,
		/// The frame when cosign is due
		#[codec(compact)]
		pub cosign_due_frame: FrameId,
		/// The script pubkey where the bitcoin is to be sent
		pub to_script_pubkey: BitcoinScriptPubkey,
		/// The securitization exposed if the vault fails to cosign this release.
		#[codec(compact)]
		pub securitization_at_risk: Balance,
	}

	#[derive(
		Decode, Encode, CloneNoBound, PartialEqNoBound, EqNoBound, Debug, TypeInfo, MaxEncodedLen,
	)]
	pub struct OrphanedUtxo<BlockNumber: Clone + Eq + PartialEq + TypeInfo + Codec + MaxEncodedLen> {
		/// The utxo id this request is related to
		#[codec(compact)]
		pub utxo_id: UtxoId,
		/// The vault id this request is related to
		#[codec(compact)]
		pub vault_id: VaultId,
		/// The number of satoshis in the orphaned utxo
		#[codec(compact)]
		pub satoshis: Satoshis,
		/// The block where this was detected
		#[codec(compact)]
		pub recorded_argon_block_number: BlockNumber,
		/// The cosign request to release this utxo, if any
		pub cosign_request: Option<OrphanedUtxoCosignRequest<BlockNumber>>,
	}

	#[derive(
		Decode, Encode, CloneNoBound, PartialEqNoBound, EqNoBound, Debug, TypeInfo, MaxEncodedLen,
	)]
	pub struct OrphanedUtxoCosignRequest<
		BlockNumber: Clone + Eq + PartialEq + TypeInfo + Codec + MaxEncodedLen,
	> {
		/// The network fee to take out of the bitcoin being released
		pub bitcoin_network_fee: Satoshis,
		/// The script pubkey where the bitcoin is to be sent
		pub to_script_pubkey: BitcoinScriptPubkey,
		/// When this was requested to be released
		pub created_at_argon_block_number: BlockNumber,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		BitcoinLockCreated {
			utxo_id: UtxoId,
			vault_id: VaultId,
			securitized_satoshis: Satoshis,
			microgons_at_target_per_btc: T::Balance,
			collateral_required: T::Balance,
			account_id: T::AccountId,
			security_fee: T::Balance,
		},
		BitcoinLockBurned {
			utxo_id: UtxoId,
			vault_id: VaultId,
			was_utxo_spent: bool,
		},
		BitcoinUtxoCosignRequested {
			utxo_id: UtxoId,
			vault_id: VaultId,
		},
		BitcoinUtxoCosigned {
			utxo_id: UtxoId,
			vault_id: VaultId,
			signature: BitcoinSignature,
		},
		BitcoinSpentAfterRelease {
			utxo_id: UtxoId,
			vault_id: VaultId,
		},
		BitcoinCosignPastDue {
			utxo_id: UtxoId,
			vault_id: VaultId,
			compensation_amount: T::Balance,
			compensated_account_id: T::AccountId,
		},
		/// An error occurred while refunding an overdue cosigned bitcoin lock
		CosignOverdueError {
			utxo_id: UtxoId,
			error: DispatchError,
		},
		/// An error occurred while completing a lock
		LockExpirationError {
			utxo_id: UtxoId,
			error: DispatchError,
		},
		OrphanedUtxoReceived {
			utxo_id: UtxoId,
			utxo_ref: UtxoRef,
			vault_id: VaultId,
			satoshis: Satoshis,
		},
		OrphanedUtxoReleaseRequested {
			utxo_id: UtxoId,
			utxo_ref: UtxoRef,
			vault_id: VaultId,
			account_id: T::AccountId,
		},
		OrphanedUtxoCosigned {
			utxo_id: UtxoId,
			utxo_ref: UtxoRef,
			vault_id: VaultId,
			account_id: T::AccountId,
			signature: BitcoinSignature,
		},
		/// An orphaned UTXO expiration could not reconcile its pending Vault cosign state.
		OrphanedUtxoExpirationError {
			account_id: T::AccountId,
			utxo_ref: UtxoRef,
			error: DispatchError,
		},
		/// Not all orphaned UTXOs for a retired Lock fit in the cleanup schedule.
		OrphanedUtxoCleanupScheduleOverflow {
			account_id: T::AccountId,
			utxo_id: UtxoId,
			expiration_frame: FrameId,
		},
		BitcoinLockResecuritized {
			utxo_id: UtxoId,
			vault_id: VaultId,
			securitized_satoshis: Satoshis,
			microgons_at_target_per_btc: T::Balance,
			account_id: T::AccountId,
		},
		BitcoinLockFlexibleChanged {
			utxo_id: UtxoId,
			vault_id: VaultId,
			is_flexible: bool,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		InsufficientFunds,
		InsufficientVaultFunds,
		/// The proposed transaction would take the account below the minimum (existential) balance
		AccountWouldGoBelowMinimumBalance,
		/// This vault is closed
		VaultClosed,
		/// Funding would result in an overflow of the balance type
		InvalidVaultAmount,
		/// This bitcoin redemption has not been locked in
		RedemptionNotLocked,
		/// The bitcoin has passed the deadline to release it
		BitcoinReleaseInitiationDeadlinePassed,
		/// The fee for this bitcoin release is too high
		BitcoinFeeTooHigh,
		/// The Bitcoin Unspect Transaction Output (UTXO) was not found
		BitcoinUtxoNotFound,
		/// This bitcoin cosign script couldn't be decoded for release
		BitcoinUnableToBeDecodedForRelease,
		/// This bitcoin signature couldn't be decoded for release
		BitcoinSignatureUnableToBeDecoded,
		/// This bitcoin pubkey couldn't be decoded for release
		BitcoinPubkeyUnableToBeDecoded,
		/// The cosign signature is not valid for the bitcoin release
		BitcoinInvalidCosignature,
		/// The price provider has no bitcoin prices available. This is a temporary error
		NoBitcoinPricesAvailable,
		/// The bitcoin script to lock this bitcoin has errors
		InvalidBitcoinScript,
		/// The user does not have permissions to perform this action
		NoPermissions,
		/// The Lock cannot be released while it has active Fissions.
		LockHasActiveFissions,
		/// The requested Lock securitization has fewer satoshis than its active Fissions.
		InsufficientSatoshisForFissions,
		/// The Lock records fissioned satoshis without matching active Fission requirements.
		FissionStateMismatch,
		/// The requested Lock securitization does not cover its active Fission liabilities.
		InsufficientSecuritizationForFissions,
		/// The expected amount of funds to return from hold was not available
		HoldUnexpectedlyModified,
		/// The hold on funds could not be recovered
		UnrecoverableHold,
		/// The vault was not found
		VaultNotFound,
		/// An error occurred in the vault module
		GenericVaultError(VaultError),
		/// The Bitcoin Lock record was not found
		LockNotFound,
		/// No Vault public keys are available
		NoVaultBitcoinPubkeysAvailable,
		/// Unable to generate a new vault public key
		UnableToGenerateVaultBitcoinPubkey,
		/// This vault is not yet active
		VaultNotYetActive,
		/// An overflow occurred recording a lock expiration
		ExpirationAtBlockOverflow,
		/// The requested securitization matches the Lock's current securitization.
		NoResecuritizationChange,
		/// A Lock in the release process cannot be resecuritized.
		LockInProcessOfRelease,
		/// The lock funding has not been confirmed on bitcoin
		LockPendingFunding,
		/// An overflow or underflow occurred while calculating the redemption price
		OverflowError,
		/// The requested target-normalized BTC value is not present in recent price history.
		IneligibleMicrogonsAtTargetPerBtcRequested,
		/// The requested price-history entry predates the Lock's current securitization.
		MicrogonsAtTargetPerBtcTickOlderThanCurrent,
		/// The fee coupon is past its expiration frame.
		FeeCouponExpired,
		/// The fee coupon was not signed by the vault delegate.
		InvalidFeeCouponSignature,
		/// The fee coupon nonce is not the next unconsumed nonce.
		FeeCouponAlreadyUsed,
		/// Cannot fund with an orphaned utxo after lock funding is confirmed
		OrphanedUtxoFundingConflict,
		/// Cannot lock an orphaned utxo with a pending release request
		OrphanedUtxoReleaseRequested,
		/// Cannot request an orphaned release for the funding UTXO
		FundingUtxoCannotBeReleased,
		/// Too many orphaned utxo release requests for a lock
		MaxOrphanedUtxoReleaseRequestsExceeded,
	}

	impl<T> From<VaultError> for Error<T> {
		fn from(e: VaultError) -> Error<T> {
			match e {
				VaultError::InsufficientFunds => Error::<T>::InsufficientFunds,
				VaultError::HoldUnexpectedlyModified => Error::<T>::HoldUnexpectedlyModified,
				VaultError::UnrecoverableHold => Error::<T>::UnrecoverableHold,
				VaultError::VaultNotFound => Error::<T>::VaultNotFound,
				VaultError::InsufficientVaultFunds => Error::<T>::InsufficientVaultFunds,
				VaultError::VaultClosed => Error::<T>::VaultClosed,
				VaultError::AccountWouldBeBelowMinimum =>
					Error::<T>::AccountWouldGoBelowMinimumBalance,
				VaultError::InvalidBitcoinScript => Error::<T>::InvalidBitcoinScript,
				VaultError::NoVaultBitcoinPubkeysAvailable =>
					Error::<T>::NoVaultBitcoinPubkeysAvailable,
				VaultError::UnableToGenerateVaultBitcoinPubkey =>
					Error::<T>::UnableToGenerateVaultBitcoinPubkey,
				VaultError::VaultNotYetActive => Error::<T>::VaultNotYetActive,

				e => Error::<T>::GenericVaultError(e),
			}
		}
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// The minimum number of satoshis accepted in one watched funding UTXO.
		pub minimum_bitcoin_lock_satoshis: Satoshis,
		#[serde(skip)]
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			MinimumSatoshis::<T>::put(self.minimum_bitcoin_lock_satoshis);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let (start_bitcoin_height, bitcoin_block_height) = T::BitcoinBlockHeightChange::get();
			let synched_bitcoin_height = T::BitcoinUtxoTracker::get_synched_height();
			let mut last_pending_funding_expiration =
				LastPendingFundingExpirationHeight::<T>::get();
			let pending_funding_start = last_pending_funding_expiration
				.map(|height| height.saturating_add(1))
				.unwrap_or(start_bitcoin_height);
			let mut pending_funding_count = 0u64;
			for expiration_height in pending_funding_start..=synched_bitcoin_height {
				let expirations = LocksPendingFundingByBitcoinHeight::<T>::take(expiration_height);
				let (expired_count, has_failed_expiration) =
					Self::process_pending_funding_expirations(expirations, expiration_height);
				pending_funding_count = pending_funding_count.saturating_add(expired_count);
				if has_failed_expiration {
					break
				}
				last_pending_funding_expiration = Some(expiration_height);
			}
			if let Some(expiration_height) = last_pending_funding_expiration {
				LastPendingFundingExpirationHeight::<T>::put(expiration_height);
			}

			let expirations = (start_bitcoin_height..=bitcoin_block_height)
				.flat_map(LockExpirationsByBitcoinHeight::<T>::take);
			let expiring_count = Self::process_expiring_locks(expirations);

			let overdue = LockCosignDueByFrame::<T>::take(T::CurrentFrameId::get());
			let overdue_count = Self::process_overdue_releases(overdue);

			let expiring = OrphanedUtxoExpirationByFrame::<T>::take(T::CurrentFrameId::get());
			let orphan_expiring_count = Self::process_orphaned_utxo_expirations(expiring);

			T::WeightInfo::on_initialize_with_expirations_and_overdue(
				expiring_count.min(u32::MAX as u64) as u32,
				overdue_count.min(u32::MAX as u64) as u32,
				orphan_expiring_count.min(u32::MAX as u64) as u32,
				pending_funding_count.min(u32::MAX as u64) as u32,
			)
			.saturating_add(T::DbWeight::get().reads(1))
		}

		fn on_finalize(_n: BlockNumberFor<T>) {
			let current_tick = T::CurrentTick::get();
			let oldest_allowed_tick =
				current_tick.saturating_sub(T::MaxBtcPriceTickAge::get() as Tick);
			let current_target_price =
				T::PriceProvider::get_btc_price_in_target_microgons(SATOSHIS_PER_BITCOIN);

			MicrogonsAtTargetPerBtcHistory::<T>::mutate(|history| {
				history.retain(|y| y.0 >= oldest_allowed_tick);

				if let Some(target_price) = current_target_price {
					let mut should_insert = true;
					if let Some((_, last_price)) = history.last() {
						should_insert = *last_price != target_price;
					}
					if should_insert {
						_ = history.try_push((current_tick, target_price));
					}
				}
			});
		}
	}

	#[derive(
		Decode,
		Encode,
		DecodeWithMemTracking,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		DebugNoBound,
		TypeInfo,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct FeeCoupon<T: Config> {
		/// Maximum amount deducted from the vault's Bitcoin lock fee.
		#[codec(compact)]
		pub fee_discount: T::Balance,
		/// Aggregate vault securitization space released atomically with this operation.
		#[codec(compact)]
		pub securitization_space_to_unreserve: T::Balance,
		/// Last frame in which this coupon can be used.
		#[codec(compact)]
		pub expires_at_frame: FrameId,
		/// Monotonically increasing value preventing replay for this vault and beneficiary.
		#[codec(compact)]
		pub nonce: u64,
		/// Vault delegate signature over the coupon and operation terms.
		pub signature: T::FeeCouponSignature,
	}

	pub const FEE_COUPON_MESSAGE_KEY: &[u8] = b"bitcoin_lock_fee_coupon";

	impl<T: Config> FeeCoupon<T> {
		pub fn verify(
			&self,
			signer: &T::AccountId,
			vault_id: VaultId,
			beneficiary: &T::AccountId,
			utxo_id: Option<UtxoId>,
			satoshis: Satoshis,
			microgons_at_target_per_btc: T::Balance,
		) -> bool {
			// FRAME preserves block zero and uses this lookup for CheckGenesis.
			let message = (
				FEE_COUPON_MESSAGE_KEY,
				frame_system::Pallet::<T>::block_hash(BlockNumberFor::<T>::zero()),
				vault_id,
				beneficiary,
				utxo_id,
				satoshis,
				microgons_at_target_per_btc,
				self.fee_discount,
				self.securitization_space_to_unreserve,
				self.expires_at_frame,
				self.nonce,
			)
				.using_encoded(blake2_256);
			let verified = self.signature.verify(message.as_slice(), signer);
			#[cfg(feature = "runtime-benchmarks")]
			{
				let _ = verified;
				true
			}
			#[cfg(not(feature = "runtime-benchmarks"))]
			{
				verified
			}
		}
	}

	#[derive(
		Decode,
		Encode,
		DecodeWithMemTracking,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		DebugNoBound,
		TypeInfo,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct LockOptions<T: Config> {
		/// The microgon value per BTC if Argon were trading at its target price.
		#[codec(compact)]
		pub microgons_at_target_per_btc: T::Balance,
		/// Optional Vault-delegate authorization for the lock terms and fixed fee discount.
		pub fee_coupon: Option<FeeCoupon<T>>,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a Bitcoin receive address backed by a Lock for the submitting account.
		///
		/// The pubkey submitted here will be used to create a script pubkey that will be used in a
		/// timelock multisig script to lock the bitcoin.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_receive_address())]
		pub fn create_receive_address(
			origin: OriginFor<T>,
			vault_id: VaultId,
			#[pallet::compact] satoshis: Satoshis,
			bitcoin_pubkey: CompressedBitcoinPubkey,
			options: Option<LockOptions<T>>,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			let fee_coupon =
				Self::validate_fee_coupon(vault_id, &account_id, None, satoshis, options.as_ref())?;
			let securitization_request = ReserveSecuritizationRequest {
				fee_discount: fee_coupon
					.map(|coupon| coupon.fee_discount)
					.unwrap_or_else(T::Balance::zero),
				securitization_space_to_unreserve: fee_coupon
					.map(|coupon| coupon.securitization_space_to_unreserve)
					.unwrap_or_else(T::Balance::zero),
			};
			let coupon_nonce = fee_coupon.map(|coupon| coupon.nonce);
			Self::create_bitcoin_lock(
				&account_id,
				vault_id,
				satoshis,
				bitcoin_pubkey,
				options,
				securitization_request,
			)?;
			if let Some(coupon_nonce) = coupon_nonce {
				LastFeeCouponNonceByVaultAndAccount::<T>::insert(
					vault_id,
					account_id,
					coupon_nonce,
				);
			}
			Ok(())
		}

		/// Submitted by a Bitcoin holder to trigger the release of their Utxo out of the cosign
		/// script. A transaction spending the UTXO should be pre-created so that the sighash
		/// can be submitted here. The vault operator will have 10 days to counter-sign the
		/// transaction. It will be published with the public key as a BitcoinUtxoCosigned Event.
		///
		/// Owner must submit a script pubkey and also a fee to pay to the bitcoin network.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::request_release())]
		pub fn request_release(
			origin: OriginFor<T>,
			utxo_id: UtxoId,
			to_script_pubkey: BitcoinScriptPubkey,
			bitcoin_network_fee: Satoshis,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			let vault_id = lock.vault_id;
			ensure!(lock.owner_account == who, Error::<T>::NoPermissions);

			// if no refund is needed, we can just cancel the lock
			if !lock.is_funded() {
				Self::cancel_lock(utxo_id, &lock)?;
				return Ok(());
			}
			ensure!(lock.fissioned_satoshis == 0, Error::<T>::LockHasActiveFissions);

			ensure!(
				!LockReleaseRequestsByUtxoId::<T>::contains_key(utxo_id),
				Error::<T>::LockInProcessOfRelease
			);

			// The user must request a co-sign 10 days before the vault can claim on bitcoin to give
			// them enough time to react. At the time of claim height, the utxo is claimable on the
			// bitcoin network, so this time frame must be "inside" the claim height
			// NOTE: we are losing a little cosign time here since we are rounding up to 10 entire
			// frames
			let ticks_until_cosign_overdue =
				T::LockReleaseCosignDeadlineFrames::get() * T::ArgonTicksPerDay::get();

			let safe_bitcoin_blocks_remaining =
				lock.vault_claim_height.saturating_sub(T::BitcoinBlockHeightChange::get().1);
			let ticks_until_vault_claim =
				safe_bitcoin_blocks_remaining.saturating_mul(T::TicksPerBitcoinBlock::get());
			ensure!(
				ticks_until_cosign_overdue < ticks_until_vault_claim,
				Error::<T>::BitcoinReleaseInitiationDeadlinePassed
			);

			ensure!(bitcoin_network_fee < lock.funded_satoshis, Error::<T>::BitcoinFeeTooHigh);
			let securitization_at_risk = Self::calculate_redemption_amount_from_satoshis(
				&lock.funded_satoshis,
				Some(lock.btc_value_in_microgons()),
			)?;

			let cosign_due_frame =
				T::LockReleaseCosignDeadlineFrames::get() + T::CurrentFrameId::get();
			LockReleaseRequestsByUtxoId::<T>::insert(
				utxo_id,
				LockReleaseRequest {
					utxo_id,
					vault_id,
					bitcoin_network_fee,
					cosign_due_frame,
					to_script_pubkey,
					securitization_at_risk,
				},
			);

			LockCosignDueByFrame::<T>::try_mutate(cosign_due_frame, |a| a.try_insert(utxo_id))
				.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;
			T::VaultProvider::update_pending_cosign_list(vault_id, utxo_id, false)
				.map_err(Error::<T>::from)?;

			Self::deposit_event(Event::<T>::BitcoinUtxoCosignRequested { utxo_id, vault_id });
			Ok(())
		}

		/// Submitted by a Vault operator to cosign the release of a bitcoin UTXO. The Lock's
		/// securitization will be scheduled for release without a penalty.
		///
		/// This is submitted as a no-fee transaction off chain to allow keys to remain in cold
		/// wallets.
		#[pallet::call_index(2)]
		#[pallet::weight((T::WeightInfo::cosign_release(), DispatchClass::Operational))]
		#[pallet::feeless_if(|origin: &OriginFor<T>, utxo_id: &UtxoId, _signature: &BitcoinSignature| -> bool {
			let Ok(who) = ensure_signed(origin.clone()) else {
				return false;
			};
			if let Some(lock) = LocksByUtxoId::<T>::get(utxo_id) {
				return T::VaultProvider::is_owner(lock.vault_id, &who)
			}
			false
		})]
		#[allow(clippy::useless_conversion)]
		pub fn cosign_release(
			origin: OriginFor<T>,
			utxo_id: UtxoId,
			signature: BitcoinSignature,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let lock = LocksByUtxoId::<T>::take(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			UtxoIdsByVaultId::<T>::remove(lock.vault_id, utxo_id);
			UtxoIdsByOwnerAccount::<T>::remove(&lock.owner_account, utxo_id);
			let lock_extension = lock.get_lock_extension();
			let funded_satoshis = lock.funded_satoshis;
			let securitization = lock.get_securitization();
			let vault_id = lock.vault_id;
			let vault_pubkey = lock.vault_pubkey;

			ensure!(T::VaultProvider::is_owner(vault_id, &who), Error::<T>::NoPermissions);
			let request = Self::take_release_request(utxo_id)?;
			let bitcoin_network_fee = request.bitcoin_network_fee;
			let to_script_pubkey = request.to_script_pubkey.clone();

			let utxo_ref = UtxoIdToFundingUtxoRef::<T>::take(utxo_id)
				.ok_or(Error::<T>::BitcoinUtxoNotFound)?;

			let script_args = CosignScriptArgs {
				vault_pubkey,
				owner_pubkey: lock.owner_pubkey,
				vault_claim_pubkey: lock.vault_claim_pubkey,
				created_at_height: lock.created_at_height,
				vault_claim_height: lock.vault_claim_height,
				open_claim_height: lock.open_claim_height,
			};
			let releaser = CosignReleaser::new(
				script_args,
				funded_satoshis,
				utxo_ref.txid.into(),
				utxo_ref.output_index,
				ReleaseStep::VaultCosign,
				Amount::from_sat(bitcoin_network_fee),
				to_script_pubkey.into(),
				T::GetBitcoinNetwork::get().into(),
			)
			.map_err(|_| Error::<T>::BitcoinUnableToBeDecodedForRelease)?;

			let is_valid =
				T::BitcoinSignatureVerifier::verify_signature(releaser, vault_pubkey, &signature)?;
			ensure!(is_valid, Error::<T>::BitcoinInvalidCosignature);

			Self::finalize_release_request(
				utxo_id,
				lock,
				vault_id,
				&securitization,
				&lock_extension,
			)?;
			LockReleaseCosignHeightById::<T>::insert(
				utxo_id,
				frame_system::Pallet::<T>::block_number(),
			);

			Self::deposit_event(Event::BitcoinUtxoCosigned { utxo_id, vault_id, signature });

			// no fee for cosigning
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::admin_modify_minimum_locked_sats())]
		pub fn admin_modify_minimum_locked_sats(
			origin: OriginFor<T>,
			satoshis: Satoshis,
		) -> DispatchResult {
			ensure_root(origin)?;
			MinimumSatoshis::<T>::put(satoshis);
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::request_orphaned_utxo_release())]
		pub fn request_orphaned_utxo_release(
			origin: OriginFor<T>,
			utxo_ref: UtxoRef,
			to_script_pubkey: BitcoinScriptPubkey,
			bitcoin_network_fee: Satoshis,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			OrphanedUtxosByAccount::<T>::try_mutate(&who, utxo_ref.clone(), |entry_maybe| {
				let entry = entry_maybe.as_mut().ok_or(Error::<T>::BitcoinUtxoNotFound)?;
				ensure!(
					UtxoIdToFundingUtxoRef::<T>::get(entry.utxo_id) != Some(utxo_ref.clone()),
					Error::<T>::FundingUtxoCannotBeReleased
				);
				ensure!(entry.cosign_request.is_none(), Error::<T>::OrphanedUtxoReleaseRequested);

				entry.cosign_request = Some(OrphanedUtxoCosignRequest {
					bitcoin_network_fee,
					to_script_pubkey: to_script_pubkey.clone(),
					created_at_argon_block_number: <frame_system::Pallet<T>>::block_number(),
				});
				// send to a queue for vault to know about it
				T::VaultProvider::update_orphan_cosign_list(
					entry.vault_id,
					entry.utxo_id,
					&who,
					false,
				)
				.map_err(Error::<T>::from)?;

				Self::deposit_event(Event::OrphanedUtxoReleaseRequested {
					utxo_id: entry.utxo_id,
					utxo_ref: utxo_ref.clone(),
					vault_id: entry.vault_id,
					account_id: who.clone(),
				});
				Ok::<(), Error<T>>(())
			})?;

			Ok(())
		}

		/// NOTE: The `signature` parameter is NOT verified on-chain. The orphan record does
		/// not retain the cosign script args needed for verification (they are cleaned up
		/// with the BitcoinLock). The signature is passed through to the
		/// `OrphanedUtxoCosigned` event so the lock owner can construct the Bitcoin release
		/// transaction off-chain. A garbage signature means the owner can't spend — no
		/// on-chain damage. To add on-chain verification, the cosign script args would need
		/// to be stored in `OrphanedUtxo` (storage migration required).
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::cosign_orphaned_utxo_release())]
		pub fn cosign_orphaned_utxo_release(
			origin: OriginFor<T>,
			orphan_owner: T::AccountId,
			utxo_ref: UtxoRef,
			signature: BitcoinSignature,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let orphan = OrphanedUtxosByAccount::<T>::take(&orphan_owner, &utxo_ref)
				.ok_or(Error::<T>::BitcoinUtxoNotFound)?;
			let vault_id = orphan.vault_id;
			let utxo_id = orphan.utxo_id;
			// if not owner, "take" of ref will rollback
			ensure!(T::VaultProvider::is_owner(vault_id, &who), Error::<T>::NoPermissions);
			T::VaultProvider::update_orphan_cosign_list(vault_id, utxo_id, &orphan_owner, true)
				.map_err(Error::<T>::from)?;
			T::BitcoinUtxoTracker::unwatch_utxo(utxo_id, &utxo_ref);
			Self::deposit_event(Event::OrphanedUtxoCosigned {
				utxo_id,
				vault_id,
				utxo_ref: utxo_ref.clone(),
				account_id: orphan_owner,
				signature,
			});

			Ok(())
		}

		/// Replace this Lock's BTC coverage and target value for its remaining term.
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::resecuritize())]
		pub fn resecuritize(
			origin: OriginFor<T>,
			utxo_id: UtxoId,
			#[pallet::compact] satoshis: Satoshis,
			options: Option<LockOptions<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			ensure!(lock.owner_account == who, Error::<T>::NoPermissions);
			ensure!(
				!LockReleaseRequestsByUtxoId::<T>::contains_key(utxo_id),
				Error::<T>::LockInProcessOfRelease
			);
			ensure!(
				satoshis >= lock.fissioned_satoshis,
				Error::<T>::InsufficientSatoshisForFissions
			);
			let (microgons_at_target_per_btc, microgons_at_target_per_btc_tick) =
				Self::resolve_microgons_at_target_per_btc(
					options.as_ref().map(|options| options.microgons_at_target_per_btc),
				)?;
			let fee_coupon = Self::validate_fee_coupon(
				lock.vault_id,
				&who,
				Some(utxo_id),
				satoshis,
				options.as_ref(),
			)?;
			let securitization_request = ReserveSecuritizationRequest {
				fee_discount: fee_coupon
					.map(|coupon| coupon.fee_discount)
					.unwrap_or_else(T::Balance::zero),
				securitization_space_to_unreserve: fee_coupon
					.map(|coupon| coupon.securitization_space_to_unreserve)
					.unwrap_or_else(T::Balance::zero),
			};
			let coupon_nonce = fee_coupon.map(|coupon| coupon.nonce);
			ensure!(
				satoshis != lock.securitized_satoshis ||
					microgons_at_target_per_btc != lock.microgons_at_target_per_btc,
				Error::<T>::NoResecuritizationChange
			);
			ensure!(
				microgons_at_target_per_btc_tick >= lock.securitization_tick,
				Error::<T>::MicrogonsAtTargetPerBtcTickOlderThanCurrent
			);
			let fission_requirements =
				T::FissionsProvider::get_lock_fission_requirements(&who, utxo_id);
			ensure!(
				lock.fissioned_satoshis == 0 || fission_requirements.is_some(),
				Error::<T>::FissionStateMismatch
			);
			let btc_value_in_microgons =
				FixedU128::from_rational(satoshis as u128, SATOSHIS_PER_BITCOIN as u128)
					.saturating_mul_int(microgons_at_target_per_btc);
			let securitization_coverage_microgons =
				Self::calculate_redemption_amount(btc_value_in_microgons, None)?;
			let replacement_securitization = BitcoinSecuritization {
				securitized_satoshis: satoshis,
				microgons_at_target_per_btc,
				securitization_coverage_microgons,
				securitization_ratio: lock.securitization_ratio,
			};
			if let Some(requirements) = fission_requirements {
				ensure!(
					microgons_at_target_per_btc_tick >= requirements.last_ratchet_tick,
					Error::<T>::MicrogonsAtTargetPerBtcTickOlderThanCurrent
				);
				ensure!(
					microgons_at_target_per_btc >= requirements.microgons_at_target_per_btc &&
						securitization_coverage_microgons >= requirements.liquidity_promised,
					Error::<T>::InsufficientSecuritizationForFissions
				);
			}

			let current_bitcoin_height = T::BitcoinBlockHeightChange::get().1;
			let elapsed_blocks = current_bitcoin_height.saturating_sub(lock.created_at_height);
			let full_term = lock.vault_claim_height.saturating_sub(lock.created_at_height).max(1);
			let remaining_blocks = full_term.saturating_sub(elapsed_blocks);
			let remaining_term =
				FixedU128::from_rational(remaining_blocks as u128, full_term as u128);
			let current_securitization = lock.get_securitization();
			let mut lock_extension = lock.get_lock_extension();
			let (fee, coupon_paid_fees) = T::VaultProvider::resecuritize(
				lock.vault_id,
				&who,
				BitcoinResecuritization {
					current: &current_securitization,
					replacement: &replacement_securitization,
					funded_satoshis: lock.funded_satoshis,
					remaining_term,
					lock_extension: &mut lock_extension,
					is_flexible: lock.is_flexible,
					fee_discount: securitization_request.fee_discount,
					securitization_space_to_unreserve: securitization_request
						.securitization_space_to_unreserve,
				},
			)
			.map_err(Error::<T>::from)?;
			if !lock.is_funded() {
				Self::unschedule_pending_funding(utxo_id, lock.funding_expiration_height);
				let funding_expiration_height =
					Self::pending_funding_expiration_height(current_bitcoin_height);
				LocksPendingFundingByBitcoinHeight::<T>::try_mutate(
					funding_expiration_height,
					|locks| locks.try_insert(utxo_id).map(|_| ()),
				)
				.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;
				lock.funding_expiration_height = funding_expiration_height;
			}
			lock.security_fees.saturating_accrue(fee);
			lock.coupon_paid_fees.saturating_accrue(coupon_paid_fees);
			lock.fund_hold_extensions = lock_extension.extended_expiration_funds;
			lock.securitized_satoshis = satoshis;
			lock.microgons_at_target_per_btc = microgons_at_target_per_btc;
			lock.securitization_coverage_microgons = securitization_coverage_microgons;
			lock.securitization_tick = microgons_at_target_per_btc_tick;
			let vault_id = lock.vault_id;
			LocksByUtxoId::<T>::insert(utxo_id, lock);
			if let Some(coupon_nonce) = coupon_nonce {
				LastFeeCouponNonceByVaultAndAccount::<T>::insert(vault_id, &who, coupon_nonce);
			}
			Self::deposit_event(Event::BitcoinLockResecuritized {
				utxo_id,
				vault_id,
				securitized_satoshis: satoshis,
				microgons_at_target_per_btc,
				account_id: who,
			});
			Ok(())
		}

		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::set_flexible())]
		pub fn set_flexible(
			origin: OriginFor<T>,
			utxo_id: UtxoId,
			is_flexible: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			ensure!(lock.owner_account == who, Error::<T>::NoPermissions);
			let operator = T::VaultProvider::get_vault_operator(lock.vault_id)
				.ok_or(Error::<T>::VaultNotFound)?;
			ensure!(operator == who, Error::<T>::NoPermissions);
			ensure!(lock.is_funded(), Error::<T>::LockPendingFunding);
			ensure!(
				!LockReleaseRequestsByUtxoId::<T>::contains_key(utxo_id),
				Error::<T>::LockInProcessOfRelease
			);
			if lock.is_flexible == is_flexible {
				return Ok(());
			}

			T::VaultProvider::set_bitcoin_lock_flexible(
				lock.vault_id,
				&lock.get_securitization(),
				lock.funded_satoshis,
				is_flexible,
			)
			.map_err(Error::<T>::from)?;
			lock.is_flexible = is_flexible;
			let vault_id = lock.vault_id;
			LocksByUtxoId::<T>::insert(utxo_id, lock);
			Self::deposit_event(Event::BitcoinLockFlexibleChanged {
				utxo_id,
				vault_id,
				is_flexible,
			});
			Ok(())
		}
	}

	impl<T: Config> BitcoinUtxoEvents<T::AccountId> for Pallet<T> {
		type Weights = ProviderWeightAdapter<T>;

		fn utxo_detected(
			utxo_id: UtxoId,
			utxo_ref: UtxoRef,
			funded_satoshis: Satoshis,
			_bitcoin_height: BitcoinHeight,
		) -> DispatchResult {
			let mut lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			if lock.is_funded() {
				return Self::orphaned_utxo_detected(utxo_id, funded_satoshis, utxo_ref);
			}
			if LastPendingFundingExpirationHeight::<T>::get()
				.is_some_and(|height| lock.funding_expiration_height <= height)
			{
				return Self::orphaned_utxo_detected(utxo_id, funded_satoshis, utxo_ref);
			}
			Self::unschedule_pending_funding(utxo_id, lock.funding_expiration_height);
			UtxoIdToFundingUtxoRef::<T>::insert(utxo_id, utxo_ref);

			let securitization = lock.get_securitization();
			lock.funded_satoshis = funded_satoshis;
			T::VaultProvider::activate_securitization(
				lock.vault_id,
				&securitization,
				funded_satoshis,
			)
			.map_err(Error::<T>::from)?;
			LocksByUtxoId::<T>::insert(utxo_id, lock);
			Ok(())
		}

		fn spent(utxo_id: UtxoId, utxo_ref: UtxoRef) -> DispatchResult {
			let Some(lock) = LocksByUtxoId::<T>::get(utxo_id) else {
				T::BitcoinUtxoTracker::unwatch(utxo_id);
				return Ok(());
			};
			if UtxoIdToFundingUtxoRef::<T>::get(utxo_id).as_ref() != Some(&utxo_ref) {
				if let Some(orphan) =
					OrphanedUtxosByAccount::<T>::take(&lock.owner_account, &utxo_ref) &&
					orphan.cosign_request.is_some()
				{
					T::VaultProvider::update_orphan_cosign_list(
						orphan.vault_id,
						orphan.utxo_id,
						&lock.owner_account,
						true,
					)
					.map_err(Error::<T>::from)?;
				}
				T::BitcoinUtxoTracker::unwatch_utxo(utxo_id, &utxo_ref);
				return Ok(());
			}
			if LockReleaseRequestsByUtxoId::<T>::contains_key(utxo_id) {
				return Self::complete_release_after_spent(utxo_id);
			}
			Self::burn_bitcoin_lock(utxo_id, true)
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: Codec,
	{
		pub fn orphaned_utxo_detected(
			utxo_id: UtxoId,
			satoshis: Satoshis,
			utxo_ref: UtxoRef,
		) -> DispatchResult {
			let block_number = frame_system::Pallet::<T>::block_number();
			let lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			let did_modify = OrphanedUtxosByAccount::<T>::mutate(
				lock.owner_account.clone(),
				&utxo_ref,
				|entry| {
					if entry.is_some() {
						// Avoid overwriting a pending cosign request if the UTXO is re-reported.
						return false;
					}
					*entry = Some(OrphanedUtxo {
						utxo_id,
						vault_id: lock.vault_id,
						satoshis,
						recorded_argon_block_number: block_number,
						cosign_request: None,
					});
					true
				},
			);
			if did_modify {
				Self::deposit_event(Event::OrphanedUtxoReceived {
					utxo_id,
					utxo_ref,
					vault_id: lock.vault_id,
					satoshis,
				});
			}
			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: Codec,
	{
		fn validate_fee_coupon<'a>(
			vault_id: VaultId,
			account_id: &T::AccountId,
			utxo_id: Option<UtxoId>,
			satoshis: Satoshis,
			options: Option<&'a LockOptions<T>>,
		) -> Result<Option<&'a FeeCoupon<T>>, Error<T>> {
			let Some(options) = options else { return Ok(None) };
			let Some(coupon) = options.fee_coupon.as_ref() else { return Ok(None) };

			ensure!(
				T::CurrentFrameId::get() <= coupon.expires_at_frame,
				Error::<T>::FeeCouponExpired
			);
			let delegate = T::VaultProvider::get_vault_delegate(vault_id)
				.ok_or(Error::<T>::InvalidFeeCouponSignature)?;
			ensure!(
				coupon.verify(
					&delegate,
					vault_id,
					account_id,
					utxo_id,
					satoshis,
					options.microgons_at_target_per_btc,
				),
				Error::<T>::InvalidFeeCouponSignature
			);
			let next_nonce = LastFeeCouponNonceByVaultAndAccount::<T>::get(vault_id, account_id)
				.unwrap_or_default()
				.checked_add(1);
			ensure!(next_nonce == Some(coupon.nonce), Error::<T>::FeeCouponAlreadyUsed);

			Ok(Some(coupon))
		}

		fn create_bitcoin_lock(
			account_id: &<T as frame_system::Config>::AccountId,
			vault_id: VaultId,
			satoshis: Satoshis,
			bitcoin_pubkey: CompressedBitcoinPubkey,
			options: Option<LockOptions<T>>,
			securitization_request: ReserveSecuritizationRequest<T::Balance>,
		) -> DispatchResult {
			let current_bitcoin_height = T::BitcoinBlockHeightChange::get().1;
			let vault_claim_height =
				current_bitcoin_height.saturating_add(T::LockDurationBlocks::get());
			let open_claim_height =
				vault_claim_height.saturating_add(T::LockReclamationBlocks::get());

			let (securitization, securitization_tick) =
				Self::prepare_lock_securitization(vault_id, satoshis, options.as_ref())?;

			let (fee, coupon_paid_fees) = T::VaultProvider::reserve_securitization(
				vault_id,
				account_id,
				&securitization,
				securitization_request,
			)
			.map_err(Error::<T>::from)?;

			let (vault_xpub, vault_claim_xpub, script_pubkey) =
				T::VaultProvider::create_utxo_script_pubkey(
					vault_id,
					bitcoin_pubkey,
					vault_claim_height,
					open_claim_height,
					current_bitcoin_height,
				)
				.map_err(|_| Error::<T>::InvalidBitcoinScript)?;

			let vault_pubkey = vault_xpub.public_key;
			let vault_claim_pubkey = vault_claim_xpub.public_key;
			let vault_xpub_sources = (
				vault_xpub.parent_fingerprint,
				vault_xpub.child_number,
				vault_claim_xpub.child_number,
			);

			let utxo_id = NextUtxoId::<T>::mutate(|a| {
				let next = a.unwrap_or_default() + 1;
				*a = Some(next);
				next
			});
			LockExpirationsByBitcoinHeight::<T>::mutate(vault_claim_height, |x| {
				x.try_insert(utxo_id)
			})
			.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;
			let funding_expiration_height = current_bitcoin_height
				.saturating_add(T::MaxPendingConfirmationBlocks::get())
				.saturating_add(1);
			LocksPendingFundingByBitcoinHeight::<T>::mutate(funding_expiration_height, |locks| {
				locks.try_insert(utxo_id)
			})
			.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;

			T::BitcoinUtxoTracker::watch_for_utxo(utxo_id, script_pubkey)?;

			LocksByUtxoId::<T>::insert(
				utxo_id,
				LockedBitcoin {
					owner_account: account_id.clone(),
					vault_id,
					securitized_satoshis: satoshis,
					microgons_at_target_per_btc: securitization.microgons_at_target_per_btc,
					securitization_coverage_microgons: securitization
						.securitization_coverage_microgons,
					securitization_tick,
					funded_satoshis: 0,
					fissioned_satoshis: 0,
					security_fees: fee,
					securitization_ratio: securitization.securitization_ratio,
					coupon_paid_fees,
					vault_pubkey,
					vault_claim_pubkey,
					vault_xpub_sources,
					owner_pubkey: bitcoin_pubkey,
					vault_claim_height,
					open_claim_height,
					created_at_height: current_bitcoin_height,
					funding_expiration_height,
					utxo_script_pubkey: script_pubkey,
					is_flexible: false,
					fund_hold_extensions: BoundedBTreeMap::default(),
					created_at_argon_block: <frame_system::Pallet<T>>::block_number(),
				},
			);
			UtxoIdsByVaultId::<T>::insert(vault_id, utxo_id, ());
			UtxoIdsByOwnerAccount::<T>::insert(account_id, utxo_id, ());
			Self::deposit_event(Event::<T>::BitcoinLockCreated {
				utxo_id,
				vault_id,
				securitized_satoshis: satoshis,
				microgons_at_target_per_btc: securitization.microgons_at_target_per_btc,
				collateral_required: securitization.collateral_required(),
				account_id: account_id.clone(),
				security_fee: fee,
			});

			Ok(())
		}

		fn prepare_lock_securitization(
			vault_id: VaultId,
			satoshis: Satoshis,
			options: Option<&LockOptions<T>>,
		) -> Result<(BitcoinSecuritization<T::Balance>, Tick), Error<T>> {
			let (microgons_at_target_per_btc, securitization_tick) =
				Self::resolve_microgons_at_target_per_btc(
					options.map(|options| options.microgons_at_target_per_btc),
				)?;
			let securitization_ratio =
				T::VaultProvider::get_securitization_ratio(vault_id).map_err(Error::<T>::from)?;
			let btc_value_in_microgons =
				FixedU128::from_rational(satoshis as u128, SATOSHIS_PER_BITCOIN as u128)
					.saturating_mul_int(microgons_at_target_per_btc);
			let securitization_coverage_microgons =
				Self::calculate_redemption_amount(btc_value_in_microgons, None)?;

			Ok((
				BitcoinSecuritization {
					securitized_satoshis: satoshis,
					microgons_at_target_per_btc,
					securitization_coverage_microgons,
					securitization_ratio,
				},
				securitization_tick,
			))
		}

		fn resolve_microgons_at_target_per_btc(
			microgons_at_target_per_btc: Option<T::Balance>,
		) -> Result<(T::Balance, Tick), Error<T>> {
			if let Some(microgons_at_target_per_btc) = microgons_at_target_per_btc {
				let tick =
					Self::latest_microgons_at_target_per_btc_tick(microgons_at_target_per_btc)
						.ok_or(Error::<T>::IneligibleMicrogonsAtTargetPerBtcRequested)?;
				return Ok((microgons_at_target_per_btc, tick));
			}

			let microgons_at_target_per_btc =
				T::PriceProvider::get_btc_price_in_target_microgons(SATOSHIS_PER_BITCOIN)
					.ok_or(Error::<T>::NoBitcoinPricesAvailable)?;
			let tick = Self::latest_microgons_at_target_per_btc_tick(microgons_at_target_per_btc)
				.unwrap_or_else(T::CurrentTick::get);
			Ok((microgons_at_target_per_btc, tick))
		}

		pub(crate) fn latest_microgons_at_target_per_btc_tick(
			microgons_at_target_per_btc: T::Balance,
		) -> Option<Tick> {
			MicrogonsAtTargetPerBtcHistory::<T>::get()
				.iter()
				.rev()
				.find_map(|(tick, value)| (*value == microgons_at_target_per_btc).then_some(*tick))
		}

		pub fn minimum_satoshis() -> Satoshis {
			MinimumSatoshis::<T>::get()
		}

		pub(crate) fn process_expiring_locks(expirations: impl IntoIterator<Item = UtxoId>) -> u64 {
			let mut expiring_count: u64 = 0;
			for utxo_id in expirations {
				expiring_count = expiring_count.saturating_add(1);
				let res = with_storage_layer(|| {
					Self::burn_bitcoin_lock(utxo_id, false)?;
					Ok(())
				});
				if let Err(e) = res {
					log::error!("Bitcoin utxo id {utxo_id:?} failed to be burned {e:?}");
					Self::deposit_event(Event::<T>::LockExpirationError { utxo_id, error: e });
				}
			}
			expiring_count
		}

		pub(crate) fn process_overdue_releases(overdue: impl IntoIterator<Item = UtxoId>) -> u64 {
			let mut overdue_count: u64 = 0;
			for utxo_id in overdue {
				overdue_count = overdue_count.saturating_add(1);
				let res = with_storage_layer(|| Self::cosign_bitcoin_overdue(utxo_id));
				if let Err(e) = res {
					log::error!(
						"Bitcoin lock id {utxo_id:?} failed to handle overdue `cosign` {e:?}"
					);
					Self::deposit_event(Event::<T>::CosignOverdueError { utxo_id, error: e });
				}
			}
			overdue_count
		}

		pub(crate) fn process_orphaned_utxo_expirations(
			expiring: impl IntoIterator<Item = (T::AccountId, UtxoRef)>,
		) -> u64 {
			let mut orphan_expiring_count: u64 = 0;
			for (account_id, utxo_ref) in expiring {
				orphan_expiring_count = orphan_expiring_count.saturating_add(1);
				let res: Result<(), DispatchError> = with_storage_layer(|| {
					if let Some(request) = OrphanedUtxosByAccount::<T>::take(&account_id, &utxo_ref) &&
						request.cosign_request.is_some()
					{
						T::VaultProvider::update_orphan_cosign_list(
							request.vault_id,
							request.utxo_id,
							&account_id,
							true,
						)
						.map_err(Error::<T>::from)?;
					}
					Ok::<(), DispatchError>(())
				});
				if let Err(e) = res {
					log::error!("Orphaned bitcoin utxo {utxo_ref:?} failed expiry cleanup {e:?}");
					Self::deposit_event(Event::OrphanedUtxoExpirationError {
						account_id,
						utxo_ref,
						error: e,
					});
				}
			}
			orphan_expiring_count
		}

		fn burn_bitcoin_lock(utxo_id: UtxoId, is_externally_spent: bool) -> DispatchResult {
			let lock = LocksByUtxoId::<T>::take(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			UtxoIdsByVaultId::<T>::remove(lock.vault_id, utxo_id);
			UtxoIdsByOwnerAccount::<T>::remove(&lock.owner_account, utxo_id);
			UtxoIdToFundingUtxoRef::<T>::remove(utxo_id);
			if LockReleaseRequestsByUtxoId::<T>::contains_key(utxo_id) {
				Self::take_release_request(utxo_id)?;
				// Version 10 compatibility: retire the owner's release hold on this terminal path.
				if let Some(release_hold) = MigratedReleaseHoldByUtxoId::<T>::take(utxo_id) {
					T::Currency::burn_held(
						&HoldReason::ReleaseBitcoinLock.into(),
						&lock.owner_account,
						release_hold,
						Precision::Exact,
						Fortitude::Force,
					)?;
					frame_system::Pallet::<T>::dec_providers(&lock.owner_account)?;
				}
			}
			if is_externally_spent {
				Self::clear_orphans_for_lock(utxo_id, &lock)?;
			} else {
				Self::schedule_orphans_for_cleanup(utxo_id, &lock);
			}
			T::BitcoinUtxoTracker::unwatch(utxo_id);

			if !lock.is_funded() {
				T::VaultProvider::return_securitization(lock.vault_id, &lock.get_securitization())
					.map_err(Error::<T>::from)?;
				return Ok(());
			}

			// burn the current redemption price from the vault at value of actual satoshis locked
			let redemption_amount = Self::calculate_redemption_amount_from_satoshis(
				&lock.funded_satoshis,
				Some(lock.btc_value_in_microgons()),
			)?;

			let burned_argons = T::VaultProvider::burn(
				lock.vault_id,
				&lock.get_securitization(),
				lock.funded_satoshis,
				redemption_amount,
				&lock.get_lock_extension(),
				lock.is_flexible,
			)
			.map_err(Error::<T>::from)?;
			T::FissionsProvider::close_for_lock(&lock.owner_account, utxo_id, burned_argons)?;

			Self::deposit_event(Event::BitcoinLockBurned {
				utxo_id,
				vault_id: lock.vault_id,
				was_utxo_spent: is_externally_spent,
			});

			Ok(())
		}

		fn complete_release_after_spent(utxo_id: UtxoId) -> DispatchResult {
			let lock = LocksByUtxoId::<T>::take(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			UtxoIdsByVaultId::<T>::remove(lock.vault_id, utxo_id);
			UtxoIdsByOwnerAccount::<T>::remove(&lock.owner_account, utxo_id);
			UtxoIdToFundingUtxoRef::<T>::remove(utxo_id);
			let vault_id = lock.vault_id;
			let securitization = lock.get_securitization();
			let lock_extension = lock.get_lock_extension();
			Self::take_release_request(utxo_id)?;

			Self::finalize_release_request(
				utxo_id,
				lock,
				vault_id,
				&securitization,
				&lock_extension,
			)?;
			Self::deposit_event(Event::BitcoinSpentAfterRelease { utxo_id, vault_id });
			Ok(())
		}

		fn take_release_request(
			utxo_id: UtxoId,
		) -> Result<LockReleaseRequest<T::Balance>, Error<T>> {
			let request = LockReleaseRequestsByUtxoId::<T>::take(utxo_id)
				.ok_or(Error::<T>::RedemptionNotLocked)?;

			LockCosignDueByFrame::<T>::mutate(request.cosign_due_frame, |a| {
				a.remove(&utxo_id);
			});
			T::VaultProvider::update_pending_cosign_list(request.vault_id, utxo_id, true)?;
			Ok(request)
		}

		fn finalize_release_request(
			utxo_id: UtxoId,
			lock: LockedBitcoin<T>,
			vault_id: VaultId,
			securitization: &BitcoinSecuritization<T::Balance>,
			lock_extension: &LockExtension<T::Balance>,
		) -> DispatchResult {
			ensure!(lock.is_funded(), Error::<T>::LockPendingFunding);
			let mut burned_argons = T::Balance::zero();
			// Version 10 compatibility: retire the owner's release hold on this terminal path.
			if let Some(release_hold) = MigratedReleaseHoldByUtxoId::<T>::take(utxo_id) {
				T::Currency::burn_held(
					&HoldReason::ReleaseBitcoinLock.into(),
					&lock.owner_account,
					release_hold,
					Precision::Exact,
					Fortitude::Force,
				)?;
				frame_system::Pallet::<T>::dec_providers(&lock.owner_account)?;
				burned_argons = release_hold;
			}
			T::FissionsProvider::close_for_lock(&lock.owner_account, utxo_id, burned_argons)?;

			T::VaultProvider::schedule_securitization_release(
				vault_id,
				securitization,
				lock.funded_satoshis,
				lock_extension,
				lock.is_flexible,
			)
			.map_err(Error::<T>::from)?;

			Self::schedule_orphans_for_cleanup(utxo_id, &lock);
			T::BitcoinUtxoTracker::unwatch(utxo_id);
			Ok(())
		}

		/// Call made during the on_initialize to implement cosign overdue penalties.
		pub(crate) fn cosign_bitcoin_overdue(utxo_id: UtxoId) -> DispatchResult {
			let entry = Self::take_release_request(utxo_id)?;
			let Some(lock) = LocksByUtxoId::<T>::take(utxo_id) else {
				UtxoIdsByVaultId::<T>::remove(entry.vault_id, utxo_id);
				UtxoIdToFundingUtxoRef::<T>::remove(utxo_id);
				T::BitcoinUtxoTracker::unwatch(utxo_id);
				log::warn!(
					"Cleared overdue bitcoin cosign request for missing lock {utxo_id:?} in vault {:?}",
					entry.vault_id
				);
				return Ok(());
			};
			UtxoIdsByVaultId::<T>::remove(lock.vault_id, utxo_id);
			UtxoIdsByOwnerAccount::<T>::remove(&lock.owner_account, utxo_id);
			UtxoIdToFundingUtxoRef::<T>::remove(utxo_id);
			let vault_id = lock.vault_id;

			// Compensate the Bitcoin owner up to the securitization frozen when release began.
			let compensation = T::VaultProvider::compensate_lost_bitcoin(
				vault_id,
				&lock.owner_account,
				&lock.get_securitization(),
				lock.funded_satoshis,
				entry.securitization_at_risk,
				&lock.get_lock_extension(),
				lock.is_flexible,
			)
			.map_err(Error::<T>::from)?;
			// Version 10 compatibility: return the owner's release hold after vault compensation.
			if let Some(release_hold) = MigratedReleaseHoldByUtxoId::<T>::take(utxo_id) {
				T::Currency::release(
					&HoldReason::ReleaseBitcoinLock.into(),
					&lock.owner_account,
					release_hold,
					Precision::Exact,
				)?;
				frame_system::Pallet::<T>::dec_providers(&lock.owner_account)?;
			}
			T::FissionsProvider::close_for_lock(&lock.owner_account, utxo_id, compensation.burned)?;
			Self::deposit_event(Event::BitcoinCosignPastDue {
				utxo_id,
				vault_id,
				compensation_amount: compensation.to_beneficiary,
				compensated_account_id: lock.owner_account.clone(),
			});
			Self::schedule_orphans_for_cleanup(utxo_id, &lock);
			T::BitcoinUtxoTracker::unwatch(utxo_id);

			Ok(())
		}

		pub fn calculate_redemption_amount_from_satoshis(
			satoshis: &Satoshis,
			max_btc_value_in_microgons: Option<T::Balance>,
		) -> Result<T::Balance, Error<T>> {
			let btc_value_in_microgons =
				T::PriceProvider::get_btc_price_in_target_microgons(*satoshis)
					.ok_or(Error::<T>::NoBitcoinPricesAvailable)?;
			Self::calculate_redemption_amount(btc_value_in_microgons, max_btc_value_in_microgons)
		}

		pub(crate) fn calculate_redemption_amount(
			btc_value_in_microgons: T::Balance,
			max_btc_value_in_microgons: Option<T::Balance>,
		) -> Result<T::Balance, Error<T>> {
			let mut price = FixedU128::from_rational(btc_value_in_microgons.into(), 1u128);

			if let Some(max_microgons) = max_btc_value_in_microgons {
				price = price.min(FixedU128::from_rational(max_microgons.into(), 1u128));
			}

			let r = T::PriceProvider::get_redemption_r_value().unwrap_or(FixedU128::one());

			// Case 1: If argon is at or above target price, no penalty — unlock cost is just b.
			let multiplier = if r >= FixedU128::one() {
				FixedU128::one()
			}
			// Case 2: Mild deviation (0.90 ≤ r < 1) — apply quadratic curve to scale unlock cost.
			else if r >= FixedU128::from_rational(0_9, 1_0) {
				const FX_20: FixedU128 = FixedU128::from_u32(20);
				const FX_38: FixedU128 = FixedU128::from_u32(38);
				const FX_19: FixedU128 = FixedU128::from_u32(19);

				// Formula: b * (20r² - 38r + 19)
				((FX_20 * r.saturating_pow(2)) + FX_19)
					.ensure_sub(FX_38 * r)
					.map_err(|_| Error::<T>::OverflowError)?
			}
			// Case 3: Moderate deviation (0.01 ≤ r < 0.90) — apply rational linear formula.
			else if r >= FixedU128::from_rational(0_01, 1_00) {
				const FX_0_5618: FixedU128 = FixedU128::from_rational(0_5618, 1_0000);
				const FX_0_3944: FixedU128 = FixedU128::from_rational(0_3944, 1_0000);
				// Formula: b * ((0.5618r + 0.3944) / r)
				((FX_0_5618 * r) + FX_0_3944)
					.ensure_div(r)
					.map_err(|_| Error::<T>::OverflowError)?
			}
			// Case 4: Extreme deviation (r < 0.01) — maximize burn using an aggressive slope.
			else {
				const FX_0_576: FixedU128 = FixedU128::from_rational(0_576, 1_000);
				const FX_0_4: FixedU128 = FixedU128::from_rational(0_4, 1_0);
				// Formula: (b / r) * (0.576r + 0.4)
				FixedU128::from_u32(1).div(r).saturating_mul((FX_0_576 * r) + FX_0_4)
			};
			let price = price.saturating_mul(multiplier);
			// now scale to microgons
			let microgons = price.saturating_mul_int(T::Balance::one());
			Ok(microgons)
		}

		fn cancel_lock(utxo_id: UtxoId, lock: &LockedBitcoin<T>) -> DispatchResult {
			Self::unschedule_pending_funding(utxo_id, lock.funding_expiration_height);
			T::VaultProvider::return_securitization(lock.vault_id, &lock.get_securitization())
				.map_err(Error::<T>::from)?;
			T::BitcoinUtxoTracker::unwatch(utxo_id);
			Self::schedule_orphans_for_cleanup(utxo_id, lock);
			LocksByUtxoId::<T>::remove(utxo_id);
			UtxoIdsByVaultId::<T>::remove(lock.vault_id, utxo_id);
			UtxoIdsByOwnerAccount::<T>::remove(&lock.owner_account, utxo_id);
			UtxoIdToFundingUtxoRef::<T>::remove(utxo_id);

			Ok(())
		}

		fn pending_funding_expiration_height(start_height: BitcoinHeight) -> BitcoinHeight {
			start_height
				.saturating_add(T::MaxPendingConfirmationBlocks::get())
				.saturating_add(1)
		}

		fn unschedule_pending_funding(utxo_id: UtxoId, expiration_height: BitcoinHeight) {
			LocksPendingFundingByBitcoinHeight::<T>::mutate_exists(expiration_height, |locks| {
				let mut is_empty = false;
				if let Some(locks) = locks {
					locks.remove(&utxo_id);
					is_empty = locks.is_empty();
				}
				if is_empty {
					*locks = None;
				}
			});
		}

		pub(crate) fn process_pending_funding_expirations(
			expirations: impl IntoIterator<Item = UtxoId>,
			expiration_height: BitcoinHeight,
		) -> (u64, bool) {
			let mut expired_count = 0u64;
			let mut has_failed_expiration = false;
			for utxo_id in expirations {
				expired_count = expired_count.saturating_add(1);
				let Some(mut lock) = LocksByUtxoId::<T>::get(utxo_id) else {
					continue;
				};
				if lock.is_funded() {
					continue;
				}

				let result = with_storage_layer(|| {
					let securitization = lock.get_securitization();
					T::VaultProvider::return_securitization(lock.vault_id, &securitization)
						.map_err(Error::<T>::from)?;
					lock.securitized_satoshis = 0;
					lock.microgons_at_target_per_btc = T::Balance::zero();
					lock.securitization_coverage_microgons = T::Balance::zero();
					LocksByUtxoId::<T>::insert(utxo_id, &lock);
					Ok::<(), DispatchError>(())
				});
				if let Err(error) = result {
					log::error!(
						"Bitcoin lock {utxo_id:?} failed pending funding expiration: {error:?}"
					);
					has_failed_expiration = true;
					LocksPendingFundingByBitcoinHeight::<T>::try_mutate(
						expiration_height,
						|locks| locks.try_insert(utxo_id).map(|_| ()),
					)
					.expect("failed expirations fit back into their source bucket");
				}
			}
			(expired_count, has_failed_expiration)
		}

		fn schedule_orphans_for_cleanup(utxo_id: UtxoId, lock: &LockedBitcoin<T>) {
			let expiry_frame = T::CurrentFrameId::get() + T::OrphanedUtxoReleaseExpiryFrames::get();
			// Orphans are stored by account, so scan the owner's list for this lock's entries.
			let mut to_schedule = Vec::new();
			for (utxo_ref, entrant) in OrphanedUtxosByAccount::<T>::iter_prefix(&lock.owner_account)
			{
				if entrant.utxo_id != utxo_id {
					continue;
				}
				to_schedule.push(utxo_ref);
			}
			if to_schedule.is_empty() {
				return;
			}

			let owner_account = lock.owner_account.clone();
			let mut overflowed = false;
			OrphanedUtxoExpirationByFrame::<T>::mutate(expiry_frame, |a| {
				for utxo_ref in to_schedule {
					if a.try_insert((owner_account.clone(), utxo_ref)).is_err() {
						overflowed = true;
					}
				}
			});
			if overflowed {
				log::warn!(
					"Orphaned UTXO cleanup schedule overflowed for lock {utxo_id:?} at frame {expiry_frame:?}"
				);
				Self::deposit_event(Event::OrphanedUtxoCleanupScheduleOverflow {
					account_id: owner_account,
					utxo_id,
					expiration_frame: expiry_frame,
				});
			}
		}

		fn clear_orphans_for_lock(utxo_id: UtxoId, lock: &LockedBitcoin<T>) -> DispatchResult {
			let mut to_remove = Vec::new();
			for (utxo_ref, orphan) in OrphanedUtxosByAccount::<T>::iter_prefix(&lock.owner_account)
			{
				if orphan.utxo_id != utxo_id {
					continue;
				}
				to_remove.push((utxo_ref, orphan.cosign_request.is_some()));
			}

			for (utxo_ref, had_cosign) in to_remove {
				OrphanedUtxosByAccount::<T>::remove(&lock.owner_account, &utxo_ref);
				if had_cosign {
					T::VaultProvider::update_orphan_cosign_list(
						lock.vault_id,
						utxo_id,
						&lock.owner_account,
						true,
					)
					.map_err(Error::<T>::from)?;
				}
			}

			Ok(())
		}
	}
}

impl<T: Config> BitcoinFissionLockProvider<T::AccountId, T::Balance> for Pallet<T> {
	type Weights = weights::ProviderWeightAdapter<T>;

	fn fission_satoshis(
		account_id: &T::AccountId,
		utxo_id: UtxoId,
		satoshis: Satoshis,
		microgons_at_target_per_btc: T::Balance,
	) -> Result<(T::Balance, Tick), BitcoinFissionLockError> {
		LocksByUtxoId::<T>::try_mutate(utxo_id, |lock| {
			let lock = lock.as_mut().ok_or(BitcoinFissionLockError::LockNotFound)?;

			ensure!(lock.owner_account == *account_id, BitcoinFissionLockError::NoPermissions);
			ensure!(lock.is_funded(), BitcoinFissionLockError::LockNotFunded);
			ensure!(
				!LockReleaseRequestsByUtxoId::<T>::contains_key(utxo_id),
				BitcoinFissionLockError::LockReleasePending
			);
			let last_ratchet_tick =
				Self::latest_microgons_at_target_per_btc_tick(microgons_at_target_per_btc)
					.ok_or(BitcoinFissionLockError::IneligibleMicrogonsAtTargetPerBtc)?;
			ensure!(
				last_ratchet_tick >= lock.securitization_tick,
				BitcoinFissionLockError::MicrogonsAtTargetPerBtcTickOlderThanCurrent
			);

			let allocated_satoshis = lock
				.fissioned_satoshis
				.checked_add(satoshis)
				.ok_or(BitcoinFissionLockError::Overflow)?;

			ensure!(
				allocated_satoshis <= lock.funded_satoshis,
				BitcoinFissionLockError::InsufficientFundedSatoshis
			);
			let btc_value_in_microgons =
				FixedU128::from_rational(satoshis as u128, SATOSHIS_PER_BITCOIN as u128)
					.saturating_mul_int(microgons_at_target_per_btc);
			let liquidity_promised =
				Self::calculate_redemption_amount(btc_value_in_microgons, None)
					.map_err(|_| BitcoinFissionLockError::Overflow)?;
			let existing_liquidity =
				T::FissionsProvider::get_lock_fission_requirements(account_id, utxo_id)
					.map(|requirements| requirements.liquidity_promised)
					.unwrap_or_default();
			let required_liquidity = existing_liquidity
				.checked_add(&liquidity_promised)
				.ok_or(BitcoinFissionLockError::Overflow)?;
			ensure!(
				allocated_satoshis <= lock.securitized_satoshis &&
					microgons_at_target_per_btc <= lock.microgons_at_target_per_btc &&
					required_liquidity <= lock.securitization_coverage_microgons,
				BitcoinFissionLockError::InsufficientSecuritization
			);

			lock.fissioned_satoshis = allocated_satoshis;
			Ok((liquidity_promised, last_ratchet_tick))
		})
	}

	fn validate_fission(
		account_id: &T::AccountId,
		utxo_id: UtxoId,
		satoshis: Satoshis,
		microgons_at_target_per_btc: T::Balance,
		minimum_last_ratchet_tick: Tick,
		current_liquidity_promised: T::Balance,
		replacement_liquidity_promised: T::Balance,
	) -> Result<Tick, BitcoinFissionLockError> {
		let lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(BitcoinFissionLockError::LockNotFound)?;

		ensure!(lock.owner_account == *account_id, BitcoinFissionLockError::NoPermissions);
		ensure!(lock.is_funded(), BitcoinFissionLockError::LockNotFunded);
		ensure!(
			!LockReleaseRequestsByUtxoId::<T>::contains_key(utxo_id),
			BitcoinFissionLockError::LockReleasePending
		);
		let last_ratchet_tick =
			Self::latest_microgons_at_target_per_btc_tick(microgons_at_target_per_btc)
				.ok_or(BitcoinFissionLockError::IneligibleMicrogonsAtTargetPerBtc)?;
		ensure!(
			last_ratchet_tick >= minimum_last_ratchet_tick &&
				last_ratchet_tick >= lock.securitization_tick,
			BitcoinFissionLockError::MicrogonsAtTargetPerBtcTickOlderThanCurrent
		);
		ensure!(
			satoshis <= lock.fissioned_satoshis,
			BitcoinFissionLockError::InsufficientFissionedSatoshis
		);
		let current_requirements =
			T::FissionsProvider::get_lock_fission_requirements(account_id, utxo_id)
				.ok_or(BitcoinFissionLockError::InsufficientFissionedSatoshis)?;
		let required_liquidity = current_requirements
			.liquidity_promised
			.checked_sub(&current_liquidity_promised)
			.and_then(|liquidity| liquidity.checked_add(&replacement_liquidity_promised))
			.ok_or(BitcoinFissionLockError::Overflow)?;
		ensure!(
			lock.fissioned_satoshis <= lock.securitized_satoshis &&
				microgons_at_target_per_btc <= lock.microgons_at_target_per_btc &&
				required_liquidity <= lock.securitization_coverage_microgons,
			BitcoinFissionLockError::InsufficientSecuritization
		);

		Ok(last_ratchet_tick)
	}

	fn calculate_liquidity_promised(
		satoshis: Satoshis,
		microgons_at_target_per_btc: T::Balance,
	) -> Result<T::Balance, BitcoinFissionLockError> {
		let btc_value_in_microgons =
			FixedU128::from_rational(satoshis as u128, SATOSHIS_PER_BITCOIN as u128)
				.saturating_mul_int(microgons_at_target_per_btc);
		Self::calculate_redemption_amount(btc_value_in_microgons, None)
			.map_err(|_| BitcoinFissionLockError::Overflow)
	}

	fn fuse_satoshis(
		account_id: &T::AccountId,
		utxo_id: UtxoId,
		satoshis: Satoshis,
		microgons_at_target_per_btc: T::Balance,
	) -> Result<T::Balance, BitcoinFissionLockError> {
		LocksByUtxoId::<T>::try_mutate(utxo_id, |lock| {
			let lock = lock.as_mut().ok_or(BitcoinFissionLockError::LockNotFound)?;

			ensure!(lock.owner_account == *account_id, BitcoinFissionLockError::NoPermissions);
			let remaining_fissioned_satoshis = lock
				.fissioned_satoshis
				.checked_sub(satoshis)
				.ok_or(BitcoinFissionLockError::InsufficientFissionedSatoshis)?;

			let fission_btc_value_in_microgons =
				FixedU128::from_rational(satoshis as u128, SATOSHIS_PER_BITCOIN as u128)
					.saturating_mul_int(microgons_at_target_per_btc);
			let redemption_amount = Self::calculate_redemption_amount_from_satoshis(
				&satoshis,
				Some(fission_btc_value_in_microgons),
			)
			.map_err(|error| match error {
				Error::<T>::NoBitcoinPricesAvailable =>
					BitcoinFissionLockError::NoBitcoinPricesAvailable,
				_ => BitcoinFissionLockError::Overflow,
			})?;

			lock.fissioned_satoshis = remaining_fissioned_satoshis;
			Ok(redemption_amount)
		})
	}
}

pub trait BitcoinVerifier<T: Config> {
	fn verify_signature(
		utxo_releaser: CosignReleaser,
		pubkey: CompressedBitcoinPubkey,
		signature: &BitcoinSignature,
	) -> Result<bool, DispatchError> {
		utxo_releaser.verify_signature_raw(pubkey, signature).map_err(|e| {
			match e {
				argon_bitcoin::Error::InvalidCompressPubkeyBytes =>
					Error::<T>::BitcoinPubkeyUnableToBeDecoded,
				argon_bitcoin::Error::InvalidSignatureBytes =>
					Error::<T>::BitcoinSignatureUnableToBeDecoded,
				_ => Error::<T>::BitcoinInvalidCosignature,
			}
			.into()
		})
	}
}
