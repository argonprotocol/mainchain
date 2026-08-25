#![allow(clippy::zero_prefixed_literal)]
#![allow(clippy::multiple_bound_locations)]
#![allow(clippy::inconsistent_digit_grouping)]

use codec::Encode;
use pallet_prelude::*;

use crate::{
	mock::*,
	pallet::{
		LastFeeCouponNonceByVaultAndAccount, LastPendingFundingExpirationHeight,
		LockCosignDueByFrame, LockReleaseCosignHeightById, LockReleaseRequestsByUtxoId,
		LocksByUtxoId, LocksPendingFundingByBitcoinHeight, MigratedReleaseHoldByUtxoId,
		UtxoIdToFundingUtxoRef, UtxoIdsByOwnerAccount, UtxoIdsByVaultId,
	},
	Error, Event, FeeCoupon, HoldReason, LockExpirationsByBitcoinHeight, LockOptions,
	LockReleaseRequest, MicrogonsAtTargetPerBtcHistory, OrphanedUtxoExpirationByFrame,
	OrphanedUtxosByAccount, FEE_COUPON_MESSAGE_KEY,
};
use argon_bitcoin::{Amount, CosignReleaser, CosignScriptArgs, ReleaseStep};
use argon_primitives::{
	bitcoin::{
		BitcoinBlock, BitcoinScriptPubkey, BitcoinSignature, CompressedBitcoinPubkey, H256Le,
		Satoshis, UtxoId, UtxoRef, SATOSHIS_PER_BITCOIN,
	},
	inherents::{BitcoinUtxoFunding, BitcoinUtxoSync},
	providers::{BitcoinFissionLockError, BitcoinFissionLockProvider},
	BitcoinUtxoEvents, BitcoinUtxoTracker, PriceProvider, MICROGONS_PER_ARGON,
};

const FEE_COUPON_TARGET_RATE: Balance = 62_000 * MICROGONS_PER_ARGON;

fn funding_received(utxo_id: UtxoId, satoshis: Satoshis) -> DispatchResult {
	let utxo_ref = UtxoIdToFundingUtxoRef::<Test>::get(utxo_id)
		.unwrap_or(UtxoRef { txid: H256Le([0; 32]), output_index: 0 });
	let bitcoin_height = LocksByUtxoId::<Test>::get(utxo_id)
		.map(|lock| lock.created_at_height)
		.unwrap_or_default();
	<BitcoinLocks as BitcoinUtxoEvents<u64>>::utxo_detected(
		utxo_id,
		utxo_ref,
		satoshis,
		bitcoin_height,
	)
}

fn spent(utxo_id: UtxoId) -> DispatchResult {
	let Some(lock) = LocksByUtxoId::<Test>::get(utxo_id) else { return Ok(()) };
	let default_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };
	let utxo_ref = UtxoIdToFundingUtxoRef::<Test>::get(utxo_id).or_else(|| {
		OrphanedUtxosByAccount::<Test>::iter_prefix(lock.owner_account)
			.find_map(|(utxo_ref, orphan)| (orphan.utxo_id == utxo_id).then_some(utxo_ref))
	});
	let utxo_ref = utxo_ref.unwrap_or(default_ref.clone());
	if !lock.is_funded() {
		UtxoIdToFundingUtxoRef::<Test>::insert(utxo_id, default_ref);
	}
	<BitcoinLocks as BitcoinUtxoEvents<u64>>::spent(utxo_id, utxo_ref)
}

#[test]
fn create_receive_address_stores_lock_accounting() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		set_argons(2, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.owner_account, 2);
		assert_eq!(lock.funded_satoshis, 0);
		assert_eq!(lock.fissioned_satoshis, 0);
		assert_eq!(lock.securitized_satoshis, SATOSHIS_PER_BITCOIN);
		assert_eq!(
			lock.microgons_at_target_per_btc,
			StaticPriceProvider::get_btc_price_in_target_microgons(SATOSHIS_PER_BITCOIN)
				.expect("should have price")
		);

		assert_eq!(WatchedUtxosById::get().len(), 1);
		assert!(LockExpirationsByBitcoinHeight::<Test>::get(lock.vault_claim_height).contains(&1));
		assert!(UtxoIdsByVaultId::<Test>::contains_key(1, 1));
	});
}

#[test]
fn create_receive_address_uses_the_redemption_curve_for_securitization() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let target_rate = 500_000 * MICROGONS_PER_ARGON;
		ArgonPriceInUsd::set(Some(FixedU128::from_float(1.0)));
		ArgonTargetPriceInUsd::set(Some(FixedU128::from_float(1.1)));
		DefaultVault::mutate(|vault| vault.securitization = 600_000 * MICROGONS_PER_ARGON);
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((1, target_rate));
		});
		set_argons(2, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			Some(LockOptions { microgons_at_target_per_btc: target_rate, fee_coupon: None }),
		));

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		assert_eq!(lock.securitization_coverage_microgons, 491_735_537_190);
		assert_eq!(
			DefaultVault::get().securitization_locked,
			lock.securitization_coverage_microgons
		);
		assert_eq!(
			lock.security_fees,
			FixedU128::from_float(0.1).saturating_mul_int(lock.securitization_coverage_microgons)
		);
	});
}

#[test]
fn first_output_funds_without_creating_liquidity() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None
		));

		let funded_satoshis = SATOSHIS_PER_BITCOIN - 3_000;
		assert_ok!(funding_received(1, funded_satoshis));

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.funded_satoshis, funded_satoshis);
		assert_eq!(lock.fissioned_satoshis, 0);
	});
}

#[test]
fn fission_uses_funded_coverage_and_the_existing_redemption_formula() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN / 2));

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let microgons_at_target_per_btc = lock.microgons_at_target_per_btc;
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((12, microgons_at_target_per_btc));
		});
		let fissioned_satoshis = SATOSHIS_PER_BITCOIN / 4;
		assert_ok!(BitcoinFissions::create(
			RuntimeOrigin::signed(2),
			0,
			77,
			1,
			fissioned_satoshis,
			microgons_at_target_per_btc,
		));
		let fission =
			pallet_bitcoin_fissions::FissionByOwnerAndId::<Test>::get(2, 0).expect("fission");
		assert_eq!(fission.last_ratchet_tick, 12);

		assert_eq!(
			fission.liquidity_promised,
			BitcoinLocks::calculate_redemption_amount_from_satoshis(&fissioned_satoshis, None)
				.expect("redemption amount")
		);
		assert_eq!(
			LocksByUtxoId::<Test>::get(1).expect("lock").fissioned_satoshis,
			fissioned_satoshis
		);

		let up_rate = microgons_at_target_per_btc + 10_000 * MICROGONS_PER_ARGON;
		let down_rate = microgons_at_target_per_btc - 10_000 * MICROGONS_PER_ARGON;
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((13, up_rate));
			_ = rates.try_push((14, down_rate));
		});
		let up_liquidity = fission.liquidity_promised.saturating_add(2_500 * MICROGONS_PER_ARGON);
		let down_liquidity = 13_000 * MICROGONS_PER_ARGON;
		LocksByUtxoId::<Test>::mutate(1, |lock| {
			let lock = lock.as_mut().expect("lock");
			lock.microgons_at_target_per_btc = up_rate;
			lock.securitization_coverage_microgons = up_liquidity;
		});
		assert_eq!(
			BitcoinLocks::validate_fission(
				&2,
				1,
				fissioned_satoshis,
				up_rate,
				12,
				fission.liquidity_promised,
				up_liquidity,
			),
			Ok(13)
		);
		assert_eq!(
			BitcoinLocks::validate_fission(
				&2,
				1,
				fissioned_satoshis,
				down_rate,
				13,
				fission.liquidity_promised,
				down_liquidity,
			),
			Ok(14)
		);
		assert_eq!(
			BitcoinLocks::calculate_liquidity_promised(
				fissioned_satoshis,
				up_rate - microgons_at_target_per_btc,
			),
			Ok(2_500 * MICROGONS_PER_ARGON)
		);
		assert_eq!(
			BitcoinLocks::calculate_liquidity_promised(fissioned_satoshis, down_rate),
			Ok(13_000 * MICROGONS_PER_ARGON)
		);
		assert_eq!(
			LocksByUtxoId::<Test>::get(1).expect("lock").fissioned_satoshis,
			fissioned_satoshis
		);
		assert_eq!(
			BitcoinLocks::fission_satoshis(
				&2,
				1,
				SATOSHIS_PER_BITCOIN / 2,
				microgons_at_target_per_btc,
			),
			Err(BitcoinFissionLockError::InsufficientFundedSatoshis)
		);
		assert_eq!(
			BitcoinLocks::fission_satoshis(&3, 1, 1, microgons_at_target_per_btc,),
			Err(BitcoinFissionLockError::NoPermissions)
		);

		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(31_000)));
		assert_eq!(
			BitcoinLocks::fuse_satoshis(&2, 1, fissioned_satoshis, microgons_at_target_per_btc,),
			Ok(7_750 * MICROGONS_PER_ARGON)
		);
		pallet_bitcoin_fissions::FissionByOwnerAndId::<Test>::remove(2, 0);
		pallet_bitcoin_fissions::FissionIdsByLockId::<Test>::remove(1);
		assert_eq!(LocksByUtxoId::<Test>::get(1).expect("lock").fissioned_satoshis, 0);
		assert_eq!(
			BitcoinLocks::fuse_satoshis(&2, 1, fissioned_satoshis, microgons_at_target_per_btc,),
			Err(BitcoinFissionLockError::InsufficientFissionedSatoshis)
		);

		BitcoinLocks::fission_satoshis(&2, 1, fissioned_satoshis, microgons_at_target_per_btc)
			.expect("second allocation");
		BitcoinPriceInUsd::set(None);
		assert_eq!(
			BitcoinLocks::fuse_satoshis(&2, 1, fissioned_satoshis, microgons_at_target_per_btc,),
			Err(BitcoinFissionLockError::NoBitcoinPricesAvailable)
		);
		assert_eq!(
			LocksByUtxoId::<Test>::get(1).expect("lock").fissioned_satoshis,
			fissioned_satoshis
		);
	});
}

#[test]
fn fission_creation_requires_aggregate_curve_adjusted_coverage() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let rate = lock.microgons_at_target_per_btc;
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((12, rate));
		});
		assert_ok!(BitcoinFissions::create(
			RuntimeOrigin::signed(2),
			0,
			77,
			1,
			SATOSHIS_PER_BITCOIN / 2,
			rate,
		));

		ArgonPriceInUsd::set(Some(FixedU128::from_rational(1, 2)));
		assert_noop!(
			BitcoinFissions::create(
				RuntimeOrigin::signed(2),
				1,
				77,
				1,
				SATOSHIS_PER_BITCOIN / 2,
				rate,
			),
			pallet_bitcoin_fissions::Error::<Test>::InsufficientSecuritization
		);
	});
}

#[test]
fn fission_creation_rejects_a_target_value_from_before_the_lock_securitization() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let stale_rate =
			lock.microgons_at_target_per_btc.saturating_sub(10_000 * MICROGONS_PER_ARGON);
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((lock.securitization_tick.saturating_sub(1), stale_rate));
		});

		assert_noop!(
			BitcoinFissions::create(
				RuntimeOrigin::signed(2),
				0,
				77,
				1,
				SATOSHIS_PER_BITCOIN / 2,
				stale_rate,
			),
			pallet_bitcoin_fissions::Error::<Test>::MicrogonsAtTargetPerBtcTickOlderThanCurrent
		);
	});
}

#[test]
fn resecuritize_replaces_funded_vault_backing() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let old_lock = LocksByUtxoId::<Test>::get(1).unwrap();
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((12, old_lock.microgons_at_target_per_btc));
		});
		let securitized_satoshis = SATOSHIS_PER_BITCOIN / 2;
		assert_ok!(BitcoinLocks::resecuritize(
			RuntimeOrigin::signed(2),
			1,
			securitized_satoshis,
			lock_options(old_lock.microgons_at_target_per_btc),
		));

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.securitized_satoshis, securitized_satoshis);
		assert_eq!(
			DefaultVault::get().securitization_locked,
			lock.get_securitization().collateral_required()
		);
		assert_eq!(DefaultVault::get().locked_satoshis, SATOSHIS_PER_BITCOIN);
		assert_eq!(DefaultVault::get().ratio_adjusted_satoshis, securitized_satoshis);
	});
}

#[test]
fn resecuritize_rejects_a_target_value_from_before_the_current_securitization() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let old_rate = lock.microgons_at_target_per_btc;
		let replacement_rate = old_rate.saturating_add(10_000 * MICROGONS_PER_ARGON);
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((12, old_rate));
			_ = rates.try_push((13, replacement_rate));
		});

		assert_ok!(BitcoinLocks::resecuritize(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			lock_options(replacement_rate),
		));
		assert_eq!(LocksByUtxoId::<Test>::get(1).expect("lock").securitization_tick, 13);
		assert_noop!(
			BitcoinLocks::resecuritize(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				lock_options(old_rate),
			),
			Error::<Test>::MicrogonsAtTargetPerBtcTickOlderThanCurrent
		);
	});
}

#[test]
fn resecuritize_preserves_curve_adjusted_fission_coverage() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);
		ArgonPriceInUsd::set(Some(FixedU128::from_rational(1, 2)));
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let rate = lock.microgons_at_target_per_btc;
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((12, rate));
		});
		assert_ok!(BitcoinFissions::create(
			RuntimeOrigin::signed(2),
			0,
			77,
			1,
			SATOSHIS_PER_BITCOIN / 2,
			rate,
		));

		ArgonPriceInUsd::set(Some(FixedU128::one()));
		assert_noop!(
			BitcoinLocks::resecuritize(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN / 2,
				lock_options(rate),
			),
			Error::<Test>::InsufficientSecuritizationForFissions
		);
	});
}

#[test]
fn fission_ratchet_rejects_a_target_value_from_before_its_previous_ratchet() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 100_000 * MICROGONS_PER_ARGON);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let first_rate = 40_000 * MICROGONS_PER_ARGON;
		let second_rate = 50_000 * MICROGONS_PER_ARGON;
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((12, first_rate));
			_ = rates.try_push((13, second_rate));
		});

		assert_ok!(BitcoinFissions::create(
			RuntimeOrigin::signed(2),
			0,
			77,
			1,
			SATOSHIS_PER_BITCOIN / 2,
			first_rate,
		));
		assert_eq!(
			pallet_bitcoin_fissions::FissionByOwnerAndId::<Test>::get(2, 0)
				.expect("fission")
				.last_ratchet_tick,
			12
		);
		assert_ok!(BitcoinFissions::ratchet(RuntimeOrigin::signed(2), 0, second_rate));
		assert_eq!(
			pallet_bitcoin_fissions::FissionByOwnerAndId::<Test>::get(2, 0)
				.expect("fission")
				.last_ratchet_tick,
			13
		);
		assert_noop!(
			BitcoinFissions::ratchet(RuntimeOrigin::signed(2), 0, first_rate),
			pallet_bitcoin_fissions::Error::<Test>::MicrogonsAtTargetPerBtcTickOlderThanCurrent
		);
	});
}

#[test]
fn resecuritize_rejects_a_target_value_from_before_an_active_fission() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 100_000 * MICROGONS_PER_ARGON);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let original_rate = lock.microgons_at_target_per_btc;
		let replacement_rate = original_rate - 10_000 * MICROGONS_PER_ARGON;
		let fission_rate = original_rate - 20_000 * MICROGONS_PER_ARGON;
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((12, original_rate));
			_ = rates.try_push((13, replacement_rate));
			_ = rates.try_push((14, fission_rate));
		});
		assert_ok!(BitcoinFissions::create(
			RuntimeOrigin::signed(2),
			0,
			77,
			1,
			SATOSHIS_PER_BITCOIN / 2,
			original_rate,
		));
		assert_ok!(BitcoinFissions::ratchet(RuntimeOrigin::signed(2), 0, fission_rate));

		assert_noop!(
			BitcoinLocks::resecuritize(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				lock_options(replacement_rate),
			),
			Error::<Test>::MicrogonsAtTargetPerBtcTickOlderThanCurrent
		);
	});
}

#[test]
fn resecuritize_rejects_a_target_value_reduction_with_active_fissions() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let fission_rate = lock.microgons_at_target_per_btc;
		let replacement_rate = fission_rate - 10_000 * MICROGONS_PER_ARGON;
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((12, fission_rate));
			_ = rates.try_push((13, replacement_rate));
		});
		assert_ok!(BitcoinFissions::create(
			RuntimeOrigin::signed(2),
			0,
			77,
			1,
			SATOSHIS_PER_BITCOIN / 2,
			fission_rate,
		));

		assert_noop!(
			BitcoinLocks::resecuritize(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN / 4,
				lock_options(fission_rate),
			),
			Error::<Test>::InsufficientSatoshisForFissions
		);

		assert_noop!(
			BitcoinLocks::resecuritize(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				lock_options(replacement_rate),
			),
			Error::<Test>::InsufficientSecuritizationForFissions
		);
	});
}

#[test]
fn resecuritize_allows_a_target_value_reduction_after_the_fission_ratchets_lower() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 100_000 * MICROGONS_PER_ARGON);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let original_rate = lock.microgons_at_target_per_btc;
		let fission_rate = original_rate - 20_000 * MICROGONS_PER_ARGON;
		let replacement_rate = original_rate - 10_000 * MICROGONS_PER_ARGON;
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((12, original_rate));
			_ = rates.try_push((13, fission_rate));
			_ = rates.try_push((14, replacement_rate));
		});

		assert_ok!(BitcoinFissions::create(
			RuntimeOrigin::signed(2),
			0,
			77,
			1,
			SATOSHIS_PER_BITCOIN / 2,
			original_rate,
		));
		assert_ok!(BitcoinFissions::ratchet(RuntimeOrigin::signed(2), 0, fission_rate));

		assert_ok!(BitcoinLocks::resecuritize(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			lock_options(replacement_rate),
		));
		assert_eq!(
			LocksByUtxoId::<Test>::get(1).expect("lock").microgons_at_target_per_btc,
			replacement_rate
		);
	});
}

#[test]
fn resecuritize_replaces_pending_vault_backing_without_activating_bitcoin() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None
		));

		let old_lock = LocksByUtxoId::<Test>::get(1).unwrap();
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((12, old_lock.microgons_at_target_per_btc));
		});
		let securitized_satoshis = SATOSHIS_PER_BITCOIN / 2;
		assert_ok!(BitcoinLocks::resecuritize(
			RuntimeOrigin::signed(2),
			1,
			securitized_satoshis,
			lock_options(old_lock.microgons_at_target_per_btc),
		));

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		let vault = DefaultVault::get();
		assert!(!lock.is_funded());
		assert_eq!(lock.securitized_satoshis, securitized_satoshis);
		assert_eq!(vault.securitization_locked, lock.get_securitization().collateral_required());
		assert_eq!(vault.securitization_pending_activation, vault.securitization_locked);
		assert_eq!(vault.locked_satoshis, 0);
		assert_eq!(vault.ratio_adjusted_satoshis, 0);
	});
}

#[test]
fn resecuritize_rejects_an_ineligible_target_value() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None
		));

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_err!(
			BitcoinLocks::resecuritize(
				RuntimeOrigin::signed(2),
				1,
				lock.securitized_satoshis,
				lock_options(lock.microgons_at_target_per_btc.saturating_add(1)),
			),
			Error::<Test>::IneligibleMicrogonsAtTargetPerBtcRequested
		);
		assert_eq!(LocksByUtxoId::<Test>::get(1), Some(lock));
	});
}

#[test]
fn zero_securitization_still_tracks_funded_satoshis() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			0,
			CompressedBitcoinPubkey([1; 33]),
			None
		));

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		let funding_expiration_height = lock.funding_expiration_height;
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		let vault = DefaultVault::get();
		assert!(lock.is_funded());
		assert_eq!(lock.funding_expiration_height, funding_expiration_height);
		assert_eq!(lock.btc_value_in_microgons(), 0);
		assert_eq!(vault.securitization_locked, 0);
		assert_eq!(vault.locked_satoshis, SATOSHIS_PER_BITCOIN);
		assert_eq!(vault.ratio_adjusted_satoshis, 0);
	});
}

#[test]
fn funding_after_securitization_expires_is_recorded_as_an_orphan() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None
		));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		let expiration_height = lock.created_at_height + MaxPendingConfirmationBlocks::get() + 1;
		BitcoinBlockHeightChange::set((expiration_height, expiration_height));
		BitcoinLocks::on_initialize(2);

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.securitized_satoshis, 0);
		assert_eq!(lock.microgons_at_target_per_btc, 0);
		assert!(WatchedUtxosById::get().contains_key(&1));

		let orphan_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));
		assert!(!LocksByUtxoId::<Test>::get(1).unwrap().is_funded());
		assert_eq!(UtxoIdToFundingUtxoRef::<Test>::get(1), None);
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(2, orphan_ref));
	});
}

#[test]
fn final_eligible_funding_is_processed_before_pending_expiration() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let expiration_height = lock.funding_expiration_height;
		let prior_block = BitcoinBlock::new(expiration_height - 1, H256Le([1; 32]));
		let expiration_block = BitcoinBlock::new(expiration_height, H256Le([2; 32]));
		pallet_bitcoin_utxos::ConfirmedBitcoinBlockTip::<Test>::put(expiration_block.clone());
		pallet_bitcoin_utxos::SynchedBitcoinBlock::<Test>::put(prior_block);
		<BitcoinUtxos as BitcoinUtxoTracker>::watch_for_utxo(1, lock.utxo_script_pubkey)
			.expect("watch address");
		BitcoinBlockHeightChange::set((expiration_height, expiration_height));

		BitcoinLocks::on_initialize(2);
		assert_ne!(LocksByUtxoId::<Test>::get(1).expect("lock").securitized_satoshis, 0);

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				spent: vec![],
				funded: vec![BitcoinUtxoFunding {
					utxo_id: 1,
					utxo_ref: UtxoRef { txid: H256Le([3; 32]), output_index: 0 },
					satoshis: SATOSHIS_PER_BITCOIN,
					expected_satoshis: SATOSHIS_PER_BITCOIN,
					bitcoin_height: expiration_height - 1,
				}],
				sync_to_block: expiration_block,
			},
		));
		BitcoinLocks::on_initialize(3);

		assert!(LocksByUtxoId::<Test>::get(1).expect("lock").is_funded());
	});
}

#[test]
fn resecuritized_expired_lock_can_receive_funding() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		let original_lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let expiration_height =
			original_lock.created_at_height + MaxPendingConfirmationBlocks::get() + 1;

		BitcoinBlockHeightChange::set((expiration_height, expiration_height));
		BitcoinLocks::on_initialize(2);
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((expiration_height, original_lock.microgons_at_target_per_btc));
		});

		assert_ok!(BitcoinLocks::resecuritize(
			RuntimeOrigin::signed(2),
			1,
			original_lock.securitized_satoshis,
			lock_options(original_lock.microgons_at_target_per_btc),
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		assert_eq!(lock.funded_satoshis, SATOSHIS_PER_BITCOIN);
		assert_eq!(UtxoIdToFundingUtxoRef::<Test>::get(1).expect("funding ref").output_index, 0);
		assert!(OrphanedUtxosByAccount::<Test>::iter_prefix(2).next().is_none());
	});
}

#[test]
fn resecuritized_expired_lock_gets_a_new_funding_window() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		let original_lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let expiration_height =
			original_lock.created_at_height + MaxPendingConfirmationBlocks::get() + 1;

		BitcoinBlockHeightChange::set((expiration_height, expiration_height));
		BitcoinLocks::on_initialize(2);
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((expiration_height, original_lock.microgons_at_target_per_btc));
		});

		assert_ok!(BitcoinLocks::resecuritize(
			RuntimeOrigin::signed(2),
			1,
			original_lock.securitized_satoshis,
			lock_options(original_lock.microgons_at_target_per_btc),
		));

		let renewed_expiration_height = expiration_height + MaxPendingConfirmationBlocks::get() + 1;
		BitcoinBlockHeightChange::set((renewed_expiration_height, renewed_expiration_height));
		BitcoinLocks::on_initialize(3);

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		assert_eq!(lock.securitized_satoshis, 0);
		assert_eq!(lock.microgons_at_target_per_btc, 0);
	});
}

#[test]
fn release_is_rejected_while_funded_satoshis_are_fissioned() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(2, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));
		LocksByUtxoId::<Test>::mutate(1, |lock| {
			lock.as_mut().unwrap().fissioned_satoshis = 1;
		});

		assert_noop!(
			BitcoinLocks::request_release(
				RuntimeOrigin::signed(2),
				1,
				make_script_pubkey(&[1; 32]),
				1_000
			),
			Error::<Test>::LockHasActiveFissions
		);
	});
}

#[test]
fn can_lock_a_bitcoin_utxo_with_a_preset_target_value() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		DefaultVault::mutate(|x| {
			x.securitization = 500_000 * MICROGONS_PER_ARGON;
		});

		set_argons(2, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_err!(
			BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				pubkey,
				Some(LockOptions {
					microgons_at_target_per_btc: 500_000 * MICROGONS_PER_ARGON,
					fee_coupon: None,
				})
			),
			Error::<Test>::IneligibleMicrogonsAtTargetPerBtcRequested
		);

		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(100_000)));
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|x| {
			_ = x.try_push((1, 500_000 * MICROGONS_PER_ARGON));
			_ = x.try_push((2, 100_000 * MICROGONS_PER_ARGON));
		});

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			Some(LockOptions {
				microgons_at_target_per_btc: 500_000 * MICROGONS_PER_ARGON,
				fee_coupon: None,
			})
		));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.owner_account, 2);
		assert!(!lock.is_funded());
		assert_eq!(lock.btc_value_in_microgons(), 500_000 * MICROGONS_PER_ARGON);
	});
}

#[test]
fn cancels_an_unfunded_lock_on_release_request() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));
		let price = StaticPriceProvider::get_btc_price_in_market_microgons(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().securitization_locked, price);
		assert_eq!(DefaultVault::get().ratio_adjusted_satoshis, 0);

		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			0,
		));
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(DefaultVault::get().ratio_adjusted_satoshis, 0);
	});
}

#[test]
fn expires_pending_securitization_without_unwatching_the_lock() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let who = 1;
		set_argons(who, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None
		));
		let lock = LocksByUtxoId::<Test>::get(1).expect("lock should exist");
		let expiration_height = lock.created_at_height + MaxPendingConfirmationBlocks::get() + 1;
		assert!(LocksPendingFundingByBitcoinHeight::<Test>::get(expiration_height).contains(&1));

		BitcoinBlockHeightChange::set((expiration_height, expiration_height));
		BitcoinLocks::on_initialize(2);

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock should remain watched");
		assert_eq!(lock.funding_expiration_height, expiration_height);
		assert_eq!(DefaultVault::get().securitization_locked, 0);
		assert_eq!(DefaultVault::get().securitization_pending_activation, 0);
		assert!(UtxoIdsByVaultId::<Test>::contains_key(1, 1));
		assert!(UtxoIdsByOwnerAccount::<Test>::contains_key(who, 1));
		assert_eq!(WatchedUtxosById::get().len(), 1);
		assert!(LocksPendingFundingByBitcoinHeight::<Test>::get(expiration_height).is_empty());
	});
}

#[test]
fn retries_pending_securitization_expiration_after_provider_error() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(1),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None
		));
		let lock = LocksByUtxoId::<Test>::get(1).expect("lock should exist");
		let expiration_height = lock.created_at_height + MaxPendingConfirmationBlocks::get() + 1;

		FailReturnSecuritization::set(true);
		BitcoinBlockHeightChange::set((expiration_height, expiration_height));
		BitcoinLocks::on_initialize(2);

		assert!(LocksPendingFundingByBitcoinHeight::<Test>::get(expiration_height).contains(&1));
		assert_eq!(LastPendingFundingExpirationHeight::<Test>::get(), None);
		assert_ne!(DefaultVault::get().securitization_locked, 0);
		assert_ne!(DefaultVault::get().securitization_pending_activation, 0);

		FailReturnSecuritization::set(false);
		BitcoinLocks::on_initialize(3);

		assert!(LocksPendingFundingByBitcoinHeight::<Test>::get(expiration_height).is_empty());
		assert_eq!(LastPendingFundingExpirationHeight::<Test>::get(), Some(expiration_height));
		assert_eq!(DefaultVault::get().securitization_locked, 0);
		assert_eq!(DefaultVault::get().securitization_pending_activation, 0);
	});
}

#[test]
fn funding_remains_eligible_after_expiration_cleanup_fails() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(1),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		let expiration_height =
			LocksByUtxoId::<Test>::get(1).expect("lock").funding_expiration_height;

		FailReturnSecuritization::set(true);
		BitcoinBlockHeightChange::set((expiration_height, expiration_height));
		BitcoinLocks::on_initialize(2);
		FailReturnSecuritization::set(false);

		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));
		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		assert!(lock.is_funded());
		assert_eq!(lock.funding_expiration_height, expiration_height);
		assert!(OrphanedUtxosByAccount::<Test>::iter_prefix(1).next().is_none());
	});
}

#[test]
fn expiration_retry_does_not_depend_on_future_bucket_capacity() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(1),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		let expiration_height =
			LocksByUtxoId::<Test>::get(1).expect("lock").funding_expiration_height;
		let retry_height = expiration_height + 1;
		let mut saturated_retry_bucket = BoundedBTreeSet::<_, ConstU32<100>>::new();
		for utxo_id in 1000..1100 {
			saturated_retry_bucket.try_insert(utxo_id).expect("bucket has capacity");
		}
		LocksPendingFundingByBitcoinHeight::<Test>::insert(retry_height, saturated_retry_bucket);

		FailReturnSecuritization::set(true);
		BitcoinBlockHeightChange::set((expiration_height, expiration_height));
		BitcoinLocks::on_initialize(2);

		assert!(LocksPendingFundingByBitcoinHeight::<Test>::get(expiration_height).contains(&1));
		assert_eq!(LocksPendingFundingByBitcoinHeight::<Test>::get(retry_height).len(), 100);
		assert_eq!(
			LocksByUtxoId::<Test>::get(1).expect("lock").funding_expiration_height,
			expiration_height,
		);
	});
}

#[test]
fn create_receive_address_applies_a_delegate_signed_fee_discount() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		allow_fee_coupon_target_rate();
		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let fee = StaticPriceProvider::get_btc_price_in_market_microgons(SATOSHIS_PER_BITCOIN)
			.expect("should have price") /
			10;

		DefaultVault::mutate(|vault| vault.delegate_account_id = Some(9));
		ChargeFee::set(true);
		set_argons(2, fee);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			Some(LockOptions {
				microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
				fee_coupon: Some(fee_coupon(2, SATOSHIS_PER_BITCOIN, fee / 2, 0, 2, 1)),
			}),
		));

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.owner_account, 2);
		assert_eq!(lock.btc_value_in_microgons(), FEE_COUPON_TARGET_RATE);
		assert_eq!(lock.coupon_paid_fees, fee / 2);
		assert_eq!(Balances::free_balance(2), fee / 2);
	});
	set_bitcoin_height(12);
}

#[test]
fn resecuritization_coupon_discounts_the_fee_and_unreserves_vault_space() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		allow_fee_coupon_target_rate();
		DefaultVault::mutate(|vault| {
			vault.delegate_account_id = Some(9);
			vault.reserved_securitization_space = 150_000 * MICROGONS_PER_ARGON;
		});
		ChargeFee::set(true);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			0,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let fee = FEE_COUPON_TARGET_RATE / 10;
		set_argons(2, fee / 2);
		assert_ok!(BitcoinLocks::resecuritize(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			Some(LockOptions {
				microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
				fee_coupon: Some(resecuritization_fee_coupon(
					2,
					1,
					SATOSHIS_PER_BITCOIN,
					fee / 2,
					150_000 * MICROGONS_PER_ARGON,
					2,
					1,
				)),
			}),
		));

		let lock = LocksByUtxoId::<Test>::get(1).expect("resecuritized Lock");
		assert_eq!(lock.security_fees, fee);
		assert_eq!(lock.coupon_paid_fees, fee / 2);
		assert_eq!(Balances::free_balance(2), 0);
		assert_eq!(DefaultVault::get().reserved_securitization_space, 0);
		assert_eq!(LastFeeCouponNonceByVaultAndAccount::<Test>::get(1, 2), Some(1));
	});
}

#[test]
fn resecuritization_coupon_is_bound_to_its_lock_and_nonce() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		allow_fee_coupon_target_rate();
		DefaultVault::mutate(|vault| vault.delegate_account_id = Some(9));
		ChargeFee::set(true);

		for pubkey_byte in 1..=2 {
			assert_ok!(BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				0,
				CompressedBitcoinPubkey([pubkey_byte; 33]),
				None,
			));
		}

		let coupon = resecuritization_fee_coupon(
			2,
			1,
			SATOSHIS_PER_BITCOIN,
			FEE_COUPON_TARGET_RATE / 10,
			0,
			2,
			1,
		);
		let options = Some(LockOptions {
			microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
			fee_coupon: Some(coupon),
		});

		assert_noop!(
			BitcoinLocks::resecuritize(
				RuntimeOrigin::signed(2),
				2,
				SATOSHIS_PER_BITCOIN,
				options.clone(),
			),
			Error::<Test>::InvalidFeeCouponSignature
		);
		assert_ok!(BitcoinLocks::resecuritize(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			options.clone(),
		));
		assert_noop!(
			BitcoinLocks::resecuritize(RuntimeOrigin::signed(2), 1, SATOSHIS_PER_BITCOIN, options,),
			Error::<Test>::FeeCouponAlreadyUsed
		);
	});
}

#[test]
fn fee_coupon_must_match_the_lock_and_delegate_signature() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		DefaultVault::mutate(|vault| vault.delegate_account_id = Some(9));
		set_argons(2, 2_000_000);
		let nonce = System::block_number();

		let create_receive_address = |coupon| {
			BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				CompressedBitcoinPubkey([1; 33]),
				Some(LockOptions {
					microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
					fee_coupon: Some(coupon),
				}),
			)
		};

		assert_err!(
			create_receive_address(fee_coupon(3, SATOSHIS_PER_BITCOIN, 100, 0, 2, nonce)),
			Error::<Test>::InvalidFeeCouponSignature
		);

		let mut invalid_signature = fee_coupon(2, SATOSHIS_PER_BITCOIN, 100, 0, 2, nonce);
		invalid_signature.signature = polkadot_sdk::sp_runtime::testing::TestSignature(8, vec![]);
		assert_err!(
			create_receive_address(invalid_signature),
			Error::<Test>::InvalidFeeCouponSignature
		);

		let mut tampered_discount = fee_coupon(2, SATOSHIS_PER_BITCOIN, 100, 0, 2, nonce);
		tampered_discount.fee_discount += 1;
		assert_err!(
			create_receive_address(tampered_discount),
			Error::<Test>::InvalidFeeCouponSignature
		);

		let mut tampered_flexible = fee_coupon(2, SATOSHIS_PER_BITCOIN, 100, 100, 2, nonce);
		tampered_flexible.securitization_space_to_unreserve += 1;
		assert_err!(
			create_receive_address(tampered_flexible),
			Error::<Test>::InvalidFeeCouponSignature
		);

		CurrentFrameId::set(3);
		assert_err!(
			create_receive_address(fee_coupon(2, SATOSHIS_PER_BITCOIN, 100, 0, 2, nonce)),
			Error::<Test>::FeeCouponExpired
		);
	});
}

#[test]
fn fee_coupon_rejects_tampered_lock_terms() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		DefaultVault::mutate(|vault| vault.delegate_account_id = Some(9));
		set_argons(2, 2_000_000);

		let coupon = fee_coupon(2, SATOSHIS_PER_BITCOIN, 100, 0, 2, 1);
		assert_err!(
			BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN - 1,
				CompressedBitcoinPubkey([1; 33]),
				Some(LockOptions {
					microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
					fee_coupon: Some(coupon),
				}),
			),
			Error::<Test>::InvalidFeeCouponSignature
		);

		let coupon = fee_coupon(2, SATOSHIS_PER_BITCOIN, 100, 0, 2, 1);
		assert_err!(
			BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				CompressedBitcoinPubkey([1; 33]),
				Some(LockOptions {
					microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE + 1,
					fee_coupon: Some(coupon),
				}),
			),
			Error::<Test>::InvalidFeeCouponSignature
		);
	});
}

#[test]
fn replacement_fee_coupons_share_the_next_nonce_until_consumed() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		allow_fee_coupon_target_rate();
		DefaultVault::mutate(|vault| vault.delegate_account_id = Some(9));
		ChargeFee::set(true);
		let fee = StaticPriceProvider::get_btc_price_in_market_microgons(SATOSHIS_PER_BITCOIN)
			.expect("should have price") /
			10;
		set_argons(2, fee / 2);

		let original = fee_coupon(2, SATOSHIS_PER_BITCOIN, 0, 0, 2, 1);
		assert_noop!(
			BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				CompressedBitcoinPubkey([1; 33]),
				Some(LockOptions {
					microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
					fee_coupon: Some(original.clone()),
				}),
			),
			Error::<Test>::InsufficientFunds
		);
		assert_eq!(LastFeeCouponNonceByVaultAndAccount::<Test>::get(1, 2), None);

		let replacement = fee_coupon(2, SATOSHIS_PER_BITCOIN, fee / 2, 0, 2, 1);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([2; 33]),
			Some(LockOptions {
				microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
				fee_coupon: Some(replacement),
			}),
		));
		assert_eq!(LastFeeCouponNonceByVaultAndAccount::<Test>::get(1, 2), Some(1));
		assert_err!(
			BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				CompressedBitcoinPubkey([3; 33]),
				Some(LockOptions {
					microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
					fee_coupon: Some(original),
				}),
			),
			Error::<Test>::FeeCouponAlreadyUsed
		);
	});
}

#[test]
fn fee_coupon_nonce_must_be_exactly_next() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		allow_fee_coupon_target_rate();
		DefaultVault::mutate(|vault| vault.delegate_account_id = Some(9));
		set_argons(2, 2_000_000);

		let create_receive_address = |nonce, pubkey_byte| {
			BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				CompressedBitcoinPubkey([pubkey_byte; 33]),
				Some(LockOptions {
					microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
					fee_coupon: Some(fee_coupon(2, SATOSHIS_PER_BITCOIN, 100, 0, 2, nonce)),
				}),
			)
		};

		assert_err!(create_receive_address(2, 1), Error::<Test>::FeeCouponAlreadyUsed);
		assert_ok!(create_receive_address(1, 2));
		assert_err!(create_receive_address(3, 3), Error::<Test>::FeeCouponAlreadyUsed);
		assert_ok!(create_receive_address(2, 4));
	});
}

#[test]
fn fee_coupon_replay_is_rejected_after_consumption() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		allow_fee_coupon_target_rate();
		DefaultVault::mutate(|vault| vault.delegate_account_id = Some(9));
		set_argons(2, 2_000_000);
		let coupon = fee_coupon(2, SATOSHIS_PER_BITCOIN, 100, 0, 2, 1);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			Some(LockOptions {
				microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
				fee_coupon: Some(coupon.clone()),
			}),
		));
		assert_err!(
			BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				CompressedBitcoinPubkey([2; 33]),
				Some(LockOptions {
					microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
					fee_coupon: Some(coupon),
				}),
			),
			Error::<Test>::FeeCouponAlreadyUsed
		);
	});
}

#[test]
fn fee_coupon_unreserves_securitization_space_atomically() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		allow_fee_coupon_target_rate();
		let satoshis = SATOSHIS_PER_BITCOIN;
		let required_collateral =
			StaticPriceProvider::get_btc_price_in_market_microgons(satoshis).expect("price");
		DefaultVault::mutate(|vault| {
			vault.securitization = required_collateral;
			vault.securitization_target = required_collateral;
		});
		set_argons(1, required_collateral);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(1),
			1,
			satoshis,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, satoshis));
		assert_ok!(BitcoinLocks::set_flexible(RuntimeOrigin::signed(1), 1, true));
		DefaultVault::mutate(|vault| {
			vault
				.set_reserved_securitization_space(required_collateral)
				.expect("reserve securitization space");
		});

		assert_noop!(
			BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				satoshis,
				CompressedBitcoinPubkey([2; 33]),
				None,
			),
			Error::<Test>::InsufficientVaultFunds
		);
		DefaultVault::mutate(|vault| vault.delegate_account_id = Some(9));
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			satoshis,
			CompressedBitcoinPubkey([2; 33]),
			Some(LockOptions {
				microgons_at_target_per_btc: FEE_COUPON_TARGET_RATE,
				fee_coupon: Some(fee_coupon(
					2,
					satoshis,
					0,
					required_collateral.saturating_mul(2),
					2,
					1,
				)),
			}),
		));
		assert_eq!(DefaultVault::get().reserved_securitization_space, 0);
		assert_eq!(LocksByUtxoId::<Test>::get(2).expect("new lock").owner_account, 2);
		assert_noop!(
			BitcoinLocks::set_flexible(RuntimeOrigin::signed(1), 1, false),
			Error::<Test>::InsufficientVaultFunds
		);
		assert!(LocksByUtxoId::<Test>::get(1).expect("flexible lock").is_flexible);
	});
}

#[test]
fn set_flexible_rejects_a_missing_vault() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 100_000 * MICROGONS_PER_ARGON);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(1),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		LocksByUtxoId::<Test>::mutate(1, |lock| {
			lock.as_mut().expect("lock should exist").vault_id = 2;
		});

		assert_noop!(
			BitcoinLocks::set_flexible(RuntimeOrigin::signed(1), 1, true),
			Error::<Test>::VaultNotFound
		);
	});
}

/// Records orphaned UTXOs without funding the lock.
#[test]
fn records_orphaned_utxos_while_lock_pending() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));
		let price = StaticPriceProvider::get_btc_price_in_market_microgons(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().securitization_locked, price);

		let utxo_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };
		assert_ok!(BitcoinLocks::orphaned_utxo_detected(1, 10_000, utxo_ref.clone()));
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(who, &utxo_ref));
		// still waiting for funding
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert!(!lock.is_funded());
	});
}

/// Requests an orphaned UTXO release, tracks cosign state, and clears it after cosign.
#[test]
fn allows_users_to_reclaim_orphaned_utxos() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		let vault_id = 1;
		set_argons(who, 2_000_000);
		let secp = bitcoin::secp256k1::Secp256k1::new();
		let rng = &mut bitcoin::secp256k1::rand::thread_rng();
		let keypair = bitcoin::secp256k1::SecretKey::new(rng);
		let pubkey = keypair.public_key(&secp).serialize();

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			vault_id,
			SATOSHIS_PER_BITCOIN,
			pubkey.into(),
			None
		));
		let price = StaticPriceProvider::get_btc_price_in_market_microgons(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().securitization_locked, price);
		let utxo_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };
		assert_ok!(BitcoinLocks::orphaned_utxo_detected(1, 10_000, utxo_ref.clone()));
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(who, &utxo_ref));
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(who, &utxo_ref));

		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(BitcoinLocks::request_orphaned_utxo_release(
			RuntimeOrigin::signed(who),
			utxo_ref.clone(),
			release_script_pubkey.clone(),
			1000
		));
		assert_eq!(
			VaultViewOfOrphanedUtxoCosigns::get()
				.get(&vault_id)
				.and_then(|entries| entries.get(&who))
				.copied(),
			Some(1)
		);

		let orphan_entry = OrphanedUtxosByAccount::<Test>::get(who, &utxo_ref).unwrap();
		let orphaned_cosign = orphan_entry.cosign_request.unwrap();
		assert_eq!(orphaned_cosign.to_script_pubkey, release_script_pubkey.clone());
		assert_eq!(orphaned_cosign.bitcoin_network_fee, 1000);
		assert_eq!(orphaned_cosign.created_at_argon_block_number, 1);

		let signature = BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec()));
		assert_ok!(BitcoinLocks::cosign_orphaned_utxo_release(
			RuntimeOrigin::signed(1),
			who,
			utxo_ref.clone(),
			signature.clone(),
		));
		System::assert_last_event(
			Event::<Test>::OrphanedUtxoCosigned {
				utxo_id: orphan_entry.utxo_id,
				vault_id,
				utxo_ref: utxo_ref.clone(),
				account_id: who,
				signature,
			}
			.into(),
		);
		assert_eq!(OrphanedUtxosByAccount::<Test>::get(who, &utxo_ref), None);
		assert!(!VaultViewOfOrphanedUtxoCosigns::get().contains_key(&vault_id));
	});
}

/// Allows orphaned UTXO release requests after a lock is canceled.
#[test]
fn allows_orphan_release_after_cancel() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		let vault_id = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			vault_id,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));
		let utxo_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };
		assert_ok!(BitcoinLocks::orphaned_utxo_detected(1, 10_000, utxo_ref.clone()));
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(who, &utxo_ref));

		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			0,
		));
		assert!(LocksByUtxoId::<Test>::get(1).is_none());

		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(BitcoinLocks::request_orphaned_utxo_release(
			RuntimeOrigin::signed(who),
			utxo_ref.clone(),
			release_script_pubkey.clone(),
			1000
		));
		assert!(OrphanedUtxosByAccount::<Test>::get(who, &utxo_ref)
			.unwrap()
			.cosign_request
			.is_some());
		assert_eq!(
			VaultViewOfOrphanedUtxoCosigns::get()
				.get(&vault_id)
				.and_then(|entries| entries.get(&who))
				.copied(),
			Some(1)
		);
	});
}

/// Expires orphaned UTXO release requests after the configured window.
#[test]
fn orphan_release_requests_expire() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));
		let utxo_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };
		assert_ok!(BitcoinLocks::orphaned_utxo_detected(1, 10_000, utxo_ref.clone()));
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(who, &utxo_ref));

		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(BitcoinLocks::request_orphaned_utxo_release(
			RuntimeOrigin::signed(who),
			utxo_ref.clone(),
			release_script_pubkey,
			1000
		));
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			0,
		));
		assert!(LocksByUtxoId::<Test>::get(1).is_none());

		let expires_at = CurrentFrameId::get() + OrphanedUtxoReleaseExpiryFrames::get();
		assert!(OrphanedUtxoExpirationByFrame::<Test>::get(expires_at)
			.contains(&(who, utxo_ref.clone())));
		CurrentFrameId::set(expires_at);
		BitcoinLocks::on_initialize(3);

		assert_eq!(OrphanedUtxosByAccount::<Test>::get(who, &utxo_ref), None);
	});
}

/// Clears orphaned release requests when a lock is externally spent.
#[test]
fn external_spend_clears_orphan_release_requests() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		let vault_id = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			vault_id,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			vault_id,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));

		let utxo_ref = UtxoRef { txid: H256Le([7; 32]), output_index: 0 };
		assert_ok!(BitcoinLocks::orphaned_utxo_detected(1, 10_000, utxo_ref.clone()));
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(who, &utxo_ref));
		let other_utxo_ref = UtxoRef { txid: H256Le([8; 32]), output_index: 0 };
		assert_ok!(BitcoinLocks::orphaned_utxo_detected(2, 15_000, other_utxo_ref.clone()));
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(who, &other_utxo_ref));

		let release_script_pubkey = make_script_pubkey(&[2; 32]);
		assert_ok!(BitcoinLocks::request_orphaned_utxo_release(
			RuntimeOrigin::signed(who),
			utxo_ref.clone(),
			release_script_pubkey,
			1000
		));
		assert!(OrphanedUtxosByAccount::<Test>::get(who, &utxo_ref)
			.unwrap()
			.cosign_request
			.is_some());
		assert_eq!(
			VaultViewOfOrphanedUtxoCosigns::get()
				.get(&vault_id)
				.and_then(|entries| entries.get(&who))
				.copied(),
			Some(1)
		);

		assert_ok!(spent(1));

		assert_eq!(OrphanedUtxosByAccount::<Test>::get(who, &utxo_ref), None);
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(who, &other_utxo_ref));
		assert!(!VaultViewOfOrphanedUtxoCosigns::get().contains_key(&vault_id));
	});
}

/// Rejects duplicate orphaned UTXO release requests.
#[test]
fn orphan_release_request_is_rejected_when_duplicate() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));
		let utxo_ref = UtxoRef { txid: H256Le([3; 32]), output_index: 0 };
		assert_ok!(BitcoinLocks::orphaned_utxo_detected(1, 10_000, utxo_ref.clone()));
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(who, &utxo_ref));

		let release_script_pubkey = make_script_pubkey(&[1; 32]);
		assert_ok!(BitcoinLocks::request_orphaned_utxo_release(
			RuntimeOrigin::signed(who),
			utxo_ref.clone(),
			release_script_pubkey.clone(),
			1000
		));
		assert_noop!(
			BitcoinLocks::request_orphaned_utxo_release(
				RuntimeOrigin::signed(who),
				utxo_ref,
				release_script_pubkey,
				1000
			),
			Error::<Test>::OrphanedUtxoReleaseRequested
		);
	});
}

/// Rejects orphaned UTXO releases that target the funding UTXO ref.
#[test]
fn rejects_release_for_funding_utxo_ref() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));
		let utxo_ref = UtxoRef { txid: H256Le([4; 32]), output_index: 0 };
		UtxoIdToFundingUtxoRef::<Test>::insert(1, utxo_ref.clone());
		assert_ok!(BitcoinLocks::orphaned_utxo_detected(1, 10_000, utxo_ref.clone()));
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(who, &utxo_ref));

		let release_script_pubkey = make_script_pubkey(&[2; 32]);
		assert_noop!(
			BitcoinLocks::request_orphaned_utxo_release(
				RuntimeOrigin::signed(who),
				utxo_ref,
				release_script_pubkey,
				1000
			),
			Error::<Test>::FundingUtxoCannotBeReleased
		);
	});
}

/// Allows release requests even when orphaned releases are pending.
#[test]
fn request_release_allows_pending_orphaned_releases() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));
		let utxo_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };
		assert_ok!(BitcoinLocks::orphaned_utxo_detected(1, 10_000, utxo_ref.clone()));

		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(BitcoinLocks::request_orphaned_utxo_release(
			RuntimeOrigin::signed(who),
			utxo_ref,
			release_script_pubkey,
			1000
		));

		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[1; 32]),
			1000
		));
	});
}

/// Scales lock values and emits events when securitization is increased.

#[test]
fn preset_target_value_is_stored_as_securitization() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let target_rate = 500_000 * MICROGONS_PER_ARGON;
		ArgonPriceInUsd::set(Some(FixedU128::from_float(1.0)));
		ArgonTargetPriceInUsd::set(Some(FixedU128::from_float(1.1)));
		DefaultVault::mutate(|x| {
			x.securitization = 600_000 * MICROGONS_PER_ARGON;
		});
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|x| {
			_ = x.try_push((1, target_rate));
		});

		set_argons(2, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(2),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			Some(LockOptions { microgons_at_target_per_btc: target_rate, fee_coupon: None })
		));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.microgons_at_target_per_btc, target_rate);
		assert_eq!(lock.btc_value_in_microgons(), 500_000 * MICROGONS_PER_ARGON);
	});
}

#[test]
fn classifies_attached_outputs_as_funding_or_orphans() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 2_000_000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(1),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		let funding_ref = UtxoRef { txid: H256Le([1; 32]), output_index: 0 };
		assert_ok!(BitcoinLocks::utxo_detected(1, funding_ref.clone(), SATOSHIS_PER_BITCOIN, 12,));
		assert!(LocksByUtxoId::<Test>::get(1).unwrap().is_funded());
		assert_eq!(UtxoIdToFundingUtxoRef::<Test>::get(1), Some(funding_ref));

		let orphan_ref = UtxoRef { txid: H256Le([2; 32]), output_index: 0 };
		assert_ok!(BitcoinLocks::utxo_detected(1, orphan_ref.clone(), 10_000, 13));
		assert!(OrphanedUtxosByAccount::<Test>::contains_key(1, orphan_ref));
	});
}

#[test]
fn funding_classification_uses_pending_state_not_the_reported_height() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 2_000_000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(1),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		let funding_ref = UtxoRef { txid: H256Le([3; 32]), output_index: 0 };
		assert_ok!(BitcoinLocks::utxo_detected(
			1,
			funding_ref.clone(),
			SATOSHIS_PER_BITCOIN,
			12 + MaxPendingConfirmationBlocks::get() + 1,
		));

		assert!(LocksByUtxoId::<Test>::get(1).unwrap().is_funded());
		assert_eq!(UtxoIdToFundingUtxoRef::<Test>::get(1), Some(funding_ref));
	});
}

#[test]
fn calculates_redemption_amounts() {
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
				.unwrap_or_else(|_| panic!("should parse price {price}"));

			FixedU128::from_float(price)
		}
		fn test_scenario(name: &str, scenario: Scenario) {
			ArgonPriceInUsd::set(Some(parse_price(scenario.argon_price)));
			ArgonTargetPriceInUsd::set(Some(FixedU128::from_u32(1)));
			BitcoinPriceInUsd::set(Some(parse_price(scenario.btc_price)));
			let new_price = BitcoinLocks::calculate_redemption_amount_from_satoshis(
				&SATOSHIS_PER_BITCOIN,
				None,
			)
			.expect("should have price");
			let expected_price = parse_price(scenario.expected_redemption);
			let expected_microgons = expected_price.saturating_mul_int(MICROGONS_PER_ARGON);
			let diff = new_price.abs_diff(expected_microgons);
			assert!(diff <= 1, "{name}: redemption price {new_price} != {expected_microgons}");
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
fn cancels_an_unfunded_spent_bitcoin() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));
		assert!(!LocksByUtxoId::<Test>::get(1).unwrap().is_funded());
		assert_eq!(WatchedUtxosById::get().len(), 1);
		assert_eq!(DefaultVault::get().ratio_adjusted_satoshis, 0);
		// spend before verify
		assert_ok!(spent(1));

		assert_eq!(WatchedUtxosById::get().len(), 0);
		assert_eq!(LocksByUtxoId::<Test>::get(1), None);
		assert_eq!(CanceledLocks::get().len(), 1);
		assert_eq!(DefaultVault::get().ratio_adjusted_satoshis, 0);
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
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey,
			None
		));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(DefaultVault::get().securitization_locked, lock.btc_value_in_microgons());
		let expiration_block = lock.vault_claim_height;
		// first verify
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, lock.btc_value_in_microgons()));

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
		let securitization_at_risk = BitcoinLocks::calculate_redemption_amount_from_satoshis(
			&SATOSHIS_PER_BITCOIN,
			Some(lock.btc_value_in_microgons()),
		)
		.expect("should calculate securitization at risk");
		assert_eq!(
			LockReleaseRequestsByUtxoId::<Test>::get(1).unwrap(),
			LockReleaseRequest {
				utxo_id: 1,
				vault_id: 1,
				cosign_due_frame: CurrentFrameId::get() + LockReleaseCosignDeadlineFrames::get(),
				securitization_at_risk,
				to_script_pubkey: release_script_pubkey,
				bitcoin_network_fee: 1000
			}
		);
		assert!(LockCosignDueByFrame::<Test>::get(
			CurrentFrameId::get() + LockReleaseCosignDeadlineFrames::get()
		)
		.contains(&1));
		assert!(VaultViewOfCosignPendingLocks::get().contains_key(&1));
		assert!(LocksByUtxoId::<Test>::get(1).is_some());
		System::assert_last_event(
			Event::<Test>::BitcoinUtxoCosignRequested { vault_id: 1, utxo_id: 1 }.into(),
		);

		assert_eq!(Balances::free_balance(who), 2_000 + lock.btc_value_in_microgons());
	});
}

#[test]
fn externally_spent_bitcoin_burns_the_vault_securitization() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let who = 1;
		let securitized_satoshis = SATOSHIS_PER_BITCOIN;
		set_argons(who, 2_000);
		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(62_000)));
		ArgonPriceInUsd::set(Some(FixedU128::from_rational(95, 100)));
		let allocated = DefaultVault::get().securitization;

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			securitized_satoshis,
			CompressedBitcoinPubkey([1; 33]),
			None
		));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_ok!(funding_received(1, securitized_satoshis));

		ArgonPriceInUsd::set(Some(FixedU128::from_rational(80, 100)));
		let redemption_amount = BitcoinLocks::calculate_redemption_amount_from_satoshis(
			&securitized_satoshis,
			Some(lock.btc_value_in_microgons()),
		)
		.unwrap();
		let securitization_burned =
			lock.get_securitization().collateral_required().min(redemption_amount);
		pallet_mint::MintedBitcoinMicrogons::<Test>::set(1_000);

		assert_ok!(spent(1));

		assert!(!LocksByUtxoId::<Test>::contains_key(1));
		assert_eq!(WatchedUtxosById::get().len(), 0);
		assert_eq!(DefaultVault::get().securitization_locked, 0);
		assert_eq!(DefaultVault::get().securitization, allocated - securitization_burned);
		assert_eq!(DefaultVault::get().ratio_adjusted_satoshis, 0);
		assert_eq!(pallet_mint::MintedBitcoinMicrogons::<Test>::get(), 1_000);
		System::assert_last_event(
			Event::<Test>::BitcoinLockBurned { vault_id: 1, utxo_id: 1, was_utxo_spent: true }
				.into(),
		);
	});
}

#[test]
fn external_spend_closes_the_fission_without_removing_its_pending_mint() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let account_id = 2;
		set_argons(account_id, 100_000 * MICROGONS_PER_ARGON);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(account_id),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));

		let lock = LocksByUtxoId::<Test>::get(1).expect("lock");
		let fission_rate = lock.microgons_at_target_per_btc;
		MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
			_ = rates.try_push((12, fission_rate));
		});
		assert_ok!(BitcoinFissions::create(
			RuntimeOrigin::signed(account_id),
			0,
			77,
			1,
			SATOSHIS_PER_BITCOIN / 2,
			fission_rate,
		));

		let fission = pallet_bitcoin_fissions::FissionByOwnerAndId::<Test>::get(account_id, 0)
			.expect("fission");
		let pending_indices = pallet_mint::PendingMintUtxoIdLookup::<Test>::get(1);
		assert_eq!(pending_indices.as_slice(), &[0]);

		let redemption_amount = BitcoinLocks::calculate_redemption_amount_from_satoshis(
			&lock.funded_satoshis,
			Some(lock.btc_value_in_microgons()),
		)
		.expect("redemption amount");
		let settlement_burned = lock.securitization_coverage_microgons.min(redemption_amount);
		assert!(settlement_burned >= fission.liquidity_promised);
		pallet_mint::MintedBitcoinMicrogons::<Test>::set(settlement_burned);

		assert_ok!(spent(1));

		assert!(!LocksByUtxoId::<Test>::contains_key(1));
		assert!(!pallet_bitcoin_fissions::FissionByOwnerAndId::<Test>::contains_key(account_id, 0));
		assert_eq!(pallet_mint::PendingMintUtxoIdLookup::<Test>::get(1), pending_indices);
		assert!(pallet_mint::PendingMintUtxosByIndex::<Test>::contains_key(0));
		assert_eq!(
			pallet_mint::MintedBitcoinMicrogons::<Test>::get(),
			settlement_burned - fission.liquidity_promised
		);
		assert_eq!(
			AccountBitcoinChanges::get(),
			vec![
				(account_id, fission.liquidity_promised, true),
				(account_id, fission.liquidity_promised, false),
			]
		);
	});
}

#[test]
fn spent_after_release_request_schedules_securitization_release() {
	set_bitcoin_height(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let who = 1;
		set_argons(who, 2_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			None
		));
		let vault = DefaultVault::get();
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			11
		));
		let cosign_due_frame = CurrentFrameId::get() + LockReleaseCosignDeadlineFrames::get();

		assert_ok!(spent(1));

		assert!(!LocksByUtxoId::<Test>::contains_key(1));
		assert!(!LockReleaseRequestsByUtxoId::<Test>::contains_key(1));
		assert!(LockCosignDueByFrame::<Test>::get(cosign_due_frame).is_empty());
		assert!(VaultViewOfCosignPendingLocks::get().get(&1).unwrap().is_empty());
		assert_eq!(DefaultVault::get().securitization_locked, 0);
		assert_eq!(DefaultVault::get().get_relock_capacity(), lock.btc_value_in_microgons());
		assert_eq!(DefaultVault::get().securitization, vault.securitization);
		assert_eq!(DefaultVault::get().ratio_adjusted_satoshis, 0);
		System::assert_last_event(
			Event::<Test>::BitcoinSpentAfterRelease { vault_id: 1, utxo_id: 1 }.into(),
		);
	});
}

#[test]
fn overdue_cosign_uses_the_frozen_securitization_at_risk() {
	set_bitcoin_height(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let who = 1;
		let funded_satoshis = SATOSHIS_PER_BITCOIN + 5_000;
		set_argons(who, 2_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			funded_satoshis,
			CompressedBitcoinPubkey([1; 33]),
			None
		));
		assert_ok!(funding_received(1, funded_satoshis));
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			2_000
		));
		let request = LockReleaseRequestsByUtxoId::<Test>::get(1).unwrap();
		let cosign_due_frame = request.cosign_due_frame;
		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(10_000)));

		CurrentFrameId::set(cosign_due_frame);
		System::set_block_number(2);
		BitcoinLocks::on_initialize(2);

		assert!(!LocksByUtxoId::<Test>::contains_key(1));
		assert!(!LockReleaseRequestsByUtxoId::<Test>::contains_key(1));
		assert!(LockCosignDueByFrame::<Test>::get(cosign_due_frame).is_empty());
		assert_eq!(DefaultVault::get().ratio_adjusted_satoshis, 0);
		System::assert_last_event(
			Event::<Test>::BitcoinCosignPastDue {
				vault_id: 1,
				utxo_id: 1,
				compensation_amount: 0,
				compensated_account_id: who,
			}
			.into(),
		);
	});
}

#[test]
fn overdue_cosign_returns_a_migrated_release_hold() {
	set_bitcoin_height(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let who = 1;
		let funded_satoshis = SATOSHIS_PER_BITCOIN;
		set_argons(who, 100_000_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			funded_satoshis,
			CompressedBitcoinPubkey([1; 33]),
			None
		));
		assert_ok!(funding_received(1, funded_satoshis));
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			2_000
		));
		let release = LockReleaseRequestsByUtxoId::<Test>::get(1).expect("release");
		MigratedReleaseHoldByUtxoId::<Test>::insert(1, release.securitization_at_risk);

		let hold_reason = HoldReason::ReleaseBitcoinLock.into();
		let providers_before_hold = System::account(who).providers;
		System::inc_providers(&who);
		assert_ok!(Balances::hold(&hold_reason, &who, release.securitization_at_risk));

		CurrentFrameId::set(release.cosign_due_frame);
		BitcoinLocks::on_initialize(2);

		assert_eq!(Balances::balance_on_hold(&hold_reason, &who), 0);
		assert_eq!(System::account(who).providers, providers_before_hold);
		assert!(!MigratedReleaseHoldByUtxoId::<Test>::contains_key(1));
	});
}

#[test]
fn cosigned_release_schedules_securitization_release() {
	set_bitcoin_height(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let who = 2;
		let funded_satoshis = SATOSHIS_PER_BITCOIN + 25_000;
		let secp = bitcoin::secp256k1::Secp256k1::new();
		let owner_pubkey =
			bitcoin::secp256k1::SecretKey::new(&mut bitcoin::secp256k1::rand::thread_rng())
				.public_key(&secp)
				.serialize();
		set_argons(who, 2_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			funded_satoshis,
			owner_pubkey.into(),
			None
		));
		let vault = DefaultVault::get();
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_ok!(funding_received(1, funded_satoshis));
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			11
		));
		let cosign_due_frame = CurrentFrameId::get() + LockReleaseCosignDeadlineFrames::get();
		let signature = BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec()));

		assert_err!(
			BitcoinLocks::cosign_release(RuntimeOrigin::signed(2), 1, signature.clone()),
			Error::<Test>::NoPermissions
		);
		assert_ok!(BitcoinLocks::cosign_release(RuntimeOrigin::signed(1), 1, signature.clone()));

		assert!(!LocksByUtxoId::<Test>::contains_key(1));
		assert!(!LockReleaseRequestsByUtxoId::<Test>::contains_key(1));
		assert!(LockCosignDueByFrame::<Test>::get(cosign_due_frame).is_empty());
		assert_eq!(DefaultVault::get().securitization_locked, 0);
		assert_eq!(DefaultVault::get().get_relock_capacity(), lock.btc_value_in_microgons());
		assert_eq!(DefaultVault::get().securitization, vault.securitization);
		assert_eq!(DefaultVault::get().ratio_adjusted_satoshis, 0);
		assert_eq!(LockReleaseCosignHeightById::<Test>::get(1), Some(1));
		System::assert_last_event(
			Event::<Test>::BitcoinUtxoCosigned { vault_id: 1, utxo_id: 1, signature }.into(),
		);
	});
}

#[test]
fn cosigned_release_retires_a_migrated_release_hold() {
	set_bitcoin_height(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let who = 2;
		let funded_satoshis = SATOSHIS_PER_BITCOIN;
		let secp = bitcoin::secp256k1::Secp256k1::new();
		let owner_pubkey =
			bitcoin::secp256k1::SecretKey::new(&mut bitcoin::secp256k1::rand::thread_rng())
				.public_key(&secp)
				.serialize();
		set_argons(who, 100_000_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			funded_satoshis,
			owner_pubkey.into(),
			None
		));
		assert_ok!(funding_received(1, funded_satoshis));
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			11
		));
		let release = LockReleaseRequestsByUtxoId::<Test>::get(1).expect("release");
		MigratedReleaseHoldByUtxoId::<Test>::insert(1, release.securitization_at_risk);

		let hold_reason = HoldReason::ReleaseBitcoinLock.into();
		let providers_before_hold = System::account(who).providers;
		System::inc_providers(&who);
		assert_ok!(Balances::hold(&hold_reason, &who, release.securitization_at_risk));

		let signature = BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec()));
		assert_ok!(BitcoinLocks::cosign_release(RuntimeOrigin::signed(1), 1, signature));

		assert_eq!(Balances::balance_on_hold(&hold_reason, &who), 0);
		assert_eq!(System::account(who).providers, providers_before_hold);
		assert!(!MigratedReleaseHoldByUtxoId::<Test>::contains_key(1));
	});
}

#[test]
fn test_redemption_amount_vs_market() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BitcoinPriceInUsd::set(Some(FixedU128::from_rational(60_000_50, 1_00)));

		let market_rate =
			StaticPriceProvider::get_btc_price_in_market_microgons(100).expect("should have price");
		assert_eq!(market_rate, 60_000);
		assert_eq!(
			BitcoinLocks::calculate_redemption_amount_from_satoshis(&100, None).unwrap(),
			60_000
		);
	});
}

#[test]
fn overdue_cleanup_clears_stale_cosign_state_when_lock_is_missing() {
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let who = 1;
		let satoshis = SATOSHIS_PER_BITCOIN + 5_000;
		let pubkey = CompressedBitcoinPubkey([1; 33]);
		set_argons(who, 2_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			satoshis,
			pubkey,
			None
		));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_ok!(funding_received(1, satoshis));
		assert_ok!(Balances::mint_into(&who, lock.btc_value_in_microgons()));
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			2_000
		));

		let cosign_due_frame = CurrentFrameId::get() + LockReleaseCosignDeadlineFrames::get();
		assert!(LockReleaseRequestsByUtxoId::<Test>::contains_key(1));
		assert!(LockCosignDueByFrame::<Test>::get(cosign_due_frame).contains(&1));
		assert!(VaultViewOfCosignPendingLocks::get().get(&1).unwrap().contains(&1));
		assert!(UtxoIdsByVaultId::<Test>::contains_key(1, 1));
		assert_eq!(WatchedUtxosById::get().len(), 1);

		LocksByUtxoId::<Test>::remove(1);

		CurrentFrameId::set(cosign_due_frame);
		System::set_block_number(2);
		BitcoinLocks::on_initialize(2);

		assert!(!LockReleaseRequestsByUtxoId::<Test>::contains_key(1));
		assert!(LockCosignDueByFrame::<Test>::get(cosign_due_frame).is_empty());
		assert!(VaultViewOfCosignPendingLocks::get().get(&1).unwrap().is_empty());
		assert!(!UtxoIdsByVaultId::<Test>::contains_key(1, 1));
		assert_eq!(WatchedUtxosById::get().len(), 0);
	});
}

#[test]
fn cosign_release_rejects_invalid_signature_with_real_verifier() {
	new_test_ext().execute_with(|| {
		UseRealBitcoinVerifier::set(true);
		set_bitcoin_height(1);
		System::set_block_number(1);

		let network = bitcoin::Network::Regtest;
		let secp = bitcoin::secp256k1::Secp256k1::new();

		let vault_privkey = bitcoin::PrivateKey::generate(network);
		DefaultVaultBitcoinPubkey::set(vault_privkey.public_key(&secp));
		DefaultVaultReclaimBitcoinPubkey::set(
			bitcoin::PrivateKey::generate(network).public_key(&secp),
		);

		let owner_pubkey: CompressedBitcoinPubkey =
			bitcoin::PrivateKey::generate(network).public_key(&secp).into();
		let who = 2;
		let satoshis = SATOSHIS_PER_BITCOIN + 25_000;
		set_argons(who, 2_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			satoshis,
			owner_pubkey,
			None
		));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.vault_pubkey, vault_privkey.public_key(&secp).into());
		assert_ok!(funding_received(1, satoshis));
		assert_ok!(Balances::mint_into(&who, lock.btc_value_in_microgons()));

		let release_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			release_script_pubkey.clone(),
			11
		));
		UtxoIdToFundingUtxoRef::<Test>::insert(
			1,
			UtxoRef { txid: H256Le([0; 32]), output_index: 0 },
		);

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		let script_args = CosignScriptArgs {
			vault_pubkey: lock.vault_pubkey,
			owner_pubkey: lock.owner_pubkey,
			vault_claim_pubkey: lock.vault_claim_pubkey,
			created_at_height: lock.created_at_height,
			open_claim_height: lock.open_claim_height,
			vault_claim_height: lock.vault_claim_height,
		};
		let releaser = CosignReleaser::new(
			script_args,
			lock.funded_satoshis,
			H256Le([0; 32]).into(),
			0,
			ReleaseStep::VaultCosign,
			Amount::from_sat(11),
			release_script_pubkey.clone().into(),
			GetBitcoinNetwork::get().into(),
		)
		.expect("should build releaser");

		let mut invalid_releaser = releaser.clone();
		let (invalid_sig, _) =
			invalid_releaser.sign(bitcoin::PrivateKey::generate(network)).unwrap();
		let invalid_sig: BitcoinSignature = invalid_sig.try_into().unwrap();

		assert_err!(
			BitcoinLocks::cosign_release(RuntimeOrigin::signed(1), 1, invalid_sig),
			Error::<Test>::BitcoinInvalidCosignature
		);

		let mut valid_releaser = releaser;
		let (valid_sig, _) = valid_releaser.sign(vault_privkey).unwrap();
		let valid_sig: BitcoinSignature = valid_sig.try_into().unwrap();
		assert!(valid_releaser.verify_signature_raw(lock.vault_pubkey, &valid_sig).unwrap());

		assert_ok!(BitcoinLocks::cosign_release(RuntimeOrigin::signed(1), 1, valid_sig));

		UseRealBitcoinVerifier::set(false);
	});
}

#[test]
fn it_rejects_duplicate_release_request() {
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		let satoshis = SATOSHIS_PER_BITCOIN;
		set_argons(who, 2_000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			satoshis,
			pubkey,
			None
		));
		assert_ok!(funding_received(1, satoshis));
		assert_ok!(Balances::mint_into(&who, 200_000 * MICROGONS_PER_ARGON));

		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			10
		));
		assert_noop!(
			BitcoinLocks::request_release(
				RuntimeOrigin::signed(who),
				1,
				make_script_pubkey(&[0; 32]),
				10
			),
			Error::<Test>::LockInProcessOfRelease
		);
	});
}

#[test]
fn funding_amount_does_not_rewrite_securitization() {
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		let satoshis = 150_000_000;
		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(100)));
		let apr = FixedU128::from_float(0.00000001);
		DefaultVault::mutate(|a| {
			a.securitization = 250_000_000_000;
			a.terms.bitcoin_base_fee = 1000;
			a.terms.bitcoin_annual_percent_rate = apr;
		});
		set_argons(who, 5000);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			satoshis,
			pubkey,
			None
		));

		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.btc_value_in_microgons(), 150_000_000);
		assert_eq!(lock.security_fees, 1000 + 1);

		assert_ok!(funding_received(1, satoshis + 5000));
		let lock = LocksByUtxoId::<Test>::get(1).unwrap();
		assert_eq!(lock.btc_value_in_microgons(), 150_000_000);
		assert_eq!(lock.funded_satoshis, satoshis + 5000);
		assert_eq!(lock.security_fees, 1000 + 1, "fees shouldn't change");
		assert_eq!(DefaultVault::get().locked_satoshis, satoshis + 5000);
		assert_eq!(DefaultVault::get().ratio_adjusted_satoshis, satoshis);

		// A smaller funding output is tracked independently from the existing coverage.
		let pubkey2 = CompressedBitcoinPubkey([2; 33]);
		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(who),
			1,
			100_000_000,
			pubkey2,
			None
		));
		let lock = LocksByUtxoId::<Test>::get(2).unwrap();
		assert_eq!(lock.btc_value_in_microgons(), 100_000_000);
		assert_eq!(lock.security_fees, 1000 + 1);

		let actual_sats: Satoshis = 100_000_000 - 3000;
		assert_ok!(funding_received(2, actual_sats));
		let lock = LocksByUtxoId::<Test>::get(2).unwrap();
		assert_eq!(lock.btc_value_in_microgons(), 100_000_000);
		assert_eq!(lock.funded_satoshis, actual_sats);
		assert_eq!(lock.securitized_satoshis, 100_000_000);
		assert_eq!(lock.security_fees, 1000 + 1, "fees shouldn't change");
		assert_eq!(DefaultVault::get().locked_satoshis, satoshis + 5000 + actual_sats);
		assert_eq!(DefaultVault::get().ratio_adjusted_satoshis, satoshis + actual_sats);
	});
}

#[test]
fn underfunded_lock_uses_funded_satoshis_when_set_flexible() {
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let securitized_satoshis = SATOSHIS_PER_BITCOIN;
		let funded_satoshis = securitized_satoshis - 3_000;
		set_argons(1, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(1),
			1,
			securitized_satoshis,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, funded_satoshis));
		assert_ok!(BitcoinLocks::set_flexible(RuntimeOrigin::signed(1), 1, true));

		let vault = DefaultVault::get();
		assert_eq!(vault.flexible_ratio_adjusted_satoshis, funded_satoshis);
		assert!(LocksByUtxoId::<Test>::get(1).expect("lock").is_flexible);
	});
}

#[test]
fn underfunded_lock_uses_funded_satoshis_when_spent_externally() {
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let securitized_satoshis = SATOSHIS_PER_BITCOIN;
		let funded_satoshis = securitized_satoshis - 3_000;
		set_argons(1, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(1),
			1,
			securitized_satoshis,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, funded_satoshis));
		assert_ok!(spent(1));

		let vault = DefaultVault::get();
		assert_eq!(vault.locked_satoshis, 0);
		assert_eq!(vault.ratio_adjusted_satoshis, 0);
		assert!(!LocksByUtxoId::<Test>::contains_key(1));
	});
}

#[test]
fn underfunded_lock_uses_funded_satoshis_when_cosign_is_overdue() {
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		let securitized_satoshis = SATOSHIS_PER_BITCOIN;
		let funded_satoshis = securitized_satoshis - 3_000;
		set_argons(1, 2_000_000);

		assert_ok!(BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(1),
			1,
			securitized_satoshis,
			CompressedBitcoinPubkey([1; 33]),
			None,
		));
		assert_ok!(funding_received(1, funded_satoshis));
		assert_ok!(BitcoinLocks::request_release(
			RuntimeOrigin::signed(1),
			1,
			make_script_pubkey(&[0; 32]),
			1_000,
		));
		assert_ok!(BitcoinLocks::cosign_bitcoin_overdue(1));

		let vault = DefaultVault::get();
		assert_eq!(vault.locked_satoshis, 0);
		assert_eq!(vault.ratio_adjusted_satoshis, 0);
		assert!(!LocksByUtxoId::<Test>::contains_key(1));
	});
}

#[test]
fn redemption_amount_matches_market_rate_at_non_one_dollar_target() {
	new_test_ext().execute_with(|| {
		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(55_000)));
		ArgonPriceInUsd::set(Some(FixedU128::from_rational(110, 100)));
		ArgonTargetPriceInUsd::set(Some(FixedU128::from_rational(110, 100)));

		let market_rate =
			StaticPriceProvider::get_btc_price_in_market_microgons(SATOSHIS_PER_BITCOIN)
				.expect("should have price");
		let redemption_amount =
			BitcoinLocks::calculate_redemption_amount_from_satoshis(&SATOSHIS_PER_BITCOIN, None)
				.expect("should have redemption price");

		assert_eq!(market_rate, 50_000 * MICROGONS_PER_ARGON);
		assert_eq!(redemption_amount, market_rate);
	});
}

#[test]
fn it_should_record_btc_history() {
	new_test_ext().execute_with(|| {
		set_bitcoin_height(1);
		System::set_block_number(1);

		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(100_000)));

		for i in 1..=12 {
			CurrentTick::set(i);
			System::initialize(
				&(System::block_number() + 1),
				&System::parent_hash(),
				&Default::default(),
			);
			BitcoinLocks::on_initialize(System::block_number());
			BitcoinLocks::on_finalize(System::block_number());
		}
		// only changes when first one expires
		assert_eq!(
			MicrogonsAtTargetPerBtcHistory::<Test>::get().to_vec(),
			vec![(12, 100_000 * MICROGONS_PER_ARGON)]
		);

		// set new price
		BitcoinPriceInUsd::set(Some(FixedU128::saturating_from_integer(120_000)));
		CurrentTick::set(13);
		System::initialize(
			&(System::block_number() + 1),
			&System::parent_hash(),
			&Default::default(),
		);
		BitcoinLocks::on_initialize(System::block_number());
		BitcoinLocks::on_finalize(System::block_number());
		assert_eq!(
			MicrogonsAtTargetPerBtcHistory::<Test>::get().to_vec(),
			vec![(12, 100_000 * MICROGONS_PER_ARGON), (13, 120_000 * MICROGONS_PER_ARGON)]
		);
	});
}

fn lock_options(microgons_at_target_per_btc: Balance) -> Option<LockOptions<Test>> {
	Some(LockOptions { microgons_at_target_per_btc, fee_coupon: None })
}

fn fee_coupon(
	beneficiary: u64,
	satoshis: Satoshis,
	fee_discount: Balance,
	securitization_space_to_unreserve: Balance,
	expires_at_frame: FrameId,
	nonce: u64,
) -> FeeCoupon<Test> {
	signed_fee_coupon(
		beneficiary,
		None,
		satoshis,
		fee_discount,
		securitization_space_to_unreserve,
		expires_at_frame,
		nonce,
	)
}

fn resecuritization_fee_coupon(
	beneficiary: u64,
	utxo_id: UtxoId,
	satoshis: Satoshis,
	fee_discount: Balance,
	securitization_space_to_unreserve: Balance,
	expires_at_frame: FrameId,
	nonce: u64,
) -> FeeCoupon<Test> {
	signed_fee_coupon(
		beneficiary,
		Some(utxo_id),
		satoshis,
		fee_discount,
		securitization_space_to_unreserve,
		expires_at_frame,
		nonce,
	)
}

fn signed_fee_coupon(
	beneficiary: u64,
	utxo_id: Option<UtxoId>,
	satoshis: Satoshis,
	fee_discount: Balance,
	securitization_space_to_unreserve: Balance,
	expires_at_frame: FrameId,
	nonce: u64,
) -> FeeCoupon<Test> {
	let genesis_hash = System::block_hash(0);
	let message = (
		FEE_COUPON_MESSAGE_KEY,
		genesis_hash,
		1u32,
		beneficiary,
		utxo_id,
		satoshis,
		FEE_COUPON_TARGET_RATE,
		fee_discount,
		securitization_space_to_unreserve,
		expires_at_frame,
		nonce,
	)
		.using_encoded(blake2_256);
	FeeCoupon {
		fee_discount,
		securitization_space_to_unreserve,
		expires_at_frame,
		nonce,
		signature: polkadot_sdk::sp_runtime::testing::TestSignature(9, message.to_vec()),
	}
}

fn allow_fee_coupon_target_rate() {
	MicrogonsAtTargetPerBtcHistory::<Test>::mutate(|rates| {
		rates
			.try_push((1, FEE_COUPON_TARGET_RATE))
			.expect("fee coupon target rate should fit");
	});
}

fn make_script_pubkey(vec: &[u8]) -> BitcoinScriptPubkey {
	BitcoinScriptPubkey(BoundedVec::try_from(vec.to_vec()).unwrap())
}
