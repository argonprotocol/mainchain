use crate::{InboundEvmTransfer, InboundEvmTransfers, InboundTransfersExpiringAt};
use alloy_primitives::U256;
use alloy_sol_types::SolValue;
use codec::Encode;
use ismp::{host::StateMachine, router::PostRequest};
use pallet_prelude::*;
use pallet_token_gateway::types::Body;
use sp_core::{H160, H256};
use sp_io::hashing::blake2_256;
use sp_runtime::AccountId32;

use crate::mock::{
	InboundTransferLog as InboundTransferLogPallet, Test,
	gateway::{
		GatewayTest, InboundTransferLog as GatewayInboundTransferLog,
		new_test_ext as new_gateway_test_ext,
	},
	new_test_ext,
};
#[test]
fn test_on_initialize_removes_expired_transfers() {
	new_test_ext().execute_with(|| {
		let key = H256::repeat_byte(1);
		let transfer = InboundEvmTransfer::<Test> {
			source: ismp::host::StateMachine::Evm(1),
			nonce: 1,
			evm_from: H160::repeat_byte(2),
			to: 10,
			asset_kind: crate::AssetKind::Argon,
			amount: 1_000,
			expires_at_block: 1,
		};

		InboundEvmTransfers::<Test>::insert(key, transfer);
		InboundTransfersExpiringAt::<Test>::try_mutate(1, |keys| keys.try_push(key).map(|_| ()))
			.unwrap();

		let cleanup_block = 1u32.into();
		InboundTransferLogPallet::on_initialize(cleanup_block);

		assert!(!InboundEvmTransfers::<Test>::contains_key(key));
		assert!(InboundTransfersExpiringAt::<Test>::get(1).is_empty());
	});
}

#[test]
fn test_on_token_gateway_request_ignores_invalid_body() {
	new_gateway_test_ext().execute_with(|| {
		initialize_gateway_block();
		let asset_id = H256::repeat_byte(7);
		let local_asset_id = 0u32;
		pallet_token_gateway::LocalAssets::<GatewayTest>::insert(asset_id, local_asset_id);
		pallet_token_gateway::Precisions::<GatewayTest>::insert(
			local_asset_id,
			StateMachine::Evm(1),
			6,
		);

		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Substrate(*b"tstt"),
			nonce: 99,
			from: b"token-gateway".to_vec(),
			to: b"token-gateway".to_vec(),
			timeout_timestamp: 0,
			body: vec![0u8, 0xFF, 0xAA],
		};

		GatewayInboundTransferLog::on_token_gateway_request(&post);

		let record_key = H256::from(blake2_256(&(StateMachine::Evm(1), 99u64).encode()));
		assert!(!InboundEvmTransfers::<GatewayTest>::contains_key(record_key));
		assert_drop_event_emitted(
			StateMachine::Evm(1),
			99,
			crate::InboundTransferDropReason::AbiDecodeFailed,
		);
	});
}

#[test]
fn test_on_token_gateway_request_continues_processing_large_body() {
	new_gateway_test_ext().execute_with(|| {
		initialize_gateway_block();
		let max_len = crate::mock::MaxInboundTransferBytes::get() as usize;
		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Substrate(*b"tstt"),
			nonce: 77,
			from: b"token-gateway".to_vec(),
			to: b"token-gateway".to_vec(),
			timeout_timestamp: 0,
			body: vec![0u8; max_len.saturating_add(1)],
		};

		GatewayInboundTransferLog::on_token_gateway_request(&post);

		let record_key = H256::from(blake2_256(&(StateMachine::Evm(1), 77u64).encode()));
		assert!(!InboundEvmTransfers::<GatewayTest>::contains_key(record_key));
		assert_drop_event_emitted(
			StateMachine::Evm(1),
			77,
			crate::InboundTransferDropReason::UnknownAsset,
		);
	});
}

#[test]
fn test_on_token_gateway_request_emits_non_evm_source_drop_event() {
	new_gateway_test_ext().execute_with(|| {
		initialize_gateway_block();
		let asset_id = H256::repeat_byte(7);
		let local_asset_id = 0u32;
		pallet_token_gateway::LocalAssets::<GatewayTest>::insert(asset_id, local_asset_id);
		pallet_token_gateway::Precisions::<GatewayTest>::insert(
			local_asset_id,
			StateMachine::Substrate(*b"eth0"),
			6,
		);

		let evm_from = H160::repeat_byte(0x11);
		let mut from_bytes = [0u8; 32];
		from_bytes[12..].copy_from_slice(evm_from.as_bytes());
		let to = AccountId32::from([0x22; 32]);
		let to_bytes: [u8; 32] = *to.as_ref();
		let body = Body {
			amount: U256::from(1_500_000u128),
			asset_id: asset_id.0.into(),
			redeem: false,
			from: from_bytes.into(),
			to: to_bytes.into(),
		};
		let mut encoded = vec![0u8];
		encoded.extend_from_slice(&Body::abi_encode(&body));

		let source = StateMachine::Substrate(*b"eth0");
		let post = PostRequest {
			source,
			dest: StateMachine::Substrate(*b"tstt"),
			nonce: 55,
			from: b"token-gateway".to_vec(),
			to: b"token-gateway".to_vec(),
			timeout_timestamp: 0,
			body: encoded,
		};

		GatewayInboundTransferLog::on_token_gateway_request(&post);

		let record_key = H256::from(blake2_256(&(source, 55u64).encode()));
		assert!(!InboundEvmTransfers::<GatewayTest>::contains_key(record_key));
		assert_drop_event_emitted(source, 55, crate::InboundTransferDropReason::NonEvmSource);
	});
}

#[test]
fn test_on_token_gateway_request_emits_unknown_asset_drop_event() {
	new_gateway_test_ext().execute_with(|| {
		initialize_gateway_block();
		let asset_id = H256::repeat_byte(8);
		let evm_from = H160::repeat_byte(0x11);
		let mut from_bytes = [0u8; 32];
		from_bytes[12..].copy_from_slice(evm_from.as_bytes());
		let to = AccountId32::from([0x33; 32]);
		let to_bytes: [u8; 32] = *to.as_ref();
		let body = Body {
			amount: U256::from(1_500_000u128),
			asset_id: asset_id.0.into(),
			redeem: false,
			from: from_bytes.into(),
			to: to_bytes.into(),
		};
		let mut encoded = vec![0u8];
		encoded.extend_from_slice(&Body::abi_encode(&body));

		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Substrate(*b"tstt"),
			nonce: 56,
			from: b"token-gateway".to_vec(),
			to: b"token-gateway".to_vec(),
			timeout_timestamp: 0,
			body: encoded,
		};

		GatewayInboundTransferLog::on_token_gateway_request(&post);

		let record_key = H256::from(blake2_256(&(StateMachine::Evm(1), 56u64).encode()));
		assert!(!InboundEvmTransfers::<GatewayTest>::contains_key(record_key));
		assert_drop_event_emitted(
			StateMachine::Evm(1),
			56,
			crate::InboundTransferDropReason::UnknownAsset,
		);
	});
}

#[test]
fn test_on_token_gateway_request_emits_unsupported_asset_drop_event() {
	new_gateway_test_ext().execute_with(|| {
		initialize_gateway_block();
		let asset_id = H256::repeat_byte(9);
		let unsupported_asset_id = 77u32;
		pallet_token_gateway::LocalAssets::<GatewayTest>::insert(asset_id, unsupported_asset_id);

		let evm_from = H160::repeat_byte(0x44);
		let mut from_bytes = [0u8; 32];
		from_bytes[12..].copy_from_slice(evm_from.as_bytes());
		let to = AccountId32::from([0x55; 32]);
		let to_bytes: [u8; 32] = *to.as_ref();
		let body = Body {
			amount: U256::from(1_500_000u128),
			asset_id: asset_id.0.into(),
			redeem: false,
			from: from_bytes.into(),
			to: to_bytes.into(),
		};
		let mut encoded = vec![0u8];
		encoded.extend_from_slice(&Body::abi_encode(&body));

		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Substrate(*b"tstt"),
			nonce: 57,
			from: b"token-gateway".to_vec(),
			to: b"token-gateway".to_vec(),
			timeout_timestamp: 0,
			body: encoded,
		};

		GatewayInboundTransferLog::on_token_gateway_request(&post);

		let record_key = H256::from(blake2_256(&(StateMachine::Evm(1), 57u64).encode()));
		assert!(!InboundEvmTransfers::<GatewayTest>::contains_key(record_key));
		assert_drop_event_emitted(
			StateMachine::Evm(1),
			57,
			crate::InboundTransferDropReason::UnsupportedAsset,
		);
	});
}

#[test]
fn test_on_initialize_keeps_future_transfers() {
	new_test_ext().execute_with(|| {
		let key = H256::repeat_byte(3);
		let transfer = InboundEvmTransfer::<Test> {
			source: ismp::host::StateMachine::Evm(1),
			nonce: 2,
			evm_from: H160::repeat_byte(4),
			to: 11,
			asset_kind: crate::AssetKind::Argon,
			amount: 1_000,
			expires_at_block: 2,
		};

		InboundEvmTransfers::<Test>::insert(key, transfer);
		InboundTransfersExpiringAt::<Test>::try_mutate(2, |keys| keys.try_push(key).map(|_| ()))
			.unwrap();

		let cleanup_block = 1u32.into();
		InboundTransferLogPallet::on_initialize(cleanup_block);

		assert!(InboundEvmTransfers::<Test>::contains_key(key));
		assert_eq!(InboundTransfersExpiringAt::<Test>::get(2).len(), 1);
	});
}

#[test]
fn test_on_token_gateway_request_records_transfer() {
	new_gateway_test_ext().execute_with(|| {
		initialize_gateway_block();
		let asset_id = H256::repeat_byte(7);
		let local_asset_id = 0u32;
		pallet_token_gateway::LocalAssets::<GatewayTest>::insert(asset_id, local_asset_id);
		pallet_token_gateway::Precisions::<GatewayTest>::insert(
			local_asset_id,
			StateMachine::Evm(1),
			6,
		);

		let evm_from = H160::repeat_byte(0x11);
		let mut from_bytes = [0u8; 32];
		from_bytes[12..].copy_from_slice(evm_from.as_bytes());

		let to = AccountId32::from([0x22; 32]);
		let to_bytes: [u8; 32] = *to.as_ref();

		let body = Body {
			amount: U256::from(1_500_000u128),
			asset_id: asset_id.0.into(),
			redeem: false,
			from: from_bytes.into(),
			to: to_bytes.into(),
		};

		let mut encoded = vec![0u8];
		encoded.extend_from_slice(&Body::abi_encode(&body));

		let post = PostRequest {
			source: StateMachine::Evm(1),
			dest: StateMachine::Substrate(*b"tstt"),
			nonce: 42,
			from: b"token-gateway".to_vec(),
			to: b"token-gateway".to_vec(),
			timeout_timestamp: 0,
			body: encoded,
		};

		GatewayInboundTransferLog::on_token_gateway_request(&post);

		let record_key = H256::from(blake2_256(&(StateMachine::Evm(1), 42u64).encode()));
		let record = InboundEvmTransfers::<GatewayTest>::get(record_key).expect("record");

		assert_eq!(record.evm_from, evm_from);
		assert_eq!(record.to, to);
		assert_eq!(record.asset_kind, crate::AssetKind::Argon);
		assert_eq!(record.amount, 1_500_000);
		let retention_blocks: BlockNumberFor<GatewayTest> =
			crate::mock::gateway::GatewayInboundTransfersRetentionBlocks::get();
		assert_eq!(
			record.expires_at_block,
			frame_system::Pallet::<GatewayTest>::block_number() + retention_blocks
		);
	});
}

fn assert_drop_event_emitted(
	source: StateMachine,
	nonce: u64,
	reason: crate::InboundTransferDropReason,
) {
	let events = frame_system::Pallet::<GatewayTest>::events();
	let has_event = events.iter().any(|record| {
		matches!(
			&record.event,
			crate::mock::gateway::RuntimeEvent::InboundTransferLog(
				crate::Event::InboundEvmTransferDropped {
					source: event_source,
					nonce: event_nonce,
					reason: event_reason,
				},
			) if event_source == &source && event_nonce == &nonce && event_reason == &reason
		)
	});
	assert!(
		has_event,
		"expected drop event not found, events: {:?}",
		events.iter().map(|record| &record.event).collect::<Vec<_>>()
	);
}

fn initialize_gateway_block() {
	frame_system::Pallet::<GatewayTest>::set_block_number(1u32.into());
}
