use futures::FutureExt;
use sc_utils::notification::NotificationSender;
use sp_core::{ed25519, H256};
use sp_keystore::KeystorePtr;
use sqlx::{postgres::PgListener, PgPool};
use tokio::task::JoinHandle;

use argon_notary_apis::error::Error;
use argon_primitives::{tick::Ticker, NotaryId, SignedNotebookHeader};

use crate::stores::{
	notebook::NotebookStore,
	notebook_audit_failure::NotebookAuditFailureStore,
	notebook_header::NotebookHeaderStore,
	notebook_status::{NotebookFinalizationStep, NotebookStatusStore},
	registered_key::RegisteredKeyStore,
	BoxFutureResult,
};

pub const NOTARY_KEYID: sp_core::crypto::KeyTypeId = sp_core::crypto::KeyTypeId(*b"nota");

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
			Ok(notebook_number) =>
				NotebookHeaderStore::load_with_signature(&self.pool, notebook_number).await?,
			Err(e) => return Err(anyhow::anyhow!("Error parsing notified notebook number {:?}", e)),
		};

		self.completed_notebook_sender.notify(|| Ok(header.clone())).map_err(
			|e: anyhow::Error| {
				anyhow::anyhow!("Error sending completed notebook notification {:?}", e)
			},
		)?;
		Ok(header)
	}

	pub async fn create_task(&'_ mut self) -> anyhow::Result<()> {
		loop {
			match self.next().await {
				Ok(_) => (),
				Err(e) => {
					tracing::error!("Error listening for finalized notebook header {:?}", e);
					if e.to_string().contains("closed pool") {
						return Err(e);
					}
				},
			}
		}
	}
}

type NotebookCloserHandles = (JoinHandle<anyhow::Result<()>>, JoinHandle<anyhow::Result<()>>);
pub fn spawn_notebook_closer(
	pool: PgPool,
	notary_id: NotaryId,
	keystore: KeystorePtr,
	ticker: Ticker,
	completed_notebook_sender: NotificationSender<SignedNotebookHeader>,
) -> anyhow::Result<NotebookCloserHandles> {
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

impl NotebookCloser {
	pub async fn create_task(&'_ mut self) -> anyhow::Result<(), Error> {
		loop {
			if let Some(has_failed_audit) =
				NotebookAuditFailureStore::has_unresolved_audit_failure(&self.pool).await?
			{
				tracing::error!(
					"This notary has a failed audit. Need to shut down processing. Notebook={}, Reason={}",
					has_failed_audit.notebook_number,
					has_failed_audit.failure_reason
				);
				return Ok(());
			}
			let _ = self.iterate_notebook_close_loop().await;
			let tick = self.ticker.current();
			let next_tick = self.ticker.time_for_tick(tick + 1);
			let loop_millis = if self.ticker.tick_duration_millis <= 2000 { 200 } else { 1000 };
			let sleep = next_tick.min(loop_millis) + 1;
			// wait before resuming
			tokio::time::sleep(tokio::time::Duration::from_millis(sleep)).await;
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
			let notebook_number = match NotebookStatusStore::step_up_expired_open(&mut tx).await? {
				Some(notebook_number) => notebook_number,
				None => return Ok(()),
			};
			let next_notebook = notebook_number + 1u32;
			let tick = self.ticker.current();
			let end_time = self.ticker.time_for_tick(tick + 1);

			NotebookHeaderStore::create(&mut tx, self.notary_id, next_notebook, tick, end_time)
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
			let (notebook_number, tick) =
				match NotebookStatusStore::find_and_lock_ready_for_close(&mut tx).await? {
					Some(notebook_number) => notebook_number,
					None => return Ok(()),
				};

			let public = RegisteredKeyStore::get_valid_public(&mut *tx, tick).await?;

			NotebookStore::close_notebook(&mut tx, notebook_number, public, &self.keystore).await?;

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
		.ed25519_sign(NOTARY_KEYID, public, &hash[..])
		.map_err(|e| {
			Error::InternalError(format!(
				"Unable to sign notebook header for submission to mainchain {}",
				e
			))
		})?
		.unwrap_or_else(|| panic!("Could not sign the notebook header. Ensure the notary key {:?} is installed in the keystore", public));
	Ok(sig)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct NotebookAuditResponse {
	pub signature: String,
}

#[cfg(test)]
mod tests {
	use anyhow::anyhow;
	use codec::{Decode, Encode};
	use frame_support::assert_ok;
	use futures::{task::noop_waker_ref, StreamExt};
	use sp_core::{bounded_vec, crypto::AccountId32, ed25519::Public, sr25519::Signature, Pair};
	use sp_keyring::{
		sr25519::Keyring,
		AccountKeyring,
		Sr25519Keyring::{Alice, Bob, Ferdie},
	};
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
	use sqlx::PgPool;
	use std::{
		future::Future,
		net::{IpAddr, SocketAddr},
		pin::Pin,
		sync::Arc,
		task::{Context, Poll},
	};
	use subxt::{
		blocks::Block,
		tx::{TxInBlock, TxProgress},
		OnlineClient,
	};
	use tokio::sync::Mutex;

	use argon_client::{
		api,
		api::{
			runtime_types,
			runtime_types::{
				argon_primitives::notary::NotaryMeta, argon_runtime::RuntimeCall,
				pallet_notaries::pallet::Call as NotaryCall,
			},
			storage, tx,
		},
		conversion::SubxtRuntime,
		signer::Sr25519Signer,
		ArgonConfig, ArgonOnlineClient, MainchainClient, ReconnectingClient,
	};
	use argon_notary_apis::localchain::BalanceChangeResult;
	use argon_notary_audit::VerifyError;
	use argon_primitives::{
		fork_power::ForkPower,
		host::Host,
		prelude::*,
		AccountOrigin,
		AccountType::{Deposit, Tax},
		ArgonDigests, BalanceChange, BalanceProof, BalanceTip, BlockSealDigest, BlockVote,
		BlockVoteDigest, Domain, DomainHash, DomainTopLevel, HashOutput, MerkleProof,
		NoteType::{ChannelHoldClaim, ChannelHoldSettle},
		NotebookDigest, ParentVotingKeyDigest, TransferToLocalchainId, VotingSchedule,
		DOMAIN_LEASE_COST,
	};
	use argon_testing::start_argon_test_node;

	use crate::{
		block_watch::track_blocks,
		notebook_closer::NOTARY_KEYID,
		stores::{notarizations::NotarizationsStore, notebook_status::NotebookStatusStore},
		NotaryServer,
	};

	use super::*;

	#[sqlx::test]
	async fn test_submitting_votes(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let ctx = start_argon_test_node().await;

		let bob_id = Bob.to_account_id();

		let notary_id = 1;
		let ws_url = ctx.client.url.clone();
		let keystore = MemoryKeystore::new();
		let keystore = KeystoreExt::new(keystore);
		let notary_key =
			keystore.ed25519_generate_new(NOTARY_KEYID, None).expect("should have a key");

		let mut client = ReconnectingClient::new(vec![ws_url.clone()]);
		let ticker: Ticker = client.get().await?.lookup_ticker().await?;

		let server = NotaryServer::create_http_server("127.0.0.1:0").await?;
		let addr = server.local_addr()?;
		let block_tracker = track_blocks(ws_url.clone(), 1, pool.clone(), ticker);
		let block_tracker = Arc::new(Mutex::new(Some(block_tracker)));

		let mut notary_server =
			NotaryServer::start_with(server, notary_id, ticker, pool.clone()).await?;
		let watches = spawn_notebook_closer(
			pool.clone(),
			notary_id,
			keystore.clone(),
			ticker,
			notary_server.completed_notebook_sender.clone(),
		)?;

		propose_bob_as_notary(&ctx.client.live, notary_key, addr).await?;

		activate_notary(&pool, &ctx.client.live, &bob_id).await?;

		let bob_balance = 8_000_000;
		// Submit a transfer to localchain and wait for result
		let bob_transfer = create_localchain_transfer(&ctx.client.live, Bob, bob_balance).await?;
		let ferdie_transfer =
			create_localchain_transfer(&ctx.client.live, Ferdie, 1_000_000).await?;
		println!("bob and ferdie transfers created");
		wait_for_transfers(&pool, vec![bob_transfer, ferdie_transfer]).await?;
		println!("bob and ferdie transfers confirmed");

		let domain_hash = Domain::new("HelloWorld", DomainTopLevel::Entertainment).hash();
		let result = submit_balance_change_to_notary_and_create_domain(
			&pool,
			&ticker,
			ferdie_transfer,
			domain_hash,
			Alice.to_account_id(),
		)
		.await?;

		// wait for domain finalized
		loop {
			let status = NotebookStatusStore::get(&pool, result.notebook_number).await?;
			if status.finalized_time.is_some() {
				break;
			}
			// yield
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
			check_block_watch_status(block_tracker.clone()).await?;
		}
		let zone_block = set_zone_record(&ctx.client.live, domain_hash, Alice).await?;
		println!("set zone record");
		assert_eq!(
			ctx.client
				.fetch_storage(
					&storage()
						.domains()
						.zone_records_by_domain(argon_client::types::H256::from(domain_hash)),
					Some(zone_block)
				)
				.await?
				.unwrap()
				.payment_account,
			Alice.to_account_id().into(),
			"Should have stored alice as payment key"
		);

		// Record the balance change
		let result = submit_balance_change_to_notary(&pool, &ticker, bob_transfer).await?;
		let origin = AccountOrigin {
			account_uid: result.new_account_origins[0].account_uid,
			notebook_number: result.notebook_number,
		};

		let (hold_note, hold_result) = create_channel_hold(
			&pool,
			bob_balance as u128,
			5_000_000,
			&ticker,
			result.tick,
			origin.clone(),
			Alice.to_account_id(),
		)
		.await?;

		println!("created channel hold. Waiting for notebook {}", hold_result.notebook_number);

		let channel_hold_expiration_ticks = ticker.channel_hold_expiration_ticks;

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
							if header.tick >= hold_result.tick + channel_hold_expiration_ticks
							{
								println!("Expiration of channel_hold ready");
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

		let best_hash = ctx.client.best_block_hash().await.expect("should find a best block");

		println!(
			"Got a notebook proof at block {:?}. Current tick {}. Runtime Tick {}",
			best_hash,
			ticker.current(),
			ctx.client
				.call(api::runtime_apis::tick_apis::TickApis.current_tick(), Some(best_hash))
				.await?
		);

		let mut attempts = 0;
		let best_grandparent = {
			loop {
				match ctx.client.get_vote_block_hash(ticker.current()).await.ok() {
					Some((grandparent_block, _)) => {
						println!(
							"Voting for grandparent {:?}. Current tick {}",
							grandparent_block,
							ticker.current()
						);
						break grandparent_block;
					},
					// wait a second
					None => {
						println!(
							"No grandparents found. Waiting. Runtime tick={}",
							ticker.current()
						);
						if attempts > 5 {
							panic!("Should have found a grandparent");
						}
						attempts += 1;
						tokio::time::sleep(ticker.duration_to_next_tick()).await
					},
				}
			}
		};

		let vote_power = (hold_note.microgons as f64 * 0.2f64) as u128;

		let channel_hold_result = settle_channel_hold_and_vote(
			&pool,
			&ticker,
			hold_note,
			best_grandparent,
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
		println!("ChannelHold result is {:?}", channel_hold_result);

		let voting_schedule = VotingSchedule::when_creating_votes(channel_hold_result.tick);
		let mut best_sub = ctx.client.live.blocks().subscribe_finalized().await?;
		let mut did_see_voting_key = false;
		let mut last_block_fork = ForkPower::default();
		while let Some(block) = best_sub.next().await {
			match block {
				Ok(block) => {
					let block_hash = block.hash();
					let (tick, votes, seal, parent_key, notebooks, fork_power) = get_digests(block);

					println!(
						"Got block with tick {tick}. Notebooks: {:?}, {:?} {:?} {:?}",
						votes,
						notebooks.notebooks,
						if matches!(seal, BlockSealDigest::Compute { .. }) {
							"Compute"
						} else {
							"Vote"
						},
						block_hash
					);
					assert!(fork_power >= last_block_fork, "Should have increasing fork power");
					last_block_fork = fork_power;
					if let Some(notebook) = notebooks.notebooks.first() {
						assert_eq!(notebook.audit_first_failure, None);
						if notebook.notebook_number == channel_hold_result.notebook_number {
							assert_eq!(votes.votes_count, 1, "Should have votes");
							assert_eq!(votes.voting_power, vote_power);
							assert_eq!(tick, voting_schedule.block_tick())
						}
					}
					if let Some(Some(parent_voting_key)) = ctx
						.client
						.fetch_storage(
							&storage().block_seal().parent_voting_key(),
							Some(block_hash),
						)
						.await?
					{
						assert_eq!(
							parent_voting_key.0,
							parent_key.parent_voting_key.expect("Should have parent voting key").0
						);
						did_see_voting_key = true;
					}

					// should have gotten a vote in tick 2
					if tick >= voting_schedule.block_tick() + 3 {
						break;
					}
				},
				_ => break,
			}
		}
		assert!(did_see_voting_key, "Should have seen a voting key");
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
			let mut cx = Context::from_waker(waker);
			match Pin::new(&mut block_tracker_inner).poll(&mut cx) {
				Poll::Ready(Err(e)) => {
					tracing::error!("Error tracking blocks {:?}", e);
					return Err(anyhow!(e.to_string()));
				},
				_ => {
					*block_tracker_lock = Some(block_tracker_inner);
				},
			}
		}
		Ok(())
	}

	fn get_digests(
		block: Block<ArgonConfig, OnlineClient<ArgonConfig>>,
	) -> (
		Tick,
		BlockVoteDigest,
		BlockSealDigest,
		ParentVotingKeyDigest,
		NotebookDigest<VerifyError>,
		ForkPower,
	) {
		let mut tick = None;
		let mut votes = None;
		let mut block_seal = None;
		let mut notebook_digest = None;
		let mut parent_voting_key = None;
		let mut fork_power = None;
		for log in block.header().runtime_digest().logs.iter() {
			let digest = sp_runtime::DigestItem::decode(&mut &log.encode()[..])
				.expect("Should be able to decode digest item");
			if let Some(d) = digest.as_tick() {
				tick = Some(d);
			} else if let Some(d) = digest.as_block_vote() {
				votes = Some(d);
			} else if let Some(d) = digest.as_parent_voting_key() {
				parent_voting_key = Some(d);
			} else if let Some(d) = digest.as_fork_power() {
				fork_power = Some(d);
			} else if let Some(d) = digest.as_notebooks() {
				notebook_digest = Some(d);
			} else if let Some(d) = digest.as_block_seal() {
				block_seal = Some(d);
			}
		}
		let tick = tick.expect("Should have a tick").0;
		let votes = votes.expect("Should have votes");
		let block_seal = block_seal.expect("Should have block seal");
		let notebook_digest = notebook_digest.expect("Should have notebook digest");
		let fork_power = fork_power.expect("Should have fork power");
		let parent_voting_key = parent_voting_key.expect("Should have parent voting key");

		(tick, votes, block_seal, parent_voting_key, notebook_digest, fork_power)
	}

	async fn propose_bob_as_notary(
		client: &ArgonOnlineClient,
		notary_key: Public,
		addr: SocketAddr,
	) -> anyhow::Result<()> {
		let ip = match addr.ip() {
			IpAddr::V4(ip) => ip,
			IpAddr::V6(_) => panic!("Should be ipv4"),
		};
		let host: Host = format!("ws://{}:{}", ip, addr.port()).into();
		let notary_proposal = tx().notaries().propose(NotaryMeta {
			name: runtime_types::argon_primitives::notary::NotaryName(
				"test".as_bytes().to_vec().into(),
			),
			hosts: vec![runtime_types::argon_primitives::host::Host(host.0.into())].into(),
			public: notary_key.0,
		});
		println!("notary proposal {:?}", notary_proposal.call_data());
		let signer: Sr25519Signer = Bob.pair().into();
		let tx_progress = client
			.tx()
			.sign_and_submit_then_watch_default(&notary_proposal, &signer)
			.await?;
		let result = wait_for_in_block(tx_progress).await;

		assert_ok!(&result);

		println!("notary in block {:?}", result?.block_hash());
		Ok(())
	}

	async fn submit_balance_change_to_notary(
		pool: &PgPool,
		ticker: &Ticker,
		transfer: (TransferToLocalchainId, u32, Keyring),
	) -> anyhow::Result<BalanceChangeResult> {
		let (transfer_id, amount, keyring) = transfer;

		let keypair = if keyring == Bob {
			Bob.pair()
		} else if keyring == Alice {
			Alice.pair()
		} else {
			Ferdie.pair()
		};
		let result = NotarizationsStore::apply(
			pool,
			1,
			ticker,
			vec![BalanceChange {
				account_id: keypair.public().into(),
				account_type: Deposit,
				change_number: 1,
				balance: amount as u128,
				previous_balance_proof: None,
				notes: bounded_vec![Note::create(
					amount as u128,
					NoteType::ClaimFromMainchain { transfer_id },
				)],
				channel_hold_note: None,
				signature: Signature::from_raw([0u8; 64]).into(),
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
		ticker: &Ticker,
		transfer: (TransferToLocalchainId, u32, Keyring),
		domain_hash: DomainHash,
		register_domain_to: AccountId,
	) -> anyhow::Result<AccountOrigin> {
		let (transfer_id, amount, keyring) = transfer;

		let keypair = if keyring == Bob {
			Bob.pair()
		} else if keyring == Alice {
			Alice.pair()
		} else {
			Ferdie.pair()
		};
		let result = NotarizationsStore::apply(
			pool,
			1,
			ticker,
			vec![
				BalanceChange {
					account_id: keypair.public().into(),
					account_type: Deposit,
					change_number: 1,
					balance: amount as u128 - DOMAIN_LEASE_COST,
					previous_balance_proof: None,
					notes: bounded_vec![
						Note::create(amount as u128, NoteType::ClaimFromMainchain { transfer_id },),
						Note::create(DOMAIN_LEASE_COST, NoteType::LeaseDomain,)
					],
					channel_hold_note: None,
					signature: Signature::from_raw([0u8; 64]).into(),
				}
				.sign(keypair.clone())
				.clone(),
				BalanceChange {
					account_id: keypair.public().into(),
					account_type: Tax,
					change_number: 1,
					balance: DOMAIN_LEASE_COST,
					previous_balance_proof: None,
					notes: bounded_vec![Note::create(DOMAIN_LEASE_COST, NoteType::Claim,)],
					channel_hold_note: None,
					signature: Signature::from_raw([0u8; 64]).into(),
				}
				.sign(keypair.clone())
				.clone(),
			],
			vec![],
			vec![(domain_hash, register_domain_to)],
		)
		.await?;
		println!("submitted chain transfer + domain to notary");

		Ok(AccountOrigin {
			notebook_number: result.notebook_number,
			account_uid: result.new_account_origins[0].account_uid,
		})
	}

	async fn set_zone_record(
		client: &ArgonOnlineClient,
		domain_hash: DomainHash,
		account: AccountKeyring,
	) -> anyhow::Result<H256> {
		let signer = Sr25519Signer::new(account.pair());
		let tx_progress = client
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().domains().set_zone_record(
					domain_hash.into(),
					runtime_types::argon_primitives::domain::ZoneRecord {
						payment_account: account.public().into(),
						notary_id: 1,
						versions: subxt::utils::KeyedVec::new(),
					},
				),
				&signer,
			)
			.await?;
		let result = wait_for_in_block(tx_progress).await;
		assert_ok!(&result);
		Ok(result.unwrap().block_hash())
	}

	#[allow(clippy::too_many_arguments)]
	async fn create_channel_hold(
		pool: &PgPool,
		balance: u128,
		amount: u128,
		ticker: &Ticker,
		tick: Tick,
		account_origin: AccountOrigin,
		domain_account: AccountId,
	) -> anyhow::Result<(Note, BalanceChangeResult)> {
		let hold_note = Note::create(
			amount,
			NoteType::ChannelHold {
				recipient: domain_account,
				delegated_signer: None,
				domain_hash: None,
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
			signature: sp_core::sr25519::Signature::from_raw([0u8; 64]).into(),
		}
		.sign(Bob.pair())
		.clone()];

		let result = NotarizationsStore::apply(pool, 1, ticker, changes, vec![], vec![]).await?;
		Ok((hold_note.clone(), result))
	}

	async fn settle_channel_hold_and_vote(
		pool: &PgPool,
		ticker: &Ticker,
		hold_note: Note,
		vote_block_hash: HashOutput,
		bob_balance_proof: BalanceProof,
	) -> anyhow::Result<BalanceChangeResult> {
		let tax = (hold_note.microgons as f64 * 0.2f64) as u128;
		let changes = vec![
			BalanceChange {
				account_id: Bob.to_account_id(),
				account_type: Deposit,
				change_number: 3,
				balance: bob_balance_proof.balance - hold_note.microgons,
				previous_balance_proof: Some(bob_balance_proof),
				channel_hold_note: Some(hold_note.clone()),
				notes: bounded_vec![Note::create(hold_note.microgons, ChannelHoldSettle)],
				signature: sp_core::sr25519::Signature::from_raw([0u8; 64]).into(),
			}
			.sign(Bob.pair())
			.clone(),
			BalanceChange {
				account_id: Alice.to_account_id(),
				account_type: Deposit,
				change_number: 1,
				balance: hold_note.microgons - tax,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![
					Note::create(hold_note.microgons, ChannelHoldClaim),
					Note::create(tax, NoteType::Tax)
				],
				signature: sp_core::sr25519::Signature::from_raw([0u8; 64]).into(),
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
				signature: sp_core::sr25519::Signature::from_raw([0u8; 64]).into(),
			}
			.sign(Alice.pair())
			.clone(),
		];
		let result = NotarizationsStore::apply(
			pool,
			1,
			ticker,
			changes,
			vec![BlockVote {
				account_id: Alice.to_account_id(),
				index: 1,
				tick: ticker.current(),
				block_hash: vote_block_hash,
				power: tax,
				block_rewards_account_id: Alice.to_account_id(),
				signature: Signature::from_raw([0u8; 64]).into(),
			}
			.sign(Alice.pair())
			.clone()],
			vec![],
		)
		.await?;
		Ok(result)
	}

	async fn create_localchain_transfer(
		client: &ArgonOnlineClient,
		account: Keyring,
		amount: u32,
	) -> anyhow::Result<(TransferToLocalchainId, u32, Keyring)> {
		let signer: Sr25519Signer = account.pair().into();
		let in_block = client
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().chain_transfer().send_to_localchain(amount.into(), 1),
				&signer,
			)
			.await?
			.wait_for_finalized_success()
			.await?;
		let events = in_block.all_events_in_block();

		for event in events.iter().flatten() {
			if let Some(Ok(transfer)) = event
				.as_event::<api::chain_transfer::events::TransferToLocalchain>()
				.transpose()
			{
				if transfer.account_id == account.public().into() {
					return Ok((transfer.transfer_id, transfer.amount as u32, account));
				}
			}
		}
		Err(anyhow!("Should have found the chain transfer in events"))
	}

	async fn wait_for_transfers(
		pool: &PgPool,
		transfers: Vec<(TransferToLocalchainId, u32, Keyring)>,
	) -> anyhow::Result<()> {
		let mut found = false;
		for _ in 0..5 {
			let rows = sqlx::query!("select * from chain_transfers").fetch_all(pool).await?;
			let is_complete = transfers.iter().filter_map(|(transfer_id, amount, _account)| {
				if let Some(record) =
					rows.iter().find(|r| (r.transfer_id.unwrap_or_default() as u32) == *transfer_id)
				{
					assert_eq!(
						record.amount,
						amount.to_string(),
						"Should have recorded a chain transfer amount"
					);
					return Some(());
				}
				None
			});
			if is_complete.count() == transfers.len() {
				found = true;
				break;
			}
			// wait for 500 ms
			tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		}
		assert!(found, "Should have recorded a chain transfer");

		Ok(())
	}

	async fn activate_notary(
		pool: &PgPool,
		client: &ArgonOnlineClient,
		bob_id: &AccountId32,
	) -> anyhow::Result<()> {
		let api_bob = argon_client::types::AccountId32::from(bob_id.clone());
		let signer: Sr25519Signer = Alice.pair().into();
		let notary_activated_finalized_block = client
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().sudo().sudo(RuntimeCall::Notaries(NotaryCall::activate {
					operator_account: api_bob,
				})),
				&signer,
			)
			.await?
			.wait_for_finalized()
			.await?;

		let events = notary_activated_finalized_block.wait_for_success().await;

		println!("notary activated");
		assert_ok!(&events);
		let block_hash = notary_activated_finalized_block.block_hash().0;

		let mut found = false;
		for _ in 0..5 {
			let meta = sqlx::query!("select * from blocks where block_hash=$1", &block_hash)
				.fetch_optional(pool)
				.await?;
			if meta.is_some() {
				found = true;
				break;
			}
			// wait for 500 ms
			tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		}
		assert!(found, "Should have found the finalized block");
		Ok(())
	}

	async fn wait_for_in_block(
		tx_progress: TxProgress<ArgonConfig, OnlineClient<ArgonConfig>>,
	) -> anyhow::Result<TxInBlock<ArgonConfig, OnlineClient<ArgonConfig>>, Error> {
		let res = MainchainClient::wait_for_ext_in_block(tx_progress, false).await?;
		Ok(res)
	}
}
