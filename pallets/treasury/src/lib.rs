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
/// - frame earnings belong to each purchased bond lot, not an aggregated account balance
///
/// ## Treasury Pool Allocation
/// A vault can accept whole regular bonds up to its raw Argon securitization. Flexible operator
/// bonds yield their admission capacity to regular bonds. Displaced flexible bonds do not earn
/// after the current frame; regular bonds continue earning even if securitization later falls.
///
/// ## Profits from Bid Pool
/// Once each bid pool is closed, 20% is reserved for treasury reserves, 15% is paid to Argonot
/// Stakes, 6% reserved for future mining-operator rewards is burned, and 3% reserved for Bitcoin
/// Liquids is burned in this release. Argon bonds may earn up to 5% and vaults may earn up to 51%
/// based on their coverage of the configured Bitcoin target. Target shortfall and rounding are
/// burned. Bond earnings are paid directly; vault earnings enter the vault collection flow.
///
/// The limitations on bond purchases are:
/// - the maximum number of live Argon bond lots while frame payouts are direct
/// - the maximum number of active Argonot bond lots (`MaxActiveArgonotBondLots`)
/// - the maximum active Argonot bonds as a percent of ownership circulation
///   (`MaxArgonotBondedPercentOfCirculation`)
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
	use alloc::{collections::BTreeMap, vec::Vec};
	use argon_primitives::{
		providers::PriceProviderWeightInfo,
		vault::{
			TreasuryBonusApprovalProof, TreasuryVaultProvider, TreasuryVaultProviderWeightInfo,
			VaultSecuritization, VaultTreasuryFrameEarnings,
		},
		BitcoinMintedProvider, BlockSealAuthorityId, BurnEventHandler, OnNewSlot, PriceProvider,
		TreasuryPoolProvider, MICROGONS_PER_ARGON,
	};
	use pallet_prelude::argon_primitives::{
		MiningFrameTransitionProvider, OperationalAccountsHook, OperationalRewardsPayer,
	};
	use sp_runtime::{
		AccountId32, ArithmeticError, BoundedBTreeMap, FixedU128, Permill, TokenError,
	};
	use tracing::info;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(9);
	/// Maximum rewarded Argonot backing is worth twice the vault's Argon securitization.
	const ARGONOT_SECURITIZATION_MULTIPLIER: u32 = 2;

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

		/// The currency representing ownership tokens (argonots).
		type OwnershipCurrency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>;

		/// The hold reason when reserving funds for treasury bond lots.
		type RuntimeHoldReason: From<HoldReason>;

		/// Provider for vault treasury settings and vault-side earnings collection.
		type TreasuryVaultProvider: TreasuryVaultProvider<
			Balance = Self::Balance,
			AccountId = Self::AccountId,
		>;

		/// Provider for Bitcoin-minted Argons that have not been explicitly repaid.
		type BitcoinMintedProvider: BitcoinMintedProvider<Self::Balance>;
		/// Market prices used to value locked Bitcoin and committed Argonots at frame start.
		type PriceProvider: PriceProvider<Self::Balance>;
		/// Records reward allocations burned from circulation.
		type BurnEventHandler: BurnEventHandler<Self::Balance>;

		/// The minimum whole-bond purchase amount.
		#[pallet::constant]
		type MinimumArgonsPerContributor: Get<Self::Balance>;

		/// The maximum number of active Argonot bond lots.
		#[pallet::constant]
		type MaxActiveArgonotBondLots: Get<u32>;

		/// Maximum live Argon bond lots while direct frame payouts iterate every lot.
		#[pallet::constant]
		type MaxArgonBondLots: Get<u32>;

		/// The maximum percent of ownership-token circulation that can be bonded.
		#[pallet::constant]
		type MaxArgonotBondedPercentOfCirculation: Get<Percent>;

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

		/// Percent of the full bid pool paid to Argonot Stakes.
		#[pallet::constant]
		type PercentForStakePool: Get<Percent>;

		/// Percent reserved for future mining-operator rewards and burned in this release.
		#[pallet::constant]
		type PercentForMiningOperatorPool: Get<Percent>;

		/// Percent reserved for Bitcoin Liquids and burned in this release.
		#[pallet::constant]
		type PercentForBitcoinLiquidPool: Get<Percent>;

		/// Maximum percent paid to Argon bonds when the network target is filled.
		#[pallet::constant]
		type PercentForArgonBondPool: Get<Percent>;

		/// Maximum percent paid to vaults when the network target is filled.
		#[pallet::constant]
		type PercentForVaultPool: Get<Percent>;

		/// Default percent of maximum Bitcoin-mintable Argons targeted by the network.
		#[pallet::constant]
		type DefaultTargetBitcoinPercent: Get<Percent>;

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
	/// Payout uses this for the network bond total and participating vault positions.
	#[pallet::storage]
	pub type CurrentFrameVaultCapital<T: Config> =
		StorageValue<_, FrameVaultCapital<T>, OptionQuery>;

	/// The Argonot bond participants locked for the current frame.
	///
	/// Payout uses this to see which Argonot bond lots are participating in the frame.
	#[pallet::storage]
	pub type CurrentFrameArgonotBondParticipants<T: Config> =
		StorageValue<_, FrameArgonotBondParticipants<T>, OptionQuery>;

	/// Configurable percent of maximum Bitcoin-mintable Argons targeted by the network.
	#[pallet::storage]
	#[pallet::getter(fn target_bitcoin_percent)]
	pub type TargetBitcoinPercent<T: Config> =
		StorageValue<_, Percent, ValueQuery, T::DefaultTargetBitcoinPercent>;

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

	/// Live Argon bond lot ids associated with a vault, including lots awaiting release. Direct
	/// frame payouts iterate this admission-bounded index.
	#[pallet::storage]
	pub type BondLotIdsByVault<T: Config> =
		StorageDoubleMap<_, Twox64Concat, VaultId, Twox64Concat, BondLotId, (), OptionQuery>;

	/// Live Argon bond lots, including those awaiting release; stakes have a separate admission
	/// limit.
	#[pallet::storage]
	pub type TotalArgonBondLots<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	pub type LastBonusApprovalNonceByVaultAndAccount<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		VaultId,
		Blake2_128Concat,
		T::AccountId,
		u64,
		OptionQuery,
	>;

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

	/// The active bond state for a vault.
	///
	/// Admission totals and flexible displacement; individual lots live in `BondLotById`.
	#[pallet::storage]
	pub type BondLotsByVault<T: Config> =
		StorageMap<_, Twox64Concat, VaultId, VaultBondState, ValueQuery>;

	/// The active Argonot set keeps the smallest bond amount first, then lower ids first when
	/// amounts tie.
	#[pallet::storage]
	pub type ArgonotBondLots<T: Config> =
		StorageValue<_, BoundedVec<BondLotSummary, T::MaxActiveArgonotBondLots>, ValueQuery>;

	/// The total number of active Argonot bonds in the active set.
	#[pallet::storage]
	pub type TotalActiveArgonotBonds<T: Config> = StorageValue<_, Bonds, ValueQuery>;

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
		/// An error occurred while paying frame earnings for an Argonot bond lot.
		CouldNotDistributeEarningsToArgonotBondLot {
			frame_id: FrameId,
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
		/// A fixed or unearned reward allocation could not be burned.
		CouldNotBurnRewardAllocation {
			frame_id: FrameId,
			amount: T::Balance,
			dispatch_error: DispatchError,
		},
		/// Frame earnings were distributed.
		FrameEarningsDistributed {
			frame_id: FrameId,
			/// The total gross bid-pool allocation made to non-reserve recipients this frame.
			bid_pool_distributed: T::Balance,
			/// ARGON paid directly to Argonot Stake lots.
			stake_pool_distributed: T::Balance,
			/// ARGON paid directly to vault Argon bond lots.
			argon_bond_pool_distributed: T::Balance,
			/// ARGON recorded for vault collection.
			vault_pool_distributed: T::Balance,
			/// Fixed allocations, target shortfall, failures, and rounding burned this frame.
			burned: T::Balance,
			/// The amount moved into treasury reserves.
			treasury_reserves: T::Balance,
			participating_vaults: u32,
		},
		RewardEconomicsConfigured {
			target_bitcoin_percent: Percent,
		},
		/// The current frame's vault capital was locked in.
		FrameVaultCapitalLocked {
			frame_id: FrameId,
			total_active_bonds: u128,
			participating_vaults: u32,
		},
		/// An error occurred while releasing a bond lot.
		CouldNotReleaseBondLot {
			frame_id: FrameId,
			program_id: BondProgramId,
			bond_lot_id: BondLotId,
			amount: T::Balance,
			account_id: T::AccountId,
			dispatch_error: DispatchError,
		},
		/// A bond purchase entered its active program set.
		BondLotPurchased {
			program_id: BondProgramId,
			bond_lot_id: BondLotId,
			account_id: T::AccountId,
			bonds: Bonds,
		},
		/// A bond lot was removed from future frames and scheduled for release.
		BondLotReleaseScheduled {
			program_id: BondProgramId,
			bond_lot_id: BondLotId,
			account_id: T::AccountId,
			bonds: Bonds,
			release_frame_id: FrameId,
			reason: BondReleaseReason,
		},
		/// A bond lot was released.
		BondLotReleased {
			frame_id: FrameId,
			program_id: BondProgramId,
			bond_lot_id: BondLotId,
			account_id: T::AccountId,
			bonds: Bonds,
		},
		BondLotFlexibilityChanged {
			vault_id: VaultId,
			bond_lot_id: BondLotId,
			is_flexible: bool,
		},
		ReservedBondSpaceChanged {
			vault_id: VaultId,
			reserved_bond_space: Bonds,
		},
		/// Encumbered treasury backing was burned and any no-longer-needed fractional hold was
		/// returned.
		EncumberedBondMicrogonsBurned {
			account_id: T::AccountId,
			burned_amount: T::Balance,
			released_amount: T::Balance,
		},
		/// Historical flexible-bond earnings were attributed to a surviving vault lot. No funds
		/// move.
		BondLotEarningsBackfilled {
			bond_lot_id: BondLotId,
			added_frames: u32,
			added_earnings: T::Balance,
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
		/// The network has reached the direct-payout vault bond lot admission limit.
		MaxArgonBondLotsExceeded,
		/// The vault already has the maximum number of flexible bond lots.
		MaxFlexibleBondLotsExceeded,
		/// Too many bond lot releases are scheduled for the same frame.
		MaxPendingBondReleasesExceeded,
		/// The bond lot could not be found.
		BondLotNotFound,
		/// Historical metrics can only be backfilled for vault bond lots.
		BondLotCannotBeBackfilled,
		/// The lot's metrics changed since the backfill was calculated.
		BondLotMetricsChanged,
		/// The proposed backfill would reduce or corrupt the lot's earnings metrics.
		InvalidBondLotMetrics,
		/// The caller does not own the bond lot.
		NotBondLotOwner,
		/// The bond lot is already scheduled for release.
		BondLotAlreadyReleasing,
		/// The vault doesn't have enough raw securitization to support this bond purchase.
		InsufficientBondSpace,
		/// Liquidating this bond lot would take the account below its crosschain-encumbered
		/// treasury backing.
		ActiveBondAmountBelowEncumberedBacking,
		/// The bonus approval was signed for a different vault.
		BonusApprovalWrongVault,
		/// The bonus approval was signed for a different beneficiary.
		BonusApprovalWrongAccount,
		/// The bonus approval already expired.
		BonusApprovalExpired,
		/// The bonus approval nonce has already been consumed or superseded.
		BonusApprovalAlreadyUsed,
		/// The bonus approval signature is invalid or unauthorized.
		InvalidBonusApprovalSignature,
		/// The Argonot bond purchase did not beat the current active-set cutoff.
		ArgonotBondPurchaseBelowCutoff,
		/// The Argonot bond purchase would exceed the active circulation cap.
		ArgonotBondPurchaseAboveCap,
		/// Only an active vault bond owned by its operator can be used as flexible.
		BondLotCannotBeFlexible,
		/// The caller does not have permission to perform this action.
		NoPermissions,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Buy whole `1 ARGON` bonds for a vault.
		///
		/// The purchase either creates a bond lot or fails.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::buy_bonds().saturating_add(<T::TreasuryVaultProvider as TreasuryVaultProvider>::Weights::commit_securitization_for_bonds()))]
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
			ensure!(
				TotalArgonBondLots::<T>::get() < T::MaxArgonBondLots::get(),
				Error::<T>::MaxArgonBondLotsExceeded
			);

			let bond_capacity = Self::balance_to_bonds(Self::get_vault_bond_capacity(vault_id));

			let current_frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
			let mut vault_bonds = BondLotsByVault::<T>::get(vault_id);
			ensure!(!bond_capacity.is_zero(), Error::<T>::VaultNotAcceptingBondPurchases);

			let bonus_percent = Self::validate_bonus_approval(
				vault_id,
				&who,
				current_frame_id,
				bonus_approval.as_ref(),
			)?;
			let bond_space_to_unreserve = bonus_approval
				.as_ref()
				.map(|proof| proof.bond_space_to_unreserve)
				.unwrap_or_default();
			ensure!(
				bond_space_to_unreserve <= vault_bonds.reserved_bond_space,
				Error::<T>::InsufficientBondSpace
			);
			vault_bonds.reserved_bond_space =
				vault_bonds.reserved_bond_space.saturating_sub(bond_space_to_unreserve);

			ensure!(
				bonds <= vault_bonds.available_bond_space(bond_capacity),
				Error::<T>::InsufficientBondSpace
			);
			Self::lock_vault_frame_terms(&mut vault_bonds);

			let bond_lot_id = Self::next_bond_lot_id()?;
			let purchase_amount = Self::bonds_to_balance(bonds);
			Self::create_hold::<T::Currency>(&who, purchase_amount)?;

			vault_bonds.regular_bonds.saturating_accrue(bonds);
			T::TreasuryVaultProvider::commit_securitization_for_bonds(
				vault_id,
				Self::bonds_to_balance(vault_bonds.regular_bonds),
			)
			.map_err(|_| Error::<T>::InsufficientBondSpace)?;

			let program =
				BondProgram::Vault { vault_id, sharing_percent: Permill::zero(), bonus_percent };
			BondLotById::<T>::insert(
				bond_lot_id,
				BondLot {
					owner: who.clone(),
					program,
					bonds,
					is_flexible: false,
					locked_frame_terms: Self::has_current_bond_payout_frame()
						.then_some(LockedFrameBondTerms { bonds: 0, is_flexible: false }),
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
			Self::update_vault_displacement(&mut vault_bonds, bond_capacity);
			BondLotsByVault::<T>::insert(vault_id, vault_bonds);

			Self::deposit_event(Event::<T>::BondLotPurchased {
				program_id: program.id(),
				bond_lot_id,
				account_id: who.clone(),
				bonds,
			});
			BondLotIdsByVault::<T>::insert(vault_id, bond_lot_id, ());
			TotalArgonBondLots::<T>::mutate(|count| count.saturating_accrue(1));
			Self::update_account_vault_bond_total(&who)?;
			if let Some(bonus_approval) = bonus_approval {
				LastBonusApprovalNonceByVaultAndAccount::<T>::insert(
					vault_id,
					who,
					bonus_approval.nonce,
				);
			}
			Ok(())
		}

		/// Liquidate one full bond lot. It keeps the locked frame's payout terms and is
		/// released after the delay.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::liquidate_bond_lot())]
		pub fn liquidate_bond_lot(origin: OriginFor<T>, bond_lot_id: BondLotId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let bond_lot = BondLotById::<T>::get(bond_lot_id).ok_or(Error::<T>::BondLotNotFound)?;
			ensure!(bond_lot.owner == who, Error::<T>::NotBondLotOwner);
			ensure!(bond_lot.release_reason.is_none(), Error::<T>::BondLotAlreadyReleasing);

			let remaining_vault_bonds = match bond_lot.program {
				BondProgram::Vault { vault_id, .. } => {
					let (active_balance, current_hold) = Self::account_vault_bond_status(&who)?;
					let remaining_non_releasing_hold =
						current_hold.saturating_sub(Self::bonds_to_balance(bond_lot.bonds));
					ensure!(
						remaining_non_releasing_hold >= Self::encumbered_bond_microgons(&who),
						Error::<T>::ActiveBondAmountBelowEncumberedBacking,
					);

					Self::remove_bond_lot_from_vault(vault_id, &bond_lot);
					Self::lock_bond_frame_terms(bond_lot_id, &bond_lot);
					Some(active_balance.saturating_sub(Self::bonds_to_balance(bond_lot.bonds)))
				},
				BondProgram::Argonot => {
					ArgonotBondLots::<T>::try_mutate(|active_lots| -> DispatchResult {
						let index = active_lots
							.iter()
							.position(|summary| summary.bond_lot_id == bond_lot_id)
							.ok_or(Error::<T>::BondLotNotFound)?;
						active_lots.remove(index);
						Ok(())
					})?;
					TotalActiveArgonotBonds::<T>::put(
						TotalActiveArgonotBonds::<T>::get()
							.checked_sub(bond_lot.bonds)
							.ok_or(ArithmeticError::Underflow)?,
					);
					None
				},
			};

			Self::schedule_bond_lot_release(bond_lot_id, BondReleaseReason::UserLiquidation)?;
			if let Some(amount) = remaining_vault_bonds {
				T::OperationalAccountsHook::account_vault_bond_total_updated(&who, amount);
			}
			Ok(())
		}

		/// Buy whole bond units for the Argonot active set.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::buy_argonot_bonds())]
		pub fn buy_argonot_bonds(origin: OriginFor<T>, bonds: Bonds) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(bonds >= Self::minimum_purchase_bonds(), Error::<T>::BondPurchaseBelowMinimum);

			let current_total_bonds = TotalActiveArgonotBonds::<T>::get();
			let max_active_bonds = Self::maximum_active_argonot_bonds();
			let active_lots = ArgonotBondLots::<T>::get();
			let active_lot_count = active_lots.len() as u32;
			let mut evicted_bond_lot_id = None;

			let next_total_bonds = if active_lot_count < T::MaxActiveArgonotBondLots::get() {
				current_total_bonds.checked_add(bonds).ok_or(ArithmeticError::Overflow)?
			} else {
				let floor_lot = active_lots.first().ok_or(Error::<T>::InternalError)?;
				ensure!(bonds > floor_lot.bonds, Error::<T>::ArgonotBondPurchaseBelowCutoff);
				evicted_bond_lot_id = Some(floor_lot.bond_lot_id);

				current_total_bonds
					.checked_sub(floor_lot.bonds)
					.ok_or(ArithmeticError::Underflow)?
					.checked_add(bonds)
					.ok_or(ArithmeticError::Overflow)?
			};
			ensure!(next_total_bonds <= max_active_bonds, Error::<T>::ArgonotBondPurchaseAboveCap);

			let program = BondProgram::Argonot;
			let program_id = program.id();
			let bond_lot_id = Self::next_bond_lot_id()?;
			let current_frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
			Self::create_hold::<T::OwnershipCurrency>(&who, Self::bonds_to_balance(bonds))?;
			BondLotById::<T>::insert(
				bond_lot_id,
				BondLot {
					owner: who.clone(),
					program,
					bonds,
					is_flexible: false,
					locked_frame_terms: None,
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

			ArgonotBondLots::<T>::try_mutate(|active_lots| -> DispatchResult {
				if let Some(evicted_bond_lot_id) = evicted_bond_lot_id {
					let removed = active_lots.remove(0);
					ensure!(removed.bond_lot_id == evicted_bond_lot_id, Error::<T>::InternalError);
				}

				let insert_index = active_lots
					.iter()
					.position(|summary| {
						summary.bonds > bonds ||
							(summary.bonds == bonds && summary.bond_lot_id > bond_lot_id)
					})
					.unwrap_or(active_lots.len());
				active_lots
					.try_insert(insert_index, BondLotSummary { bond_lot_id, bonds })
					.map_err(|_| Error::<T>::InternalError)?;
				Ok(())
			})?;

			if let Some(evicted_bond_lot_id) = evicted_bond_lot_id {
				Self::schedule_bond_lot_release(evicted_bond_lot_id, BondReleaseReason::Bumped)?;
			}
			TotalActiveArgonotBonds::<T>::put(next_total_bonds);

			Self::deposit_event(Event::<T>::BondLotPurchased {
				program_id,
				bond_lot_id,
				account_id: who.clone(),
				bonds,
			});
			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::set_bond_lot_flexible().saturating_add(<T::TreasuryVaultProvider as TreasuryVaultProvider>::Weights::commit_securitization_for_bonds()))]
		pub fn set_bond_lot_flexible(
			origin: OriginFor<T>,
			bond_lot_id: BondLotId,
			is_flexible: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut bond_lot =
				BondLotById::<T>::get(bond_lot_id).ok_or(Error::<T>::BondLotNotFound)?;
			ensure!(bond_lot.owner == who, Error::<T>::NotBondLotOwner);
			ensure!(bond_lot.release_reason.is_none(), Error::<T>::BondLotAlreadyReleasing);
			let BondProgram::Vault { vault_id, .. } = bond_lot.program else {
				return Err(Error::<T>::BondLotCannotBeFlexible.into());
			};
			ensure!(
				T::TreasuryVaultProvider::get_vault_operator(vault_id).as_ref() == Some(&who),
				Error::<T>::BondLotCannotBeFlexible
			);
			if bond_lot.is_flexible == is_flexible {
				return Ok(());
			}

			let bond_capacity = Self::balance_to_bonds(Self::get_vault_bond_capacity(vault_id));
			BondLotsByVault::<T>::try_mutate(vault_id, |vault_bonds| -> DispatchResult {
				Self::lock_vault_frame_terms(vault_bonds);
				if is_flexible {
					ensure!(
						vault_bonds.flexible_bonds <= Bonds::MAX - bond_lot.bonds,
						ArithmeticError::Overflow
					);
					vault_bonds.regular_bonds.saturating_reduce(bond_lot.bonds);
					vault_bonds.flexible_bonds.saturating_accrue(bond_lot.bonds);
				} else {
					ensure!(
						bond_lot.bonds <= vault_bonds.available_bond_space(bond_capacity),
						Error::<T>::InsufficientBondSpace
					);
					T::TreasuryVaultProvider::commit_securitization_for_bonds(
						vault_id,
						Self::bonds_to_balance(
							vault_bonds.regular_bonds.saturating_add(bond_lot.bonds),
						),
					)
					.map_err(|_| Error::<T>::InsufficientBondSpace)?;
					vault_bonds.flexible_bonds.saturating_reduce(bond_lot.bonds);
					vault_bonds.regular_bonds.saturating_accrue(bond_lot.bonds);
				}
				Self::update_vault_displacement(vault_bonds, bond_capacity);
				Ok(())
			})?;
			if Self::has_current_bond_payout_frame() {
				bond_lot.locked_frame_terms.get_or_insert(LockedFrameBondTerms {
					bonds: bond_lot.bonds,
					is_flexible: bond_lot.is_flexible,
				});
			}
			bond_lot.is_flexible = is_flexible;
			BondLotById::<T>::insert(bond_lot_id, bond_lot);
			Self::deposit_event(Event::BondLotFlexibilityChanged {
				vault_id,
				bond_lot_id,
				is_flexible,
			});
			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::set_reserved_bond_space())]
		/// Reserve vault bond space for future bond purchases.
		pub fn set_reserved_bond_space(
			origin: OriginFor<T>,
			vault_id: VaultId,
			reserved_bond_space: Bonds,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let operator = T::TreasuryVaultProvider::get_vault_operator(vault_id);
			ensure!(operator.as_ref() == Some(&who), Error::<T>::NoPermissions);
			let bond_capacity = Self::balance_to_bonds(Self::get_vault_bond_capacity(vault_id));
			BondLotsByVault::<T>::try_mutate(vault_id, |vault_bonds| -> DispatchResult {
				ensure!(
					reserved_bond_space <= vault_bonds.bond_space(bond_capacity),
					Error::<T>::InsufficientBondSpace
				);
				vault_bonds.reserved_bond_space = reserved_bond_space;
				Ok(())
			})?;
			Self::deposit_event(Event::ReservedBondSpaceChanged { vault_id, reserved_bond_space });
			Ok(())
		}

		/// Update reward economics without resetting fields omitted by the caller.
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::configure_reward_economics())]
		pub fn configure_reward_economics(
			origin: OriginFor<T>,
			target_bitcoin_percent: Option<Percent>,
		) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(target_bitcoin_percent) = target_bitcoin_percent {
				TargetBitcoinPercent::<T>::put(target_bitcoin_percent);
			}
			Self::deposit_event(Event::RewardEconomicsConfigured {
				target_bitcoin_percent: TargetBitcoinPercent::<T>::get(),
			});
			Ok(())
		}

		/// Attribute earnings already paid to a vault under the old aggregate flexible-bond model.
		/// This changes bond-lot metrics only; it never pays, holds, or releases funds. `expected`
		/// guards against overwriting a newer payout, and replaying `updated` is a no-op.
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::backfill_bond_lot_earnings())]
		pub fn backfill_bond_lot_earnings(
			origin: OriginFor<T>,
			bond_lot_id: BondLotId,
			expected: BondLotEarningsMetrics<T>,
			updated: BondLotEarningsMetrics<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let mut lot = BondLotById::<T>::get(bond_lot_id).ok_or(Error::<T>::BondLotNotFound)?;
			ensure!(
				matches!(lot.program, BondProgram::Vault { .. }),
				Error::<T>::BondLotCannotBeBackfilled
			);
			let current = BondLotEarningsMetrics {
				participated_frames: lot.participated_frames,
				last_frame_earnings_frame_id: lot.last_frame_earnings_frame_id,
				last_frame_earnings: lot.last_frame_earnings,
				cumulative_earnings: lot.cumulative_earnings,
			};
			if current == updated {
				return Ok(());
			}
			ensure!(current == expected, Error::<T>::BondLotMetricsChanged);
			ensure!(
				updated.participated_frames >= expected.participated_frames &&
					updated.cumulative_earnings >= expected.cumulative_earnings &&
					updated.last_frame_earnings_frame_id >=
						expected.last_frame_earnings_frame_id &&
					updated.last_frame_earnings_frame_id.is_some() ==
						updated.last_frame_earnings.is_some(),
				Error::<T>::InvalidBondLotMetrics
			);
			if let Some(frame_id) = updated.last_frame_earnings_frame_id {
				ensure!(
					frame_id >= lot.created_frame_id &&
						frame_id <= T::MiningFrameTransitionProvider::get_current_frame_id(),
					Error::<T>::InvalidBondLotMetrics
				);
			}
			if updated.last_frame_earnings_frame_id == expected.last_frame_earnings_frame_id {
				ensure!(
					updated.last_frame_earnings == expected.last_frame_earnings,
					Error::<T>::InvalidBondLotMetrics
				);
			}
			lot.participated_frames = updated.participated_frames;
			lot.last_frame_earnings_frame_id = updated.last_frame_earnings_frame_id;
			lot.last_frame_earnings = updated.last_frame_earnings;
			lot.cumulative_earnings = updated.cumulative_earnings;
			BondLotById::<T>::insert(bond_lot_id, lot);
			Self::deposit_event(Event::BondLotEarningsBackfilled {
				bond_lot_id,
				added_frames: updated
					.participated_frames
					.saturating_sub(expected.participated_frames),
				added_earnings: updated
					.cumulative_earnings
					.saturating_sub(expected.cumulative_earnings),
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

		pub(crate) fn create_hold<C>(
			account_id: &T::AccountId,
			amount: T::Balance,
		) -> DispatchResult
		where
			C: MutateHold<T::AccountId, Reason = T::RuntimeHoldReason, Balance = T::Balance>,
		{
			if amount.is_zero() {
				return Ok(());
			}
			let hold_reason = HoldReason::ContributedToTreasury;
			if C::balance_on_hold(&hold_reason.into(), account_id).is_zero() {
				frame_system::Pallet::<T>::inc_providers(account_id);
			}

			C::hold(&hold_reason.into(), account_id, amount)?;
			Ok(())
		}

		fn release_hold<C>(who: &T::AccountId, amount: T::Balance) -> DispatchResult
		where
			C: MutateHold<T::AccountId, Reason = T::RuntimeHoldReason, Balance = T::Balance>,
		{
			if amount.is_zero() {
				return Ok(());
			}
			let reason = HoldReason::ContributedToTreasury;
			C::release(&reason.into(), who, amount, Precision::Exact)?;

			if C::balance_on_hold(&reason.into(), who).is_zero() {
				let _ = frame_system::Pallet::<T>::dec_providers(who);
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
			ensure!(
				LastBonusApprovalNonceByVaultAndAccount::<T>::get(vault_id, beneficiary)
					.is_none_or(|nonce| bonus_approval.nonce > nonce),
				Error::<T>::BonusApprovalAlreadyUsed
			);

			Ok(bonus_approval.bonus_percent)
		}

		/// Once the frame is complete, this fn distributes frame earnings to Argonot bond lots
		/// first, then Argon bonds, and finally vaults.
		pub(crate) fn distribute_bid_pool(frame_id: FrameId) {
			let argonot_participants = match CurrentFrameArgonotBondParticipants::<T>::take() {
				Some(participants) if participants.frame_id == frame_id => Some(participants),
				Some(participants) => {
					CurrentFrameArgonotBondParticipants::<T>::put(participants);
					None
				},
				None => None,
			};
			let frame_capital = match CurrentFrameVaultCapital::<T>::take() {
				Some(frame_capital) if frame_capital.frame_id == frame_id => Some(frame_capital),
				Some(frame_capital) => {
					CurrentFrameVaultCapital::<T>::put(frame_capital);
					None
				},
				None => None,
			};
			let bid_pool_account = T::MiningBidPoolAccount::get();
			Self::ensure_account_provider(&bid_pool_account);
			let full_bid_pool_amount = T::Currency::balance(&bid_pool_account);
			let initial_reserves_amount =
				T::PercentForTreasuryReserves::get().mul_floor(full_bid_pool_amount);
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
				}
			}

			let stake_pool = T::PercentForStakePool::get().mul_floor(full_bid_pool_amount);
			let stake_pool_distributed = Self::distribute_stake_pool(
				frame_id,
				&bid_pool_account,
				stake_pool,
				argonot_participants,
			);
			let argon_bond_pool = T::PercentForArgonBondPool::get().mul_floor(full_bid_pool_amount);
			let argon_bond_pool_distributed = Self::distribute_argon_bond_pool(
				frame_id,
				&bid_pool_account,
				argon_bond_pool,
				frame_capital.as_ref(),
			);
			let vault_pool = T::PercentForVaultPool::get().mul_floor(full_bid_pool_amount);
			let (vault_pool_distributed, participating_vaults) = Self::distribute_vault_pool(
				frame_id,
				&bid_pool_account,
				full_bid_pool_amount,
				frame_capital.as_ref(),
			);

			let mining_operator_pool =
				T::PercentForMiningOperatorPool::get().mul_floor(full_bid_pool_amount);
			let bitcoin_liquid_pool =
				T::PercentForBitcoinLiquidPool::get().mul_floor(full_bid_pool_amount);
			let allocated = initial_reserves_amount
				.saturating_add(stake_pool)
				.saturating_add(mining_operator_pool)
				.saturating_add(bitcoin_liquid_pool)
				.saturating_add(argon_bond_pool)
				.saturating_add(vault_pool);
			let burn_amount = mining_operator_pool
				.saturating_add(bitcoin_liquid_pool)
				.saturating_add(initial_reserves_amount.saturating_sub(total_treasury_reserves))
				.saturating_add(stake_pool.saturating_sub(stake_pool_distributed))
				.saturating_add(argon_bond_pool.saturating_sub(argon_bond_pool_distributed))
				.saturating_add(vault_pool.saturating_sub(vault_pool_distributed))
				.saturating_add(full_bid_pool_amount.saturating_sub(allocated));
			let burned = Self::burn_unearned_rewards(frame_id, &bid_pool_account, burn_amount);

			Self::deposit_event(Event::<T>::FrameEarningsDistributed {
				frame_id,
				bid_pool_distributed: stake_pool_distributed
					.saturating_add(argon_bond_pool_distributed)
					.saturating_add(vault_pool_distributed),
				stake_pool_distributed,
				argon_bond_pool_distributed,
				vault_pool_distributed,
				burned,
				treasury_reserves: total_treasury_reserves,
				participating_vaults,
			});
		}

		pub(crate) fn lock_in_argonot_bond_participants(frame_id: FrameId) {
			let bond_lots = ArgonotBondLots::<T>::get();
			let total_bonds = TotalActiveArgonotBonds::<T>::get();
			if bond_lots.is_empty() || total_bonds == 0 {
				CurrentFrameArgonotBondParticipants::<T>::kill();
				return;
			}

			CurrentFrameArgonotBondParticipants::<T>::put(FrameArgonotBondParticipants {
				frame_id,
				total_bonds,
				bond_lots,
			});
		}

		/// Activates pending bond positions and locks the vault inputs for the next frame.
		pub(crate) fn lock_in_vault_capital(frame_id: FrameId) {
			let max_vaults = T::MaxVaultsPerPool::get() as usize;
			let mut total_active_bonds = 0u128;
			let mut empty_vaults = Vec::new();
			for (vault_id, mut vault_bonds) in BondLotsByVault::<T>::iter() {
				total_active_bonds.saturating_accrue(
					(vault_bonds.regular_bonds as u128)
						.saturating_add(vault_bonds.flexible_bonds as u128)
						.saturating_sub(vault_bonds.displaced_flexible_bonds as u128),
				);
				if vault_bonds.locked_frame_terms.take().is_some() {
					if vault_bonds.regular_bonds == 0 &&
						vault_bonds.flexible_bonds == 0 &&
						vault_bonds.reserved_bond_space == 0
					{
						empty_vaults.push(vault_id);
					} else {
						BondLotsByVault::<T>::insert(vault_id, vault_bonds);
					}
				}
			}
			for vault_id in empty_vaults {
				BondLotsByVault::<T>::remove(vault_id);
			}
			let (securitization_positions, total_securitization) =
				T::TreasuryVaultProvider::get_top_vaults_by_securitization(max_vaults as u32);

			let mut vault_securitization_positions = BoundedBTreeMap::new();
			let previous_frame_id = frame_id.saturating_sub(1);
			let microgons_per_micronot = FixedU128::from_rational(
				T::PriceProvider::get_average_microgons_per_argonot(previous_frame_id)
					.unwrap_or_default()
					.into(),
				MICROGONS_PER_ARGON,
			);
			for VaultSecuritization {
				vault_id,
				operator_account_id,
				securitization,
				activated_securitization,
				bitcoin_locked_satoshis,
				securitization_micronots,
			} in securitization_positions
			{
				let argonot_capacity = FixedU128::saturating_from_integer(securitization.into())
					.saturating_mul(FixedU128::from_u32(ARGONOT_SECURITIZATION_MULTIPLIER));
				let required_micronots = argonot_capacity
					.checked_div(&microgons_per_micronot)
					.unwrap_or_default()
					.ceil()
					.saturating_mul_int(1u128);
				let reward_micronots = securitization_micronots.min(required_micronots.into());
				T::TreasuryVaultProvider::commit_securitization_for_rewards(
					vault_id,
					reward_micronots,
				);
				let vault_bonds = BondLotsByVault::<T>::get(vault_id);
				let active_bonds = vault_bonds.regular_bonds.saturating_add(
					vault_bonds.flexible_bonds.saturating_sub(vault_bonds.displaced_flexible_bonds),
				);
				let _ = vault_securitization_positions.try_insert(
					vault_id,
					VaultSecuritizationPosition {
						operator_account_id,
						securitization,
						activated_securitization,
						bitcoin_locked_microgons:
							T::PriceProvider::get_btc_price_in_market_microgons(
								bitcoin_locked_satoshis,
							)
							.unwrap_or_default(),
						argonot_securitization_in_microgons: microgons_per_micronot
							.saturating_mul_int(securitization_micronots.into())
							.into(),
						active_bond_microgons: Self::bonds_to_balance(active_bonds),
					},
				);
			}
			let participating_vaults = vault_securitization_positions.len() as u32;

			let bitcoin_minted = T::BitcoinMintedProvider::minted_bitcoin_microgons();
			let mining_minted = T::Currency::total_issuance().saturating_sub(bitcoin_minted);
			CurrentFrameVaultCapital::<T>::put(FrameVaultCapital {
				frame_id,
				total_active_bonds,
				target_securitization: TargetBitcoinPercent::<T>::get()
					.mul_floor(mining_minted)
					.max(bitcoin_minted),
				total_securitization,
				vault_securitization_positions,
			});

			Self::deposit_event(Event::<T>::FrameVaultCapitalLocked {
				frame_id,
				total_active_bonds,
				participating_vaults,
			});
		}

		/// Pay the completed frame, release matured lots, then activate next-frame positions.
		pub(crate) fn run_frame_transition(frame_id: FrameId) {
			if frame_id == 0 {
				return;
			}

			let payout_frame = frame_id - 1;
			info!("Starting treasury bond frame {frame_id}. Distributing frame {payout_frame}.");
			Self::distribute_bid_pool(payout_frame);
			Self::release_pending_bond_lots(frame_id);
			Self::lock_in_argonot_bond_participants(frame_id);
			Self::lock_in_vault_capital(frame_id);
		}

		fn distribute_stake_pool(
			frame_id: FrameId,
			bid_pool_account: &T::AccountId,
			stake_pool: T::Balance,
			argonot_participants: Option<FrameArgonotBondParticipants<T>>,
		) -> T::Balance {
			let Some(argonot_participants) = argonot_participants else {
				return T::Balance::zero();
			};
			if argonot_participants.total_bonds == 0 {
				return T::Balance::zero();
			}

			let mut distributed = T::Balance::zero();
			for participant in argonot_participants.bond_lots.iter() {
				let Some(bond_lot) = BondLotById::<T>::get(participant.bond_lot_id) else {
					continue;
				};
				if !matches!(bond_lot.program, BondProgram::Argonot) {
					continue;
				}

				let gross_lot_yield = Perbill::from_rational(
					participant.bonds as u128,
					argonot_participants.total_bonds as u128,
				)
				.mul_floor(stake_pool);

				let mut paid_payout = gross_lot_yield;
				if !paid_payout.is_zero() &&
					let Err(e) = T::Currency::transfer(
						bid_pool_account,
						&bond_lot.owner,
						paid_payout,
						Preservation::Expendable,
					) {
					Self::deposit_event(Event::<T>::CouldNotDistributeEarningsToArgonotBondLot {
						frame_id,
						bond_lot_id: participant.bond_lot_id,
						account_id: bond_lot.owner,
						amount: paid_payout,
						dispatch_error: e,
					});
					paid_payout = T::Balance::zero();
				}

				Self::record_bond_lot_earnings(participant.bond_lot_id, frame_id, paid_payout);
				distributed.saturating_accrue(paid_payout);
			}
			distributed
		}

		fn distribute_argon_bond_pool(
			frame_id: FrameId,
			bid_pool_account: &T::AccountId,
			argon_bond_pool: T::Balance,
			frame_capital: Option<&FrameVaultCapital<T>>,
		) -> T::Balance {
			let Some(frame_capital) = frame_capital else {
				for (_, bond_lot_id, ()) in BondLotIdsByVault::<T>::iter() {
					let Some(bond_lot) = BondLotById::<T>::get(bond_lot_id) else { continue };
					if bond_lot.locked_frame_terms.is_some() {
						BondLotById::<T>::mutate(bond_lot_id, |lot| {
							if let Some(lot) = lot {
								lot.locked_frame_terms = None;
							}
						});
					}
				}
				return T::Balance::zero();
			};
			let total_eligible_balance =
				frame_capital.total_active_bonds.saturating_mul(MICROGONS_PER_ARGON).into();
			let denominator = frame_capital.target_securitization.max(total_eligible_balance);
			let mut distributed = T::Balance::zero();
			let mut flexible_fractions = BTreeMap::new();
			for (vault_id, bond_lot_id, ()) in BondLotIdsByVault::<T>::iter() {
				let Some(bond_lot) = BondLotById::<T>::get(bond_lot_id) else { continue };
				let (bonds, is_flexible) = match bond_lot.locked_frame_terms.as_ref() {
					Some(terms) => (terms.bonds, terms.is_flexible),
					None if bond_lot.release_reason.is_some() => continue,
					None => (bond_lot.bonds, bond_lot.is_flexible),
				};
				if bond_lot.locked_frame_terms.is_some() {
					BondLotById::<T>::mutate(bond_lot_id, |lot| {
						if let Some(lot) = lot {
							lot.locked_frame_terms = None;
						}
					});
				}
				if bonds == 0 {
					continue;
				}
				let principal_microgons = (bonds as u128).saturating_mul(MICROGONS_PER_ARGON);
				let eligible_microgons = if is_flexible {
					let fraction = flexible_fractions.entry(vault_id).or_insert_with(|| {
						let state = BondLotsByVault::<T>::get(vault_id);
						let (flexible, displaced) = match state.locked_frame_terms {
							Some(terms) => (terms.flexible_bonds, terms.displaced_flexible_bonds),
							None => (state.flexible_bonds, state.displaced_flexible_bonds),
						};
						FixedU128::from_rational(
							flexible.saturating_sub(displaced) as u128,
							flexible as u128,
						)
					});
					fraction.saturating_mul_int(principal_microgons)
				} else {
					principal_microgons
				};
				let payout = FixedU128::from_rational(eligible_microgons, denominator.into())
					.saturating_mul_int(argon_bond_pool);
				distributed.saturating_accrue(Self::pay_argon_bond_lot(
					frame_id,
					bid_pool_account,
					bond_lot_id,
					vault_id,
					&bond_lot,
					payout,
				));
			}
			distributed
		}

		fn pay_argon_bond_lot(
			frame_id: FrameId,
			bid_pool_account: &T::AccountId,
			bond_lot_id: BondLotId,
			vault_id: VaultId,
			bond_lot: &BondLot<T>,
			payout: T::Balance,
		) -> T::Balance {
			let mut paid_payout = payout;
			if !payout.is_zero() &&
				let Err(e) = T::Currency::transfer(
					bid_pool_account,
					&bond_lot.owner,
					payout,
					Preservation::Expendable,
				) {
				Self::deposit_event(Event::<T>::CouldNotDistributeEarningsToBondLot {
					frame_id,
					vault_id,
					bond_lot_id,
					account_id: bond_lot.owner.clone(),
					amount: payout,
					dispatch_error: e,
				});
				paid_payout = T::Balance::zero();
			}
			Self::record_bond_lot_earnings(bond_lot_id, frame_id, paid_payout);
			paid_payout
		}

		fn distribute_vault_pool(
			frame_id: FrameId,
			bid_pool_account: &T::AccountId,
			full_bid_pool_amount: T::Balance,
			frame_capital: Option<&FrameVaultCapital<T>>,
		) -> (T::Balance, u32) {
			let Some(frame_capital) = frame_capital else {
				return (T::Balance::zero(), 0);
			};
			let denominator =
				frame_capital.target_securitization.max(frame_capital.total_securitization);
			let minimum_profit_rate = FixedU128::from_rational(1, 100);
			let maximum_profit_rate =
				FixedU128::from_rational(T::PercentForVaultPool::get().deconstruct() as u128, 100);
			// The website curve divides its maximum by 3x capital and a 1.29x Argonot bonus.
			// Its 57% maximum includes the 6% mining-operator allocation burned this release.
			let maximum_core_profit_rate = maximum_profit_rate
				.checked_div(&FixedU128::from_rational(387, 100))
				.unwrap_or_default();
			let mut distributed = T::Balance::zero();

			for (vault_id, position) in frame_capital.vault_securitization_positions.iter() {
				let securitization: u128 = position.securitization.into();
				if securitization == 0 {
					continue;
				}
				let excess_bitcoin_value =
					position.bitcoin_locked_microgons.saturating_sub(position.securitization);
				let bitcoin_utilization = FixedU128::from_rational(
					position
						.activated_securitization
						.min(position.securitization)
						.saturating_sub(excess_bitcoin_value)
						.into(),
					securitization,
				);
				let bond_utilization = FixedU128::from_rational(
					position.active_bond_microgons.min(position.securitization).into(),
					securitization,
				);
				let argonot_capacity =
					securitization.saturating_mul(ARGONOT_SECURITIZATION_MULTIPLIER.into());
				let argonot_value: u128 = position.argonot_securitization_in_microgons.into();
				let argonot_value = argonot_value.min(argonot_capacity);
				let argonot_utilization = FixedU128::from_rational(argonot_value, argonot_capacity);
				let core_utilization = bitcoin_utilization.saturating_mul(
					FixedU128::from_rational(9, 10).saturating_add(
						FixedU128::from_rational(1, 10).saturating_mul(bond_utilization),
					),
				);
				let core_profit_rate =
					minimum_profit_rate.saturating_add(core_utilization.saturating_mul(
						maximum_core_profit_rate.saturating_sub(minimum_profit_rate),
					));
				let capital_multiplier = FixedU128::from_rational(
					securitization.saturating_add(argonot_value),
					securitization,
				);
				let argonot_bonus = FixedU128::one().saturating_add(
					FixedU128::from_rational(29, 100)
						.saturating_mul(argonot_utilization)
						.saturating_mul(bitcoin_utilization),
				);
				let profit_rate = core_profit_rate
					.saturating_mul(capital_multiplier)
					.saturating_mul(argonot_bonus)
					.min(maximum_profit_rate);
				let coverage = FixedU128::from_rational(securitization, denominator.into());
				let earnings =
					coverage.saturating_mul(profit_rate).saturating_mul_int(full_bid_pool_amount);

				if T::TreasuryVaultProvider::record_vault_frame_earnings(
					bid_pool_account,
					VaultTreasuryFrameEarnings {
						vault_id: *vault_id,
						vault_operator_account_id: position.operator_account_id.clone(),
						frame_id,
						earnings_for_vault: earnings,
						earnings,
						capital_contributed: position.active_bond_microgons,
						capital_contributed_by_vault: T::Balance::zero(),
					},
				)
				.is_ok()
				{
					distributed.saturating_accrue(earnings);
				}
			}

			(distributed, frame_capital.vault_securitization_positions.len() as u32)
		}

		fn burn_unearned_rewards(
			frame_id: FrameId,
			bid_pool_account: &T::AccountId,
			amount: T::Balance,
		) -> T::Balance {
			if amount.is_zero() {
				return T::Balance::zero();
			}
			match T::Currency::burn_from(
				bid_pool_account,
				amount,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			) {
				Ok(burned) => {
					T::BurnEventHandler::on_argon_burn(&burned);
					burned
				},
				Err(e) => {
					Self::deposit_event(Event::<T>::CouldNotBurnRewardAllocation {
						frame_id,
						amount,
						dispatch_error: e,
					});
					T::Balance::zero()
				},
			}
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
					let program_id = bond_lot.program.id();

					let release_result = match bond_lot.program {
						BondProgram::Vault { .. } =>
							Self::release_hold::<T::Currency>(&bond_lot.owner, release_amount),
						BondProgram::Argonot => Self::release_hold::<T::OwnershipCurrency>(
							&bond_lot.owner,
							release_amount,
						),
					};

					if let Err(e) = release_result {
						let _ = failed_releases.try_push(bond_lot_id);
						if next_retry_frame.is_none() {
							next_retry_frame = Some(due_frame);
						}
						Self::deposit_event(Event::<T>::CouldNotReleaseBondLot {
							frame_id: due_frame,
							program_id,
							bond_lot_id,
							amount: release_amount,
							account_id: bond_lot.owner,
							dispatch_error: e,
						});
						continue;
					}

					BondLotIdsByAccount::<T>::remove(&bond_lot.owner, bond_lot_id);
					if let BondProgram::Vault { vault_id, .. } = bond_lot.program {
						BondLotIdsByVault::<T>::remove(vault_id, bond_lot_id);
						TotalArgonBondLots::<T>::mutate(|count| count.saturating_reduce(1));
					}
					BondLotById::<T>::remove(bond_lot_id);
					Self::deposit_event(Event::<T>::BondLotReleased {
						frame_id: due_frame,
						program_id,
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

		pub(crate) fn get_vault_bond_capacity(vault_id: VaultId) -> T::Balance {
			T::TreasuryVaultProvider::get_vault_securitization(vault_id).unwrap_or_default()
		}

		fn update_vault_displacement(vault_bonds: &mut VaultBondState, capacity: Bonds) {
			let flexible_capacity = capacity.saturating_sub(vault_bonds.regular_bonds);
			vault_bonds.displaced_flexible_bonds =
				vault_bonds.flexible_bonds.saturating_sub(flexible_capacity);
		}

		fn has_current_bond_payout_frame() -> bool {
			CurrentFrameVaultCapital::<T>::get().is_some_and(|capital| {
				capital.frame_id == T::MiningFrameTransitionProvider::get_current_frame_id()
			})
		}

		fn lock_bond_frame_terms(bond_lot_id: BondLotId, bond_lot: &BondLot<T>) {
			if !Self::has_current_bond_payout_frame() {
				return;
			}
			BondLotById::<T>::mutate(bond_lot_id, |stored| {
				if let Some(stored) = stored {
					stored.locked_frame_terms.get_or_insert(LockedFrameBondTerms {
						bonds: bond_lot.bonds,
						is_flexible: bond_lot.is_flexible,
					});
				}
			});
		}

		fn lock_vault_frame_terms(vault_bonds: &mut VaultBondState) {
			if Self::has_current_bond_payout_frame() {
				vault_bonds.locked_frame_terms.get_or_insert(LockedFrameVaultTerms {
					flexible_bonds: vault_bonds.flexible_bonds,
					displaced_flexible_bonds: vault_bonds.displaced_flexible_bonds,
				});
			}
		}

		fn record_bond_lot_earnings(
			bond_lot_id: BondLotId,
			frame_id: FrameId,
			paid_payout: T::Balance,
		) {
			BondLotById::<T>::mutate_exists(bond_lot_id, |maybe_bond_lot| {
				let Some(bond_lot) = maybe_bond_lot.as_mut() else {
					return;
				};
				bond_lot.participated_frames = bond_lot.participated_frames.saturating_add(1);
				bond_lot.last_frame_earnings_frame_id = Some(frame_id);
				bond_lot.last_frame_earnings = Some(paid_payout);
				bond_lot.cumulative_earnings.saturating_accrue(paid_payout);
			});
		}

		fn account_vault_bond_status(
			account_id: &T::AccountId,
		) -> Result<(T::Balance, T::Balance), Error<T>> {
			let mut active_balance = T::Balance::zero();
			let mut releasing_balance = T::Balance::zero();

			for (bond_lot_id, ()) in BondLotIdsByAccount::<T>::iter_prefix(account_id) {
				let bond_lot =
					BondLotById::<T>::get(bond_lot_id).ok_or(Error::<T>::BondLotNotFound)?;
				if !matches!(bond_lot.program, BondProgram::Vault { .. }) {
					continue;
				}

				let bond_balance = Self::bonds_to_balance(bond_lot.bonds);
				if bond_lot.release_reason.is_some() {
					releasing_balance.saturating_accrue(bond_balance);
				} else {
					active_balance.saturating_accrue(bond_balance);
				}
			}

			let held_balance =
				T::Currency::balance_on_hold(&HoldReason::ContributedToTreasury.into(), account_id)
					.saturating_sub(releasing_balance);

			Ok((active_balance, held_balance))
		}

		fn active_non_releasing_vault_bond_amount(
			account_id: &T::AccountId,
			program_id: Option<BondProgramId>,
		) -> Result<T::Balance, Error<T>> {
			let mut active_balance = T::Balance::zero();

			for (bond_lot_id, ()) in BondLotIdsByAccount::<T>::iter_prefix(account_id) {
				let bond_lot =
					BondLotById::<T>::get(bond_lot_id).ok_or(Error::<T>::BondLotNotFound)?;
				if bond_lot.release_reason.is_some() {
					continue;
				}

				let bond_program_id = match bond_lot.program {
					BondProgram::Vault { .. } => bond_lot.program.id(),
					BondProgram::Argonot => continue,
				};

				if let Some(expected_program_id) = program_id &&
					bond_program_id != expected_program_id
				{
					continue;
				}

				active_balance.saturating_accrue(Self::bonds_to_balance(bond_lot.bonds));
			}

			Ok(active_balance)
		}

		pub(crate) fn encumbered_bond_microgons(account_id: &T::AccountId) -> T::Balance {
			EncumberedBondMicrogonsByAccount::<T>::get(account_id)
		}

		fn maximum_active_argonot_bonds() -> Bonds {
			let circulation = T::OwnershipCurrency::total_issuance();
			let cap_balance = T::MaxArgonotBondedPercentOfCirculation::get().mul_floor(circulation);
			Self::balance_to_bonds(cap_balance)
		}

		fn remove_bond_lot_from_vault(vault_id: VaultId, bond_lot: &BondLot<T>) {
			BondLotsByVault::<T>::mutate_exists(vault_id, |maybe_vault_bonds| {
				let Some(vault_bonds) = maybe_vault_bonds.as_mut() else {
					return;
				};
				Self::lock_vault_frame_terms(vault_bonds);

				if bond_lot.is_flexible {
					vault_bonds.flexible_bonds.saturating_reduce(bond_lot.bonds);
				} else {
					vault_bonds.regular_bonds.saturating_reduce(bond_lot.bonds);
				}

				Self::update_vault_displacement(
					vault_bonds,
					Self::balance_to_bonds(Self::get_vault_bond_capacity(vault_id)),
				);
				if vault_bonds.regular_bonds.is_zero() &&
					vault_bonds.flexible_bonds.is_zero() &&
					vault_bonds.locked_frame_terms.is_none() &&
					vault_bonds.reserved_bond_space.is_zero()
				{
					*maybe_vault_bonds = None;
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
				let program_id = bond_lot.program.id();
				let account_id = bond_lot.owner.clone();
				let bonds = bond_lot.bonds;
				let event_reason = reason.clone();
				bond_lot.release_frame_id = Some(release_frame_id);
				bond_lot.release_reason = Some(reason);
				Self::deposit_event(Event::<T>::BondLotReleaseScheduled {
					program_id,
					bond_lot_id,
					account_id,
					bonds,
					release_frame_id,
					reason: event_reason,
				});
				Ok(())
			})?;

			Ok(release_frame_id)
		}

		fn update_account_vault_bond_total(account_id: &T::AccountId) -> DispatchResult {
			let active_account_vault_bond_amount =
				Self::active_non_releasing_vault_bond_amount(account_id, None)?;
			T::OperationalAccountsHook::account_vault_bond_total_updated(
				account_id,
				active_account_vault_bond_amount,
			);
			Ok(())
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

		fn on_frame_start_weight(frame_id: FrameId) -> Weight {
			let payout_argonot_participants = CurrentFrameArgonotBondParticipants::<T>::get()
				.map(|participants| participants.bond_lots.len() as u32)
				.unwrap_or_default();
			// The transition pays the frozen list and locks the live list for the next frame.
			let argonot_lots =
				payout_argonot_participants.max(ArgonotBondLots::<T>::get().len() as u32);
			let first_due_frame =
				PendingBondReleaseRetryCursor::<T>::get().unwrap_or(frame_id).min(frame_id);
			let mut due_releases = 0u32;
			for due_frame in first_due_frame..=frame_id {
				due_releases
					.saturating_accrue(PendingBondReleasesByFrame::<T>::get(due_frame).len() as u32);
			}
			let due_frames = frame_id.saturating_sub(first_due_frame).saturating_add(1);
			let price_weight = <T::PriceProvider as PriceProvider<T::Balance>>::Weights::
				get_average_microgons_per_argonot();
			T::WeightInfo::on_frame_transition(
				TotalArgonBondLots::<T>::get(),
				argonot_lots,
				due_releases,
			)
			.saturating_add(T::DbWeight::get().reads(due_frames.saturating_add(4)))
			.saturating_add(
				T::DbWeight::get()
					.reads_writes(due_frames.saturating_sub(1), due_frames.saturating_sub(1)),
			)
			.saturating_add(price_weight)
			.saturating_add(
				<T::TreasuryVaultProvider as TreasuryVaultProvider>::Weights::
					get_top_vaults_by_securitization(T::MaxVaultsPerPool::get()),
			)
			.saturating_add(
				<T::TreasuryVaultProvider as TreasuryVaultProvider>::Weights::
					commit_securitization_for_rewards()
					.saturating_mul(T::MaxVaultsPerPool::get().into()),
			)
			.saturating_add(
				<T::TreasuryVaultProvider as TreasuryVaultProvider>::Weights::
					record_vault_frame_earnings()
					.saturating_mul(T::MaxVaultsPerPool::get().into()),
			)
		}
	}

	impl<T: Config> TreasuryPoolProvider<T::AccountId> for Pallet<T> {
		type Weights = ProviderWeightAdapter<T>;
		type Balance = T::Balance;

		fn vault_securitization_changed(vault_id: VaultId, securitization: Self::Balance) {
			BondLotsByVault::<T>::mutate_exists(vault_id, |maybe_vault_bonds| {
				if let Some(vault_bonds) = maybe_vault_bonds {
					Self::lock_vault_frame_terms(vault_bonds);
					Self::update_vault_displacement(
						vault_bonds,
						Self::balance_to_bonds(securitization),
					);
				}
			});
		}

		fn has_vault_bond_participation(vault_id: VaultId, account_id: &T::AccountId) -> bool {
			let active_balance = Self::active_non_releasing_vault_bond_amount(
				account_id,
				Some(BondProgramId::Vault { vault_id }),
			)
			.unwrap_or_default();
			!active_balance.is_zero()
		}

		fn active_vault_bond_amount(vault_id: VaultId, account_id: &T::AccountId) -> Self::Balance {
			Self::active_non_releasing_vault_bond_amount(
				account_id,
				Some(BondProgramId::Vault { vault_id }),
			)
			.unwrap_or_default()
		}

		fn active_account_vault_bond_amount(account_id: &T::AccountId) -> Self::Balance {
			Self::active_non_releasing_vault_bond_amount(account_id, None).unwrap_or_default()
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
			let (_, current_hold) = Self::account_vault_bond_status(account_id)?;
			ensure!(
				current_hold >= next_encumbered,
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
			let (current_active, current_hold) = Self::account_vault_bond_status(account_id)?;
			let required_hold = current_active.max(remaining_encumbered);
			let released_amount = current_hold.saturating_sub(required_hold);
			if !released_amount.is_zero() {
				Self::release_hold::<T::Currency>(account_id, released_amount)?;
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

			let (active_balance_before_trim, held_microgons_after_burn) =
				Self::account_vault_bond_status(account_id)?;
			ensure!(held_microgons_after_burn >= encumbered_after_burn, Error::<T>::InternalError,);

			let active_bonds_before_trim = Self::balance_to_bonds(active_balance_before_trim);
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
					let BondProgram::Vault { vault_id, .. } = bond_lot.program else {
						continue;
					};
					let removed_bonds = bond_lot.bonds.min(remaining_bonds_to_trim);
					let remaining_bonds = bond_lot.bonds.saturating_sub(removed_bonds);
					remaining_bonds_to_trim = remaining_bonds_to_trim.saturating_sub(removed_bonds);

					if remaining_bonds == 0 {
						BondLotById::<T>::remove(bond_lot_id);
						BondLotIdsByAccount::<T>::remove(account_id, bond_lot_id);
						BondLotIdsByVault::<T>::remove(vault_id, bond_lot_id);
						TotalArgonBondLots::<T>::mutate(|count| count.saturating_reduce(1));
						Self::remove_bond_lot_from_vault(vault_id, &bond_lot);
					} else {
						BondLotById::<T>::mutate_exists(bond_lot_id, |maybe_bond_lot| {
							let Some(bond_lot) = maybe_bond_lot.as_mut() else {
								return;
							};
							if Self::has_current_bond_payout_frame() {
								bond_lot.locked_frame_terms.get_or_insert(LockedFrameBondTerms {
									bonds: bond_lot.bonds,
									is_flexible: bond_lot.is_flexible,
								});
							}
							bond_lot.bonds = remaining_bonds;
						});
						BondLotsByVault::<T>::mutate_exists(vault_id, |maybe_vault_bonds| {
							let Some(vault_bonds) = maybe_vault_bonds.as_mut() else {
								return;
							};
							Self::lock_vault_frame_terms(vault_bonds);

							if bond_lot.is_flexible {
								vault_bonds.flexible_bonds.saturating_reduce(removed_bonds);
							} else {
								vault_bonds.regular_bonds.saturating_reduce(removed_bonds);
							}
							Self::update_vault_displacement(
								vault_bonds,
								Self::balance_to_bonds(Self::get_vault_bond_capacity(vault_id)),
							);
						});
					}
				}
			}

			ensure!(remaining_bonds_to_trim == 0, Error::<T>::InternalError);

			let target_active_hold = Self::bonds_to_balance(target_active_bonds);
			let required_hold = target_active_hold.max(encumbered_after_burn);
			let released_amount = held_microgons_after_burn.saturating_sub(required_hold);
			if !released_amount.is_zero() {
				Self::release_hold::<T::Currency>(account_id, released_amount)?;
			}
			Self::update_account_vault_bond_total(account_id)?;
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

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Copy,
		PartialEq,
		Eq,
		Debug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub enum BondProgram {
		Vault {
			#[codec(compact)]
			vault_id: VaultId,
			/// Historical purchase term; new bonds set it to zero and direct payouts ignore it.
			#[codec(compact)]
			sharing_percent: Permill,
			#[codec(compact)]
			bonus_percent: Permill,
		},
		Argonot,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Copy,
		PartialEq,
		Eq,
		Debug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub enum BondProgramId {
		Vault {
			#[codec(compact)]
			vault_id: VaultId,
		},
		Argonot,
	}

	impl BondProgram {
		fn id(self) -> BondProgramId {
			match self {
				Self::Vault { vault_id, .. } => BondProgramId::Vault { vault_id },
				Self::Argonot => BondProgramId::Argonot,
			}
		}
	}

	/// One purchase of `N` bonds for one treasury bond program.
	#[derive(
		Encode, Decode, Clone, PartialEqNoBound, Eq, DebugNoBound, TypeInfo, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct BondLot<T: Config> {
		/// The account that owns this purchase lot.
		pub owner: T::AccountId,
		/// The treasury bond program this purchase belongs to.
		pub program: BondProgram,
		/// The number of bonds in this lot. `1 ARGON = 1 bond`.
		#[codec(compact)]
		pub bonds: Bonds,
		/// Whether this operator-owned lot yields its admission capacity to regular bonds.
		pub is_flexible: bool,
		/// Terms used for this frame's payout when the live lot changes mid-frame.
		pub locked_frame_terms: Option<LockedFrameBondTerms>,
		/// The frame when this lot was purchased.
		#[codec(compact)]
		pub created_frame_id: FrameId,
		/// How many earning frames this lot has actually been in.
		#[codec(compact)]
		pub participated_frames: u32,
		/// The frame where `last_frame_earnings` was recorded.
		pub last_frame_earnings_frame_id: Option<FrameId>,
		/// The last frame's earnings attributed to this lot. Historical flexible-bond earnings
		/// were paid to the vault and may be backfilled here for reporting only.
		pub last_frame_earnings: Option<T::Balance>,
		/// Cumulative earnings attributed to this lot, including any historical flexible-bond
		/// share paid to its vault and backfilled for reporting only.
		#[codec(compact)]
		pub cumulative_earnings: T::Balance,
		/// The frame when the release delay finishes, if this lot is releasing.
		pub release_frame_id: Option<FrameId>,
		/// Why this lot entered release, if it is releasing.
		pub release_reason: Option<BondReleaseReason>,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEqNoBound,
		Eq,
		DebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct BondLotEarningsMetrics<T: Config> {
		#[codec(compact)]
		pub participated_frames: u32,
		pub last_frame_earnings_frame_id: Option<FrameId>,
		pub last_frame_earnings: Option<T::Balance>,
		#[codec(compact)]
		pub cumulative_earnings: T::Balance,
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub struct LockedFrameBondTerms {
		#[codec(compact)]
		pub bonds: Bonds,
		pub is_flexible: bool,
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

	/// Per-vault bond totals used for purchase admission and flexible displacement.
	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, Default, TypeInfo, MaxEncodedLen)]
	pub struct VaultBondState {
		/// Regular bond principal admitted against the vault's securitization.
		#[codec(compact)]
		pub regular_bonds: Bonds,
		/// Total active flexible bonds.
		#[codec(compact)]
		pub flexible_bonds: Bonds,
		/// Flexible bonds displaced by regular bonds or missing securitization.
		#[codec(compact)]
		pub displaced_flexible_bonds: Bonds,
		/// Flexible payout terms frozen by the first change in the current frame.
		pub locked_frame_terms: Option<LockedFrameVaultTerms>,
		/// Vault bond space reserved for future bond purchases.
		#[codec(compact)]
		pub reserved_bond_space: Bonds,
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub struct LockedFrameVaultTerms {
		#[codec(compact)]
		pub flexible_bonds: Bonds,
		#[codec(compact)]
		pub displaced_flexible_bonds: Bonds,
	}

	impl VaultBondState {
		pub fn bond_space(&self, bond_capacity: Bonds) -> Bonds {
			bond_capacity.saturating_sub(self.regular_bonds)
		}

		pub fn available_bond_space(&self, bond_capacity: Bonds) -> Bonds {
			self.bond_space(bond_capacity).saturating_sub(self.reserved_bond_space)
		}
	}

	/// The frame-wide locked capital object.
	#[derive(Encode, Decode, PartialEqNoBound, DebugNoBound, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct FrameVaultCapital<T: Config> {
		/// The frame this locked capital object belongs to.
		#[codec(compact)]
		pub frame_id: FrameId,
		/// Active regular and undisplaced flexible bonds across all vaults.
		#[codec(compact)]
		pub total_active_bonds: u128,
		/// Effective network target used for both bond and vault rewards.
		#[codec(compact)]
		pub target_securitization: T::Balance,
		/// Raw securitization across every open vault, including bounded-out vaults.
		#[codec(compact)]
		pub total_securitization: T::Balance,
		/// The top raw-securitization positions eligible for vault payout.
		pub vault_securitization_positions:
			BoundedBTreeMap<VaultId, VaultSecuritizationPosition<T>, T::MaxVaultsPerPool>,
	}

	/// The Argonot bond participants locked for one frame.
	#[derive(Encode, Decode, PartialEqNoBound, DebugNoBound, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct FrameArgonotBondParticipants<T: Config> {
		/// The frame this participant set belongs to.
		#[codec(compact)]
		pub frame_id: FrameId,
		/// The total active bonds sharing the Argonot bond pool for the frame.
		#[codec(compact)]
		pub total_bonds: Bonds,
		/// The Argonot bond lots participating in this frame.
		pub bond_lots: BoundedVec<BondLotSummary, T::MaxActiveArgonotBondLots>,
	}

	#[derive(Encode, Decode, PartialEqNoBound, DebugNoBound, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct VaultSecuritizationPosition<T: Config> {
		pub operator_account_id: T::AccountId,
		#[codec(compact)]
		pub securitization: T::Balance,
		/// Collateral activated by confirmed Bitcoin funding at frame start.
		#[codec(compact)]
		pub activated_securitization: T::Balance,
		/// Locked Bitcoin valued in Argon microgons using frame-start market prices.
		#[codec(compact)]
		pub bitcoin_locked_microgons: T::Balance,
		/// Held Argonots valued in Argon microgons using frame-start market prices.
		#[codec(compact)]
		pub argonot_securitization_in_microgons: T::Balance,
		/// Frame-start participating bond principal used for vault utilization and revenue
		/// metrics.
		#[codec(compact)]
		pub active_bond_microgons: T::Balance,
	}
}
