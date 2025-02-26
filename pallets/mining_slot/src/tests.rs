use crate::{
	mock::{MiningSlots, Ownership, *},
	pallet::{
		AccountIndexLookup, ActiveMinersByIndex, ActiveMinersCount, ArgonotsPerMiningSeat,
		AuthorityHashByIndex, AuthorityIdToMinerId, HistoricalBidsPerSlot, IsNextSlotBiddingOpen,
		LastActivatedCohortId, MiningConfig, NextSlotCohort,
	},
	Error, Event, HoldReason, MiningSlotBid,
};
use argon_primitives::{
	bitcoin::OpaqueBitcoinXpub,
	block_seal::{
		MiningAuthority, MiningBidStats, MiningRegistration, RewardDestination, RewardSharing,
	},
	inherents::BlockSealInherent,
	vault::{BitcoinObligationProvider, FundType, ObligationExpiration, VaultTerms},
	AuthorityProvider, BlockRewardAccountsProvider, BlockVote, MerkleProof,
};
use bitcoin::{
	bip32::{ChildNumber, Xpriv, Xpub},
	key::Secp256k1,
};
use frame_support::{
	assert_err, assert_noop, assert_ok,
	traits::{
		fungible::{InspectHold, Unbalanced},
		Currency, OnInitialize,
	},
};
use k256::elliptic_curve::rand_core::{OsRng, RngCore};
use pallet_balances::Event as OwnershipEvent;
use sp_core::{blake2_256, bounded_vec, crypto::AccountId32, ByteArray, H256, U256};
use sp_runtime::{testing::UintAuthorityId, traits::Zero, BoundedVec, FixedU128};
use std::{collections::HashMap, env};

#[test]
fn it_doesnt_add_cohorts_until_time() {
	TicksBetweenSlots::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		NextSlotCohort::<Test>::set(bounded_vec![MiningRegistration {
			account_id: 1,
			obligation_id: None,
			argonots: 0,
			bonded_argons: 0,
			reward_destination: RewardDestination::Owner,
			reward_sharing: None,
			authority_keys: 1.into(),
			cohort_id: 1,
		}]);

		MiningSlots::on_initialize(1);

		assert_eq!(NextSlotCohort::<Test>::get().len(), 1);
		assert_eq!(NextSlotCohort::<Test>::get()[0].reward_destination, RewardDestination::Owner,);
	});
}

#[test]
fn get_validation_window_blocks() {
	MaxCohortSize::set(2);
	MaxMiners::set(10);
	TicksBetweenSlots::set(1);

	new_test_ext().execute_with(|| {
		assert_eq!(MiningSlots::get_mining_window_ticks(), 5);
	});

	MaxCohortSize::set(5);
	MaxMiners::set(10);
	TicksBetweenSlots::set(10);

	new_test_ext().execute_with(|| {
		assert_eq!(MiningSlots::get_mining_window_ticks(), 2 * 10);
	});
}

#[test]
fn calculate_cohort_id() {
	MaxCohortSize::set(10);
	MaxMiners::set(100);
	let ticks_between_slots = 1440;
	TicksBetweenSlots::set(ticks_between_slots);

	SlotBiddingStartAfterTicks::set(20_000);
	let mining_window = 14400;

	let genesis = 28_000_000;
	let mut current_tick = genesis + 19_999;
	CurrentTick::set(current_tick);

	new_test_ext().execute_with(|| {
		LastActivatedCohortId::<Test>::set(0);
		assert!(!IsNextSlotBiddingOpen::<Test>::get());
		ElapsedTicks::set(current_tick - genesis);
		// bidding should open at 20k
		MiningSlots::on_initialize(1);
		assert!(!IsNextSlotBiddingOpen::<Test>::get());
		// now go to 20k
		current_tick += 1;
		CurrentTick::set(current_tick);
		ElapsedTicks::set(current_tick - genesis);
		MiningSlots::on_initialize(2);
		assert!(IsNextSlotBiddingOpen::<Test>::get());

		current_tick += ticks_between_slots - 1;
		CurrentTick::set(current_tick);
		ElapsedTicks::set(current_tick - genesis);
		MiningSlots::on_initialize(3);
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		assert_eq!(MiningSlots::slot_1_tick(), current_tick + 1);
		assert_eq!(MiningSlots::get_next_slot_tick(), current_tick + 1);
		assert_eq!(MiningSlots::ticks_since_mining_start(), 0);
		assert_eq!(MiningSlots::calculate_cohort_id(), 0);
		assert_eq!(
			MiningSlots::get_next_slot_era(),
			(current_tick + 1, current_tick + 1 + mining_window)
		);

		current_tick += 1;
		ElapsedTicks::mutate(|a| *a += 1u64);
		CurrentTick::set(current_tick);
		MiningSlots::on_initialize(4);
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		assert_eq!(MiningSlots::slot_1_tick(), current_tick);
		assert_eq!(MiningSlots::ticks_since_mining_start(), 0);
		assert_eq!(MiningSlots::calculate_cohort_id(), 1);
		assert_eq!(
			LastActivatedCohortId::<Test>::get(),
			1,
			"if not set, will show the current era"
		);
		assert_eq!(
			MiningSlots::get_next_slot_era(),
			(current_tick + 1440, current_tick + 1440 + mining_window)
		);
		assert_eq!(MiningSlots::get_next_slot_tick(), current_tick + 1440);
	});
}

#[test]
fn extends_bidding_if_mining_slot_extends() {
	MaxCohortSize::set(10);
	MaxMiners::set(100);
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

		LastActivatedCohortId::<Test>::set(0);
		assert!(!IsNextSlotBiddingOpen::<Test>::get());
		// now go to 20k
		current_tick += 1;
		CurrentTick::set(current_tick);
		ElapsedTicks::set(current_tick - genesis);
		MiningSlots::on_initialize(2);
		assert!(IsNextSlotBiddingOpen::<Test>::get());

		assert_eq!(MiningSlots::slot_1_tick(), current_tick + 1440);
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			None,
			RewardDestination::Owner,
			1.into()
		));
		MiningConfig::<Test>::mutate(|a| {
			a.slot_bidding_start_after_ticks = 21_000;
		});
		MiningSlots::on_initialize(3);
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(2),
			None,
			RewardDestination::Owner,
			2.into()
		));

		current_tick += 1440;
		ElapsedTicks::set(current_tick - genesis);
		CurrentTick::set(current_tick);
		MiningSlots::on_initialize(4);
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		// now it's bumped out
		assert_eq!(MiningSlots::slot_1_tick(), current_tick + 1000);
		assert_eq!(MiningSlots::ticks_since_mining_start(), 0);
		assert_eq!(MiningSlots::calculate_cohort_id(), 0);
		assert_eq!(LastActivatedCohortId::<Test>::get(), 0);

		current_tick += 1000;
		ElapsedTicks::set(current_tick - genesis);
		CurrentTick::set(current_tick);
		MiningSlots::on_initialize(5);
		assert_eq!(MiningSlots::slot_1_tick(), current_tick);
		assert_eq!(LastActivatedCohortId::<Test>::get(), 1,);
		assert_eq!(
			MiningSlots::get_next_slot_era(),
			(current_tick + 1440, current_tick + 1440 + mining_window)
		);
		assert_eq!(MiningSlots::get_next_slot_tick(), current_tick + 1440);
	});
}

#[test]
fn starting_cohort_index() {
	let max_cohort_size = 3;
	let max_validators = 12;

	assert_eq!(MiningSlots::get_slot_starting_index(0, max_validators, max_cohort_size), 0);
	assert_eq!(MiningSlots::get_slot_starting_index(1, max_validators, max_cohort_size), 3);
	assert_eq!(MiningSlots::get_slot_starting_index(2, max_validators, max_cohort_size), 6);
	assert_eq!(MiningSlots::get_slot_starting_index(3, max_validators, max_cohort_size), 9);

	assert_eq!(MiningSlots::get_slot_starting_index(4, max_validators, max_cohort_size), 0);
}

#[test]
fn it_adds_new_cohorts_on_block() {
	TicksBetweenSlots::set(2);
	MaxMiners::set(6);
	MaxCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(8);
		ElapsedTicks::set(8);
		CurrentTick::set(8);
		assert_eq!(MiningSlots::slot_1_tick(), 4);
		assert_eq!(MiningSlots::calculate_cohort_id(), 3);
		LastActivatedCohortId::<Test>::put(3);

		for i in 0..4u32 {
			let account_id: u64 = (i + 4).into();
			ActiveMinersByIndex::<Test>::insert(
				i,
				MiningRegistration {
					account_id,
					obligation_id: None,
					argonots: 0,
					bonded_argons: 0,
					reward_destination: RewardDestination::Owner,
					reward_sharing: None,
					authority_keys: account_id.into(),
					cohort_id: i as u64 + 1,
				},
			);
			AccountIndexLookup::<Test>::insert(account_id, i);
			AuthorityHashByIndex::<Test>::try_mutate(|index| {
				let hash = blake2_256(&account_id.to_le_bytes());
				index.try_insert(i, U256::from_big_endian(&hash))
			})
			.unwrap();
		}
		ActiveMinersCount::<Test>::put(4);
		// filled indexes are [0, 1, 2, 3, _, _]

		let cohort = BoundedVec::truncate_from(vec![MiningRegistration {
			account_id: 1,
			obligation_id: None,
			argonots: 0,
			bonded_argons: 0,
			reward_destination: RewardDestination::Owner,
			reward_sharing: None,
			authority_keys: 1.into(),
			cohort_id: 5,
		}]);

		NextSlotCohort::<Test>::set(cohort.clone());

		CurrentTick::set(10);
		ElapsedTicks::set(10);
		assert_eq!(MiningSlots::calculate_cohort_id(), 4);
		// on 8, we're filling indexes 2 and 3 [0, 1, -> 2, -> 3, _, _]
		MiningSlots::on_initialize(10);
		assert_eq!(LastActivatedCohortId::<Test>::get(), 4);

		// re-fetch
		assert_eq!(
			ActiveMinersCount::<Test>::get(),
			3,
			"Should have 3 validators still after insertion"
		);
		let validators = ActiveMinersByIndex::<Test>::iter().collect::<HashMap<_, _>>();
		let authority_hashes = AuthorityHashByIndex::<Test>::get().into_inner();
		assert_eq!(
			NextSlotCohort::<Test>::get().len(),
			0,
			"Queued mining_slot for block 8 should be removed"
		);
		assert_eq!(authority_hashes.len(), 3, "Should have 3 authority hashes after insertion");
		assert_eq!(validators.len(), 3, "Should have 3 validators still after insertion");

		assert!(validators.contains_key(&2), "Should insert validator at index 2");
		assert!(!validators.contains_key(&3), "Should no longer have a validator at index 3");

		assert_eq!(
			AccountIndexLookup::<Test>::get(1),
			Some(2),
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
		assert!(!authority_hashes.contains_key(&3), "Should no longer have hash index 3");

		// check what was called from the events
		assert_eq!(
			LastSlotAdded::get().len(),
			1,
			"Should emit a new slot event for the new cohort"
		);
		assert_eq!(LastSlotRemoved::get().len(), 2);

		System::assert_last_event(
			Event::NewMiners {
				start_index: 2,
				new_miners: BoundedVec::truncate_from(cohort.to_vec()),
				cohort_id: LastActivatedCohortId::<Test>::get(),
			}
			.into(),
		)
	});
}

#[test]
fn it_releases_argonots_when_a_window_closes() {
	TicksBetweenSlots::set(2);
	MaxMiners::set(6);
	MaxCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		ArgonotsPerMiningSeat::<Test>::set(1000);
		LastActivatedCohortId::<Test>::put(1);
		CurrentTick::set(7);
		ElapsedTicks::set(7);
		assert_eq!(MiningSlots::slot_1_tick(), 2);
		assert_eq!(MiningSlots::calculate_cohort_id(), 3);
		LastActivatedCohortId::<Test>::set(3);
		System::set_block_number(7);

		for i in 0..4u32 {
			let account_id: u64 = i.into();
			set_ownership(account_id, 1000u32.into());
			set_argons(account_id, 10_000u32.into());

			let bond_amount = (1000u32 + i).into();
			let ownership_tokens = MiningSlots::hold_argonots(&account_id, None).ok().unwrap();

			ActiveMinersByIndex::<Test>::insert(
				i,
				MiningRegistration {
					account_id,
					obligation_id: None,
					argonots: ownership_tokens,
					bonded_argons: bond_amount,
					reward_destination: RewardDestination::Owner,
					reward_sharing: None,
					authority_keys: 1.into(),
					cohort_id: 1,
				},
			);
			AccountIndexLookup::<Test>::insert(account_id, i);
		}
		ActiveMinersCount::<Test>::put(4);
		IsNextSlotBiddingOpen::<Test>::set(true);
		assert_eq!(MiningSlots::get_next_slot_era(), (8, 8 + (2 * 3)));

		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(2),
			None,
			RewardDestination::Owner,
			1.into()
		));

		CurrentTick::set(8);
		ElapsedTicks::set(8);
		System::set_block_number(8);

		MiningSlots::on_initialize(8);

		System::assert_last_event(
			Event::NewMiners {
				cohort_id: LastActivatedCohortId::<Test>::get(),
				start_index: 2,
				new_miners: BoundedVec::truncate_from(vec![MiningRegistration {
					obligation_id: None,
					account_id: 2,
					argonots: 1000u32.into(),
					bonded_argons: 0,
					reward_destination: RewardDestination::Owner,
					reward_sharing: None,
					authority_keys: 1.into(),
					cohort_id: 4,
				}]),
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

		System::assert_has_event(
			Event::<Test>::ReleasedMinerSeat {
				account_id: 3,
				obligation_id: None,
				preserved_argonot_hold: false,
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
	MaxMiners::set(6);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(6);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(2), None, RewardDestination::Owner, 1.into()),
			Error::<Test>::SlotNotTakingBids
		);

		SlotBiddingStartAfterTicks::set(0);
		IsNextSlotBiddingOpen::<Test>::set(true);

		set_ownership(3, 5000u32.into());
		let share_amount = 5000 / 6;
		MiningSlots::on_initialize(6);
		ArgonotsPerMiningSeat::<Test>::set(share_amount);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(1), None, RewardDestination::Owner, 1.into()),
			Error::<Test>::InsufficientOwnershipTokens
		);

		set_ownership(1, 1000u32.into());

		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			None,
			RewardDestination::Owner,
			1.into()
		));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 0u32.into(), index: 0 }.into(),
		);
		assert_eq!(Ownership::free_balance(1), 1000 - share_amount);

		// should be able to re-register
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			None,
			RewardDestination::Owner,
			1.into()
		));
		assert_eq!(
			NextSlotCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
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
	MaxMiners::set(6);
	MaxCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(12);
	ElapsedTicks::set(11);

	new_test_ext().execute_with(|| {
		set_ownership(2, 100u32.into());
		for i in 1..11u64 {
			System::set_block_number(i);

			MiningSlots::on_initialize(i);
			assert_err!(
				MiningSlots::bid(
					RuntimeOrigin::signed(2),
					None,
					RewardDestination::Owner,
					1.into()
				),
				Error::<Test>::SlotNotTakingBids
			);
		}

		System::set_block_number(12 + TicksBetweenSlots::get());
		ElapsedTicks::set(12 + TicksBetweenSlots::get());
		CurrentTick::set(12 + TicksBetweenSlots::get());
		MiningSlots::on_initialize(12 + TicksBetweenSlots::get());

		assert!(IsNextSlotBiddingOpen::<Test>::get(), "bidding should now be open");
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(2),
			None,
			RewardDestination::Owner,
			1.into()
		));
	});
}
#[test]
fn it_wont_let_you_reuse_ownership_tokens_for_two_bids() {
	TicksBetweenSlots::set(4);
	MaxMiners::set(6);
	MaxCohortSize::set(2);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		ElapsedTicks::set(12);
		CurrentTick::set(12);
		System::set_block_number(12);

		IsNextSlotBiddingOpen::<Test>::set(true);
		LastActivatedCohortId::<Test>::put(MiningSlots::calculate_cohort_id());

		assert_eq!(MiningSlots::get_next_slot_era(), (16, 16 + (4 * 3)));
		assert_eq!(MiningSlots::get_next_slot_starting_index(), 2);
		set_ownership(2, 100u32.into());
		set_ownership(1, 100u32.into());
		MiningSlots::on_initialize(12);

		let ownership = (200 / 6) as u128;
		ArgonotsPerMiningSeat::<Test>::set(ownership);
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			None,
			RewardDestination::Owner,
			1.into()
		));
		System::set_block_number(16);
		CurrentTick::set(16);
		ElapsedTicks::set(16);
		MiningSlots::on_initialize(16);

		System::assert_last_event(
			Event::NewMiners {
				cohort_id: LastActivatedCohortId::<Test>::get(),
				start_index: 2,
				new_miners: BoundedVec::truncate_from(vec![MiningRegistration {
					account_id: 1,
					obligation_id: None,
					argonots: ownership,
					bonded_argons: 0,
					reward_destination: RewardDestination::Owner,
					reward_sharing: None,
					authority_keys: 1.into(),
					cohort_id: 4,
				}]),
			}
			.into(),
		);
		assert_eq!(MiningSlots::get_next_slot_era(), (20, 20 + (4 * 3)));
		assert_eq!(MiningSlots::get_next_slot_starting_index(), 4);

		CurrentTick::set(20);
		ElapsedTicks::set(20);
		System::set_block_number(20);
		MiningSlots::on_initialize(20);
		assert_eq!(MiningSlots::get_next_slot_era(), (24, 24 + (4 * 3)));
		assert_eq!(MiningSlots::get_next_slot_starting_index(), 0);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(1), None, RewardDestination::Owner, 1.into()),
			Error::<Test>::CannotRegisterOverlappingSessions
		);
		assert!(System::account_exists(&1));
		assert!(System::account_exists(&2));
	})
}

#[test]
fn it_will_order_bids_with_bonded_argons() {
	TicksBetweenSlots::set(3);
	MaxMiners::set(6);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		ElapsedTicks::set(6);
		CurrentTick::set(6);
		System::set_block_number(6);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(2), None, RewardDestination::Owner, 1.into()),
			Error::<Test>::SlotNotTakingBids
		);

		SlotBiddingStartAfterTicks::set(0);
		IsNextSlotBiddingOpen::<Test>::set(true);

		set_ownership(1, 1000u32.into());
		set_ownership(2, 1000u32.into());
		set_ownership(3, 1000u32.into());

		MiningSlots::on_initialize(6);
		let share_amount = 3000 / 6;
		ArgonotsPerMiningSeat::<Test>::set(share_amount);

		// 1. Account 1 bids
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			Some(MiningSlotBid { vault_id: 1, amount: 1000u32.into() }),
			RewardDestination::Owner,
			1.into()
		));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 1000u32.into(), index: 0 }.into(),
		);
		assert_eq!(Ownership::free_balance(1), 1000 - share_amount);
		let first_bid = HistoricalBidsPerSlot::<Test>::get().into_inner()[0].clone();
		assert_eq!(first_bid.bids_count, 1);
		assert_eq!(first_bid.bid_amount_min, 1000);
		assert_eq!(first_bid.bid_amount_max, 1000);
		assert_eq!(first_bid.bid_amount_sum, 1000);

		assert_eq!(Obligations::get().len(), 1);

		// 2. Account 2 bids highest and takes top slot

		// should be able to re-register
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(2),
			Some(MiningSlotBid { vault_id: 1, amount: 1001u32.into() }),
			RewardDestination::Owner,
			2.into()
		));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 2, bid_amount: 1001u32.into(), index: 0 }.into(),
		);

		let first_bid = HistoricalBidsPerSlot::<Test>::get().into_inner()[0].clone();
		assert_eq!(first_bid.bids_count, 2);
		assert_eq!(first_bid.bid_amount_min, 1000);
		assert_eq!(first_bid.bid_amount_max, 1001);
		assert_eq!(first_bid.bid_amount_sum, 2001);
		assert_eq!(
			NextSlotCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![2, 1]
		);

		// 3. Account 2 bids above 1

		// should be able to re-register

		System::reset_events();
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(3),
			Some(MiningSlotBid { vault_id: 1, amount: 1001u32.into() }),
			RewardDestination::Owner,
			3.into()
		));
		System::assert_has_event(
			Event::SlotBidderAdded { account_id: 3, bid_amount: 1001u32.into(), index: 1 }.into(),
		);
		System::assert_last_event(
			Event::SlotBidderOut {
				account_id: 1,
				bid_amount: 1000u32.into(),
				obligation_id: Some(1),
			}
			.into(),
		);
		assert_eq!(HistoricalBidsPerSlot::<Test>::get().into_inner()[0].clone().bids_count, 3);

		assert_eq!(
			NextSlotCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![2, 3, 1]
		);

		let obligations = Obligations::get();
		assert_eq!(obligations.len(), 3);

		// should still hold amount
		assert_eq!(Ownership::free_balance(1), 500);
		assert!(Ownership::hold_available(&HoldReason::RegisterAsMiner.into(), &1));

		// 4. Account 1 increases bid and resubmits

		System::reset_events();
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			Some(MiningSlotBid { vault_id: 1, amount: 1002u32.into() }),
			RewardDestination::Owner,
			1.into()
		));
		let first_bid = HistoricalBidsPerSlot::<Test>::get().into_inner()[0].clone();
		assert_eq!(first_bid.bids_count, 4);
		assert_eq!(first_bid.bid_amount_min, 1000);
		assert_eq!(first_bid.bid_amount_max, 1002);
		assert_eq!(first_bid.bid_amount_sum, 4004);

		// compare to the last event record
		System::assert_last_event(
			Event::SlotBidderOut {
				account_id: 3,
				bid_amount: 1001u32.into(),
				obligation_id: Some(3),
			}
			.into(),
		);

		System::assert_has_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 1002u32.into(), index: 0 }.into(),
		);
		assert_eq!(Ownership::free_balance(3), 500);

		assert_eq!(
			NextSlotCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![1, 2, 3]
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
		assert_eq!(
			AuthorityIdToMinerId::<Test>::iter_keys().collect::<Vec<_>>(),
			vec![1u64.into(), 2u64.into()]
		);
		System::assert_has_event(
			Event::SlotBidderDropped {
				account_id: 3,
				obligation_id: Some(3),
				preserved_argonot_hold: false,
			}
			.into(),
		);
		assert_eq!(Ownership::free_balance(3), 1000);
		assert_eq!(Ownership::total_balance(&3), 1000);
		System::assert_has_event(
			Event::NewMiners {
				cohort_id: 2,
				start_index: 4,
				new_miners: BoundedVec::truncate_from(vec![
					MiningRegistration {
						account_id: 1,
						obligation_id: Some(1),
						argonots: 500u32.into(),
						bonded_argons: 1002u32.into(),
						reward_destination: RewardDestination::Owner,
						reward_sharing: None,
						authority_keys: 1.into(),
						cohort_id: 2,
					},
					MiningRegistration {
						account_id: 2,
						obligation_id: Some(2),
						argonots: 500u32.into(),
						bonded_argons: 1001u32.into(),
						reward_destination: RewardDestination::Owner,
						reward_sharing: None,
						authority_keys: 2.into(),
						cohort_id: 2,
					},
				]),
			}
			.into(),
		);
	});
}

#[test]
fn it_will_set_bids_to_max() {
	TicksBetweenSlots::set(3);
	MaxMiners::set(6);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(6);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(2), None, RewardDestination::Owner, 1.into()),
			Error::<Test>::SlotNotTakingBids
		);

		SlotBiddingStartAfterTicks::set(0);
		IsNextSlotBiddingOpen::<Test>::set(true);

		set_ownership(1, 1000u32.into());
		set_ownership(2, 1000u32.into());
		set_ownership(3, 1000u32.into());

		MiningSlots::on_initialize(6);
		let share_amount = 3000 / 6;
		ArgonotsPerMiningSeat::<Test>::set(share_amount);

		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			Some(MiningSlotBid { vault_id: 1, amount: 1000u32.into() }),
			RewardDestination::Owner,
			1.into()
		));
		let obligation_id = NextSlotCohort::<Test>::get()[0].obligation_id;
		assert_eq!(obligation_id, Some(1));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 1000u32.into(), index: 0 }.into(),
		);
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			Some(MiningSlotBid { vault_id: 1, amount: 2000u32.into() }),
			RewardDestination::Owner,
			1.into()
		));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 2000u32.into(), index: 0 }.into(),
		);
		assert_eq!(NextSlotCohort::<Test>::get().len(), 1);
		let entry = &NextSlotCohort::<Test>::get()[0];
		assert_eq!(entry.bonded_argons, 2000u32.into());
		assert_eq!(entry.obligation_id, obligation_id);
	});
}

#[test]
fn it_will_disallow_bids_with_new_vault() {
	TicksBetweenSlots::set(3);
	MaxMiners::set(12);
	MaxCohortSize::set(4);

	new_test_ext().execute_with(|| {
		System::set_block_number(6);
		SlotBiddingStartAfterTicks::set(0);
		IsNextSlotBiddingOpen::<Test>::set(true);

		set_ownership(1, 1000u32.into());

		MiningSlots::on_initialize(6);
		ArgonotsPerMiningSeat::<Test>::set(1000);
		NextSlotCohort::<Test>::set(bounded_vec![
			MiningRegistration {
				account_id: 4,
				obligation_id: None,
				argonots: 0,
				bonded_argons: 1001,
				reward_destination: RewardDestination::Owner,
				reward_sharing: None,
				authority_keys: 1.into(),
				cohort_id: 1,
			},
			MiningRegistration {
				account_id: 2,
				obligation_id: None,
				argonots: 0,
				bonded_argons: 1000,
				reward_destination: RewardDestination::Owner,
				reward_sharing: None,
				authority_keys: 1.into(),
				cohort_id: 1,
			},
			MiningRegistration {
				account_id: 3,
				obligation_id: None,
				argonots: 0,
				bonded_argons: 999,
				reward_destination: RewardDestination::Owner,
				reward_sharing: None,
				authority_keys: 1.into(),
				cohort_id: 1,
			}
		]);

		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			Some(MiningSlotBid { vault_id: 1, amount: 1000u32.into() }),
			RewardDestination::Owner,
			1.into()
		));
		let next_cohort = NextSlotCohort::<Test>::get();
		assert_eq!(next_cohort.len(), 4);
		assert_eq!(next_cohort[2].account_id, 1);
		let obligation_id = next_cohort[2].obligation_id;
		assert_eq!(obligation_id, Some(1));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 1000u32.into(), index: 2 }.into(),
		);

		assert_err!(
			MiningSlots::bid(
				RuntimeOrigin::signed(1),
				Some(MiningSlotBid { vault_id: 1, amount: 999u32.into() }),
				RewardDestination::Owner,
				1.into()
			),
			Error::<Test>::CannotReduceBondedArgons
		);

		assert_err!(
			MiningSlots::bid(
				RuntimeOrigin::signed(1),
				Some(MiningSlotBid { vault_id: 2, amount: 2000u32.into() }),
				RewardDestination::Owner,
				1.into()
			),
			Error::<Test>::InvalidVaultSwitch
		);
	});
}

fn keys() -> OpaqueBitcoinXpub {
	let mut seed = [0u8; 32];
	OsRng.fill_bytes(&mut seed);

	let xpriv = Xpriv::new_master(GetBitcoinNetwork::get(), &seed).unwrap();
	let child = xpriv
		.derive_priv(
			&Secp256k1::new(),
			&[ChildNumber::from_normal_idx(0).unwrap(), ChildNumber::from_hardened_idx(1).unwrap()],
		)
		.unwrap();
	let xpub = Xpub::from_priv(&Secp256k1::new(), &child);
	OpaqueBitcoinXpub(xpub.encode())
}

#[test]
fn it_handles_rebids() {
	TicksBetweenSlots::set(3);
	MaxMiners::set(12);
	MaxCohortSize::set(4);
	UseRealVaults::set(true);

	new_test_ext().execute_with(|| {
		System::set_block_number(6);
		SlotBiddingStartAfterTicks::set(0);
		ArgonotsPerMiningSeat::<Test>::set(1000);
		IsNextSlotBiddingOpen::<Test>::set(true);

		set_argons(0, 10_000_000);

		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(0),
			pallet_vaults::VaultConfig {
				terms: VaultTerms {
					bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
					bonded_argons_annual_percent_rate: FixedU128::from_float(0.1),
					bitcoin_base_fee: 1000,
					bonded_argons_base_fee: 1000,
					mining_reward_sharing_percent_take: FixedU128::zero(),
				},
				bitcoin_xpubkey: keys(),
				bitcoin_amount_allocated: 3_000_000,
				bonded_argons_allocated: 3_000_000,
				added_securitization_percent: FixedU128::from_float(0.0),
			}
		));

		set_argons(10, 5_000_000);

		// create bonded argons
		assert_ok!(Vaults::create_obligation(
			1,
			&10,
			FundType::Bitcoin,
			3_000_000,
			ObligationExpiration::BitcoinBlock(100),
			365 * 24 * 60,
		));

		for i in 1..=4u64 {
			set_ownership(i, 1000u32.into());
			set_argons(i, 500_004u32.into());

			assert_ok!(MiningSlots::bid(
				RuntimeOrigin::signed(i),
				Some(MiningSlotBid { vault_id: 1, amount: (i + 500_000u64).into() }),
				RewardDestination::Owner,
				i.into()
			));
			assert_eq!(
				Balances::balance_on_hold(&pallet_vaults::HoldReason::ObligationFee.into(), &i),
				1
			);
		}

		MiningSlots::on_initialize(6);

		let next_cohort = NextSlotCohort::<Test>::get();
		assert_eq!(next_cohort.len(), 4);
		assert_eq!(next_cohort.iter().map(|a| a.account_id).collect::<Vec<_>>(), vec![4, 3, 2, 1]);
		assert_eq!(
			Balances::balance_on_hold(&pallet_vaults::HoldReason::ObligationFee.into(), &0),
			5000
		);
		// ensure adding bids works
		assert_err!(
			MiningSlots::bid(
				RuntimeOrigin::signed(1),
				Some(MiningSlotBid { vault_id: 1, amount: 1005u64.into() }),
				RewardDestination::Owner,
				1.into()
			),
			Error::<Test>::CannotReduceBondedArgons
		);
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			Some(MiningSlotBid { vault_id: 1, amount: 501_005u64.into() }),
			RewardDestination::Owner,
			1.into()
		));
		assert_eq!(
			Balances::balance_on_hold(&pallet_vaults::HoldReason::ObligationFee.into(), &1),
			1
		);
		assert_eq!(
			Balances::balance_on_hold(&pallet_vaults::HoldReason::ObligationFee.into(), &0),
			6000
		);
		assert_eq!(
			NextSlotCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![1, 4, 3, 2]
		);

		set_argons(11, 10_000_000);

		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(11),
			pallet_vaults::VaultConfig {
				terms: VaultTerms {
					bitcoin_annual_percent_rate: FixedU128::from_float(2.0),
					bonded_argons_annual_percent_rate: FixedU128::from_float(2.0),
					bitcoin_base_fee: 10_000,
					bonded_argons_base_fee: 10_000,
					mining_reward_sharing_percent_take: FixedU128::zero(),
				},
				bitcoin_xpubkey: keys(),
				bitcoin_amount_allocated: 2_000_000,
				bonded_argons_allocated: 2_000_000,
				added_securitization_percent: FixedU128::from_float(0.0),
			}
		));
		set_argons(12, 5_000_000);
		assert_ok!(Vaults::create_obligation(
			2,
			&12,
			FundType::Bitcoin,
			2_000_000,
			ObligationExpiration::BitcoinBlock(100),
			365 * 24 * 60,
		));

		CurrentTick::set(20);
	});
}

#[test]
fn handles_a_max_of_bids_per_block() {
	TicksBetweenSlots::set(1);
	MaxMiners::set(4);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(4);
		ElapsedTicks::set(4);
		MiningSlots::on_initialize(4);
		SlotBiddingStartAfterTicks::set(0);
		IsNextSlotBiddingOpen::<Test>::set(true);

		for i in 1..5 {
			set_ownership(i, 1000u32.into());
		}

		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			None,
			RewardDestination::Owner,
			1.into()
		));
		assert_eq!(HistoricalBidsPerSlot::<Test>::get().into_inner()[0].clone().bids_count, 1);
		assert_eq!(AuthorityIdToMinerId::<Test>::get::<UintAuthorityId>(1.into()), Some(1));

		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 0u32.into(), index: 0 }.into(),
		);
		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(2), None, RewardDestination::Owner, 1.into()),
			Error::<Test>::CannotRegisterDuplicateKeys
		);
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(2),
			None,
			RewardDestination::Owner,
			2.into()
		));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 2, bid_amount: 0u32.into(), index: 1 }.into(),
		);
		assert_noop!(
			MiningSlots::bid(RuntimeOrigin::signed(3), None, RewardDestination::Owner, 3.into()),
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
fn records_profit_sharing_if_applicable() {
	TicksBetweenSlots::set(1);
	MaxMiners::set(4);
	MaxCohortSize::set(2);
	VaultSharing::set(Some(RewardSharing {
		account_id: 30,
		percent_take: FixedU128::from_rational(90, 100),
	}));

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ElapsedTicks::set(4);
		CurrentTick::set(4);
		MiningSlots::on_initialize(4);
		IsNextSlotBiddingOpen::<Test>::set(true);

		set_ownership(1, 1000u32.into());

		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			Some(MiningSlotBid { vault_id: 1, amount: 1000u32.into() }),
			RewardDestination::Account(25),
			1.into()
		));
		assert_eq!(HistoricalBidsPerSlot::<Test>::get().into_inner()[0].bids_count, 1);

		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 1000u32.into(), index: 0 }.into(),
		);
		assert_eq!(
			NextSlotCohort::<Test>::get()[0].reward_sharing,
			Some(RewardSharing { account_id: 30, percent_take: FixedU128::from_rational(90, 100) })
		);

		ElapsedTicks::set(6);
		CurrentTick::set(6);
		MiningSlots::on_initialize(6);
		assert_eq!(HistoricalBidsPerSlot::<Test>::get().into_inner()[0].bids_count, 0);
		assert_eq!(NextSlotCohort::<Test>::get().len(), 0);
		let rewards_accounts = MiningSlots::get_all_rewards_accounts();
		assert_eq!(rewards_accounts.len(), 2);
		assert_eq!(rewards_accounts[0], (25, Some(FixedU128::from_rational(10, 100))));
		assert_eq!(rewards_accounts[1], (30, Some(FixedU128::from_rational(90, 100))));

		assert_eq!(
			MiningSlots::get_rewards_account(&1),
			(
				Some(25),
				Some(RewardSharing {
					account_id: 30,
					percent_take: FixedU128::from_rational(90, 100)
				})
			)
		);
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
	MaxMiners::set(100);
	new_test_ext().execute_with(|| {
		System::set_block_number(8);
		ElapsedTicks::set(8);

		for i in 0..100u32 {
			let account_id: u64 = i.into();
			ActiveMinersByIndex::<Test>::insert(
				i,
				MiningRegistration {
					account_id,
					obligation_id: None,
					argonots: 0,
					bonded_argons: 0,
					reward_destination: RewardDestination::Owner,
					reward_sharing: None,
					authority_keys: account_id.into(),
					cohort_id: 1,
				},
			);
			AccountIndexLookup::<Test>::insert(account_id, i);
		}
		AuthorityHashByIndex::<Test>::try_mutate(|a| {
			for i in 0..100u32 {
				// these are normally hashed, but we'll simplify to ease the xor calculation
				let hash = U256::from(i);
				let _ = a.try_insert(i, hash);
			}
			Ok::<(), Error<Test>>(())
		})
		.expect("Didn't insert authorities");

		assert_eq!(
			MiningSlots::xor_closest_authority(U256::from(100)),
			Some(MiningAuthority {
				account_id: 96,
				authority_id: UintAuthorityId(96),
				authority_index: 96,
			})
		);
	});
}

#[test]
fn it_will_end_auctions_if_a_seal_qualifies() {
	TicksBetweenSlots::set(100);
	MaxMiners::set(6);
	MaxCohortSize::set(2);
	BlocksBeforeBidEndForVrfClose::set(10);
	SlotBiddingStartAfterTicks::set(0);

	new_test_ext().execute_with(|| {
		System::set_block_number(89);
		CurrentTick::set(89);
		ElapsedTicks::set(89);

		IsNextSlotBiddingOpen::<Test>::set(true);

		let seal = BlockSealInherent::Compute;
		// it's too soon
		assert!(!MiningSlots::check_for_bidding_close(&seal));

		// This seal strength was generated using the commented out loop below
		let seal_strength = U256::from_dec_str(
			"55660301883345363905660969606306034026269601808931936101802154266730817045052",
		)
		.expect("can read seal strength u256");

		let seal = create_block_vote_seal(seal_strength);
		assert!(!MiningSlots::check_for_bidding_close(&seal));

		// now we're the right block
		System::set_block_number(90);
		ElapsedTicks::set(90);
		CurrentTick::set(90);
		assert!(MiningSlots::check_for_bidding_close(&seal));
		assert!(!MiningSlots::check_for_bidding_close(&BlockSealInherent::Compute));

		let invalid_strength = U256::from(1);
		let seal = create_block_vote_seal(invalid_strength);
		assert!(!MiningSlots::check_for_bidding_close(&seal));

		System::assert_last_event(
			Event::MiningBidsClosed { cohort_id: LastActivatedCohortId::<Test>::get() + 1 }.into(),
		);

		if env::var("TEST_DISTRO").unwrap_or("false".to_string()) == "true" {
			let mut valid_seals = vec![];
			for _ in 0..10u32 {
				let seal_strength = U256::from_big_endian(H256::random().as_ref());
				let seal = create_block_vote_seal(seal_strength);

				if MiningSlots::check_for_bidding_close(&seal) {
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
	MaxMiners::set(100);
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
fn it_doesnt_accept_bids_until_first_slot() {
	TicksBetweenSlots::set(1440);
	MaxMiners::set(100);
	MaxCohortSize::set(10);
	SlotBiddingStartAfterTicks::set(12_960);
	TargetBidsPerSlot::set(12);
	MinOwnershipBondAmount::set(100_000);

	new_test_ext().execute_with(|| {
		set_ownership(2, 1000u32.into());
		ArgonotsPerMiningSeat::<Test>::set(1_000);

		System::set_block_number(1);
		ElapsedTicks::set(12959);

		MiningSlots::on_initialize(1);
		assert!(!IsNextSlotBiddingOpen::<Test>::get());

		// bidding will start on the first (block % 1440 == 0)
		ElapsedTicks::set(12960);
		MiningSlots::on_initialize(12960);
		assert_eq!(LastActivatedCohortId::<Test>::get(), 0);
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(2),
			None,
			RewardDestination::Owner,
			1.into()
		));
		assert_eq!(ActiveMinersCount::<Test>::get(), 0);

		let next_divisible_period = 12960 + 1440;
		System::set_block_number(next_divisible_period);
		ElapsedTicks::set(next_divisible_period);
		CurrentTick::set(next_divisible_period);
		MiningSlots::on_initialize(next_divisible_period);
		assert_eq!(LastActivatedCohortId::<Test>::get(), 1);
		assert!(IsNextSlotBiddingOpen::<Test>::get());
		assert_eq!(MiningSlots::get_next_slot_starting_index(), 20);
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
		let starting_config = MiningConfig::<Test>::get();
		assert_ok!(MiningSlots::configure_mining_slot_delay(RuntimeOrigin::root(), 12961));
		assert_eq!(MiningConfig::<Test>::get().slot_bidding_start_after_ticks, 12961);
		System::assert_last_event(
			Event::MiningConfigurationUpdated {
				slot_bidding_start_after_ticks: 12961,
				ticks_between_slots: starting_config.ticks_between_slots,
				ticks_before_bid_end_for_vrf_close: starting_config
					.ticks_before_bid_end_for_vrf_close,
			}
			.into(),
		);
	});
}

#[test]
fn it_should_rotate_grandpas() {
	SlotBiddingStartAfterTicks::set(12_960 - 2);
	GrandpaRotationFrequency::set(4);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ElapsedTicks::set(1);
		CurrentTick::set(1);
		MiningSlots::on_initialize(1);
		assert_eq!(LastActivatedCohortId::<Test>::get(), 0);
		assert_eq!(GrandaRotations::get(), vec![0]);

		LastActivatedCohortId::<Test>::put(1);
		System::set_block_number(4);
		ElapsedTicks::set(4);
		CurrentTick::set(4);
		MiningSlots::on_initialize(4);
		assert_eq!(GrandaRotations::get(), vec![0, 1]);

		// should auto-increment and rotate
		System::set_block_number(12_960);
		ElapsedTicks::set(12_960);
		CurrentTick::set(12_960);
		MiningSlots::on_initialize(12_960);
		assert_eq!(GrandaRotations::get(), vec![0, 1, 2]);
	});
}

fn create_block_vote_seal(seal_strength: U256) -> BlockSealInherent {
	BlockSealInherent::Vote {
		seal_strength,
		notary_id: 1,
		block_vote: BlockVote {
			block_hash: System::block_hash(System::block_number().saturating_sub(4)),
			account_id: AccountId32::from_slice(&[2u8; 32]).expect("32 bytes"),
			index: 1,
			power: 500,
			tick: 1,
			block_rewards_account_id: AccountId32::from_slice(&[3u8; 32]).expect("32 bytes"),
			signature: sp_core::sr25519::Signature::from_raw([0u8; 64]).into(),
		},
		source_notebook_number: 1,
		source_notebook_proof: MerkleProof::default(),
	}
}
