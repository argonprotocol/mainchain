use crate::{
	migrations::InitializeCrosschainTransferMigration, AssetKind, BurnNotice, ChainConfig,
	ChainConfigBySourceChain, Event, InboundTransfersExpiringAt, NonceBySourceAccount,
	RecentArgonTransfersByAccount, SourceChain, TransferProof, BURN_FOR_TRANSFER_EVENT_SIGNATURE,
};
use alloy_primitives::keccak256;
use argon_primitives::{EthereumLog, EthereumProof};
use frame_support::{assert_noop, assert_ok, traits::OnRuntimeUpgrade};
use pallet_prelude::*;
use sp_core::crypto::Ss58Codec;
use sp_runtime::AccountId32;

use crate::mock::{
	account, h160, legacy_token_gateway_account, new_test_ext, Balances, ConfirmedTransfers,
	CrosschainTransfer, CurrentTick, Ownership, ProofVerificationAllowed, RuntimeEvent,
	RuntimeOrigin, System, Test,
};

#[test]
fn prove_transfer_pays_argon_and_marks_recent_transfer() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));
		assert_eq!(
			ChainConfigBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(chain_config()),
		);

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let recipient = account(2);
		let proof = argon_proof(recipient.clone(), 1, 1_250);

		assert_ok!(CrosschainTransfer::prove_transfer(RuntimeOrigin::signed(account(1)), proof));

		assert_eq!(Balances::balance(&recipient), 1_250);
		assert_eq!(NonceBySourceAccount::<Test>::get((SourceChain::Ethereum, h160(0x11))), Some(1),);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&recipient), 1);
		assert_eq!(ConfirmedTransfers::get(), vec![(recipient.clone(), 1_250)]);
		assert!(System::events().iter().any(|record| matches!(
			record.event,
			RuntimeEvent::CrosschainTransfer(Event::BurnNoticeAccepted {
				source_chain: SourceChain::Ethereum,
				ref notice,
			}) if notice == &BurnNotice::<Test> {
				from: h160(0x11),
				to: recipient.clone(),
				asset_kind: AssetKind::Argon,
				amount: 1_250,
				account_nonce: 1,
			}
		)));
	});
}

#[test]
fn prove_transfer_pays_argonot_from_burn_account() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Ownership::mint_into(&burn_account, 777));

		let recipient = account(3);
		let proof = argonot_proof(recipient.clone(), 1, 777);

		assert_ok!(CrosschainTransfer::prove_transfer(RuntimeOrigin::signed(account(1)), proof));

		assert_eq!(Ownership::balance(&recipient), 777);
		assert_eq!(Ownership::balance(&burn_account), 0);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&recipient), 0);
		assert!(ConfirmedTransfers::get().is_empty());
	});
}

#[test]
fn prove_transfer_rejects_unsupported_gateway() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let recipient = account(4);
		let proof = TransferProof::Ethereum {
			source_chain: SourceChain::Ethereum,
			event_log: burn_for_transfer_log(
				h160(0x44),
				h160(0x11),
				h160(0x21),
				5u128,
				destination_bytes(&recipient),
				1,
			),
			proof: dummy_proof(),
		};

		assert_noop!(
			CrosschainTransfer::prove_transfer(RuntimeOrigin::signed(account(1)), proof),
			crate::Error::<Test>::UnsupportedGateway,
		);
	});
}

#[test]
fn prove_transfer_rejects_out_of_order_nonce() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let recipient = account(5);
		let proof = argon_proof(recipient.clone(), 2, 10);

		assert_noop!(
			CrosschainTransfer::prove_transfer(RuntimeOrigin::signed(account(1)), proof),
			crate::Error::<Test>::UnexpectedNonce,
		);
	});
}

#[test]
fn previous_release_is_accepted_until_tick_expiry() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentTick::set(4);
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			chain_config_with_previous(5),
		));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let recipient = account(6);
		let previous_release_proof = TransferProof::Ethereum {
			source_chain: SourceChain::Ethereum,
			event_log: burn_for_transfer_log(
				h160(0x40),
				h160(0x11),
				h160(0x31),
				88u128,
				destination_bytes(&recipient),
				1,
			),
			proof: dummy_proof(),
		};

		assert_ok!(CrosschainTransfer::prove_transfer(
			RuntimeOrigin::signed(account(1)),
			previous_release_proof,
		));

		CurrentTick::set(6);
		let expired_previous_release_proof = TransferProof::Ethereum {
			source_chain: SourceChain::Ethereum,
			event_log: burn_for_transfer_log(
				h160(0x40),
				h160(0x11),
				h160(0x31),
				88u128,
				destination_bytes(&recipient),
				2,
			),
			proof: dummy_proof(),
		};

		assert_noop!(
			CrosschainTransfer::prove_transfer(
				RuntimeOrigin::signed(account(1)),
				expired_previous_release_proof,
			),
			crate::Error::<Test>::UnsupportedGateway,
		);
	});
}

#[test]
fn on_initialize_expires_recent_transfers() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentTick::set(1);
		CrosschainTransfer::on_initialize(System::block_number());
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let recipient = account(7);
		assert_ok!(CrosschainTransfer::prove_transfer(
			RuntimeOrigin::signed(account(1)),
			argon_proof(recipient.clone(), 1, 55),
		));

		let expires_at = CurrentTick::get() + crate::mock::RecentTransferRetentionTicks::get();
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&recipient), 1);
		assert_eq!(InboundTransfersExpiringAt::<Test>::get(expires_at).len(), 1);

		CurrentTick::set(expires_at + 2);
		System::set_block_number(2);
		CrosschainTransfer::on_initialize(System::block_number());

		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&recipient), 0);
		assert!(InboundTransfersExpiringAt::<Test>::get(expires_at).is_empty());
	});
}

#[test]
fn invalid_proof_from_provider_is_rejected() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		ProofVerificationAllowed::set(false);

		assert_noop!(
			CrosschainTransfer::prove_transfer(
				RuntimeOrigin::signed(account(1)),
				argon_proof(account(8), 1, 100),
			),
			crate::Error::<Test>::InvalidProof,
		);
	});
}

#[test]
fn migration_moves_legacy_balances_and_refunds_ready_cases() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let legacy_account = legacy_token_gateway_account();
		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);

		let _ = InitializeCrosschainTransferMigration::<Test>::on_runtime_upgrade();

		assert_eq!(Balances::balance(&legacy_account), 0);
		assert_eq!(Ownership::balance(&legacy_account), 0);

		assert_eq!(Balances::balance(&burn_account), 1_928_409);
		assert_eq!(Ownership::balance(&burn_account), 299_993);

		assert_eq!(Balances::balance(&launch_era_argon_refund_account()), 1_000_001);
		assert_eq!(Balances::balance(&small_launch_era_argon_refund_account()), 2_000);
		assert_eq!(Balances::balance(&post_hack_argon_refund_account()), 197_069_590);
		assert_eq!(Ownership::balance(&ready_argonot_refund_account()), 200_000);
	});
}

fn chain_config() -> ChainConfig {
	ChainConfig::Ethereum {
		gateway: h160(0x21),
		argon_token: h160(0x31),
		argonot_token: h160(0x32),
		previous_gateway: None,
		previous_release_expiration: None,
	}
}

fn chain_config_with_previous(previous_release_expiration: Tick) -> ChainConfig {
	ChainConfig::Ethereum {
		gateway: h160(0x21),
		argon_token: h160(0x31),
		argonot_token: h160(0x32),
		previous_gateway: Some(h160(0x40)),
		previous_release_expiration: Some(previous_release_expiration),
	}
}

fn argon_proof(
	recipient: crate::mock::TestAccountId,
	account_nonce: u64,
	amount: Balance,
) -> TransferProof {
	TransferProof::Ethereum {
		source_chain: SourceChain::Ethereum,
		event_log: burn_for_transfer_log(
			h160(0x21),
			h160(0x11),
			h160(0x31),
			amount as u128,
			destination_bytes(&recipient),
			account_nonce,
		),
		proof: dummy_proof(),
	}
}

fn argonot_proof(
	recipient: crate::mock::TestAccountId,
	account_nonce: u64,
	amount: Balance,
) -> TransferProof {
	TransferProof::Ethereum {
		source_chain: SourceChain::Ethereum,
		event_log: burn_for_transfer_log(
			h160(0x21),
			h160(0x11),
			h160(0x32),
			amount as u128,
			destination_bytes(&recipient),
			account_nonce,
		),
		proof: dummy_proof(),
	}
}

fn destination_bytes(recipient: &crate::mock::TestAccountId) -> [u8; 32] {
	let bytes: &[u8] = recipient.as_ref();
	bytes.try_into().expect("account id is 32 bytes")
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
		execution_block_proof: argon_primitives::ethereum::EthereumExecutionBlockProof {
			anchor_block_hash: H256::repeat_byte(1),
			target_to_anchor_header_chain: Vec::new(),
		},
		receipt_proof: argon_primitives::ethereum::EthereumReceiptProof {
			transaction_index: 0,
			nodes: vec![vec![1u8]],
		},
	}
}

fn launch_era_argon_refund_account() -> AccountId32 {
	AccountId32::from_ss58check("5C5CdgR7eNjc8HCtR43uWSKsMoWZGZpMYGgCXAPJPMKdoVU2")
		.expect("valid ss58")
}

fn small_launch_era_argon_refund_account() -> AccountId32 {
	AccountId32::from_ss58check("5F4UiKa1o5LLrLwgZz3pFizXhZnEvEr3mvtoGrEk3fZTXeyd")
		.expect("valid ss58")
}

fn post_hack_argon_refund_account() -> AccountId32 {
	AccountId32::from_ss58check("5Cz3PZVcLitGyqc1Su4KYcvseoLhn93pUHtXDNBLx5aoKsF5")
		.expect("valid ss58")
}

fn ready_argonot_refund_account() -> AccountId32 {
	AccountId32::from_ss58check("5EqsqBNe1LfkGLEah9GpSMWTT4XHzGeVEAZ4dGUm5vFHA4t8")
		.expect("valid ss58")
}
