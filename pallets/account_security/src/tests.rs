use crate::{
	mock::{System, *},
	*,
};
use argon_primitives::{AccountSecurityProvider, Signature};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError;

type SecurityError = Error<Test>;

#[test]
fn inclusion_rechecks_protection_that_changed_after_pool_validation() {
	externalities().execute_with(|| {
		let exit = signed(1, transfer());
		assert_ok!(Executive::validate_transaction(
			TransactionSource::External,
			exit.clone(),
			Default::default()
		));

		// Enrollment changes security state without consuming the pending transaction's nonce.
		let enrollment = sign_call(registration_call(1, enrollment(2, 3)), &[2, 3]);
		assert_ok!(enrollment.dispatch(RuntimeOrigin::signed(account(1))));
		let recipient_before = Balances::free_balance(account(4));
		assert_eq!(Executive::apply_extrinsic(exit), Err(SecurityError::ApprovalRequired.into()));
		assert_eq!(Balances::free_balance(account(4)), recipient_before);
	});
}

#[test]
fn unfunded_signers_retain_native_nonces_without_spending_them_on_approvals() {
	externalities().execute_with(|| {
		enroll(1, 20, 21);
		for signer in [20, 21] {
			assert_eq!(System::account(account(signer)).sufficients, 1);
			assert_eq!(System::account_nonce(account(signer)), 1);
			assert_eq!(Balances::free_balance(account(signer)), 0);
		}
		let call = sign_call(execution_call(1, transfer()), &[20]);
		assert_ok!(Executive::apply_extrinsic(signed(1, call)).unwrap());
		assert_eq!(System::account_nonce(account(20)), 1);
		assert_eq!(Balances::free_balance(account(20)), 0);

		let approved = sign_call(execution_call(1, transfer()), &[20]);
		let RuntimeCall::AccountSecurity(Call::execute { approvals, .. }) = &approved else {
			unreachable!()
		};
		let ApprovalSignature::Sr25519(signature) = approvals[0].signature.clone() else {
			unreachable!()
		};
		let replay = UncheckedExtrinsic::new_signed(
			execution_call(1, transfer()),
			account(20),
			Signature::Sr25519(signature),
			(
				frame_system::CheckNonce::<Test>::from(0),
				CheckAccountSecurity::<Test>::new(),
				frame_metadata_hash_extension::CheckMetadataHash::<Test>::new(false),
			),
		);
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), account(20), ARGON));
		assert_eq!(Executive::apply_extrinsic(replay), Err(InvalidTransaction::Stale.into()));
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), account(20), 0));
		assert_eq!(System::account_nonce(account(20)), 1);
	});
}

#[test]
fn shared_signers_survive_pending_cancellation_promotion_and_balance_reaping() {
	externalities().execute_with(|| {
		let mut initial = enrollment(20, 21);
		initial.recovery_delay = 10;
		let register = sign_call(registration_call(1, initial.clone()), &[20, 21]);
		assert_ok!(Executive::apply_extrinsic(signed(1, register)).unwrap());
		enroll(6, 20, 21);
		assert_eq!(System::account(account(20)).sufficients, 2);

		let mut replacement = initial;
		replacement.signers = vec![account(22)].try_into().unwrap();
		let update = sign_call(update_call(1, replacement.clone()), &[21, 22]);
		assert_ok!(Executive::apply_extrinsic(signed(1, update)).unwrap());
		let pending = PendingPolicyUpdates::<Test>::get(account(1)).unwrap();
		assert_eq!(System::account(account(21)).sufficients, 3);
		assert_eq!(System::account(account(22)).sufficients, 1);
		assert_eq!(System::account_nonce(account(22)), 1);

		let cancel = sign_call(cancel_call(1, pending.nonce), &[20]);
		assert_ok!(Executive::apply_extrinsic(signed(1, cancel)).unwrap());
		assert!(!frame_system::Account::<Test>::contains_key(account(22)));
		assert_eq!(System::account(account(20)).sufficients, 2);
		assert_eq!(System::account(account(21)).sufficients, 2);

		let update = sign_call(update_call(1, replacement), &[21, 22]);
		assert_ok!(Executive::apply_extrinsic(signed(1, update)).unwrap());
		let pending = PendingPolicyUpdates::<Test>::get(account(1)).unwrap();
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), account(22), ARGON,));
		let remark = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
		assert_ok!(Executive::apply_extrinsic(signed(22, remark)).unwrap());
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), account(22), 0));
		assert_eq!(System::account(account(22)).providers, 0);
		assert_eq!(System::account_nonce(account(22)), 2);

		CurrentTick::set(10);
		let nonce = NextNonce::<Test>::get(account(1));
		NextNonce::<Test>::insert(account(1), u64::MAX);
		assert_noop!(
			AccountSecurity::complete_policy_update(
				RuntimeOrigin::signed(account(5)),
				account(1),
				pending.nonce,
			),
			SecurityError::CounterExhausted
		);
		assert!(PendingPolicyUpdates::<Test>::contains_key(account(1)));
		assert_eq!(System::account(account(20)).sufficients, 2);
		assert_eq!(System::account(account(22)).sufficients, 1);
		assert_eq!(System::account_nonce(account(22)), 2);

		NextNonce::<Test>::insert(account(1), nonce);
		assert_ok!(AccountSecurity::complete_policy_update(
			RuntimeOrigin::signed(account(5)),
			account(1),
			pending.nonce,
		));
		assert_eq!(System::account(account(20)).sufficients, 1);
		assert_eq!(System::account(account(21)).sufficients, 2);
		assert_eq!(System::account(account(22)).sufficients, 1);
		assert_eq!(System::account_nonce(account(22)), 2);
	});
}

#[test]
fn enrollment_requires_every_invitee_and_backup_even_when_quorum_is_smaller() {
	externalities().execute_with(|| {
		let configuration = policy(&[2, 4, 5, 6], 1, 3);
		let call = registration_call(1, configuration.clone());
		for seeds in [&[2, 5, 3][..], &[2, 4, 5, 6][..]] {
			let missing = sign_call(call.clone(), seeds);
			assert_eq!(
				Executive::apply_extrinsic(signed(1, missing)).unwrap(),
				Err(SecurityError::ApprovalRequired.into())
			);
		}
		assert!(!Registrations::<Test>::contains_key(account(1)));
		assert_eq!(Balances::reserved_balance(account(1)), 0);
		assert_eq!(NextNonce::<Test>::get(account(1)), 0);

		let accepted = sign_call(call, &[2, 4, 5, 6, 3]);
		assert_ok!(Executive::apply_extrinsic(signed(1, accepted)).unwrap());
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, configuration);
		assert_eq!(NextNonce::<Test>::get(account(1)), 1);
		let duplicate = sign_call(registration_call(1, enrollment(2, 3)), &[2, 3]);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, duplicate)).unwrap(),
			Err(SecurityError::AlreadyRegistered.into())
		);
	});
}

#[test]
fn linking_moves_protection_without_duplicating_deposits_or_nonces() {
	for (default_protected, deposit_base) in [(false, 10), (true, 10), (true, 6)] {
		externalities().execute_with(|| {
			SecurityDepositBase::set(10);
			if default_protected {
				enroll(1, 2, 3);
			}
			enroll(6, 2, 3);
			let registration = Registrations::<Test>::get(account(6)).unwrap();
			let default_free = Balances::free_balance(account(1));
			let default_reserved = Balances::reserved_balance(account(1));
			let member_free = Balances::free_balance(account(6));
			let default_nonce = NextNonce::<Test>::get(account(1));
			let member_nonce = NextNonce::<Test>::get(account(6));
			SecurityDepositBase::set(deposit_base);
			let deposit = deposit_base + 4;

			assert_ok!(AccountSecurity::on_operational_registration(
				&account(1),
				&[account(6), account(1), account(6)],
			));

			let linked = Registrations::<Test>::get(account(1)).unwrap();
			assert_eq!(linked.policy, registration.policy);
			assert_eq!(linked.deposit, deposit);
			assert!(!Registrations::<Test>::contains_key(account(6)));
			assert_eq!(Balances::reserved_balance(account(1)), deposit);
			assert_eq!(Balances::reserved_balance(account(6)), 0);
			assert_eq!(Balances::free_balance(account(6)), member_free + registration.deposit);
			assert_eq!(
				Balances::free_balance(account(1)),
				default_free + default_reserved - deposit,
			);
			assert_eq!(NextNonce::<Test>::get(account(1)), default_nonce + 1);
			assert_eq!(NextNonce::<Test>::get(account(6)), member_nonce + 1);
			for signer in [2, 3] {
				assert_eq!(System::account(account(signer)).sufficients, 1);
			}
			SecurityDepositBase::set(10);
		});
	}
}

#[test]
fn incompatible_group_link_is_atomic() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		enroll(6, 4, 5);
		let first = Registrations::<Test>::get(account(1)).unwrap();
		assert_noop!(
			AccountSecurity::on_operational_registration(&account(1), &[account(1), account(6)]),
			SecurityError::IncompatibleAccountProtection
		);
		assert_eq!(Registrations::<Test>::get(account(1)), Some(first));
		assert!(Registrations::<Test>::contains_key(account(6)));
	});
}

#[test]
fn one_quorum_authorizes_a_batch_of_authority_changes_and_exits() {
	externalities().execute_with(|| {
		let setup = policy(&[2, 4, 5], 2, 3);
		let registration = sign_call(registration_call(1, setup), &[2, 4, 5, 3]);
		assert_ok!(Executive::apply_extrinsic(signed(1, registration)).unwrap());

		let install = RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
			delegate: account(6),
			proxy_type: ProxyType::Mining,
			delay: 0,
		});
		let operation = RuntimeCall::Utility(pallet_utility::Call::batch_all {
			calls: vec![install, transfer()],
		});
		let call = execution_call(1, operation);
		for seeds in [&[2][..], &[4][..], &[5][..]] {
			let incomplete = sign_call(call.clone(), seeds);
			assert_eq!(
				Executive::apply_extrinsic(signed(1, incomplete)).unwrap(),
				Err(SecurityError::ApprovalRequired.into())
			);
		}
		assert!(Proxy::find_proxy(&account(1), &account(6), None).is_err());
		let before = Balances::free_balance(account(4));
		let complete = sign_call(call, &[4, 5]);
		assert_ok!(Executive::apply_extrinsic(signed(1, complete)).unwrap());
		assert!(Proxy::find_proxy(&account(1), &account(6), None).is_ok());
		assert_eq!(Balances::free_balance(account(4)), before + ARGON);
	});
}

#[test]
fn one_signer_can_approve_a_batch_but_duplicate_proofs_are_rejected() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let install = RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
			delegate: account(6),
			proxy_type: ProxyType::Mining,
			delay: 0,
		});
		let call = execution_call(
			1,
			RuntimeCall::Utility(pallet_utility::Call::batch_all {
				calls: vec![install, transfer()],
			}),
		);
		let duplicate = sign_call(call.clone(), &[2, 2]);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, duplicate)).unwrap(),
			Err(SecurityError::DuplicateApproval.into())
		);
		let complete = sign_call(call, &[2]);
		assert_ok!(Executive::apply_extrinsic(signed(1, complete)).unwrap());
	});
}

#[test]
fn a_relayer_cannot_execute_without_main_account_consent() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let call = execution_call(1, transfer());
		let exit_only = sign_call(call.clone(), &[2]);
		assert_eq!(
			Executive::apply_extrinsic(signed(5, exit_only)).unwrap(),
			Err(SecurityError::ApprovalRequired.into())
		);
		let both = sign_call(call, &[1, 2]);
		assert_ok!(Executive::apply_extrinsic(signed(5, both)).unwrap());
		assert_eq!(NextNonce::<Test>::get(account(1)), 2);
	});
}

#[test]
fn a_relayer_can_enroll_with_main_and_all_initial_authority_proofs() {
	externalities().execute_with(|| {
		let registration = sign_call(registration_call(1, enrollment(20, 21)), &[1, 20, 21]);
		assert_ok!(Executive::apply_extrinsic(signed(5, registration)).unwrap());
		assert!(Registrations::<Test>::contains_key(account(1)));
		assert_eq!(System::account(account(20)).sufficients, 1);
	});
}

#[test]
fn normal_policy_changes_require_current_quorum_and_acceptance_of_new_signers() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let replacement = enrollment(4, 3);
		let call = update_call(1, replacement.clone());
		for seeds in [&[2][..], &[4][..]] {
			let incomplete = sign_call(call.clone(), seeds);
			assert_eq!(
				Executive::apply_extrinsic(signed(1, incomplete)).unwrap(),
				Err(SecurityError::ApprovalRequired.into())
			);
		}
		let complete = sign_call(call, &[2, 4]);
		assert_ok!(Executive::apply_extrinsic(signed(1, complete)).unwrap());
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, replacement);

		let replacement = enrollment(2, 3);
		let change_signer = sign_call(update_call(1, replacement), &[4]);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, change_signer)).unwrap(),
			Err(SecurityError::ApprovalRequired.into())
		);
	});
}

#[test]
fn the_same_update_api_recovers_lost_signers_with_main_backup_and_new_devices() {
	externalities().execute_with(|| {
		enroll(1, 20, 21);
		let stale = sign_call(execution_call(1, transfer()), &[20]);
		let replacement = enrollment(22, 21);
		let recovery = sign_call(update_call(1, replacement.clone()), &[21, 22]);
		assert_ok!(Executive::apply_extrinsic(signed(1, recovery)).unwrap());
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, replacement);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, stale)).unwrap(),
			Err(SecurityError::InvalidNonce.into())
		);
		let fresh = sign_call(execution_call(1, transfer()), &[22]);
		assert_ok!(Executive::apply_extrinsic(signed(1, fresh)).unwrap());
		assert_eq!(System::account(account(20)).sufficients, 0);
	});
}

#[test]
fn replacing_a_lost_backup_accepts_main_quorum_and_the_new_backup() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let replacement = enrollment(2, 4);
		let call = update_call(1, replacement.clone());
		for seeds in [&[2][..], &[3, 4][..]] {
			let incomplete = sign_call(call.clone(), seeds);
			assert_eq!(
				Executive::apply_extrinsic(signed(1, incomplete)).unwrap(),
				Err(SecurityError::ApprovalRequired.into())
			);
		}
		let complete = sign_call(call, &[2, 4]);
		assert_ok!(Executive::apply_extrinsic(signed(1, complete)).unwrap());
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, replacement);
	});
}

#[test]
fn backup_replacement_waits_ticks_not_blocks_and_survives_unrelated_operations() {
	externalities().execute_with(|| {
		let mut initial = enrollment(2, 3);
		initial.recovery_delay = 10;
		let register = sign_call(registration_call(1, initial.clone()), &[2, 3]);
		assert_ok!(Executive::apply_extrinsic(signed(1, register)).unwrap());
		let active_deposit = Balances::reserved_balance(account(1));

		CurrentTick::set(50);
		let mut replacement = initial.clone();
		replacement.recovery_account = account(4);
		let update = sign_call(update_call(1, replacement.clone()), &[2, 4]);
		assert_ok!(Executive::apply_extrinsic(signed(1, update)).unwrap());
		let pending = PendingPolicyUpdates::<Test>::get(account(1)).unwrap();
		assert_eq!(pending.execute_after_tick, 60);
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, initial);
		assert_eq!(Balances::reserved_balance(account(1)), active_deposit + pending.deposit);

		let operation = sign_call(execution_call(1, transfer()), &[2]);
		assert_ok!(Executive::apply_extrinsic(signed(1, operation)).unwrap());
		System::set_block_number(200); // The original approval expired, but was already accepted.
		CurrentTick::set(59);
		assert_noop!(
			AccountSecurity::complete_policy_update(
				RuntimeOrigin::signed(account(5)),
				account(1),
				pending.nonce
			),
			SecurityError::RecoveryDelayActive
		);
		CurrentTick::set(60);
		assert_noop!(
			AccountSecurity::complete_policy_update(
				RuntimeOrigin::signed(account(5)),
				account(1),
				pending.nonce + 1
			),
			SecurityError::UnknownPolicyUpdate
		);
		assert_ok!(AccountSecurity::complete_policy_update(
			RuntimeOrigin::signed(account(5)),
			account(1),
			pending.nonce
		));
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, replacement);
		assert_eq!(Balances::reserved_balance(account(1)), active_deposit);
		assert!(!PendingPolicyUpdates::<Test>::contains_key(account(1)));
		assert_eq!(NextNonce::<Test>::get(account(1)), 4);
		assert_noop!(
			AccountSecurity::complete_policy_update(
				RuntimeOrigin::signed(account(5)),
				account(1),
				pending.nonce
			),
			SecurityError::UnknownPolicyUpdate
		);
	});
}

#[test]
fn recovery_and_backup_replacement_can_each_be_vetoed_by_the_other_authority() {
	for recover_devices in [true, false] {
		externalities().execute_with(|| {
			let mut initial = enrollment(2, 3);
			initial.recovery_delay = 10;
			let register = sign_call(registration_call(1, initial.clone()), &[2, 3]);
			assert_ok!(Executive::apply_extrinsic(signed(1, register)).unwrap());
			let reserve = Balances::reserved_balance(account(1));
			let mut replacement = initial.clone();
			let (proposers, veto): (&[u8], &[u8]) = if recover_devices {
				replacement.signers = vec![account(4)].try_into().unwrap();
				(&[3, 4], &[2])
			} else {
				replacement.recovery_account = account(4);
				(&[2, 4], &[3])
			};
			let update = sign_call(update_call(1, replacement.clone()), proposers);
			assert_ok!(Executive::apply_extrinsic(signed(1, update)).unwrap());
			let pending = PendingPolicyUpdates::<Test>::get(account(1)).unwrap();
			let retry = sign_call(update_call(1, replacement.clone()), proposers);
			assert_eq!(
				Executive::apply_extrinsic(signed(1, retry)).unwrap(),
				Err(SecurityError::PolicyUpdatePending.into())
			);
			let cancel = sign_call(cancel_call(1, pending.nonce), veto);
			let nonce = NextNonce::<Test>::get(account(1));
			let remark = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
			let main_only = execution_call(1, remark.clone());
			assert_eq!(
				Executive::apply_extrinsic(signed(1, main_only)).unwrap(),
				Err(SecurityError::ApprovalRequired.into())
			);
			assert_eq!(NextNonce::<Test>::get(account(1)), nonce);
			assert_ok!(Executive::apply_extrinsic(signed(1, remark)).unwrap());
			assert_eq!(NextNonce::<Test>::get(account(1)), nonce);
			assert_ok!(Executive::apply_extrinsic(signed(1, cancel.clone())).unwrap());
			assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, initial);
			assert_eq!(Balances::reserved_balance(account(1)), reserve);
			assert!(!PendingPolicyUpdates::<Test>::contains_key(account(1)));
			assert_eq!(
				Executive::apply_extrinsic(signed(1, cancel)).unwrap(),
				Err(SecurityError::InvalidNonce.into())
			);

			let update = sign_call(update_call(1, replacement.clone()), proposers);
			assert_ok!(Executive::apply_extrinsic(signed(1, update)).unwrap());
			CurrentTick::set(10);
			assert_noop!(
				AccountSecurity::complete_policy_update(
					RuntimeOrigin::signed(account(5)),
					account(1),
					pending.nonce
				),
				SecurityError::UnknownPolicyUpdate
			);
			let next = PendingPolicyUpdates::<Test>::get(account(1)).unwrap();
			assert_ok!(AccountSecurity::complete_policy_update(
				RuntimeOrigin::signed(account(5)),
				account(1),
				next.nonce
			));
			assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, replacement);
		});
	}
}

#[test]
fn shortening_the_delay_waits_the_old_window_and_backup_cannot_change_its_own_authority() {
	externalities().execute_with(|| {
		let mut initial = enrollment(2, 3);
		initial.recovery_delay = 20;
		let register = sign_call(registration_call(1, initial.clone()), &[2, 3]);
		assert_ok!(Executive::apply_extrinsic(signed(1, register)).unwrap());
		let mut replacement = initial.clone();
		replacement.recovery_delay = 0;
		let backup_only = sign_call(update_call(1, replacement.clone()), &[3]);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, backup_only)).unwrap(),
			Err(SecurityError::ApprovalRequired.into())
		);
		let quorum = sign_call(update_call(1, replacement.clone()), &[2]);
		assert_ok!(Executive::apply_extrinsic(signed(1, quorum)).unwrap());
		assert_eq!(PendingPolicyUpdates::<Test>::get(account(1)).unwrap().execute_after_tick, 20);
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, initial);
		CurrentTick::set(20);
		assert_ok!(AccountSecurity::complete_policy_update(
			RuntimeOrigin::signed(account(5)),
			account(1),
			1
		));
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, replacement);
	});
}

#[test]
fn a_pure_proxy_recovery_origin_accepts_and_recovers_but_cannot_impersonate_the_main_key() {
	externalities().execute_with(|| {
		let recovery = Proxy::pure_account(&account(5), &ProxyType::Any, 0, None);
		assert_ok!(Proxy::create_pure(RuntimeOrigin::signed(account(5)), ProxyType::Any, 0, 0));
		let mut initial = enrollment(2, 3);
		initial.recovery_account = recovery.clone();
		initial.recovery_delay = 10;
		let proxy_call = |call: RuntimeCall| {
			RuntimeCall::Proxy(pallet_proxy::Call::proxy {
				real: recovery.clone(),
				force_proxy_type: None,
				call: Box::new(call),
			})
		};
		let missing_main = sign_call(registration_call(1, initial.clone()), &[2]);
		assert_ok!(Executive::apply_extrinsic(signed(5, proxy_call(missing_main))).unwrap());
		assert!(!Registrations::<Test>::contains_key(account(1)));
		let accepted = sign_call(registration_call(1, initial.clone()), &[1, 2]);
		assert_ok!(Executive::apply_extrinsic(signed(5, proxy_call(accepted))).unwrap());
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, initial);
		assert_eq!(Balances::free_balance(&recovery), 0);

		let mut replacement = initial;
		replacement.signers = vec![account(4)].try_into().unwrap();
		let recover = sign_call(update_call(1, replacement.clone()), &[1, 4]);
		assert_ok!(Executive::apply_extrinsic(signed(5, proxy_call(recover))).unwrap());
		CurrentTick::set(10);
		assert_ok!(AccountSecurity::complete_policy_update(
			RuntimeOrigin::signed(account(6)),
			account(1),
			1
		));
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, replacement);

		// The same native recovery authority can veto replacement of itself.
		let mut next = replacement.clone();
		next.recovery_account = account(3);
		let update = sign_call(update_call(1, next), &[4, 3]);
		assert_ok!(Executive::apply_extrinsic(signed(1, update)).unwrap());
		let pending = PendingPolicyUpdates::<Test>::get(account(1)).unwrap();
		let veto = sign_call(cancel_call(1, pending.nonce), &[1]);
		assert_ok!(Executive::apply_extrinsic(signed(5, proxy_call(veto))).unwrap());
		assert!(!PendingPolicyUpdates::<Test>::contains_key(account(1)));
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, replacement);
	});
}

#[test]
fn native_multisig_recovery_acceptance_uses_its_real_quorum() {
	externalities().execute_with(|| {
		let mut members = vec![account(4), account(5)];
		members.sort();
		let recovery = Multisig::multi_account_id(&members, 2);
		let mut initial = enrollment(2, 3);
		initial.recovery_account = recovery.clone();
		let enrollment = sign_call(registration_call(1, initial.clone()), &[1, 2]);
		let hash = enrollment.using_encoded(blake2_256);
		let first = RuntimeCall::Multisig(pallet_multisig::Call::as_multi {
			threshold: 2,
			other_signatories: vec![account(5)],
			maybe_timepoint: None,
			call: Box::new(enrollment.clone()),
			max_weight: Weight::MAX,
		});
		assert_ok!(Executive::apply_extrinsic(signed(4, first)).unwrap());
		assert!(!Registrations::<Test>::contains_key(account(1)));
		let pending = pallet_multisig::Multisigs::<Test>::get(&recovery, hash).unwrap();
		let second = RuntimeCall::Multisig(pallet_multisig::Call::as_multi {
			threshold: 2,
			other_signatories: vec![account(4)],
			maybe_timepoint: Some(pending.when),
			call: Box::new(enrollment),
			max_weight: Weight::from_parts(500_000_000, 20_000),
		});
		assert_ok!(Executive::apply_extrinsic(signed(5, second)).unwrap());
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, initial);
		assert_eq!(Balances::free_balance(&recovery), 0);
	});
}

#[test]
fn wrapping_enrollment_cannot_hide_a_later_unapproved_exit() {
	externalities().execute_with(|| {
		register_proxy(account(3), account(5), ProxyType::Any, 0);
		register_proxy(account(1), account(5), ProxyType::Any, 0);
		let register = sign_call(registration_call(1, enrollment(2, 3)), &[1, 2]);
		let batch = RuntimeCall::Utility(pallet_utility::Call::batch_all {
			calls: vec![
				RuntimeCall::Proxy(pallet_proxy::Call::proxy {
					real: account(3),
					force_proxy_type: None,
					call: Box::new(register),
				}),
				RuntimeCall::Proxy(pallet_proxy::Call::proxy {
					real: account(1),
					force_proxy_type: None,
					call: Box::new(transfer()),
				}),
			],
		});
		assert_eq!(
			Executive::apply_extrinsic(signed(5, batch)),
			Err(SecurityError::UnsupportedComposition.into())
		);
		assert!(!Registrations::<Test>::contains_key(account(1)));
	});
}

#[test]
fn pending_storage_is_deposit_backed_and_its_reserve_funds_the_completed_policy() {
	externalities().execute_with(|| {
		let mut initial = enrollment(2, 3);
		initial.recovery_delay = 10;
		let register = sign_call(registration_call(1, initial.clone()), &[2, 3]);
		assert_ok!(Executive::apply_extrinsic(signed(1, register)).unwrap());
		let active_reserve = Balances::reserved_balance(account(1));
		let mut replacement = policy(&[4, 5, 6], 2, 3);
		replacement.recovery_delay = 10;
		let update = sign_call(update_call(1, replacement.clone()), &[3, 4, 5, 6]);
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), account(1), 1));
		assert!(Executive::apply_extrinsic(signed(1, update.clone())).unwrap().is_err());
		assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, initial);
		assert_eq!(NextNonce::<Test>::get(account(1)), 1);
		assert!(!PendingPolicyUpdates::<Test>::contains_key(account(1)));
		assert_eq!(Balances::reserved_balance(account(1)), active_reserve);

		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), account(1), ARGON));
		assert_ok!(Executive::apply_extrinsic(signed(1, update)).unwrap());
		let pending = PendingPolicyUpdates::<Test>::get(account(1)).unwrap();
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), account(1), 1));
		CurrentTick::set(10);
		assert_ok!(AccountSecurity::complete_policy_update(
			RuntimeOrigin::signed(account(7)),
			account(1),
			pending.nonce
		));
		let registration = Registrations::<Test>::get(account(1)).unwrap();
		assert_eq!(registration.policy, replacement);
		assert_eq!(Balances::reserved_balance(account(1)), registration.deposit);
		assert_eq!(
			Balances::free_balance(account(1)),
			1 + active_reserve + pending.deposit - registration.deposit
		);
	});
}

#[test]
fn normal_updates_and_operational_linking_clear_pending_records_and_refund_their_reserves() {
	for link in [false, true] {
		externalities().execute_with(|| {
			let mut initial = enrollment(2, 3);
			initial.recovery_delay = 10;
			let register = sign_call(registration_call(1, initial.clone()), &[2, 3]);
			assert_ok!(Executive::apply_extrinsic(signed(1, register)).unwrap());
			let reserve = Balances::reserved_balance(account(1));
			let mut replacement = initial.clone();
			replacement.recovery_account = account(4);
			let update = sign_call(update_call(1, replacement), &[2, 4]);
			assert_ok!(Executive::apply_extrinsic(signed(1, update)).unwrap());
			if link {
				assert_ok!(AccountSecurity::on_operational_registration(
					&account(1),
					&[account(1), account(6)]
				));
			} else {
				let update = sign_call(update_call(1, initial.clone()), &[2]);
				assert_ok!(Executive::apply_extrinsic(signed(1, update)).unwrap());
			}
			assert_eq!(Balances::reserved_balance(account(1)), reserve);
			assert_eq!(Registrations::<Test>::get(account(1)).unwrap().policy, initial);
			assert!(!PendingPolicyUpdates::<Test>::contains_key(account(1)));
		});
	}
}

#[test]
fn restoring_an_older_policy_cannot_revive_a_previously_signed_operation() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let stale = sign_call(execution_call(1, transfer()), &[2]);
		let replacement = sign_call(update_call(1, enrollment(4, 3)), &[2, 4]);
		assert_ok!(Executive::apply_extrinsic(signed(1, replacement)).unwrap());
		let restore = sign_call(update_call(1, enrollment(2, 3)), &[4, 2]);
		assert_ok!(Executive::apply_extrinsic(signed(1, restore)).unwrap());
		assert_eq!(
			Executive::apply_extrinsic(signed(1, stale)).unwrap(),
			Err(SecurityError::InvalidNonce.into())
		);
	});
}

#[test]
fn changing_call_or_signed_metadata_invalidates_the_proofs() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let signed_call = sign_call(execution_call(1, transfer()), &[2]);
		let mut changed = signed_call.clone();
		let RuntimeCall::AccountSecurity(Call::execute { call, .. }) = &mut changed else {
			unreachable!()
		};
		*call = Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![] }));
		assert_eq!(
			Executive::apply_extrinsic(signed(1, changed)).unwrap(),
			Err(SecurityError::InvalidSignature.into())
		);

		let mut changed = signed_call.clone();
		let RuntimeCall::AccountSecurity(Call::execute { authorization, .. }) = &mut changed else {
			unreachable!()
		};
		authorization.expires_at = 99;
		assert_eq!(
			Executive::apply_extrinsic(signed(1, changed)).unwrap(),
			Err(SecurityError::InvalidSignature.into())
		);

		let mut changed = signed_call;
		let RuntimeCall::AccountSecurity(Call::execute { authorization, .. }) = &mut changed else {
			unreachable!()
		};
		authorization.policy_hash = [7; 32];
		assert_eq!(
			Executive::apply_extrinsic(signed(1, changed)).unwrap(),
			Err(SecurityError::StalePolicy.into())
		);
		assert_eq!(NextNonce::<Test>::get(account(1)), 1);
	});
}

#[test]
fn expiry_nonce_and_group_bounds_are_checked_without_consuming_authorization() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let mut expired = execution_call(1, transfer());
		let RuntimeCall::AccountSecurity(Call::execute { authorization, .. }) = &mut expired else {
			unreachable!()
		};
		authorization.expires_at = 1;
		let expired = sign_call(expired, &[2]);
		System::set_block_number(2);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, expired)).unwrap(),
			Err(SecurityError::InvalidExpiry.into())
		);

		let mut wrong_group = execution_call(1, transfer());
		let RuntimeCall::AccountSecurity(Call::execute { authorization, .. }) = &mut wrong_group
		else {
			unreachable!()
		};
		authorization.group = account(5);
		let wrong_group = sign_call(wrong_group, &[2]);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, wrong_group)).unwrap(),
			Err(SecurityError::StalePolicy.into())
		);

		NextNonce::<Test>::insert(account(1), u64::MAX);
		let exhausted = sign_call(execution_call(1, transfer()), &[2]);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, exhausted)).unwrap(),
			Err(SecurityError::CounterExhausted.into())
		);
		assert_eq!(NextNonce::<Test>::get(account(1)), u64::MAX);
	});
}

#[test]
fn one_successful_execution_consumes_the_group_nonce_for_all_competing_proposals() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let first = sign_call(execution_call(1, transfer()), &[2]);
		let second = first.clone();
		let before = Balances::free_balance(account(4));
		assert_ok!(Executive::apply_extrinsic(signed(1, first)).unwrap());
		assert_eq!(
			Executive::apply_extrinsic(signed(1, second)).unwrap(),
			Err(SecurityError::InvalidNonce.into())
		);
		assert_eq!(Balances::free_balance(account(4)), before + ARGON);
	});
}

#[test]
fn failed_atomic_execution_consumes_the_nonce_without_leaking_partial_effects() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let failure = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: account(4),
			value: 1000 * ARGON,
		});
		let batch = RuntimeCall::Utility(pallet_utility::Call::batch_all {
			calls: vec![transfer(), failure],
		});
		let call = sign_call(execution_call(1, batch), &[2]);
		let before = Balances::free_balance(account(4));
		assert_ok!(Executive::apply_extrinsic(signed(1, call.clone())).unwrap());
		assert_eq!(Balances::free_balance(account(4)), before);
		assert_eq!(NextNonce::<Test>::get(account(1)), 2);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, call)).unwrap(),
			Err(SecurityError::InvalidNonce.into())
		);
		assert!(System::events().iter().any(|record| matches!(
			&record.event,
			RuntimeEvent::AccountSecurity(Event::Executed { nonce: 1, result: Err(_), .. })
		)));
	});
}

#[test]
fn native_proxy_and_nested_security_calls_cannot_forge_authorization() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		register_proxy(account(1), account(2), ProxyType::Any, 0);
		let native = RuntimeCall::Proxy(pallet_proxy::Call::proxy {
			real: account(1),
			force_proxy_type: None,
			call: Box::new(transfer()),
		});
		assert_eq!(
			Executive::apply_extrinsic(signed(2, native)),
			Err(SecurityError::ApprovalRequired.into())
		);

		let security = sign_call(execution_call(1, transfer()), &[2]);
		let proxy = RuntimeCall::Proxy(pallet_proxy::Call::proxy {
			real: account(1),
			force_proxy_type: None,
			call: Box::new(security.clone()),
		});
		assert_eq!(
			Executive::apply_extrinsic(signed(2, proxy)),
			Err(SecurityError::UnsupportedComposition.into())
		);

		let batch = RuntimeCall::Utility(pallet_utility::Call::batch_all { calls: vec![security] });
		assert_eq!(
			Executive::apply_extrinsic(signed(1, batch)),
			Err(SecurityError::UnsupportedComposition.into())
		);

		let recursive = sign_call(execution_call(1, execution_call(1, transfer())), &[2]);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, recursive)).unwrap(),
			Err(SecurityError::UnsupportedComposition.into())
		);
	});
}

#[test]
fn invalid_policy_and_deposit_failures_leave_registration_and_nonce_unchanged() {
	externalities().execute_with(|| {
		let mut zero_threshold = enrollment(2, 3);
		zero_threshold.quorum = 0;
		let mut duplicate_signers = enrollment(2, 3);
		duplicate_signers.signers = vec![account(2), account(2)].try_into().unwrap();
		for configuration in [zero_threshold, duplicate_signers] {
			let call = sign_call(registration_call(1, configuration), &[2, 3]);
			assert_eq!(
				Executive::apply_extrinsic(signed(1, call)).unwrap(),
				Err(SecurityError::InvalidPolicy.into())
			);
		}
		let call = sign_call(registration_call(1, enrollment(1, 3)), &[1, 3]);
		assert_eq!(
			Executive::apply_extrinsic(signed(1, call)).unwrap(),
			Err(SecurityError::KeysNotIndependent.into())
		);
		assert_eq!(NextNonce::<Test>::get(account(1)), 0);
		assert_eq!(Balances::reserved_balance(account(1)), 0);

		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), account(1), 1));
		let call = sign_call(registration_call(1, enrollment(2, 3)), &[2, 3]);
		assert!(Executive::apply_extrinsic(signed(1, call)).unwrap().is_err());
		assert!(!Registrations::<Test>::contains_key(account(1)));
		assert_eq!(NextNonce::<Test>::get(account(1)), 0);
		for signer in [2, 3] {
			assert_eq!(System::account(account(signer)).sufficients, 0);
			assert_eq!(System::account_nonce(account(signer)), 0);
		}
	});
}

#[test]
fn canonical_payload_preserves_native_short_and_long_signing_rules() {
	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let short =
			execution_call(1, RuntimeCall::System(frame_system::Call::remark { remark: vec![] }));
		let long = execution_call(
			1,
			RuntimeCall::System(frame_system::Call::remark { remark: vec![7; 512] }),
		);
		let short_payload = AccountSecurity::approval_payload(&short, false).unwrap();
		assert!(short_payload.len() > 32 && short_payload.len() <= 256);
		assert_eq!(AccountSecurity::approval_payload(&long, false).unwrap().len(), 32);
		for operation in [vec![], vec![7; 512]] {
			let call = execution_call(
				1,
				RuntimeCall::System(frame_system::Call::remark { remark: operation }),
			);
			let payload = AccountSecurity::approval_payload(&call, false).unwrap();
			let approved = sign_call(call, &[2]);
			assert_eq!(AccountSecurity::approval_payload(&approved, false).unwrap(), payload);
			assert_ok!(Executive::apply_extrinsic(signed(1, approved)).unwrap());
		}
	});
}

#[test]
fn metadata_bound_proofs_use_the_runtime_hash_not_an_app_supplied_substitute() {
	use frame_metadata_hash_extension::CheckMetadataHash;
	use sp_core::{sr25519, Pair};

	externalities().execute_with(|| {
		enroll(1, 2, 3);
		let mut call = execution_call(1, transfer());
		let payload = AccountSecurity::approval_payload(&call, true).unwrap();
		let pair = sr25519::Pair::from_seed(&[2; 32]);
		let proof = Approval {
			signer: account(2),
			signature: pair.sign(&payload).into(),
			with_metadata_hash: true,
		};
		let RuntimeCall::AccountSecurity(Call::execute { approvals, .. }) = &mut call else {
			unreachable!()
		};
		approvals.try_push(proof).unwrap();

		let mut wrong_mode = call.clone();
		let RuntimeCall::AccountSecurity(Call::execute { approvals, .. }) = &mut wrong_mode else {
			unreachable!()
		};
		approvals[0].with_metadata_hash = false;
		assert_eq!(
			Executive::apply_extrinsic(signed(1, wrong_mode)).unwrap(),
			Err(SecurityError::InvalidSignature.into())
		);

		let canonical = execution_call(1, transfer());
		let extra = (
			frame_system::CheckNonce::<Test>::from(0),
			CheckAccountSecurity::<Test>::new(),
			CheckMetadataHash::<Test>::new_with_custom_hash([0x99; 32]),
		);
		let wrong_payload = sp_runtime::generic::SignedPayload::new(canonical, extra).unwrap();
		let mut wrong_hash = call.clone();
		let RuntimeCall::AccountSecurity(Call::execute { approvals, .. }) = &mut wrong_hash else {
			unreachable!()
		};
		approvals[0].signature = wrong_payload.using_encoded(|bytes| pair.sign(bytes)).into();
		assert_eq!(
			Executive::apply_extrinsic(signed(1, wrong_hash)).unwrap(),
			Err(SecurityError::InvalidSignature.into())
		);
		assert_eq!(NextNonce::<Test>::get(account(1)), 1);
		assert_ok!(Executive::apply_extrinsic(signed(1, call)).unwrap());
	});
}

#[test]
fn a_p256_approval_accepts_and_executes_without_a_funded_signer() {
	use argon_primitives::account_security::{ApprovalSignature, P256Public, P256Signature};
	use p256::ecdsa::{signature::Signer, SigningKey};

	externalities().execute_with(|| {
		let key = SigningKey::from_bytes((&[27; 32]).into()).unwrap();
		let public =
			P256Public::from_sec1_bytes(key.verifying_key().to_encoded_point(true).as_bytes())
				.unwrap();
		let signer = public.signer_id();
		let configuration = Policy {
			signers: vec![signer.clone()].try_into().unwrap(),
			quorum: 1,
			recovery_account: account(3),
			recovery_delay: 0,
		};

		let sign_device = |mut call: RuntimeCall| {
			let payload = AccountSecurity::approval_payload(&call, false).unwrap();
			let signature: p256::ecdsa::Signature = key.sign(&payload);
			let proof = Approval {
				signer: signer.clone(),
				with_metadata_hash: false,
				signature: ApprovalSignature::P256(
					P256Signature::from_device_signature(
						&payload,
						&signature.to_bytes().into(),
						&public,
					)
					.unwrap(),
				),
			};
			let RuntimeCall::AccountSecurity(security) = &mut call else { unreachable!() };
			match security {
				Call::register { approvals, .. } | Call::execute { approvals, .. } =>
					approvals.try_push(proof).unwrap(),
				_ => unreachable!(),
			}
			call
		};
		let registration = sign_call(registration_call(1, configuration), &[3]);
		let registration = sign_device(registration);
		assert_ok!(Executive::apply_extrinsic(signed(1, registration)).unwrap());
		let execution = sign_device(execution_call(1, transfer()));
		assert_ok!(Executive::apply_extrinsic(signed(1, execution)).unwrap());
		assert_eq!(Balances::free_balance(&signer), 0);
		assert_eq!(System::account(&signer).sufficients, 1);
		assert_eq!(System::account_nonce(&signer), 1);
	});
}

#[test]
fn unprotected_and_unmatched_calls_keep_native_permissions() {
	externalities().execute_with(|| {
		assert_ok!(Executive::apply_extrinsic(signed(1, transfer())).unwrap());
		enroll(1, 2, 3);
		let remark = RuntimeCall::System(frame_system::Call::remark { remark: vec![1] });
		assert_ok!(Executive::apply_extrinsic(signed(1, remark)).unwrap());
		let privileged = RuntimeCall::Balances(pallet_balances::Call::force_set_balance {
			who: account(4),
			new_free: 0,
		});
		assert_eq!(
			Executive::apply_extrinsic(signed(1, privileged)).unwrap(),
			Err(DispatchError::BadOrigin)
		);
	});
}

#[test]
fn native_multisig_cannot_hide_execution_after_an_earlier_vote_in_the_same_batch() {
	externalities().execute_with(|| {
		let mut signatories = vec![account(4), account(5), account(6)];
		signatories.sort();
		let multisig = Multisig::multi_account_id(&signatories, 3);
		enroll(1, 2, 3);
		register_proxy(account(1), multisig.clone(), ProxyType::Any, 0);
		register_proxy(account(6), account(4), ProxyType::Any, 0);
		let exit = RuntimeCall::Proxy(pallet_proxy::Call::proxy {
			real: account(1),
			force_proxy_type: None,
			call: Box::new(transfer()),
		});
		let hash = exit.using_encoded(blake2_256);
		let others = |seeds: &[u8]| {
			let mut ids: Vec<_> = seeds.iter().map(|seed| account(*seed)).collect();
			ids.sort();
			ids
		};
		let initial = RuntimeCall::Multisig(pallet_multisig::Call::as_multi {
			threshold: 3,
			other_signatories: others(&[4, 6]),
			maybe_timepoint: None,
			call: Box::new(exit.clone()),
			max_weight: Weight::from_parts(500_000_000, 20_000),
		});
		assert_ok!(Executive::apply_extrinsic(signed(5, initial)).unwrap());
		let pending = pallet_multisig::Multisigs::<Test>::get(&multisig, hash).unwrap();
		let vote = RuntimeCall::Multisig(pallet_multisig::Call::approve_as_multi {
			threshold: 3,
			other_signatories: others(&[4, 5]),
			maybe_timepoint: Some(pending.when),
			call_hash: hash,
			max_weight: Weight::from_parts(500_000_000, 20_000),
		});
		let batch = RuntimeCall::Utility(pallet_utility::Call::batch_all {
			calls: vec![
				RuntimeCall::Proxy(pallet_proxy::Call::proxy {
					real: account(6),
					force_proxy_type: None,
					call: Box::new(vote),
				}),
				RuntimeCall::Multisig(pallet_multisig::Call::as_multi {
					threshold: 3,
					other_signatories: others(&[5, 6]),
					maybe_timepoint: Some(pending.when),
					call: Box::new(exit),
					max_weight: Weight::from_parts(500_000_000, 20_000),
				}),
			],
		});
		assert_eq!(
			Executive::apply_extrinsic(signed(4, batch)),
			Err(SecurityError::UnsupportedComposition.into())
		);
		assert_eq!(
			pallet_multisig::Multisigs::<Test>::get(&multisig, hash).unwrap().approvals,
			pending.approvals
		);
	});
}

#[test]
fn oversized_native_multisigs_are_rejected_before_inspection() {
	externalities().execute_with(|| {
		let max_signatories = <<Test as MultisigConfig>::MaxSignatories as Get<u32>>::get();
		let signatories = vec![account(2); max_signatories as usize];
		let child = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
		let calls = [
			MultisigCall::as_multi {
				threshold: 2,
				other_signatories: signatories.clone(),
				maybe_timepoint: None,
				call: Box::new(child.clone()),
				max_weight: Weight::zero(),
			},
			MultisigCall::as_multi_threshold_1 {
				other_signatories: signatories.clone(),
				call: Box::new(child),
			},
			MultisigCall::approve_as_multi {
				threshold: 2,
				other_signatories: signatories,
				maybe_timepoint: None,
				call_hash: [0; 32],
				max_weight: Weight::zero(),
			},
		];

		for call in calls {
			assert_eq!(
				AccountSecurity::ensure_authorized(&account(1), &RuntimeCall::Multisig(call)),
				Err(SecurityError::CallLimitExceeded),
			);
		}
	});
}
