use crate::{
	Error, Event,
	mock::{System, *},
	pallet::{
		CandidateUtxoRefsByUtxoId, ConfirmedBitcoinBlockTip, InherentIncluded, LockedUtxos,
		LocksPendingFunding, UtxoIdToFundingUtxoRef,
	},
};
use argon_primitives::{
	BitcoinUtxoTracker,
	bitcoin::{
		BitcoinBlock, BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinRejectedReason, H256Le,
		Satoshis, UtxoId, UtxoRef, UtxoValue,
	},
	inherents::{BitcoinUtxoFunding, BitcoinUtxoSpend, BitcoinUtxoSync},
};
use frame_support::{assert_err, assert_noop, assert_ok, pallet_prelude::Hooks};
use pallet_prelude::*;
use sp_core::H256;
use sp_runtime::{DispatchError, DispatchResult};

#[test]
fn only_an_operator_can_submit_oracle_block() {
	new_test_ext().execute_with(|| {
		let who = 1;
		System::set_block_number(1);
		assert!(!BitcoinUtxos::has_new_bitcoin_tip());
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
		assert!(BitcoinUtxos::has_new_bitcoin_tip());
		assert_eq!(
			ConfirmedBitcoinBlockTip::<Test>::get(),
			Some(BitcoinBlock { block_height: 1, block_hash: H256Le([0; 32]) })
		);
		BitcoinUtxos::on_finalize(1);
		System::initialize(&2, &System::parent_hash(), &Default::default());
		BitcoinUtxos::on_initialize(2);
		assert!(!BitcoinUtxos::has_new_bitcoin_tip());
		assert_ok!(BitcoinUtxos::set_confirmed_block(
			RuntimeOrigin::signed(who),
			2,
			H256Le([2; 32])
		),);
		assert!(BitcoinUtxos::has_new_bitcoin_tip());
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
			*LocksPendingFunding::<Test>::get().get(&1).unwrap(),
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
				funded: Default::default(),
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 10, block_hash: H256Le([0; 32]) },
			},
		));
		System::assert_last_event(Event::UtxoUnwatched { utxo_id: 1 }.into());
		assert!(LocksPendingFunding::<Test>::get().is_empty());
	});
}

#[test]
fn it_requires_block_sync_to_be_newer() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let mut utxo_sync = BitcoinUtxoSync {
			funded: Default::default(),
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
		let satoshis = MinimumSatoshisPerCandidateUtxo::get();
		let watch_for_spent_until = 10;
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script_pubkey, satoshis, watch_for_spent_until),);
		let utxo_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };

		let utxo_sync = BitcoinUtxoSync {
			funded: vec![BitcoinUtxoFunding {
				utxo_id: 1,
				utxo_ref: utxo_ref.clone(),
				satoshis,
				expected_satoshis: satoshis,
				bitcoin_height: 2,
			}],
			spent: Default::default(),
			sync_to_block: BitcoinBlock { block_height: 1, block_hash: H256Le([0; 32]) },
		};
		assert_ok!(BitcoinUtxos::sync(RuntimeOrigin::none(), utxo_sync),);
		System::assert_last_event(
			Event::UtxoVerified { utxo_id: 1, satoshis_received: satoshis }.into(),
		);
		assert!(LocksPendingFunding::<Test>::get().is_empty(),);
		assert!(CandidateUtxoRefsByUtxoId::<Test>::get(1).is_empty());
		assert_eq!(UtxoIdToFundingUtxoRef::<Test>::get(1), Some(utxo_ref.clone()));
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
	});
}

/// Records mismatched UTXOs while pending, but clears candidate refs once funding is confirmed.
#[test]
fn it_should_ignore_extra_funding_utxos() {
	MinimumSatoshisPerCandidateUtxo::set(1000);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		OrphanDetectedCallback::set(Some(record_orphan_detected));
		LastOrphanDetected::set(None);
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});

		// first one is under minimum
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, make_pubkey([0u8; 34]), 100_000, 100),);
		let utxo_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };
		let utxo_ref_2 = UtxoRef { txid: H256Le([1; 32]), output_index: 1 };
		let utxo_ref_3 = UtxoRef { txid: H256Le([2; 32]), output_index: 1 };
		assert_eq!(LocksPendingFunding::<Test>::get().len(), 1);

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				funded: vec![
					BitcoinUtxoFunding {
						utxo_id: 1,
						utxo_ref: utxo_ref.clone(),
						satoshis: 999,
						expected_satoshis: 100_000,
						bitcoin_height: 2,
					},
					BitcoinUtxoFunding {
						utxo_id: 1,
						utxo_ref: utxo_ref_2.clone(),
						satoshis: 89_999, // outside amount threshold,
						expected_satoshis: 100_000,
						bitcoin_height: 2,
					},
					BitcoinUtxoFunding {
						utxo_id: 1,
						utxo_ref: utxo_ref_3.clone(),
						satoshis: 110_000,
						expected_satoshis: 100_000,
						bitcoin_height: 2,
					}
				],
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 1, block_hash: H256Le([0; 32]) },
			}
		),);
		System::assert_has_event(
			Event::UtxoVerified { utxo_id: 1, satoshis_received: 110_000 }.into(),
		);
		// we will just ignore the < 1000 utxo
		System::assert_has_event(
			Event::UtxoRejected {
				utxo_id: 1,
				utxo_ref: utxo_ref_2.clone(),
				satoshis_received: 89_999,
				rejected_reason: BitcoinRejectedReason::SatoshisOutsideAcceptedRange,
			}
			.into(),
		);
		assert!(CandidateUtxoRefsByUtxoId::<Test>::get(1).is_empty());
		assert!(LocksPendingFunding::<Test>::get().is_empty(),);
		assert_eq!(LastOrphanDetected::get(), Some((1, utxo_ref_2.clone(), 89_999)));
		assert_eq!(LockedUtxos::<Test>::get(&utxo_ref), None);
		assert_eq!(LockedUtxos::<Test>::get(&utxo_ref_2), None);
		assert_eq!(
			LockedUtxos::<Test>::get(&utxo_ref_3),
			Some(UtxoValue {
				utxo_id: 1,
				script_pubkey: make_pubkey([0u8; 34]),
				satoshis: 100_000, // doesn't update
				submitted_at_height: 2,
				watch_for_spent_until_height: 100,
			})
		);
		assert_eq!(UtxoIdToFundingUtxoRef::<Test>::get(1), Some(utxo_ref_3.clone()));

		// test cleanup
		BitcoinUtxos::unwatch(1);
		assert_eq!(LockedUtxos::<Test>::get(&utxo_ref_3), None);
		assert_eq!(UtxoIdToFundingUtxoRef::<Test>::get(1), None);
		OrphanDetectedCallback::set(None);
	});
}

#[test]
fn it_should_preserve_storage_if_one_sync_fails() {
	MinimumSatoshisPerCandidateUtxo::set(100);
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
		UtxoVerifiedCallback::set(Some(|(id, _satoshis)| {
			if id == 2 {
				return Err(Error::<Test>::NoPermissions.into());
			}
			Ok(())
		}));

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				funded: vec![
					BitcoinUtxoFunding {
						utxo_id: 1,
						utxo_ref: utxo_ref.clone(),
						satoshis: 100,
						expected_satoshis: 100,
						bitcoin_height: 2,
					},
					BitcoinUtxoFunding {
						utxo_id: 2,
						utxo_ref: utxo_ref_2.clone(),
						satoshis: 101,
						expected_satoshis: 101,
						bitcoin_height: 2,
					}
				],
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 1, block_hash: H256Le([0; 32]) },
			}
		),);
		System::assert_has_event(Event::UtxoVerified { utxo_id: 1, satoshis_received: 100 }.into());
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
		assert_eq!(
			LocksPendingFunding::<Test>::get().get(&2),
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

/// Does not track extra UTXOs after a lock is funded as candidates.
#[test]
fn does_not_track_utxos_after_funding() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		OrphanDetectedCallback::set(Some(record_orphan_detected));
		LastOrphanDetected::set(None);

		let script_pubkey = make_pubkey([0u8; 34]);
		let satoshis = MinimumSatoshisPerCandidateUtxo::get();
		let watch_for_spent_until = 10;
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script_pubkey, satoshis, watch_for_spent_until),);
		let funded_ref = UtxoRef { txid: H256Le([0; 32]), output_index: 0 };

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				funded: vec![BitcoinUtxoFunding {
					utxo_id: 1,
					utxo_ref: funded_ref.clone(),
					satoshis,
					expected_satoshis: satoshis,
					bitcoin_height: 2,
				}],
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 2, block_hash: H256Le([0; 32]) },
			},
		));
		assert!(LocksPendingFunding::<Test>::get().is_empty());
		assert_eq!(UtxoIdToFundingUtxoRef::<Test>::get(1), Some(funded_ref.clone()));

		InherentIncluded::<Test>::set(false);
		let extra_ref = UtxoRef { txid: H256Le([1; 32]), output_index: 1 };
		System::set_block_number(2);
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 3,
			block_hash: H256Le([1; 32]),
		});
		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				funded: vec![BitcoinUtxoFunding {
					utxo_id: 1,
					utxo_ref: extra_ref.clone(),
					satoshis: satoshis + 10_000,
					expected_satoshis: satoshis,
					bitcoin_height: 3,
				}],
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 3, block_hash: H256Le([1; 32]) },
			},
		));

		assert!(CandidateUtxoRefsByUtxoId::<Test>::get(1).is_empty());
		assert_eq!(LastOrphanDetected::get(), Some((1, extra_ref.clone(), satoshis + 10_000)));

		OrphanDetectedCallback::set(None);
	});
}

/// Promotes only non-selected candidates to orphans when a user funds with a candidate.
#[test]
fn promotes_non_selected_candidate_on_manual_funding() {
	MinimumSatoshisPerCandidateUtxo::set(1000);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		OrphanDetectedCallback::set(Some(record_orphan_detected));
		LastOrphanDetected::set(None);
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});

		let script_pubkey = make_pubkey([0u8; 34]);
		let expected_satoshis = 100_000;
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script_pubkey, expected_satoshis, 100),);

		let chosen_ref = UtxoRef { txid: H256Le([1; 32]), output_index: 0 };
		let other_ref = UtxoRef { txid: H256Le([2; 32]), output_index: 1 };
		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				funded: vec![
					BitcoinUtxoFunding {
						utxo_id: 1,
						utxo_ref: chosen_ref.clone(),
						satoshis: expected_satoshis - 10_001,
						expected_satoshis,
						bitcoin_height: 2,
					},
					BitcoinUtxoFunding {
						utxo_id: 1,
						utxo_ref: other_ref.clone(),
						satoshis: expected_satoshis - 12_000,
						expected_satoshis,
						bitcoin_height: 2,
					},
				],
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 2, block_hash: H256Le([0; 32]) },
			},
		));
		assert!(CandidateUtxoRefsByUtxoId::<Test>::get(1).contains_key(&chosen_ref));
		assert!(CandidateUtxoRefsByUtxoId::<Test>::get(1).contains_key(&other_ref));

		assert_ok!(BitcoinUtxos::fund_with_utxo_candidate(
			RuntimeOrigin::signed(1),
			1,
			chosen_ref.clone()
		));

		assert_eq!(
			LastOrphanDetected::get(),
			Some((1, other_ref.clone(), expected_satoshis - 12_000))
		);
		assert!(LocksPendingFunding::<Test>::get().is_empty());
		assert!(CandidateUtxoRefsByUtxoId::<Test>::get(1).is_empty());
		assert_eq!(UtxoIdToFundingUtxoRef::<Test>::get(1), Some(chosen_ref.clone()));
		assert!(LockedUtxos::<Test>::get(&chosen_ref).is_some());

		OrphanDetectedCallback::set(None);
	});
}

/// Promotes candidate UTXOs to orphan entries when a lock times out.
#[test]
fn promotes_candidates_to_orphans_on_timeout() {
	MinimumSatoshisPerCandidateUtxo::set(1000);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		OrphanDetectedCallback::set(Some(record_orphan_detected));
		LastOrphanDetected::set(None);
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});

		let script_pubkey = make_pubkey([0u8; 34]);
		let expected_satoshis = 100_000;
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script_pubkey, expected_satoshis, 100),);

		let orphan_ref = UtxoRef { txid: H256Le([9; 32]), output_index: 0 };
		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				funded: vec![BitcoinUtxoFunding {
					utxo_id: 1,
					utxo_ref: orphan_ref.clone(),
					satoshis: expected_satoshis - 10_001,
					expected_satoshis,
					bitcoin_height: 2,
				}],
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 2, block_hash: H256Le([0; 32]) },
			},
		));
		assert!(CandidateUtxoRefsByUtxoId::<Test>::get(1).contains_key(&orphan_ref));

		assert_ok!(BitcoinUtxos::lock_timeout_pending_funding(1));
		assert!(!LocksPendingFunding::<Test>::get().contains_key(&1));
		assert_eq!(
			LastOrphanDetected::get(),
			Some((1, orphan_ref.clone(), expected_satoshis - 10_001))
		);
		assert!(CandidateUtxoRefsByUtxoId::<Test>::get(1).is_empty());
	});
}

/// Clears candidates and pending funding when a tracked UTXO is reported spent.
#[test]
fn spent_clears_candidates_and_pending() {
	MinimumSatoshisPerCandidateUtxo::set(1000);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});

		let script_pubkey = make_pubkey([0u8; 34]);
		let expected_satoshis = 100_000;
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script_pubkey, expected_satoshis, 100),);

		let candidate_ref = UtxoRef { txid: H256Le([9; 32]), output_index: 0 };
		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				funded: vec![BitcoinUtxoFunding {
					utxo_id: 1,
					utxo_ref: candidate_ref.clone(),
					satoshis: expected_satoshis - 10_001,
					expected_satoshis,
					bitcoin_height: 2,
				}],
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 2, block_hash: H256Le([0; 32]) },
			},
		));
		assert!(CandidateUtxoRefsByUtxoId::<Test>::get(1).contains_key(&candidate_ref));

		InherentIncluded::<Test>::set(false);
		System::set_block_number(2);
		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				funded: Default::default(),
				spent: vec![BitcoinUtxoSpend {
					utxo_id: 1,
					utxo_ref: Some(candidate_ref.clone()),
					bitcoin_height: 2,
				}],
				sync_to_block: BitcoinBlock { block_height: 2, block_hash: H256Le([0; 32]) },
			},
		));

		assert!(CandidateUtxoRefsByUtxoId::<Test>::get(1).is_empty());
		assert!(LocksPendingFunding::<Test>::get().is_empty());
		assert_eq!(UtxoIdToFundingUtxoRef::<Test>::get(1), None);
		assert!(LockedUtxos::<Test>::get(&candidate_ref).is_none());
	});
}

/// Ignores duplicate candidate UTXOs for the same lock.
#[test]
fn ignores_duplicate_candidates() {
	MinimumSatoshisPerCandidateUtxo::set(1000);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});

		let script_pubkey = make_pubkey([0u8; 34]);
		let expected_satoshis = 100_000;
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script_pubkey, expected_satoshis, 100),);

		let duplicate_ref = UtxoRef { txid: H256Le([1; 32]), output_index: 0 };
		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				funded: vec![
					BitcoinUtxoFunding {
						utxo_id: 1,
						utxo_ref: duplicate_ref.clone(),
						satoshis: expected_satoshis - 10_001,
						expected_satoshis,
						bitcoin_height: 2,
					},
					BitcoinUtxoFunding {
						utxo_id: 1,
						utxo_ref: duplicate_ref.clone(),
						satoshis: expected_satoshis - 10_001,
						expected_satoshis,
						bitcoin_height: 2,
					},
				],
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 2, block_hash: H256Le([0; 32]) },
			},
		));

		assert_eq!(CandidateUtxoRefsByUtxoId::<Test>::get(1).len(), 1);
		assert!(CandidateUtxoRefsByUtxoId::<Test>::get(1).contains_key(&duplicate_ref));
	});
}

/// Rejects candidate storage when the max candidate count is exceeded.
#[test]
fn rejects_when_candidate_limit_exceeded() {
	MinimumSatoshisPerCandidateUtxo::set(1000);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ConfirmedBitcoinBlockTip::<Test>::put(BitcoinBlock {
			block_height: 2,
			block_hash: H256Le([0; 32]),
		});

		let script_pubkey = make_pubkey([0u8; 34]);
		let expected_satoshis = 100_000;
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script_pubkey, expected_satoshis, 100),);

		let mut funded = Vec::new();
		for i in 0..(MaxCandidateUtxosPerLock::get() + 1) {
			funded.push(BitcoinUtxoFunding {
				utxo_id: 1,
				utxo_ref: UtxoRef { txid: H256Le([i as u8; 32]), output_index: 0 },
				satoshis: expected_satoshis - 10_001,
				expected_satoshis,
				bitcoin_height: 2,
			});
		}

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			BitcoinUtxoSync {
				funded,
				spent: Default::default(),
				sync_to_block: BitcoinBlock { block_height: 2, block_hash: H256Le([0; 32]) },
			},
		));

		let error: DispatchError = Error::<Test>::MaxCandidateUtxosExceeded.into();
		System::assert_has_event(
			Event::UtxoVerifiedError { utxo_id: 1, error: error.stripped() }.into(),
		);
		assert_eq!(
			CandidateUtxoRefsByUtxoId::<Test>::get(1).len() as u32,
			MaxCandidateUtxosPerLock::get()
		);
	});
}

fn record_orphan_detected(input: (UtxoId, UtxoRef, Satoshis)) -> DispatchResult {
	LastOrphanDetected::set(Some(input));
	Ok(())
}

fn make_pubkey(pubkey: [u8; 34]) -> BitcoinCosignScriptPubkey {
	let mut hash = [0u8; 32];
	hash.copy_from_slice(&pubkey[0..32]);
	BitcoinCosignScriptPubkey::P2WSH { wscript_hash: H256::from(hash) }
}
