use crate::{
	Error, Event,
	mock::{ChainTransfer as ChainTransferPallet, *},
	pallet::{ExpiringTransfersOutByNotary, NextTransferId},
};
use argon_primitives::{
	NotebookEventHandler,
	notebook::{AccountOrigin, ChainTransfer, NotebookHeader},
	tick::Tick,
};
use frame_support::{assert_noop, assert_ok};
use pallet_prelude::*;
use sp_core::bounded_vec;
use sp_keyring::Sr25519Keyring::Bob;
use sp_runtime::testing::H256;

#[test]
fn it_can_send_funds_to_localchain() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		set_argons(&who, 5000);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			1000,
			1,
		));
		assert_eq!(Balances::free_balance(&who), 4000);
		let expires_tick: Tick = 1 + TransferExpirationTicks::get();
		assert_eq!(ExpiringTransfersOutByNotary::<Test>::get(1, expires_tick)[0], 1);
		assert_eq!(NextTransferId::<Test>::get(), Some(2));
	});
}

#[test]
fn it_allows_you_to_transfer_full_balance() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		System::inc_account_nonce(&who);
		set_argons(&who, 5000);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			5000,
			1,
		));
		assert_eq!(Balances::free_balance(&who), 0);
		assert!(!System::account_exists(&who));
	});
}

#[test]
fn it_expires_transfers_on_notebook_tick() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		set_argons(&who, 2000);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			2000,
			1,
		));
		assert_eq!(Balances::free_balance(&who), 0);
		assert!(!System::account_exists(&who));
		let expires_tick: Tick = 1 + TransferExpirationTicks::get();
		assert_eq!(ExpiringTransfersOutByNotary::<Test>::get(1, expires_tick)[0], 1);

		ChainTransferPallet::notebook_submitted(&NotebookHeader {
			notary_id: 1,
			notebook_number: 10,
			tick: expires_tick,
			chain_transfers: Default::default(),
			changed_accounts_root: Default::default(),
			changed_account_origins: Default::default(),
			version: 1,
			tax: 0,
			block_voting_power: 0,
			blocks_with_votes: Default::default(),
			block_votes_root: H256::random(),
			secret_hash: H256::random(),

			parent_secret: None,
			block_votes_count: 0,
			domains: Default::default(),
		});
		assert!(System::account_exists(&who));
		assert_eq!(Balances::free_balance(&who), 2000);
	});
}

#[test]
fn it_can_handle_multiple_transfer() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		MaxPendingTransfersOutPerBlock::set(2);
		System::set_block_number(1);
		set_argons(&who, 5000);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			1000,
			1,
		));
		System::inc_account_nonce(&who);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			700,
			1,
		),);
		assert_eq!(Balances::free_balance(&who), 3300);
		let expires_tick: Tick = 1 + TransferExpirationTicks::get();
		assert_eq!(ExpiringTransfersOutByNotary::<Test>::get(1, expires_tick), vec![1, 2]);

		System::inc_account_nonce(&who);
		// We have a max number of transfers out per block
		assert_noop!(
			ChainTransferPallet::send_to_localchain(RuntimeOrigin::signed(who.clone()), 1200, 1,),
			Error::<Test>::MaxBlockTransfersExceeded
		);
	});
}

#[test]
fn it_can_handle_transfers_in() {
	MaxNotebookBlocksToRemember::set(2);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let who = Bob.to_account_id();
		set_argons(&who, 5000);
		assert_ok!(ChainTransferPallet::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			5000,
			1,
		));
		let expires_tick: Tick = 1 + TransferExpirationTicks::get();
		assert_eq!(ExpiringTransfersOutByNotary::<Test>::get(1, expires_tick)[0], 1);

		let changed_accounts_root = H256::random();
		ChainTransferPallet::notebook_submitted(&NotebookHeader {
			notary_id: 1,
			notebook_number: 1,
			tick: 1,
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain { transfer_id: 1 }],
			changed_accounts_root,
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			version: 1,
			tax: 0,
			block_voting_power: 0,
			blocks_with_votes: Default::default(),
			block_votes_root: H256::random(),
			secret_hash: H256::random(),

			parent_secret: None,
			block_votes_count: 0,
			domains: Default::default(),
		});

		assert_eq!(ExpiringTransfersOutByNotary::<Test>::get(1, expires_tick).len(), 0);
		assert_eq!(Balances::free_balance(&who), 0);

		let change_root_2 = H256::random();
		ChainTransferPallet::notebook_submitted(&NotebookHeader {
			notary_id: 1,
			notebook_number: 2,
			tick: 2,
			chain_transfers: bounded_vec![ChainTransfer::ToMainchain {
				account_id: who.clone(),
				amount: 5000
			}],
			changed_accounts_root: change_root_2,
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			version: 1,
			tax: 0,
			block_voting_power: 0,
			blocks_with_votes: Default::default(),
			block_votes_root: H256::random(),
			secret_hash: H256::random(),
			parent_secret: None,
			block_votes_count: 0,
			domains: Default::default(),
		});
		assert_eq!(Balances::free_balance(&who), 5000);
	});
}

#[test]
fn it_reduces_circulation_on_tax() {
	MaxNotebookBlocksToRemember::set(2);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let who = ChainTransferPallet::notary_account_id(1);
		set_argons(&who, 25000);
		assert_eq!(Balances::total_issuance(), 25_000);

		ChainTransferPallet::notebook_submitted(&NotebookHeader {
			notary_id: 1,
			notebook_number: 1,
			tick: 1,
			chain_transfers: bounded_vec![],
			changed_accounts_root: H256::random(),
			changed_account_origins: bounded_vec![],
			version: 1,
			tax: 2000,
			block_voting_power: 0,
			blocks_with_votes: Default::default(),
			block_votes_root: H256::random(),
			secret_hash: H256::random(),
			parent_secret: None,
			block_votes_count: 0,
			domains: Default::default(),
		});

		assert_eq!(Balances::total_issuance(), 23_000);
		assert_eq!(Balances::free_balance(&who), 23_000);
	})
}

#[test]
fn it_doesnt_allow_a_notary_balance_to_go_negative() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);

		ChainTransferPallet::notebook_submitted(&NotebookHeader {
			notary_id: 1,
			notebook_number: 1,
			tick: 0,
			chain_transfers: bounded_vec![ChainTransfer::ToMainchain {
				account_id: Bob.to_account_id(),
				amount: 5000
			}],
			changed_accounts_root: H256::random(),
			changed_account_origins: bounded_vec![],
			version: 1,
			tax: 0,
			block_voting_power: 0,
			blocks_with_votes: Default::default(),
			block_votes_root: H256::random(),
			secret_hash: H256::random(),
			parent_secret: None,
			block_votes_count: 0,
			domains: Default::default(),
		});
		System::assert_last_event(
			Event::<Test>::TransferFromLocalchainError {
				notary_id: 1,
				notebook_number: 1,
				amount: 5000,
				account_id: Bob.to_account_id(),
				error: Error::<Test>::InsufficientNotarizedFunds.into(),
			}
			.into(),
		);
	});
}
