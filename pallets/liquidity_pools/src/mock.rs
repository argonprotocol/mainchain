use crate as pallet_liquidity_pools;
use argon_primitives::vault::LiquidityPoolVaultProvider;
use frame_support::traits::Currency;
use pallet_prelude::*;
use std::collections::HashMap;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		LiquidityPools: pallet_liquidity_pools,
		Balances: pallet_balances,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
}

parameter_types! {
	pub const BidIncrements: u128 = 10_000; // 1 cent

	pub static ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
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

pub fn set_argons(account_id: u64, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

parameter_types! {
	pub static NextSlot: BlockNumberFor<Test> = 100;
	pub static MiningWindowBlocks: BlockNumberFor<Test> = 100;

	pub const BidPoolAccountId: u64 = 10000;

	pub static LastBidPoolDistribution: (CohortId, Tick) = (0, 0);

	pub static MaxLiquidityPoolContributors: u32 = 10;
	pub static MinimumArgonsPerContributor: u128 = 100_000_000;
	pub static MaxBidPoolVaultParticipants: u32 = 100;
	pub static VaultPalletId: PalletId = PalletId(*b"bidPools");

	pub static BurnFromBidPoolAmount: Percent = Percent::from_percent(20);
	pub static NextCohortId: CohortId = 1;

	pub static VaultsById: HashMap<VaultId, TestVault> = HashMap::new();
}

#[derive(Clone)]
pub struct TestVault {
	pub activated: Balance,
	pub sharing_percent: Permill,
	pub account_id: u64,
	pub is_closed: bool,
}

pub fn insert_vault(vault_id: VaultId, vault: TestVault) {
	VaultsById::mutate(|x| {
		x.insert(vault_id, vault);
	});
}

pub struct StaticLiquidityPoolVaultProvider;
impl LiquidityPoolVaultProvider for StaticLiquidityPoolVaultProvider {
	type Balance = Balance;
	type AccountId = u64;

	fn get_activated_securitization(vault_id: VaultId) -> Self::Balance {
		VaultsById::get().get(&vault_id).map(|a| a.activated).unwrap_or_default()
	}

	fn get_vault_profit_sharing_percent(vault_id: VaultId) -> Option<Permill> {
		VaultsById::get().get(&vault_id).map(|a| a.sharing_percent)
	}

	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId> {
		VaultsById::get().get(&vault_id).map(|a| a.account_id)
	}

	fn is_vault_open(vault_id: VaultId) -> bool {
		VaultsById::get().get(&vault_id).map(|a| !a.is_closed).unwrap_or_default()
	}
}

impl pallet_liquidity_pools::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Balance = Balance;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type LiquidityPoolVaultProvider = StaticLiquidityPoolVaultProvider;
	type MaxLiquidityPoolContributors = MaxLiquidityPoolContributors;
	type MinimumArgonsPerContributor = MinimumArgonsPerContributor;
	type PalletId = VaultPalletId;
	type BidPoolBurnPercent = BurnFromBidPoolAmount;
	type MaxBidPoolVaultParticipants = MaxBidPoolVaultParticipants;
	type NextCohortId = NextCohortId;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
