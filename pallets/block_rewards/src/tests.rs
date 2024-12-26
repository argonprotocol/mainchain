use frame_support::{
	assert_err, assert_ok,
	traits::{
		fungible::{Inspect, InspectFreeze, Mutate},
		tokens::{Fortitude, Preservation},
		OnFinalize, OnInitialize,
	},
};
use sp_arithmetic::traits::UniqueSaturatedInto;
use sp_core::ByteArray;
use sp_runtime::{DispatchError, FixedU128, TokenError};

use crate::{
	mock::{Balances, BlockRewards, Ownership, *},
	Event, FreezeReason, RewardAmounts,
};
use argon_primitives::{
	block_seal::{BlockPayout, BlockRewardType, RewardSharing},
	BlockSealAuthorityId, BlockSealerInfo,
};

fn test_authority(id: [u8; 32]) -> BlockSealAuthorityId {
	BlockSealAuthorityId::from_slice(&id).unwrap()
}

#[test]
fn it_should_only_allow_a_single_seal() {
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
				maturation_block: (1 + MaturationBlocks::get()).into(),
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
		assert_eq!(Balances::reducible_balance(&1, Preservation::Expendable, Fortitude::Polite), 0);
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

		assert_err!(
			<Balances as Mutate<AccountId>>::transfer(&1, &2, 3000, Preservation::Expendable),
			DispatchError::Token(TokenError::Frozen)
		);

		// test that we can transfer regular funds still
		let _ = Balances::mint_into(&1, 3000);
		assert_err!(
			<Balances as Mutate<AccountId>>::transfer(&1, &2, 3001, Preservation::Expendable),
			DispatchError::Token(TokenError::Frozen)
		);
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
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
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
		let freeze_id = FreezeReason::MaturationPeriod.into();
		assert_eq!(Balances::balance_frozen(&freeze_id, &1), 3750);
		assert_eq!(Ownership::balance_frozen(&freeze_id, &1), 3750);
		assert_eq!(Balances::balance_frozen(&freeze_id, &2), 1250);
		assert_eq!(Ownership::balance_frozen(&freeze_id, &2), 1250);

		System::set_block_number(maturation_block);
		BlockRewards::on_initialize(maturation_block);
		System::assert_last_event(
			Event::RewardUnlocked {
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
		assert_eq!(Balances::free_balance(1), 3750);
		assert_eq!(Ownership::free_balance(1), 3750);
		assert_eq!(Balances::free_balance(2), 1250);
		assert_eq!(Ownership::free_balance(2), 1250);
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
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
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
		NotebooksInBlock::set(vec![(1, 1, 1)]);
		NotebookTick::set(1);
		BlockRewards::on_initialize(halving.into());
		BlockRewards::on_finalize(halving.into());
		let maturation_block = (halving + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
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
			BlockRewards::get_reward_amounts(1),
			RewardAmounts { argons: 5000, ownership: 5000 }
		);

		let (increments, increment_blocks, final_reward) = IncrementalGrowth::get();
		assert_eq!(
			BlockRewards::get_reward_amounts(increment_blocks - 1),
			RewardAmounts { argons: 5000, ownership: 5000 }
		);

		assert_eq!(
			BlockRewards::get_reward_amounts(increment_blocks),
			RewardAmounts { argons: 5000 + increments, ownership: 5000 + increments }
		);

		let halving_begin =
			UniqueSaturatedInto::<u128>::unique_saturated_into(HalvingBeginBlock::get());
		let increments = UniqueSaturatedInto::<u128>::unique_saturated_into(increments);
		let increment_blocks = UniqueSaturatedInto::<u128>::unique_saturated_into(increment_blocks);

		let final_halvings = (halving_begin / increment_blocks) - 1;

		assert_eq!(
			BlockRewards::get_reward_amounts(halving_begin as u64 - 1),
			RewardAmounts {
				argons: (5000 + increments * final_halvings),
				ownership: (5000 + increments * final_halvings)
			}
		);

		assert_eq!(
			BlockRewards::get_reward_amounts(HalvingBeginBlock::get() as u64),
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
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
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
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
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
fn it_should_support_profit_sharing() {
	ActiveNotaries::set(vec![1, 2]);
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: Some(2),
		block_seal_authority: None,
	});
	GetRewardSharing::set(Some(RewardSharing {
		account_id: 3,
		percent_take: FixedU128::from_rational(40, 100),
	}));
	NotebooksInBlock::set(vec![(1, 1, 1), (2, 1, 1)]);
	NotebookTick::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		let maturation_block = (1 + MaturationBlocks::get()).into();
		let share_amount = (3750.0 * 0.4) as u128;
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
				rewards: vec![
					BlockPayout {
						account_id: 1,
						ownership: 3750,
						argons: 3750 - share_amount,
						block_seal_authority: None,
						reward_type: BlockRewardType::Miner,
					},
					BlockPayout {
						account_id: 3,
						ownership: 0,
						argons: share_amount,
						block_seal_authority: None,
						reward_type: BlockRewardType::ProfitShare,
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
		let freeze_id = FreezeReason::MaturationPeriod.into();

		assert_eq!(Balances::balance_frozen(&freeze_id, &3), share_amount);
	});
}
