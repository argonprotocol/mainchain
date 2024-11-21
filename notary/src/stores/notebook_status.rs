use argon_notary_apis::error::Error;
use argon_primitives::{tick::Tick, NotebookNumber};
use chrono::{DateTime, Utc};
use sqlx::{query, Executor, FromRow, PgConnection};

use crate::ensure;

#[cfg(not(test))]
pub const NOTEBOOK_DURATION_MS: i64 = 60_000;

#[cfg(test)]
pub const NOTEBOOK_DURATION_MS: i64 = 10_000;

pub struct NotebookStatusStore;

#[derive(Copy, Clone)]
pub enum NotebookFinalizationStep {
	Open = 1,
	ReadyForClose = 2,
	Closed = 3,
	Finalized = 4,
}

impl From<i32> for NotebookFinalizationStep {
	fn from(i: i32) -> Self {
		match i {
			1 => NotebookFinalizationStep::Open,
			2 => NotebookFinalizationStep::ReadyForClose,
			3 => NotebookFinalizationStep::Closed,
			4 => NotebookFinalizationStep::Finalized,
			_ => panic!("Invalid notebook finalization step"),
		}
	}
}

#[derive(FromRow)]
pub struct NotebookStatusRow {
	pub notebook_number: i32,
	pub tick: i32,
	pub step: NotebookFinalizationStep,
	pub open_time: DateTime<Utc>,
	pub end_time: DateTime<Utc>,
	pub ready_for_close_time: Option<DateTime<Utc>>,
	pub closed_time: Option<DateTime<Utc>>,
	pub finalized_time: Option<DateTime<Utc>>,
}

impl NotebookStatusStore {
	pub async fn get<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<NotebookStatusRow, Error> {
		let row = sqlx::query_as!(
			NotebookStatusRow,
			r#"
			SELECT * FROM notebook_status WHERE notebook_number = $1 LIMIT 1
			"#,
			notebook_number as i32,
		)
		.fetch_one(db)
		.await?;
		Ok(row)
	}

	pub async fn lock_open_for_appending<'a>(
		db: &mut PgConnection,
	) -> anyhow::Result<(NotebookNumber, Tick), Error> {
		for _ in 0..3 {
			let row = query!(
				r#"SELECT notebook_number, tick FROM notebook_status WHERE step=$1 FOR SHARE LIMIT 1"#,
				NotebookFinalizationStep::Open as i32
			)
			.fetch_optional(&mut *db)
			.await?;
			if let Some(row) = row {
				return Ok((row.notebook_number as NotebookNumber, row.tick as Tick));
			}
			tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
		}
		Err(Error::InternalError("Unable to lock notebook for write".to_string()))
	}

	async fn lock_to_stop_appends(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<(), Error> {
		db.execute("SET statement_timeout = 5000").await?;
		sqlx::query!(
			r#"
			SELECT 1 as exists FROM notebook_status WHERE notebook_number = $1 FOR UPDATE NOWAIT LIMIT 1
			"#,
			notebook_number as i32,
		)
		.fetch_one(&mut *db)
		.await?;
		Ok(())
	}

	pub async fn find_and_lock_ready_for_close(
		db: &mut PgConnection,
	) -> anyhow::Result<Option<(NotebookNumber, Tick)>, Error> {
		let result = sqlx::query!(
			r#"
				SELECT notebook_number, tick FROM notebook_status
				WHERE step=$1
				ORDER BY notebook_number ASC
				LIMIT 1
			"#,
			NotebookFinalizationStep::ReadyForClose as i32,
		)
		.fetch_optional(&mut *db)
		.await?;

		if let Some(row) = result {
			let notebook_number = row.notebook_number as u32;
			Self::lock_to_stop_appends(&mut *db, notebook_number).await?;
			return Ok(Some((notebook_number, row.tick as Tick)));
		}
		Ok(None)
	}

	pub async fn create<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
		tick: Tick,
		next_tick_end: DateTime<Utc>,
	) -> anyhow::Result<(), Error> {
		let res = sqlx::query!(
			r#"
				INSERT INTO notebook_status (notebook_number, open_time, end_time, tick, step) VALUES ($1, now(), $2, $3, $4)
			"#,
			notebook_number as i32,
			next_tick_end,
			tick as i64,
			NotebookFinalizationStep::Open as i32,
		)
		.execute(db)
		.await?;
		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert notebook status".to_string())
		);
		Ok(())
	}

	pub async fn step_up_expired_open<'a>(
		db: &mut PgConnection,
	) -> anyhow::Result<Option<u32>, Error> {
		let result = sqlx::query!(
			r#"
				SELECT * FROM notebook_status
				WHERE step = $1 AND end_time <= $2
				ORDER BY open_time ASC
				LIMIT 1
			"#,
			NotebookFinalizationStep::Open as i32,
			Utc::now()
		)
		.fetch_optional(&mut *db)
		.await?;

		if let Some(result) = result {
			let notebook_number = result.notebook_number as u32;
			Self::next_step(&mut *db, notebook_number, NotebookFinalizationStep::Open).await?;
			return Ok(Some(notebook_number));
		}
		Ok(None)
	}

	pub async fn next_step<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
		current_step: NotebookFinalizationStep,
	) -> anyhow::Result<(), Error> {
		let (next_step, time_field) = match current_step {
			NotebookFinalizationStep::Open =>
				(NotebookFinalizationStep::ReadyForClose, "ready_for_close_time"),
			NotebookFinalizationStep::ReadyForClose =>
				(NotebookFinalizationStep::Closed, "closed_time"),
			NotebookFinalizationStep::Closed =>
				(NotebookFinalizationStep::Finalized, "finalized_time"),
			NotebookFinalizationStep::Finalized => return Ok(()),
		};

		let res = sqlx::query(&format!(
			r#"
				UPDATE notebook_status SET step=$1, {time_field}=$2 WHERE notebook_number=$3 AND step=$4
			"#,
		))
		.bind(next_step as i32)
		.bind(Utc::now())
		.bind(notebook_number as i32)
		.bind(current_step as i32)
		.execute(db)
		.await?;
		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to update notebook step".to_string())
		);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::ops::Add;

	use crate::stores::notebook_status::{NotebookFinalizationStep, NotebookStatusStore};
	use argon_notary_apis::error::Error;
	use chrono::{Duration, Utc};
	use frame_support::assert_ok;
	use futures::future::try_join;
	use sqlx::PgPool;

	#[sqlx::test]
	async fn test_locks(pool: PgPool) -> anyhow::Result<()> {
		sqlx::query!("ALTER TABLE notebook_status DROP CONSTRAINT IF EXISTS notebook_status_notebook_number_fkey")
			.execute(&pool)
			.await?;
		let _ = tracing_subscriber::fmt::try_init();
		let notebook_number = 1;
		{
			let mut tx = pool.begin().await?;

			NotebookStatusStore::create(
				&mut *tx,
				1,
				1,
				Utc::now().add(Duration::try_minutes(1).unwrap()),
			)
			.await?;

			tx.commit().await?;
		}
		{
			let mut tx1 = pool.begin().await?;
			let mut tx2 = pool.begin().await?;
			assert_eq!(
				NotebookStatusStore::lock_open_for_appending(&mut tx1).await?.0,
				notebook_number
			);
			assert_eq!(
				NotebookStatusStore::lock_open_for_appending(&mut tx2).await?.0,
				notebook_number
			);

			let mut fail_tx = pool.begin().await?;
			assert!(NotebookStatusStore::lock_to_stop_appends(&mut fail_tx, notebook_number)
				.await
				.is_err());
			fail_tx.commit().await?;

			tx1.commit().await?;
			tx2.commit().await?;

			let (rx, txer) = tokio::sync::oneshot::channel();

			let cloned = pool.clone();
			let task1 = tokio::spawn(async move {
				let mut tx = cloned.begin().await?;
				assert_ok!(
					NotebookStatusStore::lock_to_stop_appends(&mut tx, notebook_number).await
				);
				let _ = rx.send(0);
				// wait for 500 ms
				tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
				NotebookStatusStore::next_step(
					&mut *tx,
					notebook_number,
					NotebookFinalizationStep::Open,
				)
				.await?;
				NotebookStatusStore::create(
					&mut *tx,
					2,
					2,
					Utc::now().add(Duration::try_minutes(1).unwrap()),
				)
				.await?;
				tx.commit().await?;
				Result::<(), Error>::Ok(())
			});

			let cloned2 = pool.clone();
			let task2 = tokio::spawn(async move {
				let mut tx = cloned2.begin().await?;
				let _ = txer.await;
				let next_notebook = NotebookStatusStore::lock_open_for_appending(&mut tx).await?;
				tx.commit().await?;
				Result::<u32, Error>::Ok(next_notebook.0)
			});

			let results = try_join(task1, task2).await?;
			assert_eq!(results.1?, 2);
		}
		Ok(())
	}

	#[sqlx::test]
	async fn test_locks_step(pool: PgPool) -> anyhow::Result<()> {
		sqlx::query!("ALTER TABLE notebook_status DROP CONSTRAINT IF EXISTS notebook_status_notebook_number_fkey")
			.execute(&pool)
			.await?;
		let _ = tracing_subscriber::fmt::try_init();
		{
			let mut tx = pool.begin().await?;

			NotebookStatusStore::create(&mut *tx, 1, 1, Utc::now()).await?;
			let _ = NotebookStatusStore::step_up_expired_open(&mut tx).await?;
			assert_eq!(
				NotebookStatusStore::find_and_lock_ready_for_close(&mut tx,).await?,
				Some((1, 1))
			);

			tx.commit().await?;
		}

		let mut tx = pool.begin().await?;
		assert_eq!(
			NotebookStatusStore::find_and_lock_ready_for_close(&mut tx).await?,
			Some((1, 1))
		);
		{
			let mut tx2 = pool.begin().await?;
			assert!(NotebookStatusStore::find_and_lock_ready_for_close(&mut tx2).await.is_err());
			tx2.rollback().await?;
		}

		assert_ok!(
			NotebookStatusStore::next_step(&mut *tx, 1, NotebookFinalizationStep::ReadyForClose)
				.await
		);
		tx.commit().await?;
		Ok(())
	}

	#[sqlx::test]
	async fn test_expire_open(pool: PgPool) -> anyhow::Result<()> {
		sqlx::query!("ALTER TABLE notebook_status DROP CONSTRAINT IF EXISTS notebook_status_notebook_number_fkey")
			.execute(&pool)
			.await?;
		let mut tx = pool.begin().await?;

		NotebookStatusStore::create(
			&mut *tx,
			1,
			1,
			Utc::now().add(Duration::try_minutes(1).unwrap()),
		)
		.await?;
		assert_eq!(NotebookStatusStore::step_up_expired_open(&mut tx).await?, None);
		tx.commit().await?;

		sqlx::query("update notebook_status set end_time = now() where notebook_number = 1")
			.execute(&pool)
			.await?;

		let mut tx = pool.begin().await?;
		assert_eq!(NotebookStatusStore::step_up_expired_open(&mut tx).await?, Some(1));

		Ok(())
	}
}
