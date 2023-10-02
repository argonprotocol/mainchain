use frame_support::{
	assert_err, assert_noop, assert_ok,
	traits::{Currency, OnInitialize, OneSessionHandler},
};
use sp_core::{bounded_vec, OpaquePeerId, U256};
use sp_runtime::{testing::UintAuthorityId, BoundedVec};
use std::net::Ipv4Addr;

use ulx_primitives::{
	block_seal::{AuthorityProvider, PeerId, RewardDestination, ValidatorRegistration},
	bond::BondProvider,
	BlockSealAuthorityId,
};

use crate::{
	mock::{Cohorts, UlixeeBalances, *},
	pallet::{
		AccountIndexLookup, ActiveValidatorsByIndex, AuthoritiesByIndex, IsCohortAcceptingBids,
		NextCohort, OwnershipBondAmount,
	},
	Error, Event,
};

pub fn ip_to_u32(ip: Ipv4Addr) -> u32 {
	let octets = ip.octets();
	u32::from_be_bytes(octets)
}

pub fn ip_from_u32(ip: u32) -> Ipv4Addr {
	Ipv4Addr::from(ip.to_be_bytes())
}

#[test]
fn it_doesnt_add_cohorts_until_time() {
	BlocksBetweenCohorts::set(2);

	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		NextCohort::<Test>::set(bounded_vec![ValidatorRegistration {
			account_id: 1,
			peer_id: PeerId(OpaquePeerId::default()),
			bond_id: None,
			ownership_tokens: 0,
			bond_amount: 0,
			reward_destination: RewardDestination::Owner,
			rpc_hosts: rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
		}]);

		Cohorts::on_initialize(1);

		assert_eq!(NextCohort::<Test>::get().len(), 1);
		assert_eq!(
			ip_from_u32(NextCohort::<Test>::get()[0].rpc_hosts[0].ip).to_string(),
			"127.0.0.1"
		);
	});
}

#[test]
fn get_validation_window_blocks() {
	MaxCohortSize::set(2);
	MaxValidators::set(10);
	BlocksBetweenCohorts::set(1);

	assert_eq!(Cohorts::get_validation_window_blocks(), 5);

	MaxCohortSize::set(5);
	MaxValidators::set(10);
	BlocksBetweenCohorts::set(10);

	assert_eq!(Cohorts::get_validation_window_blocks(), 2 * 10);
}

#[test]
fn get_next_cohort_period() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(8);

		MaxCohortSize::set(2);
		MaxValidators::set(10);
		BlocksBetweenCohorts::set(140);
		let window = 140 * 5;

		assert_eq!(Cohorts::get_next_cohort_period(), (140, 140 + window));

		MaxCohortSize::set(5);
		MaxValidators::set(10);
		BlocksBetweenCohorts::set(10);

		assert_eq!(Cohorts::get_next_cohort_period(), (10, 10 + (10 * 2)));

		// on block, should start showing next period
		System::set_block_number(10);
		assert_eq!(Cohorts::get_next_cohort_period(), (20, 20 + (10 * 2)));

		System::set_block_number(18);
		assert_eq!(Cohorts::get_next_cohort_period(), (20, 20 + (10 * 2)));

		System::set_block_number(20);
		assert_eq!(Cohorts::get_next_cohort_period(), (30, 30 + (10 * 2)));

		BlocksBetweenCohorts::set(4);
		MaxValidators::set(6);
		MaxCohortSize::set(2);
		System::set_block_number(3);
		assert_eq!(Cohorts::get_next_cohort_period(), (4, 4 + (4 * 3)));
		System::set_block_number(4);
		assert_eq!(Cohorts::get_next_cohort_period(), (8, 8 + (4 * 3)));
		System::set_block_number(6);
		assert_eq!(Cohorts::get_next_cohort_period(), (8, 8 + (4 * 3)));
	})
}

#[test]
fn starting_cohort_index() {
	let max_cohort_size = 3;
	let max_validators = 12;
	let blocks_between_cohorts = 5;

	assert_eq!(
		Cohorts::get_start_cohort_index(0, blocks_between_cohorts, max_validators, max_cohort_size),
		0
	);
	assert_eq!(
		Cohorts::get_start_cohort_index(5, blocks_between_cohorts, max_validators, max_cohort_size),
		3
	);
	assert_eq!(
		Cohorts::get_start_cohort_index(
			10,
			blocks_between_cohorts,
			max_validators,
			max_cohort_size
		),
		6
	);
	assert_eq!(
		Cohorts::get_start_cohort_index(
			15,
			blocks_between_cohorts,
			max_validators,
			max_cohort_size
		),
		9
	);

	assert_eq!(
		Cohorts::get_start_cohort_index(
			20,
			blocks_between_cohorts,
			max_validators,
			max_cohort_size
		),
		0
	);
}

#[test]
fn it_adds_new_cohorts_on_block() {
	BlocksBetweenCohorts::set(2);
	MaxValidators::set(6);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(8);

		ActiveValidatorsByIndex::<Test>::try_mutate(|validators| {
			for i in 0..4u32 {
				let account_id: u64 = (i + 4).into();
				let _ = validators.try_insert(
					i,
					ValidatorRegistration {
						account_id,
						peer_id: empty_peer(),
						bond_id: None,
						ownership_tokens: 0,
						bond_amount: 0,
						reward_destination: RewardDestination::Owner,
						rpc_hosts: rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000 + i as u16),
					},
				);
				AccountIndexLookup::<Test>::insert(account_id, i);
			}
			Ok::<(), Error<Test>>(())
		})
		.expect("Didn't insert validators");

		let cohort = BoundedVec::truncate_from(vec![ValidatorRegistration {
			account_id: 1,
			peer_id: empty_peer(),
			bond_id: None,
			ownership_tokens: 0,
			bond_amount: 0,
			reward_destination: RewardDestination::Owner,
			rpc_hosts: rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3001),
		}]);

		NextCohort::<Test>::set(cohort.clone());

		Cohorts::on_initialize(8);

		// re-fetch
		let validators = ActiveValidatorsByIndex::<Test>::get();
		assert_eq!(
			NextCohort::<Test>::get().len(),
			0,
			"Queued cohorts for block 8 should be removed"
		);
		assert_eq!(validators.len(), 3, "Should have 3 validators still after insertion");
		assert_eq!(validators.contains_key(&2), true, "Should insert validator at index 2");
		assert_eq!(
			validators.contains_key(&3),
			false,
			"Should no longer have a validator at index 3"
		);
		assert_eq!(
			AccountIndexLookup::<Test>::get(1),
			Some(2),
			"Should add an index lookup for account 1 at index 2"
		);
		assert_eq!(
			AccountIndexLookup::<Test>::contains_key(6),
			false,
			"Should no longer have account 6 registered"
		);
		assert_eq!(
			AccountIndexLookup::<Test>::contains_key(7),
			false,
			"Should no longer have account 7 registered"
		);

		System::assert_last_event(
			Event::NewValidators { start_index: 2, new_validators: cohort }.into(),
		)
	});
}

use pallet_balances::Event as UlixeeBalancesEvent;
use ulx_primitives::block_seal::Host;

#[test]
fn it_unbonds_accounts_when_a_window_closes() {
	BlocksBetweenCohorts::set(2);
	MaxValidators::set(6);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		OwnershipBondAmount::<Test>::set(1000);
		// Go past genesis block so events get deposited
		System::set_block_number(7);

		let mut bond_2: BondId = 0;
		let mut bond_3: BondId = 0;
		ActiveValidatorsByIndex::<Test>::try_mutate(|validators| {
			for i in 0..4u32 {
				let account_id: u64 = (i).into();
				set_ownership(account_id, 1000u32.into());
				set_argons(account_id, 10_000u32.into());

				let bond_amount = (1000u32 + i).into();
				let bond_id = match <Bonds as BondProvider>::bond_self(account_id, bond_amount, 10)
				{
					Ok(id) => id,
					Err(_) => panic!("bond should exist"),
				};
				if account_id == 2u64 {
					bond_2 = bond_id
				}
				if account_id == 3u64 {
					bond_3 = bond_id
				}
				<Bonds as BondProvider>::lock_bond(bond_id).expect("bond should lock");
				let ownership_tokens =
					Cohorts::hold_ownership_bond(&account_id, None).ok().unwrap();

				let _ = validators.try_insert(
					i,
					ValidatorRegistration {
						account_id,
						peer_id: empty_peer(),
						rpc_hosts: rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
						bond_id: Some(bond_id),
						ownership_tokens,
						bond_amount,
						reward_destination: RewardDestination::Owner,
					},
				);
				AccountIndexLookup::<Test>::insert(account_id, i);
			}
			Ok::<(), Error<Test>>(())
		})
		.expect("Didn't insert validators");

		IsCohortAcceptingBids::<Test>::set(true);
		assert_eq!(Cohorts::get_next_cohort_period(), (8, 8 + (2 * 3)));

		<Bonds as BondProvider>::extend_bond(bond_2, 2, 1010, 16).expect("can increase bond");
		assert_ok!(Cohorts::bid(
			RuntimeOrigin::signed(2),
			OpaquePeerId::default(),
			rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
			Some(bond_2),
			RewardDestination::Owner
		));

		System::set_block_number(8);

		Cohorts::on_initialize(8);

		System::assert_last_event(
			Event::NewValidators {
				start_index: 2,
				new_validators: BoundedVec::truncate_from(vec![ValidatorRegistration {
					bond_id: Some(bond_2),
					account_id: 2,
					ownership_tokens: 1000u32.into(),
					bond_amount: 1010,
					reward_destination: RewardDestination::Owner,
					peer_id: PeerId(OpaquePeerId::default()),
					rpc_hosts: rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
				}]),
			}
			.into(),
		);

		let events = frame_system::Pallet::<Test>::events();
		// compare to the last event record
		let events = &events[events.len() - 4..]
			.into_iter()
			.map(|a| a.event.clone())
			.collect::<Vec<_>>();
		assert_eq!(
			events[0],
			UlixeeBalancesEvent::<Test, UlixeeToken>::Endowed {
				account: 3,
				free_balance: 1000u32.into()
			}
			.into()
		);

		assert_eq!(
			events[2],
			Event::<Test>::UnbondedValidator {
				account_id: 3,
				bond_id: Some(bond_3),
				kept_ownership_bond: false
			}
			.into()
		);
		assert_eq!(UlixeeBalances::free_balance(&2), 0);
		assert_eq!(UlixeeBalances::total_balance(&2), 1000);
		assert_eq!(ArgonBalances::free_balance(&2), 8990);

		let bond2 = <Bonds as BondProvider>::get_bond(bond_2).expect("bond should exist");
		assert_eq!(bond2.is_locked, true);

		assert_eq!(UlixeeBalances::free_balance(&3), 1000);
		assert_eq!(ArgonBalances::free_balance(&3), 8997);
		let bond3 = <Bonds as BondProvider>::get_bond(bond_3).expect("bond should exist");
		assert_eq!(bond3.is_locked, false);

		assert!(System::account_exists(&0));
		assert!(System::account_exists(&1));
		assert!(System::account_exists(&2));
		assert!(System::account_exists(&3));
	});
}

#[test]
fn it_can_take_cohort_bids() {
	BlocksBetweenCohorts::set(3);
	MaxValidators::set(6);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(3);

		assert_err!(
			Cohorts::bid(
				RuntimeOrigin::signed(2),
				OpaquePeerId::default(),
				rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
				None,
				RewardDestination::Owner
			),
			Error::<Test>::CohortNotTakingBids
		);

		IsCohortAcceptingBids::<Test>::set(true);

		set_ownership(3, 5000u32.into());
		Cohorts::on_initialize(3);
		assert_eq!(OwnershipBondAmount::<Test>::get(), 666);

		assert_err!(
			Cohorts::bid(
				RuntimeOrigin::signed(1),
				OpaquePeerId::default(),
				rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
				None,
				RewardDestination::Owner
			),
			Error::<Test>::InsufficientOwnershipTokens
		);

		set_ownership(1, 1000u32.into());

		assert_ok!(Cohorts::bid(
			RuntimeOrigin::signed(1),
			OpaquePeerId::default(),
			rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
			None,
			RewardDestination::Owner
		));
		System::assert_last_event(
			Event::CohortRegistrantAdded { account_id: 1, bid_amount: 0u32.into(), index: 0 }
				.into(),
		);
		assert_eq!(UlixeeBalances::free_balance(&1), 334);

		// should be able to re-register
		assert_ok!(Cohorts::bid(
			RuntimeOrigin::signed(1),
			OpaquePeerId::default(),
			rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
			None,
			RewardDestination::Owner
		));
		assert_eq!(
			NextCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![1]
		);
		assert_eq!(UlixeeBalances::free_balance(&1), 334, "should not alter reserved balance");
		assert_eq!(UlixeeBalances::total_balance(&1), 1000, "should still have their full balance");

		assert!(System::account_exists(&1));
		assert!(System::account_exists(&3));
	});
}

#[test]
fn it_wont_let_you_use_ownership_shares_for_two_bids() {
	BlocksBetweenCohorts::set(4);
	MaxValidators::set(6);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(3);

		IsCohortAcceptingBids::<Test>::set(true);

		assert_eq!(Cohorts::get_next_cohort_period(), (4, 4 + (4 * 3)));
		assert_eq!(Cohorts::get_next_start_cohort_index(), 2);
		set_ownership(2, 100u32.into());
		set_ownership(1, 100u32.into());
		Cohorts::on_initialize(3);

		assert_ok!(Cohorts::bid(
			RuntimeOrigin::signed(1),
			OpaquePeerId::default(),
			rpc_hosts(Ipv4Addr::new(192, 255, 255, 255), 15555),
			None,
			RewardDestination::Owner
		));
		System::set_block_number(4);
		Cohorts::on_initialize(4);

		System::assert_last_event(
			Event::NewValidators {
				start_index: 2,
				new_validators: BoundedVec::truncate_from(vec![ValidatorRegistration {
					account_id: 1,
					peer_id: PeerId(OpaquePeerId::default()),
					bond_id: None,
					ownership_tokens: 26u32.into(),
					bond_amount: 0,
					reward_destination: RewardDestination::Owner,
					rpc_hosts: rpc_hosts(Ipv4Addr::new(192, 255, 255, 255), 15555),
				}]),
			}
			.into(),
		);
		assert_eq!(OwnershipBondAmount::<Test>::get(), 26);
		assert_eq!(Cohorts::get_next_cohort_period(), (8, 8 + (4 * 3)));
		assert_eq!(Cohorts::get_next_start_cohort_index(), 4);

		System::set_block_number(6);
		Cohorts::on_initialize(6);
		assert_eq!(Cohorts::get_next_cohort_period(), (8, 8 + (4 * 3)));
		assert_eq!(Cohorts::get_next_start_cohort_index(), 4);

		assert_err!(
			Cohorts::bid(
				RuntimeOrigin::signed(1),
				OpaquePeerId::default(),
				rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
				None,
				RewardDestination::Owner
			),
			Error::<Test>::CannotRegisteredOverlappingSessions
		);
		assert!(System::account_exists(&1));
		assert!(System::account_exists(&2));
	})
}

#[test]
fn it_will_order_bids_with_argon_bonds() {
	BlocksBetweenCohorts::set(3);
	MaxValidators::set(6);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(3);

		assert_err!(
			Cohorts::bid(
				RuntimeOrigin::signed(2),
				OpaquePeerId::default(),
				rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
				None,
				RewardDestination::Owner
			),
			Error::<Test>::CohortNotTakingBids
		);

		IsCohortAcceptingBids::<Test>::set(true);

		set_ownership(1, 1000u32.into());
		set_argons(1, 10_000u32.into());
		set_ownership(2, 1000u32.into());
		set_argons(2, 10_000u32.into());
		set_ownership(3, 1000u32.into());
		set_argons(3, 10_000u32.into());

		Cohorts::on_initialize(3);
		assert_eq!(OwnershipBondAmount::<Test>::get(), 400);

		// 1. Account 1 bids
		let bond_until_block = Cohorts::get_next_cohort_period().1;
		let acc_1_bond_id =
			match <Bonds as BondProvider>::bond_self(1, 1000u32.into(), bond_until_block) {
				Ok(id) => id,
				Err(_) => panic!("bond should exist"),
			};

		assert_ok!(Cohorts::bid(
			RuntimeOrigin::signed(1),
			OpaquePeerId::default(),
			rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
			Some(acc_1_bond_id),
			RewardDestination::Owner
		));
		System::assert_last_event(
			Event::CohortRegistrantAdded { account_id: 1, bid_amount: 1000u32.into(), index: 0 }
				.into(),
		);
		assert_eq!(UlixeeBalances::free_balance(&1), 600);

		let bond = <Bonds as BondProvider>::get_bond(acc_1_bond_id).expect("bond should exist");
		assert_eq!(bond.is_locked, true);

		// 2. Account 2 bids highest and takes top slot
		let acc_2_bond_id =
			<Bonds as BondProvider>::bond_self(2, 1_001_u32.into(), bond_until_block).ok();

		// should be able to re-register
		assert_ok!(Cohorts::bid(
			RuntimeOrigin::signed(2),
			OpaquePeerId::default(),
			rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
			acc_2_bond_id,
			RewardDestination::Owner
		));
		System::assert_last_event(
			Event::CohortRegistrantAdded { account_id: 2, bid_amount: 1001u32.into(), index: 0 }
				.into(),
		);

		assert_eq!(
			NextCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![2, 1]
		);

		// 3. Account 2 bids above 1
		let acc_3_bond_id =
			<Bonds as BondProvider>::bond_self(3, 1_001_u32.into(), bond_until_block).ok();

		// should be able to re-register
		assert_ok!(Cohorts::bid(
			RuntimeOrigin::signed(3),
			OpaquePeerId::default(),
			rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
			acc_3_bond_id,
			RewardDestination::Owner
		));
		System::assert_last_event(
			Event::CohortRegistrantAdded { account_id: 3, bid_amount: 1001u32.into(), index: 1 }
				.into(),
		);

		assert_eq!(
			NextCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![2, 3]
		);

		let bond_1 = <Bonds as BondProvider>::get_bond(acc_1_bond_id).expect("bond should exist");
		assert_eq!(bond_1.is_locked, false, "bond should be unlocked");
		assert_eq!(UlixeeBalances::free_balance(&1), 1000);
		assert_eq!(ArgonBalances::free_balance(&1), 9000); // should still be locked up

		// 4. Account 1 increases bid and resubmits
		<Bonds as BondProvider>::extend_bond(acc_1_bond_id, 1, 1002, bond_until_block)
			.expect("can increse bond");
		assert_ok!(Cohorts::bid(
			RuntimeOrigin::signed(1),
			OpaquePeerId::default(),
			rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
			Some(acc_1_bond_id),
			RewardDestination::Owner
		));

		let events = frame_system::Pallet::<Test>::events();
		// compare to the last event record
		let frame_system::EventRecord { event, .. } = &events[events.len() - 2];
		assert_eq!(
			event,
			&<Test as frame_system::Config>::RuntimeEvent::from(
				Event::<Test>::CohortRegistrantReplaced {
					bond_id: acc_3_bond_id,
					account_id: 3u64,
					kept_ownership_bond: false
				}
			)
		);
		assert_eq!(ArgonBalances::free_balance(&1), 8998); // should still be locked up

		System::assert_last_event(
			Event::CohortRegistrantAdded { account_id: 1, bid_amount: 1002u32.into(), index: 0 }
				.into(),
		);
		assert_eq!(UlixeeBalances::free_balance(&3), 1000);

		assert_eq!(
			NextCohort::<Test>::get().iter().map(|a| a.account_id).collect::<Vec<_>>(),
			vec![1, 2]
		);
		assert!(System::account_exists(&1));
		assert!(System::account_exists(&2));
		assert!(System::account_exists(&3));
	});
}
#[test]
fn handles_a_max_of_bids_per_block() {
	BlocksBetweenCohorts::set(1440);
	MaxValidators::set(50);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		IsCohortAcceptingBids::<Test>::set(true);

		for i in 1..5 {
			set_ownership(i, 1000u32.into());
		}

		assert_ok!(Cohorts::bid(
			RuntimeOrigin::signed(1),
			OpaquePeerId::default(),
			rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
			None,
			RewardDestination::Owner
		));
		System::assert_last_event(
			Event::CohortRegistrantAdded { account_id: 1, bid_amount: 0u32.into(), index: 0 }
				.into(),
		);
		assert_ok!(Cohorts::bid(
			RuntimeOrigin::signed(2),
			OpaquePeerId::default(),
			rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
			None,
			RewardDestination::Owner
		));
		System::assert_last_event(
			Event::CohortRegistrantAdded { account_id: 2, bid_amount: 0u32.into(), index: 1 }
				.into(),
		);
		assert_noop!(
			Cohorts::bid(
				RuntimeOrigin::signed(3),
				OpaquePeerId::default(),
				rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
				None,
				RewardDestination::Owner
			),
			Error::<Test>::BidTooLow,
		);
		// should not have changed
		System::assert_last_event(
			Event::CohortRegistrantAdded { account_id: 2, bid_amount: 0u32.into(), index: 1 }
				.into(),
		);
		for i in 1..5 {
			assert!(System::account_exists(&i));
		}
	});
}

#[test]
fn it_handles_null_authority() {
	new_test_ext().execute_with(|| {
		assert_eq!(Cohorts::get_authority(1), None);
	});
}

#[test]
fn it_can_find_xor_closest() {
	MaxValidators::set(100);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(8);

		ActiveValidatorsByIndex::<Test>::try_mutate(|validators| {
			for i in 0..100u32 {
				let account_id: u64 = i.into();
				let _ = validators.try_insert(
					i,
					ValidatorRegistration {
						account_id,
						peer_id: empty_peer(),
						bond_id: None,
						ownership_tokens: 0,
						bond_amount: 0,
						reward_destination: RewardDestination::Owner,
						rpc_hosts: rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 15550),
					},
				);
				AccountIndexLookup::<Test>::insert(account_id, i);
			}
			Ok::<(), Error<Test>>(())
		})
		.expect("Didn't insert validators");
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
			Cohorts::find_xor_closest_authorities(U256::from(10), 5)
				.into_iter()
				.map(|a| a.authority_index)
				.collect::<Vec<_>>(),
			vec![10, 11, 8, 9, 14]
		);
		assert_eq!(
			Cohorts::find_xor_closest_authorities(U256::from(99), 5)
				.into_iter()
				.map(|a| a.authority_index)
				.collect::<Vec<_>>(),
			vec![99, 98, 97, 96, 67]
		);
	});
}

#[test]
fn it_can_replace_authority_keys() {
	MaxValidators::set(10);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(8);

		ActiveValidatorsByIndex::<Test>::try_mutate(|validators| {
			for i in 0..10u32 {
				let account_id: u64 = i.into();
				let _ = validators.try_insert(
					i,
					ValidatorRegistration {
						account_id,
						peer_id: empty_peer(),
						bond_id: None,
						ownership_tokens: 0,
						bond_amount: 0,
						reward_destination: RewardDestination::Owner,
						rpc_hosts: rpc_hosts(Ipv4Addr::new(127, 0, 0, 1), 3000),
					},
				);
				AccountIndexLookup::<Test>::insert(account_id, i);
			}
			Ok::<(), Error<Test>>(())
		})
		.expect("Validators failed to insert");

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
			Box::new(account_keys.iter().filter_map(|k| Some((&k.0, k.clone().1))));

		let queued_keys: Box<dyn Iterator<Item = _>> =
			Box::new(account_keys.iter().filter_map(|k| Some((&k.0, k.clone().1))));

		assert_eq!(AuthoritiesByIndex::<Test>::get().len(), 1);
		Cohorts::on_new_session(true, with_keys, queued_keys);
		assert_eq!(AuthoritiesByIndex::<Test>::get().len(), 8, "should register only 8 keys");
		assert_eq!(
			AuthoritiesByIndex::<Test>::get().contains_key(&0u32),
			false,
			"should not have index 0"
		);
		assert_eq!(
			AuthoritiesByIndex::<Test>::get().contains_key(&10u32),
			false,
			"should not have index 11"
		);
	});
}

fn empty_peer() -> PeerId {
	PeerId(OpaquePeerId::default())
}
fn rpc_hosts<S>(ip: Ipv4Addr, port: u16) -> BoundedVec<Host, S>
where
	S: sp_core::Get<u32>,
{
	bounded_vec![Host { ip: ip_to_u32(ip), port, is_secure: false }]
}
