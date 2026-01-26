use crate::{
	Error, Event, FrameRewardTicksRemaining, FrameStartTicks, HoldReason, MinerNonce,
	MinerNonceScoring, Registration, ScheduledCohortSizeChangeByFrame,
	mock::{MiningSlots, Ownership, *},
	pallet::{
		AccountIndexLookup, ActiveMinersCount, ArgonotsPerMiningSeat, AveragePricePerSeat,
		BidsForNextSlotCohort, FrameStartBlockNumbers, HistoricalBidsPerSlot,
		IsNextSlotBiddingOpen, MinerNonceScoringByCohort, MinersByCohort, MiningConfig,
		NextCohortSize, NextFrameId,
	},
};
use argon_primitives::{
	AuthorityProvider,
	block_seal::{MiningBidStats, MiningRegistration},
};
use frame_support::traits::fungible::Unbalanced;
use frame_system::AccountInfo;
use pallet_balances::{AccountData, Event as OwnershipEvent, ExtraFlags};
use pallet_prelude::{
	argon_primitives::{FRAME_INFO_DIGEST, FrameInfo},
	sp_core::Pair,
	*,
};
use polkadot_sdk::sp_core::bounded_btree_map;
use sp_core::bounded_vec;
use std::{collections::HashMap, env};

#[test]
#[should_panic]
fn it_should_create_and_validate_a_digest() {
	new_test_ext().execute_with(|| {
		System::initialize(
			&1,
			&System::parent_hash(),
			&Digest {
				logs: vec![DigestItem::Consensus(
					FRAME_INFO_DIGEST,
					FrameInfo { frame_id: 1, is_new_frame: true, frame_reward_ticks_remaining: 10 }
						.encode(),
				)],
			},
		);

		MiningSlots::on_initialize(1);
		MiningSlots::on_finalize(1);
	});
}

#[test]
fn it_should_validate_the_digest() {
	new_test_ext().execute_with(|| {
		FrameRewardTicksRemaining::<Test>::set(1);
		NextFrameId::<Test>::set(2);
		IsNextSlotBiddingOpen::<Test>::set(true);
		ElapsedTicks::set(SlotBiddingStartAfterTicks::get() + TicksBetweenSlots::get());
		CurrentTick::set(SlotBiddingStartAfterTicks::get() + TicksBetweenSlots::get());
		System::initialize(
			&1,
			&System::parent_hash(),
			&Digest {
				logs: vec![DigestItem::Consensus(
					FRAME_INFO_DIGEST,
					FrameInfo {
						frame_id: 2,
						is_new_frame: true,
						frame_reward_ticks_remaining: TicksBetweenSlots::get() as u32,
					}
					.encode(),
				)],
			},
		);

		MiningSlots::on_initialize(1);
		MiningSlots::on_finalize(1);
	});
}

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

		System::initialize(&2, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(2);
		MiningSlots::on_finalize(2);
		assert!(IsNextSlotBiddingOpen::<Test>::get());

		assert_eq!(MiningSlots::frame_1_begins_tick(), current_tick + 1440);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 0, 1.into(), None));
		MiningConfig::<Test>::mutate(|a| {
			a.slot_bidding_start_after_ticks = 21_000;
		});
		System::initialize(&3, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(3);
		MiningSlots::on_finalize(3);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), 0, 2.into(), None));

		current_tick += 1440;
		ElapsedTicks::set(current_tick - genesis);
		CurrentTick::set(current_tick);
		System::initialize(&4, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(4);
		MiningSlots::on_finalize(4);
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		// now it's bumped out
		assert_eq!(MiningSlots::frame_1_begins_tick(), current_tick + 1000);
		assert_eq!(FrameRewardTicksRemaining::<Test>::get(), 1440);
		assert_eq!(NextFrameId::<Test>::get(), 1);

		current_tick += 1000;
		ElapsedTicks::set(current_tick - genesis);
		CurrentTick::set(current_tick);
		System::initialize(&5, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(5);
		MiningSlots::on_finalize(5);
		assert_eq!(MiningSlots::frame_1_begins_tick(), current_tick);
		assert_eq!(NextFrameId::<Test>::get(), 2,);
		assert_eq!(
			MiningSlots::get_next_mining_epoch(),
			(current_tick + 1440, current_tick + 1440 + mining_window)
		);
		assert_eq!(MiningSlots::get_next_frame_tick(), current_tick + 1440);
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

		CurrentTick::set(8);
		FrameRewardTicksRemaining::<Test>::put(1);
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
				MinerNonceScoringByCohort::<Test>::try_mutate(|x| {
					if !x.contains_key(&frame_id) {
						x.try_insert(frame_id, BoundedVec::default()).unwrap();
					}
					x.get_mut(&frame_id).unwrap().try_insert(
						index,
						MinerNonceScoring {
							nonce: MinerNonce::<Test> {
								account_id,
								block_hash: System::parent_hash(),
							}
							.generate(),
							blocks_won_in_frame: 0,
							last_win_block: None,
							frame_start_blocks_won_surplus: 0,
						},
					)
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

		IsBlockVoteSeal::set(true);
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
		let authority_hash_frames = MinerNonceScoringByCohort::<Test>::get().into_inner();
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
				hashes.iter().enumerate().map(move |(i, hash)| ((frame_id, i), hash.nonce))
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
fn it_releases_argonots_when_an_epoch_ends() {
	TicksBetweenSlots::set(2);
	FramesPerMiningTerm::set(3);
	MinCohortSize::set(2);
	MaxCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		ArgonotsPerMiningSeat::<Test>::set(1000);
		CurrentTick::set(7);

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
		FrameRewardTicksRemaining::<Test>::set(1);
		assert_eq!(MiningSlots::get_next_mining_epoch(), (8, 8 + (2 * 3)));

		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), 0, 1.into(), None));

		CurrentTick::set(8);
		IsBlockVoteSeal::set(true);
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
			System::initialize(&i, &System::parent_hash(), &Default::default());

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
		System::initialize(
			&(12 + TicksBetweenSlots::get()),
			&System::parent_hash(),
			&Default::default(),
		);
		MiningSlots::on_initialize(12 + TicksBetweenSlots::get());
		MiningSlots::on_finalize(12 + TicksBetweenSlots::get());

		assert!(IsNextSlotBiddingOpen::<Test>::get(), "bidding should now be open");
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), 0, 1.into(), None));
	});
}
#[test]
fn it_should_decrement_reward_ticks_for_frames() {
	TicksBetweenSlots::set(4);
	FramesPerMiningTerm::set(3);
	MinCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		CurrentTick::set(12);
		System::set_block_number(12);
		FrameRewardTicksRemaining::<Test>::set(10);
		IsBlockVoteSeal::set(false);
		System::initialize(&12, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(12);
		MiningSlots::on_finalize(12);
		assert_eq!(FrameRewardTicksRemaining::<Test>::get(), 10);

		IsBlockVoteSeal::set(true);
		System::initialize(&13, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(13);
		MiningSlots::on_finalize(13);
		assert_eq!(FrameRewardTicksRemaining::<Test>::get(), 9);

		// if there are no miners registered, we need to move forward with the frame
		IsBlockVoteSeal::set(false);
		CurrentTick::set(16);
		NextFrameId::<Test>::set(2);
		System::initialize(&16, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(16);
		MiningSlots::on_finalize(16);
		assert_eq!(FrameRewardTicksRemaining::<Test>::get(), 8);
	});
}

#[test]
fn it_wont_let_you_reuse_ownership_tokens_for_two_bids() {
	TicksBetweenSlots::set(4);
	FramesPerMiningTerm::set(3);
	MinCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		CurrentTick::set(12);
		System::set_block_number(12);

		IsNextSlotBiddingOpen::<Test>::set(true);
		NextFrameId::<Test>::put(4);

		assert_eq!(MiningSlots::get_next_mining_epoch(), (16, 16 + (4 * 3)));
		set_ownership(2, 100u32.into());
		set_ownership(1, 100u32.into());
		System::initialize(&12, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(12);
		MiningSlots::on_finalize(12);

		let ownership = (200 / 6) as u128;
		ArgonotsPerMiningSeat::<Test>::set(ownership);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 0, 1.into(), None));
		System::set_block_number(16);
		CurrentTick::set(16);
		IsBlockVoteSeal::set(true);
		FrameRewardTicksRemaining::<Test>::set(1);
		System::initialize(&16, &System::parent_hash(), &Default::default());
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
		assert_eq!(FrameStartTicks::<Test>::get().len(), 1);
		assert_eq!(FrameStartTicks::<Test>::get().get(&4), Some(&16));
		assert_eq!(MiningSlots::get_next_mining_epoch(), (20, 20 + (4 * 3)));

		CurrentTick::set(20);
		System::set_block_number(20);

		FrameRewardTicksRemaining::<Test>::set(1);
		System::initialize(&20, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(20);
		MiningSlots::on_finalize(20);
		assert_eq!(MiningSlots::get_next_mining_epoch(), (24, 24 + (4 * 3)));

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

		IsBlockVoteSeal::set(true);
		FrameRewardTicksRemaining::<Test>::set(1);
		System::initialize(&6, &System::parent_hash(), &Default::default());
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

		FrameRewardTicksRemaining::<Test>::set(1);
		System::initialize(&9, &System::parent_hash(), &Default::default());
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
fn it_handles_cleaning_up_miner_nonces() {
	TicksBetweenSlots::set(10);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(2);
	ExistentialDeposit::set(100_000);

	new_test_ext().execute_with(|| {
		CurrentTick::set(1);
		System::set_block_number(1);
		SlotBiddingStartAfterTicks::set(0);
		IsNextSlotBiddingOpen::<Test>::set(true);
		IsBlockVoteSeal::set(true);
		for i in 1..20 {
			set_ownership(i, 100_000u32.into());
			set_argons(i, 5_000_000u32.into());
			MiningSlots::on_initialize(i);
			MiningSlots::on_finalize(i);

			assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(i), 0, 1.into(), None));
			System::set_block_number(i);
			System::initialize(&(i), &System::parent_hash(), &Default::default());
			FrameRewardTicksRemaining::<Test>::set(1);
		}
		assert_eq!(
			MinersByCohort::<Test>::iter_keys().collect::<Vec<_>>().len(),
			10,
			"Should have 10 cohorts"
		);
		assert_eq!(
			MinerNonceScoringByCohort::<Test>::get().len(),
			10,
			"Should have 10 xor key cohorts"
		);
	});
}

#[test]
fn handles_a_max_of_bids_per_block() {
	TicksBetweenSlots::set(1);
	FramesPerMiningTerm::set(2);
	MinCohortSize::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(4);

		CurrentTick::set(4);
		IsBlockVoteSeal::set(true);
		FrameRewardTicksRemaining::<Test>::set(1);
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

		CurrentTick::set(8);
		System::set_block_number(8);
		FrameRewardTicksRemaining::<Test>::set(1);
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
fn it_will_end_auctions_if_a_seal_qualifies() {
	TicksBetweenSlots::set(100);
	FramesPerMiningTerm::set(3);
	MinCohortSize::set(2);
	BlocksBeforeBidEndForVrfClose::set(10);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		System::set_block_number(89);
		CurrentTick::set(89);

		IsNextSlotBiddingOpen::<Test>::set(true);

		// This seal strength was generated using the commented out loop below
		let seal_strength = U256::from_dec_str(
			"11579208923731619542357098500868790785326998466564056403945758400791312963992",
		)
		.expect("can read seal strength u256");

		assert!(!MiningSlots::check_for_bidding_close(seal_strength));

		// now we're the right block
		System::set_block_number(90);
		FrameRewardTicksRemaining::<Test>::set(10);
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

#[test]
fn it_should_allow_each_miner_to_get_full_rewards() {
	TicksBetweenSlots::set(10);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(10);
	MaxCohortSize::set(100);
	BlocksBeforeBidEndForVrfClose::set(0);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		let mut current_tick = 1;
		CurrentTick::set(current_tick);
		System::set_block_number(1);
		System::initialize(&1, &System::parent_hash(), &Default::default());
		set_argons(1, 5_000_000);
		set_ownership(1, 100_000u32.into());
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 1_000_000, 1.into(), None));
		current_tick += TicksBetweenSlots::get();

		CurrentTick::set(current_tick);
		ElapsedTicks::set(current_tick);
		System::set_block_number(2);
		System::initialize(&2, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(2);
		MiningSlots::on_finalize(2);
		assert_eq!(NextFrameId::<Test>::get(), 2);
		assert_eq!(ActiveMinersCount::<Test>::get(), 1);

		// create 9 vote blocks
		for i in 1..10 {
			IsBlockVoteSeal::set(true);
			current_tick += 1;
			CurrentTick::set(current_tick);

			System::initialize(&i, &System::parent_hash(), &Default::default());
			MiningSlots::on_initialize(i);
			MiningSlots::on_finalize(i);
		}
		assert_eq!(NextFrameId::<Test>::get(), 2);
		assert_eq!(FrameRewardTicksRemaining::<Test>::get(), 1);

		// if next vote is not vote block, it should keep vote blocks the same
		IsBlockVoteSeal::set(false);
		current_tick += 1;
		CurrentTick::set(current_tick);
		System::initialize(&10, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(10);
		MiningSlots::on_finalize(10);
		assert_eq!(NextFrameId::<Test>::get(), 2);
		assert_eq!(FrameRewardTicksRemaining::<Test>::get(), 1);

		// now final vote block should move to next frame
		IsBlockVoteSeal::set(true);
		current_tick += 1;
		CurrentTick::set(current_tick);
		System::initialize(&11, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(11);
		MiningSlots::on_finalize(11);
		assert_eq!(NextFrameId::<Test>::get(), 3);
		assert_eq!(FrameRewardTicksRemaining::<Test>::get(), 10);
	});
}

#[test]
fn it_distributes_seals_evenly() {
	TicksBetweenSlots::set(100);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(10);
	MaxCohortSize::set(100);
	BlocksBeforeBidEndForVrfClose::set(0);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		let mut current_tick = 1;
		CurrentTick::set(current_tick);

		for i in 1..=10 {
			System::set_block_number(i);
			System::initialize(&i, &System::parent_hash(), &Default::default());
			for x in 1..=10u64 {
				let id = (10 * i) + x;
				set_argons(id, 5_000_000);
				set_ownership(id, 100_000u32.into());
				assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(id), 1_000_000, 1.into(), None));
			}
			current_tick += TicksBetweenSlots::get();
			IsBlockVoteSeal::set(true);
			FrameRewardTicksRemaining::<Test>::set(1);
			CurrentTick::set(current_tick);

			MiningSlots::on_initialize(i);
			MiningSlots::on_finalize(i);
		}

		let miner_nonces = MinerNonceScoringByCohort::<Test>::get();
		let all_nonces: Vec<U256> = miner_nonces
			.iter()
			.flat_map(|(_, keys)| keys.iter().map(|key| key.nonce).collect::<Vec<U256>>())
			.collect();
		assert_eq!(all_nonces.len(), 100);

		let (notary_pair, _) = sp_core::sr25519::Pair::generate();
		let notary_account = AccountId::from(notary_pair.public());
		let mut winners_by_id = HashMap::<u64, u16>::new();
		let start_block_number = System::block_number();
		for i in 1..=10_000u64 {
			current_tick += 1;
			System::set_block_number(start_block_number + i);
			System::initialize(
				&(start_block_number + i),
				&System::parent_hash(),
				&Default::default(),
			);
			CurrentTick::set(current_tick);

			FrameRewardTicksRemaining::<Test>::mutate(|x| x.saturating_reduce(1));

			let vote = BlockVote::create_default_vote(notary_account.clone(), current_tick);
			let vote_bytes = vote.encode();
			let seal_proof = BlockVote::calculate_seal_proof(vote_bytes.clone(), 1, H256::random());
			let closest_miner = MiningSlots::get_winning_managed_authority(seal_proof, None, None);
			if let Some((closest, _, _)) = closest_miner {
				*winners_by_id.entry(closest.account_id).or_insert(0) += 1;
				MiningSlots::record_block_author(closest.account_id);
			} else {
				panic!("Should have found a closest miner");
			}
			if FrameRewardTicksRemaining::<Test>::get() == 0 {
				FrameRewardTicksRemaining::<Test>::set(100);
				NextFrameId::<Test>::mutate(|a| *a += 1);

				MiningSlots::reset_miner_nonce_scoring();
			}
		}
		println!("{:#?}", winners_by_id);
		assert_eq!(winners_by_id.len(), 100, "Should have 100 unique winning miners");
		let expected_wins_per_miner = 10_000f64 / 100f64;
		let mut max_diff = 0f64;
		let mut sum = 0;
		for (account, wins) in winners_by_id.iter() {
			let diff = (*wins as f64 - expected_wins_per_miner).abs();
			let diff_percent = diff / expected_wins_per_miner;
			if diff_percent.abs() > max_diff.abs() {
				max_diff = diff_percent;
			}
			assert!(
				diff_percent.abs() <= 0.05,
				"Account {:?} had {} wins which is more than 5% different from expected {}",
				account,
				wins,
				expected_wins_per_miner
			);
			sum += wins;
		}
		println!("Max difference from expected wins: {}%", max_diff * 100f64);
		assert_eq!(sum, 10_000, "Should have 10,000 total wins recorded");
	});
}

#[test]
fn it_should_track_latest_10_frame_ticks() {
	TicksBetweenSlots::set(100);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(10);
	MaxCohortSize::set(100);
	BlocksBeforeBidEndForVrfClose::set(0);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		let mut current_tick = 1;
		CurrentTick::set(current_tick);
		NextFrameId::<Test>::set(2);

		for i in 1..12 {
			current_tick += TicksBetweenSlots::get();
			IsBlockVoteSeal::set(true);
			FrameRewardTicksRemaining::<Test>::set(1);
			CurrentTick::set(current_tick);

			System::initialize(&i, &System::parent_hash(), &Default::default());
			MiningSlots::on_initialize(i);
			MiningSlots::on_finalize(i);
		}

		assert_eq!(NextFrameId::<Test>::get(), 13);
		assert_eq!(FrameStartTicks::<Test>::get().len(), 10);
		let frame_ids = FrameStartTicks::<Test>::get().keys().copied().collect::<Vec<_>>();
		assert_eq!(frame_ids, vec![3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
	});
}

#[test]
fn it_should_yield_results_when_highly_negative() {
	TicksBetweenSlots::set(10);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		System::set_block_number(14);

		IsNextSlotBiddingOpen::<Test>::set(true);
		ActiveMinersCount::<Test>::put(3);
		MinersByCohort::<Test>::mutate(1, |x| {
			let _ = x.try_push(MiningRegistration {
				account_id: 1,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 1.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
			let _ = x.try_push(MiningRegistration {
				account_id: 2,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 2.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
			let _ = x.try_push(MiningRegistration {
				account_id: 3,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 3.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
		});
		AccountIndexLookup::<Test>::insert(1, (1, 0));
		AccountIndexLookup::<Test>::insert(2, (1, 1));
		AccountIndexLookup::<Test>::insert(3, (1, 2));
		MinerNonceScoringByCohort::<Test>::mutate(|x| {
			let _ = x.try_insert(
				1,
				bounded_vec![
					MinerNonceScoring {
						nonce: U256::from(100u32),
						blocks_won_in_frame: 20,
						last_win_block: Some(10),
						frame_start_blocks_won_surplus: 0
					},
					MinerNonceScoring {
						nonce: U256::from(101u32),
						blocks_won_in_frame: 25,
						last_win_block: Some(10),
						frame_start_blocks_won_surplus: 0
					},
					MinerNonceScoring {
						nonce: U256::from(102u32),
						blocks_won_in_frame: 18,
						last_win_block: Some(10),
						frame_start_blocks_won_surplus: 0
					},
				],
			);
		});

		let res1 = MiningSlots::get_winning_managed_authority(U256::from(141u32), None, None);
		assert!(res1.is_some(), "Should have a winning miner");
		assert_ne!(res1.clone().unwrap().1, U256::zero());
		assert_eq!(res1.unwrap().0.account_id, 3);
	});
}

#[test]
fn it_should_be_unpredictable_who_wins_next() {
	TicksBetweenSlots::set(10);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		System::set_block_number(14);

		IsNextSlotBiddingOpen::<Test>::set(true);
		ActiveMinersCount::<Test>::put(3);
		MinersByCohort::<Test>::mutate(1, |x| {
			let _ = x.try_push(MiningRegistration {
				account_id: 1,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 1.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
			let _ = x.try_push(MiningRegistration {
				account_id: 2,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 2.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
			let _ = x.try_push(MiningRegistration {
				account_id: 3,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 3.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
		});
		AccountIndexLookup::<Test>::insert(1, (1, 0));
		AccountIndexLookup::<Test>::insert(2, (1, 1));
		AccountIndexLookup::<Test>::insert(3, (1, 2));
		MinerNonceScoringByCohort::<Test>::mutate(|x| {
			let _ = x.try_insert(
				1,
				bounded_vec![
					MinerNonceScoring {
						nonce: U256::from(100u32),
						blocks_won_in_frame: 1,
						last_win_block: Some(10),
						frame_start_blocks_won_surplus: 0
					},
					MinerNonceScoring {
						nonce: U256::from(101u32),
						blocks_won_in_frame: 2,
						last_win_block: Some(11),
						frame_start_blocks_won_surplus: 0
					},
					MinerNonceScoring {
						nonce: U256::from(102u32),
						blocks_won_in_frame: 3,
						last_win_block: Some(12),
						frame_start_blocks_won_surplus: 0
					},
				],
			);
		});

		// should still be possible for anyone to win
		let mut winners = Vec::new();
		for i in 0..10 {
			let res =
				MiningSlots::get_winning_managed_authority(U256::from(140u32 + i), None, None);
			assert!(res.is_some(), "Should have a winning miner");
			assert_ne!(res.clone().unwrap().1, U256::zero());
			let account_id = res.unwrap().0.account_id;
			if !winners.contains(&account_id) {
				winners.push(account_id);
			}
			if winners.len() == 3 {
				break;
			}
		}
		assert_eq!(winners.len(), 3, "All miners should have won at least once");
	});
}

#[test]
fn it_should_not_be_able_to_capture_all_blocks_in_mining_deficit() {
	TicksBetweenSlots::set(10);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		System::set_block_number(14);

		IsNextSlotBiddingOpen::<Test>::set(true);
		ActiveMinersCount::<Test>::put(3);
		MinersByCohort::<Test>::mutate(1, |x| {
			let _ = x.try_push(MiningRegistration {
				account_id: 1,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 1.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
			let _ = x.try_push(MiningRegistration {
				account_id: 2,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 2.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
			let _ = x.try_push(MiningRegistration {
				account_id: 3,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 3.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
		});
		AccountIndexLookup::<Test>::insert(1, (1, 0));
		AccountIndexLookup::<Test>::insert(2, (1, 1));
		AccountIndexLookup::<Test>::insert(3, (1, 2));
		MinerNonceScoringByCohort::<Test>::mutate(|x| {
			let _ = x.try_insert(
				1,
				bounded_vec![
					MinerNonceScoring {
						nonce: U256::from(100u32),
						blocks_won_in_frame: 10,
						last_win_block: Some(10),
						frame_start_blocks_won_surplus: 0
					},
					MinerNonceScoring {
						nonce: U256::from(101u32),
						blocks_won_in_frame: 10,
						last_win_block: Some(11),
						frame_start_blocks_won_surplus: 0
					},
					MinerNonceScoring {
						nonce: U256::from(102u32),
						blocks_won_in_frame: 1,
						last_win_block: Some(1),
						frame_start_blocks_won_surplus: 0
					},
				],
			);
		});

		// should still be possible for anyone to win
		let mut winners = Vec::new();
		for i in 0..10 {
			let res =
				MiningSlots::get_winning_managed_authority(U256::from(140u32 + i), None, None);
			assert!(res.is_some(), "Should have a winning miner");
			assert_ne!(res.clone().unwrap().1, U256::zero());
			let account_id = res.unwrap().0.account_id;
			MiningSlots::record_block_author(account_id);
			if !winners.contains(&account_id) {
				winners.push(account_id);
			}
			if winners.len() == 3 {
				break;
			}
		}
		assert_eq!(winners.len(), 3, "All miners should have won at least once");
	});
}

#[test]
fn it_should_allow_a_tie() {
	TicksBetweenSlots::set(10);
	FramesPerMiningTerm::set(10);
	MinCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		System::set_block_number(14);

		IsNextSlotBiddingOpen::<Test>::set(true);
		ActiveMinersCount::<Test>::put(3);
		MinersByCohort::<Test>::mutate(1, |x| {
			let _ = x.try_push(MiningRegistration {
				account_id: 1,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 1.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
			let _ = x.try_push(MiningRegistration {
				account_id: 2,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 2.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
			let _ = x.try_push(MiningRegistration {
				account_id: 3,
				argonots: 100_000,
				bid: 1_000_000u32.into(),
				authority_keys: 3.into(),
				starting_frame_id: 1,
				external_funding_account: None,
				bid_at_tick: 10,
			});
		});
		AccountIndexLookup::<Test>::insert(1, (1, 0));
		AccountIndexLookup::<Test>::insert(2, (1, 1));
		AccountIndexLookup::<Test>::insert(3, (1, 2));
		MinerNonceScoringByCohort::<Test>::mutate(|x| {
			let _ = x.try_insert(
				1,
				bounded_vec![
					MinerNonceScoring {
						nonce: U256::from(100u32),
						blocks_won_in_frame: 1,
						last_win_block: Some(10),
						frame_start_blocks_won_surplus: 0
					},
					MinerNonceScoring {
						nonce: U256::from(101u32),
						blocks_won_in_frame: 1,
						last_win_block: Some(11),
						frame_start_blocks_won_surplus: 0
					},
					MinerNonceScoring {
						nonce: U256::from(102u32),
						blocks_won_in_frame: 1,
						last_win_block: Some(12),
						frame_start_blocks_won_surplus: 0
					},
				],
			);
		});

		let res1 = MiningSlots::get_winning_managed_authority(U256::from(150u32), None, None);
		assert!(res1.is_some(), "Should have a winning miner");
		let top_score = res1.unwrap().1;
		assert!(
			MiningSlots::get_winning_managed_authority(U256::from(150u32), None, Some(top_score))
				.is_none()
		);
		let res2 =
			MiningSlots::get_winning_managed_authority(U256::from(155u32), None, Some(top_score));
		assert!(res2.is_some(), "Should have a winning miner");
	});
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
	TargetBidsPerSeatPercent::set(FixedU128::from_rational(12, 10));
	MinOwnershipBondAmount::set(100_000);

	new_test_ext().execute_with(|| {
		System::set_block_number(10);

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
			if next == 200_000 {
				break;
			}
			assert_eq!(next, (last as f64 * 1.2) as u128);
			last = next;
		}

		// max increase is to a set amount of the total issuance
		assert_eq!(ArgonotsPerMiningSeat::<Test>::get(), (500_000.0 * 0.4) as u128);
	});
}

#[test]
fn it_tracks_the_block_rewards() {
	TicksBetweenSlots::set(10);
	FramesPerMiningTerm::set(10);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		NextFrameId::<Test>::set(2);
		CurrentTick::set(1);
		System::set_block_number(10);
		IsNextSlotBiddingOpen::<Test>::set(true);
		// submit 10 bids
		set_ownership(1, 100_000);
		set_argons(1, 111_000_000);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), 110 * 1_000_000, 1.into(), None));

		CurrentTick::set(20);
		IsBlockVoteSeal::set(true);
		FrameRewardTicksRemaining::<Test>::set(1);
		System::initialize(&11, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(11);
		MiningSlots::on_finalize(11);
		assert_eq!(NextFrameId::<Test>::get(), 3);
		IsBlockVoteSeal::set(true);
		System::initialize(&12, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(12);
		MiningSlots::on_finalize(12);

		let nonce_scoring = MinerNonceScoringByCohort::<Test>::get();
		assert_eq!(nonce_scoring.get(&2).unwrap()[0].blocks_won_in_frame, 1);
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

		CurrentTick::set(20);
		IsBlockVoteSeal::set(true);
		FrameRewardTicksRemaining::<Test>::set(1);
		System::initialize(&11, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(11);
		MiningSlots::on_finalize(11);
		assert_eq!(NextFrameId::<Test>::get(), 2);
		assert_eq!(AveragePricePerSeat::<Test>::get().to_vec(), vec![110 * 1_000_000]);
		assert_eq!(NextCohortSize::<Test>::get(), 10);
		let expected_frame: FrameId = NextFrameId::<Test>::get() as FrameId + 10u64;
		let expected_schedule: BoundedBTreeMap<FrameId, u32, ConstU32<11>> =
			bounded_btree_map! { expected_frame => 11 };
		assert_eq!(ScheduledCohortSizeChangeByFrame::<Test>::get(), expected_schedule);

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

			CurrentTick::set(20 + frame * 10);
			FrameRewardTicksRemaining::<Test>::set(1);
			System::initialize(&(20 + frame * 10), &System::parent_hash(), &Default::default());
			MiningSlots::on_initialize(20 + frame * 10);
			MiningSlots::on_finalize(20 + frame * 10);
			assert_eq!(NextFrameId::<Test>::get(), frame + 2);
		}
		// should have applied a single change now
		assert_eq!(NextFrameId::<Test>::get(), 13);
		assert_eq!(NextCohortSize::<Test>::get(), 11);
		assert!(!ScheduledCohortSizeChangeByFrame::<Test>::get().contains_key(&expected_frame));
		assert_eq!(ScheduledCohortSizeChangeByFrame::<Test>::get().len(), 11);
		assert_eq!(
			ScheduledCohortSizeChangeByFrame::<Test>::get().into_iter().next_back(),
			Some((23u32.into(), 11 + 13))
		);
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
	TargetBidsPerSeatPercent::set(FixedU128::from_rational(12, 10));
	MinOwnershipBondAmount::set(100_000);

	new_test_ext().execute_with(|| {
		let argonots_per_seat = 1_000;
		// use this test to ensure we can hold the entire ownership balance
		set_ownership(2, argonots_per_seat + ExistentialDeposit::get());
		ArgonotsPerMiningSeat::<Test>::set(argonots_per_seat);

		System::set_block_number(1);
		ElapsedTicks::set(12959);
		System::initialize(&1, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(1);
		MiningSlots::on_finalize(1);
		assert!(!IsNextSlotBiddingOpen::<Test>::get());

		// bidding will start on the first (block % 1440 == 0)
		ElapsedTicks::set(12960);
		System::initialize(&12960, &System::parent_hash(), &Default::default());
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
		System::initialize(&next_divisible_period, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(next_divisible_period);
		MiningSlots::on_finalize(next_divisible_period);
		assert_eq!(NextFrameId::<Test>::get(), 2);
		assert_eq!(FrameStartBlockNumbers::<Test>::get().to_vec(), vec![next_divisible_period]);
		assert_eq!(FrameStartTicks::<Test>::get().get(&1), Some(&next_divisible_period));
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		assert_eq!(ActiveMinersCount::<Test>::get(), 1);
	});
}

#[test]
fn it_can_change_the_compute_mining_block() {
	SlotBiddingStartAfterTicks::set(12_960);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

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
	SlotBiddingStartAfterTicks::set(100);
	GrandpaRotationFrequency::set(4);

	new_test_ext().execute_with(|| {
		ElapsedTicks::set(1);
		CurrentTick::set(1);
		NextFrameId::<Test>::set(1);
		MiningSlots::on_initialize(1);
		MiningSlots::on_finalize(1);
		assert_eq!(NextFrameId::<Test>::get(), 1);
		// genesis rotate
		assert_eq!(GrandaRotations::get(), vec![0]);

		ElapsedTicks::set(4);
		CurrentTick::set(4);
		System::initialize(&4, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(4);
		MiningSlots::on_finalize(4);
		assert_eq!(GrandaRotations::get(), vec![0, 0]);

		CurrentTick::set(100);
		ElapsedTicks::set(100);
		assert_eq!(MiningSlots::frame_1_begins_tick(), 100 + TicksBetweenSlots::get());
		assert_eq!(MiningSlots::get_newly_started_frame(), None);
		CurrentTick::set(100 + TicksBetweenSlots::get());
		ElapsedTicks::set(100 + TicksBetweenSlots::get());
		assert_eq!(MiningSlots::get_newly_started_frame(), Some(1));
		System::initialize(&5, &System::parent_hash(), &Default::default());
		MiningSlots::on_initialize(5);
		MiningSlots::on_finalize(5);
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
