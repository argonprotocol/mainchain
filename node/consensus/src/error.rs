use sc_consensus::ImportResult;
use sp_api::ApiError;
use sp_blockchain::Error as BlockchainError;
use sp_consensus::Error as ConsensusError;
use sp_inherents::Error as InherentsError;
use sp_runtime::RuntimeString;
use ulx_node_runtime::AccountId;
use ulx_primitives::{tick::Tick, NotaryId, NotebookNumber};

#[derive(thiserror::Error, std::fmt::Debug)]
pub enum Error {
	#[error("Header uses the wrong engine {0:?}")]
	WrongEngine([u8; 4]),
	#[error("Block seal signature missing or invalid")]
	InvalidSealSignature,
	#[error("The notebook digest record is invalid {0}")]
	InvalidNotebookDigest(String),
	#[error("Fetching best header failed: {0}")]
	NoBestHeader(String),
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

	#[error("Duplicate seal signature digests")]
	DuplicateBlockSealDigest,
	#[error("Missing block seal digest")]
	MissingBlockSealDigest,

	#[error("Duplicate consensus digest")]
	DuplicateConsensusDigest,
	#[error("Missing consensus digest")]
	MissingConsensusDigest,

	#[error("Failed to sync Bitcoin Utxos")]
	FailedToSyncBitcoinUtxos,

	#[error(transparent)]
	Client(sp_blockchain::Error),
	#[error(transparent)]
	Api(#[from] ApiError),
	#[error(transparent)]
	Codec(codec::Error),
	#[error("{0}")]
	Environment(String),
	#[error("{0}")]
	Runtime(RuntimeString),
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

	#[error("A duplicate block was created by this author {0} for the given voting key")]
	DuplicateAuthoredBlock(AccountId),

	#[error("Notebook error while building block: {0}")]
	NotebookHeaderBuildError(String),

	#[error("Duplicate notebook at tick {0}. Notary {1}, notebook {2}")]
	DuplicateNotebookAtTick(Tick, NotaryId, NotebookNumber),
}

#[cfg(test)]
impl PartialEq for Error {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Error::WrongEngine(s1), Error::WrongEngine(s2)) => s1 == s2,
			(Error::InvalidSealSignature, Error::InvalidSealSignature) => true,
			(Error::CreateInherents(s1), Error::CreateInherents(s2)) =>
				s1.to_string() == s2.to_string(),
			(Error::CheckInherents(s1), Error::CheckInherents(s2)) =>
				s1.to_string() == s2.to_string(),
			(Error::CheckInherentsUnknownError(s1), Error::CheckInherentsUnknownError(s2)) =>
				s1 == s2,
			(Error::BlockImportError(s1), Error::BlockImportError(s2)) => s1 == s2,
			(Error::ConsensusError(s1), Error::ConsensusError(s2)) =>
				s1.to_string() == s2.to_string(),
			(Error::BlockNotFound(s1), Error::BlockNotFound(s2)) => s1 == s2,
			(Error::StringError(s1), Error::StringError(s2)) => s1 == s2,
			(Error::NotaryError(s1), Error::NotaryError(s2)) => s1 == s2,
			(Error::DuplicateAuthoredBlock(s1), Error::DuplicateAuthoredBlock(s2)) => s1 == s2,
			(Error::NotebookHeaderBuildError(s1), Error::NotebookHeaderBuildError(s2)) => s1 == s2,
			(
				Error::DuplicateNotebookAtTick(t1, n1, nb1),
				Error::DuplicateNotebookAtTick(t2, n2, nb2),
			) => t1 == t2 && n1 == n2 && nb1 == nb2,

			_ => false,
		}
	}
}
impl From<ImportResult> for Error {
	fn from(err: ImportResult) -> Self {
		Error::BlockImportError(err)
	}
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
