#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
use argon_primitives::treasury::{NetworkPositionQuantities, PositionQuantities};
use codec::DecodeAll;
#[cfg(feature = "try-runtime")]
use codec::Encode;
use core::marker::PhantomData;
use pallet_treasury_positions::NetworkTotals;
use pallet_vaults::VaultsById;
#[cfg(feature = "try-runtime")]
use polkadot_sdk::frame_support::ensure;
use polkadot_sdk::{
	frame_support::{
		storage::unhashed,
		traits::{Get, OnRuntimeUpgrade, StorageVersion},
		weights::Weight,
	},
	frame_system,
};

/// Extend Treasury Positions' network total with covered Bitcoin from canonical Vaults.
pub struct AddVaultSecuritizedSatoshis<T>(PhantomData<T>);

impl<T> OnRuntimeUpgrade for AddVaultSecuritizedSatoshis<T>
where
	T: pallet_vaults::Config + pallet_treasury_positions::Config,
{
	fn on_runtime_upgrade() -> Weight {
		let db_weight = <T as frame_system::Config>::DbWeight::get();
		if StorageVersion::get::<pallet_treasury_positions::Pallet<T>>() != StorageVersion::new(1) {
			return db_weight.reads(1);
		}

		let account_quantities: PositionQuantities<
			<T as pallet_treasury_positions::Config>::Balance,
		> = if let Some(bytes) = unhashed::get_raw(&NetworkTotals::<T>::hashed_key()) {
			let Ok(quantities) = PositionQuantities::decode_all(&mut &bytes[..]) else {
				log::error!("Cannot decode the old Treasury Positions network total");
				return db_weight.reads(2);
			};
			quantities
		} else {
			PositionQuantities::default()
		};

		let mut vault_securitized_satoshis = 0u128;
		let mut vaults_read = 0u64;
		for vault in VaultsById::<T>::iter_values() {
			vaults_read = vaults_read.saturating_add(1);
			vault_securitized_satoshis =
				vault_securitized_satoshis.saturating_add(vault.securitized_satoshis as u128);
		}

		NetworkTotals::<T>::put(NetworkPositionQuantities {
			account_quantities,
			vault_securitized_satoshis,
		});
		StorageVersion::new(2).put::<pallet_treasury_positions::Pallet<T>>();

		db_weight.reads_writes(vaults_read.saturating_add(2), 2)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, polkadot_sdk::sp_runtime::TryRuntimeError> {
		if StorageVersion::get::<pallet_treasury_positions::Pallet<T>>() != StorageVersion::new(1) {
			return Ok(Vec::new());
		}

		let account_quantities: PositionQuantities<
			<T as pallet_treasury_positions::Config>::Balance,
		> = if let Some(bytes) = unhashed::get_raw(&NetworkTotals::<T>::hashed_key()) {
			PositionQuantities::decode_all(&mut &bytes[..])
				.map_err(|_| "Cannot decode the old Treasury Positions network total")?
		} else {
			PositionQuantities::default()
		};
		let mut vault_securitized_satoshis = 0u128;
		for vault in VaultsById::<T>::iter_values() {
			vault_securitized_satoshis =
				vault_securitized_satoshis.saturating_add(vault.securitized_satoshis as u128);
		}
		Ok((account_quantities, vault_securitized_satoshis).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), polkadot_sdk::sp_runtime::TryRuntimeError> {
		if state.is_empty() {
			return Ok(());
		}

		let (account_quantities, vault_securitized_satoshis) = <(
			PositionQuantities<<T as pallet_treasury_positions::Config>::Balance>,
			u128,
		)>::decode_all(&mut &state[..])
		.map_err(|_| "Invalid Treasury Positions migration state")?;
		let migrated = NetworkTotals::<T>::get();
		ensure!(migrated.account_quantities == account_quantities, "Account totals changed");
		ensure!(
			migrated.vault_securitized_satoshis == vault_securitized_satoshis,
			"Vault covered Bitcoin total is wrong"
		);
		ensure!(
			StorageVersion::get::<pallet_treasury_positions::Pallet<T>>() == StorageVersion::new(2),
			"Treasury Positions storage version was not updated"
		);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use argon_primitives::{
		bitcoin::Satoshis,
		vault::{Vault, VaultTerms},
		Balance,
	};
	use argon_runtime::{Runtime, RuntimeGenesisConfig};
	use codec::Encode;
	use pallet_treasury_positions::{Position, PositionsByAccount};
	use polkadot_sdk::sp_runtime::{AccountId32, BuildStorage, FixedU128};

	fn vault(operator: AccountId32, covered: Satoshis) -> Vault<AccountId32, Balance> {
		Vault {
			operator_account_id: operator,
			delegate_account_id: None,
			securitization: 0,
			securitization_target: 0,
			securitization_locked: 0,
			flexible_securitization_locked: 0,
			reserved_securitization_space: 0,
			securitization_pending_activation: 0,
			securitized_satoshis: covered,
			total_satoshis: covered,
			securitization_release_schedule: Default::default(),
			committed_microgons: 0,
			is_closed: false,
			terms: VaultTerms {
				bitcoin_annual_percent_rate: FixedU128::from_u32(0),
				bitcoin_base_fee: 0,
			},
			pending_terms: None,
			opened_tick: 0,
		}
	}

	#[test]
	fn upgrade_preserves_account_totals_and_backfills_covered_satoshis() {
		polkadot_sdk::sp_io::TestExternalities::new(
			RuntimeGenesisConfig::default().build_storage().unwrap(),
		)
		.execute_with(|| {
			StorageVersion::new(1).put::<pallet_treasury_positions::Pallet<Runtime>>();
			let alice = AccountId32::new([1; 32]);
			let bob = AccountId32::new([2; 32]);
			VaultsById::<Runtime>::insert(1, vault(alice.clone(), 8));
			VaultsById::<Runtime>::insert(2, vault(alice.clone(), 13));
			VaultsById::<Runtime>::insert(3, vault(bob.clone(), 5));
			let account = Position {
				quantities: PositionQuantities { bonds: 7, ..Default::default() },
				..Default::default()
			};
			PositionsByAccount::<Runtime>::insert(&alice, &account);
			let old_network_total = account.quantities;
			polkadot_sdk::sp_io::storage::set(
				&NetworkTotals::<Runtime>::hashed_key(),
				&old_network_total.encode(),
			);
			#[cfg(feature = "try-runtime")]
			let pre_upgrade = AddVaultSecuritizedSatoshis::<Runtime>::pre_upgrade().unwrap();

			AddVaultSecuritizedSatoshis::<Runtime>::on_runtime_upgrade();
			#[cfg(feature = "try-runtime")]
			AddVaultSecuritizedSatoshis::<Runtime>::post_upgrade(pre_upgrade).unwrap();
			assert_eq!(PositionsByAccount::<Runtime>::get(&alice), Some(account));
			assert_eq!(PositionsByAccount::<Runtime>::get(&bob), None);
			assert_eq!(NetworkTotals::<Runtime>::get().account_quantities, old_network_total);
			assert_eq!(NetworkTotals::<Runtime>::get().vault_securitized_satoshis, 26);
		});
	}
}
