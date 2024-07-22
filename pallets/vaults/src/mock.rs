use crate as pallet_vaults;
use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types, traits::Currency};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_core::{ConstU32, ConstU64};
use sp_runtime::BuildStorage;
use ulx_primitives::MiningSlotProvider;

pub type Balance = u128;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Vaults: pallet_vaults,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {

	pub static ExistentialDeposit: Balance = 10;
	pub const MinimumBondAmount:u128 = 1_000;
	pub const BlocksPerYear:u32 = 1440*365;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type ReserveIdentifier = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

pub fn set_argons(account_id: u64, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

parameter_types! {
	pub static MaxPendingVaultBitcoinPubkeys: u32 = 10;
	pub static NextSlot: BlockNumberFor<Test> = 100;
	pub static MiningWindowBlocks: BlockNumberFor<Test> = 100;
	pub const MinTermsModificationBlockDelay: BlockNumberFor<Test> = 25;
}
pub struct StaticMiningSlotProvider;
impl MiningSlotProvider<BlockNumberFor<Test>> for StaticMiningSlotProvider {
	fn get_next_slot_block_number() -> BlockNumberFor<Test> {
		NextSlot::get()
	}

	fn mining_window_blocks() -> BlockNumberFor<Test> {
		MiningWindowBlocks::get()
	}
}
impl pallet_vaults::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Balance = Balance;
	type BlocksPerDay = ConstU64<1440>;
	type MinimumBondAmount = MinimumBondAmount;
	type MaxPendingVaultBitcoinPubkeys = MaxPendingVaultBitcoinPubkeys;
	type MiningSlotProvider = StaticMiningSlotProvider;
	type MaxPendingTermModificationsPerBlock = ConstU32<100>;
	type MinTermsModificationBlockDelay = MinTermsModificationBlockDelay;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
