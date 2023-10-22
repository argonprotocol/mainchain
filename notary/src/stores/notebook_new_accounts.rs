use codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{crypto::AccountId32, ByteArray, RuntimeDebug};
use sqlx::FromRow;

use ulx_notary_primitives::{AccountOrigin, Chain};

use crate::Error;

pub type AccountOriginUid = u32;
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
pub struct NotebookNewAccountsStore {
	pub origin: AccountOrigin,
	pub account_id: AccountId32,
	pub chain: Chain,
}

#[derive(FromRow)]
#[allow(dead_code)]
struct NotebookOriginsRow {
	pub notebook_number: i32,
	pub uid: i32,
	pub account_id: Vec<u8>,
	pub chain: i32,
}

impl TryInto<NotebookNewAccountsStore> for NotebookOriginsRow {
	type Error = Error;

	fn try_into(self) -> Result<NotebookNewAccountsStore, Self::Error> {
		Ok(NotebookNewAccountsStore {
			origin: AccountOrigin {
				notebook_number: self.notebook_number as u32,
				account_uid: self.uid as u32,
			},
			account_id: AccountId32::from_slice(&self.account_id.as_slice()).map_err(|_| {
				Error::InternalError(format!(
					"Unable to read account id from db {:?}",
					self.account_id
				))
			})?,
			chain: self.chain.try_into().map_err(Error::InternalError)?,
		})
	}
}

impl NotebookNewAccountsStore {
	pub async fn insert_origin<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: u32,
		account_id: &AccountId32,
		chain: &Chain,
	) -> anyhow::Result<AccountOrigin, Error> {
		let next = sqlx::query_scalar!(
				r#"
				INSERT INTO notebook_origins (notebook_number, uid, account_id, chain) VALUES ($1, nextval('uid_seq_' || $2::TEXT), $3, $4) RETURNING uid
				"#,
				notebook_number as i32,
				(notebook_number % 5u32) as i32,
				account_id.as_slice(),
				chain.clone() as i32,
			)
			.fetch_one(db)
			.await?;

		let next_number = next as u32;
		Ok(AccountOrigin { notebook_number, account_uid: next_number })
	}
	pub async fn take_notebook_origins<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: u32,
	) -> anyhow::Result<Vec<NotebookNewAccountsStore>, Error> {
		let rows = sqlx::query_as!(
			NotebookOriginsRow,
			r#"
				DELETE from notebook_origins WHERE notebook_number = $1 RETURNING *
				"#,
			notebook_number as i32,
		)
		.fetch_all(db)
		.await?;

		let entries: Result<Vec<NotebookNewAccountsStore>, Error> =
			rows.into_iter().map(TryInto::try_into).collect();

		Ok(entries?)
	}
	pub async fn reset_seq<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: u32,
	) -> anyhow::Result<(), Error> {
		sqlx::query!(
			r#"
				SELECT setval('uid_seq_' || $1::TEXT, 1, false)
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
	use crate::stores::notebook_new_accounts::NotebookNewAccountsStore;
	use sp_keyring::AccountKeyring::{Alice, Bob, Dave};
	use sqlx::PgPool;
	use ulx_notary_primitives::Chain::Argon;

	#[sqlx::test]
	async fn test_storage(pool: PgPool) -> anyhow::Result<()> {
		{
			let mut tx = pool.begin().await?;

			let origin = NotebookNewAccountsStore::insert_origin(
				&mut *tx,
				1,
				&Alice.to_account_id(),
				&Argon,
			)
			.await?;

			assert_eq!(origin.account_uid, 1);

			let origin2 =
				NotebookNewAccountsStore::insert_origin(&mut *tx, 1, &Bob.to_account_id(), &Argon)
					.await?;

			assert_eq!(origin2.account_uid, 2);

			// should restart at 1
			let origin =
				NotebookNewAccountsStore::insert_origin(&mut *tx, 2, &Dave.to_account_id(), &Argon)
					.await?;

			assert_eq!(origin.account_uid, 1);
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;

			assert!(matches!(
				NotebookNewAccountsStore::insert_origin(&mut *tx, 1, &Bob.to_account_id(), &Argon)
					.await,
				Err(crate::Error::Database(_))
			));

			tx.commit().await?
		}

		Ok(())
	}

	#[sqlx::test]
	async fn can_reset_the_sequences(pool: PgPool) -> anyhow::Result<()> {
		let mut tx = pool.begin().await?;

		let origin =
			NotebookNewAccountsStore::insert_origin(&mut *tx, 1, &Alice.to_account_id(), &Argon)
				.await?;

		assert_eq!(origin.account_uid, 1);

		// should reset 1
		NotebookNewAccountsStore::reset_seq(&mut *tx, 6).await?;
		let origin =
			NotebookNewAccountsStore::insert_origin(&mut *tx, 6, &Bob.to_account_id(), &Argon)
				.await?;

		assert_eq!(origin.account_uid, 1);
		tx.commit().await?;

		Ok(())
	}
}
