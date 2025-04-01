use crate::{Config, HoldReason, ObligationsById, VaultsById};
#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
use argon_primitives::vault::{FundType, Obligation, Vault, VaultTerms};
use frame_support::{
	pallet_prelude::*,
	traits::{fungible::MutateHold, tokens::Precision, UncheckedOnRuntimeUpgrade},
};
use log::{error, info};
use sp_runtime::{traits::Zero, FixedU128, Permill};

mod old_storage {
	use crate::{Config, Pallet};
	#[cfg(feature = "try-runtime")]
	use alloc::vec::Vec;
	use argon_primitives::{prelude::Tick, vault::ObligationExpiration, ObligationId, VaultId};
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		pallet_prelude::{OptionQuery, ValueQuery},
		storage_alias, BoundedVec, Parameter, Twox64Concat,
	};
	use scale_info::TypeInfo;
	use sp_core::ConstU32;
	use sp_runtime::{FixedU128, RuntimeDebug};

	#[storage_alias]
	pub(super) type PendingFundingModificationsByTick<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		Tick,
		BoundedVec<VaultId, <T as Config>::MaxPendingTermModificationsPerTick>,
		ValueQuery,
	>;

	#[storage_alias]
	pub(super) type BondedArgonCompletions<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		Tick,
		BoundedVec<ObligationId, <T as Config>::MaxConcurrentlyExpiringObligations>,
		ValueQuery,
	>;

	#[storage_alias]
	pub(super) type VaultsById<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		VaultId,
		Vault<<T as Config>::Balance, <T as frame_system::Config>::AccountId>,
		OptionQuery,
	>;

	#[storage_alias]
	pub(super) type ObligationsById<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		ObligationId,
		Obligation<<T as Config>::Balance, <T as frame_system::Config>::AccountId>,
		OptionQuery,
	>;

	#[storage_alias]
	pub(super) type PendingBaseFeeMaturationByTick<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		Tick,
		BoundedVec<
			(<T as frame_system::Config>::AccountId, <T as Config>::Balance, VaultId, ObligationId),
			ConstU32<1000>,
		>,
		ValueQuery,
	>;

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct Vault<B: MaxEncodedLen + Parameter, A: MaxEncodedLen + Parameter> {
		pub operator_account_id: A,
		pub bitcoin_argons: VaultArgons<B>,
		#[codec(compact)]
		pub added_securitization_ratio: FixedU128,
		#[codec(compact)]
		pub added_securitization_argons: B,
		pub bonded_argons: VaultArgons<B>,
		#[codec(compact)]
		pub mining_reward_sharing_percent_take: FixedU128,
		pub is_closed: bool,
		pub pending_terms: Option<(Tick, VaultTerms<B>)>,
		pub pending_bonded_argons: Option<(Tick, B)>,
		pub pending_bitcoins: B,
		pub activation_tick: Tick,
	}

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct Obligation<B: MaxEncodedLen + Parameter, A: MaxEncodedLen + Parameter> {
		#[codec(compact)]
		pub obligation_id: ObligationId,
		pub fund_type: FundType,
		#[codec(compact)]
		pub vault_id: VaultId,
		pub beneficiary: A,
		#[codec(compact)]
		pub total_fee: B,
		#[codec(compact)]
		pub prepaid_fee: B,
		#[codec(compact)]
		pub amount: B,
		#[codec(compact)]
		pub start_tick: Tick,
		pub expiration: ObligationExpiration,
	}

	#[derive(
		Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default,
	)]
	pub struct VaultArgons<B: MaxEncodedLen + Parameter> {
		#[codec(compact)]
		pub annual_percent_rate: FixedU128,
		#[codec(compact)]
		pub allocated: B,
		#[codec(compact)]
		pub reserved: B,
		#[codec(compact)]
		pub base_fee: B,
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct VaultTerms<B: MaxEncodedLen + Parameter> {
		#[codec(compact)]
		pub bitcoin_annual_percent_rate: FixedU128,
		#[codec(compact)]
		pub bitcoin_base_fee: B,
		#[codec(compact)]
		pub bonded_argons_annual_percent_rate: FixedU128,
		#[codec(compact)]
		pub bonded_argons_base_fee: B,
		#[codec(compact)]
		pub mining_reward_sharing_percent_take: FixedU128, // max 100, actual percent
	}

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum FundType {
		BondedArgons,
		Bitcoin,
	}

	#[cfg(feature = "try-runtime")]
	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Model<T: Config> {
		pub vaults_by_id:
			Vec<(VaultId, Vault<<T as Config>::Balance, <T as frame_system::Config>::AccountId>)>,
		pub obligations_by_id: Vec<(
			ObligationId,
			Obligation<<T as Config>::Balance, <T as frame_system::Config>::AccountId>,
		)>,
	}
}

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		let vaults_by_id = old_storage::VaultsById::<T>::iter().collect::<Vec<_>>();
		let obligations_by_id = old_storage::ObligationsById::<T>::iter().collect::<Vec<_>>();

		Ok(old_storage::Model::<T> { vaults_by_id, obligations_by_id }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		info!("Migrating vault storage");
		let pending_funding = old_storage::PendingFundingModificationsByTick::<T>::drain();
		let bonded_argon_completions = old_storage::BondedArgonCompletions::<T>::drain();

		count += pending_funding.count() + bonded_argon_completions.count();

		let base_fee_maturation = old_storage::PendingBaseFeeMaturationByTick::<T>::drain();
		for (_tick, fee_holds) in base_fee_maturation {
			count += 1;
			for (who, amount, vault_id, obligation_id) in fee_holds {
				count += 1;
				if let Err(e) = T::Currency::release(
					&HoldReason::ObligationFee.into(),
					&who,
					amount,
					Precision::Exact,
				) {
					error!(
						"Failed to release {:?} fee hold for obligation {} for vault {}: {:?}",
						amount, obligation_id, vault_id, e
					);
				}
			}
		}

		VaultsById::<T>::translate::<old_storage::Vault<T::Balance, T::AccountId>, _>(|id, vlt| {
			count += 1;
			info!("Migrating vault {id}");
			let mut to_return = vlt.bonded_argons.allocated;
			if let Some((_, argons)) = vlt.pending_bonded_argons {
				to_return = argons;
			}

			if to_return > T::Balance::zero() {
				if let Err(e) = T::Currency::release(
					&HoldReason::EnterVault.into(),
					&vlt.operator_account_id,
					to_return,
					Precision::Exact,
				) {
					error!("Failed to release {:?} funds for vault {}: {:?}", to_return, id, e);
				}
			}
			Some(Vault {
				operator_account_id: vlt.operator_account_id,
				securitization: vlt.bitcoin_argons.allocated + vlt.added_securitization_argons,
				securitization_ratio: vlt.added_securitization_ratio + FixedU128::from_u32(1),
				bitcoin_locked: vlt.bitcoin_argons.reserved,
				terms: VaultTerms {
					bitcoin_annual_percent_rate: vlt.bitcoin_argons.annual_percent_rate,
					bitcoin_base_fee: vlt.bitcoin_argons.base_fee,
					mining_bond_percent_take: Permill::zero(),
				},
				pending_terms: vlt.pending_terms.map(|(tick, terms)| {
					(
						tick,
						VaultTerms {
							bitcoin_base_fee: terms.bitcoin_base_fee,
							bitcoin_annual_percent_rate: terms.bitcoin_annual_percent_rate,
							mining_bond_percent_take: Permill::zero(),
						},
					)
				}),
				is_closed: vlt.is_closed,
				bitcoin_pending: vlt.pending_bitcoins,
				opened_tick: vlt.activation_tick,
			})
		});

		ObligationsById::<T>::translate::<old_storage::Obligation<T::Balance, T::AccountId>, _>(
			|_id, ob| {
				count += 1;
				// removing bonded argons
				if ob.fund_type == old_storage::FundType::BondedArgons {
					return None;
				}
				let vault = VaultsById::<T>::get(ob.vault_id).expect("Vault must exist");
				let converted = Obligation {
					obligation_id: ob.obligation_id,
					fund_type: FundType::LockedBitcoin,
					vault_id: ob.vault_id,
					amount: ob.amount,
					prepaid_fee: ob.prepaid_fee,
					total_fee: ob.total_fee,
					beneficiary: ob.beneficiary,
					start_tick: ob.start_tick,
					expiration: ob.expiration,
					bitcoin_annual_percent_rate: Some(vault.terms.bitcoin_annual_percent_rate),
				};

				info!(
					"Migrated {:?} obligation {} for {:?}",
					ob.fund_type, ob.obligation_id, ob.amount
				);
				Some(converted)
			},
		);

		count += 1;

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use crate::ObligationCompletionByTick;
		use argon_primitives::vault::ObligationExpiration;
		use codec::Decode;
		use frame_support::{ensure, fail};

		let old = <old_storage::Model<T>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		for (id, vlt) in old.vaults_by_id {
			let new_vlt = VaultsById::<T>::get(&id).expect("Vault not found");
			info!("Checking vault {id} post migrate");
			ensure!(
				vlt.pending_bitcoins == new_vlt.bitcoin_pending,
				"New pending bitcoins value not set correctly"
			);
			ensure!(
				vlt.operator_account_id == new_vlt.operator_account_id,
				"New operator account id not correctly correctly"
			);
			ensure!(
				vlt.bitcoin_argons.reserved == new_vlt.bitcoin_locked,
				"New locked bitcoin argons not set correctly"
			);
			ensure!(
				vlt.bitcoin_argons.allocated + vlt.added_securitization_argons ==
					new_vlt.securitization,
				"New locked bitcoin allocated argons not set correctly"
			);

			ensure!(
				vlt.bitcoin_argons.base_fee == new_vlt.terms.bitcoin_base_fee,
				"New terms not set correctly"
			);
			ensure!(vlt.is_closed == new_vlt.is_closed, "New is closed not set correctly");
		}

		for (id, ob) in old.obligations_by_id {
			// removed
			if let ObligationExpiration::AtTick(tick) = ob.expiration {
				if ObligationCompletionByTick::<T>::get(tick).contains(&id) {
					error!(
							"obligation_id {} should not exist in ObligationCompletionByTick for tick {}",
							id, tick
						);
					fail!("Obligation not found in ObligationCompletionByTick for tick");
				}
				continue;
			}
			let new = ObligationsById::<T>::get(&id).expect("Obligation not found");
			ensure!(new.amount == ob.amount, "Obligation mismatch");
		}

		Ok(())
	}
}

pub type MiningBondBidPoolMigration<T> = frame_support::migrations::VersionedMigration<
	3, // The migration will only execute when the on-chain storage version is 1
	4, // The on-chain storage version will be set to 2 after the migration is complete
	InnerMigrate<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::mock::{new_test_ext, Balances, System, Test};
	use argon_primitives::vault::ObligationExpiration;
	use frame_support::{assert_ok, traits::fungible::UnbalancedHold};
	use sp_runtime::FixedU128;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			old_storage::VaultsById::<Test>::insert(
				1,
				old_storage::Vault {
					operator_account_id: 1,
					bitcoin_argons: old_storage::VaultArgons {
						annual_percent_rate: FixedU128::from_float(1.0),
						allocated: 2,
						reserved: 3,
						base_fee: 4,
					},
					added_securitization_ratio: FixedU128::from_float(5.0),
					added_securitization_argons: 6,
					bonded_argons: old_storage::VaultArgons {
						annual_percent_rate: FixedU128::from_float(1.1),
						allocated: 8,
						reserved: 10,
						base_fee: 9,
					},
					mining_reward_sharing_percent_take: FixedU128::from_float(11.0),
					is_closed: false,
					pending_terms: Some((
						1,
						old_storage::VaultTerms {
							bitcoin_annual_percent_rate: FixedU128::from_float(12.0),
							bitcoin_base_fee: 13,
							bonded_argons_annual_percent_rate: FixedU128::from_float(14.0),
							bonded_argons_base_fee: 15,
							mining_reward_sharing_percent_take: FixedU128::from_float(60.0),
						},
					)),
					pending_bonded_argons: Some((1, 1000)),
					pending_bitcoins: 1,
					activation_tick: 1,
				},
			);
			System::inc_providers(&1);
			Balances::set_balance_on_hold(&HoldReason::EnterVault.into(), &1, 1014)
				.expect("Cannot set hold balances");
			old_storage::VaultsById::<Test>::insert(
				2,
				old_storage::Vault {
					operator_account_id: 2,
					bitcoin_argons: old_storage::VaultArgons {
						annual_percent_rate: FixedU128::from_float(11.0),
						allocated: 12,
						reserved: 13,
						base_fee: 14,
					},
					added_securitization_ratio: FixedU128::from_float(15.0),
					added_securitization_argons: 16,
					bonded_argons: old_storage::VaultArgons {
						annual_percent_rate: FixedU128::from_float(17.0),
						allocated: 18,
						reserved: 17,
						base_fee: 20,
					},
					mining_reward_sharing_percent_take: FixedU128::from_float(1.0),
					is_closed: false,
					pending_terms: Some((
						2,
						old_storage::VaultTerms {
							bitcoin_annual_percent_rate: FixedU128::from_float(11.0),
							bitcoin_base_fee: 12,
							bonded_argons_annual_percent_rate: FixedU128::from_float(1.0),
							bonded_argons_base_fee: 1,
							mining_reward_sharing_percent_take: FixedU128::from_float(1.0),
						},
					)),
					pending_bonded_argons: None,
					pending_bitcoins: 1,
					activation_tick: 1,
				},
			);
			System::inc_providers(&2);
			Balances::set_balance_on_hold(&HoldReason::EnterVault.into(), &2, 20)
				.expect("Cannot set hold balances");
			old_storage::ObligationsById::<Test>::insert(
				1,
				old_storage::Obligation {
					obligation_id: 1,
					fund_type: old_storage::FundType::BondedArgons,
					vault_id: 1,
					beneficiary: 1,
					total_fee: 1,
					prepaid_fee: 1,
					amount: 1,
					start_tick: 1,
					expiration: ObligationExpiration::AtTick(10),
				},
			);
			old_storage::BondedArgonCompletions::<Test>::try_append(10, 1).unwrap();
			old_storage::ObligationsById::<Test>::insert(
				2,
				old_storage::Obligation {
					obligation_id: 2,
					fund_type: old_storage::FundType::Bitcoin,
					vault_id: 2,
					beneficiary: 2,
					total_fee: 1,
					prepaid_fee: 1,
					amount: 1,
					start_tick: 1,
					expiration: ObligationExpiration::BitcoinBlock(100000),
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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(6, 6));

			// Check the new value
			let new = VaultsById::<Test>::iter().collect::<Vec<_>>();
			assert_eq!(new.len(), 2);
			assert_eq!(new[0].1.operator_account_id, 1);
			assert_eq!(new[1].1.operator_account_id, 2);
			assert_eq!(Balances::free_balance(&1), 1000); // pending bonded argons
			assert_eq!(Balances::free_balance(&2), 18);

			assert_eq!(new[0].1.securitization, 2 + 6);
			assert_eq!(new[1].1.securitization, 12 + 16);
			assert_eq!(new[0].1.bitcoin_locked, 3);
			assert_eq!(new[1].1.bitcoin_locked, 13);
			assert_eq!(new[0].1.terms.bitcoin_base_fee, 4);
			assert_eq!(new[1].1.terms.bitcoin_base_fee, 14);
			assert_eq!(new[0].1.is_closed, false);
			assert_eq!(new[1].1.is_closed, false);

			assert_eq!(ObligationsById::<Test>::get(1), None);
			assert_eq!(ObligationsById::<Test>::get(2).unwrap().fund_type, FundType::LockedBitcoin);
		});
	}
}
