use std::collections::BTreeSet;

use crate::{
	Error,
	stores::{
		BoxFutureResult, notarizations::NotarizationsStore,
		notebook_constraints::NotebookConstraintsStore,
		notebook_new_accounts::NotebookNewAccountsStore, notebook_status::NotebookStatusStore,
	},
};
use argon_primitives::{
	AccountOrigin, BlockVotingPower, ChainTransfer, DomainHash, NOTEBOOK_VERSION, NotebookHeader,
	NotebookMeta, SignedNotebookHeader, ensure, notary::NotarySignature, prelude::*,
};
use chrono::{DateTime, TimeZone, Utc};
use codec::Encode;
use rand::RngCore;
use serde_json::{from_value, json};
use sp_core::{H256, bounded::BoundedVec};
use sqlx::{FromRow, PgConnection, query, types::JsonValue};

#[derive(FromRow)]
#[allow(dead_code)]
struct NotebookHeaderRow {
	pub version: i32,
	pub notebook_number: i32,
	pub hash: Option<Vec<u8>>,
	pub signature: Option<Vec<u8>>,
	pub tick: i64,
	pub notary_id: i32,
	pub tax: Option<String>,
	pub chain_transfers: JsonValue,
	pub changed_accounts_root: Vec<u8>,
	pub changed_account_origins: JsonValue,
	pub block_votes_root: Vec<u8>,
	pub block_votes_count: i32,
	pub block_voting_power: String,
	pub blocks_with_votes: Vec<Vec<u8>>,
	pub secret_hash: Vec<u8>,
	pub parent_secret: Option<Vec<u8>>,
	pub last_updated: chrono::DateTime<Utc>,
	pub domains: JsonValue,
}

impl TryInto<NotebookHeader> for NotebookHeaderRow {
	type Error = Error;
	fn try_into(self) -> Result<NotebookHeader, Error> {
		Ok(NotebookHeader {
			version: self.version as u16,
			notebook_number: self.notebook_number as NotebookNumber,
			tick: self.tick as Tick,
			notary_id: self.notary_id as NotaryId,
			tax: self
				.tax
				.unwrap_or("0".to_string())
				.parse::<u128>()
				.map_err(|e| Error::InternalError(e.to_string()))?,
			chain_transfers: BoundedVec::truncate_from(from_value(self.chain_transfers)?),
			changed_accounts_root: H256::from_slice(&self.changed_accounts_root[..]),
			changed_account_origins: BoundedVec::truncate_from(from_value(
				self.changed_account_origins,
			)?),
			block_votes_root: H256::from_slice(&self.block_votes_root[..]),
			block_votes_count: self.block_votes_count as u32,
			block_voting_power: self.block_voting_power.parse::<u128>().map_err(|e| {
				Error::InternalError(format!("Error decoding block voting power: {:?}", e))
			})?,
			blocks_with_votes: BoundedVec::truncate_from(
				self.blocks_with_votes
					.into_iter()
					.map(|a| H256::from_slice(a.as_slice()))
					.collect::<Vec<_>>(),
			),
			secret_hash: H256::from_slice(&self.secret_hash[..]),
			parent_secret: self.parent_secret.map(|a| H256::from_slice(&a[..])),
			domains: BoundedVec::truncate_from(from_value(self.domains)?),
		})
	}
}

pub struct NotebookHeaderStore;

impl NotebookHeaderStore {
	async fn save_secret(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
		secret: H256,
	) -> anyhow::Result<(), Error> {
		let res = query!(
			r#"
			INSERT INTO notebook_secrets (notebook_number, secret)
			VALUES ($1, $2)
			"#,
			notebook_number as i32,
			secret.as_bytes()
		)
		.execute(db)
		.await?;

		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert notebook secret".to_string())
		);
		Ok(())
	}

	async fn create_header(
		db: &mut PgConnection,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		tick: u64,
	) -> anyhow::Result<(), Error> {
		let version = NOTEBOOK_VERSION;
		let empty = json!([]);

		let res = query!(
			r#"
			INSERT INTO notebook_headers (version, notary_id, tick, notebook_number, chain_transfers,
				changed_account_origins, changed_accounts_root, secret_hash, block_votes_root,
				block_voting_power, block_votes_count, blocks_with_votes, domains)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
			"#,
			version as i16,
			notary_id as i32,
			tick as i64,
			notebook_number as i32,
			empty.clone(),
			empty.clone(),
			&[0u8; 32],
			&[0u8; 32],
			&[0u8; 32],
			0.to_string(),
			0,
			&[],
			empty.clone(),
		)
		.execute(db)
		.await?;

		ensure!(
			res.rows_affected() == 1,
			Error::InternalError("Unable to insert notebook header".to_string())
		);
		Ok(())
	}

	/// Creates the next notebook header and returns the current notebook id.
	///
	/// NOTE: there might still be some uncommitted changes in the notebook. Must wait for the row
	/// to have no more share locks
	pub fn create(
		db: &mut PgConnection,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		tick: Tick,
		end_time_for_tick: u64,
	) -> BoxFutureResult<()> {
		Box::pin(async move {
			Self::create_header(&mut *db, notary_id, notebook_number, tick).await?;
			NotebookNewAccountsStore::reset_seq(&mut *db, notebook_number).await?;
			NotarizationsStore::reset_seq(&mut *db, notebook_number).await?;
			NotebookConstraintsStore::create(&mut *db, notebook_number).await?;
			NotebookStatusStore::create(
				&mut *db,
				notebook_number,
				tick,
				Utc.from_utc_datetime(
					&DateTime::from_timestamp_millis(end_time_for_tick as i64).unwrap().naive_utc(),
				),
			)
			.await?;
			Ok(())
		})
	}
	pub async fn latest<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
	) -> anyhow::Result<NotebookMeta, Error> {
		let record = sqlx::query!(
			r#"
				SELECT notebook_number, tick
				FROM notebook_headers WHERE hash IS NOT NULL ORDER BY notebook_number DESC LIMIT 1
				"#,
		)
		.fetch_optional(db)
		.await?;
		let Some(record) = record else {
			return Ok(NotebookMeta {
				last_closed_notebook_tick: 0,
				last_closed_notebook_number: 0,
			});
		};

		Ok(NotebookMeta {
			last_closed_notebook_tick: record.tick as Tick,
			last_closed_notebook_number: record.notebook_number as NotebookNumber,
		})
	}

	pub async fn get_hash(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<H256, Error> {
		let row = sqlx::query_scalar!(
			"SELECT hash FROM notebook_headers WHERE hash IS NOT NULL AND notebook_number = $1 LIMIT 1",
			notebook_number as i32
		)
		.fetch_one(db)
		.await?;
		if let Some(row) = row {
			Ok(H256::from_slice(row.as_slice()))
		} else {
			Err(Error::Database("Notebook header not found".to_string()))
		}
	}

	pub async fn load<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<NotebookHeader, Error> {
		let record = sqlx::query_as!(
			NotebookHeaderRow,
			r#"
				SELECT *
				FROM notebook_headers WHERE notebook_number = $1 LIMIT 1
				"#,
			notebook_number as i32
		)
		.fetch_one(db)
		.await?;

		record.try_into()
	}

	pub async fn load_raw_header<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<Vec<u8>, Error> {
		let signed_header = Self::load_with_signature(db, notebook_number).await?;
		Ok(signed_header.encode())
	}

	pub async fn load_with_signature<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<SignedNotebookHeader, Error> {
		let record = sqlx::query_as!(
			NotebookHeaderRow,
			r#"
				SELECT *
				FROM notebook_headers WHERE notebook_number = $1
				AND signature IS NOT NULL
				 LIMIT 1
				"#,
			notebook_number as i32
		)
		.fetch_one(db)
		.await?;

		let bytes: [u8; 64] = record
			.signature
			.clone()
			.unwrap_or_default()
			.try_into()
			.map_err(|_| Error::UnsignedNotebookHeader)?;
		let signature = NotarySignature::from_raw(bytes);
		let header: NotebookHeader = record.try_into()?;
		let signed_header = SignedNotebookHeader { header, signature };

		Ok(signed_header)
	}

	pub async fn get_notebook_tick(
		db: &mut PgConnection,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<Tick, Error> {
		let row = sqlx::query_scalar!(
			"SELECT tick FROM notebook_headers WHERE notebook_number = $1 LIMIT 1",
			notebook_number as i32
		)
		.fetch_one(db)
		.await?;
		Ok(row as Tick)
	}

	pub async fn get_changed_accounts_root<'a>(
		db: impl sqlx::PgExecutor<'a> + 'a,
		notebook_number: NotebookNumber,
	) -> anyhow::Result<H256, Error> {
		let record = sqlx::query_scalar!(
			r#"
				SELECT changed_accounts_root
				FROM notebook_headers WHERE notebook_number = $1 LIMIT 1
				"#,
			notebook_number as i32
		)
		.fetch_one(db)
		.await?;

		let data: [u8; 32] = record.try_into().map_err(|_| {
			Error::InternalError(format!(
				"Error decoding notebook accounts_root for {}",
				notebook_number
			))
		})?;
		Ok(data.into())
	}

	pub async fn add_secrets(
		db: &mut PgConnection,
		header: &mut NotebookHeader,
	) -> anyhow::Result<(), Error> {
		let notebook_number = header.notebook_number;
		// set the secret
		let mut secret = [0u8; 32];
		rand::rng().fill_bytes(&mut secret);
		header.secret_hash = header.hash_secret(secret.into());

		Self::save_secret(db, notebook_number, H256::from_slice(&secret)).await?;
		let parent_secret = sqlx::query_scalar!(
			"SELECT secret FROM notebook_secrets WHERE notebook_number = $1 LIMIT 1",
			(notebook_number - 1) as i32
		)
		.fetch_optional(&mut *db)
		.await?;

		header.parent_secret = parent_secret.map(|a| H256::from_slice(&a[..]));
		Ok(())
	}

	#[allow(clippy::too_many_arguments)]
	pub fn complete_notebook<'a>(
		db: &'a mut PgConnection,
		notebook_number: NotebookNumber,
		transfers: Vec<ChainTransfer>,
		domains: Vec<(DomainHash, AccountId)>,
		tax: u128,
		changed_accounts_root: H256,
		account_changelist: Vec<AccountOrigin>,
		block_votes_root: H256,
		block_votes_count: u32,
		blocks_with_votes: BTreeSet<H256>,
		block_voting_power: BlockVotingPower,
		sign_fn: impl FnOnce(&H256) -> Result<NotarySignature, Error> + Send + 'a,
	) -> BoxFutureResult<'a, ()> {
		Box::pin(async move {
			let mut header = Self::load(&mut *db, notebook_number).await?;
			header.chain_transfers = BoundedVec::try_from(transfers).map_err(|_| {
				Error::InternalError(
					"Unable to bound chain transfers. Possibly exceeded max size.".to_string(),
				)
			})?;
			header.domains = BoundedVec::try_from(domains).map_err(|_| {
				Error::InternalError(
					"Unable to bound domains. Possibly exceeded max size.".to_string(),
				)
			})?;
			header.changed_accounts_root = changed_accounts_root;
			// need to sort the changelist
			let account_changelist = BTreeSet::from_iter(account_changelist);
			header.changed_account_origins =
				BoundedVec::truncate_from(account_changelist.into_iter().collect::<Vec<_>>());
			header.block_votes_root = block_votes_root;
			header.block_votes_count = block_votes_count;
			header.blocks_with_votes =
				BoundedVec::truncate_from(blocks_with_votes.into_iter().collect::<Vec<_>>());
			header.block_voting_power = block_voting_power;
			header.tax = tax;

			Self::add_secrets(db, &mut header).await?;

			let hash = header.hash().0;
			let signature = sign_fn(&H256::from_slice(&hash[..]))?;

			let res = sqlx::query!(
				r#"
				UPDATE notebook_headers
				SET hash = $1,
					changed_accounts_root = $2,
					changed_account_origins = $3,
					chain_transfers = $4,
					tax=$5,
					block_voting_power=$6,
					block_votes_root=$7,
					block_votes_count=$8,
					blocks_with_votes=$9,
					secret_hash=$10,
					parent_secret=$11,
					signature=$12,
					domains=$13
				WHERE notebook_number = $14
			"#,
				&hash,
				header.changed_accounts_root.as_bytes(),
				json!(header.changed_account_origins.to_vec()),
				json!(header.chain_transfers.to_vec()),
				header.tax as i64,
				header.block_voting_power.to_string(),
				header.block_votes_root.as_bytes(),
				header.block_votes_count as i32,
				&header
					.blocks_with_votes
					.into_iter()
					.map(|a| a.as_bytes().to_vec())
					.collect::<Vec<_>>()[..],
				header.secret_hash.as_bytes(),
				header.parent_secret.map(|a| {
					let data = a.clone();
					data.as_bytes().to_vec()
				}),
				&signature.0,
				json!(header.domains.to_vec()),
				header.notebook_number as i32,
			)
			.execute(db)
			.await?;

			ensure!(
				res.rows_affected() == 1,
				Error::InternalError("Notebook header not updated".to_string())
			);

			Ok(())
		})
	}
}

#[cfg(test)]
mod tests {
	use polkadot_sdk::*;
	use std::ops::Add;

	use chrono::{Duration, Utc};
	use sp_core::H256;
	use sp_keyring::Sr25519Keyring::Alice;
	use sp_keystore::{KeystoreExt, testing::MemoryKeystore};
	use sp_runtime::traits::Verify;
	use sqlx::PgPool;

	use argon_primitives::{AccountOrigin, ChainTransfer, NOTEBOOK_VERSION};

	use crate::{
		notebook_closer::{NOTARY_KEYID, notary_sign},
		stores::notebook_header::NotebookHeaderStore,
	};

	#[sqlx::test]
	async fn test_storage(pool: PgPool) -> anyhow::Result<()> {
		let notebook_number = 1;
		{
			let mut tx = pool.begin().await?;

			NotebookHeaderStore::create(
				&mut tx,
				1,
				notebook_number,
				1,
				Utc::now().add(Duration::try_minutes(1).unwrap()).timestamp_millis() as u64,
			)
			.await?;

			let loaded = NotebookHeaderStore::load(&mut *tx, notebook_number).await?;
			assert_eq!(loaded.notebook_number, notebook_number);
			assert_eq!(loaded.version, NOTEBOOK_VERSION);
			assert_eq!(loaded.tick, 1);
			assert_eq!(loaded.notary_id, 1);
			assert_eq!(loaded.chain_transfers.len(), 0);

			tx.commit().await?;

			assert_eq!(
				NotebookHeaderStore::get_changed_accounts_root(&pool, notebook_number).await?,
				H256([0u8; 32])
			);
		}

		Ok(())
	}
	#[sqlx::test]
	async fn test_cannot_load_before_close(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let notebook_number = 1;
		{
			let mut tx = pool.begin().await?;

			NotebookHeaderStore::create(
				&mut tx,
				1,
				notebook_number,
				1,
				Utc::now().add(Duration::try_minutes(1).unwrap()).timestamp_millis() as u64,
			)
			.await?;

			tx.commit().await?;
		}
		assert!(NotebookHeaderStore::load_with_signature(&pool, notebook_number).await.is_err());
		Ok(())
	}

	#[sqlx::test]
	async fn test_close(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let keystore = MemoryKeystore::new();
		let keystore = KeystoreExt::new(keystore);
		let notary_key =
			keystore.ed25519_generate_new(NOTARY_KEYID, None).expect("should have a key");
		let notebook_number = 1;
		{
			let mut tx = pool.begin().await?;

			NotebookHeaderStore::create(
				&mut tx,
				1,
				notebook_number,
				1,
				Utc::now().add(Duration::try_minutes(1).unwrap()).timestamp_millis() as u64,
			)
			.await?;

			tx.commit().await?;
		}

		{
			let mut tx = pool.begin().await?;
			NotebookHeaderStore::complete_notebook(
				&mut tx,
				notebook_number,
				vec![
					ChainTransfer::ToLocalchain { transfer_id: 1 },
					ChainTransfer::ToMainchain { account_id: Alice.to_account_id(), amount: 100 },
				],
				vec![],
				0,
				[1u8; 32].into(),
				vec![
					AccountOrigin { notebook_number: 1, account_uid: 1 },
					AccountOrigin { notebook_number: 1, account_uid: 2 },
				],
				[1u8; 32].into(),
				0,
				Default::default(),
				0,
				|h| notary_sign(&keystore, &notary_key, h),
			)
			.await?;
			tx.commit().await?;
		}
		{
			let mut tx = pool.begin().await?;
			let header = NotebookHeaderStore::load(&mut *tx, notebook_number).await?;

			assert_eq!(header.chain_transfers.len(), 2);
			assert_eq!(header.chain_transfers[0], ChainTransfer::ToLocalchain { transfer_id: 1 });
			assert_eq!(
				header.chain_transfers[1],
				ChainTransfer::ToMainchain { account_id: Alice.to_account_id(), amount: 100 }
			);

			assert_eq!(header.tick, 1);
			assert_eq!(header.changed_accounts_root, H256([1u8; 32]));
			assert_eq!(header.changed_account_origins.len(), 2);
			assert_eq!(header.changed_account_origins[0].account_uid, 1);
			assert_eq!(header.changed_account_origins[1].account_uid, 2);
		}
		{
			let mut tx = pool.begin().await?;
			let header =
				NotebookHeaderStore::load_with_signature(&mut *tx, notebook_number).await?;

			assert!(header.signature.verify(&header.header.hash()[..], &notary_key));
		}
		Ok(())
	}
}
