use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::U256;
use sp_inherents::{InherentData, InherentIdentifier, IsFatalError};
use sp_runtime::RuntimeDebug;

use crate::{block_seal::BlockProof, ProofOfWorkType};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"ulx_seal";

type InherentType = UlxBlockSealInherent;

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct UlxBlockSealInherent {
	pub work_type: ProofOfWorkType,
	pub tax_nonce: Option<U256>,
	pub tax_block_proof: Option<BlockProof>,
}

pub trait UlxBlockSealInherentData {
	fn block_seal(&self) -> Result<Option<InherentType>, sp_inherents::Error>;
}

impl UlxBlockSealInherentData for InherentData {
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
	WrongProofOfWork,
	/// The inherent wasn't included
	#[cfg_attr(feature = "std", error("The proof of tax inherent is required and missing."))]
	MissingProofOfTaxInherent,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		match self {
			InherentError::WrongNonce => true,
			InherentError::WrongProofOfWork => true,
			InherentError::MissingProofOfTaxInherent => true,
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
pub struct InherentDataProvider {
	seal: InherentType,
}

#[cfg(feature = "std")]
impl InherentDataProvider {
	pub fn new(block_seal: InherentType) -> Self {
		Self { seal: block_seal }
	}
}

#[cfg(feature = "std")]
impl sp_std::ops::Deref for InherentDataProvider {
	type Target = InherentType;

	fn deref(&self) -> &Self::Target {
		&self.seal
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for InherentDataProvider {
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
