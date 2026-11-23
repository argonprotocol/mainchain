use super::*;

#[test]
fn vault_snapshot_uses_raw_securitization_and_the_effective_bitcoin_target() {
	new_test_ext().execute_with(|| {
		MintedBitcoinMicrogons::set(200 * MICROGONS_PER_ARGON);
		Balances::set_total_issuance(1_200 * MICROGONS_PER_ARGON);

		let mut idle_vault = test_vault(10, 0);
		idle_vault.securitization = 75 * MICROGONS_PER_ARGON;
		insert_vault(1, idle_vault);

		Treasury::lock_in_vault_capital(1);

		let snapshot = CurrentFrameVaultCapital::<Test>::get().expect("frame capital");
		assert_eq!(snapshot.frame_id, 1);
		assert_eq!(snapshot.target_securitization, 200 * MICROGONS_PER_ARGON);
		assert_eq!(snapshot.total_securitization, 75 * MICROGONS_PER_ARGON);
		assert_eq!(
			snapshot
				.vault_securitization_positions
				.get(&1)
				.map(|entry| entry.securitization),
			Some(75 * MICROGONS_PER_ARGON)
		);

		MintedBitcoinMicrogons::set(100 * MICROGONS_PER_ARGON);
		Balances::set_total_issuance(1_100 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(2);
		let snapshot = CurrentFrameVaultCapital::<Test>::get().expect("frame capital");
		assert_eq!(snapshot.target_securitization, 150 * MICROGONS_PER_ARGON);
	});
}

#[test]
fn vault_rewards_use_only_covered_bitcoin_and_keep_frame_start_backing() {
	for (satoshis, covered, bonds, argonots, expected) in [
		(0, 0, 0, 0, 5_000_000),
		(0, 0, 0, 10, 5_000_000),
		(0, 0, 0, 20, 5_000_000),
		(0, 0, 0, 100, 5_000_000),
		(100_000_000, 0, 0, 0, 5_000_000),
		(100_000_000, 100_000_000, 10, 20, 51_000_000),
	] {
		new_test_ext().execute_with(|| {
			assert_ok!(Treasury::configure_reward_economics(
				RuntimeOrigin::root(),
				None,
				None,
				None,
				Some(Percent::from_percent(100))
			));
			MinimumArgonsPerContributor::set(1);
			let mut vault = test_vault(10, (10 * MICROGONS_PER_ARGON) as u64);
			if covered > 0 {
				vault.activated_securitization = 10 * MICROGONS_PER_ARGON;
			}
			insert_vault(1, vault);
			VaultBitcoinSatoshis::mutate(|bitcoin| {
				bitcoin.insert(1, satoshis);
			});
			set_vault_securitized_satoshis(1, covered);
			VaultArgonotMicronots::mutate(|backing| {
				backing.insert(1, argonots * MICROGONS_PER_ARGON);
			});
			if bonds > 0 {
				set_argons(2, u128::from(bonds) * MICROGONS_PER_ARGON);
				assert_ok!(Treasury::buy_bonds(origin(2), 1, bonds, None));
			}
			set_target_securitization(10 * MICROGONS_PER_ARGON);
			Treasury::lock_in_vault_capital(1);
			assert_eq!(VaultsById::get()[&1].committed_microgons, 10 * MICROGONS_PER_ARGON);
			assert_eq!(
				VaultRewardCommittedMicronots::get().get(&1).copied().unwrap_or_default(),
				argonots.min(20) * MICROGONS_PER_ARGON
			);
			let frame = CurrentFrameVaultCapital::<Test>::get().unwrap();
			assert_eq!(
				frame.vault_securitization_positions[&1].argonot_securitization_in_microgons,
				argonots * MICROGONS_PER_ARGON
			);

			VaultBitcoinSatoshis::set(Default::default());
			set_vault_securitized_satoshis(1, 0);
			VaultArgonotMicronots::set(Default::default());
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			assert_ok!(Treasury::distribute_bid_pool(1));
			let earnings = LastVaultProfits::get()[0].earnings_for_vault;
			assert!(earnings.abs_diff(expected) <= 1, "{earnings} vs {expected}");
		});
	}
}

#[test]
fn vault_reward_tiers_increase_independently() {
	for (securitization, covered_satoshis, bonds, expected_earnings) in [
		(10, 0, 0, 2_500_000),
		(15, 0, 0, 3_000_000),
		(15, 0, 10, 4_582_500),
		(15, 100_000_000, 0, 24_417_500),
		(15, 100_000_000, 10, 26_000_000),
	] {
		new_test_ext().execute_with(|| {
			MinimumArgonsPerContributor::set(1);
			let mut vault = test_vault(10, securitization * MICROGONS_PER_ARGON as u64);
			if covered_satoshis > 0 {
				vault.activated_securitization = 10 * MICROGONS_PER_ARGON;
			}
			insert_vault(1, vault);
			insert_vault(2, test_vault(11, 10 * MICROGONS_PER_ARGON as u64));
			VaultBitcoinSatoshis::mutate(|bitcoin| {
				bitcoin.insert(1, covered_satoshis);
			});
			set_vault_securitized_satoshis(1, covered_satoshis);
			if bonds > 0 {
				set_argons(3, 10 * MICROGONS_PER_ARGON);
				assert_ok!(Treasury::buy_bonds(origin(3), 1, bonds, None));
			}
			set_target_securitization(20 * MICROGONS_PER_ARGON);
			Treasury::lock_in_vault_capital(1);
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			assert_ok!(Treasury::distribute_bid_pool(1));

			let earnings = LastVaultProfits::get()
				.into_iter()
				.find(|entry| entry.vault_id == 1)
				.expect("vault payout")
				.earnings_for_vault;
			assert!(earnings.abs_diff(expected_earnings) <= 1, "{earnings} vs {expected_earnings}");
		});
	}
}

#[test]
fn bounded_vault_bitcoin_share_uses_all_network_covered_satoshis() {
	new_test_ext().execute_with(|| {
		MaxVaultsPerPool::set(1);
		for vault_id in [1, 2] {
			insert_vault(
				vault_id,
				test_vault(vault_id as u64 + 10, 10 * MICROGONS_PER_ARGON as u64),
			);
			VaultBitcoinSatoshis::mutate(|bitcoin| {
				bitcoin.insert(vault_id, 100_000_000);
			});
			set_vault_securitized_satoshis(vault_id, 100_000_000);
		}
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		assert_eq!(
			CurrentFrameVaultBitcoin::<Test>::get()
				.unwrap()
				.total_network_securitized_satoshis,
			200_000_000
		);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		assert_ok!(Treasury::distribute_bid_pool(1));
		assert_eq!(LastVaultProfits::get().len(), 1);
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, 23_917_500);
	});
}

#[test]
fn a_frame_frozen_before_the_bitcoin_snapshot_uses_its_original_formula() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, 10 * MICROGONS_PER_ARGON as u64));
		set_argons(2, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		CurrentFrameVaultBitcoin::<Test>::kill();
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		assert_ok!(Treasury::distribute_bid_pool(1));
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, MICROGONS_PER_ARGON);

		CurrentFrameId::set(2);
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(2);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		assert_ok!(Treasury::distribute_bid_pool(2));
		assert_eq!(LastVaultProfits::get().last().unwrap().earnings_for_vault, 8_165_000);
	});
}

#[test]
fn zero_target_and_empty_variable_tiers_keep_the_securitization_reward() {
	new_test_ext().execute_with(|| {
		insert_vault(1, test_vault(10, 10 * MICROGONS_PER_ARGON as u64));
		TargetBitcoinPercent::<Test>::put(Percent::zero());
		MintedBitcoinMicrogons::set(0);
		Balances::set_total_issuance(0);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		assert_ok!(Treasury::distribute_bid_pool(1));
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, 5 * MICROGONS_PER_ARGON);
	});
}

#[test]
fn zero_argonot_weight_removes_backing_requirements_without_reducing_the_vault_pool() {
	for (activated, satoshis, bonds, expected_earnings) in [
		(0, 0, 0, 5_000_000),
		(0, 0, 10, 8_165_000),
		(10, 100_000_000, 0, 47_835_000),
		(10, 100_000_000, 10, 51_000_000),
	] {
		for (argonot_backing, argonot_price) in
			[(0, 0), (20 * MICROGONS_PER_ARGON, MICROGONS_PER_ARGON)]
		{
			new_test_ext().execute_with(|| {
				assert_eq!(ArgonotParticipationFactor::<Test>::get(), Percent::zero());
				MinimumArgonsPerContributor::set(1);
				AverageArgonotPriceInMicrogons::set(argonot_price);
				let mut vault = test_vault(10, (10 * MICROGONS_PER_ARGON) as u64);
				vault.activated_securitization = activated * MICROGONS_PER_ARGON;
				insert_vault(1, vault);
				VaultBitcoinSatoshis::mutate(|bitcoin| {
					bitcoin.insert(1, satoshis);
				});
				set_vault_securitized_satoshis(1, satoshis);
				VaultArgonotMicronots::mutate(|backing| {
					backing.insert(1, argonot_backing);
				});
				if bonds > 0 {
					set_argons(2, 10 * MICROGONS_PER_ARGON);
					assert_ok!(Treasury::buy_bonds(origin(2), 1, bonds, None));
				}
				set_target_securitization(10 * MICROGONS_PER_ARGON);

				Treasury::lock_in_vault_capital(1);
				assert_eq!(VaultRewardCommittedMicronots::get().get(&1).copied(), Some(0));
				assert_ok!(Balances::mint_into(
					&BidPoolAccountId::get(),
					100 * MICROGONS_PER_ARGON
				));
				assert_ok!(Treasury::distribute_bid_pool(1));

				let earnings = LastVaultProfits::get().last().unwrap().earnings_for_vault;
				assert!(earnings.abs_diff(expected_earnings) <= 1);
			});
		}
	}
}

#[test]
fn argonot_weight_only_controls_its_share_of_variable_vault_earnings() {
	for (weight, backing, expected_earnings) in [
		(0, 0, 51_000_000),
		(20, 0, 41_800_000),
		(20, 10, 46_400_000),
		(20, 20, 51_000_000),
		(20, 30, 51_000_000),
		(100, 0, 5_000_000),
		(100, 10, 28_000_000),
		(100, 20, 51_000_000),
	] {
		new_test_ext().execute_with(|| {
			MinimumArgonsPerContributor::set(1);
			let mut vault = test_vault(10, 10 * MICROGONS_PER_ARGON as u64);
			vault.activated_securitization = 10 * MICROGONS_PER_ARGON;
			insert_vault(1, vault);
			VaultBitcoinSatoshis::mutate(|bitcoin| {
				bitcoin.insert(1, 100_000_000);
			});
			set_vault_securitized_satoshis(1, 100_000_000);
			VaultArgonotMicronots::mutate(|argonots| {
				argonots.insert(1, backing * MICROGONS_PER_ARGON);
			});
			set_argons(2, 10 * MICROGONS_PER_ARGON);
			assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));
			set_target_securitization(10 * MICROGONS_PER_ARGON);
			assert_ok!(Treasury::configure_reward_economics(
				RuntimeOrigin::root(),
				None,
				None,
				None,
				Some(Percent::from_percent(weight)),
			));
			Treasury::lock_in_vault_capital(1);
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			assert_ok!(Treasury::distribute_bid_pool(1));
			let earnings = LastVaultProfits::get()[0].earnings_for_vault;
			assert!(
				earnings.abs_diff(expected_earnings) <= 1,
				"weight {weight}, backing {backing}: {earnings} vs {expected_earnings}"
			);
		});
	}
}

#[test]
fn argonot_weight_updates_apply_to_the_next_frame_and_preserve_pre_upgrade_payouts() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		let mut vault = test_vault(10, 10 * MICROGONS_PER_ARGON as u64);
		vault.activated_securitization = 10 * MICROGONS_PER_ARGON;
		insert_vault(1, vault);
		VaultBitcoinSatoshis::mutate(|bitcoin| {
			bitcoin.insert(1, 100_000_000);
		});
		set_vault_securitized_satoshis(1, 100_000_000);
		set_argons(2, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			None,
			None,
			Some(Percent::from_percent(20)),
		));
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			None,
			None,
			Some(Percent::from_percent(100)),
		));
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		assert_ok!(Treasury::distribute_bid_pool(1));
		assert!(LastVaultProfits::get()[0].earnings_for_vault.abs_diff(41_800_000) <= 1);
		assert!(CurrentFrameArgonotParticipation::<Test>::get().is_none());

		CurrentFrameId::set(2);
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(2);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		assert_ok!(Treasury::distribute_bid_pool(2));
		assert_eq!(LastVaultProfits::get()[1].earnings_for_vault, 5 * MICROGONS_PER_ARGON);

		CurrentFrameId::set(3);
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(3);
		// An existing pre-upgrade capital snapshot has no participation companion.
		CurrentFrameArgonotParticipation::<Test>::kill();
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		assert_ok!(Treasury::distribute_bid_pool(3));
		assert!(LastVaultProfits::get()[2].earnings_for_vault.abs_diff(51_000_000) <= 1);
	});
}

#[test]
fn vault_bitcoin_penalty_is_capped_and_does_not_reduce_bond_rewards() {
	for (securitization, activated, satoshis, bonds, expected_earnings) in [
		(10, 8, 50_000_000, 0, 26_417_500),
		(10, 8, 80_000_000, 0, 39_268_000),
		(10, 10, 100_000_000, 0, 47_835_000),
		(10, 10, 150_000_000, 0, 43_551_500),
		(15, 15, 150_000_000, 0, 47_835_000),
		(10, 10, 200_000_000, 0, 39_268_000),
		(10, 10, 300_000_000, 0, 39_268_000),
		(10, 10, 200_000_000, 10, 42_433_000),
	] {
		new_test_ext().execute_with(|| {
			MinimumArgonsPerContributor::set(1);
			let mut vault = test_vault(10, (securitization * MICROGONS_PER_ARGON) as u64);
			vault.activated_securitization = activated * MICROGONS_PER_ARGON;
			insert_vault(1, vault);
			VaultBitcoinSatoshis::mutate(|bitcoin| {
				bitcoin.insert(1, satoshis);
			});
			set_vault_securitized_satoshis(1, satoshis.min(securitization as u64 * 10_000_000));
			if bonds > 0 {
				set_argons(2, 10 * MICROGONS_PER_ARGON);
				assert_ok!(Treasury::buy_bonds(origin(2), 1, bonds, None));
			}
			set_target_securitization(securitization * MICROGONS_PER_ARGON);
			Treasury::lock_in_vault_capital(1);
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			assert_ok!(Treasury::distribute_bid_pool(1));
			let earnings = LastVaultProfits::get()[0].earnings_for_vault;
			assert!(earnings.abs_diff(expected_earnings) <= 1, "{earnings} vs {expected_earnings}");
		});
	}
}

#[test]
fn vault_snapshot_values_argonot_backing_at_the_previous_frame_average() {
	new_test_ext().execute_with(|| {
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			None,
			None,
			Some(Percent::from_percent(100)),
		));
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		VaultArgonotMicronots::mutate(|backing| {
			backing.insert(1, 5 * MICROGONS_PER_ARGON);
		});
		ArgonotPriceInUsd::set(FixedU128::from_u32(3));
		AverageArgonotPriceInMicrogons::set(2 * MICROGONS_PER_ARGON);

		Treasury::lock_in_vault_capital(2);
		assert_eq!(LastAverageArgonotPriceFrame::get(), Some(1));
		let snapshot = CurrentFrameVaultCapital::<Test>::get().expect("frame capital");
		assert_eq!(
			snapshot
				.vault_securitization_positions
				.get(&1)
				.expect("vault position")
				.argonot_securitization_in_microgons,
			10 * MICROGONS_PER_ARGON,
		);
		VaultArgonotMicronots::mutate(|backing| {
			backing.insert(1, 50 * MICROGONS_PER_ARGON);
		});
		AverageArgonotPriceInMicrogons::set(3 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(3);
		assert_eq!(VaultRewardCommittedMicronots::get().get(&1).copied(), Some(6_666_667));
		AverageArgonotPriceInMicrogons::set(0);
		Treasury::lock_in_vault_capital(4);
		assert_eq!(VaultRewardCommittedMicronots::get().get(&1).copied(), Some(6_666_667));
	});
}

#[test]
fn bounded_out_vault_share_burns_but_all_bonds_still_earn() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		assert_ok!(TreasuryPositions::prepare_collect());
		System::set_block_number(1);
		MinimumArgonsPerContributor::set(1);
		MaxVaultsPerPool::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		insert_vault(2, test_vault(11, (10 * MICROGONS_PER_ARGON) as u64));

		set_argons(2, 10 * MICROGONS_PER_ARGON);
		set_argons(3, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));
		assert_ok!(Treasury::buy_bonds(origin(3), 2, 10, None));

		set_target_securitization(20 * MICROGONS_PER_ARGON);
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));

		collect_frame(1, &[2, 3]);
		assert_eq!(LastVaultProfits::get()[0].earnings_for_vault, 4_082_500);
		let distribution = System::events()
			.into_iter()
			.find_map(|record| match record.event {
				RuntimeEvent::Treasury(crate::Event::FrameEarningsAllocated {
					bond_earnings,
					vault_earnings,
					reserved_from_circulation,
					participating_vaults,
					..
				}) => Some((
					bond_earnings,
					vault_earnings,
					reserved_from_circulation,
					participating_vaults,
				)),
				_ => None,
			})
			.expect("distribution event");
		assert_eq!(distribution, (5_000_000, 4_082_500, 90_917_500, 1));
	});
}

#[test]
fn lock_in_vault_capital_selects_top_vaults_by_securitization() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxVaultsPerPool::set(2);
		CurrentFrameId::set(1);

		insert_vault(1, test_vault(11, MICROGONS_PER_ARGON as u64));
		insert_vault(2, test_vault(12, (2 * MICROGONS_PER_ARGON) as u64));
		insert_vault(3, test_vault(13, (3 * MICROGONS_PER_ARGON) as u64));

		set_argons(11, 10 * MICROGONS_PER_ARGON);
		set_argons(12, 10 * MICROGONS_PER_ARGON);
		set_argons(13, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(11), 1, 1, None));
		assert_ok!(Treasury::buy_bonds(origin(12), 2, 2, None));
		assert_ok!(Treasury::buy_bonds(origin(13), 3, 3, None));

		Treasury::lock_in_vault_capital(1);

		let current = CurrentFrameVaultCapital::<Test>::get().expect("current frame capital");
		let vaults = VaultsById::get();
		assert_eq!(vaults[&1].committed_microgons, MICROGONS_PER_ARGON);
		assert_eq!(vaults[&2].committed_microgons, 2 * MICROGONS_PER_ARGON);
		assert_eq!(vaults[&3].committed_microgons, 3 * MICROGONS_PER_ARGON);
		assert_eq!(current.frame_id, 1);
		assert_eq!(current.vault_securitization_positions.len(), 2);
		assert!(current.vault_securitization_positions.get(&1).is_none());
		assert_eq!(
			current
				.vault_securitization_positions
				.get(&2)
				.map(|vault| vault.active_bond_microgons),
			Some(2 * MICROGONS_PER_ARGON)
		);
		assert_eq!(
			current
				.vault_securitization_positions
				.get(&3)
				.map(|vault| vault.active_bond_microgons),
			Some(3 * MICROGONS_PER_ARGON)
		);
	});
}

#[test]
fn vault_taper_counts_only_reward_eligible_argonot_value() {
	for (participation, multiplier, held_argonots, expected_payout) in [
		(None, 2, 1_000, 25_246_049),
		(Some(0), 2, 1_000, 25_246_049),
		(Some(20), 0, 1_000, 25_246_049),
		(Some(20), 2, 0, 25_246_049),
		(Some(20), 2, 50, 25_492_099),
		(Some(20), 2, 100, 25_738_149),
		(Some(20), 2, 1_000, 25_738_149),
	] {
		new_test_ext().execute_with(|| {
			RewardTaperingPolicy::<Test>::kill();
			ArgonotSecuritizationMultiplier::set(multiplier);
			ArgonotParticipationFactor::<Test>::put(Percent::from_percent(
				participation.unwrap_or_default(),
			));
			AverageArgonotPriceInMicrogons::set(2 * MICROGONS_PER_ARGON);
			insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
			VaultArgonotMicronots::mutate(|backing| {
				backing.insert(1, held_argonots * MICROGONS_PER_ARGON);
			});
			set_target_securitization(100 * MICROGONS_PER_ARGON);
			Treasury::lock_in_vault_capital(1);
			if participation.is_none() {
				// Frames frozen before configurable participation have no companion snapshot.
				CurrentFrameArgonotParticipation::<Test>::kill();
			}

			// Later holdings and prices cannot change this frame's frozen capital basis.
			VaultArgonotMicronots::set(Default::default());
			AverageArgonotPriceInMicrogons::set(10 * MICROGONS_PER_ARGON);
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 1_000 * MICROGONS_PER_ARGON));
			assert_ok!(Treasury::distribute_bid_pool(1));

			assert_eq!(LastVaultProfits::get()[0].earnings, expected_payout);
			let reserves = RewardReserves::<Test>::get();
			assert_eq!(reserves.marketing, 100 * MICROGONS_PER_ARGON);
			assert_eq!(reserves.bitcoin_liquids, 0);
			assert_eq!(reserves.stabilizer_pools, 100 * MICROGONS_PER_ARGON);
			assert_eq!(reserves.liquidity_buys, 770 * MICROGONS_PER_ARGON - expected_payout);
		});
	}
}

#[test]
fn vault_taper_uses_each_vaults_own_capital_and_revenue() {
	new_test_ext().execute_with(|| {
		RewardTaperingPolicy::<Test>::kill();
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		insert_vault(2, test_vault(11, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 100 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 100, None));
		set_target_securitization(200 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 50 * MICROGONS_PER_ARGON));
		assert_ok!(Treasury::distribute_bid_pool(1));

		let profits = LastVaultProfits::get();
		// The higher-return vault cannot borrow the other vault's unused daily threshold.
		assert_eq!(profits[0].earnings, 1_266_674);
		assert_eq!(profits[1].earnings, 871_049);
	});
}

#[test]
fn splitting_vaults_does_not_increase_rewards() {
	new_test_ext().execute_with(|| {
		RewardTaperingPolicy::<Test>::kill();
		insert_vault(1, test_vault(10, (50 * MICROGONS_PER_ARGON) as u64));
		insert_vault(2, test_vault(11, (50 * MICROGONS_PER_ARGON) as u64));
		set_target_securitization(100 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		assert_ok!(Treasury::distribute_bid_pool(1));

		let total: u128 = LastVaultProfits::get().iter().map(|profit| profit.earnings).sum();
		// The same 100 ARGON in one vault earns 2,746,049; splitting rounds one microgon down.
		assert_eq!(total, 2_746_048);
	});
}

#[test]
fn vault_rewards_use_frame_cpi_instead_of_the_closing_price() {
	for (average_cpi, expected) in [
		(ArgonCPI::zero(), 2_746_049),
		(ArgonCPI::from_rational(1, 19), 2_670_545),
		(ArgonCPI::from_rational(1, 9), 2_595_041),
	] {
		new_test_ext().execute_with(|| {
			RewardTaperingPolicy::<Test>::kill();
			insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
			set_target_securitization(100 * MICROGONS_PER_ARGON);
			Treasury::lock_in_vault_capital(1);

			ArgonPriceInUsd::set(FixedU128::from_rational(8, 10));
			AverageArgonCpi::set(average_cpi);
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			assert_ok!(Treasury::distribute_bid_pool(1));
			assert_eq!(LastVaultProfits::get()[0].earnings, expected);
		});
	}
}

#[test]
fn vault_target_declines_below_price_target_while_stabilizer_rewards_stay_high() {
	new_test_ext().execute_with(|| {
		let policy = RewardTaperingPolicy::<Test>::get();
		let capital = 100 * MICROGONS_PER_ARGON;
		for (ratio, expected_vault_threshold) in [
			(1_100, 492_099),
			(1_000, 492_099),
			(975, 416_595),
			(950, 341_091),
			(925, 265_587),
			(900, 190_083),
			(800, 190_083),
		] {
			let price_ratio = Some(FixedU128::from_rational(ratio, 1_000));
			let vault_daily_return =
				Treasury::price_adjusted_daily_return(&policy.vault_revenue, price_ratio);
			assert_eq!(vault_daily_return.saturating_mul_int(capital), expected_vault_threshold);

			let stabilizer_daily_return =
				Treasury::price_adjusted_daily_return(&policy.stabilizer_pools, price_ratio);
			assert_eq!(stabilizer_daily_return.saturating_mul_int(capital), 492_099);
			assert_eq!(
				Treasury::tapered_payout(
					1_000_000,
					capital,
					stabilizer_daily_return,
					policy.stabilizer_pools.excess_reward_payout_percent,
				),
				746_049
			);
		}
	});
}

#[test]
fn vault_taper_sends_withheld_rewards_to_liquidity_buys() {
	new_test_ext().execute_with(|| {
		RewardTaperingPolicy::<Test>::kill();
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		assert_ok!(Treasury::distribute_bid_pool(1));

		let paid = LastVaultProfits::get()[0].earnings;
		let reserves = RewardReserves::<Test>::get();
		assert_eq!(paid, 2_524_604);
		assert_eq!(reserves.liquidity_buys, 74_475_396);
		assert_eq!(
			paid + reserves.marketing +
				reserves.bitcoin_liquids +
				3 * MICROGONS_PER_ARGON +
				reserves.stabilizer_pools +
				reserves.liquidity_buys,
			100 * MICROGONS_PER_ARGON
		);
	});
}

#[test]
fn upstream_participation_caps_each_asset_and_applies_the_configured_factor() {
	use argon_primitives::treasury::UpstreamPosition;
	for (factor, bitcoin, bonds, expected_basis_points) in [
		(Some(100), 0, 0, 0),
		(Some(100), 1_250, 1_250, 2_500),
		(Some(100), 2_500, 2_500, 5_000),
		(Some(100), 3_750, 3_750, 7_500),
		(Some(100), 5_000, 5_000, 10_000),
		(Some(100), 7_500, 7_500, 10_000),
		(Some(100), 5_000, 0, 5_000),
		(Some(100), 0, 5_000, 5_000),
		(Some(100), 10_000, 0, 5_000),
		(Some(100), 0, 10_000, 5_000),
		(Some(100), 10_000, 2_500, 7_500),
		(Some(100), 2_500, 10_000, 7_500),
		(None, 0, 0, 9_500),
		(None, 2_500, 2_500, 9_750),
		(None, 5_000, 5_000, 10_000),
		(Some(0), 0, 0, 10_000),
		(Some(0), 2_500, 2_500, 10_000),
		(Some(50), 0, 0, 5_000),
	] {
		new_test_ext().execute_with(|| {
			MinimumArgonsPerContributor::set(1);
			insert_vault(1, test_vault(1, (15_000 * MICROGONS_PER_ARGON) as u64));
			Upstreams::mutate(|upstreams| {
				upstreams.insert(account(2), (account(2), 1));
			});
			AvailableSecuritizationSpace::mutate(|space| {
				space.insert(1, 15_000 * MICROGONS_PER_ARGON);
			});
			TreasuryPositions::set_upstream_position(
				&account(2),
				Some(UpstreamPosition {
					vault_id: 1,
					bitcoin_securitization: bitcoin * MICROGONS_PER_ARGON,
					bitcoin_allocated_securitization: bitcoin * MICROGONS_PER_ARGON,
					..Default::default()
				}),
			);
			if bonds > 0 {
				set_argons(2, u128::from(bonds) * MICROGONS_PER_ARGON);
				assert_ok!(Treasury::buy_bonds(origin(2), 1, bonds, None));
			}
			if let Some(factor) = factor {
				assert_ok!(Treasury::configure_reward_economics(
					RuntimeOrigin::root(),
					None,
					Some(Percent::from_percent(factor)),
					None,
					None
				));
			}
			assert_eq!(
				Treasury::upstream_participation(&account(2)),
				FixedU128::from_rational(expected_basis_points, 10_000)
			);
			assert_eq!(Treasury::upstream_participation(&account(9)), FixedU128::one());
		});
	}
}

#[test]
fn upstream_nominal_targets_ignore_capacity_and_unfunded_reservations() {
	use crate::mock::{AvailableSecuritizationSpace, TreasuryPositions, Upstreams};
	use argon_primitives::treasury::{BitcoinLockPosition, TreasuryPositionProvider};
	new_test_ext().execute_with(|| {
		UpstreamParticipationFactor::<Test>::put(Percent::from_percent(100));
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(1, (5_000 * MICROGONS_PER_ARGON) as u64));
		Upstreams::mutate(|upstreams| {
			upstreams.insert(account(2), (account(2), 1));
		});
		AvailableSecuritizationSpace::mutate(|space| {
			space.insert(1, 0);
		});
		set_argons(2, 2_500 * MICROGONS_PER_ARGON);
		set_argons(3, 2_500 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(3), 1, 2_500, None));
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 2_500, None));

		// Exhausted capacity leaves the nominal Bitcoin target unmet.
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::from_rational(1, 4));
		// Reserving the full Bitcoin target adds no funded credit.
		assert_ok!(TreasuryPositions::bitcoin_position_updated(
			&account(2),
			1,
			BitcoinLockPosition::default(),
			BitcoinLockPosition {
				allocated_securitization: 5_000 * MICROGONS_PER_ARGON,
				..Default::default()
			},
		));
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::from_rational(1, 4));
		assert_ok!(TreasuryPositions::bitcoin_position_updated(
			&account(2),
			1,
			BitcoinLockPosition {
				allocated_securitization: 5_000 * MICROGONS_PER_ARGON,
				..Default::default()
			},
			BitcoinLockPosition {
				activated_securitization: 2_500 * MICROGONS_PER_ARGON,
				allocated_securitization: 5_000 * MICROGONS_PER_ARGON,
			},
		));
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::from_rational(1, 2));
		// Releasing the unfunded reservation cannot change funded participation.
		assert_ok!(TreasuryPositions::bitcoin_position_updated(
			&account(2),
			1,
			BitcoinLockPosition {
				activated_securitization: 2_500 * MICROGONS_PER_ARGON,
				allocated_securitization: 5_000 * MICROGONS_PER_ARGON,
			},
			BitcoinLockPosition {
				activated_securitization: 2_500 * MICROGONS_PER_ARGON,
				allocated_securitization: 2_500 * MICROGONS_PER_ARGON,
			},
		));
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::from_rational(1, 2));
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			Some(Percent::from_percent(5)),
			None,
			None,
		));
		assert_eq!(
			Treasury::upstream_participation(&account(2)),
			FixedU128::from_rational(975, 1_000)
		);
		UpstreamParticipationFactor::<Test>::put(Percent::from_percent(100));

		// Capacity and upstream lifecycle changes cannot forgive either shortfall.
		AvailableSecuritizationSpace::mutate(|space| {
			space.insert(1, 10_000 * MICROGONS_PER_ARGON);
		});
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::from_rational(1, 2));
		VaultsById::mutate(|vaults| vaults.get_mut(&1).unwrap().is_closed = true);
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::from_rational(1, 2));
		VaultsById::mutate(|vaults| {
			let upstream = vaults.get_mut(&1).unwrap();
			upstream.is_closed = false;
			upstream.activated_securitization = 0;
		});
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::from_rational(1, 2));
		VaultsById::mutate(|vaults| vaults.remove(&1));
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::from_rational(1, 2));
		assert_eq!(Treasury::upstream_participation(&account(9)), FixedU128::one());

		// A position for another vault cannot satisfy the configured relationship.
		Upstreams::mutate(|upstreams| {
			upstreams.insert(account(2), (account(2), 2));
		});
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::zero());
	});
}

#[test]
fn upstream_nominal_targets_preserve_individual_zero_target_credit() {
	use crate::mock::{UpstreamBitcoinTarget, UpstreamBondTarget};

	for (bitcoin_target, bond_target, expected) in [
		(0, 5_000, FixedU128::from_rational(1, 2)),
		(5_000, 0, FixedU128::from_rational(1, 2)),
		(0, 0, FixedU128::one()),
	] {
		new_test_ext().execute_with(|| {
			UpstreamParticipationFactor::<Test>::put(Percent::from_percent(100));
			UpstreamBitcoinTarget::set(bitcoin_target * MICROGONS_PER_ARGON);
			UpstreamBondTarget::set(bond_target * MICROGONS_PER_ARGON);
			Upstreams::mutate(|upstreams| {
				upstreams.insert(account(2), (account(2), 1));
			});

			assert_eq!(Treasury::upstream_participation(&account(2)), expected);
		});
	}
}

#[test]
fn upstream_bond_requirements_keep_nominal_targets_below_the_purchase_minimum() {
	use crate::mock::{AvailableSecuritizationSpace, TreasuryPositions, Upstreams};
	use argon_primitives::treasury::{TreasuryPositionProvider, UpstreamPosition};
	for (minimum, retained, expected) in [
		(100, 0, FixedU128::from_rational(1, 10)),
		(100, 50, FixedU128::from_rational(105, 1_000)),
		(10, 0, FixedU128::from_rational(1, 10)),
		(10, 50, FixedU128::from_rational(105, 1_000)),
	] {
		new_test_ext().execute_with(|| {
			UpstreamParticipationFactor::<Test>::put(Percent::from_percent(100));
			MinimumArgonsPerContributor::set(minimum * MICROGONS_PER_ARGON);
			insert_vault(1, test_vault(1, (1_000 * MICROGONS_PER_ARGON) as u64));
			insert_vault(2, test_vault(2, (1_000 * MICROGONS_PER_ARGON) as u64));
			Upstreams::mutate(|upstreams| {
				upstreams.insert(account(2), (account(2), 1));
			});
			AvailableSecuritizationSpace::mutate(|space| {
				space.insert(1, 0);
			});
			TreasuryPositions::set_upstream_position(
				&account(2),
				Some(UpstreamPosition {
					vault_id: 1,
					bitcoin_securitization: 1_000 * MICROGONS_PER_ARGON,
					bitcoin_allocated_securitization: 1_000 * MICROGONS_PER_ARGON,
					..Default::default()
				}),
			);
			for owner in [2, 3, 4] {
				set_argons(owner, 1_000 * MICROGONS_PER_ARGON);
			}
			if retained > 0 {
				assert_ok!(Treasury::buy_bonds(origin(2), 1, 100, None));
				// Burning part of a valid purchase can leave principal below the new-purchase
				// minimum. Only the retained principal counts toward the nominal target.
				assert_ok!(Treasury::encumber_bond_microgons(
					&account(2),
					50 * MICROGONS_PER_ARGON,
					None,
				));
				assert_ok!(Treasury::settle_bond_collateral_burn(
					&account(2),
					50 * MICROGONS_PER_ARGON,
					&mut Weight::zero()
				));
			}
			assert_ok!(Treasury::buy_bonds(origin(3), 1, 500, None));
			assert_ok!(Treasury::buy_bonds(origin(4), 1, 450 - retained, None));
			assert_eq!(Treasury::upstream_participation(&account(2)), expected);
			Treasury::lock_in_vault_capital(1);

			if minimum == 100 {
				assert_noop!(
					Treasury::buy_bonds(origin(2), 1, 50, None),
					Error::<Test>::BondPurchaseBelowMinimum
				);
				assert_noop!(
					Treasury::buy_bonds(origin(2), 1, 100, None),
					Error::<Test>::InsufficientBondSpace
				);
			} else {
				assert_ok!(Treasury::buy_bonds(origin(2), 1, 50, None));
				assert_eq!(
					Treasury::upstream_participation(&account(2)),
					expected.saturating_add(FixedU128::from_rational(5, 1_000))
				);
			}
			assert_eq!(
				CurrentFrameVaultCapital::<Test>::get().unwrap().vault_securitization_positions[&2]
					.upstream_participation,
				expected,
			);
		});
	}
}

#[test]
fn upstream_bond_principal_is_live_but_the_earnings_multiplier_is_frozen_for_the_frame() {
	use crate::mock::{AvailableSecuritizationSpace, TreasuryPositions, Upstreams};
	use argon_primitives::treasury::{TreasuryPositionProvider, UpstreamPosition};
	new_test_ext().execute_with(|| {
		UpstreamParticipationFactor::<Test>::put(Percent::from_percent(100));
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(1, (2_000 * MICROGONS_PER_ARGON) as u64));
		insert_vault(2, test_vault(2, (1_000 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 1_000 * MICROGONS_PER_ARGON);
		Upstreams::mutate(|upstreams| {
			upstreams.insert(account(2), (account(2), 1));
		});
		AvailableSecuritizationSpace::mutate(|space| {
			space.insert(1, 0);
		});
		TreasuryPositions::set_upstream_position(
			&account(2),
			Some(UpstreamPosition { vault_id: 1, ..Default::default() }),
		);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 1_000, None));
		assert_eq!(TreasuryPositions::bond_principal(&account(2)), 1_000 * MICROGONS_PER_ARGON);
		Treasury::lock_in_vault_capital(1);
		let frozen = CurrentFrameVaultCapital::<Test>::get().unwrap();
		assert_eq!(
			frozen.vault_securitization_positions[&2].upstream_participation,
			FixedU128::from_rational(1, 10)
		);
		assert_ok!(Treasury::liquidate_bond_lot(origin(2), account_bond_lot_ids(2)[0]));
		assert_eq!(TreasuryPositions::bond_principal(&account(2)), 0);
		assert_eq!(TreasuryPositions::upstream_position(&account(2)).unwrap().bond_principal, 0);
		assert_ok!(Treasury::configure_reward_economics(
			RuntimeOrigin::root(),
			None,
			Some(Percent::from_percent(5)),
			None,
			None,
		));
		assert_eq!(
			CurrentFrameVaultCapital::<Test>::get().unwrap().vault_securitization_positions[&2]
				.upstream_participation,
			FixedU128::from_rational(1, 10)
		);
		Treasury::lock_in_vault_capital(2);
		assert_eq!(
			CurrentFrameVaultCapital::<Test>::get().unwrap().vault_securitization_positions[&2]
				.upstream_participation,
			FixedU128::from_rational(95, 100)
		);
	});
}

#[test]
fn upstream_reducer_preserves_the_earnings_floor_and_only_scales_the_uplift() {
	for (participation, expected_earnings) in
		[(FixedU128::zero(), 5), (FixedU128::from_rational(1, 2), 28), (FixedU128::one(), 51)]
	{
		new_test_ext().execute_with(|| {
			CurrentFrameId::set(0);
			RewardReserves::<Test>::mutate(|reserves| {
				reserves.marketing = ExistentialDeposit::get()
			});
			assert_ok!(TreasuryPositions::prepare_collect());
			MinimumArgonsPerContributor::set(1);
			let mut vault = test_vault(10, (10 * MICROGONS_PER_ARGON) as u64);
			vault.activated_securitization = 10 * MICROGONS_PER_ARGON;
			insert_vault(1, vault);
			VaultBitcoinSatoshis::mutate(|bitcoin| {
				bitcoin.insert(1, 100_000_000);
			});
			set_vault_securitized_satoshis(1, 100_000_000);
			VaultArgonotMicronots::mutate(|backing| {
				backing.insert(1, 20 * MICROGONS_PER_ARGON);
			});
			set_argons(2, 10 * MICROGONS_PER_ARGON);
			assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));
			set_target_securitization(10 * MICROGONS_PER_ARGON);
			CurrentFrameId::set(1);
			Treasury::lock_in_vault_capital(1);
			CurrentFrameVaultCapital::<Test>::mutate(|capital| {
				capital
					.as_mut()
					.unwrap()
					.vault_securitization_positions
					.get_mut(&1)
					.unwrap()
					.upstream_participation = participation;
			});
			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			collect_frame(1, &[2]);
			let earnings = LastVaultProfits::get().last().unwrap().earnings_for_vault;
			assert!(
				earnings.abs_diff(expected_earnings * MICROGONS_PER_ARGON) <= 1,
				"score {participation:?}: {earnings} vs {expected_earnings}"
			);
			assert_eq!(
				PositionsByAccount::<Test>::get(account(2)).unwrap().collected.bond_earnings,
				5000000
			);
		});
	}
}

#[test]
fn upstream_principal_includes_displaced_bonds_and_tracks_partial_and_full_burns() {
	use argon_primitives::treasury::{TreasuryPositionProvider, UpstreamPosition};
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(2, (1_000 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 1_000 * MICROGONS_PER_ARGON);
		set_argons(3, 1_000 * MICROGONS_PER_ARGON);
		TreasuryPositions::set_upstream_position(
			&account(2),
			Some(UpstreamPosition { vault_id: 1, ..Default::default() }),
		);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 1_000, None));
		assert_ok!(Treasury::set_bond_lot_flexible(origin(2), account_bond_lot_ids(2)[0], true));
		assert_ok!(Treasury::buy_bonds(origin(3), 1, 1_000, None));
		assert_eq!(BondLotsByVault::<Test>::get(1).displaced_flexible_bonds, 1_000);
		assert_eq!(
			TreasuryPositions::upstream_position(&account(2)).unwrap().bond_principal,
			1_000 * MICROGONS_PER_ARGON
		);
		assert_ok!(Treasury::encumber_bond_microgons(
			&account(2),
			1_000 * MICROGONS_PER_ARGON,
			None,
		));
		assert_ok!(Treasury::settle_bond_collateral_burn(
			&account(2),
			333_500_000,
			&mut Weight::zero()
		));
		assert_eq!(TreasuryPositions::bond_principal(&account(2)), 666 * MICROGONS_PER_ARGON);
		assert_eq!(
			TreasuryPositions::upstream_position(&account(2)).unwrap().bond_principal,
			666 * MICROGONS_PER_ARGON
		);
		assert_ok!(Treasury::settle_bond_collateral_burn(
			&account(2),
			666_500_000,
			&mut Weight::zero()
		));
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(TreasuryPositions::bond_principal(&account(2)), 0);
		assert_eq!(TreasuryPositions::upstream_position(&account(2)).unwrap().bond_principal, 0);
	});
}

#[test]
fn an_invite_predating_its_upstream_vault_binds_on_the_first_bond_purchase() {
	use crate::mock::{TreasuryPositions, Upstreams};
	use argon_primitives::treasury::TreasuryPositionProvider;
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		assert_eq!(TreasuryPositions::upstream_position(&account(2)), None);
		// The operational relationship becomes resolvable when its upstream creates a vault.
		insert_vault(1, test_vault(1, (1_000 * MICROGONS_PER_ARGON) as u64));
		Upstreams::mutate(|upstreams| {
			upstreams.insert(account(2), (account(2), 1));
		});
		set_argons(2, 100 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 100, None));
		let position = TreasuryPositions::upstream_position(&account(2)).unwrap();
		assert_eq!(position.vault_id, 1);
		assert_eq!(position.bond_principal, 100 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::liquidate_bond_lot(origin(2), account_bond_lot_ids(2)[0]));
		assert_eq!(TreasuryPositions::upstream_position(&account(2)).unwrap().bond_principal, 0);
	});
}
