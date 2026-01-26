use crate::{Config, pallet::VaultsById};

use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::{argon_primitives::vault::Vault, *};

mod old_storage {
	use crate::Config;
	use argon_bitcoin::primitives::BitcoinHeight;
	use frame_support::storage_alias;
	use pallet_prelude::{argon_primitives::vault::VaultTerms, *};

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

impl<T: Config + pallet_bitcoin_locks::Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		let vaults = old_storage::VaultsById::<T>::iter().collect::<Vec<_>>();
		Ok(old_storage::Model::<T> { vaults }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating vaults");

		let bitcoins = pallet_bitcoin_locks::LocksByUtxoId::<T>::iter().collect::<Vec<_>>();
		VaultsById::<T>::translate::<old_storage::Vault<T::AccountId, <T as Config>::Balance>, _>(
			|id, vault| {
				count += 1;

				let mut securitization_locked: u128 = 0;
				let mut securitization_pending_activation: u128 = 0;
				for (_utxo_id, lock) in
					bitcoins.iter().filter(|(_utxo_id, lock)| lock.vault_id == id)
				{
					let securitization = lock.get_securitization().collateral_required.into();
					securitization_locked += securitization;
					if !lock.is_verified {
						securitization_pending_activation += securitization;
					}
				}

				Some(Vault {
					operator_account_id: vault.operator_account_id,
					securitization: vault.securitization,
					securitization_locked: securitization_locked.into(),
					securitization_pending_activation: securitization_pending_activation.into(),
					securitization_release_schedule: vault.argons_scheduled_for_release,
					securitization_ratio: vault.securitization_ratio,
					is_closed: vault.is_closed,
					terms: vault.terms,
					pending_terms: vault.pending_terms,
					opened_tick: vault.opened_tick,
				})
			},
		);

		T::DbWeight::get().reads_writes((count + bitcoins.len()) as u64, count as u64)
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
		}

		Ok(())
	}
}

pub type SecuritizationMigration<T> = frame_support::migrations::VersionedMigration<
	10,
	11,
	InnerMigrate<T>,
	crate::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::mock::{Test, new_test_ext};
	use argon_bitcoin::primitives::{BitcoinCosignScriptPubkey, CompressedBitcoinPubkey};
	use frame_support::assert_ok;
	use pallet_prelude::argon_primitives::vault::VaultTerms;

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
					terms: VaultTerms {
						bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
						bitcoin_base_fee: 0,
						treasury_profit_sharing: Permill::from_percent(10),
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
					terms: VaultTerms {
						bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
						bitcoin_base_fee: 0,
						treasury_profit_sharing: Permill::from_percent(20),
					},
					pending_terms: None,
					opened_tick: 0,
				},
			);
			let utxo_1 = pallet_bitcoin_locks::LockedBitcoin {
				vault_id: 1,
				liquidity_promised: 100,
				locked_market_rate: 100,
				owner_account: 1,
				satoshis: 1,
				utxo_satoshis: Some(1),
				created_at_argon_block: 1,
				coupon_paid_fees: 0u128,
				securitization_ratio: FixedU128::from_u32(2),
				vault_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_claim_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_xpub_sources: Default::default(),
				owner_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_claim_height: 1,
				open_claim_height: 1,
				created_at_height: 1,
				utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
					wscript_hash: H256::from([0u8; 32]),
				},
				is_verified: true,
				fund_hold_extensions: Default::default(),
				security_fees: 10u128,
			};
			pallet_bitcoin_locks::LocksByUtxoId::<Test>::insert(1, utxo_1.clone());

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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(3, 2));

			// Verify the new storage has the expected values
			let new_value_1 = VaultsById::<Test>::get(1).unwrap();
			let new_value_2 = VaultsById::<Test>::get(2).unwrap();
			assert_eq!(new_value_1.operator_account_id, 1);
			assert_eq!(new_value_1.securitization_locked, 200);
			assert_eq!(new_value_2.operator_account_id, 2);
			assert_eq!(new_value_2.securitization_locked, 0);
		});
	}
}
