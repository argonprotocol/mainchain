use codec::{Decode, Encode};
use sp_core::{crypto::AccountId32, U256};
use sp_runtime::{
	generic::DigestItem,
	traits::{Block as BlockT, Header as HeaderT},
	Digest,
};

use ulx_node_runtime::{AccountId, BlockNumber};
use ulx_primitives::{
	digests::{
		BlockSealMinimumsDigest, BlockVoteDigest, FinalizedBlockNeededDigest, SealSource,
		AUTHORITY_DIGEST_ID, BLOCK_VOTES_DIGEST_ID, FINALIZED_BLOCK_DIGEST_ID,
		NEXT_SEAL_MINIMUMS_DIGEST_ID,
	},
	BlockSealAuthorityId, BlockSealAuthoritySignature, BlockSealDigest, AUTHOR_DIGEST_ID,
	BLOCK_SEAL_DIGEST_ID,
};

use crate::error::Error;

pub struct Digests<B: BlockT> {
	pub finalized_block: FinalizedBlockNeededDigest<B>,
	pub authority: BlockSealAuthorityId,
	pub author: AccountId,
	pub block_vote: BlockVoteDigest,
	pub seal_minimums: BlockSealMinimumsDigest,
}

pub fn load_digests<B: BlockT>(header: &B::Header) -> Result<Digests<B>, Error<B>> {
	let mut author = None;
	let mut finalized_block = None;
	let mut authority = None;
	let mut block_vote = None;
	let mut seal_minimums = None;

	for log in header.digest().logs() {
		match log {
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
			DigestItem::PreRuntime(AUTHORITY_DIGEST_ID, v) => {
				if authority.is_some() {
					return Err(Error::DuplicatePreRuntimeDigest("AuthorityDigest".to_string()))
				}
				let digest = BlockSealAuthorityId::decode(&mut &v[..])
					.map_err(|e| Error::<B>::Codec(e.clone()))?;
				authority = Some(digest);
			},
			DigestItem::Consensus(NEXT_SEAL_MINIMUMS_DIGEST_ID, v) => {
				if seal_minimums.is_some() {
					return Err(Error::DuplicateConsensusDigest)
				}
				let digest = BlockSealMinimumsDigest::decode(&mut &v[..])
					.map_err(|e| Error::<B>::Codec(e.clone()))?;
				seal_minimums = Some(digest);
			},
			_ => {},
		}
	}

	Ok(Digests {
		finalized_block: finalized_block
			.ok_or(Error::<B>::MissingPreRuntimeDigest("FinalizedBlockNeededDigest".to_string()))?,
		block_vote: block_vote
			.ok_or(Error::<B>::MissingPreRuntimeDigest("BlockVoteDigest".to_string()))?,
		author: author.ok_or(Error::<B>::MissingPreRuntimeDigest("AuthorDigest".to_string()))?,
		seal_minimums: seal_minimums.ok_or(Error::<B>::MissingConsensusDigest)?,
		authority: authority
			.ok_or(Error::<B>::MissingPreRuntimeDigest("AuthorityDigest".to_string()))?,
	})
}

pub fn create_seal_digest(
	nonce: &U256,
	seal_source: SealSource,
	signature: BlockSealAuthoritySignature,
) -> DigestItem {
	DigestItem::Seal(
		BLOCK_SEAL_DIGEST_ID,
		BlockSealDigest { nonce: nonce.clone(), seal_source, signature }.encode(),
	)
}

pub fn read_seal_digest(digest: &DigestItem) -> Option<BlockSealDigest> {
	digest.seal_try_to(&BLOCK_SEAL_DIGEST_ID)
}

pub fn create_digests<B: BlockT>(
	author: &AccountId32,
	block_vote_digest: BlockVoteDigest,
	block_seal_authority: &BlockSealAuthorityId,
	latest_finalized_block_needed: BlockNumber,
	finalized_hash_needed: B::Hash,
) -> Digest {
	let mut inherent_digest = Digest::default();

	// add author in pow standard field (for client)
	inherent_digest.push(DigestItem::PreRuntime(AUTHOR_DIGEST_ID, author.encode()));
	inherent_digest.push(DigestItem::PreRuntime(
		FINALIZED_BLOCK_DIGEST_ID,
		FinalizedBlockNeededDigest::<B> {
			hash: finalized_hash_needed,
			number: latest_finalized_block_needed.into(),
		}
		.encode(),
	));
	inherent_digest
		.push(DigestItem::PreRuntime(AUTHORITY_DIGEST_ID, block_seal_authority.encode()));
	inherent_digest.push(DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, block_vote_digest.encode()));
	inherent_digest
}
