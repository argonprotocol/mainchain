use argon_primitives::{
	bitcoin::{BitcoinHeight, Satoshis},
	vault::{Vault, VaultTerms},
	VaultId,
};
use codec::{Decode, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

use crate::{Config, Pallet, VaultsById};

type VaultName = BoundedVec<u8, ConstU32<18>>;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use frame_support::{ensure, traits::StorageVersion};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[derive(Encode, Decode)]
struct VaultTermsV16<Balance> {
	#[codec(compact)]
	bitcoin_annual_percent_rate: FixedU128,
	#[codec(compact)]
	bitcoin_base_fee: Balance,
	#[codec(compact)]
	treasury_profit_sharing: Permill,
	#[codec(compact)]
	treasury_bonus_profit_sharing: Permill,
}

#[derive(Encode, Decode)]
struct VaultV16<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	operator_account_id: AccountId,
	delegate_account_id: Option<AccountId>,
	name: Option<VaultName>,
	last_name_change_tick: Option<Tick>,
	#[codec(compact)]
	securitization: Balance,
	#[codec(compact)]
	securitization_target: Balance,
	#[codec(compact)]
	securitization_locked: Balance,
	#[codec(compact)]
	backfill_securitization_locked: Balance,
	#[codec(compact)]
	backfill_securitization_reserved: Balance,
	#[codec(compact)]
	securitization_pending_activation: Balance,
	#[codec(compact)]
	locked_satoshis: Satoshis,
	#[codec(compact)]
	securitized_satoshis: Satoshis,
	#[codec(compact)]
	backfill_securitized_satoshis: Satoshis,
	securitization_release_schedule: BoundedBTreeMap<BitcoinHeight, Balance, ConstU32<366>>,
	#[codec(compact)]
	securitization_ratio: FixedU128,
	is_closed: bool,
	terms: VaultTermsV16<Balance>,
	pending_terms: Option<(Tick, VaultTermsV16<Balance>)>,
	#[codec(compact)]
	opened_tick: Tick,
	operational_minimum_release_tick: Option<Tick>,
}

#[derive(Encode, Decode)]
struct OperationalAccountV3<T: pallet_operational_accounts::Config> {
	vault_account: T::AccountId,
	mining_account: T::AccountId,
	encryption_pubkey: pallet_operational_accounts::OpaqueEncryptionPubkey,
	upstream_account: Option<T::AccountId>,
	uniswap_argon_transfers_in_amount: T::Balance,
	account_bitcoin_amount: T::Balance,
	account_vault_bond_amount: T::Balance,
	vault_created: bool,
	vault_bitcoin_accrual: T::Balance,
	vault_bitcoin_applied_total: T::Balance,
	#[codec(compact)]
	mining_seat_accrual: u32,
	#[codec(compact)]
	mining_seat_applied_total: u32,
	#[codec(compact)]
	operational_certifications_count: u32,
	access_code_pending: bool,
	#[codec(compact)]
	available_access_codes: u32,
	#[codec(compact)]
	rewards_earned_count: u32,
	rewards_earned_amount: T::Balance,
	rewards_collected_amount: T::Balance,
	is_operationally_certified: bool,
}

mod v16 {
	use super::*;

	#[storage_alias]
	pub(super) type VaultsById<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		VaultId,
		VaultV16<<T as frame_system::Config>::AccountId, <T as crate::Config>::Balance>,
		OptionQuery,
	>;
}

pub struct MoveVaultNameToOperationalAccountProfile<T>(core::marker::PhantomData<T>);

impl<T> UncheckedOnRuntimeUpgrade for MoveVaultNameToOperationalAccountProfile<T>
where
	T: Config + pallet_operational_accounts::Config,
{
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		ensure!(
			StorageVersion::get::<Pallet<T>>() == 16,
			TryRuntimeError::Other("vault storage version must be 16 before profile migration"),
		);
		ensure!(
			StorageVersion::get::<pallet_operational_accounts::Pallet<T>>() == 3,
			TryRuntimeError::Other(
				"operational account storage version must be 3 before profile migration",
			),
		);

		let operational_account_count =
			pallet_operational_accounts::OperationalAccounts::<T>::iter_keys()
				.fold(0u64, |count, _| count.saturating_add(1));
		let vault_count =
			v16::VaultsById::<T>::iter_keys().fold(0u64, |count, _| count.saturating_add(1));
		let linked_named_vault_count = v16::VaultsById::<T>::iter()
			.filter(|(_, vault)| {
				(vault.name.is_some() || vault.last_name_change_tick.is_some()) &&
					pallet_operational_accounts::OperationalAccountBySubAccount::<T>::contains_key(
						&vault.operator_account_id,
					)
			})
			.fold(0u64, |count, _| count.saturating_add(1));

		Ok((operational_account_count, vault_count, linked_named_vault_count).encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut operational_account_count = 0u64;
		pallet_operational_accounts::OperationalAccounts::<T>::translate::<
			OperationalAccountV3<T>,
			_,
		>(|_, account| {
			operational_account_count = operational_account_count.saturating_add(1);
			Some(pallet_operational_accounts::OperationalAccount {
				vault_account: account.vault_account,
				mining_account: account.mining_account,
				encryption_pubkey: account.encryption_pubkey,
				upstream_account: account.upstream_account,
				name: None,
				last_name_change_tick: None,
				uniswap_argon_transfers_in_amount: account.uniswap_argon_transfers_in_amount,
				account_bitcoin_amount: account.account_bitcoin_amount,
				account_vault_bond_amount: account.account_vault_bond_amount,
				vault_created: account.vault_created,
				vault_bitcoin_accrual: Zero::zero(),
				vault_bitcoin_applied_total: account
					.vault_bitcoin_applied_total
					.saturating_add(account.vault_bitcoin_accrual),
				mining_seat_accrual: 0,
				mining_seat_applied_total: account
					.mining_seat_applied_total
					.saturating_add(account.mining_seat_accrual),
				operational_certifications_count: account.operational_certifications_count,
				available_access_codes: account.available_access_codes,
				rewards_earned_count: account.rewards_earned_count,
				rewards_earned_amount: account.rewards_earned_amount,
				rewards_collected_amount: account.rewards_collected_amount,
				is_operationally_certified: account.is_operationally_certified,
			})
		});
		let mut vault_count = 0u64;
		VaultsById::<T>::translate::<
			VaultV16<<T as frame_system::Config>::AccountId, <T as crate::Config>::Balance>,
			_,
		>(|_, vault| {
			vault_count = vault_count.saturating_add(1);
			if let Some(owner) =
				pallet_operational_accounts::OperationalAccountBySubAccount::<T>::get(
					&vault.operator_account_id,
				) {
				pallet_operational_accounts::OperationalAccounts::<T>::mutate(
					owner,
					|maybe_account| {
						if let Some(account) = maybe_account {
							account.name = vault.name;
							account.last_name_change_tick = vault.last_name_change_tick;
						}
					},
				);
			}

			Some(Vault {
				operator_account_id: vault.operator_account_id,
				delegate_account_id: vault.delegate_account_id,
				securitization: vault.securitization,
				securitization_target: vault.securitization_target,
				securitization_locked: vault.securitization_locked,
				backfill_securitization_locked: vault.backfill_securitization_locked,
				backfill_securitization_reserved: vault.backfill_securitization_reserved,
				securitization_pending_activation: vault.securitization_pending_activation,
				locked_satoshis: vault.locked_satoshis,
				securitized_satoshis: vault.securitized_satoshis,
				backfill_securitized_satoshis: vault.backfill_securitized_satoshis,
				securitization_release_schedule: vault.securitization_release_schedule,
				securitization_ratio: vault.securitization_ratio,
				is_closed: vault.is_closed,
				terms: VaultTerms {
					bitcoin_annual_percent_rate: vault.terms.bitcoin_annual_percent_rate,
					bitcoin_base_fee: vault.terms.bitcoin_base_fee,
					treasury_profit_sharing: vault.terms.treasury_profit_sharing,
				},
				pending_terms: vault.pending_terms.map(|(tick, terms)| {
					(
						tick,
						VaultTerms {
							bitcoin_annual_percent_rate: terms.bitcoin_annual_percent_rate,
							bitcoin_base_fee: terms.bitcoin_base_fee,
							treasury_profit_sharing: terms.treasury_profit_sharing,
						},
					)
				}),
				opened_tick: vault.opened_tick,
				operational_minimum_release_tick: vault.operational_minimum_release_tick,
			})
		});

		frame_support::traits::StorageVersion::new(17).put::<Pallet<T>>();
		T::DbWeight::get().reads_writes(
			operational_account_count.saturating_add(vault_count.saturating_mul(3)),
			operational_account_count
				.saturating_add(vault_count.saturating_mul(2))
				.saturating_add(1),
		)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		ensure!(
			StorageVersion::get::<Pallet<T>>() == 17,
			TryRuntimeError::Other("vault storage version was not updated"),
		);
		ensure!(
			StorageVersion::get::<pallet_operational_accounts::Pallet<T>>() == 4,
			TryRuntimeError::Other("operational account storage version was not updated"),
		);

		let (expected_operational_account_count, expected_vault_count, expected_named_count) =
			<(u64, u64, u64)>::decode(&mut state.as_slice())
				.map_err(|_| TryRuntimeError::Other("could not decode profile migration state"))?;
		let mut operational_account_count = 0u64;
		let mut named_account_count = 0u64;
		let mut has_uncleared_access_code_progress = false;
		for (_, account) in pallet_operational_accounts::OperationalAccounts::<T>::iter() {
			operational_account_count.saturating_accrue(1);
			if account.name.is_some() || account.last_name_change_tick.is_some() {
				named_account_count.saturating_accrue(1);
			}
			has_uncleared_access_code_progress |=
				!account.vault_bitcoin_accrual.is_zero() || account.mining_seat_accrual > 0;
		}
		let vault_count =
			VaultsById::<T>::iter_keys().fold(0u64, |count, _| count.saturating_add(1));

		ensure!(
			operational_account_count == expected_operational_account_count,
			TryRuntimeError::Other("operational account count changed during profile migration"),
		);
		ensure!(
			vault_count == expected_vault_count,
			TryRuntimeError::Other("vault count changed during profile migration"),
		);
		ensure!(
			named_account_count == expected_named_count,
			TryRuntimeError::Other("linked vault names were not moved into account profiles"),
		);
		ensure!(
			!has_uncleared_access_code_progress,
			TryRuntimeError::Other("access code progress was not reset"),
		);

		Ok(())
	}
}

pub type MoveVaultNameToOperationalAccountProfileMigration<T> =
	frame_support::migrations::VersionedMigration<
		3,
		4,
		MoveVaultNameToOperationalAccountProfile<T>,
		pallet_operational_accounts::Pallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::{assert_ok, traits::OnRuntimeUpgrade};

	mod operational_v3 {
		use super::*;

		#[storage_alias]
		pub(super) type OperationalAccounts<T: pallet_operational_accounts::Config> = StorageMap<
			pallet_operational_accounts::Pallet<T>,
			Blake2_128Concat,
			<T as frame_system::Config>::AccountId,
			OperationalAccountV3<T>,
			OptionQuery,
		>;
	}

	#[test]
	fn moves_vault_name_into_operational_account_profile() {
		new_test_ext().execute_with(|| {
			let owner = 1;
			let vault_account = 2;
			let mining_account = 3;
			let vault_id = 1;
			let name = VaultName::truncate_from(b"VaultAlpha1".to_vec());
			let last_name_change_tick = 42;

			operational_v3::OperationalAccounts::<Test>::insert(
				owner,
				OperationalAccountV3::<Test> {
					vault_account,
					mining_account,
					encryption_pubkey: pallet_operational_accounts::OpaqueEncryptionPubkey(
						[7u8; 32],
					),
					upstream_account: Some(4),
					uniswap_argon_transfers_in_amount: 10,
					account_bitcoin_amount: 11,
					account_vault_bond_amount: 12,
					vault_created: true,
					vault_bitcoin_accrual: 13,
					vault_bitcoin_applied_total: 14,
					mining_seat_accrual: 15,
					mining_seat_applied_total: 16,
					operational_certifications_count: 17,
					access_code_pending: true,
					available_access_codes: 18,
					rewards_earned_count: 19,
					rewards_earned_amount: 20,
					rewards_collected_amount: 21,
					is_operationally_certified: true,
				},
			);
			pallet_operational_accounts::OperationalAccountBySubAccount::<Test>::insert(
				vault_account,
				owner,
			);
			v16::VaultsById::<Test>::insert(
				vault_id,
				VaultV16 {
					operator_account_id: vault_account,
					delegate_account_id: Some(5),
					name: Some(name.clone()),
					last_name_change_tick: Some(last_name_change_tick),
					securitization: 100,
					securitization_target: 101,
					securitization_locked: 102,
					backfill_securitization_locked: 103,
					backfill_securitization_reserved: 104,
					securitization_pending_activation: 105,
					locked_satoshis: 106,
					securitized_satoshis: 107,
					backfill_securitized_satoshis: 108,
					securitization_release_schedule: BoundedBTreeMap::new(),
					securitization_ratio: FixedU128::from_rational(3, 2),
					is_closed: true,
					terms: VaultTermsV16 {
						bitcoin_annual_percent_rate: FixedU128::from_rational(11, 10),
						bitcoin_base_fee: 109,
						treasury_profit_sharing: Permill::from_percent(10),
						treasury_bonus_profit_sharing: Permill::from_percent(5),
					},
					pending_terms: Some((
						120,
						VaultTermsV16 {
							bitcoin_annual_percent_rate: FixedU128::from_rational(12, 10),
							bitcoin_base_fee: 110,
							treasury_profit_sharing: Permill::from_percent(20),
							treasury_bonus_profit_sharing: Permill::from_percent(15),
						},
					)),
					opened_tick: 110,
					operational_minimum_release_tick: Some(111),
				},
			);
			StorageVersion::new(16).put::<Pallet<Test>>();
			StorageVersion::new(3).put::<pallet_operational_accounts::Pallet<Test>>();

			let state = MoveVaultNameToOperationalAccountProfileMigration::<Test>::pre_upgrade()
				.expect("pre-upgrade checks");
			let _ = MoveVaultNameToOperationalAccountProfileMigration::<Test>::on_runtime_upgrade();
			assert_ok!(MoveVaultNameToOperationalAccountProfileMigration::<Test>::post_upgrade(
				state
			));

			let account = pallet_operational_accounts::OperationalAccounts::<Test>::get(owner)
				.expect("migrated operational account");
			assert_eq!(account.name, Some(name));
			assert_eq!(account.last_name_change_tick, Some(last_name_change_tick));
			assert_eq!(account.vault_bitcoin_accrual, 0);
			assert_eq!(account.vault_bitcoin_applied_total, 27);
			assert_eq!(account.mining_seat_accrual, 0);
			assert_eq!(account.mining_seat_applied_total, 31);
			assert_eq!(account.rewards_collected_amount, 21);

			let vault = VaultsById::<Test>::get(vault_id).expect("migrated vault");
			assert_eq!(vault.operator_account_id, vault_account);
			assert_eq!(vault.delegate_account_id, Some(5));
			assert_eq!(vault.backfill_securitization_locked, 103);
			assert_eq!(vault.backfill_securitization_reserved, 104);
			assert_eq!(vault.backfill_securitized_satoshis, 108);
			assert_eq!(vault.terms.bitcoin_annual_percent_rate, FixedU128::from_rational(11, 10));
			assert_eq!(vault.terms.bitcoin_base_fee, 109);
			assert_eq!(vault.terms.treasury_profit_sharing, Permill::from_percent(10));
			let (change_tick, pending_terms) =
				vault.pending_terms.expect("pending terms preserved");
			assert_eq!(change_tick, 120);
			assert_eq!(pending_terms.bitcoin_annual_percent_rate, FixedU128::from_rational(12, 10));
			assert_eq!(pending_terms.bitcoin_base_fee, 110);
			assert_eq!(pending_terms.treasury_profit_sharing, Permill::from_percent(20));
			assert_eq!(vault.operational_minimum_release_tick, Some(111));
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 17);
			assert_eq!(StorageVersion::get::<pallet_operational_accounts::Pallet<Test>>(), 4);
		});
	}
}
