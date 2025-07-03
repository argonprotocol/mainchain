use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn set_block_rewards_paused() -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn set_block_rewards_paused() -> Weight {
		Weight::zero()
	}
}
