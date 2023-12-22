use std::collections::BTreeMap;

use binary_merkle_tree::merkle_root;
use codec::Encode;
use frame_support::{assert_err, assert_noop, assert_ok, traits::OnInitialize};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_core::{
	bounded_vec, ed25519,
	ed25519::{Public, Signature},
	Blake2Hasher, Pair,
};
use sp_keyring::AccountKeyring::Bob;
use sp_runtime::{
	testing::{UintAuthorityId, H256},
	traits::{UniqueSaturatedInto, ValidateUnsigned},
	transaction_validity::{InvalidTransaction, TransactionSource},
	BoundedVec, DigestItem,
};

use ulx_primitives::{
	block_seal::BlockSealAuthorityPair,
	digests::{FinalizedBlockNeededDigest, FINALIZED_BLOCK_DIGEST_ID},
	localchain::{AccountType, BalanceChange, Note, NoteType},
	notebook::{
		AccountOrigin, AuditedNotebook, BalanceTip, ChainTransfer, NewAccountOrigin, Notebook,
		NotebookHeader,
	},
	BlockSealAuthorityId,
};

use crate::{
	mock::{LocalchainRelay, *},
	pallet::{
		AccountOriginLastChangedNotebookByNotary, ExpiringTransfersOut, FinalizedBlockNumber,
		LastNotebookNumberByNotary, NotebookChangedAccountsRootByNotary, PendingTransfersOut,
	},
	Error, QueuedTransferOut,
};

type Hash = H256;

#[test]
fn it_can_send_funds_to_localchain() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let nonce = System::account_nonce(&who);
		System::inc_account_nonce(&who);
		set_argons(&who, 5000);
		assert_ok!(LocalchainRelay::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			1000,
			1,
			nonce
		));
		assert_eq!(Balances::free_balance(&who), 4000);
		let expires_block: BlockNumberFor<Test> = (1u32 + TransferExpirationBlocks::get()).into();
		assert_eq!(ExpiringTransfersOut::<Test>::get(expires_block)[0], (who.clone(), nonce));
	});
}

#[test]
fn it_allows_you_to_transfer_full_balance() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let nonce = System::account_nonce(&who);
		System::inc_account_nonce(&who);
		set_argons(&who, 5000);
		assert_ok!(LocalchainRelay::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			5000,
			1,
			nonce
		));
		assert_eq!(Balances::free_balance(&who), 0);
		assert_eq!(System::account_exists(&who), false);
	});
}

#[test]
fn it_can_recreate_a_killed_account() {
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let nonce = System::account_nonce(&who);
		System::inc_account_nonce(&who);
		set_argons(&who, 2000);
		assert_ok!(LocalchainRelay::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			2000,
			1,
			nonce
		));
		assert_eq!(Balances::free_balance(&who), 0);
		assert_eq!(System::account_exists(&who), false);
		let expires_block: BlockNumberFor<Test> = (1u32 + TransferExpirationBlocks::get()).into();
		assert_eq!(ExpiringTransfersOut::<Test>::get(expires_block)[0], (who.clone(), nonce));
		System::set_block_number(expires_block);
		LocalchainRelay::on_initialize(expires_block);
		assert_eq!(System::account_exists(&who), true);
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
		let nonce = System::account_nonce(&who);
		System::inc_account_nonce(&who);
		set_argons(&who, 5000);
		assert_ok!(LocalchainRelay::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			1000,
			1,
			nonce
		));
		System::inc_account_nonce(&who);
		assert_noop!(
			LocalchainRelay::send_to_localchain(RuntimeOrigin::signed(who.clone()), 700, 1, nonce),
			Error::<Test>::InvalidAccountNonce
		);
		assert_ok!(LocalchainRelay::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			700,
			1,
			nonce + 1
		),);
		assert_eq!(Balances::free_balance(&who), 3300);
		let expires_block: BlockNumberFor<Test> = (1u32 + TransferExpirationBlocks::get()).into();
		assert_eq!(
			ExpiringTransfersOut::<Test>::get(expires_block),
			vec![(who.clone(), nonce), (who.clone(), nonce + 1)]
		);

		System::inc_account_nonce(&who);
		// We have a max number of transfers out per block
		assert_noop!(
			LocalchainRelay::send_to_localchain(
				RuntimeOrigin::signed(who.clone()),
				1200,
				1,
				nonce + 2
			),
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
		let who = Bob.to_account_id();
		let nonce = System::account_nonce(&who).into();
		set_argons(&who, 5000);
		assert_ok!(LocalchainRelay::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			5000,
			1,
			nonce
		));
		let expires_block: BlockNumberFor<Test> = (1u32 + TransferExpirationBlocks::get()).into();
		assert_eq!(ExpiringTransfersOut::<Test>::get(expires_block)[0], (who.clone(), nonce));

		System::set_block_number(2);
		System::on_initialize(2);
		LocalchainRelay::on_initialize(2);
		let changed_accounts_root = H256::random();
		assert_ok!(LocalchainRelay::submit_notebook(
			RuntimeOrigin::none(),
			AuditedNotebook {
				header_hash: Hash::random(),
				header: NotebookHeader {
					notary_id: 1,
					notebook_number: 1,
					pinned_to_block_number: 1,
					chain_transfers: bounded_vec![ChainTransfer::ToLocalchain {
						account_id: Bob.to_account_id(),
						account_nonce: nonce.unique_saturated_into()
					}],
					changed_accounts_root: changed_accounts_root.clone(),
					changed_account_origins: bounded_vec![AccountOrigin {
						notebook_number: 1,
						account_uid: 1
					}],
					version: 1,
					finalized_block_number: 1,
					start_time: 0,
					end_time: 0,
					tax: 0,
				},
				auditors: bounded_vec![],
			},
			ed25519::Signature([0u8; 64]),
		),);
		assert_eq!(
			NotebookChangedAccountsRootByNotary::<Test>::get(1, 1),
			Some(changed_accounts_root)
		);
		assert_eq!(
			AccountOriginLastChangedNotebookByNotary::<Test>::get(
				1,
				AccountOrigin { notebook_number: 1, account_uid: 1 }
			),
			Some(1)
		);

		System::set_block_number(3);
		System::on_initialize(3);
		LocalchainRelay::on_initialize(3);
		assert_eq!(ExpiringTransfersOut::<Test>::get(expires_block).len(), 0);
		assert_eq!(Balances::free_balance(&who), 0);

		let change_root_2 = H256::random();
		assert_ok!(LocalchainRelay::submit_notebook(
			RuntimeOrigin::none(),
			AuditedNotebook {
				header_hash: Hash::random(),
				header: NotebookHeader {
					notary_id: 1,
					notebook_number: 2,
					pinned_to_block_number: 2,
					chain_transfers: bounded_vec![ChainTransfer::ToMainchain {
						account_id: who.clone(),
						amount: 5000
					}],
					changed_accounts_root: change_root_2.clone(),
					changed_account_origins: bounded_vec![AccountOrigin {
						notebook_number: 1,
						account_uid: 1
					}],
					version: 1,
					finalized_block_number: 1,
					start_time: 0,
					end_time: 0,
					tax: 0,
				},
				auditors: bounded_vec![],
			},
			ed25519::Signature([0u8; 64]),
		),);
		assert_eq!(Balances::free_balance(&who), 5000);
		assert_eq!(
			NotebookChangedAccountsRootByNotary::<Test>::get(1, 1),
			Some(changed_accounts_root)
		);
		assert_eq!(
			AccountOriginLastChangedNotebookByNotary::<Test>::get(
				1,
				AccountOrigin { notebook_number: 1, account_uid: 1 }
			),
			Some(2)
		);
		assert_eq!(NotebookChangedAccountsRootByNotary::<Test>::get(1, 2), Some(change_root_2));
		assert_eq!(LastNotebookNumberByNotary::<Test>::get(1), Some((2, 2)));

		assert_ok!(LocalchainRelay::submit_notebook(
			RuntimeOrigin::none(),
			AuditedNotebook {
				header_hash: Hash::random(),
				header: NotebookHeader {
					notary_id: 1,
					notebook_number: 3,
					pinned_to_block_number: 2,
					chain_transfers: bounded_vec![],
					changed_accounts_root: H256::random(),
					changed_account_origins: bounded_vec![],
					version: 1,
					finalized_block_number: 1,
					start_time: 0,
					end_time: 0,
					tax: 0,
				},
				auditors: bounded_vec![],
			},
			ed25519::Signature([0u8; 64]),
		),);
		assert_eq!(LastNotebookNumberByNotary::<Test>::get(1), Some((3, 2)));
		assert_eq!(
			AccountOriginLastChangedNotebookByNotary::<Test>::get(
				1,
				AccountOrigin { notebook_number: 1, account_uid: 1 }
			),
			Some(2)
		);
	});
}

#[test]
fn it_reduces_circulation_on_tax() {
	RequiredNotebookAuditors::set(0);
	MaxNotebookBlocksToRemember::set(2);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let who = LocalchainRelay::notary_account_id(1);
		set_argons(&who, 25000);
		assert_eq!(Balances::total_issuance(), 25_000);

		LocalchainRelay::on_initialize(2);
		assert_ok!(LocalchainRelay::submit_notebook(
			RuntimeOrigin::none(),
			AuditedNotebook {
				header_hash: Hash::random(),
				header: NotebookHeader {
					notary_id: 1,
					notebook_number: 1,
					pinned_to_block_number: 1,
					chain_transfers: bounded_vec![],
					changed_accounts_root: H256::random(),
					changed_account_origins: bounded_vec![],
					version: 1,
					finalized_block_number: 1,
					start_time: 0,
					end_time: 0,
					tax: 2000,
				},
				auditors: bounded_vec![],
			},
			ed25519::Signature([0u8; 64]),
		),);

		assert_eq!(Balances::total_issuance(), 23_000);
		assert_eq!(Balances::free_balance(&who), 23_000);
	})
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
				AuditedNotebook {
					header_hash: Hash::random(),
					header: NotebookHeader {
						notary_id: 1,
						notebook_number: 1,
						pinned_to_block_number: 0,
						chain_transfers: bounded_vec![ChainTransfer::ToMainchain {
							account_id: Bob.to_account_id(),
							amount: 5000
						}],
						changed_accounts_root: H256::random(),
						changed_account_origins: bounded_vec![],
						version: 1,
						finalized_block_number: 1,
						start_time: 0,
						end_time: 0,
						tax: 0,
					},
					auditors: bounded_vec![],
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
				AuditedNotebook {
					header: NotebookHeader {
						notary_id: 1,
						notebook_number: 1,
						pinned_to_block_number: 1,
						chain_transfers: bounded_vec![],
						changed_accounts_root: H256::random(),
						changed_account_origins: bounded_vec![],
						version: 1,
						finalized_block_number: 1,
						start_time: 0,
						end_time: 0,
						tax: 0,
					},
					header_hash: Hash::random(),
					auditors: bound(
						authorities
							.iter()
							.map(|a| (a.to_public_key(), ed25519::Signature([0u8; 64])))
							.collect::<Vec<_>>()
					),
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

		let notebook = NotebookHeader {
			notary_id: 1,
			notebook_number: 1,
			pinned_to_block_number: 2,
			chain_transfers: bounded_vec![],
			changed_accounts_root: H256::random(),
			changed_account_origins: bounded_vec![],
			version: 1,
			finalized_block_number: 1,
			start_time: 0,
			end_time: 0,
			tax: 0,
		};

		let header_hash = notebook.hash();

		let create_signatures = |list: Vec<usize>| -> Vec<(Public, Signature)> {
			list.into_iter()
				.map(|e| {
					let signature = authorities[e].sign(&header_hash.0);
					(ids[e].clone().into_inner(), signature.into_inner())
				})
				.collect::<Vec<_>>()
		};

		let mut audited_notebook = AuditedNotebook {
			header_hash,
			header: notebook.clone(),
			auditors: bound(create_signatures(vec![0, 1, 2, 8])),
		};

		assert_noop!(
			LocalchainRelay::submit_notebook(
				RuntimeOrigin::none(),
				audited_notebook.clone(),
				ed25519::Signature([0u8; 64]),
			),
			Error::<Test>::InvalidNotebookAuditor
		);

		audited_notebook.auditors = bound(create_signatures(vec![0, 3, 4, 8]));

		assert_noop!(
			LocalchainRelay::submit_notebook(
				RuntimeOrigin::none(),
				audited_notebook.clone(),
				ed25519::Signature([0u8; 64]),
			),
			Error::<Test>::InvalidNotebookAuditorIndex
		);

		audited_notebook.auditors = bound(create_signatures(vec![0, 1, 2, 3]));
		assert_ok!(LocalchainRelay::submit_notebook(
			RuntimeOrigin::none(),
			audited_notebook.clone(),
			ed25519::Signature([0u8; 64]),
		),);
	});
}

#[test]
fn it_expires_transfers_if_not_committed() {
	RequiredNotebookAuditors::set(0);
	new_test_ext().execute_with(|| {
		let who = Bob.to_account_id();
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let nonce = System::account_nonce(&who);
		System::inc_account_nonce(&who);
		set_argons(&who, 5000);
		assert_ok!(LocalchainRelay::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			1000,
			1,
			nonce
		));
		assert_eq!(
			PendingTransfersOut::<Test>::get(&who, nonce).unwrap(),
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
				AuditedNotebook {
					header: NotebookHeader {
						notary_id: 1,
						notebook_number: 1,
						pinned_to_block_number: 0,
						chain_transfers: bounded_vec![ChainTransfer::ToLocalchain {
							account_id: who.clone(),
							account_nonce: nonce.unique_saturated_into()
						}],
						changed_accounts_root: H256::random(),
						changed_account_origins: bounded_vec![],
						version: 1,
						finalized_block_number: 1,
						start_time: 0,
						end_time: 0,
						tax: 0,
					},
					header_hash: H256::random(),
					auditors: bounded_vec![],
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
					notebook: AuditedNotebook {
						header: NotebookHeader {
							notary_id: 1,
							notebook_number: 1,
							pinned_to_block_number: 1,
							chain_transfers: bounded_vec![],
							changed_accounts_root: H256::random(),
							changed_account_origins: bounded_vec![],
							version: 1,
							finalized_block_number: 1,
							start_time: 0,
							end_time: 0,
							tax: 0,
						},
						header_hash: H256::random(),
						auditors: bounded_vec![],
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

#[test]
fn it_can_audit_notebooks() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		FinalizedBlockNumber::<Test>::set(0);
		let who = Bob.to_account_id();
		let nonce = System::account_nonce(&who);
		let notary_id = 1;
		System::inc_account_nonce(&who);
		set_argons(&who, 2000);
		assert_ok!(LocalchainRelay::send_to_localchain(
			RuntimeOrigin::signed(who.clone()),
			2000,
			notary_id,
			nonce
		));

		System::set_block_number(2);
		LocalchainRelay::on_initialize(2);

		let header = NotebookHeader {
			notary_id,
			notebook_number: 1,
			pinned_to_block_number: 1,
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain {
				account_id: who.clone(),
				account_nonce: nonce.unique_saturated_into()
			}],
			changed_accounts_root: merkle_root::<Blake2Hasher, _>(vec![BalanceTip {
				account_id: who.clone(),
				account_type: AccountType::Deposit,
				change_number: 1,
				balance: 2000,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
				channel_hold_note: None,
			}
			.encode()]),
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			version: 1,
			finalized_block_number: 1,
			start_time: 0,
			end_time: 0,
			tax: 0,
		};
		let header_hash = header.hash();
		let signature = ed25519::Signature([0u8; 64]);

		let notebook = Notebook {
			header,
			new_account_origins: bounded_vec![NewAccountOrigin::new(
				who.clone(),
				AccountType::Deposit,
				1
			)],
			balance_changes: bounded_vec![bounded_vec![BalanceChange {
				account_id: who.clone(),
				account_type: AccountType::Deposit,
				balance: 2000,
				previous_balance_proof: None,
				change_number: 1,
				notes: bounded_vec![Note::create(
					2000,
					NoteType::ClaimFromMainchain { account_nonce: nonce.unique_saturated_into() },
				)],
				channel_hold_note: None,
				signature: signature.clone().into(),
			}
			.sign(Bob.pair())
			.clone()]],
		};

		assert_ok!(LocalchainRelay::audit_notebook(
			1,
			notary_id,
			notary_id,
			signature,
			header_hash,
			notebook.encode(),
		));
	});
}
fn bound<T, S>(list: Vec<T>) -> BoundedVec<T, S>
where
	S: sp_core::Get<u32>,
{
	BoundedVec::<T, S>::truncate_from(list)
}
