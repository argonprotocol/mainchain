use crate::Config;
use argon_primitives::{
	treasury::{
		BitcoinLockPositionProvider, BitcoinLockPositionProviderWeightInfo,
		EarningsSettlementProvider, EarningsSettlementProviderWeightInfo,
		TreasuryPositionProviderWeightInfo, VaultEarningsProvider, VaultEarningsProviderWeightInfo,
	},
	OperationalAccountProvider, OperationalAccountProviderWeightInfo, TreasuryPoolProvider,
	TreasuryPoolProviderWeightInfo,
};
use core::marker::PhantomData;
use pallet_prelude::*;

pub trait WeightInfo {
	fn allocate_mining_mint() -> Weight {
		Weight::zero()
	}
	fn mining_seats_changed() -> Weight {
		Weight::zero()
	}
	fn record_funding() -> Weight;
	fn expire_vault_earnings() -> Weight;
	fn allocate_frame_earnings() -> Weight;
	fn prepare_collect() -> Weight;
	fn settle_earning_changes(changes: u32) -> Weight;
	fn indexed_frame() -> Weight;
	fn ensure_account() -> Weight;
	fn collect() -> Weight;
	fn fund() -> Weight;
	fn accrue() -> Weight;
	fn bond_principal() -> Weight;
	fn account_quantities() -> Weight;
	fn network_totals() -> Weight;
	fn account_quantity_updated() -> Weight;
	fn upstream_position() -> Weight;
	fn set_upstream_position() -> Weight;
	fn bitcoin_position_updated() -> Weight;
	fn bond_position_updated() -> Weight;
}
impl WeightInfo for () {
	fn record_funding() -> Weight {
		Weight::zero()
	}
	fn expire_vault_earnings() -> Weight {
		Weight::zero()
	}
	fn allocate_frame_earnings() -> Weight {
		Weight::zero()
	}
	fn prepare_collect() -> Weight {
		Weight::zero()
	}
	fn settle_earning_changes(_: u32) -> Weight {
		Weight::zero()
	}
	fn indexed_frame() -> Weight {
		Weight::zero()
	}
	fn fund() -> Weight {
		Weight::zero()
	}
	fn accrue() -> Weight {
		Weight::zero()
	}
	fn ensure_account() -> Weight {
		Weight::zero()
	}
	fn collect() -> Weight {
		Weight::zero()
	}
	fn bond_principal() -> Weight {
		Weight::zero()
	}
	fn account_quantities() -> Weight {
		Weight::zero()
	}
	fn network_totals() -> Weight {
		Weight::zero()
	}
	fn account_quantity_updated() -> Weight {
		Weight::zero()
	}
	fn upstream_position() -> Weight {
		Weight::zero()
	}
	fn set_upstream_position() -> Weight {
		Weight::zero()
	}
	fn bitcoin_position_updated() -> Weight {
		Weight::zero()
	}
	fn bond_position_updated() -> Weight {
		Weight::zero()
	}
}

type VaultEarningsWeights<T> = <<T as Config>::VaultEarningsProvider as VaultEarningsProvider<
	<T as frame_system::Config>::AccountId,
	<T as Config>::Balance,
>>::Weights;

pub fn collect<T: Config>() -> Weight {
	T::WeightInfo::collect()
		.saturating_add(VaultEarningsWeights::<T>::collect_earnings())
		.saturating_add(T::WeightInfo::settle_earning_changes(T::MaxEarningChangesPerCall::get()))
		.saturating_add(<T::EarningsSettlementProvider as EarningsSettlementProvider<
			T::AccountId,
		>>::Weights::settle_account())
}
pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: Config> TreasuryPositionProviderWeightInfo for ProviderWeightAdapter<T> {
	fn allocate_mining_mint() -> Weight {
		T::WeightInfo::allocate_mining_mint()
	}
	fn mining_seats_changed() -> Weight {
		T::WeightInfo::mining_seats_changed().saturating_add(T::WeightInfo::ensure_account())
	}
	fn record_funding() -> Weight {
		T::WeightInfo::record_funding()
	}
	fn expire_vault_earnings() -> Weight {
		T::WeightInfo::expire_vault_earnings()
	}
	fn prepare_collect() -> Weight {
		T::WeightInfo::prepare_collect()
	}
	fn allocate_frame_earnings() -> Weight {
		T::WeightInfo::allocate_frame_earnings()
	}
	fn indexed_frame() -> Weight {
		T::WeightInfo::indexed_frame()
	}

	fn fund() -> Weight {
		T::WeightInfo::fund()
	}
	fn accrue() -> Weight {
		T::WeightInfo::accrue()
	}
	fn ensure_account() -> Weight {
		T::WeightInfo::ensure_account()
	}
	fn operational_account_registered() -> Weight {
		Self::set_upstream_position()
			.saturating_add(<T::OperationalAccountProvider as OperationalAccountProvider<T::AccountId>>::Weights::upstream_vault())
			.saturating_add(<T::BitcoinPositionProvider as BitcoinLockPositionProvider<T::AccountId, T::Balance>>::Weights::account_position())
			.saturating_add(<T::TreasuryPoolProvider as TreasuryPoolProvider<T::AccountId>>::Weights::active_vault_bond_amount())
	}
	fn bond_principal() -> Weight {
		T::WeightInfo::bond_principal()
	}
	fn account_quantities() -> Weight {
		T::WeightInfo::account_quantities()
	}
	fn network_totals() -> Weight {
		T::WeightInfo::network_totals()
	}
	fn account_quantity_updated() -> Weight {
		T::WeightInfo::account_quantity_updated()
			.saturating_add(T::WeightInfo::ensure_account())
			.saturating_add(T::WeightInfo::settle_earning_changes(1))
	}
	fn network_securitized_satoshis_updated() -> Weight {
		// An account update also writes NetworkTotals, so its measured cost is conservative here.
		Self::account_quantity_updated()
	}
	fn upstream_position() -> Weight {
		T::WeightInfo::upstream_position()
	}
	fn set_upstream_position() -> Weight {
		T::WeightInfo::set_upstream_position().saturating_add(T::WeightInfo::ensure_account())
	}
	fn position_updated() -> Weight {
		T::WeightInfo::bitcoin_position_updated()
			.max(T::WeightInfo::bond_position_updated())
			// A first source update also retains a system provider reference.
			.saturating_add(T::WeightInfo::ensure_account())
			.saturating_add(<T::OperationalAccountProvider as OperationalAccountProvider<
				T::AccountId,
			>>::Weights::upstream_vault())
	}
}
