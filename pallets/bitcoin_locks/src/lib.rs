#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::zero_prefixed_literal)]

extern crate alloc;
extern crate core;

use alloc::vec::Vec;
use pallet_prelude::*;

use argon_bitcoin::CosignReleaser;
use argon_primitives::bitcoin::{BitcoinNetwork, BitcoinSignature, CompressedBitcoinPubkey};
pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

pub mod migrations;
#[cfg(test)]
mod tests;
pub mod weights;

/// The bitcoin locks pallet allows users to manage the lifecycle of Bitcoin locks. Bitcoin Locks
/// lock up argons for a pre-defined period of time for a fee. A vault issuer can determine that fee
/// on their own.
///
/// ** Vaults: **
/// Vaults are managed in the vault pallet, but determine the amount of funding eligible for
/// locking.
///
/// ** Bitcoin Locks: **
///
/// Bitcoin locks allow a user to mint new argons equal to the current market price of the locked
/// UTXO's satoshis. The lock must lock up the equivalent argons for a year's time. At any time
/// during the locked year, a Bitcoin holder is eligible to release their
/// bitcoin. To release a bitcoin, a user must pay back the current market price of bitcoin (capped
/// at their locked price). Should they move their UTXO via the bitcoin network, the current value
/// of the UTXO will be burned from the vault funds.
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
		BitcoinUtxoEvents, BitcoinUtxoTracker, MICROGONS_PER_ARGON, PriceProvider,
		TransactionSponsorProvider, TxSponsor, UtxoLockEvents, VaultId,
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinScriptPubkey, BitcoinSignature,
			CompressedBitcoinPubkey, SATOSHIS_PER_BITCOIN, Satoshis, UtxoId, UtxoRef,
			XPubChildNumber, XPubFingerprint,
		},
		vault::{BitcoinVaultProvider, LockExtension, Securitization, VaultError},
	};
	use codec::{EncodeLike, HasCompact};
	use core::iter::Sum;
	use polkadot_sdk::frame_support::traits::IsSubType;
	use sp_core::sr25519;
	use sp_runtime::traits::Verify;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(6);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

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

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		type LockEvents: UtxoLockEvents<Self::AccountId, Self::Balance>;

		/// Utxo tracker for bitcoin
		type BitcoinUtxoTracker: BitcoinUtxoTracker;

		type PriceProvider: PriceProvider<Self::Balance>;

		type BitcoinSignatureVerifier: BitcoinVerifier<Self>;

		/// Bitcoin time provider
		type BitcoinBlockHeightChange: Get<(BitcoinHeight, BitcoinHeight)>;

		type GetBitcoinNetwork: Get<BitcoinNetwork>;

		type VaultProvider: BitcoinVaultProvider<AccountId = Self::AccountId, Balance = Self::Balance>;

		/// Argon tick per day
		#[pallet::constant]
		type ArgonTicksPerDay: Get<Tick>;

		/// Maximum releasing utxos at a time
		#[pallet::constant]
		type MaxConcurrentlyReleasingLocks: Get<u32>;

		/// The number of bitcoin blocks a bitcoin is locked for
		#[pallet::constant]
		type LockDurationBlocks: Get<BitcoinHeight>;

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

	/// A reason for the pallet placing a hold on funds.
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

	/// Stores the block number where the lock was released
	#[pallet::storage]
	pub type LockReleaseCosignHeightById<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, BlockNumberFor<T>, OptionQuery>;

	/// Stores bitcoin locks that have requested to be released
	#[pallet::storage]
	pub type LockReleaseRequestsByUtxoId<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, LockReleaseRequest<T::Balance>, OptionQuery>;

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

	/// The minimum number of satoshis that can be locked
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

	/// History of microgons per btc
	#[pallet::storage]
	pub type MicrogonPerBtcHistory<T: Config> =
		StorageValue<_, BoundedVec<(Tick, T::Balance), T::MaxBtcPriceTickAge>, ValueQuery>;

	/// Fee Coupons
	#[pallet::storage]
	pub type FeeCouponsByPublic<T: Config> =
		StorageMap<_, Blake2_128Concat, sr25519::Public, FeeCoupon<T::Balance>, OptionQuery>;

	/// Fee Coupon Expirations
	#[pallet::storage]
	pub type FeeCouponsExpiringByFrame<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		FrameId,
		Blake2_128Concat,
		sr25519::Public,
		(),
		OptionQuery,
	>;

	#[derive(Decode, Encode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct LockedBitcoin<T: Config> {
		#[codec(compact)]
		pub vault_id: VaultId,
		/// The mintable liquidity of this lock, in microgons
		#[codec(compact)]
		pub liquidity_promised: T::Balance,
		/// The market rate of the satoshis locked, adjusted for any inflation offset of the argon
		#[codec(compact)]
		pub locked_market_rate: T::Balance,
		/// The owner account
		pub owner_account: T::AccountId,
		/// The guaranteed securitization ratio for this lock
		pub securitization_ratio: FixedU128,
		/// Sum of all lock fees (initial plus any ratcheting)
		#[codec(compact)]
		pub security_fees: T::Balance,
		/// Fees paid using coupons for this lock
		#[codec(compact)]
		pub coupon_paid_fees: T::Balance,
		/// The number of satoshis reserved for this lock
		#[codec(compact)]
		pub satoshis: Satoshis,
		/// The number of satoshis in the funding utxo (allowed some variance from the `satoshis`
		/// field)
		pub utxo_satoshis: Option<Satoshis>,
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
		/// Whether this lock has been funded (confirmed) on-bitcoin
		pub is_funded: bool,
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
		pub fn effective_satoshis(&self) -> Satoshis {
			self.utxo_satoshis.unwrap_or(self.satoshis)
		}

		pub fn get_securitization(&self) -> Securitization<T::Balance> {
			Securitization::new(self.liquidity_promised, self.securitization_ratio)
		}
	}

	#[derive(
		Decode,
		Encode,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
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
		/// The price at which the bitcoin is being released (to be confiscated from vault it not
		/// cosigned)
		#[codec(compact)]
		pub redemption_price: Balance,
	}

	#[derive(
		Decode,
		Encode,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
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
		Decode,
		Encode,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
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

	#[derive(
		Decode,
		DecodeWithMemTracking,
		Encode,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct FeeCouponProof {
		/// The public id of the fee
		pub public: sr25519::Public,
		/// The signature over the prepaid id and amount
		pub signature: sr25519::Signature,
	}

	pub const FEE_PROOF_MESSAGE_KEY: &[u8; 17] = b"fee_proof_message";

	impl FeeCouponProof {
		pub fn verify<AccountId: EncodeLike>(&self, account_id: &AccountId) -> bool {
			let message =
				(FEE_PROOF_MESSAGE_KEY, self.public, account_id).using_encoded(blake2_256);
			self.signature.verify(message.as_slice(), &self.public)
		}
	}

	#[derive(
		Decode,
		DecodeWithMemTracking,
		Encode,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct FeeCoupon<Balance: Clone + Eq + PartialEq + TypeInfo + Codec + MaxEncodedLen> {
		/// The vault id this coupon is associated with
		#[codec(compact)]
		pub vault_id: VaultId,
		/// The max number of satoshis this coupon can be used for
		#[codec(compact)]
		pub max_satoshis: Satoshis,
		/// The expiration frame of this coupon
		#[codec(compact)]
		pub expiration_frame: FrameId,
		/// Optional maximum fee + tip that can be used with this coupon
		pub max_fee_plus_tip: Option<Balance>,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		BitcoinLockCreated {
			utxo_id: UtxoId,
			vault_id: VaultId,
			liquidity_promised: T::Balance,
			securitization: T::Balance,
			locked_market_rate: T::Balance,
			account_id: T::AccountId,
			security_fee: T::Balance,
		},
		BitcoinLockRatcheted {
			utxo_id: UtxoId,
			vault_id: VaultId,
			liquidity_promised: T::Balance,
			original_market_rate: T::Balance,
			security_fee: T::Balance,
			new_locked_market_rate: T::Balance,
			amount_burned: T::Balance,
			account_id: T::AccountId,
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
		UtxoFundedFromCandidate {
			utxo_id: UtxoId,
			utxo_ref: UtxoRef,
			vault_id: VaultId,
			account_id: T::AccountId,
		},
		SecuritizationIncreased {
			utxo_id: UtxoId,
			vault_id: VaultId,
			new_satoshis: Satoshis,
			account_id: T::AccountId,
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
		/// The minimum number of satoshis was not met
		InsufficientSatoshisLocked,
		/// The price provider has no bitcoin prices available. This is a temporary error
		NoBitcoinPricesAvailable,
		/// The bitcoin script to lock this bitcoin has errors
		InvalidBitcoinScript,
		/// The user does not have permissions to perform this action
		NoPermissions,
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
		/// Nothing to ratchet
		NoRatchetingAvailable,
		/// A lock in process of release cannot be ratcheted
		LockInProcessOfRelease,
		/// The lock funding has not been confirmed on bitcoin
		LockPendingFunding,
		/// An overflow or underflow occurred while calculating the redemption price
		OverflowError,
		/// An ineligible microgon rate per btc was requested
		IneligibleMicrogonRateRequested,
		/// The provided fee coupon is already used or invalid
		InvalidFeeCoupon,
		/// The provided fee coupon proof is invalid
		InvalidFeeCouponProof,
		/// This bitcoin lock exceeded the maximum allowed number of satoshis for the provided fee
		/// coupon
		MaxFeeCouponSatoshisExceeded,
		/// The fee coupon already exists
		FeeCouponAlreadyExists,
		/// Initializing a lock for another account requires a fee coupon
		FeeCouponRequired,
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
		/// The minimum number of satoshis that can be locked
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
			let expirations = (start_bitcoin_height..=bitcoin_block_height)
				.flat_map(LockExpirationsByBitcoinHeight::<T>::take);
			// if a lock expires, we need to burn the funds
			let mut expiring_count: u64 = 0;
			for utxo_id in expirations {
				expiring_count = expiring_count.saturating_add(1);
				let res = with_storage_layer(|| {
					Self::burn_bitcoin_lock(utxo_id, false)?;
					Ok(())
				});
				if let Err(e) = res {
					log::error!("Bitcoin utxo id {:?} failed to be burned {:?}", utxo_id, e);
					Self::deposit_event(Event::<T>::LockExpirationError { utxo_id, error: e });
				}
			}

			let overdue = LockCosignDueByFrame::<T>::take(T::CurrentFrameId::get());
			let overdue_count = overdue.len() as u64;
			for utxo_id in overdue {
				let res = with_storage_layer(|| Self::cosign_bitcoin_overdue(utxo_id));
				if let Err(e) = res {
					log::error!(
						"Bitcoin lock id {:?} failed to handle overdue `cosign` {:?}",
						utxo_id,
						e
					);
					Self::deposit_event(Event::<T>::CosignOverdueError { utxo_id, error: e });
				}
			}

			let expiring = OrphanedUtxoExpirationByFrame::<T>::take(T::CurrentFrameId::get());
			let orphan_expiring_count = expiring.len() as u64;
			for (account_id, utxo_ref) in expiring {
				let res: Result<(), DispatchError> = with_storage_layer(|| {
					if let Some(request) = OrphanedUtxosByAccount::<T>::take(&account_id, &utxo_ref)
					{
						if request.cosign_request.is_some() {
							T::VaultProvider::update_orphan_cosign_list(
								request.vault_id,
								request.utxo_id,
								&account_id,
								true,
							)
							.map_err(Error::<T>::from)?;
						}
					}
					Ok::<(), DispatchError>(())
				});
				if let Err(e) = res {
					log::error!(
						"Orphaned bitcoin utxo {:?} failed expiry cleanup {:?}",
						utxo_ref,
						e
					);
				}
			}

			let extra_items = expiring_count
				.saturating_add(overdue_count)
				.saturating_add(orphan_expiring_count);
			T::DbWeight::get()
				.reads_writes(2, 1)
				.saturating_add(T::DbWeight::get().reads_writes(extra_items, extra_items))
		}

		fn on_finalize(_n: BlockNumberFor<T>) {
			let current_tick = T::CurrentTick::get();
			let oldest_allowed_tick =
				current_tick.saturating_sub(T::MaxBtcPriceTickAge::get() as Tick);
			let current_price = T::PriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN);

			MicrogonPerBtcHistory::<T>::mutate(|x| {
				x.retain(|y| y.0 >= oldest_allowed_tick);
				if let Some(price) = current_price {
					let mut should_insert = true;
					if let Some((_, last)) = x.last() {
						should_insert = *last != price;
					}
					if should_insert {
						_ = x.try_push((current_tick, price));
					}
				}
			});

			for (prepaid_id, _) in
				FeeCouponsExpiringByFrame::<T>::drain_prefix(T::CurrentFrameId::get())
			{
				FeeCouponsByPublic::<T>::remove(prepaid_id);
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
		RuntimeDebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub enum LockOptions<T: Config> {
		V1 {
			/// The microgons per btc rate to use for this lock
			microgons_per_btc: Option<T::Balance>,
			/// Proof for the use of a fee coupon
			fee_coupon_proof: Option<FeeCouponProof>,
		},
	}

	impl<T: Config> LockOptions<T> {
		pub fn microgons_per_btc(&self) -> Option<T::Balance> {
			match self {
				LockOptions::<T>::V1 { microgons_per_btc, .. } => *microgons_per_btc,
			}
		}

		pub fn fee_coupon_proof(&self) -> Option<&FeeCouponProof> {
			match self {
				LockOptions::<T>::V1 { fee_coupon_proof, .. } => fee_coupon_proof.as_ref(),
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Initialize a bitcoin lock. This will create a LockedBitcoin for the submitting account
		/// and log the Bitcoin Script hash to Events.
		///
		/// The pubkey submitted here will be used to create a script pubkey that will be used in a
		/// timelock multisig script to lock the bitcoin.
		///
		/// NOTE: A "lock-er" must send btc to the cosigner UTXO address to "complete" the
		/// LockedBitcoin and be added to the Bitcoin Mint line.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::initialize())]
		pub fn initialize(
			origin: OriginFor<T>,
			vault_id: VaultId,
			#[pallet::compact] satoshis: Satoshis,
			bitcoin_pubkey: CompressedBitcoinPubkey,
			options: Option<LockOptions<T>>,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			Self::create_bitcoin_lock(&account_id, vault_id, satoshis, bitcoin_pubkey, options)?;
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
			if !lock.is_funded {
				Self::cancel_lock(utxo_id, &lock)?;
				return Ok(());
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

			let mut redemption_price = T::Balance::zero();

			// If this is a confirmed utxo, we require the release price to be paid
			if lock.is_funded {
				// we have to take the fee out of the locked satoshis, so this won't work
				ensure!(
					bitcoin_network_fee < lock.effective_satoshis(),
					Error::<T>::BitcoinFeeTooHigh
				);
				redemption_price =
					Self::get_redemption_price(&lock.satoshis, Some(lock.locked_market_rate))?;
				// hold funds until the utxo is seen in the chain
				let balance = T::Currency::balance(&who);
				ensure!(
					balance.saturating_sub(redemption_price) >= T::Currency::minimum_balance(),
					Error::<T>::AccountWouldGoBelowMinimumBalance
				);

				frame_system::Pallet::<T>::inc_providers(&who);
				T::Currency::hold(&HoldReason::ReleaseBitcoinLock.into(), &who, redemption_price)
					.map_err(|e| match e {
					Token(TokenError::BelowMinimum) =>
						Error::<T>::AccountWouldGoBelowMinimumBalance,
					_ => Error::<T>::InsufficientFunds,
				})?;
			}

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
					redemption_price,
				},
			);

			LockCosignDueByFrame::<T>::try_mutate(cosign_due_frame, |a| a.try_insert(utxo_id))
				.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;
			T::VaultProvider::update_pending_cosign_list(vault_id, utxo_id, false)
				.map_err(Error::<T>::from)?;

			Self::deposit_event(Event::<T>::BitcoinUtxoCosignRequested { utxo_id, vault_id });
			Ok(())
		}

		/// Submitted by a Vault operator to cosign the release of a bitcoin utxo. The Bitcoin owner
		/// release fee will be burned, and the lock will be allowed to expire without a penalty.
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
			let lock_extension = lock.get_lock_extension();
			let utxo_satoshis = lock.effective_satoshis();
			let securitization = lock.get_securitization();
			let vault_id = lock.vault_id;
			let vault_pubkey = lock.vault_pubkey;
			let owner_account = lock.owner_account.clone();

			ensure!(T::VaultProvider::is_owner(vault_id, &who), Error::<T>::NoPermissions);
			let request = Self::take_release_request(utxo_id)?;

			let utxo_ref = T::BitcoinUtxoTracker::get_funding_utxo_ref(utxo_id)
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
				utxo_satoshis,
				utxo_ref.txid.into(),
				utxo_ref.output_index,
				ReleaseStep::VaultCosign,
				Amount::from_sat(request.bitcoin_network_fee),
				request.to_script_pubkey.into(),
				T::GetBitcoinNetwork::get().into(),
			)
			.map_err(|_| Error::<T>::BitcoinUnableToBeDecodedForRelease)?;

			let is_valid =
				T::BitcoinSignatureVerifier::verify_signature(releaser, vault_pubkey, &signature)?;
			ensure!(is_valid, Error::<T>::BitcoinInvalidCosignature);

			// burn the owner's held funds
			let burn_amount = request.redemption_price;
			let _ = T::Currency::burn_held(
				&HoldReason::ReleaseBitcoinLock.into(),
				&owner_account,
				burn_amount,
				Precision::Exact,
				Fortitude::Force,
			)?;
			frame_system::Pallet::<T>::dec_providers(&owner_account)?;
			T::LockEvents::utxo_released(utxo_id, false, burn_amount)?;

			T::VaultProvider::schedule_for_release(
				vault_id,
				&securitization,
				lock.satoshis,
				&lock_extension,
			)
			.map_err(Error::<T>::from)?;

			LockReleaseCosignHeightById::<T>::insert(
				utxo_id,
				frame_system::Pallet::<T>::block_number(),
			);

			Self::schedule_orphans_for_cleanup(utxo_id, &lock);
			T::BitcoinUtxoTracker::unwatch(utxo_id);

			Self::deposit_event(Event::BitcoinUtxoCosigned { utxo_id, vault_id, signature });

			// no fee for cosigning
			Ok(())
		}

		/// Ratcheting allows a user to change the lock price of their bitcoin lock. This is
		/// functionally the same as releasing and re-initializing, but it allows a user to skip
		/// sending transactions through bitcoin and any associated fees. It also allows you to stay
		/// on your original lock expiration without having to pay the full year of fees again.
		///
		/// Ratcheting "down" - when the price of bitcoin is lower than your lock price, you pay the
		/// full release price and get added back to the mint queue at the current market rate. You
		/// pocket the difference between the already minted "lock price" and the new market value
		/// (which you just had burned). Your new lock price is set to the market low, so you can
		/// take advantage of ratchets "up" in the future.
		///
		/// Ratcheting "up" - when the price of bitcoin is higher than your lock price, you pay a
		/// prorated fee for the remainder of your existing lock duration. You are added to the mint
		/// queue for the difference in your new lock price vs the previous lock price.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::ratchet())]
		pub fn ratchet(
			origin: OriginFor<T>,
			utxo_id: UtxoId,
			options: Option<LockOptions<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			ensure!(lock.owner_account == who, Error::<T>::NoPermissions);
			ensure!(lock.is_funded, Error::<T>::LockPendingFunding);
			ensure!(
				!LockReleaseRequestsByUtxoId::<T>::contains_key(utxo_id),
				Error::<T>::LockInProcessOfRelease
			);
			let has_fee_coupon =
				if let Some(coupon_proof) = options.as_ref().and_then(|o| o.fee_coupon_proof()) {
					let coupon = FeeCouponsByPublic::<T>::take(coupon_proof.public)
						.ok_or(Error::<T>::InvalidFeeCoupon)?;
					ensure!(lock.vault_id == coupon.vault_id, Error::<T>::InvalidFeeCoupon);
					ensure!(
						coupon_proof.verify(&lock.owner_account),
						Error::<T>::InvalidFeeCouponProof
					);
					true
				} else {
					false
				};
			let new_liquidity_promised =
				if let Some(rate) = options.as_ref().and_then(|x| x.microgons_per_btc()) {
					Self::get_bitcoin_argons_at_rate(lock.satoshis, rate)?
				} else {
					T::PriceProvider::get_bitcoin_argon_price(lock.satoshis)
						.ok_or(Error::<T>::NoBitcoinPricesAvailable)?
				};

			let original_market_rate = lock.locked_market_rate;
			ensure!(
				original_market_rate != new_liquidity_promised,
				Error::<T>::NoRatchetingAvailable
			);

			let vault_id = lock.vault_id;
			let expiration_height = lock.vault_claim_height;
			let mut amount_burned = T::Balance::zero();
			let mut to_mint = new_liquidity_promised;

			let mut duration_for_new_funds = FixedU128::zero();
			if new_liquidity_promised > original_market_rate {
				to_mint = new_liquidity_promised - original_market_rate;

				let start_height = lock.created_at_height;
				let elapsed_blocks =
					T::BitcoinBlockHeightChange::get().1.saturating_sub(start_height);
				let full_term = expiration_height.saturating_sub(start_height);
				duration_for_new_funds =
					FixedU128::from_rational(elapsed_blocks as u128, full_term as u128);
			} else {
				let redemption_price =
					Self::get_redemption_price(&lock.satoshis, Some(original_market_rate))
						.map_err(|_| Error::<T>::NoBitcoinPricesAvailable)?;
				// first we'll burn the redemption price (just like a release)
				amount_burned = redemption_price;

				// NOTE: we send redemption price to be released
				T::LockEvents::utxo_released(utxo_id, false, redemption_price)?;
				let _ = T::Currency::burn_from(
					&who,
					amount_burned,
					Preservation::Expendable,
					Precision::Exact,
					Fortitude::Force,
				)?;
				// add liquidity back to the vault by scheduling this for release
				T::VaultProvider::schedule_for_release(
					vault_id,
					&lock.get_securitization(),
					0,
					&lock.get_lock_extension(),
				)
				.map_err(Error::<T>::from)?;
			}

			let mut lock_extension = lock.get_lock_extension();
			let securitization = Securitization::new(to_mint, lock.securitization_ratio);
			let fee = T::VaultProvider::lock(
				vault_id,
				&who,
				&securitization,
				0,
				Some((duration_for_new_funds, &mut lock_extension)),
				has_fee_coupon,
			)
			.map_err(Error::<T>::from)?;

			lock.security_fees.saturating_accrue(fee);
			lock.fund_hold_extensions = lock_extension.extended_expiration_funds.clone();
			let new_market_rate = Self::calculate_adjusted_market_rate(new_liquidity_promised)?;
			lock.locked_market_rate = new_market_rate;
			lock.liquidity_promised = new_liquidity_promised;
			T::LockEvents::utxo_locked(utxo_id, &who, to_mint)?;

			Self::deposit_event(Event::BitcoinLockRatcheted {
				utxo_id,
				vault_id: lock.vault_id,
				security_fee: fee,
				original_market_rate,
				new_locked_market_rate: new_market_rate,
				liquidity_promised: new_liquidity_promised,
				amount_burned,
				account_id: who.clone(),
			});
			LocksByUtxoId::<T>::insert(utxo_id, lock);

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
					T::BitcoinUtxoTracker::get_funding_utxo_ref(entry.utxo_id) !=
						Some(utxo_ref.clone()),
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
			T::BitcoinUtxoTracker::unwatch_candidate(utxo_id, &utxo_ref);
			Self::deposit_event(Event::OrphanedUtxoCosigned {
				utxo_id,
				vault_id,
				utxo_ref: utxo_ref.clone(),
				account_id: orphan_owner,
				signature,
			});

			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::register_fee_coupon())]
		pub fn register_fee_coupon(
			origin: OriginFor<T>,
			public: sr25519::Public,
			#[pallet::compact] max_satoshis: Satoshis,
			max_fee_plus_tip: Option<T::Balance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				!FeeCouponsByPublic::<T>::contains_key(public),
				Error::<T>::FeeCouponAlreadyExists
			);
			let vault_id = T::VaultProvider::get_vault_id(&who).ok_or(Error::<T>::NoPermissions)?;
			ensure!(
				max_satoshis >= MinimumSatoshis::<T>::get(),
				Error::<T>::InsufficientSatoshisLocked
			);

			let expiration_frame = T::CurrentFrameId::get() + 2; // expires at the beginning of 2 frames from now. Means 24 hours plus rest of current frame

			FeeCouponsByPublic::<T>::insert(
				public,
				FeeCoupon { vault_id, expiration_frame, max_satoshis, max_fee_plus_tip },
			);
			FeeCouponsExpiringByFrame::<T>::insert(expiration_frame, public, ());

			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::initialize())]
		pub fn initialize_for(
			origin: OriginFor<T>,
			account_id: T::AccountId,
			vault_id: VaultId,
			#[pallet::compact] satoshis: Satoshis,
			bitcoin_pubkey: CompressedBitcoinPubkey,
			options: Option<LockOptions<T>>,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			ensure!(
				options.as_ref().and_then(|o| o.fee_coupon_proof()).is_some(),
				Error::<T>::FeeCouponRequired
			);
			Self::create_bitcoin_lock(&account_id, vault_id, satoshis, bitcoin_pubkey, options)?;
			Ok(())
		}

		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::increase_securitization())]
		pub fn increase_securitization(
			origin: OriginFor<T>,
			utxo_id: UtxoId,
			#[pallet::compact] new_satoshis: Satoshis,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			ensure!(lock.owner_account == who, Error::<T>::NoPermissions);
			ensure!(!lock.is_funded, Error::<T>::OrphanedUtxoFundingConflict);
			ensure!(new_satoshis > lock.satoshis, Error::<T>::InsufficientSatoshisLocked);

			let current_bitcoin_height = T::BitcoinBlockHeightChange::get().1;
			let elapsed_blocks = current_bitcoin_height.saturating_sub(lock.created_at_height);
			let full_term = lock.vault_claim_height.saturating_sub(lock.created_at_height).max(1);
			let remaining_blocks = full_term.saturating_sub(elapsed_blocks);
			let duration_for_new_funds =
				FixedU128::from_rational(remaining_blocks as u128, full_term as u128);

			let ratio = FixedU128::from_rational(new_satoshis as u128, lock.satoshis as u128);
			let new_liquidity_promised = ratio.saturating_mul_int(lock.liquidity_promised);
			let additional_liquidity =
				new_liquidity_promised.saturating_sub(lock.liquidity_promised);
			if additional_liquidity > T::Balance::zero() {
				let mut lock_extension = lock.get_lock_extension();
				let securitization =
					Securitization::new(additional_liquidity, lock.securitization_ratio);
				let fee = T::VaultProvider::lock(
					lock.vault_id,
					&who,
					&securitization,
					new_satoshis.saturating_sub(lock.satoshis),
					Some((duration_for_new_funds, &mut lock_extension)),
					false,
				)
				.map_err(Error::<T>::from)?;

				lock.security_fees.saturating_accrue(fee);
				lock.fund_hold_extensions = lock_extension.extended_expiration_funds.clone();
				lock.liquidity_promised = new_liquidity_promised;
				lock.locked_market_rate = ratio.saturating_mul_int(lock.locked_market_rate);
			}
			lock.satoshis = new_satoshis;
			let vault_id = lock.vault_id;
			LocksByUtxoId::<T>::insert(utxo_id, lock);
			Self::deposit_event(Event::SecuritizationIncreased {
				utxo_id,
				vault_id,
				new_satoshis,
				account_id: who,
			});
			Ok(())
		}
	}

	impl<T: Config>
		TransactionSponsorProvider<
			<T as frame_system::Config>::AccountId,
			T::RuntimeCall,
			T::Balance,
		> for Pallet<T>
	where
		T::RuntimeCall: IsSubType<Call<T>>,
	{
		fn get_transaction_sponsor(
			signer: &<T as frame_system::Config>::AccountId,
			call: &T::RuntimeCall,
		) -> Option<TxSponsor<<T as frame_system::Config>::AccountId, T::Balance>> {
			let pallet_call: &Call<T> = <T::RuntimeCall as IsSubType<Call<T>>>::is_sub_type(call)?;

			match pallet_call {
				Call::initialize { vault_id, satoshis, options, .. } |
				Call::initialize_for { vault_id, satoshis, options, .. } => {
					let account_id = match pallet_call {
						Call::initialize { .. } => signer,
						Call::initialize_for { account_id, .. } => account_id,
						_ => unreachable!(),
					};
					let coupon_proof = options.as_ref().and_then(|a| a.fee_coupon_proof())?;
					let Some(coupon) = FeeCouponsByPublic::<T>::get(coupon_proof.public) else {
						log::info!(
							"Fee coupon supplied without matching runtime coupon {:?}",
							coupon_proof.public
						);
						return None;
					};
					if coupon.vault_id == *vault_id &&
						*satoshis <= coupon.max_satoshis &&
						coupon_proof.verify(account_id)
					{
						let vault_operator = T::VaultProvider::get_vault_operator(*vault_id)?;
						Some(TxSponsor {
							payer: vault_operator,
							max_fee_with_tip: coupon.max_fee_plus_tip,
							// only allow a single use of the coupon
							unique_tx_key: Some(coupon_proof.public.encode()),
						})
					} else {
						None
					}
				},
				_ => None,
			}
		}
	}
	impl<T: Config> BitcoinUtxoEvents<T::AccountId> for Pallet<T> {
		fn funding_received(utxo_id: UtxoId, received_satoshis: Satoshis) -> DispatchResult {
			LocksByUtxoId::<T>::mutate(utxo_id, |a| {
				if let Some(lock) = a {
					if lock.is_funded {
						log::warn!("Utxo id {:?} already funded", utxo_id);
						return Ok(());
					}
					lock.is_funded = true;
					T::VaultProvider::remove_pending(lock.vault_id, &lock.get_securitization())
						.map_err(Error::<T>::from)?;

					// If we received different amount of sats than expected, we need to adjust
					// the lock parameters. We will not change the fee.
					if received_satoshis < lock.satoshis {
						let ratio = FixedU128::from_rational(
							received_satoshis as u128,
							lock.satoshis as u128,
						);
						let starting_liquidity = lock.liquidity_promised;
						lock.locked_market_rate = ratio.saturating_mul_int(lock.locked_market_rate);
						lock.liquidity_promised = ratio.saturating_mul_int(starting_liquidity);
						let to_return = starting_liquidity.saturating_sub(lock.liquidity_promised);

						if !to_return.is_zero() {
							let securitization_to_return =
								Securitization::new(to_return, lock.securitization_ratio);
							T::VaultProvider::cancel(lock.vault_id, &securitization_to_return)
								.map_err(Error::<T>::from)?;
						}
						lock.satoshis = received_satoshis;
					}
					lock.utxo_satoshis = Some(received_satoshis);
					T::LockEvents::utxo_locked(
						utxo_id,
						&lock.owner_account,
						lock.liquidity_promised,
					)?;
					T::VaultProvider::remove_pending(lock.vault_id, &lock.get_securitization())
						.map_err(Error::<T>::from)?;
				} else {
					log::warn!("Funded utxo_id {:?} not found", utxo_id);
				}
				Ok::<(), DispatchError>(())
			})
		}

		fn timeout_waiting_for_funding(utxo_id: UtxoId) -> DispatchResult {
			if let Some(lock) = LocksByUtxoId::<T>::get(utxo_id) {
				// Funded locks are handled by their own lifecycle; don't cancel due to pending
				// funding timeouts.
				if lock.is_funded {
					return Ok(());
				}
			}

			if let Some(lock) = LocksByUtxoId::<T>::take(utxo_id) {
				Self::schedule_orphans_for_cleanup(utxo_id, &lock);
				let securitization = lock.get_securitization();
				T::VaultProvider::remove_pending(lock.vault_id, &securitization)
					.map_err(Error::<T>::from)?;
				T::VaultProvider::cancel(lock.vault_id, &securitization)
					.map_err(Error::<T>::from)?;
			}
			Ok(())
		}

		fn orphaned_utxo_detected(
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

		fn funding_promoted_by_account(
			utxo_id: UtxoId,
			received_satoshis: Satoshis,
			account_id: &T::AccountId,
			utxo_ref: &UtxoRef,
		) -> sp_runtime::DispatchResult {
			let lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			ensure!(lock.owner_account == *account_id, Error::<T>::NoPermissions);
			ensure!(!lock.is_funded, Error::<T>::OrphanedUtxoFundingConflict);

			if let Some(orphan) = OrphanedUtxosByAccount::<T>::take(account_id, utxo_ref) {
				ensure!(orphan.cosign_request.is_none(), Error::<T>::OrphanedUtxoReleaseRequested);
			};

			Self::funding_received(utxo_id, received_satoshis)?;
			Self::deposit_event(Event::UtxoFundedFromCandidate {
				utxo_id,
				utxo_ref: utxo_ref.clone(),
				vault_id: lock.vault_id,
				account_id: account_id.clone(),
			});
			Ok(())
		}

		fn spent(utxo_id: UtxoId) -> DispatchResult {
			if LocksByUtxoId::<T>::contains_key(utxo_id) {
				Self::burn_bitcoin_lock(utxo_id, true)
			} else {
				Ok(())
			}
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: Codec,
	{
		fn create_bitcoin_lock(
			account_id: &<T as frame_system::Config>::AccountId,
			vault_id: VaultId,
			satoshis: Satoshis,
			bitcoin_pubkey: CompressedBitcoinPubkey,
			options: Option<LockOptions<T>>,
		) -> DispatchResult {
			ensure!(
				satoshis >= MinimumSatoshis::<T>::get(),
				Error::<T>::InsufficientSatoshisLocked
			);
			let has_fee_coupon = if let Some(coupon_proof) =
				options.as_ref().and_then(|o| o.fee_coupon_proof())
			{
				let coupon = FeeCouponsByPublic::<T>::take(coupon_proof.public)
					.ok_or(Error::<T>::InvalidFeeCoupon)?;
				ensure!(satoshis <= coupon.max_satoshis, Error::<T>::MaxFeeCouponSatoshisExceeded);
				ensure!(vault_id == coupon.vault_id, Error::<T>::InvalidFeeCoupon);
				ensure!(coupon_proof.verify(&account_id), Error::<T>::InvalidFeeCouponProof);
				true
			} else {
				false
			};

			let current_bitcoin_height = T::BitcoinBlockHeightChange::get().1;
			let vault_claim_height = current_bitcoin_height + T::LockDurationBlocks::get();
			let open_claim_height = vault_claim_height + T::LockReclamationBlocks::get();

			let liquidity_promised = if let Some(rate) = options.and_then(|a| a.microgons_per_btc())
			{
				Self::get_bitcoin_argons_at_rate(satoshis, rate)?
			} else {
				T::PriceProvider::get_bitcoin_argon_price(satoshis)
					.ok_or(Error::<T>::NoBitcoinPricesAvailable)?
			};
			let locked_market_rate = Self::calculate_adjusted_market_rate(liquidity_promised)?;
			let securitization_ratio =
				T::VaultProvider::get_securitization_ratio(vault_id).map_err(Error::<T>::from)?;
			let securitization = Securitization::new(liquidity_promised, securitization_ratio);

			let fee = T::VaultProvider::lock(
				vault_id,
				account_id,
				&securitization,
				satoshis,
				None,
				has_fee_coupon,
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

			T::BitcoinUtxoTracker::watch_for_utxo(
				utxo_id,
				script_pubkey,
				satoshis,
				// translate back into a time with millis
				vault_claim_height,
			)?;

			LocksByUtxoId::<T>::insert(
				utxo_id,
				LockedBitcoin {
					owner_account: account_id.clone(),
					vault_id,
					locked_market_rate,
					liquidity_promised,
					security_fees: fee,
					securitization_ratio,
					coupon_paid_fees: if has_fee_coupon { fee } else { T::Balance::zero() },
					utxo_satoshis: None,
					satoshis,
					vault_pubkey,
					vault_claim_pubkey,
					vault_xpub_sources,
					owner_pubkey: bitcoin_pubkey,
					vault_claim_height,
					open_claim_height,
					created_at_height: current_bitcoin_height,
					utxo_script_pubkey: script_pubkey,
					is_funded: false,
					fund_hold_extensions: BoundedBTreeMap::default(),
					created_at_argon_block: <frame_system::Pallet<T>>::block_number(),
				},
			);
			Self::deposit_event(Event::<T>::BitcoinLockCreated {
				utxo_id,
				vault_id,
				liquidity_promised,
				locked_market_rate,
				securitization: securitization.collateral_required,
				account_id: account_id.clone(),
				security_fee: fee,
			});

			Ok(())
		}

		fn get_bitcoin_argons_at_rate(
			satoshis: Satoshis,
			microgons_per_btc: T::Balance,
		) -> Result<T::Balance, Error<T>> {
			let rates = MicrogonPerBtcHistory::<T>::get();
			if !rates.iter().any(|(_, r)| r == &microgons_per_btc) {
				return Err(Error::<T>::IneligibleMicrogonRateRequested);
			}
			let satoshis = FixedU128::saturating_from_integer(satoshis);
			let satoshis_per_bitcoin = FixedU128::saturating_from_integer(SATOSHIS_PER_BITCOIN);

			let sat_ratio = satoshis / satoshis_per_bitcoin;

			Ok(sat_ratio.saturating_mul_int(microgons_per_btc))
		}

		pub fn minimum_satoshis() -> Satoshis {
			MinimumSatoshis::<T>::get()
		}

		/// The liquidity price accounts for BTC -> USD and MICROGON -> USD prices, which gives us
		/// MICROGON -> BTC, but we don't account for how far off target the ARGON is from the CPI
		/// target price. So, for instance, we might be trading 1-1 with the USD, but in reality,
		/// there's been 100% inflation, so the lock price should be 2 ARGON per BTC, not 1
		/// ARGON per BTC.
		pub(crate) fn calculate_adjusted_market_rate(
			liquidity_promised: T::Balance,
		) -> Result<T::Balance, Error<T>> {
			let target_offset = T::PriceProvider::get_argon_cpi()
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?
				.add(FixedI128::one());
			// target = 1.5, price = 1.4, offset = 0.1, target_offset = 1.1
			// need to multiply price by 1.1 to get to pegged price
			Ok(target_offset.saturating_mul_int(liquidity_promised))
		}

		fn burn_bitcoin_lock(utxo_id: UtxoId, is_externally_spent: bool) -> DispatchResult {
			let lock = LocksByUtxoId::<T>::take(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			if is_externally_spent {
				Self::clear_orphans_for_lock(utxo_id, &lock)?;
			} else {
				Self::schedule_orphans_for_cleanup(utxo_id, &lock);
			}
			T::BitcoinUtxoTracker::unwatch(utxo_id);

			if !lock.is_funded {
				T::VaultProvider::remove_pending(lock.vault_id, &lock.get_securitization())
					.map_err(Error::<T>::from)?;
				T::VaultProvider::cancel(lock.vault_id, &lock.get_securitization())
					.map_err(Error::<T>::from)?;
				return Ok(());
			}

			// burn the current redemption price from the vault at value of actual satoshis locked
			let redemption_price = Self::get_redemption_price(
				&lock.effective_satoshis(),
				Some(lock.locked_market_rate),
			)?;

			T::VaultProvider::burn(
				lock.vault_id,
				&lock.get_securitization(),
				redemption_price,
				&lock.get_lock_extension(),
			)
			.map_err(Error::<T>::from)?;

			let amount_eligible_for_pool = lock.liquidity_promised.min(redemption_price);
			T::LockEvents::utxo_released(utxo_id, is_externally_spent, amount_eligible_for_pool)?;

			Self::deposit_event(Event::BitcoinLockBurned {
				utxo_id,
				vault_id: lock.vault_id,
				was_utxo_spent: is_externally_spent,
			});

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

		/// Call made during the on_initialize to implement cosign overdue penalties.
		pub(crate) fn cosign_bitcoin_overdue(utxo_id: UtxoId) -> DispatchResult {
			let lock = LocksByUtxoId::<T>::take(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			let vault_id = lock.vault_id;
			let entry = Self::take_release_request(utxo_id)?;

			let redemption_price_on_hold = entry.redemption_price;

			// need to compensate with market price, not the redemption price - use the real
			// satoshis locked to get the market price
			let market_price = T::PriceProvider::get_bitcoin_argon_price(lock.effective_satoshis())
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?;

			let adjusted_market_rate = market_price.min(redemption_price_on_hold);

			// 1. Return funds to user
			// 2. Any difference from market rate comes from vault, capped by securitization
			// 3. Everything else up to market is burned from the vault
			let compensation_amount = T::VaultProvider::compensate_lost_bitcoin(
				vault_id,
				&lock.owner_account,
				&lock.get_securitization(),
				adjusted_market_rate,
				&lock.get_lock_extension(),
			)
			.map_err(Error::<T>::from)?;

			// we return this amount to the bitcoin holder
			if redemption_price_on_hold > T::Balance::zero() {
				T::Currency::release(
					&HoldReason::ReleaseBitcoinLock.into(),
					&lock.owner_account,
					redemption_price_on_hold,
					Precision::Exact,
				)?;
				frame_system::Pallet::<T>::dec_providers(&lock.owner_account)?;
			}
			// count the amount we took from the vault as the burn amount
			T::LockEvents::utxo_released(utxo_id, false, adjusted_market_rate)?;

			Self::deposit_event(Event::BitcoinCosignPastDue {
				utxo_id,
				vault_id,
				compensation_amount,
				compensated_account_id: lock.owner_account.clone(),
			});
			Self::schedule_orphans_for_cleanup(utxo_id, &lock);
			T::BitcoinUtxoTracker::unwatch(utxo_id);

			Ok(())
		}

		pub fn get_redemption_price(
			satoshis: &Satoshis,
			locked_market_rate: Option<T::Balance>,
		) -> Result<T::Balance, Error<T>> {
			let satoshis = FixedU128::from_rational(*satoshis as u128, 1);
			let sats_per_argon =
				FixedU128::from_rational(SATOSHIS_PER_BITCOIN as u128 / MICROGONS_PER_ARGON, 1);

			let mut price = T::PriceProvider::get_latest_btc_price_in_usd()
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?
				.checked_div(&sats_per_argon)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?
				.saturating_mul(satoshis);

			if let Some(locked_market_rate) = locked_market_rate {
				price = price.min(FixedU128::from_rational(locked_market_rate.into(), 1u128));
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
			// now scale to argons (really microgons)
			let argons = price.saturating_mul_int(T::Balance::one());
			Ok(argons)
		}

		fn cancel_lock(utxo_id: UtxoId, lock: &LockedBitcoin<T>) -> DispatchResult {
			if !lock.is_funded {
				T::VaultProvider::remove_pending(lock.vault_id, &lock.get_securitization())
					.map_err(Error::<T>::from)?;
			}
			T::VaultProvider::cancel(lock.vault_id, &lock.get_securitization())
				.map_err(Error::<T>::from)?;
			T::BitcoinUtxoTracker::unwatch(utxo_id);
			Self::schedule_orphans_for_cleanup(utxo_id, lock);
			LocksByUtxoId::<T>::remove(utxo_id);

			Ok(())
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
					"Orphaned UTXO cleanup schedule overflowed for lock {:?} at frame {:?}",
					utxo_id,
					expiry_frame
				);
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
