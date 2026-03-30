use argon_primitives::{
	MiningSlotProvider, MiningSlotProviderWeightInfo, TreasuryPoolProvider,
	TreasuryPoolProviderWeightInfo,
	providers::OperationalRewardsProviderWeightInfo,
	vault::{BitcoinVaultProvider, BitcoinVaultProviderWeightInfo},
};
use core::marker::PhantomData;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn register() -> Weight;
	fn issue_access_code() -> Weight;
	fn set_reward_config() -> Weight;
	fn force_set_progress() -> Weight;
	fn set_encrypted_server_for_sponsee() -> Weight;
	fn on_vault_created() -> Weight;
	fn on_bitcoin_lock_funded() -> Weight;
	fn on_mining_seat_won() -> Weight;
	fn on_treasury_pool_participated() -> Weight;
	fn on_uniswap_transfer() -> Weight;
	fn provider_pending_rewards() -> Weight;
	fn provider_mark_reward_paid() -> Weight;
}

type VaultProviderWeights<T> =
	<<T as crate::Config>::VaultProvider as BitcoinVaultProvider>::Weights;
type MiningSlotProviderWeights<T> =
	<<T as crate::Config>::MiningSlotProvider as MiningSlotProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;
type TreasuryPoolProviderWeights<T> =
	<<T as crate::Config>::TreasuryPoolProvider as TreasuryPoolProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;

pub struct WithProviderWeights<
	T,
	Base,
	VaultProviderWeight = VaultProviderWeights<T>,
	MiningSlotProviderWeight = MiningSlotProviderWeights<T>,
	TreasuryPoolProviderWeight = TreasuryPoolProviderWeights<T>,
>(
	PhantomData<(
		T,
		Base,
		VaultProviderWeight,
		MiningSlotProviderWeight,
		TreasuryPoolProviderWeight,
	)>,
);
impl<T, Base, VaultProviderWeight, MiningSlotProviderWeight, TreasuryPoolProviderWeight> WeightInfo
	for WithProviderWeights<
		T,
		Base,
		VaultProviderWeight,
		MiningSlotProviderWeight,
		TreasuryPoolProviderWeight,
	>
where
	T: crate::Config,
	Base: WeightInfo,
	VaultProviderWeight: BitcoinVaultProviderWeightInfo,
	MiningSlotProviderWeight: MiningSlotProviderWeightInfo,
	TreasuryPoolProviderWeight: TreasuryPoolProviderWeightInfo,
{
	fn register() -> Weight {
		Base::register()
			.saturating_add(VaultProviderWeight::get_registration_vault_data())
			.saturating_add(MiningSlotProviderWeight::has_active_rewards_account_seat())
			.saturating_add(TreasuryPoolProviderWeight::has_pool_participation())
	}

	fn issue_access_code() -> Weight {
		Base::issue_access_code()
	}

	fn set_reward_config() -> Weight {
		Base::set_reward_config()
	}

	fn force_set_progress() -> Weight {
		Base::force_set_progress()
	}

	fn set_encrypted_server_for_sponsee() -> Weight {
		Base::set_encrypted_server_for_sponsee()
	}

	fn on_vault_created() -> Weight {
		Base::on_vault_created()
	}

	fn on_bitcoin_lock_funded() -> Weight {
		Base::on_bitcoin_lock_funded()
	}

	fn on_mining_seat_won() -> Weight {
		Base::on_mining_seat_won()
	}

	fn on_treasury_pool_participated() -> Weight {
		Base::on_treasury_pool_participated()
	}

	fn on_uniswap_transfer() -> Weight {
		Base::on_uniswap_transfer()
	}

	fn provider_pending_rewards() -> Weight {
		Base::provider_pending_rewards()
	}

	fn provider_mark_reward_paid() -> Weight {
		Base::provider_mark_reward_paid()
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);

impl<T: crate::Config> OperationalRewardsProviderWeightInfo for ProviderWeightAdapter<T> {
	fn pending_rewards() -> Weight {
		<T as crate::Config>::WeightInfo::provider_pending_rewards()
	}

	fn mark_reward_paid() -> Weight {
		<T as crate::Config>::WeightInfo::provider_mark_reward_paid()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn register() -> Weight {
		Weight::zero()
	}
	fn issue_access_code() -> Weight {
		Weight::zero()
	}
	fn set_reward_config() -> Weight {
		Weight::zero()
	}
	fn force_set_progress() -> Weight {
		Weight::zero()
	}
	fn set_encrypted_server_for_sponsee() -> Weight {
		Weight::zero()
	}
	fn on_vault_created() -> Weight {
		Weight::zero()
	}
	fn on_bitcoin_lock_funded() -> Weight {
		Weight::zero()
	}
	fn on_mining_seat_won() -> Weight {
		Weight::zero()
	}
	fn on_treasury_pool_participated() -> Weight {
		Weight::zero()
	}
	fn on_uniswap_transfer() -> Weight {
		Weight::zero()
	}

	fn provider_pending_rewards() -> Weight {
		Weight::zero()
	}

	fn provider_mark_reward_paid() -> Weight {
		Weight::zero()
	}
}
