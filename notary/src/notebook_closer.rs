use std::{future::Future, sync::Arc};

use futures::FutureExt;
use sc_utils::notification::NotificationSender;
use sp_core::{ed25519, H256};
use sp_keystore::KeystorePtr;
use sqlx::{Executor, PgPool};
use subxt::utils::AccountId32;
use tokio::sync::RwLock;
use tracing::info;

use ulixee_client::{
	api,
	api::{
		runtime_types,
		runtime_types::{
			bounded_collections::bounded_vec::BoundedVec as SubxtBoundedVec, primitive_types::U256,
			sp_core::ed25519::Signature,
		},
	},
	UlxClient,
};
use ulx_notary_primitives::{ChainTransfer, NotaryId, NotebookHeader, VoteSource};

use crate::{
	error::Error,
	stores::{
		blocks::BlocksStore,
		notebook::NotebookStore,
		notebook_header::NotebookHeaderStore,
		notebook_status::{NotebookFinalizationStep, NotebookStatusStore},
		registered_key::RegisteredKeyStore,
		BoxFutureResult,
	},
};

pub const NOTARY_KEYID: sp_core::crypto::KeyTypeId = sp_core::crypto::KeyTypeId(*b"unot");

#[derive(Clone)]
pub struct NotebookCloser {
	pub pool: PgPool,
	pub keystore: KeystorePtr,
	pub notary_id: NotaryId,
	pub client: MainchainClient,

	pub completed_notebook_sender: NotificationSender<NotebookHeader>,
}

#[derive(Clone)]
pub struct MainchainClient {
	urls: Vec<String>,
	client: Arc<RwLock<Option<UlxClient>>>,
	current_index: usize,
}

impl MainchainClient {
	pub fn new(urls: Vec<String>) -> Self {
		Self { urls, client: Arc::new(RwLock::new(None)), current_index: 0 }
	}

	pub fn get(&mut self) -> BoxFutureResult<UlxClient> {
		async {
			{
				let lock = self.client.read().await;
				if let Some(client) = &*lock {
					return Ok(client.clone())
				}
			}

			self.current_index += 1;
			if self.current_index >= self.urls.len() {
				self.current_index = 0;
			}
			let url = self.urls[self.current_index].clone();

			let mut lock = self.client.write().await;
			let ulx_client = UlxClient::from_url(url).await?;
			*lock = Some(ulx_client.clone());
			drop(lock);

			Ok(ulx_client)
		}
		.boxed()
	}

	pub fn on_rpc_error(&mut self) {
		let mut lock = self.client.write().now_or_never().unwrap();
		*lock = None;
	}
}

pub fn spawn_notebook_closer(
	pool: PgPool,
	notary_id: NotaryId,
	client: MainchainClient,
	keystore: KeystorePtr,
	completed_notebook_sender: NotificationSender<NotebookHeader>,
) {
	tokio::spawn(async move {
		let mut notebook_closer =
			NotebookCloser { pool, notary_id, client, keystore, completed_notebook_sender };
		let _ = notebook_closer.create_task().await;
	});
}

impl NotebookCloser {
	pub fn create_task(
		&'_ mut self,
	) -> impl Future<Output = anyhow::Result<(), Error>> + Send + '_ {
		async move {
			loop {
				let _ = self.iterate_notebook_close_loop().await;
				// wait before resuming
				tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
			}
		}
	}

	pub(super) async fn iterate_notebook_close_loop(&mut self) {
		let _ = &self.try_rotate_notebook().await.map_err(|e| {
			tracing::error!("Error rotating notebook: {:?}", e);
			e
		});
		let _ = &self.try_close_notebook().await.map_err(|e| {
			tracing::error!("Error closing open notebook: {:?}", e);
			e
		});
		let _ = &self.try_submit_notebook().await.map_err(|e| {
			tracing::error!("Error submitting notebook: {:?}", e);
			e
		});
	}

	pub(super) fn try_rotate_notebook(&self) -> BoxFutureResult<()> {
		async move {
			let mut tx = self.pool.begin().await?;
			// NOTE: must rotate existing first. The db has a constraint to only allow a single open
			// notebook at a time.
			let notebook_number = match NotebookStatusStore::step_up_expired_open(&mut *tx).await? {
				Some(notebook_number) => notebook_number,
				None => return Ok(()),
			};
			let next_notebook = notebook_number + 1u32;
			let mut block_number = BlocksStore::get_latest_block_number(&mut *tx).await?;
			let last_notebook_block =
				NotebookHeaderStore::get_block_number(&mut *tx, notebook_number).await?;

			if block_number <= last_notebook_block {
				block_number = last_notebook_block + 1;
			}
			NotebookHeaderStore::create(&mut *tx, self.notary_id, next_notebook, block_number)
				.await?;

			tx.commit().await?;
			Ok(())
		}
		.boxed()
	}
	pub(super) fn try_close_notebook(&mut self) -> BoxFutureResult<()> {
		async move {
			let mut tx = self.pool.begin().await?;
			let step = NotebookFinalizationStep::ReadyForClose;
			let notebook_number = match NotebookStatusStore::lock_with_step(&mut *tx, step).await? {
				Some(notebook_number) => notebook_number,
				None => return Ok(()),
			};

			tx.execute("SET statement_timeout = 5000").await?;
			NotebookStatusStore::lock_to_stop_appends(&mut *tx, notebook_number).await?;

			// TODO: we can potentially improve mainchain intake speed by only referencing the
			// 	latest finalized block needed by the chain transfers/keys
			let finalized_block = BlocksStore::get_latest_finalized_block_number(&mut *tx).await?;
			let public = RegisteredKeyStore::get_valid_public(&mut *tx, finalized_block).await?;

			NotebookStore::close_notebook(
				&mut *tx,
				notebook_number,
				finalized_block,
				self.notary_id,
				public,
				&self.keystore,
			)
			.await?;

			NotebookStatusStore::next_step(&mut *tx, notebook_number, step).await?;
			tx.commit().await?;

			let header = NotebookHeaderStore::load(&self.pool, notebook_number).await?;

			let _ =
				self.completed_notebook_sender
					.notify(|| Ok(header))
					.map_err(|e: anyhow::Error| {
						tracing::error!("Error sending completed notebook notification {:?}", e);
					});

			Ok(())
		}
		.boxed()
	}

	fn try_submit_notebook(&mut self) -> BoxFutureResult<Option<NotebookHeader>> {
		async move {
			let pool = &self.pool;
			let mut tx = pool.begin().await?;
			let step = NotebookFinalizationStep::Closed;
			let notebook_number = match NotebookStatusStore::lock_with_step(&mut *tx, step).await? {
				Some(notebook_number) => notebook_number,
				None => return Ok(None),
			};

			let header = NotebookHeaderStore::load(&mut *tx, notebook_number).await?;
			let public =
				RegisteredKeyStore::get_valid_public(&mut *tx, header.block_number).await?;
			let header_hash = header.hash();

			let notebook = runtime_types::ulx_notary_primitives::notebook::NotebookHeader {
				version: header.version,
				block_number: header.block_number,
				finalized_block_number: header.finalized_block_number,
				notebook_number: header.notebook_number,
				notary_id: header.notary_id,
				start_time: header.start_time,
				end_time: header.end_time,
				changed_accounts_root: header.changed_accounts_root,
				changed_account_origins: SubxtBoundedVec(
					header
						.changed_account_origins
						.iter()
						.map(|a| {
							runtime_types::ulx_notary_primitives::balance_change::AccountOrigin {
								notebook_number: a.notebook_number,
								account_uid: a.account_uid,
							}
						})
						.collect(),
				),
				tax: header.tax,
				chain_transfers: SubxtBoundedVec(
					header
						.chain_transfers
						.iter()
						.map(|t| {
							match t {
									ChainTransfer::ToMainchain { account_id, amount } =>
										runtime_types::ulx_notary_primitives::notebook::ChainTransfer::ToMainchain {
											account_id: AccountId32::from(Into::<[u8; 32]>::into(account_id.clone())),
											amount: amount.clone(),
										},
									ChainTransfer::ToLocalchain { account_id, account_nonce } =>
										runtime_types::ulx_notary_primitives::notebook::ChainTransfer::ToLocalchain {
											account_id: AccountId32::from(Into::<[u8; 32]>::into(account_id.clone())),
											account_nonce: account_nonce.clone(),
										},
								}
						})
						.collect(),
				),
				block_voting_power: header.block_voting_power,
				block_votes_count: header.block_votes_count,
				block_votes_root: header.block_votes_root,
				secret_hash: header.secret_hash,
				parent_secret: header.parent_secret,
				best_block_nonces: SubxtBoundedVec(
					header
						.best_block_nonces
						.iter()
						.map(|(voting_root, block_nonce)| {
							let vote = block_nonce.block_vote.clone();
							let api_best_nonce = runtime_types::ulx_notary_primitives::block_vote::BestBlockNonceT {
								nonce: U256(block_nonce.nonce.0),
								proof: runtime_types::ulx_notary_primitives::balance_change::MerkleProof {
									proof: SubxtBoundedVec(
										block_nonce.proof.proof.iter().map(|p| p.clone()).collect(),
									),
									leaf_index: block_nonce.proof.leaf_index,
									number_of_leaves: block_nonce.proof.number_of_leaves,
								},
								block_vote: runtime_types::ulx_notary_primitives::block_vote::BlockVoteT {
									account_id: AccountId32(vote.account_id.into()),
									block_hash: vote.block_hash,
									power: vote.power,
									index: vote.index,
									vote_source: match vote.vote_source {
										VoteSource::Tax { channel_pass } => {
											runtime_types::ulx_notary_primitives::block_vote::VoteSource::Tax {
												channel_pass: runtime_types::ulx_notary_primitives::note::ChannelPass {
													at_block_height: channel_pass.at_block_height,
													id: channel_pass.id,
													miner_index: channel_pass.miner_index,
													zone_record_hash: channel_pass.zone_record_hash,
												},
											}
										}
										VoteSource::Compute { puzzle_proof} => {
											runtime_types::ulx_notary_primitives::block_vote::VoteSource::Compute {
												puzzle_proof: U256(puzzle_proof.0),
											}
										}
									}
								},
							};
							(voting_root.clone(), api_best_nonce)
						})
						.collect::<Vec<_>>(),
				),
			};

			let signature = notary_sign(&self.keystore, &public, &header_hash)?;

			let ext =
				api::tx()
					.notebook()
					.submit(notebook.clone(), header_hash, Signature(signature.0));

			let client = self.client.get().await?;
			let ext = client.tx().create_unsigned(&ext)?;
			let submission = ext.submit_and_watch().await.map_err(|e| {
				self.client.on_rpc_error();
				e
			})?;

			NotebookStatusStore::next_step(&mut *tx, notebook_number, step).await?;
			tx.commit().await?;

			let included_in_block =
				submission.wait_for_in_block().await?.wait_for_success().await?;
			info!("Submitted notebook {}. In block {:?}", notebook_number, included_in_block);

			Ok(Some(header))
		}
		.boxed()
	}
}

pub fn notary_sign(
	keystore: &KeystorePtr,
	public: &ed25519::Public,
	hash: &H256,
) -> anyhow::Result<ed25519::Signature, Error> {
	let sig = keystore
		.ed25519_sign(NOTARY_KEYID, &public, &hash[..])
		.map_err(|e| {
			Error::InternalError(format!(
				"Unable to sign notebook header for submission to mainchain {}",
				e
			))
		})?
		.unwrap();
	Ok(sig)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct NotebookAuditResponse {
	pub signature: String,
}

#[cfg(test)]
mod tests {
	use std::{env, net::Ipv4Addr};

	use frame_support::assert_ok;
	use futures::StreamExt;
	use sp_core::{bounded_vec, ed25519::Public};
	use sp_keyring::Sr25519Keyring::Bob;
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
	use sqlx::PgPool;
	use subxt::utils::AccountId32;
	use subxt_signer::sr25519::dev;

	use ulixee_client::{
		api::{
			runtime_apis::account_nonce_api::AccountNonceApi,
			runtime_types::{
				bounded_collections::bounded_vec::BoundedVec,
				pallet_notaries::pallet::Call as NotaryCall,
				sp_core::ed25519,
				ulx_node_runtime::RuntimeCall,
				ulx_notary_primitives::balance_change::AccountOrigin,
				ulx_primitives::{block_seal::Host, notary::NotaryMeta},
			},
			storage, tx,
		},
		UlxClient,
	};
	use ulx_notary_primitives::{AccountType::Deposit, BalanceChange, Note};
	use ulx_testing::{test_context, test_context_from_url};

	use crate::{
		block_watch::track_blocks,
		notebook_closer::NOTARY_KEYID,
		server::NotebookHeaderStream,
		stores::{notarizations::NotarizationsStore, notebook_status::NotebookStatusStore},
	};

	use super::*;

	#[sqlx::test]
	async fn test_notebook_cycle(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let use_live = env::var("USE_LIVE").unwrap_or(String::from("false")).parse::<bool>()?;
		let ctx = if use_live {
			test_context_from_url("ws://localhost:9944").await
		} else {
			test_context().await
		};
		let bob_id = dev::bob().public_key().to_account_id();

		let ws_url = ctx.ws_url.clone();
		let keystore = MemoryKeystore::new();
		let keystore = KeystoreExt::new(keystore);
		let notary_key =
			keystore.ed25519_generate_new(NOTARY_KEYID, None).expect("should have a key");

		track_blocks(ctx.ws_url, 1, &pool.clone());

		propose_bob_as_notary(&ctx.client, notary_key).await?;

		activate_notary(&pool, &ctx.client, &bob_id).await?;

		{
			let mut tx = pool.begin().await?;
			assert_eq!(
				NotebookStatusStore::lock_open_for_appending(&mut *tx).await?,
				1,
				"There should be a notebook active now"
			);
		}

		let bob_nonce = get_bob_nonce(&ctx.client, bob_id).await?;

		// Submit a transfer to localchain and wait for result
		create_localchain_transfer(&pool, &ctx.client, bob_nonce).await?;

		// Record the balance change
		submit_balance_change_to_notary(&pool, bob_nonce).await?;

		sqlx::query("update notebook_status set open_time = now() - interval '2 minutes' where notebook_number = 1")
			.execute(&pool)
			.await?;

		let (completed_notebook_sender, completed_notebook_stream) =
			NotebookHeaderStream::channel();

		let mut closer = NotebookCloser {
			pool: pool.clone(),
			keystore: keystore.clone(),
			notary_id: 1,
			client: MainchainClient::new(vec![ws_url.clone()]),
			completed_notebook_sender,
		};

		let mut subscription = completed_notebook_stream.subscribe(100);

		loop {
			let _ = &closer.iterate_notebook_close_loop().await;
			let status = NotebookStatusStore::get(&pool, 1).await?;
			if status.finalized_time.is_some() {
				break
			}
			// yield
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		}

		let next_header = subscription.next().await;
		assert_eq!(next_header.is_some(), true);
		let next_header = next_header.expect("Should have a header");

		assert_eq!(
			ctx.client
				.storage()
				.at_latest()
				.await?
				.fetch(&storage().notebook().account_origin_last_changed_notebook_by_notary(
					1,
					AccountOrigin { notebook_number: 1, account_uid: 1 }
				))
				.await?,
			Some(1),
			"Should have updated Bob's last change notebook"
		);

		assert_eq!(
			ctx.client
				.storage()
				.at_latest()
				.await?
				.fetch(&storage().notebook().notebook_changed_accounts_root_by_notary(1, 1))
				.await?,
			next_header.changed_accounts_root.into(),
			"Should have updated Bob's last change notebook"
		);

		let last_notary_details = ctx
			.client
			.storage()
			.at_latest()
			.await?
			.fetch(&storage().notebook().last_notebook_details_by_notary(1))
			.await?
			.expect("should get details")
			.0;

		assert_eq!(
			last_notary_details[0].block_number, next_header.block_number,
			"Should have updated the last block number"
		);
		assert_eq!(
			last_notary_details[0].notebook_number, 1,
			"Should have updated the last notebook number"
		);

		Ok(())
	}

	async fn propose_bob_as_notary(client: &UlxClient, notary_key: Public) -> anyhow::Result<()> {
		let notary_proposal = tx().notaries().propose(NotaryMeta {
			hosts: BoundedVec(vec![Host {
				is_secure: false,
				ip: Ipv4Addr::LOCALHOST.into(),
				port: 0u16,
			}]),
			public: ed25519::Public(notary_key.0),
		});

		let result = client
			.tx()
			.sign_and_submit_then_watch_default(&notary_proposal, &dev::bob())
			.await?
			.wait_for_in_block()
			.await?
			.wait_for_success()
			.await;

		assert_ok!(result);
		Ok(())
	}

	async fn submit_balance_change_to_notary(pool: &PgPool, bob_nonce: u32) -> anyhow::Result<()> {
		let result = NotarizationsStore::apply(
			&pool,
			1,
			vec![BalanceChange {
				account_id: Bob.to_account_id(),
				account_type: Deposit,
				change_number: 1,
				balance: 1000,
				previous_balance_proof: None,
				notes: bounded_vec![Note::create(
					1000,
					ulx_notary_primitives::NoteType::ClaimFromMainchain {
						account_nonce: bob_nonce
					},
				)],
				channel_hold_note: None,
				signature: sp_core::ed25519::Signature([0u8; 64]).into(),
			}
			.sign(Bob.pair())
			.clone()],
			vec![],
		)
		.await?;
		assert_eq!(result.notebook_number, 1);

		Ok(())
	}

	async fn get_bob_nonce(client: &UlxClient, bob_id: AccountId32) -> anyhow::Result<u32> {
		let bob_nonce = client
			.runtime_api()
			.at_latest()
			.await?
			.call(AccountNonceApi.account_nonce(bob_id.clone()))
			.await?;
		Ok(bob_nonce)
	}

	async fn create_localchain_transfer(
		pool: &PgPool,
		client: &UlxClient,
		bob_nonce: u32,
	) -> anyhow::Result<()> {
		client
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().chain_transfer().send_to_localchain(1000u32.into(), 1, bob_nonce),
				&dev::bob(),
			)
			.await?
			.wait_for_finalized_success()
			.await?;

		let mut found = false;
		for _ in 0..5 {
			let row = sqlx::query!("select * from chain_transfers").fetch_optional(pool).await?;
			if let Some(_) = row {
				found = true;
				break
			}
			// wait for 500 ms
			tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		}
		assert_eq!(found, true, "Should have recorded a chain transfer");

		Ok(())
	}

	async fn activate_notary(
		pool: &PgPool,
		client: &UlxClient,
		bob_id: &AccountId32,
	) -> anyhow::Result<()> {
		let notary_activated_finalized_block = client
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().sudo().sudo(RuntimeCall::Notaries(NotaryCall::activate {
					operator_account: bob_id.clone(),
				})),
				&dev::alice(),
			)
			.await?
			.wait_for_finalized_success()
			.await;

		assert_ok!(&notary_activated_finalized_block);
		let notary_activated_finalized_block = notary_activated_finalized_block.unwrap();

		let mut found = false;
		for _ in 0..5 {
			let meta = sqlx::query!(
				"select * from blocks where block_hash=$1",
				&notary_activated_finalized_block.block_hash().0
			)
			.fetch_optional(pool)
			.await?;
			if let Some(_) = meta {
				found = true;
				break
			}
			// wait for 500 ms
			tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		}
		assert_eq!(found, true, "Should have found the finalized block");
		Ok(())
	}
}
