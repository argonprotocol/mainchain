use super::*;

frame_benchmarking::define_benchmarks!(
	[frame_benchmarking, BaselineBench::<Runtime>]
	[frame_system, SystemBench::<Runtime>]
	[pallet_balances, Balances]
	[pallet_balances, Ownership]
	[pallet_timestamp, Timestamp]
	[pallet_mining_slot, MiningSlot]
);
