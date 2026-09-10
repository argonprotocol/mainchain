use crate::{
	Config, MintIndex, NextPendingBitcoinMintIndex, Pallet, PendingBitcoinMint,
	PendingBitcoinMintsByIndex, PendingMintIndicesByLockId,
};
use argon_primitives::bitcoin::BitcoinLockId;
use codec::{Decode, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[derive(Decode, Encode)]
struct PendingMintUtxoV2<T: Config>
where
	T::AccountId: Codec,
	T::Balance: Codec,
{
	#[codec(compact)]
	utxo_id: BitcoinLockId,
	account_id: T::AccountId,
	#[codec(compact)]
	remaining_amount: T::Balance,
	#[codec(compact)]
	max_amount_per_frame: T::Balance,
}

mod v2 {
	use super::*;

	#[storage_alias]
	pub(super) type PendingMintUtxosByIndex<T: Config> =
		StorageMap<Pallet<T>, Blake2_128Concat, MintIndex, PendingMintUtxoV2<T>, OptionQuery>;

	#[storage_alias]
	pub(super) type PendingMintUtxoIdLookup<T: Config> = StorageMap<
		Pallet<T>,
		Blake2_128Concat,
		BitcoinLockId,
		BoundedVec<MintIndex, <T as Config>::MaxPendingMintsPerUtxo>,
		ValueQuery,
	>;

	#[storage_alias]
	pub(super) type NextPendingMintUtxoIndex<T: Config> =
		StorageValue<Pallet<T>, MintIndex, ValueQuery>;
}

pub struct AddFissionIdToPendingMints<T>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for AddFissionIdToPendingMints<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let pending = v2::PendingMintUtxosByIndex::<T>::iter()
			.map(|(index, pending)| {
				(
					index,
					pending.utxo_id,
					pending.account_id,
					pending.remaining_amount,
					pending.max_amount_per_frame,
				)
			})
			.collect::<Vec<_>>();
		let reverse_lookup = v2::PendingMintUtxoIdLookup::<T>::iter().collect::<Vec<_>>();
		let next_index = v2::NextPendingMintUtxoIndex::<T>::get();

		Ok((pending, reverse_lookup, next_index).encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut pending_count = 0u64;
		for (index, pending) in v2::PendingMintUtxosByIndex::<T>::drain() {
			pending_count = pending_count.saturating_add(1);
			PendingBitcoinMintsByIndex::<T>::insert(
				index,
				PendingBitcoinMint {
					fission_id: pending.utxo_id,
					lock_id: pending.utxo_id,
					account_id: pending.account_id,
					remaining_amount: pending.remaining_amount,
					max_amount_per_frame: pending.max_amount_per_frame,
				},
			);
		}

		let mut reverse_lookup_count = 0u64;
		for (lock_id, pending_indices) in v2::PendingMintUtxoIdLookup::<T>::drain() {
			reverse_lookup_count = reverse_lookup_count.saturating_add(1);
			PendingMintIndicesByLockId::<T>::insert(lock_id, pending_indices);
		}

		NextPendingBitcoinMintIndex::<T>::put(v2::NextPendingMintUtxoIndex::<T>::take());

		T::DbWeight::get().reads_writes(
			pending_count.saturating_add(reverse_lookup_count).saturating_add(1),
			pending_count
				.saturating_mul(2)
				.saturating_add(reverse_lookup_count.saturating_mul(2))
				.saturating_add(2),
		)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		type PendingState<T> = Vec<(
			MintIndex,
			BitcoinLockId,
			<T as frame_system::Config>::AccountId,
			<T as Config>::Balance,
			<T as Config>::Balance,
		)>;
		type ReverseLookupState<T> =
			Vec<(BitcoinLockId, BoundedVec<MintIndex, <T as Config>::MaxPendingMintsPerUtxo>)>;
		let (expected, expected_reverse_lookup, expected_next_index): (
			PendingState<T>,
			ReverseLookupState<T>,
			MintIndex,
		) = Decode::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("could not decode pending mint migration state"))?;
		ensure!(
			PendingBitcoinMintsByIndex::<T>::iter_keys().count() == expected.len(),
			TryRuntimeError::Other("pending mint count changed during migration"),
		);
		for (index, lock_id, account_id, remaining_amount, max_amount_per_frame) in expected {
			let pending = PendingBitcoinMintsByIndex::<T>::get(index)
				.ok_or(TryRuntimeError::Other("pending mint was not migrated"))?;
			ensure!(
				pending.fission_id == lock_id &&
					pending.lock_id == lock_id &&
					pending.account_id == account_id &&
					pending.remaining_amount == remaining_amount &&
					pending.max_amount_per_frame == max_amount_per_frame,
				TryRuntimeError::Other("pending mint accounting changed during migration"),
			);
		}
		ensure!(
			PendingMintIndicesByLockId::<T>::iter().collect::<Vec<_>>() == expected_reverse_lookup,
			TryRuntimeError::Other("pending mint reverse lookup changed during migration"),
		);
		ensure!(
			NextPendingBitcoinMintIndex::<T>::get() == expected_next_index,
			TryRuntimeError::Other("next pending mint index changed during migration"),
		);
		ensure!(
			v2::PendingMintUtxosByIndex::<T>::iter_keys().next().is_none() &&
				v2::PendingMintUtxoIdLookup::<T>::iter_keys().next().is_none(),
			TryRuntimeError::Other("old pending mint storage was not removed"),
		);
		Ok(())
	}
}

pub type AddFissionIdToPendingMintsMigration<T> = frame_support::migrations::VersionedMigration<
	2,
	3,
	AddFissionIdToPendingMints<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		mock::*, NextPendingBitcoinMintIndex, PendingBitcoinMint, PendingBitcoinMintsByIndex,
		PendingMintIndicesByLockId, PendingMintQueueState,
	};

	#[test]
	fn v2_pending_mint_preserves_queue_accounting_and_uses_its_utxo_as_fission_id() {
		new_test_ext().execute_with(|| {
			v2::PendingMintUtxosByIndex::<Test>::insert(
				4,
				PendingMintUtxoV2 {
					utxo_id: 9,
					account_id: 2,
					remaining_amount: 700,
					max_amount_per_frame: 70,
				},
			);
			v2::PendingMintUtxoIdLookup::<Test>::insert(9, BoundedVec::truncate_from(vec![4]));
			v2::NextPendingMintUtxoIndex::<Test>::put(5);
			let queue_state = crate::MintQueueCursor {
				payout_start_index: 3,
				payout_cursor_index: 4,
				payout_cursor_frame_id: Some(6),
			};
			PendingMintQueueState::<Test>::put(queue_state.clone());

			AddFissionIdToPendingMints::<Test>::on_runtime_upgrade();

			assert_eq!(
				PendingBitcoinMintsByIndex::<Test>::get(4),
				Some(PendingBitcoinMint {
					fission_id: 9,
					lock_id: 9,
					account_id: 2,
					remaining_amount: 700,
					max_amount_per_frame: 70,
				})
			);
			assert_eq!(PendingMintIndicesByLockId::<Test>::get(9).to_vec(), vec![4]);
			assert_eq!(NextPendingBitcoinMintIndex::<Test>::get(), 5);
			assert!(!v2::PendingMintUtxosByIndex::<Test>::contains_key(4));
			assert!(!v2::PendingMintUtxoIdLookup::<Test>::contains_key(9));
			assert_eq!(PendingMintQueueState::<Test>::get(), queue_state);
		});
	}
}
