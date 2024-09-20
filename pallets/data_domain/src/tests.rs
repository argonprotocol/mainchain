use crate::{
	mock::{DataDomain as DataDomainPallet, *},
	pallet::{ExpiringDomainsByBlock, RegisteredDataDomains},
	DataDomainRegistration, Error, Event,
};
use argon_primitives::{
	notebook::NotebookHeader, tick::Tick, AccountId, DataDomain, DataDomainHash, DataTLD,
	NotebookEventHandler, Semver, VersionHost, ZoneRecord,
};
use frame_support::{assert_err, assert_ok, traits::Hooks};
use sp_keyring::AccountKeyring::{Alice, Bob};
use sp_runtime::{testing::H256, BoundedVec};
use std::collections::BTreeMap;

#[test]
fn it_can_register_domains() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let domain =
			DataDomain { top_level_domain: DataTLD::Analytics, domain_name: "test".into() }.hash();
		DataDomainPallet::notebook_submitted(&create_notebook(
			1,
			vec![(domain, Bob.to_account_id())],
		));
		assert_eq!(
			RegisteredDataDomains::<Test>::get(domain),
			Some(DataDomainRegistration { account_id: Bob.to_account_id(), registered_at_tick: 1 })
		);
		assert_eq!(ExpiringDomainsByBlock::<Test>::get(1001).len(), 1);
		System::assert_last_event(
			Event::DataDomainRegistered {
				domain_hash: domain,
				registration: DataDomainRegistration {
					account_id: Bob.to_account_id(),
					registered_at_tick: 1,
				},
			}
			.into(),
		);
	});
}
#[test]
fn it_cancels_conflicting_domains() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let domain =
			DataDomain { top_level_domain: DataTLD::Analytics, domain_name: "test".into() }.hash();
		DataDomainPallet::notebook_submitted(&create_notebook(
			1,
			vec![(domain, Bob.to_account_id()), (domain, Alice.to_account_id())],
		));
		assert_eq!(RegisteredDataDomains::<Test>::get(domain), None);
		assert_eq!(ExpiringDomainsByBlock::<Test>::get(1001).len(), 0);
		System::assert_last_event(
			Event::DataDomainRegistrationCanceled {
				domain_hash: domain,
				registration: DataDomainRegistration {
					account_id: Bob.to_account_id(),
					registered_at_tick: 1,
				},
			}
			.into(),
		);
	});
}
#[test]
fn it_renews_domains() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let domain =
			DataDomain { top_level_domain: DataTLD::Analytics, domain_name: "test".into() }.hash();
		DataDomainPallet::notebook_submitted(&create_notebook(
			1,
			vec![(domain, Bob.to_account_id())],
		));
		assert_eq!(ExpiringDomainsByBlock::<Test>::get(1001).len(), 1);

		System::set_block_number(100);
		CurrentTick::set(100);
		DataDomainPallet::notebook_submitted(&create_notebook(
			100,
			vec![(domain, Bob.to_account_id())],
		));
		assert_eq!(ExpiringDomainsByBlock::<Test>::get(1001).len(), 0);
		assert_eq!(ExpiringDomainsByBlock::<Test>::get(1100).len(), 1);
		System::assert_last_event(Event::DataDomainRenewed { domain_hash: domain }.into());
	});
}
#[test]
fn it_ignores_duplicated_domains() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let domain =
			DataDomain { top_level_domain: DataTLD::Analytics, domain_name: "test".into() }.hash();
		DataDomainPallet::notebook_submitted(&create_notebook(
			1,
			vec![(domain, Bob.to_account_id())],
		));
		let registered_to_bob =
			DataDomainRegistration { account_id: Bob.to_account_id(), registered_at_tick: 1 };
		assert_eq!(RegisteredDataDomains::<Test>::get(domain), Some(registered_to_bob.clone()));

		System::set_block_number(2);
		CurrentTick::set(2);
		DataDomainPallet::notebook_submitted(&create_notebook(
			2,
			vec![(domain, Alice.to_account_id())],
		));
		assert_eq!(RegisteredDataDomains::<Test>::get(domain), Some(registered_to_bob));
	});
}
#[test]
fn it_registers_zone_records() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentTick::set(2);
		let domain =
			DataDomain { top_level_domain: DataTLD::Analytics, domain_name: "test".into() }.hash();
		DataDomainPallet::notebook_submitted(&create_notebook(
			1,
			vec![(domain, Bob.to_account_id())],
		));

		let zone = ZoneRecord {
			payment_account: Bob.to_account_id(),
			notary_id: 1,
			versions: BTreeMap::from([(
				Semver::new(1, 0, 0),
				VersionHost {
					host: "wss://127.0.0.1:8080".into(),
					datastore_id: BoundedVec::truncate_from(b"test".to_vec()),
				},
			)]),
		};

		assert_ok!(DataDomainPallet::set_zone_record(
			RuntimeOrigin::signed(Bob.to_account_id()),
			domain,
			zone.clone()
		));
		System::assert_last_event(
			Event::ZoneRecordUpdated { domain_hash: domain, zone_record: zone.clone() }.into(),
		);
		assert_err!(
			DataDomainPallet::set_zone_record(
				RuntimeOrigin::signed(Alice.to_account_id()),
				domain,
				zone.clone()
			),
			Error::<Test>::NotDomainOwner
		);

		assert_err!(
			DataDomainPallet::set_zone_record(
				RuntimeOrigin::signed(Bob.to_account_id()),
				DataDomain { top_level_domain: DataTLD::Automotive, domain_name: "test".into() }
					.hash(),
				zone.clone()
			),
			Error::<Test>::DomainNotRegistered
		);
	});
}

#[test]
fn it_expires_domains() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let domain =
			DataDomain { top_level_domain: DataTLD::Analytics, domain_name: "test".into() }.hash();
		DataDomainPallet::notebook_submitted(&create_notebook(
			1,
			vec![(domain, Bob.to_account_id())],
		));

		System::set_block_number(1001);
		CurrentTick::set(1001);
		DataDomainPallet::on_initialize(1001);
		assert_eq!(RegisteredDataDomains::<Test>::get(domain), None);
	});
}

fn create_notebook(tick: Tick, domains: Vec<(DataDomainHash, AccountId)>) -> NotebookHeader {
	NotebookHeader {
		version: 1,
		notary_id: 1,
		notebook_number: 1,
		tick,
		changed_accounts_root: Default::default(),
		chain_transfers: Default::default(),
		changed_account_origins: Default::default(),
		tax: 0,
		// Block Votes
		parent_secret: None,
		secret_hash: H256::from_slice(&[0u8; 32]),
		block_voting_power: 1,
		block_votes_root: H256::from_slice(&[0u8; 32]),
		block_votes_count: 1,
		blocks_with_votes: Default::default(),
		data_domains: BoundedVec::truncate_from(domains),
	}
}
