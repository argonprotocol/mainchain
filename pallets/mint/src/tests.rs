use crate::{
	mock::*,
	pallet::{
		BlockMintAction, MintIndex, MintedBitcoinMicrogons, MintedMiningMicrogons,
		NextPendingBitcoinMintIndex, PendingBitcoinMint, PendingBitcoinMintsByIndex,
		PendingMintIndicesByLockId, PendingMintQueueState,
	},
	Error, Event, MiningMintPerCohort, MintType,
};
use argon_primitives::{
	block_seal::{BlockPayout, BlockRewardType},
	BitcoinFissionMinting, BlockRewardsEventHandler,
};
use frame_support::traits::fungible::Unbalanced;
use pallet_prelude::*;

fn pending_mints() -> Vec<(MintIndex, PendingBitcoinMint<Test>)> {
	let mut pending = PendingBitcoinMintsByIndex::<Test>::iter().collect::<Vec<_>>();
	pending.sort_by_key(|(queue_index, _)| *queue_index);
	pending
}

#[test]
fn it_queues_fission_entitlement() {
	new_test_ext().execute_with(|| {
		assert_ok!(Mint::request_mint(&1, 0, 9, 100));

		assert_eq!(
			pending_mints(),
			vec![(
				0,
				PendingBitcoinMint {
					fission_id: 0,
					lock_id: 9,
					account_id: 1,
					remaining_amount: 100,
					max_amount_per_frame: 10,
				},
			)]
		);
		assert_eq!(PendingMintIndicesByLockId::<Test>::get(9).to_vec(), vec![0]);
	});
}

#[test]
fn it_reopens_capacity_without_removing_pending_entitlements_when_a_fission_closes() {
	new_test_ext().execute_with(|| {
		assert_ok!(Mint::request_mint(&1, 0, 9, 100));
		assert_ok!(Mint::request_mint(&2, 0, 9, 80));
		assert_ok!(Mint::request_mint(&1, 1, 9, 60));
		MintedBitcoinMicrogons::<Test>::set(100);

		Mint::record_mint_repayment(40);

		assert_eq!(PendingMintIndicesByLockId::<Test>::get(9).to_vec(), vec![0, 1, 2]);
		assert_eq!(
			pending_mints(),
			vec![
				(
					0,
					PendingBitcoinMint {
						fission_id: 0,
						lock_id: 9,
						account_id: 1,
						remaining_amount: 100,
						max_amount_per_frame: 10,
					},
				),
				(
					1,
					PendingBitcoinMint {
						fission_id: 0,
						lock_id: 9,
						account_id: 2,
						remaining_amount: 80,
						max_amount_per_frame: 8,
					},
				),
				(
					2,
					PendingBitcoinMint {
						fission_id: 1,
						lock_id: 9,
						account_id: 1,
						remaining_amount: 60,
						max_amount_per_frame: 6,
					},
				),
			]
		);
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 60);

		assert_ok!(Balances::mint_into(&2, ExistentialDeposit::get()));
		MintedMiningMicrogons::<Test>::set(100);
		MinerRewardsAccounts::set(vec![(10, 1)]);
		CurrentFrameId::set(1);
		set_cpi(-0.1);
		Mint::on_initialize(1);

		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 84);
		assert_eq!(Balances::free_balance(1), 16);
		assert_eq!(Balances::free_balance(2), ExistentialDeposit::get() + 8);
	});
}

#[test]
fn fission_up_ratchet_appends_only_the_additional_mint_entitlement() {
	new_test_ext().execute_with(|| {
		assert_ok!(Mint::request_mint(&1, 0, 9, 100));
		MintedBitcoinMicrogons::<Test>::set(40);

		assert_ok!(Mint::request_mint(&1, 0, 9, 25));

		assert_eq!(PendingMintIndicesByLockId::<Test>::get(9).to_vec(), vec![0, 1]);
		assert_eq!(pending_mints()[0].1.remaining_amount, 100);
		assert_eq!(pending_mints()[1].1.remaining_amount, 25);
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 40);
	});
}

#[test]
fn fission_down_ratchet_reopens_global_capacity_and_appends_the_replacement_entitlement() {
	new_test_ext().execute_with(|| {
		assert_ok!(Mint::request_mint(&1, 0, 9, 100));
		MintedBitcoinMicrogons::<Test>::set(100);

		assert_ok!(Mint::request_mint(&1, 0, 9, 60));
		Mint::record_mint_repayment(60);

		assert_eq!(PendingMintIndicesByLockId::<Test>::get(9).to_vec(), vec![0, 1]);
		assert_eq!(pending_mints()[0].1.remaining_amount, 100);
		assert_eq!(pending_mints()[1].1.remaining_amount, 60);
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 40);
	});
}

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

		// Burns larger than the tracked mint totals should saturate both buckets to zero.
		MintedMiningMicrogons::<Test>::set(5);
		MintedBitcoinMicrogons::<Test>::set(5);
		Mint::on_argon_burn(100);
		assert_eq!(MintedMiningMicrogons::<Test>::get(), 0);
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 0);
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
		// Zero CPI or zero miners should never mint.
		assert_eq!(Mint::get_microgons_to_print_per_miner(0), 0);
		assert_eq!(Mint::get_microgons_to_print_per_miner(1), 0);

		set_cpi(1.0);
		assert_eq!(Mint::get_microgons_to_print_per_miner(100), 0);

		set_cpi(0.01);
		assert_eq!(Mint::get_microgons_to_print_per_miner(0), 0);

		// Negative CPI mints should divide cleanly when liquidity splits evenly.
		set_cpi(-0.01);
		assert_eq!(Mint::get_microgons_to_print_per_miner(1), 10);
		assert_eq!(Mint::get_microgons_to_print_per_miner(2), 5);
		set_cpi(-0.02);
		assert_eq!(Mint::get_microgons_to_print_per_miner(2), 10);

		// Integer division should floor uneven splits.
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

		MintedMiningMicrogons::<Test>::set(0);

		Mint::on_initialize(1);
		Mint::on_finalize(1);
		System::assert_last_event(
			Event::MintError {
				mint_type: MintType::Mining,
				account_id: 1,
				fission_id: None,
				lock_id: None,
				error: DispatchError::Token(TokenError::BelowMinimum),
				amount,
			}
			.into(),
		);

		assert_eq!(MintedMiningMicrogons::<Test>::get(), 0);
		assert_eq!(MiningMintPerCohort::<Test>::get().get(&1), None);
		assert_eq!(Balances::total_issuance(), amount);

		System::reset_events();
		assert_ok!(Mint::request_mint(&1, 10, 9, 1));
		MintedMiningMicrogons::<Test>::set(1);
		CurrentFrameId::set(1);
		set_cpi(-0.1);

		Mint::on_initialize(2);

		System::assert_last_event(
			Event::MintError {
				mint_type: MintType::Bitcoin,
				account_id: 1,
				fission_id: Some(10),
				lock_id: Some(9),
				amount: 1,
				error: DispatchError::Token(TokenError::BelowMinimum),
			}
			.into(),
		);
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
		let lock_id = 1;
		let account_id = 1;
		let amount = 1000u128;
		assert_ok!(Mint::request_mint(&account_id, 0, lock_id, amount));

		assert_eq!(Balances::total_issuance(), 0u128);
		assert_eq!(
			pending_mints(),
			vec![(
				0,
				PendingBitcoinMint {
					fission_id: 0,
					lock_id,
					account_id,
					remaining_amount: amount,
					max_amount_per_frame: 100,
				},
			)]
		);
		let queue_cursor = PendingMintQueueState::<Test>::get();
		assert_eq!(NextPendingBitcoinMintIndex::<Test>::get(), 1);
		assert_eq!(queue_cursor.payout_start_index, 0);
		assert_eq!(queue_cursor.payout_cursor_index, 0);

		MintedMiningMicrogons::<Test>::set(1000);
		MintedBitcoinMicrogons::<Test>::set(0);
		ArgonCPI::set(Some(FixedI128::from_float(0.01)));
		IsNewFrameStart::set(Some(2));

		Mint::on_initialize(1);
		Mint::on_finalize(1);
		assert_eq!(
			pending_mints(),
			vec![(
				0,
				PendingBitcoinMint {
					fission_id: 0,
					lock_id,
					account_id,
					remaining_amount: amount,
					max_amount_per_frame: 100,
				},
			)]
		);
		assert!(System::events().is_empty());

		ArgonCPI::set(Some(FixedI128::from_float(0.1)));

		Mint::on_initialize(2);
		assert_eq!(
			pending_mints(),
			vec![(
				0,
				PendingBitcoinMint {
					fission_id: 0,
					lock_id,
					account_id,
					remaining_amount: amount,
					max_amount_per_frame: 100,
				},
			)]
		);
		assert!(System::events().is_empty());
	});
}

#[test]
fn it_pays_bitcoin_mints() {
	new_test_ext().execute_with(|| {
		let lock_id = 1;
		let account_id = 1;
		let amount = 62_000_000u128;
		ArgonCirculation::set(0);
		assert_ok!(Mint::request_mint(&account_id, 0, lock_id, amount));
		assert_ok!(Mint::request_mint(&2, 0, 2, 500));
		assert_eq!(
			pending_mints(),
			vec![
				(
					0,
					PendingBitcoinMint {
						fission_id: 0,
						lock_id,
						account_id,
						remaining_amount: amount,
						max_amount_per_frame: 6_200_000,
					},
				),
				(
					1,
					PendingBitcoinMint {
						fission_id: 0,
						lock_id: 2,
						account_id: 2,
						remaining_amount: 500,
						max_amount_per_frame: 50,
					},
				),
			]
		);
		let queue_cursor = PendingMintQueueState::<Test>::get();
		assert_eq!(queue_cursor.payout_start_index, 0);
		assert_eq!(queue_cursor.payout_cursor_index, 0);

		assert_eq!(Balances::total_issuance(), 0u128);

		MintedMiningMicrogons::<Test>::set(0);
		MintedBitcoinMicrogons::<Test>::set(0);
		// Bitcoin mint payouts still require an eligible miner set for the frame.
		MinerRewardsAccounts::set(vec![(10, 1)]);

		Mint::on_initialize(1);
		assert_eq!(Balances::total_issuance(), 0u128);
		assert_eq!(
			pending_mints(),
			vec![
				(
					0,
					PendingBitcoinMint {
						fission_id: 0,
						lock_id,
						account_id,
						remaining_amount: amount,
						max_amount_per_frame: 6_200_000,
					},
				),
				(
					1,
					PendingBitcoinMint {
						fission_id: 0,
						lock_id: 2,
						account_id: 2,
						remaining_amount: 500,
						max_amount_per_frame: 50,
					},
				),
			]
		);
		assert!(System::events().is_empty());

		System::set_block_number(2);
		CurrentFrameId::set(1);
		MintedMiningMicrogons::<Test>::set(amount);
		set_cpi(-0.1);

		Mint::on_initialize(2);
		assert_eq!(
			pending_mints(),
			vec![
				(
					0,
					PendingBitcoinMint {
						fission_id: 0,
						lock_id,
						account_id,
						remaining_amount: amount - 6_200_000,
						max_amount_per_frame: 6_200_000,
					},
				),
				(
					1,
					PendingBitcoinMint {
						fission_id: 0,
						lock_id: 2,
						account_id: 2,
						remaining_amount: 450,
						max_amount_per_frame: 50,
					},
				),
			]
		);
		let queue_cursor = PendingMintQueueState::<Test>::get();
		assert_eq!(queue_cursor.payout_start_index, 0);
		assert_eq!(queue_cursor.payout_cursor_index, 2);
		assert_eq!(queue_cursor.payout_cursor_frame_id, Some(1));
		System::assert_has_event(
			Event::BitcoinMint {
				account_id,
				fission_id: 0,
				lock_id: Some(lock_id),
				amount: 6_200_000,
			}
			.into(),
		);
		System::assert_last_event(
			Event::BitcoinMint { account_id: 2, fission_id: 0, lock_id: Some(2), amount: 50 }
				.into(),
		);
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 6_200_050);
		assert_eq!(Balances::total_issuance(), 6_200_050u128);

		assert_eq!(BlockMintAction::<Test>::get().1.argon_minted, 0);
		assert_eq!(BlockMintAction::<Test>::get().1.bitcoin_minted, 6_200_050);

		System::set_block_number(3);
		Mint::on_initialize(3);
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 6_200_050);
		assert_eq!(Balances::total_issuance(), 6_200_050u128);

		ArgonCirculation::set(100);
		MintedMiningMicrogons::<Test>::set(amount);
		IsNewFrameStart::set(Some(2));
		CurrentFrameId::set(2);
		MinerRewardsAccounts::set(vec![(10, 1)]);
		BlockMintAction::<Test>::kill();
		Mint::on_finalize(3);
		let miner_amount = 10;
		assert_eq!(MintedMiningMicrogons::<Test>::get(), amount + miner_amount);
		assert_eq!(Balances::free_balance(10), miner_amount);
		assert_eq!(BlockMintAction::<Test>::get().1.argon_minted, miner_amount);
		assert_eq!(BlockMintAction::<Test>::get().1.bitcoin_minted, 0);

		System::reset_events();
		IsNewFrameStart::set(None);
		Mint::on_initialize(4);
		let queue_cursor = PendingMintQueueState::<Test>::get();
		assert_eq!(queue_cursor.payout_start_index, 0);
		assert_eq!(queue_cursor.payout_cursor_index, 2);
		assert_eq!(queue_cursor.payout_cursor_frame_id, Some(2));
		System::assert_has_event(
			Event::BitcoinMint {
				account_id,
				fission_id: 0,
				lock_id: Some(lock_id),
				amount: 6_200_000,
			}
			.into(),
		);
		System::assert_has_event(
			Event::BitcoinMint { account_id: 2, fission_id: 0, lock_id: Some(2), amount: 50 }
				.into(),
		);
		assert_eq!(MintedBitcoinMicrogons::<Test>::get(), 12_400_100);
		assert_eq!(Balances::total_issuance(), 12_400_110);

		assert_eq!(BlockMintAction::<Test>::get().1.bitcoin_minted, 6_200_050);

		BlockMintAction::<Test>::kill();
		System::set_block_number(4);
		Mint::on_argon_burn(100);
		assert_eq!(BlockMintAction::<Test>::get().1.argon_burned, 100);
		assert_eq!(BlockMintAction::<Test>::get().1.argon_minted, 0);
		assert_eq!(BlockMintAction::<Test>::get().1.bitcoin_minted, 0);
		Mint::on_initialize(4);
		// Initializing the same block should not clear previously tracked burns.
		assert_eq!(BlockMintAction::<Test>::get().1.argon_burned, 100);
	});
}

#[test]
fn it_keeps_later_pending_mints_behind_the_frame_payout_window() {
	new_test_ext().execute_with(|| {
		assert_ok!(Mint::request_mint(&1, 0, 1, 100));
		assert_ok!(Mint::request_mint(&2, 0, 2, 100));
		assert_ok!(Mint::request_mint(&3, 0, 3, 100));
		MaxPendingMintPayoutWindowSize::set(2);

		MintedMiningMicrogons::<Test>::set(30);
		CurrentFrameId::set(1);
		set_cpi(-0.1);
		Mint::on_initialize(1);

		assert_eq!(Balances::free_balance(1), 10);
		assert_eq!(Balances::free_balance(2), 10);
		assert_eq!(Balances::free_balance(3), 0);
		let queue_cursor = PendingMintQueueState::<Test>::get();
		assert_eq!(queue_cursor.payout_start_index, 0);
		assert_eq!(queue_cursor.payout_cursor_index, 2);

		CurrentFrameId::set(2);
		MintedMiningMicrogons::<Test>::set(60);
		Mint::on_initialize(2);

		assert_eq!(Balances::free_balance(1), 20);
		assert_eq!(Balances::free_balance(2), 20);
		assert_eq!(Balances::free_balance(3), 0);
		assert_eq!(PendingMintQueueState::<Test>::get().payout_start_index, 0);
		assert_eq!(PendingBitcoinMintsByIndex::<Test>::get(2).unwrap().remaining_amount, 100);
		MaxPendingMintPayoutWindowSize::set(1_000);
	});
}

#[test]
fn it_does_not_partially_pay_or_advance_when_block_cannot_cover_frame_chunk() {
	new_test_ext().execute_with(|| {
		assert_ok!(Mint::request_mint(&1, 0, 1, 100));
		assert_ok!(Mint::request_mint(&2, 0, 2, 100));
		MaxPendingMintPayoutWindowSize::set(2);

		MintedMiningMicrogons::<Test>::set(5);
		CurrentFrameId::set(1);
		set_cpi(-0.1);
		Mint::on_initialize(1);

		assert_eq!(Balances::free_balance(1), 0);
		assert_eq!(Balances::free_balance(2), 0);
		assert_eq!(
			pending_mints(),
			vec![
				(
					0,
					PendingBitcoinMint {
						fission_id: 0,
						lock_id: 1,
						account_id: 1,
						remaining_amount: 100,
						max_amount_per_frame: 10,
					},
				),
				(
					1,
					PendingBitcoinMint {
						fission_id: 0,
						lock_id: 2,
						account_id: 2,
						remaining_amount: 100,
						max_amount_per_frame: 10,
					},
				),
			]
		);
		let queue_cursor = PendingMintQueueState::<Test>::get();
		assert_eq!(queue_cursor.payout_start_index, 0);
		assert_eq!(queue_cursor.payout_cursor_index, 0);
		assert_eq!(queue_cursor.payout_cursor_frame_id, Some(1));

		MintedMiningMicrogons::<Test>::set(10);
		Mint::on_initialize(2);

		assert_eq!(Balances::free_balance(1), 10);
		assert_eq!(Balances::free_balance(2), 0);
		let queue_cursor = PendingMintQueueState::<Test>::get();
		assert_eq!(queue_cursor.payout_start_index, 0);
		assert_eq!(queue_cursor.payout_cursor_index, 1);
		MaxPendingMintPayoutWindowSize::set(1_000);
	});
}

#[test]
fn it_does_not_backfill_the_frame_payout_window() {
	new_test_ext().execute_with(|| {
		assert_ok!(Balances::mint_into(&1, ExistentialDeposit::get()));
		assert_ok!(Mint::request_mint(&1, 0, 1, 1));
		assert_ok!(Mint::request_mint(&2, 0, 2, 100));
		assert_ok!(Mint::request_mint(&3, 0, 3, 100));
		MaxPendingMintPayoutWindowSize::set(2);

		MintedMiningMicrogons::<Test>::set(21);
		CurrentFrameId::set(1);
		set_cpi(-0.1);
		Mint::on_initialize(1);

		assert_eq!(Balances::free_balance(1), ExistentialDeposit::get() + 1);
		assert_eq!(Balances::free_balance(2), 10);
		assert_eq!(Balances::free_balance(3), 0);

		let queue_cursor = PendingMintQueueState::<Test>::get();
		assert_eq!(queue_cursor.payout_start_index, 1);
		assert_eq!(queue_cursor.payout_cursor_index, 2);
		assert_eq!(PendingBitcoinMintsByIndex::<Test>::get(2).unwrap().remaining_amount, 100);
		MaxPendingMintPayoutWindowSize::set(1_000);
	});
}

#[test]
fn it_tracks_multiple_pending_mints_for_the_same_utxo() {
	new_test_ext().execute_with(|| {
		assert_ok!(Mint::request_mint(&1, 10, 1, 100));
		assert_ok!(Mint::request_mint(&1, 11, 1, 50));

		assert_eq!(PendingMintIndicesByLockId::<Test>::get(1).to_vec(), vec![0, 1]);
		assert_eq!(
			pending_mints(),
			vec![
				(
					0,
					PendingBitcoinMint {
						fission_id: 10,
						lock_id: 1,
						account_id: 1,
						remaining_amount: 100,
						max_amount_per_frame: 10,
					},
				),
				(
					1,
					PendingBitcoinMint {
						fission_id: 11,
						lock_id: 1,
						account_id: 1,
						remaining_amount: 50,
						max_amount_per_frame: 5,
					},
				),
			]
		);

		System::set_block_number(1);
		MintedMiningMicrogons::<Test>::set(20);
		MinerRewardsAccounts::set(vec![(10, 1)]);
		CurrentFrameId::set(1);
		set_cpi(-0.1);

		Mint::on_initialize(1);

		System::assert_has_event(
			Event::BitcoinMint { account_id: 1, fission_id: 10, lock_id: Some(1), amount: 10 }
				.into(),
		);
		System::assert_last_event(
			Event::BitcoinMint { account_id: 1, fission_id: 11, lock_id: Some(1), amount: 5 }
				.into(),
		);
	});
}

#[test]
fn it_advances_payout_start_when_loop_reaches_a_missing_front_entry() {
	new_test_ext().execute_with(|| {
		assert_ok!(Mint::request_mint(&1, 0, 1, 100));
		assert_ok!(Mint::request_mint(&2, 0, 2, 100));
		assert_ok!(Mint::request_mint(&3, 0, 3, 100));
		MaxPendingMintPayoutWindowSize::set(2);

		PendingBitcoinMintsByIndex::<Test>::remove(0);
		PendingMintIndicesByLockId::<Test>::remove(1);
		let queue_cursor = PendingMintQueueState::<Test>::get();
		assert_eq!(queue_cursor.payout_start_index, 0);
		assert_eq!(queue_cursor.payout_cursor_index, 0);

		MintedMiningMicrogons::<Test>::set(20);
		CurrentFrameId::set(1);
		set_cpi(-0.1);
		Mint::on_initialize(1);

		assert_eq!(Balances::free_balance(1), 0);
		assert_eq!(Balances::free_balance(2), 10);
		assert_eq!(Balances::free_balance(3), 0);

		let queue_cursor = PendingMintQueueState::<Test>::get();
		assert_eq!(queue_cursor.payout_start_index, 1);
		assert_eq!(queue_cursor.payout_cursor_index, 2);
		MaxPendingMintPayoutWindowSize::set(1_000);
	});
}

#[test]
fn it_limits_pending_mints_per_utxo() {
	new_test_ext().execute_with(|| {
		MaxPendingMintsPerUtxo::set(2);

		assert_ok!(Mint::request_mint(&1, 0, 1, 100));
		assert_ok!(Mint::request_mint(&1, 0, 1, 50));
		assert_noop!(Mint::request_mint(&1, 0, 1, 25), Error::<Test>::TooManyPendingMints);

		MaxPendingMintsPerUtxo::set(50);
	});
}

#[test]
fn it_ignores_zero_amount_pending_mints() {
	new_test_ext().execute_with(|| {
		assert_ok!(Mint::request_mint(&1, 0, 1, 0));

		assert!(pending_mints().is_empty());
		assert!(PendingMintIndicesByLockId::<Test>::get(1).is_empty());
		assert_eq!(NextPendingBitcoinMintIndex::<Test>::get(), 0);
	});
}
