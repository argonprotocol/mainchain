use std::{future::Future, sync::Arc};

use futures::FutureExt;
use sc_utils::notification::NotificationSender;
use sp_core::{ed25519, H256};
use sp_keystore::KeystorePtr;
use sqlx::PgPool;
use tokio::sync::RwLock;

use ulixee_client::{api, UlxClient};
use ulx_primitives::{prod_or_fast, tick::Ticker, NotaryId, SignedNotebookHeader};

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
	pub ticker: Ticker,

	pub completed_notebook_sender: NotificationSender<SignedNotebookHeader>,
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

	pub async fn lookup_ticker(&mut self) -> anyhow::Result<Ticker> {
		let client = self.get().await?;
		let ticker_data = client
			.runtime_api()
			.at(client.genesis_hash())
			.call(api::runtime_apis::tick_apis::TickApis.ticker())
			.await?;
		let ticker = Ticker::new(ticker_data.tick_duration_millis, ticker_data.genesis_utc_time);
		Ok(ticker)
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
	ticker: Ticker,
	completed_notebook_sender: NotificationSender<SignedNotebookHeader>,
) {
	tokio::spawn(async move {
		let mut notebook_closer =
			NotebookCloser { pool, notary_id, client, keystore, ticker, completed_notebook_sender };
		let _ = notebook_closer.create_task().await;
	});
}

const LOOP_MILLIS: u64 = prod_or_fast!(1000, 200);

impl NotebookCloser {
	pub fn create_task(
		&'_ mut self,
	) -> impl Future<Output = anyhow::Result<(), Error>> + Send + '_ {
		async move {
			loop {
				let _ = self.iterate_notebook_close_loop().await;
				// wait before resuming
				tokio::time::sleep(tokio::time::Duration::from_millis(LOOP_MILLIS)).await;
			}
		}
	}

	pub(super) async fn iterate_notebook_close_loop(&mut self) {
		let _ = &self.try_rotate_notebook().await.map_err(|e| {
			tracing::error!("Error rotating notebook: {:?}", e);
		});
		let _ = &self.try_close_notebook().await.map_err(|e| {
			tracing::error!("Error closing open notebook: {:?}", e);
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
			let tick = self.ticker.current();
			let end_time = self.ticker.time_for_tick(tick + 1);

			NotebookHeaderStore::create(&mut *tx, self.notary_id, next_notebook, tick, end_time)
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
			let notebook_number =
				match NotebookStatusStore::find_and_lock_ready_for_close(&mut *tx).await? {
					Some(notebook_number) => notebook_number,
					None => return Ok(()),
				};

			// TODO: we can potentially improve mainchain intake speed by only referencing the
			// 	latest finalized block needed by the chain transfers/keys
			let finalized_block = BlocksStore::get_latest_finalized_block_number(&mut *tx).await?;
			let public = RegisteredKeyStore::get_valid_public(&mut *tx, finalized_block).await?;

			NotebookStore::close_notebook(
				&mut *tx,
				notebook_number,
				finalized_block,
				public,
				&self.keystore,
			)
			.await?;

			NotebookStatusStore::next_step(&mut *tx, notebook_number, step).await?;
			tx.commit().await?;

			let header =
				NotebookHeaderStore::load_with_signature(&self.pool, notebook_number).await?;

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
	use chrono::Utc;
	use codec::Decode;
	use frame_support::assert_ok;
	use futures::StreamExt;
	use sp_core::{bounded_vec, ed25519::Public};
	use sp_keyring::Sr25519Keyring::{Alice, Bob};
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
	use sqlx::PgPool;
	use std::{
		env,
		net::{IpAddr, SocketAddr},
	};
	use subxt::{
		blocks::Block,
		config::substrate::DigestItem,
		tx::{TxInBlock, TxProgress, TxStatus},
		utils::AccountId32,
		OnlineClient,
	};
	use subxt_signer::sr25519::dev;

	use ulixee_client::{
		api::{
			runtime_apis::account_nonce_api::AccountNonceApi,
			runtime_types::{
				bounded_collections::bounded_vec::BoundedVec,
				pallet_notaries::pallet::Call as NotaryCall,
				sp_core::ed25519,
				ulx_node_runtime::RuntimeCall,
				ulx_primitives::{
					balance_change::AccountOrigin as SubxtAccountOrigin, block_seal::Host,
					notary::NotaryMeta,
				},
			},
			storage, tx,
		},
		UlxClient, UlxConfig,
	};
	use ulx_notary_audit::VerifyError;
	use ulx_primitives::{
		tick::Tick,
		AccountOrigin,
		AccountType::{Deposit, Tax},
		BalanceChange, BalanceProof, BalanceTip, BlockSealDigest, BlockVote, BlockVoteDigest,
		ChannelPass, HashOutput, MerkleProof, Note, NoteType,
		NoteType::{ChannelClaim, ChannelSettle},
		NotebookDigest, ParentVotingKeyDigest, TickDigest, CHANNEL_EXPIRATION_NOTEBOOKS,
	};
	use ulx_testing::{test_context, test_context_from_url};

	use crate::{
		apis::localchain::BalanceChangeResult,
		block_watch::track_blocks,
		notebook_closer::NOTARY_KEYID,
		stores::{notarizations::NotarizationsStore, notebook_status::NotebookStatusStore},
		NotaryServer,
	};

	use super::*;

	#[sqlx::test]
	async fn test_chain_to_chain(pool: PgPool) -> anyhow::Result<()> {
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

		let mut client = MainchainClient::new(vec![ws_url.clone()]);
		let ticker = client.lookup_ticker().await?;
		let server = NotaryServer::create_http_server("127.0.0.1:0").await?;
		track_blocks(ctx.ws_url, 1, &pool.clone(), ticker.clone());

		propose_bob_as_notary(&ctx.client, notary_key, server.local_addr()?).await?;

		let mut notary_server = NotaryServer::start_with(server, 1, pool.clone()).await?;

		activate_notary(&pool, &ctx.client, &bob_id).await?;

		{
			let mut tx = pool.begin().await?;
			assert_eq!(
				NotebookStatusStore::lock_open_for_appending(&mut *tx).await?.0,
				1,
				"There should be a notebook active now"
			);
		}

		let bob_nonce = get_bob_nonce(&ctx.client, bob_id).await?;

		// Submit a transfer to localchain and wait for result
		create_localchain_transfer(&pool, &ctx.client, bob_nonce, 1000).await?;

		// Record the balance change
		submit_balance_change_to_notary(&pool, bob_nonce, 1000).await?;

		sqlx::query("update notebook_status set end_time = $1 where notebook_number = 1")
			.bind(Utc::now())
			.execute(&pool)
			.await?;

		let mut closer = NotebookCloser {
			pool: pool.clone(),
			keystore: keystore.clone(),
			notary_id: 1,
			client,
			completed_notebook_sender: notary_server.completed_notebook_sender.clone(),
			ticker: ticker.clone(),
		};

		let mut subscription = notary_server.completed_notebook_stream.subscribe(100);

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
					SubxtAccountOrigin { notebook_number: 1, account_uid: 1 }
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
			next_header.header.changed_accounts_root.into(),
			"Should have updated Bob's last change notebook"
		);
		notary_server.stop().await;
		Ok(())
	}

	#[sqlx::test]
	async fn test_submitting_votes(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let use_live = env::var("USE_LIVE").unwrap_or(String::from("false")).parse::<bool>()?;
		let ctx = if use_live {
			test_context_from_url("ws://localhost:9944").await
		} else {
			test_context().await
		};
		let bob_id = dev::bob().public_key().to_account_id();

		let notary_id = 1;
		let ws_url = ctx.ws_url.clone();
		let keystore = MemoryKeystore::new();
		let keystore = KeystoreExt::new(keystore);
		let notary_key =
			keystore.ed25519_generate_new(NOTARY_KEYID, None).expect("should have a key");

		let mut client = MainchainClient::new(vec![ws_url.clone()]);
		let ticker = client.lookup_ticker().await?;
		let server = NotaryServer::create_http_server("127.0.0.1:0").await?;
		let addr = server.local_addr()?;
		track_blocks(ctx.ws_url, 1, &pool.clone(), ticker.clone());
		let mut notary_server = NotaryServer::start_with(server, notary_id, pool.clone()).await?;
		spawn_notebook_closer(
			pool.clone(),
			notary_id,
			client.clone(),
			keystore.clone(),
			ticker.clone(),
			notary_server.completed_notebook_sender.clone(),
		);

		propose_bob_as_notary(&ctx.client, notary_key, addr).await?;

		activate_notary(&pool, &ctx.client, &bob_id).await?;
		let bob_nonce = get_bob_nonce(&ctx.client, bob_id).await?;

		let bob_balance = 8000;
		// Submit a transfer to localchain and wait for result
		create_localchain_transfer(&pool, &ctx.client, bob_nonce, bob_balance).await?;

		// Record the balance change
		let origin = submit_balance_change_to_notary(&pool, bob_nonce, bob_balance).await?;

		let (hold_note, hold_result) =
			create_channel_hold(&pool, bob_balance as u128, 5000, origin.clone()).await?;

		let mut header_sub = notary_server.completed_notebook_stream.subscribe(100);
		let mut notebook_proof: Option<MerkleProof> = None;
		let mut best_sub = ctx.client.blocks().subscribe_best().await?;
		let mut best_hash: HashOutput = H256::default();
		let mut best_block_number = 0;
		loop {
			tokio::select! {biased;
				block_next = best_sub.next() => {
					match block_next {
						Some(Ok(block)) => {
							best_hash = block.hash();
							best_block_number = block.header().number;
						},
						_ => break
					}
				},
				next_header = header_sub.next() => {
					match next_header {
						Some(SignedNotebookHeader{ header, ..}) => {
							println!("notebook header {}", header.notebook_number);
							if header.notebook_number == hold_result.notebook_number {
								notebook_proof = Some(
									NotebookStore::get_balance_proof(
										&pool,
										notary_id,
										hold_result.notebook_number,
										&BalanceTip {
											account_origin: origin.clone(),
											balance: bob_balance as u128,
											account_id: Bob.to_account_id(),
											channel_hold_note: Some(hold_note.clone()),
											change_number: 2,
											account_type: Deposit,
										},
									)
									.await?,
								);
							}
							if header.notebook_number >= hold_result.notebook_number + CHANNEL_EXPIRATION_NOTEBOOKS
							{
								println!("Expiration of channel ready");
								break;
							}
						},
						None => break
					}
				},
			};
		}
		assert!(notebook_proof.is_some(), "Should have a notebook proof");
		println!(
			"Got a notebook proof at block {:?}. Current tick {}",
			best_block_number,
			ticker.current()
		);

		let grandparent_tick = ticker.current() - 2;
		let best_grandparents = ctx
			.client
			.storage()
			.at(best_hash)
			.fetch(&api::ticks::storage::StorageApi.recent_blocks_at_ticks(grandparent_tick))
			.await?
			.expect("Should have a block hash")
			.0;

		let best_grandparent = best_grandparents.last().expect("Should have blocks in every tick");
		println!("Voting for grandparent {:?} at tick {}", best_grandparent, grandparent_tick);

		let vote_power = (hold_note.milligons as f64 * 0.2f64) as u128;

		let channel_result = settle_channel_and_vote(
			&pool,
			hold_note,
			best_grandparent.clone(),
			BalanceProof {
				balance: bob_balance as u128,
				notebook_number: hold_result.notebook_number,
				notebook_proof,
				notary_id,
				account_origin: origin.clone(),
			},
		)
		.await?;
		println!("Channel result is {:?}", channel_result);

		let mut best_sub = ctx.client.blocks().subscribe_finalized().await?;
		while let Some(block) = best_sub.next().await {
			match block {
				Ok(block) => {
					let (tick, votes, seal, key, notebooks) = get_digests(block);
					if let Some(notebook) = notebooks.notebooks.get(0) {
						assert_eq!(notebook.audit_first_failure, None);
						if notebook.notebook_number == channel_result.notebook_number {
							assert_eq!(votes.votes_count, 1, "Should have votes");
							assert_eq!(votes.voting_power, vote_power);
							assert_eq!(tick, channel_result.tick)
						}
					}
					println!("Got block with tick {tick} {:?} {:?}", votes, seal);

					if tick >= channel_result.tick + 2 {
						assert!(
							key.parent_voting_key.is_some(),
							"Should be including parent voting keys"
						);
						assert!(
							matches!(seal, BlockSealDigest::Vote { .. }),
							"Should be vote seal"
						);
						break
					}
				},
				_ => break,
			}
		}

		notary_server.stop().await;
		Ok(())
	}

	fn get_digests(
		block: Block<UlxConfig, OnlineClient<UlxConfig>>,
	) -> (Tick, BlockVoteDigest, BlockSealDigest, ParentVotingKeyDigest, NotebookDigest<VerifyError>)
	{
		let mut tick = None;
		let mut votes = None;
		let mut block_seal = None;
		let mut notebook_digest = None;
		let mut parent_voting_key = None;
		for log in block.header().digest.logs.iter() {
			match log {
				DigestItem::PreRuntime(ulx_primitives::TICK_DIGEST_ID, data) =>
					tick = TickDigest::decode(&mut &data[..]).ok(),
				DigestItem::PreRuntime(ulx_primitives::BLOCK_VOTES_DIGEST_ID, data) =>
					votes = BlockVoteDigest::decode(&mut &data[..]).ok(),
				DigestItem::PreRuntime(ulx_primitives::NOTEBOOKS_DIGEST_ID, data) =>
					notebook_digest = NotebookDigest::decode(&mut &data[..]).ok(),
				DigestItem::Seal(ulx_primitives::BLOCK_SEAL_DIGEST_ID, data) =>
					block_seal = BlockSealDigest::decode(&mut &data[..]).ok(),
				DigestItem::Consensus(ulx_primitives::PARENT_VOTING_KEY_DIGEST, data) =>
					parent_voting_key = ParentVotingKeyDigest::decode(&mut &data[..]).ok(),
				_ => (),
			}
		}
		let tick = tick.expect("Should have a tick").tick;
		let votes = votes.expect("Should have votes");
		let block_seal = block_seal.expect("Should have block seal");
		let notebook_digest = notebook_digest.expect("Should have notebook digest");
		let parent_voting_key = parent_voting_key.expect("Should have parent voting key");

		(tick, votes, block_seal, parent_voting_key, notebook_digest)
	}

	async fn propose_bob_as_notary(
		client: &UlxClient,
		notary_key: Public,
		addr: SocketAddr,
	) -> anyhow::Result<()> {
		let ip = match addr.ip() {
			IpAddr::V4(ip) => ip,
			IpAddr::V6(_) => panic!("Should be ipv4"),
		};
		let notary_proposal = tx().notaries().propose(NotaryMeta {
			hosts: BoundedVec(vec![Host {
				is_secure: false,
				ip: ip.into(),
				port: addr.port().into(),
			}]),
			public: ed25519::Public(notary_key.0),
		});
		println!("notary proposal {:?}", notary_proposal.call_data());
		let tx_progress = client
			.tx()
			.sign_and_submit_then_watch_default(&notary_proposal, &dev::bob())
			.await?;
		let result = wait_for_in_block(tx_progress).await;

		assert_ok!(&result);

		println!("notary in block {:?}", result?.block_hash());
		Ok(())
	}

	async fn submit_balance_change_to_notary(
		pool: &PgPool,
		bob_nonce: u32,
		amount: u32,
	) -> anyhow::Result<AccountOrigin> {
		let result = NotarizationsStore::apply(
			&pool,
			1,
			vec![BalanceChange {
				account_id: Bob.to_account_id(),
				account_type: Deposit,
				change_number: 1,
				balance: amount as u128,
				previous_balance_proof: None,
				notes: bounded_vec![Note::create(
					amount as u128,
					ulx_primitives::NoteType::ClaimFromMainchain { account_nonce: bob_nonce },
				)],
				channel_hold_note: None,
				signature: sp_core::ed25519::Signature([0u8; 64]).into(),
			}
			.sign(Bob.pair())
			.clone()],
			vec![],
		)
		.await?;

		Ok(AccountOrigin {
			notebook_number: result.notebook_number,
			account_uid: result.new_account_origins[0].account_uid,
		})
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

	async fn create_channel_hold(
		pool: &PgPool,
		balance: u128,
		amount: u128,
		account_origin: AccountOrigin,
	) -> anyhow::Result<(Note, BalanceChangeResult)> {
		let hold_note = Note::create(
			amount,
			ulx_primitives::NoteType::ChannelHold { recipient: Alice.to_account_id() },
		);
		let changes = vec![BalanceChange {
			account_id: Bob.to_account_id(),
			account_type: Deposit,
			change_number: 2,
			balance,
			previous_balance_proof: Some(BalanceProof {
				balance,
				notebook_number: account_origin.notebook_number,
				notebook_proof: None, // notebook still open
				notary_id: 1,
				account_origin: account_origin.clone(),
			}),
			notes: bounded_vec![hold_note.clone()],
			channel_hold_note: None,
			signature: sp_core::sr25519::Signature([0u8; 64]).into(),
		}
		.sign(Bob.pair())
		.clone()];

		let result = NotarizationsStore::apply(&pool, 1, changes, vec![]).await?;
		Ok((hold_note.clone(), result))
	}

	async fn settle_channel_and_vote(
		pool: &PgPool,
		hold_note: Note,
		vote_block_hash: HashOutput,
		bob_balance_proof: BalanceProof,
	) -> anyhow::Result<BalanceChangeResult> {
		let channel_pass = ChannelPass {
			at_block_height: 1,
			id: 1,
			zone_record_hash: H256::zero(),
			miner_index: 0,
		};

		let tax = (hold_note.milligons as f64 * 0.2f64) as u128;
		let changes = vec![
			BalanceChange {
				account_id: Bob.to_account_id(),
				account_type: Deposit,
				change_number: 3,
				balance: bob_balance_proof.balance - hold_note.milligons,
				previous_balance_proof: Some(bob_balance_proof),
				channel_hold_note: Some(hold_note.clone()),
				notes: bounded_vec![Note::create(
					hold_note.milligons,
					ChannelSettle { channel_pass_hash: channel_pass.hash() }
				)],
				signature: sp_core::sr25519::Signature([0u8; 64]).into(),
			}
			.sign(Bob.pair())
			.clone(),
			BalanceChange {
				account_id: Alice.to_account_id(),
				account_type: Deposit,
				change_number: 1,
				balance: hold_note.milligons - tax,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![
					Note::create(hold_note.milligons, ChannelClaim),
					Note::create(tax, NoteType::Tax)
				],
				signature: sp_core::sr25519::Signature([0u8; 64]).into(),
			}
			.sign(Alice.pair())
			.clone(),
			BalanceChange {
				account_id: Alice.to_account_id(),
				account_type: Tax,
				change_number: 1,
				balance: 0,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![
					Note::create(tax, NoteType::Claim),
					Note::create(tax, NoteType::SendToVote)
				],
				signature: sp_core::sr25519::Signature([0u8; 64]).into(),
			}
			.sign(Alice.pair())
			.clone(),
		];
		let result = NotarizationsStore::apply(
			&pool,
			1,
			changes,
			vec![BlockVote {
				channel_pass: channel_pass.clone(),
				account_id: Alice.to_account_id(),
				index: 1,
				block_hash: vote_block_hash,
				power: tax,
			}],
		)
		.await?;
		Ok(result)
	}

	async fn create_localchain_transfer(
		pool: &PgPool,
		client: &UlxClient,
		bob_nonce: u32,
		amount: u32,
	) -> anyhow::Result<()> {
		client
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().chain_transfer().send_to_localchain(amount.into(), 1, bob_nonce),
				&dev::bob(),
			)
			.await?
			.wait_for_finalized_success()
			.await?;

		let mut found = false;
		for _ in 0..5 {
			let row = sqlx::query!("select * from chain_transfers").fetch_optional(pool).await?;
			if let Some(record) = row {
				assert_eq!(
					record.amount,
					amount.to_string(),
					"Should have recorded a chain transfer"
				);
				assert_eq!(
					record.account_nonce,
					Some(bob_nonce as i32),
					"Should have recorded a chain transfer"
				);
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

		println!("notary activated");
		assert_ok!(&notary_activated_finalized_block);
		let notary_activated_finalized_block = notary_activated_finalized_block.unwrap();
		let block_hash = notary_activated_finalized_block.block_hash().0;

		let mut found = false;
		for _ in 0..5 {
			let meta = sqlx::query!("select * from blocks where block_hash=$1", &block_hash)
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

	async fn wait_for_in_block(
		mut tx_progress: TxProgress<UlxConfig, OnlineClient<UlxConfig>>,
	) -> anyhow::Result<TxInBlock<UlxConfig, OnlineClient<UlxConfig>>, Error> {
		while let Some(status) = tx_progress.next().await {
			match status? {
				TxStatus::InBestBlock(tx_in_block) | TxStatus::InFinalizedBlock(tx_in_block) => {
					// now, we can attempt to work with the block, eg:
					tx_in_block.wait_for_success().await?;
					return Ok(tx_in_block)
				},
				TxStatus::Error { message } |
				TxStatus::Invalid { message } |
				TxStatus::Dropped { message } => {
					// Handle any errors:
					return Err(Error::InternalError(format!(
						"Error submitting notebook to block: {message}"
					)));
				},
				// Continue otherwise:
				_ => continue,
			}
		}
		Err(Error::InternalError(format!("No valid status encountered for notebook")))
	}
}
