use super::mock::{
	account, authorized_call, council_approval, enroll, enrollment, execution_call, externalities,
	link_accounts, register_proxy, registration, registration_call, registration_minimums,
	set_vault_delegate, sign_call, signed, yubikey_approval,
};
use crate::*;
use codec::Encode;
use frame_support::{assert_noop, assert_ok, traits::fungible::MutateHold};
use pallet_account_security::{
	Authorization, Error as SecurityError, Event as SecurityEvent, NextNonce, Registrations,
};
use pallet_multisig::{Call as MultisigCall, Multisigs};
use pallet_operational_accounts::{
	Call as OperationalCall, OperationalAccountBySubAccount, OperationalAccounts as LinkedAccounts,
};
use sp_core::hashing::blake2_256;
use sp_runtime::traits::{Dispatchable, Verify};

type Error = SecurityError<Runtime>;

#[test]
fn linked_accounts_share_one_registration_and_internal_transfers_remain_unattended() {
	externalities().execute_with(|| {
		link_accounts(account(5), account(1), account(6));
		enroll(1, 2, 3);
		assert_noop!(
			registration_call(6, enrollment(2, 3)).dispatch(RuntimeOrigin::signed(account(6))),
			Error::NotDefaultAccount
		);
		let internal = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: account(6).into(),
			value: ARGON,
		});
		assert_ok!(Executive::apply_extrinsic(signed(1, internal)).unwrap());
		let exit = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: account(4).into(),
			value: ARGON,
		});
		assert_eq!(
			Executive::apply_extrinsic(signed(6, exit.clone())),
			Err(Error::ApprovalRequired.into())
		);
		let call = authorized_call(6, exit);
		assert_ok!(Executive::apply_extrinsic(signed(2, call)).unwrap());
		assert!(!Registrations::<Runtime>::contains_key(account(6)));
	});
}

#[test]
fn operational_linking_preserves_policies_and_invalidates_precollected_authorizations() {
	externalities().execute_with(|| {
		registration_minimums(account(1));
		enroll(5, 2, 3);
		enroll(1, 2, 3);
		let exit = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: account(4).into(),
			value: ARGON,
		});
		let stale = authorized_call(5, exit);
		let register = RuntimeCall::OperationalAccounts(OperationalCall::register {
			registration: registration(5, 1, 6),
		});
		assert_eq!(
			Executive::apply_extrinsic(signed(5, register.clone())),
			Err(Error::ApprovalRequired.into())
		);
		let call = authorized_call(5, register);
		assert_ok!(Executive::apply_extrinsic(signed(2, call)).unwrap());
		assert!(!Registrations::<Runtime>::contains_key(account(5)));
		assert!(Registrations::<Runtime>::contains_key(account(1)));
		assert_eq!(System::account(account(2)).sufficients, 1);
		assert_eq!(
			Executive::apply_extrinsic(signed(2, stale)).unwrap(),
			Err(Error::StalePolicy.into())
		);
	});
}

#[test]
fn failed_group_hook_rolls_back_relationships_and_retains_protection() {
	externalities().execute_with(|| {
		registration_minimums(account(1));
		enroll(5, 1, 3);

		let register = RuntimeCall::OperationalAccounts(OperationalCall::register {
			registration: registration(5, 1, 6),
		});
		let call = sign_call(execution_call(5, register), &[5, 1]);
		assert_ok!(Executive::apply_extrinsic(signed(1, call)).unwrap());

		assert!(!LinkedAccounts::<Runtime>::contains_key(account(5)));
		assert!(!OperationalAccountBySubAccount::<Runtime>::contains_key(account(1)));
		assert!(Registrations::<Runtime>::contains_key(account(5)));
		assert!(!Registrations::<Runtime>::contains_key(account(1)));
		assert!(System::events().iter().any(|record| matches!(
			&record.event,
			RuntimeEvent::AccountSecurity(SecurityEvent::Executed {
				result: Err(error),
				..
			}) if *error == Error::KeysNotIndependent.into()
		)));
	});
}

#[test]
fn group_members_cannot_be_registered_as_their_own_security_factors() {
	externalities().execute_with(|| {
		link_accounts(account(5), account(1), account(6));
		for policy in [enrollment(6, 3), enrollment(2, 5)] {
			assert_noop!(
				registration_call(1, policy).dispatch(RuntimeOrigin::signed(account(1))),
				Error::KeysNotIndependent
			);
		}
	});
}

#[test]
fn vault_delegate_remains_external_without_an_inherited_funding_exception() {
	externalities().execute_with(|| {
		set_vault_delegate(1, account(1), account(4));
		enroll(1, 2, 3);
		let transfer = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: account(4).into(),
			value: ARGON,
		});
		assert_eq!(
			Executive::apply_extrinsic(signed(1, transfer)),
			Err(Error::ApprovalRequired.into())
		);
	});
}

#[test]
fn linked_members_share_one_nonce_even_when_proposals_are_collected_off_chain() {
	externalities().execute_with(|| {
		link_accounts(account(5), account(1), account(6));
		enroll(1, 2, 3);
		let transfer = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: account(4).into(),
			value: ARGON,
		});
		let member = authorized_call(6, transfer.clone());
		let owner = authorized_call(1, transfer);
		let nonce = NextNonce::<Runtime>::get(account(1));
		assert_ok!(Executive::apply_extrinsic(signed(2, member)).unwrap());
		assert_eq!(NextNonce::<Runtime>::get(account(1)), nonce + 1);
		assert_eq!(
			Executive::apply_extrinsic(signed(2, owner)).unwrap(),
			Err(Error::InvalidNonce.into())
		);
	});
}

#[test]
fn bitcoin_funding_trusts_only_own_and_registered_sponsor_vaults() {
	externalities().execute_with(|| {
		link_accounts(account(5), account(1), account(6));
		link_accounts(account(7), account(3), account(8));
		pallet_operational_accounts::OperationalAccounts::<Runtime>::mutate(account(5), |record| {
			record.as_mut().unwrap().upstream_account = Some(account(7));
		});
		set_vault_delegate(1, account(1), account(4));
		set_vault_delegate(2, account(3), account(4));
		set_vault_delegate(3, account(4), account(2));
		enroll(1, 2, 3);

		for vault_id in [1, 2, 3] {
			let call =
				RuntimeCall::BitcoinLocks(pallet_bitcoin_locks::Call::create_receive_address {
					vault_id,
					satoshis: 100_000,
					bitcoin_pubkey: [2u8; 33].into(),
					options: None,
				});
			if vault_id == 3 {
				assert_eq!(
					AccountSecurity::ensure_authorized(&account(1), &call),
					Err(Error::ApprovalRequired)
				);
			} else {
				assert_ok!(AccountSecurity::ensure_authorized(&account(1), &call));
				assert_ok!(AccountSecurity::ensure_authorized(&account(6), &call));
			}
		}
	});
}

#[test]
fn self_collateral_changes_and_authority_retirement_are_not_exits() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		for call in [
			RuntimeCall::BitcoinLocks(pallet_bitcoin_locks::Call::set_flexible {
				lock_id: 1,
				is_flexible: true,
			}),
			RuntimeCall::BitcoinFissions(pallet_bitcoin_fissions::Call::close { fission_id: 1 }),
			RuntimeCall::CrosschainTransfer(
				pallet_crosschain_transfer::Call::deactivate_minting_authority {
					destination_signing_key: [3u8; 20].into(),
				},
			),
		] {
			assert_ok!(AccountSecurity::ensure_authorized(&account(1), &call));
		}

		let localchain =
			RuntimeCall::LocalchainTransfer(pallet_localchain_transfer::Call::send_to_localchain {
				amount: ARGON,
				notary_id: 1,
			});
		assert_eq!(
			AccountSecurity::ensure_authorized(&account(1), &localchain),
			Err(Error::ApprovalRequired)
		);
	});
}

#[test]
fn installing_and_removing_mining_authority_requires_approval() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);

		for (operation, installed) in [
			(
				pallet_proxy::Call::add_proxy {
					delegate: account(4).into(),
					proxy_type: ProxyType::MiningBidRealPaysFee,
					delay: 0,
				},
				true,
			),
			(
				pallet_proxy::Call::remove_proxy {
					delegate: account(4).into(),
					proxy_type: ProxyType::MiningBidRealPaysFee,
					delay: 0,
				},
				false,
			),
		] {
			let operation = RuntimeCall::Proxy(operation);
			assert_eq!(
				Executive::apply_extrinsic(signed(1, operation.clone())),
				Err(Error::ApprovalRequired.into())
			);
			let call = authorized_call(1, operation);
			assert_ok!(Executive::apply_extrinsic(signed(2, call)).unwrap());
			assert_eq!(
				Proxy::find_proxy(&account(1), &account(4), Some(ProxyType::MiningBidRealPaysFee))
					.is_ok(),
				installed
			);
		}
	});
}

#[test]
fn failed_batch_consumes_nonce_and_reports_child_error_without_partial_transfers() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let failure = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: account(4).into(),
			value: 1000 * ARGON,
		});
		let batch = RuntimeCall::Utility(pallet_utility::Call::batch_all {
			calls: vec![transfer(), failure],
		});
		let call = authorized_call(1, batch);
		let before = Balances::free_balance(account(4));
		assert_ok!(Executive::apply_extrinsic(signed(2, call.clone())).unwrap());
		assert_eq!(Balances::free_balance(account(4)), before);
		assert_eq!(
			Executive::apply_extrinsic(signed(2, call)).unwrap(),
			Err(Error::InvalidNonce.into())
		);
		assert!(System::events().iter().any(|record| matches!(
			&record.event,
			RuntimeEvent::AccountSecurity(pallet_account_security::Event::Executed {
				result: Err(_),
				..
			})
		)));
	});
}

#[test]
fn one_device_signature_covers_both_mining_setup_and_external_funding() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let batch = RuntimeCall::Utility(pallet_utility::Call::batch_all {
			calls: vec![
				RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
					delegate: account(5).into(),
					proxy_type: ProxyType::MiningBidRealPaysFee,
					delay: 0,
				}),
				transfer(),
			],
		});
		let call = sign_call(execution_call(1, batch), &[2]);
		assert_ok!(Executive::apply_extrinsic(signed(1, call)).unwrap());
		assert!(Proxy::find_proxy(&account(1), &account(5), Some(ProxyType::MiningBidRealPaysFee))
			.is_ok());
	});
}

#[test]
fn routine_crosschain_collect_requires_no_exit_approval_and_keeps_fee_refund() {
	externalities().execute_with(|| {
		set_vault_delegate(1, account(1), account(5));
		enroll(1, 2, 3);
		let approval = council_approval(account(1));
		assert_ok!(Balances::hold(
			&pallet_vaults::HoldReason::PendingCollect.into(),
			&account(1),
			5 * ARGON
		));
		let before = Balances::free_balance(account(1));
		let batch = RuntimeCall::Utility(pallet_utility::Call::batch_all {
			calls: vec![
				approval,
				RuntimeCall::Vaults(pallet_vaults::Call::collect { vault_id: 1 }),
			],
		});
		assert_ok!(Executive::apply_extrinsic(signed(1, batch)).unwrap());
		assert_eq!(Balances::free_balance(account(1)), before + 5 * ARGON);
	});
}

#[test]
fn zero_balance_devices_supply_proofs_and_main_pays_only_the_submission_fee() {
	externalities().execute_with(|| {
		enroll(1, 20, 21);
		let call = sign_call(execution_call(1, transfer()), &[20]);
		let before = Balances::free_balance(account(1));
		let recipient = Balances::free_balance(account(4));
		assert_ok!(Executive::apply_extrinsic(signed(1, call)).unwrap());
		assert!(Balances::free_balance(account(1)) < before - ARGON);
		assert_eq!(Balances::free_balance(account(4)), recipient + ARGON);
		assert_eq!(Balances::free_balance(account(20)), 0);
		assert_eq!(System::account_nonce(account(20)), 1);
		assert_eq!(System::account(account(20)).sufficients, 1);
	});
}

#[test]
fn nested_pending_multisigs_record_only_the_outer_approval_without_dispatching() {
	externalities().execute_with(|| {
		let mut call =
			RuntimeCall::System(frame_system::Call::remark_with_event { remark: vec![7] });
		for _ in 0..3 {
			call = RuntimeCall::Multisig(MultisigCall::as_multi {
				threshold: 2,
				other_signatories: vec![account(2)],
				maybe_timepoint: None,
				call: Box::new(call),
				max_weight: Weight::zero(),
			});
		}
		let RuntimeCall::Multisig(MultisigCall::as_multi { call: child, .. }) = &call else {
			unreachable!()
		};
		let call_hash = blake2_256(&child.encode());
		let mut signatories = vec![account(1), account(2)];
		signatories.sort();
		let multisig = Multisig::multi_account_id(&signatories, 2);

		assert_ok!(Executive::apply_extrinsic(signed(1, call)).unwrap());
		assert_eq!(
			Multisigs::<Runtime>::get(multisig, call_hash).unwrap().approvals.as_slice(),
			&[account(1)]
		);
		assert_eq!(Multisigs::<Runtime>::iter().count(), 1);
		assert!(!System::events().iter().any(|record| matches!(
			record.event,
			RuntimeEvent::System(frame_system::Event::Remarked { .. })
		)));
	});
}

#[test]
fn stale_or_revoked_proxy_proposals_cannot_charge_the_main_account_through_a_relayer() {
	externalities().execute_with(|| {
		link_accounts(account(5), account(1), account(6));
		enroll(1, 20, 21);
		register_proxy(account(1), account(6), ProxyType::Any, 0);
		let proxied = RuntimeCall::Proxy(pallet_proxy::Call::proxy {
			real: account(1).into(),
			force_proxy_type: None,
			call: Box::new(transfer()),
		});
		let stale = sign_call(execution_call(6, proxied.clone()), &[6, 20]);
		let removal = RuntimeCall::Proxy(pallet_proxy::Call::remove_proxy {
			delegate: account(6).into(),
			proxy_type: ProxyType::Any,
			delay: 0,
		});
		let removal = sign_call(execution_call(1, removal), &[20]);
		assert_ok!(Executive::apply_extrinsic(signed(1, removal)).unwrap());

		let source_before = Balances::free_balance(account(6));
		let destination_before = Balances::free_balance(account(4));
		assert_eq!(
			Executive::apply_extrinsic(signed(5, stale)).unwrap(),
			Err(Error::InvalidNonce.into())
		);
		assert_eq!(Balances::free_balance(account(6)), source_before);
		let fresh = sign_call(execution_call(6, proxied), &[6, 20]);
		let nonce = NextNonce::<Runtime>::get(account(1));
		assert_ok!(Executive::apply_extrinsic(signed(5, fresh)).unwrap());
		assert_eq!(NextNonce::<Runtime>::get(account(1)), nonce + 1);
		assert!(System::events().iter().any(|record| matches!(
			&record.event,
			RuntimeEvent::AccountSecurity(SecurityEvent::Executed { result: Err(error), .. })
				if *error == pallet_proxy::Error::<Runtime>::NotProxy.into()
		)));
		assert_eq!(Balances::free_balance(account(6)), source_before);
		assert_eq!(Balances::free_balance(account(4)), destination_before);
	});
}

#[test]
fn detached_proofs_use_the_real_native_signing_payload_and_bind_the_genesis_hash() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let call = execution_call(1, transfer());
		let payload = AccountSecurity::approval_payload(&call, false).unwrap();
		// A fresh native account supplies the nonce-zero context used by detached proofs.
		let tx = signed(5, call.clone());
		let sp_runtime::generic::Preamble::Signed(_, signature, extra) = tx.preamble else {
			unreachable!()
		};
		let native = SignedPayload::new(call.clone(), extra).unwrap();
		native.using_encoded(|bytes| {
			assert_eq!(bytes, payload.as_slice());
			assert!(signature.verify(bytes, &account(5)));
		});
		let approved = sign_call(call, &[2]);
		let vector = RuntimeCall::AccountSecurity(pallet_account_security::Call::execute {
			account: AccountId::new([0x11; 32]),
			call: Box::new(RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
				dest: AccountId::new([0x22; 32]).into(),
				value: 123,
			})),
			authorization: Authorization {
				group: AccountId::new([0x11; 32]),
				nonce: 9,
				expires_at: 100,
				policy_hash: [0x33; 32],
			},
			approvals: Default::default(),
		});
		let vector = AccountSecurity::approval_payload(&vector, false).unwrap();
		// Shared with the JavaScript signing helper; commits to native SCALE and extensions.
		assert_eq!(
			sp_core::hashing::blake2_256(&vector),
			[
				191, 16, 150, 58, 107, 87, 121, 167, 171, 44, 150, 150, 81, 171, 225, 189, 101,
				167, 18, 45, 17, 36, 63, 217, 164, 124, 155, 173, 20, 155, 201, 52,
			]
		);
		let nonce = NextNonce::<Runtime>::get(account(1));
		frame_system::BlockHash::<Runtime>::insert(0, H256::repeat_byte(7));
		assert_eq!(
			Executive::apply_extrinsic(signed(1, approved)).unwrap(),
			Err(Error::InvalidSignature.into())
		);
		assert_eq!(NextNonce::<Runtime>::get(account(1)), nonce);
	});
}

#[test]
#[ignore = "Requires an existing ECCP256 PIV key; run testing/signers/run.sh yubikey"]
fn yubikey_piv_enrolls_and_authorizes_without_a_funded_signer() {
	use argon_primitives::account_security::P256Public;
	use p256::{pkcs8::DecodePublicKey, PublicKey};
	use std::{env, path::PathBuf, process::Command};

	let slot = env::var("ARGON_YUBIKEY_SLOT").expect("explicit existing PIV slot required");
	let directory = PathBuf::from(env::var_os("ARGON_YUBIKEY_TEST_DIR").unwrap());
	let certificate = directory.join("certificate.pem");
	assert!(Command::new("yubico-piv-tool")
		.args(["-a", "read-certificate", "-s", &slot, "-o"])
		.arg(&certificate)
		.status()
		.expect("yubico-piv-tool must be installed")
		.success());
	let pem = Command::new("openssl")
		.args(["x509", "-pubkey", "-noout", "-in"])
		.arg(&certificate)
		.output()
		.expect("openssl must be installed");
	assert!(pem.status.success());
	let public = PublicKey::from_public_key_pem(std::str::from_utf8(&pem.stdout).unwrap())
		.expect("the existing slot certificate must contain a P-256 key");
	let public = P256Public::from_sec1_bytes(&public.to_sec1_bytes()).unwrap();
	let device = public.signer_id();

	externalities().execute_with(|| {
		let mut policy = enrollment(2, 3);
		policy.signers = vec![device.clone()].try_into().unwrap();
		let register = sign_call(registration_call(1, policy), &[3]);
		let register = yubikey_approval(register, &public, &slot, &directory);
		assert_ok!(Executive::apply_extrinsic(signed(1, register)).unwrap());

		assert_eq!(System::account(&device).sufficients, 1);
		assert_eq!(System::account_nonce(&device), 1);
		assert_eq!(Balances::free_balance(&device), 0);
		let execute = yubikey_approval(execution_call(1, transfer()), &public, &slot, &directory);
		let mut altered = execute.clone();
		let RuntimeCall::AccountSecurity(pallet_account_security::Call::execute { call, .. }) =
			&mut altered
		else {
			unreachable!()
		};
		let RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive { value, .. }) =
			call.as_mut()
		else {
			unreachable!()
		};
		*value += ARGON;

		let balance = Balances::free_balance(account(4));
		let nonce = NextNonce::<Runtime>::get(account(1));
		assert_eq!(
			Executive::apply_extrinsic(signed(1, altered)).unwrap(),
			Err(Error::InvalidSignature.into())
		);
		assert_eq!(Balances::free_balance(account(4)), balance);
		assert_eq!(NextNonce::<Runtime>::get(account(1)), nonce);

		assert_ok!(Executive::apply_extrinsic(signed(1, execute.clone())).unwrap());
		assert_eq!(Balances::free_balance(account(4)), balance + ARGON);
		assert_eq!(NextNonce::<Runtime>::get(account(1)), nonce + 1);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, execute)).unwrap(),
			Err(Error::InvalidNonce.into())
		);
		assert_eq!(System::account(&device).sufficients, 1);
		assert_eq!(System::account_nonce(&device), 1);
	});
}

fn transfer() -> RuntimeCall {
	RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
		dest: account(4).into(),
		value: ARGON,
	})
}
