use frame_support::{
	assert_ok,
	traits::{fungible::Unbalanced, OnInitialize},
};
use sp_arithmetic::FixedI128;
use ulx_primitives::{block_seal::BlockPayout, BlockRewardsEventHandler, UtxoBondedEvents};

use crate::{
	mock::*,
	pallet::{MintedBitcoinArgons, MintedUlixeeArgons, PendingMintUtxos},
	Event, MintType,
};

#[test]
fn it_records_burnt_argons_by_prorata() {
	new_test_ext().execute_with(|| {
		MintedUlixeeArgons::<Test>::set(100);
		MintedBitcoinArgons::<Test>::set(100);
		UlixeeMint::on_argon_burn(50);
		assert_eq!(MintedBitcoinArgons::<Test>::get(), 100 - 25);
		assert_eq!(MintedUlixeeArgons::<Test>::get(), 100 - 25);

		MintedUlixeeArgons::<Test>::set(200);
		MintedBitcoinArgons::<Test>::set(0);
		UlixeeMint::on_argon_burn(50);
		assert_eq!(MintedUlixeeArgons::<Test>::get(), 200 - 50);
		assert_eq!(MintedBitcoinArgons::<Test>::get(), 0);

		MintedUlixeeArgons::<Test>::set(0);
		MintedBitcoinArgons::<Test>::set(100);
		UlixeeMint::on_argon_burn(50);
		assert_eq!(MintedUlixeeArgons::<Test>::get(), 0);

		MintedUlixeeArgons::<Test>::set(33);
		MintedBitcoinArgons::<Test>::set(66);
		UlixeeMint::on_argon_burn(10);
		assert_eq!(MintedUlixeeArgons::<Test>::get(), 33 - 3);
	});
}

#[test]
fn it_tracks_block_rewards() {
	new_test_ext().execute_with(|| {
		<UlixeeMint as BlockRewardsEventHandler<_, _>>::rewards_created(&[
			BlockPayout { account_id: 1, argons: 100, ulixees: 100 },
			BlockPayout { account_id: 1, argons: 1, ulixees: 1 },
			BlockPayout { account_id: 2, argons: 5, ulixees: 5 },
		]);

		assert_eq!(MintedUlixeeArgons::<Test>::get(), 106);
	});
}

#[test]
fn it_calculates_per_miner_mint() {
	new_test_ext().execute_with(|| {
		Balances::set_total_issuance(1000);
		// zero conditions
		assert_eq!(UlixeeMint::get_argons_to_print_per_miner(FixedI128::from_float(0.0), 0), 0);
		assert_eq!(UlixeeMint::get_argons_to_print_per_miner(FixedI128::from_float(-1.0), 100), 0);
		assert_eq!(UlixeeMint::get_argons_to_print_per_miner(FixedI128::from_float(0.01), 0), 0);
		assert_eq!(UlixeeMint::get_argons_to_print_per_miner(FixedI128::from_float(0.0), 1), 0);

		// divides cleanly
		assert_eq!(UlixeeMint::get_argons_to_print_per_miner(FixedI128::from_float(0.01), 1), 10);
		assert_eq!(UlixeeMint::get_argons_to_print_per_miner(FixedI128::from_float(0.01), 2), 5);
		assert_eq!(UlixeeMint::get_argons_to_print_per_miner(FixedI128::from_float(0.02), 2), 10);

		// handles uneven splits
		assert_eq!(UlixeeMint::get_argons_to_print_per_miner(FixedI128::from_float(0.01), 3), 3);
	});
}

#[test]
fn it_does_not_mint_bitcoin_with_cpi_ge_zero() {
	new_test_ext().execute_with(|| {
		let utxo_id = 1;
		let account_id = 1;
		let amount = 1000u128;
		assert_ok!(UlixeeMint::utxo_bonded(utxo_id, &account_id, amount));

		assert_eq!(Balances::total_issuance(), 0u128);

		// nothing to mint
		MintedUlixeeArgons::<Test>::set(1000);
		MintedBitcoinArgons::<Test>::set(0);
		ArgonCPI::set(Some(FixedI128::from_float(0.0)));

		UlixeeMint::on_initialize(1);
		// should not mint
		assert_eq!(PendingMintUtxos::<Test>::get().to_vec(), vec![(utxo_id, account_id, amount)]);
		assert!(System::events().is_empty());

		ArgonCPI::set(Some(FixedI128::from_float(0.1)));

		UlixeeMint::on_initialize(2);
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
		assert_ok!(UlixeeMint::utxo_bonded(utxo_id, &account_id, amount));
		assert_ok!(UlixeeMint::utxo_bonded(2, &2, 500));
		assert_eq!(
			PendingMintUtxos::<Test>::get().to_vec(),
			vec![(utxo_id, account_id, amount), (2, 2, 500)]
		);

		assert_eq!(Balances::total_issuance(), 0u128);

		// nothing to mint
		MintedUlixeeArgons::<Test>::set(0);
		MintedBitcoinArgons::<Test>::set(0);

		UlixeeMint::on_initialize(1);
		assert_eq!(
			PendingMintUtxos::<Test>::get().to_vec(),
			vec![(utxo_id, account_id, amount), (2, 2, 500)]
		);
		assert!(System::events().is_empty());

		System::set_block_number(2);
		MintedUlixeeArgons::<Test>::set(100);
		ArgonCPI::set(Some(FixedI128::from_float(-0.1)));

		UlixeeMint::on_initialize(2);
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
		MintedUlixeeArgons::<Test>::set(62_000_100);
		UlixeeMint::on_initialize(2);
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
		// should equal ulixee argons
		assert_eq!(MintedBitcoinArgons::<Test>::get(), 62_000_100);
		assert_eq!(Balances::total_issuance(), 62_000_100u128);
	});
}
