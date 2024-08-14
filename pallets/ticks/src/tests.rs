use argon_primitives::{digests::TICK_DIGEST_ID, tick::Ticker, TickDigest, TickProvider};
use frame_support::{pallet_prelude::*, traits::OnTimestampSet};
use sp_runtime::{Digest, DigestItem};
use std::panic::catch_unwind;

use crate::{
	mock::{System, Ticks, *},
	pallet::RecentBlocksAtTicks,
};

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
			&Digest {
				logs: vec![DigestItem::PreRuntime(
					TICK_DIGEST_ID,
					TickDigest { tick: 2u32 }.encode(),
				)],
			},
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
			&Digest {
				logs: vec![DigestItem::PreRuntime(
					TICK_DIGEST_ID,
					TickDigest { tick: 2u32 }.encode(),
				)],
			},
		);
		Ticks::on_initialize(2);
		Ticks::on_timestamp_set(2500);
		assert_eq!(Ticks::current_tick(), 2);
	});
}

#[test]
fn it_should_track_blocks_at_tick() {
	new_test_ext(5, 500).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		let ticker = Ticker::new(5, 500, 2);
		let mut parent_hash = System::parent_hash();
		let mut last_block_tick = 0;
		for i in 0..=501 {
			let block_number = (i + 2u32).into();
			let timestamp = 500u64 + i as u64;
			let tick = ticker.tick_for_time(timestamp);
			System::initialize(
				&block_number,
				&parent_hash,
				&Digest {
					logs: vec![DigestItem::PreRuntime(
						TICK_DIGEST_ID,
						TickDigest { tick }.encode(),
					)],
				},
			);
			Ticks::on_initialize(block_number);

			if block_number > 0 {
				// we have to run 1 block behind because the current block is not yet finalized
				assert!(Ticks::blocks_at_tick(last_block_tick).contains(&System::parent_hash()));
			}
			last_block_tick = tick;
			let header = System::finalize();
			parent_hash = header.hash();
		}
		assert_eq!(<RecentBlocksAtTicks<Test>>::get(0).len(), 0);
		assert_eq!(<RecentBlocksAtTicks<Test>>::get(100).len(), 1);
	});
}
