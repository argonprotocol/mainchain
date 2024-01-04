use crate::{Error, Event};
use frame_support::{
	assert_noop, assert_ok,
	traits::{Len, OnFinalize, OnInitialize},
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_core::H256;
use sp_keyring::Ed25519Keyring;
use sp_runtime::BoundedVec;
use ulx_primitives::{
	host::Host,
	notary::{NotaryMeta, NotaryProvider, NotaryPublic, NotaryRecord},
};

use crate::{
	mock::*,
	pallet::{
		ActiveNotaries, ExpiringProposals, NotaryKeyHistory, ProposedNotaries,
		QueuedNotaryMetaChanges,
	},
};

#[test]
fn it_can_propose_a_notary() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Notaries::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta::<MaxNotaryHosts> {
				public: Ed25519Keyring::Alice.public().into(),
				hosts: rpc_hosts(0, 0),
			}
		));

		System::assert_last_event(
			Event::NotaryProposed {
				operator_account: 1,
				meta: NotaryMeta::<MaxNotaryHosts> {
					public: Ed25519Keyring::Alice.public().into(),
					hosts: rpc_hosts(0, 0),
				},
				expires: (1u32 + MaxProposalHoldBlocks::get()).into(),
			}
			.into(),
		);

		assert_eq!(ProposedNotaries::<Test>::get(1).len(), 1);
		assert_eq!(ActiveNotaries::<Test>::get().len(), 0);
	});
}

#[test]
fn it_cleans_up_proposals() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Notaries::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta::<MaxNotaryHosts> {
				public: Ed25519Keyring::Alice.public().into(),
				hosts: rpc_hosts(0, 0),
			}
		));
		assert_eq!(ProposedNotaries::<Test>::get(1).len(), 1);

		System::set_block_number(11);
		Notaries::on_initialize(11);
		Notaries::on_finalize(11);
		assert_eq!(ProposedNotaries::<Test>::get(1).len(), 0);
	});
}

#[test]
fn it_only_allows_one_proposal_per_account_at_a_time() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Notaries::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta::<MaxNotaryHosts> {
				public: Ed25519Keyring::Alice.public().into(),
				hosts: rpc_hosts(0, 0),
			}
		));
		System::set_block_number(2);
		assert_ok!(Notaries::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta::<MaxNotaryHosts> {
				public: Ed25519Keyring::Alice.public().into(),
				hosts: rpc_hosts(1, 0),
			}
		));
		assert_eq!(ProposedNotaries::<Test>::get(1).len(), 1);
	});
}

#[test]
fn it_allows_proposal_activation() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Notaries::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta::<MaxNotaryHosts> {
				public: Ed25519Keyring::Alice.public().into(),
				hosts: rpc_hosts(0, 0),
			}
		));
		System::set_block_number(2);
		assert_ok!(Notaries::activate(RuntimeOrigin::root(), 1,));
		assert_eq!(ProposedNotaries::<Test>::get(1).len(), 0);
		assert_eq!(
			ActiveNotaries::<Test>::get().into_inner(),
			vec![NotaryRecord {
				notary_id: 1,
				operator_account_id: 1,
				meta: {
					NotaryMeta::<MaxNotaryHosts> {
						public: Ed25519Keyring::Alice.public().into(),
						hosts: rpc_hosts(0, 0),
					}
				},
				activated_block: 2,
				meta_updated_block: 2,
			}]
		);

		System::assert_last_event(
			Event::NotaryActivated { notary: ActiveNotaries::<Test>::get()[0].clone() }.into(),
		);

		assert_eq!(ExpiringProposals::<Test>::get(11).len(), 0);
		System::set_block_number(11);
		Notaries::on_initialize(11);
		Notaries::on_finalize(11);
		assert_eq!(ProposedNotaries::<Test>::get(1).len(), 0);
		assert_eq!(
			NotaryKeyHistory::<Test>::get(1),
			BoundedVec::<(BlockNumberFor<Test>, NotaryPublic), MaxBlocksForKeyHistory>::truncate_from(vec![(2u32, Ed25519Keyring::Alice.public())])
		);
	});
}

#[test]
fn it_allows_a_notary_to_update_meta() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Notaries::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta::<MaxNotaryHosts> {
				public: Ed25519Keyring::Alice.public().into(),
				hosts: rpc_hosts(0, 0),
			}
		));
		System::set_block_number(2);
		Notaries::on_initialize(2);
		assert_ok!(Notaries::activate(RuntimeOrigin::root(), 1,));
		Notaries::on_finalize(2);

		System::set_block_number(3);
		Notaries::on_initialize(3);
		assert_noop!(
			Notaries::update(
				RuntimeOrigin::signed(2),
				1,
				NotaryMeta::<MaxNotaryHosts> {
					public: Ed25519Keyring::Alice.public().into(),
					hosts: rpc_hosts(2, 2),
				}
			),
			Error::<Test>::InvalidNotaryOperator
		);
		assert_ok!(Notaries::update(
			RuntimeOrigin::signed(1),
			1,
			NotaryMeta::<MaxNotaryHosts> {
				public: Ed25519Keyring::Bob.public().into(),
				hosts: rpc_hosts(2, 2),
			}
		),);
		Notaries::on_finalize(3);

		// should not take effect yet!
		assert_eq!(ActiveNotaries::<Test>::get()[0].meta.hosts[0].ip, 0);
		assert_eq!(ActiveNotaries::<Test>::get()[0].meta_updated_block, 2);
		System::assert_last_event(
			Event::NotaryMetaUpdateQueued {
				notary_id: 1,
				meta: NotaryMeta::<MaxNotaryHosts> {
					public: Ed25519Keyring::Bob.public().into(),
					hosts: rpc_hosts(2, 2),
				},
				effective_block: 4,
			}
			.into(),
		);
		assert_eq!(QueuedNotaryMetaChanges::<Test>::get(4).len(), 1);

		System::set_block_number(4);
		Notaries::on_initialize(4);
		assert_eq!(ActiveNotaries::<Test>::get()[0].meta.hosts[0].ip, 2);
		assert_eq!(ActiveNotaries::<Test>::get()[0].meta_updated_block, 4);
		System::assert_last_event(
			Event::NotaryMetaUpdated {
				notary_id: 1,
				meta: NotaryMeta::<MaxNotaryHosts> {
					public: Ed25519Keyring::Bob.public().into(),
					hosts: rpc_hosts(2, 2),
				},
			}
			.into(),
		);
		assert_eq!(
			NotaryKeyHistory::<Test>::get(1),
			BoundedVec::<(BlockNumberFor<Test>, NotaryPublic), MaxBlocksForKeyHistory>::truncate_from(vec![
				(2u32, Ed25519Keyring::Alice.public()),
				(4u32, Ed25519Keyring::Bob.public())
			])
		);
	});
}

#[test]
fn it_verifies_notary_signatures_matching_block_heights() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Notaries::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta::<MaxNotaryHosts> {
				public: Ed25519Keyring::Alice.public().into(),
				hosts: rpc_hosts(0, 0),
			}
		));
		System::set_block_number(2);
		Notaries::on_initialize(2);
		assert_ok!(Notaries::activate(RuntimeOrigin::root(), 1,));
		Notaries::on_finalize(2);

		System::set_block_number(3);
		Notaries::on_initialize(3);
		assert_ok!(Notaries::update(
			RuntimeOrigin::signed(1),
			1,
			NotaryMeta::<MaxNotaryHosts> {
				public: Ed25519Keyring::Bob.public().into(),
				hosts: rpc_hosts(2, 2),
			}
		),);
		Notaries::on_finalize(3);
		System::set_block_number(4);
		Notaries::on_initialize(4);
		Notaries::on_initialize(4);
		let hash: H256 = [1u8; 32].into();

		assert_eq!(
			<Notaries as NotaryProvider<Block>>::verify_signature(
				1,
				4,
				&hash,
				&Ed25519Keyring::Alice.sign(&hash[..]),
			),
			false
		);
		assert_eq!(
			<Notaries as NotaryProvider<Block>>::verify_signature(
				1,
				2,
				&hash,
				&Ed25519Keyring::Alice.sign(&hash[..]),
			),
			true
		);
		assert_eq!(
			<Notaries as NotaryProvider<Block>>::verify_signature(
				1,
				4,
				&hash,
				&Ed25519Keyring::Bob.sign(&hash[..]),
			),
			true
		);
	});
}

fn rpc_hosts<S>(ip: u32, port: u16) -> BoundedVec<Host, S>
where
	S: sp_core::Get<u32>,
{
	BoundedVec::<Host, S>::truncate_from(vec![Host { ip, port, is_secure: false }])
}
