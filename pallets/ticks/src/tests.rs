use frame_support::{pallet_prelude::*, traits::OnTimestampSet};
use sp_runtime::{Digest, DigestItem};
use std::panic::catch_unwind;
use ulx_primitives::digests::TICK_DIGEST_ID;

use crate::mock::{System, Ticks, *};

#[test]
#[should_panic]
fn it_panics_if_no_tick_digest() {
	new_test_ext(1000, 500).execute_with(|| {
		System::on_initialize(2);
		Ticks::on_timestamp_set(1000);
		Ticks::on_initialize(2);
	});
}
#[test]
fn it_panics_if_the_tick_is_invalid() {
	new_test_ext(1000, 500).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		System::initialize(
			&2,
			&System::parent_hash(),
			&Digest { logs: vec![DigestItem::PreRuntime(TICK_DIGEST_ID, 2u32.encode())] },
		);
		Ticks::on_initialize(2);
		let err = catch_unwind(|| {
			Ticks::on_timestamp_set(1000);
		});
		assert!(err.is_err());
	});
}
#[test]
fn it_tests_the_current_tick() {
	new_test_ext(1000, 500).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		System::initialize(
			&2,
			&System::parent_hash(),
			&Digest { logs: vec![DigestItem::PreRuntime(TICK_DIGEST_ID, 2u32.encode())] },
		);
		Ticks::on_initialize(2);
		Ticks::on_timestamp_set(2500);
		assert_eq!(Ticks::current_tick(), 2);
	});
}
