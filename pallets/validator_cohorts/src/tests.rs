use frame_support::{
	assert_noop, assert_ok, assert_storage_noop,
	traits::{OnInitialize, OneSessionHandler},
};
use sp_core::{OpaquePeerId, U256};
use sp_runtime::{testing::UintAuthorityId, BoundedVec};

use ulx_primitives::{AuthorityProvider, BlockSealAuthorityId, PeerId, ValidatorRegistration};

use crate::{
	mock::*,
	pallet::{AccountIndexLookup, ActiveValidatorsByIndex, AuthoritiesByIndex},
	Error, Event, QueuedCohorts,
};

#[test]
fn it_doesnt_add_cohorts_until_time() {
	BlocksBetweenCohorts::set(2);

	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		QueuedCohorts::<Test>::set(
			1,
			BoundedVec::truncate_from(vec![ValidatorRegistration {
				account_id: 1,
				peer_id: PeerId(OpaquePeerId::default()),
			}]),
		);

		assert_storage_noop!(ValidatorCohorts::on_initialize(1));
	});
}

#[test]
fn starting_cohort_index() {
	let max_cohort_size = 3;
	let max_validators = 12;
	let blocks_between_cohorts = 5;

	assert_eq!(
		ValidatorCohorts::get_start_cohort_index(
			0,
			blocks_between_cohorts,
			max_validators,
			max_cohort_size
		),
		0
	);
	assert_eq!(
		ValidatorCohorts::get_start_cohort_index(
			5,
			blocks_between_cohorts,
			max_validators,
			max_cohort_size
		),
		3
	);
	assert_eq!(
		ValidatorCohorts::get_start_cohort_index(
			10,
			blocks_between_cohorts,
			max_validators,
			max_cohort_size
		),
		6
	);
	assert_eq!(
		ValidatorCohorts::get_start_cohort_index(
			15,
			blocks_between_cohorts,
			max_validators,
			max_cohort_size
		),
		9
	);

	assert_eq!(
		ValidatorCohorts::get_start_cohort_index(
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
				let _ = validators
					.try_insert(i, ValidatorRegistration { account_id, peer_id: empty_peer() });
				AccountIndexLookup::<Test>::insert(account_id, i);
			}
			Ok::<(), Error<Test>>(())
		})
		.expect("Didn't insert validators");

		let cohort = BoundedVec::truncate_from(vec![ValidatorRegistration {
			account_id: 1,
			peer_id: empty_peer(),
		}]);

		QueuedCohorts::<Test>::set(8, cohort.clone());

		ValidatorCohorts::on_initialize(8);

		// re-fetch
		let validators = ActiveValidatorsByIndex::<Test>::get();
		assert_eq!(
			QueuedCohorts::<Test>::contains_key(8),
			false,
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
			Event::NewValidators { cohort_size: 2, start_index: 2, new_validators: cohort }.into(),
		)
	});
}

#[test]
fn it_can_reserve_validation_slots() {
	BlocksBetweenCohorts::set(3);
	MaxValidators::set(6);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(3);
		assert_noop!(
			ValidatorCohorts::reserve(RuntimeOrigin::signed(1), 1, OpaquePeerId::default()),
			Error::<Test>::CohortBlockNumberNotAnEntrypoint
		);
		assert_noop!(
			ValidatorCohorts::reserve(RuntimeOrigin::signed(1), 3, OpaquePeerId::default()),
			Error::<Test>::CohortBlockTooOld
		);

		assert_ok!(ValidatorCohorts::reserve(RuntimeOrigin::signed(1), 6, OpaquePeerId::default()));
		assert_eq!(QueuedCohorts::<Test>::contains_key(6), true);
		assert_eq!(QueuedCohorts::<Test>::get(6).len(), 1);
	});
}

#[test]
fn handles_a_max_of_reservations() {
	BlocksBetweenCohorts::set(1440);
	MaxValidators::set(50);
	MaxCohortSize::set(2);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(ValidatorCohorts::reserve(
			RuntimeOrigin::signed(1),
			1440,
			OpaquePeerId::default()
		));
		assert_ok!(ValidatorCohorts::reserve(
			RuntimeOrigin::signed(2),
			1440,
			OpaquePeerId::default()
		));
		// can re-insert same validator
		assert_ok!(ValidatorCohorts::reserve(
			RuntimeOrigin::signed(2),
			1440,
			OpaquePeerId::default()
		));
		assert_noop!(
			ValidatorCohorts::reserve(RuntimeOrigin::signed(3), 1440, OpaquePeerId::default()),
			Error::<Test>::TooManyBlockRegistrants
		);
	});
}

#[test]
fn it_handles_null_authority() {
	new_test_ext().execute_with(|| {
		assert_eq!(ValidatorCohorts::get_authority(1), None);
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
				let _ = validators
					.try_insert(i, ValidatorRegistration { account_id, peer_id: empty_peer() });
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
			ValidatorCohorts::find_xor_closest_authorities(U256::from(10), 5)
				.into_iter()
				.map(|a| a.authority_index)
				.collect::<Vec<_>>(),
			vec![10, 11, 8, 9, 14]
		);
		assert_eq!(
			ValidatorCohorts::find_xor_closest_authorities(U256::from(99), 5)
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
				let _ = validators
					.try_insert(i, ValidatorRegistration { account_id, peer_id: empty_peer() });
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
		ValidatorCohorts::on_new_session(true, with_keys, queued_keys);
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
