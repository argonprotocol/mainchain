#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use alloc::{collections::BTreeMap, vec, vec::Vec};
use argon_primitives::{
	NotaryId, NotebookNumber, NotebookSecret, VotingSchedule,
	block_seal::MiningAuthority,
	digests::{
		AUTHOR_DIGEST_ID, BLOCK_VOTES_DIGEST_ID, BlockVoteDigest, NOTEBOOKS_DIGEST_ID,
		NotebookDigest, TICK_DIGEST_ID,
	},
	notary::{NotaryProvider, NotarySignature},
	providers::{AuthorityProvider, NotebookProvider},
	tick::{Tick, TickDigest, Ticker},
};
use codec::Decode;
use core::marker::PhantomData;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BenchmarkNotebookProviderCallCounters {
	pub notebooks_in_block: u32,
	pub vote_eligible_notebook_count: u32,
	pub eligible_notebooks_for_vote: u32,
	pub get_eligible_tick_votes_root: u32,
	pub is_notary_locked_at_tick: u32,
}

#[derive(Clone, Default)]
pub struct BenchmarkNotebookProviderState {
	pub notebooks_in_block: Vec<(NotaryId, NotebookNumber, Tick)>,
	pub notebooks_by_tick: BTreeMap<Tick, Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)>>,
	pub eligible_vote_roots: BTreeMap<(NotaryId, Tick), (H256, NotebookNumber)>,
	pub notary_locked_from_tick: BTreeMap<NotaryId, Tick>,
	pub call_counters: BenchmarkNotebookProviderCallCounters,
}

pub fn set_all_digests<T, VerifyError>(
	author: T::AccountId,
	tick: Tick,
	block_vote: BlockVoteDigest,
	notebooks: NotebookDigest<VerifyError>,
) where
	T: frame_system::Config,
	T::AccountId: Codec,
	VerifyError: Codec + MaxEncodedLen,
{
	let digest = frame_system::Pallet::<T>::digest();
	let has_author = digest
		.logs
		.iter()
		.any(|entry| matches!(entry, DigestItem::PreRuntime(id, _) if *id == AUTHOR_DIGEST_ID));
	let has_tick = digest
		.logs
		.iter()
		.any(|entry| matches!(entry, DigestItem::PreRuntime(id, _) if *id == TICK_DIGEST_ID));
	let has_votes = digest.logs.iter().any(
		|entry| matches!(entry, DigestItem::PreRuntime(id, _) if *id == BLOCK_VOTES_DIGEST_ID),
	);
	let has_notebooks = digest
		.logs
		.iter()
		.any(|entry| matches!(entry, DigestItem::PreRuntime(id, _) if *id == NOTEBOOKS_DIGEST_ID));

	if !has_author {
		frame_system::Pallet::<T>::deposit_log(DigestItem::PreRuntime(
			AUTHOR_DIGEST_ID,
			author.encode(),
		));
	}
	if !has_tick {
		frame_system::Pallet::<T>::deposit_log(DigestItem::PreRuntime(
			TICK_DIGEST_ID,
			TickDigest(tick).encode(),
		));
	}
	if !has_votes {
		frame_system::Pallet::<T>::deposit_log(DigestItem::PreRuntime(
			BLOCK_VOTES_DIGEST_ID,
			block_vote.encode(),
		));
	}
	if !has_notebooks {
		frame_system::Pallet::<T>::deposit_log(DigestItem::PreRuntime(
			NOTEBOOKS_DIGEST_ID,
			notebooks.encode(),
		));
	}
}

pub struct BenchmarkNotaryProvider;
impl<B: BlockT, AccountId: Decode> NotaryProvider<B, AccountId> for BenchmarkNotaryProvider {
	type Weights = ();

	fn verify_signature(
		_notary_id: NotaryId,
		_at_tick: Tick,
		_message: &H256,
		_signature: &NotarySignature,
	) -> bool {
		// RFC 8032 test vector #1 (empty message): real ed25519 verify cost without runtime state
		// dependencies.
		let public = polkadot_sdk::sp_core::ed25519::Public::from_raw([
			0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
			0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
			0xf7, 0x07, 0x51, 0x1a,
		]);
		let signature = NotarySignature::from_raw([
			0xe5, 0x56, 0x43, 0x00, 0xc3, 0x60, 0xac, 0x72, 0x90, 0x86, 0xe2, 0xcc, 0x80, 0x6e,
			0x82, 0x8a, 0x84, 0x87, 0x7f, 0x1e, 0xb8, 0xe5, 0xd9, 0x74, 0xd8, 0x73, 0xe0, 0x65,
			0x22, 0x49, 0x01, 0x55, 0x5f, 0xb8, 0x82, 0x15, 0x90, 0xa3, 0x3b, 0xac, 0xc6, 0x1e,
			0x39, 0x70, 0x1c, 0xf9, 0xb4, 0x6b, 0xd2, 0x5b, 0xf5, 0xf0, 0x59, 0x5b, 0xbe, 0x24,
			0x65, 0x51, 0x41, 0x43, 0x8e, 0x7a, 0x10, 0x0b,
		]);
		polkadot_sdk::sp_io::crypto::ed25519_verify(&signature, &[], &public)
	}

	fn active_notaries() -> Vec<NotaryId> {
		vec![1]
	}

	fn notary_operator_account_id(_notary_id: NotaryId) -> Option<AccountId> {
		let bytes = [0u8; 32];
		AccountId::decode(&mut &bytes[..]).ok()
	}
}

fn benchmark_authority_account_id<AccountId: Decode>() -> AccountId {
	let bytes = [1u8; 32];
	AccountId::decode(&mut &bytes[..]).expect("benchmark authority account id must decode")
}

fn benchmark_authority_id<AuthorityId: Decode>() -> AuthorityId {
	let bytes = [2u8; 32];
	AuthorityId::decode(&mut &bytes[..]).expect("benchmark authority id must decode")
}

pub struct BenchmarkAuthorityProvider<T, AuthorityId>(PhantomData<(T, AuthorityId)>);
impl<T, AuthorityId> AuthorityProvider<AuthorityId, T::Block, T::AccountId>
	for BenchmarkAuthorityProvider<T, AuthorityId>
where
	T: frame_system::Config,
	T::AccountId: Decode + PartialEq,
	AuthorityId: Decode + PartialEq,
{
	fn authority_count() -> u32 {
		1
	}

	fn get_authority(author: T::AccountId) -> Option<AuthorityId> {
		if author == benchmark_authority_account_id::<T::AccountId>() {
			Some(benchmark_authority_id::<AuthorityId>())
		} else {
			None
		}
	}

	fn get_winning_managed_authority(
		seal_proof: U256,
		signing_key: Option<AuthorityId>,
		best_miner_nonce_score: Option<U256>,
	) -> Option<(MiningAuthority<AuthorityId, T::AccountId>, U256, Permill)> {
		let authority_id = benchmark_authority_id::<AuthorityId>();
		if let Some(expected_authority) = signing_key {
			if expected_authority != authority_id {
				return None;
			}
		}
		let score = seal_proof;
		if let Some(best_score) = best_miner_nonce_score {
			if best_score <= score {
				return None;
			}
		}
		Some((
			MiningAuthority {
				authority_index: (1, 0),
				authority_id,
				account_id: benchmark_authority_account_id::<T::AccountId>(),
			},
			score,
			Permill::zero(),
		))
	}

	fn get_authority_score(
		seal_proof: U256,
		authority_id: &AuthorityId,
		account: &T::AccountId,
		_ref_block_number: NumberFor<T::Block>,
	) -> Option<U256> {
		if *authority_id == benchmark_authority_id::<AuthorityId>() &&
			*account == benchmark_authority_account_id::<T::AccountId>()
		{
			Some(seal_proof)
		} else {
			None
		}
	}
}

fn notebook_entries_from_digests<T: frame_system::Config>() -> Vec<(NotaryId, NotebookNumber, Tick)>
{
	let digest = frame_system::Pallet::<T>::digest();
	for entry in digest.logs.iter() {
		if let DigestItem::PreRuntime(id, data) = entry {
			if *id == NOTEBOOKS_DIGEST_ID {
				if let Ok(notebook_digest) = NotebookDigest::<()>::decode(&mut &data[..]) {
					return notebook_digest
						.notebooks
						.into_inner()
						.into_iter()
						.map(|notebook| {
							(notebook.notary_id, notebook.notebook_number, notebook.tick)
						})
						.collect();
				}
			}
		}
	}
	Vec::new()
}

pub struct BenchmarkNotebookProvider<T>(PhantomData<T>);
impl<T: frame_system::Config> NotebookProvider for BenchmarkNotebookProvider<T> {
	type Weights = ();

	fn get_eligible_tick_votes_root(
		notary_id: NotaryId,
		tick: Tick,
	) -> Option<(H256, NotebookNumber)> {
		let mut state = benchmark_notebook_provider_state();
		state.call_counters.get_eligible_tick_votes_root =
			state.call_counters.get_eligible_tick_votes_root.saturating_add(1);
		let mut result = state.eligible_vote_roots.get(&(notary_id, tick)).copied();
		if result.is_none() {
			result = notebook_entries_from_digests::<T>()
				.into_iter()
				.find(|(digest_notary_id, _, digest_tick)| {
					*digest_notary_id == notary_id && *digest_tick == tick
				})
				.map(|(_, notebook_number, _)| {
					(synthetic_benchmark_votes_root(notary_id, tick), notebook_number)
				});
		}
		set_benchmark_notebook_provider_state(state);
		result
	}

	fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)> {
		let mut state = benchmark_notebook_provider_state();
		state.call_counters.notebooks_in_block =
			state.call_counters.notebooks_in_block.saturating_add(1);
		let notebooks = if state.notebooks_in_block.is_empty() {
			notebook_entries_from_digests::<T>()
		} else {
			state.notebooks_in_block.clone()
		};
		set_benchmark_notebook_provider_state(state);
		notebooks
	}

	fn vote_eligible_notebook_count(voting_schedule: &VotingSchedule) -> u32 {
		let mut state = benchmark_notebook_provider_state();
		state.call_counters.vote_eligible_notebook_count =
			state.call_counters.vote_eligible_notebook_count.saturating_add(1);
		let notebook_tick = voting_schedule.notebook_tick();
		let count = if state.notebooks_by_tick.contains_key(&notebook_tick) {
			state
				.notebooks_by_tick
				.get(&notebook_tick)
				.map(|entries| entries.len() as u32)
				.unwrap_or_default()
		} else {
			notebook_entries_from_digests::<T>()
				.into_iter()
				.filter(|(_, _, tick)| *tick == notebook_tick)
				.count() as u32
		};
		set_benchmark_notebook_provider_state(state);
		count
	}

	fn eligible_notebooks_for_vote(
		voting_schedule: &VotingSchedule,
	) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)> {
		let mut state = benchmark_notebook_provider_state();
		state.call_counters.eligible_notebooks_for_vote =
			state.call_counters.eligible_notebooks_for_vote.saturating_add(1);
		let notebook_tick = voting_schedule.notebook_tick();
		let notebooks = if let Some(notebooks) = state.notebooks_by_tick.get(&notebook_tick) {
			notebooks.clone()
		} else {
			notebook_entries_from_digests::<T>()
				.into_iter()
				.filter(|(_, _, tick)| *tick == notebook_tick)
				.map(|(notary_id, notebook_number, _)| {
					(notary_id, notebook_number, Some(synthetic_benchmark_parent_secret(notary_id)))
				})
				.collect()
		};
		set_benchmark_notebook_provider_state(state);
		notebooks
	}

	fn is_notary_locked_at_tick(notary_id: NotaryId, tick: Tick) -> bool {
		let mut state = benchmark_notebook_provider_state();
		state.call_counters.is_notary_locked_at_tick =
			state.call_counters.is_notary_locked_at_tick.saturating_add(1);
		let is_locked = state
			.notary_locked_from_tick
			.get(&notary_id)
			.map(|locked_from_tick| *locked_from_tick <= tick)
			.unwrap_or(false);
		set_benchmark_notebook_provider_state(state);
		is_locked
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

pub fn set_benchmark_notebook_provider_state(state: BenchmarkNotebookProviderState) {
	state_backend::set(state);
}

pub fn reset_benchmark_notebook_provider_state() {
	state_backend::reset();
}

pub fn benchmark_notebook_provider_state() -> BenchmarkNotebookProviderState {
	state_backend::get()
}

pub fn benchmark_notebook_provider_call_counters() -> BenchmarkNotebookProviderCallCounters {
	benchmark_notebook_provider_state().call_counters
}

pub fn reset_benchmark_notebook_provider_call_counters() {
	let mut state = benchmark_notebook_provider_state();
	state.call_counters = BenchmarkNotebookProviderCallCounters::default();
	set_benchmark_notebook_provider_state(state);
}

pub fn synthetic_benchmark_votes_root(notary_id: NotaryId, tick: Tick) -> H256 {
	let mut bytes = [0u8; 32];
	bytes[..4].copy_from_slice(&notary_id.to_be_bytes());
	let tick_bytes = tick.to_be_bytes();
	bytes[4..(4 + tick_bytes.len())].copy_from_slice(&tick_bytes);
	H256::from(bytes)
}

pub fn synthetic_benchmark_parent_secret(notary_id: NotaryId) -> NotebookSecret {
	H256::repeat_byte((notary_id % 251) as u8 + 1)
}

#[cfg(feature = "std")]
mod state_backend {
	use super::*;
	use frame_support::parameter_types;
	parameter_types! {
		pub static BenchmarkNotebookProviderStateHolder: BenchmarkNotebookProviderState =
			BenchmarkNotebookProviderState::default();
	}

	pub(super) fn set(state: BenchmarkNotebookProviderState) {
		BenchmarkNotebookProviderStateHolder::set(state);
	}

	pub(super) fn reset() {
		BenchmarkNotebookProviderStateHolder::reset();
	}

	pub(super) fn get() -> BenchmarkNotebookProviderState {
		BenchmarkNotebookProviderStateHolder::get()
	}
}

#[cfg(not(feature = "std"))]
mod state_backend {
	use super::*;
	use core::cell::UnsafeCell;

	// Runtime-benchmarks-only in-memory state for WASM execution (no `std`, so no
	// `parameter_types! { static ... }`/`thread_local!`). This is never enabled in production
	// runtime builds.
	struct BenchmarkStateCell(UnsafeCell<Option<BenchmarkNotebookProviderState>>);
	unsafe impl Sync for BenchmarkStateCell {}

	static BENCHMARK_NOTEBOOK_PROVIDER_STATE: BenchmarkStateCell =
		BenchmarkStateCell(UnsafeCell::new(None));

	pub(super) fn set(state: BenchmarkNotebookProviderState) {
		unsafe {
			*BENCHMARK_NOTEBOOK_PROVIDER_STATE.0.get() = Some(state);
		}
	}

	pub(super) fn reset() {
		set(BenchmarkNotebookProviderState::default());
	}

	pub(super) fn get() -> BenchmarkNotebookProviderState {
		unsafe { (*BENCHMARK_NOTEBOOK_PROVIDER_STATE.0.get()).clone().unwrap_or_default() }
	}
}
