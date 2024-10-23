use std::{default::Default, marker::PhantomData, sync::Arc};

use codec::Codec;
use futures::{channel::mpsc::*, prelude::*};
use log::*;
use sc_client_api::AuxStore;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{Error as ConsensusError, SelectChain};
use sp_core::{ByteArray, U256};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_timestamp::Timestamp;

use argon_primitives::{
	block_seal::BLOCK_SEAL_CRYPTO_ID,
	digests::{BlockVoteDigest, BLOCK_VOTES_DIGEST_ID},
	localchain::BlockVote,
	tick::Tick,
	BlockSealApis, BlockSealAuthorityId, BlockSealAuthoritySignature, BlockVotingPower, TickApis,
	BLOCK_SEAL_KEY_TYPE,
};

use crate::{
	aux_client::{ArgonAux, ForkPower},
	block_creator::CreateTaxVoteBlock,
	error::Error,
};

const LOG_TARGET: &str = "node::consensus::notebook_sealer";

pub struct NotebookSealer<B: BlockT, C: AuxStore, SC, AC: Clone + Codec> {
	client: Arc<C>,
	select_chain: Arc<SC>,
	keystore: KeystorePtr,
	sender: Sender<CreateTaxVoteBlock<B, AC>>,
	aux_client: ArgonAux<B, C>,
	_phantom: PhantomData<B>,
}

impl<B, C, SC, AC> Clone for NotebookSealer<B, C, SC, AC>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + AuxStore + 'static,
	C::Api: BlockSealApis<B, AC, BlockSealAuthorityId> + TickApis<B>,
	AC: Codec + Clone,
{
	fn clone(&self) -> Self {
		Self {
			client: self.client.clone(),
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
	C: ProvideRuntimeApi<B> + AuxStore + 'static,
	C::Api: BlockSealApis<B, AC, BlockSealAuthorityId> + TickApis<B>,
	SC: SelectChain<B> + 'static,
	AC: Codec + Clone,
{
	pub fn new(
		client: Arc<C>,
		select_chain: SC,
		keystore: KeystorePtr,
		aux_client: ArgonAux<B, C>,
		sender: Sender<CreateTaxVoteBlock<B, AC>>,
	) -> Self {
		Self {
			client: client.clone(),
			select_chain: Arc::new(select_chain),
			aux_client,
			keystore,
			sender,
			_phantom: PhantomData,
		}
	}

	pub async fn check_for_new_blocks(
		&self,
		tick: Tick,
		voting_power: BlockVotingPower,
		notebooks: u32,
	) -> Result<(), Error> {
		let timestamp_millis = Timestamp::current().as_millis();

		let votes_tick = tick.saturating_sub(2);
		let vote_key_tick = tick.saturating_sub(1);
		let block_votes = self.aux_client.get_votes(votes_tick)?.get();
		let votes_count = block_votes.iter().fold(0u32, |acc, x| acc + x.raw_votes.len() as u32);
		if votes_count == 0 {
			trace!(target: LOG_TARGET, "No block votes at tick {}", votes_tick);
			return Ok(());
		}
		info!(target: LOG_TARGET, "Checking {} block votes for tick {}", votes_count, votes_tick);

		// aren't these ordered?
		let strongest_fork_at_tick = self.aux_client.strongest_fork_at_tick(tick)?.get();
		let strongest_seal_strength = strongest_fork_at_tick.seal_strength;

		let Some(best_hash) = self
			.get_best_beatable_fork(vote_key_tick, voting_power, notebooks, strongest_fork_at_tick)
			.await?
		else {
			trace!(target: LOG_TARGET, "No beatable fork at tick {} with {} notebooks and {} voting power", vote_key_tick, notebooks, voting_power);
			return Ok(());
		};

		let stronger_seals = self
			.client
			.runtime_api()
			.find_vote_block_seals(best_hash, block_votes, strongest_seal_strength)?
			.expect("Must be able to call the runtime api");

		for vote in stronger_seals.into_iter() {
			let Ok(miner_signature) = try_sign_vote(
				&self.keystore,
				&best_hash,
				&vote.closest_miner.1,
				vote.seal_strength,
			) else {
				continue;
			};

			let mut sender = self.sender.clone();
			sender
				.send(CreateTaxVoteBlock::<B, AC> {
					tick,
					timestamp_millis,
					vote,
					parent_hash: best_hash,
					signature: miner_signature,
				})
				.await?;
			return Ok(());
		}
		Ok(())
	}

	/// This function gets the active forks and the associated block voting for each
	///
	/// The leaves are all the active forks that have no children. We are going to get all at the
	/// tick level
	async fn get_best_beatable_fork(
		&self,
		tick: Tick,
		block_voting_power: BlockVotingPower,
		block_notebooks: u32,
		strongest_fork_at_tick: ForkPower,
	) -> Result<Option<B::Hash>, Error> {
		let leaves = self.select_chain.leaves().await?;

		let mut best_fork = ForkPower::default();
		let mut best_hash = None;

		let max_nonce = U256::zero();

		for leaf in leaves {
			let Some(block_hash) = get_block_descendent_with_tick(&self.client, leaf, tick) else {
				continue;
			};

			let mut fork_power = self.aux_client.get_fork_voting_power(&block_hash)?.get();
			fork_power.add_vote(block_voting_power, block_notebooks, max_nonce);

			if fork_power > strongest_fork_at_tick && fork_power > best_fork {
				best_fork = fork_power;
				best_hash = Some(block_hash);
			}
		}

		Ok(best_hash)
	}
}

/// Checks if the applicable parent height has tax votes. This would be the parent block of the
/// given header
pub fn has_votes_at_tick<C, B>(client: &Arc<C>, solve_header: &B::Header, at_tick: Tick) -> bool
where
	B: BlockT,
	C: HeaderBackend<B> + ProvideRuntimeApi<B>,
	C::Api: TickApis<B>,
{
	if at_tick < 2 {
		return false;
	}
	let Some(block_with_tick) =
		get_block_descendent_with_tick(client, solve_header.hash(), at_tick)
	else {
		return false;
	};

	let Some(descendent_with_tick) =
		client.header(block_with_tick).expect("Must be able to get header from client")
	else {
		return false;
	};

	for log in &descendent_with_tick.digest().logs {
		if let Some(votes) = log.pre_runtime_try_to::<BlockVoteDigest>(&BLOCK_VOTES_DIGEST_ID) {
			return votes.votes_count > 0;
		}
	}

	false
}

pub fn get_block_descendent_with_tick<B: BlockT, C: ProvideRuntimeApi<B>>(
	client: &Arc<C>,
	hash: B::Hash,
	tick: Tick,
) -> Option<B::Hash>
where
	C::Api: TickApis<B>,
{
	if let Ok(current_tick) = client.runtime_api().current_tick(hash) {
		if current_tick == tick {
			return Some(hash);
		}
	}
	if let Ok(blocks) = client.runtime_api().blocks_at_tick(hash, tick) {
		return blocks.last().copied();
	}
	None
}

pub(crate) fn try_sign_vote<Hash: Codec>(
	keystore: &KeystorePtr,
	block_hash: &Hash,
	seal_authority_id: &BlockSealAuthorityId,
	seal_strength: U256,
) -> Result<BlockSealAuthoritySignature, ConsensusError> {
	if !keystore.has_keys(&[(seal_authority_id.to_raw_vec(), BLOCK_SEAL_KEY_TYPE)]) {
		return Err(ConsensusError::CannotSign(format!(
			"Keystore does not have keys for {}",
			seal_authority_id
		)));
	}

	let message = BlockVote::seal_signature_message(block_hash, seal_strength);
	let signature = keystore
		.sign_with(
			BLOCK_SEAL_KEY_TYPE,
			BLOCK_SEAL_CRYPTO_ID,
			seal_authority_id.as_slice(),
			&message,
		)
		.map_err(|e| ConsensusError::CannotSign(format!("{}. Key: {:?}", e, seal_authority_id)))?
		.ok_or_else(|| {
			ConsensusError::CannotSign(format!(
				"Could not find key in keystore. Key: {:?}",
				seal_authority_id
			))
		})?;

	let signature = signature
		.clone()
		.try_into()
		.map_err(|_| ConsensusError::InvalidSignature(signature, seal_authority_id.to_raw_vec()))?;
	Ok(signature)
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;
	use sp_core::H256;
	use sp_keyring::Ed25519Keyring;
	use sp_keystore::{testing::MemoryKeystore, Keystore};

	use argon_primitives::block_seal::BLOCK_SEAL_KEY_TYPE;

	use crate::tests::setup_logs;

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

		assert_ok!(try_sign_vote(
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
			try_sign_vote(&keystore, &block_hash, &Ed25519Keyring::Bob.public().into(), nonce),
			Err(ConsensusError::CannotSign(_))
		),);
	}
}
