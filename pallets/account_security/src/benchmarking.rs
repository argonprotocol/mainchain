use super::*;
use alloc::format;
use argon_primitives::account_security::{ApprovalSignature, P256Public, P256Signature};
use core::{hint::black_box, iter::once};
use frame_benchmarking::{account as benchmark_account, v2::*};
use frame_support::{
	instances::Instance1,
	traits::{Currency, Get},
};
use frame_system::RawOrigin;
use p256::ecdsa::{signature::Signer, SigningKey};
use pallet_balances::Call as BalancesCall;
use pallet_multisig::{
	BalanceOf as MultisigBalance, Multisig, Multisigs, Pallet as NativeMultisig,
};
use sp_core::{
	crypto::KeyTypeId,
	ecdsa,
	hashing::{blake2_256, keccak_256},
	sr25519,
};
use sp_runtime::{
	traits::{Hash, Zero},
	AccountId32, MultiSigner,
};

const BENCH_KEY_TYPE: KeyTypeId = KeyTypeId(*b"asec");
const MAX_APPROVALS: u32 = 50;
const MAX_CALL_BYTES: u32 = 5_242_880;

#[benchmarks(
	where
		T: Config<AccountId = AccountId32, Signature = ApprovalSignature>
			+ pallet_balances::Config<Instance1, Balance = u128>,
		RuntimeCallOf<T>: From<frame_system::Call<T>>
			+ From<BalancesCall<T, Instance1>> + From<UtilityCall<T>> + From<ProxyCall<T>>
			+ From<MultisigCall<T>>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn register() {
		let account = benchmark_account::<AccountId32>("security-main", 0, 0);
		T::CallAuthorizationPolicy::prepare_benchmark_account(&account, &account);
		T::SecurityCurrency::make_free_balance_be(&account, u128::MAX / 2);
		let policy = full_policy::<T>(0);
		let mut call: RuntimeCallOf<T> = Call::<T>::register {
			account: account.clone(),
			signers: policy.signers.clone(),
			quorum: policy.quorum,
			recovery_account: policy.recovery_account.clone(),
			recovery_delay: policy.recovery_delay,
			authorization: authorization::<T>(&account),
			approvals: Default::default(),
		}
		.into();
		let proofs = sign_approvals::<T>(&call, ProofKind::Sr25519, 0, T::MaxApprovals::get());
		set_approvals::<T>(&mut call, proofs);

		#[block]
		{
			call.dispatch(RawOrigin::Signed(account.clone()).into()).unwrap();
		}
		assert_eq!(Registrations::<T>::get(&account).unwrap().policy, policy);
		assert_eq!(NextNonce::<T>::get(account), 1);
	}

	#[benchmark]
	fn update_policy() {
		let account = benchmark_account::<AccountId32>("security-main", 0, 0);
		T::CallAuthorizationPolicy::prepare_benchmark_account(&account, &account);
		T::SecurityCurrency::make_free_balance_be(&account, u128::MAX / 2);
		let previous = full_policy::<T>(0);
		Pallet::<T>::store_policy(&account, previous.clone()).unwrap();
		pending_policy::<T>(&account, full_policy::<T>(2 * (T::MaxSigners::get() + 1)));
		let policy = full_policy::<T>(T::MaxSigners::get() + 1);
		// Omit the backup proof so the current quorum must authorize replacement.
		let mut call: RuntimeCallOf<T> = Call::<T>::update_policy {
			account: account.clone(),
			signers: policy.signers.clone(),
			quorum: policy.quorum,
			recovery_account: policy.recovery_account.clone(),
			recovery_delay: policy.recovery_delay,
			authorization: authorization::<T>(&account),
			approvals: Default::default(),
		}
		.into();
		let proofs = quorum_approvals::<T>(&call);
		set_approvals::<T>(&mut call, proofs);

		#[block]
		{
			call.dispatch(RawOrigin::Signed(account.clone()).into()).unwrap();
		}
		assert_eq!(Registrations::<T>::get(&account).unwrap().policy, policy);
		assert_eq!(NextNonce::<T>::get(account), 1);
	}

	#[benchmark]
	fn execute(b: Linear<0, MAX_CALL_BYTES>) {
		let account = benchmark_account::<AccountId32>("security-main", 0, 0);
		T::CallAuthorizationPolicy::prepare_benchmark_account(&account, &account);
		T::SecurityCurrency::make_free_balance_be(&account, u128::MAX / 2);
		Pallet::<T>::store_policy(&account, full_policy::<T>(0)).unwrap();
		// Exercise quorum authorization, then encode a full-size child. Child execution is
		// retained in this overhead measurement rather than subtracting another weight.
		let child: RuntimeCallOf<T> = UtilityCall::<T>::batch_all {
			calls: vec![
				ProxyCall::<T>::remove_proxies {}.into(),
				BalancesCall::<T, Instance1>::burn { value: 0, keep_alive: true }.into(),
				frame_system::Call::<T>::remark { remark: vec![0xff; b as usize] }.into(),
			],
		}
		.into();
		let mut call = execution_call::<T>(&account, child);
		let proofs = sign_approvals::<T>(&call, ProofKind::Sr25519, 0, T::MaxApprovals::get());
		set_approvals::<T>(&mut call, proofs);

		#[block]
		{
			call.dispatch(RawOrigin::Signed(account.clone()).into()).unwrap();
		}
		assert_eq!(NextNonce::<T>::get(account), 1);
	}

	#[benchmark]
	fn schedule_policy_update() {
		let account = benchmark_account::<AccountId32>("security-main", 0, 0);
		T::CallAuthorizationPolicy::prepare_benchmark_account(&account, &account);
		T::SecurityCurrency::make_free_balance_be(&account, u128::MAX / 2);
		let mut previous = full_policy::<T>(0);
		previous.recovery_delay = 10;
		Pallet::<T>::store_policy(&account, previous.clone()).unwrap();
		let mut policy = full_policy::<T>(T::MaxSigners::get() + 1);
		policy.recovery_delay = 20;
		let mut call: RuntimeCallOf<T> = Call::<T>::update_policy {
			account: account.clone(),
			signers: policy.signers.clone(),
			quorum: policy.quorum,
			recovery_account: policy.recovery_account.clone(),
			recovery_delay: policy.recovery_delay,
			authorization: authorization::<T>(&account),
			approvals: Default::default(),
		}
		.into();
		let proofs = quorum_approvals::<T>(&call);
		set_approvals::<T>(&mut call, proofs);

		#[block]
		{
			call.dispatch(RawOrigin::Signed(account.clone()).into()).unwrap();
		}
		assert!(PendingPolicyUpdates::<T>::contains_key(&account));
		assert_eq!(Registrations::<T>::get(&account).unwrap().policy, previous);
		assert_eq!(PendingPolicyUpdates::<T>::get(&account).unwrap().policy, policy);
	}

	#[benchmark]
	fn complete_policy_update() {
		let account = benchmark_account::<AccountId32>("security-main", 0, 0);
		T::CallAuthorizationPolicy::prepare_benchmark_account(&account, &account);
		T::SecurityCurrency::make_free_balance_be(&account, u128::MAX / 2);
		Pallet::<T>::store_policy(&account, full_policy::<T>(0)).unwrap();
		let policy = full_policy::<T>(T::MaxSigners::get() + 1);
		pending_policy::<T>(&account, policy.clone());

		#[extrinsic_call]
		_(RawOrigin::Signed(account.clone()), account.clone(), 0);
		assert_eq!(Registrations::<T>::get(&account).unwrap().policy, policy);
		assert!(!PendingPolicyUpdates::<T>::contains_key(account));
	}

	#[benchmark]
	fn cancel_policy_update() {
		let account = benchmark_account::<AccountId32>("security-main", 0, 0);
		T::CallAuthorizationPolicy::prepare_benchmark_account(&account, &account);
		T::SecurityCurrency::make_free_balance_be(&account, u128::MAX / 2);
		let policy = full_policy::<T>(0);
		Pallet::<T>::store_policy(&account, policy.clone()).unwrap();
		pending_policy::<T>(&account, full_policy::<T>(T::MaxSigners::get() + 1));
		let mut call: RuntimeCallOf<T> = Call::<T>::cancel_policy_update {
			account: account.clone(),
			update_nonce: 0,
			authorization: authorization::<T>(&account),
			approvals: Default::default(),
		}
		.into();
		let proofs = quorum_approvals::<T>(&call);
		set_approvals::<T>(&mut call, proofs);

		#[block]
		{
			call.dispatch(RawOrigin::Signed(account.clone()).into()).unwrap();
		}
		assert_eq!(Registrations::<T>::get(&account).unwrap().policy, policy);
		assert!(!PendingPolicyUpdates::<T>::contains_key(account));
	}

	#[benchmark]
	fn verify_ed25519(a: Linear<1, MAX_APPROVALS>) {
		assert_eq!(T::MaxApprovals::get(), MAX_APPROVALS);
		let (call, approvals) = proof_fixture::<T>(ProofKind::Ed25519, a);

		#[block]
		{
			assert_eq!(Pallet::<T>::verify_approvals(&call, &approvals).unwrap().len(), a as usize);
		}
	}

	#[benchmark]
	fn verify_sr25519(a: Linear<1, MAX_APPROVALS>) {
		let (call, approvals) = proof_fixture::<T>(ProofKind::Sr25519, a);

		#[block]
		{
			assert_eq!(Pallet::<T>::verify_approvals(&call, &approvals).unwrap().len(), a as usize);
		}
	}

	#[benchmark]
	fn verify_ecdsa(a: Linear<1, MAX_APPROVALS>) {
		let (call, approvals) = proof_fixture::<T>(ProofKind::Ecdsa, a);

		#[block]
		{
			assert_eq!(Pallet::<T>::verify_approvals(&call, &approvals).unwrap().len(), a as usize);
		}
	}

	#[benchmark]
	fn verify_eth(a: Linear<1, MAX_APPROVALS>) {
		let (call, approvals) = proof_fixture::<T>(ProofKind::Eth, a);

		#[block]
		{
			assert_eq!(Pallet::<T>::verify_approvals(&call, &approvals).unwrap().len(), a as usize);
		}
	}

	#[benchmark]
	fn verify_p256(a: Linear<1, MAX_APPROVALS>) {
		let (call, approvals) = proof_fixture::<T>(ProofKind::P256, a);

		#[block]
		{
			assert_eq!(Pallet::<T>::verify_approvals(&call, &approvals).unwrap().len(), a as usize);
		}
	}

	#[benchmark]
	fn authorization_leaf() {
		let account = benchmark_account::<AccountId32>("security-main", 0, 0);
		let call = frame_system::Call::<T>::remark { remark: vec![] }.into();

		#[block]
		{
			Pallet::<T>::ensure_authorized(&account, black_box(&call)).unwrap();
		}
	}

	#[benchmark]
	fn authorization_protection() {
		let account = benchmark_account::<AccountId32>("security-member", 0, 0);
		let group = benchmark_account::<AccountId32>("security-group", 0, 0);
		T::CallAuthorizationPolicy::prepare_benchmark_account(&account, &group);
		T::SecurityCurrency::make_free_balance_be(&group, u128::MAX / 2);
		Pallet::<T>::store_policy(&group, full_policy::<T>(0)).unwrap();
		let context = AuthorizationContext {
			account: account.clone(),
			authorized_group: Some(group),
			executes: true,
		};

		#[block]
		{
			assert!(Pallet::<T>::approval_status(&account, &context) == ApprovalStatus::Approved);
		}
	}

	#[benchmark]
	fn authorization_multisig(
		s: Linear<1, { T::MaxSignatories::get() - 1 }>,
		b: Linear<0, MAX_CALL_BYTES>,
	) {
		let account = benchmark_account::<AccountId32>("security-main", 0, 0);
		let others = (0..s)
			.map(|index| benchmark_account::<AccountId32>("security-signatory", index, 0))
			.collect::<Vec<_>>();
		let threshold = (s + 1) as u16;
		let multisig = Pallet::<T>::multisig_account(&account, threshold, &others);
		let group = benchmark_account::<AccountId32>("security-group", 0, 0);
		T::CallAuthorizationPolicy::prepare_benchmark_account(&multisig, &group);
		T::SecurityCurrency::make_free_balance_be(&group, u128::MAX / 2);
		Pallet::<T>::store_policy(&group, full_policy::<T>(0)).unwrap();

		let child: RuntimeCallOf<T> =
			frame_system::Call::<T>::remark_with_event { remark: vec![0xff; b as usize] }.into();
		let call_hash = child.using_encoded(blake2_256);
		let when = NativeMultisig::<T>::timepoint();
		let mut approvals = others.clone();
		approvals.sort();
		let pending = Multisig::<_, MultisigBalance<T>, _, T::MaxSignatories> {
			when,
			deposit: Zero::zero(),
			depositor: approvals[0].clone(),
			approvals: approvals.try_into().unwrap(),
		};
		Multisigs::<T>::insert(&multisig, call_hash, pending);
		let max_weight = child.get_dispatch_info().call_weight;
		let call = MultisigCall::<T>::as_multi {
			threshold,
			other_signatories: others,
			maybe_timepoint: Some(when),
			call: Box::new(child),
			max_weight,
		}
		.into();
		let events_before = System::<T>::events().len();
		let context =
			AuthorizationContext { account, authorized_group: Some(group), executes: true };
		// Measure the wrapper only. Its child reaches the visit limit before doing any work;
		// call_inspection_weight charges each child separately.
		let mut inspection =
			AuthorizationInspection { visited_calls: T::MaxCalls::get() - 1, ..Default::default() };

		#[block]
		{
			assert_eq!(
				Pallet::<T>::inspect(black_box(&call), &context, &mut inspection, 0),
				Err(Error::<T>::CallLimitExceeded)
			);
		}
		assert_eq!(Multisigs::<T>::get(multisig, call_hash).unwrap().approvals.len(), s as usize);
		assert_eq!(System::<T>::events().len(), events_before);
	}

	#[benchmark]
	fn authorization_multisig_derivation(s: Linear<1, { T::MaxSignatories::get() - 1 }>) {
		let account = benchmark_account::<AccountId32>("security-main", 0, 0);
		let others = (0..s)
			.map(|index| benchmark_account::<AccountId32>("security-signatory", index, 0))
			.collect::<Vec<_>>();
		let child = frame_system::Call::<T>::remark_with_event { remark: vec![] }.into();
		let call = MultisigCall::<T>::as_multi_threshold_1 {
			other_signatories: others,
			call: Box::new(child),
		}
		.into();
		let context = AuthorizationContext { account, authorized_group: None, executes: true };
		let mut inspection =
			AuthorizationInspection { visited_calls: T::MaxCalls::get() - 1, ..Default::default() };

		#[block]
		{
			assert_eq!(
				Pallet::<T>::inspect(black_box(&call), &context, &mut inspection, 0),
				Err(Error::<T>::CallLimitExceeded)
			);
		}
		assert!(System::<T>::events().is_empty());
	}

	#[benchmark]
	fn authorization_batch() {
		let account = benchmark_account::<AccountId32>("security-main", 0, 0);
		let calls = vec![frame_system::Call::<T>::remark { remark: vec![] }.into()];
		let call = UtilityCall::<T>::batch_all { calls }.into();
		let context = AuthorizationContext { account, authorized_group: None, executes: true };
		let mut inspection =
			AuthorizationInspection { visited_calls: T::MaxCalls::get() - 1, ..Default::default() };

		#[block]
		{
			assert_eq!(
				Pallet::<T>::inspect(black_box(&call), &context, &mut inspection, 0),
				Err(Error::<T>::CallLimitExceeded)
			);
		}
	}

	#[benchmark]
	fn authorization_derivative() {
		let account = benchmark_account::<AccountId32>("security-main", 0, 0);
		let child = frame_system::Call::<T>::remark { remark: vec![] }.into();
		let call = UtilityCall::<T>::as_derivative { index: 0, call: Box::new(child) }.into();
		let context = AuthorizationContext { account, authorized_group: None, executes: true };
		let mut inspection =
			AuthorizationInspection { visited_calls: T::MaxCalls::get() - 1, ..Default::default() };

		#[block]
		{
			assert_eq!(
				Pallet::<T>::inspect(black_box(&call), &context, &mut inspection, 0),
				Err(Error::<T>::CallLimitExceeded)
			);
		}
	}

	#[benchmark]
	fn on_operational_registration() {
		let default_account = benchmark_account::<AccountId32>("security-default", 0, 0);
		T::CallAuthorizationPolicy::prepare_benchmark_account(&default_account, &default_account);
		let participants = (0..3)
			.map(|index| benchmark_account::<AccountId32>("security-link", index, 0))
			.collect::<Vec<_>>();
		let policy = full_policy::<T>(0);
		for (index, participant) in participants.iter().enumerate() {
			T::CallAuthorizationPolicy::prepare_benchmark_account(participant, participant);
			T::SecurityCurrency::make_free_balance_be(participant, u128::MAX / 2);
			Pallet::<T>::store_policy(participant, policy.clone()).unwrap();
			let pending = full_policy::<T>((index as u32 + 1) * (T::MaxSigners::get() + 1));
			pending_policy::<T>(participant, pending);
		}
		T::SecurityCurrency::make_free_balance_be(&default_account, u128::MAX / 2);

		#[block]
		{
			<Pallet<T> as AccountSecurityProvider<T::AccountId>>::on_operational_registration(
				&default_account,
				&participants,
			)
			.unwrap();
		}
		assert_eq!(Registrations::<T>::get(&default_account).unwrap().policy, policy);
		for participant in participants {
			assert!(!Registrations::<T>::contains_key(&participant));
			assert_eq!(NextNonce::<T>::get(participant), 1);
		}
	}

	#[benchmark]
	fn on_operational_registration_existing() {
		let default_account = benchmark_account::<AccountId32>("security-default", 0, 0);
		let participants = (0..3)
			.map(|index| benchmark_account::<AccountId32>("security-link", index, 0))
			.collect::<Vec<_>>();
		let policy = full_policy::<T>(0);
		for (index, participant) in participants.iter().chain(once(&default_account)).enumerate() {
			T::CallAuthorizationPolicy::prepare_benchmark_account(participant, participant);
			T::SecurityCurrency::make_free_balance_be(participant, u128::MAX / 2);
			Pallet::<T>::store_policy(participant, policy.clone()).unwrap();
			let pending = full_policy::<T>((index as u32 + 1) * (T::MaxSigners::get() + 1));
			pending_policy::<T>(participant, pending);
		}

		#[block]
		{
			<Pallet<T> as AccountSecurityProvider<T::AccountId>>::on_operational_registration(
				&default_account,
				&participants,
			)
			.unwrap();
		}
		assert_eq!(Registrations::<T>::get(&default_account).unwrap().policy, policy);
		for participant in participants {
			assert!(!Registrations::<T>::contains_key(&participant));
			assert_eq!(NextNonce::<T>::get(participant), 1);
		}
	}
}

#[derive(Clone, Copy)]
enum ProofKind {
	Ed25519,
	Sr25519,
	Ecdsa,
	Eth,
	P256,
}

fn authorization<T: Config>(account: &T::AccountId) -> Authorization<T> {
	// FRAME starts measurement at block one even when setup began without genesis state.
	if System::<T>::block_number().is_zero() {
		System::<T>::set_block_number(1u32.into());
	}
	if !frame_system::BlockHash::<T>::contains_key(BlockNumberFor::<T>::zero()) {
		frame_system::BlockHash::<T>::insert(
			BlockNumberFor::<T>::zero(),
			T::Hashing::hash(b"account-security-benchmark-genesis"),
		);
	}

	let group = T::CallAuthorizationPolicy::default_account(account);
	let policy = Registrations::<T>::get(&group).map(|registration| registration.policy);
	Authorization {
		nonce: NextNonce::<T>::get(&group),
		group,
		expires_at: System::<T>::block_number(),
		policy_hash: policy.using_encoded(blake2_256),
	}
}

fn full_policy<T: Config<AccountId = AccountId32>>(offset: u32) -> Policy<T> {
	let count = T::MaxSigners::get();
	let mut accounts = (offset..offset + count + 1)
		.map(|index| {
			let public = native_key(index);
			let account = AccountId32::from(public.0);
			let group = benchmark_account::<AccountId32>("security-signer-group", index, 0);
			T::CallAuthorizationPolicy::prepare_benchmark_account(&account, &group);
			account
		})
		.collect::<Vec<_>>();
	let recovery_account = accounts.pop().unwrap();
	accounts.sort();
	Policy {
		signers: accounts.try_into().unwrap(),
		quorum: count as u16,
		recovery_account,
		recovery_delay: 0,
	}
}

fn native_key(index: u32) -> sr25519::Public {
	sp_io::crypto::sr25519_generate(
		BENCH_KEY_TYPE,
		Some(format!("//account-security//{index}").into_bytes()),
	)
}

fn pending_policy<T: Config>(account: &T::AccountId, policy: Policy<T>) {
	let deposit = Pallet::<T>::storage_deposit(policy.signers.len() + 1);
	T::SecurityCurrency::reserve(account, deposit).unwrap();
	Pallet::<T>::retain_policy_accounts(&policy);
	let previous_policy_hash =
		Some(Registrations::<T>::get(account).unwrap().policy).using_encoded(blake2_256);
	PendingPolicyUpdates::<T>::insert(
		account,
		PendingPolicyUpdate {
			policy,
			nonce: 0,
			previous_policy_hash,
			execute_after_tick: T::CurrentTick::get(),
			deposit,
		},
	);
}

fn execution_call<T: Config>(account: &T::AccountId, call: RuntimeCallOf<T>) -> RuntimeCallOf<T> {
	Call::<T>::execute {
		account: account.clone(),
		call: Box::new(call),
		authorization: authorization::<T>(account),
		approvals: Default::default(),
	}
	.into()
}

fn set_approvals<T: Config>(
	call: &mut RuntimeCallOf<T>,
	approvals: BoundedVec<Approval<T>, T::MaxApprovals>,
) {
	let mut security = <RuntimeCallOf<T> as IsSubType<Call<T>>>::is_sub_type(call).unwrap().clone();
	match &mut security {
		Call::register { approvals: stored, .. } |
		Call::update_policy { approvals: stored, .. } |
		Call::cancel_policy_update { approvals: stored, .. } |
		Call::execute { approvals: stored, .. } => *stored = approvals,
		_ => unreachable!(),
	}
	*call = security.into();
}

fn proof_fixture<T>(
	kind: ProofKind,
	count: u32,
) -> (RuntimeCallOf<T>, BoundedVec<Approval<T>, T::MaxApprovals>)
where
	T: Config<AccountId = AccountId32, Signature = ApprovalSignature>,
	RuntimeCallOf<T>: From<frame_system::Call<T>>,
{
	let account = benchmark_account::<AccountId32>("security-main", 0, 0);
	let child = frame_system::Call::<T>::remark { remark: vec![0xff; 256] }.into();
	let call = execution_call::<T>(&account, child);
	let approvals = sign_approvals::<T>(&call, kind, 0, count);
	(call, approvals)
}

/// Fill the proof bound without the old backup, so quorum scanning cannot short-circuit.
fn quorum_approvals<T>(call: &RuntimeCallOf<T>) -> BoundedVec<Approval<T>, T::MaxApprovals>
where
	T: Config<AccountId = AccountId32, Signature = ApprovalSignature>,
{
	let count = T::MaxSigners::get();
	let mut proofs = sign_approvals::<T>(call, ProofKind::Sr25519, 0, count);
	for proof in
		sign_approvals::<T>(call, ProofKind::Sr25519, count + 1, T::MaxApprovals::get() - count)
	{
		proofs.try_push(proof).unwrap();
	}
	proofs
}

fn sign_approvals<T>(
	call: &RuntimeCallOf<T>,
	kind: ProofKind,
	offset: u32,
	count: u32,
) -> BoundedVec<Approval<T>, T::MaxApprovals>
where
	T: Config<AccountId = AccountId32, Signature = ApprovalSignature>,
{
	let plain_payload = Pallet::<T>::approval_payload(call, false).unwrap();
	let metadata_payload = Pallet::<T>::approval_payload(call, true)
		.expect("AccountSecurity benchmarks require the metadata-hash runtime feature");
	(offset..offset + count)
		.map(|index| {
			let with_metadata_hash = index % 2 == 1;
			let payload = if with_metadata_hash { &metadata_payload } else { &plain_payload };
			let seed = Some(format!("//account-security//{index}").into_bytes());
			let (signer, signature) = match kind {
				ProofKind::Ed25519 => {
					let public = sp_io::crypto::ed25519_generate(BENCH_KEY_TYPE, seed);
					let signature =
						sp_io::crypto::ed25519_sign(BENCH_KEY_TYPE, &public, payload).unwrap();
					(
						MultiSigner::Ed25519(public).into_account(),
						ApprovalSignature::Ed25519(signature),
					)
				},
				ProofKind::Sr25519 => {
					let public = native_key(index);
					let signature =
						sp_io::crypto::sr25519_sign(BENCH_KEY_TYPE, &public, payload).unwrap();
					(
						MultiSigner::Sr25519(public).into_account(),
						ApprovalSignature::Sr25519(signature),
					)
				},
				ProofKind::Ecdsa => {
					let public = sp_io::crypto::ecdsa_generate(BENCH_KEY_TYPE, seed);
					let signature =
						sp_io::crypto::ecdsa_sign(BENCH_KEY_TYPE, &public, payload).unwrap();
					(MultiSigner::Ecdsa(public).into_account(), ApprovalSignature::Ecdsa(signature))
				},
				ProofKind::Eth => {
					let public = sp_io::crypto::ecdsa_generate(BENCH_KEY_TYPE, seed);
					let signature = sp_io::crypto::ecdsa_sign_prehashed(
						BENCH_KEY_TYPE,
						&public,
						&keccak_256(payload),
					)
					.unwrap();
					(
						MultiSigner::Eth(ecdsa::KeccakPublic::from_raw(public.0)).into_account(),
						ApprovalSignature::Eth(ecdsa::KeccakSignature::from_raw(signature.0)),
					)
				},
				ProofKind::P256 => p256_proof(payload, index),
			};
			Approval { signer, signature, with_metadata_hash }
		})
		.collect::<Vec<_>>()
		.try_into()
		.unwrap()
}

fn p256_proof(payload: &[u8], index: u32) -> (AccountId32, ApprovalSignature) {
	let seed = (b"account-security-p256", index).using_encoded(blake2_256);
	let key = SigningKey::from_bytes((&seed).into()).unwrap();
	let public =
		P256Public::from_sec1_bytes(key.verifying_key().to_encoded_point(true).as_bytes()).unwrap();
	let device_signature: p256::ecdsa::Signature = key.sign(payload);
	let signature =
		P256Signature::from_device_signature(payload, &device_signature.to_bytes().into(), &public)
			.unwrap();
	(public.signer_id(), ApprovalSignature::P256(signature))
}
