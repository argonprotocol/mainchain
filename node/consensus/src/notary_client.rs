use std::{collections::BTreeMap, default::Default, marker::PhantomData, sync::Arc};

use codec::Codec;
use log::{info, trace, warn};
use sc_client_api::AuxStore;
use sc_utils::mpsc::TracingUnboundedSender;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::blake2_256;
use sp_runtime::traits::Block as BlockT;
use tokio::sync::Mutex;

use crate::{
	aux_client::{ArgonAux, NotebookAuditResult},
	error::Error,
};
use argon_node_runtime::{NotaryRecordT, NotebookVerifyError};
use argon_notary_apis::notebook::{NotebookRpcClient, RawHeadersSubscription};
use argon_primitives::{
	notary::{NotaryNotebookDetails, NotaryNotebookTickState},
	notebook::NotebookNumber,
	tick::Tick,
	BlockSealApis, BlockSealAuthorityId, NotaryApis, NotaryId, NotebookApis, NotebookDigest,
	NotebookHeaderData,
};

pub struct NotaryClient<B: BlockT, C: AuxStore, AC> {
	client: Arc<C>,
	pub notary_client_by_id: Arc<Mutex<BTreeMap<NotaryId, Arc<argon_notary_apis::Client>>>>,
	pub notaries_by_id: Arc<Mutex<BTreeMap<NotaryId, NotaryRecordT>>>,
	pub subscriptions_by_id: Arc<Mutex<BTreeMap<NotaryId, RawHeadersSubscription>>>,
	header_stream: TracingUnboundedSender<(NotaryId, NotebookNumber, Vec<u8>)>,
	aux_client: ArgonAux<B, C>,
	_block: PhantomData<AC>,
}

const LOG_TARGET: &str = "node::notary_client";

impl<B, C, AC> NotaryClient<B, C, AC>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + AuxStore + 'static,
	C::Api: NotaryApis<B, NotaryRecordT>
		+ NotebookApis<B, NotebookVerifyError>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>,
	AC: Clone + Codec,
{
	pub fn new(
		client: Arc<C>,
		aux_client: ArgonAux<B, C>,
		header_stream: TracingUnboundedSender<(NotaryId, NotebookNumber, Vec<u8>)>,
	) -> Self {
		Self {
			client,
			subscriptions_by_id: Default::default(),
			notary_client_by_id: Default::default(),
			notaries_by_id: Default::default(),
			header_stream,
			aux_client,
			_block: PhantomData,
		}
	}

	pub async fn update_notaries(&self, block_hash: &B::Hash) -> Result<(), Error> {
		let mut needs_connect = vec![];
		{
			let notaries = self.client.runtime_api().notaries(*block_hash)?;
			let mut clients = self.notary_client_by_id.lock().await;
			let mut notaries_by_id = self.notaries_by_id.lock().await;
			let next_notaries_by_id =
				notaries.iter().map(|n| (n.notary_id, n.clone())).collect::<BTreeMap<_, _>>();

			if next_notaries_by_id != *notaries_by_id {
				let mut subscriptions_by_id = self.subscriptions_by_id.lock().await;
				for notary in notaries {
					if let Some(existing) = notaries_by_id.get(&notary.notary_id) {
						if existing.meta_updated_block < notary.meta_updated_block ||
							!clients.contains_key(&notary.notary_id)
						{
							// need to reconnect
							needs_connect.push(notary.notary_id);
						}
					} else {
						// need to connect
						needs_connect.push(notary.notary_id);
					}
				}
				clients.retain(|id, _| {
					if notaries_by_id.contains_key(id) {
						return true;
					}

					subscriptions_by_id.remove(id);
					false
				});
				*notaries_by_id = next_notaries_by_id;
				info!(target: LOG_TARGET, "Notaries updated. {} notaries. {} need to connect.", &notaries_by_id.len(), &needs_connect.len());
			} else {
				for notary in notaries {
					if !clients.contains_key(&notary.notary_id) {
						needs_connect.push(notary.notary_id);
					}
				}
				if !needs_connect.is_empty() {
					info!(target: LOG_TARGET, "Notaries unchanged. {} need to re-connect.", &needs_connect.len());
				}
			}
		}
		if needs_connect.is_empty() {
			return Ok(());
		}

		for id in needs_connect {
			if let Err(e) = self.sync_notebooks(id).await {
				self.disconnect(&id, Some(format!("Notary {} sync failed. {:?}", id, e))).await;
			}

			if let Err(e) = self.subscribe_to_notebooks(id).await {
				self.disconnect(&id, Some(format!("Notary {} subscription failed. {:?}", id, e)))
					.await;
			}
		}

		Ok(())
	}

	pub async fn retrieve_missing_notebooks(&self) -> Result<(), Error> {
		let mut notaries: Vec<NotaryId> = vec![];
		{
			let notaries_by_id = self.notaries_by_id.lock().await;
			for (id, _) in notaries_by_id.iter() {
				notaries.push(*id);
			}
		}
		for notary_id in notaries {
			let _ = self.retrieve_notary_missing_notebooks(notary_id).await.map_err(|err| {
				warn!(target: LOG_TARGET, "Error synching missing notebooks from notary #{} - {:?}", notary_id, err)
			});
		}

		Ok(())
	}

	async fn retrieve_notary_missing_notebooks(&self, notary_id: NotaryId) -> Result<(), Error> {
		let client = self.get_client(notary_id).await?;
		let lock = self.notary_client_by_id.lock().await;
		let missing_notebooks_aux = self.aux_client.get_missing_notebooks(notary_id)?;
		let missing_notebooks = missing_notebooks_aux.get();
		if missing_notebooks.is_empty() {
			return Ok(());
		}

		info!(target: LOG_TARGET, "Retrieving missing notebooks from notary #{} - {:?}", notary_id, missing_notebooks);

		let mut headers = client
			.get_raw_headers(None, Some(missing_notebooks.iter().cloned().collect::<Vec<_>>()))
			.await
			.map_err(|e| {
				Error::NotaryError(format!("Could not get notebooks from notary - {:?}", e))
			})?;

		// ensure headers are sorted
		headers.sort_by(|a, b| a.0.cmp(&b.0));

		// remove notebooks from missing list
		let notebooks_retrieved = headers.iter().map(|(n, _)| *n).collect::<Vec<_>>();
		missing_notebooks_aux.mutate(|x| {
			for notebook_number in notebooks_retrieved {
				x.remove(&notebook_number);
			}
		})?;
		drop(lock);

		for (notebook_number, header) in headers {
			self.header_stream
				.unbounded_send((notary_id, notebook_number, header))
				.map_err(|e| {
					Error::NotaryError(format!("Could not send header to stream - {:?}", e))
				})?;
		}

		Ok(())
	}

	async fn sync_notebooks(&self, id: NotaryId) -> Result<(), Error> {
		let client = self.get_client(id).await?;
		let notebook_meta = client.metadata().await.map_err(|e| {
			Error::NotaryError(format!("Could not get notebooks from notary - {:?}", e))
		})?;
		let notary_notebooks = self.aux_client.get_notary_audit_history(id)?.get();
		let latest_stored = notary_notebooks.last().map(|n| n.notebook_number).unwrap_or_default();
		if latest_stored < notebook_meta.finalized_notebook_number.saturating_sub(1) {
			let catchup = client.get_raw_headers(Some(latest_stored), None).await.map_err(|e| {
				Error::NotaryError(format!("Could not get notebooks from notary - {:?}", e))
			})?;
			for (notebook_number, header) in catchup {
				self.header_stream.unbounded_send((id, notebook_number, header)).map_err(|e| {
					Error::NotaryError(format!("Could not send header to stream - {:?}", e))
				})?;
			}
		}

		Ok(())
	}

	pub async fn disconnect(&self, notary_id: &NotaryId, reason: Option<String>) {
		let mut clients = self.notary_client_by_id.lock().await;
		info!(target: LOG_TARGET, "Notary client disconnected from notary #{} (or could not connect). Reason? {:?}", notary_id, reason);
		if !clients.contains_key(notary_id) {
			return;
		}
		clients.remove(notary_id);
		let mut subs = self.subscriptions_by_id.lock().await;
		drop(subs.remove(notary_id));
	}

	async fn subscribe_to_notebooks(&self, id: NotaryId) -> Result<(), Error> {
		let client = self.get_client(id).await?;
		let stream = client.subscribe_raw_headers().await.map_err(|e| {
			Error::NotaryError(format!("Could not subscribe to notebooks from notary - {:?}", e))
		})?;
		let mut subs = self.subscriptions_by_id.lock().await;
		subs.insert(id, stream);
		Ok(())
	}

	pub async fn try_audit_notebook(
		&self,
		finalized_hash: &B::Hash,
		best_hash: &B::Hash,
		raw_header: Vec<u8>,
		vote_details: &NotaryNotebookDetails<B::Hash>,
	) -> Result<NotaryNotebookTickState, Error> {
		let notary_id = vote_details.notary_id;
		let notebook_number = vote_details.notebook_number;

		// load the audit history for the notary
		let latest_notebook_by_notary =
			self.client.runtime_api().latest_notebook_by_notary(*best_hash)?;
		let mut notebook_dependencies = vec![];
		let latest_notebook =
			latest_notebook_by_notary.get(&notary_id).map(|a| a.0).unwrap_or_default();

		let mut missing_notebooks = vec![];
		if latest_notebook < notebook_number - 1 {
			let notary_notebooks = self.aux_client.get_audit_summaries(notary_id)?.get();
			for notebook_number_needed in latest_notebook + 1..notebook_number {
				if let Some(summary) =
					notary_notebooks.iter().find(|s| s.notebook_number == notebook_number_needed)
				{
					notebook_dependencies.push(summary.clone());
				} else {
					missing_notebooks.push(notebook_number_needed);
				}
			}
		}

		if !missing_notebooks.is_empty() {
			let msg = format!(
				"Missing notebooks #{:?} to audit {} for notary {}",
				missing_notebooks, notebook_number, notary_id
			);
			self.aux_client.get_missing_notebooks(notary_id)?.mutate(|a| {
				for notebook_number in missing_notebooks {
					a.insert(notebook_number);
				}
				// process self afterwards
				a.insert(notebook_number);
			})?;
			return Err(Error::NotaryError(msg));
		}

		// for all other notaries, load the history we have?
		// TODO: what do we need for cross-notary transfers?

		info!(
			target: LOG_TARGET,
			"Attempting to audit notebook. Notary {}, #{}.",
			notary_id,
			notebook_number);

		let full_notebook = self.download_notebook(notary_id, notebook_number).await?;

		trace!(
			target: LOG_TARGET,
			"Notebook downloaded. Notary {}, #{}. {} bytes.",
			notary_id,
			notebook_number,
			full_notebook.len()
		);
		let mut vote_minimums = BTreeMap::new();
		for block_hash in &vote_details.blocks_with_votes {
			vote_minimums.insert(
				*block_hash,
				self.client.runtime_api().vote_minimum(*block_hash).map_err(|e| {
					let message = format!(
						"Error getting vote minimums for block {}. Notary {}, notebook {}. {:?}",
						block_hash, notary_id, notebook_number, e
					);
					Error::StringError(message)
				})?,
			);
		}

		let mut audit_result = NotebookAuditResult {
			tick: vote_details.tick,
			notebook_number,
			is_valid: true,
			body_hash: blake2_256(&full_notebook),
			first_error_reason: None,
		};

		let latest_finalized_notebook_by_notary =
			self.client.runtime_api().latest_notebook_by_notary(*finalized_hash)?;
		let oldest_tick_to_keep = latest_finalized_notebook_by_notary
			.get(&notary_id)
			.map(|(_, t)| *t)
			.unwrap_or_default();

		let mut vote_count = 0;
		// audit on the best block at the height of the notebook
		match self.client.runtime_api().audit_notebook_and_get_votes(
			*best_hash,
			vote_details.version,
			notary_id,
			notebook_number,
			vote_details.header_hash,
			&vote_minimums,
			&full_notebook,
			notebook_dependencies,
		)? {
			Ok(result) => {
				vote_count = result.raw_votes.len();
				let votes = result;
				self.aux_client.store_votes(vote_details.tick, votes)?;
			},
			Err(error) => {
				audit_result.is_valid = false;
				audit_result.first_error_reason = Some(error);
			},
		}

		trace!(
			target: LOG_TARGET,
			"Notebook audit result - {}. Notary {}, #{}. {} block vote(s).",
			match audit_result.is_valid {
				true => "Valid".to_string(),
				false => format!("Invalid - {:?}", audit_result.first_error_reason),
			},
			notary_id,
			notebook_number,
			vote_count
		);
		let notary_state = self.aux_client.store_notebook_result(
			audit_result,
			raw_header,
			vote_details,
			oldest_tick_to_keep,
		)?;

		Ok(notary_state)
	}

	async fn get_client(
		&self,
		notary_id: NotaryId,
	) -> Result<Arc<argon_notary_apis::Client>, Error> {
		let mut clients = self.notary_client_by_id.lock().await;
		if let std::collections::btree_map::Entry::Vacant(e) = clients.entry(notary_id) {
			let notaries = self.notaries_by_id.lock().await;
			let record = notaries.get(&notary_id).ok_or_else(|| {
				Error::NotaryError("No rpc endpoints found for notary".to_string())
			})?;
			let host = record.meta.hosts.first().ok_or_else(|| {
				Error::NotaryError("No rpc endpoint found for notary".to_string())
			})?;
			let host_str: String = host.clone().try_into().map_err(|e| {
				Error::NotaryError(format!(
					"Could not convert host to string for notary {} - {:?}",
					notary_id, e
				))
			})?;
			let c = argon_notary_apis::create_client(&host_str).await.map_err(|e| {
				Error::NotaryError(format!(
					"Could not connect to notary {} ({}) for audit - {:?}",
					notary_id, host_str, e
				))
			})?;
			let c = Arc::new(c);
			e.insert(c.clone());
		}
		let client = clients.get(&notary_id).ok_or_else(|| {
			Error::NotaryError("Could not connect to notary for audit".to_string())
		})?;
		Ok(client.clone())
	}

	async fn download_notebook(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
	) -> Result<Vec<u8>, Error> {
		let client = self.get_client(notary_id).await?;

		match client.get_raw_body(notebook_number).await {
			Err(err) => {
				self.disconnect(&notary_id, Some(format!("Error downloading notebook: {}", err)))
					.await;
				Err(Error::NotaryError(format!("Error downloading notebook: {}", err)))
			},
			Ok(body) => Ok(body),
		}
	}
}

pub async fn verify_notebook_audits<B: BlockT, C>(
	aux_client: &ArgonAux<B, C>,
	notebook_digest: &NotebookDigest<NotebookVerifyError>,
) -> Result<(), Error>
where
	C: AuxStore + 'static,
{
	let mut is_missing_entries = false;
	'retries: for _ in 0..10 {
		for digest_record in &notebook_digest.notebooks {
			let notary_audits = aux_client.get_notary_audit_history(digest_record.notary_id)?;

			match notary_audits
				.get()
				.iter()
				.find(|a| a.notebook_number == digest_record.notebook_number)
			{
				Some(audit) =>
					if digest_record.audit_first_failure != audit.first_error_reason {
						return Err(Error::InvalidNotebookDigest(format!(
							"Notary {}, notebook #{} has an audit mismatch \"{:?}\" with local result. \"{:?}\"",
							digest_record.notary_id, digest_record.notebook_number, digest_record.audit_first_failure, audit.first_error_reason
						)));
					},
				None => {
					is_missing_entries = true;
					info!(
						target: LOG_TARGET,
						"Notebook digest record not found in local storage. Delaying to allow import. Notary {}, notebook #{}",
						digest_record.notary_id, digest_record.notebook_number);
					tokio::time::sleep(std::time::Duration::from_secs(1)).await;
					continue 'retries;
				},
			}
		}
		if !is_missing_entries {
			return Ok(());
		}
	}
	Err(Error::InvalidNotebookDigest(
		"Notebook digest record could not verify all records in local storage".to_string(),
	))
}

pub async fn get_notebook_header_data<B: BlockT, C, AccountId: Codec>(
	client: &Arc<C>,
	aux_client: &ArgonAux<B, C>,
	best_hash: &B::Hash,
	submitting_tick: Tick,
) -> Result<NotebookHeaderData<NotebookVerifyError>, Error>
where
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + AuxStore + 'static,
	C::Api: NotebookApis<B, NotebookVerifyError>
		+ NotaryApis<B, NotaryRecordT>
		+ BlockSealApis<B, AccountId, BlockSealAuthorityId>,
{
	let latest_notebooks_in_runtime = client.runtime_api().latest_notebook_by_notary(*best_hash)?;
	let mut headers = NotebookHeaderData::default();
	let mut tick_notebooks = vec![];

	let notaries = client.runtime_api().notaries(*best_hash)?;
	for notary in notaries {
		let (latest_runtime_notebook_number, _) =
			latest_notebooks_in_runtime.get(&notary.notary_id).unwrap_or(&(0, 0));
		let (mut notary_headers, tick_notebook) = aux_client.get_notary_notebooks_for_header(
			notary.notary_id,
			*latest_runtime_notebook_number,
			submitting_tick,
		)?;
		notary_headers
			.notebook_digest
			.notebooks
			.sort_by(|a, b| a.notebook_number.cmp(&b.notebook_number));

		let mut expected_next_number = *latest_runtime_notebook_number;
		// if there are any notebooks supplied, they must be next in sequence
		if notary_headers.notebook_digest.notebooks.iter().any(|notebook| {
			expected_next_number += 1;
			notebook.notebook_number != expected_next_number
		}) {
			warn!(
				target: LOG_TARGET,
				"Notebook(s) missing for notary {}. Delaying to allow import.",
				notary.notary_id
			);
			continue;
		}

		headers.signed_headers.append(&mut notary_headers.signed_headers);
		headers
			.notebook_digest
			.notebooks
			.append(&mut notary_headers.notebook_digest.notebooks);
		if let Some(tick_notebook) = tick_notebook {
			tick_notebooks.push(tick_notebook);
		}
	}

	headers.vote_digest =
		client
			.runtime_api()
			.create_vote_digest(*best_hash, submitting_tick, tick_notebooks)?;
	Ok(headers)
}
