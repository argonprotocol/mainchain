use crate::{
	Event, MiningMintPerCohort, MintType,
	mock::*,
	pallet::{BlockMintAction, MintedBitcoinMicrogons, MintedMiningMicrogons, PendingMintUtxos},
};
use argon_primitives::{
	BlockRewardsEventHandler, UtxoLockEvents,
	block_seal::{BlockPayout, BlockRewardType},
};
use frame_support::traits::fungible::Unbalanced;
use pallet_prelude::*;

#[test]
fn it_records_burnt_argons_by_prorata() {
	new_test_ext().execute_with(|| {
		MintedMiningMicrogons::<Test>::set(100);
		MintedBitcoinMicrogons::<Test>::set(100);
		Mint::on_argon_burn(50);
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 100 - 25);
		assert_eq!(MintedMiningMicrogons::<Test>::get(), 100 - 25);

		MintedMiningMicrogons::<Test>::set(200);
		MintedBitcoinMicrogons::<Test>::set(0);
		Mint::on_argon_burn(50);
		assert_eq!(MintedMiningMicrogons::<Test>::get(), 200 - 50);
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 0);

		MintedMiningMicrogons::<Test>::set(0);
		MintedBitcoinMicrogons::<Test>::set(100);
		Mint::on_argon_burn(50);
		assert_eq!(MintedMiningMicrogons::<Test>::get(), 0);

		MintedMiningMicrogons::<Test>::set(33);
		MintedBitcoinMicrogons::<Test>::set(66);
		Mint::on_argon_burn(10);
		assert_eq!(MintedMiningMicrogons::<Test>::get(), 33 - 3);
	});
}

#[test]
fn it_tracks_block_rewards() {
	new_test_ext().execute_with(|| {
		<Mint as BlockRewardsEventHandler<_, _>>::rewards_created(&[
			BlockPayout {
				account_id: 1,
				argons: 100,
				ownership: 100,
				reward_type: BlockRewardType::Miner,
				block_seal_authority: None,
			},
			BlockPayout {
				account_id: 1,
				argons: 1,
				ownership: 1,
				reward_type: BlockRewardType::Voter,
				block_seal_authority: None,
			},
			BlockPayout {
				account_id: 2,
				argons: 5,
				ownership: 5,
				reward_type: BlockRewardType::ProfitShare,
				block_seal_authority: None,
			},
		]);

		assert_eq!(MintedMiningMicrogons::<Test>::get(), 106);
		assert_eq!(BlockMintAction::<Test>::get().1.argon_minted, 106);
	});
}

fn set_cpi(value: f64) {
	ArgonCPI::set(Some(FixedI128::from_float(value)));
	AverageCPI::set(FixedI128::from_float(value));
}

#[test]
fn it_calculates_per_miner_mint() {
	new_test_ext().execute_with(|| {
		ArgonCirculation::set(1000);
		set_cpi(0.0);
		// zero conditions
		assert_eq!(Mint::get_microgons_to_print_per_miner(0), 0);
		assert_eq!(Mint::get_microgons_to_print_per_miner(1), 0);

		set_cpi(1.0);
		assert_eq!(Mint::get_microgons_to_print_per_miner(100), 0);

		set_cpi(0.01);
		assert_eq!(Mint::get_microgons_to_print_per_miner(0), 0);

		// divides cleanly
		set_cpi(-0.01);
		assert_eq!(Mint::get_microgons_to_print_per_miner(1), 10);
		assert_eq!(Mint::get_microgons_to_print_per_miner(2), 5);
		set_cpi(-0.02);
		assert_eq!(Mint::get_microgons_to_print_per_miner(2), 10);

		// handles uneven splits
		set_cpi(-0.01);
		assert_eq!(Mint::get_microgons_to_print_per_miner(3), 3);
	});
}

#[test]
fn it_can_mint() {
	set_cpi(-1.0);

	MinerRewardsAccounts::set(vec![(1, 1)]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		Balances::set_total_issuance(25_000);
		ArgonCirculation::set(25_000);

		// nothing to mint
		MintedMiningMicrogons::<Test>::set(500);
		MintedBitcoinMicrogons::<Test>::set(0);

		IsNewFrameStart::set(Some(2));
		Mint::on_initialize(1);
		Mint::on_finalize(1);
		let mint_amount = 25_000;
		System::assert_last_event(
			Event::MiningMint {
				argon_cpi: FixedI128::from_float(-1.0),
				liquidity: 25_000,
				amount: mint_amount,
				per_miner: mint_amount,
			}
			.into(),
		);

		assert_eq!(MintedMiningMicrogons::<Test>::get(), mint_amount + 500);
		assert_eq!(MiningMintPerCohort::<Test>::get().get(&1), Some(&mint_amount));
		assert_eq!(Balances::total_issuance(), 25_000 + mint_amount);
		assert_eq!(Balances::free_balance(1), mint_amount);
		assert_eq!(BlockMintAction::<Test>::get().1.argon_minted, mint_amount);
	});
}

#[test]
fn it_records_failed_mints() {
	set_cpi(-1.0);

	MinerRewardsAccounts::set(vec![(1, 1)]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let amount = ExistentialDeposit::get() - 1;
		Balances::set_total_issuance(amount);
		ArgonCirculation::set(amount);
		IsNewFrameStart::set(Some(2));

		// nothing to mint
		MintedMiningMicrogons::<Test>::set(0);

		Mint::on_initialize(1);
		Mint::on_finalize(1);
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

		assert_eq!(MintedMiningMicrogons::<Test>::get(), 0);
		assert_eq!(MiningMintPerCohort::<Test>::get().get(&1), None);
		assert_eq!(Balances::total_issuance(), amount);
	});
}

#[test]
fn it_cleans_old_cohorts() {
	set_cpi(-1.0);

	MinerRewardsAccounts::set(vec![(1, 1), (2, 2), (3, 3), (4, 3)]);

	new_test_ext().execute_with(|| {
		MiningMintPerCohort::<Test>::mutate(|a| {
			for i in 1..=10 {
				a.try_insert(i, 100u128).expect("should insert");
			}
		});
		System::set_block_number(1);
		Balances::set_total_issuance(60_000);
		ArgonCirculation::set(60_000);

		// nothing to mint
		MintedMiningMicrogons::<Test>::set(500);
		MintedBitcoinMicrogons::<Test>::set(0);

		IsNewFrameStart::set(Some(2));
		Mint::on_initialize(1);
		Mint::on_finalize(1);
		let mint_amount = 60_000;
		System::assert_last_event(
			Event::MiningMint {
				argon_cpi: FixedI128::from_float(-1.0),
				liquidity: 60_000,
				amount: mint_amount,
				per_miner: mint_amount / 4,
			}
			.into(),
		);

		assert_eq!(MintedMiningMicrogons::<Test>::get(), mint_amount + 500);
		assert_eq!(MiningMintPerCohort::<Test>::get().get(&1), Some(&15100u128));
		assert_eq!(MiningMintPerCohort::<Test>::get().get(&2), Some(&15100u128));
		assert_eq!(MiningMintPerCohort::<Test>::get().get(&3), Some(&30100u128));

		assert_eq!(MiningMintPerCohort::<Test>::get().keys().len(), 10);

		MinerRewardsAccounts::set(vec![(2, 2), (3, 3), (4, 3), (5, 11)]);
		Mint::on_initialize(2);
		Mint::on_finalize(2);
		assert_eq!(MiningMintPerCohort::<Test>::get().keys().len(), 10);
		assert_eq!(MiningMintPerCohort::<Test>::get().get(&1), None);
		assert_eq!(MiningMintPerCohort::<Test>::get().get(&11), Some(&15000u128));
	});
}
#[test]
fn it_doesnt_mint_before_active_miners() {
	ArgonCPI::set(Some(FixedI128::from_float(-1.0)));

	MinerRewardsAccounts::set(vec![]);

	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		Balances::set_total_issuance(1000);
		ArgonCirculation::set(1000);

		// nothing to mint
		MintedMiningMicrogons::<Test>::set(0);
		MintedBitcoinMicrogons::<Test>::set(0);

		Mint::on_initialize(1);

		assert!(System::events().is_empty());
		assert_eq!(BlockMintAction::<Test>::get().1.argon_minted, 0);
	});
}

#[test]
fn it_does_not_mint_bitcoin_with_cpi_gt_zero() {
	new_test_ext().execute_with(|| {
		let utxo_id = 1;
		let account_id = 1;
		let amount = 1000u128;
		assert_ok!(Mint::utxo_locked(utxo_id, &account_id, amount));

		assert_eq!(Balances::total_issuance(), 0u128);

		// nothing to mint
		MintedMiningMicrogons::<Test>::set(1000);
		MintedBitcoinMicrogons::<Test>::set(0);
		ArgonCPI::set(Some(FixedI128::from_float(0.01)));
		IsNewFrameStart::set(Some(2));

		Mint::on_initialize(1);
		Mint::on_finalize(1);
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
		ArgonCirculation::set(0);
		assert_ok!(Mint::utxo_locked(utxo_id, &account_id, amount));
		assert_ok!(Mint::utxo_locked(2, &2, 500));
		assert_eq!(
			PendingMintUtxos::<Test>::get().to_vec(),
			vec![(utxo_id, account_id, amount), (2, 2, 500)]
		);

		assert_eq!(Balances::total_issuance(), 0u128);

		// nothing to mint
		MintedMiningMicrogons::<Test>::set(0);
		MintedBitcoinMicrogons::<Test>::set(0);
		// must have miners to mint
		MinerRewardsAccounts::set(vec![(10, 1)]);

		Mint::on_initialize(1);
		assert_eq!(Balances::total_issuance(), 0u128);
		assert_eq!(
			PendingMintUtxos::<Test>::get().to_vec(),
			vec![(utxo_id, account_id, amount), (2, 2, 500)]
		);
		assert!(System::events().is_empty());

		System::set_block_number(2);
		MintedMiningMicrogons::<Test>::set(100);
		set_cpi(-0.1);

		Mint::on_initialize(2);
		assert_eq!(
			PendingMintUtxos::<Test>::get().to_vec(),
			vec![(utxo_id, account_id, amount - 100), (2, 2, 500)]
		);
		System::assert_last_event(
			Event::BitcoinMint { account_id, utxo_id: Some(utxo_id), amount: 100 }.into(),
		);
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 100);
		assert_eq!(Balances::total_issuance(), 100u128);

		assert_eq!(BlockMintAction::<Test>::get().1.argon_minted, 0);
		assert_eq!(BlockMintAction::<Test>::get().1.bitcoin_minted, 100);
		// now allow whole

		ArgonCirculation::set(100);
		System::set_block_number(3);
		MintedMiningMicrogons::<Test>::set(amount + 100);
		IsNewFrameStart::set(Some(2));
		MinerRewardsAccounts::set(vec![(10, 1)]);
		BlockMintAction::<Test>::kill();
		Mint::on_finalize(3);
		// should print argons to the miner
		// issuance is 100, so will mint 100 * 0.1 = 10
		let miner_amount = 10;
		assert_eq!(MintedMiningMicrogons::<Test>::get(), amount + 100 + miner_amount);
		assert_eq!(Balances::free_balance(10), miner_amount);
		assert_eq!(BlockMintAction::<Test>::get().1.argon_minted, miner_amount);
		assert_eq!(BlockMintAction::<Test>::get().1.bitcoin_minted, 0);

		System::reset_events();
		Mint::on_initialize(4);
		System::assert_has_event(
			Event::BitcoinMint { account_id, utxo_id: Some(utxo_id), amount: amount - 100 }.into(),
		);
		let bitcoin_amount_to_match = 100 + miner_amount;
		System::assert_has_event(
			Event::BitcoinMint { account_id: 2, utxo_id: Some(2), amount: bitcoin_amount_to_match }
				.into(),
		);
		// should equal mined argons
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), amount + bitcoin_amount_to_match);
		assert_eq!(Balances::total_issuance(), amount + bitcoin_amount_to_match + miner_amount);

		assert_eq!(
			BlockMintAction::<Test>::get().1.bitcoin_minted,
			amount - 100 + bitcoin_amount_to_match
		);

		BlockMintAction::<Test>::kill();
		System::set_block_number(4);
		Mint::on_argon_burn(100);
		assert_eq!(BlockMintAction::<Test>::get().1.argon_burned, 100);
		assert_eq!(BlockMintAction::<Test>::get().1.argon_minted, 0);
		assert_eq!(BlockMintAction::<Test>::get().1.bitcoin_minted, 0);
		Mint::on_initialize(4);
		// make sure it doesn't get cleared out
		assert_eq!(BlockMintAction::<Test>::get().1.argon_burned, 100);
	});
}

#[test]
fn it_decrements_unlocked_bitcoins() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		MintedBitcoinMicrogons::<Test>::set(100);

		assert_ok!(Mint::utxo_released(1, true, 50));
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 50);

		PendingMintUtxos::<Test>::try_append((1, 1, 10)).unwrap();

		assert_ok!(Mint::utxo_released(1, false, 10));

		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 40);
		// should still be in line
		assert_eq!(PendingMintUtxos::<Test>::get().to_vec(), vec![(1, 1, 10)]);

		assert_ok!(Mint::utxo_released(1, true, 40));
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 0);
		assert!(PendingMintUtxos::<Test>::get().is_empty());
	});
}
