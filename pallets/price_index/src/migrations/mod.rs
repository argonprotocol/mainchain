use crate::{Config, Current, LastValid, Pallet, PriceIndex};
use argon_primitives::price_index::PriceIndexV1;
use pallet_prelude::*;

/// Extend the open v2 index layout with optional USDC and stabilizer-pool observations.
pub fn add_optional_pool_prices<T: Config>() -> Weight {
	if StorageVersion::get::<Pallet<T>>() != StorageVersion::new(1) {
		return T::DbWeight::get().reads(1);
	}
	Current::<T>::translate::<PriceIndexV1, _>(|old| old.map(PriceIndex::from))
		.expect("version 1 price index must decode with its stored layout");
	LastValid::<T>::translate::<PriceIndexV1, _>(|old| old.map(PriceIndex::from))
		.expect("version 1 price index must decode with its stored layout");
	StorageVersion::new(2).put::<Pallet<T>>();
	T::DbWeight::get().reads_writes(3, 3)
}
