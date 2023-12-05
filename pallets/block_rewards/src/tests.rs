use frame_support::{
	assert_err, assert_ok,
	traits::{
		fungible::{Inspect, Mutate},
		tokens::{Fortitude, Preservation},
		OnFinalize, OnInitialize,
	},
};
use sp_runtime::{DispatchError, TokenError};

use crate::{
	mock::{ArgonBalances, BlockRewards, UlixeeBalances, *},
	BlockPayout, Event,
};
use ulx_primitives::BlockSealerInfo;

#[test]
fn it_should_only_allow_a_single_seal() {
	BlockSealer::set(BlockSealerInfo {
		notaries_included: 1,
		miner_rewards_account: 1,
		block_vote_rewards_account: 2,
	});
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block: (1 + MaturationBlocks::get()).into(),
				rewards: vec![
					BlockPayout { account_id: 1, ulixees: 3750, argons: 3750 },
					BlockPayout { account_id: 2, ulixees: 1250, argons: 1250 },
				],
			}
			.into(),
		);
		assert_eq!(
			ArgonBalances::reducible_balance(&1, Preservation::Expendable, Fortitude::Polite),
			0
		);
		assert_eq!(
			ArgonBalances::reducible_balance(&1, Preservation::Expendable, Fortitude::Force),
			3750
		);
		assert_eq!(
			UlixeeBalances::reducible_balance(&1, Preservation::Expendable, Fortitude::Force),
			3750
		);

		assert_eq!(
			ArgonBalances::reducible_balance(&2, Preservation::Expendable, Fortitude::Force),
			1250
		);
		assert_eq!(
			UlixeeBalances::reducible_balance(&2, Preservation::Expendable, Fortitude::Force),
			1250
		);

		assert_err!(
			<ArgonBalances as Mutate<AccountId>>::transfer(&1, &2, 3000, Preservation::Expendable),
			DispatchError::Token(TokenError::Frozen)
		);

		// test that we can transfer regular funds still
		let _ = ArgonBalances::mint_into(&1, 3000);
		assert_err!(
			<ArgonBalances as Mutate<AccountId>>::transfer(&1, &2, 3001, Preservation::Expendable),
			DispatchError::Token(TokenError::Frozen)
		);
		assert_ok!(<ArgonBalances as Mutate<AccountId>>::transfer(
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
		notaries_included: 1,
		miner_rewards_account: 1,
		block_vote_rewards_account: 2,
	});
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
				rewards: vec![
					BlockPayout { account_id: 1, ulixees: 3750, argons: 3750 },
					BlockPayout { account_id: 2, ulixees: 1250, argons: 1250 },
				],
			}
			.into(),
		);

		System::set_block_number(maturation_block);
		BlockRewards::on_initialize(maturation_block);
		System::assert_last_event(
			Event::RewardUnlocked {
				rewards: vec![
					BlockPayout { account_id: 1, ulixees: 3750, argons: 3750 },
					BlockPayout { account_id: 2, ulixees: 1250, argons: 1250 },
				],
			}
			.into(),
		);
		assert_eq!(
			ArgonBalances::reducible_balance(&1, Preservation::Expendable, Fortitude::Polite),
			3750
		);
	});
}
#[test]
fn it_should_halve_rewards() {
	BlockSealer::set(BlockSealerInfo {
		notaries_included: 1,
		miner_rewards_account: 1,
		block_vote_rewards_account: 2,
	});
	new_test_ext().execute_with(|| {
		let halving = HalvingBlocks::get() + 1;
		System::set_block_number(halving.into());
		BlockRewards::on_initialize(halving.into());
		BlockRewards::on_finalize(halving.into());
		let maturation_block = (halving + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
				rewards: vec![
					BlockPayout { account_id: 1, ulixees: 1875, argons: 3750 },
					BlockPayout { account_id: 2, ulixees: 625, argons: 1250 },
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
		notaries_included: 1,
		miner_rewards_account: 1,
		block_vote_rewards_account: 2,
	});
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
				rewards: vec![
					BlockPayout { account_id: 1, ulixees: 3750 / 2, argons: 3750 / 2 },
					BlockPayout { account_id: 2, ulixees: 1250 / 2, argons: 1250 / 2 },
				],
			}
			.into(),
		);
	});
}

#[test]
fn it_should_not_fail_with_no_notaries() {
	ActiveNotaries::set(vec![1, 2]);
	BlockSealer::set(BlockSealerInfo {
		notaries_included: 0,
		miner_rewards_account: 1,
		block_vote_rewards_account: 2,
	});
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		BlockRewards::on_initialize(1);
		BlockRewards::on_finalize(1);
		let maturation_block = (1 + MaturationBlocks::get()).into();
		System::assert_last_event(
			Event::RewardCreated {
				maturation_block,
				rewards: vec![
					BlockPayout { account_id: 1, ulixees: 0, argons: 0 },
					BlockPayout { account_id: 2, ulixees: 0, argons: 0 },
				],
			}
			.into(),
		);
		assert_eq!(
			ArgonBalances::reducible_balance(&1, Preservation::Expendable, Fortitude::Polite),
			0
		);
	});
}
