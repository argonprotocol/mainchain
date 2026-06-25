use crate::{
	BondLot, BondLotAllocation, BondLotById, BondLotId, BondReleaseReason, Bonds, Config,
	CurrentFrameVaultCapital, FrameVaultCapital, Pallet, VaultCapital,
};
use argon_primitives::vault::TreasuryVaultProvider;
use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;

#[derive(Encode, Decode, DecodeWithMemTracking)]
struct BondLotV4<AccountId, Balance> {
	pub owner: AccountId,
	#[codec(compact)]
	pub vault_id: VaultId,
	#[codec(compact)]
	pub bonds: Bonds,
	#[codec(compact)]
	pub created_frame_id: FrameId,
	#[codec(compact)]
	pub participated_frames: u32,
	pub last_frame_earnings_frame_id: Option<FrameId>,
	pub last_frame_earnings: Option<Balance>,
	#[codec(compact)]
	pub cumulative_earnings: Balance,
	pub release_frame_id: Option<FrameId>,
	pub release_reason: Option<BondReleaseReason>,
}

#[derive(Encode, Decode, DecodeWithMemTracking)]
struct VaultCapitalV4<T: Config> {
	pub bond_lot_allocations: BoundedVec<BondLotAllocation, T::MaxTreasuryContributors>,
	#[codec(compact)]
	pub eligible_bonds: Bonds,
	#[codec(compact)]
	pub vault_sharing_percent: Permill,
}

#[derive(Encode, Decode, DecodeWithMemTracking)]
struct FrameVaultCapitalV4<T: Config> {
	#[codec(compact)]
	pub frame_id: FrameId,
	pub vaults: BoundedBTreeMap<VaultId, VaultCapitalV4<T>, T::MaxVaultsPerPool>,
}

#[cfg(feature = "try-runtime")]
#[derive(Encode, Decode)]
struct TreasuryMigrationState {
	bond_lot_count: u64,
	had_frame_capital: bool,
	frame_vault_count: u32,
}

mod v4 {
	use super::*;

	#[storage_alias]
	pub(super) type BondLotById<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		BondLotId,
		BondLotV4<<T as frame_system::Config>::AccountId, <T as crate::Config>::Balance>,
		OptionQuery,
	>;

	#[storage_alias]
	pub(super) type CurrentFrameVaultCapital<T: Config> =
		StorageValue<Pallet<T>, FrameVaultCapitalV4<T>, OptionQuery>;
}

pub struct MigratePerLotBondSharing<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for MigratePerLotBondSharing<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		let frame_vault_count = v4::CurrentFrameVaultCapital::<T>::get()
			.map(|frame| frame.vaults.len() as u32)
			.unwrap_or_default();
		Ok(TreasuryMigrationState {
			bond_lot_count: v4::BondLotById::<T>::iter().count() as u64,
			had_frame_capital: frame_vault_count > 0,
			frame_vault_count,
		}
		.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let migrated_lots = v4::BondLotById::<T>::iter_keys().count() as u64;
		BondLotById::<T>::translate::<BondLotV4<T::AccountId, T::Balance>, _>(|_, old_bond_lot| {
			Some(BondLot {
				owner: old_bond_lot.owner,
				vault_id: old_bond_lot.vault_id,
				bonds: old_bond_lot.bonds,
				sharing_percent: T::TreasuryVaultProvider::get_vault_profit_sharing_percent(
					old_bond_lot.vault_id,
				)
				.unwrap_or_default(),
				bonus_percent: Permill::zero(),
				created_frame_id: old_bond_lot.created_frame_id,
				participated_frames: old_bond_lot.participated_frames,
				last_frame_earnings_frame_id: old_bond_lot.last_frame_earnings_frame_id,
				last_frame_earnings: old_bond_lot.last_frame_earnings,
				cumulative_earnings: old_bond_lot.cumulative_earnings,
				release_frame_id: old_bond_lot.release_frame_id,
				release_reason: old_bond_lot.release_reason,
			})
		});

		let had_frame_capital =
			CurrentFrameVaultCapital::<T>::translate::<FrameVaultCapitalV4<T>, _>(|old_frame| {
				old_frame.map(|old_frame| {
					let mut vaults = BoundedBTreeMap::new();
					for (vault_id, vault_capital) in old_frame.vaults {
						let _ = vaults.try_insert(
							vault_id,
							VaultCapital {
								bond_lot_allocations: vault_capital.bond_lot_allocations,
								eligible_bonds: vault_capital.eligible_bonds,
							},
						);
					}
					FrameVaultCapital { frame_id: old_frame.frame_id, vaults }
				})
			})
			.expect("Current frame vault capital should decode during treasury migration")
			.is_some();

		T::DbWeight::get().reads_writes(
			migrated_lots.saturating_mul(2).saturating_add(1),
			migrated_lots.saturating_add(if had_frame_capital { 1 } else { 0 }),
		)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;

		let state = TreasuryMigrationState::decode(&mut &state[..])
			.map_err(|_| sp_runtime::TryRuntimeError::Other("Failed to decode treasury state"))?;

		ensure!(
			frame_support::traits::StorageVersion::get::<Pallet<T>>() == StorageVersion::new(5),
			"Treasury storage version mismatch after migration",
		);
		ensure!(
			BondLotById::<T>::iter().count() as u64 == state.bond_lot_count,
			"Bond lot count changed during migration",
		);

		for (_, bond_lot) in BondLotById::<T>::iter() {
			ensure!(
				bond_lot.bonus_percent.is_zero(),
				"Migrated bond lot bonus percent must be zero"
			);
			ensure!(
				bond_lot.sharing_percent ==
					T::TreasuryVaultProvider::get_vault_profit_sharing_percent(
						bond_lot.vault_id
					)
					.unwrap_or_default(),
				"Migrated bond lot sharing percent did not copy vault sharing",
			);
		}

		let migrated_frame_capital = CurrentFrameVaultCapital::<T>::get();
		ensure!(
			migrated_frame_capital.is_some() == state.had_frame_capital,
			"Current frame vault capital presence changed during migration",
		);
		if let Some(frame_capital) = migrated_frame_capital {
			ensure!(
				frame_capital.vaults.len() as u32 == state.frame_vault_count,
				"Current frame vault count changed during migration",
			);
		}

		Ok(())
	}
}

pub type BondLotSharingMigration<T> = frame_support::migrations::VersionedMigration<
	4,
	5,
	MigratePerLotBondSharing<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{
		account_id_from_seed, insert_vault, new_test_ext, MaxVaultsPerPool, Test, TestVault,
	};
	use frame_support::{assert_ok, traits::OnRuntimeUpgrade};

	#[test]
	fn migrates_bond_lots_and_frame_capital() {
		new_test_ext().execute_with(|| {
			frame_support::traits::StorageVersion::new(4).put::<Pallet<Test>>();
			insert_vault(
				1,
				TestVault {
					account_id: account_id_from_seed(10),
					securitized_satoshis: 100,
					sharing_percent: Permill::from_percent(30),
					bonus_percent: Permill::from_percent(15),
					delegate_account_id: Some(account_id_from_seed(11)),
					is_closed: false,
				},
			);

			v4::BondLotById::<Test>::insert(
				0,
				BondLotV4 {
					owner: account_id_from_seed(2),
					vault_id: 1,
					bonds: 5,
					created_frame_id: 1,
					participated_frames: 0,
					last_frame_earnings_frame_id: None,
					last_frame_earnings: None,
					cumulative_earnings: 0,
					release_frame_id: None,
					release_reason: None,
				},
			);

			let mut old_vaults =
				BoundedBTreeMap::<VaultId, VaultCapitalV4<Test>, MaxVaultsPerPool>::new();
			assert!(old_vaults
				.try_insert(
					1,
					VaultCapitalV4 {
						bond_lot_allocations: BoundedVec::truncate_from(vec![BondLotAllocation {
							bond_lot_id: 0,
							prorata: FixedU128::one(),
						}]),
						eligible_bonds: 5,
						vault_sharing_percent: Permill::from_percent(70),
					},
				)
				.is_ok());
			v4::CurrentFrameVaultCapital::<Test>::put(FrameVaultCapitalV4 {
				frame_id: 1,
				vaults: old_vaults,
			});

			let bytes = BondLotSharingMigration::<Test>::pre_upgrade().unwrap();
			let _ = BondLotSharingMigration::<Test>::on_runtime_upgrade();
			assert_ok!(BondLotSharingMigration::<Test>::post_upgrade(bytes));

			let bond_lot = BondLotById::<Test>::get(0).expect("bond lot should exist");
			assert_eq!(bond_lot.sharing_percent, Permill::from_percent(30));
			assert_eq!(bond_lot.bonus_percent, Permill::zero());

			let frame_capital =
				CurrentFrameVaultCapital::<Test>::get().expect("frame capital should exist");
			assert_eq!(frame_capital.frame_id, 1);
			assert_eq!(frame_capital.vaults.get(&1).map(|vault| vault.eligible_bonds), Some(5));
		});
	}
}
