use sqlx::PgConnection;

use argon_primitives::{
	NotebookNumber, MAX_BALANCE_CHANGES_PER_NOTARIZATION, MAX_BLOCK_VOTES_PER_NOTEBOOK,
	MAX_DOMAINS_PER_NOTEBOOK, MAX_NOTARIZATIONS_PER_NOTEBOOK, MAX_NOTEBOOK_TRANSFERS,
};

use crate::{ensure, error::Error};

#[derive(Default, Clone, Debug)]
pub struct NotarizationCounts {
	pub block_votes: u32,
	pub balance_changes: u32,
	pub data_domains: u32,
	pub chain_transfers: u32,
}

#[derive(Debug, Clone)]
pub struct MaxNotebookCounts {
	pub max_block_votes: u32,
	pub max_balance_changes: u32,
	pub max_data_domains: u32,
	pub max_notarizations: u32,
	pub max_chain_transfers: u32,
}

impl MaxNotebookCounts {
	pub fn new(
		max_block_votes: u32,
		max_balance_changes: u32,
		max_data_domains: u32,
		max_chain_transfers: u32,
		max_notarizations: u32,
	) -> Self {
		Self {
			max_notarizations,
			max_block_votes,
			max_balance_changes,
			max_data_domains,
			max_chain_transfers,
		}
	}
}

impl Default for MaxNotebookCounts {
	fn default() -> Self {
		Self {
			max_notarizations: MAX_NOTARIZATIONS_PER_NOTEBOOK,
			max_block_votes: MAX_BLOCK_VOTES_PER_NOTEBOOK,
			max_balance_changes: MAX_BALANCE_CHANGES_PER_NOTARIZATION *
				MAX_NOTARIZATIONS_PER_NOTEBOOK,
			max_data_domains: MAX_DOMAINS_PER_NOTEBOOK,
			max_chain_transfers: MAX_NOTEBOOK_TRANSFERS,
		}
	}
}

pub struct NotebookConstraintsStore;
impl NotebookConstraintsStore {
	pub async fn create(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<(), Error> {
		let res = sqlx::query!(
			r#"
				INSERT INTO notebook_constraints (notebook_number) VALUES ($1)
			"#,
			notebook_number as i32
		)
		.execute(db)
		.await?;
		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert notebook constraints".to_string())
		);

		Ok(())
	}
	pub async fn try_increment<'a>(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
		counts: NotarizationCounts,
		max_notarization_counts: MaxNotebookCounts,
	) -> anyhow::Result<(), Error> {
		// get an advisory lock for the notebook
		sqlx::query!("SELECT pg_advisory_xact_lock($1)", notebook_number as i32)
			.execute(&mut *db)
			.await?;

		let result = sqlx::query!(
			r#"
				UPDATE notebook_constraints SET
					block_votes = block_votes + $2,
					balance_changes = balance_changes + $3,
					data_domains = data_domains + $4,
					chain_transfers = chain_transfers + $5,
					notarizations = notarizations + 1
				WHERE notebook_number = $1
					AND block_votes + $2 <= $6
					AND balance_changes + $3 <= $7
					AND data_domains + $4 <= $8
					AND chain_transfers + $5 <= $9
					AND notarizations < $10
			"#,
			notebook_number as i32,
			counts.block_votes as i32,
			counts.balance_changes as i32,
			counts.data_domains as i32,
			counts.chain_transfers as i32,
			max_notarization_counts.max_block_votes as i32,
			max_notarization_counts.max_balance_changes as i32,
			max_notarization_counts.max_data_domains as i32,
			max_notarization_counts.max_chain_transfers as i32,
			max_notarization_counts.max_notarizations as i32,
		)
		.execute(db)
		.await?;

		ensure!(result.rows_affected() == 1, Error::MaxNotebookChainTransfersReached);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;
	use sqlx::PgPool;

	use crate::{error::Error, stores::notebook_header::NotebookHeaderStore};

	use super::*;

	#[sqlx::test]
	async fn test_max_chain_transfers(pool: PgPool) -> anyhow::Result<()> {
		let mut tx = pool.begin().await?;

		NotebookHeaderStore::create(&mut tx, 1, 1, 1, 1).await?;
		let constraints = MaxNotebookCounts::new(1, 1, 1, 3, 3);
		let counts = NotarizationCounts {
			block_votes: 0,
			balance_changes: 0,
			data_domains: 0,
			chain_transfers: 1,
		};
		assert_ok!(
			NotebookConstraintsStore::try_increment(
				&mut tx,
				1,
				counts.clone(),
				constraints.clone()
			)
			.await
		);
		tx.commit().await?;
		let mut tx = pool.begin().await?;
		assert_ok!(
			NotebookConstraintsStore::try_increment(
				&mut tx,
				1,
				counts.clone(),
				constraints.clone()
			)
			.await
		);
		tx.commit().await?;
		let mut tx = pool.begin().await?;
		assert_ok!(
			NotebookConstraintsStore::try_increment(
				&mut tx,
				1,
				counts.clone(),
				constraints.clone()
			)
			.await
		);
		tx.commit().await?;
		let mut tx = pool.begin().await?;

		assert!(matches!(
			NotebookConstraintsStore::try_increment(
				&mut tx,
				1,
				counts.clone(),
				constraints.clone()
			)
			.await,
			Err(Error::MaxNotebookChainTransfersReached)
		));

		tx.commit().await?;
		Ok(())
	}
}
