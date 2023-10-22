use std::collections::BTreeMap;

use codec::Encode;
use frame_support::{assert_err, assert_noop, assert_ok, traits::OnInitialize};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_core::{blake2_256, ed25519, Pair};
use sp_runtime::{
	testing::{UintAuthorityId, H256},
	traits::ValidateUnsigned,
	transaction_validity::{InvalidTransaction, TransactionSource},
	BoundedVec, DigestItem,
};
type Hash = H256;

use ulx_primitives::{
	block_seal::BlockSealAuthorityPair,
	digests::{FinalizedBlockNeededDigest, FINALIZED_BLOCK_DIGEST_ID},
	notebook::{to_notebook_audit_signature_message, ChainTransfer, Notebook},
	BlockSealAuthorityId, BlockSealAuthoritySignature,
};

use crate::{
	mock::{LocalchainRelay, *},
	pallet::{
		ExpiringTransfersOut, FinalizedBlockNumber, PendingTransfersOut,
		SubmittedNotebookBlocksByNotaryId,
	},
	Error, QueuedTransferOut,
};

#[test]
fn it_can_send_funds_to_localchain() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let nonce = System::account_nonce(1);
		System::inc_account_nonce(1);
		set_argons(1, 5000);
		assert_ok!(LocalchainRelay::send_to_localchain(RuntimeOrigin::signed(1), 1000, 1, nonce));
		assert_eq!(Balances::free_balance(1), 4000);
		let expires_block: BlockNumberFor<Test> = (1u32 + TransferExpirationBlocks::get()).into();
		assert_eq!(ExpiringTransfersOut::<Test>::get(expires_block)[0], (1, nonce));
	});
}

#[test]
fn it_allows_you_to_transfer_full_balance() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let nonce = System::account_nonce(1);
		System::inc_account_nonce(1);
		set_argons(1, 5000);
		assert_ok!(LocalchainRelay::send_to_localchain(RuntimeOrigin::signed(1), 5000, 1, nonce));
		assert_eq!(Balances::free_balance(1), 0);
		assert_eq!(System::account_exists(&1), false);
	});
}

#[test]
fn it_can_recreate_a_killed_account() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let nonce = System::account_nonce(1);
		System::inc_account_nonce(1);
		set_argons(1, 2000);
		assert_ok!(LocalchainRelay::send_to_localchain(RuntimeOrigin::signed(1), 2000, 1, nonce));
		assert_eq!(Balances::free_balance(1), 0);
		assert_eq!(System::account_exists(&1), false);
		let expires_block: BlockNumberFor<Test> = (1u32 + TransferExpirationBlocks::get()).into();
		assert_eq!(ExpiringTransfersOut::<Test>::get(expires_block)[0], (1, nonce));
		System::set_block_number(expires_block);
		LocalchainRelay::on_initialize(expires_block);
		assert_eq!(System::account_exists(&1), true);
		assert_eq!(Balances::free_balance(1), 2000);
	});
}

#[test]
fn it_can_handle_multiple_transfer() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		MaxPendingTransfersOutPerBlock::set(2);
		System::set_block_number(1);
		let nonce = System::account_nonce(1);
		System::inc_account_nonce(1);
		set_argons(1, 5000);
		assert_ok!(LocalchainRelay::send_to_localchain(RuntimeOrigin::signed(1), 1000, 1, nonce));
		System::inc_account_nonce(1);
		assert_noop!(
			LocalchainRelay::send_to_localchain(RuntimeOrigin::signed(1), 700, 1, nonce),
			Error::<Test>::InvalidAccountNonce
		);
		assert_ok!(LocalchainRelay::send_to_localchain(
			RuntimeOrigin::signed(1),
			700,
			1,
			nonce + 1
		),);
		assert_eq!(Balances::free_balance(1), 3300);
		let expires_block: BlockNumberFor<Test> = (1u32 + TransferExpirationBlocks::get()).into();
		assert_eq!(
			ExpiringTransfersOut::<Test>::get(expires_block),
			vec![(1, nonce), (1, nonce + 1)]
		);

		System::inc_account_nonce(1);
		// We have a max number of transfers out per block
		assert_noop!(
			LocalchainRelay::send_to_localchain(RuntimeOrigin::signed(1), 1200, 1, nonce + 2),
			Error::<Test>::MaxBlockTransfersExceeded
		);
	});
}

#[test]
fn it_can_handle_transfers_in() {
	RequiredNotebookAuditors::set(0);
	MaxNotebookBlocksToRemember::set(2);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let nonce = System::account_nonce(1);
		set_argons(1, 5000);
		assert_ok!(LocalchainRelay::send_to_localchain(RuntimeOrigin::signed(1), 5000, 1, nonce));
		let expires_block: BlockNumberFor<Test> = (1u32 + TransferExpirationBlocks::get()).into();
		assert_eq!(ExpiringTransfersOut::<Test>::get(expires_block)[0], (1, nonce));

		System::set_block_number(2);
		System::on_initialize(2);
		LocalchainRelay::on_initialize(2);
		assert_ok!(LocalchainRelay::submit_notebook(
			RuntimeOrigin::none(),
			Hash::random(),
			Notebook {
				notary_id: 1,
				notebook_number: 1,
				auditors: bound(vec![]),
				pinned_to_block_number: 1,
				transfers: bound(vec![ChainTransfer::ToLocalchain { account_id: 1, nonce }]),
			},
			ed25519::Signature([0u8; 64]),
		),);
		assert_eq!(SubmittedNotebookBlocksByNotaryId::<Test>::get(1).len(), 1);

		System::set_block_number(3);
		System::on_initialize(3);
		LocalchainRelay::on_initialize(3);
		assert_eq!(ExpiringTransfersOut::<Test>::get(expires_block).len(), 0);
		assert_eq!(Balances::free_balance(1), 0);

		assert_ok!(LocalchainRelay::submit_notebook(
			RuntimeOrigin::none(),
			Hash::random(),
			Notebook {
				notary_id: 1,
				notebook_number: 2,
				auditors: bound(vec![]),
				pinned_to_block_number: 2,
				transfers: bound(vec![ChainTransfer::ToMainchain { account_id: 1, amount: 5000 }]),
			},
			ed25519::Signature([0u8; 64]),
		),);
		assert_eq!(Balances::free_balance(1), 5000);
		assert_eq!(SubmittedNotebookBlocksByNotaryId::<Test>::get(1), vec![1, 2]);

		assert_ok!(LocalchainRelay::submit_notebook(
			RuntimeOrigin::none(),
			Hash::random(),
			Notebook {
				notary_id: 1,
				notebook_number: 3,
				auditors: bound(vec![]),
				pinned_to_block_number: 3,
				transfers: bound(vec![]),
			},
			ed25519::Signature([0u8; 64]),
		),);
		assert_eq!(SubmittedNotebookBlocksByNotaryId::<Test>::get(1), vec![2, 3]);
	});
}

#[test]
fn it_doesnt_allow_a_notary_balance_to_go_negative() {
	RequiredNotebookAuditors::set(0);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		assert_noop!(
			LocalchainRelay::submit_notebook(
				RuntimeOrigin::none(),
				Hash::random(),
				Notebook {
					notary_id: 1,
					notebook_number: 1,
					auditors: bound(vec![]),
					pinned_to_block_number: 0,
					transfers: bound(vec![ChainTransfer::ToMainchain {
						account_id: 1,
						amount: 5000
					}]),
				},
				ed25519::Signature([0u8; 64]),
			),
			Error::<Test>::InsufficientNotarizedFunds
		);
	});
}

#[test]
fn it_requires_minimum_audits() {
	RequiredNotebookAuditors::set(4);
	new_test_ext().execute_with(|| {
		let ids = (0..7)
			.map(|a| UintAuthorityId(a).to_public_key::<BlockSealAuthorityId>())
			.collect::<Vec<_>>();
		let mut sealers = BTreeMap::new();
		sealers.insert(1, ids);
		BlockSealers::set(sealers);

		// Go past genesis block so events get deposited
		System::set_block_number(2);
		let authorities = (0..3).map(|i| UintAuthorityId(i)).collect::<Vec<_>>();
		assert_noop!(
			LocalchainRelay::submit_notebook(
				RuntimeOrigin::none(),
				Hash::random(),
				Notebook {
					notary_id: 1,
					notebook_number: 1,
					auditors: bound(
						authorities
							.iter()
							.map(|a| (
								a.to_public_key(),
								BlockSealAuthoritySignature::from(ed25519::Signature([0u8; 64]))
							))
							.collect::<Vec<(BlockSealAuthorityId, BlockSealAuthoritySignature)>>()
					),
					pinned_to_block_number: 1,
					transfers: bound(vec![]),
				},
				ed25519::Signature([0u8; 64]),
			),
			Error::<Test>::InsufficientNotebookSignatures
		);
	});
}

#[test]
fn it_requires_valid_auditors() {
	RequiredNotebookAuditors::set(4);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		let authorities = (0..10).map(|_| BlockSealAuthorityPair::generate().0).collect::<Vec<_>>();
		let ids = authorities.iter().map(|a| a.public().clone()).collect::<Vec<_>>();
		let mut sealers = BTreeMap::new();
		sealers.insert(1, ids[0..2].to_vec());
		sealers.insert(2, ids[0..7].to_vec());
		BlockSealers::set(sealers);

		let mut notebook = Notebook {
			notary_id: 1,
			notebook_number: 1,
			auditors: bound(vec![]),
			pinned_to_block_number: 2,
			transfers: bound(vec![]),
		};

		let signature_message =
			to_notebook_audit_signature_message(&notebook).using_encoded(blake2_256);

		let create_signatures =
			|list: Vec<usize>| -> Vec<(BlockSealAuthorityId, BlockSealAuthoritySignature)> {
				list.into_iter()
					.map(|e| {
						let signature = authorities[e].sign(&signature_message);
						(ids[e].clone(), signature)
					})
					.collect::<Vec<_>>()
			};

		notebook.auditors = bound(create_signatures(vec![0, 1, 2, 8]));

		assert_noop!(
			LocalchainRelay::submit_notebook(
				RuntimeOrigin::none(),
				Hash::random(),
				notebook.clone(),
				ed25519::Signature([0u8; 64]),
			),
			Error::<Test>::InvalidNotebookAuditor
		);

		notebook.auditors = bound(create_signatures(vec![0, 3, 4, 8]));

		assert_noop!(
			LocalchainRelay::submit_notebook(
				RuntimeOrigin::none(),
				Hash::from([0u8; 32]),
				notebook.clone(),
				ed25519::Signature([0u8; 64]),
			),
			Error::<Test>::InvalidNotebookAuditorIndex
		);

		notebook.auditors = bound(create_signatures(vec![0, 1, 2, 3]));
		assert_ok!(LocalchainRelay::submit_notebook(
			RuntimeOrigin::none(),
			Hash::random(),
			notebook.clone(),
			ed25519::Signature([0u8; 64]),
		),);
	});
}

#[test]
fn it_expires_transfers_if_not_committed() {
	RequiredNotebookAuditors::set(0);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let nonce = System::account_nonce(1);
		System::inc_account_nonce(1);
		set_argons(1, 5000);
		assert_ok!(LocalchainRelay::send_to_localchain(RuntimeOrigin::signed(1), 1000, 1, nonce));
		assert_eq!(
			PendingTransfersOut::<Test>::get(1u64, nonce).unwrap(),
			QueuedTransferOut {
				amount: 1000u128,
				notary_id: 1u32,
				expiration_block: (1u32 + TransferExpirationBlocks::get()).into(),
			}
		);

		System::set_block_number((1u32 + TransferExpirationBlocks::get()).into());
		assert_err!(
			LocalchainRelay::submit_notebook(
				RuntimeOrigin::none(),
				H256::random(),
				Notebook {
					notary_id: 1,
					notebook_number: 1,
					auditors: bound(vec![]),
					pinned_to_block_number: 0,
					transfers: bound(vec![ChainTransfer::ToLocalchain { account_id: 1, nonce }]),
				},
				ed25519::Signature([0u8; 64]),
			),
			Error::<Test>::NotebookIncludesExpiredLocalchainTransfer
		)
	});
}

#[test]
fn it_delays_for_finalization() {
	RequiredNotebookAuditors::set(4);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		FinalizedBlockNumber::<Test>::set(0);
		assert_noop!(
			LocalchainRelay::validate_unsigned(
				TransactionSource::Local,
				&crate::Call::submit_notebook {
					notebook_hash: Hash::default(),
					notebook: Notebook {
						notary_id: 1,
						notebook_number: 1,
						auditors: bound(vec![]),
						pinned_to_block_number: 1,
						transfers: bound(vec![]),
					},
					signature: ed25519::Signature([0u8; 64]),
				},
			),
			InvalidTransaction::Future
		);
	});
}

#[test]
fn it_processes_the_latest_finalized_block() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::deposit_log(DigestItem::PreRuntime(
			FINALIZED_BLOCK_DIGEST_ID,
			FinalizedBlockNeededDigest::<Block> { hash: [0u8; 32].into(), number: 5 }
				.encode()
				.to_vec(),
		));
		System::set_block_number(20);
		System::on_initialize(20);
		LocalchainRelay::on_initialize(20);
		assert_eq!(FinalizedBlockNumber::<Test>::get(), 5);
	});
}

fn bound<T, S>(list: Vec<T>) -> BoundedVec<T, S>
where
	S: sp_core::Get<u32>,
{
	BoundedVec::<T, S>::truncate_from(list)
}
