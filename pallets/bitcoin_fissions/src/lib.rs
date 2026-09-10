#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

use argon_primitives::{
	bitcoin::BitcoinLockId, BitcoinFissionMinting, BitcoinFissionRequirements,
	BitcoinFissionsProvider, OperationalAccountsHook,
};
use pallet_prelude::*;

pub use pallet::*;
pub use weights::{ProviderWeightAdapter, WeightInfo, WithProviderWeights};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use argon_primitives::{
		bitcoin::{BitcoinLockId, FissionId, LiquidId, Satoshis},
		providers::{BitcoinFissionLockError, BitcoinFissionLockProvider},
	};
	use codec::HasCompact;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
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
			+ HasCompact;
		type LockProvider: BitcoinFissionLockProvider<Self::AccountId, Self::Balance>;
		type Minting: BitcoinFissionMinting<Self::AccountId, Self::Balance>;
		type OperationalAccountsHook: OperationalAccountsHook<Self::AccountId, Self::Balance>;
		type Currency: Mutate<Self::AccountId, Balance = Self::Balance>;

		/// Maximum number of active Fissions that may allocate satoshis from one Lock.
		#[pallet::constant]
		type MaxFissionsPerLock: Get<u32>;

		/// Minimum percentage change required to ratchet a Fission.
		#[pallet::constant]
		type MinimumRatchetPercent: Get<Percent>;
	}

	/// Minimum Fission ID accepted from each owner.
	#[pallet::storage]
	pub type NextFissionIdByOwner<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, FissionId, ValueQuery>;

	/// Active Fission records addressed by their owner and owner-local Fission ID.
	#[pallet::storage]
	pub type FissionByOwnerAndId<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Twox64Concat,
		FissionId,
		Fission<T>,
		OptionQuery,
	>;

	/// Active Fission IDs allocating satoshis from each Lock.
	#[pallet::storage]
	pub type FissionIdsByLockId<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinLockId,
		BoundedBTreeSet<FissionId, T::MaxFissionsPerLock>,
		ValueQuery,
	>;

	/// One owner-scoped Fission allocated from a single Bitcoin Lock.
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
	#[scale_info(skip_type_params(T))]
	pub struct Fission<T: Config> {
		/// Owner-selected Liquid ID grouping related Fissions for clients.
		#[codec(compact)]
		pub liquid_id: LiquidId,
		/// Source Lock whose funded satoshis back this Fission.
		#[codec(compact)]
		pub lock_id: BitcoinLockId,
		/// Satoshis allocated from the source Lock.
		#[codec(compact)]
		pub satoshis: Satoshis,
		/// Target-normalized microgon value per BTC applied to this Fission.
		#[codec(compact)]
		pub microgons_at_target_per_btc: T::Balance,
		/// Outstanding Fission liability.
		#[codec(compact)]
		pub liquidity_promised: T::Balance,
		/// Argon block when this Fission was created.
		#[codec(compact)]
		pub created_at_argon_block: BlockNumberFor<T>,
		/// Number of successful ratchet calls applied to this Fission.
		#[codec(compact)]
		pub ratchet_number: u32,
		/// Tick of the price-history entry used by the latest creation or ratchet.
		#[codec(compact)]
		pub last_ratchet_tick: Tick,
		/// Argon block when this Fission was created or last ratcheted.
		#[codec(compact)]
		pub last_updated_argon_block: BlockNumberFor<T>,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A Fission was created and allocated from its source Lock.
		FissionCreated {
			account_id: T::AccountId,
			fission_id: FissionId,
			liquid_id: LiquidId,
			lock_id: BitcoinLockId,
			satoshis: Satoshis,
			microgons_at_target_per_btc: T::Balance,
			liquidity_promised: T::Balance,
		},
		/// A Fission ratchet changed its target-normalized BTC value and liability.
		FissionRatcheted {
			account_id: T::AccountId,
			fission_id: FissionId,
			ratchet_number: u32,
			microgons_at_target_per_btc: T::Balance,
			liquidity_promised: T::Balance,
			amount_minted: T::Balance,
			amount_burned: T::Balance,
		},
		/// A Fission was closed and its source Lock allocation was released.
		FissionClosed {
			account_id: T::AccountId,
			fission_id: FissionId,
			redemption_amount: T::Balance,
		},
		/// An active Fission was closed because its source Lock was spent.
		FissionClosedByLock {
			account_id: T::AccountId,
			fission_id: FissionId,
			lock_id: BitcoinLockId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The submitted Fission ID is below the owner's current minimum.
		FissionIdBelowMinimum,
		/// The owner's Fission ID counter cannot be incremented.
		FissionIdOverflow,
		/// A Fission already exists under this owner and Fission ID.
		FissionAlreadyExists,
		/// No Fission exists under this owner and Fission ID.
		FissionNotFound,
		/// The Fission did not meet both the minimum change and Lock coverage requirements.
		NoRatchetingAvailable,
		/// The Fission's ratchet counter cannot be incremented.
		RatchetNumberOverflow,
		/// A Fission must allocate at least one satoshi.
		FissionHasNoSatoshis,
		/// The source Lock already has the maximum number of active Fissions.
		TooManyFissionsForLock,
		/// The requested source Lock does not exist.
		LockNotFound,
		/// The caller does not own a requested source Lock.
		NoPermissions,
		/// A requested source Lock has no confirmed funding satoshis.
		LockNotFunded,
		/// A requested source Lock is already in the release process.
		LockReleasePending,
		/// A requested source Lock does not have enough unallocated funded satoshis.
		InsufficientFundedSatoshis,
		/// A source Lock's securitization does not cover the requested allocation and liability.
		InsufficientSecuritization,
		/// The requested target-normalized BTC value is not present in recent price history.
		IneligibleMicrogonsAtTargetPerBtc,
		/// The requested price-history entry predates the Fission or Lock coverage floor.
		MicrogonsAtTargetPerBtcTickOlderThanCurrent,
		/// A source Lock has fewer active Fission satoshis than the Fission being closed.
		InsufficientFissionedSatoshis,
		/// No current Bitcoin price is available to calculate the redemption amount.
		NoBitcoinPricesAvailable,
		/// The Fission owner cannot burn the complete redemption amount.
		InsufficientFunds,
		/// A Fission allocation, liability, or identifier overflowed.
		Overflow,
	}

	impl<T: Config> From<BitcoinFissionLockError> for Error<T> {
		fn from(error: BitcoinFissionLockError) -> Self {
			match error {
				BitcoinFissionLockError::LockNotFound => Self::LockNotFound,
				BitcoinFissionLockError::NoPermissions => Self::NoPermissions,
				BitcoinFissionLockError::LockNotFunded => Self::LockNotFunded,
				BitcoinFissionLockError::LockReleasePending => Self::LockReleasePending,
				BitcoinFissionLockError::InsufficientFundedSatoshis =>
					Self::InsufficientFundedSatoshis,
				BitcoinFissionLockError::InsufficientSecuritization =>
					Self::InsufficientSecuritization,
				BitcoinFissionLockError::IneligibleMicrogonsAtTargetPerBtc =>
					Self::IneligibleMicrogonsAtTargetPerBtc,
				BitcoinFissionLockError::MicrogonsAtTargetPerBtcTickOlderThanCurrent =>
					Self::MicrogonsAtTargetPerBtcTickOlderThanCurrent,
				BitcoinFissionLockError::InsufficientFissionedSatoshis =>
					Self::InsufficientFissionedSatoshis,
				BitcoinFissionLockError::NoBitcoinPricesAvailable => Self::NoBitcoinPricesAvailable,
				BitcoinFissionLockError::Overflow => Self::Overflow,
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a Fission from one owner-held Lock and associate it with a Liquid ID.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create())]
		pub fn create(
			origin: OriginFor<T>,
			#[pallet::compact] fission_id: FissionId,
			#[pallet::compact] liquid_id: LiquidId,
			#[pallet::compact] lock_id: BitcoinLockId,
			#[pallet::compact] satoshis: Satoshis,
			#[pallet::compact] microgons_at_target_per_btc: T::Balance,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			let minimum_fission_id = NextFissionIdByOwner::<T>::get(&account_id);
			ensure!(fission_id >= minimum_fission_id, Error::<T>::FissionIdBelowMinimum);
			ensure!(satoshis > 0, Error::<T>::FissionHasNoSatoshis);
			ensure!(
				!FissionByOwnerAndId::<T>::contains_key(&account_id, fission_id),
				Error::<T>::FissionAlreadyExists
			);
			let next_fission_id = fission_id.checked_add(1).ok_or(Error::<T>::FissionIdOverflow)?;

			FissionIdsByLockId::<T>::try_mutate(lock_id, |fission_ids| {
				fission_ids
					.try_insert(fission_id)
					.map(|_| ())
					.map_err(|_| Error::<T>::TooManyFissionsForLock)
			})?;
			let (liquidity_promised, last_ratchet_tick) = T::LockProvider::fission_satoshis(
				&account_id,
				lock_id,
				satoshis,
				microgons_at_target_per_btc,
			)
			.map_err(Error::<T>::from)?;
			T::Minting::request_mint(&account_id, fission_id, lock_id, liquidity_promised)?;

			let block_number = frame_system::Pallet::<T>::block_number();
			FissionByOwnerAndId::<T>::insert(
				&account_id,
				fission_id,
				Fission {
					liquid_id,
					lock_id,
					satoshis,
					microgons_at_target_per_btc,
					last_ratchet_tick,
					liquidity_promised,
					created_at_argon_block: block_number,
					ratchet_number: 0,
					last_updated_argon_block: block_number,
				},
			);
			NextFissionIdByOwner::<T>::insert(&account_id, next_fission_id);
			T::OperationalAccountsHook::account_bitcoin_amount_changed(
				&account_id,
				liquidity_promised,
				true,
			);
			Self::deposit_event(Event::FissionCreated {
				account_id,
				fission_id,
				liquid_id,
				lock_id,
				satoshis,
				microgons_at_target_per_btc,
				liquidity_promised,
			});

			Ok(())
		}

		/// Ratchet an active Fission to a replacement target-normalized BTC value.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::ratchet())]
		pub fn ratchet(
			origin: OriginFor<T>,
			#[pallet::compact] fission_id: FissionId,
			#[pallet::compact] microgons_at_target_per_btc: T::Balance,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			let mut fission = FissionByOwnerAndId::<T>::get(&account_id, fission_id)
				.ok_or(Error::<T>::FissionNotFound)?;

			let ratchet_number =
				fission.ratchet_number.checked_add(1).ok_or(Error::<T>::RatchetNumberOverflow)?;
			let difference = if microgons_at_target_per_btc >= fission.microgons_at_target_per_btc {
				microgons_at_target_per_btc - fission.microgons_at_target_per_btc
			} else {
				fission.microgons_at_target_per_btc - microgons_at_target_per_btc
			};
			let minimum_change =
				T::MinimumRatchetPercent::get().mul_ceil(fission.microgons_at_target_per_btc);
			ensure!(
				!difference.is_zero() && difference >= minimum_change,
				Error::<T>::NoRatchetingAvailable
			);

			let liquidity_promised = T::LockProvider::calculate_liquidity_promised(
				fission.satoshis,
				microgons_at_target_per_btc,
			)
			.map_err(Error::<T>::from)?;
			let (amount_minted, amount_burned) = if liquidity_promised >= fission.liquidity_promised
			{
				(liquidity_promised.saturating_sub(fission.liquidity_promised), T::Balance::zero())
			} else {
				(liquidity_promised, liquidity_promised)
			};
			let last_ratchet_tick = match T::LockProvider::validate_fission(
				&account_id,
				fission.lock_id,
				fission.satoshis,
				microgons_at_target_per_btc,
				fission.last_ratchet_tick,
				fission.liquidity_promised,
				liquidity_promised,
			) {
				Ok(tick) => tick,
				Err(BitcoinFissionLockError::InsufficientSecuritization) =>
					return Err(Error::<T>::NoRatchetingAvailable.into()),
				Err(error) => return Err(Error::<T>::from(error).into()),
			};
			T::Minting::request_mint(&account_id, fission_id, fission.lock_id, amount_minted)?;

			if !amount_burned.is_zero() {
				T::Currency::burn_from(
					&account_id,
					amount_burned,
					Preservation::Expendable,
					Precision::Exact,
					Fortitude::Force,
				)
				.map_err(|_| Error::<T>::InsufficientFunds)?;
				T::Minting::record_mint_repayment(amount_burned);
			}

			let prior_liquidity_promised = fission.liquidity_promised;
			let (liquidity_change, is_increase) = if liquidity_promised >= prior_liquidity_promised
			{
				(liquidity_promised.saturating_sub(prior_liquidity_promised), true)
			} else {
				(prior_liquidity_promised.saturating_sub(liquidity_promised), false)
			};
			if !liquidity_change.is_zero() {
				T::OperationalAccountsHook::account_bitcoin_amount_changed(
					&account_id,
					liquidity_change,
					is_increase,
				);
			}

			fission.microgons_at_target_per_btc = microgons_at_target_per_btc;
			fission.last_ratchet_tick = last_ratchet_tick;
			fission.liquidity_promised = liquidity_promised;
			fission.ratchet_number = ratchet_number;
			fission.last_updated_argon_block = frame_system::Pallet::<T>::block_number();
			FissionByOwnerAndId::<T>::insert(&account_id, fission_id, fission);
			Self::deposit_event(Event::FissionRatcheted {
				account_id,
				fission_id,
				ratchet_number,
				microgons_at_target_per_btc,
				liquidity_promised,
				amount_minted,
				amount_burned,
			});

			Ok(())
		}

		/// Close a Fission without moving Bitcoin and remove its active record.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::close())]
		pub fn close(
			origin: OriginFor<T>,
			#[pallet::compact] fission_id: FissionId,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			let fission = FissionByOwnerAndId::<T>::get(&account_id, fission_id)
				.ok_or(Error::<T>::FissionNotFound)?;

			let redemption_amount = T::LockProvider::fuse_satoshis(
				&account_id,
				fission.lock_id,
				fission.satoshis,
				fission.microgons_at_target_per_btc,
			)
			.map_err(Error::<T>::from)?;
			FissionIdsByLockId::<T>::mutate_exists(fission.lock_id, |fission_ids| {
				let is_empty = fission_ids
					.as_mut()
					.map(|fission_ids| {
						fission_ids.remove(&fission_id);
						fission_ids.is_empty()
					})
					.unwrap_or(false);
				if is_empty {
					*fission_ids = None;
				}
			});

			T::Currency::burn_from(
				&account_id,
				redemption_amount,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)
			.map_err(|_| Error::<T>::InsufficientFunds)?;
			T::Minting::record_mint_repayment(redemption_amount);

			T::OperationalAccountsHook::account_bitcoin_amount_changed(
				&account_id,
				fission.liquidity_promised,
				false,
			);
			FissionByOwnerAndId::<T>::remove(&account_id, fission_id);
			Self::deposit_event(Event::FissionClosed { account_id, fission_id, redemption_amount });

			Ok(())
		}
	}
}

impl<T: Config> BitcoinFissionsProvider<T::AccountId, T::Balance> for Pallet<T> {
	type Weights = weights::ProviderWeightAdapter<T>;

	fn get_account_fission_liquidity(account_id: &T::AccountId) -> T::Balance {
		let mut liquidity = T::Balance::zero();
		for (_, fission) in FissionByOwnerAndId::<T>::iter_prefix(account_id) {
			liquidity.saturating_accrue(fission.liquidity_promised);
		}
		liquidity
	}

	fn get_lock_fission_requirements(
		account_id: &T::AccountId,
		lock_id: BitcoinLockId,
	) -> Option<BitcoinFissionRequirements<T::Balance>> {
		let mut requirements: Option<BitcoinFissionRequirements<T::Balance>> = None;
		for fission_id in FissionIdsByLockId::<T>::get(lock_id) {
			let Some(fission) = FissionByOwnerAndId::<T>::get(account_id, fission_id) else {
				continue
			};

			if let Some(requirements) = requirements.as_mut() {
				requirements.microgons_at_target_per_btc = requirements
					.microgons_at_target_per_btc
					.max(fission.microgons_at_target_per_btc);
				requirements.liquidity_promised.saturating_accrue(fission.liquidity_promised);
				requirements.last_ratchet_tick =
					requirements.last_ratchet_tick.max(fission.last_ratchet_tick);
			} else {
				requirements = Some(BitcoinFissionRequirements {
					microgons_at_target_per_btc: fission.microgons_at_target_per_btc,
					liquidity_promised: fission.liquidity_promised,
					last_ratchet_tick: fission.last_ratchet_tick,
				});
			}
		}
		requirements
	}

	fn close_for_lock(
		account_id: &T::AccountId,
		lock_id: BitcoinLockId,
		burned_argons: T::Balance,
	) -> DispatchResult {
		let mut fission_liability = T::Balance::zero();
		for fission_id in FissionIdsByLockId::<T>::take(lock_id) {
			FissionByOwnerAndId::<T>::try_mutate_exists(
				account_id,
				fission_id,
				|fission| -> DispatchResult {
					let Some(fission) = fission.take() else {
						return Ok(());
					};
					fission_liability.saturating_accrue(fission.liquidity_promised);

					T::OperationalAccountsHook::account_bitcoin_amount_changed(
						account_id,
						fission.liquidity_promised,
						false,
					);
					Self::deposit_event(Event::FissionClosedByLock {
						account_id: account_id.clone(),
						fission_id,
						lock_id,
					});
					Ok(())
				},
			)?;
		}
		let repayment_amount = fission_liability.min(burned_argons);
		if !repayment_amount.is_zero() {
			T::Minting::record_mint_repayment(repayment_amount);
		}

		Ok(())
	}
}
