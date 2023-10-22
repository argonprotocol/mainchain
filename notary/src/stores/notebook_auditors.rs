use std::net::Ipv4Addr;

use chrono::Utc;
use sqlx::{query, FromRow, PgConnection};

use ulixee_client::api::runtime_types::ulx_primitives::block_seal::Host;

use crate::{ensure, Error};

#[derive(FromRow)]
#[allow(dead_code)]
struct NotebookAuditorRow {
	pub notebook_number: i32,
	pub public: Vec<u8>,
	pub rpc_urls: Vec<String>,
	pub signature: Option<Vec<u8>>,
	pub auditor_order: i32,
	pub attempts: i32,
	pub last_attempt: Option<chrono::DateTime<Utc>>,
}

pub struct NotebookAuditorStore;
pub struct NotebookAuditor {
	pub notebook_number: i32,
	pub public: [u8; 32],
	pub signature: Option<[u8; 64]>,
	pub rpc_urls: Vec<String>,
	pub auditor_order: i32,
	pub attempts: i32,
	pub last_attempt: Option<chrono::DateTime<Utc>>,
}

impl TryInto<NotebookAuditor> for NotebookAuditorRow {
	type Error = Error;
	fn try_into(self) -> Result<NotebookAuditor, Self::Error> {
		Ok(NotebookAuditor {
			notebook_number: self.notebook_number,
			public: self.public.try_into().map_err(|x| {
				Error::InternalError(format!("Unable to read public key from db {:?}", x))
			})?,
			signature: match self.signature {
				Some(s) => Some(s.try_into().map_err(|x| {
					Error::InternalError(format!("Unable to read signature from db {:?}", x))
				})?),
				None => None,
			},
			rpc_urls: self.rpc_urls,
			auditor_order: self.auditor_order,
			attempts: self.attempts,
			last_attempt: self.last_attempt,
		})
	}
}
impl NotebookAuditorStore {
	pub async fn insert<'a>(
		db: &'a mut PgConnection,
		notebook_number: u32,
		auditor_public: &'a [u8; 32],
		auditor_order: u16,
		hosts: &'a Vec<Host>,
	) -> anyhow::Result<(), Error> {
		let host_strings = hosts
			.iter()
			.map(|h| {
				let secure = if h.is_secure { "s" } else { "" };
				format!("ws{}://{}:{}", secure, Ipv4Addr::from(h.ip.to_be_bytes()), h.port)
			})
			.collect::<Vec<_>>();
		let res = query!(
			r#"
			INSERT INTO notebook_auditors (notebook_number, public, rpc_urls, auditor_order, attempts)
			VALUES ($1, $2, $3, $4,  0)
			"#,
			notebook_number as i32,
			auditor_public,
			host_strings.as_slice(),
			auditor_order as i16,
		)
		.execute(&mut *db)
		.await?;

		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert auditor".to_string())
		);

		Ok(())
	}
	pub async fn increment_attempts<'a>(
		db: &'a mut PgConnection,
		notebook_number: u32,
		auditor_public: &'a [u8; 32],
	) -> anyhow::Result<(), Error> {
		let res = query!(
			r#"
			UPDATE notebook_auditors SET attempts = attempts+1, last_attempt=now() WHERE notebook_number = $1 AND public = $2
			"#,
			notebook_number as i32,
			auditor_public,
		)
		.execute(&mut *db)
		.await?;
		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to increment auditor attempts".to_string())
		);
		Ok(())
	}
	pub async fn update_signature<'a>(
		db: &'a mut PgConnection,
		notebook_number: u32,
		auditor_public: &'a [u8; 32],
		signature: &'a [u8; 64],
	) -> anyhow::Result<(), Error> {
		let res = query!(
			r#"
			UPDATE notebook_auditors SET signature = $3 WHERE notebook_number = $1 AND public = $2
			"#,
			notebook_number as i32,
			auditor_public,
			signature,
		)
		.execute(&mut *db)
		.await?;
		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to update signature".to_string())
		);
		Ok(())
	}

	pub async fn get_auditors(
		db: &mut PgConnection,
		notebook_number: u32,
	) -> anyhow::Result<Vec<NotebookAuditor>, Error> {
		let rows = sqlx::query_as!(
			NotebookAuditorRow,
			r#"
			SELECT * FROM notebook_auditors WHERE notebook_number = $1 ORDER BY auditor_order ASC
			"#,
			notebook_number as i32
		)
		.fetch_all(&mut *db)
		.await?;

		let result: Result<Vec<NotebookAuditor>, Error> =
			rows.into_iter().map(|r| r.try_into()).collect();

		Ok(result?)
	}
}

#[cfg(test)]
mod tests {
	use std::net::Ipv4Addr;

	use frame_support::assert_ok;
	use sqlx::PgPool;

	use ulixee_client::api::runtime_types::ulx_primitives::block_seal::Host;

	use crate::stores::notebook_auditors::NotebookAuditorStore;

	#[sqlx::test]
	async fn test_storage(pool: PgPool) -> anyhow::Result<()> {
		let notebook_number = 1;
		{
			let mut tx = pool.begin().await?;
			assert_ok!(
				NotebookAuditorStore::insert(
					&mut *tx,
					notebook_number,
					&[1u8; 32],
					1,
					&vec![Host {
						ip: Ipv4Addr::new(127, 0, 0, 1).into(),
						port: 1234,
						is_secure: false,
					}]
				)
				.await
			);
			assert_ok!(
				NotebookAuditorStore::insert(
					&mut *tx,
					notebook_number,
					&[2u8; 32],
					2,
					&vec![Host {
						ip: Ipv4Addr::new(127, 0, 0, 2).into(),
						port: 1235,
						is_secure: false,
					}]
				)
				.await
			);
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			let auditors = NotebookAuditorStore::get_auditors(&mut *tx, notebook_number).await?;
			assert_eq!(auditors.len(), 2);
			assert_eq!(auditors[0].auditor_order, 1);
			assert_eq!(auditors[1].auditor_order, 2);
			for auditor in auditors {
				assert_eq!(auditor.attempts, 0);
				NotebookAuditorStore::increment_attempts(
					&mut *tx,
					notebook_number,
					&auditor.public,
				)
				.await?;
			}
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			let auditors = NotebookAuditorStore::get_auditors(&mut *tx, notebook_number).await?;
			assert_eq!(auditors.len(), 2);
			for auditor in auditors {
				assert_eq!(auditor.attempts, 1);
				NotebookAuditorStore::update_signature(
					&mut *tx,
					notebook_number,
					&auditor.public,
					&[0u8; 64],
				)
				.await?;
			}
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			let auditors = NotebookAuditorStore::get_auditors(&mut *tx, notebook_number).await?;
			assert_eq!(auditors.len(), 2);
			for auditor in auditors {
				assert!(auditor.signature.is_some());
			}
			tx.commit().await?;
		}
		Ok(())
	}
}
