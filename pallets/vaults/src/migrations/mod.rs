use crate::{Config, pallet::VaultsById};

use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::{
	argon_primitives::vault::{Vault, VaultTerms},
	*,
};

mod old_storage {
	use crate::Config;
	use argon_bitcoin::primitives::BitcoinHeight;
	use frame_support::storage_alias;
	use pallet_prelude::*;

	#[derive(
		Clone,
		PartialEq,
		Eq,
		Encode,
		Decode,
		DecodeWithMemTracking,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct Vault<AccountId, Balance>
	where
		AccountId: Codec,
		Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
	{
		/// The account assigned to operate this vault
		pub operator_account_id: AccountId,
		/// The securitization in the vault
		#[codec(compact)]
		pub securitization: Balance,
		/// The argons locked for bitcoin
		#[codec(compact)]
		pub argons_locked: Balance,
		/// Argons for bitcoin pending verification (this is "out of" the bitcoin_locked, not in
		/// addition to)
		#[codec(compact)]
		pub argons_pending_activation: Balance,
		/// Argons that will be released at the given block height (NOTE: these are grouped by next
		/// day of bitcoin blocks). These argons can be re-locked
		pub argons_scheduled_for_release: BoundedBTreeMap<BitcoinHeight, Balance, ConstU32<366>>,
		/// The securitization ratio of "total securitization" to "available for locked bitcoin"
		#[codec(compact)]
		pub securitization_ratio: FixedU128,
		/// If the vault is closed, no new bitcoin locks can be issued
		pub is_closed: bool,
		/// The terms for locked bitcoin
		pub terms: VaultTerms<Balance>,
		/// The terms that are pending to be applied to this vault at the given tick
		pub pending_terms: Option<(Tick, VaultTerms<Balance>)>,
		/// A tick at which this vault is active
		#[codec(compact)]
		pub opened_tick: Tick,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct VaultTerms<Balance>
	where
		Balance: Codec + MaxEncodedLen + Clone + TypeInfo + PartialEq + Eq,
	{
		/// The annual percent rate per argon vaulted for bitcoin locks
		#[codec(compact)]
		pub bitcoin_annual_percent_rate: FixedU128,
		/// The base fee for a bitcoin lock
		#[codec(compact)]
		pub bitcoin_base_fee: Balance,
		/// The percent of mining bonds taken by the vault
		#[codec(compact)]
		pub liquidity_pool_profit_sharing: Permill,
	}

	#[storage_alias]
	pub(super) type VaultsById<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		VaultId,
		Vault<<T as frame_system::Config>::AccountId, <T as Config>::Balance>,
		OptionQuery,
	>;

	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		#[allow(clippy::type_complexity)]
		pub vaults: Vec<(VaultId, Vault<<T as frame_system::Config>::AccountId, T::Balance>)>,
	}
}
pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		let vaults = old_storage::VaultsById::<T>::iter().collect::<Vec<_>>();

		Ok(old_storage::Model::<T> { vaults }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating vaults");

		VaultsById::<T>::translate::<old_storage::Vault<T::AccountId, T::Balance>, _>(
			|id, vault| {
				count += 1;

				// We had a bad migration on 5/23/2025 that inserted all existing bitcoin locks into
				// the "argons_scheduled_for_release", which should only be populated once bitcoins
				// are released. It's meant to track argons on hold for a year, but re-lendable.
				// In testnet, this was not an issue, but in mainnet, this has duplicated the amount
				// of argons "in vaults". The only vault with valid data in this field is vault
				// ID 7, which was opened after the faulty migration. So we will clear this
				// field for all vaults
				let mut argons_scheduled_for_release = vault.argons_scheduled_for_release;

				// check that this has the two known entries for vault ID 7
				// if so, we will restore them, otherwise we will clear the field
				if id == 7 &&
					argons_scheduled_for_release.get(&937728) ==
						Some(&(209_920_128u128 * 2u128).into())
				{
					log::info!(
						"Fixing argons_scheduled_for_release for vault ID 7 during migration"
					);
					argons_scheduled_for_release.clear();
					// utxoid 4 =
					//	 	vaultId: 7
					// 	 	lockPrice: 209,920,128
					// 		ownerAccount: 5EbtpegDiogFpFS9PTRvcXn166c7PbwgADm6bwc92bcDCh3U
					// 		securityFees: 0
					// 		satoshis: 219,090
					// 		vaultClaimHeight: 937,611
					// rounded up bitcoin "day" height = 937,728
					argons_scheduled_for_release
						.try_insert(937728, 209_920_128u128.into())
						.unwrap();

					// utxoid 8 =
					//	 	vaultId: 7
					// 	 	lockPrice: 3,389,887,723
					// 		ownerAccount: 5EbtpegDiogFpFS9PTRvcXn166c7PbwgADm6bwc92bcDCh3U
					// 		securityFees: 0
					//   	satoshis: 3,691,035
					// 		vaultClaimHeight: 937,749
					argons_scheduled_for_release
						.try_insert(937872, 3_389_887_723u128.into())
						.unwrap();
				} else {
					argons_scheduled_for_release.clear();
					log::warn!(
						"Clearing argons_scheduled_for_release for vault ID {} during migration",
						id
					);
				}

				Some(Vault {
					operator_account_id: vault.operator_account_id.clone(),
					securitization: vault.securitization,
					argons_scheduled_for_release,
					argons_locked: vault.argons_locked,
					argons_pending_activation: vault.argons_pending_activation,
					securitization_ratio: vault.securitization_ratio,
					is_closed: vault.is_closed,
					terms: VaultTerms {
						bitcoin_annual_percent_rate: vault.terms.bitcoin_annual_percent_rate,
						bitcoin_base_fee: vault.terms.bitcoin_base_fee,
						treasury_profit_sharing: vault.terms.liquidity_pool_profit_sharing,
					},
					pending_terms: vault.pending_terms.map(|x| {
						(
							x.0,
							VaultTerms {
								bitcoin_annual_percent_rate: x.1.bitcoin_annual_percent_rate,
								bitcoin_base_fee: x.1.bitcoin_base_fee,
								treasury_profit_sharing: x.1.liquidity_pool_profit_sharing,
							},
						)
					}),
					opened_tick: vault.opened_tick,
				})
			},
		);

		T::DbWeight::get().reads_writes((count) as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use alloc::collections::BTreeMap;
		use sp_core::Decode;

		let old = <old_storage::Model<T>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		let new_vaults = VaultsById::<T>::iter().collect::<BTreeMap<_, _>>();
		assert_eq!(old.vaults.len(), new_vaults.len(), "Mismatch in number of vaults");
		for (vault_id, _new_vault) in old.vaults {
			assert!(
				new_vaults.contains_key(&vault_id),
				"Vault ID {} missing in new storage",
				vault_id
			);
			if vault_id != 7 {
				// For vault ID 7, we restored the two known entries in
				// argons_scheduled_for_release, so we skip this check
				assert!(
					new_vaults[&vault_id].argons_scheduled_for_release.is_empty(),
					"argons_scheduled_for_release not cleared for vault ID {}",
					vault_id
				);
			} else {
				assert_eq!(
					new_vaults[&vault_id].argons_scheduled_for_release.len(),
					2,
					"argons_scheduled_for_release should have 2 entries for vault ID 7"
				);
			}
		}

		Ok(())
	}
}

pub type TreasuryPool<T> = frame_support::migrations::VersionedMigration<
	8,
	9,
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

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			old_storage::VaultsById::<Test>::insert(
				1,
				old_storage::Vault {
					operator_account_id: 1,
					securitization: 100,
					argons_scheduled_for_release: Default::default(),
					argons_locked: 100,
					argons_pending_activation: 0,
					securitization_ratio: FixedU128::one(),
					is_closed: false,
					terms: old_storage::VaultTerms {
						bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
						bitcoin_base_fee: 0,
						liquidity_pool_profit_sharing: Permill::from_percent(10),
					},
					pending_terms: None,
					opened_tick: 0,
				},
			);
			old_storage::VaultsById::<Test>::insert(
				2,
				old_storage::Vault {
					operator_account_id: 2,
					securitization: 200,
					argons_scheduled_for_release: Default::default(),
					argons_locked: 100,
					argons_pending_activation: 0,
					securitization_ratio: FixedU128::one(),
					is_closed: false,
					terms: old_storage::VaultTerms {
						bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
						bitcoin_base_fee: 0,
						liquidity_pool_profit_sharing: Permill::from_percent(20),
					},
					pending_terms: None,
					opened_tick: 0,
				},
			);
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

			// Verify the new storage has the expected values
			let new_value_1 = VaultsById::<Test>::get(1).unwrap();
			let new_value_2 = VaultsById::<Test>::get(2).unwrap();
			assert_eq!(new_value_1.operator_account_id, 1);
			assert_eq!(new_value_2.operator_account_id, 2);
			assert_eq!(new_value_1.terms.treasury_profit_sharing, Permill::from_percent(10));
			assert_eq!(new_value_2.terms.treasury_profit_sharing, Permill::from_percent(20));
		});
	}
}
