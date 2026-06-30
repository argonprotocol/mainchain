use crate as pallet_treasury;
use argon_primitives::{
	bitcoin::Satoshis, providers::PriceProvider, vault::TreasuryVaultProvider,
	OperationalAccountsHook,
};
use frame_support::traits::{Currency, StorageMapShim};
use pallet_prelude::{
	argon_primitives::{vault::VaultTreasuryFrameEarnings, MiningFrameTransitionProvider},
	*,
};
use sp_core::{crypto::AccountId32, sr25519, Pair};
use sp_runtime::{traits::IdentifyAccount, MultiSigner};
use std::collections::HashMap;

type Block = frame_system::mocking::MockBlock<Test>;
pub type TestAccountId = AccountId32;

pub struct TestOperationalAccountsHook;

impl OperationalAccountsHook<TestAccountId, Balance> for TestOperationalAccountsHook {
	fn vault_created_weight() -> Weight {
		Weight::zero()
	}

	fn vault_bitcoin_lock_funded_weight() -> Weight {
		Weight::zero()
	}

	fn mining_seat_won_weight() -> Weight {
		Weight::zero()
	}

	fn account_vault_bond_total_updated_weight() -> Weight {
		Weight::zero()
	}

	fn account_uniswap_argon_transfers_in_updated_weight() -> Weight {
		Weight::zero()
	}
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Treasury: pallet_treasury,
		Balances: pallet_balances::<Instance1>,
		Ownership: pallet_balances::<Instance2>,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = TestAccountId;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
	type Lookup = IdentityLookup<TestAccountId>;
}

parameter_types! {
	pub const BidIncrements: u128 = 10_000; // 1 cent

	pub static ExistentialDeposit: Balance = 1;
	pub static BidPoolAccountId: TestAccountId = AccountId32::new([250; 32]);
	pub static TreasuryReservesAccountId: TestAccountId = AccountId32::new([251; 32]);
}

pub(crate) type ArgonToken = pallet_balances::Instance1;
impl pallet_balances::Config<ArgonToken> for Test {
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

pub(crate) type OwnershipToken = pallet_balances::Instance2;
impl pallet_balances::Config<OwnershipToken> for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test, OwnershipToken>,
		Self::AccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type MaxFreezes = ();
	type DoneSlashHandler = ();
}

pub(crate) fn account_pair_from_seed(seed: u64) -> sr25519::Pair {
	sr25519::Pair::from_seed(&[(seed & 0xff) as u8; 32])
}

pub(crate) fn account_id_from_seed(seed: u64) -> TestAccountId {
	MultiSigner::from(account_pair_from_seed(seed).public()).into_account()
}

pub(crate) trait IntoTestAccountId {
	fn into_test_account_id(self) -> TestAccountId;
}

impl IntoTestAccountId for u64 {
	fn into_test_account_id(self) -> TestAccountId {
		account_id_from_seed(self)
	}
}

impl IntoTestAccountId for TestAccountId {
	fn into_test_account_id(self) -> TestAccountId {
		self
	}
}

impl IntoTestAccountId for &TestAccountId {
	fn into_test_account_id(self) -> TestAccountId {
		self.clone()
	}
}

pub(crate) fn set_argons(account_id: impl IntoTestAccountId, amount: Balance) {
	let account_id = account_id.into_test_account_id();
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

pub(crate) fn set_ownership(account_id: impl IntoTestAccountId, amount: Balance) {
	let account_id = account_id.into_test_account_id();
	let _ = Ownership::make_free_balance_be(&account_id, amount);
	drop(Ownership::issue(amount));
}

parameter_types! {
	pub const NextSlot: BlockNumberFor<Test> = 100;
	pub const MiningWindowBlocks: BlockNumberFor<Test> = 100;

	pub const LastBidPoolDistribution: (FrameId, Tick) = (0, 0);

	pub static MaxTreasuryContributors: u32 = 10;
	pub static MinimumArgonsPerContributor: u128 = 100_000_000;
	pub static MaxActiveArgonotBondLots: u32 = 1_000;
	pub static MaxVaultsPerPool: u32 = 100;
	pub static MaxPendingUnlocksPerFrame: u32 = 100;
	pub static TreasuryExitDelayFrames: FrameId = 10;
	pub const VaultPalletId: PalletId = PalletId(*b"bidPools");

	pub const PercentForTreasuryReserves: Percent = Percent::from_percent(20);
	pub const PercentForArgonotBondPool: Percent = Percent::from_percent(10);
	pub static MaxArgonotBondedPercentOfCirculation: Percent = Percent::from_percent(40);
	pub static CurrentFrameId: FrameId = 1;

	pub static VaultsById: HashMap<VaultId, TestVault> = HashMap::new();

	// BTC=$100 / argon=$1 makes 1 sat = 1 microgon for clean test math
	pub static BitcoinPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(100.00));
	pub static ArgonPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(1.00));

	pub static LastVaultProfits: Vec<VaultTreasuryFrameEarnings<Balance, TestAccountId>> = vec![];
}

#[derive(Clone)]
pub struct TestVault {
	pub securitized_satoshis: Satoshis,
	pub sharing_percent: Permill,
	pub bonus_percent: Permill,
	pub account_id: TestAccountId,
	pub delegate_account_id: Option<TestAccountId>,
	pub is_closed: bool,
}

pub(crate) fn insert_vault(vault_id: VaultId, vault: TestVault) {
	VaultsById::mutate(|x| {
		x.insert(vault_id, vault);
	});
}

pub struct StaticPriceProvider;
impl PriceProvider<Balance> for StaticPriceProvider {
	type Weights = ();

	fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
		BitcoinPricePerUsd::get()
	}
	fn get_latest_argon_price_in_usd() -> Option<FixedU128> {
		ArgonPricePerUsd::get()
	}
	fn get_argonot_price_in_usd() -> Option<FixedU128> {
		ArgonPricePerUsd::get()
	}
	fn get_target_argon_price_in_usd() -> Option<FixedU128> {
		ArgonPricePerUsd::get()
	}
	fn get_argon_cpi() -> Option<argon_primitives::ArgonCPI> {
		None
	}
	fn get_redemption_r_value() -> Option<FixedU128> {
		None
	}
	fn get_circulation() -> Balance {
		0
	}
	fn get_average_cpi_for_ticks(_tick_range: (Tick, Tick)) -> argon_primitives::ArgonCPI {
		FixedI128::zero()
	}
}

pub struct StaticTreasuryVaultProvider;
impl TreasuryVaultProvider for StaticTreasuryVaultProvider {
	type Balance = Balance;
	type AccountId = TestAccountId;

	fn get_securitized_satoshis(vault_id: VaultId) -> Satoshis {
		VaultsById::get()
			.get(&vault_id)
			.map(|a| a.securitized_satoshis)
			.unwrap_or_default()
	}

	fn get_vault_profit_sharing_percent(vault_id: VaultId) -> Option<Permill> {
		VaultsById::get().get(&vault_id).map(|a| a.sharing_percent)
	}

	fn get_vault_treasury_bonus_profit_sharing(vault_id: VaultId) -> Option<Permill> {
		VaultsById::get().get(&vault_id).map(|a| a.bonus_percent)
	}

	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId> {
		VaultsById::get().get(&vault_id).map(|a| a.account_id.clone())
	}

	fn get_vault_delegate(vault_id: VaultId) -> Option<Self::AccountId> {
		VaultsById::get().get(&vault_id).and_then(|a| a.delegate_account_id.clone())
	}

	fn is_vault_open(vault_id: VaultId) -> bool {
		VaultsById::get().get(&vault_id).map(|a| !a.is_closed).unwrap_or_default()
	}

	fn record_vault_frame_earnings(
		_source_account_id: &Self::AccountId,
		profit: VaultTreasuryFrameEarnings<Self::Balance, Self::AccountId>,
	) {
		let _ = Balances::burn_from(
			&BidPoolAccountId::get(),
			profit.earnings_for_vault,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Force,
		);
		LastVaultProfits::mutate(|a| a.push(profit));
	}
}

pub struct StaticMiningFrameTransitionProvider;
impl MiningFrameTransitionProvider for StaticMiningFrameTransitionProvider {
	fn get_current_frame_id() -> FrameId {
		CurrentFrameId::get()
	}

	fn is_new_frame_started() -> Option<FrameId> {
		None
	}
}

impl pallet_treasury::Config for Test {
	type WeightInfo = ();
	type Balance = Balance;
	type Currency = Balances;
	type OwnershipCurrency = Ownership;
	type RuntimeHoldReason = RuntimeHoldReason;
	type TreasuryVaultProvider = StaticTreasuryVaultProvider;
	type PriceProvider = StaticPriceProvider;
	type MaxTreasuryContributors = MaxTreasuryContributors;
	type MinimumArgonsPerContributor = MinimumArgonsPerContributor;
	type MaxActiveArgonotBondLots = MaxActiveArgonotBondLots;
	type MaxArgonotBondedPercentOfCirculation = MaxArgonotBondedPercentOfCirculation;
	type PalletId = VaultPalletId;
	type MiningBidPoolAccount = BidPoolAccountId;
	type TreasuryReservesAccount = TreasuryReservesAccountId;
	type PercentForTreasuryReserves = PercentForTreasuryReserves;
	type PercentForArgonotBondPool = PercentForArgonotBondPool;
	type MaxVaultsPerPool = MaxVaultsPerPool;
	type MaxPendingUnlocksPerFrame = MaxPendingUnlocksPerFrame;
	type TreasuryExitDelayFrames = TreasuryExitDelayFrames;
	type MiningFrameTransitionProvider = StaticMiningFrameTransitionProvider;
	type OperationalAccountsHook = TestOperationalAccountsHook;
}

pub(crate) fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
