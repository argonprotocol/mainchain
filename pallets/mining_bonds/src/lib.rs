#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
pub mod weights;
pub use pallet::*;

/// This pallet allows users to contribute to a mining bond fund. Mining bond funds are the only way
/// to collect the bid pools accumulated from the mining slot auctions. Bonds are created at a level
/// that cannot exceed the activated securitization of each vault.
///
/// Each day, capital for tomorrow's mining bond fund is allocated. The prorata for the bid pool,
/// and within each mining bond fund is locked in at the beginning of the slot, and then distributed
/// once that cohort is activated.
///
/// The only limitations to bond funds are:
/// - The maximum number of contributors to a fund (`MaxBondFundContributors`)
/// - The minimum amount of argons per contributor (`MinimumArgonsPerContributor`)
/// - The amount of prorata share each vault can have of the overall bid pool
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use alloc::{collections::BTreeMap, vec};
	use argon_primitives::{
		block_seal::CohortId,
		vault::{MiningBidPoolProvider, MiningBondFundVaultProvider},
		BlockSealAuthorityId, OnNewSlot, VaultId,
	};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, InspectHold, Mutate, MutateHold},
			tokens::{Fortitude, Precision, Preservation},
		},
		BoundedVec, DefaultNoBound, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{AccountIdConversion, AtLeast32BitUnsigned, Zero},
		BoundedBTreeMap, Perbill, Percent, Permill, Saturating,
	};
	use tracing::warn;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config
	where
		<Self as Config>::Balance: Into<u128>,
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		/// The balance type
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

		/// The currency representing argons
		type Currency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>;

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		type MiningBondFundVaultProvider: MiningBondFundVaultProvider<
			Balance = Self::Balance,
			AccountId = Self::AccountId,
		>;

		/// The maximum number of contributors to a bond fund
		#[pallet::constant]
		type MaxBondFundContributors: Get<u32>;
		/// The minimum argons per fund contributor
		#[pallet::constant]
		type MinimumArgonsPerContributor: Get<Self::Balance>;

		/// A pallet id that is used to hold the bid pool
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Bid Pool burn percent
		#[pallet::constant]
		type BidPoolBurnPercent: Get<Percent>;

		/// The number of vaults that can participate in the bid pools. This is a substrate limit.
		#[pallet::constant]
		type MaxBidPoolVaultParticipants: Get<u32>;

		type NextCohortId: Get<CohortId>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		ContributedToBondFund,
	}

	/// The currently earning contributors for the current epoch's bond funds. Sorted by highest
	/// bids first
	#[pallet::storage]
	pub(super) type MiningBondFundsByCohort<T: Config> = StorageMap<
		_,
		Twox64Concat,
		CohortId,
		BoundedBTreeMap<VaultId, MiningBondFund<T>, T::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	/// The entrants in the mining bond pool that will be paid out for the active bid pool. They
	/// apply to the next closed mining slot cohort bid pool. Sorted with biggest share last.
	#[pallet::storage]
	pub(super) type OpenVaultBidPoolCapital<T: Config> = StorageValue<
		_,
		BoundedVec<VaultBidPoolCapital<T>, T::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	/// The bid pool capital for the next bid pool.
	#[pallet::storage]
	pub(super) type NextVaultBidPoolCapital<T: Config> = StorageValue<
		_,
		BoundedVec<VaultBidPoolCapital<T>, T::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An error occurred distributing a bid pool
		CouldNotDistributeBidPool {
			account_id: T::AccountId,
			cohort_id: CohortId,
			vault_id: VaultId,
			amount: T::Balance,
			dispatch_error: DispatchError,
			is_for_vault: bool,
		},
		/// An error occurred burning from the bid pool
		CouldNotBurnBidPool {
			cohort_id: CohortId,
			amount: T::Balance,
			dispatch_error: DispatchError,
		},
		/// Funds from the active bid pool have been distributed
		BidPoolDistributed {
			cohort_id: CohortId,
			bid_pool_distributed: T::Balance,
			bid_pool_burned: T::Balance,
			bid_pool_shares: u32,
		},
		/// The next bid pool has been locked in
		NextBidPoolCapitalLocked {
			cohort_id: CohortId,
			total_activated_capital: T::Balance,
			participating_vaults: u32,
		},
		/// An error occurred releasing a contributor hold
		ErrorRefundingBondFundCapital {
			cohort_id: CohortId,
			vault_id: VaultId,
			amount: T::Balance,
			account_id: T::AccountId,
			dispatch_error: DispatchError,
		},
		/// Some mining bond capital was refunded due to less activated vault funds than bond
		/// capital
		RefundedBondFundCapital {
			cohort_id: CohortId,
			vault_id: VaultId,
			amount: T::Balance,
			account_id: T::AccountId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The contributed amount would not make this account a contributor
		ContributionTooLow,
		/// The given vault is not accepting mining bonds
		VaultNotAcceptingMiningBonds,
		/// Below the minimum amount of argons per contributor
		BelowMinimum,
		/// This account is not an active mining fund contributor
		NotAFundContributor,
		/// An internal error occurred (like an overflow)
		InternalError,
		/// Unable to update the vault fund
		CouldNotFindBondFund,
		/// Max contributors for a fund exceeded
		MaxContributorsExceeded,
		/// The added amount would exceed the activated securitization
		ActivatedSecuritizationExceeded,
		/// Max Vaults exceeded
		MaxVaultsExceeded,
		/// This fund has already been renewed
		AlreadyRenewed,
	}

	impl<T: Config> OnNewSlot<T::AccountId> for Pallet<T> {
		type Key = BlockSealAuthorityId;
		fn on_new_cohort(cohort_id: CohortId) {
			Self::release_rolling_contributors(cohort_id);
			Self::distribute_bid_pool(cohort_id);
			let open_cohort_id = cohort_id + 1;
			Self::lock_next_bid_pool_capital(open_cohort_id);

			Self::rollover_contributors(cohort_id);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add capital to the next cohort's mining bids (aka, tomorrow after noon EST). The amount
		/// raised per vault cannot exceed the allocated securitization for the vault.
		///
		/// The funds and profits will be automatically rolled over to the next fund up to the max
		/// securitization activated.
		///
		/// - `origin`: The account that is joining the fund
		/// - `vault_id`: The vault id that the account would like to join a fund for
		/// - `amount`: The amount of argons to contribute to the fund. If you change this amount,
		///   it will just add the incremental amount
		#[pallet::call_index(0)]
		#[pallet::weight(0)] //T::WeightInfo::hold())]
		pub fn add_capital(
			origin: OriginFor<T>,
			vault_id: VaultId,
			amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut hold_amount = amount;
			ensure!(
				T::MiningBondFundVaultProvider::is_vault_accepting_mining_bonds(vault_id),
				Error::<T>::VaultNotAcceptingMiningBonds
			);
			ensure!(amount >= T::MinimumArgonsPerContributor::get(), Error::<T>::BelowMinimum);

			let next_cohort = T::NextCohortId::get() + 1;
			MiningBondFundsByCohort::<T>::try_mutate(next_cohort, |a| -> DispatchResult {
				let activated_securitization = Self::get_vault_activated_funds_per_slot(vault_id);

				NextVaultBidPoolCapital::<T>::try_mutate(|list| {
					if let Some(entry) = list.iter_mut().find(|x| x.vault_id == vault_id) {
						ensure!(
							entry.activated_capital.saturating_add(amount) <=
								activated_securitization,
							Error::<T>::ActivatedSecuritizationExceeded
						);
						entry.activated_capital.saturating_accrue(amount);
					} else {
						ensure!(
							amount <= activated_securitization,
							Error::<T>::ActivatedSecuritizationExceeded
						);
						let entry = VaultBidPoolCapital {
							vault_id,
							activated_capital: amount,
							cohort_id: next_cohort,
						};
						list.try_push(entry).map_err(|_| Error::<T>::MaxVaultsExceeded)?;
					}
					list.sort_by(|a, b| b.activated_capital.cmp(&a.activated_capital));
					Ok::<_, Error<T>>(())
				})?;

				let mut mining_fund = a.remove(&vault_id);
				if mining_fund.is_none() {
					let mut fund = MiningBondFund::default();
					let (_, vault_take) =
						T::MiningBondFundVaultProvider::get_vault_payment_info(vault_id)
							.ok_or(Error::<T>::CouldNotFindBondFund)?;
					fund.vault_percent_take = vault_take;
					mining_fund = Some(fund);
				}
				let mut mining_fund = mining_fund.ok_or(Error::<T>::CouldNotFindBondFund)?;

				let existing_pos =
					mining_fund.contributor_balances.iter().position(|(a, _)| *a == who);

				if let Some(pos) = existing_pos {
					let (_, balance) = mining_fund.contributor_balances.remove(pos);
					hold_amount = amount.saturating_sub(balance);
				}

				let insert_pos = mining_fund
					.contributor_balances
					.binary_search_by(|a| a.1.cmp(&amount).reverse())
					.unwrap_or_else(|x| x);

				if mining_fund.contributor_balances.is_full() {
					ensure!(
						insert_pos < mining_fund.contributor_balances.len(),
						Error::<T>::ContributionTooLow
					);
					if let Some((lowest_account, balance)) = mining_fund.contributor_balances.pop()
					{
						Self::release_hold(&lowest_account, balance)?;
					}
				}

				mining_fund
					.contributor_balances
					.try_insert(insert_pos, (who.clone(), amount))
					.map_err(|_| Error::<T>::MaxContributorsExceeded)?;
				a.try_insert(vault_id, mining_fund).map_err(|_| Error::<T>::MaxVaultsExceeded)?;

				Self::create_hold(&who, hold_amount)?;
				Ok(())
			})?;

			Ok(())
		}

		/// Allows a user to remove their contribution from the fund after the hold is released
		/// (once cohort slot period is complete).
		#[pallet::call_index(2)]
		#[pallet::weight(0)] //T::WeightInfo::hold())]
		pub fn end_renewal(
			origin: OriginFor<T>,
			vault_id: VaultId,
			cohort_id: CohortId,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;
			MiningBondFundsByCohort::<T>::try_mutate(cohort_id, |a| -> DispatchResult {
				let fund = a.get_mut(&vault_id).ok_or(Error::<T>::CouldNotFindBondFund)?;

				ensure!(
					fund.contributor_balances.iter().any(|(a, _)| *a == account),
					Error::<T>::NotAFundContributor
				);
				ensure!(!fund.is_rolled_over, Error::<T>::AlreadyRenewed);

				fund.do_not_renew
					.try_push(account.clone())
					.map_err(|_| Error::<T>::MaxContributorsExceeded)?;

				Ok(())
			})
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn create_hold(account_id: &T::AccountId, amount: T::Balance) -> DispatchResult {
			if amount == Zero::zero() {
				return Ok(());
			}
			let hold_reason = HoldReason::ContributedToBondFund;
			if T::Currency::balance_on_hold(&hold_reason.into(), account_id).is_zero() {
				frame_system::Pallet::<T>::inc_providers(account_id);
			}

			T::Currency::hold(&hold_reason.into(), account_id, amount)?;
			Ok(())
		}

		fn release_hold(who: &T::AccountId, amount: T::Balance) -> DispatchResult {
			if amount == T::Balance::zero() {
				return Ok(());
			}
			let reason = HoldReason::ContributedToBondFund;

			T::Currency::release(&reason.into(), who, amount, Precision::Exact)?;

			if T::Currency::balance_on_hold(&reason.into(), who) == 0u128.into() {
				frame_system::Pallet::<T>::dec_providers(who)?;
			}
			Ok(())
		}

		pub(crate) fn distribute_bid_pool(cohort_id: CohortId) {
			let bid_pool_account = Self::get_bid_pool_account();
			let mut total_bid_pool_amount = T::Currency::balance(&bid_pool_account);

			let burn_amount = T::BidPoolBurnPercent::get().mul_ceil(total_bid_pool_amount);
			if let Err(e) = T::Currency::burn_from(
				&bid_pool_account,
				burn_amount,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			) {
				Self::deposit_event(Event::<T>::CouldNotBurnBidPool {
					cohort_id,
					amount: burn_amount,
					dispatch_error: e,
				});
			}

			total_bid_pool_amount.saturating_reduce(burn_amount);
			let mut remaining_bid_pool = total_bid_pool_amount;
			let bid_pool_capital = OpenVaultBidPoolCapital::<T>::take();
			let bid_pool_entrants = bid_pool_capital.len();
			let total_bid_pool_capital = bid_pool_capital
				.iter()
				.fold(T::Balance::zero(), |acc, x| acc.saturating_add(x.activated_capital));

			let mut cohort_mining_bonds = MiningBondFundsByCohort::<T>::get(cohort_id);

			for (i, entrant) in bid_pool_capital.iter().rev().enumerate() {
				let Some(vault_fund) = cohort_mining_bonds.get_mut(&entrant.vault_id) else {
					continue;
				};
				let Some((vault_account_id, _)) =
					T::MiningBondFundVaultProvider::get_vault_payment_info(entrant.vault_id)
				else {
					continue;
				};
				let mut bond_fund_share =
					Perbill::from_rational(entrant.activated_capital, total_bid_pool_capital)
						.mul_floor(total_bid_pool_amount);
				remaining_bid_pool.saturating_reduce(bond_fund_share);
				if i == bid_pool_capital.len() - 1 {
					bond_fund_share.saturating_accrue(remaining_bid_pool);
					remaining_bid_pool = T::Balance::zero();
				}
				vault_fund.distributed_earnings = Some(bond_fund_share);

				let vault_share = vault_fund.vault_percent_take.mul_floor(bond_fund_share);

				// pay vault
				{
					if let Err(e) = T::Currency::transfer(
						&bid_pool_account,
						&vault_account_id,
						vault_share,
						Preservation::Expendable,
					) {
						Self::deposit_event(Event::<T>::CouldNotDistributeBidPool {
							account_id: vault_account_id.clone(),
							cohort_id,
							vault_id: entrant.vault_id,
							amount: vault_share,
							dispatch_error: e,
							is_for_vault: true,
						});
					}
				}

				let contributor_amount = bond_fund_share.saturating_sub(vault_share);
				let contributor_funds = entrant.activated_capital;
				let mut total_distributed = T::Balance::zero();
				let mut distributions = BTreeMap::<T::AccountId, T::Balance>::new();

				for (account, contrib) in vault_fund.contributor_balances.iter_mut() {
					let prorata = Permill::from_rational(*contrib, contributor_funds)
						.mul_floor(contributor_amount);
					contrib.saturating_accrue(prorata);
					total_distributed.saturating_accrue(prorata);
					distributions.entry(account.clone()).or_default().saturating_accrue(prorata);
				}
				// items are sorted by highest bid first, so the last one is the lowest
				if total_distributed < contributor_amount {
					let change = contributor_amount.saturating_sub(total_distributed);
					if let Some((account, amount)) = vault_fund.contributor_balances.get_mut(0) {
						amount.saturating_accrue(change);
						distributions.entry(account.clone()).or_default().saturating_accrue(change);
					}
				}
				for (account, amount) in distributions {
					if amount == T::Balance::zero() {
						continue;
					}
					if let Err(e) = T::Currency::transfer_and_hold(
						&HoldReason::ContributedToBondFund.into(),
						&bid_pool_account,
						&account,
						amount,
						Precision::Exact,
						Preservation::Expendable,
						Fortitude::Force,
					) {
						Self::deposit_event(Event::<T>::CouldNotDistributeBidPool {
							account_id: account.clone(),
							cohort_id,
							vault_id: entrant.vault_id,
							amount,
							dispatch_error: e,
							is_for_vault: false,
						});
					}
				}
			}
			MiningBondFundsByCohort::<T>::insert(cohort_id, cohort_mining_bonds);

			Self::deposit_event(Event::<T>::BidPoolDistributed {
				cohort_id,
				bid_pool_distributed: total_bid_pool_amount - remaining_bid_pool,
				bid_pool_burned: burn_amount,
				bid_pool_shares: bid_pool_entrants as u32,
			});
		}

		pub(crate) fn lock_next_bid_pool_capital(cohort_id: CohortId) {
			let mut next_bid_pool_capital = NextVaultBidPoolCapital::<T>::take();
			let mut cohort_funds = MiningBondFundsByCohort::<T>::get(cohort_id);

			for bid_pool_capital in next_bid_pool_capital.iter_mut() {
				let vault_id = bid_pool_capital.vault_id;
				let activated_securitization = Self::get_vault_activated_funds_per_slot(vault_id);
				// if we raised too much capital, we need to return excess now
				if bid_pool_capital.activated_capital > activated_securitization {
					let mut total_to_refund =
						bid_pool_capital.activated_capital.saturating_sub(activated_securitization);
					bid_pool_capital.activated_capital = activated_securitization;

					let Some(vault_fund) = cohort_funds.get_mut(&vault_id) else {
						continue;
					};

					while total_to_refund > T::Balance::zero() {
						// take smallest (last entry)
						let Some((account, amount)) = vault_fund.contributor_balances.pop() else {
							continue;
						};
						let to_refund = total_to_refund.min(amount);
						Self::refund_fund_capital(cohort_id, vault_id, &account, to_refund);
						total_to_refund.saturating_reduce(to_refund);
						let final_amount = amount.saturating_sub(to_refund);
						if final_amount > T::Balance::zero() {
							vault_fund.contributor_balances.try_push((account, final_amount)).ok();
						}
					}
				}
			}
			next_bid_pool_capital.sort_by(|a, b| b.activated_capital.cmp(&a.activated_capital));

			let participating_vaults = next_bid_pool_capital.len() as u32;
			let total_activated_capital = next_bid_pool_capital
				.iter()
				.fold(T::Balance::zero(), |acc, x| acc.saturating_add(x.activated_capital));
			Self::deposit_event(Event::<T>::NextBidPoolCapitalLocked {
				cohort_id,
				total_activated_capital,
				participating_vaults,
			});
			OpenVaultBidPoolCapital::<T>::set(next_bid_pool_capital);
			MiningBondFundsByCohort::<T>::insert(cohort_id, cohort_funds);
		}

		/// Release the held fund for all vaults and move the active fund contributors to the held
		pub(crate) fn rollover_contributors(activated_cohort_id: CohortId) {
			let next_cohort_id = activated_cohort_id + 2;
			if next_cohort_id < 10 {
				return;
			}
			MiningBondFundsByCohort::<T>::mutate(next_cohort_id, |next| {
				MiningBondFundsByCohort::<T>::mutate(next_cohort_id - 10, |tree| {
					let mut entrants = BoundedVec::new();
					for (vault_id, fund) in tree {
						let vault_id = *vault_id;
						let mut total = T::Balance::zero();
						let vault_take =
							T::MiningBondFundVaultProvider::get_vault_payment_info(vault_id)
								.map(|(_, t)| t)
								.unwrap_or(Permill::from_percent(100));

						let mut participants = vec![];
						for (account, amount) in &fund.contributor_balances {
							if fund.do_not_renew.contains(account) {
								continue;
							}
							if *amount < T::MinimumArgonsPerContributor::get() {
								fund.do_not_renew.try_push(account.clone()).ok();
								continue;
							}
							if vault_take > fund.vault_percent_take {
								fund.do_not_renew.try_push(account.clone()).ok();
								continue;
							}
							participants.push((account.clone(), *amount));
							total.saturating_accrue(*amount);
						}
						fund.is_rolled_over = true;
						if !participants.is_empty() {
							let new_fund = MiningBondFund {
								contributor_balances: BoundedVec::truncate_from(participants),
								vault_percent_take: vault_take,
								..Default::default()
							};
							next.try_insert(vault_id, new_fund).ok();
						}
						if total > T::Balance::zero() {
							entrants
								.try_push(VaultBidPoolCapital {
									vault_id,
									activated_capital: total,
									cohort_id: next_cohort_id,
								})
								.ok();
						}
					}
					NextVaultBidPoolCapital::<T>::set(entrants);
				});
			});
		}

		pub(crate) fn release_rolling_contributors(activated_cohort_id: CohortId) {
			if activated_cohort_id < 10 {
				return;
			}
			let release_cohort_id = activated_cohort_id - 10;
			for (vault_id, fund) in MiningBondFundsByCohort::<T>::take(release_cohort_id) {
				for (account, amount) in fund.contributor_balances {
					if fund.do_not_renew.contains(&account) {
						Self::refund_fund_capital(release_cohort_id, vault_id, &account, amount);
					}
				}
			}
		}

		fn refund_fund_capital(
			cohort_id: CohortId,
			vault_id: VaultId,
			account: &T::AccountId,
			refund_amount: T::Balance,
		) {
			if let Err(e) = Self::release_hold(account, refund_amount) {
				warn!(
					vault_id,
					?account,
					?refund_amount,
					"Error releasing vault hold for fund. {:?}",
					e
				);
				Self::deposit_event(Event::<T>::ErrorRefundingBondFundCapital {
					cohort_id,
					vault_id,
					amount: refund_amount,
					account_id: account.clone(),
					dispatch_error: e,
				})
			} else {
				Self::deposit_event(Event::<T>::RefundedBondFundCapital {
					cohort_id,
					vault_id,
					amount: refund_amount,
					account_id: account.clone(),
				});
			}
		}

		fn get_vault_activated_funds_per_slot(vault_id: VaultId) -> T::Balance {
			let activated_securitization =
				T::MiningBondFundVaultProvider::get_activated_securitization(vault_id);
			activated_securitization / 10u128.into()
		}
	}

	impl<T: Config> MiningBidPoolProvider for Pallet<T> {
		type Balance = T::Balance;
		type AccountId = T::AccountId;

		fn get_bid_pool_account() -> Self::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct VaultBidPoolCapital<T: Config> {
		#[codec(compact)]
		pub vault_id: VaultId,
		#[codec(compact)]
		pub activated_capital: T::Balance,
		#[codec(compact)]
		pub cohort_id: CohortId,
	}

	#[derive(
		Encode, Decode, Clone, PartialEqNoBound, Eq, RuntimeDebugNoBound, TypeInfo, DefaultNoBound,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct MiningBondFund<T: Config> {
		/// The amount of argons per account. Sorted with largest first. After bid pool is
		/// distributed, earnings are added to this balance
		pub contributor_balances:
			BoundedVec<(T::AccountId, T::Balance), T::MaxBondFundContributors>,

		/// Accounts not wishing to be re-upped
		pub do_not_renew: BoundedVec<T::AccountId, T::MaxBondFundContributors>,

		/// Did this fund already roll over?
		pub is_rolled_over: bool,

		/// The amount of argons that have been distributed to the fund (vault + contributors)
		pub distributed_earnings: Option<T::Balance>,

		/// The vault percent take
		#[codec(compact)]
		pub vault_percent_take: Permill,
	}
}
