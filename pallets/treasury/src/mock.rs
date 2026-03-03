use crate as pallet_treasury;
use argon_primitives::{
	OperationalAccountsHook, OperationalRewardPayout, OperationalRewardsProvider,
	bitcoin::Satoshis, vault::TreasuryVaultProvider,
};
use frame_support::traits::Currency;
use pallet_prelude::{
	argon_primitives::{MiningFrameTransitionProvider, vault::VaultTreasuryFrameEarnings},
	*,
};
use std::collections::HashMap;

type Block = frame_system::mocking::MockBlock<Test>;

pub struct TestOperationalAccountsHook;

impl OperationalAccountsHook<u64, Balance> for TestOperationalAccountsHook {
	fn vault_created_weight() -> Weight {
		Weight::zero()
	}

	fn bitcoin_lock_funded_weight() -> Weight {
		Weight::zero()
	}

	fn mining_seat_won_weight() -> Weight {
		Weight::zero()
	}

	fn treasury_pool_participated_weight() -> Weight {
		Weight::zero()
	}

	fn uniswap_transfer_confirmed_weight() -> Weight {
		Weight::zero()
	}

	fn treasury_pool_participated(vault_operator_account: &u64, amount: Balance) {
		TreasuryPoolParticipated::mutate(|calls| calls.push((*vault_operator_account, amount)));
	}
}

pub struct TestOperationalRewardsProvider;

impl OperationalRewardsProvider<u64, Balance> for TestOperationalRewardsProvider {
	fn pending_rewards() -> Vec<OperationalRewardPayout<u64, Balance>> {
		PendingOperationalRewards::get()
	}

	fn mark_reward_paid(reward: &OperationalRewardPayout<u64, Balance>, amount_paid: Balance) {
		let mut paid_amount = 0;
		PendingOperationalRewards::mutate(|pending| {
			let Some(pos) = pending.iter().position(|entry| entry == reward) else {
				return;
			};
			let queued_amount = pending[pos].amount;
			paid_amount = amount_paid.min(queued_amount);
			pending.remove(pos);
		});
		if paid_amount.is_zero() {
			return;
		}
		let mut paid_reward = reward.clone();
		paid_reward.amount = paid_amount;
		PaidOperationalRewards::mutate(|paid| paid.push(paid_reward));
	}
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Treasury: pallet_treasury,
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
	pub const NextSlot: BlockNumberFor<Test> = 100;
	pub const MiningWindowBlocks: BlockNumberFor<Test> = 100;

	pub const BidPoolAccountId: u64 = 10000;

	pub const LastBidPoolDistribution: (FrameId, Tick) = (0, 0);

	pub static MaxTreasuryContributors: u32 = 10;
	pub static MinimumArgonsPerContributor: u128 = 100_000_000;
	pub static MaxVaultsPerPool: u32 = 100;
	pub const VaultPalletId: PalletId = PalletId(*b"bidPools");

	pub const PercentForTreasuryReserves: Percent = Percent::from_percent(20);
	pub static CurrentFrameId: FrameId = 1;

	pub static VaultsById: HashMap<VaultId, TestVault> = HashMap::new();

	pub static LastVaultProfits: Vec<VaultTreasuryFrameEarnings<Balance, u64>> = vec![];
	pub static TreasuryPoolParticipated: Vec<(u64, Balance)> = vec![];
	pub static PendingOperationalRewards: Vec<OperationalRewardPayout<u64, Balance>> = vec![];
	pub static PaidOperationalRewards: Vec<OperationalRewardPayout<u64, Balance>> = vec![];
}

#[derive(Clone)]
pub struct TestVault {
	pub securitized_satoshis: Satoshis,
	pub sharing_percent: Permill,
	pub account_id: u64,
	pub is_closed: bool,
}

pub fn insert_vault(vault_id: VaultId, vault: TestVault) {
	VaultsById::mutate(|x| {
		x.insert(vault_id, vault);
	});
}

pub fn set_vault_securitized_satoshis(vault_id: VaultId, satoshis: Satoshis) {
	VaultsById::mutate(|x| {
		if let Some(vault) = x.get_mut(&vault_id) {
			vault.securitized_satoshis = satoshis;
		}
	});
}

pub struct StaticTreasuryVaultProvider;
impl TreasuryVaultProvider for StaticTreasuryVaultProvider {
	type Balance = Balance;
	type AccountId = u64;

	fn get_securitized_satoshis(vault_id: VaultId) -> Satoshis {
		VaultsById::get()
			.get(&vault_id)
			.map(|a| a.securitized_satoshis)
			.unwrap_or_default()
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

	fn record_vault_frame_earnings(
		_source_account_id: &Self::AccountId,
		profit: VaultTreasuryFrameEarnings<Self::Balance, Self::AccountId>,
	) {
		let _ = Balances::burn_from(
			&Treasury::get_bid_pool_account(),
			profit.earnings_for_vault,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Force,
		);
		LastVaultProfits::mutate(|a| a.push(profit));
	}
}

pub struct StaticMiningBidPoolProvider;
impl MiningFrameTransitionProvider for StaticMiningBidPoolProvider {
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
	type RuntimeHoldReason = RuntimeHoldReason;
	type TreasuryVaultProvider = StaticTreasuryVaultProvider;
	type MaxTreasuryContributors = MaxTreasuryContributors;
	type MinimumArgonsPerContributor = MinimumArgonsPerContributor;
	type PalletId = VaultPalletId;
	type PercentForTreasuryReserves = PercentForTreasuryReserves;
	type MaxVaultsPerPool = MaxVaultsPerPool;
	type MiningFrameTransitionProvider = StaticMiningBidPoolProvider;
	type OperationalAccountsHook = TestOperationalAccountsHook;
	type OperationalRewardsProvider = TestOperationalRewardsProvider;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}

pub fn reset_treasury_pool_participated() {
	TreasuryPoolParticipated::set(vec![]);
}

pub fn take_treasury_pool_participated() -> Vec<(u64, Balance)> {
	let values = TreasuryPoolParticipated::get();
	TreasuryPoolParticipated::set(vec![]);
	values
}

pub fn set_pending_operational_rewards(rewards: Vec<OperationalRewardPayout<u64, Balance>>) {
	PendingOperationalRewards::set(rewards);
}

pub fn pending_operational_rewards() -> Vec<OperationalRewardPayout<u64, Balance>> {
	PendingOperationalRewards::get()
}

pub fn take_paid_operational_rewards() -> Vec<OperationalRewardPayout<u64, Balance>> {
	let values = PaidOperationalRewards::get();
	PaidOperationalRewards::set(vec![]);
	values
}
