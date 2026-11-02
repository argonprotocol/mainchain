#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::zero_prefixed_literal)]

extern crate alloc;
extern crate core;

use alloc::{collections::BTreeSet, vec::Vec};
use codec::Encode;
use pallet_prelude::*;
use polkadot_sdk::{
	sp_arithmetic::{helpers_128bit::multiply_by_rational_with_rounding, Rational128, Rounding},
	sp_runtime::traits::{IdentifyAccount, Verify},
};

use argon_bitcoin::CosignReleaser;
use argon_primitives::{
	bitcoin::{
		BitcoinLockId, BitcoinNetwork, BitcoinSignature, CompressedBitcoinPubkey, H256Le, Satoshis,
		SATOSHIS_PER_BITCOIN,
	},
	providers::{BitcoinFissionLockError, BitcoinFissionLockProvider},
	treasury::{BitcoinLockPosition, BitcoinLockPositionProvider, TreasuryPositionProvider},
	vault::{
		BitcoinLockFunding, BitcoinSecuritizationBasis, BitcoinVaultProvider, CosignerGuarantee,
		ReserveSecuritizationRequest, VaultError,
	},
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
#[cfg(feature = "try-runtime")]
mod try_state;
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
			XPubFingerprint,
		},
		vault::{
			BitcoinSecuritizationBasis, BitcoinVaultProvider, CosignerGuarantee,
			ReserveSecuritizationRequest,
		},
		BitcoinFissionsProvider, BitcoinUtxoEvents, BitcoinUtxoTracker, PriceProvider, VaultId,
	};
	use codec::HasCompact;
	use core::iter::Sum;
	pub(super) const STORAGE_VERSION: StorageVersion = StorageVersion::new(12);

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
		type PositionProvider: TreasuryPositionProvider<Self::AccountId, Self::Balance>;

		/// Currency used to retire pre-Fission release holds during migration.
		type Currency: MutateHold<
			Self::AccountId,
			Reason = Self::RuntimeHoldReason,
			Balance = Self::Balance,
		>;

		/// Runtime hold reason containing the deprecated Bitcoin Lock release variant for
		/// migration.
		type RuntimeHoldReason: From<HoldReason>;

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

		/// Initial Bitcoin deposit window, measured in Bitcoin blocks.
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

	/// Compatibility reason for pre-Fission release holds.
	///
	/// Remove after an upgrade has verified that no balances remain held for this reason.
	#[pallet::composite_enum]
	pub enum HoldReason {
		#[deprecated(note = "retained only to migrate pre-Fission Bitcoin release holds")]
		ReleaseBitcoinLock,
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

	/// Locks whose funding-window or terminal expiration failed and must be retried.
	#[pallet::storage]
	pub type PendingLockExpirations<T: Config> =
		StorageMap<_, Twox64Concat, BitcoinLockId, (), OptionQuery>;

	/// Resume retries after this Lock so a persistent failure cannot starve other expirations.
	#[pallet::storage]
	pub type LastRetriedLockExpiration<T: Config> = StorageValue<_, BitcoinLockId, OptionQuery>;

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

	/// Highest Bitcoin UTXO sync height checked for funding expirations; failures remain pending.
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
		/// Cosigner guarantee covering Bitcoin outside active Fissions.
		pub cosigner_guarantee: CosignerGuarantee<T::Balance>,
		/// Unfunded guarantee capacity expires at this Bitcoin height.
		pub securitization_hold_expiration_bitcoin_height: Option<BitcoinHeight>,
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
		/// Sum of all Lock fees, including guarantee purchases and changes.
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
		/// The script pubkey where funds are sent to fund this bitcoin lock
		pub utxo_script_pubkey: BitcoinCosignScriptPubkey,
		/// Whether this operator-owned lock may be displaced by outside vault capital.
		pub is_flexible: bool,
		/// The argon block when this lock was created
		#[codec(compact)]
		pub created_at_argon_block: BlockNumberFor<T>,
	}

	impl<T: Config> LockedBitcoin<T> {
		pub fn is_funded(&self) -> bool {
			self.funded_satoshis > 0
		}

		/// Funded Bitcoin includes satoshis assigned to Fissions. Covered satoshis and activated
		/// securitization count only the guarantee still held by this Lock.
		pub(crate) fn funding_and_securitization(&self) -> BitcoinLockFunding<T::Balance> {
			let available = self.funded_satoshis.saturating_sub(self.fissioned_satoshis);
			BitcoinLockFunding {
				funded_satoshis: self.funded_satoshis,
				securitized_satoshis: self.cosigner_guarantee.securitized_satoshis(available),
				activated_securitization: self
					.cosigner_guarantee
					.securitization_for_satoshis(available),
				is_flexible: self.is_flexible,
			}
		}

		pub fn lock_held_securitization(&self) -> BitcoinLockPosition<T::Balance> {
			BitcoinLockPosition {
				allocated_securitization: self.cosigner_guarantee.guaranteed_microgons,
				activated_securitization: self
					.funding_and_securitization()
					.activated_securitization,
			}
		}

		/// Total securitization allocated to this Lock and its active Fissions.
		pub fn total_securitization(
			&self,
			lock_id: BitcoinLockId,
		) -> BitcoinLockPosition<T::Balance> {
			let fissions =
				T::FissionsProvider::get_lock_fission_guarantees(&self.owner_account, lock_id);
			let mut position = self.lock_held_securitization();
			for (_, guarantee) in fissions {
				position
					.activated_securitization
					.saturating_accrue(guarantee.guaranteed_microgons);
				position
					.allocated_securitization
					.saturating_accrue(guarantee.guaranteed_microgons);
			}
			position
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
			securitization_microgons: T::Balance,
			account_id: T::AccountId,
			security_fee: T::Balance,
		},
		BitcoinLockTerminated {
			lock_id: BitcoinLockId,
			vault_id: VaultId,
			was_utxo_spent: bool,
			burned_argons: T::Balance,
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
		BitcoinLockGuaranteeSet {
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
		NoGuaranteeChange,
		/// A Lock's guarantee cannot be changed while release is pending.
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
		#[cfg(feature = "try-runtime")]
		fn try_state(_: BlockNumberFor<T>) -> Result<(), sp_runtime::TryRuntimeError> {
			crate::try_state::check::<T>()
		}

		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let (start_bitcoin_height, bitcoin_block_height) = T::BitcoinBlockHeightChange::get();
			let synched_bitcoin_height = T::BitcoinUtxoTracker::get_synched_height();
			let expiration_start = LastProcessedSecuritizationHoldBitcoinHeight::<T>::get()
				.map(|height| height.saturating_add(1))
				.unwrap_or(start_bitcoin_height);
			let mut expirations = (expiration_start..=synched_bitcoin_height)
				.flat_map(SecuritizationHoldExpirationsByBitcoinHeight::<T>::take)
				.collect::<BTreeSet<_>>();
			if expiration_start <= synched_bitcoin_height {
				LastProcessedSecuritizationHoldBitcoinHeight::<T>::put(synched_bitcoin_height);
			}

			// Retry one failed Lock per block, advancing past persistent failures.
			let last_retried = LastRetriedLockExpiration::<T>::get();
			let pending_expiration = if let Some(lock_id) = last_retried {
				PendingLockExpirations::<T>::iter_keys_from_key(lock_id)
					.next()
					.or_else(|| PendingLockExpirations::<T>::iter_keys().next())
			} else {
				PendingLockExpirations::<T>::iter_keys().next()
			};
			if pending_expiration != last_retried {
				LastRetriedLockExpiration::<T>::set(pending_expiration);
			}

			expirations.extend(pending_expiration);
			expirations.extend(
				(start_bitcoin_height..=bitcoin_block_height)
					.flat_map(LockExpirationsByBitcoinHeight::<T>::take),
			);
			let (expiring_count, securitization_hold_expiration_count) =
				Self::process_expiring_locks(
					expirations,
					bitcoin_block_height,
					synched_bitcoin_height,
				);

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
			let fee_coupon = if let Some(options) = options.as_ref() {
				Self::validate_fee_coupon(
					vault_id,
					&account_id,
					None,
					satoshis,
					options.microgons_at_target_per_btc,
					options.fee_coupon.as_ref(),
				)?
			} else {
				None
			};

			let securitization_request = ReserveSecuritizationRequest {
				remaining_term: FixedU128::from_u32(1),
				is_flexible: false,
				lock_expiration: T::BitcoinBlockHeightChange::get()
					.1
					.saturating_add(T::LockDurationBlocks::get()),
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
			let ticks_until_cosign_overdue = T::LockReleaseCosignDeadlineFrames::get()
				.saturating_mul(T::ArgonTicksPerDay::get());

			let safe_bitcoin_blocks_remaining =
				lock.vault_claim_height.saturating_sub(T::BitcoinBlockHeightChange::get().1);
			let ticks_until_vault_claim =
				safe_bitcoin_blocks_remaining.saturating_mul(T::TicksPerBitcoinBlock::get());
			ensure!(
				ticks_until_cosign_overdue < ticks_until_vault_claim,
				Error::<T>::BitcoinReleaseInitiationDeadlinePassed
			);

			let guarantee = &lock.cosigner_guarantee;
			let redemption = Self::calculate_redemption_amount_from_satoshis(
				&lock.funded_satoshis,
				Some(guarantee.basis.btc_value_in_microgons()),
			)?;
			let securitization_at_risk =
				redemption.min(guarantee.securitization_for_satoshis(lock.funded_satoshis));

			let cosign_due_frame =
				T::CurrentFrameId::get().saturating_add(T::LockReleaseCosignDeadlineFrames::get());
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

		/// Replace the Lock's guarantee, leaving active Fission guarantees unchanged.
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::set_guarantee())]
		pub fn set_guarantee(
			origin: OriginFor<T>,
			lock_id: BitcoinLockId,
			#[pallet::compact] satoshis: Satoshis,
			options: Option<LockOptions<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut lock = LocksById::<T>::get(lock_id).ok_or(Error::<T>::LockNotFound)?;
			ensure!(lock.owner_account == who, Error::<T>::NoPermissions);
			ensure!(!Self::is_release_pending(lock_id), Error::<T>::LockInProcessOfRelease);

			let (microgons_at_target_per_btc, tick) = Self::resolve_microgons_at_target_per_btc(
				options.as_ref().map(|options| options.microgons_at_target_per_btc),
			)?;

			let coupon = Self::validate_fee_coupon(
				lock.vault_id,
				&who,
				Some(lock_id),
				satoshis,
				microgons_at_target_per_btc,
				options.as_ref().and_then(|options| options.fee_coupon.as_ref()),
			)?;

			let requested_basis =
				BitcoinSecuritizationBasis { satoshis, microgons_at_target_per_btc, tick };
			let requested_guaranteed_microgons =
				Self::calculate_redemption_amount(requested_basis.btc_value_in_microgons(), None)?;

			ensure!(
				lock.cosigner_guarantee.basis != requested_basis ||
					lock.cosigner_guarantee.guaranteed_microgons !=
						requested_guaranteed_microgons,
				Error::<T>::NoGuaranteeChange
			);

			// Active Fissions keep their guarantees; only the Bitcoin available on the Lock counts
			// here.
			let previous_position = lock.lock_held_securitization();
			let funded_satoshis = lock.funded_satoshis.saturating_sub(lock.fissioned_satoshis);
			let mut replacement_guarantee = CosignerGuarantee {
				basis: requested_basis,
				guaranteed_microgons: requested_guaranteed_microgons,
				fund_hold_extensions: Default::default(),
			};

			let existing_lock_guarantee = lock.cosigner_guarantee.clone();
			Self::replace_guarantee(
				&mut lock,
				&existing_lock_guarantee,
				&mut replacement_guarantee,
				funded_satoshis,
				coupon,
			)?;

			lock.cosigner_guarantee = replacement_guarantee;
			Self::renew_securitization_hold(lock_id, &mut lock)?;

			T::PositionProvider::bitcoin_position_updated(
				&who,
				lock.vault_id,
				previous_position,
				lock.lock_held_securitization(),
			)?;

			let vault_id = lock.vault_id;
			LocksById::<T>::insert(lock_id, lock);

			Self::deposit_event(Event::BitcoinLockGuaranteeSet {
				lock_id,
				vault_id,
				securitization_basis: requested_basis,
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

			let activated = lock.total_securitization(lock_id).activated_securitization;
			T::VaultProvider::set_bitcoin_lock_flexible(lock.vault_id, activated, is_flexible)
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

			let previous_position = lock.lock_held_securitization();
			let funded_satoshis = lock
				.funded_satoshis
				.checked_add(utxo_satoshis)
				.ok_or(Error::<T>::OverflowError)?;
			if lock.funding_utxos.try_insert(utxo_ref.clone(), utxo_satoshis).is_err() {
				return Self::orphaned_utxo_detected(lock_id, utxo_satoshis, utxo_ref);
			}

			let previous_funding = lock.funding_and_securitization();
			lock.funded_satoshis = funded_satoshis;
			let updated_funding = lock.funding_and_securitization();
			T::VaultProvider::record_bitcoin_lock_funding(
				lock.vault_id,
				previous_funding,
				updated_funding,
			)
			.map_err(Error::<T>::from)?;

			if updated_funding.securitized_satoshis == lock.cosigner_guarantee.basis.satoshis &&
				let Some(deadline) = lock.securitization_hold_expiration_bitcoin_height.take()
			{
				Self::unschedule_securitization_hold_expiration(lock_id, deadline);
			}

			T::PositionProvider::bitcoin_position_updated(
				&lock.owner_account,
				lock.vault_id,
				previous_position,
				lock.lock_held_securitization(),
			)?;

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
		pub(crate) fn validate_fee_coupon<'a>(
			vault_id: VaultId,
			account_id: &T::AccountId,
			lock_id: Option<BitcoinLockId>,
			satoshis: Satoshis,
			microgons_at_target_per_btc: T::Balance,
			coupon: Option<&'a FeeCoupon<T>>,
		) -> Result<Option<&'a FeeCoupon<T>>, Error<T>> {
			let Some(coupon) = coupon else { return Ok(None) };

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
					microgons_at_target_per_btc,
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

			let (microgons_at_target_per_btc, securitization_tick) =
				Self::resolve_microgons_at_target_per_btc(
					options.as_ref().map(|options| options.microgons_at_target_per_btc),
				)?;
			let basis = BitcoinSecuritizationBasis {
				satoshis,
				microgons_at_target_per_btc,
				tick: securitization_tick,
			};
			let guaranteed_microgons =
				Self::calculate_redemption_amount(basis.btc_value_in_microgons(), None)?;

			let guarantee = CosignerGuarantee {
				basis,
				guaranteed_microgons,
				fund_hold_extensions: Default::default(),
			};

			let (fee, coupon_paid_fees) = T::VaultProvider::reserve_securitization(
				vault_id,
				account_id,
				&guarantee,
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

			let lock_id = NextBitcoinLockId::<T>::get()
				.unwrap_or_default()
				.checked_add(1)
				.ok_or(Error::<T>::OverflowError)?;
			NextBitcoinLockId::<T>::put(lock_id);
			LockExpirationsByBitcoinHeight::<T>::mutate(vault_claim_height, |x| {
				x.try_insert(lock_id)
			})
			.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;

			T::BitcoinUtxoTracker::watch_for_utxo(lock_id, script_pubkey)?;

			// The new Lock reserves securitization before any Bitcoin funding activates it.
			T::PositionProvider::bitcoin_position_updated(
				account_id,
				vault_id,
				BitcoinLockPosition::default(),
				BitcoinLockPosition {
					allocated_securitization: guarantee.guaranteed_microgons,
					..Default::default()
				},
			)?;
			let mut lock = LockedBitcoin {
				owner_account: account_id.clone(),
				vault_id,
				cosigner_guarantee: guarantee,
				securitization_hold_expiration_bitcoin_height: None,
				funded_satoshis: 0,
				funding_utxos: BoundedBTreeMap::default(),
				fissioned_satoshis: 0,
				security_fees: fee,
				coupon_paid_fees,
				vault_pubkey,
				vault_claim_pubkey,
				vault_xpub_sources,
				owner_pubkey: bitcoin_pubkey,
				vault_claim_height,
				open_claim_height,
				created_at_height: current_bitcoin_height,
				utxo_script_pubkey: script_pubkey,
				is_flexible: false,
				created_at_argon_block: <frame_system::Pallet<T>>::block_number(),
			};
			Self::renew_securitization_hold(lock_id, &mut lock)?;
			LocksById::<T>::insert(lock_id, lock);
			LockIdsByVaultId::<T>::insert(vault_id, lock_id, ());
			LockIdsByOwnerAccount::<T>::insert(account_id, lock_id, ());
			Self::deposit_event(Event::<T>::BitcoinLockCreated {
				lock_id,
				vault_id,
				securitization_basis: basis,
				securitization_microgons: guaranteed_microgons,
				account_id: account_id.clone(),
				security_fee: fee,
			});

			Ok(())
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

		pub(crate) fn create_release_releaser(
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
			bitcoin_height: BitcoinHeight,
			synched_height: BitcoinHeight,
		) -> (u64, u64) {
			let mut terminal_expirations = Vec::new();
			let mut funding_expiration_count = 0u64;

			// Retire all due reservations before any settlement draws unused Vault capital.
			for lock_id in expirations {
				PendingLockExpirations::<T>::remove(lock_id);
				let Some(mut lock) = LocksById::<T>::get(lock_id) else {
					funding_expiration_count.saturating_accrue(1);
					continue;
				};
				let funding_expired = lock
					.securitization_hold_expiration_bitcoin_height
					.is_some_and(|height| height <= synched_height);

				if funding_expired {
					funding_expiration_count.saturating_accrue(1);
					let result = with_storage_layer(|| {
						let previous_position = lock.lock_held_securitization();
						Self::shrink_lock_securitization(&mut lock, true)?;
						T::PositionProvider::bitcoin_position_updated(
							&lock.owner_account,
							lock.vault_id,
							previous_position,
							lock.lock_held_securitization(),
						)?;
						LocksById::<T>::insert(lock_id, &lock);
						Ok::<(), DispatchError>(())
					});
					if let Err(error) = result {
						PendingLockExpirations::<T>::insert(lock_id, ());
						log::error!(
							"Bitcoin lock {lock_id:?} failed funding expiration: {error:?}"
						);
					}
				}

				// A signed partial release stays watched until its Bitcoin spend is observed.
				if lock.vault_claim_height <= bitcoin_height &&
					!PendingPartialReleaseByLockId::<T>::contains_key(lock_id)
				{
					terminal_expirations.push(lock_id);
				} else if !funding_expired {
					// The funding-expiration weight also covers stale or deferred checks.
					funding_expiration_count.saturating_accrue(1);
				}
			}

			let expiring_count = terminal_expirations.len() as u64;
			for lock_id in terminal_expirations {
				if let Err(error) = with_storage_layer(|| Self::burn_bitcoin_lock(lock_id, false)) {
					PendingLockExpirations::<T>::insert(lock_id, ());
					log::error!("Bitcoin lock {lock_id:?} failed terminal expiration: {error:?}");
					Self::deposit_event(Event::<T>::LockExpirationError { lock_id, error });
				}
			}

			(expiring_count, funding_expiration_count)
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

		/// Settle active Liquids at their current redemption cost and release the remaining
		/// securitization at its existing maturities.
		fn burn_bitcoin_lock(lock_id: BitcoinLockId, is_externally_spent: bool) -> DispatchResult {
			let lock = Self::take_lock(lock_id)?;

			if is_externally_spent {
				Self::clear_orphans_for_lock(lock_id, &lock)?;
			} else {
				Self::schedule_orphans_for_cleanup(lock_id, &lock);
			}

			let fissions =
				T::FissionsProvider::get_lock_fission_guarantees(&lock.owner_account, lock_id);
			let mut fissioned_satoshis = 0u64;
			let mut redemption = T::Balance::zero();
			for (basis, _) in &fissions {
				fissioned_satoshis.saturating_accrue(basis.satoshis);
				let amount = Self::calculate_redemption_amount_from_satoshis(
					&basis.satoshis,
					Some(basis.btc_value_in_microgons()),
				)?;
				redemption.saturating_accrue(amount);
			}
			ensure!(
				fissioned_satoshis == lock.fissioned_satoshis,
				Error::<T>::FissionStateMismatch
			);

			// Retire the whole Lock allocation, including any unused funding reservation.
			let mut guarantees = vec![(
				&lock.cosigner_guarantee,
				lock.funded_satoshis.saturating_sub(lock.fissioned_satoshis),
			)];
			guarantees
				.extend(fissions.iter().map(|(basis, guarantee)| (guarantee, basis.satoshis)));

			// Settle once so splitting guarantees cannot change which release holds survive.
			let burned = T::VaultProvider::burn(
				lock.vault_id,
				&guarantees,
				lock.vault_claim_height,
				lock.is_flexible,
				redemption,
			)
			.map_err(Error::<T>::from)?;

			T::FissionsProvider::close_for_lock(&lock.owner_account, lock_id, burned)?;

			Self::deposit_event(Event::BitcoinLockTerminated {
				lock_id,
				vault_id: lock.vault_id,
				was_utxo_spent: is_externally_spent,
				burned_argons: burned,
			});

			Ok(())
		}

		fn release_lock_securitization(lock: &LockedBitcoin<T>) -> DispatchResult {
			T::VaultProvider::release_bitcoin_lock_securitization(
				lock.vault_id,
				&lock.cosigner_guarantee,
				lock.funded_satoshis.saturating_sub(lock.fissioned_satoshis),
				lock.vault_claim_height,
				lock.is_flexible,
			)
			.map_err(|error| Error::<T>::from(error).into())
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
			let previous_position = lock.lock_held_securitization();

			ensure!(
				change_satoshis <= lock.funded_satoshis,
				Error::<T>::InvalidBitcoinReleaseAmount
			);
			ensure!(
				change_satoshis >= lock.fissioned_satoshis,
				Error::<T>::InsufficientSatoshisForFissions
			);

			let previous_funding = lock.funding_and_securitization();
			lock.funded_satoshis = change_satoshis;
			T::VaultProvider::record_bitcoin_lock_funding(
				lock.vault_id,
				previous_funding,
				lock.funding_and_securitization(),
			)
			.map_err(Error::<T>::from)?;

			Self::shrink_lock_securitization(&mut lock, false)?;

			for utxo_ref in lock.funding_utxos.keys() {
				T::BitcoinUtxoTracker::unwatch_utxo(lock_id, utxo_ref);
			}
			lock.funding_utxos.clear();
			lock.funding_utxos
				.try_insert(change_ref, change_satoshis)
				.map_err(|_| Error::<T>::MaxUtxosPerLockExceeded)?;

			let is_expired = T::BitcoinBlockHeightChange::get().1 >= lock.vault_claim_height;
			if LockReleaseRequestsById::<T>::contains_key(lock_id) {
				Self::take_release_request(lock_id)?;
			} else {
				PendingPartialReleaseByLockId::<T>::remove(lock_id);
			}

			T::PositionProvider::bitcoin_position_updated(
				&lock.owner_account,
				lock.vault_id,
				previous_position,
				lock.lock_held_securitization(),
			)?;

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
			Self::release_lock_securitization(&lock)?;
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

		fn take_lock(lock_id: BitcoinLockId) -> Result<LockedBitcoin<T>, DispatchError> {
			let lock = LocksById::<T>::get(lock_id).ok_or(Error::<T>::LockNotFound)?;
			if LockReleaseRequestsById::<T>::contains_key(lock_id) {
				Self::take_release_request(lock_id)?;
			}
			T::PositionProvider::bitcoin_position_updated(
				&lock.owner_account,
				lock.vault_id,
				lock.total_securitization(lock_id),
				BitcoinLockPosition::default(),
			)?;
			LocksById::<T>::remove(lock_id);
			PendingLockExpirations::<T>::remove(lock_id);
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
			let lock = LocksById::<T>::get(lock_id).ok_or(Error::<T>::LockNotFound)?;
			if entry.change_satoshis > 0 {
				// No Bitcoin has moved, and the remaining Lock may still back active Fissions.
				Self::take_release_request(lock_id)?;
				Self::deposit_event(Event::BitcoinCosignPastDue {
					lock_id,
					vault_id: lock.vault_id,
					release_number: entry.release_number,
					compensation_amount: T::Balance::zero(),
					compensated_account_id: lock.owner_account,
				});
				return Ok(())
			}
			ensure!(lock.fissioned_satoshis == 0, Error::<T>::LockHasActiveFissions);
			let lock = Self::take_lock(lock_id)?;
			let vault_id = lock.vault_id;

			let result = T::VaultProvider::compensate_lost_bitcoin(
				vault_id,
				&lock.owner_account,
				&lock.cosigner_guarantee,
				lock.funded_satoshis,
				lock.vault_claim_height,
				lock.is_flexible,
				entry.securitization_at_risk,
			)
			.map_err(Error::<T>::from)?;
			let compensated = result.to_beneficiary;

			Self::deposit_event(Event::BitcoinCosignPastDue {
				lock_id,
				vault_id,
				release_number: entry.release_number,
				compensation_amount: compensated,
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
			if let Some(deadline) = lock.securitization_hold_expiration_bitcoin_height {
				Self::unschedule_securitization_hold_expiration(lock_id, deadline);
			}

			Self::release_lock_securitization(&lock)?;
			Self::schedule_orphans_for_cleanup(lock_id, &lock);

			Ok(())
		}

		pub(crate) fn unschedule_securitization_hold_expiration(
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

		fn shrink_lock_securitization(
			lock: &mut LockedBitcoin<T>,
			release_unused_immediately: bool,
		) -> DispatchResult {
			if lock.securitization_hold_expiration_bitcoin_height.is_some() &&
				!release_unused_immediately
			{
				return Ok(());
			}

			let lock_funding = lock.funding_and_securitization();
			let retained_guarantee = lock
				.cosigner_guarantee
				.take_portion(
					lock_funding.securitized_satoshis,
					lock_funding.activated_securitization,
				)
				.ok_or(Error::<T>::OverflowError)?;

			T::VaultProvider::shrink_securitization(
				lock.vault_id,
				&lock.cosigner_guarantee,
				lock.vault_claim_height,
				release_unused_immediately,
			)
			.map_err(Error::<T>::from)?;

			lock.cosigner_guarantee = retained_guarantee;
			lock.securitization_hold_expiration_bitcoin_height = None;

			Ok(())
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
	type FeeCoupon = FeeCoupon<T>;

	fn fission_satoshis(
		account_id: &T::AccountId,
		lock_id: BitcoinLockId,
		satoshis: Satoshis,
		microgons_at_target_per_btc: T::Balance,
		fee_coupon: Option<Self::FeeCoupon>,
	) -> Result<(T::Balance, Tick, CosignerGuarantee<T::Balance>), BitcoinFissionLockError> {
		let mut lock = LocksById::<T>::get(lock_id).ok_or(BitcoinFissionLockError::LockNotFound)?;
		ensure!(lock.owner_account == *account_id, BitcoinFissionLockError::NoPermissions);
		ensure!(lock.is_funded(), BitcoinFissionLockError::LockNotFunded);
		ensure!(!Self::is_release_pending(lock_id), BitcoinFissionLockError::LockReleasePending);

		let tick = Self::latest_microgons_at_target_per_btc_tick(microgons_at_target_per_btc)
			.ok_or(BitcoinFissionLockError::IneligibleMicrogonsAtTargetPerBtc)?;
		let available_satoshis = lock.funded_satoshis.saturating_sub(lock.fissioned_satoshis);
		ensure!(
			satoshis <= available_satoshis,
			BitcoinFissionLockError::InsufficientFundedSatoshis
		);

		let remaining_satoshis = available_satoshis.saturating_sub(satoshis);
		let liquidity_promised =
			Self::calculate_liquidity_promised(satoshis, microgons_at_target_per_btc)?;

		let securitization_before_fission = lock.lock_held_securitization();
		let funding_before_fission = lock.funding_and_securitization();

		// Transfer only coverage that the Bitcoin remaining on the Lock does not need.
		let transferred_satoshis =
			funding_before_fission.securitized_satoshis.saturating_sub(remaining_satoshis);
		let transferred_microgons =
			lock.cosigner_guarantee.securitization_for_satoshis(transferred_satoshis);
		let mut fission_guarantee = lock
			.cosigner_guarantee
			.take_portion(transferred_satoshis, transferred_microgons)
			.ok_or(BitcoinFissionLockError::Overflow)?;

		// Transfer the existing guarantee before purchasing any additional securitization.
		lock.fissioned_satoshis.saturating_accrue(satoshis);
		let mut funding_after_fission = lock.funding_and_securitization();
		funding_after_fission
			.securitized_satoshis
			.saturating_accrue(fission_guarantee.basis.satoshis);
		funding_after_fission
			.activated_securitization
			.saturating_accrue(fission_guarantee.guaranteed_microgons);

		T::VaultProvider::record_bitcoin_lock_funding(
			lock.vault_id,
			funding_before_fission,
			funding_after_fission,
		)
		.map_err(|_| BitcoinFissionLockError::ResecuritizationFailed)?;

		let securitization_shortfall =
			liquidity_promised.saturating_sub(fission_guarantee.guaranteed_microgons);

		if !securitization_shortfall.is_zero() {
			let purchased_satoshis = multiply_by_rational_with_rounding(
				u128::from(satoshis),
				securitization_shortfall.into(),
				liquidity_promised.into(),
				Rounding::Up,
			)
			.and_then(|satoshis| Satoshis::try_from(satoshis).ok())
			.ok_or(BitcoinFissionLockError::Overflow)?;

			let additional_guarantee = CosignerGuarantee {
				basis: BitcoinSecuritizationBasis {
					satoshis: purchased_satoshis,
					microgons_at_target_per_btc,
					tick,
				},
				guaranteed_microgons: securitization_shortfall,
				fund_hold_extensions: Default::default(),
			};

			let transferred_guarantee = fission_guarantee.clone();
			Self::merge_guarantees(&mut fission_guarantee, &additional_guarantee)?;
			fission_guarantee.basis.satoshis = fission_guarantee.basis.satoshis.min(satoshis);

			// The Vault assigns hold extensions when it reallocates securitization.
			fission_guarantee.fund_hold_extensions.clear();

			let coupon = Self::validate_fee_coupon(
				lock.vault_id,
				account_id,
				Some(lock_id),
				satoshis,
				microgons_at_target_per_btc,
				fee_coupon.as_ref(),
			)
			.map_err(|_| BitcoinFissionLockError::InvalidFeeCoupon)?;

			Self::replace_guarantee(
				&mut lock,
				&transferred_guarantee,
				&mut fission_guarantee,
				satoshis,
				coupon,
			)
			.map_err(|error| match error {
				Error::<T>::InsufficientFunds => BitcoinFissionLockError::InsufficientFunds,
				_ => BitcoinFissionLockError::ResecuritizationFailed,
			})?;
		}

		let mut securitization_after_fission = lock.lock_held_securitization();
		securitization_after_fission
			.allocated_securitization
			.saturating_accrue(fission_guarantee.guaranteed_microgons);
		securitization_after_fission
			.activated_securitization
			.saturating_accrue(fission_guarantee.guaranteed_microgons);

		T::PositionProvider::bitcoin_position_updated(
			account_id,
			lock.vault_id,
			securitization_before_fission,
			securitization_after_fission,
		)
		.map_err(|_| BitcoinFissionLockError::Overflow)?;

		LocksById::<T>::insert(lock_id, lock);

		Ok((liquidity_promised, tick, fission_guarantee))
	}

	fn update_fission_guarantee(
		account_id: &T::AccountId,
		lock_id: BitcoinLockId,
		satoshis: Satoshis,
		fission_guarantee: &mut CosignerGuarantee<T::Balance>,
		microgons_at_target_per_btc: T::Balance,
		minimum_last_ratchet_tick: Tick,
		fee_coupon: Option<Self::FeeCoupon>,
	) -> Result<Tick, BitcoinFissionLockError> {
		let mut lock = LocksById::<T>::get(lock_id).ok_or(BitcoinFissionLockError::LockNotFound)?;
		ensure!(lock.owner_account == *account_id, BitcoinFissionLockError::NoPermissions);
		ensure!(!Self::is_release_pending(lock_id), BitcoinFissionLockError::LockReleasePending);

		let tick = Self::latest_microgons_at_target_per_btc_tick(microgons_at_target_per_btc)
			.ok_or(BitcoinFissionLockError::IneligibleMicrogonsAtTargetPerBtc)?;
		ensure!(
			tick >= minimum_last_ratchet_tick,
			BitcoinFissionLockError::MicrogonsAtTargetPerBtcTickOlderThanCurrent
		);
		ensure!(
			satoshis <= lock.fissioned_satoshis,
			BitcoinFissionLockError::InsufficientFissionedSatoshis
		);

		let liquidity_promised =
			Self::calculate_liquidity_promised(satoshis, microgons_at_target_per_btc)?;

		// Replace the guarantee when its price cap or coverage cannot support the new terms.
		if microgons_at_target_per_btc > fission_guarantee.basis.microgons_at_target_per_btc ||
			liquidity_promised > fission_guarantee.guaranteed_microgons
		{
			let previous_fission_securitization = BitcoinLockPosition {
				allocated_securitization: fission_guarantee.guaranteed_microgons,
				activated_securitization: fission_guarantee.guaranteed_microgons,
			};
			let existing_fission_guarantee = fission_guarantee.clone();
			let mut replacement_fission_guarantee = CosignerGuarantee {
				basis: BitcoinSecuritizationBasis { satoshis, microgons_at_target_per_btc, tick },
				guaranteed_microgons: liquidity_promised,
				fund_hold_extensions: Default::default(),
			};
			let replacement_fission_securitization = BitcoinLockPosition {
				allocated_securitization: liquidity_promised,
				activated_securitization: liquidity_promised,
			};

			let coupon = Self::validate_fee_coupon(
				lock.vault_id,
				account_id,
				Some(lock_id),
				satoshis,
				microgons_at_target_per_btc,
				fee_coupon.as_ref(),
			)
			.map_err(|_| BitcoinFissionLockError::InvalidFeeCoupon)?;

			Self::replace_guarantee(
				&mut lock,
				&existing_fission_guarantee,
				&mut replacement_fission_guarantee,
				satoshis,
				coupon,
			)
			.map_err(|error| match error {
				Error::<T>::InsufficientFunds => BitcoinFissionLockError::InsufficientFunds,
				_ => BitcoinFissionLockError::ResecuritizationFailed,
			})?;

			*fission_guarantee = replacement_fission_guarantee;

			T::PositionProvider::bitcoin_position_updated(
				account_id,
				lock.vault_id,
				previous_fission_securitization,
				replacement_fission_securitization,
			)
			.map_err(|_| BitcoinFissionLockError::Overflow)?;

			LocksById::<T>::insert(lock_id, lock);
		}

		Ok(tick)
	}

	fn calculate_liquidity_promised(
		satoshis: Satoshis,
		microgons_at_target_per_btc: T::Balance,
	) -> Result<T::Balance, BitcoinFissionLockError> {
		let btc_value_at_target_in_microgons =
			FixedU128::from_rational(satoshis as u128, SATOSHIS_PER_BITCOIN as u128)
				.saturating_mul_int(microgons_at_target_per_btc);
		Self::calculate_redemption_amount(btc_value_at_target_in_microgons, None)
			.map_err(|_| BitcoinFissionLockError::Overflow)
	}

	fn fuse_satoshis(
		account_id: &T::AccountId,
		lock_id: BitcoinLockId,
		satoshis: Satoshis,
		fission_guarantee: CosignerGuarantee<T::Balance>,
		microgons_at_target_per_btc: T::Balance,
	) -> Result<T::Balance, BitcoinFissionLockError> {
		let mut lock = LocksById::<T>::get(lock_id).ok_or(BitcoinFissionLockError::LockNotFound)?;
		ensure!(lock.owner_account == *account_id, BitcoinFissionLockError::NoPermissions);

		let mut previous_securitization = lock.lock_held_securitization();
		previous_securitization
			.allocated_securitization
			.saturating_accrue(fission_guarantee.guaranteed_microgons);
		previous_securitization
			.activated_securitization
			.saturating_accrue(fission_guarantee.guaranteed_microgons);

		let mut previous_funding = lock.funding_and_securitization();
		previous_funding
			.securitized_satoshis
			.saturating_accrue(fission_guarantee.basis.satoshis);
		previous_funding.activated_securitization =
			previous_securitization.activated_securitization;

		ensure!(
			satoshis <= lock.fissioned_satoshis,
			BitcoinFissionLockError::InsufficientFissionedSatoshis
		);

		// The Bitcoin stays funded in the Lock; closing frees its Fission allocation.
		lock.fissioned_satoshis.saturating_reduce(satoshis);

		let fission_btc_value_in_microgons = BitcoinSecuritizationBasis {
			satoshis,
			microgons_at_target_per_btc,
			..fission_guarantee.basis
		}
		.btc_value_in_microgons();
		let redemption = Self::calculate_redemption_amount_from_satoshis(
			&satoshis,
			Some(fission_btc_value_in_microgons),
		)
		.map_err(|_| BitcoinFissionLockError::NoBitcoinPricesAvailable)?;

		Self::merge_guarantees(&mut lock.cosigner_guarantee, &fission_guarantee)?;
		let merged_funding = lock.funding_and_securitization();

		Self::renew_securitization_hold(lock_id, &mut lock)
			.map_err(|_| BitcoinFissionLockError::ResecuritizationFailed)?;

		let merged_securitization = lock.lock_held_securitization();
		T::VaultProvider::record_bitcoin_lock_funding(
			lock.vault_id,
			previous_funding,
			merged_funding,
		)
		.map_err(|_| BitcoinFissionLockError::ResecuritizationFailed)?;

		T::PositionProvider::bitcoin_position_updated(
			account_id,
			lock.vault_id,
			previous_securitization,
			merged_securitization,
		)
		.map_err(|_| BitcoinFissionLockError::Overflow)?;

		LocksById::<T>::insert(lock_id, lock);

		Ok(redemption)
	}
}

impl<T: Config> Pallet<T> {
	fn renew_securitization_hold(
		lock_id: BitcoinLockId,
		lock: &mut LockedBitcoin<T>,
	) -> Result<(), Error<T>> {
		if let Some(height) = lock.securitization_hold_expiration_bitcoin_height.take() {
			Self::unschedule_securitization_hold_expiration(lock_id, height);
		}

		// New deposits can activate the combined guarantee while this window is open.
		if lock.cosigner_guarantee.basis.satoshis >
			lock.funded_satoshis.saturating_sub(lock.fissioned_satoshis)
		{
			let height = T::BitcoinBlockHeightChange::get()
				.1
				.saturating_add(T::SecuritizationHoldBlocks::get())
				.saturating_add(1);
			SecuritizationHoldExpirationsByBitcoinHeight::<T>::try_mutate(height, |locks| {
				locks.try_insert(lock_id)
			})
			.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;
			lock.securitization_hold_expiration_bitcoin_height = Some(height);
		}

		Ok(())
	}

	fn replace_guarantee(
		lock: &mut LockedBitcoin<T>,
		existing_guarantee: &CosignerGuarantee<T::Balance>,
		replacement_guarantee: &mut CosignerGuarantee<T::Balance>,
		funded_satoshis: Satoshis,
		coupon: Option<&FeeCoupon<T>>,
	) -> Result<(), Error<T>> {
		let lock_term_blocks =
			lock.vault_claim_height.saturating_sub(lock.created_at_height).max(1);
		let remaining_lock_blocks =
			lock.vault_claim_height.saturating_sub(T::BitcoinBlockHeightChange::get().1);

		let (fee, coupon_paid) = T::VaultProvider::resecuritize(
			lock.vault_id,
			&lock.owner_account,
			existing_guarantee,
			replacement_guarantee,
			funded_satoshis,
			ReserveSecuritizationRequest {
				remaining_term: FixedU128::from_rational(
					u128::from(remaining_lock_blocks),
					u128::from(lock_term_blocks),
				),
				lock_expiration: lock.vault_claim_height,
				is_flexible: lock.is_flexible,
				fee_discount: coupon.map(|coupon| coupon.fee_discount).unwrap_or_default(),
				securitization_space_to_unreserve: coupon
					.map(|coupon| coupon.securitization_space_to_unreserve)
					.unwrap_or_default(),
			},
		)
		.map_err(Error::<T>::from)?;

		lock.security_fees.saturating_accrue(fee);
		lock.coupon_paid_fees.saturating_accrue(coupon_paid);

		if let Some(coupon) = coupon {
			LastFeeCouponNonceByVaultAndAccount::<T>::insert(
				lock.vault_id,
				&lock.owner_account,
				coupon.nonce,
			);
		}

		Ok(())
	}

	/// Combine paid securitization and hold extensions at the better guarantee-per-BTC rate.
	fn merge_guarantees(
		retained_guarantee: &mut CosignerGuarantee<T::Balance>,
		incoming_guarantee: &CosignerGuarantee<T::Balance>,
	) -> Result<(), BitcoinFissionLockError> {
		if retained_guarantee.guaranteed_microgons.is_zero() {
			*retained_guarantee = incoming_guarantee.clone();
			return Ok(());
		}

		if incoming_guarantee.guaranteed_microgons.is_zero() {
			return Ok(());
		}

		let retained_microgons_per_satoshi = Rational128::from(
			retained_guarantee.guaranteed_microgons.into(),
			retained_guarantee.basis.satoshis.into(),
		);
		let incoming_microgons_per_satoshi = Rational128::from(
			incoming_guarantee.guaranteed_microgons.into(),
			incoming_guarantee.basis.satoshis.into(),
		);
		let better_coverage_guarantee =
			if incoming_microgons_per_satoshi > retained_microgons_per_satoshi {
				incoming_guarantee
			} else {
				&*retained_guarantee
			};

		let combined_guaranteed_microgons = retained_guarantee
			.guaranteed_microgons
			.saturating_add(incoming_guarantee.guaranteed_microgons);
		let combined_guarantee_capacity_satoshis = multiply_by_rational_with_rounding(
			combined_guaranteed_microgons.into(),
			better_coverage_guarantee.basis.satoshis.into(),
			better_coverage_guarantee.guaranteed_microgons.into(),
			Rounding::Down,
		)
		.and_then(|combined_guarantee_capacity_satoshis| {
			Satoshis::try_from(combined_guarantee_capacity_satoshis).ok()
		})
		.ok_or(BitcoinFissionLockError::Overflow)?;

		// Keep the higher Bitcoin price cap; funding uses the better guarantee-per-BTC rate.
		if incoming_guarantee.basis.microgons_at_target_per_btc >
			retained_guarantee.basis.microgons_at_target_per_btc
		{
			retained_guarantee.basis = incoming_guarantee.basis;
		}

		retained_guarantee.basis.satoshis = combined_guarantee_capacity_satoshis;
		retained_guarantee.guaranteed_microgons = combined_guaranteed_microgons;

		// Improved coverage keeps the existing release dates on all paid securitization.
		for (release_height, hold_microgons) in incoming_guarantee.fund_hold_extensions.iter() {
			let combined_hold_microgons = retained_guarantee
				.fund_hold_extensions
				.get(release_height)
				.copied()
				.unwrap_or_default()
				.saturating_add(*hold_microgons);
			retained_guarantee
				.fund_hold_extensions
				.try_insert(*release_height, combined_hold_microgons)
				.map_err(|_| BitcoinFissionLockError::Overflow)?;
		}

		Ok(())
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

impl<T: Config> BitcoinLockPositionProvider<T::AccountId, T::Balance> for Pallet<T> {
	type Weights = ProviderWeightAdapter<T>;
	/// Seed a newly registered account's upstream subset from locks that predate registration.
	/// Later lock mutations update Treasury Positions directly; frame calculators do not call this.
	fn account_position(
		account: &T::AccountId,
		vault: VaultId,
	) -> Result<BitcoinLockPosition<T::Balance>, DispatchError> {
		let mut position = BitcoinLockPosition::<T::Balance>::default();
		for (id, ()) in LockIdsByOwnerAccount::<T>::iter_prefix(account) {
			if let Some(lock) = LocksById::<T>::get(id).filter(|lock| lock.vault_id == vault) {
				let contribution = lock.total_securitization(id);
				position
					.activated_securitization
					.saturating_accrue(contribution.activated_securitization);
				position
					.allocated_securitization
					.saturating_accrue(contribution.allocated_securitization);
			}
		}
		Ok(position)
	}
}
