use crate::prelude::*;
use ismp::host::StateMachine;
use pallet_transaction_payment::Multiplier;
use smallvec::smallvec;
use sp_runtime::traits::One;

pub type AccountData = pallet_balances::AccountData<Balance>;

/// TODO: adjust this to match measured
/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
/// used to limit the maximal weight of a single extrinsic.
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

/// TODO: measure this
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
/// `Operational` extrinsics.
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(10), u64::MAX);

/// The existential deposit.
pub const EXISTENTIAL_DEPOSIT: Balance = 100_000;

pub const ARGON: Balance = 1_000_000;
pub const CENTS: Balance = ARGON / 100_000;
pub const MILLIGONS: Balance = 1_000;
pub const MICROGONS: Balance = 1;

const FINAL_ARGONS_PER_BLOCK: Balance = 5_000_000;
const INCREMENTAL_REWARD_AMOUNT: Balance = 1_000;
const INCREMENT_TICKS: Tick = 118;

pub type NotaryRecordT =
	NotaryRecordWithState<AccountId, BlockNumber, MaxNotaryHosts, NotebookVerifyError>;

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 100 * CENTS + (bytes as Balance) * 5 * MICROGONS
}
parameter_types! {
	// ### block weights

	pub const BlockHashCount: BlockNumber = 4096;
	/// We allow for 60 seconds of compute with a 10 second average block time.
	pub BlockWeights:  frame_system::limits::BlockWeights =  frame_system::limits::BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);

	// ### pallet_block_seal
	pub const TargetComputeBlockPercent: FixedU128 = FixedU128::from_rational(49, 100); // aim for less than full compute time so it can wait for notebooks
	pub const TargetBlockVotes: u32 = 50_000;
	pub const SealSpecVoteHistoryForAverage: u32 = 24 * 60; // 24 hours of history
	pub const SealSpecComputeHistoryToTrack: u32 = 6 * 60; // 6 hours of history
	pub const SealSpecComputeDifficultyChangePeriod: u32 = 60; // change difficulty every hour

	pub const DefaultChannelHoldDuration: Tick = 60;
	pub const HistoricalPaymentAddressTicksToKeep: Tick = DefaultChannelHoldDuration::get() + CHANNEL_HOLD_CLAWBACK_TICKS + 10;

	// ### pallet_rewards
	pub const StartingArgonsPerBlock: Balance = 500_000;
	pub const StartingOwnershipTokensPerBlock: Balance = 500_000;
	pub const IncrementalGrowth: GrowthPath<Balance> = (INCREMENTAL_REWARD_AMOUNT, INCREMENT_TICKS, FINAL_ARGONS_PER_BLOCK); // we add 1 milligon every 118 blocks until we reach 5 argons/ownership tokens
	pub const HalvingBeginTick: Tick = INCREMENT_TICKS  * (FINAL_ARGONS_PER_BLOCK as Tick - StartingArgonsPerBlock::get() as Tick) / INCREMENTAL_REWARD_AMOUNT as Tick; // starts after ~ one year of increments
	pub const HalvingTicks: Tick = 2_100_000; // based on bitcoin, but 10x since we're 1 block per minute
	pub const MaturationBlocks: u32 = 5;
	pub const MinerPayoutPercent: FixedU128 = FixedU128::from_rational(75, 100);
	pub const DomainExpirationTicks: Tick = 60 * 24 * 365; // 1 year

	// ### pallet_mining_slot
	pub const MaxMiners: u32 = 100; // must multiply cleanly by MaxCohortSize
	pub const MaxCohortSize: u32 = MaxMiners::get() / 10; // this means mining_slots last 10 days
	pub const ArgonotsPercentAdjustmentDamper: FixedU128 = FixedU128::from_rational(20, 100);
	pub const MaximumArgonotProrataPercent: Percent = Percent::from_percent(80);
	pub const TargetBidsPerSlot: u32 = 12; // Ideally we want 12 bids per slot
	pub const GrandpaRotationBlocks: BlockNumber = 260;

	// ### pallet_vaults
	pub const MaxConcurrentlyExpiringObligations: u32 = 1_000;
	pub const MinimumObligationAmount: u128 = 100_000;
	pub const TicksPerDay: Tick = 1440;
	pub const TicksPerYear: Tick = 1440 * 365;

	const BitcoinBlocksPerDay: BitcoinHeight = 6 * 24;
	pub const BitcoinLockDurationBlocks: BitcoinHeight = BitcoinBlocksPerDay::get() * 365; // 1 year
	pub const BitcoinLockReclamationBlocks: BitcoinHeight = BitcoinBlocksPerDay::get() * 30; // 30 days
	pub const LockReleaseCosignDeadlineBlocks: BitcoinHeight = BitcoinBlocksPerDay::get() * 5; // 5 days

	pub const MaxSetIdSessionEntries: u32 = 2u32;

	pub const MaxConcurrentlyReleasingLocks: u32 = 1000;
	pub const MaxPendingTermModificationsPerTick: u32 = 100;
	pub const MinTermsModificationTickDelay: Tick = TicksPerDay::get() - 1; // must be at least one slot (day)
	pub const VaultFundingModificationDelay: Tick = 60; // 1 hour

	pub const EnableRewardSharing: bool = false;

	// ### pallet chain transfer
	pub const ChainTransferPalletId: PalletId = PalletId(*b"transfer");

	/// How long a transfer should remain in storage before returning.
	pub const TransferExpirationTicks: u32 = 1400 * 10;

	/// How many transfers out can be queued per block
	pub const MaxPendingTransfersOutPerBlock: u32 = 1000;

	// ### pallet_notary
	pub const MaxActiveNotaries: u32 = 25; // arbitrarily set
	pub const MaxProposalHoldBlocks: u32 = 1440 * 14; // 2 weeks to approve
	pub const MaxProposalsPerBlock: u32 = 10;
	pub const MetaChangesTickDelay: u64 = 6; // delay pubkey changes for minimum of an hour
	pub const MaxTicksForKeyHistory: u32 = 1440 * 2; // keep for 2 days.. only used for notebook submission
	/// Max host ips a notary can provide
	pub const MaxNotaryHosts: u32 = 4;

	// ### pallet_proxy

	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 40);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	// One storage item; key size 32, value size 16
	pub const AnnouncementDepositBase: Balance = deposit(1, 48);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
	pub const MaxProxies: u16 = 32;
	pub const MaxPending: u16 = 32;

	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u32 = 100;

	// ### pallet_bitcoin_locks
	pub const BitcoinLockDuration: u32 = 60 * 24 * 365; // 1 year
	pub const MaxPendingMintUtxos: u32 = 10_000;
	pub const MaxTrackedUtxos: u32 = 1_000_000_000;

	pub const MaxDowntimeTicksBeforeReset: Tick = 60; // 1 hour
	pub const MaxHistoryToKeep: u32 = 24 * 60; // 1 day worth of prices
	pub const MaxPriceAgeInTicks: Tick = 24 * 60; // 1 day
	pub const MaxArgonChangePerTickAwayFromTarget: FixedU128 = FixedU128::from_rational(1, 100); // 1 centagon
	pub const MaxArgonTargetChangePerTick: FixedU128 = FixedU128::from_rational(1, 100); // 1 centagon

	pub const MaxPendingConfirmationBlocks: BitcoinHeight = 10 * (6 * 24); // 10 days of bitcoin blocks

	pub const MaxPendingConfirmationUtxos: u32 = 10_000;
	pub const MaxBitcoinBirthBlocksOld: BitcoinHeight = 10 * (6 * 24); // 10 days of bitcoin blocks

	// Fees
	pub FeeMultiplier: Multiplier = Multiplier::one();
	pub const TransactionByteFee: Balance = 10;


	// ## pallet_hyperbridge
	// The host state machine of this pallet
	pub const HostStateMachine: StateMachine = StateMachine::Substrate(*b"argn");
	// A constant that should represent the native asset id, this id must be unique to the native currency
	pub const NativeAssetId: u32 = 0;

	// The ownership token Asset Id
	pub const OwnershipTokenAssetId: u32 = 1;
	// Set the correct decimals for the native currency
	pub const Decimals: u8 = 6;
}

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - `[0, MAXIMUM_BLOCK_WEIGHT]`
///   - `[Balance::min, Balance::max]`
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		let p = ARGON; // microgons
		let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
		// BAB - disabling wage protector for fees. Makes it hard to keep system stable
		// let cpi = PriceIndex::get_argon_cpi().unwrap_or(ArgonCPI::zero());
		// if cpi.is_positive() {
		// 	let cpi = cpi.into_inner() / ArgonCPI::accuracy();
		// 	let adjustment = (p * (cpi as u128) * 1_000).checked_div(1_000).unwrap_or_default();
		// 	p += adjustment;
		// }
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}
