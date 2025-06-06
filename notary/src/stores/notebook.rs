use polkadot_sdk::*;
use std::collections::{BTreeMap, BTreeSet};

use crate::{
	Error,
	notebook_closer::notary_sign,
	stores::{
		BoxFutureResult, chain_transfer::ChainTransferStore, notarizations::NotarizationsStore,
		notebook_header::NotebookHeaderStore, notebook_new_accounts::NotebookNewAccountsStore,
	},
};
use argon_primitives::{
	AccountId, AccountOrigin, AccountType, Balance, BalanceTip, BlockVote, ChainTransfer,
	LocalchainAccountId, MaxNotebookNotarizations, MerkleProof, NewAccountOrigin, Notarization,
	NotaryId, Note, NoteType, Notebook, NotebookNumber, ensure, tick::Tick,
};
use binary_merkle_tree::{Leaf, merkle_proof, merkle_root, verify_proof};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json};
use sp_core::{
	Blake2Hasher, H256, RuntimeDebug,
	bounded::BoundedVec,
	ed25519::{Public, Signature},
};
use sp_keystore::KeystorePtr;
use sqlx::PgConnection;
use tracing::info;

pub struct NotebookStore;
impl NotebookStore {
	/// Get proofs for a set of balance tips. This fn should retrieve from the database, not
	/// calculate.
	pub fn get_balance_proof<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		_notary_id: NotaryId,
		notebook_number: NotebookNumber,
		balance_tip: &'a BalanceTip,
	) -> BoxFutureResult<'a, MerkleProof> {
		Box::pin(async move {
			let rows = sqlx::query!(
				"SELECT change_merkle_leafs FROM notebooks WHERE notebook_number = $1 LIMIT 1",
				notebook_number as i32
			)
			.fetch_one(db)
			.await
			.map_err(|_| Error::NotebookNotFinalized)?;

			let merkle_leafs = rows.change_merkle_leafs;

			let record = balance_tip.encode();

			let index = merkle_leafs
				.iter()
				.position(|x| *x == record)
				.ok_or(Error::InvalidBalanceProofRequested)?;

			let proof = merkle_proof::<Blake2Hasher, _, _>(&merkle_leafs, index as u32);

			Ok(MerkleProof {
				proof: BoundedVec::truncate_from(proof.proof.into_iter().collect()),
				leaf_index: index as u32,
				number_of_leaves: merkle_leafs.len() as u32,
			})
		})
	}

	pub async fn get_account_origin(
		db: &mut PgConnection,
		account_id: AccountId,
		account_type: AccountType,
	) -> anyhow::Result<AccountOrigin, Error> {
		let origin = json!([
			{
				"accountId": account_id,
				"accountType": account_type
			}
		]);
		let result = sqlx::query!(
			r#"
			SELECT new_account_origins, notebook_number FROM notebooks
			WHERE new_account_origins @> $1::jsonb
			ORDER BY notebook_number DESC LIMIT 1
			"#,
			origin
		)
		.fetch_one(db)
		.await
		.map_err(|_| Error::MissingAccountOrigin)?;

		let origins: Vec<NewAccountOrigin> = serde_json::from_value(result.new_account_origins)?;

		let origin = origins
			.iter()
			.find(|a| a.account_type == account_type && a.account_id == account_id)
			.ok_or(Error::MissingAccountOrigin)?;

		Ok(AccountOrigin {
			notebook_number: result.notebook_number as NotebookNumber,
			account_uid: origin.account_uid,
		})
	}

	pub fn get_account_origins<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> BoxFutureResult<'a, BoundedVec<NewAccountOrigin, MaxNotebookNotarizations>> {
		Box::pin(async move {
			let rows = sqlx::query!(
				"SELECT new_account_origins FROM notebooks WHERE notebook_number = $1 LIMIT 1",
				notebook_number as i32
			)
			.fetch_one(db)
			.await?;

			let new_account_origins: Vec<NewAccountOrigin> = from_value(rows.new_account_origins)?;

			Ok(BoundedVec::truncate_from(new_account_origins))
		})
	}

	pub async fn is_valid_proof<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		balance_tip: &'a BalanceTip,
		notebook_number: NotebookNumber,
		notebook_proof: &'a MerkleProof,
	) -> anyhow::Result<bool, Error> {
		let root = NotebookHeaderStore::get_changed_accounts_root(db, notebook_number).await?;

		let is_valid = verify_proof::<Blake2Hasher, _, _>(
			&root,
			notebook_proof.proof.clone().into_inner(),
			notebook_proof.number_of_leaves,
			notebook_proof.leaf_index,
			Leaf::Value(&balance_tip.encode()),
		);

		Ok(is_valid)
	}

	pub async fn load_finalized(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<Notebook, Error> {
		let header = NotebookHeaderStore::load_with_signature(&mut *db, notebook_number).await?;

		let notarizations = NotarizationsStore::get_for_notebook(&mut *db, notebook_number).await?;

		let rows = sqlx::query!(
			"SELECT new_account_origins, hash, signature FROM notebooks WHERE notebook_number = $1 LIMIT 1",
			notebook_number as i32
		)
		.fetch_one(db)
		.await?;
		let new_account_origins = from_value(rows.new_account_origins)?;

		Ok(Notebook {
			header: header.header,
			hash: H256::from_slice(&rows.hash),
			signature: Signature::try_from(&rows.signature[..])
				.map_err(|e| Error::InternalError(format!("Unable to read signature: {:?}", e)))?,
			notarizations: BoundedVec::truncate_from(notarizations),
			new_account_origins: BoundedVec::truncate_from(new_account_origins),
		})
	}

	pub async fn close_notebook(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
		tick: Tick,
		public: Public,
		operator_account_id: AccountId,
		keystore: &KeystorePtr,
	) -> anyhow::Result<NotebookBytes, Error> {
		let mut notarizations =
			NotarizationsStore::get_for_notebook(&mut *db, notebook_number).await?;

		let mut changed_accounts =
			BTreeMap::<LocalchainAccountId, (u32, Balance, AccountOrigin, Option<Note>)>::new();
		let mut block_votes = BTreeMap::<(AccountId, u32), BlockVote>::new();
		let new_account_origins =
			NotebookNewAccountsStore::take_notebook_origins(&mut *db, notebook_number).await?;

		let new_account_origin_map =
			BTreeMap::from_iter(new_account_origins.iter().map(|origin| {
				(
					LocalchainAccountId::new(origin.account_id.clone(), origin.account_type),
					AccountOrigin { notebook_number, account_uid: origin.account_uid },
				)
			}));

		let mut voting_power = 0u128;
		let mut tax = 0u128;
		let mut blocks_with_votes = BTreeSet::new();
		let mut domains = Vec::new();
		// NOTE: rebuild transfers list so it matches the final order
		let mut transfers = vec![];
		for change in notarizations.clone() {
			for change in change.balance_changes {
				let account_id = change.account_id;
				let localchain_account_id =
					LocalchainAccountId::new(account_id.clone(), change.account_type);
				let origin = change
					.previous_balance_proof
					.map(|a| a.account_origin)
					.or_else(|| new_account_origin_map.get(&localchain_account_id).cloned())
					.ok_or(|| {
						Error::InternalError(format!(
							"Could not find origin for account {:?}",
							localchain_account_id
						))
					})
					.map_err(|e| Error::InternalError(e().to_string()))?;

				let mut change_note = None;
				for note in change.notes {
					match note.note_type {
						NoteType::Tax | NoteType::LeaseDomain => tax += note.microgons,
						NoteType::ChannelHold { .. } => change_note = Some(note.clone()),
						NoteType::ChannelHoldSettle => change_note = None,
						NoteType::ClaimFromMainchain { transfer_id } =>
							transfers.push(ChainTransfer::ToLocalchain { transfer_id }),
						NoteType::SendToMainchain => transfers.push(ChainTransfer::ToMainchain {
							account_id: account_id.clone(),
							amount: note.microgons,
						}),
						_ => {},
					}
				}

				if !changed_accounts.contains_key(&localchain_account_id) ||
					changed_accounts
						.get(&localchain_account_id)
						.is_some_and(|a| a.0 < change.change_number)
				{
					changed_accounts.insert(
						localchain_account_id.clone(),
						(change.change_number, change.balance, origin, change_note),
					);
				}
			}
			for vote in change.block_votes {
				let block_hash = vote.block_hash;
				let key = (vote.account_id.clone(), vote.index);
				voting_power += vote.power;
				block_votes.insert(key, vote);
				blocks_with_votes.insert(block_hash);
			}
			for domain in change.domains {
				domains.push(domain);
			}
		}

		let mut account_changelist = vec![];
		let account_changes = changed_accounts.len();
		let merkle_leafs = changed_accounts
			.into_iter()
			.map(|(localchain_account_id, (nonce, balance, account_origin, channel_hold_note))| {
				account_changelist.push(account_origin.clone());
				BalanceTip {
					account_id: localchain_account_id.account_id,
					account_type: localchain_account_id.account_type,
					change_number: nonce,
					balance,
					account_origin,
					channel_hold_note,
				}
				.encode()
			})
			.collect::<Vec<_>>();

		let changes_root = merkle_root::<Blake2Hasher, _>(&merkle_leafs);
		if block_votes.is_empty() {
			let default_vote = BlockVote::create_default_vote(operator_account_id.clone(), tick);
			notarizations.push(Notarization::new(vec![], vec![default_vote.clone()], vec![]));
			let next_sequence =
				NotarizationsStore::next_sequence_number(&mut *db, notebook_number).await?;
			NotarizationsStore::append_to_notebook(
				&mut *db,
				notebook_number,
				next_sequence,
				vec![],
				vec![default_vote.clone()],
				vec![],
			)
			.await?;
			block_votes.insert((operator_account_id, 0), default_vote);
		}

		let block_votes_count = block_votes.len();

		let votes_merkle_leafs =
			block_votes.into_values().map(|vote| vote.encode()).collect::<Vec<_>>();
		let votes_root = merkle_root::<Blake2Hasher, _>(&votes_merkle_leafs);

		let _ = ChainTransferStore::take_for_notebook(&mut *db, notebook_number).await?;

		NotebookHeaderStore::complete_notebook(
			&mut *db,
			notebook_number,
			transfers,
			domains,
			tax,
			changes_root,
			account_changelist,
			votes_root,
			block_votes_count as u32,
			blocks_with_votes,
			voting_power,
			|hash| {
				notary_sign(keystore, &public, hash)
					.map_err(|e| Error::InternalError(format!("Unable to sign notebook: {:?}", e)))
			},
		)
		.await?;

		let new_account_origins = new_account_origins
			.iter()
			.map(|a| NewAccountOrigin {
				account_id: a.account_id.clone(),
				account_type: a.account_type,
				account_uid: a.account_uid,
			})
			.collect::<Vec<NewAccountOrigin>>();

		let signed_header =
			NotebookHeaderStore::load_with_signature(&mut *db, notebook_number).await?;
		info!(notebook_number, tick, block_votes_count, account_changes, "Notebook closed");
		let origins_json = json!(new_account_origins);
		let signed_header_bytes = signed_header.encode();

		let mut full_notebook =
			Notebook::build(signed_header.header, notarizations, new_account_origins);
		let hash = full_notebook.hash;
		full_notebook.signature = notary_sign(keystore, &public, &hash)?;

		let raw_body = full_notebook.encode();

		let res = sqlx::query!(
			r#"
				INSERT INTO notebooks (notebook_number, change_merkle_leafs, new_account_origins, hash, signature)
				VALUES ($1, $2, $3, $4, $5)
			"#,
			notebook_number as i32,
			merkle_leafs.as_slice(),
			origins_json,
			hash.as_bytes(),
			&full_notebook.signature.0[..]
		)
		.execute(db)
		.await?;
		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert notebook".to_string())
		);

		Ok(NotebookBytes { notebook: raw_body, signed_header: signed_header_bytes })
	}
}

pub struct NotebookBytes {
	pub notebook: Vec<u8>,
	pub signed_header: Vec<u8>,
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
struct AccountIdAndOrigin {
	pub key: [u8; 32],
	pub origin: AccountOrigin,
}
#[cfg(test)]
mod tests {
	use chrono::{Duration, Utc};
	use polkadot_sdk::*;
	use sp_core::{bounded_vec, ed25519::Signature};
	use sp_keyring::Sr25519Keyring::{Alice, Bob, Dave, Ferdie};
	use sp_keystore::{Keystore, testing::MemoryKeystore};
	use sqlx::PgPool;
	use std::ops::Add;

	use argon_primitives::{
		AccountOrigin, AccountType::Deposit, BalanceChange, BalanceTip, NewAccountOrigin,
	};

	use crate::{
		notebook_closer::NOTARY_KEYID,
		stores::{
			chain_transfer::ChainTransferStore, notarizations::NotarizationsStore,
			notebook::NotebookStore, notebook_header::NotebookHeaderStore,
			notebook_new_accounts::NotebookNewAccountsStore, registered_key::RegisteredKeyStore,
		},
	};

	#[sqlx::test]
	async fn test_close_notebook(pool: PgPool) -> anyhow::Result<()> {
		// Initialize the logger
		let _ = tracing_subscriber::fmt::try_init();
		let keystore = MemoryKeystore::new();
		let public = keystore.ed25519_generate_new(NOTARY_KEYID, None)?;

		let mut tx = pool.begin().await?;
		RegisteredKeyStore::store_public(&mut *tx, public, 1).await?;
		NotebookHeaderStore::create(
			&mut tx,
			1,
			1,
			1,
			Utc::now().add(Duration::try_minutes(1).unwrap()).timestamp_millis() as u64,
		)
		.await?;
		ChainTransferStore::record_transfer_to_local_from_block(
			&mut *tx,
			100,
			100,
			&Bob.to_account_id(),
			1,
			1000,
		)
		.await?;
		ChainTransferStore::take_and_record_transfer_local(
			&mut tx,
			1,
			1,
			&Bob.to_account_id(),
			1,
			1000,
			0,
			0,
		)
		.await?;
		NotarizationsStore::append_to_notebook(
			&mut *tx,
			1,
			1,
			vec![
				BalanceChange {
					account_id: Bob.to_account_id(),
					account_type: Deposit,
					change_number: 1,
					balance: 1000,
					previous_balance_proof: None,
					notes: bounded_vec![],
					channel_hold_note: None,
					signature: Signature::from_raw([0u8; 64]).into(),
				},
				BalanceChange {
					account_id: Alice.to_account_id(),
					account_type: Deposit,
					change_number: 1,
					balance: 2500,
					previous_balance_proof: None,
					notes: bounded_vec![],
					channel_hold_note: None,
					signature: Signature::from_raw([0u8; 64]).into(),
				},
				BalanceChange {
					account_id: Dave.to_account_id(),
					account_type: Deposit,
					change_number: 1,
					balance: 500,
					previous_balance_proof: None,
					notes: bounded_vec![],
					channel_hold_note: None,
					signature: Signature::from_raw([0u8; 64]).into(),
				},
			],
			vec![],
			vec![],
		)
		.await?;
		NotebookNewAccountsStore::insert_origin(&mut *tx, 1, &Bob.to_account_id(), &Deposit)
			.await?;
		NotebookNewAccountsStore::insert_origin(&mut *tx, 1, &Alice.to_account_id(), &Deposit)
			.await?;
		NotebookNewAccountsStore::insert_origin(&mut *tx, 1, &Dave.to_account_id(), &Deposit)
			.await?;
		tx.commit().await?;

		let mut tx = pool.begin().await?;
		NotebookStore::close_notebook(
			&mut tx,
			1,
			1,
			public,
			Ferdie.to_account_id(),
			&keystore.into(),
		)
		.await?;
		tx.commit().await?;

		let balance_tip = BalanceTip {
			account_id: Bob.to_account_id(),
			account_type: Deposit,
			change_number: 1,
			balance: 1000,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			channel_hold_note: None,
		};
		let proof = NotebookStore::get_balance_proof(&pool, 1, 1, &balance_tip).await?;

		assert_eq!(proof.number_of_leaves, 3);

		assert!(NotebookStore::is_valid_proof(&pool, &balance_tip, 1, &proof).await?);

		assert_eq!(
			NotebookStore::get_account_origins(&pool, 1).await?.into_inner(),
			vec![
				NewAccountOrigin::new(Bob.to_account_id(), Deposit, 1),
				NewAccountOrigin::new(Alice.to_account_id(), Deposit, 2),
				NewAccountOrigin::new(Dave.to_account_id(), Deposit, 3),
			]
		);

		let mut db = pool.acquire().await?;
		assert_eq!(
			NotebookStore::get_account_origin(&mut db, Bob.to_account_id(), Deposit).await?,
			AccountOrigin { notebook_number: 1, account_uid: 1 }
		);

		Ok(())
	}
}
