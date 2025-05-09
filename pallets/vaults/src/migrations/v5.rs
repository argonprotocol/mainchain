use crate::{migrations::v4, Config, HoldReason, ObligationsById, VaultsById};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use pallet_prelude::argon_primitives::vault::Obligation;

#[cfg(feature = "try-runtime")]
mod old_storage {
	use super::Obligation;
	use crate::Config;
	use pallet_prelude::*;

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Model<T: Config> {
		pub obligations_by_id: Vec<(
			ObligationId,
			Obligation<<T as frame_system::Config>::AccountId, <T as Config>::Balance>,
		)>,
	}
}

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		let obligations_by_id = ObligationsById::<T>::iter().collect::<Vec<_>>();

		Ok(old_storage::Model::<T> { obligations_by_id }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating vault storage");
		let completions = v4::ObligationCompletionByTick::<T>::drain();

		count += completions.count();

		for (id, mut obligation) in ObligationsById::<T>::iter() {
			count += 1;
			let vault_id = obligation.vault_id;
			let Some(vault) = VaultsById::<T>::get(vault_id) else {
				log::error!("Vault {} not found for obligation {}", vault_id, id);
				continue;
			};
			let remaining_fee = obligation.total_fee.saturating_sub(obligation.prepaid_fee);
			if let Err(e) = T::Currency::transfer_on_hold(
				#[allow(deprecated)]
				&HoldReason::ObligationFee.into(),
				&obligation.beneficiary,
				&vault.operator_account_id,
				remaining_fee,
				Precision::Exact,
				Restriction::Free,
				Fortitude::Force,
			) {
				log::error!(
					"Failed to release {:?} apr fee hold for obligation {} for vault {}: {:?}",
					remaining_fee,
					id,
					vault_id,
					e
				);
			}
			obligation.prepaid_fee = obligation.total_fee;
			ObligationsById::<T>::insert(id, obligation);
		}

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;
		use frame_support::ensure;

		let old = <old_storage::Model<T>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		for (id, ob) in old.obligations_by_id {
			let new = ObligationsById::<T>::get(&id).expect("Obligation not found");
			ensure!(new.amount == ob.amount, "Obligation mismatch");
			ensure!(new.total_fee == new.prepaid_fee, "Obligation fee not applied");
		}

		Ok(())
	}
}

pub type VaultStats<T> = frame_support::migrations::VersionedMigration<
	4, // The migration will only execute when the on-chain storage version is 1
	5, // The on-chain storage version will be set to 2 after the migration is complete
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
	use pallet_prelude::argon_primitives::vault::{FundType, Obligation, Vault, VaultTerms};
	use sp_runtime::FixedU128;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			VaultsById::<Test>::insert(
				1,
				Vault {
					operator_account_id: 1,
					bitcoin_locked: 0,
					securitization: 2,
					securitization_ratio: FixedU128::from_float(0.5),
					is_closed: false,
					pending_terms: None,
					terms: VaultTerms {
						bitcoin_base_fee: 4,
						bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
						liquidity_pool_profit_sharing: Permill::from_percent(10),
					},
					bitcoin_pending: 0,
					opened_tick: 1,
				},
			);
			System::inc_providers(&1);
			Balances::set_balance_on_hold(&HoldReason::EnterVault.into(), &1, 1014)
				.expect("Cannot set hold balances");
			System::inc_providers(&2);
			#[allow(deprecated)]
			Balances::set_balance_on_hold(&HoldReason::ObligationFee.into(), &2, 2000)
				.expect("Cannot set hold balances");
			ObligationsById::<Test>::insert(
				1,
				Obligation {
					obligation_id: 1,
					fund_type: FundType::LockedBitcoin,
					vault_id: 1,
					beneficiary: 2,
					total_fee: 5000,
					prepaid_fee: 3000,
					amount: 1,
					start_tick: 1,
					expiration: ObligationExpiration::BitcoinBlock(100000),
					bitcoin_annual_percent_rate: Some(FixedU128::from_float(0.1)),
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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(1, 1));

			assert_eq!(ObligationsById::<Test>::get(1).unwrap().prepaid_fee, 5000);
			#[allow(deprecated)]
			let balance_on_hold = Balances::balance_on_hold(&HoldReason::ObligationFee.into(), &2);
			assert_eq!(balance_on_hold, 0);
		});
	}
}
