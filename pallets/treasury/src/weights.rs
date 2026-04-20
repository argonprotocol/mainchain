use super::Config;
use argon_primitives::TreasuryPoolProviderWeightInfo;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_frame_transition() -> Weight;
	fn release_pending_bond_lots() -> Weight;
	fn distribute_bid_pool() -> Weight;
	fn lock_in_vault_capital() -> Weight;
	fn claim_reward() -> Weight;
	fn buy_bonds() -> Weight;
	fn liquidate_bond_lot() -> Weight;
	fn provider_has_bond_participation() -> Weight;
}

pub struct WithProviderWeights<T, Base>(core::marker::PhantomData<(T, Base)>);

impl<T, Base> WeightInfo for WithProviderWeights<T, Base>
where
	T: Config,
	Base: WeightInfo,
{
	fn on_frame_transition() -> Weight {
		Base::on_frame_transition()
	}

	fn release_pending_bond_lots() -> Weight {
		Base::release_pending_bond_lots()
	}

	fn distribute_bid_pool() -> Weight {
		Base::distribute_bid_pool()
	}

	fn lock_in_vault_capital() -> Weight {
		Base::lock_in_vault_capital()
	}

	fn claim_reward() -> Weight {
		Base::claim_reward()
	}

	fn buy_bonds() -> Weight {
		Base::buy_bonds()
	}

	fn liquidate_bond_lot() -> Weight {
		Base::liquidate_bond_lot()
	}

	fn provider_has_bond_participation() -> Weight {
		Base::provider_has_bond_participation()
	}
}

pub struct ProviderWeightAdapter<T>(core::marker::PhantomData<T>);
impl<T: Config> TreasuryPoolProviderWeightInfo for ProviderWeightAdapter<T> {
	fn has_bond_participation() -> Weight {
		<T as Config>::WeightInfo::provider_has_bond_participation()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_frame_transition() -> Weight {
		Weight::zero()
	}
	fn release_pending_bond_lots() -> Weight {
		Weight::zero()
	}
	fn distribute_bid_pool() -> Weight {
		Weight::zero()
	}
	fn lock_in_vault_capital() -> Weight {
		Weight::zero()
	}
	fn claim_reward() -> Weight {
		// Conservative placeholder until pallet_treasury runtime benchmarks are wired.
		Weight::from_parts(100_000_000, 0)
	}
	fn buy_bonds() -> Weight {
		Weight::zero()
	}
	fn liquidate_bond_lot() -> Weight {
		Weight::zero()
	}
	fn provider_has_bond_participation() -> Weight {
		Weight::zero()
	}
}
