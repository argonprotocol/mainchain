use chrono::Utc;
use serde_json::{from_value, json};
use sp_core::{bounded::BoundedVec, H256};
use sqlx::{query, query_scalar, types::JsonValue, FromRow, PgConnection};

use ulx_notary_primitives::{
	ensure, AccountOrigin, ChainTransfer, NotaryId, NotebookHeader, NOTEBOOK_VERSION,
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
	pub finalized_block_number: Option<i32>,
	pub pinned_to_block_number: Option<i32>,
	pub starting_best_block_number: i32,
	pub start_time: chrono::DateTime<Utc>,
	pub end_time: Option<chrono::DateTime<Utc>>,
	pub notary_id: i32,
	pub chain_transfers: JsonValue,
	pub changed_accounts_root: Option<Vec<u8>>,
	pub changed_account_origins: JsonValue,
}

impl TryInto<NotebookHeader> for NotebookHeaderRow {
	type Error = Error;
	fn try_into(self) -> Result<NotebookHeader, Error> {
		Ok(NotebookHeader {
			version: self.version as u16,
			notebook_number: self.notebook_number as u32,
			finalized_block_number: self.finalized_block_number.unwrap_or(0i32) as u32,
			pinned_to_block_number: self.pinned_to_block_number.unwrap_or(0i32) as u32,
			start_time: self.start_time.timestamp_millis() as u64,
			end_time: self.end_time.map(|e| e.timestamp_millis() as u64).unwrap_or_default(),
			notary_id: self.notary_id as u32,
			chain_transfers: BoundedVec::truncate_from(from_value::<Vec<ChainTransfer>>(
				self.chain_transfers,
			)?),
			changed_accounts_root: H256::from_slice(
				&self.changed_accounts_root.unwrap_or_default()[..],
			),
			changed_account_origins: BoundedVec::truncate_from(from_value::<Vec<AccountOrigin>>(
				self.changed_account_origins,
			)?),
		})
	}
}

pub struct NotebookHeaderStore;

impl NotebookHeaderStore {
	async fn create_header<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notary_id: NotaryId,
		notebook_number: u32,
		best_block_number: u32,
	) -> anyhow::Result<(), Error> {
		let version = NOTEBOOK_VERSION;
		let empty = json!([]);
		let res = query!(
			r#"
			INSERT INTO notebook_headers (version, notary_id, notebook_number, starting_best_block_number, start_time, chain_transfers, changed_account_origins, changed_accounts_root) 
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
			"#,
			version as i16,
			notary_id as i32,
			notebook_number as i32,
			best_block_number as i32,
			Utc::now(),
			empty.clone(),
			empty,
			[0u8; 32].as_ref(),
		)
		.execute(db)
		.await?;
		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert notebook header".to_string())
		);
		Ok(())
	}
	pub async fn get_pinned_block_number<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: u32,
	) -> anyhow::Result<u32, Error> {
		let result = query_scalar!(
			r#"
			SELECT pinned_to_block_number FROM notebook_headers WHERE notebook_number = $1 LIMIT 1
			"#,
			notebook_number as i32
		)
		.fetch_one(db)
		.await?;

		if let Some(result) = result {
			return Ok(result as u32)
		}

		Err(Error::InternalError(format!(
			"Notebook pinned_to_block_number unset for notebook  {}",
			notebook_number
		)))
	}

	/// Creates the next notebook header and returns the current notebook id.
	///
	/// NOTE: there might still be some uncommitted changes in the notebook. Must wait for the row
	/// to have no more share locks
	pub fn create(
		db: &mut PgConnection,
		notary_id: NotaryId,
		notebook_number: u32,
		best_block_number: u32,
	) -> BoxFutureResult<()> {
		Box::pin(async move {
			Self::create_header(&mut *db, notary_id, notebook_number, best_block_number).await?;
			NotebookNewAccountsStore::reset_seq(&mut *db, notebook_number).await?;
			NotebookStatusStore::create(&mut *db, notebook_number).await?;
			Ok(())
		})
	}

	pub async fn load<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: u32,
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

		println!("record: {:?}", record.chain_transfers);

		Ok(record.try_into()?)
	}

	pub async fn get_changed_accounts_root<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: u32,
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
		notebook_number: u32,
		transfers: Vec<ChainTransfer>,
		pinned_to_block_number: u32,
		changed_accounts_root: H256,
		account_changelist: Vec<AccountOrigin>,
	) -> BoxFutureResult<()> {
		Box::pin(async move {
			let mut header = Self::load(&mut *db, notebook_number).await?;
			header.chain_transfers = BoundedVec::try_from(transfers).map_err(|_| {
				Error::InternalError(
					"Unable to decode chain transfers. Possibly exceeded max size.".to_string(),
				)
			})?;
			header.pinned_to_block_number = pinned_to_block_number;
			header.changed_accounts_root = changed_accounts_root;
			header.changed_account_origins = BoundedVec::truncate_from(account_changelist.clone());

			let hash = header.hash().0;

			let res = sqlx::query!(
				r#"
				UPDATE notebook_headers 
				SET hash = $1, changed_accounts_root = $2, changed_account_origins = $3, 
					chain_transfers = $4, end_time = $5, pinned_to_block_number = $6 
				WHERE notebook_number = $7
			"#,
				&hash,
				changed_accounts_root.as_bytes(),
				json!(header.changed_account_origins.to_vec()),
				json!(header.chain_transfers.to_vec()),
				Utc::now(),
				pinned_to_block_number as i32,
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
	use tracing::debug;

	use ulx_notary_primitives::{AccountOrigin, ChainTransfer, NOTEBOOK_VERSION};

	use crate::{error::Error, stores::notebook_header::NotebookHeaderStore};

	#[sqlx::test]
	async fn test_storage(pool: PgPool) -> anyhow::Result<()> {
		let notebook_number = 1;
		{
			let mut tx = pool.begin().await?;

			let _ = NotebookHeaderStore::create(&mut *tx, 1, notebook_number, 101).await?;

			let loaded = NotebookHeaderStore::load(&mut *tx, notebook_number).await?;
			assert_eq!(loaded.notebook_number, notebook_number);
			assert_eq!(loaded.version, NOTEBOOK_VERSION);
			assert_eq!(loaded.finalized_block_number, 0);
			assert_eq!(loaded.pinned_to_block_number, 0);
			assert!(loaded.start_time as i64 >= Utc::now().timestamp_millis() - 1000);
			assert_eq!(loaded.notary_id, 1);
			assert_eq!(loaded.chain_transfers.len(), 0);

			tx.commit().await?;

			assert_eq!(
				NotebookHeaderStore::get_changed_accounts_root(&pool, notebook_number).await?,
				[0u8; 32].into()
			);
			assert!(matches!(
				NotebookHeaderStore::get_pinned_block_number(&pool, notebook_number).await,
				Err(Error::InternalError(_))
			));
		}

		Ok(())
	}

	#[sqlx::test]
	async fn test_close(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let notebook_number = 1;
		{
			let mut tx = pool.begin().await?;

			let _ = NotebookHeaderStore::create(&mut *tx, 1, notebook_number, 101).await?;

			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			NotebookHeaderStore::complete_notebook(
				&mut *tx,
				notebook_number,
				vec![
					ChainTransfer::ToLocalchain { account_id: Bob.to_account_id(), nonce: 1 },
					ChainTransfer::ToMainchain { account_id: Alice.to_account_id(), amount: 100 },
				],
				100,
				[1u8; 32].into(),
				vec![
					AccountOrigin { notebook_number: 1, account_uid: 1 },
					AccountOrigin { notebook_number: 1, account_uid: 2 },
				],
			)
			.await?;
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			let header = NotebookHeaderStore::load(&mut *tx, notebook_number).await?;
			tx.commit().await?;
			debug!("header: {:?}", header);
			assert_eq!(header.chain_transfers.len(), 2);
			assert_eq!(
				header.chain_transfers[0],
				ChainTransfer::ToLocalchain { account_id: Bob.to_account_id(), nonce: 1 }
			);
			assert_eq!(
				header.chain_transfers[1],
				ChainTransfer::ToMainchain { account_id: Alice.to_account_id(), amount: 100 }
			);

			assert_eq!(header.pinned_to_block_number, 100);
			assert_eq!(header.changed_accounts_root, [1u8; 32].into());
			assert_eq!(header.changed_account_origins.len(), 2);
			assert_eq!(header.changed_account_origins[0].account_uid, 1);
			assert_eq!(header.changed_account_origins[1].account_uid, 2);
		}
		Ok(())
	}
}
