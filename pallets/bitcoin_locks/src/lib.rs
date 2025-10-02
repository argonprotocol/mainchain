#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::zero_prefixed_literal)]

extern crate alloc;
extern crate core;

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
		BitcoinUtxoEvents, BitcoinUtxoTracker, MICROGONS_PER_ARGON, PriceProvider, UtxoLockEvents,
		VaultId,
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinRejectedReason, BitcoinScriptPubkey,
			BitcoinSignature, CompressedBitcoinPubkey, SATOSHIS_PER_BITCOIN, Satoshis, UtxoId,
			XPubChildNumber, XPubFingerprint,
		},
		vault::{BitcoinVaultProvider, LockExtension, VaultError},
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);

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
			+ MaxEncodedLen;

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

		/// Getter for the current frame id
		type CurrentFrameId: Get<FrameId>;

		/// Pallet storage requires bounds, so we have to set a maximum number that can expire in a
		/// single block
		#[pallet::constant]
		type MaxConcurrentlyExpiringLocks: Get<u32>;

		/// Number of ticks per bitcoin block
		#[pallet::constant]
		type TicksPerBitcoinBlock: Get<Tick>;
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

	#[pallet::storage]
	pub type LockReleaseRequestsByUtxoId<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, LockReleaseRequest<T::Balance>, OptionQuery>;

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
	/// unlock it
	#[pallet::storage]
	pub type LockExpirationsByBitcoinHeight<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinHeight,
		BoundedBTreeSet<UtxoId, T::MaxConcurrentlyExpiringLocks>,
		ValueQuery,
	>;

	#[derive(Decode, Encode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct LockedBitcoin<T: Config> {
		#[codec(compact)]
		pub vault_id: VaultId,
		/// The mintable liquidity of this lock, in microgons
		pub liquidity_promised: T::Balance,
		/// The lock price in microgons, adjusted for any inflation offset of the argon
		pub pegged_price: T::Balance,
		pub owner_account: T::AccountId,
		/// Sum of all lock fees (initial plus any ratcheting)
		pub security_fees: T::Balance,
		#[codec(compact)]
		pub satoshis: Satoshis,
		pub vault_pubkey: CompressedBitcoinPubkey,
		pub vault_claim_pubkey: CompressedBitcoinPubkey,
		/// The vault xpub sources. First is the cosign number, second is the claim number
		pub vault_xpub_sources: (XPubFingerprint, XPubChildNumber, XPubChildNumber),
		pub owner_pubkey: CompressedBitcoinPubkey,
		#[codec(compact)]
		pub vault_claim_height: BitcoinHeight,
		#[codec(compact)]
		pub open_claim_height: BitcoinHeight,
		#[codec(compact)]
		pub created_at_height: BitcoinHeight,
		pub utxo_script_pubkey: BitcoinCosignScriptPubkey,
		pub is_verified: bool,
		pub is_rejected_needs_release: bool,
		pub fund_hold_extensions: BoundedBTreeMap<BitcoinHeight, T::Balance, ConstU32<366>>,
	}

	impl<T: Config> LockedBitcoin<T> {
		pub fn get_lock_extension(&self) -> LockExtension<T::Balance> {
			LockExtension {
				extended_expiration_funds: self.fund_hold_extensions.clone(),
				lock_expiration: self.vault_claim_height,
			}
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
		#[codec(compact)]
		pub utxo_id: UtxoId,
		#[codec(compact)]
		pub vault_id: VaultId,
		#[codec(compact)]
		pub bitcoin_network_fee: Satoshis,
		#[codec(compact)]
		pub cosign_due_frame: FrameId,
		pub to_script_pubkey: BitcoinScriptPubkey,
		#[codec(compact)]
		pub redemption_price: Balance,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		BitcoinLockCreated {
			utxo_id: UtxoId,
			vault_id: VaultId,
			liquidity_promised: T::Balance,
			pegged_price: T::Balance,
			account_id: T::AccountId,
			security_fee: T::Balance,
		},
		BitcoinLockRatcheted {
			utxo_id: UtxoId,
			vault_id: VaultId,
			liquidity_promised: T::Balance,
			original_pegged_price: T::Balance,
			security_fee: T::Balance,
			new_pegged_price: T::Balance,
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
	}

	#[pallet::error]
	pub enum Error<T> {
		InsufficientFunds,
		InsufficientVaultFunds,
		/// The proposed transaction would take the account below the minimum (existential) balance
		AccountWouldGoBelowMinimumBalance,
		VaultClosed,
		/// Funding would result in an overflow of the balance type
		InvalidVaultAmount,
		/// This bitcoin redemption has not been locked in
		RedemptionNotLocked,
		/// The bitcoin has passed the deadline to release it
		BitcoinReleaseInitiationDeadlinePassed,
		/// The fee for this bitcoin release is too high
		BitcoinFeeTooHigh,
		BitcoinUtxoNotFound,
		/// This bitcoin cosign script couldn't be decoded for release
		BitcoinUnableToBeDecodedForRelease,
		/// This bitcoin signature couldn't be decoded for release
		BitcoinSignatureUnableToBeDecoded,
		/// This bitcoin pubkey couldn't be decoded for release
		BitcoinPubkeyUnableToBeDecoded,
		/// The cosign signature is not valid for the bitcoin release
		BitcoinInvalidCosignature,
		InsufficientSatoshisLocked,
		NoBitcoinPricesAvailable,
		/// The bitcoin script to lock this bitcoin has errors
		InvalidBitcoinScript,
		NoPermissions,
		HoldUnexpectedlyModified,
		UnrecoverableHold,
		VaultNotFound,
		GenericVaultError(VaultError),
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
		/// The lock is not verified
		UnverifiedLock,
		/// An overflow or underflow occurred while calculating the redemption price
		OverflowError,
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
			for utxo_id in expirations {
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

			T::DbWeight::get().reads_writes(2, 1)
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
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;

			ensure!(
				satoshis >= MinimumSatoshis::<T>::get(),
				Error::<T>::InsufficientSatoshisLocked
			);

			let current_bitcoin_height = T::BitcoinBlockHeightChange::get().1;
			let vault_claim_height = current_bitcoin_height + T::LockDurationBlocks::get();
			let open_claim_height = vault_claim_height + T::LockReclamationBlocks::get();

			let liquidity_promised = T::PriceProvider::get_bitcoin_argon_price(satoshis)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?;
			let pegged_price = Self::calculate_pegged_price(liquidity_promised)?;

			let fee =
				T::VaultProvider::lock(vault_id, &account_id, liquidity_promised, satoshis, None)
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
					pegged_price,
					liquidity_promised,
					security_fees: fee,
					satoshis,
					vault_pubkey,
					vault_claim_pubkey,
					vault_xpub_sources,
					owner_pubkey: bitcoin_pubkey,
					vault_claim_height,
					open_claim_height,
					created_at_height: current_bitcoin_height,
					utxo_script_pubkey: script_pubkey,
					is_verified: false,
					is_rejected_needs_release: false,
					fund_hold_extensions: BoundedBTreeMap::default(),
				},
			);
			Self::deposit_event(Event::<T>::BitcoinLockCreated {
				utxo_id,
				vault_id,
				liquidity_promised,
				pegged_price,
				account_id,
				security_fee: fee,
			});

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
			if !lock.is_verified && !lock.is_rejected_needs_release {
				Self::cancel_lock(utxo_id)?;
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
			if lock.is_verified {
				ensure!(bitcoin_network_fee < lock.satoshis, Error::<T>::BitcoinFeeTooHigh);
				redemption_price =
					Self::get_redemption_price(&lock.satoshis, Some(lock.pegged_price))?;
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
			let LockedBitcoin {
				liquidity_promised,
				vault_id,
				satoshis,
				vault_pubkey,
				vault_claim_pubkey,
				owner_pubkey,
				created_at_height,
				open_claim_height,
				owner_account,
				is_rejected_needs_release,
				vault_claim_height,
				..
			} = lock;

			ensure!(T::VaultProvider::is_owner(vault_id, &who), Error::<T>::NoPermissions);
			let request = Self::take_release_request(utxo_id)?;

			let utxo_ref =
				T::BitcoinUtxoTracker::get(utxo_id).ok_or(Error::<T>::BitcoinUtxoNotFound)?;

			let script_args = CosignScriptArgs {
				vault_pubkey,
				owner_pubkey,
				vault_claim_pubkey,
				created_at_height,
				vault_claim_height,
				open_claim_height,
			};
			let releaser = CosignReleaser::new(
				script_args,
				satoshis,
				utxo_ref.txid.into(),
				utxo_ref.output_index,
				ReleaseStep::VaultCosign,
				Amount::from_sat(request.bitcoin_network_fee),
				request.to_script_pubkey.into(),
				T::GetBitcoinNetwork::get().into(),
			)
			.map_err(|_| Error::<T>::BitcoinUnableToBeDecodedForRelease)?;

			T::BitcoinSignatureVerifier::verify_signature(releaser, vault_pubkey, &signature)?;

			if is_rejected_needs_release {
				// NOTE: this isn't strictly needed yet, but maintains some consistency in calling
				T::LockEvents::utxo_released(utxo_id, false, 0u32.into())?;
				T::VaultProvider::cancel(vault_id, liquidity_promised).map_err(Error::<T>::from)?;
			} else {
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
					liquidity_promised,
					satoshis,
					&lock_extension,
				)
				.map_err(Error::<T>::from)?;
			}

			LockReleaseCosignHeightById::<T>::insert(
				utxo_id,
				frame_system::Pallet::<T>::block_number(),
			);

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
		pub fn ratchet(origin: OriginFor<T>, utxo_id: UtxoId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			ensure!(lock.owner_account == who, Error::<T>::NoPermissions);
			ensure!(lock.is_verified, Error::<T>::UnverifiedLock);
			ensure!(
				!LockReleaseRequestsByUtxoId::<T>::contains_key(utxo_id),
				Error::<T>::LockInProcessOfRelease
			);

			let new_liquidity_promised = T::PriceProvider::get_bitcoin_argon_price(lock.satoshis)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?;

			let original_pegged_price = lock.pegged_price;
			ensure!(
				original_pegged_price != new_liquidity_promised,
				Error::<T>::NoRatchetingAvailable
			);

			let vault_id = lock.vault_id;
			let expiration_height = lock.vault_claim_height;
			let mut amount_burned = T::Balance::zero();
			let mut to_mint = new_liquidity_promised;
			let mut duration_for_new_funds = FixedU128::zero();
			if new_liquidity_promised > original_pegged_price {
				to_mint = new_liquidity_promised.saturating_sub(original_pegged_price);

				let start_height = lock.created_at_height;
				let elapsed_blocks =
					T::BitcoinBlockHeightChange::get().1.saturating_sub(start_height);
				let full_term = expiration_height.saturating_sub(start_height);
				duration_for_new_funds =
					FixedU128::from_rational(elapsed_blocks as u128, full_term as u128);
			} else {
				let redemption_price =
					Self::get_redemption_price(&lock.satoshis, Some(original_pegged_price))
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
				T::VaultProvider::schedule_for_release(
					vault_id,
					lock.liquidity_promised,
					0,
					&lock.get_lock_extension(),
				)
				.map_err(Error::<T>::from)?;

				// next we'll add the current market rate as a mint amount with a zero-term duration
				// (only base fee)
			}

			let mut lock_extension = lock.get_lock_extension();
			let fee = T::VaultProvider::lock(
				vault_id,
				&who,
				to_mint,
				0,
				Some((duration_for_new_funds, &mut lock_extension)),
			)
			.map_err(Error::<T>::from)?;

			lock.security_fees.saturating_accrue(fee);
			lock.fund_hold_extensions = lock_extension.extended_expiration_funds.clone();
			let new_pegged_price = Self::calculate_pegged_price(new_liquidity_promised)?;
			lock.pegged_price = new_pegged_price;
			lock.liquidity_promised = new_liquidity_promised;
			T::LockEvents::utxo_locked(utxo_id, &who, to_mint)?;

			Self::deposit_event(Event::BitcoinLockRatcheted {
				utxo_id,
				vault_id: lock.vault_id,
				security_fee: fee,
				original_pegged_price,
				new_pegged_price,
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
	}

	impl<T: Config> BitcoinUtxoEvents for Pallet<T> {
		fn utxo_verified(utxo_id: UtxoId) -> DispatchResult {
			LocksByUtxoId::<T>::mutate(utxo_id, |a| {
				if let Some(lock) = a {
					lock.is_verified = true;
					T::LockEvents::utxo_locked(
						utxo_id,
						&lock.owner_account,
						lock.liquidity_promised,
					)?;
					T::VaultProvider::remove_pending(lock.vault_id, lock.liquidity_promised)
						.map_err(Error::<T>::from)?;
				} else {
					log::warn!("Verified utxo_id {:?} not found", utxo_id);
				}
				Ok::<(), DispatchError>(())
			})
		}

		fn utxo_rejected(utxo_id: UtxoId, reason: BitcoinRejectedReason) -> DispatchResult {
			// if the failure is due to a satoshi mismatch, allow a user to reclaim it
			if matches!(reason, BitcoinRejectedReason::SatoshisMismatch) {
				LocksByUtxoId::<T>::mutate_extant(utxo_id, |lock| {
					lock.is_rejected_needs_release = true;
				});
				Ok(())
			} else {
				Self::cancel_lock(utxo_id)?;
				Ok::<(), DispatchError>(())
			}
		}

		fn utxo_spent(utxo_id: UtxoId) -> DispatchResult {
			if LocksByUtxoId::<T>::contains_key(utxo_id) {
				Self::burn_bitcoin_lock(utxo_id, true)
			} else {
				Ok(())
			}
		}

		fn utxo_expired(utxo_id: UtxoId) -> DispatchResult {
			if LocksByUtxoId::<T>::contains_key(utxo_id) {
				Self::burn_bitcoin_lock(utxo_id, false)
			} else {
				Ok(())
			}
		}
	}

	impl<T: Config> Pallet<T> {
		/// The liquidity price accounts for BTC -> USD and MICROGON -> USD prices, which gives us
		/// MICROGON -> BTC, but we don't account for how far off target the ARGON is from the CPI
		/// target price. So, for instance, we might be trading 1-1 with the USD, but in reality,
		/// there's been 100% inflation, so the lock price should be 2 ARGON per BTC, not 1
		/// ARGON per BTC.
		pub(crate) fn calculate_pegged_price(
			liquidity_promised: T::Balance,
		) -> Result<T::Balance, Error<T>> {
			let target_offset = T::PriceProvider::get_argon_cpi()
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?
				.add(FixedI128::one());
			// target = 1.5, price = 1.4, offset = 0.1
			// need to multiply price by 1.1 to get to target
			Ok(target_offset.saturating_mul_int(liquidity_promised))
		}

		fn burn_bitcoin_lock(utxo_id: UtxoId, is_externally_spent: bool) -> DispatchResult {
			let lock = LocksByUtxoId::<T>::take(utxo_id).ok_or(Error::<T>::LockNotFound)?;

			T::BitcoinUtxoTracker::unwatch(utxo_id);

			if !lock.is_verified {
				T::VaultProvider::remove_pending(lock.vault_id, lock.liquidity_promised)
					.map_err(Error::<T>::from)?;
				T::VaultProvider::cancel(lock.vault_id, lock.liquidity_promised)
					.map_err(Error::<T>::from)?;
				return Ok(());
			}

			// burn the current redemption price from the vault
			let redemption_rate =
				Self::get_redemption_price(&lock.satoshis, Some(lock.pegged_price))?;

			T::VaultProvider::burn(
				lock.vault_id,
				lock.liquidity_promised,
				redemption_rate,
				&lock.get_lock_extension(),
			)
			.map_err(Error::<T>::from)?;

			let amount_eligible_for_pool = lock.liquidity_promised.min(redemption_rate);
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
			let lock = LocksByUtxoId::<T>::take(utxo_id).ok_or(Error::<T>::BitcoinUtxoNotFound)?;
			let vault_id = lock.vault_id;
			let entry = Self::take_release_request(utxo_id)?;

			let redemption_amount_held = entry.redemption_price;

			// need to compensate with market price, not the redemption price
			let market_price = T::PriceProvider::get_bitcoin_argon_price(lock.satoshis)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?;

			let adjusted_market_rate = market_price.min(redemption_amount_held);

			// 1. Return funds to user
			// 2. Any difference from market rate comes from vault, capped by securitization pct
			// 3. Everything else up to market is burned from the vault
			let compensation_amount = T::VaultProvider::compensate_lost_bitcoin(
				vault_id,
				&lock.owner_account,
				lock.liquidity_promised,
				adjusted_market_rate,
				&lock.get_lock_extension(),
			)
			.map_err(Error::<T>::from)?;

			// we return this amount to the bitcoin holder
			if redemption_amount_held > T::Balance::zero() {
				T::Currency::release(
					&HoldReason::ReleaseBitcoinLock.into(),
					&lock.owner_account,
					redemption_amount_held,
					Precision::Exact,
				)?;
				frame_system::Pallet::<T>::dec_providers(&lock.owner_account)?;
			}
			if !lock.is_rejected_needs_release {
				// count the amount we took from the vault as the burn amount
				T::LockEvents::utxo_released(utxo_id, false, adjusted_market_rate)?;
			}

			Self::deposit_event(Event::BitcoinCosignPastDue {
				utxo_id,
				vault_id,
				compensation_amount,
				compensated_account_id: lock.owner_account.clone(),
			});
			T::BitcoinUtxoTracker::unwatch(utxo_id);

			Ok(())
		}

		pub fn get_redemption_price(
			satoshis: &Satoshis,
			pegged_price: Option<T::Balance>,
		) -> Result<T::Balance, Error<T>> {
			let satoshis = FixedU128::from_rational(*satoshis as u128, 1);
			let sats_per_argon =
				FixedU128::from_rational(SATOSHIS_PER_BITCOIN as u128 / MICROGONS_PER_ARGON, 1);

			let mut price = T::PriceProvider::get_latest_btc_price_in_usd()
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?
				.checked_div(&sats_per_argon)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?
				.saturating_mul(satoshis);

			if let Some(pegged_price) = pegged_price {
				price = price.min(FixedU128::from_rational(pegged_price.into(), 1u128));
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

		fn cancel_lock(utxo_id: UtxoId) -> Result<(), Error<T>> {
			if let Some(lock) = LocksByUtxoId::<T>::take(utxo_id) {
				if !lock.is_verified {
					T::VaultProvider::remove_pending(lock.vault_id, lock.liquidity_promised)
						.map_err(Error::<T>::from)?;
				}
				T::VaultProvider::cancel(lock.vault_id, lock.liquidity_promised)
					.map_err(Error::<T>::from)?;
			}
			T::BitcoinUtxoTracker::unwatch(utxo_id);

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
