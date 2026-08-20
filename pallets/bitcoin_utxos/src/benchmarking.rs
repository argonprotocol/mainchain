#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_primitives::{
	bitcoin::{
		BitcoinBlock, BitcoinCosignScriptPubkey, BitcoinHeight, H256Le, Satoshis, UtxoAddress,
		UtxoId, UtxoRef,
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
			let utxo_id = i.saturating_add(1) as UtxoId;
			let funding_ref = benchmark_utxo_ref(10_000 + i);
			UtxoAddressByUtxoId::<T>::insert(utxo_id, benchmark_utxo_value(utxo_id, satoshis));
			UtxoRefsByUtxoId::<T>::try_mutate(utxo_id, |refs| refs.try_insert(funding_ref.clone()))
				.map_err(|_| BenchmarkError::Stop("UTXO refs full"))?;
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
			assert!(UtxoRefsByUtxoId::<T>::get(utxo_id).is_empty());
		}
		Ok(())
	}

	#[benchmark]
	fn lock_verified(n: Linear<1, MAX_SYNC_ITEMS>) -> Result<(), BenchmarkError> {
		let satoshis = benchmark_satoshis::<T>();
		let bitcoin_height: BitcoinHeight = 10;
		for i in 0..n {
			let utxo_id = i.saturating_add(1) as UtxoId;
			let address = benchmark_utxo_value(utxo_id, satoshis);
			UtxoIdByScriptPubkey::<T>::insert(&address.script_pubkey, utxo_id);
			UtxoAddressByUtxoId::<T>::insert(utxo_id, address);
		}

		#[block]
		{
			for i in 0..n {
				let utxo_id = i.saturating_add(1) as UtxoId;
				let verified_ref = benchmark_utxo_ref(20_000 + i);
				Pallet::<T>::utxo_detected(utxo_id, verified_ref, satoshis, bitcoin_height)
					.map_err(|_| BenchmarkError::Stop("lock verification failed"))?;
			}
		}

		for i in 0..n {
			let utxo_id = i.saturating_add(1) as UtxoId;
			assert_eq!(UtxoRefsByUtxoId::<T>::get(utxo_id).len(), 1);
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

fn benchmark_utxo_value(utxo_id: UtxoId, _satoshis: Satoshis) -> UtxoAddress {
	UtxoAddress {
		utxo_id,
		script_pubkey: benchmark_script_pubkey(utxo_id as u32),
		submitted_at_height: 1,
	}
}
