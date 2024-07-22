use frame_support::{assert_err, assert_ok};
use sp_arithmetic::{FixedPointNumber, FixedU128};

use ulx_primitives::{bitcoin::SATOSHIS_PER_BITCOIN, ArgonCPI, PriceProvider};

use crate::{mock::*, Current, Operator, PriceIndex as PriceIndexEntry};

type Event = crate::Event<Test>;
type Error = crate::Error<Test>;

#[test]
fn should_require_an_operator_to_submit() {
	new_test_ext(None).execute_with(|| {
		System::set_block_number(1);
		assert_err!(
			PriceIndex::submit(RuntimeOrigin::signed(1), create_index()),
			Error::NotAuthorizedOperator
		);

		assert!(System::events().is_empty());
	});
}

#[test]
fn can_set_an_operator() {
	new_test_ext(None).execute_with(|| {
		System::set_block_number(1);
		assert_err!(
			PriceIndex::submit(RuntimeOrigin::signed(1), create_index()),
			Error::NotAuthorizedOperator
		);

		assert_ok!(PriceIndex::set_operator(RuntimeOrigin::root(), 1));

		assert_eq!(Operator::<Test>::get(), Some(1));
		System::assert_last_event(Event::OperatorChanged { operator_id: 1 }.into());
	});
}

#[test]
fn can_set_a_price_index() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);
		let entry = create_index();
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry),);
		assert_eq!(Current::<Test>::get(), Some(entry));

		System::assert_last_event(Event::NewIndex.into());
	});
}

#[test]
fn uses_latest_as_current() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);

		let mut entry = create_index();
		entry.tick = 1;
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry),);
		assert_eq!(Current::<Test>::get(), Some(entry));
		System::assert_last_event(Event::NewIndex.into());

		let mut entry2 = entry;
		entry2.argon_usd_target_price = FixedU128::from_float(1.01);
		entry2.tick = 3;
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry2),);
		assert_eq!(Current::<Test>::get(), Some(entry2));

		let mut entry_backwards = entry;
		entry_backwards.argon_usd_target_price = FixedU128::from_float(1.02);
		entry_backwards.tick = 2;
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry_backwards),);
		assert_eq!(Current::<Test>::get(), Some(entry2));
	});
}

#[test]
fn doesnt_use_expired_values() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);
		MaxPriceAgeInTicks::set(10);
		CurrentTick::set(12);
		let mut entry = create_index();
		entry.tick = 1;
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry));
		assert_eq!(Current::<Test>::get(), None);
	});
}

#[test]
fn can_convert_argon_prices() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);
		let mut index = PriceIndexEntry {
			tick: 1,
			btc_usd_price: FixedU128::from_float(62_000.00), // 62,000.00
			argon_usd_price: FixedU128::from_float(1.00),    // 100 cents
			argon_usd_target_price: FixedU128::from_float(1.00),
		};
		Current::<Test>::put(index);

		assert_eq!(
			<PriceIndex as PriceProvider<u128>>::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN),
			Some(62_000 * 1000),
			"price in milligons"
		);

		index.argon_usd_price = FixedU128::from_float(1.01);
		Current::<Test>::put(index);

		assert_eq!(
			<PriceIndex as PriceProvider<u128>>::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN),
			Some(1000 * (62_000 * 100) / 101),
		);
	});
}

#[test]
fn clamps_argon_price_changes_away_from_target() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);
		let base_entry = PriceIndexEntry {
			tick: 1,
			btc_usd_price: FixedU128::from_float(62_000.00), // 62,000.00
			argon_usd_price: FixedU128::from_float(1.00),    // 100 cents
			argon_usd_target_price: FixedU128::from_float(1.00),
		};
		Current::<Test>::put(base_entry);
		MaxPriceAgeInTicks::set(10);
		CurrentTick::set(12);
		let mut next = base_entry.clone();
		next.tick = 2;
		// if we're in inflation, price can't go up 2 cents per tick
		next.argon_usd_target_price = FixedU128::from_float(1.00);
		next.argon_usd_price = FixedU128::from_float(1.02);
		PriceIndex::clamp_argon_prices(&base_entry, &mut next);
		assert_eq!(next.argon_usd_price, FixedU128::from_float(1.01));

		// if we're in deflation, price can't go down 2 cents per tick
		next.argon_usd_target_price = FixedU128::from_float(1.00);
		next.argon_usd_price = FixedU128::from_float(0.98);
		PriceIndex::clamp_argon_prices(&base_entry, &mut next);
		assert_eq!(next.argon_usd_price, FixedU128::from_float(0.99));

		// but it will allow a scaled amount
		next.tick = 3;
		next.argon_usd_target_price = FixedU128::from_float(1.00);
		next.argon_usd_price = FixedU128::from_float(0.98);
		PriceIndex::clamp_argon_prices(&base_entry, &mut next);
		assert_eq!(next.argon_usd_price, FixedU128::from_float(0.98));

		// no limit on price decrease if we're in inflation
		next.tick = 2;
		next.argon_usd_target_price = FixedU128::from_float(0.97);
		next.argon_usd_price = FixedU128::from_float(0.98);
		PriceIndex::clamp_argon_prices(&base_entry, &mut next);
		assert_eq!(next.argon_usd_price, FixedU128::from_float(0.98));
	});
}

#[test]
fn clamps_argon_target_price_changes() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);
		let base_entry = PriceIndexEntry {
			tick: 1,
			btc_usd_price: FixedU128::from_float(62_000.00), // 62,000.00
			argon_usd_price: FixedU128::from_float(1.00),    // 100 cents
			argon_usd_target_price: FixedU128::from_float(1.00),
		};
		Current::<Test>::put(base_entry);
		MaxPriceAgeInTicks::set(10);
		CurrentTick::set(12);
		let mut next = base_entry.clone();
		next.tick = 2;
		next.argon_usd_target_price = FixedU128::from_float(1.05);
		PriceIndex::clamp_argon_prices(&base_entry, &mut next);
		assert_eq!(next.argon_usd_target_price, FixedU128::from_float(1.01));

		next.tick = 5;
		next.argon_usd_target_price = FixedU128::from_float(1.05);
		PriceIndex::clamp_argon_prices(&base_entry, &mut next);
		// clamps to 4 ticks worth
		assert_eq!(next.argon_usd_target_price, FixedU128::from_float(1.04));

		next.tick = 2;
		next.argon_usd_target_price = FixedU128::from_float(0.95);
		PriceIndex::clamp_argon_prices(&base_entry, &mut next);
		assert_eq!(next.argon_usd_target_price, FixedU128::from_float(0.99));
	});
}

#[test]
fn price_below_target_means_deflation() {
	let mut price_index = create_index();
	price_index.argon_usd_price = FixedU128::from_float(1.00);
	price_index.argon_usd_target_price = FixedU128::from_float(1.10);

	assert!(price_index.argon_cpi().is_positive());
}

#[test]
fn price_above_target_means_inflation() {
	let mut price_index = create_index();
	price_index.argon_usd_price = FixedU128::from_float(1.15);
	price_index.argon_usd_target_price = FixedU128::from_float(1.10);

	assert!(price_index.argon_cpi().is_negative());
}

#[test]
fn equilibrium_should_have_0_cpi() {
	let mut price_index = create_index();
	price_index.argon_usd_price = FixedU128::from_float(1.15);
	price_index.argon_usd_target_price = FixedU128::from_float(1.15);

	assert_eq!(price_index.argon_cpi().round(), ArgonCPI::from_float(0.0));
}

fn create_index() -> PriceIndexEntry {
	PriceIndexEntry {
		tick: 0,
		btc_usd_price: FixedU128::from_float(62_000.00),
		argon_usd_price: FixedU128::from_float(1.0),
		argon_usd_target_price: FixedU128::from_float(1.0),
	}
}
