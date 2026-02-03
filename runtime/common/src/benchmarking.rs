//! Benchmark-only runtime stubs and helpers.
#![cfg(feature = "runtime-benchmarks")]

use alloc::{vec, vec::Vec};
use polkadot_sdk::{
	sp_core::H256,
	sp_runtime::{DispatchError, traits::Block as BlockT},
};

use argon_bitcoin::CosignReleaser;
use argon_primitives::{
	NotaryId, NotebookNumber, NotebookSecret, VotingSchedule,
	bitcoin::{BitcoinSignature, CompressedBitcoinPubkey},
	notary::{NotaryProvider, NotarySignature},
	providers::NotebookProvider,
	tick::{Tick, Ticker},
};
use pallet_bitcoin_locks::BitcoinVerifier;

pub struct BenchmarkNotaryProvider;
impl<B: BlockT, AccountId> NotaryProvider<B, AccountId> for BenchmarkNotaryProvider {
	fn verify_signature(
		_notary_id: NotaryId,
		_at_tick: Tick,
		_message: &H256,
		_signature: &NotarySignature,
	) -> bool {
		true
	}

	fn active_notaries() -> Vec<NotaryId> {
		vec![1]
	}

	fn notary_operator_account_id(_notary_id: NotaryId) -> Option<AccountId> {
		None
	}
}

pub struct BenchmarkNotebookProvider;
impl NotebookProvider for BenchmarkNotebookProvider {
	fn get_eligible_tick_votes_root(
		_notary_id: NotaryId,
		_tick: Tick,
	) -> Option<(H256, NotebookNumber)> {
		None
	}

	fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)> {
		Vec::new()
	}

	fn notebooks_at_tick(_tick: Tick) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)> {
		Vec::new()
	}

	fn is_notary_locked_at_tick(_notary_id: NotaryId, _tick: Tick) -> bool {
		false
	}
}

pub struct BenchmarkTickProvider;
impl<B: BlockT> argon_primitives::TickProvider<B> for BenchmarkTickProvider {
	fn previous_tick() -> Tick {
		14
	}

	fn current_tick() -> Tick {
		15
	}

	fn elapsed_ticks() -> Tick {
		10
	}

	fn voting_schedule() -> VotingSchedule {
		VotingSchedule::from_runtime_current_tick(
			<Self as argon_primitives::TickProvider<B>>::current_tick(),
		)
	}

	fn ticker() -> Ticker {
		Ticker::new(1000, 10)
	}

	fn blocks_at_tick(_tick: Tick) -> Vec<B::Hash> {
		Vec::new()
	}
}

pub struct BenchmarkBitcoinSignatureVerifier;
impl<T: pallet_bitcoin_locks::Config> BitcoinVerifier<T> for BenchmarkBitcoinSignatureVerifier {
	fn verify_signature(
		_utxo_releaser: CosignReleaser,
		_pubkey: CompressedBitcoinPubkey,
		_signature: &BitcoinSignature,
	) -> Result<bool, DispatchError> {
		Ok(true)
	}
}
