mod fees;

use super::{
	currency::*, Balances, BitcoinUtxos, BlockSeal, BlockSealSpec, Bonds, ChainTransfer, Digests,
	Domains, Grandpa, MiningSlot, Mint, Notaries, Notebook, NotebookVerifyError, OriginCaller,
	Ownership, PriceIndex, Runtime, RuntimeCall, RuntimeEvent, RuntimeFreezeReason,
	RuntimeHoldReason, System, Ticks, Vaults, VERSION,
};
use crate::SessionKeys;
use alloc::vec::Vec;
use argon_primitives::{
	bitcoin::BitcoinHeight, notary::NotaryRecordWithState, tick::Tick, AccountId, Balance,
	BlockNumber, BlockSealAuthorityId, Moment, TickProvider, CHANNEL_HOLD_CLAWBACK_TICKS,
};
pub use frame_support::{
	construct_runtime, derive_impl,
	pallet_prelude::*,
	parameter_types,
	traits::{
		fungible, fungible::Balanced, ConstBool, ConstU128, ConstU16, ConstU32, ConstU64, ConstU8,
		Contains, Currency, Everything, Imbalance, InsideBoth, InstanceFilter, KeyOwnerProofSystem,
		OnUnbalanced, PalletInfo, Randomness, StorageInfo, StorageMapShim, TransformOrigin,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		IdentityFee, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
		WeightToFeePolynomial,
	},
	PalletId, StorageValue,
};
use sp_consensus_grandpa::{AuthorityId as GrandpaId, AuthorityList};

use frame_system::EnsureRoot;
use pallet_bond::BitcoinVerifier;
use pallet_mining_slot::OnNewSlot;
use pallet_tx_pause::RuntimeCallNameOf;
use sp_arithmetic::{FixedU128, Perbill};
use sp_runtime::traits::{BlakeTwo256, Zero};
use sp_version::RuntimeVersion;

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

parameter_types! {
	pub const BlockHashCount: BlockNumber = 4096;
	pub const Version: RuntimeVersion = VERSION;
	/// We allow for 60 seconds of compute with a 6 second average block time.
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
}

/// Calls that cannot be paused by the tx-pause pallet.
pub struct TxPauseWhitelistedCalls;

impl Contains<RuntimeCallNameOf<Runtime>> for TxPauseWhitelistedCalls {
	fn contains(full_name: &RuntimeCallNameOf<Runtime>) -> bool {
		#[allow(clippy::match_like_matches_macro)]
		match (full_name.0.as_slice(), full_name.1.as_slice()) {
			(b"System", _) => true,
			(b"ParachainSystem", _) => true,
			(b"Xcm", b"force_suspension") => true,
			_ => false,
		}
	}
}

/// This pallet is intended to be used as a shortterm security measure.
impl pallet_tx_pause::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PauseOrigin = EnsureRoot<AccountId>;
	type UnpauseOrigin = EnsureRoot<AccountId>;
	type WhitelistedCalls = TxPauseWhitelistedCalls;
	type MaxNameLen = ConstU32<256>;
	type WeightInfo = pallet_tx_pause::weights::SubstrateWeight<Runtime>;
}

pub struct BaseCallFilter;
impl Contains<RuntimeCall> for BaseCallFilter {
	fn contains(call: &RuntimeCall) -> bool {
		// placeholder for filter
		match call {
			RuntimeCall::System(..) => true,
			_ => true,
		}
	}
}

parameter_types! {
	pub const TargetComputeBlockPercent: FixedU128 = FixedU128::from_rational(125, 100); // aim for compute to take a bit longer than vote
	pub const TargetBlockVotes: u32 = 50_000;
	pub const SealSpecMinimumsChangePeriod: u32 = 60 * 24; // change block_seal_spec once a day

	pub const DefaultChannelHoldDuration: Tick = 60;
	pub const HistoricalPaymentAddressTicksToKeep: Tick = DefaultChannelHoldDuration::get() + CHANNEL_HOLD_CLAWBACK_TICKS + 10;

	pub const ArgonsPerBlock: u32 = 5_000;
	pub const StartingOwnershipTokensPerBlock: u32 = 5_000;
	pub const HalvingBlocks: u32 = 2_100_000; // based on bitcoin, but 10x since we're 1 block per minute
	pub const MaturationBlocks: u32 = 5;
	pub const MinerPayoutPercent: FixedU128 = FixedU128::from_rational(75, 100);
	pub const DomainExpirationTicks: Tick = 60 * 24 * 365; // 1 year
}

impl pallet_block_seal_spec::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type TargetComputeBlockPercent = TargetComputeBlockPercent;
	type AuthorityProvider = MiningSlot;
	type NotebookProvider = Notebook;
	type WeightInfo = pallet_block_seal_spec::weights::SubstrateWeight<Runtime>;
	type TargetBlockVotes = TargetBlockVotes;
	type ChangePeriod = SealSpecMinimumsChangePeriod;
	type SealInherent = BlockSeal;
	type TickProvider = Ticks;
	type MaxActiveNotaries = MaxActiveNotaries;
}

pub struct NotebookTickProvider;
impl Get<Tick> for NotebookTickProvider {
	fn get() -> Tick {
		let schedule = Ticks::voting_schedule();
		schedule.notebook_tick()
	}
}
impl pallet_block_rewards::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_block_rewards::weights::SubstrateWeight<Runtime>;
	type ArgonCurrency = Balances;
	type OwnershipCurrency = Ownership;
	type Balance = Balance;
	type BlockSealerProvider = BlockSeal;
	type NotaryProvider = Notaries;
	type NotebookProvider = Notebook;
	type NotebookTick = NotebookTickProvider;
	type ArgonsPerBlock = ArgonsPerBlock;
	type StartingOwnershipTokensPerBlock = StartingOwnershipTokensPerBlock;
	type HalvingBlocks = HalvingBlocks;
	type MinerPayoutPercent = MinerPayoutPercent;
	type MaturationBlocks = MaturationBlocks;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type EventHandler = Mint;
	type BlockRewardAccountsProvider = MiningSlot;
}

impl pallet_domains::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_domains::weights::SubstrateWeight<Runtime>;
	type NotebookTick = NotebookTickProvider;
	type DomainExpirationTicks = DomainExpirationTicks;
	type HistoricalPaymentAddressTicksToKeep = HistoricalPaymentAddressTicksToKeep;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = Digests;
	type EventHandler = ();
}

impl pallet_digests::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_digests::weights::SubstrateWeight<Runtime>;
	type NotebookVerifyError = NotebookVerifyError;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = (BlockSealSpec, Ticks);
	type MinimumPeriod = ConstU64<1000>;
	type WeightInfo = ();
}

impl pallet_ticks::Config for Runtime {
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxCohortSize: u32 = 1_000; // this means mining_slots last 10 days
	pub const MaxMiners: u32 = 10_000; // must multiply cleanly by MaxCohortSize
	pub const OwnershipPercentAdjustmentDamper: FixedU128 = FixedU128::from_rational(20, 100);
	pub const TargetBidsPerSlot: u32 = 1_200; // 20% extra bids

	pub const MaxConcurrentlyExpiringBonds: u32 = 1000;
	pub const MinimumBondAmount: u128 = 1_000;
	pub const BlocksPerDay: u32 = 1440;
	pub const BlocksPerYear: u32 = 1440 * 365;

	const BitcoinBlocksPerDay: BitcoinHeight = 6 * 24;
	pub const BitcoinBondDurationBlocks: BitcoinHeight = BitcoinBlocksPerDay::get() * 365; // 1 year
	pub const BitcoinBondReclamationBlocks: BitcoinHeight = BitcoinBlocksPerDay::get() * 30; // 30 days
	pub const UtxoUnlockCosignDeadlineBlocks: BitcoinHeight = BitcoinBlocksPerDay::get() * 5; // 5 days

	pub const MaxSetIdSessionEntries: u32 = 2u32;

	pub const MaxUnlockingUtxos: u32 = 1000;
	pub const MaxPendingTermModificationsPerBlock: u32 = 100;
	pub const MinTermsModificationBlockDelay: u32 = 1439; // must be at least one slot (day)
}

impl pallet_vaults::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_vaults::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type Balance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MinimumBondAmount = MinimumBondAmount;
	type BlocksPerDay = BlocksPerDay;
	type MaxPendingTermModificationsPerBlock = MaxPendingTermModificationsPerBlock;
	type MiningSlotProvider = MiningSlot;
	type MinTermsModificationBlockDelay = MinTermsModificationBlockDelay;
	type GetBitcoinNetwork = BitcoinUtxos;
}

pub struct BitcoinSignatureVerifier;
impl BitcoinVerifier<Runtime> for BitcoinSignatureVerifier {}
impl pallet_bond::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_bond::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MinimumBondAmount = MinimumBondAmount;
	type MaxConcurrentlyExpiringBonds = MaxConcurrentlyExpiringBonds;
	type Balance = Balance;
	type VaultProvider = Vaults;
	type PriceProvider = PriceIndex;
	type BitcoinBlockHeight = BitcoinUtxos;
	type GetBitcoinNetwork = BitcoinUtxos;
	type BitcoinBondDurationBlocks = BitcoinBondDurationBlocks;
	type BitcoinBondReclamationBlocks = BitcoinBondReclamationBlocks;
	type BitcoinUtxoTracker = BitcoinUtxos;
	type MaxUnlockingUtxos = MaxUnlockingUtxos;
	type BondEvents = Mint;
	type ArgonBlocksPerDay = BlocksPerDay;
	type UtxoUnlockCosignDeadlineBlocks = UtxoUnlockCosignDeadlineBlocks;
	type BitcoinSignatureVerifier = BitcoinSignatureVerifier;
}

pub struct GrandpaSlotRotation;

impl OnNewSlot<AccountId> for GrandpaSlotRotation {
	type Key = GrandpaId;
	fn on_new_slot(
		removed_authorities: Vec<(&AccountId, Self::Key)>,
		added_authorities: Vec<(&AccountId, Self::Key)>,
	) {
		if removed_authorities.is_empty() && added_authorities.is_empty() {
			return;
		}
		let mut next_authorities: AuthorityList = Grandpa::grandpa_authorities();
		for (_, authority_id) in removed_authorities {
			if let Some(index) = next_authorities.iter().position(|x| x.0 == authority_id) {
				next_authorities.remove(index);
			}
		}
		for (_, authority_id) in added_authorities {
			next_authorities.push((authority_id, 1));
		}

		if let Err(err) = Grandpa::schedule_change(next_authorities, Zero::zero(), None) {
			log::error!("Failed to schedule grandpa change: {:?}", err);
		}
	}
}

impl pallet_mining_slot::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_mining_slot::weights::SubstrateWeight<Runtime>;
	type MaxMiners = MaxMiners;
	type OwnershipCurrency = Ownership;
	type OwnershipPercentAdjustmentDamper = OwnershipPercentAdjustmentDamper;
	type TargetBidsPerSlot = TargetBidsPerSlot;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxCohortSize = MaxCohortSize;
	type Balance = Balance;
	type BondProvider = Bonds;
	type SlotEvents = (GrandpaSlotRotation,);
	type Keys = SessionKeys;
	type MiningAuthorityId = BlockSealAuthorityId;
}

impl pallet_block_seal::Config for Runtime {
	type AuthorityId = BlockSealAuthorityId;
	type WeightInfo = pallet_block_seal::weights::SubstrateWeight<Runtime>;
	type AuthorityProvider = MiningSlot;
	type NotebookProvider = Notebook;
	type BlockSealSpecProvider = BlockSealSpec;
	type TickProvider = Ticks;
	type EventHandler = MiningSlot;
	type Digests = Digests;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxAuthorities = MaxMiners;
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
}

parameter_types! {
	pub const ChainTransferPalletId: PalletId = PalletId(*b"transfer");

	/// How long a transfer should remain in storage before returning.
	pub const TransferExpirationTicks: u32 = 1400 * 10;

	/// How many transfers out can be queued per block
	pub const MaxPendingTransfersOutPerBlock: u32 = 1000;
}

impl pallet_chain_transfer::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_chain_transfer::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type Balance = Balance;
	type PalletId = ChainTransferPalletId;
	type TransferExpirationTicks = TransferExpirationTicks;
	type MaxPendingTransfersOutPerBlock = MaxPendingTransfersOutPerBlock;
	type NotebookProvider = Notebook;
	type EventHandler = Mint;
	type NotebookTick = NotebookTickProvider;
}

impl pallet_notebook::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_notebook::weights::SubstrateWeight<Runtime>;
	type EventHandler = (ChainTransfer, BlockSealSpec, Domains);
	type NotaryProvider = Notaries;
	type ChainTransferLookup = ChainTransfer;
	type BlockSealSpecProvider = BlockSealSpec;
	type TickProvider = Ticks;
	type Digests = Digests;
}

parameter_types! {
	pub const MaxActiveNotaries: u32 = 25; // arbitrarily set
	pub const MaxProposalHoldBlocks: u32 = 1440 * 14; // 2 weeks to approve
	pub const MaxProposalsPerBlock: u32 = 10;
	pub const MetaChangesTickDelay: u32 = 6; // delay pubkey changes for minimum of an hour
	pub const MaxTicksForKeyHistory: u32 = 1440 * 2; // keep for 2 days.. only used for notebook submission
	/// Max host ips a notary can provide
	pub const MaxNotaryHosts: u32 = 4;
}

pub type NotaryRecordT =
	NotaryRecordWithState<AccountId, BlockNumber, MaxNotaryHosts, NotebookVerifyError>;

impl pallet_notaries::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_notaries::weights::SubstrateWeight<Runtime>;
	type MaxActiveNotaries = MaxActiveNotaries;
	type MaxProposalHoldBlocks = MaxProposalHoldBlocks;
	type MaxProposalsPerBlock = MaxProposalsPerBlock;
	type MetaChangesTickDelay = MetaChangesTickDelay;
	type MaxTicksForKeyHistory = MaxTicksForKeyHistory;
	type MaxNotaryHosts = MaxNotaryHosts;
	type TickProvider = Ticks;
}
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type ArgonToken = pallet_balances::Instance1;
impl pallet_balances::Config<ArgonToken> for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<2>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 40);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	// One storage item; key size 32, value size 16
	pub const AnnouncementDepositBase: Balance = deposit(1, 48);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
	pub const MaxProxies: u16 = 32;
	pub const MaxPending: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	TypeInfo,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
)]
pub enum ProxyType {
	Any,
	NonTransfer,
	PriceIndex,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer =>
				!matches!(c, RuntimeCall::Balances(..) | RuntimeCall::Ownership(..)),
			ProxyType::PriceIndex => matches!(c, RuntimeCall::PriceIndex(..)),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u32 = 100;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}
impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
	pub const BitcoinBondDuration: u32 = 60 * 24 * 365; // 1 year
	pub const MaxPendingMintUtxos: u32 = 10_000;
	pub const MaxTrackedUtxos: u32 = 18_000_000;

	pub const MaxDowntimeTicksBeforeReset: Tick = 60; // 1 hour
	pub const MaxHistoryToKeep: u32 = 24 * 60; // 1 day worth of prices
	pub const MaxPriceAgeInTicks: Tick = 24 * 60; // 1 day
	pub const MaxArgonChangePerTickAwayFromTarget: FixedU128 = FixedU128::from_rational(1, 100); // 1 centagon
	pub const MaxArgonTargetChangePerTick: FixedU128 = FixedU128::from_rational(1, 100); // 1 centagon

	pub const MaxPendingConfirmationBlocks: BitcoinHeight = 10 * (6 * 24); // 10 days of bitcoin blocks

	pub const MaxPendingConfirmationUtxos: u32 = 10_000;
	pub const MaxBitcoinBirthBlocksOld: BitcoinHeight = 10 * (6 * 24); // 10 days of bitcoin blocks
}

impl pallet_price_index::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type CurrentTick = Ticks;
	type WeightInfo = pallet_price_index::weights::SubstrateWeight<Runtime>;
	type Balance = Balance;
	type MaxDowntimeTicksBeforeReset = MaxDowntimeTicksBeforeReset;
	type MaxPriceAgeInTicks = MaxPriceAgeInTicks;
	type MaxArgonChangePerTickAwayFromTarget = MaxArgonChangePerTickAwayFromTarget;
	type MaxArgonTargetChangePerTick = MaxArgonTargetChangePerTick;
}

impl pallet_bitcoin_utxos::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_bitcoin_utxos::weights::SubstrateWeight<Runtime>;
	type EventHandler = Bonds;
	type MaxPendingConfirmationBlocks = MaxPendingConfirmationBlocks;
	type MaxPendingConfirmationUtxos = MaxPendingConfirmationUtxos;
}

impl pallet_mint::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_mint::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type Balance = Balance;
	type PriceProvider = PriceIndex;
	type MaxPendingMintUtxos = MaxPendingMintUtxos;
	type BlockRewardAccountsProvider = MiningSlot;
}

type OwnershipToken = pallet_balances::Instance2;
impl pallet_balances::Config<OwnershipToken> for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Runtime, OwnershipToken>,
		AccountId,
		pallet_balances::AccountData<Balance>,
	>;

	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<2>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
	type PalletsOrigin = OriginCaller;
}
