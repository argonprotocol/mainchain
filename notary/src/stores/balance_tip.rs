use std::default::Default;

use sp_core::H256;
use sqlx::PgConnection;

use ulx_notary_primitives::{ensure, AccountId, AccountOrigin, BalanceTip, Chain, NotebookNumber};

use crate::{stores::BoxFutureResult, Error};

/// This table is used as a quick verification of the last balance change. It is also the last valid
/// entry in a notebook. Without this table, you must obtain proof that a balance has not changed
/// since the merkle-proven change.
#[derive(Debug, Default, sqlx::FromRow)]
#[allow(dead_code)]
struct BalanceTipRow {
	pub key: Vec<u8>,
	pub value: Vec<u8>,
	pub last_changed_notebook: i32,
}
pub struct BalanceTipStore;

impl BalanceTipStore {
	pub fn lock<'a>(
		db: &'a mut PgConnection,
		account_id: &AccountId,
		chain: Chain,
		proposed_nonce: u32,
		previous_balance: u128,
		account_origin: &AccountOrigin,
		change_index: usize,
		timeout_millis: u32,
	) -> BoxFutureResult<'a, Option<H256>> {
		let key = BalanceTip::create_key(account_id, &chain);

		let mut provided_tip: Option<H256> = None;
		if proposed_nonce > 1u32 {
			provided_tip = Some(
				BalanceTip::compute_tip(
					proposed_nonce - 1u32,
					previous_balance,
					account_origin.clone(),
				)
				.into(),
			);
		}

		Box::pin(async move {
			sqlx::query(format!("SET lock_timeout TO '{timeout_millis}ms'").as_str())
				.execute(&mut *db)
				.await?;
			let value = sqlx::query!(
				r#"SELECT value FROM balance_tips WHERE key = $1 FOR UPDATE LIMIT 1"#,
				key.as_slice()
			)
			.fetch_optional(db)
			.await?;

			let tip = if let Some(value) = value {
				let vec = value.value;
				let tip = H256::from_slice(&vec.as_slice());
				Some(tip)
			} else {
				None
			};

			ensure!(
				tip == provided_tip,
				Error::BalanceTipMismatch { change_index, stored_tip: tip, provided_tip }
			);

			Ok(tip)
		})
	}

	pub fn update<'a>(
		db: &'a mut PgConnection,
		account_id: &AccountId,
		chain: Chain,
		nonce: u32,
		balance: u128,
		notebook_number: NotebookNumber,
		account_origin: AccountOrigin,
		prev_balance: u128,
	) -> BoxFutureResult<'a, ()> {
		let key = BalanceTip::create_key(account_id, &chain);
		let tip = BalanceTip::compute_tip(nonce, balance, account_origin.clone());
		let prev = BalanceTip::compute_tip(nonce - 1, prev_balance, account_origin);
		Box::pin(async move {
			let res = sqlx::query!(
				r#"
			INSERT INTO balance_tips (key, value, last_changed_notebook) VALUES ($1, $2, $3) 
			ON CONFLICT (key) 
			DO UPDATE SET value = $2, last_changed_notebook = $3 
				WHERE balance_tips.value = $4;
			"#,
				key.as_slice(),
				tip.as_slice(),
				notebook_number as i32,
				prev.as_slice()
			)
			.execute(db)
			.await?;
			ensure!(
				res.rows_affected() == 1,
				Error::InternalError("Unable to upsert this balance".to_string())
			);

			Ok(())
		})
	}
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;
	use sp_keyring::Sr25519Keyring::Bob;
	use sqlx::PgPool;

	use ulx_notary_primitives::{note::Chain::Argon, AccountOrigin, BalanceTip};

	use crate::stores::balance_tip::BalanceTipStore;

	#[sqlx::test]
	async fn test_only_one_tx_can_update(pool: PgPool) -> anyhow::Result<()> {
		// Initialize the logger
		let _ = tracing_subscriber::fmt::try_init();

		{
			let mut tx1 = pool.begin().await?;
			assert_eq!(
				BalanceTipStore::lock(
					&mut tx1,
					&Bob.to_account_id(),
					Argon,
					1,
					0,
					&AccountOrigin { notebook_number: 1, account_uid: 1 },
					0,
					10
				)
				.await?,
				None
			);
			BalanceTipStore::update(
				&mut *tx1,
				&Bob.to_account_id(),
				Argon,
				1,
				1000,
				1,
				AccountOrigin { notebook_number: 1, account_uid: 1 },
				0,
			)
			.await?;
			tx1.commit().await?;
		}

		let mut tx2 = pool.begin().await?;
		assert_eq!(
			BalanceTipStore::lock(
				&mut tx2,
				&Bob.to_account_id(),
				Argon,
				2,
				1000,
				&AccountOrigin { notebook_number: 1, account_uid: 1 },
				0,
				10
			)
			.await?,
			Some(
				BalanceTip::compute_tip(
					1,
					1000,
					AccountOrigin { notebook_number: 1, account_uid: 1 },
				)
				.into()
			)
		);

		let mut tx3 = pool.begin().await?;
		assert!(BalanceTipStore::lock(
			&mut *tx3,
			&Bob.to_account_id(),
			Argon,
			2,
			1000,
			&AccountOrigin { notebook_number: 1, account_uid: 1 },
			0,
			10
		)
		.await
		.unwrap_err()
		.to_string()
		.contains("lock timeout"),);

		assert_ok!(
			BalanceTipStore::update(
				&mut *tx2,
				&Bob.to_account_id(),
				Argon,
				2,
				1001,
				1,
				AccountOrigin { notebook_number: 1, account_uid: 1 },
				1000
			)
			.await
		);
		tx2.commit().await?;

		Ok(())
	}
}
