use crate::{
	migrations::InitializeCrosschainTransferMigration, Call, ChainConfig, ChainConfigBySourceChain,
	Event, GatewayActivityProofBatch, GatewayActivityProofBlock, GatewayState,
	GatewayStateBySourceChain, InboundTransfersExpiringAt, RecentArgonTransfersByAccount,
	SourceChain, TransferToArgonActivity, TRANSFER_TO_ARGON_STARTED_EVENT_SIGNATURE,
};
use alloy_primitives::keccak256;
use argon_primitives::{
	ethereum::{
		EthereumCombinedReceiptProof, EthereumExecutionBlockProof, EthereumReceiptProofReceipt,
	},
	CallTxPoolKeyProvider, CallTxValidityProvider, EthereumLog, EthereumReceiptLog,
};
use frame_support::{assert_noop, assert_ok, dispatch::Pays, traits::OnRuntimeUpgrade};
use pallet_prelude::*;
use sp_core::crypto::Ss58Codec;
use sp_runtime::{transaction_validity::InvalidTransaction, AccountId32};

use crate::mock::{
	account, h160, legacy_token_gateway_account, new_test_ext, Balances, ConfirmedTransfers,
	CrosschainTransfer, CurrentTick, Ownership, ProofVerificationAllowed,
	ProofVerificationRejectedTransactionIndexes, RuntimeCall, RuntimeEvent, RuntimeOrigin, System,
	Test, TestAccountId,
};

#[test]
fn prove_gateway_activity_pays_argon_and_marks_recent_transfer() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));
		assert_eq!(
			ChainConfigBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(chain_config()),
		);

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert!(System::providers(&burn_account) > 0);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let recipient = account(2);
		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![argon_activity_log(recipient.clone(), 1, 1_250)])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(Balances::balance(&recipient), 1_250);
		assert_eq!(
			GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewayState::<Test> {
				gateway_activity_nonce: 1,
				argon_approvals_nonce: 0,
				argon_circulation: 0,
				argonot_circulation: 0,
			}),
		);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&recipient), 1);
		assert_eq!(ConfirmedTransfers::get(), vec![(recipient.clone(), 1_250)]);
		assert!(System::events().iter().any(|record| match &record.event {
			RuntimeEvent::CrosschainTransfer(Event::TransferToArgonSettled {
				source_chain: SourceChain::Ethereum,
				transfer,
			}) => {
				transfer ==
					&TransferToArgonActivity::<Test> {
						gateway_activity_nonce: 1,
						from: h160(0x11),
						asset: crate::AssetKind::Argon,
						to: recipient.clone(),
						amount: 1_250,
					}
			},
			_ => false,
		}));
		assert!(System::events().iter().any(|record| match &record.event {
			RuntimeEvent::CrosschainTransfer(Event::GatewayStateAdvanced {
				source_chain: SourceChain::Ethereum,
				gateway_state,
			}) => {
				gateway_state ==
					&GatewayState::<Test> {
						gateway_activity_nonce: 1,
						argon_approvals_nonce: 0,
						argon_circulation: 0,
						argonot_circulation: 0,
					}
			},
			_ => false,
		}));
	});
}

#[test]
fn prove_gateway_activity_allows_zero_amount_without_side_effects() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let recipient = account(9);
		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![argon_activity_log(recipient.clone(), 1, 0)])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(Balances::balance(&recipient), 0);
		assert_eq!(
			GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewayState::<Test> {
				gateway_activity_nonce: 1,
				argon_approvals_nonce: 0,
				argon_circulation: 0,
				argonot_circulation: 0,
			}),
		);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&recipient), 0);
		assert!(ConfirmedTransfers::get().is_empty());
	});
}

#[test]
fn prove_gateway_activity_pays_argonot_from_burn_account() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Ownership::mint_into(&burn_account, 777));

		let recipient = account(3);
		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![argonot_activity_log(recipient.clone(), 1, 777)])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(Ownership::balance(&recipient), 777);
		assert_eq!(Ownership::balance(&burn_account), 0);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&recipient), 0);
		assert!(ConfirmedTransfers::get().is_empty());
	});
}

#[test]
fn prove_gateway_activity_rejects_empty_proof_batch() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				GatewayActivityProofBatch::<Test> {
					execution_block_proof: dummy_execution_block_proof(),
					blocks: Vec::new()
						.try_into()
						.expect("empty proof batch stays within pallet block bound"),
				},
			),
			crate::Error::<Test>::NoGatewayProofBlocksProvided,
		);
	});
}

#[test]
fn prove_gateway_activity_rejects_empty_proof_block() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![])]),
			),
			crate::Error::<Test>::NoGatewayActivitiesProvided,
		);
	});
}

#[test]
fn prove_gateway_activity_accepts_anchor_target_block_number_for_anchor_path() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));
		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));
		let recipient = account(2);

		assert_ok!(CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			GatewayActivityProofBatch::<Test> {
				execution_block_proof: dummy_execution_block_proof(),
				blocks: vec![GatewayActivityProofBlock::<Test> {
					target_block_number: 0,
					receipt_proof: dummy_receipt_proof(),
					receipt_logs: activity_logs(vec![argon_activity_log(recipient.clone(), 1, 10)]),
				}]
				.try_into()
				.expect("single proof block stays within pallet block bound"),
			},
		),);
		assert_eq!(Balances::balance(&recipient), 10);
	});
}

#[test]
fn prove_gateway_activity_pool_key_uses_payload_not_signer() {
	new_test_ext().execute_with(|| {
		let first_call = RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
			source_chain: SourceChain::Ethereum,
			previous_gateway_activity_nonce: 0,
			proof_batch: proof_batch(vec![activity_logs(vec![argon_activity_log(
				account(2),
				1,
				1_250,
			)])]),
		});
		let retry_call = RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
			source_chain: SourceChain::Ethereum,
			previous_gateway_activity_nonce: 0,
			proof_batch: proof_batch(vec![activity_logs(vec![argon_activity_log(
				account(2),
				1,
				1_250,
			)])]),
		});
		let different_sender_same_nonce_call =
			RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
				source_chain: SourceChain::Ethereum,
				previous_gateway_activity_nonce: 0,
				proof_batch: proof_batch(vec![activity_logs(vec![transfer_to_argon_started_log(
					h160(0x21),
					h160(0x99),
					h160(0x31),
					1_250,
					destination_bytes(&account(2)),
					1,
					0,
				)])]),
			});
		let different_nonce_call =
			RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
				source_chain: SourceChain::Ethereum,
				previous_gateway_activity_nonce: 1,
				proof_batch: proof_batch(vec![activity_logs(vec![argon_activity_log(
					account(2),
					2,
					1_250,
				)])]),
			});

		let first_key =
			<CrosschainTransfer as CallTxPoolKeyProvider<RuntimeCall, TestAccountId>>::key_for(
				&first_call,
				Some(&account(1)),
			)
			.expect("prove_gateway_activity should publish a pool key");
		let retry_key =
			<CrosschainTransfer as CallTxPoolKeyProvider<RuntimeCall, TestAccountId>>::key_for(
				&retry_call,
				Some(&account(9)),
			)
			.expect("prove_gateway_activity retry should publish a pool key");
		let different_sender_same_nonce_key =
			<CrosschainTransfer as CallTxPoolKeyProvider<RuntimeCall, TestAccountId>>::key_for(
				&different_sender_same_nonce_call,
				Some(&account(9)),
			)
			.expect("prove_gateway_activity with same first nonce should publish a pool key");
		let different_nonce_key = <CrosschainTransfer as CallTxPoolKeyProvider<
			RuntimeCall,
			TestAccountId,
		>>::key_for(&different_nonce_call, Some(&account(1)))
		.expect("prove_gateway_activity with different nonce should publish a pool key");

		assert_eq!(first_key, retry_key);
		assert_ne!(first_key, different_sender_same_nonce_key);
		assert_ne!(first_key, different_nonce_key);
	});
}

#[test]
fn prove_gateway_activity_validate_marks_previous_gateway_activity_nonce_stale() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		assert_ok!(CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![argon_activity_log(account(2), 1, 500)])]),
		));

		let stale_call = RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
			source_chain: SourceChain::Ethereum,
			previous_gateway_activity_nonce: 0,
			proof_batch: proof_batch(vec![activity_logs(vec![argon_activity_log(
				account(3),
				2,
				750,
			)])]),
		});

		assert!(matches!(
			<CrosschainTransfer as CallTxValidityProvider<RuntimeCall, TestAccountId>>::validate(
				&stale_call,
				Some(&account(1)),
			),
			Err(TransactionValidityError::Invalid(InvalidTransaction::Stale))
		));
	});
}

#[test]
fn prove_gateway_activity_accepts_contiguous_batch_across_proof_blocks() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let first_recipient = account(2);
		let second_recipient = account(3);

		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![
				activity_logs(vec![argon_activity_log(first_recipient.clone(), 1, 500)]),
				activity_logs(vec![argon_activity_log(second_recipient.clone(), 2, 750)]),
			]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(Balances::balance(&first_recipient), 500);
		assert_eq!(Balances::balance(&second_recipient), 750);
		assert_eq!(
			GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum)
				.expect("gateway state should be written")
				.gateway_activity_nonce,
			2,
		);
		assert_eq!(
			ConfirmedTransfers::get(),
			vec![(first_recipient.clone(), 500), (second_recipient.clone(), 750)],
		);
	});
}

#[test]
fn prove_gateway_activity_accepts_multiple_logs_from_one_receipt_in_order() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let first_recipient = account(10);
		let second_recipient = account(11);

		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![
				argon_activity_log(first_recipient.clone(), 1, 400),
				argon_activity_log(second_recipient.clone(), 2, 600),
			])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(Balances::balance(&first_recipient), 400);
		assert_eq!(Balances::balance(&second_recipient), 600);
		assert_eq!(
			GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum)
				.expect("gateway state should be written")
				.gateway_activity_nonce,
			2,
		);
		assert_eq!(
			ConfirmedTransfers::get(),
			vec![(first_recipient.clone(), 400), (second_recipient.clone(), 600)],
		);
	});
}

#[test]
fn prove_gateway_activity_rejects_multiple_logs_from_one_receipt_out_of_order_without_leakage() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let first_recipient = account(12);
		let second_recipient = account(13);

		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![
					argon_activity_log(first_recipient.clone(), 2, 400),
					argon_activity_log(second_recipient.clone(), 1, 600),
				])]),
			),
			crate::Error::<Test>::UnexpectedGatewayActivityNonce,
		);

		assert_eq!(Balances::balance(&first_recipient), 0);
		assert_eq!(Balances::balance(&second_recipient), 0);
		assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&first_recipient), 0);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&second_recipient), 0);
		assert!(
			!System::events()
				.iter()
				.any(|record| { matches!(record.event, RuntimeEvent::CrosschainTransfer(_)) }),
			"transactional batch failure should roll back crosschain events",
		);
	});
}

#[test]
fn prove_gateway_activity_rejects_invalid_later_proof_block_without_partial_settlement() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));
		ProofVerificationRejectedTransactionIndexes::set(vec![1]);

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let first_recipient = account(14);
		let second_recipient = account(15);

		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![
					activity_logs_for_transaction_index(
						vec![argon_activity_log(first_recipient.clone(), 1, 500)],
						0,
					),
					activity_logs_for_transaction_index(
						vec![argon_activity_log(second_recipient.clone(), 2, 750)],
						1,
					),
				]),
			),
			crate::Error::<Test>::InvalidProof,
		);

		assert_eq!(Balances::balance(&first_recipient), 0);
		assert_eq!(Balances::balance(&second_recipient), 0);
		assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&first_recipient), 0);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&second_recipient), 0);
		assert!(
			!System::events()
				.iter()
				.any(|record| { matches!(record.event, RuntimeEvent::CrosschainTransfer(_)) }),
			"invalid later proof block should not leave partial crosschain events behind",
		);
	});
}

#[test]
fn prove_gateway_activity_rejects_invalid_later_activity_without_partial_settlement() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let first_recipient = account(16);
		let second_recipient = account(17);

		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![
					argon_activity_log(first_recipient.clone(), 1, 500),
					argon_activity_log(second_recipient.clone(), 3, 750),
				])]),
			),
			crate::Error::<Test>::UnexpectedGatewayActivityNonce,
		);

		assert_eq!(Balances::balance(&first_recipient), 0);
		assert_eq!(Balances::balance(&second_recipient), 0);
		assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&first_recipient), 0);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&second_recipient), 0);
		assert!(
			!System::events()
				.iter()
				.any(|record| { matches!(record.event, RuntimeEvent::CrosschainTransfer(_)) }),
			"invalid later activity should not leave partial crosschain events behind",
		);
	});
}

#[test]
fn prove_gateway_activity_rejects_unsupported_gateway() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let recipient = account(4);
		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![transfer_to_argon_started_log(
					h160(0x44),
					h160(0x11),
					h160(0x31),
					5,
					destination_bytes(&recipient),
					1,
					0,
				)])]),
			),
			crate::Error::<Test>::UnsupportedGateway,
		);
	});
}

#[test]
fn prove_gateway_activity_rejects_wrong_previous_gateway_activity_nonce() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let recipient = account(5);
		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				1,
				proof_batch(vec![activity_logs(vec![argon_activity_log(
					recipient.clone(),
					1,
					10,
				)])]),
			),
			crate::Error::<Test>::UnexpectedPreviousGatewayActivityNonce,
		);
	});
}

#[test]
fn prove_gateway_activity_rejects_non_contiguous_gateway_activity_nonce() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(RuntimeOrigin::root(), chain_config(),));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		let recipient = account(5);
		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![argon_activity_log(
					recipient.clone(),
					2,
					10,
				)])]),
			),
			crate::Error::<Test>::UnexpectedGatewayActivityNonce,
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
		assert_ok!(CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![argon_activity_log(recipient.clone(), 1, 55)])]),
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
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![argon_activity_log(account(8), 1, 100)])]),
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
	}
}

fn argon_activity_log(
	recipient: crate::mock::TestAccountId,
	gateway_activity_nonce: u64,
	amount: Balance,
) -> EthereumLog {
	transfer_to_argon_started_log(
		h160(0x21),
		h160(0x11),
		h160(0x31),
		amount,
		destination_bytes(&recipient),
		gateway_activity_nonce,
		0,
	)
}

fn argonot_activity_log(
	recipient: crate::mock::TestAccountId,
	gateway_activity_nonce: u64,
	amount: Balance,
) -> EthereumLog {
	transfer_to_argon_started_log(
		h160(0x21),
		h160(0x11),
		h160(0x32),
		amount,
		destination_bytes(&recipient),
		gateway_activity_nonce,
		0,
	)
}

fn destination_bytes(recipient: &crate::mock::TestAccountId) -> [u8; 32] {
	let bytes: &[u8] = recipient.as_ref();
	bytes.try_into().expect("account id is 32 bytes")
}

fn transfer_to_argon_started_log(
	gateway: H160,
	from: H160,
	token: H160,
	amount: Balance,
	destination: [u8; 32],
	gateway_activity_nonce: u64,
	argon_approvals_nonce: u64,
) -> EthereumLog {
	let mut data = Vec::with_capacity(192);
	data.extend_from_slice(&u128_word(amount as u128));
	data.extend_from_slice(&destination);
	data.extend_from_slice(&u64_word(gateway_activity_nonce));
	data.extend_from_slice(&u64_word(argon_approvals_nonce));
	data.extend_from_slice(&u128_word(0));
	data.extend_from_slice(&u128_word(0));

	EthereumLog {
		address: gateway,
		topics: vec![
			H256::from_slice(keccak256(TRANSFER_TO_ARGON_STARTED_EVENT_SIGNATURE).as_slice()),
			indexed_address_word(from),
			indexed_address_word(token),
		]
		.try_into()
		.expect("topics stay within Ethereum log topic bounds"),
		data: data
			.try_into()
			.expect("transfer-to-argon event data stays within bounded log payload"),
	}
}

fn activity_logs(
	logs: Vec<EthereumLog>,
) -> BoundedVec<EthereumReceiptLog, crate::mock::MaxActivitiesPerReceiptProof> {
	activity_logs_for_transaction_index(logs, 0)
}

fn activity_logs_for_transaction_index(
	logs: Vec<EthereumLog>,
	transaction_index: u64,
) -> BoundedVec<EthereumReceiptLog, crate::mock::MaxActivitiesPerReceiptProof> {
	logs.into_iter()
		.map(|event_log| EthereumReceiptLog { transaction_index, event_log })
		.collect::<Vec<_>>()
		.try_into()
		.expect("test gateway activity logs stay within pallet bound")
}

fn proof_batch(
	log_blocks: Vec<BoundedVec<EthereumReceiptLog, crate::mock::MaxActivitiesPerReceiptProof>>,
) -> GatewayActivityProofBatch<Test> {
	GatewayActivityProofBatch::<Test> {
		execution_block_proof: dummy_execution_block_proof(),
		blocks: log_blocks
			.into_iter()
			.map(|receipt_logs| GatewayActivityProofBlock::<Test> {
				target_block_number: 0,
				receipt_proof: dummy_receipt_proof(),
				receipt_logs,
			})
			.collect::<Vec<_>>()
			.try_into()
			.expect("test gateway proof blocks stay within pallet bound"),
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

fn u128_word(value: u128) -> [u8; 32] {
	let mut bytes = [0u8; 32];
	bytes[16..].copy_from_slice(&value.to_be_bytes());
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

fn dummy_receipt_proof() -> EthereumCombinedReceiptProof {
	EthereumCombinedReceiptProof {
		nodes: vec![vec![1u8].try_into().expect("tiny receipt proof node stays within bounds")]
			.try_into()
			.expect("single-node receipt proof stays within bounds"),
		receipts: vec![EthereumReceiptProofReceipt {
			transaction_index: 0,
			node_indexes: vec![0]
				.try_into()
				.expect("single node index stays within bounded receipt proof refs"),
		}]
		.try_into()
		.expect("single receipt reference stays within bounded receipt proof count"),
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
