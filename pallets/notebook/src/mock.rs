use env_logger::{Builder, Env};
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64, Currency},
};
use sp_core::{crypto::AccountId32, ConstU32, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup, NumberFor},
	BuildStorage,
};
use sp_std::collections::btree_map::BTreeMap;

use ulx_primitives::{
	block_seal::{Host, MiningAuthority},
	notary::{NotaryId, NotaryProvider, NotarySignature},
	AuthorityProvider, BlockSealAuthorityId, ChainTransferLookup,
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
}

parameter_types! {

	pub static ExistentialDeposit: Balance = 10;
	pub static IsProofOfCompute: bool = false;
}

parameter_types! {
	pub static AuthorityList: Vec<(AccountId32, BlockSealAuthorityId)> = vec![];
	pub static XorClosest: Option<MiningAuthority<BlockSealAuthorityId>> = None;
}
pub struct StaticAuthorityProvider;
impl AuthorityProvider<BlockSealAuthorityId, Block, AccountId32> for StaticAuthorityProvider {
	fn miner_zero() -> Option<(u16, BlockSealAuthorityId, Vec<Host>, AccountId32)> {
		None
	}
	fn is_active(authority_id: &BlockSealAuthorityId) -> bool {
		Self::authorities().contains(authority_id)
	}
	fn authorities() -> Vec<BlockSealAuthorityId> {
		AuthorityList::get().iter().map(|(_account, id)| id.clone()).collect()
	}
	fn authority_id_by_index() -> BTreeMap<u16, BlockSealAuthorityId> {
		let mut map = BTreeMap::new();
		for (i, id) in AuthorityList::get().into_iter().enumerate() {
			map.insert(i as u16, id.1);
		}
		map
	}
	fn authority_count() -> u16 {
		AuthorityList::get().len() as u16
	}
	fn get_authority(author: AccountId32) -> Option<BlockSealAuthorityId> {
		AuthorityList::get().iter().find_map(|(account, id)| {
			if *account == author {
				Some(id.clone())
			} else {
				None
			}
		})
	}
	fn block_peer(
		_block_hash: &<Block as sp_runtime::traits::Block>::Hash,
		_account_id: &AccountId32,
	) -> Option<MiningAuthority<BlockSealAuthorityId>> {
		XorClosest::get().clone()
	}
	fn get_rewards_account(_author: AccountId32) -> Option<AccountId32> {
		None
	}
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

parameter_types! {
	pub static ChainTransfers: Vec<(NotaryId, AccountId32, u64)> = vec![];
}
pub struct ChainTransferLookupImpl;
impl ChainTransferLookup<u64, AccountId32> for ChainTransferLookupImpl {
	fn is_valid_transfer_to_localchain(
		notary_id: NotaryId,
		account_id: &AccountId32,
		nonce: u64,
	) -> bool {
		ChainTransfers::get()
			.iter()
			.find(|(id, acc, n)| *id == notary_id && *acc == *account_id && *n == nonce)
			.is_some()
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
	type MaxHolds = ConstU32<100>;
}

pub fn set_argons(account_id: &AccountId32, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

impl pallet_notebook::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();

	type NotaryProvider = NotaryProviderImpl;
	type EventHandler = ();

	type ChainTransferLookup = ChainTransferLookupImpl;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
