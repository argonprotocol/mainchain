use std::{default::Default, marker::PhantomData, sync::Arc};

use crate::{
	aux::{ForkPower, UlxAux},
	block_creator::CreateTaxVoteBlock,
	digests::get_tick_digest,
	error::Error,
	notebook_auditor::NotebookAuditor,
	sign_vote::sign_vote,
	LOG_TARGET,
};
use codec::{Decode, Encode};
use futures::{channel::mpsc::*, prelude::*};
use log::*;
use sc_client_api::{AuxStore, BlockOf, BlockchainEvents};
use sc_transaction_pool_api::{TransactionFor, TransactionPool};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::SelectChain;
use sp_core::{H256, U256};
use sp_keystore::KeystorePtr;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_timestamp::Timestamp;
use ulx_node_runtime::NotaryRecordT;
use ulx_primitives::{
	digests::{BlockVoteDigest, BLOCK_VOTES_DIGEST_ID},
	inherents::BlockSealInherent,
	localchain::BlockVote,
	notary::{NotaryNotebookTickState, NotaryNotebookVoteDetails, NotaryNotebookVoteDigestDetails},
	tick::Tick,
	BlockSealSpecApis, MiningAuthorityApis, NotaryApis, NotebookApis,
};

pub struct NotebookState<Block: BlockT, TP, C, SC> {
	pub pool: Arc<TP>,
	client: Arc<C>,
	select_chain: SC,
	keystore: KeystorePtr,
	sender: Sender<CreateTaxVoteBlock<Block>>,
	_phantom: PhantomData<Block>,
}

pub fn get_notary_state<B: BlockT, C: AuxStore>(
	client: &Arc<C>,
	tick: Tick,
) -> Result<NotaryNotebookTickState, Error<B>> {
	let state_key = notary_state_key(tick);
	let notary_state = match client.get_aux(&state_key)? {
		Some(bytes) => NotaryNotebookTickState::decode(&mut &bytes[..]).unwrap_or_default(),
		None => Default::default(),
	};
	Ok(notary_state)
}

fn notary_state_key(tich: Tick) -> Vec<u8> {
	("NotaryStateAtTick", tich).encode()
}

impl<B, TP, C, SC> NotebookState<B, TP, C, SC>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + BlockchainEvents<B> + HeaderBackend<B> + AuxStore + BlockOf + 'static,
	C::Api: NotebookApis<B>
		+ BlockSealSpecApis<B>
		+ NotaryApis<B, NotaryRecordT>
		+ MiningAuthorityApis<B>,
	TP: TransactionPool<Block = B>,
	SC: SelectChain<B>,
{
	pub fn new(
		pool: Arc<TP>,
		client: Arc<C>,
		select_chain: SC,
		keystore: KeystorePtr,
		sender: Sender<CreateTaxVoteBlock<B>>,
	) -> Self {
		Self { pool, client, select_chain, keystore, sender, _phantom: PhantomData }
	}

	pub async fn try_process_notebook(
		&mut self,
		tx_data: &TransactionFor<TP>,
		auditor: &mut NotebookAuditor<B, C>,
	) -> Result<(), Error<B>> {
		// Use the latest hash to check the state of the notebooks. The API should NOT
		// use the block hash for state.
		let best_header = self.get_best_block_header().await.ok_or_else(|| {
			Error::NoBestHeader("Unable to get best header for notebook processing".to_string())
		})?;
		let best_hash = best_header.hash();

		let Some(vote_details) =
			self.client.runtime_api().decode_notebook_vote_details(best_hash, tx_data)?
		else {
			return Err(Error::NotaryError("Unable to decode notebook vote details".to_string()))
		};

		let tick = vote_details.tick;
		let notary_id = vote_details.notary_id.clone();
		let notebook_number = vote_details.notebook_number.clone();

		let notary_state = self.update_tick_state(best_hash, &vote_details)?;

		let tick_at_best_block = get_tick_digest(&best_header.digest()).ok_or_else(|| {
			Error::StringError(format!(
				"Unable to get tick digest for best block hash {}",
				best_hash
			))
		})?;

		let mut best_hash_at_notebook_tick = best_hash.clone();
		if tick_at_best_block > tick {
			(best_hash_at_notebook_tick, _) =
				get_block_descendent_with_tick(&self.client, best_hash, tick).ok_or(
					Error::BlockNotFound(format!(
						"Could not find a block with the notebook tick ({}) starting at this block hash {}",
						tick,
						best_hash
					)),
				)?;
		}

		auditor.try_audit_notebook(&best_hash_at_notebook_tick, &vote_details).await?;

		if tick < 2 {
			return Ok(())
		}

		let timestamp = Timestamp::current();

		let block_votes = UlxAux::get_votes(self.client.as_ref(), tick - 2)?;
		if block_votes.is_empty() {
			return Ok(())
		}

		// aren't these ordered?
		let strongest_fork_at_tick = UlxAux::strongest_fork_at_tick(self.client.as_ref(), tick)?;

		let beatable_forks = self
			.get_beatable_forks(tick - 1, &notary_state, &strongest_fork_at_tick)
			.await?;

		for fork in beatable_forks {
			info!(
				target: LOG_TARGET,
				"Trying to add to fork {:?}, tick={}.",
				fork.block_hash,
				tick -1,
			);
			let best_vote_proofs = self
				.client
				.runtime_api()
				.get_best_vote_proofs(fork.block_hash, &block_votes)?
				.expect("Must be able to call the runtime api");

			for best_vote in best_vote_proofs {
				if best_vote.vote_proof > strongest_fork_at_tick.vote_proof {
					info!(
						target: LOG_TARGET,
						"Vote proof not smaller than current best. Skipping vote. Vote proof={}, Best vote proof={}",
						best_vote.vote_proof, strongest_fork_at_tick.vote_proof
					);
					continue
				}

				let Ok((miner_signature, account_id)) =
					sign_vote(&self.client, &self.keystore, &fork.block_hash, best_vote.vote_proof)
				else {
					continue
				};

				let vote = best_vote.block_vote;

				self.sender
					.send(CreateTaxVoteBlock::<B> {
						tick,
						timestamp_millis: timestamp.as_millis(),
						account_id,
						vote_proof: best_vote.vote_proof,
						parent_hash: fork.block_hash,
						latest_finalized_block_needed: notary_state.latest_finalized_block_needed,
						block_vote_digest: notary_state.block_vote_digest.clone(),
						seal_inherent: BlockSealInherent::Vote {
							vote_proof: best_vote.vote_proof,
							notary_id,
							source_notebook_number: notebook_number,
							source_notebook_proof: best_vote.source_notebook_proof,
							block_vote: BlockVote {
								account_id: vote.account_id,
								power: vote.power,
								channel_pass: vote.channel_pass,
								index: vote.index,
								grandparent_block_hash: H256::from_slice(
									vote.grandparent_block_hash.as_ref(),
								),
							},
							miner_signature,
						},
					})
					.await?;
			}
		}
		Ok(())
	}

	/// This function gets the active forks and the associated block voting for each
	///
	/// The leaves are all the active forks that have no children. We are going to get all that have
	/// a given block height.
	///
	/// ## Tiers
	/// ==== Grandparent - votes were submitted for best block
	/// ==== Parent - we included 1+ notebooks that showed votes for the grandparent. A secret key
	/// is omitted for each notebook. ==== At block height - secret keys are revealed, and the
	/// parent voting key can be formed.
	async fn get_beatable_forks(
		&self,
		tick: Tick,
		notary_state: &NotaryNotebookTickState,
		strongest_fork_at_tick: &ForkPower,
	) -> Result<Vec<VotingFork<B>>, Error<B>> {
		let leaves = self.select_chain.leaves().await?;

		let mut beatable_forks = vec![];
		if tick < 2 {
			return Ok(beatable_forks)
		}

		for leaf in leaves {
			let (block_hash, _) = match get_block_descendent_with_tick(&self.client, leaf, tick) {
				Some(x) => x,
				_ => continue,
			};

			let mut fork_power = UlxAux::get_fork_voting_power(self.client.as_ref(), &block_hash)?;
			fork_power.add_vote(
				notary_state.block_vote_digest.voting_power,
				notary_state.notebook_key_details_by_notary.len() as u32,
				U256::zero(),
			);

			if fork_power > *strongest_fork_at_tick {
				beatable_forks.push(VotingFork {
					block_hash,
					voting_key: self
						.client
						.runtime_api()
						.parent_voting_key(block_hash.clone())
						.expect("Must be able to call the runtime api"),
					fork_power,
				});
			}
		}
		Ok(beatable_forks)
	}

	async fn get_best_block_header(&self) -> Option<B::Header> {
		let best_header = match self.select_chain.best_chain().await {
			Ok(x) => x,
			Err(err) => {
				warn!(
					target: LOG_TARGET,
					"Unable to pull new block for authoring. \
					 Select best chain error: {}",
					err
				);
				return None
			},
		};
		Some(best_header)
	}

	fn update_tick_state(
		&self,
		block_hash: B::Hash,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<NotaryNotebookTickState, Error<B>> {
		let mut state = get_notary_state(&self.client, vote_details.tick)?;

		let finalized_block_needed = vote_details.finalized_block_number;
		let vote_details = NotaryNotebookVoteDigestDetails::from(vote_details);
		let notary_id = vote_details.notary_id;
		if state
			.notebook_key_details_by_notary
			.insert(notary_id, vote_details.clone())
			.is_none()
		{
			if finalized_block_needed > state.latest_finalized_block_needed {
				state.latest_finalized_block_needed = finalized_block_needed;
			}

			let tick_notebooks = state
				.notebook_key_details_by_notary
				.iter()
				.map(|(_, a)| a.clone())
				.collect::<Vec<_>>();

			state.block_vote_digest =
				self.client.runtime_api().create_vote_digest(block_hash, tick_notebooks)?;

			let state_key = notary_state_key(vote_details.tick);
			self.client
				.insert_aux(&[(state_key.as_slice(), state.encode().as_slice())], &[])?;
		}
		Ok(state)
	}
}

/// Checks if the applicable parent height has tax votes. This would be the parent block of the
/// given header
pub fn has_applicable_tax_votes<C, B>(
	client: &Arc<C>,
	solve_header: &B::Header,
	current_tick: Tick,
) -> bool
where
	B: BlockT,
	C: HeaderBackend<B>,
{
	if current_tick < 2 {
		return false
	}
	let (_, descendent_with_tick) =
		match get_block_descendent_with_tick(client, solve_header.hash(), current_tick - 2) {
			Some(x) => x,
			_ => return false,
		};

	for log in &descendent_with_tick.digest().logs {
		if let Some(votes) = log.pre_runtime_try_to::<BlockVoteDigest>(&BLOCK_VOTES_DIGEST_ID) {
			return votes.votes_count > 0
		}
	}

	false
}

pub fn get_block_descendent_with_tick<B: BlockT, C: HeaderBackend<B>>(
	client: &Arc<C>,
	hash: B::Hash,
	tick: Tick,
) -> Option<(B::Hash, B::Header)> {
	let mut hash = hash;
	// put in a large artificial limit
	for _ in 0..10_000 {
		match client.header(hash) {
			Ok(Some(header)) => {
				if let Some(header_tick) = get_tick_digest(header.digest()) {
					if tick == header_tick {
						return Some((header.hash(), header))
					}
					if header_tick < tick {
						return None
					}
				}
				hash = header.parent_hash().clone();
			},
			_ => return None,
		}
	}
	None
}

#[derive(Clone, PartialEq, Eq, Encode, Decode)]
struct VotingFork<Block: BlockT> {
	block_hash: Block::Hash,
	voting_key: Option<H256>,
	fork_power: ForkPower,
}
