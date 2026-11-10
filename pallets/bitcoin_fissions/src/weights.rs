use crate::Config;
use argon_primitives::{
	providers::{
		BitcoinFissionLockProvider, BitcoinFissionLockProviderWeightInfo, BitcoinFissionMinting,
		BitcoinFissionMintingWeightInfo,
	},
	treasury::{TreasuryPositionProvider, TreasuryPositionProviderWeightInfo},
	BitcoinFissionsProviderWeightInfo, CrosschainGatewayProvider,
	CrosschainGatewayProviderWeightInfo, LiquidRepaymentProvider,
	LiquidRepaymentProviderWeightInfo, MiningFrameTransitionProvider,
	MiningFrameTransitionProviderWeightInfo, OperationalAccountsHook,
	OperationalAccountsHookWeightInfo, TickProvider, TickProviderWeightInfo,
};
use pallet_prelude::*;

pub trait WeightInfo {
	fn provider_has_repayment_window(fissions: u32) -> Weight;
	fn provider_next_r_value_high_water() -> Weight;
	fn record_r_value() -> Weight;
	fn prune_r_values(max_entries: u32) -> Weight;
	fn create() -> Weight;
	fn ratchet() -> Weight;
	fn close() -> Weight;
	fn repay_liquid(fissions: u32) -> Weight;
	fn repay_liquid_complete(fissions: u32) -> Weight;
	fn lock_spent(fissions: u32) -> Weight;
	fn provider_get_account_fission_liquidity() -> Weight;
	fn provider_get_lock_fission_guarantees() -> Weight;
}

type FissionMintingWeights<T> = <<T as Config>::Minting as BitcoinFissionMinting<
	<T as frame_system::Config>::AccountId,
	<T as Config>::Balance,
>>::Weights;
type LockProviderWeights<T> = <<T as Config>::LockProvider as BitcoinFissionLockProvider<
	<T as frame_system::Config>::AccountId,
	<T as Config>::Balance,
>>::Weights;
type PositionProviderWeights<T> = <<T as Config>::PositionProvider as TreasuryPositionProvider<
	<T as frame_system::Config>::AccountId,
	<T as Config>::Balance,
>>::Weights;

type ClockWeights<T> =
	<<T as Config>::TickProvider as TickProvider<<T as frame_system::Config>::Block>>::Weights;
type FrameWeights<T> =
	<<T as Config>::MiningFrameTransitionProvider as MiningFrameTransitionProvider>::Weights;
type GatewayWeights<T> = <<T as Config>::CrosschainGatewayProvider as CrosschainGatewayProvider<
	<T as Config>::Balance,
>>::Weights;

pub struct WithProviderWeights<
	T,
	Base,
	MintingWeight = FissionMintingWeights<T>,
	LockProviderWeight = LockProviderWeights<T>,
>(PhantomData<(T, Base, MintingWeight, LockProviderWeight)>);

impl<T, Base, MintingWeight, LockProviderWeight> WeightInfo
	for WithProviderWeights<T, Base, MintingWeight, LockProviderWeight>
where
	T: Config,
	Base: WeightInfo,
	MintingWeight: BitcoinFissionMintingWeightInfo,
	LockProviderWeight: BitcoinFissionLockProviderWeightInfo,
{
	fn provider_has_repayment_window(fissions: u32) -> Weight {
		Base::provider_has_repayment_window(fissions).saturating_add(
			LockProviderWeight::has_repayment_window().saturating_mul(fissions.into()),
		)
	}

	fn provider_next_r_value_high_water() -> Weight {
		Base::provider_next_r_value_high_water()
	}
	fn record_r_value() -> Weight {
		Base::record_r_value()
			.saturating_add(ClockWeights::<T>::current_tick())
			.saturating_add(FrameWeights::<T>::is_new_frame_started())
			.saturating_add(FrameWeights::<T>::get_current_frame_id())
	}
	fn prune_r_values(max_entries: u32) -> Weight {
		Base::prune_r_values(max_entries)
			.saturating_add(<T::LiquidRepaymentProvider as LiquidRepaymentProvider<T::AccountId>>::Weights::oldest_pending_purchase_tick())
			.saturating_add(GatewayWeights::<T>::last_state_by_chain())
			.saturating_add(ClockWeights::<T>::ticker())
			.saturating_add(ClockWeights::<T>::current_tick().saturating_mul(2))
			.saturating_add(FrameWeights::<T>::is_new_frame_started())
			.saturating_add(FrameWeights::<T>::get_current_frame_id())
	}
	fn create() -> Weight {
		Base::create()
			.saturating_add(ClockWeights::<T>::ticker())
			.saturating_add(PositionProviderWeights::<T>::account_quantity_updated())
			.saturating_add(LockProviderWeight::fission_satoshis())
			.saturating_add(MintingWeight::request_mint())
			.saturating_add(<T::OperationalAccountsHook as OperationalAccountsHook<
				T::AccountId,
				T::Balance,
			>>::Weights::account_bitcoin_amount_changed())
	}

	fn ratchet() -> Weight {
		Base::ratchet()
			.saturating_add(<T::LiquidRepaymentProvider as LiquidRepaymentProvider<T::AccountId>>::Weights::has_pending_repayment())
			.saturating_add(ClockWeights::<T>::ticker())
			.saturating_add(PositionProviderWeights::<T>::account_quantity_updated())
			.saturating_add(LockProviderWeight::update_fission_guarantee())
			.saturating_add(LockProviderWeight::calculate_liquidity_promised())
			.saturating_add(MintingWeight::request_mint())
			.saturating_add(MintingWeight::record_mint_repayment())
			.saturating_add(<T::OperationalAccountsHook as OperationalAccountsHook<
				T::AccountId,
				T::Balance,
			>>::Weights::account_bitcoin_amount_changed())
	}

	fn close() -> Weight {
		Base::close()
			.saturating_add(LockProviderWeight::calculate_redemption_amount())
			.saturating_add(<T::LiquidRepaymentProvider as LiquidRepaymentProvider<T::AccountId>>::Weights::has_pending_repayment())
			.saturating_add(PositionProviderWeights::<T>::account_quantity_updated())
			.saturating_add(LockProviderWeight::fuse_satoshis())
			.saturating_add(MintingWeight::record_mint_repayment())
			.saturating_add(<T::OperationalAccountsHook as OperationalAccountsHook<
				T::AccountId,
				T::Balance,
			>>::Weights::account_bitcoin_amount_changed())
	}

	fn repay_liquid(fissions: u32) -> Weight {
		let account_providers = PositionProviderWeights::<T>::account_quantity_updated()
			.saturating_add(MintingWeight::record_mint_repayment())
			.saturating_add(<T::OperationalAccountsHook as OperationalAccountsHook<
				T::AccountId,
				T::Balance,
			>>::Weights::account_bitcoin_amount_changed());
		// Full repayment removes each Fission's index entry; partial repayment performs the search.
		Base::repay_liquid(fissions)
			.max(Base::repay_liquid_complete(fissions))
			.saturating_add(account_providers)
			.saturating_add(LockProviderWeight::fuse_satoshis().saturating_mul(fissions.into()))
			.saturating_add(
				LockProviderWeight::get_lock_expiration().saturating_mul(fissions.into()),
			)
	}

	fn repay_liquid_complete(fissions: u32) -> Weight {
		Self::repay_liquid(fissions)
	}

	fn lock_spent(fissions: u32) -> Weight {
		Base::lock_spent(fissions)
			.saturating_add(PositionProviderWeights::<T>::account_quantity_updated().saturating_mul(fissions.into()))
			.saturating_add(
				<T::OperationalAccountsHook as OperationalAccountsHook<
					T::AccountId,
					T::Balance,
				>>::Weights::account_bitcoin_amount_changed()
				.saturating_mul(fissions.into()),
			)
			.saturating_add(MintingWeight::record_mint_repayment())
	}

	fn provider_get_account_fission_liquidity() -> Weight {
		Base::provider_get_account_fission_liquidity()
			.saturating_add(PositionProviderWeights::<T>::account_quantities())
	}

	fn provider_get_lock_fission_guarantees() -> Weight {
		Base::provider_get_lock_fission_guarantees()
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: Config> BitcoinFissionsProviderWeightInfo for ProviderWeightAdapter<T> {
	fn has_repayment_window() -> Weight {
		T::WeightInfo::provider_has_repayment_window(T::MaxExpectedFissionsPerLiquid::get())
	}

	fn next_r_value_high_water() -> Weight {
		T::WeightInfo::provider_next_r_value_high_water()
	}
	fn repay_liquid(burn_callback_weight: Weight) -> Weight {
		let fissions = T::MaxExpectedFissionsPerLiquid::get();
		T::WeightInfo::repay_liquid(fissions).saturating_add(burn_callback_weight)
	}

	fn get_account_fission_liquidity() -> Weight {
		T::WeightInfo::provider_get_account_fission_liquidity()
	}

	fn get_lock_fission_guarantees() -> Weight {
		T::WeightInfo::provider_get_lock_fission_guarantees()
	}

	fn close_for_lock() -> Weight {
		T::WeightInfo::lock_spent(T::MaxFissionsPerLock::get())
	}
}

impl WeightInfo for () {
	fn provider_has_repayment_window(_: u32) -> Weight {
		Weight::zero()
	}

	fn provider_next_r_value_high_water() -> Weight {
		Weight::zero()
	}
	fn repay_liquid(_fissions: u32) -> Weight {
		Weight::zero()
	}
	fn repay_liquid_complete(_fissions: u32) -> Weight {
		Weight::zero()
	}

	fn prune_r_values(_max_entries: u32) -> Weight {
		Weight::zero()
	}
	fn record_r_value() -> Weight {
		Weight::zero()
	}
	fn create() -> Weight {
		Weight::zero()
	}

	fn ratchet() -> Weight {
		Weight::zero()
	}

	fn close() -> Weight {
		Weight::zero()
	}

	fn lock_spent(_fissions: u32) -> Weight {
		Weight::zero()
	}

	fn provider_get_account_fission_liquidity() -> Weight {
		Weight::zero()
	}

	fn provider_get_lock_fission_guarantees() -> Weight {
		Weight::zero()
	}
}
