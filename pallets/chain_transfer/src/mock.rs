use alloc::collections::btree_map::BTreeMap;
use env_logger::{Builder, Env};
use frame_support::{
	derive_impl, parameter_types,
	traits::{Currency, StorageMapShim},
	PalletId,
};
use frame_system::{pallet_prelude::BlockNumberFor, EnsureRoot};
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, StateCommitment, StateMachineClient,
		VerifiedCommitments,
	},
	host::{IsmpHost, StateMachine},
	messaging::Proof,
	module::IsmpModule,
	router::{IsmpRouter, RequestResponse},
	Error as IsmpError,
};
use pallet_ismp::NoOpMmrTree;
use sp_core::{crypto::AccountId32, ConstU32, ConstU64, H256};
use sp_keyring::AccountKeyring::Alice;
use sp_runtime::{traits::IdentityLookup, BuildStorage};

use crate as pallet_chain_transfer;
use argon_primitives::{
	notary::NotaryId, tick::Tick, BlockSealAuthorityId, NotebookNumber, NotebookProvider,
	NotebookSecret,
};

pub type Balance = u128;
pub(crate) type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = BlockNumberFor<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Ownership: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
		ChainTransfer: pallet_chain_transfer,
		Hyperbridge: pallet_hyperbridge,
		Ismp: pallet_ismp,
		Timestamp: pallet_timestamp,

	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {

	pub static ExistentialDeposit: Balance = 10;
	pub const MinimumBondAmount:u128 = 1_000;
	pub const BlocksPerYear:u32 = 1440*365;

	pub const LocalchainAccountId :u32 = 1;
	pub static MaxNotebookBlocksToRemember :u32 = 1;
	pub const MaxNotebookTransfers :u32 = 1;
	pub static MaxPendingTransfersOutPerBlock :u32 = 1;
	pub static TransferExpirationTicks :u64 = 2;
	pub static NotebookTick: Tick = 1;

	pub const LocalchainPalletId: PalletId = PalletId(*b"loclchai");

	pub static BlockSealers: BTreeMap<BlockNumber, Vec<BlockSealAuthorityId>> = BTreeMap::new();

	pub static LockedNotaries: BTreeMap<NotaryId, Tick> = BTreeMap::new();

	pub static IsProofOfCompute: bool = false;
}

type ArgonToken = pallet_balances::Instance1;
impl pallet_balances::Config<ArgonToken> for Test {
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type ReserveIdentifier = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<1>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

type OwnershipToken = pallet_balances::Instance2;
impl pallet_balances::Config<OwnershipToken> for Test {
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type ReserveIdentifier = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test, OwnershipToken>,
		AccountId32,
		pallet_balances::AccountData<Balance>,
	>;
	type WeightInfo = ();
	type FreezeIdentifier = RuntimeFreezeReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<1>;
	type RuntimeHoldReason = RuntimeHoldReason;
}

pub fn set_argons(account_id: &AccountId32, amount: Balance) {
	let _ = Balances::make_free_balance_be(account_id, amount);
	drop(Balances::issue(amount));
}
pub fn set_ownership(account_id: &AccountId32, amount: Balance) {
	let _ = Ownership::make_free_balance_be(account_id, amount);
	drop(Ownership::issue(amount));
}

pub struct StaticNotebookProvider;
impl NotebookProvider for StaticNotebookProvider {
	fn get_eligible_tick_votes_root(_: NotaryId, _tick: Tick) -> Option<(H256, NotebookNumber)> {
		None
	}
	fn notebooks_at_tick(_: Tick) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)> {
		todo!()
	}
	fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)> {
		todo!()
	}
	fn is_notary_locked_at_tick(notary_id: NotaryId, tick: Tick) -> bool {
		if let Some(lock_tick) = LockedNotaries::get().get(&notary_id) {
			return *lock_tick <= tick;
		}
		false
	}
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

parameter_types! {
	pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Polkadot(3367));
	pub const HostStateMachine: StateMachine = StateMachine::Substrate(*b"argn");
}

impl pallet_chain_transfer::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Argon = Balances;
	type OwnershipTokens = Ownership;
	type ExistentialDeposit = ExistentialDeposit;
	type Balance = Balance;
	type MaxPendingTransfersOutPerBlock = MaxPendingTransfersOutPerBlock;
	type PalletId = LocalchainPalletId;
	type TransferExpirationTicks = TransferExpirationTicks;
	type NotebookProvider = StaticNotebookProvider;
	type EventHandler = ();
	type NotebookTick = NotebookTick;
	type Dispatcher = Hyperbridge;
}

impl pallet_hyperbridge::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type IsmpHost = Ismp;
}

impl pallet_ismp::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = EnsureRoot<AccountId32>;
	type HostStateMachine = HostStateMachine;
	type Balance = Balance;
	type ConsensusClients = (MockConsensusClient,);
	type Coprocessor = Coprocessor;
	type Currency = Balances;
	type Mmr = NoOpMmrTree<Test>;
	type Router = ModuleRouter;
	type TimestampProvider = Timestamp;
	type WeightProvider = ();
}

#[derive(Default)]
pub struct ModuleRouter;
impl IsmpRouter for ModuleRouter {
	fn module_for_id(&self, _id: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
		Ok(Box::new(ChainTransfer::default()))
	}
}

pub const MOCK_CONSENSUS_CLIENT_ID: [u8; 4] = [1u8; 4];

#[derive(Default)]
pub struct MockConsensusClient;

impl ConsensusClient for MockConsensusClient {
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		_cs_id: ismp::consensus::ConsensusStateId,
		_trusted_consensus_state: Vec<u8>,
		_proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), IsmpError> {
		Ok(Default::default())
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
		_trusted_consensus_state: Vec<u8>,
		_proof_1: Vec<u8>,
		_proof_2: Vec<u8>,
	) -> Result<(), IsmpError> {
		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		MOCK_CONSENSUS_CLIENT_ID
	}

	fn state_machine(&self, _id: StateMachine) -> Result<Box<dyn StateMachineClient>, IsmpError> {
		let state_machine: Box<dyn StateMachineClient> = Box::new(MockStateMachine);
		Ok(state_machine)
	}
}

/// Mock State Machine
pub struct MockStateMachine;
impl StateMachineClient for MockStateMachine {
	fn verify_membership(
		&self,
		_host: &dyn IsmpHost,
		_item: RequestResponse,
		_root: StateCommitment,
		_proof: &Proof,
	) -> Result<(), IsmpError> {
		Ok(())
	}

	fn receipts_state_trie_key(&self, _request: RequestResponse) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		_keys: Vec<Vec<u8>>,
		_root: StateCommitment,
		_proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, IsmpError> {
		Ok(Default::default())
	}
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_chain_transfer::GenesisConfig::<Test> {
		use_evm_test_networks: true,
		token_admin: Some(Alice.to_account_id()),
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::new(t)
}
