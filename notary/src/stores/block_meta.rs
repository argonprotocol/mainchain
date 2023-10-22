use sp_core::H256;
use sqlx::{FromRow, PgConnection, PgPool};

use crate::{error::Error, stores::BoxFutureResult};
use ulx_notary_primitives::ensure;

#[derive(Debug, Clone)]
pub struct BlockMetaStore;

#[derive(Debug, Clone, Default)]
pub struct BlockMeta {
	pub key: u32,
	pub best_block_number: u32,
	pub best_block_hash: [u8; 32],
	pub finalized_block_number: u32,
	pub finalized_block_hash: [u8; 32],
}

#[derive(FromRow)]
struct BlockMetaRow {
	pub key: i32,
	pub best_block_number: i32,
	pub best_block_hash: Vec<u8>,
	pub finalized_block_number: i32,
	pub finalized_block_hash: Vec<u8>,
}

impl TryInto<BlockMeta> for BlockMetaRow {
	type Error = Error;

	fn try_into(self) -> Result<BlockMeta, Self::Error> {
		Ok(BlockMeta {
			key: self.key as u32,
			best_block_number: self.best_block_number as u32,
			best_block_hash: self.best_block_hash.try_into().map_err(|_| {
				Error::InternalError("Unable to convert stored best block hash".to_string())
			})?,
			finalized_block_number: self.finalized_block_number as u32,
			finalized_block_hash: self.finalized_block_hash.try_into().map_err(|_| {
				Error::InternalError("Unable to convert stored finalized block hash".to_string())
			})?,
		})
	}
}

impl BlockMetaStore {
	pub fn start(pool: &PgPool, genesis_block: H256) -> BoxFutureResult<()> {
		Box::pin(async move {
			if let Some(_) =
				sqlx::query!("SELECT 1 as exists FROM block_meta where key = 1 LIMIT 1")
					.fetch_optional(pool)
					.await?
			{
				return Ok(())
			}

			let res = sqlx::query!(
				r#"
				INSERT INTO block_meta (key, best_block_number, best_block_hash, finalized_block_number, finalized_block_hash)
				VALUES ($1, $2, $3, $4, $5)
			"#,
				1 as i32,
				0 as i32,
				genesis_block.as_ref(),
				0 as i32,
				genesis_block.as_ref(),
			)
			.execute(pool)
			.await?;

			ensure!(
				res.rows_affected() == 1,
				Error::InternalError("Unable to insert first block meta".to_string())
			);
			Ok(())
		})
	}

	pub fn load(db: &mut PgConnection) -> BoxFutureResult<BlockMeta> {
		Box::pin(async move {
			let block_meta =
				sqlx::query_as!(BlockMetaRow, "SELECT * FROM block_meta where key = 1 LIMIT 1")
					.fetch_one(db)
					.await?;
			Ok(block_meta.try_into()?)
		})
	}

	pub async fn lock(db: &mut PgConnection) -> anyhow::Result<BlockMeta, Error> {
		let row = sqlx::query_as!(
			BlockMetaRow,
			"SELECT * FROM block_meta where key = 1 FOR UPDATE NOWAIT LIMIT 1"
		)
		.fetch_one(db)
		.await?;
		Ok(row.try_into()?)
	}

	pub fn store_best_block(
		db: &mut PgConnection,
		best_block_number: u32,
		best_block_hash: [u8; 32],
	) -> BoxFutureResult<()> {
		Box::pin(async move {
			sqlx::query!(
				r#"
				UPDATE block_meta SET best_block_number = $1, best_block_hash = $2
				WHERE best_block_number < $1 AND key = 1
				"#,
				best_block_number as i32,
				best_block_hash.as_ref()
			)
			.execute(db)
			.await?;
			Ok(())
		})
	}

	pub fn store_finalized_block(
		db: &mut PgConnection,
		block_number: u32,
		block_hash: [u8; 32],
	) -> BoxFutureResult<()> {
		Box::pin(async move {
			sqlx::query!(
				r#"
				UPDATE block_meta SET finalized_block_number = $1, finalized_block_hash = $2
				WHERE finalized_block_number < $1 AND key = 1
				"#,
				block_number as i32,
				block_hash.as_ref()
			)
			.execute(db)
			.await?;
			Ok(())
		})
	}
}

#[cfg(test)]
mod tests {
	use crate::stores::block_meta::BlockMetaStore;
	use frame_support::assert_ok;
	use sqlx::PgPool;

	#[sqlx::test]
	async fn test_can_load(pool: PgPool) -> anyhow::Result<()> {
		BlockMetaStore::start(&pool, Default::default()).await?;

		let mut db = pool.begin().await?;
		assert_eq!(BlockMetaStore::load(&mut *db).await?.best_block_number, 0);

		Ok(())
	}

	#[sqlx::test]
	async fn test_can_store_finalized(pool: PgPool) -> anyhow::Result<()> {
		BlockMetaStore::start(&pool, Default::default()).await?;

		{
			let mut tx = pool.begin().await?;
			assert_ok!(BlockMetaStore::store_best_block(&mut *tx, 2, [2; 32],).await);
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			assert_ok!(BlockMetaStore::store_finalized_block(&mut *tx, 1, [1; 32],).await);
			tx.commit().await?;
		}

		let mut db = pool.acquire().await?;
		let loaded = BlockMetaStore::load(&mut *db).await?;
		assert_eq!(loaded.best_block_number, 2);
		assert_eq!(loaded.best_block_hash, [2; 32]);
		assert_eq!(loaded.finalized_block_number, 1);
		assert_eq!(loaded.finalized_block_hash, [1; 32]);

		Ok(())
	}
}
