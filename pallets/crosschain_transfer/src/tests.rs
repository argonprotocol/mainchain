pub(super) use super::*;
pub(super) use alloy_primitives::Address as AlloyAddress;
pub(super) use alloy_sol_types::SolEvent;
pub(super) use argon_ethereum_contracts::minting_gateway as ethereum_contracts;
pub(super) use argon_primitives::{
	Balance, CallTxPoolKeyProvider, CallTxValidityProvider, CollectBlockerProvider, EthereumLog,
	EthereumReceiptLog,
};
pub(super) use frame_support::{
	assert_noop, assert_ok,
	dispatch::Pays,
	traits::{
		fungible::{Inspect, InspectHold, Mutate, MutateHold},
		tokens::{Fortitude, Precision, Preservation},
	},
};
pub(super) use pallet_prelude::{BoundedVec, H160, H256};
pub(super) use sp_core::{bounded_vec, ecdsa::KeccakPair, Pair};
pub(super) use sp_runtime::transaction_validity::{InvalidTransaction, TransactionValidityError};

pub(super) use super::mock::*;

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

pub(super) fn chain_config() -> ChainConfig {
	ChainConfig::Evm {
		chain_id: 1,
		gateway: h160(0x21),
		argon_token: h160(0x31),
		argonot_token: h160(0x32),
	}
}

pub(super) fn council_signing_pair(seed_byte: u8) -> KeccakPair {
	KeccakPair::from_seed(&[seed_byte; 32])
}

pub(super) fn council_signer(pair: &KeccakPair) -> H160 {
	CrosschainTransfer::evm_address_from_public_key(&pair.public())
		.expect("test council public key should map to an ethereum address")
}

pub(super) fn council_signer_registration_signature(
	council_signing_pair: &KeccakPair,
	account_id: &TestAccountId,
) -> sp_core::ecdsa::KeccakSignature {
	let signable_message = CrosschainTransfer::evm_personal_message(
		CrosschainTransfer::council_signer_registration_message(SourceChain::Ethereum, account_id)
			.as_slice(),
	);

	council_signing_pair.sign(signable_message.as_slice())
}

pub(super) fn minting_authority_registration_signature(
	signing_pair: &KeccakPair,
	account_id: &TestAccountId,
) -> sp_core::ecdsa::KeccakSignature {
	let signable_message = CrosschainTransfer::evm_personal_message(
		CrosschainTransfer::minting_authority_signer_registration_message(
			SourceChain::Ethereum,
			account_id,
		)
		.as_slice(),
	);

	signing_pair.sign(signable_message.as_slice())
}

pub(super) fn minting_authority_approval_signature(
	council_signing_pair: &KeccakPair,
	queue_nonce: CouncilApprovalQueueNonce,
) -> sp_core::ecdsa::KeccakSignature {
	let queue_entry = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
		SourceChain::Ethereum,
		queue_nonce,
	)
	.expect("queue entry should exist for approval signature test helper");
	let signable_message = CrosschainTransfer::evm_signed_message(queue_entry.approval_hash);

	council_signing_pair.sign(signable_message.as_slice())
}

pub(super) fn configure_single_member_ethereum_council(
	council_account: TestAccountId,
	vault_id: u32,
	securitization: Balance,
	council_pair: &KeccakPair,
) {
	register_vault_operator(council_account.clone(), vault_id, securitization);
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
		RuntimeOrigin::signed(council_account.clone()),
		SourceChain::Ethereum,
		council_signer(council_pair),
		council_signer_registration_signature(council_pair, &council_account),
	));
	assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
		RuntimeOrigin::root(),
		SourceChain::Ethereum,
		GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum)
			.map_or(0, |state| state.argon_approvals_nonce),
		vec![council_account]
			.try_into()
			.expect("single council member stays within limit"),
	));
}

pub(super) fn transfer_collateral_signature(
	signing_pair: &KeccakPair,
	transfer_id: H256,
	microgon_collateral: Balance,
	micronot_collateral: Balance,
) -> sp_core::ecdsa::KeccakSignature {
	let transfer = TransferOutById::<Test>::get(transfer_id)
		.expect("transfer should exist for signing helper");
	let approval_hash = H256::from_slice(
		ethereum_contracts::hash_minting_authorization(
			1,
			AlloyAddress::from_slice(h160(0x21).as_bytes()),
			alloy_primitives::B256::from(
				CrosschainTransfer::transfer_out_request_id(&transfer)
					.expect("transfer request id should hash")
					.0,
			),
			microgon_collateral,
			micronot_collateral,
		)
		.as_slice(),
	);

	signing_pair.sign(CrosschainTransfer::evm_signed_message(approval_hash).as_slice())
}

pub(super) fn destination_bytes(recipient: &TestAccountId) -> [u8; 32] {
	let bytes: &[u8] = recipient.as_ref();
	bytes.try_into().expect("account id is 32 bytes")
}

pub(super) fn activate_test_minting_authority(
	operator_account: TestAccountId,
	vault_id: u32,
	activated_securitization: Balance,
	council_pair: &KeccakPair,
	authority_pair: &KeccakPair,
	microgon_collateral: Balance,
	micronot_collateral: Balance,
) -> H160 {
	configure_single_member_ethereum_council(
		operator_account.clone(),
		vault_id,
		activated_securitization,
		council_pair,
	);
	assert_ok!(Balances::mint_into(&operator_account, 1_000));
	set_active_vault_bond_amount(vault_id, operator_account.clone(), microgon_collateral);
	if micronot_collateral != 0 {
		assert_ok!(Ownership::mint_into(&operator_account, micronot_collateral));
		assert_ok!(set_committed_argonots(operator_account.clone(), micronot_collateral,));
	}

	let signing_key = council_signer(authority_pair);
	assert_ok!(CrosschainTransfer::register_minting_authority(
		RuntimeOrigin::signed(operator_account.clone()),
		SourceChain::Ethereum,
		signing_key,
		minting_authority_registration_signature(authority_pair, &operator_account),
		microgon_collateral,
		micronot_collateral,
	));
	let approval_queue_nonce = MintingAuthoritiesBySigner::<Test>::get(signing_key)
		.expect("authority should be inserted")
		.activation_approval_queue_nonce;
	assert_ok!(CrosschainTransfer::approve_queue_entries(
		RuntimeOrigin::signed(operator_account.clone()),
		SourceChain::Ethereum,
		bounded_vec![minting_authority_approval_signature(council_pair, approval_queue_nonce,)],
	));
	let approval_hash = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
		SourceChain::Ethereum,
		approval_queue_nonce,
	)
	.expect("activation queue entry should exist before proving the activation")
	.approval_hash;

	let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
	let previous_gateway_activity_nonce =
		GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum)
			.map_or(0, |state| state.gateway_activity_nonce);
	let gateway_activity_nonce = previous_gateway_activity_nonce.saturating_add(1);
	let event_log = EthereumLog {
		address: h160(0x21),
		topics: vec![
			H256::from_slice(
				ethereum_contracts::MintingAuthorityActivated::SIGNATURE_HASH.as_slice(),
			),
			indexed_address_word(signing_key),
		]
		.try_into()
		.expect("topics stay within Ethereum log topic bounds"),
		data: {
			let mut data = Vec::with_capacity(288);
			data.extend_from_slice(&u128_word(microgon_collateral));
			data.extend_from_slice(&u128_word(micronot_collateral));
			data.extend_from_slice(&u64_word(1));
			data.extend_from_slice(&u64_word(1));
			data.extend_from_slice(approval_hash.as_bytes());
			data.extend_from_slice(&destination_bytes(&operator_account));
			data.extend_from_slice(&u64_word(gateway_activity_nonce));
			data.extend_from_slice(&u64_word(approval_queue_nonce));
			data.extend_from_slice(&u128_word(Balances::balance(&burn_account)));
			data.extend_from_slice(&u128_word(Ownership::balance(&burn_account)));
			data.try_into()
				.expect("minting-authority activation data stays within bounded log payload")
		},
	};
	let receipt_log = EthereumReceiptLog { transaction_index: 0, event_log };
	let decoded_activity = match CrosschainTransfer::decode_evm_gateway_activity(
		SourceChain::Ethereum,
		&receipt_log.event_log,
	) {
		Ok(decoded_activity) => decoded_activity,
		Err(_) => panic!("activation proof should succeed"),
	};
	let gateway_state = match CrosschainTransfer::apply_decoded_gateway_activity(
		SourceChain::Ethereum,
		previous_gateway_activity_nonce,
		decoded_activity,
	) {
		Ok(gateway_state) => gateway_state,
		Err(_) => panic!("activation proof should succeed"),
	};
	GatewayStateBySourceChain::<Test>::insert(SourceChain::Ethereum, gateway_state);
	signing_key
}

pub(super) fn transfer_out_id(
	account_id: &TestAccountId,
	argon_transfer_nonce: TransferOutRequestNonce,
) -> H256 {
	TransferOutById::<Test>::iter()
		.find_map(|(transfer_id, transfer)| {
			(transfer.argon_account_id == *account_id &&
				transfer.argon_transfer_nonce == argon_transfer_nonce)
				.then_some(transfer_id)
		})
		.expect("transfer should be stored for account nonce")
}

pub(super) fn indexed_address_word(address: H160) -> H256 {
	let mut bytes = [0u8; 32];
	bytes[12..].copy_from_slice(address.as_bytes());
	H256::from(bytes)
}

pub(super) fn u64_word(value: u64) -> [u8; 32] {
	let mut bytes = [0u8; 32];
	bytes[24..].copy_from_slice(&value.to_be_bytes());
	bytes
}

pub(super) fn u128_word(value: u128) -> [u8; 32] {
	let mut bytes = [0u8; 32];
	bytes[16..].copy_from_slice(&value.to_be_bytes());
	bytes
}
