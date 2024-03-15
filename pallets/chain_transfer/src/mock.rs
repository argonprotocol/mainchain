use env_logger::{Builder, Env};
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64, Currency},
	PalletId,
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_core::{crypto::AccountId32, ConstU32, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup, NumberFor},
	BuildStorage,
};
use sp_std::collections::btree_map::BTreeMap;

use ulx_primitives::{
	notary::{NotaryId, NotaryProvider, NotarySignature},
	tick::Tick,
	BlockSealAuthorityId, NotebookNumber, NotebookProvider, NotebookSecret,
};

use crate as pallet_chain_transfer;

pub type Balance = u128;
pub(crate) type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = BlockNumberFor<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		ChainTransfer: pallet_chain_transfer
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
	type AccountId = AccountId32;
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
	pub const MinimumBondAmount:u128 = 1_000;
	pub const BlocksPerYear:u32 = 1440*365;

	pub const LocalchainAccountId :u32 = 1;
	pub static MaxNotebookBlocksToRemember :u32 = 1;
	pub const MaxNotebookTransfers :u32 = 1;
	pub static MaxPendingTransfersOutPerBlock :u32 = 1;
	pub static TransferExpirationBlocks :u32 = 2;

	pub const LocalchainPalletId: PalletId = PalletId(*b"loclchai");

	pub static BlockSealers: BTreeMap<BlockNumber, Vec<BlockSealAuthorityId>> = BTreeMap::new();

	pub static LockedNotaries: BTreeMap<NotaryId, Tick> = BTreeMap::new();

	pub static IsProofOfCompute: bool = false;
}

pub struct NotaryProviderImpl;
impl NotaryProvider<Block> for NotaryProviderImpl {
	fn verify_signature(_: NotaryId, _: NumberFor<Block>, _: &H256, _: &NotarySignature) -> bool {
		true
	}
	fn active_notaries() -> Vec<NotaryId> {
		vec![1]
	}
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
}

pub fn set_argons(account_id: &AccountId32, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

pub struct StaticNotebookProvider;
impl NotebookProvider for StaticNotebookProvider {
	fn get_eligible_tick_votes_root(_: NotaryId, _tick: Tick) -> Option<(H256, NotebookNumber)> {
		None
	}
	fn notebooks_at_tick(_: Tick) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)> {
		todo!()
	}
	fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)> {
		todo!()
	}
	fn is_notary_locked_at_tick(notary_id: NotaryId, tick: Tick) -> bool {
		if let Some(lock_tick) = LockedNotaries::get().get(&notary_id) {
			return *lock_tick <= tick
		}
		false
	}
}
impl pallet_chain_transfer::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type Balance = Balance;
	type MaxPendingTransfersOutPerBlock = MaxPendingTransfersOutPerBlock;
	type NotaryProvider = NotaryProviderImpl;
	type PalletId = LocalchainPalletId;
	type TransferExpirationBlocks = TransferExpirationBlocks;
	type NotebookProvider = StaticNotebookProvider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
