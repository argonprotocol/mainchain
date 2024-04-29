use env_logger::{Builder, Env};
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64, Currency},
};
use sp_core::{ConstU32, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use ulx_primitives::MintCirculationProvider;

use crate as pallet_ulixee_mint;

pub type Balance = u128;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		UlixeeMint: pallet_ulixee_mint
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

parameter_types! {

	pub static ExistentialDeposit: Balance = 10;
}

type UlixeeToken = pallet_balances::Instance1;
impl pallet_balances::Config<UlixeeToken> for Test {
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type ReserveIdentifier = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = UlixeeMint;
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

	pub static BitcoinMintCirculation: Balance = 0;
}
pub struct BitcoinMintCirculationProvider;

impl MintCirculationProvider<Balance> for BitcoinMintCirculationProvider {
	fn get_mint_circulation() -> Balance {
		BitcoinMintCirculation::get()
	}
}

impl pallet_ulixee_mint::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Balance = Balance;
	type UlixeeTokenStorage = pallet_balances::Account<Test, UlixeeToken>;
	type BitcoinMintCirculation = BitcoinMintCirculationProvider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
