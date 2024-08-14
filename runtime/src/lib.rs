#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;
extern crate alloc;

use alloc::{boxed::Box, collections::btree_map::BTreeMap, vec, vec::Vec};
use codec::{Decode, Encode, MaxEncodedLen};
use core::marker::PhantomData;
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, KeyOwnerProofSystem, Randomness,
		StorageInfo,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		IdentityFee, Weight, WeightToFeePolynomial,
	},
	StorageValue,
};
use frame_support::{
	derive_impl,
	genesis_builder_helper::{build_state, get_preset},
	traits::{
		fungible, fungible::Balanced, Contains, Currency, Everything, Imbalance, InsideBoth,
		InstanceFilter, OnUnbalanced, StorageMapShim,
	},
	weights::{WeightToFeeCoefficient, WeightToFeeCoefficients},
	PalletId,
};
pub use frame_system::Call as SystemCall;
use frame_system::EnsureRoot;
use pallet_session::historical as pallet_session_historical;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use pallet_tx_pause::RuntimeCallNameOf;
use scale_info::TypeInfo;
use smallvec::smallvec;
use sp_api::impl_runtime_apis;
use sp_arithmetic::{traits::Zero, FixedPointNumber, FixedU128, Percent};
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::KeyTypeId, ConstU16, OpaqueMetadata, H256, U256};
use sp_debug_derive::RuntimeDebug;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
	create_runtime_str, generic,
	traits::{BlakeTwo256, Block as BlockT, NumberFor, One, OpaqueKeys},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, BoundedVec, DispatchError,
};
pub use sp_runtime::{Perbill, Permill};

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use argon_primitives::{
	bitcoin::{BitcoinHeight, BitcoinSyncStatus, Satoshis, UtxoRef, UtxoValue},
	block_seal::MiningAuthority,
	block_vote::VoteMinimum,
	digests::BlockVoteDigest,
	localchain::BestBlockVoteSeal,
	notary::{NotaryId, NotaryNotebookVoteDetails, NotaryNotebookVoteDigestDetails, NotaryRecord},
	notebook::NotebookNumber,
	tick::{Tick, Ticker},
	ArgonCPI, BlockSealAuthorityId, NotaryNotebookVotes, NotebookAuditResult, NotebookAuditSummary,
	PriceProvider, TickProvider, ESCROW_CLAWBACK_TICKS,
};
pub use argon_primitives::{
	AccountId, Balance, BlockHash, BlockNumber, HashOutput, Moment, Nonce, Signature,
};
pub use currency::*;
use pallet_bond::BitcoinVerifier;
pub use pallet_notebook::NotebookVerifyError;

use crate::opaque::SessionKeys;

// A few exports that help ease life for downstream crates.

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub type AccountData = pallet_balances::AccountData<Balance>;
/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use sp_runtime::impl_opaque_keys;
	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	use super::*;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlockHash>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub grandpa: Grandpa,
			pub block_seal_authority: MiningSlot,
		}
	}
}

/// Money matters.
pub mod currency {
	use argon_primitives::Balance;

	/// The existential deposit.
	pub const EXISTENTIAL_DEPOSIT: Balance = 500;

	pub const UNITS: Balance = 1_000;
	pub const CENTS: Balance = UNITS / 100;
	pub const MILLICENTS: Balance = CENTS / 1_000;
	pub const GRAND: Balance = CENTS * 100_000;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 100 * CENTS + (bytes as Balance) * 5 * MILLICENTS
	}
}

pub type ArgonBalancesCall = pallet_balances::Call<Runtime, ArgonToken>;

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("argon"),
	impl_name: create_runtime_str!("argon"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 100,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	/// We allow for 60 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::with_sensible_defaults(
			Weight::from_parts(60u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
			NORMAL_DISPATCH_RATIO,
		);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
}
/// Calls that cannot be paused by the tx-pause pallet.
pub struct TxPauseWhitelistedCalls;
/// All calls are pausable.
impl Contains<RuntimeCallNameOf<Runtime>> for TxPauseWhitelistedCalls {
	fn contains(_full_name: &RuntimeCallNameOf<Runtime>) -> bool {
		false
		// match (full_name.0.as_slice(), full_name.1.as_slice()) {
		// 	(b"Balances", b"transfer_keep_alive") => true,
		// 	_ => false,
		// }
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

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	/// example filter: https://github.com/AcalaNetwork/Acala/blob/f4b80d7200c19b78d3777e8a4a87bc6893740d23/runtime/karura/src/lib.rs#L198
	type BaseCallFilter = InsideBoth<Everything, TxPause>;
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
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const TargetComputeBlockPercent: Percent = Percent::from_percent(50); // aim for compute to take half of vote time
	pub const TargetBlockVotes: u32 = 50_000;
	pub const MinimumsChangePeriod: u32 = 60 * 24; // change block_seal_spec once a day

	pub const DefaultEscrowDuration: Tick = 60;
	pub const HistoricalPaymentAddressTicksToKeep: Tick = DefaultEscrowDuration::get() + ESCROW_CLAWBACK_TICKS + 10;

	pub const ArgonsPerBlock: u32 = 5_000;
	pub const StartingSharesPerBlock: u32 = 5_000;
	pub const HalvingBlocks: u32 = 2_100_000; // based on bitcoin, but 10x since we're block per minute
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
	type ChangePeriod = MinimumsChangePeriod;
	type SealInherent = BlockSeal;
	type TickProvider = Ticks;
	type MaxActiveNotaries = MaxActiveNotaries;
}

impl pallet_block_rewards::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_block_rewards::weights::SubstrateWeight<Runtime>;
	type ArgonCurrency = ArgonBalances;
	type SharesCurrency = ShareBalances;
	type Balance = Balance;
	type BlockSealerProvider = BlockSeal;
	type NotaryProvider = Notaries;
	type NotebookProvider = Notebook;
	type CurrentTick = Ticks;
	type ArgonsPerBlock = ArgonsPerBlock;
	type StartingSharesPerBlock = StartingSharesPerBlock;
	type HalvingBlocks = HalvingBlocks;
	type MinerPayoutPercent = MinerPayoutPercent;
	type MaturationBlocks = MaturationBlocks;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type EventHandler = Mint;
	type BlockRewardAccountsProvider = MiningSlot;
}

impl pallet_data_domain::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_data_domain::weights::SubstrateWeight<Runtime>;
	type TickProvider = Ticks;
	type DomainExpirationTicks = DomainExpirationTicks;
	type HistoricalPaymentAddressTicksToKeep = HistoricalPaymentAddressTicksToKeep;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = BlockSeal;
	type EventHandler = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = (BlockSealSpec, Ticks);
	type MinimumPeriod = ConstU64<500>;
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

	// Arbitrarily chosen. We keep these around for equivocation reporting in grandpa, and for
	// notary auditing using validators of finalized blocks.
	pub const SessionWindowsToKeepInHistory: u32 = 10;

	const BitcoinBlocksPerDay: BitcoinHeight = 6 * 24;
	pub const BitcoinBondDurationBlocks: BitcoinHeight = BitcoinBlocksPerDay::get() * 365; // 1 year
	pub const BitcoinBondReclamationBlocks: BitcoinHeight = BitcoinBlocksPerDay::get() * 30; // 30 days
	pub const UtxoUnlockCosignDeadlineBlocks: BitcoinHeight = BitcoinBlocksPerDay::get() * 5; // 5 days

	pub const SessionsPerMiningWindow: u32 = 2;

	pub const MaxSetIdSessionEntries: u32 = SessionsPerMiningWindow::get() * 2u32;
	// keep around for 2
	pub const ReportLongevity: u64 = BlocksPerDay::get() as u64 * 10;

	pub const MaxUnlockingUtxos: u32 = 1000;
	pub const MinBitcoinSatoshiAmount: Satoshis = 10_000_000; // 1/10th bitcoin minimum
	pub const MaxPendingTermModificationsPerBlock: u32 = 100;
	pub const MinTermsModificationBlockDelay: u32 = 1439; // must be at least one slot (day)
}

impl pallet_vaults::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_vaults::weights::SubstrateWeight<Runtime>;
	type Currency = ArgonBalances;
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
	type Currency = ArgonBalances;
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
	type MinimumBitcoinBondSatoshis = MinBitcoinSatoshiAmount;
	type ArgonBlocksPerDay = BlocksPerDay;
	type UtxoUnlockCosignDeadlineBlocks = UtxoUnlockCosignDeadlineBlocks;
	type BitcoinSignatureVerifier = BitcoinSignatureVerifier;
}

impl pallet_mining_slot::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_mining_slot::weights::SubstrateWeight<Runtime>;
	type MaxMiners = MaxMiners;
	type OwnershipCurrency = ShareBalances;
	type OwnershipPercentAdjustmentDamper = OwnershipPercentAdjustmentDamper;
	type TargetBidsPerSlot = TargetBidsPerSlot;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxCohortSize = MaxCohortSize;
	type SessionWindowsToKeepInHistory = SessionWindowsToKeepInHistory;
	type Balance = Balance;
	type BondProvider = Bonds;
	type SessionRotationsPerMiningWindow = SessionsPerMiningWindow;
}

impl pallet_block_seal::Config for Runtime {
	type AuthorityId = BlockSealAuthorityId;
	type WeightInfo = pallet_block_seal::weights::SubstrateWeight<Runtime>;
	type AuthorityProvider = MiningSlot;
	type NotebookProvider = Notebook;
	type BlockVotingProvider = BlockSealSpec;
	type TickProvider = Ticks;
	type DataDomainProvider = DataDomain;
	type EventHandler = MiningSlot;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_mining_slot::ValidatorIdOf<Self>;
	type ShouldEndSession = MiningSlot;
	type NextSessionRotation = MiningSlot;
	type SessionManager = pallet_session_historical::NoteHistoricalRoot<Self, MiningSlot>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session_historical::Config for Runtime {
	type FullIdentification = pallet_mining_slot::MinerHistory;
	type FullIdentificationOf = pallet_mining_slot::FullIdentificationOf<Runtime>;
}

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session_historical::IdentificationTuple<Self>;
	// TODO: mining_slot should deal with offenses
	type OnOffenceHandler = ();
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxAuthorities = MaxMiners;
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type EquivocationReportSystem =
		pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as sp_runtime::traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = UncheckedExtrinsic;
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
	type Currency = ArgonBalances;
	type Balance = Balance;
	type NotaryProvider = Notaries;
	type PalletId = ChainTransferPalletId;
	type TransferExpirationTicks = TransferExpirationTicks;
	type MaxPendingTransfersOutPerBlock = MaxPendingTransfersOutPerBlock;
	type NotebookProvider = Notebook;
	type EventHandler = Mint;
	type CurrentTick = Ticks;
}

impl pallet_notebook::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_notebook::weights::SubstrateWeight<Runtime>;
	type EventHandler = (ChainTransfer, BlockSealSpec, DataDomain);
	type NotaryProvider = Notaries;
	type ChainTransferLookup = ChainTransfer;
	type BlockVotingProvider = BlockSealSpec;
	type TickProvider = Ticks;
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

pub type NotaryRecordT = NotaryRecord<AccountId, BlockNumber, crate::MaxNotaryHosts>;

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
pub type ArgonNegativeImbalance<T> = <pallet_balances::Pallet<T, ArgonToken> as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

pub struct Author<R>(PhantomData<R>);
impl<R> OnUnbalanced<ArgonNegativeImbalance<R>> for Author<R>
where
	R: pallet_authorship::Config + pallet_balances::Config<ArgonToken>,
	AccountIdOf<R>: From<AccountId> + Into<AccountId>,
	<R as frame_system::Config>::RuntimeEvent: From<pallet_balances::Event<R, ArgonToken>>,
{
	fn on_nonzero_unbalanced(amount: ArgonNegativeImbalance<R>) {
		if let Some(author) = pallet_authorship::Pallet::<R>::author() {
			<pallet_balances::Pallet<R, ArgonToken>>::resolve_creating(&author, amount);
		} else {
			drop(amount);
		}
	}
}

pub struct DealWithFees<R>(PhantomData<R>);

impl<R> OnUnbalanced<fungible::Credit<R::AccountId, pallet_balances::Pallet<R, ArgonToken>>>
	for DealWithFees<R>
where
	R: pallet_authorship::Config + pallet_balances::Config<ArgonToken>,
	AccountIdOf<R>: From<AccountId> + Into<AccountId>,
	<R as frame_system::Config>::RuntimeEvent: From<pallet_balances::Event<R, ArgonToken>>,
{
	fn on_unbalanceds<B>(
		mut fees_then_tips: impl Iterator<
			Item = fungible::Credit<R::AccountId, pallet_balances::Pallet<R, ArgonToken>>,
		>,
	) {
		if let Some(mut fees) = fees_then_tips.next() {
			if let Some(tips) = fees_then_tips.next() {
				tips.merge_into(&mut fees);
			}
			if let Some(author) = pallet_authorship::Pallet::<R>::author() {
				let _ =
					<pallet_balances::Pallet<R, ArgonToken>>::resolve(&author, fees).map_err(drop);
			}
		}
	}
}

type ArgonToken = pallet_balances::Instance1;
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
				!matches!(c, RuntimeCall::ArgonBalances(..) | RuntimeCall::ShareBalances(..)),
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
	type Currency = ArgonBalances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}
impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = ArgonBalances;
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
	type MaxUtxoBirthBlocksOld = MaxBitcoinBirthBlocksOld;
	type MaxPendingConfirmationBlocks = MaxPendingConfirmationBlocks;
	type MaxPendingConfirmationUtxos = MaxPendingConfirmationUtxos;
}

impl pallet_mint::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_mint::weights::SubstrateWeight<Runtime>;
	type Currency = ArgonBalances;
	type Balance = Balance;
	type PriceProvider = PriceIndex;
	type MaxPendingMintUtxos = MaxPendingMintUtxos;
	type BlockRewardAccountsProvider = MiningSlot;
}

type SharesToken = pallet_balances::Instance2;
impl pallet_balances::Config<SharesToken> for Runtime {
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
		pallet_balances::Account<Runtime, SharesToken>,
		AccountId,
		pallet_balances::AccountData<Balance>,
	>;

	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<2>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

pub struct WageProtectorFee;

impl WeightToFeePolynomial for WageProtectorFee {
	type Balance = Balance;

	/// This function attempts to add some weight to larger transactions, but given the 3 digits of
	/// milligons to work with, it can be difficult to scale this properly.
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		let cpi = PriceIndex::get_argon_cpi().unwrap_or(ArgonCPI::zero());
		let mut p = 1_000; // milligons
		if cpi.is_positive() {
			let cpi = cpi.into_inner() / ArgonCPI::accuracy();
			let adjustment = (p * (cpi as u128) * 1_000).checked_div(1_000).unwrap_or_default();
			p += adjustment;
		}
		let q = 10 * Self::Balance::from(ExtrinsicBaseWeight::get().ref_time());

		smallvec![WeightToFeeCoefficient::<Self::Balance> {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<ArgonBalances, DealWithFees<Runtime>>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = WageProtectorFee;
	type LengthToFee = WageProtectorFee;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.

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
		RuntimeTask
	)]
	pub struct Runtime;
	#[runtime::pallet_index(0)]
	pub type System = frame_system;
	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp;
	#[runtime::pallet_index(2)]
	pub type Multisig = pallet_multisig;

	#[runtime::pallet_index(3)]
	pub type Proxy = pallet_proxy;
	#[runtime::pallet_index(4)]
	pub type Ticks = pallet_ticks;
	#[runtime::pallet_index(5)]
	pub type MiningSlot = pallet_mining_slot;
	#[runtime::pallet_index(6)]
	pub type BitcoinUtxos = pallet_bitcoin_utxos;
	#[runtime::pallet_index(7)]
	pub type Vaults = pallet_vaults;
	#[runtime::pallet_index(8)]
	pub type Bonds = pallet_bond;
	#[runtime::pallet_index(9)]
	pub type Notaries = pallet_notaries;
	#[runtime::pallet_index(10)]
	pub type Notebook = pallet_notebook;
	#[runtime::pallet_index(11)]
	pub type ChainTransfer = pallet_chain_transfer;
	#[runtime::pallet_index(12)]
	pub type BlockSealSpec = pallet_block_seal_spec;
	#[runtime::pallet_index(13)]
	pub type DataDomain = pallet_data_domain;
	#[runtime::pallet_index(14)]
	pub type PriceIndex = pallet_price_index;
	// NOTE: Authorship must be before session
	#[runtime::pallet_index(15)]
	pub type Authorship = pallet_authorship;
	#[runtime::pallet_index(16)]
	pub type Historical = pallet_session_historical;
	#[runtime::pallet_index(17)]
	pub type Session = pallet_session;
	#[runtime::pallet_index(18)]
	pub type BlockSeal = pallet_block_seal;
	// NOTE: BlockRewards must come after seal (on_finalize uses seal info)
	#[runtime::pallet_index(19)]
	pub type BlockRewards = pallet_block_rewards;
	#[runtime::pallet_index(20)]
	pub type Grandpa = pallet_grandpa;
	#[runtime::pallet_index(21)]
	pub type Offences = pallet_offences;
	#[runtime::pallet_index(22)]
	pub type Mint = pallet_mint;
	#[runtime::pallet_index(23)]
	pub type ArgonBalances = pallet_balances<Instance1>;
	#[runtime::pallet_index(24)]
	pub type ShareBalances = pallet_balances<Instance2>;
	#[runtime::pallet_index(25)]
	pub type TxPause = pallet_tx_pause;
	#[runtime::pallet_index(26)]
	pub type TransactionPayment = pallet_transaction_payment;
	#[runtime::pallet_index(27)]
	pub type Sudo = pallet_sudo;
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlockHash>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckMortality<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);
/// All migrations of the runtime, aside from the ones declared in the pallets.
///
/// This can be a tuple of types, each implementing `OnRuntimeUpgrade`.
#[allow(unused_parens)]
type Migrations = ();

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> alloc::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl argon_primitives::MiningApis<Block, AccountId, BlockSealAuthorityId> for Runtime {
		fn get_authority_id(account_id: &AccountId) -> Option<MiningAuthority< BlockSealAuthorityId, AccountId>> {
			MiningSlot::get_mining_authority(account_id)
		}
	}

	impl argon_primitives::BlockSealApis<Block, AccountId, BlockSealAuthorityId> for Runtime {
		fn vote_minimum() -> VoteMinimum {
			BlockSealSpec::vote_minimum()
		}

		fn compute_difficulty() -> u128 {
			BlockSealSpec::compute_difficulty()
		}

		fn create_vote_digest(tick: Tick, included_notebooks: Vec<NotaryNotebookVoteDigestDetails>) -> BlockVoteDigest {
			BlockSealSpec::create_block_vote_digest(tick, included_notebooks)
		}

		fn find_vote_block_seals(
			votes: Vec<NotaryNotebookVotes>,
			with_better_strength: U256,
		) -> Result<BoundedVec<BestBlockVoteSeal<AccountId, BlockSealAuthorityId>, ConstU32<2>>, DispatchError>{
			Ok(BlockSeal::find_vote_block_seals(votes,with_better_strength)?)
		}

	}

	impl argon_primitives::NotaryApis<Block, NotaryRecordT> for Runtime {
		fn notary_by_id(notary_id: NotaryId) -> Option<NotaryRecordT> {
			Notaries::notaries().iter().find(|a| a.notary_id == notary_id).cloned()
		}
		fn notaries() -> Vec<NotaryRecordT> {
			Notaries::notaries().iter().cloned().collect()
		}
	}

	impl pallet_mining_slot::MiningSlotApi<Block, BlockNumber> for Runtime {
		fn next_slot_era() -> (BlockNumber, BlockNumber) {
			MiningSlot::get_slot_era()
		}
	}

	impl argon_primitives::NotebookApis<Block, NotebookVerifyError> for Runtime {
		fn audit_notebook_and_get_votes(
			version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			header_hash: H256,
			vote_minimums: &BTreeMap<<Block as BlockT>::Hash, VoteMinimum>,
			bytes: &Vec<u8>,
			audit_dependency_summaries: Vec<NotebookAuditSummary>,
		) -> Result<NotebookAuditResult, NotebookVerifyError> {
			Notebook::audit_notebook(version, notary_id, notebook_number, header_hash, vote_minimums, bytes, audit_dependency_summaries)
		}

		fn decode_signed_raw_notebook_header(raw_header: Vec<u8>) -> Result<NotaryNotebookVoteDetails<<Block as BlockT>::Hash>, DispatchError> {
			Notebook::decode_signed_raw_notebook_header(raw_header)
		}

		fn latest_notebook_by_notary() -> BTreeMap<NotaryId, (NotebookNumber, Tick)> {
			Notebook::latest_notebook_by_notary()
		}
	}

	impl argon_primitives::TickApis<Block> for Runtime {
		fn current_tick() -> Tick {
			Ticks::current_tick()
		}
		fn ticker() -> Ticker {
			Ticks::ticker()
		}
		fn blocks_at_tick(tick: Tick) -> Vec<<Block as BlockT>::Hash> {
			Ticks::blocks_at_tick(tick)
		}
	}

	impl argon_primitives::BitcoinApis<Block,Balance> for Runtime {
		fn get_sync_status() -> Option<BitcoinSyncStatus> {
			BitcoinUtxos::get_sync_status()
		}

		fn active_utxos() -> Vec<(Option<UtxoRef>, UtxoValue)>{
			BitcoinUtxos::active_utxos()
		}

		fn redemption_rate(satoshis: Satoshis) -> Option<Balance> {
			Bonds::get_redemption_price(&satoshis).ok()
		}

		fn market_rate(satoshis: Satoshis) -> Option<Balance> {
			PriceIndex::get_bitcoin_argon_price(satoshis)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((sp_consensus_grandpa::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_grandpa::OpaqueKeyOwnershipProof::new)
		}
	}


	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, TrackedStorageKey};

			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			vec![]
		}
	}
}
#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, ArgonTokens]
		[pallet_balances, SharesTokens]
		[pallet_timestamp, Timestamp]
		[pallet_ticks, Ticks]
		[pallet_data_domain, DataDomain]
		[pallet_block_seal_spec, VoteEligibility]
		[pallet_block_rewards, BlockRewards]
		[pallet_mining_slot, MiningSlot]
		[pallet_bond, Bonds]
		[pallet_bitcoin_utxos, BitcoinMint]
		[pallet_mint, Mint]
		[pallet_session, Session]
		[pallet_block_seal, BlockSeal]
		[pallet_authorship, Authorship]
		[pallet_sudo, Sudo]
		[pallet_grandpa, Grandpa]
		[pallet_offences, Offences]
		[pallet_notaries, Notaries]
		[pallet_chain_transfer, ChainTransfer]
	);
}
