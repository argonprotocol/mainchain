use crate::{
	weights::WeightInfo, BondLot, BondLotById, BondLotId, BondLotIdsByVault, BondLotSummary,
	BondLotsByVault, BondProgram, BondReleaseReason, Bonds, Config,
	CurrentFrameArgonotBondParticipants, Pallet, TotalArgonBondLots, VaultBondState,
};
use argon_primitives::{prelude::FrameId, MiningFrameTransitionProvider, VaultId};
use frame_support::{
	migrations::VersionedMigration,
	storage_alias,
	traits::{ConstU32, UncheckedOnRuntimeUpgrade},
};
use pallet_prelude::*;

#[cfg(any(test, feature = "try-runtime"))]
use alloc::vec::Vec;

#[cfg(feature = "try-runtime")]
use crate::CurrentFrameVaultCapital;
use codec::{Decode, Encode};
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

mod old {
	use super::*;

	#[derive(Encode, Decode)]
	pub struct BondLot<T: Config> {
		pub owner: T::AccountId,
		pub program: BondProgram,
		#[codec(compact)]
		pub bonds: Bonds,
		pub is_flexible: bool,
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

	#[storage_alias]
	pub type BondLotById<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, BondLotId, BondLot<T>, OptionQuery>;

	#[derive(Encode, Decode, DefaultNoBound)]
	pub struct VaultBondState {
		pub regular_bond_lots: BoundedVec<BondLotSummary, ConstU32<100>>,
		#[codec(compact)]
		pub flexible_bonds: Bonds,
		#[codec(compact)]
		pub reserved_bond_space: Bonds,
	}

	#[storage_alias]
	pub type BondLotsByVault<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, VaultId, VaultBondState, ValueQuery>;
}

/// Convert vault bond lists to totals and seed the new reward snapshot for the current frame.
pub struct SeedRewardState<T>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for SeedRewardState<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
		let bid_pool_balance = T::Currency::balance(&T::MiningBidPoolAccount::get());
		let reserves_balance = T::Currency::balance(&T::TreasuryReservesAccount::get());
		let issuance = T::Currency::total_issuance();
		let bond_lot_count = old::BondLotById::<T>::iter().count() as u64;
		let vault_count = old::BondLotsByVault::<T>::iter().count() as u64;
		Ok((frame_id, bid_pool_balance, reserves_balance, issuance, bond_lot_count, vault_count)
			.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut reads = 0u64;
		let mut writes = 0u64;
		let mut vault_bond_lot_count = 0u32;
		BondLotById::<T>::translate::<old::BondLot<T>, _>(|bond_lot_id, bond_lot| {
			reads.saturating_accrue(1);
			writes.saturating_accrue(1);
			if let BondProgram::Vault { vault_id, .. } = &bond_lot.program {
				BondLotIdsByVault::<T>::insert(vault_id, bond_lot_id, ());
				vault_bond_lot_count.saturating_accrue(1);
				writes.saturating_accrue(1);
			}
			Some(BondLot {
				owner: bond_lot.owner,
				program: bond_lot.program,
				bonds: bond_lot.bonds,
				is_flexible: bond_lot.is_flexible,
				locked_frame_terms: None,
				created_frame_id: bond_lot.created_frame_id,
				participated_frames: bond_lot.participated_frames,
				last_frame_earnings_frame_id: bond_lot.last_frame_earnings_frame_id,
				last_frame_earnings: bond_lot.last_frame_earnings,
				cumulative_earnings: bond_lot.cumulative_earnings,
				release_frame_id: bond_lot.release_frame_id,
				release_reason: bond_lot.release_reason,
			})
		});
		TotalArgonBondLots::<T>::put(vault_bond_lot_count);
		writes.saturating_accrue(1);
		BondLotsByVault::<T>::translate::<old::VaultBondState, _>(|vault_id, old_vault| {
			reads.saturating_accrue(1);
			writes.saturating_accrue(1);
			let regular_bonds = old_vault
				.regular_bond_lots
				.iter()
				.fold(0u32, |total, lot| total.saturating_add(lot.bonds));
			let flexible_bonds = old_vault.flexible_bonds;
			let capacity =
				Pallet::<T>::balance_to_bonds(Pallet::<T>::get_vault_bond_capacity(vault_id));
			let displaced = flexible_bonds.saturating_sub(capacity.saturating_sub(regular_bonds));
			Some(VaultBondState {
				regular_bonds,
				flexible_bonds,
				displaced_flexible_bonds: displaced,
				locked_frame_terms: None,
				reserved_bond_space: old_vault.reserved_bond_space,
			})
		});
		let frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
		Pallet::<T>::lock_in_vault_capital(frame_id);
		let argonot_participants = CurrentFrameArgonotBondParticipants::<T>::get()
			.map(|participants| participants.bond_lots.len() as u32)
			.unwrap_or_default();
		<T::WeightInfo as WeightInfo>::on_frame_transition(
			TotalArgonBondLots::<T>::get(),
			argonot_participants,
			0,
		)
		.saturating_add(T::DbWeight::get().reads_writes(reads, writes))
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let (frame_id, bid_pool_balance, reserves_balance, issuance, bond_lot_count, vault_count) =
			<(FrameId, T::Balance, T::Balance, T::Balance, u64, u64)>::decode(&mut state.as_slice())
				.map_err(|_| TryRuntimeError::Other("invalid treasury reward migration state"))?;
		ensure!(
			T::MiningFrameTransitionProvider::get_current_frame_id() == frame_id,
			TryRuntimeError::Other("migration rotated the mining frame"),
		);
		ensure!(
			CurrentFrameVaultCapital::<T>::get()
				.is_some_and(|capital| capital.frame_id == frame_id),
			TryRuntimeError::Other("current frame reward snapshot was not seeded"),
		);
		ensure!(
			T::Currency::balance(&T::MiningBidPoolAccount::get()) == bid_pool_balance &&
				T::Currency::balance(&T::TreasuryReservesAccount::get()) == reserves_balance &&
				T::Currency::total_issuance() == issuance,
			TryRuntimeError::Other("migration distributed or burned frame earnings"),
		);
		ensure!(
			BondLotById::<T>::iter().count() as u64 == bond_lot_count &&
				BondLotsByVault::<T>::iter().count() as u64 == vault_count,
			TryRuntimeError::Other("bond positions changed during migration")
		);
		let expected_vault_lots = BondLotById::<T>::iter()
			.filter(|(_, lot)| matches!(lot.program, BondProgram::Vault { .. }))
			.count() as u32;
		ensure!(
			TotalArgonBondLots::<T>::get() == expected_vault_lots &&
				BondLotIdsByVault::<T>::iter().count() as u32 == expected_vault_lots &&
				BondLotById::<T>::iter().all(|(bond_lot_id, lot)| match lot.program {
					BondProgram::Vault { vault_id, .. } =>
						BondLotIdsByVault::<T>::contains_key(vault_id, bond_lot_id),
					BondProgram::Argonot => true,
				}),
			TryRuntimeError::Other("vault bond id index mismatch"),
		);
		ensure!(
			BondLotById::<T>::iter().all(|(_, lot)| lot.locked_frame_terms.is_none()),
			TryRuntimeError::Other("bond payout terms unexpectedly locked during migration")
		);
		let expected_total = BondLotsByVault::<T>::iter().fold(0u128, |total, (_, vault_bonds)| {
			total
				.saturating_add(vault_bonds.regular_bonds as u128)
				.saturating_add(vault_bonds.flexible_bonds as u128)
				.saturating_sub(vault_bonds.displaced_flexible_bonds as u128)
		});
		ensure!(
			CurrentFrameVaultCapital::<T>::get()
				.is_some_and(|capital| capital.total_active_bonds == expected_total),
			TryRuntimeError::Other("active bond total mismatch"),
		);
		Ok(())
	}
}

pub type SeedRewardStateMigration<T> =
	VersionedMigration<8, 9, SeedRewardState<T>, Pallet<T>, <T as frame_system::Config>::DbWeight>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		mock::{
			account_id_from_seed, insert_vault, new_test_ext, set_argons, Balances,
			BidPoolAccountId, CurrentFrameId, LastVaultProfits, MinimumArgonsPerContributor,
			RuntimeEvent, RuntimeOrigin, System, Test, TestVault, Treasury, VaultsById,
		},
		BondLotById, BondLotIdsByAccount, BondLotsByVault, CurrentFrameVaultCapital,
	};
	use alloc::collections::BTreeMap;
	use argon_primitives::MICROGONS_PER_ARGON;
	use frame_support::{
		assert_ok,
		traits::{fungible::Mutate, OnRuntimeUpgrade, StorageVersion},
	};

	fn write_previous_storage_layout() {
		let lots = BondLotById::<Test>::iter().collect::<Vec<_>>();
		for (vault_id, bond_lot_id, ()) in BondLotIdsByVault::<Test>::iter().collect::<Vec<_>>() {
			BondLotIdsByVault::<Test>::remove(vault_id, bond_lot_id);
		}
		TotalArgonBondLots::<Test>::kill();
		let mut regular_lots_by_vault = BTreeMap::<VaultId, Vec<BondLotSummary>>::new();
		for (bond_lot_id, lot) in &lots {
			if lot.release_reason.is_none() && !lot.is_flexible {
				if let BondProgram::Vault { vault_id, .. } = lot.program {
					regular_lots_by_vault
						.entry(vault_id)
						.or_default()
						.push(BondLotSummary { bond_lot_id: *bond_lot_id, bonds: lot.bonds });
				}
			}
		}
		for (bond_lot_id, lot) in lots {
			old::BondLotById::<Test>::insert(
				bond_lot_id,
				old::BondLot {
					owner: lot.owner,
					program: lot.program,
					bonds: lot.bonds,
					is_flexible: lot.is_flexible,
					created_frame_id: lot.created_frame_id,
					participated_frames: lot.participated_frames,
					last_frame_earnings_frame_id: lot.last_frame_earnings_frame_id,
					last_frame_earnings: lot.last_frame_earnings,
					cumulative_earnings: lot.cumulative_earnings,
					release_frame_id: lot.release_frame_id,
					release_reason: lot.release_reason,
				},
			);
		}
		for (vault_id, state) in BondLotsByVault::<Test>::iter().collect::<Vec<_>>() {
			let regular_bond_lots = regular_lots_by_vault.remove(&vault_id).unwrap_or_default();
			old::BondLotsByVault::<Test>::insert(
				vault_id,
				old::VaultBondState {
					regular_bond_lots: regular_bond_lots.try_into().expect("old lot limit"),
					flexible_bonds: state.flexible_bonds,
					reserved_bond_space: state.reserved_bond_space,
				},
			);
		}
	}

	#[test]
	fn seeds_current_frame_without_paying_or_rotating_it() {
		new_test_ext().execute_with(|| {
			CurrentFrameId::set(20);
			MinimumArgonsPerContributor::set(1);
			StorageVersion::new(8).put::<Pallet<Test>>();
			let operator = account_id_from_seed(1);
			insert_vault(
				1,
				TestVault {
					securitization: 10 * MICROGONS_PER_ARGON,
					exit_notice_amount: 0,
					committed_microgons: 0,
					activated_securitization: 0,
					account_id: operator.clone(),
					delegate_account_id: None,
					is_closed: false,
				},
			);
			set_argons(&operator, 10 * MICROGONS_PER_ARGON);
			assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(operator.clone()), 1, 10, None));
			let lot_id = BondLotIdsByAccount::<Test>::iter_key_prefix(&operator)
				.next()
				.expect("bond lot");
			assert_ok!(Treasury::set_bond_lot_flexible(
				RuntimeOrigin::signed(operator),
				lot_id,
				true
			));
			write_previous_storage_layout();
			let bid_pool_account = BidPoolAccountId::get();
			let bid_pool_amount = 100 * MICROGONS_PER_ARGON;
			assert_ok!(Balances::mint_into(&bid_pool_account, bid_pool_amount));
			#[cfg(feature = "try-runtime")]
			let upgrade_state = SeedRewardState::<Test>::pre_upgrade().unwrap();

			SeedRewardStateMigration::<Test>::on_runtime_upgrade();
			#[cfg(feature = "try-runtime")]
			SeedRewardState::<Test>::post_upgrade(upgrade_state).unwrap();

			assert_eq!(BondLotsByVault::<Test>::get(1).flexible_bonds, 10);
			assert!(BondLotById::<Test>::get(lot_id).unwrap().is_flexible);
			assert_eq!(CurrentFrameVaultCapital::<Test>::get().unwrap().frame_id, 20);
			assert_eq!(
				VaultsById::get().get(&1).map(|vault| vault.committed_microgons),
				Some(10 * MICROGONS_PER_ARGON),
			);
			assert_eq!(CurrentFrameId::get(), 20);
			assert_eq!(Balances::free_balance(&bid_pool_account), bid_pool_amount);
			assert_eq!(BondLotById::<Test>::get(lot_id).unwrap().cumulative_earnings, 0);
			assert!(LastVaultProfits::get().is_empty());
			assert!(!System::events().iter().any(|record| matches!(
				record.event,
				RuntimeEvent::Treasury(crate::Event::FrameEarningsDistributed { .. })
			)));
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), StorageVersion::new(9));
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn migration_keeps_existing_flexible_lots() {
		new_test_ext().execute_with(|| {
			CurrentFrameId::set(20);
			StorageVersion::new(8).put::<Pallet<Test>>();
			MinimumArgonsPerContributor::set(1);
			let operator = account_id_from_seed(1);
			insert_vault(
				1,
				TestVault {
					securitization: 2 * MICROGONS_PER_ARGON,
					exit_notice_amount: 0,
					committed_microgons: 0,
					activated_securitization: 0,
					account_id: operator.clone(),
					delegate_account_id: None,
					is_closed: false,
				},
			);
			set_argons(&operator, 2 * MICROGONS_PER_ARGON);
			for _ in 0..2 {
				assert_ok!(Treasury::buy_bonds(
					RuntimeOrigin::signed(operator.clone()),
					1,
					1,
					None
				));
			}
			insert_vault(
				1,
				TestVault {
					securitization: MICROGONS_PER_ARGON,
					exit_notice_amount: 0,
					committed_microgons: 0,
					activated_securitization: 0,
					account_id: operator.clone(),
					delegate_account_id: None,
					is_closed: false,
				},
			);
			let lot_ids =
				BondLotIdsByAccount::<Test>::iter_key_prefix(&operator).collect::<Vec<_>>();
			for lot_id in &lot_ids {
				BondLotById::<Test>::mutate(lot_id, |lot| {
					lot.as_mut().unwrap().is_flexible = true;
				});
			}
			BondLotsByVault::<Test>::mutate(1, |bonds| {
				bonds.regular_bonds = 0;
				bonds.flexible_bonds = 2;
			});
			write_previous_storage_layout();
			let upgrade_state = SeedRewardState::<Test>::pre_upgrade().unwrap();
			SeedRewardStateMigration::<Test>::on_runtime_upgrade();
			SeedRewardState::<Test>::post_upgrade(upgrade_state).unwrap();

			assert_eq!(BondLotsByVault::<Test>::get(1).regular_bonds, 0);
			assert_eq!(BondLotsByVault::<Test>::get(1).flexible_bonds, 2);
			assert_eq!(BondLotsByVault::<Test>::get(1).displaced_flexible_bonds, 1);
			assert_eq!(CurrentFrameVaultCapital::<Test>::get().unwrap().total_active_bonds, 1);
			assert_eq!(
				lot_ids
					.iter()
					.filter(|lot_id| BondLotById::<Test>::get(lot_id).unwrap().is_flexible)
					.count(),
				2
			);
		});
	}
}
