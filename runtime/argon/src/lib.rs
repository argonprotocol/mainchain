#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
mod benchmark;

mod migrations;
pub mod weights;

use frame_support::weights::ConstantMultiplier;
pub use pallet_notebook::NotebookVerifyError;
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter};
use polkadot_sdk::*;

#[cfg(feature = "runtime-benchmarks")]
use argon_runtime_common::benchmarking;
pub use argon_runtime_common::config::NotaryRecordT;
use argon_runtime_common::{prelude::*, use_unless_benchmark};

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use pallet_bitcoin_locks::MinimumSatoshis;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{impl_opaque_keys, Weight};
#[cfg(feature = "std")]
use sp_version::NativeVersion;

impl_opaque_keys! {
	pub struct SessionKeys {
	pub grandpa: Grandpa,
	pub block_seal_authority: MiningSlot,
	// NOTE: if you add a key, you must also update the `max_encoded_len` method
	}
}
impl MaxEncodedLen for SessionKeys {
	fn max_encoded_len() -> usize {
		GrandpaId::max_encoded_len() + BlockSealAuthorityId::max_encoded_len()
	}
}

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;
	#[runtime::pallet_index(1)]
	pub type Digests = pallet_digests;
	#[runtime::pallet_index(2)]
	pub type Timestamp = pallet_timestamp;
	#[runtime::pallet_index(3)]
	pub type Multisig = pallet_multisig;
	#[runtime::pallet_index(4)]
	pub type Proxy = pallet_proxy;
	#[runtime::pallet_index(5)]
	pub type Ticks = pallet_ticks;
	// NOTE: `MiningSlot::is_new_frame_started()` is set during MiningSlot on_initialize.
	// Pallets with a lower index cannot rely on it in their on_initialize hooks.
	#[runtime::pallet_index(6)]
	pub type MiningSlot = pallet_mining_slot;
	#[runtime::pallet_index(7)]
	pub type BitcoinUtxos = pallet_bitcoin_utxos;
	#[runtime::pallet_index(8)]
	pub type Vaults = pallet_vaults;
	#[runtime::pallet_index(9)]
	pub type BitcoinLocks = pallet_bitcoin_locks;
	#[runtime::pallet_index(10)]
	pub type Notaries = pallet_notaries;
	#[runtime::pallet_index(11)]
	pub type Notebook = pallet_notebook;
	#[runtime::pallet_index(12)]
	pub type LocalchainTransfer = pallet_localchain_transfer;
	#[runtime::pallet_index(13)]
	pub type BlockSealSpec = pallet_block_seal_spec;
	#[runtime::pallet_index(14)]
	pub type Domains = pallet_domains;
	#[runtime::pallet_index(15)]
	pub type PriceIndex = pallet_price_index;
	#[runtime::pallet_index(16)]
	pub type Authorship = pallet_authorship;
	#[runtime::pallet_index(17)]
	pub type Grandpa = pallet_grandpa;
	// Block Seal uses notebooks and ticks
	#[runtime::pallet_index(18)]
	pub type BlockSeal = pallet_block_seal;
	// NOTE: BlockRewards must come after seal (on_finalize uses seal info)
	#[runtime::pallet_index(19)]
	pub type BlockRewards = pallet_block_rewards;
	// NOTE: must come after MiningSlot so `is_new_frame_started` is available in initialize.
	#[runtime::pallet_index(20)]
	pub type Mint = pallet_mint;
	#[runtime::pallet_index(21)]
	pub type Balances = pallet_balances<Instance1>;
	#[runtime::pallet_index(22)]
	pub type Ownership = pallet_balances<Instance2>;
	#[runtime::pallet_index(23)]
	pub type TxPause = pallet_tx_pause;
	#[runtime::pallet_index(24)]
	pub type TransactionPayment = pallet_transaction_payment;
	#[runtime::pallet_index(25)]
	pub type Utility = pallet_utility;
	#[runtime::pallet_index(26)]
	pub type Sudo = pallet_sudo;

	#[runtime::pallet_index(31)]
	pub type Treasury = pallet_treasury;
	#[runtime::pallet_index(32)]
	pub type FeeControl = pallet_fee_control;
	#[runtime::pallet_index(34)]
	pub type OperationalAccounts = pallet_operational_accounts;
	#[runtime::pallet_index(35)]
	pub type EthereumVerifier = pallet_ethereum_verifier;
	#[runtime::pallet_index(36)]
	pub type CrosschainTransfer = pallet_crosschain_transfer;
}

argon_runtime_common::call_filters!();
argon_runtime_common::vault_admin_fee_refund_policy!();
argon_runtime_common::deal_with_fees!();
argon_runtime_common::inject_runtime_vars!();
argon_runtime_common::inject_common_apis!();

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = InsideBoth<BaseCallFilter, TxPause>;
	/// The block type for the runtime.
	type Block = Block;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = HashOutput;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = ConstU16<{ argon_primitives::ADDRESS_PREFIX }>;
	type MaxConsumers = ConstU32<16>;
}

/// This pallet is intended to be used as a shortterm security measure.
impl pallet_tx_pause::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PauseOrigin = EnsureRoot<AccountId>;
	type UnpauseOrigin = EnsureRoot<AccountId>;
	type WhitelistedCalls = TxPauseWhitelistedCalls;
	type MaxNameLen = ConstU32<256>;
	type WeightInfo = weights::pallet_tx_pause::WeightInfo<Runtime>;
}

impl pallet_block_seal_spec::Config for Runtime {
	type TargetComputeBlockPercent = TargetComputeBlockPercent;
	type AuthorityProvider = MiningSlot;
	type MaxActiveNotaries = MaxActiveNotaries;
	type NotebookProvider =
		use_unless_benchmark!(Notebook, benchmarking::BenchmarkNotebookProvider<Runtime>);
	type TickProvider = use_unless_benchmark!(Ticks, benchmarking::BenchmarkTickProvider);
	type WeightInfo = pallet_block_seal_spec::weights::WithProviderWeights<
		Runtime,
		weights::pallet_block_seal_spec::WeightInfo<Runtime>,
	>;
	type TargetBlockVotes = TargetBlockVotes;
	type HistoricalComputeBlocksForAverage = SealSpecComputeHistoryToTrack;
	type HistoricalVoteBlocksForAverage = SealSpecVoteHistoryForAverage;
	type SealInherent = BlockSeal;
}

pub struct NotebookTickProvider;
impl Get<Tick> for NotebookTickProvider {
	fn get() -> Tick {
		let schedule = Ticks::voting_schedule();
		schedule.notebook_tick()
	}
}

impl pallet_block_rewards::Config for Runtime {
	type WeightInfo = pallet_block_rewards::weights::WithProviderWeights<
		Runtime,
		weights::pallet_block_rewards::WeightInfo<Runtime>,
	>;
	type ArgonCurrency = Balances;
	type OwnershipCurrency = Ownership;
	type Balance = Balance;
	type BlockSealerProvider =
		use_unless_benchmark!(BlockSeal, benchmarking::BenchmarkBlockSealerProvider<AccountId>);
	type BlockRewardAccountsProvider = use_unless_benchmark!(
		MiningSlot,
		benchmarking::BenchmarkBlockRewardAccountsProvider<AccountId>
	);
	type NotaryProvider = use_unless_benchmark!(Notaries, benchmarking::BenchmarkNotaryProvider);
	type NotebookProvider =
		use_unless_benchmark!(Notebook, benchmarking::BenchmarkNotebookProvider<Runtime>);
	type TickProvider = use_unless_benchmark!(Ticks, benchmarking::BenchmarkTickProvider);
	type StartingArgonsPerBlock = StartingArgonsPerBlock;
	type StartingOwnershipTokensPerBlock = StartingOwnershipTokensPerBlock;
	type IncrementalGrowth = IncrementalGrowth;
	type HalvingTicks = HalvingTicks;
	type HalvingBeginTicks = HalvingBeginTick;
	type MinerPayoutPercent = MinerPayoutPercent;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type EventHandler = use_unless_benchmark!(Mint, ());
	type PayoutHistoryBlocks = PayoutHistoryBlocks;
	type PriceProvider = PriceIndex;
	type CohortBlockRewardsToKeep = BlockRewardsCohortHistoryToKeep;
	type EpochTicks = EpochTicks;
	type PerBlockArgonReducerPercent = BlockRewardsDampener;
}

impl pallet_domains::Config for Runtime {
	type WeightInfo = ();
	type DomainExpirationTicks = DomainExpirationTicks;
	type NotebookTick = NotebookTickProvider;
	type HistoricalPaymentAddressTicksToKeep = HistoricalPaymentAddressTicksToKeep;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = Digests;
	type EventHandler = ();
}

impl pallet_digests::Config for Runtime {
	type WeightInfo = ();
	type NotebookVerifyError = NotebookVerifyError;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = (BlockSealSpec, Ticks);
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

pub struct MultiBlockPerTickEnabled;
impl Get<bool> for MultiBlockPerTickEnabled {
	fn get() -> bool {
		!MiningSlot::is_registered_mining_active()
	}
}

impl pallet_ticks::Config for Runtime {
	type WeightInfo = weights::pallet_ticks::WeightInfo<Runtime>;
	type Digests = Digests;
}

pub struct GetCurrentFrameId;
impl Get<FrameId> for GetCurrentFrameId {
	fn get() -> FrameId {
		MiningSlot::current_frame_id()
	}
}

impl pallet_vaults::Config for Runtime {
	type WeightInfo = pallet_vaults::weights::WithProviderWeights<
		Runtime,
		weights::pallet_vaults::WeightInfo<Runtime>,
	>;
	type Currency = Balances;
	type OwnershipCurrency = Ownership;
	type Balance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxPendingTermModificationsPerTick = MaxPendingTermModificationsPerTick;
	type MiningFrameProvider = MiningSlot;
	type GetBitcoinNetwork = BitcoinUtxos;
	type BitcoinBlockHeightChange = BitcoinUtxos;
	type TicksPerBitcoinBlock = TicksPerBitcoinBlock;
	type TicksPerFrame = use_unless_benchmark!(GetTicksPerFrame, ConstU64<10>);
	type TickProvider = use_unless_benchmark!(Ticks, benchmarking::BenchmarkTickProvider);
	type CurrentFrameId = GetCurrentFrameId;
	type MaxVaults = MaxVaults;
	type MaxPendingCosignsPerVault = MaxPendingCosignsPerVault;
	type RevenueCollectionExpirationFrames = LockReleaseCosignDeadlineFrames;
	type OperationalMinimumVaultSecuritization = OperationalMinimumVaultSecuritization;
	type OperationalMinimumVaultLockTicks = OperationalMinimumVaultLockTicks;
	type RecentCapacityDropBlockWindow = RecentCapacityDropBlockWindow;
	type MaxRecentCapacityDropsPerVault = MaxRecentCapacityDropsPerVault;
	type CapacityDropAttemptUnit = CapacityDropAttemptUnit;
	type OperationalAccountsHook = use_unless_benchmark!(OperationalAccounts, ());
	type OperationalAccountProvider = use_unless_benchmark!(
		OperationalAccounts,
		benchmarking::BenchmarkOperationalAccountProvider<AccountId>
	);
	type CollectBlockerProvider = use_unless_benchmark!(CrosschainTransfer, ());
}

pub struct GetTicksPerFrame;
impl Get<Tick> for GetTicksPerFrame {
	fn get() -> Tick {
		pallet_mining_slot::MiningConfig::<Runtime>::get().ticks_between_slots
	}
}

pub struct DidStartNewFrame;
impl Get<bool> for DidStartNewFrame {
	fn get() -> bool {
		MiningSlot::is_new_frame_started().is_some()
	}
}
pub struct BitcoinSignatureVerifier;
impl BitcoinVerifier<Runtime> for BitcoinSignatureVerifier {}
impl pallet_bitcoin_locks::Config for Runtime {
	type WeightInfo = weights::pallet_bitcoin_locks::WeightInfo<Runtime>;
	type Currency = Balances;
	type Balance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type LockEvents =
		use_unless_benchmark!((Mint,), benchmarking::BenchmarkUtxoLockEvents<AccountId, Balance>);
	type BitcoinUtxoTracker =
		use_unless_benchmark!(BitcoinUtxos, benchmarking::BenchmarkBitcoinUtxoTracker);
	type PriceProvider =
		use_unless_benchmark!(PriceIndex, benchmarking::BenchmarkPriceProvider<Balance>);
	type BitcoinSignatureVerifier = use_unless_benchmark!(
		BitcoinSignatureVerifier,
		benchmarking::BenchmarkBitcoinSignatureVerifier
	);
	type BitcoinBlockHeightChange =
		use_unless_benchmark!(BitcoinUtxos, benchmarking::BenchmarkBitcoinBlockHeightChange);
	type GetBitcoinNetwork =
		use_unless_benchmark!(BitcoinUtxos, benchmarking::BenchmarkBitcoinNetwork);
	type VaultProvider = use_unless_benchmark!(
		Vaults,
		benchmarking::BenchmarkBitcoinVaultProvider<Balances, AccountId, Balance>
	);
	type ArgonTicksPerDay = TicksPerDay;
	type MaxConcurrentlyReleasingLocks = MaxConcurrentlyReleasingLocks;
	type LockDurationBlocks = BitcoinLockDurationBlocks;
	type LockReclamationBlocks = BitcoinLockReclamationBlocks;
	type LockReleaseCosignDeadlineFrames = LockReleaseCosignDeadlineFrames;
	type OrphanedUtxoReleaseExpiryFrames = OrphanedUtxoReleaseExpiryFrames;
	type TicksPerBitcoinBlock = TicksPerBitcoinBlock;
	type CurrentFrameId =
		use_unless_benchmark!(GetCurrentFrameId, benchmarking::BenchmarkCurrentFrameId);
	type MaxConcurrentlyExpiringLocks = MaxConcurrentlyExpiringLocks;
	type CurrentTick = use_unless_benchmark!(Ticks, benchmarking::BenchmarkCurrentTick);
	type MaxBtcPriceTickAge = MaxBtcPriceTickAge;
	type DidStartNewFrame =
		use_unless_benchmark!(DidStartNewFrame, benchmarking::BenchmarkDidStartNewFrame);
}

pub struct GrandpaSlotRotation;

impl OnNewSlot<AccountId> for GrandpaSlotRotation {
	type Key = GrandpaId;
	fn rotate_grandpas(
		_current_frame_id: FrameId,
		_removed_authorities: Vec<(&AccountId, Self::Key)>,
		_added_authorities: Vec<(&AccountId, Self::Key)>,
	) {
		let next_authorities: AuthorityList = Grandpa::grandpa_authorities();

		// TODO: we need to be able to run multiple grandpas on a single miner before activating
		// 	changing the authorities. We want to activate a trailing 3 hours of miners who closed
		//  blocks to activate a more decentralized grandpa process
		// for (_, authority_id) in removed_authorities {
		// 	if let Some(index) = next_authorities.iter().position(|x| x.0 == authority_id) {
		// 		next_authorities.remove(index);
		// 	}
		// }
		// for (_, authority_id) in added_authorities {
		// 	next_authorities.push((authority_id, 1));
		// }

		log::info!("Scheduling grandpa change");
		if let Err(err) = Grandpa::schedule_change(next_authorities, 0, None) {
			log::error!("Failed to schedule grandpa change: {err:?}");
		}
		pallet_grandpa::CurrentSetId::<Runtime>::mutate(|x| *x += 1);
	}
}

pub struct TicksSinceGenesis;
impl Get<Tick> for TicksSinceGenesis {
	fn get() -> Tick {
		Ticks::ticks_since_genesis()
	}
}

parameter_types! {
	pub const MinimumArgonsPerContributor: Balance = 100 * ARGON; // 100 argons minimum
	pub const MaxPendingUnlocksPerFrame: u32 = 1000;
	pub const TreasuryExitDelayFrames: FrameId = 10;
}

impl pallet_treasury::Config for Runtime {
	type WeightInfo = pallet_treasury::weights::WithProviderWeights<
		Runtime,
		weights::pallet_treasury::WeightInfo<Runtime>,
	>;
	type Balance = Balance;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type TreasuryVaultProvider = use_unless_benchmark!(
		Vaults,
		benchmarking::BenchmarkBitcoinVaultProvider<Balances, AccountId, Balance>
	);
	type PriceProvider =
		use_unless_benchmark!(PriceIndex, benchmarking::BenchmarkPriceProvider<Balance>);
	type MaxTreasuryContributors = MaxTreasuryContributors;
	type MinimumArgonsPerContributor = MinimumArgonsPerContributor;
	type PalletId = TreasuryInternalPalletId;
	type MiningBidPoolAccount = TreasuryMiningBidPoolAccount;
	type TreasuryReservesAccount = TreasuryReservesAccount;
	type PercentForTreasuryReserves = PercentForTreasuryReserves;
	type MaxVaultsPerPool = MaxVaultsPerPool;
	type MaxPendingUnlocksPerFrame = MaxPendingUnlocksPerFrame;
	type TreasuryExitDelayFrames = TreasuryExitDelayFrames;
	type MiningFrameTransitionProvider = MiningSlot;
	type OperationalAccountsHook = use_unless_benchmark!(OperationalAccounts, ());
}

impl pallet_mining_slot::Config for Runtime {
	type WeightInfo = pallet_mining_slot::weights::WithProviderWeights<
		Runtime,
		weights::pallet_mining_slot::WeightInfo<Runtime>,
	>;
	type FramesPerMiningTerm = FramesPerMiningTerm;
	type MinCohortSize = MinCohortSize;
	type MaxCohortSize = MaxCohortSize;
	type InitialArgonotsPerSeat = InitialArgonotsPerSeat;
	type ArgonotBidCollateralMultiple = ArgonotBidCollateralMultiple;
	type TargetPricePerSeat = TargetPricePerSeat;
	type PricePerSeatAdjustmentDamper = PricePerSeatAdjustmentDamper;
	type Balance = Balance;
	type OwnershipCurrency = Ownership;
	type ArgonCurrency = Balances;
	type PriceProvider =
		use_unless_benchmark!(PriceIndex, benchmarking::BenchmarkPriceProvider<Balance>);
	type RuntimeHoldReason = RuntimeHoldReason;
	type MiningBidPoolAccount = TreasuryMiningBidPoolAccount;
	type OperationalAccountProvider = use_unless_benchmark!(
		OperationalAccounts,
		benchmarking::BenchmarkOperationalAccountProvider<AccountId>
	);
	type OperationalAccountsHook = use_unless_benchmark!(OperationalAccounts, ());
	type SlotEvents = use_unless_benchmark!(
		(GrandpaSlotRotation, BlockRewards, Treasury, Vaults),
		(GrandpaSlotRotation,)
	);
	type GrandpaRotationBlocks = GrandpaRotationBlocks;
	type MiningAuthorityId = BlockSealAuthorityId;
	type Keys = SessionKeys;
	type TickProvider = use_unless_benchmark!(Ticks, benchmarking::BenchmarkTickProvider);
	type BidIncrements = MiningSlotBidIncrement;
	type SealerInfo = BlockSeal;
}

impl pallet_block_seal::Config for Runtime {
	type AuthorityId = BlockSealAuthorityId;
	type WeightInfo = pallet_block_seal::weights::WithProviderWeights<
		Runtime,
		weights::pallet_block_seal::WeightInfo<Runtime>,
	>;
	type AuthorityProvider = use_unless_benchmark!(
		MiningSlot,
		benchmarking::BenchmarkAuthorityProvider<Runtime, BlockSealAuthorityId>
	);
	type NotebookProvider =
		use_unless_benchmark!(Notebook, benchmarking::BenchmarkNotebookProvider<Runtime>);
	type BlockSealSpecProvider = BlockSealSpec;
	type FindAuthor = Digests;
	type TickProvider = use_unless_benchmark!(Ticks, benchmarking::BenchmarkTickProvider);
	type EventHandler = use_unless_benchmark!(MiningSlot, ());
	type Digests = Digests;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_grandpa::WeightInfo<Runtime>;
	type MaxAuthorities = MaxGrandpas;
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
}

impl pallet_localchain_transfer::Config for Runtime {
	type WeightInfo = pallet_localchain_transfer::weights::WithProviderWeights<
		Runtime,
		weights::pallet_localchain_transfer::WeightInfo<Runtime>,
	>;
	type Argon = Balances;
	type Balance = Balance;
	type NotebookProvider =
		use_unless_benchmark!(Notebook, benchmarking::BenchmarkNotebookProvider<Runtime>);
	type NotaryProvider = use_unless_benchmark!(Notaries, benchmarking::BenchmarkNotaryProvider);
	type TickProvider = use_unless_benchmark!(Ticks, benchmarking::BenchmarkTickProvider);
	type EventHandler = use_unless_benchmark!(Mint, ());
	type PalletId = LocalchainTransferPalletId;
	type TransferExpirationTicks = TransferExpirationTicks;
	type MaxPendingTransfersOutPerBlock = MaxPendingTransfersOutPerBlock;
}

impl pallet_notebook::Config for Runtime {
	type WeightInfo = pallet_notebook::weights::WithProviderWeights<
		Runtime,
		weights::pallet_notebook::WeightInfo<Runtime>,
	>;
	type EventHandler = use_unless_benchmark!((LocalchainTransfer, BlockSealSpec, Domains), ());
	type NotaryProvider = use_unless_benchmark!(Notaries, benchmarking::BenchmarkNotaryProvider);
	type ChainTransferLookup = LocalchainTransfer;
	type BlockSealSpecProvider = BlockSealSpec;
	type TickProvider = use_unless_benchmark!(Ticks, benchmarking::BenchmarkTickProvider);
	type Digests = Digests;
}

impl pallet_notaries::Config for Runtime {
	type WeightInfo = ();
	type MaxActiveNotaries = MaxActiveNotaries;
	type MaxProposalHoldBlocks = MaxProposalHoldBlocks;
	type MaxProposalsPerBlock = MaxProposalsPerBlock;
	type MetaChangesTickDelay = MetaChangesTickDelay;
	type MaxTicksForKeyHistory = MaxTicksForKeyHistory;
	type MaxNotaryHosts = MaxNotaryHosts;
	type TickProvider = use_unless_benchmark!(Ticks, benchmarking::BenchmarkTickProvider);
}
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type ArgonToken = pallet_balances::Instance1;
impl pallet_balances::Config<ArgonToken> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = weights::pallet_balances_balances::WeightInfo<Runtime>;
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<ARGON_EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type MaxFreezes = ConstU32<2>;
	type DoneSlashHandler = ();
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
	type BlockNumberProvider = frame_system::Pallet<Runtime>;
}
impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

impl pallet_price_index::Config for Runtime {
	type WeightInfo = weights::pallet_price_index::WeightInfo<Runtime>;
	type Currency = Balances;
	type Balance = Balance;
	type MaxDowntimeTicksBeforeReset = MaxDowntimeTicksBeforeReset;
	type MaxPriceAgeInTicks = MaxPriceAgeInTicks;
	type CurrentFrameId =
		use_unless_benchmark!(GetCurrentFrameId, benchmarking::BenchmarkCurrentFrameId);
	type MiningFrameTransitionProvider =
		use_unless_benchmark!(MiningSlot, benchmarking::BenchmarkCurrentFrameId);
	type CurrentTick = use_unless_benchmark!(Ticks, benchmarking::BenchmarkCurrentTick);
	type MaxArgonotFloorHistoryFrames = MaxArgonotFloorHistoryFrames;
	type MaxArgonotAverageHistoryFrames = MaxArgonotAverageHistoryFrames;
	type MaxArgonChangePerTickAwayFromTarget = MaxArgonChangePerTickAwayFromTarget;
	type MaxArgonTargetChangePerTick = MaxArgonTargetChangePerTick;
}

pub struct GetMinimumSatoshisPerLock;
impl Get<Satoshis> for GetMinimumSatoshisPerLock {
	fn get() -> Satoshis {
		MinimumSatoshis::<Runtime>::get()
	}
}

impl pallet_bitcoin_utxos::Config for Runtime {
	type WeightInfo = pallet_bitcoin_utxos::weights::WithProviderWeights<
		Runtime,
		weights::pallet_bitcoin_utxos::WeightInfo<Runtime>,
	>;
	type EventHandler = use_unless_benchmark!(BitcoinLocks, ());
	type MaxPendingConfirmationUtxos = MaxPendingConfirmationUtxos;
	type MaxPendingFundingExpirationsPerBlock = MaxPendingFundingExpirationsPerBlock;
	type MaxCandidateUtxosPerLock = MaxCandidateUtxosPerLock;
	type MaxPendingConfirmationBlocks = MaxPendingConfirmationBlocks;
	type MinimumSatoshisPerCandidateUtxo = GetMinimumSatoshisPerLock;
	type MaximumSatoshiThresholdFromExpected = MaximumSatoshiThresholdFromExpected;
}

impl pallet_mint::Config for Runtime {
	type WeightInfo = ();
	type Currency = Balances;
	type PriceProvider = PriceIndex;
	type Balance = Balance;
	type MaxPendingMintUtxos = MaxPendingMintUtxos;
	type BlockRewardAccountsProvider = MiningSlot;
	type MaxMintHistoryToMaintain = MaxMintHistoryToMaintain;
	type MaxPossibleMiners = MaxPossibleMiners;
	type MiningFrameProvider = MiningSlot;
}

pub(crate) type OwnershipToken = pallet_balances::Instance2;
impl pallet_balances::Config<OwnershipToken> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = weights::pallet_balances_ownership::WeightInfo<Runtime>;
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<ARGONOT_EXISTENTIAL_DEPOSIT>;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Runtime, OwnershipToken>,
		AccountId,
		pallet_balances::AccountData<Balance>,
	>;

	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type MaxFreezes = ConstU32<2>;
	type DoneSlashHandler = ();
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = weights::pallet_sudo::WeightInfo<Runtime>;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, DealWithFees<Runtime>>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = ArgonWeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
	type WeightInfo = weights::pallet_transaction_payment::WeightInfo<Runtime>;
}

impl pallet_crosschain_transfer::Config for Runtime {
	type Balance = Balance;
	type EthereumBurnAccount = CrosschainTransferEthereumBurnAccount;
	type NativeCurrency = Balances;
	type OwnershipCurrency = Ownership;
	type RuntimeHoldReason = RuntimeHoldReason;
	type EthereumVerifier = use_unless_benchmark!(
		EthereumVerifier,
		benchmarking::BenchmarkCrosschainTransferEthereumVerifier
	);
	type OperationalAccountsHook = use_unless_benchmark!(OperationalAccounts, ());
	type VaultProvider = use_unless_benchmark!(Vaults, benchmarking::BenchmarkBitcoinVaultProvider<Balances, AccountId, Balance>);
	type TreasuryPoolProvider = use_unless_benchmark!(Treasury, benchmarking::BenchmarkOperationalAccountsTreasuryPoolProvider<AccountId, Balance>);
	type PriceProvider =
		use_unless_benchmark!(PriceIndex, benchmarking::BenchmarkPriceProvider<Balance>);
	type CurrentFrameId =
		use_unless_benchmark!(GetCurrentFrameId, benchmarking::BenchmarkCurrentFrameId);
	type CurrentTick = Ticks;
	type TickProvider = use_unless_benchmark!(Ticks, benchmarking::BenchmarkTickProvider);
	type RecentTransferRetentionTicks = RecentTransferRetentionTicks;
	type MaxActivitiesPerReceiptProof = MaxActivitiesPerReceiptProof;
	type MaxReceiptProofsPerExtrinsic = MaxReceiptProofsPerExtrinsic;
	type MaxCouncilMembers = MaxCouncilMembers;
	type MaxQueueApprovalsPerCall = MaxQueueApprovalsPerCall;
	type TransferOutValidityEthereumBlocks = TransferOutValidityEthereumBlocks;
	type MaxVerifiedExecutionBlockAgeTicks = MaxVerifiedExecutionBlockAgeTicks;
	type TransferOutMintingAuthorityTipBasisPoints = TransferOutMintingAuthorityTipBasisPoints;
	type MinTransferCollateralIncrement = MinTransferCollateralIncrement;
	type DefaultMinimumMintingAuthorityMicrogonValue = DefaultMinimumMintingAuthorityMicrogonValue;
	type MaxPendingTransferOutsPerDestinationChain = MaxPendingTransferOutsPerDestinationChain;
	type CouncilRotationFrames = CouncilRotationFrames;
	type WeightInfo = pallet_crosschain_transfer::WithProviderWeights<
		Runtime,
		weights::pallet_crosschain_transfer::WeightInfo<Runtime>,
	>;
}
impl pallet_operational_accounts::Config for Runtime {
	type Balance = Balance;
	type FrameProvider = MiningSlot;
	type MaxAvailableReferrals = MaxAvailableOperationalReferrals;
	type MaxExpiredReferralCodeCleanupsPerBlock = MaxExpiredReferralCodeCleanupsPerBlock;
	type MaxEncryptedServerLen = MaxEncryptedServerLen;
	type OperationalMinimumVaultSecuritization = OperationalMinimumVaultSecuritization;
	type BitcoinLockSizeForReferral = BitcoinLockSizeForReferral;
	type MiningSeatsForOperational = MiningSeatsForOperational;
	type MiningSeatsPerReferral = MiningSeatsPerReferral;
	type ReferralBonusEveryXOperationalSponsees = ReferralBonusEveryXOperationalSponsees;
	type OperationalReferralReward = OperationalActivationReward;
	type OperationalReferralBonusReward = OperationalReferralBonusReward;
	type VaultProvider = use_unless_benchmark!(
		Vaults,
		benchmarking::BenchmarkOperationalAccountsVaultProvider<Balance, AccountId>
	);
	type MiningSlotProvider = use_unless_benchmark!(
		MiningSlot,
		benchmarking::BenchmarkOperationalAccountsMiningSlotProvider<AccountId>
	);
	type TreasuryPoolProvider = use_unless_benchmark!(
		Treasury,
		benchmarking::BenchmarkOperationalAccountsTreasuryPoolProvider<AccountId, Balance>
	);
	type UniswapTransferProvider = use_unless_benchmark!(
		CrosschainTransfer,
		benchmarking::BenchmarkOperationalAccountsUniswapTransferProvider
	);
	type OperationalRewardsPayer =
		use_unless_benchmark!(Treasury, benchmarking::BenchmarkOperationalRewardsPayer);
	type WeightInfo = pallet_operational_accounts::WithProviderWeights<
		Runtime,
		weights::pallet_operational_accounts::WeightInfo<Runtime>,
	>;
}

impl pallet_ethereum_verifier::Config for Runtime {
	type FreeHeadersInterval = EthereumFreeHeadersInterval;
	type WeightInfo = weights::pallet_ethereum_verifier::WeightInfo<Runtime>;
}
impl pallet_fee_control::Config for Runtime {
	type Balance = Balance;
	type FeelessCallTxPoolKeyProviders = ();
	type CallTxPoolKeyProviders = (BitcoinLocks, EthereumVerifier, CrosschainTransfer);
	type CallTxValidityProviders = (EthereumVerifier, CrosschainTransfer);
	type TransactionSponsorProviders = ProxyFeeDelegate<Runtime>;
	type CallFeeRefundProviders = VaultAdminFeeRefundPolicy;
}
