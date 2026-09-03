use argon_primitives::providers::{
	BitcoinUtxoEvents, BitcoinUtxoEventsWeightInfo, BitcoinUtxoTrackerWeightInfo,
	FeelessCallWeightInfo,
};
use pallet_prelude::*;

/// Weight functions needed for pallet_bitcoin_utxos.
pub trait WeightInfo {
	fn provider_is_feeless() -> Weight;
	fn provider_watch_for_utxo() -> Weight;
	fn provider_unwatch_utxo() -> Weight;
	fn provider_unwatch() -> Weight;
	// Actual extrinsics
	fn sync(spent: u32, verified: u32) -> Weight {
		let mut weight = Self::sync_base();
		if spent > 0 {
			weight = weight.saturating_add(Self::utxo_spent(spent));
		}
		if verified > 0 {
			weight = weight.saturating_add(Self::lock_verified(verified));
		}
		weight
	}
	fn on_initialize() -> Weight {
		Self::on_initialize_base()
	}
	fn on_initialize_base() -> Weight;
	fn sync_base() -> Weight;
	fn set_confirmed_block() -> Weight;
	fn set_operator() -> Weight;

	// Individual UTXO operation weights (linear benchmarks for sync composition)
	fn utxo_spent(n: u32) -> Weight;
	fn lock_verified(n: u32) -> Weight;
}

type EventHandlerWeights<T> = <<T as crate::pallet::Config>::EventHandler as BitcoinUtxoEvents<
	<T as frame_system::Config>::AccountId,
>>::Weights;

pub struct WithProviderWeights<T, Base, EventHandlerWeight = EventHandlerWeights<T>>(
	PhantomData<(T, Base, EventHandlerWeight)>,
);
impl<T, Base, EventHandlerWeight> WeightInfo for WithProviderWeights<T, Base, EventHandlerWeight>
where
	T: crate::pallet::Config,
	Base: WeightInfo,
	EventHandlerWeight: BitcoinUtxoEventsWeightInfo,
{
	fn provider_is_feeless() -> Weight {
		Base::provider_is_feeless()
	}
	fn provider_watch_for_utxo() -> Weight {
		Base::provider_watch_for_utxo()
	}
	fn provider_unwatch_utxo() -> Weight {
		Base::provider_unwatch_utxo()
	}
	fn provider_unwatch() -> Weight {
		Base::provider_unwatch()
	}
	fn on_initialize_base() -> Weight {
		Base::on_initialize_base()
	}

	fn sync_base() -> Weight {
		Base::sync_base()
	}

	fn set_confirmed_block() -> Weight {
		Base::set_confirmed_block()
	}

	fn set_operator() -> Weight {
		Base::set_operator()
	}

	fn utxo_spent(n: u32) -> Weight {
		Base::utxo_spent(n).saturating_add(EventHandlerWeight::spent().saturating_mul(n.into()))
	}

	fn lock_verified(n: u32) -> Weight {
		Base::lock_verified(n)
			.saturating_add(EventHandlerWeight::utxo_detected().saturating_mul(n.into()))
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> BitcoinUtxoTrackerWeightInfo for ProviderWeightAdapter<T> {
	fn watch_for_utxo() -> Weight {
		T::WeightInfo::provider_watch_for_utxo()
	}

	fn unwatch_utxo() -> Weight {
		T::WeightInfo::provider_unwatch_utxo()
	}

	fn unwatch() -> Weight {
		T::WeightInfo::provider_unwatch()
	}
}

impl<T: crate::Config> FeelessCallWeightInfo<T::RuntimeCall> for ProviderWeightAdapter<T>
where
	T::RuntimeCall: frame_support::traits::IsSubType<crate::Call<T>>,
{
	fn is_feeless(call: &T::RuntimeCall) -> Weight {
		if matches!(
			<T::RuntimeCall as frame_support::traits::IsSubType<crate::Call<T>>>::is_sub_type(call),
			Some(crate::Call::set_confirmed_block { .. })
		) {
			return T::WeightInfo::provider_is_feeless();
		}
		Weight::zero()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn provider_watch_for_utxo() -> Weight {
		Weight::zero()
	}
	fn provider_unwatch_utxo() -> Weight {
		Weight::zero()
	}
	fn provider_unwatch() -> Weight {
		Weight::zero()
	}
	fn provider_is_feeless() -> Weight {
		Weight::zero()
	}
	fn on_initialize_base() -> Weight {
		Weight::zero()
	}

	fn sync_base() -> Weight {
		Weight::zero()
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

	fn lock_verified(_n: u32) -> Weight {
		Weight::zero()
	}
}
