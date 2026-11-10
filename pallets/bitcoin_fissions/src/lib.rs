#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "try-runtime")]
extern crate alloc;

use argon_primitives::{
	amount_rank::AmountRankId,
	bitcoin::{BitcoinLockId, FissionId, LiquidId, LiquidRepayPrices, Satoshis},
	treasury::{PositionQuantity, TreasuryPositionProvider},
	vault::{BitcoinSecuritizationBasis, CosignerGuarantee},
	BitcoinFissionLockProvider, BitcoinFissionMinting, BitcoinFissionsProvider,
	CrosschainGatewayProvider, LiquidRepaymentProvider, MiningFrameTransitionProvider,
	OperationalAccountsHook, TickProvider,
};
use frame_support::{require_transactional, traits::UnixTime};
use pallet_prelude::*;
use polkadot_sdk::sp_arithmetic::{helpers_128bit::multiply_by_rational_with_rounding, Rounding};

pub use pallet::*;
pub use weights::{ProviderWeightAdapter, WeightInfo, WithProviderWeights};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
#[cfg(test)]
mod mock;
mod price_history;
#[cfg(test)]
mod tests;
#[cfg(feature = "try-runtime")]
mod try_state;
mod weights;
pub use argon_primitives::RValueHighWaterMark;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use argon_primitives::providers::BitcoinFissionLockError;
	use codec::HasCompact;

	pub(super) const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

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
		type FeeCoupon: Parameter;
		type LockProvider: BitcoinFissionLockProvider<
			Self::AccountId,
			Self::Balance,
			FeeCoupon = Self::FeeCoupon,
		>;
		type Minting: BitcoinFissionMinting<Self::AccountId, Self::Balance>;
		type PositionProvider: TreasuryPositionProvider<Self::AccountId, Self::Balance>;
		type OperationalAccountsHook: OperationalAccountsHook<Self::AccountId, Self::Balance>;
		type Currency: Mutate<Self::AccountId, Balance = Self::Balance>;
		/// Pending purchase commitments that prevent direct close of their owner's Liquid.
		type LiquidRepaymentProvider: LiquidRepaymentProvider<Self::AccountId>;

		/// Maximum number of active Fissions that may allocate satoshis from one Lock.
		#[pallet::constant]
		type MaxFissionsPerLock: Get<u32>;
		/// Fissions per Liquid covered by repayment benchmarks and checked by try-state.
		#[pallet::constant]
		type MaxExpectedFissionsPerLiquid: Get<u32>;

		/// Minimum percentage change required to ratchet a Fission.
		#[pallet::constant]
		type MinimumRatchetPercent: Get<Percent>;
		/// Supplies actual reward-frame transitions; frames are not reconstructed from tick counts.
		type MiningFrameTransitionProvider: MiningFrameTransitionProvider;
		/// Network clock and source-time conversion used to retain frame history.
		type TickProvider: TickProvider<Self::Block>;
		/// Block time used to record Liquid creation and ratchets alongside Ethereum purchases.
		type TimeProvider: UnixTime;
		/// Latest accepted source-chain gateway state; this pallet owns its retention range.
		type CrosschainGatewayProvider: CrosschainGatewayProvider<Self::Balance>;
		/// Number of recent frame high-water marks retained after their source interval has synced.
		#[pallet::constant]
		type RValueHistoryFrames: Get<u32>;
		/// Maximum older frame records pruned in one block after gateway sync catches up.
		#[pallet::constant]
		type MaxRValueHistoryPrunesPerBlock: Get<u32>;
	}

	/// Highest valid market-to-target ARGON ratio in each frame. Ordered frame keys support
	/// bounded pruning without loading the complete history during an Ethereum stall.
	#[pallet::storage]
	pub type RValueHighWaterByFrame<T: Config> =
		StorageMap<_, Identity, <FrameId as AmountRankId>::Ordered, RValueHighWaterMark>;

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

	/// Active Fission IDs grouped by their owner's Liquid. Debt remains on the Fission records.
	#[pallet::storage]
	pub type FissionIdsByOwnerAndLiquid<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		(T::AccountId, LiquidId),
		Twox64Concat,
		FissionId,
		(),
		OptionQuery,
	>;

	/// Creation and ratchet timing for each owner-scoped Liquid. Retained after closure so an
	/// old Liquid ID cannot reopen its creation window or discard its ratchet history.
	#[pallet::storage]
	pub type LiquidTicksByOwnerAndId<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Twox64Concat,
		LiquidId,
		LiquidTicks,
		OptionQuery,
	>;

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Copy,
		Default,
		PartialEq,
		Eq,
		Debug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct LiquidTicks {
		/// Block-time tick of the first Fission creation. Further Fissions may join only in this
		/// tick. None means the Liquid predates tracking and its creation window is closed.
		pub created_at_tick: Option<Tick>,
		/// Block-time tick of the latest successful ratchet, independent of its price-history
		/// tick.
		pub last_ratchet_tick: Option<Tick>,
	}

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
		/// Owner-selected Liquid ID grouping related Fissions; new allocations require a nonzero
		/// ID.
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
		/// Cosigner guarantee owned by this Fission.
		pub cosigner_guarantee: CosignerGuarantee<T::Balance>,
		/// Argon block when this Fission was created.
		#[codec(compact)]
		pub created_at_argon_block: BlockNumberFor<T>,
		/// Number of successful ratchet calls applied to this Fission.
		#[codec(compact)]
		pub ratchet_number: u32,
		/// Tick of the price-history entry used by the latest creation or ratchet.
		#[codec(compact)]
		pub last_ratchet_price_tick: Tick,
		/// Argon block when this Fission was created, ratcheted or partially repaid.
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
		/// Part of a Fission was repaid and its Bitcoin allocation returned to the source Lock.
		FissionRepaid {
			/// Owner of the Fission and source Lock.
			account_id: T::AccountId,
			/// Owner-local Fission ID that remains active after repayment.
			fission_id: FissionId,
			/// Satoshis returned to the Lock's available funded allocation.
			satoshis: Satoshis,
			/// Liability retired in microgons, independent of the curve-adjusted burn.
			liquidity_repaid: T::Balance,
			/// Microgons actually burned at the existing redemption price.
			redemption_amount: T::Balance,
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
		/// The Vault could not upgrade the guarantee assigned to this Fission.
		ResecuritizationFailed,
		/// The fee coupon is invalid, expired, or already consumed.
		InvalidFeeCoupon,
		/// Zero is reserved for the absence of a Liquid and cannot identify a new allocation.
		InvalidLiquidId,
		/// A repayment must return at least one satoshi to its source Lock.
		RepaymentHasNoSatoshis,
		/// The repayment requests more satoshis than the Fission currently allocates.
		RepaymentExceedsFission,
		/// This owner has no active Fissions in the selected Liquid.
		LiquidNotFound,
		/// The actual burn exceeds the remaining purchase budget.
		RepaymentBudgetExceeded,
		/// This owner's Liquid has unsettled repayment purchases, so it cannot close or ratchet.
		LiquidRepaymentPending,
		/// Fissions can join an existing Liquid only during its original creation tick.
		LiquidCreationWindowClosed,
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
				BitcoinFissionLockError::InsufficientFunds => Self::InsufficientFunds,
				BitcoinFissionLockError::ResecuritizationFailed => Self::ResecuritizationFailed,
				BitcoinFissionLockError::InvalidFeeCoupon => Self::InvalidFeeCoupon,
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			Self::prune_r_value_history();
			T::WeightInfo::prune_r_values(T::MaxRValueHistoryPrunesPerBlock::get())
		}
		#[cfg(feature = "try-runtime")]
		fn try_state(_: BlockNumberFor<T>) -> Result<(), sp_runtime::TryRuntimeError> {
			crate::try_state::check::<T>()
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
			fee_coupon: Option<T::FeeCoupon>,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			ensure!(liquid_id != 0, Error::<T>::InvalidLiquidId);
			let minimum_fission_id = NextFissionIdByOwner::<T>::get(&account_id);
			ensure!(fission_id >= minimum_fission_id, Error::<T>::FissionIdBelowMinimum);
			ensure!(satoshis > 0, Error::<T>::FissionHasNoSatoshis);
			ensure!(
				!FissionByOwnerAndId::<T>::contains_key(&account_id, fission_id),
				Error::<T>::FissionAlreadyExists
			);

			let timestamp = T::TimeProvider::now().as_millis().saturated_into::<u64>();
			let created_at_tick = T::TickProvider::ticker().tick_for_time(timestamp);
			LiquidTicksByOwnerAndId::<T>::try_mutate(&account_id, liquid_id, |ticks| {
				if let Some(ticks) = ticks {
					ensure!(
						ticks.created_at_tick == Some(created_at_tick),
						Error::<T>::LiquidCreationWindowClosed
					);
				} else {
					*ticks = Some(LiquidTicks {
						created_at_tick: Some(created_at_tick),
						last_ratchet_tick: None,
					});
				}
				Ok::<_, DispatchError>(())
			})?;

			let next_fission_id = fission_id.checked_add(1).ok_or(Error::<T>::FissionIdOverflow)?;
			FissionIdsByOwnerAndLiquid::<T>::insert((&account_id, liquid_id), fission_id, ());

			FissionIdsByLockId::<T>::try_mutate(lock_id, |fission_ids| {
				fission_ids
					.try_insert(fission_id)
					.map(|_| ())
					.map_err(|_| Error::<T>::TooManyFissionsForLock)
			})?;

			let (liquidity_promised, last_ratchet_price_tick, cosigner_guarantee) =
				T::LockProvider::fission_satoshis(
					&account_id,
					lock_id,
					satoshis,
					microgons_at_target_per_btc,
					fee_coupon,
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
					last_ratchet_price_tick,
					liquidity_promised,
					cosigner_guarantee,
					created_at_argon_block: block_number,
					ratchet_number: 0,
					last_updated_argon_block: block_number,
				},
			);
			NextFissionIdByOwner::<T>::insert(&account_id, next_fission_id);

			T::PositionProvider::account_quantity_updated(
				&account_id,
				PositionQuantity::FissionLiquidity,
				T::Balance::zero(),
				liquidity_promised,
			)?;
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
			fee_coupon: Option<T::FeeCoupon>,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			let mut fission = FissionByOwnerAndId::<T>::get(&account_id, fission_id)
				.ok_or(Error::<T>::FissionNotFound)?;
			ensure!(
				!T::LiquidRepaymentProvider::has_pending_repayment(&account_id, fission.liquid_id),
				Error::<T>::LiquidRepaymentPending
			);

			let ratchet_number =
				fission.ratchet_number.checked_add(1).ok_or(Error::<T>::RatchetNumberOverflow)?;
			let difference = if microgons_at_target_per_btc >= fission.microgons_at_target_per_btc {
				microgons_at_target_per_btc.saturating_sub(fission.microgons_at_target_per_btc)
			} else {
				fission.microgons_at_target_per_btc.saturating_sub(microgons_at_target_per_btc)
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

			let last_ratchet_price_tick = match T::LockProvider::update_fission_guarantee(
				&account_id,
				fission.lock_id,
				fission.satoshis,
				&mut fission.cosigner_guarantee,
				microgons_at_target_per_btc,
				fission.last_ratchet_price_tick,
				fee_coupon,
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
			T::PositionProvider::account_quantity_updated(
				&account_id,
				PositionQuantity::FissionLiquidity,
				prior_liquidity_promised,
				liquidity_promised,
			)?;

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

			let timestamp = T::TimeProvider::now().as_millis().saturated_into::<u64>();
			let ratchet_tick = T::TickProvider::ticker().tick_for_time(timestamp);
			LiquidTicksByOwnerAndId::<T>::mutate(&account_id, fission.liquid_id, |ticks| {
				let ticks = ticks.get_or_insert_with(LiquidTicks::default);
				ticks.last_ratchet_tick = Some(ratchet_tick);
			});

			fission.microgons_at_target_per_btc = microgons_at_target_per_btc;
			fission.last_ratchet_price_tick = last_ratchet_price_tick;
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

			let satoshis = fission.satoshis;
			let liquid_id = fission.liquid_id;
			ensure!(
				!T::LiquidRepaymentProvider::has_pending_repayment(&account_id, liquid_id),
				Error::<T>::LiquidRepaymentPending
			);

			let redemption = T::LockProvider::calculate_redemption_amount(
				satoshis,
				fission.microgons_at_target_per_btc,
			)
			.map_err(Error::<T>::from)?;
			let liquidity =
				Self::apply_repayment(&account_id, fission_id, fission, satoshis, redemption)?;
			T::Currency::burn_from(
				&account_id,
				redemption,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)
			.map_err(|_| Error::<T>::InsufficientFunds)?;
			Self::record_repayment(&account_id, redemption, liquidity)?;
			FissionIdsByOwnerAndLiquid::<T>::remove((&account_id, liquid_id), fission_id);
			Ok(())
		}
	}
}

impl<T: Config> BitcoinFissionsProvider<T::AccountId, T::Balance> for Pallet<T> {
	type Weights = weights::ProviderWeightAdapter<T>;

	fn is_liquid_repayment_valid(
		account_id: &T::AccountId,
		liquid_id: LiquidId,
		purchase_tick: Tick,
		hold_frames: FrameId,
	) -> bool {
		let Some(ticks) = LiquidTicksByOwnerAndId::<T>::get(account_id, liquid_id) else {
			return false;
		};
		let last_change_tick = ticks.last_ratchet_tick.or(ticks.created_at_tick);
		// Keep one tick between debt changes and the purchase to allow for clock differences.
		if last_change_tick.is_some_and(|tick| purchase_tick <= tick.saturating_add(1)) {
			return false;
		}

		let mut found = false;
		for id in FissionIdsByOwnerAndLiquid::<T>::iter_key_prefix((account_id, liquid_id)) {
			found = true;
			let Some(fission) = FissionByOwnerAndId::<T>::get(account_id, id) else { return false };
			if !T::LockProvider::can_repay_before_expiry(fission.lock_id, hold_frames) {
				return false;
			}
		}
		found
	}

	fn next_r_value_high_water(from_frame: FrameId) -> Option<(FrameId, RValueHighWaterMark)> {
		let mut history = RValueHighWaterByFrame::<T>::iter();
		if let Some(previous_frame) = from_frame.checked_sub(1) {
			history = RValueHighWaterByFrame::<T>::iter_from(
				RValueHighWaterByFrame::<T>::hashed_key_for(previous_frame.to_ordered()),
			);
		}
		history.next().map(|(key, high)| (FrameId::from_ordered(&key), high))
	}

	#[require_transactional]
	fn repay_liquid<BurnCallback: FnMut(T::Balance) -> DispatchResult>(
		account_id: &T::AccountId,
		liquid_id: LiquidId,
		budget: T::Balance,
		prices: LiquidRepayPrices,
		mut burn_callback: BurnCallback,
	) -> Result<T::Balance, DispatchError> {
		let fission_ids: Vec<_> =
			FissionIdsByOwnerAndLiquid::<T>::iter_key_prefix((account_id, liquid_id)).collect();
		// Earlier repayments or terminal Lock cleanup may have removed every Fission.
		if fission_ids.is_empty() {
			return Ok(T::Balance::zero());
		}

		// Repay the earliest-expiring Locks first, using Fission age to break ties.
		let mut ordered_fissions = Vec::with_capacity(fission_ids.len());
		for id in &fission_ids {
			let fission =
				FissionByOwnerAndId::<T>::get(account_id, id).ok_or(Error::<T>::FissionNotFound)?;
			let expiration =
				T::LockProvider::get_lock_expiration(fission.lock_id).map_err(Error::<T>::from)?;
			ordered_fissions.push((expiration, fission.created_at_argon_block, *id, fission));
		}
		ordered_fissions.sort_by(|left, right| {
			(&left.0, &left.1, &left.2).cmp(&(&right.0, &right.1, &right.2))
		});

		// Quote both the purchase commitment that selects Bitcoin and the burn after recovery.
		let quote = |satoshis, original_price: T::Balance| {
			let (committed, burned) =
				prices.quote(satoshis, original_price.into()).ok_or(Error::<T>::Overflow)?;
			Ok::<(T::Balance, T::Balance), DispatchError>((committed.into(), burned.into()))
		};

		// Spend the purchase budget across the ordered Fissions, repaying each in full until
		// only a partial repayment fits. Track burns separately: recovery savings are refunded
		// rather than spent on additional Bitcoin.
		let mut remaining_commitment = budget;
		let mut total_burned = T::Balance::zero();
		let mut total_liquidity_repaid = T::Balance::zero();
		for (_, _, id, fission) in ordered_fissions {
			let (full_commitment, full_burn) =
				quote(fission.satoshis, fission.microgons_at_target_per_btc)?;
			let mut satoshis = fission.satoshis;
			let partial = full_commitment > remaining_commitment;
			if partial {
				// Find the largest whole-satoshi amount this budget can redeem. BTC valuation
				// rounds before the curve multiplier, so use the exact quote for each candidate.
				let mut low = 0;
				let mut high = satoshis - 1;
				while low < high {
					let middle = high - (high - low) / 2;
					let (committed_microgons, _) =
						quote(middle, fission.microgons_at_target_per_btc)?;
					if committed_microgons <= remaining_commitment {
						low = middle;
					} else {
						high = middle - 1;
					}
				}
				satoshis = low;
			}

			if satoshis == 0 {
				break;
			}

			let closes_fission = satoshis == fission.satoshis;
			let (committed_microgons, burned_microgons) = if partial {
				quote(satoshis, fission.microgons_at_target_per_btc)?
			} else {
				(full_commitment, full_burn)
			};
			let liquidity_repaid =
				Self::apply_repayment(account_id, id, fission, satoshis, burned_microgons)?;

			remaining_commitment = remaining_commitment
				.checked_sub(&committed_microgons)
				.ok_or(Error::<T>::RepaymentBudgetExceeded)?;
			total_burned =
				total_burned.checked_add(&burned_microgons).ok_or(Error::<T>::Overflow)?;
			total_liquidity_repaid.saturating_accrue(liquidity_repaid);
			if closes_fission {
				FissionIdsByOwnerAndLiquid::<T>::remove((account_id, liquid_id), id);
			}

			if partial {
				break;
			}
		}

		// Burn once for the whole repayment, then retire the repaid mint and liquidity amounts.
		if total_burned > T::Balance::zero() {
			burn_callback(total_burned)?;
		}
		Self::record_repayment(account_id, total_burned, total_liquidity_repaid)?;
		Ok(total_burned)
	}

	fn get_account_fission_liquidity(account_id: &T::AccountId) -> T::Balance {
		T::PositionProvider::account_quantities(account_id).fission_liquidity
	}

	fn get_lock_fission_guarantees(
		account_id: &T::AccountId,
		lock_id: BitcoinLockId,
	) -> Vec<(BitcoinSecuritizationBasis<T::Balance>, CosignerGuarantee<T::Balance>)> {
		FissionIdsByLockId::<T>::get(lock_id)
			.into_iter()
			.filter_map(|fission_id| {
				let fission = FissionByOwnerAndId::<T>::get(account_id, fission_id)?;
				Some((
					BitcoinSecuritizationBasis {
						satoshis: fission.satoshis,
						microgons_at_target_per_btc: fission.microgons_at_target_per_btc,
						tick: fission.last_ratchet_price_tick,
					},
					fission.cosigner_guarantee,
				))
			})
			.collect()
	}

	fn close_for_lock(
		account_id: &T::AccountId,
		lock_id: BitcoinLockId,
		burned_argons: T::Balance,
	) -> DispatchResult {
		for fission_id in FissionIdsByLockId::<T>::take(lock_id) {
			FissionByOwnerAndId::<T>::try_mutate_exists(
				account_id,
				fission_id,
				|fission| -> DispatchResult {
					let Some(fission) = fission.take() else {
						return Ok(());
					};
					FissionIdsByOwnerAndLiquid::<T>::remove(
						(account_id, fission.liquid_id),
						fission_id,
					);
					T::PositionProvider::account_quantity_updated(
						account_id,
						PositionQuantity::FissionLiquidity,
						fission.liquidity_promised,
						T::Balance::zero(),
					)?;
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
		if !burned_argons.is_zero() {
			T::Minting::record_mint_repayment(burned_argons);
		}

		Ok(())
	}
}

impl<T: Config> Pallet<T> {
	/// Return Bitcoin and update one loaded Fission inside its caller's transaction.
	/// The caller supplies the actual burn quote and retires the returned liability.
	#[require_transactional]
	fn apply_repayment(
		account_id: &T::AccountId,
		fission_id: FissionId,
		mut fission: Fission<T>,
		satoshis: Satoshis,
		redemption_amount: T::Balance,
	) -> Result<T::Balance, DispatchError> {
		ensure!(satoshis > 0, Error::<T>::RepaymentHasNoSatoshis);
		ensure!(satoshis <= fission.satoshis, Error::<T>::RepaymentExceedsFission);

		// Split the promised liquidity and guarantee in proportion to the Bitcoin being repaid.
		let previous_liquidity = fission.liquidity_promised;
		let liquidity_repaid: T::Balance = multiply_by_rational_with_rounding(
			previous_liquidity.into(),
			satoshis.into(),
			fission.satoshis.into(),
			Rounding::Down,
		)
		.ok_or(Error::<T>::Overflow)?
		.into();
		let returned_guarantee = if satoshis == fission.satoshis {
			fission.cosigner_guarantee.clone()
		} else {
			// Guarantee capacity can differ from allocated Bitcoin. Round its repaid share down,
			// then price that capacity at the guarantee's own rate; final repayment takes the rest.
			let returned_capacity = multiply_by_rational_with_rounding(
				fission.cosigner_guarantee.basis.satoshis.into(),
				satoshis.into(),
				fission.satoshis.into(),
				Rounding::Down,
			)
			.and_then(|capacity| Satoshis::try_from(capacity).ok())
			.ok_or(Error::<T>::Overflow)?;
			let returned_microgons =
				fission.cosigner_guarantee.securitization_for_satoshis(returned_capacity);
			fission
				.cosigner_guarantee
				.take_portion(returned_capacity, returned_microgons)
				.ok_or(Error::<T>::Overflow)?
		};

		// Return the allocation to the parent Lock; withdrawing Bitcoin is a separate release.
		T::LockProvider::fuse_satoshis(account_id, fission.lock_id, satoshis, returned_guarantee)
			.map_err(Error::<T>::from)?;

		fission.satoshis.saturating_reduce(satoshis);
		fission.liquidity_promised.saturating_reduce(liquidity_repaid);

		// Close a fully repaid Fission, or retain its remaining allocation and guarantee.
		if fission.satoshis == 0 {
			FissionByOwnerAndId::<T>::remove(account_id, fission_id);
			FissionIdsByLockId::<T>::mutate_exists(fission.lock_id, |ids| {
				if let Some(ids) = ids.as_mut() {
					ids.remove(&fission_id);
				}
				if ids.as_ref().is_some_and(|ids| ids.is_empty()) {
					*ids = None;
				}
			});
			Self::deposit_event(Event::FissionClosed {
				account_id: account_id.clone(),
				fission_id,
				redemption_amount,
			});
		} else {
			fission.last_updated_argon_block = frame_system::Pallet::<T>::block_number();
			FissionByOwnerAndId::<T>::insert(account_id, fission_id, fission);
			Self::deposit_event(Event::FissionRepaid {
				account_id: account_id.clone(),
				fission_id,
				satoshis,
				liquidity_repaid,
				redemption_amount,
			});
		}
		Ok(liquidity_repaid)
	}

	/// Retire the account's repaid issuance and liquidity once for the whole repayment.
	fn record_repayment(
		account_id: &T::AccountId,
		redemption: T::Balance,
		liquidity: T::Balance,
	) -> DispatchResult {
		T::Minting::record_mint_repayment(redemption);
		T::PositionProvider::account_quantity_updated(
			account_id,
			PositionQuantity::FissionLiquidity,
			liquidity,
			T::Balance::zero(),
		)?;
		T::OperationalAccountsHook::account_bitcoin_amount_changed(account_id, liquidity, false);
		Ok(())
	}
}
