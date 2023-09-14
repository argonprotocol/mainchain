//! Benchmarking setup for pallet-block-seal
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Cohorts;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

const USER_SEED: u32 = 0;
#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn bond() {
		let stash = account("stash", 1, USER_SEED);
		T::OwnershipCurrency::make_free_balance_be(&stash, 100);
		let reward_destination = RewardDestination::Stash;
		let amount = T::OwnershipCurrency::minimum_balance() * 10u32.into();
		whitelist_account!(stash);
		#[extrinsic_call]
		bond(RawOrigin::Signed(stash.clone()).into(), stash.clone(), amount, reward_destination);

		assert!(Bonded::<T>::contains_key(stash.clone()));
		assert!(Ledger::<T>::contains_key(stash));
	}

	impl_benchmark_test_suite!(Cohorts, crate::mock::new_test_ext(), crate::mock::Test);
}
