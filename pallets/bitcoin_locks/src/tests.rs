#![allow(clippy::zero_prefixed_literal)]
#![allow(clippy::multiple_bound_locations)]
#![allow(clippy::inconsistent_digit_grouping)]

use pallet_prelude::*;

use crate::{
	Error, Event, HoldReason, LockReleaseRequest,
	mock::*,
	pallet::{
		LockCosignDueByFrame, LockReleaseCosignHeightById, LockReleaseRequestsByUtxoId,
		LocksByUtxoId,
	},
};
use argon_primitives::{
	BitcoinUtxoEvents, MICROGONS_PER_ARGON, PriceProvider,
	bitcoin::{
		BitcoinRejectedReason, BitcoinScriptPubkey, BitcoinSignature, CompressedBitcoinPubkey,
		H256Le, SATOSHIS_PER_BITCOIN, UtxoRef,
	},
	vault::LockExtension,
};

#[test]
fn can_lock_a_bitcoin_utxo() {
	set_bitcoin_height(12);
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
		assert_eq!(lock.owner_account, 2);
		assert!(!lock.is_verified);
		let liquidity_promised = StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(lock.liquidity_promised, liquidity_promised);

		assert_eq!(WatchedUtxosById::get().len(), 1);
		System::assert_last_event(
			Event::<Test>::BitcoinLockCreated {
				utxo_id: 1,
				vault_id: 1,
				liquidity_promised,
				pegged_price: liquidity_promised,
				account_id: 2,
				security_fee: liquidity_promised / 10,
			}
			.into(),
		);

		assert_ok!(BitcoinLocks::utxo_expired(1));
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
	});
}

#[test]
fn cleans_up_a_rejected_bitcoin() {
	set_bitcoin_height(12);
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
		assert_eq!(DefaultVault::get().argons_locked, price);

		assert_ok!(BitcoinLocks::utxo_rejected(1, BitcoinRejectedReason::LookupExpired));
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(WatchedUtxosById::get().len(), 0);
	});
}

#[test]
fn allows_users_to_reclaim_mismatched_bitcoins() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let secp = bitcoin::secp256k1::Secp256k1::new();
		let rng = &mut bitcoin::secp256k1::rand::thread_rng();
		let keypair = bitcoin::secp256k1::SecretKey::new(rng);
		let pubkey = keypair.public_key(&secp).serialize();

		assert_ok!(BitcoinLocks::initialize(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey.into()
		));
		let price = StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().argons_locked, price);

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

		let cosign_due = CurrentFrameId::get() + LockReleaseCosignDeadlineFrames::get();
		assert_eq!(
			LockReleaseRequestsByUtxoId::<Test>::get(1).unwrap(),
			LockReleaseRequest {
				utxo_id: 1,
				vault_id: 1,
				cosign_due_frame: cosign_due,
				redemption_price: 0,
				to_script_pubkey: release_script_pubkey,
				bitcoin_network_fee: 1000
			}
		);
		assert!(LocksByUtxoId::<Test>::get(1).is_some());
		System::assert_last_event(
			Event::<Test>::BitcoinUtxoCosignRequested { vault_id: 1, utxo_id: 1 }.into(),
		);

		GetUtxoRef::set(Some(UtxoRef { txid: H256Le([0; 32]), output_index: 0 }));

		assert_ok!(BitcoinLocks::cosign_release(
			RuntimeOrigin::signed(1),
			1,
			BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec()))
		));
		assert_eq!(LastReleaseEvent::get(), Some((1, false, 0)));
		assert!(!LockCosignDueByFrame::<Test>::get(cosign_due).contains(&1));
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(DefaultVault::get().argons_locked, 0);
	});
}

#[test]
fn marks_a_verified_bitcoin() {
	set_bitcoin_height(12);
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
		assert_eq!(DefaultVault::get().argons_locked, price);

		assert_ok!(BitcoinLocks::utxo_verified(1));
		assert!(LocksByUtxoId::<Test>::get(1).unwrap().is_verified);
		assert_eq!(WatchedUtxosById::get().len(), 1);
		assert_eq!(LastLockEvent::get(), Some((1, who, price)));
	});
}

#[test]
fn calculates_redemption_prices() {
	new_test_ext().execute_with(|| {
		struct Scenario {
			argon_price: &'static str,
			btc_price: &'static str,
			expected_redemption: &'static str,
		}
		fn parse_price(price: &str) -> FixedU128 {
			let price: f64 = price
				.replace(",", "")
				.parse()
				.unwrap_or_else(|_| panic!("should parse price {}", price));

			FixedU128::from_float(price)
		}
		fn test_scenario(name: &str, scenario: Scenario) {
			ArgonPriceInUsd::set(Some(parse_price(scenario.argon_price)));
			ArgonTargetPriceInUsd::set(Some(FixedU128::from_u32(1)));
			BitcoinPriceInUsd::set(Some(parse_price(scenario.btc_price)));
			let new_price = BitcoinLocks::get_redemption_price(&SATOSHIS_PER_BITCOIN, None)
				.expect("should have price");
			let expected_price = parse_price(scenario.expected_redemption);
			assert_eq!(
				new_price,
				expected_price.saturating_mul_int(MICROGONS_PER_ARGON),
				"{}: redemption price",
				name
			);
		}
		test_scenario(
			">= 1.0 tier",
			Scenario { argon_price: "1.00", btc_price: "1.00", expected_redemption: "1.00" },
		);
		test_scenario(
			">= 0.9 tier",
			Scenario { argon_price: "0.95", btc_price: "1.00", expected_redemption: "0.95" },
		);
		test_scenario(
			"0.01 >= r < 0.9 tier (0.8)",
			Scenario { argon_price: "0.80", btc_price: "1.00", expected_redemption: "1.0548" },
		);
		test_scenario(
			"0.01 >= r < 0.9 tier (0.2)",
			Scenario { argon_price: "0.20", btc_price: "1.00", expected_redemption: "2.5338" },
		);

		test_scenario(
			"r < 0.01 tier (0.001)",
			Scenario { argon_price: "0.001", btc_price: "1.00", expected_redemption: "400.576" },
		);

		test_scenario(
			"r < 0.01 tier (0.0001)",
			Scenario { argon_price: "0.0001", btc_price: "1.00", expected_redemption: "4,000.576" },
		);
	});
}

#[test]
fn burns_a_spent_bitcoin() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let allocated = DefaultVault::get().securitization;
		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(62000)));

		assert_ok!(BitcoinLocks::initialize(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		let expiration_block = BitcoinBlockHeightChange::get().1 + LockDurationBlocks::get();

		let price = StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().argons_locked, price);
		// first verify
		assert_ok!(BitcoinLocks::utxo_verified(1));

		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(50000)));

		let new_price = BitcoinLocks::get_redemption_price(&SATOSHIS_PER_BITCOIN, None)
			.expect("should have price");
		assert_eq!(new_price, 50_000_000_000);

		assert_ok!(BitcoinLocks::utxo_spent(1));
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(WatchedUtxosById::get().len(), 0);
		assert_eq!(DefaultVault::get().argons_locked, price - new_price);
		assert_eq!(DefaultVault::get().securitization, allocated - new_price);

		System::assert_last_event(
			Event::<Test>::BitcoinLockBurned { vault_id: 1, utxo_id: 1, was_utxo_spent: true }
				.into(),
		);
		set_bitcoin_height(expiration_block);
		BitcoinLocks::on_initialize(2);

		assert_eq!(LastReleaseEvent::get(), Some((1, true, new_price)));
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
	});
}

#[test]
fn cancels_an_unverified_spent_bitcoin() {
	set_bitcoin_height(12);
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
		assert!(!LocksByUtxoId::<Test>::get(1).unwrap().is_verified);
		assert_eq!(WatchedUtxosById::get().len(), 1);
		// spend before verify
		assert_ok!(BitcoinLocks::utxo_spent(1));

		assert_eq!(WatchedUtxosById::get().len(), 0);
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(CanceledLocks::get().len(), 1);
	});
}

#[test]
fn can_release_a_bitcoin() {
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
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
		assert_eq!(DefaultVault::get().argons_locked, lock.liquidity_promised);
		let expiration_block = lock.vault_claim_height;
		// first verify
		assert_ok!(BitcoinLocks::utxo_verified(1));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, lock.liquidity_promised));

		BitcoinPriceInUsd::set(Some(FixedU128::from_u32(65_000)));
		// now the user goes to release
		// 1. We would create a psbt and output address
		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		let ticks_per_bitcoin_block = TicksPerBitcoinBlock::get();
		let cosign_due_ticks = LockReleaseCosignDeadlineFrames::get() * ArgonTicksPerDay::get();
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
		set_bitcoin_height(expiration_block - 1);
		assert_err!(
			BitcoinLocks::request_release(
				RuntimeOrigin::signed(who),
				1,
				release_script_pubkey.clone(),
				1000
			),
			Error::<Test>::BitcoinReleaseInitiationDeadlinePassed
		);
		set_bitcoin_height(expiration_block - (cosign_due_ticks / ticks_per_bitcoin_block) - 1);
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			release_script_pubkey.clone(),
			1000
		));
		assert!(LocksByUtxoId::<Test>::get(1).is_some());
		let redemption_price = BitcoinLocks::get_redemption_price(&SATOSHIS_PER_BITCOIN, None)
			.expect("should have price");
		assert!(redemption_price > lock.liquidity_promised);
		// redemption price should be the lock price since current redemption price is above
		assert_eq!(
			LockReleaseRequestsByUtxoId::<Test>::get(1).unwrap(),
			LockReleaseRequest {
				utxo_id: 1,
				vault_id: 1,
				cosign_due_frame: CurrentFrameId::get() + LockReleaseCosignDeadlineFrames::get(),
				redemption_price: lock.liquidity_promised,
				to_script_pubkey: release_script_pubkey,
				bitcoin_network_fee: 1000
			}
		);
		assert!(
			LockCosignDueByFrame::<Test>::get(
				CurrentFrameId::get() + LockReleaseCosignDeadlineFrames::get()
			)
			.contains(&1)
		);
		assert!(VaultViewOfCosignPendingLocks::get().contains_key(&1));
		assert!(LocksByUtxoId::<Test>::get(1).is_some());
		System::assert_last_event(
			Event::<Test>::BitcoinUtxoCosignRequested { vault_id: 1, utxo_id: 1 }.into(),
		);

		assert_eq!(Balances::free_balance(who), 2_000);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ReleaseBitcoinLock.into(), &who),
			lock.liquidity_promised
		);
	});
}
#[test]
fn test_redemption_rate_vs_market() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BitcoinPriceInUsd::set(Some(FixedU128::from_rational(60_000_50, 1_00)));

		let market_rate =
			StaticPriceProvider::get_bitcoin_argon_price(100).expect("should have price");
		assert_eq!(market_rate, 60_000);
		assert_eq!(BitcoinLocks::get_redemption_price(&100, None).unwrap(), 60_000);
	});
}

#[test]
fn penalizes_vault_if_not_release_countersigned() {
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		let satoshis = SATOSHIS_PER_BITCOIN + 5000;
		set_argons(who, 2_000);
		assert_ok!(BitcoinLocks::initialize(RuntimeOrigin::signed(who), 1, satoshis, pubkey));
		let vault = DefaultVault::get();
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(vault.argons_locked, lock.liquidity_promised);
		// first verify
		assert_ok!(BitcoinLocks::utxo_verified(1));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, lock.liquidity_promised));
		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			release_script_pubkey.clone(),
			2000
		));
		assert!(LocksByUtxoId::<Test>::get(1).is_some());

		let redemption_price =
			BitcoinLocks::get_redemption_price(&satoshis, None).expect("should have price");
		let cosign_due = CurrentFrameId::get() + LockReleaseCosignDeadlineFrames::get();
		assert_eq!(
			LockReleaseRequestsByUtxoId::<Test>::get(1),
			Some(LockReleaseRequest {
				utxo_id: 1,
				vault_id: 1,
				cosign_due_frame: cosign_due,
				redemption_price,
				to_script_pubkey: release_script_pubkey,
				bitcoin_network_fee: 2000
			})
		);
		assert!(LockCosignDueByFrame::<Test>::get(cosign_due).contains(&1));
		assert!(VaultViewOfCosignPendingLocks::get().contains_key(&1));
		assert!(VaultViewOfCosignPendingLocks::get().get(&1).unwrap().contains(&1));

		CurrentFrameId::set(cosign_due);
		System::set_block_number(2);
		BitcoinLocks::on_initialize(2);

		// should pay back at market price (not the discounted rate)
		let market_price =
			StaticPriceProvider::get_bitcoin_argon_price(satoshis).expect("should have price");
		assert_eq!(LockReleaseRequestsByUtxoId::<Test>::get(1), None);
		assert!(LockCosignDueByFrame::<Test>::get(cosign_due).is_empty());
		assert!(VaultViewOfCosignPendingLocks::get().get(&1).unwrap().is_empty());
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		System::assert_last_event(
			Event::<Test>::BitcoinCosignPastDue {
				vault_id: 1,
				utxo_id: 1,
				compensation_amount: redemption_price,
				compensated_account_id: who,
			}
			.into(),
		);
		assert_eq!(LastReleaseEvent::get(), Some((1, false, redemption_price)));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ReleaseBitcoinLock.into(), &who), 0);
		assert_eq!(Balances::balance(&who), 2000 + market_price);
	});
}

#[test]
fn clears_released_bitcoins() {
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let secp = bitcoin::secp256k1::Secp256k1::new();
		let rng = &mut bitcoin::secp256k1::rand::thread_rng();
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
		assert_eq!(vault.argons_locked, lock.liquidity_promised);
		// first verify
		assert_ok!(BitcoinLocks::utxo_verified(1));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, lock.liquidity_promised));
		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			release_script_pubkey.clone(),
			11
		));
		assert!(LocksByUtxoId::<Test>::get(1).is_some());

		let redemption_price =
			BitcoinLocks::get_redemption_price(&satoshis, None).expect("should have price");
		let cosign_due_frame = CurrentFrameId::get() + LockReleaseCosignDeadlineFrames::get();
		assert_eq!(
			LockReleaseRequestsByUtxoId::<Test>::get(1),
			Some(LockReleaseRequest {
				utxo_id: 1,
				vault_id: 1,
				cosign_due_frame,
				redemption_price,
				to_script_pubkey: release_script_pubkey,
				bitcoin_network_fee: 11
			})
		);
		assert!(LockCosignDueByFrame::<Test>::get(cosign_due_frame).contains(&1));
		assert!(VaultViewOfCosignPendingLocks::get().contains_key(&1));
		assert!(VaultViewOfCosignPendingLocks::get().get(&1).unwrap().contains(&1));

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
		assert!(LockCosignDueByFrame::<Test>::get(cosign_due_frame).is_empty());
		assert!(LockReleaseRequestsByUtxoId::<Test>::get(1).is_none());
		assert!(VaultViewOfCosignPendingLocks::get().get(&1).unwrap().is_empty());
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(DefaultVault::get().argons_locked, lock.liquidity_promised);
		assert_eq!(DefaultVault::get().securitization, vault.securitization);
		assert_eq!(LockReleaseCosignHeightById::<Test>::get(1), Some(1));

		System::assert_last_event(
			Event::<Test>::BitcoinUtxoCosigned {
				vault_id: 1,
				utxo_id: 1,
				signature: BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec())),
			}
			.into(),
		);

		assert_eq!(Balances::balance_on_hold(&HoldReason::ReleaseBitcoinLock.into(), &who), 0);
		assert_eq!(Balances::balance(&who), 2000 + lock.liquidity_promised - redemption_price);
		assert_eq!(Balances::total_issuance(), 2000 + lock.liquidity_promised - redemption_price);

		// should keep for the year
		System::set_block_number(2);
		set_bitcoin_height(lock.vault_claim_height);
		BitcoinLocks::on_initialize(2);
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_ok!(BitcoinLocks::utxo_spent(1));
	});
}

#[test]
fn it_should_aggregate_holds_for_a_second_release() {
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		let satoshis = SATOSHIS_PER_BITCOIN;
		set_argons(who, 2_000);
		assert_ok!(BitcoinLocks::initialize(RuntimeOrigin::signed(who), 1, satoshis, pubkey));
		assert_ok!(BitcoinLocks::initialize(RuntimeOrigin::signed(who), 1, satoshis, pubkey));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_ok!(BitcoinLocks::utxo_verified(1));
		assert_ok!(BitcoinLocks::utxo_verified(2));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, lock.liquidity_promised * 2));
		let redemption_price =
			BitcoinLocks::get_redemption_price(&satoshis, None).expect("should have price");

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
		assert_eq!(
			Balances::free_balance(who),
			2_000 + (2 * (lock.liquidity_promised - redemption_price))
		);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ReleaseBitcoinLock.into(), &who),
			redemption_price * 2
		);
	});
}

#[test]
fn it_should_allow_a_ratchet_up() {
	ChargeFee::set(true);
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		let satoshis = SATOSHIS_PER_BITCOIN;
		let current_block = BitcoinBlockHeightChange::get().1;
		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(62_000)));
		let apr = FixedU128::from_float(0.00000001);
		DefaultVault::mutate(|a| {
			a.securitization = 70_000_000_000;
			a.terms.bitcoin_base_fee = 1000;
			a.terms.bitcoin_annual_percent_rate = apr;

			a.lock(8_000 * MICROGONS_PER_ARGON).unwrap();
		});
		set_argons(who, 5000);
		assert_ok!(BitcoinLocks::initialize(RuntimeOrigin::signed(who), 1, satoshis, pubkey));
		assert_ok!(BitcoinLocks::utxo_verified(1));
		assert_eq!(LastLockEvent::get(), Some((1, who, 62_000 * MICROGONS_PER_ARGON)));
		let balance_after_one = 5000 - 1000 - 620; // 3,380
		assert_eq!(Balances::free_balance(who), balance_after_one); // 3,380

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		let expiration_block = lock.vault_claim_height;
		assert_eq!(lock.security_fees, 1000 + 620);
		let middle_block = (current_block + (expiration_block - current_block) / 2) + 1;
		BitcoinBlockHeightChange::set((middle_block, middle_block));

		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(65000)));
		assert_err!(
			BitcoinLocks::ratchet(RuntimeOrigin::signed(who), 1,),
			Error::<Test>::InsufficientVaultFunds,
		);
		let extension = LockExtension::new(expiration_block + 144);
		// release funds with an expiration
		DefaultVault::mutate(|a| {
			a.schedule_for_release(8_000 * MICROGONS_PER_ARGON, &extension).unwrap();
		});

		assert_ok!(BitcoinLocks::ratchet(RuntimeOrigin::signed(who), 1,));
		assert_eq!(LastLockEvent::get(), Some((1, who, 3_000 * MICROGONS_PER_ARGON)));
		let apr = apr.saturating_mul_int(3_000 * MICROGONS_PER_ARGON); // 30
		assert_eq!(
			Balances::free_balance(who),
			balance_after_one - 1000 - (apr / 2),
			"should pay prorated fee"
		);
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.liquidity_promised, 65_000 * MICROGONS_PER_ARGON);
		assert_eq!(
			lock.fund_hold_extensions.into_iter().collect::<Vec<_>>(),
			vec![(extension.day(), 3_000 * MICROGONS_PER_ARGON)]
		);
		assert_eq!(lock.security_fees, 1000 + 620 + (1000 + apr / 2));
		System::assert_last_event(
			Event::<Test>::BitcoinLockRatcheted {
				vault_id: 1,
				utxo_id: 1,
				liquidity_promised: 65_000 * MICROGONS_PER_ARGON,
				original_pegged_price: 62_000 * MICROGONS_PER_ARGON,
				new_pegged_price: 65_000 * MICROGONS_PER_ARGON,
				account_id: who,
				amount_burned: 0,
				security_fee: 1000 + apr / 2,
			}
			.into(),
		)
	});
}

/// During a ratchet down, the user will put up funds to unlock at the current price, and then will
/// go back into the mint queue.
#[test]
fn it_should_allow_a_ratchet_down() {
	ChargeFee::set(true);
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		let satoshis = SATOSHIS_PER_BITCOIN;
		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(62_000)));
		let apr = FixedU128::from_float(0.00000001);
		DefaultVault::mutate(|a| {
			a.securitization = 62_000_000_000;
			a.terms.bitcoin_base_fee = 1000;
			a.terms.bitcoin_annual_percent_rate = apr;
		});
		let mut balance = 5000;
		set_argons(who, balance);
		assert_ok!(BitcoinLocks::initialize(RuntimeOrigin::signed(who), 1, satoshis, pubkey));
		balance.saturating_reduce(1000 + 620);
		assert_eq!(Balances::free_balance(who), balance);

		// Mint the argons into account
		assert_ok!(BitcoinLocks::utxo_verified(1));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_ok!(Balances::mint_into(&who, lock.liquidity_promised));
		balance.saturating_accrue(lock.liquidity_promised);
		assert_eq!(Balances::free_balance(who), balance);

		assert_eq!(LastLockEvent::get(), Some((1, who, 62_000 * MICROGONS_PER_ARGON)));
		assert_eq!(Balances::free_balance(who), balance);

		// now set price to 52k and down ratchet
		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(52_000)));
		let redemption_price = BitcoinLocks::get_redemption_price(&SATOSHIS_PER_BITCOIN, None)
			.expect("should have price");
		assert_ok!(BitcoinLocks::ratchet(RuntimeOrigin::signed(who), 1,));
		assert_eq!(
			LastReleaseEvent::get(),
			Some((1, false, redemption_price)),
			"shouldn't remove from mint queue, but should release the redemption amount"
		);
		assert_eq!(
			LastLockEvent::get(),
			Some((1, who, 52_000 * MICROGONS_PER_ARGON)),
			"should record locking the new amount"
		);
		// should only pay base fee
		balance.saturating_reduce(redemption_price + 1000);
		assert_eq!(Balances::free_balance(who), balance);
		assert!(
			Balances::free_balance(who) > 10_000 * MICROGONS_PER_ARGON,
			"user should pocket the 10k"
		);
	});
}

#[test]
fn it_should_use_the_pegged_price_during_a_ratchet() {
	ChargeFee::set(true);
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		let satoshis = 101_000_000;
		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(100_000)));
		// set cpi to -0.009900990099 (1% above target)
		ArgonPriceInUsd::set(Some(FixedU128::from_float(1.01)));
		ArgonTargetPriceInUsd::set(Some(FixedU128::from_float(1.00)));
		let apr = FixedU128::from_float(0.00000001);
		DefaultVault::mutate(|a| {
			a.securitization = 120_000_000_000;
			a.terms.bitcoin_base_fee = 1000;
			a.terms.bitcoin_annual_percent_rate = apr;
		});
		set_argons(who, 5000);
		assert_ok!(BitcoinLocks::initialize(RuntimeOrigin::signed(who), 1, satoshis, pubkey));

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.liquidity_promised, 100_000 * MICROGONS_PER_ARGON);
		assert_eq!(lock.pegged_price, 99_009_900_990); // 100k / 1.01

		// Mint the argons into account
		assert_ok!(BitcoinLocks::utxo_verified(1));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_ok!(Balances::mint_into(&who, lock.liquidity_promised));

		// now set cpi back to 0 and ratchet
		ArgonPriceInUsd::set(Some(FixedU128::from_float(1.00)));

		assert_ok!(BitcoinLocks::ratchet(RuntimeOrigin::signed(who), 1,));
		System::assert_last_event(
			Event::BitcoinLockRatcheted {
				vault_id: 1,
				utxo_id: 1,
				liquidity_promised: 101_000 * MICROGONS_PER_ARGON, // 1.01 BTC
				original_pegged_price: 99_009_900_990,
				new_pegged_price: 101_000 * MICROGONS_PER_ARGON,
				account_id: who,
				amount_burned: 0,
				security_fee: 1000,
			}
			.into(),
		);
		assert_eq!(
			LastLockEvent::get(),
			Some((1, who, (101_000 * MICROGONS_PER_ARGON) - 99_009_900_990))
		);
	});
}

fn make_script_pubkey(vec: &[u8]) -> BitcoinScriptPubkey {
	BitcoinScriptPubkey(BoundedVec::try_from(vec.to_vec()).unwrap())
}
