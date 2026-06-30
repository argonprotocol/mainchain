use crate::{
	pallet::{
		MintIndex, MintQueueCursor, NextPendingMintUtxoIndex, PendingMintQueueState,
		PendingMintUtxo, PendingMintUtxoIdLookup, PendingMintUtxosByIndex,
	},
	Config, Pallet,
};
use argon_primitives::bitcoin::UtxoId;
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;

// The pre-upgrade queue was a single bounded vector with this maximum length.
const LEGACY_MAX_PENDING_MINT_QUEUE_ENTRIES: u32 = 10_000;

mod v1 {
	use super::*;

	#[storage_alias]
	pub(super) type PendingMintUtxos<T: Config> = StorageValue<
		Pallet<T>,
		BoundedVec<
			(UtxoId, <T as frame_system::Config>::AccountId, <T as crate::Config>::Balance),
			ConstU32<LEGACY_MAX_PENDING_MINT_QUEUE_ENTRIES>,
		>,
		ValueQuery,
	>;
}

pub struct MigratePendingMintQueue<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for MigratePendingMintQueue<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		Ok(v1::PendingMintUtxos::<T>::get().encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let old_pending = v1::PendingMintUtxos::<T>::take().into_inner();
		let queue_cursor = MintQueueCursor::default();

		for (queue_index, (utxo_id, account_id, remaining_amount)) in
			old_pending.iter().cloned().enumerate()
		{
			let queue_index = queue_index as MintIndex;
			PendingMintUtxosByIndex::<T>::insert(
				queue_index,
				PendingMintUtxo::<T> {
					utxo_id,
					account_id,
					remaining_amount,
					max_amount_per_frame: Pallet::<T>::get_bitcoin_mint_payout_cap(
						remaining_amount,
					),
				},
			);
			PendingMintUtxoIdLookup::<T>::try_mutate(utxo_id, |pending_indices| {
				pending_indices
					.try_push(queue_index)
					.map_err(|_| "pending mint lookup capacity exceeded")
			})
			.expect("legacy pending mint entries for a UTXO must fit MaxPendingMintsPerUtxo");
		}

		NextPendingMintUtxoIndex::<T>::put(old_pending.len() as MintIndex);
		PendingMintQueueState::<T>::put(queue_cursor);

		let migrated_count = old_pending.len() as u64;
		T::DbWeight::get().reads_writes(1 + migrated_count, 7 + (migrated_count * 2))
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;
		use frame_support::ensure;

		let old_pending = BoundedVec::<
			(UtxoId, T::AccountId, T::Balance),
			ConstU32<LEGACY_MAX_PENDING_MINT_QUEUE_ENTRIES>,
		>::decode(&mut &state[..])
		.map_err(|_| sp_runtime::TryRuntimeError::Other("Failed to decode pending mints"))?;

		let next_utxo_index = NextPendingMintUtxoIndex::<T>::get();
		let queue_cursor = PendingMintQueueState::<T>::get();

		ensure!(
			next_utxo_index == old_pending.len() as MintIndex,
			"Next pending mint index changed"
		);
		ensure!(queue_cursor.payout_start_index == 0, "Payout start index changed");
		ensure!(queue_cursor.payout_cursor_index == 0, "Payout cursor should be reset");
		ensure!(
			queue_cursor.payout_cursor_frame_id.is_none(),
			"Payout cursor frame should be cleared"
		);

		for (queue_index, (utxo_id, account_id, remaining_amount)) in
			old_pending.into_iter().enumerate()
		{
			let queue_index = queue_index as MintIndex;
			let pending = PendingMintUtxosByIndex::<T>::get(queue_index)
				.ok_or(sp_runtime::TryRuntimeError::Other("Missing migrated pending mint"))?;
			ensure!(pending.utxo_id == utxo_id, "Pending mint UTXO id changed");
			ensure!(pending.account_id == account_id, "Pending mint account changed");
			ensure!(
				pending.remaining_amount == remaining_amount,
				"Pending mint remaining amount changed"
			);
			ensure!(
				pending.max_amount_per_frame ==
					Pallet::<T>::get_bitcoin_mint_payout_cap(remaining_amount),
				"Pending mint payout cap changed"
			);
			ensure!(
				PendingMintUtxoIdLookup::<T>::get(utxo_id).contains(&queue_index),
				"Pending mint lookup changed"
			);
		}
		Ok(())
	}
}

pub type PendingMintQueueMigration<T> = frame_support::migrations::VersionedMigration<
	1,
	2,
	MigratePendingMintQueue<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::{assert_ok, traits::OnRuntimeUpgrade};

	#[test]
	fn migrates_pending_mints() {
		new_test_ext().execute_with(|| {
			v1::PendingMintUtxos::<Test>::put(BoundedVec::truncate_from(vec![
				(1, 10, 25u128),
				(2, 20, 100u128),
				(1, 10, 50u128),
			]));
			frame_support::traits::StorageVersion::new(1).put::<Pallet<Test>>();

			let state = MigratePendingMintQueue::<Test>::pre_upgrade().unwrap();
			PendingMintQueueMigration::<Test>::on_runtime_upgrade();
			assert_ok!(MigratePendingMintQueue::<Test>::post_upgrade(state));

			let queue_cursor = PendingMintQueueState::<Test>::get();
			assert_eq!(NextPendingMintUtxoIndex::<Test>::get(), 3);
			assert_eq!(queue_cursor.payout_start_index, 0);
			assert_eq!(queue_cursor.payout_cursor_index, 0);

			let pending = PendingMintUtxosByIndex::<Test>::get(0).unwrap();
			assert_eq!(pending.utxo_id, 1);
			assert_eq!(pending.max_amount_per_frame, 3);
			assert_eq!(PendingMintUtxoIdLookup::<Test>::get(1).to_vec(), vec![0, 2]);

			let pending = PendingMintUtxosByIndex::<Test>::get(1).unwrap();
			assert_eq!(pending.utxo_id, 2);
			assert_eq!(pending.max_amount_per_frame, 10);

			let pending = PendingMintUtxosByIndex::<Test>::get(2).unwrap();
			assert_eq!(pending.utxo_id, 1);
			assert_eq!(pending.max_amount_per_frame, 5);
		});
	}
}
