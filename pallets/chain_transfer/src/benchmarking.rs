//! Benchmarking setup for pallet-chain-transfer
#![cfg(feature = "runtime-benchmarks")]

use super::*;
#[allow(unused_imports)]
use crate::Pallet as ChainTransferPallet;
use argon_primitives::{
	NotebookEventHandler, TransferToLocalchainId,
	notary::NotaryProvider,
	notebook::{ChainTransfer, NotebookHeader},
	providers::ChainTransferLookup,
};
use pallet_prelude::benchmarking::{
	BenchmarkNotebookProviderCallCounters, benchmark_notebook_provider_call_counters,
	reset_benchmark_notebook_provider_call_counters, reset_benchmark_notebook_provider_state,
};
use polkadot_sdk::{
	frame_benchmarking::v2::*,
	frame_support::{
		BoundedVec,
		traits::{Get, fungible::Mutate},
	},
	sp_core::H256,
};

fn benchmark_notebook_header(
	notary_id: NotaryId,
	notebook_number: NotebookNumber,
	tick: Tick,
	chain_transfers: Vec<ChainTransfer>,
) -> NotebookHeader {
	NotebookHeader {
		version: 1,
		notebook_number,
		tick,
		tax: 0,
		notary_id,
		chain_transfers: BoundedVec::truncate_from(chain_transfers),
		changed_accounts_root: H256::repeat_byte(1),
		changed_account_origins: BoundedVec::default(),
		block_votes_root: H256::repeat_byte(2),
		block_votes_count: 1,
		blocks_with_votes: BoundedVec::default(),
		block_voting_power: 1,
		secret_hash: H256::repeat_byte(3),
		parent_secret: None,
		domains: BoundedVec::default(),
	}
}

fn seed_pending_transfer<T: Config>(
	transfer_id: TransferToLocalchainId,
	notary_id: NotaryId,
	account_id: T::AccountId,
	amount: T::Balance,
	expiration_tick: Tick,
) -> Result<(), BenchmarkError>
where
	T::Argon: Mutate<T::AccountId, Balance = T::Balance>,
{
	PendingTransfersOut::<T>::insert(
		transfer_id,
		QueuedTransferOut { account_id, amount, expiration_tick, notary_id },
	);
	ExpiringTransfersOutByNotary::<T>::try_append(notary_id, expiration_tick, transfer_id)
		.map_err(|_| BenchmarkError::Stop("expiring transfer bound exceeded"))?;
	Ok(())
}

fn assert_provider_calls(expected: BenchmarkNotebookProviderCallCounters) {
	if cfg!(test) {
		assert_eq!(benchmark_notebook_provider_call_counters(), expected);
	}
}

#[benchmarks(
	where
		T::AccountId: From<[u8; 32]>,
		[u8; 32]: From<T::AccountId>,
		T::Argon: Mutate<T::AccountId, Balance = T::Balance>,
)]
mod benchmarks {
	use super::*;
	use frame_system::RawOrigin;

	#[benchmark]
	fn send_to_localchain() -> Result<(), BenchmarkError> {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		let caller: T::AccountId = account("chain-transfer-caller", 0, 0);
		let notary_id = T::NotaryProvider::active_notaries()
			.first()
			.copied()
			.ok_or(BenchmarkError::Stop("missing active benchmark notary"))?;
		let amount: T::Balance = 1_000_000_000u128.into();
		T::Argon::mint_into(&caller, 10_000_000_000u128.into())
			.map_err(|_| BenchmarkError::Stop("failed to fund benchmark caller"))?;
		whitelist_account!(caller);

		#[extrinsic_call]
		send_to_localchain(RawOrigin::Signed(caller.clone()), amount, notary_id);

		assert!(PendingTransfersOut::<T>::iter().next().is_some());
		assert_provider_calls(BenchmarkNotebookProviderCallCounters {
			is_notary_locked_at_tick: 1,
			..Default::default()
		});
		Ok(())
	}

	#[benchmark]
	fn provider_is_valid_transfer_to_localchain() -> Result<(), BenchmarkError> {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		let notary_id = T::NotaryProvider::active_notaries()
			.first()
			.copied()
			.ok_or(BenchmarkError::Stop("missing active benchmark notary"))?;
		let account_id: T::AccountId = account("transfer-lookup-account", 0, 0);
		let transfer_id: TransferToLocalchainId = 1;
		let amount: T::Balance = 777u128.into();
		let at_tick: Tick = 10;
		seed_pending_transfer::<T>(
			transfer_id,
			notary_id,
			account_id.clone(),
			amount,
			at_tick.saturating_add(1),
		)?;

		#[block]
		{
			let is_valid = <ChainTransferPallet<T> as ChainTransferLookup<
				T::AccountId,
				T::Balance,
			>>::is_valid_transfer_to_localchain(
				notary_id, transfer_id, &account_id, amount, at_tick
			);
			assert!(is_valid);
		}

		assert_provider_calls(Default::default());
		Ok(())
	}

	#[benchmark]
	fn notebook_submitted_event_handler(t: Linear<1, 1_000>) -> Result<(), BenchmarkError> {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		let notary_id: NotaryId = 1;
		let notebook_tick: Tick = 10;
		let expiration_tick = notebook_tick.saturating_add(50);
		let transfer_count = t.min(T::MaxPendingTransfersOutPerBlock::get());

		let recipient: T::AccountId = account("chain-transfer-recipient", 0, 0);
		let mut chain_transfers = Vec::with_capacity(transfer_count as usize);
		for i in 0..transfer_count {
			let transfer_id = i.saturating_add(1);
			seed_pending_transfer::<T>(
				transfer_id,
				notary_id,
				recipient.clone(),
				1u128.into(),
				expiration_tick,
			)?;
			chain_transfers.push(ChainTransfer::ToLocalchain { transfer_id });
		}
		let header = benchmark_notebook_header(notary_id, 1, notebook_tick, chain_transfers);

		#[block]
		{
			<ChainTransferPallet<T> as NotebookEventHandler>::notebook_submitted(&header);
		}

		assert!(PendingTransfersOut::<T>::iter().next().is_none());
		assert_provider_calls(Default::default());
		Ok(())
	}

	#[benchmark]
	fn process_expired_transfers(e: Linear<1, 1_000>) -> Result<(), BenchmarkError> {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		let notary_id: NotaryId = 1;
		let start_expiration_tick: Tick = 10;
		let transfer_count = e;
		let notebook_tick =
			start_expiration_tick.saturating_add(transfer_count as Tick).saturating_add(1);
		let transfer_amount: T::Balance = 10_000_000_000u128.into();
		let total_funding = 10_000_000_000u128
			.saturating_mul(transfer_count as u128)
			.saturating_add(10_000_000_000u128);
		let notary_account = ChainTransferPallet::<T>::notary_account_id(notary_id);
		T::Argon::mint_into(&notary_account, total_funding.into())
			.map_err(|_| BenchmarkError::Stop("failed to fund notary account"))?;

		for i in 0..transfer_count {
			let transfer_id = i.saturating_add(1);
			let recipient: T::AccountId = account("expired-transfer-recipient", i, 0);
			let expiration_tick = start_expiration_tick.saturating_add(i as Tick);
			seed_pending_transfer::<T>(
				transfer_id,
				notary_id,
				recipient,
				transfer_amount,
				expiration_tick,
			)?;
		}

		let header = benchmark_notebook_header(notary_id, 1, notebook_tick, Vec::new());

		#[block]
		{
			<ChainTransferPallet<T> as NotebookEventHandler>::notebook_submitted(&header);
		}

		assert!(ExpiringTransfersOutByNotary::<T>::iter_prefix(notary_id).next().is_none());
		assert_provider_calls(Default::default());
		Ok(())
	}

	impl_benchmark_test_suite!(ChainTransferPallet, crate::mock::new_test_ext(), crate::mock::Test);
}
