use crate::Config;
use argon_primitives::{
	providers::{
		BitcoinFissionLockProvider, BitcoinFissionLockProviderWeightInfo, BitcoinFissionMinting,
		BitcoinFissionMintingWeightInfo, OperationalAccountsHook,
	},
	BitcoinFissionsProviderWeightInfo,
};
use pallet_prelude::*;

pub trait WeightInfo {
	fn create() -> Weight;
	fn ratchet() -> Weight;
	fn close() -> Weight;
	fn lock_spent(fissions: u32) -> Weight;
	fn provider_get_account_fission_liquidity() -> Weight;
	fn provider_get_lock_fission_requirements() -> Weight;
}

type FissionMintingWeights<T> = <<T as Config>::Minting as BitcoinFissionMinting<
	<T as frame_system::Config>::AccountId,
	<T as Config>::Balance,
>>::Weights;
type LockProviderWeights<T> = <<T as Config>::LockProvider as BitcoinFissionLockProvider<
	<T as frame_system::Config>::AccountId,
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
	fn create() -> Weight {
		Base::create()
			.saturating_add(LockProviderWeight::fission_satoshis())
			.saturating_add(MintingWeight::request_mint())
			.saturating_add(T::OperationalAccountsHook::account_bitcoin_amount_changed_weight())
	}

	fn ratchet() -> Weight {
		Base::ratchet()
			.saturating_add(LockProviderWeight::validate_fission())
			.saturating_add(LockProviderWeight::calculate_liquidity_promised())
			.saturating_add(MintingWeight::request_mint())
			.saturating_add(MintingWeight::record_mint_repayment())
			.saturating_add(T::OperationalAccountsHook::account_bitcoin_amount_changed_weight())
	}

	fn close() -> Weight {
		Base::close()
			.saturating_add(LockProviderWeight::fuse_satoshis())
			.saturating_add(MintingWeight::record_mint_repayment())
			.saturating_add(T::OperationalAccountsHook::account_bitcoin_amount_changed_weight())
	}

	fn lock_spent(fissions: u32) -> Weight {
		Base::lock_spent(fissions)
			.saturating_add(
				T::OperationalAccountsHook::account_bitcoin_amount_changed_weight()
					.saturating_mul(fissions.into()),
			)
			.saturating_add(MintingWeight::record_mint_repayment())
	}

	fn provider_get_account_fission_liquidity() -> Weight {
		Base::provider_get_account_fission_liquidity()
	}

	fn provider_get_lock_fission_requirements() -> Weight {
		Base::provider_get_lock_fission_requirements()
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: Config> BitcoinFissionsProviderWeightInfo for ProviderWeightAdapter<T> {
	fn get_account_fission_liquidity() -> Weight {
		T::WeightInfo::provider_get_account_fission_liquidity()
	}

	fn get_lock_fission_requirements() -> Weight {
		T::WeightInfo::provider_get_lock_fission_requirements()
	}

	fn close_for_lock() -> Weight {
		T::WeightInfo::lock_spent(T::MaxFissionsPerLock::get())
	}
}

impl WeightInfo for () {
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

	fn provider_get_lock_fission_requirements() -> Weight {
		Weight::zero()
	}
}
