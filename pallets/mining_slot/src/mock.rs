use env_logger::{Builder, Env};
use frame_support::{
	derive_impl, parameter_types,
	traits::{Currency, StorageMapShim},
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_core::ConstU32;
use sp_runtime::BuildStorage;
use ulx_primitives::{
	bond::{BondError, BondProvider},
	VaultId,
};

use crate as pallet_mining_slot;
use crate::Registration;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		MiningSlots: pallet_mining_slot,
		ArgonBalances: pallet_balances::<Instance1>,
		UlixeeBalances: pallet_balances::<Instance2>,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {
	pub static BlocksBetweenSlots: u32 = 1;
	pub static MaxCohortSize: u32 = 5;
	pub static MaxMiners: u32 = 10;
	pub static BlocksBufferToStopAcceptingBids: u32 = 0;
	pub const OwnershipPercentDamper: u32 = 80;

	pub static ExistentialDeposit: Balance = 1;
	pub const MinimumBondAmount:u128 = 1_000;
}

pub type BondId = u64;
pub type Balance = u128;

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

pub fn set_ownership(account_id: u64, amount: Balance) {
	let _ = UlixeeBalances::make_free_balance_be(&account_id, amount);
	drop(UlixeeBalances::issue(amount));
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
	pub static Bonds: Vec<(BondId, VaultId, u64, Balance)> = vec![];
	pub static NextBondId: u64 = 1;
}

pub struct StaticBondProvider;
impl BondProvider for StaticBondProvider {
	type Balance = Balance;
	type AccountId = u64;
	type BlockNumber = BlockNumberFor<Test>;

	fn bond_mining_slot(
		vault_id: VaultId,
		account_id: Self::AccountId,
		amount: Self::Balance,
		_bond_until_block: Self::BlockNumber,
	) -> Result<ulx_primitives::BondId, BondError> {
		let bond_id = NextBondId::get();
		NextBondId::set(bond_id + 1);
		Bonds::mutate(|a| a.push((bond_id, vault_id, account_id, amount)));
		Ok(bond_id)
	}

	fn cancel_bond(bond_id: ulx_primitives::BondId) -> Result<(), BondError> {
		Bonds::mutate(|a| {
			if let Some(pos) = a.iter().position(|(id, _, _, _)| *id == bond_id) {
				a.remove(pos);
			}
		});
		Ok(())
	}
}

impl pallet_mining_slot::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type SessionIndicesToKeepInHistory = ConstU32<10>;
	type BlocksBetweenSlots = BlocksBetweenSlots;
	type MaxCohortSize = MaxCohortSize;
	type MaxMiners = MaxMiners;
	type OwnershipCurrency = UlixeeBalances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type OwnershipPercentDamper = OwnershipPercentDamper;
	type BlocksBufferToStopAcceptingBids = BlocksBufferToStopAcceptingBids;
	type Balance = Balance;
	type BondProvider = StaticBondProvider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(miner_zero: Option<Registration<Test>>) -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();

	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_mining_slot::GenesisConfig::<Test> { miner_zero, _phantom: Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();

	sp_io::TestExternalities::new(t)
}
