pub(super) use super::{
	migrations::InitializeCrosschainTransferMigration,
	ActiveGlobalIssuanceCouncilByDestinationChain, ArgonApprovalsNonce, AssetKind, Call,
	ChainConfig, ChainConfigBySourceChain, CouncilApprovalCursorByDestinationChainAndAccountId,
	CouncilApprovalQueueByDestinationChainAndNonce, CouncilApprovalQueueEntry,
	CouncilApprovalQueueNonce, CouncilApprovalTargetId,
	CouncilSignerByDestinationChainAndAccountId, Error, Event, GatewayActivityNonce,
	GatewayActivityProofBatch, GatewayActivityProofBlock, GatewayState, GatewayStateBySourceChain,
	GatewaySyncPause, GatewaySyncPauseBySourceChain, GatewaySyncPauseReason,
	GlobalIssuanceCouncilByHash, HoldReason, InboundTransfersExpiringAt,
	MinimumMintingAuthorityValueByDestinationChain, MintingAuthoritiesBySigner, MintingAuthority,
	MintingAuthorityActivationRepaymentPricing, MintingAuthorityState,
	NonTerminalTransferOutCountByDestinationChain, PendingCollateralizationRequest,
	PendingCollateralizationRequestsByChain, PendingCouncilSignerByDestinationChainAndAccountId,
	PendingTransferOutCirculationByDestinationChain, RecentArgonTransfersByAccount, SourceChain,
	TransferOutById, TransferOutRequestNonce, TransferOutState, TransferToArgonActivity,
};
pub(super) use alloy_primitives::Address as AlloyAddress;
pub(super) use alloy_sol_types::SolEvent;
pub(super) use argon_ethereum_contracts::minting_gateway::{self as ethereum_contracts};
pub(super) use argon_primitives::{
	CallTxPoolKeyProvider, CallTxValidityProvider, CollectBlockerProvider, EthereumLog,
	EthereumReceiptLog,
};
pub(super) use frame_support::{
	assert_noop, assert_ok,
	dispatch::Pays,
	traits::{
		fungible::{InspectHold, Mutate},
		OnRuntimeUpgrade,
	},
};
pub(super) use pallet_prelude::*;
pub(super) use sp_core::{bounded_vec, crypto::Ss58Codec, ecdsa::KeccakPair, Pair};
pub(super) use sp_runtime::{transaction_validity::InvalidTransaction, AccountId32};

pub(super) use super::mock::{
	account, active_bond_microgons, committed_argonot_micronots, encumbered_argonot_micronots,
	encumbered_bond_microgons, h160, legacy_token_gateway_account, new_test_ext,
	register_vault_operator, set_active_bond_amount, set_committed_argonots, ArgonPriceInUsd,
	ArgonotPriceInUsd, Balances, ConfirmedTransfers, CrosschainTransfer, CurrentFrameId,
	CurrentTick, LatestExecutionBlockTimestamp, LowestMicrogonsPerArgonot,
	MaxActivitiesPerReceiptProof, Ownership, ProofVerificationAllowed,
	ProofVerificationRejectedTransactionIndexes, RecentTransferRetentionTicks, RuntimeCall,
	RuntimeEvent, RuntimeHoldReason, RuntimeOrigin, System, Test, TestAccountId,
};

#[path = "tests/council.rs"]
mod council_tests;
#[path = "tests/gateway_activity.rs"]
mod gateway_activity_tests;
#[path = "tests/lifecycle.rs"]
mod lifecycle_tests;
#[path = "tests/minting_authority.rs"]
mod minting_authority_tests;
#[path = "tests/transfer_out.rs"]
mod transfer_out_tests;

fn chain_config() -> ChainConfig {
	ChainConfig::Evm {
		chain_id: 1,
		gateway: h160(0x21),
		argon_token: h160(0x31),
		argonot_token: h160(0x32),
	}
}

fn council_signing_pair(seed_byte: u8) -> KeccakPair {
	KeccakPair::from_seed(&[seed_byte; 32])
}

fn council_signer(pair: &KeccakPair) -> H160 {
	CrosschainTransfer::evm_address_from_public_key(&pair.public())
		.expect("test council public key should map to an ethereum address")
}

fn council_signer_registration_signature(
	council_signing_pair: &KeccakPair,
	account_id: &TestAccountId,
) -> sp_core::ecdsa::KeccakSignature {
	let signable_message = CrosschainTransfer::evm_personal_message(
		CrosschainTransfer::council_signer_registration_message(SourceChain::Ethereum, account_id)
			.as_slice(),
	);

	council_signing_pair.sign(signable_message.as_slice())
}

fn minting_authority_registration_signature(
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

fn minting_authority_approval_signature(
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

fn minting_authority_deactivation_signature(
	signing_pair: &KeccakPair,
	queue_nonce: CouncilApprovalQueueNonce,
	signing_key: H160,
	previous_approval_hash: H256,
) -> sp_core::ecdsa::KeccakSignature {
	let approval_hash = CrosschainTransfer::hash_council_approval_queue_entry(
		SourceChain::Ethereum,
		queue_nonce,
		&CouncilApprovalQueueEntry::<Test> {
			approving_council_hash: H256::zero(),
			target: CouncilApprovalTargetId::MintingAuthorityDeactivation(signing_key),
			target_payload_hash: CrosschainTransfer::hash_deactivate_minting_authority_target(
				signing_key,
			),
			due_frame_id: 0,
			previous_approval_hash,
			approval_hash: H256::zero(),
			approved_total_weight: 0,
			signatures: BoundedBTreeMap::new(),
		},
	)
	.expect("deactivation queue hash should be computable in tests");

	signing_pair.sign(CrosschainTransfer::evm_signed_message(approval_hash).as_slice())
}

fn configure_single_member_ethereum_council(
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

fn transfer_collateral_signature(
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

fn destination_bytes(recipient: &TestAccountId) -> [u8; 32] {
	let bytes: &[u8] = recipient.as_ref();
	bytes.try_into().expect("account id is 32 bytes")
}

fn activate_test_minting_authority(
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
	set_active_bond_amount(vault_id, operator_account.clone(), microgon_collateral);
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
			data.extend_from_slice(&destination_bytes(&operator_account));
			data.extend_from_slice(&u64_word(gateway_activity_nonce));
			data.extend_from_slice(&u64_word(approval_queue_nonce));
			data.extend_from_slice(&u128_word(Balances::balance(&burn_account)));
			data.extend_from_slice(&u128_word(Ownership::balance(&burn_account)));
			data.try_into()
				.expect("minting-authority activation data stays within bounded log payload")
		},
	};
	let gateway_state = match CrosschainTransfer::apply_proved_gateway_activity_log(
		SourceChain::Ethereum,
		previous_gateway_activity_nonce,
		EthereumReceiptLog { transaction_index: 0, event_log },
	) {
		Ok(gateway_state) => gateway_state,
		Err(_) => panic!("activation proof should succeed"),
	};
	GatewayStateBySourceChain::<Test>::insert(SourceChain::Ethereum, gateway_state);
	signing_key
}

fn transfer_out_id(
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
