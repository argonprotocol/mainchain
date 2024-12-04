use std::collections::BTreeMap;

use argon_primitives::{
	bitcoin::{
		BitcoinBlock, BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinRejectedReason, H256Le,
		UtxoRef, UtxoValue,
	},
	inherents::BitcoinUtxoSync,
	BitcoinUtxoTracker,
};
use frame_support::{assert_err, assert_noop, assert_ok, pallet_prelude::Hooks};
use sp_core::H256;
use sp_runtime::DispatchError;

use crate::{
	mock::{System, *},
	pallet::{
		ConfirmedBitcoinBlockTip, InherentIncluded, LockedUtxoExpirationsByBlock, LockedUtxos,
		UtxoIdToRef, UtxosPendingConfirmation,
	},
	Error, Event,
};

#[test]
fn only_an_operator_can_submit_oracle_block() {
	new_test_ext().execute_with(|| {
		let who = 1;
		System::set_block_number(1);
		assert_noop!(
			BitcoinUtxos::set_confirmed_block(RuntimeOrigin::signed(who), 1, H256Le([0; 32])),
			Error::<Test>::NoPermissions
		);

		assert_ok!(BitcoinUtxos::set_operator(RuntimeOrigin::root(), who));
		assert_ok!(BitcoinUtxos::set_confirmed_block(
			RuntimeOrigin::signed(who),
			1,
			H256Le([0; 32])
		),);
		assert_eq!(
			ConfirmedBitcoinBlockTip::<Test>::get(),
			Some(BitcoinBlock { block_height: 1, block_hash: H256Le([0; 32]) })
		);
	});
}

#[test]
fn can_watch_utxos() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let script_pubkey = make_pubkey([0u8; 34]);
		let satoshis = 100;
		let watch_for_spent_until = 10;
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 1,
			block_hash: H256Le([0; 32]),
		});
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script_pubkey, satoshis, watch_for_spent_until),);
		assert_eq!(
			*UtxosPendingConfirmation::<Test>::get().get(&1).unwrap(),
			UtxoValue {
				utxo_id: 1,
				script_pubkey,
				satoshis,
				submitted_at_height: 1,
				watch_for_spent_until_height: 10
			}
		);

		// should not be able to watch the same pubkey again
		assert_err!(
			BitcoinUtxos::watch_for_utxo(2, script_pubkey, satoshis, watch_for_spent_until),
			Error::<Test>::ScriptPubkeyConflict
		);

		// check that it is removed on expiration
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: (1 + MaxPendingConfirmationBlocks::get() as BitcoinHeight) + 1,
			block_hash: H256Le([0; 32]),
		});
		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				verified: Default::default(),
				invalid: Default::default(),
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 10, block_hash: H256Le([0; 32]) },
			},
		));
		System::assert_last_event(
			Event::UtxoRejected {
				utxo_id: 1,
				rejected_reason: BitcoinRejectedReason::LookupExpired,
			}
			.into(),
		);
		assert!(UtxosPendingConfirmation::<Test>::get().is_empty());
	});
}

#[test]
fn it_requires_block_sync_to_be_newer() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let mut utxo_sync = BitcoinUtxoSync {
			verified: Default::default(),
			invalid: Default::default(),
			spent: Default::default(),
			sync_to_block: BitcoinBlock { block_height: 2, block_hash: H256Le([0; 32]) },
		};
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 1,
			block_hash: H256Le([0; 32]),
		});

		assert_err!(
			BitcoinUtxos::sync(RuntimeOrigin::none(), utxo_sync.clone()),
			Error::<Test>::InvalidBitcoinSyncHeight
		);

		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});
		assert_ok!(BitcoinUtxos::sync(RuntimeOrigin::none(), utxo_sync.clone()),);

		// simulate next block
		BitcoinUtxos::on_finalize(2);
		InherentIncluded::<Test>::set(false);
		// should not allow synching an older block
		utxo_sync.sync_to_block.block_height = 1;
		assert_err!(
			BitcoinUtxos::sync(RuntimeOrigin::none(), utxo_sync),
			Error::<Test>::InvalidBitcoinSyncHeight
		);
	});
}

#[test]
fn it_should_move_utxos_to_lock_once_verified() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let script_pubkey = make_pubkey([0u8; 34]);
		let satoshis = 100;
		let watch_for_spent_until = 10;
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script_pubkey, satoshis, watch_for_spent_until),);
		let utxo_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };

		let utxo_sync = BitcoinUtxoSync {
			verified: BTreeMap::from([(1, utxo_ref.clone())]),
			invalid: Default::default(),
			spent: Default::default(),
			sync_to_block: BitcoinBlock { block_height: 1, block_hash: H256Le([0; 32]) },
		};
		assert_ok!(BitcoinUtxos::sync(RuntimeOrigin::none(), utxo_sync),);
		System::assert_last_event(Event::UtxoVerified { utxo_id: 1 }.into());
		assert!(UtxosPendingConfirmation::<Test>::get().is_empty(),);
		assert_eq!(
			LockedUtxos::<Test>::get(&utxo_ref),
			Some(UtxoValue {
				utxo_id: 1,
				script_pubkey,
				satoshis,
				submitted_at_height: 2,
				watch_for_spent_until_height: 10
			})
		);
		assert_eq!(LockedUtxoExpirationsByBlock::<Test>::get(10).to_vec(), vec![utxo_ref.clone()]);

		// test expiring
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 10,
			block_hash: H256Le([0; 32]),
		});
		// simulate next block
		InherentIncluded::<Test>::set(false);
		BitcoinUtxos::on_finalize(2);
		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				verified: Default::default(),
				invalid: Default::default(),
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 10, block_hash: H256Le([0; 32]) },
			},
		));
		System::assert_last_event(Event::UtxoUnwatched { utxo_id: 1 }.into());
		assert_eq!(LockedUtxos::<Test>::get(&utxo_ref), None);
		assert_eq!(UtxoIdToRef::<Test>::get(1), None);
		assert_eq!(LockedUtxoExpirationsByBlock::<Test>::get(10).to_vec(), vec![]);
	});
}

#[test]
fn it_should_block_duplicated_utxos() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});

		assert_ok!(BitcoinUtxos::watch_for_utxo(1, make_pubkey([0u8; 34]), 100, 100),);
		assert_ok!(BitcoinUtxos::watch_for_utxo(2, make_pubkey([1u8; 34]), 101, 100),);
		let utxo_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				verified: BTreeMap::from([(1, utxo_ref.clone()), (2, utxo_ref.clone())]),
				invalid: Default::default(),
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 1, block_hash: H256Le([0; 32]) },
			}
		),);
		System::assert_has_event(Event::UtxoVerified { utxo_id: 1 }.into());
		System::assert_has_event(
			Event::UtxoRejected {
				utxo_id: 2,
				rejected_reason: BitcoinRejectedReason::DuplicateUtxo,
			}
			.into(),
		);
		assert!(UtxosPendingConfirmation::<Test>::get().is_empty(),);
		assert_eq!(
			LockedUtxos::<Test>::get(&utxo_ref),
			Some(UtxoValue {
				utxo_id: 1,
				script_pubkey: make_pubkey([0u8; 34]),
				satoshis: 100,
				submitted_at_height: 2,
				watch_for_spent_until_height: 100,
			})
		);
		assert_eq!(LockedUtxoExpirationsByBlock::<Test>::get(100).to_vec(), vec![utxo_ref.clone()]);
		assert_eq!(UtxoIdToRef::<Test>::get(1), Some(utxo_ref.clone()));
		assert_eq!(UtxoIdToRef::<Test>::get(2), None);

		// test cleanup
		BitcoinUtxos::unwatch(1);
		assert_eq!(LockedUtxos::<Test>::get(&utxo_ref), None);
		assert_eq!(UtxoIdToRef::<Test>::get(1), None);
	});
}

#[test]
fn it_should_preserve_storage_if_one_sync_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});

		assert_ok!(BitcoinUtxos::watch_for_utxo(1, make_pubkey([0u8; 34]), 100, 100),);
		assert_ok!(BitcoinUtxos::watch_for_utxo(2, make_pubkey([1u8; 34]), 101, 100),);
		let utxo_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };
		let utxo_ref_2 = UtxoRef { txid: H256Le([1; 32]), output_index: 1 };
		UtxoVerifiedCallback::set(Some(|id| {
			if id == 2 {
				return Err(Error::<Test>::NoPermissions.into());
			}
			Ok(())
		}));

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				verified: BTreeMap::from([(1, utxo_ref.clone()), (2, utxo_ref_2.clone())]),
				invalid: Default::default(),
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 1, block_hash: H256Le([0; 32]) },
			}
		),);
		System::assert_has_event(Event::UtxoVerified { utxo_id: 1 }.into());
		let match_error: DispatchError = Error::<Test>::NoPermissions.into();
		System::assert_has_event(
			Event::UtxoVerifiedError { utxo_id: 2, error: match_error.stripped() }.into(),
		);

		assert_eq!(
			LockedUtxos::<Test>::get(&utxo_ref),
			Some(UtxoValue {
				utxo_id: 1,
				script_pubkey: make_pubkey([0u8; 34]),
				satoshis: 100,
				submitted_at_height: 2,
				watch_for_spent_until_height: 100,
			})
		);
		assert_eq!(LockedUtxoExpirationsByBlock::<Test>::get(100).to_vec(), vec![utxo_ref.clone()]);
		assert_eq!(
			UtxosPendingConfirmation::<Test>::get().get(&2),
			Some(&UtxoValue {
				utxo_id: 2,
				script_pubkey: make_pubkey([1u8; 34]),
				satoshis: 101,
				submitted_at_height: 2,
				watch_for_spent_until_height: 100,
			})
		);
	});
}

fn make_pubkey(pubkey: [u8; 34]) -> BitcoinCosignScriptPubkey {
	let mut hash = [0u8; 32];
	hash.copy_from_slice(&pubkey[0..32]);
	BitcoinCosignScriptPubkey::P2WSH { wscript_hash: H256::from(hash) }
}
