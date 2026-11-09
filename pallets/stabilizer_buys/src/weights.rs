use crate::{Balance, Config};
use argon_primitives::{
	treasury::{TreasuryRewardProvider, TreasuryRewardProviderWeightInfo},
	BitcoinFissionsProvider, BitcoinFissionsProviderWeightInfo, CrosschainGatewayProvider,
	CrosschainGatewayProviderWeightInfo, LiquidRepaymentProviderWeightInfo,
	MiningFrameTransitionProvider, MiningFrameTransitionProviderWeightInfo, TickProvider,
	TickProviderWeightInfo, UniswapPurchaseProviderWeightInfo,
};
use core::marker::PhantomData;
use polkadot_sdk::frame_support::weights::Weight;

/// Measured execution costs for price history, purchase admission and bounded payout work.
pub trait WeightInfo {
	fn oldest_pending_purchase_tick() -> Weight;
	fn process_repayment_prices(reads: u32) -> Weight;
	fn has_pending_liquid_repayment() -> Weight;
	fn process_liquid_repayment() -> Weight;
	fn process_reward_payouts(accounts: u32) -> Weight;
	fn record_prices() -> Weight;
	fn prune_prices(max_entries: u32) -> Weight;
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

/// Add separately measured Fission and Treasury operations to the buys pallet's own costs.
pub struct WithProviderWeights<T, Base>(PhantomData<(T, Base)>);
type TreasuryWeights<T> = <<T as Config>::TreasuryRewardProvider as TreasuryRewardProvider<
	<T as polkadot_sdk::frame_system::Config>::AccountId,
>>::Weights;

type FissionsWeights<T> = <<T as Config>::FissionsProvider as BitcoinFissionsProvider<
	<T as polkadot_sdk::frame_system::Config>::AccountId,
	Balance,
>>::Weights;

type ClockWeights<T> = <<T as Config>::TickProvider as TickProvider<
	<T as polkadot_sdk::frame_system::Config>::Block,
>>::Weights;
type FrameWeights<T> =
	<<T as Config>::MiningFrameTransitionProvider as MiningFrameTransitionProvider>::Weights;
type GatewayWeights<T> =
	<<T as Config>::CrosschainGatewayProvider as CrosschainGatewayProvider<Balance>>::Weights;

impl<T: Config, Base: WeightInfo> WeightInfo for WithProviderWeights<T, Base> {
	fn oldest_pending_purchase_tick() -> Weight {
		Base::oldest_pending_purchase_tick()
	}
	fn process_repayment_prices(reads: u32) -> Weight {
		Base::process_repayment_prices(reads).saturating_add(
			FissionsWeights::<T>::next_r_value_high_water().saturating_mul(reads.into()),
		)
	}
	fn has_pending_liquid_repayment() -> Weight {
		Base::has_pending_liquid_repayment()
	}

	fn process_liquid_repayment() -> Weight {
		Base::process_liquid_repayment()
			.saturating_add(FrameWeights::<T>::get_current_frame_id())
			.saturating_add(FissionsWeights::<T>::repay_liquid(Weight::zero()))
	}

	fn process_reward_payouts(accounts: u32) -> Weight {
		Base::process_reward_payouts(accounts).saturating_add(
			FrameWeights::<T>::get_current_frame_id().saturating_mul(2 * u64::from(accounts)),
		)
	}

	fn record_prices() -> Weight {
		Base::record_prices()
			.saturating_add(ClockWeights::<T>::current_tick())
			.saturating_add(FrameWeights::<T>::is_new_frame_started())
			.saturating_add(FrameWeights::<T>::get_current_frame_id())
	}
	fn prune_prices(max_entries: u32) -> Weight {
		Base::prune_prices(max_entries)
			.saturating_add(GatewayWeights::<T>::last_state_by_chain())
			.saturating_add(ClockWeights::<T>::ticker())
			.saturating_add(ClockWeights::<T>::current_tick())
	}
	fn record_purchase() -> Weight {
		Base::record_purchase()
			.saturating_add(FissionsWeights::<T>::has_repayment_window())
			.saturating_add(FrameWeights::<T>::get_current_frame_id())
			.saturating_add(FrameWeights::<T>::get_current_frame_id())
			.saturating_add(ClockWeights::<T>::current_tick())
	}
	fn record_purchase_first_hold() -> Weight {
		Base::record_purchase_first_hold()
			.saturating_add(FissionsWeights::<T>::has_repayment_window())
			.saturating_add(FrameWeights::<T>::get_current_frame_id())
			.saturating_add(FrameWeights::<T>::get_current_frame_id())
			.saturating_add(ClockWeights::<T>::current_tick())
	}

	fn bid() -> Weight {
		Base::bid().saturating_add(FrameWeights::<T>::get_current_frame_id())
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
		Base::process_auctions(max_bids)
			.saturating_add(
				FrameWeights::<T>::get_current_frame_id().saturating_mul(max_bids.into()),
			)
			.saturating_add(
				TreasuryWeights::<T>::commit_reward()
					.saturating_add(TreasuryWeights::<T>::return_budget())
					.saturating_mul(max_bids.into()),
			)
	}
	fn process_payouts(max_accounts: u32) -> Weight {
		Base::process_payouts(max_accounts)
			.max(Base::process_reward_payouts(max_accounts))
			.saturating_add(
				FrameWeights::<T>::get_current_frame_id()
					.saturating_mul(2 * u64::from(max_accounts)),
			)
			.saturating_add(TreasuryWeights::<T>::pay_reward().saturating_mul(max_accounts.into()))
	}
	fn process_payouts_failed(max_accounts: u32) -> Weight {
		Base::process_payouts_failed(max_accounts)
			.saturating_add(
				FrameWeights::<T>::get_current_frame_id()
					.saturating_mul(2 * u64::from(max_accounts)),
			)
			.saturating_add(TreasuryWeights::<T>::pay_reward().saturating_mul(max_accounts.into()))
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: Config> LiquidRepaymentProviderWeightInfo for ProviderWeightAdapter<T> {
	fn oldest_pending_purchase_tick() -> Weight {
		T::WeightInfo::oldest_pending_purchase_tick()
	}
	fn has_pending_repayment() -> Weight {
		T::WeightInfo::has_pending_liquid_repayment()
	}
}

impl<T: Config> UniswapPurchaseProviderWeightInfo for ProviderWeightAdapter<T> {
	fn record_transferred_purchase() -> Weight {
		T::WeightInfo::record_purchase().max(T::WeightInfo::record_purchase_first_hold())
	}
}

impl WeightInfo for () {
	fn oldest_pending_purchase_tick() -> Weight {
		Weight::zero()
	}
	fn process_repayment_prices(_reads: u32) -> Weight {
		Weight::zero()
	}
	fn has_pending_liquid_repayment() -> Weight {
		Weight::zero()
	}

	fn process_reward_payouts(_accounts: u32) -> Weight {
		Weight::zero()
	}
	fn process_liquid_repayment() -> Weight {
		Weight::zero()
	}
	fn prune_prices(_max_entries: u32) -> Weight {
		Weight::zero()
	}
	fn record_prices() -> Weight {
		Weight::zero()
	}
	fn record_purchase() -> Weight {
		Weight::zero()
	}
	fn record_purchase_first_hold() -> Weight {
		Weight::zero()
	}
	fn bid() -> Weight {
		Weight::zero()
	}
	fn process_payouts_failed(_: u32) -> Weight {
		Weight::zero()
	}
	fn process_payouts(_: u32) -> Weight {
		Weight::zero()
	}
	fn process_auctions(_: u32) -> Weight {
		Weight::zero()
	}
	fn set_approved_pool() -> Weight {
		Weight::zero()
	}
	fn set_matching_rate() -> Weight {
		Weight::zero()
	}
	fn on_frame_start() -> Weight {
		Weight::zero()
	}
}
