use serde_json::{from_value, json};
use sqlx::{query, query_scalar, types::Json, FromRow, PgPool};
use std::collections::BTreeMap;
use ulx_notary_audit::{verify_balance_changeset_allocation, verify_changeset_signatures};

use crate::apis::localchain::BalanceChangeResult;
use ulx_notary_primitives::{
	ensure, AccountId, AccountOrigin, AccountType, BalanceChange, BalanceProof, BalanceTip,
	NewAccountOrigin, NotaryId, NoteType, NotebookNumber, MAX_TRANSFERS,
};

use crate::{
	error::Error,
	stores::{
		balance_tip::BalanceTipStore, block_meta::BlockMetaStore,
		chain_transfer::ChainTransferStore, notebook::NotebookStore,
		notebook_new_accounts::NotebookNewAccountsStore, notebook_status::NotebookStatusStore,
	},
};

pub struct BalanceChangeStore;
#[derive(FromRow)]
#[allow(dead_code)]
struct BalanceChangeRow {
	pub notebook_number: i32,
	/// Scale encoded set of BalanceChangesets submitted together
	pub changeset: Json<Vec<BalanceChange>>,
}

impl BalanceChangeStore {
	pub async fn append_notebook_changeset<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
		changeset: Vec<BalanceChange>,
	) -> anyhow::Result<(), Error> {
		let data = json!(changeset);
		let res = query!(
			r#"
			INSERT INTO balance_changes (notebook_number, changeset) VALUES ($1, $2)
		"#,
			notebook_number as i32,
			data
		)
		.execute(db)
		.await?;

		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert balance changes".to_string())
		);

		Ok(())
	}

	pub async fn get_for_notebook<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<Vec<Vec<BalanceChange>>, Error> {
		let rows = query_scalar!(
			r#"
			SELECT changeset FROM balance_changes WHERE notebook_number = $1
		"#,
			notebook_number as i32,
		)
		.fetch_all(db)
		.await?;

		let result: Result<Vec<Vec<BalanceChange>>, _> =
			rows.into_iter().map(from_value::<Vec<BalanceChange>>).collect();

		Ok(result?)
	}

	/// ## Basic Mainchain -> Localchain flow:
	/// 1. Funds transfer to localchain via mainchain relay transactions ("LocalchainRelay" in
	///    Ulixee Mainchain)
	/// 2. Localchain wallet submits a balance change including Note referencing the mainchain nonce
	///    used for the transfer "in"
	/// 3. Balance change is applied to account directly
	///
	/// ## Basic Localchain -> Mainchain flow:
	/// 1. Localchain wallet submits a balance change with a Note to the mainchain
	/// 2. Notary relays transactions in the next notebook it submits to the network (every minute).
	/// 3. Mainchain relay will apply the balance change to the account id on the mainchain
	///
	/// ## Basic Transfer Flow
	/// 1. Alice Localchain sends a note (type: Send) to Bob
	/// 2. Bob applies the note to his wallet with a BalanceChange to his Wallet
	/// 3. Bob notarizes BalanceChange to his wallet and Alice's wallet in a single transaction.
	///    Transaction must allocate all funds.
	/// 4. This transaction is included in a Notebook on a block with a merkle root containing Alice
	///    and Bob's transfer Key(account, account_typechain), value: H256(balance, nonce, account
	///    origin).
	/// 5. Each user can retrieve proof that their balances can be proven in the recorded merkle
	///    root. They can also obtain their account origin, which must be included in all future
	///    changes. The account origin can be used to prove their balance has not changed in any
	///    blocks since that change.
	/// 6. If a notary is compromised, the user can use the proof of last balance change to migrate
	///    their balance to a new notary. NOTE: you must have proof from the completed notebook.
	pub async fn apply_balance_changes(
		pool: &PgPool,
		notary_id: NotaryId,
		changes: Vec<BalanceChange>,
	) -> anyhow::Result<BalanceChangeResult, Error> {
		// Before we use db resources, let's confirm these are valid transactions
		let initial_allocation_result = verify_balance_changeset_allocation(&changes, None)?;
		verify_changeset_signatures(&changes)?;

		// Begin database transaction
		let mut tx = pool.begin().await?;

		let meta = BlockMetaStore::load(&mut tx).await?;
		let current_notebook_number =
			NotebookStatusStore::lock_open_for_appending(&mut *tx).await?;

		if initial_allocation_result.needs_channel_settle_followup {
			verify_balance_changeset_allocation(&changes, Some(current_notebook_number))?;
		}

		let mut new_account_origins = BTreeMap::<(AccountId, AccountType), AccountOrigin>::new();

		let mut final_changes = changes.clone();
		for (change_index, change) in changes.into_iter().enumerate() {
			let BalanceChange { account_id, account_type, change_number, balance, .. } = change;
			let key = (account_id.clone(), account_type.clone());

			let account_origin = change
				.previous_balance_proof
				.as_ref()
				.map(|p| p.account_origin.clone())
				.or_else(|| new_account_origins.get(&key).map(|a| a.clone()));

			let account_origin = match account_origin {
				Some(account_origin) => account_origin,
				None => {
					if change.change_number != 1 {
						return Err(Error::MissingAccountOrigin)
					}

					let account_uid = NotebookNewAccountsStore::insert_origin(
						&mut *tx,
						current_notebook_number,
						&account_id,
						&account_type,
					)
					.await?;

					let origin =
						AccountOrigin { notebook_number: current_notebook_number, account_uid };

					new_account_origins.insert(key.clone(), origin.clone());
					origin
				},
			};
			let previous_balance =
				change.previous_balance_proof.as_ref().map(|p| p.balance.clone()).unwrap_or(0);

			let channel_hold_note = change.channel_hold_note;
			BalanceTipStore::lock(
				&mut *tx,
				&account_id,
				account_type.clone(),
				change_number,
				previous_balance.clone(),
				&account_origin,
				change_index,
				channel_hold_note.clone(),
				5000,
			)
			.await?;

			if let Some(proof) = change.previous_balance_proof {
				let tip = BalanceTip {
					account_id: account_id.clone(),
					account_type: account_type.clone(),
					change_number,
					balance,
					account_origin: account_origin.clone(),
					channel_hold_note,
				};

				// TODO: handle notary switching
				ensure!(proof.notary_id == notary_id, Error::CrossNotaryProofsNotImplemented);

				// We fill this in when not provided as convenience.
				// TODO: We should add a fee for this though.
				if proof.notebook_number < current_notebook_number && proof.notebook_proof.is_none()
				{
					let notebook_proof = NotebookStore::get_balance_proof(
						&mut *tx,
						notary_id,
						proof.notebook_number,
						&tip,
					)
					.await?;

					// record into the final changeset
					final_changes[change_index].previous_balance_proof = Some(BalanceProof {
						balance: proof.balance,
						notary_id: proof.notary_id,
						notebook_number: proof.notebook_number,
						account_origin: proof.account_origin.clone(),
						notebook_proof: Some(notebook_proof),
					});
				}

				if let Some(notebook_proof) = &proof.notebook_proof {
					ensure!(
						NotebookStore::is_valid_proof(
							&mut *tx,
							&tip,
							proof.notebook_number,
							&notebook_proof
						)
						.await?,
						Error::InvalidBalanceProof
					);
				}
			}

			let mut channel_hold_note = None;
			for (note_index, note) in change.notes.into_iter().enumerate() {
				let _ = match note.note_type {
					NoteType::ClaimFromMainchain { account_nonce: nonce, .. } => {
						// NOTE: transfers can expire. We need to ensure this can still get into a
						// notebook
						ChainTransferStore::take_and_record_transfer_local(
							&mut *tx,
							current_notebook_number,
							&account_id,
							nonce,
							note.milligons,
							change_index,
							note_index,
							MAX_TRANSFERS,
						)
						.await
					},
					NoteType::SendToMainchain => ChainTransferStore::record_transfer_to_mainchain(
						&mut *tx,
						current_notebook_number,
						&account_id,
						note.milligons,
						MAX_TRANSFERS,
					)
					.await
					.map(|_| ()),
					NoteType::ChannelHold { .. } => {
						channel_hold_note = Some(note.clone());
						Ok(())
					},
					NoteType::ChannelSettle => {
						channel_hold_note = None;
						Ok(())
					},
					_ => Ok(()),
				}
				.map_err(|e| Error::BalanceChangeError {
					change_index,
					note_index,
					message: e.to_string(),
				})?;
			}

			BalanceTipStore::update(
				&mut *tx,
				&account_id,
				account_type,
				change_number,
				balance,
				current_notebook_number,
				account_origin,
				channel_hold_note,
				previous_balance.clone(),
			)
			.await?;
		}

		BalanceChangeStore::append_notebook_changeset(
			&mut *tx,
			current_notebook_number,
			final_changes,
		)
		.await?;

		tx.commit().await?;
		Ok(BalanceChangeResult {
			notebook_number: current_notebook_number,
			finalized_block_number: meta.finalized_block_number,
			new_account_origins: new_account_origins
				.into_iter()
				.map(|((account_id, account_type), origin)| {
					NewAccountOrigin::new(account_id, account_type, origin.account_uid)
				})
				.collect(),
		})
	}
}

#[cfg(test)]
mod tests {
	use sp_core::{bounded_vec, ed25519::Signature};
	use sp_keyring::Sr25519Keyring::Bob;
	use sqlx::PgPool;

	use ulx_notary_primitives::{AccountType::Deposit, BalanceChange, Note, NoteType};

	use crate::stores::balance_change::BalanceChangeStore;

	#[sqlx::test]
	async fn test_storage(pool: PgPool) -> anyhow::Result<()> {
		let notebook_number = 1;
		let changeset = vec![BalanceChange {
			account_id: Bob.to_account_id(),
			account_type: Deposit,
			change_number: 0,
			balance: 1000,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(
				1000,
				NoteType::ClaimFromMainchain { account_nonce: 1 }
			)],
			signature: Signature([0u8; 64]).into(),
		}];

		{
			let mut tx = pool.begin().await?;
			BalanceChangeStore::append_notebook_changeset(
				&mut *tx,
				notebook_number,
				changeset.clone(),
			)
			.await
			.unwrap();
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			let result =
				BalanceChangeStore::get_for_notebook(&mut *tx, notebook_number).await.unwrap();
			assert_eq!(result, vec![changeset]);
			tx.commit().await?;
		}

		Ok(())
	}
}
