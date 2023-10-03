// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{sync::Arc, time::Duration};

use codec::{Decode, Encode};
use futures::prelude::*;
use log::*;
use sc_client_api::{self, BlockchainEvents};
use sc_consensus::{BlockImportParams, BoxBlockImport, ImportResult, StateAction, StorageChanges};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Environment, Proposal, Proposer, SelectChain, SyncOracle};
use sp_core::{crypto::AccountId32, U256};
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_keystore::KeystorePtr;
use sp_runtime::{
	generic::{Digest, DigestItem},
	traits::{Block as BlockT, Header as HeaderT, Header, NumberFor},
};

use rpc::{CreatedBlock, Error as RpcError, SealNewBlock};
use ulx_primitives::{
	block_seal::{BlockProof, MiningAuthorityApis},
	digests::{FinalizedBlockNeededDigest, FINALIZED_BLOCK_DIGEST_ID},
	inherents::UlxBlockSealInherent,
	ProofOfWorkType, UlxConsensusApi, UlxPreDigest, UlxSeal, AUTHOR_ID, ULX_ENGINE_ID,
};

pub use crate::compute_worker::{MiningBuild, MiningHandle, MiningMetadata};
use crate::{
	authority::AuthoritySealer,
	aux::TotalDifficulty,
	compute_worker::UntilImportedOrTimeout,
	error::{
		Error,
		Error::{BlockNotFound, InvalidNonceDifficulty, InvalidProofOfWorkTypeUsed},
	},
};

#[cfg(test)]
mod tests;

pub mod authority;
mod aux;
pub mod basic_queue;
pub mod compute_worker;
pub mod error;
pub mod import_queue;
pub mod inherents;
mod metrics;
pub mod nonce_verify;
pub mod rpc;

const LOG_TARGET: &str = "node::consensus";

/// Algorithm used for proof of work.
pub trait NonceAlgorithm<B: BlockT> {
	/// Difficulty for the algorithm.
	type Difficulty: TotalDifficulty
		+ Default
		+ Encode
		+ Decode
		+ Ord
		+ Clone
		+ Copy
		+ Into<U256>
		+ TryFrom<U256>;

	fn next_digest(&self, parent: &B::Hash) -> Result<UlxPreDigest, Error<B>>;
	fn easing(&self, parent: &B::Hash, block_proof: &BlockProof) -> Result<u128, Error<B>>;

	/// Verify that the difficulty is valid against given seal.
	fn verify(
		&self,
		parent: &B::Hash,
		pre_hash: &B::Hash,
		pre_digest: &UlxPreDigest,
		seal: &UlxSeal,
	) -> Result<bool, Error<B>>;
}

pub async fn listen_for_block_seal<Block, C, S, Algorithm, E, SO, L, CIDP, CS>(
	mut block_import: BoxBlockImport<Block>,
	client: Arc<C>,
	select_chain: S,
	algorithm: Algorithm,
	mut env: E,
	_sync_oracle: SO,
	justification_sync_link: L,
	author: AccountId32,
	create_inherent_data_providers: CIDP,
	build_time: Duration,
	mut block_seal_stream: CS,
	keystore: KeystorePtr,
) where
	Block: BlockT + 'static,
	Block::Hash: Send + 'static,
	C: ProvideRuntimeApi<Block> + BlockchainEvents<Block> + HeaderBackend<Block> + 'static,
	C::Api: UlxConsensusApi<Block>,
	C::Api: MiningAuthorityApis<Block>,
	S: SelectChain<Block> + 'static,
	CS: Stream<Item = SealNewBlock<Block::Hash>> + Unpin + 'static,
	Algorithm: NonceAlgorithm<Block> + Send + Clone + Sync + 'static,
	Algorithm::Difficulty: Send + 'static,
	E: Environment<Block> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<Block>,
	SO: SyncOracle + Clone + Send + Sync + 'static,
	CIDP: CreateInherentDataProviders<Block, UlxBlockSealInherent> + Clone,
	L: sc_consensus::JustificationSyncLink<Block> + 'static,
{
	let authority_sealer = AuthoritySealer::<Block, C>::new(client.clone(), keystore.clone());

	while let Some(command) = block_seal_stream.next().await {
		match command {
			SealNewBlock::Submit { block_proof, nonce, parent_hash, mut sender } => {
				info!(target: LOG_TARGET, "Inbound BlockProof seal received (id={:?}, author={})", block_proof.tax_proof_id, block_proof.author_id);
				let future = async {
					// this finds the longest current chain off finalized path
					let mut parent_header = select_chain.best_chain().await?;

					if parent_hash != parent_header.hash() {
						match select_chain.finality_target(parent_hash, None).await {
							Ok(_) => (),
							Err(err) => {
								warn!(
									target: LOG_TARGET,
									"Unable to propose new block for authoring on the given parent hash {:?}. \
									 Select best chain error: {}",
									parent_hash,
									err
								);
								return Err(err.into())
							},
						};

						parent_header = match client.header(parent_hash) {
							Ok(Some(x)) => x,
							Ok(None) => return Err(BlockNotFound(parent_hash.to_string()).into()),
							Err(err) => return Err(err.into()),
						};
					}

					let pre_digest = match algorithm.next_digest(&parent_hash) {
						Ok(x) => x,
						Err(err) => {
							warn!(
								target: LOG_TARGET,
								"Unable to propose new block for authoring. \
								 Fetch next digest failed: {}",
								err,
							);
							return Err(err.into())
						},
					};

					if pre_digest.work_type != ProofOfWorkType::Tax {
						return Err(InvalidProofOfWorkTypeUsed.into())
					}

					let sealer = match authority_sealer.check_if_can_seal(
						&parent_hash,
						&block_proof,
						true,
					) {
						Err(err) => {
							warn!(
								target: LOG_TARGET,
								"Unable to propose new block for authoring. \
								 Fetch authorities failed: {}",
								err,
							);
							return Err(err.into())
						},
						Ok(x) => x,
					};

					let mut seal = UlxSeal {
						easing: algorithm.easing(&parent_hash, &block_proof)?,
						nonce: nonce.into(),
						// we will fill this after proposing a block
						authority: None,
					};

					match algorithm.verify(
						&parent_hash,
						// NOTE: this is on purpose! we are going to validate against the parent
						// hash here, so what gets passed in doesn't matter
						&parent_hash,
						&pre_digest,
						&seal,
					) {
						Ok(true) => true,
						Ok(false) => return Err(InvalidNonceDifficulty),
						Err(err) => {
							warn!(
								target: LOG_TARGET,
								"Unable to propose new block for authoring. \
								 Fetch next digest failed: {}",
								err,
							);
							return Err(err.into())
						},
					};

					let proposal = match propose(
						&mut env,
						author.clone(),
						create_inherent_data_providers.clone(),
						build_time,
						&parent_header,
						parent_hash,
						(&client.info().finalized_hash, client.info().finalized_number),
						&pre_digest,
						Some(nonce),
						Some(block_proof.clone()),
					)
					.await
					{
						Some(x) => x,
						None => return Err(Error::BlockProposingError("No proposal".into())),
					};

					let (header, body) = proposal.block.deconstruct();
					let block_number = header.number().clone();

					// NOTE: we will sign the pre-hash
					let seal = match authority_sealer.sign_seal(
						sealer.0,
						&sealer.1,
						&header.hash(),
						&mut seal,
					) {
						Ok(x) => x,
						Err(error) => {
							warn!(target: LOG_TARGET, "Unable to sign seal: {}", error);
							return Err(Error::ConsensusError(error.into()))
						},
					};

					let mut import_block = BlockImportParams::new(BlockOrigin::Own, header);
					import_block.post_digests.push(seal);
					import_block.body = Some(body);
					import_block.state_action = StateAction::ApplyChanges(StorageChanges::Changes(
						proposal.storage_changes,
					));

					let post_hash = import_block.post_hash();
					warn!(target: LOG_TARGET, "Importing block: {:?}", &post_hash);

					match block_import.import_block(import_block).await {
						Ok(res) => match res {
							ImportResult::Imported(_) => {
								res.handle_justification(
									&post_hash,
									block_number,
									&justification_sync_link,
								);

								info!(
									target: LOG_TARGET,
									"✅ Successfully mined 'tax' block on top of: {} -> {}", parent_hash, post_hash
								);
								Ok(CreatedBlock { hash: post_hash })
							},
							other => Err(other.into()),
						},
						Err(err) => {
							warn!(target: LOG_TARGET, "Unable to import 'tax seal' block: {}", err,);
							Err(err.into())
						},
					}
				}
				.map_err(|e| {
					warn!(
						target: LOG_TARGET,
						"Error using imported tax seal: {}",
						e
					);
					RpcError::from(e.to_string())
				});

				if let Some(sender) = sender.take() {
					let _ = sender.send(future.await);
				}
			},
		}
	}
}

pub fn create_compute_miner<Block, C, S, Algorithm, E, SO, L, CIDP>(
	block_import: BoxBlockImport<Block>,
	client: Arc<C>,
	select_chain: S,
	algorithm: Algorithm,
	mut env: E,
	sync_oracle: SO,
	justification_sync_link: L,
	author: AccountId32,
	create_inherent_data_providers: CIDP,
	timeout: Duration,
	build_time: Duration,
) -> (MiningHandle<Block, L, <E::Proposer as Proposer<Block>>::Proof>, impl Future<Output = ()>)
where
	Block: BlockT,
	C: ProvideRuntimeApi<Block> + BlockchainEvents<Block> + HeaderBackend<Block> + 'static,
	S: SelectChain<Block> + 'static,
	Algorithm: NonceAlgorithm<Block> + Send + Clone + Sync + 'static,
	Algorithm::Difficulty: Send + 'static,
	E: Environment<Block> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<Block>,
	SO: SyncOracle + Clone + Send + Sync + 'static,
	L: sc_consensus::JustificationSyncLink<Block> + 'static,
	CIDP: CreateInherentDataProviders<Block, UlxBlockSealInherent> + Clone,
{
	let mut timer = UntilImportedOrTimeout::new(client.import_notification_stream(), timeout);
	let worker = MiningHandle::new(block_import, justification_sync_link);

	let worker_ret = worker.clone();

	let task = async move {
		loop {
			if timer.next().await.is_none() {
				// this should occur if the block import notifications completely stop... indicating
				// we should exit
				break
			}

			if sync_oracle.is_major_syncing() {
				debug!(target: LOG_TARGET, "Skipping proposal due to sync.");
				worker.on_major_syncing();
				continue
			}

			let best_header = match select_chain.best_chain().await {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: LOG_TARGET,
						"Unable to pull new block for authoring. \
						 Select best chain error: {}",
						err
					);
					continue
				},
			};
			let best_hash = best_header.hash();

			if worker.best_hash() == Some(best_hash) {
				continue
			}

			// The worker is locked for the duration of the whole proposing period. Within this
			// period, the mining target is outdated and useless anyway.
			let pre_digest = match algorithm.next_digest(&best_hash) {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: LOG_TARGET,
						"Unable to propose new block for authoring. \
						 Fetch difficulty failed: {}",
						err,
					);
					continue
				},
			};

			let proposal = propose(
				&mut env,
				author.clone(),
				create_inherent_data_providers.clone(),
				build_time,
				&best_header,
				best_hash,
				(&client.info().finalized_hash, client.info().finalized_number),
				&pre_digest,
				None,
				None,
			)
			.await;

			let proposal = match proposal {
				Some(x) => x,
				None => continue,
			};
			let pre_hash = proposal.block.header().hash();

			worker.on_build(proposal, best_hash, pre_hash, pre_digest);
		}
	};

	(worker_ret, task)
}

async fn propose<Block, E, CIDP>(
	env: &mut E,
	author: AccountId32,
	create_inherent_data_providers: CIDP,
	build_time: Duration,
	best_header: &<Block>::Header,
	best_hash: <<Block>::Header as Header>::Hash,
	finalized_block_needed: (&<Block>::Hash, NumberFor<Block>),
	pre_digest: &UlxPreDigest,
	tax_nonce: Option<U256>,
	tax_block_proof: Option<BlockProof>,
) -> Option<Proposal<Block, <E::Proposer as Proposer<Block>>::Proof>>
where
	Block: BlockT,
	E: Environment<Block> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<Block>,
	CIDP: CreateInherentDataProviders<Block, UlxBlockSealInherent>,
{
	let future = async {
		let seal = UlxBlockSealInherent {
			work_type: match tax_block_proof {
				Some(_) => ProofOfWorkType::Tax,
				None => ProofOfWorkType::Compute,
			},
			tax_nonce,
			tax_block_proof,
		};

		let inherent_data_providers = match create_inherent_data_providers
			.create_inherent_data_providers(best_hash, seal)
			.await
		{
			Ok(x) => x,
			Err(err) => {
				warn!(
					target: LOG_TARGET,
					"Unable to propose new block for authoring. \
					 Creating inherent data providers failed: {:?}",
					err,
				);
				return None
			},
		};

		let inherent_data = match inherent_data_providers.create_inherent_data().await {
			Ok(r) => r,
			Err(err) => {
				warn!(
					target: LOG_TARGET,
					"Unable to propose new block for authoring. \
					 Creating inherent data failed: {:?}",
					err,
				);
				return None
			},
		};

		let proposer: E::Proposer = match env.init(&best_header).await {
			Ok(x) => x,
			Err(err) => {
				warn!(
					target: LOG_TARGET,
					"Unable to propose new block for authoring. \
					 Creating proposer failed: {:?}",
					err,
				);
				return None
			},
		};

		let mut inherent_digest = Digest::default();
		// add author in pow standard field (for client)
		inherent_digest.push(DigestItem::PreRuntime(AUTHOR_ID, author.encode().to_vec()));
		inherent_digest.push(DigestItem::PreRuntime(ULX_ENGINE_ID, pre_digest.encode().to_vec()));
		inherent_digest.push(DigestItem::PreRuntime(
			FINALIZED_BLOCK_DIGEST_ID,
			FinalizedBlockNeededDigest::<Block> {
				hash: *finalized_block_needed.0,
				number: finalized_block_needed.1,
			}
			.encode()
			.to_vec(),
		));

		let block = match proposer.propose(inherent_data, inherent_digest, build_time, None).await {
			Ok(x) => x,
			Err(err) => {
				warn!(target: LOG_TARGET, "Unable to propose. Creating proposer failed: {:?}", err);
				return None
			},
		};
		Some(block)
	};
	future.await
}
