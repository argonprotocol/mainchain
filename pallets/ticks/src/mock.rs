use crate as pallet_ticks;
use argon_notary_audit::VerifyError;
use argon_primitives::{
	tick::{TickDigest, Ticker},
	BlockVoteDigest, Digestset, NotebookDigest,
};

use pallet_prelude::*;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Timestamp: pallet_timestamp,
		Ticks: pallet_ticks,
	}
);

parameter_types! {

	pub static Digests: Digestset<VerifyError, u64> = Digestset {
		block_vote: BlockVoteDigest { voting_power: 500, votes_count: 1 },
		author: 1,
		voting_key: None,
		tick: TickDigest(2),
		fork_power: None,
		notebooks: NotebookDigest {
			notebooks: vec![],
		},
	};
}

pub struct DigestGetter;
impl Get<Result<Digestset<VerifyError, u64>, DispatchError>> for DigestGetter {
	fn get() -> Result<Digestset<VerifyError, u64>, DispatchError> {
		Ok(Digests::get())
	}
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

impl pallet_ticks::Config for Test {
	type WeightInfo = ();
	type Digests = DigestGetter;
}

pub fn new_test_ext(tick_duration_millis: u64) -> TestState {
	new_test_with_genesis::<Test>(|t: &mut Storage| {
		pallet_ticks::GenesisConfig::<Test> {
			ticker: Ticker::new(tick_duration_millis, 2),
			_phantom: Default::default(),
		}
		.assimilate_storage(t)
		.unwrap();
	})
}
