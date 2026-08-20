use crate::{
	mock::{System, *},
	pallet::{ConfirmedBitcoinBlockTip, InherentIncluded},
	Error, Event, UtxoAddressByUtxoId, UtxoRefsByUtxoId,
};
use pallet_prelude::{
	argon_primitives::{
		bitcoin::{BitcoinBlock, BitcoinCosignScriptPubkey, H256Le, UtxoAddress, UtxoId, UtxoRef},
		inherents::{BitcoinUtxoFunding, BitcoinUtxoSpend, BitcoinUtxoSync},
		BitcoinUtxoTracker,
	},
	*,
};

#[test]
fn watches_a_lock_address_until_explicitly_unwatched() {
	new_test_ext().execute_with(|| {
		ConfirmedBitcoinBlockTip::<Test>::put(block(1));
		let script = script([1; 34]);

		assert_ok!(BitcoinUtxos::watch_for_utxo(1, script));
		assert_eq!(
			UtxoAddressByUtxoId::<Test>::get(1),
			Some(UtxoAddress { utxo_id: 1, script_pubkey: script, submitted_at_height: 1 })
		);
		assert_noop!(BitcoinUtxos::watch_for_utxo(2, script), Error::<Test>::ScriptPubkeyConflict);

		ConfirmedBitcoinBlockTip::<Test>::put(block(500));
		BitcoinUtxos::on_initialize(2);
		assert!(UtxoAddressByUtxoId::<Test>::contains_key(1));

		BitcoinUtxos::unwatch(1);
		assert!(!UtxoAddressByUtxoId::<Test>::contains_key(1));
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

		let refs = UtxoRefsByUtxoId::<Test>::get(1);
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
				vec![BitcoinUtxoSpend {
					utxo_id: 1,
					utxo_ref: Some(first.clone()),
					bitcoin_height: 3,
				}],
			),
		));

		let refs = UtxoRefsByUtxoId::<Test>::get(1);
		assert!(!refs.contains(&first));
		assert!(refs.contains(&second));
		assert!(UtxoAddressByUtxoId::<Test>::contains_key(1));
		assert_eq!(LastSpent::get(), Some((1, first.clone())));
		System::assert_last_event(
			Event::UtxoSpent { utxo_id: 1, utxo_ref: first, block_height: 3 }.into(),
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

		assert!(!UtxoRefsByUtxoId::<Test>::get(1).contains(&utxo_ref));
		System::assert_last_event(
			Event::UtxoDetectedError { utxo_id: 1, error: DispatchError::Other("") }.into(),
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

		assert_eq!(UtxoRefsByUtxoId::<Test>::get(1).len(), 1);
	});
}

fn select_funding(_: (UtxoId, UtxoRef, u64)) -> DispatchResult {
	Ok(())
}

fn fail_detection(_: (UtxoId, UtxoRef, u64)) -> DispatchResult {
	Err(DispatchError::Other("failed"))
}

fn sync(
	height: u64,
	funded: Vec<BitcoinUtxoFunding>,
	spent: Vec<BitcoinUtxoSpend>,
) -> BitcoinUtxoSync {
	BitcoinUtxoSync { funded, spent, sync_to_block: block(height) }
}

fn funding(utxo_id: UtxoId, utxo_ref: UtxoRef, satoshis: u64, height: u64) -> BitcoinUtxoFunding {
	BitcoinUtxoFunding { utxo_id, utxo_ref, satoshis, expected_satoshis: 0, bitcoin_height: height }
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
