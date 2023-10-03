#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

extern crate alloc;
#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

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
	traits::{Currency, OnUnbalanced, StorageMapShim},
	PalletId,
};
pub use frame_system::Call as SystemCall;
use pallet_session::historical as pallet_session_historical;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, CurrencyAdapter, Multiplier};
use sp_api::impl_runtime_apis;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
	create_runtime_str, generic,
	traits::{
		AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, One, OpaqueKeys,
		Verify,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	AccountId32, ApplyExtrinsicResult, MultiSignature,
};
pub use sp_runtime::{Perbill, Permill};
use sp_std::{collections::btree_map::BTreeMap, prelude::*, vec::Vec};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use ulx_primitives::{
	block_seal::{AuthorityDistance, AuthorityProvider},
	BlockSealAuthorityId, NextWork,
};

use crate::opaque::SessionKeys;
// A few exports that help ease life for downstream crates.
use crate::wage_protector::WageProtectorFee;

pub type ArgonBalancesCall = pallet_balances::Call<Runtime, ArgonToken>;

mod macros;
pub mod wage_protector;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// A timestamp: milliseconds since the unix epoch.
pub type Moment = u64;

pub type AccountData = pallet_balances::AccountData<Balance>;

pub type BlockHash = BlakeTwo256;

pub type BondId = u64;
pub type BondFundId = u32;

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
			pub block_seal_authority: Cohorts,
		}
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("ulixee"),
	impl_name: create_runtime_str!("ulixee"),
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
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::with_sensible_defaults(
			Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
			NORMAL_DISPATCH_RATIO,
		);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	/// example filter: https://github.com/AcalaNetwork/Acala/blob/f4b80d7200c19b78d3777e8a4a87bc6893740d23/runtime/karura/src/lib.rs#L198
	type BaseCallFilter = frame_support::traits::Everything;
	/// The block type for the runtime.
	type Block = Block;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const TargetBlockTime: u32 = 60_000;
	pub const DifficultyBlockChangePeriod: u32 = 60 * 24; // change difficulty once a day
}

impl pallet_difficulty::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_difficulty::weights::SubstrateWeight<Runtime>;
	type TargetBlockTime = TargetBlockTime;
	type BlockChangePeriod = DifficultyBlockChangePeriod;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = BlockSeal;
	type EventHandler = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = Difficulty;
	type MinimumPeriod = ConstU64<500>;
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxCohortSize: u32 = 250; // this means cohorts last 40 days
	pub const BlocksBetweenCohorts: u32 = prod_or_fast!(1440, 4); // going to add a cohort every day
	pub const MaxValidators: u32 = 10_000; // must multiply cleanly by MaxCohortSize
	pub const SessionRotationPeriod: u32 = prod_or_fast!(120, 2); // must be cleanly divisible by BlocksBetweenCohorts
	pub const Offset: u32 = 0;
	pub const OwnershipPercentDamper: u32 = 80;

	pub const NextCohortBufferToStopAcceptingBids: u32 = prod_or_fast!(10, 1);
	pub const MaxConcurrentlyExpiringBondFunds: u32 = 1000;
	pub const MaxConcurrentlyExpiringBonds: u32 = 1000;
	pub const MinimumBondAmount:u128 = 1_000;
	pub const BlocksPerYear:u32 = 1440 * 365;

	const ValidatorWindow: u32 = (MaxValidators::get() / MaxCohortSize::get()) * BlocksBetweenCohorts::get();
	const SessionsPerWindow: u32 = ValidatorWindow::get() / SessionRotationPeriod::get();
	// Arbitrarily chosen. We keep these around for equivocation reporting in grandpa, and for
	// notary auditing using validators of finalized blocks.
	pub const SessionIndicesToKeepInHistory: u32 = SessionsPerWindow::get() * 10;

	// How long to keep grandpa set ids around for equivocations
	pub const MaxSetIdSessionEntries: u32 = SessionsPerWindow::get() * 2u32;
	pub const ReportLongevity: u64 = ValidatorWindow::get() as u64 * 2;
	pub const HistoricalBlockSealersToKeep: u32 = BlocksBetweenCohorts::get();
}

impl pallet_bonds::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_bonds::weights::SubstrateWeight<Runtime>;
	type Currency = ArgonBalances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type BondFundId = BondFundId;
	type BondId = BondId;
	type MinimumBondAmount = MinimumBondAmount;
	type MaxConcurrentlyExpiringBonds = MaxConcurrentlyExpiringBonds;
	type MaxConcurrentlyExpiringBondFunds = MaxConcurrentlyExpiringBondFunds;
	type Balance = Balance;
	type BlocksPerYear = BlocksPerYear;
}

impl pallet_cohorts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_cohorts::weights::SubstrateWeight<Runtime>;
	type MaxValidators = MaxValidators;
	type OwnershipCurrency = UlixeeBalances;
	type OwnershipPercentDamper = OwnershipPercentDamper;
	type NextCohortBufferToStopAcceptingBids = NextCohortBufferToStopAcceptingBids;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxCohortSize = MaxCohortSize;
	type SessionIndicesToKeepInHistory = SessionIndicesToKeepInHistory;
	type BlocksBetweenCohorts = BlocksBetweenCohorts;
	type Balance = Balance;
	type BondId = BondId;
	type BondProvider = Bonds;
}

impl pallet_block_seal::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_block_seal::weights::SubstrateWeight<Runtime>;
	type AuthorityProvider = Cohorts;
	type AuthorityId = BlockSealAuthorityId;
	type HistoricalBlockSealersToKeep = HistoricalBlockSealersToKeep;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_cohorts::ValidatorOf<Self>;
	type ShouldEndSession = pallet_session::PeriodicSessions<SessionRotationPeriod, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<SessionRotationPeriod, Offset>;
	type SessionManager = pallet_session_historical::NoteHistoricalRoot<Self, Cohorts>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session_historical::Config for Runtime {
	type FullIdentification = pallet_cohorts::MinerHistory;
	type FullIdentificationOf = pallet_cohorts::FullIdentificationOf<Runtime>;
}

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session_historical::IdentificationTuple<Self>;
	// TODO: cohorts should deal with offenses
	type OnOffenceHandler = ();
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxAuthorities = MaxValidators;
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
	pub const LocalchainPalletId: PalletId = PalletId(*b"locrelay");

	/// How long a transfer should remain in storage before returning.
	pub const TransferExpirationBlocks: u32 = 1400 * 10;

	/// How many transfers out can be queued per block
	pub const MaxPendingTransfersOutPerBlock: u32 = 1000;

	/// How many transfers can be in a single notebook
	pub const MaxNotebookTransfers: u32 = 10_000;

	/// How many auditors are expected to sign a notary block.
	pub const RequiredNotebookAuditors: u32 = 5; // half of seal signers

	/// Number of blocks to keep around for preventing notebook double-submit
	pub const MaxNotebookBlocksToRemember: u32 = 10;

}

impl pallet_localchain_relay::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_localchain_relay::weights::SubstrateWeight<Runtime>;
	type HistoricalBlockSealersLookup = BlockSeal;
	type Balance = Balance;
	type Currency = ArgonBalances;

	type NotaryProvider = NotaryAdmin;
	type PalletId = LocalchainPalletId;
	type LocalchainAccountId = AccountId;
	type TransferExpirationBlocks = TransferExpirationBlocks;
	type MaxPendingTransfersOutPerBlock = MaxPendingTransfersOutPerBlock;
	type MaxNotebookTransfers = MaxNotebookTransfers;
	type RequiredNotebookAuditors = RequiredNotebookAuditors;
	type MaxNotebookBlocksToRemember = MaxNotebookBlocksToRemember;
}

parameter_types! {
	pub const MaxActiveNotaries: u32 = 25; // arbitrarily set
	pub const MaxProposalHoldBlocks: u32 = 1440 * 14; // 2 weeks to approve
	pub const MaxProposalsPerBlock: u32 = 10;
	pub const MetaChangesBlockDelay: u32 = 1;
	/// Max host ips a notary can provide
	pub const MaxNotaryHosts: u32 = 4;
	pub const MaxBlocksForKeyHistory: u32 = 1440 * 2; // keep for 2 days.. only used for notebook submission
}

impl pallet_notary_admin::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_notary_admin::weights::SubstrateWeight<Runtime>;
	type MaxActiveNotaries = MaxActiveNotaries;
	type MaxProposalHoldBlocks = MaxProposalHoldBlocks;
	type MaxProposalsPerBlock = MaxProposalsPerBlock;
	type MetaChangesBlockDelay = MetaChangesBlockDelay;
	type MaxNotaryHosts = MaxNotaryHosts;
	type MaxBlocksForKeyHistory = MaxBlocksForKeyHistory;
}

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: u128 = 500;

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		if let Some(author) = BlockSeal::author() {
			ArgonBalances::resolve_creating(&author, amount);
		} else {
			drop(amount);
		}
	}
}

pub struct DealWithFees;
type NegativeImbalance = <ArgonBalances as Currency<AccountId>>::NegativeImbalance;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
		if let Some(fees) = fees_then_tips.next() {
			Author::on_unbalanced(fees);
			if let Some(tips) = fees_then_tips.next() {
				Author::on_unbalanced(tips);
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
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxHolds = ConstU32<100>;
}

type UlixeeToken = pallet_balances::Instance2;
impl pallet_balances::Config<UlixeeToken> for Runtime {
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
		pallet_balances::Account<Runtime, UlixeeToken>,
		AccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxHolds = ConstU32<50>;
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = CurrencyAdapter<ArgonBalances, DealWithFees>;
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
construct_runtime!(
	pub struct Runtime {
		System: frame_system,
		Timestamp: pallet_timestamp,
		Cohorts: pallet_cohorts,
		Bonds: pallet_bonds,
		NotaryAdmin: pallet_notary_admin,
		LocalchainRelay: pallet_localchain_relay,
		Difficulty: pallet_difficulty,// Consensus support.
		// Authorship must be before session in order to note author in the correct session and era
		// for im-online.
		Authorship: pallet_authorship,
		Historical: pallet_session_historical,
		Session: pallet_session,
		BlockSeal: pallet_block_seal,
		Grandpa: pallet_grandpa,
		Offences: pallet_offences,
		ArgonBalances: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		UlixeeBalances: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
		TransactionPayment: pallet_transaction_payment,
		Sudo: pallet_sudo,
	}
);

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
);

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
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, ArgonTokens]
		[pallet_balances, UlixeeTokens]
		[pallet_timestamp, Timestamp]
		[pallet_difficulty, Difficulty]
		[pallet_cohorts, Cohorts]
		[pallet_bonds, Bonds]
		[pallet_session, Session]
		[pallet_block_seal, BlockSeal]
		[pallet_authorship, Authorship]
		[pallet_sudo, Sudo]
		[pallet_grandpa, Grandpa]
		[pallet_offences, Offences],
		[pallet_notary_admin, NotaryAdmin],
		[pallet_localchain_relay, LocalchainRelay],
	);
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
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

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
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

	impl ulx_primitives::UlxConsensusApi<Block> for Runtime {
		fn next_work() -> NextWork {
			NextWork {
				work_type: BlockSeal::work_type(),
				difficulty: Difficulty::difficulty(),
				min_seal_signers: BlockSeal::min_seal_signers(),
				closest_x_authorities_required: BlockSeal::closest_x_authorities_required(),
			}
		}

		fn calculate_easing(tax_amount: u128, validators: u8) -> u128 {
			 Difficulty::calculate_easing(tax_amount, validators)
		}
	}

	impl ulx_primitives::block_seal::AuthorityApis<Block> for Runtime {
		fn authorities() -> Vec<BlockSealAuthorityId> {
			Cohorts::authorities()
		}
		fn authorities_by_index() -> BTreeMap<u16, BlockSealAuthorityId> {
			Cohorts::authorities_by_index()
		}
		fn block_peers(account_id: AccountId32) -> Vec<AuthorityDistance<BlockSealAuthorityId>> {
			let block_number = System::block_number();
			let current_block_hash = System::block_hash(block_number);
			BlockSeal::block_peers(current_block_hash, account_id)
		}
		fn active_authorities() -> u16 {
			Cohorts::authority_count().into()
		}
	}

	impl pallet_cohorts::CohortsApi<Block, BlockNumber> for Runtime {

		fn next_cohort_block_period() -> (BlockNumber, BlockNumber) {
			Cohorts::get_next_cohort_period()
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
}
