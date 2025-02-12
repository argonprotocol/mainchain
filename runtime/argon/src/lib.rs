#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
mod benchmark;

mod migrations;
pub mod weights;

use frame_support::weights::ConstantMultiplier;
use frame_system::EnsureSignedBy;
use ismp::{module::IsmpModule, router::IsmpRouter, Error};
pub use pallet_notebook::NotebookVerifyError;
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter};

pub use argon_runtime_common::config::NotaryRecordT;
use argon_runtime_common::prelude::*;

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use sp_runtime::impl_opaque_keys;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
#[cfg(feature = "std")]
use sp_version::NativeVersion;

impl_opaque_keys! {
	pub struct SessionKeys {
	pub grandpa: Grandpa,
	pub block_seal_authority: MiningSlot,
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
		RuntimeTask
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
	pub type ChainTransfer = pallet_chain_transfer;
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

	#[runtime::pallet_index(27)]
	pub type Ismp = pallet_ismp;
	#[runtime::pallet_index(28)]
	pub type IsmpGrandpa = ismp_grandpa;
	#[runtime::pallet_index(29)]
	pub type Hyperbridge = pallet_hyperbridge;
	#[runtime::pallet_index(30)]
	pub type TokenGateway = pallet_token_gateway;
}

argon_runtime_common::inject_runtime_vars!();
argon_runtime_common::inject_common_apis!();
argon_runtime_common::call_filters!();
argon_runtime_common::deal_with_fees!();

parameter_types! {
	// The hyperbridge parachain on Polkadot
	pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Polkadot(3367));
}

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
	type WeightInfo = pallet_tx_pause::weights::SubstrateWeight<Runtime>;
}

impl pallet_block_seal_spec::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type TargetComputeBlockPercent = TargetComputeBlockPercent;
	type AuthorityProvider = MiningSlot;
	type MaxActiveNotaries = MaxActiveNotaries;
	type NotebookProvider = Notebook;
	type TickProvider = Ticks;
	type WeightInfo = pallet_block_seal_spec::weights::SubstrateWeight<Runtime>;
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
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_block_rewards::weights::SubstrateWeight<Runtime>;
	type ArgonCurrency = Balances;
	type OwnershipCurrency = Ownership;
	type Balance = Balance;
	type BlockSealerProvider = BlockSeal;
	type BlockRewardAccountsProvider = MiningSlot;
	type NotaryProvider = Notaries;
	type NotebookProvider = Notebook;
	type TickProvider = Ticks;
	type StartingArgonsPerBlock = StartingArgonsPerBlock;
	type StartingOwnershipTokensPerBlock = StartingOwnershipTokensPerBlock;
	type IncrementalGrowth = IncrementalGrowth;
	type HalvingTicks = HalvingTicks;
	type HalvingBeginTick = HalvingBeginTick;
	type MinerPayoutPercent = MinerPayoutPercent;
	type MaturationBlocks = MaturationBlocks;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type EventHandler = Mint;
}

impl pallet_domains::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_domains::weights::SubstrateWeight<Runtime>;
	type DomainExpirationTicks = DomainExpirationTicks;
	type NotebookTick = NotebookTickProvider;
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

pub struct MultiBlockPerTickEnabled;
impl Get<bool> for MultiBlockPerTickEnabled {
	fn get() -> bool {
		!MiningSlot::is_registered_mining_active()
	}
}

impl pallet_ticks::Config for Runtime {
	type WeightInfo = ();
	type Digests = Digests;
}

impl pallet_vaults::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_vaults::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type Balance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MinimumObligationAmount = MinimumObligationAmount;
	type TicksPerDay = TicksPerDay;
	type MaxPendingTermModificationsPerTick = MaxPendingTermModificationsPerTick;
	type MinTermsModificationTickDelay = MinTermsModificationTickDelay;
	type MiningArgonIncreaseTickDelay = VaultFundingModificationDelay;
	type MiningSlotProvider = MiningSlot;
	type GetBitcoinNetwork = BitcoinUtxos;
	type BitcoinBlockHeightChange = BitcoinUtxos;
	type TickProvider = Ticks;
	type MaxConcurrentlyExpiringObligations = MaxConcurrentlyExpiringObligations;
	type EventHandler = (BitcoinLocks,);
	type EnableRewardSharing = EnableRewardSharing;
}

pub struct BitcoinSignatureVerifier;
impl BitcoinVerifier<Runtime> for BitcoinSignatureVerifier {}
impl pallet_bitcoin_locks::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_bitcoin_locks::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type Balance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type LockEvents = (Mint,);
	type BitcoinUtxoTracker = BitcoinUtxos;
	type PriceProvider = PriceIndex;
	type BitcoinSignatureVerifier = BitcoinSignatureVerifier;
	type BitcoinBlockHeight = BitcoinUtxos;
	type GetBitcoinNetwork = BitcoinUtxos;
	type BitcoinObligationProvider = Vaults;
	type ArgonTicksPerDay = TicksPerDay;
	type MaxConcurrentlyReleasingLocks = MaxConcurrentlyReleasingLocks;
	type LockDurationBlocks = BitcoinLockDurationBlocks;
	type LockReclamationBlocks = BitcoinLockReclamationBlocks;
	type LockReleaseCosignDeadlineBlocks = LockReleaseCosignDeadlineBlocks;
	type TickProvider = Ticks;
}

pub struct GrandpaSlotRotation;

impl OnNewSlot<AccountId> for GrandpaSlotRotation {
	type Key = GrandpaId;
	fn rotate_grandpas(
		_current_slot_id: SlotId,
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
			log::error!("Failed to schedule grandpa change: {:?}", err);
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

impl pallet_mining_slot::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_mining_slot::weights::SubstrateWeight<Runtime>;
	type MaxMiners = MaxMiners;
	type MaxCohortSize = MaxCohortSize;
	type ArgonotsPercentAdjustmentDamper = ArgonotsPercentAdjustmentDamper;
	type MinimumArgonotsPerSeat = ConstU128<EXISTENTIAL_DEPOSIT>;
	type MaximumArgonotProrataPercent = MaximumArgonotProrataPercent;
	type TargetBidsPerSlot = TargetBidsPerSlot;
	type Balance = Balance;
	type OwnershipCurrency = Ownership;
	type RuntimeHoldReason = RuntimeHoldReason;
	type BondedArgonsProvider = Vaults;
	type SlotEvents = (GrandpaSlotRotation,);
	type GrandpaRotationBlocks = GrandpaRotationBlocks;
	type MiningAuthorityId = BlockSealAuthorityId;
	type Keys = SessionKeys;
	type TickProvider = Ticks;
}

impl pallet_block_seal::Config for Runtime {
	type AuthorityId = BlockSealAuthorityId;
	type WeightInfo = pallet_block_seal::weights::SubstrateWeight<Runtime>;
	type AuthorityProvider = MiningSlot;
	type NotebookProvider = Notebook;
	type BlockSealSpecProvider = BlockSealSpec;
	type FindAuthor = Digests;
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

impl pallet_chain_transfer::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_chain_transfer::weights::SubstrateWeight<Runtime>;
	type Argon = Balances;
	type Balance = Balance;
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type NotebookProvider = Notebook;
	type NotebookTick = NotebookTickProvider;
	type EventHandler = Mint;
	type PalletId = ChainTransferPalletId;
	type TransferExpirationTicks = TransferExpirationTicks;
	type MaxPendingTransfersOutPerBlock = MaxPendingTransfersOutPerBlock;
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
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type MaxFreezes = ConstU32<2>;
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

impl pallet_price_index::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = pallet_price_index::weights::SubstrateWeight<Runtime>;
	type Balance = Balance;
	type MaxDowntimeTicksBeforeReset = MaxDowntimeTicksBeforeReset;
	type MaxPriceAgeInTicks = MaxPriceAgeInTicks;
	type CurrentTick = Ticks;
	type MaxArgonChangePerTickAwayFromTarget = MaxArgonChangePerTickAwayFromTarget;
	type MaxArgonTargetChangePerTick = MaxArgonTargetChangePerTick;
}

impl pallet_bitcoin_utxos::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_bitcoin_utxos::weights::SubstrateWeight<Runtime>;
	type EventHandler = BitcoinLocks;
	type MaxPendingConfirmationUtxos = MaxPendingConfirmationUtxos;
	type MaxPendingConfirmationBlocks = MaxPendingConfirmationBlocks;
}

impl pallet_mint::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_mint::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type PriceProvider = PriceIndex;
	type Balance = Balance;
	type MaxPendingMintUtxos = MaxPendingMintUtxos;
	type BlockRewardAccountsProvider = MiningSlot;
}

pub(crate) type OwnershipToken = pallet_balances::Instance2;
impl pallet_balances::Config<OwnershipToken> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
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
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, DealWithFees<Runtime>>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

impl pallet_token_gateway::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// Configured as Pallet Ismp
	type Dispatcher = Ismp;
	// Configured as Pallet balances
	type NativeCurrency = Balances;
	// AssetAdmin account to register new assets on the chain. We don't use this
	type AssetAdmin = TokenAdmin;
	// Configured as Pallet Assets
	type Assets = OwnershipTokenAsset;
	// The Native asset Id
	type NativeAssetId = NativeAssetId;
	// The precision of the native asset
	type Decimals = Decimals;
	type CreateOrigin = EnsureSignedBy<TokenAdmins, AccountId>;
	type WeightInfo = ();
	type EvmToSubstrate = ();
}

impl pallet_ismp::Config for Runtime {
	// configure the runtime event
	type RuntimeEvent = RuntimeEvent;
	// Permissioned origin who can create or update consensus clients
	type AdminOrigin = EnsureRoot<AccountId>;
	// The pallet_timestamp pallet
	type TimestampProvider = Timestamp;
	// The balance type for the currency implementation
	type Balance = Balance;
	// The currency implementation that is offered to relayers
	type Currency = Balances;
	// The state machine identifier for this state machine
	type HostStateMachine = HostStateMachine;
	// Optional coprocessor for incoming requests/responses
	type Coprocessor = Coprocessor;
	// Router implementation for routing requests/responses to their respective modules
	type Router = Router;
	// Supported consensus clients
	type ConsensusClients = (
		// Add the grandpa or beefy consensus client here
		ismp_grandpa::consensus::GrandpaConsensusClient<Runtime>,
	);
	// Weight provider for local modules
	type WeightProvider = ();
	// Optional merkle mountain range overlay tree, for cheaper outgoing request proofs.
	// You most likely don't need it, just use the `NoOpMmrTree`
	type OffchainDB = ();
}

impl ismp_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
	type WeightInfo = weights::ismp_grandpa::WeightInfo<Runtime>;
}

impl pallet_hyperbridge::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
}

// Add the token gateway pallet to your ISMP router
#[derive(Default)]
pub struct Router;

impl IsmpRouter for Router {
	fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
		match id.as_slice() {
			id if TokenGateway::is_token_gateway(id) => Ok(Box::new(TokenGateway::default())),
			_ => Err(Error::ModuleNotFound(id))?,
		}
	}
}

argon_runtime_common::token_asset!(Ownership, ChainTransfer::hyperbridge_token_admin());
