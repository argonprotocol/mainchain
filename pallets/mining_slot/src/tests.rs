use crate::{
	Error, Event, HoldReason, Registration,
	mock::{MiningSlots, Ownership, *},
	pallet::{
		AccountIndexLookup, ActiveMinersCount, ArgonotsPerMiningSeat, AveragePricePerSeat,
		BidsForNextSlotCohort, FrameStartBlockNumbers, HistoricalBidsPerSlot,
		IsNextSlotBiddingOpen, MinerXorKeysByCohort, MinersByCohort, MiningConfig, NextCohortSize,
		NextFrameId,
	},
};
use argon_primitives::{
	AuthorityProvider,
	block_seal::{MiningAuthority, MiningBidStats, MiningRegistration},
};
use frame_support::traits::fungible::Unbalanced;
use frame_system::AccountInfo;
use pallet_balances::{AccountData, Event as OwnershipEvent, ExtraFlags};
use pallet_prelude::*;
use sp_core::bounded_vec;
use sp_runtime::testing::UintAuthorityId;
use std::{collections::HashMap, env};

#[test]
fn it_doesnt_add_cohorts_until_time() {
	TicksBetweenSlots::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		BidsForNextSlotCohort::<Test>::set(bounded_vec![MiningRegistration {
			account_id: 1,
			argonots: 0,
			bid: 0,
			authority_keys: 1.into(),
			starting_frame_id: 1,
			external_funding_account: None,
			bid_at_tick: 1
		}]);

		MiningSlots::on_initialize(1);
		MiningSlots::on_finalize(1);

		assert_eq!(BidsForNextSlotCohort::<Test>::get().len(), 1);
		assert_eq!(BidsForNextSlotCohort::<Test>::get()[0].account_id, 1,);
	});
}

#[test]
fn get_validation_window_blocks() {
	MinCohortSize::set(2);
	FramesPerMiningTerm::set(5);
	TicksBetweenSlots::set(1);

	new_test_ext().execute_with(|| {
		assert_eq!(MiningSlots::get_mining_window_ticks(), 5);
	});

	MinCohortSize::set(5);
	FramesPerMiningTerm::set(2);
	TicksBetweenSlots::set(10);

	new_test_ext().execute_with(|| {
		assert_eq!(MiningSlots::get_mining_window_ticks(), 2 * 10);
	});
}

#[test]
fn calculate_frame_id() {
	MinCohortSize::set(10);
	MaxCohortSize::set(10);
	FramesPerMiningTerm::set(10);
	let ticks_between_slots = 1440;
	TicksBetweenSlots::set(ticks_between_slots);

	SlotBiddingStartAfterTicks::set(20_000);
	let mining_window = 14400;

	let genesis = 28_000_000;
	let mut current_tick = genesis + 19_999;
	CurrentTick::set(current_tick);

	new_test_ext().execute_with(|| {
		assert!(!IsNextSlotBiddingOpen::<Test>::get());
		ElapsedTicks::set(current_tick - genesis);
		// bidding should open at 20k
		MiningSlots::on_initialize(1);
		MiningSlots::on_finalize(1);
		assert!(!IsNextSlotBiddingOpen::<Test>::get());
		// now go to 20k
		current_tick += 1;
		CurrentTick::set(current_tick);
		ElapsedTicks::set(current_tick - genesis);
		MiningSlots::on_initialize(2);
		MiningSlots::on_finalize(2);
		assert!(IsNextSlotBiddingOpen::<Test>::get());

		current_tick += ticks_between_slots - 1;
		CurrentTick::set(current_tick);
		ElapsedTicks::set(current_tick - genesis);
		MiningSlots::on_initialize(3);
		MiningSlots::on_finalize(3);
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		assert_eq!(MiningSlots::slot_1_tick(), current_tick + 1);
		assert_eq!(MiningSlots::get_next_slot_tick(), current_tick + 1);
		assert_eq!(MiningSlots::ticks_since_mining_start(), 0);
		assert_eq!(MiningSlots::calculated_frame_id(), 0);
		assert_eq!(
			MiningSlots::get_next_slot_era(),
			(current_tick + 1, current_tick + 1 + mining_window)
		);

		current_tick += 1;
		ElapsedTicks::mutate(|a| *a += 1u64);
		CurrentTick::set(current_tick);
		MiningSlots::on_initialize(4);
		MiningSlots::on_finalize(4);
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		assert_eq!(MiningSlots::slot_1_tick(), current_tick);
		assert_eq!(MiningSlots::ticks_since_mining_start(), 0);
		assert_eq!(MiningSlots::calculated_frame_id(), 1);
		assert_eq!(NextFrameId::<Test>::get(), 2, "if not set, will show the current era");
		assert_eq!(
			MiningSlots::get_next_slot_era(),
			(current_tick + 1440, current_tick + 1440 + mining_window)
		);
		assert_eq!(MiningSlots::get_next_slot_tick(), current_tick + 1440);
	});
}

#[test]
fn it_updates_frame_id_at_right_time() {
	MinCohortSize::set(10);
	MaxCohortSize::set(10);
	FramesPerMiningTerm::set(10);
	let ticks_between_slots = 1440;
	TicksBetweenSlots::set(ticks_between_slots);

	SlotBiddingStartAfterTicks::set(55_661);

	let genesis = 28_949_839;
	let _bidding_start_tick = 29_005_500;
	let current_tick = 29_118_062;
	CurrentTick::set(current_tick);

	let _frame_78_tick = 29_117_820; // bidding_start_tick + (1440 * 78);

	new_test_ext().execute_with(|| {
		NextFrameId::<Test>::set(79);
		ElapsedTicks::mutate(|a| *a = current_tick - genesis);
		CurrentTick::set(current_tick);
		assert_eq!(MiningSlots::calculated_frame_id(), 78);
		MiningSlots::on_initialize(4);
		MiningSlots::on_finalize(4);
		assert_eq!(NextFrameId::<Test>::get(), 79);

		ElapsedTicks::mutate(|a| *a = genesis + SlotBiddingStartAfterTicks::get() - 1440 * 79);
	});
}

#[test]
fn extends_bidding_if_mining_slot_extends() {
	MinCohortSize::set(10);
	MaxCohortSize::set(10);
	FramesPerMiningTerm::set(10);
	let ticks_between_slots = 1440;
	TicksBetweenSlots::set(ticks_between_slots);

	SlotBiddingStartAfterTicks::set(20_000);
	let mining_window = 14400;

	let genesis = 28_000_000;
	let mut current_tick = genesis + 19_999;
	CurrentTick::set(current_tick);

	new_test_ext().execute_with(|| {
		set_ownership(1, 1000u32.into());
		set_ownership(2, 1000u32.into());

		NextFrameId::<Test>::set(1);
		assert!(!IsNextSlotBiddingOpen::<Test>::get());
		// now go to 20k
		current_tick += 1;
		CurrentTick::set(current_tick);
		ElapsedTicks::set(current_tick - genesis);
		MiningSlots::on_initialize(2);
		MiningSlots::on_finalize(2);
		assert!(IsNextSlotBiddingOpen::<Test>::get());

		assert_eq!(MiningSlots::slot_1_tick(), current_tick + 1440);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 0, 1.into(), None));
		MiningConfig::<Test>::mutate(|a| {
			a.slot_bidding_start_after_ticks = 21_000;
		});
		MiningSlots::on_initialize(3);
		MiningSlots::on_finalize(3);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), 0, 2.into(), None));

		current_tick += 1440;
		ElapsedTicks::set(current_tick - genesis);
		CurrentTick::set(current_tick);
		MiningSlots::on_initialize(4);
		MiningSlots::on_finalize(4);
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		// now it's bumped out
		assert_eq!(MiningSlots::slot_1_tick(), current_tick + 1000);
		assert_eq!(MiningSlots::ticks_since_mining_start(), 0);
		assert_eq!(MiningSlots::calculated_frame_id(), 0);
		assert_eq!(NextFrameId::<Test>::get(), 1);

		current_tick += 1000;
		ElapsedTicks::set(current_tick - genesis);
		CurrentTick::set(current_tick);
		MiningSlots::on_initialize(5);
		MiningSlots::on_finalize(5);
		assert_eq!(MiningSlots::slot_1_tick(), current_tick);
		assert_eq!(NextFrameId::<Test>::get(), 2,);
		assert_eq!(
			MiningSlots::get_next_slot_era(),
			(current_tick + 1440, current_tick + 1440 + mining_window)
		);
		assert_eq!(MiningSlots::get_next_slot_tick(), current_tick + 1440);
	});
}

#[test]
fn it_adds_new_cohorts_on_block() {
	TicksBetweenSlots::set(2);
	FramesPerMiningTerm::set(3);
	MinCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(8);
		ElapsedTicks::set(8);
		CurrentTick::set(8);
		assert_eq!(MiningSlots::slot_1_tick(), 4);
		assert_eq!(MiningSlots::calculated_frame_id(), 3);
		NextFrameId::<Test>::put(4);

		for frame_id in [1, 3] {
			for i in [1, 2] {
				let frame_account_offset = (frame_id - 1) * 3;
				let account_id: u64 = (i + 4 + frame_account_offset as u32).into();
				let mut index = 0;
				MinersByCohort::<Test>::mutate(frame_id, |cohort| {
					index = cohort.len();
					cohort
						.try_push(MiningRegistration {
							account_id,
							argonots: 0,
							bid: 0,
							authority_keys: account_id.into(),
							starting_frame_id: frame_id,
							external_funding_account: None,
							bid_at_tick: 1,
						})
						.ok();
				});
				AccountIndexLookup::<Test>::insert(account_id, (frame_id, index as u32));
				MinerXorKeysByCohort::<Test>::try_mutate(|x| {
					let hash = blake2_256(&account_id.to_le_bytes());
					if !x.contains_key(&frame_id) {
						x.try_insert(frame_id, BoundedVec::default()).unwrap();
					}
					x.get_mut(&frame_id).unwrap().try_insert(index, U256::from_big_endian(&hash))
				})
				.unwrap();
			}
		}
		ActiveMinersCount::<Test>::put(4);
		println!("MinersByCohort: {:#?}", MinersByCohort::<Test>::iter().collect::<Vec<_>>());

		let cohort = BoundedVec::truncate_from(vec![MiningRegistration {
			account_id: 1,
			argonots: 0,
			bid: 0,
			authority_keys: 1.into(),
			starting_frame_id: 5,
			external_funding_account: None,
			bid_at_tick: 1,
		}]);

		BidsForNextSlotCohort::<Test>::set(cohort.clone());

		CurrentTick::set(10);
		ElapsedTicks::set(10);
		assert_eq!(MiningSlots::calculated_frame_id(), 4);
		// on 8, we're filling indexes 2 and 3 [0, 1, -> 2, -> 3, _, _]
		MiningSlots::on_initialize(10);
		MiningSlots::on_finalize(10);
		assert_eq!(NextFrameId::<Test>::get(), 5);

		// re-fetch
		assert_eq!(
			ActiveMinersCount::<Test>::get(),
			3,
			"Should have 3 validators still after insertion"
		);
		let miners = MinersByCohort::<Test>::iter().collect::<HashMap<_, _>>();
		let authority_hash_frames = MinerXorKeysByCohort::<Test>::get().into_inner();
		assert_eq!(
			BidsForNextSlotCohort::<Test>::get().len(),
			0,
			"Queued mining_slot for block 8 should be removed"
		);
		assert_eq!(
			authority_hash_frames.len(),
			2,
			"Should have only 2 authority hash frames after insertion"
		);

		let authority_hashes = authority_hash_frames
			.iter()
			.flat_map(|(frame_id, hashes)| {
				hashes.iter().enumerate().map(move |(i, hash)| ((frame_id, i), *hash))
			})
			.collect::<Vec<_>>();
		assert_eq!(authority_hashes.len(), 3, "Should have 3 authority hashes after insertion");

		assert_eq!(miners.len(), 2, "Should have 3 mining frames still after insertion");
		assert!(miners.contains_key(&3), "Should have miner for frame 3, 4");
		assert!(miners.contains_key(&4), "Should have miner for frame 3, 4");
		assert!(!miners.contains_key(&1), "Should no longer have miners at 1");

		assert_eq!(
			AccountIndexLookup::<Test>::get(1),
			Some((4, 0)),
			"Should add an index lookup for account 1 at index 2"
		);
		assert!(
			!AccountIndexLookup::<Test>::contains_key(6),
			"Should no longer have account 6 registered"
		);
		assert!(
			!AccountIndexLookup::<Test>::contains_key(7),
			"Should no longer have account 7 registered"
		);
		assert!(
			!authority_hashes.iter().any(|(index, _hash)| index.0 == &1),
			"Should no longer have frame 1"
		);

		// check what was called from the events
		assert_eq!(
			LastSlotAdded::get().len(),
			1,
			"Should emit a new slot event for the new cohort"
		);
		assert_eq!(LastSlotRemoved::get().len(), 2);

		System::assert_last_event(
			Event::NewMiners {
				new_miners: BoundedVec::truncate_from(cohort.to_vec()),
				released_miners: 2,
				frame_id: NextFrameId::<Test>::get() - 1,
			}
			.into(),
		)
	});
}

#[test]
fn it_releases_argonots_when_a_window_closes() {
	TicksBetweenSlots::set(2);
	FramesPerMiningTerm::set(3);
	MinCohortSize::set(2);
	MaxCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		ArgonotsPerMiningSeat::<Test>::set(1000);
		NextFrameId::<Test>::put(2);
		CurrentTick::set(7);
		ElapsedTicks::set(7);
		assert_eq!(MiningSlots::slot_1_tick(), 2);
		assert_eq!(MiningSlots::calculated_frame_id(), 3);
		NextFrameId::<Test>::set(4);
		System::set_block_number(7);

		for frame_id in 1..2 {
			for i in 0..4u32 {
				let account_id: u64 = i.into();
				set_ownership(account_id, 1000u32.into());
				set_argons(account_id, 10_000u32.into());
				MinersByCohort::<Test>::mutate(frame_id, |cohort| {
					cohort
						.try_push(MiningRegistration {
							account_id,
							argonots: 0,
							bid: 0,
							authority_keys: account_id.into(),
							starting_frame_id: frame_id,
							external_funding_account: None,
							bid_at_tick: 1,
						})
						.ok();
				});
				AccountIndexLookup::<Test>::insert(account_id, (frame_id, i));
			}
		}
		ActiveMinersCount::<Test>::put(4);
		IsNextSlotBiddingOpen::<Test>::set(true);
		assert_eq!(MiningSlots::get_next_slot_era(), (8, 8 + (2 * 3)));

		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), 0, 1.into(), None));

		CurrentTick::set(8);
		ElapsedTicks::set(8);
		System::set_block_number(8);

		MiningSlots::on_initialize(8);
		MiningSlots::on_finalize(8);

		System::assert_last_event(
			Event::NewMiners {
				frame_id: NextFrameId::<Test>::get() - 1,
				new_miners: BoundedVec::truncate_from(vec![MiningRegistration {
					account_id: 2,
					argonots: 1000u32.into(),
					bid: 0,
					authority_keys: 1.into(),
					starting_frame_id: 4,
					external_funding_account: None,
					bid_at_tick: 7,
				}]),
				released_miners: 2,
			}
			.into(),
		);

		System::assert_has_event(
			OwnershipEvent::<Test, OwnershipToken>::Endowed {
				account: 3,
				free_balance: 1000u32.into(),
			}
			.into(),
		);

		assert_eq!(Ownership::free_balance(2), 0);
		assert_eq!(Ownership::total_balance(&2), 1000);

		assert_eq!(Ownership::free_balance(3), 1000);

		assert!(System::account_exists(&0));
		assert!(System::account_exists(&1));
		assert!(System::account_exists(&2));
		assert!(System::account_exists(&3));
	});
}

#[test]
fn it_holds_ownership_tokens_for_a_slot() {
	TicksBetweenSlots::set(3);
	FramesPerMiningTerm::set(3);
	MinCohortSize::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(6);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(2), 0, 1.into(), None),
			Error::<Test>::SlotNotTakingBids
		);

		SlotBiddingStartAfterTicks::set(0);
		IsNextSlotBiddingOpen::<Test>::set(true);

		set_ownership(3, 5000u32.into());
		let share_amount = 500;
		MiningSlots::on_initialize(6);
		MiningSlots::on_finalize(6);
		ArgonotsPerMiningSeat::<Test>::set(share_amount);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(1), 0, 1.into(), None),
			Error::<Test>::InsufficientOwnershipTokens
		);

		set_ownership(1, 1000u32.into());

		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 0, 1.into(), None));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 0u32.into(), index: 0 }.into(),
		);
		assert_eq!(Ownership::free_balance(1), 1000 - share_amount);

		// should be able to re-register
		set_argons(1, 1_100_000);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 1_000_000u32.into(), 1.into(), None));
		assert_eq!(
			BidsForNextSlotCohort::<Test>::get()
				.iter()
				.map(|a| a.account_id)
				.collect::<Vec<_>>(),
			vec![1]
		);
		assert_eq!(
			Ownership::free_balance(1),
			1000 - share_amount,
			"should not alter reserved balance"
		);
		assert_eq!(Ownership::total_balance(&1), 1000, "should still have their full balance");

		assert!(System::account_exists(&1));
		assert!(System::account_exists(&3));
	});
}

#[test]
fn it_wont_accept_bids_until_bidding_starts() {
	TicksBetweenSlots::set(4);
	FramesPerMiningTerm::set(3);
	MinCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(12);
	ElapsedTicks::set(11);

	new_test_ext().execute_with(|| {
		set_ownership(2, 100u32.into());
		for i in 1..11u64 {
			System::set_block_number(i);

			MiningSlots::on_initialize(i);
			MiningSlots::on_finalize(i);
			assert_err!(
				MiningSlots::bid(RuntimeOrigin::signed(2), 0, 1.into(), None),
				Error::<Test>::SlotNotTakingBids
			);
		}

		System::set_block_number(12 + TicksBetweenSlots::get());
		ElapsedTicks::set(12 + TicksBetweenSlots::get());
		CurrentTick::set(12 + TicksBetweenSlots::get());
		MiningSlots::on_initialize(12 + TicksBetweenSlots::get());
		MiningSlots::on_finalize(12 + TicksBetweenSlots::get());

		assert!(IsNextSlotBiddingOpen::<Test>::get(), "bidding should now be open");
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), 0, 1.into(), None));
	});
}

#[test]
fn it_wont_let_you_reuse_ownership_tokens_for_two_bids() {
	TicksBetweenSlots::set(4);
	FramesPerMiningTerm::set(3);
	MinCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		ElapsedTicks::set(12);
		CurrentTick::set(12);
		System::set_block_number(12);

		IsNextSlotBiddingOpen::<Test>::set(true);
		NextFrameId::<Test>::put(MiningSlots::calculated_frame_id() + 1);

		assert_eq!(MiningSlots::get_next_slot_era(), (16, 16 + (4 * 3)));
		set_ownership(2, 100u32.into());
		set_ownership(1, 100u32.into());
		MiningSlots::on_initialize(12);
		MiningSlots::on_finalize(12);

		let ownership = (200 / 6) as u128;
		ArgonotsPerMiningSeat::<Test>::set(ownership);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 0, 1.into(), None));
		System::set_block_number(16);
		CurrentTick::set(16);
		ElapsedTicks::set(16);
		MiningSlots::on_initialize(16);
		MiningSlots::on_finalize(16);

		System::assert_last_event(
			Event::NewMiners {
				frame_id: NextFrameId::<Test>::get() - 1,
				new_miners: BoundedVec::truncate_from(vec![MiningRegistration {
					account_id: 1,
					argonots: ownership,
					bid: 0,
					authority_keys: 1.into(),
					starting_frame_id: 4,
					external_funding_account: None,
					bid_at_tick: 12,
				}]),
				released_miners: 0,
			}
			.into(),
		);
		assert_eq!(FrameStartBlockNumbers::<Test>::get().to_vec(), vec![16]);
		assert_eq!(MiningSlots::get_next_slot_era(), (20, 20 + (4 * 3)));

		CurrentTick::set(20);
		ElapsedTicks::set(20);
		System::set_block_number(20);
		MiningSlots::on_initialize(20);
		MiningSlots::on_finalize(20);
		assert_eq!(MiningSlots::get_next_slot_era(), (24, 24 + (4 * 3)));

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(1), 0, 1.into(), None),
			Error::<Test>::CannotRegisterOverlappingSessions
		);
		assert!(System::account_exists(&1));
		assert!(System::account_exists(&2));
	})
}

#[test]
fn it_will_order_bids() {
	TicksBetweenSlots::set(3);
	FramesPerMiningTerm::set(2);
	MinCohortSize::set(2);
	ExistentialDeposit::set(100_000);
	let bid_pool_account_id = BidPoolAccountId::get();

	new_test_ext().execute_with(|| {
		ElapsedTicks::set(6);
		CurrentTick::set(6);
		System::set_block_number(6);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(2), 0, 1.into(), None),
			Error::<Test>::SlotNotTakingBids
		);

		SlotBiddingStartAfterTicks::set(0);
		IsNextSlotBiddingOpen::<Test>::set(true);

		set_ownership(1, 101_000u32.into());
		set_ownership(2, 100_500u32.into());
		set_ownership(3, 100_500u32.into());

		MiningSlots::on_initialize(6);
		MiningSlots::on_finalize(6);
		let share_amount = 500;
		ArgonotsPerMiningSeat::<Test>::set(share_amount);

		set_argons(1, 3_100_000);
		// Bids must be increments of 10 cents
		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(1), 1_000u32.into(), 1.into(), None),
			Error::<Test>::InvalidBidAmount
		);
		// 1. Account 1 bids
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 1_000_000u32.into(), 1.into(), None));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 1_000_000u32.into(), index: 0 }
				.into(),
		);
		assert_eq!(Ownership::free_balance(1), 100_500);
		assert_eq!(Balances::free_balance(1), 2_100_000);
		assert_eq!(Balances::free_balance(bid_pool_account_id), 1_000_000);
		let first_bid = HistoricalBidsPerSlot::<Test>::get().into_inner()[0].clone();
		assert_eq!(first_bid.bids_count, 1);
		assert_eq!(first_bid.bid_amount_min, 1_000_000);
		assert_eq!(first_bid.bid_amount_max, 1_000_000);
		assert_eq!(first_bid.bid_amount_sum, 1_000_000);

		// 2. Account 2 bids highest and takes top slot
		// should be able to re-register
		set_argons(2, 5_000_000);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), 2_000_000, 2.into(), None));
		assert_eq!(Balances::free_balance(2), 3_000_000);
		assert_eq!(Balances::free_balance(bid_pool_account_id), 3_000_000);
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 2, bid_amount: 2_000_000, index: 0 }.into(),
		);

		let first_bid = HistoricalBidsPerSlot::<Test>::get().into_inner()[0].clone();
		assert_eq!(first_bid.bids_count, 2);
		assert_eq!(first_bid.bid_amount_min, 1_000_000);
		assert_eq!(first_bid.bid_amount_max, 2_000_000);
		assert_eq!(first_bid.bid_amount_sum, 3_000_000);
		assert_eq!(
			BidsForNextSlotCohort::<Test>::get()
				.iter()
				.map(|a| a.account_id)
				.collect::<Vec<_>>(),
			vec![2, 1]
		);

		// 3. Account 2 bids above 1

		// should be able to re-register
		System::reset_events();
		set_argons(3, 5_000_000);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(3), 2_000_000, 3.into(), None));
		assert_eq!(Balances::free_balance(3), 3_000_000);
		assert_eq!(Balances::free_balance(bid_pool_account_id), 4_000_000,);
		assert_eq!(Balances::free_balance(1), 3_100_000, "bid is returned");
		System::assert_has_event(
			Event::SlotBidderAdded { account_id: 3, bid_amount: 2_000_000, index: 1 }.into(),
		);
		System::assert_has_event(
			Event::SlotBidderDropped { account_id: 1, preserved_argonot_hold: false }.into(),
		);
		assert_eq!(HistoricalBidsPerSlot::<Test>::get().into_inner()[0].clone().bids_count, 3);

		assert_eq!(
			BidsForNextSlotCohort::<Test>::get()
				.iter()
				.map(|a| a.account_id)
				.collect::<Vec<_>>(),
			vec![2, 3]
		);

		// should return hold amount
		assert_eq!(Ownership::free_balance(1), 101_000);
		assert!(Ownership::hold_available(&HoldReason::RegisterAsMiner.into(), &1));

		// 4. Account 1 increases bid and resubmits

		System::reset_events();
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 2_010_000, 1.into(), None));
		assert_eq!(Balances::free_balance(1), 1_090_000);
		assert_eq!(Balances::free_balance(bid_pool_account_id), 4_010_000);

		let first_bid = HistoricalBidsPerSlot::<Test>::get().into_inner()[0].clone();
		assert_eq!(first_bid.bids_count, 4);
		assert_eq!(first_bid.bid_amount_min, 1_000_000);
		assert_eq!(first_bid.bid_amount_max, 2_010_000);
		assert_eq!(first_bid.bid_amount_sum, 7_010_000);

		System::assert_has_event(
			Event::SlotBidderDropped { account_id: 3, preserved_argonot_hold: false }.into(),
		);

		System::assert_has_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 2_010_000, index: 0 }.into(),
		);
		assert_eq!(Ownership::free_balance(3), 100_500);

		assert_eq!(
			BidsForNextSlotCohort::<Test>::get()
				.iter()
				.map(|a| a.account_id)
				.collect::<Vec<_>>(),
			vec![1, 2]
		);
		assert!(System::account_exists(&1));
		assert!(System::account_exists(&2));
		assert!(System::account_exists(&3));

		System::reset_events();
		System::set_block_number(9);
		CurrentTick::set(9);
		ElapsedTicks::set(9);
		System::on_initialize(9);
		MiningSlots::on_initialize(9);
		MiningSlots::on_finalize(9);
		assert_eq!(Ownership::free_balance(3), 100_500);
		assert_eq!(Ownership::total_balance(&3), 100_500);
		assert_eq!(Balances::free_balance(3), 5_000_000);
		assert_eq!(
			Balances::free_balance(bid_pool_account_id),
			4_010_000,
			"only has the final bids"
		);
		System::assert_has_event(
			Event::NewMiners {
				frame_id: 2,
				new_miners: BoundedVec::truncate_from(vec![
					MiningRegistration {
						account_id: 1,
						argonots: 500u32.into(),
						bid: 2_010_000,
						authority_keys: 1.into(),
						starting_frame_id: 2,
						external_funding_account: None,
						bid_at_tick: 6,
					},
					MiningRegistration {
						account_id: 2,
						argonots: 500u32.into(),
						bid: 2_000_000,
						authority_keys: 2.into(),
						starting_frame_id: 2,
						external_funding_account: None,
						bid_at_tick: 6,
					},
				]),
				released_miners: 0,
			}
			.into(),
		);
	});
}

#[test]
fn it_handles_cleaning_up_xor_keys() {
	TicksBetweenSlots::set(10);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(2);
	ExistentialDeposit::set(100_000);

	new_test_ext().execute_with(|| {
		ElapsedTicks::set(1);
		CurrentTick::set(1);
		System::set_block_number(1);
		SlotBiddingStartAfterTicks::set(0);
		IsNextSlotBiddingOpen::<Test>::set(true);
		for i in 1..20 {
			set_ownership(i, 100_000u32.into());
			set_argons(i, 5_000_000u32.into());
			MiningSlots::on_initialize(i);
			MiningSlots::on_finalize(i);
			CurrentTick::set(1 + (i * TicksBetweenSlots::get()));
			ElapsedTicks::set(i * TicksBetweenSlots::get());
			assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(i), 0, 1.into(), None));
			System::set_block_number(i);
			System::initialize(&(i), &System::parent_hash(), &Default::default());
		}
		assert_eq!(
			MinersByCohort::<Test>::iter_keys().collect::<Vec<_>>().len(),
			10,
			"Should have 10 cohorts"
		);
		assert_eq!(MinerXorKeysByCohort::<Test>::get().len(), 10, "Should have 10 xor key cohorts");
	});
}

#[test]
fn handles_a_max_of_bids_per_block() {
	TicksBetweenSlots::set(1);
	FramesPerMiningTerm::set(2);
	MinCohortSize::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(4);
		ElapsedTicks::set(4);
		CurrentTick::set(4);
		MiningSlots::on_initialize(4);
		MiningSlots::on_finalize(4);
		SlotBiddingStartAfterTicks::set(0);
		IsNextSlotBiddingOpen::<Test>::set(true);

		for i in 1..5 {
			set_ownership(i, 1000u32.into());
		}

		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 0, 1.into(), None));
		assert_eq!(HistoricalBidsPerSlot::<Test>::get().into_inner()[0].clone().bids_count, 1);

		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 0u32.into(), index: 0 }.into(),
		);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), 0, 2.into(), None));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 2, bid_amount: 0u32.into(), index: 1 }.into(),
		);
		assert_noop!(
			MiningSlots::bid(RuntimeOrigin::signed(3), 0, 3.into(), None),
			Error::<Test>::BidTooLow,
		);
		// should not have changed
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 2, bid_amount: 0u32.into(), index: 1 }.into(),
		);
		assert_eq!(HistoricalBidsPerSlot::<Test>::get().into_inner()[0].clone().bids_count, 2);
		for i in 1..5 {
			assert!(System::account_exists(&i));
		}
	});
}

#[test]
fn it_allows_bids_from_an_external_funding_account() {
	TicksBetweenSlots::set(3);
	FramesPerMiningTerm::set(3);
	MinCohortSize::set(2);
	ExistentialDeposit::set(100_000);
	let bid_pool_account_id = BidPoolAccountId::get();

	new_test_ext().execute_with(|| {
		ElapsedTicks::set(6);
		CurrentTick::set(6);
		System::set_block_number(6);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(2), 0, 1.into(), None),
			Error::<Test>::SlotNotTakingBids
		);

		ArgonotsPerMiningSeat::<Test>::set(100_000);
		SlotBiddingStartAfterTicks::set(0);
		IsNextSlotBiddingOpen::<Test>::set(true);

		set_ownership(1, 300_000);
		set_argons(1, 5_100_000);

		System::set_block_number(1);

		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 1_000_000, 1.into(), Some(2)));
		assert_eq!(Ownership::balance_on_hold(&HoldReason::RegisterAsMiner.into(), &1), 100_000);
		assert_eq!(Balances::free_balance(bid_pool_account_id), 1_000_000);
		assert_eq!(Balances::free_balance(1), 4_100_000);

		// we bid as account 2
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 2, bid_amount: 1_000_000, index: 0 }.into(),
		);
		assert_eq!(
			BidsForNextSlotCohort::<Test>::get().to_vec(),
			vec![Registration::<Test> {
				account_id: 2,
				argonots: 100_000u32.into(),
				bid: 1_000_000u32.into(),
				authority_keys: 1.into(),
				starting_frame_id: 1,
				external_funding_account: Some(1),
				bid_at_tick: 6,
			}]
		);

		// bid again as account 3
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 2_000_000, 3.into(), Some(3)));
		assert_eq!(Ownership::balance_on_hold(&HoldReason::RegisterAsMiner.into(), &1), 200_000);
		assert_eq!(Balances::free_balance(bid_pool_account_id), 3_000_000);
		assert_eq!(Balances::free_balance(1), 2_100_000);
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 3, bid_amount: 2_000_000, index: 0 }.into(),
		);
		assert_eq!(
			BidsForNextSlotCohort::<Test>::get().to_vec(),
			vec![
				Registration::<Test> {
					account_id: 3,
					argonots: 100_000u32.into(),
					bid: 2_000_000u32.into(),
					authority_keys: 3.into(),
					starting_frame_id: 1,
					external_funding_account: Some(1),
					bid_at_tick: 6,
				},
				Registration::<Test> {
					account_id: 2,
					argonots: 100_000u32.into(),
					bid: 1_000_000u32.into(),
					authority_keys: 1.into(),
					starting_frame_id: 1,
					external_funding_account: Some(1),
					bid_at_tick: 6,
				}
			]
		);

		// overflow
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 2_000_000, 4.into(), Some(4)));
		assert_eq!(Ownership::balance_on_hold(&HoldReason::RegisterAsMiner.into(), &1), 200_000);
		assert_eq!(Balances::free_balance(bid_pool_account_id), 4_000_000);
		assert_eq!(Balances::free_balance(1), 1_100_000);

		// now close the bids and ensure funds go back to the right wallet

		ElapsedTicks::set(8);
		CurrentTick::set(8);
		System::set_block_number(8);
		MiningSlots::on_initialize(8);
		MiningSlots::on_finalize(8);
		assert_eq!(Balances::free_balance(bid_pool_account_id), 4_000_000);
		assert_eq!(Balances::free_balance(1), 1_100_000);
		assert_eq!(Ownership::balance_on_hold(&HoldReason::RegisterAsMiner.into(), &1), 200_000);
		assert_eq!(Ownership::free_balance(1), 100_000);
	});
}

#[test]
fn it_handles_null_authority() {
	new_test_ext().execute_with(|| {
		assert_eq!(MiningSlots::get_authority(1), None);
	});
}

#[test]
fn it_can_get_closest_authority() {
	MinCohortSize::set(100);
	MaxCohortSize::set(100);
	FramesPerMiningTerm::set(10);
	new_test_ext().execute_with(|| {
		System::set_block_number(8);
		ElapsedTicks::set(8);

		for frame_id in 1..=10u64 {
			for i in 0..10u32 {
				let account_id: u64 = (((frame_id - 1) * 10) as u32 + i).into();
				MinersByCohort::<Test>::mutate(frame_id, |x| {
					x.try_insert(
						i as usize,
						MiningRegistration {
							account_id,
							argonots: 0,
							bid: 0,
							authority_keys: account_id.into(),
							starting_frame_id: frame_id,
							external_funding_account: None,
							bid_at_tick: 1,
						},
					)
					.unwrap();
				});
				AccountIndexLookup::<Test>::insert(account_id, (frame_id, i));
				MinerXorKeysByCohort::<Test>::try_mutate(|a| {
					if !a.contains_key(&frame_id) {
						a.try_insert(frame_id, Default::default()).ok();
					}
					let hash = U256::from(account_id);
					a.get_mut(&frame_id).unwrap().try_insert(i as usize, hash)
				})
				.expect("Didn't insert authorities");
			}
		}

		assert_eq!(
			MiningSlots::xor_closest_authority(U256::from(100)),
			Some(MiningAuthority {
				account_id: 96,
				authority_id: UintAuthorityId(96),
				authority_index: (10, 6),
			})
		);
	});
}

#[test]
fn it_will_end_auctions_if_a_seal_qualifies() {
	TicksBetweenSlots::set(100);
	FramesPerMiningTerm::set(3);
	MinCohortSize::set(2);
	BlocksBeforeBidEndForVrfClose::set(10);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		System::set_block_number(89);
		CurrentTick::set(89);
		ElapsedTicks::set(89);

		IsNextSlotBiddingOpen::<Test>::set(true);

		// This seal strength was generated using the commented out loop below
		let seal_strength = U256::from_dec_str(
			"11579208923731619542357098500868790785326998466564056403945758400791312963992",
		)
		.expect("can read seal strength u256");

		assert!(!MiningSlots::check_for_bidding_close(seal_strength));

		// now we're the right block
		System::set_block_number(90);
		ElapsedTicks::set(90);
		CurrentTick::set(90);
		assert!(MiningSlots::check_for_bidding_close(seal_strength));

		let invalid_strength = U256::from_dec_str(
			"11579208923731619542357098500868790785326998466564056403945758400791312963993",
		)
		.unwrap();
		assert!(!MiningSlots::check_for_bidding_close(invalid_strength));

		let frame_id = NextFrameId::<Test>::get();
		System::assert_last_event(Event::MiningBidsClosed { frame_id }.into());

		if env::var("TEST_DISTRO").unwrap_or("false".to_string()) == "true" {
			let mut valid_seals = vec![];
			for _ in 0..10u32 {
				let seal_strength = U256::from_big_endian(H256::random().as_ref());

				if MiningSlots::check_for_bidding_close(seal_strength) {
					valid_seals.push(seal_strength);
				}
			}
			assert!(!valid_seals.is_empty(), "Should have found at least one valid seal");
			println!("Valid seals: {:?}", valid_seals);
		}
	})
}

fn bid_stats(count: u32, amount: Balance) -> MiningBidStats {
	MiningBidStats {
		bids_count: count,
		bid_amount_sum: amount * count as u128,
		bid_amount_min: amount,
		bid_amount_max: amount,
	}
}

#[test]
fn it_adjusts_locked_argonots() {
	TicksBetweenSlots::set(10);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(10);
	MaxCohortSize::set(10);
	SlotBiddingStartAfterTicks::set(10);
	TargetBidsPerSlot::set(12);
	MinOwnershipBondAmount::set(100_000);

	new_test_ext().execute_with(|| {
		System::set_block_number(10);
		ElapsedTicks::set(10);

		Ownership::set_total_issuance(500_000 * 100);
		ArgonotsPerMiningSeat::<Test>::set(120_000);
		// should have 10 per slot, make it 12
		HistoricalBidsPerSlot::<Test>::set(bounded_vec![bid_stats(12, 10), bid_stats(12, 10)]);
		MiningSlots::adjust_argonots_per_seat();

		// we're targeting 12 bids per slot, so should stay the same
		assert_eq!(ArgonotsPerMiningSeat::<Test>::get(), 120_000);

		// simulate bids being past 20%
		HistoricalBidsPerSlot::<Test>::set(bounded_vec![
			bid_stats(20, 100),
			bid_stats(20, 101),
			bid_stats(20, 102)
		]);
		MiningSlots::adjust_argonots_per_seat();
		// 120k * 1.2
		assert_eq!(ArgonotsPerMiningSeat::<Test>::get(), 144000);

		// simulate bids being way past 20%
		// should have 10 per slot, make it 12
		HistoricalBidsPerSlot::<Test>::set(bounded_vec![
			bid_stats(0, 0),
			bid_stats(1, 100),
			bid_stats(0, 0)
		]);
		MiningSlots::adjust_argonots_per_seat();
		assert_eq!(ArgonotsPerMiningSeat::<Test>::get(), (144000.0 * 0.8f64) as u128);

		// simulate bids being way past 20%
		let mut last = ArgonotsPerMiningSeat::<Test>::get();
		for _ in 0..100 {
			HistoricalBidsPerSlot::<Test>::set(bounded_vec![
				bid_stats(100, 0),
				bid_stats(100, 100),
				bid_stats(100, 0)
			]);
			MiningSlots::adjust_argonots_per_seat();
			let next = ArgonotsPerMiningSeat::<Test>::get();
			if next == 400_000 {
				break;
			}
			assert_eq!(next, (last as f64 * 1.2) as u128);
			last = next;
		}

		// max increase is to a set amount of the total issuance
		assert_eq!(ArgonotsPerMiningSeat::<Test>::get(), (500_000.0 * 0.8) as u128);
	});
}

#[test]
fn it_adjusts_mining_seats() {
	TicksBetweenSlots::set(10);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(10);
	MaxCohortSize::set(1000);
	SlotBiddingStartAfterTicks::set(10);
	TargetPricePerSeat::set(100 * 1_000_000);

	new_test_ext().execute_with(|| {
		System::set_block_number(10);
		ArgonotsPerMiningSeat::<Test>::set(100_000);
		IsNextSlotBiddingOpen::<Test>::set(true);
		// submit 10 bids
		for i in 0..10 {
			set_ownership(i, 100_000);
			set_argons(i, 111_000_000);
			assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(i), 110 * 1_000_000, 1.into(), None));
		}

		ElapsedTicks::set(20);
		CurrentTick::set(20);
		MiningSlots::on_initialize(11);
		MiningSlots::on_finalize(11);
		assert_eq!(NextFrameId::<Test>::get(), 2);
		assert_eq!(AveragePricePerSeat::<Test>::get().to_vec(), vec![110 * 1_000_000]);
		assert_eq!(NextCohortSize::<Test>::get(), 11);

		for frame in 1..=11 {
			for i in 0..10 {
				let account = (((frame) * 10) as u32 + i).into();
				set_ownership(account, 100_000);
				set_argons(account, 111_000_000);
				assert_ok!(MiningSlots::bid(
					RuntimeOrigin::signed(account),
					110 * 1_000_000,
					1.into(),
					None
				));
			}
			ElapsedTicks::set(20 + frame * 10);
			CurrentTick::set(20 + frame * 10);
			MiningSlots::on_initialize(20 + frame * 10);
			MiningSlots::on_finalize(20 + frame * 10);
			assert_eq!(NextFrameId::<Test>::get(), frame + 2);
		}
		assert_eq!(NextCohortSize::<Test>::get(), 11 + 13);
		assert_eq!(AveragePricePerSeat::<Test>::get().len(), 10);
	});
}

#[test]
fn it_doesnt_accept_bids_until_first_slot() {
	TicksBetweenSlots::set(1440);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(10);
	MaxCohortSize::set(10);
	SlotBiddingStartAfterTicks::set(12_960);
	TargetBidsPerSlot::set(12);
	MinOwnershipBondAmount::set(100_000);

	new_test_ext().execute_with(|| {
		let argonots_per_seat = 1_000;
		// use this test to ensure we can hold the entire ownership balance
		set_ownership(2, argonots_per_seat + ExistentialDeposit::get());
		ArgonotsPerMiningSeat::<Test>::set(argonots_per_seat);

		System::set_block_number(1);
		ElapsedTicks::set(12959);

		MiningSlots::on_initialize(1);
		MiningSlots::on_finalize(1);
		assert!(!IsNextSlotBiddingOpen::<Test>::get());

		// bidding will start on the first (block % 1440 == 0)
		ElapsedTicks::set(12960);
		MiningSlots::on_initialize(12960);
		MiningSlots::on_finalize(12960);
		assert_eq!(NextFrameId::<Test>::get(), 1);
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), 0, 1.into(), None));
		assert_eq!(ActiveMinersCount::<Test>::get(), 0);

		let next_divisible_period = 12960 + 1440;
		System::set_block_number(next_divisible_period);
		ElapsedTicks::set(next_divisible_period);
		CurrentTick::set(next_divisible_period);
		MiningSlots::on_initialize(next_divisible_period);
		MiningSlots::on_finalize(next_divisible_period);
		assert_eq!(NextFrameId::<Test>::get(), 2);
		assert_eq!(FrameStartBlockNumbers::<Test>::get().to_vec(), vec![next_divisible_period]);
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		assert_eq!(ActiveMinersCount::<Test>::get(), 1);
	});
}

#[test]
fn it_can_change_the_compute_mining_block() {
	SlotBiddingStartAfterTicks::set(12_960);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ElapsedTicks::set(1);
		MiningSlots::on_initialize(1);
		MiningSlots::on_finalize(1);
		let starting_config = MiningConfig::<Test>::get();
		assert_ok!(MiningSlots::configure_mining_slot_delay(
			RuntimeOrigin::root(),
			Some(12961),
			Some(15)
		));
		assert_eq!(MiningConfig::<Test>::get().slot_bidding_start_after_ticks, 12961);
		System::assert_last_event(
			Event::MiningConfigurationUpdated {
				slot_bidding_start_after_ticks: 12961,
				ticks_between_slots: starting_config.ticks_between_slots,
				ticks_before_bid_end_for_vrf_close: 15,
			}
			.into(),
		);
	});
}

#[test]
fn it_should_rotate_grandpas() {
	SlotBiddingStartAfterTicks::set(12_960 - 1);
	GrandpaRotationFrequency::set(4);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ElapsedTicks::set(1);
		CurrentTick::set(1);
		NextFrameId::<Test>::set(1);
		MiningSlots::on_initialize(1);
		MiningSlots::on_finalize(1);
		assert_eq!(NextFrameId::<Test>::get(), 1);
		// genesis rotate
		assert_eq!(GrandaRotations::get(), vec![0]);

		System::set_block_number(4);
		ElapsedTicks::set(4);
		CurrentTick::set(4);
		MiningSlots::on_initialize(4);
		MiningSlots::on_finalize(4);
		assert_eq!(GrandaRotations::get(), vec![0, 0]);

		// should auto-increment and rotate
		System::set_block_number(12_960);
		ElapsedTicks::set(12_960);
		CurrentTick::set(12_960);
		MiningSlots::on_initialize(12_960);
		MiningSlots::on_finalize(12_960);
		assert_eq!(GrandaRotations::get(), vec![0, 0, 1]);
	});
}

#[test]
fn it_allows_a_bidder_to_use_their_full_balance() {
	ExistentialDeposit::set(10_000);
	new_test_ext().execute_with(|| {
		set_argons(1, 1_000_000);
		set_ownership(1, 1_000_000);

		IsNextSlotBiddingOpen::<Test>::set(true);
		ArgonotsPerMiningSeat::<Test>::set(10_000);
		// ### Use most of balance
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 900_000, 1.into(), None));

		assert_eq!(Balances::free_balance(1), 100_000);
		assert_eq!(System::providers(&1), 3);

		// Can send the rest
		assert_ok!(Balances::transfer_allow_death(RuntimeOrigin::signed(1), 2, 100_000));
		assert!(System::account_exists(&1));

		set_argons(2, 1_000_000);
		set_ownership(2, 10_000);
		System::inc_account_nonce(2);
		// ### Use all of argonots and balance
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), 1_000_000, 2.into(), None));
		assert_eq!(Balances::free_balance(2), 0);
		assert_eq!(System::providers(&2), 1);
		assert!(System::account_exists(&2));
		assert_eq!(System::account_nonce(2), 1);

		// verify account is still here
		assert_eq!(
			System::account(2),
			AccountInfo {
				nonce: 1,
				consumers: 1,
				providers: 1,
				sufficients: 0,
				data: AccountData { free: 0, reserved: 0, flags: ExtraFlags::default(), frozen: 0 },
			}
		);
		assert_eq!(Ownership::free_balance(2), 0);
		assert_eq!(Ownership::total_balance(&2), 10_000);
		assert_eq!(MiningSlots::bid_pool_balance(), 1_900_000);
	});
}
