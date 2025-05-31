use alloc::collections::btree_map::BTreeMap;

use Sr25519Keyring::Alice;
use pallet_prelude::*;

use crate as pallet_chain_transfer;
use argon_primitives::{NotebookProvider, NotebookSecret};
use frame_support::traits::Currency;
use sp_runtime::AccountId32;
pub(crate) type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = BlockNumberFor<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		ChainTransfer: pallet_chain_transfer,
		Timestamp: pallet_timestamp,

	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {

	pub static ExistentialDeposit: Balance = 10;
	pub const BlocksPerYear:u32 = 1440*365;

	pub const LocalchainAccountId :u32 = 1;
	pub static MaxNotebookBlocksToRemember :u32 = 1;
	pub const MaxNotebookTransfers :u32 = 1;
	pub static MaxPendingTransfersOutPerBlock :u32 = 1;
	pub static TransferExpirationTicks :u64 = 2;
	pub static NotebookTick: Tick = 1;

	pub const LocalchainPalletId: PalletId = PalletId(*b"loclchai");

	pub static BlockSealers: BTreeMap<BlockNumber, Vec<BlockSealAuthorityId>> = BTreeMap::new();

	pub static LockedNotaries: BTreeMap<NotaryId, Tick> = BTreeMap::new();

	pub static IsProofOfCompute: bool = false;
}

type ArgonToken = pallet_balances::Instance1;
impl pallet_balances::Config<ArgonToken> for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type ReserveIdentifier = ();
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type MaxFreezes = ConstU32<1>;
	type DoneSlashHandler = ();
}

pub fn set_argons(account_id: &AccountId32, amount: Balance) {
	let _ = Balances::make_free_balance_be(account_id, amount);
	drop(Balances::issue(amount));
}

pub struct StaticNotebookProvider;
impl NotebookProvider for StaticNotebookProvider {
	fn get_eligible_tick_votes_root(_: NotaryId, _tick: Tick) -> Option<(H256, NotebookNumber)> {
		None
	}
	fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)> {
		todo!()
	}
	fn notebooks_at_tick(_: Tick) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)> {
		todo!()
	}
	fn is_notary_locked_at_tick(notary_id: NotaryId, tick: Tick) -> bool {
		if let Some(lock_tick) = LockedNotaries::get().get(&notary_id) {
			return *lock_tick <= tick;
		}
		false
	}
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

impl pallet_chain_transfer::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Argon = Balances;
	type Balance = Balance;
	type NotebookProvider = StaticNotebookProvider;
	type NotebookTick = NotebookTick;
	type EventHandler = ();
	type PalletId = LocalchainPalletId;
	type TransferExpirationTicks = TransferExpirationTicks;
	type MaxPendingTransfersOutPerBlock = MaxPendingTransfersOutPerBlock;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|t: &mut Storage| {
		pallet_chain_transfer::GenesisConfig::<Test> {
			hyperbridge_token_admin: Some(Alice.to_account_id()),
			..Default::default()
		}
		.assimilate_storage(t)
		.unwrap();
	})
}
