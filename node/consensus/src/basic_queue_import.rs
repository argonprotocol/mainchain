use log::{debug, trace};
use sc_consensus::{
	BlockCheckParams, BlockImport, BlockImportError, BlockImportParams, BlockImportStatus,
	ImportResult, IncomingBlock, StateAction, Verifier,
};
use sp_consensus::{BlockOrigin, Error as ConsensusError};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};

use crate::{error::Error, metrics::Metrics};

///
///
/// NOTE!!! Modified from substrate block queue. This has only minor modifications!! Mark changes
/// with ULIXEE MODIFICATION

pub(crate) enum ImportOrFinalizeError<B: BlockT> {
	BlockImportError(BlockImportError),
	FinalizedBlockNeeded(B::Hash, NumberFor<B>),
}
const LOG_TARGET: &str = "node::consensus::import_block";

impl<B: BlockT> From<BlockImportError> for ImportOrFinalizeError<B> {
	fn from(value: BlockImportError) -> Self {
		ImportOrFinalizeError::BlockImportError(value)
	}
}
/// Single block import function with metering.
pub(crate) async fn import_single_block_metered<B: BlockT, V: Verifier<B>>(
	import_handle: &mut impl BlockImport<B, Error = ConsensusError>,
	block_origin: BlockOrigin,
	block: IncomingBlock<B>,
	verifier: &mut V,
	metrics: Option<Metrics>,
) -> Result<BlockImportStatus<NumberFor<B>>, ImportOrFinalizeError<B>> {
	let peer = block.origin;

	let (header, justifications) = match (block.header, block.justifications) {
		(Some(header), justifications) => (header, justifications),
		(None, _) => {
			if let Some(ref peer) = peer {
				debug!(target: LOG_TARGET, "Header {} was not provided by {} ", block.hash, peer);
			} else {
				debug!(target: LOG_TARGET, "Header {} was not provided ", block.hash);
			}
			return Err(BlockImportError::IncompleteHeader(peer).into())
		},
	};

	trace!(target: LOG_TARGET, "Header {} has {:?} logs", block.hash, header.digest().logs().len());

	let number = *header.number();
	let hash = block.hash;
	let parent_hash = *header.parent_hash();

	let import_handler = |import| match import {
		Ok(ImportResult::AlreadyInChain) => {
			trace!(target: LOG_TARGET, "Block already in chain {}: {:?}", number, hash);
			Ok(BlockImportStatus::ImportedKnown(number, peer))
		},
		Ok(ImportResult::Imported(aux)) =>
			Ok(BlockImportStatus::ImportedUnknown(number, aux, peer)),
		Ok(ImportResult::MissingState) => {
			debug!(
				target: LOG_TARGET,
				"Parent state is missing for {}: {:?}, parent: {:?}", number, hash, parent_hash
			);
			Err(BlockImportError::MissingState.into())
		},
		Ok(ImportResult::UnknownParent) => {
			debug!(
				target: LOG_TARGET,
				"Block with unknown parent {}: {:?}, parent: {:?}", number, hash, parent_hash
			);
			Err(BlockImportError::UnknownParent.into())
		},
		Ok(ImportResult::KnownBad) => {
			debug!(target: LOG_TARGET, "Peer gave us a bad block {}: {:?}", number, hash);
			Err(BlockImportError::BadBlock(peer).into())
		},
		Err(e) => {
			debug!(target: LOG_TARGET, "Error importing block {}: {:?}: {}", number, hash, e);
			Err(BlockImportError::Other(e).into())
		},
	};

	match import_handler(
		import_handle
			.check_block(BlockCheckParams {
				hash,
				number,
				parent_hash,
				allow_missing_state: block.allow_missing_state,
				import_existing: block.import_existing,
				allow_missing_parent: block.state.is_some(),
			})
			.await,
	)? {
		BlockImportStatus::ImportedUnknown { .. } => (),
		r => return Ok(r), // Any other successful result means that the block is already imported.
	}

	let started = std::time::Instant::now();

	let mut import_block = BlockImportParams::new(block_origin, header);
	import_block.body = block.body;
	import_block.justifications = justifications;
	import_block.post_hash = Some(hash);
	import_block.import_existing = block.import_existing;
	import_block.indexed_body = block.indexed_body;

	if let Some(state) = block.state {
		let changes = sc_consensus::block_import::StorageChanges::Import(state);
		import_block.state_action = StateAction::ApplyChanges(changes);
	} else if block.skip_execution {
		import_block.state_action = StateAction::Skip;
	} else if block.allow_missing_state {
		import_block.state_action = StateAction::ExecuteIfPossible;
	}

	let import_block = verifier.verify(import_block).await.map_err(|msg| {
		if let Some(ref peer) = peer {
			trace!(
				target: LOG_TARGET,
				"Verifying {}({}) from {} failed: {}",
				number,
				hash,
				peer,
				msg
			);
		} else {
			trace!(target: LOG_TARGET, "Verifying {}({}) failed: {}", number, hash, msg);
		}
		if let Some(metrics) = metrics.as_ref() {
			metrics.report_verification(false, started.elapsed());
		}

		BlockImportError::VerificationFailed(peer, msg)
	})?;

	if let Some(metrics) = metrics.as_ref() {
		metrics.report_verification(true, started.elapsed());
	}

	let imported = import_handle.import_block(import_block).await;
	if let Some(metrics) = metrics.as_ref() {
		metrics.report_verification_and_import(started.elapsed());
	}

	// ULIXEE MODIFICATION
	if let Err(ConsensusError::Other(o)) = &imported {
		if let Some(Error::<B>::PendingFinalizedBlockDigest(hash, num)) =
			o.downcast_ref::<Error<B>>()
		{
			return Err(ImportOrFinalizeError::<B>::FinalizedBlockNeeded(*hash, *num))
		}
	}

	import_handler(imported)
}
