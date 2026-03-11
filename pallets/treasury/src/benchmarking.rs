#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_primitives::{TreasuryPoolProvider, VaultId};
use frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn provider_has_pool_participation() -> Result<(), BenchmarkError> {
		let account_id: T::AccountId = account("treasury_pool_participant", 0, 0);
		let vault_id: VaultId = 1;
		FunderStateByVaultAndAccount::<T>::insert(
			vault_id,
			&account_id,
			FunderState::<T>::default(),
		);

		#[block]
		{
			assert!(<Pallet<T> as TreasuryPoolProvider<T::AccountId>>::has_pool_participation(
				vault_id,
				&account_id,
			));
		}

		Ok(())
	}
}
