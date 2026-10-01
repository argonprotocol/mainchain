use super::{
	ArgonotBondLots, BondLot, BondLotById, BondLotIdsByAccount, BondLotIdsByVault, BondLotSummary,
	BondLotsByVault, BondProgram, BondProgramId, BondReleaseReason,
	CurrentFrameArgonotBondParticipants, CurrentFrameVaultCapital, HoldReason,
	PendingBondReleaseRetryCursor, PendingBondReleasesByFrame, TotalActiveArgonotBonds,
	TotalArgonBondLots,
};
use crate::{
	mock::{
		account_id_from_seed, account_pair_from_seed, insert_vault, new_test_ext, set_argons,
		set_ownership, ArgonotPriceInUsd, AverageArgonotPriceInMicrogons, Balances,
		BidPoolAccountId, CurrentFrameId, ExistentialDeposit, LastAverageArgonotPriceFrame,
		LastOperationalBondTotal, LastVaultProfits, MaxActiveArgonotBondLots, MaxArgonBondLots,
		MaxArgonotBondedPercentOfCirculation, MaxVaultsPerPool, MinimumArgonsPerContributor,
		MintedBitcoinMicrogons, Ownership, RuntimeEvent, RuntimeHoldReason, RuntimeOrigin, System,
		Test, TestAccountId, TestVault, Treasury, TreasuryExitDelayFrames,
		TreasuryReservesAccountId, VaultArgonotMicronots, VaultBitcoinSatoshis,
		VaultRewardCommittedMicronots, VaultsById,
	},
	pallet::{BondLotEarningsMetrics, Bonds, Error, FrameVaultCapital, TargetBitcoinPercent},
};
use argon_primitives::{
	vault::{TreasuryBonusApprovalProof, TREASURY_BONUS_APPROVAL_PROOF_MESSAGE_KEY},
	OperationalRewardsPayer, Signature, TreasuryPoolProvider, MICROGONS_PER_ARGON,
};
use frame_support::{
	assert_err, assert_ok,
	traits::fungible::{Inspect, InspectHold, Unbalanced},
};
use pallet_prelude::*;
use sp_core::{blake2_256, Pair};
use sp_runtime::{ArithmeticError, BoundedBTreeMap, FixedU128, Permill, TokenError};

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

#[test]
fn flexible_bond_lots_are_not_capped_per_vault() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(1, (26 * MICROGONS_PER_ARGON) as u64));
		set_argons(1, 26 * MICROGONS_PER_ARGON);
		for _ in 0..26 {
			assert_ok!(Treasury::buy_bonds(origin(1), 1, 1, None));
		}
		let lot_ids = account_bond_lot_ids(1);
		for lot_id in &lot_ids {
			assert_ok!(Treasury::set_bond_lot_flexible(origin(1), *lot_id, true));
		}
		assert_eq!(BondLotsByVault::<Test>::get(1).flexible_bonds, 26);
		assert!(BondLotById::<Test>::get(lot_ids[25]).unwrap().is_flexible);
	});
}

fn test_vault(account_id: u64, securitization: u64) -> TestVault {
	TestVault {
		account_id: account(account_id),
		securitization: securitization as Balance,
		exit_notice_amount: 0,
		committed_microgons: 0,
		activated_securitization: 0,
		delegate_account_id: None,
		is_closed: false,
	}
}

fn set_target_securitization(amount: Balance) {
	TargetBitcoinPercent::<Test>::put(Percent::from_percent(100));
	MintedBitcoinMicrogons::set(0);
	Balances::set_total_issuance(amount);
}

#[test]
fn root_configures_target_bitcoin_percent_without_resetting_omitted_values() {
	new_test_ext().execute_with(|| {
		assert_eq!(Treasury::target_bitcoin_percent(), Percent::from_percent(15));
		assert_noop!(
			Treasury::configure_reward_economics(origin(1), Some(Percent::from_percent(20)),),
			sp_runtime::DispatchError::BadOrigin,
		);

		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			Some(Percent::from_percent(20)),
		));
		assert_ok!(Treasury::configure_reward_economics(RuntimeOrigin::root(), None));
		assert_eq!(Treasury::target_bitcoin_percent(), Percent::from_percent(20));
	});
}

#[test]
fn root_backfills_existing_vault_lot_metrics_without_moving_funds() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 3, None));
		let bond_lot_id = account_bond_lot_ids(2)[0];
		CurrentFrameId::set(10);
		let initial = BondLotEarningsMetrics {
			participated_frames: 0,
			last_frame_earnings_frame_id: None,
			last_frame_earnings: None,
			cumulative_earnings: 0,
		};
		let backfilled = BondLotEarningsMetrics {
			participated_frames: 2,
			last_frame_earnings_frame_id: Some(3),
			last_frame_earnings: Some(200_000),
			cumulative_earnings: 500_000,
		};
		let free_balance = Balances::free_balance(account(2));
		let total_issuance = Balances::total_issuance();
		let held_balance = Balances::balance_on_hold(
			&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
			&account(2),
		);

		assert_noop!(
			Treasury::backfill_bond_lot_earnings(
				origin(2),
				bond_lot_id,
				initial.clone(),
				backfilled.clone(),
			),
			sp_runtime::DispatchError::BadOrigin,
		);
		assert_ok!(Treasury::backfill_bond_lot_earnings(
			RuntimeOrigin::root(),
			bond_lot_id,
			initial.clone(),
			backfilled.clone(),
		));
		let lot = BondLotById::<Test>::get(bond_lot_id).unwrap();
		assert_eq!(lot.participated_frames, backfilled.participated_frames);
		assert_eq!(lot.last_frame_earnings_frame_id, backfilled.last_frame_earnings_frame_id);
		assert_eq!(lot.last_frame_earnings, backfilled.last_frame_earnings);
		assert_eq!(lot.cumulative_earnings, backfilled.cumulative_earnings);
		assert_eq!(Balances::free_balance(account(2)), free_balance);
		assert_eq!(Balances::total_issuance(), total_issuance);
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(2),
			),
			held_balance,
		);
		let event_count = System::events().len();
		assert_ok!(Treasury::backfill_bond_lot_earnings(
			RuntimeOrigin::root(),
			bond_lot_id,
			initial.clone(),
			backfilled.clone(),
		));
		assert_eq!(System::events().len(), event_count);
		assert_noop!(
			Treasury::backfill_bond_lot_earnings(
				RuntimeOrigin::root(),
				bond_lot_id,
				backfilled.clone(),
				BondLotEarningsMetrics { cumulative_earnings: 400_000, ..backfilled.clone() },
			),
			Error::<Test>::InvalidBondLotMetrics,
		);
		assert_noop!(
			Treasury::backfill_bond_lot_earnings(
				RuntimeOrigin::root(),
				bond_lot_id,
				backfilled.clone(),
				BondLotEarningsMetrics {
					last_frame_earnings_frame_id: Some(11),
					..backfilled.clone()
				},
			),
			Error::<Test>::InvalidBondLotMetrics,
		);
		assert_noop!(
			Treasury::backfill_bond_lot_earnings(
				RuntimeOrigin::root(),
				bond_lot_id,
				initial,
				BondLotEarningsMetrics { cumulative_earnings: 600_000, ..backfilled },
			),
			Error::<Test>::BondLotMetricsChanged,
		);
	});
}

#[test]
fn vault_snapshot_uses_raw_securitization_and_the_effective_bitcoin_target() {
	new_test_ext().execute_with(|| {
		MintedBitcoinMicrogons::set(200 * MICROGONS_PER_ARGON);
		Balances::set_total_issuance(1_200 * MICROGONS_PER_ARGON);

		let mut idle_vault = test_vault(10, 0);
		idle_vault.securitization = 75 * MICROGONS_PER_ARGON;
		insert_vault(1, idle_vault);

		Treasury::lock_in_vault_capital(1);

		let snapshot = CurrentFrameVaultCapital::<Test>::get().expect("frame capital");
		assert_eq!(snapshot.frame_id, 1);
		assert_eq!(snapshot.target_securitization, 200 * MICROGONS_PER_ARGON);
		assert_eq!(snapshot.total_securitization, 75 * MICROGONS_PER_ARGON);
		assert_eq!(
			snapshot
				.vault_securitization_positions
				.get(&1)
				.map(|entry| entry.securitization),
			Some(75 * MICROGONS_PER_ARGON)
		);

		MintedBitcoinMicrogons::set(100 * MICROGONS_PER_ARGON);
		Balances::set_total_issuance(1_100 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(2);
		let snapshot = CurrentFrameVaultCapital::<Test>::get().expect("frame capital");
		assert_eq!(snapshot.target_securitization, 150 * MICROGONS_PER_ARGON);
	});
}

fn bonus_approval(
	vault_id: u32,
	beneficiary: u64,
	bonus_percent: Permill,
	expires_at_frame: FrameId,
	bond_space_to_unreserve: Bonds,
	nonce: u64,
) -> TreasuryBonusApprovalProof {
	let beneficiary = account(beneficiary);
	let message = (
		TREASURY_BONUS_APPROVAL_PROOF_MESSAGE_KEY,
		vault_id,
		beneficiary.clone(),
		bonus_percent,
		expires_at_frame,
		bond_space_to_unreserve,
		nonce,
	)
		.using_encoded(blake2_256);
	let signature: Signature = account_pair_from_seed(10).sign(message.as_slice()).into();
	TreasuryBonusApprovalProof {
		vault_id,
		beneficiary,
		bonus_percent,
		expires_at_frame,
		bond_space_to_unreserve,
		nonce,
		signature,
	}
}

#[test]
fn buy_bonds_uses_raw_vault_securitization() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);

		let mut securitization_funded_vault = test_vault(10, (4 * MICROGONS_PER_ARGON) as u64);
		securitization_funded_vault.securitization = 6 * MICROGONS_PER_ARGON;
		insert_vault(1, securitization_funded_vault);

		let mut bitcoin_funded_vault = test_vault(11, (6 * MICROGONS_PER_ARGON) as u64);
		bitcoin_funded_vault.securitization = 4 * MICROGONS_PER_ARGON;
		insert_vault(2, bitcoin_funded_vault);

		set_argons(2, 7 * MICROGONS_PER_ARGON);
		set_argons(3, 7 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 6, None));
		assert_eq!(VaultsById::get().get(&1).unwrap().committed_microgons, 6 * MICROGONS_PER_ARGON);
		assert_noop!(
			Treasury::buy_bonds(origin(2), 1, 1, None),
			Error::<Test>::InsufficientBondSpace
		);

		assert_ok!(Treasury::buy_bonds(origin(3), 2, 4, None));
		assert_eq!(VaultsById::get().get(&2).unwrap().committed_microgons, 4 * MICROGONS_PER_ARGON);
		assert_noop!(
			Treasury::buy_bonds(origin(3), 2, 1, None),
			Error::<Test>::InsufficientBondSpace
		);

		Treasury::lock_in_vault_capital(1);

		let frame = CurrentFrameVaultCapital::<Test>::get().expect("frame capital");
		assert_eq!(frame.total_active_bonds, 10);
		assert_eq!(
			frame
				.vault_securitization_positions
				.get(&1)
				.map(|vault| vault.active_bond_microgons),
			Some(6 * MICROGONS_PER_ARGON)
		);
		assert_eq!(
			frame
				.vault_securitization_positions
				.get(&2)
				.map(|vault| vault.active_bond_microgons),
			Some(4 * MICROGONS_PER_ARGON)
		);
	});
}

#[test]
fn bond_purchases_cannot_reuse_securitization_under_exit_notice() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(10, 6 * MICROGONS_PER_ARGON);
		set_argons(2, 5 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 6, None));
		let flexible_lot_id = account_bond_lot_ids(10)[0];
		assert_eq!(VaultsById::get().get(&1).unwrap().committed_microgons, 6 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), flexible_lot_id, true));
		VaultsById::mutate(|vaults| {
			vaults.get_mut(&1).unwrap().exit_notice_amount = 6 * MICROGONS_PER_ARGON;
		});

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));
		assert_noop!(
			Treasury::buy_bonds(origin(2), 1, 1, None),
			Error::<Test>::InsufficientBondSpace,
		);
		assert_noop!(
			Treasury::set_bond_lot_flexible(origin(10), flexible_lot_id, false),
			Error::<Test>::InsufficientBondSpace,
		);
		assert_eq!(BondLotsByVault::<Test>::get(1).displaced_flexible_bonds, 0);
	});
}

#[test]
fn buy_bonds_store_plain_and_bonus_terms_and_track_pool_participation() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let vault = test_vault(10, (100 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);

		set_argons(2, 20 * MICROGONS_PER_ARGON);
		set_argons(3, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, None));
		assert_ok!(Treasury::buy_bonds(
			origin(3),
			1,
			5,
			Some(bonus_approval(1, 3, Permill::from_percent(15), 1, 0, 1)),
		));

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
				sharing_percent: Permill::zero(),
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
				sharing_percent: Permill::zero(),
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
		assert_eq!(BondLotsByVault::<Test>::get(1).regular_bonds, 10);
		assert!(<Treasury as TreasuryPoolProvider<TestAccountId>>::has_vault_bond_participation(
			1,
			&account(2),
		));
		assert!(<Treasury as TreasuryPoolProvider<TestAccountId>>::has_vault_bond_participation(
			1,
			&account(3),
		));
	});
}

#[test]
fn flexible_bonds_yield_their_vault_admission_capacity() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));

		set_argons(10, 10 * MICROGONS_PER_ARGON);
		set_argons(2, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(10), 1, 10, None));
		let operator_lot_id = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), operator_lot_id, true));
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));

		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.flexible_bonds, 10);
		assert_eq!(vault_bonds.regular_bonds, 10);

		assert_ok!(Treasury::liquidate_bond_lot(origin(10), operator_lot_id));

		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.flexible_bonds, 0);
		assert_eq!(vault_bonds.regular_bonds, 10);
	});
}

#[test]
fn reservation_uses_capacity_yielded_by_flexible_bonds() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(10, 10 * MICROGONS_PER_ARGON);
		set_argons(2, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(10), 1, 10, None));
		let operator_lot_id = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), operator_lot_id, true));
		assert_noop!(
			Treasury::set_reserved_bond_space(origin(10), 1, 11),
			Error::<Test>::InsufficientBondSpace
		);
		assert_ok!(Treasury::set_reserved_bond_space(origin(10), 1, 10));

		assert_ok!(Treasury::buy_bonds(
			origin(2),
			1,
			10,
			Some(bonus_approval(1, 2, Permill::zero(), 1, 10, System::block_number(),)),
		));
		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.flexible_bonds, 10);
		assert_eq!(vault_bonds.regular_bonds, 10);
		assert_eq!(vault_bonds.reserved_bond_space, 0);
		assert_noop!(
			Treasury::set_reserved_bond_space(origin(10), 1, 1),
			Error::<Test>::InsufficientBondSpace
		);
		assert_noop!(
			Treasury::set_bond_lot_flexible(origin(10), operator_lot_id, false),
			Error::<Test>::InsufficientBondSpace,
		);
		assert_ok!(Treasury::liquidate_bond_lot(origin(10), operator_lot_id));
	});
}

#[test]
fn bond_purchase_unreserves_reserved_bond_space_from_approval() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		let mut vault = test_vault(10, (10 * MICROGONS_PER_ARGON) as u64);
		vault.delegate_account_id = Some(account(3));
		insert_vault(1, vault);
		set_argons(2, 10 * MICROGONS_PER_ARGON);

		assert_noop!(
			Treasury::set_reserved_bond_space(origin(2), 1, 10),
			Error::<Test>::NoPermissions
		);
		assert_noop!(
			Treasury::set_reserved_bond_space(origin(3), 1, 10),
			Error::<Test>::NoPermissions
		);
		assert_noop!(
			Treasury::set_reserved_bond_space(origin(10), 1, 11),
			Error::<Test>::InsufficientBondSpace
		);
		assert_ok!(Treasury::set_reserved_bond_space(origin(10), 1, 10));

		assert_noop!(
			Treasury::buy_bonds(origin(2), 1, 10, None),
			Error::<Test>::InsufficientBondSpace
		);
		assert_ok!(Treasury::buy_bonds(
			origin(2),
			1,
			6,
			Some(bonus_approval(1, 2, Permill::from_percent(15), 1, 10, 1)),
		));
		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.reserved_bond_space, 0);
		assert_eq!(vault_bonds.regular_bonds, 6);
	});
}

#[test]
fn bonus_approval_rejects_wrong_vault_account_expiry_and_signature() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let vault = test_vault(10, (100 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);

		set_argons(2, 20 * MICROGONS_PER_ARGON);
		set_argons(3, 20 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(
			origin(2),
			1,
			5,
			Some(bonus_approval(1, 2, Permill::from_percent(15), 1, 0, 1)),
		));

		assert_err!(
			Treasury::buy_bonds(
				origin(2),
				1,
				5,
				Some(bonus_approval(2, 2, Permill::from_percent(15), 1, 0, 2)),
			),
			Error::<Test>::BonusApprovalWrongVault,
		);
		assert_err!(
			Treasury::buy_bonds(
				origin(2),
				1,
				5,
				Some(bonus_approval(1, 3, Permill::from_percent(15), 1, 0, 2)),
			),
			Error::<Test>::BonusApprovalWrongAccount,
		);
		assert_err!(
			Treasury::buy_bonds(
				origin(2),
				1,
				5,
				Some(bonus_approval(1, 2, Permill::from_percent(15), 0, 0, 2)),
			),
			Error::<Test>::BonusApprovalExpired,
		);

		let mut invalid_signature = bonus_approval(1, 3, Permill::from_percent(15), 1, 0, 2);
		invalid_signature.signature = Signature::Sr25519([1; 64].into());
		assert_err!(
			Treasury::buy_bonds(origin(3), 1, 5, Some(invalid_signature)),
			Error::<Test>::InvalidBonusApprovalSignature,
		);
	});
}

#[test]
fn bonus_approval_rejects_fields_changed_after_signing() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let vault = test_vault(10, (100 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);
		set_argons(2, 20 * MICROGONS_PER_ARGON);

		let mut approval = bonus_approval(1, 2, Permill::from_percent(10), 1, 0, 1);
		approval.bonus_percent = Permill::from_percent(15);

		assert_err!(
			Treasury::buy_bonds(origin(2), 1, 5, Some(approval)),
			Error::<Test>::InvalidBonusApprovalSignature,
		);

		let mut approval = bonus_approval(1, 2, Permill::from_percent(10), 1, 0, 1);
		approval.nonce = 2;

		assert_err!(
			Treasury::buy_bonds(origin(2), 1, 5, Some(approval)),
			Error::<Test>::InvalidBonusApprovalSignature,
		);
	});
}

#[test]
fn bonus_approval_does_not_require_vault_wide_profit_sharing() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let vault = test_vault(10, (100 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);
		set_argons(2, 20 * MICROGONS_PER_ARGON);

		let approval = bonus_approval(1, 2, Permill::from_percent(11), 1, 0, 1);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, Some(approval)));
		let lot_id = account_bond_lot_ids(2)[0];
		assert_eq!(
			BondLotById::<Test>::get(lot_id).unwrap().program,
			BondProgram::Vault {
				vault_id: 1,
				sharing_percent: Permill::zero(),
				bonus_percent: Permill::from_percent(11),
			},
		);
	});
}

#[test]
fn bonus_approval_allows_multiple_coupons_and_rejects_reuse() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let vault = test_vault(10, (100 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);

		set_argons(2, 20 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(
			origin(2),
			1,
			5,
			Some(bonus_approval(1, 2, Permill::from_percent(15), 2, 0, 1)),
		));
		assert_ok!(Treasury::buy_bonds(
			origin(2),
			1,
			5,
			Some(bonus_approval(1, 2, Permill::from_percent(10), 2, 0, 2)),
		));
		assert_eq!(account_bond_lot_ids(2).len(), 2);

		assert_err!(
			Treasury::buy_bonds(
				origin(2),
				1,
				5,
				Some(bonus_approval(1, 2, Permill::from_percent(10), 2, 0, 2)),
			),
			Error::<Test>::BonusApprovalAlreadyUsed,
		);
		assert_err!(
			Treasury::buy_bonds(
				origin(2),
				1,
				5,
				Some(bonus_approval(1, 2, Permill::from_percent(10), 2, 0, 1)),
			),
			Error::<Test>::BonusApprovalAlreadyUsed,
		);
	});
}

#[test]
fn liquidate_bond_lot_removes_it_from_future_frames_and_releases_on_maturity() {
	new_test_ext().execute_with(|| {
		LastOperationalBondTotal::set(None);
		MinimumArgonsPerContributor::set(1);
		MaxArgonBondLots::set(2);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		insert_vault(2, test_vault(11, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 20 * MICROGONS_PER_ARGON);
		set_argons(3, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(3), 2, 1, None));
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 3, None));
		let bond_lot_id = account_bond_lot_ids(2)[0];
		let other_bond_lot_id = account_bond_lot_ids(3)[0];
		assert!(BondLotIdsByVault::<Test>::contains_key(1, bond_lot_id));
		assert!(BondLotIdsByVault::<Test>::contains_key(2, other_bond_lot_id));
		assert_eq!(TotalArgonBondLots::<Test>::get(), 2);
		assert_err!(
			Treasury::buy_bonds(origin(2), 1, 1, None),
			Error::<Test>::MaxArgonBondLotsExceeded,
		);
		assert_eq!(LastOperationalBondTotal::get(), Some((account(2), 3 * MICROGONS_PER_ARGON)));

		assert_ok!(Treasury::liquidate_bond_lot(origin(2), bond_lot_id));
		assert!(BondLotIdsByVault::<Test>::contains_key(1, bond_lot_id));
		assert_eq!(TotalArgonBondLots::<Test>::get(), 2);
		assert_err!(
			Treasury::buy_bonds(origin(2), 1, 1, None),
			Error::<Test>::MaxArgonBondLotsExceeded,
		);
		assert_eq!(LastOperationalBondTotal::get(), Some((account(2), 0)));

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("releasing bond lot");
		assert_eq!(bond_lot.release_reason, Some(BondReleaseReason::UserLiquidation));
		assert_eq!(bond_lot.release_frame_id, Some(11));
		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.regular_bonds, 0);
		assert_eq!(vault_bonds.flexible_bonds, 0);
		assert_eq!(PendingBondReleasesByFrame::<Test>::get(11), vec![bond_lot_id]);

		Treasury::release_pending_bond_lots(11);

		assert!(BondLotById::<Test>::get(bond_lot_id).is_none());
		assert!(!BondLotIdsByVault::<Test>::contains_key(1, bond_lot_id));
		assert!(BondLotIdsByVault::<Test>::contains_key(2, other_bond_lot_id));
		assert_eq!(TotalArgonBondLots::<Test>::get(), 1);
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&account(2)
			),
			0,
		);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 1, None));
	});
}

#[test]
fn liquidate_bond_lot_rejects_when_it_would_drop_below_encumbered_backing() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
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
		assert_eq!(BondLotsByVault::<Test>::get(1).regular_bonds, 5);
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
fn active_account_vault_bond_amount_only_counts_vault_bonds() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));

		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		insert_vault(2, test_vault(11, (100 * MICROGONS_PER_ARGON) as u64));

		set_argons(2, 30 * MICROGONS_PER_ARGON);
		set_ownership(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, None));
		assert_ok!(Treasury::buy_bonds(origin(2), 2, 7, None));
		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 11));

		assert_eq!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::active_vault_bond_amount(
				1,
				&account(2),
			),
			5 * MICROGONS_PER_ARGON,
		);
		assert_eq!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::active_vault_bond_amount(
				2,
				&account(2),
			),
			7 * MICROGONS_PER_ARGON,
		);
		assert_eq!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::active_account_vault_bond_amount(
				&account(2),
			),
			12 * MICROGONS_PER_ARGON,
		);
	});
}

#[test]
fn buy_argonot_bonds_rejects_when_full_queue_lot_does_not_beat_floor() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxActiveArgonotBondLots::set(2);
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
		MaxActiveArgonotBondLots::set(2);
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
		set_ownership(2, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 6));

		assert_err!(
			Treasury::buy_argonot_bonds(origin(2), 1),
			Error::<Test>::ArgonotBondPurchaseAboveCap,
		);

		assert_eq!(account_bond_lot_ids(2).len(), 1);
		assert_eq!(argonot_bond_lots().len(), 1);
		assert_eq!(TotalActiveArgonotBonds::<Test>::get(), 6);
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
fn regular_lots_do_not_bump_smaller_purchases() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (102 * MICROGONS_PER_ARGON) as u64));

		set_argons(10, MICROGONS_PER_ARGON);
		set_argons(2, 101 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(10), 1, 1, None));
		for _ in 0..101 {
			assert_ok!(Treasury::buy_bonds(origin(2), 1, 1, None));
		}

		assert_eq!(BondLotsByVault::<Test>::get(1).regular_bonds, 102);
		let operator_lot_id = account_bond_lot_ids(10)[0];
		let operator_lot = BondLotById::<Test>::get(operator_lot_id).expect("operator lot");
		assert_eq!(operator_lot.release_reason, None);
		assert_eq!(account_bond_lot_ids(2).len(), 101);
		assert_eq!(BondLotsByVault::<Test>::get(1).flexible_bonds, 0);
	});
}

#[test]
fn flexible_bond_total_does_not_saturate_past_bonds_max() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (Bonds::MAX as u64) * MICROGONS_PER_ARGON as u64));
		set_argons(10, (Bonds::MAX as u128 + 1) * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, Bonds::MAX, None));
		let first_lot = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), first_lot, true));
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 1, None));
		let second_lot = account_bond_lot_ids(10)
			.into_iter()
			.find(|lot_id| BondLotById::<Test>::get(lot_id).is_some_and(|lot| lot.bonds == 1))
			.expect("one-bond lot");
		assert_err!(
			Treasury::set_bond_lot_flexible(origin(10), second_lot, true),
			ArithmeticError::Overflow,
		);
		assert_eq!(BondLotsByVault::<Test>::get(1).flexible_bonds, Bonds::MAX);
		assert_eq!(BondLotsByVault::<Test>::get(1).regular_bonds, 1);
		assert!(!BondLotById::<Test>::get(second_lot).unwrap().is_flexible);
	});
}

#[test]
fn frame_lock_counts_regular_and_undisplaced_flexible_bonds() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));

		set_argons(10, 10 * MICROGONS_PER_ARGON);
		set_argons(2, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 10, None));
		let flexible_lot_id = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), flexible_lot_id, true));
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 6, None));

		Treasury::lock_in_vault_capital(1);

		let frame = CurrentFrameVaultCapital::<Test>::get().expect("frame capital");
		assert_eq!(frame.total_active_bonds, 10);
		assert_eq!(BondLotsByVault::<Test>::get(1).displaced_flexible_bonds, 6);
		assert_eq!(
			frame.vault_securitization_positions.get(&1).unwrap().active_bond_microgons,
			10 * MICROGONS_PER_ARGON
		);
	});
}

#[test]
fn regular_bonds_displace_flexible_bonds_without_losing_their_own_earnings() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));

		set_argons(10, 10 * MICROGONS_PER_ARGON);
		set_argons(2, 6 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 10, None));
		let flexible_lot_id = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), flexible_lot_id, true));
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 6, None));
		let regular_lot_id = account_bond_lot_ids(2)[0];

		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		let flexible_lot = BondLotById::<Test>::get(flexible_lot_id).expect("flexible lot");
		let regular_lot = BondLotById::<Test>::get(regular_lot_id).expect("regular lot");
		assert_eq!(flexible_lot.last_frame_earnings, Some(2_000_000));
		assert_eq!(regular_lot.last_frame_earnings, Some(3_000_000));
		assert_eq!(flexible_lot.cumulative_earnings, 2_000_000);
		assert_eq!(regular_lot.cumulative_earnings, 3_000_000);
		assert_eq!(LastVaultProfits::get()[0].capital_contributed_by_vault, 0);
	});
}

#[test]
fn flexible_bond_displacement_is_shared_across_small_lots() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (2 * MICROGONS_PER_ARGON) as u64));
		set_argons(10, 2 * MICROGONS_PER_ARGON);
		for _ in 0..2 {
			assert_ok!(Treasury::buy_bonds(origin(10), 1, 1, None));
		}
		let flexible_lot_ids = account_bond_lot_ids(10);
		for bond_lot_id in &flexible_lot_ids {
			assert_ok!(Treasury::set_bond_lot_flexible(origin(10), *bond_lot_id, true,));
		}
		set_argons(4, MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(4), 1, 1, None));
		assert_eq!(BondLotsByVault::<Test>::get(1).displaced_flexible_bonds, 1);
		set_target_securitization(2 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		Treasury::distribute_bid_pool(1);
		for bond_lot_id in flexible_lot_ids {
			assert_eq!(
				BondLotById::<Test>::get(bond_lot_id).unwrap().last_frame_earnings,
				Some(1_250_000),
			);
		}
		assert_eq!(
			BondLotById::<Test>::get(account_bond_lot_ids(4)[0])
				.unwrap()
				.last_frame_earnings,
			Some(2_500_000),
		);
	});
}

#[test]
fn mid_frame_purchase_displaces_flexible_bonds_for_the_next_frame() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(10, 10 * MICROGONS_PER_ARGON);
		set_argons(2, 6 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 10, None));
		let flexible_lot_id = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), flexible_lot_id, true));
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 6, None));
		let regular_lot_id = account_bond_lot_ids(2)[0];
		assert_eq!(BondLotsByVault::<Test>::get(1).displaced_flexible_bonds, 6);
		assert_eq!(
			BondLotById::<Test>::get(regular_lot_id)
				.unwrap()
				.locked_frame_terms
				.unwrap()
				.bonds,
			0
		);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		Treasury::distribute_bid_pool(1);
		assert_eq!(
			BondLotById::<Test>::get(flexible_lot_id).unwrap().last_frame_earnings,
			Some(5_000_000)
		);
		assert_eq!(BondLotById::<Test>::get(regular_lot_id).unwrap().last_frame_earnings, None);

		CurrentFrameId::set(2);
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(2);
		assert_eq!(CurrentFrameVaultCapital::<Test>::get().unwrap().total_active_bonds, 10);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		Treasury::distribute_bid_pool(2);
		assert_eq!(
			BondLotById::<Test>::get(flexible_lot_id).unwrap().last_frame_earnings,
			Some(2_000_000)
		);
		assert_eq!(
			BondLotById::<Test>::get(regular_lot_id).unwrap().last_frame_earnings,
			Some(3_000_000)
		);
	});
}

#[test]
fn flexible_lots_all_receive_direct_payouts() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (2 * MICROGONS_PER_ARGON) as u64));

		set_argons(10, 2 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 1, None));
		let first_lot_id = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), first_lot_id, true));
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 1, None));
		let second_lot_id = account_bond_lot_ids(10)
			.into_iter()
			.find(|bond_lot_id| *bond_lot_id != first_lot_id)
			.expect("second flexible lot");
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), second_lot_id, true));

		set_target_securitization(2 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		assert_eq!(CurrentFrameVaultCapital::<Test>::get().unwrap().total_active_bonds, 2);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		let earnings = [first_lot_id, second_lot_id].map(|bond_lot_id| {
			BondLotById::<Test>::get(bond_lot_id).and_then(|lot| lot.last_frame_earnings)
		});
		assert_eq!(earnings, [Some(2_500_000), Some(2_500_000)]);
	});
}

#[test]
fn locked_frame_terms_capture_the_first_flexibility_change_only() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(10, 3 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 3, None));
		let lot_id = account_bond_lot_ids(10)[0];
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), lot_id, true));
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), lot_id, false));
		let lot = BondLotById::<Test>::get(lot_id).unwrap();
		let terms = lot.locked_frame_terms.unwrap();
		assert_eq!(terms.bonds, 3);
		assert!(!terms.is_flexible);
		assert!(!lot.is_flexible);

		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		Treasury::distribute_bid_pool(1);
		let lot = BondLotById::<Test>::get(lot_id).unwrap();
		assert_eq!(lot.last_frame_earnings, Some(1_500_000));
		assert!(lot.locked_frame_terms.is_none());
	});
}

#[test]
fn fixed_pools_are_reserved_and_burned_even_without_participants() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		assert_eq!(Balances::balance(&BidPoolAccountId::get()), 0);
		assert_eq!(Balances::balance(&TreasuryReservesAccountId::get()), 20 * MICROGONS_PER_ARGON);
		assert_eq!(Balances::total_issuance(), 20 * MICROGONS_PER_ARGON);
		let distribution = System::events()
			.into_iter()
			.find_map(|record| match record.event {
				RuntimeEvent::Treasury(crate::Event::FrameEarningsDistributed {
					frame_id: 1,
					bid_pool_distributed,
					burned,
					treasury_reserves,
					..
				}) => Some((bid_pool_distributed, burned, treasury_reserves)),
				_ => None,
			})
			.expect("distribution event");
		assert_eq!(distribution, (0, 80 * MICROGONS_PER_ARGON, 20 * MICROGONS_PER_ARGON));
	});
}

#[test]
fn regular_bonds_keep_reward_weight_after_securitization_drops() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));

		insert_vault(1, test_vault(10, (5 * MICROGONS_PER_ARGON) as u64));
		Treasury::lock_in_vault_capital(1);

		let frame = CurrentFrameVaultCapital::<Test>::get().expect("frame capital");
		assert_eq!(frame.total_active_bonds, 10);
		assert_eq!(
			frame.vault_securitization_positions.get(&1).unwrap().active_bond_microgons,
			10 * MICROGONS_PER_ARGON
		);
	});
}

#[test]
fn distribution_burns_unfilled_bond_pool_without_reducing_vault_rewards() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));

		let bond_lot_id = account_bond_lot_ids(2)[0];
		let balance_before = Balances::balance(&account(2));

		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);

		let current = CurrentFrameVaultCapital::<Test>::get().expect("current frame capital");
		assert_eq!(current.frame_id, 1);
		assert_eq!(current.total_active_bonds, 4);
		assert_eq!(
			current.vault_securitization_positions.get(&1).unwrap().active_bond_microgons,
			4 * MICROGONS_PER_ARGON
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
					stake_pool_distributed,
					argon_bond_pool_distributed,
					vault_pool_distributed,
					burned,
					treasury_reserves,
					participating_vaults,
				}) if *frame_id == 1
					&& *bid_pool_distributed == 3 * MICROGONS_PER_ARGON
					&& *stake_pool_distributed == 0
					&& *argon_bond_pool_distributed == 2 * MICROGONS_PER_ARGON
					&& *vault_pool_distributed == MICROGONS_PER_ARGON
					&& *burned == 77 * MICROGONS_PER_ARGON
					&& *treasury_reserves == 20 * MICROGONS_PER_ARGON
					&& *participating_vaults == 1
			)
		}));

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("paid bond lot");
		assert_eq!(bond_lot.participated_frames, 1);
		assert_eq!(bond_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(bond_lot.last_frame_earnings, Some(2_000_000));
		assert_eq!(bond_lot.cumulative_earnings, 2_000_000);
		assert_eq!(Balances::balance(&account(2)), balance_before + 2_000_000);
		assert_eq!(Balances::balance(&TreasuryReservesAccountId::get()), 20_000_000);

		assert_eq!(LastVaultProfits::get().len(), 1);
		assert_eq!(LastVaultProfits::get()[0].vault_id, 1);
		assert_eq!(LastVaultProfits::get()[0].earnings, MICROGONS_PER_ARGON);
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, MICROGONS_PER_ARGON);
		assert_eq!(LastVaultProfits::get()[0].capital_contributed, 4 * MICROGONS_PER_ARGON);
		assert_eq!(LastVaultProfits::get()[0].capital_contributed_by_vault, 0);
	});
}

#[test]
fn vault_calculator_keeps_idle_payout_low_and_rewards_argonot_backing() {
	for (argonot_backing, expected_earnings) in [
		(0, MICROGONS_PER_ARGON),
		(10 * MICROGONS_PER_ARGON, 2 * MICROGONS_PER_ARGON),
		(20 * MICROGONS_PER_ARGON, 3 * MICROGONS_PER_ARGON),
		(100 * MICROGONS_PER_ARGON, 3 * MICROGONS_PER_ARGON),
	] {
		new_test_ext().execute_with(|| {
			LastVaultProfits::set(vec![]);
			VaultRewardCommittedMicronots::set(Default::default());
			insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
			VaultArgonotMicronots::mutate(|backing| {
				backing.insert(1, argonot_backing);
			});
			set_target_securitization(10 * MICROGONS_PER_ARGON);
			Treasury::lock_in_vault_capital(1);
			assert_eq!(
				VaultsById::get().get(&1).map(|vault| vault.committed_microgons),
				Some(10 * MICROGONS_PER_ARGON),
			);
			assert_eq!(
				VaultRewardCommittedMicronots::get().get(&1).copied().unwrap_or_default(),
				argonot_backing.min(20 * MICROGONS_PER_ARGON)
			);
			assert_eq!(
				CurrentFrameVaultCapital::<Test>::get()
					.unwrap()
					.vault_securitization_positions
					.get(&1)
					.unwrap()
					.argonot_securitization_in_microgons,
				argonot_backing,
			);
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			Treasury::distribute_bid_pool(1);
			let earnings = LastVaultProfits::get()[0].earnings_for_vault;
			assert_eq!(earnings, expected_earnings);
		});
	}
}

#[test]
fn vault_calculator_does_not_reward_unsecuritized_bitcoin() {
	new_test_ext().execute_with(|| {
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		VaultBitcoinSatoshis::mutate(|bitcoin| {
			bitcoin.insert(1, 100_000_000);
		});
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		Treasury::distribute_bid_pool(1);
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, MICROGONS_PER_ARGON);
	});
}

#[test]
fn vault_calculator_rewards_activated_collateral_and_penalizes_excess_bitcoin_value() {
	let mut earnings = Vec::new();
	for (securitization, activated, satoshis) in [
		(10, 8, 50_000_000),
		(10, 8, 80_000_000),
		(10, 10, 100_000_000),
		(10, 10, 150_000_000),
		(15, 15, 150_000_000),
		(10, 10, 200_000_000),
	] {
		new_test_ext().execute_with(|| {
			LastVaultProfits::set(vec![]);
			let mut vault = test_vault(10, (securitization * MICROGONS_PER_ARGON) as u64);
			vault.activated_securitization = activated * MICROGONS_PER_ARGON;
			insert_vault(1, vault);
			VaultBitcoinSatoshis::mutate(|bitcoin| {
				bitcoin.insert(1, satoshis);
			});
			set_target_securitization(securitization * MICROGONS_PER_ARGON);
			Treasury::lock_in_vault_capital(1);
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			Treasury::distribute_bid_pool(1);
			earnings.push(LastVaultProfits::get()[0].earnings_for_vault);
		});
	}
	assert_eq!(earnings[0], earnings[1]);
	assert!(earnings[3] < earnings[2]);
	assert_eq!(earnings[2], earnings[4]);
	assert_eq!(earnings[5], MICROGONS_PER_ARGON);
}

#[test]
fn vault_snapshot_values_argonot_backing_at_the_previous_frame_average() {
	new_test_ext().execute_with(|| {
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		VaultArgonotMicronots::mutate(|backing| {
			backing.insert(1, 5 * MICROGONS_PER_ARGON);
		});
		ArgonotPriceInUsd::set(FixedU128::from_u32(3));
		AverageArgonotPriceInMicrogons::set(2 * MICROGONS_PER_ARGON);

		Treasury::lock_in_vault_capital(2);
		assert_eq!(LastAverageArgonotPriceFrame::get(), Some(1));
		let snapshot = CurrentFrameVaultCapital::<Test>::get().expect("frame capital");
		assert_eq!(
			snapshot
				.vault_securitization_positions
				.get(&1)
				.expect("vault position")
				.argonot_securitization_in_microgons,
			10 * MICROGONS_PER_ARGON,
		);
		VaultArgonotMicronots::mutate(|backing| {
			backing.insert(1, 50 * MICROGONS_PER_ARGON);
		});
		AverageArgonotPriceInMicrogons::set(3 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(3);
		assert_eq!(VaultRewardCommittedMicronots::get().get(&1).copied(), Some(6_666_667));
		AverageArgonotPriceInMicrogons::set(0);
		Treasury::lock_in_vault_capital(4);
		assert_eq!(VaultRewardCommittedMicronots::get().get(&1).copied(), Some(6_666_667));
	});
}

#[test]
fn vault_calculator_reaches_full_share_with_bitcoin_bonds_and_argonots() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		let mut vault = test_vault(10, (10 * MICROGONS_PER_ARGON) as u64);
		vault.activated_securitization = 10 * MICROGONS_PER_ARGON;
		insert_vault(1, vault);
		VaultBitcoinSatoshis::mutate(|bitcoin| {
			bitcoin.insert(1, 100_000_000);
		});
		VaultArgonotMicronots::mutate(|backing| {
			backing.insert(1, 20 * MICROGONS_PER_ARGON);
		});
		set_argons(2, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		// A position change after locking must not alter this frame's payout terms.
		VaultBitcoinSatoshis::mutate(|bitcoin| {
			bitcoin.remove(&1);
		});
		VaultArgonotMicronots::mutate(|backing| {
			backing.remove(&1);
		});
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		Treasury::distribute_bid_pool(1);
		assert!(LastVaultProfits::get()[0].earnings_for_vault >= 51 * MICROGONS_PER_ARGON - 1);
	});
}

#[test]
fn bounded_out_vault_share_burns_but_all_bonds_still_earn() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		MinimumArgonsPerContributor::set(1);
		MaxVaultsPerPool::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		insert_vault(2, test_vault(11, (10 * MICROGONS_PER_ARGON) as u64));

		set_argons(2, 10 * MICROGONS_PER_ARGON);
		set_argons(3, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));
		assert_ok!(Treasury::buy_bonds(origin(3), 2, 10, None));
		let paid_bond_lot_id = account_bond_lot_ids(2)[0];
		let other_bond_lot_id = account_bond_lot_ids(3)[0];

		set_target_securitization(20 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		assert_eq!(
			BondLotById::<Test>::get(paid_bond_lot_id).and_then(|lot| lot.last_frame_earnings),
			Some(2_500_000),
		);
		assert_eq!(
			BondLotById::<Test>::get(other_bond_lot_id).unwrap().last_frame_earnings,
			Some(2_500_000)
		);
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, 500_000);
		let distribution = System::events()
			.into_iter()
			.find_map(|record| match record.event {
				RuntimeEvent::Treasury(crate::Event::FrameEarningsDistributed {
					argon_bond_pool_distributed,
					vault_pool_distributed,
					burned,
					participating_vaults,
					..
				}) => Some((
					argon_bond_pool_distributed,
					vault_pool_distributed,
					burned,
					participating_vaults,
				)),
				_ => None,
			})
			.expect("distribution event");
		assert_eq!(distribution, (5_000_000, 500_000, 74_500_000, 1));
	});
}

#[test]
fn stake_and_argon_bond_pools_are_paid_before_the_vault_pool() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		insert_vault(1, test_vault(10, (4 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 50 * MICROGONS_PER_ARGON);
		set_ownership(3, 50 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));
		assert_ok!(Treasury::buy_argonot_bonds(origin(3), 5));

		let vault_bond_lot_id = account_bond_lot_ids(2)[0];
		let argonot_bond_lot_id = account_bond_lot_ids(3)[0];

		Treasury::lock_in_argonot_bond_participants(1);
		set_target_securitization(4 * MICROGONS_PER_ARGON);
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
					stake_pool_distributed,
					argon_bond_pool_distributed,
					vault_pool_distributed,
					burned,
					treasury_reserves,
					participating_vaults,
				}) if *frame_id == 1
					&& *bid_pool_distributed == 21 * MICROGONS_PER_ARGON
					&& *stake_pool_distributed == 15 * MICROGONS_PER_ARGON
					&& *argon_bond_pool_distributed == 5 * MICROGONS_PER_ARGON
					&& *vault_pool_distributed == MICROGONS_PER_ARGON
					&& *burned == 59 * MICROGONS_PER_ARGON
					&& *treasury_reserves == 20 * MICROGONS_PER_ARGON
					&& *participating_vaults == 1
			)
		}));

		let argonot_bond_lot = BondLotById::<Test>::get(argonot_bond_lot_id).expect("argonot lot");
		assert_eq!(argonot_bond_lot.last_frame_earnings, Some(15_000_000));
		assert_eq!(argonot_bond_lot.cumulative_earnings, 15_000_000);
		assert_eq!(Balances::balance(&account(3)), 15_000_000);

		let vault_bond_lot = BondLotById::<Test>::get(vault_bond_lot_id).expect("vault lot");
		assert_eq!(vault_bond_lot.last_frame_earnings, Some(5_000_000));
		assert_eq!(vault_bond_lot.cumulative_earnings, 5_000_000);

		assert_eq!(Balances::balance(&TreasuryReservesAccountId::get()), 20_000_000);
		assert_eq!(LastVaultProfits::get().len(), 1);
		assert_eq!(LastVaultProfits::get()[0].earnings, MICROGONS_PER_ARGON);
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, 1_000_000);
	});
}

#[test]
fn base_argon_bond_pool_ignores_future_revenue_sharing_terms() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let vault = test_vault(10, (20 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		set_argons(3, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));
		assert_ok!(Treasury::buy_bonds(
			origin(3),
			1,
			4,
			Some(bonus_approval(1, 3, Permill::from_percent(10), 1, 0, 1)),
		));

		let plain_lot_id = account_bond_lot_ids(2)[0];
		let bonus_lot_id = account_bond_lot_ids(3)[0];

		set_target_securitization(20 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		let plain_lot = BondLotById::<Test>::get(plain_lot_id).expect("plain bond lot");
		let bonus_lot = BondLotById::<Test>::get(bonus_lot_id).expect("bonus bond lot");
		assert_eq!(plain_lot.last_frame_earnings, Some(1_000_000));
		assert_eq!(bonus_lot.last_frame_earnings, Some(1_000_000));
		assert_eq!(bonus_lot.cumulative_earnings, plain_lot.cumulative_earnings);
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, 1_000_000);
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
		assert_eq!(bond_lot.last_frame_earnings, Some(15_000_000));
		assert_eq!(bond_lot.cumulative_earnings, 15_000_000);
		assert_eq!(Balances::balance(&account(2)), 15_000_000);
	});
}

#[test]
fn lock_in_vault_capital_selects_top_vaults_by_securitization() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxVaultsPerPool::set(2);
		CurrentFrameId::set(1);

		insert_vault(1, test_vault(11, MICROGONS_PER_ARGON as u64));
		insert_vault(2, test_vault(12, (2 * MICROGONS_PER_ARGON) as u64));
		insert_vault(3, test_vault(13, (3 * MICROGONS_PER_ARGON) as u64));

		set_argons(11, 10 * MICROGONS_PER_ARGON);
		set_argons(12, 10 * MICROGONS_PER_ARGON);
		set_argons(13, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(11), 1, 1, None));
		assert_ok!(Treasury::buy_bonds(origin(12), 2, 2, None));
		assert_ok!(Treasury::buy_bonds(origin(13), 3, 3, None));

		Treasury::lock_in_vault_capital(1);

		let current = CurrentFrameVaultCapital::<Test>::get().expect("current frame capital");
		let vaults = VaultsById::get();
		assert_eq!(vaults[&1].committed_microgons, MICROGONS_PER_ARGON);
		assert_eq!(vaults[&2].committed_microgons, 2 * MICROGONS_PER_ARGON);
		assert_eq!(vaults[&3].committed_microgons, 3 * MICROGONS_PER_ARGON);
		assert_eq!(current.frame_id, 1);
		assert_eq!(current.vault_securitization_positions.len(), 2);
		assert!(current.vault_securitization_positions.get(&1).is_none());
		assert_eq!(
			current
				.vault_securitization_positions
				.get(&2)
				.map(|vault| vault.active_bond_microgons),
			Some(2 * MICROGONS_PER_ARGON)
		);
		assert_eq!(
			current
				.vault_securitization_positions
				.get(&3)
				.map(|vault| vault.active_bond_microgons),
			Some(3 * MICROGONS_PER_ARGON)
		);
	});
}

#[test]
fn locked_frame_still_pays_after_lot_is_liquidated() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));

		let bond_lot_id = account_bond_lot_ids(2)[0];
		let balance_before = Balances::balance(&account(2));

		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Treasury::liquidate_bond_lot(origin(2), bond_lot_id));
		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.regular_bonds, 0);
		assert_eq!(vault_bonds.flexible_bonds, 0);

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		Treasury::distribute_bid_pool(1);

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("liquidating bond lot");
		assert_eq!(bond_lot.release_reason, Some(BondReleaseReason::UserLiquidation));
		assert_eq!(bond_lot.participated_frames, 1);
		assert_eq!(bond_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(bond_lot.last_frame_earnings, Some(2_000_000));
		assert_eq!(bond_lot.cumulative_earnings, 2_000_000);
		assert_eq!(Balances::balance(&account(2)), balance_before + 2_000_000);
	});
}

#[test]
fn locked_frame_skips_lot_after_it_is_fully_burned() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));

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
		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.regular_bonds, 0);
		assert_eq!(vault_bonds.flexible_bonds, 0);
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
		System::set_block_number(1);
		ExistentialDeposit::set(10);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));

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
				is_flexible: false,
				locked_frame_terms: None,
				created_frame_id: 1,
				participated_frames: 0,
				last_frame_earnings_frame_id: None,
				last_frame_earnings: None,
				cumulative_earnings: 0,
				release_frame_id: None,
				release_reason: None,
			},
		);
		BondLotIdsByVault::<Test>::insert(1, 0, ());
		TotalArgonBondLots::<Test>::put(1);
		CurrentFrameVaultCapital::<Test>::put(FrameVaultCapital {
			frame_id: 1,
			total_active_bonds: 1,
			target_securitization: 1,
			total_securitization: 0,
			vault_securitization_positions: BoundedBTreeMap::new(),
		});

		frame_system::Pallet::<Test>::inc_providers(&BidPoolAccountId::get());
		set_argons(BidPoolAccountId::get(), 39);

		Treasury::distribute_bid_pool(1);

		let bond_lot = BondLotById::<Test>::get(0).expect("bond lot");
		assert_eq!(bond_lot.participated_frames, 1);
		assert_eq!(bond_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(bond_lot.last_frame_earnings, Some(0));
		assert_eq!(bond_lot.cumulative_earnings, 0);
		assert_eq!(Balances::balance(&account(99)), 0);
		assert_eq!(Balances::balance(&TreasuryReservesAccountId::get()), 0);
		assert_eq!(Balances::balance(&BidPoolAccountId::get()), 0);
		assert!(System::events().iter().any(|record| matches!(
			&record.event,
			RuntimeEvent::Treasury(crate::Event::FrameEarningsDistributed {
				frame_id: 1,
				bid_pool_distributed: 0,
				burned: 39,
				treasury_reserves: 0,
				..
			})
		)));
	});
}

#[test]
fn run_frame_transition_releases_distributes_and_locks_without_paying_operational_rewards() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		TreasuryExitDelayFrames::set(1);
		CurrentFrameId::set(1);

		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		insert_vault(2, test_vault(11, (10 * MICROGONS_PER_ARGON) as u64));

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));
		let payout_bond_lot_id = account_bond_lot_ids(2)[0];
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		set_argons(3, 20 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(3), 2, 2, None));
		let released_bond_lot_id = account_bond_lot_ids(3)[0];
		assert_ok!(Treasury::liquidate_bond_lot(origin(3), released_bond_lot_id,));
		set_argons(42, 0);

		set_target_securitization(10 * MICROGONS_PER_ARGON);
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
		assert!(current.vault_securitization_positions.get(&2).is_some());
		assert_eq!(Balances::balance(&account(42)), 0);
		assert_eq!(LastVaultProfits::get().len(), 2);
		assert_eq!(LastVaultProfits::get()[0].vault_id, 1);
		assert_eq!(LastVaultProfits::get()[0].capital_contributed, 4 * MICROGONS_PER_ARGON);

		let payout_lot = BondLotById::<Test>::get(payout_bond_lot_id).expect("payout lot");
		assert_eq!(payout_lot.participated_frames, 1);
		assert_eq!(payout_lot.last_frame_earnings_frame_id, Some(1));
		assert_eq!(payout_lot.last_frame_earnings, Some(2_000_000));
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
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
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
fn burn_encumbered_flexible_bonds_preserves_reserved_bond_space() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(10, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(10), 1, 10, None));
		let bond_lot_id = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), bond_lot_id, true));
		assert_ok!(Treasury::set_reserved_bond_space(origin(10), 1, 10));
		let encumber_result =
			<Treasury as TreasuryPoolProvider<TestAccountId>>::encumber_bond_microgons(
				&account(10),
				10 * MICROGONS_PER_ARGON,
			);
		assert!(encumber_result.is_ok());

		let burn_result =
			<Treasury as TreasuryPoolProvider<TestAccountId>>::burn_encumbered_bond_microgons(
				&account(10),
				5 * MICROGONS_PER_ARGON,
			);
		assert!(burn_result.is_ok());

		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.flexible_bonds, 5);
		assert_eq!(vault_bonds.reserved_bond_space, 10);
		assert_eq!(BondLotById::<Test>::get(bond_lot_id).expect("bond lot").bonds, 5);

		let burn_result =
			<Treasury as TreasuryPoolProvider<TestAccountId>>::burn_encumbered_bond_microgons(
				&account(10),
				5 * MICROGONS_PER_ARGON,
			);
		assert!(burn_result.is_ok());
		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.flexible_bonds, 0);
		assert_eq!(vault_bonds.reserved_bond_space, 10);
		assert!(BondLotById::<Test>::get(bond_lot_id).is_none());
	});
}

#[test]
fn burn_encumbered_flexible_lots_preserves_reserved_bond_space() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(10, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(10), 1, 5, None));
		let first_bond_lot_id = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), first_bond_lot_id, true,));
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 5, None));
		let second_bond_lot_id = account_bond_lot_ids(10)
			.into_iter()
			.find(|bond_lot_id| *bond_lot_id != first_bond_lot_id)
			.expect("second bond lot");
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), second_bond_lot_id, true,));
		assert_ok!(Treasury::set_reserved_bond_space(origin(10), 1, 10));
		let encumber_result =
			<Treasury as TreasuryPoolProvider<TestAccountId>>::encumber_bond_microgons(
				&account(10),
				10 * MICROGONS_PER_ARGON,
			);
		assert!(encumber_result.is_ok());

		let burn_result =
			<Treasury as TreasuryPoolProvider<TestAccountId>>::burn_encumbered_bond_microgons(
				&account(10),
				10 * MICROGONS_PER_ARGON,
			);
		assert!(burn_result.is_ok());

		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.flexible_bonds, 0);
		assert_eq!(vault_bonds.reserved_bond_space, 10);
		assert!(account_bond_lot_ids(10).is_empty());
	});
}

#[test]
fn burn_encumbered_bond_microgons_keeps_fractional_remainder_held_until_it_is_released() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
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

		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.regular_bonds, 0);
		assert_eq!(vault_bonds.flexible_bonds, 0);
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
				is_flexible: false,
				locked_frame_terms: None,
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
				is_flexible: false,
				locked_frame_terms: None,
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

#[test]
fn release_succeeds_when_other_consumers_need_the_last_provider() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owner = account(2);
		set_ownership(&owner, MICROGONS_PER_ARGON);
		assert_ok!(Treasury::create_hold::<Ownership>(&owner, MICROGONS_PER_ARGON));
		assert_ok!(frame_system::Pallet::<Test>::inc_consumers(&owner));
		assert_eq!(System::providers(&owner), 1);
		assert_eq!(System::consumers(&owner), 2);

		BondLotById::<Test>::insert(
			0,
			BondLot {
				owner: owner.clone(),
				program: BondProgram::Argonot,
				bonds: 1,
				is_flexible: false,
				locked_frame_terms: None,
				created_frame_id: 1,
				participated_frames: 0,
				last_frame_earnings_frame_id: None,
				last_frame_earnings: None,
				cumulative_earnings: 0,
				release_frame_id: Some(11),
				release_reason: Some(BondReleaseReason::UserLiquidation),
			},
		);
		BondLotIdsByAccount::<Test>::insert(&owner, 0, ());
		PendingBondReleasesByFrame::<Test>::insert(11, BoundedVec::truncate_from(vec![0]));

		Treasury::release_pending_bond_lots(11);

		assert_eq!(
			Ownership::balance_on_hold(
				&RuntimeHoldReason::from(HoldReason::ContributedToTreasury),
				&owner,
			),
			0,
		);
		assert_eq!(System::providers(&owner), 1);
		assert_eq!(System::consumers(&owner), 1);
		assert!(!BondLotById::<Test>::contains_key(0));
		assert!(!BondLotIdsByAccount::<Test>::contains_key(&owner, 0));
		assert!(PendingBondReleasesByFrame::<Test>::get(11).is_empty());
		assert_eq!(PendingBondReleaseRetryCursor::<Test>::get(), None);
		System::assert_last_event(RuntimeEvent::Treasury(super::Event::BondLotReleased {
			frame_id: 11,
			program_id: BondProgramId::Argonot,
			bond_lot_id: 0,
			account_id: owner,
			bonds: 1,
		}));
	});
}
