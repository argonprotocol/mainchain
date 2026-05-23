use super::*;

#[test]
fn on_initialize_expires_recent_transfers() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentTick::set(1);
		CrosschainTransfer::on_initialize(System::block_number());

		let recipient = account(7);
		let expires_at = CurrentTick::get() + RecentTransferRetentionTicks::get();
		RecentArgonTransfersByAccount::<Test>::insert(&recipient, 1);
		InboundTransfersExpiringAt::<Test>::append(expires_at, recipient.clone());

		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&recipient), 1);
		assert_eq!(InboundTransfersExpiringAt::<Test>::get(expires_at).len(), 1);

		CurrentTick::set(expires_at + 2);
		System::set_block_number(2);
		CrosschainTransfer::on_initialize(System::block_number());

		assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&recipient), 0);
		assert!(InboundTransfersExpiringAt::<Test>::get(expires_at).is_empty());
	});
}

#[test]
fn migration_moves_legacy_balances_and_refunds_ready_cases() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let legacy_account = legacy_token_gateway_account();
		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);

		let _ = InitializeCrosschainTransferMigration::<Test>::on_runtime_upgrade();

		assert_eq!(Balances::balance(&legacy_account), 0);
		assert_eq!(Ownership::balance(&legacy_account), 0);

		assert_eq!(Balances::balance(&burn_account), 1_928_409);
		assert_eq!(Ownership::balance(&burn_account), 299_993);

		assert_eq!(Balances::balance(&launch_era_argon_refund_account()), 1_000_001);
		assert_eq!(Balances::balance(&small_launch_era_argon_refund_account()), 2_000);
		assert_eq!(Balances::balance(&post_hack_argon_refund_account()), 197_069_590);
		assert_eq!(Ownership::balance(&ready_argonot_refund_account()), 200_000);
	});
}
