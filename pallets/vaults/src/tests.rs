use crate::{
	mock::{Vaults, *},
	pallet::{
		ArgonotSecuritizationByVaultId, BitcoinLockUpdate, NextVaultId,
		PendingTermsModificationsByTick, RevenuePerFrameByVault, RevenuePerFrameByVaultCount,
		VaultFundsReleasingByHeight, VaultXPubById, VaultsById,
	},
	Error, Event, HoldReason, LastCollectFrameByVaultId, OrphanedUtxoAccountsByVaultId,
	PendingCosignByVaultId, VaultConfig, VaultIdByOperator,
};
use argon_primitives::{
	bitcoin::{CompressedBitcoinPubkey, OpaqueBitcoinXpub, SATOSHIS_PER_BITCOIN},
	vault::{
		BitcoinLockFundingUpdate, BitcoinResecuritization, BitcoinSecuritization,
		BitcoinSecuritizationBasis, BitcoinVaultProvider, ReserveSecuritizationRequest, VaultError,
		VaultTerms,
	},
};
use bitcoin::{
	bip32::{ChildNumber, Xpriv, Xpub},
	key::Secp256k1,
};
use frame_support::traits::Hooks;
use k256::elliptic_curve::rand_core::{OsRng, RngCore};
use pallet_operational_accounts::{
	OpaqueEncryptionPubkey, OperationalAccount, OperationalAccountBySubAccount,
	OperationalAccounts as OperationalAccountsById,
};
use pallet_prelude::{
	argon_primitives::{
		vault::{LockExtension, TreasuryVaultProvider, VaultTreasuryFrameEarnings},
		OnNewSlot,
	},
	*,
};

const TEN_PCT: FixedU128 = FixedU128::from_rational(110, 100);

pub(crate) fn keys() -> OpaqueBitcoinXpub {
	let mut seed = [0u8; 32];
	OsRng.fill_bytes(&mut seed);

	let xpriv = Xpriv::new_master(GetBitcoinNetwork::get(), &seed).unwrap();
	let child = xpriv
		.derive_priv(
			&Secp256k1::new(),
			&[ChildNumber::from_normal_idx(0).unwrap(), ChildNumber::from_hardened_idx(1).unwrap()],
		)
		.unwrap();
	let xpub = Xpub::from_priv(&Secp256k1::new(), &child);
	OpaqueBitcoinXpub(xpub.encode())
}

fn default_terms(pct: FixedU128) -> VaultTerms<Balance> {
	VaultTerms { bitcoin_annual_percent_rate: pct, bitcoin_base_fee: 0 }
}

fn securitization(amount: Balance) -> BitcoinSecuritization<Balance> {
	BitcoinSecuritization {
		basis: BitcoinSecuritizationBasis {
			satoshis: amount.saturated_into(),
			microgons_at_target_per_btc: SATOSHIS_PER_BITCOIN.into(),
		},
		securitization_coverage_microgons: amount,
		securitization_ratio: FixedU128::one(),
	}
}

fn funding_update(
	securitization: &BitcoinSecuritization<Balance>,
	funded_satoshis: u64,
) -> BitcoinLockFundingUpdate<Balance> {
	BitcoinLockFundingUpdate {
		funded_satoshis,
		securitized_satoshis: securitization.securitized_satoshis(funded_satoshis),
		collateral_required: securitization.collateral_for_satoshis(funded_satoshis),
		eligible_satoshis: securitization.eligible_satoshis(funded_satoshis),
		is_flexible: false,
	}
}

fn standard_reservation_request() -> ReserveSecuritizationRequest<Balance> {
	ReserveSecuritizationRequest {
		lock_expiration: 100,
		fee_discount: 0,
		securitization_space_to_unreserve: 0,
	}
}

fn default_vault() -> VaultConfig<u64, Balance> {
	VaultConfig {
		terms: default_terms(TEN_PCT),
		delegate_account_id: None,
		bitcoin_xpubkey: keys(),
		securitization: 50_000,
		securitization_ratio: FixedU128::one(),
	}
}

#[test]
fn it_can_create_a_vault() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		assert_noop!(
			Vaults::create(RuntimeOrigin::signed(1), default_vault()),
			Error::<Test>::InsufficientFunds
		);

		set_argons(1, 100_010);

		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		System::assert_last_event(
			Event::VaultCreated {
				vault_id: 1,
				opened_tick: CurrentTick::get(),
				operator_account_id: 1,
				securitization: 50_000,
				securitization_ratio: FixedU128::one(),
			}
			.into(),
		);

		assert!(System::account_exists(&1));

		assert_eq!(Balances::reserved_balance(1), 50_000);
		assert_eq!(Balances::free_balance(1), 50_010);

		assert_eq!(NextVaultId::<Test>::get(), Some(2u32));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().operator_account_id, 1);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().delegate_account_id, None);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 50_000);
		assert_eq!(VaultIdByOperator::<Test>::get(1), Some(1u32));

		// user can't create a second vault
		assert_err!(
			Vaults::create(RuntimeOrigin::signed(1), default_vault()),
			Error::<Test>::AccountAlreadyHasVault
		);
	});
}

#[test]
fn open_vault_index_returns_the_largest_positions_and_full_total() {
	new_test_ext().execute_with(|| {
		for (operator, securitization) in [(1, 50_000), (2, 70_000), (3, 70_000)] {
			set_argons(operator, 200_000);
			let mut config = default_vault();
			config.securitization = securitization;
			assert_ok!(Vaults::create(RuntimeOrigin::signed(operator), config));
		}

		let (positions, total) = Vaults::get_top_vaults_by_securitization(2);
		assert_eq!(
			positions.iter().map(|position| position.vault_id).collect::<Vec<_>>(),
			vec![2, 3]
		);
		assert_eq!(total, 190_000);

		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 90_000, FixedU128::one()));
		let (positions, total) = Vaults::get_top_vaults_by_securitization(2);
		assert_eq!(
			positions.iter().map(|position| position.vault_id).collect::<Vec<_>>(),
			vec![1, 2]
		);
		assert_eq!(total, 230_000);

		assert_ok!(Vaults::close(RuntimeOrigin::signed(1), 1));
		let (positions, total) = Vaults::get_top_vaults_by_securitization(2);
		assert_eq!(
			positions.iter().map(|position| position.vault_id).collect::<Vec<_>>(),
			vec![2, 3]
		);
		assert_eq!(total, 140_000);
	});
}

#[test]
fn it_requires_operational_account_upgrade_when_invite_only() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		OperationalAccountsInviteOnly::set(true);
		set_argons(1, 100_010);

		assert_noop!(
			Vaults::create(RuntimeOrigin::signed(1), default_vault()),
			Error::<Test>::OperationalAccountRegistrationRequired
		);

		UpgradedOperationalAccounts::mutate(|accounts| {
			accounts.insert(1);
		});
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
	});
}

#[test]
fn it_can_create_a_vault_with_a_delegate_account() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let mut config = default_vault();
		config.delegate_account_id = Some(9);

		set_argons(1, 100_010);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));

		let vault = VaultsById::<Test>::get(1).expect("vault should exist");
		assert_eq!(vault.delegate_account_id, Some(9));
	});
}

#[test]
fn it_can_set_argonot_securitization_for_a_vault() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 120_000);

		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 10_000));

		System::assert_last_event(
			Event::ArgonotSecuritizationSet { vault_id: 1, operator_account_id: 1, amount: 10_000 }
				.into(),
		);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 10_000,);
		assert_eq!(<Vaults as BitcoinVaultProvider>::get_held_argonots(&1), Some(10_000));
		let commitment =
			ArgonotSecuritizationByVaultId::<Test>::get(1).expect("commitment should exist");
		assert_eq!(commitment.held_micronots, 10_000);
		assert_eq!(commitment.encumbered_micronots, 0);

		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 4_000));
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 4_000);
		assert_eq!(<Vaults as BitcoinVaultProvider>::get_held_argonots(&1), Some(4_000));
		let commitment =
			ArgonotSecuritizationByVaultId::<Test>::get(1).expect("commitment should exist");
		assert_eq!(commitment.held_micronots, 4_000);
		assert_eq!(commitment.encumbered_micronots, 0);
	});
}

#[test]
fn held_argonots_cannot_be_reduced_below_encumbered_backing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 120_000);

		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 10_000));
		assert_ok!(<Vaults as BitcoinVaultProvider>::encumber_argonots(&1, 6_000));

		assert_noop!(
			Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 5_999),
			Error::<Test>::ArgonotsBelowEncumberedBacking
		);
		assert_eq!(
			<Vaults as BitcoinVaultProvider>::release_encumbered_argonots(&1, 6_001),
			Err(VaultError::ArgonotsBelowEncumberedBacking)
		);
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 6_000));
		let commitment =
			ArgonotSecuritizationByVaultId::<Test>::get(1).expect("commitment should exist");
		assert_eq!(commitment.held_micronots, 6_000);
		assert_eq!(commitment.encumbered_micronots, 6_000);
	});
}

#[test]
fn reward_committed_argonots_earn_until_their_whole_withdrawal_is_due() {
	new_test_ext().execute_with(|| {
		set_argons(1, 120_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 10_000));
		<Vaults as TreasuryVaultProvider>::commit_securitization_for_rewards(1, 4_000);
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 2_000));
		let state = ArgonotSecuritizationByVaultId::<Test>::get(1).unwrap();
		assert_eq!(state.held_micronots, 4_000);
		assert_eq!(state.committed_micronots, 4_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 4_000);
		let vault = VaultsById::<Test>::get(1).unwrap();
		let (&height, entry) = vault.securitization_release_schedule.iter().next().unwrap();
		assert!(height >= LastBitcoinHeightChange::get().1 + 52_560);
		assert_eq!(height % 144, 0);
		assert_eq!(entry.argonot_withdrawals, 2_000);
		assert_eq!(
			<Vaults as TreasuryVaultProvider>::get_top_vaults_by_securitization(1).0[0]
				.securitization_micronots,
			4_000
		);

		LastBitcoinHeightChange::set((height - 1, height - 1));
		<Vaults as TreasuryVaultProvider>::commit_securitization_for_rewards(1, 2_000);
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 2_000));
		Vaults::on_initialize(2);
		assert_eq!(
			ArgonotSecuritizationByVaultId::<Test>::get(1).unwrap().committed_micronots,
			4_000
		);
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_release_schedule,
			vault.securitization_release_schedule
		);

		LastBitcoinHeightChange::set((height, height));
		Vaults::on_initialize(3);
		let state = ArgonotSecuritizationByVaultId::<Test>::get(1).unwrap();
		assert_eq!(state.held_micronots, 2_000);
		assert_eq!(state.committed_micronots, 2_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 2_000);
		assert!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule.is_empty());
	});
}

#[test]
fn argonot_funding_increases_cancel_newest_withdrawals_without_restarting_older_notices() {
	new_test_ext().execute_with(|| {
		set_argons(1, 120_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 10_000));
		<Vaults as TreasuryVaultProvider>::commit_securitization_for_rewards(1, 10_000);
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 7_000));
		let first_height = *VaultsById::<Test>::get(1)
			.unwrap()
			.securitization_release_schedule
			.keys()
			.next()
			.unwrap();
		let (_, current_height) = LastBitcoinHeightChange::get();
		LastBitcoinHeightChange::set((current_height + 144, current_height + 144));
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 5_000));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule.len(), 2);

		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 8_000));
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization_release_schedule.len(), 1);
		assert_eq!(vault.securitization_release_schedule[&first_height].argonot_withdrawals, 2_000);
		LastBitcoinHeightChange::set((first_height, first_height));
		Vaults::on_initialize(2);
		let backing = ArgonotSecuritizationByVaultId::<Test>::get(1).unwrap();
		assert_eq!(backing.held_micronots, 8_000);
		assert_eq!(backing.committed_micronots, 8_000);
	});
}

#[test]
fn argonot_withdrawals_wait_for_encumbered_backing_without_partial_refunds() {
	new_test_ext().execute_with(|| {
		set_argons(1, 120_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 10_000));
		<Vaults as TreasuryVaultProvider>::commit_securitization_for_rewards(1, 10_000);
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 6_000));
		assert_ok!(<Vaults as BitcoinVaultProvider>::encumber_argonots(&1, 8_000));
		let height = *VaultsById::<Test>::get(1)
			.unwrap()
			.securitization_release_schedule
			.keys()
			.next()
			.unwrap();
		LastBitcoinHeightChange::set((height, height));
		Vaults::on_initialize(2);
		assert_eq!(ArgonotSecuritizationByVaultId::<Test>::get(1).unwrap().held_micronots, 10_000);
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_release_schedule[&height]
				.argonot_withdrawals,
			4_000
		);
		assert!(VaultFundsReleasingByHeight::<Test>::get(height + 144).contains(&1));

		assert_ok!(<Vaults as BitcoinVaultProvider>::release_encumbered_argonots(&1, 2_000));
		LastBitcoinHeightChange::set((height + 144, height + 144));
		Vaults::on_initialize(3);
		let backing = ArgonotSecuritizationByVaultId::<Test>::get(1).unwrap();
		assert_eq!(backing.held_micronots, 6_000);
		assert_eq!(backing.committed_micronots, 6_000);
		assert_eq!(backing.encumbered_micronots, 6_000);
		assert!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule.is_empty());
	});
}

#[test]
fn burning_argonot_backing_reduces_commitments_and_unfunded_withdrawals() {
	new_test_ext().execute_with(|| {
		set_argons(1, 120_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 10_000));
		<Vaults as TreasuryVaultProvider>::commit_securitization_for_rewards(1, 8_000);
		assert_ok!(Vaults::set_argonot_securitization(RuntimeOrigin::signed(1), 0));
		assert_ok!(<Vaults as BitcoinVaultProvider>::encumber_argonots(&1, 5_000));
		assert_ok!(<Vaults as BitcoinVaultProvider>::burn_encumbered_argonots(&1, 5_000));
		let backing = ArgonotSecuritizationByVaultId::<Test>::get(1).unwrap();
		assert_eq!(backing.held_micronots, 3_000);
		assert_eq!(backing.committed_micronots, 3_000);
		assert_eq!(backing.encumbered_micronots, 0);
		let vault = VaultsById::<Test>::get(1).unwrap();
		let (&height, entry) = vault.securitization_release_schedule.iter().next().unwrap();
		assert_eq!(entry.argonot_withdrawals, 3_000);
		LastBitcoinHeightChange::set((height, height));
		Vaults::on_initialize(2);
		assert_eq!(<Vaults as BitcoinVaultProvider>::get_held_argonots(&1), Some(0));
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 0);
		assert!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule.is_empty());
	});
}

#[test]
fn committed_securitization_includes_relock_capacity() {
	new_test_ext().execute_with(|| {
		set_argons(1, 120_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));

		VaultsById::<Test>::mutate(1, |vault| {
			let vault = vault.as_mut().expect("vault should exist");
			vault.securitization_locked = 40_000;
			vault.securitization_pending_activation = 10_000;
			vault
				.scheduled_release(100)
				.expect("release schedule stays within test bounds")
				.relockable_commitments = 7_500;
		});

		assert_eq!(
			<Vaults as BitcoinVaultProvider>::get_committed_securitization(&1, 10),
			Some(37_500)
		);
		assert_ok!(<Vaults as TreasuryVaultProvider>::commit_securitization_for_bonds(1, 40_000));
		assert_eq!(Vaults::get_committed_securitization(&1, 10), Some(40_000));
	});
}

#[test]
fn bitcoin_height_after_tick_range_rounds_up_partial_blocks() {
	new_test_ext().execute_with(|| {
		assert_eq!(Vaults::bitcoin_height_after_tick_range(11, 0), Some(11));
		assert_eq!(Vaults::bitcoin_height_after_tick_range(11, 1), Some(12));
		assert_eq!(Vaults::bitcoin_height_after_tick_range(11, 10), Some(12));
		assert_eq!(Vaults::bitcoin_height_after_tick_range(11, 11), Some(13));
	});
}

#[test]
fn committed_securitization_excludes_release_schedule_inside_the_commitment_window() {
	new_test_ext().execute_with(|| {
		set_argons(1, 120_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		CurrentTick::set(5);
		NextSlot::set(10);
		LastBitcoinHeightChange::set((136, 137));

		VaultsById::<Test>::mutate(1, |vault| {
			let vault = vault.as_mut().expect("vault should exist");
			vault.securitization_locked = 40_000;
			vault.securitization_pending_activation = 10_000;
			vault
				.scheduled_release(144)
				.expect("release schedule stays within test bounds")
				.relockable_commitments = 4_000;
			vault
				.scheduled_release(288)
				.expect("release schedule stays within test bounds")
				.relockable_commitments = 7_500;
		});

		assert_eq!(
			<Vaults as BitcoinVaultProvider>::get_committed_securitization(&1, 7),
			Some(37_500)
		);
		<Vaults as TreasuryVaultProvider>::commit_securitization_for_rewards(1, 0);
		CurrentFrameId::set(2);
		VaultsById::<Test>::mutate(1, |vault| {
			let vault = vault.as_mut().unwrap();
			vault.request_securitization_exit(500, 143).unwrap();
			vault.request_securitization_exit(500, 144).unwrap();
			vault.request_securitization_exit(41_000, 288).unwrap();
			vault.securitization_target = 8_000;
		});
		assert_eq!(Vaults::get_committed_securitization(&1, 7), Some(49_000));
	});
}

#[test]
fn it_can_set_securitization_ratio_for_a_vault() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let mut config = default_vault();
		config.securitization_ratio = TEN_PCT;
		set_argons(1, 110_010);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));

		System::assert_last_event(
			Event::VaultCreated {
				vault_id: 1,
				opened_tick: CurrentTick::get(),
				operator_account_id: 1,
				securitization: 50_000,
				securitization_ratio: TEN_PCT,
			}
			.into(),
		);
		assert!(System::account_exists(&1));
		assert_eq!(Balances::reserved_balance(1), 50_000);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 50_000);
		assert_eq!(vault.operator_account_id, 1);
		assert_eq!(vault.securitization_ratio, TEN_PCT);
		// uses 10% for recovery
		assert_eq!(vault.available_securitization_space(true), 50_000);
	});
}

#[test]
fn it_can_set_a_vault_delegate_account() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		set_argons(1, 100_010);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));

		assert_err!(
			Vaults::set_delegate_account(RuntimeOrigin::signed(2), Some(9)),
			Error::<Test>::VaultNotFound
		);

		assert_ok!(Vaults::set_delegate_account(RuntimeOrigin::signed(1), Some(9)));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().delegate_account_id, Some(9));

		assert_ok!(Vaults::set_delegate_account(RuntimeOrigin::signed(1), None));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().delegate_account_id, None);
	});
}

#[test]
fn operator_can_reserve_securitization_space_for_own_vault() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 100_010);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		VaultsById::<Test>::mutate(1, |vault| {
			let vault = vault.as_mut().expect("vault");
			vault.delegate_account_id = Some(3);
		});

		assert_noop!(
			Vaults::set_reserved_securitization_space(RuntimeOrigin::signed(2), 50),
			Error::<Test>::VaultNotFound
		);
		assert_noop!(
			Vaults::set_reserved_securitization_space(RuntimeOrigin::signed(3), 50),
			Error::<Test>::VaultNotFound
		);
		assert_noop!(
			Vaults::set_reserved_securitization_space(RuntimeOrigin::signed(1), 50_001),
			Error::<Test>::InsufficientVaultFunds
		);
		assert_ok!(Vaults::set_reserved_securitization_space(RuntimeOrigin::signed(1), 50_000,));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().reserved_securitization_space, 50_000);
		System::assert_last_event(
			Event::ReservedSecuritizationSpaceChanged {
				vault_id: 1,
				reserved_securitization_space: 50_000,
			}
			.into(),
		);
	});
}

#[test]
fn operator_resecuritization_uses_releasing_funds_before_flexible_space() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 100);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig { securitization: 100, ..default_vault() }
		));
		VaultsById::<Test>::mutate(1, |vault| {
			let vault = vault.as_mut().expect("vault");
			vault.securitization_locked = 60;
			vault.flexible_securitization_locked = 40;
			vault.total_satoshis = 20;
			vault.securitized_satoshis = 20;
			vault.ratio_adjusted_satoshis = 20;
			vault.scheduled_release(288).unwrap().relockable_commitments = 20;
		});

		let current = securitization(20);
		let replacement = securitization(60);
		let mut lock_extension = LockExtension::new(143);
		assert_ok!(<Vaults as BitcoinVaultProvider>::resecuritize(
			1,
			&1,
			BitcoinResecuritization {
				current: &current,
				replacement: &replacement,
				funded_satoshis: 20,
				remaining_term: FixedU128::one(),
				lock_extension: &mut lock_extension,
				is_flexible: false,
				fee_discount: 0,
				securitization_space_to_unreserve: 0,
			},
		));

		let vault = VaultsById::<Test>::get(1).expect("vault");
		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.flexible_securitization_locked, 40);
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(lock_extension.get(&288), Some(&20));
	});
}

#[test]
fn it_will_reject_non_hardened_xpubs() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let mut config = default_vault();
		let mut seed = [0u8; 32];
		OsRng.fill_bytes(&mut seed);
		let network = GetBitcoinNetwork::get();
		let xpriv = Xpriv::new_master(network, &seed).unwrap();
		let child = xpriv
			.derive_priv(&Secp256k1::new(), &[ChildNumber::from_normal_idx(0).unwrap()])
			.unwrap();
		let xpub = Xpub::from_priv(&Secp256k1::new(), &child);

		config.bitcoin_xpubkey = OpaqueBitcoinXpub(xpub.encode());
		set_argons(1, 110_010);
		assert_noop!(
			Vaults::create(RuntimeOrigin::signed(1), config),
			Error::<Test>::UnsafeXpubkey
		);
	});
}

#[test]
fn it_can_modify_a_vault_funds() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let mut config = default_vault();
		config.securitization = 2000;

		set_argons(1, 20_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));
		assert_eq!(Balances::reserved_balance(1), 2000);

		assert_noop!(
			Vaults::modify_funding(RuntimeOrigin::signed(2), 1, 1000, FixedU128::from_float(2.0)),
			Error::<Test>::NoPermissions
		);

		assert_ok!(Vaults::modify_funding(
			RuntimeOrigin::signed(1),
			1,
			1000,
			FixedU128::from_float(2.0)
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_ratio,
			FixedU128::from_float(2.0)
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().available_securitization_space(true), 1000);
		System::assert_last_event(
			Event::VaultModified {
				vault_id: 1,
				securitization: 1000,
				securitization_target: 1000,
				securitization_ratio: FixedU128::from_float(2.0),
			}
			.into(),
		);
		assert_eq!(Balances::reserved_balance(1), 1000);

		assert_ok!(Vaults::modify_funding(
			RuntimeOrigin::signed(1),
			1,
			2000,
			FixedU128::from_float(2.0)
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_ratio,
			FixedU128::from_float(2.0)
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 2000);

		assert_ok!(Vaults::set_reserved_securitization_space(RuntimeOrigin::signed(1), 1500,));
		assert_ok!(Vaults::modify_funding(
			RuntimeOrigin::signed(1),
			1,
			0,
			FixedU128::from_float(2.0)
		));
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 1500);
		assert_eq!(vault.securitization_target, 0);
		assert_eq!(vault.available_securitization_space(true), 0);
		assert_eq!(Balances::reserved_balance(1), 1500);
	});
}

#[test]
fn reward_committed_argons_require_notice_but_new_funds_can_leave() {
	new_test_ext().execute_with(|| {
		set_argons(1, 10_000);
		let mut config = default_vault();
		config.securitization = 1_000;
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().committed_microgons, 0);

		<Vaults as TreasuryVaultProvider>::commit_securitization_for_rewards(1, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().committed_microgons, 1_000);

		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 1_500, FixedU128::one()));
		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 1_000, FixedU128::one()));
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 1_000);
		assert_eq!(vault.committed_microgons, 1_000);
		assert_eq!(vault.exit_notice_amount(), 0);

		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 500, FixedU128::one()));
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 1_000);
		assert_eq!(vault.committed_microgons, 1_000);
		assert_eq!(vault.exit_notice_amount(), 500);

		let exit_height = *vault.securitization_release_schedule.keys().next().unwrap();
		LastBitcoinHeightChange::set((exit_height, exit_height));
		Vaults::on_initialize(2);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 500);
		assert_eq!(vault.committed_microgons, 500);
	});
}

#[test]
fn regular_bonds_require_notice_before_the_first_reward_snapshot() {
	new_test_ext().execute_with(|| {
		set_argons(1, 10_000);
		let mut config = default_vault();
		config.securitization = 1_000;
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().committed_microgons, 0);

		assert_ok!(<Vaults as TreasuryVaultProvider>::commit_securitization_for_bonds(1, 600));
		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 0, FixedU128::one()));
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 600);
		assert_eq!(vault.exit_notice_amount(), 600);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 600);

		let exit_height = *vault.securitization_release_schedule.keys().next().unwrap();
		LastBitcoinHeightChange::set((exit_height, exit_height));
		Vaults::on_initialize(2);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 0);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 0);
	});
}

#[test]
fn exit_notice_keeps_other_securitization_available() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 10_000);
		set_argons(2, 10_000);
		let mut config = default_vault();
		config.securitization = 1_000;
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		let collateral = securitization(500);
		assert_ok!(Vaults::reserve_securitization(
			1,
			&2,
			&collateral,
			ReserveSecuritizationRequest { lock_expiration: 365, ..standard_reservation_request() },
		));
		assert_ok!(Vaults::record_bitcoin_lock_funding(1, funding_update(&collateral, 500)));

		<Vaults as TreasuryVaultProvider>::commit_securitization_for_rewards(1, 0);
		CurrentFrameId::set(2);
		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 500, FixedU128::one()));
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 1_000);
		assert_eq!(vault.exit_notice_amount(), 500);
		assert_eq!(vault.available_securitization_space(true), 500);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 1_000);

		let exit_height = *vault
			.securitization_release_schedule
			.iter()
			.find(|(_, entry)| !entry.argon_withdrawals.is_zero())
			.unwrap()
			.0;
		assert_ok!(Vaults::reserve_securitization(
			1,
			&2,
			&collateral,
			ReserveSecuritizationRequest {
				lock_expiration: exit_height + 144,
				..standard_reservation_request()
			},
		));
		assert_ok!(Vaults::record_bitcoin_lock_funding(1, funding_update(&collateral, 500)));
		assert_ok!(Vaults::release_bitcoin_lock_securitization(
			1,
			&collateral,
			500,
			&LockExtension::new(365),
			false,
		));
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization_locked, 500);
		assert_eq!(vault.exit_notice_amount(), 500);
		assert!(exit_height >= LastBitcoinHeightChange::get().1 + 52_560);
		assert_eq!(vault.securitization_release_schedule[&exit_height].argon_withdrawals, 500);
		assert_eq!(vault.available_securitization_space(true), 500);

		LastBitcoinHeightChange::set((exit_height, exit_height));
		Vaults::on_initialize(2);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 500);
		assert_eq!(vault.securitization_locked, 500);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 500);
	});
}

#[test]
fn blocked_withdrawals_remain_whole_and_retry_after_capacity_is_freed() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 10_000);
		let mut config = default_vault();
		config.securitization = 1_000;
		config.terms = default_terms(FixedU128::zero());
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		assert_ok!(Vaults::reserve_securitization(
			1,
			&2,
			&securitization(750),
			standard_reservation_request()
		));
		<Vaults as TreasuryVaultProvider>::commit_securitization_for_rewards(1, 0);
		CurrentFrameId::set(2);
		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 500, FixedU128::one()));
		let exit_height = *VaultsById::<Test>::get(1)
			.unwrap()
			.securitization_release_schedule
			.iter()
			.find(|(_, entry)| !entry.argon_withdrawals.is_zero())
			.unwrap()
			.0;
		LastBitcoinHeightChange::set((exit_height, exit_height));
		Vaults::on_initialize(2);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization_release_schedule[&exit_height].argon_withdrawals, 500);
		assert_eq!(vault.securitization, 1_000);
		assert_eq!(crate::TotalVaultSecuritization::<Test>::get(), 1_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 1_000);
		let retry_height = exit_height + 144;
		assert!(VaultFundsReleasingByHeight::<Test>::get(retry_height).contains(&1));

		assert_ok!(Vaults::release_unactivated_securitization(1, 500, &LockExtension::new(100), 0));
		assert_ok!(Balances::release(&HoldReason::EnterVault.into(), &1, 600, Precision::Exact));
		let events_before_failure = System::events().len();
		LastBitcoinHeightChange::set((retry_height, retry_height));
		Vaults::on_initialize(3);
		let unchanged = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(unchanged.securitization, 1_000);
		assert_eq!(unchanged.securitization_release_schedule[&exit_height].argon_withdrawals, 500);
		assert_eq!(crate::TotalVaultSecuritization::<Test>::get(), 1_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 400);
		assert_eq!(System::events().len(), events_before_failure + 1);
		assert!(matches!(
			System::events().last().unwrap().event,
			RuntimeEvent::Vaults(Event::FundsReleasedError { vault_id: 1, .. })
		));
		assert!(VaultFundsReleasingByHeight::<Test>::get(retry_height + 1).contains(&1));

		assert_ok!(Balances::hold(&HoldReason::EnterVault.into(), &1, 600));
		LastBitcoinHeightChange::set((retry_height + 1, retry_height + 1));
		Vaults::on_initialize(4);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 500);
		assert_eq!(vault.exit_notice_amount(), 0);
		assert_eq!(crate::TotalVaultSecuritization::<Test>::get(), 500);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 500);
	});
}

#[test]
fn raising_the_target_cancels_the_unmatured_exit() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 10_000);
		let mut config = default_vault();
		config.securitization = 1_000;
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		<Vaults as TreasuryVaultProvider>::commit_securitization_for_rewards(1, 0);
		CurrentFrameId::set(2);

		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 500, FixedU128::one()));
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.exit_notice_amount(), 500);
		assert_eq!(vault.available_securitization_space(false), 1_000);

		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 750, FixedU128::one()));
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 1_000);
		assert_eq!(vault.exit_notice_amount(), 250);
		assert_eq!(vault.available_securitization_space(false), 1_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 1_000);

		let exit_height = *vault
			.securitization_release_schedule
			.iter()
			.find(|(_, entry)| !entry.argon_withdrawals.is_zero())
			.unwrap()
			.0;
		LastBitcoinHeightChange::set((exit_height, exit_height));
		Vaults::on_initialize(2);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 750);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 750);
	});
}

#[test]
fn uncommitted_funds_do_not_count_as_committed_by_a_cancellable_notice() {
	new_test_ext().execute_with(|| {
		set_argons(1, 10_000);
		set_argons(2, 10_000);
		let mut config = default_vault();
		config.securitization = 1_000;
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		assert_ok!(Vaults::reserve_securitization(
			1,
			&2,
			&securitization(1_000),
			standard_reservation_request()
		));
		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 500, FixedU128::one()));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().exit_notice_amount(), 500);
		assert_eq!(Vaults::get_committed_securitization(&1, 0), Some(0));

		assert_ok!(Vaults::release_unactivated_securitization(
			1,
			1_000,
			&LockExtension::new(100),
			0
		));
		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 1_000, FixedU128::one()));
		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 0, FixedU128::one()));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 0);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 0);
	});
}

#[test]
fn operational_activation_preserves_existing_commitments_and_withdrawal_notices() {
	for committed in [0, 4_000] {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			set_argons(1, 10_000);
			let mut config = default_vault();
			config.securitization = 4_000;
			assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
			if committed != 0 {
				<Vaults as TreasuryVaultProvider>::commit_securitization_for_rewards(1, 0);
				assert_ok!(Vaults::modify_funding(
					RuntimeOrigin::signed(1),
					1,
					3_000,
					FixedU128::one()
				));
			}
			let before = VaultsById::<Test>::get(1).unwrap();
			assert_eq!(before.committed_microgons, committed);

			OperationalAccountsById::<Test>::insert(
				2,
				OperationalAccount {
					vault_account: 1,
					mining_account: 3,
					encryption_pubkey: OpaqueEncryptionPubkey([0; 32]),
					upstream_account: None,
					name: None,
					last_name_change_tick: None,
					uniswap_argon_transfers_in_amount: 0,
					account_bitcoin_amount: MinimumBitcoin::get(),
					account_vault_bond_amount: MinimumBonds::get(),
					vault_created: true,
					vault_bitcoin_accrual: 0,
					vault_bitcoin_applied_total: 0,
					mining_seat_accrual: MiningSeatsForOperational::get(),
					mining_seat_applied_total: 0,
					operational_certifications_count: 0,
					available_access_codes: 0,
					rewards_earned_count: 0,
					rewards_earned_amount: 0,
					rewards_collected_amount: 0,
					is_operationally_certified: false,
				},
			);
			OperationalAccountBySubAccount::<Test>::insert(3, 2);

			assert_ok!(OperationalAccounts::activate(RuntimeOrigin::signed(3)));
			let account = OperationalAccountsById::<Test>::get(2).unwrap();
			assert!(account.is_operationally_certified);
			let vault = VaultsById::<Test>::get(1).unwrap();
			assert_eq!(vault.committed_microgons, committed.max(2_000));
			assert_eq!(vault.securitization, before.securitization);
			assert_eq!(
				vault.securitization_release_schedule,
				before.securitization_release_schedule
			);
		});
	}
}

#[test]
fn operational_certification_uses_the_normal_securitization_exit_notice() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let mut config = default_vault();
		config.securitization = 2_500;
		set_argons(1, 10_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		CurrentTick::set(1_440 * 366);
		assert_eq!(Vaults::get_committed_securitization(&1, 10), Some(0));
		<Vaults as BitcoinVaultProvider>::account_became_operational(&1);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().committed_microgons, 2_000);
		assert_eq!(Vaults::get_committed_securitization(&1, 10), Some(2_000));

		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 1_000, FixedU128::one()));
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 2_000);
		assert_eq!(vault.exit_notice_amount(), 1_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 2_000);
		assert_eq!(Vaults::get_committed_securitization(&1, 10), Some(2_000));

		assert_ok!(Vaults::close(RuntimeOrigin::signed(1), 1));

		let vault = VaultsById::<Test>::get(1).expect("vault should exist");
		assert!(vault.is_closed);
		assert_eq!(vault.securitization_target, 0);
		assert_eq!(vault.securitization, OperationalMinimumVaultSecuritization::get());
		assert_eq!(vault.exit_notice_amount(), 2_000);
		let exit_height = *vault
			.securitization_release_schedule
			.iter()
			.find(|(_, entry)| !entry.argon_withdrawals.is_zero())
			.unwrap()
			.0;
		LastBitcoinHeightChange::set((exit_height - 1, exit_height - 1));
		Vaults::on_initialize(1);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 2_000);
		LastBitcoinHeightChange::set((exit_height, exit_height));
		assert_eq!(Vaults::get_committed_securitization(&1, 10), Some(0));
		Vaults::on_initialize(2);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization, 0);
		assert_eq!(vault.committed_microgons, 0);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 0);
	});
}

#[test]
fn it_can_reduce_vault_funds_down_to_activated() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		set_argons(1, 20_000);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms: default_terms(TEN_PCT),
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 1000,
				securitization_ratio: FixedU128::from_float(2.0),
			}
		));
		assert_eq!(Balances::reserved_balance(1), 1000);

		VaultsById::<Test>::mutate(1, |vault| {
			if let Some(vault) = vault {
				vault.securitization_locked = 998;
			}
		});
		// amount eligible for mining is 2x the bitcoin argons (+2x), but capped at the 1000 which
		// have been securitization
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_activated_securitization(), 998);

		assert_ok!(Vaults::modify_funding(
			RuntimeOrigin::signed(1),
			1,
			997,
			FixedU128::from_float(2.0)
		));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 998);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_target, 997);
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_ratio,
			FixedU128::from_float(2.0)
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().available_securitization_space(true), 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().uninhibited_securitization(), 0);

		System::assert_last_event(
			Event::VaultModified {
				vault_id: 1,
				securitization: 998,
				securitization_target: 997,
				securitization_ratio: FixedU128::from_float(2.0),
			}
			.into(),
		);
		// should have returned the difference
		assert_eq!(Balances::reserved_balance(1), 998);

		// should now return funds once the locked securitization goes down
		VaultsById::<Test>::mutate(1, |vault| {
			if let Some(vault) = vault {
				vault.securitization_locked = 500;
				vault.scheduled_release(100).unwrap().relockable_commitments = 498;
			}
		});
		VaultFundsReleasingByHeight::<Test>::mutate(100, |a| {
			let _ = a.try_insert(1);
		});
		LastBitcoinHeightChange::set((100, 100));
		Vaults::on_initialize(2);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 998);
		let exit_height = *VaultsById::<Test>::get(1)
			.unwrap()
			.securitization_release_schedule
			.iter()
			.find(|(_, entry)| !entry.argon_withdrawals.is_zero())
			.unwrap()
			.0;
		LastBitcoinHeightChange::set((exit_height, exit_height));
		Vaults::on_initialize(3);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 997);
		assert_eq!(Balances::reserved_balance(1), 997, "should shrink after the notice");
	});
}

#[test]
fn it_can_close_a_vault() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let vault_owner_balance = 201_000;
		set_argons(1, vault_owner_balance);
		set_argons(2, 100_000);
		let mut terms = default_terms(FixedU128::from_float(0.01));
		terms.bitcoin_base_fee = 1;
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 100_000,
				securitization_ratio: FixedU128::from_float(2.0),
			}
		));
		assert_eq!(Balances::free_balance(1), 101_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 100_000);

		let amount = 40_000;
		let (fee, _) = Vaults::reserve_securitization(
			1,
			&2,
			&securitization(amount),
			standard_reservation_request(),
		)
		.expect("bonding failed");
		assert_eq!(fee, 401);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization_locked, 40_000);
		assert_eq!(vault.securitization, 100_000);

		assert_ok!(Vaults::close(RuntimeOrigin::signed(1), 1));
		// only need to preserve 2x
		System::assert_last_event(
			Event::VaultClosed {
				vault_id: 1,
				securitization_remaining: 40_000,
				securitization_released: 60_000,
			}
			.into(),
		);
		assert_eq!(Balances::free_balance(1), vault_owner_balance - 40_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PendingCollect.into(), &1), fee);
		assert!(VaultsById::<Test>::get(1).unwrap().is_closed);

		// set to full fee block
		CurrentTick::set(1440 * 365 + 1);
		// now when we return the securitization, it should return the funds to the vault
		assert_ok!(Vaults::release_unactivated_securitization(
			1,
			amount,
			&LockExtension::new(100),
			0
		));
		assert_eq!(Balances::free_balance(1), vault_owner_balance - amount);
		let exit_height = *VaultsById::<Test>::get(1)
			.unwrap()
			.securitization_release_schedule
			.iter()
			.find(|(_, entry)| !entry.argon_withdrawals.is_zero())
			.unwrap()
			.0;
		LastBitcoinHeightChange::set((exit_height, exit_height));
		Vaults::on_initialize(2);
		assert_eq!(Balances::free_balance(1), vault_owner_balance);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PendingCollect.into(), &1), fee);
		assert_eq!(Balances::free_balance(2), 100_000 - fee);
		assert_err!(
			Vaults::reserve_securitization(
				1,
				&2,
				&securitization(1000),
				standard_reservation_request(),
			),
			VaultError::VaultClosed
		);
	});
}

#[test]
fn it_can_lock_funds() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		let mut terms = default_terms(FixedU128::from_float(0.01));
		terms.bitcoin_base_fee = 1000;
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 500_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		assert_eq!(Balances::free_balance(1), 500_000);

		set_argons(2, 6_000);
		let (fee, _) = Vaults::reserve_securitization(
			1,
			&2,
			&securitization(500_000),
			standard_reservation_request(),
		)
		.expect("bonding failed");

		let apr_fee = (0.01f64 * 500_000f64) as u128;
		assert_eq!(fee, apr_fee + 1000);
		assert_eq!(Balances::free_balance(2), 6_000 - fee);
		assert_eq!(Balances::free_balance(1), 500_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PendingCollect.into(), &1), fee);
		// if we return the securitization, the fee won't be returned
		assert_ok!(Vaults::release_unactivated_securitization(
			1,
			500_000,
			&LockExtension::new(100),
			0
		));
		assert_eq!(Balances::free_balance(1), 500_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PendingCollect.into(), &1), fee);
		assert_eq!(Balances::free_balance(2), 6_000 - fee);

		let current_frame_id = CurrentFrameId::get();
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 1);
		assert_eq!(RevenuePerFrameByVaultCount::<Test>::get(), 1);
		assert_eq!(vault_revenue[0].frame_id, current_frame_id);
		assert_eq!(vault_revenue[0].bitcoin_lock_fee_revenue, fee);
		assert_eq!(vault_revenue[0].bitcoin_locks_new_securitization, 500_000);
		assert_eq!(vault_revenue[0].bitcoin_locks_added_satoshis, 0);
		assert_eq!(vault_revenue[0].bitcoin_locks_created, 1);
	});
}

#[test]
fn lock_saturates_securitization_space_release() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 100);
		set_argons(2, 100);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig { securitization: 100, ..default_vault() }
		));
		VaultsById::<Test>::mutate(1, |vault| {
			let vault = vault.as_mut().expect("vault");
			vault.securitization_locked = 100;
			vault.flexible_securitization_locked = 100;
			vault.reserved_securitization_space = 100;
		});

		assert_ok!(Vaults::reserve_securitization(
			1,
			&2,
			&securitization(40),
			ReserveSecuritizationRequest {
				lock_expiration: 100,
				securitization_space_to_unreserve: 150,
				..standard_reservation_request()
			},
		));

		let vault = VaultsById::<Test>::get(1).expect("vault");
		assert_eq!(vault.reserved_securitization_space, 0);
		assert_eq!(vault.securitization_locked, 140);
		assert_eq!(vault.securitization_pending_activation, 40);
	});
}

#[test]
fn it_doesnt_charge_lock_fees_to_operator() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		let mut terms = default_terms(FixedU128::from_float(0.01));
		terms.bitcoin_base_fee = 1000;
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 500_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		assert_eq!(Balances::free_balance(1), 500_000);

		let (fee, fee_discount) = Vaults::reserve_securitization(
			1,
			&1,
			&securitization(500_000),
			standard_reservation_request(),
		)
		.expect("bonding failed");

		assert_eq!(Balances::free_balance(1), 500_000);
		assert_eq!(fee, 6000);
		assert_eq!(fee_discount, fee);

		let current_frame_id = CurrentFrameId::get();
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 1);
		assert_eq!(vault_revenue[0].frame_id, current_frame_id);
		assert_eq!(vault_revenue[0].bitcoin_lock_fee_revenue, fee);
		assert_eq!(vault_revenue[0].bitcoin_lock_fee_coupon_value_used, fee_discount);
		assert_eq!(vault_revenue[0].bitcoin_locks_new_securitization, 500_000);
		assert_eq!(vault_revenue[0].bitcoin_locks_added_satoshis, 0);
		assert_eq!(vault_revenue[0].bitcoin_locks_created, 1);
	});
}

#[test]
fn fee_discount_is_capped_at_the_lock_fee() {
	new_test_ext().execute_with(|| {
		System::set_block_number(5);
		set_argons(1, 1_000_000);
		set_argons(2, 0);

		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		let (fee, fee_discount) = Vaults::reserve_securitization(
			1,
			&2,
			&securitization(50_000),
			ReserveSecuritizationRequest {
				lock_expiration: 100,
				fee_discount: 100_000,
				..standard_reservation_request()
			},
		)
		.expect("bonding failed");

		assert_eq!(fee, 55_000);
		assert_eq!(fee_discount, fee);
		assert_eq!(Balances::free_balance(2), 0);

		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1);
		assert_eq!(vault_revenue[0].bitcoin_lock_fee_revenue, fee);
		assert_eq!(vault_revenue[0].bitcoin_lock_fee_coupon_value_used, fee);
		assert_eq!(vault_revenue[0].uncollected_revenue, 0);
	});
}

#[test]
fn it_handles_overflowing_metrics() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);
		for i in 1..=10 {
			let account_id = i as u64;
			let vault = VaultConfig {
				terms: default_terms(TEN_PCT),
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 10_000 + (i * 1000) as Balance,
				securitization_ratio: FixedU128::one(),
			};
			set_argons(account_id, 100_000_000);
			assert_ok!(Vaults::create(RuntimeOrigin::signed(account_id), vault.clone()));
			Balances::set_on_hold(&HoldReason::PendingCollect.into(), &account_id, 10_000_000)
				.map_err(|_| VaultError::UnrecoverableHold)
				.unwrap();
		}
		for frame_id in 1..50 {
			CurrentFrameId::set(frame_id);
			Vaults::on_frame_start(frame_id);
			if frame_id == 42 {
				VaultsById::<Test>::mutate(2, |vault| {
					if let Some(v) = vault {
						v.securitization_locked = 1_000_000;
						v.securitization = 1_000_000;
					}
				});
			}
			for vault_id in 1..=10 {
				for _ in 0..100 {
					Vaults::update_vault_bitcoin_metrics(BitcoinLockUpdate {
						vault_id,
						total_fee: 1000,
						fee_discount: 0,
						locks_created: 1,
						securitization_released: 0,
						total_satoshis_added: 1000,
						total_satoshis_released: 0,
						securitization_locked: 10_000,
					})
					.unwrap();
				}
			}
		}
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 10);
		assert_eq!(vault_revenue[0].frame_id, 49);
		assert_eq!(vault_revenue[0].bitcoin_lock_fee_revenue, 1000 * 100);
		assert_eq!(vault_revenue[0].bitcoin_locks_new_securitization, 10_000 * 100);
		assert_eq!(vault_revenue[0].bitcoin_locks_added_satoshis, 1000 * 100);
		assert_eq!(vault_revenue[0].bitcoin_locks_created, 100);
		assert_eq!(vault_revenue[0].securitization, 11000);

		assert_eq!(vault_revenue[9].frame_id, 40);
		assert_eq!(vault_revenue[9].bitcoin_lock_fee_revenue, 1000 * 100);
		assert_eq!(vault_revenue[9].bitcoin_locks_new_securitization, 10_000 * 100);

		let vault_revenue_2 = RevenuePerFrameByVault::<Test>::get(2).to_vec();
		assert_eq!(vault_revenue_2.len(), 10);
		assert_eq!(vault_revenue_2[0].frame_id, 49);
		assert_eq!(vault_revenue_2[0].securitization, 1_000_000);
		assert_eq!(vault_revenue_2[0].securitization_activated, 1_000_000);

		assert_eq!(vault_revenue_2[8].frame_id, 41);
		assert_eq!(vault_revenue_2[8].securitization, 12_000);
		assert_eq!(vault_revenue_2[8].securitization_activated, 0); // only set manually in this test

		assert_eq!(vault_revenue_2[7].frame_id, 42);
		assert_eq!(vault_revenue_2[7].securitization, 1_000_000);
		assert_eq!(vault_revenue_2[7].securitization_activated, 1_000_000);

		assert_eq!(RevenuePerFrameByVault::<Test>::get(10).len(), 10);
	})
}

#[test]
fn it_accounts_for_pending_bitcoins() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);

		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms: VaultTerms {
					bitcoin_annual_percent_rate: FixedU128::from_float(0.0),
					bitcoin_base_fee: 0,
				},
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 100_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		assert_eq!(Balances::free_balance(1), 900_000);
		let _ = Vaults::reserve_securitization(
			1,
			&2,
			&securitization(100_000),
			standard_reservation_request(),
		)
		.expect("bonding failed");

		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_activated_securitization(), 0,);

		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_activated_securitization(), 0);

		Vaults::record_bitcoin_lock_funding(1, funding_update(&securitization(100_000), 0))
			.unwrap();
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_activated_securitization(), 0);
	});
}

#[test]
fn it_tracks_funded_and_ratio_adjusted_satoshis_via_provider_methods() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		set_argons(1, 100_010);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().ratio_adjusted_satoshis, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitized_satoshis, 0);

		assert_ok!(Vaults::reserve_securitization(
			1,
			&1,
			&securitization(1_000),
			standard_reservation_request(),
		));
		let (positions, _) = Vaults::get_top_vaults_by_securitization(1);
		assert_eq!(positions[0].activated_securitization, 0);
		assert_ok!(Vaults::record_bitcoin_lock_funding(
			1,
			funding_update(&securitization(1_000), 1_000),
		));
		assert_ok!(Vaults::reserve_securitization(
			1,
			&1,
			&securitization(500),
			standard_reservation_request(),
		));
		assert_ok!(Vaults::record_bitcoin_lock_funding(
			1,
			funding_update(&securitization(500), 500),
		));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().ratio_adjusted_satoshis, 1_500);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitized_satoshis, 1_500);
		let (positions, _) = Vaults::get_top_vaults_by_securitization(1);
		assert_eq!(positions[0].activated_securitization, 1_500);
		assert_eq!(positions[0].bitcoin_locked_satoshis, 1_500);

		assert_ok!(Vaults::release_bitcoin_lock_securitization(
			1,
			&securitization(600),
			600,
			&LockExtension::new(100),
			false,
		));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().ratio_adjusted_satoshis, 900);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitized_satoshis, 900);

		let mut config = default_vault();
		config.securitization_ratio = FixedU128::from_u32(2);
		set_argons(2, 100_010);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(2), config));

		let doubled_securitization = BitcoinSecuritization {
			securitization_ratio: FixedU128::from_u32(2),
			..securitization(1_000)
		};
		assert_ok!(Vaults::reserve_securitization(
			2,
			&2,
			&doubled_securitization,
			standard_reservation_request(),
		));
		assert_ok!(Vaults::record_bitcoin_lock_funding(
			2,
			funding_update(&doubled_securitization, 1_000),
		));
		let vault = VaultsById::<Test>::get(2).unwrap();
		assert_eq!(vault.securitized_satoshis, 1_000);
		assert_eq!(vault.ratio_adjusted_satoshis, 2_000);

		assert_ok!(Vaults::release_bitcoin_lock_securitization(
			2,
			&BitcoinSecuritization {
				securitization_ratio: FixedU128::from_u32(2),
				..securitization(600)
			},
			600,
			&LockExtension::new(100),
			false,
		));
		let vault = VaultsById::<Test>::get(2).unwrap();
		assert_eq!(vault.securitized_satoshis, 400);
		assert_eq!(vault.ratio_adjusted_satoshis, 800);
	});
}

#[test]
fn provider_resecuritizes_a_funded_lock_and_reuses_its_backing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 100_010);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));

		let current = securitization(1_000);
		let replacement = securitization(800);
		assert_ok!(
			Vaults::reserve_securitization(1, &1, &current, standard_reservation_request(),)
		);
		assert_ok!(Vaults::record_bitcoin_lock_funding(1, funding_update(&current, 1_000),));
		CurrentFrameId::set(2);

		let mut lock_extension = LockExtension::new(100);
		assert_eq!(
			<Vaults as BitcoinVaultProvider>::resecuritize(
				1,
				&1,
				BitcoinResecuritization {
					current: &current,
					replacement: &replacement,
					funded_satoshis: 1_000,
					remaining_term: FixedU128::one(),
					lock_extension: &mut lock_extension,
					is_flexible: false,
					fee_discount: 0,
					securitization_space_to_unreserve: 0,
				},
			)
			.unwrap(),
			(0, 0)
		);

		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization_locked, 800);
		assert_eq!(vault.securitization_pending_activation, 0);
		assert_eq!(vault.securitized_satoshis, 800);
		assert_eq!(vault.ratio_adjusted_satoshis, 800);
		assert_eq!(vault.get_relock_capacity(), 200);
		let revenue = RevenuePerFrameByVault::<Test>::get(1);
		let replacement_revenue = revenue.first().unwrap();
		assert_eq!(replacement_revenue.frame_id, 2);
		assert_eq!(replacement_revenue.bitcoin_locks_new_securitization, 0);
		assert_eq!(replacement_revenue.bitcoin_locks_released_securitization, 200);
		assert_eq!(replacement_revenue.bitcoin_locks_added_satoshis, 0);
		assert_eq!(replacement_revenue.bitcoin_locks_released_satoshis, 0);
	});
}

#[test]
fn resecuritization_preserves_a_whole_exit_without_earmarking_backing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 10_000);
		let mut config = default_vault();
		config.securitization = 1_000;
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));

		let current = securitization(1_000);
		assert_ok!(
			Vaults::reserve_securitization(1, &1, &current, standard_reservation_request(),)
		);
		assert_ok!(Vaults::record_bitcoin_lock_funding(1, funding_update(&current, 1_000)));
		CurrentFrameId::set(2);
		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 500, FixedU128::one()));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().exit_notice_amount(), 500);

		let replacement = securitization(500);
		let mut lock_extension = LockExtension::new(100);
		assert_ok!(<Vaults as BitcoinVaultProvider>::resecuritize(
			1,
			&1,
			BitcoinResecuritization {
				current: &current,
				replacement: &replacement,
				funded_satoshis: 1_000,
				remaining_term: FixedU128::one(),
				lock_extension: &mut lock_extension,
				is_flexible: false,
				fee_discount: 0,
				securitization_space_to_unreserve: 0,
			},
		));

		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization_locked, 500);
		assert_eq!(vault.exit_notice_amount(), 500);
		let exit_height = *vault
			.securitization_release_schedule
			.iter()
			.find(|(_, entry)| !entry.argon_withdrawals.is_zero())
			.unwrap()
			.0;
		assert!(VaultFundsReleasingByHeight::<Test>::get(exit_height).contains(&1));

		LastBitcoinHeightChange::set((exit_height, exit_height));
		Vaults::on_initialize(2);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 500);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 500);
	});
}

#[test]
fn provider_resecuritization_applies_coupon_terms() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 100);
		set_argons(2, 100);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig { securitization: 100, ..default_vault() }
		));
		VaultsById::<Test>::mutate(1, |vault| {
			vault.as_mut().expect("vault").reserved_securitization_space = 100;
		});

		let current = securitization(0);
		let replacement = securitization(40);
		let mut lock_extension = LockExtension::new(100);
		let (fee, fee_discount) = <Vaults as BitcoinVaultProvider>::resecuritize(
			1,
			&2,
			BitcoinResecuritization {
				current: &current,
				replacement: &replacement,
				funded_satoshis: 0,
				remaining_term: FixedU128::one(),
				lock_extension: &mut lock_extension,
				is_flexible: false,
				fee_discount: 20,
				securitization_space_to_unreserve: 150,
			},
		)
		.expect("resecuritization");

		assert_eq!((fee, fee_discount), (44, 20));
		assert_eq!(Balances::free_balance(2), 76);
		let vault = VaultsById::<Test>::get(1).expect("vault");
		assert_eq!(vault.reserved_securitization_space, 0);
		assert_eq!(vault.securitization_pending_activation, 40);
		let revenue = RevenuePerFrameByVault::<Test>::get(1);
		assert_eq!(revenue[0].bitcoin_lock_fee_revenue, fee);
		assert_eq!(revenue[0].bitcoin_lock_fee_coupon_value_used, fee_discount);
	});
}

#[test]
fn it_errors_when_releasing_more_funded_satoshis_than_the_vault_tracks() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		set_argons(1, 100_010);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().ratio_adjusted_satoshis, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitized_satoshis, 0);
		VaultsById::<Test>::mutate(1, |vault| {
			vault.as_mut().expect("vault").securitization_locked = 1;
		});

		assert_err!(
			Vaults::release_bitcoin_lock_securitization(
				1,
				&securitization(1),
				1,
				&LockExtension::new(100),
				false,
			),
			VaultError::InternalError
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().ratio_adjusted_satoshis, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitized_satoshis, 0);
	});
}

#[test]
fn it_can_burn_funds() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		let terms = default_terms(FixedU128::zero());
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 100_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		assert_eq!(Balances::free_balance(1), 900_000);

		set_argons(2, 2_000);
		let (fee, _) = Vaults::reserve_securitization(
			1,
			&2,
			&securitization(100_000),
			standard_reservation_request(),
		)
		.expect("bonding failed");

		assert_eq!(fee, 0);
		assert_eq!(Balances::free_balance(2), 2_000);
		assert_ok!(Vaults::record_bitcoin_lock_funding(
			1,
			funding_update(&securitization(100_000), 500),
		));
		CurrentFrameId::set(2);
		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 0, FixedU128::one()));
		assert_ok!(Vaults::burn(
			1,
			&securitization(100_000),
			500,
			100_000,
			&LockExtension::new(2440),
			false,
		));

		assert_eq!(Balances::free_balance(1), 900_000);
		assert_eq!(Balances::total_balance(&1), 900_000, "Burned from the vault owner");
		assert_eq!(Balances::free_balance(2), 2_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_locked, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().exit_notice_amount(), 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_target, 0);
		assert_eq!(crate::TotalVaultSecuritization::<Test>::get(), 0);
	});
}

#[test]
fn zero_burn_retires_funded_satoshis_and_schedules_funded_collateral() {
	new_test_ext().execute_with(|| {
		System::set_block_number(5);
		set_argons(1, 1_000_000);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms: default_terms(FixedU128::zero()),
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 100_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		set_argons(2, 2_000);
		assert_ok!(Vaults::reserve_securitization(
			1,
			&2,
			&securitization(100_000),
			standard_reservation_request(),
		));
		assert_ok!(Vaults::record_bitcoin_lock_funding(
			1,
			funding_update(&securitization(100_000), 500),
		));
		CurrentFrameId::set(2);

		assert_eq!(
			Vaults::burn(1, &securitization(100_000), 500, 0, &LockExtension::new(2440), false),
			Ok(0),
		);
		let vault = VaultsById::<Test>::get(1).expect("vault remains");
		assert_eq!(vault.securitization, 100_000);
		assert_eq!(vault.securitization_locked, 0);
		assert_eq!(vault.total_satoshis, 0);
		assert_eq!(vault.securitized_satoshis, 0);
		assert_eq!(
			vault.get_relock_capacity(),
			securitization(100_000).collateral_for_satoshis(500)
		);
		assert_eq!(Balances::total_balance(&1), 1_000_000);
		let revenue = RevenuePerFrameByVault::<Test>::get(1);
		let release_frame = revenue.iter().find(|entry| entry.frame_id == 2).unwrap();
		assert_eq!(release_frame.bitcoin_locks_released_securitization, 100_000);
		assert_eq!(release_frame.bitcoin_locks_released_satoshis, 500);
	});
}

fn insured_redemption_compensation_scenario(
	securitization_ratio: f32,
	redemption_amount: u128,
	expected_compensation: u128,
) {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let vault_operator = 1;
		let bitcoin_locker = 2;
		let securitization = 200_000;
		let securitization_coverage_microgons = 100_000;
		let securitization_ratio = FixedU128::from_float(securitization_ratio as f64);

		set_argons(bitcoin_locker, 0);
		set_argons(vault_operator, securitization);

		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(vault_operator),
			VaultConfig {
				terms: VaultTerms {
					bitcoin_annual_percent_rate: FixedU128::zero(),
					bitcoin_base_fee: 0,
				},
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization,
				securitization_ratio,
			}
		));

		assert_eq!(Balances::free_balance(vault_operator), 0);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::EnterVault.into(), &vault_operator),
			securitization
		);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.securitization_locked, 0);
		assert_eq!(vault.securitization, securitization);

		let lock_extensions = LockExtension::new(1440 * 365);
		let btc_value_in_microgons = securitization_coverage_microgons / 2;
		let bitcoin_securitization = BitcoinSecuritization {
			basis: BitcoinSecuritizationBasis {
				satoshis: 500,
				microgons_at_target_per_btc: btc_value_in_microgons
					.saturating_mul(SATOSHIS_PER_BITCOIN.into())
					.checked_div(&500)
					.unwrap(),
			},
			securitization_coverage_microgons,
			securitization_ratio,
		};

		Vaults::reserve_securitization(
			1,
			&bitcoin_locker,
			&bitcoin_securitization,
			standard_reservation_request(),
		)
		.expect("bonding failed");
		Vaults::record_bitcoin_lock_funding(1, funding_update(&bitcoin_securitization, 500))
			.expect("activation failed");

		let compensation = Vaults::compensate_lost_bitcoin(
			1,
			&bitcoin_locker,
			&bitcoin_securitization,
			500,
			redemption_amount,
			&lock_extensions,
			false,
		)
		.expect("compensation failed");
		assert_eq!(compensation.to_beneficiary, expected_compensation);
		assert_eq!(compensation.burned, 0);

		assert_eq!(
			Balances::total_balance(&vault_operator),
			securitization - expected_compensation,
			"vault operator total balance"
		);
		// should keep the rest on hold
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::EnterVault.into(), &vault_operator),
			securitization - expected_compensation,
			"vault operator balance on hold"
		);
		let vault = VaultsById::<Test>::get(1).unwrap();
		let remaining_lock_collateral = securitization_ratio
			.saturating_mul_int(securitization_coverage_microgons)
			.saturating_sub(expected_compensation);
		assert_eq!(vault.get_relock_capacity(), remaining_lock_collateral, "relock capacity");
		assert_eq!(vault.securitization_locked, 0, "argons locked");
		assert_eq!(vault.securitization, securitization - expected_compensation, "securitization");
		assert_eq!(
			Balances::free_balance(bitcoin_locker),
			expected_compensation,
			"locker free balance"
		);
	});
}

#[test]
fn it_records_use_of_fee_coupons() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		let mut terms = default_terms(FixedU128::from_float(0.01));
		terms.bitcoin_base_fee = 1000;
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms: terms.clone(),
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 500_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		assert_eq!(Balances::free_balance(1), 500_000);

		set_argons(2, 6_000);
		let fee_discount = 2_000;
		let (fee, discount_applied) = Vaults::reserve_securitization(
			1,
			&2,
			&securitization(500_000),
			ReserveSecuritizationRequest {
				lock_expiration: 100,
				fee_discount,
				securitization_space_to_unreserve: 0,
			},
		)
		.expect("bonding failed");

		let calculated_fee = terms.bitcoin_base_fee + (0.01f64 * 500_000f64) as u128;
		assert_eq!(fee, calculated_fee);
		assert_eq!(discount_applied, fee_discount);
		assert_eq!(Balances::free_balance(2), 2_000, "fee discount applied");
		let revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(revenue.len(), 1);
		assert_eq!(revenue[0].bitcoin_lock_fee_revenue, fee, "revenue recorded correctly");
		assert_eq!(
			revenue[0].bitcoin_lock_fee_coupon_value_used, fee_discount,
			"records fee coupons correctly"
		);
	});
}

#[test]
fn full_insurance_is_paid_with_a_1x_vault_ratio() {
	insured_redemption_compensation_scenario(1.0, 150_000, 100_000);
}

#[test]
fn full_insurance_is_paid_with_a_2x_vault_ratio() {
	insured_redemption_compensation_scenario(2.0, 150_000, 100_000);
}

#[test]
fn redemption_below_insurance_limits_compensation() {
	insured_redemption_compensation_scenario(1.0, 60_000, 60_000);
}

#[test]
fn compensation_pays_available_collateral_when_insurance_exceeds_vault_funds() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 50_000);
		set_argons(2, 0);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms: default_terms(FixedU128::zero()),
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 50_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		// Model a Lock whose insured amount exceeds its allocated collateral.
		let mut insured_lock = securitization(100_000);
		insured_lock.securitization_ratio = FixedU128::from_rational(1, 2);
		assert_ok!(Vaults::reserve_securitization(
			1,
			&2,
			&insured_lock,
			standard_reservation_request(),
		));
		assert_ok!(Vaults::record_bitcoin_lock_funding(1, funding_update(&insured_lock, 100_000),));

		let compensation = Vaults::compensate_lost_bitcoin(
			1,
			&2,
			&insured_lock,
			100_000,
			100_000,
			&LockExtension::new(365),
			false,
		)
		.expect("available collateral should still be paid");
		assert_eq!(compensation.to_beneficiary, 50_000);
		assert_eq!(Balances::free_balance(2), 50_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 0);
		System::assert_last_event(
			Event::LostBitcoinCompensated {
				vault_id: 1,
				beneficiary: 2,
				to_beneficiary: 50_000,
				shortfall: 50_000,
				burned: 0,
			}
			.into(),
		);
	});
}

#[test]
fn it_should_allow_vaults_to_rotate_xpubs() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		let terms = default_terms(TEN_PCT);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 100_000,
				securitization_ratio: FixedU128::one(),
			}
		));

		let first_keyset = VaultXPubById::<Test>::get(1).unwrap();
		assert_eq!(first_keyset.1, 0);

		let mut seed = [0u8; 32];
		OsRng.fill_bytes(&mut seed);
		let network = GetBitcoinNetwork::get();
		let owner_xpriv = Xpriv::new_master(network, &seed).unwrap();
		let owner_pubkey = Xpub::from_priv(&Secp256k1::new(), &owner_xpriv);
		let owner_pubkey: CompressedBitcoinPubkey = owner_pubkey.public_key.serialize().into();

		let key1 = Vaults::create_utxo_script_pubkey(1, owner_pubkey, 100, 120, 80);
		assert!(key1.is_ok());
		let key1 = key1.unwrap();

		let key2 = Vaults::create_utxo_script_pubkey(1, owner_pubkey, 100, 120, 80);
		assert!(key2.is_ok());
		let key2 = key2.unwrap();
		assert_ne!(key1.0.public_key, key2.0.public_key);
		assert_eq!(key1.0.child_number, 1);
		assert_eq!(key2.0.child_number, 3);

		let new_xpub = keys();
		assert_ok!(Vaults::replace_bitcoin_xpub(RuntimeOrigin::signed(1), 1, new_xpub));
		let new_keyset = VaultXPubById::<Test>::get(1).unwrap();
		assert_eq!(new_keyset.1, 0);
	});
}

#[test]
fn it_can_schedule_term_changes() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let mut terms = default_terms(TEN_PCT);
		let config = VaultConfig {
			terms: terms.clone(),
			delegate_account_id: None,
			bitcoin_xpubkey: keys(),
			securitization: 100_000,
			securitization_ratio: FixedU128::one(),
		};
		set_argons(1, 1_000_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));

		System::set_block_number(10);
		Vaults::on_finalize(10);
		IsSlotBiddingStarted::set(true);

		terms.bitcoin_base_fee = 1000;
		assert_ok!(Vaults::modify_terms(RuntimeOrigin::signed(1), 1, terms.clone()));
		assert_ne!(VaultsById::<Test>::get(1).unwrap().terms.bitcoin_base_fee, 1000);
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().pending_terms.unwrap().1.bitcoin_base_fee,
			1000
		);
		System::assert_last_event(
			Event::VaultTermsChangeScheduled { vault_id: 1, change_tick: 100 }.into(),
		);
		assert_eq!(PendingTermsModificationsByTick::<Test>::get(100).first().unwrap().clone(), 1);

		// should not be able to schedule another change
		assert_err!(
			Vaults::modify_terms(RuntimeOrigin::signed(1), 1, terms.clone()),
			Error::<Test>::TermsChangeAlreadyScheduled
		);

		CurrentTick::set(100);
		Vaults::on_finalize(100);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().terms.bitcoin_base_fee, 1000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().pending_terms, None);
		assert_eq!(PendingTermsModificationsByTick::<Test>::get(100).first(), None);
	});
}

#[test]
fn it_can_schedule_terms_changes_before_bidding_starts() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let mut terms = default_terms(TEN_PCT);
		let config = VaultConfig {
			terms: terms.clone(),
			delegate_account_id: None,
			bitcoin_xpubkey: keys(),
			securitization: 100_000,
			securitization_ratio: FixedU128::one(),
		};
		set_argons(1, 1_000_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));

		System::set_block_number(10);
		Vaults::on_finalize(10);
		IsSlotBiddingStarted::set(false);

		terms.bitcoin_base_fee = 1000;
		System::initialize(&11, &System::parent_hash(), &Default::default());
		Vaults::on_initialize(11);
		assert_ok!(Vaults::modify_terms(RuntimeOrigin::signed(1), 1, terms.clone()));
		Vaults::on_finalize(11);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().terms.bitcoin_base_fee, 1000);
	});
}

#[test]
fn it_can_send_minimum_balance_transfers() {
	new_test_ext().execute_with(|| {
		set_argons(1, 1060);
		assert_ok!(Balances::transfer(&1, &2, 1000, Preservation::Preserve));
		assert_ok!(Balances::transfer(&1, &2, 50, Preservation::Preserve));
		assert_eq!(Balances::free_balance(1), 10);
		assert_ok!(Balances::transfer(&1, &2, 4, Preservation::Expendable));
		// dusted! will remove anything below ED
		assert_eq!(Balances::free_balance(1), 0);
	})
}

#[test]
fn it_can_cleanup_at_bitcoin_heights() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 200_000_000_000);
		set_argons(2, 50_000_000);

		let terms = default_terms(FixedU128::from_float(0.01));
		let config = VaultConfig {
			terms: terms.clone(),
			delegate_account_id: None,
			bitcoin_xpubkey: keys(),
			securitization: 1_000_000_000,
			securitization_ratio: FixedU128::one(),
		};
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));

		let amount = 1_000_000;

		CurrentTick::set(1);
		assert_ok!(Vaults::reserve_securitization(
			1,
			&2,
			&securitization(amount),
			standard_reservation_request(),
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().available_securitization_space(true),
			1_000_000_000 - 1_000_000
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_locked, 1_000_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_relock_capacity(), 0);
		assert_ok!(Vaults::record_bitcoin_lock_funding(
			1,
			funding_update(&securitization(amount), 500),
		));
		CurrentFrameId::set(2);

		assert_ok!(Vaults::release_bitcoin_lock_securitization(
			1,
			&securitization(amount),
			500,
			&LockExtension::new(365),
			false,
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().available_securitization_space(true),
			1_000_000_000
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_locked, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_relock_capacity(), 500);
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_release_schedule[&432]
				.relockable_commitments,
			500
		);
		assert_eq!(VaultFundsReleasingByHeight::<Test>::get(432).len(), 1);
		assert_eq!(VaultFundsReleasingByHeight::<Test>::get(432).first().unwrap(), &1);
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 2);
		assert_eq!(vault_revenue[0].frame_id, 2);
		assert_eq!(vault_revenue[0].bitcoin_locks_new_securitization, 0);
		assert_eq!(vault_revenue[0].bitcoin_locks_released_securitization, 1_000_000);
		assert_eq!(vault_revenue[0].bitcoin_locks_released_satoshis, 500);
		assert_eq!(vault_revenue[0].bitcoin_locks_created, 0);
		assert_eq!(vault_revenue[0].uncollected_revenue, 0);

		// expire it
		System::set_block_number(10);
		LastBitcoinHeightChange::set((431, 432));
		Vaults::on_initialize(10);

		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_locked, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_relock_capacity(), 0);
		assert_eq!(VaultFundsReleasingByHeight::<Test>::get(432).len(), 0);
	});
}

#[test]
fn it_can_reuse_locked_argons() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 200_000_000_000);
		set_argons(2, 50_000_000);

		let terms = default_terms(FixedU128::from_float(0.01));
		let config = VaultConfig {
			terms: terms.clone(),
			delegate_account_id: None,
			bitcoin_xpubkey: keys(),
			securitization: 10_000_000,
			securitization_ratio: FixedU128::one(),
		};
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));

		let amount = 1_000_000;

		CurrentTick::set(1);
		assert_ok!(Vaults::reserve_securitization(
			1,
			&2,
			&securitization(amount),
			standard_reservation_request(),
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().available_securitization_space(true),
			10_000_000 - amount
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_locked, amount);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_relock_capacity(), 0);
		assert_ok!(Vaults::record_bitcoin_lock_funding(
			1,
			funding_update(&securitization(amount), 500),
		));

		assert_ok!(Vaults::release_bitcoin_lock_securitization(
			1,
			&securitization(amount),
			500,
			&LockExtension::new(365),
			false,
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().available_securitization_space(true),
			10_000_000
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_locked, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_relock_capacity(), 500);
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_release_schedule[&432]
				.relockable_commitments,
			500
		);

		set_argons(3, 3_000_000);
		assert_ok!(Vaults::reserve_securitization(
			1,
			&3,
			&securitization(2_500_000),
			standard_reservation_request(),
		));
		assert_ok!(Vaults::record_bitcoin_lock_funding(
			1,
			funding_update(&securitization(2_500_000), 2_500),
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().available_securitization_space(true),
			10_000_000 - 2_500_000
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_locked, 2_500_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_relock_capacity(), 0);

		assert_ok!(Vaults::release_bitcoin_lock_securitization(
			1,
			&securitization(2_500_000),
			2500,
			&LockExtension::new(365),
			false,
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().available_securitization_space(true),
			10_000_000
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_locked, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_relock_capacity(), 2_500);
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_release_schedule[&432]
				.relockable_commitments,
			2_500
		);
	});
}

#[test]
fn vaults_can_collect_revenue() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		CurrentFrameId::set(1);

		set_argons(1, 1_000_000);
		let mut terms = default_terms(FixedU128::from_float(0.01));
		terms.bitcoin_base_fee = 1000;
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 500_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		assert_eq!(Balances::free_balance(1), 500_000);

		CurrentFrameId::set(2);
		Vaults::on_frame_start(2);
		RevenuePerFrameByVault::<Test>::get(1);
		assert_eq!(RevenuePerFrameByVault::<Test>::get(1).len(), 0);
		assert_eq!(RevenuePerFrameByVaultCount::<Test>::get(), 0);

		set_argons(2, 6_000);
		let (fee, _) = Vaults::reserve_securitization(
			1,
			&2,
			&securitization(500_000),
			standard_reservation_request(),
		)
		.expect("bonding failed");

		let current_frame_id = CurrentFrameId::get();
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 1);
		assert_eq!(vault_revenue[0].frame_id, current_frame_id);
		assert_eq!(vault_revenue[0].bitcoin_lock_fee_revenue, fee);
		assert_eq!(vault_revenue[0].bitcoin_locks_new_securitization, 500_000);
		assert_eq!(vault_revenue[0].bitcoin_locks_added_satoshis, 0);
		assert_eq!(vault_revenue[0].bitcoin_locks_created, 1);
		assert_eq!(vault_revenue[0].uncollected_revenue, fee);
		assert_eq!(vault_revenue[0].securitization, 500_000);
		assert_eq!(vault_revenue[0].securitization_activated, 0); // funds are still pending
		assert_eq!(Balances::balance_on_hold(&HoldReason::PendingCollect.into(), &1), fee);

		// set up a mining bid pallet account. 10k is "already distributed"
		let vault_lp_earnings = 40_000;
		set_argons(100, vault_lp_earnings);
		assert_ok!(Vaults::record_vault_frame_earnings(
			&100,
			VaultTreasuryFrameEarnings {
				vault_id: 1,
				vault_operator_account_id: 1,
				frame_id: current_frame_id,
				earnings: 50_000,
				capital_contributed: 100_000,
				earnings_for_vault: vault_lp_earnings,
				capital_contributed_by_vault: 10_000,
			},
		));
		assert_eq!(Balances::free_balance(100), 0);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PendingCollect.into(), &1),
			fee + vault_lp_earnings
		);
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 1);
		assert_eq!(vault_revenue[0].frame_id, current_frame_id);
		assert_eq!(vault_revenue[0].treasury_vault_earnings, vault_lp_earnings);
		assert_eq!(vault_revenue[0].treasury_total_earnings, 50_000);
		assert_eq!(vault_revenue[0].treasury_vault_capital, 10_000);
		assert_eq!(vault_revenue[0].treasury_external_capital, 90_000);
		assert_eq!(vault_revenue[0].uncollected_revenue, fee + vault_lp_earnings);

		assert!(LastCollectFrameByVaultId::<Test>::get(1).is_none());
		assert_err!(Vaults::collect(RuntimeOrigin::signed(2), 1), Error::<Test>::NoPermissions);
		// test that you can only collect if you have no pending cosigns
		PendingCosignByVaultId::<Test>::mutate(1, |a| a.try_insert(1).unwrap());
		assert_err!(
			Vaults::collect(RuntimeOrigin::signed(1), 1),
			Error::<Test>::PendingCosignsBeforeCollect
		);
		PendingCosignByVaultId::<Test>::mutate(1, |a| a.remove(&1));
		OrphanedUtxoAccountsByVaultId::<Test>::insert(1, 1, 1);
		assert_err!(
			Vaults::collect(RuntimeOrigin::signed(1), 1),
			Error::<Test>::PendingOrphanedUtxoCosignsBeforeCollect
		);
		OrphanedUtxoAccountsByVaultId::<Test>::remove(1, 1);
		OverdueCollectBlockers::mutate(|entries| {
			entries.insert(1);
		});
		assert_err!(
			Vaults::collect(RuntimeOrigin::signed(1), 1),
			Error::<Test>::OverdueCollectBlockersBeforeCollect
		);
		OverdueCollectBlockers::mutate(|entries| {
			entries.remove(&1);
		});

		let providers_before_collect = System::providers(&1);
		assert_ok!(Vaults::collect(RuntimeOrigin::signed(1), 1));
		assert_eq!(System::providers(&1), providers_before_collect);
		assert_eq!(Balances::free_balance(1), 1_000_000 - 500_000 + fee + vault_lp_earnings);
		assert_eq!(Balances::free_balance(100), 0);
		assert_eq!(LastCollectFrameByVaultId::<Test>::get(1), Some(CurrentFrameId::get()));

		VaultsById::<Test>::mutate(1, |v| {
			if let Some(v) = v {
				v.securitization_pending_activation = 0;
			}
		});
		CurrentFrameId::set(3);
		Vaults::on_frame_start(3);
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 1);
		assert_eq!(vault_revenue[0].frame_id, 2);
		assert_eq!(vault_revenue[0].uncollected_revenue, 0);
		assert_eq!(vault_revenue[0].securitization_activated, 500_000);

		// make sure order is correct if we get another entry

		let vault_lp_earnings = 20_000;
		set_argons(100, vault_lp_earnings);
		assert_eq!(CurrentFrameId::get(), 3);
		assert_ok!(Vaults::record_vault_frame_earnings(
			&100,
			VaultTreasuryFrameEarnings {
				vault_id: 1,
				vault_operator_account_id: 1,
				frame_id: CurrentFrameId::get(),
				earnings: 50_000,
				capital_contributed: 100_000,
				earnings_for_vault: vault_lp_earnings,
				capital_contributed_by_vault: 10_000,
			},
		));
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 2);
		assert_eq!(vault_revenue[0].frame_id, 3);
		assert_eq!(vault_revenue[0].uncollected_revenue, vault_lp_earnings);
		assert_eq!(vault_revenue[1].frame_id, 2);
		assert_eq!(vault_revenue[1].uncollected_revenue, 0);
	});
}

#[test]
fn it_burns_uncollected_revenue() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		let terms = default_terms(FixedU128::from_float(0.01));
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 500_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		let bid_pool_account = 100;
		set_argons(bid_pool_account, 1_100_000);
		for i in 1..=10 {
			// Go past genesis block so events get deposited
			System::set_block_number(5 + i);
			CurrentFrameId::set(i);
			// Go past the frame start so revenue gets recorded
			Vaults::on_frame_start(i);

			assert_ok!(Vaults::record_vault_frame_earnings(
				&bid_pool_account,
				VaultTreasuryFrameEarnings {
					vault_id: 1,
					vault_operator_account_id: 1,
					frame_id: i,
					earnings: 100_000,
					capital_contributed: 100_000,
					earnings_for_vault: 100_000,
					capital_contributed_by_vault: 100_000,
				},
			));
		}
		let pending_revenue = RevenuePerFrameByVault::<Test>::get(1);
		assert_eq!(pending_revenue.len(), 10);
		assert_eq!(RevenuePerFrameByVaultCount::<Test>::get(), 1);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PendingCollect.into(), &1), 1_000_000);
		assert_eq!(pending_revenue[0].uncollected_revenue, 100_000);
		assert_eq!(pending_revenue[0].frame_id, 10);
		assert_eq!(pending_revenue[9].frame_id, 1);
		System::reset_events();
		CurrentFrameId::set(11);
		Vaults::on_frame_start(11);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PendingCollect.into(), &1),
			900_000,
			"should burn 100k"
		);
		System::assert_has_event(
			Event::<Test>::VaultRevenueUncollected { vault_id: 1, amount: 100_000, frame_id: 1 }
				.into(),
		);
		let pending_revenue = RevenuePerFrameByVault::<Test>::get(1);
		assert_eq!(pending_revenue.len(), 9);
		assert_eq!(RevenuePerFrameByVaultCount::<Test>::get(), 1);
		assert!(!pending_revenue.iter().any(|a| a.frame_id == 1));
	})
}

#[test]
fn it_tracks_pending_cosign_utxos_for_vaults() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		assert_ok!(Vaults::update_pending_cosign_list(1, 1, false));
		assert_ok!(Vaults::update_pending_cosign_list(1, 2, false));
		assert_ok!(Vaults::update_pending_cosign_list(2, 3, false));
		assert_ok!(Vaults::update_pending_cosign_list(2, 4, false));
		assert_eq!(
			PendingCosignByVaultId::<Test>::get(1).into_iter().collect::<Vec<_>>(),
			vec![1, 2]
		);
		assert_eq!(
			PendingCosignByVaultId::<Test>::get(2).into_iter().collect::<Vec<_>>(),
			vec![3, 4]
		);

		assert_ok!(Vaults::update_pending_cosign_list(1, 1, true));
		assert_eq!(PendingCosignByVaultId::<Test>::get(1).into_iter().collect::<Vec<_>>(), vec![2]);

		assert_ok!(Vaults::update_pending_cosign_list(2, 4, true));
		assert_eq!(PendingCosignByVaultId::<Test>::get(2).into_iter().collect::<Vec<_>>(), vec![3]);
	});
}
