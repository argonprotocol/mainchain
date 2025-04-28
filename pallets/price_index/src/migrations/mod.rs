use crate::Config;
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

mod old_storage {
	use super::*;
	use crate::Config;
	use argon_primitives::prelude::Tick;
	use frame_support::storage_alias;
	use sp_arithmetic::FixedU128;

	#[derive(
		Encode,
		Decode,
		Eq,
		PartialEq,
		Clone,
		Copy,
		Ord,
		PartialOrd,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct PriceIndex {
		/// Bitcoin to usd price in cents
		#[codec(compact)]
		pub btc_usd_price: FixedU128,
		/// Argon to usd price in cents
		#[codec(compact)]
		pub argon_usd_price: FixedU128,
		/// The target price for argon based on inflation since start
		pub argon_usd_target_price: FixedU128,
		/// Tick of price index
		#[codec(compact)]
		pub tick: Tick,
	}

	#[storage_alias]
	pub(super) type Current<T: Config> = StorageValue<crate::Pallet<T>, PriceIndex>;
}

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		Ok(vec![])
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating price index (removing)");
		old_storage::Current::<T>::take();
		count += 1;

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		Ok(())
	}
}

pub type PriceIndexTwal<T> = frame_support::migrations::VersionedMigration<
	0,
	1,
	InnerMigrate<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::{
		mock::{new_test_ext, Test},
		Current,
	};
	use frame_support::assert_ok;
	use sp_arithmetic::FixedU128;

	#[test]
	fn handles_existing_value() {
		new_test_ext(None).execute_with(|| {
			old_storage::Current::<Test>::put(old_storage::PriceIndex {
				btc_usd_price: FixedU128::from_float(62_000.0),
				argon_usd_price: FixedU128::from_float(1.01),
				argon_usd_target_price: FixedU128::from_float(1.02),
				tick: 1,
			});

			// Get the pre_upgrade bytes
			let bytes = match InnerMigrate::<Test>::pre_upgrade() {
				Ok(bytes) => bytes,
				Err(e) => panic!("pre_upgrade failed: {:?}", e),
			};
			// Execute the migration
			let weight = InnerMigrate::<Test>::on_runtime_upgrade();

			// Verify post_upgrade succeeds
			assert_ok!(InnerMigrate::<Test>::post_upgrade(bytes));

			// The weight used should be 1 read for the old value, and 1 write for the new
			// value.
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(1, 1));

			assert!(Current::<Test>::get().is_none(), "Current should be removed");
		});
	}
}
