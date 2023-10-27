#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

pub use block_seal::{BlockSealAuthorityId, BlockSealAuthoritySignature, BLOCK_SEAL_KEY_TYPE};
pub use digests::{Difficulty, ProofOfWorkType, UlxPreDigest, UlxSeal, AUTHOR_ID, ULX_ENGINE_ID};

pub mod block_seal;
pub mod bond;
pub mod digests;
pub mod inherents;
pub mod notary;

pub mod notebook {
	pub use ulx_notary_primitives::{
		AccountOrigin, AccountOriginUid, AuditedNotebook, BalanceTip, ChainTransfer,
		MaxBalanceChanges, Notebook, NotebookHeader, NotebookNumber, RequiredNotebookAuditors,
	};
}

pub mod localchain {
	pub use ulx_notary_primitives::{BalanceChange, Chain, Note, NoteType};
}

sp_api::decl_runtime_apis! {
	/// This runtime api allows people to query the current authority set
	pub trait UlxConsensusApi {
		fn next_work() -> NextWork;
		fn calculate_easing(tax_amount: u128, validators: u8) -> Difficulty;
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct NextWork {
	pub work_type: ProofOfWorkType,
	#[codec(compact)]
	pub difficulty: Difficulty,
	#[codec(compact)]
	pub closest_x_authorities_required: u32,
	#[codec(compact)]
	pub min_seal_signers: u32,
}
