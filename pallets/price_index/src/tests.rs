use pallet_prelude::*;

use argon_primitives::{
	bitcoin::SATOSHIS_PER_BITCOIN, ArgonCPI, PriceProvider, MICROGONS_PER_ARGON,
};

use crate::{
	mock::*, CpiMeasurementBucket, Current, HistoricArgonCPI, HistoricArgonotAverageByFrame,
	HistoricArgonotFloorByFrame, Operator, PriceIndex as PriceIndexEntry,
};

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
fn it_should_keep_trailing_averages() {
	new_test_ext(None).execute_with(|| {
		// test out that buckets fall inside the range
		let mut cpi = CpiMeasurementBucket::default();
		cpi.record(ArgonCPI::from_float(0.5));
		assert_eq!(cpi.average(), ArgonCPI::from_float(0.5));
		cpi.record(ArgonCPI::from_float(1.0));
		cpi.record(ArgonCPI::from_float(1.5));
		assert_eq!(cpi.average(), ArgonCPI::from_float(1.0));
	})
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
fn stores_history_grouped() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);

		CurrentTick::set(181);
		let mut entry = create_index();
		entry.tick = 181;
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry),);
		assert_eq!(Current::<Test>::get(), Some(entry));
		System::assert_last_event(Event::NewIndex.into());
		assert_eq!(HistoricArgonCPI::<Test>::get().len(), 1);
		assert!(HistoricArgonCPI::<Test>::get()
			.iter()
			.any(|a| a.tick_range.0 == 180 && a.tick_range.1 == 240));

		let mut entry2 = entry;
		entry2.argon_usd_target_price = FixedU128::from_float(1.01);
		entry2.tick = 183;
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry2),);
		assert_eq!(HistoricArgonCPI::<Test>::get().len(), 1);
		assert_eq!(HistoricArgonCPI::<Test>::get()[0].measurements_count, 2);
		assert_eq!(HistoricArgonCPI::<Test>::get()[0].tick_range.0, 180);

		let mut entry_backwards = entry;
		entry_backwards.argon_usd_target_price = FixedU128::from_float(1.02);
		entry_backwards.tick = 241;
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry_backwards),);

		assert_eq!(HistoricArgonCPI::<Test>::get().len(), 2);
		assert_eq!(
			HistoricArgonCPI::<Test>::get()
				.iter()
				.find(|a| a.tick_range.0 == 240)
				.unwrap()
				.measurements_count,
			1
		);
		assert_eq!(HistoricArgonCPI::<Test>::get()[0].tick_range.0, 240, "should be newest first");
	});
}

#[test]
fn tracks_lowest_argonot_floor_per_frame() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);

		CurrentTick::set(5);
		CurrentFrameId::set(0);
		let mut first = create_index();
		first.tick = 5;
		first.argonot_usd_price = FixedU128::from_rational(5, 2);
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), first));

		CurrentTick::set(9);
		CurrentFrameId::set(0);
		let mut lower_same_frame = first;
		lower_same_frame.tick = 9;
		lower_same_frame.argonot_usd_price = FixedU128::from_rational(3, 2);
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), lower_same_frame));

		CurrentTick::set(12);
		CurrentFrameId::set(1);
		let mut next_frame = first;
		next_frame.tick = 12;
		next_frame.argonot_usd_price = FixedU128::from_rational(7, 2);
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), next_frame));

		let history = HistoricArgonotFloorByFrame::<Test>::get();
		assert_eq!(history.len(), 2);
		assert_eq!(history.get(&0), Some(&1_500_000));
		assert_eq!(history.get(&1), Some(&3_500_000));
		assert_eq!(
			<PriceIndex as PriceProvider<u128>>::get_lowest_microgons_per_argonot(1),
			Some(3_500_000),
		);
		assert_eq!(
			<PriceIndex as PriceProvider<u128>>::get_lowest_microgons_per_argonot(2),
			Some(1_500_000),
		);
	});
}

#[test]
fn carries_argonot_floor_forward_to_new_frames_without_new_submissions() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);

		CurrentTick::set(12);
		CurrentFrameId::set(1);
		let mut index = create_index();
		index.tick = 12;
		index.argonot_usd_price = FixedU128::from_u32(2);
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), index));

		CurrentTick::set(21);
		CurrentFrameId::set(2);
		PriceIndex::on_initialize(2);

		let history = HistoricArgonotFloorByFrame::<Test>::get();
		assert_eq!(history.len(), 2);
		assert_eq!(history.get(&1), Some(&(2 * MICROGONS_PER_ARGON)));
		assert_eq!(history.get(&2), Some(&(2 * MICROGONS_PER_ARGON)));
		assert_eq!(
			<PriceIndex as PriceProvider<u128>>::get_lowest_microgons_per_argonot(2),
			Some(2 * MICROGONS_PER_ARGON),
		);
	});
}

#[test]
fn buckets_transition_block_floor_samples_under_the_new_frame() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);

		CurrentTick::set(12);
		CurrentFrameId::set(1);
		let mut index = create_index();
		index.tick = 12;
		index.argonot_usd_price = FixedU128::from_u32(2);
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), index));

		NewlyStartedFrameId::set(Some(2));
		CurrentTick::set(13);
		let mut transition_block_index = index;
		transition_block_index.tick = 13;
		transition_block_index.argonot_usd_price = FixedU128::from_u32(3);
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), transition_block_index));

		let history = HistoricArgonotFloorByFrame::<Test>::get();
		assert_eq!(history.get(&1), Some(&(2 * MICROGONS_PER_ARGON)));
		assert_eq!(history.get(&2), Some(&(3 * MICROGONS_PER_ARGON)));
	});
}

#[test]
fn buckets_transition_block_initialize_floor_samples_under_the_new_frame() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);

		CurrentTick::set(12);
		CurrentFrameId::set(1);
		let mut index = create_index();
		index.tick = 12;
		index.argonot_usd_price = FixedU128::from_u32(2);
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), index));

		NewlyStartedFrameId::set(Some(2));
		CurrentTick::set(13);
		PriceIndex::on_initialize(2);

		let history = HistoricArgonotFloorByFrame::<Test>::get();
		assert_eq!(history.get(&1), Some(&(2 * MICROGONS_PER_ARGON)));
		assert_eq!(history.get(&2), Some(&(2 * MICROGONS_PER_ARGON)));
	});
}

#[test]
fn finalizes_argonot_average_for_completed_frames() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);

		let mut first = create_index();
		first.tick = 5;
		first.argonot_usd_price = FixedU128::from_u32(2);
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), first));

		CurrentTick::set(5);
		CurrentFrameId::set(0);
		PriceIndex::on_initialize(1);

		let mut second = first;
		second.tick = 7;
		second.argonot_usd_price = FixedU128::from_u32(4);
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), second));

		CurrentTick::set(7);
		CurrentFrameId::set(0);
		PriceIndex::on_initialize(2);

		NewlyStartedFrameId::set(Some(1));
		CurrentTick::set(8);
		PriceIndex::on_initialize(3);

		let history = HistoricArgonotAverageByFrame::<Test>::get();
		assert_eq!(history.get(&0), Some(&(3 * MICROGONS_PER_ARGON)));
		assert_eq!(
			<PriceIndex as PriceProvider<u128>>::get_average_microgons_per_argonot(0),
			Some(3 * MICROGONS_PER_ARGON),
		);
	});
}

#[test]
fn carries_forward_previous_argonot_average_when_a_frame_has_no_samples() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);

		let mut index = create_index();
		index.tick = 1;
		index.argonot_usd_price = FixedU128::from_u32(2);
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), index));

		CurrentTick::set(1);
		CurrentFrameId::set(1);
		PriceIndex::on_initialize(1);

		NewlyStartedFrameId::set(Some(2));
		CurrentTick::set(100);
		PriceIndex::on_initialize(2);

		NewlyStartedFrameId::set(Some(3));
		CurrentTick::set(101);
		PriceIndex::on_initialize(3);

		let history = HistoricArgonotAverageByFrame::<Test>::get();
		assert_eq!(history.get(&1), Some(&(2 * MICROGONS_PER_ARGON)));
		assert_eq!(history.get(&2), Some(&(2 * MICROGONS_PER_ARGON)));
	});
}

#[test]
fn can_find_a_range() {
	new_test_ext(Some(1)).execute_with(|| {
		HistoricArgonCPI::<Test>::put(BoundedVec::truncate_from(vec![
			CpiMeasurementBucket {
				tick_range: (80, 140),
				total_cpi: ArgonCPI::from_float(1.0),
				measurements_count: 1,
			},
			CpiMeasurementBucket {
				tick_range: (140, 200),
				total_cpi: ArgonCPI::from_float(2.0),
				measurements_count: 1,
			},
			CpiMeasurementBucket {
				tick_range: (200, 260),
				total_cpi: ArgonCPI::from_float(3.0),
				measurements_count: 1,
			},
			CpiMeasurementBucket {
				tick_range: (260, 320),
				total_cpi: ArgonCPI::from_float(4.0),
				measurements_count: 1,
			},
		]));

		assert_eq!(
			PriceIndex::get_average_cpi_for_ticks((80, 320)),
			ArgonCPI::from_float(10.0 / 4.0)
		);
		assert_eq!(
			PriceIndex::get_average_cpi_for_ticks((120, 320)),
			ArgonCPI::from_float(9.0 / 3.0)
		);
		assert_eq!(PriceIndex::get_average_cpi_for_ticks((200, 280)), ArgonCPI::from_float(3.0));
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
			argonot_usd_price: FixedU128::from_float(2.00),
			argon_time_weighted_average_liquidity: 100_000_000,
		};
		Current::<Test>::put(index);

		assert_eq!(
			<PriceIndex as PriceProvider<u128>>::get_btc_price_in_market_microgons(
				SATOSHIS_PER_BITCOIN
			),
			Some(62_000 * 1_000_000),
			"price in microgons"
		);

		index.argon_usd_price = FixedU128::from_float(1.01);
		Current::<Test>::put(index);

		assert_eq!(
			<PriceIndex as PriceProvider<u128>>::get_btc_price_in_market_microgons(
				SATOSHIS_PER_BITCOIN
			),
			Some(1_000_000 * (62_000 * 100) / 101),
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
			argonot_usd_price: FixedU128::from_float(2.00),
			argon_time_weighted_average_liquidity: 100_000_000,
		};
		Current::<Test>::put(base_entry);
		MaxPriceAgeInTicks::set(10);
		CurrentTick::set(12);
		let mut next = base_entry;
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
			argonot_usd_price: FixedU128::from_float(2.00),
			argon_time_weighted_average_liquidity: 100_000_000,
		};
		Current::<Test>::put(base_entry);
		MaxPriceAgeInTicks::set(10);
		CurrentTick::set(12);
		let mut next = base_entry;
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
fn price_within_thousandth_of_target_is_on_target() {
	let mut price_index = create_index();
	price_index.argon_usd_price = FixedU128::from_float(1.0995);
	price_index.argon_usd_target_price = FixedU128::from_float(1.10);

	assert_eq!(price_index.argon_cpi(), ArgonCPI::from_float(0.0));

	price_index.argon_usd_price = FixedU128::from_float(1.10005);
	assert_eq!(price_index.argon_cpi(), ArgonCPI::from_float(0.0));

	price_index.argon_usd_price = FixedU128::from_float(1.11004);
	assert_ne!(price_index.argon_cpi(), ArgonCPI::from_float(0.0));
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
		argonot_usd_price: FixedU128::from_float(2.00),
		argon_time_weighted_average_liquidity: 100_000_000,
	}
}
