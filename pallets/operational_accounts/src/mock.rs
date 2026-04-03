use crate as pallet_operational_accounts;
use argon_primitives::{
	MiningFrameTransitionProvider, MiningSlotProvider, TreasuryPoolProvider,
	vault::{BitcoinVaultProvider, RegistrationVaultData},
};
use pallet_inbound_transfer_log as inbound_transfer_log;
use pallet_prelude::*;
use std::collections::{BTreeMap, BTreeSet};

use argon_primitives::{AccountId, VaultId};
use sp_runtime::FixedU128;

pub type TestAccountId = AccountId;

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		OperationalAccounts: pallet_operational_accounts,
		InboundTransferLog: inbound_transfer_log,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = TestAccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = ();
	type DbWeight = RocksDbWeight;
}

parameter_types! {
	pub static CurrentFrameId: FrameId = 1;
	pub const AccessCodeExpirationFrames: FrameId = 2;
	pub const MaxAccessCodesExpiringPerFrame: u32 = 16;
	pub const MaxUnactivatedAccessCodes: u32 = 2;
	pub const MaxIssuableAccessCodes: u32 = 2;
	pub const MaxEncryptedServerLen: u32 = 256;
	pub const MaxOperationalRewardsQueued: u32 = 100;
	pub const OperationalMinimumVaultSecuritization: Balance = 2_000;
	pub const BitcoinLockSizeForAccessCode: Balance = 5_000;
	pub const MiningSeatsForOperational: u32 = 2;
	pub const MiningSeatsPerAccessCode: u32 = 5;
	pub const ReferralBonusEveryXOperationalSponsees: u32 = 5;
	pub const OperationalReferralReward: Balance = 1_000;
	pub const OperationalReferralBonusReward: Balance = 500;

	pub const InboundTransfersRetentionBlocks: BlockNumberFor<Test> = 10;
	pub const MaxTransfersToRetainPerBlock: u32 = 10;
	pub const MinimumTransferMicrogonsToRecord: Balance = 1;
	pub const MaxInboundTransferBytes: u32 = 10 * 1024;
	pub const OwnershipAssetId: u32 = 2;
	pub static RegistrationVaultDataByAccount:
		BTreeMap<TestAccountId, RegistrationVaultData<Balance>> = BTreeMap::new();
	pub static TreasuryPoolParticipantsByVaultId:
		BTreeMap<VaultId, BTreeSet<TestAccountId>> = BTreeMap::new();
	pub static ActiveMiningRewardsAccounts: BTreeSet<TestAccountId> = BTreeSet::new();
	pub static OperationalVaultsMarkedOperational: BTreeSet<TestAccountId> = BTreeSet::new();
}

pub struct StaticFrameProvider;
impl MiningFrameTransitionProvider for StaticFrameProvider {
	fn get_current_frame_id() -> FrameId {
		CurrentFrameId::get()
	}

	fn is_new_frame_started() -> Option<FrameId> {
		None
	}
}

pub struct MockVaultProvider;
impl BitcoinVaultProvider for MockVaultProvider {
	type Weights = ();
	type Balance = Balance;
	type AccountId = TestAccountId;

	fn is_owner(_vault_id: VaultId, _account_id: &Self::AccountId) -> bool {
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

	fn lock(
		_vault_id: VaultId,
		_locker: &Self::AccountId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_extension: Option<(FixedU128, &mut argon_primitives::vault::LockExtension<Self::Balance>)>,
		_has_fee_coupon: bool,
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

pub struct MockTreasuryPoolProvider;
impl TreasuryPoolProvider<TestAccountId> for MockTreasuryPoolProvider {
	type Weights = ();

	fn has_pool_participation(vault_id: VaultId, account_id: &TestAccountId) -> bool {
		TreasuryPoolParticipantsByVaultId::get()
			.get(&vault_id)
			.is_some_and(|accounts| accounts.contains(account_id))
	}
}

impl pallet_operational_accounts::Config for Test {
	type Balance = Balance;
	type FrameProvider = StaticFrameProvider;
	type AccessCodeExpirationFrames = AccessCodeExpirationFrames;
	type MaxAccessCodesExpiringPerFrame = MaxAccessCodesExpiringPerFrame;
	type MaxUnactivatedAccessCodes = MaxUnactivatedAccessCodes;
	type MaxIssuableAccessCodes = MaxIssuableAccessCodes;
	type MaxEncryptedServerLen = MaxEncryptedServerLen;
	type MaxOperationalRewardsQueued = MaxOperationalRewardsQueued;
	type OperationalMinimumVaultSecuritization = OperationalMinimumVaultSecuritization;
	type BitcoinLockSizeForAccessCode = BitcoinLockSizeForAccessCode;
	type MiningSeatsForOperational = MiningSeatsForOperational;
	type MiningSeatsPerAccessCode = MiningSeatsPerAccessCode;
	type ReferralBonusEveryXOperationalSponsees = ReferralBonusEveryXOperationalSponsees;
	type OperationalReferralReward = OperationalReferralReward;
	type OperationalReferralBonusReward = OperationalReferralBonusReward;
	type VaultProvider = MockVaultProvider;
	type MiningSlotProvider = MockMiningSlotProvider;
	type TreasuryPoolProvider = MockTreasuryPoolProvider;
	type RecentArgonTransferLookup = InboundTransferLog;
	type OperationalRewardsPayer = ();
	type WeightInfo = ();
}

impl inbound_transfer_log::Config for Test {
	type InboundTransfersRetentionBlocks = InboundTransfersRetentionBlocks;
	type MaxTransfersToRetainPerBlock = MaxTransfersToRetainPerBlock;
	type MinimumTransferMicrogonsToRecord = MinimumTransferMicrogonsToRecord;
	type MaxInboundTransferBytes = MaxInboundTransferBytes;
	type OwnershipAssetId = OwnershipAssetId;
	type WeightInfo = ();
	type OperationalAccountsHook = OperationalAccounts;
}

pub fn new_test_ext() -> TestState {
	let mut ext = new_test_with_genesis::<Test>(|_t: &mut Storage| {});
	ext.execute_with(|| {
		RegistrationVaultDataByAccount::set(BTreeMap::new());
		TreasuryPoolParticipantsByVaultId::set(BTreeMap::new());
		ActiveMiningRewardsAccounts::set(BTreeSet::new());
		OperationalVaultsMarkedOperational::set(BTreeSet::new());
		pallet_operational_accounts::Rewards::<Test>::put(crate::RewardsConfig {
			operational_referral_reward: OperationalReferralReward::get(),
			referral_bonus_reward: OperationalReferralBonusReward::get(),
		});
	});
	ext
}

pub fn set_registration_lookup(
	vault_account: TestAccountId,
	mining_funding_account: TestAccountId,
	activated_securitization: Balance,
	securitization: Balance,
	has_treasury_pool_participation: bool,
	observed_mining_seat_total: u32,
) {
	let account_bytes: &[u8] = vault_account.as_ref();
	let vault_id = u32::from_le_bytes(account_bytes[0..4].try_into().unwrap_or([0u8; 4]));
	RegistrationVaultDataByAccount::mutate(|entries| {
		entries.insert(
			vault_account.clone(),
			RegistrationVaultData { vault_id, activated_securitization, securitization },
		);
	});
	if has_treasury_pool_participation {
		TreasuryPoolParticipantsByVaultId::mutate(|entries| {
			entries.entry(vault_id).or_default().insert(vault_account.clone());
		});
	}
	if observed_mining_seat_total > 0 {
		ActiveMiningRewardsAccounts::mutate(|entries| {
			entries.insert(mining_funding_account);
		});
	}
}

pub fn ensure_registration_lookup(
	vault_account: TestAccountId,
	mining_funding_account: TestAccountId,
) {
	if RegistrationVaultDataByAccount::get().contains_key(&vault_account) {
		return;
	}

	set_registration_lookup(
		vault_account,
		mining_funding_account,
		0,
		OperationalMinimumVaultSecuritization::get(),
		false,
		0,
	);
}

pub fn has_vault_operational_mark(vault_account: &TestAccountId) -> bool {
	OperationalVaultsMarkedOperational::get().contains(vault_account)
}
