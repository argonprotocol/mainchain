use serde_json::{from_value, json};
use sqlx::{query, query_scalar, types::Json, FromRow};

use ulx_notary_primitives::{ensure, BalanceChange, NotebookNumber};

use crate::error::Error;

pub struct BalanceChangeStore;
#[derive(FromRow)]
#[allow(dead_code)]
struct BalanceChangeRow {
	pub notebook_number: i32,
	/// Scale encoded set of BalanceChangesets submitted together
	pub changeset: Json<Vec<BalanceChange>>,
}

impl BalanceChangeStore {
	pub async fn append_notebook_changeset<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
		changeset: Vec<BalanceChange>,
	) -> anyhow::Result<(), Error> {
		let data = json!(changeset);
		let res = query!(
			r#"
			INSERT INTO balance_changes (notebook_number, changeset) VALUES ($1, $2)
		"#,
			notebook_number as i32,
			data
		)
		.execute(db)
		.await?;

		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert balance changes".to_string())
		);

		Ok(())
	}

	pub async fn get_for_notebook<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<Vec<Vec<BalanceChange>>, Error> {
		let rows = query_scalar!(
			r#"
			SELECT changeset FROM balance_changes WHERE notebook_number = $1
		"#,
			notebook_number as i32,
		)
		.fetch_all(db)
		.await?;

		let result: Result<Vec<Vec<BalanceChange>>, _> =
			rows.into_iter().map(from_value::<Vec<BalanceChange>>).collect();

		Ok(result?)
	}
}

#[cfg(test)]
mod tests {
	use sp_core::bounded_vec;
	use sp_keyring::Sr25519Keyring::Bob;
	use sqlx::PgPool;

	use ulx_notary_primitives::{BalanceChange, Chain::Argon, Note, NoteType};

	use crate::stores::balance_change::BalanceChangeStore;

	#[sqlx::test]
	async fn test_storage(pool: PgPool) -> anyhow::Result<()> {
		let notebook_number = 1;
		let changeset = vec![BalanceChange {
			account_id: Bob.to_account_id(),
			chain: Argon,
			nonce: 0,
			balance: 1000,
			previous_balance: 0,
			previous_balance_proof: None,
			notes: bounded_vec![Note::create_unsigned(
				&Bob.to_account_id(),
				&Argon,
				1,
				1000,
				NoteType::ClaimFromMainchain { nonce: 1 }
			)],
		}];

		{
			let mut tx = pool.begin().await?;
			BalanceChangeStore::append_notebook_changeset(
				&mut *tx,
				notebook_number,
				changeset.clone(),
			)
			.await
			.unwrap();
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			let result =
				BalanceChangeStore::get_for_notebook(&mut *tx, notebook_number).await.unwrap();
			assert_eq!(result, vec![changeset]);
			tx.commit().await?;
		}

		Ok(())
	}
}
