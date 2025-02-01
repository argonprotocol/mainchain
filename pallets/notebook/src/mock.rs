use crate as pallet_notebook;
use argon_notary_audit::VerifyError;
use argon_primitives::{
	block_vote::VoteMinimum,
	notary::{NotaryId, NotaryProvider, NotarySignature},
	tick::{Tick, TickDigest, Ticker},
	AccountId, BlockSealSpecProvider, BlockVoteDigest, ChainTransferLookup, ComputeDifficulty,
	Digestset, NotebookDigest, NotebookEventHandler, NotebookHeader, TickProvider,
	TransferToLocalchainId, VotingSchedule,
};
use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types, traits::Currency};
use sp_core::{crypto::AccountId32, ConstU32, Get, H256};
use sp_keyring::{ed25519::Keyring, Ed25519Keyring};
use sp_runtime::{
	traits::{Block as BlockT, IdentityLookup},
	BuildStorage, DispatchError,
};

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
	pub static NotaryOperator: AccountId32 = Ed25519Keyring::Bob.to_account_id();
}

pub struct NotaryProviderImpl;
impl NotaryProvider<Block, AccountId32> for NotaryProviderImpl {
	fn verify_signature(_: NotaryId, _: Tick, _: &H256, _: &NotarySignature) -> bool {
		true
	}
	fn active_notaries() -> Vec<NotaryId> {
		vec![1]
	}
	fn notary_operator_account_id(_notary_id: NotaryId) -> Option<AccountId> {
		Some(NotaryOperator::get())
	}
}

parameter_types! {
	pub static ChainTransfers: Vec<(NotaryId, AccountId32, TransferToLocalchainId, Balance)> = vec![];
	pub static ParentVotingKey: Option<H256> = None;
	pub static GrandpaVoteMinimum: Option<VoteMinimum> = None;
	pub static CurrentTick: Tick = 0;
	pub static NotebookEvents: Vec<NotebookHeader> = vec![];

	pub static Digests: Digestset<VerifyError, AccountId32> = Digestset {
		block_vote: BlockVoteDigest { voting_power: 500, votes_count: 1 },
		author: Keyring::Alice.to_account_id(),
		voting_key: None,
		tick: TickDigest(2),
		fork_power: None,
		notebooks: NotebookDigest {
			notebooks: vec![],
		},
	};
}

pub struct DigestGetter;
impl Get<Result<Digestset<VerifyError, AccountId32>, DispatchError>> for DigestGetter {
	fn get() -> Result<Digestset<VerifyError, AccountId32>, DispatchError> {
		Ok(Digests::get())
	}
}

pub struct ChainTransferLookupImpl;
impl ChainTransferLookup<AccountId32, Balance> for ChainTransferLookupImpl {
	fn is_valid_transfer_to_localchain(
		notary_id: NotaryId,
		transfer_to_localchain_id: TransferToLocalchainId,
		account_id: &AccountId32,
		microgons: Balance,
		_for_notebook_tick: Tick,
	) -> bool {
		ChainTransfers::get().iter().any(|(id, acc, tid, t_mill)| {
			*id == notary_id &&
				*acc == *account_id &&
				*tid == transfer_to_localchain_id &&
				*t_mill == microgons
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

pub struct StaticBlockSealSpecProvider;
impl BlockSealSpecProvider<Block> for StaticBlockSealSpecProvider {
	fn grandparent_vote_minimum() -> Option<VoteMinimum> {
		GrandpaVoteMinimum::get()
	}
	fn compute_difficulty() -> ComputeDifficulty {
		todo!("(")
	}
	fn compute_key_block_hash() -> Option<<Block as BlockT>::Hash> {
		todo!()
	}
}

pub struct StaticTickProvider;
impl TickProvider<Block> for StaticTickProvider {
	fn previous_tick() -> Tick {
		todo!()
	}
	fn current_tick() -> Tick {
		CurrentTick::get()
	}
	fn elapsed_ticks() -> Tick {
		CurrentTick::get()
	}
	fn ticker() -> Ticker {
		Ticker::new(1, 2)
	}
	fn blocks_at_tick(_: Tick) -> Vec<H256> {
		todo!()
	}
	fn voting_schedule() -> VotingSchedule {
		VotingSchedule::from_runtime_current_tick(CurrentTick::get())
	}
}

pub struct OnNotebook;
impl NotebookEventHandler for OnNotebook {
	fn notebook_submitted(header: &NotebookHeader) {
		NotebookEvents::mutate(|events| events.push(header.clone()));
	}
}

impl pallet_notebook::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();

	type NotaryProvider = NotaryProviderImpl;
	type EventHandler = OnNotebook;

	type ChainTransferLookup = ChainTransferLookupImpl;
	type BlockSealSpecProvider = StaticBlockSealSpecProvider;
	type TickProvider = StaticTickProvider;
	type Digests = DigestGetter;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
