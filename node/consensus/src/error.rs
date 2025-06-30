use argon_primitives::AccountId;
use polkadot_sdk::*;
use sc_consensus::ImportResult;
use sp_api::ApiError;
use sp_blockchain::Error as BlockchainError;
use sp_consensus::Error as ConsensusError;
use sp_inherents::Error as InherentsError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Header uses the wrong engine {0:?}")]
	WrongEngine([u8; 4]),
	#[error("The notebook digest record is invalid {0}")]
	InvalidNotebookDigest(String),

	#[error("Creating inherents failed: {0}")]
	CreateInherents(sp_inherents::Error),
	#[error("Checking inherents failed: {0}")]
	CheckInherents(sp_inherents::Error),
	#[error("Invalid compute nonce provided")]
	InvalidComputeNonce,
	#[error(
		"Checking inherents unknown error for identifier: {}",
		String::from_utf8_lossy(.0)
	)]
	CheckInherentsUnknownError(sp_inherents::InherentIdentifier),
	#[error("Duplicate {0} pre-runtime digests")]
	DuplicatePreRuntimeDigest(String),
	#[error("Missing {0} pre-runtime digest")]
	MissingPreRuntimeDigest(String),

	#[error("Missing block seal digest")]
	MissingBlockSealDigest,
	#[error("Unable to decode digest {0}")]
	UnableToDecodeDigest(String),

	#[error("Invalid vote seal signature")]
	InvalidVoteSealSignature,

	#[error(transparent)]
	Client(sp_blockchain::Error),
	#[error(transparent)]
	Api(#[from] ApiError),
	#[error(transparent)]
	Codec(codec::Error),
	#[error("{0}")]
	Environment(String),
	#[error("{0}")]
	Runtime(String),
	#[error("Missing runtime data {0}")]
	MissingRuntimeData(String),
	#[error("Block import error {0:?}")]
	BlockImportError(ImportResult),
	#[error(transparent)]
	ConsensusError(#[from] ConsensusError),
	#[error(transparent)]
	InherentError(#[from] InherentsError),
	#[error(transparent)]
	BlockchainError(#[from] BlockchainError),
	#[error("Supplied parent_hash: {0} doesn't exist in chain")]
	BlockNotFound(String),
	#[error("{0}")]
	StringError(String),

	#[error("Internal channel send error {0}")]
	SendError(#[from] futures::channel::mpsc::SendError),

	#[error("Notary error: {0}")]
	NotaryError(String),
	#[error("The notebook can't be audited yet {0}")]
	NotebookAuditBeforeTick(String),
	#[error("Notary archive error: {0}")]
	NotaryArchiveError(String),
	#[error("A block could not be verified because a notary could not be synchronized with. {0}")]
	UnableToSyncNotary(String),

	#[error("Notary sync missing notebook dependencies: {0}")]
	MissingNotebooksError(String),

	#[error("A duplicate block was created by this author {0} for the given {1} key")]
	DuplicateAuthoredBlock(AccountId, String),

	#[error("The block state is not available")]
	StateUnavailableError,
}

impl From<String> for Error {
	fn from(err: String) -> Self {
		Error::StringError(err)
	}
}

impl From<Error> for String {
	fn from(error: Error) -> String {
		error.to_string()
	}
}

impl From<Error> for ConsensusError {
	fn from(error: Error) -> ConsensusError {
		ConsensusError::ClientImport(error.to_string())
	}
}
