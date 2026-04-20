use crate::{Config, OpaqueEncryptionPubkey, OperationalAccount, OperationalAccounts, Pallet};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use pallet_prelude::{
	frame_support::{
		Blake2_128Concat, Twox64Concat, migrations::VersionedMigration, storage_alias,
		traits::UncheckedOnRuntimeUpgrade, weights::Weight,
	},
	frame_system, log, *,
};
use polkadot_sdk::sp_core::sr25519;
#[cfg(feature = "try-runtime")]
use polkadot_sdk::sp_runtime::TryRuntimeError;
use scale_info::TypeInfo;

pub struct RemoveStoredAccessCodes<T: Config>(core::marker::PhantomData<T>);

mod v0 {
	use super::*;

	#[derive(
		Encode, Decode, DecodeWithMemTracking, TypeInfo, RuntimeDebugNoBound, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct AccessCodeMetadata<T: Config> {
		pub sponsor: T::AccountId,
		#[codec(compact)]
		pub expiration_frame: FrameId,
	}

	#[storage_alias]
	pub type AccessCodesByPublic<T: Config> = StorageMap<
		Pallet<T>,
		Blake2_128Concat,
		sr25519::Public,
		AccessCodeMetadata<T>,
		OptionQuery,
	>;

	#[storage_alias]
	pub type AccessCodesExpiringByFrame<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, FrameId, Vec<sr25519::Public>, OptionQuery>;

	#[derive(
		Encode, Decode, DecodeWithMemTracking, TypeInfo, RuntimeDebugNoBound, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct OperationalAccount<T: Config> {
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
		pub referral_access_code_pending: bool,
		#[codec(compact)]
		pub issuable_access_codes: u32,
		#[codec(compact)]
		pub unactivated_access_codes: u32,
		#[codec(compact)]
		pub rewards_earned_count: u32,
		pub rewards_earned_amount: T::Balance,
		pub rewards_collected_amount: T::Balance,
		pub is_operational: bool,
	}

	impl<T: Config> OperationalAccount<T> {
		pub fn into_current(self) -> super::OperationalAccount<T> {
			super::OperationalAccount {
				vault_account: self.vault_account,
				mining_funding_account: self.mining_funding_account,
				mining_bot_account: self.mining_bot_account,
				encryption_pubkey: self.encryption_pubkey,
				sponsor: self.sponsor,
				has_uniswap_transfer: self.has_uniswap_transfer,
				vault_created: self.vault_created,
				bitcoin_accrual: self.bitcoin_accrual,
				bitcoin_applied_total: self.bitcoin_applied_total,
				has_treasury_pool_participation: self.has_treasury_pool_participation,
				mining_seat_accrual: self.mining_seat_accrual,
				mining_seat_applied_total: self.mining_seat_applied_total,
				operational_referrals_count: self.operational_referrals_count,
				referral_pending: self.referral_access_code_pending,
				available_referrals: self.issuable_access_codes,
				rewards_earned_count: self.rewards_earned_count,
				rewards_earned_amount: self.rewards_earned_amount,
				rewards_collected_amount: self.rewards_collected_amount,
				is_operational: self.is_operational,
			}
		}
	}
}

impl<T: Config> UncheckedOnRuntimeUpgrade for RemoveStoredAccessCodes<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		Ok((
			v0::AccessCodesByPublic::<T>::iter_keys().count() as u64,
			v0::AccessCodesExpiringByFrame::<T>::iter_keys().count() as u64,
			OperationalAccounts::<T>::iter_keys().count() as u64,
		)
			.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut operational_accounts_updated = 0u64;
		OperationalAccounts::<T>::translate::<v0::OperationalAccount<T>, _>(|_, account| {
			operational_accounts_updated = operational_accounts_updated.saturating_add(1);
			Some(account.into_current())
		});

		let access_code_result = v0::AccessCodesByPublic::<T>::clear(u32::MAX, None);
		if access_code_result.maybe_cursor.is_some() {
			log::error!("failed to fully clear legacy operational access code storage");
		}

		let expiring_code_result = v0::AccessCodesExpiringByFrame::<T>::clear(u32::MAX, None);
		if expiring_code_result.maybe_cursor.is_some() {
			log::error!("failed to fully clear legacy operational access code expiration storage");
		}

		let access_code_keys_removed = access_code_result.unique as u64;
		let expiring_code_keys_removed = expiring_code_result.unique as u64;
		let removed_keys = access_code_keys_removed.saturating_add(expiring_code_keys_removed);

		T::DbWeight::get().reads_writes(
			removed_keys.saturating_add(operational_accounts_updated).saturating_add(2),
			removed_keys.saturating_add(operational_accounts_updated),
		)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		use pallet_prelude::frame_support::ensure;

		let (_access_code_count, _expiring_code_count, operational_account_count) =
			<(u64, u64, u64)>::decode(&mut &state[..])
				.map_err(|_| TryRuntimeError::Other("failed to decode migration counts"))?;

		ensure!(
			v0::AccessCodesByPublic::<T>::iter_keys().next().is_none(),
			"legacy access code storage was not cleared"
		);
		ensure!(
			v0::AccessCodesExpiringByFrame::<T>::iter_keys().next().is_none(),
			"legacy access code expiration storage was not cleared"
		);
		ensure!(
			OperationalAccounts::<T>::iter_keys().count() as u64 == operational_account_count,
			"operational account migration changed account count"
		);
		Ok(())
	}
}

pub type RemoveStoredAccessCodesMigration<T> = VersionedMigration<
	0,
	1,
	RemoveStoredAccessCodes<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
