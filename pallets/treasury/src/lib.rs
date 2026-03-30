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

/// This pallet allows users to bond argons to a Vault's Treasury Pool. Treasury pools serve as
/// instant liquidity for LockedBitcoins. "Bonding argons" to a treasury pool means that the argons
/// will be inserted into the Treasury Pool for a slot and will continue to roll-over to follow-on
/// funds until they are unbonded. Any profits are automatically bonded and combined with existing
/// funds.
///
///
/// TODO: ## Bitcoin Minting
/// The system will only mint argons for BitcoinLocks when the CPI is negative. Treasury pools
/// allow Bitcoins to still be granted liquidity by adding the following funds to the pool:
/// 1. The mint rights garnered over the current day (slot period)
/// 2. 80% of the mining bid pool for the next slot cohort (20% reserved for treasury reserves)
/// 3. The treasury pool for each vault
///
/// Funds are then distributed in this order:
/// 1. Bitcoins locked in this slot
/// 2. Treasury pool contributors based on pro-rata
///
/// Treasury pool imbalances are added to the front of the "Mint" queue. Before minting occurs
/// for bitcoins in the list, any pending Treasury Pools are paid out (oldest first). Within the
/// pool, contributors are paid out at a floored pro-rata. Excess remains in the bid pool.
///
/// Bitcoins with remaining mint-able argons are added to the end of the mint-queue. Only bitcoins
/// locked the same day as a slot are eligible for instant-liquidity.
///
/// ## Treasury Pool Allocation
/// Each slot's treasury pool can bond argons up to the full argon value of a vault's securitized
/// satoshis (`sats * securitization ratio`).
///
/// ## Profits from Bid Pool
/// Once each bid pool is closed, 20% of the pool is reserved for treasury reserves. (Operational
/// rewards are one use of these reserves.) The remaining funds are
/// distributed pro-rata to each vault's slot treasury pool. Vault's automatically disperse funds
/// to contributors based on the vault's sharing percent, and each individual contributor's
/// pro-rata.
///
/// The limitations to bonding argons are:
/// - The maximum number of contributors in an active vault pool (`MaxTreasuryContributors`)
/// - The minimum amount of bonded argons per contributor (`MinimumArgonsPerContributor`)
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::{collections::BTreeMap, vec::Vec};
	use argon_primitives::{
		BlockSealAuthorityId, OnNewSlot, OperationalRewardPayout, TreasuryPoolProvider,
		providers::{OperationalRewardsProviderWeightInfo, PriceProvider},
		vault::{MiningBidPoolProvider, TreasuryVaultProvider, VaultTreasuryFrameEarnings},
	};
	use pallet_prelude::argon_primitives::{
		MiningFrameTransitionProvider, OperationalAccountsHook, OperationalRewardsPayer,
		OperationalRewardsProvider,
	};
	use sp_runtime::{BoundedBTreeMap, traits::AccountIdConversion};
	use tracing::info;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);
	const TREASURY_RESERVES_SUB_ACCOUNT: [u8; 16] = *b"treasury-reserve";
	type OperationalRewardsProviderWeights<T> =
		<<T as Config>::OperationalRewardsProvider as OperationalRewardsProvider<
			<T as frame_system::Config>::AccountId,
			<T as Config>::Balance,
		>>::Weights;

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

		type TreasuryVaultProvider: TreasuryVaultProvider<Balance = Self::Balance, AccountId = Self::AccountId>;

		type PriceProvider: PriceProvider<Self::Balance>;

		/// The maximum number of contributors in a vault's treasury pool
		#[pallet::constant]
		type MaxTreasuryContributors: Get<u32>;
		/// The maximum number of tracked funded contributors kept per vault, including standby
		/// entries beyond the active pool size.
		#[pallet::constant]
		type MaxTrackedTreasuryFunders: Get<u32>;
		/// The minimum argons per fund contributor
		#[pallet::constant]
		type MinimumArgonsPerContributor: Get<Self::Balance>;

		/// A pallet id that is used to hold the bid pool
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Percent of the bid pool reserved for treasury reserves.
		#[pallet::constant]
		type PercentForTreasuryReserves: Get<Percent>;

		/// The number of vaults that can participate in each bond. This is a substrate limit.
		#[pallet::constant]
		type MaxVaultsPerPool: Get<u32>;

		/// The maximum number of pending unlock entries that may mature in a single frame.
		#[pallet::constant]
		type MaxPendingUnlocksPerFrame: Get<u32>;
		/// The number of frames an allocation decrease remains locked before release.
		#[pallet::constant]
		type TreasuryExitDelayFrames: Get<FrameId>;

		type MiningFrameTransitionProvider: MiningFrameTransitionProvider;

		/// Optional hook for operational account state updates.
		type OperationalAccountsHook: OperationalAccountsHook<Self::AccountId, Self::Balance>;

		/// Provider of pending operational rewards for payout from treasury reserves.
		type OperationalRewardsProvider: OperationalRewardsProvider<Self::AccountId, Self::Balance>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		ContributedToTreasury,
	}

	/// The currently earning contributors for the current epoch's bond funds. Sorted by highest
	/// bids first
	#[pallet::storage]
	pub type VaultPoolsByFrame<T: Config> = StorageMap<
		_,
		Twox64Concat,
		FrameId,
		BoundedBTreeMap<VaultId, TreasuryPool<T>, T::MaxVaultsPerPool>,
		ValueQuery,
	>;

	/// Per-vault per-account commitment and held principal (long-lived source of truth).
	#[pallet::storage]
	pub type FunderStateByVaultAndAccount<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		VaultId,
		Twox64Concat,
		T::AccountId,
		FunderState<T>,
		OptionQuery,
	>;

	/// The treasury pool for the current frame. This correlates with the bids coming in for the
	/// current frame. Sorted with the biggest share first. (current frame)
	#[pallet::storage]
	pub type CapitalActive<T: Config> =
		StorageValue<_, BoundedVec<TreasuryCapital<T>, T::MaxVaultsPerPool>, ValueQuery>;

	/// Index of delayed unlocks that mature at the given frame.
	#[pallet::storage]
	pub type PendingUnlocksByFrame<T: Config> = StorageMap<
		_,
		Twox64Concat,
		FrameId,
		BoundedVec<PendingUnlock<T>, T::MaxPendingUnlocksPerFrame>,
		ValueQuery,
	>;

	/// Oldest matured unlock frame that still has entries to retry.
	#[pallet::storage]
	pub type PendingUnlockRetryCursor<T: Config> = StorageValue<_, FrameId, OptionQuery>;

	/// Bounded, sorted working set for a vault's treasury pool construction.
	///
	/// `FunderStateByVaultAndAccount` stores every funder's state. This index only keeps the top
	/// funded contributors plus a small standby window so treasury can recover from a few exits
	/// without a global scan. Entries are stored with the operator first while funded, then the
	/// remaining accounts sorted by held principal descending.
	#[pallet::storage]
	pub type FundersByVaultId<T: Config> = StorageMap<
		_,
		Twox64Concat,
		VaultId,
		BoundedVec<(T::AccountId, T::Balance), T::MaxTrackedTreasuryFunders>,
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
		/// An error occurred reserving treasury reserves from the bid pool.
		CouldNotFundTreasury {
			frame_id: FrameId,
			amount: T::Balance,
			dispatch_error: DispatchError,
		},
		/// Funds from the active bid pool have been distributed
		BidPoolDistributed {
			frame_id: FrameId,
			bid_pool_distributed: T::Balance,
			treasury_reserves: T::Balance,
			bid_pool_shares: u32,
		},
		/// The next bid pool has been locked in
		NextBidPoolCapitalLocked {
			frame_id: FrameId,
			total_activated_capital: T::Balance,
			participating_vaults: u32,
		},
		/// An error occurred releasing a contributor hold
		ErrorRefundingTreasuryCapital {
			frame_id: FrameId,
			vault_id: VaultId,
			amount: T::Balance,
			account_id: T::AccountId,
			dispatch_error: DispatchError,
		},
		/// Some mining bond capital was refunded because vault securitized satoshis (`sats *
		/// securitization ratio`) were lower than bond capital
		RefundedTreasuryCapital {
			frame_id: FrameId,
			vault_id: VaultId,
			amount: T::Balance,
			account_id: T::AccountId,
		},
		/// A funder has set their allocation for a vault
		VaultFunderAllocation {
			vault_id: VaultId,
			account_id: T::AccountId,
			amount: T::Balance,
			previous_amount: Option<T::Balance>,
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
		CouldNotFindTreasury,
		/// Max contributors for a fund exceeded
		MaxContributorsExceeded,
		/// Max Vaults exceeded
		MaxVaultsExceeded,
		/// Max pending unlocks scheduled for a frame exceeded
		MaxPendingUnlocksExceeded,
		/// This fund has already been renewed
		AlreadyRenewed,
		/// Vault operator only
		NotAVaultOperator,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// All funders can set their committed principal for a vault.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::set_allocation())]
		pub fn set_allocation(
			origin: OriginFor<T>,
			vault_id: VaultId,
			amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let new_commitment = amount;
			ensure!(
				amount.is_zero() || amount >= T::MinimumArgonsPerContributor::get(),
				Error::<T>::BelowMinimum
			);

			let frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
			let mut state =
				FunderStateByVaultAndAccount::<T>::get(vault_id, &who).unwrap_or_default();
			let previous_allocation = state.target_principal();

			if new_commitment > previous_allocation {
				let delta = new_commitment.saturating_sub(previous_allocation);
				Self::update_basis(&mut state, frame_id);
				let canceled_unlock = delta.min(state.pending_unlock_amount);
				if !canceled_unlock.is_zero() {
					state.pending_unlock_amount.saturating_reduce(canceled_unlock);
					if state.pending_unlock_amount.is_zero() {
						Self::clear_pending_unlock(vault_id, &who, &mut state);
					}
				}

				let additional_hold = delta.saturating_sub(canceled_unlock);
				if !additional_hold.is_zero() {
					Self::create_hold(&who, additional_hold)?;
					state.held_principal.saturating_accrue(additional_hold);
				}
			} else if new_commitment < previous_allocation {
				let to_unlock = previous_allocation.saturating_sub(new_commitment);

				Self::update_basis(&mut state, frame_id);
				let unlock_frame_id = frame_id.saturating_add(T::TreasuryExitDelayFrames::get());
				if state.pending_unlock_amount.is_zero() ||
					state.pending_unlock_at_frame != Some(unlock_frame_id)
				{
					PendingUnlocksByFrame::<T>::try_mutate(unlock_frame_id, |pending_unlocks| {
						if pending_unlocks.iter().any(|pending_unlock| {
							pending_unlock.vault_id == vault_id && pending_unlock.account_id == who
						}) {
							return Ok::<(), Error<T>>(());
						}

						pending_unlocks
							.try_push(PendingUnlock { vault_id, account_id: who.clone() })
							.map_err(|_| Error::<T>::MaxPendingUnlocksExceeded)?;
						Ok::<(), Error<T>>(())
					})?;
				}
				if state.pending_unlock_amount != T::Balance::zero() &&
					state.pending_unlock_at_frame != Some(unlock_frame_id)
				{
					Self::remove_pending_unlock_index(
						vault_id,
						&who,
						state.pending_unlock_at_frame,
					);
				}
				state.pending_unlock_amount.saturating_accrue(to_unlock);
				state.pending_unlock_at_frame = Some(unlock_frame_id);
			}

			if state.held_principal.is_zero() {
				FunderStateByVaultAndAccount::<T>::remove(vault_id, &who);
				Self::refresh_funder_index(vault_id, &who, T::Balance::zero());
			} else {
				FunderStateByVaultAndAccount::<T>::insert(vault_id, &who, &state);
				Self::refresh_funder_index(vault_id, &who, state.held_principal);
			}

			Self::deposit_event(Event::<T>::VaultFunderAllocation {
				vault_id,
				account_id: who,
				amount: new_commitment,
				previous_amount: if previous_allocation.is_zero() {
					None
				} else {
					Some(previous_allocation)
				},
			});
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_bid_pool_account() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		pub fn get_treasury_reserves_account() -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(TREASURY_RESERVES_SUB_ACCOUNT)
		}

		fn ensure_account_provider(account_id: &T::AccountId) {
			if frame_system::Pallet::<T>::providers(account_id) == 0 {
				frame_system::Pallet::<T>::inc_providers(account_id);
			}
		}

		pub(crate) fn refresh_funder_index(
			vault_id: VaultId,
			account_id: &T::AccountId,
			held_principal: T::Balance,
		) {
			let operator = T::TreasuryVaultProvider::get_vault_operator(vault_id);

			FundersByVaultId::<T>::mutate_exists(vault_id, |maybe_funders| {
				if maybe_funders.is_none() {
					if held_principal.is_zero() {
						return;
					}
					*maybe_funders = Some(BoundedVec::default());
				}

				let funders = maybe_funders
					.as_mut()
					.expect("tracked funders are initialized unless zero-principal was returned");

				if let Some(existing_index) =
					funders.iter().position(|(existing_account, _)| existing_account == account_id)
				{
					funders.remove(existing_index);
				}

				if held_principal.is_zero() {
					if funders.is_empty() {
						*maybe_funders = None;
					}
					return;
				}

				let is_operator = operator.as_ref() == Some(account_id);
				let insert_pos = if is_operator {
					0
				} else {
					funders
						.iter()
						.position(|(tracked_account, tracked_principal)| {
							operator.as_ref() != Some(tracked_account) &&
								held_principal > *tracked_principal
						})
						.unwrap_or(funders.len())
				};

				if !funders.is_full() {
					let _ = funders.try_insert(insert_pos, (account_id.clone(), held_principal));
					return;
				}

				if is_operator {
					funders.pop();
					let _ = funders.try_insert(0, (account_id.clone(), held_principal));
					return;
				}

				if insert_pos >= funders.len() {
					return;
				}

				funders.pop();
				let _ = funders.try_insert(insert_pos, (account_id.clone(), held_principal));
			});
		}

		pub(crate) fn create_hold(account_id: &T::AccountId, amount: T::Balance) -> DispatchResult {
			if amount == Zero::zero() {
				return Ok(());
			}
			let hold_reason = HoldReason::ContributedToTreasury;
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
			let reason = HoldReason::ContributedToTreasury;

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
			Self::ensure_account_provider(&bid_pool_account);
			let mut total_bid_pool_amount = T::Currency::balance(&bid_pool_account);

			let reserves_amount =
				T::PercentForTreasuryReserves::get().mul_ceil(total_bid_pool_amount);
			let mut reserves_reserved = T::Balance::zero();
			let reserves_account = Self::get_treasury_reserves_account();
			Self::ensure_account_provider(&reserves_account);
			if let Err(e) = T::Currency::transfer(
				&bid_pool_account,
				&reserves_account,
				reserves_amount,
				Preservation::Preserve,
			) {
				Self::deposit_event(Event::<T>::CouldNotFundTreasury {
					frame_id,
					amount: reserves_amount,
					dispatch_error: e,
				});
			} else {
				reserves_reserved = reserves_amount;
				total_bid_pool_amount.saturating_reduce(reserves_amount);
			}
			let mut remaining_bid_pool = total_bid_pool_amount;

			let mut treasury_by_vault = VaultPoolsByFrame::<T>::get(frame_id);

			// Use locked-in capital snapshot for this frame.
			let bid_pool_capital = CapitalActive::<T>::take();
			let bid_pool_entrants = bid_pool_capital.len();
			let total_bid_pool_capital = bid_pool_capital
				.iter()
				.fold(T::Balance::zero(), |acc, x| acc.saturating_add(x.activated_capital));

			for entrant in bid_pool_capital {
				if total_bid_pool_capital.is_zero() {
					break;
				}
				let Some(vault_fund) = treasury_by_vault.get_mut(&entrant.vault_id) else {
					continue;
				};
				let Some(vault_account_id) =
					T::TreasuryVaultProvider::get_vault_operator(entrant.vault_id)
				else {
					continue;
				};
				let earnings =
					Perbill::from_rational(entrant.activated_capital, total_bid_pool_capital)
						.mul_floor(total_bid_pool_amount);
				remaining_bid_pool.saturating_reduce(earnings);
				vault_fund.distributed_earnings = Some(earnings);

				let mut vault_share = Permill::one()
					.saturating_sub(vault_fund.vault_sharing_percent)
					.mul_floor(earnings);

				let contributor_amount = earnings.saturating_sub(vault_share);
				let contributor_funds = entrant.activated_capital;
				let mut contributor_distribution_remaining = contributor_amount;
				let mut distributions = BTreeMap::<T::AccountId, T::Balance>::new();

				let mut vault_contributed_capital = T::Balance::zero();
				for (account, amount) in &vault_fund.bond_holders {
					if *account == vault_account_id {
						vault_contributed_capital = *amount;
					}
					let prorata = Permill::from_rational(*amount, contributor_funds)
						.mul_floor(contributor_amount);
					contributor_distribution_remaining.saturating_reduce(prorata);
					distributions.entry(account.clone()).or_default().saturating_accrue(prorata);
				}
				// add remaining back to vault share
				vault_share.saturating_accrue(contributor_distribution_remaining);

				// give change to vault
				let mut earnings_for_vault = vault_share;
				for (account, account_earnings) in distributions {
					// the vault must collect earnings
					if account == vault_account_id {
						earnings_for_vault.saturating_accrue(account_earnings);
						continue;
					}
					let should_rollover =
						FunderStateByVaultAndAccount::<T>::get(entrant.vault_id, &account)
							.map(|state| !state.held_principal.is_zero())
							.unwrap_or(false);

					let distribution_result = if should_rollover {
						T::Currency::transfer_and_hold(
							&HoldReason::ContributedToTreasury.into(),
							&bid_pool_account,
							&account,
							account_earnings,
							Precision::Exact,
							Preservation::Expendable,
							Fortitude::Force,
						)
						.map(|_| {
							// For active non-vault operators, auto-rollover: increase committed
							// principal by earnings so it is active in the next frame.
							FunderStateByVaultAndAccount::<T>::mutate(
								entrant.vault_id,
								&account,
								|e| {
									let mut s = e.take().unwrap_or_default();
									Self::update_basis(&mut s, frame_id);
									s.held_principal.saturating_accrue(account_earnings);
									s.lifetime_compounded_earnings
										.saturating_accrue(account_earnings);
									Self::refresh_funder_index(
										entrant.vault_id,
										&account,
										s.held_principal,
									);
									*e = Some(s);
								},
							);
						})
					} else {
						// If a contributor fully exited at frame start but still has earnings from
						// the just-finished frame, pay them out free instead of recreating a held
						// treasury position.
						T::Currency::transfer(
							&bid_pool_account,
							&account,
							account_earnings,
							Preservation::Preserve,
						)
						.map(|_| ())
					};

					if let Err(e) = distribution_result {
						Self::deposit_event(Event::<T>::CouldNotDistributeBidPool {
							account_id: account.clone(),
							frame_id,
							vault_id: entrant.vault_id,
							amount: account_earnings,
							dispatch_error: e,
							is_for_vault: false,
						});
					}
				}

				T::TreasuryVaultProvider::record_vault_frame_earnings(
					&bid_pool_account,
					VaultTreasuryFrameEarnings {
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
			VaultPoolsByFrame::<T>::insert(frame_id, treasury_by_vault);

			Self::deposit_event(Event::<T>::BidPoolDistributed {
				frame_id,
				bid_pool_distributed: total_bid_pool_amount - remaining_bid_pool,
				treasury_reserves: reserves_reserved,
				bid_pool_shares: bid_pool_entrants as u32,
			});
		}

		/// Locks in the vault capital for the next frame based on vault securitized satoshis (`sats
		/// * securitization ratio`) and funders.
		pub(crate) fn lock_in_vault_capital(frame_id: FrameId) {
			let mut vault_candidates: Vec<VaultCandidateCapital<T>> = Vec::new();

			for vault_id in FundersByVaultId::<T>::iter_keys() {
				let activated_cap = Self::get_vault_securitized_funds_cap(vault_id);
				if activated_cap.is_zero() {
					continue;
				}

				let mut pool = TreasuryPool::<T>::new(vault_id);
				let operator = T::TreasuryVaultProvider::get_vault_operator(vault_id);
				let mut raised = T::Balance::zero();
				for (account, held_principal) in FundersByVaultId::<T>::get(vault_id) {
					let remaining = activated_cap.saturating_sub(raised);
					let to_bond = held_principal.min(remaining);
					if to_bond.is_zero() {
						break;
					}

					if pool.try_insert_bond_holder(account, to_bond, operator.as_ref()).is_ok() {
						raised.saturating_accrue(to_bond);
					}
				}

				if raised.is_zero() {
					continue;
				}

				vault_candidates.push(VaultCandidateCapital {
					vault_id,
					capital_potential: raised,
					pool,
				});
			}

			// Pick top vaults by target capital.
			vault_candidates.sort_by(|a, b| b.capital_potential.cmp(&a.capital_potential));
			vault_candidates.truncate(T::MaxVaultsPerPool::get() as usize);

			// Mutate pools only for selected vaults and build the locked snapshot.
			let mut vault_pools = BoundedBTreeMap::new();
			let mut capital_entrants = BoundedVec::new();
			let mut total_activated_capital = T::Balance::zero();

			for candidate in vault_candidates {
				let VaultCandidateCapital { vault_id, capital_potential, pool } = candidate;
				if capital_potential.is_zero() {
					continue;
				}

				// Ensure we have a pool for this vault; do NOT fail if it already exists.
				if vault_pools.try_insert(vault_id, pool.clone()).is_err() {
					// MaxVaultsPerPool reached in BTreeMap; stop.
					break;
				}

				if capital_entrants
					.try_push(TreasuryCapital {
						vault_id,
						activated_capital: capital_potential,
						frame_id,
					})
					.is_err()
				{
					break;
				}

				total_activated_capital.saturating_accrue(capital_potential);
				let vault_operator = T::TreasuryVaultProvider::get_vault_operator(vault_id);
				for (account_id, amount) in pool.bond_holders {
					let is_operator = vault_operator.as_ref() == Some(&account_id);
					if is_operator && !amount.is_zero() {
						T::OperationalAccountsHook::treasury_pool_participated(&account_id, amount);
					}
				}
			}

			// Store locked snapshot in the same order as the selected candidates (largest first).
			let participating_vaults = capital_entrants.len() as u32;

			CapitalActive::<T>::set(capital_entrants);

			Self::deposit_event(Event::<T>::NextBidPoolCapitalLocked {
				frame_id,
				total_activated_capital,
				participating_vaults,
			});

			VaultPoolsByFrame::<T>::insert(frame_id, vault_pools);
		}

		pub(crate) fn pay_operational_rewards(
			payouts: Vec<OperationalRewardPayout<T::AccountId, T::Balance>>,
		) {
			if payouts.is_empty() {
				return;
			}
			let treasury_reserves_account = Self::get_treasury_reserves_account();
			Self::ensure_account_provider(&treasury_reserves_account);
			let available = T::Currency::reducible_balance(
				&treasury_reserves_account,
				Preservation::Preserve,
				Fortitude::Polite,
			);
			let total_pending = payouts
				.iter()
				.fold(T::Balance::zero(), |acc, payout| acc.saturating_add(payout.amount));

			let pay_in_full = !total_pending.is_zero() && available >= total_pending;
			let payout_budget = if pay_in_full { total_pending } else { available };
			let mut remaining_budget = payout_budget;
			for payout in payouts {
				let mut payout_amount = if pay_in_full {
					payout.amount
				} else if total_pending.is_zero() ||
					payout.amount.is_zero() ||
					remaining_budget.is_zero()
				{
					T::Balance::zero()
				} else {
					Perbill::from_rational(payout.amount, total_pending).mul_floor(payout_budget)
				};
				if payout_amount > remaining_budget {
					payout_amount = remaining_budget;
				}
				if payout_amount.is_zero() {
					T::OperationalRewardsProvider::mark_reward_paid(&payout, T::Balance::zero());
					continue;
				}

				if let Err(e) = T::Currency::transfer(
					&treasury_reserves_account,
					&payout.payout_account,
					payout_amount,
					Preservation::Preserve,
				) {
					log::error!(
						"Failed to pay operational reward {:?} to {:?}: {:?}",
						payout.reward_kind,
						payout.payout_account,
						e
					);
					T::OperationalRewardsProvider::mark_reward_paid(&payout, T::Balance::zero());
					continue;
				}
				remaining_budget.saturating_reduce(payout_amount);
				T::OperationalRewardsProvider::mark_reward_paid(&payout, payout_amount);
			}
		}

		fn frame_transition_base_weight() -> Weight {
			T::WeightInfo::on_frame_transition().saturating_add(
				T::OperationalAccountsHook::treasury_pool_participated_weight()
					.saturating_mul(u64::from(T::MaxVaultsPerPool::get())),
			)
		}

		pub(crate) fn run_frame_transition(frame_id: FrameId) {
			if frame_id == 0 {
				return;
			}

			let pending_rewards = T::OperationalRewardsProvider::pending_rewards();
			let payout_frame = frame_id - 1;
			info!(
				"Starting next treasury pool for frame {frame_id}. Distributing frame {payout_frame}."
			);
			Self::release_pending_unlocks(frame_id);
			Self::distribute_bid_pool(payout_frame);
			Self::lock_in_vault_capital(frame_id);
			Self::pay_operational_rewards(pending_rewards);
		}

		pub(crate) fn release_pending_unlocks(frame_id: FrameId) {
			let exit_delay_frames = T::TreasuryExitDelayFrames::get();
			if frame_id >= exit_delay_frames {
				VaultPoolsByFrame::<T>::remove(frame_id.saturating_sub(exit_delay_frames));
			}

			let start_frame =
				PendingUnlockRetryCursor::<T>::take().unwrap_or(frame_id).min(frame_id);
			let mut next_retry_frame = None;

			for due_frame in start_frame..=frame_id {
				let pending_unlocks = PendingUnlocksByFrame::<T>::take(due_frame);
				if pending_unlocks.is_empty() {
					continue;
				}

				let mut failed_pending_unlocks =
					BoundedVec::<PendingUnlock<T>, T::MaxPendingUnlocksPerFrame>::default();

				for pending_unlock in pending_unlocks {
					let mut released_amount = T::Balance::zero();
					let res = FunderStateByVaultAndAccount::<T>::try_mutate_exists(
						pending_unlock.vault_id,
						&pending_unlock.account_id,
						|entry| {
							let Some(state) = entry.as_mut() else {
								return Ok(());
							};
							if state.pending_unlock_amount.is_zero() ||
								state.pending_unlock_at_frame != Some(due_frame)
							{
								return Ok(());
							}

							Self::update_basis(state, due_frame);
							released_amount = state.pending_unlock_amount;
							Self::release_hold(&pending_unlock.account_id, released_amount)?;
							state.held_principal.saturating_reduce(released_amount);
							Self::clear_pending_unlock(
								pending_unlock.vault_id,
								&pending_unlock.account_id,
								state,
							);
							Self::refresh_funder_index(
								pending_unlock.vault_id,
								&pending_unlock.account_id,
								state.held_principal,
							);

							if state.held_principal.is_zero() {
								*entry = None;
							}

							Ok(())
						},
					);

					if let Err(e) = res {
						let _ = failed_pending_unlocks.try_push(PendingUnlock {
							vault_id: pending_unlock.vault_id,
							account_id: pending_unlock.account_id.clone(),
						});
						if next_retry_frame.is_none() {
							next_retry_frame = Some(due_frame);
						}
						Self::deposit_event(Event::<T>::ErrorRefundingTreasuryCapital {
							frame_id: due_frame,
							vault_id: pending_unlock.vault_id,
							amount: released_amount,
							account_id: pending_unlock.account_id,
							dispatch_error: e,
						});
						continue;
					}
					if released_amount.is_zero() {
						continue;
					}

					Self::deposit_event(Event::<T>::RefundedTreasuryCapital {
						frame_id: due_frame,
						vault_id: pending_unlock.vault_id,
						amount: released_amount,
						account_id: pending_unlock.account_id,
					});
				}

				if !failed_pending_unlocks.is_empty() {
					PendingUnlocksByFrame::<T>::insert(due_frame, failed_pending_unlocks);
				}
			}

			if let Some(retry_frame) = next_retry_frame {
				PendingUnlockRetryCursor::<T>::put(retry_frame);
			}
		}

		fn update_basis(state: &mut FunderState<T>, at_frame_id: FrameId) {
			let frames_passed =
				at_frame_id.saturating_sub(state.lifetime_principal_last_basis_frame);
			if frames_passed > 0 {
				let additional_principal =
					state.held_principal.saturating_mul((frames_passed as u128).into());
				state.lifetime_principal_deployed.saturating_accrue(additional_principal);
				state.lifetime_principal_last_basis_frame = at_frame_id;
			}
		}

		fn get_vault_securitized_funds_cap(vault_id: VaultId) -> T::Balance {
			let securitized_satoshis = T::TreasuryVaultProvider::get_securitized_satoshis(vault_id);
			T::PriceProvider::get_bitcoin_argon_price(securitized_satoshis).unwrap_or_default()
		}

		fn clear_pending_unlock(
			vault_id: VaultId,
			account_id: &T::AccountId,
			state: &mut FunderState<T>,
		) {
			Self::remove_pending_unlock_index(vault_id, account_id, state.pending_unlock_at_frame);
			state.pending_unlock_amount = T::Balance::zero();
			state.pending_unlock_at_frame = None;
		}

		fn remove_pending_unlock_index(
			vault_id: VaultId,
			account_id: &T::AccountId,
			unlock_frame_id: Option<FrameId>,
		) {
			let Some(unlock_frame_id) = unlock_frame_id else {
				return;
			};

			PendingUnlocksByFrame::<T>::mutate_exists(unlock_frame_id, |maybe_pending_unlocks| {
				let Some(pending_unlocks) = maybe_pending_unlocks.as_mut() else {
					return;
				};

				if let Some(index) = pending_unlocks.iter().position(|pending_unlock| {
					pending_unlock.vault_id == vault_id && pending_unlock.account_id == *account_id
				}) {
					pending_unlocks.remove(index);
					if pending_unlocks.is_empty() {
						*maybe_pending_unlocks = None;
					}
				}
			});
		}
	}

	impl<T: Config> OperationalRewardsPayer<T::AccountId, T::Balance> for Pallet<T> {
		fn try_pay_reward_weight() -> Weight {
			T::WeightInfo::try_pay_reward()
		}

		fn try_pay_reward(reward: &OperationalRewardPayout<T::AccountId, T::Balance>) -> bool {
			if reward.amount.is_zero() {
				return true;
			}
			let treasury_reserves_account = Self::get_treasury_reserves_account();
			Self::ensure_account_provider(&treasury_reserves_account);
			let available = T::Currency::reducible_balance(
				&treasury_reserves_account,
				Preservation::Preserve,
				Fortitude::Polite,
			);
			if reward.amount > available {
				return false;
			}
			if let Err(e) = T::Currency::transfer(
				&treasury_reserves_account,
				&reward.payout_account,
				reward.amount,
				Preservation::Preserve,
			) {
				log::error!(
					"Failed to pay operational reward {:?} to {:?}: {:?}",
					reward.reward_kind,
					reward.payout_account,
					e
				);
				return false;
			}
			true
		}
	}

	impl<T: Config> MiningBidPoolProvider for Pallet<T> {
		type Balance = T::Balance;
		type AccountId = T::AccountId;

		fn get_bid_pool_account() -> Self::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}

	impl<T: Config> OnNewSlot<T::AccountId> for Pallet<T> {
		type Key = BlockSealAuthorityId;

		fn on_frame_start(frame_id: FrameId) {
			Self::run_frame_transition(frame_id);
		}

		fn on_frame_start_weight(_frame_id: FrameId) -> Weight {
			Self::frame_transition_base_weight()
				.saturating_add(OperationalRewardsProviderWeights::<T>::pending_rewards())
				.saturating_add(T::WeightInfo::pay_operational_rewards())
		}
	}

	impl<T: Config> TreasuryPoolProvider<T::AccountId> for Pallet<T> {
		type Weights = ProviderWeightAdapter<T>;

		fn has_pool_participation(vault_id: VaultId, account_id: &T::AccountId) -> bool {
			FunderStateByVaultAndAccount::<T>::get(vault_id, account_id)
				.map(|state| !state.held_principal.is_zero())
				.unwrap_or(false)
		}
	}

	#[derive(
		Encode, Decode, Clone, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct TreasuryCapital<T: Config> {
		#[codec(compact)]
		pub vault_id: VaultId,
		#[codec(compact)]
		pub activated_capital: T::Balance,
		/// The frame id this treasury pool is for
		#[codec(compact)]
		pub frame_id: FrameId,
	}

	#[derive(
		Encode,
		Decode,
		PartialEqNoBound,
		Eq,
		RuntimeDebugNoBound,
		TypeInfo,
		DefaultNoBound,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct TreasuryPool<T: Config> {
		/// The amount of microgons per bond holder. Sorted with largest first
		pub bond_holders: BoundedVec<(T::AccountId, T::Balance), T::MaxTreasuryContributors>,

		/// The amount of argons that have been distributed to the fund (vault + contributors)
		pub distributed_earnings: Option<T::Balance>,

		/// The vault percent of profits shared
		#[codec(compact)]
		pub vault_sharing_percent: Permill,
	}

	impl<T: Config> Clone for TreasuryPool<T> {
		fn clone(&self) -> Self {
			Self {
				bond_holders: self.bond_holders.clone(),
				distributed_earnings: self.distributed_earnings,
				vault_sharing_percent: self.vault_sharing_percent,
			}
		}
	}

	struct VaultCandidateCapital<T: Config> {
		vault_id: VaultId,
		capital_potential: T::Balance,
		pool: TreasuryPool<T>,
	}

	#[derive(
		Encode, Decode, Clone, PartialEqNoBound, Eq, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct PendingUnlock<T: Config> {
		#[codec(compact)]
		pub vault_id: VaultId,
		pub account_id: T::AccountId,
	}

	impl<T: Config> TreasuryPool<T> {
		pub fn new(vault_id: VaultId) -> Self {
			let sharing = T::TreasuryVaultProvider::get_vault_profit_sharing_percent(vault_id)
				.unwrap_or_default();
			Self { vault_sharing_percent: sharing, ..Default::default() }
		}

		pub fn raised_capital(&self) -> T::Balance {
			self.bond_holders
				.iter()
				.fold(T::Balance::zero(), |acc, (_, b)| acc.saturating_add(*b))
		}

		/// returns any bond holder that needs to be refunded due to being the lowest and the
		/// list being full
		pub fn try_insert_bond_holder(
			&mut self,
			account: T::AccountId,
			amount: T::Balance,
			vault_operator: Option<&T::AccountId>,
		) -> Result<(), Error<T>> {
			// Remove any existing entry for this account to avoid duplicates and re-sort correctly.
			if let Some(existing_idx) = self.bond_holders.iter().position(|(a, _)| a == &account) {
				self.bond_holders.remove(existing_idx);
			}

			// Maintain bond_holders sorted by starting_balance DESC (largest first).
			let insert_pos =
				self.bond_holders.binary_search_by(|(_, x)| amount.cmp(x)).unwrap_or_else(|x| x);

			let is_operator = vault_operator == Some(&account);

			if self.bond_holders.is_full() {
				// Non-operators must beat at least one existing entry.
				if !is_operator {
					ensure!(insert_pos < self.bond_holders.len(), Error::<T>::ContributionTooLow);
				}

				// Evict the lowest non-operator (from the end). The operator is never evicted.
				let mut drop_index = self.bond_holders.len().saturating_sub(1);
				if let Some(op) = vault_operator {
					while drop_index > 0 {
						if self.bond_holders[drop_index].0 != *op {
							break;
						}
						drop_index -= 1;
					}
					if drop_index == 0 && self.bond_holders[drop_index].0 == *op {
						// Only the operator remains; cannot evict.
						return Err(Error::<T>::ContributionTooLow);
					}
				}
				self.bond_holders.remove(drop_index);
			}

			self.bond_holders
				.try_insert(insert_pos, (account, amount))
				.map_err(|_| Error::<T>::MaxContributorsExceeded)?;
			Ok(())
		}
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
	pub struct FunderState<T: Config> {
		/// Principal currently held by the Currency pallet and active for treasury pool selection.
		#[codec(compact)]
		pub held_principal: T::Balance,
		/// Principal scheduled to unlock after the current cooldown.
		#[codec(compact)]
		pub pending_unlock_amount: T::Balance,
		/// Frame at which the pending unlock amount becomes releasable.
		pub pending_unlock_at_frame: Option<FrameId>,
		/// Earnings over the lifetime of a user with compounding earnings
		#[codec(compact)]
		pub lifetime_compounded_earnings: T::Balance,
		/// Lifetime target principal deployed
		#[codec(compact)]
		pub lifetime_principal_deployed: T::Balance,
		/// Lifetime principal last frame basis
		#[codec(compact)]
		pub lifetime_principal_last_basis_frame: FrameId,
	}

	impl<T: Config> FunderState<T> {
		fn target_principal(&self) -> T::Balance {
			self.held_principal.saturating_sub(self.pending_unlock_amount)
		}
	}
}
