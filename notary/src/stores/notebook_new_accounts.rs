use codec::{Decode, Encode};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ByteArray, RuntimeDebug};
use sqlx::FromRow;

use argon_primitives::{AccountId, AccountOriginUid, AccountType, NotebookNumber};

use crate::Error;

pub struct NotebookNewAccountsStore;
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
pub struct NotebookNewAccount {
	pub notebook_number: NotebookNumber,
	pub account_uid: AccountOriginUid,
	pub account_id: AccountId,
	pub account_type: AccountType,
}

#[derive(FromRow)]
#[allow(dead_code)]
struct NotebookOriginsRow {
	pub notebook_number: i32,
	pub uid: i32,
	pub account_id: Vec<u8>,
	pub account_type: i32,
}

impl TryInto<NotebookNewAccount> for NotebookOriginsRow {
	type Error = Error;

	fn try_into(self) -> Result<NotebookNewAccount, Self::Error> {
		Ok(NotebookNewAccount {
			notebook_number: self.notebook_number as u32,
			account_uid: self.uid as u32,
			account_id: AccountId::from_slice(self.account_id.as_slice()).map_err(|_| {
				Error::InternalError(format!(
					"Unable to read account id from db {:?}",
					self.account_id
				))
			})?,
			account_type: self.account_type.try_into().map_err(Error::InternalError)?,
		})
	}
}

impl NotebookNewAccountsStore {
	pub async fn insert_origin<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
		account_id: &AccountId,
		account_type: &AccountType,
	) -> anyhow::Result<AccountOriginUid, Error> {
		let next = sqlx::query_scalar!(
				r#"
				INSERT INTO notebook_origins (notebook_number, uid, account_id, account_type) VALUES ($1, nextval('uid_seq_' || $2::TEXT), $3, $4) RETURNING uid
				"#,
				notebook_number as i32,
				(notebook_number % 5u32) as i32,
				account_id.as_slice(),
				account_type.clone() as i32,
			)
			.fetch_one(db)
			.await?;

		let next_number = next as u32;
		Ok(next_number)
	}

	pub async fn take_notebook_origins<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<Vec<NotebookNewAccount>, Error> {
		let rows = sqlx::query_as!(
			NotebookOriginsRow,
			r#"
				DELETE from notebook_origins WHERE notebook_number = $1 RETURNING *
				"#,
			notebook_number as i32,
		)
		.fetch_all(db)
		.await?;

		let entries: Result<Vec<NotebookNewAccount>, Error> =
			rows.into_iter().map(TryInto::try_into).collect();

		entries
	}
	pub async fn reset_seq<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
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
	use polkadot_sdk::*;
	use sp_keyring::Sr25519Keyring::{Alice, Bob, Dave};
	use sqlx::PgPool;

	use argon_primitives::AccountType::Deposit;

	use crate::stores::notebook_new_accounts::NotebookNewAccountsStore;

	#[sqlx::test]
	async fn test_storage(pool: PgPool) -> anyhow::Result<()> {
		sqlx::query!(
			"ALTER TABLE notebook_origins DROP CONSTRAINT IF EXISTS notebook_origins_notebook_number_fkey"
		)
		.execute(&pool)
		.await?;
		{
			let mut tx = pool.begin().await?;

			let uid = NotebookNewAccountsStore::insert_origin(
				&mut *tx,
				1,
				&Alice.to_account_id(),
				&Deposit,
			)
			.await?;

			assert_eq!(uid, 1);

			let origin2 = NotebookNewAccountsStore::insert_origin(
				&mut *tx,
				1,
				&Bob.to_account_id(),
				&Deposit,
			)
			.await?;

			assert_eq!(origin2, 2);

			// should restart at 1
			let origin = NotebookNewAccountsStore::insert_origin(
				&mut *tx,
				2,
				&Dave.to_account_id(),
				&Deposit,
			)
			.await?;

			assert_eq!(origin, 1);
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;

			assert!(matches!(
				NotebookNewAccountsStore::insert_origin(
					&mut *tx,
					1,
					&Bob.to_account_id(),
					&Deposit
				)
				.await,
				Err(crate::Error::Database(_))
			));

			tx.commit().await?
		}

		Ok(())
	}

	#[sqlx::test]
	async fn can_reset_the_sequences(pool: PgPool) -> anyhow::Result<()> {
		sqlx::query!(
			"ALTER TABLE notebook_origins DROP CONSTRAINT IF EXISTS notebook_origins_notebook_number_fkey"
		)
		.execute(&pool)
		.await?;
		let mut tx = pool.begin().await?;

		let origin =
			NotebookNewAccountsStore::insert_origin(&mut *tx, 1, &Alice.to_account_id(), &Deposit)
				.await?;

		assert_eq!(origin, 1);

		// should reset 1
		NotebookNewAccountsStore::reset_seq(&mut *tx, 6).await?;
		let origin =
			NotebookNewAccountsStore::insert_origin(&mut *tx, 6, &Bob.to_account_id(), &Deposit)
				.await?;

		assert_eq!(origin, 1);
		tx.commit().await?;

		Ok(())
	}
}
