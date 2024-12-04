use frame_support::{
	assert_err, assert_ok,
	pallet_prelude::*,
	traits::fungible::{Inspect, InspectHold, Mutate},
};
use sp_arithmetic::{traits::Zero, FixedI128, FixedPointNumber, FixedU128};
use sp_core::H256;

use crate::{
	mock::*,
	pallet::{
		BitcoinBondCompletions, BondsById, MiningBondCompletions, OwedUtxoAggrieved, UtxosById,
		UtxosCosignReleaseHeightById, UtxosPendingUnlockByUtxoId,
	},
	Error, Event, HoldReason, UtxoCosignRequest, UtxoState,
};
use argon_primitives::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinRejectedReason, BitcoinScriptPubkey, BitcoinSignature,
		CompressedBitcoinPubkey, H256Le, Satoshis, UtxoRef, SATOSHIS_PER_BITCOIN,
	},
	bond::{Bond, BondExpiration, BondProvider, BondType},
	BitcoinUtxoEvents, BondId, PriceProvider,
};

#[test]
fn can_bond_a_bitcoin_utxo() {
	BitcoinBlockHeight::set(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		set_argons(2, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_err!(
			Bonds::bond_bitcoin(RuntimeOrigin::signed(2), 1, 1_000_000, pubkey),
			Error::<Test>::InsufficientSatoshisBonded
		);
		assert_ok!(Bonds::bond_bitcoin(RuntimeOrigin::signed(2), 1, SATOSHIS_PER_BITCOIN, pubkey));
		assert_eq!(UtxosById::<Test>::get(1).unwrap(), default_utxo_state(1, SATOSHIS_PER_BITCOIN));
		assert_eq!(
			BondsById::<Test>::get(1).unwrap().amount,
			StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
				.expect("should have price")
		);
		assert_eq!(WatchedUtxosById::get().len(), 1);
		assert_eq!(
			BitcoinBondCompletions::<Test>::get(
				BitcoinBlockHeight::get() + BitcoinBondDurationBlocks::get()
			)
			.to_vec(),
			vec![1]
		);

		// should expire if nothing happens until bitcoin end
		System::set_block_number(2);
		BitcoinBlockHeight::set(12 + BitcoinBondDurationBlocks::get());
		Bonds::on_initialize(2);
		assert_eq!(UtxosById::<Test>::get(1), None);
		assert_eq!(BondsById::<Test>::get(1), None);
		assert_eq!(WatchedUtxosById::get().len(), 0);
	});
}

#[test]
fn cleans_up_a_rejected_bitcoin() {
	BitcoinBlockHeight::set(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(Bonds::bond_bitcoin(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		let price = StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().bitcoin_argons.bonded, price);

		assert_ok!(Bonds::utxo_rejected(1, BitcoinRejectedReason::LookupExpired));
		assert_eq!(UtxosById::<Test>::get(1), None);
		assert_eq!(BondsById::<Test>::get(1), None);
		assert_eq!(WatchedUtxosById::get().len(), 0);
		assert_eq!(DefaultVault::get().bitcoin_argons.bonded, 0);
	});
}

#[test]
fn marks_a_verified_bitcoin() {
	BitcoinBlockHeight::set(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);

		assert_ok!(Bonds::bond_bitcoin(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		let price = StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().bitcoin_argons.bonded, price);

		assert_ok!(Bonds::utxo_verified(1));
		assert!(UtxosById::<Test>::get(1).unwrap().is_verified);
		assert_eq!(WatchedUtxosById::get().len(), 1);
		assert_eq!(LastBondEvent::get(), Some((1, who, price)));
	});
}

#[test]
fn calculates_redemption_prices() {
	new_test_ext().execute_with(|| {
		BitcoinPricePerUsd::set(Some(FixedU128::saturating_from_integer(50000)));
		ArgonPricePerUsd::set(Some(FixedU128::saturating_from_integer(1)));
		ArgonCPI::set(Some(FixedI128::zero()));
		{
			let new_price = Bonds::get_redemption_price(&100_000_000).expect("should have price");
			assert_eq!(new_price, 50_000_000_000);
		}
		ArgonPricePerUsd::set(Some(FixedU128::from_float(1.01)));
		{
			let new_price = Bonds::get_redemption_price(&100_000_000).expect("should have price");
			assert_eq!(new_price, (50_000_000_000f64 / 1.01f64) as u128);
		}
		ArgonCPI::set(Some(FixedI128::from_float(0.1)));
		{
			let new_price = Bonds::get_redemption_price(&100_000_000).expect("should have price");
			// round to 3 digit precision for multiplier
			let multiplier = 0.713 * 1.01 + 0.274;
			// NOTE: floating point yields different rounding - might need to modify if you change
			// values
			assert_eq!(new_price, (multiplier * (50_000_000_000.0 / 1.01)) as u128);
		}
	});
}

#[test]
fn burns_a_spent_bitcoin() {
	BitcoinBlockHeight::set(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let allocated = DefaultVault::get().bitcoin_argons.allocated;

		assert_ok!(Bonds::bond_bitcoin(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		let expiration_block = BitcoinBlockHeight::get() + BitcoinBondDurationBlocks::get();

		let price = StaticPriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
			.expect("should have price");
		assert_eq!(DefaultVault::get().bitcoin_argons.bonded, price);
		// first verify
		assert_ok!(Bonds::utxo_verified(1));

		BitcoinPricePerUsd::set(Some(FixedU128::saturating_from_integer(50000)));

		let new_price =
			Bonds::get_redemption_price(&SATOSHIS_PER_BITCOIN).expect("should have price");
		// 50_000_000_000 microgons for a bitcoin
		// 50m * 0.987 = 49,350,000
		assert_eq!(new_price, 49_350_000_000);

		assert_ok!(Bonds::utxo_spent(1));
		assert_eq!(UtxosById::<Test>::get(1), None);
		assert_eq!(
			BondsById::<Test>::get(1),
			Some(Bond {
				amount: price - new_price,
				utxo_id: Some(1),
				bond_type: BondType::Bitcoin,
				total_fee: 0,
				prepaid_fee: 0,
				vault_id: 1,
				expiration: BondExpiration::BitcoinBlock(expiration_block),
				bonded_account_id: who,
				start_block: 1,
			})
		);
		assert_eq!(WatchedUtxosById::get().len(), 0);
		assert_eq!(DefaultVault::get().bitcoin_argons.bonded, price - new_price);
		assert_eq!(DefaultVault::get().bitcoin_argons.allocated, allocated - new_price);
		// should still exist in completions
		assert_eq!(BitcoinBondCompletions::<Test>::get(expiration_block).to_vec(), vec![1]);

		System::assert_last_event(
			Event::<Test>::BitcoinBondBurned {
				vault_id: 1,
				bond_id: 1,
				utxo_id: 1,
				amount_burned: new_price,
				amount_held: price - new_price,
				was_utxo_spent: true,
			}
			.into(),
		);
		BitcoinBlockHeight::set(expiration_block);
		Bonds::on_initialize(2);

		assert_eq!(LastUnlockEvent::get(), Some((1, true, new_price)));
		assert!(BitcoinBondCompletions::<Test>::get(expiration_block).is_empty());
		assert_eq!(BondsById::<Test>::get(1), None);
		assert_eq!(DefaultVault::get().bitcoin_argons.bonded, 0);
		assert_eq!(DefaultVault::get().bitcoin_argons.allocated, allocated - new_price);
	});
}

#[test]
fn cancels_an_unverified_spent_bitcoin() {
	BitcoinBlockHeight::set(12);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let who = 1;
		set_argons(who, 2_000);
		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let allocated = DefaultVault::get().bitcoin_argons.allocated;

		assert_ok!(Bonds::bond_bitcoin(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		let bond = BondsById::<Test>::get(1).unwrap();
		let expiration_block = match bond.expiration {
			BondExpiration::BitcoinBlock(block) => block,
			_ => panic!("should be bitcoin block"),
		};
		// spend before verify
		assert_ok!(Bonds::utxo_spent(1));

		assert_eq!(WatchedUtxosById::get().len(), 0);
		assert_eq!(DefaultVault::get().bitcoin_argons.bonded, 0);
		assert_eq!(DefaultVault::get().bitcoin_argons.allocated, allocated);
		assert_eq!(BondsById::<Test>::get(1), None);
		assert!(BitcoinBondCompletions::<Test>::get(expiration_block).is_empty());

		System::assert_last_event(
			Event::<Test>::BondCanceled {
				vault_id: 1,
				bond_id: 1,
				bonded_account_id: who,
				bond_type: BondType::Bitcoin,
				returned_fee: 0,
			}
			.into(),
		);
	});
}

#[test]
fn can_unlock_a_bitcoin() {
	new_test_ext().execute_with(|| {
		BitcoinBlockHeight::set(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		set_argons(who, 2_000);
		assert_ok!(Bonds::bond_bitcoin(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			pubkey
		));
		let bond = BondsById::<Test>::get(1).unwrap();
		let expiration_block = match bond.expiration {
			BondExpiration::BitcoinBlock(block) => block,
			_ => panic!("should be bitcoin block"),
		};
		assert_eq!(DefaultVault::get().bitcoin_argons.bonded, bond.amount);
		// first verify
		assert_ok!(Bonds::utxo_verified(1));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, bond.amount));

		BitcoinPricePerUsd::set(Some(FixedU128::from_float(65_000.00)));
		// now the user goes to unlock
		// 1. We would create a psbt and output address
		let unlock_script_pubkey = make_script_pubkey(&[0; 32]);
		// must be the right user!
		assert_err!(
			Bonds::unlock_bitcoin_bond(
				RuntimeOrigin::signed(2),
				1,
				unlock_script_pubkey.clone(),
				1000
			),
			Error::<Test>::NoPermissions
		);
		// must be before the cutoff
		BitcoinBlockHeight::set(expiration_block - 1);
		assert_err!(
			Bonds::unlock_bitcoin_bond(
				RuntimeOrigin::signed(who),
				1,
				unlock_script_pubkey.clone(),
				1000
			),
			Error::<Test>::BitcoinUnlockInitiationDeadlinePassed
		);
		BitcoinBlockHeight::set(expiration_block - UtxoUnlockCosignDeadlineBlocks::get() - 1);
		assert_ok!(Bonds::unlock_bitcoin_bond(
			RuntimeOrigin::signed(who),
			1,
			unlock_script_pubkey.clone(),
			1000
		));
		assert!(UtxosById::<Test>::get(1).is_some());
		let redemption_price =
			Bonds::get_redemption_price(&SATOSHIS_PER_BITCOIN).expect("should have price");
		assert!(redemption_price > bond.amount);
		// redemption price should be the bond price since current redemption price is above
		assert_eq!(
			UtxosPendingUnlockByUtxoId::<Test>::get().get(&1),
			Some(UtxoCosignRequest {
				vault_id: 1,
				bond_id: 1,
				cosign_due_block: BitcoinBlockHeight::get() + UtxoUnlockCosignDeadlineBlocks::get(),
				redemption_price: bond.amount,
				to_script_pubkey: unlock_script_pubkey,
				bitcoin_network_fee: 1000
			})
			.as_ref()
		);
		assert!(BondsById::<Test>::get(1).is_some());
		System::assert_last_event(
			Event::<Test>::BitcoinUtxoCosignRequested { bond_id: 1, vault_id: 1, utxo_id: 1 }
				.into(),
		);

		assert_eq!(Balances::free_balance(who), 2_000);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::UnlockingBitcoin.into(), &who),
			bond.amount
		);
	});
}

#[test]
fn penalizes_vault_if_not_unlock_countersigned() {
	new_test_ext().execute_with(|| {
		BitcoinBlockHeight::set(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		let satoshis = SATOSHIS_PER_BITCOIN + 5000;
		set_argons(who, 2_000);
		assert_ok!(Bonds::bond_bitcoin(RuntimeOrigin::signed(who), 1, satoshis, pubkey));
		let vault = DefaultVault::get();
		let mut bond = BondsById::<Test>::get(1).unwrap();
		assert_eq!(vault.bitcoin_argons.bonded, bond.amount);
		// first verify
		assert_ok!(Bonds::utxo_verified(1));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, bond.amount));
		let unlock_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(Bonds::unlock_bitcoin_bond(
			RuntimeOrigin::signed(who),
			1,
			unlock_script_pubkey.clone(),
			2000
		));
		assert!(UtxosById::<Test>::get(1).is_some());

		let redemption_price = Bonds::get_redemption_price(&satoshis).expect("should have price");
		let cosign_due = BitcoinBlockHeight::get() + UtxoUnlockCosignDeadlineBlocks::get();
		assert_eq!(
			UtxosPendingUnlockByUtxoId::<Test>::get().get(&1),
			Some(UtxoCosignRequest {
				vault_id: 1,
				bond_id: 1,
				cosign_due_block: cosign_due,
				redemption_price,
				to_script_pubkey: unlock_script_pubkey,
				bitcoin_network_fee: 2000
			})
			.as_ref()
		);

		BitcoinBlockHeight::set(cosign_due);
		System::set_block_number(2);
		Bonds::on_initialize(2);

		// should pay back at market price (not the discounted rate)
		let market_price =
			StaticPriceProvider::get_bitcoin_argon_price(satoshis).expect("should have price");
		assert_eq!(UtxosPendingUnlockByUtxoId::<Test>::get().get(&1), None);
		assert_eq!(UtxosById::<Test>::get(1), None);
		assert_eq!(OwedUtxoAggrieved::<Test>::get(1), None);
		System::assert_last_event(
			Event::<Test>::BitcoinCosignPastDue {
				bond_id: 1,
				vault_id: 1,
				utxo_id: 1,
				compensation_amount: market_price,
				compensation_still_owed: 0,
				compensated_account_id: who,
			}
			.into(),
		);
		let original_bond_amount = bond.amount;
		assert_eq!(LastUnlockEvent::get(), Some((1, false, bond.amount)));
		bond.amount = original_bond_amount - market_price;
		assert_eq!(BondsById::<Test>::get(1), Some(bond));
		assert_eq!(DefaultVault::get().bitcoin_argons.bonded, original_bond_amount - market_price);
		assert_eq!(
			DefaultVault::get().bitcoin_argons.allocated,
			vault.bitcoin_argons.allocated.saturating_sub(market_price)
		);

		assert_eq!(Balances::balance_on_hold(&HoldReason::UnlockingBitcoin.into(), &who), 0);
		assert_eq!(Balances::balance(&who), 2000 + market_price);
	});
}

#[test]
fn clears_unlocked_bitcoin_bonds() {
	new_test_ext().execute_with(|| {
		BitcoinBlockHeight::set(1);
		System::set_block_number(1);

		let secp = bitcoin::secp256k1::Secp256k1::new();
		let rng = &mut rand::thread_rng();
		let keypair = bitcoin::secp256k1::SecretKey::new(rng);
		let pubkey = keypair.public_key(&secp).serialize();
		let who = 2;
		let satoshis = SATOSHIS_PER_BITCOIN + 25000;
		set_argons(who, 2_000);
		assert_ok!(Bonds::bond_bitcoin(RuntimeOrigin::signed(who), 1, satoshis, pubkey.into()));
		let vault = DefaultVault::get();
		let bond = BondsById::<Test>::get(1).unwrap();
		assert_eq!(vault.bitcoin_argons.bonded, bond.amount);
		// first verify
		assert_ok!(Bonds::utxo_verified(1));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, bond.amount));
		let unlock_script_pubkey = make_script_pubkey(&[0; 32]);
		assert_ok!(Bonds::unlock_bitcoin_bond(
			RuntimeOrigin::signed(who),
			1,
			unlock_script_pubkey.clone(),
			11
		));
		assert!(UtxosById::<Test>::get(1).is_some());

		let redemption_price = Bonds::get_redemption_price(&satoshis).expect("should have price");
		let cosign_due_block = BitcoinBlockHeight::get() + UtxoUnlockCosignDeadlineBlocks::get();
		assert_eq!(
			UtxosPendingUnlockByUtxoId::<Test>::get().get(&1),
			Some(UtxoCosignRequest {
				vault_id: 1,
				bond_id: 1,
				cosign_due_block,
				redemption_price,
				to_script_pubkey: unlock_script_pubkey,
				bitcoin_network_fee: 11
			})
			.as_ref()
		);
		assert_err!(
			Bonds::cosign_bitcoin_unlock(
				RuntimeOrigin::signed(2),
				1,
				BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec()))
			),
			Error::<Test>::NoPermissions
		);
		GetUtxoRef::set(Some(UtxoRef { txid: H256Le([0; 32]), output_index: 0 }));

		assert_ok!(Bonds::cosign_bitcoin_unlock(
			RuntimeOrigin::signed(1),
			1,
			BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec()))
		));
		assert_eq!(LastUnlockEvent::get(), Some((1, false, redemption_price)));
		assert_eq!(UtxosPendingUnlockByUtxoId::<Test>::get().get(&1), None);
		assert_eq!(UtxosById::<Test>::get(1), None);
		assert_eq!(OwedUtxoAggrieved::<Test>::get(1), None);
		// should keep bond for the year
		assert_eq!(BondsById::<Test>::get(1), Some(bond.clone()));
		assert_eq!(DefaultVault::get().bitcoin_argons.bonded, bond.amount);
		assert_eq!(DefaultVault::get().bitcoin_argons.allocated, vault.bitcoin_argons.allocated);
		assert_eq!(UtxosCosignReleaseHeightById::<Test>::get(1), Some(1));

		System::assert_last_event(
			Event::<Test>::BitcoinUtxoCosigned {
				bond_id: 1,
				vault_id: 1,
				utxo_id: 1,
				signature: BitcoinSignature(BoundedVec::truncate_from([0u8; 73].to_vec())),
			}
			.into(),
		);

		assert_eq!(Balances::balance_on_hold(&HoldReason::UnlockingBitcoin.into(), &who), 0);
		assert_eq!(Balances::balance(&who), 2000 + bond.amount - redemption_price);
		assert_eq!(Balances::total_issuance(), 2000 + bond.amount - redemption_price);

		// should keep bond for the year
		System::set_block_number(2);
		match bond.expiration {
			BondExpiration::BitcoinBlock(h) => {
				BitcoinBlockHeight::set(h);
			},
			_ => panic!("should be bitcoin block"),
		};
		Bonds::on_initialize(2);
		assert_eq!(BondsById::<Test>::get(1), None);
		assert_eq!(DefaultVault::get().bitcoin_argons.bonded, 0);
		assert_eq!(DefaultVault::get().bitcoin_argons.allocated, vault.bitcoin_argons.allocated);
		assert_ok!(Bonds::utxo_spent(1));
	});
}

#[test]
fn it_should_aggregate_holds_for_a_second_unlock() {
	new_test_ext().execute_with(|| {
		BitcoinBlockHeight::set(1);
		System::set_block_number(1);

		let pubkey = CompressedBitcoinPubkey([1; 33]);
		let who = 1;
		let satoshis = 2 * SATOSHIS_PER_BITCOIN;
		set_argons(who, 2_000);
		assert_ok!(Bonds::bond_bitcoin(RuntimeOrigin::signed(who), 1, satoshis, pubkey));
		assert_ok!(Bonds::bond_bitcoin(RuntimeOrigin::signed(who), 1, satoshis, pubkey));
		let bond = BondsById::<Test>::get(1).unwrap();
		assert_ok!(Bonds::utxo_verified(1));
		assert_ok!(Bonds::utxo_verified(2));
		// Mint the argons into account
		assert_ok!(Balances::mint_into(&who, bond.amount * 2));
		let redemption_price = Bonds::get_redemption_price(&satoshis).expect("should have price");

		assert_ok!(Bonds::unlock_bitcoin_bond(
			RuntimeOrigin::signed(who),
			1,
			make_script_pubkey(&[0; 32]),
			10
		));
		assert_ok!(Bonds::unlock_bitcoin_bond(
			RuntimeOrigin::signed(who),
			2,
			make_script_pubkey(&[0; 32]),
			10
		));
		assert_eq!(Balances::free_balance(who), 2_000 + (2 * (bond.amount - redemption_price)));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::UnlockingBitcoin.into(), &who),
			redemption_price * 2
		);
	});
}

#[test]
fn it_can_create_a_mining_bond() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let who = 1;
		set_argons(who, 2_000_000);
		let amount = 1_000_000;
		let vault = DefaultVault::get();
		assert_ok!(Bonds::bond_mining_slot(1, who, amount, 10));
		assert_eq!(
			BondsById::<Test>::get(1).unwrap(),
			Bond {
				amount,
				utxo_id: None,
				bond_type: BondType::Mining,
				total_fee: 0,
				prepaid_fee: 0,
				vault_id: 1,
				expiration: BondExpiration::ArgonBlock(10),
				bonded_account_id: who,
				start_block: 1,
			}
		);
		assert_eq!(DefaultVault::get().mining_argons.bonded, amount);
		assert_eq!(DefaultVault::get().mining_argons.allocated, vault.mining_argons.allocated);
		assert_eq!(MiningBondCompletions::<Test>::get(10).to_vec(), vec![1]);
		System::assert_last_event(
			Event::<Test>::BondCreated {
				vault_id: 1,
				bond_id: 1,
				amount,
				bond_type: BondType::Mining,
				expiration: BondExpiration::ArgonBlock(10),
				bonded_account_id: who,
				utxo_id: None,
			}
			.into(),
		);

		// expire it
		System::set_block_number(10);
		Bonds::on_initialize(10);
		assert_eq!(BondsById::<Test>::get(1), None);
		assert_eq!(DefaultVault::get().mining_argons.bonded, 0);
	});
}

fn default_utxo_state(bond_id: BondId, satoshis: Satoshis) -> UtxoState {
	let current_height = BitcoinBlockHeight::get();
	UtxoState {
		bond_id,
		satoshis,
		vault_claim_height: current_height + BitcoinBondDurationBlocks::get(),
		open_claim_height: current_height +
			BitcoinBondDurationBlocks::get() +
			BitcoinBondReclamationBlocks::get(),
		is_verified: false,
		utxo_script_pubkey: make_cosign_pubkey([0; 32]),
		owner_pubkey: CompressedBitcoinPubkey([1; 33]),
		vault_claim_pubkey: DefaultVaultReclaimBitcoinPubkey::get().into(),
		vault_pubkey: DefaultVaultBitcoinPubkey::get().into(),
		created_at_height: current_height,
		vault_xpub_sources: ([0; 4], 0, 1),
	}
}

fn make_cosign_pubkey(hash: [u8; 32]) -> BitcoinCosignScriptPubkey {
	BitcoinCosignScriptPubkey::P2WSH { wscript_hash: H256::from(hash) }
}

fn make_script_pubkey(vec: &[u8]) -> BitcoinScriptPubkey {
	BitcoinScriptPubkey(BoundedVec::try_from(vec.to_vec()).unwrap())
}
