use super::*;
use frame_support::traits::tokens::Imbalance;

#[test]
fn vaults_can_collect_revenue() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		CurrentFrameId::set(1);

		set_argons(1, 1_000_000);
		let mut terms = default_terms(FixedU128::from_float(0.01));
		terms.bitcoin_base_fee = 1000;
		assert_ok!(Vaults::create(
			RuntimeOrigin::signed(1),
			VaultConfig {
				terms,
				delegate_account_id: None,
				bitcoin_xpubkey: keys(),
				securitization: 500_000,
			}
		));
		assert_eq!(Balances::free_balance(1), 500_000);

		CurrentFrameId::set(2);
		Vaults::on_frame_start(2);
		RevenuePerFrameByVault::<Test>::get(1);
		assert_eq!(RevenuePerFrameByVault::<Test>::get(1).len(), 0);
		assert_eq!(RevenuePerFrameByVaultCount::<Test>::get(), 0);

		set_argons(2, 6_000);
		let (fee, _) = Vaults::reserve_securitization(
			1,
			&2,
			&securitization(500_000),
			standard_reservation_request(),
		)
		.expect("bonding failed");

		let current_frame_id = CurrentFrameId::get();
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 1);
		assert_eq!(vault_revenue[0].frame_id, current_frame_id);
		assert_eq!(vault_revenue[0].bitcoin_lock_fee_revenue, fee);
		assert_eq!(vault_revenue[0].bitcoin_locks_new_securitization, 500_000);
		assert_eq!(vault_revenue[0].bitcoin_locks_added_satoshis, 0);
		assert_eq!(vault_revenue[0].bitcoin_locks_created, 1);
		assert_eq!(vault_revenue[0].uncollected_revenue, fee);
		assert_eq!(vault_revenue[0].securitization, 500_000);
		assert_eq!(vault_revenue[0].securitization_activated, 0); // funds are still pending
		assert_eq!(vault_earnings(1), fee);

		// set up a mining bid pallet account. 10k is "already distributed"
		let vault_lp_earnings = 40_000;
		set_argons(100, vault_lp_earnings);
		let mut funding = <Balances as Balanced<_>>::withdraw(
			&100,
			vault_lp_earnings,
			Precision::Exact,
			Preservation::Expendable,
			Fortitude::Polite,
		)
		.unwrap();
		let profit = VaultTreasuryFrameEarnings {
			vault_id: 1,
			vault_operator_account_id: 1,
			frame_id: current_frame_id,
			earnings: 50_000,
			capital_contributed: 100_000,
			earnings_for_vault: vault_lp_earnings,
			capital_contributed_by_vault: 10_000,
		};
		let issuance_before = Balances::total_issuance();
		let custody_before = Balances::balance(&TreasuryPositions::account_id());
		assert_err!(
			Vaults::record_vault_frame_earnings(
				&mut funding,
				VaultTreasuryFrameEarnings {
					earnings_for_vault: vault_lp_earnings + 1,
					..profit.clone()
				}
			),
			TokenError::FundsUnavailable
		);
		assert_eq!(funding.peek(), vault_lp_earnings);
		assert_eq!(Balances::total_issuance(), issuance_before);
		assert_eq!(RevenuePerFrameByVault::<Test>::get(1).to_vec(), vault_revenue);
		assert_eq!(vault_earnings(1), fee);

		assert_ok!(Vaults::record_vault_frame_earnings(&mut funding, profit));
		assert_eq!(funding.peek(), 0);
		assert_eq!(Balances::total_issuance(), issuance_before);
		assert_eq!(
			Balances::balance(&TreasuryPositions::account_id()),
			custody_before + vault_lp_earnings
		);
		assert_eq!(Balances::free_balance(100), 0);
		assert_eq!(vault_earnings(1), fee + vault_lp_earnings);
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 1);
		assert_eq!(vault_revenue[0].frame_id, current_frame_id);
		assert_eq!(vault_revenue[0].treasury_vault_earnings, vault_lp_earnings);
		assert_eq!(vault_revenue[0].treasury_total_earnings, 50_000);
		assert_eq!(vault_revenue[0].treasury_vault_capital, 10_000);
		assert_eq!(vault_revenue[0].treasury_external_capital, 90_000);
		assert_eq!(vault_revenue[0].uncollected_revenue, fee + vault_lp_earnings);

		assert!(LastCollectFrameByVaultId::<Test>::get(1).is_none());
		let other_wallet = Balances::balance(&2);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(2)));
		assert_eq!(Balances::balance(&2), other_wallet);
		assert_eq!(vault_earnings(1), fee + vault_lp_earnings);
		// test that you can only collect if you have no pending cosigns
		Vaults::update_pending_cosign(1, 0, CosignRequest::Lock(1), false);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_eq!(vault_earnings(1), fee + vault_lp_earnings);
		Vaults::update_pending_cosign(1, 0, CosignRequest::Lock(1), true);
		let orphan = CosignRequest::Orphan {
			owner: 2,
			utxo_ref: UtxoRef { txid: H256Le([1; 32]), output_index: 0 },
		};
		Vaults::update_pending_cosign(1, 0, orphan.clone(), false);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_eq!(vault_earnings(1), fee + vault_lp_earnings);
		Vaults::update_pending_cosign(1, 0, orphan, true);
		OverdueCollectBlockers::mutate(|entries| {
			entries.insert(1);
		});
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_eq!(vault_earnings(1), fee + vault_lp_earnings);
		OverdueCollectBlockers::mutate(|entries| {
			entries.remove(&1);
		});

		let providers_before_collect = System::providers(&1);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_eq!(System::providers(&1), providers_before_collect);
		assert_eq!(Balances::free_balance(1), 1_000_000 - 500_000 + fee + vault_lp_earnings);
		assert_eq!(Balances::free_balance(100), 0);
		assert_eq!(LastCollectFrameByVaultId::<Test>::get(1), Some(CurrentFrameId::get()));

		VaultsById::<Test>::mutate(1, |v| {
			if let Some(v) = v {
				v.securitization_pending_activation = 0;
			}
		});
		CurrentFrameId::set(3);
		Vaults::on_frame_start(3);
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 1);
		assert_eq!(vault_revenue[0].frame_id, 2);
		assert_eq!(vault_revenue[0].uncollected_revenue, 0);
		assert_eq!(vault_revenue[0].securitization_activated, 500_000);

		// make sure order is correct if we get another entry

		let vault_lp_earnings = 20_000;
		set_argons(100, vault_lp_earnings);
		assert_eq!(CurrentFrameId::get(), 3);
		assert_ok!(Vaults::record_vault_frame_earnings(
			&mut <Balances as Balanced<_>>::withdraw(
				&100,
				vault_lp_earnings,
				Precision::Exact,
				Preservation::Expendable,
				Fortitude::Polite,
			)
			.unwrap(),
			VaultTreasuryFrameEarnings {
				vault_id: 1,
				vault_operator_account_id: 1,
				frame_id: CurrentFrameId::get(),
				earnings: 50_000,
				capital_contributed: 100_000,
				earnings_for_vault: vault_lp_earnings,
				capital_contributed_by_vault: 10_000,
			},
		));
		let vault_revenue = RevenuePerFrameByVault::<Test>::get(1).to_vec();
		assert_eq!(vault_revenue.len(), 2);
		assert_eq!(vault_revenue[0].frame_id, 3);
		assert_eq!(vault_revenue[0].uncollected_revenue, vault_lp_earnings);
		assert_eq!(vault_revenue[1].frame_id, 2);
		assert_eq!(vault_revenue[1].uncollected_revenue, 0);
	});
}

#[test]
fn shared_collect_skips_blocked_vault_earnings_and_expires_only_their_backing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 1_000_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		record_vault_earnings(1, 100);
		record_vault_earnings(2, 200);
		assert_ok!(TreasuryPositions::fund(CollectSource::Bonds, 50));
		assert_ok!(TreasuryPositions::accrue(&1, CollectSource::Bonds, 50));
		let wallet = Balances::balance(&1);
		let issuance = Balances::total_issuance();

		OverdueCollectBlockers::mutate(|accounts| {
			accounts.insert(1);
		});
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_eq!(Balances::balance(&1), wallet + 50);
		assert_eq!(vault_earnings(1), 300);
		assert_eq!(RevenuePerFrameByVault::<Test>::get(1).len(), 2);
		assert_eq!(LastCollectFrameByVaultId::<Test>::get(1), None);

		CurrentFrameId::set(11);
		Vaults::on_frame_start(11);
		Vaults::on_frame_start(11);
		assert_eq!(vault_earnings(1), 200);
		assert_eq!(Balances::total_issuance(), issuance - 100);
		let totals = pallet_treasury_positions::CollectTotals::<Test>::get();
		System::assert_has_event(
			Event::<Test>::VaultRevenueUncollected { vault_id: 1, frame_id: 1, amount: 100 }.into(),
		);
		assert_eq!(totals.expired_vault_earnings, 100);
		assert_eq!(totals.collected.bond_earnings, 50);
		let revenue = RevenuePerFrameByVault::<Test>::get(1);
		assert_eq!(revenue.len(), 1);
		assert_eq!(revenue[0].frame_id, 2);

		OverdueCollectBlockers::mutate(|accounts| {
			accounts.remove(&1);
		});
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_eq!(Balances::balance(&1), wallet + 250);
		assert_eq!(vault_earnings(1), 0);
		assert_eq!(RevenuePerFrameByVault::<Test>::get(1)[0].uncollected_revenue, 0);
		assert_eq!(Balances::balance(&TreasuryPositions::account_id()), ExistentialDeposit::get());
		assert_eq!(Balances::total_issuance(), issuance - 100);
	});
}

#[test]
fn failed_payment_and_expiry_preserve_both_vault_ledgers_for_retry() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 1_000_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		record_vault_earnings(0, 100);
		CurrentFrameId::set(1);
		Vaults::on_frame_start(1);
		assert_eq!(vault_earnings(1), 100);
		assert_eq!(RevenuePerFrameByVault::<Test>::get(1)[0].frame_id, 0);
		VaultIdByOperator::<Test>::remove(1);
		assert_noop!(
			TreasuryPositions::collect(RuntimeOrigin::signed(1)),
			Error::<Test>::VaultNotFound
		);
		VaultIdByOperator::<Test>::insert(1, 1);
		let custody = TreasuryPositions::account_id();
		let backing = Balances::balance(&custody);
		assert_ok!(Balances::force_set_balance(
			RuntimeOrigin::root(),
			custody,
			ExistentialDeposit::get()
		));

		assert!(TreasuryPositions::collect(RuntimeOrigin::signed(1)).is_err());
		assert_eq!(LastCollectFrameByVaultId::<Test>::get(1), None);
		assert_eq!(RevenuePerFrameByVault::<Test>::get(1)[0].uncollected_revenue, 100);
		assert_eq!(vault_earnings(1), 100);
		CurrentFrameId::set(10);
		Vaults::on_frame_start(10);
		assert_eq!(RevenuePerFrameByVault::<Test>::get(1)[0].uncollected_revenue, 100);
		assert_eq!(vault_earnings(1), 100);
		assert_eq!(
			pallet_treasury_positions::CollectTotals::<Test>::get().expired_vault_earnings,
			0
		);

		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), custody, backing));
		let issuance = Balances::total_issuance();
		Vaults::on_frame_start(10);
		Vaults::on_frame_start(10);
		assert!(RevenuePerFrameByVault::<Test>::get(1).is_empty());
		assert_eq!(RevenuePerFrameByVaultCount::<Test>::get(), 0);
		assert_eq!(vault_earnings(1), 0);
		assert_eq!(Balances::total_issuance(), issuance - 100);
		assert_eq!(
			pallet_treasury_positions::CollectTotals::<Test>::get().expired_vault_earnings,
			100
		);
	});
}

#[test]
fn oldest_signing_request_blocks_only_from_its_collect_due_frame() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		set_argons(1, 1_000_000);
		assert_ok!(Vaults::create(RuntimeOrigin::signed(1), default_vault()));
		record_vault_earnings(1, 100);
		let orphan = CosignRequest::Orphan {
			owner: 2,
			utxo_ref: UtxoRef { txid: H256Le([1; 32]), output_index: 0 },
		};
		Vaults::update_pending_cosign(1, 2, orphan.clone(), false);
		let wallet = Balances::balance(&1);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_eq!(Balances::balance(&1), wallet + 100);

		record_vault_earnings(2, 100);
		Vaults::update_pending_cosign(1, 3, CosignRequest::Lock(1), false);
		assert_eq!(
			PendingCosignsByVault::<Test>::iter_key_prefix(1).next(),
			Some((2u64.to_be_bytes(), orphan.clone()))
		);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_eq!(vault_earnings(1), 100);
		Vaults::update_pending_cosign(1, 2, orphan, true);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_eq!(Balances::balance(&1), wallet + 200);

		record_vault_earnings(3, 100);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_eq!(vault_earnings(1), 100);
		Vaults::update_pending_cosign(1, 3, CosignRequest::Lock(1), true);
		Vaults::update_pending_cosign(1, 3, CosignRequest::Lock(1), true);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(1)));
		assert_eq!(Balances::balance(&1), wallet + 300);
		assert!(PendingCosignsByVault::<Test>::iter_key_prefix(1).next().is_none());
	});
}
