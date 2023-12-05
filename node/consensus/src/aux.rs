use std::{
	cmp::Ordering,
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
	sync::Arc,
};

use codec::{Decode, Encode};
use sc_client_api::{self, backend::AuxStore};
use sc_consensus::BlockImportParams;
use sp_core::{H256, U256, U512};
use sp_runtime::traits::{Block as BlockT, Header};

use ulx_node_runtime::{AccountId, BlockNumber};
use ulx_primitives::{block_seal::BlockVotingPower, localchain::BlockVoteT, NotaryId};

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
		notaries: u32,
		block_voting_power: BlockVotingPower,
		nonce: U256,
		is_tax_vote: bool,
	) -> Result<(ForkPower, ForkPower), Error<B>> {
		let block_number = convert_u32::<B>(&block.header.number());
		let strongest_at_height = Self::strongest_vote_at_height(client.as_ref(), block_number)?;

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
		let fork_voting_power = Self::get_fork_voting_power(client.as_ref(), &parent_hash)?;
		let voting_power =
			fork_voting_power.voting_power.saturating_add(U256::from(block_voting_power));

		let aggregate_nonce = fork_voting_power.aggregate_nonce.saturating_add(nonce.into());
		let mut tax_blocks = fork_voting_power.tax_blocks;
		if is_tax_vote {
			tax_blocks = tax_blocks.saturating_add(1);
		}
		let fork_power = ForkPower { voting_power, notaries, nonce, aggregate_nonce, tax_blocks };
		if voting_power >= strongest_at_height.voting_power &&
			aggregate_nonce < strongest_at_height.aggregate_nonce
		{
			let key = get_strongest_vote_at_height_key(block_number);
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
			block.auxiliary.push((get_strongest_vote_at_height_key(cleanup_height), None));
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

	pub fn strongest_vote_at_height(
		client: &C,
		block_number: BlockNumber,
	) -> Result<ForkPower, Error<B>> {
		let state_key = get_strongest_vote_at_height_key(block_number);
		let aux = match client.get_aux(&state_key)? {
			Some(bytes) => ForkPower::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		};
		Ok(aux)
	}

	pub fn get_block_votes(
		client: &C,
		block_number: BlockNumber,
	) -> Result<BTreeMap<(NotaryId, AccountId, u32), BlockVoteT<B::Hash>>, Error<B>> {
		let state_key = get_votes_key(block_number);
		let aux = match client.get_aux(&state_key)? {
			Some(bytes) => BTreeMap::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		};
		Ok(aux)
	}

	pub fn store_vote(
		client: &C,
		notary_id: NotaryId,
		block_vote: BlockVoteT<B::Hash>,
		block_number: BlockNumber,
	) -> Result<(), Error<B>> {
		let mut block_votes = Self::get_block_votes(client, block_number)?;
		let key = (notary_id, block_vote.account_id.clone(), block_vote.index);
		if !block_votes.contains_key(&key) {
			block_votes.insert(key, block_vote);
			client.insert_aux(
				&[(get_votes_key(block_number).as_slice(), block_votes.encode().as_slice())],
				None,
			)?;
		}
		Ok(())
	}
}

fn get_authors_at_height_key(block_number: BlockNumber) -> Vec<u8> {
	("AuthorsAtHeight", block_number).encode()
}
fn get_votes_key(block_number: BlockNumber) -> Vec<u8> {
	("VotesAtHeight", block_number).encode()
}
fn get_fork_voting_power_aux_key<B: BlockT>(block_hash: &B::Hash) -> Vec<u8> {
	("ForkVotingPower", block_hash).encode()
}
fn get_strongest_vote_at_height_key(block_number: BlockNumber) -> Vec<u8> {
	("MaxVotingPowerAtHeight", block_number).encode()
}

#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq)]
pub struct ForkPower {
	pub notaries: u32,
	pub voting_power: U256,
	pub nonce: U256,
	pub aggregate_nonce: U512,
	pub tax_blocks: u128,
}

impl Default for ForkPower {
	fn default() -> Self {
		Self {
			voting_power: U256::zero(),
			notaries: 0,
			nonce: U256::MAX,
			aggregate_nonce: U512::MAX,
			tax_blocks: 0,
		}
	}
}

impl PartialOrd for ForkPower {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		let mut cmp = other.notaries.cmp(&self.notaries);
		if cmp == Ordering::Equal {
			cmp = other.voting_power.cmp(&self.voting_power);
		}
		if cmp == Ordering::Equal {
			// count forks with tax votes over compute
			cmp = other.tax_blocks.cmp(&self.tax_blocks);
		}
		if cmp == Ordering::Equal {
			// NOTE: Smallest first!
			cmp = self.aggregate_nonce.cmp(&other.aggregate_nonce)
		}
		Some(cmp)
	}
}
