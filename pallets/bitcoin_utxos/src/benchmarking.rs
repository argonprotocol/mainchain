#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_primitives::{
	bitcoin::{
		BitcoinBlock, BitcoinCosignScriptPubkey, BitcoinHeight, H256Le, Satoshis, UtxoId, UtxoRef,
		UtxoValue,
	},
	inherents::BitcoinUtxoSync,
};
use frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
	use super::*;
	use frame_system::RawOrigin;

	const MAX_SYNC_ITEMS: u32 = 20;

	#[benchmark]
	fn sync_base() -> Result<(), BenchmarkError> {
		let sync_to_block =
			BitcoinBlock { block_height: 1_000, block_hash: benchmark_block_hash(200) };
		ConfirmedBitcoinBlockTip::<T>::put(sync_to_block.clone());
		SynchedBitcoinBlock::<T>::put(BitcoinBlock {
			block_height: sync_to_block.block_height.saturating_sub(1),
			block_hash: benchmark_block_hash(199),
		});
		InherentIncluded::<T>::put(false);
		let satoshis = benchmark_satoshis::<T>();
		for i in 0..T::MaxPendingConfirmationUtxos::get() {
			let utxo_id = i.saturating_add(1) as UtxoId;
			seed_pending_funding_at_height::<T>(utxo_id, satoshis, sync_to_block.block_height)?;
		}
		let utxo_sync =
			BitcoinUtxoSync { spent: vec![], funded: vec![], sync_to_block: sync_to_block.clone() };

		#[block]
		{
			Pallet::<T>::sync(RawOrigin::None.into(), utxo_sync)
				.map_err(|_| BenchmarkError::Stop("sync base failed"))?;
		}

		assert_eq!(SynchedBitcoinBlock::<T>::get(), Some(sync_to_block));
		assert!(InherentIncluded::<T>::get());
		Ok(())
	}

	#[benchmark]
	fn on_initialize_base() -> Result<(), BenchmarkError> {
		let sync_to_block =
			BitcoinBlock { block_height: 1_000, block_hash: benchmark_block_hash(201) };
		ConfirmedBitcoinBlockTip::<T>::put(sync_to_block.clone());
		PreviousBitcoinBlockTip::<T>::kill();
		TempParentHasSyncState::<T>::put(false);
		let satoshis = benchmark_satoshis::<T>();
		ExpiredPendingFunding::<T>::try_mutate(|expired| {
			for i in 0..T::MaxPendingConfirmationUtxos::get() {
				let utxo_id = i.saturating_add(1) as UtxoId;
				expired
					.try_insert(utxo_id, benchmark_utxo_value(utxo_id, satoshis))
					.map_err(|_| BenchmarkError::Stop("expired pending funding storage full"))?;
			}
			Ok::<(), BenchmarkError>(())
		})?;

		#[block]
		{
			Pallet::<T>::prepare_expired_pending_funding_cleanup(
				T::MaxPendingFundingExpirationsPerBlock::get(),
			);
		}

		assert_eq!(PreviousBitcoinBlockTip::<T>::get(), Some(sync_to_block));
		assert!(TempParentHasSyncState::<T>::get());
		assert_eq!(
			ExpiredPendingFunding::<T>::get().len(),
			T::MaxPendingConfirmationUtxos::get() as usize
		);
		Ok(())
	}

	#[benchmark]
	fn set_confirmed_block() -> Result<(), BenchmarkError> {
		let operator: T::AccountId = account("bitcoin-utxo-operator", 0, 0);
		let bitcoin_height: BitcoinHeight = 10;
		let bitcoin_block_hash = benchmark_block_hash(1);
		OracleOperatorAccount::<T>::put(operator.clone());

		#[extrinsic_call]
		_(RawOrigin::Signed(operator), bitcoin_height, bitcoin_block_hash.clone());

		assert_eq!(
			ConfirmedBitcoinBlockTip::<T>::get(),
			Some(BitcoinBlock { block_height: bitcoin_height, block_hash: bitcoin_block_hash })
		);
		Ok(())
	}

	#[benchmark]
	fn set_operator() -> Result<(), BenchmarkError> {
		let operator: T::AccountId = account("bitcoin-utxo-operator", 1, 0);

		#[extrinsic_call]
		_(RawOrigin::Root, operator.clone());

		assert_eq!(OracleOperatorAccount::<T>::get(), Some(operator));
		Ok(())
	}

	#[benchmark]
	fn fund_with_utxo_candidate() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("funding-caller", 0, 0);
		let utxo_id: UtxoId = 1;
		let satoshis: Satoshis = benchmark_satoshis::<T>();
		let selected_ref = benchmark_utxo_ref(0);
		seed_pending_funding::<T>(utxo_id, satoshis)?;
		seed_candidates::<T>(utxo_id, T::MaxCandidateUtxosPerLock::get(), Some(&selected_ref))?;

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), utxo_id, selected_ref.clone());

		assert_eq!(UtxoIdToFundingUtxoRef::<T>::get(utxo_id), Some(selected_ref.clone()));
		assert!(LockedUtxos::<T>::contains_key(selected_ref));
		assert!(!LocksPendingFunding::<T>::get().contains_key(&utxo_id));
		assert!(CandidateUtxoRefsByUtxoId::<T>::get(utxo_id).is_empty());
		Ok(())
	}

	#[benchmark]
	fn reject_utxo_candidate() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("reject-caller", 0, 0);
		let utxo_id: UtxoId = 1;
		let candidate_ref = benchmark_utxo_ref(1);
		seed_candidates::<T>(utxo_id, T::MaxCandidateUtxosPerLock::get(), Some(&candidate_ref))?;

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), utxo_id, candidate_ref.clone());

		assert!(!CandidateUtxoRefsByUtxoId::<T>::get(utxo_id).contains_key(&candidate_ref));
		Ok(())
	}

	#[benchmark]
	fn utxo_spent(n: Linear<1, MAX_SYNC_ITEMS>) -> Result<(), BenchmarkError> {
		let satoshis = benchmark_satoshis::<T>();
		let block_height: BitcoinHeight = 10;
		let candidate_count = T::MaxCandidateUtxosPerLock::get();

		for i in 0..n {
			let utxo_id = i.saturating_add(1) as UtxoId;
			let funding_ref = benchmark_utxo_ref(10_000 + i);
			LockedUtxos::<T>::insert(funding_ref.clone(), benchmark_utxo_value(utxo_id, satoshis));
			UtxoIdToFundingUtxoRef::<T>::insert(utxo_id, funding_ref.clone());
			seed_candidates::<T>(utxo_id, candidate_count, None)?;
		}

		#[block]
		{
			for i in 0..n {
				let utxo_id = i.saturating_add(1) as UtxoId;
				let funding_ref = benchmark_utxo_ref(10_000 + i);
				Pallet::<T>::utxo_spent(utxo_id, Some(funding_ref), block_height)
					.map_err(|_| BenchmarkError::Stop("utxo spent failed"))?;
			}
		}

		for i in 0..n {
			let utxo_id = i.saturating_add(1) as UtxoId;
			assert!(UtxoIdToFundingUtxoRef::<T>::get(utxo_id).is_none());
			assert!(CandidateUtxoRefsByUtxoId::<T>::get(utxo_id).is_empty());
		}
		Ok(())
	}

	#[benchmark]
	fn lock_verified(n: Linear<1, MAX_SYNC_ITEMS>) -> Result<(), BenchmarkError> {
		let satoshis = benchmark_satoshis::<T>();
		let bitcoin_height: BitcoinHeight = 10;
		let candidate_count = T::MaxCandidateUtxosPerLock::get();

		for i in 0..n {
			let utxo_id = i.saturating_add(1) as UtxoId;
			seed_pending_funding::<T>(utxo_id, satoshis)?;
			seed_candidates::<T>(utxo_id, candidate_count, None)?;
		}

		#[block]
		{
			for i in 0..n {
				let utxo_id = i.saturating_add(1) as UtxoId;
				let verified_ref = benchmark_utxo_ref(20_000 + i);
				Pallet::<T>::lock_funding_received(utxo_id, verified_ref, satoshis, bitcoin_height)
					.map_err(|_| BenchmarkError::Stop("lock verification failed"))?;
			}
		}

		for i in 0..n {
			let utxo_id = i.saturating_add(1) as UtxoId;
			assert!(UtxoIdToFundingUtxoRef::<T>::get(utxo_id).is_some());
			assert!(CandidateUtxoRefsByUtxoId::<T>::get(utxo_id).is_empty());
		}
		Ok(())
	}

	#[benchmark]
	fn pending_funding_timeout() -> Result<(), BenchmarkError> {
		let satoshis = benchmark_satoshis::<T>();
		let candidate_count = T::MaxCandidateUtxosPerLock::get();
		let utxo_id: UtxoId = 1;
		seed_candidates::<T>(utxo_id, candidate_count, None)?;
		ExpiredPendingFunding::<T>::try_mutate(|expired| {
			expired
				.try_insert(utxo_id, benchmark_utxo_value(utxo_id, satoshis))
				.map_err(|_| BenchmarkError::Stop("expired pending funding storage full"))?;
			Ok::<(), BenchmarkError>(())
		})?;

		#[block]
		{
			Pallet::<T>::process_expired_pending_funding_entry(utxo_id);
		}

		assert!(LocksPendingFunding::<T>::get().is_empty());
		assert!(!ExpiredPendingFunding::<T>::get().contains_key(&utxo_id));
		assert!(CandidateUtxoRefsByUtxoId::<T>::get(utxo_id).is_empty());
		Ok(())
	}
}

fn benchmark_satoshis<T: Config>() -> Satoshis {
	T::MinimumSatoshisPerCandidateUtxo::get()
		.saturating_add(T::MaximumSatoshiThresholdFromExpected::get())
		.saturating_add(1_000)
}

fn benchmark_block_hash(seed: u8) -> H256Le {
	H256Le([seed; 32])
}

fn benchmark_script_pubkey(seed: u32) -> BitcoinCosignScriptPubkey {
	BitcoinCosignScriptPubkey::P2WSH { wscript_hash: sp_core::H256::repeat_byte(seed as u8) }
}

fn benchmark_utxo_ref(seed: u32) -> UtxoRef {
	UtxoRef { txid: benchmark_block_hash(seed as u8), output_index: seed }
}

fn benchmark_utxo_value(utxo_id: UtxoId, satoshis: Satoshis) -> UtxoValue {
	UtxoValue {
		utxo_id,
		script_pubkey: benchmark_script_pubkey(utxo_id as u32),
		satoshis,
		submitted_at_height: 1,
		watch_for_spent_until_height: 100,
	}
}

fn seed_pending_funding<T: Config>(
	utxo_id: UtxoId,
	satoshis: Satoshis,
) -> Result<(), BenchmarkError> {
	seed_pending_funding_at_height::<T>(utxo_id, satoshis, 1)
}

fn seed_pending_funding_at_height<T: Config>(
	utxo_id: UtxoId,
	satoshis: Satoshis,
	submitted_at_height: BitcoinHeight,
) -> Result<(), BenchmarkError> {
	LocksPendingFunding::<T>::try_mutate(|pending| {
		let mut utxo_value = benchmark_utxo_value(utxo_id, satoshis);
		utxo_value.submitted_at_height = submitted_at_height;
		pending
			.try_insert(utxo_id, utxo_value)
			.map_err(|_| BenchmarkError::Stop("pending funding storage full"))?;
		Ok::<(), BenchmarkError>(())
	})?;
	Ok(())
}

fn seed_candidates<T: Config>(
	utxo_id: UtxoId,
	count: u32,
	selected_ref: Option<&UtxoRef>,
) -> Result<(), BenchmarkError> {
	let satoshis = benchmark_satoshis::<T>();
	CandidateUtxoRefsByUtxoId::<T>::try_mutate(utxo_id, |refs| {
		for i in 0..count {
			let seed_base = 30_000u32.saturating_add((utxo_id as u32).saturating_mul(1_000));
			let utxo_ref = if i == 0 {
				selected_ref
					.cloned()
					.unwrap_or_else(|| benchmark_utxo_ref(seed_base.saturating_add(i)))
			} else {
				benchmark_utxo_ref(seed_base.saturating_add(i))
			};
			refs.try_insert(utxo_ref, satoshis)
				.map_err(|_| BenchmarkError::Stop("candidate storage full"))?;
		}
		Ok(())
	})
}
