use super::{
	BondLot, BondLotAllocation, BondLotById, BondLotId, BondLotIdsByAccount, BondLotSummary,
	BondLotsByVault, Bonds, Config, CurrentFrameVaultCapital, FrameVaultCapital, NextBondLotId,
	Pallet, VaultCapital,
};
use alloc::{collections::BTreeMap, vec::Vec};
use frame_support::{
	storage_alias,
	traits::{
		UncheckedOnRuntimeUpgrade,
		fungible::{InspectHold, MutateHold},
	},
	weights::Weight,
};
use pallet_prelude::{argon_primitives::MiningFrameTransitionProvider, *};
use sp_runtime::{BoundedBTreeMap, BoundedBTreeSet, FixedU128, Permill};

pub struct MigrateBondLots<T: Config>(core::marker::PhantomData<T>);

mod old_storage {
	use super::*;

	#[derive(Clone, Decode, DecodeWithMemTracking, Encode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct FunderState<T: Config> {
		#[codec(compact)]
		pub target_principal: T::Balance,
		#[codec(compact)]
		pub bonded_principal: T::Balance,
		#[codec(compact)]
		pub held_principal: T::Balance,
		#[codec(compact)]
		pub lifetime_compounded_earnings: T::Balance,
		#[codec(compact)]
		pub lifetime_principal_deployed: T::Balance,
		#[codec(compact)]
		pub lifetime_principal_last_basis_frame: FrameId,
	}

	#[derive(Clone, Decode, DecodeWithMemTracking, Encode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct TreasuryCapital<T: Config> {
		#[codec(compact)]
		pub vault_id: VaultId,
		#[codec(compact)]
		pub activated_capital: T::Balance,
		#[codec(compact)]
		pub frame_id: FrameId,
	}

	#[derive(Clone, Decode, DecodeWithMemTracking, Encode, TypeInfo, Default)]
	#[scale_info(skip_type_params(T))]
	pub struct TreasuryPool<T: Config> {
		pub bond_holders: BoundedVec<(T::AccountId, T::Balance), T::MaxTreasuryContributors>,
		pub distributed_earnings: Option<T::Balance>,
		#[codec(compact)]
		pub vault_sharing_percent: Permill,
	}

	#[storage_alias]
	pub type VaultPoolsByFrame<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		FrameId,
		BoundedBTreeMap<VaultId, TreasuryPool<T>, <T as Config>::MaxVaultsPerPool>,
		ValueQuery,
	>;

	#[storage_alias]
	pub type FunderStateByVaultAndAccount<T: Config> = StorageDoubleMap<
		Pallet<T>,
		Twox64Concat,
		VaultId,
		Twox64Concat,
		<T as frame_system::Config>::AccountId,
		FunderState<T>,
		OptionQuery,
	>;

	#[storage_alias]
	pub type CapitalActive<T: Config> = StorageValue<
		Pallet<T>,
		BoundedVec<TreasuryCapital<T>, <T as Config>::MaxVaultsPerPool>,
		ValueQuery,
	>;

	#[storage_alias]
	pub type FundersByVaultId<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		VaultId,
		BoundedBTreeSet<
			<T as frame_system::Config>::AccountId,
			<T as Config>::MaxTreasuryContributors,
		>,
		ValueQuery,
	>;
}

struct ActiveBondLot<AccountId> {
	vault_id: VaultId,
	account_id: AccountId,
	bond_lot_id: BondLotId,
}

impl<T: Config> UncheckedOnRuntimeUpgrade for MigrateBondLots<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		Ok((
			T::MiningFrameTransitionProvider::get_current_frame_id(),
			old_storage::VaultPoolsByFrame::<T>::iter().count() as u64,
			old_storage::FunderStateByVaultAndAccount::<T>::iter().count() as u64,
			old_storage::FundersByVaultId::<T>::iter().count() as u64,
			old_storage::CapitalActive::<T>::get().len() as u64,
		)
			.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let current_frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
		let mut db_reads = 0u64;
		let mut db_writes = 0u64;

		let legacy_pool_keys: Vec<_> = old_storage::VaultPoolsByFrame::<T>::iter_keys().collect();
		db_reads = db_reads.saturating_add(legacy_pool_keys.len() as u64);
		let current_frame_pools = old_storage::VaultPoolsByFrame::<T>::get(current_frame_id);
		db_reads = db_reads.saturating_add(1);

		let legacy_active_capital = old_storage::CapitalActive::<T>::get();
		db_reads = db_reads.saturating_add(1);

		let legacy_funders: Vec<_> =
			old_storage::FunderStateByVaultAndAccount::<T>::iter().collect();
		db_reads = db_reads.saturating_add(legacy_funders.len() as u64);

		let mut next_bond_lot_id = 0u64;
		let mut active_bond_lots = Vec::new();
		let mut bond_lot_summaries_by_vault =
			BTreeMap::<VaultId, Vec<(T::AccountId, BondLotSummary)>>::new();

		for (vault_id, account_id, legacy_state) in &legacy_funders {
			let migrated_principal = legacy_state.target_principal.min(legacy_state.held_principal);
			let migrated_bonds = balance_to_bonds::<T>(migrated_principal);
			let migrated_balance = bonds_to_balance::<T>(migrated_bonds);
			let immediate_refund = legacy_state.held_principal.saturating_sub(migrated_balance);

			if migrated_bonds > 0 {
				let bond_lot_id = insert_bond_lot::<T>(
					&mut next_bond_lot_id,
					account_id,
					*vault_id,
					migrated_bonds,
					current_frame_id,
					&mut db_writes,
				);
				active_bond_lots.push(ActiveBondLot {
					vault_id: *vault_id,
					account_id: account_id.clone(),
					bond_lot_id,
				});
				bond_lot_summaries_by_vault.entry(*vault_id).or_default().push((
					account_id.clone(),
					BondLotSummary { bond_lot_id, bonds: migrated_bonds },
				));
			}

			if !immediate_refund.is_zero() {
				if let Err(error) = release_migration_refund::<T>(account_id, immediate_refund) {
					log::error!(
						"failed to refund unmigrated treasury principal for account {account_id:?} in vault {vault_id:?}: {error:?}",
					);
				}
				db_writes = db_writes.saturating_add(1);
			}
		}

		for (vault_id, mut summaries) in bond_lot_summaries_by_vault {
			let eligible_bonds = Pallet::<T>::balance_to_bonds(
				Pallet::<T>::get_vault_securitized_funds_cap(vault_id),
			);

			summaries.sort_by(|a, b| compare_summaries(&a.1, &b.1));

			let mut remaining_eligible_bonds = eligible_bonds;
			let mut accepted_summaries = Vec::new();

			for (account_id, mut summary) in summaries {
				let allowed_bonds = if accepted_summaries.len() <
					T::MaxTreasuryContributors::get() as usize &&
					!remaining_eligible_bonds.is_zero()
				{
					summary.bonds.min(remaining_eligible_bonds)
				} else {
					0
				};

				if allowed_bonds < summary.bonds {
					let refund_amount =
						bonds_to_balance::<T>(summary.bonds.saturating_sub(allowed_bonds));
					if let Err(error) = release_migration_refund::<T>(&account_id, refund_amount) {
						log::error!(
							"failed to refund over-cap treasury principal for account {account_id:?} in vault {vault_id:?}: {error:?}",
						);
					}
					db_writes = db_writes.saturating_add(1);

					if allowed_bonds.is_zero() {
						BondLotById::<T>::remove(summary.bond_lot_id);
						BondLotIdsByAccount::<T>::remove(&account_id, summary.bond_lot_id);
						active_bond_lots
							.retain(|bond_lot| bond_lot.bond_lot_id != summary.bond_lot_id);
						db_writes = db_writes.saturating_add(2);
					} else {
						BondLotById::<T>::mutate_exists(summary.bond_lot_id, |entry| {
							let Some(bond_lot) = entry.as_mut() else {
								return;
							};
							bond_lot.bonds = allowed_bonds;
						});
						db_writes = db_writes.saturating_add(1);
					}
				}

				if allowed_bonds.is_zero() {
					continue;
				}

				summary.bonds = allowed_bonds;
				remaining_eligible_bonds = remaining_eligible_bonds.saturating_sub(allowed_bonds);
				accepted_summaries.push(summary);
			}

			let Ok(vault_summaries) =
				BoundedVec::<BondLotSummary, T::MaxTreasuryContributors>::try_from(
					accepted_summaries,
				)
			else {
				continue;
			};
			if vault_summaries.is_empty() {
				continue;
			}
			BondLotsByVault::<T>::insert(vault_id, vault_summaries);
			db_writes = db_writes.saturating_add(1);
		}

		let mut migrated_current_vaults = BoundedBTreeMap::new();
		for legacy_capital in legacy_active_capital
			.into_iter()
			.filter(|capital| capital.frame_id == current_frame_id)
		{
			let vault_id = legacy_capital.vault_id;
			let Some(legacy_pool) = current_frame_pools.get(&vault_id) else {
				continue;
			};

			let eligible_bonds = balance_to_bonds::<T>(legacy_capital.activated_capital);
			let pool_denominator = legacy_capital.activated_capital.into();
			if eligible_bonds.is_zero() || pool_denominator.is_zero() {
				continue;
			}

			let mut bond_lot_allocations = BoundedVec::default();
			for (account_id, amount) in &legacy_pool.bond_holders {
				let Some(bond_lot_id) = active_bond_lots.iter().find_map(|bond_lot| {
					(bond_lot.vault_id == vault_id && bond_lot.account_id == *account_id)
						.then_some(bond_lot.bond_lot_id)
				}) else {
					continue;
				};

				let amount = (*amount).into();
				if amount == 0 {
					continue;
				}

				if bond_lot_allocations
					.try_push(BondLotAllocation {
						bond_lot_id,
						prorata: FixedU128::from_rational(amount, pool_denominator),
					})
					.is_err()
				{
					break;
				}
			}

			if bond_lot_allocations.is_empty() {
				continue;
			}

			let vault_capital = VaultCapital {
				bond_lot_allocations,
				eligible_bonds,
				vault_sharing_percent: legacy_pool.vault_sharing_percent,
			};
			if migrated_current_vaults
				.try_insert(legacy_capital.vault_id, vault_capital)
				.is_err()
			{
				break;
			}
		}

		CurrentFrameVaultCapital::<T>::put(FrameVaultCapital {
			frame_id: current_frame_id,
			vaults: migrated_current_vaults,
		});
		db_writes = db_writes.saturating_add(1);

		NextBondLotId::<T>::put(next_bond_lot_id);
		db_writes = db_writes.saturating_add(1);

		clear_legacy_storage::<T>(legacy_pool_keys, legacy_funders, &mut db_writes);

		T::DbWeight::get().reads_writes(db_reads, db_writes)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;
		use frame_support::ensure;

		let (
			expected_current_frame_id,
			expected_legacy_pool_count,
			expected_legacy_funder_count,
			expected_legacy_funders_by_vault_count,
			expected_legacy_capital_count,
		) = <(FrameId, u64, u64, u64, u64)>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("failed to decode treasury migration state")
		})?;

		ensure!(
			T::MiningFrameTransitionProvider::get_current_frame_id() == expected_current_frame_id,
			"current frame changed during treasury migration",
		);
		ensure!(
			old_storage::VaultPoolsByFrame::<T>::iter().count() == 0,
			"legacy VaultPoolsByFrame was not cleared",
		);
		ensure!(
			old_storage::FunderStateByVaultAndAccount::<T>::iter().count() == 0,
			"legacy FunderStateByVaultAndAccount was not cleared",
		);
		ensure!(
			old_storage::FundersByVaultId::<T>::iter().count() == 0,
			"legacy FundersByVaultId was not cleared",
		);
		ensure!(
			old_storage::CapitalActive::<T>::get().is_empty(),
			"legacy CapitalActive was not cleared",
		);

		let bond_lot_count = BondLotById::<T>::iter().count() as u64;
		let next_bond_lot_id = NextBondLotId::<T>::get();
		ensure!(
			next_bond_lot_id >= bond_lot_count as BondLotId,
			"NextBondLotId is below the number of migrated bond lots",
		);

		for (account_id, bond_lot_id, ()) in BondLotIdsByAccount::<T>::iter() {
			let Some(bond_lot) = BondLotById::<T>::get(bond_lot_id) else {
				return Err(sp_runtime::TryRuntimeError::Other(
					"BondLotIdsByAccount pointed at a missing bond lot",
				));
			};
			ensure!(bond_lot.owner == account_id, "bond lot account index owner mismatch");
		}

		for (vault_id, summaries) in BondLotsByVault::<T>::iter() {
			for summary in summaries {
				let Some(bond_lot) = BondLotById::<T>::get(summary.bond_lot_id) else {
					return Err(sp_runtime::TryRuntimeError::Other(
						"BondLotsByVault pointed at a missing bond lot",
					));
				};
				ensure!(bond_lot.vault_id == vault_id, "accepted lot vault mismatch");
				ensure!(bond_lot.bonds == summary.bonds, "accepted lot bond amount mismatch");
				ensure!(
					bond_lot.release_reason.is_none(),
					"accepted bond lot should not already be releasing",
				);
			}
		}

		if let Some(current_frame_vault_capital) = CurrentFrameVaultCapital::<T>::get() {
			ensure!(
				current_frame_vault_capital.frame_id == expected_current_frame_id,
				"migrated current frame capital has the wrong frame id",
			);
			for (vault_id, vault_capital) in current_frame_vault_capital.vaults {
				for allocation in vault_capital.bond_lot_allocations {
					let Some(bond_lot) = BondLotById::<T>::get(allocation.bond_lot_id) else {
						return Err(sp_runtime::TryRuntimeError::Other(
							"current frame allocation pointed at a missing bond lot",
						));
					};
					ensure!(
						bond_lot.vault_id == vault_id,
						"current frame allocation vault mismatch"
					);
				}
			}
		}

		ensure!(
			expected_legacy_funder_count >= bond_lot_count &&
				expected_legacy_funders_by_vault_count >=
					BondLotsByVault::<T>::iter().count() as u64 &&
				expected_legacy_capital_count >=
					CurrentFrameVaultCapital::<T>::get()
						.map(|capital| capital.vaults.len() as u64)
						.unwrap_or_default(),
			"treasury migration produced more tracked state than the legacy source",
		);
		ensure!(
			expected_legacy_pool_count >=
				CurrentFrameVaultCapital::<T>::get().map(|_| 1u64).unwrap_or_default(),
			"treasury migration produced current frame capital without a legacy frame pool",
		);

		Ok(())
	}
}

fn clear_legacy_storage<T: Config>(
	legacy_pool_keys: Vec<FrameId>,
	legacy_funders: Vec<(VaultId, T::AccountId, old_storage::FunderState<T>)>,
	db_writes: &mut u64,
) {
	for frame_id in legacy_pool_keys {
		old_storage::VaultPoolsByFrame::<T>::remove(frame_id);
		*db_writes = db_writes.saturating_add(1);
	}

	for (vault_id, account_id, _) in legacy_funders {
		old_storage::FunderStateByVaultAndAccount::<T>::remove(vault_id, account_id);
		*db_writes = db_writes.saturating_add(1);
	}

	let legacy_tracked_vaults: Vec<_> = old_storage::FundersByVaultId::<T>::iter_keys().collect();
	for vault_id in legacy_tracked_vaults {
		old_storage::FundersByVaultId::<T>::remove(vault_id);
		*db_writes = db_writes.saturating_add(1);
	}

	old_storage::CapitalActive::<T>::kill();
	*db_writes = db_writes.saturating_add(1);
}

fn insert_bond_lot<T: Config>(
	next_bond_lot_id: &mut BondLotId,
	account_id: &T::AccountId,
	vault_id: VaultId,
	bonds: Bonds,
	created_frame_id: FrameId,
	db_writes: &mut u64,
) -> BondLotId {
	let bond_lot_id = *next_bond_lot_id;
	*next_bond_lot_id = next_bond_lot_id.saturating_add(1);

	BondLotById::<T>::insert(
		bond_lot_id,
		BondLot {
			owner: account_id.clone(),
			vault_id,
			bonds,
			created_frame_id,
			participated_frames: 0,
			last_frame_earnings_frame_id: None,
			last_frame_earnings: None,
			cumulative_earnings: T::Balance::zero(),
			release_frame_id: None,
			release_reason: None,
		},
	);
	BondLotIdsByAccount::<T>::insert(account_id, bond_lot_id, ());
	*db_writes = db_writes.saturating_add(2);
	bond_lot_id
}

fn release_migration_refund<T: Config>(
	account_id: &T::AccountId,
	amount: T::Balance,
) -> DispatchResult {
	if amount.is_zero() {
		return Ok(());
	}
	let reason = super::HoldReason::ContributedToTreasury;
	T::Currency::release(&reason.into(), account_id, amount, Precision::Exact)?;

	if T::Currency::balance_on_hold(&reason.into(), account_id).is_zero() {
		frame_system::Pallet::<T>::dec_providers(account_id)?;
	}

	Ok(())
}

fn compare_summaries(a: &BondLotSummary, b: &BondLotSummary) -> core::cmp::Ordering {
	b.bonds.cmp(&a.bonds).then_with(|| a.bond_lot_id.cmp(&b.bond_lot_id))
}

fn bonds_to_balance<T: Config>(bonds: Bonds) -> T::Balance {
	(bonds as u128).saturating_mul(argon_primitives::MICROGONS_PER_ARGON).into()
}

fn balance_to_bonds<T: Config>(balance: T::Balance) -> Bonds {
	let bonds = balance.into() / argon_primitives::MICROGONS_PER_ARGON;
	bonds.min(Bonds::MAX as u128) as Bonds
}

pub type BondLotsMigration<T> = frame_support::migrations::VersionedMigration<
	3,
	4,
	MigrateBondLots<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod test {
	use super::{
		super::{
			BondLotById, BondLotIdsByAccount, BondLotsByVault, CurrentFrameVaultCapital,
			HoldReason, NextBondLotId, PendingBondReleaseRetryCursor,
			mock::{
				Balances, CurrentFrameId, RuntimeHoldReason, Test, TestVault, Treasury,
				insert_vault, new_test_ext, set_argons,
			},
		},
		BondLotsMigration, old_storage,
	};
	use argon_primitives::MICROGONS_PER_ARGON;
	use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
	use pallet_prelude::*;
	use sp_runtime::Permill;

	#[test]
	fn migrates_legacy_aggregate_state_into_bond_lots() {
		new_test_ext().execute_with(|| {
			insert_vault(
				1,
				TestVault {
					account_id: 10,
					securitized_satoshis: (100 * MICROGONS_PER_ARGON) as u64,
					sharing_percent: Permill::from_percent(20),
					is_closed: false,
				},
			);
			CurrentFrameId::set(2);

			StorageVersion::new(3).put::<Treasury>();

			let one_bond = MICROGONS_PER_ARGON;
			let hold_reason = RuntimeHoldReason::from(HoldReason::ContributedToTreasury);
			set_argons(2, 7 * one_bond);
			frame_system::Pallet::<Test>::inc_providers(&2);
			assert!(Balances::hold(&hold_reason, &2, 7 * one_bond).is_ok());

			old_storage::FunderStateByVaultAndAccount::<Test>::insert(
				1,
				2,
				old_storage::FunderState {
					target_principal: 5 * one_bond,
					bonded_principal: 4 * one_bond,
					held_principal: 7 * one_bond,
					lifetime_compounded_earnings: 0,
					lifetime_principal_deployed: 0,
					lifetime_principal_last_basis_frame: 0,
				},
			);

			let mut current_pools =
				BoundedBTreeMap::<VaultId, old_storage::TreasuryPool<Test>, _>::new();
			assert!(
				current_pools
					.try_insert(
						1,
						old_storage::TreasuryPool::<Test> {
							bond_holders: BoundedVec::truncate_from(vec![(2, 4 * one_bond)]),
							distributed_earnings: None,
							vault_sharing_percent: Permill::from_percent(20),
						},
					)
					.is_ok()
			);
			old_storage::VaultPoolsByFrame::<Test>::insert(2, current_pools);
			old_storage::CapitalActive::<Test>::put(BoundedVec::truncate_from(vec![
				old_storage::TreasuryCapital {
					vault_id: 1,
					activated_capital: 4 * one_bond,
					frame_id: 2,
				},
			]));

			BondLotsMigration::<Test>::on_runtime_upgrade();

			assert_eq!(NextBondLotId::<Test>::get(), 1);
			assert_eq!(BondLotById::<Test>::iter().count(), 1);

			let accepted = BondLotsByVault::<Test>::get(1);
			assert_eq!(accepted.len(), 1);
			let active_lot = BondLotById::<Test>::get(accepted[0].bond_lot_id).expect("active lot");
			assert_eq!(active_lot.owner, 2);
			assert_eq!(active_lot.bonds, 5);
			assert_eq!(active_lot.release_reason, None);
			assert_eq!(Balances::balance_on_hold(&hold_reason, &2), 5 * one_bond);

			let current_frame_capital =
				CurrentFrameVaultCapital::<Test>::get().expect("current frame capital");
			assert_eq!(current_frame_capital.frame_id, 2);
			assert_eq!(current_frame_capital.vaults.get(&1).unwrap().eligible_bonds, 4);
			assert_eq!(PendingBondReleaseRetryCursor::<Test>::get(), None);
		});
	}

	#[test]
	fn migration_caps_future_bond_lots_by_vault_security_not_current_frame_participation() {
		new_test_ext().execute_with(|| {
			insert_vault(
				1,
				TestVault {
					account_id: 10,
					securitized_satoshis: (2 * MICROGONS_PER_ARGON) as u64,
					sharing_percent: Permill::from_percent(20),
					is_closed: false,
				},
			);
			CurrentFrameId::set(2);

			StorageVersion::new(3).put::<Treasury>();

			let one_bond = MICROGONS_PER_ARGON;
			let hold_reason = RuntimeHoldReason::from(HoldReason::ContributedToTreasury);
			set_argons(2, 3 * one_bond);
			frame_system::Pallet::<Test>::inc_providers(&2);
			assert!(Balances::hold(&hold_reason, &2, 3 * one_bond).is_ok());

			old_storage::FunderStateByVaultAndAccount::<Test>::insert(
				1,
				2,
				old_storage::FunderState {
					target_principal: 3 * one_bond,
					bonded_principal: 3 * one_bond,
					held_principal: 3 * one_bond,
					lifetime_compounded_earnings: 0,
					lifetime_principal_deployed: 0,
					lifetime_principal_last_basis_frame: 0,
				},
			);

			BondLotsMigration::<Test>::on_runtime_upgrade();

			assert_eq!(NextBondLotId::<Test>::get(), 1);
			assert_eq!(BondLotById::<Test>::iter().count(), 1);
			let accepted = BondLotsByVault::<Test>::get(1);
			assert_eq!(accepted.len(), 1);
			assert_eq!(accepted[0].bonds, 2);

			let active_lot = BondLotById::<Test>::get(accepted[0].bond_lot_id).expect("active lot");
			assert_eq!(active_lot.owner, 2);
			assert_eq!(active_lot.bonds, 2);
			assert!(BondLotIdsByAccount::<Test>::iter_key_prefix(2).next().is_some());

			let current_frame_capital =
				CurrentFrameVaultCapital::<Test>::get().expect("current frame capital");
			assert_eq!(current_frame_capital.frame_id, 2);
			assert!(current_frame_capital.vaults.is_empty());
			assert_eq!(Balances::balance_on_hold(&hold_reason, &2), 2 * one_bond);
		});
	}
}
