#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate core;

use sp_runtime::DispatchError;

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
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use alloc::vec;

	use codec::Codec;
	use frame_support::{
		pallet_prelude::*,
		storage::with_storage_layer,
		traits::{
			fungible::{Inspect, Mutate, MutateHold},
			tokens::{Fortitude, Precision},
		},
	};
	use frame_system::pallet_prelude::*;
	use log::warn;
	use sp_arithmetic::FixedU128;
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, UniqueSaturatedInto},
		DispatchError::Token,
		FixedPointNumber, Saturating, TokenError,
	};

	use super::*;
	use argon_bitcoin::{Amount, CosignReleaser, CosignScriptArgs, ReleaseStep};
	use argon_primitives::{
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinRejectedReason, BitcoinScriptPubkey,
			BitcoinSignature, CompressedBitcoinPubkey, Satoshis, UtxoId, XPubChildNumber,
			XPubFingerprint,
		},
		tick::Tick,
		vault::{
			BitcoinObligationProvider, FundType, Obligation, ObligationError, ObligationExpiration,
		},
		BitcoinUtxoEvents, BitcoinUtxoTracker, ObligationEvents, ObligationId, PriceProvider,
		TickProvider, UtxoLockEvents, VaultId,
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		type Currency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
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
		type BitcoinBlockHeight: Get<BitcoinHeight>;

		type GetBitcoinNetwork: Get<BitcoinNetwork>;

		type BitcoinObligationProvider: BitcoinObligationProvider<
			AccountId = Self::AccountId,
			Balance = Self::Balance,
		>;

		/// Argon blocks per day
		#[pallet::constant]
		type ArgonTicksPerDay: Get<Tick>;

		/// Maximum releasing utxos at a time
		#[pallet::constant]
		type MaxConcurrentlyReleasingLocks: Get<u32>;

		/// The number of bitcoin blocks a bitcoin is locked for
		#[pallet::constant]
		type LockDurationBlocks: Get<BitcoinHeight>;

		/// The bitcoin blocks after an obligation expires which the vault will be allowed to claim
		/// a bitcoin
		#[pallet::constant]
		type LockReclamationBlocks: Get<BitcoinHeight>;

		/// Number of bitcoin blocks a vault has to counter-sign a bitcoin release
		#[pallet::constant]
		type LockReleaseCosignDeadlineBlocks: Get<BitcoinHeight>;

		type TickProvider: TickProvider<Self::Block>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		ReleaseBitcoinLock,
	}

	#[pallet::storage]
	pub(super) type NextUtxoId<T: Config> = StorageValue<_, UtxoId, OptionQuery>;

	/// Stores bitcoin utxos that have requested to be released
	#[pallet::storage]
	pub(super) type LocksByUtxoId<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, LockedBitcoin<T>, OptionQuery>;

	/// Mapping of obligation id to lock id
	#[pallet::storage]
	pub(super) type ObligationIdToUtxoId<T: Config> =
		StorageMap<_, Twox64Concat, ObligationId, UtxoId, OptionQuery>;

	/// Stores the block number where the lock was released
	#[pallet::storage]
	pub(super) type LockReleaseCosignHeightById<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, BlockNumberFor<T>, OptionQuery>;

	/// The minimum number of satoshis that can be locked
	#[pallet::storage]
	pub(super) type MinimumSatoshis<T: Config> = StorageValue<_, Satoshis, ValueQuery>;

	/// Stores Utxos that were not paid back in full
	///
	/// Tuple stores Account, Vault, Still Owed, State
	#[pallet::storage]
	pub(super) type OwedUtxoAggrieved<T: Config> = StorageMap<
		_,
		Twox64Concat,
		UtxoId,
		(T::AccountId, VaultId, T::Balance, LockedBitcoin<T>),
		OptionQuery,
	>;

	/// Utxos that have been requested to be cosigned for releasing
	#[pallet::storage]
	pub(super) type LocksPendingReleaseByUtxoId<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<UtxoId, LockReleaseRequest<T::Balance>, T::MaxConcurrentlyReleasingLocks>,
		ValueQuery,
	>;

	#[derive(Decode, Encode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct LockedBitcoin<T: Config> {
		#[codec(compact)]
		pub obligation_id: ObligationId,
		#[codec(compact)]
		pub vault_id: VaultId,
		pub lock_price: T::Balance,
		pub owner_account: T::AccountId,
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
	}

	#[derive(Decode, Encode, CloneNoBound, PartialEqNoBound, EqNoBound, RuntimeDebug, TypeInfo)]
	pub struct LockReleaseRequest<Balance: Clone + Eq + PartialEq + TypeInfo + Codec> {
		#[codec(compact)]
		pub utxo_id: UtxoId,
		#[codec(compact)]
		pub obligation_id: ObligationId,
		#[codec(compact)]
		pub vault_id: VaultId,
		#[codec(compact)]
		pub bitcoin_network_fee: Satoshis,
		#[codec(compact)]
		pub cosign_due_block: BitcoinHeight,
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
			obligation_id: ObligationId,
			lock_price: T::Balance,
			account_id: T::AccountId,
		},
		BitcoinLockBurned {
			utxo_id: UtxoId,
			vault_id: VaultId,
			obligation_id: ObligationId,
			amount_burned: T::Balance,
			amount_held: T::Balance,
			was_utxo_spent: bool,
		},
		BitcoinUtxoCosignRequested {
			utxo_id: UtxoId,
			obligation_id: ObligationId,
			vault_id: VaultId,
		},
		BitcoinUtxoCosigned {
			utxo_id: UtxoId,
			obligation_id: ObligationId,
			vault_id: VaultId,
			signature: BitcoinSignature,
		},
		BitcoinCosignPastDue {
			utxo_id: UtxoId,
			obligation_id: ObligationId,
			vault_id: VaultId,
			compensation_amount: T::Balance,
			compensation_still_owed: T::Balance,
			compensated_account_id: T::AccountId,
		},
		/// An error occurred while refunding an overdue cosigned bitcoin lock
		CosignOverdueError { utxo_id: UtxoId, error: DispatchError },
	}

	#[pallet::error]
	pub enum Error<T> {
		ObligationNotFound,
		NoMoreObligationIds,
		MinimumObligationAmountNotMet,
		/// There are too many obligations expiring in the given expiration block
		ExpirationAtBlockOverflow,
		InsufficientFunds,
		InsufficientVaultFunds,
		/// The vault does not have enough bitcoins to cover the bonded argons
		InsufficientBondedArgons,
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
		ExpirationTooSoon,
		NoPermissions,
		HoldUnexpectedlyModified,
		UnrecoverableHold,
		VaultNotFound,
		GenericObligationError(ObligationError),
		LockNotFound,
		/// No Vault public keys are available
		NoVaultBitcoinPubkeysAvailable,
		/// Unable to generate a new vault public key
		UnableToGenerateVaultBitcoinPubkey,
	}

	impl<T> From<ObligationError> for Error<T> {
		fn from(e: ObligationError) -> Error<T> {
			match e {
				ObligationError::ObligationNotFound => Error::<T>::ObligationNotFound,
				ObligationError::NoMoreObligationIds => Error::<T>::NoMoreObligationIds,
				ObligationError::MinimumObligationAmountNotMet =>
					Error::<T>::MinimumObligationAmountNotMet,
				ObligationError::ExpirationAtBlockOverflow => Error::<T>::ExpirationAtBlockOverflow,
				ObligationError::InsufficientFunds => Error::<T>::InsufficientFunds,
				ObligationError::ExpirationTooSoon => Error::<T>::ExpirationTooSoon,
				ObligationError::NoPermissions => Error::<T>::NoPermissions,
				ObligationError::HoldUnexpectedlyModified => Error::<T>::HoldUnexpectedlyModified,
				ObligationError::UnrecoverableHold => Error::<T>::UnrecoverableHold,
				ObligationError::VaultNotFound => Error::<T>::VaultNotFound,
				ObligationError::InsufficientVaultFunds => Error::<T>::InsufficientVaultFunds,
				ObligationError::InsufficientBondedArgons => Error::<T>::InsufficientBondedArgons,
				ObligationError::VaultClosed => Error::<T>::VaultClosed,
				ObligationError::AccountWouldBeBelowMinimum =>
					Error::<T>::AccountWouldGoBelowMinimumBalance,
				ObligationError::InvalidBitcoinScript => Error::<T>::InvalidBitcoinScript,
				ObligationError::NoVaultBitcoinPubkeysAvailable =>
					Error::<T>::NoVaultBitcoinPubkeysAvailable,
				ObligationError::UnableToGenerateVaultBitcoinPubkey =>
					Error::<T>::UnableToGenerateVaultBitcoinPubkey,

				e => Error::<T>::GenericObligationError(e),
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
			let mut overdue = vec![];
			let bitcoin_block_height = T::BitcoinBlockHeight::get();
			<LocksPendingReleaseByUtxoId<T>>::mutate(|pending| {
				pending.retain(|id, x| {
					if x.cosign_due_block > bitcoin_block_height {
						return true;
					}
					overdue.push((*id, x.redemption_price));
					false
				});
			});

			for (utxo_id, redemption_amount) in overdue {
				let res =
					with_storage_layer(|| Self::cosign_bitcoin_overdue(utxo_id, redemption_amount));
				if let Err(e) = res {
					log::error!("Bitcoin lock id {:?} failed to `cosign` {:?}", utxo_id, e);
					Self::deposit_event(Event::<T>::CosignOverdueError { utxo_id, error: e });
				}
			}

			T::DbWeight::get().reads_writes(2, 1)
		}

		fn on_finalize(_: BlockNumberFor<T>) {}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Initialize a bitcoin lock. This will create a LockedBitcoin for the submitting account
		/// and log the Bitcoin Script hash to Events.
		///
		/// The pubkey submitted here will be used to create a script pubkey that will be used in a
		/// timelock multisig script to lock the bitcoin.
		///
		/// NOTE: A "lock-er" must sends btc to the cosign UTXO address in order to "complete" the
		/// LockedBitcoin and be added to the Bitcoin Mint line.
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
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

			let current_bitcoin_height = T::BitcoinBlockHeight::get();
			let vault_claim_height = current_bitcoin_height + T::LockDurationBlocks::get();
			let open_claim_height = vault_claim_height + T::LockReclamationBlocks::get();

			let lock_price = T::PriceProvider::get_bitcoin_argon_price(satoshis)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?;

			let obligation = T::BitcoinObligationProvider::create_obligation(
				vault_id,
				&account_id,
				FundType::Bitcoin,
				lock_price,
				ObligationExpiration::BitcoinBlock(vault_claim_height),
				// charge in 1 year of ticks (even though we'll expire off bitcoin time)
				T::ArgonTicksPerDay::get() * 365u64,
			)
			.map_err(Error::<T>::from)?;

			T::BitcoinObligationProvider::modify_pending_bitcoin_funds(vault_id, lock_price, false)
				.map_err(Error::<T>::from)?;

			let (vault_xpub, vault_claim_xpub, script_pubkey) =
				T::BitcoinObligationProvider::create_utxo_script_pubkey(
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
					lock_price,
					obligation_id: obligation.obligation_id,
					satoshis,
					vault_pubkey,
					vault_claim_pubkey,
					vault_xpub_sources,
					owner_pubkey: bitcoin_pubkey,
					vault_claim_height,
					open_claim_height,
					created_at_height: T::BitcoinBlockHeight::get(),
					utxo_script_pubkey: script_pubkey,
					is_verified: false,
					is_rejected_needs_release: false,
				},
			);
			ObligationIdToUtxoId::<T>::insert(obligation.obligation_id, utxo_id);
			Self::deposit_event(Event::<T>::BitcoinLockCreated {
				utxo_id,
				obligation_id: obligation.obligation_id,
				vault_id,
				lock_price,
				account_id,
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
		#[pallet::weight(0)]
		pub fn request_release(
			origin: OriginFor<T>,
			utxo_id: UtxoId,
			to_script_pubkey: BitcoinScriptPubkey,
			bitcoin_network_fee: Satoshis,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(Error::<T>::ObligationNotFound)?;
			ensure!(lock.owner_account == who, Error::<T>::NoPermissions);
			let release_due_date = lock
				.vault_claim_height
				.saturating_sub(T::LockReleaseCosignDeadlineBlocks::get());
			ensure!(
				T::BitcoinBlockHeight::get() <= release_due_date,
				Error::<T>::BitcoinReleaseInitiationDeadlinePassed
			);
			let obligation_id = lock.obligation_id;

			// If this is a confirmed utxo, we require the release price to be paid
			if lock.is_verified {
				ensure!(bitcoin_network_fee < lock.satoshis, Error::<T>::BitcoinFeeTooHigh);
				let redemption_price =
					Self::get_redemption_price(&lock.satoshis)?.min(lock.lock_price);
				let cosign_due_block =
					T::LockReleaseCosignDeadlineBlocks::get() + T::BitcoinBlockHeight::get();

				// hold funds until the utxo is seen in the chain
				let balance = T::Currency::balance(&who);
				ensure!(
					balance.saturating_sub(redemption_price) >= T::Currency::minimum_balance(),
					Error::<T>::AccountWouldGoBelowMinimumBalance
				);

				T::Currency::hold(&HoldReason::ReleaseBitcoinLock.into(), &who, redemption_price)
					.map_err(|e| match e {
					Token(TokenError::BelowMinimum) =>
						Error::<T>::AccountWouldGoBelowMinimumBalance,
					_ => Error::<T>::InsufficientFunds,
				})?;
				frame_system::Pallet::<T>::inc_providers(&who);

				<LocksPendingReleaseByUtxoId<T>>::try_mutate(|a| {
					a.try_insert(
						utxo_id,
						LockReleaseRequest {
							utxo_id,
							obligation_id,
							vault_id: lock.vault_id,
							bitcoin_network_fee,
							cosign_due_block,
							to_script_pubkey,
							redemption_price,
						},
					)
				})
				.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;

				Self::deposit_event(Event::<T>::BitcoinUtxoCosignRequested {
					utxo_id,
					obligation_id,
					vault_id: lock.vault_id,
				});
			} else if lock.is_rejected_needs_release {
				let cosign_due_block =
					T::LockReleaseCosignDeadlineBlocks::get() + T::BitcoinBlockHeight::get();

				<LocksPendingReleaseByUtxoId<T>>::try_mutate(|a| {
					a.try_insert(
						utxo_id,
						LockReleaseRequest {
							utxo_id,
							obligation_id,
							vault_id: lock.vault_id,
							bitcoin_network_fee,
							cosign_due_block,
							to_script_pubkey,
							redemption_price: 0u32.into(),
						},
					)
				})
				.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;

				Self::deposit_event(Event::<T>::BitcoinUtxoCosignRequested {
					utxo_id,
					obligation_id,
					vault_id: lock.vault_id,
				});
			} else {
				T::BitcoinObligationProvider::cancel_obligation(lock.obligation_id)
					.map_err(Error::<T>::from)?;
			}
			Ok(())
		}

		/// Submitted by a Vault operator to cosign the release of a bitcoin utxo. The Bitcoin owner
		/// release fee will be burned, and the obligation will be allowed to expire without
		/// penalty.
		///
		/// This is submitted as a no-fee transaction off chain to allow keys to remain in cold
		/// wallets.
		#[pallet::call_index(2)]
		#[pallet::weight((0, DispatchClass::Operational))]
		#[allow(clippy::useless_conversion)]
		pub fn cosign_release(
			origin: OriginFor<T>,
			utxo_id: UtxoId,
			signature: BitcoinSignature,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let lock = LocksByUtxoId::<T>::get(utxo_id).ok_or(Error::<T>::ObligationNotFound)?;

			let vault_id = lock.vault_id;

			ensure!(
				T::BitcoinObligationProvider::is_owner(vault_id, &who),
				Error::<T>::NoPermissions
			);
			let request = LocksPendingReleaseByUtxoId::<T>::mutate(|a| a.remove(&utxo_id))
				.ok_or(Error::<T>::RedemptionNotLocked)?;

			let utxo_ref =
				T::BitcoinUtxoTracker::get(utxo_id).ok_or(Error::<T>::BitcoinUtxoNotFound)?;

			let script_args = CosignScriptArgs {
				vault_pubkey: lock.vault_pubkey,
				owner_pubkey: lock.owner_pubkey,
				vault_claim_pubkey: lock.vault_claim_pubkey,
				created_at_height: lock.created_at_height,
				vault_claim_height: lock.vault_claim_height,
				open_claim_height: lock.open_claim_height,
			};
			let releaser = CosignReleaser::new(
				script_args,
				lock.satoshis,
				utxo_ref.txid.into(),
				utxo_ref.output_index,
				ReleaseStep::VaultCosign,
				Amount::from_sat(request.bitcoin_network_fee),
				request.to_script_pubkey.into(),
				T::GetBitcoinNetwork::get().into(),
			)
			.map_err(|_| Error::<T>::BitcoinUnableToBeDecodedForRelease)?;

			T::BitcoinSignatureVerifier::verify_signature(releaser, lock.vault_pubkey, &signature)?;

			if lock.is_rejected_needs_release {
				T::LockEvents::utxo_released(utxo_id, false, 0u32.into())?;
				T::BitcoinObligationProvider::cancel_obligation(lock.obligation_id)
					.map_err(Error::<T>::from)?;
			} else {
				// burn the owner's held funds
				let burn_amount = request.redemption_price;
				let _ = T::Currency::burn_held(
					&HoldReason::ReleaseBitcoinLock.into(),
					&lock.owner_account,
					burn_amount,
					Precision::Exact,
					Fortitude::Force,
				)?;
				frame_system::Pallet::<T>::dec_providers(&lock.owner_account)?;
				T::LockEvents::utxo_released(utxo_id, false, burn_amount)?;
			}

			LocksByUtxoId::<T>::take(utxo_id);
			ObligationIdToUtxoId::<T>::take(lock.obligation_id);
			LockReleaseCosignHeightById::<T>::insert(
				utxo_id,
				frame_system::Pallet::<T>::block_number(),
			);

			T::BitcoinUtxoTracker::unwatch(utxo_id);

			Self::deposit_event(Event::BitcoinUtxoCosigned {
				utxo_id,
				obligation_id: lock.obligation_id,
				vault_id,
				signature,
			});

			// no fee for cosigning
			Ok(Pays::No.into())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(0)]
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
					T::LockEvents::utxo_locked(utxo_id, &lock.owner_account, lock.lock_price)?;
					T::BitcoinObligationProvider::modify_pending_bitcoin_funds(
						lock.vault_id,
						lock.lock_price,
						true,
					)
					.map_err(Error::<T>::from)?;
				} else {
					warn!("Verified utxo_id {:?} not found", utxo_id);
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
				if let Some(lock) = LocksByUtxoId::<T>::get(utxo_id) {
					T::BitcoinObligationProvider::cancel_obligation(lock.obligation_id)
						.map_err(Error::<T>::from)?;
				}
				Ok::<(), DispatchError>(())
			}
		}

		fn utxo_spent(utxo_id: UtxoId) -> DispatchResult {
			if LocksByUtxoId::<T>::contains_key(utxo_id) {
				Self::burn_bitcoin_obligation(utxo_id, true)
			} else {
				Ok(())
			}
		}

		fn utxo_expired(utxo_id: UtxoId) -> DispatchResult {
			if LocksByUtxoId::<T>::contains_key(utxo_id) {
				Self::burn_bitcoin_obligation(utxo_id, false)
			} else {
				Ok(())
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn burn_bitcoin_obligation(utxo_id: UtxoId, is_externally_spent: bool) -> DispatchResult {
			let lock = LocksByUtxoId::<T>::take(utxo_id).ok_or(Error::<T>::LockNotFound)?;
			let obligation_id = lock.obligation_id;
			LockReleaseCosignHeightById::<T>::remove(utxo_id);
			ObligationIdToUtxoId::<T>::take(obligation_id);

			T::BitcoinUtxoTracker::unwatch(utxo_id);

			if !lock.is_verified {
				T::BitcoinObligationProvider::cancel_obligation(obligation_id)
					.map_err(Error::<T>::from)?;
				return Ok(());
			}

			// burn the current redemption price from the obligation
			let amount_to_burn = Self::get_redemption_price(&lock.satoshis)
				.unwrap_or(lock.lock_price)
				.min(lock.lock_price);

			let obligation = T::BitcoinObligationProvider::burn_vault_bitcoin_obligation(
				lock.obligation_id,
				amount_to_burn,
			)
			.map_err(Error::<T>::from)?;

			T::LockEvents::utxo_released(utxo_id, is_externally_spent, amount_to_burn)?;

			Self::deposit_event(Event::BitcoinLockBurned {
				utxo_id,
				vault_id: lock.vault_id,
				obligation_id,
				amount_burned: amount_to_burn,
				amount_held: obligation.amount,
				was_utxo_spent: is_externally_spent,
			});

			Ok(())
		}

		/// Call made during the on_initialize to implement cosign overdue penalties.
		pub(crate) fn cosign_bitcoin_overdue(
			utxo_id: UtxoId,
			redemption_amount_held: T::Balance,
		) -> DispatchResult {
			let lock = LocksByUtxoId::<T>::take(utxo_id).ok_or(Error::<T>::BitcoinUtxoNotFound)?;
			let obligation_id = lock.obligation_id;
			let vault_id = lock.vault_id;

			// need to compensate with market price, not the redemption price
			let market_price = T::PriceProvider::get_bitcoin_argon_price(lock.satoshis)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?;

			// 1. Return funds to user
			// 2. Any difference from market rate comes from vault, capped by securitization pct
			// 3. Everything else up to market is burned from the vault
			let (still_owed, repaid) = T::BitcoinObligationProvider::compensate_lost_bitcoin(
				lock.obligation_id,
				market_price,
				redemption_amount_held,
			)
			.map_err(Error::<T>::from)?;

			if still_owed > 0u128.into() {
				<OwedUtxoAggrieved<T>>::insert(
					utxo_id,
					(lock.owner_account.clone(), vault_id, still_owed, lock.clone()),
				);
			}

			// we return this amount to the bitcoin holder
			if redemption_amount_held > 0u128.into() {
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
				T::LockEvents::utxo_released(utxo_id, false, market_price)?;
			}

			Self::deposit_event(Event::BitcoinCosignPastDue {
				utxo_id,
				vault_id,
				obligation_id,
				compensation_amount: repaid,
				compensation_still_owed: still_owed,
				compensated_account_id: lock.owner_account.clone(),
			});
			T::BitcoinUtxoTracker::unwatch(utxo_id);
			LockReleaseCosignHeightById::<T>::remove(utxo_id);

			Ok(())
		}

		pub fn get_redemption_price(satoshis: &Satoshis) -> Result<T::Balance, Error<T>> {
			const REDEMPTION_MULTIPLIER: FixedU128 = FixedU128::from_rational(713, 1000);
			const REDEMPTION_ADDON: FixedU128 = FixedU128::from_rational(274, 1000);
			let mut price: u128 = T::PriceProvider::get_bitcoin_argon_price(*satoshis)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?
				.unique_saturated_into();
			let cpi = T::PriceProvider::get_argon_cpi().unwrap_or_default();

			if cpi.is_positive() {
				let argon_price =
					T::PriceProvider::get_latest_argon_price_in_us_cents().unwrap_or_default();

				let multiplier = REDEMPTION_MULTIPLIER
					.saturating_mul(argon_price)
					.saturating_add(REDEMPTION_ADDON);

				// Apply the formula: R = (Pb / Pa) * (0.713 * Pa + 0.274)
				// Pa should be in a float value (1.01)
				// `price` is already (Pb / Pa)
				// The redemption price of the argon allocates fluctuating incentives based on how
				// fast the dip should be incentivized to be capitalized on.
				let adjusted_price = FixedU128::saturating_from_integer(price);
				let fixed_price =
					multiplier.saturating_mul(adjusted_price).into_inner() / FixedU128::accuracy();
				price = fixed_price.unique_saturated_into();
			};

			Ok(price.into())
		}
	}

	impl<T: Config> ObligationEvents<T::AccountId, T::Balance> for Pallet<T> {
		fn on_canceled(
			obligation: &Obligation<T::AccountId, T::Balance>,
		) -> sp_runtime::DispatchResult {
			if let Some(utxo_id) = ObligationIdToUtxoId::<T>::take(obligation.obligation_id) {
				if let Some(lock) = LocksByUtxoId::<T>::take(utxo_id) {
					if !lock.is_verified {
						T::BitcoinObligationProvider::modify_pending_bitcoin_funds(
							obligation.vault_id,
							obligation.amount,
							true,
						)
						.map_err(Error::<T>::from)?;
					}
				}
				LockReleaseCosignHeightById::<T>::remove(utxo_id);
				T::BitcoinUtxoTracker::unwatch(utxo_id);
			}
			Ok(())
		}

		/// If an obligation carries through to completion, the vault has access to the bitcoin, so
		/// we need to burn the corresponding funds from the vault
		fn on_completed(
			obligation: &Obligation<T::AccountId, T::Balance>,
		) -> sp_runtime::DispatchResult {
			if let Some(utxo_id) = ObligationIdToUtxoId::<T>::take(obligation.obligation_id) {
				Self::burn_bitcoin_obligation(utxo_id, false)?;
			}
			Ok(())
		}
	}
}

pub trait BitcoinVerifier<T: Config> {
	fn verify_signature(
		utxo_releaseer: CosignReleaser,
		pubkey: CompressedBitcoinPubkey,
		signature: &BitcoinSignature,
	) -> Result<bool, DispatchError> {
		let is_ok =
			utxo_releaseer.verify_signature_raw(pubkey, signature).map_err(|e| match e {
				argon_bitcoin::Error::InvalidCompressPubkeyBytes =>
					Error::<T>::BitcoinPubkeyUnableToBeDecoded,
				argon_bitcoin::Error::InvalidSignatureBytes =>
					Error::<T>::BitcoinSignatureUnableToBeDecoded,
				_ => Error::<T>::BitcoinInvalidCosignature,
			})?;
		Ok(is_ok)
	}
}
