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

/// This pallet allows users to buy whole `1 ARGON` bonds into a Vault's Treasury Pool. Treasury
/// pools serve as instant liquidity for LockedBitcoins. Each purchase becomes a purchase-level
/// bond lot that participates in frame payouts until it is liquidated and later released, and
/// earnings are paid directly instead of compounding back into principal.
///
/// The current treasury pallet used to model a vault contribution as one aggregated held balance
/// per `(vault_id, account_id)`. That worked for a "single rolling funder" model, but it breaks
/// down for a real bond model where:
///
/// - bonds are bought in whole `1 ARGON` units
/// - one account may have multiple separate purchases
/// - earnings should pay out directly instead of compounding
/// - a purchase needs its own start date, frame count, and cumulative earnings
/// - top-`MaxTreasuryContributors` selection needs to work on purchases/lots, not on a single
///   aggregated account balance
/// - a bumped position should stay in any frame snapshot already created for payout, but be out of
///   the next snapshot build and enter release delay
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
/// 2. Bond lots in the locked frame capital based on pro-rata
///
/// Treasury pool imbalances are added to the front of the "Mint" queue. Before minting occurs
/// for bitcoins in the list, any pending Treasury Pools are paid out oldest first. Within the
/// pool, bond lots are paid out at a floored pro-rata. Underfilled-vault remainder is returned
/// to treasury reserves.
///
/// Bitcoins with remaining mint-able argons are added to the end of the mint-queue. Only bitcoins
/// locked the same day as a slot are eligible for instant-liquidity.
///
/// ## Treasury Pool Allocation
/// Each frame's treasury pool can sell whole bonds up to the full argon value of a vault's
/// securitized satoshis (`sats * securitization ratio`).
///
/// ## Profits from Bid Pool
/// Once each bid pool is closed, 20% of the pool is reserved for treasury reserves. (Operational
/// rewards are one use of these reserves.) The remaining funds are distributed pro-rata to each
/// vault's frame treasury pool. Vaults disperse funds to bond lots based on the vault's sharing
/// percent, each lot's stored frame share, and any underfilled-vault remainder is returned to
/// treasury reserves.
///
/// The limitations on bond purchases are:
/// - the maximum number of accepted bond lots in an active vault pool (`MaxTreasuryContributors`)
/// - the minimum whole-bond purchase amount (`MinimumArgonsPerContributor`)
///
/// Terminology note:
/// - a `frame` is the Argon time duration itself
/// - a `bond` is one `1 ARGON` unit
/// - a `bond lot` is one purchase record that contains `N` bonds
/// - a `frame snapshot` is the locked treasury capital snapshot created for a frame by
///   `lock_in_vault_capital(frame_id)`
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec::Vec;
	use argon_primitives::{
		providers::PriceProvider,
		vault::{TreasuryBonusApprovalProof, TreasuryVaultProvider, VaultTreasuryFrameEarnings},
		BlockSealAuthorityId, OnNewSlot, TreasuryPoolProvider, MICROGONS_PER_ARGON,
	};
	use pallet_prelude::argon_primitives::{
		MiningFrameTransitionProvider, OperationalAccountsHook, OperationalRewardsPayer,
	};
	use sp_runtime::{AccountId32, BoundedBTreeMap, FixedU128, Permill, TokenError};
	use tracing::info;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);

	pub type BondLotId = u64;
	pub type Bonds = u32;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config<AccountId = AccountId32>
	where
		<Self as Config>::Balance: Into<u128>,
	{
		/// Type representing the weight of this pallet.
		type WeightInfo: WeightInfo;

		/// The balance type.
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

		/// The hold reason when reserving funds for treasury bond lots.
		type RuntimeHoldReason: From<HoldReason>;

		/// Provider for vault treasury settings and vault-side earnings collection.
		type TreasuryVaultProvider: TreasuryVaultProvider<
			Balance = Self::Balance,
			AccountId = Self::AccountId,
		>;

		/// Provider for turning securitized satoshis into argon value.
		type PriceProvider: PriceProvider<Self::Balance>;

		/// The maximum number of accepted bond lots in a vault's accepted bond-lot list.
		#[pallet::constant]
		type MaxTreasuryContributors: Get<u32>;

		/// The minimum whole-bond purchase amount.
		#[pallet::constant]
		type MinimumArgonsPerContributor: Get<Self::Balance>;

		/// Treasury pallet id retained in metadata for account derivation.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Account that receives mining bid funds before frame distribution.
		type MiningBidPoolAccount: Get<Self::AccountId>;

		/// Account that holds treasury reserves for claims and reserve-funded payouts.
		#[pallet::constant]
		type TreasuryReservesAccount: Get<Self::AccountId>;

		/// Percent of the bid pool reserved for treasury reserves.
		#[pallet::constant]
		type PercentForTreasuryReserves: Get<Percent>;

		/// The maximum number of vaults that can participate in one frame's locked vault capital.
		#[pallet::constant]
		type MaxVaultsPerPool: Get<u32>;

		/// The maximum number of bond lots whose release delay may mature in a single frame.
		#[pallet::constant]
		type MaxPendingUnlocksPerFrame: Get<u32>;

		/// The number of frames a releasing bond lot remains held before release.
		#[pallet::constant]
		type TreasuryExitDelayFrames: Get<FrameId>;

		/// Provider for the current mining frame id.
		type MiningFrameTransitionProvider: MiningFrameTransitionProvider;

		/// Optional hook for operational account state updates.
		type OperationalAccountsHook: OperationalAccountsHook<Self::AccountId, Self::Balance>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Funds held for an active or releasing treasury bond lot.
		ContributedToTreasury,
	}

	/// The vault capital locked for the current frame.
	///
	/// Payout uses this to see which vaults and bond lots are participating in the frame.
	#[pallet::storage]
	pub type CurrentFrameVaultCapital<T: Config> =
		StorageValue<_, FrameVaultCapital<T>, OptionQuery>;

	/// The next bond lot id.
	#[pallet::storage]
	pub type NextBondLotId<T> = StorageValue<_, BondLotId, ValueQuery>;

	/// The stored state for each bond lot.
	#[pallet::storage]
	pub type BondLotById<T: Config> =
		StorageMap<_, Twox64Concat, BondLotId, BondLot<T>, OptionQuery>;

	/// The bond lot ids that belong to an account.
	#[pallet::storage]
	pub type BondLotIdsByAccount<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, BondLotId, (), OptionQuery>;

	/// Exact treasury bond backing reserved for crosschain minting authorities by account.
	///
	/// This is an exact microgon claim, not a mirror of the account's active whole-bond lots.
	/// Bond lots stay coarse because participation only moves in whole bonds, while encumbrance can
	/// remain fractional after a burn. The key invariant is that the non-releasing held balance
	/// always covers this amount.
	#[pallet::storage]
	pub type EncumberedBondMicrogonsByAccount<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, T::Balance, ValueQuery>;

	/// Bond lots to release at the given frame.
	#[pallet::storage]
	pub type PendingBondReleasesByFrame<T: Config> = StorageMap<
		_,
		Twox64Concat,
		FrameId,
		BoundedVec<BondLotId, T::MaxPendingUnlocksPerFrame>,
		ValueQuery,
	>;

	/// The oldest frame that still has bond lots to retry releasing.
	#[pallet::storage]
	pub type PendingBondReleaseRetryCursor<T: Config> = StorageValue<_, FrameId, OptionQuery>;

	/// The accepted bond lots for a vault.
	///
	/// Lots are kept in descending bond order, then lower `bond_lot_id` first for ties.
	#[pallet::storage]
	pub type BondLotsByVault<T: Config> = StorageMap<
		_,
		Twox64Concat,
		VaultId,
		BoundedVec<BondLotSummary, T::MaxTreasuryContributors>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An error occurred while paying frame earnings for a bond lot.
		CouldNotDistributeEarningsToBondLot {
			frame_id: FrameId,
			vault_id: VaultId,
			bond_lot_id: BondLotId,
			account_id: T::AccountId,
			amount: T::Balance,
			dispatch_error: DispatchError,
		},
		/// An error occurred while moving bid-pool funds into treasury reserves.
		CouldNotTransferToTreasuryReserves {
			frame_id: FrameId,
			amount: T::Balance,
			dispatch_error: DispatchError,
		},
		/// Frame earnings were distributed.
		FrameEarningsDistributed {
			frame_id: FrameId,
			bid_pool_distributed: T::Balance,
			treasury_reserves: T::Balance,
			participating_vaults: u32,
		},
		/// The current frame's vault capital was locked in.
		FrameVaultCapitalLocked {
			frame_id: FrameId,
			total_eligible_bonds: u128,
			participating_vaults: u32,
		},
		/// An error occurred while releasing a bond lot.
		CouldNotReleaseBondLot {
			frame_id: FrameId,
			vault_id: VaultId,
			bond_lot_id: BondLotId,
			amount: T::Balance,
			account_id: T::AccountId,
			dispatch_error: DispatchError,
		},
		/// A bond purchase entered a vault's accepted list.
		BondLotPurchased {
			vault_id: VaultId,
			bond_lot_id: BondLotId,
			account_id: T::AccountId,
			bonds: Bonds,
		},
		/// A bond lot was removed from future frames and scheduled for release.
		BondLotReleaseScheduled {
			vault_id: VaultId,
			bond_lot_id: BondLotId,
			account_id: T::AccountId,
			bonds: Bonds,
			release_frame_id: FrameId,
			reason: BondReleaseReason,
		},
		/// A bond lot was released.
		BondLotReleased {
			frame_id: FrameId,
			vault_id: VaultId,
			bond_lot_id: BondLotId,
			account_id: T::AccountId,
			bonds: Bonds,
		},
		/// Encumbered treasury backing was burned and any no-longer-needed fractional hold was
		/// returned.
		EncumberedBondMicrogonsBurned {
			account_id: T::AccountId,
			burned_amount: T::Balance,
			released_amount: T::Balance,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The purchase would not enter the vault's accepted list.
		BondPurchaseRejected,
		/// The vault is not accepting bond purchases.
		VaultNotAcceptingBondPurchases,
		/// The purchase is below the minimum amount.
		BondPurchaseBelowMinimum,
		/// An internal error occurred.
		InternalError,
		/// The vault already has the maximum number of accepted bond lots.
		MaxAcceptedBondLotsExceeded,
		/// Too many bond lot releases are scheduled for the same frame.
		MaxPendingBondReleasesExceeded,
		/// The bond lot could not be found.
		BondLotNotFound,
		/// The caller does not own the bond lot.
		NotBondLotOwner,
		/// The bond lot is already scheduled for release.
		BondLotAlreadyReleasing,
		/// The vault doesn't have enough bitcoin security to support this bond purchase
		BondPurchaseAboveSecurity,
		/// Liquidating this bond lot would take the account below its crosschain-encumbered
		/// treasury backing.
		ActiveBondAmountBelowEncumberedBacking,
		/// The bonus approval was signed for a different vault.
		BonusApprovalWrongVault,
		/// The bonus approval was signed for a different beneficiary.
		BonusApprovalWrongAccount,
		/// The bonus approval already expired.
		BonusApprovalExpired,
		/// The beneficiary already has a bond lot for this vault.
		BonusApprovalExistingBondLot,
		/// The bonus approval signature is invalid or unauthorized.
		InvalidBonusApprovalSignature,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Buy whole `1 ARGON` bonds for a vault.
		///
		/// The purchase either enters the accepted list or fails.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::buy_bonds())]
		pub fn buy_bonds(
			origin: OriginFor<T>,
			vault_id: VaultId,
			bonds: Bonds,
			bonus_approval: Option<TreasuryBonusApprovalProof>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				T::TreasuryVaultProvider::is_vault_open(vault_id),
				Error::<T>::VaultNotAcceptingBondPurchases
			);
			ensure!(bonds >= Self::minimum_purchase_bonds(), Error::<T>::BondPurchaseBelowMinimum);

			let activated_vault_bonds =
				Self::balance_to_bonds(Self::get_vault_securitized_funds_cap(vault_id));
			ensure!(!activated_vault_bonds.is_zero(), Error::<T>::VaultNotAcceptingBondPurchases);

			let current_frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
			let sharing_percent =
				T::TreasuryVaultProvider::get_vault_profit_sharing_percent(vault_id)
					.unwrap_or_default();
			let mut accepted_lots = BondLotsByVault::<T>::get(vault_id);

			let sold_bonds = Self::sum_bonds(&accepted_lots);
			let available_bond_space_now = activated_vault_bonds.saturating_sub(sold_bonds);

			let evicted_summary = if accepted_lots.len() <
				T::MaxTreasuryContributors::get() as usize
			{
				ensure!(bonds <= available_bond_space_now, Error::<T>::BondPurchaseAboveSecurity);
				None
			} else {
				let evicted_summary =
					accepted_lots.pop().ok_or(Error::<T>::BondPurchaseRejected)?;

				ensure!(bonds > evicted_summary.bonds, Error::<T>::BondPurchaseRejected);
				ensure!(
					bonds.saturating_sub(evicted_summary.bonds) <= available_bond_space_now,
					Error::<T>::BondPurchaseAboveSecurity
				);
				Some(evicted_summary)
			};

			let bonus_percent = Self::validate_bonus_approval(
				vault_id,
				&who,
				current_frame_id,
				bonus_approval.as_ref(),
			)?;

			if let Some(evicted_summary) = evicted_summary {
				let evicted_lot = BondLotById::<T>::get(evicted_summary.bond_lot_id)
					.ok_or(Error::<T>::BondLotNotFound)?;
				let release_frame_id = Self::schedule_bond_lot_release(
					evicted_summary.bond_lot_id,
					BondReleaseReason::Bumped,
				)?;

				Self::deposit_event(Event::<T>::BondLotReleaseScheduled {
					vault_id,
					bond_lot_id: evicted_summary.bond_lot_id,
					account_id: evicted_lot.owner,
					bonds: evicted_lot.bonds,
					release_frame_id,
					reason: BondReleaseReason::Bumped,
				});
			}

			let bond_lot_id = Self::next_bond_lot_id()?;
			let purchase_amount = Self::bonds_to_balance(bonds);
			Self::create_hold(&who, purchase_amount)?;

			let insert_index = accepted_lots
				.iter()
				.position(|summary| summary.bonds < bonds)
				.unwrap_or(accepted_lots.len());

			accepted_lots
				.try_insert(insert_index, BondLotSummary { bond_lot_id, bonds })
				.map_err(|_| Error::<T>::MaxAcceptedBondLotsExceeded)?;

			BondLotById::<T>::insert(
				bond_lot_id,
				BondLot {
					owner: who.clone(),
					vault_id,
					bonds,
					sharing_percent,
					bonus_percent,
					created_frame_id: current_frame_id,
					participated_frames: 0,
					last_frame_earnings_frame_id: None,
					last_frame_earnings: None,
					cumulative_earnings: T::Balance::zero(),
					release_frame_id: None,
					release_reason: None,
				},
			);
			BondLotIdsByAccount::<T>::insert(&who, bond_lot_id, ());
			BondLotsByVault::<T>::insert(vault_id, accepted_lots);

			Self::deposit_event(Event::<T>::BondLotPurchased {
				vault_id,
				bond_lot_id,
				account_id: who.clone(),
				bonds,
			});
			Ok(())
		}

		/// Liquidate one full bond lot.
		///
		/// The lot stops participating right away and is released after the delay.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::liquidate_bond_lot())]
		pub fn liquidate_bond_lot(origin: OriginFor<T>, bond_lot_id: BondLotId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let bond_lot = BondLotById::<T>::get(bond_lot_id).ok_or(Error::<T>::BondLotNotFound)?;
			ensure!(bond_lot.owner == who, Error::<T>::NotBondLotOwner);
			ensure!(bond_lot.release_reason.is_none(), Error::<T>::BondLotAlreadyReleasing);
			let remaining_non_releasing_hold = Self::non_releasing_held_bond_amount(&who)?
				.saturating_sub(Self::bonds_to_balance(bond_lot.bonds));
			ensure!(
				remaining_non_releasing_hold >= Self::encumbered_bond_microgons(&who),
				Error::<T>::ActiveBondAmountBelowEncumberedBacking,
			);

			Self::remove_bond_lot_from_vault(bond_lot.vault_id, bond_lot_id);
			let release_frame_id =
				Self::schedule_bond_lot_release(bond_lot_id, BondReleaseReason::UserLiquidation)?;

			Self::deposit_event(Event::<T>::BondLotReleaseScheduled {
				vault_id: bond_lot.vault_id,
				bond_lot_id,
				account_id: who,
				bonds: bond_lot.bonds,
				release_frame_id,
				reason: BondReleaseReason::UserLiquidation,
			});
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn ensure_account_provider(account_id: &T::AccountId) {
			let providers = frame_system::Pallet::<T>::providers(account_id);
			for _ in providers..2 {
				frame_system::Pallet::<T>::inc_providers(account_id);
			}
		}

		pub(crate) fn create_hold(account_id: &T::AccountId, amount: T::Balance) -> DispatchResult {
			if amount.is_zero() {
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
			if amount.is_zero() {
				return Ok(());
			}
			let reason = HoldReason::ContributedToTreasury;
			T::Currency::release(&reason.into(), who, amount, Precision::Exact)?;

			if T::Currency::balance_on_hold(&reason.into(), who).is_zero() {
				frame_system::Pallet::<T>::dec_providers(who)?;
			}
			Ok(())
		}

		fn validate_bonus_approval(
			vault_id: VaultId,
			beneficiary: &T::AccountId,
			current_frame_id: FrameId,
			bonus_approval: Option<&TreasuryBonusApprovalProof>,
		) -> Result<Permill, Error<T>> {
			let Some(bonus_approval) = bonus_approval else {
				return Ok(Permill::zero());
			};

			ensure!(bonus_approval.vault_id == vault_id, Error::<T>::BonusApprovalWrongVault);
			ensure!(
				bonus_approval.beneficiary == *beneficiary,
				Error::<T>::BonusApprovalWrongAccount
			);
			ensure!(
				current_frame_id <= bonus_approval.expires_at_frame,
				Error::<T>::BonusApprovalExpired
			);

			let signed_by_operator = T::TreasuryVaultProvider::get_vault_operator(vault_id)
				.as_ref()
				.is_some_and(|account_id| bonus_approval.verify(account_id));
			let signed_by_delegate = T::TreasuryVaultProvider::get_vault_delegate(vault_id)
				.as_ref()
				.is_some_and(|account_id| bonus_approval.verify(account_id));
			ensure!(
				signed_by_operator || signed_by_delegate,
				Error::<T>::InvalidBonusApprovalSignature
			);

			for (bond_lot_id, ()) in BondLotIdsByAccount::<T>::iter_prefix(beneficiary) {
				let bond_lot =
					BondLotById::<T>::get(bond_lot_id).ok_or(Error::<T>::BondLotNotFound)?;
				if bond_lot.vault_id == vault_id {
					return Err(Error::<T>::BonusApprovalExistingBondLot);
				}
			}

			Ok(T::TreasuryVaultProvider::get_vault_treasury_bonus_profit_sharing(vault_id)
				.unwrap_or_default())
		}

		/// Once the frame is complete, this fn distributes the bid pool to each vault based on
		/// their prorata eligible bonds. Then within each vault, profits are distributed to bond
		/// lots based on the stored frame shares.
		pub(crate) fn distribute_bid_pool(frame_id: FrameId) {
			let Some(frame_capital) = CurrentFrameVaultCapital::<T>::take() else {
				return;
			};
			if frame_capital.frame_id != frame_id {
				CurrentFrameVaultCapital::<T>::put(frame_capital);
				return;
			}

			let bid_pool_account = T::MiningBidPoolAccount::get();
			Self::ensure_account_provider(&bid_pool_account);
			let mut total_bid_pool_amount = T::Currency::balance(&bid_pool_account);

			let initial_reserves_amount =
				T::PercentForTreasuryReserves::get().mul_ceil(total_bid_pool_amount);
			let reserves_account = T::TreasuryReservesAccount::get();
			Self::ensure_account_provider(&reserves_account);

			let mut total_treasury_reserves = T::Balance::zero();
			if !initial_reserves_amount.is_zero() {
				if let Err(e) = T::Currency::transfer(
					&bid_pool_account,
					&reserves_account,
					initial_reserves_amount,
					Preservation::Expendable,
				) {
					Self::deposit_event(Event::<T>::CouldNotTransferToTreasuryReserves {
						frame_id,
						amount: initial_reserves_amount,
						dispatch_error: e,
					});
				} else {
					total_treasury_reserves = initial_reserves_amount;
					total_bid_pool_amount.saturating_reduce(initial_reserves_amount);
				}
			}

			let frame_total_eligible_bonds = frame_capital
				.vaults
				.values()
				.fold(0u128, |acc, vault| acc.saturating_add(vault.eligible_bonds as u128));

			let mut remaining_bid_pool = total_bid_pool_amount;
			let mut treasury_refund_total = T::Balance::zero();

			for (vault_id, vault_capital) in frame_capital.vaults.iter() {
				if frame_total_eligible_bonds.is_zero() {
					continue;
				}

				let Some(vault_account_id) =
					T::TreasuryVaultProvider::get_vault_operator(*vault_id)
				else {
					continue;
				};

				let gross_vault_earnings = Perbill::from_rational(
					vault_capital.eligible_bonds as u128,
					frame_total_eligible_bonds,
				)
				.mul_floor(total_bid_pool_amount);
				remaining_bid_pool.saturating_reduce(gross_vault_earnings);

				let mut gross_lot_yield_total = T::Balance::zero();
				let mut earnings_for_vault = T::Balance::zero();
				let mut capital_contributed_by_vault = T::Balance::zero();

				for allocation in vault_capital.bond_lot_allocations.iter() {
					let Some(bond_lot) = BondLotById::<T>::get(allocation.bond_lot_id) else {
						continue;
					};

					let gross_lot_yield =
						allocation.prorata.saturating_mul_int(gross_vault_earnings);
					gross_lot_yield_total.saturating_accrue(gross_lot_yield);

					let bonder_percent = Permill::from_parts(
						bond_lot
							.sharing_percent
							.deconstruct()
							.saturating_add(bond_lot.bonus_percent.deconstruct()),
					);
					let mut paid_payout = bonder_percent.mul_floor(gross_lot_yield);

					if bond_lot.owner == vault_account_id {
						earnings_for_vault.saturating_accrue(gross_lot_yield);
						capital_contributed_by_vault
							.saturating_accrue(Self::bonds_to_balance(bond_lot.bonds));
					} else {
						earnings_for_vault
							.saturating_accrue(gross_lot_yield.saturating_sub(paid_payout));
						if !paid_payout.is_zero() &&
							let Err(e) = T::Currency::transfer(
								&bid_pool_account,
								&bond_lot.owner,
								paid_payout,
								Preservation::Expendable,
							) {
							Self::deposit_event(Event::<T>::CouldNotDistributeEarningsToBondLot {
								frame_id,
								vault_id: *vault_id,
								bond_lot_id: allocation.bond_lot_id,
								account_id: bond_lot.owner,
								amount: paid_payout,
								dispatch_error: e,
							});
							treasury_refund_total.saturating_accrue(paid_payout);
							paid_payout = T::Balance::zero();
						}
					}

					BondLotById::<T>::mutate_exists(allocation.bond_lot_id, |maybe_bond_lot| {
						let Some(bond_lot) = maybe_bond_lot.as_mut() else {
							return;
						};
						bond_lot.participated_frames =
							bond_lot.participated_frames.saturating_add(1);
						bond_lot.last_frame_earnings_frame_id = Some(frame_id);
						bond_lot.last_frame_earnings = Some(paid_payout);
						bond_lot.cumulative_earnings.saturating_accrue(paid_payout);
					});
				}

				treasury_refund_total
					.saturating_accrue(gross_vault_earnings.saturating_sub(gross_lot_yield_total));

				T::TreasuryVaultProvider::record_vault_frame_earnings(
					&bid_pool_account,
					VaultTreasuryFrameEarnings {
						vault_id: *vault_id,
						vault_operator_account_id: vault_account_id,
						frame_id,
						earnings_for_vault,
						earnings: gross_vault_earnings,
						capital_contributed: Self::bonds_to_balance(vault_capital.eligible_bonds),
						capital_contributed_by_vault,
					},
				);
			}

			if !treasury_refund_total.is_zero() {
				if let Err(e) = T::Currency::transfer(
					&bid_pool_account,
					&reserves_account,
					treasury_refund_total,
					Preservation::Expendable,
				) {
					Self::deposit_event(Event::<T>::CouldNotTransferToTreasuryReserves {
						frame_id,
						amount: treasury_refund_total,
						dispatch_error: e,
					});
				} else {
					total_treasury_reserves.saturating_accrue(treasury_refund_total);
				}
			}

			let participating_vaults = frame_capital.vaults.len() as u32;

			Self::deposit_event(Event::<T>::FrameEarningsDistributed {
				frame_id,
				bid_pool_distributed: total_bid_pool_amount.saturating_sub(remaining_bid_pool),
				treasury_reserves: total_treasury_reserves,
				participating_vaults,
			});
		}

		/// Locks in the vault capital for the next frame based on vault securitized satoshis (`sats
		/// * securitization ratio`) and accepted bond lots.
		pub(crate) fn lock_in_vault_capital(frame_id: FrameId) {
			let mut vault_candidates = Vec::new();
			let max_vaults = T::MaxVaultsPerPool::get() as usize;

			for (vault_id, accepted_lots) in BondLotsByVault::<T>::iter() {
				if accepted_lots.is_empty() {
					continue;
				}

				let sold_bonds = Self::sum_bonds(&accepted_lots);
				let activated_bitcoin_security_as_bonds =
					Self::balance_to_bonds(Self::get_vault_securitized_funds_cap(vault_id));
				let payout_denominator = activated_bitcoin_security_as_bonds.max(sold_bonds);
				if payout_denominator.is_zero() {
					continue;
				}

				let mut bond_lot_allocations = BoundedVec::default();
				for entry in accepted_lots {
					let prorata =
						FixedU128::from_rational(entry.bonds as u128, payout_denominator as u128);
					if bond_lot_allocations
						.try_push(BondLotAllocation { bond_lot_id: entry.bond_lot_id, prorata })
						.is_err()
					{
						break;
					}
				}

				let eligible_bonds = activated_bitcoin_security_as_bonds.min(sold_bonds);
				vault_candidates.push((
					vault_id,
					eligible_bonds,
					VaultCapital { bond_lot_allocations, eligible_bonds },
				));
			}

			vault_candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
			vault_candidates.truncate(max_vaults);

			let mut vaults = BoundedBTreeMap::new();
			let mut total_eligible_bonds = 0u128;

			for (vault_id, eligible_bonds, vault_capital) in vault_candidates {
				total_eligible_bonds = total_eligible_bonds.saturating_add(eligible_bonds as u128);
				let _ = vaults.try_insert(vault_id, vault_capital);

				if let Some(operator) = T::TreasuryVaultProvider::get_vault_operator(vault_id) &&
					let Some(operator_amount) =
						Self::active_account_bond_amount(vault_id, &operator).unwrap_or_default() &&
					!operator_amount.is_zero()
				{
					T::OperationalAccountsHook::treasury_pool_participated(
						&operator,
						operator_amount,
					);
				}
			}

			let participating_vaults = vaults.len() as u32;
			CurrentFrameVaultCapital::<T>::put(FrameVaultCapital { frame_id, vaults });

			Self::deposit_event(Event::<T>::FrameVaultCapitalLocked {
				frame_id,
				total_eligible_bonds,
				participating_vaults,
			});
		}

		/// Runs the treasury frame transition in the current pallet order: release, distribute,
		/// then lock in next-frame capital.
		pub(crate) fn run_frame_transition(frame_id: FrameId) {
			if frame_id == 0 {
				return;
			}

			let payout_frame = frame_id - 1;
			info!("Starting treasury bond frame {frame_id}. Distributing frame {payout_frame}.");
			Self::release_pending_bond_lots(frame_id);
			Self::distribute_bid_pool(payout_frame);
			Self::lock_in_vault_capital(frame_id);
		}

		/// Releases bond lots whose release delay has matured.
		pub(crate) fn release_pending_bond_lots(frame_id: FrameId) {
			let start_frame =
				PendingBondReleaseRetryCursor::<T>::take().unwrap_or(frame_id).min(frame_id);
			let mut next_retry_frame = None;

			for due_frame in start_frame..=frame_id {
				let pending_releases = PendingBondReleasesByFrame::<T>::take(due_frame);
				if pending_releases.is_empty() {
					continue;
				}

				let mut failed_releases = BoundedVec::default();

				for bond_lot_id in pending_releases {
					let Some(bond_lot) = BondLotById::<T>::get(bond_lot_id) else {
						continue;
					};
					let release_amount = Self::bonds_to_balance(bond_lot.bonds);

					if let Err(e) = Self::release_hold(&bond_lot.owner, release_amount) {
						let _ = failed_releases.try_push(bond_lot_id);
						if next_retry_frame.is_none() {
							next_retry_frame = Some(due_frame);
						}
						Self::deposit_event(Event::<T>::CouldNotReleaseBondLot {
							frame_id: due_frame,
							vault_id: bond_lot.vault_id,
							bond_lot_id,
							amount: release_amount,
							account_id: bond_lot.owner,
							dispatch_error: e,
						});
						continue;
					}

					BondLotIdsByAccount::<T>::remove(&bond_lot.owner, bond_lot_id);
					BondLotById::<T>::remove(bond_lot_id);

					Self::deposit_event(Event::<T>::BondLotReleased {
						frame_id: due_frame,
						vault_id: bond_lot.vault_id,
						bond_lot_id,
						account_id: bond_lot.owner,
						bonds: bond_lot.bonds,
					});
				}

				if !failed_releases.is_empty() {
					PendingBondReleasesByFrame::<T>::insert(due_frame, failed_releases);
				}
			}

			if let Some(retry_frame) = next_retry_frame {
				PendingBondReleaseRetryCursor::<T>::put(retry_frame);
			}
		}

		fn next_bond_lot_id() -> Result<BondLotId, Error<T>> {
			let next = NextBondLotId::<T>::get();
			let updated = next.checked_add(1).ok_or(Error::<T>::InternalError)?;
			NextBondLotId::<T>::put(updated);
			Ok(next)
		}

		fn minimum_purchase_bonds() -> Bonds {
			let minimum = T::MinimumArgonsPerContributor::get().into();
			let minimum_bonds = minimum.div_ceil(MICROGONS_PER_ARGON).max(1);
			minimum_bonds.min(Bonds::MAX as u128) as Bonds
		}

		fn bonds_to_balance(bonds: Bonds) -> T::Balance {
			(bonds as u128).saturating_mul(MICROGONS_PER_ARGON).into()
		}

		pub(crate) fn balance_to_bonds(balance: T::Balance) -> Bonds {
			let bonds = balance.into() / MICROGONS_PER_ARGON;
			bonds.min(Bonds::MAX as u128) as Bonds
		}

		pub(crate) fn get_vault_securitized_funds_cap(vault_id: VaultId) -> T::Balance {
			let securitized_satoshis = T::TreasuryVaultProvider::get_securitized_satoshis(vault_id);
			T::PriceProvider::get_btc_price_in_market_microgons(securitized_satoshis)
				.unwrap_or_default()
		}

		fn sum_bonds(summaries: &BoundedVec<BondLotSummary, T::MaxTreasuryContributors>) -> Bonds {
			summaries
				.iter()
				.fold(0u128, |acc, summary| acc.saturating_add(summary.bonds as u128))
				.min(Bonds::MAX as u128) as Bonds
		}

		fn active_account_bond_amount(
			vault_id: VaultId,
			account_id: &T::AccountId,
		) -> Result<Option<T::Balance>, Error<T>> {
			let mut total = T::Balance::zero();
			for summary in BondLotsByVault::<T>::get(vault_id) {
				let bond_lot = BondLotById::<T>::get(summary.bond_lot_id)
					.ok_or(Error::<T>::BondLotNotFound)?;
				if bond_lot.owner == *account_id {
					total.saturating_accrue(Self::bonds_to_balance(bond_lot.bonds));
				}
			}

			Ok((!total.is_zero()).then_some(total))
		}

		fn active_non_releasing_account_bond_amount(
			account_id: &T::AccountId,
		) -> Result<Option<T::Balance>, Error<T>> {
			let mut total = T::Balance::zero();
			for (bond_lot_id, ()) in BondLotIdsByAccount::<T>::iter_prefix(account_id) {
				let bond_lot =
					BondLotById::<T>::get(bond_lot_id).ok_or(Error::<T>::BondLotNotFound)?;
				if bond_lot.release_reason.is_some() {
					continue;
				}

				total.saturating_accrue(Self::bonds_to_balance(bond_lot.bonds));
			}

			Ok((!total.is_zero()).then_some(total))
		}

		fn non_releasing_held_bond_amount(
			account_id: &T::AccountId,
		) -> Result<T::Balance, Error<T>> {
			let mut held_balance =
				T::Currency::balance_on_hold(&HoldReason::ContributedToTreasury.into(), account_id);

			for (bond_lot_id, ()) in BondLotIdsByAccount::<T>::iter_prefix(account_id) {
				let bond_lot =
					BondLotById::<T>::get(bond_lot_id).ok_or(Error::<T>::BondLotNotFound)?;
				if bond_lot.release_reason.is_none() {
					continue;
				}

				held_balance = held_balance.saturating_sub(Self::bonds_to_balance(bond_lot.bonds));
			}

			Ok(held_balance)
		}

		pub(crate) fn encumbered_bond_microgons(account_id: &T::AccountId) -> T::Balance {
			EncumberedBondMicrogonsByAccount::<T>::get(account_id)
		}

		fn remove_bond_lot_from_vault(vault_id: VaultId, bond_lot_id: BondLotId) {
			BondLotsByVault::<T>::mutate_exists(vault_id, |maybe_summaries| {
				let Some(summaries) = maybe_summaries.as_mut() else {
					return;
				};

				if let Some(index) =
					summaries.iter().position(|summary| summary.bond_lot_id == bond_lot_id)
				{
					summaries.remove(index);
				}

				if summaries.is_empty() {
					*maybe_summaries = None;
				}
			});
		}

		fn schedule_bond_lot_release(
			bond_lot_id: BondLotId,
			reason: BondReleaseReason,
		) -> Result<FrameId, DispatchError> {
			let release_frame_id = T::MiningFrameTransitionProvider::get_current_frame_id()
				.saturating_add(T::TreasuryExitDelayFrames::get());

			PendingBondReleasesByFrame::<T>::try_mutate(release_frame_id, |pending| {
				if pending.contains(&bond_lot_id) {
					return Ok::<(), Error<T>>(());
				}

				pending
					.try_push(bond_lot_id)
					.map_err(|_| Error::<T>::MaxPendingBondReleasesExceeded)?;
				Ok::<(), Error<T>>(())
			})?;

			BondLotById::<T>::try_mutate_exists(bond_lot_id, |maybe_bond_lot| -> DispatchResult {
				let bond_lot = maybe_bond_lot.as_mut().ok_or(Error::<T>::BondLotNotFound)?;
				if bond_lot.release_reason.is_some() {
					return Err(Error::<T>::BondLotAlreadyReleasing.into());
				}
				bond_lot.release_frame_id = Some(release_frame_id);
				bond_lot.release_reason = Some(reason);
				Ok(())
			})?;

			Ok(release_frame_id)
		}
	}

	impl<T: Config> OperationalRewardsPayer<T::AccountId, T::Balance> for Pallet<T> {
		fn claim_reward_weight() -> Weight {
			T::WeightInfo::claim_reward()
		}

		fn claim_reward(account_id: &T::AccountId, amount: T::Balance) -> DispatchResult {
			if amount.is_zero() {
				return Ok(());
			}
			let treasury_reserves_account = T::TreasuryReservesAccount::get();
			Self::ensure_account_provider(&treasury_reserves_account);
			let available = T::Currency::reducible_balance(
				&treasury_reserves_account,
				Preservation::Preserve,
				Fortitude::Polite,
			);
			ensure!(amount <= available, TokenError::FundsUnavailable);

			T::Currency::transfer(
				&treasury_reserves_account,
				account_id,
				amount,
				Preservation::Preserve,
			)?;
			Ok(())
		}
	}

	impl<T: Config> OnNewSlot<T::AccountId> for Pallet<T> {
		type Key = BlockSealAuthorityId;

		fn on_frame_start(frame_id: FrameId) {
			Self::run_frame_transition(frame_id);
		}

		fn on_frame_start_weight(_frame_id: FrameId) -> Weight {
			T::WeightInfo::on_frame_transition().saturating_add(
				T::OperationalAccountsHook::treasury_pool_participated_weight()
					.saturating_mul(u64::from(T::MaxVaultsPerPool::get())),
			)
		}
	}

	impl<T: Config> TreasuryPoolProvider<T::AccountId> for Pallet<T> {
		type Weights = ProviderWeightAdapter<T>;
		type Balance = T::Balance;

		fn has_bond_participation(vault_id: VaultId, account_id: &T::AccountId) -> bool {
			BondLotsByVault::<T>::get(vault_id).into_iter().any(|summary| {
				BondLotById::<T>::get(summary.bond_lot_id)
					.map(|bond_lot| {
						bond_lot.owner == *account_id && bond_lot.release_reason.is_none()
					})
					.unwrap_or(false)
			})
		}

		fn encumber_bond_microgons(
			account_id: &T::AccountId,
			microgon_amount: Self::Balance,
		) -> DispatchResult {
			if microgon_amount.is_zero() {
				return Ok(());
			}

			let next_encumbered = Self::encumbered_bond_microgons(account_id)
				.checked_add(&microgon_amount)
				.ok_or(Error::<T>::InternalError)?;
			ensure!(
				Self::non_releasing_held_bond_amount(account_id)? >= next_encumbered,
				Error::<T>::ActiveBondAmountBelowEncumberedBacking,
			);

			EncumberedBondMicrogonsByAccount::<T>::mutate(account_id, |encumbered| {
				encumbered.saturating_accrue(microgon_amount);
			});
			Ok(())
		}

		fn release_encumbered_bond_microgons(
			account_id: &T::AccountId,
			microgon_amount: Self::Balance,
		) -> DispatchResult {
			if microgon_amount.is_zero() {
				return Ok(());
			}

			let current_encumbered = Self::encumbered_bond_microgons(account_id);
			ensure!(
				current_encumbered >= microgon_amount,
				Error::<T>::ActiveBondAmountBelowEncumberedBacking,
			);
			let remaining_encumbered = current_encumbered.saturating_sub(microgon_amount);
			EncumberedBondMicrogonsByAccount::<T>::insert(account_id, remaining_encumbered);
			let current_active =
				Self::active_non_releasing_account_bond_amount(account_id)?.unwrap_or_default();
			let current_hold = Self::non_releasing_held_bond_amount(account_id)?;
			let required_hold = current_active.max(remaining_encumbered);
			let released_amount = current_hold.saturating_sub(required_hold);
			if !released_amount.is_zero() {
				Self::release_hold(account_id, released_amount)?;
			}
			Ok(())
		}

		fn burn_encumbered_bond_microgons(
			account_id: &T::AccountId,
			microgon_amount: Self::Balance,
		) -> DispatchResult {
			if microgon_amount.is_zero() {
				return Ok(());
			}

			ensure!(
				Self::encumbered_bond_microgons(account_id) >= microgon_amount,
				Error::<T>::ActiveBondAmountBelowEncumberedBacking,
			);
			let encumbered_after_burn =
				Self::encumbered_bond_microgons(account_id).saturating_sub(microgon_amount);
			T::Currency::burn_held(
				&HoldReason::ContributedToTreasury.into(),
				account_id,
				microgon_amount,
				Precision::Exact,
				Fortitude::Force,
			)
			.map_err(|_| Error::<T>::InternalError)?;
			if T::Currency::balance_on_hold(&HoldReason::ContributedToTreasury.into(), account_id)
				.is_zero()
			{
				let _ = frame_system::Pallet::<T>::dec_providers(account_id);
			}
			EncumberedBondMicrogonsByAccount::<T>::insert(account_id, encumbered_after_burn);

			let held_microgons_after_burn = Self::non_releasing_held_bond_amount(account_id)?;
			ensure!(held_microgons_after_burn >= encumbered_after_burn, Error::<T>::InternalError,);

			let active_bonds_before_trim = Self::balance_to_bonds(
				Self::active_non_releasing_account_bond_amount(account_id)?.unwrap_or_default(),
			);
			let target_active_bonds = Self::balance_to_bonds(held_microgons_after_burn);
			let mut remaining_bonds_to_trim =
				active_bonds_before_trim.saturating_sub(target_active_bonds);

			if remaining_bonds_to_trim != 0 {
				let mut bond_lot_ids = BondLotIdsByAccount::<T>::iter_prefix(account_id)
					.map(|(bond_lot_id, ())| bond_lot_id)
					.collect::<Vec<_>>();
				bond_lot_ids.sort_unstable_by(|left, right| right.cmp(left));

				for bond_lot_id in bond_lot_ids {
					if remaining_bonds_to_trim == 0 {
						break;
					}

					let Some(bond_lot) = BondLotById::<T>::get(bond_lot_id) else {
						continue;
					};
					if bond_lot.release_reason.is_some() || bond_lot.bonds == 0 {
						continue;
					}

					let removed_bonds = bond_lot.bonds.min(remaining_bonds_to_trim);
					let remaining_bonds = bond_lot.bonds.saturating_sub(removed_bonds);
					remaining_bonds_to_trim = remaining_bonds_to_trim.saturating_sub(removed_bonds);

					if remaining_bonds == 0 {
						BondLotById::<T>::remove(bond_lot_id);
						BondLotIdsByAccount::<T>::remove(account_id, bond_lot_id);
						Self::remove_bond_lot_from_vault(bond_lot.vault_id, bond_lot_id);
					} else {
						BondLotById::<T>::mutate_exists(bond_lot_id, |maybe_bond_lot| {
							let Some(bond_lot) = maybe_bond_lot.as_mut() else {
								return;
							};
							bond_lot.bonds = remaining_bonds;
						});
						BondLotsByVault::<T>::mutate_exists(bond_lot.vault_id, |maybe_summaries| {
							let Some(summaries) = maybe_summaries.as_mut() else {
								return;
							};

							if let Some(summary) = summaries
								.iter_mut()
								.find(|summary| summary.bond_lot_id == bond_lot_id)
							{
								summary.bonds = remaining_bonds;
								summaries.sort_by(|left, right| {
									right
										.bonds
										.cmp(&left.bonds)
										.then_with(|| left.bond_lot_id.cmp(&right.bond_lot_id))
								});
							}
						});
					}
				}
			}

			ensure!(remaining_bonds_to_trim == 0, Error::<T>::InternalError);

			let target_active_hold = Self::bonds_to_balance(target_active_bonds);
			let required_hold = target_active_hold.max(encumbered_after_burn);
			let released_amount = held_microgons_after_burn.saturating_sub(required_hold);
			if !released_amount.is_zero() {
				Self::release_hold(account_id, released_amount)?;
			}
			Self::deposit_event(Event::<T>::EncumberedBondMicrogonsBurned {
				account_id: account_id.clone(),
				burned_amount: microgon_amount,
				released_amount,
			});
			Ok(())
		}
	}

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
	)]
	pub enum BondReleaseReason {
		/// The owner requested full-lot liquidation.
		UserLiquidation,
		/// The lot was bumped out by a later accepted purchase.
		Bumped,
		/// The vault closed and the lot was forced into release.
		VaultClosed,
	}

	/// One purchase of `N` whole-argon bonds for one vault.
	#[derive(
		Encode, Decode, Clone, PartialEqNoBound, Eq, DebugNoBound, TypeInfo, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct BondLot<T: Config> {
		/// The account that owns this purchase lot.
		pub owner: T::AccountId,
		/// The vault this purchase belongs to.
		#[codec(compact)]
		pub vault_id: VaultId,
		/// The number of bonds in this lot. `1 ARGON = 1 bond`.
		#[codec(compact)]
		pub bonds: Bonds,
		/// The immutable bonder-side percent of lot yield shared to the bond holder.
		#[codec(compact)]
		pub sharing_percent: Permill,
		/// The immutable bonus bonder-side percent of lot yield shared to the bond holder.
		#[codec(compact)]
		pub bonus_percent: Permill,
		/// The frame when this lot was purchased.
		#[codec(compact)]
		pub created_frame_id: FrameId,
		/// How many earning frames this lot has actually been in.
		#[codec(compact)]
		pub participated_frames: u32,
		/// The frame where `last_frame_earnings` was recorded.
		pub last_frame_earnings_frame_id: Option<FrameId>,
		/// The direct earnings this lot received in its most recent paid frame.
		pub last_frame_earnings: Option<T::Balance>,
		/// The cumulative direct earnings paid to this lot.
		#[codec(compact)]
		pub cumulative_earnings: T::Balance,
		/// The frame when the release delay finishes, if this lot is releasing.
		pub release_frame_id: Option<FrameId>,
		/// Why this lot entered release, if it is releasing.
		pub release_reason: Option<BondReleaseReason>,
	}

	/// The hot-path accepted-lot entry stored on a vault.
	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub struct BondLotSummary {
		/// The accepted lot id.
		#[codec(compact)]
		pub bond_lot_id: BondLotId,
		/// The number of bonds in the accepted lot.
		#[codec(compact)]
		pub bonds: Bonds,
	}

	/// A lot's stored frame allocation.
	#[derive(Encode, Decode, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub struct BondLotAllocation {
		/// The lot participating in this frame snapshot.
		#[codec(compact)]
		pub bond_lot_id: BondLotId,
		/// This lot's stored frame share:
		/// `lot_bonds / max(activated_bitcoin_security_bonds, sold_bonds)`.
		pub prorata: FixedU128,
	}

	/// One vault's locked capital state for a frame.
	#[derive(Encode, Decode, PartialEqNoBound, DebugNoBound, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct VaultCapital<T: Config> {
		/// The lots that share this vault's frame earnings.
		pub bond_lot_allocations: BoundedVec<BondLotAllocation, T::MaxTreasuryContributors>,
		/// The cross-vault frame weight:
		/// `min(activated_bitcoin_security_bonds, sold_bonds)`.
		#[codec(compact)]
		pub eligible_bonds: Bonds,
	}

	/// The frame-wide locked capital object.
	#[derive(Encode, Decode, PartialEqNoBound, DebugNoBound, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct FrameVaultCapital<T: Config> {
		/// The frame this locked capital object belongs to.
		#[codec(compact)]
		pub frame_id: FrameId,
		/// The per-vault frame capital snapshot keyed by vault id.
		pub vaults: BoundedBTreeMap<VaultId, VaultCapital<T>, T::MaxVaultsPerPool>,
	}
}
