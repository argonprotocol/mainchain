use crate::{Config, RewardReserves};
use frame_support::traits::OnRuntimeUpgrade;
use pallet_prelude::*;
use polkadot_sdk::sp_runtime::traits::AccountIdConversion;

/// Move the marketing account balance into reserves without changing existing storage encodings.
/// No version guard is needed: only successfully removed funds are credited, once.
pub struct VirtualizeMarketingReserveMigration<T>(PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for VirtualizeMarketingReserveMigration<T> {
	fn on_runtime_upgrade() -> Weight {
		let account: T::AccountId =
			T::PalletId::get().into_sub_account_truncating(*b"treasury-reserve");
		let balance = T::Currency::balance(&account);
		if balance.is_zero() {
			return T::DbWeight::get().reads(1);
		}

		match T::Currency::burn_from(
			&account,
			balance,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Force,
		) {
			Ok(burned) => {
				RewardReserves::<T>::mutate(|reserves| {
					reserves.marketing.saturating_accrue(burned);
				});
			},
			Err(error) => {
				tracing::error!(?error, "Could not virtualize the legacy marketing reserve");
			},
		}

		// The source balance makes this idempotent and allows retry on a later upgrade.
		T::DbWeight::get().reads_writes(4, 3)
	}
}
