use super::Config;
use argon_primitives::{
	treasury::{TreasuryPositionProvider, TreasuryPositionProviderWeightInfo},
	BitcoinFissionMintingWeightInfo, BlockRewardAccountsProvider,
	BlockRewardAccountsProviderWeightInfo, MiningFrameTransitionProvider,
	MiningFrameTransitionProviderWeightInfo, PriceProvider, PriceProviderWeightInfo,
};
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_finalize() -> Weight {
		Weight::zero()
	}
	fn on_initialize(utxo_count: u32) -> Weight;
	fn provider_mint_requested() -> Weight;
	fn provider_mint_repaid() -> Weight;
}

type Positions<T> = <<T as Config>::PositionProvider as TreasuryPositionProvider<
	<T as frame_system::Config>::AccountId,
	<T as Config>::Balance,
>>::Weights;
type Accounts<T> = <<T as Config>::BlockRewardAccountsProvider as BlockRewardAccountsProvider<
	<T as frame_system::Config>::AccountId,
>>::Weights;
type FrameWeights<T> =
	<<T as Config>::MiningFrameProvider as MiningFrameTransitionProvider>::Weights;
type PriceWeights<T> =
	<<T as Config>::PriceProvider as PriceProvider<<T as Config>::Balance>>::Weights;

/// Reserve both hooks' frame checks on every block, plus settlement when a frame starts.
pub fn mining_frame<T: Config>() -> Weight {
	let frame_checks = FrameWeights::<T>::is_new_frame_started().saturating_mul(2);
	if T::MiningFrameProvider::is_new_frame_started().is_none() {
		return frame_checks
	}

	let settlement =
		T::WeightInfo::on_finalize()
			.saturating_add(Positions::<T>::allocate_mining_mint())
			.saturating_add(Positions::<T>::mining_seats_changed().saturating_mul(u64::from(
				T::BlockRewardAccountsProvider::max_mining_seat_changes(),
			)))
			.saturating_add(Accounts::<T>::get_mint_rewards_cohorts())
			.saturating_add(Accounts::<T>::get_mining_seat_changes())
			.saturating_add(PriceWeights::<T>::get_average_cpi_for_ticks())
			.saturating_add(PriceWeights::<T>::get_liquidity_change_needed());
	frame_checks.saturating_add(settlement)
}

pub struct FissionMintingWeightAdapter<T>(PhantomData<T>);
impl<T: Config> BitcoinFissionMintingWeightInfo for FissionMintingWeightAdapter<T> {
	fn request_mint() -> Weight {
		<T as Config>::WeightInfo::provider_mint_requested()
	}

	fn record_mint_repayment() -> Weight {
		<T as Config>::WeightInfo::provider_mint_repaid()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_initialize(_utxo_count: u32) -> Weight {
		Weight::zero()
	}

	fn provider_mint_requested() -> Weight {
		Weight::zero()
	}

	fn provider_mint_repaid() -> Weight {
		Weight::zero()
	}
}
