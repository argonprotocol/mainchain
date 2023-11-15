use std::{sync::Arc, time::Duration};

use codec::Encode;
use futures::prelude::*;
use log::*;
use sc_consensus::{BlockImportParams, BoxBlockImport, ImportResult, StateAction, StorageChanges};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Environment, Proposer};
use sp_core::{crypto::AccountId32, U256};
use sp_inherents::InherentDataProvider;
use sp_keystore::KeystorePtr;
use sp_runtime::{
	generic::{Digest, DigestItem},
	traits::{Block as BlockT, Header as HeaderT, UniqueSaturatedInto},
};

pub use notebook_state::create_notebook_watch;
use ulx_primitives::{
	digests::{
		BlockVoteDigest, FinalizedBlockNeededDigest, BLOCK_VOTES_DIGEST_ID,
		COMPUTE_AUTHORITY_DIGEST_ID, FINALIZED_BLOCK_DIGEST_ID, SIGNATURE_DIGEST_ID,
	},
	inherents::{BlockSealInherent, BlockSealInherentDataProvider},
	BlockSealAuthorityId, BlockSealDigest, MiningAuthorityApis, AUTHOR_DIGEST_ID,
	BLOCK_SEAL_DIGEST_ID,
};

use crate::{authority::AuthoritySealer, error::Error, notebook_state::CreateBlockEvent};

#[cfg(test)]
mod tests;

pub mod authority;
mod aux;
pub mod basic_queue;
mod basic_queue_import;
mod digests;
pub mod error;
pub mod import_queue;
mod metrics;
pub mod notebook_state;
pub mod rpc_block_votes;

const LOG_TARGET: &str = "node::consensus";

pub async fn block_creator<Block, C, E, L, CS>(
	mut block_import: BoxBlockImport<Block>,
	client: Arc<C>,
	mut env: E,
	justification_sync_link: L,
	author: AccountId32,
	build_time: Duration,
	mut block_create_stream: CS,
	keystore: KeystorePtr,
) where
	Block: BlockT + 'static,
	Block::Hash: Send + 'static,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
	C::Api: MiningAuthorityApis<Block>,
	E: Environment<Block> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<Block>,
	L: sc_consensus::JustificationSyncLink<Block> + 'static,
	CS: Stream<Item = CreateBlockEvent<Block>> + Unpin + 'static,
{
	let authority_sealer = AuthoritySealer::<Block, C>::new(client.clone(), keystore.clone());
	while let Some(command) = block_create_stream.next().await {
		let CreateBlockEvent {
			is_compute_seal,
			block_vote_digest,
			parent_block_number,
			parent_hash,
			seal_inherent,
			block_seal_authority,
			latest_finalized_block_needed,
		} = command;

		info!(target: LOG_TARGET, "Building next block (at={}, parent={:?})", parent_block_number + 1, parent_hash);
		let Some(import_block) = build_block(
			client.clone(),
			&mut env,
			&author,
			parent_hash,
			block_vote_digest,
			is_compute_seal,
			latest_finalized_block_needed.into(),
			seal_inherent,
			block_seal_authority,
			&authority_sealer,
			build_time,
		)
		.map_err(|err| {
			warn!(target: LOG_TARGET, "Unable to build block: {:?}", err);
		})
		.await
		.ok() else {
			continue
		};

		let post_hash = import_block.post_hash();
		let block_number = import_block.header.number().clone();
		warn!(target: LOG_TARGET, "Importing generated block: {:?}", &post_hash);
		match block_import.import_block(import_block).await {
			Ok(res) => match res {
				ImportResult::Imported(_) => {
					res.handle_justification(&post_hash, block_number, &justification_sync_link);

					info!(
						target: LOG_TARGET,
						"✅ Successfully mined block on top of: {} -> {}", parent_hash, post_hash
					);
				},
				other => {
					warn!(target: LOG_TARGET, "Import result not success: {:?}", other);
				},
			},
			Err(err) => {
				warn!(target: LOG_TARGET, "Unable to import block: {:?}", err);
			},
		}
	}
}

async fn build_block<Block, C, E>(
	client: Arc<C>,
	env: &mut E,
	author: &AccountId32,
	parent_hash: Block::Hash,
	block_vote_digest: BlockVoteDigest,
	is_compute_seal: bool,
	latest_finalized_block_needed: <Block::Header as HeaderT>::Number,
	seal_inherent: BlockSealInherent,
	block_seal_authority: BlockSealAuthorityId,
	authority_sealer: &AuthoritySealer<Block, C>,
	build_time: Duration,
) -> Result<BlockImportParams<Block>, Error<Block>>
where
	Block: BlockT + 'static,
	Block::Hash: Send + 'static,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
	C::Api: MiningAuthorityApis<Block>,
	E: Environment<Block> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<Block>,
{
	let best_header = match client.header(parent_hash) {
		Ok(Some(x)) => x,
		Ok(None) => return Err(Error::BlockNotFound(parent_hash.to_string())),
		Err(err) => return Err(err.into()),
	};

	let nonce = match &seal_inherent {
		BlockSealInherent::BestVote { nonce, .. } => nonce.clone(),
		BlockSealInherent::Continuation => U256::MAX,
	};
	let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
	let seal = BlockSealInherentDataProvider::new(seal_inherent);
	let inherent_data = match (timestamp, seal).create_inherent_data().await {
		Ok(r) => r,
		Err(err) => {
			warn!(
				target: LOG_TARGET,
				"Unable to propose new block for authoring. \
				 Creating inherent data failed: {:?}",
				err,
			);
			return Err(err.into())
		},
	};

	let proposer: E::Proposer = match env.init(&best_header).await {
		Ok(x) => x,
		Err(err) => {
			let msg = format!(
				"Unable to propose new block for authoring. \
						Initializing proposer failed: {:?}",
				err
			);
			return Err(Error::StringError(msg))
		},
	};

	let finalized_hash_needed = match client.hash(latest_finalized_block_needed) {
		Ok(Some(x)) => x,
		Ok(None) => return Err(Error::InvalidFinalizedBlockNeeded),
		Err(err) => return Err(err.into()),
	};

	let mut inherent_digest = Digest::default();
	if is_compute_seal {
		inherent_digest.push(DigestItem::PreRuntime(
			COMPUTE_AUTHORITY_DIGEST_ID,
			block_seal_authority.encode(),
		));
	}
	// add author in pow standard field (for client)
	inherent_digest.push(DigestItem::PreRuntime(AUTHOR_DIGEST_ID, author.encode()));
	inherent_digest.push(DigestItem::PreRuntime(
		FINALIZED_BLOCK_DIGEST_ID,
		FinalizedBlockNeededDigest::<Block> {
			hash: finalized_hash_needed,
			number: latest_finalized_block_needed,
		}
		.encode(),
	));
	inherent_digest
		.push(DigestItem::PreRuntime(BLOCK_SEAL_DIGEST_ID, BlockSealDigest { nonce }.encode()));
	inherent_digest.push(DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, block_vote_digest.encode()));

	let proposal = match proposer.propose(inherent_data, inherent_digest, build_time, None).await {
		Ok(x) => x,
		Err(err) => {
			let msg = format!("Unable to propose. Creating proposer failed: {:?}", err);
			return Err(Error::StringError(msg))
		},
	};

	let (header, body) = proposal.block.deconstruct();
	let pre_hash = &header.hash();

	let mut import_block = BlockImportParams::new(BlockOrigin::Own, header);

	let signature = authority_sealer.sign_message(&block_seal_authority, &pre_hash.as_ref())?;

	import_block
		.post_digests
		.push(DigestItem::Seal(SIGNATURE_DIGEST_ID, signature.encode()));

	import_block.body = Some(body);
	import_block.state_action =
		StateAction::ApplyChanges(StorageChanges::Changes(proposal.storage_changes));

	Ok(import_block)
}

pub(crate) fn convert_u32<Block: BlockT>(number: &<Block::Header as HeaderT>::Number) -> u32 {
	UniqueSaturatedInto::<u32>::unique_saturated_into(number.clone())
}
