#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
const LOG_TARGET: &str = "runtime::bond";

/// The bond pallet allows users to lock funds for a period of time, and optionally lease them out
/// to other users for a fee. The intended operation is to lock argons into a bond that allow users
/// to perform operations like bidding for mining slots and operating as a vault.
///
/// Terms:
///
/// ** BondFund ** A bond fund is a pool of funds that are offered for bonding. The provider of a
/// bond fund can set their APR (annual percentage return) rate as well as a base fee and expiration
/// date. BondFunds can be extended and/or ended. When a bond fund is ended, it will no longer
/// accept new bond, and will return any expired bond to the bond fund provider once they are
/// completed.
///
/// ** Leasing a Bond ** A user can lease a bond from a bond fund. The user specifies the amount of
/// time they want to lease the bond for, and the amount of the bond they want to lease. The apr on
/// the bond fund determines the upfront fee that will be charged. A bond can be returned at any
/// point and they will receive a prorated return back. If the leaser no longer has funds, the funds
/// will be taken from the bond-fund itself.
///
/// ** Self-Bond ** A user can self-bond, which essentially puts their own funds on hold. This is a
/// convenience if you possess the necessary funds and want to bid for a validator slot without
/// paying fees.
///
///
/// -----
///
/// Bonds are available via the BondProvider trait for use in other pallets. It's used in the
/// mining_slot pallet and expected to also be used in the vaults pallet in the future.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, InspectHold, Mutate, MutateHold},
			tokens::{Fortitude, Precision, Preservation},
			Incrementable,
		},
		BoundedVec,
	};
	use frame_system::pallet_prelude::*;
	use log::warn;
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, CheckedSub, UniqueSaturatedInto},
		DispatchError::Token,
		Saturating, TokenError,
	};

	use ulx_primitives::bond::{Bond, BondError, BondFund, BondProvider, Fee};

	use super::*;

	#[pallet::pallet]
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
			+ sp_std::fmt::Debug
			+ Default
			+ From<u128>
			+ TryInto<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		/// Identifier for the bond fund id
		type BondFundId: Member
			+ Parameter
			+ MaxEncodedLen
			+ Copy
			+ Incrementable
			+ AtLeast32BitUnsigned;

		/// Identifier for the bond id
		type BondId: Member
			+ Parameter
			+ MaxEncodedLen
			+ Copy
			+ Incrementable
			+ AtLeast32BitUnsigned;

		/// Minimum amount for a bond
		#[pallet::constant]
		type MinimumBondAmount: Get<Self::Balance>;

		/// Blocks per year used for APR calculations
		#[pallet::constant]
		type BlocksPerYear: Get<BlockNumberFor<Self>>;

		/// Pallet storage requires bounds, so we have to set a maximum number that can expire in a
		/// single block
		#[pallet::constant]
		type MaxConcurrentlyExpiringBondFunds: Get<u32>;
		/// Pallet storage requires bounds, so we have to set a maximum number that can expire in a
		/// single block
		#[pallet::constant]
		type MaxConcurrentlyExpiringBonds: Get<u32>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		EnterBondFund,
	}

	#[pallet::storage]
	pub(super) type NextBondId<T: Config> = StorageValue<_, T::BondId, OptionQuery>;
	#[pallet::storage]
	pub(super) type NextBondFundId<T: Config> = StorageValue<_, T::BondFundId, OptionQuery>;

	/// BondFunds by id
	#[pallet::storage]
	pub(super) type BondFunds<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::BondFundId,
		BondFund<T::AccountId, T::Balance, BlockNumberFor<T>>,
		OptionQuery,
	>;
	/// Expiration block number for each bond fund
	#[pallet::storage]
	pub(super) type BondFundExpirations<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<T::BondFundId, T::MaxConcurrentlyExpiringBondFunds>,
		ValueQuery,
	>;

	/// Bonds by id
	#[pallet::storage]
	pub(super) type Bonds<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::BondId,
		Bond<T::AccountId, T::Balance, BlockNumberFor<T>, T::BondFundId>,
		OptionQuery,
	>;
	/// Completion of each bond, upon which date funds are returned to the bond fund or self-bonder
	#[pallet::storage]
	pub(super) type BondCompletions<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<T::BondId, T::MaxConcurrentlyExpiringBonds>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		BondFundOffered {
			bond_fund_id: T::BondFundId,
			amount_offered: T::Balance,
			expiration_block: BlockNumberFor<T>,
			offer_account_id: T::AccountId,
		},
		BondFundExtended {
			bond_fund_id: T::BondFundId,
			amount_offered: T::Balance,
			expiration_block: BlockNumberFor<T>,
		},
		BondFundEnded {
			bond_fund_id: T::BondFundId,
			amount_still_bonded: T::Balance,
		},
		BondFundExpired {
			bond_fund_id: T::BondFundId,
			offer_account_id: T::AccountId,
		},
		BondedSelf {
			bond_id: T::BondId,
			bonded_account_id: T::AccountId,
			amount: T::Balance,
			completion_block: BlockNumberFor<T>,
		},
		BondLeased {
			bond_fund_id: T::BondFundId,
			bond_id: T::BondId,
			bonded_account_id: T::AccountId,
			amount: T::Balance,
			total_fee: T::Balance,
			annual_percent_rate: u32,
			completion_block: BlockNumberFor<T>,
		},
		BondExtended {
			bond_fund_id: Option<T::BondFundId>,
			bond_id: T::BondId,
			amount: T::Balance,
			completion_block: BlockNumberFor<T>,
			fee_change: T::Balance,
			annual_percent_rate: u32,
		},
		BondCompleted {
			bond_fund_id: Option<T::BondFundId>,
			bond_id: T::BondId,
		},
		BondBurned {
			bond_fund_id: Option<T::BondFundId>,
			bond_id: T::BondId,
			amount_burned: T::Balance,
			amount_returned: T::Balance,
		},
		BondFeeRefund {
			bond_fund_id: T::BondFundId,
			bond_id: T::BondId,
			bonded_account_id: T::AccountId,
			bond_fund_reduction_for_payment: T::Balance,
			final_fee: T::Balance,
			refund_amount: T::Balance,
		},
		BondLocked {
			bond_id: T::BondId,
			bonded_account_id: T::AccountId,
		},
		BondUnlocked {
			bond_id: T::BondId,
			bonded_account_id: T::AccountId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		BadState,
		BondNotFound,
		NoMoreBondFundIds,
		NoMoreBondIds,
		MinimumBondAmountNotMet,
		/// There are too many bond or bond funds expiring in the given expiration block
		ExpirationAtBlockOverflow,
		InsufficientFunds,
		InsufficientBondFunds,
		TransactionWouldTakeAccountBelowMinimumBalance,
		BondFundClosed,
		/// This reduction in bond funds offered goes below the amount that is already committed to
		/// bond
		BondFundReductionExceedsAllocatedFunds,
		ExpirationTooSoon,
		LeaseUntilBlockTooSoon,
		LeaseUntilPastFundExpiration,
		NoPermissions,
		NoBondFundFound,
		FundExtensionMustBeLater,
		HoldUnexpectedlyModified,
		BondFundMaximumBondsExceeded,
		UnrecoverableHold,
		BondFundNotFound,
		BondAlreadyLocked,
		BondLockedCannotModify,
		/// The fee for this bond exceeds the amount of the bond, which is unsafe
		FeeExceedsBondAmount,
	}

	impl<T> From<BondError> for Error<T> {
		fn from(e: BondError) -> Error<T> {
			match e {
				BondError::BadState => Error::<T>::BadState,
				BondError::BondNotFound => Error::<T>::BondNotFound,
				BondError::NoMoreBondIds => Error::<T>::NoMoreBondIds,
				BondError::MinimumBondAmountNotMet => Error::<T>::MinimumBondAmountNotMet,
				BondError::ExpirationAtBlockOverflow => Error::<T>::ExpirationAtBlockOverflow,
				BondError::InsufficientFunds => Error::<T>::InsufficientFunds,
				BondError::InsufficientBondFunds => Error::<T>::InsufficientBondFunds,
				BondError::ExpirationTooSoon => Error::<T>::ExpirationTooSoon,
				BondError::NoPermissions => Error::<T>::NoPermissions,
				BondError::NoBondFundFound => Error::<T>::NoBondFundFound,
				BondError::HoldUnexpectedlyModified => Error::<T>::HoldUnexpectedlyModified,
				BondError::BondFundMaximumBondsExceeded => Error::<T>::BondFundMaximumBondsExceeded,
				BondError::UnrecoverableHold => Error::<T>::UnrecoverableHold,
				BondError::BondFundNotFound => Error::<T>::BondFundNotFound,
				BondError::BondAlreadyLocked => Error::<T>::BondAlreadyLocked,
				BondError::BondLockedCannotModify => Error::<T>::BondLockedCannotModify,
				BondError::FeeExceedsBondAmount => Error::<T>::FeeExceedsBondAmount,
				BondError::LeaseUntilBlockTooSoon => Error::<T>::LeaseUntilBlockTooSoon,
				BondError::LeaseUntilPastFundExpiration => Error::<T>::LeaseUntilPastFundExpiration,
				BondError::BondAlreadyClosed => Error::<T>::BondLockedCannotModify,
				BondError::BondFundClosed => Error::<T>::BondFundClosed,
				BondError::AccountWouldBeBelowMinimum =>
					Error::<T>::TransactionWouldTakeAccountBelowMinimumBalance,
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			let bond_completions = BondCompletions::<T>::take(block_number);
			for bond_id in bond_completions {
				Self::bond_completed(bond_id)
					.map_err(|err| {
						warn!( target: LOG_TARGET, "Bond id {:?} failed to `complete` {:?}", bond_id, err);
					})
					.ok();
			}

			let fund_expirations = BondFundExpirations::<T>::take(block_number);
			for bond_fund_id in fund_expirations {
				if let Some(bond_fund) = BondFunds::<T>::take(bond_fund_id) {
					Self::release_hold(&bond_fund.offer_account_id, bond_fund.amount_reserved)
						.map_err(|err| {
							warn!( target: LOG_TARGET, "Bond fund {:?} failed to `release hold` {:?}", bond_fund, err);
						})
						.ok();
					Self::deposit_event(Event::BondFundExpired {
						bond_fund_id,
						offer_account_id: bond_fund.offer_account_id,
					});
				} else {
					warn!( target: LOG_TARGET, "Bond fund id {:?} expired but was not found", bond_fund_id);
				}
			}
			T::DbWeight::get().reads_writes(2, 1)
		}

		fn on_finalize(_: BlockNumberFor<T>) {}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn offer_fund(
			origin: OriginFor<T>,
			#[pallet::compact] lease_annual_percent_rate: u32,
			#[pallet::compact] lease_base_fee: T::Balance,
			#[pallet::compact] amount_offered: T::Balance,
			expiration_block: BlockNumberFor<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if amount_offered < T::MinimumBondAmount::get() {
				return Err(Error::<T>::MinimumBondAmountNotMet.into());
			}

			if expiration_block <= frame_system::Pallet::<T>::block_number() {
				return Err(Error::<T>::ExpirationTooSoon.into());
			}

			Self::hold(&who, amount_offered).map_err(Error::<T>::from)?;

			let bond_fund_id = Self::next_bond_fund_id()?;

			let bond_fund = BondFund {
				lease_annual_percent_rate,
				lease_base_fee,
				offer_account_id: who.clone(),
				amount_reserved: amount_offered,
				amount_bonded: 0u32.into(),
				offer_expiration_block: expiration_block,
				is_ended: false,
			};
			BondFunds::<T>::insert(bond_fund_id, bond_fund);
			BondFundExpirations::<T>::try_mutate(expiration_block, |funds| {
				funds.try_push(bond_fund_id)
			})
			.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;
			Self::deposit_event(Event::BondFundOffered {
				bond_fund_id,
				amount_offered,
				expiration_block,
				offer_account_id: who,
			});

			Ok(())
		}

		/// Stop offering this fund for new bond. Will not affect existing bond. Unreserved funds
		/// are returned immediately.
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn end_fund(origin: OriginFor<T>, bond_fund_id: T::BondFundId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut fund =
				BondFunds::<T>::get(bond_fund_id).ok_or::<Error<T>>(Error::<T>::NoBondFundFound)?;

			if fund.offer_account_id != who {
				return Err(Error::<T>::NoPermissions.into());
			}

			let return_amount = fund.amount_reserved.saturating_sub(fund.amount_bonded);
			if Self::held_balance(&who) < return_amount {
				return Err(Error::<T>::HoldUnexpectedlyModified.into());
			}
			Self::release_hold(&who, return_amount)?;

			fund.is_ended = true;
			let amount_still_bonded = fund.amount_bonded;
			fund.amount_reserved = amount_still_bonded;
			BondFunds::<T>::set(bond_fund_id, Some(fund));

			Self::deposit_event(Event::BondFundEnded { bond_fund_id, amount_still_bonded });
			Ok(())
		}

		/// Add additional time or funds to the bond fund
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn extend_fund(
			origin: OriginFor<T>,
			bond_fund_id: T::BondFundId,
			total_amount_offered: T::Balance,
			expiration_block: BlockNumberFor<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut fund = BondFunds::<T>::get(bond_fund_id).ok_or(Error::<T>::NoBondFundFound)?;
			if fund.offer_account_id != who {
				return Err(Error::<T>::NoPermissions.into());
			}
			if fund.offer_expiration_block > expiration_block {
				return Err(Error::<T>::FundExtensionMustBeLater.into());
			}

			if total_amount_offered < fund.amount_bonded {
				return Err(Error::<T>::BondFundReductionExceedsAllocatedFunds.into());
			}

			if fund.amount_reserved > total_amount_offered {
				let return_amount = fund.amount_reserved.saturating_sub(total_amount_offered);
				Self::release_hold(&who, return_amount)?;
			} else {
				let amount_to_reserve = total_amount_offered.saturating_sub(fund.amount_reserved);
				Self::hold(&who, amount_to_reserve).map_err(Error::<T>::from)?;
			}

			if expiration_block != fund.offer_expiration_block {
				Self::remove_bond_fund_expiration(bond_fund_id, fund.offer_expiration_block);

				BondFundExpirations::<T>::try_mutate(expiration_block, |funds| {
					funds.try_push(bond_fund_id)
				})
				.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;

				fund.offer_expiration_block = expiration_block;
			}
			BondFunds::<T>::set(bond_fund_id, Some(fund));
			Self::deposit_event(Event::BondFundExtended {
				bond_fund_id,
				amount_offered: total_amount_offered,
				expiration_block,
			});

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn bond_self(
			origin: OriginFor<T>,
			amount: T::Balance,
			bond_until_block: BlockNumberFor<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as BondProvider>::bond_self(who, amount, bond_until_block)
				.map_err(Error::<T>::from)?;

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(0)]
		pub fn lease(
			origin: OriginFor<T>,
			bond_fund_id: T::BondFundId,
			amount: T::Balance,
			lease_until_block: BlockNumberFor<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as BondProvider>::lease(bond_fund_id, who, amount, lease_until_block)
				.map_err(Error::<T>::from)?;
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(0)]
		pub fn return_bond(origin: OriginFor<T>, bond_id: T::BondId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as BondProvider>::return_bond(bond_id, who).map_err(Error::<T>::from)?;
			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(0)]
		pub fn extend_bond(
			origin: OriginFor<T>,
			bond_id: T::BondId,
			total_amount: T::Balance,
			bond_until_block: BlockNumberFor<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as BondProvider>::extend_bond(bond_id, who, total_amount, bond_until_block)
				.map_err(Error::<T>::from)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn next_bond_id() -> Result<T::BondId, BondError> {
			let bond_id =
				NextBondId::<T>::get().or(Some(1u32.into())).ok_or(BondError::NoMoreBondIds)?;
			let next_bond_id = bond_id.increment().ok_or(BondError::NoMoreBondIds)?;
			NextBondId::<T>::set(Some(next_bond_id));
			Ok(bond_id)
		}

		fn next_bond_fund_id() -> Result<T::BondFundId, Error<T>> {
			let bond_fund_id = NextBondFundId::<T>::get()
				.or(Some(1u32.into()))
				.ok_or(Error::<T>::NoMoreBondFundIds)?;
			let next_bond_fund_id =
				bond_fund_id.increment().ok_or(Error::<T>::NoMoreBondFundIds)?;
			NextBondFundId::<T>::set(Some(next_bond_fund_id));
			Ok(bond_fund_id)
		}

		fn hold(who: &T::AccountId, amount: T::Balance) -> Result<(), BondError> {
			T::Currency::hold(&HoldReason::EnterBondFund.into(), who, amount).map_err(|e| {
				let balance = T::Currency::balance(who);
				warn!( target: LOG_TARGET, "Hold failed for {:?} from {:?}. Current Balance={:?}. {:?}", amount, who, balance, e);

				match e {
					Token(TokenError::BelowMinimum) => BondError::AccountWouldBeBelowMinimum,
					_ => {
						if balance.checked_sub(&amount).is_some()  && balance.saturating_sub(amount) <
							T::Currency::minimum_balance()
						{
							return BondError::AccountWouldBeBelowMinimum
						}

						BondError::InsufficientFunds
					},
				}
			})?;
			frame_system::Pallet::<T>::inc_providers(who);
			Ok(())
		}

		fn release_hold(
			who: &T::AccountId,
			amount: T::Balance,
		) -> Result<T::Balance, DispatchError> {
			let reason = &HoldReason::EnterBondFund.into();
			if amount == Self::held_balance(who) {
				let _ = frame_system::Pallet::<T>::dec_providers(who);
			}
			T::Currency::release(reason, who, amount, Precision::Exact)
		}

		fn burn_hold(who: &T::AccountId, amount: T::Balance) -> Result<T::Balance, DispatchError> {
			let reason = &HoldReason::EnterBondFund.into();
			if amount == Self::held_balance(&who) {
				let _ = frame_system::Pallet::<T>::dec_providers(&who);
			}
			T::Currency::burn_held(&reason, &who, amount, Precision::Exact, Fortitude::Force)
		}

		fn held_balance(who: &T::AccountId) -> T::Balance {
			let reason = &HoldReason::EnterBondFund.into();
			T::Currency::balance_on_hold(reason, who)
		}

		fn transfer(
			from: &T::AccountId,
			to: &T::AccountId,
			amount: T::Balance,
		) -> Result<(), BondError> {
			T::Currency::transfer(from, to, amount, Preservation::Preserve).map_err(|e| {
				warn!( target: LOG_TARGET, "Transfer failed for {:?} from {:?} to {:?} {:?}", amount, from, to, e);
				match e {
					Token(TokenError::BelowMinimum) => BondError::AccountWouldBeBelowMinimum,
					_ => BondError::InsufficientFunds,
				}
			})?;
			Ok(())
		}

		fn can_transfer(from: &T::AccountId, amount: T::Balance) -> bool {
			T::Currency::reducible_balance(from, Preservation::Preserve, Fortitude::Force) >= amount
		}

		/// Return bonded funds to a bond fund or to the self-bonder if necessary
		fn bond_completed(bond_id: T::BondId) -> DispatchResult {
			let bond = Bonds::<T>::take(bond_id).ok_or(Error::<T>::BondNotFound)?;
			Self::remove_bond_completion(bond_id, bond.completion_block);
			Self::deposit_event(Event::BondCompleted { bond_fund_id: bond.bond_fund_id, bond_id });
			match bond.bond_fund_id {
				None => {
					let unreserved = Self::release_hold(&bond.bonded_account_id, bond.amount)?;
					if unreserved != bond.amount {
						warn!(
							"Expiring bond hold could not all be returned {:?} - remaining not un-reserved ({:?}).",
							bond, unreserved
						);
						return Err(Error::<T>::UnrecoverableHold.into());
					}
					Ok(())
				},
				Some(bond_fund_id) => {
					if let Some(mut bond_fund) = BondFunds::<T>::get(bond_fund_id) {
						// return bond amount to fund
						bond_fund.amount_bonded =
							bond_fund.amount_bonded.saturating_sub(bond.amount);
						if bond_fund.is_ended {
							Self::release_hold(&bond_fund.offer_account_id, bond.amount)
								.map_err(|_| Error::<T>::UnrecoverableHold)?;
							bond_fund.amount_reserved =
								bond_fund.amount_reserved.saturating_sub(bond.amount);
						}

						if bond_fund.amount_bonded == 0u32.into() && bond_fund.is_ended {
							BondFunds::<T>::take(bond_fund_id);
							Self::remove_bond_fund_expiration(
								bond_fund_id,
								bond_fund.offer_expiration_block,
							);
						} else {
							BondFunds::<T>::set(bond_fund_id, Some(bond_fund));
						}
						Ok(())
					} else {
						Err(Error::<T>::BondFundNotFound.into())
					}
				},
			}
		}

		fn remove_bond_completion(bond_id: T::BondId, completion_block: BlockNumberFor<T>) {
			if !BondCompletions::<T>::contains_key(completion_block) {
				return;
			}
			BondCompletions::<T>::mutate(completion_block, |bonds| {
				if let Some(index) = bonds.iter().position(|b| *b == bond_id) {
					bonds.remove(index);
				}
			});
		}

		fn remove_bond_fund_expiration(
			bond_fund_id: T::BondFundId,
			expiration_block: BlockNumberFor<T>,
		) {
			if !BondFundExpirations::<T>::contains_key(expiration_block) {
				return;
			}
			BondFundExpirations::<T>::mutate(expiration_block, |funds| {
				if let Some(index) = funds.iter().position(|id| *id == bond_fund_id) {
					funds.remove(index);
				}
			});
		}

		pub fn calculate_fees(
			annual_percentage_rate: u32,
			base_fee: T::Balance,
			amount: T::Balance,
			blocks: BlockNumberFor<T>,
			blocks_per_year: BlockNumberFor<T>,
		) -> T::Balance {
			let amount: u128 = amount.try_into().unwrap_or(0u128);
			let percent_basis = 100_000u128;
			let lease_price: u128 = amount
				.saturating_mul(annual_percentage_rate.into())
				.saturating_mul(blocks_into_u128::<T>(blocks))
				.checked_div(blocks_into_u128::<T>(blocks_per_year))
				.unwrap_or_default()
				.checked_div(percent_basis)
				.unwrap_or_default(); // amount is in milligons

			let lease_price = T::Balance::from(lease_price);

			base_fee.saturating_add(lease_price)
		}

		pub fn charge_lease_fees(
			bond_fund_id: <T as Config>::BondFundId,
			amount: T::Balance,
			lease_until_block: BlockNumberFor<T>,
			who: &T::AccountId,
			block_number: BlockNumberFor<T>,
		) -> Result<Fee<T::Balance>, BondError> {
			let mut bond_fund =
				BondFunds::<T>::get(bond_fund_id).ok_or::<BondError>(BondError::NoBondFundFound)?;

			if bond_fund.amount_reserved.saturating_sub(bond_fund.amount_bonded) < amount {
				log::info!(
					"Insufficient bond funds for bond fund {:?} amount {:?} lease_until_block {:?}",
					bond_fund,
					amount,
					lease_until_block
				);
				return Err(BondError::InsufficientBondFunds);
			}

			if bond_fund.is_ended {
				return Err(BondError::BondFundClosed);
			}

			if lease_until_block > bond_fund.offer_expiration_block {
				return Err(BondError::LeaseUntilPastFundExpiration);
			}

			let base_fee = bond_fund.lease_base_fee;
			let apr = bond_fund.lease_annual_percent_rate;
			let fee = Self::calculate_fees(
				apr,
				base_fee,
				amount,
				lease_until_block - block_number,
				T::BlocksPerYear::get(),
			);

			Self::transfer(who, &bond_fund.offer_account_id, fee)?;

			bond_fund.amount_bonded = bond_fund.amount_bonded.saturating_add(amount);
			BondFunds::<T>::set(bond_fund_id, Some(bond_fund));

			Ok(Fee { total_fee: fee, base_fee, annual_percent_rate: apr })
		}
	}

	impl<T: Config> BondProvider for Pallet<T> {
		type BondFundId = T::BondFundId;
		type BondId = T::BondId;
		type Balance = T::Balance;
		type AccountId = T::AccountId;
		type BlockNumber = BlockNumberFor<T>;

		fn get_bond(
			bond_id: Self::BondId,
		) -> Result<Bond<T::AccountId, T::Balance, BlockNumberFor<T>, Self::BondFundId>, BondError>
		{
			Bonds::<T>::get(bond_id).ok_or(BondError::BondNotFound)
		}

		fn bond_self(
			account_id: T::AccountId,
			amount: T::Balance,
			bond_until_block: BlockNumberFor<T>,
		) -> Result<Self::BondId, BondError> {
			if amount < T::MinimumBondAmount::get() {
				return Err(BondError::MinimumBondAmountNotMet);
			}

			let block_number = frame_system::Pallet::<T>::block_number();
			if bond_until_block <= block_number {
				return Err(BondError::ExpirationTooSoon);
			}

			Self::hold(&account_id, amount)?;
			let bond_id = Self::next_bond_id()?;

			let bond = Bond {
				bond_fund_id: None,
				bonded_account_id: account_id.clone(),
				amount,
				start_block: block_number,
				completion_block: bond_until_block,
				annual_percent_rate: 0u32,
				base_fee: 0u32.into(),
				fee: 0u32.into(),
				is_locked: false,
			};
			Bonds::<T>::set(bond_id, Some(bond));
			BondCompletions::<T>::try_mutate(bond_until_block, |bonds| bonds.try_push(bond_id))
				.map_err(|_| BondError::ExpirationAtBlockOverflow)?;

			Self::deposit_event(Event::BondedSelf {
				bond_id,
				bonded_account_id: account_id,
				amount,
				completion_block: bond_until_block,
			});

			Ok(bond_id)
		}

		fn lease(
			bond_fund_id: Self::BondFundId,
			account_id: T::AccountId,
			amount: T::Balance,
			lease_until_block: BlockNumberFor<T>,
		) -> Result<Self::BondId, BondError> {
			if amount < T::MinimumBondAmount::get() {
				return Err(BondError::MinimumBondAmountNotMet);
			}
			let block_number = frame_system::Pallet::<T>::block_number();
			if lease_until_block <= block_number {
				return Err(BondError::LeaseUntilBlockTooSoon);
			}

			let fee = Self::charge_lease_fees(
				bond_fund_id,
				amount,
				lease_until_block,
				&account_id,
				block_number,
			)?;

			if fee.total_fee > amount {
				return Err(BondError::FeeExceedsBondAmount);
			}

			let bond_id = Self::next_bond_id()?;
			let bond = Bond {
				bond_fund_id: Some(bond_fund_id),
				bonded_account_id: account_id.clone(),
				amount,
				start_block: block_number,
				completion_block: lease_until_block,
				base_fee: fee.base_fee,
				annual_percent_rate: fee.annual_percent_rate,
				fee: fee.total_fee,
				is_locked: false,
			};
			Bonds::<T>::set(bond_id, Some(bond));
			BondCompletions::<T>::try_mutate(lease_until_block, |bonds| bonds.try_push(bond_id))
				.map_err(|_| BondError::ExpirationAtBlockOverflow)?;

			Self::deposit_event(Event::BondLeased {
				bond_fund_id,
				bond_id,
				bonded_account_id: account_id,
				amount,
				completion_block: lease_until_block,
				annual_percent_rate: fee.annual_percent_rate,
				total_fee: fee.total_fee,
			});

			Ok(bond_id)
		}

		fn burn_bond(
			bond_id: T::BondId,
			final_amount: Option<T::Balance>,
		) -> Result<(), BondError> {
			let bond = Bonds::<T>::take(bond_id).ok_or(BondError::BondNotFound)?;
			Self::remove_bond_completion(bond_id, bond.completion_block);
			let amount_burned = final_amount.unwrap_or(bond.amount);
			let amount_returned = bond.amount.saturating_sub(amount_burned);
			ensure!(amount_burned <= bond.amount, BondError::InsufficientFunds);

			Self::deposit_event(Event::BondBurned {
				bond_fund_id: bond.bond_fund_id,
				bond_id,
				amount_burned,
				amount_returned,
			});
			match bond.bond_fund_id {
				None => {
					let unreserved = Self::burn_hold(&bond.bonded_account_id, amount_burned)
						.map_err(|_| BondError::UnrecoverableHold)?;
					if unreserved != amount_burned {
						warn!(
							"Expiring bond hold could not all be burned {:?}. Final amount {:?} - remaining not un-reserved ({:?}).",
							bond, final_amount, unreserved
						);
						return Err(BondError::UnrecoverableHold);
					}
					Ok(())
				},
				Some(bond_fund_id) => {
					if let Some(mut bond_fund) = BondFunds::<T>::get(bond_fund_id) {
						// return bond amount to fund
						bond_fund.amount_bonded = bond_fund
							.amount_bonded
							.saturating_sub(bond.amount)
							.saturating_add(amount_returned);
						bond_fund.amount_reserved = bond_fund
							.amount_reserved
							.saturating_sub(bond.amount)
							.saturating_add(amount_returned);

						Self::burn_hold(&bond_fund.offer_account_id, amount_burned)
							.map_err(|_| BondError::UnrecoverableHold)?;

						if bond_fund.amount_bonded == 0u32.into() && bond_fund.is_ended {
							BondFunds::<T>::take(bond_fund_id);
							Self::remove_bond_fund_expiration(
								bond_fund_id,
								bond_fund.offer_expiration_block,
							);
						} else {
							BondFunds::<T>::set(bond_fund_id, Some(bond_fund));
						}
						Ok(())
					} else {
						Err(BondError::BondFundNotFound)
					}
				},
			}
		}

		fn return_bond(bond_id: T::BondId, account_id: T::AccountId) -> Result<(), BondError> {
			let bond = Bonds::<T>::get(bond_id).ok_or(BondError::BondNotFound)?;
			if bond.is_locked {
				return Err(BondError::BondLockedCannotModify);
			}
			if bond.bonded_account_id != account_id {
				return Err(BondError::NoPermissions);
			}

			// if own bond, go ahead and return it
			if bond.bond_fund_id.is_none() {
				return Ok(Self::bond_completed(bond_id).map_err(|_| BondError::UnrecoverableHold))?;
			}

			let bond_fund_id = bond.bond_fund_id.ok_or(BondError::NoBondFundFound)?;

			let mut bond_fund =
				BondFunds::<T>::get(bond_fund_id).ok_or::<BondError>(BondError::NoBondFundFound)?;

			let current_block_number = frame_system::Pallet::<T>::block_number();
			let remaining_blocks =
				blocks_into_u32::<T>(bond.completion_block - current_block_number);
			if remaining_blocks == 0 {
				return Err(BondError::BondAlreadyClosed);
			}
			let updated_fee: T::Balance = Self::calculate_fees(
				// use rate stored on bond in case it change
				bond.annual_percent_rate,
				// don't refund base fee
				bond.base_fee,
				bond.amount,
				current_block_number - bond.start_block,
				T::BlocksPerYear::get(),
			);

			let refund_amount = bond.fee - updated_fee;

			if refund_amount > 0u32.into() {
				let offer_account_id = &bond_fund.offer_account_id.clone();
				// first try to get from the account
				if Self::can_transfer(offer_account_id, refund_amount) {
					Self::transfer(offer_account_id, &bond.bonded_account_id, refund_amount)?;
					Self::deposit_event(Event::BondFeeRefund {
						bond_fund_id,
						bond_id,
						bonded_account_id: account_id,
						bond_fund_reduction_for_payment: 0u32.into(),
						final_fee: bond.fee - refund_amount,
						refund_amount,
					});
				}
				// if that fails, try to get from the bond fund
				else {
					if bond_fund.amount_reserved < refund_amount {
						// should not be possible!
						return Err(BondError::InsufficientFunds);
					}
					let mut amount_to_pull = refund_amount;
					if T::Currency::balance(offer_account_id) < T::Currency::minimum_balance() {
						amount_to_pull += T::Currency::minimum_balance();
					}
					log::info!(target: LOG_TARGET, "Cannot refund returned bond. Pulling funds from the bond fund instead. refund_amount={:?}, amount_to_satisfy_minimum_balance={:?}, bond_fund_id={:?} bond_id={:?}", refund_amount, amount_to_pull, bond_fund_id, bond_id);
					bond_fund.amount_reserved =
						bond_fund.amount_reserved.saturating_sub(amount_to_pull);
					bond_fund.amount_bonded =
						bond_fund.amount_bonded.saturating_sub(amount_to_pull);
					BondFunds::<T>::set(bond_fund_id, Some(bond_fund));
					// move refund amount out of hold and into bonded account
					Self::release_hold(offer_account_id, amount_to_pull)
						.map_err(|e| {
							warn!( target: LOG_TARGET, "Cannot release hold from bond fund owner amount={:?}, account={:?}, {:?}", amount_to_pull, offer_account_id, e);

							BondError::UnrecoverableHold
						})?;
					Self::transfer(offer_account_id, &bond.bonded_account_id, refund_amount)?;

					Self::deposit_event(Event::BondFeeRefund {
						bond_fund_id,
						bond_id,
						bonded_account_id: account_id,
						bond_fund_reduction_for_payment: amount_to_pull,
						final_fee: bond.fee - refund_amount,
						refund_amount,
					});
				}
			}
			Self::bond_completed(bond_id).map_err(|_| BondError::UnrecoverableHold)?;
			Ok(())
		}

		fn extend_bond(
			bond_id: T::BondId,
			account_id: T::AccountId,
			total_amount: T::Balance,
			lease_until: BlockNumberFor<T>,
		) -> Result<(), BondError> {
			if total_amount < T::MinimumBondAmount::get() {
				return Err(BondError::MinimumBondAmountNotMet);
			}
			let block_number = frame_system::Pallet::<T>::block_number();
			if lease_until <= block_number {
				return Err(BondError::LeaseUntilBlockTooSoon);
			}

			let mut bond = Bonds::<T>::get(bond_id).ok_or(BondError::BondNotFound)?;
			if bond.bonded_account_id != account_id {
				return Err(BondError::NoPermissions);
			}

			// If a bond is locked, it can only be increased
			if bond.is_locked && (total_amount < bond.amount || lease_until < bond.completion_block)
			{
				return Err(BondError::BondLockedCannotModify);
			}

			// if the expiration changed, remove from old slot
			let needs_new_expiration = bond.completion_block != lease_until;
			if needs_new_expiration {
				Self::remove_bond_completion(bond_id, bond.completion_block);
			}
			let start_fee = bond.fee;

			match bond.bond_fund_id {
				None => {
					// if self bonded, adjust hold
					if total_amount > bond.amount {
						Self::hold(&account_id, total_amount - bond.amount)?;
					} else if total_amount < bond.amount {
						Self::release_hold(&account_id, bond.amount - total_amount)
							.map_err(|_| BondError::UnrecoverableHold)?;
					}
				},
				Some(bond_fund_id) => {
					let mut bond_fund = BondFunds::<T>::get(bond_fund_id)
						.ok_or::<BondError>(BondError::NoBondFundFound)?;
					let additional_funds = total_amount.saturating_sub(bond.amount);

					if additional_funds >
						bond_fund.amount_reserved.saturating_sub(bond_fund.amount_bonded)
					{
						return Err(BondError::InsufficientBondFunds);
					}

					if lease_until > bond_fund.offer_expiration_block {
						return Err(BondError::LeaseUntilPastFundExpiration);
					}

					let lease_annual_percent_rate = bond_fund.lease_annual_percent_rate;
					// must take the current fee structure
					let fee = Self::calculate_fees(
						// we pay the current rate to extend
						lease_annual_percent_rate,
						bond_fund.lease_base_fee,
						total_amount,
						lease_until - bond.start_block,
						T::BlocksPerYear::get(),
					);

					if fee > bond.fee {
						let additional_fee = fee - bond.fee;

						Self::transfer(&account_id, &bond_fund.offer_account_id, additional_fee)?;

						bond_fund.amount_bonded =
							bond_fund.amount_bonded.saturating_add(additional_fee);
						BondFunds::<T>::set(bond_fund_id, Some(bond_fund));
					} else if fee < bond.fee {
						let refund_amount = bond.fee - fee;

						// Extensions only support refunding from the payee account
						Self::transfer(&bond_fund.offer_account_id, &account_id, refund_amount)?;

						bond_fund.amount_bonded =
							bond_fund.amount_bonded.saturating_sub(refund_amount);
						BondFunds::<T>::set(bond_fund_id, Some(bond_fund));
					}

					bond.annual_percent_rate = lease_annual_percent_rate;
					bond.fee = fee;
				},
			}

			bond.amount = total_amount;
			bond.completion_block = lease_until;

			Self::deposit_event(Event::BondExtended {
				bond_fund_id: bond.bond_fund_id,
				bond_id,
				amount: total_amount,
				completion_block: lease_until,
				fee_change: bond.fee - start_fee,
				annual_percent_rate: bond.annual_percent_rate,
			});

			Bonds::<T>::set(bond_id, Some(bond));

			if needs_new_expiration {
				BondCompletions::<T>::try_mutate(lease_until, |bonds| bonds.try_push(bond_id))
					.map_err(|_| BondError::ExpirationAtBlockOverflow)?;
			}

			Ok(())
		}

		fn lock_bond(bond_id: T::BondId) -> Result<(), BondError> {
			let mut bond = Bonds::<T>::get(bond_id).ok_or(BondError::BondNotFound)?;
			if bond.is_locked {
				return Err(BondError::BondAlreadyLocked);
			}
			bond.is_locked = true;

			Self::deposit_event(Event::BondLocked {
				bond_id,
				bonded_account_id: bond.bonded_account_id.clone(),
			});
			Bonds::<T>::set(bond_id, Some(bond));

			Ok(())
		}

		fn unlock_bond(bond_id: T::BondId) -> Result<(), BondError> {
			let mut bond = Bonds::<T>::get(bond_id).ok_or(BondError::BondNotFound)?;
			if !bond.is_locked {
				return Ok(());
			}
			bond.is_locked = false;

			Self::deposit_event(Event::BondUnlocked {
				bond_id,
				bonded_account_id: bond.bonded_account_id.clone(),
			});
			Bonds::<T>::set(bond_id, Some(bond));

			Ok(())
		}
	}

	fn blocks_into_u32<T: Config>(blocks: BlockNumberFor<T>) -> u32 {
		UniqueSaturatedInto::<u32>::unique_saturated_into(blocks)
	}

	fn blocks_into_u128<T: Config>(blocks: BlockNumberFor<T>) -> u128 {
		UniqueSaturatedInto::<u128>::unique_saturated_into(blocks)
	}
}
