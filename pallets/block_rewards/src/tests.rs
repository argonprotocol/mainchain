use frame_support::{
	assert_err, assert_ok,
	traits::{
		fungible::{Inspect, InspectFreeze, Mutate},
		tokens::{Fortitude, Preservation},
		OnFinalize, OnInitialize,
	},
};
use sp_runtime::{DispatchError, FixedU128, TokenError};

use crate::{
	mock::{Balances, BlockRewards, Ownership, *},
	Event, FreezeReason,
};
use argon_primitives::{
	block_seal::{BlockPayout, RewardSharing},
	BlockSealerInfo,
};

#[test]
fn it_should_only_allow_a_single_seal() {
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: Some(2),
	});
	NotebooksInBlock::set(vec![(1, 1, 1)]);
	CurrentTick::set(1);
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block: (1 + MaturationBlocks::get()).into(),
				rewards: vec![
					BlockPayout { account_id: 1, ownership: 3750, argons: 3750 },
					BlockPayout { account_id: 2, ownership: 1250, argons: 1250 },
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
	});
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		NotebooksInBlock::set(vec![(1, 1, 1)]);
		CurrentTick::set(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
				rewards: vec![
					BlockPayout { account_id: 1, ownership: 3750, argons: 3750 },
					BlockPayout { account_id: 2, ownership: 1250, argons: 1250 },
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
					BlockPayout { account_id: 1, ownership: 3750, argons: 3750 },
					BlockPayout { account_id: 2, ownership: 1250, argons: 1250 },
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
fn it_should_payout_block_vote_to_miner() {
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: None,
	});
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		NotebooksInBlock::set(vec![(1, 1, 1)]);
		CurrentTick::set(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
				rewards: vec![
					BlockPayout { account_id: 1, ownership: 3750, argons: 3750 },
					BlockPayout { account_id: 1, ownership: 1250, argons: 1250 },
				],
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
	});
	new_test_ext().execute_with(|| {
		let halving = HalvingBlocks::get() + 1;
		System::set_block_number(halving.into());
		NotebooksInBlock::set(vec![(1, 1, 1)]);
		CurrentTick::set(1);
		BlockRewards::on_initialize(halving.into());
		BlockRewards::on_finalize(halving.into());
		let maturation_block = (halving + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
				rewards: vec![
					BlockPayout { account_id: 1, ownership: 1875, argons: 3750 },
					BlockPayout { account_id: 2, ownership: 625, argons: 1250 },
				],
			}
			.into(),
		);
	});
}

#[test]
fn it_should_scale_rewards_based_on_notaries() {
	ActiveNotaries::set(vec![1, 2]);
	BlockSealer::set(BlockSealerInfo {
		block_author_account_id: 1,
		block_vote_rewards_account: Some(2),
	});
	NotebooksInBlock::set(vec![(1, 1, 1)]);
	CurrentTick::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
				rewards: vec![
					BlockPayout { account_id: 1, ownership: 3750 / 2, argons: 3750 / 2 },
					BlockPayout { account_id: 2, ownership: 1250 / 2, argons: 1250 / 2 },
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
	});
	NotebooksInBlock::set(vec![]);
	CurrentTick::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
				rewards: vec![
					BlockPayout { account_id: 1, ownership: 0, argons: 0 },
					BlockPayout { account_id: 2, ownership: 0, argons: 0 },
				],
			}
			.into(),
		);
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
	});
	NotebooksInBlock::set(vec![]);
	CurrentTick::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
				rewards: vec![
					BlockPayout { account_id: 1, ownership: 3750, argons: 3750 },
					BlockPayout { account_id: 2, ownership: 1250, argons: 1250 },
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
	});
	GetRewardSharing::set(Some(RewardSharing {
		account_id: 3,
		percent_take: FixedU128::from_rational(40, 100),
	}));
	NotebooksInBlock::set(vec![(1, 1, 1), (2, 1, 1)]);
	CurrentTick::set(1);
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
					BlockPayout { account_id: 1, ownership: 3750, argons: 3750 - share_amount },
					BlockPayout { account_id: 3, ownership: 0, argons: share_amount },
					BlockPayout { account_id: 2, ownership: 1250, argons: 1250 },
				],
			}
			.into(),
		);
		let freeze_id = FreezeReason::MaturationPeriod.into();

		assert_eq!(Balances::balance_frozen(&freeze_id, &3), share_amount);
	});
}
