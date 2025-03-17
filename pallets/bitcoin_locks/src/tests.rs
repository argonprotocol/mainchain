use frame_support::{
	assert_err, assert_ok,
	pallet_prelude::*,
	traits::fungible::{Inspect, InspectHold, Mutate},
};
use sp_arithmetic::{traits::Zero, FixedI128, FixedPointNumber, FixedU128};

use crate::{
	mock::*,
	pallet::{
		LockReleaseCosignHeightById, LocksByUtxoId, LocksPendingReleaseByUtxoId,
		ObligationIdToUtxoId, OwedUtxoAggrieved,
	},
	Error, Event, HoldReason, LockReleaseRequest,
};
use argon_primitives::{
	bitcoin::{
		BitcoinRejectedReason, BitcoinScriptPubkey, BitcoinSignature, CompressedBitcoinPubkey,
		H256Le, UtxoRef, SATOSHIS_PER_BITCOIN,
	},
	BitcoinUtxoEvents, PriceProvider,
};

#[test]
fn can_lock_a_bitcoin_utxo() {
	BitcoinBlockHeight::set(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		set_argons(2, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_err!(
			BitcoinLocks::initialize(RuntimeOrigin::signed(2), 1, 1_000_000, pubkey),
			Error::<Test>::InsufficientSatoshisLocked
		);
		assert_ok!(BitcoinLocks::initialize(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.obligation_id, 1);
		assert_eq!(lock.owner_account, 2);
		assert!(!lock.is_verified);
		let lock_price = StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(lock.lock_price, lock_price);
		assert_eq!(WatchedUtxosById::get().len(), 1);
		System::assert_last_event(
			Event::<Test>::BitcoinLockCreated {
				utxo_id: 1,
				obligation_id: 1,
				vault_id: 1,
				lock_price,
				account_id: 2,
			}
			.into(),
		);

		assert_ok!(BitcoinLocks::utxo_expired(1));
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
	});
}

#[test]
fn cleans_up_a_rejected_bitcoin() {
	BitcoinBlockHeight::set(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);
		CurrentTick::set(1);

		assert_ok!(BitcoinLocks::initialize(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		let price = StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().locked_bitcoin_argons.reserved, price);

		assert_ok!(BitcoinLocks::utxo_rejected(1, BitcoinRejectedReason::LookupExpired));
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(ObligationIdToUtxoId::<Test>::get(1), None);
		assert_eq!(WatchedUtxosById::get().len(), 0);
	});
}

#[test]
fn allows_users_to_reclaim_mismatched_bitcoins() {
	BitcoinBlockHeight::set(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let secp = bitcoin::secp256k1::Secp256k1::new();
		let rng = &mut rand::thread_rng();
		let keypair = bitcoin::secp256k1::SecretKey::new(rng);
		let pubkey = keypair.public_key(&secp).serialize();
		CurrentTick::set(1);

		assert_ok!(BitcoinLocks::initialize(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey.into()
		));
		let price = StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().locked_bitcoin_argons.reserved, price);

		assert_ok!(BitcoinLocks::utxo_rejected(1, BitcoinRejectedReason::SatoshisMismatch));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert!(lock.is_rejected_needs_release);

		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			release_script_pubkey.clone(),
			1000
		));

		assert_eq!(
			LocksPendingReleaseByUtxoId::<Test>::get().get(&1),
			Some(LockReleaseRequest {
				utxo_id: 1,
				vault_id: 1,
				obligation_id: 1,
				cosign_due_block: BitcoinBlockHeight::get() +
					LockReleaseCosignDeadlineBlocks::get(),
				redemption_price: 0,
				to_script_pubkey: release_script_pubkey,
				bitcoin_network_fee: 1000
			})
			.as_ref()
		);
		assert!(LocksByUtxoId::<Test>::get(1).is_some());
		System::assert_last_event(
			Event::<Test>::BitcoinUtxoCosignRequested { obligation_id: 1, vault_id: 1, utxo_id: 1 }
				.into(),
		);

		GetUtxoRef::set(Some(UtxoRef { txid: H256Le([0; 32]), output_index: 0 }));

		assert_ok!(BitcoinLocks::cosign_release(
			RuntimeOrigin::signed(1),
			1,
			BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec()))
		));
		assert_eq!(LastReleaseEvent::get(), Some((1, false, 0)));
		assert_eq!(LocksPendingReleaseByUtxoId::<Test>::get().get(&1), None);
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(OwedUtxoAggrieved::<Test>::get(1), None);
		assert_eq!(DefaultVault::get().locked_bitcoin_argons.reserved, 0);
	});
}

#[test]
fn marks_a_verified_bitcoin() {
	BitcoinBlockHeight::set(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::initialize(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		let price = StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().locked_bitcoin_argons.reserved, price);

		assert_ok!(BitcoinLocks::utxo_verified(1));
		assert!(LocksByUtxoId::<Test>::get(1).unwrap().is_verified);
		assert_eq!(WatchedUtxosById::get().len(), 1);
		assert_eq!(LastLockEvent::get(), Some((1, who, price)));
	});
}

#[test]
fn calculates_redemption_prices() {
	new_test_ext().execute_with(|| {
		BitcoinPricePerUsd::set(Some(FixedU128::saturating_from_integer(50000)));
		ArgonPricePerUsd::set(Some(FixedU128::saturating_from_integer(1)));
		ArgonCPI::set(Some(FixedI128::zero()));
		{
			let new_price =
				BitcoinLocks::get_redemption_price(&100_000_000).expect("should have price");
			assert_eq!(new_price, 50_000_000_000);
		}
		ArgonPricePerUsd::set(Some(FixedU128::from_float(1.01)));
		{
			let new_price =
				BitcoinLocks::get_redemption_price(&100_000_000).expect("should have price");
			assert_eq!(new_price, (50_000_000_000f64 / 1.01f64) as u128);
		}
		ArgonCPI::set(Some(FixedI128::from_float(0.1)));
		{
			let new_price =
				BitcoinLocks::get_redemption_price(&100_000_000).expect("should have price");
			// round to 3 digit precision for multiplier
			let multiplier = 0.713 * 1.01 + 0.274;
			// NOTE: floating point yields different rounding - might need to modify if you change
			// values
			assert_eq!(new_price, (multiplier * (50_000_000_000.0 / 1.01)) as u128);
		}
	});
}

#[test]
fn burns_a_spent_bitcoin() {
	BitcoinBlockHeight::set(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let allocated = DefaultVault::get().locked_bitcoin_argons.allocated;
		CurrentTick::set(1);

		assert_ok!(BitcoinLocks::initialize(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		let expiration_block = BitcoinBlockHeight::get() + LockDurationBlocks::get();

		let price = StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().locked_bitcoin_argons.reserved, price);
		// first verify
		assert_ok!(BitcoinLocks::utxo_verified(1));

		BitcoinPricePerUsd::set(Some(FixedU128::saturating_from_integer(50000)));

		let new_price =
			BitcoinLocks::get_redemption_price(&SATOSHIS_PER_BITCOIN).expect("should have price");
		// 50_000_000_000 microgons for a bitcoin
		// 50m * 0.987 = 49,350,000
		assert_eq!(new_price, 49_350_000_000);

		assert_ok!(BitcoinLocks::utxo_spent(1));
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(WatchedUtxosById::get().len(), 0);
		assert_eq!(DefaultVault::get().locked_bitcoin_argons.reserved, price - new_price);
		assert_eq!(DefaultVault::get().locked_bitcoin_argons.allocated, allocated - new_price);

		System::assert_last_event(
			Event::<Test>::BitcoinLockBurned {
				vault_id: 1,
				obligation_id: 1,
				utxo_id: 1,
				amount_burned: new_price,
				amount_held: new_price, /* NOTE: this is wrong but just how the mock is returning
				                         * the data */
				was_utxo_spent: true,
			}
			.into(),
		);
		BitcoinBlockHeight::set(expiration_block);
		BitcoinLocks::on_initialize(2);

		assert_eq!(LastReleaseEvent::get(), Some((1, true, new_price)));
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
	});
}

#[test]
fn cancels_an_unverified_spent_bitcoin() {
	BitcoinBlockHeight::set(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::initialize(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		assert_eq!(WatchedUtxosById::get().len(), 1);
		// spend before verify
		assert_ok!(BitcoinLocks::utxo_spent(1));

		assert_eq!(WatchedUtxosById::get().len(), 0);
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(CanceledObligations::get(), vec![1]);
	});
}

#[test]
fn can_release_a_bitcoin() {
	new_test_ext().execute_with(|| {
		BitcoinBlockHeight::set(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		set_argons(who, 2_000);
		assert_ok!(BitcoinLocks::initialize(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(DefaultVault::get().locked_bitcoin_argons.reserved, lock.lock_price);
		let expiration_block = lock.vault_claim_height;
		// first verify
		assert_ok!(BitcoinLocks::utxo_verified(1));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, lock.lock_price));

		BitcoinPricePerUsd::set(Some(FixedU128::from_float(65_000.00)));
		// now the user goes to release
		// 1. We would create a psbt and output address
		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		// must be the right user!
		assert_err!(
			BitcoinLocks::request_release(
				RuntimeOrigin::signed(2),
				1,
				release_script_pubkey.clone(),
				1000
			),
			Error::<Test>::NoPermissions
		);
		// must be before the cutoff
		BitcoinBlockHeight::set(expiration_block - 1);
		assert_err!(
			BitcoinLocks::request_release(
				RuntimeOrigin::signed(who),
				1,
				release_script_pubkey.clone(),
				1000
			),
			Error::<Test>::BitcoinReleaseInitiationDeadlinePassed
		);
		BitcoinBlockHeight::set(expiration_block - LockReleaseCosignDeadlineBlocks::get() - 1);
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			release_script_pubkey.clone(),
			1000
		));
		assert!(LocksByUtxoId::<Test>::get(1).is_some());
		let redemption_price =
			BitcoinLocks::get_redemption_price(&SATOSHIS_PER_BITCOIN).expect("should have price");
		assert!(redemption_price > lock.lock_price);
		// redemption price should be the lock price since current redemption price is above
		assert_eq!(
			LocksPendingReleaseByUtxoId::<Test>::get().get(&1),
			Some(LockReleaseRequest {
				utxo_id: 1,
				vault_id: 1,
				obligation_id: 1,
				cosign_due_block: BitcoinBlockHeight::get() +
					LockReleaseCosignDeadlineBlocks::get(),
				redemption_price: lock.lock_price,
				to_script_pubkey: release_script_pubkey,
				bitcoin_network_fee: 1000
			})
			.as_ref()
		);
		assert!(LocksByUtxoId::<Test>::get(1).is_some());
		System::assert_last_event(
			Event::<Test>::BitcoinUtxoCosignRequested { obligation_id: 1, vault_id: 1, utxo_id: 1 }
				.into(),
		);

		assert_eq!(Balances::free_balance(who), 2_000);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ReleaseBitcoinLock.into(), &who),
			lock.lock_price
		);
	});
}

#[test]
fn penalizes_vault_if_not_release_countersigned() {
	new_test_ext().execute_with(|| {
		BitcoinBlockHeight::set(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		let satoshis = SATOSHIS_PER_BITCOIN + 5000;
		set_argons(who, 2_000);
		assert_ok!(BitcoinLocks::initialize(RuntimeOrigin::signed(who), 1, satoshis, pubkey));
		let vault = DefaultVault::get();
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(vault.locked_bitcoin_argons.reserved, lock.lock_price);
		// first verify
		assert_ok!(BitcoinLocks::utxo_verified(1));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, lock.lock_price));
		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			release_script_pubkey.clone(),
			2000
		));
		assert!(LocksByUtxoId::<Test>::get(1).is_some());

		let redemption_price =
			BitcoinLocks::get_redemption_price(&satoshis).expect("should have price");
		let cosign_due = BitcoinBlockHeight::get() + LockReleaseCosignDeadlineBlocks::get();
		assert_eq!(
			LocksPendingReleaseByUtxoId::<Test>::get().get(&1),
			Some(LockReleaseRequest {
				utxo_id: 1,
				vault_id: 1,
				obligation_id: 1,
				cosign_due_block: cosign_due,
				redemption_price,
				to_script_pubkey: release_script_pubkey,
				bitcoin_network_fee: 2000
			})
			.as_ref()
		);

		BitcoinBlockHeight::set(cosign_due);
		System::set_block_number(2);
		BitcoinLocks::on_initialize(2);

		// should pay back at market price (not the discounted rate)
		let market_price =
			StaticPriceProvider::get_bitcoin_argon_price(satoshis).expect("should have price");
		assert_eq!(LocksPendingReleaseByUtxoId::<Test>::get().get(&1), None);
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(OwedUtxoAggrieved::<Test>::get(1), None);
		System::assert_last_event(
			Event::<Test>::BitcoinCosignPastDue {
				obligation_id: 1,
				vault_id: 1,
				utxo_id: 1,
				compensation_amount: redemption_price,
				compensation_still_owed: 0,
				compensated_account_id: who,
			}
			.into(),
		);
		assert_eq!(LastReleaseEvent::get(), Some((1, false, lock.lock_price)));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ReleaseBitcoinLock.into(), &who), 0);
		assert_eq!(Balances::balance(&who), 2000 + market_price);
	});
}

#[test]
fn clears_released_bitcoins() {
	new_test_ext().execute_with(|| {
		BitcoinBlockHeight::set(1);
		System::set_block_number(1);

		let secp = bitcoin::secp256k1::Secp256k1::new();
		let rng = &mut rand::thread_rng();
		let keypair = bitcoin::secp256k1::SecretKey::new(rng);
		let pubkey = keypair.public_key(&secp).serialize();
		let who = 2;
		let satoshis = SATOSHIS_PER_BITCOIN + 25000;
		set_argons(who, 2_000);
		assert_ok!(BitcoinLocks::initialize(
			RuntimeOrigin::signed(who),
			1,
			satoshis,
			pubkey.into()
		));
		let vault = DefaultVault::get();
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(vault.locked_bitcoin_argons.reserved, lock.lock_price);
		// first verify
		assert_ok!(BitcoinLocks::utxo_verified(1));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, lock.lock_price));
		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			release_script_pubkey.clone(),
			11
		));
		assert!(LocksByUtxoId::<Test>::get(1).is_some());

		let redemption_price =
			BitcoinLocks::get_redemption_price(&satoshis).expect("should have price");
		let cosign_due_block = BitcoinBlockHeight::get() + LockReleaseCosignDeadlineBlocks::get();
		assert_eq!(
			LocksPendingReleaseByUtxoId::<Test>::get().get(&1),
			Some(LockReleaseRequest {
				utxo_id: 1,
				vault_id: 1,
				obligation_id: 1,
				cosign_due_block,
				redemption_price,
				to_script_pubkey: release_script_pubkey,
				bitcoin_network_fee: 11
			})
			.as_ref()
		);
		assert_err!(
			BitcoinLocks::cosign_release(
				RuntimeOrigin::signed(2),
				1,
				BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec()))
			),
			Error::<Test>::NoPermissions
		);
		GetUtxoRef::set(Some(UtxoRef { txid: H256Le([0; 32]), output_index: 0 }));

		assert_ok!(BitcoinLocks::cosign_release(
			RuntimeOrigin::signed(1),
			1,
			BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec()))
		));
		assert_eq!(LastReleaseEvent::get(), Some((1, false, redemption_price)));
		assert_eq!(LocksPendingReleaseByUtxoId::<Test>::get().get(&1), None);
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(OwedUtxoAggrieved::<Test>::get(1), None);
		assert_eq!(DefaultVault::get().locked_bitcoin_argons.reserved, lock.lock_price);
		assert_eq!(
			DefaultVault::get().locked_bitcoin_argons.allocated,
			vault.locked_bitcoin_argons.allocated
		);
		assert_eq!(LockReleaseCosignHeightById::<Test>::get(1), Some(1));

		System::assert_last_event(
			Event::<Test>::BitcoinUtxoCosigned {
				obligation_id: 1,
				vault_id: 1,
				utxo_id: 1,
				signature: BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec())),
			}
			.into(),
		);

		assert_eq!(Balances::balance_on_hold(&HoldReason::ReleaseBitcoinLock.into(), &who), 0);
		assert_eq!(Balances::balance(&who), 2000 + lock.lock_price - redemption_price);
		assert_eq!(Balances::total_issuance(), 2000 + lock.lock_price - redemption_price);

		// should keep for the year
		System::set_block_number(2);
		BitcoinBlockHeight::set(lock.vault_claim_height);
		BitcoinLocks::on_initialize(2);
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_ok!(BitcoinLocks::utxo_spent(1));
	});
}

#[test]
fn it_should_aggregate_holds_for_a_second_release() {
	new_test_ext().execute_with(|| {
		BitcoinBlockHeight::set(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		let satoshis = 2 * SATOSHIS_PER_BITCOIN;
		set_argons(who, 2_000);
		assert_ok!(BitcoinLocks::initialize(RuntimeOrigin::signed(who), 1, satoshis, pubkey));
		assert_ok!(BitcoinLocks::initialize(RuntimeOrigin::signed(who), 1, satoshis, pubkey));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_ok!(BitcoinLocks::utxo_verified(1));
		assert_ok!(BitcoinLocks::utxo_verified(2));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, lock.lock_price * 2));
		let redemption_price =
			BitcoinLocks::get_redemption_price(&satoshis).expect("should have price");

		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			10
		));
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			2,
			make_script_pubkey(&[0; 32]),
			10
		));
		assert_eq!(Balances::free_balance(who), 2_000 + (2 * (lock.lock_price - redemption_price)));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ReleaseBitcoinLock.into(), &who),
			redemption_price * 2
		);
	});
}

fn make_script_pubkey(vec: &[u8]) -> BitcoinScriptPubkey {
	BitcoinScriptPubkey(BoundedVec::try_from(vec.to_vec()).unwrap())
}
