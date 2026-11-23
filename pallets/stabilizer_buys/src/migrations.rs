use crate::{ApprovedPoolRevision, Config, NextSettlementFrameByAccount, Pallet, PayoutQueue};
use argon_primitives::{
	treasury::{
		CollectAmounts, CollectSource, TreasuryPositionProvider, TreasuryPositionProviderWeightInfo,
	},
	AscendingRankKey, MiningFrameTransitionProvider,
};
use frame_support::{migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

/// Runtime 162 account and earnings storage conversion.
pub struct InitializeCollect<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InitializeCollect<T> {
	fn on_runtime_upgrade() -> Weight {
		ApprovedPoolRevision::<T>::put(0);

		let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
		let mut accounts = 0u64;
		for (account, old) in PayoutStateByAccount::<T>::drain() {
			accounts.saturating_accrue(1);
			T::PositionProvider::migrate_accrued(
				&account,
				CollectAmounts::from_source(CollectSource::StabilizerBuys, old.payable_reward),
			);
			let frame = old.next_payout_frame.unwrap_or(current_frame);
			NextSettlementFrameByAccount::<T>::insert(&account, frame);
			PayoutQueue::<T>::insert(AscendingRankKey::new(frame, account), ());
		}

		// Unfunded migration claims retain their aggregate backing marker. Collect and funding
		// require that backing before any payment or new allocation.
		if let Err(error) = T::PositionProvider::prepare_collect() {
			log::warn!("Pre-upgrade buy rewards await Collect backing: {error:?}");
		}

		T::DbWeight::get()
			.reads_writes(
				accounts.saturating_mul(5).saturating_add(3),
				accounts.saturating_mul(6).saturating_add(1),
			)
			.saturating_add(<T::PositionProvider as TreasuryPositionProvider<
				T::AccountId,
				Balance,
			>>::Weights::fund())
	}
}

/// Account payout storage version 1.
#[derive(Encode, Decode)]
pub(crate) struct AccountPayoutStateV1 {
	pub payable_reward: Balance,
	pub next_payout_frame: Option<FrameId>,
}

/// Runtime 162 upgrade from account payout storage version 1.
pub type CollectMigration<T> = VersionedMigration<
	1,
	2,
	InitializeCollect<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[frame_support::storage_alias]
pub(crate) type PayoutStateByAccount<T: Config> = StorageMap<
	Pallet<T>,
	Blake2_128Concat,
	<T as frame_system::Config>::AccountId,
	AccountPayoutStateV1,
	OptionQuery,
>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, *};
	use frame_support::{
		assert_ok,
		storage::unhashed,
		traits::{Hooks, OnRuntimeUpgrade},
	};
	#[test]
	fn migration_preserves_unpaid_awards_and_existing_maturity_schedules() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(1).put::<StabilizerBuys>();
			ApprovedPoolRevision::<Test>::put(7);
			System::inc_providers(&9);
			unhashed::put(
				&crate::migrations::PayoutStateByAccount::<Test>::hashed_key_for(9),
				&AccountPayoutStateV1 { payable_reward: 12_000, next_payout_frame: Some(31) },
			);
			PayoutQueue::<Test>::insert(AscendingRankKey::new(31, 9), ());
			let issuance = Balances::total_issuance();

			CollectMigration::<Test>::on_runtime_upgrade();
			CollectMigration::<Test>::on_runtime_upgrade();
			assert_eq!(StorageVersion::get::<StabilizerBuys>(), StorageVersion::new(2));
			assert_eq!(ApprovedPoolRevision::<Test>::get(), 0);
			assert_eq!(NextSettlementFrameByAccount::<Test>::get(9), Some(31));
			assert_eq!(collect_accrued(9).stabilizer_buy_earnings, 12_000);
			assert_eq!(Balances::total_issuance(), issuance + 12_000);
			assert_eq!(System::providers(&9), 2);
			assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
			assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
			assert_eq!(Balances::free_balance(9), 12_000);
			assert_eq!(Balances::total_issuance(), issuance + 12_000);

			CurrentFrame::set(31);
			StabilizerBuys::on_initialize(2);
			assert!(!NextSettlementFrameByAccount::<Test>::contains_key(9));
			assert!(!PayoutQueue::<Test>::contains_key(AscendingRankKey::new(31, 9)));
			assert_eq!(System::providers(&9), 2);
		});
	}
}
