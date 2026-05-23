use super::*;
use alloy_sol_types::SolEvent;
use argon_ethereum_contracts::minting_gateway::{
	GatewayActivityState as ContractGatewayActivityState, MintingAuthorityActivated,
	MintingAuthorityCollateral as ContractMintingAuthorityCollateral, MintingAuthorityDeactivated,
	TransferOutOfArgonCanceled as ContractTransferOutOfArgonCanceled,
	TransferOutOfArgonFinalized as ContractTransferOutOfArgonFinalized, TransferToArgonStarted,
};
use argon_primitives::{
	ethereum::{
		EthereumCombinedReceiptProof, EthereumExecutionBlockProof, EthereumReceiptProofReceipt,
	},
	EthereumLog, EthereumReceiptLog,
};

#[test]
fn prove_gateway_activity_pays_argon_marks_recent_transfer_and_ignores_zero_amounts() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));
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
				argon_circulation: 8_750,
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
						asset: AssetKind::Argon,
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
						argon_circulation: 8_750,
						argonot_circulation: 0,
					}
			},
			_ => false,
		}));
		let zero_amount_recipient = account(9);
		let zero_result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			1,
			proof_batch(vec![activity_logs(vec![argon_activity_log(
				zero_amount_recipient.clone(),
				2,
				0,
			)])]),
		);

		assert!(matches!(zero_result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(Balances::balance(&zero_amount_recipient), 0);
		assert_eq!(
			GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewayState::<Test> {
				gateway_activity_nonce: 2,
				argon_approvals_nonce: 0,
				argon_circulation: 8_750,
				argonot_circulation: 0,
			}),
		);
		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&zero_amount_recipient), 0);
		assert_eq!(ConfirmedTransfers::get(), vec![(recipient, 1_250)]);
	});
}

#[test]
fn prove_gateway_activity_pays_argonot_from_burn_account() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));

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
fn prove_gateway_activity_records_minting_authority_activation_and_prunes_older_synced_queue_entries(
) {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owner_vault_operator = account(31);
		let council_pair = council_signing_pair(3);
		let minting_authority_pair = council_signing_pair(4);
		let signing_key = council_signer(&minting_authority_pair);

		configure_single_member_ethereum_council(
			owner_vault_operator.clone(),
			8,
			10_000,
			&council_pair,
		);
		assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
		assert_ok!(Ownership::mint_into(&owner_vault_operator, 500));
		assert_ok!(set_committed_argonots(owner_vault_operator.clone(), 200));
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			signing_key,
			minting_authority_registration_signature(
				&minting_authority_pair,
				&owner_vault_operator,
			),
			10_000,
			0,
		));
		let approval_signature = minting_authority_approval_signature(&council_pair, 1);
		assert_ok!(CrosschainTransfer::approve_queue_entries(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			bounded_vec![approval_signature],
		));

		assert_ok!(CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![minting_authority_activated_log(
				10_000,
				0,
				signing_key,
				owner_vault_operator.clone(),
				1,
				1,
				1,
				1,
			)])]),
		));

		assert_eq!(
			MintingAuthoritiesBySigner::<Test>::get(signing_key),
			Some(MintingAuthority::<Test> {
				account_id: owner_vault_operator.clone(),
				destination_chain: SourceChain::Ethereum,
				destination_signing_key: signing_key,
				state: MintingAuthorityState::Active,
				gateway_remaining_microgon_collateral: 10_000,
				gateway_remaining_micronot_collateral: 0,
				pending_reserved_microgon_collateral: 0,
				pending_reserved_micronot_collateral: 0,
				active_pending_transfer_ids: bounded_vec![],
				activation_approval_queue_nonce: 1,
				activation_base_repayment_due: None,
				activation_signature_repayment_due: None,
				activation_repayment_due: None,
				deactivation_approval_queue_nonce: None,
			}),
		);
		let activation_entry =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 1)
				.expect("last local synced queue entry should remain as previous-hash anchor");
		assert_eq!(
			CrosschainTransfer::previous_gateway_update_hash(SourceChain::Ethereum, 2),
			Ok(activation_entry.approval_hash),
		);
		assert_eq!(
			GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewayState::<Test> {
				gateway_activity_nonce: 1,
				argon_approvals_nonce: 1,
				argon_circulation: 0,
				argonot_circulation: 0,
			}),
		);
		let deactivation_signature = minting_authority_deactivation_signature(
			&minting_authority_pair,
			2,
			signing_key,
			CrosschainTransfer::previous_gateway_update_hash(SourceChain::Ethereum, 2)
				.expect("activation queue entry should anchor deactivation"),
		);
		assert_ok!(CrosschainTransfer::deactivate_minting_authority(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			signing_key,
			deactivation_signature,
		));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 500));
		let recipient = account(33);
		let (argon_circulation, argonot_circulation) =
			current_gateway_circulation_after(Some((AssetKind::Argon, 5)), None);
		assert_ok!(CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			1,
			proof_batch(vec![activity_logs(vec![transfer_to_argon_started_log(
				h160(0x21),
				h160(0x11),
				h160(0x31),
				5,
				destination_bytes(&recipient),
				2,
				2,
				argon_circulation,
				argonot_circulation,
			)])]),
		));

		assert!(
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 1)
				.is_none(),
			"older fully-synced queue entries should be pruned",
		);
		let retained_entry =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 2)
				.expect("latest local synced queue entry should be retained");
		assert_eq!(
			CrosschainTransfer::previous_gateway_update_hash(SourceChain::Ethereum, 3),
			Ok(retained_entry.approval_hash),
		);
	});
}

#[test]
fn third_party_relayed_activation_pays_relayer_and_activates_immediately() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owner_vault_operator = account(34);
		let relayer = account(35);
		let council_pair = council_signing_pair(5);
		let minting_authority_pair = council_signing_pair(6);
		let signing_key = council_signer(&minting_authority_pair);

		configure_single_member_ethereum_council(
			owner_vault_operator.clone(),
			10,
			10_000,
			&council_pair,
		);
		assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			signing_key,
			minting_authority_registration_signature(
				&minting_authority_pair,
				&owner_vault_operator,
			),
			10_000,
			0,
		));
		assert_ok!(CrosschainTransfer::approve_queue_entries(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			bounded_vec![minting_authority_approval_signature(&council_pair, 1)],
		));

		let deactivation_signature = minting_authority_deactivation_signature(
			&minting_authority_pair,
			2,
			signing_key,
			CrosschainTransfer::previous_gateway_update_hash(SourceChain::Ethereum, 2)
				.expect("activation queue entry should anchor deactivation"),
		);
		assert_noop!(
			CrosschainTransfer::deactivate_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				signing_key,
				deactivation_signature,
			),
			Error::<Test>::UnexpectedMintingAuthorityState,
		);

		assert_ok!(CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![minting_authority_activated_log(
				10_000,
				0,
				signing_key,
				relayer.clone(),
				1,
				1,
				1,
				1,
			)])]),
		));

		assert_eq!(
			MintingAuthoritiesBySigner::<Test>::get(signing_key),
			Some(MintingAuthority::<Test> {
				account_id: owner_vault_operator.clone(),
				destination_chain: SourceChain::Ethereum,
				destination_signing_key: signing_key,
				state: MintingAuthorityState::Active,
				gateway_remaining_microgon_collateral: 10_000,
				gateway_remaining_micronot_collateral: 0,
				pending_reserved_microgon_collateral: 0,
				pending_reserved_micronot_collateral: 0,
				active_pending_transfer_ids: bounded_vec![],
				activation_approval_queue_nonce: 1,
				activation_base_repayment_due: None,
				activation_signature_repayment_due: None,
				activation_repayment_due: None,
				deactivation_approval_queue_nonce: None,
			}),
		);
		assert_eq!(Balances::balance(&relayer), 150);

		assert_ok!(CrosschainTransfer::deactivate_minting_authority(
			RuntimeOrigin::signed(owner_vault_operator),
			signing_key,
			deactivation_signature,
		));
		assert_eq!(
			MintingAuthoritiesBySigner::<Test>::get(signing_key)
				.expect("authority should stay available until proof-backed deactivation")
				.state,
			MintingAuthorityState::Deactivating,
		);
	});
}

#[test]
fn shared_activation_signature_refunds_excess_hold_back_to_authorities() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let council_account = account(41);
		let first_authority_account = account(42);
		let second_authority_account = account(43);
		let relayer = account(44);
		let council_pair = council_signing_pair(41);
		let first_authority_pair = council_signing_pair(42);
		let second_authority_pair = council_signing_pair(43);
		let first_signing_key = council_signer(&first_authority_pair);
		let second_signing_key = council_signer(&second_authority_pair);

		configure_single_member_ethereum_council(
			council_account.clone(),
			41,
			10_000,
			&council_pair,
		);
		register_vault_operator(first_authority_account.clone(), 42, 10_000);
		register_vault_operator(second_authority_account.clone(), 43, 10_000);
		assert_ok!(Balances::mint_into(&first_authority_account, 10_000));
		assert_ok!(Balances::mint_into(&second_authority_account, 10_000));
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(first_authority_account.clone()),
			SourceChain::Ethereum,
			first_signing_key,
			minting_authority_registration_signature(
				&first_authority_pair,
				&first_authority_account,
			),
			10_000,
			0,
		));
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(second_authority_account.clone()),
			SourceChain::Ethereum,
			second_signing_key,
			minting_authority_registration_signature(
				&second_authority_pair,
				&second_authority_account,
			),
			10_000,
			0,
		));
		assert_ok!(CrosschainTransfer::approve_queue_entries(
			RuntimeOrigin::signed(council_account),
			SourceChain::Ethereum,
			bounded_vec![
				minting_authority_approval_signature(&council_pair, 1),
				minting_authority_approval_signature(&council_pair, 2),
			],
		));

		assert_ok!(CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![
				minting_authority_activated_log(
					10_000,
					0,
					first_signing_key,
					relayer.clone(),
					2,
					1,
					1,
					1,
				),
				minting_authority_activated_log(
					10_000,
					0,
					second_signing_key,
					relayer.clone(),
					2,
					1,
					2,
					2,
				),
			])]),
		));

		assert_eq!(Balances::balance(&relayer), 250);
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::CrosschainTransfer(
					HoldReason::MintingAuthorityActivationRepayment,
				),
				&first_authority_account,
			),
			0,
		);
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::CrosschainTransfer(
					HoldReason::MintingAuthorityActivationRepayment,
				),
				&second_authority_account,
			),
			0,
		);
		assert_eq!(
			MintingAuthoritiesBySigner::<Test>::get(first_signing_key)
				.expect("first authority should activate")
				.state,
			MintingAuthorityState::Active,
		);
		assert_eq!(
			MintingAuthoritiesBySigner::<Test>::get(second_signing_key)
				.expect("second authority should activate")
				.state,
			MintingAuthorityState::Active,
		);
	});
}

#[test]
fn prove_gateway_activity_uses_gateway_deactivation_collateral_without_pausing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owner_vault_operator = account(33);
		let council_pair = council_signing_pair(31);
		let minting_authority_pair = council_signing_pair(32);
		let signing_key = council_signer(&minting_authority_pair);

		configure_single_member_ethereum_council(
			owner_vault_operator.clone(),
			9,
			4_000,
			&council_pair,
		);
		assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
		assert_ok!(Ownership::mint_into(&owner_vault_operator, 500));
		assert_ok!(CrosschainTransfer::set_minimum_minting_authority_value(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			3_000,
		));
		assert_ok!(set_committed_argonots(owner_vault_operator.clone(), 250));
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			signing_key,
			minting_authority_registration_signature(
				&minting_authority_pair,
				&owner_vault_operator,
			),
			4_000,
			250,
		));
		MintingAuthoritiesBySigner::<Test>::insert(
			signing_key,
			MintingAuthority::<Test> {
				account_id: owner_vault_operator.clone(),
				destination_chain: SourceChain::Ethereum,
				destination_signing_key: signing_key,
				state: MintingAuthorityState::Active,
				gateway_remaining_microgon_collateral: 4_000,
				gateway_remaining_micronot_collateral: 250,
				pending_reserved_microgon_collateral: 0,
				pending_reserved_micronot_collateral: 0,
				active_pending_transfer_ids: bounded_vec![],
				activation_approval_queue_nonce: 1,
				activation_base_repayment_due: None,
				activation_signature_repayment_due: None,
				activation_repayment_due: None,
				deactivation_approval_queue_nonce: None,
			},
		);

		assert_ok!(CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![minting_authority_deactivated_log(
				3_000,
				200,
				signing_key,
				account(35),
				1,
				5,
			)])]),
		));

		assert_eq!(MintingAuthoritiesBySigner::<Test>::get(signing_key), None);
		assert_eq!(encumbered_bond_microgons(&owner_vault_operator), 0);
		assert_eq!(encumbered_argonot_micronots(&owner_vault_operator), 0);
		assert_eq!(active_bond_microgons(&owner_vault_operator), 3_000);
		assert_eq!(committed_argonot_micronots(&owner_vault_operator), 200);
		assert_eq!(GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum), None,);
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			signing_key,
			minting_authority_registration_signature(
				&minting_authority_pair,
				&owner_vault_operator,
			),
			3_000,
			200,
		));
		assert_eq!(
			MintingAuthoritiesBySigner::<Test>::get(signing_key),
			Some(MintingAuthority::<Test> {
				account_id: owner_vault_operator,
				destination_chain: SourceChain::Ethereum,
				destination_signing_key: signing_key,
				state: MintingAuthorityState::PendingActivation,
				gateway_remaining_microgon_collateral: 3_000,
				gateway_remaining_micronot_collateral: 200,
				pending_reserved_microgon_collateral: 0,
				pending_reserved_micronot_collateral: 0,
				active_pending_transfer_ids: bounded_vec![],
				activation_approval_queue_nonce: 2,
				activation_base_repayment_due: Some(100),
				activation_signature_repayment_due: Some(50),
				activation_repayment_due: Some(150),
				deactivation_approval_queue_nonce: None,
			}),
		);
		assert_eq!(
			encumbered_bond_microgons(
				&MintingAuthoritiesBySigner::<Test>::get(signing_key)
					.expect("minting authority should be re-registered")
					.account_id,
			),
			3_000,
		);
		assert_eq!(
			encumbered_argonot_micronots(
				&MintingAuthoritiesBySigner::<Test>::get(signing_key)
					.expect("minting authority should be re-registered")
					.account_id,
			),
			200,
		);
	});
}

#[test]
fn prove_gateway_activity_deactivation_invalidates_all_pending_signer_reservations() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let authority_account = account(36);
		let authority_pair = council_signing_pair(37);
		let signing_key = activate_test_minting_authority(
			authority_account.clone(),
			48,
			20_000,
			&council_signing_pair(38),
			&authority_pair,
			12_000,
			0,
		);
		let first_user = account(39);
		let second_user = account(40);

		assert_ok!(Balances::mint_into(&first_user, 5_000));
		assert_ok!(Balances::mint_into(&second_user, 4_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(first_user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x61),
			4_000,
		));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(second_user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x62),
			3_000,
		));

		let first_transfer_id = transfer_out_id(&first_user, 1);
		let second_transfer_id = transfer_out_id(&second_user, 1);
		assert_ok!(CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(authority_account.clone()),
			first_transfer_id,
			transfer_collateral_signature(&authority_pair, first_transfer_id, 4_000, 0),
			4_000,
			0,
		));
		assert_ok!(CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(authority_account),
			second_transfer_id,
			transfer_collateral_signature(&authority_pair, second_transfer_id, 3_000, 0),
			3_000,
			0,
		));

		assert_ok!(CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			1,
			proof_batch(vec![activity_logs(vec![minting_authority_deactivated_log(
				12_000,
				0,
				signing_key,
				account(41),
				2,
				1,
			)])]),
		));

		assert_eq!(MintingAuthoritiesBySigner::<Test>::get(signing_key), None);
		let first_transfer =
			TransferOutById::<Test>::get(first_transfer_id).expect("first transfer should remain");
		let second_transfer = TransferOutById::<Test>::get(second_transfer_id)
			.expect("second transfer should remain");
		assert_eq!(first_transfer.state, TransferOutState::Started);
		assert_eq!(second_transfer.state, TransferOutState::Started);
		assert_eq!(first_transfer.total_attached_collateral, 0);
		assert_eq!(second_transfer.total_attached_collateral, 0);
		assert!(first_transfer
			.minting_authority_collateral_by_signer
			.get(&signing_key)
			.is_none());
		assert!(second_transfer
			.minting_authority_collateral_by_signer
			.get(&signing_key)
			.is_none());
		let pending_requests =
			PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum);
		assert_eq!(pending_requests.len(), 2);
		assert!(pending_requests.contains(&PendingCollateralizationRequest::<Test> {
			transfer_id: first_transfer_id,
			remaining_collateral: 4_000,
			remaining_minting_authority_tip: 4,
		}));
		assert!(pending_requests.contains(&PendingCollateralizationRequest::<Test> {
			transfer_id: second_transfer_id,
			remaining_collateral: 3_000,
			remaining_minting_authority_tip: 3,
		}));
		assert_eq!(GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum), None);
		assert!(System::events().iter().any(|record| match &record.event {
			RuntimeEvent::CrosschainTransfer(Event::TransferCollateralInvalidated {
				transfer_id,
				destination_signing_key,
			}) => *transfer_id == first_transfer_id && *destination_signing_key == signing_key,
			_ => false,
		}));
		assert!(System::events().iter().any(|record| match &record.event {
			RuntimeEvent::CrosschainTransfer(Event::TransferCollateralInvalidated {
				transfer_id,
				destination_signing_key,
			}) => *transfer_id == second_transfer_id && *destination_signing_key == signing_key,
			_ => false,
		}));
	});
}

#[test]
fn prove_gateway_activity_rolls_back_failed_finalized_activity_before_pausing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let council_account = account(170);
		let council_pair = council_signing_pair(171);
		let user = account(172);

		configure_single_member_ethereum_council(
			council_account.clone(),
			145,
			20_000,
			&council_pair,
		);
		assert_ok!(Balances::mint_into(&user, 25_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x58),
			20_000,
		));

		let transfer_id = transfer_out_id(&user, 1);
		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![transfer_out_of_argon_finalized_log(
				transfer_id,
				vec![(h160(0xad), 20_000, 0)],
				1,
				0,
			)])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
		assert_eq!(
			GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewaySyncPause {
				last_good_gateway_activity_nonce: 0,
				failed_gateway_activity_nonce: 1,
				reason: GatewaySyncPauseReason::MintingAuthorityNotFound,
			}),
		);
		assert_eq!(
			TransferOutById::<Test>::get(transfer_id).map(|transfer| transfer.amount),
			Some(20_000),
		);
		assert_eq!(
			PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum),
			vec![PendingCollateralizationRequest::<Test> {
				transfer_id,
				remaining_collateral: 20_000,
				remaining_minting_authority_tip: 20,
			}],
		);
		assert_eq!(
			NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
			1,
		);
		assert_eq!(
			PendingTransferOutCirculationByDestinationChain::<Test>::get(SourceChain::Ethereum)
				.argon_circulation,
			20_000,
		);
	});
}

#[test]
fn prove_gateway_activity_rolls_back_finalized_activity_when_collateral_exceeds_remaining() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let authority_account = account(175);
		let authority_pair = council_signing_pair(92);
		let signing_key = activate_test_minting_authority(
			authority_account.clone(),
			46,
			20_000,
			&council_signing_pair(93),
			&authority_pair,
			10_000,
			0,
		);
		let user = account(176);

		assert_ok!(Balances::mint_into(&user, 6_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x5a),
			5_000,
		));
		let transfer_id = transfer_out_id(&user, 1);
		assert_ok!(CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(authority_account),
			transfer_id,
			transfer_collateral_signature(&authority_pair, transfer_id, 5_000, 0),
			5_000,
			0,
		));

		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			1,
			proof_batch(vec![activity_logs(vec![transfer_out_of_argon_finalized_log(
				transfer_id,
				vec![(signing_key, 11_000, 0)],
				2,
				1,
			)])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(
			GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewayState::<Test> {
				gateway_activity_nonce: 1,
				argon_approvals_nonce: 1,
				argon_circulation: 0,
				argonot_circulation: 0,
			}),
		);
		assert_eq!(
			GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewaySyncPause {
				last_good_gateway_activity_nonce: 1,
				failed_gateway_activity_nonce: 2,
				reason: GatewaySyncPauseReason::MintingAuthorityMismatch,
			}),
		);

		let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
			.expect("authority should remain registered");
		let transfer =
			TransferOutById::<Test>::get(transfer_id).expect("transfer should remain stored");
		assert_eq!(authority.gateway_remaining_microgon_collateral, 10_000);
		assert_eq!(authority.pending_reserved_microgon_collateral, 5_000);
		assert_eq!(transfer.state, TransferOutState::Ready);
		assert_eq!(transfer.total_attached_collateral, 5_000);
		assert_eq!(
			transfer
				.minting_authority_collateral_by_signer
				.get(&signing_key)
				.map(|row| row.microgon_collateral),
			Some(5_000),
		);
		assert_eq!(authority.active_pending_transfer_ids, vec![transfer_id],);
		assert!(
			PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum).is_empty()
		);
		assert_eq!(
			NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
			1,
		);
		assert_eq!(
			PendingTransferOutCirculationByDestinationChain::<Test>::get(SourceChain::Ethereum)
				.argon_circulation,
			5_000,
		);
	});
}

#[test]
fn prove_gateway_activity_mints_unknown_finalized_principal_into_burn_account() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let authority_account = account(179);
		let authority_pair = council_signing_pair(96);
		let signing_key = activate_test_minting_authority(
			authority_account.clone(),
			47,
			20_000,
			&council_signing_pair(97),
			&authority_pair,
			10_000,
			0,
		);
		let user = account(180);

		assert_ok!(Balances::mint_into(&user, 6_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x5c),
			5_000,
		));
		let local_transfer_id = transfer_out_id(&user, 1);
		let external_transfer_id = H256::repeat_byte(0x93);
		assert_ok!(CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(authority_account.clone()),
			local_transfer_id,
			transfer_collateral_signature(&authority_pair, local_transfer_id, 5_000, 0),
			5_000,
			0,
		));
		let (argon_circulation, argonot_circulation) =
			current_gateway_circulation_after(None, Some((AssetKind::Argon, 5_000)));

		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			1,
			proof_batch(vec![activity_logs(vec![
				transfer_out_of_argon_finalized_log_with_circulation(
					external_transfer_id,
					AssetKind::Argon,
					5_000,
					vec![(signing_key, 5_000, 0)],
					2,
					1,
					argon_circulation,
					argonot_circulation,
				),
			])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum), None);
		assert_eq!(
			GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewayState::<Test> {
				gateway_activity_nonce: 2,
				argon_approvals_nonce: 1,
				argon_circulation: 5_000,
				argonot_circulation: 0,
			}),
		);
		assert_eq!(
			Balances::balance(&CrosschainTransfer::burn_account(SourceChain::Ethereum)),
			10_000,
		);
		let local_transfer = TransferOutById::<Test>::get(local_transfer_id)
			.expect("local transfer should remain after unrelated unknown finalization");
		assert!(TransferOutById::<Test>::get(external_transfer_id).is_none());
		assert_eq!(local_transfer.state, TransferOutState::Ready);
		assert_eq!(local_transfer.total_attached_collateral, 5_000);
		assert_eq!(local_transfer.minting_authority_collateral_by_signer.len(), 1);
		assert_eq!(encumbered_bond_microgons(&authority_account), 5_000);
	});
}

#[test]
fn prove_gateway_activity_pauses_when_unknown_finalized_burn_cleanup_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let authority_account = account(179);
		let authority_pair = council_signing_pair(96);
		let signing_key = activate_test_minting_authority(
			authority_account.clone(),
			47,
			20_000,
			&council_signing_pair(97),
			&authority_pair,
			10_000,
			0,
		);
		let user = account(180);

		assert_ok!(Balances::mint_into(&user, 6_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x5c),
			5_000,
		));
		let local_transfer_id = transfer_out_id(&user, 1);
		let external_transfer_id = H256::repeat_byte(0x93);
		assert_ok!(CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(authority_account.clone()),
			local_transfer_id,
			transfer_collateral_signature(&authority_pair, local_transfer_id, 5_000, 0),
			5_000,
			0,
		));
		crate::mock::EncumberedBondMicrogonsByAccount::mutate(|entries| {
			entries.insert(authority_account.clone(), 1_000);
		});
		let (argon_circulation, argonot_circulation) =
			current_gateway_circulation_after(None, Some((AssetKind::Argon, 5_000)));

		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			1,
			proof_batch(vec![activity_logs(vec![
				transfer_out_of_argon_finalized_log_with_circulation(
					external_transfer_id,
					AssetKind::Argon,
					5_000,
					vec![(signing_key, 5_000, 0)],
					2,
					1,
					argon_circulation,
					argonot_circulation,
				),
			])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(
			GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewayState::<Test> {
				gateway_activity_nonce: 1,
				argon_approvals_nonce: 1,
				argon_circulation: 0,
				argonot_circulation: 0,
			}),
		);
		assert_eq!(
			GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewaySyncPause {
				last_good_gateway_activity_nonce: 1,
				failed_gateway_activity_nonce: 2,
				reason: GatewaySyncPauseReason::GatewayStateDrift,
			}),
		);
		assert_eq!(
			Balances::balance(&CrosschainTransfer::burn_account(SourceChain::Ethereum)),
			5_000,
		);
		let local_transfer = TransferOutById::<Test>::get(local_transfer_id)
			.expect("local transfer should remain after unknown finalization");
		assert!(TransferOutById::<Test>::get(external_transfer_id).is_none());
		assert_eq!(local_transfer.state, TransferOutState::Ready);
		assert_eq!(local_transfer.total_attached_collateral, 5_000);
		assert_eq!(local_transfer.minting_authority_collateral_by_signer.len(), 1);
		assert_eq!(encumbered_bond_microgons(&authority_account), 1_000);
	});
}

#[test]
fn prove_gateway_activity_rolls_back_failed_canceled_activity_before_pausing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let council_account = account(177);
		let council_pair = council_signing_pair(94);
		let user = account(178);

		configure_single_member_ethereum_council(
			council_account.clone(),
			146,
			20_000,
			&council_pair,
		);
		assert_ok!(Balances::mint_into(&user, 25_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x5b),
			20_000,
		));

		let transfer_id = transfer_out_id(&user, 1);
		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		let _ = Balances::burn_from(
			&burn_account,
			20_000,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Force,
		)
		.expect("draining the burn account should succeed in the mock");

		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![transfer_out_of_argon_canceled_log(
				transfer_id,
				1,
				0,
			)])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
		assert_eq!(
			GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewaySyncPause {
				last_good_gateway_activity_nonce: 0,
				failed_gateway_activity_nonce: 1,
				reason: GatewaySyncPauseReason::GatewayStateDrift,
			}),
		);
		assert_eq!(
			TransferOutById::<Test>::get(transfer_id).map(|transfer| transfer.amount),
			Some(20_000),
		);
		assert_eq!(
			PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum),
			vec![PendingCollateralizationRequest::<Test> {
				transfer_id,
				remaining_collateral: 20_000,
				remaining_minting_authority_tip: 20,
			}],
		);
		assert_eq!(
			NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
			1,
		);
		assert_eq!(
			PendingTransferOutCirculationByDestinationChain::<Test>::get(SourceChain::Ethereum)
				.argon_circulation,
			20_000,
		);
		assert_eq!(Balances::balance(&burn_account), 0);
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::CrosschainTransfer(HoldReason::TransferOutMintingAuthorityTip,),
				&user,
			),
			20,
		);
	});
}

#[test]
fn prove_gateway_activity_pauses_without_advancing_and_refunds_submitter() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));

		let signing_key = council_signer(&council_signing_pair(55));
		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![minting_authority_activated_log(
				9_000,
				0,
				signing_key,
				account(56),
				1,
				1,
				1,
				4,
			)])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
		assert_eq!(
			GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewaySyncPause {
				last_good_gateway_activity_nonce: 0,
				failed_gateway_activity_nonce: 1,
				reason: GatewaySyncPauseReason::MintingAuthorityNotFound,
			}),
		);
		assert!(System::events().iter().any(|record| match &record.event {
			RuntimeEvent::CrosschainTransfer(Event::GatewaySyncPaused {
				source_chain: SourceChain::Ethereum,
				pause,
			}) => {
				*pause ==
					GatewaySyncPause {
						last_good_gateway_activity_nonce: 0,
						failed_gateway_activity_nonce: 1,
						reason: GatewaySyncPauseReason::MintingAuthorityNotFound,
					}
			},
			_ => false,
		}));
	});

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));
		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));

		GatewayStateBySourceChain::<Test>::insert(
			SourceChain::Ethereum,
			GatewayState::<Test> {
				gateway_activity_nonce: 0,
				argon_approvals_nonce: 0,
				argon_circulation: 10_000,
				argonot_circulation: 0,
			},
		);

		let recipient = account(57);
		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![transfer_to_argon_started_log(
				h160(0x21),
				h160(0x11),
				h160(0x31),
				1_250,
				destination_bytes(&recipient),
				1,
				0,
				0,
				0,
			)])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(Balances::balance(&recipient), 0);
		assert_eq!(
			GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewayState::<Test> {
				gateway_activity_nonce: 0,
				argon_approvals_nonce: 0,
				argon_circulation: 10_000,
				argonot_circulation: 0,
			}),
		);
		assert_eq!(
			GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewaySyncPause {
				last_good_gateway_activity_nonce: 0,
				failed_gateway_activity_nonce: 1,
				reason: GatewaySyncPauseReason::GatewayStateDrift,
			}),
		);
	});
}

#[test]
fn prove_gateway_activity_rejects_empty_proof_inputs() {
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
			Error::<Test>::NoGatewayProofBlocksProvided,
		);
		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![])]),
			),
			Error::<Test>::NoGatewayActivitiesProvided,
		);
	});
}

#[test]
fn prove_gateway_activity_accepts_anchor_target_block_number_for_anchor_path() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));
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
					8_750,
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
fn prove_gateway_activity_validate_marks_stale_cases() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));

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

	new_test_ext().execute_with(|| {
		GatewaySyncPauseBySourceChain::<Test>::insert(
			SourceChain::Ethereum,
			GatewaySyncPause {
				last_good_gateway_activity_nonce: 4,
				failed_gateway_activity_nonce: 5,
				reason: GatewaySyncPauseReason::GatewayStateDrift,
			},
		);

		let paused_call = RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
			source_chain: SourceChain::Ethereum,
			previous_gateway_activity_nonce: 4,
			proof_batch: proof_batch(vec![activity_logs(vec![argon_activity_log(
				account(3),
				5,
				750,
			)])]),
		});

		assert!(matches!(
			<CrosschainTransfer as CallTxValidityProvider<RuntimeCall, TestAccountId>>::validate(
				&paused_call,
				Some(&account(1)),
			),
			Err(TransactionValidityError::Invalid(InvalidTransaction::Stale))
		));
	});
}

#[test]
fn prove_gateway_activity_refunds_batch_that_advances_before_pausing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert_ok!(Balances::mint_into(&burn_account, 10_000));
		let recipient = account(2);
		let signing_key = council_signer(&council_signing_pair(55));

		let result = CrosschainTransfer::prove_gateway_activity(
			RuntimeOrigin::signed(account(1)),
			SourceChain::Ethereum,
			0,
			proof_batch(vec![activity_logs(vec![
				argon_activity_log(recipient.clone(), 1, 500),
				minting_authority_activated_log(9_000, 0, signing_key, account(56), 1, 1, 2, 4),
			])]),
		);

		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
		assert_eq!(Balances::balance(&recipient), 500);
		assert_eq!(
			GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewayState::<Test> {
				gateway_activity_nonce: 1,
				argon_approvals_nonce: 0,
				argon_circulation: 9_500,
				argonot_circulation: 0,
			}),
		);
		assert_eq!(
			GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
			Some(GatewaySyncPause {
				last_good_gateway_activity_nonce: 1,
				failed_gateway_activity_nonce: 2,
				reason: GatewaySyncPauseReason::GatewayStateDrift,
			}),
		);
	});
}

#[test]
fn prove_gateway_activity_accepts_contiguous_batch_across_proof_blocks() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));

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
				activity_logs(vec![argon_activity_log_with_circulation(
					second_recipient.clone(),
					2,
					750,
					8_750,
					0,
				)]),
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
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));

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
				argon_activity_log_with_circulation(second_recipient.clone(), 2, 600, 9_000, 0),
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
fn prove_gateway_activity_rejects_invalid_batches_without_partial_settlement() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));

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
			Error::<Test>::UnexpectedGatewayActivityNonce,
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
		ProofVerificationRejectedTransactionIndexes::set(vec![1]);

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
						vec![argon_activity_log_with_circulation(
							second_recipient.clone(),
							2,
							750,
							8_750,
							0,
						)],
						1,
					),
				]),
			),
			Error::<Test>::InvalidProof,
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
		ProofVerificationRejectedTransactionIndexes::set(vec![]);

		let first_recipient = account(16);
		let second_recipient = account(17);

		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![
					argon_activity_log(first_recipient.clone(), 1, 500),
					argon_activity_log_with_circulation(second_recipient.clone(), 3, 750, 8_750, 0),
				])]),
			),
			Error::<Test>::UnexpectedGatewayActivityNonce,
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
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));

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
					0,
					0,
				)])]),
			),
			Error::<Test>::UnsupportedGateway,
		);
	});
}

#[test]
fn prove_gateway_activity_rejects_invalid_gateway_activity_progression() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));

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
			Error::<Test>::UnexpectedPreviousGatewayActivityNonce,
		);
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
			Error::<Test>::UnexpectedGatewayActivityNonce,
		);
	});
}

#[test]
fn invalid_proof_from_provider_is_rejected() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));

		ProofVerificationAllowed::set(false);

		assert_noop!(
			CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![argon_activity_log(account(8), 1, 100)])]),
			),
			Error::<Test>::InvalidProof,
		);
	});
}

fn argon_activity_log(
	recipient: TestAccountId,
	gateway_activity_nonce: GatewayActivityNonce,
	amount: Balance,
) -> EthereumLog {
	let (argon_circulation, argonot_circulation) =
		current_gateway_circulation_after(Some((AssetKind::Argon, amount)), None);
	argon_activity_log_with_circulation(
		recipient,
		gateway_activity_nonce,
		amount,
		argon_circulation,
		argonot_circulation,
	)
}

fn argon_activity_log_with_circulation(
	recipient: TestAccountId,
	gateway_activity_nonce: GatewayActivityNonce,
	amount: Balance,
	argon_circulation: Balance,
	argonot_circulation: Balance,
) -> EthereumLog {
	transfer_to_argon_started_log(
		h160(0x21),
		h160(0x11),
		h160(0x31),
		amount,
		destination_bytes(&recipient),
		gateway_activity_nonce,
		0,
		argon_circulation,
		argonot_circulation,
	)
}

fn argonot_activity_log(
	recipient: TestAccountId,
	gateway_activity_nonce: GatewayActivityNonce,
	amount: Balance,
) -> EthereumLog {
	let (argon_circulation, argonot_circulation) =
		current_gateway_circulation_after(Some((AssetKind::Argonot, amount)), None);
	transfer_to_argon_started_log(
		h160(0x21),
		h160(0x11),
		h160(0x32),
		amount,
		destination_bytes(&recipient),
		gateway_activity_nonce,
		0,
		argon_circulation,
		argonot_circulation,
	)
}

#[allow(clippy::too_many_arguments)]
fn minting_authority_activated_log(
	microgon_collateral: Balance,
	micronot_collateral: Balance,
	destination_signing_key: H160,
	relayer_argon_account_id: TestAccountId,
	coactivation_count: u32,
	shared_signature_count: u32,
	gateway_activity_nonce: GatewayActivityNonce,
	argon_approvals_nonce: ArgonApprovalsNonce,
) -> EthereumLog {
	let (argon_circulation, argonot_circulation) = current_gateway_circulation_after(None, None);
	let mut data = Vec::with_capacity(288);
	data.extend_from_slice(&u128_word(microgon_collateral));
	data.extend_from_slice(&u128_word(micronot_collateral));
	data.extend_from_slice(&u64_word(coactivation_count as u64));
	data.extend_from_slice(&u64_word(shared_signature_count as u64));
	data.extend_from_slice(&destination_bytes(&relayer_argon_account_id));
	data.extend_from_slice(&u64_word(gateway_activity_nonce));
	data.extend_from_slice(&u64_word(argon_approvals_nonce));
	data.extend_from_slice(&u128_word(argon_circulation));
	data.extend_from_slice(&u128_word(argonot_circulation));

	EthereumLog {
		address: h160(0x21),
		topics: vec![
			H256::from_slice(MintingAuthorityActivated::SIGNATURE_HASH.as_slice()),
			indexed_address_word(destination_signing_key),
		]
		.try_into()
		.expect("topics stay within Ethereum log topic bounds"),
		data: data
			.try_into()
			.expect("minting-authority activation data stays within bounded log payload"),
	}
}

fn minting_authority_deactivated_log(
	microgon_collateral: Balance,
	micronot_collateral: Balance,
	destination_signing_key: H160,
	relayer_argon_account_id: TestAccountId,
	gateway_activity_nonce: GatewayActivityNonce,
	argon_approvals_nonce: ArgonApprovalsNonce,
) -> EthereumLog {
	let (argon_circulation, argonot_circulation) = current_gateway_circulation_after(None, None);
	let mut data = Vec::with_capacity(192);
	data.extend_from_slice(&u128_word(microgon_collateral));
	data.extend_from_slice(&u128_word(micronot_collateral));
	data.extend_from_slice(&destination_bytes(&relayer_argon_account_id));
	data.extend_from_slice(&u64_word(gateway_activity_nonce));
	data.extend_from_slice(&u64_word(argon_approvals_nonce));
	data.extend_from_slice(&u128_word(argon_circulation));
	data.extend_from_slice(&u128_word(argonot_circulation));

	EthereumLog {
		address: h160(0x21),
		topics: vec![
			H256::from_slice(MintingAuthorityDeactivated::SIGNATURE_HASH.as_slice()),
			indexed_address_word(destination_signing_key),
		]
		.try_into()
		.expect("topics stay within Ethereum log topic bounds"),
		data: data
			.try_into()
			.expect("minting-authority deactivation data stays within bounded log payload"),
	}
}

fn transfer_out_of_argon_finalized_log(
	transfer_id: H256,
	minting_collateral: Vec<(H160, Balance, Balance)>,
	gateway_activity_nonce: GatewayActivityNonce,
	argon_approvals_nonce: ArgonApprovalsNonce,
) -> EthereumLog {
	let finalized_transfer = TransferOutById::<Test>::get(transfer_id)
		.expect("finalized transfer should still be present when building event");
	let (argon_circulation, argonot_circulation) = current_gateway_circulation_after(
		None,
		Some((finalized_transfer.asset, finalized_transfer.amount)),
	);
	transfer_out_of_argon_finalized_log_with_circulation(
		transfer_id,
		finalized_transfer.asset,
		finalized_transfer.amount,
		minting_collateral,
		gateway_activity_nonce,
		argon_approvals_nonce,
		argon_circulation,
		argonot_circulation,
	)
}

#[allow(clippy::too_many_arguments)]
fn transfer_out_of_argon_finalized_log_with_circulation(
	transfer_id: H256,
	asset: AssetKind,
	amount: Balance,
	minting_collateral: Vec<(H160, Balance, Balance)>,
	gateway_activity_nonce: GatewayActivityNonce,
	argon_approvals_nonce: ArgonApprovalsNonce,
	argon_circulation: Balance,
	argonot_circulation: Balance,
) -> EthereumLog {
	let event = ContractTransferOutOfArgonFinalized {
		transferId: alloy_primitives::B256::from(transfer_id.0),
		token: AlloyAddress::from_slice(
			match asset {
				AssetKind::Argon => h160(0x31),
				AssetKind::Argonot => h160(0x32),
			}
			.as_bytes(),
		),
		amount,
		mintingCollateral: minting_collateral
			.into_iter()
			.map(|(signing_key, microgon_collateral, micronot_collateral)| {
				ContractMintingAuthorityCollateral {
					signingKey: AlloyAddress::from_slice(signing_key.as_bytes()),
					microgonCollateral: microgon_collateral,
					micronotCollateral: micronot_collateral,
				}
			})
			.collect(),
		gatewayState: contract_gateway_activity_state(
			gateway_activity_nonce,
			argon_approvals_nonce,
			argon_circulation,
			argonot_circulation,
		),
	};

	EthereumLog {
		address: h160(0x21),
		topics: vec![H256::from_slice(
			ContractTransferOutOfArgonFinalized::SIGNATURE_HASH.as_slice(),
		)]
		.try_into()
		.expect("topics stay within Ethereum log topic bounds"),
		data: event
			.encode_data()
			.try_into()
			.expect("transfer-out finalized data stays within bounded log payload"),
	}
}

fn transfer_out_of_argon_canceled_log(
	transfer_id: H256,
	gateway_activity_nonce: GatewayActivityNonce,
	argon_approvals_nonce: ArgonApprovalsNonce,
) -> EthereumLog {
	let (argon_circulation, argonot_circulation) = current_gateway_circulation_after(None, None);
	let event = ContractTransferOutOfArgonCanceled {
		transferId: alloy_primitives::B256::from(transfer_id.0),
		gatewayState: contract_gateway_activity_state(
			gateway_activity_nonce,
			argon_approvals_nonce,
			argon_circulation,
			argonot_circulation,
		),
	};

	EthereumLog {
		address: h160(0x21),
		topics: vec![H256::from_slice(
			ContractTransferOutOfArgonCanceled::SIGNATURE_HASH.as_slice(),
		)]
		.try_into()
		.expect("topics stay within Ethereum log topic bounds"),
		data: event
			.encode_data()
			.try_into()
			.expect("transfer-out canceled data stays within bounded log payload"),
	}
}

#[allow(clippy::too_many_arguments)]
fn transfer_to_argon_started_log(
	gateway: H160,
	from: H160,
	token: H160,
	amount: Balance,
	destination: [u8; 32],
	gateway_activity_nonce: GatewayActivityNonce,
	argon_approvals_nonce: ArgonApprovalsNonce,
	argon_circulation: Balance,
	argonot_circulation: Balance,
) -> EthereumLog {
	let mut data = Vec::with_capacity(192);
	data.extend_from_slice(&u128_word(amount));
	data.extend_from_slice(&destination);
	data.extend_from_slice(&u64_word(gateway_activity_nonce));
	data.extend_from_slice(&u64_word(argon_approvals_nonce));
	data.extend_from_slice(&u128_word(argon_circulation));
	data.extend_from_slice(&u128_word(argonot_circulation));

	EthereumLog {
		address: gateway,
		topics: vec![
			H256::from_slice(TransferToArgonStarted::SIGNATURE_HASH.as_slice()),
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

fn current_gateway_circulation_after(
	decrease: Option<(AssetKind, Balance)>,
	increase: Option<(AssetKind, Balance)>,
) -> (Balance, Balance) {
	let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
	let pending =
		PendingTransferOutCirculationByDestinationChain::<Test>::get(SourceChain::Ethereum);
	let mut argon_circulation =
		Balances::balance(&burn_account).saturating_sub(pending.argon_circulation);
	let mut argonot_circulation =
		Ownership::balance(&burn_account).saturating_sub(pending.argonot_circulation);

	if let Some((asset, amount)) = decrease {
		match asset {
			AssetKind::Argon => argon_circulation = argon_circulation.saturating_sub(amount),
			AssetKind::Argonot => argonot_circulation = argonot_circulation.saturating_sub(amount),
		}
	}

	if let Some((asset, amount)) = increase {
		match asset {
			AssetKind::Argon => argon_circulation = argon_circulation.saturating_add(amount),
			AssetKind::Argonot => argonot_circulation = argonot_circulation.saturating_add(amount),
		}
	}

	(argon_circulation, argonot_circulation)
}

fn contract_gateway_activity_state(
	gateway_activity_nonce: GatewayActivityNonce,
	argon_approvals_nonce: ArgonApprovalsNonce,
	argon_circulation: Balance,
	argonot_circulation: Balance,
) -> ContractGatewayActivityState {
	ContractGatewayActivityState {
		gatewayActivityNonce: gateway_activity_nonce,
		argonApprovalsNonce: argon_approvals_nonce,
		argonCirculation: argon_circulation,
		argonotCirculation: argonot_circulation,
	}
}

fn activity_logs(
	logs: Vec<EthereumLog>,
) -> BoundedVec<EthereumReceiptLog, MaxActivitiesPerReceiptProof> {
	activity_logs_for_transaction_index(logs, 0)
}

fn activity_logs_for_transaction_index(
	logs: Vec<EthereumLog>,
	transaction_index: u64,
) -> BoundedVec<EthereumReceiptLog, MaxActivitiesPerReceiptProof> {
	logs.into_iter()
		.map(|event_log| EthereumReceiptLog { transaction_index, event_log })
		.collect::<Vec<_>>()
		.try_into()
		.expect("test gateway activity logs stay within pallet bound")
}

fn proof_batch(
	log_blocks: Vec<BoundedVec<EthereumReceiptLog, MaxActivitiesPerReceiptProof>>,
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
