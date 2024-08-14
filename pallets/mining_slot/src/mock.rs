use crate as pallet_mining_slot;
use crate::Registration;
use argon_primitives::{
	block_seal::{MiningSlotConfig, RewardSharing},
	bond::{BondError, BondProvider},
	VaultId,
};
use env_logger::{Builder, Env};
use frame_support::{
	derive_impl, parameter_types,
	traits::{Currency, StorageMapShim},
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_core::ConstU32;
use sp_runtime::{BuildStorage, FixedU128};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		MiningSlots: pallet_mining_slot,
		ArgonBalances: pallet_balances::<Instance1>,
		ShareBalances: pallet_balances::<Instance2>,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {
	pub static BlocksBetweenSlots: u64 = 1;
	pub static MaxCohortSize: u32 = 5;
	pub static MaxMiners: u32 = 10;
	pub static BlocksBeforeBidEndForVrfClose: u64 = 0;
	pub static SlotBiddingStartBlock: u64 = 3;
	pub static TargetBidsPerSlot: u32 = 5;
	pub const OwnershipPercentAdjustmentDamper: FixedU128 = FixedU128::from_rational(20, 100);

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
	let _ = ShareBalances::make_free_balance_be(&account_id, amount);
	drop(ShareBalances::issue(amount));
}

pub fn set_argons(account_id: u64, amount: Balance) {
	let _ = ArgonBalances::make_free_balance_be(&account_id, amount);
	drop(ArgonBalances::issue(amount));
}

pub(crate) type SharesToken = pallet_balances::Instance2;
impl pallet_balances::Config<SharesToken> for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type ReserveIdentifier = [u8; 8];
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test, SharesToken>,
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
	pub static VaultSharing: Option<RewardSharing<u64>> = None;
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
	) -> Result<(argon_primitives::BondId, Option<RewardSharing<u64>>), BondError> {
		let bond_id = NextBondId::get();
		NextBondId::set(bond_id + 1);
		Bonds::mutate(|a| a.push((bond_id, vault_id, account_id, amount)));
		Ok((bond_id, VaultSharing::get()))
	}

	fn cancel_bond(bond_id: argon_primitives::BondId) -> Result<(), BondError> {
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
	type SessionWindowsToKeepInHistory = ConstU32<10>;
	type MaxCohortSize = MaxCohortSize;
	type TargetBidsPerSlot = TargetBidsPerSlot;
	type MaxMiners = MaxMiners;
	type OwnershipCurrency = ShareBalances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type OwnershipPercentAdjustmentDamper = OwnershipPercentAdjustmentDamper;
	type Balance = Balance;
	type BondProvider = StaticBondProvider;
	type SessionRotationsPerMiningWindow = ConstU32<2>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(miner_zero: Option<Registration<Test>>) -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();

	let mining_config = MiningSlotConfig::<BlockNumberFor<Test>> {
		slot_bidding_start_block: SlotBiddingStartBlock::get(),
		blocks_between_slots: BlocksBetweenSlots::get(),
		blocks_before_bid_end_for_vrf_close: BlocksBeforeBidEndForVrfClose::get(),
	};

	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_mining_slot::GenesisConfig::<Test> {
		miner_zero,
		mining_config,
		_phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::new(t)
}
