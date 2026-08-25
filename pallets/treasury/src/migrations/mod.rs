use codec::{Decode, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

use crate::{BondLotAllocation, Bonds, Config, FrameVaultCapital, Pallet, VaultCapital};

mod v7 {
	use super::*;

	#[derive(Encode, Decode)]
	pub struct VaultCapital<T: Config> {
		pub regular_bond_allocations: BoundedVec<BondLotAllocation, T::MaxTreasuryContributors>,
		#[codec(compact)]
		pub flexible_bonds_eligible: Bonds,
		pub flexible_prorata: FixedU128,
		#[codec(compact)]
		pub eligible_bonds: Bonds,
	}

	#[derive(Encode, Decode)]
	pub struct FrameVaultCapital<T: Config> {
		#[codec(compact)]
		pub frame_id: FrameId,
		pub vaults: BoundedBTreeMap<VaultId, VaultCapital<T>, T::MaxVaultsPerPool>,
	}

	#[storage_alias]
	pub type CurrentFrameVaultCapital<T: Config> =
		StorageValue<Pallet<T>, FrameVaultCapital<T>, OptionQuery>;
}

pub struct AddVaultBondEarningsEligibility<T>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for AddVaultBondEarningsEligibility<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		Ok(v7::CurrentFrameVaultCapital::<T>::get()
			.map(|frame| (frame.frame_id, frame.vaults.len() as u32))
			.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let Some(frame) = v7::CurrentFrameVaultCapital::<T>::get() else {
			return T::DbWeight::get().reads(1);
		};

		let mut vaults = BoundedBTreeMap::new();
		for (vault_id, vault) in frame.vaults {
			let _ = vaults.try_insert(
				vault_id,
				VaultCapital {
					regular_bond_allocations: vault.regular_bond_allocations,
					flexible_bonds_eligible: vault.flexible_bonds_eligible,
					flexible_prorata: vault.flexible_prorata,
					eligible_bonds: vault.eligible_bonds,
					argonot_securitization: T::Balance::zero(),
					argonots_for_max_earnings: T::Balance::zero(),
					bond_earnings_eligibility: FixedU128::one(),
				},
			);
		}

		crate::CurrentFrameVaultCapital::<T>::put(FrameVaultCapital {
			frame_id: frame.frame_id,
			vaults,
		});
		T::DbWeight::get().reads_writes(1, 1)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let previous_frame = Option::<(FrameId, u32)>::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("could not decode treasury migration state"))?;
		let migrated_frame = crate::CurrentFrameVaultCapital::<T>::get();

		match (previous_frame, migrated_frame) {
			(None, None) => {},
			(Some((frame_id, vault_count)), Some(frame)) => {
				ensure!(
					frame.frame_id == frame_id && frame.vaults.len() as u32 == vault_count,
					TryRuntimeError::Other("treasury frame capital was not preserved"),
				);
				ensure!(
					frame
						.vaults
						.values()
						.all(|vault| vault.bond_earnings_eligibility == FixedU128::one()),
					TryRuntimeError::Other("legacy vault earnings did not retain full eligibility"),
				);
			},
			_ => return Err(TryRuntimeError::Other("treasury frame migration changed presence")),
		}

		Ok(())
	}
}

pub type AddVaultBondEarningsEligibilityMigration<T> =
	frame_support::migrations::VersionedMigration<
		7,
		8,
		AddVaultBondEarningsEligibility<T>,
		Pallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;

#[cfg(all(feature = "try-runtime", test))]
mod tests {
	use super::*;
	use crate::mock::{new_test_ext, MaxTreasuryContributors, MaxVaultsPerPool, Test};
	use frame_support::traits::OnRuntimeUpgrade;

	#[test]
	fn preserves_an_in_flight_frame_with_full_legacy_eligibility() {
		new_test_ext().execute_with(|| {
			let mut vaults =
				BoundedBTreeMap::<VaultId, v7::VaultCapital<Test>, MaxVaultsPerPool>::new();
			assert!(vaults
				.try_insert(
					1,
					v7::VaultCapital {
						regular_bond_allocations: BoundedVec::<
							BondLotAllocation,
							MaxTreasuryContributors,
						>::default(),
						flexible_bonds_eligible: 2,
						flexible_prorata: FixedU128::from_rational(1, 2),
						eligible_bonds: 4,
					},
				)
				.is_ok());
			v7::CurrentFrameVaultCapital::<Test>::put(v7::FrameVaultCapital {
				frame_id: 9,
				vaults,
			});
			StorageVersion::new(7).put::<Pallet<Test>>();

			AddVaultBondEarningsEligibilityMigration::<Test>::try_on_runtime_upgrade(true)
				.expect("runtime upgrade checks");

			let frame = crate::CurrentFrameVaultCapital::<Test>::get().expect("migrated frame");
			let vault = frame.vaults.get(&1).expect("migrated vault");
			assert_eq!(frame.frame_id, 9);
			assert_eq!(vault.flexible_bonds_eligible, 2);
			assert_eq!(vault.flexible_prorata, FixedU128::from_rational(1, 2));
			assert_eq!(vault.eligible_bonds, 4);
			assert_eq!(vault.bond_earnings_eligibility, FixedU128::one());
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 8);
		});
	}
}
