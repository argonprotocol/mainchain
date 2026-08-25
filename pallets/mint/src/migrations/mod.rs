use crate::{Config, MintIndex, Pallet, PendingMintUtxo, PendingMintUtxosByIndex};
use argon_primitives::bitcoin::UtxoId;
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
	utxo_id: UtxoId,
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
}

pub struct AddFissionIdToPendingMints<T>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for AddFissionIdToPendingMints<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		Ok(v2::PendingMintUtxosByIndex::<T>::iter()
			.map(|(index, pending)| {
				(
					index,
					pending.utxo_id,
					pending.account_id,
					pending.remaining_amount,
					pending.max_amount_per_frame,
				)
			})
			.collect::<Vec<_>>()
			.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut migrated = 0u64;
		PendingMintUtxosByIndex::<T>::translate::<PendingMintUtxoV2<T>, _>(|_, pending| {
			migrated = migrated.saturating_add(1);
			Some(PendingMintUtxo {
				fission_id: pending.utxo_id,
				utxo_id: pending.utxo_id,
				account_id: pending.account_id,
				remaining_amount: pending.remaining_amount,
				max_amount_per_frame: pending.max_amount_per_frame,
			})
		});

		T::DbWeight::get().reads_writes(migrated, migrated)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let expected = <Vec<(MintIndex, UtxoId, T::AccountId, T::Balance, T::Balance)>>::decode(
			&mut state.as_slice(),
		)
		.map_err(|_| TryRuntimeError::Other("could not decode pending mint migration state"))?;
		ensure!(
			PendingMintUtxosByIndex::<T>::iter_keys().count() == expected.len(),
			TryRuntimeError::Other("pending mint count changed during migration"),
		);
		for (index, utxo_id, account_id, remaining_amount, max_amount_per_frame) in expected {
			let pending = PendingMintUtxosByIndex::<T>::get(index)
				.ok_or(TryRuntimeError::Other("pending mint was not migrated"))?;
			ensure!(
				pending.fission_id == utxo_id &&
					pending.utxo_id == utxo_id &&
					pending.account_id == account_id &&
					pending.remaining_amount == remaining_amount &&
					pending.max_amount_per_frame == max_amount_per_frame,
				TryRuntimeError::Other("pending mint accounting changed during migration"),
			);
		}
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
	use crate::{mock::*, PendingMintQueueState, PendingMintUtxo, PendingMintUtxoIdLookup};

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
			PendingMintUtxoIdLookup::<Test>::insert(9, BoundedVec::truncate_from(vec![4]));
			let queue_state = crate::MintQueueCursor {
				payout_start_index: 3,
				payout_cursor_index: 4,
				payout_cursor_frame_id: Some(6),
			};
			PendingMintQueueState::<Test>::put(queue_state.clone());

			AddFissionIdToPendingMints::<Test>::on_runtime_upgrade();

			assert_eq!(
				PendingMintUtxosByIndex::<Test>::get(4),
				Some(PendingMintUtxo {
					fission_id: 9,
					utxo_id: 9,
					account_id: 2,
					remaining_amount: 700,
					max_amount_per_frame: 70,
				})
			);
			assert_eq!(PendingMintUtxoIdLookup::<Test>::get(9).to_vec(), vec![4]);
			assert_eq!(PendingMintQueueState::<Test>::get(), queue_state);
		});
	}
}
