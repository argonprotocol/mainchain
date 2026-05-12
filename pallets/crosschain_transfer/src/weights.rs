use argon_primitives::{
	CurrentTransactionFeeProvider, CurrentTransactionFeeProviderWeightInfo, EthereumVerifyProvider,
	EthereumVerifyProviderWeightInfo, UniswapTransferProviderWeightInfo,
};
use core::marker::PhantomData;
use pallet_prelude::*;

pub trait WeightInfo {
	fn set_chain_config() -> Weight;
	fn prove_transfer() -> Weight;
	fn on_initialize_cleanup(expiring: u32) -> Weight;
	fn provider_is_crosschain_activated() -> Weight;
	fn provider_has_recent_argon_transfer() -> Weight;
}

type EthereumVerifyProviderWeights<T> =
	<<T as crate::Config>::EthereumVerifier as EthereumVerifyProvider>::Weights;
type CurrentTransactionFeeProviderWeights<T> =
	<<T as crate::Config>::CurrentTransactionFeeProvider as CurrentTransactionFeeProvider<
		<T as crate::Config>::Balance,
	>>::Weights;

pub struct WithProviderWeights<
	T,
	Base,
	EthereumVerifyWeight = EthereumVerifyProviderWeights<T>,
	CurrentFeeWeight = CurrentTransactionFeeProviderWeights<T>,
>(PhantomData<(T, Base, EthereumVerifyWeight, CurrentFeeWeight)>);
impl<T, Base, EthereumVerifyWeight, CurrentFeeWeight> WeightInfo
	for WithProviderWeights<T, Base, EthereumVerifyWeight, CurrentFeeWeight>
where
	T: crate::Config,
	Base: WeightInfo,
	EthereumVerifyWeight: EthereumVerifyProviderWeightInfo,
	CurrentFeeWeight: CurrentTransactionFeeProviderWeightInfo,
{
	fn set_chain_config() -> Weight {
		Base::set_chain_config()
	}

	fn prove_transfer() -> Weight {
		Base::prove_transfer()
			.saturating_add(EthereumVerifyWeight::verify_event_log())
			.saturating_add(CurrentFeeWeight::current_transaction_fee())
	}

	fn on_initialize_cleanup(expiring: u32) -> Weight {
		Base::on_initialize_cleanup(expiring)
	}

	fn provider_is_crosschain_activated() -> Weight {
		Base::provider_is_crosschain_activated()
	}

	fn provider_has_recent_argon_transfer() -> Weight {
		Base::provider_has_recent_argon_transfer()
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> UniswapTransferProviderWeightInfo for ProviderWeightAdapter<T> {
	fn is_crosschain_activated() -> Weight {
		T::WeightInfo::provider_is_crosschain_activated()
	}

	fn has_recent_argon_transfer() -> Weight {
		T::WeightInfo::provider_has_recent_argon_transfer()
	}
}

impl WeightInfo for () {
	fn set_chain_config() -> Weight {
		Weight::zero()
	}

	fn prove_transfer() -> Weight {
		Weight::zero()
	}

	fn on_initialize_cleanup(_expiring: u32) -> Weight {
		Weight::zero()
	}

	fn provider_is_crosschain_activated() -> Weight {
		Weight::zero()
	}

	fn provider_has_recent_argon_transfer() -> Weight {
		Weight::zero()
	}
}
