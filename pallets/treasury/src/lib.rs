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
/// Each slot's treasury pool can bond argons up to 1/10th of a vault's `activated securitization`.
/// `Activated securitization` is 2x the amount of LockedBitcoins.
///
/// ## Profits from Bid Pool
/// Once each bid pool is closed, 20% of the pool is reserved for treasury reserves. (Operational
/// rewards are one use of these reserves.) The remaining funds are
/// distributed pro-rata to each vault's slot treasury pool. Vault's automatically disperse funds
/// to contributors based on the vault's sharing percent, and each individual contributor's
/// pro-rata.
///
/// The limitations to bonding argons are:
/// - The maximum number of contributors to a fund (`MaxTreasuryContributors`)
/// - The minimum amount of bonded argons per contributor (`MinimumArgonsPerContributor`)
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::{collections::BTreeMap, vec::Vec};
	use argon_primitives::vault::{
		MiningBidPoolProvider, TreasuryVaultProvider, VaultTreasuryFrameEarnings,
	};
	use pallet_prelude::argon_primitives::{
		MiningFrameTransitionProvider, OperationalAccountsHook, OperationalRewardPayout,
		OperationalRewardsPayer, OperationalRewardsProvider,
	};
	use sp_runtime::{BoundedBTreeMap, traits::AccountIdConversion};
	use tracing::info;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);
	const TREASURY_RESERVES_SUB_ACCOUNT: [u8; 16] = *b"treasury-reserve";

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

		/// The maximum number of contributors to a bond fund
		#[pallet::constant]
		type MaxTreasuryContributors: Get<u32>;
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

	/// Per-vault per-account commitment and current bonded principal (long-lived source of truth).
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

	/// Accounts that have a non-zero commitment for a vault. Bounded for predictable weights.
	#[pallet::storage]
	pub type FundersByVaultId<T: Config> = StorageMap<
		_,
		Twox64Concat,
		VaultId,
		BoundedBTreeSet<T::AccountId, T::MaxTreasuryContributors>,
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
		/// Some mining bond capital was refunded due to less activated vault funds than bond
		/// capital
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
		/// The added amount would exceed the activated securitization
		ActivatedSecuritizationExceeded,
		/// Max Vaults exceeded
		MaxVaultsExceeded,
		/// This fund has already been renewed
		AlreadyRenewed,
		/// Vault operator only
		NotAVaultOperator,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			if T::MiningFrameTransitionProvider::is_new_frame_started().is_some() {
				// Snapshot how many queued rewards we will attempt this frame so
				// on_finalize work is pre-accounted in on_initialize.
				let payout_count = T::OperationalRewardsProvider::pending_rewards().len() as u64;
				Self::frame_transition_base_weight()
					.saturating_add(T::WeightInfo::try_pay_reward().saturating_mul(payout_count))
					.saturating_add(T::DbWeight::get().reads_writes(1, 1))
			} else {
				Weight::zero()
			}
		}

		fn on_finalize(_n: BlockNumberFor<T>) {
			if let Some(frame_id) = T::MiningFrameTransitionProvider::is_new_frame_started() {
				if frame_id == 0 {
					return;
				}
				let payout_frame = frame_id - 1;
				info!(
					"Starting next treasury pool for frame {frame_id}. Distributing frame {payout_frame}."
				);
				Self::release_bonded_principal(frame_id);
				Self::distribute_bid_pool(payout_frame);
				Self::lock_in_vault_capital(frame_id);
				Self::pay_operational_rewards();
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// All funders can set their committed principal for a vault.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::bond_argons())]
		pub fn set_allocation(
			origin: OriginFor<T>,
			vault_id: VaultId,
			amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Commitment is a target principal; it is spread evenly across 10 frames.
			let new_commitment = amount;
			ensure!(
				amount.is_zero() || amount >= T::MinimumArgonsPerContributor::get(),
				Error::<T>::BelowMinimum
			);

			// The canonical bonded amount is tracked in long-lived storage.
			let existing_funding =
				FunderStateByVaultAndAccount::<T>::get(vault_id, &who).unwrap_or_default();
			let frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
			if new_commitment > existing_funding.held_principal {
				Self::create_hold(&who, new_commitment - existing_funding.held_principal)?;
				FunderStateByVaultAndAccount::<T>::mutate(vault_id, who.clone(), |e| {
					let mut s = e.take().unwrap_or_default();
					Self::update_basis(&mut s, frame_id);
					s.target_principal = new_commitment;
					s.held_principal = new_commitment;
					*e = Some(s);
				});
				FundersByVaultId::<T>::try_mutate(vault_id, |v| {
					v.try_insert(who.clone()).map_err(|_| Error::<T>::MaxContributorsExceeded)
				})?;
			} else if new_commitment < existing_funding.held_principal {
				FunderStateByVaultAndAccount::<T>::try_mutate(vault_id, who.clone(), |state| {
					if let Some(s) = state.as_mut() {
						s.target_principal = new_commitment;
					}
					Self::reduce_allocation(
						who.clone(),
						vault_id,
						state,
						T::MiningFrameTransitionProvider::get_current_frame_id(),
					)
				})?;
			}

			Self::deposit_event(Event::<T>::VaultFunderAllocation {
				vault_id,
				account_id: who,
				amount: new_commitment,
				previous_amount: if existing_funding.held_principal.is_zero() {
					None
				} else {
					Some(existing_funding.held_principal)
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
					match T::Currency::transfer_and_hold(
						&HoldReason::ContributedToTreasury.into(),
						&bid_pool_account,
						&account,
						account_earnings,
						Precision::Exact,
						Preservation::Expendable,
						Fortitude::Force,
					) {
						// For non-vault operators, auto-rollover: increase both held and target
						// principal by earnings
						Ok(_) => FunderStateByVaultAndAccount::<T>::mutate(
							entrant.vault_id,
							&account,
							|e| {
								let mut s = e.take().unwrap_or_default();
								// Increase both committed and held principal by earnings (eg,
								// auto-rollover)
								Self::update_basis(&mut s, frame_id);
								s.target_principal.saturating_accrue(account_earnings);
								s.held_principal.saturating_accrue(account_earnings);
								s.lifetime_compounded_earnings.saturating_accrue(account_earnings);
								*e = Some(s);
							},
						),
						Err(e) => Self::deposit_event(Event::<T>::CouldNotDistributeBidPool {
							account_id: account.clone(),
							frame_id,
							vault_id: entrant.vault_id,
							amount: account_earnings,
							dispatch_error: e,
							is_for_vault: false,
						}),
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

		/// Locks in the vault capital for the next frame based on activated securitization and
		/// funders.
		pub(crate) fn lock_in_vault_capital(frame_id: FrameId) {
			let mut vault_candidates: Vec<VaultCandidateCapital<T>> = Vec::new();

			for vault_id in FundersByVaultId::<T>::iter_keys() {
				let activated_cap = Self::get_vault_activated_funds_per_slot(vault_id);
				if activated_cap.is_zero() {
					continue;
				}

				let mut pool = TreasuryPool::<T>::new(vault_id);
				let operator = T::TreasuryVaultProvider::get_vault_operator(vault_id);
				let mut raised = T::Balance::zero();
				for account in FundersByVaultId::<T>::get(vault_id) {
					if raised >= activated_cap {
						break;
					}

					let Some(state) = FunderStateByVaultAndAccount::<T>::get(vault_id, &account)
					else {
						continue;
					};
					let tranche = state.bondable_tranche_for_frame(frame_id);
					if tranche.is_zero() {
						continue;
					}

					let remaining = activated_cap.saturating_sub(raised);
					let to_bond = tranche.min(remaining);
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
				// Bond principal into this frame's pool up to the vault target.
				for (account_id, amount) in pool.bond_holders {
					let is_operator = vault_operator.as_ref() == Some(&account_id);
					let operator_was_zero = if is_operator {
						FunderStateByVaultAndAccount::<T>::get(vault_id, &account_id)
							.map(|state| state.bonded_principal.is_zero())
							.unwrap_or(true)
					} else {
						false
					};
					// Update long-lived bonded principal.
					FunderStateByVaultAndAccount::<T>::mutate(vault_id, &account_id, |e| {
						let mut s = e.take().unwrap_or_default();
						s.bonded_principal.saturating_accrue(amount);
						*e = Some(s);
					});
					if is_operator && operator_was_zero && !amount.is_zero() {
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

		pub(crate) fn pay_operational_rewards() {
			let payouts = T::OperationalRewardsProvider::pending_rewards();
			if payouts.is_empty() {
				return;
			}
			let treasury_reserves_account = Self::get_treasury_reserves_account();
			Self::ensure_account_provider(&treasury_reserves_account);
			let mut available = T::Currency::balance(&treasury_reserves_account);
			for payout in payouts {
				if payout.amount.is_zero() {
					continue;
				}
				if payout.amount > available {
					break;
				}
				if let Err(e) = T::Currency::transfer(
					&treasury_reserves_account,
					&payout.payout_account,
					payout.amount,
					Preservation::Preserve,
				) {
					log::error!(
						"Failed to pay operational reward {:?} to {:?}: {:?}",
						payout.reward_kind,
						payout.payout_account,
						e
					);
					continue;
				}
				available.saturating_reduce(payout.amount);
				T::OperationalRewardsProvider::mark_reward_paid(&payout);
			}
		}

		fn frame_transition_base_weight() -> Weight {
			T::WeightInfo::on_frame_transition().saturating_add(
				T::OperationalAccountsHook::treasury_pool_participated_weight()
					.saturating_mul(u64::from(T::MaxVaultsPerPool::get())),
			)
		}

		pub(crate) fn release_bonded_principal(frame_id: FrameId) {
			if frame_id < 10 {
				return;
			}
			let release_frame_id = frame_id - 10;
			for (vault_id, fund) in VaultPoolsByFrame::<T>::take(release_frame_id) {
				for (account, amount) in fund.bond_holders {
					let res = FunderStateByVaultAndAccount::<T>::try_mutate(
						vault_id,
						&account,
						|entry| {
							if let Some(state) = entry.as_mut() {
								state.bonded_principal.saturating_reduce(amount);
							}
							Self::reduce_allocation(
								account.clone(),
								vault_id,
								entry,
								release_frame_id,
							)
						},
					);

					if let Err(e) = res {
						Self::deposit_event(Event::<T>::ErrorRefundingTreasuryCapital {
							frame_id: release_frame_id,
							vault_id,
							amount,
							account_id: account.clone(),
							dispatch_error: e,
						});
					}
				}
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

		fn reduce_allocation(
			account: T::AccountId,
			vault_id: VaultId,
			entry: &mut Option<FunderState<T>>,
			at_frame_id: FrameId,
		) -> DispatchResult {
			if let Some(state) = entry.as_mut() {
				// Decrement long-lived bonded principal as this tranche rolls off.
				Self::update_basis(state, at_frame_id);
				let mut to_release = T::Balance::zero();
				let minimum_principal = state.target_principal.max(state.bonded_principal);
				if state.held_principal > minimum_principal {
					to_release = state.held_principal.saturating_sub(minimum_principal);
					state.held_principal = minimum_principal;
				}

				if state.target_principal.is_zero() &&
					state.bonded_principal.is_zero() &&
					state.held_principal.is_zero()
				{
					*entry = None;
					FundersByVaultId::<T>::mutate(vault_id, |v| v.remove(&account));
				}

				if to_release > T::Balance::zero() {
					Self::release_hold(&account, to_release)?;
					Self::deposit_event(Event::<T>::RefundedTreasuryCapital {
						frame_id: at_frame_id,
						vault_id,
						amount: to_release,
						account_id: account.clone(),
					});
				}
			}
			Ok(())
		}

		fn get_vault_activated_funds_per_slot(vault_id: VaultId) -> T::Balance {
			let activated_securitization =
				T::TreasuryVaultProvider::get_activated_securitization(vault_id);
			activated_securitization / 10u128.into()
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
			let available = T::Currency::balance(&treasury_reserves_account);
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
		/// The current amount of principal the funder would like to spread across 10 frames (the
		/// target)
		#[codec(compact)]
		pub target_principal: T::Balance,
		/// Total principal currently bonded across the rolling 10-frame window.
		#[codec(compact)]
		pub bonded_principal: T::Balance,
		/// Principal currently held by the Currency pallet. This is stored separately from the
		/// target to allow gradual releases when bonded tranches roll off (after a target
		/// reduction)
		#[codec(compact)]
		pub held_principal: T::Balance,
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
		/// The per-frame tranche amount for this specific frame (evenly spread across 10 frames).
		///
		/// Algorithm:
		/// - We interpret `target_principal` as the total principal to be deployed over a rolling
		///   10-frame window.
		/// - `base` is the floor of an even split across 10 frames.
		/// - `rem` is the remainder that cannot be evenly divided.
		/// - We then distribute that remainder deterministically by giving +1 unit of principal to
		///   the earliest `rem` frame offsets in each 10-frame cycle, where the offset is `frame_id
		///   % 10`.
		///
		/// This scheme ensures:
		/// - Over any contiguous 10-frame window, the sum of all `tranche_for_frame` values equals
		///   exactly `target_principal`.
		/// - The distribution of the remainder is stable and predictable, always biasing toward
		///   earlier offsets (0, 1, ..., `rem - 1`) within each cycle, which avoids drift and keeps
		///   the principal deployment as uniform as integer division allows.
		pub fn tranche_for_frame(&self, frame_id: FrameId) -> T::Balance {
			let committed_u128: u128 = self.target_principal.into();
			// Base amount per frame from an even split across 10 frames.
			let base: u128 = committed_u128 / 10u128;
			// Remainder that cannot be evenly divided into 10 equal parts.
			let rem: u128 = committed_u128 % 10u128;
			// Frame offset within the 10-frame cycle.
			let offset: u128 = (frame_id % 10) as u128;
			// Assign one extra unit to the earliest `rem` offsets in the cycle so that
			// the sum over any 10-frame window matches `target_principal`.
			let extra: u128 = if offset < rem { 1 } else { 0 };
			T::Balance::from(base.saturating_add(extra))
		}

		/// Maximum additional principal we should bond for this frame, respecting the target.
		pub fn bondable_tranche_for_frame(&self, frame_id: FrameId) -> T::Balance {
			let desired = self.tranche_for_frame(frame_id);
			let remaining = self.target_principal.saturating_sub(self.bonded_principal);
			desired.min(remaining)
		}
	}
}
