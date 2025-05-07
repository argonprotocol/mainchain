use crate as pallet_restricted_account;
use frame_executive::Executive;
use frame_support::traits::{Currency, InstanceFilter};
use frame_system::ChainContext;
use pallet_prelude::*;
use polkadot_sdk::{
	frame_support::weights::IdentityFee, pallet_transaction_payment::FungibleAdapter,
	sp_core::ConstU8,
};
use sp_runtime::{
	generic,
	testing::UintAuthorityId,
	traits::{BlakeTwo256, IdentityLookup},
};

type AccountId = u64;

pub type BlockNumber = u32;

/// Our `TransactionExtension` fit for general transactions.
pub type TxExtension =
	(frame_system::CheckMortality<Test>, pallet_restricted_account::CheckRestrictedAccess<Test>);

pub type UncheckedXt =
	generic::UncheckedExtrinsic<AccountId, RuntimeCall, UintAuthorityId, TxExtension>;

pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = generic::Block<Header, UncheckedXt>;

/// Alias for the executive to apply extrinsics in tests.
pub type TestExecutive = Executive<Test, Block, ChainContext<Test>, Test, AllPalletsWithSystem>;

// Configure a mock Runtime to Runtime the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		RestrictedAccount: pallet_restricted_account,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountId = AccountId;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
}
parameter_types! {
	pub static ExistentialDeposit: Balance = 10;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type MaxReserves = ();
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

#[derive_impl(pallet_transaction_payment::config_preludes::TestDefaultConfig)]
impl pallet_transaction_payment::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
}

pub(crate) fn set_argons(account_id: &u64, amount: Balance) {
	let _ = Balances::make_free_balance_be(account_id, amount);
	drop(Balances::issue(amount));
}
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	TypeInfo,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	Default,
)]
pub enum AccessType {
	#[default]
	Any,
	NonTransfer,
	MiningBid,
}

impl InstanceFilter<RuntimeCall> for AccessType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			AccessType::Any => true,
			AccessType::NonTransfer => !matches!(c, RuntimeCall::Balances(..)),
			_ => false,
		}
	}
	fn is_superset(&self, _o: &Self) -> bool {
		false
	}
}
impl pallet_restricted_account::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type AccessTypes = AccessType;
	type MaxAccessTypes = ConstU32<10>;
}
pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_| {})
}
