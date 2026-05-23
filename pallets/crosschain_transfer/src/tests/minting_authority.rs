use super::*;

#[test]
fn register_minting_authority_validates_inputs_encumbers_collateral_and_queues_approval() {
	new_test_ext().execute_with(|| {
		let owner_vault_operator = account(30);
		let council_pair = council_signing_pair(11);
		let minting_authority_pair = council_signing_pair(12);
		let wrong_pair = council_signing_pair(13);
		let signing_key = council_signer(&minting_authority_pair);

		configure_single_member_ethereum_council(
			owner_vault_operator.clone(),
			7,
			10_000,
			&council_pair,
		);
		assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
		assert_ok!(Ownership::mint_into(&owner_vault_operator, 900));
		assert_ok!(set_committed_argonots(owner_vault_operator.clone(), 500));
		assert_noop!(
			CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(&wrong_pair, &owner_vault_operator),
				10_000,
				300,
			),
			Error::<Test>::InvalidMintingAuthoritySigningKeyProof,
		);
		set_active_bond_amount(7, owner_vault_operator.clone(), 500);
		assert_noop!(
			CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				10_000,
				300,
			),
			Error::<Test>::InsufficientCommittedMicrogonCollateral,
		);
		set_active_bond_amount(7, owner_vault_operator.clone(), 10_000);

		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			signing_key,
			minting_authority_registration_signature(
				&minting_authority_pair,
				&owner_vault_operator,
			),
			10_000,
			300,
		));
		assert_eq!(
			MintingAuthoritiesBySigner::<Test>::get(signing_key),
			Some(MintingAuthority::<Test> {
				account_id: owner_vault_operator.clone(),
				destination_chain: SourceChain::Ethereum,
				destination_signing_key: signing_key,
				state: MintingAuthorityState::PendingActivation,
				gateway_remaining_microgon_collateral: 10_000,
				gateway_remaining_micronot_collateral: 300,
				pending_reserved_microgon_collateral: 0,
				pending_reserved_micronot_collateral: 0,
				active_pending_transfer_ids: bounded_vec![],
				activation_approval_queue_nonce: 1,
				activation_base_repayment_due: Some(100),
				activation_signature_repayment_due: Some(50),
				activation_repayment_due: Some(150),
				deactivation_approval_queue_nonce: None,
			}),
		);
		assert_eq!(encumbered_bond_microgons(&owner_vault_operator), 10_000);
		assert_eq!(encumbered_argonot_micronots(&owner_vault_operator), 300);
		assert_noop!(set_committed_argonots(owner_vault_operator.clone(), 299), TokenError::Frozen,);

		let queue_entry =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 1)
				.expect("queue entry should be stored");
		assert_eq!(
			queue_entry.approving_council_hash,
			ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
				.expect("council should stay active")
		);
		assert_eq!(
			queue_entry.target,
			CouncilApprovalTargetId::MintingAuthorityActivation(signing_key),
		);
		assert_eq!(
			queue_entry.target_payload_hash,
			sp_core::H256::from_slice(
				ethereum_contracts::hash_activate_minting_authority(
					1,
					AlloyAddress::from_slice(h160(0x21).as_bytes()),
					10_000,
					300,
					AlloyAddress::from_slice(signing_key.as_bytes()),
				)
				.as_slice(),
			),
		);
		assert_eq!(queue_entry.previous_approval_hash, sp_core::H256::zero(),);
		assert_ne!(queue_entry.approval_hash, sp_core::H256::zero(),);
		assert_eq!(
			CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				owner_vault_operator.clone(),
			),
			Some(0),
		);
		assert_noop!(
			CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				2_000,
				100,
			),
			Error::<Test>::MintingAuthorityAlreadyRegistered,
		);
	});
}

#[test]
fn register_minting_authority_uses_per_chain_minimum_override() {
	new_test_ext().execute_with(|| {
		let owner_vault_operator = account(31);
		let council_pair = council_signing_pair(14);
		let minting_authority_pair = council_signing_pair(15);
		let signing_key = council_signer(&minting_authority_pair);

		configure_single_member_ethereum_council(
			owner_vault_operator.clone(),
			17,
			10_000,
			&council_pair,
		);
		assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
		assert_ok!(Ownership::mint_into(&owner_vault_operator, 900));
		assert_ok!(set_committed_argonots(owner_vault_operator.clone(), 300));

		assert_noop!(
			CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				8_000,
				300,
			),
			Error::<Test>::MintingAuthorityCollateralBelowMinimum,
		);
		assert_ok!(CrosschainTransfer::set_minimum_minting_authority_value(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			8_300,
		));
		assert_eq!(
			MinimumMintingAuthorityValueByDestinationChain::<Test>::get(SourceChain::Ethereum),
			8_300,
		);
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			signing_key,
			minting_authority_registration_signature(
				&minting_authority_pair,
				&owner_vault_operator,
			),
			10_000,
			300,
		));
	});
}

#[test]
fn deactivate_minting_authority_moves_authority_into_deactivating_and_queues_entry() {
	new_test_ext().execute_with(|| {
		let authority_account = account(34);
		let authority_pair = council_signing_pair(24);
		let signing_key = activate_test_minting_authority(
			authority_account.clone(),
			24,
			20_000,
			&council_signing_pair(23),
			&authority_pair,
			10_000,
			0,
		);
		let deactivation_signature = minting_authority_deactivation_signature(
			&authority_pair,
			2,
			signing_key,
			CrosschainTransfer::previous_gateway_update_hash(SourceChain::Ethereum, 2)
				.expect("activation queue entry should anchor deactivation"),
		);

		assert_ok!(CrosschainTransfer::deactivate_minting_authority(
			RuntimeOrigin::signed(authority_account.clone()),
			signing_key,
			deactivation_signature,
		));

		let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
			.expect("authority should remain until proof-backed deactivation");
		assert_eq!(authority.account_id, authority_account);
		assert_eq!(authority.state, MintingAuthorityState::Deactivating);
		assert_eq!(authority.deactivation_approval_queue_nonce, Some(2));

		let deactivation_entry =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 2)
				.expect("deactivation queue entry should be stored");
		assert_eq!(
			deactivation_entry.target,
			CouncilApprovalTargetId::MintingAuthorityDeactivation(signing_key),
		);
		assert_eq!(
			deactivation_entry.target_payload_hash,
			CrosschainTransfer::hash_deactivate_minting_authority_target(signing_key),
		);
		assert_eq!(deactivation_entry.signatures.get(&signing_key), Some(&deactivation_signature),);
	});
}

#[test]
fn deactivate_minting_authority_skips_council_cursor_blocking() {
	new_test_ext().execute_with(|| {
		let authority_account = account(35);
		let council_pair = council_signing_pair(22);
		let authority_pair = council_signing_pair(25);
		let second_authority_pair = council_signing_pair(26);
		let signing_key = activate_test_minting_authority(
			authority_account.clone(),
			25,
			30_000,
			&council_pair,
			&authority_pair,
			10_000,
			0,
		);
		let user = account(36);

		assert_ok!(Balances::mint_into(&user, 6_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x72),
			5_000,
		));
		let transfer_id = transfer_out_id(&user, 1);
		assert_ok!(CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(authority_account.clone()),
			transfer_id,
			transfer_collateral_signature(&authority_pair, transfer_id, 5_000, 0),
			5_000,
			0,
		));
		let previous_approval_hash =
			CrosschainTransfer::previous_gateway_update_hash(SourceChain::Ethereum, 2)
				.expect("activation queue entry should anchor deactivation");
		let deactivation_signature = minting_authority_deactivation_signature(
			&authority_pair,
			2,
			signing_key,
			previous_approval_hash,
		);
		assert_ok!(CrosschainTransfer::deactivate_minting_authority(
			RuntimeOrigin::signed(authority_account.clone()),
			signing_key,
			deactivation_signature,
		));

		let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
			.expect("authority should remain until proof-backed deactivation");
		assert_eq!(authority.state, MintingAuthorityState::Deactivating);
		assert_eq!(authority.deactivation_approval_queue_nonce, Some(2));

		let deactivation_entry =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 2)
				.expect("deactivation queue entry should be stored");
		assert_eq!(
			deactivation_entry.target,
			CouncilApprovalTargetId::MintingAuthorityDeactivation(signing_key),
		);
		assert_eq!(
			deactivation_entry.target_payload_hash,
			CrosschainTransfer::hash_deactivate_minting_authority_target(signing_key),
		);
		assert_eq!(deactivation_entry.signatures.get(&signing_key), Some(&deactivation_signature),);
		assert_eq!(
			MintingAuthoritiesBySigner::<Test>::get(signing_key)
				.expect("authority should still exist before proof-back")
				.active_pending_transfer_ids,
			vec![transfer_id],
			"deactivation should queue immediately and let proof-back invalidate later",
		);

		let second_signing_key = council_signer(&second_authority_pair);
		set_active_bond_amount(25, authority_account.clone(), 20_000);
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(authority_account.clone()),
			SourceChain::Ethereum,
			second_signing_key,
			minting_authority_registration_signature(&second_authority_pair, &authority_account),
			10_000,
			0,
		));
		assert_ok!(CrosschainTransfer::approve_queue_entries(
			RuntimeOrigin::signed(authority_account.clone()),
			SourceChain::Ethereum,
			bounded_vec![minting_authority_approval_signature(&council_pair, 3)],
		));
		assert_eq!(
			CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				authority_account,
			),
			Some(3),
		);
	});
}

#[test]
fn approve_queue_entries_validates_signature_and_records_council_quorum() {
	new_test_ext().execute_with(|| {
		let owner_vault_operator = account(30);
		let council_pair = council_signing_pair(1);
		let wrong_pair = council_signing_pair(9);
		let council_address = council_signer(&council_pair);
		let minting_authority_pair = council_signing_pair(2);
		let signing_key = council_signer(&minting_authority_pair);

		configure_single_member_ethereum_council(
			owner_vault_operator.clone(),
			7,
			10_000,
			&council_pair,
		);
		assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
		assert_ok!(Ownership::mint_into(&owner_vault_operator, 900));
		assert_ok!(set_committed_argonots(owner_vault_operator.clone(), 500));
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			signing_key,
			minting_authority_registration_signature(
				&minting_authority_pair,
				&owner_vault_operator,
			),
			10_000,
			300,
		));
		assert_noop!(
			CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&wrong_pair, 1)],
			),
			Error::<Test>::InvalidCouncilApprovalSignature,
		);
		let result = CrosschainTransfer::approve_queue_entries(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			bounded_vec![minting_authority_approval_signature(&council_pair, 1)],
		);
		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));

		let queue_entry =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 1)
				.expect("queue entry should stay available");
		assert_eq!(queue_entry.approved_total_weight, 10_000);
		assert_eq!(queue_entry.signatures.len(), 1);
		assert!(queue_entry.signatures.contains_key(&council_address));
		assert_eq!(
			CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				owner_vault_operator,
			),
			Some(1),
		);
	});
}

#[test]
fn approve_queue_entries_keeps_non_signers_blocked_until_gateway_sync_advances() {
	new_test_ext().execute_with(|| {
		let first_council_account = account(31);
		let second_council_account = account(32);
		let third_council_account = account(33);
		let first_council_pair = council_signing_pair(11);
		let second_council_pair = council_signing_pair(12);
		let third_council_pair = council_signing_pair(13);
		let minting_authority_pair = council_signing_pair(14);
		let first_council_signer = council_signer(&first_council_pair);
		let second_council_signer = council_signer(&second_council_pair);
		let third_council_signer = council_signer(&third_council_pair);
		let signing_key = council_signer(&minting_authority_pair);

		register_vault_operator(first_council_account.clone(), 21, 50);
		register_vault_operator(second_council_account.clone(), 22, 30);
		register_vault_operator(third_council_account.clone(), 23, 20);
		assert_ok!(Balances::mint_into(&first_council_account, 10_000));
		assert_ok!(Ownership::mint_into(&first_council_account, 900));
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));
		assert_ok!(CrosschainTransfer::set_minting_authority_activation_repayment_pricing(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			MintingAuthorityActivationRepaymentPricing::<Test> {
				activation_gas_cost: 100_000,
				signature_gas_cost: 50_000,
				estimated_wei_per_gas: 1_000_000_000,
				estimated_microgons_per_eth: 1_000_000,
			},
		));
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(first_council_account.clone()),
			SourceChain::Ethereum,
			first_council_signer,
			council_signer_registration_signature(&first_council_pair, &first_council_account),
		));
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(second_council_account.clone()),
			SourceChain::Ethereum,
			second_council_signer,
			council_signer_registration_signature(&second_council_pair, &second_council_account),
		));
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(third_council_account.clone()),
			SourceChain::Ethereum,
			third_council_signer,
			council_signer_registration_signature(&third_council_pair, &third_council_account),
		));
		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![
				first_council_account.clone(),
				second_council_account.clone(),
				third_council_account.clone(),
			]
			.try_into()
			.expect("three council members stay within limit"),
		));
		set_active_bond_amount(21, first_council_account.clone(), 10_000);
		assert_ok!(set_committed_argonots(first_council_account.clone(), 500));
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(first_council_account.clone()),
			SourceChain::Ethereum,
			signing_key,
			minting_authority_registration_signature(
				&minting_authority_pair,
				&first_council_account,
			),
			10_000,
			300,
		));

		let queue_entry =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 1)
				.expect("queue entry should exist");
		assert!(
			!<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
				&first_council_account,
			)
		);
		CurrentFrameId::set(queue_entry.due_frame_id);
		assert!(
			<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
				&first_council_account,
			)
		);
		assert!(
			<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
				&second_council_account,
			)
		);
		assert!(
			<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
				&third_council_account,
			)
		);

		assert_ok!(CrosschainTransfer::approve_queue_entries(
			RuntimeOrigin::signed(first_council_account.clone()),
			SourceChain::Ethereum,
			bounded_vec![minting_authority_approval_signature(&first_council_pair, 1)],
		));
		assert!(
			!<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
				&first_council_account,
			)
		);
		assert!(
			<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
				&second_council_account,
			)
		);
		assert!(
			<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
				&third_council_account,
			)
		);

		assert_ok!(CrosschainTransfer::approve_queue_entries(
			RuntimeOrigin::signed(second_council_account.clone()),
			SourceChain::Ethereum,
			bounded_vec![minting_authority_approval_signature(&second_council_pair, 1)],
		));
		assert!(
			!<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
				&second_council_account,
			)
		);
		assert!(
			<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
				&third_council_account,
			)
		);
	});
}

#[test]
fn approve_queue_entries_applies_contiguous_batch_fee_free() {
	new_test_ext().execute_with(|| {
		let owner_vault_operator = account(34);
		let council_pair = council_signing_pair(15);
		let council_address = council_signer(&council_pair);
		let first_minting_authority_pair = council_signing_pair(16);
		let second_minting_authority_pair = council_signing_pair(17);
		let first_signing_key = council_signer(&first_minting_authority_pair);
		let second_signing_key = council_signer(&second_minting_authority_pair);

		register_vault_operator(owner_vault_operator.clone(), 24, 20_000);
		assert_ok!(Balances::mint_into(&owner_vault_operator, 20_000));
		assert_ok!(Ownership::mint_into(&owner_vault_operator, 1_000));
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));
		assert_ok!(CrosschainTransfer::set_minting_authority_activation_repayment_pricing(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			MintingAuthorityActivationRepaymentPricing::<Test> {
				activation_gas_cost: 100_000,
				signature_gas_cost: 50_000,
				estimated_wei_per_gas: 1_000_000_000,
				estimated_microgons_per_eth: 1_000_000,
			},
		));
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			council_address,
			council_signer_registration_signature(&council_pair, &owner_vault_operator),
		));
		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![owner_vault_operator.clone()]
				.try_into()
				.expect("single council member stays within limit"),
		));
		assert_ok!(set_committed_argonots(owner_vault_operator.clone(), 600));
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			first_signing_key,
			minting_authority_registration_signature(
				&first_minting_authority_pair,
				&owner_vault_operator,
			),
			10_000,
			200,
		));
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(owner_vault_operator.clone()),
			SourceChain::Ethereum,
			second_signing_key,
			minting_authority_registration_signature(
				&second_minting_authority_pair,
				&owner_vault_operator,
			),
			10_000,
			100,
		));

		let result = CrosschainTransfer::approve_queue_entries(
			RuntimeOrigin::signed(owner_vault_operator),
			SourceChain::Ethereum,
			bounded_vec![
				minting_authority_approval_signature(&council_pair, 1),
				minting_authority_approval_signature(&council_pair, 2)
			],
		);
		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));

		let first_queue_entry =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 1)
				.expect("first queue entry should stay available");
		let second_queue_entry =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 2)
				.expect("second queue entry should stay available");
		assert_eq!(first_queue_entry.approved_total_weight, 20_000);
		assert_eq!(second_queue_entry.approved_total_weight, 20_000);
		assert_eq!(first_queue_entry.signatures.len(), 1);
		assert_eq!(second_queue_entry.signatures.len(), 1);
		assert_eq!(
			CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				account(34),
			),
			Some(2),
		);
	});
}

#[test]
fn force_set_global_issuance_council_rebases_queued_approving_council() {
	new_test_ext().execute_with(|| {
		let queued_council_account = account(42);
		let replacement_council_account = account(43);
		let queued_council_pair = council_signing_pair(61);
		let replacement_council_pair = council_signing_pair(62);
		let minting_authority_pair = council_signing_pair(63);
		let queued_council_signer = council_signer(&queued_council_pair);
		let replacement_council_signer = council_signer(&replacement_council_pair);
		let signing_key = council_signer(&minting_authority_pair);

		register_vault_operator(queued_council_account.clone(), 13, 8_000);
		register_vault_operator(replacement_council_account.clone(), 14, 7_000);
		assert_ok!(Balances::mint_into(&queued_council_account, 10_000));
		assert_ok!(Ownership::mint_into(&queued_council_account, 500));
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));
		assert_ok!(CrosschainTransfer::set_minting_authority_activation_repayment_pricing(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			MintingAuthorityActivationRepaymentPricing::<Test> {
				activation_gas_cost: 100_000,
				signature_gas_cost: 50_000,
				estimated_wei_per_gas: 1_000_000_000,
				estimated_microgons_per_eth: 1_000_000,
			},
		));
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(queued_council_account.clone()),
			SourceChain::Ethereum,
			queued_council_signer,
			council_signer_registration_signature(&queued_council_pair, &queued_council_account,),
		));
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(replacement_council_account.clone()),
			SourceChain::Ethereum,
			replacement_council_signer,
			council_signer_registration_signature(
				&replacement_council_pair,
				&replacement_council_account,
			),
		));
		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![queued_council_account.clone()]
				.try_into()
				.expect("single council member stays within limit"),
		));
		assert_ok!(CrosschainTransfer::set_minimum_minting_authority_value(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			4_000,
		));
		assert_ok!(set_committed_argonots(queued_council_account.clone(), 300));
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(queued_council_account.clone()),
			SourceChain::Ethereum,
			signing_key,
			minting_authority_registration_signature(
				&minting_authority_pair,
				&queued_council_account,
			),
			4_000,
			100,
		));

		let queued_approving_council_hash =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 1)
				.expect("queued approval should exist")
				.approving_council_hash;
		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![replacement_council_account.clone()]
				.try_into()
				.expect("single council member stays within limit"),
		));
		assert_ne!(
			ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
				.expect("replacement council should be active"),
			queued_approving_council_hash,
		);

		let replacement_signature =
			minting_authority_approval_signature(&replacement_council_pair, 1);
		assert_ok!(CrosschainTransfer::approve_queue_entries(
			RuntimeOrigin::signed(replacement_council_account),
			SourceChain::Ethereum,
			bounded_vec![replacement_signature],
		));

		let queued_signature = minting_authority_approval_signature(&queued_council_pair, 1);
		assert_noop!(
			CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(queued_council_account),
				SourceChain::Ethereum,
				bounded_vec![queued_signature],
			),
			Error::<Test>::GlobalIssuanceCouncilMemberNotFound,
		);
	});
}

#[test]
fn force_set_global_issuance_council_rebases_unresolved_queue_entries() {
	new_test_ext().execute_with(|| {
		let original_council_account = account(44);
		let replacement_council_account = account(45);
		let original_council_pair = council_signing_pair(71);
		let replacement_council_pair = council_signing_pair(72);
		let first_minting_authority_pair = council_signing_pair(73);
		let second_minting_authority_pair = council_signing_pair(74);
		let original_council_signer = council_signer(&original_council_pair);
		let replacement_council_signer = council_signer(&replacement_council_pair);
		let first_signing_key = council_signer(&first_minting_authority_pair);
		let second_signing_key = council_signer(&second_minting_authority_pair);

		register_vault_operator(original_council_account.clone(), 15, 12_000);
		register_vault_operator(replacement_council_account.clone(), 16, 11_000);
		assert_ok!(Balances::mint_into(&original_council_account, 20_000));
		assert_ok!(Ownership::mint_into(&original_council_account, 2_000));
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));
		assert_ok!(CrosschainTransfer::set_minting_authority_activation_repayment_pricing(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			MintingAuthorityActivationRepaymentPricing::<Test> {
				activation_gas_cost: 100_000,
				signature_gas_cost: 50_000,
				estimated_wei_per_gas: 1_000_000_000,
				estimated_microgons_per_eth: 1_000_000,
			},
		));
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(original_council_account.clone()),
			SourceChain::Ethereum,
			original_council_signer,
			council_signer_registration_signature(
				&original_council_pair,
				&original_council_account,
			),
		));
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(replacement_council_account.clone()),
			SourceChain::Ethereum,
			replacement_council_signer,
			council_signer_registration_signature(
				&replacement_council_pair,
				&replacement_council_account,
			),
		));
		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![original_council_account.clone()]
				.try_into()
				.expect("single council member stays within limit"),
		));
		assert_ok!(CrosschainTransfer::set_minimum_minting_authority_value(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			3_000,
		));
		assert_ok!(set_committed_argonots(original_council_account.clone(), 500));

		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(original_council_account.clone()),
			SourceChain::Ethereum,
			first_signing_key,
			minting_authority_registration_signature(
				&first_minting_authority_pair,
				&original_council_account,
			),
			4_000,
			100,
		));
		assert_ok!(CrosschainTransfer::register_minting_authority(
			RuntimeOrigin::signed(original_council_account.clone()),
			SourceChain::Ethereum,
			second_signing_key,
			minting_authority_registration_signature(
				&second_minting_authority_pair,
				&original_council_account,
			),
			3_000,
			50,
		));
		assert_ok!(CrosschainTransfer::approve_queue_entries(
			RuntimeOrigin::signed(original_council_account.clone()),
			SourceChain::Ethereum,
			bounded_vec![minting_authority_approval_signature(&original_council_pair, 1)],
		));

		let first_queue_entry_before =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 1)
				.expect("first queue entry should exist");
		let second_queue_entry_before =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 2)
				.expect("second queue entry should exist");
		assert_eq!(
			CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				original_council_account.clone(),
			),
			Some(1),
		);

		assert_noop!(
			CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![replacement_council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			),
			Error::<Test>::CannotForceSetQuorumApprovedQueueEntry,
		);

		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			1,
			vec![replacement_council_account.clone()]
				.try_into()
				.expect("single council member stays within limit"),
		));

		let active_council =
			ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
				.expect("replacement council should be active");
		let first_queue_entry_after =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 1)
				.expect("first queue entry should stay queued");
		let second_queue_entry_after =
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(SourceChain::Ethereum, 2)
				.expect("second queue entry should stay queued");

		assert_eq!(
			first_queue_entry_after.approving_council_hash,
			first_queue_entry_before.approving_council_hash
		);
		assert_eq!(second_queue_entry_after.approving_council_hash, active_council);
		assert_eq!(
			first_queue_entry_after.previous_approval_hash,
			first_queue_entry_before.previous_approval_hash
		);
		assert_eq!(
			second_queue_entry_after.previous_approval_hash,
			first_queue_entry_after.approval_hash,
		);
		assert_eq!(
			first_queue_entry_after.approved_total_weight,
			first_queue_entry_before.approved_total_weight
		);
		assert_eq!(second_queue_entry_after.approved_total_weight, 0);
		assert_eq!(
			first_queue_entry_after.signatures.len(),
			first_queue_entry_before.signatures.len()
		);
		assert!(second_queue_entry_after.signatures.is_empty());
		assert_eq!(first_queue_entry_after.approval_hash, first_queue_entry_before.approval_hash,);
		assert_ne!(second_queue_entry_after.approval_hash, second_queue_entry_before.approval_hash,);
		assert_eq!(
			CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				replacement_council_account.clone(),
			),
			Some(1),
		);

		assert_noop!(
			CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(original_council_account.clone()),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&original_council_pair, 1)],
			),
			Error::<Test>::GlobalIssuanceCouncilMemberNotFound,
		);
		assert_noop!(
			CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(original_council_account),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&original_council_pair, 2)],
			),
			Error::<Test>::GlobalIssuanceCouncilMemberNotFound,
		);
		assert_ok!(CrosschainTransfer::approve_queue_entries(
			RuntimeOrigin::signed(replacement_council_account),
			SourceChain::Ethereum,
			bounded_vec![minting_authority_approval_signature(&replacement_council_pair, 2)],
		));
	});
}
