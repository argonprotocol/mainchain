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
	Balances, FeeAmount, LastPayer, MockChargePaymentExtension, PrepareCount, RuntimeCall, Test,
	TipAmount, ValidateCount, new_test_ext,
	pallet_dummy::{Call, OneUseCodes},
};
use frame_support::dispatch::DispatchInfo;
use pallet_prelude::frame_support::traits::Currency;
use sp_runtime::{traits::DispatchTransaction, transaction_validity::TransactionSource};

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
