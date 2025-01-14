use crate::{
	mock::{System, Ticks, *},
	pallet::RecentBlocksAtTicks,
};
use argon_primitives::{tick::MAX_BLOCKS_PER_TICK, NotebookAuditResult, TickProvider};
use frame_support::{pallet_prelude::*, traits::OnTimestampSet};
use std::panic::catch_unwind;

#[test]
fn it_panics_if_the_tick_is_invalid() {
	new_test_ext(500).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		Digests::mutate(|a| a.tick.0 = 2);
		System::initialize(&2, &System::parent_hash(), &Default::default());
		Ticks::on_initialize(2);
		let err = catch_unwind(|| {
			let now = 999;
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
		Digests::mutate(|a| a.tick.0 = 2);
		System::initialize(&2, &System::parent_hash(), &Default::default());
		Ticks::on_initialize(2);
		assert_eq!(Ticks::current_tick(), 2);

		Digests::mutate(|a| a.tick.0 = 3);
		let max_blocks = MAX_BLOCKS_PER_TICK as u64;
		for i in 0..=(max_blocks - 1) {
			let block = 3 + i;
			System::initialize(&block, &System::parent_hash(), &Default::default());
			Ticks::on_initialize(block);
		}
		let blocks_at_tick = RecentBlocksAtTicks::<Test>::get(3);
		assert_eq!(blocks_at_tick.len(), max_blocks as usize - 1);

		// should not allow a 6th block at tick 2
		let err = catch_unwind(|| {
			Ticks::on_initialize(9);
		});
		// should require notebooks
		assert!(err.is_err());

		Digests::mutate(|a| {
			a.notebooks.notebooks.push(NotebookAuditResult {
				tick: 3,
				notary_id: 1,
				notebook_number: 10,
				audit_first_failure: None,
			})
		});

		Ticks::on_initialize(9);
		let blocks_at_tick = RecentBlocksAtTicks::<Test>::get(3);
		assert_eq!(blocks_at_tick.len(), max_blocks as usize);

		// can't push more than 5 though
		let err = catch_unwind(|| {
			Ticks::on_initialize(10);
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
			Digests::mutate(|a| a.tick.0 = tick);
			System::initialize(&block_number, &parent_hash, &Default::default());
			Ticks::on_initialize(block_number);

			if block_number > 0 {
				// we have to run 1 block behind because the current block is not yet finalized
				assert!(Ticks::blocks_at_tick(last_block_tick).contains(&System::parent_hash()));
			}
			last_block_tick = tick;
			let header = System::finalize();
			parent_hash = header.hash();
		}
		assert_eq!(RecentBlocksAtTicks::<Test>::iter_keys().collect::<Vec<_>>().len(), 10);
		assert_eq!(Ticks::blocks_at_tick(0).len(), 1);
		assert!(Ticks::blocks_at_tick(501).contains(&System::parent_hash()));
	});
}

#[test]
#[should_panic]
fn it_should_track_multiple_blocks_at_tick_if_enabled() {
	new_test_ext(500).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		Digests::mutate(|a| a.tick.0 = 1);
		System::initialize(&1, &System::parent_hash(), &Default::default());
		Ticks::on_initialize(1);
		Digests::mutate(|a| a.tick.0 = 1);
		System::initialize(&2, &System::parent_hash(), &Default::default());
		Ticks::on_initialize(2);
		assert!(Ticks::blocks_at_tick(1).contains(&System::parent_hash()));
		System::initialize(&3, &System::parent_hash(), &Default::default());
		Ticks::on_initialize(3);
		assert_eq!(Ticks::blocks_at_tick(1).len(), 2);

		Digests::mutate(|a| a.tick.0 = 2);
		System::initialize(&4, &System::parent_hash(), &Default::default());
		Ticks::on_initialize(4);
		assert_eq!(Ticks::blocks_at_tick(2).len(), 1);
		System::initialize(&5, &System::parent_hash(), &Default::default());
		Ticks::on_initialize(5);
		assert_eq!(Ticks::blocks_at_tick(2).len(), 2);
	});
}
