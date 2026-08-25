use super::Config;
use argon_primitives::BitcoinFissionMintingWeightInfo;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_initialize(utxo_count: u32) -> Weight;
	fn provider_mint_requested() -> Weight;
	fn provider_mint_repaid() -> Weight;
}

pub struct FissionMintingWeightAdapter<T>(PhantomData<T>);
impl<T: Config> BitcoinFissionMintingWeightInfo for FissionMintingWeightAdapter<T> {
	fn request_mint() -> Weight {
		<T as Config>::WeightInfo::provider_mint_requested()
	}

	fn record_mint_repayment() -> Weight {
		<T as Config>::WeightInfo::provider_mint_repaid()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_initialize(_utxo_count: u32) -> Weight {
		Weight::zero()
	}

	fn provider_mint_requested() -> Weight {
		Weight::zero()
	}

	fn provider_mint_repaid() -> Weight {
		Weight::zero()
	}
}
