use codec::{Decode, Encode};
use sp_core::crypto::AccountId32;
use sp_runtime::{
	generic::DigestItem,
	traits::{Block as BlockT, Header as HeaderT},
	Digest,
};
use ulx_node_runtime::AccountId;

use ulx_primitives::{
	digests::{
		BlockVoteDigest, FinalizedBlockNeededDigest, BLOCK_VOTES_DIGEST_ID,
		FINALIZED_BLOCK_DIGEST_ID, TICK_DIGEST_ID,
	},
	tick::Tick,
	BlockSealDigest, AUTHOR_DIGEST_ID, BLOCK_SEAL_DIGEST_ID,
};

use crate::error::Error;

pub struct Digests<B: BlockT> {
	pub finalized_block: FinalizedBlockNeededDigest<B>,
	pub author: AccountId,
	pub block_vote: BlockVoteDigest,
	pub tick: Tick,
}

pub fn load_digests<B: BlockT>(header: &B::Header) -> Result<Digests<B>, Error<B>> {
	let mut author = None;
	let mut finalized_block = None;
	let mut block_vote = None;
	let mut tick = None;

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
			DigestItem::PreRuntime(TICK_DIGEST_ID, v) => {
				if tick.is_some() {
					return Err(Error::DuplicatePreRuntimeDigest("TickDigest".to_string()))
				}
				let digest = Tick::decode(&mut &v[..]).map_err(|e| Error::<B>::Codec(e.clone()))?;
				tick = Some(digest);
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
		tick: tick.ok_or(Error::<B>::MissingPreRuntimeDigest("TickDigest".to_string()))?,
	})
}

pub fn create_seal_digest(block_seal_digest: &BlockSealDigest) -> DigestItem {
	DigestItem::Seal(BLOCK_SEAL_DIGEST_ID, block_seal_digest.encode())
}

pub fn read_seal_digest(digest: &DigestItem) -> Option<BlockSealDigest> {
	digest.seal_try_to(&BLOCK_SEAL_DIGEST_ID)
}

pub fn get_tick_digest(digest: &Digest) -> Option<Tick> {
	for log in digest.logs() {
		if let Some(tick) = log.pre_runtime_try_to(&TICK_DIGEST_ID) {
			return Some(tick)
		}
	}
	None
}

pub fn create_digests<B: BlockT>(
	author: AccountId32,
	tick: Tick,
	block_vote_digest: BlockVoteDigest,
	finalized_block_needed_digest: FinalizedBlockNeededDigest<B>,
) -> Digest {
	let mut inherent_digest = Digest::default();

	// add author in pow standard field (for client)
	inherent_digest.push(DigestItem::PreRuntime(AUTHOR_DIGEST_ID, author.encode()));
	inherent_digest.push(DigestItem::PreRuntime(TICK_DIGEST_ID, tick.encode()));
	inherent_digest.push(DigestItem::PreRuntime(
		FINALIZED_BLOCK_DIGEST_ID,
		finalized_block_needed_digest.encode(),
	));
	inherent_digest.push(DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, block_vote_digest.encode()));
	inherent_digest
}
