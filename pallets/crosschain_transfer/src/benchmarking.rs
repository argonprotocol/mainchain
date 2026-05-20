#![cfg(feature = "runtime-benchmarks")]

use super::*;
#[allow(unused_imports)]
use crate::Pallet as CrosschainTransferPallet;
use alloy_primitives::keccak256;
use argon_primitives::{
	ethereum::{
		EthereumCombinedReceiptProof, EthereumExecutionBlockProof, EthereumReceiptProofReceipt,
	},
	EthereumLog, EthereumReceiptLog, UniswapTransferProvider,
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
	fn prove_gateway_activity(
		b: Linear<1, { T::MaxReceiptProofsPerExtrinsic::get() }>,
		e: Linear<0, { T::MaxActivitiesPerReceiptProof::get() - 1 }>,
	) -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let burn_account = CrosschainTransferPallet::<T>::burn_account(SourceChain::Ethereum);
		let amount: T::Balance = 1_000_000_000u128.into();
		let burn_funding: T::Balance = u128::from(b.saturating_add(e).saturating_add(1))
			.saturating_mul(1_000_000_000)
			.into();

		ChainConfigBySourceChain::<T>::insert(SourceChain::Ethereum, benchmark_chain_config(0x21));
		T::NativeCurrency::mint_into(&burn_account, burn_funding)
			.map_err(|_| BenchmarkError::Stop("failed to fund benchmark burn account"))?;
		let mut next_gateway_activity_nonce = 1u64;
		let proof_blocks = (0..b)
			.map(|block_index| {
				let logs_in_block = if block_index == 0 { e.saturating_add(1) } else { 1 };
				let receipt_logs = (0..logs_in_block)
					.map(|log_index| {
						let recipient: T::AccountId = account(
							"crosschain-transfer-recipient",
							block_index.saturating_mul(T::MaxActivitiesPerReceiptProof::get()) +
								log_index,
							0,
						);
						let receipt_log = EthereumReceiptLog {
							transaction_index: u64::from(log_index),
							event_log: transfer_to_argon_started_log(
								h160(0x21),
								h160(0x11),
								h160(0x31),
								amount.into(),
								<[u8; 32]>::from(recipient),
								next_gateway_activity_nonce,
								0,
							),
						};
						next_gateway_activity_nonce = next_gateway_activity_nonce.saturating_add(1);
						receipt_log
					})
					.collect::<Vec<_>>()
					.try_into()
					.map_err(|_| {
						BenchmarkError::Stop("benchmark receipt logs exceeded pallet bound")
					})?;

				Ok(GatewayActivityProofBlock::<T> {
					target_block_number: u64::from(block_index),
					receipt_proof: dummy_receipt_proof(logs_in_block),
					receipt_logs,
				})
			})
			.collect::<Result<Vec<_>, BenchmarkError>>()?;
		let proof_batch = GatewayActivityProofBatch::<T> {
			execution_block_proof: dummy_execution_block_proof(),
			blocks: proof_blocks.try_into().map_err(|_| {
				BenchmarkError::Stop("benchmark proof blocks exceeded pallet bound")
			})?,
		};

		#[extrinsic_call]
		prove_gateway_activity(RawOrigin::Signed(caller), SourceChain::Ethereum, 0, proof_batch);

		assert_eq!(
			GatewayStateBySourceChain::<T>::get(SourceChain::Ethereum)
				.expect("gateway state should be written")
				.gateway_activity_nonce,
			u64::from(b).saturating_add(u64::from(e))
		);
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
	}
}

fn transfer_to_argon_started_log(
	gateway: H160,
	from: H160,
	token: H160,
	amount: u128,
	destination: [u8; 32],
	gateway_activity_nonce: u64,
	argon_approvals_nonce: u64,
) -> EthereumLog {
	let mut data = Vec::with_capacity(192);
	data.extend_from_slice(&u64_word(amount as u64));
	data.extend_from_slice(&destination);
	data.extend_from_slice(&u64_word(gateway_activity_nonce));
	data.extend_from_slice(&u64_word(argon_approvals_nonce));
	data.extend_from_slice(&u64_word(0));
	data.extend_from_slice(&u64_word(0));

	EthereumLog {
		address: gateway,
		topics: vec![
			H256::from_slice(keccak256(TRANSFER_TO_ARGON_STARTED_EVENT_SIGNATURE).as_slice()),
			indexed_address_word(from),
			indexed_address_word(token),
		]
		.try_into()
		.expect("topics stay within Ethereum log topic bounds"),
		data: data.try_into().expect("burn event data stays within bounded log payload"),
	}
}

fn indexed_address_word(address: H160) -> H256 {
	let mut bytes = [0u8; 32];
	bytes[12..].copy_from_slice(address.as_bytes());
	H256::from(bytes)
}

fn u64_word(value: u64) -> [u8; 32] {
	let mut bytes = [0u8; 32];
	bytes[24..].copy_from_slice(&value.to_be_bytes());
	bytes
}

fn dummy_execution_block_proof() -> EthereumExecutionBlockProof {
	EthereumExecutionBlockProof {
		anchor_block_hash: H256::repeat_byte(1),
		target_to_anchor_header_chain: Vec::new()
			.try_into()
			.expect("empty header chain stays within bounds"),
	}
}

fn dummy_receipt_proof(receipt_count: u32) -> EthereumCombinedReceiptProof {
	EthereumCombinedReceiptProof {
		nodes: vec![vec![1u8].try_into().expect("tiny receipt proof node stays within bounds")]
			.try_into()
			.expect("single-node receipt proof stays within bounds"),
		receipts: (0..receipt_count)
			.map(|transaction_index| EthereumReceiptProofReceipt {
				transaction_index: u64::from(transaction_index),
				node_indexes: vec![0u16]
					.try_into()
					.expect("single node index stays within bounded receipt proof refs"),
			})
			.collect::<Vec<_>>()
			.try_into()
			.expect("benchmark receipt proofs stay within bounded receipt count"),
	}
}

fn h160(byte: u8) -> H160 {
	H160::repeat_byte(byte)
}
