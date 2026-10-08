use argon_primitives::{
	providers::{
		CollectBlockerProvider, OperationalAccountProvider, OperationalAccountProviderWeightInfo,
		OperationalAccountsHook, OperationalAccountsHookWeightInfo, TickProvider,
		TickProviderWeightInfo, TreasuryPoolProvider, TreasuryPoolProviderWeightInfo,
	},
	vault::{
		BitcoinVaultProviderWeightInfo, TreasuryVaultProviderWeightInfo,
		MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES,
	},
};
use core::marker::PhantomData;
use pallet_prelude::*;

/// Weight functions needed for pallet_vaults.
pub trait WeightInfo {
	fn provider_get_vault_operator() -> Weight {
		Weight::zero()
	}
	fn provider_get_participation_capacity() -> Weight {
		Weight::zero()
	}
	fn create() -> Weight;
	fn modify_funding() -> Weight;
	fn modify_terms() -> Weight;
	fn close() -> Weight;
	fn replace_bitcoin_xpub() -> Weight;
	fn set_delegate_account() -> Weight;
	fn set_reserved_securitization_space() -> Weight;
	fn set_argonot_securitization() -> Weight;
	fn on_initialize_with_vault_releases(
		height_range: u32,
		bitcoin_release_vault_count: u32,
	) -> Weight;
	fn collect() -> Weight;
	fn on_frame_start(vault_count: u32) -> Weight;
	fn provider_get_registration_vault_data() -> Weight;
	fn provider_get_committed_securitization() -> Weight;
	fn provider_get_held_argonots() -> Weight;
	fn provider_encumber_argonots() -> Weight;
	fn provider_release_encumbered_argonots() -> Weight;
	fn provider_burn_encumbered_argonots() -> Weight;
	fn provider_account_became_operational() -> Weight;
	fn provider_set_bitcoin_lock_flexible() -> Weight;
	fn provider_reserve_securitization(release_schedule_entries: u32) -> Weight;
	fn provider_resecuritize(release_schedule_entries: u32) -> Weight;
	fn provider_resecuritize_unfunded(release_schedule_entries: u32) -> Weight;
	fn provider_burn(release_schedule_entries: u32) -> Weight;
	fn provider_get_top_vaults_by_securitization(vaults: u32) -> Weight;
	fn provider_commit_securitization_for_rewards() -> Weight;
	fn provider_record_vault_frame_earnings() -> Weight;
}

type TickProviderWeights<T> = <<T as crate::Config>::TickProvider as TickProvider<
	<T as frame_system::Config>::Block,
>>::Weights;
type CollectBlockerProviderWeights<T> =
	<<T as crate::Config>::CollectBlockerProvider as CollectBlockerProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;
type OperationalAccountProviderWeights<T> =
	<<T as crate::Config>::OperationalAccountProvider as OperationalAccountProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;

type OperationalAccountsHookWeights<T> =
	<<T as crate::Config>::OperationalAccountsHook as OperationalAccountsHook<
		<T as frame_system::Config>::AccountId,
		<T as crate::Config>::Balance,
	>>::Weights;

type TreasuryPoolWeights<T> =
	<<T as crate::Config>::TreasuryPoolProvider as TreasuryPoolProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;

pub struct WithProviderWeights<
	T,
	Base,
	TickProviderWeight = TickProviderWeights<T>,
	CollectBlockerWeight = CollectBlockerProviderWeights<T>,
	OperationalAccountProviderWeight = OperationalAccountProviderWeights<T>,
>(
	PhantomData<(
		T,
		Base,
		TickProviderWeight,
		CollectBlockerWeight,
		OperationalAccountProviderWeight,
	)>,
);
impl<T, Base, TickProviderWeight, CollectBlockerWeight, OperationalAccountProviderWeight> WeightInfo
	for WithProviderWeights<
		T,
		Base,
		TickProviderWeight,
		CollectBlockerWeight,
		OperationalAccountProviderWeight,
	>
where
	T: crate::Config,
	Base: WeightInfo,
	TickProviderWeight: TickProviderWeightInfo,
	CollectBlockerWeight: argon_primitives::CollectBlockerProviderWeightInfo,
	OperationalAccountProviderWeight: OperationalAccountProviderWeightInfo,
{
	fn provider_get_participation_capacity() -> Weight {
		Base::provider_get_participation_capacity()
			.saturating_add(TickProviderWeight::current_tick())
	}

	fn provider_get_vault_operator() -> Weight {
		Base::provider_get_vault_operator()
	}

	fn create() -> Weight {
		Base::create()
			.saturating_add(TickProviderWeight::current_tick())
			.saturating_add(OperationalAccountProviderWeight::is_eligible())
	}

	fn modify_funding() -> Weight {
		Base::modify_funding()
			.saturating_add(TreasuryPoolWeights::<T>::vault_securitization_changed())
	}

	fn modify_terms() -> Weight {
		Base::modify_terms().saturating_add(TickProviderWeight::current_tick())
	}

	fn close() -> Weight {
		Base::close().saturating_add(TreasuryPoolWeights::<T>::vault_securitization_changed())
	}

	fn replace_bitcoin_xpub() -> Weight {
		Base::replace_bitcoin_xpub()
	}

	fn set_delegate_account() -> Weight {
		Base::set_delegate_account()
	}

	fn set_reserved_securitization_space() -> Weight {
		Base::set_reserved_securitization_space()
	}

	fn set_argonot_securitization() -> Weight {
		Base::set_argonot_securitization()
	}

	fn on_initialize_with_vault_releases(
		height_range: u32,
		bitcoin_release_vault_count: u32,
	) -> Weight {
		Base::on_initialize_with_vault_releases(height_range, bitcoin_release_vault_count)
			.saturating_add(
				TreasuryPoolWeights::<T>::vault_securitization_changed()
					.saturating_mul(bitcoin_release_vault_count.into()),
			)
	}

	fn collect() -> Weight {
		Base::collect().saturating_add(CollectBlockerWeight::has_overdue_collect_blocker())
	}

	fn on_frame_start(vault_count: u32) -> Weight {
		Base::on_frame_start(vault_count).saturating_add(TickProviderWeight::current_tick())
	}

	fn provider_get_registration_vault_data() -> Weight {
		Base::provider_get_registration_vault_data()
	}

	fn provider_get_committed_securitization() -> Weight {
		Base::provider_get_committed_securitization()
	}

	fn provider_get_held_argonots() -> Weight {
		Base::provider_get_held_argonots()
	}

	fn provider_encumber_argonots() -> Weight {
		Base::provider_encumber_argonots()
	}

	fn provider_release_encumbered_argonots() -> Weight {
		Base::provider_release_encumbered_argonots()
	}

	fn provider_burn_encumbered_argonots() -> Weight {
		Base::provider_burn_encumbered_argonots()
	}

	fn provider_account_became_operational() -> Weight {
		Base::provider_account_became_operational()
	}

	fn provider_set_bitcoin_lock_flexible() -> Weight {
		Base::provider_set_bitcoin_lock_flexible()
	}

	fn provider_resecuritize(release_schedule_entries: u32) -> Weight {
		Base::provider_resecuritize(release_schedule_entries)
			.saturating_add(TickProviderWeight::current_tick())
			.saturating_add(OperationalAccountsHookWeights::<T>::vault_bitcoin_lock_funded())
	}

	fn provider_reserve_securitization(release_schedule_entries: u32) -> Weight {
		Base::provider_reserve_securitization(release_schedule_entries)
			.saturating_add(TickProviderWeight::current_tick())
	}

	fn provider_resecuritize_unfunded(release_schedule_entries: u32) -> Weight {
		Base::provider_resecuritize_unfunded(release_schedule_entries)
			.saturating_add(TickProviderWeight::current_tick())
	}

	fn provider_burn(release_schedule_entries: u32) -> Weight {
		Base::provider_burn(release_schedule_entries)
			.saturating_add(TreasuryPoolWeights::<T>::vault_securitization_changed())
	}

	fn provider_get_top_vaults_by_securitization(vaults: u32) -> Weight {
		Base::provider_get_top_vaults_by_securitization(vaults)
	}

	fn provider_commit_securitization_for_rewards() -> Weight {
		Base::provider_commit_securitization_for_rewards()
	}

	fn provider_record_vault_frame_earnings() -> Weight {
		Base::provider_record_vault_frame_earnings()
	}
}

impl<T: crate::Config> TreasuryVaultProviderWeightInfo for ProviderWeightAdapter<T> {
	fn get_vault_operator() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_vault_operator()
	}
	fn get_participation_capacity() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_participation_capacity()
	}

	fn get_top_vaults_by_securitization(vaults: u32) -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_top_vaults_by_securitization(vaults)
	}

	fn commit_securitization_for_bonds() -> Weight {
		// This path scans the bounded daily schedule. Resecuritization covers the same
		// maximum schedule walk and additional storage writes.
		<T as crate::Config>::WeightInfo::provider_resecuritize(
			MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES,
		)
	}

	fn commit_securitization_for_rewards() -> Weight {
		<T as crate::Config>::WeightInfo::provider_commit_securitization_for_rewards()
	}

	fn record_vault_frame_earnings() -> Weight {
		<T as crate::Config>::WeightInfo::provider_record_vault_frame_earnings()
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> BitcoinVaultProviderWeightInfo for ProviderWeightAdapter<T> {
	fn get_registration_vault_data() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_registration_vault_data()
	}

	fn get_committed_securitization() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_committed_securitization()
	}

	fn get_held_argonots() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_held_argonots()
	}

	fn encumber_argonots() -> Weight {
		<T as crate::Config>::WeightInfo::provider_encumber_argonots()
	}

	fn release_encumbered_argonots() -> Weight {
		<T as crate::Config>::WeightInfo::provider_release_encumbered_argonots()
	}

	fn burn_encumbered_argonots() -> Weight {
		<T as crate::Config>::WeightInfo::provider_burn_encumbered_argonots()
	}

	fn account_became_operational() -> Weight {
		<T as crate::Config>::WeightInfo::provider_account_became_operational()
	}

	fn set_bitcoin_lock_flexible() -> Weight {
		<T as crate::Config>::WeightInfo::provider_set_bitcoin_lock_flexible()
	}

	fn reserve_securitization() -> Weight {
		<T as crate::Config>::WeightInfo::provider_reserve_securitization(
			MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES,
		)
	}

	fn resecuritize() -> Weight {
		<T as crate::Config>::WeightInfo::provider_resecuritize(
			MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES,
		)
		.max(<T as crate::Config>::WeightInfo::provider_resecuritize_unfunded(
			MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES,
		))
	}

	fn burn() -> Weight {
		<T as crate::Config>::WeightInfo::provider_burn(MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES)
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn create() -> Weight {
		Weight::zero()
	}
	fn modify_funding() -> Weight {
		Weight::zero()
	}
	fn modify_terms() -> Weight {
		Weight::zero()
	}
	fn close() -> Weight {
		Weight::zero()
	}
	fn replace_bitcoin_xpub() -> Weight {
		Weight::zero()
	}
	fn set_delegate_account() -> Weight {
		Weight::zero()
	}
	fn set_reserved_securitization_space() -> Weight {
		Weight::zero()
	}
	fn set_argonot_securitization() -> Weight {
		Weight::zero()
	}
	fn on_initialize_with_vault_releases(
		_height_range: u32,
		_bitcoin_release_vault_count: u32,
	) -> Weight {
		Weight::zero()
	}
	fn collect() -> Weight {
		Weight::zero()
	}
	fn on_frame_start(_vault_count: u32) -> Weight {
		Weight::zero()
	}
	fn provider_get_registration_vault_data() -> Weight {
		Weight::zero()
	}

	fn provider_get_committed_securitization() -> Weight {
		Weight::zero()
	}

	fn provider_get_held_argonots() -> Weight {
		Weight::zero()
	}

	fn provider_encumber_argonots() -> Weight {
		Weight::zero()
	}

	fn provider_release_encumbered_argonots() -> Weight {
		Weight::zero()
	}

	fn provider_burn_encumbered_argonots() -> Weight {
		Weight::zero()
	}

	fn provider_account_became_operational() -> Weight {
		Weight::zero()
	}
	fn provider_set_bitcoin_lock_flexible() -> Weight {
		Weight::zero()
	}
	fn provider_reserve_securitization(_release_schedule_entries: u32) -> Weight {
		Weight::zero()
	}
	fn provider_resecuritize(_release_schedule_entries: u32) -> Weight {
		Weight::zero()
	}
	fn provider_resecuritize_unfunded(_release_schedule_entries: u32) -> Weight {
		Weight::zero()
	}
	fn provider_burn(_release_schedule_entries: u32) -> Weight {
		Weight::zero()
	}
	fn provider_get_top_vaults_by_securitization(_vaults: u32) -> Weight {
		Weight::zero()
	}
	fn provider_commit_securitization_for_rewards() -> Weight {
		Weight::zero()
	}
	fn provider_record_vault_frame_earnings() -> Weight {
		Weight::zero()
	}
}
