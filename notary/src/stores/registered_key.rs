use crate::ensure;
use sp_core::{ed25519, ByteArray};

use crate::error::Error;

pub struct RegisteredKeyStore {
	pub public: ed25519::Public,
}

impl RegisteredKeyStore {
	pub async fn store_public<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		public: ed25519::Public,
		finalized_block_number: u32,
	) -> anyhow::Result<(), Error> {
		let res = sqlx::query!(
			r#"
            INSERT INTO registered_keys (public, finalized_block_number)
            VALUES ($1, $2)
            "#,
			&public.0,
			finalized_block_number as i32,
		)
		.execute(db)
		.await?;
		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert registered keys".to_string())
		);
		Ok(())
	}

	pub async fn get_valid_public<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		block_number: u32,
	) -> anyhow::Result<ed25519::Public, Error> {
		let public = sqlx::query!(
			r#"
				SELECT public FROM registered_keys
				WHERE finalized_block_number <= $1
				ORDER BY finalized_block_number DESC
				LIMIT 1
            "#,
			block_number as i32
		)
		.fetch_one(db)
		.await?
		.public;

		Ok(ed25519::Public::from_slice(&public).unwrap())
	}
}
