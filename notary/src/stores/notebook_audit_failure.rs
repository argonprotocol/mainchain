use crate::ensure;
use argon_notary_apis::error::Error;
use argon_primitives::NotebookNumber;
use chrono::{DateTime, Utc};
use polkadot_sdk::*;
use sc_utils::notification::{NotificationSender, NotificationStream, TracingKeyStr};
use sp_core::H256;
use sqlx::{postgres::PgListener, Error as SqlxError, FromRow, PgConnection, PgPool};
use std::time::Duration;
pub type AuditFailureStream = NotificationStream<NotebookNumber, AuditFailureTracingKey>;

#[derive(Clone)]
pub struct AuditFailureTracingKey;
impl TracingKeyStr for AuditFailureTracingKey {
	const TRACING_KEY: &'static str = "mpsc_audit_failure_notification_stream";
}

pub struct AuditFailureListener {
	pool: PgPool,
	failed_audits_notification: NotificationSender<NotebookNumber>,
	listener: PgListener,
}
impl AuditFailureListener {
	pub async fn connect(
		pool: PgPool,
		failed_audits_notification: NotificationSender<NotebookNumber>,
	) -> anyhow::Result<Self> {
		let listener = Self::connect_listener(&pool).await?;
		Ok(Self { pool, failed_audits_notification, listener })
	}

	async fn connect_listener(pool: &PgPool) -> anyhow::Result<PgListener> {
		let mut listener = PgListener::connect_with(pool).await?;
		listener.listen("audit_failure").await?;
		Ok(listener)
	}

	pub async fn next(&mut self) -> anyhow::Result<NotebookNumber> {
		loop {
			let notification = self.listener.recv().await?;
			let notebook_number = match notification.payload().parse() {
				Ok(notebook_number) => notebook_number,
				Err(e) => {
					tracing::error!(
						"Ignoring malformed audit failure notification payload {:?}: {:?}",
						notification.payload(),
						e
					);
					continue;
				},
			};

			self.notify_failed_audit(notebook_number)?;
			return Ok(notebook_number);
		}
	}

	pub(crate) async fn reconnect(
		&mut self,
		delay: Duration,
	) -> anyhow::Result<Option<NotebookNumber>> {
		loop {
			match self.try_reconnect().await {
				Ok(notebook_number) => return Ok(notebook_number),
				Err(e) => {
					tracing::error!("Error reconnecting audit failure listener {:?}", e);
					if is_closed_pool_error(&e) {
						return Err(e);
					}

					tokio::time::sleep(delay).await;
				},
			}
		}
	}

	async fn try_reconnect(&mut self) -> anyhow::Result<Option<NotebookNumber>> {
		self.listener = Self::connect_listener(&self.pool).await?;

		if let Some(failure) =
			NotebookAuditFailureStore::has_unresolved_audit_failure(&self.pool).await?
		{
			let notebook_number = failure.notebook_number as NotebookNumber;
			self.notify_failed_audit(notebook_number)?;
			return Ok(Some(notebook_number));
		}

		Ok(None)
	}

	fn notify_failed_audit(&self, notebook_number: NotebookNumber) -> anyhow::Result<()> {
		self.failed_audits_notification.notify(|| Ok(notebook_number)).map_err(
			|e: anyhow::Error| anyhow::anyhow!("Error sending failed audits notification {e:?}"),
		)?;
		Ok(())
	}
}

#[derive(FromRow)]
pub struct NotebookAuditFailureRow {
	pub notebook_number: i32,
	pub hash: Vec<u8>,
	pub failure_reason: String,
	pub failure_block_number: i32,
	pub is_resolved: bool,
	pub last_updated: DateTime<Utc>,
}

pub struct NotebookAuditFailureStore {}

impl NotebookAuditFailureStore {
	pub async fn record(
		db: &mut PgConnection,
		notebook_number: u32,
		hash: H256,
		failure_reason: String,
		failure_block_number: u32,
	) -> anyhow::Result<(), Error> {
		let notebook_number = notebook_number as i32;
		let hash = hash.as_bytes().to_vec();
		let failure_block_number = failure_block_number as i32;
		let res = sqlx::query!(
			r#"
            INSERT INTO notebook_audit_failures (notebook_number, hash, failure_reason, failure_block_number, is_resolved)
            VALUES ($1, $2, $3, $4, false)
            "#,
			notebook_number,
			hash,
			failure_reason,
			failure_block_number
		)
		.execute(db)
		.await?;
		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert failed audit".to_string())
		);
		Ok(())
	}

	pub async fn has_unresolved_audit_failure(
		db: &PgPool,
	) -> anyhow::Result<Option<NotebookAuditFailureRow>, Error> {
		let result = sqlx::query_as!(
			NotebookAuditFailureRow,
			r#"
				SELECT * from notebook_audit_failures
				WHERE is_resolved = false
				LIMIT 1
            "#,
		)
		.fetch_optional(db)
		.await?;
		Ok(result)
	}
}

fn is_closed_pool_error(error: &anyhow::Error) -> bool {
	error
		.chain()
		.filter_map(|source| source.downcast_ref::<SqlxError>())
		.any(|source| matches!(source, SqlxError::PoolClosed))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::stores::notebook_header::NotebookHeaderStore;
	use futures::StreamExt;

	#[sqlx::test]
	async fn test_reconnect_publishes_existing_audit_failure(pool: PgPool) -> anyhow::Result<()> {
		let (sender, stream) = AuditFailureStream::channel();
		let mut listener = AuditFailureListener::connect(pool.clone(), sender).await?;
		let mut subscription = stream.subscribe(10);

		let mut tx = pool.begin().await?;
		NotebookHeaderStore::create(&mut tx, 1, 1, 1, 1).await?;
		tx.commit().await?;

		let mut db = pool.acquire().await?;
		NotebookAuditFailureStore::record(
			&mut db,
			1,
			H256::from([1u8; 32]),
			"failure".to_string(),
			1,
		)
		.await?;

		let notebook_number = listener.reconnect(Duration::ZERO).await?;
		assert_eq!(notebook_number, Some(1));

		let published_notebook = tokio::time::timeout(Duration::from_secs(1), subscription.next())
			.await?
			.expect("should publish the unresolved audit failure");
		assert_eq!(published_notebook, 1);
		assert!(tokio::time::timeout(Duration::from_millis(100), subscription.next())
			.await
			.is_err());

		Ok(())
	}
}
