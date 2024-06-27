use frame_support::{
	assert_err, assert_noop, assert_ok,
	traits::{
		fungible::{Inspect, InspectHold, Mutate},
		tokens::Preservation,
	},
};
use sp_core::bounded_vec;
use sp_runtime::{traits::Zero, BoundedVec, FixedU128};
use ulx_primitives::{
	bitcoin::BitcoinPubkeyHash,
	bond::{Bond, BondError, BondExpiration, BondType, VaultProvider},
};

use crate::{
	mock::{Vaults, *},
	pallet::{NextVaultId, VaultsById},
	Error, Event, HoldReason,
};

const TEN_PCT: FixedU128 = FixedU128::from_rational(10, 100);

fn keys() -> BoundedVec<BitcoinPubkeyHash, MaxVaultBitcoinPubkeys> {
	bounded_vec![BitcoinPubkeyHash([0u8; 20])]
}
#[test]
fn it_can_create_a_vault() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		assert_noop!(
			Vaults::create(
				RuntimeOrigin::signed(1),
				TEN_PCT,
				TEN_PCT,
				50_000,
				50_000,
				FixedU128::zero(),
				keys()
			),
			Error::<Test>::InsufficientFunds
		);

		set_argons(1, 100_010);

		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			TEN_PCT,
			TEN_PCT,
			50_000,
			50_000,
			FixedU128::zero(),
			keys()
		));
		System::assert_last_event(
			Event::VaultCreated {
				vault_id: 1,
				operator_account_id: 1,
				bitcoin_argons: 50_000,
				mining_argons: 50_000,
				securitization_percent: FixedU128::zero(),
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
fn it_can_add_securitization_to_a_vault() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		set_argons(1, 110_010);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			TEN_PCT,
			TEN_PCT,
			50_000,
			50_000,
			TEN_PCT, // 10%
			keys()
		));

		System::assert_last_event(
			Event::VaultCreated {
				vault_id: 1,
				operator_account_id: 1,
				bitcoin_argons: 50_000,
				mining_argons: 50_000,
				securitization_percent: TEN_PCT,
			}
			.into(),
		);
		assert!(System::account_exists(&1));
		let bitcoin_securitization = 50_000 * 10 / 100;
		assert_eq!(Balances::reserved_balance(1), 100_000 + bitcoin_securitization);

		// can only go up to 200% (2x)
		assert_err!(
			Vaults::create(
				RuntimeOrigin::signed(2),
				TEN_PCT,
				TEN_PCT,
				1,
				1,
				FixedU128::from_float(2.1),
				keys()
			),
			Error::<Test>::MaxSecuritizationPercentExceeded
		);
	});
}

#[test]
fn it_can_modify_a_vault() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		set_argons(1, 20_000);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			TEN_PCT,
			TEN_PCT,
			1000,
			1000,
			FixedU128::zero(),
			keys()
		));
		assert_eq!(Balances::reserved_balance(1), 2000);

		assert_noop!(
			Vaults::modify(RuntimeOrigin::signed(2), 1, 1000, 1000, FixedU128::from_float(2.0)),
			Error::<Test>::NoPermissions
		);

		assert_ok!(Vaults::modify(
			RuntimeOrigin::signed(1),
			1,
			1000,
			1010,
			FixedU128::from_float(2.0)
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_percent,
			FixedU128::from_float(2.0)
		);
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().get_minimum_securitization_needed(),
			1010 * 2
		);
		System::assert_last_event(
			Event::VaultModified {
				vault_id: 1,
				bitcoin_argons: 1010,
				mining_argons: 1000,
				securitization_percent: FixedU128::from_float(2.0),
			}
			.into(),
		);
		assert_eq!(Balances::reserved_balance(1), 2010 + 2020);
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
			TEN_PCT,
			TEN_PCT,
			1000,
			1000,
			FixedU128::from_float(2.0),
			keys()
		));
		assert_eq!(Balances::reserved_balance(1), 4000);

		VaultsById::<Test>::mutate(1, |vault| {
			if let Some(vault) = vault {
				vault.bitcoin_argons.bonded = 500;
			}
		});
		// amount eligible for mining is 2x the bitcoin argons
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().amount_eligible_for_mining(),
			500 * 200 / 100
		);

		assert_err!(
			Vaults::modify(RuntimeOrigin::signed(1), 1, 1000, 499, FixedU128::from_float(2.0)),
			Error::<Test>::VaultReductionBelowAllocatedFunds
		);
		// can't reduce the securitization
		assert_err!(
			Vaults::modify(RuntimeOrigin::signed(1), 1, 1000, 500, FixedU128::from_float(1.5)),
			Error::<Test>::InvalidSecuritization
		);

		assert_ok!(Vaults::modify(
			RuntimeOrigin::signed(1),
			1,
			1000,
			500,
			FixedU128::from_float(2.0)
		));
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().securitization_percent,
			FixedU128::from_float(2.0)
		);
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().get_minimum_securitization_needed(),
			500 * 2
		);
		System::assert_last_event(
			Event::VaultModified {
				vault_id: 1,
				bitcoin_argons: 500,
				mining_argons: 1000,
				securitization_percent: FixedU128::from_float(2.0),
			}
			.into(),
		);
		// should have returned the difference
		assert_eq!(Balances::reserved_balance(1), 1500 + 1000);

		// amount eligible for mining doesn't change
		assert_eq!(
			VaultsById::<Test>::get(1).unwrap().amount_eligible_for_mining(),
			500 * 200 / 100
		);
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
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			FixedU128::from_float(0.01),
			FixedU128::from_float(0.01),
			50_000,
			50_000,
			FixedU128::from_float(2.0),
			keys()
		));
		assert_eq!(Balances::free_balance(1), 1000);

		let bond_amount = 50_000;
		let (fee, paid) = Vaults::bond_funds(1, bond_amount, BondType::Bitcoin, 1440 * 365, &2)
			.expect("bonding failed");
		assert_eq!(fee, 500);
		assert_eq!(paid, 1);

		let mining_bond = 400;
		VaultsById::<Test>::mutate(1, |vault| {
			if let Some(vault) = vault {
				vault.mining_argons.bonded = mining_bond;
			}
		});

		assert_ok!(Vaults::close(RuntimeOrigin::signed(1), 1));
		System::assert_last_event(
			Event::VaultClosed {
				vault_id: 1,
				securitization_still_bonded: bond_amount * 2,
				bitcoin_amount_still_bonded: bond_amount,
				mining_amount_still_bonded: mining_bond,
			}
			.into(),
		);
		assert_eq!(
			Balances::free_balance(1),
			vault_owner_balance - (bond_amount * 2) - bond_amount - mining_bond + 1
		);
		assert!(VaultsById::<Test>::get(1).unwrap().is_closed);

		// now when we complete a bond, it should return the funds to the vault
		assert_ok!(Vaults::release_bonded_funds(
			&Bond {
				vault_id: 1,
				bonded_account_id: 2,
				amount: bond_amount,
				prepaid_fee: paid,
				total_fee: fee,
				expiration: BondExpiration::BitcoinBlock(5000),
				bond_type: BondType::Bitcoin,
				utxo_id: Some(1)
			},
			true
		));
		// should release the 1000 from the bitcoin bond and the 2000 in securitization
		assert_eq!(Balances::free_balance(1), vault_owner_balance - mining_bond + fee);
		assert_eq!(Balances::free_balance(2), 100_000 - fee);

		assert_err!(
			Vaults::bond_funds(1, 1000, BondType::Bitcoin, 1440 * 365, &2),
			BondError::VaultClosed
		);
	});
}

#[test]
fn it_can_bond_funds() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			FixedU128::from_float(0.01), // 1%
			FixedU128::zero(),
			500_000,
			0,
			FixedU128::zero(),
			keys()
		));
		assert_eq!(Balances::free_balance(1), 500_000);

		set_argons(2, 2_000);
		let (total_fee, paid) =
			Vaults::bond_funds(1, 500_000, BondType::Bitcoin, 2440, &2).expect("bonding failed");

		let per_block_fee = 0.01f64 * 500_000f64 / (1440f64 * 365f64);
		// fee is 9 milligons per block per argon (rented 5 argons)
		let fee = (2440f64 * per_block_fee) as u128;
		assert_eq!(total_fee, fee);
		assert_eq!(paid, (per_block_fee * 1440f64) as u128);
		assert_eq!(Balances::free_balance(2), 2_000 - fee);
		assert_eq!(Balances::balance_on_hold(&HoldReason::BondFee.into(), &2), fee - paid);
		assert_eq!(Balances::free_balance(1), 500_000 + paid);

		// if we cancel the bond, the prepaid won't be returned
		assert_ok!(Vaults::release_bonded_funds(
			&Bond {
				vault_id: 1,
				bonded_account_id: 2,
				amount: 500_000,
				prepaid_fee: paid,
				total_fee: fee,
				expiration: BondExpiration::BitcoinBlock(2440),
				bond_type: BondType::Bitcoin,
				utxo_id: Some(1)
			},
			false
		));
		assert_eq!(Balances::free_balance(1), 500_000 + paid);
		assert_eq!(Balances::free_balance(2), 2_000 - paid);
	});
}

#[test]
fn it_can_burn_a_bond() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			FixedU128::zero(),
			FixedU128::zero(),
			100_000,
			100_000,
			FixedU128::zero(),
			keys()
		));
		assert_eq!(Balances::free_balance(1), 800_000);

		set_argons(2, 2_000);
		let (total_fee, paid) =
			Vaults::bond_funds(1, 100_000, BondType::Bitcoin, 2440, &2).expect("bonding failed");

		assert_eq!(total_fee, 0);
		assert_eq!(paid, 0);
		assert_eq!(Balances::free_balance(2), 2_000);

		assert_ok!(Vaults::burn_vault_bitcoin_funds(
			&Bond {
				vault_id: 1,
				bonded_account_id: 2,
				amount: 100_000,
				prepaid_fee: paid,
				total_fee,
				expiration: BondExpiration::BitcoinBlock(2440),
				bond_type: BondType::Bitcoin,
				utxo_id: Some(1)
			},
			100_000
		));

		assert_eq!(Balances::free_balance(1), 800_000);
		assert_eq!(Balances::total_balance(&1), 900_000);
		assert_eq!(Balances::free_balance(2), 2_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bitcoin_argons.bonded, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bitcoin_argons.allocated, 0);
	});
}

#[test]
fn it_can_recoup_reduced_value_bitcoins_from_bond_funds() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(2, 2_000);

		set_argons(1, 200_200);

		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			FixedU128::from_float(0.001),
			FixedU128::from_float(0.001),
			100_000,
			100_000,
			FixedU128::zero(),
			keys()
		));

		assert_eq!(Balances::free_balance(1), 200);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 200_000);

		let (total_fee, paid) = Vaults::bond_funds(1, 100_000, BondType::Bitcoin, 1440 * 365, &2)
			.expect("bonding failed");
		assert_eq!(total_fee, 100);
		assert_eq!(paid, 0);

		assert_eq!(
			Vaults::compensate_lost_bitcoin(
				&Bond {
					vault_id: 1,
					bonded_account_id: 2,
					amount: 100_000,
					prepaid_fee: paid,
					total_fee,
					expiration: BondExpiration::BitcoinBlock(1440),
					bond_type: BondType::Bitcoin,
					utxo_id: Some(1)
				},
				50_000
			)
			.expect("compensation failed"),
			50_000
		);
		assert_eq!(Balances::total_balance(&1), 200 + 150_000);
		// should keep the rest on hold
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 150_000);
		// returns the fee
		assert_eq!(Balances::free_balance(2), 2_000 + 50_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bitcoin_argons.bonded, 50_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bitcoin_argons.allocated, 50_000);
	});
}

#[test]
fn it_can_recoup_increased_value_bitcoins_from_securitizations() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(2, 2_000);

		set_argons(1, 350_200);

		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			FixedU128::from_float(0.001),
			FixedU128::from_float(0.001),
			100_000,
			50_000,
			FixedU128::from_float(2.0),
			keys()
		));

		assert_eq!(Balances::free_balance(1), 200);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 350_000);

		let (total_fee, paid) = Vaults::bond_funds(1, 50_000, BondType::Bitcoin, 1440 * 365, &2)
			.expect("bonding failed");
		assert_eq!(total_fee, 50);
		assert_eq!(paid, 0);
		assert_eq!(Balances::free_balance(2), 2_000 - 50);

		assert_eq!(
			Vaults::compensate_lost_bitcoin(
				&Bond {
					vault_id: 1,
					bonded_account_id: 2,
					amount: 50_000,
					prepaid_fee: paid,
					total_fee,
					expiration: BondExpiration::BitcoinBlock(1440),
					bond_type: BondType::Bitcoin,
					utxo_id: Some(1)
				},
				200_000
			)
			.expect("compensation failed"),
			200_000,
			"gets back out of securitization"
		);
		assert_eq!(Balances::total_balance(&1), 350_200 - 200_000);
		// mining bonds are not at risk
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1),
			350_000 - 200_000
		);
		// returns the fee
		assert_eq!(Balances::free_balance(2), 2_000 + 200_000);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bitcoin_argons.bonded, 0);
		assert_eq!(VaultsById::<Test>::get(1).unwrap().bitcoin_argons.allocated, 0);
	});
}

#[test]
fn it_should_allow_vaults_to_add_public_keys() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		let mut keys = BoundedVec::<BitcoinPubkeyHash, MaxVaultBitcoinPubkeys>::new();
		for i in 0..10 {
			let _ = keys.try_push(BitcoinPubkeyHash([i as u8; 20]));
		}
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			TEN_PCT,
			TEN_PCT,
			100_000,
			100_000,
			FixedU128::zero(),
			keys.clone()
		));
		assert_eq!(
			Vaults::create_utxo_script_pubkey(1, 1, BitcoinPubkeyHash([0u8; 20]), 1, 2)
				.expect("")
				.0,
			keys[9]
		);
		assert_eq!(
			Vaults::create_utxo_script_pubkey(1, 1, BitcoinPubkeyHash([0u8; 20]), 1, 2)
				.expect("")
				.0,
			keys[8]
		);

		assert_err!(
			Vaults::add_bitcoin_pubkey_hashes(RuntimeOrigin::signed(1), 1, keys.clone()),
			Error::<Test>::MaxVaultBitcoinPubkeys
		);
		assert_ok!(Vaults::add_bitcoin_pubkey_hashes(
			RuntimeOrigin::signed(1),
			1,
			bounded_vec!(BitcoinPubkeyHash([11; 20]), BitcoinPubkeyHash([12; 20]))
		),);
		assert_eq!(
			Vaults::create_utxo_script_pubkey(1, 1, BitcoinPubkeyHash([0u8; 20]), 1, 2)
				.expect("")
				.0,
			keys[7]
		);
	});
}

#[test]
fn it_should_allow_multiple_vaults_per_account() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			TEN_PCT,
			TEN_PCT,
			100_000,
			100_000,
			FixedU128::zero(),
			keys()
		));
		assert_eq!(Balances::free_balance(1), 800_000);

		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			TEN_PCT,
			TEN_PCT,
			100_000,
			100_000,
			FixedU128::zero(),
			keys()
		));
		assert_eq!(Balances::free_balance(1), 600_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterVault.into(), &1), 400_000);

		assert_eq!(NextVaultId::<Test>::get(), Some(3u32));
	});
}

#[test]
fn it_can_calculate_apr() {
	new_test_ext().execute_with(|| {
		let percent = FixedU128::from_float(10.0); // 1000%
		assert_eq!(Vaults::calculate_fees(percent, 1000, 1440), 27);
		assert_eq!(Vaults::calculate_fees(percent, 100, 1440 * 365), 1000);
		assert_eq!(Vaults::calculate_fees(percent, 99, 1440 * 365), 990);
		assert_eq!(Vaults::calculate_fees(percent, 365000, 1440 * 365), 3650000);
		assert_eq!(Vaults::calculate_fees(percent, 365000, 1440), 9999);
		// minimum argons for a day that will charge anything
		assert_eq!(Vaults::calculate_fees(percent, 36500, 1200), 999);
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
