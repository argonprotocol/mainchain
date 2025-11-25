use crate::{
	Event, RewardAmounts,
	mock::{Balances, BlockRewards, Ownership, *},
	pallet::{ArgonsPerBlock, BlockRewardsByCohort},
};
use argon_primitives::{
	BlockSealAuthorityId, BlockSealerInfo, OnNewSlot,
	block_seal::{BlockPayout, BlockRewardType},
};
use pallet_prelude::*;
use sp_core::ByteArray;

pub(crate) fn test_authority(id: [u8; 32]) -> BlockSealAuthorityId {
	BlockSealAuthorityId::from_slice(&id).unwrap()
}

#[test]
fn it_mints_immediately_available_funds() {
	let id = test_authority([1; 32]);
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: Some(2),
		block_seal_authority: Some(id.clone()),
	});
	NotebooksInBlock::set(vec![(1, 1, 1)]);
	NotebookTick::set(1);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		System::assert_last_event(
			Event::RewardCreated {
				rewards: vec![
					BlockPayout {
						account_id: 1,
						ownership: 3750,
						argons: 3750,
						block_seal_authority: Some(id),
						reward_type: BlockRewardType::Miner,
					},
					BlockPayout {
						account_id: 2,
						ownership: 1250,
						argons: 1250,
						block_seal_authority: None,
						reward_type: BlockRewardType::Voter,
					},
				],
			}
			.into(),
		);

		assert_eq!(
			Balances::reducible_balance(&1, Preservation::Expendable, Fortitude::Force),
			3750
		);
		assert_eq!(
			Ownership::reducible_balance(&1, Preservation::Expendable, Fortitude::Force),
			3750
		);

		assert_eq!(
			Balances::reducible_balance(&2, Preservation::Expendable, Fortitude::Force),
			1250
		);
		assert_eq!(
			Ownership::reducible_balance(&2, Preservation::Expendable, Fortitude::Force),
			1250
		);

		// test that we can transfer regular funds still
		let _ = Balances::mint_into(&1, 3000);
		assert_ok!(<Balances as Mutate<TestAccountId>>::transfer(
			&1,
			&2,
			3000,
			Preservation::Expendable
		),);
	});
}

#[test]
fn it_should_unlock_rewards() {
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: Some(2),
		block_seal_authority: None,
	});
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		NotebooksInBlock::set(vec![(1, 1, 1)]);
		NotebookTick::set(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		System::assert_last_event(
			Event::RewardCreated {
				rewards: vec![
					BlockPayout {
						account_id: 1,
						ownership: 3750,
						argons: 3750,
						block_seal_authority: None,
						reward_type: BlockRewardType::Miner,
					},
					BlockPayout {
						account_id: 2,
						ownership: 1250,
						argons: 1250,
						block_seal_authority: None,
						reward_type: BlockRewardType::Voter,
					},
				],
			}
			.into(),
		);
		assert_eq!(Balances::balance(&1), 3750);
		assert_eq!(Ownership::balance(&1), 3750);
		assert_eq!(Balances::balance(&2), 1250);
		assert_eq!(Ownership::balance(&2), 1250);
	});
}

#[test]
fn it_should_payout_only_to_set_miners() {
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: None,
		block_seal_authority: None,
	});
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		NotebooksInBlock::set(vec![(1, 1, 1)]);
		NotebookTick::set(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		System::assert_last_event(
			Event::RewardCreated {
				rewards: vec![BlockPayout {
					account_id: 1,
					ownership: 3750,
					argons: 3750,
					block_seal_authority: None,
					reward_type: BlockRewardType::Miner,
				}],
			}
			.into(),
		);
	});
}

/// make sure deflationary schedule works
#[test]
fn it_should_halve_rewards() {
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: Some(2),
		block_seal_authority: None,
	});
	new_test_ext().execute_with(|| {
		let halving = HalvingBlocks::get() + HalvingBeginBlock::get() + 1;
		System::set_block_number(halving.into());
		ElapsedTicks::set(halving as u64);
		NotebooksInBlock::set(vec![(1, 1, 1)]);
		NotebookTick::set(1);
		BlockRewards::on_initialize(halving.into());
		BlockRewards::on_finalize(halving.into());
		System::assert_last_event(
			Event::RewardCreated {
				rewards: vec![
					BlockPayout {
						account_id: 1,
						ownership: 187500,
						argons: 375000,
						block_seal_authority: None,
						reward_type: BlockRewardType::Miner,
					},
					BlockPayout {
						account_id: 2,
						ownership: 62500,
						argons: 125000,
						block_seal_authority: None,
						reward_type: BlockRewardType::Voter,
					},
				],
			}
			.into(),
		);
	});
}

#[test]
fn it_increments_rewards_until_halving() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			BlockRewards::get_minimum_reward_amounts(1),
			RewardAmounts { argons: 5000, ownership: 5000 }
		);

		let (increments, increment_blocks, final_reward) = IncrementalGrowth::get();
		assert_eq!(
			BlockRewards::get_minimum_reward_amounts(increment_blocks - 1),
			RewardAmounts { argons: 5000, ownership: 5000 }
		);

		assert_eq!(
			BlockRewards::get_minimum_reward_amounts(increment_blocks),
			RewardAmounts { argons: 5000 + increments, ownership: 5000 + increments }
		);

		let halving_begin =
			UniqueSaturatedInto::<u128>::unique_saturated_into(HalvingBeginBlock::get());
		let increments = UniqueSaturatedInto::<u128>::unique_saturated_into(increments);
		let increment_blocks = UniqueSaturatedInto::<u128>::unique_saturated_into(increment_blocks);

		let final_halvings = (halving_begin / increment_blocks) - 1;

		assert_eq!(
			BlockRewards::get_minimum_reward_amounts(halving_begin as u64 - 1),
			RewardAmounts {
				argons: (5000 + increments * final_halvings),
				ownership: (5000 + increments * final_halvings)
			}
		);

		assert_eq!(
			BlockRewards::get_minimum_reward_amounts(HalvingBeginBlock::get() as u64),
			RewardAmounts { argons: final_reward, ownership: final_reward }
		);
	})
}

#[test]
fn it_should_scale_rewards_based_on_notaries() {
	ActiveNotaries::set(vec![1, 2]);
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: Some(2),
		block_seal_authority: None,
	});
	NotebooksInBlock::set(vec![(1, 1, 1)]);
	NotebookTick::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		System::assert_last_event(
			Event::RewardCreated {
				rewards: vec![
					BlockPayout {
						account_id: 1,
						ownership: 3750 / 2,
						argons: 3750 / 2,
						block_seal_authority: None,
						reward_type: BlockRewardType::Miner,
					},
					BlockPayout {
						account_id: 2,
						ownership: 1250 / 2,
						argons: 1250 / 2,
						block_seal_authority: None,
						reward_type: BlockRewardType::Voter,
					},
				],
			}
			.into(),
		);
	});
}

#[test]
fn it_should_disable_compute_rewards_after_bidding_begins() {
	ActiveNotaries::set(vec![1]);
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: Some(2),
		block_seal_authority: None,
	});
	NotebooksInBlock::set(vec![(1, 1, 1)]);
	NotebookTick::set(1);
	new_test_ext().execute_with(|| {
		IsBlockVoteSeal::set(false);
		IsMiningSlotsActive::set(false);
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		assert_eq!(LastRewards::get().len(), 2);

		IsMiningSlotsActive::set(true);
		IsBlockVoteSeal::set(true);
		System::initialize(&2, &System::parent_hash(), &Default::default());
		BlockRewards::on_initialize(2);
		BlockRewards::on_finalize(2);
		assert_eq!(LastRewards::get().len(), 2);

		LastRewards::set(vec![]);
		IsBlockVoteSeal::set(false);
		System::initialize(&3, &System::parent_hash(), &Default::default());
		BlockRewards::on_initialize(3);
		BlockRewards::on_finalize(3);
		assert_eq!(LastRewards::get().len(), 0);
	});
}

#[test]
fn it_should_not_fail_with_no_notebooks() {
	ActiveNotaries::set(vec![1, 2]);
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: Some(2),
		block_seal_authority: None,
	});
	NotebooksInBlock::set(vec![]);
	NotebookTick::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		System::assert_has_event(
			Event::RewardCreateError {
				account_id: 2,
				ownership: 1u128.into(),
				argons: None,
				error: DispatchError::Token(TokenError::BelowMinimum),
			}
			.into(),
		);
		assert_eq!(Balances::reducible_balance(&2, Preservation::Expendable, Fortitude::Polite), 0);
	});
}

#[test]
fn it_should_not_fail_with_no_notaries() {
	ActiveNotaries::set(vec![]);
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: Some(2),
		block_seal_authority: None,
	});
	NotebooksInBlock::set(vec![]);
	NotebookTick::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		System::assert_last_event(
			Event::RewardCreated {
				rewards: vec![
					BlockPayout {
						account_id: 1,
						ownership: 3750,
						argons: 3750,
						block_seal_authority: None,
						reward_type: BlockRewardType::Miner,
					},
					BlockPayout {
						account_id: 2,
						ownership: 1250,
						argons: 1250,
						block_seal_authority: None,
						reward_type: BlockRewardType::Voter,
					},
				],
			}
			.into(),
		);
	});
}

#[test]
fn it_should_modify_block_rewards() {
	ActiveNotaries::set(vec![1]);
	NotebooksInBlock::set(vec![(1, 1, 1)]);
	NotebookTick::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		Circulation::set(1000);
		ArgonsPerBlock::<Test>::set(11_000);
		// negative cpi means price is growing, need to mint
		ArgonCPI::set(FixedI128::from_float(-0.01));
		BlockRewards::adjust_block_argons(10_000);

		assert_eq!(ArgonsPerBlock::<Test>::get(), 11_000);

		// test the reducer separately
		BlockRewardsDampener::set(FixedU128::one());
		// first change at 0.01^ is 14400 * 100
		let minimum_change: Balance = 14400 * 100; // 86.4 argons
		Circulation::set(minimum_change);
		BlockRewards::adjust_block_argons(10_000);
		// won't add anything until we get to this
		assert_eq!(ArgonsPerBlock::<Test>::get(), 11_001);

		ArgonCPI::set(FixedI128::from_float(1.0));

		let minus_pct = 10_901;

		BlockRewards::on_frame_start(2);
		assert_eq!(ArgonsPerBlock::<Test>::get(), minus_pct);
		// it sets rewards for the next cohort
		assert_eq!(
			BlockRewardsByCohort::<Test>::get().into_iter().collect::<Vec<_>>(),
			vec![(3, minus_pct)]
		);

		let rewards = BlockRewards::calculate_reward_amounts(
			3,
			RewardAmounts { argons: 10_000, ownership: 10_000 },
		);
		assert_eq!(rewards.argons, minus_pct);

		assert_eq!(
			BlockRewards::calculate_reward_amounts(
				4,
				RewardAmounts { argons: 10_000, ownership: 10_000 }
			)
			.argons,
			10_000,
			"returns minimum if unknown cohort"
		);

		// ensure we can't go below 100
		ArgonCPI::set(FixedI128::from_float(10.0));
		BlockRewards::adjust_block_argons(10_000);
		assert_eq!(ArgonsPerBlock::<Test>::get(), 10_000);
	});
}

#[test]
fn it_should_dampen_block_rewards() {
	ActiveNotaries::set(vec![1]);
	NotebooksInBlock::set(vec![(1, 1, 1)]);
	NotebookTick::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		Circulation::set(1000);

		ArgonsPerBlock::<Test>::set(25_000);
		// test the reducer separately
		BlockRewardsDampener::set(FixedU128::from_rational(75, 100));
		Circulation::set(14_400_000_000);
		ArgonCPI::set(FixedI128::from_float(-0.1));
		BlockRewards::adjust_block_argons(10_000);
		// 10% of 14_400_000 is 1_440_000, divided by 14_400 blocks is 100_000
		let incr = 100_000;
		let incr_dampened = (incr as f64 * 0.75) as Balance;
		assert_eq!(ArgonsPerBlock::<Test>::get(), 25_000 + incr_dampened);

		// test other direction with dampener
		ArgonCPI::set(FixedI128::from_float(0.1));
		BlockRewards::adjust_block_argons(10_000);
		assert_eq!(ArgonsPerBlock::<Test>::get(), 25_000);
	});
}
