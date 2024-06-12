use crate::ensure;
use sp_core::{ed25519, ByteArray};
use ulx_primitives::tick::Tick;

use crate::error::Error;

pub struct RegisteredKeyStore {
	pub public: ed25519::Public,
}

impl RegisteredKeyStore {
	pub async fn store_public<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		public: ed25519::Public,
		effective_tick: Tick,
	) -> anyhow::Result<(), Error> {
		let res = sqlx::query!(
			r#"
            INSERT INTO registered_keys (public, effective_tick)
            VALUES ($1, $2)
            "#,
			&public.0,
			effective_tick as i32,
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
		tick: Tick,
	) -> anyhow::Result<ed25519::Public, Error> {
		let public = sqlx::query!(
			r#"
				SELECT public FROM registered_keys
				WHERE effective_tick <= $1
				ORDER BY effective_tick DESC
				LIMIT 1
            "#,
			tick as i32
		)
		.fetch_one(db)
		.await?
		.public;

		Ok(ed25519::Public::from_slice(&public).unwrap())
	}
}
