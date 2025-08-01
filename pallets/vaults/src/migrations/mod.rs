use crate::{Config, RevenuePerFrameByVault, VaultFrameRevenue, pallet::VaultsById};
use alloc::collections::BTreeMap;
use argon_primitives::prelude::{sp_arithmetic::Permill, sp_runtime::Saturating};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_liquidity_pools::VaultPoolsByFrame;
use pallet_prelude::*;

mod old_storage {
	use crate::{Config, Pallet};
	use argon_bitcoin::primitives::Satoshis;
	use frame_support_procedural::storage_alias;
	use pallet_prelude::*;

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
	pub struct VaultFrameFeeRevenue<T: Config> {
		/// The frame id in question
		#[codec(compact)]
		pub frame_id: FrameId,
		/// The fee revenue for the value
		#[codec(compact)]
		pub fee_revenue: T::Balance,
		/// The number of bitcoin locks created
		#[codec(compact)]
		pub bitcoin_locks_created: u32,
		/// The argon market value of the locked satoshis
		#[codec(compact)]
		pub bitcoin_locks_market_value: T::Balance,
		/// The number of satoshis locked into the vault
		#[codec(compact)]
		pub bitcoin_locks_total_satoshis: Satoshis,
		/// The number of satoshis released during this period
		#[codec(compact)]
		pub satoshis_released: Satoshis,
	}

	#[storage_alias]
	pub type PerFrameFeeRevenueByVault<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		VaultId,
		BoundedVec<VaultFrameFeeRevenue<T>, ConstU32<10>>,
		ValueQuery,
	>;

	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub frame_revenue: Vec<(VaultId, BoundedVec<VaultFrameFeeRevenue<T>, ConstU32<10>>)>,
	}
}
pub struct InnerMigrate<T: crate::Config + pallet_liquidity_pools::Config>(
	core::marker::PhantomData<T>,
);

impl<T: Config + pallet_liquidity_pools::Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let frame_revenue = old_storage::PerFrameFeeRevenueByVault::<T>::iter().collect::<Vec<_>>();

		Ok(old_storage::Model::<T> { frame_revenue }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating vaults");
		let frame_revenue =
			old_storage::PerFrameFeeRevenueByVault::<T>::drain().collect::<Vec<_>>();
		let liquidity_pools = VaultPoolsByFrame::<T>::iter().collect::<BTreeMap<_, _>>();

		let mut extra_reads = 0;
		for (vault_id, revenue) in frame_revenue {
			let vault = VaultsById::<T>::get(vault_id)
				.expect("Vault should exist in the new storage; migration is not complete");
			extra_reads += 1;
			count += 2;
			let vault_revenue = revenue
				.into_inner()
				.into_iter()
				.map(|v| {
					let mut liquidity_pool_vault_earnings = 0u128;
					let mut liquidity_pool_total_earnings = 0u128;
					let mut liquidity_pool_vault_capital = 0u128;
					let mut liquidity_pool_external_capital = 0u128;

					if let Some(earnings) =
						liquidity_pools.get(&v.frame_id).and_then(|lp| lp.get(&vault_id))
					{
						liquidity_pool_total_earnings =
							earnings.distributed_profits.unwrap_or_default().into();
						liquidity_pool_vault_earnings = Permill::one()
							.saturating_sub(earnings.vault_sharing_percent)
							.mul_floor(liquidity_pool_total_earnings);
						let external_profit =
							liquidity_pool_total_earnings - liquidity_pool_vault_earnings;
						for (who, amount) in &earnings.contributor_balances {
							if *who == vault.operator_account_id {
								liquidity_pool_vault_capital = (*amount).into();
							} else {
								liquidity_pool_external_capital += (*amount).into();
							}
						}
						liquidity_pool_external_capital.saturating_reduce(external_profit);
					}
					VaultFrameRevenue {
						frame_id: v.frame_id,
						bitcoin_lock_fee_revenue: v.fee_revenue,
						bitcoin_locks_created: v.bitcoin_locks_created,
						bitcoin_locks_market_value: v.bitcoin_locks_market_value,
						bitcoin_locks_total_satoshis: v.bitcoin_locks_total_satoshis,
						satoshis_released: v.satoshis_released,
						uncollected_revenue: 0u128.into(),
						securitization_activated: vault.get_activated_securitization(),
						securitization: vault.securitization,
						liquidity_pool_vault_earnings: liquidity_pool_vault_earnings.into(),
						liquidity_pool_total_earnings: liquidity_pool_total_earnings.into(),
						liquidity_pool_vault_capital: liquidity_pool_vault_capital.into(),
						liquidity_pool_external_capital: liquidity_pool_external_capital.into(),
					}
				})
				.collect::<Vec<_>>();
			RevenuePerFrameByVault::<T>::insert(vault_id, BoundedVec::truncate_from(vault_revenue));
		}

		T::DbWeight::get().reads_writes((count + extra_reads) as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use sp_core::Decode;

		let old = <old_storage::Model<T>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;
		let old_revenue = old.frame_revenue;
		let new_revenue = RevenuePerFrameByVault::<T>::iter().collect::<BTreeMap<_, _>>();
		for (vault_id, entries) in old_revenue {
			let new_entries = new_revenue
				.get(&vault_id)
				.expect("Vault should exist in the new storage; migration is not complete");
			assert_eq!(new_entries.len(), entries.len());
		}

		Ok(())
	}
}

pub type PoolEarningsMigration<T> = frame_support::migrations::VersionedMigration<
	6,
	7,
	InnerMigrate<T>,
	crate::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::mock::{Test, new_test_ext};
	use frame_support::assert_ok;
	use pallet_liquidity_pools::{LiquidityPool, VaultPoolsByFrame};
	use pallet_prelude::argon_primitives::vault::{Vault, VaultTerms};

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			VaultsById::<Test>::insert(
				1,
				Vault {
					operator_account_id: 1,
					securitization: 100,
					argons_scheduled_for_release: Default::default(),
					argons_locked: 100,
					argons_pending_activation: 0,
					securitization_ratio: FixedU128::one(),
					is_closed: false,
					terms: VaultTerms {
						bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
						bitcoin_base_fee: 0,
						liquidity_pool_profit_sharing: Permill::from_percent(10),
					},
					pending_terms: None,
					opened_tick: 0,
				},
			);
			VaultsById::<Test>::insert(
				2,
				Vault {
					operator_account_id: 2,
					securitization: 200,
					argons_scheduled_for_release: Default::default(),
					argons_locked: 100,
					argons_pending_activation: 0,
					securitization_ratio: FixedU128::one(),
					is_closed: false,
					terms: VaultTerms {
						bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
						bitcoin_base_fee: 0,
						liquidity_pool_profit_sharing: Permill::from_percent(10),
					},
					pending_terms: None,
					opened_tick: 0,
				},
			);
			for i in 144..154 {
				VaultPoolsByFrame::<Test>::mutate(i, |x| {
					let _ = x.try_insert(
						1,
						LiquidityPool {
							contributor_balances: BoundedVec::truncate_from(vec![
								(1u64, 190_000),
								(2u64, 10_500),
							]),
							distributed_profits: Some(100_000),
							vault_sharing_percent: Permill::from_percent(10),
							do_not_renew: BoundedVec::default(),
							is_rolled_over: false,
						},
					);
					let _ = x.try_insert(
						2,
						LiquidityPool {
							contributor_balances: BoundedVec::truncate_from(vec![
								(1u64, 2000),
								(2u64, 1000),
							]),
							distributed_profits: Some(200_000),
							vault_sharing_percent: Permill::from_percent(10),
							do_not_renew: BoundedVec::default(),
							is_rolled_over: false,
						},
					);
				});
			}
			old_storage::PerFrameFeeRevenueByVault::<Test>::mutate(1, |x| {
				for i in 144..154 {
					let _ = x.try_push(old_storage::VaultFrameFeeRevenue {
						frame_id: i,
						fee_revenue: 1000u128.into(),
						bitcoin_locks_created: 1,
						bitcoin_locks_market_value: 1000u128.into(),
						bitcoin_locks_total_satoshis: 1000u64.into(),
						satoshis_released: 100u64.into(),
					});
				}
			});
			old_storage::PerFrameFeeRevenueByVault::<Test>::mutate(2, |x| {
				for i in 144..154 {
					let _ = x.try_push(old_storage::VaultFrameFeeRevenue {
						frame_id: i,
						fee_revenue: 2000u128.into(),
						bitcoin_locks_created: 1,
						bitcoin_locks_market_value: 2000u128.into(),
						bitcoin_locks_total_satoshis: 2000u64.into(),
						satoshis_released: 200u64.into(),
					});
				}
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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(6, 4));

			let vault_1_revenue = RevenuePerFrameByVault::<Test>::get(1);
			assert_eq!(vault_1_revenue.len(), 10);
			assert_eq!(vault_1_revenue[0].frame_id, 144);
			assert_eq!(vault_1_revenue[0].bitcoin_lock_fee_revenue, 1000);
			assert_eq!(vault_1_revenue[0].bitcoin_locks_created, 1);
			assert_eq!(vault_1_revenue[0].bitcoin_locks_market_value, 1000);
			assert_eq!(vault_1_revenue[0].bitcoin_locks_total_satoshis, 1000);
			assert_eq!(vault_1_revenue[0].satoshis_released, 100);
			assert_eq!(vault_1_revenue[0].uncollected_revenue, 0);
			assert_eq!(vault_1_revenue[0].securitization_activated, 100);
			assert_eq!(vault_1_revenue[0].securitization, 100);
			assert_eq!(vault_1_revenue[0].liquidity_pool_vault_earnings, 90_000);
			assert_eq!(vault_1_revenue[0].liquidity_pool_total_earnings, 100_000);
			assert_eq!(vault_1_revenue[0].liquidity_pool_vault_capital, 190_000);
			let vault_2_revenue = RevenuePerFrameByVault::<Test>::get(2);
			assert_eq!(vault_2_revenue.len(), 10);
			assert_eq!(vault_2_revenue[9].frame_id, 153);
		});
	}
}
