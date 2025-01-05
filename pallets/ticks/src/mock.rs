use crate as pallet_ticks;
use argon_notary_audit::VerifyError;
use argon_primitives::{
	tick::{TickDigest, Ticker},
	BlockVoteDigest, Digestset, NotebookDigest,
};
use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types, traits::ConstU64};
use sp_runtime::{traits::Get, BuildStorage, DispatchError};

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

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(tick_duration_millis: u64) -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_ticks::GenesisConfig::<Test> {
		ticker: Ticker::new(tick_duration_millis, 2),
		_phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::new(t)
}
