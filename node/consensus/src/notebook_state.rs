use std::{collections::BTreeMap, convert::Into, default::Default, marker::PhantomData, sync::Arc};

use codec::{Codec, Decode, Encode};
use futures::{channel::mpsc::*, prelude::*};
use lazy_static::lazy_static;
use log::*;
use sc_client_api::{AuxStore, BlockOf, BlockchainEvents};
use sc_transaction_pool_api::{InPoolTransaction, TransactionFor, TransactionPool};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::SelectChain;
use sp_core::{H256, U256};
use sp_keystore::KeystorePtr;
use sp_runtime::{
	traits::{Block as BlockT, Header as HeaderT},
	transaction_validity::TransactionTag,
	BoundedVec, DigestItem,
};

use ulx_node_runtime::{AccountId, BlockNumber, NotaryRecordT};
use ulx_notary::apis::notebook::NotebookRpcClient;
use ulx_primitives::{
	block_seal::{BlockVotingPower, VoteSource},
	digests::{BlockVoteDigest, NotaryNotebookDigest, COMPUTE_AUTHORITY_DIGEST_ID},
	inherents::BlockSealInherent,
	localchain::BlockVote,
	notary::{NotaryNotebookSubmissionState, NotaryNotebookVoteDetails},
	notebook::BlockVotingKey,
	BlockSealAuthorityId, BlockVotingApis, MiningAuthorityApis, NotaryApis, NotaryId, NotebookApis,
	AUTHOR_DIGEST_ID,
};

use crate::{
	authority::AuthoritySealer,
	aux::{ForkPower, UlxAux},
	convert_u32,
	error::Error,
	LOG_TARGET,
};

lazy_static! {
	static ref TX_NOTEBOOK_PROVIDE_PREFIX: Vec<u8> = ("Notebook").encode();
}

pub fn create_notebook_watch<B, TP, C, SC>(
	pool: Arc<TP>,
	client: Arc<C>,
	select_chain: SC,
	miner_account_id: AccountId,
	keystore: KeystorePtr,
) -> (impl Future<Output = ()>, Receiver<CreateBlockEvent<B>>)
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + BlockchainEvents<B> + HeaderBackend<B> + AuxStore + BlockOf + 'static,
	C::Api: NotebookApis<B>
		+ BlockVotingApis<B>
		+ NotaryApis<B, NotaryRecordT>
		+ MiningAuthorityApis<B>,
	TP: TransactionPool<Block = B>,
	SC: SelectChain<B>,
{
	let (sender, receiver) = channel(1000);
	let authority_sealer = AuthoritySealer::<B, C>::new(client.clone(), keystore);
	let state =
		NotebookState::new(pool, client, select_chain, miner_account_id, authority_sealer, sender);
	let task = async move {
		let pool = state.pool.clone();
		let mut state = state;
		let mut tx_stream = Box::pin(pool.import_notification_stream());
		while let Some(tx_hash) = tx_stream.next().await {
			if let Some(tx) = pool.ready_transaction(&tx_hash) {
				let tag: TransactionTag = TX_NOTEBOOK_PROVIDE_PREFIX.to_vec();
				if tx.provides().len() > 0 && tx.provides()[0].starts_with(&tag) {
					info!("Got inbound Notebook. {:?}", tx_hash);
					let _ = state.try_process_notebook(tx.data()).await.map_err(|e| {
						warn!(
							target: LOG_TARGET,
							"Unable to process notebook. Error: {}",
							e.to_string()
						);
					});
				}
			}
		}
	};
	(task, receiver)
}

pub struct NotebookState<Block: BlockT, TP, C, SC> {
	pool: Arc<TP>,
	client: Arc<C>,
	select_chain: SC,
	miner_account_id: AccountId,
	authority_sealer: AuthoritySealer<Block, C>,
	notary_client_by_id: BTreeMap<NotaryId, Arc<ulx_notary::Client>>,
	sender: Sender<CreateBlockEvent<Block>>,
	_phantom: PhantomData<Block>,
}

pub struct CreateBlockEvent<Block: BlockT> {
	pub parent_block_number: BlockNumber,
	pub parent_hash: Block::Hash,
	pub block_vote_digest: BlockVoteDigest,
	pub seal_inherent: BlockSealInherent,
	pub is_compute_seal: bool,
	pub block_seal_authority: BlockSealAuthorityId,
	pub latest_finalized_block_needed: BlockNumber,
}

impl<B, TP, C, SC> NotebookState<B, TP, C, SC>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + BlockchainEvents<B> + HeaderBackend<B> + AuxStore + BlockOf + 'static,
	C::Api: NotebookApis<B>
		+ BlockVotingApis<B>
		+ NotaryApis<B, NotaryRecordT>
		+ MiningAuthorityApis<B>,
	TP: TransactionPool<Block = B>,
	SC: SelectChain<B>,
{
	pub fn new(
		pool: Arc<TP>,
		client: Arc<C>,
		select_chain: SC,
		miner_account_id: AccountId,
		authority_sealer: AuthoritySealer<B, C>,
		sender: Sender<CreateBlockEvent<B>>,
	) -> Self {
		Self {
			pool,
			client,
			select_chain,
			authority_sealer,
			miner_account_id,
			notary_client_by_id: Default::default(),
			sender,
			_phantom: PhantomData,
		}
	}

	async fn try_process_notebook(&mut self, tx_data: &TransactionFor<TP>) -> Result<(), Error<B>> {
		// Use the latest hash to check the state of the notebooks. The API should NOT
		// use the block hash for state.
		let best_header = self.get_best_block_header().await.ok_or_else(|| {
			Error::NoBestHeader("Unable to get best header for notebook processing".to_string())
		})?;
		let best_hash = best_header.hash();
		let best_header_number = convert_u32::<B>(best_header.number());

		let Some(vote_details) =
			self.client.runtime_api().decode_notebook_vote_details(best_hash, tx_data)?
		else {
			return Err(Error::NotaryError("Unable to decode notebook vote details".to_string()))
		};

		let block_number = vote_details.key_details.block_number;
		let notary_id = vote_details.notary_id.clone();
		let notebook_number = vote_details.notebook_number.clone();
		assert!(block_number >= best_header_number.saturating_sub(1),
			 "Notebook block number too old, not processing. Notary={},notebook={}. Block={} vs current {}", 
			notary_id, notebook_number, block_number, best_header.number()
		);

		let notary_state = self.get_updated_notary_state(block_number, &vote_details)?;

		let Some(notary_details) =
			self.client.runtime_api().notary_by_id(best_hash, notary_id).map_err(|err| {
				Error::NotaryError(format!(
					"Unknown notebook submitted. Notary id {}, notebook = {}; Error: {:?}",
					notary_id, notebook_number, err
				))
			})?
		else {
			return Err(Error::NotaryError(format!(
				"Unknown notebook submitted. Notary id {}, notebook = {}",
				notary_id, notebook_number
			)))
		};

		let mut best_hash_at_notebook_block = best_hash;
		if best_header_number > block_number.into() {
			best_hash_at_notebook_block =
				self.get_parent(best_hash, best_header_number - block_number);
		}

		self.try_audit_notebook(&best_hash_at_notebook_block, &notary_details, &vote_details)
			.await?;
		let strongest_fork_at_height =
			UlxAux::strongest_vote_at_height(self.client.as_ref(), block_number + 1)?;

		if block_number < 3 || notary_state.block_votes < 5 {
			let Some(build_on_header) = self.client.header(best_hash_at_notebook_block)? else {
				return Err(Error::NoBestHeader(
					"Unable to get best header for notebook processing".to_string(),
				))
			};

			let mut account_id = None;
			let mut is_compute_seal = false;
			for digest in build_on_header.digest().logs() {
				match digest {
					DigestItem::PreRuntime(AUTHOR_DIGEST_ID, data) => {
						account_id = AccountId::decode(&mut &data[..]).ok();
					},
					DigestItem::PreRuntime(COMPUTE_AUTHORITY_DIGEST_ID, _) => {
						is_compute_seal = true;
					},
					_ => (),
				}
			}

			let Some(account_id) = account_id else {
				return Err(Error::MissingPreRuntimeDigest("Author Digest".to_string()))
			};

			if account_id == self.miner_account_id {
				let can_make_strongest = self.can_build_strongest_fork(
					best_hash_at_notebook_block,
					notary_state.block_voting_power,
					strongest_fork_at_height.voting_power,
				);

				if can_make_strongest {
					let block_seal_authority = if is_compute_seal {
						self.authority_sealer.get_compute_authority()
					} else {
						self.authority_sealer
							.check_if_can_seal(&build_on_header.hash(), &account_id)
							.ok()
					}
					.ok_or(Error::NoActiveAuthorityInKeystore)?;

					self.sender
						.send(CreateBlockEvent {
							parent_block_number: convert_u32::<B>(build_on_header.number()),
							parent_hash: best_hash_at_notebook_block,
							seal_inherent: BlockSealInherent::Continuation,
							block_vote_digest: notary_state_to_vote_digest(&notary_state),
							block_seal_authority,
							is_compute_seal,
							latest_finalized_block_needed: notary_state
								.latest_finalized_block_needed,
						})
						.await?;
				}
			}

			// don't try to also find a closest nonce
			if block_number < 3 {
				return Ok(())
			}
		}

		let active_forks_by_great_grandparent = self.get_active_fork_weight(block_number).await?;

		let block_votes = if block_number >= 2 {
			UlxAux::get_block_votes(self.client.as_ref(), block_number - 2)?
		} else {
			Default::default()
		};
		let block_vote_digest = notary_state_to_vote_digest(&notary_state);

		for (voting_key, notary_id, best_nonce) in notary_state.best_nonces {
			let vote = best_nonce.block_vote;
			let account_id = vote.account_id;
			let mut can_close =
				block_votes.contains_key(&(notary_id, account_id.clone(), vote.index));

			// check if we are closest, even if not downloaded
			if !can_close {
				can_close = self
					.authority_sealer
					.is_block_peer(&vote.block_hash, account_id.clone())
					.unwrap_or_default();
			}
			if !can_close {
				continue
			}

			// 1. was the vote for an active fork?
			let voted_great_grandparent = vote.block_hash;
			let Some(active_forks) =
				active_forks_by_great_grandparent.get(&voted_great_grandparent.into())
			else {
				info!("Best hash not found for fork. Skipping vote. Great Grandparent {voted_great_grandparent}");
				continue
			};

			// 2. filter the forks by those with the given voting key at the parent tier
			let Some(best_fork) = active_forks.iter().find(|x| {
				if let Some(key) = &x.voting_key {
					return key == &voting_key
				}
				false
			}) else {
				info!("Voting key not found in forks. Skipping vote. Voting key {voting_key}");
				continue
			};

			// 3. is the voting power of the fork high enough?
			let can_build_best_fork = self.can_build_strongest_fork(
				best_fork.block_hash,
				notary_state.block_voting_power,
				strongest_fork_at_height.voting_power,
			);
			if !can_build_best_fork {
				info!(
					"Voting power not high enough. Skipping vote. Voting power {} Max power {}",
					U256::from(notary_state.block_voting_power.clone()) +
						best_fork.voting_power.voting_power,
					strongest_fork_at_height.voting_power
				);
				continue
			};
			if best_nonce.nonce > strongest_fork_at_height.nonce {
				info!(
					"Nonce not smaller than current best. Skipping vote. Nonce {}, Best nonce {}",
					best_nonce.nonce, strongest_fork_at_height.nonce
				);
				continue
			}

			let (block_seal_authority, is_compute_seal) = match &vote.vote_source {
				VoteSource::Compute { .. } => (self.authority_sealer.get_compute_authority(), true),
				VoteSource::Tax { .. } => (
					self.authority_sealer
						.check_if_can_seal(&vote.block_hash, &account_id)
						.ok()
						.map(|a| a),
					false,
				),
			};

			let Some(block_seal_authority) = block_seal_authority else {
				info!(
					"Unable to find block seal authority. Skipping vote. Notary {}, notebook {}, account_id {}",
					notary_id, notebook_number, account_id
				);
				continue
			};

			self.sender
				.send(CreateBlockEvent {
					is_compute_seal,
					parent_block_number: block_number,
					parent_hash: best_fork.block_hash,
					latest_finalized_block_needed: notary_state.latest_finalized_block_needed,
					block_vote_digest: block_vote_digest.clone(),
					seal_inherent: BlockSealInherent::ClosestNonce {
						notary_id,
						source_notebook_number: notebook_number,
						nonce: best_nonce.nonce,
						source_notebook_proof: best_nonce.proof.clone(),
						block_vote: BlockVote {
							account_id: account_id.clone(),
							block_hash: H256::from_slice(vote.block_hash.as_ref()),
							index: vote.index,
							power: vote.power,
							vote_source: vote.vote_source,
						},
					},
					block_seal_authority,
				})
				.await?;
		}
		Ok(())
	}

	fn can_build_strongest_fork(
		&self,
		block_hash: B::Hash,
		block_voting_power: BlockVotingPower,
		max_power_at_next_block: U256,
	) -> bool {
		let fork_voting_power =
			UlxAux::<C, B>::get_fork_voting_power(self.client.as_ref(), &block_hash)
				.unwrap_or_default();
		let new_power = fork_voting_power.voting_power + U256::from(block_voting_power);
		new_power > max_power_at_next_block
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
	async fn get_active_fork_weight(
		&self,
		block_number: u32,
	) -> Result<BTreeMap<B::Hash, Vec<VotingFork<B>>>, Error<B>> {
		let leaves = self.select_chain.leaves().await?;

		let mut active_forks_by_great_grandparent = BTreeMap::new();
		for leaf in leaves {
			let Ok(Some(num)) = self.client.number(leaf) else { continue };
			let num = convert_u32::<B>(&num);
			// stalled before this point
			if num < block_number {
				continue
			};
			let mut block_hash = leaf;
			if num > block_number {
				block_hash = self.get_parent(leaf, num - block_number);
			}

			let great_grandparent = self.get_parent(block_hash, 2);

			let voting_power = UlxAux::get_fork_voting_power(self.client.as_ref(), &block_hash)?;

			let fork = VotingFork {
				block_hash,
				voting_key: self
					.client
					.runtime_api()
					.parent_voting_key(block_hash.clone())
					.expect("Must be able to call the runtime api"),
				voting_power,
			};

			if let Some(mut existing) = active_forks_by_great_grandparent
				.insert(great_grandparent.clone(), vec![fork.clone()])
			{
				existing.push(fork);
				existing.sort_by(|a, b| {
					let cmp = a.voting_power.partial_cmp(&b.voting_power);
					cmp.unwrap_or(std::cmp::Ordering::Equal)
				});
				active_forks_by_great_grandparent.insert(great_grandparent, existing);
			}
		}
		Ok(active_forks_by_great_grandparent)
	}

	fn get_parent(&self, hash: B::Hash, height: BlockNumber) -> B::Hash {
		let mut parent = hash;
		for _ in 0..height {
			match self.client.header(parent) {
				Ok(Some(header)) => {
					if convert_u32::<B>(header.number()) == 0u32 {
						return parent
					}
					parent = header.parent_hash().clone();
				},
				_ => return parent,
			}
		}
		parent
	}

	fn get_updated_notary_state(
		&self,
		block_number: BlockNumber,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<NotaryNotebookSubmissionState<B::Hash>, Error<B>> {
		let state_key = ("NotaryStateAtHeight", block_number).encode();
		let mut notary_state = match self.client.get_aux(&state_key)? {
			Some(bytes) =>
				NotaryNotebookSubmissionState::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		};
		if self.update_block_height_state(&mut notary_state, &vote_details) {
			self.client
				.insert_aux(&[(state_key.as_slice(), notary_state.encode().as_slice())], &[])?;
		}
		Ok(notary_state)
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

	fn update_block_height_state(
		&self,
		state: &mut NotaryNotebookSubmissionState<B::Hash>,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> bool {
		let key_details = vote_details.key_details.clone();
		let notary_id = vote_details.notary_id;
		if state.notebook_key_details_by_notary.insert(notary_id, key_details).is_none() {
			state.block_votes += vote_details.block_votes;
			state.block_voting_power += vote_details.block_voting_power;
			if vote_details.finalized_block_number > state.latest_finalized_block_needed {
				state.latest_finalized_block_needed = vote_details.finalized_block_number;
			}
			state.next_parent_voting_key = BlockVotingKey::create_key(
				state
					.notebook_key_details_by_notary
					.values()
					.map(|x| BlockVotingKey {
						parent_vote_root: x.block_votes_root,
						parent_secret: x.secret_hash,
					})
					.collect::<Vec<_>>(),
			);

			for x in vote_details.best_nonces.iter() {
				state.best_nonces.push(x.clone());
			}
			return true
		}
		false
	}

	async fn try_audit_notebook(
		&mut self,
		block_hash: &B::Hash,
		notary_details: &NotaryRecordT,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<(), Error<B>> {
		let full_notebook = self.download_notebook(&notary_details, &vote_details).await?;
		let mut vote_eligibility = BTreeMap::new();
		for block_hash in &vote_details.blocks_with_votes {
			vote_eligibility.insert(
				block_hash.clone(),
				self.client.runtime_api().vote_eligibility(block_hash.clone()).map_err(|e| {
					let message = format!(
							"Notebook failed audit. Skipping continuation. Notary {}, notebook {}. {:?}",
							vote_details.notary_id, vote_details.notebook_number, e
						);
					Error::<B>::StringError(message)
				})?,
			);
		}
		// audit on the best block at the height of the notebook
		let _ = self
			.client
			.runtime_api()
			.audit_notebook(
				block_hash.clone(),
				vote_details.version.clone(),
				vote_details.notary_id.clone(),
				vote_details.notebook_number.clone(),
				vote_details.header_hash.clone(),
				&vote_eligibility,
				&full_notebook,
			)
			.map_err(|e| {
				let message = format!(
					"Notebook failed audit. Skipping continuation. Notary {}, notebook {}. {:?}",
					vote_details.notary_id, vote_details.notebook_number, e
				);
				Error::<B>::StringError(message)
			})?;
		Ok(())
	}

	async fn download_notebook(
		&mut self,
		notary_details: &NotaryRecordT,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<Vec<u8>, Error<B>> {
		if !self.notary_client_by_id.contains_key(&notary_details.notary_id) {
			let host = notary_details.meta.hosts.get(0).ok_or_else(|| {
				Error::NotaryError("No rpc endpoint found for notary".to_string())
			})?;
			let c = ulx_notary::create_client(host.get_url().as_str()).await.map_err(|e| {
				Error::NotaryError(format!("Could not connect to notary for audit - {:?}", e))
			})?;
			let c = Arc::new(c);
			self.notary_client_by_id.insert(notary_details.notary_id.clone(), c.clone());
		}

		let Some(client) = self.notary_client_by_id.get(&notary_details.notary_id) else {
			return Err(Error::NotaryError("Could not connect to notary for audit".to_string()))
		};
		client.get_raw_body(vote_details.notebook_number).await.map_err(|err| {
			self.notary_client_by_id.remove(&notary_details.notary_id);
			Error::NotaryError(format!("Error downloading notebook: {}", err))
		})
	}
}

fn notary_state_to_vote_digest<Hash: Codec>(
	notary_state: &NotaryNotebookSubmissionState<Hash>,
) -> BlockVoteDigest {
	BlockVoteDigest {
		parent_voting_key: Some(notary_state.next_parent_voting_key),
		notebook_numbers: BoundedVec::truncate_from(
			notary_state
				.notebook_key_details_by_notary
				.iter()
				.map(|(notary_id, key_details)| NotaryNotebookDigest {
					notary_id: notary_id.clone(),
					notebook_number: key_details.notebook_number,
				})
				.collect::<Vec<_>>(),
		),
		voting_power: notary_state.block_voting_power,
		votes_count: notary_state.block_votes,
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode)]
struct VotingFork<Block: BlockT> {
	block_hash: Block::Hash,
	voting_key: Option<H256>,
	voting_power: ForkPower,
}
