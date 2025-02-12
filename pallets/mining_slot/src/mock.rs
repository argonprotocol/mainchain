use crate as pallet_mining_slot;
use crate::OnNewSlot;
use argon_primitives::{
	block_seal::{MiningSlotConfig, RewardSharing, SlotId},
	tick::{Tick, Ticker},
	vault::{BondedArgonsProvider, ObligationError},
	BlockNumber, TickProvider, VaultId, VotingSchedule,
};
use env_logger::{Builder, Env};
use frame_support::{
	derive_impl, parameter_types,
	traits::{Currency, StorageMapShim},
	weights::constants::RocksDbWeight,
};
use sp_core::{Get, H256};
use sp_runtime::{
	impl_opaque_keys, testing::UintAuthorityId, traits::Zero, BuildStorage, FixedU128, Percent,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		MiningSlots: pallet_mining_slot,
		Balances: pallet_balances::<Instance1>,
		Ownership: pallet_balances::<Instance2>,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
}

parameter_types! {
	pub static TicksBetweenSlots: u64 = 1;
	pub static MaxCohortSize: u32 = 5;
	pub static MaxMiners: u32 = 10;
	pub static BlocksBeforeBidEndForVrfClose: u64 = 0;
	pub static SlotBiddingStartAfterTicks: u64 = 3;
	pub static TargetBidsPerSlot: u32 = 5;
	pub static MinOwnershipBondAmount: Balance = 1;
	pub static MaxOwnershipPercent: Percent = Percent::from_float(0.8);
	pub const ArgonotsPercentAdjustmentDamper: FixedU128 = FixedU128::from_rational(20, 100);

	pub static ExistentialDeposit: Balance = 1;
	pub const MinimumBondAmount:u128 = 1_000_000;
}

pub type ObligationId = u64;
pub type Balance = u128;

type ArgonToken = pallet_balances::Instance1;
impl pallet_balances::Config<ArgonToken> for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
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

pub fn set_ownership(account_id: u64, amount: Balance) {
	let _ = Ownership::make_free_balance_be(&account_id, amount);
	drop(Ownership::issue(amount));
}

pub fn set_argons(account_id: u64, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

pub(crate) type OwnershipToken = pallet_balances::Instance2;
impl pallet_balances::Config<OwnershipToken> for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type ReserveIdentifier = [u8; 8];
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test, OwnershipToken>,
		Self::AccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_types! {
	pub static Obligations: Vec<(ObligationId, VaultId, u64, Balance)> = vec![];
	pub static NextObligationId: u64 = 1;
	pub static VaultSharing: Option<RewardSharing<u64>> = None;

	pub static LastSlotRemoved: Vec<(u64, UintAuthorityId)> = vec![];
	pub static LastSlotAdded: Vec<(u64, UintAuthorityId)> = vec![];
	pub static GrandaRotations: Vec<SlotId> = vec![];

	// set slot bidding active by default
	pub static ElapsedTicks: u64 = 3;
	pub static CurrentTick: Tick = 1;

	pub static GrandpaRotationFrequency: BlockNumber = 10;
}

pub struct StaticBondProvider;
impl BondedArgonsProvider for StaticBondProvider {
	type Balance = Balance;
	type AccountId = u64;

	fn create_bonded_argons(
		vault_id: VaultId,
		account_id: Self::AccountId,
		amount: Self::Balance,
		_reserve_until_tick: Tick,
		modify_obligation_id: Option<ObligationId>,
	) -> Result<
		(argon_primitives::ObligationId, Option<RewardSharing<u64>>, Self::Balance),
		ObligationError,
	> {
		if let Some(modify_obligation_id) = modify_obligation_id {
			let res = Obligations::mutate(|a| {
				let existing = a
					.iter_mut()
					.find(|(id, v, _, _)| *id == modify_obligation_id && *v == vault_id);
				if let Some((_, _, _, a)) = existing {
					*a = a.saturating_add(amount);
					return Some(*a);
				}
				None
			});
			if let Some(total) = res {
				return Ok((modify_obligation_id, VaultSharing::get(), total));
			}
		}
		let obligation_id = NextObligationId::get();
		NextObligationId::set(obligation_id + 1);
		Obligations::mutate(|a| a.push((obligation_id, vault_id, account_id, amount)));
		Ok((obligation_id, VaultSharing::get(), amount))
	}

	fn cancel_bonded_argons(
		obligation_id: argon_primitives::ObligationId,
	) -> Result<Self::Balance, ObligationError> {
		let mut amount = Self::Balance::zero();
		Obligations::mutate(|a| {
			if let Some(pos) = a.iter().position(|(id, _, _, _)| *id == obligation_id) {
				let (_, _, _, amt) = a.remove(pos);
				amount = amt
			}
		});
		Ok(amount)
	}
}

pub struct StaticNewSlotEvent;
impl OnNewSlot<u64> for StaticNewSlotEvent {
	type Key = UintAuthorityId;
	fn rotate_grandpas(
		current_slot_id: SlotId,
		removed_authorities: Vec<(&u64, Self::Key)>,
		added_authorities: Vec<(&u64, Self::Key)>,
	) {
		LastSlotRemoved::set(removed_authorities.into_iter().map(|(a, b)| (*a, b)).collect());
		LastSlotAdded::set(added_authorities.into_iter().map(|(a, b)| (*a, b)).collect());
		GrandaRotations::mutate(|a| a.push(current_slot_id));
	}
}

impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub dummy: UintAuthorityId,
	}
}

impl Get<UintAuthorityId> for MockSessionKeys {
	fn get() -> UintAuthorityId {
		MockSessionKeys { dummy: 0.into() }.dummy
	}
}

impl From<UintAuthorityId> for MockSessionKeys {
	fn from(dummy: UintAuthorityId) -> Self {
		Self { dummy }
	}
}

impl From<u64> for MockSessionKeys {
	fn from(dummy: u64) -> Self {
		Self { dummy: dummy.into() }
	}
}

pub struct StaticTickProvider;
impl TickProvider<Block> for StaticTickProvider {
	fn previous_tick() -> Tick {
		CurrentTick::get() - 1
	}
	fn current_tick() -> Tick {
		CurrentTick::get()
	}
	fn voting_schedule() -> VotingSchedule {
		todo!()
	}
	fn ticker() -> Ticker {
		Ticker::new(1, 2)
	}
	fn elapsed_ticks() -> Tick {
		ElapsedTicks::get()
	}
	fn blocks_at_tick(_: Tick) -> Vec<H256> {
		todo!()
	}
}

impl pallet_mining_slot::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxMiners = MaxMiners;
	type MaxCohortSize = MaxCohortSize;
	type ArgonotsPercentAdjustmentDamper = ArgonotsPercentAdjustmentDamper;
	type MinimumArgonotsPerSeat = MinOwnershipBondAmount;
	type MaximumArgonotProrataPercent = MaxOwnershipPercent;
	type TargetBidsPerSlot = TargetBidsPerSlot;
	type Balance = Balance;
	type OwnershipCurrency = Ownership;
	type RuntimeHoldReason = RuntimeHoldReason;
	type BondedArgonsProvider = StaticBondProvider;
	type SlotEvents = (StaticNewSlotEvent,);
	type MiningAuthorityId = UintAuthorityId;
	type Keys = MockSessionKeys;
	type TickProvider = StaticTickProvider;

	type GrandpaRotationBlocks = GrandpaRotationFrequency;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();

	let mining_config = MiningSlotConfig {
		slot_bidding_start_after_ticks: SlotBiddingStartAfterTicks::get(),
		ticks_between_slots: TicksBetweenSlots::get(),
		ticks_before_bid_end_for_vrf_close: BlocksBeforeBidEndForVrfClose::get(),
	};

	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_mining_slot::GenesisConfig::<Test> { mining_config, _phantom: Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();

	sp_io::TestExternalities::new(t)
}
