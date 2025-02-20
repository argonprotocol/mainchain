use crate::{aux_client::ArgonAux, block_creator::CreateTaxVoteBlock, error::Error};
use argon_primitives::{
	block_seal::BLOCK_SEAL_CRYPTO_ID,
	fork_power::ForkPower,
	tick::{Tick, Ticker},
	BlockCreatorApis, BlockSealApis, BlockSealAuthorityId, BlockSealDigest, BlockVote,
	BlockVotingPower, TickApis, VotingSchedule, BLOCK_SEAL_KEY_TYPE,
};
use argon_runtime::NotebookVerifyError;
use codec::Codec;
use log::*;
use sc_client_api::AuxStore;
use sc_utils::mpsc::TracingUnboundedSender;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{Error as ConsensusError, SelectChain};
use sp_core::{ByteArray, U256};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::traits::{Block as BlockT, Header};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};

pub struct NotebookSealer<B: BlockT, C: AuxStore, SC, AC: Clone + Codec> {
	client: Arc<C>,
	ticker: Ticker,
	select_chain: Arc<SC>,
	keystore: KeystorePtr,
	sender: TracingUnboundedSender<CreateTaxVoteBlock<B, AC>>,
	aux_client: ArgonAux<B, C>,
	_phantom: PhantomData<B>,
}

impl<B, C, SC, AC> Clone for NotebookSealer<B, C, SC, AC>
where
	B: BlockT,
	C: AuxStore + Clone,
	AC: Codec + Clone,
{
	fn clone(&self) -> Self {
		Self {
			client: self.client.clone(),
			ticker: self.ticker,
			select_chain: self.select_chain.clone(),
			keystore: self.keystore.clone(),
			sender: self.sender.clone(),
			aux_client: self.aux_client.clone(),
			_phantom: PhantomData,
		}
	}
}

impl<B, C, SC, AC> NotebookSealer<B, C, SC, AC>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + AuxStore + 'static,
	C::Api: BlockSealApis<B, AC, BlockSealAuthorityId>
		+ TickApis<B>
		+ BlockCreatorApis<B, AC, NotebookVerifyError>,
	SC: SelectChain<B> + 'static,
	AC: Codec + Clone,
{
	pub fn new(
		client: Arc<C>,
		ticker: Ticker,
		select_chain: SC,
		keystore: KeystorePtr,
		aux_client: ArgonAux<B, C>,
		sender: TracingUnboundedSender<CreateTaxVoteBlock<B, AC>>,
	) -> Self {
		Self {
			client: client.clone(),
			ticker,
			select_chain: Arc::new(select_chain),
			aux_client,
			keystore,
			sender,
			_phantom: PhantomData,
		}
	}

	pub async fn check_for_new_blocks(
		&self,
		notebook_tick: Tick,
		voting_power: BlockVotingPower,
		notebooks: u32,
	) -> Result<(), Error> {
		let current_tick = self.ticker.current();

		if current_tick <= notebook_tick {
			trace!(
				"Current tick {} is not greater than notebook tick {}",
				current_tick,
				notebook_tick
			);
			return Ok(());
		}

		// Votes only work when they're for active ticks, so no point in doing this for old ticks
		const OLDEST_TICK_TO_SOLVE_FOR: Tick = 5;

		if notebook_tick < current_tick.saturating_sub(OLDEST_TICK_TO_SOLVE_FOR) {
			trace!(
				"Notebook tick {} is too old to be considered for block creation with votes",
				notebook_tick
			);
			return Ok(());
		}

		let keys = self.keystore.ed25519_public_keys(BLOCK_SEAL_KEY_TYPE);
		if keys.is_empty() {
			trace!("No block vote keys to sign block with");
			return Ok(());
		}

		let voting_schedule = VotingSchedule::on_notebook_tick_state(notebook_tick);
		let votes_tick = voting_schedule.eligible_votes_tick();
		// get the active votes, which were from 2 notebooks previous
		let block_votes = self.aux_client.get_votes(votes_tick)?.get();
		let votes_count = block_votes.iter().fold(0u32, |acc, x| acc + x.raw_votes.len() as u32);
		if votes_count == 0 {
			trace!("No block votes at tick {}", votes_tick);
			return Ok(());
		}

		let blocks_to_build_on = self
			.get_parent_blocks_to_build_on(&voting_schedule, notebooks, voting_power)
			.await?;
		trace!( "Checking tick {} for better blocks with {} votes. Found {} blocks to attempt to build on",
			votes_tick, votes_count, blocks_to_build_on.len());

		for (block_hash, best_seal_strength) in blocks_to_build_on.into_iter() {
			// TODO: should we add more top seals to check to ensure someone matches a block?
			//   - we could delay the later ones to publish only if still best after a delay
			let Ok(stronger_seals) = self
				.client
				.runtime_api()
				.find_vote_block_seals(
					block_hash,
					block_votes.clone(),
					best_seal_strength,
					votes_tick,
				)?
				.inspect_err(|e| {
					error!("Unable to lookup vote block seals: {:?}", e);
				})
			else {
				trace!( "Could not find any stronger seals for block {:?}. Notebook tick {}, votes at tick {}. Existing power {:?}.",
					block_hash, notebook_tick, votes_tick, best_seal_strength);
				continue;
			};

			for vote in stronger_seals.into_iter() {
				let raw_authority = vote.closest_miner.1.to_raw_vec();
				if !self.keystore.has_keys(&[(raw_authority, BLOCK_SEAL_KEY_TYPE)]) {
					trace!(
						"Could not sign vote for block with seal strength {}",
						vote.seal_strength
					);
					continue;
				}

				trace!("Found vote-eligible block with seal strength {}", vote.seal_strength);
				self.sender
					.unbounded_send(CreateTaxVoteBlock::<B, AC> {
						current_tick: notebook_tick + 1,
						timestamp_millis: self.ticker.now_adjusted_to_ntp(),
						vote,
						parent_hash: block_hash,
					})
					.map_err(|e| {
						Error::StringError(format!(
							"Failed to send CreateTaxVoteBlock message: {:?}",
							e
						))
					})?;
				return Ok(());
			}
		}
		Ok(())
	}

	async fn get_parent_blocks_to_build_on(
		&self,
		voting_schedule: &VotingSchedule,
		notebooks: u32,
		voting_power: BlockVotingPower,
	) -> Result<HashMap<B::Hash, U256>, Error> {
		let leaves = self.select_chain.leaves().await?;

		let mut blocks_to_build_on = HashMap::new();
		let finalized_block = self.client.info().finalized_number;

		// Blocks are always created with a tick at least notebook tick +1, so the parent will be at
		// notebook tick
		let notebook_in_block_tick = voting_schedule.block_tick();
		let parent_tick = voting_schedule.parent_block_tick();
		for leaf in leaves {
			let Some(parent_block) = self.get_block_descendent_with_tick(leaf, parent_tick) else {
				trace!("No block at notebook parent tick {} for leaf {:?}", parent_tick, leaf);
				continue;
			};

			// blocks to beat that include notebook are tick +1
			let Some(block_hash_to_beat) =
				self.get_block_descendent_with_tick(leaf, notebook_in_block_tick)
			else {
				trace!(
					"Adding parent block (at tick {}) since no competition {:?}",
					parent_tick,
					leaf
				);
				// if not trying to beat anyone, just add the parent hash
				blocks_to_build_on.insert(parent_block, U256::MAX);
				continue;
			};

			let fork_power_to_beat = self.get_fork_power(block_hash_to_beat)?;
			let best_seal_strength = fork_power_to_beat.seal_strength;

			// pretend we beat the best vote - could we beat this fork power?
			let mut theoretical_power = self.get_fork_power(parent_block)?;
			theoretical_power.add_vote(
				voting_power,
				notebooks,
				best_seal_strength.saturating_sub(U256::one()),
			);

			// needs to be greater or we will start creating blocks to match our own and out-pace
			// the fork depth
			if theoretical_power > fork_power_to_beat {
				trace!(
					"Adding parent block (at tick {}) since we can beat the competition {:?}",
					parent_tick,
					leaf
				);
				blocks_to_build_on.insert(parent_block, best_seal_strength);
			}
		}
		blocks_to_build_on.retain(|block, strength| {
			let block_number = match self.client.header(*block) {
				Ok(Some(header)) => *header.number(),
				_ => {
					trace!("Could not get block number for block {:?}", block);
					return false
				},
			};
			// don't try to build on something older than the latest finalized block
			if block_number < finalized_block {
				trace!(
					"Block {:?} is older than finalized block {}. Not building on it",
					block,
					finalized_block
				);
				return false;
			}
			let has_eligible_votes =
				self.client.runtime_api().has_eligible_votes(*block).unwrap_or_default();
			let block_tick = self.client.runtime_api().current_tick(*block).unwrap_or_default();
			trace!(
				"Looking at parent blocks to build on. Block {:?} with strength {}. Has Votes? {}. Block Runtime Tick {}",
				block,
				strength,
				has_eligible_votes,
				block_tick
			);
			has_eligible_votes
		});

		Ok(blocks_to_build_on)
	}

	fn get_block_descendent_with_tick(&self, hash: B::Hash, tick: Tick) -> Option<B::Hash> {
		// first check this because `block_at_tick` can't include a block until it's a parent block
		if let Ok(current_tick) = self.client.runtime_api().current_tick(hash) {
			if current_tick == tick {
				return Some(hash);
			}
		}
		if let Ok(blocks) = self.client.runtime_api().blocks_at_tick(hash, tick) {
			return blocks.last().copied();
		}
		None
	}

	fn get_fork_power(&self, block_hash: B::Hash) -> Result<ForkPower, Error> {
		let header = self.client.header(block_hash)?.ok_or_else(|| {
			Error::StringError(format!("Could not find header for block {:?}", block_hash))
		})?;
		let digest = header.digest();
		ForkPower::try_from(digest).map_err(|e| {
			Error::StringError(format!("Could not get fork power from header: {:?}", e))
		})
	}
}

pub fn create_vote_seal<Hash: AsRef<[u8]>>(
	keystore: &KeystorePtr,
	pre_hash: &Hash,
	vote_authority: &BlockSealAuthorityId,
	seal_strength: U256,
) -> Result<BlockSealDigest, Error> {
	let message = BlockVote::seal_signature_message(pre_hash);
	let signature = keystore
		.sign_with(BLOCK_SEAL_KEY_TYPE, BLOCK_SEAL_CRYPTO_ID, vote_authority.as_slice(), &message)
		.map_err(|e| ConsensusError::CannotSign(format!("{}. Key: {:?}", e, vote_authority)))?
		.ok_or_else(|| {
			ConsensusError::CannotSign(format!(
				"Could not find key in keystore. Key: {:?}",
				vote_authority
			))
		})?;

	let signature = signature
		.clone()
		.try_into()
		.map_err(|_| ConsensusError::InvalidSignature(signature, vote_authority.to_raw_vec()))?;

	Ok(BlockSealDigest::Vote { seal_strength, signature })
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;
	use sp_core::H256;
	use sp_keyring::Ed25519Keyring;
	use sp_keystore::{testing::MemoryKeystore, Keystore};

	use argon_primitives::block_seal::BLOCK_SEAL_KEY_TYPE;

	use crate::mock_notary::setup_logs;

	use super::*;

	fn create_keystore(authority: Ed25519Keyring) -> KeystorePtr {
		let keystore = MemoryKeystore::new();
		keystore
			.ed25519_generate_new(BLOCK_SEAL_KEY_TYPE, Some(&authority.to_seed()))
			.expect("Creates authority key");
		keystore.into()
	}

	#[test]
	fn it_can_sign_a_vote() {
		setup_logs();
		let keystore = create_keystore(Ed25519Keyring::Alice);

		assert_ok!(create_vote_seal(
			&keystore,
			&H256::from_slice(&[2u8; 32]),
			&Ed25519Keyring::Alice.public().into(),
			U256::from(1)
		));
	}

	#[test]
	fn it_fails_if_not_installed() {
		setup_logs();
		let keystore = create_keystore(Ed25519Keyring::Alice);

		let block_hash = H256::from([31; 32]);
		let nonce = U256::from(1);

		assert!(matches!(
			create_vote_seal(&keystore, &block_hash, &Ed25519Keyring::Bob.public().into(), nonce),
			Err(Error::ConsensusError(ConsensusError::CannotSign(_)))
		),);
	}
}
