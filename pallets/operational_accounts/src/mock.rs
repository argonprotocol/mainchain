use crate as pallet_operational_accounts;
use pallet_inbound_transfer_log as inbound_transfer_log;
use pallet_prelude::*;

use argon_primitives::{AccountId, MiningFrameTransitionProvider};

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
	pub const MaxOperationalRewardsQueued: u32 = 100;
	pub const MinBitcoinLockSizeForOperational: Balance = 2_000;
	pub const BitcoinLockSizeForAccessCode: Balance = 5_000;
	pub const MiningSeatsForOperational: u32 = 2;
	pub const MiningSeatsPerAccessCode: u32 = 5;
	pub const ReferralBonusEveryXOperationalSponsees: u32 = 5;
	pub const OperationalReferralReward: Balance = 1_000;
	pub const OperationalReferralBonusReward: Balance = 500;
	pub const MaxLegacyVaultRegistrations: u32 = 200;

	pub const InboundTransfersRetentionBlocks: BlockNumberFor<Test> = 10;
	pub const MaxTransfersToRetainPerBlock: u32 = 10;
	pub const MinimumTransferMicrogonsToRecord: Balance = 1;
	pub const MaxInboundTransferBytes: u32 = 10 * 1024;
	pub const OwnershipAssetId: u32 = 2;
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

impl pallet_operational_accounts::Config for Test {
	type Balance = Balance;
	type FrameProvider = StaticFrameProvider;
	type AccessCodeExpirationFrames = AccessCodeExpirationFrames;
	type MaxAccessCodesExpiringPerFrame = MaxAccessCodesExpiringPerFrame;
	type MaxUnactivatedAccessCodes = MaxUnactivatedAccessCodes;
	type MaxIssuableAccessCodes = MaxIssuableAccessCodes;
	type MaxOperationalRewardsQueued = MaxOperationalRewardsQueued;
	type MinBitcoinLockSizeForOperational = MinBitcoinLockSizeForOperational;
	type BitcoinLockSizeForAccessCode = BitcoinLockSizeForAccessCode;
	type MiningSeatsForOperational = MiningSeatsForOperational;
	type MiningSeatsPerAccessCode = MiningSeatsPerAccessCode;
	type ReferralBonusEveryXOperationalSponsees = ReferralBonusEveryXOperationalSponsees;
	type OperationalReferralReward = OperationalReferralReward;
	type OperationalReferralBonusReward = OperationalReferralBonusReward;
	type MaxLegacyVaultRegistrations = MaxLegacyVaultRegistrations;
	type LegacyVaultProvider = ();
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
		pallet_operational_accounts::Rewards::<Test>::put(crate::RewardsConfig {
			operational_referral_reward: OperationalReferralReward::get(),
			referral_bonus_reward: OperationalReferralBonusReward::get(),
		});
	});
	ext
}
