use std::{collections::BTreeMap, net::Ipv4Addr};

use env_logger::{Builder, Env};
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64},
};
use sp_core::{crypto::AccountId32, ConstU32, OpaquePeerId, H256, U256};
use sp_runtime::{
	testing::UintAuthorityId,
	traits::{BlakeTwo256, IdentityLookup, UniqueSaturatedInto},
	BoundedVec, BuildStorage,
};

use ulx_primitives::block_seal::{
	AuthorityDistance, AuthorityProvider, BlockSealAuthorityId, Host, PeerId,
};

use crate as pallet_block_seal;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		BlockSeal: pallet_block_seal,
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub static AuthorityList: Vec<(u64, BlockSealAuthorityId)> = vec![];
	pub static XorClosest: Vec<AuthorityDistance<BlockSealAuthorityId>> = vec![];
	pub static AuthorityCountInitiatingTaxProof: u32 = 1;
}

pub struct StaticAuthorityProvider;
impl AuthorityProvider<BlockSealAuthorityId, Block, u64> for StaticAuthorityProvider {
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
	fn get_authority(author: u64) -> Option<BlockSealAuthorityId> {
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

impl pallet_block_seal::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type HistoricalBlockSealersToKeep = ConstU32<10>;
	type AuthorityId = BlockSealAuthorityId;
	type AuthorityProvider = StaticAuthorityProvider;
}

pub fn set_authorities(authorities: Vec<u64>, xor_closest_accounts: Vec<u64>) {
	let authorities: Vec<(u64, BlockSealAuthorityId)> = authorities
		.into_iter()
		.map(|a| (a, UintAuthorityId(a).to_public_key()))
		.collect();
	AuthorityList::mutate(|a| {
		a.truncate(0);
		a.extend(authorities.clone());
	});
	XorClosest::mutate(|a| {
		a.truncate(0);
		let xor_closest = xor_closest_accounts
			.into_iter()
			.map(|account_id| {
				let index = authorities.iter().position(|x| x.0 == account_id).unwrap_or_default();
				let id = authorities[index].1.clone();
				// put in a nonsense distance for now
				let distance = U256::from(index as u32);
				AuthorityDistance {
					authority_index: index.unique_saturated_into(),
					authority_id: id.clone(),
					peer_id: PeerId(OpaquePeerId::default()),
					rpc_hosts: BoundedVec::truncate_from(vec![Host {
						ip: Ipv4Addr::new(127, 0, 0, 1).into(),
						port: 3000,
						is_secure: false,
					}]),
					distance,
				}
			})
			.collect::<Vec<_>>();

		a.extend(xor_closest);
	});
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(
	authorities: Vec<u64>,
	xor_closest_accounts: Vec<u64>,
	min_seal_signers: u32,
	closest_xor_authorities_required: u32,
) -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();

	set_authorities(authorities, xor_closest_accounts);

	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into();
	pallet_block_seal::GenesisConfig::<Test> {
		min_seal_signers,
		closest_xor_authorities_required,
		authority_count_starting_tax_seal: AuthorityCountInitiatingTaxProof::get(),
		_phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::new(t)
}
