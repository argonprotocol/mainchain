use crate::Config;
use argon_primitives::{
	vault::{BitcoinVaultProvider, BitcoinVaultProviderWeightInfo},
	BitcoinLocksProvider, BitcoinLocksProviderWeightInfo, MiningSlotProvider,
	MiningSlotProviderWeightInfo, OperationalAccountProviderWeightInfo, TreasuryPoolProvider,
	TreasuryPoolProviderWeightInfo, UniswapTransferProvider, UniswapTransferProviderWeightInfo,
	UtxoLockEventsWeightInfo,
};
use core::marker::PhantomData;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn register() -> Weight;
	fn upgrade_account() -> Weight;
	fn set_reward_config() -> Weight;
	fn force_set_progress() -> Weight;
	fn set_encrypted_server_for_downstream_account() -> Weight;
	fn activate() -> Weight;
	fn claim_rewards() -> Weight;
	fn on_vault_created() -> Weight;
	fn on_vault_bitcoin_lock_funded() -> Weight;
	fn on_mining_seat_won() -> Weight;
	fn on_account_bitcoin_amount_updated() -> Weight;
	fn on_account_vault_bond_total_updated() -> Weight;
	fn on_account_uniswap_argon_transfers_in_updated() -> Weight;
}

type VaultProviderWeights<T> = <<T as Config>::VaultProvider as BitcoinVaultProvider>::Weights;
type MiningSlotProviderWeights<T> = <<T as Config>::MiningSlotProvider as MiningSlotProvider<
	<T as frame_system::Config>::AccountId,
>>::Weights;
type UniswapTransferProviderWeights<T> =
	<<T as Config>::UniswapTransferProvider as UniswapTransferProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;
type TreasuryPoolProviderWeights<T> =
	<<T as Config>::TreasuryPoolProvider as TreasuryPoolProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;
type BitcoinLocksProviderWeights<T> =
	<<T as Config>::BitcoinLocksProvider as BitcoinLocksProvider<
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
	>>::Weights;

pub struct WithProviderWeights<
	T,
	Base,
	VaultProviderWeight = VaultProviderWeights<T>,
	MiningSlotProviderWeight = MiningSlotProviderWeights<T>,
	UniswapTransferWeight = UniswapTransferProviderWeights<T>,
	TreasuryPoolProviderWeight = TreasuryPoolProviderWeights<T>,
	BitcoinLocksProviderWeight = BitcoinLocksProviderWeights<T>,
>(
	PhantomData<(
		T,
		Base,
		VaultProviderWeight,
		MiningSlotProviderWeight,
		UniswapTransferWeight,
		TreasuryPoolProviderWeight,
		BitcoinLocksProviderWeight,
	)>,
);
impl<
		T,
		Base,
		VaultProviderWeight,
		MiningSlotProviderWeight,
		UniswapTransferWeight,
		TreasuryPoolProviderWeight,
		BitcoinLocksProviderWeight,
	> WeightInfo
	for WithProviderWeights<
		T,
		Base,
		VaultProviderWeight,
		MiningSlotProviderWeight,
		UniswapTransferWeight,
		TreasuryPoolProviderWeight,
		BitcoinLocksProviderWeight,
	>
where
	T: Config,
	Base: WeightInfo,
	VaultProviderWeight: BitcoinVaultProviderWeightInfo,
	MiningSlotProviderWeight: MiningSlotProviderWeightInfo,
	UniswapTransferWeight: UniswapTransferProviderWeightInfo,
	TreasuryPoolProviderWeight: TreasuryPoolProviderWeightInfo,
	BitcoinLocksProviderWeight: BitcoinLocksProviderWeightInfo,
{
	fn register() -> Weight {
		Base::register()
			.saturating_add(VaultProviderWeight::get_registration_vault_data())
			.saturating_add(BitcoinLocksProviderWeight::get_account_funded_bitcoin_amount())
			.saturating_add(MiningSlotProviderWeight::has_active_rewards_account_seat())
			.saturating_add(TreasuryPoolProviderWeight::active_account_vault_bond_amount())
			.saturating_add(UniswapTransferWeight::is_crosschain_activated())
			.saturating_add(
				UniswapTransferWeight::account_uniswap_argon_transfers_in_amount()
					.saturating_mul(3),
			)
	}

	fn upgrade_account() -> Weight {
		Base::upgrade_account()
	}

	fn set_reward_config() -> Weight {
		Base::set_reward_config()
	}

	fn force_set_progress() -> Weight {
		Base::force_set_progress()
	}

	fn set_encrypted_server_for_downstream_account() -> Weight {
		Base::set_encrypted_server_for_downstream_account()
	}

	fn activate() -> Weight {
		Base::activate()
			.saturating_add(VaultProviderWeight::get_registration_vault_data())
			.saturating_add(VaultProviderWeight::account_became_operational())
	}

	fn claim_rewards() -> Weight {
		Base::claim_rewards().saturating_add(
			<<T as Config>::OperationalRewardsPayer as argon_primitives::OperationalRewardsPayer<
				<T as frame_system::Config>::AccountId,
				<T as Config>::Balance,
			>>::claim_reward_weight(),
		)
	}

	fn on_vault_created() -> Weight {
		Base::on_vault_created()
	}

	fn on_vault_bitcoin_lock_funded() -> Weight {
		Base::on_vault_bitcoin_lock_funded()
	}

	fn on_mining_seat_won() -> Weight {
		Base::on_mining_seat_won()
	}

	fn on_account_bitcoin_amount_updated() -> Weight {
		Base::on_account_bitcoin_amount_updated()
	}

	fn on_account_vault_bond_total_updated() -> Weight {
		Base::on_account_vault_bond_total_updated()
	}

	fn on_account_uniswap_argon_transfers_in_updated() -> Weight {
		Base::on_account_uniswap_argon_transfers_in_updated().saturating_add(
			UniswapTransferWeight::account_uniswap_argon_transfers_in_amount().saturating_mul(3),
		)
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: Config> UtxoLockEventsWeightInfo for ProviderWeightAdapter<T> {
	fn utxo_locked() -> Weight {
		T::WeightInfo::on_account_bitcoin_amount_updated()
	}

	fn utxo_released() -> Weight {
		T::WeightInfo::on_account_bitcoin_amount_updated()
	}

	fn utxo_released_with_pending_mints() -> Weight {
		T::WeightInfo::on_account_bitcoin_amount_updated()
	}
}

impl<T: Config> OperationalAccountProviderWeightInfo for ProviderWeightAdapter<T> {
	fn is_eligible() -> Weight {
		T::DbWeight::get().reads(3)
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn register() -> Weight {
		Weight::zero()
	}
	fn upgrade_account() -> Weight {
		Weight::zero()
	}
	fn set_reward_config() -> Weight {
		Weight::zero()
	}
	fn force_set_progress() -> Weight {
		Weight::zero()
	}
	fn set_encrypted_server_for_downstream_account() -> Weight {
		Weight::zero()
	}
	fn activate() -> Weight {
		Weight::zero()
	}
	fn claim_rewards() -> Weight {
		Weight::zero()
	}
	fn on_vault_created() -> Weight {
		Weight::zero()
	}
	fn on_vault_bitcoin_lock_funded() -> Weight {
		Weight::zero()
	}
	fn on_mining_seat_won() -> Weight {
		Weight::zero()
	}
	fn on_account_bitcoin_amount_updated() -> Weight {
		Weight::zero()
	}
	fn on_account_vault_bond_total_updated() -> Weight {
		Weight::zero()
	}
	fn on_account_uniswap_argon_transfers_in_updated() -> Weight {
		Weight::zero()
	}
}
