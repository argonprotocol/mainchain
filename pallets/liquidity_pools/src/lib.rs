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
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::collections::BTreeMap;
	use argon_primitives::{
		OnNewSlot,
		vault::{
			LiquidityPoolVaultProvider, MiningBidPoolProvider, VaultLiquidityPoolFrameEarnings,
		},
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
	pub type VaultPoolsByFrame<T: Config> = StorageMap<
		_,
		Twox64Concat,
		FrameId,
		BoundedBTreeMap<VaultId, LiquidityPool<T>, T::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	/// The liquidity pool for the current frame. This correlates with the bids coming in for the
	/// current frame. Sorted with the biggest share last. (current frame + 1)
	#[pallet::storage]
	pub type CapitalActive<T: Config> = StorageValue<
		_,
		BoundedVec<LiquidityPoolCapital<T>, T::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	/// The liquidity pool still raising capital. (current frame + 2)
	#[pallet::storage]
	pub type CapitalRaising<T: Config> = StorageValue<
		_,
		BoundedVec<LiquidityPoolCapital<T>, T::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	/// Any vaults that have been pre-registered for bonding argons. This is used by the vault
	/// operator to allocate argons to be bonded once bitcoins are securitized in their vault.
	#[pallet::storage]
	pub type PrebondedByVaultId<T: Config> =
		StorageMap<_, Twox64Concat, VaultId, PrebondedArgons<T>, OptionQuery>;

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
		/// The vault operator pre-registered to bond argons for a vault
		VaultOperatorPrebond {
			/// The vault id that the operator is pre-bonding for
			vault_id: VaultId,
			account_id: T::AccountId,
			amount_per_frame: T::Balance,
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
		/// Vault operator only
		NotAVaultOperator,
		/// The prebond amount cannot be reduced and this takes it below the previous allocation
		MaxAmountBelowMinimum,
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
		#[pallet::weight(T::WeightInfo::bond_argons())]
		pub fn bond_argons(
			origin: OriginFor<T>,
			vault_id: VaultId,
			amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
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
					mining_fund = Some(LiquidityPool::new(vault_id));
				}
				let mut mining_fund = mining_fund.ok_or(Error::<T>::CouldNotFindLiquidityPool)?;

				let InsertContributorResponse { hold_amount, needs_refund } =
					mining_fund.try_insert_contributor(who.clone(), amount)?;
				if let Some((lowest_account, balance)) = needs_refund {
					Self::release_hold(&lowest_account, balance)?;
				}

				a.try_insert(vault_id, mining_fund).map_err(|_| Error::<T>::MaxVaultsExceeded)?;

				Self::create_hold(&who, hold_amount)?;
				Ok(())
			})?;

			Ok(())
		}

		/// Allows a user to remove their bonded argons from the fund after the hold is released
		/// (once epoch starting at bonded frame is complete).
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::unbond_argons())]
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

		/// Set the prebonded argons for a vault. This is used by the vault operator to
		/// pre-register funding for each frame. The total allocation will be capped per frame using
		/// the `max_amount_per_frame` parameter.
		///
		/// NOTE: calling this a second time will ensure your max_amount_per_frame is updated.
		/// However, it will not reduce your allocation
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::vault_operator_prebond())]
		pub fn vault_operator_prebond(
			origin: OriginFor<T>,
			vault_id: VaultId,
			max_amount_per_frame: T::Balance,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;
			ensure!(
				T::LiquidityPoolVaultProvider::is_vault_open(vault_id),
				Error::<T>::VaultNotAcceptingMiningBonds
			);
			let operator = T::LiquidityPoolVaultProvider::get_vault_operator(vault_id)
				.ok_or(Error::<T>::CouldNotFindLiquidityPool)?;
			ensure!(account == operator, Error::<T>::NotAVaultOperator);
			let amount_to_distribute: T::Balance = max_amount_per_frame * 10u128.into();
			let mut amount_already_distributed: T::Balance = T::Balance::zero();

			if let Some(prebond) = PrebondedByVaultId::<T>::get(vault_id) {
				amount_already_distributed = prebond.amount_unbonded;
			}

			// We can safely go through the existing liquidity pools for the last 10 frames to see
			// what has already been allocated. The vault operator won't be automatically rolled
			// over, so we can just subtract off anything allocated
			let raising_frame_id = T::GetCurrentFrameId::get() + 2;
			for frame_id in raising_frame_id.saturating_sub(10)..=raising_frame_id {
				let frame_pools = VaultPoolsByFrame::<T>::get(frame_id);
				let Some(vault_pool) = frame_pools.get(&vault_id) else {
					continue;
				};
				for (account_id, amount) in &vault_pool.contributor_balances {
					if *account_id == operator {
						amount_already_distributed.saturating_accrue(*amount);
					}
				}
			}

			if amount_to_distribute <= amount_already_distributed {
				return Err(Error::<T>::MaxAmountBelowMinimum.into());
			}

			let amount_needed = amount_to_distribute.saturating_sub(amount_already_distributed);
			Self::create_hold(&account, amount_needed)?;
			PrebondedByVaultId::<T>::mutate(vault_id, |e| {
				if let Some(prebond) = e {
					prebond.amount_unbonded.saturating_accrue(amount_needed);
					prebond.max_amount_per_frame = max_amount_per_frame;
				} else {
					*e = Some(PrebondedArgons::new(
						vault_id,
						account.clone(),
						amount_needed,
						max_amount_per_frame,
					));
				}
				Ok::<(), Error<T>>(())
			})?;
			Self::deposit_event(Event::<T>::VaultOperatorPrebond {
				vault_id,
				account_id: account.clone(),
				amount_per_frame: max_amount_per_frame,
			});
			Ok(())
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

		/// Once the frame is complete, this fn distributes the bid pool to each vault based on
		/// their prorata funds raised. Then within each vault, profits are distributed to any
		/// contributors besides the vault operator, who must collect theirs.
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
				let mut earnings =
					Perbill::from_rational(entrant.activated_capital, total_bid_pool_capital)
						.mul_floor(total_bid_pool_amount);
				remaining_bid_pool.saturating_reduce(earnings);
				if i == bid_pool_capital.len() - 1 {
					earnings.saturating_accrue(remaining_bid_pool);
					remaining_bid_pool = T::Balance::zero();
				}
				vault_fund.distributed_profits = Some(earnings);

				let mut vault_share = Permill::one()
					.saturating_sub(vault_fund.vault_sharing_percent)
					.mul_floor(earnings);

				let contributor_amount = earnings.saturating_sub(vault_share);
				let contributor_funds = entrant.activated_capital;
				let mut contributor_distribution_remaining = contributor_amount;
				let mut distributions = BTreeMap::<T::AccountId, T::Balance>::new();

				let mut vault_contributed_capital = T::Balance::zero();
				for (account, contrib) in vault_fund.contributor_balances.iter_mut() {
					if *account == vault_account_id {
						vault_contributed_capital = *contrib;
					}
					let prorata = Permill::from_rational(*contrib, contributor_funds)
						.mul_floor(contributor_amount);
					contrib.saturating_accrue(prorata);
					contributor_distribution_remaining.saturating_reduce(prorata);
					distributions.entry(account.clone()).or_default().saturating_accrue(prorata);
				}
				// add remaining back to vault share
				vault_share.saturating_accrue(contributor_distribution_remaining);

				// give change to vault
				let mut earnings_for_vault = vault_share;
				for (account, amount) in distributions {
					// the vault must collect earnings
					if account == vault_account_id {
						earnings_for_vault.saturating_accrue(amount);
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
				T::LiquidityPoolVaultProvider::record_vault_frame_earnings(
					&bid_pool_account,
					VaultLiquidityPoolFrameEarnings {
						vault_id: entrant.vault_id,
						vault_operator_account_id: vault_account_id,
						frame_id,
						earnings_for_vault,
						earnings,
						capital_contributed: contributor_funds,
						capital_contributed_by_vault: vault_contributed_capital,
					},
				);
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
			for vault_id in PrebondedByVaultId::<T>::iter_keys() {
				if !next_bid_pool_capital.iter().any(|x| x.vault_id == vault_id) {
					let _ = next_bid_pool_capital.try_push(LiquidityPoolCapital {
						vault_id,
						activated_capital: T::Balance::zero(),
						frame_id,
					});
				}
			}

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
						// if we have some left, we need to re-add the contributor
						if final_amount > T::Balance::zero() {
							vault_fund.contributor_balances.try_push((account, final_amount)).ok();
						}
					}
				}

				if activated_securitization > bid_pool_capital.activated_capital {
					// check the prebonded funds to allocate
					let Some(mut prebond) = PrebondedByVaultId::<T>::get(vault_id) else {
						continue;
					};

					// we can't add this vault to the frame funds if we have too many participants
					if !frame_funds.contains_key(&vault_id) &&
						frame_funds.try_insert(vault_id, LiquidityPool::new(vault_id)).is_err()
					{
						continue;
					}

					let vault_fund = frame_funds
						.get_mut(&vault_id)
						.expect("we just inserted this entry above; qed");

					if !vault_fund.can_add_contributor(&prebond.account_id) {
						// we can't add a new contributor if the list is full
						continue;
					}

					let amount_available =
						activated_securitization.saturating_sub(bid_pool_capital.activated_capital);
					let already_allocated =
						vault_fund
							.contributor_balances
							.iter()
							.find_map(|(account, amount)| {
								if account == &prebond.account_id { Some(*amount) } else { None }
							})
							.unwrap_or_default();

					let to_bond =
						prebond.take_unbonded(frame_id, amount_available, already_allocated);

					if to_bond > T::Balance::zero() {
						bid_pool_capital.activated_capital.saturating_accrue(to_bond);
						vault_fund
							.try_insert_contributor(prebond.account_id.clone(), to_bond)
							.expect(
								"already checked if this was full, so shouldn't be possible to fail",
							);
						// if we have no more funds to bond, remove the entry
						if prebond.amount_unbonded == T::Balance::zero() {
							PrebondedByVaultId::<T>::remove(vault_id);
						} else {
							PrebondedByVaultId::<T>::insert(vault_id, prebond);
						}
					}
				}
			}
			next_bid_pool_capital.retain(|a| a.activated_capital > T::Balance::zero());
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
							let mut new_fund = LiquidityPool::new(vault_id);
							new_fund.contributor_balances = BoundedVec::truncate_from(participants);
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

	#[derive(
		Encode, Decode, Clone, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
	)]
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
		Encode,
		Decode,
		Clone,
		PartialEqNoBound,
		Eq,
		RuntimeDebugNoBound,
		TypeInfo,
		DefaultNoBound,
		MaxEncodedLen,
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

	#[derive(EqNoBound, PartialEqNoBound, DebugNoBound)]
	pub struct InsertContributorResponse<T: Config> {
		pub(crate) hold_amount: T::Balance,
		pub(crate) needs_refund: Option<(T::AccountId, T::Balance)>,
	}

	impl<T: Config> LiquidityPool<T> {
		pub fn new(vault_id: VaultId) -> Self {
			let sharing = T::LiquidityPoolVaultProvider::get_vault_profit_sharing_percent(vault_id)
				.unwrap_or_default();
			Self { vault_sharing_percent: sharing, ..Default::default() }
		}

		pub fn can_add_contributor(&self, account_id: &T::AccountId) -> bool {
			self.contributor_balances.iter().any(|(a, _)| *a == *account_id) ||
				!self.contributor_balances.is_full()
		}

		pub fn try_insert_contributor(
			&mut self,
			account: T::AccountId,
			amount: T::Balance,
		) -> Result<InsertContributorResponse<T>, Error<T>> {
			let existing_pos = self.contributor_balances.iter().position(|(a, _)| *a == account);
			let mut hold_amount = amount;
			if let Some(pos) = existing_pos {
				let (_, balance) = self.contributor_balances.remove(pos);
				hold_amount = amount.saturating_sub(balance);
			}

			let insert_pos = self
				.contributor_balances
				.binary_search_by(|a| a.1.cmp(&amount).reverse())
				.unwrap_or_else(|x| x);

			let mut needs_refund = None;
			if self.contributor_balances.is_full() {
				ensure!(
					insert_pos < self.contributor_balances.len(),
					Error::<T>::ContributionTooLow
				);
				needs_refund = self.contributor_balances.pop();
			}

			self.contributor_balances
				.try_insert(insert_pos, (account, amount))
				.map_err(|_| Error::<T>::MaxContributorsExceeded)?;
			Ok(InsertContributorResponse { hold_amount, needs_refund })
		}
	}

	#[derive(PartialEq, Eq, Clone, Debug, TypeInfo, MaxEncodedLen, Encode, Decode)]
	#[scale_info(skip_type_params(T))]
	pub struct PrebondedArgons<T: Config> {
		/// The vault id that the argons are pre-bonded for
		#[codec(compact)]
		pub vault_id: VaultId,
		/// The account that is pre-bonding the argons
		pub account_id: T::AccountId,
		/// The amount of argons remaining to be bonded
		#[codec(compact)]
		pub amount_unbonded: T::Balance,
		/// The frame id that the pre-bonding started
		#[codec(compact)]
		pub starting_frame_id: FrameId,
		/// The amount bonded by offset since the starting frame (eg, frame - starting_frame % 10)
		#[deprecated(since = "1.3.6", note = "Use amounts allocated to liquidity pools instead")]
		pub bonded_by_start_offset: BoundedVec<T::Balance, ConstU32<10>>,
		/// The max amount of argons that can be bonded per frame offset
		#[codec(compact)]
		pub max_amount_per_frame: T::Balance,
	}

	impl<T: Config> PrebondedArgons<T> {
		pub fn new(
			vault_id: VaultId,
			account_id: T::AccountId,
			amount: T::Balance,
			max_amount_per_frame: T::Balance,
		) -> Self {
			#[allow(deprecated)]
			Self {
				vault_id,
				account_id,
				amount_unbonded: amount,
				starting_frame_id: T::GetCurrentFrameId::get(),
				bonded_by_start_offset: Default::default(),
				max_amount_per_frame,
			}
		}

		pub fn take_unbonded(
			&mut self,
			frame_id: FrameId,
			max_amount: T::Balance,
			already_allocated: T::Balance,
		) -> T::Balance {
			if frame_id < self.starting_frame_id {
				// We can't bond for a frame before the starting frame
				return T::Balance::zero();
			}
			let available_to_use = self.amount_unbonded.min(max_amount);
			let max_bondable_for_frame =
				self.max_amount_per_frame.saturating_sub(already_allocated);
			let to_bond = available_to_use.min(max_bondable_for_frame);
			self.amount_unbonded.saturating_reduce(to_bond);
			to_bond
		}
	}
}
