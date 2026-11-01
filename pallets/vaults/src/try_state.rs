use crate::{pallet::STORAGE_VERSION, *};
use argon_primitives::AmountRankKey;
use polkadot_sdk::sp_runtime::traits::SaturatedConversion;

pub(crate) fn check<T: Config>() -> Result<(), sp_runtime::TryRuntimeError> {
	ensure!(
		StorageVersion::get::<Pallet<T>>() == STORAGE_VERSION,
		"Vaults storage must be migrated before checking its state"
	);

	let mut total_securitization = T::Balance::zero();
	for (vault_id, vault) in VaultsById::<T>::iter() {
		ensure!(
			VaultIdByOperator::<T>::get(&vault.operator_account_id) == Some(vault_id),
			"Vault is missing its operator index"
		);
		if vault.is_closed || vault.securitization.is_zero() {
			continue;
		}

		ensure!(
			VaultSecuritizationRanks::<T>::contains_key(AmountRankKey::new(
				vault.securitization.saturated_into::<u128>(),
				vault_id,
			)),
			"Open vault is missing its securitization rank"
		);
		total_securitization = total_securitization
			.checked_add(&vault.securitization)
			.ok_or("Open vault securitization total overflows")?;
	}
	ensure!(
		total_securitization == TotalVaultSecuritization::<T>::get(),
		"Open vault securitization disagrees with its network total"
	);

	for (operator, vault_id) in VaultIdByOperator::<T>::iter() {
		let vault = VaultsById::<T>::get(vault_id)
			.ok_or("Vault operator index references a missing vault")?;
		ensure!(
			vault.operator_account_id == operator,
			"Vault operator index references the wrong operator"
		);
	}
	for rank in VaultSecuritizationRanks::<T>::iter_keys() {
		let vault = VaultsById::<T>::get(rank.holder_id())
			.ok_or("Vault securitization rank references a missing vault")?;
		ensure!(!vault.is_closed, "Closed vault remains in the securitization ranks");
		ensure!(
			!vault.securitization.is_zero(),
			"Zero-securitization vault remains in the securitization ranks"
		);
		ensure!(
			rank.amount() == vault.securitization.saturated_into::<u128>(),
			"Vault securitization rank has the wrong amount"
		);
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use frame_support::traits::Hooks;

	#[test]
	fn try_state_detects_missing_and_stale_vault_ranks() {
		new_test_ext().execute_with(|| {
			<Vaults as frame_support::traits::OnGenesis>::on_genesis();
			set_argons(1, 100_010);
			assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
			assert_ok!(Vaults::try_state(1));

			let rank = argon_primitives::AmountRankKey::new(50_000, 1u32);
			crate::VaultSecuritizationRanks::<Test>::remove(&rank);
			assert_eq!(
				Vaults::try_state(1),
				Err(DispatchError::Other("Open vault is missing its securitization rank")),
			);
			crate::VaultSecuritizationRanks::<Test>::insert(rank, ());
			crate::VaultSecuritizationRanks::<Test>::insert(
				argon_primitives::AmountRankKey::new(40_000, 1u32),
				(),
			);
			assert_eq!(
				Vaults::try_state(1),
				Err(DispatchError::Other("Vault securitization rank has the wrong amount")),
			);
		});
	}

	#[test]
	fn try_state_reconciles_vault_totals_and_operator_indexes() {
		new_test_ext().execute_with(|| {
			<Vaults as frame_support::traits::OnGenesis>::on_genesis();
			set_argons(1, 100_010);
			assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
			crate::TotalVaultSecuritization::<Test>::put(49_999);
			assert_eq!(
				Vaults::try_state(1),
				Err(DispatchError::Other(
					"Open vault securitization disagrees with its network total"
				)),
			);
			crate::TotalVaultSecuritization::<Test>::put(50_000);
			VaultIdByOperator::<Test>::insert(2, 1);
			assert_eq!(
				Vaults::try_state(1),
				Err(DispatchError::Other("Vault operator index references the wrong operator")),
			);
		});
	}
}
