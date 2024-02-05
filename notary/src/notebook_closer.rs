use std::future::Future;

use futures::FutureExt;
use sc_utils::notification::NotificationSender;
use sp_core::{ed25519, H256};
use sp_keystore::KeystorePtr;
use sqlx::{postgres::PgListener, PgPool};
use tokio::task::JoinHandle;

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
	pub ticker: Ticker,
}

pub struct FinalizedNotebookHeaderListener {
	pool: PgPool,
	completed_notebook_sender: NotificationSender<SignedNotebookHeader>,
	listener: PgListener,
}
impl FinalizedNotebookHeaderListener {
	pub async fn connect(
		pool: PgPool,
		completed_notebook_sender: NotificationSender<SignedNotebookHeader>,
	) -> anyhow::Result<Self> {
		let mut listener = PgListener::connect_with(&pool).await?;
		listener.listen("notebook_finalized").await?;
		Ok(Self { pool, completed_notebook_sender, listener })
	}

	pub async fn next(&mut self) -> anyhow::Result<SignedNotebookHeader> {
		let notification = &self.listener.recv().await?;
		let header = match notification.payload().parse() {
			Ok(notebook_number) => {
				let header =
					NotebookHeaderStore::load_with_signature(&self.pool, notebook_number).await?;

				header
			},
			Err(e) => return Err(anyhow::anyhow!("Error parsing notified notebook number {:?}", e)),
		};

		self.completed_notebook_sender.notify(|| Ok(header.clone())).map_err(
			|e: anyhow::Error| {
				anyhow::anyhow!("Error sending completed notebook notification {:?}", e)
			},
		)?;
		Ok(header)
	}

	pub fn create_task(&'_ mut self) -> impl Future<Output = anyhow::Result<()>> + Send + '_ {
		async move {
			loop {
				match self.next().await {
					Ok(_) => (),
					Err(e) => {
						tracing::error!("Error listening for finalized notebook header {:?}", e);
						if e.to_string().contains("closed pool") {
							return Err(e.into())
						}
					},
				}
			}
		}
	}
}

pub fn spawn_notebook_closer(
	pool: PgPool,
	notary_id: NotaryId,
	keystore: KeystorePtr,
	ticker: Ticker,
	completed_notebook_sender: NotificationSender<SignedNotebookHeader>,
) -> anyhow::Result<(JoinHandle<anyhow::Result<()>>, JoinHandle<anyhow::Result<()>>)> {
	let pool1 = pool.clone();
	let handle_1 = tokio::spawn(async move {
		let mut notebook_closer = NotebookCloser { pool: pool1, notary_id, keystore, ticker };
		notebook_closer.create_task().await?;
		Ok(())
	});

	let handle_2 = tokio::spawn(async move {
		let mut listener =
			FinalizedNotebookHeaderListener::connect(pool, completed_notebook_sender).await?;
		listener.create_task().await?;
		Ok(())
	});
	Ok((handle_1, handle_2))
}

const LOOP_MILLIS: u64 = prod_or_fast!(1000, 200);

impl NotebookCloser {
	pub fn create_task(
		&'_ mut self,
	) -> impl Future<Output = anyhow::Result<(), Error>> + Send + '_ {
		async move {
			loop {
				let _ = self.iterate_notebook_close_loop().await;
				let tick = self.ticker.current();
				let next_tick = self.ticker.time_for_tick(tick + 1);
				let sleep = next_tick.min(LOOP_MILLIS) - 1;
				// wait before resuming
				tokio::time::sleep(tokio::time::Duration::from_millis(sleep)).await;
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
	use std::{
		env,
		net::{IpAddr, SocketAddr},
		pin::Pin,
		sync::Arc,
		task::{Context, Poll},
	};

	use anyhow::anyhow;
	use chrono::Utc;
	use codec::Decode;
	use frame_support::assert_ok;
	use futures::{task::noop_waker_ref, StreamExt};
	use sp_core::{bounded_vec, ed25519::Public, Pair};
	use sp_keyring::Sr25519Keyring::{Alice, Bob, Ferdie};
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
	use sqlx::PgPool;
	use subxt::{
		blocks::Block,
		config::substrate::DigestItem,
		ext::sp_core::hexdisplay::AsBytesRef,
		tx::{TxInBlock, TxProgress, TxStatus},
		utils::AccountId32,
		OnlineClient,
	};
	use subxt_signer::sr25519::{dev, Keypair};
	use tokio::{spawn, sync::Mutex};

	use ulixee_client::{
		api,
		api::{
			runtime_types,
			runtime_types::{
				bounded_collections::bounded_vec::BoundedVec,
				pallet_notaries::pallet::Call as NotaryCall,
				sp_core::ed25519,
				ulx_node_runtime::RuntimeCall,
				ulx_primitives::{
					balance_change::AccountOrigin as SubxtAccountOrigin, host::Host,
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
		AccountId, AccountOrigin,
		AccountType::{Deposit, Tax},
		BalanceChange, BalanceProof, BalanceTip, BlockSealDigest, BlockVote, BlockVoteDigest,
		DataDomain, DataDomainHash, DataTLD, HashOutput, MerkleProof, Note, NoteType,
		NoteType::{ChannelClaim, ChannelSettle},
		NotebookDigest, ParentVotingKeyDigest, TickDigest, CHANNEL_EXPIRATION_TICKS,
		DATA_DOMAIN_LEASE_COST,
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
	use ulixee_client::MultiurlClient;

	type Nonce = u32;

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

		let mut client = MultiurlClient::new(vec![ws_url.clone()]);
		let ticker = client.lookup_ticker().await?;
		let ticker = Ticker::new(ticker.tick_duration_millis, ticker.genesis_utc_time);
		let server = NotaryServer::create_http_server("127.0.0.1:0").await?;
		let block_tracker = track_blocks(ctx.ws_url, 1, &pool.clone(), ticker.clone());
		let block_tracker = Arc::new(Mutex::new(Some(block_tracker)));

		propose_bob_as_notary(&ctx.client, notary_key, server.local_addr()?).await?;

		let notary_server = NotaryServer::start_with(server, 1, pool.clone()).await?;

		activate_notary(&pool, &ctx.client, &bob_id).await?;

		{
			let mut tx = pool.begin().await?;
			assert_eq!(
				NotebookStatusStore::lock_open_for_appending(&mut *tx).await?.0,
				1,
				"There should be a notebook active now"
			);
		}

		// Submit a transfer to localchain and wait for result
		let bob_transfer = create_localchain_transfer(&ctx.client, dev::bob(), 1000).await?;
		wait_for_transfers(&pool, vec![bob_transfer.clone()]).await?;

		// Record the balance change
		submit_balance_change_to_notary(&pool, bob_transfer).await?;

		sqlx::query("update notebook_status set end_time = $1 where notebook_number = 1")
			.bind(Utc::now())
			.execute(&pool)
			.await?;

		let mut closer = NotebookCloser {
			pool: pool.clone(),
			keystore: keystore.clone(),
			notary_id: 1,
			ticker: ticker.clone(),
		};
		let mut listener = FinalizedNotebookHeaderListener::connect(
			pool.clone(),
			notary_server.completed_notebook_sender.clone(),
		)
		.await?;
		let mut subscription = notary_server.completed_notebook_stream.subscribe(100);

		let listen_handle = spawn(async move {
			loop {
				match listener.next().await {
					Ok(n) => println!("Notebook finalized {}", n.header.notebook_number),
					Err(_) => break,
				}
			}
		});

		loop {
			let _ = &closer.iterate_notebook_close_loop().await;
			let status = NotebookStatusStore::get(&pool, 1).await?;
			if status.finalized_time.is_some() {
				break
			}
			// yield
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
			check_block_watch_status(block_tracker.clone()).await?;
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
		client.close().await;
		let mut block_tracker_lock = block_tracker.lock().await;
		if let Some(tracker) = block_tracker_lock.take() {
			tracker.abort();
		}
		listen_handle.abort();
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

		let mut client = MultiurlClient::new(vec![ws_url.clone()]);
		let ticker = client.lookup_ticker().await?;
		let ticker = Ticker::new(ticker.tick_duration_millis, ticker.genesis_utc_time);

		let server = NotaryServer::create_http_server("127.0.0.1:0").await?;
		let addr = server.local_addr()?;
		let block_tracker = track_blocks(ctx.ws_url, 1, &pool.clone(), ticker.clone());
		let block_tracker = Arc::new(Mutex::new(Some(block_tracker)));

		let mut notary_server = NotaryServer::start_with(server, notary_id, pool.clone()).await?;
		let watches = spawn_notebook_closer(
			pool.clone(),
			notary_id,
			keystore.clone(),
			ticker.clone(),
			notary_server.completed_notebook_sender.clone(),
		)?;

		propose_bob_as_notary(&ctx.client, notary_key, addr).await?;

		activate_notary(&pool, &ctx.client, &bob_id).await?;

		let bob_balance = 8000;
		// Submit a transfer to localchain and wait for result
		let bob_transfer = create_localchain_transfer(&ctx.client, dev::bob(), bob_balance).await?;
		let ferdie_transfer = create_localchain_transfer(&ctx.client, dev::ferdie(), 1000).await?;
		println!("bob and ferdie transfers created");
		wait_for_transfers(&pool, vec![bob_transfer.clone(), ferdie_transfer.clone()]).await?;
		println!("bob and ferdie transfers confirmed");

		let domain_hash = DataDomain::new("HelloWorld", DataTLD::Entertainment).hash();
		let result = submit_balance_change_to_notary_and_create_domain(
			&pool,
			ferdie_transfer,
			domain_hash.clone(),
			Alice.to_account_id(),
		)
		.await?;

		// wait for domain finalized
		loop {
			let status = NotebookStatusStore::get(&pool, result.notebook_number).await?;
			if status.finalized_time.is_some() {
				break
			}
			// yield
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
			check_block_watch_status(block_tracker.clone()).await?;
		}
		let zone_block = set_zone_record(&ctx.client, domain_hash.clone(), dev::alice()).await?;
		println!("set zone record");
		assert_eq!(
			ctx.client
				.storage()
				.at(zone_block)
				.fetch(&storage().data_domain().zone_records_by_domain(domain_hash))
				.await?
				.unwrap()
				.payment_account,
			dev::alice().public_key().into(),
			"Should have stored alice as payment key"
		);

		// Record the balance change
		let result = submit_balance_change_to_notary(&pool, bob_transfer).await?;
		let origin = AccountOrigin {
			account_uid: result.new_account_origins[0].account_uid,
			notebook_number: result.notebook_number,
		};

		let (hold_note, hold_result) = create_channel_hold(
			&pool,
			bob_balance as u128,
			5000,
			result.tick,
			origin.clone(),
			domain_hash.clone(),
			Alice.to_account_id(),
		)
		.await?;

		#[cfg(not(feature = "fast-runtime"))]
		{
			panic!("This test will not complete in time because the fast-runtime feature is not enabled in the build (run cargo build --release --features=fast-runtime)");
		}
		println!("created channel hold. Waiting for notebook {}", hold_result.notebook_number);

		let mut header_sub = notary_server.completed_notebook_stream.subscribe(100);
		let mut notebook_proof: Option<MerkleProof> = None;
		loop {
			tokio::select! {biased;
				next_header = header_sub.next() => {
					match next_header {
						Some(SignedNotebookHeader{ header, ..}) => {
							println!("Header complete {}", header.notebook_number);
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
							if header.notebook_number >= hold_result.notebook_number + CHANNEL_EXPIRATION_TICKS
							{
								println!("Expiration of channel ready");
								break;
							}
						},
						None => break
					}
				},
			}
			check_block_watch_status(block_tracker.clone()).await?;
		}
		assert!(notebook_proof.is_some(), "Should have a notebook proof");

		let best_hash = ctx
			.rpc_methods
			.chain_get_block_hash(None)
			.await?
			.expect("should find a best block");

		println!(
			"Got a notebook proof at block {:?}. Current tick {}",
			best_hash,
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
				tick: hold_result.tick,
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
		watches.0.abort();
		watches.1.abort();
		client.close().await;
		notary_server.stop().await;
		Ok(())
	}

	async fn check_block_watch_status(
		block_tracker: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
	) -> anyhow::Result<()> {
		let mut block_tracker_lock = block_tracker.lock().await;
		if let Some(mut block_tracker_inner) = block_tracker_lock.take() {
			let waker = noop_waker_ref();
			let mut cx = Context::from_waker(&waker);
			match Pin::new(&mut block_tracker_inner).poll(&mut cx) {
				Poll::Ready(Err(e)) => {
					tracing::error!("Error tracking blocks {:?}", e);
					return Err(anyhow!(e.to_string()))
				},
				_ => {
					*block_tracker_lock = Some(block_tracker_inner);
				},
			}
		}
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
		transfer: (Nonce, u32, Keypair),
	) -> anyhow::Result<BalanceChangeResult> {
		let (account_nonce, amount, keypair) = transfer;
		let public = keypair.public_key();
		let public = public.as_ref();

		let keypair = if public == Bob.public().as_bytes_ref() {
			Bob.pair()
		} else if public == Alice.public().as_bytes_ref() {
			Alice.pair()
		} else {
			Ferdie.pair()
		};
		let result = NotarizationsStore::apply(
			&pool,
			1,
			vec![BalanceChange {
				account_id: keypair.public().into(),
				account_type: Deposit,
				change_number: 1,
				balance: amount as u128,
				previous_balance_proof: None,
				notes: bounded_vec![Note::create(
					amount as u128,
					NoteType::ClaimFromMainchain { account_nonce },
				)],
				channel_hold_note: None,
				signature: sp_core::ed25519::Signature([0u8; 64]).into(),
			}
			.sign(keypair)
			.clone()],
			vec![],
			vec![],
		)
		.await?;

		println!("submitted chain transfer to notary");

		Ok(result)
	}
	async fn submit_balance_change_to_notary_and_create_domain(
		pool: &PgPool,
		transfer: (Nonce, u32, Keypair),
		domain_hash: DataDomainHash,
		register_domain_to: AccountId,
	) -> anyhow::Result<AccountOrigin> {
		let (account_nonce, amount, keypair) = transfer;
		let public = keypair.public_key();
		let public = public.as_ref();

		let keypair = if public == Bob.public().as_bytes_ref() {
			Bob.pair()
		} else if public == Alice.public().as_bytes_ref() {
			Alice.pair()
		} else {
			Ferdie.pair()
		};
		let result = NotarizationsStore::apply(
			&pool,
			1,
			vec![
				BalanceChange {
					account_id: keypair.public().into(),
					account_type: Deposit,
					change_number: 1,
					balance: amount as u128 - DATA_DOMAIN_LEASE_COST,
					previous_balance_proof: None,
					notes: bounded_vec![
						Note::create(
							amount as u128,
							NoteType::ClaimFromMainchain { account_nonce },
						),
						Note::create(DATA_DOMAIN_LEASE_COST, NoteType::LeaseDomain,)
					],
					channel_hold_note: None,
					signature: sp_core::ed25519::Signature([0u8; 64]).into(),
				}
				.sign(keypair.clone())
				.clone(),
				BalanceChange {
					account_id: keypair.public().into(),
					account_type: Tax,
					change_number: 1,
					balance: DATA_DOMAIN_LEASE_COST,
					previous_balance_proof: None,
					notes: bounded_vec![Note::create(DATA_DOMAIN_LEASE_COST, NoteType::Claim,)],
					channel_hold_note: None,
					signature: sp_core::ed25519::Signature([0u8; 64]).into(),
				}
				.sign(keypair.clone())
				.clone(),
			],
			vec![],
			vec![(domain_hash, register_domain_to)],
		)
		.await?;
		println!("submitted chain transfer + data domain to notary");

		Ok(AccountOrigin {
			notebook_number: result.notebook_number,
			account_uid: result.new_account_origins[0].account_uid,
		})
	}

	async fn set_zone_record(
		client: &UlxClient,
		data_domain_hash: DataDomainHash,
		account: Keypair,
	) -> anyhow::Result<H256> {
		let tx_progress = client
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().data_domain().set_zone_record(
					data_domain_hash,
					runtime_types::ulx_primitives::data_domain::ZoneRecord {
						payment_account: AccountId32::from(account.public_key()),
						notary_id: 1,
						versions: subxt::utils::KeyedVec::new(),
					},
				),
				&account,
			)
			.await?;
		let result = wait_for_in_block(tx_progress).await;
		assert_ok!(&result);
		Ok(result.unwrap().block_hash())
	}

	async fn create_channel_hold(
		pool: &PgPool,
		balance: u128,
		amount: u128,
		tick: Tick,
		account_origin: AccountOrigin,
		domain_hash: DataDomainHash,
		domain_account: AccountId,
	) -> anyhow::Result<(Note, BalanceChangeResult)> {
		let hold_note = Note::create(
			amount,
			NoteType::ChannelHold {
				recipient: domain_account,
				data_domain_hash: Some(domain_hash),
			},
		);
		let changes = vec![BalanceChange {
			account_id: Bob.to_account_id(),
			account_type: Deposit,
			change_number: 2,
			balance,
			previous_balance_proof: Some(BalanceProof {
				balance,
				notebook_number: account_origin.notebook_number,
				tick,
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

		let result = NotarizationsStore::apply(&pool, 1, changes, vec![], vec![]).await?;
		Ok((hold_note.clone(), result))
	}

	async fn settle_channel_and_vote(
		pool: &PgPool,
		hold_note: Note,
		vote_block_hash: HashOutput,
		bob_balance_proof: BalanceProof,
	) -> anyhow::Result<BalanceChangeResult> {
		let (data_domain_hash, recipient) = match hold_note.note_type.clone() {
			NoteType::ChannelHold { recipient, data_domain_hash } => (data_domain_hash, recipient),
			_ => panic!("Should be a channel hold note"),
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
				notes: bounded_vec![Note::create(hold_note.milligons, ChannelSettle)],
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
				data_domain_hash: data_domain_hash.unwrap().clone(),
				data_domain_account: recipient,
				account_id: Alice.to_account_id(),
				index: 1,
				block_hash: vote_block_hash,
				power: tax,
				signature: sp_core::sr25519::Signature([0u8; 64]).into(),
			}
			.sign(Alice.pair())
			.clone()],
			vec![],
		)
		.await?;
		Ok(result)
	}

	async fn create_localchain_transfer(
		client: &UlxClient,
		account: Keypair,
		amount: u32,
	) -> anyhow::Result<(Nonce, u32, Keypair)> {
		let in_block = client
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().chain_transfer().send_to_localchain(amount.into(), 1),
				&account,
			)
			.await?
			.wait_for_finalized()
			.await?;
		in_block.wait_for_success().await?;
		let events = in_block.fetch_events().await?;

		for event in events.iter() {
			if let Ok(event) = event {
				if let Some(Ok(transfer)) = event
					.as_event::<api::chain_transfer::events::TransferToLocalchain>()
					.transpose()
				{
					if transfer.account_id == account.public_key().to_account_id() {
						return Ok((transfer.account_nonce, transfer.amount as u32, account.clone()))
					}
				}
			}
		}
		return Err(anyhow!("Should have found the chain transfer in events"))
	}

	async fn wait_for_transfers(
		pool: &PgPool,
		transfers: Vec<(Nonce, u32, Keypair)>,
	) -> anyhow::Result<()> {
		let mut found = false;
		for _ in 0..5 {
			let rows = sqlx::query!("select * from chain_transfers").fetch_all(pool).await?;
			let is_complete = transfers.iter().filter_map(|(nonce, amount, account)| {
				if let Some(record) =
					rows.iter().find(|r| r.account_id.as_slice() == account.public_key().as_ref())
				{
					assert_eq!(
						record.amount,
						amount.to_string(),
						"Should have recorded a chain transfer"
					);
					assert_eq!(
						record.account_nonce,
						Some(*nonce as i32),
						"Should have recorded a chain transfer"
					);
					return Some(())
				}
				None
			});
			if is_complete.count() == transfers.len() {
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
