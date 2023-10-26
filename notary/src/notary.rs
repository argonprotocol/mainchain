use sc_utils::notification::{NotificationStream, TracingKeyStr};
use sp_core::{crypto::KeyTypeId, H256};
use sp_keystore::KeystorePtr;
use sqlx::PgPool;
use std::collections::btree_map::BTreeMap;
use tokio::task::JoinHandle;
use ulx_notary_audit::{verify_balance_changeset_allocation, verify_changeset_signatures};
use ulx_notary_primitives::{
	ensure,
	note::{Chain, ChainTransferDestination, NoteType},
	AccountId, AccountOrigin, BalanceChange, BalanceTip, NotaryId, NotebookHeader, MAX_TRANSFERS,
};

use crate::{
	apis::localchain::BalanceChangeResult,
	block_watch::{sync_blocks, track_blocks},
	error::Error,
	notebook_closer::NotebookCloser,
	stores::{
		balance_change::BalanceChangeStore, balance_tip::BalanceTipStore,
		block_meta::BlockMetaStore, chain_transfer::ChainTransferStore, notebook::NotebookStore,
		notebook_new_accounts::NotebookNewAccountsStore, notebook_status::NotebookStatusStore,
	},
};

type NotebookHeaderStream = NotificationStream<NotebookHeader, NotebookHeaderTracingKey>;
#[derive(Clone)]
pub struct NotebookHeaderTracingKey;
impl TracingKeyStr for NotebookHeaderTracingKey {
	const TRACING_KEY: &'static str = "mpsc_notebook_header_notification_stream";
}
// TODO: Should we take anything from
// 	cumulus/client/relay-chain-rpc-interface/src/reconnecting_ws_client.rs
#[derive(Clone)]
pub struct Notary {
	pub notary_id: NotaryId,
	rpc_url: String,
	notebook_closer: NotebookCloser,
	pub pool: PgPool,
	pub completed_notebook_stream: NotebookHeaderStream,
}

pub const NOTARY_KEYID: KeyTypeId = KeyTypeId(*b"unot");

impl Notary {
	pub async fn start(
		rpc_url: &str,
		notary_id: NotaryId,
		genesis_block_hash: H256,
		keystore: KeystorePtr,
		pool: PgPool,
		finalize_notebooks: bool,
		sync_blocks: bool,
	) -> anyhow::Result<Self> {
		let (completed_notebook_sender, completed_notebook_stream) =
			NotebookHeaderStream::channel();

		BlockMetaStore::start(&pool, genesis_block_hash).await?;

		let pool_ref = pool.clone();
		let notary = Self {
			notary_id,
			rpc_url: rpc_url.to_string(),
			completed_notebook_stream,
			pool,
			notebook_closer: NotebookCloser {
				pool: pool_ref,
				notary_id,
				rpc_url: rpc_url.to_string(),
				keystore,
				completed_notebook_sender,
			},
		};
		if sync_blocks {
			notary.start_block_sync().await?;
		}
		if finalize_notebooks {
			let _ = notary.start_notebook_closer().await?;
		}

		Ok(notary)
	}

	/// ## Basic Mainchain -> Localchain flow:
	/// 1. Funds transfer to localchain via mainchain relay transactions ("LocalchainRelay" in
	///    Ulixee Mainchain)
	/// 2. Localchain wallet submits a balance change including Note referencing the mainchain nonce
	///    used for the transfer "in"
	/// 3. Balance change is applied to account directly
	///
	/// ## Basic Localchain -> Mainchain flow:
	/// 1. Localchain wallet submits a balance change with a Note to the mainchain
	/// 2. Notary relays transactions in the next notebook it submits to the network (every minute).
	/// 3. Mainchain relay will apply the balance change to the account id on the mainchain
	///
	/// ## Basic Transfer Flow
	/// 1. Alice Localchain sends a note (type: Send) to Bob
	/// 2. Bob applies the note to his wallet with a BalanceChange to his Wallet
	/// 3. Bob notarizes BalanceChange to his wallet and Alice's wallet in a single transaction.
	///    Transaction must allocate all funds.
	/// 4. This transaction is included in a Notebook on a block with a merkle root containing Alice
	///    and Bob's transfer Key(account, chain), value: H256(balance, nonce, account origin).
	/// 5. Each user can retrieve proof that their balances can be proven in the recorded merkle
	///    root. They can also obtain their account origin, which must be included in all future
	///    changes. The account origin can be used to prove their balance has not changed in any
	///    blocks since that change.
	/// 6. If a notary is compromised, the user can use the proof of last balance change to migrate
	///    their balance to a new notary. NOTE: you must have proof from the completed notebook.
	pub async fn apply_balance_changes(
		&self,
		changes: Vec<BalanceChange>,
	) -> anyhow::Result<BalanceChangeResult, Error> {
		// Before we use db resources, let's confirm these are valid transactions
		verify_balance_changeset_allocation(&changes)?;
		verify_changeset_signatures(&changes)?;

		// Begin database transaction
		let mut tx = self.pool.begin().await?;

		let meta = BlockMetaStore::load(&mut tx).await?;
		let current_notebook_number =
			NotebookStatusStore::lock_latest_for_appending(&mut *tx).await?;

		let mut new_account_origins = BTreeMap::<(AccountId, Chain), AccountOrigin>::new();

		let to_add = changes.clone();
		for (change_index, change) in changes.into_iter().enumerate() {
			let BalanceChange { account_id, chain, nonce, previous_balance, balance, .. } = change;
			let key = (account_id.clone(), chain.clone());

			let account_origin = change
				.previous_balance_proof
				.as_ref()
				.map(|p| p.account_origin.clone())
				.or_else(|| new_account_origins.get(&key).map(|a| a.clone()));

			let account_origin = match account_origin {
				Some(account_origin) => account_origin,
				None => {
					if change.nonce != 1 {
						return Err(Error::MissingAccountOrigin)
					}

					let origin = NotebookNewAccountsStore::insert_origin(
						&mut *tx,
						current_notebook_number,
						&account_id,
						&chain,
					)
					.await?;

					new_account_origins.insert(key.clone(), origin.clone());
					origin
				},
			};

			BalanceTipStore::lock(
				&mut *tx,
				&account_id,
				chain.clone(),
				nonce,
				previous_balance,
				&account_origin,
				change_index,
				5000,
			)
			.await?;

			if let Some(proof) = change.previous_balance_proof {
				let tip = BalanceTip {
					account_id: account_id.clone(),
					chain: chain.clone(),
					nonce,
					balance,
					account_origin: account_origin.clone(),
				};
				ensure!(
					NotebookStore::is_valid_proof(&mut *tx, &tip, &proof).await?,
					Error::InvalidBalanceProof
				);
			}

			for (note_index, note) in change.notes.into_iter().enumerate() {
				let _ = match note.note_type {
					NoteType::ChainTransfer {
						destination: ChainTransferDestination::ToLocalchain { nonce },
						..
					} => {
						// NOTE: transfers can expire. We need to ensure this can still get into a
						// notebook
						ChainTransferStore::take_and_record_transfer_local(
							&mut *tx,
							current_notebook_number,
							&account_id,
							nonce,
							note.milligons,
							change_index,
							note_index,
							MAX_TRANSFERS,
						)
						.await
					},
					NoteType::ChainTransfer {
						destination: ChainTransferDestination::ToMainchain,
						..
					} => ChainTransferStore::record_transfer_to_mainchain(
						&mut *tx,
						current_notebook_number,
						&account_id,
						note.milligons,
						MAX_TRANSFERS,
					)
					.await
					.map(|_| ()),
					_ => Ok(()),
				}
				.map_err(|e| Error::BalanceChangeError {
					change_index,
					note_index,
					message: e.to_string(),
				})?;
			}

			BalanceTipStore::update(
				&mut *tx,
				&account_id,
				chain,
				nonce,
				balance,
				current_notebook_number,
				account_origin,
				previous_balance,
			)
			.await?;
		}

		BalanceChangeStore::append_notebook_changeset(&mut *tx, current_notebook_number, to_add)
			.await?;

		tx.commit().await?;
		Ok(BalanceChangeResult {
			notebook_number: current_notebook_number,
			finalized_block_number: meta.finalized_block_number,
			new_account_origins: new_account_origins
				.into_iter()
				.map(|((account_id, chain), origin)| (account_id, chain, origin))
				.collect(),
		})
	}

	pub async fn start_block_sync(&self) -> anyhow::Result<(), Error> {
		sync_blocks(self.rpc_url.clone(), self.notary_id, &self.pool)
			.await
			.map_err(|e| Error::BlockSyncError(e.to_string()))?;
		track_blocks(self.rpc_url.clone(), self.notary_id, &self.pool);

		Ok(())
	}

	pub fn start_notebook_closer(&self) -> JoinHandle<anyhow::Result<(), Error>> {
		tokio::spawn(self.notebook_closer.create_task())
	}
}
