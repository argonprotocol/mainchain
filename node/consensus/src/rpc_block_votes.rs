use std::{
	collections::{BTreeMap, BTreeSet},
	marker::PhantomData,
	sync::Arc,
};

use codec::Codec;
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
use sc_client_api::AuxStore;
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::{Block as BlockT, Header};

use ulx_primitives::{localchain::BlockVoteT, MiningAuthorityApis, NotaryId};

use crate::{
	authority::AuthorityClient, aux::UlxAux, convert_u32, rpc_block_votes::Error::StringError,
};

#[rpc(client, server)]
pub trait BlockVoteApi<Hash: Codec> {
	#[method(name = "blockVotes_submit")]
	async fn submit(
		&self,
		notary_id: NotaryId,
		block_votes: Vec<BlockVoteT<Hash>>,
	) -> RpcResult<Response>;
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Response {
	/// hash of the created block.
	pub accepted: bool,
}

pub struct BlockVoteRpc<Block, C> {
	client: Arc<C>,
	keystore: KeystorePtr,
	_block: PhantomData<Block>,
}

impl<Block, C> BlockVoteRpc<Block, C> {
	/// Create new `ProofOfTax` instance with the given reference to the client.
	pub fn new(client: Arc<C>, keystore: KeystorePtr) -> Self {
		Self { client, keystore, _block: PhantomData }
	}
}

#[async_trait]
impl<B, C> BlockVoteApiServer<<B as BlockT>::Hash> for BlockVoteRpc<B, C>
where
	B: BlockT,
	B::Hash: Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + AuxStore + Send + Sync + 'static,
	C::Api: MiningAuthorityApis<B>,
{
	async fn submit(
		&self,
		notary_id: NotaryId,
		block_votes: Vec<BlockVoteT<B::Hash>>,
	) -> RpcResult<Response> {
		let mut block_numbers = BTreeMap::new();
		let mut account_ids = BTreeSet::new();
		let current = convert_u32::<B>(&self.client.info().best_number);
		if current < 2 {
			return Err(StringError(format!("BlockVoteRpc: Not accepting votes yet")).into())
		}
		for block_vote in block_votes {
			let block_hash = block_vote.grandparent_block_hash;
			if !block_numbers.contains_key(&block_hash) {
				let block_number = self.get_block_number(block_vote.grandparent_block_hash).await?;
				block_numbers.insert(block_hash, block_number);
			}
			let needs_lookup = account_ids.insert(block_vote.account_id.clone());

			if needs_lookup &&
				!AuthorityClient::can_seal_tax_vote(
					&self.client,
					&self.keystore,
					&block_hash,
					&block_vote.account_id,
				)
				.is_ok()
			{
				return Err(StringError(format!(
					"BlockVoteRpc: Cannot server as block peer for this account: {:?}",
					block_vote.account_id
				))
				.into())
			}

			let block_number = block_numbers.get(&block_hash).unwrap_or(&0u32).clone();
			if block_number < current - 2u32 {
				return Err(StringError(format!(
					"BlockVoteRpc: Not accepting votes for previous blocks: {:?} < {:?}",
					block_numbers, current
				))
				.into())
			}
			UlxAux::<C, B>::store_vote(
				self.client.as_ref(),
				notary_id,
				block_vote.clone(),
				block_number,
			)
			.map_err(|e| StringError(format!("BlockVoteRpc: Failed to record vote: {:?}", e)))?;
		}

		Ok(Response { accepted: true })
	}
}

impl<B, C> BlockVoteRpc<B, C>
where
	B: BlockT,
	B::Hash: Send + Sync + 'static,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + AuxStore + Send + Sync + 'static,
	C::Api: MiningAuthorityApis<B>,
{
	async fn get_block_number(&self, block_hash: B::Hash) -> Result<u32, Error> {
		let Some(block_header) = self.client.header(block_hash).map_err(|e| {
			StringError(format!(
				"BlockVoteRpc: Failed to fetch parent block with hash: {:?} {:?}",
				block_hash, e
			))
		})?
		else {
			return Err(StringError(format!(
				"BlockVoteRpc: Failed to fetch parent block with hash: {:?}",
				block_hash
			))
			.into())
		};
		let block_number = convert_u32::<B>(&block_header.number());
		Ok(block_number)
	}
}

/// Error code for rpc
mod codes {
	pub const UNKNOWN_ERROR: i32 = 20_000;
}

/// errors encountered by background block authorship task
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// Some string error
	#[error("{0}")]
	StringError(String),
	/// Some other error.
	#[error("Other error: {0}")]
	Other(Box<dyn std::error::Error + Send + Sync>),
}

impl From<String> for Error {
	fn from(s: String) -> Self {
		Error::StringError(s)
	}
}

impl Error {
	fn to_code(&self) -> i32 {
		match self {
			_ => codes::UNKNOWN_ERROR,
		}
	}
}

impl From<Error> for JsonRpseeError {
	fn from(err: Error) -> Self {
		CallError::Custom(ErrorObject::owned(err.to_code(), err.to_string(), None::<()>)).into()
	}
}
