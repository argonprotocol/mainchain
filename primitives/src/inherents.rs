use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{H256, U256};
use sp_inherents::{InherentData, InherentIdentifier, IsFatalError};
use sp_runtime::RuntimeDebug;

use crate::{BestBlockVoteProofT, BlockVote, MerkleProof, NotaryId, NotebookNumber};

use crate::{BlockSealAuthoritySignature, BlockSealDigest};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"ulx_seal";
pub const INHERENT_SEAL_DIGEST_IDENTIFIER: InherentIdentifier = *b"seal_dig";

type InherentType = BlockSealInherent;

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BlockSealInherent {
	Vote {
		vote_proof: U256,
		notary_id: NotaryId,
		block_vote: BlockVote,
		source_notebook_number: NotebookNumber,
		source_notebook_proof: MerkleProof,
		miner_signature: BlockSealAuthoritySignature,
	},
	Compute,
}

impl BlockSealInherent {
	pub fn matches(&self, seal_digest: BlockSealDigest) -> bool {
		match self {
			Self::Vote { vote_proof, .. } => match seal_digest {
				BlockSealDigest::Vote { vote_proof: seal_vote_proof } =>
					vote_proof == &seal_vote_proof,
				_ => false,
			},
			Self::Compute => match seal_digest {
				BlockSealDigest::Compute { .. } => true,
				_ => false,
			},
		}
	}

	pub fn from_vote<H: Codec + AsRef<[u8]>>(
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		best_vote: BestBlockVoteProofT<H>,
		miner_signature: BlockSealAuthoritySignature,
	) -> Self {
		let vote = best_vote.block_vote;
		Self::Vote {
			vote_proof: best_vote.vote_proof,
			notary_id,
			source_notebook_number: notebook_number,
			source_notebook_proof: best_vote.source_notebook_proof,
			block_vote: BlockVote {
				account_id: vote.account_id,
				power: vote.power,
				channel_pass: vote.channel_pass,
				index: vote.index,
				grandparent_block_hash: H256::from_slice(vote.grandparent_block_hash.as_ref()),
			},
			miner_signature,
		}
	}
}

pub trait BlockSealInherentData {
	fn block_seal(&self) -> Result<Option<InherentType>, sp_inherents::Error>;
	fn digest(&self) -> Result<Option<BlockSealDigest>, sp_inherents::Error>;
}

impl BlockSealInherentData for InherentData {
	fn block_seal(&self) -> Result<Option<InherentType>, sp_inherents::Error> {
		self.get_data(&INHERENT_IDENTIFIER)
	}
	fn digest(&self) -> Result<Option<BlockSealDigest>, sp_inherents::Error> {
		self.get_data(&INHERENT_SEAL_DIGEST_IDENTIFIER)
	}
}
/// Errors that can occur while checking the timestamp inherent.
#[derive(Encode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum InherentError {
	/// The block seal is missing
	#[cfg_attr(feature = "std", error("The block seal is missing."))]
	MissingSeal,
	/// The block seal is invalid
	#[cfg_attr(feature = "std", error("The block seal does not match the digest."))]
	InvalidSeal,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

impl InherentError {
	/// Try to create an instance ouf of the given identifier and data.
	#[cfg(feature = "std")]
	pub fn try_from(id: &InherentIdentifier, mut data: &[u8]) -> Option<Self> {
		if id == &INHERENT_IDENTIFIER {
			<InherentError as codec::Decode>::decode(&mut data).ok()
		} else {
			None
		}
	}
}
#[cfg(feature = "std")]
pub struct BlockSealInherentDataProvider {
	pub seal: Option<InherentType>,
	pub digest: Option<BlockSealDigest>,
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for BlockSealInherentDataProvider {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		if let Some(seal) = &self.seal {
			inherent_data.put_data(INHERENT_IDENTIFIER, seal)?;
		}
		if let Some(digest) = &self.digest {
			inherent_data.put_data(INHERENT_SEAL_DIGEST_IDENTIFIER, digest)?;
		}
		Ok(())
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		Some(Err(sp_inherents::Error::Application(Box::from(InherentError::try_from(
			identifier, error,
		)?))))
	}
}
