use crate::{
	CollectAmounts, CollectTotals, Config, EarningPositions, EarningsIndexState, EarningsIndexes,
	Pallet, PendingCollectBacking, Position, PositionsByAccount,
};
use argon_primitives::{
	treasury::{
		PositionQuantities, TreasuryPositionProvider, TreasuryPositionProviderWeightInfo,
		UpstreamPosition,
	},
	MiningFrameTransitionProvider,
};
use frame_support::{storage::with_storage_layer, traits::OnRuntimeUpgrade};
use pallet_prelude::*;

#[derive(Encode, Decode)]
pub(crate) struct PositionV2<Balance: codec::Codec + MaxEncodedLen> {
	#[codec(compact)]
	pub bond_principal: Balance,
	pub quantities: PositionQuantities<Balance>,
	pub upstream: Option<UpstreamPosition<Balance>>,
}

#[derive(Encode, Decode)]
pub(crate) struct CollectAmountsV3<Balance: codec::Codec + MaxEncodedLen> {
	#[codec(compact)]
	pub bond_earnings: Balance,
	#[codec(compact)]
	pub stake_earnings: Balance,
	#[codec(compact)]
	pub bitcoin_earnings: Balance,
	#[codec(compact)]
	pub bitcoin_mints: Balance,
	#[codec(compact)]
	pub stabilizer_pool_earnings: Balance,
}

impl<Balance: codec::Codec + MaxEncodedLen + Default> From<CollectAmountsV3<Balance>>
	for CollectAmounts<Balance>
{
	fn from(previous: CollectAmountsV3<Balance>) -> Self {
		Self {
			bond_earnings: previous.bond_earnings,
			stake_earnings: previous.stake_earnings,
			bitcoin_earnings: previous.bitcoin_earnings,
			stabilizer_pool_earnings: previous.stabilizer_pool_earnings,
			..Default::default()
		}
	}
}

#[derive(Encode, Decode)]
pub(crate) struct PositionV3<Balance: codec::Codec + MaxEncodedLen> {
	#[codec(compact)]
	pub bond_principal: Balance,
	pub quantities: PositionQuantities<Balance>,
	pub upstream: Option<UpstreamPosition<Balance>>,
	pub collect_accrued: CollectAmountsV3<Balance>,
	pub collected: CollectAmountsV3<Balance>,
}

/// Preserve supported account layouts and back only their unpaid amounts at cutover.
pub struct BackCollectMigration<T>(PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for BackCollectMigration<T> {
	fn on_runtime_upgrade() -> Weight {
		let version = StorageVersion::get::<Pallet<T>>();
		if version != StorageVersion::new(2) && version != StorageVersion::new(3) {
			return T::DbWeight::get().reads(1);
		}

		let mut count = 0u64;
		let result: DispatchResult = with_storage_layer(|| {
			if version == StorageVersion::new(2) {
				PositionsByAccount::<T>::translate_values::<PositionV2<T::Balance>, _>(
					|previous| {
						Some(Position {
							bond_principal: previous.bond_principal,
							quantities: previous.quantities,
							upstream: previous.upstream,
							..Default::default()
						})
					},
				);
			} else {
				let mut has_mint_claim = false;
				PositionsByAccount::<T>::translate_values::<PositionV3<T::Balance>, _>(
					|previous| {
						has_mint_claim |= !previous.collect_accrued.bitcoin_mints.is_zero() ||
							!previous.collected.bitcoin_mints.is_zero();
						Some(Position {
							bond_principal: previous.bond_principal,
							quantities: previous.quantities,
							upstream: previous.upstream,
							collect_accrued: previous.collect_accrued.into(),
							collected: previous.collected.into(),
							..Default::default()
						})
					},
				);
				ensure!(
					!has_mint_claim,
					"Unsupported Bitcoin mint claim; preserve the account layout for recovery"
				);
			}

			EarningsIndexes::<T>::put(EarningsIndexState {
				start_frame: T::MiningFrameTransitionProvider::get_current_frame_id().max(1),
				..Default::default()
			});

			let mut unpaid = CollectAmounts::<T::Balance>::default();
			for (account, mut position) in PositionsByAccount::<T>::iter() {
				count.saturating_accrue(1);
				frame_system::Pallet::<T>::inc_providers(&account);
				unpaid.saturating_accrue(&position.collect_accrued);
				position.earning_positions = EarningPositions::default();
				position.earning_positions.bonds.quantity = position.quantities.bonds;
				position.earning_positions.stakes.quantity = position.quantities.stakes;
				position.earning_positions.bitcoin_liquidity.quantity =
					position.quantities.fission_liquidity.into();
				PositionsByAccount::<T>::insert(account, position);
			}

			if !unpaid.total().is_zero() {
				PendingCollectBacking::<T>::put(unpaid);
			}
			CollectTotals::<T>::put(
				argon_primitives::treasury::CollectTotals::<T::Balance>::default(),
			);
			StorageVersion::new(4).put::<Pallet<T>>();
			Ok(())
		});

		match result {
			Ok(()) =>
				if let Err(error) = with_storage_layer(Pallet::<T>::prepare_collect) {
					log::warn!("Collect custody initialization failed: {error:?}");
				},
			Err(error) => log::error!("Collect backing migration failed: {error:?}"),
		}

		T::DbWeight::get()
			.reads_writes(
				count.saturating_mul(3).saturating_add(5),
				count.saturating_mul(3).saturating_add(5),
			)
			.saturating_add(crate::ProviderWeightAdapter::<T>::fund())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, *};
	use frame_support::{assert_ok, storage::unhashed};
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
			assert_eq!(StorageVersion::get::<TreasuryPositions>(), StorageVersion::new(4));
			assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 10);
			assert_eq!(
				CollectTotals::<Test>::get(),
				argon_primitives::treasury::CollectTotals::default()
			);
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
				})
			);

			PositionsByAccount::<Test>::mutate(9, |position| {
				position.as_mut().unwrap().collected.stabilizer_pool_earnings = 7;
			});
			BackCollectMigration::<Test>::on_runtime_upgrade();
			assert_eq!(Balances::total_issuance(), 10);
			assert_eq!(
				PositionsByAccount::<Test>::get(9).unwrap().collected.stabilizer_pool_earnings,
				7
			);
		});
	}

	#[test]
	fn migration_backs_unpaid_v3_amounts_without_reminting_paid_history() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			StorageVersion::new(3).put::<TreasuryPositions>();
			let mut position = Position::<u128>::default();
			position.collect_accrued.stabilizer_pool_earnings = 20;
			position.collected.stabilizer_pool_earnings = 1_000;
			unhashed::put(
				&PositionsByAccount::<Test>::hashed_key_for(9),
				&PositionV3 {
					bond_principal: position.bond_principal,
					quantities: position.quantities,
					upstream: position.upstream,
					collect_accrued: CollectAmountsV3 {
						bond_earnings: 0,
						stake_earnings: 0,
						bitcoin_earnings: 0,
						bitcoin_mints: 0,
						stabilizer_pool_earnings: 20,
					},
					collected: CollectAmountsV3 {
						bond_earnings: 0,
						stake_earnings: 0,
						bitcoin_earnings: 0,
						bitcoin_mints: 0,
						stabilizer_pool_earnings: 1_000,
					},
				},
			);
			BackCollectMigration::<Test>::on_runtime_upgrade();
			assert_eq!(StorageVersion::get::<TreasuryPositions>(), StorageVersion::new(4));
			assert_eq!(Balances::total_issuance(), 30);
			assert_eq!(PositionsByAccount::<Test>::get(9), Some(position));
			assert_eq!(System::providers(&9), 1);
			BackCollectMigration::<Test>::on_runtime_upgrade();
			assert_eq!(System::providers(&9), 1);
			assert_eq!(Balances::total_issuance(), 30);
			TreasuryPositions::migrate_accrued(
				&11,
				CollectAmounts { stabilizer_buy_earnings: 17, ..Default::default() },
			);
			assert_eq!(PendingCollectBacking::<Test>::get().unwrap().stabilizer_buy_earnings, 17);
			assert_ok!(TreasuryPositions::fund(CollectSource::StabilizerPools, 100));
			assert_ok!(TreasuryPositions::accrue(&10, CollectSource::StabilizerPools, 100));
			assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(10)));
			assert_eq!(Balances::balance(&10), 100);
			assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 47);
			assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
			assert_eq!(Balances::balance(&9), 20);
			assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(11)));
			assert_eq!(Balances::balance(&11), 17);
			assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 10);
			assert_eq!(Balances::total_issuance(), 147);
			assert_eq!(
				PositionsByAccount::<Test>::get(9).unwrap().collected.stabilizer_pool_earnings,
				1_020
			);
		});
	}
}
