use crate::{Config, RevenuePerFrameByVault, VaultFrameRevenue};

use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

mod old_storage {
	use crate::Config;
	use argon_bitcoin::primitives::Satoshis;
	use frame_support::storage_alias;
	use pallet_prelude::*;

	#[storage_alias]
	pub type RevenuePerFrameByVault<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		VaultId,
		BoundedVec<VaultFrameRevenue<T>, ConstU32<12>>,
		ValueQuery,
	>;

	/// Tracks the fee revenue for a Vault for a single Frame (mining day). Includes the associated
	/// amount of bitcoin locked up in Satoshi and Argon values.
	#[derive(
		Encode,
		Decode,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct VaultFrameRevenue<T: Config> {
		/// The frame id in question
		#[codec(compact)]
		pub frame_id: FrameId,
		/// The bitcoin lock fe for the value
		#[codec(compact)]
		pub bitcoin_lock_fee_revenue: T::Balance,
		/// The number of bitcoin locks created
		#[codec(compact)]
		pub bitcoin_locks_created: u32,
		/// The argon market value of the locked satoshis
		#[codec(compact)]
		pub bitcoin_locks_market_value: T::Balance,
		/// The number of satoshis locked into the vault during this period
		#[codec(compact)]
		pub bitcoin_locks_total_satoshis: Satoshis,
		/// The number of satoshis released during this period
		#[codec(compact)]
		pub satoshis_released: Satoshis,
		/// The amount of securitization activated at the end of this frame
		#[codec(compact)]
		pub securitization_activated: T::Balance,
		/// The securitization committed at the end of this frame
		#[codec(compact)]
		pub securitization: T::Balance,
		/// The vault treasury pool profits for this frame
		#[codec(compact)]
		pub treasury_vault_earnings: T::Balance,
		/// The treasury pool aggregate profit
		#[codec(compact)]
		pub treasury_total_earnings: T::Balance,
		/// Vault treasury pool capital capital
		#[codec(compact)]
		pub treasury_vault_capital: T::Balance,
		/// External capital contributed
		#[codec(compact)]
		pub treasury_external_capital: T::Balance,
		/// The amount of revenue still to be collected
		#[codec(compact)]
		pub uncollected_revenue: T::Balance,
	}

	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		#[allow(clippy::type_complexity)]
		pub revenue_per_frame: Vec<(VaultId, BoundedVec<VaultFrameRevenue<T>, ConstU32<12>>)>,
	}
}
pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		let revenue_per_frame =
			old_storage::RevenuePerFrameByVault::<T>::iter().collect::<Vec<_>>();

		Ok(old_storage::Model::<T> { revenue_per_frame }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating vaults");

		RevenuePerFrameByVault::<T>::translate::<
			BoundedVec<old_storage::VaultFrameRevenue<T>, ConstU32<12>>,
			_,
		>(|_id, vec| {
			count += 1;
			let new = vec
				.into_iter()
				.map(|v| VaultFrameRevenue {
					frame_id: v.frame_id,
					bitcoin_lock_fee_revenue: v.bitcoin_lock_fee_revenue,
					bitcoin_locks_created: v.bitcoin_locks_created,
					bitcoin_locks_new_liquidity_promised: v.bitcoin_locks_market_value,
					bitcoin_locks_released_liquidity: T::Balance::zero(),
					bitcoin_locks_added_satoshis: v.bitcoin_locks_total_satoshis,
					bitcoin_locks_released_satoshis: v.satoshis_released,
					securitization_relockable: T::Balance::zero(),
					securitization_activated: v.securitization_activated,
					securitization: v.securitization,
					treasury_vault_earnings: v.treasury_vault_earnings,
					treasury_total_earnings: v.treasury_total_earnings,
					treasury_vault_capital: v.treasury_vault_capital,
					treasury_external_capital: v.treasury_external_capital,
					uncollected_revenue: v.uncollected_revenue,
				})
				.collect::<Vec<_>>();
			Some(BoundedVec::truncate_from(new))
		});

		T::DbWeight::get().reads_writes((count) as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use alloc::collections::BTreeMap;
		use sp_core::Decode;

		let old = <old_storage::Model<T>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		let new_revenue = RevenuePerFrameByVault::<T>::iter().collect::<BTreeMap<_, _>>();
		assert_eq!(
			old.revenue_per_frame.len(),
			new_revenue.len(),
			"Mismatch in number of revenue_per_frame entries"
		);
		for (vault_id, _old_vec) in old.revenue_per_frame {
			assert_eq!(
				new_revenue.get(&vault_id).map(|x| x.len()),
				Some(_old_vec.len()),
				"Vault ID {} missing in new revenue_per_frame storage",
				vault_id
			);
		}

		Ok(())
	}
}

pub type RevenueStatsUpdate<T> = frame_support::migrations::VersionedMigration<
	9,
	10,
	InnerMigrate<T>,
	crate::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::{
		Error,
		mock::{Test, new_test_ext},
	};
	use frame_support::assert_ok;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			old_storage::RevenuePerFrameByVault::<Test>::try_mutate(1u32, |v| {
				v.try_push(old_storage::VaultFrameRevenue {
					frame_id: 42,
					bitcoin_lock_fee_revenue: 100u32.into(),
					bitcoin_locks_created: 2,
					bitcoin_locks_market_value: 500u32.into(),
					bitcoin_locks_total_satoshis: 10_000,
					satoshis_released: 1_000,
					securitization_activated: 200u32.into(),
					securitization: 300u32.into(),
					treasury_vault_earnings: 50u32.into(),
					treasury_total_earnings: 150u32.into(),
					treasury_vault_capital: 1_000u32.into(),
					treasury_external_capital: 2_000u32.into(),
					uncollected_revenue: 25u32.into(),
				})
				.expect("Inserting old storage value should work");
				Ok::<(), Error<Test>>(())
			})
			.expect("Inserting old storage value should work");

			old_storage::RevenuePerFrameByVault::<Test>::try_mutate(2u32, |v| {
				v.try_push(old_storage::VaultFrameRevenue {
					frame_id: 43,
					bitcoin_lock_fee_revenue: 200u32.into(),
					bitcoin_locks_created: 3,
					bitcoin_locks_market_value: 600u32.into(),
					bitcoin_locks_total_satoshis: 20_000,
					satoshis_released: 2_000,
					securitization_activated: 300u32.into(),
					securitization: 400u32.into(),
					treasury_vault_earnings: 60u32.into(),
					treasury_total_earnings: 160u32.into(),
					treasury_vault_capital: 1_500u32.into(),
					treasury_external_capital: 2_500u32.into(),
					uncollected_revenue: 35u32.into(),
				})
				.expect("Inserting old storage value should work");
				Ok::<(), Error<Test>>(())
			})
			.expect("Inserting old storage value should work");

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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(2, 2));

			let new_value_1 = RevenuePerFrameByVault::<Test>::get(1u32);
			assert_eq!(new_value_1.len(), 1);
			let rev_1 = &new_value_1[0];
			assert_eq!(rev_1.frame_id, 42);
			assert_eq!(rev_1.bitcoin_lock_fee_revenue, 100u32.into());
			assert_eq!(rev_1.bitcoin_locks_created, 2);
			assert_eq!(rev_1.bitcoin_locks_new_liquidity_promised, 500u32.into());
			assert_eq!(rev_1.bitcoin_locks_released_liquidity, 0u32.into());
			assert_eq!(rev_1.bitcoin_locks_added_satoshis, 10_000);
			assert_eq!(rev_1.bitcoin_locks_released_satoshis, 1_000);
			assert_eq!(rev_1.securitization_activated, 200u32.into());
			assert_eq!(rev_1.securitization, 300u32.into());
			assert_eq!(rev_1.treasury_vault_earnings, 50u32.into());
			assert_eq!(rev_1.treasury_total_earnings, 150u32.into());
			assert_eq!(rev_1.treasury_vault_capital, 1_000u32.into());
			assert_eq!(rev_1.treasury_external_capital, 2_000u32.into());

			let rev_2 = &RevenuePerFrameByVault::<Test>::get(2u32)[0];
			assert_eq!(rev_2.frame_id, 43);
			assert_eq!(rev_2.bitcoin_lock_fee_revenue, 200u32.into());
			assert_eq!(rev_2.bitcoin_locks_created, 3);
			assert_eq!(rev_2.bitcoin_locks_new_liquidity_promised, 600u32.into());
			assert_eq!(rev_2.bitcoin_locks_released_liquidity, 0u32.into());
			assert_eq!(rev_2.bitcoin_locks_added_satoshis, 20_000);
			assert_eq!(rev_2.bitcoin_locks_released_satoshis, 2_000);
			assert_eq!(rev_2.securitization_activated, 300u32.into());
			assert_eq!(rev_2.securitization, 400u32.into());
			assert_eq!(rev_2.treasury_vault_earnings, 60u32.into());
			assert_eq!(rev_2.treasury_total_earnings, 160u32.into());
			assert_eq!(rev_2.treasury_vault_capital, 1_500u32.into());
			assert_eq!(rev_2.treasury_external_capital, 2_500u32.into());
		});
	}
}
