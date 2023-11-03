use crate::Error;
use chrono::Utc;
use sp_core::H256;
use sqlx::{FromRow, PgConnection};
use ulx_notary_primitives::ensure;

pub struct BlocksStore;

#[derive(Debug, FromRow)]
pub struct BlockRow {
	pub block_number: i32,
	pub block_hash: Vec<u8>,
	pub parent_hash: Vec<u8>,
	pub received_time: chrono::DateTime<Utc>,
}

impl BlocksStore {
	pub(crate) async fn get_hash(
		db: &mut PgConnection,
		block_number: u32,
	) -> anyhow::Result<Option<BlockRow>, crate::Error> {
		let row = sqlx::query_as!(
			BlockRow,
			r#"
		SELECT * FROM blocks where block_number=$1 LIMIT 1;
		"#,
			block_number as i32,
		)
		.fetch_optional(db)
		.await?;
		Ok(row)
	}

	pub async fn get_latest_finalized(
		db: &mut PgConnection,
	) -> anyhow::Result<Option<BlockRow>, crate::Error> {
		let row = sqlx::query_as!(
			BlockRow,
			r#"
		SELECT * FROM blocks ORDER BY block_number DESC LIMIT 1;
		"#,
		)
		.fetch_optional(db)
		.await?;
		Ok(row)
	}

	pub async fn record_finalized(
		db: &mut PgConnection,
		number: u32,
		hash: H256,
		parent_hash: H256,
	) -> anyhow::Result<(), crate::Error> {
		let res = sqlx::query!(
			r#"
		INSERT INTO blocks (block_number, block_hash, parent_hash, received_time) VALUES ($1, $2, $3, $4);
		"#,
			number as i32,
			hash.0.to_vec(),
			parent_hash.0.to_vec(),
			Utc::now(),
		)
		.execute(db)
		.await?;

		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert block".to_string())
		);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use sp_core::H256;
	use sqlx::PgPool;

	use crate::stores::blocks::BlocksStore;

	#[sqlx::test]
	async fn test_storage(pool: PgPool) -> anyhow::Result<()> {
		{
			let mut tx = pool.begin().await?;
			BlocksStore::record_finalized(&mut *tx, 0, H256::from_slice(&[1u8; 32]), H256::zero())
				.await?;
			BlocksStore::record_finalized(
				&mut *tx,
				1,
				H256::from_slice(&[2u8; 32]),
				H256::from_slice(&[1u8; 32]),
			)
			.await?;
			BlocksStore::record_finalized(
				&mut *tx,
				2,
				H256::from_slice(&[3u8; 32]),
				H256::from_slice(&[2u8; 32]),
			)
			.await?;
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			let result = BlocksStore::get_latest_finalized(&mut *tx).await?;
			assert_eq!(result.map(|a| a.block_number), Some(2));
			tx.commit().await?;
		}

		Ok(())
	}
	#[sqlx::test]
	async fn test_cant_overwrite(pool: PgPool) -> anyhow::Result<()> {
		let mut tx = pool.begin().await?;
		BlocksStore::record_finalized(&mut *tx, 1, H256::from_slice(&[1u8; 32]), H256::zero())
			.await?;
		assert!(BlocksStore::record_finalized(
			&mut *tx,
			1,
			H256::from_slice(&[1u8; 32]),
			H256::from_slice(&[1u8; 32])
		)
		.await
		.unwrap_err()
		.to_string()
		.contains("duplicate key"));

		Ok(())
	}
}
