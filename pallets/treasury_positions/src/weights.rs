use crate::Config;
use argon_primitives::{
	treasury::{
		BitcoinLockPositionProvider, BitcoinLockPositionProviderWeightInfo,
		TreasuryPositionProviderWeightInfo,
	},
	OperationalAccountProvider, OperationalAccountProviderWeightInfo, TreasuryPoolProvider,
	TreasuryPoolProviderWeightInfo,
};
use core::marker::PhantomData;
use pallet_prelude::*;
pub trait WeightInfo {
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
pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: Config> TreasuryPositionProviderWeightInfo for ProviderWeightAdapter<T> {
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
	}
	fn upstream_position() -> Weight {
		T::WeightInfo::upstream_position()
	}
	fn set_upstream_position() -> Weight {
		T::WeightInfo::set_upstream_position()
	}
	fn position_updated() -> Weight {
		T::WeightInfo::bitcoin_position_updated()
			.max(T::WeightInfo::bond_position_updated())
			.saturating_add(<T::OperationalAccountProvider as OperationalAccountProvider<
				T::AccountId,
			>>::Weights::upstream_vault())
	}
}
