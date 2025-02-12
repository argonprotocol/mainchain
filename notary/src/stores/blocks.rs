use argon_notary_audit::VerifyError;
use argon_primitives::{ensure, NotebookAuditResult};
use chrono::Utc;
use serde_json::json;
use sp_core::H256;
use sqlx::{postgres::PgDatabaseError, FromRow, PgConnection};

use crate::Error;

pub struct BlocksStore;

#[derive(Debug, FromRow)]
pub struct BlockRow {
	pub block_hash: Vec<u8>,
	pub parent_hash: Vec<u8>,
	pub block_number: i32,
	pub block_vote_minimum: String,
	pub latest_notebook_number: Option<i32>,
	pub is_finalized: bool,
	pub notebook_digests: Option<serde_json::Value>,
	pub finalized_time: Option<chrono::DateTime<Utc>>,
	pub received_time: chrono::DateTime<Utc>,
}

impl BlocksStore {
	pub async fn lock(db: &mut PgConnection) -> anyhow::Result<(), crate::Error> {
		let _ = sqlx::query_scalar!(
			"SELECT key FROM block_sync_lock where key = 1 FOR UPDATE NOWAIT LIMIT 1"
		)
		.fetch_one(db)
		.await?;

		Ok(())
	}
	pub async fn get_latest_finalized_block_number(
		db: &mut PgConnection,
	) -> anyhow::Result<u32, crate::Error> {
		let row = sqlx::query_scalar!(
			r#"
		SELECT COALESCE(MAX(block_number), 0) FROM blocks WHERE is_finalized=true;
		"#,
		)
		.fetch_one(db)
		.await?
		.unwrap_or_default();
		Ok(row as u32)
	}

	pub async fn get_latest_block_number(
		db: &mut PgConnection,
	) -> anyhow::Result<u32, crate::Error> {
		let row = sqlx::query_scalar!(
			r#"
		SELECT COALESCE(MAX(block_number), 0) FROM blocks;
		"#,
		)
		.fetch_one(db)
		.await?
		.unwrap_or_default();
		Ok(row as u32)
	}

	pub async fn get_by_hash(
		db: &mut PgConnection,
		block_hash: H256,
	) -> anyhow::Result<BlockRow, Error> {
		let row = sqlx::query_as!(
			BlockRow,
			r#"
		SELECT * FROM blocks WHERE block_hash=$1 LIMIT 1;
		"#,
			block_hash.0.to_vec()
		)
		.fetch_one(db)
		.await?;
		Ok(row)
	}

	pub async fn has_block(db: &mut PgConnection, block_hash: H256) -> anyhow::Result<bool, Error> {
		let row = sqlx::query_scalar!(
			r#"
			SELECT 1 as true FROM blocks WHERE block_hash=$1 LIMIT 1;
			"#,
			block_hash.0.to_vec()
		)
		.fetch_optional(db)
		.await?;
		Ok(row.is_some())
	}

	pub async fn get_parent_hash(
		db: &mut PgConnection,
		block_hash: H256,
	) -> anyhow::Result<H256, Error> {
		let row = sqlx::query_scalar!(
			r#"
		SELECT parent_hash FROM blocks WHERE block_hash=$1 LIMIT 1;
		"#,
			block_hash.0.to_vec()
		)
		.fetch_one(db)
		.await?;
		Ok(H256::from_slice(&row[..]))
	}

	pub async fn record_finalized(
		db: &mut PgConnection,
		block_hash: H256,
	) -> anyhow::Result<(), crate::Error> {
		let res = sqlx::query!(
			r#"
			UPDATE blocks SET finalized_time=$1, is_finalized=true
			WHERE block_hash = $2 and is_finalized=false
		"#,
			Utc::now(),
			block_hash.0.to_vec(),
		)
		.execute(db)
		.await?;

		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to mark block finalized".to_string())
		);
		Ok(())
	}

	pub async fn record(
		db: &mut PgConnection,
		block_number: u32,
		block_hash: H256,
		parent_hash: H256,
		notebook_digests: Vec<NotebookAuditResult<VerifyError>>,
	) -> anyhow::Result<(), Error> {
		let latest_notebook_number =
			notebook_digests.iter().map(|x| x.notebook_number as i32).max();

		let digests = json!(notebook_digests);
		let res = sqlx::query!(
			r#"
			INSERT INTO blocks (block_hash, block_number, parent_hash, block_vote_minimum, received_time, is_finalized,
				latest_notebook_number, notebook_digests)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
		"#,
			block_hash.0.to_vec(),
			block_number as i32,
			parent_hash.0.to_vec(),
			"0",
			Utc::now(),
			false,
			latest_notebook_number,
			digests,
		)
		.execute(db)
		.await;

		match res {
			Ok(res) => {
				ensure!(
					res.rows_affected() == 1,
					Error::InternalError("Unable to insert block".to_string())
				);
				Ok(())
			},
			Err(sqlx::Error::Database(db_err)) => {
				let pg_err = db_err.downcast_ref::<PgDatabaseError>();
				const UNIQUE_VIOLATION: &str = "23505"; // PostgreSQL error code for unique_violation

				if pg_err.code() == UNIQUE_VIOLATION {
					return Ok(())
				}

				Err(Error::Database(db_err.to_string()))
			},
			Err(e) => Err(e.into()),
		}
	}
}

#[cfg(test)]
mod tests {
	use argon_notary_audit::VerifyError;
	use argon_primitives::NotebookAuditResult;
	use sp_core::H256;
	use sqlx::PgPool;

	use crate::stores::blocks::BlocksStore;

	#[sqlx::test]
	async fn test_storage(pool: PgPool) -> anyhow::Result<()> {
		{
			let mut tx = pool.begin().await?;
			BlocksStore::record(&mut tx, 0, H256::from_slice(&[1u8; 32]), H256::zero(), vec![])
				.await?;
			BlocksStore::record(
				&mut tx,
				1,
				H256::from_slice(&[2u8; 32]),
				H256::from_slice(&[1u8; 32]),
				vec![NotebookAuditResult {
					notebook_number: 1,
					notary_id: 1,
					audit_first_failure: Some(VerifyError::AccountAlreadyHasChannelHold),
					tick: 1,
				}],
			)
			.await?;
			BlocksStore::record(
				&mut tx,
				2,
				H256::from_slice(&[3u8; 32]),
				H256::from_slice(&[2u8; 32]),
				vec![],
			)
			.await?;
			BlocksStore::record_finalized(&mut tx, H256::from_slice(&[3u8; 32])).await?;
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			let result = BlocksStore::get_latest_finalized_block_number(&mut tx).await?;
			assert_eq!(result, 2);
			tx.commit().await?;
		}

		Ok(())
	}

	#[sqlx::test]
	async fn test_ignores_duplicates(pool: PgPool) -> anyhow::Result<()> {
		let mut tx = pool.begin().await?;
		BlocksStore::record(&mut tx, 1, H256::from_slice(&[1u8; 32]), H256::zero(), vec![]).await?;
		tx.commit().await?;

		let mut tx = pool.begin().await?;
		BlocksStore::record(&mut tx, 1, H256::from_slice(&[1u8; 32]), H256::zero(), vec![]).await?;
		tx.commit().await?;

		let mut db = pool.acquire().await?;
		let block = BlocksStore::get_by_hash(&mut db, H256::from_slice(&[1u8; 32])).await?;
		assert_eq!(block.block_vote_minimum, "0");
		Ok(())
	}
}
