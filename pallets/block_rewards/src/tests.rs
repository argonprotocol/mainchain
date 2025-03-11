use frame_support::{
	assert_ok,
	traits::{
		fungible::{Inspect, Mutate},
		tokens::{Fortitude, Preservation},
		Len, OnFinalize, OnInitialize,
	},
};
use sp_arithmetic::{traits::UniqueSaturatedInto, FixedI128};
use sp_core::ByteArray;
use sp_runtime::{DispatchError, TokenError};

use crate::{
	mock::{Balances, BlockRewards, Ownership, *},
	pallet::{ArgonsPerBlock, ArgonsPerBlockHistory, BlockRewardsByCohort},
	Event, RewardAmounts,
};
use argon_primitives::{
	block_seal::{BlockPayout, BlockRewardType},
	BlockSealAuthorityId, BlockSealerInfo, OnNewSlot,
};

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
		assert_ok!(<Balances as Mutate<AccountId>>::transfer(
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
		ArgonsPerBlockHistory::<Test>::try_append(45_000).unwrap();
		ArgonsPerBlock::<Test>::set(45_000);
		ArgonCPI::set(FixedI128::from_float(0.01));
		BlockRewards::adjust_block_argons(10_000);

		// added 1%
		assert_eq!(ArgonsPerBlock::<Test>::get(), 45_450);
		assert_eq!(ArgonsPerBlockHistory::<Test>::get(), vec![45_000, 45_450]);

		// we should be capped out now
		BlockRewards::adjust_block_argons(10_000);
		assert_eq!(ArgonsPerBlock::<Test>::get(), 45_450);
		assert_eq!(ArgonsPerBlockHistory::<Test>::get(), vec![45_000, 45_450, 45_450]);

		ArgonCPI::set(FixedI128::from_float(1.0));
		for _ in 0..58 {
			BlockRewards::adjust_block_argons(10_000);
			assert_eq!(ArgonsPerBlock::<Test>::get(), 45_450);
		}
		assert_eq!(ArgonsPerBlockHistory::<Test>::get().len(), 60);
		// now we've changed baseline
		assert_eq!(ArgonsPerBlockHistory::<Test>::get()[0], 45450);
		BlockRewards::adjust_block_argons(10_000);
		assert_eq!(ArgonsPerBlock::<Test>::get(), 45_905);

		// now simulate falling
		ArgonCPI::set(FixedI128::from_float(-0.1));
		BlockRewards::adjust_block_argons(10_000);
		// max decrease is 2% in an hour
		assert_eq!(ArgonsPerBlock::<Test>::get(), 45450 - (454.5 * 2.0) as u128);

		// can't go farther
		ArgonCPI::set(FixedI128::from_float(-1.0));
		BlockRewards::adjust_block_argons(10_000);
		assert_eq!(ArgonsPerBlock::<Test>::get(), 45450 - (454.5 * 2.0) as u128);

		ArgonCPI::set(FixedI128::from_float(0.1));
		BlockRewards::adjust_block_argons(10_000);
		assert_eq!(ArgonsPerBlock::<Test>::get(), 45905);

		BlockRewards::on_new_cohort(2);
		// it sets rewards for the next cohort
		assert_eq!(
			BlockRewardsByCohort::<Test>::get().into_iter().collect::<Vec<_>>(),
			vec![(3, 45905)]
		);

		let rewards = BlockRewards::calculate_reward_amounts(
			3,
			RewardAmounts { argons: 10_000, ownership: 10_000 },
		);
		assert_eq!(rewards.argons, 45905);

		assert_eq!(
			BlockRewards::calculate_reward_amounts(
				4,
				RewardAmounts { argons: 10_000, ownership: 10_000 }
			)
			.argons,
			10_000,
			"returns minimum if unknown cohort"
		);
	});
}

#[test]
fn it_wont_change_rewards_below_minimum() {
	ActiveNotaries::set(vec![1]);
	NotebooksInBlock::set(vec![(1, 1, 1)]);
	NotebookTick::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BlockRewards::adjust_block_argons(10_000);
		assert_eq!(ArgonsPerBlockHistory::<Test>::get(), vec![10_000]);
		assert_eq!(ArgonsPerBlock::<Test>::get(), 10_000);

		ArgonCPI::set(FixedI128::from_float(0.01));
		BlockRewards::adjust_block_argons(10_000);
		assert_eq!(ArgonsPerBlock::<Test>::get(), 10_100);
		assert_eq!(ArgonsPerBlockHistory::<Test>::get(), vec![10_000, 10_100]);

		ArgonCPI::set(FixedI128::from_float(-0.01));
		BlockRewards::adjust_block_argons(10_000);

		// max reduction is 2%. but floor is 10_000
		assert_eq!(ArgonsPerBlock::<Test>::get(), 10_000);
		assert_eq!(ArgonsPerBlockHistory::<Test>::get(), vec![10_000, 10_100, 10_000]);
	});
}
