use std::collections::{BTreeMap, BTreeSet};

use crate::{
	notary_metrics::NotaryMetrics,
	stores::{
		balance_tip::BalanceTipStore,
		chain_transfer::ChainTransferStore,
		notebook::NotebookStore,
		notebook_constraints::{MaxNotebookCounts, NotarizationCounts, NotebookConstraintsStore},
		notebook_new_accounts::NotebookNewAccountsStore,
		notebook_status::NotebookStatusStore,
	},
};
use argon_notary_apis::{error::Error, localchain::BalanceChangeResult};
use argon_notary_audit::{
	verify_changeset_signatures, verify_notarization_allocation, verify_voting_sources,
};
use argon_primitives::{
	ensure, tick::Ticker, AccountId, AccountOrigin, AccountType, Balance, BalanceChange,
	BalanceProof, BalanceTip, BlockVote, DomainHash, LocalchainAccountId, NewAccountOrigin,
	Notarization, NotaryId, NoteType, NotebookNumber,
};
use codec::Encode;
use polkadot_sdk::*;
use serde_json::{from_value, json};
use sp_runtime::BoundedVec;
use sqlx::{query, types::Json, FromRow, PgConnection, PgPool};

#[derive(FromRow)]
#[allow(dead_code)]
struct NotarizationRow {
	pub notebook_number: i32,
	/// An ordered sequence number within the notebook
	pub sequence_number: i32,
	/// Scale encoded set of BalanceChangesets submitted together
	pub balance_changes: Json<Vec<BalanceChange>>,
	/// Scale encoded set of BlockVotes submitted together
	pub block_votes: Json<Vec<BlockVote>>,
	/// Scale encoded set of Domains submitted together
	pub domains: Json<Vec<(DomainHash, AccountId)>>,
}
pub struct NotarizationsStore;

impl NotarizationsStore {
	pub fn create_account_lookup_key(
		account_id: &AccountId,
		account_type: &AccountType,
		change_number: u32,
	) -> Vec<u8> {
		(account_id.clone(), *account_type, change_number).encode()
	}

	pub async fn append_to_notebook<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
		sequence_number: u32,
		balance_changes: Vec<BalanceChange>,
		block_votes: Vec<BlockVote>,
		domains: Vec<(DomainHash, AccountId)>,
	) -> anyhow::Result<(), Error> {
		let balance_changes_json = json!(balance_changes);
		let mut account_lookups = BTreeSet::new();
		for change in &balance_changes {
			account_lookups.insert(Self::create_account_lookup_key(
				&change.account_id,
				&change.account_type,
				change.change_number,
			));
		}

		let res = query!(
			r#"
			INSERT INTO notarizations (notebook_number, sequence_number, balance_changes, block_votes, domains, account_lookups) VALUES ($1, $2, $3, $4, $5, $6)
		"#,
			notebook_number as i32,
			sequence_number as i32,
			balance_changes_json,
			json!(block_votes),
			json!(domains),
			&account_lookups.into_iter().collect::<Vec<_>>(),
		)
		.execute(db)
		.await?;

		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert balance changes".to_string())
		);

		Ok(())
	}

	pub async fn get_account_change(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
		account_id: AccountId,
		account_type: AccountType,
		change_number: u32,
	) -> anyhow::Result<Notarization, Error> {
		let lookup_key = Self::create_account_lookup_key(&account_id, &account_type, change_number);
		let row = sqlx::query!(
			"SELECT * FROM notarizations WHERE notebook_number = $1 AND $2 = ANY (account_lookups) LIMIT 1",
			notebook_number as i32,
			lookup_key,
		)
		.fetch_one(db)
		.await?;

		let balance_changes = from_value::<Vec<BalanceChange>>(row.balance_changes)?;
		let block_votes = from_value::<Vec<BlockVote>>(row.block_votes)?;
		let domains = from_value::<Vec<(DomainHash, AccountId)>>(row.domains)?;
		Ok(Notarization {
			balance_changes: BoundedVec::truncate_from(balance_changes),
			block_votes: BoundedVec::truncate_from(block_votes),
			domains: BoundedVec::truncate_from(domains),
		})
	}

	pub async fn get_for_notebook<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<Vec<Notarization>, Error> {
		let rows = query!(
			r#"
			SELECT balance_changes, block_votes, domains FROM notarizations WHERE notebook_number = $1 ORDER BY sequence_number ASC
		"#,
			notebook_number as i32,
		)
		.fetch_all(db)
		.await?;

		let mut result = Vec::new();
		for row in rows {
			let balance_changes = from_value::<Vec<BalanceChange>>(row.balance_changes)?;
			let block_votes = from_value::<Vec<BlockVote>>(row.block_votes)?;
			let domains = from_value::<Vec<(DomainHash, AccountId)>>(row.domains)?;
			result.push(Notarization::new(balance_changes, block_votes, domains));
		}

		Ok(result)
	}

	/// ## Basic Mainchain -> Localchain flow:
	/// 1. Funds transfer to localchain via mainchain relay transactions ("LocalchainRelay" in Argon
	///    Mainchain)
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
	#[allow(clippy::too_many_arguments)]
	pub async fn apply(
		pool: &PgPool,
		notary_id: NotaryId,
		operator_account_id: &AccountId,
		ticker: &Ticker,
		notary_metrics: &NotaryMetrics,
		changes: Vec<BalanceChange>,
		block_votes: Vec<BlockVote>,
		domains: Vec<(DomainHash, AccountId)>,
	) -> anyhow::Result<BalanceChangeResult, Error> {
		if changes.is_empty() {
			return Err(Error::EmptyNotarizationProposed);
		}

		let balance_change_len = changes.len();
		let block_votes_len = block_votes.len();
		let domains_len = domains.len();
		// Before we use db resources, let's confirm these are valid transactions
		let initial_allocation_result = verify_notarization_allocation(
			&changes,
			&block_votes,
			&domains,
			None,
			ticker.channel_hold_expiration_ticks,
		)?;
		verify_changeset_signatures(&changes)?;

		let mut voted_blocks = BTreeSet::new();
		for vote in &block_votes {
			voted_blocks.insert(vote.block_hash);
		}

		// Begin database transaction
		let mut tx = pool.begin().await?;

		let (current_notebook_number, tick) =
			NotebookStatusStore::lock_open_for_appending(&mut tx).await?;
		let sequence_number = Self::next_sequence_number(&mut *tx, current_notebook_number).await?;

		if initial_allocation_result.needs_channel_hold_settle_followup {
			verify_notarization_allocation(
				&changes,
				&block_votes,
				&domains,
				Some(tick),
				ticker.channel_hold_expiration_ticks,
			)?;
		}

		let mut new_account_origins = BTreeMap::<LocalchainAccountId, AccountOrigin>::new();

		let mut changes_with_proofs = changes.clone();
		let mut chain_transfers: u32 = 0;
		let mut tax: Balance = 0;

		for (change_index, change) in changes.into_iter().enumerate() {
			let change_index = change_index as u32;
			let BalanceChange { account_id, account_type, change_number, balance, .. } = change;
			let localchain_account_id = LocalchainAccountId::new(account_id.clone(), account_type);

			let account_origin = change
				.previous_balance_proof
				.as_ref()
				.map(|p| p.account_origin.clone())
				.or_else(|| new_account_origins.get(&localchain_account_id).cloned());

			let account_origin = match account_origin {
				Some(account_origin) => account_origin,
				None => {
					if change.change_number != 1 {
						return Err(Error::MissingAccountOrigin);
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

					new_account_origins.insert(localchain_account_id.clone(), origin.clone());
					origin
				},
			};
			let previous_balance =
				change.previous_balance_proof.as_ref().map(|p| p.balance).unwrap_or(0);

			let prev_channel_hold_note = change.channel_hold_note;
			BalanceTipStore::lock(
				&mut tx,
				&account_id,
				account_type,
				change_number,
				previous_balance,
				&account_origin,
				change_index,
				prev_channel_hold_note.clone(),
				5000,
			)
			.await?;

			if let Some(proof) = change.previous_balance_proof {
				let proof_tip = BalanceTip {
					account_id: account_id.clone(),
					account_type,
					change_number: change_number - 1,
					balance: previous_balance,
					account_origin: account_origin.clone(),
					channel_hold_note: prev_channel_hold_note.clone(),
				};

				// TODO: handle notary switching
				ensure!(proof.notary_id == notary_id, Error::CrossNotaryProofsNotImplemented);

				// We fill this in when not provided as convenience, and for when a proof cannot be
				// provided because we're in the middle of a notebook TODO: charge a lookup fee
				if proof.notebook_number < current_notebook_number && proof.notebook_proof.is_none()
				{
					let notebook_proof = NotebookStore::get_balance_proof(
						&mut *tx,
						notary_id,
						proof.notebook_number,
						&proof_tip,
					)
					.await?;

					// record into the final changeset
					changes_with_proofs[change_index as usize].previous_balance_proof =
						Some(BalanceProof {
							balance: proof.balance,
							notary_id: proof.notary_id,
							notebook_number: proof.notebook_number,
							account_origin: proof.account_origin.clone(),
							notebook_proof: Some(notebook_proof),
							tick: proof.tick,
						});
				}

				if let Some(notebook_proof) = &proof.notebook_proof {
					ensure!(
						NotebookStore::is_valid_proof(
							&mut *tx,
							&proof_tip,
							proof.notebook_number,
							notebook_proof
						)
						.await?,
						Error::InvalidBalanceProof
					);
				}
			}

			let mut channel_hold_note = None;
			for (note_index, note) in change.notes.into_iter().enumerate() {
				let note_index = note_index as u32;
				match note.note_type {
					NoteType::ClaimFromMainchain { transfer_id, .. } => {
						chain_transfers += 1;
						// NOTE: transfers can expire. We need to ensure this can still get into a
						// notebook
						ChainTransferStore::take_and_record_transfer_local(
							&mut tx,
							current_notebook_number,
							tick,
							&account_id,
							transfer_id,
							note.microgons,
							change_index,
							note_index,
						)
						.await
					},
					NoteType::SendToMainchain => {
						chain_transfers += 1;
						ChainTransferStore::record_transfer_to_mainchain(
							&mut tx,
							current_notebook_number,
							&account_id,
							note.microgons,
						)
						.await
						.map(|_| ())
					},
					NoteType::ChannelHold { .. } => {
						channel_hold_note = Some(note.clone());
						Ok(())
					},
					NoteType::ChannelHoldSettle => {
						channel_hold_note = None;
						Ok(())
					},
					NoteType::Tax => {
						tax += note.microgons;
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
				&mut tx,
				&account_id,
				account_type,
				change_number,
				balance,
				current_notebook_number,
				tick,
				account_origin,
				channel_hold_note,
				previous_balance,
				prev_channel_hold_note,
			)
			.await?;
		}

		verify_voting_sources(&block_votes, tick, operator_account_id)?;

		NotebookConstraintsStore::try_increment(
			&mut tx,
			current_notebook_number,
			NotarizationCounts {
				balance_changes: changes_with_proofs.len() as u32,
				block_votes: block_votes.len() as u32,
				domains: domains.len() as u32,
				chain_transfers,
			},
			MaxNotebookCounts::default(),
		)
		.await?;

		NotarizationsStore::append_to_notebook(
			&mut *tx,
			current_notebook_number,
			sequence_number,
			changes_with_proofs,
			block_votes,
			domains,
		)
		.await?;

		tx.commit().await?;

		notary_metrics.on_notarization(balance_change_len, block_votes_len, domains_len, tax);

		Ok(BalanceChangeResult {
			notebook_number: current_notebook_number,
			tick,
			new_account_origins: new_account_origins
				.into_iter()
				.map(|(localchain_account_id, origin)| {
					NewAccountOrigin::new(
						localchain_account_id.account_id,
						localchain_account_id.account_type,
						origin.account_uid,
					)
				})
				.collect(),
		})
	}

	pub async fn next_sequence_number<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<u32, Error> {
		let next = sqlx::query_scalar!(
			"SELECT nextval('notar_id_seq_' || $1::TEXT)",
			(notebook_number % 5u32) as i32
		)
		.fetch_one(db)
		.await?
		.ok_or(Error::InternalError("Unable to get next sequence number".to_string()))?;

		Ok(next as u32)
	}

	pub async fn reset_seq<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<(), Error> {
		sqlx::query!(
			r#"
				SELECT setval('notar_id_seq_' || $1::TEXT, 1, false)
			"#,
			(notebook_number % 5u32) as i32
		)
		.fetch_optional(db)
		.await?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use polkadot_sdk::*;
	use sp_core::{bounded_vec, ed25519::Signature};
	use sp_keyring::Sr25519Keyring::{Bob, Ferdie};
	use sqlx::PgPool;

	use argon_primitives::{
		AccountType, AccountType::Deposit, BalanceChange, BlockVote, Domain, DomainTopLevel,
		Notarization, Note, NoteType,
	};

	use crate::stores::notarizations::NotarizationsStore;

	#[sqlx::test]
	async fn test_storage(pool: PgPool) -> anyhow::Result<()> {
		sqlx::query!("ALTER TABLE notarizations DROP CONSTRAINT IF EXISTS notarizations_notebook_number_fkey")
			.execute(&pool)
			.await?;
		let notebook_number = 1;
		let changeset = vec![
			BalanceChange {
				account_id: Bob.to_account_id(),
				account_type: Deposit,
				change_number: 0,
				balance: 1000,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![Note::create(
					1000,
					NoteType::ClaimFromMainchain { transfer_id: 1 }
				)],
				signature: Signature::from_raw([0u8; 64]).into(),
			},
			BalanceChange {
				account_id: Ferdie.to_account_id(),
				account_type: Deposit,
				change_number: 4,
				balance: 1000,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![Note::create(1000, NoteType::Claim,)],
				signature: Signature::from_raw([0u8; 64]).into(),
			},
		];

		let block_votes = vec![BlockVote {
			block_hash: [0u8; 32].into(),
			power: 1222,
			tick: 1,
			account_id: Bob.to_account_id(),
			index: 0,
			block_rewards_account_id: Bob.to_account_id(),
			signature: Signature::from_raw([0u8; 64]).into(),
		}
		.sign(Bob.pair())
		.clone()];
		let domains =
			vec![(Domain::new("test", DomainTopLevel::Analytics).hash(), Bob.to_account_id())];

		let next_seq = NotarizationsStore::next_sequence_number(&pool, notebook_number).await?;
		{
			let mut tx = pool.begin().await?;
			NotarizationsStore::append_to_notebook(
				&mut *tx,
				notebook_number,
				next_seq,
				changeset.clone(),
				block_votes.clone(),
				domains.clone(),
			)
			.await
			.unwrap();
			tx.commit().await?;
		}

		let notarization = Notarization::new(changeset, block_votes, domains);
		{
			let mut tx = pool.begin().await?;
			let result =
				NotarizationsStore::get_for_notebook(&mut *tx, notebook_number).await.unwrap();
			assert_eq!(result, vec![notarization.clone()]);
			tx.commit().await?;
		}

		{
			let mut tx = pool.begin().await?;
			let result = NotarizationsStore::get_account_change(
				&mut tx,
				notebook_number,
				Ferdie.to_account_id(),
				AccountType::Deposit,
				4,
			)
			.await
			.unwrap();
			assert_eq!(result, notarization.clone());

			assert!(NotarizationsStore::get_account_change(
				&mut tx,
				notebook_number,
				Ferdie.to_account_id(),
				AccountType::Deposit,
				3,
			)
			.await
			.is_err());
			tx.commit().await?;
		}

		Ok(())
	}
}
