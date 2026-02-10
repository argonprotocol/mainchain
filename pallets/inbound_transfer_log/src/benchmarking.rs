//! Benchmarking setup for pallet-inbound-transfer-log
#![cfg(feature = "runtime-benchmarks")]
use super::*;

use crate::Pallet as InboundTransferLogPallet;
use frame_support::traits::{Get, Hooks};
use ismp::{host::StateMachine, router::PostRequest};
use polkadot_sdk::frame_benchmarking::v2::*;
use sp_runtime::traits::{One, SaturatedConversion, Saturating};

#[benchmarks(
	where
		T: pallet_token_gateway::Config,
		T::AccountId: From<[u8; 32]>,
		TokenGatewayAssetId<T>: From<u32>,
)]
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

	#[benchmark]
	fn on_token_gateway_request_recorded() {
		let source = StateMachine::Evm(1);
		let asset_id = H256::repeat_byte(7);
		pallet_token_gateway::LocalAssets::<T>::insert(asset_id, T::NativeAssetId::get());
		pallet_token_gateway::Precisions::<T>::insert(
			T::NativeAssetId::get(),
			source,
			T::Decimals::get(),
		);
		let amount = T::MinimumTransferMicrogonsToRecord::get().saturating_add(1);
		let request = valid_token_gateway_request(asset_id, source, 42, amount);

		#[block]
		{
			InboundTransferLogPallet::<T>::on_token_gateway_request(&request);
		}

		let record_key = H256::from(sp_io::hashing::blake2_256(&(source, 42u64).encode()));
		assert!(InboundEvmTransfers::<T>::contains_key(record_key));
	}

	#[benchmark]
	fn on_token_gateway_request_dropped() {
		let source = StateMachine::Evm(1);
		let asset_id = H256::repeat_byte(8);
		pallet_token_gateway::LocalAssets::<T>::insert(asset_id, T::NativeAssetId::get());
		// Intentionally omit `Precisions` entry so request handling hits the
		// `UnknownPrecision` drop branch.
		let amount = T::MinimumTransferMicrogonsToRecord::get().saturating_add(1);
		let request = valid_token_gateway_request(asset_id, source, 43, amount);

		#[block]
		{
			InboundTransferLogPallet::<T>::on_token_gateway_request(&request);
		}

		let record_key = H256::from(sp_io::hashing::blake2_256(&(source, 43u64).encode()));
		assert!(!InboundEvmTransfers::<T>::contains_key(record_key));
	}

	fn valid_token_gateway_request(
		asset_id: H256,
		source: StateMachine,
		nonce: u64,
		amount: Balance,
	) -> PostRequest {
		let evm_from = H160::repeat_byte(0x11);
		let mut from_bytes = [0u8; 32];
		from_bytes[12..].copy_from_slice(evm_from.as_bytes());
		let to_bytes = [0x22; 32];
		let mut encoded = vec![0u8];
		encoded.extend_from_slice(&u256_word(amount));
		encoded.extend_from_slice(&asset_id.0);
		encoded.extend_from_slice(&[0u8; 32]);
		encoded.extend_from_slice(&from_bytes);
		encoded.extend_from_slice(&to_bytes);

		PostRequest {
			source,
			dest: StateMachine::Substrate(*b"tstt"),
			nonce,
			from: b"token-gateway".to_vec(),
			to: b"token-gateway".to_vec(),
			timeout_timestamp: 0,
			body: encoded,
		}
	}

	fn u256_word(value: u128) -> [u8; 32] {
		let mut word = [0u8; 32];
		word[16..].copy_from_slice(&value.to_be_bytes());
		word
	}

	impl_benchmark_test_suite!(
		InboundTransferLogPallet,
		crate::mock::gateway::new_test_ext(),
		crate::mock::gateway::GatewayTest
	);
}
