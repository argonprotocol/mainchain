use std::marker::PhantomData;

use codec::Encode;
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
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Error as BlockchainError;
use sp_consensus::Error as ConsensusError;
use sp_core::{blake2_256, bytes::to_hex, U256};
use sp_inherents::Error as InherentsError;
use sp_runtime::traits::Block as BlockT;

use ulx_primitives::block_seal::{
	BlockProof, MiningAuthorityApis, SealerSignatureMessage, SEALER_SIGNATURE_PREFIX,
};

use crate::{authority::AuthoritySealer, rpc::Error::StringError};

/// Sender passed to the authorship task to report errors or successes.
pub type Sender<T> = Option<oneshot::Sender<std::result::Result<T, Error>>>;

#[rpc(client, server)]
pub trait BlockSealApi<Hash> {
	#[method(name = "blockSeal_submit")]
	async fn submit(
		&self,
		parent_hash: Hash,
		nonce: U256,
		block_proof: BlockProof,
	) -> RpcResult<CreatedBlock<Hash>>;
	#[method(name = "blockSeal_seekApproval")]
	async fn seek_approval(
		&self,
		parent_hash: Hash,
		block_proof: BlockProof,
	) -> RpcResult<ApprovalResponse>;
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ApprovalResponse {
	pub signature: String,
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
pub struct BlockSealRpc<Block, Hash, C> {
	seal_channel: mpsc::Sender<SealNewBlock<Hash>>,
	authority_sealer: AuthoritySealer<Block, C>,
	_block: PhantomData<Block>,
}

impl<Block, Hash, C> BlockSealRpc<Block, Hash, C> {
	/// Create new `ProofOfTax` instance with the given reference to the client.
	pub fn new(
		seal_channel: mpsc::Sender<SealNewBlock<Hash>>,
		authority_sealer: AuthoritySealer<Block, C>,
	) -> Self {
		Self { seal_channel, authority_sealer, _block: PhantomData }
	}
}

#[async_trait]
impl<Block, C> BlockSealApiServer<<Block as BlockT>::Hash> for BlockSealRpc<Block, Block::Hash, C>
where
	Block: BlockT,
	Block::Hash: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block> + Send + Sync + 'static,
	C::Api: MiningAuthorityApis<Block>,
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

	async fn seek_approval(
		&self,
		parent_hash: Block::Hash,
		block_proof: BlockProof,
	) -> RpcResult<ApprovalResponse> {
		let (authority_idx, authority_id, authorities) =
			match self.authority_sealer.check_if_can_seal(&parent_hash, &block_proof, false) {
				Ok(x) => x,
				Err(e) => return Err(StringError(e.to_string()).into()),
			};

		if !block_proof.seal_stampers.iter().any(|a| a.authority_idx == authority_idx) {
			return Err(StringError("Cannot sign as one of these block sealers.".into()).into())
		}

		if authorities.iter().map(|a| a.0).collect::<Vec<_>>() !=
			block_proof.seal_stampers.iter().map(|a| a.authority_idx).collect::<Vec<_>>()
		{
			return Err(StringError("Invalid block sealers proposed.".into()).into())
		}

		// TODO: do we seek charge for this?
		let signature_message = SealerSignatureMessage {
			prefix: SEALER_SIGNATURE_PREFIX,
			parent_hash,
			author_id: block_proof.author_id,
			tax_proof_id: block_proof.tax_proof_id,
			tax_amount: block_proof.tax_amount,
			seal_stampers: authorities.iter().map(|a| a.1.clone()).collect(),
		}
		.using_encoded(blake2_256);

		let signature = self
			.authority_sealer
			.sign_message(&authority_id, &signature_message[..])
			.map_err(|e| Error::StringError(e.to_string()))?;

		Ok(ApprovalResponse { signature: to_hex(&signature[..], true) })
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
