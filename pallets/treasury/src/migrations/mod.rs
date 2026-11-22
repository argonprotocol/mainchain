use crate::{
	ActiveBondsByAccountAndVault, AvailableBondCollateralLots, BondCollateralLot, BondLot,
	BondLotById, BondProgram, BondReleaseReason, Bonds, Config, CurrentFrameVaultCapital,
	EncumberedBondLotsByAccount, EncumberedBondMicrogonsByAccount, FrameVaultCapital, Pallet,
	RewardReserves, VaultSecuritizationPosition,
};
use alloc::collections::BTreeMap;
use argon_primitives::{
	block_seal::FrameId,
	treasury::{BondTotal, TreasuryPositionProvider},
	VaultId, MICROGONS_PER_ARGON,
};
use frame_support::traits::OnRuntimeUpgrade;
use pallet_prelude::*;
use polkadot_sdk::sp_runtime::traits::AccountIdConversion;

/// Virtualize marketing reserves and start indexed earnings from the live upgrade cut.
pub struct CollectMigration<T>(PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for CollectMigration<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut weight = T::DbWeight::get().reads(1);
		let account: T::AccountId =
			T::PalletId::get().into_sub_account_truncating(*b"treasury-reserve");
		let balance = T::Currency::balance(&account);
		if !balance.is_zero() {
			match T::Currency::burn_from(
				&account,
				balance,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			) {
				Ok(burned) => {
					RewardReserves::<T>::mutate(|reserves| {
						reserves.marketing.saturating_accrue(burned);
					});
				},
				Err(error) => {
					tracing::error!(?error, "Could not virtualize the legacy marketing reserve");
				},
			}
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(3, 3));
		}

		weight = weight.saturating_add(T::DbWeight::get().reads(1));
		if StorageVersion::get::<Pallet<T>>() != StorageVersion::new(9) {
			return weight
		}

		let mut lots = 0u64;
		let mut collateral_accounts = 0u64;
		BondLotById::<T>::translate::<BondLotV9<T>, _>(|_, old| {
			lots.saturating_accrue(1);
			Some(BondLot {
				owner: old.owner,
				program: old.program,
				bonds: old.bonds,
				is_flexible: old.is_flexible,
				created_frame_id: old.created_frame_id,
				participated_frames: old.participated_frames,
				last_frame_earnings_frame_id: old.last_frame_earnings_frame_id,
				last_frame_earnings: old.last_frame_earnings,
				cumulative_earnings: old.cumulative_earnings,
				release_frame_id: old.release_frame_id,
				release_reason: old.release_reason,
			})
		});

		let mut principal = BTreeMap::<T::AccountId, (T::Balance, T::Balance)>::new();
		let mut active_lots = BTreeMap::<T::AccountId, Vec<(u64, T::Balance)>>::new();
		for (id, lot) in BondLotById::<T>::iter() {
			let BondProgram::Vault { vault_id, .. } = lot.program else { continue };
			let amount: T::Balance =
				u128::from(lot.bonds).saturating_mul(MICROGONS_PER_ARGON).into();
			let amounts = principal.entry(lot.owner.clone()).or_default();
			if lot.release_reason.is_some() {
				amounts.1.saturating_accrue(amount);
				continue
			}
			amounts.0.saturating_accrue(amount);
			active_lots.entry(lot.owner.clone()).or_default().push((id, amount));
			ActiveBondsByAccountAndVault::<T>::mutate(&lot.owner, vault_id, |total| {
				total.saturating_accrue(u128::from(lot.bonds));
			});
			AvailableBondCollateralLots::<T>::insert(&lot.owner, id, ());
		}
		for (account, amount) in EncumberedBondMicrogonsByAccount::<T>::iter() {
			collateral_accounts.saturating_accrue(1);
			let (active, releasing) = principal.get(&account).copied().unwrap_or_default();
			let fractional = amount.saturating_sub(active);
			let mut remaining = amount.saturating_sub(fractional);
			let mut backing =
				BoundedVec::<BondCollateralLot<T::Balance>, T::MaxBondCollateralLots>::default();
			let candidates = active_lots.entry(account.clone()).or_default();
			candidates.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
			for (id, principal) in candidates.iter().take(T::MaxBondCollateralLots::get() as usize)
			{
				if remaining.is_zero() {
					break
				}
				let reserved = remaining.min(*principal);
				let _ = backing.try_push(BondCollateralLot {
					bond_lot_id: Some(*id),
					reserved_microgons: reserved,
				});
				remaining.saturating_reduce(reserved);
			}
			if !remaining.is_zero() {
				backing.clear();
				log::error!(
					"Existing Treasury collateral requires a selection within the lot limit"
				);
			}
			if !fractional.is_zero() {
				if let Some(first) = backing.get_mut(0) {
					first.reserved_microgons.saturating_accrue(fractional);
				} else {
					let _ = backing.try_push(BondCollateralLot {
						bond_lot_id: None,
						reserved_microgons: fractional,
					});
				}
			}
			for entry in &backing {
				if let Some(id) = entry.bond_lot_id {
					let lot_principal = candidates
						.iter()
						.find(|(lot, _)| *lot == id)
						.map(|(_, value)| *value)
						.unwrap_or_default();
					if entry.reserved_microgons >= lot_principal {
						AvailableBondCollateralLots::<T>::remove(&account, id);
					}
				}
			}
			if !backing.is_empty() {
				EncumberedBondLotsByAccount::<T>::insert(&account, backing);
			}

			let required = active.max(amount).saturating_add(releasing);
			let held = T::Currency::balance_on_hold(
				&crate::HoldReason::ContributedToTreasury.into(),
				&account,
			);
			let surplus = held.saturating_sub(required);
			if let Err(error) = Pallet::<T>::release_hold::<T::Currency>(&account, surplus) {
				log::error!("Could not release surplus Treasury hold at cutover: {error:?}");
			}
		}

		let quantities = T::PositionProvider::network_totals().account_quantities;
		CurrentFrameVaultCapital::<T>::translate::<FrameVaultCapitalV9<T>, _>(|old| {
			old.map(|old| FrameVaultCapital {
				frame_id: old.frame_id,
				total_active_bonds: old.total_active_bonds,
				quantities,
				target_securitization: old.target_securitization,
				total_securitization: old.total_securitization,
				vault_securitization_positions: old.vault_securitization_positions,
			})
		})
		.expect("supported frame snapshot decodes");
		CurrentFrameArgonotBondParticipants::<T>::kill();
		OutstandingPurchaseRewards::<T>::kill();
		StorageVersion::new(10).put::<Pallet<T>>();

		weight.saturating_add(
			T::DbWeight::get().reads_writes(
				lots.saturating_mul(8)
					.saturating_add(collateral_accounts.saturating_mul(3))
					.saturating_add(3),
				lots.saturating_mul(7).saturating_add(collateral_accounts).saturating_add(4),
			),
		)
	}
}

#[frame_support::storage_alias]
pub(crate) type CurrentFrameArgonotBondParticipants<T: Config> =
	StorageValue<Pallet<T>, (), OptionQuery>;

#[derive(Encode, Decode)]
pub(crate) struct FrameVaultCapitalV9<T: Config> {
	#[codec(compact)]
	pub frame_id: FrameId,
	#[codec(compact)]
	pub total_active_bonds: BondTotal,
	#[codec(compact)]
	pub target_securitization: T::Balance,
	#[codec(compact)]
	pub total_securitization: T::Balance,
	pub vault_securitization_positions:
		BoundedBTreeMap<VaultId, VaultSecuritizationPosition<T>, T::MaxVaultsPerPool>,
}

/// Lot encoding before account indexes replaced per-lot frame snapshots.
#[derive(Encode, Decode)]
pub(crate) struct BondLotV9<T: Config> {
	pub owner: T::AccountId,
	pub program: BondProgram,
	#[codec(compact)]
	pub bonds: Bonds,
	pub is_flexible: bool,
	pub locked_frame_terms: Option<LockedFrameBondTermsV9>,
	#[codec(compact)]
	pub created_frame_id: FrameId,
	#[codec(compact)]
	pub participated_frames: u32,
	pub last_frame_earnings_frame_id: Option<FrameId>,
	pub last_frame_earnings: Option<T::Balance>,
	#[codec(compact)]
	pub cumulative_earnings: T::Balance,
	pub release_frame_id: Option<FrameId>,
	pub release_reason: Option<BondReleaseReason>,
}

#[derive(Encode, Decode)]
pub(crate) struct LockedFrameBondTermsV9 {
	#[codec(compact)]
	pub bonds: Bonds,
	pub is_flexible: bool,
}

/// Outstanding purchase rewards in Treasury storage version 9.
#[frame_support::storage_alias]
pub(crate) type OutstandingPurchaseRewards<T: Config> =
	StorageValue<Pallet<T>, <T as Config>::Balance, ValueQuery>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, Error, HoldReason, PendingBondCollateralByAccount};
	use argon_primitives::{
		treasury::{PositionQuantity, TreasuryPositionProvider},
		AscendingRankKey, TreasuryPoolProvider,
	};
	use frame_support::{
		assert_noop, assert_ok,
		storage::unhashed,
		traits::{
			fungible::{Inspect, InspectHold, Unbalanced},
			Hooks,
		},
	};
	use sp_runtime::{traits::AccountIdConversion, Permill};

	type TreasuryMigration = CollectMigration<Test>;

	#[test]
	fn marketing_migration_removes_issuance_once() {
		new_test_ext().execute_with(|| {
			let account: TestAccountId =
				VaultPalletId::get().into_sub_account_truncating(*b"treasury-reserve");
			set_argons(&account, 1_000_000);
			let issuance_before = Balances::total_issuance();

			TreasuryMigration::on_runtime_upgrade();

			assert_eq!(Balances::balance(&account), 0);
			assert_eq!(Balances::total_issuance(), issuance_before - 1_000_000);
			assert_eq!(RewardReserves::<Test>::get().marketing, 1_000_000);

			TreasuryMigration::on_runtime_upgrade();
			assert_eq!(RewardReserves::<Test>::get().marketing, 1_000_000);
		});
	}

	#[test]
	fn failed_marketing_migration_preserves_funds_for_retry() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(9).put::<Treasury>();
			let account: TestAccountId =
				VaultPalletId::get().into_sub_account_truncating(*b"treasury-reserve");
			set_argons(&account, 1_000_000);
			let issuance = Balances::total_issuance();
			Balances::set_total_issuance(0);

			TreasuryMigration::on_runtime_upgrade();
			assert_eq!(Balances::balance(&account), 1_000_000);
			assert_eq!(RewardReserves::<Test>::get().marketing, 0);
			assert_eq!(StorageVersion::get::<Treasury>(), StorageVersion::new(10));

			Balances::set_total_issuance(issuance);
			TreasuryMigration::on_runtime_upgrade();
			assert_eq!(Balances::balance(&account), 0);
			assert_eq!(RewardReserves::<Test>::get().marketing, 1_000_000);
		});
	}

	#[test]
	fn collect_migration_preserves_lots_frozen_capital_and_is_idempotent() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(9).put::<Treasury>();
			CurrentFrameId::set(7);
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&account(2),
				PositionQuantity::Bonds,
				0u128,
				5u128
			));
			let old = crate::migrations::FrameVaultCapitalV9::<Test> {
				frame_id: 7,
				total_active_bonds: 2,
				target_securitization: 100,
				total_securitization: 90,
				vault_securitization_positions: BoundedBTreeMap::new(),
			};
			unhashed::put(&CurrentFrameVaultCapital::<Test>::hashed_key(), &old);
			crate::migrations::CurrentFrameArgonotBondParticipants::<Test>::put(());
			let program = BondProgram::Vault {
				vault_id: 3,
				sharing_percent: Permill::from_percent(20),
				bonus_percent: Permill::from_percent(5),
			};
			for (id, terms) in [
				(7, None),
				(
					8,
					Some(crate::migrations::LockedFrameBondTermsV9 {
						bonds: 40,
						is_flexible: false,
					}),
				),
			] {
				unhashed::put(
					&BondLotById::<Test>::hashed_key_for(id),
					&crate::migrations::BondLotV9::<Test> {
						owner: account(2),
						program,
						bonds: 50,
						is_flexible: true,
						locked_frame_terms: terms,
						created_frame_id: 4,
						participated_frames: 25,
						last_frame_earnings_frame_id: Some(31),
						last_frame_earnings: Some(120_000),
						cumulative_earnings: 3_000_000,
						release_frame_id: Some(40),
						release_reason: Some(BondReleaseReason::UserLiquidation),
					},
				);
			}

			TreasuryMigration::on_runtime_upgrade();
			let frame = CurrentFrameVaultCapital::<Test>::get().unwrap();
			assert_eq!(frame.frame_id, 7);
			assert_eq!(frame.total_active_bonds, 2);
			assert_eq!(frame.quantities.bonds, 5);
			assert_eq!(frame.target_securitization, 100);
			assert_eq!(frame.total_securitization, 90);
			assert!(!crate::migrations::CurrentFrameArgonotBondParticipants::<Test>::exists());
			assert_eq!(StorageVersion::get::<Treasury>(), StorageVersion::new(10));
			let expected = BondLot::<Test> {
				owner: account(2),
				program,
				bonds: 50,
				is_flexible: true,
				created_frame_id: 4,
				participated_frames: 25,
				last_frame_earnings_frame_id: Some(31),
				last_frame_earnings: Some(120_000),
				cumulative_earnings: 3_000_000,
				release_frame_id: Some(40),
				release_reason: Some(BondReleaseReason::UserLiquidation),
			};
			for id in [7, 8] {
				assert_eq!(BondLotById::<Test>::get(id), Some(expected.clone()));
			}
			TreasuryMigration::on_runtime_upgrade();
			assert_eq!(CurrentFrameVaultCapital::<Test>::get().unwrap(), frame);
			for id in [7, 8] {
				assert_eq!(BondLotById::<Test>::get(id), Some(expected.clone()));
			}
		});
	}

	#[test]
	fn migration_preserves_bounded_and_unassigned_collateral_with_exact_fractional_backing() {
		for lot_count in [10, 11] {
			new_test_ext().execute_with(|| {
				MinimumArgonsPerContributor::set(1);
				CurrentFrameId::set(0);
				insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
				set_argons(2, 100 * MICROGONS_PER_ARGON);
				for _ in 0..lot_count {
					assert_ok!(Treasury::buy_bonds(origin(2), 1, 1, None));
				}
				for id in account_bond_lot_ids(2) {
					let lot = BondLotById::<Test>::get(id).unwrap();
					unhashed::put(
						&BondLotById::<Test>::hashed_key_for(id),
						&crate::migrations::BondLotV9::<Test> {
							owner: lot.owner,
							program: lot.program,
							bonds: lot.bonds,
							is_flexible: lot.is_flexible,
							locked_frame_terms: None,
							created_frame_id: lot.created_frame_id,
							participated_frames: 0,
							last_frame_earnings_frame_id: None,
							last_frame_earnings: None,
							cumulative_earnings: 0,
							release_frame_id: None,
							release_reason: None,
						},
					);
				}
				let owner = account(2);
				let reserved = lot_count * MICROGONS_PER_ARGON + MICROGONS_PER_ARGON / 2;
				assert_ok!(Treasury::create_hold::<Balances>(&owner, MICROGONS_PER_ARGON / 2));
				crate::EncumberedBondMicrogonsByAccount::<Test>::insert(&owner, reserved);
				let _ = crate::AvailableBondCollateralLots::<Test>::clear(u32::MAX, None);
				let _ = crate::ActiveBondsByAccountAndVault::<Test>::clear(u32::MAX, None);
				PendingBondCollateralByAccount::<Test>::insert(
					&owner,
					crate::PendingBondCollateral {
						burn_microgons: reserved - MICROGONS_PER_ARGON / 4,
						release_microgons: MICROGONS_PER_ARGON / 4,
					},
				);
				crate::BondCollateralSettlementQueue::<Test>::insert(
					AscendingRankKey::new(2, owner.clone()),
					(),
				);

				StorageVersion::new(9).put::<Treasury>();
				TreasuryMigration::on_runtime_upgrade();
				if lot_count == 10 {
					assert_eq!(crate::EncumberedBondLotsByAccount::<Test>::get(&owner).len(), 10);
					assert_ok!(Treasury::try_state(1));
				} else {
					assert_eq!(crate::EncumberedBondLotsByAccount::<Test>::get(&owner).len(), 1);
					assert_eq!(
						crate::EncumberedBondLotsByAccount::<Test>::get(&owner)[0]
							.reserved_microgons,
						MICROGONS_PER_ARGON / 2
					);
					assert_ok!(Treasury::try_state(1));
					assert_noop!(
						Treasury::liquidate_bond_lot(origin(2), 0),
						Error::<Test>::BondCollateralSelectionRequired
					);
					Treasury::on_initialize(2);
					assert_eq!(Treasury::encumbered_bond_microgons(&owner), reserved);
					assert!(PendingBondCollateralByAccount::<Test>::contains_key(&owner));
					assert_ok!(Treasury::buy_bonds(origin(2), 1, 12, None));
					let id = crate::NextBondLotId::<Test>::get() - 1;
					assert_ok!(Treasury::encumber_bond_microgons(
						&owner,
						MICROGONS_PER_ARGON / 2,
						Some(&[id])
					));
					assert_ok!(Treasury::try_state(2));
				}
				TreasuryMigration::on_runtime_upgrade();
				assert_eq!(
					Treasury::encumbered_bond_microgons(&owner),
					reserved + if lot_count == 11 { MICROGONS_PER_ARGON / 2 } else { 0 }
				);
				let issuance = Balances::total_issuance();
				for block in 3..=4 {
					Treasury::on_initialize(block);
					assert_ok!(Treasury::try_state(block));
				}
				assert!(!PendingBondCollateralByAccount::<Test>::contains_key(&owner));
				assert_eq!(
					Treasury::encumbered_bond_microgons(&owner),
					if lot_count == 11 { MICROGONS_PER_ARGON / 2 } else { 0 }
				);
				assert_eq!(
					Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &owner),
					if lot_count == 11 { 12 * MICROGONS_PER_ARGON } else { 0 }
				);
				assert_eq!(
					Balances::total_issuance(),
					issuance - reserved + MICROGONS_PER_ARGON / 4
				);
			});
		}
	}
}
