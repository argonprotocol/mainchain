use super::*;
use crate::gateway_activity::GatewayMintingAuthorityCollateral;

#[test]
fn transfer_out_moves_principal_to_burn_and_holds_tip() {
	new_test_ext().execute_with(|| {
		let council_account = account(60);
		let council_pair = council_signing_pair(80);
		let user = account(61);

		configure_single_member_ethereum_council(council_account, 40, 20_000, &council_pair);
		assert_ok!(Balances::mint_into(&user, 25_000));

		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x44),
			20_000,
		));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		let transfer_id = transfer_out_id(&user, 1);
		assert_eq!(Balances::balance(&burn_account), 20_000);
		assert_eq!(Balances::balance(&user), 4_980);
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::CrosschainTransfer(HoldReason::TransferOutMintingAuthorityTip,),
				&user,
			),
			20,
		);
		let transfer =
			TransferOutById::<Test>::get(transfer_id).expect("transfer should be stored");
		assert_eq!(transfer.argon_account_id, user.clone());
		assert_eq!(transfer.argon_transfer_nonce, 1);
		assert_eq!(transfer.destination_chain, SourceChain::Ethereum);
		assert_eq!(
			transfer.council_hash,
			ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
				.expect("active council should be present"),
		);
		assert_eq!(transfer.destination_account, h160(0x44));
		assert_eq!(transfer.valid_until_ethereum_block, 73_000);
		assert_eq!(transfer.asset, AssetKind::Argon);
		assert_eq!(transfer.amount, 20_000);
		assert_eq!(transfer.minting_authority_tip, 20);
		assert_eq!(transfer.total_attached_collateral, 0);
		assert!(transfer.minting_authority_collateral_by_signer.is_empty());
		assert_eq!(transfer.state, TransferOutState::Started);
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
	});
}

#[test]
fn transfer_out_rejects_when_non_terminal_request_cap_is_reached() {
	new_test_ext().execute_with(|| {
		let council_account = account(160);
		let council_pair = council_signing_pair(180);
		let user = account(161);

		configure_single_member_ethereum_council(council_account, 140, 20_000, &council_pair);
		assert_ok!(Balances::mint_into(&user, 25_000));
		NonTerminalTransferOutCountByDestinationChain::<Test>::insert(SourceChain::Ethereum, 100);

		assert_noop!(
			CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x54),
				20_000,
			),
			Error::<Test>::TooManyPendingTransferOuts,
		);
	});
}

#[test]
fn transfer_out_rejects_stale_verified_execution_anchor() {
	new_test_ext().execute_with(|| {
		let council_account = account(162);
		let council_pair = council_signing_pair(181);
		let user = account(163);

		configure_single_member_ethereum_council(council_account, 141, 20_000, &council_pair);
		assert_ok!(Balances::mint_into(&user, 25_000));
		CurrentTick::set(61);
		LatestExecutionBlockTimestamp::set(Some(0));

		assert_noop!(
			CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x55),
				20_000,
			),
			Error::<Test>::StaleVerifiedExecutionBlock,
		);
	});
}

#[test]
fn collateralize_transfer_marks_ready_tracks_pending_reservations_and_rejects_updates() {
	new_test_ext().execute_with(|| {
		let authority_account = account(62);
		let authority_pair = council_signing_pair(81);
		let signing_key = activate_test_minting_authority(
			authority_account.clone(),
			41,
			20_000,
			&council_signing_pair(82),
			&authority_pair,
			30_000,
			0,
		);
		let user = account(63);

		assert_ok!(Balances::mint_into(&user, 25_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x45),
			20_000,
		));
		let transfer_id = transfer_out_id(&user, 1);
		let signature = transfer_collateral_signature(&authority_pair, transfer_id, 20_000, 0);

		let result = CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(authority_account.clone()),
			transfer_id,
			signature,
			20_000,
			0,
		);
		assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));

		let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
			.expect("authority should stay registered");
		let transfer =
			TransferOutById::<Test>::get(transfer_id).expect("transfer should remain stored");
		assert_eq!(authority.pending_reserved_microgon_collateral, 20_000);
		assert_eq!(authority.active_pending_transfer_ids, vec![transfer_id]);
		assert_eq!(transfer.state, TransferOutState::Ready);
		assert_eq!(transfer.total_attached_collateral, 20_000);
		assert_eq!(
			transfer
				.minting_authority_collateral_by_signer
				.get(&signing_key)
				.expect("authority row should be stored")
				.microgon_collateral,
			20_000,
		);
		assert!(
			PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum).is_empty()
		);
		assert_eq!(
			NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
			1,
		);
		assert_noop!(
			CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account),
				transfer_id,
				transfer_collateral_signature(&authority_pair, transfer_id, 25_000, 0),
				25_000,
				0,
			),
			Error::<Test>::TransferOutAlreadyReady,
		);
	});
}

#[test]
fn collateralize_transfer_rejects_duplicate_signer_and_allows_another_authority_to_complete() {
	new_test_ext().execute_with(|| {
		let first_authority_account = account(64);
		let first_authority_pair = council_signing_pair(83);
		let first_signing_key = activate_test_minting_authority(
			first_authority_account.clone(),
			42,
			20_000,
			&council_signing_pair(84),
			&first_authority_pair,
			20_000,
			0,
		);
		let second_authority_account = account(166);
		let second_authority_pair = council_signing_pair(185);
		let second_signing_key = activate_test_minting_authority(
			second_authority_account.clone(),
			46,
			20_000,
			&council_signing_pair(186),
			&second_authority_pair,
			20_000,
			0,
		);
		let user = account(65);

		assert_ok!(Balances::mint_into(&user, 20_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x46),
			15_000,
		));
		let transfer_id = transfer_out_id(&user, 1);

		assert_ok!(CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(first_authority_account.clone()),
			transfer_id,
			transfer_collateral_signature(&first_authority_pair, transfer_id, 10_000, 0),
			10_000,
			0,
		));
		assert_noop!(
			CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(first_authority_account),
				transfer_id,
				transfer_collateral_signature(&first_authority_pair, transfer_id, 12_000, 0),
				12_000,
				0,
			),
			Error::<Test>::InvalidTransferCollateralUpdate,
		);
		assert_ok!(CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(second_authority_account),
			transfer_id,
			transfer_collateral_signature(&second_authority_pair, transfer_id, 5_000, 0),
			5_000,
			0,
		));

		let first_authority = MintingAuthoritiesBySigner::<Test>::get(first_signing_key)
			.expect("first authority should remain registered");
		let second_authority = MintingAuthoritiesBySigner::<Test>::get(second_signing_key)
			.expect("second authority should remain registered");
		let transfer =
			TransferOutById::<Test>::get(transfer_id).expect("transfer should still exist");
		assert_eq!(first_authority.pending_reserved_microgon_collateral, 10_000);
		assert_eq!(first_authority.active_pending_transfer_ids, vec![transfer_id]);
		assert_eq!(second_authority.pending_reserved_microgon_collateral, 5_000);
		assert_eq!(second_authority.active_pending_transfer_ids, vec![transfer_id]);
		assert_eq!(transfer.state, TransferOutState::Ready);
		assert_eq!(transfer.total_attached_collateral, 15_000);
	});
}

#[test]
fn collateralize_transfer_for_argon_requires_exhausting_microgons_before_micronots() {
	new_test_ext().execute_with(|| {
		let authority_account = account(167);
		let authority_pair = council_signing_pair(187);
		activate_test_minting_authority(
			authority_account.clone(),
			47,
			20_000,
			&council_signing_pair(188),
			&authority_pair,
			10_000,
			10_000,
		);
		let user = account(168);

		assert_ok!(Balances::mint_into(&user, 20_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x49),
			15_000,
		));
		let transfer_id = transfer_out_id(&user, 1);

		assert_noop!(
			CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account.clone()),
				transfer_id,
				transfer_collateral_signature(&authority_pair, transfer_id, 5_000, 5_000),
				5_000,
				5_000,
			),
			Error::<Test>::InvalidTransferCollateral,
		);

		assert_ok!(CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(authority_account),
			transfer_id,
			transfer_collateral_signature(&authority_pair, transfer_id, 10_000, 5_000),
			10_000,
			5_000,
		));

		let transfer =
			TransferOutById::<Test>::get(transfer_id).expect("transfer should stay stored");
		assert_eq!(transfer.state, TransferOutState::Ready);
		assert_eq!(transfer.total_attached_collateral, 15_000);
	});
}

#[test]
fn finalize_transfer_out_reconciles_external_consumption_and_invalidates_newer_rows() {
	new_test_ext().execute_with(|| {
		let authority_account = account(66);
		let authority_pair = council_signing_pair(85);
		let signing_key = activate_test_minting_authority(
			authority_account.clone(),
			43,
			20_000,
			&council_signing_pair(86),
			&authority_pair,
			10_000,
			0,
		);
		let first_user = account(67);
		let second_user = account(68);

		assert_ok!(Balances::mint_into(&first_user, 6_000));
		assert_ok!(Balances::mint_into(&second_user, 11_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(first_user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x47),
			5_000,
		));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(second_user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x48),
			10_000,
		));
		let first_transfer_id = transfer_out_id(&first_user, 1);
		let second_transfer_id = transfer_out_id(&second_user, 1);

		assert_ok!(CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(authority_account),
			second_transfer_id,
			transfer_collateral_signature(&authority_pair, second_transfer_id, 10_000, 0),
			10_000,
			0,
		));
		assert_eq!(
			TransferOutById::<Test>::get(second_transfer_id)
				.expect("second transfer should stay stored")
				.state,
			TransferOutState::Ready,
		);

		assert_ok!(CrosschainTransfer::finalize_transfer_out_from_gateway(
			SourceChain::Ethereum,
			first_transfer_id,
			AssetKind::Argon,
			5_000,
			vec![GatewayMintingAuthorityCollateral::<Test> {
				signing_key,
				microgon_collateral: 5_000,
				micronot_collateral: 0,
			}],
		));

		let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
			.expect("authority should remain registered");
		let second_transfer = TransferOutById::<Test>::get(second_transfer_id)
			.expect("second transfer should remain");
		assert!(TransferOutById::<Test>::get(first_transfer_id).is_none());
		assert_eq!(authority.gateway_remaining_microgon_collateral, 5_000);
		assert_eq!(authority.pending_reserved_microgon_collateral, 0);
		assert_eq!(encumbered_bond_microgons(&account(66)), 5_000);
		assert_eq!(active_bond_microgons(&account(66)), 10_000);
		assert_eq!(second_transfer.state, TransferOutState::Started);
		assert_eq!(second_transfer.total_attached_collateral, 0);
		assert!(second_transfer.minting_authority_collateral_by_signer.is_empty());
		assert!(authority.active_pending_transfer_ids.is_empty());
		assert_eq!(
			PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum),
			vec![PendingCollateralizationRequest::<Test> {
				transfer_id: second_transfer_id,
				remaining_collateral: 10_000,
				remaining_minting_authority_tip: 10,
			}],
		);
		assert_eq!(
			NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
			1,
		);
	});
}

#[test]
fn finalize_transfer_out_unknown_transfer_invalidates_newer_local_reservation() {
	new_test_ext().execute_with(|| {
		let authority_account = account(173);
		let authority_pair = council_signing_pair(90);
		let signing_key = activate_test_minting_authority(
			authority_account.clone(),
			45,
			20_000,
			&council_signing_pair(91),
			&authority_pair,
			10_000,
			0,
		);
		let user = account(174);

		assert_ok!(Balances::mint_into(&user, 11_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x59),
			10_000,
		));
		let local_transfer_id = transfer_out_id(&user, 1);
		let external_transfer_id = H256::repeat_byte(0x92);

		assert_ok!(CrosschainTransfer::collateralize_transfer(
			RuntimeOrigin::signed(authority_account.clone()),
			local_transfer_id,
			transfer_collateral_signature(&authority_pair, local_transfer_id, 10_000, 0),
			10_000,
			0,
		));
		assert_eq!(
			TransferOutById::<Test>::get(local_transfer_id)
				.expect("local transfer should stay stored")
				.state,
			TransferOutState::Ready,
		);

		assert_ok!(CrosschainTransfer::finalize_transfer_out_from_gateway(
			SourceChain::Ethereum,
			external_transfer_id,
			AssetKind::Argon,
			4_000,
			vec![GatewayMintingAuthorityCollateral::<Test> {
				signing_key,
				microgon_collateral: 4_000,
				micronot_collateral: 0,
			}],
		));

		let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
			.expect("authority should remain registered");
		let local_transfer =
			TransferOutById::<Test>::get(local_transfer_id).expect("local transfer should remain");
		assert!(TransferOutById::<Test>::get(external_transfer_id).is_none());
		assert_eq!(authority.gateway_remaining_microgon_collateral, 6_000);
		assert_eq!(authority.pending_reserved_microgon_collateral, 0);
		assert_eq!(encumbered_bond_microgons(&authority_account), 6_000);
		assert_eq!(active_bond_microgons(&authority_account), 6_000);
		assert_eq!(local_transfer.state, TransferOutState::Started);
		assert_eq!(local_transfer.total_attached_collateral, 0);
		assert!(local_transfer.minting_authority_collateral_by_signer.is_empty());
		assert!(authority.active_pending_transfer_ids.is_empty());
		assert_eq!(
			PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum),
			vec![PendingCollateralizationRequest::<Test> {
				transfer_id: local_transfer_id,
				remaining_collateral: 10_000,
				remaining_minting_authority_tip: 10,
			}],
		);
		assert_eq!(
			NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
			1,
		);
	});
}

#[test]
fn finalize_transfer_out_burns_backing_for_unknown_local_transfer() {
	new_test_ext().execute_with(|| {
		let authority_account = account(169);
		let authority_pair = council_signing_pair(88);
		let signing_key = activate_test_minting_authority(
			authority_account.clone(),
			44,
			20_000,
			&council_signing_pair(89),
			&authority_pair,
			10_000,
			250,
		);
		let transfer_id = H256::repeat_byte(0x91);
		assert_ok!(CrosschainTransfer::finalize_transfer_out_from_gateway(
			SourceChain::Ethereum,
			transfer_id,
			AssetKind::Argon,
			4_000,
			vec![GatewayMintingAuthorityCollateral::<Test> {
				signing_key,
				microgon_collateral: 4_000,
				micronot_collateral: 100,
			}],
		));

		let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
			.expect("authority should remain registered");
		assert!(TransferOutById::<Test>::get(transfer_id).is_none());
		assert_eq!(authority.gateway_remaining_microgon_collateral, 6_000);
		assert_eq!(authority.gateway_remaining_micronot_collateral, 150);
		assert_eq!(encumbered_bond_microgons(&authority_account), 6_000);
		assert_eq!(active_bond_microgons(&authority_account), 6_000);
		assert_eq!(encumbered_argonot_micronots(&authority_account), 150);
		assert_eq!(committed_argonot_micronots(&authority_account), 150);
	});
}

#[test]
fn cancel_transfer_out_refunds_principal_and_tip() {
	new_test_ext().execute_with(|| {
		let council_account = account(69);
		let council_pair = council_signing_pair(87);
		let user = account(70);
		let second_user = account(71);

		register_vault_operator(council_account.clone(), 44, 20_000);
		assert_ok!(CrosschainTransfer::set_chain_config(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			chain_config(),
		));
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(council_account.clone()),
			SourceChain::Ethereum,
			council_signer(&council_pair),
			council_signer_registration_signature(&council_pair, &council_account),
		));
		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![council_account]
				.try_into()
				.expect("single council member stays within limit"),
		));
		assert_ok!(Balances::mint_into(&user, 25_000));
		assert_ok!(Balances::mint_into(&second_user, 6_000));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x49),
			20_000,
		));
		assert_ok!(CrosschainTransfer::transfer_out(
			RuntimeOrigin::signed(second_user.clone()),
			SourceChain::Ethereum,
			AssetKind::Argon,
			h160(0x50),
			5_000,
		));
		let transfer_id = transfer_out_id(&user, 1);
		let second_transfer_id = transfer_out_id(&second_user, 1);

		assert_ok!(CrosschainTransfer::cancel_transfer_out_from_gateway(
			SourceChain::Ethereum,
			transfer_id,
		));

		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		assert!(TransferOutById::<Test>::get(transfer_id).is_none());
		assert_eq!(Balances::balance(&burn_account), 5_000);
		assert_eq!(Balances::balance(&user), 25_000);
		assert_eq!(
			TransferOutById::<Test>::get(second_transfer_id).map(|transfer| transfer.amount),
			Some(5_000)
		);
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::CrosschainTransfer(HoldReason::TransferOutMintingAuthorityTip,),
				&user,
			),
			0,
		);
	});
}

#[test]
fn cancel_transfer_out_skips_unknown_transfer_ids() {
	new_test_ext().execute_with(|| {
		let transfer_id = H256::repeat_byte(7);
		assert_ok!(CrosschainTransfer::cancel_transfer_out_from_gateway(
			SourceChain::Ethereum,
			transfer_id,
		));
	});
}
