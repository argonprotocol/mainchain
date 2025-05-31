use crate::{
	Config,
	pallet::{VaultFundsReleasingByHeight, VaultsById},
};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::{
	argon_primitives::vault::{LockExtension, Vault},
	*,
};

mod old_storage {
	use crate::Config;
	use frame_support_procedural::storage_alias;
	use pallet_prelude::{
		argon_primitives::{bitcoin::BitcoinHeight, vault::VaultTerms},
		*,
	};

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct Obligation<AccountId: Codec, Balance: Codec> {
		#[codec(compact)]
		pub obligation_id: ObligationId,
		/// The type of funds this obligation drew from
		pub fund_type: FundType,
		#[codec(compact)]
		pub vault_id: VaultId,
		/// The recipient/beneficiary of this obligation activity
		pub beneficiary: AccountId,
		#[codec(compact)]
		pub total_fee: Balance,
		#[codec(compact)]
		pub prepaid_fee: Balance,
		#[codec(compact)]
		pub amount: Balance,
		#[codec(compact)]
		pub start_tick: Tick,
		pub expiration: ObligationExpiration,
		pub bitcoin_annual_percent_rate: Option<FixedU128>,
	}

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum ObligationExpiration {
		/// The obligation will expire at the given tick
		#[deprecated = "No longer in use"]
		AtTick(#[codec(compact)] Tick),
		/// The obligation will expire at a bitcoin block height
		BitcoinBlock(#[codec(compact)] BitcoinHeight),
	}

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum FundType {
		LockedBitcoin,
	}

	pub type ObligationId = u64;
	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub vaults_by_id: Vec<(VaultId, Vault<T>)>,
	}
	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct Vault<T: Config> {
		/// The account assigned to operate this vault
		pub operator_account_id: <T as frame_system::Config>::AccountId,
		/// The securitization in the vault
		#[codec(compact)]
		pub securitization: <T as Config>::Balance,
		/// The amount of locked bitcoins
		#[codec(compact)]
		pub bitcoin_locked: <T as Config>::Balance,
		/// Bitcoins pending verification (this is "out of" the bitcoin_locked, not in addition to)
		#[codec(compact)]
		pub bitcoin_pending: <T as Config>::Balance,
		/// The securitization ratio of "total securitization" to "available for locked bitcoin"
		#[codec(compact)]
		pub securitization_ratio: FixedU128,
		/// If the vault is closed, no new obligations can be issued
		pub is_closed: bool,
		/// The terms for locked bitcoin
		pub terms: VaultTerms<<T as Config>::Balance>,
		/// The terms that are pending to be applied to this vault at the given tick
		pub pending_terms: Option<(Tick, VaultTerms<<T as Config>::Balance>)>,
		/// A tick at which this vault is active
		#[codec(compact)]
		pub opened_tick: Tick,
	}
	#[storage_alias]
	pub(super) type VaultsById<T: Config> =
		StorageMap<crate::Pallet<T>, Twox64Concat, VaultId, Vault<T>, OptionQuery>;

	#[storage_alias]
	pub(super) type BitcoinLockCompletions<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		BitcoinHeight,
		BoundedVec<ObligationId, ConstU32<1_000>>,
		ValueQuery,
	>;
	/// Obligation by id
	#[storage_alias]
	pub(super) type ObligationsById<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		ObligationId,
		Obligation<<T as frame_system::Config>::AccountId, <T as Config>::Balance>,
		OptionQuery,
	>;

	#[storage_alias]
	pub(super) type NextObligationId<T: Config> =
		StorageValue<crate::Pallet<T>, ObligationId, OptionQuery>;
}

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let vaults_by_id = old_storage::VaultsById::<T>::iter().collect::<Vec<_>>();

		Ok(old_storage::Model::<T> { vaults_by_id }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating vaults");
		let locks = old_storage::BitcoinLockCompletions::<T>::drain().collect::<Vec<_>>();
		count += locks.len();
		let all_obligations = old_storage::ObligationsById::<T>::drain().collect::<Vec<_>>();
		count += all_obligations.len();
		let _next_obligation_id = old_storage::NextObligationId::<T>::take();
		count += 1;
		VaultsById::<T>::translate::<old_storage::Vault<T>, _>(|vault_id, vault| {
			count += 1;

			let mut vault = Vault {
				operator_account_id: vault.operator_account_id,
				securitization: vault.securitization,
				argons_locked: vault.bitcoin_locked,
				argons_pending_activation: vault.bitcoin_pending,
				argons_scheduled_for_release: Default::default(),
				securitization_ratio: vault.securitization_ratio,
				is_closed: vault.is_closed,
				terms: vault.terms,
				pending_terms: vault.pending_terms,
				opened_tick: vault.opened_tick,
			};
			for (_obligation_id, obligation) in all_obligations.iter() {
				if obligation.vault_id != vault_id {
					continue;
				}
				if let old_storage::ObligationExpiration::BitcoinBlock(block) =
					obligation.expiration
				{
					let heights = vault
						.schedule_for_release(obligation.amount, &LockExtension::new(block))
						.expect("Should be able to add");
					for height in heights {
						VaultFundsReleasingByHeight::<T>::mutate(height, |v| {
							v.try_insert(vault_id).expect("Should be able to insert");
						});
					}
				}
			}
			Some(vault)
		});

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use sp_core::Decode;

		let old = <old_storage::Model<T>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;
		let old_vaults_by_id = old.vaults_by_id;
		for (vault_id, vault) in old_vaults_by_id {
			let new_vault = VaultsById::<T>::get(vault_id).expect("Should be able to get");
			assert_eq!(new_vault.terms, vault.terms);
			assert_eq!(new_vault.argons_locked, vault.bitcoin_locked);
		}

		Ok(())
	}
}

pub type ObligationMigration<T> = frame_support::migrations::VersionedMigration<
	5,
	6,
	InnerMigrate<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::mock::{Test, new_test_ext};
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
					bitcoin_locked: 100,
					bitcoin_pending: 0,
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
			old_storage::VaultsById::<Test>::insert(
				2,
				old_storage::Vault::<Test> {
					operator_account_id: 2,
					securitization: 200,
					bitcoin_locked: 200,
					bitcoin_pending: 0,
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
			old_storage::ObligationsById::<Test>::insert(
				1,
				old_storage::Obligation {
					obligation_id: 1,
					fund_type: old_storage::FundType::LockedBitcoin,
					vault_id: 1,
					beneficiary: 1u64.into(),
					total_fee: 0,
					prepaid_fee: 0,
					amount: 100,
					start_tick: 0,
					expiration: old_storage::ObligationExpiration::BitcoinBlock(14),
					bitcoin_annual_percent_rate: None,
				},
			);
			old_storage::ObligationsById::<Test>::insert(
				2,
				old_storage::Obligation {
					obligation_id: 2,
					fund_type: old_storage::FundType::LockedBitcoin,
					vault_id: 1,
					beneficiary: 1u64.into(),
					total_fee: 0,
					prepaid_fee: 0,
					amount: 101,
					start_tick: 0,
					expiration: old_storage::ObligationExpiration::BitcoinBlock(25),
					bitcoin_annual_percent_rate: None,
				},
			);
			old_storage::ObligationsById::<Test>::insert(
				3,
				old_storage::Obligation {
					obligation_id: 3,
					fund_type: old_storage::FundType::LockedBitcoin,
					vault_id: 2,
					beneficiary: 2u64.into(),
					total_fee: 0,
					prepaid_fee: 0,
					amount: 200,
					start_tick: 0,
					expiration: old_storage::ObligationExpiration::BitcoinBlock(300),
					bitcoin_annual_percent_rate: None,
				},
			);

			old_storage::BitcoinLockCompletions::<Test>::insert(
				14u64,
				BoundedVec::truncate_from(vec![1u64]),
			);
			old_storage::BitcoinLockCompletions::<Test>::insert(
				25,
				BoundedVec::truncate_from(vec![2u64]),
			);
			old_storage::BitcoinLockCompletions::<Test>::insert(
				300,
				BoundedVec::truncate_from(vec![2u64]),
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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(9, 9));

			// check that obligations are removed
			assert_eq!(old_storage::ObligationsById::<Test>::get(1), None);
			assert_eq!(old_storage::ObligationsById::<Test>::get(2), None);

			// check that vaults are updated
			assert_eq!(VaultsById::<Test>::get(1).unwrap().argons_locked, 100);
			assert_eq!(
				VaultsById::<Test>::get(1)
					.unwrap()
					.argons_scheduled_for_release
					.into_inner()
					.into_iter()
					.collect::<Vec<_>>(),
				vec![(144, 201)]
			);
			assert_eq!(VaultsById::<Test>::get(2).unwrap().argons_locked, 200);
			assert_eq!(
				VaultsById::<Test>::get(2)
					.unwrap()
					.argons_scheduled_for_release
					.into_inner()
					.into_iter()
					.collect::<Vec<_>>(),
				vec![(432, 200)]
			);
			assert_eq!(
				VaultFundsReleasingByHeight::<Test>::get(144).into_iter().collect::<Vec<_>>(),
				vec![1]
			);
			assert_eq!(
				VaultFundsReleasingByHeight::<Test>::get(432).into_iter().collect::<Vec<_>>(),
				vec![2]
			);
		});
	}
}
