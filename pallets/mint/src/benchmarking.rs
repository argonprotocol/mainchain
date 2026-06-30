#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_support::traits::Hooks;
use pallet_prelude::{
	argon_primitives::{bitcoin::UtxoId, UtxoLockEvents},
	benchmarking::{
		set_benchmark_bitcoin_locks_runtime_state, set_benchmark_price_provider_state,
		BenchmarkBitcoinLocksRuntimeState, BenchmarkPriceProviderState,
	},
};
use polkadot_sdk::{frame_benchmarking::v2::*, sp_runtime::FixedU128};

const MAX_PENDING_MINT_PAYOUT_WINDOW_BENCH: u32 = 1_000;

#[benchmarks(
	where
		T::AccountId: codec::Decode,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn on_initialize(
		u: Linear<0, MAX_PENDING_MINT_PAYOUT_WINDOW_BENCH>,
	) -> Result<(), BenchmarkError> {
		let utxo_count = u;
		let max_pending_mints_per_utxo = T::MaxPendingMintsPerUtxo::get().max(1);
		let queued_amount = T::Balance::from(1u128);
		let max_amount_per_frame = Pallet::<T>::get_bitcoin_mint_payout_cap(queued_amount);
		let minimum_balance = T::Currency::minimum_balance();
		for i in 0..utxo_count {
			let utxo_id = i.saturating_div(max_pending_mints_per_utxo).saturating_add(1) as UtxoId;
			let account_id = account("mint-queue", i, 0);
			if minimum_balance > T::Balance::zero() {
				let initial_balance = minimum_balance.saturating_add(queued_amount);
				T::Currency::mint_into(&account_id, initial_balance)
					.map_err(|_| BenchmarkError::Stop("failed to seed benchmark mint account"))?;
			}
			PendingMintUtxosByIndex::<T>::insert(
				i as MintIndex,
				PendingMintUtxo::<T> {
					utxo_id,
					account_id,
					remaining_amount: queued_amount,
					max_amount_per_frame,
				},
			);
			PendingMintUtxoIdLookup::<T>::try_mutate(utxo_id, |pending_indices| {
				pending_indices
					.try_push(i as MintIndex)
					.map_err(|_| BenchmarkError::Stop("pending mint lookup capacity exceeded"))
			})?;
		}
		NextPendingMintUtxoIndex::<T>::put(utxo_count as MintIndex);

		let total_frame_payout: T::Balance = T::Balance::from(utxo_count.max(1) as u128);
		MintedMiningMicrogons::<T>::put(total_frame_payout);
		MintedBitcoinMicrogons::<T>::put(T::Balance::from(0u128));

		set_benchmark_bitcoin_locks_runtime_state(BenchmarkBitcoinLocksRuntimeState {
			current_frame_id: 2,
			current_tick: 20,
			did_start_new_frame: true,
		});
		set_benchmark_price_provider_state(BenchmarkPriceProviderState {
			btc_price_in_usd: Some(FixedU128::from_rational(62_000_00u128, 100u128)),
			argon_price_in_usd: Some(FixedU128::one()),
			argonot_price_in_usd: Some(FixedU128::one()),
			argon_target_price_in_usd: Some(FixedU128::from_rational(9u128, 10u128)),
			circulation: 200_000,
		});
		frame_system::Pallet::<T>::set_block_number(1u32.into());

		#[block]
		{
			Pallet::<T>::on_initialize(1u32.into());
		}

		let queue_cursor = PendingMintQueueState::<T>::get();
		assert_eq!(NextPendingMintUtxoIndex::<T>::get(), utxo_count as MintIndex);
		assert_eq!(queue_cursor.payout_start_index, utxo_count as MintIndex);
		assert_eq!(queue_cursor.payout_cursor_index, utxo_count as MintIndex);
		for i in 0..utxo_count {
			assert!(PendingMintUtxosByIndex::<T>::get(i as MintIndex).is_none());
			assert!(PendingMintUtxoIdLookup::<T>::get(
				i.saturating_div(max_pending_mints_per_utxo).saturating_add(1) as UtxoId
			)
			.is_empty());
		}
		assert_eq!(MintedBitcoinMicrogons::<T>::get(), T::Balance::from(utxo_count as u128));

		Ok(())
	}

	#[benchmark]
	fn provider_utxo_locked() -> Result<(), BenchmarkError> {
		let utxo_id = 1u64;
		let account_id: T::AccountId = account("mint-provider-lock", 0, 0);
		let amount = T::Balance::from(100u128);
		let existing_pending = T::MaxPendingMintsPerUtxo::get().saturating_sub(1);

		for i in 0..existing_pending {
			PendingMintUtxosByIndex::<T>::insert(
				i as MintIndex,
				PendingMintUtxo::<T> {
					utxo_id,
					account_id: account_id.clone(),
					remaining_amount: amount,
					max_amount_per_frame: Pallet::<T>::get_bitcoin_mint_payout_cap(amount),
				},
			);
			PendingMintUtxoIdLookup::<T>::try_mutate(utxo_id, |pending_indices| {
				pending_indices
					.try_push(i as MintIndex)
					.map_err(|_| BenchmarkError::Stop("pending mint lookup capacity exceeded"))
			})?;
		}
		NextPendingMintUtxoIndex::<T>::put(existing_pending as MintIndex);

		#[block]
		{
			<Pallet<T> as UtxoLockEvents<T::AccountId, T::Balance>>::utxo_locked(
				utxo_id,
				&account_id,
				amount,
			)?;
		}

		let pending_indices = PendingMintUtxoIdLookup::<T>::get(utxo_id);
		assert_eq!(pending_indices.len(), existing_pending.saturating_add(1) as usize);
		assert!(PendingMintUtxosByIndex::<T>::contains_key(existing_pending as MintIndex));

		Ok(())
	}

	#[benchmark]
	fn provider_utxo_released() -> Result<(), BenchmarkError> {
		let account_id: T::AccountId = account("mint-provider-release", 0, 0);
		let amount_burned = T::Balance::from(100u128);
		MintedBitcoinMicrogons::<T>::put(amount_burned);

		#[block]
		{
			<Pallet<T> as UtxoLockEvents<T::AccountId, T::Balance>>::utxo_released(
				1u64,
				&account_id,
				false,
				amount_burned,
				amount_burned,
			)?;
		}

		assert_eq!(MintedBitcoinMicrogons::<T>::get(), T::Balance::zero());

		Ok(())
	}

	#[benchmark]
	fn provider_utxo_released_with_pending_mints() -> Result<(), BenchmarkError> {
		let utxo_id = 1u64;
		let account_id: T::AccountId = account("mint-provider-release", 0, 0);
		let amount = T::Balance::from(100u128);
		let pending_count = T::MaxPendingMintsPerUtxo::get();

		for i in 0..pending_count {
			PendingMintUtxosByIndex::<T>::insert(
				i as MintIndex,
				PendingMintUtxo::<T> {
					utxo_id,
					account_id: account_id.clone(),
					remaining_amount: amount,
					max_amount_per_frame: Pallet::<T>::get_bitcoin_mint_payout_cap(amount),
				},
			);
			PendingMintUtxoIdLookup::<T>::try_mutate(utxo_id, |pending_indices| {
				pending_indices
					.try_push(i as MintIndex)
					.map_err(|_| BenchmarkError::Stop("pending mint lookup capacity exceeded"))
			})?;
		}
		MintedBitcoinMicrogons::<T>::put(amount);

		#[block]
		{
			<Pallet<T> as UtxoLockEvents<T::AccountId, T::Balance>>::utxo_released(
				utxo_id,
				&account_id,
				true,
				amount,
				amount,
			)?;
		}

		assert!(PendingMintUtxoIdLookup::<T>::get(utxo_id).is_empty());
		for i in 0..pending_count {
			assert!(!PendingMintUtxosByIndex::<T>::contains_key(i as MintIndex));
		}

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
