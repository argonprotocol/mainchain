use crate as pallet_operational_accounts;
use argon_primitives::{
	vault::{BitcoinVaultProvider, RegistrationVaultData},
	BitcoinLocksProvider, MiningSlotProvider, OperationalRewardsPayer, TreasuryPoolProvider,
	UniswapTransferProvider,
};
use frame_support::traits::{
	fungible::{Inspect, Mutate},
	Currency,
};
use pallet_balances::AccountData;
use pallet_prelude::*;
use std::collections::{BTreeMap, BTreeSet};

use argon_primitives::{vault::VaultError, AccountId, VaultId};
use sp_runtime::FixedU128;

pub type TestAccountId = AccountId;

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		OperationalAccounts: pallet_operational_accounts,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = TestAccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = AccountData<Balance>;
	type DbWeight = RocksDbWeight;
}

parameter_types! {
	pub static ExistentialDeposit: Balance = 10;
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
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const MaxAvailableAccessCodes: u32 = 2;
	pub const MaxEncryptedServerLen: u32 = 256;
	pub const MinimumUniswapTransfer: Balance = 250;
	pub const MinimumBitcoin: Balance = 500;
	pub const MinimumBonds: Balance = 250;
	pub const OperationalMinimumUniswapTransfer: Balance = 3_000;
	pub const OperationalMinimumVaultSecuritization: Balance = 2_000;
	pub const BitcoinLockSizeForAccessCode: Balance = 5_000;
	pub const MiningSeatsForOperational: u32 = 2;
	pub const MiningSeatsPerAccessCode: u32 = 5;
	pub const OperationalCertificationsPerBonusReward: u32 = 5;
	pub const OperationalCertificationReward: Balance = 1_000;
	pub const OperationalCertificationBonusReward: Balance = 500;
	pub static IsCrosschainActivated: bool = true;
	pub static MicrogonsInByAccount:
		BTreeMap<TestAccountId, Balance> = BTreeMap::new();
	pub static MicrogonsOutByAccount:
		BTreeMap<TestAccountId, Balance> = BTreeMap::new();
	pub static FundedBitcoinAmountsByAccount:
		BTreeMap<TestAccountId, Balance> = BTreeMap::new();
	pub static RegistrationVaultDataByAccount:
		BTreeMap<TestAccountId, RegistrationVaultData<Balance>> = BTreeMap::new();
	pub static ActiveBondAmountsByVaultAndAccount:
		BTreeMap<(VaultId, TestAccountId), Balance> = BTreeMap::new();
	pub static ActiveMiningRewardsAccounts: BTreeSet<TestAccountId> = BTreeSet::new();
	pub static OperationalVaultsMarkedOperational: BTreeSet<TestAccountId> = BTreeSet::new();
	pub static ClaimableTreasuryBalance: Balance = 0;
	pub static ClaimedOperationalRewards: Vec<(TestAccountId, Balance)> = Vec::new();
}

pub struct MockVaultProvider;
impl BitcoinVaultProvider for MockVaultProvider {
	type Weights = ();
	type Balance = Balance;
	type AccountId = TestAccountId;

	fn is_owner(_vault_id: VaultId, _account_id: &Self::AccountId) -> bool {
		false
	}

	fn can_initialize_bitcoin_locks(_vault_id: VaultId, _account_id: &Self::AccountId) -> bool {
		false
	}

	fn get_vault_operator(_vault_id: VaultId) -> Option<Self::AccountId> {
		None
	}

	fn get_vault_id(account_id: &Self::AccountId) -> Option<VaultId> {
		RegistrationVaultDataByAccount::get()
			.get(account_id)
			.map(|entry| entry.vault_id)
	}

	fn get_registration_vault_data(
		account_id: &Self::AccountId,
	) -> Option<RegistrationVaultData<Self::Balance>> {
		RegistrationVaultDataByAccount::get().get(account_id).cloned()
	}

	fn get_committed_securitization(
		account_id: &Self::AccountId,
		_min_frames_remaining: FrameId,
	) -> Option<Self::Balance> {
		Self::get_registration_vault_data(account_id).map(|entry| entry.activated_securitization)
	}

	fn get_committed_argonots(account_id: &Self::AccountId) -> Option<Self::Balance> {
		Self::get_vault_id(account_id).map(|_| Default::default())
	}

	fn encumber_argonots(
		_account_id: &Self::AccountId,
		_amount: Self::Balance,
	) -> Result<(), argon_primitives::vault::VaultError> {
		Ok(())
	}

	fn release_encumbered_argonots(
		_account_id: &Self::AccountId,
		_amount: Self::Balance,
	) -> Result<(), argon_primitives::vault::VaultError> {
		Ok(())
	}

	fn burn_encumbered_argonots(
		_account_id: &Self::AccountId,
		_amount: Self::Balance,
	) -> Result<(), VaultError> {
		Ok(())
	}

	fn account_became_operational(vault_operator_account: &TestAccountId) {
		OperationalVaultsMarkedOperational::mutate(|entries| {
			entries.insert(vault_operator_account.clone());
		});
	}

	fn get_securitization_ratio(
		_vault_id: VaultId,
	) -> Result<FixedU128, argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn add_securitized_satoshis(
		_vault_id: VaultId,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_securitization_ratio: FixedU128,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn reduce_securitized_satoshis(
		_vault_id: VaultId,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_securitization_ratio: FixedU128,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn consume_recent_capacity_drop_budget(
		_vault_id: VaultId,
		_required_collateral: Self::Balance,
	) -> Result<bool, argon_primitives::vault::VaultError> {
		Ok(false)
	}

	fn lock(
		_vault_id: VaultId,
		_locker: &Self::AccountId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_extension: Option<(FixedU128, &mut argon_primitives::vault::LockExtension<Self::Balance>)>,
		_vault_covers_fee: bool,
	) -> Result<Self::Balance, argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn schedule_for_release(
		_vault_id: VaultId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_lock_extension: &argon_primitives::vault::LockExtension<Self::Balance>,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn cancel(
		_vault_id: VaultId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn burn(
		_vault_id: VaultId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
		_market_rate: Self::Balance,
		_lock_extension: &argon_primitives::vault::LockExtension<Self::Balance>,
	) -> Result<Self::Balance, argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn compensate_lost_bitcoin(
		_vault_id: VaultId,
		_beneficiary: &Self::AccountId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
		_market_rate: Self::Balance,
		_lock_extension: &argon_primitives::vault::LockExtension<Self::Balance>,
	) -> Result<Self::Balance, argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn create_utxo_script_pubkey(
		_vault_id: VaultId,
		_owner_pubkey: argon_primitives::bitcoin::CompressedBitcoinPubkey,
		_vault_claim_height: argon_primitives::bitcoin::BitcoinHeight,
		_open_claim_height: argon_primitives::bitcoin::BitcoinHeight,
		_current_height: argon_primitives::bitcoin::BitcoinHeight,
	) -> Result<
		(
			argon_primitives::bitcoin::BitcoinXPub,
			argon_primitives::bitcoin::BitcoinXPub,
			argon_primitives::bitcoin::BitcoinCosignScriptPubkey,
		),
		argon_primitives::vault::VaultError,
	> {
		unimplemented!()
	}

	fn remove_pending(
		_vault_id: VaultId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn update_pending_cosign_list(
		_vault_id: VaultId,
		_utxo_id: argon_primitives::bitcoin::UtxoId,
		_should_remove: bool,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn update_orphan_cosign_list(
		_vault_id: VaultId,
		_utxo_id: argon_primitives::bitcoin::UtxoId,
		_account_id: &Self::AccountId,
		_should_remove: bool,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}
}

pub struct MockMiningSlotProvider;
impl MiningSlotProvider<TestAccountId> for MockMiningSlotProvider {
	type Weights = ();

	fn has_active_rewards_account_seat(account_id: &TestAccountId) -> bool {
		ActiveMiningRewardsAccounts::get().contains(account_id)
	}
}

pub struct MockBitcoinLocksProvider;
impl BitcoinLocksProvider<TestAccountId, Balance> for MockBitcoinLocksProvider {
	type Weights = ();

	fn get_account_funded_bitcoin_amount(account_id: &TestAccountId) -> Balance {
		FundedBitcoinAmountsByAccount::get()
			.get(account_id)
			.copied()
			.unwrap_or_default()
	}
}

pub struct MockTreasuryPoolProvider;
impl TreasuryPoolProvider<TestAccountId> for MockTreasuryPoolProvider {
	type Weights = ();
	type Balance = Balance;

	fn has_vault_bond_participation(vault_id: VaultId, account_id: &TestAccountId) -> bool {
		Self::active_vault_bond_amount(vault_id, account_id) > 0
	}

	fn active_vault_bond_amount(vault_id: VaultId, account_id: &TestAccountId) -> Self::Balance {
		ActiveBondAmountsByVaultAndAccount::get()
			.get(&(vault_id, account_id.clone()))
			.copied()
			.unwrap_or_default()
	}

	fn active_account_vault_bond_amount(account_id: &TestAccountId) -> Self::Balance {
		let mut amount = 0;
		for ((_, owner), balance) in ActiveBondAmountsByVaultAndAccount::get() {
			if owner == *account_id {
				amount = amount.saturating_add(balance);
			}
		}
		amount
	}

	fn encumber_bond_microgons(
		_account_id: &TestAccountId,
		_microgon_amount: Self::Balance,
	) -> DispatchResult {
		Ok(())
	}

	fn release_encumbered_bond_microgons(
		_account_id: &TestAccountId,
		_microgon_amount: Self::Balance,
	) -> DispatchResult {
		Ok(())
	}

	fn burn_encumbered_bond_microgons(
		_account_id: &TestAccountId,
		_microgon_amount: Self::Balance,
	) -> sp_runtime::DispatchResult {
		Ok(())
	}
}

pub struct MockUniswapTransferProvider;
impl UniswapTransferProvider<TestAccountId> for MockUniswapTransferProvider {
	type Weights = ();
	type Balance = Balance;

	fn is_crosschain_activated() -> bool {
		IsCrosschainActivated::get()
	}

	fn account_uniswap_argon_transfers_in_amount(account_id: &TestAccountId) -> Self::Balance {
		MicrogonsInByAccount::get().get(account_id).copied().unwrap_or_default()
	}
}

pub struct MockOperationalRewardsPayer;
impl OperationalRewardsPayer<TestAccountId, Balance> for MockOperationalRewardsPayer {
	fn claim_reward(account_id: &TestAccountId, amount: Balance) -> DispatchResult {
		let available = ClaimableTreasuryBalance::get();
		if amount > available {
			return Err(DispatchError::Other("insufficient mock treasury balance"));
		}

		ClaimableTreasuryBalance::set(available.saturating_sub(amount));
		ClaimedOperationalRewards::mutate(|claimed| claimed.push((account_id.clone(), amount)));
		Ok(())
	}
}

impl pallet_operational_accounts::Config for Test {
	type Balance = Balance;
	type MaxAvailableAccessCodes = MaxAvailableAccessCodes;
	type MaxEncryptedServerLen = MaxEncryptedServerLen;
	type MinimumUniswapTransfer = MinimumUniswapTransfer;
	type MinimumBitcoin = MinimumBitcoin;
	type MinimumBonds = MinimumBonds;
	type OperationalMinimumUniswapTransfer = OperationalMinimumUniswapTransfer;
	type OperationalMinimumVaultSecuritization = OperationalMinimumVaultSecuritization;
	type BitcoinLockSizeForAccessCode = BitcoinLockSizeForAccessCode;
	type MiningSeatsForOperational = MiningSeatsForOperational;
	type MiningSeatsPerAccessCode = MiningSeatsPerAccessCode;
	type OperationalCertificationsPerBonusReward = OperationalCertificationsPerBonusReward;
	type OperationalCertificationReward = OperationalCertificationReward;
	type OperationalCertificationBonusReward = OperationalCertificationBonusReward;
	type VaultProvider = MockVaultProvider;
	type MiningSlotProvider = MockMiningSlotProvider;
	type BitcoinLocksProvider = MockBitcoinLocksProvider;
	type TreasuryPoolProvider = MockTreasuryPoolProvider;
	type UniswapTransferProvider = MockUniswapTransferProvider;
	type Currency = Balances;
	type OperationalRewardsPayer = MockOperationalRewardsPayer;
	type WeightInfo = ();
}

pub fn new_test_ext() -> TestState {
	let mut ext = new_test_with_genesis::<Test>(|t: &mut Storage| {
		pallet_operational_accounts::GenesisConfig::<Test> {
			is_operational_account_invite_only: false,
			..Default::default()
		}
		.assimilate_storage(t)
		.unwrap();
	});
	ext.execute_with(|| {
		IsCrosschainActivated::set(true);
		MicrogonsInByAccount::set(BTreeMap::new());
		MicrogonsOutByAccount::set(BTreeMap::new());
		FundedBitcoinAmountsByAccount::set(BTreeMap::new());
		RegistrationVaultDataByAccount::set(BTreeMap::new());
		ActiveBondAmountsByVaultAndAccount::set(BTreeMap::new());
		ActiveMiningRewardsAccounts::set(BTreeSet::new());
		OperationalVaultsMarkedOperational::set(BTreeSet::new());
		ClaimableTreasuryBalance::set(0);
		ClaimedOperationalRewards::set(Vec::new());
	});
	ext
}

pub fn set_crosschain_activated(activated: bool) {
	IsCrosschainActivated::set(activated);
}

pub fn record_microgons_in(account_id: &TestAccountId, amount: Balance) {
	MicrogonsInByAccount::mutate(|accounts| {
		accounts.insert(account_id.clone(), amount);
	});
}

pub fn record_microgons_out(account_id: &TestAccountId, amount: Balance) {
	MicrogonsOutByAccount::mutate(|accounts| {
		accounts.insert(account_id.clone(), amount);
	});
}

pub fn record_funded_bitcoin_amount(account_id: &TestAccountId, amount: Balance) {
	FundedBitcoinAmountsByAccount::mutate(|accounts| {
		accounts.insert(account_id.clone(), amount);
	});
}

pub fn funded_bitcoin_amount(account_id: &TestAccountId) -> Balance {
	FundedBitcoinAmountsByAccount::get()
		.get(account_id)
		.copied()
		.unwrap_or_default()
}

pub fn record_active_vault_bond_amount(
	vault_id: VaultId,
	account_id: &TestAccountId,
	amount: Balance,
) {
	ActiveBondAmountsByVaultAndAccount::mutate(|entries| {
		entries.insert((vault_id, account_id.clone()), amount);
	});
}

pub fn set_argon_balance(account_id: &TestAccountId, amount: Balance) {
	let current = Balances::balance(account_id);
	if amount > current {
		let _ = Balances::mint_into(account_id, amount.saturating_sub(current));
		return;
	}

	let _ = Balances::make_free_balance_be(account_id, amount);
}

pub fn set_registration_lookup(
	vault_account: TestAccountId,
	mining_account: TestAccountId,
	activated_securitization: Balance,
	securitization: Balance,
	active_vault_bond_amount: Balance,
	mining_seat_count: u32,
) {
	let account_bytes: &[u8] = vault_account.as_ref();
	let vault_id = u32::from_le_bytes(account_bytes[0..4].try_into().unwrap_or([0u8; 4]));
	RegistrationVaultDataByAccount::mutate(|entries| {
		entries.insert(
			vault_account.clone(),
			RegistrationVaultData { vault_id, activated_securitization, securitization },
		);
	});
	if active_vault_bond_amount > 0 {
		record_active_vault_bond_amount(vault_id, &vault_account, active_vault_bond_amount);
	}
	if mining_seat_count > 0 {
		ActiveMiningRewardsAccounts::mutate(|entries| {
			entries.insert(mining_account);
		});
	}
}

pub fn has_vault_operational_mark(vault_account: &TestAccountId) -> bool {
	OperationalVaultsMarkedOperational::get().contains(vault_account)
}
