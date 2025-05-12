use crate::{
	mock::{Vaults, *},
	pallet::{
		BitcoinLockCompletions, NextVaultId, ObligationsById, PendingTermsModificationsByTick,
		PerFrameFeeRevenueByVault, VaultXPubById, VaultsById,
	},
	Error, Event, HoldReason, VaultConfig,
};
use argon_primitives::{
	bitcoin::{CompressedBitcoinPubkey, OpaqueBitcoinXpub},
	vault::{
		BitcoinObligationProvider, FundType, Obligation, ObligationError, ObligationExpiration,
		VaultTerms,
	},
};
use bitcoin::{
	bip32::{ChildNumber, Xpriv, Xpub},
	key::Secp256k1,
};
use k256::elliptic_curve::rand_core::{OsRng, RngCore};
use pallet_prelude::*;

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
		liquidity_pool_profit_sharing: Permill::zero(),
	}
}

fn default_vault() -> VaultConfig<Balance> {
	VaultConfig {
		terms: default_terms(TEN_PCT),
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
		assert_eq!(vault.free_balance(), TEN_PCT.reciprocal().unwrap().saturating_mul_int(50_000));
	});
}

#[test]
fn it_delays_vault_activation_after_bidding() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		assert_noop!(
			Vaults::create(RuntimeOrigin::signed(1), default_vault()),
			Error::<Test>::InsufficientFunds
		);

		set_argons(1, 100_010);
		IsSlotBiddingStarted::set(true);

		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		System::assert_last_event(
			Event::VaultCreated {
				vault_id: 1,
				opened_tick: 100,
				operator_account_id: 1,
				securitization: 50_000,
				securitization_ratio: FixedU128::one(),
			}
			.into(),
		);

		assert_err!(
			Vaults::create_obligation(1, &2, 50_000, 500, 100,),
			ObligationError::VaultNotYetActive
		);
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
		assert_eq!(VaultsById::<Test>::get(1).unwrap().free_balance(), 500);
		System::assert_last_event(
			Event::VaultModified {
				vault_id: 1,
				securitization: 1000,
				securitization_ratio: FixedU128::from_float(2.0),
			}
			.into(),
		);
		assert_eq!(Balances::reserved_balance(1), 1000);

		assert_err!(
			Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 2000, FixedU128::from_float(1.9)),
			Error::<Test>::InvalidSecuritization
		);
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
	});
}

#[test]
fn it_can_correctly_calculate_activated_securitization() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		set_argons(1, 20_000);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms: default_terms(TEN_PCT),
				bitcoin_xpubkey: keys(),
				securitization: 1000,
				securitization_ratio: FixedU128::from_float(1.0),
			}
		));

		let mut vault = VaultsById::<Test>::get(1).unwrap();

		assert_eq!(vault.get_activated_securitization(), 0);
		vault.bitcoin_locked = 500;
		assert_eq!(vault.get_activated_securitization(), 500);
		vault.bitcoin_locked = 1000;
		assert_eq!(vault.get_activated_securitization(), 1000);

		vault.securitization = 1000;
		vault.bitcoin_locked = 500; // 500 free
		assert_eq!(vault.get_activated_securitization(), 500);

		vault.securitization = 1000;
		vault.bitcoin_locked = 500; // 500 free
		vault.securitization_ratio = FixedU128::from_float(2.0);
		assert_eq!(vault.get_activated_securitization(), 1000);

		vault.securitization = 2000;
		vault.bitcoin_locked = 800;
		// can only 2x the amount bonded
		assert_eq!(vault.get_activated_securitization(), 1600);

		vault.bitcoin_pending = 600;
		assert_eq!(vault.get_activated_securitization(), 400);
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
				bitcoin_xpubkey: keys(),
				securitization: 1000,
				securitization_ratio: FixedU128::from_float(2.0),
			}
		));
		assert_eq!(Balances::reserved_balance(1), 1000);

		VaultsById::<Test>::mutate(1, |vault| {
			if let Some(vault) = vault {
				vault.bitcoin_locked = 499;
			}
		});
		// amount eligible for mining is 2x the bitcoin argons (+2x), but capped at the 1000 which
		// have been securitization
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_activated_securitization(), 499 * 2);

		assert_err!(
			Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 997, FixedU128::from_float(2.0)),
			Error::<Test>::VaultReductionBelowSecuritization
		);
		// can't reduce the securitization
		assert_err!(
			Vaults::modify_funding(RuntimeOrigin::signed(1), 1, 997, FixedU128::from_float(1.5)),
			Error::<Test>::InvalidSecuritization
		);

		assert_ok!(Vaults::modify_funding(
			RuntimeOrigin::signed(1),
			1,
			998,
			FixedU128::from_float(2.0)
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_ratio,
			FixedU128::from_float(2.0)
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().free_balance(), 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_minimum_securitization_needed(), 998);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_recovery_securitization(), 499);

		System::assert_last_event(
			Event::VaultModified {
				vault_id: 1,
				securitization: 998,
				securitization_ratio: FixedU128::from_float(2.0),
			}
			.into(),
		);
		// should have returned the difference
		assert_eq!(Balances::reserved_balance(1), 998);
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
				bitcoin_xpubkey: keys(),
				securitization: 100_000,
				securitization_ratio: FixedU128::from_float(2.0),
			}
		));
		assert_eq!(Balances::free_balance(1), 101_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 100_000);

		let amount = 40_000;
		let obligation =
			Vaults::create_obligation(1, &2, amount, 500, 1440 * 365).expect("bonding failed");
		let fee = obligation.total_fee;
		assert_eq!(obligation.total_fee, 401);
		assert_eq!(obligation.prepaid_fee, 401);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.bitcoin_locked, 40_000);
		assert_eq!(vault.securitization, 100_000);

		assert_ok!(Vaults::close(RuntimeOrigin::signed(1), 1));
		// only need to preserve 2x
		System::assert_last_event(
			Event::VaultClosed { vault_id: 1, remaining_securitization: 80_000, released: 20_000 }
				.into(),
		);
		assert_eq!(
			Balances::free_balance(1),
			vault_owner_balance - 80_000 + obligation.prepaid_fee
		);
		assert!(VaultsById::<Test>::get(1).unwrap().is_closed);

		// set to full fee block
		CurrentTick::set(1440 * 365 + 1);
		// now when we complete an obligation, it should return the funds to the vault
		assert_ok!(Vaults::cancel_obligation(1));
		// should release the 1000 from the bitcoin lock and the 2000 in securitization
		assert_eq!(Balances::free_balance(1), vault_owner_balance + fee);
		assert_eq!(Balances::free_balance(2), 100_000 - fee);
		assert_err!(
			Vaults::create_obligation(1, &2, 1000, 500, 1440 * 365),
			ObligationError::VaultClosed
		);
	});
}

#[test]
fn it_can_create_obligation() {
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
				bitcoin_xpubkey: keys(),
				securitization: 500_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		assert_eq!(Balances::free_balance(1), 500_000);

		set_argons(2, 6_000);
		let obligation =
			Vaults::create_obligation(1, &2, 500_000, 500, 2440).expect("bonding failed");

		let total_fee = obligation.total_fee;
		let apr_fee = (0.01f64 * 500_000f64) as u128;
		assert_eq!(total_fee, apr_fee + 1000);
		assert_eq!(Balances::free_balance(2), 6_000 - total_fee);
		assert_eq!(Balances::free_balance(1), 500_000 + total_fee);
		// if we cancel the obligation, the fee won't be returned
		assert_ok!(Vaults::cancel_obligation(1));
		assert_eq!(Balances::free_balance(1), 500_000 + total_fee);
		assert_eq!(Balances::free_balance(2), 6_000 - total_fee);

		let current_frame_id = CurrentFrameId::get();
		let vault_revenue = PerFrameFeeRevenueByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 1);
		assert_eq!(vault_revenue[0].frame_id, current_frame_id);
		assert_eq!(vault_revenue[0].fee_revenue, total_fee);
		assert_eq!(vault_revenue[0].bitcoin_locks_market_value, 500_000);
		assert_eq!(vault_revenue[0].bitcoin_locks_total_satoshis, 500);
		assert_eq!(vault_revenue[0].bitcoin_locks_created, 1);
	});
}

#[test]
fn it_handles_overflowing_metrics() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);
		for frame_id in 1..50 {
			CurrentFrameId::set(frame_id);
			for vault_id in 0..10 {
				for _ in 0..100 {
					Vaults::update_vault_metrics(vault_id, 1, 1000, 10_000, 1000, 0).unwrap();
				}
			}
		}
		let vault_revenue = PerFrameFeeRevenueByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 10);
		assert_eq!(vault_revenue[0].frame_id, 49);
		assert_eq!(vault_revenue[0].fee_revenue, 1000 * 100);
		assert_eq!(vault_revenue[0].bitcoin_locks_market_value, 10_000 * 100);
		assert_eq!(vault_revenue[0].bitcoin_locks_total_satoshis, 1000 * 100);

		assert_eq!(vault_revenue[9].frame_id, 40);
		assert_eq!(vault_revenue[9].fee_revenue, 1000 * 100);
		assert_eq!(vault_revenue[9].bitcoin_locks_market_value, 10_000 * 100);

		assert_eq!(PerFrameFeeRevenueByVault::<Test>::get(9).len(), 10);
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
					liquidity_pool_profit_sharing: Permill::zero(),
				},
				bitcoin_xpubkey: keys(),
				securitization: 100_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		assert_eq!(Balances::free_balance(1), 900_000);
		let _ = Vaults::create_obligation(1, &2, 100_000, 500, 14_400).expect("bonding failed");

		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().get_activated_securitization(),
			100_000,
			"its 100k until the bitcoin locks marks the funds as pending"
		);

		Vaults::modify_pending_bitcoin_funds(1, 100_000, false).unwrap();
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_activated_securitization(), 0);

		Vaults::modify_pending_bitcoin_funds(1, 90_000, true).unwrap();
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_activated_securitization(), 90_000);
	});
}

#[test]
fn it_can_burn_an_obligation() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		let terms = default_terms(FixedU128::zero());
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				bitcoin_xpubkey: keys(),
				securitization: 100_000,
				securitization_ratio: FixedU128::one(),
			}
		));
		assert_eq!(Balances::free_balance(1), 900_000);

		set_argons(2, 2_000);
		let obligation =
			Vaults::create_obligation(1, &2, 100_000, 500, 2440).expect("bonding failed");
		let total_fee = obligation.total_fee;
		let paid = obligation.prepaid_fee;

		assert_eq!(total_fee, 0);
		assert_eq!(paid, 0);
		assert_eq!(Balances::free_balance(2), 2_000);

		assert_ok!(Vaults::burn_vault_bitcoin_obligation(1, 100_000));

		assert_eq!(Balances::free_balance(1), 900_000);
		assert_eq!(Balances::total_balance(&1), 900_000, "Burned from the vault owner");
		assert_eq!(Balances::free_balance(2), 2_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bitcoin_locked, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 0);
	});
}

#[test]
fn it_can_recoup_reduced_value_bitcoins_from_create_obligation() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(2, 2_000);

		set_argons(1, 200_200);

		let terms = default_terms(FixedU128::from_float(0.001));
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				bitcoin_xpubkey: keys(),
				securitization: 200_000,
				securitization_ratio: FixedU128::one(),
			}
		));

		assert_eq!(Balances::free_balance(1), 200);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 200_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bitcoin_locked, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 200_000);

		let obligation =
			Vaults::create_obligation(1, &2, 100_000, 500, 1440 * 365).expect("bonding failed");
		let total_fee = obligation.total_fee;
		let paid = obligation.prepaid_fee;

		assert_eq!(Balances::free_balance(2), 1900, "fee on hold");
		assert_eq!(Balances::free_balance(1), 300, "it doesn't actually mint anything");
		assert_eq!(total_fee, 100);
		assert_eq!(paid, 100);

		assert_eq!(
			Vaults::compensate_lost_bitcoin(1, 50_000, 50_000).expect("compensation failed"),
			(0, 0)
		);

		assert_eq!(
			Balances::total_balance(&1),
			300 + 150_000,
			"should burn 50 from the held funds"
		);
		// should keep the rest on hold
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 150_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bitcoin_locked, 50_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().securitization, 150_000);
	});
}

#[test]
fn it_can_recoup_increased_value_bitcoins_from_securitizations() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(2, 2_000);

		set_argons(1, 350_200);

		let terms = default_terms(FixedU128::from_float(0.001));
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				bitcoin_xpubkey: keys(),
				securitization: 350_000,
				securitization_ratio: FixedU128::from_float(2.0),
			}
		));

		assert_eq!(Balances::free_balance(1), 200);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 350_000);

		let obligation =
			Vaults::create_obligation(1, &2, 50_000, 500, 1440 * 365).expect("bonding failed");
		let total_fee = obligation.total_fee;
		let paid = obligation.prepaid_fee;
		assert_eq!(total_fee, 50);
		assert_eq!(paid, 50);
		assert_eq!(Balances::free_balance(2), 2_000 - 50);

		assert_eq!(
			Vaults::compensate_lost_bitcoin(1, 200_000, 50_000).expect("compensation failed"),
			(0, 100_000 - 50_000), /* should max out at an extra 2x obligation amount, with 50k
			                        * already paid to user */
			"gets back out of securitization"
		);
		// 50k burned, 50k sent to user
		assert_eq!(Balances::total_balance(&1), 350_200 - 100_000 + 50);

		assert_eq!(Balances::free_balance(2), 2_000 + 50_000 - 50);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.bitcoin_locked, 0);
		assert_eq!(vault.securitization, 350_000 - 100_000);
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
fn it_should_allow_multiple_vaults_per_account() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		let terms = default_terms(TEN_PCT);
		let config = VaultConfig {
			terms,
			bitcoin_xpubkey: keys(),
			securitization: 100_000,
			securitization_ratio: FixedU128::one(),
		};
		set_argons(1, 1_000_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));
		assert_eq!(Balances::free_balance(1), 900_000);

		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		assert_eq!(Balances::free_balance(1), 800_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 200_000);

		assert_eq!(NextVaultId::<Test>::get(), Some(3u32));
	});
}

#[test]
fn it_can_schedule_term_changes() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let mut terms = default_terms(TEN_PCT);
		let config = VaultConfig {
			terms: terms.clone(),
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
			bitcoin_xpubkey: keys(),
			securitization: 1_000_000_000,
			securitization_ratio: FixedU128::one(),
		};
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));

		let amount = 1_000_000;

		CurrentTick::set(1);
		assert_ok!(Vaults::create_obligation(1, &2, amount, 500, 365));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bitcoin_locked, 1_000_000);
		assert_eq!(
			ObligationsById::<Test>::get(1).unwrap(),
			Obligation {
				obligation_id: 1,
				amount,
				fund_type: FundType::LockedBitcoin,
				total_fee: 10000,
				prepaid_fee: 10000,
				vault_id: 1,
				expiration: ObligationExpiration::BitcoinBlock(365),
				beneficiary: 2,
				start_tick: 1,
				bitcoin_annual_percent_rate: Some(FixedU128::from_float(0.01)),
			}
		);

		assert_eq!(BitcoinLockCompletions::<Test>::get(365).to_vec(), vec![1]);

		// expire it
		System::set_block_number(10);
		LastBitcoinHeightChange::set((364, 364));
		Vaults::on_initialize(10);
		assert!(ObligationsById::<Test>::get(1).is_some());
		assert_eq!(BitcoinLockCompletions::<Test>::get(365).to_vec(), vec![1]);

		System::set_block_number(11);
		LastBitcoinHeightChange::set((364, 365));
		Vaults::on_initialize(11);
		assert_eq!(ObligationsById::<Test>::get(1), None);
		assert_eq!(BitcoinLockCompletions::<Test>::get(365).len(), 0);

		assert_eq!(VaultsById::<Test>::get(1).unwrap().bitcoin_locked, 0);
	});
}
