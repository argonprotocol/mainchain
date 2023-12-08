#![cfg_attr(not(feature = "std"), no_std)]

pub use block_seal::{BlockSealAuthorityId, BlockSealAuthoritySignature, BLOCK_SEAL_KEY_TYPE};
pub use digests::{BlockSealDigest, AUTHOR_DIGEST_ID, BLOCK_SEAL_DIGEST_ID};
pub use ulx_notary_primitives::{MerkleProof, NotaryId};

pub use crate::{apis::*, providers::*};

pub type ComputeDifficulty = u128;

mod apis;
pub mod block_seal;
pub mod bond;
pub mod digests;
pub mod inherents;
pub mod notary;
mod providers;
pub mod tick;

pub mod notebook {
	pub use ulx_notary_primitives::{
		AccountOrigin, AccountOriginUid, BalanceTip, BlockVotingKey, ChainTransfer,
		MaxNotebookNotarizations, NewAccountOrigin, Notarization, Notebook, NotebookHeader,
		NotebookNumber, NotebookSecretHash,
	};
}

pub mod localchain {
	pub use ulx_notary_primitives::{
		AccountType, BalanceChange, BestBlockNonce, BestBlockVoteProofT, BlockVote, BlockVoteT,
		ChannelPass, Note, NoteType, VoteMinimum,
	};
}
