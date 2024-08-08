use argon_primitives::{block_seal::BlockPayout, BlockRewardsEventHandler, UtxoBondedEvents};
use frame_support::{
	assert_ok,
	traits::{fungible::Unbalanced, OnInitialize},
};
use sp_arithmetic::{FixedI128, FixedU128};
use sp_runtime::{DispatchError, TokenError};

use crate::{
	mock::*,
	pallet::{MintedBitcoinArgons, MintedMiningArgons, PendingMintUtxos},
	Event, MintType,
};

#[test]
fn it_records_burnt_argons_by_prorata() {
	new_test_ext().execute_with(|| {
		MintedMiningArgons::<Test>::set(100);
		MintedBitcoinArgons::<Test>::set(100);
		Mint::on_argon_burn(50);
		assert_eq!(MintedBitcoinArgons::<Test>::get(), 100 - 25);
		assert_eq!(MintedMiningArgons::<Test>::get(), 100 - 25);

		MintedMiningArgons::<Test>::set(200);
		MintedBitcoinArgons::<Test>::set(0);
		Mint::on_argon_burn(50);
		assert_eq!(MintedMiningArgons::<Test>::get(), 200 - 50);
		assert_eq!(MintedBitcoinArgons::<Test>::get(), 0);

		MintedMiningArgons::<Test>::set(0);
		MintedBitcoinArgons::<Test>::set(100);
		Mint::on_argon_burn(50);
		assert_eq!(MintedMiningArgons::<Test>::get(), 0);

		MintedMiningArgons::<Test>::set(33);
		MintedBitcoinArgons::<Test>::set(66);
		Mint::on_argon_burn(10);
		assert_eq!(MintedMiningArgons::<Test>::get(), 33 - 3);
	});
}

#[test]
fn it_tracks_block_rewards() {
	new_test_ext().execute_with(|| {
		<Mint as BlockRewardsEventHandler<_, _>>::rewards_created(&[
			BlockPayout { account_id: 1, argons: 100, shares: 100 },
			BlockPayout { account_id: 1, argons: 1, shares: 1 },
			BlockPayout { account_id: 2, argons: 5, shares: 5 },
		]);

		assert_eq!(MintedMiningArgons::<Test>::get(), 106);
	});
}

#[test]
fn it_calculates_per_miner_mint() {
	new_test_ext().execute_with(|| {
		Balances::set_total_issuance(1000);
		// zero conditions
		assert_eq!(Mint::get_argons_to_print_per_miner(FixedI128::from_float(0.0), 0), 0);
		assert_eq!(Mint::get_argons_to_print_per_miner(FixedI128::from_float(1.0), 100), 0);
		assert_eq!(Mint::get_argons_to_print_per_miner(FixedI128::from_float(0.01), 0), 0);
		assert_eq!(Mint::get_argons_to_print_per_miner(FixedI128::from_float(0.0), 1), 0);

		// divides cleanly
		assert_eq!(Mint::get_argons_to_print_per_miner(FixedI128::from_float(-0.01), 1), 10);
		assert_eq!(Mint::get_argons_to_print_per_miner(FixedI128::from_float(-0.01), 2), 5);
		assert_eq!(Mint::get_argons_to_print_per_miner(FixedI128::from_float(-0.02), 2), 10);

		// handles uneven splits
		assert_eq!(Mint::get_argons_to_print_per_miner(FixedI128::from_float(-0.01), 3), 3);
	});
}

#[test]
fn it_can_mint() {
	ArgonCPI::set(Some(FixedI128::from_float(-1.0)));

	MinerRewardsAccounts::set(vec![(1, None)]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		Balances::set_total_issuance(25_000);

		// nothing to mint
		MintedMiningArgons::<Test>::set(500);
		MintedBitcoinArgons::<Test>::set(0);

		Mint::on_initialize(1);
		System::assert_last_event(
			Event::ArgonsMinted {
				mint_type: MintType::Mining,
				account_id: 1,
				utxo_id: None,
				amount: 25_000,
			}
			.into(),
		);

		assert_eq!(MintedMiningArgons::<Test>::get(), 25_500);
		assert_eq!(Balances::total_issuance(), 25_000 + 25_000);
		assert_eq!(Balances::free_balance(1), 25_000);
	});
}
#[test]
fn it_records_failed_mints() {
	ArgonCPI::set(Some(FixedI128::from_float(-1.0)));

	MinerRewardsAccounts::set(vec![(1, None)]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let amount = ExistentialDeposit::get() - 1;
		Balances::set_total_issuance(amount);

		// nothing to mint
		MintedMiningArgons::<Test>::set(0);

		Mint::on_initialize(1);
		System::assert_last_event(
			Event::MintError {
				mint_type: MintType::Mining,
				account_id: 1,
				utxo_id: None,
				error: DispatchError::Token(TokenError::BelowMinimum),
				amount,
			}
			.into(),
		);

		assert_eq!(MintedMiningArgons::<Test>::get(), 0);
		assert_eq!(Balances::total_issuance(), amount);
	});
}

#[test]
fn it_can_mint_profit_sharing() {
	ArgonCPI::set(Some(FixedI128::from_float(-1.0)));

	MinerRewardsAccounts::set(vec![
		(1, Some(FixedU128::from_rational(30, 100))),
		(2, Some(FixedU128::from_rational(70, 100))),
		(3, None),
	]);

	let per_miner = (1000 / 3) as u128;

	let amount_for_sharer = (per_miner as f64 * 0.3) as u128;
	let amount_for_share_lender = (per_miner as f64 * 0.7) as u128;
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		Balances::set_total_issuance(1000);

		// nothing to mint
		MintedMiningArgons::<Test>::set(0);
		MintedBitcoinArgons::<Test>::set(0);

		Mint::on_initialize(1);
		System::assert_last_event(
			Event::ArgonsMinted {
				mint_type: MintType::Mining,
				account_id: 3,
				utxo_id: None,
				amount: per_miner,
			}
			.into(),
		);
		System::assert_has_event(
			Event::ArgonsMinted {
				mint_type: MintType::Mining,
				account_id: 1,
				utxo_id: None,
				amount: amount_for_sharer,
			}
			.into(),
		);
		System::assert_has_event(
			Event::ArgonsMinted {
				mint_type: MintType::Mining,
				account_id: 2,
				utxo_id: None,
				amount: amount_for_share_lender,
			}
			.into(),
		);
		let amount_minted = per_miner + amount_for_sharer + amount_for_share_lender;
		assert_eq!(MintedMiningArgons::<Test>::get(), amount_minted);
		assert_eq!(Balances::total_issuance(), 1000 + amount_minted);
		assert_eq!(Balances::free_balance(1), amount_for_sharer);
		assert_eq!(Balances::free_balance(2), amount_for_share_lender);
		assert_eq!(Balances::free_balance(3), per_miner);
	});
}

#[test]
fn it_does_not_mint_bitcoin_with_cpi_ge_zero() {
	new_test_ext().execute_with(|| {
		let utxo_id = 1;
		let account_id = 1;
		let amount = 1000u128;
		assert_ok!(Mint::utxo_bonded(utxo_id, &account_id, amount));

		assert_eq!(Balances::total_issuance(), 0u128);

		// nothing to mint
		MintedMiningArgons::<Test>::set(1000);
		MintedBitcoinArgons::<Test>::set(0);
		ArgonCPI::set(Some(FixedI128::from_float(0.0)));

		Mint::on_initialize(1);
		// should not mint
		assert_eq!(PendingMintUtxos::<Test>::get().to_vec(), vec![(utxo_id, account_id, amount)]);
		assert!(System::events().is_empty());

		ArgonCPI::set(Some(FixedI128::from_float(0.1)));

		Mint::on_initialize(2);
		// should not mint
		assert_eq!(PendingMintUtxos::<Test>::get().to_vec(), vec![(utxo_id, account_id, amount)]);
		assert!(System::events().is_empty());
	});
}
#[test]
fn it_pays_bitcoin_mints() {
	new_test_ext().execute_with(|| {
		let utxo_id = 1;
		let account_id = 1;
		let amount = 62_000_000u128;
		assert_ok!(Mint::utxo_bonded(utxo_id, &account_id, amount));
		assert_ok!(Mint::utxo_bonded(2, &2, 500));
		assert_eq!(
			PendingMintUtxos::<Test>::get().to_vec(),
			vec![(utxo_id, account_id, amount), (2, 2, 500)]
		);

		assert_eq!(Balances::total_issuance(), 0u128);

		// nothing to mint
		MintedMiningArgons::<Test>::set(0);
		MintedBitcoinArgons::<Test>::set(0);

		Mint::on_initialize(1);
		assert_eq!(
			PendingMintUtxos::<Test>::get().to_vec(),
			vec![(utxo_id, account_id, amount), (2, 2, 500)]
		);
		assert!(System::events().is_empty());

		System::set_block_number(2);
		MintedMiningArgons::<Test>::set(100);
		ArgonCPI::set(Some(FixedI128::from_float(-0.1)));

		Mint::on_initialize(2);
		assert_eq!(
			PendingMintUtxos::<Test>::get().to_vec(),
			vec![(utxo_id, account_id, amount - 100), (2, 2, 500)]
		);
		System::assert_last_event(
			Event::ArgonsMinted {
				mint_type: MintType::Bitcoin,
				account_id,
				utxo_id: Some(utxo_id),
				amount: 100,
			}
			.into(),
		);
		assert_eq!(MintedBitcoinArgons::<Test>::get(), 100);
		assert_eq!(Balances::total_issuance(), 100u128);

		// now allow whole

		System::set_block_number(2);
		MintedMiningArgons::<Test>::set(62_000_100);
		Mint::on_initialize(2);
		assert_eq!(PendingMintUtxos::<Test>::get().to_vec(), vec![(2, 2, 500 - 100)]);
		System::assert_has_event(
			Event::ArgonsMinted {
				mint_type: MintType::Bitcoin,
				account_id,
				utxo_id: Some(utxo_id),
				amount: 62_000_000 - 100,
			}
			.into(),
		);
		System::assert_has_event(
			Event::ArgonsMinted {
				mint_type: MintType::Bitcoin,
				account_id: 2,
				utxo_id: Some(2),
				amount: 100,
			}
			.into(),
		);
		// should equal mined argons
		assert_eq!(MintedBitcoinArgons::<Test>::get(), 62_000_100);
		assert_eq!(Balances::total_issuance(), 62_000_100u128);
	});
}
