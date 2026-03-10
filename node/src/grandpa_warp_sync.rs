use codec::{DecodeAll, Encode};
use polkadot_sdk::*;
use sc_client_api::Backend as ClientBackend;
use sc_consensus_grandpa::{
	AuthoritySetChanges, AuthoritySetHardFork, BlockNumberOps, GrandpaJustification,
	SharedAuthoritySet, best_justification, find_scheduled_change,
	warp_proof::{Error as WarpProofError, NetworkProvider, WarpSyncFragment},
};
use sc_network_sync::strategy::warp::{EncodedProof, WarpSyncProvider};
use sp_blockchain::{Backend as BlockchainBackend, HeaderBackend};
use sp_consensus_grandpa::{AuthorityList, GRANDPA_ENGINE_ID, SetId};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, NumberFor, One},
};
use std::{collections::BTreeMap, error::Error as StdError, sync::Arc};

// This must stay aligned with upstream GRANDPA warp proof generation.
const MAX_WARP_SYNC_PROOF_SIZE: usize = 8 * 1024 * 1024;
// This matches upstream's small slack for SCALE-encoding the outer proof wrapper.
const WARP_SYNC_PROOF_ENCODING_SLACK: usize = 50;

pub struct ArgonWarpSyncProvider<Block: BlockT, Backend: ClientBackend<Block>>
where
	NumberFor<Block>: BlockNumberOps,
{
	backend: Arc<Backend>,
	authority_set: SharedAuthoritySet<Block::Hash, NumberFor<Block>>,
	inner: NetworkProvider<Block, Backend>,
	hard_fork_hash_by_block_number: BTreeMap<NumberFor<Block>, Block::Hash>,
}

impl<Block: BlockT, Backend: ClientBackend<Block>> ArgonWarpSyncProvider<Block, Backend>
where
	NumberFor<Block>: BlockNumberOps,
{
	pub fn new(
		backend: Arc<Backend>,
		authority_set: SharedAuthoritySet<Block::Hash, NumberFor<Block>>,
		hard_forks: Vec<AuthoritySetHardFork<Block>>,
	) -> Self {
		let hard_fork_hash_by_block_number = hard_forks
			.iter()
			// `AuthoritySetHardFork::block` is `(hash, number)`, but warp generation
			// looks hard forks up by the boundary block number.
			.map(|fork| (fork.block.1, fork.block.0))
			.collect::<BTreeMap<_, _>>();
		let inner = NetworkProvider::new(backend.clone(), authority_set.clone(), hard_forks);

		Self { backend, authority_set, inner, hard_fork_hash_by_block_number }
	}
}

impl<Block: BlockT, Backend: ClientBackend<Block>> WarpSyncProvider<Block>
	for ArgonWarpSyncProvider<Block, Backend>
where
	NumberFor<Block>: BlockNumberOps,
{
	fn generate(
		&self,
		start: Block::Hash,
	) -> Result<EncodedProof, Box<dyn StdError + Send + Sync>> {
		let proof = generate_warp_sync_proof(
			&*self.backend,
			start,
			&self.authority_set.authority_set_changes(),
			&self.hard_fork_hash_by_block_number,
		)
		.map_err(Box::new)?;
		Ok(EncodedProof(proof.encode()))
	}

	fn verify(
		&self,
		proof: &EncodedProof,
		set_id: SetId,
		authorities: AuthorityList,
	) -> Result<
		sc_network_sync::strategy::warp::VerificationResult<Block>,
		Box<dyn StdError + Send + Sync>,
	> {
		self.inner.verify(proof, set_id, authorities)
	}

	fn current_authorities(&self) -> AuthorityList {
		self.inner.current_authorities()
	}
}

#[derive(Encode)]
struct ArgonWarpSyncProof<Block: BlockT> {
	proofs: Vec<WarpSyncFragment<Block>>,
	is_finished: bool,
}

fn generate_warp_sync_proof<Backend, Block: BlockT>(
	backend: &Backend,
	begin: Block::Hash,
	set_changes: &AuthoritySetChanges<NumberFor<Block>>,
	hard_fork_hash_by_block_number: &BTreeMap<NumberFor<Block>, Block::Hash>,
) -> Result<ArgonWarpSyncProof<Block>, WarpProofError>
where
	Backend: ClientBackend<Block>,
	NumberFor<Block>: BlockNumberOps,
{
	let blockchain = backend.blockchain();

	let begin_number = blockchain
		.block_number_from_id(&BlockId::Hash(begin))?
		.ok_or_else(|| WarpProofError::InvalidRequest("Missing start block".to_string()))?;

	if begin_number > blockchain.info().finalized_number {
		return Err(WarpProofError::InvalidRequest("Start block is not finalized".to_string()))
	}

	let canon_hash = blockchain.hash(begin_number)?.ok_or_else(|| {
		WarpProofError::InvalidRequest("Missing canonical hash for start block".to_string())
	})?;
	if canon_hash != begin {
		return Err(WarpProofError::InvalidRequest(
			"Start block is not in the finalized chain".to_string(),
		))
	}

	let mut proofs = Vec::new();
	let mut proofs_encoded_len = 0;
	let mut proof_limit_reached = false;

	let set_changes = set_changes.iter_from(begin_number).ok_or(WarpProofError::MissingData)?;
	for (_, last_block) in set_changes {
		let Some(proof) =
			find_warp_fragment(blockchain, *last_block, hard_fork_hash_by_block_number)?
		else {
			break;
		};
		let proof_size = proof.encoded_size();

		if proofs_encoded_len + proof_size >=
			MAX_WARP_SYNC_PROOF_SIZE - WARP_SYNC_PROOF_ENCODING_SLACK
		{
			proof_limit_reached = true;
			break
		}

		proofs_encoded_len += proof_size;
		proofs.push(proof);
	}

	let is_finished = if proof_limit_reached {
		false
	} else {
		let latest_justification = best_justification(backend)?.filter(|justification| {
			let limit = proofs
				.last()
				.map(|proof| proof.justification.target().0 + One::one())
				.unwrap_or(begin_number);

			justification.target().0 >= limit
		});

		if let Some(latest_justification) = latest_justification {
			let header = blockchain
				.header(latest_justification.target().1)?
				.ok_or(WarpProofError::MissingData)?;
			let proof = WarpSyncFragment { header, justification: latest_justification };

			if proofs_encoded_len + proof.encoded_size() >=
				MAX_WARP_SYNC_PROOF_SIZE - WARP_SYNC_PROOF_ENCODING_SLACK
			{
				false
			} else {
				proofs.push(proof);
				true
			}
		} else {
			true
		}
	};

	let final_outcome = ArgonWarpSyncProof { proofs, is_finished };
	debug_assert!(final_outcome.encoded_size() <= MAX_WARP_SYNC_PROOF_SIZE);
	Ok(final_outcome)
}

fn find_warp_fragment<Backend, Block: BlockT>(
	blockchain: &Backend,
	last_block: NumberFor<Block>,
	hard_fork_hash_by_block_number: &BTreeMap<NumberFor<Block>, Block::Hash>,
) -> Result<Option<WarpSyncFragment<Block>>, WarpProofError>
where
	Backend: BlockchainBackend<Block>,
	NumberFor<Block>: BlockNumberOps,
{
	let hash = blockchain
		.block_hash_from_id(&BlockId::Number(last_block))?
		.ok_or(WarpProofError::MissingData)?;
	let header = blockchain.header(hash)?.ok_or(WarpProofError::MissingData)?;

	if !is_authority_boundary::<Block>(&header, hash, last_block, hard_fork_hash_by_block_number)? {
		return Ok(None)
	}

	let justification = blockchain
		.justifications(hash)?
		.and_then(|just| just.into_justification(GRANDPA_ENGINE_ID))
		.ok_or(WarpProofError::MissingData)?;
	let justification = GrandpaJustification::<Block>::decode_all(&mut &justification[..])?;

	Ok(Some(WarpSyncFragment { header, justification }))
}

fn is_authority_boundary<Block: BlockT>(
	header: &Block::Header,
	hash: Block::Hash,
	number: NumberFor<Block>,
	hard_fork_hash_by_block_number: &BTreeMap<NumberFor<Block>, Block::Hash>,
) -> Result<bool, WarpProofError>
where
	NumberFor<Block>: BlockNumberOps,
{
	if let Some(expected_hash) = hard_fork_hash_by_block_number.get(&number) {
		if *expected_hash != hash {
			return Err(WarpProofError::InvalidRequest(format!(
				"Configured GRANDPA hard fork does not match the canonical chain at block {number:?}: expected hash {expected_hash:?}, canonical hash {hash:?}",
			)))
		}
		return Ok(true)
	}

	Ok(find_scheduled_change::<Block>(header).is_some())
}

#[cfg(test)]
mod tests {
	use super::{GRANDPA_ENGINE_ID, is_authority_boundary};
	use codec::Encode;
	use polkadot_sdk::*;
	use sp_consensus_grandpa::{ConsensusLog, ScheduledChange};
	use sp_core::H256;
	use sp_keyring::Ed25519Keyring;
	use sp_runtime::{
		generic::DigestItem,
		testing::{Block as TestBlock, Header as TestHeader},
	};
	use std::collections::BTreeMap;

	type Block = TestBlock<sp_runtime::OpaqueExtrinsic>;
	type Number = sp_runtime::traits::NumberFor<Block>;

	#[test]
	fn boundary_block_uses_scheduled_change_digest() {
		let header = header(2, H256::default(), true);
		let hash = header.hash();

		assert!(is_authority_boundary::<Block>(&header, hash, 2, &BTreeMap::new()).unwrap());
	}

	#[test]
	fn boundary_block_uses_hard_fork_when_digest_is_missing() {
		let header = header(17_573, H256::default(), false);
		let hash = header.hash();
		let hard_forks = BTreeMap::from([(17_573u64, hash)]);

		assert!(is_authority_boundary::<Block>(&header, hash, 17_573, &hard_forks).unwrap());
	}

	#[test]
	fn boundary_block_rejects_mismatched_hard_fork_hash() {
		let header = header(17_573, H256::default(), false);
		let hard_forks = BTreeMap::from([(17_573u64, H256::repeat_byte(7))]);

		assert!(
			is_authority_boundary::<Block>(&header, header.hash(), 17_573, &hard_forks).is_err()
		);
	}

	#[test]
	fn non_boundary_block_without_hard_fork_is_skipped() {
		let header = header(5, H256::default(), false);

		assert!(
			!is_authority_boundary::<Block>(&header, header.hash(), 5, &BTreeMap::new()).unwrap()
		);
	}

	fn header(number: Number, parent_hash: H256, has_scheduled_change: bool) -> TestHeader {
		let mut header = TestHeader::new_from_number(number);
		header.parent_hash = parent_hash;
		if has_scheduled_change {
			header.digest.push(scheduled_change_digest());
		}
		header
	}

	fn scheduled_change_digest() -> DigestItem {
		let next_authorities = vec![(Ed25519Keyring::Alice.public().into(), 1)];
		DigestItem::Consensus(
			GRANDPA_ENGINE_ID,
			ConsensusLog::ScheduledChange(ScheduledChange { next_authorities, delay: 0u64 })
				.encode(),
		)
	}
}
