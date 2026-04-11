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
	Balances, FeeAmount, LastPayer, MockChargePaymentExtension, PrepareCount, Proxy, ProxyType,
	RuntimeCall, Test, TipAmount, ValidateCount, new_test_ext,
	pallet_dummy::{Call, OneUseCodes},
};
use frame_support::dispatch::DispatchInfo;
use frame_system::RawOrigin;
use pallet_prelude::frame_support::traits::Currency;
use sp_runtime::{
	traits::{DispatchTransaction, Hash},
	transaction_validity::TransactionSource,
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
			delegate: delegate.into(),
			real: real.into(),
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
			delegate: delegate.into(),
			real: real.into(),
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
			delegate: delegate.into(),
			real: real.into(),
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

fn set_argons(account_id: u64, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}
