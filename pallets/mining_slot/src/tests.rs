use std::{collections::HashMap, env};

use frame_support::{
	assert_err, assert_noop, assert_ok,
	traits::{fungible::InspectHold, Currency, OnInitialize, OneSessionHandler},
};
use pallet_balances::Event as UlixeeBalancesEvent;
use sp_core::{bounded_vec, crypto::AccountId32, ByteArray, H256, U256};
use sp_runtime::{testing::UintAuthorityId, BoundedVec};

use ulx_primitives::{
	block_seal::{MiningAuthority, MiningRegistration, RewardDestination},
	inherents::BlockSealInherent,
	AuthorityProvider, BlockSealAuthorityId, BlockVote, DataDomain, DataTLD, MerkleProof,
};

use crate::{
	mock::{MiningSlots, UlixeeBalances, *},
	pallet::{
		AccountIndexLookup, ActiveMinersByIndex, ActiveMinersCount, AuthoritiesByIndex,
		IsNextSlotBiddingOpen, NextSlotCohort, OwnershipBondAmount,
	},
	Error, Event, HoldReason, MiningSlotBid,
};

#[test]
fn it_doesnt_add_cohorts_until_time() {
	BlocksBetweenSlots::set(2);

	new_test_ext(None).execute_with(|| {
		System::set_block_number(1);

		NextSlotCohort::<Test>::set(bounded_vec![MiningRegistration {
			account_id: 1,
			bond_id: None,
			ownership_tokens: 0,
			bond_amount: 0,
			reward_destination: RewardDestination::Owner,
		}]);

		MiningSlots::on_initialize(1);

		assert_eq!(NextSlotCohort::<Test>::get().len(), 1);
		assert_eq!(NextSlotCohort::<Test>::get()[0].reward_destination, RewardDestination::Owner);
	});
}

#[test]
fn get_validation_window_blocks() {
	MaxCohortSize::set(2);
	MaxMiners::set(10);
	BlocksBetweenSlots::set(1);

	assert_eq!(MiningSlots::get_mining_window_blocks(), 5);

	MaxCohortSize::set(5);
	MaxMiners::set(10);
	BlocksBetweenSlots::set(10);

	assert_eq!(MiningSlots::get_mining_window_blocks(), 2 * 10);
}

#[test]
fn get_slot_era() {
	new_test_ext(None).execute_with(|| {
		System::set_block_number(8);

		MaxCohortSize::set(2);
		MaxMiners::set(10);
		BlocksBetweenSlots::set(140);
		let window = 140 * 5;

		assert_eq!(MiningSlots::get_slot_era(), (140, 140 + window));

		MaxCohortSize::set(5);
		MaxMiners::set(10);
		BlocksBetweenSlots::set(10);

		assert_eq!(MiningSlots::get_slot_era(), (10, 10 + (10 * 2)));

		// on block, should start showing next period
		System::set_block_number(10);
		assert_eq!(MiningSlots::get_slot_era(), (20, 20 + (10 * 2)));

		System::set_block_number(18);
		assert_eq!(MiningSlots::get_slot_era(), (20, 20 + (10 * 2)));

		System::set_block_number(20);
		assert_eq!(MiningSlots::get_slot_era(), (30, 30 + (10 * 2)));

		BlocksBetweenSlots::set(4);
		MaxMiners::set(6);
		MaxCohortSize::set(2);
		System::set_block_number(3);
		assert_eq!(MiningSlots::get_slot_era(), (4, 4 + (4 * 3)));
		System::set_block_number(4);
		assert_eq!(MiningSlots::get_slot_era(), (8, 8 + (4 * 3)));
		System::set_block_number(6);
		assert_eq!(MiningSlots::get_slot_era(), (8, 8 + (4 * 3)));
	})
}

#[test]
fn starting_cohort_index() {
	let max_cohort_size = 3;
	let max_validators = 12;
	let blocks_between_slots = 5;

	assert_eq!(
		MiningSlots::get_slot_starting_index(
			0,
			blocks_between_slots,
			max_validators,
			max_cohort_size
		),
		0
	);
	assert_eq!(
		MiningSlots::get_slot_starting_index(
			5,
			blocks_between_slots,
			max_validators,
			max_cohort_size
		),
		3
	);
	assert_eq!(
		MiningSlots::get_slot_starting_index(
			10,
			blocks_between_slots,
			max_validators,
			max_cohort_size
		),
		6
	);
	assert_eq!(
		MiningSlots::get_slot_starting_index(
			15,
			blocks_between_slots,
			max_validators,
			max_cohort_size
		),
		9
	);

	assert_eq!(
		MiningSlots::get_slot_starting_index(
			20,
			blocks_between_slots,
			max_validators,
			max_cohort_size
		),
		0
	);
}

#[test]
fn it_activates_miner_zero_if_no_miners() {
	BlocksBetweenSlots::set(2);
	MaxMiners::set(6);
	MaxCohortSize::set(2);

	new_test_ext(Some(MiningRegistration {
		account_id: 1,
		bond_id: None,
		ownership_tokens: 0,
		bond_amount: 0,
		reward_destination: RewardDestination::Owner,
	}))
	.execute_with(|| {
		MiningSlots::on_initialize(1);

		assert_eq!(ActiveMinersByIndex::<Test>::get(0).unwrap().account_id, 1);
	});
}

#[test]
fn it_activates_miner_zero_if_upcoming_miners_will_empty() {
	BlocksBetweenSlots::set(2);
	MaxMiners::set(6);
	MaxCohortSize::set(2);

	new_test_ext(Some(MiningRegistration {
		account_id: 1,
		bond_id: None,
		ownership_tokens: 0,
		bond_amount: 0,
		reward_destination: RewardDestination::Owner,
	}))
	.execute_with(|| {
		ActiveMinersByIndex::<Test>::insert(
			5,
			MiningRegistration {
				account_id: 10,
				bond_id: None,
				ownership_tokens: 0,
				bond_amount: 0,
				reward_destination: RewardDestination::Owner,
			},
		);
		AccountIndexLookup::<Test>::insert(10, 5);
		ActiveMinersCount::<Test>::put(1);
		MiningSlots::on_initialize(2);

		assert_eq!(ActiveMinersByIndex::<Test>::get(0).unwrap().account_id, 1);
		assert_eq!(ActiveMinersCount::<Test>::get(), 1);
	});
}

#[test]
fn it_adds_new_cohorts_on_block() {
	BlocksBetweenSlots::set(2);
	MaxMiners::set(6);
	MaxCohortSize::set(2);

	new_test_ext(None).execute_with(|| {
		System::set_block_number(8);

		for i in 0..4u32 {
			let account_id: u64 = (i + 4).into();
			ActiveMinersByIndex::<Test>::insert(
				i,
				MiningRegistration {
					account_id,
					bond_id: None,
					ownership_tokens: 0,
					bond_amount: 0,
					reward_destination: RewardDestination::Owner,
				},
			);
			AccountIndexLookup::<Test>::insert(account_id, i);
		}
		ActiveMinersCount::<Test>::put(4);

		let cohort = BoundedVec::truncate_from(vec![MiningRegistration {
			account_id: 1,
			bond_id: None,
			ownership_tokens: 0,
			bond_amount: 0,
			reward_destination: RewardDestination::Owner,
		}]);

		NextSlotCohort::<Test>::set(cohort.clone());

		MiningSlots::on_initialize(8);

		// re-fetch
		let validators = ActiveMinersByIndex::<Test>::iter().collect::<HashMap<_, _>>();
		assert_eq!(
			NextSlotCohort::<Test>::get().len(),
			0,
			"Queued mining_slot for block 8 should be removed"
		);
		assert_eq!(validators.len(), 3, "Should have 3 validators still after insertion");
		assert_eq!(
			ActiveMinersCount::<Test>::get(),
			3,
			"Should have 3 validators still after insertion"
		);
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

		System::assert_last_event(Event::NewMiners { start_index: 2, new_miners: cohort }.into())
	});
}

#[test]
fn it_unbonds_accounts_when_a_window_closes() {
	BlocksBetweenSlots::set(2);
	MaxMiners::set(6);
	MaxCohortSize::set(2);

	new_test_ext(None).execute_with(|| {
		OwnershipBondAmount::<Test>::set(1000);

		System::set_block_number(7);

		for i in 0..4u32 {
			let account_id: u64 = (i).into();
			set_ownership(account_id, 1000u32.into());
			set_argons(account_id, 10_000u32.into());

			let bond_amount = (1000u32 + i).into();
			let ownership_tokens =
				MiningSlots::hold_ownership_bond(&account_id, None).ok().unwrap();

			ActiveMinersByIndex::<Test>::insert(
				i,
				MiningRegistration {
					account_id,
					bond_id: None,
					ownership_tokens,
					bond_amount,
					reward_destination: RewardDestination::Owner,
				},
			);
			AccountIndexLookup::<Test>::insert(account_id, i);
		}
		ActiveMinersCount::<Test>::put(4);
		IsNextSlotBiddingOpen::<Test>::set(true);
		assert_eq!(MiningSlots::get_slot_era(), (8, 8 + (2 * 3)));

		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), None, RewardDestination::Owner));

		System::set_block_number(8);

		MiningSlots::on_initialize(8);

		System::assert_last_event(
			Event::NewMiners {
				start_index: 2,
				new_miners: BoundedVec::truncate_from(vec![MiningRegistration {
					bond_id: None,
					account_id: 2,
					ownership_tokens: 1000u32.into(),
					bond_amount: 0,
					reward_destination: RewardDestination::Owner,
				}]),
			}
			.into(),
		);

		System::assert_has_event(
			UlixeeBalancesEvent::<Test, UlixeeToken>::Endowed {
				account: 3,
				free_balance: 1000u32.into(),
			}
			.into(),
		);

		System::assert_has_event(
			Event::<Test>::UnbondedMiner {
				account_id: 3,
				bond_id: None,
				kept_ownership_bond: false,
			}
			.into(),
		);
		assert_eq!(UlixeeBalances::free_balance(2), 0);
		assert_eq!(UlixeeBalances::total_balance(&2), 1000);

		assert_eq!(UlixeeBalances::free_balance(3), 1000);

		assert!(System::account_exists(&0));
		assert!(System::account_exists(&1));
		assert!(System::account_exists(&2));
		assert!(System::account_exists(&3));
	});
}

#[test]
fn it_holds_ownership_shares_for_a_slot() {
	BlocksBetweenSlots::set(3);
	MaxMiners::set(6);
	MaxCohortSize::set(2);

	new_test_ext(None).execute_with(|| {
		System::set_block_number(3);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(2), None, RewardDestination::Owner),
			Error::<Test>::SlotNotTakingBids
		);

		IsNextSlotBiddingOpen::<Test>::set(true);

		set_ownership(3, 5000u32.into());
		MiningSlots::on_initialize(3);
		assert_eq!(OwnershipBondAmount::<Test>::get(), 666);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(1), None, RewardDestination::Owner),
			Error::<Test>::InsufficientOwnershipTokens
		);

		set_ownership(1, 1000u32.into());

		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), None, RewardDestination::Owner));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 0u32.into(), index: 0 }.into(),
		);
		assert_eq!(UlixeeBalances::free_balance(1), 334);

		// should be able to re-register
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), None, RewardDestination::Owner));
		assert_eq!(
			NextSlotCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![1]
		);
		assert_eq!(UlixeeBalances::free_balance(1), 334, "should not alter reserved balance");
		assert_eq!(UlixeeBalances::total_balance(&1), 1000, "should still have their full balance");

		assert!(System::account_exists(&1));
		assert!(System::account_exists(&3));
	});
}

#[test]
fn it_wont_let_you_reuse_ownership_shares_for_two_bids() {
	BlocksBetweenSlots::set(4);
	MaxMiners::set(6);
	MaxCohortSize::set(2);

	new_test_ext(None).execute_with(|| {
		System::set_block_number(3);

		IsNextSlotBiddingOpen::<Test>::set(true);

		assert_eq!(MiningSlots::get_slot_era(), (4, 4 + (4 * 3)));
		assert_eq!(MiningSlots::get_next_slot_starting_index(), 2);
		set_ownership(2, 100u32.into());
		set_ownership(1, 100u32.into());
		MiningSlots::on_initialize(3);

		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), None, RewardDestination::Owner));
		System::set_block_number(4);
		MiningSlots::on_initialize(4);

		System::assert_last_event(
			Event::NewMiners {
				start_index: 2,
				new_miners: BoundedVec::truncate_from(vec![MiningRegistration {
					account_id: 1,
					bond_id: None,
					ownership_tokens: 26u32.into(),
					bond_amount: 0,
					reward_destination: RewardDestination::Owner,
				}]),
			}
			.into(),
		);
		assert_eq!(OwnershipBondAmount::<Test>::get(), 26);
		assert_eq!(MiningSlots::get_slot_era(), (8, 8 + (4 * 3)));
		assert_eq!(MiningSlots::get_next_slot_starting_index(), 4);

		System::set_block_number(6);
		MiningSlots::on_initialize(6);
		assert_eq!(MiningSlots::get_slot_era(), (8, 8 + (4 * 3)));
		assert_eq!(MiningSlots::get_next_slot_starting_index(), 4);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(1), None, RewardDestination::Owner),
			Error::<Test>::CannotRegisterOverlappingSessions
		);
		assert!(System::account_exists(&1));
		assert!(System::account_exists(&2));
	})
}

#[test]
fn it_will_order_bids_with_argon_bonds() {
	BlocksBetweenSlots::set(3);
	MaxMiners::set(6);
	MaxCohortSize::set(2);

	new_test_ext(None).execute_with(|| {
		System::set_block_number(3);

		assert_err!(
			MiningSlots::bid(RuntimeOrigin::signed(2), None, RewardDestination::Owner),
			Error::<Test>::SlotNotTakingBids
		);

		IsNextSlotBiddingOpen::<Test>::set(true);

		set_ownership(1, 1000u32.into());
		set_ownership(2, 1000u32.into());
		set_ownership(3, 1000u32.into());

		MiningSlots::on_initialize(3);
		assert_eq!(OwnershipBondAmount::<Test>::get(), 400);

		// 1. Account 1 bids
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			Some(MiningSlotBid { vault_id: 1, amount: 1000u32.into() }),
			RewardDestination::Owner
		));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 1000u32.into(), index: 0 }.into(),
		);
		assert_eq!(UlixeeBalances::free_balance(1), 600);

		assert_eq!(Bonds::get().len(), 1);

		// 2. Account 2 bids highest and takes top slot

		// should be able to re-register
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(2),
			Some(MiningSlotBid { vault_id: 1, amount: 1001u32.into() }),
			RewardDestination::Owner
		));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 2, bid_amount: 1001u32.into(), index: 0 }.into(),
		);

		assert_eq!(
			NextSlotCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![2, 1]
		);

		// 3. Account 2 bids above 1

		// should be able to re-register
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(3),
			Some(MiningSlotBid { vault_id: 1, amount: 1001u32.into() }),
			RewardDestination::Owner
		));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 3, bid_amount: 1001u32.into(), index: 1 }.into(),
		);

		assert_eq!(
			NextSlotCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![2, 3]
		);

		let bonds = Bonds::get();
		let acc_3_bond_id = bonds.iter().find(|(_, _, a, _)| *a == 3).map(|a| a.0);
		assert!(!bonds.iter().any(|(_, _, a, _)| *a == 1));
		assert_eq!(bonds.len(), 2);
		{
			let events = frame_system::Pallet::<Test>::events();
			let frame_system::EventRecord { event, .. } = &events[events.len() - 2];
			assert_eq!(
				event,
				&<Test as frame_system::Config>::RuntimeEvent::from(
					Event::<Test>::SlotBidderReplaced {
						bond_id: Some(1u64),
						account_id: 1u64,
						kept_ownership_bond: false,
					}
				)
			);
		}
		assert_eq!(UlixeeBalances::free_balance(1), 1000);
		assert!(UlixeeBalances::hold_available(&HoldReason::RegisterAsMiner.into(), &1));

		// 4. Account 1 increases bid and resubmits
		assert_ok!(MiningSlots::bid(
			RuntimeOrigin::signed(1),
			Some(MiningSlotBid { vault_id: 1, amount: 1002u32.into() }),
			RewardDestination::Owner
		));

		let events = frame_system::Pallet::<Test>::events();
		// compare to the last event record
		let frame_system::EventRecord { event, .. } = &events[events.len() - 2];
		assert_eq!(
			event,
			&<Test as frame_system::Config>::RuntimeEvent::from(
				Event::<Test>::SlotBidderReplaced {
					bond_id: acc_3_bond_id,
					account_id: 3u64,
					kept_ownership_bond: false
				}
			)
		);

		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 1002u32.into(), index: 0 }.into(),
		);
		assert_eq!(UlixeeBalances::free_balance(3), 1000);

		assert_eq!(
			NextSlotCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![1, 2]
		);
		assert!(System::account_exists(&1));
		assert!(System::account_exists(&2));
		assert!(System::account_exists(&3));
	});
}
#[test]
fn handles_a_max_of_bids_per_block() {
	BlocksBetweenSlots::set(1440);
	MaxMiners::set(50);
	MaxCohortSize::set(2);

	new_test_ext(None).execute_with(|| {
		System::set_block_number(1);
		IsNextSlotBiddingOpen::<Test>::set(true);

		for i in 1..5 {
			set_ownership(i, 1000u32.into());
		}

		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(1), None, RewardDestination::Owner));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 1, bid_amount: 0u32.into(), index: 0 }.into(),
		);
		assert_ok!(MiningSlots::bid(RuntimeOrigin::signed(2), None, RewardDestination::Owner));
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 2, bid_amount: 0u32.into(), index: 1 }.into(),
		);
		assert_noop!(
			MiningSlots::bid(RuntimeOrigin::signed(3), None, RewardDestination::Owner),
			Error::<Test>::BidTooLow,
		);
		// should not have changed
		System::assert_last_event(
			Event::SlotBidderAdded { account_id: 2, bid_amount: 0u32.into(), index: 1 }.into(),
		);
		for i in 1..5 {
			assert!(System::account_exists(&i));
		}
	});
}

#[test]
fn it_handles_null_authority() {
	new_test_ext(None).execute_with(|| {
		assert_eq!(MiningSlots::get_authority(1), None);
	});
}

#[test]
fn it_can_get_closest_authority() {
	MaxMiners::set(100);
	new_test_ext(None).execute_with(|| {
		System::set_block_number(8);

		for i in 0..100u32 {
			let account_id: u64 = i.into();
			ActiveMinersByIndex::<Test>::insert(
				i,
				MiningRegistration {
					account_id,
					bond_id: None,
					ownership_tokens: 0,
					bond_amount: 0,
					reward_destination: RewardDestination::Owner,
				},
			);
			AccountIndexLookup::<Test>::insert(account_id, i);
		}
		AuthoritiesByIndex::<Test>::try_mutate(|a| {
			for i in 0..100u32 {
				let account_id: u64 = i.into();
				let public_key: BlockSealAuthorityId = UintAuthorityId(account_id).to_public_key();
				// these are normally hashed, but we'll simplify to ease the xor calculation
				let hash = U256::from(i);
				let _ = a.try_insert(i, (public_key, hash));
			}
			Ok::<(), Error<Test>>(())
		})
		.expect("Didn't insert authorities");

		assert_eq!(
			MiningSlots::xor_closest_authority(U256::from(100)),
			Some(MiningAuthority {
				account_id: 96,
				authority_id: UintAuthorityId(96).to_public_key(),
				authority_index: 96,
			})
		);
	});
}

#[test]
fn it_can_replace_authority_keys() {
	MaxMiners::set(10);
	new_test_ext(None).execute_with(|| {
		System::set_block_number(8);

		for i in 0..10u32 {
			let account_id: u64 = i.into();
			ActiveMinersByIndex::<Test>::insert(
				i,
				MiningRegistration {
					account_id,
					bond_id: None,
					ownership_tokens: 0,
					bond_amount: 0,
					reward_destination: RewardDestination::Owner,
				},
			);
			AccountIndexLookup::<Test>::insert(account_id, i);
		}

		AuthoritiesByIndex::<Test>::try_mutate(|a| {
			a.try_insert(0u32, (UintAuthorityId(0u64).to_public_key(), U256::from(0)))
		})
		.expect("Could seed authorities");

		let account_keys = (2..12u32)
			.map(|i| {
				let account_id: u64 = i.into();
				let public_key: BlockSealAuthorityId = UintAuthorityId(account_id).to_public_key();
				(account_id, public_key)
			})
			.collect::<Vec<(u64, BlockSealAuthorityId)>>();

		let with_keys: Box<dyn Iterator<Item = _>> =
			Box::new(account_keys.iter().map(|k| (&k.0, k.clone().1)));

		let queued_keys: Box<dyn Iterator<Item = _>> =
			Box::new(account_keys.iter().map(|k| (&k.0, k.clone().1)));

		assert_eq!(AuthoritiesByIndex::<Test>::get().len(), 1);
		MiningSlots::on_new_session(true, with_keys, queued_keys);
		assert_eq!(AuthoritiesByIndex::<Test>::get().len(), 8, "should register only 8 keys");
		assert!(!AuthoritiesByIndex::<Test>::get().contains_key(&0u32), "should not have index 0");
		assert!(
			!AuthoritiesByIndex::<Test>::get().contains_key(&10u32),
			"should not have index 11"
		);
	});
}

#[test]
fn it_will_end_auctions_if_a_seal_qualifies() {
	BlocksBetweenSlots::set(100);
	MaxMiners::set(6);
	MaxCohortSize::set(2);
	BlocksBeforeBidEndForVrfClose::set(10);

	new_test_ext(None).execute_with(|| {
		System::set_block_number(89);

		IsNextSlotBiddingOpen::<Test>::set(true);

		let seal = BlockSealInherent::Compute;
		// it's too soon
		assert_eq!(MiningSlots::check_for_bidding_close(&seal), false);

		// This seal strength was generated using the commented out loop below
		let seal_strength = U256::from_dec_str(
			"55660301883345363905660969606306034026269601808931936101802154266730817045052",
		)
		.expect("can read seal strength u256");

		let seal = create_block_vote_seal(seal_strength);
		assert_eq!(MiningSlots::check_for_bidding_close(&seal), false);

		// now we're the right block
		System::set_block_number(90);
		assert_eq!(MiningSlots::check_for_bidding_close(&seal), true);
		assert_eq!(MiningSlots::check_for_bidding_close(&BlockSealInherent::Compute), false);

		let invalid_strength = U256::from(1);
		let seal = create_block_vote_seal(invalid_strength);
		assert_eq!(MiningSlots::check_for_bidding_close(&seal), false);

		if env::var("TEST_DISTRO").unwrap_or("false".to_string()) == "true" {
			let mut valid_seals = vec![];
			for _ in 0..10u32 {
				let seal_strength = U256::from_big_endian(H256::random().as_ref());
				let seal = create_block_vote_seal(seal_strength);

				if MiningSlots::check_for_bidding_close(&seal) {
					valid_seals.push(seal_strength);
				}
			}
			assert!(valid_seals.len() > 0, "Should have found at least one valid seal");
			println!("Valid seals: {:?}", valid_seals);
		}
	})
}

fn create_block_vote_seal(seal_strength: U256) -> BlockSealInherent {
	BlockSealInherent::Vote {
		seal_strength,
		notary_id: 1,
		block_vote: BlockVote {
			block_hash: System::block_hash(System::block_number().saturating_sub(4)),
			data_domain_hash: DataDomain::new("test", DataTLD::Bikes).hash(),
			data_domain_account: AccountId32::from_slice(&[0u8; 32]).expect("32 bytes"),
			account_id: AccountId32::from_slice(&[2u8; 32]).expect("32 bytes"),
			index: 1,
			power: 500,
			block_rewards_account_id: AccountId32::from_slice(&[3u8; 32]).expect("32 bytes"),
			signature: sp_core::sr25519::Signature::from_raw([0u8; 64]).into(),
		},
		miner_signature: sp_core::ed25519::Signature::from_raw([0u8; 64]).into(),
		source_notebook_number: 1,
		source_notebook_proof: MerkleProof::default(),
	}
}
