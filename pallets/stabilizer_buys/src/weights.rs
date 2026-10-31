use crate::Config;
use argon_primitives::{
	treasury::{TreasuryRewardProvider, TreasuryRewardProviderWeightInfo},
	UniswapPurchaseProviderWeightInfo,
};
use core::marker::PhantomData;
use polkadot_sdk::frame_support::weights::Weight;

/// Measured execution costs for price history, purchase admission and bounded payout work.
pub trait WeightInfo {
	fn record_prices() -> Weight;
	fn record_purchase() -> Weight;
	fn record_purchase_first_hold() -> Weight;
	fn bid() -> Weight;
	fn process_payouts(max_accounts: u32) -> Weight;
	fn process_payouts_failed(max_accounts: u32) -> Weight;
	fn process_auctions(max_bids: u32) -> Weight;
	fn set_approved_pool() -> Weight;
	fn set_matching_rate() -> Weight;
	fn on_frame_start() -> Weight;
}

/// Add the separately benchmarked Treasury calls to the auction's own execution costs.
pub struct WithProviderWeights<T, Base>(PhantomData<(T, Base)>);
type TreasuryWeights<T> = <<T as Config>::TreasuryRewardProvider as TreasuryRewardProvider<
	<T as polkadot_sdk::frame_system::Config>::AccountId,
>>::Weights;

impl<T: Config, Base: WeightInfo> WeightInfo for WithProviderWeights<T, Base> {
	fn record_prices() -> Weight {
		Base::record_prices()
	}
	fn record_purchase() -> Weight {
		Base::record_purchase()
	}
	fn record_purchase_first_hold() -> Weight {
		Base::record_purchase_first_hold()
	}
	fn bid() -> Weight {
		Base::bid()
	}
	fn set_approved_pool() -> Weight {
		Base::set_approved_pool()
	}
	fn set_matching_rate() -> Weight {
		Base::set_matching_rate()
	}
	fn on_frame_start() -> Weight {
		Base::on_frame_start()
			.saturating_add(TreasuryWeights::<T>::available_budget())
			.saturating_add(TreasuryWeights::<T>::reserve_budget())
	}
	fn process_auctions(max_bids: u32) -> Weight {
		Base::process_auctions(max_bids).saturating_add(
			TreasuryWeights::<T>::commit_reward()
				.saturating_add(TreasuryWeights::<T>::return_budget())
				.saturating_mul(max_bids.into()),
		)
	}
	fn process_payouts(max_accounts: u32) -> Weight {
		Base::process_payouts(max_accounts)
			.saturating_add(TreasuryWeights::<T>::pay_reward().saturating_mul(max_accounts.into()))
	}
	fn process_payouts_failed(max_accounts: u32) -> Weight {
		Base::process_payouts_failed(max_accounts)
			.saturating_add(TreasuryWeights::<T>::pay_reward().saturating_mul(max_accounts.into()))
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: Config> UniswapPurchaseProviderWeightInfo for ProviderWeightAdapter<T> {
	fn record_transferred_purchase() -> Weight {
		T::WeightInfo::record_purchase().max(T::WeightInfo::record_purchase_first_hold())
	}
}
