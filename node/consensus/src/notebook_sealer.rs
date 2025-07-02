use crate::{
	NotebookTickChecker, aux_client::ArgonAux, block_creator::CreateTaxVoteBlock, error::Error,
};
use argon_primitives::{
	AccountId, BestBlockVoteSeal, BlockCreatorApis, BlockSealApis, BlockSealAuthorityId,
	BlockSealDigest, BlockVote, TickApis, VotingSchedule,
	block_seal::{BLOCK_SEAL_CRYPTO_ID, BLOCK_SEAL_KEY_TYPE},
	fork_power::ForkPower,
	notary::NotaryNotebookRawVotes,
	prelude::sp_api::{Core, RuntimeApiInfo},
	tick::{Tick, Ticker},
};
use argon_runtime::NotebookVerifyError;
use codec::Codec;
use log::*;
use polkadot_sdk::*;
use sc_client_api::AuxStore;
use sc_utils::mpsc::TracingUnboundedSender;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{Error as ConsensusError, SelectChain};
use sp_core::{ByteArray, U256};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::traits::{Block as BlockT, Header};
use std::{marker::PhantomData, sync::Arc};
use tokio::time::Instant;

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

#[derive(Debug, Default, Clone)]
pub struct CheckForNotebookResult {
	pub found_block: bool,
	pub recheck_notebook_tick_time: Option<Instant>,
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
	) -> Result<CheckForNotebookResult, Error> {
		let current_clocktime_tick = self.ticker.current();
		let mut result = CheckForNotebookResult::default();

		if current_clocktime_tick <= notebook_tick {
			tracing::trace!(
				current_clocktime_tick,
				notebook_tick,
				"Clock-time current tick is before notebook tick",
			);
			return Ok(result);
		}

		// Votes only work when they're for active ticks, so no point in doing this for old ticks
		const OLDEST_TICK_TO_SOLVE_FOR: Tick = 5;

		if notebook_tick < current_clocktime_tick.saturating_sub(OLDEST_TICK_TO_SOLVE_FOR) {
			trace!(
				"Notebook tick {} is too old to be considered for block creation with votes",
				notebook_tick
			);
			return Ok(result);
		}

		let keys: Vec<BlockSealAuthorityId> = self
			.keystore
			.ed25519_public_keys(BLOCK_SEAL_KEY_TYPE)
			.into_iter()
			.map(Into::into)
			.collect::<Vec<_>>();
		if keys.is_empty() {
			trace!("No block vote keys to sign block with");
			return Ok(result);
		}

		let voting_schedule = VotingSchedule::on_notebook_tick_state(notebook_tick);
		let votes_tick = voting_schedule.eligible_votes_tick();
		// get the active votes, which were from 2 notebooks previous
		let block_votes = self.aux_client.get_votes(votes_tick)?.get();
		let votes_count = block_votes.iter().fold(0u32, |acc, x| acc + x.raw_votes.len() as u32);
		if votes_count == 0 {
			trace!("No block votes at tick {}", votes_tick);
			return Ok(result);
		}

		let finalized_hash = self.client.info().finalized_hash;
		let finalized_tick = self.client.runtime_api().current_tick(finalized_hash)?;
		if voting_schedule.block_tick() <= finalized_tick {
			tracing::trace!(
				notebook_tick,
				finalized_tick,
				include_in_block_tick = ?voting_schedule.block_tick(),
				"Block tick for received notebook has already been finalized, no need to seal block",
			);
			return Ok(result);
		}

		let Some((block_hash, seal_strength, xor_distance)) =
			self.get_best_block_at_parent(&voting_schedule).await?
		else {
			trace!("No parent block to build on for tick {}", votes_tick);
			return Ok(result);
		};
		tracing::trace!(
			votes_tick,
			votes_count,
			block_hash = ?block_hash,
			seal_strength = ?seal_strength,
			xor_distance = ?xor_distance,
			"Building vote seal on block",
		);

		let api_version = self.client.runtime_api().version(block_hash)?;
		let block_seal_version = api_version
			.api_version(&<dyn BlockSealApis<B, AccountId, BlockSealAuthorityId>>::ID)
			.unwrap_or_default();

		// TODO: remove v1 once we have safely migrated to v2 (July 2025)
		let seal = if block_seal_version == 1 {
			self.check_v1_seals(&block_votes, block_hash, seal_strength, votes_tick).await?
		} else {
			self.check_v2_seals(
				&block_votes,
				keys,
				block_hash,
				seal_strength,
				xor_distance,
				votes_tick,
			)
			.await?
		};
		if let Some(vote_seal) = seal {
			tracing::trace!(block = ?block_hash, strength = ?vote_seal.seal_strength, miner_xor_distance = ?vote_seal.miner_xor_distance,
				"Found vote-eligible block");
			let block_tick = voting_schedule.block_tick();
			if let Some(recheck_at) = NotebookTickChecker::should_delay_block_attempt(
				block_tick,
				&self.ticker,
				vote_seal.miner_xor_distance,
			) {
				result.recheck_notebook_tick_time = Some(recheck_at);
				return Ok(result);
			}
			self.sender
				.unbounded_send(CreateTaxVoteBlock::<B, AC> {
					current_tick: block_tick,
					timestamp_millis: self.ticker.now_adjusted_to_ntp(),
					vote: vote_seal,
					parent_hash: block_hash,
				})
				.map_err(|e| {
					Error::StringError(format!(
						"Failed to send CreateTaxVoteBlockV2 message: {:?}",
						e
					))
				})?;
			result.found_block = true;
			return Ok(result);
		}

		tracing::trace!(
			block_hash = ?block_hash, notebook_tick, votes_tick, best_seal_strength = ?seal_strength, best_xor = ?xor_distance,
			"Could not find any stronger seals for block",
		);
		Ok(result)
	}

	async fn check_v2_seals(
		&self,
		block_votes: &[NotaryNotebookRawVotes],
		keys: Vec<BlockSealAuthorityId>,
		block_hash: B::Hash,
		mut best_seal_strength: U256,
		mut best_xor_distance: Option<U256>,
		votes_tick: Tick,
	) -> Result<Option<BestBlockVoteSeal<AC, BlockSealAuthorityId>>, Error> {
		let mut best_block_vote_seal = None;

		for key in keys {
			match self.client.runtime_api().find_better_vote_block_seal(
				block_hash,
				block_votes.to_owned(),
				best_seal_strength,
				best_xor_distance.unwrap_or(U256::MAX),
				key,
				votes_tick,
			) {
				Ok(Ok(Some(strongest))) => {
					best_seal_strength = strongest.seal_strength;
					best_xor_distance = strongest.miner_xor_distance.map(|(d, _)| d);
					best_block_vote_seal = Some(strongest.clone());
				},
				Err(e) => {
					tracing::error!(err = ?e, block=?block_hash, "Unable to call vote block seals v2");
				},
				Ok(Err(e)) => {
					tracing::error!(err = ?e, block=?block_hash, "Unable to lookup vote block seals v2");
				},
				_ => {},
			}
		}
		Ok(best_block_vote_seal)
	}

	async fn check_v1_seals(
		&self,
		block_votes: &[NotaryNotebookRawVotes],
		block_hash: B::Hash,
		best_seal_strength: U256,
		votes_tick: Tick,
	) -> Result<Option<BestBlockVoteSeal<AC, BlockSealAuthorityId>>, Error> {
		let stronger_seals = self
			.client
			.runtime_api()
			.find_vote_block_seals(
				block_hash,
				block_votes.to_owned(),
				best_seal_strength,
				votes_tick,
			)
			.inspect_err(|e| {
				tracing::error!(err = ?e, block=?block_hash, "Unable to call vote block seals");
			})?
			.inspect_err(|e| {
				tracing::error!(err = ?e, block=?block_hash, "Unable to lookup vote block seals");
			})
			.unwrap_or_default();

		for vote in stronger_seals.into_iter() {
			let raw_authority = vote.closest_miner.1.to_raw_vec();
			if !self.keystore.has_keys(&[(raw_authority, BLOCK_SEAL_KEY_TYPE)]) {
				tracing::trace!(block = ?block_hash, strength = ?vote.seal_strength,
					"Could not sign vote for block",
				);
				continue;
			}
			return Ok(Some(vote));
		}
		Ok(None)
	}

	async fn get_best_block_at_parent(
		&self,
		voting_schedule: &VotingSchedule,
	) -> Result<Option<(B::Hash, U256, Option<U256>)>, Error> {
		let leaves = self.select_chain.leaves().await?;
		// Blocks are always created with a tick at least notebook tick +1, so the parent will be at
		// notebook tick
		let parent_tick = voting_schedule.parent_block_tick();
		let mut best_parent: Option<(B::Hash, ForkPower)> = None;
		for leaf in &leaves {
			let Some(parent_hash) = self.get_block_ancestor_with_tick(*leaf, parent_tick) else {
				continue;
			};
			// **only** consider it if there really are votes to seal here
			if !self.client.runtime_api().has_eligible_votes(parent_hash).unwrap_or(false) {
				continue;
			}

			let forkpower = self.get_fork_power(parent_hash)?;
			if let Some((_, best_forkpower)) = &best_parent {
				// If we already have a parent, we only want to replace it if the new one has
				// strictly more fork power.
				if forkpower <= *best_forkpower {
					continue;
				}
			}
			best_parent = Some((parent_hash, forkpower));
		}
		let Some((best_parent_hash, _)) = best_parent else {
			tracing::trace!(parent_tick, "Could not find any parent block with votes to seal",);
			return Ok(None);
		};

		// now figure out of there's a descendent we want to be able to beat
		let mut best_child: Option<ForkPower> = None;
		let notebook_in_block_tick = voting_schedule.block_tick();
		for leaf in leaves {
			if leaf == best_parent_hash {
				continue;
			}
			let blocks_at_parent_tick =
				self.client.runtime_api().blocks_at_tick(leaf, parent_tick).unwrap_or_default();
			if blocks_at_parent_tick.is_empty() ||
				!blocks_at_parent_tick.contains(&best_parent_hash)
			{
				continue;
			}

			if let Some(competing_hash) =
				self.get_block_ancestor_with_tick(leaf, notebook_in_block_tick)
			{
				let fork_power = self.get_fork_power(competing_hash)?;
				// If we already have a child, we only want to replace it if the new one has
				// strictly more fork power.
				if let Some(best_fork_power) = &best_child {
					if fork_power < *best_fork_power {
						continue;
					}
				}
				best_child = Some(fork_power);
			}
		}

		let (best_peer_seal_strength, best_peer_xor_distance) = if let Some(power) = &best_child {
			(power.seal_strength, power.miner_vote_xor_distance)
		} else {
			(U256::MAX, None)
		};

		Ok(Some((best_parent_hash, best_peer_seal_strength, best_peer_xor_distance)))
	}

	fn get_block_ancestor_with_tick(&self, hash: B::Hash, tick: Tick) -> Option<B::Hash> {
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
	xor_distance: Option<U256>,
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
	Ok(BlockSealDigest::Vote { seal_strength, signature, xor_distance })
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;
	use sp_core::H256;
	use sp_keyring::Ed25519Keyring;
	use sp_keystore::{Keystore, testing::MemoryKeystore};

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
			U256::from(1),
			None
		));
	}

	#[test]
	fn it_fails_if_not_installed() {
		setup_logs();
		let keystore = create_keystore(Ed25519Keyring::Alice);

		let block_hash = H256::from([31; 32]);
		let nonce = U256::from(1);

		assert!(matches!(
			create_vote_seal(
				&keystore,
				&block_hash,
				&Ed25519Keyring::Bob.public().into(),
				nonce,
				None
			),
			Err(Error::ConsensusError(ConsensusError::CannotSign(_)))
		),);
	}
}
