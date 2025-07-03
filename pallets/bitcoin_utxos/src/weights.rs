use pallet_prelude::*;

/// Weight functions needed for pallet_bitcoin_utxos.
pub trait WeightInfo {
	// Actual extrinsics
	fn sync(spent: u32, verified: u32, invalid: u32) -> Weight {
		Weight::zero()
			.saturating_add(Self::utxo_spent(spent))
			.saturating_add(Self::utxo_verified(verified))
			.saturating_add(Self::utxo_rejected_invalid(invalid))
	}
	fn set_confirmed_block() -> Weight;
	fn set_operator() -> Weight;

	// Individual UTXO operation weights (linear benchmarks for sync composition)
	fn utxo_spent(n: u32) -> Weight;
	fn utxo_verified(n: u32) -> Weight;
	fn utxo_rejected_invalid(n: u32) -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn sync(spent: u32, verified: u32, invalid: u32) -> Weight {
		Weight::zero()
			.saturating_add(Self::utxo_spent(spent))
			.saturating_add(Self::utxo_verified(verified))
			.saturating_add(Self::utxo_rejected_invalid(invalid))
	}

	fn set_confirmed_block() -> Weight {
		Weight::zero()
	}

	fn set_operator() -> Weight {
		Weight::zero()
	}

	fn utxo_spent(_n: u32) -> Weight {
		Weight::zero()
	}

	fn utxo_verified(_n: u32) -> Weight {
		Weight::zero()
	}

	fn utxo_rejected_invalid(_n: u32) -> Weight {
		Weight::zero()
	}
}
