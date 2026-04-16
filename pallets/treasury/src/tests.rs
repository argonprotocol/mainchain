use super::{
	BondLot, BondLotById, BondLotIdsByAccount, BondLotsByVault, BondReleaseReason,
	CurrentFrameVaultCapital, HoldReason, PendingBondReleaseRetryCursor,
	PendingBondReleasesByFrame,
};
use crate::{
	mock::{
		Balances, BidPoolAccountId, CurrentFrameId, ExistentialDeposit, LastVaultProfits,
		MaxTreasuryContributors, MaxVaultsPerPool, MinimumArgonsPerContributor,
		RuntimeHoldReason, RuntimeOrigin, Test, TestVault, Treasury, TreasuryExitDelayFrames,
		TreasuryReservesAccountId, insert_vault, new_test_ext, reset_treasury_pool_participated,
		set_argons, take_treasury_pool_participated,
	},
	pallet::{BondLotAllocation, FrameVaultCapital, VaultCapital},
};
use argon_primitives::{MICROGONS_PER_ARGON, OperationalRewardsPayer, TreasuryPoolProvider};
use frame_support::{
	assert_err, assert_ok,
	traits::fungible::{Inspect, InspectHold},
};
use pallet_prelude::*;
use sp_runtime::{BoundedBTreeMap, FixedU128, Permill, TokenError};

fn account_bond_lot_ids(account_id: u64) -> Vec<u64> {
	BondLotIdsByAccount::<Test>::iter_key_prefix(account_id).collect()
}

#[test]
fn buy_bonds_creates_a_lot_and_tracks_pool_participation() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: (100 * MICROGONS_PER_ARGON) as u64,
				sharing_percent: Permill::from_percent(20),
				is_closed: false,
			},
		);
		set_argons(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(2), 1, 5));

		let bond_lot_ids = account_bond_lot_ids(2);
		assert_eq!(bond_lot_ids.len(), 1);

		let bond_lot = BondLotById::<Test>::get(bond_lot_ids[0]).expect("bond lot");
		assert_eq!(bond_lot.owner, 2);
		assert_eq!(bond_lot.vault_id, 1);
		assert_eq!(bond_lot.bonds, 5);
		assert_eq!(bond_lot.created_frame_id, 1);

		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&2
			),
			5 * MICROGONS_PER_ARGON,
		);
		assert_eq!(BondLotsByVault::<Test>::get(1).len(), 1);
		assert!(<Treasury as TreasuryPoolProvider<u64>>::has_bond_participation(1, &2));
	});
}

#[test]
fn liquidate_bond_lot_removes_it_from_future_frames_and_releases_on_maturity() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: (100 * MICROGONS_PER_ARGON) as u64,
				sharing_percent: Permill::from_percent(20),
				is_closed: false,
			},
		);
		set_argons(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(2), 1, 3));
		let bond_lot_id = account_bond_lot_ids(2)[0];

		assert_ok!(Treasury::liquidate_bond_lot(RuntimeOrigin::signed(2), bond_lot_id));

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("releasing bond lot");
		assert_eq!(bond_lot.release_reason, Some(BondReleaseReason::UserLiquidation));
		assert_eq!(bond_lot.release_frame_id, Some(11));
		assert!(BondLotsByVault::<Test>::get(1).is_empty());
		assert_eq!(PendingBondReleasesByFrame::<Test>::get(11), vec![bond_lot_id]);

		Treasury::release_pending_bond_lots(11);

		assert!(BondLotById::<Test>::get(bond_lot_id).is_none());
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&2
			),
			0,
		);
	});
}

#[test]
fn accepted_lots_are_ranked_only_by_size() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxTreasuryContributors::set(2);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: (100 * MICROGONS_PER_ARGON) as u64,
				sharing_percent: Permill::from_percent(20),
				is_closed: false,
			},
		);

		set_argons(10, 20 * MICROGONS_PER_ARGON);
		set_argons(2, 20 * MICROGONS_PER_ARGON);
		set_argons(3, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(10), 1, 1));
		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(2), 1, 5));
		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(3), 1, 6));

		let accepted_lots = BondLotsByVault::<Test>::get(1);
		assert_eq!(accepted_lots.len(), 2);

		let first_lot =
			BondLotById::<Test>::get(accepted_lots[0].bond_lot_id).expect("largest lot");
		let second_lot =
			BondLotById::<Test>::get(accepted_lots[1].bond_lot_id).expect("next largest lot");
		assert_eq!(first_lot.owner, 3);
		assert_eq!(first_lot.bonds, 6);
		assert_eq!(second_lot.owner, 2);
		assert_eq!(second_lot.bonds, 5);

		let bumped_lot_id = account_bond_lot_ids(10)[0];
		let bumped_lot = BondLotById::<Test>::get(bumped_lot_id).expect("bumped lot");
		assert_eq!(bumped_lot.release_reason, Some(BondReleaseReason::Bumped));
		assert_eq!(bumped_lot.release_frame_id, Some(11));
	});
}

#[test]
fn distribution_uses_frame_snapshot_payouts_and_refunds_underfill_to_treasury() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: (10 * MICROGONS_PER_ARGON) as u64,
				sharing_percent: Permill::from_percent(20),
				is_closed: false,
			},
		);

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(2), 1, 4));

		let bond_lot_id = account_bond_lot_ids(2)[0];
		let balance_before = Balances::balance(&2);

		Treasury::lock_in_vault_capital(1);

		let current = CurrentFrameVaultCapital::<Test>::get().expect("current frame capital");
		assert_eq!(current.frame_id, 1);
		let vault_capital = current.vaults.get(&1).expect("vault capital");
		assert_eq!(vault_capital.eligible_bonds, 4);
		assert_eq!(vault_capital.bond_lot_allocations.len(), 1);
		assert_eq!(
			vault_capital.bond_lot_allocations[0].prorata,
			FixedU128::from_rational(4u128, 10u128),
		);

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		assert!(CurrentFrameVaultCapital::<Test>::get().is_none());

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("paid bond lot");
		assert_eq!(bond_lot.participated_frames, 1);
		assert_eq!(bond_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(bond_lot.last_frame_earnings, Some(25_600_000));
		assert_eq!(bond_lot.cumulative_earnings, 25_600_000);
		assert_eq!(Balances::balance(&2), balance_before + 25_600_000);
		assert_eq!(Balances::balance(&TreasuryReservesAccountId::get()), 58_400_000);

		assert_eq!(LastVaultProfits::get().len(), 1);
		assert_eq!(LastVaultProfits::get()[0].vault_id, 1);
		assert_eq!(LastVaultProfits::get()[0].earnings, 80 * MICROGONS_PER_ARGON);
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, 16 * MICROGONS_PER_ARGON);
		assert_eq!(LastVaultProfits::get()[0].capital_contributed, 4 * MICROGONS_PER_ARGON);
		assert_eq!(LastVaultProfits::get()[0].capital_contributed_by_vault, 0);
	});
}

#[test]
fn lock_in_vault_capital_selects_top_vaults_by_eligible_bonds() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxVaultsPerPool::set(2);
		CurrentFrameId::set(1);

		insert_vault(
			1,
			TestVault {
				account_id: 11,
				securitized_satoshis: MICROGONS_PER_ARGON as u64,
				sharing_percent: Permill::from_percent(20),
				is_closed: false,
			},
		);
		insert_vault(
			2,
			TestVault {
				account_id: 12,
				securitized_satoshis: (2 * MICROGONS_PER_ARGON) as u64,
				sharing_percent: Permill::from_percent(20),
				is_closed: false,
			},
		);
		insert_vault(
			3,
			TestVault {
				account_id: 13,
				securitized_satoshis: (3 * MICROGONS_PER_ARGON) as u64,
				sharing_percent: Permill::from_percent(20),
				is_closed: false,
			},
		);

		set_argons(11, 10 * MICROGONS_PER_ARGON);
		set_argons(12, 10 * MICROGONS_PER_ARGON);
		set_argons(13, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(11), 1, 1));
		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(12), 2, 2));
		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(13), 3, 3));

		Treasury::lock_in_vault_capital(1);

		let current = CurrentFrameVaultCapital::<Test>::get().expect("current frame capital");
		assert_eq!(current.frame_id, 1);
		assert_eq!(current.vaults.len(), 2);
		assert!(current.vaults.get(&1).is_none());
		assert_eq!(current.vaults.get(&2).map(|vault| vault.eligible_bonds), Some(2));
		assert_eq!(current.vaults.get(&3).map(|vault| vault.eligible_bonds), Some(3));

		let participation = take_treasury_pool_participated();
		assert_eq!(
			participation,
			vec![(13, 3 * MICROGONS_PER_ARGON), (12, 2 * MICROGONS_PER_ARGON)]
		);
	});
}

#[test]
fn locked_frame_still_pays_after_lot_is_liquidated() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: (10 * MICROGONS_PER_ARGON) as u64,
				sharing_percent: Permill::from_percent(20),
				is_closed: false,
			},
		);

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(2), 1, 4));

		let bond_lot_id = account_bond_lot_ids(2)[0];
		let balance_before = Balances::balance(&2);

		Treasury::lock_in_vault_capital(1);
		assert_ok!(Treasury::liquidate_bond_lot(RuntimeOrigin::signed(2), bond_lot_id));
		assert!(BondLotsByVault::<Test>::get(1).is_empty());

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("liquidating bond lot");
		assert_eq!(bond_lot.release_reason, Some(BondReleaseReason::UserLiquidation));
		assert_eq!(bond_lot.participated_frames, 1);
		assert_eq!(bond_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(bond_lot.last_frame_earnings, Some(25_600_000));
		assert_eq!(bond_lot.cumulative_earnings, 25_600_000);
		assert_eq!(Balances::balance(&2), balance_before + 25_600_000);
	});
}

#[test]
fn failed_bond_lot_payout_is_refunded_to_treasury_and_not_recorded_as_earned() {
	new_test_ext().execute_with(|| {
		ExistentialDeposit::set(10);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: (10 * MICROGONS_PER_ARGON) as u64,
				sharing_percent: Permill::zero(),
				is_closed: false,
			},
		);

		BondLotById::<Test>::insert(
			0,
			BondLot {
				owner: 99,
				vault_id: 1,
				bonds: 1,
				created_frame_id: 1,
				participated_frames: 0,
				last_frame_earnings_frame_id: None,
				last_frame_earnings: None,
				cumulative_earnings: 0,
				release_frame_id: None,
				release_reason: None,
			},
		);
		let mut vaults = BoundedBTreeMap::<VaultId, VaultCapital<Test>, MaxVaultsPerPool>::new();
		assert!(
			vaults
				.try_insert(
					1,
					VaultCapital {
						bond_lot_allocations: BoundedVec::truncate_from(vec![BondLotAllocation {
							bond_lot_id: 0,
							prorata: FixedU128::one(),
						}]),
						eligible_bonds: 1,
						vault_sharing_percent: Permill::zero(),
					},
				)
				.is_ok()
		);
		CurrentFrameVaultCapital::<Test>::put(FrameVaultCapital { frame_id: 1, vaults });

		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 11));

		Treasury::distribute_bid_pool(1);

		let bond_lot = BondLotById::<Test>::get(0).expect("bond lot");
		assert_eq!(bond_lot.participated_frames, 1);
		assert_eq!(bond_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(bond_lot.last_frame_earnings, Some(0));
		assert_eq!(bond_lot.cumulative_earnings, 0);
		assert_eq!(Balances::balance(&99), 0);
		assert_eq!(Balances::balance(&TreasuryReservesAccountId::get()), 11);
	});
}

#[test]
fn run_frame_transition_releases_distributes_and_locks_without_paying_operational_rewards() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		TreasuryExitDelayFrames::set(1);
		CurrentFrameId::set(1);

		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: (10 * MICROGONS_PER_ARGON) as u64,
				sharing_percent: Permill::from_percent(20),
				is_closed: false,
			},
		);
		insert_vault(
			2,
			TestVault {
				account_id: 11,
				securitized_satoshis: (10 * MICROGONS_PER_ARGON) as u64,
				sharing_percent: Permill::from_percent(20),
				is_closed: false,
			},
		);

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(2), 1, 4));
		let payout_bond_lot_id = account_bond_lot_ids(2)[0];
		Treasury::lock_in_vault_capital(1);

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		set_argons(3, 20 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(3), 2, 2));
		let released_bond_lot_id = account_bond_lot_ids(3)[0];
		assert_ok!(Treasury::liquidate_bond_lot(RuntimeOrigin::signed(3), released_bond_lot_id,));
		set_argons(42, 0);

		Treasury::run_frame_transition(2);

		assert!(BondLotById::<Test>::get(released_bond_lot_id).is_none());
		assert!(account_bond_lot_ids(3).is_empty());
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&3,
			),
			0,
		);

		let current = CurrentFrameVaultCapital::<Test>::get().expect("current frame capital");
		assert_eq!(current.frame_id, 2);
		assert!(current.vaults.get(&2).is_none());
		assert_eq!(Balances::balance(&42), 0);
		assert_eq!(LastVaultProfits::get().len(), 1);
		assert_eq!(LastVaultProfits::get()[0].vault_id, 1);
		assert_eq!(LastVaultProfits::get()[0].capital_contributed, 4 * MICROGONS_PER_ARGON);

		let payout_lot = BondLotById::<Test>::get(payout_bond_lot_id).expect("payout lot");
		assert_eq!(payout_lot.participated_frames, 1);
		assert_eq!(payout_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(payout_lot.last_frame_earnings, Some(25_600_000));
	});
}

#[test]
fn claim_operational_reward_pays_immediately_when_funded() {
	new_test_ext().execute_with(|| {
		let reserves_account = TreasuryReservesAccountId::get();
		set_argons(reserves_account, 1_000_000);
		set_argons(42, 0);

		assert_ok!(<Treasury as OperationalRewardsPayer<u64, u128>>::claim_reward(&42, 250_000,));
		assert_eq!(Balances::balance(&42), 250_000);
		assert_eq!(Balances::balance(&reserves_account), 750_000);
	});
}

#[test]
fn claim_operational_reward_fails_when_insufficient() {
	new_test_ext().execute_with(|| {
		let reserves_account = TreasuryReservesAccountId::get();
		set_argons(reserves_account, 10);
		set_argons(42, 0);

		assert_err!(
			<Treasury as OperationalRewardsPayer<u64, u128>>::claim_reward(&42, 250),
			TokenError::FundsUnavailable
		);
		assert_eq!(Balances::balance(&42), 0);
	});
}

#[test]
fn failed_release_retries_and_does_not_block_current_frame_releases() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(1);

		BondLotById::<Test>::insert(
			0,
			BondLot {
				owner: 2,
				vault_id: 1,
				bonds: 1,
				created_frame_id: 1,
				participated_frames: 0,
				last_frame_earnings_frame_id: None,
				last_frame_earnings: None,
				cumulative_earnings: 0,
				release_frame_id: Some(11),
				release_reason: Some(BondReleaseReason::UserLiquidation),
			},
		);
		BondLotIdsByAccount::<Test>::insert(2, 0, ());
		PendingBondReleasesByFrame::<Test>::insert(11, BoundedVec::truncate_from(vec![0]));

		set_argons(3, MICROGONS_PER_ARGON);
		assert_ok!(Treasury::create_hold(&3, MICROGONS_PER_ARGON));
		BondLotById::<Test>::insert(
			1,
			BondLot {
				owner: 3,
				vault_id: 1,
				bonds: 1,
				created_frame_id: 1,
				participated_frames: 0,
				last_frame_earnings_frame_id: None,
				last_frame_earnings: None,
				cumulative_earnings: 0,
				release_frame_id: Some(12),
				release_reason: Some(BondReleaseReason::UserLiquidation),
			},
		);
		BondLotIdsByAccount::<Test>::insert(3, 1, ());
		PendingBondReleasesByFrame::<Test>::insert(12, BoundedVec::truncate_from(vec![1]));

		Treasury::release_pending_bond_lots(11);
		assert_eq!(PendingBondReleaseRetryCursor::<Test>::get(), Some(11));
		assert!(BondLotById::<Test>::get(0).is_some());
		assert_eq!(PendingBondReleasesByFrame::<Test>::get(11), vec![0]);

		Treasury::release_pending_bond_lots(12);
		assert_eq!(PendingBondReleaseRetryCursor::<Test>::get(), Some(11));
		assert!(BondLotById::<Test>::get(0).is_some());
		assert!(BondLotById::<Test>::get(1).is_none());
		assert_eq!(PendingBondReleasesByFrame::<Test>::get(11), vec![0]);
		assert!(PendingBondReleasesByFrame::<Test>::get(12).is_empty());
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&3,
			),
			0,
		);

		set_argons(2, MICROGONS_PER_ARGON);
		assert_ok!(Treasury::create_hold(&2, MICROGONS_PER_ARGON));
		Treasury::release_pending_bond_lots(13);

		assert_eq!(PendingBondReleaseRetryCursor::<Test>::get(), None);
		assert!(BondLotById::<Test>::get(0).is_none());
		assert!(PendingBondReleasesByFrame::<Test>::get(11).is_empty());
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&2,
			),
			0,
		);
	});
}
