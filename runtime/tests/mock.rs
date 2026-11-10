//! Fixtures use the real runtime that includes this shared test suite.

use crate::{
	AccountId, Balances, CallAuthorizationPolicy, MinimumUniswapTransfer,
	OperationalMinimumBitcoinLiquidity, OperationalMinimumVaultBondPrincipal, Proxy, ProxyType,
	Runtime, RuntimeCall, RuntimeOrigin, SignedPayload, System, UncheckedExtrinsic, ARGON,
};
use argon_primitives::{
	crypto::{P256Public, P256Signature},
	vault::{Vault, VaultTerms},
	BlockSealerInfo, MultiSigner as NativeSigner, Signature,
};
use codec::Encode;
use frame_support::assert_ok;
use p256::ecdsa::Signature as DeviceSignature;
use pallet_account_security::{
	AccountSecurityPolicy, Approval, Authorization, NextNonce, Policy, Registrations,
};
use pallet_operational_accounts::{
	AccountOwnershipProof, OperationalAccount, OperationalAccountBySubAccount, OperationalAccounts,
	Registration, RegistrationV1, MINING_ACCOUNT_PROOF_MESSAGE_KEY,
	OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY, VAULT_ACCOUNT_PROOF_MESSAGE_KEY,
};
use pallet_vaults::{VaultIdByOperator, VaultsById};
use polkadot_sdk::*;
use sp_arithmetic::FixedU128;
use sp_core::{hashing::blake2_256, sr25519, Pair};
use sp_runtime::{
	generic::Era,
	traits::{IdentifyAccount, Zero},
	BuildStorage, MultiSigner,
};
use std::{fs, path::Path, process::Command};

pub(super) fn signed(seed: u8, call: RuntimeCall) -> UncheckedExtrinsic {
	let pair = sr25519::Pair::from_seed(&[seed; 32]);
	let signer = MultiSigner::from(pair.public()).into_account();
	let extra = (
		frame_system::AuthorizeCall::<Runtime>::new(),
		frame_system::CheckNonZeroSender::<Runtime>::new(),
		frame_system::CheckSpecVersion::<Runtime>::new(),
		frame_system::CheckTxVersion::<Runtime>::new(),
		frame_system::CheckGenesis::<Runtime>::new(),
		frame_system::CheckMortality::<Runtime>::from(Era::Immortal),
		frame_system::CheckNonce::<Runtime>::from(System::account_nonce(&signer)),
		frame_system::CheckWeight::<Runtime>::new(),
		pallet_account_security::CheckAccountSecurity::<Runtime>::new(),
		pallet_fee_control::CheckFeeWrapper::from(
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
		),
		frame_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
		frame_system::WeightReclaim::<Runtime>::new(),
	);
	let payload = SignedPayload::new(call.clone(), extra.clone()).unwrap();
	let signature = payload.using_encoded(|bytes| pair.sign(bytes));
	UncheckedExtrinsic::new_signed(call, signer.into(), signature.into(), extra)
}

pub(super) fn account(seed: u8) -> AccountId {
	sr25519::Pair::from_seed(&[seed; 32]).public().0.into()
}

pub(super) fn authorization(account: &AccountId) -> Authorization<Runtime> {
	let group = CallAuthorizationPolicy::default_account(account);
	let policy = Registrations::<Runtime>::get(&group).map(|registration| registration.policy);
	Authorization {
		nonce: NextNonce::<Runtime>::get(&group),
		group,
		expires_at: 100,
		policy_hash: policy.using_encoded(blake2_256),
	}
}

pub(super) fn enrollment(exit: u8, recovery: u8) -> Policy<Runtime> {
	Policy {
		signers: vec![account(exit)].try_into().unwrap(),
		quorum: 1,
		recovery_account: account(recovery),
		recovery_delay: 0,
	}
}

pub(super) fn registration_call(main: u8, policy: Policy<Runtime>) -> RuntimeCall {
	RuntimeCall::AccountSecurity(pallet_account_security::Call::register {
		account: account(main),
		signers: policy.signers,
		quorum: policy.quorum,
		recovery_account: policy.recovery_account,
		recovery_delay: policy.recovery_delay,
		authorization: authorization(&account(main)),
		approvals: Default::default(),
	})
}

pub(super) fn execution_call(main: u8, call: RuntimeCall) -> RuntimeCall {
	RuntimeCall::AccountSecurity(pallet_account_security::Call::execute {
		account: account(main),
		call: Box::new(call),
		authorization: authorization(&account(main)),
		approvals: Default::default(),
	})
}

pub(super) fn sign_call(mut call: RuntimeCall, seeds: &[u8]) -> RuntimeCall {
	let payload = crate::AccountSecurity::approval_payload(&call, false).unwrap();
	let signatures = seeds
		.iter()
		.map(|seed| {
			let pair = sr25519::Pair::from_seed(&[*seed; 32]);
			Approval {
				signer: account(*seed),
				signature: pair.sign(&payload).into(),
				with_metadata_hash: false,
			}
		})
		.collect::<Vec<_>>()
		.try_into()
		.unwrap();
	let RuntimeCall::AccountSecurity(security) = &mut call else {
		panic!("expected security call")
	};
	match security {
		pallet_account_security::Call::register { approvals, .. } |
		pallet_account_security::Call::update_policy { approvals, .. } |
		pallet_account_security::Call::execute { approvals, .. } => *approvals = signatures,
		_ => panic!("expected security entrypoint"),
	}
	call
}

/// Signs with an existing PIV key. PIN entry stays in the tool's terminal prompt, not argv/env.
pub(super) fn yubikey_approval(
	mut call: RuntimeCall,
	public: &P256Public,
	slot: &str,
	directory: &Path,
) -> RuntimeCall {
	let payload = crate::AccountSecurity::approval_payload(&call, false).unwrap();
	let input = directory.join("payload.bin");
	let output = directory.join("signature.der");
	fs::write(&input, &payload).unwrap();

	assert!(Command::new("yubico-piv-tool")
		.args(["-a", "verify-pin", "--sign", "-s", slot, "-A", "ECCP256", "-H", "SHA256"])
		.arg("-i")
		.arg(&input)
		.arg("-o")
		.arg(&output)
		.status()
		.expect("yubico-piv-tool must be installed")
		.success());

	let der = fs::read(output).unwrap();
	let signature = DeviceSignature::from_der(&der).expect("PIV returns DER ECDSA");
	let signature =
		P256Signature::from_device_signature(&payload, &signature.to_bytes().into(), public)
			.expect("PIV key must match the slot certificate and exact payload");
	let approval = Approval {
		signer: NativeSigner::P256(*public).into_account(),
		signature: Signature::P256(signature),
		with_metadata_hash: false,
	};

	let RuntimeCall::AccountSecurity(security) = &mut call else {
		panic!("expected security call")
	};
	match security {
		pallet_account_security::Call::register { approvals, .. } |
		pallet_account_security::Call::execute { approvals, .. } => approvals.try_push(approval).unwrap(),
		_ => panic!("expected enrollment or execution"),
	}
	call
}

pub(super) fn enroll(main: u8, exit: u8, recovery: u8) {
	let call = sign_call(registration_call(main, enrollment(exit, recovery)), &[exit, recovery]);
	assert_ok!(crate::Executive::apply_extrinsic(signed(main, call)).unwrap());
}

pub(super) fn authorized_call(main: u8, call: RuntimeCall) -> RuntimeCall {
	sign_call(execution_call(main, call), &[main, 2])
}

pub(super) fn register_proxy(
	real: AccountId,
	delegate: AccountId,
	proxy_type: ProxyType,
	delay: u32,
) {
	assert_ok!(Proxy::add_proxy(RuntimeOrigin::signed(real), delegate.into(), proxy_type, delay));
}

pub(super) fn externalities() -> sp_io::TestExternalities {
	let storage = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	let mut externalities = sp_io::TestExternalities::new(storage);
	externalities.execute_with(|| {
		System::set_block_number(1);
		pallet_block_seal::LastBlockSealerInfo::<Runtime>::put(BlockSealerInfo {
			block_author_account_id: account(9),
			block_vote_rewards_account: None,
			block_seal_authority: None,
		});
		for seed in 1..=8 {
			assert_ok!(Balances::force_set_balance(
				RuntimeOrigin::root(),
				account(seed).into(),
				100 * ARGON
			));
		}
	});
	externalities
}

pub(super) fn link_accounts(operational: AccountId, vault: AccountId, mining: AccountId) {
	let record = OperationalAccount::<Runtime> {
		vault_account: vault.clone(),
		mining_account: mining.clone(),
		encryption_pubkey: Default::default(),
		upstream_account: None,
		name: None,
		last_name_change_tick: None,
		uniswap_argon_transfers_in_amount: MinimumUniswapTransfer::get(),
		account_bitcoin_amount: OperationalMinimumBitcoinLiquidity::get(),
		account_vault_bond_amount: OperationalMinimumVaultBondPrincipal::get(),
		vault_created: false,
		vault_bitcoin_accrual: 0,
		vault_bitcoin_applied_total: 0,
		mining_seat_accrual: 0,
		mining_seat_applied_total: 0,
		operational_certifications_count: 0,
		available_access_codes: 0,
		rewards_earned_count: 0,
		rewards_earned_amount: 0,
		rewards_collected_amount: 0,
		is_operationally_certified: false,
	};
	OperationalAccounts::<Runtime>::insert(&operational, record);
	OperationalAccountBySubAccount::<Runtime>::insert(vault, &operational);
	OperationalAccountBySubAccount::<Runtime>::insert(mining, operational);
}

pub(super) fn set_vault_delegate(vault_id: u32, operator: AccountId, delegate: AccountId) {
	VaultIdByOperator::<Runtime>::insert(&operator, vault_id);
	VaultsById::<Runtime>::insert(
		vault_id,
		Vault {
			operator_account_id: operator,
			delegate_account_id: Some(delegate),
			securitization: 0,
			securitization_target: 0,
			securitization_locked: 0,
			flexible_securitization_locked: 0,
			reserved_securitization_space: 0,
			securitization_pending_activation: 0,
			total_satoshis: 0,
			securitized_satoshis: 0,
			securitization_release_schedule: Default::default(),
			committed_microgons: 0,
			is_closed: false,
			terms: VaultTerms {
				bitcoin_annual_percent_rate: FixedU128::zero(),
				bitcoin_base_fee: 0,
			},
			pending_terms: None,
			opened_tick: 0,
		},
	);
}

pub(super) fn registration(
	operational_seed: u8,
	vault_seed: u8,
	mining_seed: u8,
) -> Registration<Runtime> {
	let operational = account(operational_seed);
	let proof = |seed: u8, domain: &[u8]| {
		let message = (domain, &operational, &account(seed)).using_encoded(blake2_256);
		AccountOwnershipProof {
			signature: sr25519::Pair::from_seed(&[seed; 32]).sign(&message).into(),
		}
	};
	Registration::V1(RegistrationV1 {
		operational_account: operational.clone(),
		encryption_pubkey: Default::default(),
		operational_account_proof: proof(operational_seed, OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY),
		vault_account: account(vault_seed),
		mining_account: account(mining_seed),
		vault_account_proof: proof(vault_seed, VAULT_ACCOUNT_PROOF_MESSAGE_KEY),
		mining_account_proof: proof(mining_seed, MINING_ACCOUNT_PROOF_MESSAGE_KEY),
		access_proof: None,
	})
}

pub(super) fn registration_minimums(vault: AccountId) {
	pallet_operational_accounts::IsOperationalAccountInviteOnly::<Runtime>::put(false);
	pallet_treasury_positions::PositionsByAccount::<Runtime>::insert(
		vault,
		pallet_treasury_positions::Position {
			bond_principal: OperationalMinimumVaultBondPrincipal::get(),
			quantities: argon_primitives::treasury::PositionQuantities {
				fission_liquidity: OperationalMinimumBitcoinLiquidity::get(),
				..Default::default()
			},
			upstream: None,
		},
	);
}

pub(super) fn council_approval(owner: AccountId) -> RuntimeCall {
	use pallet_crosschain_transfer::{
		CouncilApprovalCursorByDestinationChainAndAccountId,
		CouncilApprovalQueueByDestinationChainAndNonce, CouncilApprovalQueueEntry,
		CouncilApprovalTargetId, GlobalIssuanceCouncil, GlobalIssuanceCouncilByHash,
		GlobalIssuanceCouncilMember, SourceChain,
	};
	use sp_core::{ecdsa::KeccakPair, hashing::keccak_256, H160, H256};

	let hash = H256::repeat_byte(11);
	let mut message = b"\x19Ethereum Signed Message:\n32".to_vec();
	message.extend_from_slice(hash.as_bytes());
	let signature = KeccakPair::from_seed(&[11; 32]).sign(&message);
	let public = sp_io::crypto::secp256k1_ecdsa_recover(&signature.0, &keccak_256(&message))
		.ok()
		.expect("fixture signature must recover");
	let signer = H160::from_slice(&keccak_256(&public)[12..]);
	let council_hash = H256::repeat_byte(12);
	let member =
		GlobalIssuanceCouncilMember::<Runtime> { account_id: owner.clone(), signer, weight: ARGON };
	// The native snapshot and queue are input; approval execution is not mocked.
	let mut council = GlobalIssuanceCouncil::<Runtime> {
		epoch_microgons_per_argonot: ARGON,
		members: Default::default(),
		total_weight: ARGON,
	};
	council.members.try_insert(signer, member).unwrap();
	GlobalIssuanceCouncilByHash::<Runtime>::insert(council_hash, council);
	CouncilApprovalCursorByDestinationChainAndAccountId::<Runtime>::insert(
		SourceChain::Ethereum,
		owner,
		0u64,
	);
	CouncilApprovalQueueByDestinationChainAndNonce::<Runtime>::insert(
		SourceChain::Ethereum,
		1u64,
		CouncilApprovalQueueEntry::<Runtime> {
			approving_council_hash: council_hash,
			target: CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(H256::repeat_byte(13)),
			target_payload_hash: H256::repeat_byte(14),
			due_frame_id: 100,
			previous_approval_hash: H256::zero(),
			approval_hash: hash,
			approved_total_weight: 0,
			signatures: Default::default(),
		},
	);
	RuntimeCall::CrosschainTransfer(pallet_crosschain_transfer::Call::approve_queue_entries {
		destination_chain: SourceChain::Ethereum,
		signatures: vec![signature].try_into().unwrap(),
	})
}
