use crate::{
	mock::{System, *},
	pallet::{ConfirmedBitcoinBlockTip, InherentIncluded},
	Error, Event, UtxoAddressByLockId, UtxoRefsByLockId,
};
use frame_support::inherent::{InherentData, ProvideInherent};
use pallet_prelude::{
	argon_primitives::{
		bitcoin::{
			BitcoinBlock, BitcoinCosignScriptPubkey, BitcoinLockId, H256Le, UtxoAddress, UtxoRef,
		},
		inherents::{
			BitcoinUtxoFunding, BitcoinUtxoSpend, BitcoinUtxoSpendV2, BitcoinUtxoSync,
			BitcoinUtxoSyncV2, BITCOIN_INHERENT_IDENTIFIER, BITCOIN_INHERENT_IDENTIFIER_V2,
		},
		BitcoinUtxoTracker,
	},
	*,
};

#[test]
fn creates_inherent_from_data_provided_by_pre_upgrade_nodes() {
	let mut inherent_data = InherentData::new();
	inherent_data
		.put_data(
			BITCOIN_INHERENT_IDENTIFIER_V2,
			&BitcoinUtxoSyncV2 { spent: vec![], funded: vec![], sync_to_block: block(1) },
		)
		.expect("valid legacy inherent data");

	assert!(BitcoinUtxos::create_inherent(&inherent_data).is_some());
}

#[test]
fn converts_current_inherent_data_to_the_existing_sync_call() {
	let mut inherent_data = InherentData::new();
	let utxo_ref = utxo_ref(7);
	let sync = BitcoinUtxoSync {
		spent: vec![BitcoinUtxoSpend {
			lock_id: 1,
			utxo_ref: Some(utxo_ref.clone()),
			bitcoin_height: 2,
			spending_txid: H256Le([9; 32]),
		}],
		funded: vec![],
		sync_to_block: block(2),
	};
	let expected_sync: BitcoinUtxoSyncV2 = sync.clone().into();
	inherent_data
		.put_data(BITCOIN_INHERENT_IDENTIFIER, &sync)
		.expect("valid current inherent data");

	let call =
		BitcoinUtxos::create_inherent(&inherent_data).expect("current data creates inherent");
	assert_eq!(call, crate::Call::sync { utxo_sync: expected_sync });
}

#[test]
fn watches_a_lock_address_until_explicitly_unwatched() {
	new_test_ext().execute_with(|| {
		ConfirmedBitcoinBlockTip::<Test>::put(block(1));
		let script = script([1; 34]);

		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script));
		assert_eq!(
			UtxoAddressByLockId::<Test>::get(1),
			Some(UtxoAddress { lock_id: 1, script_pubkey: script, submitted_at_height: 1 })
		);
		assert_noop!(BitcoinUtxos::watch_for_utxo(2, script), Error::<Test>::ScriptPubkeyConflict);

		ConfirmedBitcoinBlockTip::<Test>::put(block(500));
		BitcoinUtxos::on_initialize(2);
		assert!(UtxoAddressByLockId::<Test>::contains_key(1));

		BitcoinUtxos::unwatch(1);
		assert!(!UtxoAddressByLockId::<Test>::contains_key(1));
	});
}

#[test]
fn attaches_every_output_without_classifying_it() {
	MinimumSatoshisPerUtxo::set(1);
	new_test_ext().execute_with(|| {
		ConfirmedBitcoinBlockTip::<Test>::put(block(10));
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script([1; 34])));
		let first = utxo_ref(1);
		let second = utxo_ref(2);

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			sync(
				10,
				vec![funding(1, first.clone(), 90, 2), funding(1, second.clone(), 110, 9)],
				vec![],
			),
		));

		let refs = UtxoRefsByLockId::<Test>::get(1);
		assert!(refs.contains(&first));
		assert!(refs.contains(&second));
		assert_eq!(BitcoinUtxos::active_utxos().len(), 2);
	});
}

#[test]
fn consumer_can_select_the_funding_output() {
	MinimumSatoshisPerUtxo::set(1);
	new_test_ext().execute_with(|| {
		ConfirmedBitcoinBlockTip::<Test>::put(block(2));
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script([1; 34])));
		UtxoDetectedCallback::set(Some(select_funding));
		let funding_ref = utxo_ref(1);

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			sync(2, vec![funding(1, funding_ref.clone(), 100, 2)], vec![]),
		));
	});
}

#[test]
fn spends_remove_only_the_exact_attached_output() {
	MinimumSatoshisPerUtxo::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ConfirmedBitcoinBlockTip::<Test>::put(block(3));
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script([1; 34])));
		let first = utxo_ref(1);
		let second = utxo_ref(2);
		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			sync(
				2,
				vec![funding(1, first.clone(), 100, 1), funding(1, second.clone(), 50, 2)],
				vec![],
			),
		));
		InherentIncluded::<Test>::set(false);

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			sync(
				3,
				vec![],
				vec![BitcoinUtxoSpendV2 {
					lock_id: 1,
					utxo_ref: Some(first.clone()),
					bitcoin_height: 3,
				}],
			),
		));

		let refs = UtxoRefsByLockId::<Test>::get(1);
		assert!(!refs.contains(&first));
		assert!(refs.contains(&second));
		assert!(UtxoAddressByLockId::<Test>::contains_key(1));
		assert_eq!(LastSpent::get(), Some((1, first.clone(), 3)));
		System::assert_last_event(
			Event::UtxoSpent { lock_id: 1, utxo_ref: first, block_height: 3 }.into(),
		);
	});
}

#[test]
fn callback_failure_rolls_back_the_attachment() {
	MinimumSatoshisPerUtxo::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ConfirmedBitcoinBlockTip::<Test>::put(block(2));
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script([1; 34])));
		UtxoDetectedCallback::set(Some(fail_detection));
		let utxo_ref = utxo_ref(1);

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			sync(2, vec![funding(1, utxo_ref.clone(), 100, 2)], vec![]),
		));

		assert!(!UtxoRefsByLockId::<Test>::get(1).contains(&utxo_ref));
		System::assert_last_event(
			Event::UtxoDetectedError { lock_id: 1, error: DispatchError::Other("") }.into(),
		);
	});
}

#[test]
fn duplicate_reports_do_not_duplicate_attachments() {
	MinimumSatoshisPerUtxo::set(1);
	new_test_ext().execute_with(|| {
		ConfirmedBitcoinBlockTip::<Test>::put(block(2));
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script([1; 34])));
		let utxo_ref = utxo_ref(1);
		let report = funding(1, utxo_ref.clone(), 100, 2);

		assert_ok!(BitcoinUtxos::sync(
			RuntimeOrigin::none(),
			sync(2, vec![report.clone(), report], vec![]),
		));

		assert_eq!(UtxoRefsByLockId::<Test>::get(1).len(), 1);
	});
}

#[test]
fn outputs_over_the_tracking_limit_still_reach_the_consumer() {
	MinimumSatoshisPerUtxo::set(1);
	UtxoDetectionCount::set(0);
	new_test_ext().execute_with(|| {
		ConfirmedBitcoinBlockTip::<Test>::put(block(11));
		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script([1; 34])));
		let funded = (1..=11).map(|value| funding(1, utxo_ref(value), 100, 11)).collect();

		assert_ok!(BitcoinUtxos::sync(RuntimeOrigin::none(), sync(11, funded, vec![])));

		assert_eq!(UtxoDetectionCount::get(), 11);
		assert_eq!(UtxoRefsByLockId::<Test>::get(1).len(), 10);
	});
}

fn select_funding(_: (BitcoinLockId, UtxoRef, u64)) -> DispatchResult {
	Ok(())
}

fn fail_detection(_: (BitcoinLockId, UtxoRef, u64)) -> DispatchResult {
	Err(DispatchError::Other("failed"))
}

fn sync(
	height: u64,
	funded: Vec<BitcoinUtxoFunding>,
	spent: Vec<BitcoinUtxoSpendV2>,
) -> BitcoinUtxoSyncV2 {
	BitcoinUtxoSyncV2 { funded, spent, sync_to_block: block(height) }
}

fn funding(
	lock_id: BitcoinLockId,
	utxo_ref: UtxoRef,
	satoshis: u64,
	height: u64,
) -> BitcoinUtxoFunding {
	BitcoinUtxoFunding { lock_id, utxo_ref, satoshis, expected_satoshis: 0, bitcoin_height: height }
}

fn block(height: u64) -> BitcoinBlock {
	BitcoinBlock { block_height: height, block_hash: H256Le([height as u8; 32]) }
}

fn utxo_ref(value: u8) -> UtxoRef {
	UtxoRef { txid: H256Le([value; 32]), output_index: 0 }
}

fn script(value: [u8; 34]) -> BitcoinCosignScriptPubkey {
	let mut hash = [0; 32];
	hash.copy_from_slice(&value[..32]);
	BitcoinCosignScriptPubkey::P2WSH { wscript_hash: sp_core::H256::from(hash) }
}
