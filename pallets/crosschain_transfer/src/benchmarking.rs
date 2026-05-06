#![cfg(feature = "runtime-benchmarks")]

use super::*;
#[allow(unused_imports)]
use crate::Pallet as CrosschainTransferPallet;
use alloy_primitives::keccak256;
use argon_primitives::{
	ethereum::{EthereumExecutionBlockProof, EthereumReceiptProof},
	EthereumLog, EthereumProof, UniswapTransferProvider,
};
use polkadot_sdk::{
	frame_benchmarking::v2::*, frame_support::traits::fungible::Mutate, sp_core::H256,
};

#[benchmarks(where T::AccountId: From<[u8; 32]>, [u8; 32]: From<T::AccountId>)]
mod benchmarks {
	use super::*;
	use frame_system::RawOrigin;

	#[benchmark]
	fn set_chain_config() -> Result<(), BenchmarkError> {
		ChainConfigBySourceChain::<T>::insert(SourceChain::Ethereum, benchmark_chain_config(0x20));

		#[extrinsic_call]
		set_chain_config(RawOrigin::Root, benchmark_chain_config(0x21));

		assert_eq!(
			ChainConfigBySourceChain::<T>::get(SourceChain::Ethereum),
			Some(benchmark_chain_config(0x21)),
		);
		Ok(())
	}

	#[benchmark]
	fn prove_transfer() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let recipient = caller.clone();
		let burn_account = CrosschainTransferPallet::<T>::burn_account(SourceChain::Ethereum);
		let amount: T::Balance = 1_000_000_000u128.into();

		ChainConfigBySourceChain::<T>::insert(SourceChain::Ethereum, benchmark_chain_config(0x21));
		T::NativeCurrency::mint_into(&recipient, amount)
			.map_err(|_| BenchmarkError::Stop("failed to seed benchmark recipient"))?;
		T::NativeCurrency::mint_into(&burn_account, 10_000_000_000u128.into())
			.map_err(|_| BenchmarkError::Stop("failed to fund benchmark burn account"))?;

		let proof = TransferProof::Ethereum {
			source_chain: SourceChain::Ethereum,
			event_log: burn_for_transfer_log(
				h160(0x21),
				h160(0x11),
				h160(0x31),
				amount.into(),
				<[u8; 32]>::from(recipient),
				1,
			),
			proof: dummy_proof(),
		};

		#[extrinsic_call]
		prove_transfer(RawOrigin::Signed(caller), proof);

		assert_eq!(NonceBySourceAccount::<T>::get((SourceChain::Ethereum, h160(0x11))), Some(1));
		Ok(())
	}

	#[benchmark]
	fn on_initialize_cleanup(e: Linear<1, 1_000>) -> Result<(), BenchmarkError> {
		let current_tick = T::CurrentTick::get();

		for index in 0..e {
			let account_id: T::AccountId = account("expiring-crosschain-account", index, 0);
			RecentArgonTransfersByAccount::<T>::insert(&account_id, 1);
			InboundTransfersExpiringAt::<T>::append(current_tick, account_id);
		}

		#[block]
		{
			CrosschainTransferPallet::<T>::on_initialize(frame_system::Pallet::<T>::block_number());
		}

		assert!(InboundTransfersExpiringAt::<T>::get(current_tick).is_empty());
		assert_eq!(LastTransferExpiryCleanupTick::<T>::get(), current_tick);
		Ok(())
	}

	#[benchmark]
	fn provider_is_crosschain_activated() {
		ChainConfigBySourceChain::<T>::insert(SourceChain::Ethereum, benchmark_chain_config(0x21));

		#[block]
		{
			assert!(<CrosschainTransferPallet<T> as UniswapTransferProvider<T::AccountId>>::is_crosschain_activated());
		}
	}

	#[benchmark]
	fn provider_has_recent_argon_transfer() {
		let account_id: T::AccountId = account("recent-crosschain-transfer-account", 0, 0);
		RecentArgonTransfersByAccount::<T>::insert(&account_id, 1);

		#[block]
		{
			assert!(<CrosschainTransferPallet<T> as UniswapTransferProvider<T::AccountId>>::has_recent_argon_transfer(&account_id));
		}
	}

	impl_benchmark_test_suite!(
		CrosschainTransferPallet,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);
}

fn benchmark_chain_config(gateway_byte: u8) -> ChainConfig {
	ChainConfig::Ethereum {
		gateway: h160(gateway_byte),
		argon_token: h160(0x31),
		argonot_token: h160(0x32),
		previous_gateway: None,
		previous_release_expiration: None,
	}
}

fn burn_for_transfer_log(
	gateway: H160,
	from: H160,
	token: H160,
	amount: u128,
	destination: [u8; 32],
	account_nonce: u64,
) -> EthereumLog {
	let mut data = Vec::with_capacity(96);
	data.extend_from_slice(&u256_word(amount));
	data.extend_from_slice(&destination);
	data.extend_from_slice(&u64_word(account_nonce));

	EthereumLog {
		address: gateway,
		topics: vec![
			H256::from_slice(keccak256(BURN_FOR_TRANSFER_EVENT_SIGNATURE).as_slice()),
			indexed_address_word(from),
			indexed_address_word(token),
		],
		data,
	}
}

fn indexed_address_word(address: H160) -> H256 {
	let mut bytes = [0u8; 32];
	bytes[12..].copy_from_slice(address.as_bytes());
	H256::from(bytes)
}

fn u256_word(value: u128) -> [u8; 32] {
	let mut bytes = [0u8; 32];
	bytes[16..].copy_from_slice(&value.to_be_bytes());
	bytes
}

fn u64_word(value: u64) -> [u8; 32] {
	let mut bytes = [0u8; 32];
	bytes[24..].copy_from_slice(&value.to_be_bytes());
	bytes
}

fn dummy_proof() -> EthereumProof {
	EthereumProof {
		execution_block_proof: EthereumExecutionBlockProof {
			anchor_block_hash: H256::repeat_byte(1),
			target_to_anchor_header_chain: Vec::new(),
		},
		receipt_proof: EthereumReceiptProof { transaction_index: 0, nodes: vec![vec![1u8]] },
	}
}

fn h160(byte: u8) -> H160 {
	H160::repeat_byte(byte)
}
