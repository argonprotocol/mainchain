//! Benchmarking setup for pallet-notebook
#![cfg(feature = "runtime-benchmarks")]

use super::*;
#[allow(unused_imports)]
use crate::Pallet as NotebookPallet;
use argon_primitives::{
	NotebookAuditResult, SignedNotebookHeader, TickProvider,
	digests::{BlockVoteDigest, NotebookDigest},
	notary::NotaryProvider,
	notebook::{AccountOrigin, ChainTransfer, NotebookHeader},
	providers::NotebookProvider,
};
use frame_system::RawOrigin;
use pallet_prelude::benchmarking::set_all_digests;
use polkadot_sdk::{
	frame_benchmarking::v2::*,
	frame_support::BoundedVec,
	sp_core::{H256, ed25519},
};

fn notebook_digest(headers: &[SignedNotebookHeader]) -> NotebookDigest<NotebookVerifyError> {
	NotebookDigest {
		notebooks: BoundedVec::truncate_from(
			headers
				.iter()
				.map(|entry| NotebookAuditResult {
					notary_id: entry.header.notary_id,
					notebook_number: entry.header.notebook_number,
					tick: entry.header.tick,
					audit_first_failure: None,
				})
				.collect::<Vec<_>>(),
		),
	}
}

fn notebook_digest_from_count(
	n: u32,
	tick: Tick,
	notebook_number: NotebookNumber,
) -> NotebookDigest<NotebookVerifyError> {
	NotebookDigest {
		notebooks: BoundedVec::truncate_from(
			(0..n)
				.map(|index| NotebookAuditResult {
					notary_id: index.saturating_add(1),
					notebook_number,
					tick,
					audit_first_failure: None,
				})
				.collect::<Vec<_>>(),
		),
	}
}

fn make_signed_header(
	notary_id: NotaryId,
	notebook_number: NotebookNumber,
	tick: Tick,
	account_origins: u32,
	chain_transfer_count: u32,
) -> SignedNotebookHeader {
	let changed_account_origins = BoundedVec::truncate_from(
		(0..account_origins)
			.map(|index| AccountOrigin { notebook_number, account_uid: index.saturating_add(1) })
			.collect::<Vec<_>>(),
	);
	let chain_transfers = BoundedVec::truncate_from(
		(0..chain_transfer_count)
			.map(|index| ChainTransfer::ToLocalchain { transfer_id: index.saturating_add(1) })
			.collect::<Vec<_>>(),
	);
	let header = NotebookHeader {
		version: 1,
		notebook_number,
		tick,
		tax: 0,
		notary_id,
		chain_transfers,
		changed_accounts_root: H256::repeat_byte((notary_id % 251) as u8 + 1),
		changed_account_origins,
		block_votes_root: H256::repeat_byte((notary_id % 251) as u8 + 2),
		block_votes_count: 1,
		blocks_with_votes: Default::default(),
		block_voting_power: 1,
		secret_hash: H256::repeat_byte((notary_id % 251) as u8 + 3),
		parent_secret: None,
		domains: Default::default(),
	};
	SignedNotebookHeader { header, signature: ed25519::Signature::from_raw([0u8; 64]) }
}

fn ensure_required_digests<T: Config>(notebook_digest: NotebookDigest<NotebookVerifyError>) {
	let author = T::NotaryProvider::active_notaries()
		.first()
		.and_then(|notary_id| T::NotaryProvider::notary_operator_account_id(*notary_id))
		.or_else(|| T::NotaryProvider::notary_operator_account_id(1))
		.expect("benchmark notary operator account must exist");

	set_all_digests::<T, NotebookVerifyError>(
		author,
		T::TickProvider::current_tick(),
		BlockVoteDigest { voting_power: 1, votes_count: 1 },
		notebook_digest,
	);
}

fn seed_notary_history_with_max_depth<T: Config>(notary_id: NotaryId, notebook_tick: Tick) {
	for notebook_number in 1u32..=16u32 {
		let tick =
			if notebook_number == 1 { notebook_tick } else { notebook_tick.saturating_sub(1) };
		let mut header = make_signed_header(notary_id, notebook_number, tick, 0, 0).header;
		header.parent_secret = Some(H256::repeat_byte((notary_id % 251) as u8 + 1));
		NotebookPallet::<T>::process_notebook(header);
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn submit(n: Linear<1, 100>) {
		let current_tick = T::TickProvider::current_tick();
		let notebook_tick = current_tick.saturating_sub(1);
		let notebooks = (0..n)
			.map(|index| make_signed_header(index.saturating_add(1), 1, notebook_tick, 0, 0))
			.collect::<Vec<_>>();
		let digest = notebook_digest(&notebooks);
		ensure_required_digests::<T>(digest);

		#[extrinsic_call]
		submit(RawOrigin::None, notebooks);

		assert_eq!(BlockNotebooks::<T>::get().notebooks.len(), n as usize);
	}

	#[benchmark]
	fn submit_with_account_origins(a: Linear<1, 1_000>) {
		let current_tick = T::TickProvider::current_tick();
		let notebook_tick = current_tick.saturating_sub(1);
		let notary_id = 1;
		let notebook_number = 1;
		let notebooks = vec![make_signed_header(notary_id, notebook_number, notebook_tick, a, 0)];
		let digest = notebook_digest(&notebooks);
		ensure_required_digests::<T>(digest);

		#[extrinsic_call]
		submit(RawOrigin::None, notebooks);

		assert_eq!(
			AccountOriginLastChangedNotebookByNotary::<T>::iter_prefix(notary_id).count(),
			a as usize
		);
	}

	#[benchmark]
	fn submit_with_chain_transfers(t: Linear<1, 10_000>) {
		let current_tick = T::TickProvider::current_tick();
		let notebook_tick = current_tick.saturating_sub(1);
		let notebooks = vec![make_signed_header(1, 1, notebook_tick, 0, t)];
		let digest = notebook_digest(&notebooks);
		ensure_required_digests::<T>(digest);

		#[extrinsic_call]
		submit(RawOrigin::None, notebooks);

		assert_eq!(BlockNotebooks::<T>::get().notebooks.len(), 1);
	}

	#[benchmark]
	fn unlock() -> Result<(), BenchmarkError> {
		let notary_id = 1;
		NotariesLockedForFailedAudit::<T>::insert(
			notary_id,
			(
				1,
				T::TickProvider::current_tick().saturating_sub(1),
				NotebookVerifyError::InvalidBlockVoteRoot,
			),
		);
		let who = T::NotaryProvider::notary_operator_account_id(notary_id)
			.ok_or(BenchmarkError::Stop("missing benchmark notary operator"))?;
		whitelist_account!(who);

		#[extrinsic_call]
		unlock(RawOrigin::Signed(who), notary_id);

		assert!(LockedNotaryReadyForReprocess::<T>::contains_key(notary_id));
		Ok(())
	}

	#[benchmark]
	fn provider_vote_eligible_notebook_count(n: Linear<1, 100>) {
		let schedule = T::TickProvider::voting_schedule();
		let digest = notebook_digest_from_count(n, schedule.notebook_tick(), 1);
		ensure_required_digests::<T>(digest);

		#[block]
		{
			let count =
				<NotebookPallet<T> as NotebookProvider>::vote_eligible_notebook_count(&schedule);
			assert_eq!(count, n);
		}
	}

	#[benchmark]
	fn provider_notebooks_in_block() {
		let n: u32 = 256;
		let digest =
			notebook_digest_from_count(n, T::TickProvider::voting_schedule().notebook_tick(), 1);
		BlockNotebooks::<T>::put(digest);

		#[block]
		{
			let notebooks = <NotebookPallet<T> as NotebookProvider>::notebooks_in_block();
			assert_eq!(notebooks.len(), n as usize);
		}
	}

	#[benchmark]
	fn provider_eligible_notebooks_for_vote(n: Linear<1, 100>) {
		let schedule = T::TickProvider::voting_schedule();
		let notebook_tick = schedule.notebook_tick();
		for i in 0..n {
			let notary_id = i.saturating_add(1);
			seed_notary_history_with_max_depth::<T>(notary_id, notebook_tick);
		}

		#[block]
		{
			let notebooks =
				<NotebookPallet<T> as NotebookProvider>::eligible_notebooks_for_vote(&schedule);
			assert_eq!(notebooks.len(), n as usize);
		}
	}

	#[benchmark]
	fn provider_get_eligible_tick_votes_root() {
		let schedule = T::TickProvider::voting_schedule();
		let notebook_tick = schedule.notebook_tick();
		let notary_id = 1u32;
		let expected_root =
			make_signed_header(notary_id, 1, notebook_tick, 0, 0).header.block_votes_root;
		seed_notary_history_with_max_depth::<T>(notary_id, notebook_tick);

		#[block]
		{
			let result = <NotebookPallet<T> as NotebookProvider>::get_eligible_tick_votes_root(
				notary_id,
				notebook_tick,
			);
			assert_eq!(result, Some((expected_root, 1)));
		}
	}

	#[benchmark]
	fn provider_is_notary_locked_at_tick() {
		let schedule = T::TickProvider::voting_schedule();
		let notary_id = 1u32;
		let locked_tick = schedule.notebook_tick().saturating_sub(1);
		NotariesLockedForFailedAudit::<T>::insert(
			notary_id,
			(1, locked_tick, NotebookVerifyError::InvalidBlockVoteRoot),
		);

		#[block]
		{
			let is_locked = <NotebookPallet<T> as NotebookProvider>::is_notary_locked_at_tick(
				notary_id,
				schedule.notebook_tick(),
			);
			assert!(is_locked);
		}
	}

	impl_benchmark_test_suite!(NotebookPallet, crate::mock::new_test_ext(), crate::mock::Test);
}
