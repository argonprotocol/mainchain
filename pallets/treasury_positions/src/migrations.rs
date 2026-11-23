#[cfg(feature = "try-runtime")]
use crate::{pallet::STORAGE_VERSION, PendingCollectBacking};
use crate::{
	CollectTotals, Config, EarningPositions, EarningsIndexState, EarningsIndexes, MiningMintIndex,
	MiningMintIndexState, Pallet, Position, PositionsByAccount,
};
#[cfg(feature = "try-runtime")]
use alloc::collections::BTreeMap;
use argon_primitives::{
	treasury::{
		CollectTotals as CollectTotalsData, IndexedEarningsPosition, PositionQuantities,
		UpstreamPosition,
	},
	BlockRewardAccountsProvider, MiningFrameTransitionProvider,
};
#[cfg(any(test, feature = "try-runtime"))]
use frame_support::storage::unhashed;
use frame_support::{migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

/// Positions layout in the released v1.6.0 runtime (storage version 2).
#[derive(Encode, Decode, Default)]
pub(crate) struct PositionV2<Balance: Codec + MaxEncodedLen> {
	#[codec(compact)]
	pub bond_principal: Balance,
	pub quantities: PositionQuantities<Balance>,
	pub upstream: Option<UpstreamPosition<Balance>>,
}

#[cfg(feature = "try-runtime")]
type CollectUpgradeSnapshot<T> = (
	BTreeMap<
		<T as frame_system::Config>::AccountId,
		(Option<PositionV2<<T as Config>::Balance>>, u32),
	>,
	BTreeMap<<T as frame_system::Config>::AccountId, u32>,
	FrameId,
	<T as Config>::Balance,
	<T as Config>::Balance,
);

pub type BackCollectMigration<T> = VersionedMigration<
	2,
	3,
	InitializeCollect<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

/// Preserve released positions and initialize their Collect and mining checkpoints.
pub struct InitializeCollect<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InitializeCollect<T> {
	fn on_runtime_upgrade() -> Weight {
		let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
		EarningsIndexes::<T>::put(EarningsIndexState {
			start_frame: current_frame.max(1),
			..Default::default()
		});
		MiningMintIndex::<T>::put(MiningMintIndexState {
			microgons_per_seat: T::Balance::zero(),
			last_processed_frame: current_frame.checked_sub(1),
		});

		let mut accounts = 0u64;
		PositionsByAccount::<T>::translate::<PositionV2<T::Balance>, _>(|account, previous| {
			accounts.saturating_accrue(1);
			frame_system::Pallet::<T>::inc_providers(&account);
			Some(Position {
				bond_principal: previous.bond_principal,
				quantities: previous.quantities,
				upstream: previous.upstream,
				earning_positions: EarningPositions {
					bonds: IndexedEarningsPosition {
						quantity: previous.quantities.bonds,
						..Default::default()
					},
					stakes: IndexedEarningsPosition {
						quantity: previous.quantities.stakes,
						..Default::default()
					},
					bitcoin_liquidity: IndexedEarningsPosition {
						quantity: previous.quantities.fission_liquidity.into(),
						..Default::default()
					},
					..Default::default()
				},
				..Default::default()
			})
		});
		for account in T::BlockRewardAccountsProvider::get_active_mining_rewards_accounts() {
			PositionsByAccount::<T>::mutate(&account, |record| {
				let position = record.get_or_insert_with(|| {
					frame_system::Pallet::<T>::inc_providers(&account);
					Position::default()
				});
				position.mining_seats.saturating_accrue(1);
			});
			accounts.saturating_accrue(1);
		}
		CollectTotals::<T>::put(CollectTotalsData::<T::Balance>::default());

		T::DbWeight::get().reads_writes(
			accounts.saturating_mul(3).saturating_add(1),
			accounts.saturating_mul(3).saturating_add(3),
		)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let mut positions = BTreeMap::<T::AccountId, (Option<PositionV2<T::Balance>>, u32)>::new();
		for account in PositionsByAccount::<T>::iter_keys() {
			let previous = unhashed::get::<PositionV2<T::Balance>>(
				&PositionsByAccount::<T>::hashed_key_for(&account),
			)
			.ok_or("Cannot decode a released V2 position")?;
			let providers = frame_system::Pallet::<T>::providers(&account);
			positions.insert(account, (Some(previous), providers));
		}
		let mut mining_seats = BTreeMap::<T::AccountId, u32>::new();
		for account in T::BlockRewardAccountsProvider::get_active_mining_rewards_accounts() {
			mining_seats.entry(account.clone()).or_default().saturating_accrue(1);
			let providers = frame_system::Pallet::<T>::providers(&account);
			positions.entry(account).or_insert((None, providers));
		}
		let snapshot: CollectUpgradeSnapshot<T> = (
			positions,
			mining_seats,
			T::MiningFrameTransitionProvider::get_current_frame_id(),
			T::Currency::total_issuance(),
			T::OwnershipCurrency::total_issuance(),
		);
		Ok(snapshot.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let (positions, mining_seats, frame, argons, micronots) =
			CollectUpgradeSnapshot::<T>::decode(&mut &state[..])
				.map_err(|_| "Cannot decode Collect upgrade snapshot")?;
		ensure!(
			StorageVersion::get::<Pallet<T>>() == STORAGE_VERSION,
			"Collect version wrapper did not complete"
		);
		ensure!(
			PositionsByAccount::<T>::iter_keys().count() == positions.len(),
			"Collect account set changed"
		);
		for (account, (previous, providers)) in positions {
			let previous = previous.unwrap_or_default();
			let position =
				PositionsByAccount::<T>::get(&account).ok_or("Migrated position is missing")?;
			ensure!(
				position.bond_principal == previous.bond_principal,
				"Position principal changed"
			);
			ensure!(position.quantities == previous.quantities, "Position quantities changed");
			ensure!(position.upstream == previous.upstream, "Position upstream changed");
			ensure!(
				position.mining_seats == mining_seats.get(&account).copied().unwrap_or_default(),
				"Mining seat count is incorrect"
			);
			ensure!(
				position.mining_mint_checkpoint.is_zero(),
				"Mining checkpoint includes old rewards"
			);
			ensure!(position.collect_accrued.is_zero(), "Migration credited historical rewards");
			ensure!(position.collected.is_zero(), "Migration credited historical collections");
			ensure!(
				position.earning_positions ==
					EarningPositions {
						bonds: IndexedEarningsPosition {
							quantity: previous.quantities.bonds,
							..Default::default()
						},
						stakes: IndexedEarningsPosition {
							quantity: previous.quantities.stakes,
							..Default::default()
						},
						bitcoin_liquidity: IndexedEarningsPosition {
							quantity: previous.quantities.fission_liquidity.into(),
							..Default::default()
						},
						..Default::default()
					},
				"Earning quantities or checkpoints were not initialized"
			);
			ensure!(
				frame_system::Pallet::<T>::providers(&account) == providers.saturating_add(1),
				"Position provider reference is incorrect"
			);
		}
		ensure!(
			EarningsIndexes::<T>::get() ==
				EarningsIndexState { start_frame: frame.max(1), ..Default::default() },
			"Earnings index is incorrect"
		);
		ensure!(
			MiningMintIndex::<T>::get() ==
				MiningMintIndexState {
					microgons_per_seat: T::Balance::zero(),
					last_processed_frame: frame.checked_sub(1)
				},
			"Mining index is incorrect"
		);
		ensure!(
			CollectTotals::<T>::get() == CollectTotalsData::default(),
			"Migration changed Collect totals"
		);
		ensure!(
			!PendingCollectBacking::<T>::exists(),
			"Released V2 positions have no unpaid backing"
		);
		ensure!(T::Currency::total_issuance() == argons, "Migration changed ARGON issuance");
		ensure!(
			T::OwnershipCurrency::total_issuance() == micronots,
			"Migration changed ARGNOT issuance"
		);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, *};
	use frame_support::{assert_ok, traits::OnRuntimeUpgrade};
	#[test]
	fn collect_cutover_seeds_active_seats_without_crediting_past_mining_rewards() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(2).put::<TreasuryPositions>();
			CurrentFrame::set(8);
			ActiveMiningAccounts::set(vec![9, 9, 10]);
			BackCollectMigration::<Test>::on_runtime_upgrade();
			let position = PositionsByAccount::<Test>::get(9).unwrap();
			assert_eq!(position.mining_seats, 2);
			assert_eq!(position.mining_mint_checkpoint, 0);
			assert_eq!(position.collect_accrued, CollectAmounts::default());
			assert_eq!(MiningMintIndex::<Test>::get().last_processed_frame, Some(7));
			assert_eq!(Pallet::<Test>::allocate_mining_mint(7, 100, 3), Ok(false));
			assert_eq!(Pallet::<Test>::allocate_mining_mint(8, 10, 3), Ok(true));
			BackCollectMigration::<Test>::on_runtime_upgrade();
			assert_eq!(PositionsByAccount::<Test>::get(9).unwrap().mining_seats, 2);
			assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
			assert_eq!(Balances::balance(&9), 20);
		});
	}

	#[test]
	fn migrating_positions_preserves_principal_quantities_and_upstream() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(2).put::<TreasuryPositions>();
			let quantities = PositionQuantities { bonds: 12, stakes: 2, fission_liquidity: 50 };
			let upstream = UpstreamPosition {
				vault_id: 3,
				bitcoin_securitization: 10,
				bitcoin_allocated_securitization: 45,
				bond_principal: 100,
			};
			let position_key = PositionsByAccount::<Test>::hashed_key_for(9);
			unhashed::put(
				&position_key,
				&PositionV2 { bond_principal: 300u128, quantities, upstream: Some(upstream) },
			);

			BackCollectMigration::<Test>::on_runtime_upgrade();
			assert_eq!(StorageVersion::get::<TreasuryPositions>(), StorageVersion::new(3));
			assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 0);
			assert_eq!(CollectTotals::<Test>::get(), CollectTotalsData::default());
			assert_eq!(
				PositionsByAccount::<Test>::get(9),
				Some(Position {
					bond_principal: 300,
					quantities,
					upstream: Some(upstream),
					collect_accrued: CollectAmounts::default(),
					collected: CollectAmounts::default(),
					earning_positions: EarningPositions {
						bonds: IndexedEarningsPosition { quantity: 12, ..Default::default() },
						stakes: IndexedEarningsPosition { quantity: 2, ..Default::default() },
						bitcoin_liquidity: IndexedEarningsPosition {
							quantity: 50,
							..Default::default()
						},
						..Default::default()
					},
					..Default::default()
				})
			);

			PositionsByAccount::<Test>::mutate(9, |position| {
				position.as_mut().unwrap().collected.stabilizer_pool_earnings = 7;
			});
			BackCollectMigration::<Test>::on_runtime_upgrade();
			assert_eq!(Balances::total_issuance(), 0);
			assert_eq!(
				PositionsByAccount::<Test>::get(9).unwrap().collected.stabilizer_pool_earnings,
				7
			);
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn upgrade_checks_preserve_released_positions_and_detect_mining_seat_drift() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(2).put::<TreasuryPositions>();
			ActiveMiningAccounts::set(vec![9, 9, 10]);
			unhashed::put(
				&PositionsByAccount::<Test>::hashed_key_for(9),
				&PositionV2 {
					bond_principal: 300u128,
					quantities: PositionQuantities { bonds: 12, stakes: 2, fission_liquidity: 50 },
					upstream: None,
				},
			);
			let snapshot = BackCollectMigration::<Test>::pre_upgrade().unwrap();
			assert_ok!(BackCollectMigration::<Test>::try_on_runtime_upgrade(true));
			PositionsByAccount::<Test>::mutate(9, |position| {
				position.as_mut().unwrap().mining_seats = 0
			});
			assert!(BackCollectMigration::<Test>::post_upgrade(snapshot).is_err());
		});
	}
}
