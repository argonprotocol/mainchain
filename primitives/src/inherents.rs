use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::U256;
use sp_inherents::{InherentData, InherentIdentifier, IsFatalError};
use sp_runtime::RuntimeDebug;

use ulx_notary_primitives::{BlockVote, MerkleProof, NotaryId, NotebookNumber};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"ulx_seal";

type InherentType = BlockSealInherent;

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BlockSealInherent {
	ClosestNonce {
		nonce: U256,
		notary_id: NotaryId,
		block_vote: BlockVote,
		source_notebook_number: NotebookNumber,
		source_notebook_proof: MerkleProof,
	},
	Continuation,
}

pub trait BlockSealInherentData {
	fn block_seal(&self) -> Result<Option<InherentType>, sp_inherents::Error>;
}

impl BlockSealInherentData for InherentData {
	fn block_seal(&self) -> Result<Option<InherentType>, sp_inherents::Error> {
		self.get_data(&INHERENT_IDENTIFIER)
	}
}
/// Errors that can occur while checking the timestamp inherent.
#[derive(Encode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum InherentError {
	/// The nonce doesn't match the seal.
	#[cfg_attr(
		feature = "std",
		error("The nonce used in the inherent does not match the block seal nonce.")
	)]
	WrongNonce,
	/// The proof of work is wrong
	#[cfg_attr(feature = "std", error("The wrong proof of work was used."))]
	WrongSource,
	/// The proof of work is wrong
	#[cfg_attr(feature = "std", error("The block seal is missing."))]
	MissingSeal,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		match self {
			InherentError::WrongNonce => true,
			InherentError::WrongSource => true,
			InherentError::MissingSeal => true,
		}
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
	seal: InherentType,
}

#[cfg(feature = "std")]
impl BlockSealInherentDataProvider {
	pub fn new(block_seal: InherentType) -> Self {
		Self { seal: block_seal }
	}
}

#[cfg(feature = "std")]
impl sp_std::ops::Deref for BlockSealInherentDataProvider {
	type Target = InherentType;

	fn deref(&self) -> &Self::Target {
		&self.seal
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for BlockSealInherentDataProvider {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.seal)
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
