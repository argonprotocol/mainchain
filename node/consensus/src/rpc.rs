use std::marker::PhantomData;

use futures::{
	channel::{mpsc, mpsc::SendError, oneshot},
	SinkExt,
};
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
use sc_consensus::ImportResult;
use serde::{Deserialize, Serialize};
use sp_blockchain::Error as BlockchainError;
use sp_consensus::Error as ConsensusError;
use sp_core::U256;
use sp_inherents::Error as InherentsError;
use sp_runtime::traits::Block as BlockT;

use ulx_primitives::BlockProof;

/// Sender passed to the authorship task to report errors or successes.
pub type Sender<T> = Option<oneshot::Sender<std::result::Result<T, Error>>>;

#[rpc(client, server)]
pub trait BlockSealApi<Hash> {
	#[method(name = "proofOfTax_submit")]
	async fn submit(
		&self,
		parent_hash: Hash,
		nonce: U256,
		block_proof: BlockProof,
	) -> RpcResult<CreatedBlock<Hash>>;
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CreatedBlock<Hash> {
	/// hash of the created block.
	pub hash: Hash,
}

pub enum SealNewBlock<Hash> {
	Submit {
		block_proof: BlockProof,
		parent_hash: Hash,
		nonce: U256,
		/// sender to report errors/success to the rpc.
		sender: Sender<CreatedBlock<Hash>>,
	},
}

/// A struct that implements the `ProofOfTaxApi`.
pub struct BlockSealRpc<Block, Hash> {
	seal_channel: mpsc::Sender<SealNewBlock<Hash>>,
	_block: PhantomData<Block>,
}

impl<Block, Hash> BlockSealRpc<Block, Hash> {
	/// Create new `ProofOfTax` instance with the given reference to the client.
	pub fn new(seal_channel: mpsc::Sender<SealNewBlock<Hash>>) -> Self {
		Self { seal_channel, _block: PhantomData }
	}
}

#[async_trait]
impl<Block> BlockSealApiServer<<Block as BlockT>::Hash> for BlockSealRpc<Block, Block::Hash>
where
	Block: BlockT,
	Block::Hash: Send + Sync + 'static,
{
	async fn submit(
		&self,
		parent_hash: Block::Hash,
		nonce: U256,
		block_proof: BlockProof,
	) -> RpcResult<CreatedBlock<Block::Hash>> {
		// TBD: how do we create a cost to submitting this...
		// also... are we putting any validation on the runtime? Or only in inherent?
		let mut sink = self.seal_channel.clone();
		let (sender, receiver) = oneshot::channel();
		let command =
			SealNewBlock::Submit { block_proof, sender: Some(sender), nonce, parent_hash };

		sink.send(command).await?;

		match receiver.await {
			Ok(Ok(rx)) => Ok(rx),
			Ok(Err(e)) => Err(e.into()),
			Err(e) => Err(JsonRpseeError::to_call_error(e)),
		}
	}
}

/// Error code for rpc
mod codes {
	pub const SERVER_SHUTTING_DOWN: i32 = 10_000;
	pub const BLOCK_IMPORT_FAILED: i32 = 11_000;
	pub const BLOCK_NOT_FOUND: i32 = 13_000;
	pub const CONSENSUS_ERROR: i32 = 14_000;
	pub const INHERENTS_ERROR: i32 = 15_000;
	pub const BLOCKCHAIN_ERROR: i32 = 16_000;
	pub const UNKNOWN_ERROR: i32 = 20_000;
}

/// errors encountered by background block authorship task
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// An error occurred while importing the block
	#[error("Block import failed: {0:?}")]
	BlockImportError(ImportResult),
	#[error("Didn't solve block")]
	DidNotSolveBlock,
	/// encountered during creation of Proposer.
	#[error("Consensus Error: {0}")]
	ConsensusError(#[from] ConsensusError),
	/// Failed to create Inherents data
	#[error("Inherents Error: {0}")]
	InherentError(#[from] InherentsError),
	/// error encountered during finalization
	#[error("Finalization Error: {0}")]
	BlockchainError(#[from] BlockchainError),
	/// Supplied parent_hash doesn't exist in chain
	#[error("Supplied parent_hash: {0} doesn't exist in chain")]
	BlockNotFound(String),
	/// Some string error
	#[error("{0}")]
	StringError(String),
	/// send error
	#[error("Consensus process is terminating")]
	Canceled(#[from] oneshot::Canceled),
	/// send error
	#[error("Consensus process is terminating")]
	SendError(#[from] SendError),
	/// Some other error.
	#[error("Other error: {0}")]
	Other(Box<dyn std::error::Error + Send + Sync>),
}

impl From<ImportResult> for Error {
	fn from(err: ImportResult) -> Self {
		Error::BlockImportError(err)
	}
}

impl From<String> for Error {
	fn from(s: String) -> Self {
		Error::StringError(s)
	}
}

impl Error {
	fn to_code(&self) -> i32 {
		use Error::*;
		match self {
			BlockImportError(_) => codes::BLOCK_IMPORT_FAILED,
			BlockNotFound(_) => codes::BLOCK_NOT_FOUND,
			ConsensusError(_) => codes::CONSENSUS_ERROR,
			InherentError(_) => codes::INHERENTS_ERROR,
			BlockchainError(_) => codes::BLOCKCHAIN_ERROR,
			SendError(_) | Canceled(_) => codes::SERVER_SHUTTING_DOWN,
			_ => codes::UNKNOWN_ERROR,
		}
	}
}

impl From<Error> for JsonRpseeError {
	fn from(err: Error) -> Self {
		CallError::Custom(ErrorObject::owned(err.to_code(), err.to_string(), None::<()>)).into()
	}
}
