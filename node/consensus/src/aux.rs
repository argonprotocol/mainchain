use std::{
	cmp::Ordering,
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
	sync::Arc,
};

use codec::{Decode, Encode};
use sc_client_api::{self, backend::AuxStore};
use sc_consensus::BlockImportParams;
use sp_core::{H256, U256};
use sp_runtime::traits::{Block as BlockT, Header};

use ulx_node_runtime::{AccountId, BlockNumber};
use ulx_primitives::{
	block_seal::BlockVotingPower, tick::Tick, BlockSealDigest, ComputeDifficulty, NotaryId,
	NotebookVotes,
};

use crate::{convert_u32, error::Error};

pub struct UlxAux<C: AuxStore, B: BlockT> {
	_block: std::marker::PhantomData<(C, B)>,
}

///
/// Stores auxiliary data for Ulx consensus (eg - cross block data)
///
/// We store several types of data
/// - `ForkPower` - stored at each block to determine the aggregate voting power for a fork
///   (++voting_power, --nonce)
/// - `BlockVotes` - all block votes submitted (voting for a block hash)
/// - `StrongestVoteAtHeight` - the strongest vote at a given height - helps determine if we should
///   create a block
/// - `AuthorsAtHeight` - the authors at a given height for every voting key. A block will only be
///   accepted once per author per key
impl<C: AuxStore, B: BlockT> UlxAux<C, B> {
	pub fn record_block(
		client: &Arc<C>,
		best_header: B::Header,
		block: &mut BlockImportParams<B>,
		author: AccountId,
		voting_key: Option<H256>,
		notebooks: u32,
		tick: Tick,
		block_voting_power: BlockVotingPower,
		seal_digest: BlockSealDigest,
		compute_difficulty: Option<ComputeDifficulty>,
	) -> Result<(ForkPower, ForkPower), Error<B>> {
		let block_number = convert_u32::<B>(&block.header.number());
		let strongest_at_height = Self::strongest_fork_at_tick(client.as_ref(), tick)?;

		let mut authors_at_height =
			Self::authors_by_voting_key_at_height(client.as_ref(), block_number)?;
		// add author to voting key
		if let Some(voting_key) = voting_key {
			let is_new_entry = authors_at_height
				.entry(voting_key)
				.or_insert_with(BTreeSet::new)
				.insert(author.clone());
			if !is_new_entry {
				return Err(Error::<B>::DuplicateAuthoredBlock(author).into())
			}
		}

		let parent_hash = block.header.parent_hash();
		let mut fork_power = Self::get_fork_voting_power(client.as_ref(), &parent_hash)?;
		fork_power.add(block_voting_power, notebooks, seal_digest, compute_difficulty);

		if fork_power > strongest_at_height {
			let key = get_strongest_fork_at_tick_key(tick);
			block.auxiliary.push((key, Some(fork_power.encode())));
		}

		let best_header_fork_power =
			Self::get_fork_voting_power(client.as_ref(), &best_header.hash())?;

		block.auxiliary.push((
			get_fork_voting_power_aux_key::<B>(&block.post_hash()),
			Some(fork_power.encode()),
		));

		// cleanup old votes (None deletes)
		if block_number > 5 {
			let cleanup_height = block_number - 5;
			block.auxiliary.push((get_votes_key(cleanup_height), None));
			block.auxiliary.push((get_strongest_fork_at_tick_key(cleanup_height), None));
			block.auxiliary.push((get_authors_at_height_key(cleanup_height), None));
		}

		Ok((fork_power, best_header_fork_power))
	}

	pub fn authors_by_voting_key_at_height(
		client: &C,
		block_number: BlockNumber,
	) -> Result<BTreeMap<H256, BTreeSet<AccountId>>, Error<B>> {
		let state_key = get_authors_at_height_key(block_number);
		let aux = match client.get_aux(&state_key)? {
			Some(bytes) => BTreeMap::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		};
		Ok(aux)
	}

	/// Retrieves aggregate voting power for a fork
	pub fn get_fork_voting_power(client: &C, block_hash: &B::Hash) -> Result<ForkPower, Error<B>> {
		let key = get_fork_voting_power_aux_key::<B>(block_hash);
		let aux = match client.get_aux(&key)? {
			Some(bytes) => ForkPower::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		};
		Ok(aux)
	}

	pub fn store_votes(
		client: &C,
		tick: Tick,
		notary_id: NotaryId,
		votes: NotebookVotes,
	) -> Result<BTreeMap<NotaryId, NotebookVotes>, Error<B>> {
		let mut existing = Self::get_votes(client, tick)?;
		existing.insert(notary_id, votes);

		let key = get_votes_key(tick);
		client.insert_aux(&[(key.as_slice(), existing.encode().as_slice())], &[])?;
		Ok(existing)
	}

	pub fn get_votes(
		client: &C,
		tick: Tick,
	) -> Result<BTreeMap<NotaryId, NotebookVotes>, Error<B>> {
		let key = get_votes_key(tick);
		Ok(match client.get_aux(&key)? {
			Some(bytes) => BTreeMap::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		})
	}

	pub fn strongest_fork_at_tick(client: &C, tick: Tick) -> Result<ForkPower, Error<B>> {
		let state_key = get_strongest_fork_at_tick_key(tick);
		let aux = match client.get_aux(&state_key)? {
			Some(bytes) => ForkPower::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		};
		Ok(aux)
	}
}

fn get_authors_at_height_key(block_number: BlockNumber) -> Vec<u8> {
	("AuthorsAtHeight", block_number).encode()
}
fn get_votes_key(tick: Tick) -> Vec<u8> {
	("VotesAtTick", tick).encode()
}
fn get_fork_voting_power_aux_key<B: BlockT>(block_hash: &B::Hash) -> Vec<u8> {
	("ForkVotingPower", block_hash).encode()
}
fn get_strongest_fork_at_tick_key(tick: Tick) -> Vec<u8> {
	("MaxVotingPowerAtTick", tick).encode()
}

#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq)]
pub struct ForkPower {
	pub notebooks: u64,
	pub voting_power: U256,
	pub vote_proof: U256,
	pub total_compute_difficulty: U256,
	pub vote_created_blocks: u128,
}

impl ForkPower {
	pub fn add(
		&mut self,
		block_voting_power: BlockVotingPower,
		notebooks: u32,
		seal_digest: BlockSealDigest,
		compute_difficulty: Option<ComputeDifficulty>,
	) {
		match seal_digest {
			BlockSealDigest::Vote { vote_proof } => {
				self.add_vote(block_voting_power, notebooks, vote_proof);
			},
			BlockSealDigest::Compute { .. } => {
				self.add_compute(
					block_voting_power,
					notebooks,
					compute_difficulty.unwrap_or_default(),
				);
			},
		}
	}

	pub fn add_vote(
		&mut self,
		block_voting_power: BlockVotingPower,
		notebooks: u32,
		vote_proof: U256,
	) {
		self.vote_proof = self.vote_proof.saturating_add(vote_proof);
		self.vote_created_blocks = self.vote_created_blocks.saturating_add(1);
		self.voting_power = self.voting_power.saturating_add(U256::from(block_voting_power));
		self.notebooks = self.notebooks.saturating_add(notebooks as u64);
	}

	pub fn add_compute(
		&mut self,
		block_voting_power: BlockVotingPower,
		notebooks: u32,
		compute_difficulty: ComputeDifficulty,
	) {
		self.voting_power = self.voting_power.saturating_add(U256::from(block_voting_power));
		self.notebooks = self.notebooks.saturating_add(notebooks as u64);
		self.total_compute_difficulty =
			self.total_compute_difficulty.saturating_add(compute_difficulty.into());
	}
}

impl Default for ForkPower {
	fn default() -> Self {
		Self {
			voting_power: U256::zero(),
			notebooks: 0,
			vote_proof: U256::MAX,
			total_compute_difficulty: U256::zero(),
			vote_created_blocks: 0,
		}
	}
}

impl PartialOrd for ForkPower {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		let mut cmp = other.notebooks.cmp(&self.notebooks);
		if cmp == Ordering::Equal {
			cmp = other.voting_power.cmp(&self.voting_power);
		}
		if cmp == Ordering::Equal {
			// count forks with tax votes over compute
			cmp = other.vote_created_blocks.cmp(&self.vote_created_blocks);
		}
		if cmp == Ordering::Equal {
			// smaller vote proof is better
			cmp = self.vote_proof.cmp(&other.vote_proof)
		}
		if cmp == Ordering::Equal {
			cmp = other.total_compute_difficulty.cmp(&self.total_compute_difficulty)
		}
		Some(cmp)
	}
}
