use pallet_prelude::*;

use crate::{mock::*, Error, Event, FissionByOwnerAndId, FissionIdsByLockId, NextFissionIdByOwner};
use argon_primitives::BitcoinFissionsProvider;

fn lock(owner: u64, funded_satoshis: u64) -> MockLock {
	MockLock { owner, funded_satoshis, fissioned_satoshis: 0, microgons_at_target_per_btc: 100 }
}

#[test]
fn create_stores_one_lock_allocation_with_an_opaque_liquid_id() {
	new_test_ext().execute_with(|| {
		System::set_block_number(7);
		MockLocks::insert(1, lock(1, 100));

		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 10, 77, 1, 40, 90));

		let fission = FissionByOwnerAndId::<Test>::get(1, 10).expect("fission");
		assert_eq!(fission.liquid_id, 77);
		assert_eq!(fission.utxo_id, 1);
		assert_eq!(fission.satoshis, 40);
		assert_eq!(fission.microgons_at_target_per_btc, 90);
		assert_eq!(fission.last_ratchet_tick, 1);
		assert_eq!(fission.liquidity_promised, 3_600);
		assert_eq!(fission.created_at_argon_block, 7);
		assert_eq!(fission.ratchet_number, 0);
		assert_eq!(fission.last_updated_argon_block, 7);
		assert_eq!(MockLocks::get(1).expect("lock").fissioned_satoshis, 40);
		assert!(FissionIdsByLockId::<Test>::get(1).contains(&10));
		assert_eq!(NextFissionIdByOwner::<Test>::get(1), 11);
		assert_eq!(MockMintRequests::get(), vec![(1, 10, 1, 3_600)]);
		assert_eq!(MockAccountBitcoinChanges::get(), vec![(1, 3_600, true)]);
		System::assert_last_event(
			Event::FissionCreated {
				account_id: 1,
				fission_id: 10,
				liquid_id: 77,
				utxo_id: 1,
				satoshis: 40,
				microgons_at_target_per_btc: 90,
				liquidity_promised: 3_600,
			}
			.into(),
		);
	});
}

#[test]
fn create_allows_gaps_but_rejects_ids_below_the_owner_floor() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		MockLocks::insert(2, lock(1, 100));

		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 10, 77, 1, 20, 90));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 20, 77, 2, 20, 90));

		assert_noop!(
			BitcoinFissions::create(RuntimeOrigin::signed(1), 15, 78, 1, 20, 90),
			Error::<Test>::FissionIdBelowMinimum
		);
		assert_eq!(NextFissionIdByOwner::<Test>::get(1), 21);
	});
}

#[test]
fn create_rolls_back_the_lock_index_when_allocation_fails() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 10));

		assert_noop!(
			BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 20, 90),
			Error::<Test>::InsufficientFundedSatoshis
		);

		assert_eq!(MockLocks::get(1).expect("lock").fissioned_satoshis, 0);
		assert!(FissionIdsByLockId::<Test>::get(1).is_empty());
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert!(MockMintRequests::get().is_empty());
		assert_eq!(NextFissionIdByOwner::<Test>::get(1), 0);
	});
}

#[test]
fn create_limits_active_fissions_per_lock() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));

		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 1, 1, 10, 90));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 1, 1, 10, 90));
		assert_noop!(
			BitcoinFissions::create(RuntimeOrigin::signed(1), 2, 1, 1, 10, 90),
			Error::<Test>::TooManyFissionsForLock
		);
	});
}

#[test]
fn close_removes_the_fission_and_preserves_its_mint_entitlement() {
	new_test_ext().execute_with(|| {
		System::set_block_number(7);
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90));

		RedemptionMicrogonsPerSatoshi::set(50);
		System::set_block_number(12);
		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));

		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert_eq!(MockLocks::get(1).expect("lock").fissioned_satoshis, 0);
		assert!(FissionIdsByLockId::<Test>::get(1).is_empty());
		assert_eq!(Balances::free_balance(1), 18_000);
		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600)]);
		assert_eq!(MockFissionRedemptionBurns::get(), 2_000);
		assert_eq!(MockAccountBitcoinChanges::get(), vec![(1, 3_600, true), (1, 3_600, false)]);
		System::assert_last_event(
			Event::FissionClosed { account_id: 1, fission_id: 0, redemption_amount: 2_000 }.into(),
		);
	});
}

#[test]
fn close_rolls_back_when_the_owner_cannot_burn_the_redemption() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90));
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), 1, 100));

		assert_noop!(
			BitcoinFissions::close(RuntimeOrigin::signed(1), 0),
			Error::<Test>::InsufficientFunds
		);

		assert!(FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert_eq!(MockLocks::get(1).expect("lock").fissioned_satoshis, 40);
		assert!(FissionIdsByLockId::<Test>::get(1).contains(&0));
		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600)]);
	});
}

#[test]
fn close_uses_owner_scoped_lookup_and_removes_the_closed_fission() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90));

		assert_noop!(
			BitcoinFissions::close(RuntimeOrigin::signed(2), 0),
			Error::<Test>::FissionNotFound
		);
		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert_noop!(
			BitcoinFissions::close(RuntimeOrigin::signed(1), 0),
			Error::<Test>::FissionNotFound
		);
	});
}

#[test]
fn an_external_lock_spend_closes_only_active_fissions_and_preserves_their_unpaid_mints() {
	new_test_ext().execute_with(|| {
		System::set_block_number(7);
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 77, 1, 10, 90));
		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));

		System::set_block_number(12);
		assert_ok!(<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::close_for_lock(
			&1, 1, 0,
		));

		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600), (1, 1, 1, 900)]);
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 1));
		assert!(FissionIdsByLockId::<Test>::get(1).is_empty());
		assert_eq!(
			MockAccountBitcoinChanges::get(),
			vec![(1, 3_600, true), (1, 900, true), (1, 3_600, false), (1, 900, false),]
		);
		System::assert_last_event(
			Event::FissionClosedByLock { account_id: 1, fission_id: 1, utxo_id: 1 }.into(),
		);
	});
}

#[test]
fn an_approved_lock_release_closes_a_migrated_fission_and_preserves_its_mints() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90));

		assert_ok!(<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::close_for_lock(
			&1, 1, 3_600,
		));

		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600)]);
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert!(FissionIdsByLockId::<Test>::get(1).is_empty());
	});
}

#[test]
fn up_ratchet_updates_one_fission_when_threshold_and_lock_coverage_allow_it() {
	new_test_ext().execute_with(|| {
		System::set_block_number(7);
		MockLocks::insert(1, MockLock { microgons_at_target_per_btc: 120, ..lock(1, 100) });
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90));

		System::set_block_number(12);
		LiquidityMultiplierPercent::set(110);
		MockLastRatchetTick::set(2);
		assert_ok!(BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 110));

		let fission = FissionByOwnerAndId::<Test>::get(1, 0).expect("fission");
		assert_eq!(fission.microgons_at_target_per_btc, 110);
		assert_eq!(fission.last_ratchet_tick, 2);
		assert_eq!(fission.liquidity_promised, 4_840);
		assert_eq!(fission.ratchet_number, 1);
		assert_eq!(fission.last_updated_argon_block, 12);
		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600), (1, 0, 1, 1_240)]);
		assert_eq!(MockAccountBitcoinChanges::get(), vec![(1, 3_600, true), (1, 1_240, true)]);
		System::assert_last_event(
			Event::FissionRatcheted {
				account_id: 1,
				fission_id: 0,
				ratchet_number: 1,
				microgons_at_target_per_btc: 110,
				liquidity_promised: 4_840,
				amount_minted: 1_240,
				amount_burned: 0,
			}
			.into(),
		);
	});
}

#[test]
fn down_ratchet_burns_and_requeues_the_replacement_liability() {
	new_test_ext().execute_with(|| {
		System::set_block_number(7);
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90));

		System::set_block_number(12);
		MockLastRatchetTick::set(2);
		assert_ok!(BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 80));

		let fission = FissionByOwnerAndId::<Test>::get(1, 0).expect("fission");
		assert_eq!(fission.microgons_at_target_per_btc, 80);
		assert_eq!(fission.last_ratchet_tick, 2);
		assert_eq!(fission.liquidity_promised, 3_200);
		assert_eq!(fission.ratchet_number, 1);
		assert_eq!(fission.last_updated_argon_block, 12);
		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600), (1, 0, 1, 3_200)]);
		assert_eq!(MockFissionRedemptionBurns::get(), 3_200);
		assert_eq!(MockAccountBitcoinChanges::get(), vec![(1, 3_600, true), (1, 400, false)]);
		assert_eq!(Balances::free_balance(1), 16_800);
		System::assert_last_event(
			Event::FissionRatcheted {
				account_id: 1,
				fission_id: 0,
				ratchet_number: 1,
				microgons_at_target_per_btc: 80,
				liquidity_promised: 3_200,
				amount_minted: 3_200,
				amount_burned: 3_200,
			}
			.into(),
		);
	});
}

#[test]
fn registration_provider_sums_active_fission_liability() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 77, 1, 10, 90));

		assert_eq!(
			<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::get_account_fission_liquidity(
				&1
			),
			4_500
		);

		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));
		assert_eq!(
			<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::get_account_fission_liquidity(
				&1
			),
			900
		);
	});
}

#[test]
fn ratchet_rejects_a_removed_fission_or_an_ineligible_change_without_writes() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90));
		let original = FissionByOwnerAndId::<Test>::get(1, 0).expect("fission");

		assert_noop!(
			BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 95),
			Error::<Test>::NoRatchetingAvailable
		);
		MockLastRatchetTick::set(2);
		assert_noop!(
			BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 110),
			Error::<Test>::NoRatchetingAvailable
		);
		assert_eq!(FissionByOwnerAndId::<Test>::get(1, 0), Some(original));

		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));
		assert_noop!(
			BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 80),
			Error::<Test>::FissionNotFound
		);
	});
}

#[test]
fn down_ratchet_rolls_back_when_the_owner_cannot_burn_the_replacement_liability() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90));
		let original = FissionByOwnerAndId::<Test>::get(1, 0).expect("fission");
		let original_mint_requests = MockMintRequests::get();
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), 1, 100));
		MockLastRatchetTick::set(2);

		assert_noop!(
			BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 80),
			Error::<Test>::InsufficientFunds
		);

		assert_eq!(FissionByOwnerAndId::<Test>::get(1, 0), Some(original));
		assert_eq!(MockMintRequests::get(), original_mint_requests);
		assert_eq!(Balances::free_balance(1), 100);
	});
}
