use super::Config;
use argon_primitives::{
	treasury::{TreasuryPositionProvider, TreasuryPositionProviderWeightInfo},
	vault::{TreasuryVaultProvider, TreasuryVaultProviderWeightInfo},
	OperationalAccountProvider, OperationalAccountProviderWeightInfo, OperationalAccountsHook,
	OperationalAccountsHookWeightInfo, OperationalRewardsPayerWeightInfo,
	TreasuryPoolProviderWeightInfo,
};
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_frame_transition(argon_lots_scanned: u32, argonot_lots: u32, due_releases: u32)
		-> Weight;
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
	fn set_bond_lot_flexible() -> Weight;
	fn set_reserved_bond_space() -> Weight;
	fn configure_reward_economics() -> Weight;
	fn backfill_bond_lot_earnings() -> Weight;
	fn provider_has_vault_bond_participation() -> Weight;
	fn provider_active_vault_bond_amount() -> Weight;
	fn provider_active_account_vault_bond_amount() -> Weight;
	fn provider_encumber_bond_microgons() -> Weight;
	fn provider_release_encumbered_bond_microgons() -> Weight;
	fn provider_burn_encumbered_bond_microgons(lots: u32) -> Weight;
}

pub struct WithProviderWeights<T, Base>(core::marker::PhantomData<(T, Base)>);

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
	fn on_frame_transition(
		argon_lots_scanned: u32,
		argonot_lots: u32,
		due_releases: u32,
	) -> Weight {
		Base::on_frame_transition(argon_lots_scanned, argonot_lots, due_releases)
			.saturating_add(PositionWeights::<T>::network_totals().saturating_mul(2))
			.saturating_add(
				PositionWeights::<T>::upstream_position()
					.saturating_add(OperationalAccountWeights::<T>::upstream_vault())
					.saturating_add(VaultWeights::<T>::get_participation_capacity())
					.saturating_mul(T::MaxVaultsPerPool::get().into()),
			)
	}

	fn release_pending_bond_lots() -> Weight {
		Base::release_pending_bond_lots()
	}

	fn distribute_bid_pool() -> Weight {
		Base::distribute_bid_pool()
	}

	fn lock_in_vault_capital() -> Weight {
		Base::lock_in_vault_capital()
			.saturating_add(PositionWeights::<T>::network_totals())
			.saturating_add(
				PositionWeights::<T>::upstream_position()
					.saturating_add(OperationalAccountWeights::<T>::upstream_vault())
					.saturating_add(VaultWeights::<T>::get_participation_capacity())
					.saturating_mul(T::MaxVaultsPerPool::get().into()),
			)
	}

	fn upstream_participation() -> Weight {
		Base::upstream_participation()
			.saturating_add(PositionWeights::<T>::upstream_position())
			.saturating_add(OperationalAccountWeights::<T>::upstream_vault())
			.saturating_add(VaultWeights::<T>::get_participation_capacity())
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
			.saturating_add(PositionWeights::<T>::account_quantity_updated().saturating_mul(2))
			.saturating_add(VaultWeights::<T>::get_vault_operator())
			.saturating_add(PositionWeights::<T>::position_updated())
			.saturating_add(OperationalHookWeights::<T>::account_vault_bond_total_updated())
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
	}

	fn provider_release_encumbered_bond_microgons() -> Weight {
		Base::provider_release_encumbered_bond_microgons()
	}

	fn provider_burn_encumbered_bond_microgons(lots: u32) -> Weight {
		Base::provider_burn_encumbered_bond_microgons(lots)
			.saturating_add(VaultWeights::<T>::get_vault_operator().saturating_mul(lots.into()))
			.saturating_add(
				PositionWeights::<T>::account_quantity_updated()
					.saturating_mul(2 * u64::from(lots)),
			)
			.saturating_add(PositionWeights::<T>::bond_principal())
			.saturating_add(PositionWeights::<T>::position_updated().saturating_mul(lots.into()))
			.saturating_add(OperationalHookWeights::<T>::account_vault_bond_total_updated())
	}
}

pub struct ProviderWeightAdapter<T>(core::marker::PhantomData<T>);
impl<T: Config> TreasuryPoolProviderWeightInfo for ProviderWeightAdapter<T> {
	fn vault_securitization_changed() -> Weight {
		// Vault benchmarks measure Treasury storage inline. Positions and the Vault operator
		// query use benchmark providers, so their storage costs are composed separately.
		PositionWeights::<T>::account_quantity_updated()
			.saturating_add(VaultWeights::<T>::get_vault_operator())
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
		<T as Config>::WeightInfo::provider_burn_encumbered_bond_microgons(
			T::MaxArgonBondLots::get(),
		)
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_frame_transition(
		_argon_lots_scanned: u32,
		_argonot_lots: u32,
		_due_releases: u32,
	) -> Weight {
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
	fn set_bond_lot_flexible() -> Weight {
		Weight::zero()
	}
	fn set_reserved_bond_space() -> Weight {
		Weight::zero()
	}
	fn configure_reward_economics() -> Weight {
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

	fn provider_release_encumbered_bond_microgons() -> Weight {
		Weight::zero()
	}

	fn provider_burn_encumbered_bond_microgons(_: u32) -> Weight {
		Weight::zero()
	}
}

impl<T: Config> OperationalRewardsPayerWeightInfo for ProviderWeightAdapter<T> {
	fn claim_reward() -> Weight {
		T::WeightInfo::claim_reward()
	}
}
