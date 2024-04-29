use env_logger::{Builder, Env};
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64, Currency, StorageMapShim},
};
use sp_core::{ConstU32, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use ulx_primitives::{BondId, MintCirculationProvider};

use crate as pallet_bitcoin_mint;

pub type Balance = u128;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		ArgonBalances: pallet_balances::<Instance1>,
		UlixeeBalances: pallet_balances::<Instance2>,
		BitcoinMint: pallet_bitcoin_mint,
		Bonds: pallet_bond
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
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
	type MaxConsumers = ConstU32<16>;
	type RuntimeTask = ();
}

parameter_types! {

	pub static ExistentialDeposit: Balance = 10;
}

type ArgonToken = pallet_balances::Instance1;
impl pallet_balances::Config<ArgonToken> for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
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
	let _ = ArgonBalances::make_free_balance_be(&account_id, amount);
	drop(ArgonBalances::issue(amount));
}

pub(crate) type UlixeeToken = pallet_balances::Instance2;
impl pallet_balances::Config<UlixeeToken> for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type ReserveIdentifier = [u8; 8];
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test, UlixeeToken>,
		Self::AccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_types! {
	pub const MinimumBondAmount:u128 = 1_000;
}
impl pallet_bond::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = ArgonBalances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type BondFundId = u32;
	type BondId = BondId;
	type MinimumBondAmount = MinimumBondAmount;
	type MaxConcurrentlyExpiringBonds = ConstU32<10>;
	type MaxConcurrentlyExpiringBondFunds = ConstU32<10>;
	type BlocksPerYear = ConstU64<525_600>;
	type Balance = u128;
}

parameter_types! {

	pub static UlixeeMintCirculation: Balance = 0;
}
pub struct UlixeeMintCirculationProvider;

impl MintCirculationProvider<Balance> for UlixeeMintCirculationProvider {
	fn get_mint_circulation() -> Balance {
		UlixeeMintCirculation::get()
	}
}

parameter_types! {
	pub const BitcoinBondDuration: u32 = 60 * 24 * 365; // 1 year
	pub const MinBitcoinSatoshiAmount: u64 = 100_000_000; // 1 bitcoin minimum
}
impl pallet_bitcoin_mint::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = ArgonBalances;
	type Balance = Balance;
	type UlixeeMintCirculation = UlixeeMintCirculationProvider;
	type RuntimeHoldReason = RuntimeHoldReason;
	type BondProvider = Bonds;
	type BondId = BondId;
	type BitcoinPriceProvider = BitcoinMint;
	type BondDurationBlocks = BitcoinBondDuration;
	type MinimumSatoshiAmount = MinBitcoinSatoshiAmount;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
