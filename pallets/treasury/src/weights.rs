use super::Config;
use argon_primitives::{
	treasury::{
		StabilizerPoolRewardProviderWeightInfo, TreasuryPositionProvider,
		TreasuryPositionProviderWeightInfo, TreasuryRewardProviderWeightInfo,
	},
	vault::{TreasuryVaultProvider, TreasuryVaultProviderWeightInfo},
	OperationalAccountProvider, OperationalAccountProviderWeightInfo, OperationalAccountsHook,
	OperationalAccountsHookWeightInfo, OperationalRewardsPayerWeightInfo,
	TreasuryPoolProviderWeightInfo,
};
use core::marker::PhantomData;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn provider_vault_securitization_changed() -> Weight;
	fn finalize_stabilizer_frame_rewards() -> Weight;
	fn read_stabilizer_frame_reward_rate() -> Weight;
	fn claim_stabilizer_frame_rewards(frames: u32) -> Weight;
	fn read_bond_collateral_queue() -> Weight;
	fn on_frame_transition(due_releases: u32) -> Weight;
	fn release_pending_bond_lots() -> Weight;
	fn distribute_bid_pool() -> Weight;
	fn lock_in_vault_capital() -> Weight;
	fn upstream_participation() -> Weight {
		Weight::zero()
	}
	fn claim_reward() -> Weight;
	fn buy_bonds() -> Weight;
	fn buy_argonot_bonds() -> Weight;
	fn liquidate_bond_lot() -> Weight;
	fn liquidate_vault_bond_lot() -> Weight;
	fn set_bond_lot_flexible() -> Weight;
	fn set_reserved_bond_space() -> Weight;
	fn configure_reward_economics() -> Weight;
	fn configure_reward_tapering_policy() -> Weight;
	fn backfill_bond_lot_earnings() -> Weight;
	fn provider_has_vault_bond_participation() -> Weight;
	fn provider_active_vault_bond_amount() -> Weight;
	fn provider_active_account_vault_bond_amount() -> Weight;
	fn provider_encumber_bond_microgons() -> Weight;
	fn provider_encumber_selected_bond_microgons() -> Weight;
	fn provider_release_encumbered_bond_microgons() -> Weight;
	fn provider_burn_encumbered_bond_microgons() -> Weight;
	fn settle_bond_collateral_release(lots: u32) -> Weight;
	fn settle_bond_collateral_burn(lots: u32) -> Weight;
	fn provider_available_reward_budget() -> Weight;
	fn provider_reserve_reward_budget() -> Weight;
	fn provider_return_reward_budget() -> Weight;
}

pub struct WithProviderWeights<T, Base>(PhantomData<(T, Base)>);

type PositionWeights<T> = <<T as Config>::PositionProvider as TreasuryPositionProvider<
	<T as frame_system::Config>::AccountId,
	<T as Config>::Balance,
>>::Weights;
type OperationalAccountWeights<T> =
	<<T as Config>::OperationalAccountProvider as OperationalAccountProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;
type VaultWeights<T> = <<T as Config>::TreasuryVaultProvider as TreasuryVaultProvider>::Weights;
type OperationalHookWeights<T> =
	<<T as Config>::OperationalAccountsHook as OperationalAccountsHook<
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
	>>::Weights;

impl<T, Base> WeightInfo for WithProviderWeights<T, Base>
where
	T: Config,
	Base: WeightInfo,
{
	fn provider_vault_securitization_changed() -> Weight {
		Base::provider_vault_securitization_changed()
			.saturating_add(PositionWeights::<T>::account_quantity_updated())
			.saturating_add(VaultWeights::<T>::get_vault_operator())
	}
	fn finalize_stabilizer_frame_rewards() -> Weight {
		Base::finalize_stabilizer_frame_rewards()
	}
	fn read_stabilizer_frame_reward_rate() -> Weight {
		Base::read_stabilizer_frame_reward_rate()
	}
	fn claim_stabilizer_frame_rewards(frames: u32) -> Weight {
		Base::claim_stabilizer_frame_rewards(frames).saturating_add(PositionWeights::<T>::fund())
	}

	fn read_bond_collateral_queue() -> Weight {
		Base::read_bond_collateral_queue()
	}

	fn on_frame_transition(due_releases: u32) -> Weight {
		Base::on_frame_transition(due_releases)
			.saturating_add(PositionWeights::<T>::prepare_collect())
			.saturating_add(PositionWeights::<T>::allocate_frame_earnings())
			.saturating_add(PositionWeights::<T>::indexed_frame().saturating_mul(2))
			.saturating_add(PositionWeights::<T>::network_totals().saturating_mul(2))
			.saturating_add(
				PositionWeights::<T>::upstream_position()
					.saturating_add(OperationalAccountWeights::<T>::upstream_vault())
					.saturating_mul(T::MaxVaultsPerPool::get().into()),
			)
	}

	fn release_pending_bond_lots() -> Weight {
		Base::release_pending_bond_lots()
	}

	fn distribute_bid_pool() -> Weight {
		Base::distribute_bid_pool()
			.saturating_add(PositionWeights::<T>::indexed_frame().saturating_mul(2))
	}

	fn lock_in_vault_capital() -> Weight {
		Base::lock_in_vault_capital()
			.saturating_add(PositionWeights::<T>::network_totals())
			.saturating_add(
				PositionWeights::<T>::upstream_position()
					.saturating_add(OperationalAccountWeights::<T>::upstream_vault())
					.saturating_mul(T::MaxVaultsPerPool::get().into()),
			)
	}

	fn upstream_participation() -> Weight {
		Base::upstream_participation()
			.saturating_add(PositionWeights::<T>::upstream_position())
			.saturating_add(OperationalAccountWeights::<T>::upstream_vault())
	}

	fn claim_reward() -> Weight {
		Base::claim_reward()
	}

	fn buy_bonds() -> Weight {
		Base::buy_bonds()
			.saturating_add(PositionWeights::<T>::account_quantity_updated().saturating_mul(2))
			.saturating_add(VaultWeights::<T>::get_vault_operator())
			.saturating_add(PositionWeights::<T>::bond_principal())
			.saturating_add(PositionWeights::<T>::position_updated())
			.saturating_add(
				OperationalHookWeights::<T>::account_vault_bond_total_updated().saturating_mul(2),
			)
	}

	fn buy_argonot_bonds() -> Weight {
		Base::buy_argonot_bonds()
			.saturating_add(PositionWeights::<T>::network_totals())
			.saturating_add(PositionWeights::<T>::account_quantity_updated().saturating_mul(2))
	}

	fn liquidate_bond_lot() -> Weight {
		Base::liquidate_bond_lot()
			.max(Base::liquidate_vault_bond_lot())
			.saturating_add(PositionWeights::<T>::account_quantity_updated().saturating_mul(2))
			.saturating_add(VaultWeights::<T>::get_vault_operator())
			.saturating_add(PositionWeights::<T>::bond_principal())
			.saturating_add(PositionWeights::<T>::position_updated())
			.saturating_add(OperationalHookWeights::<T>::account_vault_bond_total_updated())
	}

	fn liquidate_vault_bond_lot() -> Weight {
		Self::liquidate_bond_lot()
	}

	fn set_bond_lot_flexible() -> Weight {
		Base::set_bond_lot_flexible()
			.saturating_add(PositionWeights::<T>::account_quantity_updated().saturating_mul(2))
			.saturating_add(VaultWeights::<T>::get_vault_operator())
	}

	fn set_reserved_bond_space() -> Weight {
		Base::set_reserved_bond_space()
	}

	fn configure_reward_economics() -> Weight {
		Base::configure_reward_economics()
	}

	fn configure_reward_tapering_policy() -> Weight {
		Base::configure_reward_tapering_policy()
	}

	fn backfill_bond_lot_earnings() -> Weight {
		Base::backfill_bond_lot_earnings()
	}

	fn provider_has_vault_bond_participation() -> Weight {
		Base::provider_has_vault_bond_participation()
	}

	fn provider_active_vault_bond_amount() -> Weight {
		Base::provider_active_vault_bond_amount()
	}

	fn provider_active_account_vault_bond_amount() -> Weight {
		Base::provider_active_account_vault_bond_amount()
			.saturating_add(PositionWeights::<T>::bond_principal())
	}

	fn provider_encumber_bond_microgons() -> Weight {
		Base::provider_encumber_bond_microgons()
			.max(Base::provider_encumber_selected_bond_microgons())
	}

	fn provider_encumber_selected_bond_microgons() -> Weight {
		Self::provider_encumber_bond_microgons()
	}

	fn provider_release_encumbered_bond_microgons() -> Weight {
		Base::provider_release_encumbered_bond_microgons()
	}

	fn settle_bond_collateral_release(lots: u32) -> Weight {
		Base::settle_bond_collateral_release(lots)
	}

	fn provider_burn_encumbered_bond_microgons() -> Weight {
		Base::provider_burn_encumbered_bond_microgons()
	}

	fn settle_bond_collateral_burn(lots: u32) -> Weight {
		// A regular lot updates its holder and may restore a displaced flexible operator.
		// Repeated holder calls can settle different batches of retained index changes.
		let per_lot = VaultWeights::<T>::get_vault_operator()
			.saturating_add(PositionWeights::<T>::account_quantity_updated().saturating_mul(2))
			.saturating_add(PositionWeights::<T>::position_updated());
		Base::settle_bond_collateral_burn(lots)
			.saturating_add(per_lot.saturating_mul(lots.into()))
			.saturating_add(PositionWeights::<T>::bond_principal())
			.saturating_add(OperationalHookWeights::<T>::account_vault_bond_total_updated())
	}
	fn provider_available_reward_budget() -> Weight {
		Base::provider_available_reward_budget()
	}

	fn provider_reserve_reward_budget() -> Weight {
		Base::provider_reserve_reward_budget()
	}

	fn provider_return_reward_budget() -> Weight {
		Base::provider_return_reward_budget()
	}
}

pub struct ProviderWeightAdapter<T>(core::marker::PhantomData<T>);
impl<T: Config> TreasuryPoolProviderWeightInfo for ProviderWeightAdapter<T> {
	fn vault_securitization_changed() -> Weight {
		T::WeightInfo::provider_vault_securitization_changed()
	}

	fn has_vault_bond_participation() -> Weight {
		<T as Config>::WeightInfo::provider_has_vault_bond_participation()
	}

	fn active_vault_bond_amount() -> Weight {
		<T as Config>::WeightInfo::provider_active_vault_bond_amount()
	}

	fn active_account_vault_bond_amount() -> Weight {
		<T as Config>::WeightInfo::provider_active_account_vault_bond_amount()
	}

	fn encumber_bond_microgons() -> Weight {
		<T as Config>::WeightInfo::provider_encumber_bond_microgons()
	}

	fn release_encumbered_bond_microgons() -> Weight {
		<T as Config>::WeightInfo::provider_release_encumbered_bond_microgons()
	}

	fn burn_encumbered_bond_microgons() -> Weight {
		<T as Config>::WeightInfo::provider_burn_encumbered_bond_microgons()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn provider_vault_securitization_changed() -> Weight {
		Weight::zero()
	}
	fn finalize_stabilizer_frame_rewards() -> Weight {
		Weight::zero()
	}
	fn read_stabilizer_frame_reward_rate() -> Weight {
		Weight::zero()
	}
	fn claim_stabilizer_frame_rewards(_: u32) -> Weight {
		Weight::zero()
	}
	fn read_bond_collateral_queue() -> Weight {
		Weight::zero()
	}
	fn on_frame_transition(_due_releases: u32) -> Weight {
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
	fn buy_argonot_bonds() -> Weight {
		Weight::zero()
	}
	fn liquidate_bond_lot() -> Weight {
		Weight::zero()
	}
	fn liquidate_vault_bond_lot() -> Weight {
		Weight::zero()
	}

	fn set_bond_lot_flexible() -> Weight {
		Weight::zero()
	}
	fn set_reserved_bond_space() -> Weight {
		Weight::zero()
	}
	fn configure_reward_economics() -> Weight {
		Weight::zero()
	}
	fn configure_reward_tapering_policy() -> Weight {
		Weight::zero()
	}

	fn backfill_bond_lot_earnings() -> Weight {
		Weight::zero()
	}
	fn provider_has_vault_bond_participation() -> Weight {
		Weight::zero()
	}

	fn provider_active_vault_bond_amount() -> Weight {
		Weight::zero()
	}

	fn provider_active_account_vault_bond_amount() -> Weight {
		Weight::zero()
	}

	fn provider_encumber_bond_microgons() -> Weight {
		Weight::zero()
	}

	fn provider_encumber_selected_bond_microgons() -> Weight {
		Weight::zero()
	}

	fn provider_release_encumbered_bond_microgons() -> Weight {
		Weight::zero()
	}

	fn settle_bond_collateral_release(_: u32) -> Weight {
		Weight::zero()
	}
	fn settle_bond_collateral_burn(_: u32) -> Weight {
		Weight::zero()
	}
	fn provider_burn_encumbered_bond_microgons() -> Weight {
		Weight::zero()
	}
	fn provider_available_reward_budget() -> Weight {
		Weight::zero()
	}

	fn provider_reserve_reward_budget() -> Weight {
		Weight::zero()
	}

	fn provider_return_reward_budget() -> Weight {
		Weight::zero()
	}
}

impl<T: Config> OperationalRewardsPayerWeightInfo for ProviderWeightAdapter<T> {
	fn claim_reward() -> Weight {
		T::WeightInfo::claim_reward()
	}
}

impl<T: Config> TreasuryRewardProviderWeightInfo for ProviderWeightAdapter<T> {
	fn available_budget() -> Weight {
		T::WeightInfo::provider_available_reward_budget()
	}
	fn reserve_budget() -> Weight {
		T::WeightInfo::provider_reserve_reward_budget()
	}
	fn return_budget() -> Weight {
		T::WeightInfo::provider_return_reward_budget()
	}
}

impl<T: Config> StabilizerPoolRewardProviderWeightInfo for ProviderWeightAdapter<T> {
	fn finalize_frame_rewards() -> Weight {
		T::WeightInfo::finalize_stabilizer_frame_rewards()
	}
	fn frame_reward_rate() -> Weight {
		T::WeightInfo::read_stabilizer_frame_reward_rate()
	}
	fn claim_rewards(frame_count: u32) -> Weight {
		T::WeightInfo::claim_stabilizer_frame_rewards(frame_count)
	}
}
