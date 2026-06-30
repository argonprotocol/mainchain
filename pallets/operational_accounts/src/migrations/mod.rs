use crate::{
	pallet::Pallet as OperationalAccountsPallet, Config, OpaqueEncryptionPubkey,
	OperationalAccount, OperationalAccountBySubAccount as OperationalAccountLinks,
	OperationalAccounts as CurrentOperationalAccounts,
};
use argon_primitives::{BitcoinLocksProvider, TreasuryPoolProvider, UniswapTransferProvider};
use codec::{Decode, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[derive(Decode, Encode, Clone, PartialEq, Eq, TypeInfo, DebugNoBound, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct OperationalAccountV1<T: Config> {
	pub vault_account: T::AccountId,
	pub mining_funding_account: T::AccountId,
	pub mining_bot_account: T::AccountId,
	pub encryption_pubkey: OpaqueEncryptionPubkey,
	pub sponsor: Option<T::AccountId>,
	pub has_uniswap_transfer: bool,
	pub vault_created: bool,
	pub bitcoin_accrual: T::Balance,
	pub bitcoin_applied_total: T::Balance,
	pub has_treasury_pool_participation: bool,
	#[codec(compact)]
	pub mining_seat_accrual: u32,
	#[codec(compact)]
	pub mining_seat_applied_total: u32,
	#[codec(compact)]
	pub operational_referrals_count: u32,
	pub referral_pending: bool,
	#[codec(compact)]
	pub available_referrals: u32,
	#[codec(compact)]
	pub rewards_earned_count: u32,
	pub rewards_earned_amount: T::Balance,
	pub rewards_collected_amount: T::Balance,
	pub is_operational: bool,
}

#[storage_alias]
type OperationalAccounts<T: Config> = StorageMap<
	OperationalAccountsPallet<T>,
	Blake2_128Concat,
	<T as frame_system::Config>::AccountId,
	OperationalAccountV1<T>,
>;

pub struct MigrateOperationalAccountsV1ToV2<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for MigrateOperationalAccountsV1ToV2<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		Ok(OperationalAccounts::<T>::iter().collect::<Vec<_>>().encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let account_count = OperationalAccounts::<T>::iter_keys().count() as u64;
		let mut removed_subaccounts = 0u64;

		CurrentOperationalAccounts::<T>::translate::<OperationalAccountV1<T>, _>(
			|owner, account| {
				let account_bitcoin_amount =
					T::BitcoinLocksProvider::get_account_funded_bitcoin_amount(&owner);
				let account_vault_bond_amount =
					T::TreasuryPoolProvider::active_account_vault_bond_amount(&owner);
				let should_mark_treasury_certified = should_mark_treasury_certified::<T>(
					&account,
					account_bitcoin_amount,
					account_vault_bond_amount,
				);
				if account.mining_funding_account != account.vault_account &&
					account.mining_funding_account != account.mining_bot_account
				{
					OperationalAccountLinks::<T>::remove(&account.mining_funding_account);
					removed_subaccounts = removed_subaccounts.saturating_add(1);
				}

				Some(OperationalAccount {
					vault_account: account.vault_account,
					mining_account: account.mining_bot_account,
					encryption_pubkey: account.encryption_pubkey,
					referrer: account.sponsor.clone(),
					is_treasury_certified: should_mark_treasury_certified,
					is_upgraded_to_operations: account.is_operational || account.sponsor.is_some(),
					uniswap_argon_transfers_in_amount: if account.has_uniswap_transfer {
						T::TreasuryMinimumUniswapTransfer::get()
					} else {
						T::Balance::zero()
					},
					account_bitcoin_amount,
					account_vault_bond_amount,
					vault_created: account.vault_created,
					vault_bitcoin_accrual: account.bitcoin_accrual,
					vault_bitcoin_applied_total: account.bitcoin_applied_total,
					mining_seat_accrual: account.mining_seat_accrual,
					mining_seat_applied_total: account.mining_seat_applied_total,
					operational_referrals_count: account.operational_referrals_count,
					upgrade_code_pending: account.referral_pending,
					available_upgrade_codes: account.available_referrals,
					rewards_earned_count: account.rewards_earned_count,
					rewards_earned_amount: account.rewards_earned_amount,
					rewards_collected_amount: account.rewards_collected_amount,
					is_operationally_certified: account.is_operational,
				})
			},
		);

		T::DbWeight::get()
			.reads_writes(account_count, account_count.saturating_add(removed_subaccounts))
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let old_accounts = Vec::<(T::AccountId, OperationalAccountV1<T>)>::decode(&mut &state[..])
			.map_err(|_| TryRuntimeError::Other("failed to decode pre-upgrade state"))?;

		for (owner, old_account) in old_accounts {
			let migrated_account = CurrentOperationalAccounts::<T>::get(&owner)
				.ok_or(TryRuntimeError::Other("missing migrated operational account"))?;
			let expected_account_bitcoin_amount =
				T::BitcoinLocksProvider::get_account_funded_bitcoin_amount(&owner);
			let expected_account_vault_bond_amount =
				T::TreasuryPoolProvider::active_account_vault_bond_amount(&owner);
			let should_mark_treasury_certified = should_mark_treasury_certified::<T>(
				&old_account,
				expected_account_bitcoin_amount,
				expected_account_vault_bond_amount,
			);

			ensure!(
				migrated_account.vault_account == old_account.vault_account,
				TryRuntimeError::Other("vault account mismatch"),
			);
			ensure!(
				migrated_account.mining_account == old_account.mining_bot_account,
				TryRuntimeError::Other("mining account mismatch"),
			);
			ensure!(
				migrated_account.encryption_pubkey == old_account.encryption_pubkey,
				TryRuntimeError::Other("encryption key mismatch"),
			);
			ensure!(
				migrated_account.referrer == old_account.sponsor,
				TryRuntimeError::Other("referrer mismatch"),
			);
			ensure!(
				migrated_account.is_upgraded_to_operations ==
					(old_account.is_operational || old_account.sponsor.is_some()),
				TryRuntimeError::Other("upgrade flag mismatch"),
			);
			ensure!(
				migrated_account.uniswap_argon_transfers_in_amount ==
					if old_account.has_uniswap_transfer {
						T::TreasuryMinimumUniswapTransfer::get()
					} else {
						T::Balance::zero()
					},
				TryRuntimeError::Other("uniswap transfer amount mismatch"),
			);
			ensure!(
				migrated_account.account_bitcoin_amount == expected_account_bitcoin_amount,
				TryRuntimeError::Other("account bitcoin amount mismatch"),
			);
			ensure!(
				migrated_account.account_vault_bond_amount == expected_account_vault_bond_amount,
				TryRuntimeError::Other("account vault bond amount mismatch"),
			);
			ensure!(
				migrated_account.vault_created == old_account.vault_created,
				TryRuntimeError::Other("vault created mismatch"),
			);
			ensure!(
				migrated_account.vault_bitcoin_accrual == old_account.bitcoin_accrual,
				TryRuntimeError::Other("bitcoin accrual mismatch"),
			);
			ensure!(
				migrated_account.vault_bitcoin_applied_total == old_account.bitcoin_applied_total,
				TryRuntimeError::Other("bitcoin applied total mismatch"),
			);
			ensure!(
				migrated_account.mining_seat_accrual == old_account.mining_seat_accrual,
				TryRuntimeError::Other("mining seat accrual mismatch"),
			);
			ensure!(
				migrated_account.mining_seat_applied_total == old_account.mining_seat_applied_total,
				TryRuntimeError::Other("mining seat applied total mismatch"),
			);
			ensure!(
				migrated_account.operational_referrals_count ==
					old_account.operational_referrals_count,
				TryRuntimeError::Other("operational referrals count mismatch"),
			);
			ensure!(
				migrated_account.upgrade_code_pending == old_account.referral_pending,
				TryRuntimeError::Other("upgrade code pending mismatch"),
			);
			ensure!(
				migrated_account.available_upgrade_codes == old_account.available_referrals,
				TryRuntimeError::Other("available upgrade codes mismatch"),
			);
			ensure!(
				migrated_account.rewards_earned_count == old_account.rewards_earned_count,
				TryRuntimeError::Other("rewards earned count mismatch"),
			);
			ensure!(
				migrated_account.rewards_earned_amount == old_account.rewards_earned_amount,
				TryRuntimeError::Other("rewards earned amount mismatch"),
			);
			ensure!(
				migrated_account.rewards_collected_amount == old_account.rewards_collected_amount,
				TryRuntimeError::Other("rewards collected amount mismatch"),
			);
			ensure!(
				migrated_account.is_operationally_certified == old_account.is_operational,
				TryRuntimeError::Other("operational certification mismatch"),
			);
			ensure!(
				migrated_account.is_treasury_certified == should_mark_treasury_certified,
				TryRuntimeError::Other("treasury certification mismatch"),
			);

			let should_remove_funding_mapping = old_account.mining_funding_account !=
				old_account.vault_account &&
				old_account.mining_funding_account != old_account.mining_bot_account;
			ensure!(
				!should_remove_funding_mapping ||
					!OperationalAccountLinks::<T>::contains_key(
						&old_account.mining_funding_account,
					),
				TryRuntimeError::Other("mining funding reverse lookup still exists"),
			);
		}

		Ok(())
	}
}

fn should_mark_treasury_certified<T: Config>(
	old_account: &OperationalAccountV1<T>,
	account_bitcoin_amount: T::Balance,
	account_vault_bond_amount: T::Balance,
) -> bool {
	let has_uniswap_transfer =
		!T::UniswapTransferProvider::is_crosschain_activated() || old_account.has_uniswap_transfer;
	let has_current_treasury_certification = has_uniswap_transfer &&
		account_bitcoin_amount >= T::TreasuryMinimumBitcoin::get() &&
		account_vault_bond_amount >= T::TreasuryMinimumBonds::get();
	let had_legacy_treasury_progress = old_account.has_uniswap_transfer &&
		!account_bitcoin_amount.is_zero() &&
		!account_vault_bond_amount.is_zero();

	old_account.is_operational || had_legacy_treasury_progress || has_current_treasury_certification
}

pub type BackfillOperationalUpgradeMigration<T> = frame_support::migrations::VersionedMigration<
	1,
	2,
	MigrateOperationalAccountsV1ToV2<T>,
	OperationalAccountsPallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod test {
	use super::*;
	use crate::{
		mock::{
			new_test_ext, record_active_vault_bond_amount, record_funded_bitcoin_amount,
			set_registration_lookup, OperationalMinimumVaultSecuritization, Test,
			TreasuryMinimumBitcoin, TreasuryMinimumBonds, TreasuryMinimumUniswapTransfer,
		},
		pallet::Pallet as OperationalAccountsPallet,
		OperationalAccountBySubAccount as OperationalAccountLinks,
		OperationalAccounts as CurrentOperationalAccounts,
	};
	use frame_support::traits::OnRuntimeUpgrade;
	use sp_runtime::AccountId32;

	fn account_id_from_seed(seed: u8) -> <Test as frame_system::Config>::AccountId {
		AccountId32::new([seed; 32])
	}

	fn vault_id_from_account(account_id: &<Test as frame_system::Config>::AccountId) -> u32 {
		let account_bytes: &[u8] = account_id.as_ref();
		u32::from_le_bytes(account_bytes[0..4].try_into().expect("vault id bytes"))
	}

	#[test]
	fn migrates_operational_accounts() {
		new_test_ext().execute_with(|| {
			let upgraded_owner = account_id_from_seed(1);
			let upgraded_vault = account_id_from_seed(2);
			let upgraded_mining_funding = account_id_from_seed(3);
			let upgraded_mining_bot = account_id_from_seed(4);
			let referrer = account_id_from_seed(5);

			let treasury_only_owner = account_id_from_seed(6);
			let treasury_only_vault = account_id_from_seed(7);
			let treasury_only_mining_funding = account_id_from_seed(8);
			let treasury_only_mining_bot = account_id_from_seed(9);
			let legacy_progress_owner = account_id_from_seed(10);
			let legacy_progress_vault = account_id_from_seed(11);
			let legacy_progress_mining_funding = account_id_from_seed(12);
			let legacy_progress_mining_bot = account_id_from_seed(13);
			let legacy_operational_owner = account_id_from_seed(14);
			let legacy_operational_vault = account_id_from_seed(15);
			let legacy_operational_mining_funding = account_id_from_seed(16);
			let legacy_operational_mining_bot = account_id_from_seed(17);

			set_registration_lookup(
				upgraded_vault.clone(),
				upgraded_mining_bot.clone(),
				TreasuryMinimumBitcoin::get(),
				OperationalMinimumVaultSecuritization::get(),
				TreasuryMinimumBonds::get(),
				1,
			);
			set_registration_lookup(
				treasury_only_vault.clone(),
				treasury_only_mining_bot.clone(),
				TreasuryMinimumBitcoin::get().saturating_sub(1),
				OperationalMinimumVaultSecuritization::get(),
				TreasuryMinimumBonds::get(),
				0,
			);
			set_registration_lookup(
				legacy_progress_vault.clone(),
				legacy_progress_mining_bot.clone(),
				1,
				OperationalMinimumVaultSecuritization::get(),
				1,
				0,
			);
			set_registration_lookup(
				legacy_operational_vault.clone(),
				legacy_operational_mining_bot.clone(),
				1,
				OperationalMinimumVaultSecuritization::get(),
				1,
				2,
			);
			record_funded_bitcoin_amount(&upgraded_owner, TreasuryMinimumBitcoin::get());
			record_funded_bitcoin_amount(
				&treasury_only_owner,
				TreasuryMinimumBitcoin::get().saturating_sub(1),
			);
			record_funded_bitcoin_amount(&legacy_progress_owner, 1);
			record_funded_bitcoin_amount(&legacy_operational_owner, 1);
			record_active_vault_bond_amount(
				vault_id_from_account(&upgraded_vault),
				&upgraded_owner,
				TreasuryMinimumBonds::get(),
			);
			record_active_vault_bond_amount(
				vault_id_from_account(&treasury_only_vault),
				&treasury_only_owner,
				TreasuryMinimumBonds::get(),
			);
			record_active_vault_bond_amount(
				vault_id_from_account(&legacy_progress_vault),
				&legacy_progress_owner,
				1,
			);
			record_active_vault_bond_amount(
				vault_id_from_account(&legacy_operational_vault),
				&legacy_operational_owner,
				1,
			);

			frame_support::traits::StorageVersion::new(1).put::<OperationalAccountsPallet<Test>>();

			OperationalAccounts::<Test>::insert(
				&upgraded_owner,
				OperationalAccountV1 {
					vault_account: upgraded_vault.clone(),
					mining_funding_account: upgraded_mining_funding.clone(),
					mining_bot_account: upgraded_mining_bot.clone(),
					encryption_pubkey: OpaqueEncryptionPubkey([1u8; 32]),
					sponsor: Some(referrer.clone()),
					has_uniswap_transfer: true,
					vault_created: true,
					bitcoin_accrual: TreasuryMinimumBitcoin::get(),
					bitcoin_applied_total: 0u128.into(),
					has_treasury_pool_participation: true,
					mining_seat_accrual: 1,
					mining_seat_applied_total: 0,
					operational_referrals_count: 2,
					referral_pending: true,
					available_referrals: 1,
					rewards_earned_count: 3,
					rewards_earned_amount: 777u128.into(),
					rewards_collected_amount: 111u128.into(),
					is_operational: true,
				},
			);
			OperationalAccounts::<Test>::insert(
				&treasury_only_owner,
				OperationalAccountV1 {
					vault_account: treasury_only_vault.clone(),
					mining_funding_account: treasury_only_mining_funding.clone(),
					mining_bot_account: treasury_only_mining_bot.clone(),
					encryption_pubkey: OpaqueEncryptionPubkey([2u8; 32]),
					sponsor: None,
					has_uniswap_transfer: false,
					vault_created: false,
					bitcoin_accrual: TreasuryMinimumBitcoin::get().saturating_sub(1),
					bitcoin_applied_total: 0u128.into(),
					has_treasury_pool_participation: false,
					mining_seat_accrual: 0,
					mining_seat_applied_total: 0,
					operational_referrals_count: 0,
					referral_pending: false,
					available_referrals: 0,
					rewards_earned_count: 0,
					rewards_earned_amount: 0u128.into(),
					rewards_collected_amount: 0u128.into(),
					is_operational: false,
				},
			);
			OperationalAccounts::<Test>::insert(
				&legacy_progress_owner,
				OperationalAccountV1 {
					vault_account: legacy_progress_vault.clone(),
					mining_funding_account: legacy_progress_mining_funding.clone(),
					mining_bot_account: legacy_progress_mining_bot.clone(),
					encryption_pubkey: OpaqueEncryptionPubkey([3u8; 32]),
					sponsor: None,
					has_uniswap_transfer: true,
					vault_created: true,
					bitcoin_accrual: 1u128.into(),
					bitcoin_applied_total: 0u128.into(),
					has_treasury_pool_participation: true,
					mining_seat_accrual: 0,
					mining_seat_applied_total: 0,
					operational_referrals_count: 0,
					referral_pending: false,
					available_referrals: 0,
					rewards_earned_count: 0,
					rewards_earned_amount: 0u128.into(),
					rewards_collected_amount: 0u128.into(),
					is_operational: false,
				},
			);
			OperationalAccounts::<Test>::insert(
				&legacy_operational_owner,
				OperationalAccountV1 {
					vault_account: legacy_operational_vault.clone(),
					mining_funding_account: legacy_operational_mining_funding.clone(),
					mining_bot_account: legacy_operational_mining_bot.clone(),
					encryption_pubkey: OpaqueEncryptionPubkey([4u8; 32]),
					sponsor: None,
					has_uniswap_transfer: true,
					vault_created: true,
					bitcoin_accrual: 1u128.into(),
					bitcoin_applied_total: 0u128.into(),
					has_treasury_pool_participation: true,
					mining_seat_accrual: 2,
					mining_seat_applied_total: 0,
					operational_referrals_count: 1,
					referral_pending: false,
					available_referrals: 1,
					rewards_earned_count: 1,
					rewards_earned_amount: 10u128.into(),
					rewards_collected_amount: 0u128.into(),
					is_operational: true,
				},
			);
			OperationalAccountLinks::<Test>::insert(&upgraded_vault, &upgraded_owner);
			OperationalAccountLinks::<Test>::insert(&upgraded_mining_funding, &upgraded_owner);
			OperationalAccountLinks::<Test>::insert(&upgraded_mining_bot, &upgraded_owner);

			#[cfg(feature = "try-runtime")]
			let state = BackfillOperationalUpgradeMigration::<Test>::pre_upgrade().expect("pre-upgrade");
			BackfillOperationalUpgradeMigration::<Test>::on_runtime_upgrade();
			#[cfg(feature = "try-runtime")]
			BackfillOperationalUpgradeMigration::<Test>::post_upgrade(state).expect("post-upgrade");

			let upgraded =
				CurrentOperationalAccounts::<Test>::get(&upgraded_owner).expect("upgraded");
			assert_eq!(upgraded.referrer, Some(referrer));
			assert_eq!(upgraded.mining_account, upgraded_mining_bot);
			assert!(upgraded.is_upgraded_to_operations);
			assert!(upgraded.is_treasury_certified);
			assert_eq!(
				upgraded.uniswap_argon_transfers_in_amount,
				TreasuryMinimumUniswapTransfer::get()
			);
			assert!(upgraded.is_operationally_certified);
			assert!(!OperationalAccountLinks::<Test>::contains_key(&upgraded_mining_funding));
			assert_eq!(
				OperationalAccountLinks::<Test>::get(&upgraded_mining_bot),
				Some(upgraded_owner.clone())
			);

			let treasury_only = CurrentOperationalAccounts::<Test>::get(&treasury_only_owner)
				.expect("treasury only");
			assert!(!treasury_only.is_treasury_certified);
			assert_eq!(treasury_only.mining_account, treasury_only_mining_bot);
			assert_eq!(treasury_only.uniswap_argon_transfers_in_amount, 0);
			assert!(!treasury_only.is_upgraded_to_operations);
			assert!(!treasury_only.is_operationally_certified);

			let legacy_progress = CurrentOperationalAccounts::<Test>::get(&legacy_progress_owner)
				.expect("legacy treasury progress");
			assert!(legacy_progress.is_treasury_certified);
			assert_eq!(legacy_progress.account_bitcoin_amount, 1);
			assert_eq!(legacy_progress.account_vault_bond_amount, 1);
			assert_eq!(legacy_progress.mining_account, legacy_progress_mining_bot);
			assert!(!legacy_progress.is_upgraded_to_operations);
			assert!(!legacy_progress.is_operationally_certified);

			let legacy_operational =
				CurrentOperationalAccounts::<Test>::get(&legacy_operational_owner)
					.expect("legacy operational");
			assert!(legacy_operational.is_treasury_certified);
			assert_eq!(legacy_operational.account_bitcoin_amount, 1);
			assert_eq!(legacy_operational.account_vault_bond_amount, 1);
			assert_eq!(legacy_operational.mining_account, legacy_operational_mining_bot);
			assert!(legacy_operational.is_upgraded_to_operations);
			assert!(legacy_operational.is_operationally_certified);
		});
	}
}
