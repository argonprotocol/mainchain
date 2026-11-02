use crate::{pallet::STORAGE_VERSION, *};

pub(crate) fn check<T: Config>() -> Result<(), sp_runtime::TryRuntimeError> {
	ensure!(
		StorageVersion::get::<Pallet<T>>() == STORAGE_VERSION,
		"Mint invariant checks require current storage version; run migrations first"
	);

	let next_index = NextPendingBitcoinMintIndex::<T>::get();
	let cursor = PendingMintQueueState::<T>::get();
	ensure!(
		cursor.payout_start_index <= cursor.payout_cursor_index,
		"Bitcoin mint payout cursor is before the payout start"
	);
	ensure!(
		cursor.payout_cursor_index <= next_index,
		"Bitcoin mint payout cursor exceeds the next queue index"
	);

	for (index, mint) in PendingBitcoinMintsByIndex::<T>::iter() {
		ensure!(
			index >= cursor.payout_start_index && index < next_index,
			"Pending Bitcoin mint is outside the active queue"
		);
		ensure!(
			!mint.remaining_amount.is_zero(),
			"Completed Bitcoin mint remains in the pending queue"
		);
		ensure!(
			PendingMintIndicesByLockId::<T>::get(mint.lock_id).contains(&index),
			"Pending Bitcoin mint is missing from its Lock index"
		);
	}

	for (lock_id, indices) in PendingMintIndicesByLockId::<T>::iter() {
		ensure!(!indices.is_empty(), "Empty pending mint Lock index remains in storage");
		ensure!(
			indices.windows(2).all(|pair| pair[0] < pair[1]),
			"Pending mint Lock indices are not strictly increasing"
		);
		for index in indices {
			let mint = PendingBitcoinMintsByIndex::<T>::get(index)
				.ok_or("Pending mint Lock index refers to a missing mint")?;
			ensure!(mint.lock_id == lock_id, "Pending mint Lock index refers to a different Lock");
		}
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use argon_primitives::BitcoinFissionMinting;
	use frame_support::traits::Hooks;

	#[test]
	fn try_state_accepts_partial_payouts_and_completed_queue_entries() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(4).put::<Mint>();
			assert_ok!(Balances::mint_into(&1, ExistentialDeposit::get()));
			assert_ok!(Mint::request_mint(&1, 0, 1, 100));
			assert_ok!(Mint::request_mint(&1, 1, 1, 1));
			set_cpi(-0.1);
			assert_ok!(Mint::try_state(0));

			for frame_id in 1..=10 {
				CurrentFrameId::set(frame_id);
				set_issuance_sources(1_000, MintedBitcoinMicrogons::<Test>::get());
				Mint::on_initialize(frame_id);
				assert_ok!(Mint::try_state(frame_id));
			}

			assert!(pending_mints().is_empty());
			assert!(PendingMintIndicesByLockId::<Test>::get(1).is_empty());
			assert_eq!(PendingMintQueueState::<Test>::get().payout_start_index, 2);
		});
	}

	#[test]
	fn try_state_detects_a_pending_mint_missing_from_the_lock_index() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(4).put::<Mint>();
			assert_ok!(Mint::request_mint(&1, 0, 1, 100));
			PendingMintIndicesByLockId::<Test>::remove(1);

			assert_eq!(
				Mint::try_state(0),
				Err("Pending Bitcoin mint is missing from its Lock index".into())
			);
		});
	}

	#[test]
	fn try_state_detects_duplicate_and_dangling_lock_indices() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(4).put::<Mint>();
			assert_ok!(Mint::request_mint(&1, 0, 1, 100));
			PendingMintIndicesByLockId::<Test>::mutate(1, |indices| {
				indices.try_push(0).unwrap();
			});
			assert_eq!(
				Mint::try_state(0),
				Err("Pending mint Lock indices are not strictly increasing".into())
			);

			PendingMintIndicesByLockId::<Test>::mutate(1, |indices| {
				indices.pop();
			});
			PendingBitcoinMintsByIndex::<Test>::remove(0);
			assert_eq!(
				Mint::try_state(0),
				Err("Pending mint Lock index refers to a missing mint".into())
			);
		});
	}

	#[test]
	fn try_state_detects_a_payout_start_that_skips_unpaid_mints() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(4).put::<Mint>();
			assert_ok!(Mint::request_mint(&1, 0, 1, 100));
			PendingMintQueueState::<Test>::mutate(|cursor| {
				cursor.payout_start_index = 1;
				cursor.payout_cursor_index = 1;
			});

			assert_eq!(
				Mint::try_state(0),
				Err("Pending Bitcoin mint is outside the active queue".into())
			);
		});
	}

	#[test]
	fn try_state_requires_the_current_mint_storage_layout() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(3).put::<Mint>();
			assert_eq!(
				Mint::try_state(0),
				Err("Mint invariant checks require current storage version; run migrations first"
					.into())
			);
		});
	}
}
