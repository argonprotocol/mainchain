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
		BitcoinLockId, BitcoinNetwork, BitcoinSignature, CompressedBitcoinPubkey, H256Le, Satoshis,
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
/// Confirmed outputs to a Lock address accumulate in its bounded UTXO set. Outputs received after
/// release begins remain recoverable orphans, except for the exact same-script change committed by
/// a cosigned partial release. Pending securitization may expire before funding arrives without
/// invalidating the receive address.
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
	use argon_bitcoin::{
		primitives::{ScriptBuf, TxOut},
		Amount, CosignReleaser, CosignScript, CosignScriptArgs, ReleaseStep,
	};
	use argon_primitives::{
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinLockId, BitcoinScriptPubkey,
			BitcoinSignature, CompressedBitcoinPubkey, Satoshis, UtxoRef, XPubChildNumber,
			XPubFingerprint, SATOSHIS_PER_BITCOIN,
		},
		vault::{
			BitcoinLockFundingUpdate, BitcoinResecuritization, BitcoinSecuritization,
			BitcoinSecuritizationBasis, BitcoinVaultProvider, LockExtension,
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

		/// Number of Bitcoin blocks to reserve securitization that has not yet been activated.
		#[pallet::constant]
		type SecuritizationHoldBlocks: Get<BitcoinHeight>;

		/// Maximum number of confirmed Bitcoin outputs that may fund one Lock.
		#[pallet::constant]
		type MaxUtxosPerLock: Get<u32>;

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

	#[pallet::storage]
	pub type NextBitcoinLockId<T: Config> = StorageValue<_, BitcoinLockId, OptionQuery>;

	/// Stores bitcoin utxos that have requested to be released
	#[pallet::storage]
	pub type LocksById<T: Config> =
		StorageMap<_, Twox64Concat, BitcoinLockId, LockedBitcoin<T>, OptionQuery>;

	/// Index of active UTXO IDs per vault
	#[pallet::storage]
	pub type LockIdsByVaultId<T: Config> =
		StorageDoubleMap<_, Twox64Concat, VaultId, Twox64Concat, BitcoinLockId, (), OptionQuery>;

	/// Index of active UTXO IDs per owner account.
	#[pallet::storage]
	pub type LockIdsByOwnerAccount<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		BitcoinLockId,
		(),
		OptionQuery,
	>;

	/// Latest cosigned release for each Lock, retained for signature recovery.
	#[pallet::storage]
	pub type LockReleaseCosignHeightById<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinLockId,
		LockReleaseCosignHeight<BlockNumberFor<T>>,
		OptionQuery,
	>;

	/// Release requests awaiting a Vault cosignature.
	#[pallet::storage]
	pub type LockReleaseRequestsById<T: Config> =
		StorageMap<_, Twox64Concat, BitcoinLockId, LockReleaseRequest<T::Balance>, OptionQuery>;

	/// Cosigned partial releases awaiting confirmation on Bitcoin.
	#[pallet::storage]
	pub type PendingPartialReleaseByLockId<T: Config> =
		StorageMap<_, Twox64Concat, BitcoinLockId, PendingPartialRelease, OptionQuery>;

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
		BoundedBTreeSet<BitcoinLockId, T::MaxConcurrentlyReleasingLocks>,
		ValueQuery,
	>;

	/// Expiration of bitcoin locks by bitcoin height. Funds are burned since the user did not
	/// unlock it. Bitcoin will go to vault
	#[pallet::storage]
	pub type LockExpirationsByBitcoinHeight<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinHeight,
		BoundedBTreeSet<BitcoinLockId, T::MaxConcurrentlyExpiringLocks>,
		ValueQuery,
	>;

	/// Lock IDs whose securitization hold expires at the indexed Bitcoin height.
	/// Expiry releases only the unactivated portion and leaves the lock address watched.
	#[pallet::storage]
	pub type SecuritizationHoldExpirationsByBitcoinHeight<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinHeight,
		BoundedBTreeSet<BitcoinLockId, T::MaxConcurrentlyExpiringLocks>,
		ValueQuery,
	>;

	/// Highest Bitcoin UTXO sync height processed for securitization hold expirations.
	#[pallet::storage]
	pub type LastProcessedSecuritizationHoldBitcoinHeight<T: Config> =
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
		/// Bitcoin amount and valuation used to price this Lock's securitization.
		pub securitization_basis: BitcoinSecuritizationBasis<T::Balance>,
		/// Microgon coverage purchased after applying the redemption curve.
		#[codec(compact)]
		pub securitization_coverage_microgons: T::Balance,
		/// Tick of the price-history entry used for the current securitization.
		#[codec(compact)]
		pub securitization_tick: Tick,
		/// Satoshis attached to this Lock by confirmed funding outputs. Zero means unfunded.
		#[codec(compact)]
		pub funded_satoshis: Satoshis,
		/// Confirmed Bitcoin outputs assigned to this Lock and their individual amounts.
		pub funding_utxos: BoundedBTreeMap<UtxoRef, Satoshis, T::MaxUtxosPerLock>,
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
		/// The Bitcoin height through which unused securitization remains reserved.
		#[codec(compact)]
		pub securitization_hold_expiration_bitcoin_height: BitcoinHeight,
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
				basis: self.securitization_basis,
				securitization_coverage_microgons: self.securitization_coverage_microgons,
				securitization_ratio: self.securitization_ratio,
			}
		}
	}

	#[derive(Decode, Encode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub struct LockReleaseCosignHeight<
		BlockNumber: Clone + Eq + PartialEq + TypeInfo + Codec + MaxEncodedLen,
	> {
		/// Argon block containing the Vault cosignatures.
		#[codec(compact)]
		pub cosign_height: BlockNumber,
		/// Argon block containing the preceding cosigned release, if one exists.
		pub previous_cosign_height: Option<BlockNumber>,
		/// Number of this release in the Lock's canonical cosign history.
		#[codec(compact)]
		pub release_number: u32,
	}

	#[derive(Decode, Encode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub struct PendingPartialRelease {
		/// Number correlating this pending change with its request, cosign, and settlement.
		#[codec(compact)]
		pub release_number: u32,
		/// Exact change output committed by the cosigned transaction.
		pub expected_change_utxo_ref: UtxoRef,
		/// Exact number of satoshis required in the replacement Lock UTXO.
		#[codec(compact)]
		pub change_satoshis: Satoshis,
	}

	#[derive(
		Decode, Encode, CloneNoBound, PartialEqNoBound, EqNoBound, Debug, TypeInfo, MaxEncodedLen,
	)]
	pub struct LockReleaseRequest<
		Balance: Clone + Eq + PartialEq + TypeInfo + Codec + MaxEncodedLen,
	> {
		/// The utxo id this request is related to
		#[codec(compact)]
		pub lock_id: BitcoinLockId,
		/// The vault id this request is related to
		#[codec(compact)]
		pub vault_id: VaultId,
		/// Number correlating this request with its cosign and settlement.
		#[codec(compact)]
		pub release_number: u32,
		/// The network fee to take out of the bitcoin being released
		#[codec(compact)]
		pub bitcoin_network_fee: Satoshis,
		/// The exact number of satoshis paid to the external destination.
		#[codec(compact)]
		pub destination_satoshis: Satoshis,
		/// The exact number of satoshis returned to this Lock's script.
		#[codec(compact)]
		pub change_satoshis: Satoshis,
		/// The frame when cosign is due.
		#[codec(compact)]
		pub cosign_due_frame: FrameId,
		/// The script pubkey where the bitcoin is to be sent
		pub to_script_pubkey: BitcoinScriptPubkey,
		/// Transaction ID committed by the frozen inputs and exact outputs.
		pub expected_transaction_id: H256Le,
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
		pub lock_id: BitcoinLockId,
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
			lock_id: BitcoinLockId,
			vault_id: VaultId,
			securitization_basis: BitcoinSecuritizationBasis<T::Balance>,
			collateral_required: T::Balance,
			account_id: T::AccountId,
			security_fee: T::Balance,
		},
		BitcoinLockBurned {
			lock_id: BitcoinLockId,
			vault_id: VaultId,
			was_utxo_spent: bool,
		},
		BitcoinUtxoCosignRequested {
			lock_id: BitcoinLockId,
			vault_id: VaultId,
			release_number: u32,
		},
		BitcoinUtxoCosigned {
			lock_id: BitcoinLockId,
			vault_id: VaultId,
			release_number: u32,
			signatures: BoundedVec<BitcoinSignature, T::MaxUtxosPerLock>,
		},
		BitcoinSpentAfterRelease {
			lock_id: BitcoinLockId,
			vault_id: VaultId,
			release_number: u32,
			bitcoin_height: BitcoinHeight,
		},
		BitcoinCosignPastDue {
			lock_id: BitcoinLockId,
			vault_id: VaultId,
			release_number: u32,
			compensation_amount: T::Balance,
			compensated_account_id: T::AccountId,
		},
		/// An error occurred while refunding an overdue cosigned bitcoin lock
		CosignOverdueError {
			lock_id: BitcoinLockId,
			error: DispatchError,
		},
		/// An error occurred while completing a lock
		LockExpirationError {
			lock_id: BitcoinLockId,
			error: DispatchError,
		},
		OrphanedUtxoReceived {
			lock_id: BitcoinLockId,
			utxo_ref: UtxoRef,
			vault_id: VaultId,
			satoshis: Satoshis,
		},
		OrphanedUtxoReleaseRequested {
			lock_id: BitcoinLockId,
			utxo_ref: UtxoRef,
			vault_id: VaultId,
			account_id: T::AccountId,
		},
		OrphanedUtxoCosigned {
			lock_id: BitcoinLockId,
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
			lock_id: BitcoinLockId,
			expiration_frame: FrameId,
		},
		BitcoinLockResecuritized {
			lock_id: BitcoinLockId,
			vault_id: VaultId,
			securitization_basis: BitcoinSecuritizationBasis<T::Balance>,
			account_id: T::AccountId,
		},
		BitcoinLockFlexibleChanged {
			lock_id: BitcoinLockId,
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
		/// The external destination amount must be nonzero and fit with the network fee.
		InvalidBitcoinReleaseAmount,
		/// A configured or requested Lock amount is below the required minimum.
		BitcoinReleaseChangeBelowMinimum,
		/// The minimum cannot increase while a release is pending.
		MinimumSatoshisIncreaseBlockedByPendingRelease,
		/// The release destination cannot be the Lock's own script.
		BitcoinReleaseDestinationIsLockScript,
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
		LockNotFunded,
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
		/// Too many confirmed Bitcoin outputs were assigned to one Lock.
		MaxUtxosPerLockExceeded,
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
			let mut last_securitization_hold_expiration =
				LastProcessedSecuritizationHoldBitcoinHeight::<T>::get();
			let securitization_hold_expiration_start = last_securitization_hold_expiration
				.map(|height| height.saturating_add(1))
				.unwrap_or(start_bitcoin_height);
			let mut securitization_hold_expiration_count = 0u64;
			for expiration_height in securitization_hold_expiration_start..=synched_bitcoin_height {
				let expirations =
					SecuritizationHoldExpirationsByBitcoinHeight::<T>::take(expiration_height);
				let (expired_count, has_failed_expiration) =
					Self::process_securitization_hold_expirations(expirations, expiration_height);
				securitization_hold_expiration_count =
					securitization_hold_expiration_count.saturating_add(expired_count);
				if has_failed_expiration {
					break
				}
				last_securitization_hold_expiration = Some(expiration_height);
			}
			if let Some(expiration_height) = last_securitization_hold_expiration {
				LastProcessedSecuritizationHoldBitcoinHeight::<T>::put(expiration_height);
			}

			let expirations = (start_bitcoin_height..=bitcoin_block_height)
				.flat_map(LockExpirationsByBitcoinHeight::<T>::take);
			let expiring_count = Self::process_expiring_locks(expirations);

			let overdue = LockCosignDueByFrame::<T>::take(T::CurrentFrameId::get());
			let overdue_count = Self::process_overdue_releases(overdue);

			let expiring = OrphanedUtxoExpirationByFrame::<T>::take(T::CurrentFrameId::get());
			let orphan_expiring_count = Self::process_orphaned_utxo_expirations(expiring);

			T::WeightInfo::on_initialize_with_expirations_and_overdue(
				expiring_count.saturated_into(),
				overdue_count.saturated_into(),
				orphan_expiring_count.saturated_into(),
				securitization_hold_expiration_count.saturated_into(),
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
			lock_id: Option<BitcoinLockId>,
			satoshis: Satoshis,
			microgons_at_target_per_btc: T::Balance,
		) -> bool {
			// FRAME preserves block zero and uses this lookup for CheckGenesis.
			let message = (
				FEE_COUPON_MESSAGE_KEY,
				frame_system::Pallet::<T>::block_hash(BlockNumberFor::<T>::zero()),
				vault_id,
				beneficiary,
				lock_id,
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

		/// Submitted by a Bitcoin holder to spend every current UTXO in a Lock. The exact external
		/// amount and network fee determine whether the transaction fully releases the Lock or
		/// returns one consolidated change output to the same Lock script. The vault operator has
		/// 10 days to publish one signature per frozen input in a BitcoinUtxoCosigned event.
		///
		/// Owner must submit a script pubkey and also a fee to pay to the bitcoin network.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::request_release())]
		pub fn request_release(
			origin: OriginFor<T>,
			lock_id: BitcoinLockId,
			to_script_pubkey: BitcoinScriptPubkey,
			destination_satoshis: Satoshis,
			bitcoin_network_fee: Satoshis,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let lock = LocksById::<T>::get(lock_id).ok_or(Error::<T>::LockNotFound)?;
			let vault_id = lock.vault_id;
			ensure!(lock.owner_account == who, Error::<T>::NoPermissions);

			// if no refund is needed, we can just cancel the lock
			if !lock.is_funded() {
				Self::cancel_lock(lock_id)?;
				return Ok(());
			}
			ensure!(!Self::is_release_pending(lock_id), Error::<T>::LockInProcessOfRelease);
			ensure!(bitcoin_network_fee < lock.funded_satoshis, Error::<T>::BitcoinFeeTooHigh);
			let destination_script: ScriptBuf = to_script_pubkey.clone().into();
			ensure!(
				destination_satoshis > 0 &&
					destination_satoshis >= destination_script.minimal_non_dust().to_sat(),
				Error::<T>::InvalidBitcoinReleaseAmount
			);
			ensure!(
				to_script_pubkey.0.as_slice() != lock.utxo_script_pubkey.to_script_bytes(),
				Error::<T>::BitcoinReleaseDestinationIsLockScript
			);
			let released_satoshis = destination_satoshis
				.checked_add(bitcoin_network_fee)
				.ok_or(Error::<T>::InvalidBitcoinReleaseAmount)?;
			let change_satoshis = lock
				.funded_satoshis
				.checked_sub(released_satoshis)
				.ok_or(Error::<T>::InvalidBitcoinReleaseAmount)?;
			if change_satoshis == 0 {
				ensure!(lock.fissioned_satoshis == 0, Error::<T>::LockHasActiveFissions);
			} else {
				ensure!(
					change_satoshis >= MinimumSatoshis::<T>::get(),
					Error::<T>::BitcoinReleaseChangeBelowMinimum
				);
				ensure!(
					change_satoshis >= lock.fissioned_satoshis,
					Error::<T>::InsufficientSatoshisForFissions
				);
			}

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

			let securitization_at_risk = Self::calculate_redemption_amount_from_satoshis(
				&lock.funded_satoshis,
				Some(lock.btc_value_in_microgons()),
			)?;

			let cosign_due_frame =
				T::LockReleaseCosignDeadlineFrames::get() + T::CurrentFrameId::get();
			let releaser = Self::create_release_releaser(
				&lock,
				to_script_pubkey.clone(),
				destination_satoshis,
				change_satoshis,
			)?;
			let expected_transaction_id = releaser.psbt.unsigned_tx.compute_txid().into();
			let release_number = LockReleaseCosignHeightById::<T>::get(lock_id)
				.map(|cosign| cosign.release_number.checked_add(1).ok_or(Error::<T>::OverflowError))
				.transpose()?
				.unwrap_or(1);
			LockReleaseRequestsById::<T>::insert(
				lock_id,
				LockReleaseRequest {
					lock_id,
					vault_id,
					release_number,
					bitcoin_network_fee,
					destination_satoshis,
					change_satoshis,
					cosign_due_frame,
					to_script_pubkey,
					expected_transaction_id,
					securitization_at_risk,
				},
			);

			LockCosignDueByFrame::<T>::try_mutate(cosign_due_frame, |a| a.try_insert(lock_id))
				.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;
			T::VaultProvider::update_pending_cosign_list(vault_id, lock_id, false)
				.map_err(Error::<T>::from)?;
			Self::deposit_event(Event::<T>::BitcoinUtxoCosignRequested {
				lock_id,
				vault_id,
				release_number,
			});
			Ok(())
		}

		/// Submitted by a Vault operator to cosign every input in a Bitcoin Lock release. The
		/// signatures must follow ascending `UtxoRef` order: transaction ID, then output index.
		/// The Lock remains frozen until the exact cosigned transaction is observed on Bitcoin.
		///
		/// This is submitted as a no-fee transaction off chain to allow keys to remain in cold
		/// wallets.
		#[pallet::call_index(2)]
		#[pallet::weight((T::WeightInfo::cosign_release(signatures.len() as u32), DispatchClass::Operational))]
		#[pallet::feeless_if(|origin: &OriginFor<T>, lock_id: &BitcoinLockId, _signatures: &BoundedVec<BitcoinSignature, T::MaxUtxosPerLock>| -> bool {
			let Ok(who) = ensure_signed(origin.clone()) else {
				return false;
			};
			if let Some(lock) = LocksById::<T>::get(lock_id) {
				return T::VaultProvider::is_owner(lock.vault_id, &who)
			}
			false
		})]
		#[allow(clippy::useless_conversion)]
		pub fn cosign_release(
			origin: OriginFor<T>,
			lock_id: BitcoinLockId,
			signatures: BoundedVec<BitcoinSignature, T::MaxUtxosPerLock>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let lock = LocksById::<T>::get(lock_id).ok_or(Error::<T>::LockNotFound)?;
			let vault_id = lock.vault_id;
			let vault_pubkey = lock.vault_pubkey;

			ensure!(T::VaultProvider::is_owner(vault_id, &who), Error::<T>::NoPermissions);
			ensure!(
				signatures.len() == lock.funding_utxos.len(),
				Error::<T>::BitcoinInvalidCosignature
			);
			let request = LockReleaseRequestsById::<T>::get(lock_id)
				.ok_or(Error::<T>::RedemptionNotLocked)?;
			let releaser = Self::create_release_releaser(
				&lock,
				request.to_script_pubkey.clone(),
				request.destination_satoshis,
				request.change_satoshis,
			)?;
			ensure!(
				releaser.psbt.unsigned_tx.compute_txid() ==
					request.expected_transaction_id.clone().into(),
				Error::<T>::BitcoinUnableToBeDecodedForRelease
			);

			let is_valid = T::BitcoinSignatureVerifier::verify_signatures(
				releaser,
				vault_pubkey,
				&signatures,
			)?;
			ensure!(is_valid, Error::<T>::BitcoinInvalidCosignature);

			if request.change_satoshis == 0 {
				Self::complete_full_release(lock_id)?;
			} else {
				let expected_change =
					UtxoRef { txid: request.expected_transaction_id.clone(), output_index: 1 };
				Self::take_release_request(lock_id)?;
				PendingPartialReleaseByLockId::<T>::insert(
					lock_id,
					PendingPartialRelease {
						release_number: request.release_number,
						expected_change_utxo_ref: expected_change,
						change_satoshis: request.change_satoshis,
					},
				);
			}
			let previous_cosign_height =
				LockReleaseCosignHeightById::<T>::get(lock_id).map(|cosign| cosign.cosign_height);
			LockReleaseCosignHeightById::<T>::insert(
				lock_id,
				LockReleaseCosignHeight {
					cosign_height: frame_system::Pallet::<T>::block_number(),
					previous_cosign_height,
					release_number: request.release_number,
				},
			);

			Self::deposit_event(Event::BitcoinUtxoCosigned {
				lock_id,
				vault_id,
				release_number: request.release_number,
				signatures,
			});

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
			let p2wsh_script: ScriptBuf =
				BitcoinCosignScriptPubkey::P2WSH { wscript_hash: Default::default() }.into();
			ensure!(
				satoshis >= p2wsh_script.minimal_non_dust().to_sat(),
				Error::<T>::BitcoinReleaseChangeBelowMinimum
			);
			if satoshis > MinimumSatoshis::<T>::get() {
				ensure!(
					!LockReleaseRequestsById::<T>::iter_values().any(|request| {
						request.change_satoshis > 0 && request.change_satoshis < satoshis
					}) && !PendingPartialReleaseByLockId::<T>::iter_values()
						.any(|release| release.change_satoshis < satoshis),
					Error::<T>::MinimumSatoshisIncreaseBlockedByPendingRelease
				);
			}
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
					!LocksById::<T>::get(entry.lock_id)
						.is_some_and(|lock| lock.funding_utxos.contains_key(&utxo_ref)),
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
					entry.lock_id,
					&who,
					false,
				)
				.map_err(Error::<T>::from)?;

				Self::deposit_event(Event::OrphanedUtxoReleaseRequested {
					lock_id: entry.lock_id,
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
			let lock_id = orphan.lock_id;
			// if not owner, "take" of ref will rollback
			ensure!(T::VaultProvider::is_owner(vault_id, &who), Error::<T>::NoPermissions);
			T::VaultProvider::update_orphan_cosign_list(vault_id, lock_id, &orphan_owner, true)
				.map_err(Error::<T>::from)?;
			T::BitcoinUtxoTracker::unwatch_utxo(lock_id, &utxo_ref);
			Self::deposit_event(Event::OrphanedUtxoCosigned {
				lock_id,
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
			lock_id: BitcoinLockId,
			#[pallet::compact] satoshis: Satoshis,
			options: Option<LockOptions<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut lock = LocksById::<T>::get(lock_id).ok_or(Error::<T>::LockNotFound)?;
			ensure!(lock.owner_account == who, Error::<T>::NoPermissions);
			ensure!(!Self::is_release_pending(lock_id), Error::<T>::LockInProcessOfRelease);
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
				Some(lock_id),
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
				satoshis != lock.securitization_basis.satoshis ||
					microgons_at_target_per_btc !=
						lock.securitization_basis.microgons_at_target_per_btc,
				Error::<T>::NoResecuritizationChange
			);
			ensure!(
				microgons_at_target_per_btc_tick >= lock.securitization_tick,
				Error::<T>::MicrogonsAtTargetPerBtcTickOlderThanCurrent
			);
			let fission_requirements =
				T::FissionsProvider::get_lock_fission_requirements(&who, lock_id);
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
				basis: BitcoinSecuritizationBasis { satoshis, microgons_at_target_per_btc },
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
			Self::unschedule_securitization_hold_expiration(
				lock_id,
				lock.securitization_hold_expiration_bitcoin_height,
			);
			if lock.funded_satoshis < replacement_securitization.basis.satoshis {
				let securitization_hold_expiration_bitcoin_height =
					Self::securitization_hold_expiration_height(current_bitcoin_height);
				SecuritizationHoldExpirationsByBitcoinHeight::<T>::try_mutate(
					securitization_hold_expiration_bitcoin_height,
					|locks| locks.try_insert(lock_id).map(|_| ()),
				)
				.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;
				lock.securitization_hold_expiration_bitcoin_height =
					securitization_hold_expiration_bitcoin_height;
			}
			lock.security_fees.saturating_accrue(fee);
			lock.coupon_paid_fees.saturating_accrue(coupon_paid_fees);
			lock.fund_hold_extensions = lock_extension.extended_expiration_funds;
			lock.securitization_basis = replacement_securitization.basis;
			lock.securitization_coverage_microgons = securitization_coverage_microgons;
			lock.securitization_tick = microgons_at_target_per_btc_tick;
			let vault_id = lock.vault_id;
			LocksById::<T>::insert(lock_id, lock);
			if let Some(coupon_nonce) = coupon_nonce {
				LastFeeCouponNonceByVaultAndAccount::<T>::insert(vault_id, &who, coupon_nonce);
			}
			Self::deposit_event(Event::BitcoinLockResecuritized {
				lock_id,
				vault_id,
				securitization_basis: replacement_securitization.basis,
				account_id: who,
			});
			Ok(())
		}

		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::set_flexible())]
		pub fn set_flexible(
			origin: OriginFor<T>,
			lock_id: BitcoinLockId,
			is_flexible: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut lock = LocksById::<T>::get(lock_id).ok_or(Error::<T>::LockNotFound)?;
			ensure!(lock.owner_account == who, Error::<T>::NoPermissions);
			let operator = T::VaultProvider::get_vault_operator(lock.vault_id)
				.ok_or(Error::<T>::VaultNotFound)?;
			ensure!(operator == who, Error::<T>::NoPermissions);
			ensure!(lock.is_funded(), Error::<T>::LockNotFunded);
			ensure!(!Self::is_release_pending(lock_id), Error::<T>::LockInProcessOfRelease);
			if lock.is_flexible == is_flexible {
				return Ok(());
			}

			let securitization = lock.get_securitization();
			T::VaultProvider::set_bitcoin_lock_flexible(
				lock.vault_id,
				&securitization,
				securitization.securitized_satoshis(lock.funded_satoshis),
				is_flexible,
			)
			.map_err(Error::<T>::from)?;
			lock.is_flexible = is_flexible;
			let vault_id = lock.vault_id;
			LocksById::<T>::insert(lock_id, lock);
			Self::deposit_event(Event::BitcoinLockFlexibleChanged {
				lock_id,
				vault_id,
				is_flexible,
			});
			Ok(())
		}
	}

	impl<T: Config> BitcoinUtxoEvents<T::AccountId> for Pallet<T> {
		type Weights = ProviderWeightAdapter<T>;

		fn utxo_detected(
			lock_id: BitcoinLockId,
			utxo_ref: UtxoRef,
			utxo_satoshis: Satoshis,
			bitcoin_height: BitcoinHeight,
		) -> DispatchResult {
			let mut lock = LocksById::<T>::get(lock_id).ok_or(Error::<T>::LockNotFound)?;
			if lock.funding_utxos.contains_key(&utxo_ref) {
				return Ok(());
			}
			if let Some(expected_change) = PendingPartialReleaseByLockId::<T>::get(lock_id) {
				// UtxoRef equality checks the txid and output index; the txid commits the amount.
				if utxo_ref == expected_change.expected_change_utxo_ref {
					return Self::complete_partial_release(
						lock_id,
						lock,
						expected_change.release_number,
						utxo_ref,
						utxo_satoshis,
						bitcoin_height,
					)
				}
				return Self::orphaned_utxo_detected(lock_id, utxo_satoshis, utxo_ref)
			}
			if let Some(request) = LockReleaseRequestsById::<T>::get(lock_id) {
				let expected_change_ref =
					UtxoRef { txid: request.expected_transaction_id.clone(), output_index: 1 };
				// UtxoRef equality checks the txid and output index; the txid commits the amount.
				if request.change_satoshis > 0 && utxo_ref == expected_change_ref {
					return Self::complete_partial_release(
						lock_id,
						lock,
						request.release_number,
						utxo_ref,
						utxo_satoshis,
						bitcoin_height,
					)
				}
				return Self::orphaned_utxo_detected(lock_id, utxo_satoshis, utxo_ref)
			}
			let previous_funded_satoshis = lock.funded_satoshis;
			let funded_satoshis = previous_funded_satoshis
				.checked_add(utxo_satoshis)
				.ok_or(Error::<T>::OverflowError)?;
			if lock.funding_utxos.try_insert(utxo_ref.clone(), utxo_satoshis).is_err() {
				return Self::orphaned_utxo_detected(lock_id, utxo_satoshis, utxo_ref);
			}
			let securitization = lock.get_securitization();
			let previous_securitized_satoshis =
				securitization.securitized_satoshis(previous_funded_satoshis);
			let securitized_satoshis = securitization.securitized_satoshis(funded_satoshis);
			T::VaultProvider::record_bitcoin_lock_funding(
				lock.vault_id,
				BitcoinLockFundingUpdate {
					funded_satoshis: utxo_satoshis,
					securitized_satoshis: securitized_satoshis
						.saturating_sub(previous_securitized_satoshis),
					collateral_required: securitization
						.collateral_between(previous_securitized_satoshis, securitized_satoshis),
					eligible_satoshis: securitization.eligible_satoshis_between(
						previous_securitized_satoshis,
						securitized_satoshis,
					),
					is_flexible: lock.is_flexible,
				},
			)
			.map_err(Error::<T>::from)?;
			lock.funded_satoshis = funded_satoshis;
			if funded_satoshis >= lock.securitization_basis.satoshis {
				Self::unschedule_securitization_hold_expiration(
					lock_id,
					lock.securitization_hold_expiration_bitcoin_height,
				);
			}
			LocksById::<T>::insert(lock_id, lock);
			Ok(())
		}

		fn spent(
			lock_id: BitcoinLockId,
			utxo_ref: UtxoRef,
			bitcoin_height: BitcoinHeight,
		) -> DispatchResult {
			let Some(lock) = LocksById::<T>::get(lock_id) else {
				T::BitcoinUtxoTracker::unwatch(lock_id);
				return Ok(());
			};
			if !lock.funding_utxos.contains_key(&utxo_ref) {
				if let Some(orphan) =
					OrphanedUtxosByAccount::<T>::take(&lock.owner_account, &utxo_ref) &&
					orphan.cosign_request.is_some()
				{
					T::VaultProvider::update_orphan_cosign_list(
						orphan.vault_id,
						orphan.lock_id,
						&lock.owner_account,
						true,
					)
					.map_err(Error::<T>::from)?;
				}
				T::BitcoinUtxoTracker::unwatch_utxo(lock_id, &utxo_ref);
				return Ok(());
			}
			if let Some(request) = LockReleaseRequestsById::<T>::get(lock_id) &&
				request.change_satoshis == 0
			{
				let release_number = request.release_number;
				let vault_id = Self::complete_full_release(lock_id)?;
				Self::deposit_event(Event::BitcoinSpentAfterRelease {
					lock_id,
					vault_id,
					release_number,
					bitcoin_height,
				});
				return Ok(())
			}
			Self::burn_bitcoin_lock(lock_id, true)
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: Codec,
	{
		pub fn orphaned_utxo_detected(
			lock_id: BitcoinLockId,
			satoshis: Satoshis,
			utxo_ref: UtxoRef,
		) -> DispatchResult {
			let block_number = frame_system::Pallet::<T>::block_number();
			let lock = LocksById::<T>::get(lock_id).ok_or(Error::<T>::LockNotFound)?;
			let did_modify = OrphanedUtxosByAccount::<T>::mutate(
				lock.owner_account.clone(),
				&utxo_ref,
				|entry| {
					if entry.is_some() {
						// Avoid overwriting a pending cosign request if the UTXO is re-reported.
						return false;
					}
					*entry = Some(OrphanedUtxo {
						lock_id,
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
					lock_id,
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
			lock_id: Option<BitcoinLockId>,
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
					lock_id,
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

			let lock_id = NextBitcoinLockId::<T>::mutate(|a| {
				let next = a.unwrap_or_default() + 1;
				*a = Some(next);
				next
			});
			LockExpirationsByBitcoinHeight::<T>::mutate(vault_claim_height, |x| {
				x.try_insert(lock_id)
			})
			.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;
			let securitization_hold_expiration_bitcoin_height =
				Self::securitization_hold_expiration_height(current_bitcoin_height);
			SecuritizationHoldExpirationsByBitcoinHeight::<T>::mutate(
				securitization_hold_expiration_bitcoin_height,
				|locks| locks.try_insert(lock_id),
			)
			.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;

			T::BitcoinUtxoTracker::watch_for_utxo(lock_id, script_pubkey)?;

			LocksById::<T>::insert(
				lock_id,
				LockedBitcoin {
					owner_account: account_id.clone(),
					vault_id,
					securitization_basis: securitization.basis,
					securitization_coverage_microgons: securitization
						.securitization_coverage_microgons,
					securitization_tick,
					funded_satoshis: 0,
					funding_utxos: BoundedBTreeMap::default(),
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
					securitization_hold_expiration_bitcoin_height,
					utxo_script_pubkey: script_pubkey,
					is_flexible: false,
					fund_hold_extensions: BoundedBTreeMap::default(),
					created_at_argon_block: <frame_system::Pallet<T>>::block_number(),
				},
			);
			LockIdsByVaultId::<T>::insert(vault_id, lock_id, ());
			LockIdsByOwnerAccount::<T>::insert(account_id, lock_id, ());
			Self::deposit_event(Event::<T>::BitcoinLockCreated {
				lock_id,
				vault_id,
				securitization_basis: securitization.basis,
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
					basis: BitcoinSecuritizationBasis { satoshis, microgons_at_target_per_btc },
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

		fn create_release_releaser(
			lock: &LockedBitcoin<T>,
			to_script_pubkey: BitcoinScriptPubkey,
			destination_satoshis: Satoshis,
			change_satoshis: Satoshis,
		) -> Result<CosignReleaser, Error<T>> {
			let cosign_script = CosignScript::new(
				CosignScriptArgs {
					vault_pubkey: lock.vault_pubkey,
					owner_pubkey: lock.owner_pubkey,
					vault_claim_pubkey: lock.vault_claim_pubkey,
					created_at_height: lock.created_at_height,
					vault_claim_height: lock.vault_claim_height,
					open_claim_height: lock.open_claim_height,
				},
				T::GetBitcoinNetwork::get().into(),
			)
			.map_err(|_| Error::<T>::BitcoinUnableToBeDecodedForRelease)?;
			let mut outputs = vec![TxOut {
				value: Amount::from_sat(destination_satoshis),
				script_pubkey: to_script_pubkey.into(),
			}];
			if change_satoshis > 0 {
				outputs.push(TxOut {
					value: Amount::from_sat(change_satoshis),
					script_pubkey: lock.utxo_script_pubkey.into(),
				});
			}
			CosignReleaser::from_script_outputs(
				cosign_script,
				lock.funding_utxos
					.iter()
					.map(|(utxo_ref, satoshis)| (utxo_ref.clone(), *satoshis))
					.collect(),
				ReleaseStep::VaultCosign,
				outputs,
			)
			.map_err(|_| Error::<T>::BitcoinUnableToBeDecodedForRelease)
		}

		pub(crate) fn process_expiring_locks(
			expirations: impl IntoIterator<Item = BitcoinLockId>,
		) -> u64 {
			let mut expiring_count: u64 = 0;
			for lock_id in expirations {
				expiring_count = expiring_count.saturating_add(1);
				// A cosigned partial release remains valid on Bitcoin indefinitely. Keep the Lock
				// and its frozen inputs watched until that transaction or a conflicting spend is
				// observed.
				if PendingPartialReleaseByLockId::<T>::contains_key(lock_id) {
					continue
				}
				let res = with_storage_layer(|| {
					Self::burn_bitcoin_lock(lock_id, false)?;
					Ok(())
				});
				if let Err(e) = res {
					log::error!("Bitcoin utxo id {lock_id:?} failed to be burned {e:?}");
					Self::deposit_event(Event::<T>::LockExpirationError { lock_id, error: e });
				}
			}
			expiring_count
		}

		pub(crate) fn process_overdue_releases(
			overdue: impl IntoIterator<Item = BitcoinLockId>,
		) -> u64 {
			let mut overdue_count: u64 = 0;
			for lock_id in overdue {
				overdue_count = overdue_count.saturating_add(1);
				let res = with_storage_layer(|| Self::cosign_bitcoin_overdue(lock_id));
				if let Err(e) = res {
					log::error!(
						"Bitcoin lock id {lock_id:?} failed to handle overdue `cosign` {e:?}"
					);
					Self::deposit_event(Event::<T>::CosignOverdueError { lock_id, error: e });
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
							request.lock_id,
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

		fn burn_bitcoin_lock(lock_id: BitcoinLockId, is_externally_spent: bool) -> DispatchResult {
			let lock = Self::take_lock(lock_id)?;
			if is_externally_spent {
				Self::clear_orphans_for_lock(lock_id, &lock)?;
			} else {
				Self::schedule_orphans_for_cleanup(lock_id, &lock);
			}

			if !lock.is_funded() {
				T::VaultProvider::release_unactivated_securitization(
					lock.vault_id,
					lock.get_securitization().collateral_required(),
				)
				.map_err(Error::<T>::from)?;
				return Ok(());
			}
			if is_externally_spent && lock.fissioned_satoshis == 0 {
				T::VaultProvider::release_bitcoin_lock_securitization(
					lock.vault_id,
					&lock.get_securitization(),
					lock.funded_satoshis,
					&lock.get_lock_extension(),
					lock.is_flexible,
				)
				.map_err(Error::<T>::from)?;
				return Ok(())
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
			T::FissionsProvider::close_for_lock(&lock.owner_account, lock_id, burned_argons)?;

			Self::deposit_event(Event::BitcoinLockBurned {
				lock_id,
				vault_id: lock.vault_id,
				was_utxo_spent: is_externally_spent,
			});

			Ok(())
		}

		fn complete_partial_release(
			lock_id: BitcoinLockId,
			mut lock: LockedBitcoin<T>,
			release_number: u32,
			change_ref: UtxoRef,
			change_satoshis: Satoshis,
			bitcoin_height: BitcoinHeight,
		) -> DispatchResult {
			let vault_id = lock.vault_id;
			let previous_funded_satoshis = lock.funded_satoshis;
			let removed_satoshis = previous_funded_satoshis
				.checked_sub(change_satoshis)
				.ok_or(Error::<T>::InvalidBitcoinReleaseAmount)?;
			let securitization = lock.get_securitization();
			let previous_securitized_satoshis =
				securitization.securitized_satoshis(previous_funded_satoshis);
			let remaining_securitized_satoshis =
				securitization.securitized_satoshis(change_satoshis);
			let removed_securitized_satoshis =
				previous_securitized_satoshis.saturating_sub(remaining_securitized_satoshis);
			T::VaultProvider::record_bitcoin_lock_funding_reduction(
				vault_id,
				BitcoinLockFundingUpdate {
					funded_satoshis: removed_satoshis,
					securitized_satoshis: removed_securitized_satoshis,
					collateral_required: securitization.collateral_between(
						remaining_securitized_satoshis,
						previous_securitized_satoshis,
					),
					eligible_satoshis: securitization.eligible_satoshis_between(
						remaining_securitized_satoshis,
						previous_securitized_satoshis,
					),
					is_flexible: lock.is_flexible,
				},
			)
			.map_err(Error::<T>::from)?;
			for utxo_ref in lock.funding_utxos.keys() {
				T::BitcoinUtxoTracker::unwatch_utxo(lock_id, utxo_ref);
			}
			lock.funding_utxos.clear();
			lock.funding_utxos
				.try_insert(change_ref, change_satoshis)
				.map_err(|_| Error::<T>::MaxUtxosPerLockExceeded)?;
			lock.funded_satoshis = change_satoshis;
			let is_expired = T::BitcoinBlockHeightChange::get().1 >= lock.vault_claim_height;
			if LockReleaseRequestsById::<T>::contains_key(lock_id) {
				Self::take_release_request(lock_id)?;
			} else {
				PendingPartialReleaseByLockId::<T>::remove(lock_id);
			}
			LocksById::<T>::insert(lock_id, lock);
			Self::deposit_event(Event::BitcoinSpentAfterRelease {
				lock_id,
				vault_id,
				release_number,
				bitcoin_height,
			});
			if is_expired {
				Self::burn_bitcoin_lock(lock_id, false)?;
			}
			Ok(())
		}

		fn complete_full_release(lock_id: BitcoinLockId) -> Result<VaultId, DispatchError> {
			let lock = Self::take_lock(lock_id)?;
			let vault_id = lock.vault_id;
			ensure!(lock.is_funded(), Error::<T>::LockNotFunded);
			T::FissionsProvider::close_for_lock(&lock.owner_account, lock_id, T::Balance::zero())?;
			T::VaultProvider::release_bitcoin_lock_securitization(
				vault_id,
				&lock.get_securitization(),
				lock.funded_satoshis,
				&lock.get_lock_extension(),
				lock.is_flexible,
			)
			.map_err(Error::<T>::from)?;
			Self::schedule_orphans_for_cleanup(lock_id, &lock);
			Ok(vault_id)
		}

		fn take_release_request(
			lock_id: BitcoinLockId,
		) -> Result<LockReleaseRequest<T::Balance>, Error<T>> {
			let request = LockReleaseRequestsById::<T>::get(lock_id)
				.ok_or(Error::<T>::RedemptionNotLocked)?;
			T::VaultProvider::update_pending_cosign_list(request.vault_id, lock_id, true)?;
			LockCosignDueByFrame::<T>::mutate(request.cosign_due_frame, |locks| {
				locks.remove(&lock_id);
			});
			LockReleaseRequestsById::<T>::remove(lock_id);
			Ok(request)
		}

		pub(crate) fn is_release_pending(lock_id: BitcoinLockId) -> bool {
			LockReleaseRequestsById::<T>::contains_key(lock_id) ||
				PendingPartialReleaseByLockId::<T>::contains_key(lock_id)
		}

		fn take_lock(lock_id: BitcoinLockId) -> Result<LockedBitcoin<T>, Error<T>> {
			let lock = LocksById::<T>::get(lock_id).ok_or(Error::<T>::LockNotFound)?;
			if LockReleaseRequestsById::<T>::contains_key(lock_id) {
				Self::take_release_request(lock_id)?;
			}
			LocksById::<T>::remove(lock_id);
			LockIdsByVaultId::<T>::remove(lock.vault_id, lock_id);
			LockIdsByOwnerAccount::<T>::remove(&lock.owner_account, lock_id);
			PendingPartialReleaseByLockId::<T>::remove(lock_id);
			T::BitcoinUtxoTracker::unwatch(lock_id);
			Ok(lock)
		}

		/// Call made during the on_initialize to implement cosign overdue penalties.
		pub(crate) fn cosign_bitcoin_overdue(lock_id: BitcoinLockId) -> DispatchResult {
			let entry = LockReleaseRequestsById::<T>::get(lock_id)
				.ok_or(Error::<T>::RedemptionNotLocked)?;
			let lock = Self::take_lock(lock_id)?;
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
			T::FissionsProvider::close_for_lock(&lock.owner_account, lock_id, compensation.burned)?;
			Self::deposit_event(Event::BitcoinCosignPastDue {
				lock_id,
				vault_id,
				release_number: entry.release_number,
				compensation_amount: compensation.to_beneficiary,
				compensated_account_id: lock.owner_account.clone(),
			});
			Self::schedule_orphans_for_cleanup(lock_id, &lock);

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

		fn cancel_lock(lock_id: BitcoinLockId) -> DispatchResult {
			let lock = Self::take_lock(lock_id)?;
			Self::unschedule_securitization_hold_expiration(
				lock_id,
				lock.securitization_hold_expiration_bitcoin_height,
			);
			T::VaultProvider::release_unactivated_securitization(
				lock.vault_id,
				lock.get_securitization().collateral_required(),
			)
			.map_err(Error::<T>::from)?;
			Self::schedule_orphans_for_cleanup(lock_id, &lock);

			Ok(())
		}

		fn securitization_hold_expiration_height(start_height: BitcoinHeight) -> BitcoinHeight {
			start_height
				.saturating_add(T::SecuritizationHoldBlocks::get())
				.saturating_add(1)
		}

		fn unschedule_securitization_hold_expiration(
			lock_id: BitcoinLockId,
			expiration_height: BitcoinHeight,
		) {
			SecuritizationHoldExpirationsByBitcoinHeight::<T>::mutate_exists(
				expiration_height,
				|locks| {
					let mut is_empty = false;
					if let Some(locks) = locks {
						locks.remove(&lock_id);
						is_empty = locks.is_empty();
					}
					if is_empty {
						*locks = None;
					}
				},
			);
		}

		pub(crate) fn process_securitization_hold_expirations(
			expirations: impl IntoIterator<Item = BitcoinLockId>,
			expiration_height: BitcoinHeight,
		) -> (u64, bool) {
			let mut expired_count = 0u64;
			let mut has_failed_expiration = false;
			for lock_id in expirations {
				expired_count = expired_count.saturating_add(1);
				let Some(mut lock) = LocksById::<T>::get(lock_id) else {
					continue;
				};
				if lock.funded_satoshis >= lock.securitization_basis.satoshis {
					continue;
				}

				let result = with_storage_layer(|| {
					let securitization = lock.get_securitization();
					let securitization_coverage_microgons =
						securitization.coverage_for_satoshis(lock.funded_satoshis);
					let collateral_required = lock
						.securitization_ratio
						.saturating_mul_int(securitization_coverage_microgons);
					T::VaultProvider::release_unactivated_securitization(
						lock.vault_id,
						securitization.collateral_required().saturating_sub(collateral_required),
					)
					.map_err(Error::<T>::from)?;
					lock.securitization_basis.satoshis = lock.funded_satoshis;
					lock.securitization_coverage_microgons = securitization_coverage_microgons;
					if !lock.is_funded() {
						lock.securitization_basis.microgons_at_target_per_btc = T::Balance::zero();
					}
					LocksById::<T>::insert(lock_id, &lock);
					Ok::<(), DispatchError>(())
				});
				if let Err(error) = result {
					log::error!(
						"Bitcoin lock {lock_id:?} failed securitization hold expiration: {error:?}"
					);
					has_failed_expiration = true;
					SecuritizationHoldExpirationsByBitcoinHeight::<T>::try_mutate(
						expiration_height,
						|locks| locks.try_insert(lock_id).map(|_| ()),
					)
					.expect("failed expirations fit back into their source bucket");
				}
			}
			(expired_count, has_failed_expiration)
		}

		fn schedule_orphans_for_cleanup(lock_id: BitcoinLockId, lock: &LockedBitcoin<T>) {
			let expiry_frame = T::CurrentFrameId::get() + T::OrphanedUtxoReleaseExpiryFrames::get();
			// Orphans are stored by account, so scan the owner's list for this lock's entries.
			let mut to_schedule = Vec::new();
			for (utxo_ref, entrant) in OrphanedUtxosByAccount::<T>::iter_prefix(&lock.owner_account)
			{
				if entrant.lock_id != lock_id {
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
					"Orphaned UTXO cleanup schedule overflowed for lock {lock_id:?} at frame {expiry_frame:?}"
				);
				Self::deposit_event(Event::OrphanedUtxoCleanupScheduleOverflow {
					account_id: owner_account,
					lock_id,
					expiration_frame: expiry_frame,
				});
			}
		}

		fn clear_orphans_for_lock(
			lock_id: BitcoinLockId,
			lock: &LockedBitcoin<T>,
		) -> DispatchResult {
			let mut to_remove = Vec::new();
			for (utxo_ref, orphan) in OrphanedUtxosByAccount::<T>::iter_prefix(&lock.owner_account)
			{
				if orphan.lock_id != lock_id {
					continue;
				}
				to_remove.push((utxo_ref, orphan.cosign_request.is_some()));
			}

			for (utxo_ref, had_cosign) in to_remove {
				OrphanedUtxosByAccount::<T>::remove(&lock.owner_account, &utxo_ref);
				if had_cosign {
					T::VaultProvider::update_orphan_cosign_list(
						lock.vault_id,
						lock_id,
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
		lock_id: BitcoinLockId,
		satoshis: Satoshis,
		microgons_at_target_per_btc: T::Balance,
	) -> Result<(T::Balance, Tick), BitcoinFissionLockError> {
		LocksById::<T>::try_mutate(lock_id, |lock| {
			let lock = lock.as_mut().ok_or(BitcoinFissionLockError::LockNotFound)?;

			ensure!(lock.owner_account == *account_id, BitcoinFissionLockError::NoPermissions);
			ensure!(lock.is_funded(), BitcoinFissionLockError::LockNotFunded);
			ensure!(
				!Pallet::<T>::is_release_pending(lock_id),
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
				T::FissionsProvider::get_lock_fission_requirements(account_id, lock_id)
					.map(|requirements| requirements.liquidity_promised)
					.unwrap_or_default();
			let required_liquidity = existing_liquidity
				.checked_add(&liquidity_promised)
				.ok_or(BitcoinFissionLockError::Overflow)?;
			ensure!(
				allocated_satoshis <= lock.securitization_basis.satoshis &&
					microgons_at_target_per_btc <=
						lock.securitization_basis.microgons_at_target_per_btc &&
					required_liquidity <= lock.securitization_coverage_microgons,
				BitcoinFissionLockError::InsufficientSecuritization
			);

			lock.fissioned_satoshis = allocated_satoshis;
			Ok((liquidity_promised, last_ratchet_tick))
		})
	}

	fn validate_fission(
		account_id: &T::AccountId,
		lock_id: BitcoinLockId,
		satoshis: Satoshis,
		microgons_at_target_per_btc: T::Balance,
		minimum_last_ratchet_tick: Tick,
		current_liquidity_promised: T::Balance,
		replacement_liquidity_promised: T::Balance,
	) -> Result<Tick, BitcoinFissionLockError> {
		let lock = LocksById::<T>::get(lock_id).ok_or(BitcoinFissionLockError::LockNotFound)?;

		ensure!(lock.owner_account == *account_id, BitcoinFissionLockError::NoPermissions);
		ensure!(lock.is_funded(), BitcoinFissionLockError::LockNotFunded);
		ensure!(
			!Pallet::<T>::is_release_pending(lock_id),
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
			T::FissionsProvider::get_lock_fission_requirements(account_id, lock_id)
				.ok_or(BitcoinFissionLockError::InsufficientFissionedSatoshis)?;
		let required_liquidity = current_requirements
			.liquidity_promised
			.checked_sub(&current_liquidity_promised)
			.and_then(|liquidity| liquidity.checked_add(&replacement_liquidity_promised))
			.ok_or(BitcoinFissionLockError::Overflow)?;
		ensure!(
			lock.fissioned_satoshis <= lock.securitization_basis.satoshis &&
				microgons_at_target_per_btc <=
					lock.securitization_basis.microgons_at_target_per_btc &&
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
		lock_id: BitcoinLockId,
		satoshis: Satoshis,
		microgons_at_target_per_btc: T::Balance,
	) -> Result<T::Balance, BitcoinFissionLockError> {
		LocksById::<T>::try_mutate(lock_id, |lock| {
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
	fn verify_signatures(
		utxo_releaser: CosignReleaser,
		pubkey: CompressedBitcoinPubkey,
		signatures: &[BitcoinSignature],
	) -> Result<bool, DispatchError> {
		utxo_releaser.verify_signatures_raw(pubkey, signatures).map_err(|e| {
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
