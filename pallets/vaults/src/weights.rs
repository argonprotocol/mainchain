use pallet_prelude::*;

/// Weight functions needed for pallet_vaults.
pub trait WeightInfo {
	fn create() -> Weight;
	fn modify_funding() -> Weight;
	fn modify_terms() -> Weight;
	fn close() -> Weight;
	fn replace_bitcoin_xpub() -> Weight;
	fn on_initialize_with_vault_releases(height_range: u32, vault_count: u32) -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn create() -> Weight {
		Weight::zero()
	}
	fn modify_funding() -> Weight {
		Weight::zero()
	}
	fn modify_terms() -> Weight {
		Weight::zero()
	}
	fn close() -> Weight {
		Weight::zero()
	}
	fn replace_bitcoin_xpub() -> Weight {
		Weight::zero()
	}
	fn on_initialize_with_vault_releases(_height_range: u32, _vault_count: u32) -> Weight {
		Weight::zero()
	}
}
