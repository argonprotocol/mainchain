use pallet_prelude::Weight;

/// Benchmark-derived fee-control extension costs.
pub trait WeightInfo {
	fn regular() -> Weight;
	fn batch(c: u32) -> Weight;
	fn proxy(calls: u32) -> Weight;
	fn proxy_announced(bytes: u32) -> Weight;
	fn post_event() -> Weight;
	fn pool_key_insertion(keys: u32) -> Weight;
	fn inspect_refund_calls(c: u32) -> Weight;
}

impl WeightInfo for () {
	fn regular() -> Weight {
		Weight::zero()
	}

	fn batch(_c: u32) -> Weight {
		Weight::zero()
	}

	fn proxy(_calls: u32) -> Weight {
		Weight::zero()
	}

	fn proxy_announced(_bytes: u32) -> Weight {
		Weight::zero()
	}

	fn post_event() -> Weight {
		Weight::zero()
	}

	fn pool_key_insertion(_keys: u32) -> Weight {
		Weight::zero()
	}

	fn inspect_refund_calls(_c: u32) -> Weight {
		Weight::zero()
	}
}
