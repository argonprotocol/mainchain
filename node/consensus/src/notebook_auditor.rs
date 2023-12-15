use log::trace;
use std::{collections::BTreeMap, default::Default, marker::PhantomData, sync::Arc};

use sc_client_api::AuxStore;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block as BlockT;

use ulx_node_runtime::{NotaryRecordT, NotebookVerifyError};
use ulx_notary::apis::notebook::NotebookRpcClient;
use ulx_primitives::{
	block_seal::Host, notary::NotaryNotebookVoteDetails, notebook::NotebookNumber,
	BlockSealSpecApis, NotaryApis, NotaryId, NotebookApis,
};

use crate::{aux::UlxAux, error::Error};

pub struct NotebookAuditor<B: BlockT, C> {
	client: Arc<C>,
	notary_client_by_id: BTreeMap<NotaryId, Arc<ulx_notary::Client>>,
	_block: PhantomData<B>,
}

const LOG_TARGET: &str = "node::notebook_auditor";

#[async_trait::async_trait]
pub trait NotebookAuditorProvider<B: BlockT> {
	async fn audit_notebook(
		&mut self,
		block_hash: &B::Hash,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<(), Error<B>>;
}

#[async_trait::async_trait]
impl<B, C> NotebookAuditorProvider<B> for NotebookAuditor<B, C>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + AuxStore + Send + Sync + 'static,
	C::Api:
		NotaryApis<B, NotaryRecordT> + NotebookApis<B, NotebookVerifyError> + BlockSealSpecApis<B>,
{
	async fn audit_notebook(
		&mut self,
		block_hash: &B::Hash,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<(), Error<B>> {
		self.try_audit_notebook(block_hash, vote_details).await
	}
}

impl<B, C> NotebookAuditor<B, C>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + AuxStore,
	C::Api:
		NotaryApis<B, NotaryRecordT> + NotebookApis<B, NotebookVerifyError> + BlockSealSpecApis<B>,
{
	pub fn new(client: Arc<C>) -> Self {
		Self { client, notary_client_by_id: Default::default(), _block: PhantomData }
	}

	pub async fn try_audit_notebook(
		&mut self,
		block_hash: &B::Hash,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<(), Error<B>> {
		let notary_id = vote_details.notary_id;
		let notebook_number = vote_details.notebook_number;

		let notary_details =
			self.client.runtime_api().notary_by_id(*block_hash, notary_id)?.ok_or(
				Error::NotaryError(format!(
					"Unknown notebook submitted. Notary id {}, notebook = {}",
					notary_id, notebook_number
				)),
			)?;

		let rpc_hosts = notary_details.meta.hosts.to_vec().clone();

		let full_notebook = self.download_notebook(notary_id, notebook_number, rpc_hosts).await?;
		let mut vote_minimums = BTreeMap::new();
		for block_hash in &vote_details.blocks_with_votes {
			vote_minimums.insert(
                block_hash.clone(),
                self.client.runtime_api().vote_minimum(block_hash.clone()).map_err(|e| {
                    let message = format!(
                        "Notebook failed audit. Skipping continuation. Notary {}, notebook {}. {:?}",
                        notary_id, notebook_number, e
                    );
                    Error::<B>::StringError(message)
                })?,
            );
		}
		// audit on the best block at the height of the notebook
		let votes = self
			.client
			.runtime_api()
			.audit_notebook_and_get_votes(
				block_hash.clone(),
				vote_details.version,
				notary_id,
				notebook_number,
				vote_details.header_hash.clone(),
				&vote_minimums,
				&full_notebook,
			)?
			.map_err(|e| {
				let message = format!(
					"Notebook failed audit. Skipping continuation. Notary {}, notebook {}. {:?}",
					notary_id, notebook_number, e
				);
				Error::<B>::StringError(message)
			})?;

		trace!(
			target: LOG_TARGET,
			"Notebook audit successful. Notary {}, #{}. {} block vote(s).",
			notary_id,
			notebook_number,
			votes.raw_votes.len()
		);

		UlxAux::store_votes(self.client.as_ref(), vote_details.tick, notary_id, votes)?;
		Ok(())
	}

	async fn download_notebook(
		&mut self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		rpc_hosts: Vec<Host>,
	) -> Result<Vec<u8>, Error<B>> {
		if !self.notary_client_by_id.contains_key(&notary_id) {
			let host = rpc_hosts.get(0).ok_or_else(|| {
				Error::NotaryError("No rpc endpoint found for notary".to_string())
			})?;
			let c = ulx_notary::create_client(host.get_url().as_str()).await.map_err(|e| {
				Error::NotaryError(format!("Could not connect to notary for audit - {:?}", e))
			})?;
			let c = Arc::new(c);
			self.notary_client_by_id.insert(notary_id, c.clone());
		}

		let Some(client) = self.notary_client_by_id.get(&notary_id) else {
			return Err(Error::NotaryError("Could not connect to notary for audit".to_string()))
		};
		client.get_raw_body(notebook_number).await.map_err(|err| {
			self.notary_client_by_id.remove(&notary_id);
			Error::NotaryError(format!("Error downloading notebook: {}", err))
		})
	}
}
