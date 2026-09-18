use crate::{
	BondLotById, BondLotId, BondLotIdsByAccount, BondProgram, Config, HoldReason, Pallet,
	PendingBondReleaseRetryCursor, PendingBondReleasesByFrame,
};
use alloc::{
	collections::{BTreeMap, BTreeSet},
	vec::Vec,
};
use argon_primitives::{prelude::FrameId, MiningFrameTransitionProvider, MICROGONS_PER_ARGON};
use frame_support::traits::{fungible::InspectHold, UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;
use sp_runtime::AccountId32;

#[cfg(feature = "try-runtime")]
use codec::{Decode, Encode};
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum BondCurrency {
	Argon,
	Argonot,
}

struct AccountBondLots {
	expected_hold: u128,
	overdue_total: u128,
	overdue_lots: Vec<(BondLotId, FrameId)>,
	is_consistent: bool,
}

impl Default for AccountBondLots {
	fn default() -> Self {
		Self { expected_hold: 0, overdue_total: 0, overdue_lots: Vec::new(), is_consistent: true }
	}
}

struct CleanupPlan {
	bond_lot_ids: BTreeSet<BondLotId>,
	queue_entries: BTreeMap<FrameId, BTreeSet<BondLotId>>,
	retry_frame: Option<FrameId>,
	reads: u64,
}

fn cleanup_plan<T: Config>() -> CleanupPlan {
	let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
	let existing_retry_frame = PendingBondReleaseRetryCursor::<T>::get();
	let mut groups = BTreeMap::<(AccountId32, BondCurrency), AccountBondLots>::new();
	let mut queued_releases = BTreeMap::<FrameId, BTreeSet<BondLotId>>::new();
	let mut reads = 2u64;

	for (bond_lot_id, bond_lot) in BondLotById::<T>::iter() {
		reads.saturating_accrue(1);
		let currency = match bond_lot.program {
			BondProgram::Vault { .. } => BondCurrency::Argon,
			BondProgram::Argonot => BondCurrency::Argonot,
		};
		let lot_amount = u128::from(bond_lot.bonds).saturating_mul(MICROGONS_PER_ARGON);
		let group = groups.entry((bond_lot.owner.clone(), currency)).or_default();
		if let Some(expected_hold) = group.expected_hold.checked_add(lot_amount) {
			group.expected_hold = expected_hold;
		} else {
			group.is_consistent = false;
		}

		let Some(release_frame) = bond_lot.release_frame_id.filter(|frame| *frame <= current_frame)
		else {
			continue;
		};

		if let Some(overdue_total) = group.overdue_total.checked_add(lot_amount) {
			group.overdue_total = overdue_total;
		} else {
			group.is_consistent = false;
		}
		group.overdue_lots.push((bond_lot_id, release_frame));

		reads.saturating_accrue(1);
		let has_account_index =
			BondLotIdsByAccount::<T>::contains_key(&bond_lot.owner, bond_lot_id);
		let pending_releases = queued_releases.entry(release_frame).or_insert_with(|| {
			reads.saturating_accrue(1);
			PendingBondReleasesByFrame::<T>::get(release_frame).into_iter().collect()
		});
		if !has_account_index || !pending_releases.contains(&bond_lot_id) {
			group.is_consistent = false;
		}
	}

	let hold_reason = HoldReason::ContributedToTreasury.into();
	let mut bond_lot_ids = BTreeSet::new();
	let mut queue_entries = BTreeMap::<FrameId, BTreeSet<BondLotId>>::new();
	let mut retry_frame = None;

	for ((owner, currency), group) in groups {
		reads.saturating_accrue(1);
		let actual_hold: u128 = match currency {
			BondCurrency::Argon => T::Currency::balance_on_hold(&hold_reason, &owner).into(),
			BondCurrency::Argonot =>
				T::OwnershipCurrency::balance_on_hold(&hold_reason, &owner).into(),
		};
		let can_clean = group.is_consistent &&
			group.overdue_total != 0 &&
			group.expected_hold.checked_sub(actual_hold) == Some(group.overdue_total);

		for (bond_lot_id, release_frame) in group.overdue_lots {
			if can_clean {
				bond_lot_ids.insert(bond_lot_id);
				queue_entries.entry(release_frame).or_default().insert(bond_lot_id);
			} else {
				retry_frame = Some(
					retry_frame.map_or(release_frame, |oldest: FrameId| oldest.min(release_frame)),
				);
			}
		}
	}
	if let (Some(existing), Some(retry)) = (existing_retry_frame, retry_frame.as_mut()) {
		*retry = (*retry).min(existing);
	}

	CleanupPlan { bond_lot_ids, queue_entries, retry_frame, reads }
}

/// Remove release records left behind when the old release path mutated a hold before returning
/// `ConsumerRemaining`. The migration scans all live lots once and only cleans complete account and
/// currency groups whose hold deficit exactly matches every overdue queued lot in that group.
pub struct CleanupStrandedBondLots<T>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for CleanupStrandedBondLots<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		Ok(cleanup_plan::<T>().bond_lot_ids.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let plan = cleanup_plan::<T>();
		let reads = plan
			.reads
			.saturating_add(plan.bond_lot_ids.len() as u64)
			.saturating_add(plan.queue_entries.len() as u64);
		let mut writes = 0u64;

		for bond_lot_id in &plan.bond_lot_ids {
			if let Some(bond_lot) = BondLotById::<T>::take(bond_lot_id) {
				BondLotIdsByAccount::<T>::remove(&bond_lot.owner, bond_lot_id);
				writes.saturating_accrue(2);
			}
		}

		for (release_frame, removed_ids) in &plan.queue_entries {
			PendingBondReleasesByFrame::<T>::mutate(release_frame, |pending| {
				pending.retain(|bond_lot_id| !removed_ids.contains(bond_lot_id));
			});
			writes.saturating_accrue(1);
		}

		match plan.retry_frame {
			Some(retry_frame) => PendingBondReleaseRetryCursor::<T>::put(retry_frame),
			None => PendingBondReleaseRetryCursor::<T>::kill(),
		}
		writes.saturating_accrue(1);

		log::info!("Cleaned {} stranded treasury bond lots", plan.bond_lot_ids.len());
		T::DbWeight::get().reads_writes(reads, writes)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let cleaned_ids = BTreeSet::<BondLotId>::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("could not decode treasury cleanup state"))?;
		for bond_lot_id in cleaned_ids {
			ensure!(
				!BondLotById::<T>::contains_key(bond_lot_id),
				TryRuntimeError::Other("stranded treasury bond lot was not removed"),
			);
		}
		ensure!(
			cleanup_plan::<T>().bond_lot_ids.is_empty(),
			TryRuntimeError::Other("eligible stranded treasury bond lots remain"),
		);
		Ok(())
	}
}

pub type CleanupStrandedBondLotsMigration<T> = frame_support::migrations::VersionedMigration<
	7,
	8,
	CleanupStrandedBondLots<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		mock::{
			account_id_from_seed, new_test_ext, set_argons, set_ownership, Balances,
			CurrentFrameId, Ownership, RuntimeHoldReason, Test, Treasury,
		},
		BondLot, BondReleaseReason,
	};
	use frame_support::{
		assert_ok,
		traits::{fungible::InspectHold, OnRuntimeUpgrade, StorageVersion},
	};
	use sp_runtime::Permill;

	fn bond_lot(
		owner: AccountId32,
		program: BondProgram,
		bonds: u32,
		release_frame_id: Option<FrameId>,
	) -> BondLot<Test> {
		BondLot {
			owner,
			program,
			bonds,
			is_flexible: false,
			created_frame_id: 1,
			participated_frames: 0,
			last_frame_earnings_frame_id: None,
			last_frame_earnings: None,
			cumulative_earnings: 0,
			release_frame_id,
			release_reason: release_frame_id.map(|_| BondReleaseReason::UserLiquidation),
		}
	}

	#[test]
	fn cleans_only_complete_overdue_deficits() {
		new_test_ext().execute_with(|| {
			CurrentFrameId::set(20);
			StorageVersion::new(7).put::<Pallet<Test>>();
			let safe_owner = account_id_from_seed(1);
			let partial_owner = account_id_from_seed(2);
			let unit = MICROGONS_PER_ARGON;

			set_ownership(&safe_owner, 3 * unit);
			assert_ok!(Treasury::create_hold::<Ownership>(&safe_owner, 2 * unit));
			BondLotById::<Test>::insert(
				1,
				bond_lot(safe_owner.clone(), BondProgram::Argonot, 1, Some(10)),
			);
			BondLotById::<Test>::insert(
				2,
				bond_lot(safe_owner.clone(), BondProgram::Argonot, 2, None),
			);
			BondLotIdsByAccount::<Test>::insert(&safe_owner, 1, ());
			BondLotIdsByAccount::<Test>::insert(&safe_owner, 2, ());

			set_argons(&partial_owner, 3 * unit);
			assert_ok!(Treasury::create_hold::<Balances>(&partial_owner, 2 * unit));
			let vault_program = BondProgram::Vault {
				vault_id: 1,
				sharing_percent: Permill::zero(),
				bonus_percent: Permill::zero(),
			};
			BondLotById::<Test>::insert(
				3,
				bond_lot(partial_owner.clone(), vault_program, 1, Some(10)),
			);
			BondLotById::<Test>::insert(
				4,
				bond_lot(partial_owner.clone(), vault_program, 1, Some(11)),
			);
			BondLotById::<Test>::insert(5, bond_lot(partial_owner.clone(), vault_program, 1, None));
			for bond_lot_id in 3..=5 {
				BondLotIdsByAccount::<Test>::insert(&partial_owner, bond_lot_id, ());
			}
			PendingBondReleasesByFrame::<Test>::insert(10, BoundedVec::truncate_from(vec![1, 3]));
			PendingBondReleasesByFrame::<Test>::insert(11, BoundedVec::truncate_from(vec![4]));

			CleanupStrandedBondLotsMigration::<Test>::on_runtime_upgrade();

			assert!(!BondLotById::<Test>::contains_key(1));
			assert!(!BondLotIdsByAccount::<Test>::contains_key(&safe_owner, 1));
			assert!(BondLotById::<Test>::contains_key(2));
			assert_eq!(PendingBondReleasesByFrame::<Test>::get(10).as_slice(), &[3]);
			assert!(BondLotById::<Test>::contains_key(3));
			assert!(BondLotById::<Test>::contains_key(4));
			assert_eq!(PendingBondReleasesByFrame::<Test>::get(11).as_slice(), &[4]);
			assert_eq!(PendingBondReleaseRetryCursor::<Test>::get(), Some(10));
			assert_eq!(
				Ownership::balance_on_hold(
					&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
					&safe_owner,
				),
				2 * unit,
			);
			assert_eq!(
				Balances::balance_on_hold(
					&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
					&partial_owner,
				),
				2 * unit,
			);
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), StorageVersion::new(8));
		});
	}
}
