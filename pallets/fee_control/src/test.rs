// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use crate::mock::{
	new_test_ext,
	pallet_dummy::{Call, ConsumedPoolKeys, DispatchedPoolKeys, OneUseCodes},
	Balances, FeeAmount, FeeControl, FeeUnbalancedAmount, LastPayer, LastPostDispatchPaysFee,
	MockChargePaymentExtension, PrepareCount, Proxy, ProxyType, RuntimeCall, RuntimeEvent, System,
	Test, TipAmount, TipUnbalancedAmount, ValidateCount,
};
use codec::Encode;
use frame_support::dispatch::{DispatchInfo, GetDispatchInfo, Pays, PostDispatchInfo};
use frame_system::RawOrigin;
use pallet_prelude::{
	argon_primitives::CurrentTransactionFeeProvider, frame_support::traits::Currency,
};
use pallet_transaction_payment::ChargeTransactionPayment;
use sp_runtime::{
	traits::{DispatchTransaction, Hash, TransactionExtension},
	transaction_validity::{InvalidTransaction, TransactionSource, TransactionValidityError},
};

#[test]
fn skip_feeless_payment_works() {
	new_test_ext().execute_with(|| {
		PrepareCount::set(0);
		ValidateCount::set(0);
		set_argons(0, 1_000_000u128);
		let call = RuntimeCall::DummyPallet(Call::<Test>::aux { is_feeless: false, key: 1 });
		CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
			.validate_and_prepare(Some(0).into(), &call, &DispatchInfo::default(), 0, 0)
			.unwrap();
		assert_eq!(PrepareCount::get(), 1);

		let call = RuntimeCall::DummyPallet(Call::<Test>::aux { is_feeless: true, key: 1 });
		CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
			.validate_and_prepare(Some(0).into(), &call, &DispatchInfo::default(), 0, 0)
			.unwrap();
		assert_eq!(PrepareCount::get(), 1);
	});
}

#[test]
fn feeless_calls_refund_the_skipped_payment_weight() {
	new_test_ext().execute_with(|| {
		let call = RuntimeCall::DummyPallet(Call::<Test>::aux { is_feeless: true, key: 1 });
		let info = call.get_dispatch_info();
		let len = call.encoded_size();
		let inner_weight = MockChargePaymentExtension.weight(&call);
		let (pre, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_and_prepare(Some(0).into(), &call, &info, len, 0)
				.unwrap();

		let refunded = CheckFeeWrapper::<Test, MockChargePaymentExtension>::post_dispatch_details(
			pre,
			&info,
			&PostDispatchInfo::default(),
			len,
			&Ok(()),
		)
		.unwrap();

		assert_eq!(refunded, inner_weight);
	});
}

#[test]
fn validate_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(ValidateCount::get(), 0);
		set_argons(0, 1_000_000u128);

		let call = RuntimeCall::DummyPallet(Call::<Test>::aux { is_feeless: false, key: 2 });
		let (res1, _, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(0).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();
		assert_eq!(ValidateCount::get(), 1);
		assert_eq!(PrepareCount::get(), 0);
		assert!(res1.provides.is_empty());

		let call = RuntimeCall::DummyPallet(Call::<Test>::aux { is_feeless: true, key: 1 });
		let (res2, _, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(0).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();
		assert_eq!(ValidateCount::get(), 1);
		assert_eq!(PrepareCount::get(), 0);
		assert_eq!(res2.provides[0], 1.encode());

		let call = RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 7 });
		let (res3, _, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(0).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();
		assert_eq!(ValidateCount::get(), 2);
		assert_eq!(PrepareCount::get(), 0);
		assert_eq!(res3.provides[0], (b"general", 7u32).encode());
	});
}

#[test]
fn feeless_calls_get_a_non_zero_priority_floor() {
	new_test_ext().execute_with(|| {
		set_argons(0, 1_000_000_000_000u128);

		let feeless_call = RuntimeCall::DummyPallet(Call::<Test>::stacked { key: 7 });

		let (feeless, _, _) = CheckFeeWrapper::<Test, ChargeTransactionPayment<Test>>::from(
			ChargeTransactionPayment::from(0u128),
		)
		.validate_only(
			Some(0).into(),
			&feeless_call,
			&feeless_call.get_dispatch_info(),
			feeless_call.encoded_size(),
			TransactionSource::External,
			0,
		)
		.unwrap();
		let expected_priority = ChargeTransactionPayment::<Test>::get_priority(
			&feeless_call.get_dispatch_info(),
			feeless_call.encoded_size(),
			0u128,
			pallet_transaction_payment::Pallet::<Test>::compute_fee(
				feeless_call.encoded_size() as u32,
				&feeless_call.get_dispatch_info(),
				0u128,
			),
		);

		assert!(feeless.priority > 0);
		assert_eq!(feeless.priority, expected_priority);
	});
}

#[test]
fn operational_feeless_calls_keep_operational_priority_bump() {
	new_test_ext().execute_with(|| {
		set_argons(0, 1_000_000_000_000u128);

		let call = RuntimeCall::DummyPallet(Call::<Test>::stacked_operational { key: 7 });
		let normal_call = RuntimeCall::DummyPallet(Call::<Test>::stacked { key: 7 });

		let (feeless, _, _) = CheckFeeWrapper::<Test, ChargeTransactionPayment<Test>>::from(
			ChargeTransactionPayment::from(0u128),
		)
		.validate_only(
			Some(0).into(),
			&call,
			&call.get_dispatch_info(),
			call.encoded_size(),
			TransactionSource::External,
			0,
		)
		.unwrap();
		let expected_priority = ChargeTransactionPayment::<Test>::get_priority(
			&call.get_dispatch_info(),
			call.encoded_size(),
			0u128,
			pallet_transaction_payment::Pallet::<Test>::compute_fee(
				call.encoded_size() as u32,
				&call.get_dispatch_info(),
				0u128,
			),
		);
		let normal_priority = ChargeTransactionPayment::<Test>::get_priority(
			&normal_call.get_dispatch_info(),
			normal_call.encoded_size(),
			0u128,
			pallet_transaction_payment::Pallet::<Test>::compute_fee(
				normal_call.encoded_size() as u32,
				&normal_call.get_dispatch_info(),
				0u128,
			),
		);

		assert_eq!(feeless.priority, expected_priority);
		assert!(feeless.priority > normal_priority);
	});
}

#[test]
fn reimbursable_fee_reads_transaction_payment_credit() {
	new_test_ext().execute_with(|| {
		assert_eq!(<FeeControl as CurrentTransactionFeeProvider<u128>>::reimbursable_fee(), None);

		let inclusion_fee =
			<Balances as frame_support::traits::fungible::Balanced<u64>>::issue(FeeAmount::get());
		pallet_transaction_payment::Pallet::<Test>::deposit_txfee::<u128>(inclusion_fee);

		assert_eq!(
			<FeeControl as CurrentTransactionFeeProvider<u128>>::reimbursable_fee(),
			Some(FeeAmount::get()),
		);
	});
}

#[test]
fn validate_keeps_feeless_and_general_pool_keys() {
	new_test_ext().execute_with(|| {
		let call = RuntimeCall::DummyPallet(Call::<Test>::stacked { key: 9 });
		let (res, _, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(0).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();

		assert_eq!(res.provides.len(), 2);
		assert!(res.provides.contains(&(b"general", 9u32).encode()));
		assert!(res.provides.contains(&(b"feeless", 9u32).encode()));
	});
}

#[test]
fn validate_feeless_keys_without_a_signer() {
	new_test_ext().execute_with(|| {
		let call = RuntimeCall::DummyPallet(Call::<Test>::stacked { key: 12 });
		let unsigned =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					None.into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				);
		assert!(matches!(
			unsigned,
			Err(TransactionValidityError::Invalid(InvalidTransaction::UnknownOrigin))
		));

		let (root, _, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					RawOrigin::Root.into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();

		assert_eq!(root.provides.len(), 2);
		assert!(root.provides.contains(&(b"general", 12u32).encode()));
		assert!(root.provides.contains(&(b"feeless", 12u32).encode()));
	});
}

#[test]
fn validate_keeps_sponsor_and_general_pool_keys() {
	new_test_ext().execute_with(|| {
		let sponsor_id = 7u64;
		let signer_id = 1u64;
		let key = 11u32;
		set_argons(sponsor_id, 1_000_000u128);
		OneUseCodes::<Test>::insert(key, (sponsor_id, 5000));

		let call = RuntimeCall::DummyPallet(Call::<Test>::sponsored_pooled { key });
		let (res, _, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(signer_id).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();

		assert_eq!(res.provides.len(), 2);
		assert!(res.provides.contains(&key.encode()));
		assert!(res.provides.contains(&(b"general", key).encode()));
	});
}

#[test]
fn batches_do_not_claim_standalone_pool_keys() {
	new_test_ext().execute_with(|| {
		set_argons(0, 1_000_000u128);
		set_argons(7, 1_000_000u128);
		assert_ok!(Proxy::add_proxy(Some(7).into(), 0, ProxyType::Any, 0));
		let calls = vec![
			RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 7 }),
			RuntimeCall::DummyPallet(Call::<Test>::stacked { key: 9 }),
		];
		for utility_call in [
			pallet_utility::Call::<Test>::batch { calls: calls.clone() },
			pallet_utility::Call::<Test>::batch_all { calls: calls.clone() },
			pallet_utility::Call::<Test>::force_batch { calls },
		] {
			let batch = RuntimeCall::Utility(utility_call);
			let proxy_batch = RuntimeCall::Proxy(pallet_proxy::Call::<Test>::proxy {
				real: 7,
				force_proxy_type: None,
				call: Box::new(batch.clone()),
			});
			let nested_batch = RuntimeCall::Utility(pallet_utility::Call::<Test>::force_batch {
				calls: vec![proxy_batch.clone()],
			});
			for call in [batch, proxy_batch, nested_batch] {
				let (validity, _, _) = CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(
					MockChargePaymentExtension,
				)
				.validate_only(
					Some(0).into(),
					&call,
					&call.get_dispatch_info(),
					call.encoded_size(),
					TransactionSource::External,
					0,
				)
				.unwrap();
				assert!(validity.provides.is_empty(), "{call:?}");
			}
		}
	});
}

#[test]
fn rollback_batches_do_not_claim_standalone_keys_or_skip_fees() {
	let honest_call = RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 7 });
	let failing_call = RuntimeCall::DummyPallet(Call::<Test>::pooled_fail { key: 8 });
	let atomic_batch = RuntimeCall::Utility(pallet_utility::Call::<Test>::batch_all {
		calls: vec![honest_call.clone(), failing_call.clone()],
	});
	let interrupted_batch = RuntimeCall::Utility(pallet_utility::Call::<Test>::batch {
		calls: vec![failing_call, honest_call.clone()],
	});
	let nested_batch = RuntimeCall::Utility(pallet_utility::Call::<Test>::force_batch {
		calls: vec![atomic_batch.clone()],
	});

	for (attack, dispatch_fails) in
		[(interrupted_batch, false), (atomic_batch, true), (nested_batch, false)]
	{
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			FeeUnbalancedAmount::set(0);
			TipUnbalancedAmount::set(0);
			set_argons(0, 1_000_000_000_000);
			set_argons(1, 1_000_000_000_000);
			let honest_extension = (
				frame_system::CheckNonce::<Test>::from(0),
				CheckFeeWrapper::<Test, ChargeTransactionPayment<Test>>::from(
					ChargeTransactionPayment::<Test>::from(0),
				),
			);
			let (honest_validity, _, _) = honest_extension
				.validate_only(
					Some(0).into(),
					&honest_call,
					&honest_call.get_dispatch_info(),
					honest_call.encoded_size(),
					TransactionSource::External,
					0,
				)
				.unwrap();
			assert_eq!(honest_validity.provides.len(), 2); // Account nonce and standalone key.

			let attack_extension = (
				frame_system::CheckNonce::<Test>::from(0),
				CheckFeeWrapper::<Test, ChargeTransactionPayment<Test>>::from(
					ChargeTransactionPayment::<Test>::from(10_000),
				),
			);
			let (attack_validity, _, _) = attack_extension
				.validate_only(
					Some(1).into(),
					&attack,
					&attack.get_dispatch_info(),
					attack.encoded_size(),
					TransactionSource::External,
					0,
				)
				.unwrap();
			assert!(attack_validity.priority > honest_validity.priority);
			assert!(attack_validity
				.provides
				.iter()
				.all(|key| !honest_validity.provides.contains(key)));

			// Even if the tipped wrapper executes first, it cannot commit the keyed call.
			let balance_before = Balances::free_balance(1);
			let attack_result = attack_extension
				.test_run(
					Some(1).into(),
					&attack,
					&attack.get_dispatch_info(),
					attack.encoded_size(),
					0,
					|origin| attack.clone().dispatch(origin),
				)
				.unwrap();
			assert_eq!(attack_result.is_err(), dispatch_fails);
			assert!(!DispatchedPoolKeys::<Test>::contains_key(7));
			assert!(Balances::free_balance(1) < balance_before - 10_000);
			assert_eq!(TipUnbalancedAmount::get(), 10_000);

			let honest_result = honest_extension
				.test_run(
					Some(0).into(),
					&honest_call,
					&honest_call.get_dispatch_info(),
					honest_call.encoded_size(),
					0,
					|origin| honest_call.clone().dispatch(origin),
				)
				.unwrap();
			assert_ok!(honest_result);
			assert!(DispatchedPoolKeys::<Test>::contains_key(7));
		});
	}
}

#[test]
fn proxy_batches_keep_only_the_sponsor_specific_key_and_still_execute() {
	new_test_ext().execute_with(|| {
		FeeUnbalancedAmount::set(0);
		TipUnbalancedAmount::set(0);
		set_argons(0, 1_000_000_000_000);
		set_argons(7, 1_000_000_000_000);
		assert_ok!(Proxy::add_proxy(Some(7).into(), 0, ProxyType::DummyWrapper, 0));
		let calls = vec![
			RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 7 }),
			RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 8 }),
		];
		let call = RuntimeCall::Proxy(pallet_proxy::Call::<Test>::proxy {
			real: 7,
			force_proxy_type: Some(ProxyType::DummyWrapper),
			call: Box::new(RuntimeCall::Utility(pallet_utility::Call::<Test>::batch_all {
				calls: calls.clone(),
			})),
		});
		let wrapper = CheckFeeWrapper::<Test, ChargeTransactionPayment<Test>>::from(
			ChargeTransactionPayment::<Test>::from(500),
		);
		let (validity, _, _) = wrapper
			.validate_only(
				Some(0).into(),
				&call,
				&call.get_dispatch_info(),
				call.encoded_size(),
				TransactionSource::External,
				0,
			)
			.unwrap();
		assert_eq!(validity.provides, vec![(b"proxy-batch", calls).encode()]);

		let delegate_before = Balances::free_balance(0);
		let sponsor_before = Balances::free_balance(7);
		let result = wrapper
			.test_run(
				Some(0).into(),
				&call,
				&call.get_dispatch_info(),
				call.encoded_size(),
				0,
				|origin| call.clone().dispatch(origin),
			)
			.unwrap();
		assert_ok!(result);
		assert!(DispatchedPoolKeys::<Test>::contains_key(7));
		assert!(DispatchedPoolKeys::<Test>::contains_key(8));
		assert_eq!(Balances::free_balance(0), delegate_before);
		assert!(Balances::free_balance(7) < sponsor_before);
		assert_eq!(TipUnbalancedAmount::get(), 500);
	});
}

#[test]
fn validate_rejects_stale_batched_calls() {
	new_test_ext().execute_with(|| {
		set_argons(0, 1_000_000u128);
		ConsumedPoolKeys::<Test>::insert(7, ());
		let calls = vec![
			RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 7 }),
			RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 8 }),
		];
		for utility_call in [
			pallet_utility::Call::<Test>::batch { calls: calls.clone() },
			pallet_utility::Call::<Test>::batch_all { calls: calls.clone() },
			pallet_utility::Call::<Test>::force_batch { calls },
		] {
			let call = RuntimeCall::Utility(utility_call);
			let result = CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(
				MockChargePaymentExtension,
			)
			.validate_only(
				Some(0).into(),
				&call,
				&call.get_dispatch_info(),
				call.encoded_size(),
				TransactionSource::External,
				0,
			);
			assert!(matches!(
				result,
				Err(TransactionValidityError::Invalid(InvalidTransaction::Stale))
			));
		}
	});
}

#[test]
fn post_dispatch_refunds_configured_batch_all_combinations_on_success() {
	new_test_ext().execute_with(|| {
		set_argons(0, 1_000_000u128);
		LastPostDispatchPaysFee::set(None);

		let call = RuntimeCall::Utility(pallet_utility::Call::<Test>::batch_all {
			calls: vec![
				RuntimeCall::DummyPallet(Call::<Test>::stacked { key: 7 }),
				RuntimeCall::DummyPallet(Call::<Test>::combo_paid { should_fail: false }),
			],
		});
		let info = call.get_dispatch_info();
		let len = call.encoded_size();
		let wrapper =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension);
		let (_, val, validated_origin) = wrapper
			.validate_only(Some(0).into(), &call, &info, len, TransactionSource::External, 0)
			.unwrap();
		let pre =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.prepare(val, &validated_origin, &call, &info, len)
				.unwrap();

		let dispatch_result = call.dispatch(validated_origin);
		let dispatch_outcome = match &dispatch_result {
			Ok(_) => Ok(()),
			Err(err) => Err(err.error),
		};
		let post_info = match &dispatch_result {
			Ok(post_info) => post_info,
			Err(err) => &err.post_info,
		};

		CheckFeeWrapper::<Test, MockChargePaymentExtension>::post_dispatch_details(
			pre,
			&info,
			post_info,
			len,
			&dispatch_outcome,
		)
		.unwrap();

		assert_eq!(LastPostDispatchPaysFee::get(), Some(Pays::No));
	});
}

#[test]
fn post_dispatch_keeps_fees_for_failed_configured_batch_all_combinations() {
	new_test_ext().execute_with(|| {
		set_argons(0, 1_000_000u128);
		LastPostDispatchPaysFee::set(None);

		let call = RuntimeCall::Utility(pallet_utility::Call::<Test>::batch_all {
			calls: vec![
				RuntimeCall::DummyPallet(Call::<Test>::stacked { key: 7 }),
				RuntimeCall::DummyPallet(Call::<Test>::combo_paid { should_fail: true }),
			],
		});
		let info = call.get_dispatch_info();
		let len = call.encoded_size();
		let wrapper =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension);
		let (_, val, validated_origin) = wrapper
			.validate_only(Some(0).into(), &call, &info, len, TransactionSource::External, 0)
			.unwrap();
		let pre =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.prepare(val, &validated_origin, &call, &info, len)
				.unwrap();

		let dispatch_result = call.dispatch(validated_origin);
		let dispatch_outcome = match &dispatch_result {
			Ok(_) => Ok(()),
			Err(err) => Err(err.error),
		};
		let post_info = match &dispatch_result {
			Ok(post_info) => post_info,
			Err(err) => &err.post_info,
		};

		CheckFeeWrapper::<Test, MockChargePaymentExtension>::post_dispatch_details(
			pre,
			&info,
			post_info,
			len,
			&dispatch_outcome,
		)
		.unwrap();

		assert_eq!(LastPostDispatchPaysFee::get(), Some(Pays::Yes));
	});
}

#[test]
fn prepare_rejects_calls_that_become_stale_after_validation() {
	let direct_call = RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 7 });
	let calls = vec![direct_call.clone()];
	let batch = RuntimeCall::Utility(pallet_utility::Call::<Test>::batch { calls: calls.clone() });
	let batch_all =
		RuntimeCall::Utility(pallet_utility::Call::<Test>::batch_all { calls: calls.clone() });
	let force_batch = RuntimeCall::Utility(pallet_utility::Call::<Test>::force_batch { calls });
	for call in [direct_call, batch, batch_all, force_batch] {
		new_test_ext().execute_with(|| {
			set_argons(0, 1_000_000u128);
			PrepareCount::set(0);
			ValidateCount::set(0);
			LastPayer::set(None);

			let wrapper = CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(
				MockChargePaymentExtension,
			);
			let (_, val, validated_origin) = wrapper
				.validate_only(
					Some(0).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();

			ConsumedPoolKeys::<Test>::insert(7, ());

			let result = CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(
				MockChargePaymentExtension,
			)
			.prepare(val, &validated_origin, &call, &DispatchInfo::default(), 0);

			assert!(matches!(
				result,
				Err(TransactionValidityError::Invalid(InvalidTransaction::Stale))
			));
			assert_eq!(ValidateCount::get(), 1);
			assert_eq!(PrepareCount::get(), 0);
			assert_eq!(LastPayer::get(), None);
			assert_eq!(Balances::free_balance(0), 1_000_000u128);
		});
	}
}

#[test]
fn validate_ignores_general_pool_key_for_invalid_proxy() {
	new_test_ext().execute_with(|| {
		let delegate = 1u64;
		set_argons(delegate, 1_000_000u128);

		let call = RuntimeCall::Proxy(pallet_proxy::Call::<Test>::proxy {
			real: 7u64,
			force_proxy_type: None,
			call: Box::new(RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 7 })),
		});
		let (res, _, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(delegate).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();

		assert!(res.provides.is_empty());
	});
}

#[test]
fn validate_keeps_general_pool_key_for_valid_proxy() {
	new_test_ext().execute_with(|| {
		let real = 7u64;
		let delegate = 1u64;
		set_argons(real, 1_000_000u128);
		set_argons(delegate, 1_000_000u128);
		assert_ok!(Proxy::add_proxy(Some(real).into(), delegate, ProxyType::Any, 0,));

		let call = RuntimeCall::Proxy(pallet_proxy::Call::<Test>::proxy {
			real,
			force_proxy_type: None,
			call: Box::new(RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 7 })),
		});
		let (res, _, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(delegate).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();

		assert_eq!(res.provides, vec![(b"general", 7u32).encode()]);
	});
}

#[test]
fn validate_keeps_general_pool_key_for_valid_announced_proxy() {
	new_test_ext().execute_with(|| {
		frame_system::Pallet::<Test>::set_block_number(1);

		let real = 7u64;
		let delegate = 1u64;
		let relayer = 99u64;
		let inner_call = RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 7 });
		let call_hash = <Test as pallet_proxy::Config>::CallHasher::hash_of(&inner_call);
		set_argons(real, 1_000_000u128);
		set_argons(delegate, 1_000_000u128);
		set_argons(relayer, 1_000_000u128);
		assert_ok!(Proxy::add_proxy(Some(real).into(), delegate, ProxyType::Any, 1,));
		assert_ok!(Proxy::announce(Some(delegate).into(), real, call_hash));
		frame_system::Pallet::<Test>::set_block_number(2);

		let call = RuntimeCall::Proxy(pallet_proxy::Call::<Test>::proxy_announced {
			delegate,
			real,
			force_proxy_type: None,
			call: Box::new(inner_call),
		});
		let (res, _, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(relayer).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();

		assert_eq!(res.provides, vec![(b"general", 7u32).encode()]);
	});
}

#[test]
fn validate_ignores_general_pool_key_for_unannounced_proxy_announced() {
	new_test_ext().execute_with(|| {
		frame_system::Pallet::<Test>::set_block_number(1);

		let real = 7u64;
		let delegate = 1u64;
		let relayer = 99u64;
		set_argons(real, 1_000_000u128);
		set_argons(delegate, 1_000_000u128);
		set_argons(relayer, 1_000_000u128);
		assert_ok!(Proxy::add_proxy(Some(real).into(), delegate, ProxyType::Any, 1,));

		let call = RuntimeCall::Proxy(pallet_proxy::Call::<Test>::proxy_announced {
			delegate,
			real,
			force_proxy_type: None,
			call: Box::new(RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 7 })),
		});
		let (res, _, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(relayer).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();

		assert!(res.provides.is_empty());
	});
}

#[test]
fn validate_ignores_general_pool_key_for_too_early_proxy_announced() {
	new_test_ext().execute_with(|| {
		frame_system::Pallet::<Test>::set_block_number(1);

		let real = 7u64;
		let delegate = 1u64;
		let relayer = 99u64;
		let inner_call = RuntimeCall::DummyPallet(Call::<Test>::pooled { key: 7 });
		let call_hash = <Test as pallet_proxy::Config>::CallHasher::hash_of(&inner_call);
		set_argons(real, 1_000_000u128);
		set_argons(delegate, 1_000_000u128);
		set_argons(relayer, 1_000_000u128);
		assert_ok!(Proxy::add_proxy(Some(real).into(), delegate, ProxyType::Any, 2,));
		assert_ok!(Proxy::announce(Some(delegate).into(), real, call_hash));

		let call = RuntimeCall::Proxy(pallet_proxy::Call::<Test>::proxy_announced {
			delegate,
			real,
			force_proxy_type: None,
			call: Box::new(inner_call),
		});
		let (res, _, _) =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(relayer).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				)
				.unwrap();

		assert!(res.provides.is_empty());
	});
}

#[test]
fn validate_prepare_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(ValidateCount::get(), 0);
		set_argons(0, 1_000_000u128);

		let call = RuntimeCall::DummyPallet(Call::<Test>::aux { is_feeless: false, key: 1 });
		CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
			.validate_and_prepare(Some(0).into(), &call, &DispatchInfo::default(), 0, 0)
			.unwrap();
		assert_eq!(ValidateCount::get(), 1);
		assert_eq!(PrepareCount::get(), 1);

		let call = RuntimeCall::DummyPallet(Call::<Test>::aux { is_feeless: true, key: 1 });
		CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
			.validate_and_prepare(Some(0).into(), &call, &DispatchInfo::default(), 0, 0)
			.unwrap();
		assert_eq!(ValidateCount::get(), 1);
		assert_eq!(PrepareCount::get(), 1);

		// Changes from previous prepare calls persist.
		let call = RuntimeCall::DummyPallet(Call::<Test>::aux { is_feeless: false, key: 1 });
		CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
			.validate_and_prepare(Some(0).into(), &call, &DispatchInfo::default(), 0, 0)
			.unwrap();
		assert_eq!(ValidateCount::get(), 2);
		assert_eq!(PrepareCount::get(), 2);
	});
}

#[test]
fn delegation_changes_fee_payer_seen_by_payment_ext() {
	new_test_ext().execute_with(|| {
		LastPayer::set(None);

		let sponsor_id = 7u64;
		let signer_id = 1u64;
		assert_eq!(Balances::free_balance(signer_id), 0u128);

		let key = 1u32;
		let call = RuntimeCall::DummyPallet(Call::<Test>::sponsored { key });

		OneUseCodes::<Test>::insert(key, (sponsor_id, 5000)); // delegate to account 7

		// Without delegation, the signer (who has 0 balance) should fail payment validation.
		let res =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(signer_id).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				);
		assert!(matches!(res, Err(TransactionValidityError::Invalid(InvalidTransaction::Payment))));
		let sponsor_balance: Balance = 2_000_000u128;
		set_argons(sponsor_id, sponsor_balance);

		CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
			.validate_and_prepare(Some(signer_id).into(), &call, &DispatchInfo::default(), 0, 0)
			.unwrap();

		// The payment extension should have observed the delegated payer, not 0.
		assert_eq!(LastPayer::get(), Some(sponsor_id));
		assert_eq!(
			Balances::free_balance(sponsor_id),
			sponsor_balance - TipAmount::get() - FeeAmount::get()
		);
		assert_eq!(Balances::free_balance(signer_id), 0u128);
	});
}

#[test]
fn delegation_changes_fee_payer_seen_by_payment_ext_with_proxy() {
	new_test_ext().execute_with(|| {
		LastPayer::set(None);

		let sponsor_id = 7u64;
		let signer_id = 1u64;
		assert_eq!(Balances::free_balance(signer_id), 0u128);

		let key = 1u32;
		let proxy = 8u64;
		let call = RuntimeCall::Proxy(pallet_proxy::Call::<Test>::proxy {
			real: signer_id,
			call: Box::new(RuntimeCall::DummyPallet(Call::<Test>::sponsored { key })),
			force_proxy_type: None,
		});

		OneUseCodes::<Test>::insert(key, (sponsor_id, 5000)); // delegate to account 7

		// Without delegation, the signer (who has 0 balance) should fail payment validation.
		let res =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(proxy).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				);
		assert!(matches!(res, Err(TransactionValidityError::Invalid(InvalidTransaction::Payment))));
		let sponsor_balance: Balance = 2_000_000u128;
		set_argons(sponsor_id, sponsor_balance);

		CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
			.validate_and_prepare(Some(proxy).into(), &call, &DispatchInfo::default(), 0, 0)
			.unwrap();

		// The payment extension should have observed the delegated payer, not 0.
		assert_eq!(LastPayer::get(), Some(sponsor_id));
		assert_eq!(
			Balances::free_balance(sponsor_id),
			sponsor_balance - TipAmount::get() - FeeAmount::get()
		);
		assert_eq!(Balances::free_balance(signer_id), 0u128);
	});
}

#[test]
fn proxy_specific_sponsors_require_registered_delegate() {
	new_test_ext().execute_with(|| {
		let real = 7u64;
		let delegate = 1u64;
		let attacker = 2u64;
		let key = 35u32;

		set_argons(real, 1_000_000u128);
		assert_ok!(Proxy::add_proxy(Some(real).into(), delegate, ProxyType::DummyWrapper, 0,));

		let call = RuntimeCall::Proxy(pallet_proxy::Call::<Test>::proxy {
			real,
			force_proxy_type: Some(ProxyType::DummyWrapper),
			call: Box::new(RuntimeCall::DummyPallet(Call::<Test>::pooled { key })),
		});

		let result =
			CheckFeeWrapper::<Test, MockChargePaymentExtension>::from(MockChargePaymentExtension)
				.validate_only(
					Some(attacker).into(),
					&call,
					&DispatchInfo::default(),
					0,
					TransactionSource::External,
					0,
				);

		assert!(matches!(
			result,
			Err(TransactionValidityError::Invalid(InvalidTransaction::Payment))
		));
	});
}

#[test]
fn proxy_specific_sponsors_charge_real_account_directly() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		FeeUnbalancedAmount::set(0);
		TipUnbalancedAmount::set(0);

		let real = 7u64;
		let delegate = 1u64;
		let key = 33u32;
		let delegate_balance: Balance = 1_000_000_000_000u128;
		let sponsor_balance: Balance = 1_000_000_000_000u128;
		let tip: Balance = 500u128;

		set_argons(delegate, delegate_balance);
		set_argons(real, sponsor_balance);
		assert_ok!(Proxy::add_proxy(Some(real).into(), delegate, ProxyType::DummyWrapper, 0,));

		let call = RuntimeCall::Proxy(pallet_proxy::Call::<Test>::proxy {
			real,
			force_proxy_type: Some(ProxyType::DummyWrapper),
			call: Box::new(RuntimeCall::DummyPallet(Call::<Test>::pooled { key })),
		});
		let info = call.get_dispatch_info();
		let len = call.encoded_size();
		let origin = crate::mock::RuntimeOrigin::signed(delegate);
		let delegate_before = Balances::free_balance(delegate);
		let sponsor_before = Balances::free_balance(real);
		let wrapper = CheckFeeWrapper::<Test, ChargeTransactionPayment<Test>>::from(
			ChargeTransactionPayment::<Test>::from(tip),
		);
		let (_, val, _) = wrapper
			.validate_only(origin.clone(), &call, &info, len, TransactionSource::External, 0)
			.unwrap();
		let pre = CheckFeeWrapper::<Test, ChargeTransactionPayment<Test>>::from(
			ChargeTransactionPayment::<Test>::from(tip),
		)
		.prepare(val, &origin, &call, &info, len)
		.unwrap();

		let dispatch_result = call.dispatch(origin);
		let outer_dispatch_outcome = match &dispatch_result {
			Ok(_) => Ok(()),
			Err(err) => Err(err.error),
		};
		let post_info = match &dispatch_result {
			Ok(post_info) => post_info,
			Err(err) => &err.post_info,
		};
		let actual_fee = pallet_transaction_payment::Pallet::<Test>::compute_actual_fee(
			len as u32, &info, post_info, tip,
		);

		CheckFeeWrapper::<Test, ChargeTransactionPayment<Test>>::post_dispatch_details(
			pre,
			&info,
			post_info,
			len,
			&outer_dispatch_outcome,
		)
		.unwrap();

		assert!(outer_dispatch_outcome.is_ok());
		assert!(DispatchedPoolKeys::<Test>::contains_key(key));
		assert_eq!(Balances::free_balance(delegate), delegate_before);
		assert_eq!(Balances::free_balance(real), sponsor_before - actual_fee);
		assert_eq!(FeeUnbalancedAmount::get() + TipUnbalancedAmount::get(), actual_fee);
		let events = System::events();
		assert!(
			events.iter().any(|record| {
				matches!(
					&record.event,
					RuntimeEvent::FeeControl(Event::FeeDelegated { from, to, .. })
						if *from == delegate && *to == real
				)
			}),
			"{events:?}"
		);
	});
}

#[test]
fn proxy_specific_sponsors_charge_real_account_for_failed_calls() {
	new_test_ext().execute_with(|| {
		FeeUnbalancedAmount::set(0);
		TipUnbalancedAmount::set(0);

		let real = 7u64;
		let delegate = 1u64;
		let key = 34u32;
		let delegate_balance: Balance = 1_000_000_000_000u128;
		let sponsor_balance: Balance = 1_000_000_000_000u128;
		let tip: Balance = 500u128;

		set_argons(delegate, delegate_balance);
		set_argons(real, sponsor_balance);
		assert_ok!(Proxy::add_proxy(Some(real).into(), delegate, ProxyType::DummyWrapper, 0,));

		let call = RuntimeCall::Proxy(pallet_proxy::Call::<Test>::proxy {
			real,
			force_proxy_type: Some(ProxyType::DummyWrapper),
			call: Box::new(RuntimeCall::DummyPallet(Call::<Test>::pooled_fail { key })),
		});
		let info = call.get_dispatch_info();
		let len = call.encoded_size();
		let origin = crate::mock::RuntimeOrigin::signed(delegate);
		let delegate_before = Balances::free_balance(delegate);
		let sponsor_before = Balances::free_balance(real);
		let wrapper = CheckFeeWrapper::<Test, ChargeTransactionPayment<Test>>::from(
			ChargeTransactionPayment::<Test>::from(tip),
		);
		let (_, val, _) = wrapper
			.validate_only(origin.clone(), &call, &info, len, TransactionSource::External, 0)
			.unwrap();
		let pre = CheckFeeWrapper::<Test, ChargeTransactionPayment<Test>>::from(
			ChargeTransactionPayment::<Test>::from(tip),
		)
		.prepare(val, &origin, &call, &info, len)
		.unwrap();

		let dispatch_result = call.dispatch(origin);
		let outer_dispatch_outcome = match &dispatch_result {
			Ok(_) => Ok(()),
			Err(err) => Err(err.error),
		};
		let post_info = match &dispatch_result {
			Ok(post_info) => post_info,
			Err(err) => &err.post_info,
		};
		let actual_fee = pallet_transaction_payment::Pallet::<Test>::compute_actual_fee(
			len as u32, &info, post_info, tip,
		);

		CheckFeeWrapper::<Test, ChargeTransactionPayment<Test>>::post_dispatch_details(
			pre,
			&info,
			post_info,
			len,
			&outer_dispatch_outcome,
		)
		.unwrap();

		// `proxy(...)` returns `Ok(())` even when the proxied call fails.
		assert!(outer_dispatch_outcome.is_ok());
		assert!(!DispatchedPoolKeys::<Test>::contains_key(key));
		assert_eq!(Balances::free_balance(delegate), delegate_before);
		assert_eq!(Balances::free_balance(real), sponsor_before - actual_fee);
		assert_eq!(FeeUnbalancedAmount::get() + TipUnbalancedAmount::get(), actual_fee);
	});
}

fn set_argons(account_id: u64, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}
