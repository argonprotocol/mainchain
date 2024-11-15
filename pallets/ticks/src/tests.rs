use crate::{
	mock::{System, Ticks, *},
	pallet::RecentBlocksAtTicks,
};
use argon_primitives::{digests::TICK_DIGEST_ID, tick::TickDigest, TickProvider};
use frame_support::{pallet_prelude::*, traits::OnTimestampSet};
use sp_runtime::{Digest, DigestItem};
use std::panic::catch_unwind;

#[test]
#[should_panic]
fn it_panics_if_no_tick_digest() {
	new_test_ext(500).execute_with(|| {
		System::on_initialize(2);
		Ticks::on_timestamp_set(1000);
		Ticks::on_initialize(2);
	});
}
#[test]
fn it_panics_if_the_tick_is_invalid() {
	new_test_ext(500).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		System::initialize(
			&2,
			&System::parent_hash(),
			&Digest { logs: vec![DigestItem::PreRuntime(TICK_DIGEST_ID, TickDigest(2).encode())] },
		);
		Ticks::on_initialize(2);
		let err = catch_unwind(|| {
			let now = 1000;
			assert_eq!(Ticks::ticker().tick_for_time(now), 1);
			// now tick is 1, so once timestamp comes in, it should panic (less than proposed 2)
			Ticks::on_timestamp_set(now);
		});
		assert!(err.is_err());
	});
}

#[test]
fn it_panics_if_two_blocks_use_the_same_tick() {
	new_test_ext(500).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		System::initialize(
			&2,
			&System::parent_hash(),
			&Digest { logs: vec![DigestItem::PreRuntime(TICK_DIGEST_ID, TickDigest(2).encode())] },
		);
		Ticks::on_initialize(2);
		let err = catch_unwind(|| {
			let now = 1000;
			assert_eq!(Ticks::ticker().tick_for_time(now), 1);
			// now tick is 1, so once timestamp comes in, it should panic (less than proposed 2)
			Ticks::on_timestamp_set(now);
		});
		assert!(err.is_err());
	});
}

#[test]
fn it_tests_the_current_tick() {
	new_test_ext(500).execute_with(|| {
		System::initialize(
			&2,
			&System::parent_hash(),
			&Digest { logs: vec![DigestItem::PreRuntime(TICK_DIGEST_ID, TickDigest(2).encode())] },
		);
		Ticks::on_initialize(2);
		assert_eq!(Ticks::current_tick(), 2);

		System::initialize(
			&3,
			&System::parent_hash(),
			&Digest { logs: vec![DigestItem::PreRuntime(TICK_DIGEST_ID, TickDigest(2).encode())] },
		);

		// should not allow a second block at tick 2
		let err = catch_unwind(|| {
			Ticks::on_initialize(3);
		});
		assert!(err.is_err());
	});
}

#[test]
fn it_should_track_blocks_at_tick() {
	new_test_ext(500).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		let mut parent_hash = System::parent_hash();
		let mut last_block_tick = 0;
		for i in 0..=501u64 {
			let block_number = i + 2;
			let tick = i + 1;
			System::initialize(
				&block_number,
				&parent_hash,
				&Digest {
					logs: vec![DigestItem::PreRuntime(TICK_DIGEST_ID, TickDigest(tick).encode())],
				},
			);
			Ticks::on_initialize(block_number);

			if block_number > 0 {
				// we have to run 1 block behind because the current block is not yet finalized
				assert_eq!(Ticks::block_at_tick(last_block_tick), Some(System::parent_hash()));
			}
			last_block_tick = tick;
			let header = System::finalize();
			parent_hash = header.hash();
		}
		assert_eq!(RecentBlocksAtTicks::<Test>::get().len(), 10);
		assert_eq!(Ticks::block_at_tick(0), None);
		assert_eq!(Ticks::block_at_tick(501), Some(System::parent_hash()));
	});
}
