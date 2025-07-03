use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_initialize(miner_count: u32, utxo_count: u32) -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_initialize(_miner_count: u32, _utxo_count: u32) -> Weight {
		Weight::zero()
	}
}
