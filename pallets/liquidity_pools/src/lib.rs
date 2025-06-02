#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

use pallet_prelude::*;
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

/// This pallet allows users to bond argons to a Vault's Liquidity Pool. Liquidity pools serve as
/// instant liquidity for LockedBitcoins. "Bonding argons" to a liquidity pool means that the argons
/// will be inserted into the Liquidity Pool for a slot and will continue to roll-over to follow-on
/// funds until they are unbonded. Any profits are automatically bonded and combined with existing
/// funds.
///
///
/// TODO: ## Bitcoin Minting
/// The system will only mint argons for BitcoinLocks when the CPI is negative. Liquidity pools
/// allow Bitcoins to still be granted liquidity by adding the following funds to the pool:
/// 1. The mint rights garnered over the current day (slot period)
/// 2. 80% of the mining bid pool for the next slot cohort (20% is burned)
/// 3. The liquidity pool for each vault
///
/// Funds are then distributed in this order:
/// 1. Bitcoins locked in this slot
/// 2. Liquidity pool contributors based on pro-rata
///
/// Liquidity pool imbalances are added to the front of the "Mint" queue. Before minting occurs
/// for bitcoins in the list, any pending Liquidity Pools are paid out (oldest first). Within the
/// pool, contributors are paid out at a floored pro-rata. Excess is burned.
///
/// Bitcoins with remaining mint-able argons are added to the end of the mint-queue. Only bitcoins
/// locked the same day as a slot are eligible for instant-liquidity.
///
/// ## Liquidity Pool Allocation
/// Each slot's liquidity pool can bond argons up to 1/10th of a vault's `activated securitization`.
/// `Activated securitization` is 2x the amount of LockedBitcoins.
///
/// ## Profits from Bid Pool
/// Once each bid pool is closed, 20% of the pool is burned. Then the remaining funds are
/// distributed pro-rata to each vault's slot liquidity pool. Vault's automatically disperse funds
/// to contributors based on the vault's sharing percent, and each individual contributor's
/// pro-rata.
///
/// The limitations to bonding argons are:
/// - The maximum number of contributors to a fund (`MaxLiquidityPoolContributors`)
/// - The minimum amount of bonded argons per contributor (`MinimumArgonsPerContributor`)
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use alloc::collections::BTreeMap;
	use argon_primitives::{
		OnNewSlot,
		vault::{LiquidityPoolVaultProvider, MiningBidPoolProvider},
	};
	use sp_runtime::{BoundedBTreeMap, traits::AccountIdConversion};
	use tracing::warn;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config
	where
		<Self as Config>::Balance: Into<u128>,
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		/// The balance type
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

		/// The currency representing argons
		type Currency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>;

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		type LiquidityPoolVaultProvider: LiquidityPoolVaultProvider<Balance = Self::Balance, AccountId = Self::AccountId>;

		/// The maximum number of contributors to a bond fund
		#[pallet::constant]
		type MaxLiquidityPoolContributors: Get<u32>;
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

		type GetCurrentFrameId: Get<FrameId>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		ContributedToLiquidityPool,
	}

	/// The currently earning contributors for the current epoch's bond funds. Sorted by highest
	/// bids first
	#[pallet::storage]
	pub(super) type VaultPoolsByFrame<T: Config> = StorageMap<
		_,
		Twox64Concat,
		FrameId,
		BoundedBTreeMap<VaultId, LiquidityPool<T>, T::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	/// The liquidity pool for the current frame. This correlates with the bids coming in for the
	/// current frame. Sorted with the biggest share last. (current frame + 1)
	#[pallet::storage]
	pub(super) type CapitalActive<T: Config> = StorageValue<
		_,
		BoundedVec<LiquidityPoolCapital<T>, T::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	/// The liquidity pool still raising capital. (current frame + 2)
	#[pallet::storage]
	pub(super) type CapitalRaising<T: Config> = StorageValue<
		_,
		BoundedVec<LiquidityPoolCapital<T>, T::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An error occurred distributing a bid pool
		CouldNotDistributeBidPool {
			account_id: T::AccountId,
			frame_id: FrameId,
			vault_id: VaultId,
			amount: T::Balance,
			dispatch_error: DispatchError,
			is_for_vault: bool,
		},
		/// An error occurred burning from the bid pool
		CouldNotBurnBidPool { frame_id: FrameId, amount: T::Balance, dispatch_error: DispatchError },
		/// Funds from the active bid pool have been distributed
		BidPoolDistributed {
			frame_id: FrameId,
			bid_pool_distributed: T::Balance,
			bid_pool_burned: T::Balance,
			bid_pool_shares: u32,
		},
		/// The next bid pool has been locked in
		NextBidPoolCapitalLocked {
			frame_id: FrameId,
			total_activated_capital: T::Balance,
			participating_vaults: u32,
		},
		/// An error occurred releasing a contributor hold
		ErrorRefundingLiquidityPoolCapital {
			frame_id: FrameId,
			vault_id: VaultId,
			amount: T::Balance,
			account_id: T::AccountId,
			dispatch_error: DispatchError,
		},
		/// Some mining bond capital was refunded due to less activated vault funds than bond
		/// capital
		RefundedLiquidityPoolCapital {
			frame_id: FrameId,
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
		CouldNotFindLiquidityPool,
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
		fn on_frame_start(frame_id: FrameId) {
			Self::release_rolling_contributors(frame_id);
			Self::distribute_bid_pool(frame_id);
			Self::end_pool_capital_raise(frame_id + 1);

			Self::rollover_contributors(frame_id);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Bond argons to a Vault's next liquidity pool, tied to the next frame (aka,
		/// tomorrow noon EDT to day after tomorrow noon). The amount bonded to the pool cannot
		/// exceed 1/10th of the activated securitization for the vault.
		///
		/// The bonded argons and profits will be automatically rolled over to the next fund up to
		/// the max securitization activated.
		///
		/// - `origin`: The account that is joining the fund
		/// - `vault_id`: The vault id that the account would like to join a fund for
		/// - `amount`: The amount of argons to contribute to the fund. If you change this amount,
		///   it will just add the incremental amount
		#[pallet::call_index(0)]
		#[pallet::weight(0)] //T::WeightInfo::hold())]
		pub fn bond_argons(
			origin: OriginFor<T>,
			vault_id: VaultId,
			amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut hold_amount = amount;
			ensure!(
				T::LiquidityPoolVaultProvider::is_vault_open(vault_id),
				Error::<T>::VaultNotAcceptingMiningBonds
			);
			ensure!(amount >= T::MinimumArgonsPerContributor::get(), Error::<T>::BelowMinimum);

			// the "next next" frame is the one we are adding capital to
			let raising_frame_id = T::GetCurrentFrameId::get() + 2;
			VaultPoolsByFrame::<T>::try_mutate(raising_frame_id, |a| -> DispatchResult {
				let activated_securitization = Self::get_vault_activated_funds_per_slot(vault_id);

				CapitalRaising::<T>::try_mutate(|list| {
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
						let entry = LiquidityPoolCapital {
							vault_id,
							activated_capital: amount,
							frame_id: raising_frame_id,
						};
						list.try_push(entry).map_err(|_| Error::<T>::MaxVaultsExceeded)?;
					}
					list.sort_by(|a, b| b.activated_capital.cmp(&a.activated_capital));
					Ok::<_, Error<T>>(())
				})?;

				let mut mining_fund = a.remove(&vault_id);
				if mining_fund.is_none() {
					let mut fund = LiquidityPool::default();
					let sharing_percent =
						T::LiquidityPoolVaultProvider::get_vault_profit_sharing_percent(vault_id)
							.ok_or(Error::<T>::CouldNotFindLiquidityPool)?;
					fund.vault_sharing_percent = sharing_percent;
					mining_fund = Some(fund);
				}
				let mut mining_fund = mining_fund.ok_or(Error::<T>::CouldNotFindLiquidityPool)?;

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

		/// Allows a user to remove their bonded argons from the fund after the hold is released
		/// (once epoch starting at bonded frame is complete).
		#[pallet::call_index(2)]
		#[pallet::weight(0)] //T::WeightInfo::hold())]
		pub fn unbond_argons(
			origin: OriginFor<T>,
			vault_id: VaultId,
			frame_id: FrameId,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;
			VaultPoolsByFrame::<T>::try_mutate(frame_id, |a| -> DispatchResult {
				let fund = a.get_mut(&vault_id).ok_or(Error::<T>::CouldNotFindLiquidityPool)?;

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
			let hold_reason = HoldReason::ContributedToLiquidityPool;
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
			let reason = HoldReason::ContributedToLiquidityPool;

			T::Currency::release(&reason.into(), who, amount, Precision::Exact)?;

			if T::Currency::balance_on_hold(&reason.into(), who) == 0u128.into() {
				frame_system::Pallet::<T>::dec_providers(who)?;
			}
			Ok(())
		}

		pub(crate) fn distribute_bid_pool(frame_id: FrameId) {
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
					frame_id,
					amount: burn_amount,
					dispatch_error: e,
				});
			}

			total_bid_pool_amount.saturating_reduce(burn_amount);
			let mut remaining_bid_pool = total_bid_pool_amount;
			let bid_pool_capital = CapitalActive::<T>::take();
			let bid_pool_entrants = bid_pool_capital.len();
			let total_bid_pool_capital = bid_pool_capital
				.iter()
				.fold(T::Balance::zero(), |acc, x| acc.saturating_add(x.activated_capital));

			let mut liquidity_pools_by_vault = VaultPoolsByFrame::<T>::get(frame_id);

			for (i, entrant) in bid_pool_capital.iter().rev().enumerate() {
				let Some(vault_fund) = liquidity_pools_by_vault.get_mut(&entrant.vault_id) else {
					continue;
				};
				let Some(vault_account_id) =
					T::LiquidityPoolVaultProvider::get_vault_operator(entrant.vault_id)
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
				vault_fund.distributed_profits = Some(bond_fund_share);

				let vault_share = Permill::one()
					.saturating_sub(vault_fund.vault_sharing_percent)
					.mul_floor(bond_fund_share);

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
							frame_id,
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
						&HoldReason::ContributedToLiquidityPool.into(),
						&bid_pool_account,
						&account,
						amount,
						Precision::Exact,
						Preservation::Expendable,
						Fortitude::Force,
					) {
						Self::deposit_event(Event::<T>::CouldNotDistributeBidPool {
							account_id: account.clone(),
							frame_id,
							vault_id: entrant.vault_id,
							amount,
							dispatch_error: e,
							is_for_vault: false,
						});
					}
				}
			}
			VaultPoolsByFrame::<T>::insert(frame_id, liquidity_pools_by_vault);

			Self::deposit_event(Event::<T>::BidPoolDistributed {
				frame_id,
				bid_pool_distributed: total_bid_pool_amount - remaining_bid_pool,
				bid_pool_burned: burn_amount,
				bid_pool_shares: bid_pool_entrants as u32,
			});
		}

		pub(crate) fn end_pool_capital_raise(frame_id: FrameId) {
			let mut next_bid_pool_capital = CapitalRaising::<T>::take();
			let mut frame_funds = VaultPoolsByFrame::<T>::get(frame_id);

			for bid_pool_capital in next_bid_pool_capital.iter_mut() {
				let vault_id = bid_pool_capital.vault_id;
				let activated_securitization = Self::get_vault_activated_funds_per_slot(vault_id);
				// if we raised too much capital, we need to return excess now
				if bid_pool_capital.activated_capital > activated_securitization {
					let mut total_to_refund =
						bid_pool_capital.activated_capital.saturating_sub(activated_securitization);
					bid_pool_capital.activated_capital = activated_securitization;

					let Some(vault_fund) = frame_funds.get_mut(&vault_id) else {
						continue;
					};

					while total_to_refund > T::Balance::zero() {
						// take smallest (last entry)
						let Some((account, amount)) = vault_fund.contributor_balances.pop() else {
							continue;
						};
						let to_refund = total_to_refund.min(amount);
						Self::refund_fund_capital(frame_id, vault_id, &account, to_refund);
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
				frame_id,
				total_activated_capital,
				participating_vaults,
			});
			CapitalActive::<T>::set(next_bid_pool_capital);
			VaultPoolsByFrame::<T>::insert(frame_id, frame_funds);
		}

		/// Release the held fund for all vaults and move the active fund contributors to the held
		pub(crate) fn rollover_contributors(current_frame_id: FrameId) {
			let raising_frame_id = current_frame_id + 2;
			if raising_frame_id < 10 {
				return;
			}
			VaultPoolsByFrame::<T>::mutate(raising_frame_id, |next| {
				VaultPoolsByFrame::<T>::mutate(raising_frame_id - 10, |tree| {
					let mut entrants = BoundedVec::new();
					for (vault_id, fund) in tree {
						let vault_id = *vault_id;
						let mut total = T::Balance::zero();
						let vault_sharing =
							T::LiquidityPoolVaultProvider::get_vault_profit_sharing_percent(
								vault_id,
							)
							.unwrap_or_default();

						let mut participants = vec![];
						for (account, amount) in &fund.contributor_balances {
							if fund.do_not_renew.contains(account) {
								continue;
							}
							if *amount < T::MinimumArgonsPerContributor::get() {
								fund.do_not_renew.try_push(account.clone()).ok();
								continue;
							}
							if vault_sharing < fund.vault_sharing_percent {
								fund.do_not_renew.try_push(account.clone()).ok();
								continue;
							}
							participants.push((account.clone(), *amount));
							total.saturating_accrue(*amount);
						}
						fund.is_rolled_over = true;
						if !participants.is_empty() {
							let new_fund = LiquidityPool {
								contributor_balances: BoundedVec::truncate_from(participants),
								vault_sharing_percent: vault_sharing,
								..Default::default()
							};
							next.try_insert(vault_id, new_fund).ok();
						}
						if total > T::Balance::zero() {
							entrants
								.try_push(LiquidityPoolCapital {
									vault_id,
									activated_capital: total,
									frame_id: raising_frame_id,
								})
								.ok();
						}
					}
					CapitalRaising::<T>::set(entrants);
				});
			});
		}

		pub(crate) fn release_rolling_contributors(current_frame_id: FrameId) {
			if current_frame_id < 10 {
				return;
			}
			let release_frame_id = current_frame_id - 10;
			for (vault_id, fund) in VaultPoolsByFrame::<T>::take(release_frame_id) {
				for (account, amount) in fund.contributor_balances {
					if fund.do_not_renew.contains(&account) {
						Self::refund_fund_capital(release_frame_id, vault_id, &account, amount);
					}
				}
			}
		}

		fn refund_fund_capital(
			frame_id: FrameId,
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
				Self::deposit_event(Event::<T>::ErrorRefundingLiquidityPoolCapital {
					frame_id,
					vault_id,
					amount: refund_amount,
					account_id: account.clone(),
					dispatch_error: e,
				})
			} else {
				Self::deposit_event(Event::<T>::RefundedLiquidityPoolCapital {
					frame_id,
					vault_id,
					amount: refund_amount,
					account_id: account.clone(),
				});
			}
		}

		fn get_vault_activated_funds_per_slot(vault_id: VaultId) -> T::Balance {
			let activated_securitization =
				T::LiquidityPoolVaultProvider::get_activated_securitization(vault_id);
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
	pub struct LiquidityPoolCapital<T: Config> {
		#[codec(compact)]
		pub vault_id: VaultId,
		#[codec(compact)]
		pub activated_capital: T::Balance,
		/// The frame id this liquidity pool is for
		#[codec(compact)]
		pub frame_id: FrameId,
	}

	#[derive(
		Encode, Decode, Clone, PartialEqNoBound, Eq, RuntimeDebugNoBound, TypeInfo, DefaultNoBound,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct LiquidityPool<T: Config> {
		/// The amount of argons per account. Sorted with largest first. After bid pool is
		/// distributed, profits are added to this balance
		pub contributor_balances:
			BoundedVec<(T::AccountId, T::Balance), T::MaxLiquidityPoolContributors>,

		/// Accounts not wishing to be re-upped
		pub do_not_renew: BoundedVec<T::AccountId, T::MaxLiquidityPoolContributors>,

		/// Did this fund already roll over?
		pub is_rolled_over: bool,

		/// The amount of argons that have been distributed to the fund (vault + contributors)
		pub distributed_profits: Option<T::Balance>,

		/// The vault percent of profits shared
		#[codec(compact)]
		pub vault_sharing_percent: Permill,
	}
}
