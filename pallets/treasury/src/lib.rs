#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

pub use argon_primitives::treasury::{BondTotal, Bonds};
use frame_support::traits::{
	fungible::{Balanced, Credit},
	tokens::{Balance as TokenBalance, Imbalance},
};
use pallet_prelude::*;
use sp_arithmetic::Rounding;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
#[cfg(feature = "try-runtime")]
mod try_state;
pub mod weights;
use argon_primitives::{
	treasury::{
		CollectAmounts, CollectSource, PositionQuantities, PositionQuantity,
		TreasuryPositionProvider, TreasuryRewardProvider,
	},
	MiningFrameProvider, OperationalAccountProvider,
};
pub use pallet::*;

/// This pallet lets users buy whole `1 ARGON` bonds into a Vault's Treasury Pool. Treasury pools
/// provide instant liquidity for LockedBitcoins. Each purchase creates a separate bond lot whose
/// principal stays held until release. Earnings are collected separately and do not compound into
/// principal. ARGONOT stakes also earn a share of mining bid revenue.
///
/// Bond lots retain purchase dates, principal and release terms. An account can own multiple lots;
/// Treasury Positions maintains its combined earning quantities and bond/stake checkpoints.
///
/// ## Treasury Pool Allocation
/// A vault can accept whole regular bonds up to its raw Argon securitization. Flexible operator
/// bonds yield their admission capacity to regular bonds. Displaced flexible bonds stop earning
/// from the next frame; regular bonds continue earning even if securitization later falls.
///
/// Treasury freezes earning quantities and vault capital at frame start. Purchases and eligibility
/// changes during that frame affect the next frame's earnings.
///
/// ## Mining Bid Revenue and Collection
/// After each bid pool closes, configured shares fund marketing, Bitcoin Liquids and stabilizer
/// pools. Eligible ARGONOT stakes, ARGON bonds and vaults earn rewards. Unallocated funds go to the
/// liquidity buys reserve.
///
/// Bond and stake earnings are backed in the Treasury Positions custody account and added to global
/// cumulative indexes. Each account settles its share when its position changes or it calls
/// Collect. Frame payouts do not walk bond lots or stake holders. Collect transfers backed earnings
/// to the owner's wallet; it does not add them to principal. Vault earnings retain their bounded
/// vault calculation and separate collection flow.
///
/// The frame's APY policy tapers excess bond and stake rewards into their respective reserves.
/// Excess vault rewards go to liquidity buys. Reserve support is issued into custody when funding
/// committed earnings; auction revenue already in circulation transfers into custody.
///
/// Bond purchases remain subject to:
/// - the maximum number of active ARGONOT bond lots (`MaxActiveArgonotBondLots`)
/// - the maximum active ARGONOT bonds as a percent of ownership circulation
///   (`MaxArgonotBondedPercentOfCirculation`)
/// - the minimum whole-bond purchase amount (`MinimumArgonsPerContributor`)
///
/// Terminology:
/// - a `frame` is one Argon financial day
/// - a `bond` is one `1 ARGON` unit
/// - a `bond lot` is one purchase record containing `N` bonds
/// - a `frame snapshot` holds the capital and earning quantities frozen for that frame
/// - an `earnings index` is the cumulative earnings per eligible bond or stake
/// - an account's `checkpoint` records how far it has settled against that index
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::{boxed::Box, vec::Vec};
	pub use argon_primitives::treasury::Bonds;
	use argon_primitives::{
		bitcoin::{Satoshis, SATOSHIS_PER_BITCOIN},
		providers::PriceProviderWeightInfo,
		treasury::{StabilizerPoolFrameClaim, StabilizerPoolRewardProvider},
		vault::{
			TreasuryBonusApprovalProof, TreasuryVaultProvider, TreasuryVaultProviderWeightInfo,
			VaultSecuritization, VaultTreasuryFrameEarnings,
		},
		AscendingRankKey, Balance, BitcoinMintedProvider, BlockSealAuthorityId, OnNewSlot,
		PriceProvider, TreasuryPoolProvider, MICROGONS_PER_ARGON,
	};
	use pallet_prelude::argon_primitives::{
		MiningFrameTransitionProvider, OperationalAccountsHook, OperationalRewardsPayer,
	};
	use sp_runtime::{
		AccountId32, ArithmeticError, BoundedBTreeMap, FixedU128, Permill, TokenError,
	};
	use tracing::info;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(10);

	pub use argon_primitives::treasury::BondLotId;

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
		type Balance: TokenBalance
			+ AtLeast32BitUnsigned
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
			+ Mutate<Self::AccountId, Balance = Self::Balance>
			+ Balanced<Self::AccountId, Balance = Self::Balance>;

		/// The currency representing ownership tokens (argonots).
		type OwnershipCurrency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>;

		/// The hold reason when reserving funds for treasury bond lots.
		type RuntimeHoldReason: From<HoldReason>;

		/// Provider for vault treasury settings and vault-side earnings collection.
		type TreasuryVaultProvider: TreasuryVaultProvider<
			Currency = Self::Currency,
			Balance = Self::Balance,
			AccountId = Self::AccountId,
		>;

		/// Provider for Bitcoin-minted Argons that have not been explicitly repaid.
		type BitcoinMintedProvider: BitcoinMintedProvider<Self::Balance>;
		type PositionProvider: TreasuryPositionProvider<Self::AccountId, Self::Balance>;
		/// Retain each frame's financial terms through funding and the collection window.
		#[pallet::constant]
		type StabilizerPoolRewardHistoryFrames: Get<u32>;
		type OperationalAccountProvider: OperationalAccountProvider<Self::AccountId>;
		/// Funded upstream Bitcoin securitization, in microgons, required for full Bitcoin credit.
		#[pallet::constant]
		type UpstreamBitcoinTarget: Get<Self::Balance>;
		/// Upstream bond principal, in microgons, required for full bond credit.
		#[pallet::constant]
		type UpstreamBondTarget: Get<Self::Balance>;
		#[pallet::constant]
		type UpstreamBitcoinWeight: Get<Permill>;
		/// Market prices used to value locked Bitcoin and committed Argonots at frame start.
		type PriceProvider: PriceProvider<Self::Balance>;
		/// The minimum whole-bond purchase amount.
		#[pallet::constant]
		type MinimumArgonsPerContributor: Get<Self::Balance>;

		/// The maximum number of active Argonot bond lots.
		#[pallet::constant]
		type MaxActiveArgonotBondLots: Get<u32>;

		/// Maximum concrete lots backing one operator across all of its authorities.
		#[pallet::constant]
		type MaxBondCollateralLots: Get<u32>;

		/// Maximum accounts whose queued bond collateral is attempted in one block.
		#[pallet::constant]
		type MaxBondCollateralSettlementsPerBlock: Get<u32>;

		/// The maximum percent of ownership-token circulation that can be bonded.
		#[pallet::constant]
		type MaxArgonotBondedPercentOfCirculation: Get<Percent>;

		/// Treasury pallet id retained in metadata for account derivation.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Account that receives mining bid funds before frame distribution.
		type MiningBidPoolAccount: Get<Self::AccountId>;

		/// Percent of the bid pool held for marketing rewards.
		#[pallet::constant]
		type PercentForMarketing: Get<Percent>;

		/// Percent of the full bid pool paid to Argonot Stakes.
		#[pallet::constant]
		type PercentForStakePool: Get<Percent>;

		/// Percent reserved for future mining-operator rewards and burned in this release.
		#[pallet::constant]
		type PercentForMiningOperatorPool: Get<Percent>;

		/// Percent credited to the Bitcoin Liquids reserve.
		#[pallet::constant]
		type PercentForBitcoinLiquidPool: Get<Percent>;

		/// Maximum percent paid to Argon bonds when the network target is filled.
		#[pallet::constant]
		type PercentForArgonBondPool: Get<Percent>;

		/// Percent credited to the stabilizer pool rewards reserve.
		#[pallet::constant]
		type PercentForStabilizerPool: Get<Percent>;

		/// Maximum percent paid to vaults when the network target is filled.
		#[pallet::constant]
		type PercentForVaultPool: Get<Percent>;

		/// Minimum vault earnings rate before securitization coverage is applied.
		#[pallet::constant]
		type MinimumVaultProfitPercent: Get<Percent>;

		/// Maximum share of the full bid pool paid for vault Bitcoin participation.
		#[pallet::constant]
		type VaultBitcoinRewardPercent: Get<Permill>;

		/// Maximum rewarded Argonot value as a multiple of the vault's Argon securitization.
		/// The reward weight is configured separately by `ArgonotParticipationFactor`.
		#[pallet::constant]
		type ArgonotSecuritizationMultiplier: Get<u32>;

		/// Default percent of maximum Bitcoin-mintable Argons targeted by the network.
		#[pallet::constant]
		type DefaultTargetBitcoinPercent: Get<Percent>;

		/// Default circulating ARGON fraction used as the stabilizer pool reward reference.
		#[pallet::constant]
		type DefaultTargetStabilizerPoolPercent: Get<Percent>;

		/// The maximum number of vaults that can participate in one frame's locked vault capital.
		#[pallet::constant]
		type MaxVaultsPerPool: Get<u32>;

		/// The maximum number of bond lots whose release delay may mature in a single frame.
		#[pallet::constant]
		type MaxPendingUnlocksPerFrame: Get<u32>;

		/// The number of frames a releasing bond lot remains held before release.
		#[pallet::constant]
		type TreasuryExitDelayFrames: Get<FrameId>;

		/// Provider for mining frame ids and their tick ranges.
		type MiningFrameTransitionProvider: MiningFrameTransitionProvider + MiningFrameProvider;

		/// Optional hook for operational account state updates.
		type OperationalAccountsHook: OperationalAccountsHook<Self::AccountId, Self::Balance>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Funds held for an active or releasing treasury bond lot.
		ContributedToTreasury,
	}

	/// The network bond total and vault capital locked for the current frame's payout.
	#[pallet::storage]
	pub type CurrentFrameVaultCapital<T: Config> =
		StorageValue<_, FrameVaultCapital<T>, OptionQuery>;

	/// Covered Bitcoin locked alongside vault capital for frames using independent reward tiers.
	#[pallet::storage]
	pub type CurrentFrameVaultBitcoin<T: Config> =
		StorageValue<_, FrameVaultBitcoin<T>, OptionQuery>;

	/// Argonot reward weight frozen with the current frame's capital.
	#[pallet::storage]
	pub type CurrentFrameArgonotParticipation<T: Config> =
		StorageValue<_, FrameArgonotParticipation, OptionQuery>;

	#[pallet::type_value]
	pub fn DefaultRewardTaperingPolicy() -> TaperingPolicy {
		// Daily reinvestment rates for 50%, 100%, and 500% APY, at FixedU128 precision.
		let daily_50 = FixedU128::from_inner(1_111_480_547_066_227);
		let daily_100 = FixedU128::from_inner(1_900_837_677_234_846);
		let daily_500 = FixedU128::from_inner(4_920_998_589_597_458);
		TaperingPolicy {
			bitcoin_liquids: TaperingProgram {
				target_daily_return: daily_100,
				excess_reward_payout_percent: Percent::from_percent(50),
				target_daily_return_with_price_below_target: None,
			},
			argon_bonds: TaperingProgram {
				target_daily_return: daily_100,
				excess_reward_payout_percent: Percent::from_percent(50),
				target_daily_return_with_price_below_target: None,
			},
			argonot_stakes: TaperingProgram {
				target_daily_return: daily_50,
				excess_reward_payout_percent: Percent::from_percent(50),
				target_daily_return_with_price_below_target: None,
			},
			vault_revenue: TaperingProgram {
				target_daily_return: daily_500,
				excess_reward_payout_percent: Percent::from_percent(50),
				target_daily_return_with_price_below_target: Some(BelowTargetPriceYield {
					target_daily_return: daily_100,
					argon_price_drop_for_target: Percent::from_percent(10),
				}),
			},
			stabilizer_pools: TaperingProgram {
				target_daily_return: daily_500,
				excess_reward_payout_percent: Percent::from_percent(50),
				target_daily_return_with_price_below_target: None,
			},
		}
	}

	/// Configured daily reward targets and the share paid above each target.
	#[pallet::storage]
	pub type RewardTaperingPolicy<T: Config> =
		StorageValue<_, TaperingPolicy, ValueQuery, DefaultRewardTaperingPolicy>;

	/// Reward settings frozen before a frame starts earning. Prices are measured over that frame.
	#[pallet::storage]
	pub type CurrentFrameRewardTaperingPolicy<T: Config> =
		StorageValue<_, FrameRewardTaperingPolicy, OptionQuery>;

	/// ARGON removed from circulation and available for future issuance as rewards.
	#[pallet::storage]
	pub type RewardReserves<T: Config> =
		StorageValue<_, RewardReserveBalances<T::Balance>, ValueQuery>;

	/// Shared funding and reward rates for each retained stabilizer pool earning frame.
	#[pallet::storage]
	pub type StabilizerPoolRewardFrames<T: Config> =
		StorageMap<_, Twox64Concat, FrameId, StabilizerPoolRewardFrame<T::Balance>, OptionQuery>;

	/// Configurable percent of maximum Bitcoin-mintable Argons targeted by the network.
	#[pallet::storage]
	pub type TargetBitcoinPercent<T: Config> =
		StorageValue<_, Percent, ValueQuery, T::DefaultTargetBitcoinPercent>;

	/// Circulating ARGON fraction used as the value reference for stabilizer pool rewards.
	/// A zero reference leaves the frame's allocation in the stabilizer pool reserve.
	#[pallet::storage]
	pub type TargetStabilizerPoolPercent<T: Config> =
		StorageValue<_, Percent, ValueQuery, T::DefaultTargetStabilizerPoolPercent>;

	#[pallet::type_value]
	pub fn DefaultUpstreamParticipationFactor() -> Percent {
		Percent::from_percent(5)
	}

	/// Share of reward uplift controlled by upstream participation, frozen at frame start.
	/// Zero disables the requirement; 100% applies the full participation score.
	#[pallet::storage]
	pub type UpstreamParticipationFactor<T: Config> =
		StorageValue<_, Percent, ValueQuery, DefaultUpstreamParticipationFactor>;

	/// Share of variable vault earnings controlled by Argonot backing, applied next frame.
	/// Zero disables the requirement; 100% requires full backing for variable earnings.
	#[pallet::storage]
	pub type ArgonotParticipationFactor<T: Config> = StorageValue<_, Percent, ValueQuery>;

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

	/// Live Argon bond lot ids associated with a vault, including lots awaiting release. Principal
	/// operations use this index.
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

	/// Whole non-releasing ARGON bonds by owner and vault, including displaced flexible bonds.
	#[pallet::storage]
	pub type ActiveBondsByAccountAndVault<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		VaultId,
		BondTotal,
		ValueQuery,
	>;

	/// Active lots with unreserved principal. Reservation visits at most the configured lot limit.
	#[pallet::storage]
	pub type AvailableBondCollateralLots<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, BondLotId, (), OptionQuery>;

	/// Concrete backing shared by the operator's minting authorities. The list remains bounded
	/// across registrations; each entry records exact reserved ARGON microgons.
	#[pallet::storage]
	pub type EncumberedBondLotsByAccount<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<BondCollateralLot<T::Balance>, T::MaxBondCollateralLots>,
		ValueQuery,
	>;

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
	)]
	pub struct BondCollateralLot<Balance>
	where
		Balance: Codec + MaxEncodedLen,
	{
		/// Source lot; None retains fractional backing without a surviving lot at cutover.
		pub bond_lot_id: Option<BondLotId>,
		/// Exact held ARGON microgons committed from this lot, including fractional remnants.
		#[codec(compact)]
		pub reserved_microgons: Balance,
	}

	/// Burn and release requests awaiting settlement, combined by owner. Both amounts remain
	/// included in the owner's encumbrance until processed; bonds retain their live payout terms.
	#[pallet::storage]
	pub type PendingBondCollateralByAccount<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, PendingBondCollateral<T::Balance>, OptionQuery>;

	/// Accounts due for collateral settlement, ordered by retry block and owner. Each pending
	/// account has one entry; failures move to the next block without blocking other owners.
	#[pallet::storage]
	pub type BondCollateralSettlementQueue<T: Config> =
		StorageMap<_, Identity, AscendingRankKey<T::AccountId>, (), OptionQuery>;

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

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Reward reserve balances after settlement or a reserve-funded payment.
		RewardReservesUpdated {
			reserves: RewardReserveBalances<T::Balance>,
		},
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
		/// A reserve top-up could not be added to the frame reward funding.
		CouldNotFundReserveTopup {
			frame_id: FrameId,
			amount: T::Balance,
			dispatch_error: DispatchError,
		},
		/// Bid revenue could not be withdrawn for frame distribution.
		CouldNotFundBidPool {
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
			/// ARGON allocated to active Fissions' promised liquidity.
			bitcoin_earnings: T::Balance,
			/// ARGON recorded for vault collection.
			vault_pool_distributed: T::Balance,
			/// Bid-pool funds removed from circulation, including the unfilled Bitcoin share.
			reserved_from_circulation: T::Balance,
			/// Number of vaults included in the frame's reward allocation.
			participating_vaults: u32,
		},

		RewardEconomicsConfigured {
			target_bitcoin_percent: Percent,
			upstream_participation_factor: Percent,
			/// Circulating ARGON fraction used as the stabilizer pool reward reference.
			target_stabilizer_pool_percent: Percent,
			/// Share of variable vault earnings requiring full Argonot backing.
			argonot_participation_factor: Percent,
		},
		RewardTaperingPolicyConfigured {
			policy: Box<TaperingPolicy>,
		},
		/// The current frame's vault capital was locked in.
		FrameVaultCapitalLocked {
			frame_id: FrameId,
			total_active_bonds: BondTotal,
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
		/// Settlement rolled back and remains queued for a later block.
		CouldNotSettleBondCollateral {
			account_id: T::AccountId,
			dispatch_error: DispatchError,
		},
		/// Earnings reserved for the frame; wallet delivery is recorded by Collect.
		FrameEarningsAllocated {
			frame_id: FrameId,
			/// The total gross bid-pool allocation made to non-reserve recipients this frame.
			bid_pool_allocated: T::Balance,
			/// ARGON allocated to admitted stakes.
			stake_earnings: T::Balance,
			/// ARGON allocated to eligible bonds.
			bond_earnings: T::Balance,
			/// ARGON allocated to active Fissions' promised liquidity.
			bitcoin_earnings: T::Balance,
			/// ARGON recorded for vault collection.
			vault_earnings: T::Balance,
			/// Bid-pool funds removed from circulation, including the unfilled Bitcoin share.
			reserved_from_circulation: T::Balance,
			/// Number of vaults included in the frame's reward allocation.
			participating_vaults: u32,
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
		/// A reward target or the price adjustment window is invalid.
		InvalidRewardTaperingPolicy,
		/// The caller does not own the bond lot.
		NotBondLotOwner,
		/// The bond lot is already scheduled for release.
		BondLotAlreadyReleasing,
		/// The vault doesn't have enough raw securitization to support this bond purchase.
		InsufficientBondSpace,
		/// A reservation or liquidation would leave Treasury backing uncovered, or queued
		/// settlement exceeds the account's remaining encumbrance.
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
		/// The supplied list or the operator's resulting backing set exceeds the lot limit.
		TooManyBondCollateralLots,
		/// A caller-supplied collateral selection repeats a lot ID.
		DuplicateBondCollateralLot,
		/// Only active ARGON vault bonds can back a gateway authority.
		InvalidBondCollateralLot,
		/// Pre-upgrade collateral exceeds its selected backing; supply a complete lot selection
		/// with registration before withdrawing principal or processing collateral.
		BondCollateralSelectionRequired,
		/// A requested reward frame has yet to finalize its rate.
		StabilizerPoolClaimNotReady,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let now = now.saturated_into::<u64>();
			let mut weight = Weight::zero();
			let reads = T::WeightInfo::read_bond_collateral_queue();

			for _ in 0..T::MaxBondCollateralSettlementsPerBlock::get() {
				weight.saturating_accrue(reads);
				let Some(key) = BondCollateralSettlementQueue::<T>::iter_keys().next() else {
					break;
				};
				if key.rank() > now {
					break;
				}

				let account = key.holder_id();
				let pending =
					PendingBondCollateralByAccount::<T>::get(&account).unwrap_or_default();
				weight.saturating_accrue(T::DbWeight::get().writes(3));

				let result = with_storage_layer(|| {
					Self::settle_bond_collateral_burn(
						&account,
						pending.burn_microgons,
						&mut weight,
					)?;
					Self::settle_bond_collateral_release(
						&account,
						pending.release_microgons,
						&mut weight,
					)?;
					BondCollateralSettlementQueue::<T>::remove(&key);
					PendingBondCollateralByAccount::<T>::remove(&account);
					Ok(())
				});

				if let Err(dispatch_error) = result {
					BondCollateralSettlementQueue::<T>::remove(key);
					BondCollateralSettlementQueue::<T>::insert(
						AscendingRankKey::new(now.saturating_add(1), account.clone()),
						(),
					);
					Self::deposit_event(Event::CouldNotSettleBondCollateral {
						account_id: account,
						dispatch_error,
					});
				}
			}

			weight
		}

		#[cfg(feature = "try-runtime")]
		fn try_state(_: BlockNumberFor<T>) -> Result<(), sp_runtime::TryRuntimeError> {
			crate::try_state::check::<T>()
		}
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

			let bond_capacity = Self::balance_to_bonds(Self::get_vault_bond_capacity(vault_id));

			let current_frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
			let mut vault_bonds = BondLotsByVault::<T>::get(vault_id);
			let previous_flexible = vault_bonds.eligible_flexible_bonds();
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
			AvailableBondCollateralLots::<T>::insert(&who, bond_lot_id, ());
			Self::update_bond_principal(&who, vault_id, T::Balance::zero(), purchase_amount)?;
			T::PositionProvider::account_quantity_updated(&who, PositionQuantity::Bonds, 0, bonds)?;
			Self::update_vault_displacement(
				vault_id,
				&mut vault_bonds,
				bond_capacity,
				previous_flexible,
			)?;
			BondLotsByVault::<T>::insert(vault_id, vault_bonds);

			Self::deposit_event(Event::<T>::BondLotPurchased {
				program_id: program.id(),
				bond_lot_id,
				account_id: who.clone(),
				bonds,
			});
			BondLotIdsByVault::<T>::insert(vault_id, bond_lot_id, ());
			TotalArgonBondLots::<T>::mutate(|count| count.saturating_accrue(1));
			Self::update_account_vault_bond_total(&who);
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
					let active_balance = T::PositionProvider::bond_principal(&who);
					Self::ensure_collateral_selected(
						&EncumberedBondLotsByAccount::<T>::get(&who),
						Self::encumbered_bond_microgons(&who),
					)?;
					AvailableBondCollateralLots::<T>::remove(&who, bond_lot_id);
					EncumberedBondLotsByAccount::<T>::try_mutate(
						&who,
						|backing| -> DispatchResult {
							let Some(index) = backing
								.iter()
								.position(|entry| entry.bond_lot_id == Some(bond_lot_id))
							else {
								return Ok(())
							};
							let reserved = backing.remove(index).reserved_microgons;
							Self::reserve_bond_collateral(&who, reserved, None, backing)?;
							let excess =
								reserved.saturating_sub(Self::bonds_to_balance(bond_lot.bonds));
							if !excess.is_zero() {
								Self::release_hold::<T::Currency>(&who, excess)?;
							}
							Ok(())
						},
					)?;

					Self::remove_bond_lot_from_vault(vault_id, &bond_lot)?;
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

			let current_total_bonds =
				T::PositionProvider::network_totals().account_quantities.stakes;
			let max_active_bonds = Self::maximum_active_argonot_bonds();
			let active_lots = ArgonotBondLots::<T>::get();
			let active_lot_count = active_lots.len() as u32;
			let mut evicted_bond_lot_id = None;

			let next_total_bonds = if active_lot_count < T::MaxActiveArgonotBondLots::get() {
				current_total_bonds.checked_add(bonds.into()).ok_or(ArithmeticError::Overflow)?
			} else {
				let floor_lot = active_lots.first().ok_or(Error::<T>::InternalError)?;
				ensure!(bonds > floor_lot.bonds, Error::<T>::ArgonotBondPurchaseBelowCutoff);
				evicted_bond_lot_id = Some(floor_lot.bond_lot_id);

				current_total_bonds
					.checked_sub(floor_lot.bonds.into())
					.ok_or(ArithmeticError::Underflow)?
					.checked_add(bonds.into())
					.ok_or(ArithmeticError::Overflow)?
			};
			ensure!(
				next_total_bonds <= max_active_bonds.into(),
				Error::<T>::ArgonotBondPurchaseAboveCap
			);

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
			T::PositionProvider::account_quantity_updated(
				&who,
				PositionQuantity::Stakes,
				0,
				bonds,
			)?;

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
				let previous_flexible = vault_bonds.eligible_flexible_bonds();
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
				T::PositionProvider::account_quantity_updated(
					&who,
					PositionQuantity::Bonds,
					if is_flexible { bond_lot.bonds } else { 0 },
					if is_flexible { 0 } else { bond_lot.bonds },
				)?;
				Self::update_vault_displacement(
					vault_id,
					vault_bonds,
					bond_capacity,
					previous_flexible,
				)?;
				Ok(())
			})?;
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
			upstream_participation_factor: Option<Percent>,
			target_stabilizer_pool_percent: Option<Percent>,
			argonot_participation_factor: Option<Percent>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let current_target_bitcoin_percent = TargetBitcoinPercent::<T>::get();
			if let Some(target_bitcoin_percent) = target_bitcoin_percent {
				TargetBitcoinPercent::<T>::put(target_bitcoin_percent);
			}

			let current_upstream_participation_factor = UpstreamParticipationFactor::<T>::get();
			if let Some(upstream_participation_factor) = upstream_participation_factor {
				UpstreamParticipationFactor::<T>::put(upstream_participation_factor);
			}

			let current_target_stabilizer_pool_percent = TargetStabilizerPoolPercent::<T>::get();
			if let Some(target_stabilizer_pool_percent) = target_stabilizer_pool_percent {
				TargetStabilizerPoolPercent::<T>::put(target_stabilizer_pool_percent);
			}

			let current_argonot_participation_factor = ArgonotParticipationFactor::<T>::get();
			if let Some(argonot_participation_factor) = argonot_participation_factor {
				ArgonotParticipationFactor::<T>::put(argonot_participation_factor);
			}

			Self::deposit_event(Event::RewardEconomicsConfigured {
				target_bitcoin_percent: target_bitcoin_percent
					.unwrap_or(current_target_bitcoin_percent),
				upstream_participation_factor: upstream_participation_factor
					.unwrap_or(current_upstream_participation_factor),
				target_stabilizer_pool_percent: target_stabilizer_pool_percent
					.unwrap_or(current_target_stabilizer_pool_percent),
				argonot_participation_factor: argonot_participation_factor
					.unwrap_or(current_argonot_participation_factor),
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

		/// Update selected reward programs for future frames, preserving omitted programs.
		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::configure_reward_tapering_policy())]
		pub fn configure_reward_tapering_policy(
			origin: OriginFor<T>,
			bitcoin_liquids: Option<Box<TaperingProgram>>,
			argon_bonds: Option<Box<TaperingProgram>>,
			argonot_stakes: Option<Box<TaperingProgram>>,
			vault_revenue: Option<Box<TaperingProgram>>,
			stabilizer_pools: Option<Box<TaperingProgram>>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let mut policy = RewardTaperingPolicy::<T>::get();
			for (program, update) in [
				(&mut policy.bitcoin_liquids, bitcoin_liquids),
				(&mut policy.argon_bonds, argon_bonds),
				(&mut policy.argonot_stakes, argonot_stakes),
				(&mut policy.vault_revenue, vault_revenue),
				(&mut policy.stabilizer_pools, stabilizer_pools),
			] {
				let Some(update) = update else { continue };
				if let Some(low_price) = update.target_daily_return_with_price_below_target {
					ensure!(
						low_price.argon_price_drop_for_target > Percent::zero(),
						Error::<T>::InvalidRewardTaperingPolicy
					);
					ensure!(
						low_price.target_daily_return <= update.target_daily_return,
						Error::<T>::InvalidRewardTaperingPolicy
					);
				}
				*program = *update;
			}
			RewardTaperingPolicy::<T>::put(policy.clone());
			Self::deposit_event(Event::RewardTaperingPolicyConfigured { policy: Box::new(policy) });
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

		pub(crate) fn release_hold<C>(who: &T::AccountId, amount: T::Balance) -> DispatchResult
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

		/// Allocate the completed frame's revenue, fund bond/stake indexes and record vault
		/// earnings. Custody, reserve budgets and indexes commit together because this runs from
		/// a frame hook.
		#[frame_support::transactional]
		pub(crate) fn distribute_bid_pool(frame_id: FrameId) -> DispatchResult {
			if T::PositionProvider::indexed_frame().is_some_and(|done| frame_id <= done) {
				return Ok(())
			}

			let bid_pool_account = T::MiningBidPoolAccount::get();
			Self::ensure_account_provider(&bid_pool_account);
			let full_bid_pool_amount = T::Currency::balance(&bid_pool_account);

			let frame_capital = match CurrentFrameVaultCapital::<T>::take() {
				Some(frame_capital) if frame_capital.frame_id == frame_id => Some(frame_capital),
				Some(frame_capital) => {
					CurrentFrameVaultCapital::<T>::put(frame_capital);
					None
				},
				None => None,
			};
			let frame_bitcoin = match CurrentFrameVaultBitcoin::<T>::take() {
				Some(bitcoin) if bitcoin.frame_id == frame_id => Some(bitcoin),
				Some(bitcoin) => {
					CurrentFrameVaultBitcoin::<T>::put(bitcoin);
					None
				},
				None => None,
			};
			let argonot_participation = match CurrentFrameArgonotParticipation::<T>::take() {
				Some(participation) if participation.frame_id == frame_id => participation.factor,
				Some(participation) => {
					CurrentFrameArgonotParticipation::<T>::put(participation);
					Percent::zero()
				},
				// Frames frozen before configurable participation had no Argonot requirement.
				None => Percent::zero(),
			};
			let frame_tapering = match CurrentFrameRewardTaperingPolicy::<T>::take() {
				Some(policy) if policy.frame_id == frame_id => Some(policy),
				Some(policy) => {
					CurrentFrameRewardTaperingPolicy::<T>::put(policy);
					None
				},
				None => None,
			};
			let policy = frame_tapering.as_ref().map(|frame| &frame.policy);
			let argon_price_ratio =
				T::MiningFrameTransitionProvider::get_tick_range_for_frame(frame_id)
					.map(T::PriceProvider::get_average_cpi_for_ticks)
					.and_then(|cpi| {
						// CPI is target/market - 1; convert the shared frame signal to
						// market/target.
						let ratio =
							FixedI128::one().checked_div(&FixedI128::one().saturating_add(cpi))?;
						Some(FixedU128::from_inner(ratio.into_inner().try_into().ok()?))
					});

			let mut reserves = RewardReserves::<T>::get();
			let reserves_before = reserves.clone();
			let marketing_allocation =
				T::PercentForMarketing::get().mul_floor(full_bid_pool_amount);

			let indexed_earnings_active = frame_id >= T::PositionProvider::earnings_start_frame();
			let quantities = if indexed_earnings_active {
				frame_capital.as_ref().map(|capital| capital.quantities).unwrap_or_default()
			} else {
				PositionQuantities::default()
			};
			let stake_pool = T::PercentForStakePool::get().mul_floor(full_bid_pool_amount);
			let average_price = T::PriceProvider::get_average_microgons_per_argonot(frame_id);
			let stake_capital =
				average_price.map(|price| quantities.stakes.saturating_mul(price.into()));
			let stake_payout = if quantities.stakes > 0 {
				Self::calculate_pool_allocation(
					stake_pool,
					stake_capital,
					policy.map(|policy| &policy.argonot_stakes),
					argon_price_ratio,
					reserves.argonot_stakes,
				)
			} else {
				PoolAllocation::default()
			};

			let bitcoin_target = frame_capital
				.as_ref()
				.map(|capital| capital.target_securitization.into())
				.unwrap_or_default();
			let bond_capital = quantities.bonds.saturating_mul(MICROGONS_PER_ARGON);
			let bond_pool = T::PercentForArgonBondPool::get().mul_floor(full_bid_pool_amount);
			let bond_allocation = Self::earned_pool_share(bond_pool, bond_capital, bitcoin_target);

			let bond_payout = Self::calculate_pool_allocation(
				bond_allocation,
				Some(bond_capital),
				policy.map(|policy| &policy.argon_bonds),
				argon_price_ratio,
				reserves.argon_bonds,
			);

			let bitcoin_pool =
				T::PercentForBitcoinLiquidPool::get().mul_floor(full_bid_pool_amount);
			let promised_liquidity = quantities.fission_liquidity.into();
			let bitcoin_allocation =
				Self::earned_pool_share(bitcoin_pool, promised_liquidity, bitcoin_target);

			let bitcoin_payout = Self::calculate_pool_allocation(
				bitcoin_allocation,
				Some(promised_liquidity),
				policy.map(|policy| &policy.bitcoin_liquids),
				argon_price_ratio,
				reserves.bitcoin_liquids,
			);

			let from_pool = CollectAmounts {
				bond_earnings: bond_payout.from_pool,
				bitcoin_earnings: bitcoin_payout.from_pool,
				stake_earnings: stake_payout.from_pool,
				..Default::default()
			};
			let from_reserve = CollectAmounts {
				bond_earnings: bond_payout.from_reserve,
				stake_earnings: stake_payout.from_reserve,
				bitcoin_earnings: bitcoin_payout.from_reserve,
				..Default::default()
			};
			reserves.bitcoin_liquids.saturating_reduce(from_reserve.bitcoin_earnings);
			reserves.argon_bonds.saturating_reduce(from_reserve.bond_earnings);
			reserves.argonot_stakes.saturating_reduce(from_reserve.stake_earnings);

			let mut funding = T::Currency::withdraw(
				&bid_pool_account,
				full_bid_pool_amount,
				Precision::Exact,
				Preservation::Expendable,
				Fortitude::Force,
			)?;
			let auction_earnings = funding.extract(from_pool.total());

			let (vault_earnings, participating_vaults) = Self::distribute_vault_pool(
				&mut funding,
				full_bid_pool_amount,
				frame_capital.as_ref(),
				frame_bitcoin.as_ref(),
				policy,
				argon_price_ratio,
				argonot_participation,
			);

			let stabilizer_pool_allocation =
				T::PercentForStabilizerPool::get().mul_floor(full_bid_pool_amount);
			let mining_operator_allocation =
				T::PercentForMiningOperatorPool::get().mul_floor(full_bid_pool_amount);
			let mut unallocated =
				funding.peek().saturating_sub(bitcoin_pool.saturating_sub(bitcoin_allocation));
			// Dropping unspent funding removes it from issuance exactly once.
			drop(funding);

			let reserved_from_circulation = full_bid_pool_amount
				.saturating_sub(from_pool.total())
				.saturating_sub(vault_earnings);
			for (allocation, budget) in [
				(marketing_allocation, &mut reserves.marketing),
				(bitcoin_payout.withheld, &mut reserves.bitcoin_liquids),
				(stabilizer_pool_allocation, &mut reserves.stabilizer_pools),
				(stake_payout.withheld, &mut reserves.argonot_stakes),
				(bond_payout.withheld, &mut reserves.argon_bonds),
				(mining_operator_allocation, &mut reserves.liquidity_buys),
			] {
				let funded = allocation.min(unallocated);
				budget.saturating_accrue(funded);
				unallocated.saturating_reduce(funded);
			}
			// Vault taper excess, unfilled allocations, and payout rounding fund liquidity buys.
			reserves.liquidity_buys.saturating_accrue(unallocated);

			Self::close_stabilizer_pool_reward_frame(
				frame_id,
				reserves.stabilizer_pools.saturating_sub(reserves_before.stabilizer_pools),
				policy.map(|policy| &policy.stabilizer_pools),
				argon_price_ratio,
			);

			if reserves != reserves_before {
				RewardReserves::<T>::put(&reserves);
				Self::deposit_event(Event::<T>::RewardReservesUpdated { reserves });
			}

			let mut earnings = from_pool;
			earnings.saturating_accrue(&from_reserve);
			if !earnings.total().is_zero() {
				T::PositionProvider::prepare_collect()?;
				let collect_account =
					T::PositionProvider::collect_account().ok_or(DispatchError::Unavailable)?;
				T::Currency::resolve(&collect_account, auction_earnings)
					.map_err(|_| Error::<T>::InternalError)?;
				T::Currency::mint_into(&collect_account, from_reserve.total())?;
			}
			if indexed_earnings_active {
				T::PositionProvider::allocate_frame_earnings(frame_id, &quantities, earnings)?;
			}

			Self::deposit_event(Event::<T>::FrameEarningsAllocated {
				frame_id,
				bid_pool_allocated: from_pool.total().saturating_add(vault_earnings),
				stake_earnings: earnings.stake_earnings,
				bond_earnings: earnings.bond_earnings,
				bitcoin_earnings: earnings.bitcoin_earnings,
				vault_earnings,
				reserved_from_circulation,
				participating_vaults,
			});
			Ok(())
		}

		/// Freeze vault inputs and completed position totals for a frame.
		pub(crate) fn lock_in_vault_capital(frame_id: FrameId) {
			// Config changes during this frame apply only to the next frame's rewards.
			CurrentFrameRewardTaperingPolicy::<T>::put(FrameRewardTaperingPolicy {
				frame_id,
				policy: RewardTaperingPolicy::<T>::get(),
			});
			let argonot_participation_factor = ArgonotParticipationFactor::<T>::get();
			let max_vaults = T::MaxVaultsPerPool::get() as usize;
			let network_totals = T::PositionProvider::network_totals();
			let total_active_bonds = network_totals.account_quantities.bonds;

			let (securitization_positions, total_securitization) =
				T::TreasuryVaultProvider::get_top_vaults_by_securitization(max_vaults as u32);

			let mut vault_securitization_positions = BoundedBTreeMap::new();
			let mut vault_securitized_satoshis = BoundedBTreeMap::new();
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
				securitized_satoshis,
				securitization_micronots,
			} in securitization_positions
			{
				let _ = vault_securitized_satoshis.try_insert(vault_id, securitized_satoshis);
				let argonot_capacity = FixedU128::saturating_from_integer(securitization.into())
					.saturating_mul(FixedU128::from_u32(T::ArgonotSecuritizationMultiplier::get()));
				let required_micronots = argonot_capacity
					.checked_div(&microgons_per_micronot)
					.unwrap_or_default()
					.ceil()
					.saturating_mul_int(1u128);
				let reward_micronots = if argonot_participation_factor.is_zero() {
					T::Balance::zero()
				} else {
					securitization_micronots.min(required_micronots.into())
				};
				T::TreasuryVaultProvider::commit_securitization_for_rewards(
					vault_id,
					reward_micronots,
				);
				let vault_bonds = BondLotsByVault::<T>::get(vault_id);
				let active_bonds = vault_bonds.regular_bonds.saturating_add(
					vault_bonds.flexible_bonds.saturating_sub(vault_bonds.displaced_flexible_bonds),
				);
				let upstream_participation = Self::upstream_participation(&operator_account_id);
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
						upstream_participation,
					},
				);
			}
			let participating_vaults = vault_securitization_positions.len() as u32;

			let bitcoin_minted = T::BitcoinMintedProvider::minted_bitcoin_microgons();
			let mining_minted = T::Currency::total_issuance().saturating_sub(bitcoin_minted);
			let target_securitization =
				TargetBitcoinPercent::<T>::get().mul_floor(mining_minted).max(bitcoin_minted);
			let target_securitized_satoshis =
				match T::PriceProvider::get_btc_price_in_market_microgons(SATOSHIS_PER_BITCOIN) {
					Some(price) if !price.is_zero() => Some(
						FixedU128::from_rational(target_securitization.into(), price.into())
							.saturating_mul_int(SATOSHIS_PER_BITCOIN as u128),
					),
					_ => None,
				};
			CurrentFrameArgonotParticipation::<T>::put(FrameArgonotParticipation {
				frame_id,
				factor: argonot_participation_factor,
			});
			CurrentFrameVaultBitcoin::<T>::put(FrameVaultBitcoin {
				frame_id,
				total_network_securitized_satoshis: network_totals.vault_securitized_satoshis,
				target_securitized_satoshis,
				vault_securitized_satoshis,
			});
			CurrentFrameVaultCapital::<T>::put(FrameVaultCapital {
				frame_id,
				total_active_bonds,
				quantities: network_totals.account_quantities,
				target_securitization,
				total_securitization,
				vault_securitization_positions,
			});

			Self::deposit_event(Event::<T>::FrameVaultCapitalLocked {
				frame_id,
				total_active_bonds,
				participating_vaults,
			});
		}

		pub(crate) fn price_adjusted_daily_return(
			program: &TaperingProgram,
			price_ratio: Option<FixedU128>,
		) -> FixedU128 {
			let target = program.target_daily_return;
			let Some(low_price) = program.target_daily_return_with_price_below_target else {
				return target
			};
			let Some(price_ratio) = price_ratio else { return target };
			let shortfall = FixedU128::one().saturating_sub(price_ratio);
			let price_drop_for_target = FixedU128::from_rational(
				low_price.argon_price_drop_for_target.deconstruct() as u128,
				100,
			);
			if price_drop_for_target.is_zero() || shortfall.is_zero() {
				return target;
			}
			if shortfall >= price_drop_for_target {
				return low_price.target_daily_return;
			}

			let fraction = price_drop_for_target
				.saturating_sub(shortfall)
				.checked_div(&price_drop_for_target)
				.unwrap_or_default();
			low_price.target_daily_return.saturating_add(
				target.saturating_sub(low_price.target_daily_return).saturating_mul(fraction),
			)
		}

		pub(crate) fn tapered_payout(
			proposed: u128,
			capital: u128,
			daily_return: FixedU128,
			excess_reward_payout_percent: Percent,
		) -> u128 {
			if capital == 0 {
				return 0;
			}
			let threshold = daily_return.saturating_mul_int(capital);
			let excess = proposed.saturating_sub(threshold);
			proposed - excess + excess_reward_payout_percent.mul_floor(excess)
		}

		/// Return the reward multiplier for funded participation against nominal runtime targets.
		/// Admission capacity and vault availability do not reduce either asset's requirement.
		pub fn upstream_participation(operator: &T::AccountId) -> FixedU128 {
			// Accounts without an upstream vault have no participation requirement.
			let Some((account, upstream_vault_id)) =
				T::OperationalAccountProvider::upstream_vault(operator)
			else {
				return FixedU128::one();
			};

			let position = T::PositionProvider::upstream_position(&account)
				.filter(|p| p.vault_id == upstream_vault_id)
				.unwrap_or_default();

			let required_bitcoin_securitization = T::UpstreamBitcoinTarget::get();
			let required_bond_principal = T::UpstreamBondTarget::get();

			// Only a zero target grants automatic credit; excess cannot cover the other asset.
			let bitcoin_score = if required_bitcoin_securitization.is_zero() {
				FixedU128::one()
			} else {
				let participating =
					position.bitcoin_securitization.min(required_bitcoin_securitization);
				FixedU128::from_rational(
					participating.into(),
					required_bitcoin_securitization.into(),
				)
			};

			let bond_score = if required_bond_principal.is_zero() {
				FixedU128::one()
			} else {
				let participating = position.bond_principal.min(required_bond_principal);
				FixedU128::from_rational(participating.into(), required_bond_principal.into())
			};

			let bitcoin_weight = FixedU128::from_rational(
				T::UpstreamBitcoinWeight::get().deconstruct() as u128,
				1_000_000,
			);
			let bond_weight = FixedU128::one().saturating_sub(bitcoin_weight);
			let bitcoin_participation = bitcoin_score.saturating_mul(bitcoin_weight);
			let bond_participation = bond_score.saturating_mul(bond_weight);

			let participation = bitcoin_participation.saturating_add(bond_participation);
			let factor = FixedU128::from_rational(
				UpstreamParticipationFactor::<T>::get().deconstruct() as u128,
				100,
			);

			FixedU128::one()
				.saturating_sub(factor)
				.saturating_add(factor.saturating_mul(participation))
		}

		/// Fund the completed frame and fix the new frame's financial terms.
		pub(crate) fn run_frame_transition(frame_id: FrameId) {
			if frame_id == 0 {
				return;
			}

			let payout_frame = frame_id - 1;
			info!("Starting treasury bond frame {frame_id}. Distributing frame {payout_frame}.");
			if let Err(error) = Self::distribute_bid_pool(payout_frame) {
				log::error!("Could not allocate treasury frame {payout_frame}: {error:?}");
			}

			let retained = FrameId::from(T::StabilizerPoolRewardHistoryFrames::get());
			if let Some(expired) = frame_id.checked_sub(retained.saturating_add(1)) {
				StabilizerPoolRewardFrames::<T>::remove(expired);
			}
			Self::release_pending_bond_lots(frame_id);
			Self::lock_in_vault_capital(frame_id);

			if !StabilizerPoolRewardFrames::<T>::contains_key(frame_id) {
				let circulation = T::PriceProvider::get_circulation();
				let target_pool_value_microgons =
					TargetStabilizerPoolPercent::<T>::get().mul_floor(circulation);
				StabilizerPoolRewardFrames::<T>::insert(
					frame_id,
					StabilizerPoolRewardFrame::Earning { target_pool_value_microgons },
				);
			}
		}

		fn earned_pool_share(pool: T::Balance, earning_quantity: u128, target: u128) -> T::Balance {
			let denominator = target.max(earning_quantity);
			if denominator == 0 {
				return T::Balance::zero()
			}

			FixedU128::from_rational_with_rounding(earning_quantity, denominator, Rounding::Down)
				.saturating_mul_int(pool)
		}

		fn calculate_pool_allocation(
			pool: T::Balance,
			capital: Option<u128>,
			policy: Option<&TaperingProgram>,
			price_ratio: Option<FixedU128>,
			reserve: T::Balance,
		) -> PoolAllocation<T::Balance> {
			let mut allocated = pool;
			let mut topup = T::Balance::zero();
			if let (Some(program), Some(capital)) = (policy, capital) {
				let daily_return = Self::price_adjusted_daily_return(program, price_ratio);
				allocated = Self::tapered_payout(
					pool.into(),
					capital,
					daily_return,
					program.excess_reward_payout_percent,
				)
				.into();
				if price_ratio.is_some_and(|ratio| ratio < FixedU128::one()) {
					topup = daily_return
						.saturating_mul_int(capital)
						.saturating_sub(pool.into())
						.min(reserve.into())
						.into();
				}
			}
			PoolAllocation {
				from_pool: allocated,
				from_reserve: topup,
				withheld: pool.saturating_sub(allocated),
			}
		}

		fn distribute_vault_pool(
			funding: &mut RewardFunding<T>,
			full_bid_pool_amount: T::Balance,
			frame_capital: Option<&FrameVaultCapital<T>>,
			frame_bitcoin: Option<&FrameVaultBitcoin<T>>,
			policy: Option<&TaperingPolicy>,
			argon_price_ratio: Option<FixedU128>,
			argonot_participation_factor: Percent,
		) -> (T::Balance, u32) {
			let Some(frame_capital) = frame_capital else {
				return (T::Balance::zero(), 0);
			};
			let frame_id = frame_capital.frame_id;
			let Some(frame_bitcoin) = frame_bitcoin else {
				return Self::distribute_runtime_160_vault_pool(
					frame_id,
					funding,
					full_bid_pool_amount,
					frame_capital,
				);
			};

			let securitization_denominator =
				frame_capital.target_securitization.max(frame_capital.total_securitization);
			let bond_denominator = frame_capital
				.target_securitization
				.max(frame_capital.total_active_bonds.saturating_mul(MICROGONS_PER_ARGON).into());
			let bitcoin_denominator = frame_bitcoin
				.target_securitized_satoshis
				.map(|target| target.max(frame_bitcoin.total_network_securitized_satoshis))
				.filter(|denominator| *denominator > 0);

			let securitization_rate = FixedU128::from_rational(
				T::MinimumVaultProfitPercent::get().deconstruct() as u128,
				100,
			);
			let vault_rate =
				FixedU128::from_rational(T::PercentForVaultPool::get().deconstruct() as u128, 100);
			// Securitization earns 5%, Bitcoin 42.835%, and bonds the remaining 3.165%.
			let variable_rate = vault_rate.saturating_sub(securitization_rate);
			let bitcoin_rate =
				FixedU128::from(T::VaultBitcoinRewardPercent::get()).min(variable_rate);
			let bond_rate = variable_rate.saturating_sub(bitcoin_rate);

			let argonot_multiplier = T::ArgonotSecuritizationMultiplier::get();
			let argonot_weight =
				FixedU128::from_rational(argonot_participation_factor.deconstruct() as u128, 100);

			let tapering = policy.map(|policy| {
				let program = &policy.vault_revenue;
				(
					Self::price_adjusted_daily_return(program, argon_price_ratio),
					program.excess_reward_payout_percent,
				)
			});
			let mut distributed = T::Balance::zero();
			for (vault_id, position) in frame_capital.vault_securitization_positions.iter() {
				let securitization: u128 = position.securitization.into();
				if securitization == 0 {
					continue;
				}

				let securitization_share = FixedU128::from_rational_with_rounding(
					securitization,
					securitization_denominator.into(),
					Rounding::Down,
				);
				let bitcoin_share = if let Some(denominator) = bitcoin_denominator {
					let covered_satoshis = frame_bitcoin
						.vault_securitized_satoshis
						.get(vault_id)
						.copied()
						.unwrap_or_default();
					FixedU128::from_rational_with_rounding(
						covered_satoshis as u128,
						denominator,
						Rounding::Down,
					)
				} else {
					FixedU128::zero()
				};
				let bond_share = if bond_denominator.is_zero() {
					FixedU128::zero()
				} else {
					FixedU128::from_rational_with_rounding(
						position.active_bond_microgons.into(),
						bond_denominator.into(),
						Rounding::Down,
					)
				};

				let excess_bitcoin_value: u128 = position
					.bitcoin_locked_microgons
					.saturating_sub(position.securitization)
					.into();
				// The extra Bitcoin reward cut reaches 20% when BTC value reaches 2x
				// securitization.
				let excess_bitcoin_ratio = FixedU128::from_rational(
					excess_bitcoin_value.min(securitization),
					securitization,
				);
				let bitcoin_reward_factor = FixedU128::one().saturating_sub(
					excess_bitcoin_ratio.saturating_mul(FixedU128::from_rational(1, 5)),
				);

				let argonot_capacity = securitization.saturating_mul(argonot_multiplier.into());
				// Disabled participation commits no Argonots, so they cannot raise taper capital.
				let argonot_value = if argonot_participation_factor.is_zero() {
					0
				} else {
					let value: u128 = position.argonot_securitization_in_microgons.into();
					value.min(argonot_capacity)
				};
				let argonot_utilization = if argonot_capacity == 0 {
					FixedU128::one()
				} else {
					FixedU128::from_rational(argonot_value, argonot_capacity)
				};
				let argonot_participation = FixedU128::one()
					.saturating_sub(argonot_weight)
					.saturating_add(argonot_weight.saturating_mul(argonot_utilization));

				let bitcoin_earnings_rate = bitcoin_share
					.saturating_mul(bitcoin_rate)
					.saturating_mul(bitcoin_reward_factor);
				let bond_earnings_rate = bond_share.saturating_mul(bond_rate);
				let variable_earnings_rate = bitcoin_earnings_rate
					.saturating_add(bond_earnings_rate)
					.saturating_mul(argonot_participation)
					.saturating_mul(position.upstream_participation);
				let earnings_rate = securitization_share
					.saturating_mul(securitization_rate)
					.saturating_add(variable_earnings_rate);
				let proposed = earnings_rate.saturating_mul_int(full_bid_pool_amount);
				let earnings = if let Some((daily_return, excess_reward_payout_percent)) = tapering
				{
					Self::tapered_payout(
						proposed.into(),
						securitization.saturating_add(argonot_value),
						daily_return,
						excess_reward_payout_percent,
					)
					.into()
				} else {
					proposed
				};
				if Self::record_vault_pool_earnings(
					frame_id, funding, *vault_id, position, earnings,
				)
				.is_ok()
				{
					distributed.saturating_accrue(earnings);
				}
			}

			(distributed, frame_capital.vault_securitization_positions.len() as u32)
		}

		fn distribute_runtime_160_vault_pool(
			frame_id: FrameId,
			funding: &mut RewardFunding<T>,
			full_bid_pool_amount: T::Balance,
			frame_capital: &FrameVaultCapital<T>,
		) -> (T::Balance, u32) {
			let denominator =
				frame_capital.target_securitization.max(frame_capital.total_securitization);
			// Snapshots store positions, not rates; settle runtime 160 at its original 1% floor.
			let runtime_160_profit_floor = FixedU128::from_rational(1, 100);
			let maximum_profit_rate =
				FixedU128::from_rational(T::PercentForVaultPool::get().deconstruct() as u128, 100);

			// Runtime 160 froze these frames while Argonot backing was disabled.
			let argonot_securitization_multiplier = 0u32;
			let argonot_bonus_rate = if argonot_securitization_multiplier == 0 {
				FixedU128::zero()
			} else {
				FixedU128::from_rational(29, 100)
			};
			let maximum_capital_multiplier = FixedU128::one()
				.saturating_add(FixedU128::from_u32(argonot_securitization_multiplier));
			let maximum_argonot_bonus = FixedU128::one().saturating_add(argonot_bonus_rate);
			// Scale the core rate so maximum capital backing and bonuses reach the vault pool cap.
			let maximum_core_profit_rate = maximum_profit_rate
				.checked_div(&maximum_capital_multiplier.saturating_mul(maximum_argonot_bonus))
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
					securitization.saturating_mul(argonot_securitization_multiplier.into());
				let argonot_value: u128 = position.argonot_securitization_in_microgons.into();
				let argonot_value = argonot_value.min(argonot_capacity);
				let argonot_utilization = if argonot_capacity == 0 {
					FixedU128::zero()
				} else {
					FixedU128::from_rational(argonot_value, argonot_capacity)
				};
				let core_utilization = bitcoin_utilization.saturating_mul(
					FixedU128::from_rational(9, 10).saturating_add(
						FixedU128::from_rational(1, 10).saturating_mul(bond_utilization),
					),
				);
				let core_profit_rate =
					runtime_160_profit_floor.saturating_add(core_utilization.saturating_mul(
						maximum_core_profit_rate.saturating_sub(runtime_160_profit_floor),
					));
				let capital_multiplier = FixedU128::from_rational(
					securitization.saturating_add(argonot_value),
					securitization,
				);
				let argonot_bonus = FixedU128::one().saturating_add(
					argonot_bonus_rate
						.saturating_mul(argonot_utilization)
						.saturating_mul(bitcoin_utilization),
				);
				let profit_rate = core_profit_rate
					.saturating_mul(capital_multiplier)
					.saturating_mul(argonot_bonus)
					.min(maximum_profit_rate);
				let profit_rate = runtime_160_profit_floor.saturating_add(
					position
						.upstream_participation
						.saturating_mul(profit_rate.saturating_sub(runtime_160_profit_floor)),
				);
				let coverage = FixedU128::from_rational_with_rounding(
					securitization,
					denominator.into(),
					Rounding::Down,
				);
				let earnings =
					coverage.saturating_mul(profit_rate).saturating_mul_int(full_bid_pool_amount);

				if Self::record_vault_pool_earnings(
					frame_id, funding, *vault_id, position, earnings,
				)
				.is_ok()
				{
					distributed.saturating_accrue(earnings);
				}
			}

			(distributed, frame_capital.vault_securitization_positions.len() as u32)
		}

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

		fn record_vault_pool_earnings(
			frame_id: FrameId,
			funding: &mut RewardFunding<T>,
			vault_id: VaultId,
			position: &VaultSecuritizationPosition<T>,
			earnings: T::Balance,
		) -> DispatchResult {
			T::TreasuryVaultProvider::record_vault_frame_earnings(
				funding,
				VaultTreasuryFrameEarnings {
					vault_id,
					vault_operator_account_id: position.operator_account_id.clone(),
					frame_id,
					earnings_for_vault: earnings,
					earnings,
					capital_contributed: position.active_bond_microgons,
					capital_contributed_by_vault: T::Balance::zero(),
				},
			)
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

		/// Flexible lots all belong to the operator. Replace the operator's aggregate admitted
		/// quantity when regular purchases, flexibility, releases or capital change displacement.
		fn update_vault_displacement(
			vault_id: VaultId,
			vault_bonds: &mut VaultBondState,
			capacity: Bonds,
			previous_flexible: Bonds,
		) -> DispatchResult {
			let flexible_capacity = capacity.saturating_sub(vault_bonds.regular_bonds);
			vault_bonds.displaced_flexible_bonds =
				vault_bonds.flexible_bonds.saturating_sub(flexible_capacity);
			let eligible_flexible = vault_bonds.eligible_flexible_bonds();
			if previous_flexible != eligible_flexible {
				let operator = T::TreasuryVaultProvider::get_vault_operator(vault_id)
					.ok_or(Error::<T>::InternalError)?;
				T::PositionProvider::account_quantity_updated(
					&operator,
					PositionQuantity::Bonds,
					previous_flexible,
					eligible_flexible,
				)?;
			}
			Ok(())
		}

		pub(crate) fn update_bond_principal(
			account: &T::AccountId,
			vault: VaultId,
			previous: T::Balance,
			current: T::Balance,
		) -> DispatchResult {
			ActiveBondsByAccountAndVault::<T>::mutate_exists(account, vault, |total| {
				let updated = total
					.unwrap_or_default()
					.saturating_sub(previous.into() / MICROGONS_PER_ARGON)
					.saturating_add(current.into() / MICROGONS_PER_ARGON);
				*total = if updated == 0 { None } else { Some(updated) };
			});
			T::PositionProvider::bond_position_updated(account, vault, previous, current)
		}

		pub(crate) fn encumbered_bond_microgons(account_id: &T::AccountId) -> T::Balance {
			EncumberedBondMicrogonsByAccount::<T>::get(account_id)
		}

		fn maximum_active_argonot_bonds() -> Bonds {
			let circulation = T::OwnershipCurrency::total_issuance();
			let cap_balance = T::MaxArgonotBondedPercentOfCirculation::get().mul_floor(circulation);
			Self::balance_to_bonds(cap_balance)
		}

		fn remove_bond_lot_from_vault(vault_id: VaultId, bond_lot: &BondLot<T>) -> DispatchResult {
			BondLotsByVault::<T>::try_mutate_exists(
				vault_id,
				|maybe_vault_bonds| -> DispatchResult {
					let Some(vault_bonds) = maybe_vault_bonds.as_mut() else {
						return Ok(());
					};
					let previous_flexible = vault_bonds.eligible_flexible_bonds();
					if bond_lot.is_flexible {
						vault_bonds.flexible_bonds.saturating_reduce(bond_lot.bonds);
					} else {
						vault_bonds.regular_bonds.saturating_reduce(bond_lot.bonds);
						T::PositionProvider::account_quantity_updated(
							&bond_lot.owner,
							PositionQuantity::Bonds,
							bond_lot.bonds,
							0,
						)?;
					}
					Self::update_vault_displacement(
						vault_id,
						vault_bonds,
						Self::balance_to_bonds(Self::get_vault_bond_capacity(vault_id)),
						previous_flexible,
					)?;
					if vault_bonds.regular_bonds.is_zero() &&
						vault_bonds.flexible_bonds.is_zero() &&
						vault_bonds.reserved_bond_space.is_zero()
					{
						*maybe_vault_bonds = None;
					}
					Ok(())
				},
			)
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
				if matches!(bond_lot.program, BondProgram::Argonot) {
					T::PositionProvider::account_quantity_updated(
						&bond_lot.owner,
						PositionQuantity::Stakes,
						bond_lot.bonds,
						0,
					)?;
				}
				if let BondProgram::Vault { vault_id, .. } = bond_lot.program {
					Self::update_bond_principal(
						&bond_lot.owner,
						vault_id,
						Self::bonds_to_balance(bond_lot.bonds),
						T::Balance::zero(),
					)?;
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

		fn close_stabilizer_pool_reward_frame(
			frame_id: FrameId,
			funded: T::Balance,
			policy: Option<&TaperingProgram>,
			price_ratio: Option<FixedU128>,
		) {
			let Some(StabilizerPoolRewardFrame::Earning { target_pool_value_microgons }) =
				StabilizerPoolRewardFrames::<T>::get(frame_id)
			else {
				return;
			};
			let budget = policy.map_or(funded, |program| {
				Self::tapered_payout(
					funded.into(),
					target_pool_value_microgons.into(),
					Self::price_adjusted_daily_return(program, price_ratio),
					program.excess_reward_payout_percent,
				)
				.into()
			});
			StabilizerPoolRewardFrames::<T>::insert(
				frame_id,
				StabilizerPoolRewardFrame::Allocated { target_pool_value_microgons, budget },
			);
		}

		fn update_account_vault_bond_total(account_id: &T::AccountId) {
			let active_account_vault_bond_amount = T::PositionProvider::bond_principal(account_id);
			T::OperationalAccountsHook::account_vault_bond_total_updated(
				account_id,
				active_account_vault_bond_amount,
			);
		}
	}

	impl<T: Config> StabilizerPoolRewardProvider for Pallet<T> {
		type Weights = weights::ProviderWeightAdapter<T>;

		fn finalize_frame_rewards(frame_id: FrameId, eligible_value: Balance) {
			let mut reward_rate = FixedU128::zero();
			match StabilizerPoolRewardFrames::<T>::get(frame_id) {
				Some(StabilizerPoolRewardFrame::Allocated {
					target_pool_value_microgons,
					budget,
				}) =>
					if !eligible_value.is_zero() && !target_pool_value_microgons.is_zero() {
						let denominator = eligible_value.max(target_pool_value_microgons.into());
						reward_rate =
							FixedU128::saturating_from_rational(budget.into(), denominator);
					},
				// A closed frame with no auction funding has no earnings.
				Some(StabilizerPoolRewardFrame::Earning { .. }) => {},
				_ => return,
			}
			StabilizerPoolRewardFrames::<T>::insert(
				frame_id,
				StabilizerPoolRewardFrame::Finalized { reward_rate },
			);
		}

		fn frame_reward_rate(frame_id: FrameId) -> Option<FixedU128> {
			let StabilizerPoolRewardFrame::Finalized { reward_rate } =
				StabilizerPoolRewardFrames::<T>::get(frame_id)?
			else {
				return None;
			};
			Some(reward_rate)
		}

		fn claim_rewards(claims: &[StabilizerPoolFrameClaim]) -> Result<Balance, DispatchError> {
			let mut total = 0u128;
			for claim in claims {
				ensure!(
					Self::frame_reward_rate(claim.frame_id).is_some(),
					Error::<T>::StabilizerPoolClaimNotReady
				);
				total = total.saturating_add(claim.amount);
			}
			if total.is_zero() {
				return Ok(0)
			}

			let amount = total.into();
			ensure!(
				amount <= RewardReserves::<T>::get().stabilizer_pools,
				TokenError::FundsUnavailable
			);
			T::PositionProvider::fund(CollectSource::StabilizerPools, amount)?;

			RewardReserves::<T>::mutate(|reserves| {
				reserves.stabilizer_pools = reserves.stabilizer_pools.saturating_sub(amount);
				Self::deposit_event(Event::<T>::RewardReservesUpdated {
					reserves: reserves.clone(),
				});
			});
			Ok(total)
		}
	}

	impl<T: Config> TreasuryRewardProvider for Pallet<T> {
		type Balance = T::Balance;
		type Weights = weights::ProviderWeightAdapter<T>;

		fn available_budget() -> T::Balance {
			RewardReserves::<T>::get().liquidity_buys
		}

		fn reserve_budget(amount: T::Balance) -> DispatchResult {
			if amount.is_zero() {
				return Ok(());
			}
			let mut reserves = RewardReserves::<T>::get();
			ensure!(amount <= reserves.liquidity_buys, TokenError::FundsUnavailable);
			reserves.liquidity_buys.saturating_reduce(amount);
			RewardReserves::<T>::put(&reserves);
			Self::deposit_event(Event::<T>::RewardReservesUpdated { reserves });
			Ok(())
		}

		fn return_budget(amount: T::Balance) -> DispatchResult {
			if amount.is_zero() {
				return Ok(());
			}
			let mut reserves = RewardReserves::<T>::get();
			reserves.liquidity_buys.saturating_accrue(amount);
			RewardReserves::<T>::put(&reserves);
			Self::deposit_event(Event::<T>::RewardReservesUpdated { reserves });
			Ok(())
		}
	}

	impl<T: Config> OperationalRewardsPayer<T::AccountId, T::Balance> for Pallet<T> {
		type Weights = weights::ProviderWeightAdapter<T>;

		fn claim_reward(account_id: &T::AccountId, amount: T::Balance) -> DispatchResult {
			if amount.is_zero() {
				return Ok(());
			}
			let mut reserves = RewardReserves::<T>::get();
			ensure!(amount <= reserves.marketing, TokenError::FundsUnavailable);
			let minted = T::Currency::mint_into(account_id, amount)?;
			reserves.marketing.saturating_reduce(minted);
			RewardReserves::<T>::put(&reserves);
			Self::deposit_event(Event::<T>::RewardReservesUpdated { reserves });
			Ok(())
		}
	}

	impl<T: Config> OnNewSlot<T::AccountId> for Pallet<T> {
		type Key = BlockSealAuthorityId;

		fn on_frame_start(frame_id: FrameId) {
			Self::run_frame_transition(frame_id);
		}

		fn on_frame_start_weight(frame_id: FrameId) -> Weight {
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

			T::WeightInfo::on_frame_transition(due_releases)
			// MiningSlot already writes and proves FrameStartTicks before this hook; add its read.
			.saturating_add(T::DbWeight::get().reads(due_frames.saturating_add(2)))
			.saturating_add(
				T::DbWeight::get()
					.reads_writes(due_frames.saturating_sub(1), due_frames.saturating_sub(1)),
			)
			.saturating_add(price_weight)
			// Production circulation reads Balances::TotalIssuance, whitelisted by benchmarks;
			// the benchmark price provider supplies circulation without a storage read.
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(<T::PriceProvider as PriceProvider<T::Balance>>::Weights::get_average_cpi_for_ticks())
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
		type MaxBondCollateralLots = T::MaxBondCollateralLots;
		type Weights = ProviderWeightAdapter<T>;
		type Balance = T::Balance;

		fn vault_securitization_changed(
			vault_id: VaultId,
			securitization: Self::Balance,
		) -> DispatchResult {
			BondLotsByVault::<T>::try_mutate_exists(
				vault_id,
				|maybe_vault_bonds| -> DispatchResult {
					if let Some(vault_bonds) = maybe_vault_bonds {
						let previous_flexible = vault_bonds.eligible_flexible_bonds();
						Self::update_vault_displacement(
							vault_id,
							vault_bonds,
							Self::balance_to_bonds(securitization),
							previous_flexible,
						)?;
					}
					Ok(())
				},
			)
		}

		fn has_vault_bond_participation(vault_id: VaultId, account_id: &T::AccountId) -> bool {
			ActiveBondsByAccountAndVault::<T>::get(account_id, vault_id) > 0
		}

		fn active_vault_bond_amount(vault_id: VaultId, account_id: &T::AccountId) -> Self::Balance {
			ActiveBondsByAccountAndVault::<T>::get(account_id, vault_id)
				.saturating_mul(MICROGONS_PER_ARGON)
				.into()
		}

		fn active_account_vault_bond_amount(account_id: &T::AccountId) -> Self::Balance {
			T::PositionProvider::bond_principal(account_id)
		}

		fn encumber_bond_microgons(
			account_id: &T::AccountId,
			microgon_amount: Self::Balance,
			bond_lot_ids: Option<&[BondLotId]>,
		) -> DispatchResult {
			if microgon_amount.is_zero() {
				return Ok(())
			}

			EncumberedBondLotsByAccount::<T>::try_mutate(
				account_id,
				|backing| -> DispatchResult {
					let selected = backing.iter().fold(T::Balance::zero(), |sum, entry| {
						sum.saturating_add(entry.reserved_microgons)
					});
					let unassigned =
						Self::encumbered_bond_microgons(account_id).saturating_sub(selected);
					Self::reserve_bond_collateral(
						account_id,
						unassigned.saturating_add(microgon_amount),
						bond_lot_ids,
						backing,
					)
				},
			)?;
			EncumberedBondMicrogonsByAccount::<T>::mutate(account_id, |encumbered| {
				encumbered.saturating_accrue(microgon_amount);
			});
			Ok(())
		}

		fn release_encumbered_bond_microgons(
			account_id: &T::AccountId,
			microgon_amount: Self::Balance,
		) -> DispatchResult {
			Self::queue_bond_collateral(
				account_id,
				PendingBondCollateral {
					burn_microgons: T::Balance::zero(),
					release_microgons: microgon_amount,
				},
			)
		}

		fn burn_encumbered_bond_microgons(
			account_id: &T::AccountId,
			microgon_amount: Self::Balance,
		) -> DispatchResult {
			Self::queue_bond_collateral(
				account_id,
				PendingBondCollateral {
					burn_microgons: microgon_amount,
					release_microgons: T::Balance::zero(),
				},
			)
		}
	}

	impl<T: Config> Pallet<T> {
		fn queue_bond_collateral(
			account_id: &T::AccountId,
			request: PendingBondCollateral<T::Balance>,
		) -> DispatchResult {
			if request.burn_microgons.is_zero() && request.release_microgons.is_zero() {
				return Ok(());
			}

			let lots = EncumberedBondLotsByAccount::<T>::get(account_id);
			let encumbered = Self::encumbered_bond_microgons(account_id);
			Self::ensure_collateral_selected(&lots, encumbered)?;
			let previous = PendingBondCollateralByAccount::<T>::get(account_id);
			let mut pending = previous.clone().unwrap_or_default();
			let available = encumbered
				.saturating_sub(pending.burn_microgons)
				.saturating_sub(pending.release_microgons);
			ensure!(
				request.burn_microgons <= available,
				Error::<T>::ActiveBondAmountBelowEncumberedBacking,
			);
			ensure!(
				request.release_microgons <= available.saturating_sub(request.burn_microgons),
				Error::<T>::ActiveBondAmountBelowEncumberedBacking,
			);
			pending.burn_microgons.saturating_accrue(request.burn_microgons);
			pending.release_microgons.saturating_accrue(request.release_microgons);

			if previous.is_none() {
				BondCollateralSettlementQueue::<T>::insert(
					AscendingRankKey::new(
						frame_system::Pallet::<T>::block_number()
							.saturated_into::<u64>()
							.saturating_add(1),
						account_id.clone(),
					),
					(),
				);
			}
			PendingBondCollateralByAccount::<T>::insert(account_id, pending);
			Ok(())
		}

		/// Select backing in full before publishing its availability or account record.
		fn reserve_bond_collateral(
			account: &T::AccountId,
			amount: T::Balance,
			bond_lot_ids: Option<&[BondLotId]>,
			backing: &mut BoundedVec<BondCollateralLot<T::Balance>, T::MaxBondCollateralLots>,
		) -> DispatchResult {
			if amount.is_zero() {
				return Ok(())
			}
			let ids = if let Some(ids) = bond_lot_ids {
				ensure!(
					ids.len() <= T::MaxBondCollateralLots::get() as usize,
					Error::<T>::TooManyBondCollateralLots
				);
				ensure!(
					ids.iter().enumerate().all(|(index, id)| !ids[..index].contains(id)),
					Error::<T>::DuplicateBondCollateralLot
				);
				ids.to_vec()
			} else {
				let mut ids = backing
					.iter()
					.filter_map(|entry| entry.bond_lot_id)
					.filter(|id| AvailableBondCollateralLots::<T>::contains_key(account, id))
					.collect::<Vec<_>>();
				let mut remaining_slots =
					(T::MaxBondCollateralLots::get() as usize).saturating_sub(backing.len());
				if remaining_slots > 0 {
					for id in AvailableBondCollateralLots::<T>::iter_key_prefix(account)
						.take(T::MaxBondCollateralLots::get() as usize)
					{
						if remaining_slots == 0 {
							break
						}
						if backing.iter().any(|entry| entry.bond_lot_id == Some(id)) {
							continue
						}
						ids.push(id);
						remaining_slots -= 1;
					}
				}
				ids
			};

			let mut remaining = amount;
			let mut fully_reserved = Vec::new();
			for id in ids {
				if remaining.is_zero() {
					break
				}
				let lot = BondLotById::<T>::get(id).ok_or(Error::<T>::BondLotNotFound)?;
				ensure!(lot.owner == *account, Error::<T>::NotBondLotOwner);
				ensure!(lot.release_reason.is_none(), Error::<T>::BondLotAlreadyReleasing);
				ensure!(
					matches!(lot.program, BondProgram::Vault { .. }),
					Error::<T>::InvalidBondCollateralLot
				);
				let principal = Self::bonds_to_balance(lot.bonds);
				let existing = backing.iter().position(|entry| entry.bond_lot_id == Some(id));
				let reserved =
					existing.map(|index| backing[index].reserved_microgons).unwrap_or_default();
				let added = remaining.min(principal.saturating_sub(reserved));
				if added.is_zero() {
					continue
				}
				let next_reserved = reserved.saturating_add(added);
				if let Some(index) = existing {
					backing[index].reserved_microgons = next_reserved;
				} else {
					backing
						.try_push(BondCollateralLot {
							bond_lot_id: Some(id),
							reserved_microgons: next_reserved,
						})
						.map_err(|_| Error::<T>::TooManyBondCollateralLots)?;
				}
				if next_reserved >= principal {
					fully_reserved.push(id);
				}
				remaining.saturating_reduce(added);
			}
			ensure!(remaining.is_zero(), Error::<T>::ActiveBondAmountBelowEncumberedBacking);

			for id in fully_reserved {
				AvailableBondCollateralLots::<T>::remove(account, id);
			}
			Ok(())
		}

		/// Pre-upgrade backing must have concrete lots selected before principal can leave.
		fn ensure_collateral_selected(
			lots: &[BondCollateralLot<T::Balance>],
			encumbered: T::Balance,
		) -> DispatchResult {
			let selected = lots.iter().fold(T::Balance::zero(), |sum, entry| {
				sum.saturating_add(entry.reserved_microgons)
			});
			ensure!(selected == encumbered, Error::<T>::BondCollateralSelectionRequired);
			Ok(())
		}

		pub(crate) fn settle_bond_collateral_release(
			account: &T::AccountId,
			amount: T::Balance,
			weight: &mut Weight,
		) -> DispatchResult {
			Self::settle_bond_collateral(account, amount, false, weight)
		}

		pub(crate) fn settle_bond_collateral_burn(
			account: &T::AccountId,
			amount: T::Balance,
			weight: &mut Weight,
		) -> DispatchResult {
			Self::settle_bond_collateral(account, amount, true, weight)
		}

		/// Settle the complete request against the account's bounded backing list.
		fn settle_bond_collateral(
			account: &T::AccountId,
			amount: T::Balance,
			burn: bool,
			weight: &mut Weight,
		) -> DispatchResult {
			if amount.is_zero() {
				return Ok(())
			}
			let mut lots = EncumberedBondLotsByAccount::<T>::get(account);

			let visits = lots.len() as u32;
			if burn {
				weight.saturating_accrue(T::WeightInfo::settle_bond_collateral_burn(visits));
			} else {
				weight.saturating_accrue(T::WeightInfo::settle_bond_collateral_release(visits));
			}

			let encumbered = Self::encumbered_bond_microgons(account);
			ensure!(encumbered >= amount, Error::<T>::ActiveBondAmountBelowEncumberedBacking);

			Self::ensure_collateral_selected(&lots, encumbered)?;

			let mut remaining = amount;
			let mut released = T::Balance::zero();
			for entry in lots.iter_mut() {
				let id = entry.bond_lot_id;
				let reserved = entry.reserved_microgons;
				if remaining.is_zero() {
					break
				}
				let settled = remaining.min(reserved);
				let next_reserved = reserved.saturating_sub(settled);
				let lot = id.and_then(BondLotById::<T>::get);
				let principal =
					lot.as_ref().map(|lot| Self::bonds_to_balance(lot.bonds)).unwrap_or_default();
				let mut next_principal = principal;
				let held = principal.max(reserved);
				let next_held = if burn { held.saturating_sub(settled) } else { held };
				if burn {
					T::Currency::burn_held(
						&HoldReason::ContributedToTreasury.into(),
						account,
						settled,
						Precision::Exact,
						Fortitude::Force,
					)?;
					if T::Currency::balance_on_hold(
						&HoldReason::ContributedToTreasury.into(),
						account,
					)
					.is_zero()
					{
						let _ = frame_system::Pallet::<T>::dec_providers(account);
					}
					if let (Some(id), Some(lot)) = (id, lot) {
						let remaining_bonds = Self::balance_to_bonds(next_held).min(lot.bonds);
						next_principal = Self::bonds_to_balance(remaining_bonds);
						Self::trim_collateral_lot(id, &lot, remaining_bonds)?;
					}
				}

				let slack = next_held.saturating_sub(next_principal.max(next_reserved));
				if !slack.is_zero() {
					Self::release_hold::<T::Currency>(account, slack)?;
					released.saturating_accrue(slack);
				}
				entry.reserved_microgons = next_reserved;
				if let Some(id) = id {
					if next_principal > next_reserved {
						AvailableBondCollateralLots::<T>::insert(account, id, ());
					} else {
						AvailableBondCollateralLots::<T>::remove(account, id);
					}
				}
				remaining.saturating_reduce(settled);
			}
			ensure!(remaining.is_zero(), Error::<T>::InternalError);
			lots.retain(|entry| !entry.reserved_microgons.is_zero());
			if lots.is_empty() {
				EncumberedBondLotsByAccount::<T>::remove(account);
			} else {
				EncumberedBondLotsByAccount::<T>::insert(account, lots);
			}
			EncumberedBondMicrogonsByAccount::<T>::mutate(account, |total| {
				total.saturating_reduce(amount)
			});
			if burn {
				Self::update_account_vault_bond_total(account);
				Self::deposit_event(Event::EncumberedBondMicrogonsBurned {
					account_id: account.clone(),
					burned_amount: amount,
					released_amount: released,
				});
			}
			Ok(())
		}

		fn trim_collateral_lot(
			id: BondLotId,
			lot: &BondLot<T>,
			remaining_bonds: Bonds,
		) -> DispatchResult {
			let removed_bonds = lot.bonds.saturating_sub(remaining_bonds);
			if removed_bonds == 0 {
				return Ok(())
			}
			let BondProgram::Vault { vault_id, .. } = lot.program else {
				return Err(Error::<T>::InternalError.into())
			};
			Self::update_bond_principal(
				&lot.owner,
				vault_id,
				Self::bonds_to_balance(lot.bonds),
				Self::bonds_to_balance(remaining_bonds),
			)?;
			if remaining_bonds == 0 {
				BondLotById::<T>::remove(id);
				BondLotIdsByAccount::<T>::remove(&lot.owner, id);
				BondLotIdsByVault::<T>::remove(vault_id, id);
				TotalArgonBondLots::<T>::mutate(|count| count.saturating_reduce(1));
				Self::remove_bond_lot_from_vault(vault_id, lot)?;
			} else {
				BondLotById::<T>::mutate(id, |record| {
					if let Some(lot) = record {
						lot.bonds = remaining_bonds;
					}
				});
				BondLotsByVault::<T>::try_mutate(vault_id, |vault| -> DispatchResult {
					let previous_flexible = vault.eligible_flexible_bonds();
					if lot.is_flexible {
						vault.flexible_bonds.saturating_reduce(removed_bonds);
					} else {
						vault.regular_bonds.saturating_reduce(removed_bonds);
						T::PositionProvider::account_quantity_updated(
							&lot.owner,
							PositionQuantity::Bonds,
							removed_bonds,
							0,
						)?;
					}
					Self::update_vault_displacement(
						vault_id,
						vault,
						Self::balance_to_bonds(Self::get_vault_bond_capacity(vault_id)),
						previous_flexible,
					)
				})?;
			}
			Ok(())
		}
	}

	/// Owner-scoped ARGON amounts awaiting actual burn or encumbrance release, in microgons.
	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Default,
		PartialEq,
		Eq,
		Debug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct PendingBondCollateral<Balance> {
		/// Held ARGON to burn and reconcile against the owner's bond lots at settlement.
		pub burn_microgons: Balance,
		/// Encumbrance to release after settling any pending burn.
		pub release_microgons: Balance,
	}

	type RewardFunding<T> = Credit<<T as frame_system::Config>::AccountId, <T as Config>::Currency>;

	/// Funding and completed payments for one reward pool. Unspent minted funds return to reserves.
	#[derive(Default)]
	struct PoolAllocation<Balance> {
		from_pool: Balance,
		from_reserve: Balance,
		withheld: Balance,
	}

	/// Daily yield target and share of excess rewards paid by one reward program.
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
	pub struct TaperingProgram {
		/// Daily return at which tapering starts, with one frame per financial day.
		/// Derived from the desired APY as (1 + APY)^(1/365) - 1.
		pub target_daily_return: FixedU128,
		/// Share of rewards paid above the daily APY target; the rest funds reserves.
		pub excess_reward_payout_percent: Percent,
		/// Optional lower daily target, reached linearly as ARGON falls below its price target.
		pub target_daily_return_with_price_below_target: Option<BelowTargetPriceYield>,
	}

	/// Lower daily return reached as ARGON falls below its price target.
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
	pub struct BelowTargetPriceYield {
		/// Daily reinvestment rate at or beyond the configured price drop.
		pub target_daily_return: FixedU128,
		/// Percent below the ARGON price target where the lower daily return applies.
		pub argon_price_drop_for_target: Percent,
	}

	/// Independent reward settings for each program; changes apply to future frames.
	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
	)]
	pub struct TaperingPolicy {
		/// Daily yield and excess payout settings for Bitcoin liquidity earnings.
		pub bitcoin_liquids: TaperingProgram,
		/// Daily yield and excess payout settings for ARGON bonds.
		pub argon_bonds: TaperingProgram,
		/// Daily yield and excess payout settings for ARGONOT stakes.
		pub argonot_stakes: TaperingProgram,
		/// Daily yield and excess payout settings applied independently to each vault.
		pub vault_revenue: TaperingProgram,
		/// Daily yield and excess payout settings for future stabilizer pool rewards.
		pub stabilizer_pools: TaperingProgram,
	}

	/// Reward settings captured before a frame earns revenue, unaffected by later config changes.
	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
	)]
	pub struct FrameRewardTaperingPolicy {
		/// Mining frame whose rewards use this policy; one frame is one financial day.
		pub frame_id: FrameId,
		/// Settings captured at frame start, unaffected by later configuration changes.
		pub policy: TaperingPolicy,
	}

	/// Terms for one earning frame, from opening circulation through reward-rate finalization.
	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
	)]
	pub enum StabilizerPoolRewardFrame<Balance> {
		/// The frame is still earning.
		Earning {
			/// Desired eligible pool principal, valued in microgons, from the configured share
			/// of circulating ARGON at frame opening.
			target_pool_value_microgons: Balance,
		},
		/// Funding is fixed and awaits the source's finalized eligible value.
		Allocated {
			/// Desired eligible pool principal captured at frame opening, in microgons.
			target_pool_value_microgons: Balance,
			/// Newly funded rewards after applying the frame's taper, in microgons.
			budget: Balance,
		},
		/// Claims can now settle against the frozen rate.
		Finalized {
			/// Fixed ARGON reward per microgon of eligible value, before asset weighting.
			reward_rate: FixedU128,
		},
	}

	/// Reward budgets excluded from issuance until paid. These are amounts, not funded accounts.
	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		Debug,
		Default,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct RewardReserveBalances<Balance> {
		/// Budget available for marketing reward claims.
		pub marketing: Balance,
		/// Budget for future Bitcoin Liquids rewards.
		pub bitcoin_liquids: Balance,
		/// Withheld bond rewards available to support bonds when ARGON is below target.
		pub argon_bonds: Balance,
		/// Withheld stake rewards available to support stakes when ARGON is below target.
		pub argonot_stakes: Balance,
		/// Budget for future stabilizer pool rewards and support below ARGON's price target.
		pub stabilizer_pools: Balance,
		/// Vault taper excess and other unallocated rewards available for future liquidity buys.
		pub liquidity_buys: Balance,
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
		pub fn eligible_flexible_bonds(&self) -> Bonds {
			self.flexible_bonds.saturating_sub(self.displaced_flexible_bonds)
		}

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
		pub total_active_bonds: BondTotal,
		/// Frame-effective earning quantities, independent of later source mutations.
		pub quantities: PositionQuantities<T::Balance>,
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

	/// Companion to the unchanged vault capital format, so a frame frozen before an upgrade
	/// can still settle with its original payout formula.
	#[derive(Encode, Decode, PartialEqNoBound, DebugNoBound, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct FrameVaultBitcoin<T: Config> {
		#[codec(compact)]
		pub frame_id: FrameId,
		/// Covered satoshis across every vault, including those outside the payout list.
		#[codec(compact)]
		pub total_network_securitized_satoshis: u128,
		/// Network securitization target converted at the frozen market price.
		pub target_securitized_satoshis: Option<u128>,
		pub vault_securitized_satoshis: BoundedBTreeMap<VaultId, Satoshis, T::MaxVaultsPerPool>,
	}

	/// The Argonot participation weight used for commitments and payout of one frame.
	#[derive(Encode, Decode, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub struct FrameArgonotParticipation {
		/// The frame whose commitments and payout use this participation factor.
		#[codec(compact)]
		pub frame_id: FrameId,
		/// Share of variable vault earnings requiring full Argonot backing.
		pub factor: Percent,
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
		/// Upstream participation frozen with the capital used for this frame's vault earnings.
		pub upstream_participation: FixedU128,
	}
}
