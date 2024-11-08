use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;
use sp_core::U256;
use sp_inherents::{InherentData, InherentIdentifier, IsFatalError};
use sp_runtime::RuntimeDebug;

use crate::{
	bitcoin::{BitcoinBlock, BitcoinHeight, BitcoinRejectedReason, UtxoId, UtxoRef},
	BestBlockVoteSeal, BlockSealDigest, BlockVote, MerkleProof, NotaryId, NotebookNumber,
	SignedNotebookHeader,
};

pub const SEAL_INHERENT_IDENTIFIER: InherentIdentifier = *b"seal_arg";
pub const SEAL_INHERENT_VOTE_IDENTIFIER: InherentIdentifier = *b"seal_vot";
pub const SEAL_INHERENT_DIGEST_IDENTIFIER: InherentIdentifier = *b"seal_dig";
pub const NOTEBOOKS_INHERENT_IDENTIFIER: InherentIdentifier = *b"notebook";
pub const BITCOIN_INHERENT_IDENTIFIER: InherentIdentifier = *b"bitcoin_";

#[allow(clippy::large_enum_variant)]
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BlockSealInherent {
	Vote {
		seal_strength: U256,
		#[codec(compact)]
		notary_id: NotaryId,
		#[codec(compact)]
		source_notebook_number: NotebookNumber,
		source_notebook_proof: MerkleProof,
		block_vote: BlockVote,
	},
	Compute,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BlockSealInherentNodeSide {
	Vote {
		seal_strength: U256,
		#[codec(compact)]
		notary_id: NotaryId,
		#[codec(compact)]
		source_notebook_number: NotebookNumber,
		source_notebook_proof: MerkleProof,
		/// Encoded block vote on node side
		block_vote_bytes: Vec<u8>,
	},
	Compute,
}

impl BlockSealInherentNodeSide {
	pub fn from_vote<A: Codec, Auth: Codec>(best_vote: BestBlockVoteSeal<A, Auth>) -> Self {
		Self::Vote {
			notary_id: best_vote.notary_id,
			seal_strength: best_vote.seal_strength,
			source_notebook_number: best_vote.source_notebook_number,
			source_notebook_proof: best_vote.source_notebook_proof,
			block_vote_bytes: best_vote.block_vote_bytes,
		}
	}
}

impl BlockSealInherent {
	pub fn matches(&self, seal_digest: BlockSealDigest) -> bool {
		match self {
			Self::Vote { seal_strength, .. } => match seal_digest {
				BlockSealDigest::Vote { seal_strength: included_seal_strength, .. } =>
					seal_strength == &included_seal_strength,
				_ => false,
			},
			Self::Compute => matches!(seal_digest, BlockSealDigest::Compute { .. }),
		}
	}
}

impl TryInto<BlockSealInherent> for BlockSealInherentNodeSide {
	type Error = sp_inherents::Error;
	fn try_into(self) -> Result<BlockSealInherent, Self::Error> {
		Ok(match self {
			BlockSealInherentNodeSide::Vote {
				seal_strength,
				notary_id,
				block_vote_bytes,
				source_notebook_number,
				source_notebook_proof,
			} => BlockSealInherent::Vote {
				seal_strength,
				notary_id,
				block_vote: BlockVote::decode(&mut block_vote_bytes.as_slice()).map_err(|e| {
					sp_inherents::Error::DecodingFailed(e, SEAL_INHERENT_VOTE_IDENTIFIER)
				})?,
				source_notebook_number,
				source_notebook_proof,
			},
			BlockSealInherentNodeSide::Compute => BlockSealInherent::Compute,
		})
	}
}

pub trait BlockSealInherentData {
	fn block_seal(&self) -> Result<Option<BlockSealInherent>, sp_inherents::Error>;
	fn digest(&self) -> Result<Option<BlockSealDigest>, sp_inherents::Error>;
}

impl BlockSealInherentData for InherentData {
	fn block_seal(&self) -> Result<Option<BlockSealInherent>, sp_inherents::Error> {
		if let Some(x) = self.get_data::<BlockSealInherentNodeSide>(&SEAL_INHERENT_IDENTIFIER)? {
			let result = x.try_into()?;
			return Ok(Some(result));
		}
		Ok(None)
	}
	fn digest(&self) -> Result<Option<BlockSealDigest>, sp_inherents::Error> {
		self.get_data(&SEAL_INHERENT_DIGEST_IDENTIFIER)
	}
}

/// Errors that can occur while checking the timestamp inherent.
#[derive(Encode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum SealInherentError {
	/// The block seal is missing
	#[cfg_attr(feature = "std", error("The block seal is missing."))]
	MissingSeal,
	/// The block seal is invalid
	#[cfg_attr(feature = "std", error("The block seal does not match the digest."))]
	InvalidSeal,
}

impl IsFatalError for SealInherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}
impl SealInherentError {
	/// Try to create an instance ouf of the given identifier and data.
	#[cfg(feature = "std")]
	pub fn try_from(id: &InherentIdentifier, mut data: &[u8]) -> Option<Self> {
		if id == &SEAL_INHERENT_IDENTIFIER {
			<SealInherentError as codec::Decode>::decode(&mut data).ok()
		} else {
			None
		}
	}
}
#[cfg(feature = "std")]
pub struct BlockSealInherentDataProvider {
	pub seal: Option<BlockSealInherentNodeSide>,
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
			inherent_data.put_data(SEAL_INHERENT_IDENTIFIER, seal)?;
		}
		if let Some(digest) = &self.digest {
			inherent_data.put_data(SEAL_INHERENT_DIGEST_IDENTIFIER, digest)?;
		}
		Ok(())
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		Some(Err(sp_inherents::Error::Application(Box::from(SealInherentError::try_from(
			identifier, error,
		)?))))
	}
}

/////// NOTEBOOK INHERENT ///////

pub trait NotebookInherentData {
	fn notebooks(&self) -> Result<Option<Vec<SignedNotebookHeader>>, sp_inherents::Error>;
}

impl NotebookInherentData for InherentData {
	fn notebooks(&self) -> Result<Option<Vec<SignedNotebookHeader>>, sp_inherents::Error> {
		let raw = self.get_data::<Vec<Vec<u8>>>(&NOTEBOOKS_INHERENT_IDENTIFIER)?;
		if let Some(raw) = raw {
			let mut result: Vec<SignedNotebookHeader> = Vec::new();
			for data in raw {
				let entry = SignedNotebookHeader::decode(&mut data.as_slice()).map_err(|e| {
					sp_inherents::Error::DecodingFailed(e, NOTEBOOKS_INHERENT_IDENTIFIER)
				})?;
				result.push(entry);
			}
			return Ok(Some(result));
		}
		Ok(None)
	}
}
#[cfg(feature = "std")]
pub struct NotebooksInherentDataProvider {
	pub raw_notebooks: Vec<Vec<u8>>,
}
#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for NotebooksInherentDataProvider {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(NOTEBOOKS_INHERENT_IDENTIFIER, &self.raw_notebooks)?;
		Ok(())
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		Some(Err(sp_inherents::Error::Application(Box::from(NotebookInherentError::try_from(
			identifier, error,
		)?))))
	}
}

#[derive(Encode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum NotebookInherentError {
	/// The block seal is missing
	#[cfg_attr(feature = "std", error("The notebook inherent is missing."))]
	MissingInherent,
	/// The inherent has a mismatch with the details included in the block
	#[cfg_attr(feature = "std", error("The notebook inherent does not match the block."))]
	InherentMismatch,
}

impl NotebookInherentError {
	/// Try to create an instance ouf of the given identifier and data.
	#[cfg(feature = "std")]
	pub fn try_from(id: &InherentIdentifier, mut data: &[u8]) -> Option<Self> {
		if id == &NOTEBOOKS_INHERENT_IDENTIFIER {
			<NotebookInherentError as codec::Decode>::decode(&mut data).ok()
		} else {
			None
		}
	}
}
impl IsFatalError for NotebookInherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

/////// BITCOIN INHERENT ///////

pub trait BitcoinInherentData {
	fn bitcoin_sync(&self) -> Result<Option<BitcoinUtxoSync>, sp_inherents::Error>;
}

impl BitcoinInherentData for InherentData {
	fn bitcoin_sync(&self) -> Result<Option<BitcoinUtxoSync>, sp_inherents::Error> {
		self.get_data(&BITCOIN_INHERENT_IDENTIFIER)
	}
}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct BitcoinUtxoSync {
	pub spent: BTreeMap<UtxoId, BitcoinHeight>,
	pub verified: BTreeMap<UtxoId, UtxoRef>,
	pub invalid: BTreeMap<UtxoId, BitcoinRejectedReason>,
	pub sync_to_block: BitcoinBlock,
}

#[cfg(feature = "std")]
pub struct BitcoinInherentDataProvider {
	pub bitcoin_utxo_sync: BitcoinUtxoSync,
}
#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for BitcoinInherentDataProvider {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(BITCOIN_INHERENT_IDENTIFIER, &self.bitcoin_utxo_sync)?;
		Ok(())
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		Some(Err(sp_inherents::Error::Application(Box::from(BitcoinInherentError::try_from(
			identifier, error,
		)?))))
	}
}

#[derive(Encode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum BitcoinInherentError {
	/// The inherent has a mismatch with the details coordinated between bitcoin and the block
	#[cfg_attr(
		feature = "std",
		error("The bitcoin inherent does not match the bitcoin state compared to the block.")
	)]
	InvalidInherentData,
}

impl BitcoinInherentError {
	/// Try to create an instance ouf of the given identifier and data.
	#[cfg(feature = "std")]
	pub fn try_from(id: &InherentIdentifier, mut data: &[u8]) -> Option<Self> {
		if id == &BITCOIN_INHERENT_IDENTIFIER {
			<BitcoinInherentError as codec::Decode>::decode(&mut data).ok()
		} else {
			None
		}
	}
}
impl IsFatalError for BitcoinInherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}
