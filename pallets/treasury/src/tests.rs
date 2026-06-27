use super::{
	ArgonotBondLots, BondLot, BondLotById, BondLotIdsByAccount, BondLotSummary, BondLotsByVault,
	BondProgram, BondReleaseReason, CurrentFrameArgonotBondParticipants, CurrentFrameVaultCapital,
	HoldReason, PendingBondReleaseRetryCursor, PendingBondReleasesByFrame, TotalActiveArgonotBonds,
};
use crate::{
	mock::{
		account_id_from_seed, account_pair_from_seed, insert_vault, new_test_ext, set_argons,
		set_ownership, take_treasury_pool_participated, Balances, BidPoolAccountId, CurrentFrameId,
		ExistentialDeposit, LastVaultProfits, MaxArgonotBondHolders,
		MaxArgonotBondedPercentOfCirculation, MaxTreasuryContributors, MaxVaultsPerPool,
		MinimumArgonsPerContributor, Ownership, RuntimeEvent, RuntimeHoldReason, RuntimeOrigin,
		System, Test, TestAccountId, TestVault, Treasury, TreasuryExitDelayFrames,
		TreasuryReservesAccountId,
	},
	pallet::{BondLotAllocation, Error, FrameVaultCapital, VaultCapital},
};
use argon_primitives::{
	vault::{TreasuryBonusApprovalProof, TREASURY_BONUS_APPROVAL_PROOF_MESSAGE_KEY},
	OperationalRewardsPayer, Signature, TreasuryPoolProvider, MICROGONS_PER_ARGON,
};
use frame_support::{
	assert_err, assert_ok,
	traits::fungible::{Inspect, InspectHold},
};
use pallet_prelude::*;
use sp_core::{blake2_256, Pair};
use sp_runtime::{BoundedBTreeMap, FixedU128, Permill, TokenError};

fn account_bond_lot_ids(account_id: u64) -> Vec<u64> {
	BondLotIdsByAccount::<Test>::iter_key_prefix(account(account_id)).collect()
}

fn account(seed: u64) -> TestAccountId {
	account_id_from_seed(seed)
}

fn origin(seed: u64) -> RuntimeOrigin {
	RuntimeOrigin::signed(account(seed))
}

fn argonot_bond_lots() -> Vec<BondLotSummary> {
	ArgonotBondLots::<Test>::get().into_inner()
}

fn test_vault(account_id: u64, securitized_satoshis: u64, sharing_percent: Permill) -> TestVault {
	TestVault {
		account_id: account(account_id),
		securitized_satoshis,
		sharing_percent,
		bonus_percent: Permill::zero(),
		delegate_account_id: None,
		is_closed: false,
	}
}

fn bonus_approval(
	vault_id: u32,
	beneficiary: u64,
	expires_at_frame: FrameId,
) -> TreasuryBonusApprovalProof {
	let beneficiary = account(beneficiary);
	let message = (
		TREASURY_BONUS_APPROVAL_PROOF_MESSAGE_KEY,
		vault_id,
		beneficiary.clone(),
		expires_at_frame,
	)
		.using_encoded(blake2_256);
	let signature: Signature = account_pair_from_seed(10).sign(message.as_slice()).into();
	TreasuryBonusApprovalProof { vault_id, beneficiary, expires_at_frame, signature }
}

#[test]
fn buy_bonds_store_plain_and_bonus_terms_and_track_pool_participation() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let mut vault =
			test_vault(10, (100 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20));
		vault.bonus_percent = Permill::from_percent(15);
		insert_vault(1, vault);

		set_argons(2, 20 * MICROGONS_PER_ARGON);
		set_argons(3, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, None));
		assert_ok!(Treasury::buy_bonds(origin(3), 1, 5, Some(bonus_approval(1, 3, 1)),));

		let plain_bond_lot_ids = account_bond_lot_ids(2);
		assert_eq!(plain_bond_lot_ids.len(), 1);

		let bonus_bond_lot_ids = account_bond_lot_ids(3);
		assert_eq!(bonus_bond_lot_ids.len(), 1);

		let plain_bond_lot = BondLotById::<Test>::get(plain_bond_lot_ids[0]).expect("bond lot");
		assert_eq!(plain_bond_lot.owner, account(2));
		assert_eq!(
			plain_bond_lot.program,
			BondProgram::Vault {
				vault_id: 1,
				sharing_percent: Permill::from_percent(20),
				bonus_percent: Permill::zero(),
			},
		);
		assert_eq!(plain_bond_lot.bonds, 5);
		assert_eq!(plain_bond_lot.created_frame_id, 1);

		let bonus_bond_lot = BondLotById::<Test>::get(bonus_bond_lot_ids[0]).expect("bond lot");
		assert_eq!(bonus_bond_lot.owner, account(3));
		assert_eq!(
			bonus_bond_lot.program,
			BondProgram::Vault {
				vault_id: 1,
				sharing_percent: Permill::from_percent(20),
				bonus_percent: Permill::from_percent(15),
			},
		);
		assert_eq!(bonus_bond_lot.bonds, 5);
		assert_eq!(bonus_bond_lot.created_frame_id, 1);

		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(2)
			),
			5 * MICROGONS_PER_ARGON,
		);
		assert_eq!(BondLotsByVault::<Test>::get(1).len(), 2);
		assert!(<Treasury as TreasuryPoolProvider<TestAccountId>>::has_bond_participation(
			1,
			&account(2),
		));
		assert!(<Treasury as TreasuryPoolProvider<TestAccountId>>::has_bond_participation(
			1,
			&account(3),
		));
	});
}

#[test]
fn bonus_approval_rejects_existing_lot_wrong_vault_account_expiry_and_signature() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let mut vault =
			test_vault(10, (100 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20));
		vault.bonus_percent = Permill::from_percent(15);
		insert_vault(1, vault);

		set_argons(2, 20 * MICROGONS_PER_ARGON);
		set_argons(3, 20 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, Some(bonus_approval(1, 2, 1)),));

		assert_err!(
			Treasury::buy_bonds(origin(2), 1, 5, Some(bonus_approval(1, 2, 2))),
			Error::<Test>::BonusApprovalExistingBondLot,
		);

		assert_err!(
			Treasury::buy_bonds(origin(2), 1, 5, Some(bonus_approval(2, 2, 1)),),
			Error::<Test>::BonusApprovalWrongVault,
		);
		assert_err!(
			Treasury::buy_bonds(origin(2), 1, 5, Some(bonus_approval(1, 3, 1)),),
			Error::<Test>::BonusApprovalWrongAccount,
		);
		assert_err!(
			Treasury::buy_bonds(origin(2), 1, 5, Some(bonus_approval(1, 2, 0)),),
			Error::<Test>::BonusApprovalExpired,
		);

		let mut invalid_signature = bonus_approval(1, 3, 1);
		invalid_signature.signature = Signature::Sr25519([1; 64].into());
		assert_err!(
			Treasury::buy_bonds(origin(3), 1, 5, Some(invalid_signature)),
			Error::<Test>::InvalidBonusApprovalSignature,
		);
	});
}

#[test]
fn bonus_approval_rejects_reuse_while_lot_is_releasing() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let mut vault =
			test_vault(10, (100 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20));
		vault.bonus_percent = Permill::from_percent(15);
		insert_vault(1, vault);

		set_argons(2, 20 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, Some(bonus_approval(1, 2, 1)),));
		let bond_lot_id = account_bond_lot_ids(2)[0];

		assert_ok!(Treasury::liquidate_bond_lot(origin(2), bond_lot_id));
		CurrentFrameId::set(2);
		assert_err!(
			Treasury::buy_bonds(origin(2), 1, 5, Some(bonus_approval(1, 2, 2))),
			Error::<Test>::BonusApprovalExistingBondLot,
		);
	});
}

#[test]
fn liquidate_bond_lot_removes_it_from_future_frames_and_releases_on_maturity() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			test_vault(10, (100 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);
		set_argons(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 3, None));
		let bond_lot_id = account_bond_lot_ids(2)[0];

		assert_ok!(Treasury::liquidate_bond_lot(origin(2), bond_lot_id));

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
				&account(2)
			),
			0,
		);
	});
}

#[test]
fn liquidate_bond_lot_rejects_when_it_would_drop_below_encumbered_backing() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			test_vault(10, (100 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);
		set_argons(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 3, None));
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 2, None));
		let bond_lot_ids = account_bond_lot_ids(2);
		assert_ok!(<Treasury as TreasuryPoolProvider<TestAccountId>>::encumber_bond_microgons(
			&account(2),
			4 * MICROGONS_PER_ARGON,
		));

		assert_err!(
			Treasury::liquidate_bond_lot(origin(2), bond_lot_ids[0]),
			Error::<Test>::ActiveBondAmountBelowEncumberedBacking,
		);
		assert_eq!(BondLotsByVault::<Test>::get(1).len(), 2);
		assert_eq!(
			BondLotById::<Test>::get(bond_lot_ids[0]).expect("bond lot").release_reason,
			None,
		);
	});
}

#[test]
fn buy_argonot_bonds_allows_multiple_lots_per_account() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));
		set_ownership(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 5));
		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 4));

		let bond_lot_ids = account_bond_lot_ids(2);
		assert_eq!(bond_lot_ids.len(), 2);
		assert_eq!(argonot_bond_lots().len(), 2);
		assert_eq!(TotalActiveArgonotBonds::<Test>::get(), 9);
		assert_eq!(
			Ownership::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(2),
			),
			9 * MICROGONS_PER_ARGON,
		);
	});
}

#[test]
fn buy_argonot_bonds_rejects_when_full_queue_lot_does_not_beat_floor() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxArgonotBondHolders::set(2);
		MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));

		set_ownership(2, 20 * MICROGONS_PER_ARGON);
		set_ownership(3, 20 * MICROGONS_PER_ARGON);
		set_ownership(4, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 3));
		assert_ok!(Treasury::buy_argonot_bonds(origin(3), 5));

		assert_err!(
			Treasury::buy_argonot_bonds(origin(4), 3),
			Error::<Test>::ArgonotBondPurchaseBelowCutoff,
		);

		assert!(account_bond_lot_ids(4).is_empty());
		assert_eq!(argonot_bond_lots().len(), 2);
		assert_eq!(TotalActiveArgonotBonds::<Test>::get(), 8);
	});
}

#[test]
fn buy_argonot_bonds_evicts_floor_and_schedules_release() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxArgonotBondHolders::set(2);
		MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));

		set_ownership(2, 20 * MICROGONS_PER_ARGON);
		set_ownership(3, 20 * MICROGONS_PER_ARGON);
		set_ownership(4, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 3));
		assert_ok!(Treasury::buy_argonot_bonds(origin(3), 5));
		let evicted_bond_lot_id = account_bond_lot_ids(2)[0];
		let retained_bond_lot_id = account_bond_lot_ids(3)[0];

		assert_ok!(Treasury::buy_argonot_bonds(origin(4), 6));
		let new_bond_lot_id = account_bond_lot_ids(4)[0];

		let evicted_bond_lot = BondLotById::<Test>::get(evicted_bond_lot_id).expect("bond lot");
		assert_eq!(evicted_bond_lot.release_reason, Some(BondReleaseReason::Bumped));
		assert_eq!(evicted_bond_lot.release_frame_id, Some(11));
		assert_eq!(argonot_bond_lots().len(), 2);
		assert_eq!(TotalActiveArgonotBonds::<Test>::get(), 11);
		assert_eq!(
			Ownership::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(2),
			),
			3 * MICROGONS_PER_ARGON,
		);
		assert_eq!(
			argonot_bond_lots(),
			vec![
				BondLotSummary { bond_lot_id: retained_bond_lot_id, bonds: 5 },
				BondLotSummary { bond_lot_id: new_bond_lot_id, bonds: 6 },
			],
		);
	});
}

#[test]
fn buy_argonot_bonds_enforces_circulation_cap() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(40));
		set_ownership(2, 10 * MICROGONS_PER_ARGON);

		assert_err!(
			Treasury::buy_argonot_bonds(origin(2), 5),
			Error::<Test>::ArgonotBondPurchaseAboveCap,
		);

		assert!(account_bond_lot_ids(2).is_empty());
		assert!(argonot_bond_lots().is_empty());
		assert_eq!(TotalActiveArgonotBonds::<Test>::get(), 0);
	});
}

#[test]
fn liquidate_argonot_bond_lot_removes_queue_entry_and_releases_ownership_on_maturity() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));
		set_ownership(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 3));
		let bond_lot_id = account_bond_lot_ids(2)[0];

		assert_ok!(Treasury::liquidate_bond_lot(origin(2), bond_lot_id));

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("releasing bond lot");
		assert_eq!(bond_lot.release_reason, Some(BondReleaseReason::UserLiquidation));
		assert_eq!(bond_lot.release_frame_id, Some(11));
		assert!(argonot_bond_lots().is_empty());
		assert_eq!(TotalActiveArgonotBonds::<Test>::get(), 0);
		assert_eq!(
			Ownership::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(2),
			),
			3 * MICROGONS_PER_ARGON,
		);

		Treasury::release_pending_bond_lots(11);

		assert!(BondLotById::<Test>::get(bond_lot_id).is_none());
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(
			Ownership::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(2),
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
			test_vault(10, (100 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);

		set_argons(10, 20 * MICROGONS_PER_ARGON);
		set_argons(2, 20 * MICROGONS_PER_ARGON);
		set_argons(3, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(10), 1, 1, None));
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, None));
		assert_ok!(Treasury::buy_bonds(origin(3), 1, 6, None));

		let accepted_lots = BondLotsByVault::<Test>::get(1);
		assert_eq!(accepted_lots.len(), 2);

		let first_lot =
			BondLotById::<Test>::get(accepted_lots[0].bond_lot_id).expect("largest lot");
		let second_lot =
			BondLotById::<Test>::get(accepted_lots[1].bond_lot_id).expect("next largest lot");
		assert_eq!(first_lot.owner, account(3));
		assert_eq!(first_lot.bonds, 6);
		assert_eq!(second_lot.owner, account(2));
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
		System::set_block_number(1);
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			test_vault(10, (10 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));

		let bond_lot_id = account_bond_lot_ids(2)[0];
		let balance_before = Balances::balance(&account(2));

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
		assert!(System::events().iter().any(|record| {
			matches!(
				&record.event,
				RuntimeEvent::Treasury(crate::Event::FrameEarningsDistributed {
					frame_id,
					bid_pool_distributed,
					argonot_bond_pool_distributed,
					vault_bid_pool_distributed,
					treasury_refunds,
					treasury_reserves,
					participating_vaults,
				}) if *frame_id == 1
					&& *bid_pool_distributed == 80 * MICROGONS_PER_ARGON
					&& *argonot_bond_pool_distributed == 0
					&& *vault_bid_pool_distributed == 80 * MICROGONS_PER_ARGON
					&& *treasury_refunds == 48 * MICROGONS_PER_ARGON
					&& *treasury_reserves == 68 * MICROGONS_PER_ARGON
					&& *participating_vaults == 1
			)
		}));

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("paid bond lot");
		assert_eq!(bond_lot.participated_frames, 1);
		assert_eq!(bond_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(bond_lot.last_frame_earnings, Some(6_400_000));
		assert_eq!(bond_lot.cumulative_earnings, 6_400_000);
		assert_eq!(Balances::balance(&account(2)), balance_before + 6_400_000);
		assert_eq!(Balances::balance(&TreasuryReservesAccountId::get()), 68_000_000);

		assert_eq!(LastVaultProfits::get().len(), 1);
		assert_eq!(LastVaultProfits::get()[0].vault_id, 1);
		assert_eq!(LastVaultProfits::get()[0].earnings, 80 * MICROGONS_PER_ARGON);
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, 25_600_000);
		assert_eq!(LastVaultProfits::get()[0].capital_contributed, 4 * MICROGONS_PER_ARGON);
		assert_eq!(LastVaultProfits::get()[0].capital_contributed_by_vault, 0);
	});
}

#[test]
fn argonot_bonds_are_paid_before_vault_bonds() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		insert_vault(
			1,
			test_vault(10, (4 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);
		set_argons(2, 50 * MICROGONS_PER_ARGON);
		set_ownership(3, 50 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));
		assert_ok!(Treasury::buy_argonot_bonds(origin(3), 5));

		let vault_bond_lot_id = account_bond_lot_ids(2)[0];
		let argonot_bond_lot_id = account_bond_lot_ids(3)[0];

		Treasury::lock_in_argonot_bond_participants(1);
		Treasury::lock_in_vault_capital(1);

		let argonot_participants =
			CurrentFrameArgonotBondParticipants::<Test>::get().expect("argonot participants");
		assert_eq!(argonot_participants.frame_id, 1);
		assert_eq!(argonot_participants.total_bonds, 5);
		assert_eq!(argonot_participants.bond_lots.len(), 1);

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		assert!(CurrentFrameArgonotBondParticipants::<Test>::get().is_none());
		assert!(CurrentFrameVaultCapital::<Test>::get().is_none());
		assert!(System::events().iter().any(|record| {
			matches!(
				&record.event,
				RuntimeEvent::Treasury(crate::Event::FrameEarningsDistributed {
					frame_id,
					bid_pool_distributed,
					argonot_bond_pool_distributed,
					vault_bid_pool_distributed,
					treasury_refunds,
					treasury_reserves,
					participating_vaults,
				}) if *frame_id == 1
					&& *bid_pool_distributed == 80 * MICROGONS_PER_ARGON
					&& *argonot_bond_pool_distributed == 10 * MICROGONS_PER_ARGON
					&& *vault_bid_pool_distributed == 70 * MICROGONS_PER_ARGON
					&& *treasury_refunds == 0
					&& *treasury_reserves == 20 * MICROGONS_PER_ARGON
					&& *participating_vaults == 1
			)
		}));

		let argonot_bond_lot = BondLotById::<Test>::get(argonot_bond_lot_id).expect("argonot lot");
		assert_eq!(argonot_bond_lot.last_frame_earnings, Some(10_000_000));
		assert_eq!(argonot_bond_lot.cumulative_earnings, 10_000_000);
		assert_eq!(Balances::balance(&account(3)), 10_000_000);

		let vault_bond_lot = BondLotById::<Test>::get(vault_bond_lot_id).expect("vault lot");
		assert_eq!(vault_bond_lot.last_frame_earnings, Some(14_000_000));
		assert_eq!(vault_bond_lot.cumulative_earnings, 14_000_000);

		assert_eq!(Balances::balance(&TreasuryReservesAccountId::get()), 20_000_000);
		assert_eq!(LastVaultProfits::get().len(), 1);
		assert_eq!(LastVaultProfits::get()[0].earnings, 70 * MICROGONS_PER_ARGON);
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, 56_000_000);
	});
}

#[test]
fn bonus_backed_lots_increase_bonder_payout_and_reduce_vault_remainder() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let mut vault =
			test_vault(10, (20 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20));
		vault.bonus_percent = Permill::from_percent(10);
		insert_vault(1, vault);

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		set_argons(3, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));
		assert_ok!(Treasury::buy_bonds(origin(3), 1, 4, Some(bonus_approval(1, 3, 1)),));

		let plain_lot_id = account_bond_lot_ids(2)[0];
		let bonus_lot_id = account_bond_lot_ids(3)[0];

		Treasury::lock_in_vault_capital(1);

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		let plain_lot = BondLotById::<Test>::get(plain_lot_id).expect("plain bond lot");
		let bonus_lot = BondLotById::<Test>::get(bonus_lot_id).expect("bonus bond lot");
		assert_eq!(plain_lot.last_frame_earnings, Some(3_200_000));
		assert_eq!(bonus_lot.last_frame_earnings, Some(4_800_000));
		assert!(bonus_lot.cumulative_earnings > plain_lot.cumulative_earnings);
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, 24_000_000);
	});
}

#[test]
fn locked_argonot_participants_still_pay_after_lot_is_liquidated() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		set_ownership(2, 50 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 5));

		let bond_lot_id = account_bond_lot_ids(2)[0];
		Treasury::lock_in_argonot_bond_participants(1);

		assert_ok!(Treasury::liquidate_bond_lot(origin(2), bond_lot_id));
		assert!(ArgonotBondLots::<Test>::get().is_empty());

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("liquidating argonot lot");
		assert_eq!(bond_lot.release_reason, Some(BondReleaseReason::UserLiquidation));
		assert_eq!(bond_lot.participated_frames, 1);
		assert_eq!(bond_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(bond_lot.last_frame_earnings, Some(10_000_000));
		assert_eq!(bond_lot.cumulative_earnings, 10_000_000);
		assert_eq!(Balances::balance(&account(2)), 10_000_000);
	});
}

#[test]
fn lock_in_vault_capital_selects_top_vaults_by_eligible_bonds() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxVaultsPerPool::set(2);
		CurrentFrameId::set(1);

		insert_vault(1, test_vault(11, MICROGONS_PER_ARGON as u64, Permill::from_percent(20)));
		insert_vault(
			2,
			test_vault(12, (2 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);
		insert_vault(
			3,
			test_vault(13, (3 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);

		set_argons(11, 10 * MICROGONS_PER_ARGON);
		set_argons(12, 10 * MICROGONS_PER_ARGON);
		set_argons(13, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(11), 1, 1, None));
		assert_ok!(Treasury::buy_bonds(origin(12), 2, 2, None));
		assert_ok!(Treasury::buy_bonds(origin(13), 3, 3, None));

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
			vec![(account(13), 3 * MICROGONS_PER_ARGON), (account(12), 2 * MICROGONS_PER_ARGON),]
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
			test_vault(10, (10 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));

		let bond_lot_id = account_bond_lot_ids(2)[0];
		let balance_before = Balances::balance(&account(2));

		Treasury::lock_in_vault_capital(1);
		assert_ok!(Treasury::liquidate_bond_lot(origin(2), bond_lot_id));
		assert!(BondLotsByVault::<Test>::get(1).is_empty());

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("liquidating bond lot");
		assert_eq!(bond_lot.release_reason, Some(BondReleaseReason::UserLiquidation));
		assert_eq!(bond_lot.participated_frames, 1);
		assert_eq!(bond_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(bond_lot.last_frame_earnings, Some(6_400_000));
		assert_eq!(bond_lot.cumulative_earnings, 6_400_000);
		assert_eq!(Balances::balance(&account(2)), balance_before + 6_400_000);
	});
}

#[test]
fn locked_frame_skips_lot_after_it_is_fully_burned() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			test_vault(10, (10 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));

		let bond_lot_id = account_bond_lot_ids(2)[0];
		let balance_before = Balances::balance(&account(2));

		Treasury::lock_in_vault_capital(1);
		assert_ok!(<Treasury as TreasuryPoolProvider<TestAccountId>>::encumber_bond_microgons(
			&account(2),
			4 * MICROGONS_PER_ARGON,
		));
		assert_ok!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::burn_encumbered_bond_microgons(
				&account(2),
				4 * MICROGONS_PER_ARGON,
			)
		);
		assert!(BondLotsByVault::<Test>::get(1).is_empty());
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(Treasury::encumbered_bond_microgons(&account(2)), 0);
		assert!(BondLotById::<Test>::get(bond_lot_id).is_none());

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		assert_eq!(Balances::balance(&account(2)), balance_before);
	});
}

#[test]
fn failed_bond_lot_payout_is_not_recorded_as_earned() {
	new_test_ext().execute_with(|| {
		ExistentialDeposit::set(10);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64, Permill::zero()));

		BondLotById::<Test>::insert(
			0,
			BondLot {
				owner: account(99),
				program: BondProgram::Vault {
					vault_id: 1,
					sharing_percent: Permill::one(),
					bonus_percent: Permill::zero(),
				},
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
		assert!(vaults
			.try_insert(
				1,
				VaultCapital {
					bond_lot_allocations: BoundedVec::truncate_from(vec![BondLotAllocation {
						bond_lot_id: 0,
						prorata: FixedU128::one(),
					}]),
					eligible_bonds: 1,
				},
			)
			.is_ok());
		CurrentFrameVaultCapital::<Test>::put(FrameVaultCapital { frame_id: 1, vaults });

		frame_system::Pallet::<Test>::inc_providers(&BidPoolAccountId::get());
		set_argons(BidPoolAccountId::get(), 9);

		Treasury::distribute_bid_pool(1);

		let bond_lot = BondLotById::<Test>::get(0).expect("bond lot");
		assert_eq!(bond_lot.participated_frames, 1);
		assert_eq!(bond_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(bond_lot.last_frame_earnings, Some(0));
		assert_eq!(bond_lot.cumulative_earnings, 0);
		assert_eq!(Balances::balance(&account(99)), 0);
		assert_eq!(Balances::balance(&TreasuryReservesAccountId::get()), 0);
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
			test_vault(10, (10 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);
		insert_vault(
			2,
			test_vault(11, (10 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));
		let payout_bond_lot_id = account_bond_lot_ids(2)[0];
		Treasury::lock_in_vault_capital(1);

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		set_argons(3, 20 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(3), 2, 2, None));
		let released_bond_lot_id = account_bond_lot_ids(3)[0];
		assert_ok!(Treasury::liquidate_bond_lot(origin(3), released_bond_lot_id,));
		set_argons(42, 0);

		Treasury::run_frame_transition(2);

		assert!(BondLotById::<Test>::get(released_bond_lot_id).is_none());
		assert!(account_bond_lot_ids(3).is_empty());
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(3),
			),
			0,
		);

		let current = CurrentFrameVaultCapital::<Test>::get().expect("current frame capital");
		assert_eq!(current.frame_id, 2);
		assert!(current.vaults.get(&2).is_none());
		assert_eq!(Balances::balance(&account(42)), 0);
		assert_eq!(LastVaultProfits::get().len(), 1);
		assert_eq!(LastVaultProfits::get()[0].vault_id, 1);
		assert_eq!(LastVaultProfits::get()[0].capital_contributed, 4 * MICROGONS_PER_ARGON);

		let payout_lot = BondLotById::<Test>::get(payout_bond_lot_id).expect("payout lot");
		assert_eq!(payout_lot.participated_frames, 1);
		assert_eq!(payout_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(payout_lot.last_frame_earnings, Some(6_400_000));
	});
}

#[test]
fn claim_operational_reward_pays_immediately_when_funded() {
	new_test_ext().execute_with(|| {
		let reserves_account = TreasuryReservesAccountId::get();
		set_argons(&reserves_account, 1_000_000);
		set_argons(42, 0);

		assert_ok!(<Treasury as OperationalRewardsPayer<TestAccountId, u128>>::claim_reward(
			&account(42),
			250_000,
		));
		assert_eq!(Balances::balance(&account(42)), 250_000);
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
			<Treasury as OperationalRewardsPayer<TestAccountId, u128>>::claim_reward(
				&account(42),
				250,
			),
			TokenError::FundsUnavailable
		);
		assert_eq!(Balances::balance(&account(42)), 0);
	});
}

#[test]
fn burn_encumbered_bond_microgons_releases_fractional_slack_when_whole_bonds_still_cover_backing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			test_vault(10, (100 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);
		set_argons(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));
		assert_ok!(<Treasury as TreasuryPoolProvider<TestAccountId>>::encumber_bond_microgons(
			&account(2),
			5 * MICROGONS_PER_ARGON,
		));
		assert_ok!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::burn_encumbered_bond_microgons(
				&account(2),
				(3 * MICROGONS_PER_ARGON) / 2,
			)
		);

		let bond_lot_id = account_bond_lot_ids(2)[0];
		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("bond lot");
		assert_eq!(bond_lot.bonds, 8);
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(2),
			),
			8 * MICROGONS_PER_ARGON,
		);
		assert_eq!(Treasury::encumbered_bond_microgons(&account(2)), (7 * MICROGONS_PER_ARGON) / 2,);
		assert!(System::events().iter().any(|record| match &record.event {
			RuntimeEvent::Treasury(crate::Event::EncumberedBondMicrogonsBurned {
				account_id,
				burned_amount: amount,
				released_amount,
			}) => {
				*account_id == account(2) &&
					*amount == (3 * MICROGONS_PER_ARGON) / 2 &&
					*released_amount == MICROGONS_PER_ARGON / 2
			},
			_ => false,
		}));
	});
}

#[test]
fn burn_encumbered_bond_microgons_keeps_fractional_remainder_held_until_it_is_released() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			test_vault(10, (100 * MICROGONS_PER_ARGON) as u64, Permill::from_percent(20)),
		);
		set_argons(2, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, None));
		assert_ok!(<Treasury as TreasuryPoolProvider<TestAccountId>>::encumber_bond_microgons(
			&account(2),
			5 * MICROGONS_PER_ARGON,
		));
		assert_ok!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::burn_encumbered_bond_microgons(
				&account(2),
				(9 * MICROGONS_PER_ARGON) / 2,
			)
		);

		assert!(BondLotsByVault::<Test>::get(1).is_empty());
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(2),
			),
			MICROGONS_PER_ARGON / 2,
		);
		assert_eq!(Treasury::encumbered_bond_microgons(&account(2)), MICROGONS_PER_ARGON / 2);

		assert_err!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::release_encumbered_bond_microgons(
				&account(2),
				(MICROGONS_PER_ARGON / 2) + 1,
			),
			Error::<Test>::ActiveBondAmountBelowEncumberedBacking,
		);
		assert_ok!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::release_encumbered_bond_microgons(
				&account(2),
				MICROGONS_PER_ARGON / 2,
			)
		);
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(2),
			),
			0,
		);
		assert_eq!(Treasury::encumbered_bond_microgons(&account(2)), 0);
	});
}

#[test]
fn failed_release_retries_and_does_not_block_current_frame_releases() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(1);

		BondLotById::<Test>::insert(
			0,
			BondLot {
				owner: account(2),
				program: BondProgram::Vault {
					vault_id: 1,
					sharing_percent: Permill::zero(),
					bonus_percent: Permill::zero(),
				},
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
		BondLotIdsByAccount::<Test>::insert(account(2), 0, ());
		PendingBondReleasesByFrame::<Test>::insert(11, BoundedVec::truncate_from(vec![0]));

		set_argons(3, MICROGONS_PER_ARGON);
		assert_ok!(Treasury::create_hold::<Balances>(&account(3), MICROGONS_PER_ARGON));
		BondLotById::<Test>::insert(
			1,
			BondLot {
				owner: account(3),
				program: BondProgram::Vault {
					vault_id: 1,
					sharing_percent: Permill::zero(),
					bonus_percent: Permill::zero(),
				},
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
		BondLotIdsByAccount::<Test>::insert(account(3), 1, ());
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
				&account(3),
			),
			0,
		);

		set_argons(2, MICROGONS_PER_ARGON);
		assert_ok!(Treasury::create_hold::<Balances>(&account(2), MICROGONS_PER_ARGON));
		Treasury::release_pending_bond_lots(13);

		assert_eq!(PendingBondReleaseRetryCursor::<Test>::get(), None);
		assert!(BondLotById::<Test>::get(0).is_none());
		assert!(PendingBondReleasesByFrame::<Test>::get(11).is_empty());
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(2),
			),
			0,
		);
	});
}
