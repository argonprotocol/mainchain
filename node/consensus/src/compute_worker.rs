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

use std::{
	pin::Pin,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
	thread,
	thread::JoinHandle,
	time::Duration,
};

use codec::Encode;
use futures::{
	executor::block_on,
	prelude::*,
	task::{Context, Poll},
};
use futures_timer::Delay;
use log::*;
use parking_lot::Mutex;
use sc_client_api::ImportNotifications;
use sc_consensus::{BlockImportParams, BoxBlockImport, ImportResult, StateAction, StorageChanges};
use sp_consensus::{BlockOrigin, Proposal};
use sp_core::{RuntimeDebug, U256};
use sp_runtime::{
	traits::{Block as BlockT, Header as HeaderT},
	DigestItem,
};

use ulx_primitives::*;

use crate::{nonce_verify::NonceVerifier, LOG_TARGET, ULX_ENGINE_ID};

pub enum NonceSolverResult {
	Found { nonce: [u8; 32] },
	MovedToTax,
	NotFound,
	Waiting,
}

pub struct NonceSolver<B: BlockT> {
	pub version: Version,
	pub nonce: [u8; 32],
	counter: U256,
	verifier: NonceVerifier<B>,
}

impl<B: BlockT> NonceSolver<B> {
	pub fn new(version: Version, pre_hash: &B::Hash, pre_digest: &UlxPreDigest) -> Self {
		NonceSolver {
			version,
			counter: U256::from(rand::random::<u64>()),
			nonce: [0_u8; 32],
			verifier: NonceVerifier::new(pre_hash, pre_digest),
		}
	}

	pub fn check_next(&mut self) -> NonceSolverResult {
		self.counter = self.counter.checked_add(U256::from(1)).unwrap_or(U256::from(0));

		self.counter.to_big_endian(&mut self.nonce);
		if self.verifier.is_nonce_valid(&self.nonce) {
			NonceSolverResult::Found { nonce: self.nonce }
		} else {
			NonceSolverResult::NotFound
		}
	}
}

/// Mining metadata. This is the information needed to start an actual mining loop.
#[derive(Clone, Eq, PartialEq)]
pub struct MiningMetadata<H> {
	/// Currently known best hash which the pre-hash is built on.
	pub best_hash: H,
	/// Mining pre-hash.
	pub pre_hash: H,
	/// Pre-runtime digest item.
	pub pre_digest: UlxPreDigest,
}

/// A build of mining, containing the metadata and the block proposal.
pub struct MiningBuild<Block: BlockT, Proof> {
	/// Mining metadata.
	pub metadata: MiningMetadata<Block::Hash>,
	/// Mining proposal.
	pub proposal: Proposal<Block, Proof>,
}

/// Version of the mining worker.
#[derive(Eq, PartialEq, Clone, Copy, RuntimeDebug)]
pub struct Version(pub usize);

/// Mining worker that exposes structs to query the current mining build and submit mined blocks.
pub struct MiningHandle<Block: BlockT, L: sc_consensus::JustificationSyncLink<Block>, Proof> {
	version: Arc<AtomicUsize>,
	justification_sync_link: Arc<L>,
	build: Arc<Mutex<Option<MiningBuild<Block, Proof>>>>,
	block_import: Arc<Mutex<BoxBlockImport<Block>>>,
}

impl<Block, L, Proof> MiningHandle<Block, L, Proof>
where
	Block: BlockT,
	L: sc_consensus::JustificationSyncLink<Block>,
	Proof: Send,
{
	fn increment_version(&self) {
		self.version.fetch_add(1, Ordering::SeqCst);
	}

	pub fn new(block_import: BoxBlockImport<Block>, justification_sync_link: L) -> Self {
		Self {
			version: Arc::new(AtomicUsize::new(0)),
			justification_sync_link: Arc::new(justification_sync_link),
			build: Arc::new(Mutex::new(None)),
			block_import: Arc::new(Mutex::new(block_import)),
		}
	}

	pub(crate) fn on_major_syncing(&self) {
		let mut build = self.build.lock();
		*build = None;
		self.increment_version();
	}

	pub(crate) fn on_build(
		&self,
		proposal: Proposal<Block, Proof>,
		best_hash: Block::Hash,
		pre_hash: Block::Hash,
		pre_digest: UlxPreDigest,
	) {
		let mut build = self.build.lock();
		*build = Some(MiningBuild::<Block, _> {
			metadata: MiningMetadata {
				best_hash,
				pre_hash: pre_hash.clone(),
				pre_digest: pre_digest.clone(),
			},
			proposal,
		});
		self.increment_version();
	}
	/// Get the version of the mining worker.
	///
	/// This returns type `Version` which can only compare equality. If `Version` is unchanged, then
	/// it can be certain that `best_hash` and `metadata` were not changed.
	pub fn version(&self) -> Version {
		Version(self.version.load(Ordering::SeqCst))
	}

	/// Get the current best hash. `None` if the worker has just started or the client is doing
	/// major syncing.
	pub fn best_hash(&self) -> Option<Block::Hash> {
		self.build.lock().as_ref().map(|b| b.metadata.best_hash)
	}

	/// Get a copy of the current mining metadata, if available.
	pub fn metadata(&self) -> Option<MiningMetadata<Block::Hash>> {
		self.build.lock().as_ref().map(|b| b.metadata.clone())
	}

	pub fn create_solver(&self) -> Option<NonceSolver<Block>> {
		match self.metadata() {
			Some(x) => {
				let pre_hash = x.pre_hash;
				let pre_digest = x.pre_digest;
				Some(NonceSolver::new(self.version(), &pre_hash, &pre_digest))
			},
			_ => None,
		}
	}

	pub fn is_valid_solver(&self, solver: &Option<Box<NonceSolver<Block>>>) -> bool {
		solver.as_ref().map(|a| a.version) == Some(self.version())
	}

	pub async fn submit(
		&self,
		nonce: [u8; 32],
	) -> Result<Option<ImportResult>, crate::Error<Block>> {
		let build = match {
			let mut build = self.build.lock();
			// try to take out of option. if not exists, we've moved on
			build.take()
		} {
			Some(x) => x,
			_ => {
				warn!(target: LOG_TARGET, "Unable to submit mined block: build does not exist",);
				return Ok(None)
			},
		};

		let seal =
			DigestItem::Seal(ULX_ENGINE_ID, UlxSeal { nonce, authority: None, easing: 0 }.encode());
		let storage_changes = build.proposal.storage_changes;
		let (header, body) = build.proposal.block.deconstruct();

		let mut import_block = BlockImportParams::new(BlockOrigin::Own, header);
		import_block.post_digests.push(seal);
		import_block.body = Some(body);
		import_block.state_action =
			StateAction::ApplyChanges(StorageChanges::Changes(storage_changes));

		let header = import_block.post_header();
		let mut block_import = self.block_import.lock();

		match block_import.import_block(import_block).await {
			Ok(res) => match res {
				ImportResult::Imported(_) => {
					res.handle_justification(
						&header.hash(),
						*header.number(),
						&self.justification_sync_link,
					);

					info!(
						target: LOG_TARGET,
						"✅ Successfully mined block on top of: {}", build.metadata.best_hash
					);
					Ok(Some(res))
				},
				other => Err(other.into()),
			},
			Err(err) => {
				warn!(target: LOG_TARGET, "Unable to import mined block: {}", err,);
				Err(err.into())
			},
		}
	}
}

pub fn run_miner_thread<Block, L, Proof>(worker: MiningHandle<Block, L, Proof>) -> JoinHandle<()>
where
	Block: BlockT,
	L: sc_consensus::JustificationSyncLink<Block> + 'static,
	Proof: Send + 'static,
{
	let mut solver: Option<Box<NonceSolver<Block>>> = None;
	thread::spawn(move || loop {
		if !worker.is_valid_solver(&solver) {
			if let Some(finder) = worker.create_solver() {
				solver = Some(Box::new(finder));
			}
		}

		if let Some(mut_solver) = solver.as_mut() {
			match mut_solver.check_next() {
				NonceSolverResult::Found { nonce } => {
					let _ = block_on(worker.submit(nonce));
					solver = None;
				},
				NonceSolverResult::NotFound => continue,
				NonceSolverResult::MovedToTax => {
					log::info!(
							target: LOG_TARGET,
							"Proof of Tax is activated, leaving mining thread.");
					return
				},
				NonceSolverResult::Waiting => {
					thread::sleep(Duration::new(1, 0));
				},
			}
		} else {
			thread::sleep(Duration::from_millis(500));
		}
	})
}

impl<Block, L, Proof> Clone for MiningHandle<Block, L, Proof>
where
	Block: BlockT,
	L: sc_consensus::JustificationSyncLink<Block>,
{
	fn clone(&self) -> Self {
		Self {
			version: self.version.clone(),
			justification_sync_link: self.justification_sync_link.clone(),
			build: self.build.clone(),
			block_import: self.block_import.clone(),
		}
	}
}

/// A stream that waits for a block import or timeout.
pub struct UntilImportedOrTimeout<Block: BlockT> {
	import_notifications: ImportNotifications<Block>,
	timeout: Duration,
	inner_delay: Option<Delay>,
}

impl<Block: BlockT> UntilImportedOrTimeout<Block> {
	/// Create a new stream using the given import notification and timeout duration.
	pub fn new(import_notifications: ImportNotifications<Block>, timeout: Duration) -> Self {
		Self { import_notifications, timeout, inner_delay: None }
	}
}

impl<Block: BlockT> Stream for UntilImportedOrTimeout<Block> {
	type Item = ();

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<()>> {
		let mut fire = false;

		loop {
			match Stream::poll_next(Pin::new(&mut self.import_notifications), cx) {
				Poll::Pending => break,
				Poll::Ready(Some(_)) => {
					fire = true;
				},
				Poll::Ready(None) => return Poll::Ready(None),
			}
		}

		let timeout = self.timeout;
		let inner_delay = self.inner_delay.get_or_insert_with(|| Delay::new(timeout));

		match Future::poll(Pin::new(inner_delay), cx) {
			Poll::Pending => (),
			Poll::Ready(()) => {
				fire = true;
			},
		}

		if fire {
			self.inner_delay = None;
			Poll::Ready(Some(()))
		} else {
			Poll::Pending
		}
	}
}
