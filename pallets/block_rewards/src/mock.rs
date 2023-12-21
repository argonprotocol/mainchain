use env_logger::{Builder, Env};
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64, StorageMapShim},
};
use sp_core::{ConstU32, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup, NumberFor},
	BuildStorage,
};

use ulx_primitives::{
	notary::{NotaryProvider, NotarySignature},
	tick::Tick,
	BlockSealerInfo, BlockSealerProvider, NotaryId, NotebookNumber, NotebookProvider,
	NotebookSecret,
};

use crate as pallet_block_rewards;

type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type AccountId = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		BlockRewards: pallet_block_rewards,
		ArgonBalances: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		UlixeeBalances: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub type Balance = u128;
parameter_types! {

	pub static ExistentialDeposit: Balance = 10;
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
	type MaxHolds = ConstU32<100>;
}

type UlixeeToken = pallet_balances::Instance2;
impl pallet_balances::Config<UlixeeToken> for Test {
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type ReserveIdentifier = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test, UlixeeToken>,
		AccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type WeightInfo = ();
	type FreezeIdentifier = RuntimeFreezeReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<1>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxHolds = ConstU32<100>;
}

parameter_types! {
	pub static ArgonsPerBlock :u32 = 5_000;
	pub static StartingUlixeesPerBlock :u32 = 5_000;
	pub static HalvingBlocks :u32 = 100;
	pub static MaturationBlocks :u32 = 5;
	pub static MinerPayoutPercent :u32 = 75;
	pub static ActiveNotaries: Vec<NotaryId> = vec![1];
	pub static CurrentTick: Tick = 0;

	pub static NotebooksInBlock: Vec<(NotaryId, NotebookNumber, Tick)> = vec![];

	pub static BlockSealer:BlockSealerInfo<u64> = BlockSealerInfo {
		block_vote_rewards_account: 1,
		miner_rewards_account: 1,
	};
}

pub struct StaticBlockSealerProvider;
impl BlockSealerProvider<u64> for StaticBlockSealerProvider {
	fn get_sealer_info() -> BlockSealerInfo<u64> {
		BlockSealer::get()
	}
}

pub struct TestProvider;
impl NotaryProvider<Block> for TestProvider {
	fn verify_signature(_: NotaryId, _: NumberFor<Block>, _: &H256, _: &NotarySignature) -> bool {
		true
	}
	fn active_notaries() -> Vec<NotaryId> {
		ActiveNotaries::get()
	}
}
impl NotebookProvider for TestProvider {
	fn get_eligible_tick_votes_root(
		_notary_id: NotaryId,
		_tick: Tick,
	) -> Option<(H256, NotebookNumber)> {
		todo!()
	}
	fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)> {
		NotebooksInBlock::get()
	}
	fn notebooks_at_tick(_tick: Tick) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)> {
		todo!()
	}
	fn is_notary_locked_at_tick(_notary_id: NotaryId, _tick: Tick) -> bool {
		todo!()
	}
}

impl pallet_block_rewards::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type ArgonCurrency = ArgonBalances;
	type UlixeeCurrency = UlixeeBalances;
	type ArgonsPerBlock = ArgonsPerBlock;
	type StartingUlixeesPerBlock = StartingUlixeesPerBlock;
	type MaturationBlocks = MaturationBlocks;
	type Balance = Balance;
	type HalvingBlocks = HalvingBlocks;
	type MinerPayoutPercent = MinerPayoutPercent;
	type BlockSealerProvider = StaticBlockSealerProvider;
	type NotaryProvider = TestProvider;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type CurrentTick = CurrentTick;
	type NotebookProvider = TestProvider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
