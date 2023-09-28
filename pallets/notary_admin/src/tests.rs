use frame_support::{
	assert_noop, assert_ok,
	traits::{Len, OnFinalize, OnInitialize},
};

use crate::{Error, Event};
use sp_keyring::Ed25519Keyring;
use ulx_primitives::notary::{NotaryMeta, NotaryRecord};

use crate::{
	mock::*,
	pallet::{ActiveNotaries, ExpiringProposals, ProposedNotaries, QueuedNotaryMetaChanges},
};

#[test]
fn it_can_propose_a_notary() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NotaryAdmin::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta { public: Ed25519Keyring::Alice.public().into(), ip: 0, port: 0 }
		));

		System::assert_last_event(
			Event::NotaryProposed {
				operator_account: 1,
				meta: NotaryMeta { public: Ed25519Keyring::Alice.public().into(), ip: 0, port: 0 },
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
		assert_ok!(NotaryAdmin::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta { public: Ed25519Keyring::Alice.public().into(), ip: 0, port: 0 }
		));
		assert_eq!(ProposedNotaries::<Test>::get(1).len(), 1);

		System::set_block_number(11);
		NotaryAdmin::on_initialize(11);
		NotaryAdmin::on_finalize(11);
		assert_eq!(ProposedNotaries::<Test>::get(1).len(), 0);
	});
}

#[test]
fn it_only_allows_one_proposal_per_account_at_a_time() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NotaryAdmin::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta { public: Ed25519Keyring::Alice.public().into(), ip: 0, port: 0 }
		));
		System::set_block_number(2);
		assert_ok!(NotaryAdmin::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta { public: Ed25519Keyring::Alice.public().into(), ip: 1, port: 0 }
		));
		assert_eq!(ProposedNotaries::<Test>::get(1).len(), 1);
	});
}

#[test]
fn it_allows_proposal_elevation() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NotaryAdmin::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta { public: Ed25519Keyring::Alice.public().into(), ip: 0, port: 0 }
		));
		System::set_block_number(2);
		assert_ok!(NotaryAdmin::activate(RuntimeOrigin::root(), 1,));
		assert_eq!(ProposedNotaries::<Test>::get(1).len(), 0);
		assert_eq!(
			ActiveNotaries::<Test>::get().into_inner(),
			vec![NotaryRecord {
				notary_id: 1,
				operator_account_id: 1,
				meta: {
					NotaryMeta { public: Ed25519Keyring::Alice.public().into(), ip: 0, port: 0 }
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
		NotaryAdmin::on_initialize(11);
		NotaryAdmin::on_finalize(11);
		assert_eq!(ProposedNotaries::<Test>::get(1).len(), 0);
	});
}

#[test]
fn it_allows_a_notary_to_update_meta() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NotaryAdmin::propose(
			RuntimeOrigin::signed(1),
			NotaryMeta { public: Ed25519Keyring::Alice.public().into(), ip: 0, port: 0 }
		));
		System::set_block_number(2);
		NotaryAdmin::on_initialize(2);
		assert_ok!(NotaryAdmin::activate(RuntimeOrigin::root(), 1,));
		NotaryAdmin::on_finalize(2);

		System::set_block_number(3);
		NotaryAdmin::on_initialize(3);
		assert_noop!(
			NotaryAdmin::update(
				RuntimeOrigin::signed(2),
				1,
				NotaryMeta { public: Ed25519Keyring::Alice.public().into(), ip: 2, port: 2 }
			),
			Error::<Test>::InvalidNotaryOperator
		);
		assert_ok!(NotaryAdmin::update(
			RuntimeOrigin::signed(1),
			1,
			NotaryMeta { public: Ed25519Keyring::Bob.public().into(), ip: 2, port: 2 }
		),);
		NotaryAdmin::on_finalize(3);

		// should not take effect yet!
		assert_eq!(ActiveNotaries::<Test>::get()[0].meta.ip, 0);
		assert_eq!(ActiveNotaries::<Test>::get()[0].meta_updated_block, 2);
		System::assert_last_event(
			Event::NotaryMetaUpdateQueued {
				notary_id: 1,
				meta: NotaryMeta { public: Ed25519Keyring::Bob.public().into(), ip: 2, port: 2 },
				effective_block: 4,
			}
			.into(),
		);
		assert_eq!(QueuedNotaryMetaChanges::<Test>::get(4).len(), 1);

		System::set_block_number(4);
		NotaryAdmin::on_initialize(4);
		assert_eq!(ActiveNotaries::<Test>::get()[0].meta.ip, 2);
		assert_eq!(ActiveNotaries::<Test>::get()[0].meta_updated_block, 4);
		System::assert_last_event(
			Event::NotaryMetaUpdated {
				notary_id: 1,
				meta: NotaryMeta { public: Ed25519Keyring::Bob.public().into(), ip: 2, port: 2 },
			}
			.into(),
		);
	});
}
