use super::{
	Config, FunderState, FunderStateByVaultAndAccount, FundersByVaultId, Pallet, PendingUnlock,
	PendingUnlocksByFrame, VaultPoolsByFrame,
};
use alloc::vec::Vec;
use frame_support::{traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::{
	argon_primitives::{MiningFrameTransitionProvider, vault::TreasuryVaultProvider},
	*,
};

pub struct SimplifyFunderState<T: Config>(core::marker::PhantomData<T>);

#[derive(Decode, DecodeWithMemTracking, Encode, TypeInfo)]
#[scale_info(skip_type_params(T))]
struct LegacyFunderState<T: Config> {
	#[codec(compact)]
	target_principal: T::Balance,
	#[codec(compact)]
	bonded_principal: T::Balance,
	#[codec(compact)]
	held_principal: T::Balance,
	#[codec(compact)]
	lifetime_compounded_earnings: T::Balance,
	#[codec(compact)]
	lifetime_principal_deployed: T::Balance,
	#[codec(compact)]
	lifetime_principal_last_basis_frame: FrameId,
}

struct ActiveBond<AccountId, Balance> {
	vault_id: VaultId,
	account_id: AccountId,
	release_frame_id: FrameId,
	amount: Balance,
}

struct AggregatedPendingUnlock<AccountId> {
	vault_id: VaultId,
	account_id: AccountId,
	frame_id: FrameId,
}

struct MigratedTrackedFunder<AccountId, Balance> {
	vault_id: VaultId,
	account_id: AccountId,
	held_principal: Balance,
}

impl<T: Config> UncheckedOnRuntimeUpgrade for SimplifyFunderState<T> {
	fn on_runtime_upgrade() -> Weight {
		let current_frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
		let exit_delay_frames = T::TreasuryExitDelayFrames::get();
		let mut db_reads = 0u64;
		let mut db_writes = 0u64;
		let mut active_bonds: Vec<ActiveBond<T::AccountId, T::Balance>> = Vec::new();
		let mut pending_unlocks: Vec<AggregatedPendingUnlock<T::AccountId>> = Vec::new();
		let mut tracked_funders: Vec<MigratedTrackedFunder<T::AccountId, T::Balance>> = Vec::new();

		for (frame_id, pools) in VaultPoolsByFrame::<T>::iter() {
			db_reads = db_reads.saturating_add(1);
			let release_frame_id = frame_id.saturating_add(exit_delay_frames);
			if release_frame_id <= current_frame_id {
				continue;
			}

			for (vault_id, pool) in pools {
				for (account_id, amount) in pool.bond_holders {
					active_bonds.push(ActiveBond {
						vault_id,
						account_id,
						release_frame_id,
						amount,
					});
				}
			}
		}
		active_bonds.sort_by(|a, b| a.release_frame_id.cmp(&b.release_frame_id));

		FunderStateByVaultAndAccount::<T>::translate::<LegacyFunderState<T>, _>(
			|vault_id, account_id, old_state| {
				db_reads = db_reads.saturating_add(1);
				db_writes = db_writes.saturating_add(1);

				let total_pending =
					old_state.held_principal.saturating_sub(old_state.target_principal);
				let mut remaining_pending = total_pending;
				let mut pending_unlock_at_frame = None;

				for active_bond in &active_bonds {
					if remaining_pending.is_zero() {
						break;
					}
					if active_bond.vault_id != vault_id || active_bond.account_id != account_id {
						continue;
					}

					let matched_amount = active_bond.amount.min(remaining_pending);
					if matched_amount.is_zero() {
						continue;
					}

					remaining_pending.saturating_reduce(matched_amount);
					pending_unlock_at_frame = Some(active_bond.release_frame_id);
				}

				if !total_pending.is_zero() &&
					(!remaining_pending.is_zero() || pending_unlock_at_frame.is_none())
				{
					pending_unlock_at_frame =
						Some(current_frame_id.saturating_add(exit_delay_frames));
				}

				if let Some(frame_id) = pending_unlock_at_frame {
					pending_unlocks.push(AggregatedPendingUnlock {
						vault_id,
						account_id: account_id.clone(),
						frame_id,
					});
				}

				if !old_state.held_principal.is_zero() {
					tracked_funders.push(MigratedTrackedFunder {
						vault_id,
						account_id: account_id.clone(),
						held_principal: old_state.held_principal,
					});
				}

				Some(FunderState::<T> {
					held_principal: old_state.held_principal,
					pending_unlock_amount: total_pending,
					pending_unlock_at_frame,
					lifetime_compounded_earnings: old_state.lifetime_compounded_earnings,
					lifetime_principal_deployed: old_state.lifetime_principal_deployed,
					lifetime_principal_last_basis_frame: old_state
						.lifetime_principal_last_basis_frame,
				})
			},
		);

		for pending_unlock in pending_unlocks {
			insert_pending_unlock_index::<T>(&pending_unlock, &mut db_reads, &mut db_writes);
		}

		tracked_funders.sort_by(|a, b| {
			a.vault_id
				.cmp(&b.vault_id)
				.then_with(|| b.held_principal.cmp(&a.held_principal))
		});

		let clear_results = FundersByVaultId::<T>::clear(u32::MAX, None);
		db_reads = db_reads.saturating_add(u64::from(clear_results.loops));
		db_writes = db_writes.saturating_add(u64::from(clear_results.unique));

		let mut start = 0usize;
		while start < tracked_funders.len() {
			let vault_id = tracked_funders[start].vault_id;
			let operator = T::TreasuryVaultProvider::get_vault_operator(vault_id);
			db_reads = db_reads.saturating_add(1);

			let mut end = start + 1;
			while end < tracked_funders.len() && tracked_funders[end].vault_id == vault_id {
				end += 1;
			}

			let vault_funders = &tracked_funders[start..end];
			let mut tracked =
				BoundedVec::<(T::AccountId, T::Balance), T::MaxTrackedTreasuryFunders>::default();

			if let Some(operator_account) = operator.as_ref() {
				if let Some(operator_funder) =
					vault_funders.iter().find(|funder| funder.account_id == *operator_account)
				{
					let _ = tracked.try_push((
						operator_funder.account_id.clone(),
						operator_funder.held_principal,
					));
				}
			}

			for funder in vault_funders {
				if Some(&funder.account_id) == operator.as_ref() || tracked.is_full() {
					continue;
				}

				let _ = tracked.try_push((funder.account_id.clone(), funder.held_principal));
			}

			if !tracked.is_empty() {
				FundersByVaultId::<T>::insert(vault_id, tracked);
				db_writes = db_writes.saturating_add(1);
			}

			start = end;
		}

		T::DbWeight::get().reads_writes(db_reads, db_writes)
	}
}

fn insert_pending_unlock_index<T: Config>(
	pending_unlock: &AggregatedPendingUnlock<T::AccountId>,
	db_reads: &mut u64,
	db_writes: &mut u64,
) {
	*db_reads = db_reads.saturating_add(1);
	PendingUnlocksByFrame::<T>::mutate(pending_unlock.frame_id, |scheduled_unlocks| {
		if scheduled_unlocks.iter().any(|existing| {
			existing.vault_id == pending_unlock.vault_id &&
				existing.account_id == pending_unlock.account_id
		}) {
			return;
		}

		if scheduled_unlocks
			.try_push(PendingUnlock {
				vault_id: pending_unlock.vault_id,
				account_id: pending_unlock.account_id.clone(),
			})
			.is_ok()
		{
			*db_writes = db_writes.saturating_add(1);
		}
	});
}

pub type SimplifyFunderStateMigration<T> = frame_support::migrations::VersionedMigration<
	3,
	4,
	SimplifyFunderState<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod test {
	use super::{
		super::{
			mock::{CurrentFrameId, Test, TestVault, Treasury, insert_vault, new_test_ext},
			pallet::TreasuryPool,
		},
		*,
	};
	use codec::Encode;
	use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
	use sp_runtime::{BoundedBTreeMap, Permill};

	#[test]
	fn migration_from_v3_preserves_pending_unlocks() {
		new_test_ext().execute_with(|| {
			CurrentFrameId::set(2);
			insert_vault(
				1,
				TestVault {
					account_id: 10,
					securitized_satoshis: 1_000,
					sharing_percent: Permill::zero(),
					is_closed: false,
				},
			);

			let old_state = LegacyFunderState::<Test> {
				target_principal: 60,
				bonded_principal: 100,
				held_principal: 100,
				lifetime_compounded_earnings: 5,
				lifetime_principal_deployed: 20,
				lifetime_principal_last_basis_frame: 1,
			};
			sp_io::storage::set(
				&FunderStateByVaultAndAccount::<Test>::hashed_key_for(1, 2),
				&old_state.encode(),
			);

			let mut frame_1_pools = BoundedBTreeMap::new();
			let mut frame_1_pool = TreasuryPool::<Test>::new(1);
			frame_1_pool.bond_holders.try_push((2, 50)).unwrap();
			frame_1_pool.bond_holders.try_push((10, 10)).unwrap();
			frame_1_pools.try_insert(1, frame_1_pool).unwrap();
			VaultPoolsByFrame::<Test>::insert(1, frame_1_pools);

			let mut frame_2_pools = BoundedBTreeMap::new();
			let mut frame_2_pool = TreasuryPool::<Test>::new(1);
			frame_2_pool.bond_holders.try_push((2, 50)).unwrap();
			frame_2_pool.bond_holders.try_push((10, 10)).unwrap();
			frame_2_pools.try_insert(1, frame_2_pool).unwrap();
			VaultPoolsByFrame::<Test>::insert(2, frame_2_pools);

			StorageVersion::new(3).put::<Treasury>();
			SimplifyFunderStateMigration::<Test>::on_runtime_upgrade();

			let state = FunderStateByVaultAndAccount::<Test>::get(1, 2).expect("state migrated");
			assert_eq!(state.held_principal, 100);
			assert_eq!(state.pending_unlock_amount, 40);
			assert_eq!(state.pending_unlock_at_frame, Some(11));
			assert_eq!(state.lifetime_compounded_earnings, 5);
			assert_eq!(state.lifetime_principal_deployed, 20);
			assert_eq!(state.lifetime_principal_last_basis_frame, 1);

			let unlocks = PendingUnlocksByFrame::<Test>::get(11);
			assert_eq!(unlocks.len(), 1);
			assert_eq!(unlocks[0].vault_id, 1);
			assert_eq!(unlocks[0].account_id, 2);
		});
	}
}
