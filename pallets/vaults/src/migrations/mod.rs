use crate::{pallet::VaultsById, Config};
#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
use argon_primitives::{vault::Vault, TickProvider};
use frame_support::{pallet_prelude::*, traits::UncheckedOnRuntimeUpgrade};

pub mod v2 {
	use crate::Config;
	use argon_primitives::{
		tick::Tick,
		vault::{VaultArgons, VaultTerms},
		RewardShare, VaultId,
	};
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		pallet_prelude::{OptionQuery, TypeInfo},
		storage_alias, Twox64Concat,
	};
	use sp_core::RuntimeDebug;
	use sp_runtime::FixedU128;

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct Vault<T: Config> {
		pub operator_account_id: T::AccountId,
		pub bitcoin_argons: VaultArgons<T::Balance>,
		#[codec(compact)]
		pub added_securitization_percent: FixedU128,
		#[codec(compact)]
		pub added_securitization_argons: T::Balance,
		pub bonded_argons: VaultArgons<T::Balance>,
		#[codec(compact)]
		pub mining_reward_sharing_percent_take: RewardShare,
		pub is_closed: bool,
		pub pending_terms: Option<(Tick, VaultTerms<T::Balance>)>,
		pub pending_bonded_argons: Option<(Tick, T::Balance)>,
		pub pending_bitcoins: T::Balance,
	}

	#[storage_alias]
	pub type VaultsById<T: Config> =
		StorageMap<crate::Pallet<T>, Twox64Concat, VaultId, Vault<T>, OptionQuery>;
}

pub struct InnerMigrateV2ToV3<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrateV2ToV3<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let vaults = v2::VaultsById::<T>::iter().collect::<Vec<_>>();
		Ok(vaults.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		VaultsById::<T>::translate::<v2::Vault<T>, _>(|_, v| {
			count += 1;
			Some(Vault {
				operator_account_id: v.operator_account_id,
				bitcoin_argons: v.bitcoin_argons,
				added_securitization_percent: v.added_securitization_percent,
				added_securitization_argons: v.added_securitization_argons,
				bonded_argons: v.bonded_argons,
				mining_reward_sharing_percent_take: v.mining_reward_sharing_percent_take,
				is_closed: v.is_closed,
				pending_terms: v.pending_terms,
				pending_bonded_argons: v.pending_bonded_argons,
				pending_bitcoins: v.pending_bitcoins,
				activation_tick: T::TickProvider::current_tick().saturating_sub(1),
			})
		});

		T::DbWeight::get().reads_writes(count, count)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use argon_primitives::VaultId;
		use frame_support::ensure;

		let actual_new_value = VaultsById::<T>::iter().collect::<Vec<_>>();
		let old_value = Vec::<(VaultId, v2::Vault<T>)>::decode(&mut &state[..])
			.map_err(|_| sp_runtime::TryRuntimeError::Corruption)?;

		ensure!(
			actual_new_value.len() == old_value.len(),
			"New value length does not match old value length"
		);

		Ok(())
	}
}

pub type MigrateV2ToV3<T> = frame_support::migrations::VersionedMigration<
	2, // The migration will only execute when the on-chain storage version is 0
	3, // The on-chain storage version will be set to 1 after the migration is complete
	InnerMigrateV2ToV3<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{new_test_ext, CurrentTick, System, Test};
	use argon_primitives::vault::VaultArgons;
	use frame_support::assert_ok;
	use sp_runtime::FixedU128;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			CurrentTick::set(1000);
			System::set_block_number(1000);
			let bonded_argons = VaultArgons {
				allocated: 10,
				base_fee: 0,
				annual_percent_rate: FixedU128::from(0),
				reserved: 10,
			};
			let bitcoin_argons = VaultArgons {
				allocated: 15,
				base_fee: 0,
				annual_percent_rate: FixedU128::from(1),
				reserved: 15,
			};
			let mining_reward_sharing_percent_take = FixedU128::from_float(0.1);
			let added_securitization_percent = FixedU128::from_float(0.1);
			let is_closed = false;

			v2::VaultsById::<Test>::insert(
				1,
				v2::Vault {
					bonded_argons,
					bitcoin_argons,
					mining_reward_sharing_percent_take,
					operator_account_id: Default::default(),
					pending_terms: None,
					added_securitization_percent,
					is_closed,
					pending_bonded_argons: Some((1010, 10)),
					pending_bitcoins: 0,
					added_securitization_argons: 0,
				},
			);

			// Get the pre_upgrade bytes
			let bytes = match InnerMigrateV2ToV3::<Test>::pre_upgrade() {
				Ok(bytes) => bytes,
				Err(e) => panic!("pre_upgrade failed: {:?}", e),
			};

			// Execute the migration
			let weight = InnerMigrateV2ToV3::<Test>::on_runtime_upgrade();

			// Verify post_upgrade succeeds
			assert_ok!(InnerMigrateV2ToV3::<Test>::post_upgrade(bytes));

			// The weight used should be 1 read for the old value, and 1 write for the new
			// value.
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(1, 1));

			// After the migration, the new value should be set as the `current` value.
			assert_eq!(crate::VaultsById::<Test>::iter_keys().collect::<Vec<_>>().len(), 1);
			assert_eq!(VaultsById::<Test>::get(1).unwrap().activation_tick, CurrentTick::get() - 1);
		})
	}
}
