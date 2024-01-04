use std::alloc::System;

use frame_support::{assert_err, assert_ok, traits::Hooks};
use sp_keyring::AccountKeyring::{Alice, Bob};
use sp_runtime::{testing::H256, BoundedVec};
use ulx_primitives::{
	host::Host, notebook::NotebookHeader, tick::Tick, AccountId, DataDomain, DataDomainProvider,
	DataTLD, NotebookEventHandler, Semver, VersionHost, ZoneRecord,
};

use crate::{
	mock::{DataDomain as DataDomainPallet, *},
	pallet::{DomainPaymentAddressHistory, ExpiringDomainsByBlock, RegisteredDataDomains},
	DataDomainRegistration, Error, Event,
};

#[test]
fn it_can_register_domains() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_eq!(System::block_number(), 1);
	});
}
