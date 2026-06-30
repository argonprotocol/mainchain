use argon_primitives::{
	vault::{BitcoinVaultProvider, BitcoinVaultProviderWeightInfo},
	CollectBlockerProviderWeightInfo, EthereumVerifyProvider, EthereumVerifyProviderWeightInfo,
	PriceProvider, PriceProviderWeightInfo, TickProvider, TickProviderWeightInfo,
	TreasuryPoolProvider, TreasuryPoolProviderWeightInfo, UniswapTransferProviderWeightInfo,
};
use core::marker::PhantomData;
use pallet_prelude::*;

use super::Config;

pub trait WeightInfo {
	fn set_chain_config() -> Weight;
	fn force_set_global_issuance_council() -> Weight;
	fn register_council_signer() -> Weight;
	fn pause_gateway() -> Weight;
	fn unpause_gateway() -> Weight;
	fn register_minting_authority() -> Weight;
	fn deactivate_minting_authority() -> Weight;
	fn approve_queue_entries(approvals: u32) -> Weight;
	fn prove_gateway_activity(activities: u32) -> Weight;
	fn transfer_out() -> Weight;
	fn collateralize_transfer() -> Weight;
	fn provider_is_crosschain_activated() -> Weight;
	fn provider_account_uniswap_argon_transfers_in_amount() -> Weight;
	fn provider_has_overdue_collect_blocker() -> Weight;
}

type EthereumVerifyProviderWeights<T> =
	<<T as Config>::EthereumVerifier as EthereumVerifyProvider>::Weights;
type TreasuryPoolProviderWeights<T> =
	<<T as Config>::TreasuryPoolProvider as TreasuryPoolProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;
type TickProviderWeights<T> =
	<<T as Config>::TickProvider as TickProvider<<T as frame_system::Config>::Block>>::Weights;
type VaultProviderWeights<T> = <<T as Config>::VaultProvider as BitcoinVaultProvider>::Weights;
type PriceProviderWeights<T> =
	<<T as Config>::PriceProvider as PriceProvider<<T as Config>::Balance>>::Weights;

pub struct WithProviderWeights<
	T,
	Base,
	EthereumVerifyWeight = EthereumVerifyProviderWeights<T>,
	TreasuryPoolWeight = TreasuryPoolProviderWeights<T>,
	TickProviderWeight = TickProviderWeights<T>,
	VaultProviderWeight = VaultProviderWeights<T>,
	PriceProviderWeight = PriceProviderWeights<T>,
>(
	PhantomData<(
		T,
		Base,
		EthereumVerifyWeight,
		TreasuryPoolWeight,
		TickProviderWeight,
		VaultProviderWeight,
		PriceProviderWeight,
	)>,
);
impl<
		T,
		Base,
		EthereumVerifyWeight,
		TreasuryPoolWeight,
		TickProviderWeight,
		VaultProviderWeight,
		PriceProviderWeight,
	> WeightInfo
	for WithProviderWeights<
		T,
		Base,
		EthereumVerifyWeight,
		TreasuryPoolWeight,
		TickProviderWeight,
		VaultProviderWeight,
		PriceProviderWeight,
	>
where
	T: Config,
	Base: WeightInfo,
	EthereumVerifyWeight: EthereumVerifyProviderWeightInfo,
	TreasuryPoolWeight: TreasuryPoolProviderWeightInfo,
	TickProviderWeight: TickProviderWeightInfo,
	VaultProviderWeight: BitcoinVaultProviderWeightInfo,
	PriceProviderWeight: PriceProviderWeightInfo,
{
	fn set_chain_config() -> Weight {
		Base::set_chain_config()
	}

	fn force_set_global_issuance_council() -> Weight {
		Base::force_set_global_issuance_council()
			.saturating_add(PriceProviderWeight::get_lowest_microgons_per_argonot())
	}

	fn register_council_signer() -> Weight {
		Base::register_council_signer()
	}

	fn pause_gateway() -> Weight {
		Base::pause_gateway()
	}

	fn unpause_gateway() -> Weight {
		Base::unpause_gateway()
	}

	fn register_minting_authority() -> Weight {
		Base::register_minting_authority()
			.saturating_add(PriceProviderWeight::get_lowest_microgons_per_argonot())
			.saturating_add(TreasuryPoolWeight::encumber_bond_microgons())
			.saturating_add(VaultProviderWeight::encumber_argonots())
	}

	fn deactivate_minting_authority() -> Weight {
		Base::deactivate_minting_authority()
	}

	fn approve_queue_entries(approvals: u32) -> Weight {
		Base::approve_queue_entries(approvals.max(1))
	}

	fn prove_gateway_activity(activities: u32) -> Weight {
		Base::prove_gateway_activity(activities)
	}

	fn transfer_out() -> Weight {
		Base::transfer_out()
			.saturating_add(EthereumVerifyWeight::latest_execution_block_number())
			.saturating_add(EthereumVerifyWeight::latest_execution_block_timestamp())
			.saturating_add(TickProviderWeight::ticker())
	}

	fn collateralize_transfer() -> Weight {
		Base::collateralize_transfer()
	}

	fn provider_is_crosschain_activated() -> Weight {
		Base::provider_is_crosschain_activated()
	}

	fn provider_account_uniswap_argon_transfers_in_amount() -> Weight {
		Base::provider_account_uniswap_argon_transfers_in_amount()
	}

	fn provider_has_overdue_collect_blocker() -> Weight {
		Base::provider_has_overdue_collect_blocker()
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: Config> UniswapTransferProviderWeightInfo for ProviderWeightAdapter<T> {
	fn is_crosschain_activated() -> Weight {
		T::WeightInfo::provider_is_crosschain_activated()
	}

	fn account_uniswap_argon_transfers_in_amount() -> Weight {
		T::WeightInfo::provider_account_uniswap_argon_transfers_in_amount()
	}
}

impl<T: Config> CollectBlockerProviderWeightInfo for ProviderWeightAdapter<T> {
	fn has_overdue_collect_blocker() -> Weight {
		T::WeightInfo::provider_has_overdue_collect_blocker()
	}
}

impl WeightInfo for () {
	fn set_chain_config() -> Weight {
		Weight::zero()
	}

	fn force_set_global_issuance_council() -> Weight {
		Weight::zero()
	}

	fn register_council_signer() -> Weight {
		Weight::zero()
	}

	fn pause_gateway() -> Weight {
		Weight::zero()
	}

	fn unpause_gateway() -> Weight {
		Weight::zero()
	}

	fn register_minting_authority() -> Weight {
		Weight::zero()
	}

	fn deactivate_minting_authority() -> Weight {
		Weight::zero()
	}

	fn approve_queue_entries(_approvals: u32) -> Weight {
		Weight::zero()
	}

	fn prove_gateway_activity(_activities: u32) -> Weight {
		Weight::zero()
	}

	fn transfer_out() -> Weight {
		Weight::zero()
	}

	fn collateralize_transfer() -> Weight {
		Weight::zero()
	}

	fn provider_is_crosschain_activated() -> Weight {
		Weight::zero()
	}

	fn provider_account_uniswap_argon_transfers_in_amount() -> Weight {
		Weight::zero()
	}

	fn provider_has_overdue_collect_blocker() -> Weight {
		Weight::zero()
	}
}

pub fn prove_gateway_activity_with_providers<T: Config>(
	proof_blocks: u32,
	activities: u32,
) -> Weight {
	let extra_activities = activities.saturating_sub(proof_blocks);
	T::WeightInfo::prove_gateway_activity(activities).saturating_add(
		<<T::EthereumVerifier as EthereumVerifyProvider>::Weights as
			EthereumVerifyProviderWeightInfo>::verify_receipt_logs(proof_blocks, extra_activities),
	)
}
