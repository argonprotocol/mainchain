use crate::{Config, VaultsById};
use argon_primitives::{
	bitcoin::{BitcoinHeight, Satoshis},
	tick::Tick,
	vault::{Vault, VaultTerms},
};
use codec::{Codec, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::iter::Sum;
use frame_support::{traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;
use scale_info::TypeInfo;
use sp_arithmetic::FixedU128;
use sp_runtime::traits::AtLeast32BitUnsigned;

#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct VaultV13<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	pub operator_account_id: AccountId,
	#[codec(compact)]
	pub securitization: Balance,
	#[codec(compact)]
	pub securitization_target: Balance,
	#[codec(compact)]
	pub securitization_locked: Balance,
	#[codec(compact)]
	pub securitization_pending_activation: Balance,
	#[codec(compact)]
	pub locked_satoshis: Satoshis,
	#[codec(compact)]
	pub securitized_satoshis: Satoshis,
	pub securitization_release_schedule: BoundedBTreeMap<BitcoinHeight, Balance, ConstU32<366>>,
	#[codec(compact)]
	pub securitization_ratio: FixedU128,
	pub is_closed: bool,
	pub terms: VaultTerms<Balance>,
	pub pending_terms: Option<(Tick, VaultTerms<Balance>)>,
	#[codec(compact)]
	pub opened_tick: Tick,
}

pub struct MigrateVaultV13ToV14<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for MigrateVaultV13ToV14<T>
where
	T::AccountId: Codec,
	T::Balance: Codec
		+ Copy
		+ MaxEncodedLen
		+ Default
		+ AtLeast32BitUnsigned
		+ Clone
		+ TypeInfo
		+ core::fmt::Debug
		+ PartialEq
		+ Eq
		+ Sum,
{
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;
		Ok((VaultsById::<T>::iter().count() as u64).encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut translated = 0u64;

		VaultsById::<T>::translate::<VaultV13<T::AccountId, T::Balance>, _>(|_, vault| {
			translated = translated.saturating_add(1);
			Some(Vault {
				operator_account_id: vault.operator_account_id,
				bitcoin_lock_delegate_account: None,
				name: None,
				last_name_change_tick: None,
				securitization: vault.securitization,
				securitization_target: vault.securitization_target,
				securitization_locked: vault.securitization_locked,
				securitization_pending_activation: vault.securitization_pending_activation,
				locked_satoshis: vault.locked_satoshis,
				securitized_satoshis: vault.securitized_satoshis,
				securitization_release_schedule: vault.securitization_release_schedule,
				securitization_ratio: vault.securitization_ratio,
				is_closed: vault.is_closed,
				terms: vault.terms,
				pending_terms: vault.pending_terms,
				opened_tick: vault.opened_tick,
				operational_minimum_release_tick: None,
			})
		});

		T::DbWeight::get().reads_writes(translated, translated)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;
		use frame_support::ensure;

		let expected = u64::decode(&mut &state[..])
			.map_err(|_| sp_runtime::TryRuntimeError::Other("failed to decode vault count"))?;
		let mut migrated = 0u64;
		for (_, vault) in VaultsById::<T>::iter() {
			ensure!(
				vault.bitcoin_lock_delegate_account.is_none(),
				"migrated vault unexpectedly had delegate account set"
			);
			ensure!(
				vault.operational_minimum_release_tick.is_none(),
				"migrated vault unexpectedly had operational minimum release tick set"
			);
			ensure!(vault.name.is_none(), "migrated vault unexpectedly had a name set");
			ensure!(
				vault.last_name_change_tick.is_none(),
				"migrated vault unexpectedly had a last name change tick set"
			);
			migrated = migrated.saturating_add(1);
		}
		ensure!(expected == migrated, "vault count mismatch after migration");
		Ok(())
	}
}

pub type MigrateVaultV13ToV14Migration<T> = frame_support::migrations::VersionedMigration<
	13,
	14,
	MigrateVaultV13ToV14<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
