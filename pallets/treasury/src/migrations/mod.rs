use crate::{
	BondLot, BondLotAllocation, BondLotById, BondLotId, BondLotSummary, BondLotsByVault,
	BondProgram, BondReleaseReason, Bonds, Config, CurrentFrameVaultCapital, FrameVaultCapital,
	Pallet, VaultBondState, VaultCapital,
};
use codec::{Decode, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use frame_support::{ensure, traits::StorageVersion};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[derive(Encode, Decode)]
struct BondLotV6<T: Config> {
	owner: T::AccountId,
	program: BondProgram,
	#[codec(compact)]
	bonds: Bonds,
	#[codec(compact)]
	created_frame_id: FrameId,
	#[codec(compact)]
	participated_frames: u32,
	last_frame_earnings_frame_id: Option<FrameId>,
	last_frame_earnings: Option<T::Balance>,
	#[codec(compact)]
	cumulative_earnings: T::Balance,
	release_frame_id: Option<FrameId>,
	release_reason: Option<BondReleaseReason>,
}

#[derive(Encode, Decode)]
struct VaultCapitalV6<T: Config> {
	bond_lot_allocations: BoundedVec<BondLotAllocation, T::MaxTreasuryContributors>,
	#[codec(compact)]
	eligible_bonds: Bonds,
}

#[derive(Encode, Decode)]
struct FrameVaultCapitalV6<T: Config> {
	#[codec(compact)]
	frame_id: FrameId,
	vaults: BoundedBTreeMap<VaultId, VaultCapitalV6<T>, T::MaxVaultsPerPool>,
}

mod v6 {
	use super::*;

	#[storage_alias]
	pub(super) type BondLotById<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, BondLotId, BondLotV6<T>, OptionQuery>;

	#[storage_alias]
	pub(super) type BondLotsByVault<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		VaultId,
		BoundedVec<BondLotSummary, <T as crate::Config>::MaxTreasuryContributors>,
		ValueQuery,
	>;

	#[storage_alias]
	pub(super) type CurrentFrameVaultCapital<T: Config> =
		StorageValue<Pallet<T>, FrameVaultCapitalV6<T>, OptionQuery>;
}

pub struct AddBondBackfillState<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for AddBondBackfillState<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut lot_count = 0u64;
		BondLotById::<T>::translate::<BondLotV6<T>, _>(|_, lot| {
			lot_count = lot_count.saturating_add(1);
			Some(BondLot {
				owner: lot.owner,
				program: lot.program,
				bonds: lot.bonds,
				is_backfill: false,
				created_frame_id: lot.created_frame_id,
				participated_frames: lot.participated_frames,
				last_frame_earnings_frame_id: lot.last_frame_earnings_frame_id,
				last_frame_earnings: lot.last_frame_earnings,
				cumulative_earnings: lot.cumulative_earnings,
				release_frame_id: lot.release_frame_id,
				release_reason: lot.release_reason,
			})
		});

		let mut vault_count = 0u64;
		BondLotsByVault::<T>::translate::<BoundedVec<BondLotSummary, T::MaxTreasuryContributors>, _>(
			|_, bond_lots| {
				vault_count = vault_count.saturating_add(1);
				Some(VaultBondState { bond_lots, backfill_bonds: 0, backfill_bonds_reserved: 0 })
			},
		);

		CurrentFrameVaultCapital::<T>::translate::<FrameVaultCapitalV6<T>, _>(|old_frame| {
			let old_frame = old_frame?;
			let mut vaults = BoundedBTreeMap::new();
			for (vault_id, old_capital) in old_frame.vaults {
				vaults
					.try_insert(
						vault_id,
						VaultCapital {
							bond_lot_allocations: old_capital.bond_lot_allocations,
							backfill_bonds_eligible: 0,
							backfill_prorata: FixedU128::zero(),
							eligible_bonds: old_capital.eligible_bonds,
						},
					)
					.expect("source and destination use the same MaxVaultsPerPool bound");
			}
			Some(FrameVaultCapital { frame_id: old_frame.frame_id, vaults })
		})
		.expect("current frame vault capital must decode");

		T::DbWeight::get().reads_writes(
			lot_count.saturating_add(vault_count).saturating_add(1),
			lot_count.saturating_add(vault_count).saturating_add(1),
		)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		ensure!(
			StorageVersion::get::<Pallet<T>>() == 6,
			TryRuntimeError::Other("treasury storage version must be 6 before migration"),
		);

		let lot_count = v6::BondLotById::<T>::iter().fold(0u64, |count, _| count.saturating_add(1));
		let vault_count =
			v6::BondLotsByVault::<T>::iter().fold(0u64, |count, _| count.saturating_add(1));
		let frame = v6::CurrentFrameVaultCapital::<T>::get().map(|frame| {
			let vault_count = frame.vaults.iter().fold(0u64, |count, _| count.saturating_add(1));
			(frame.frame_id, vault_count)
		});

		Ok((lot_count, vault_count, frame).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		ensure!(
			StorageVersion::get::<Pallet<T>>() == 7,
			TryRuntimeError::Other("treasury storage version was not updated"),
		);

		let (expected_lot_count, expected_vault_count, expected_frame) =
			<(u64, u64, Option<(FrameId, u64)>)>::decode(&mut state.as_slice())
				.map_err(|_| TryRuntimeError::Other("could not decode treasury migration state"))?;

		let mut migrated_lot_count = 0u64;
		for (_, lot) in BondLotById::<T>::iter() {
			migrated_lot_count = migrated_lot_count.saturating_add(1);
			ensure!(
				!lot.is_backfill,
				TryRuntimeError::Other("migrated bond lot was marked as backfill"),
			);
		}
		ensure!(
			migrated_lot_count == expected_lot_count,
			TryRuntimeError::Other("bond lot count changed during migration"),
		);

		let mut migrated_vault_count = 0u64;
		for (_, vault) in BondLotsByVault::<T>::iter() {
			migrated_vault_count = migrated_vault_count.saturating_add(1);
			ensure!(
				vault.backfill_bonds == 0 && vault.backfill_bonds_reserved == 0,
				TryRuntimeError::Other("migrated vault has nonzero backfill bond state"),
			);
		}
		ensure!(
			migrated_vault_count == expected_vault_count,
			TryRuntimeError::Other("vault bond state count changed during migration"),
		);

		match (expected_frame, CurrentFrameVaultCapital::<T>::get()) {
			(None, None) => {},
			(Some((expected_frame_id, expected_frame_vault_count)), Some(frame)) => {
				ensure!(
					frame.frame_id == expected_frame_id,
					TryRuntimeError::Other("current treasury frame changed during migration"),
				);
				let mut migrated_frame_vault_count = 0u64;
				for (_, capital) in frame.vaults {
					migrated_frame_vault_count = migrated_frame_vault_count.saturating_add(1);
					ensure!(
						capital.backfill_bonds_eligible == 0 && capital.backfill_prorata.is_zero(),
						TryRuntimeError::Other("migrated frame vault has nonzero backfill state",),
					);
				}
				ensure!(
					migrated_frame_vault_count == expected_frame_vault_count,
					TryRuntimeError::Other("current frame vault count changed during migration",),
				);
			},
			_ =>
				return Err(TryRuntimeError::Other(
					"current treasury frame presence changed during migration",
				)),
		}

		Ok(())
	}
}

pub type AddBondBackfillStateMigration<T> = frame_support::migrations::VersionedMigration<
	6,
	7,
	AddBondBackfillState<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{account_id_from_seed, new_test_ext, Test};
	use frame_support::traits::OnRuntimeUpgrade;
	use sp_runtime::bounded_vec;

	#[test]
	fn preserves_existing_bond_state_and_adds_empty_backfill_state() {
		new_test_ext().execute_with(|| {
			let owner = account_id_from_seed(1);
			let program = BondProgram::Vault {
				vault_id: 7,
				sharing_percent: Permill::from_percent(20),
				bonus_percent: Permill::from_percent(5),
			};
			v6::BondLotById::<Test>::insert(
				1,
				BondLotV6 {
					owner: owner.clone(),
					program,
					bonds: 50,
					created_frame_id: 2,
					participated_frames: 3,
					last_frame_earnings_frame_id: Some(4),
					last_frame_earnings: Some(5),
					cumulative_earnings: 6,
					release_frame_id: Some(7),
					release_reason: Some(BondReleaseReason::UserLiquidation),
				},
			);

			let bond_lots: BoundedVec<_, <Test as Config>::MaxTreasuryContributors> =
				bounded_vec![BondLotSummary { bond_lot_id: 1, bonds: 50 }];
			v6::BondLotsByVault::<Test>::insert(7, bond_lots.clone());

			let old_allocations: BoundedVec<_, <Test as Config>::MaxTreasuryContributors> =
				bounded_vec![BondLotAllocation {
					bond_lot_id: 1,
					prorata: FixedU128::from_rational(1, 2),
				}];
			let mut vaults = BoundedBTreeMap::new();
			assert!(vaults
				.try_insert(
					7,
					VaultCapitalV6::<Test> {
						bond_lot_allocations: old_allocations,
						eligible_bonds: 50,
					},
				)
				.is_ok());
			v6::CurrentFrameVaultCapital::<Test>::put(FrameVaultCapitalV6 { frame_id: 8, vaults });
			StorageVersion::new(6).put::<Pallet<Test>>();

			let state = AddBondBackfillStateMigration::<Test>::pre_upgrade().unwrap();
			AddBondBackfillStateMigration::<Test>::on_runtime_upgrade();
			AddBondBackfillStateMigration::<Test>::post_upgrade(state).unwrap();

			assert_eq!(
				BondLotById::<Test>::get(1),
				Some(BondLot {
					owner,
					program,
					bonds: 50,
					is_backfill: false,
					created_frame_id: 2,
					participated_frames: 3,
					last_frame_earnings_frame_id: Some(4),
					last_frame_earnings: Some(5),
					cumulative_earnings: 6,
					release_frame_id: Some(7),
					release_reason: Some(BondReleaseReason::UserLiquidation),
				}),
			);
			assert_eq!(
				BondLotsByVault::<Test>::get(7),
				VaultBondState { bond_lots, backfill_bonds: 0, backfill_bonds_reserved: 0 },
			);
			let allocations: BoundedVec<_, <Test as Config>::MaxTreasuryContributors> =
				bounded_vec![BondLotAllocation {
					bond_lot_id: 1,
					prorata: FixedU128::from_rational(1, 2),
				}];
			let mut expected_vaults = BoundedBTreeMap::new();
			assert!(expected_vaults
				.try_insert(
					7,
					VaultCapital {
						bond_lot_allocations: allocations,
						backfill_bonds_eligible: 0,
						backfill_prorata: FixedU128::zero(),
						eligible_bonds: 50,
					},
				)
				.is_ok());
			assert_eq!(
				CurrentFrameVaultCapital::<Test>::get(),
				Some(FrameVaultCapital { frame_id: 8, vaults: expected_vaults }),
			);
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 7);
		});
	}
}
