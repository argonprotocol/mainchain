use crate::ensure;
use argon_notary_apis::error::Error;
use argon_primitives::NotebookNumber;
use chrono::{DateTime, Utc};
use polkadot_sdk::*;
use sc_utils::notification::{NotificationSender, NotificationStream, TracingKeyStr};
use sp_core::H256;
use sqlx::{FromRow, PgConnection, PgPool, postgres::PgListener};
pub type AuditFailureStream = NotificationStream<NotebookNumber, AuditFailureTracingKey>;

#[derive(Clone)]
pub struct AuditFailureTracingKey;
impl TracingKeyStr for AuditFailureTracingKey {
	const TRACING_KEY: &'static str = "mpsc_audit_failure_notification_stream";
}

pub struct AuditFailureListener {
	failed_audits_notification: NotificationSender<NotebookNumber>,
	listener: PgListener,
}
impl AuditFailureListener {
	pub async fn connect(
		pool: PgPool,
		failed_audits_notification: NotificationSender<NotebookNumber>,
	) -> anyhow::Result<Self> {
		let mut listener = PgListener::connect_with(&pool).await?;
		listener.listen("audit_failure").await?;
		Ok(Self { failed_audits_notification, listener })
	}

	pub async fn next(&mut self) -> anyhow::Result<NotebookNumber> {
		let notification = &self.listener.recv().await?;
		let notebook_number = match notification.payload().parse() {
			Ok(notebook_number) => notebook_number,
			Err(e) => {
				return Err(anyhow::anyhow!("Error parsing notified notebook number {e:?}"));
			},
		};

		self.failed_audits_notification.notify(|| Ok(notebook_number)).map_err(
			|e: anyhow::Error| anyhow::anyhow!("Error sending failed audits notification {e:?}"),
		)?;
		Ok(notebook_number)
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
