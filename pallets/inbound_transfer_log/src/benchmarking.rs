//! Benchmarking setup for pallet-inbound-transfer-log
#![cfg(feature = "runtime-benchmarks")]
use super::*;

use crate::Pallet as InboundTransferLogPallet;
use frame_support::traits::{Get, Hooks};
use polkadot_sdk::frame_benchmarking::v2::*;
use sp_runtime::traits::{One, SaturatedConversion, Saturating};

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn on_initialize_cleanup(c: Linear<1, 1_024>) {
		let retention: BlockNumberFor<T> =
			T::InboundTransfersRetentionBlocks::get().saturated_into();
		let expiry_block = BlockNumberFor::<T>::one();
		let cleanup_block = expiry_block.saturating_add(retention);
		let to: T::AccountId = account("to", 0, 0);
		let transfer = InboundEvmTransfer::<T> {
			source: ismp::host::StateMachine::Evm(1),
			nonce: 1,
			evm_from: H160::repeat_byte(1),
			to,
			asset_kind: AssetKind::Argon,
			amount: 1_000,
			expires_at_block: expiry_block,
		};
		let count = c.min(T::MaxTransfersToRetainPerBlock::get());
		for i in 0..count {
			let key = H256::repeat_byte(i as u8);
			InboundEvmTransfers::<T>::insert(key, transfer.clone());
			InboundTransfersExpiringAt::<T>::try_mutate(expiry_block, |keys| {
				keys.try_push(key).map_err(|_| ())?;
				Ok::<(), ()>(())
			})
			.ok();
		}

		#[block]
		{
			InboundTransferLogPallet::<T>::on_initialize(cleanup_block);
		}

		assert!(InboundEvmTransfers::<T>::iter().next().is_none());
	}

	impl_benchmark_test_suite!(
		InboundTransferLogPallet,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);
}
