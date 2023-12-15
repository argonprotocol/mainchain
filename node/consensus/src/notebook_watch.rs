use std::{collections::BTreeMap, default::Default, marker::PhantomData, sync::Arc};

use codec::{Codec, Encode};
use futures::{channel::mpsc::*, prelude::*};
use log::*;
use sc_client_api::{AuxStore, BlockOf};
use sc_transaction_pool_api::{InPoolTransaction, TransactionFor, TransactionPool};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{Error as ConsensusError, SelectChain};
use sp_core::{crypto::CryptoTypeId, ByteArray, U256};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{
	app_crypto::AppCrypto,
	traits::{Block as BlockT, Header as HeaderT},
	transaction_validity::TransactionTag,
};
use sp_timestamp::Timestamp;

use ulx_node_runtime::{AccountId, NotaryRecordT, NotebookVerifyError};
use ulx_primitives::{
	digests::{BlockVoteDigest, BLOCK_VOTES_DIGEST_ID},
	inherents::BlockSealInherent,
	localchain::{BestBlockVoteProofT, BlockVote},
	notary::{NotaryNotebookTickState, NotaryNotebookVoteDetails, NotaryNotebookVoteDigestDetails},
	tick::Tick,
	BlockSealAuthorityId, BlockSealAuthoritySignature, BlockSealSpecApis, MiningAuthorityApis,
	NotaryApis, NotaryId, NotebookApis, NotebookVotes, BLOCK_SEAL_KEY_TYPE,
};

use crate::{
	aux::{ForkPower, UlxAux},
	block_creator::CreateTaxVoteBlock,
	digests::get_tick_digest,
	error::Error,
	notebook_auditor::NotebookAuditorProvider,
	LOG_TARGET,
};

const SEAL_CRYPTO_ID: CryptoTypeId = <BlockSealAuthorityId as AppCrypto>::CRYPTO_ID;
pub struct NotebookWatch<Block: BlockT, TP, C, SC> {
	pub pool: Arc<TP>,
	client: Arc<C>,
	select_chain: SC,
	keystore: KeystorePtr,
	sender: Sender<CreateTaxVoteBlock<Block>>,
	_phantom: PhantomData<Block>,
	notebook_provides_prefix: TransactionTag,
}

impl<B, TP, C, SC> NotebookWatch<B, TP, C, SC>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + AuxStore + BlockOf + 'static,
	C::Api: NotebookApis<B, NotebookVerifyError>
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
		Self {
			pool,
			client,
			select_chain,
			keystore,
			sender,
			notebook_provides_prefix: ("Notebook").encode(),
			_phantom: PhantomData,
		}
	}

	pub async fn check_tx<Aud>(
		&mut self,
		tx: &Arc<<TP as TransactionPool>::InPoolTransaction>,
		auditor: &mut Aud,
	) where
		Aud: NotebookAuditorProvider<B> + ?Sized,
	{
		if !self.is_notebook_tx(tx) {
			return
		}

		let Some((vote_details, best_hash)) = self
			.decode_notebook_vote_details(tx.data())
			.await
			.map_err(|e| {
				warn!(
					target: LOG_TARGET,
					"Unable to decode notebook vote details. Error: {}",
					e.to_string()
				);
			})
			.ok()
		else {
			return
		};

		match self.try_process_notebook(&best_hash, vote_details, auditor).await {
			Err(e) => {
				warn!(
					target: LOG_TARGET,
					"Unable to process notebook. Error: {}",
					e.to_string()
				);
			},
			_ => (),
		};
	}

	fn is_notebook_tx(&self, tx: &Arc<<TP as TransactionPool>::InPoolTransaction>) -> bool {
		let tag = &self.notebook_provides_prefix;
		tx.provides().len() > 0 && tx.provides()[0].starts_with(tag)
	}

	async fn decode_notebook_vote_details(
		&self,
		tx_data: &TransactionFor<TP>,
	) -> Result<(NotaryNotebookVoteDetails<B::Hash>, B::Hash), Error<B>> {
		let best_header = self.select_chain.best_chain().await.map_err(|_| {
			Error::NoBestHeader("Unable to get best header for notebook processing".to_string())
		})?;
		let best_hash = best_header.hash();
		let vote_details = self
			.client
			.runtime_api()
			.decode_notebook_vote_details(best_hash, tx_data)?
			.ok_or(Error::NotaryError("Unable to decode notebook vote details".to_string()))?;
		Ok((vote_details, best_hash))
	}

	async fn try_process_notebook<Aud>(
		&mut self,
		best_hash: &B::Hash,
		vote_details: NotaryNotebookVoteDetails<B::Hash>,
		auditor: &mut Aud,
	) -> Result<(), Error<B>>
	where
		Aud: NotebookAuditorProvider<B> + ?Sized,
	{
		info!(
			"Got inbound Notebook (notary {}, notebook {}, tick {}.",
			vote_details.notary_id, vote_details.notebook_number, vote_details.tick
		);

		let notary_state = self.update_tick_state(best_hash, &vote_details)?;

		let tick = vote_details.tick;

		let (best_hash_at_notebook_tick, _) =
			get_block_descendent_with_tick(&self.client, *best_hash, tick).ok_or(
				Error::BlockNotFound(format!(
					"Could not find a block with the notebook tick ({}) starting at this block hash {}",
					tick,
					best_hash
				)),
			)?;

		auditor.audit_notebook(&best_hash_at_notebook_tick, &vote_details).await?;

		if tick < 2 {
			return Ok(())
		}

		let timestamp_millis = Timestamp::current().as_millis();

		let block_votes = UlxAux::get_votes(self.client.as_ref(), tick - 2)?;
		if block_votes.is_empty() {
			return Ok(())
		}

		// aren't these ordered?
		let strongest_fork_at_tick = UlxAux::strongest_fork_at_tick(self.client.as_ref(), tick)?;
		let strongest_vote_proof = strongest_fork_at_tick.vote_proof;

		let Some(best_hash) = self
			.get_best_beatable_fork(tick - 1, &notary_state, strongest_fork_at_tick)
			.await?
		else {
			trace!("No beatable fork at tick {}", tick - 1);
			return Ok(())
		};

		let Some((best_vote, account_id, miner_signature)) =
			self.find_best_vote_proof(best_hash, block_votes, strongest_vote_proof).await?
		else {
			return Ok(())
		};

		self.sender
			.send(CreateTaxVoteBlock::<B> {
				tick,
				timestamp_millis,
				account_id,
				vote_proof: best_vote.vote_proof,
				parent_hash: best_hash,
				latest_finalized_block_needed: notary_state.latest_finalized_block_needed,
				block_vote_digest: notary_state.block_vote_digest.clone(),
				seal_inherent: BlockSealInherent::from_vote(
					vote_details.notary_id,
					vote_details.notebook_number,
					best_vote,
					miner_signature,
				),
			})
			.await?;
		Ok(())
	}

	async fn find_best_vote_proof(
		&mut self,
		best_hash: B::Hash,
		block_votes: BTreeMap<NotaryId, NotebookVotes>,
		strongest_proof: U256,
	) -> Result<
		Option<(BestBlockVoteProofT<B::Hash>, AccountId, BlockSealAuthoritySignature)>,
		Error<B>,
	> {
		let best_vote_proofs = self
			.client
			.runtime_api()
			.get_best_vote_proofs(best_hash, &block_votes)?
			.expect("Must be able to call the runtime api");

		let Some(usable_vote) =
			best_vote_proofs.into_iter().find(|x| x.vote_proof <= strongest_proof)
		else {
			trace!(
				target: LOG_TARGET,
				"No usable vote proofs found for block {}.",
				best_hash);
			return Ok(None)
		};

		let vote_proof = usable_vote.vote_proof;

		let Some(usable_authority) = self
			.client
			.runtime_api()
			.xor_closest_authority(best_hash, vote_proof)?
			.filter(|a| {
				self.keystore.has_keys(&[(a.authority_id.to_raw_vec(), BLOCK_SEAL_KEY_TYPE)])
			})
		else {
			trace!(
				target: LOG_TARGET,
				"No vote proofs with installed authority for block {}.",
				best_hash);
			return Ok(None)
		};

		let authority_id = usable_authority.authority_id;
		let account_id = usable_authority.account_id;
		let Ok(miner_signature) = sign_vote(&self.keystore, &authority_id, &best_hash, vote_proof)
		else {
			return Ok(None)
		};

		Ok(Some((usable_vote, account_id, miner_signature)))
	}

	/// This function gets the active forks and the associated block voting for each
	///
	/// The leaves are all the active forks that have no children. We are going to get all at the
	/// tick level
	async fn get_best_beatable_fork(
		&self,
		tick: Tick,
		notary_state: &NotaryNotebookTickState,
		strongest_fork_at_tick: ForkPower,
	) -> Result<Option<B::Hash>, Error<B>> {
		let leaves = self.select_chain.leaves().await?;

		let mut best_fork = ForkPower::default();
		let mut best_hash = None;
		for leaf in leaves {
			let (block_hash, _) = match get_block_descendent_with_tick(&self.client, leaf, tick) {
				Some(x) => x,
				_ => continue,
			};

			let mut fork_power = UlxAux::get_fork_voting_power(self.client.as_ref(), &block_hash)?;
			fork_power.add_vote(
				notary_state.block_vote_digest.voting_power,
				notary_state.notebook_key_details_by_notary.len() as u32,
				U256::zero(), // this is the best possible vote proof
			);

			if fork_power > strongest_fork_at_tick && fork_power > best_fork {
				best_fork = fork_power;
				best_hash = Some(block_hash);
			}
		}

		Ok(best_hash)
	}

	fn update_tick_state(
		&self,
		block_hash: &B::Hash,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<NotaryNotebookTickState, Error<B>> {
		let mut state = UlxAux::get_notebook_tick_state(self.client.as_ref(), vote_details.tick)?;

		state.latest_finalized_block_needed =
			state.latest_finalized_block_needed.max(vote_details.finalized_block_number);

		let vote_details = NotaryNotebookVoteDigestDetails::from(vote_details);
		let tick = vote_details.tick;
		if state
			.notebook_key_details_by_notary
			.insert(vote_details.notary_id, vote_details)
			.is_none()
		{
			let tick_notebooks = state
				.notebook_key_details_by_notary
				.iter()
				.map(|(_, a)| a.clone())
				.collect::<Vec<_>>();

			state.block_vote_digest =
				self.client.runtime_api().create_vote_digest(*block_hash, tick_notebooks)?;

			UlxAux::update_notebook_tick_state(self.client.as_ref(), tick, state.clone())?;
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
				// if genesis, return
				if *header.number() == 0u32.into() && tick == 0 {
					return Some((header.hash(), header));
				}
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

pub(crate) fn sign_vote<Hash: Codec>(
	keystore: &KeystorePtr,
	authority_id: &BlockSealAuthorityId,
	block_hash: &Hash,
	vote_proof: U256,
) -> Result<BlockSealAuthoritySignature, ConsensusError> {
	let message = BlockVote::vote_proof_signature_message(block_hash, vote_proof);
	let signature = keystore
		.sign_with(BLOCK_SEAL_KEY_TYPE, SEAL_CRYPTO_ID, authority_id.as_slice(), &message)
		.map_err(|e| ConsensusError::CannotSign(format!("{}. Key: {:?}", e, authority_id)))?
		.ok_or_else(|| {
			ConsensusError::CannotSign(format!(
				"Could not find key in keystore. Key: {:?}",
				authority_id
			))
		})?;

	let signature = signature
		.clone()
		.try_into()
		.map_err(|_| ConsensusError::InvalidSignature(signature, authority_id.to_raw_vec()))?;
	Ok(signature)
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;

	use frame_support::assert_ok;
	use sc_network_test::Block;
	use sp_api::{ApiRef, ProvideRuntimeApi};
	use sp_core::{bounded_vec, crypto::AccountId32, OpaquePeerId, H256};
	use sp_keyring::Ed25519Keyring;
	use sp_keystore::{testing::MemoryKeystore, Keystore};

	use ulx_primitives::block_seal::{MiningAuthority, PeerId, BLOCK_SEAL_KEY_TYPE};

	use crate::tests::setup_logs;

	use super::*;

	#[derive(Default, Clone)]
	struct TestApi {
		pub authority_id_by_index: BTreeMap<u16, (AccountId32, BlockSealAuthorityId)>,
		pub block_peer_order: Vec<u16>,
	}

	struct RuntimeApi {
		inner: TestApi,
	}

	impl ProvideRuntimeApi<Block> for TestApi {
		type Api = RuntimeApi;

		fn runtime_api(&self) -> ApiRef<'_, Self::Api> {
			RuntimeApi { inner: self.clone() }.into()
		}
	}

	sp_api::mock_impl_runtime_apis! {
		impl MiningAuthorityApis<Block> for RuntimeApi {
			fn xor_closest_authority(_: U256) -> Option<MiningAuthority<BlockSealAuthorityId, AccountId32>> {
				self.inner.block_peer_order.first().map(|a| {
					let (account_id, id)= self.inner.authority_id_by_index.get(&a).unwrap();
					MiningAuthority {
						account_id: account_id.clone(),
						authority_id: id.clone(),
						authority_index: *a,
						peer_id: PeerId(OpaquePeerId::default()),
						rpc_hosts: bounded_vec![],
					}
				})
			}
		}
	}

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

		assert_ok!(sign_vote(
			&keystore,
			&Ed25519Keyring::Alice.public().into(),
			&H256::from_slice(&[2u8; 32]),
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
			sign_vote(&keystore, &Ed25519Keyring::Bob.public().into(), &block_hash, nonce),
			Err(ConsensusError::CannotSign(_))
		),);
	}
}
