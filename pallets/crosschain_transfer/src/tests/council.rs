use super::*;

#[test]
fn force_set_global_issuance_council_replaces_active_members() {
	new_test_ext().execute_with(|| {
		let first_account = account(30);
		let second_account = account(31);
		let first_signing_pair = council_signing_pair(1);
		let second_signing_pair = council_signing_pair(2);
		let first_signer = council_signer(&first_signing_pair);
		let second_signer = council_signer(&second_signing_pair);

		register_vault_operator(first_account.clone(), 7, 8_000);
		register_vault_operator(second_account.clone(), 9, 5_000);
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(first_account.clone()),
			SourceChain::Ethereum,
			first_signer,
			council_signer_registration_signature(&first_signing_pair, &first_account),
		));
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(second_account.clone()),
			SourceChain::Ethereum,
			second_signer,
			council_signer_registration_signature(&second_signing_pair, &second_account),
		));

		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![first_account.clone()]
				.try_into()
				.expect("single council member stays within limit"),
		));
		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![second_account.clone()]
				.try_into()
				.expect("single council member stays within limit"),
		));

		let active_council =
			ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
				.expect("council should be stored");
		let council = GlobalIssuanceCouncilByHash::<Test>::get(active_council)
			.expect("council snapshot should be stored");
		assert_eq!(council.total_weight, 5_000);
		assert_eq!(
			CouncilSignerByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				first_account.clone(),
			),
			Some(first_signer),
		);
		assert_eq!(
			CouncilSignerByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				second_account.clone(),
			),
			Some(second_signer),
		);
		assert_eq!(
			CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				first_account,
			),
			None,
		);
		assert_eq!(
			CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				second_account,
			),
			Some(0),
		);
		assert!(!council.members.contains_key(&first_signer));
		assert!(council.members.contains_key(&second_signer));
	});
}

#[test]
fn force_set_global_issuance_council_uses_argonot_price_floor_for_combined_weight() {
	new_test_ext().execute_with(|| {
		let council_account = account(32);
		let council_pair = council_signing_pair(3);
		let council_signer = council_signer(&council_pair);

		register_vault_operator(council_account.clone(), 10, 8_000);
		assert_ok!(set_committed_argonots(council_account.clone(), 500));
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(council_account.clone()),
			SourceChain::Ethereum,
			council_signer,
			council_signer_registration_signature(&council_pair, &council_account),
		));

		ArgonPriceInUsd::set(FixedU128::one());
		ArgonotPriceInUsd::set(FixedU128::from_u32(9));
		LowestMicrogonsPerArgonot::set(Some(2 * argon_primitives::MICROGONS_PER_ARGON));
		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![council_account.clone()]
				.try_into()
				.expect("single council member stays within limit"),
		));

		let first_active_council =
			ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
				.expect("first council should be stored");
		let first_council = GlobalIssuanceCouncilByHash::<Test>::get(first_active_council)
			.expect("first council snapshot should be stored");
		assert_eq!(first_council.total_weight, 9_000);
		assert_eq!(
			first_council
				.members
				.get(&council_signer)
				.expect("council member should be stored")
				.weight,
			9_000,
		);

		ArgonotPriceInUsd::set(FixedU128::from_u32(11));
		LowestMicrogonsPerArgonot::set(Some(3 * argon_primitives::MICROGONS_PER_ARGON));
		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![council_account.clone()]
				.try_into()
				.expect("single council member stays within limit"),
		));

		let second_active_council =
			ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
				.expect("replacement council should be stored");
		let second_council = GlobalIssuanceCouncilByHash::<Test>::get(second_active_council)
			.expect("replacement council snapshot should be stored");
		assert_eq!(second_council.total_weight, 9_500);
		assert_eq!(
			second_council
				.members
				.get(&council_signer)
				.expect("replacement member should be stored")
				.weight,
			9_500,
		);
	});
}

#[test]
fn register_council_signer_validates_proof_and_applies_queued_replacement() {
	new_test_ext().execute_with(|| {
		let council_account = account(37);
		let current_pair = council_signing_pair(51);
		let next_pair = council_signing_pair(52);
		let wrong_pair = council_signing_pair(53);
		let current_signer = council_signer(&current_pair);
		let next_signer = council_signer(&next_pair);

		register_vault_operator(council_account.clone(), 15, 9_000);
		assert_noop!(
			CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(council_account.clone()),
				SourceChain::Ethereum,
				current_signer,
				council_signer_registration_signature(&wrong_pair, &council_account),
			),
			Error::<Test>::InvalidCouncilSignerProof,
		);
		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(council_account.clone()),
			SourceChain::Ethereum,
			current_signer,
			council_signer_registration_signature(&current_pair, &council_account),
		));
		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![council_account.clone()]
				.try_into()
				.expect("single council member stays within limit"),
		));

		assert_ok!(CrosschainTransfer::register_council_signer(
			RuntimeOrigin::signed(council_account.clone()),
			SourceChain::Ethereum,
			next_signer,
			council_signer_registration_signature(&next_pair, &council_account),
		));
		assert_eq!(
			CouncilSignerByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				&council_account,
			),
			Some(current_signer),
		);
		assert_eq!(
			PendingCouncilSignerByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				&council_account,
			),
			Some(next_signer),
		);

		assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
			RuntimeOrigin::root(),
			SourceChain::Ethereum,
			0,
			vec![council_account.clone()]
				.try_into()
				.expect("single council member stays within limit"),
		));

		let active_council =
			ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
				.expect("replacement council should be stored");
		let council = GlobalIssuanceCouncilByHash::<Test>::get(active_council)
			.expect("replacement council snapshot should be stored");
		assert!(council.members.contains_key(&next_signer));
		assert!(!council.members.contains_key(&current_signer));
		assert_eq!(
			CouncilSignerByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				&council_account,
			),
			Some(next_signer),
		);
		assert_eq!(
			PendingCouncilSignerByDestinationChainAndAccountId::<Test>::get(
				SourceChain::Ethereum,
				&council_account,
			),
			None,
		);
	});
}
