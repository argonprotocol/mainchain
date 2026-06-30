use crate::{
	BondLot, BondLotById, BondLotId, BondProgram, BondReleaseReason, Bonds, Config, Pallet,
};
use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;

#[derive(Encode, Decode, DecodeWithMemTracking)]
struct BondLotV5<AccountId, Balance> {
	pub owner: AccountId,
	#[codec(compact)]
	pub vault_id: VaultId,
	#[codec(compact)]
	pub bonds: Bonds,
	#[codec(compact)]
	pub sharing_percent: Permill,
	#[codec(compact)]
	pub bonus_percent: Permill,
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

#[cfg(feature = "try-runtime")]
#[derive(Encode, Decode)]
struct BondProgramMigrationState {
	bond_lot_count: u64,
}

mod v5 {
	use super::*;

	#[storage_alias]
	pub(super) type BondLotById<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		BondLotId,
		BondLotV5<<T as frame_system::Config>::AccountId, <T as crate::Config>::Balance>,
		OptionQuery,
	>;
}

pub struct MigrateBondLotProgram<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for MigrateBondLotProgram<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		Ok(BondProgramMigrationState {
			bond_lot_count: v5::BondLotById::<T>::iter().count() as u64,
		}
		.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let migrated_lots = v5::BondLotById::<T>::iter_keys().count() as u64;
		BondLotById::<T>::translate::<BondLotV5<T::AccountId, T::Balance>, _>(|_, old_bond_lot| {
			Some(BondLot {
				owner: old_bond_lot.owner,
				program: BondProgram::Vault {
					vault_id: old_bond_lot.vault_id,
					sharing_percent: old_bond_lot.sharing_percent,
					bonus_percent: old_bond_lot.bonus_percent,
				},
				bonds: old_bond_lot.bonds,
				created_frame_id: old_bond_lot.created_frame_id,
				participated_frames: old_bond_lot.participated_frames,
				last_frame_earnings_frame_id: old_bond_lot.last_frame_earnings_frame_id,
				last_frame_earnings: old_bond_lot.last_frame_earnings,
				cumulative_earnings: old_bond_lot.cumulative_earnings,
				release_frame_id: old_bond_lot.release_frame_id,
				release_reason: old_bond_lot.release_reason,
			})
		});

		T::DbWeight::get().reads_writes(migrated_lots, migrated_lots)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;

		let state = BondProgramMigrationState::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode treasury bond program state")
		})?;

		ensure!(
			frame_support::traits::StorageVersion::get::<Pallet<T>>() == StorageVersion::new(6),
			"Treasury storage version mismatch after bond program migration",
		);
		ensure!(
			BondLotById::<T>::iter().count() as u64 == state.bond_lot_count,
			"Bond lot count changed during bond program migration",
		);

		for (_, bond_lot) in BondLotById::<T>::iter() {
			ensure!(
				matches!(bond_lot.program, BondProgram::Vault { .. }),
				"Migrated bond lot should be a vault program",
			);
		}

		Ok(())
	}
}

pub type BondLotProgramMigration<T> = frame_support::migrations::VersionedMigration<
	5,
	6,
	MigrateBondLotProgram<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{account_id_from_seed, new_test_ext, Test};
	use frame_support::{assert_ok, traits::OnRuntimeUpgrade};

	#[test]
	fn migrates_v5_bond_lots_to_bond_program() {
		new_test_ext().execute_with(|| {
			frame_support::traits::StorageVersion::new(5).put::<Pallet<Test>>();
			v5::BondLotById::<Test>::insert(
				0,
				BondLotV5 {
					owner: account_id_from_seed(2),
					vault_id: 1,
					bonds: 5,
					sharing_percent: Permill::from_percent(30),
					bonus_percent: Permill::from_percent(10),
					created_frame_id: 1,
					participated_frames: 0,
					last_frame_earnings_frame_id: None,
					last_frame_earnings: None,
					cumulative_earnings: 0,
					release_frame_id: None,
					release_reason: None,
				},
			);

			let bytes = BondLotProgramMigration::<Test>::pre_upgrade().unwrap();
			let _ = BondLotProgramMigration::<Test>::on_runtime_upgrade();
			assert_ok!(BondLotProgramMigration::<Test>::post_upgrade(bytes));

			let bond_lot = BondLotById::<Test>::get(0).expect("bond lot should exist");
			assert_eq!(
				bond_lot.program,
				BondProgram::Vault {
					vault_id: 1,
					sharing_percent: Permill::from_percent(30),
					bonus_percent: Permill::from_percent(10),
				},
			);
		});
	}
}
