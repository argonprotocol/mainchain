use sp_core::ByteArray;
use sqlx::{query, FromRow, PgConnection};

use ulx_primitives::{ensure, AccountId, ChainTransfer, NotebookNumber};

use crate::Error;

pub struct ChainTransferStore;

#[derive(Debug, Clone, Default, FromRow)]
#[allow(dead_code)]
struct ChainTransferRow {
	pub to_localchain: bool,
	pub amount: String,
	pub account_id: Vec<u8>,
	pub account_nonce: Option<i32>,
	pub finalized_block_number: Option<i32>,
	pub included_in_notebook_number: Option<i32>,
}
impl TryInto<ChainTransfer> for ChainTransferRow {
	type Error = Error;

	fn try_into(self) -> Result<ChainTransfer, Self::Error> {
		if self.to_localchain {
			Ok(ChainTransfer::ToLocalchain {
				account_id: AccountId::from_slice(&self.account_id.as_slice()).map_err(|_| {
					Error::InternalError(format!(
						"Unable to read account id from db {:?}",
						self.account_id
					))
				})?,
				account_nonce: self
					.account_nonce
					.map(|a| a as u32)
					.expect("account_nonce is required"),
			})
		} else {
			Ok(ChainTransfer::ToMainchain {
				account_id: AccountId::from_slice(&self.account_id.as_slice()).map_err(|_| {
					Error::InternalError(format!(
						"Unable to read account id from db {:?}",
						self.account_id
					))
				})?,
				amount: self
					.amount
					.parse::<u128>()
					.map_err(|e| Error::InternalError(e.to_string()))?,
			})
		}
	}
}

impl ChainTransferStore {
	/// Records a mainchain transfer that was included in a balance change + notebook.
	pub async fn record_transfer_to_mainchain(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
		account_id: &AccountId,
		milligons: u128,
	) -> anyhow::Result<(), Error> {
		let res = query!(
			r#"
			INSERT INTO chain_transfers (to_localchain, amount, account_id, included_in_notebook_number) 
			SELECT $1, $2, $3, $4
			"#,
			false,
			milligons.to_string(),
			account_id.as_slice(),
			notebook_number as i32,
		)
		.execute(db)
		.await?;
		ensure!(res.rows_affected() == 1, Error::MaxNotebookChainTransfersReached);

		Ok(())
	}

	pub async fn take_and_record_transfer_local(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
		account_id: &AccountId,
		account_nonce: u32,
		proposed_amount: u128,
		change_index: usize,
		note_index: usize,
	) -> anyhow::Result<(), Error> {
		let stored_amount = query!(
			r#"
				UPDATE chain_transfers SET included_in_notebook_number = $1
				WHERE account_id = $2 AND account_nonce = $3
				AND included_in_notebook_number IS NULL
				RETURNING amount
				"#,
			notebook_number as i32,
			account_id.as_slice(),
			account_nonce as i32,
		)
		.fetch_one(db)
		.await
		.map_err(|_| Error::TransferToLocalchainNotFound { change_index, note_index })?;

		let amount = stored_amount.amount.parse::<u128>().map_err(|e| {
			Error::InternalError(format!("Failed to parse amount from mainchain {}", e.to_string()))
		})?;
		ensure!(
			proposed_amount == amount,
			Error::TransferToLocalchainInvalidAmount {
				change_index,
				note_index,
				provided: proposed_amount,
				amount
			}
		);

		Ok(())
	}
	pub async fn take_for_notebook<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<Vec<ChainTransfer>, Error> {
		let rows = sqlx::query_as!(
			ChainTransferRow,
			r#"
			DELETE FROM chain_transfers where included_in_notebook_number = $1 RETURNING *
			"#,
			notebook_number as i32,
		)
		.fetch_all(db)
		.await?;
		rows.into_iter().map(TryInto::try_into).collect()
	}

	/// Records observation of a transfer to a localchain that is in a finalized block.
	pub async fn record_transfer_to_local_from_block<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		finalized_block_number: u32,
		account_id: &AccountId,
		account_nonce: u32,
		milligons: u128,
	) -> anyhow::Result<()> {
		let res = query!(
			r#"
			INSERT INTO chain_transfers (to_localchain, amount, account_id, account_nonce, finalized_block_number) VALUES ($1, $2, $3, $4, $5)
			"#,
			true,
			milligons.to_string(),
			account_id.as_slice(),
			account_nonce as i32,
			finalized_block_number as i32,
		)
		.execute(db)
		.await?;
		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to record transfer".to_string())
		);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::ops::Add;

	use chrono::{Duration, Utc};
	use frame_support::assert_ok;
	use sp_keyring::Sr25519Keyring::Bob;
	use sqlx::PgPool;
	use tracing_subscriber::EnvFilter;

	use crate::stores::{chain_transfer::*, notebook_header::NotebookHeaderStore};

	fn logger() {
		let _ = tracing_subscriber::fmt()
			.compact()
			.with_env_filter(EnvFilter::from_default_env())
			.try_init();
	}

	#[sqlx::test]
	async fn test_transfer_to_localchain_flow(pool: PgPool) -> anyhow::Result<()> {
		let mut db = &mut pool.acquire().await?;
		NotebookHeaderStore::create(
			&mut db,
			1,
			1,
			1,
			Utc::now().add(Duration::try_minutes(1).unwrap()).timestamp_millis() as u64,
		)
		.await?;
		logger();
		let notebook_number = 1;
		let account_id = Bob.to_account_id();
		let account_nonce = 1;
		let milligons = 1000;
		let change_index = 0;
		let note_index = 0;
		{
			let mut tx = pool.begin().await?;
			assert_ok!(
				ChainTransferStore::record_transfer_to_local_from_block(
					&mut *tx,
					100,
					&account_id,
					account_nonce,
					milligons
				)
				.await
			);
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			assert_ok!(
				ChainTransferStore::take_and_record_transfer_local(
					&mut *tx,
					notebook_number,
					&account_id,
					account_nonce,
					milligons,
					change_index,
					note_index,
				)
				.await
			);
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			let result = ChainTransferStore::take_for_notebook(&mut *tx, notebook_number).await?;
			assert_eq!(result.len(), 1);
			if let Some(ChainTransfer::ToLocalchain {
				account_id: t_account_id,
				account_nonce: t_nonce,
			}) = result.get(0)
			{
				assert_eq!(*t_account_id, account_id);
				assert_eq!(*t_nonce, account_nonce);
			} else {
				panic!("Expected to find a to localchain transfer");
			}
			tx.commit().await?;
		}
		Ok(())
	}

	#[sqlx::test]
	async fn test_transfer_can_only_be_in_one_notebook(pool: PgPool) -> anyhow::Result<()> {
		logger();
		let mut db = &mut pool.acquire().await?;
		NotebookHeaderStore::create(
			&mut db,
			1,
			1,
			1,
			Utc::now().add(Duration::try_minutes(1).unwrap()).timestamp_millis() as u64,
		)
		.await?;
		let notebook_number = 1;
		let account_id = Bob.to_account_id();
		let account_nonce = 1;
		let milligons = 1000;
		let change_index = 0;
		let note_index = 0;

		let mut tx = pool.begin().await?;
		assert_ok!(
			ChainTransferStore::record_transfer_to_local_from_block(
				&mut *tx,
				100,
				&account_id,
				account_nonce,
				milligons
			)
			.await
		);
		assert_ok!(
			ChainTransferStore::take_and_record_transfer_local(
				&mut *tx,
				notebook_number,
				&account_id,
				account_nonce,
				milligons,
				change_index,
				note_index,
			)
			.await
		);

		assert!(ChainTransferStore::take_and_record_transfer_local(
			&mut *tx,
			notebook_number,
			&account_id,
			account_nonce,
			milligons,
			change_index,
			note_index,
		)
		.await
		.unwrap_err()
		.to_string()
		.contains("Transfer not found (or already applied)"));
		tx.commit().await?;

		Ok(())
	}
}
