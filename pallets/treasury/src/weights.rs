use super::Config;
use argon_primitives::{
	OperationalRewardsProvider, TreasuryPoolProviderWeightInfo,
	providers::OperationalRewardsProviderWeightInfo,
};
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_frame_transition() -> Weight;
	fn release_pending_unlocks() -> Weight;
	fn distribute_bid_pool() -> Weight;
	fn lock_in_vault_capital() -> Weight;
	fn pay_operational_rewards() -> Weight;
	fn try_pay_reward() -> Weight;
	fn set_allocation() -> Weight;
	fn provider_has_pool_participation() -> Weight;
}

type OperationalRewardsProviderWeights<T> =
	<<T as Config>::OperationalRewardsProvider as OperationalRewardsProvider<
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
	>>::Weights;

pub struct WithProviderWeights<
	T,
	Base,
	OperationalRewardsProviderWeight = OperationalRewardsProviderWeights<T>,
>(core::marker::PhantomData<(T, Base, OperationalRewardsProviderWeight)>);

impl<T, Base, OperationalRewardsProviderWeight> WeightInfo
	for WithProviderWeights<T, Base, OperationalRewardsProviderWeight>
where
	T: Config,
	Base: WeightInfo,
	OperationalRewardsProviderWeight: OperationalRewardsProviderWeightInfo,
{
	fn on_frame_transition() -> Weight {
		Base::on_frame_transition()
	}

	fn release_pending_unlocks() -> Weight {
		Base::release_pending_unlocks()
	}

	fn distribute_bid_pool() -> Weight {
		Base::distribute_bid_pool()
	}

	fn lock_in_vault_capital() -> Weight {
		Base::lock_in_vault_capital()
	}

	fn pay_operational_rewards() -> Weight {
		Base::pay_operational_rewards().saturating_add(
			OperationalRewardsProviderWeight::mark_reward_paid()
				.saturating_mul(u64::from(T::OperationalRewardsProvider::max_pending_rewards())),
		)
	}

	fn try_pay_reward() -> Weight {
		Base::try_pay_reward()
	}

	fn set_allocation() -> Weight {
		Base::set_allocation()
	}

	fn provider_has_pool_participation() -> Weight {
		Base::provider_has_pool_participation()
	}
}

pub struct ProviderWeightAdapter<T>(core::marker::PhantomData<T>);
impl<T: Config> TreasuryPoolProviderWeightInfo for ProviderWeightAdapter<T> {
	fn has_pool_participation() -> Weight {
		<T as Config>::WeightInfo::provider_has_pool_participation()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_frame_transition() -> Weight {
		Weight::zero()
	}
	fn release_pending_unlocks() -> Weight {
		Weight::zero()
	}
	fn distribute_bid_pool() -> Weight {
		Weight::zero()
	}
	fn lock_in_vault_capital() -> Weight {
		Weight::zero()
	}
	fn pay_operational_rewards() -> Weight {
		Weight::zero()
	}
	fn try_pay_reward() -> Weight {
		// Conservative placeholder until pallet_treasury runtime benchmarks are wired.
		Weight::from_parts(100_000_000, 0)
	}
	fn set_allocation() -> Weight {
		Weight::zero()
	}
	fn provider_has_pool_participation() -> Weight {
		Weight::zero()
	}
}
