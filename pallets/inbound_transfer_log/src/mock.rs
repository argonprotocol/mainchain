use crate as pallet_inbound_transfer_log;
use pallet_prelude::*;

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		InboundTransferLog: pallet_inbound_transfer_log,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = ();
	type DbWeight = RocksDbWeight;
}

parameter_types! {
	pub const InboundTransfersRetentionBlocks: BlockNumberFor<Test> = 5;
	pub const MaxTransfersToRetainPerBlock: u32 = 10;
	pub const MaxInboundTransferBytes: u32 = 10 * 1_024;
	pub const MinimumTransferMicrogonsToRecord: Balance = 1;
	pub const OwnershipAssetId: u32 = 2;
}

impl pallet_inbound_transfer_log::Config for Test {
	type InboundTransfersRetentionBlocks = InboundTransfersRetentionBlocks;
	type MaxTransfersToRetainPerBlock = MaxTransfersToRetainPerBlock;
	type MaxInboundTransferBytes = MaxInboundTransferBytes;
	type MinimumTransferMicrogonsToRecord = MinimumTransferMicrogonsToRecord;
	type OwnershipAssetId = OwnershipAssetId;
	type WeightInfo = ();
	type OperationalAccountsHook = ();
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t: &mut Storage| {})
}

pub mod gateway {
	use super::*;
	use frame_support::{
		dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo},
		traits::AsEnsureOriginWithArg,
	};
	use frame_system::{EnsureRoot, EnsureSigned};
	use ismp::{
		consensus::{ConsensusClient, ConsensusClientId, ConsensusStateId, StateMachineClient},
		dispatcher::{DispatchRequest, FeeMetadata, IsmpDispatcher},
		error::Error as IsmpError,
		host::{IsmpHost, StateMachine},
		messaging::{MessageWithWeight, Proof},
		module::IsmpModule,
		router::{IsmpRouter, PostResponse, RequestResponse},
	};
	use pallet_token_gateway::types::EvmToSubstrate;
	use sp_core::{H160, H256};
	use sp_runtime::AccountId32;

	type Block = frame_system::mocking::MockBlock<GatewayTest>;

	frame_support::construct_runtime!(
		pub enum GatewayTest {
			System: frame_system,
			Balances: pallet_balances,
			Assets: pallet_assets,
			Ismp: pallet_ismp,
			Hyperbridge: pallet_hyperbridge,
			TokenGateway: pallet_token_gateway,
			InboundTransferLog: pallet_inbound_transfer_log,
		}
	);

	#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
	impl frame_system::Config for GatewayTest {
		type AccountId = AccountId32;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Block = Block;
		type AccountData = pallet_balances::AccountData<Balance>;
		type DbWeight = RocksDbWeight;
	}

	parameter_types! {
		pub const ExistentialDeposit: Balance = 1;
		pub const AssetDeposit: Balance = 0;
		pub const AssetAccountDeposit: Balance = 0;
		pub const MetadataDepositBase: Balance = 0;
		pub const MetadataDepositPerByte: Balance = 0;
		pub const ApprovalDeposit: Balance = 0;
		pub const StringLimit: u32 = 50;
		pub const RemoveItemsLimit: u32 = 1_000;

		pub const HostStateMachine: StateMachine = StateMachine::Substrate(*b"tstt");
		pub const Coprocessor: Option<StateMachine> = None;
		pub const NativeAssetId: u32 = 0;
		pub const Decimals: u8 = 6;
		pub const GatewayInboundTransfersRetentionBlocks: BlockNumberFor<GatewayTest> = 5;
		pub static TokenGatewayAdmin: AccountId32 = AccountId32::new([42u8; 32]);
	}

	impl pallet_balances::Config for GatewayTest {
		type RuntimeEvent = RuntimeEvent;
		type RuntimeHoldReason = RuntimeHoldReason;
		type RuntimeFreezeReason = RuntimeFreezeReason;
		type WeightInfo = ();
		type Balance = Balance;
		type DustRemoval = ();
		type ExistentialDeposit = ExistentialDeposit;
		type AccountStore = System;
		type ReserveIdentifier = [u8; 8];
		type FreezeIdentifier = ();
		type MaxLocks = ();
		type MaxReserves = ();
		type MaxFreezes = ();
		type DoneSlashHandler = ();
	}

	impl pallet_assets::Config for GatewayTest {
		type RuntimeEvent = RuntimeEvent;
		type Balance = Balance;
		type AssetId = u32;
		type AssetIdParameter = u32;
		type Currency = Balances;
		type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId32>>;
		type ForceOrigin = EnsureRoot<AccountId32>;
		type AssetDeposit = AssetDeposit;
		type AssetAccountDeposit = AssetAccountDeposit;
		type MetadataDepositBase = MetadataDepositBase;
		type MetadataDepositPerByte = MetadataDepositPerByte;
		type ApprovalDeposit = ApprovalDeposit;
		type StringLimit = StringLimit;
		type Freezer = ();
		type Holder = ();
		type Extra = ();
		type CallbackHandle = ();
		type WeightInfo = ();
		type RemoveItemsLimit = RemoveItemsLimit;
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper = ();
	}

	#[derive(Default)]
	pub struct MockRouter;

	impl IsmpRouter for MockRouter {
		fn module_for_id(&self, _bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
			Ok(Box::new(MockModule))
		}
	}

	pub struct MockModule;
	impl IsmpModule for MockModule {}

	#[derive(Default)]
	pub struct MockConsensusClient;

	impl ConsensusClient for MockConsensusClient {
		fn verify_consensus(
			&self,
			_host: &dyn IsmpHost,
			_consensus_state_id: ConsensusStateId,
			_trusted_consensus_state: Vec<u8>,
			_proof: Vec<u8>,
		) -> Result<(Vec<u8>, ismp::consensus::VerifiedCommitments), IsmpError> {
			unimplemented!("mock consensus")
		}

		fn verify_fraud_proof(
			&self,
			_host: &dyn IsmpHost,
			_trusted_consensus_state: Vec<u8>,
			_proof_1: Vec<u8>,
			_proof_2: Vec<u8>,
		) -> Result<(), IsmpError> {
			unimplemented!("mock fraud proof")
		}

		fn consensus_client_id(&self) -> ConsensusClientId {
			*b"mock"
		}

		fn state_machine(
			&self,
			_id: StateMachine,
		) -> Result<Box<dyn StateMachineClient>, IsmpError> {
			Ok(Box::new(MockStateMachineClient))
		}
	}

	pub struct MockStateMachineClient;
	impl StateMachineClient for MockStateMachineClient {
		fn verify_membership(
			&self,
			_host: &dyn IsmpHost,
			_item: RequestResponse,
			_root: ismp::consensus::StateCommitment,
			_proof: &Proof,
		) -> Result<(), IsmpError> {
			unimplemented!("mock membership")
		}

		fn receipts_state_trie_key(&self, _request: RequestResponse) -> Vec<Vec<u8>> {
			unimplemented!("mock receipts key")
		}

		fn verify_state_proof(
			&self,
			_host: &dyn IsmpHost,
			_keys: Vec<Vec<u8>>,
			_root: ismp::consensus::StateCommitment,
			_proof: &Proof,
		) -> Result<sp_std::collections::btree_map::BTreeMap<Vec<u8>, Option<Vec<u8>>>, IsmpError>
		{
			unimplemented!("mock state proof")
		}
	}

	#[derive(Default)]
	pub struct MockDispatcher;

	impl IsmpDispatcher for MockDispatcher {
		type Account = AccountId32;
		type Balance = Balance;

		fn dispatch_request(
			&self,
			_request: DispatchRequest,
			_fee: FeeMetadata<Self::Account, Self::Balance>,
		) -> Result<H256, anyhow::Error> {
			Err(IsmpError::CannotHandleMessage.into())
		}

		fn dispatch_response(
			&self,
			_response: PostResponse,
			_fee: FeeMetadata<Self::Account, Self::Balance>,
		) -> Result<H256, anyhow::Error> {
			Err(IsmpError::CannotHandleMessage.into())
		}
	}

	pub struct MockFeeHandler;
	impl pallet_ismp::fee_handler::FeeHandler for MockFeeHandler {
		fn on_executed(
			_messages: Vec<MessageWithWeight>,
			_events: Vec<ismp::events::Event>,
		) -> DispatchResultWithPostInfo {
			Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
		}
	}

	pub struct MockEvmToSubstrate;
	impl EvmToSubstrate<GatewayTest> for MockEvmToSubstrate {
		fn convert(addr: H160) -> AccountId32 {
			let mut bytes = [0u8; 32];
			bytes[12..].copy_from_slice(addr.as_bytes());
			AccountId32::from(bytes)
		}
	}

	pub struct MockTimestamp;
	impl frame_support::traits::UnixTime for MockTimestamp {
		fn now() -> core::time::Duration {
			core::time::Duration::from_secs(0)
		}
	}

	impl pallet_ismp::Config for GatewayTest {
		type AdminOrigin = EnsureRoot<AccountId32>;
		type TimestampProvider = MockTimestamp;
		type Balance = Balance;
		type Currency = Balances;
		type HostStateMachine = HostStateMachine;
		type Coprocessor = Coprocessor;
		type Router = MockRouter;
		type ConsensusClients = (MockConsensusClient,);
		type FeeHandler = MockFeeHandler;
		type OffchainDB = ();
	}

	impl pallet_hyperbridge::Config for GatewayTest {
		type IsmpHost = MockDispatcher;
	}

	impl pallet_token_gateway::Config for GatewayTest {
		type Dispatcher = MockDispatcher;
		type NativeCurrency = Balances;
		type AssetAdmin = TokenGatewayAdmin;
		type CreateOrigin = EnsureRoot<AccountId32>;
		type Assets = Assets;
		type NativeAssetId = NativeAssetId;
		type Decimals = Decimals;
		type EvmToSubstrate = MockEvmToSubstrate;
		type WeightInfo = ();
	}

	impl pallet_inbound_transfer_log::Config for GatewayTest {
		type InboundTransfersRetentionBlocks = GatewayInboundTransfersRetentionBlocks;
		type MaxTransfersToRetainPerBlock = MaxTransfersToRetainPerBlock;
		type MaxInboundTransferBytes = MaxInboundTransferBytes;
		type MinimumTransferMicrogonsToRecord = MinimumTransferMicrogonsToRecord;
		type OwnershipAssetId = OwnershipAssetId;
		type WeightInfo = ();
		type OperationalAccountsHook = ();
	}

	pub fn new_test_ext() -> TestState {
		new_test_with_genesis::<GatewayTest>(|_t: &mut Storage| {})
	}
}
