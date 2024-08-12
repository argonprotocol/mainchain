use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types, traits::Currency};
use sp_core::{crypto::AccountId32, ConstU32, H256};
use sp_runtime::{traits::IdentityLookup, BuildStorage};

use argon_primitives::{
	block_vote::VoteMinimum,
	notary::{NotaryId, NotaryProvider, NotarySignature},
	tick::{Tick, Ticker},
	BlockVotingProvider, ChainTransferLookup, TickProvider, TransferToLocalchainId,
};

use crate as pallet_notebook;

pub type Balance = u128;
pub(crate) type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Notebook: pallet_notebook
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Nonce = u64;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {

	pub static ExistentialDeposit: Balance = 10;
	pub static IsProofOfCompute: bool = false;
}

pub struct NotaryProviderImpl;
impl NotaryProvider<Block> for NotaryProviderImpl {
	fn verify_signature(_: NotaryId, _: Tick, _: &H256, _: &NotarySignature) -> bool {
		true
	}
	fn active_notaries() -> Vec<NotaryId> {
		vec![1]
	}
}

parameter_types! {
	pub static ChainTransfers: Vec<(NotaryId, AccountId32, TransferToLocalchainId, Balance)> = vec![];
	pub static ParentVotingKey: Option<H256> = None;
	pub static GrandpaVoteMinimum: Option<VoteMinimum> = None;
	pub static CurrentTick: Tick = 0;
}
pub struct ChainTransferLookupImpl;
impl ChainTransferLookup<AccountId32, Balance> for ChainTransferLookupImpl {
	fn is_valid_transfer_to_localchain(
		notary_id: NotaryId,
		transfer_to_localchain_id: TransferToLocalchainId,
		account_id: &AccountId32,
		milligons: Balance,
		_for_notebook_tick: Tick,
	) -> bool {
		ChainTransfers::get().iter().any(|(id, acc, tid, t_mill)| {
			*id == notary_id &&
				*acc == *account_id &&
				*tid == transfer_to_localchain_id &&
				*t_mill == milligons
		})
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
	let _ = Balances::make_free_balance_be(account_id, amount);
	drop(Balances::issue(amount));
}

pub struct StaticBlockVotingProvider;
impl BlockVotingProvider<Block> for StaticBlockVotingProvider {
	fn grandparent_vote_minimum() -> Option<VoteMinimum> {
		GrandpaVoteMinimum::get()
	}
}

pub struct StaticTickProvider;
impl TickProvider<Block> for StaticTickProvider {
	fn current_tick() -> Tick {
		CurrentTick::get()
	}
	fn ticker() -> Ticker {
		Ticker::new(1, 1, 2)
	}
	fn blocks_at_tick(_: Tick) -> Vec<H256> {
		todo!()
	}
}
impl pallet_notebook::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();

	type NotaryProvider = NotaryProviderImpl;
	type EventHandler = ();

	type ChainTransferLookup = ChainTransferLookupImpl;
	type BlockVotingProvider = StaticBlockVotingProvider;
	type TickProvider = StaticTickProvider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
