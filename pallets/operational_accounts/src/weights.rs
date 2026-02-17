use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn register() -> Weight;
	fn issue_access_code() -> Weight;
	fn set_reward_config() -> Weight;
	fn on_vault_created() -> Weight;
	fn on_bitcoin_lock_funded() -> Weight;
	fn on_mining_seat_won() -> Weight;
	fn on_treasury_pool_participated() -> Weight;
	fn on_uniswap_transfer() -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn register() -> Weight {
		Weight::zero()
	}
	fn issue_access_code() -> Weight {
		Weight::zero()
	}
	fn set_reward_config() -> Weight {
		Weight::zero()
	}
	fn on_vault_created() -> Weight {
		Weight::zero()
	}
	fn on_bitcoin_lock_funded() -> Weight {
		Weight::zero()
	}
	fn on_mining_seat_won() -> Weight {
		Weight::zero()
	}
	fn on_treasury_pool_participated() -> Weight {
		Weight::zero()
	}
	fn on_uniswap_transfer() -> Weight {
		Weight::zero()
	}
}
