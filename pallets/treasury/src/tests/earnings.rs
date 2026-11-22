use super::*;

#[test]
fn pool_proofs_fund_once_and_collect_transfers_existing_issuance() {
	for (eligible_principal, expected_rate, expected_reward) in [
		(900u128, FixedU128::from_rational(75, 1_000), 12),
		(1_500, FixedU128::from_rational(75, 1_500), 7),
	] {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			let frame = 7;
			let beneficiary = account(1);
			CirculatingArgon::set(20_000);
			RewardTaperingPolicy::<Test>::mutate(|policy| {
				policy.stabilizer_pools.target_daily_return = FixedU128::from_rational(5, 100)
			});
			RewardReserves::<Test>::mutate(|reserves| {
				reserves.stabilizer_pools = 123;
				reserves.marketing = 100;
			});
			Treasury::run_frame_transition(frame);
			TargetStabilizerPoolPercent::<Test>::put(Percent::from_percent(10));
			RewardTaperingPolicy::<Test>::mutate(|policy| {
				policy.stabilizer_pools.target_daily_return = FixedU128::one()
			});
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 1_000));
			Treasury::run_frame_transition(frame + 1);
			assert_eq!(RewardReserves::<Test>::get().stabilizer_pools, 223);
			assert_eq!(
				StabilizerPoolRewardFrames::<Test>::get(frame),
				Some(StabilizerPoolRewardFrame::Allocated {
					target_pool_value_microgons: 1_000,
					budget: 75
				})
			);
			let claim = StabilizerPoolFrameClaim { frame_id: frame, amount: expected_reward };
			assert_noop!(
				Treasury::claim_rewards(&[claim]),
				Error::<Test>::StabilizerPoolClaimNotReady
			);
			let marketing = RewardReserves::<Test>::get().marketing;
			let issuance = Balances::total_issuance();
			Treasury::finalize_frame_rewards(frame, eligible_principal);
			Treasury::finalize_frame_rewards(frame, eligible_principal * 2);
			assert_eq!(Treasury::frame_reward_rate(frame), Some(expected_rate));
			assert_eq!(Balances::total_issuance(), issuance);
			assert_eq!(RewardReserves::<Test>::get().marketing, marketing);
			assert_eq!(Balances::balance(&beneficiary), 0);

			CollectClaimFrames::set(vec![claim]);
			assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(beneficiary.clone())));
			assert_eq!(Balances::total_issuance(), issuance + expected_reward + 1);
			assert_eq!(RewardReserves::<Test>::get().marketing, marketing);
			assert_eq!(RewardReserves::<Test>::get().stabilizer_pools, 223 - expected_reward);
			let issuance = Balances::total_issuance();
			assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(beneficiary.clone())));
			assert_eq!(Balances::balance(&beneficiary), expected_reward);
			assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 1);
			assert_eq!(Balances::total_issuance(), issuance);
			assert_eq!(
				PositionsByAccount::<Test>::get(&beneficiary)
					.unwrap()
					.collected
					.stabilizer_pool_earnings,
				expected_reward
			);
		});
	}
}

#[test]
fn unfunded_pool_earnings_retry_before_expiry_and_funded_accrual_never_expires() {
	new_test_ext().execute_with(|| {
		let frame_id = 7;
		let beneficiary = account(1);
		StabilizerPoolRewardFrames::<Test>::insert(
			frame_id,
			StabilizerPoolRewardFrame::Finalized { reward_rate: FixedU128::one() },
		);
		RewardReserves::<Test>::mutate(|reserves| {
			reserves.stabilizer_pools = 9;
			reserves.marketing = 0;
		});
		CollectClaimFrames::set(vec![StabilizerPoolFrameClaim { frame_id, amount: 10 }]);
		let issuance = Balances::total_issuance();
		assert_noop!(
			TreasuryPositions::collect(RuntimeOrigin::signed(beneficiary.clone())),
			TokenError::FundsUnavailable
		);
		assert_eq!(SettledClaimFrame::get(), 0);
		assert_eq!(RewardReserves::<Test>::get().stabilizer_pools, 9);
		assert_eq!(Balances::total_issuance(), issuance);

		RewardReserves::<Test>::mutate(|reserves| reserves.stabilizer_pools = 11);
		assert_ok!(with_storage_layer(|| {
			MockStabilizerPoolSettlement::settle_account(&beneficiary)
		}));
		assert_eq!(RewardReserves::<Test>::get().stabilizer_pools, 1);
		let issuance = Balances::total_issuance();
		for frame in 8..=30 {
			CurrentFrameId::set(frame);
			Treasury::run_frame_transition(frame);
		}
		assert!(Treasury::frame_reward_rate(frame_id).is_none());
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(beneficiary.clone())));
		assert_eq!(Balances::balance(&beneficiary), 10);
		assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 1);
		assert_eq!(Balances::total_issuance(), issuance);
	});
}

#[test]
fn frame_allocation_funds_each_source_and_reserves_the_unearned_share() {
	for (securitization, stakes, bond_earnings, vault_earnings) in
		[(10, 0, 2_000_000, 6_266_000), (4, 5, 5_000_000, 8_165_000)]
	{
		new_test_ext().execute_with(|| {
			CurrentFrameId::set(0);
			assert_ok!(TreasuryPositions::prepare_collect());
			System::set_block_number(1);
			MinimumArgonsPerContributor::set(1);
			insert_vault(1, test_vault(10, securitization * MICROGONS_PER_ARGON as u64));
			set_argons(2, 50 * MICROGONS_PER_ARGON);
			assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));
			if stakes > 0 {
				set_ownership(3, 50 * MICROGONS_PER_ARGON);
				assert_ok!(Treasury::buy_argonot_bonds(origin(3), stakes));
			}
			let balance_before = Balances::balance(&account(2));
			set_target_securitization(u128::from(securitization) * MICROGONS_PER_ARGON);
			CurrentFrameId::set(1);
			Treasury::lock_in_vault_capital(1);
			let frame = CurrentFrameVaultCapital::<Test>::get().unwrap();
			assert_eq!(frame.frame_id, 1);
			assert_eq!(frame.total_active_bonds, 4);
			assert_eq!(
				frame.vault_securitization_positions[&1].active_bond_microgons,
				4 * MICROGONS_PER_ARGON
			);

			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			collect_frame(1, &[2, 3]);
			let stake_earnings = if stakes > 0 { 15 * MICROGONS_PER_ARGON } else { 0 };
			let allocated = bond_earnings + stake_earnings + vault_earnings;
			System::assert_has_event(
				Event::<Test>::FrameEarningsAllocated {
					frame_id: 1,
					bid_pool_allocated: allocated,
					stake_earnings,
					bitcoin_earnings: 0,
					bond_earnings,
					vault_earnings,
					reserved_from_circulation: 100 * MICROGONS_PER_ARGON - allocated,
					participating_vaults: 1,
				}
				.into(),
			);
			assert!(CurrentFrameVaultCapital::<Test>::get().is_none());
			assert_eq!(Balances::balance(&account(2)), balance_before + bond_earnings);
			assert_eq!(Balances::balance(&account(3)), stake_earnings);
			assert_eq!(
				PositionsByAccount::<Test>::get(account(2)).unwrap().collected.bond_earnings,
				bond_earnings
			);
			assert_eq!(RewardReserves::<Test>::get().marketing, 10 * MICROGONS_PER_ARGON);
			let profits = LastVaultProfits::get();
			assert_eq!(profits.len(), 1);
			assert_eq!(profits[0].vault_id, 1);
			assert_eq!(profits[0].earnings, vault_earnings);
			assert_eq!(profits[0].earnings_for_vault, vault_earnings);
			assert_eq!(profits[0].capital_contributed, 4 * MICROGONS_PER_ARGON);
			assert_eq!(profits[0].capital_contributed_by_vault, 0);
		});
	}
}

#[test]
fn base_argon_bond_pool_ignores_future_revenue_sharing_terms() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		assert_ok!(TreasuryPositions::prepare_collect());
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(0);

		let vault = test_vault(10, (20 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		set_argons(3, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));
		assert_ok!(Treasury::buy_bonds(
			origin(3),
			1,
			4,
			Some(bonus_approval(1, 3, Permill::from_percent(10), 1, 0, 1)),
		));

		set_target_securitization(20 * MICROGONS_PER_ARGON);
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		collect_frame(1, &[2, 3]);

		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, 6_266_000);
		assert_eq!(
			PositionsByAccount::<Test>::get(account(2)).unwrap().collected.bond_earnings,
			1000000
		);
		assert_eq!(
			PositionsByAccount::<Test>::get(account(3)).unwrap().collected.bond_earnings,
			1000000
		);
	});
}

#[test]
fn frozen_earnings_survive_burning_the_principal() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		assert_ok!(TreasuryPositions::prepare_collect());
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(0);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));

		let bond_lot_id = account_bond_lot_ids(2)[0];
		let balance_before = Balances::balance(&account(2));

		set_target_securitization(10 * MICROGONS_PER_ARGON);
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(<Treasury as TreasuryPoolProvider<TestAccountId>>::encumber_bond_microgons(
			&account(2),
			4 * MICROGONS_PER_ARGON,
			None,
		));
		assert_ok!(Treasury::settle_bond_collateral_burn(
			&account(2),
			4 * MICROGONS_PER_ARGON,
			&mut Weight::zero()
		));
		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.regular_bonds, 0);
		assert_eq!(vault_bonds.flexible_bonds, 0);
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(Treasury::encumbered_bond_microgons(&account(2)), 0);
		assert!(BondLotById::<Test>::get(bond_lot_id).is_none());

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		collect_frame(1, &[2]);

		assert_eq!(Balances::balance(&account(2)), balance_before + 2_000_000);
	});
}

#[test]
fn failed_collect_preserves_indexed_earnings_and_their_backing() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		MinimumArgonsPerContributor::set(1);
		ExistentialDeposit::set(10);
		RewardReserves::<Test>::mutate(|reserves| reserves.marketing = 10);
		assert_ok!(TreasuryPositions::prepare_collect());
		set_ownership(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 1));
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 39));
		assert_ok!(Treasury::distribute_bid_pool(1));
		let custody_before = Balances::balance(&TreasuryPositions::account_id());
		let position_before = PositionsByAccount::<Test>::get(account(2));
		assert_noop!(TreasuryPositions::collect(origin(2)), TokenError::BelowMinimum);
		assert_eq!(PositionsByAccount::<Test>::get(account(2)), position_before);
		assert_eq!(Balances::balance(&TreasuryPositions::account_id()), custody_before);
		set_argons(2, 10);
		assert_ok!(TreasuryPositions::collect(origin(2)));
		assert_ok!(TreasuryPositions::collect(origin(2)));
		assert_eq!(Balances::balance(&account(2)), 15);
		assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 10);
	});
}

#[test]
fn frame_close_distributes_earnings_releases_principal_and_freezes_the_next_frame() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		MinimumArgonsPerContributor::set(1);
		TreasuryExitDelayFrames::set(1);
		CurrentFrameId::set(0);

		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		insert_vault(2, test_vault(11, (10 * MICROGONS_PER_ARGON) as u64));

		set_argons(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);

		let bid_pool_account = BidPoolAccountId::get();
		assert_ok!(Balances::mint_into(&bid_pool_account, 100 * MICROGONS_PER_ARGON));

		set_argons(3, 20 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(3), 2, 2, None));
		let released_bond_lot_id = account_bond_lot_ids(3)[0];
		assert_ok!(Treasury::liquidate_bond_lot(origin(3), released_bond_lot_id,));

		CurrentFrameId::set(2);
		Treasury::run_frame_transition(2);
		assert_ok!(TreasuryPositions::collect(origin(2)));

		assert!(BondLotById::<Test>::get(released_bond_lot_id).is_none());
		assert!(account_bond_lot_ids(3).is_empty());
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(3),),
			0,
		);

		let current = CurrentFrameVaultCapital::<Test>::get().expect("current frame capital");
		assert_eq!(current.frame_id, 2);
		assert!(current.vault_securitization_positions.get(&2).is_some());
		assert_eq!(Balances::balance(&account(42)), 0);
		assert_eq!(LastVaultProfits::get().len(), 2);
		assert_eq!(LastVaultProfits::get()[0].vault_id, 1);
		assert_eq!(LastVaultProfits::get()[0].capital_contributed, 4 * MICROGONS_PER_ARGON);
	});
}

#[test]
fn frame_funding_commits_once_and_failed_collect_preserves_each_owners_backing() {
	for minimum in [75_000, 150_000] {
		new_test_ext().execute_with(|| {
			CurrentFrameId::set(0);
			ExistentialDeposit::set(minimum);
			assert_ok!(TreasuryPositions::prepare_collect());
			RewardTaperingPolicy::<Test>::kill();
			for owner in [2, 3] {
				set_ownership(owner, 1_000 * MICROGONS_PER_ARGON);
				assert_ok!(Treasury::buy_argonot_bonds(origin(owner), 100));
			}
			set_argons(3, minimum);
			RewardReserves::<Test>::mutate(|reserves| reserves.argonot_stakes = 1_000_000);
			AverageArgonCpi::set(ArgonCPI::from_rational(1, 19));
			CurrentFrameId::set(1);
			Treasury::lock_in_vault_capital(1);
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 666_667));

			assert_ok!(Treasury::distribute_bid_pool(1));
			assert_eq!(TreasuryPositions::indexed_frame(), Some(1));
			assert_eq!(Balances::balance(&TreasuryPositions::account_id()), minimum + 222_296);
			assert_eq!(Balances::balance(&BidPoolAccountId::get()), 0);
			assert_eq!(RewardReserves::<Test>::get().argonot_stakes, 877_704);
			assert_eq!(CollectTotals::<Test>::get().funded.stake_earnings, 222_296);
			let issuance = Balances::total_issuance();
			assert_ok!(Treasury::distribute_bid_pool(1));
			assert_eq!(Balances::total_issuance(), issuance);
			assert_eq!(Balances::balance(&account(2)), 0);

			assert_ok!(TreasuryPositions::collect(origin(3)));
			assert_eq!(Balances::balance(&account(3)), minimum + 111_148);
			assert_eq!(Balances::total_issuance(), issuance);
			let mut opening_balance = 0;
			if minimum > 111_148 {
				assert_noop!(TreasuryPositions::collect(origin(2)), TokenError::BelowMinimum);
				assert_eq!(Balances::balance(&TreasuryPositions::account_id()), minimum + 111_148);
				assert_eq!(RewardReserves::<Test>::get().argonot_stakes, 877_704);
				set_argons(2, minimum);
				opening_balance = minimum;
			}

			let issuance = Balances::total_issuance();
			assert_ok!(TreasuryPositions::collect(origin(2)));
			assert_ok!(TreasuryPositions::collect(origin(2)));
			assert_eq!(Balances::balance(&account(2)), opening_balance + 111_148);
			assert_eq!(Balances::balance(&TreasuryPositions::account_id()), minimum);
			assert_eq!(Balances::total_issuance(), issuance);
		});
	}
}

#[test]
fn frame_close_initializes_custody_for_a_small_auction_and_preserves_quantity_boundaries() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		ExistentialDeposit::set(10_000);
		MinimumArgonsPerContributor::set(1);
		set_ownership(2, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 1));
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 1));
		assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 0);

		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 10_000));
		CurrentFrameId::set(2);
		Treasury::run_frame_transition(2);
		assert_eq!(RewardReserves::<Test>::get().marketing, 1_000);
		assert_eq!(TreasuryPositions::indexed_frame(), Some(1));
		assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 11_500);
		assert_eq!(Balances::balance(&BidPoolAccountId::get()), 0);
		let reserves = RewardReserves::<Test>::get();
		assert_eq!(
			reserves.marketing +
				reserves.bitcoin_liquids +
				300 + reserves.stabilizer_pools +
				reserves.liquidity_buys,
			8_500
		);
		assert_eq!(Balances::total_issuance(), 11_500);
		assert_eq!(CurrentFrameVaultCapital::<Test>::get().unwrap().frame_id, 2);

		set_argons(2, 10_000);
		let issuance = Balances::total_issuance();
		assert_ok!(TreasuryPositions::collect(origin(2)));
		assert_eq!(Balances::balance(&account(2)), 11_500);
		assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 10_000);
		assert_eq!(Balances::total_issuance(), issuance);
		let position = PositionsByAccount::<Test>::get(account(2)).unwrap();
		assert_eq!(position.collected.stake_earnings, 1_500);
		assert_eq!(position.earning_positions.stakes.quantity, 2);
		assert_eq!(position.earning_positions.next_change_frame, None);
	});
}

#[test]
fn bitcoin_earnings_use_frozen_promises_against_the_target_and_burn_the_unfilled_share() {
	for (promised, auction, reserve, earned) in
		[(50, 100, 0, 1_500_000), (200, 100, 0, 3_000_000), (50, 0, 2_000_000, 95_041)]
	{
		new_test_ext().execute_with(|| {
			CurrentFrameId::set(0);
			RewardTaperingPolicy::<Test>::mutate(|policy| {
				policy.bitcoin_liquids.excess_reward_payout_percent = Percent::one();
			});
			RewardReserves::<Test>::mutate(|reserves| reserves.bitcoin_liquids = reserve);
			AverageArgonCpi::set(ArgonCPI::from_rational(1, 19));
			let promised = promised * MICROGONS_PER_ARGON;
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&account(2),
				argon_primitives::treasury::PositionQuantity::FissionLiquidity,
				0u128,
				promised
			));
			set_target_securitization(100 * MICROGONS_PER_ARGON);
			CurrentFrameId::set(1);
			Treasury::lock_in_vault_capital(1);
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&account(2),
				argon_primitives::treasury::PositionQuantity::FissionLiquidity,
				promised,
				0u128
			));
			if auction > 0 {
				assert_ok!(Balances::mint_into(
					&BidPoolAccountId::get(),
					auction * MICROGONS_PER_ARGON
				));
			}
			assert_ok!(Treasury::distribute_bid_pool(1));
			assert_eq!(Balances::balance(&account(2)), 0);
			let issuance = Balances::total_issuance();
			assert_eq!(
				RewardReserves::<Test>::get().bitcoin_liquids,
				reserve.saturating_sub(earned)
			);
			assert_eq!(
				pallet_treasury_positions::CollectTotals::<Test>::get().funded.bitcoin_earnings,
				earned
			);
			assert_ok!(TreasuryPositions::collect(origin(2)));
			assert_ok!(TreasuryPositions::collect(origin(2)));
			assert_eq!(Balances::balance(&account(2)), earned);
			assert_eq!(Balances::total_issuance(), issuance);
			assert_eq!(
				PositionsByAccount::<Test>::get(account(2))
					.unwrap()
					.earning_positions
					.bitcoin_liquidity
					.quantity,
				0
			);
		});
	}
}

#[test]
fn root_configures_reward_economics_without_resetting_omitted_values() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_eq!(TargetBitcoinPercent::<Test>::get(), Percent::from_percent(15));
		assert_eq!(UpstreamParticipationFactor::<Test>::get(), Percent::from_percent(5));
		assert_eq!(TargetStabilizerPoolPercent::<Test>::get(), Percent::from_percent(5));
		assert_noop!(
			Treasury::configure_reward_economics(
				origin(1),
				Some(Percent::from_percent(20)),
				Some(Percent::from_percent(5)),
				Some(Percent::from_percent(10)),
				None,
			),
			DispatchError::BadOrigin,
		);

		assert_eq!(ArgonotParticipationFactor::<Test>::get(), Percent::zero());

		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			Some(Percent::from_percent(20)),
			None,
			None,
			None,
		));
		assert_eq!(UpstreamParticipationFactor::<Test>::get(), Percent::from_percent(5));
		assert_eq!(TargetStabilizerPoolPercent::<Test>::get(), Percent::from_percent(5));
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			Some(Percent::from_percent(50)),
			None,
			None,
		));
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			None,
			Some(Percent::from_percent(10)),
			None,
		));
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			None,
			None,
			None
		));
		assert_eq!(TargetBitcoinPercent::<Test>::get(), Percent::from_percent(20));
		assert_eq!(UpstreamParticipationFactor::<Test>::get(), Percent::from_percent(50));
		assert_eq!(TargetStabilizerPoolPercent::<Test>::get(), Percent::from_percent(10));
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			None,
			None,
			Some(Percent::from_percent(20)),
		));
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			None,
			None,
			None,
		));
		assert_eq!(ArgonotParticipationFactor::<Test>::get(), Percent::from_percent(20));
		System::assert_last_event(RuntimeEvent::Treasury(Event::RewardEconomicsConfigured {
			target_bitcoin_percent: Percent::from_percent(20),
			upstream_participation_factor: Percent::from_percent(50),
			target_stabilizer_pool_percent: Percent::from_percent(10),
			argonot_participation_factor: Percent::from_percent(20),
		}));

		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			None,
			Some(Percent::zero()),
			None,
		));
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			None,
			None,
			None
		));
		assert_eq!(TargetStabilizerPoolPercent::<Test>::get(), Percent::zero());
		assert_eq!(TargetBitcoinPercent::<Test>::get(), Percent::from_percent(20));
		assert_eq!(UpstreamParticipationFactor::<Test>::get(), Percent::from_percent(50));
		assert_eq!(ArgonotParticipationFactor::<Test>::get(), Percent::from_percent(20));
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			None,
			None,
			Some(Percent::zero()),
		));
		assert_eq!(ArgonotParticipationFactor::<Test>::get(), Percent::zero());
		assert_eq!(TargetStabilizerPoolPercent::<Test>::get(), Percent::zero());
	});
}

#[test]
fn bid_pool_reserves_remain_unissued_when_auction_budgets_are_reserved_or_returned() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owner = account(1);
		assert_ok!(Balances::mint_into(&owner, 200 * MICROGONS_PER_ARGON));
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		assert_noop!(
			with_storage_layer(|| -> DispatchResult {
				Treasury::distribute_bid_pool(1)?;
				Err(DispatchError::Other("settlement failed"))
			}),
			DispatchError::Other("settlement failed")
		);
		assert_eq!(Treasury::available_budget(), 0);
		assert_eq!(Balances::balance(&BidPoolAccountId::get()), 100 * MICROGONS_PER_ARGON);
		assert_eq!(Balances::total_issuance(), 300 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::distribute_bid_pool(1));
		assert_eq!(Balances::balance(&BidPoolAccountId::get()), 0);
		assert_eq!(Balances::total_issuance(), 200 * MICROGONS_PER_ARGON);
		let reserves = RewardReserves::<Test>::get();
		assert_eq!(reserves.marketing, 10 * MICROGONS_PER_ARGON);
		assert_eq!(reserves.bitcoin_liquids, 0);
		assert_eq!(reserves.stabilizer_pools, 10 * MICROGONS_PER_ARGON);
		assert_eq!(reserves.liquidity_buys, 77 * MICROGONS_PER_ARGON);
		System::assert_has_event(
			Event::<Test>::FrameEarningsAllocated {
				frame_id: 1,
				bid_pool_allocated: 0,
				stake_earnings: 0,
				bitcoin_earnings: 0,
				bond_earnings: 0,
				vault_earnings: 0,
				reserved_from_circulation: 100 * MICROGONS_PER_ARGON,
				participating_vaults: 0,
			}
			.into(),
		);

		assert_ok!(Treasury::reserve_budget(9 * MICROGONS_PER_ARGON));
		assert_eq!(Treasury::available_budget(), 68 * MICROGONS_PER_ARGON);
		assert_eq!(Balances::total_issuance(), 200 * MICROGONS_PER_ARGON);
		assert_noop!(
			with_storage_layer(|| -> DispatchResult {
				Treasury::return_budget(4_500_000)?;
				Err(DispatchError::Other("return failed"))
			}),
			DispatchError::Other("return failed")
		);
		assert_eq!(Treasury::available_budget(), 68 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::return_budget(4_500_000));
		assert_eq!(Treasury::available_budget(), 72_500_000);
		assert_eq!(Balances::balance(&owner), 200 * MICROGONS_PER_ARGON);
		assert_eq!(Balances::total_issuance(), 200 * MICROGONS_PER_ARGON);
		assert_noop!(Treasury::reserve_budget(72_500_001), TokenError::FundsUnavailable);
	});
}

#[test]
fn claim_operational_reward_pays_immediately_when_funded() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		RewardReserves::<Test>::mutate(|reserves| reserves.marketing = 1_000_000);
		let issuance_before = Balances::total_issuance();

		assert_ok!(<Treasury as OperationalRewardsPayer<TestAccountId, u128>>::claim_reward(
			&account(42),
			250_000,
		));
		assert_eq!(Balances::balance(&account(42)), 250_000);
		assert_eq!(RewardReserves::<Test>::get().marketing, 750_000);
		assert_eq!(Balances::total_issuance(), issuance_before + 250_000);
		System::assert_last_event(
			crate::Event::<Test>::RewardReservesUpdated { reserves: RewardReserves::<Test>::get() }
				.into(),
		);
	});
}

#[test]
fn claim_operational_reward_fails_when_insufficient() {
	new_test_ext().execute_with(|| {
		RewardReserves::<Test>::mutate(|reserves| reserves.marketing = 10);

		assert_err!(
			<Treasury as OperationalRewardsPayer<TestAccountId, u128>>::claim_reward(
				&account(42),
				250,
			),
			TokenError::FundsUnavailable
		);
		assert_eq!(Balances::balance(&account(42)), 0);
	});
}

#[test]
fn fixed_daily_returns_compound_to_the_agreed_apy() {
	let policy = crate::DefaultRewardTaperingPolicy::get();
	for (program, apy_percent) in [
		(policy.argonot_stakes, 50),
		(policy.argon_bonds, 100),
		(policy.vault_revenue, 500),
		(policy.stabilizer_pools, 500),
	] {
		let daily_growth = FixedU128::one().saturating_add(program.target_daily_return);
		let annual_growth =
			(0..365).fold(FixedU128::one(), |growth, _| growth.saturating_mul(daily_growth));
		let expected_growth = FixedU128::from_rational(100 + apy_percent, 100);
		assert!(annual_growth.into_inner().abs_diff(expected_growth.into_inner()) < 10_000);
	}
}

#[test]
fn invalid_daily_yield_policies_are_rejected() {
	new_test_ext().execute_with(|| {
		let original = RewardTaperingPolicy::<Test>::get();
		let mut bitcoin_liquids = original.bitcoin_liquids;
		bitcoin_liquids.excess_reward_payout_percent = Percent::from_percent(25);
		let mut program = original.vault_revenue;
		program.target_daily_return = FixedU128::zero();
		assert_noop!(
			Treasury::configure_reward_tapering_policy(
				RuntimeOrigin::root(),
				Some(Box::new(bitcoin_liquids)),
				None,
				None,
				Some(Box::new(program)),
				None
			),
			Error::<Test>::InvalidRewardTaperingPolicy
		);

		let mut program = original.vault_revenue;
		program
			.target_daily_return_with_price_below_target
			.as_mut()
			.unwrap()
			.argon_price_drop_for_target = Percent::zero();
		assert_noop!(
			Treasury::configure_reward_tapering_policy(
				RuntimeOrigin::root(),
				None,
				None,
				None,
				Some(Box::new(program)),
				None
			),
			Error::<Test>::InvalidRewardTaperingPolicy
		);
	});
}

#[test]
fn daily_apy_threshold_pays_half_the_excess() {
	let daily = crate::DefaultRewardTaperingPolicy::get().vault_revenue.target_daily_return;
	let capital = 100 * MICROGONS_PER_ARGON;
	let half = Percent::from_percent(50);
	// Compounding 0.4920998589% daily yields 500% APY.
	assert_eq!(daily.saturating_mul_int(capital), 492_099);
	for (proposed, expected) in [
		(0, 0),
		(492_098, 492_098),
		(492_099, 492_099),
		(492_100, 492_099),
		(492_101, 492_100),
		(492_200, 492_149),
		(1_000_000, 746_049),
		(u128::MAX, u128::MAX / 2 + 246_050),
	] {
		assert_eq!(Treasury::tapered_payout(proposed, capital, daily, half), expected);
	}
	assert_eq!(Treasury::tapered_payout(100, 0, daily, half), 0);
}

#[test]
fn capital_and_reward_policy_changes_apply_to_the_next_frame() {
	for (next_capital, next_daily_return, next_earnings) in
		[(200, None, 2_992_099), (100, Some(FixedU128::zero()), 2_500_000)]
	{
		new_test_ext().execute_with(|| {
			RewardTaperingPolicy::<Test>::kill();
			insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
			set_target_securitization(100 * MICROGONS_PER_ARGON);
			Treasury::lock_in_vault_capital(1);

			insert_vault(1, test_vault(10, next_capital * MICROGONS_PER_ARGON as u64));
			if let Some(daily_return) = next_daily_return {
				let original = RewardTaperingPolicy::<Test>::get();
				let mut program = original.vault_revenue;
				program.target_daily_return = daily_return;
				program.target_daily_return_with_price_below_target = None;
				assert_ok!(Treasury::configure_reward_tapering_policy(
					RuntimeOrigin::root(),
					None,
					None,
					None,
					Some(Box::new(program)),
					None
				));
				assert_eq!(
					RewardTaperingPolicy::<Test>::get(),
					TaperingPolicy { vault_revenue: program, ..original }
				);
			}
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			assert_ok!(Treasury::distribute_bid_pool(1));
			assert_eq!(LastVaultProfits::get()[0].earnings, 2_746_049);

			Treasury::lock_in_vault_capital(2);
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			assert_ok!(Treasury::distribute_bid_pool(2));
			assert_eq!(LastVaultProfits::get()[1].earnings, next_earnings);
		});
	}
}

#[test]
fn bond_and_stake_reserves_are_self_funded_and_draw_only_below_price_target() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		assert_ok!(TreasuryPositions::prepare_collect());
		System::set_block_number(1);
		MinimumArgonsPerContributor::set(1);
		RewardTaperingPolicy::<Test>::kill();

		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 200 * MICROGONS_PER_ARGON);
		set_ownership(3, 1_000 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 100, None));
		assert_ok!(Treasury::buy_argonot_bonds(origin(3), 100));

		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		set_target_securitization(100 * MICROGONS_PER_ARGON);
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		collect_frame(1, &[2, 3]);
		let funded = RewardReserves::<Test>::get();
		assert!(funded.argon_bonds > 0);
		assert!(funded.argonot_stakes > 0);
		assert_eq!(funded.stabilizer_pools, 10 * MICROGONS_PER_ARGON);

		CurrentFrameId::set(2);
		Treasury::lock_in_vault_capital(2);
		collect_frame(2, &[2, 3]);
		assert_eq!(RewardReserves::<Test>::get(), funded);

		// Reserve support follows ARGON's frame average, regardless of the closing spot price.
		ArgonPriceInUsd::set(FixedU128::one());
		AverageArgonCpi::set(ArgonCPI::from_rational(1, 19));
		ArgonotPriceInUsd::set(FixedU128::from_u32(200));
		AverageArgonotPriceInMicrogons::set(2 * MICROGONS_PER_ARGON);
		CurrentFrameId::set(3);
		Treasury::lock_in_vault_capital(3);
		let issuance_before = Balances::total_issuance();
		let bond_balance_before = Balances::balance(&account(2));
		let stake_balance_before = Balances::balance(&account(3));
		collect_frame(3, &[2, 3]);

		let bond_paid = Balances::balance(&account(2)) - bond_balance_before;
		let stake_paid = Balances::balance(&account(3)) - stake_balance_before;
		assert_eq!(bond_paid, 190_083);
		assert_eq!(stake_paid, 222_296);
		assert_eq!(LastAverageArgonotPriceFrame::get(), Some(3));
		assert_eq!(RewardReserves::<Test>::get().argon_bonds, funded.argon_bonds - bond_paid);
		assert_eq!(
			RewardReserves::<Test>::get().argonot_stakes,
			funded.argonot_stakes - stake_paid
		);
		assert_eq!(RewardReserves::<Test>::get().stabilizer_pools, funded.stabilizer_pools);
		assert_eq!(Balances::total_issuance(), issuance_before + bond_paid + stake_paid);
	});
}

#[test]
fn splitting_bond_lots_does_not_increase_rewards() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		assert_ok!(TreasuryPositions::prepare_collect());
		MinimumArgonsPerContributor::set(1);
		RewardTaperingPolicy::<Test>::kill();
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 50 * MICROGONS_PER_ARGON);
		set_argons(3, 50 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 50, None));
		assert_ok!(Treasury::buy_bonds(origin(3), 1, 50, None));

		set_target_securitization(100 * MICROGONS_PER_ARGON);
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		collect_frame(1, &[2, 3]);

		// The 5 ARGON allocation pays 2,595,041 before per-lot rounding, regardless of splitting.
		assert_eq!(RewardReserves::<Test>::get().argon_bonds, 2_404_959);
		for owner in [2, 3] {
			assert_eq!(Balances::balance(&account(owner)), 1_297_520);
		}
	});
}

#[test]
fn limited_reserves_fund_proportional_collects_including_subminimum_shares() {
	for (minimum, reserve, opening_balance) in [(10, 100_001, 0), (10_000, 10_000, 10_000)] {
		new_test_ext().execute_with(|| {
			CurrentFrameId::set(0);
			System::set_block_number(1);
			ExistentialDeposit::set(minimum);
			assert_ok!(TreasuryPositions::prepare_collect());
			MinimumArgonsPerContributor::set(1);
			RewardTaperingPolicy::<Test>::kill();
			insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
			for owner in [2, 3] {
				set_argons(owner, 50 * MICROGONS_PER_ARGON + opening_balance);
				assert_ok!(Treasury::buy_bonds(origin(owner), 1, 50, None));
			}
			for owner in [4, 5] {
				if opening_balance > 0 {
					set_argons(owner, opening_balance);
				}
				set_ownership(owner, 1_000 * MICROGONS_PER_ARGON);
				assert_ok!(Treasury::buy_argonot_bonds(origin(owner), 100));
			}
			RewardReserves::<Test>::mutate(|reserves| {
				reserves.argon_bonds = reserve;
				reserves.argonot_stakes = reserve;
			});
			AverageArgonCpi::set(ArgonCPI::from_rational(1, 19));
			set_target_securitization(100 * MICROGONS_PER_ARGON);
			CurrentFrameId::set(1);
			Treasury::lock_in_vault_capital(1);
			let issuance = Balances::total_issuance();

			collect_frame(1, &[2, 3, 4, 5]);
			for owner in [2, 3, 4, 5] {
				assert_eq!(Balances::balance(&account(owner)), opening_balance + reserve / 2);
			}
			assert_eq!(RewardReserves::<Test>::get().argon_bonds, 0);
			assert_eq!(RewardReserves::<Test>::get().argonot_stakes, 0);
			assert_eq!(CollectTotals::<Test>::get().funded.bond_earnings, reserve);
			assert_eq!(Balances::total_issuance(), issuance + 2 * reserve);
			assert_eq!(
				Balances::balance(&TreasuryPositions::account_id()),
				minimum + 2 * (reserve % 2)
			);
			CurrentFrameId::set(2);
			Treasury::lock_in_vault_capital(2);
			collect_frame(2, &[2, 3, 4, 5]);
			assert_eq!(Balances::total_issuance(), issuance + 2 * reserve);
		});
	}
}

#[test]
fn bond_reserve_shares_floor_without_spending_other_allocations() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		assert_ok!(TreasuryPositions::prepare_collect());
		System::set_block_number(1);
		RewardTaperingPolicy::<Test>::kill();
		TargetBitcoinPercent::<Test>::put(Percent::zero());
		let bonds_per_vault = 4_000_000_000;
		let capital_per_vault = u128::from(bonds_per_vault) * MICROGONS_PER_ARGON;
		set_argons(2, 501 * capital_per_vault);
		for vault_id in 1..=501 {
			insert_vault(vault_id, test_vault(u64::from(vault_id) + 10, capital_per_vault as u64));
			assert_ok!(Treasury::buy_bonds(origin(2), vault_id, bonds_per_vault, None));
		}
		RewardReserves::<Test>::mutate(|reserves| reserves.argon_bonds = 500);
		AverageArgonCpi::set(ArgonCPI::from_rational(1, 19));
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		let bid_revenue = 100 * MICROGONS_PER_ARGON;
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), bid_revenue));

		collect_frame(1, &[2]);

		let bond_paid = Balances::balance(&account(2));
		assert_eq!(bond_paid, 5_000_499);
		let vault_paid: u128 = LastVaultProfits::get().iter().map(|profit| profit.earnings).sum();
		let reserves = RewardReserves::<Test>::get();
		assert_eq!(reserves.argon_bonds, 0);
		let reserved = reserves.marketing +
			reserves.bitcoin_liquids +
			3 * MICROGONS_PER_ARGON +
			reserves.stabilizer_pools +
			reserves.liquidity_buys;
		assert_eq!(reserved + 5_000_500 + vault_paid, bid_revenue + 500);
	});
}
