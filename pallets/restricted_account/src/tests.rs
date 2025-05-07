use crate::{mock::*, AccountAccessList, AccountOwner};
use frame_support::assert_ok;
use pallet_prelude::{
	sp_runtime::{bounded_vec, generic::Era},
	*,
};

type Error = crate::Error<Test>;

#[test]
fn should_be_able_to_set_restricted_account() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(RestrictedAccount::register(
			RuntimeOrigin::signed(1),
			2,
			bounded_vec![AccessType::Any]
		));

		assert_eq!(AccountOwner::<Test>::get(1), Some(2));
		assert_eq!(AccountAccessList::<Test>::get(1), Some(bounded_vec![AccessType::Any]));

		assert_err!(
			RestrictedAccount::register(RuntimeOrigin::signed(1), 2, bounded_vec![AccessType::Any]),
			Error::AccountAlreadyRestricted
		);
	});
}

#[test]
fn it_can_modify_restricted_account() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(RestrictedAccount::register(
			RuntimeOrigin::signed(1),
			2,
			bounded_vec![AccessType::Any]
		));

		assert_err!(
			RestrictedAccount::modify_access(
				RuntimeOrigin::signed(1),
				1,
				bounded_vec![AccessType::MiningBid]
			),
			Error::AccountNotOwner
		);

		assert_ok!(RestrictedAccount::modify_access(
			RuntimeOrigin::signed(2),
			1,
			bounded_vec![AccessType::MiningBid]
		));

		assert_eq!(AccountAccessList::<Test>::get(1), Some(bounded_vec![AccessType::MiningBid]));
	});
}

#[test]
fn it_can_deregister_restricted_account() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(RestrictedAccount::register(
			RuntimeOrigin::signed(1),
			2,
			bounded_vec![AccessType::Any]
		));

		assert_err!(
			RestrictedAccount::deregister(RuntimeOrigin::signed(1), 1,),
			Error::AccountNotOwner
		);
		assert_ok!(RestrictedAccount::deregister(RuntimeOrigin::signed(2), 1));

		assert_eq!(AccountOwner::<Test>::get(1), None);
		assert_eq!(AccountAccessList::<Test>::get(1), None);
	});
}

#[test]
fn it_can_dispatch_calls_as_restricted_account() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(&1, 1000);
		assert_ok!(RestrictedAccount::register(
			RuntimeOrigin::signed(1),
			2,
			bounded_vec![AccessType::MiningBid]
		));

		// Dispatch via an unchecked extrinsic to invoke SignedExtensions
		let call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
			dest: 3,
			value: 100,
		});
		let xt = UncheckedXt::new_signed(
			call.clone(),
			1,
			1.into(),
			(frame_system::CheckEra::from(Era::Immortal), Default::default()),
		);
		assert_eq!(
			TestExecutive::validate_transaction(
				TransactionSource::InBlock,
				xt.clone(),
				Default::default(),
			),
			Err(TransactionValidityError::Invalid(InvalidTransaction::BadSigner)),
		);
		assert_eq!(
			TestExecutive::apply_extrinsic(xt),
			Err(TransactionValidityError::Invalid(InvalidTransaction::BadSigner))
		);

		assert_ok!(RestrictedAccount::owner_dispatch(
			RuntimeOrigin::signed(2),
			1,
			Box::new(call.clone())
		));
		assert_eq!(Balances::free_balance(3), 100);

		// try dispatching through the executive
		let xt = UncheckedXt::new_signed(
			RuntimeCall::RestrictedAccount(crate::Call::owner_dispatch {
				restricted_account: 1,
				call: Box::new(call),
			}),
			2,
			2.into(),
			(frame_system::CheckEra::from(Era::Immortal), Default::default()),
		);
		assert_ok!(TestExecutive::validate_transaction(
			TransactionSource::InBlock,
			xt.clone(),
			Default::default(),
		));
	});
}
