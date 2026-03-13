use argon_primitives::providers::{BitcoinUtxoEvents, BitcoinUtxoEventsWeightInfo};
use pallet_prelude::*;

/// Weight functions needed for pallet_bitcoin_utxos.
pub trait WeightInfo {
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
	fn on_initialize(timed_out: u32) -> Weight {
		let mut weight = Self::on_initialize_base();
		if timed_out > 0 {
			weight = weight
				.saturating_add(Self::pending_funding_timeout().saturating_mul(timed_out.into()));
		}
		weight
	}
	fn on_initialize_base() -> Weight;
	fn sync_base() -> Weight;
	fn set_confirmed_block() -> Weight;
	fn set_operator() -> Weight;
	fn fund_with_utxo_candidate() -> Weight;
	fn reject_utxo_candidate() -> Weight;

	// Individual UTXO operation weights (linear benchmarks for sync composition)
	fn utxo_spent(n: u32) -> Weight;
	fn lock_verified(n: u32) -> Weight;
	fn pending_funding_timeout() -> Weight;
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

	fn fund_with_utxo_candidate() -> Weight {
		Base::fund_with_utxo_candidate()
			.saturating_add(EventHandlerWeight::funding_promoted_by_account())
			.saturating_add(
				EventHandlerWeight::orphaned_utxo_detected()
					.saturating_mul(T::MaxCandidateUtxosPerLock::get().saturating_sub(1).into()),
			)
	}

	fn reject_utxo_candidate() -> Weight {
		Base::reject_utxo_candidate()
			.saturating_add(EventHandlerWeight::candidate_rejected_by_account())
	}

	fn utxo_spent(n: u32) -> Weight {
		Base::utxo_spent(n).saturating_add(EventHandlerWeight::spent().saturating_mul(n.into()))
	}

	fn lock_verified(n: u32) -> Weight {
		let provider_weight = EventHandlerWeight::funding_received().saturating_add(
			EventHandlerWeight::orphaned_utxo_detected()
				.saturating_mul(T::MaxCandidateUtxosPerLock::get().into()),
		);
		Base::lock_verified(n).saturating_add(provider_weight.saturating_mul(n.into()))
	}

	fn pending_funding_timeout() -> Weight {
		let provider_weight = EventHandlerWeight::timeout_waiting_for_funding().saturating_add(
			EventHandlerWeight::orphaned_utxo_detected()
				.saturating_mul(T::MaxCandidateUtxosPerLock::get().into()),
		);
		Base::pending_funding_timeout().saturating_add(provider_weight)
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
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

	fn fund_with_utxo_candidate() -> Weight {
		Weight::zero()
	}

	fn reject_utxo_candidate() -> Weight {
		Weight::zero()
	}

	fn utxo_spent(_n: u32) -> Weight {
		Weight::zero()
	}

	fn lock_verified(_n: u32) -> Weight {
		Weight::zero()
	}

	fn pending_funding_timeout() -> Weight {
		Weight::zero()
	}
}
