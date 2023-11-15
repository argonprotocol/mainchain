use codec::Decode;
use sp_runtime::{
	generic::DigestItem,
	traits::{Block as BlockT, Header as HeaderT},
};

use ulx_node_runtime::AccountId;
use ulx_primitives::{
	block_seal::BlockVoteEligibility,
	digests::{
		BlockSealSignatureDigest, BlockVoteDigest, FinalizedBlockNeededDigest,
		BLOCK_VOTES_DIGEST_ID, COMPUTE_AUTHORITY_DIGEST_ID, FINALIZED_BLOCK_DIGEST_ID,
		NEXT_VOTE_ELIGIBILITY_DIGEST_ID, SIGNATURE_DIGEST_ID,
	},
	BlockSealAuthorityId, BlockSealDigest, AUTHOR_DIGEST_ID, BLOCK_SEAL_DIGEST_ID,
};

use crate::Error;

pub struct Digests<B: BlockT> {
	pub finalized_block: FinalizedBlockNeededDigest<B>,
	pub signature: BlockSealSignatureDigest,
	pub author: AccountId,
	pub compute_authority: Option<BlockSealAuthorityId>,
	pub block_vote: BlockVoteDigest,
	pub block_seal: BlockSealDigest,
	pub next_eligibility: BlockVoteEligibility,
}

pub fn load_digests<B: BlockT>(header: &B::Header) -> Result<Digests<B>, Error<B>> {
	let mut author = None;
	let mut finalized_block = None;
	let mut signature = None;
	let mut block_vote = None;
	let mut next_eligibility = None;
	let mut block_seal = None;
	let mut compute_authority = None;

	for log in header.digest().logs() {
		match log {
			DigestItem::PreRuntime(BLOCK_SEAL_DIGEST_ID, v) => {
				if block_seal.is_some() {
					return Err(Error::DuplicatePreRuntimeDigest("BlockSealDigest".to_string()))
				}
				let digest = BlockSealDigest::decode(&mut &v[..])
					.map_err(|e| Error::<B>::Codec(e.clone()))?;
				block_seal = Some(digest);
			},
			DigestItem::PreRuntime(FINALIZED_BLOCK_DIGEST_ID, v) => {
				if finalized_block.is_some() {
					return Err(Error::DuplicatePreRuntimeDigest(
						"FinalizedBlockNeededDigest".to_string(),
					))
				}
				let digest = FinalizedBlockNeededDigest::<B>::decode(&mut &v[..])
					.map_err(|e| Error::<B>::Codec(e.clone()))?;
				finalized_block = Some(digest);
			},
			DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, v) => {
				if block_vote.is_some() {
					return Err(Error::DuplicatePreRuntimeDigest("BlockVoteDigest".to_string()))
				}
				let digest = BlockVoteDigest::decode(&mut &v[..])
					.map_err(|e| Error::<B>::Codec(e.clone()))?;
				block_vote = Some(digest);
			},
			DigestItem::PreRuntime(AUTHOR_DIGEST_ID, v) => {
				if author.is_some() {
					return Err(Error::DuplicatePreRuntimeDigest("AuthorDigest".to_string()))
				}
				let digest =
					AccountId::decode(&mut &v[..]).map_err(|e| Error::<B>::Codec(e.clone()))?;
				author = Some(digest);
			},
			DigestItem::PreRuntime(COMPUTE_AUTHORITY_DIGEST_ID, v) => {
				if compute_authority.is_some() {
					return Err(Error::DuplicatePreRuntimeDigest(
						"ComputeAuthorityDigest".to_string(),
					))
				}
				let digest = BlockSealAuthorityId::decode(&mut &v[..])
					.map_err(|e| Error::<B>::Codec(e.clone()))?;
				compute_authority = Some(digest);
			},
			DigestItem::Consensus(NEXT_VOTE_ELIGIBILITY_DIGEST_ID, v) => {
				if next_eligibility.is_some() {
					return Err(Error::DuplicateConsensusDigest)
				}
				let digest = BlockVoteEligibility::decode(&mut &v[..])
					.map_err(|e| Error::<B>::Codec(e.clone()))?;
				next_eligibility = Some(digest);
			},
			DigestItem::Seal(SIGNATURE_DIGEST_ID, v) => {
				if signature.is_some() {
					return Err(Error::DuplicateSignatureDigest)
				}
				let digest = BlockSealSignatureDigest::decode(&mut &v[..])
					.map_err(|e| Error::<B>::Codec(e.clone()))?;
				signature = Some(digest);
			},
			_ => {},
		}
	}

	Ok(Digests {
		block_seal: block_seal
			.ok_or(Error::<B>::MissingPreRuntimeDigest("BlockSealDigest".to_string()))?,
		finalized_block: finalized_block
			.ok_or(Error::<B>::MissingPreRuntimeDigest("FinalizedBlockNeededDigest".to_string()))?,
		signature: signature.ok_or(Error::<B>::MissingSignatureDigest)?,
		block_vote: block_vote
			.ok_or(Error::<B>::MissingPreRuntimeDigest("BlockVoteDigest".to_string()))?,
		author: author.ok_or(Error::<B>::MissingPreRuntimeDigest("AuthorDigest".to_string()))?,
		next_eligibility: next_eligibility.ok_or(Error::<B>::MissingConsensusDigest)?,
		compute_authority,
	})
}
