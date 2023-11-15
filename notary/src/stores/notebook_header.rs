use std::collections::{BTreeSet, HashMap};

use chrono::Utc;
use serde_json::{from_value, json};
use sp_core::{blake2_256, bounded::BoundedVec, H256};
use sqlx::{query, types::JsonValue, FromRow, PgConnection};

use ulx_notary_primitives::{
	ensure, AccountOrigin, BestBlockNonce, BlockVotingPower, ChainTransfer, NotaryId,
	NotebookHeader, NotebookNumber, NOTEBOOK_VERSION,
};

use crate::{
	stores::{
		notebook_new_accounts::NotebookNewAccountsStore, notebook_status::NotebookStatusStore,
		BoxFutureResult,
	},
	Error,
};

#[derive(FromRow)]
#[allow(dead_code)]
struct NotebookHeaderRow {
	pub version: i32,
	pub notebook_number: i32,
	pub hash: Option<Vec<u8>>,
	pub block_number: i32,
	pub finalized_block_number: Option<i32>,
	pub start_time: chrono::DateTime<Utc>,
	pub end_time: Option<chrono::DateTime<Utc>>,
	pub notary_id: i32,
	pub tax: Option<String>,
	pub chain_transfers: JsonValue,
	pub changed_accounts_root: Vec<u8>,
	pub changed_account_origins: JsonValue,
	pub block_votes_root: Vec<u8>,
	pub block_votes_count: i32,
	pub block_voting_power: String,
	pub blocks_with_votes: Vec<Vec<u8>>,
	pub secret_hash: Vec<u8>,
	pub parent_secret: Option<Vec<u8>>,
	pub best_nonces: JsonValue,
	pub last_updated: chrono::DateTime<Utc>,
}

impl TryInto<NotebookHeader> for NotebookHeaderRow {
	type Error = Error;
	fn try_into(self) -> Result<NotebookHeader, Error> {
		Ok(NotebookHeader {
			version: self.version as u16,
			notebook_number: self.notebook_number as u32,
			block_number: self.block_number as u32,
			finalized_block_number: self
				.finalized_block_number
				.map(|a| a as u32)
				.unwrap_or_default(),
			start_time: self.start_time.timestamp_millis() as u64,
			end_time: self.end_time.map(|e| e.timestamp_millis() as u64).unwrap_or_default(),
			notary_id: self.notary_id as u32,
			tax: self
				.tax
				.unwrap_or("0".to_string())
				.parse::<u128>()
				.map_err(|e| Error::InternalError(e.to_string()))?,
			chain_transfers: BoundedVec::truncate_from(from_value(self.chain_transfers)?),
			changed_accounts_root: H256::from_slice(&self.changed_accounts_root[..]),
			changed_account_origins: BoundedVec::truncate_from(from_value(
				self.changed_account_origins,
			)?),
			block_votes_root: H256::from_slice(&self.block_votes_root[..]),
			block_votes_count: self.block_votes_count as u32,
			block_voting_power: self.block_voting_power.parse::<u128>().map_err(|e| {
				Error::InternalError(format!("Error decoding block voting power: {:?}", e))
			})?,
			blocks_with_votes: BoundedVec::truncate_from(
				self.blocks_with_votes
					.into_iter()
					.map(|a| H256::from_slice(a.as_slice()))
					.collect::<Vec<_>>(),
			),
			secret_hash: H256::from_slice(&self.secret_hash[..]),
			parent_secret: self.parent_secret.map(|a| H256::from_slice(&a[..])),
			best_block_nonces: from_value(self.best_nonces)?,
		})
	}
}

pub struct NotebookHeaderStore;

impl NotebookHeaderStore {
	async fn save_secret<'a>(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
		secret: H256,
	) -> anyhow::Result<(), Error> {
		let res = query!(
			r#"
			INSERT INTO notebook_secrets (notebook_number, secret) 
			VALUES ($1, $2)
			"#,
			notebook_number as i32,
			secret.as_bytes()
		)
		.execute(db)
		.await?;

		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert notebook secret".to_string())
		);
		Ok(())
	}

	async fn create_header<'a>(
		db: &mut PgConnection,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		block_number: u32,
	) -> anyhow::Result<(), Error> {
		let version = NOTEBOOK_VERSION;
		let empty = json!([]);
		let mut secret = [0u8; 32];
		getrandom::getrandom(&mut secret).map_err(|e| Error::InternalError(e.to_string()))?;
		let secret_hash = blake2_256(&secret[..]);

		Self::save_secret(db, notebook_number, H256::from_slice(&secret)).await?;
		let parent_secret = sqlx::query_scalar!(
			"SELECT secret FROM notebook_secrets WHERE notebook_number = $1 LIMIT 1",
			(notebook_number - 1) as i32
		)
		.fetch_optional(&mut *db)
		.await?;

		let res = query!(
			r#"
			INSERT INTO notebook_headers (version, notary_id, block_number, notebook_number, start_time, chain_transfers, 
				changed_account_origins, changed_accounts_root, secret_hash, parent_secret, block_votes_root, 
				block_voting_power, block_votes_count, best_nonces, blocks_with_votes)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
			"#,
			version as i16,
			notary_id as i32,
			block_number as i32,
			notebook_number as i32,
			Utc::now(),
			empty.clone(),
			empty.clone(),
			[0u8; 32].as_ref(),
			secret_hash.as_slice(),
			parent_secret,
			[0u8; 32].as_ref(),
			0.to_string(),
			0,
			empty.clone(),
			&[]
		)
		.execute(db)
		.await?;

		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert notebook header".to_string())
		);
		Ok(())
	}

	/// Creates the next notebook header and returns the current notebook id.
	///
	/// NOTE: there might still be some uncommitted changes in the notebook. Must wait for the row
	/// to have no more share locks
	pub fn create(
		db: &mut PgConnection,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		block_number: u32,
	) -> BoxFutureResult<()> {
		Box::pin(async move {
			Self::create_header(&mut *db, notary_id, notebook_number, block_number).await?;
			NotebookNewAccountsStore::reset_seq(&mut *db, notebook_number).await?;
			NotebookStatusStore::create(&mut *db, notebook_number).await?;
			Ok(())
		})
	}

	pub async fn load<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<NotebookHeader, Error> {
		let record = sqlx::query_as!(
			NotebookHeaderRow,
			r#"
				SELECT *
				FROM notebook_headers WHERE notebook_number = $1 LIMIT 1
				"#,
			notebook_number as i32
		)
		.fetch_one(db)
		.await?;

		Ok(record.try_into()?)
	}

	pub async fn get_block_number(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<u32, Error> {
		let row = sqlx::query_scalar!(
			"SELECT block_number FROM notebook_headers WHERE notebook_number = $1 LIMIT 1",
			notebook_number as i32
		)
		.fetch_one(db)
		.await?;
		Ok(row as u32)
	}

	pub async fn get_changed_accounts_root<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<H256, Error> {
		let record = sqlx::query_scalar!(
			r#"
				SELECT changed_accounts_root
				FROM notebook_headers WHERE notebook_number = $1 LIMIT 1
				"#,
			notebook_number as i32
		)
		.fetch_one(db)
		.await?;

		let data: [u8; 32] = record.try_into().map_err(|_| {
			Error::InternalError(format!(
				"Error decoding notebook accounts_root for {}",
				notebook_number
			))
		})?;
		Ok(data.into())
	}

	pub fn complete_notebook(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
		finalized_block_number: u32,
		transfers: Vec<ChainTransfer>,
		tax: u128,
		changed_accounts_root: H256,
		account_changelist: Vec<AccountOrigin>,
		block_votes_root: H256,
		block_votes_count: u32,
		blocks_with_votes: BTreeSet<H256>,
		block_voting_power: BlockVotingPower,
		best_nonces: HashMap<H256, Vec<BestBlockNonce>>,
	) -> BoxFutureResult<()> {
		Box::pin(async move {
			let mut header = Self::load(&mut *db, notebook_number).await?;
			header.chain_transfers = BoundedVec::try_from(transfers).map_err(|_| {
				Error::InternalError(
					"Unable to decode chain transfers. Possibly exceeded max size.".to_string(),
				)
			})?;
			header.changed_accounts_root = changed_accounts_root;
			// need to sort the changelist
			let account_changelist = BTreeSet::from_iter(account_changelist);
			header.changed_account_origins =
				BoundedVec::truncate_from(account_changelist.into_iter().collect::<Vec<_>>());

			let best_nonces = best_nonces
				.iter()
				.map(|a| a.1.iter().map(|b| (a.0.clone(), b.clone())))
				.flatten()
				.collect::<Vec<(H256, BestBlockNonce)>>();
			let hash = header.hash().0;

			let res = sqlx::query!(
				r#"
				UPDATE notebook_headers 
				SET hash = $1, 
					changed_accounts_root = $2, 
					changed_account_origins = $3, 
					chain_transfers = $4, 
					end_time = $5,
					tax=$6, 
					block_voting_power=$7,
					block_votes_root=$8, 
					block_votes_count=$9, 
					best_nonces=$10,
					finalized_block_number=$11,
					blocks_with_votes=$12
				WHERE notebook_number = $13
			"#,
				&hash,
				changed_accounts_root.as_bytes(),
				json!(header.changed_account_origins.to_vec()),
				json!(header.chain_transfers.to_vec()),
				Utc::now(),
				tax as i64,
				block_voting_power.to_string(),
				block_votes_root.as_bytes(),
				block_votes_count as i32,
				json!(best_nonces),
				finalized_block_number as i32,
				&blocks_with_votes.into_iter().map(|a| a.as_bytes().to_vec()).collect::<Vec<_>>()[..],
				notebook_number as i32,
			)
			.execute(db)
			.await?;

			ensure!(
				res.rows_affected() == 1,
				Error::InternalError("Notebook header not updated".to_string())
			);

			Ok(())
		})
	}
}

#[cfg(test)]
mod tests {
	use chrono::Utc;
	use sp_keyring::AccountKeyring::{Alice, Bob};
	use sqlx::PgPool;
	use tracing::{debug, info};

	use ulx_notary_primitives::{AccountOrigin, ChainTransfer, NOTEBOOK_VERSION};

	use crate::stores::notebook_header::NotebookHeaderStore;

	#[sqlx::test]
	async fn test_storage(pool: PgPool) -> anyhow::Result<()> {
		let notebook_number = 1;
		{
			let mut tx = pool.begin().await?;

			let _ = NotebookHeaderStore::create(&mut *tx, 1, notebook_number, 101).await?;

			let loaded = NotebookHeaderStore::load(&mut *tx, notebook_number).await?;
			assert_eq!(loaded.notebook_number, notebook_number);
			assert_eq!(loaded.version, NOTEBOOK_VERSION);
			assert_eq!(loaded.block_number, 101);
			assert!(loaded.start_time as i64 >= Utc::now().timestamp_millis() - 1000);
			assert_eq!(loaded.notary_id, 1);
			assert_eq!(loaded.chain_transfers.len(), 0);

			tx.commit().await?;

			assert_eq!(
				NotebookHeaderStore::get_changed_accounts_root(&pool, notebook_number).await?,
				[0u8; 32].into()
			);
		}

		Ok(())
	}

	#[sqlx::test]
	async fn test_close(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let notebook_number = 1;
		{
			let mut tx = pool.begin().await?;

			let _ = NotebookHeaderStore::create(&mut *tx, 1, notebook_number, 100).await?;

			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			NotebookHeaderStore::complete_notebook(
				&mut *tx,
				notebook_number,
				notebook_number,
				vec![
					ChainTransfer::ToLocalchain {
						account_id: Bob.to_account_id(),
						account_nonce: 1,
					},
					ChainTransfer::ToMainchain { account_id: Alice.to_account_id(), amount: 100 },
				],
				0,
				[1u8; 32].into(),
				vec![
					AccountOrigin { notebook_number: 1, account_uid: 1 },
					AccountOrigin { notebook_number: 1, account_uid: 2 },
				],
				[1u8; 32].into(),
				0,
				Default::default(),
				0,
				Default::default(),
			)
			.await?;
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			let header = NotebookHeaderStore::load(&mut *tx, notebook_number).await?;

			info!("step 2");
			debug!("header: {:?}", header);
			assert_eq!(header.chain_transfers.len(), 2);
			assert_eq!(
				header.chain_transfers[0],
				ChainTransfer::ToLocalchain { account_id: Bob.to_account_id(), account_nonce: 1 }
			);
			assert_eq!(
				header.chain_transfers[1],
				ChainTransfer::ToMainchain { account_id: Alice.to_account_id(), amount: 100 }
			);

			assert_eq!(header.block_number, 100);
			assert_eq!(header.changed_accounts_root, [1u8; 32].into());
			assert_eq!(header.changed_account_origins.len(), 2);
			assert_eq!(header.changed_account_origins[0].account_uid, 1);
			assert_eq!(header.changed_account_origins[1].account_uid, 2);
		}
		Ok(())
	}
}
