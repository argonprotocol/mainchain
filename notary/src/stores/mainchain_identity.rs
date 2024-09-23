use argon_primitives::{Chain, ChainIdentity};
use chrono::Utc;
use sp_core::H256;
use sqlx::{FromRow, PgConnection};
use std::str::FromStr;

use crate::Error;
pub struct MainchainIdentityStore;

#[derive(Debug, FromRow)]
pub struct MainchainIdentity {
	pub chain: String,
	pub genesis_hash: Vec<u8>,
	pub created_at: chrono::DateTime<Utc>,
}

impl TryFrom<MainchainIdentity> for ChainIdentity {
	type Error = Error;
	fn try_from(identity: MainchainIdentity) -> Result<Self, Self::Error> {
		Ok(ChainIdentity {
			chain: Chain::from_str(&identity.chain)
				.map_err(|_| Error::InternalError("Unable to decode chain".to_string()))?,
			genesis_hash: H256::from_slice(&identity.genesis_hash),
		})
	}
}

impl MainchainIdentityStore {
	pub(crate) async fn confirm_chain(
		db: &mut PgConnection,
		chain_identity: ChainIdentity,
	) -> anyhow::Result<(), Error> {
		let existing =
			sqlx::query_as!(MainchainIdentity, "SELECT * FROM mainchain_identity LIMIT 1",)
				.fetch_optional(&mut *db)
				.await?;
		if let Some(existing) = existing {
			let existing: ChainIdentity = existing.try_into()?;
			if existing == chain_identity {
				return Ok(());
			}
			return Err(Error::ChainMismatch);
		} else {
			let now = Utc::now();
			sqlx::query!(
				"INSERT INTO mainchain_identity (chain, genesis_hash, created_at) VALUES ($1, $2, $3)",
				chain_identity.chain.to_string(),
				chain_identity.genesis_hash.as_bytes().to_vec(),
				now,
			)
			.execute(&mut *db)
			.await?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use argon_primitives::Chain;
	use sp_core::H256;
	use sqlx::PgPool;

	#[sqlx::test]
	async fn test_chain_check(pool: PgPool) -> anyhow::Result<()> {
		let mut tx = pool.begin().await?;
		MainchainIdentityStore::confirm_chain(
			&mut tx,
			ChainIdentity { chain: Chain::Mainnet, genesis_hash: H256::from_slice(&[1u8; 32]) },
		)
		.await
		.expect("Failed to confirm chain");

		assert!(matches!(
			MainchainIdentityStore::confirm_chain(
				&mut tx,
				ChainIdentity { chain: Chain::Testnet, genesis_hash: H256::from_slice(&[1u8; 32]) }
			)
			.await,
			Err(Error::ChainMismatch)
		),);

		MainchainIdentityStore::confirm_chain(
			&mut tx,
			ChainIdentity { chain: Chain::Mainnet, genesis_hash: H256::from_slice(&[1u8; 32]) },
		)
		.await
		.expect("Failed to confirm chain");

		Ok(())
	}
}
