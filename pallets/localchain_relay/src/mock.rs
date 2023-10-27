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
	block_seal::{AuthorityDistance, AuthorityProvider, BlockSealersProvider, Host},
	notary::{NotaryId, NotaryProvider, NotarySignature},
	BlockSealAuthorityId,
};

use crate as pallet_localchain_relay;

pub type Balance = u128;
pub(crate) type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = BlockNumberFor<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		LocalchainRelay: pallet_localchain_relay
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
	pub const MinimumBondAmount:u128 = 1_000;
	pub const BlocksPerYear:u32 = 1440*365;

	pub const LocalchainAccountId :u32 = 1;
	pub static MaxNotebookBlocksToRemember :u32 = 1;
	pub const MaxNotebookTransfers :u32 = 1;
	pub static MaxPendingTransfersOutPerBlock :u32 = 1;
	pub static RequiredNotebookAuditors :u32 = 1;
	pub static TransferExpirationBlocks :u32 = 2;

	pub const LocalchainPalletId: PalletId = PalletId(*b"loclchai");

	pub static BlockSealers: BTreeMap<BlockNumber, Vec<BlockSealAuthorityId>> = BTreeMap::new();

	pub static IsProofOfCompute: bool = false;
}
pub struct HistoricalBlockSealersLookupImpl;
impl BlockSealersProvider<BlockNumber, BlockSealAuthorityId> for HistoricalBlockSealersLookupImpl {
	fn get_active_block_sealers_of(n: BlockNumber) -> Vec<(u16, BlockSealAuthorityId, Vec<Host>)> {
		BlockSealers::get()
			.get(&n)
			.unwrap_or(&Vec::new())
			.clone()
			.iter()
			.enumerate()
			.map(|(idx, a)| (idx as u16, a.clone(), vec![]))
			.collect::<Vec<_>>()
	}
	fn is_using_proof_of_compute() -> bool {
		IsProofOfCompute::get()
	}
}
parameter_types! {
	pub static AuthorityList: Vec<(AccountId32, BlockSealAuthorityId)> = vec![];
	pub static XorClosest: Vec<AuthorityDistance<BlockSealAuthorityId>> = vec![];
}
pub struct StaticAuthorityProvider;
impl AuthorityProvider<BlockSealAuthorityId, Block, AccountId32> for StaticAuthorityProvider {
	fn miner_zero() -> Option<(u16, BlockSealAuthorityId, Vec<Host>)> {
		None
	}
	fn is_active(authority_id: &BlockSealAuthorityId) -> bool {
		Self::authorities().contains(authority_id)
	}
	fn authorities() -> Vec<BlockSealAuthorityId> {
		AuthorityList::get().iter().map(|(_account, id)| id.clone()).collect()
	}
	fn authorities_by_index() -> BTreeMap<u16, BlockSealAuthorityId> {
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
	fn block_peers(
		_block_hash: &<Block as sp_runtime::traits::Block>::Hash,
		_account_id: AccountId32,
		_closest: u8,
	) -> Vec<AuthorityDistance<BlockSealAuthorityId>> {
		XorClosest::get().clone()
	}
}

pub struct NotaryProviderImpl;
impl NotaryProvider<Block> for NotaryProviderImpl {
	fn verify_signature(_: NotaryId, _: NumberFor<Block>, _: &H256, _: &NotarySignature) -> bool {
		true
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

impl pallet_localchain_relay::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type Balance = Balance;
	type HistoricalBlockSealersLookup = HistoricalBlockSealersLookupImpl;
	type MaxNotebookBlocksToRemember = MaxNotebookBlocksToRemember;
	type MaxPendingTransfersOutPerBlock = MaxPendingTransfersOutPerBlock;
	type NotaryProvider = NotaryProviderImpl;
	type PalletId = LocalchainPalletId;
	type AuthorityProvider = StaticAuthorityProvider;
	type RequiredNotebookAuditors = RequiredNotebookAuditors;
	type TransferExpirationBlocks = TransferExpirationBlocks;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
