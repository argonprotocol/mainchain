#![allow(clippy::too_many_arguments)]
use std::default::Default;

use sp_core::H256;
use sqlx::PgConnection;

use argon_notary_apis::localchain::BalanceTipResult;
use argon_primitives::{
	AccountOrigin, AccountType, BalanceTip, Note, NotebookNumber, ensure, prelude::*,
};

use crate::{Error, stores::BoxFutureResult};

/// This table is used as a quick verification of the last balance change. It is also the last valid
/// entry in a notebook. Without this table, you must obtain proof that a balance has not changed
/// since the merkle-proven change.
#[derive(Debug, Default, sqlx::FromRow)]
#[allow(dead_code)]
struct BalanceTipRow {
	pub key: Vec<u8>,
	pub value: Vec<u8>,
	pub last_changed_notebook: i32,
	pub last_changed_tick: i32,
}
pub struct BalanceTipStore;

impl BalanceTipStore {
	pub fn lock<'a>(
		db: &'a mut PgConnection,
		account_id: &AccountId,
		account_type: AccountType,
		proposed_change_number: u32,
		previous_balance: u128,
		account_origin: &AccountOrigin,
		change_index: u32,
		channel_hold_note: Option<Note>,
		timeout_millis: u32,
	) -> BoxFutureResult<'a, Option<H256>> {
		let key = BalanceTip::create_key(account_id, &account_type);

		let mut provided_tip: Option<H256> = None;
		if proposed_change_number > 1u32 {
			provided_tip = Some(
				BalanceTip::compute_tip(
					proposed_change_number - 1u32,
					previous_balance,
					account_origin.clone(),
					channel_hold_note,
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
				let tip = H256::from_slice(vec.as_slice());
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
	pub async fn get_tip(
		db: &mut PgConnection,
		account_id: &AccountId,
		account_type: AccountType,
	) -> anyhow::Result<BalanceTipResult, Error> {
		let key = BalanceTip::create_key(account_id, &account_type);
		let row = sqlx::query_as!(
			BalanceTipRow,
			r#"
			SELECT * FROM balance_tips WHERE key = $1 LIMIT 1
			"#,
			key.as_slice()
		)
		.fetch_one(db)
		.await?;

		Ok(BalanceTipResult {
			balance_tip: H256::from_slice(row.value.as_slice()),
			notebook_number: row.last_changed_notebook as NotebookNumber,
			tick: Tick::from(row.last_changed_tick as Tick),
		})
	}

	pub fn update<'a>(
		db: &'a mut PgConnection,
		account_id: &AccountId,
		account_type: AccountType,
		change_number: u32,
		balance: u128,
		notebook_number: NotebookNumber,
		tick: Tick,
		account_origin: AccountOrigin,
		channel_hold_note: Option<Note>,
		prev_balance: u128,
		prev_channel_hold_note: Option<Note>,
	) -> BoxFutureResult<'a, ()> {
		let key = BalanceTip::create_key(account_id, &account_type);
		let tip = BalanceTip::compute_tip(
			change_number,
			balance,
			account_origin.clone(),
			channel_hold_note.clone(),
		);
		let prev = BalanceTip::compute_tip(
			change_number - 1,
			prev_balance,
			account_origin,
			prev_channel_hold_note,
		);
		Box::pin(async move {
			let res = sqlx::query!(
				r#"
			INSERT INTO balance_tips (key, value, last_changed_notebook, last_changed_tick) VALUES ($1, $2, $3, $4)
			ON CONFLICT (key)
			DO UPDATE SET value = $2, last_changed_notebook = $3, last_changed_tick = $4
				WHERE balance_tips.value = $5;
			"#,
				key.as_slice(),
				tip.as_slice(),
				notebook_number as i32,
				tick as i64,
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
	use polkadot_sdk::*;
	use sp_keyring::Sr25519Keyring::Bob;
	use sqlx::PgPool;

	use argon_primitives::{AccountOrigin, AccountType::Deposit, BalanceTip};

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
					Deposit,
					1,
					0,
					&AccountOrigin { notebook_number: 1, account_uid: 1 },
					0,
					None,
					10
				)
				.await?,
				None
			);
			BalanceTipStore::update(
				&mut tx1,
				&Bob.to_account_id(),
				Deposit,
				1,
				1000,
				1,
				1,
				AccountOrigin { notebook_number: 1, account_uid: 1 },
				None,
				0,
				None,
			)
			.await?;
			tx1.commit().await?;
		}

		let mut tx2 = pool.begin().await?;
		assert_eq!(
			BalanceTipStore::lock(
				&mut tx2,
				&Bob.to_account_id(),
				Deposit,
				2,
				1000,
				&AccountOrigin { notebook_number: 1, account_uid: 1 },
				0,
				None,
				10
			)
			.await?,
			Some(
				BalanceTip::compute_tip(
					1,
					1000,
					AccountOrigin { notebook_number: 1, account_uid: 1 },
					None
				)
				.into()
			)
		);

		let mut tx3 = pool.begin().await?;
		assert!(
			BalanceTipStore::lock(
				&mut tx3,
				&Bob.to_account_id(),
				Deposit,
				2,
				1000,
				&AccountOrigin { notebook_number: 1, account_uid: 1 },
				0,
				None,
				10
			)
			.await
			.unwrap_err()
			.to_string()
			.contains("lock timeout"),
		);

		assert_ok!(
			BalanceTipStore::update(
				&mut tx2,
				&Bob.to_account_id(),
				Deposit,
				2,
				1001,
				1,
				1,
				AccountOrigin { notebook_number: 1, account_uid: 1 },
				None,
				1000,
				None
			)
			.await
		);
		tx2.commit().await?;

		Ok(())
	}
}
