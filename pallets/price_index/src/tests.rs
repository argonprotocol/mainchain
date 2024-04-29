use std::time::SystemTime;

use frame_support::{assert_err, assert_ok};
use sp_runtime::BoundedVec;

use ulx_primitives::{bitcoin::SATOSHIS_PER_BITCOIN, BitcoinPriceProvider, Moment};

use crate::{mock::*, Current, History, Operator, PriceIndex as PriceIndexEntry};

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
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry.clone()),);
		assert_eq!(Current::<Test>::get(), Some(entry.clone()));
		assert_eq!(History::<Test>::get().to_vec(), vec![entry]);

		System::assert_last_event(Event::NewIndex { price_index: entry.clone() }.into());
	});
}

#[test]
fn uses_latest_as_current() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);

		let start = now();
		let mut entry = create_index();
		entry.timestamp = start;
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry.clone()),);
		assert_eq!(Current::<Test>::get(), Some(entry.clone()));
		assert_eq!(History::<Test>::get().to_vec(), vec![entry]);
		System::assert_last_event(Event::NewIndex { price_index: entry.clone() }.into());

		let mut entry2 = entry.clone();
		entry2.argon_cpi = 1;
		entry2.timestamp = start + 4;
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry2.clone()),);
		assert_eq!(Current::<Test>::get(), Some(entry2.clone()));
		assert_eq!(History::<Test>::get().to_vec(), vec![entry2, entry]);

		let mut entry_backwards = entry.clone();
		entry_backwards.argon_cpi = 2;
		entry_backwards.timestamp = start + 1;
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry_backwards.clone()),);
		assert_eq!(Current::<Test>::get(), Some(entry2.clone()));
		assert_eq!(History::<Test>::get().to_vec(), vec![entry2, entry_backwards, entry]);
	});
}

#[test]
fn doesnt_use_expired_values() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);
		OldestHistoryToKeep::set(10);
		let mut entry = create_index();
		entry.timestamp = now() - 11;
		assert_err!(
			PriceIndex::submit(RuntimeOrigin::signed(1), entry.clone()),
			Error::PricesTooOld
		);
		assert_eq!(Current::<Test>::get(), None);
		assert_eq!(History::<Test>::get().to_vec(), vec![]);
	});
}

#[test]
fn removes_expired_values() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);
		OldestHistoryToKeep::set(100);
		let mut index1 = create_index();
		index1.timestamp = now() - 101;
		let mut index2 = create_index();
		index2.timestamp = now() - 10;

		History::<Test>::put(BoundedVec::truncate_from(vec![index1, index2]));
		let entry = create_index();
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry.clone()),);
		assert_eq!(Current::<Test>::get(), Some(entry));
		assert_eq!(History::<Test>::get().to_vec(), vec![entry, index2]);
	});
}

#[test]
fn handles_overflowing_history() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);
		OldestHistoryToKeep::set(100);
		MaxHistoryToKeep::set(2);
		let index1 = create_index();
		let index2 = create_index();

		History::<Test>::put(BoundedVec::truncate_from(vec![index1, index2]));
		let entry = create_index();
		assert_ok!(PriceIndex::submit(RuntimeOrigin::signed(1), entry.clone()),);
		assert_eq!(Current::<Test>::get(), Some(entry));
		assert_eq!(History::<Test>::get().to_vec(), vec![entry, index2]);
	});
}

#[test]
fn can_convert_argon_prices() {
	new_test_ext(Some(1)).execute_with(|| {
		System::set_block_number(1);
		let mut index = PriceIndexEntry {
			timestamp: now(),
			btc_usd_price: 62_000 * 100, // 62,000.00
			argon_usd_price: 100,        // 100 cents
			argon_cpi: 0,
		};
		Current::<Test>::put(index);

		assert_eq!(
			<PriceIndex as BitcoinPriceProvider<u128>>::get_bitcoin_argon_price(
				SATOSHIS_PER_BITCOIN
			),
			Some(62_000 * 1000),
			"price in milligons"
		);

		index.argon_usd_price = 101;
		Current::<Test>::put(index);

		assert_eq!(
			<PriceIndex as BitcoinPriceProvider<u128>>::get_bitcoin_argon_price(
				SATOSHIS_PER_BITCOIN
			),
			Some(1000 * (62_000 * 100) / 101),
		);
	});
}

fn create_index() -> PriceIndexEntry<u64> {
	PriceIndexEntry {
		timestamp: now(),
		btc_usd_price: 62_000 * 100,
		argon_usd_price: 1_000 * 100,
		argon_cpi: 0,
	}
}

fn now() -> Moment {
	SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as Moment
}