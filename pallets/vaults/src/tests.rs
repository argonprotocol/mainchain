use crate::{
	mock::{Vaults, *},
	pallet::{
		ArgonotCommitmentByVaultId, BitcoinLockUpdate, NextVaultId,
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
use pallet_prelude::{
	argon_primitives::{
		vault::{LockExtension, TreasuryVaultProvider, VaultTreasuryFrameEarnings},
		OnNewSlot,
	},
	*,
};

const TEN_PCT: FixedU128 = FixedU128::from_rational(110, 100);

fn keys() -> OpaqueBitcoinXpub {
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
	VaultTerms {
		bitcoin_annual_percent_rate: pct,
		bitcoin_base_fee: 0,
		treasury_profit_sharing: Permill::zero(),
	}
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
		securitized_satoshis: funded_satoshis.min(securitization.basis.satoshis),
		collateral_required: securitization.collateral_for_satoshis(funded_satoshis),
		securitization_ratio: securitization.securitization_ratio,
		is_flexible: false,
	}
}

fn standard_reservation_request() -> ReserveSecuritizationRequest<Balance> {
	ReserveSecuritizationRequest { fee_discount: 0, securitization_space_to_unreserve: 0 }
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
fn it_can_set_committed_argonots_for_a_vault() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 120_000);

		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		assert_ok!(Vaults::set_committed_argonots(RuntimeOrigin::signed(1), 10_000));

		System::assert_last_event(
			Event::CommittedArgonotsSet { vault_id: 1, operator_account_id: 1, amount: 10_000 }
				.into(),
		);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 10_000,);
		assert_eq!(<Vaults as BitcoinVaultProvider>::get_committed_argonots(&1), Some(10_000));
		let commitment =
			ArgonotCommitmentByVaultId::<Test>::get(1).expect("commitment should exist");
		assert_eq!(commitment.committed_micronots, 10_000);
		assert_eq!(commitment.encumbered_micronots, 0);

		assert_ok!(Vaults::set_committed_argonots(RuntimeOrigin::signed(1), 4_000));
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 4_000);
		assert_eq!(<Vaults as BitcoinVaultProvider>::get_committed_argonots(&1), Some(4_000));
		let commitment =
			ArgonotCommitmentByVaultId::<Test>::get(1).expect("commitment should exist");
		assert_eq!(commitment.committed_micronots, 4_000);
		assert_eq!(commitment.encumbered_micronots, 0);
	});
}

#[test]
fn committed_argonots_cannot_be_reduced_below_encumbered_backing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 120_000);

		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		assert_ok!(Vaults::set_committed_argonots(RuntimeOrigin::signed(1), 10_000));
		assert_ok!(<Vaults as BitcoinVaultProvider>::encumber_argonots(&1, 6_000));

		assert_noop!(
			Vaults::set_committed_argonots(RuntimeOrigin::signed(1), 5_999),
			Error::<Test>::CommittedArgonotsBelowEncumberedBacking
		);
		assert_eq!(
			<Vaults as BitcoinVaultProvider>::release_encumbered_argonots(&1, 6_001),
			Err(VaultError::CommittedArgonotsBelowEncumberedBacking)
		);
		assert_ok!(Vaults::set_committed_argonots(RuntimeOrigin::signed(1), 6_000));
		let commitment =
			ArgonotCommitmentByVaultId::<Test>::get(1).expect("commitment should exist");
		assert_eq!(commitment.committed_micronots, 6_000);
		assert_eq!(commitment.encumbered_micronots, 6_000);
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
				.securitization_release_schedule
				.try_insert(100, 7_500)
				.expect("release schedule stays within test bounds");
		});

		assert_eq!(
			<Vaults as BitcoinVaultProvider>::get_committed_securitization(&1, 10),
			Some(37_500)
		);
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
		LastBitcoinHeightChange::set((10, 11));

		VaultsById::<Test>::mutate(1, |vault| {
			let vault = vault.as_mut().expect("vault should exist");
			vault.securitization_locked = 40_000;
			vault.securitization_pending_activation = 10_000;
			vault
				.securitization_release_schedule
				.try_insert(18, 4_000)
				.expect("release schedule stays within test bounds");
			vault
				.securitization_release_schedule
				.try_insert(30, 7_500)
				.expect("release schedule stays within test bounds");
		});

		assert_eq!(
			<Vaults as BitcoinVaultProvider>::get_committed_securitization(&1, 7),
			Some(37_500)
		);
	});
}

#[test]
fn committed_securitization_uses_the_operational_minimum_floor() {
	new_test_ext().execute_with(|| {
		set_argons(1, 120_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		CurrentTick::set(5);
		NextSlot::set(10);

		VaultsById::<Test>::mutate(1, |vault| {
			let vault = vault.as_mut().expect("vault should exist");
			vault.securitization_locked = 0;
			vault.securitization_pending_activation = 0;
			vault.operational_minimum_release_tick = Some(40);
		});

		assert_eq!(
			<Vaults as BitcoinVaultProvider>::get_committed_securitization(&1, 3),
			Some(OperationalMinimumVaultSecuritization::get())
		);
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
			vault.securitized_satoshis = 20;
			vault.ratio_adjusted_satoshis = 20;
			vault.securitization_release_schedule.try_insert(288, 20).unwrap();
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
		assert!(vault.securitization_release_schedule.is_empty());
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
fn it_locks_operational_minimum_after_becoming_operational() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let mut config = default_vault();
		config.securitization = 2_500;
		set_argons(1, 10_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));

		<Vaults as BitcoinVaultProvider>::account_became_operational(&1);

		let unlock_tick = 1 + OperationalMinimumVaultLockTicks::get();
		let vault = VaultsById::<Test>::get(1).expect("vault should exist");
		assert_eq!(vault.operational_minimum_release_tick, Some(unlock_tick));
		assert!(crate::pallet::VaultsReleasingOperationalMinimumByTick::<Test>::get(unlock_tick)
			.contains(&1));
	});
}

#[test]
fn it_skips_operational_minimum_for_aged_vaults() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let mut config = default_vault();
		config.securitization = 2_500;
		set_argons(1, 10_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		CurrentTick::set(1 + OperationalMinimumVaultLockTicks::get());

		<Vaults as BitcoinVaultProvider>::account_became_operational(&1);

		let vault = VaultsById::<Test>::get(1).expect("vault should exist");
		assert_eq!(vault.operational_minimum_release_tick, None);
		assert!(crate::pallet::VaultsReleasingOperationalMinimumByTick::<Test>::iter()
			.next()
			.is_none());
	});
}

#[test]
fn it_clamps_modify_funding_to_operational_minimum() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let mut config = default_vault();
		config.securitization = 2_500;
		set_argons(1, 10_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		<Vaults as BitcoinVaultProvider>::account_became_operational(&1);

		assert_ok!(Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 1_000, FixedU128::one(),));

		let vault = VaultsById::<Test>::get(1).expect("vault should exist");
		assert_eq!(vault.securitization_target, 1_000);
		assert_eq!(vault.securitization, OperationalMinimumVaultSecuritization::get());
	});
}

#[test]
fn it_keeps_then_releases_operational_minimum_for_closed_vaults() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let mut config = default_vault();
		config.securitization = 2_500;
		set_argons(1, 10_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		<Vaults as BitcoinVaultProvider>::account_became_operational(&1);
		assert_ok!(Vaults::close(RuntimeOrigin::signed(1), 1));

		let vault = VaultsById::<Test>::get(1).expect("vault should exist");
		assert!(vault.is_closed);
		assert_eq!(vault.securitization_target, 0);
		assert_eq!(vault.securitization, OperationalMinimumVaultSecuritization::get());

		let unlock_tick = 1 + OperationalMinimumVaultLockTicks::get();
		PreviousTick::set(unlock_tick.saturating_sub(1));
		CurrentTick::set(unlock_tick);

		let _ = Vaults::on_initialize(1);

		let vault = VaultsById::<Test>::get(1).expect("vault should exist");
		assert_eq!(vault.operational_minimum_release_tick, None);
		assert!(crate::pallet::VaultsReleasingOperationalMinimumByTick::<Test>::get(unlock_tick)
			.is_empty());

		let vault = VaultsById::<Test>::get(1).expect("vault should exist");
		assert!(vault.is_closed);
		assert_eq!(vault.securitization_target, 0);
		assert_eq!(vault.securitization, 0);
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
				let _ = vault.securitization_release_schedule.try_insert(100, 498);
			}
		});
		VaultFundsReleasingByHeight::<Test>::mutate(100, |a| {
			let _ = a.try_insert(1);
		});
		LastBitcoinHeightChange::set((100, 100));
		Vaults::on_initialize(2);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 997);
		assert_eq!(Balances::reserved_balance(1), 997, "should shrink the securitization now");
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
		assert_ok!(Vaults::release_unactivated_securitization(1, amount));
		// should release the 1000 from the bitcoin lock and the 2000 in securitization
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
		assert_ok!(Vaults::release_unactivated_securitization(1, 500_000));
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
						satoshis_locked: 1000,
						satoshis_released: 0,
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
					treasury_profit_sharing: Permill::zero(),
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
		assert_ok!(Vaults::burn(
			1,
			&securitization(100_000),
			500,
			100_000,
			&LockExtension::new(2440),
			false,
		));

		assert_eq!(Balances::free_balance(1), 900_000);
		assert_eq!(Balances::total_balance(&1), 999_500, "Burned from the vault owner");
		assert_eq!(Balances::free_balance(2), 2_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_locked, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 99_500);
	});
}

struct VaultScenario {
	pub securitization: Balance,
	pub securitization_ratio: f32,
	pub securitization_coverage_microgons: Balance,
	pub release_price: Balance,
	pub user_should_get: Balance,
	pub vault_should_lose: Balance,
	pub vault_should_have_hold: Balance,
}

fn vault_equilibrium_scenario(scenario: VaultScenario) {
	let VaultScenario {
		release_price,
		securitization_coverage_microgons,
		securitization,
		securitization_ratio,
		user_should_get,
		vault_should_lose,
		vault_should_have_hold,
	} = scenario;
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let vault_operator = 1;
		let bitcoin_locker = 2;
		let securitization_ratio = FixedU128::from_float(securitization_ratio as f64);

		set_argons(bitcoin_locker, 0);
		set_argons(vault_operator, securitization);

		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(vault_operator),
			VaultConfig {
				terms: VaultTerms {
					bitcoin_annual_percent_rate: FixedU128::zero(),
					bitcoin_base_fee: 0,
					treasury_profit_sharing: Permill::zero(),
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
					.checked_div(500)
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
			release_price,
			&lock_extensions,
			false,
		)
		.expect("compensation failed");
		assert_eq!(compensation.to_beneficiary, user_should_get);
		assert_eq!(compensation.burned, vault_should_lose.saturating_sub(user_should_get));

		assert_eq!(
			Balances::total_balance(&vault_operator),
			securitization - vault_should_lose,
			"vault operator total balance"
		);
		// should keep the rest on hold
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::EnterVault.into(), &vault_operator),
			securitization - vault_should_lose,
			"vault operator balance on hold"
		);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.get_relock_capacity(), vault_should_have_hold, "relock capacity");
		assert_eq!(vault.securitization_locked, 0, "argons locked");
		assert_eq!(vault.securitization, securitization - vault_should_lose, "securitization");
		assert_eq!(Balances::free_balance(bitcoin_locker), user_should_get, "locker free balance");
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
			ReserveSecuritizationRequest { fee_discount, securitization_space_to_unreserve: 0 },
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
fn it_compensates_1x_securitization_when_over_pegged_price() {
	vault_equilibrium_scenario(VaultScenario {
		securitization: 200_000,
		securitization_ratio: 1.0,
		securitization_coverage_microgons: 100_000,
		release_price: 200_000,
		user_should_get: 0,
		vault_should_lose: 100_000,
		vault_should_have_hold: 0,
	});
}
#[test]
fn it_compensates_1x_securitization_when_under_pegged_price() {
	vault_equilibrium_scenario(VaultScenario {
		securitization: 200_000,
		securitization_ratio: 1.0,
		securitization_coverage_microgons: 100_000,
		release_price: 50_000,
		user_should_get: 0,
		vault_should_lose: 50_000,
		vault_should_have_hold: 50_000,
	});
}
#[test]
fn it_compensates_2x_securitization_when_over_pegged_price() {
	vault_equilibrium_scenario(VaultScenario {
		securitization: 200_000,
		securitization_ratio: 2.0,
		securitization_coverage_microgons: 100_000,
		release_price: 200_000,
		user_should_get: 100_000,
		vault_should_lose: 200_000,
		vault_should_have_hold: 0,
	});
}
#[test]
fn it_compensates_2x_securitization_when_under_securitized_amount() {
	vault_equilibrium_scenario(VaultScenario {
		securitization: 200_000,
		securitization_ratio: 2.0,
		securitization_coverage_microgons: 100_000,
		release_price: 250_000,
		user_should_get: 100_000,
		vault_should_lose: 200_000,
		vault_should_have_hold: 0,
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
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule.len(), 0);
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
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule.len(), 1);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule[&432], 500);
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
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule.len(), 0);
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
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule.len(), 0);
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
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule.len(), 1);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule[&432], 500);

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
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule.len(), 0);

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
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization_release_schedule.len(), 1);
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_release_schedule[&432],
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
		Vaults::record_vault_frame_earnings(
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
		);
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

		assert_ok!(Vaults::collect(RuntimeOrigin::signed(1), 1));
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
		Vaults::record_vault_frame_earnings(
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
		);
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

			Vaults::record_vault_frame_earnings(
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
			);
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
