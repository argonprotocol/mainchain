#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_primitives::{
	bitcoin::{
		BitcoinBlock, BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinLockId, H256Le, Satoshis,
		UtxoAddress, UtxoRef,
	},
	inherents::BitcoinUtxoSyncV2,
	BitcoinUtxoTracker,
};
use frame_benchmarking::v2::*;
use frame_support::dispatch::CheckIfFeeless;

#[benchmarks]
mod benchmarks {
	use super::*;
	use frame_system::RawOrigin;

	#[benchmark]
	fn provider_watch_for_utxo() -> Result<(), BenchmarkError> {
		let lock_id = 1;
		let script_pubkey = benchmark_script_pubkey(lock_id as u32);
		let tip = BitcoinBlock { block_height: 1_000, block_hash: benchmark_block_hash(1) };
		ConfirmedBitcoinBlockTip::<T>::put(&tip);

		#[block]
		{
			<Pallet<T> as BitcoinUtxoTracker>::watch_for_utxo(lock_id, script_pubkey)
				.map_err(|_| BenchmarkError::Stop("watch registration failed"))?;
		}

		assert_eq!(LockIdByScriptPubkey::<T>::get(script_pubkey), Some(lock_id));
		assert_eq!(
			UtxoAddressByLockId::<T>::get(lock_id),
			Some(UtxoAddress { lock_id, script_pubkey, submitted_at_height: tip.block_height })
		);
		Ok(())
	}

	#[benchmark]
	fn provider_unwatch_utxo() -> Result<(), BenchmarkError> {
		let lock_id = 1;
		for index in 0..T::MaxUtxosPerLock::get() {
			UtxoRefsByLockId::<T>::try_mutate(lock_id, |refs| {
				refs.try_insert(benchmark_utxo_ref(u32::MAX - index))
			})
			.map_err(|_| BenchmarkError::Stop("UTXO refs full"))?;
		}
		let utxo_ref = benchmark_utxo_ref(u32::MAX);

		#[block]
		{
			<Pallet<T> as BitcoinUtxoTracker>::unwatch_utxo(lock_id, &utxo_ref);
		}

		let refs = UtxoRefsByLockId::<T>::get(lock_id);
		assert!(!refs.contains(&utxo_ref));
		assert_eq!(refs.len(), T::MaxUtxosPerLock::get() as usize - 1);
		Ok(())
	}

	#[benchmark]
	fn provider_unwatch() -> Result<(), BenchmarkError> {
		let lock_id = 1;
		let address = benchmark_utxo_value(lock_id, benchmark_satoshis::<T>());
		UtxoAddressByLockId::<T>::insert(lock_id, &address);
		LockIdByScriptPubkey::<T>::insert(address.script_pubkey, lock_id);
		for index in 0..T::MaxUtxosPerLock::get() {
			UtxoRefsByLockId::<T>::try_mutate(lock_id, |refs| {
				refs.try_insert(benchmark_utxo_ref(u32::MAX - index))
			})
			.map_err(|_| BenchmarkError::Stop("UTXO refs full"))?;
		}

		#[block]
		{
			<Pallet<T> as BitcoinUtxoTracker>::unwatch(lock_id);
		}

		assert!(!UtxoAddressByLockId::<T>::contains_key(lock_id));
		assert!(!LockIdByScriptPubkey::<T>::contains_key(address.script_pubkey));
		assert!(!UtxoRefsByLockId::<T>::contains_key(lock_id));
		Ok(())
	}

	#[benchmark]
	fn provider_is_feeless() {
		let caller: T::AccountId = whitelisted_caller();
		OracleOperatorAccount::<T>::put(&caller);
		let call = Call::<T>::set_confirmed_block {
			bitcoin_height: 1,
			bitcoin_block_hash: benchmark_block_hash(1),
		};
		let origin = RawOrigin::Signed(caller).into();

		#[block]
		{
			assert!(core::hint::black_box(&call).is_feeless(&origin));
		}
	}
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
		let utxo_sync = BitcoinUtxoSyncV2 {
			spent: vec![],
			funded: vec![],
			sync_to_block: sync_to_block.clone(),
		};

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
		SynchedBitcoinBlock::<T>::put(sync_to_block.clone());
		PreviousBitcoinBlockTip::<T>::kill();
		TempParentHasSyncState::<T>::put(false);
		#[block]
		{
			Pallet::<T>::on_initialize(frame_system::Pallet::<T>::block_number());
		}

		assert_eq!(PreviousBitcoinBlockTip::<T>::get(), Some(sync_to_block));
		assert!(TempParentHasSyncState::<T>::get());
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
	fn utxo_spent(n: Linear<1, MAX_SYNC_ITEMS>) -> Result<(), BenchmarkError> {
		let satoshis = benchmark_satoshis::<T>();
		let block_height: BitcoinHeight = 10;
		for i in 0..n {
			let lock_id = i.saturating_add(1) as BitcoinLockId;
			let funding_ref = benchmark_utxo_ref(10_000 + i);
			UtxoAddressByLockId::<T>::insert(lock_id, benchmark_utxo_value(lock_id, satoshis));
			UtxoRefsByLockId::<T>::try_mutate(lock_id, |refs| refs.try_insert(funding_ref.clone()))
				.map_err(|_| BenchmarkError::Stop("UTXO refs full"))?;
		}

		#[block]
		{
			for i in 0..n {
				let lock_id = i.saturating_add(1) as BitcoinLockId;
				let funding_ref = benchmark_utxo_ref(10_000 + i);
				Pallet::<T>::utxo_spent(lock_id, Some(funding_ref), block_height)
					.map_err(|_| BenchmarkError::Stop("utxo spent failed"))?;
			}
		}

		for i in 0..n {
			let lock_id = i.saturating_add(1) as BitcoinLockId;
			// The event handler owns retention cleanup and is stubbed in runtime benchmarks.
			frame_system::Pallet::<T>::assert_has_event(
				Event::<T>::UtxoSpent {
					lock_id,
					utxo_ref: benchmark_utxo_ref(10_000 + i),
					block_height,
				}
				.into(),
			);
		}
		Ok(())
	}

	#[benchmark]
	fn lock_verified(n: Linear<1, MAX_SYNC_ITEMS>) -> Result<(), BenchmarkError> {
		let satoshis = benchmark_satoshis::<T>();
		let bitcoin_height: BitcoinHeight = 10;
		for i in 0..n {
			let lock_id = i.saturating_add(1) as BitcoinLockId;
			let address = benchmark_utxo_value(lock_id, satoshis);
			LockIdByScriptPubkey::<T>::insert(&address.script_pubkey, lock_id);
			UtxoAddressByLockId::<T>::insert(lock_id, address);
		}

		#[block]
		{
			for i in 0..n {
				let lock_id = i.saturating_add(1) as BitcoinLockId;
				let verified_ref = benchmark_utxo_ref(20_000 + i);
				Pallet::<T>::utxo_detected(lock_id, verified_ref, satoshis, bitcoin_height)
					.map_err(|_| BenchmarkError::Stop("lock verification failed"))?;
			}
		}

		for i in 0..n {
			let lock_id = i.saturating_add(1) as BitcoinLockId;
			assert_eq!(UtxoRefsByLockId::<T>::get(lock_id).len(), 1);
		}
		Ok(())
	}
}

fn benchmark_satoshis<T: Config>() -> Satoshis {
	T::MinimumSatoshisPerUtxo::get().saturating_add(1_000)
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

fn benchmark_utxo_value(lock_id: BitcoinLockId, _satoshis: Satoshis) -> UtxoAddress {
	UtxoAddress {
		lock_id,
		script_pubkey: benchmark_script_pubkey(lock_id as u32),
		submitted_at_height: 1,
	}
}
