use bitcoin::{
	bip32::{ChildNumber, Xpriv, Xpub},
	key::Secp256k1,
};
use frame_support::{
	assert_err, assert_noop, assert_ok,
	pallet_prelude::Hooks,
	traits::{
		fungible::{Inspect, InspectHold, Mutate},
		tokens::Preservation,
	},
	BoundedVec,
};
use k256::elliptic_curve::rand_core::{OsRng, RngCore};
use sp_runtime::{traits::Zero, FixedU128, Perbill};

use crate::{
	mock::{Vaults, *},
	pallet::{
		BitcoinLockCompletions, BondedBitcoinCompletions, NextBondedBitcoinPoolEntrants,
		NextVaultId, ObligationsById, PendingTermsModificationsByTick, VaultXPubById, VaultsById,
	},
	BidPoolEntrant, Error, Event, HoldReason, VaultConfig,
};
use argon_primitives::{
	bitcoin::{CompressedBitcoinPubkey, OpaqueBitcoinXpub},
	vault::{
		BitcoinObligationProvider, BondedBitcoinsBidPoolProvider, FundType, Obligation,
		ObligationError, ObligationExpiration, ReleaseFundsResult, VaultTerms,
	},
};

const TEN_PCT: FixedU128 = FixedU128::from_rational(10, 100);

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
	VaultTerms { bitcoin_annual_percent_rate: pct, bitcoin_base_fee: 0 }
}

fn default_vault() -> VaultConfig<Balance> {
	VaultConfig {
		terms: default_terms(TEN_PCT),
		bitcoin_xpubkey: keys(),
		locked_bitcoin_argons_allocated: 50_000,
		bonded_bitcoin_argons_allocated: 50_000,
		added_securitization_percent: FixedU128::zero(),
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
				activation_tick: CurrentTick::get(),
				operator_account_id: 1,
				locked_bitcoin_argons: 50_000,
				bonded_bitcoin_argons: 50_000,
				added_securitization_percent: FixedU128::zero(),
			}
			.into(),
		);

		assert!(System::account_exists(&1));

		assert_eq!(Balances::reserved_balance(1), 100_000);
		assert_eq!(Balances::free_balance(1), 10);

		assert_eq!(NextVaultId::<Test>::get(), Some(2u32));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().operator_account_id, 1);
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
				activation_tick: 100,
				operator_account_id: 1,
				locked_bitcoin_argons: 50_000,
				bonded_bitcoin_argons: 50_000,
				added_securitization_percent: FixedU128::zero(),
			}
			.into(),
		);

		assert_err!(
			Vaults::create_obligation(1, &2, 50_000, 100, 100,),
			ObligationError::VaultNotYetActive
		);
	});
}

#[test]
fn it_can_add_securitization_to_a_vault() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let mut config = default_vault();
		config.added_securitization_percent = TEN_PCT;
		set_argons(1, 110_010);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));

		System::assert_last_event(
			Event::VaultCreated {
				vault_id: 1,
				activation_tick: CurrentTick::get(),
				operator_account_id: 1,
				locked_bitcoin_argons: 50_000,
				bonded_bitcoin_argons: 50_000,
				added_securitization_percent: TEN_PCT,
			}
			.into(),
		);
		assert!(System::account_exists(&1));
		let bitcoin_securitization = 50_000 * 10 / 100;
		assert_eq!(Balances::reserved_balance(1), 100_000 + bitcoin_securitization);
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
		config.bonded_bitcoin_argons_allocated = 1000;
		config.locked_bitcoin_argons_allocated = 1000;

		set_argons(1, 20_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));
		assert_eq!(Balances::reserved_balance(1), 2000);

		assert_noop!(
			Vaults::modify_funding(
				RuntimeOrigin::signed(2),
				1,
				1000,
				1000,
				FixedU128::from_float(2.0)
			),
			Error::<Test>::NoPermissions
		);

		assert_ok!(Vaults::modify_funding(
			RuntimeOrigin::signed(1),
			1,
			1000,
			1010,
			FixedU128::from_float(2.0)
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().added_securitization_percent,
			FixedU128::from_float(2.0)
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_added_securitization_needed(), 1010 * 2);
		System::assert_last_event(
			Event::VaultModified {
				vault_id: 1,
				locked_bitcoin_argons: 1010,
				bonded_bitcoin_argons: 1000,
				added_securitization_percent: FixedU128::from_float(2.0),
			}
			.into(),
		);
		assert_eq!(Balances::reserved_balance(1), 2010 + 2020);
	});
}

#[test]
fn it_can_correctly_calculate_available_bonded_bitcoins() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		set_argons(1, 20_000);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms: default_terms(TEN_PCT),
				bitcoin_xpubkey: keys(),
				locked_bitcoin_argons_allocated: 1000,
				bonded_bitcoin_argons_allocated: 1000,
				added_securitization_percent: FixedU128::from_float(0.0),
			}
		));

		let mut vault = VaultsById::<Test>::get(1).unwrap();
		let slots = 10;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 0);
		vault.locked_bitcoin_argons.reserved = 500;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 500 / 10);
		vault.locked_bitcoin_argons.reserved = 1000;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 1000 / 10);

		vault.locked_bitcoin_argons.allocated = 1000;
		vault.locked_bitcoin_argons.reserved = 500; // 500 free
		vault.bonded_bitcoin_argons.allocated = 500;
		vault.bonded_bitcoin_argons.reserved = 0;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 500 / 10);
		vault.bonded_bitcoin_argons.reserved = 500;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 0);
		vault.bonded_bitcoin_argons.allocated = 1000;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 0);

		////// ADDED Securitization

		vault.added_securitization_percent = FixedU128::from_float(2.0);
		vault.added_securitization_argons = 1000; // 1500 total now possible
		vault.bonded_bitcoin_argons.allocated = 500;
		vault.bonded_bitcoin_argons.reserved = 0;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 500 / 10);
		vault.bonded_bitcoin_argons.reserved = 500;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 0);

		// now we have 1000 allocated
		vault.bonded_bitcoin_argons.reserved = 0;
		vault.bonded_bitcoin_argons.allocated = 1000;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 1000 / 10);
		vault.bonded_bitcoin_argons.reserved = 500;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 1000 / 10);

		// can do up to 1x + 2x extra
		vault.bonded_bitcoin_argons.reserved = 0;
		vault.bonded_bitcoin_argons.allocated = 1500;
		vault.locked_bitcoin_argons.reserved = 500;
		vault.locked_bitcoin_argons.allocated = 1000;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 1500 / 10);

		vault.bonded_bitcoin_argons.reserved = 1000;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 1500 / 10);

		vault.bonded_bitcoin_argons.reserved = 1500;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 0);

		vault.added_securitization_argons = 2000;
		vault.locked_bitcoin_argons.reserved = 1000;
		vault.locked_bitcoin_argons.allocated = 1000;
		vault.bonded_bitcoin_argons.reserved = 0;
		vault.bonded_bitcoin_argons.allocated = 1500;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 1500 / 10);
		vault.bonded_bitcoin_argons.allocated = 3000;
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 3000 / 10);

		vault.locked_bitcoin_argons.reserved = 800;
		// can only 2x the amount bonded
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), (800 + 1600) / 10);

		// now check that the bonded is calculated right once more is reserved
		vault.bonded_bitcoin_argons.allocated = 2400;
		vault.bonded_bitcoin_argons.reserved = 2200;
		// maxes out at remainder
		assert_eq!(vault.bonded_bitcoins_for_pool(slots), 200);
	});
}

#[test]
fn it_can_reduce_vault_funds_down_to_bonded() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		set_argons(1, 20_000);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms: default_terms(TEN_PCT),
				bitcoin_xpubkey: keys(),
				locked_bitcoin_argons_allocated: 1000,
				bonded_bitcoin_argons_allocated: 1000,
				added_securitization_percent: FixedU128::from_float(2.0),
			}
		));
		assert_eq!(Balances::reserved_balance(1), 4000);

		VaultsById::<Test>::mutate(1, |vault| {
			if let Some(vault) = vault {
				vault.locked_bitcoin_argons.reserved = 500;
			}
		});
		// amount eligible for mining is 3x the bitcoin argons (+2x), but capped at the 1000 which
		// have been allocated
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bonded_bitcoins_for_pool(10), 1000 / 10);

		assert_err!(
			Vaults::modify_funding(
				RuntimeOrigin::signed(1),
				1,
				1000,
				499,
				FixedU128::from_float(2.0)
			),
			Error::<Test>::VaultReductionBelowAllocatedFunds
		);
		// can't reduce the securitization
		assert_err!(
			Vaults::modify_funding(
				RuntimeOrigin::signed(1),
				1,
				1000,
				500,
				FixedU128::from_float(1.5)
			),
			Error::<Test>::InvalidSecuritization
		);

		assert_ok!(Vaults::modify_funding(
			RuntimeOrigin::signed(1),
			1,
			1000,
			500,
			FixedU128::from_float(2.0)
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().added_securitization_percent,
			FixedU128::from_float(2.0)
		);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().get_added_securitization_needed(), 500 * 2);
		System::assert_last_event(
			Event::VaultModified {
				vault_id: 1,
				locked_bitcoin_argons: 500,
				bonded_bitcoin_argons: 1000,
				added_securitization_percent: FixedU128::from_float(2.0),
			}
			.into(),
		);
		// should have returned the difference
		assert_eq!(Balances::reserved_balance(1), 1500 + 1000);

		// amount eligible for bonded bitcoin pool doesn't change
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bonded_bitcoins_for_pool(10), 1000 / 10);
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
				locked_bitcoin_argons_allocated: 50_000,
				bonded_bitcoin_argons_allocated: 50_000,
				added_securitization_percent: FixedU128::from_float(2.0),
			}
		));
		assert_eq!(Balances::free_balance(1), 1000);

		let amount = 50_000;
		let obligation = Vaults::create_obligation(1, &2, amount, 1440 * 365, 1440 * 365)
			.expect("bonding failed");
		let fee = obligation.total_fee;
		assert_eq!(obligation.total_fee, 501);
		assert_eq!(obligation.prepaid_fee, 1);

		let bonded_bitcoin_argons = 400;
		VaultsById::<Test>::mutate(1, |vault| {
			if let Some(vault) = vault {
				vault.bonded_bitcoin_argons.reserved = bonded_bitcoin_argons;
			}
		});

		assert_ok!(Vaults::close(RuntimeOrigin::signed(1), 1));
		System::assert_last_event(
			Event::VaultClosed {
				vault_id: 1,
				securitization_still_reserved: amount * 2,
				locked_bitcoin_amount_still_reserved: amount,
				bonded_bitcoin_amount_still_reserved: bonded_bitcoin_argons,
			}
			.into(),
		);
		assert_eq!(
			Balances::free_balance(1),
			vault_owner_balance - (amount * 2) - amount - bonded_bitcoin_argons +
				obligation.prepaid_fee
		);
		assert!(VaultsById::<Test>::get(1).unwrap().is_closed);

		// set to full fee block
		CurrentTick::set(1440 * 365 + 1);
		// now when we complete an obligation, it should return the funds to the vault
		assert_ok!(Vaults::cancel_obligation(1));
		// should release the 1000 from the bitcoin lock and the 2000 in securitization
		assert_eq!(Balances::free_balance(1), vault_owner_balance - bonded_bitcoin_argons + fee);
		assert_eq!(Balances::free_balance(2), 100_000 - fee);
		assert_err!(
			Vaults::create_obligation(1, &2, 1000, 1440 * 365, 1440 * 365),
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
				locked_bitcoin_argons_allocated: 500_000,
				bonded_bitcoin_argons_allocated: 0,
				added_securitization_percent: FixedU128::zero(),
			}
		));
		assert_eq!(Balances::free_balance(1), 500_000);

		set_argons(2, 2_000);
		let obligation =
			Vaults::create_obligation(1, &2, 500_000, 2440, 2440).expect("bonding failed");

		let total_fee = obligation.total_fee;
		let paid = obligation.prepaid_fee;
		let per_block_fee = 0.01f64 * 500_000f64 / (1440f64 * 365f64);
		// fee is 9 microgons per block per argon (rented 5 argons)
		let fee = (2440f64 * per_block_fee) as u128;
		assert_eq!(total_fee, fee + 1000);
		assert_eq!(paid, 1000);
		assert_eq!(Balances::free_balance(2), 2_000 - fee - paid);
		assert_eq!(Balances::balance_on_hold(&HoldReason::ObligationFee.into(), &2), fee);
		assert_eq!(Balances::free_balance(1), 500_000 + paid);
		assert_eq!(Balances::balance_on_hold(&HoldReason::ObligationFee.into(), &1), 0);

		// if we cancel the obligation, the prepaid won't be returned
		assert_ok!(Vaults::cancel_obligation(1));
		assert_eq!(Balances::free_balance(1), 500_000 + paid);
		assert_eq!(Balances::free_balance(2), 2_000 - paid);
	});
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
				bitcoin_xpubkey: keys(),
				locked_bitcoin_argons_allocated: 100_000,
				bonded_bitcoin_argons_allocated: 100_000,
				added_securitization_percent: FixedU128::zero(),
			}
		));
		assert_eq!(Balances::free_balance(1), 800_000);
		let _ = Vaults::create_obligation(1, &2, 100_000, 14_400, 14_400).expect("bonding failed");

		let slots = 10;
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().bonded_bitcoins_for_pool(slots),
			100_000 / 10,
			"its 100k until the bitcoin locks marks the funds as pending"
		);

		Vaults::modify_pending_bitcoin_funds(1, 100_000, false).unwrap();
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bonded_bitcoins_for_pool(slots), 0);

		Vaults::modify_pending_bitcoin_funds(1, 90_000, true).unwrap();
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().bonded_bitcoins_for_pool(slots),
			90_000 / 10
		);
	});
}

#[test]
fn it_can_charge_prorated_obligation_fees() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		let terms = VaultTerms {
			bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
			bitcoin_base_fee: 123,
		};

		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				bitcoin_xpubkey: keys(),
				locked_bitcoin_argons_allocated: 500_000,
				bonded_bitcoin_argons_allocated: 0,
				added_securitization_percent: FixedU128::zero(),
			}
		));
		assert_eq!(Balances::free_balance(1), 500_000);

		set_argons(2, 2_000);
		let obligation =
			Vaults::create_obligation(1, &2, 100_000, 14_400, 14_400).expect("bonding failed");
		let total_fee = obligation.total_fee;
		let paid = obligation.prepaid_fee;

		let per_block_fee = 0.1f64 * 100_000f64 / (1440f64 * 365f64);
		println!("per block fee: {}, total {:?}, paid {:}", per_block_fee, total_fee, paid);
		let apr_fee = (14_400f64 * per_block_fee) as u128;
		let base_fee = 123;
		assert_eq!(base_fee, paid);
		assert_eq!(total_fee, apr_fee + 123);
		assert_eq!(paid, 123);
		assert_eq!(Balances::free_balance(2), 2_000 - apr_fee - paid);
		assert_eq!(Balances::balance_on_hold(&HoldReason::ObligationFee.into(), &2), apr_fee);
		assert_eq!(Balances::free_balance(1), 500_000 + paid);

		CurrentTick::set(5 + 1440);
		// if we cancel the obligation, the prepaid won't be returned
		let to_return_res = Vaults::cancel_obligation(1);
		assert!(to_return_res.is_ok());
		let expected_apr_fee = (per_block_fee * 1440f64) as u128;
		assert_eq!(
			to_return_res.unwrap(),
			ReleaseFundsResult {
				returned_to_beneficiary: total_fee - expected_apr_fee - paid,
				paid_to_vault: expected_apr_fee,
			}
		);

		assert_eq!(Balances::free_balance(1), 500_000 + paid + expected_apr_fee);
		assert_eq!(Balances::free_balance(2), 2_000 - paid - expected_apr_fee);
		assert_eq!(Balances::balance_on_hold(&HoldReason::ObligationFee.into(), &2), 0);
	});
}

#[test]
fn it_can_burn_a_bond() {
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
				locked_bitcoin_argons_allocated: 100_000,
				bonded_bitcoin_argons_allocated: 100_000,
				added_securitization_percent: FixedU128::zero(),
			}
		));
		assert_eq!(Balances::free_balance(1), 800_000);

		set_argons(2, 2_000);
		let obligation =
			Vaults::create_obligation(1, &2, 100_000, 2440, 2440).expect("bonding failed");
		let total_fee = obligation.total_fee;
		let paid = obligation.prepaid_fee;

		assert_eq!(total_fee, 0);
		assert_eq!(paid, 0);
		assert_eq!(Balances::free_balance(2), 2_000);

		assert_ok!(Vaults::burn_vault_bitcoin_obligation(1, 100_000));

		assert_eq!(Balances::free_balance(1), 800_000);
		assert_eq!(Balances::total_balance(&1), 900_000);
		assert_eq!(Balances::free_balance(2), 2_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().locked_bitcoin_argons.reserved, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().locked_bitcoin_argons.allocated, 0);
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
				locked_bitcoin_argons_allocated: 100_000,
				bonded_bitcoin_argons_allocated: 100_000,
				added_securitization_percent: FixedU128::zero(),
			}
		));

		assert_eq!(Balances::free_balance(1), 200);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 200_000);

		let obligation = Vaults::create_obligation(1, &2, 100_000, 1440 * 365, 1440 * 365)
			.expect("bonding failed");
		let total_fee = obligation.total_fee;
		let paid = obligation.prepaid_fee;

		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ObligationFee.into(), &2),
			100,
			"should hold fee"
		);
		assert_eq!(Balances::free_balance(2), 1900, "fee on hold");
		assert_eq!(Balances::free_balance(1), 200, "it doesn't actually mint anything");
		assert_eq!(total_fee, 100);
		assert_eq!(paid, 0);

		assert_eq!(
			Vaults::compensate_lost_bitcoin(1, 50_000, 50_000).expect("compensation failed"),
			(0, 0)
		);

		assert_eq!(
			Balances::total_balance(&1),
			200 + 150_000,
			"should burn 50 from the held funds"
		);
		// should keep the rest on hold
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 150_000);
		// returns the fee
		assert_eq!(Balances::free_balance(2), 2_000, "has back original funds");
		assert_eq!(VaultsById::<Test>::get(1).unwrap().locked_bitcoin_argons.reserved, 50_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().locked_bitcoin_argons.allocated, 50_000);
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
				locked_bitcoin_argons_allocated: 100_000,
				bonded_bitcoin_argons_allocated: 50_000,
				added_securitization_percent: FixedU128::from_float(2.0),
			}
		));

		assert_eq!(Balances::free_balance(1), 200);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 350_000);

		let obligation = Vaults::create_obligation(1, &2, 50_000, 1440 * 365, 1440 * 365)
			.expect("bonding failed");
		let total_fee = obligation.total_fee;
		let paid = obligation.prepaid_fee;
		assert_eq!(total_fee, 50);
		assert_eq!(paid, 0);
		assert_eq!(Balances::free_balance(2), 2_000 - 50);

		assert_eq!(
			Vaults::compensate_lost_bitcoin(1, 200_000, 50_000).expect("compensation failed"),
			(0, 100_000), /* should max out at an extra 2x obligation amount, with 50k already
			               * paid to user */
			"gets back out of securitization"
		);
		// 50k burned, 100k sent to
		assert_eq!(Balances::total_balance(&1), 350_200 - 100_000 - 50000);
		// bonded bitcoins are not at risk
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1),
			350_000 - 150_000
		);
		// returns the fee
		assert_eq!(Balances::free_balance(2), 2_000 + 100_000);
		let vault = VaultsById::<Test>::get(1).unwrap();
		assert_eq!(vault.locked_bitcoin_argons.reserved, 0);
		assert_eq!(vault.locked_bitcoin_argons.allocated, 0);
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
				locked_bitcoin_argons_allocated: 100_000,
				bonded_bitcoin_argons_allocated: 100_000,
				added_securitization_percent: FixedU128::zero(),
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
			locked_bitcoin_argons_allocated: 100_000,
			bonded_bitcoin_argons_allocated: 100_000,
			added_securitization_percent: FixedU128::zero(),
		};
		set_argons(1, 1_000_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));
		assert_eq!(Balances::free_balance(1), 800_000);

		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config));
		assert_eq!(Balances::free_balance(1), 600_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 400_000);

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
			locked_bitcoin_argons_allocated: 100_000,
			bonded_bitcoin_argons_allocated: 100_000,
			added_securitization_percent: FixedU128::zero(),
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
			locked_bitcoin_argons_allocated: 100_000,
			bonded_bitcoin_argons_allocated: 100_000,
			added_securitization_percent: FixedU128::zero(),
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
fn it_can_calculate_apr() {
	new_test_ext().execute_with(|| {
		let percent = FixedU128::from_float(10.0); // 1000%
		assert_eq!(Vaults::calculate_tick_fees(percent, 1000, 1440), 27);
		assert_eq!(Vaults::calculate_tick_fees(percent, 100, 1440 * 365), 1000);
		assert_eq!(Vaults::calculate_tick_fees(percent, 99, 1440 * 365), 990);
		assert_eq!(Vaults::calculate_tick_fees(percent, 365000, 1440 * 365), 3650000);
		assert_eq!(Vaults::calculate_tick_fees(percent, 365000, 1440), 9999);
	})
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
fn should_cleanup_multiple_completion_ticks() {
	new_test_ext().execute_with(|| {
		CurrentTick::set(10);
		PreviousTick::set(5);

		BondedBitcoinCompletions::<Test>::set(6, BoundedVec::truncate_from(vec![1, 2]));
		BondedBitcoinCompletions::<Test>::set(9, BoundedVec::truncate_from(vec![3]));
		BondedBitcoinCompletions::<Test>::set(10, BoundedVec::truncate_from(vec![4]));

		Vaults::on_initialize(10);
		assert_eq!(BondedBitcoinCompletions::<Test>::get(6).len(), 0);
		assert_eq!(BondedBitcoinCompletions::<Test>::get(9).len(), 0);
		assert_eq!(BondedBitcoinCompletions::<Test>::get(10).len(), 0);
	});
}

#[test]
fn it_can_cleanup_at_bitcoin_heights() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 200_000_000_000);
		set_argons(2, 50_000_000);

		let terms = default_terms(FixedU128::from_float(10.0));
		let config = VaultConfig {
			terms: terms.clone(),
			bitcoin_xpubkey: keys(),
			locked_bitcoin_argons_allocated: 1_000_000_000,
			bonded_bitcoin_argons_allocated: 1_000_000_000,
			added_securitization_percent: FixedU128::zero(),
		};
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));

		let amount = 1_000_000;

		CurrentTick::set(1);
		assert_ok!(Vaults::create_obligation(1, &2, amount, 365, 10));
		assert_eq!(VaultsById::<Test>::get(1).unwrap().locked_bitcoin_argons.reserved, 1_000_000);
		assert_eq!(
			ObligationsById::<Test>::get(1).unwrap(),
			Obligation {
				obligation_id: 1,
				amount,
				fund_type: FundType::LockedBitcoin,
				total_fee: 190,
				prepaid_fee: 0,
				vault_id: 1,
				expiration: ObligationExpiration::BitcoinBlock(365),
				beneficiary: 2,
				start_tick: 1,
				bitcoin_annual_percent_rate: Some(FixedU128::from_float(10.0)),
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

		assert_eq!(VaultsById::<Test>::get(1).unwrap().locked_bitcoin_argons.reserved, 0);
	});
}

#[test]
fn it_can_distribute_a_bid_pool() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let bid_pool_account = Vaults::get_bid_pool_account();
		let last_bid_pool = 100_000_000;
		set_argons(bid_pool_account, last_bid_pool);

		System::set_block_number(1);
		set_argons(1, 200_000_000_000);
		set_argons(2, 500_000_000_000);

		let config = VaultConfig {
			terms: VaultTerms {
				bitcoin_annual_percent_rate: FixedU128::zero(),
				bitcoin_base_fee: 0,
			},
			bitcoin_xpubkey: keys(),
			locked_bitcoin_argons_allocated: 1_000_000_000,
			bonded_bitcoin_argons_allocated: 1_000_000_000,
			added_securitization_percent: FixedU128::zero(),
		};
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), config.clone()));
		assert_eq!(Balances::free_balance(1), 198_000_000_000);

		let config = VaultConfig {
			terms: VaultTerms {
				bitcoin_annual_percent_rate: FixedU128::zero(),
				bitcoin_base_fee: 0,
			},
			bitcoin_xpubkey: keys(),
			locked_bitcoin_argons_allocated: 2_000_000_000,
			bonded_bitcoin_argons_allocated: 2_000_000_000,
			added_securitization_percent: FixedU128::zero(),
		};
		assert_ok!(Vaults::create(RuntimeOrigin::signed(2), config.clone()));
		assert_eq!(Balances::free_balance(2), 496_000_000_000);

		VaultsById::<Test>::mutate(1, |vault| {
			if let Some(vault) = vault {
				vault.locked_bitcoin_argons.reserved = 20_000_000;
			}
		});
		VaultsById::<Test>::mutate(2, |vault| {
			if let Some(vault) = vault {
				vault.locked_bitcoin_argons.reserved = 10_000_000;
			}
		});

		Vaults::distribute_and_rotate_bid_pool(1, 100);
		// should burn even without any entrants
		let burn_amount = BurnFromBidPoolAmount::get().mul_ceil(last_bid_pool);
		System::assert_has_event(
			Event::<Test>::BidPoolDistributed {
				bid_pool_distributed: 0,
				bid_pool_burned: burn_amount,
				bid_pool_entrants: 0,
				cohort_id: 1,
			}
			.into(),
		);

		System::assert_last_event(
			Event::<Test>::NextBidPoolAllocated {
				bid_pool_entrants: 2,
				bonded_bitcoin_pool: 30_000_000 / 10,
			}
			.into(),
		);
		assert_eq!(
			NextBondedBitcoinPoolEntrants::<Test>::get().to_vec(),
			vec![
				BidPoolEntrant::<Test> {
					operator_account_id: 2,
					vault_id: 2,
					prorata: Perbill::from_rational(1u32, 3u32),
					bonded_bitcoins: 10_000_000 / 10,
				},
				BidPoolEntrant::<Test> {
					operator_account_id: 1,
					vault_id: 1,
					prorata: Perbill::from_rational(2u32, 3u32),
					bonded_bitcoins: 20_000_000 / 10,
				}
			]
		);
		assert_eq!(ObligationsById::<Test>::iter_keys().collect::<Vec<_>>(), vec![1, 2]);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bonded_bitcoin_argons.reserved, 2_000_000);
		assert_eq!(VaultsById::<Test>::get(2).unwrap().bonded_bitcoin_argons.reserved, 1_000_000);

		// now add another vault
		set_argons(3, 200_000_000_000);
		let config = VaultConfig {
			terms: VaultTerms {
				bitcoin_annual_percent_rate: FixedU128::zero(),
				bitcoin_base_fee: 0,
			},
			bitcoin_xpubkey: keys(),
			locked_bitcoin_argons_allocated: 3_000_000_000,
			bonded_bitcoin_argons_allocated: 3_000_000_000,
			added_securitization_percent: FixedU128::zero(),
		};
		assert_ok!(Vaults::create(RuntimeOrigin::signed(3), config.clone()));
		assert_eq!(Balances::free_balance(3), 194_000_000_000);
		VaultsById::<Test>::mutate(3, |vault| {
			if let Some(vault) = vault {
				vault.locked_bitcoin_argons.reserved = 20_000_010;
			}
		});

		System::reset_events();
		// set this again so we can have a remainder
		let last_bid_pool = 100_000_000;
		set_argons(bid_pool_account, last_bid_pool);
		Vaults::distribute_and_rotate_bid_pool(2, 100);
		System::assert_last_event(
			Event::<Test>::NextBidPoolAllocated {
				bid_pool_entrants: 3,
				bonded_bitcoin_pool: (30_000_000 / 10) + (20_000_010 / 10),
			}
			.into(),
		);

		let burn_amount = BurnFromBidPoolAmount::get().mul_ceil(last_bid_pool);
		let distributed = last_bid_pool - burn_amount;
		System::assert_has_event(
			Event::<Test>::BidPoolDistributed {
				bid_pool_distributed: distributed,
				bid_pool_burned: burn_amount,
				bid_pool_entrants: 2,
				cohort_id: 2,
			}
			.into(),
		);
		assert_eq!(Balances::free_balance(bid_pool_account), 0);

		let account_2_share = Perbill::from_rational(1u32, 3u32).mul_floor(distributed);
		assert_eq!(
			Balances::free_balance(1),
			198_000_000_000u128 + (distributed - account_2_share) /* the remainder goes to the
			                                                       * biggest prorata */
		);
		assert_eq!(Balances::free_balance(2), 496_000_000_000u128 + account_2_share);

		// should not have paid out 3
		assert_eq!(Balances::free_balance(3), 194_000_000_000u128);
		assert_eq!(
			NextBondedBitcoinPoolEntrants::<Test>::get().to_vec(),
			vec![
				BidPoolEntrant::<Test> {
					operator_account_id: 2,
					vault_id: 2,
					prorata: Perbill::from_rational(1000000u32, 5000001u32),
					bonded_bitcoins: 10_000_000 / 10,
				},
				BidPoolEntrant::<Test> {
					operator_account_id: 1,
					vault_id: 1,
					prorata: Perbill::from_rational(2000000u32, 5000001u32),
					bonded_bitcoins: 20_000_000 / 10,
				},
				BidPoolEntrant::<Test> {
					operator_account_id: 3,
					vault_id: 3,
					prorata: Perbill::from_rational(2000001u32, 5000001u32),
					bonded_bitcoins: 20_000_010 / 10,
				}
			]
		);
	});
}
