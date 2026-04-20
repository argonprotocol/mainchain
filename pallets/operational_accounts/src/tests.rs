use crate::{
	AccountOwnershipProof, ConsumedReferralCodes, ConsumedReferralCodesByExpiration,
	EncryptedServerBySponsee, ExpiredReferralCodeCleanupFrame,
	MINING_BOT_ACCOUNT_PROOF_MESSAGE_KEY, MINING_FUNDING_ACCOUNT_PROOF_MESSAGE_KEY,
	OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY, OpaqueEncryptionPubkey, OperationalAccountBySubAccount,
	OperationalAccounts, OperationalProgressPatch, REFERRAL_CLAIM_PROOF_MESSAGE_KEY,
	REFERRAL_SPONSOR_GRANT_MESSAGE_KEY, ReferralProof, Registration, RegistrationV1, Rewards,
	VAULT_ACCOUNT_PROOF_MESSAGE_KEY,
};
use argon_primitives::{MICROGONS_PER_ARGON, OperationalAccountsHook, Signature};
use frame_support::{assert_err, assert_noop, assert_ok, traits::Hooks};
use pallet_prelude::*;
use sp_core::{Pair, sr25519};
use sp_io::hashing::blake2_256;
use sp_runtime::{AccountId32, DispatchError, MultiSigner, traits::IdentifyAccount};

use crate::mock::{
	BitcoinLockSizeForReferral, ClaimableTreasuryBalance, ClaimedOperationalRewards,
	CurrentFrameId, MaxAvailableReferrals, MaxEncryptedServerLen,
	OperationalAccounts as OperationalAccountsPallet, OperationalMinimumVaultSecuritization,
	OperationalReferralBonusReward, OperationalReferralReward, RequiresUniswapTransfer,
	RuntimeOrigin, Test, TestAccountId, ensure_registration_lookup, has_vault_operational_mark,
	new_test_ext, set_registration_lookup,
};

#[test]
fn test_register_creates_operational_account() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(1, 2, 3, 4);
		register_account(&account_set, None);

		let operational_account = OperationalAccounts::<Test>::get(&account_set.owner)
			.expect("operational account created");
		assert_eq!(operational_account.vault_account, account_set.vault);
		assert_eq!(operational_account.mining_funding_account, account_set.mining_funding);
		assert_eq!(operational_account.mining_bot_account, account_set.mining_bot);
		assert_eq!(operational_account.encryption_pubkey, account_set.encryption_pubkey);
		assert!(operational_account.sponsor.is_none());

		assert_eq!(
			OperationalAccountBySubAccount::<Test>::get(&account_set.vault),
			Some(account_set.owner.clone())
		);
		assert_eq!(
			OperationalAccountBySubAccount::<Test>::get(&account_set.mining_funding),
			Some(account_set.owner.clone())
		);
		assert_eq!(
			OperationalAccountBySubAccount::<Test>::get(&account_set.mining_bot),
			Some(account_set.owner.clone())
		);
	});
}

#[test]
fn test_register_rejects_duplicate_owner() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(1, 2, 3, 4);
		register_account(&account_set, None);
		let duplicate_set = make_account_set(1, 5, 6, 7);

		assert_err!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(duplicate_set.owner.clone()),
				duplicate_set.registration(None),
			),
			crate::Error::<Test>::AlreadyRegistered
		);
	});
}

#[test]
fn test_register_rejects_duplicate_subaccounts() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(1, 2, 3, 4);
		register_account(&account_set, None);

		let duplicate_vault = make_account_set(2, 2, 5, 6);
		assert_err!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(duplicate_vault.owner.clone()),
				duplicate_vault.registration(None),
			),
			crate::Error::<Test>::AccountAlreadyLinked
		);
		let duplicate_funding = make_account_set(3, 7, 3, 8);
		assert_err!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(duplicate_funding.owner.clone()),
				duplicate_funding.registration(None),
			),
			crate::Error::<Test>::AccountAlreadyLinked
		);
		let duplicate_bot = make_account_set(4, 9, 10, 4);
		assert_err!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(duplicate_bot.owner.clone()),
				duplicate_bot.registration(None),
			),
			crate::Error::<Test>::AccountAlreadyLinked
		);
	});
}

#[test]
fn test_register_rejects_owner_already_linked_as_subaccount() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(1, 2, 3, 4);
		register_account(&account_set, None);

		let duplicate_owner = make_account_set(5, 6, 7, 8);
		OperationalAccountBySubAccount::<Test>::insert(&duplicate_owner.owner, &account_set.owner);

		assert_err!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(duplicate_owner.owner.clone()),
				duplicate_owner.registration(None),
			),
			crate::Error::<Test>::AccountAlreadyLinked
		);
	});
}

#[test]
fn test_register_allows_linked_account_submitter() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(11, 12, 13, 14);

		register_account_with_submitter(&account_set, &account_set.vault, None);

		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.vault_account, account_set.vault);
	});
}

#[test]
fn test_register_rejects_outsider_submitter() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(15, 16, 17, 18);

		assert_err!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(account_id_from_seed(19)),
				account_set.registration(None),
			),
			crate::Error::<Test>::InvalidRegistrationSubmitter
		);
	});
}

#[test]
fn test_register_rejects_invalid_referral_proof_and_ignores_sponsor_without_capacity() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(10, 11, 12, 13);
		register_account(&sponsor_set, None);
		set_available_referrals(&sponsor_set.owner, 1);

		#[cfg(not(feature = "runtime-benchmarks"))]
		{
			let invalid_signature_set = make_account_set(20, 21, 22, 23);
			let invalid_signature =
				make_referral_proof(&invalid_signature_set.owner, &sponsor_set.owner, 1, 11);
			assert_noop!(
				OperationalAccountsPallet::register(
					RuntimeOrigin::signed(invalid_signature_set.owner.clone()),
					invalid_signature_set.registration(Some(invalid_signature)),
				),
				crate::Error::<Test>::InvalidReferralProof
			);
		}

		let expired_set = make_account_set(30, 31, 32, 33);
		let expired = make_referral_proof_expiring_at(
			&expired_set.owner,
			&sponsor_set.owner,
			2,
			10,
			CurrentFrameId::get(),
		);
		assert_noop!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(expired_set.owner.clone()),
				expired_set.registration(Some(expired)),
			),
			crate::Error::<Test>::ReferralProofExpired
		);

		let no_referrals_set = make_account_set(40, 41, 42, 43);
		let no_referrals = make_referral_proof(&no_referrals_set.owner, &sponsor_set.owner, 2, 10);
		set_available_referrals(&sponsor_set.owner, 0);
		register_account(&no_referrals_set, Some(no_referrals));

		let account = OperationalAccounts::<Test>::get(&no_referrals_set.owner).expect("account");
		assert!(account.sponsor.is_none());
		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.available_referrals, 0);
	});
}

#[test]
fn test_register_hydrates_recent_argon_transfer_on_linked_account() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(24, 25, 26, 27);
		pallet_inbound_transfer_log::RecentArgonTransfersByAccount::<Test>::insert(
			account_set.mining_funding.clone(),
			1,
		);

		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.has_uniswap_transfer);
	});
}

#[test]
fn test_register_marks_uniswap_transfer_satisfied_when_uniswap_is_not_required() {
	new_test_ext().execute_with(|| {
		RequiresUniswapTransfer::set(false);
		let account_set = make_account_set(28, 29, 30, 31);

		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.has_uniswap_transfer);
	});
}

#[test]
fn test_force_set_progress_guardrails() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(1, 2, 3, 4);
		register_account(&account_set, None);
		let non_empty_patch = OperationalProgressPatch {
			has_uniswap_transfer: Some(true),
			vault_created: None,
			has_treasury_pool_participation: None,
			observed_bitcoin_total: None,
			observed_mining_seat_total: None,
		};

		assert_noop!(
			OperationalAccountsPallet::force_set_progress(
				RuntimeOrigin::signed(account_set.owner.clone()),
				account_set.owner.clone(),
				non_empty_patch.clone(),
				true,
			),
			DispatchError::BadOrigin
		);

		assert_noop!(
			OperationalAccountsPallet::force_set_progress(
				RuntimeOrigin::root(),
				account_id_from_seed(99),
				non_empty_patch.clone(),
				true,
			),
			crate::Error::<Test>::NotOperationalAccount
		);

		let empty_patch = OperationalProgressPatch {
			has_uniswap_transfer: None,
			vault_created: None,
			has_treasury_pool_participation: None,
			observed_bitcoin_total: None,
			observed_mining_seat_total: None,
		};

		assert_noop!(
			OperationalAccountsPallet::force_set_progress(
				RuntimeOrigin::root(),
				account_set.owner.clone(),
				empty_patch,
				true,
			),
			crate::Error::<Test>::NoProgressUpdateProvided
		);
	});
}

#[test]
fn test_force_set_progress_applies_patch_and_reconciles_totals() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(9, 10, 11, 12);
		register_account(&account_set, None);
		OperationalAccounts::<Test>::mutate(&account_set.owner, |maybe| {
			let account = maybe.as_mut().expect("account");
			account.bitcoin_applied_total = 1_000;
			account.bitcoin_accrual = 0;
			account.mining_seat_applied_total = 5;
			account.mining_seat_accrual = 0;
		});

		assert_ok!(OperationalAccountsPallet::force_set_progress(
			RuntimeOrigin::root(),
			account_set.owner.clone(),
			OperationalProgressPatch {
				has_uniswap_transfer: Some(true),
				vault_created: None,
				has_treasury_pool_participation: None,
				observed_bitcoin_total: Some(1_400),
				observed_mining_seat_total: None,
			},
			false,
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.has_uniswap_transfer);
		assert!(!account.vault_created);
		assert!(!account.has_treasury_pool_participation);
		assert_eq!(account.bitcoin_applied_total, 1_000);
		assert_eq!(account.bitcoin_accrual, 400);
		assert_eq!(account.mining_seat_applied_total, 5);
		assert_eq!(account.mining_seat_accrual, 0);

		assert_ok!(OperationalAccountsPallet::force_set_progress(
			RuntimeOrigin::root(),
			account_set.owner.clone(),
			OperationalProgressPatch {
				has_uniswap_transfer: None,
				vault_created: Some(true),
				has_treasury_pool_participation: Some(true),
				observed_bitcoin_total: None,
				observed_mining_seat_total: Some(9),
			},
			false,
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.has_uniswap_transfer);
		assert!(account.vault_created);
		assert!(account.has_treasury_pool_participation);
		assert_eq!(account.bitcoin_applied_total, 1_000);
		assert_eq!(account.bitcoin_accrual, 400);
		assert_eq!(account.mining_seat_applied_total, 5);
		assert_eq!(account.mining_seat_accrual, 4);

		assert_ok!(OperationalAccountsPallet::force_set_progress(
			RuntimeOrigin::root(),
			account_set.owner.clone(),
			OperationalProgressPatch {
				has_uniswap_transfer: None,
				vault_created: None,
				has_treasury_pool_participation: None,
				observed_bitcoin_total: Some(600),
				observed_mining_seat_total: Some(3),
			},
			false,
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.bitcoin_applied_total, 1_000);
		assert_eq!(account.bitcoin_accrual, 0);
		assert_eq!(account.mining_seat_applied_total, 5);
		assert_eq!(account.mining_seat_accrual, 0);
	});
}

#[test]
fn test_force_set_progress_recompute_flag_controls_side_effects() {
	new_test_ext().execute_with(|| {
		let no_recompute_set = make_account_set(17, 18, 19, 20);
		register_account(&no_recompute_set, None);
		ensure_registration_lookup(
			no_recompute_set.vault.clone(),
			no_recompute_set.mining_funding.clone(),
		);

		assert_ok!(OperationalAccountsPallet::force_set_progress(
			RuntimeOrigin::root(),
			no_recompute_set.owner.clone(),
			OperationalProgressPatch {
				has_uniswap_transfer: Some(true),
				vault_created: Some(true),
				has_treasury_pool_participation: Some(true),
				observed_bitcoin_total: Some(1),
				observed_mining_seat_total: Some(2),
			},
			false,
		));

		let no_recompute =
			OperationalAccounts::<Test>::get(&no_recompute_set.owner).expect("account");
		assert!(!no_recompute.is_operational);
		assert_eq!(no_recompute.available_referrals, 0);
		assert_eq!(no_recompute.rewards_earned_amount, 0);

		let recompute_set = make_account_set(21, 22, 23, 24);
		register_account(&recompute_set, None);
		ensure_registration_lookup(
			recompute_set.vault.clone(),
			recompute_set.mining_funding.clone(),
		);
		assert_ok!(OperationalAccountsPallet::force_set_progress(
			RuntimeOrigin::root(),
			recompute_set.owner.clone(),
			OperationalProgressPatch {
				has_uniswap_transfer: Some(true),
				vault_created: Some(true),
				has_treasury_pool_participation: Some(true),
				observed_bitcoin_total: Some(1),
				observed_mining_seat_total: Some(2),
			},
			true,
		));

		let recompute = OperationalAccounts::<Test>::get(&recompute_set.owner).expect("account");
		assert!(!recompute.is_operational);
		assert_eq!(recompute.available_referrals, 0);
		assert_eq!(recompute.rewards_earned_amount, 0);

		activate_account(&recompute_set);
		let recompute = OperationalAccounts::<Test>::get(&recompute_set.owner).expect("account");
		assert!(recompute.is_operational);
		assert_eq!(recompute.available_referrals, 1);
		assert_eq!(recompute.rewards_earned_amount, OperationalReferralReward::get());
	});
}

#[test]
fn test_referral_registration_consumes_available_and_materializes_ready_referral() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(30, 31, 32, 33);
		register_account(&sponsor_set, None);
		satisfy_and_activate(&sponsor_set);
		let bitcoin_threshold = BitcoinLockSizeForReferral::get();
		OperationalAccounts::<Test>::mutate(&sponsor_set.owner, |maybe| {
			let sponsor_account = maybe.as_mut().expect("sponsor account");
			sponsor_account.available_referrals = 1;
			sponsor_account.bitcoin_accrual = bitcoin_threshold;
		});

		let recruit_set = make_account_set(40, 41, 42, 43);
		let referral_proof = make_referral_proof(&recruit_set.owner, &sponsor_set.owner, 8, 30);

		register_account(&recruit_set, Some(referral_proof));

		let recruit_account =
			OperationalAccounts::<Test>::get(&recruit_set.owner).expect("recruit account");
		assert_eq!(recruit_account.sponsor, Some(sponsor_set.owner.clone()));

		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.available_referrals, 1);
		assert_eq!(sponsor_account.bitcoin_accrual, 0);
	});
}

#[test]
fn test_reused_referral_code_registers_without_sponsor_until_expiration() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(10, 11, 12, 13);
		register_account(&sponsor_set, None);
		set_available_referrals(&sponsor_set.owner, 2);

		let first_recruit = make_account_set(20, 21, 22, 23);
		let expires_at_frame = CurrentFrameId::get().saturating_add(1);
		let first_referral = make_referral_proof_expiring_at(
			&first_recruit.owner,
			&sponsor_set.owner,
			1,
			10,
			expires_at_frame,
		);
		let referral_code = first_referral.referral_code;
		register_account(&first_recruit, Some(first_referral));

		assert_eq!(ConsumedReferralCodes::<Test>::get(referral_code), Some(expires_at_frame));
		assert!(ConsumedReferralCodesByExpiration::<Test>::contains_key(
			expires_at_frame,
			referral_code
		));

		let reused_recruit = make_account_set(30, 31, 32, 33);
		let reused_referral = make_referral_proof(&reused_recruit.owner, &sponsor_set.owner, 1, 10);
		register_account(&reused_recruit, Some(reused_referral));

		let reused_account =
			OperationalAccounts::<Test>::get(&reused_recruit.owner).expect("reused account");
		assert!(reused_account.sponsor.is_none());
		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.available_referrals, 1);

		CurrentFrameId::set(expires_at_frame);
		OperationalAccountsPallet::on_initialize(1);
		assert!(!ConsumedReferralCodes::<Test>::contains_key(referral_code));
		assert!(!ConsumedReferralCodesByExpiration::<Test>::contains_key(
			expires_at_frame,
			referral_code
		));

		let second_recruit = make_account_set(40, 41, 42, 43);
		let second_referral = make_referral_proof(&second_recruit.owner, &sponsor_set.owner, 1, 10);
		let second_expires_at_frame = second_referral.expires_at_frame;
		register_account(&second_recruit, Some(second_referral));

		let second_account =
			OperationalAccounts::<Test>::get(&second_recruit.owner).expect("second account");
		assert_eq!(second_account.sponsor, Some(sponsor_set.owner.clone()));
		assert_eq!(
			ConsumedReferralCodes::<Test>::get(referral_code),
			Some(second_expires_at_frame)
		);
		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.available_referrals, 0);
	});
}

#[test]
fn test_expired_referral_cleanup_is_bounded_across_blocks() {
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		let expires_at_frame = CurrentFrameId::get();
		let referral_codes = [
			sr25519::Public::from_raw([1u8; 32]),
			sr25519::Public::from_raw([2u8; 32]),
			sr25519::Public::from_raw([3u8; 32]),
		];

		for referral_code in referral_codes {
			ConsumedReferralCodes::<Test>::insert(referral_code, expires_at_frame);
			ConsumedReferralCodesByExpiration::<Test>::insert(expires_at_frame, referral_code, ());
		}
	});
	ext.commit_all().expect("seeded referral codes should commit");

	ext.execute_with(|| {
		let expires_at_frame = CurrentFrameId::get();
		OperationalAccountsPallet::on_initialize(1);

		assert_eq!(
			ConsumedReferralCodesByExpiration::<Test>::iter_key_prefix(expires_at_frame).count(),
			1
		);
		assert_eq!(ConsumedReferralCodes::<Test>::iter().count(), 1);
		assert_eq!(ExpiredReferralCodeCleanupFrame::<Test>::get(), Some(expires_at_frame));

		OperationalAccountsPallet::on_initialize(2);

		assert_eq!(
			ConsumedReferralCodesByExpiration::<Test>::iter_key_prefix(expires_at_frame).count(),
			0
		);
		assert_eq!(ConsumedReferralCodes::<Test>::iter().count(), 0);
		assert_eq!(ExpiredReferralCodeCleanupFrame::<Test>::get(), None);
	});
}

#[test]
fn test_set_reward_config_updates_stored_rewards() {
	new_test_ext().execute_with(|| {
		assert_ok!(OperationalAccountsPallet::set_reward_config(RuntimeOrigin::root(), 123, 45,));
		let rewards = Rewards::<Test>::get();
		assert_eq!(rewards.operational_referral_reward, 123);
		assert_eq!(rewards.referral_bonus_reward, 45);
	});
}

#[test]
fn test_registration_lookup_hydrates_current_mining_registration() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(61, 62, 63, 64);
		set_registration_lookup(
			account_set.vault.clone(),
			account_set.mining_funding.clone(),
			1,
			OperationalMinimumVaultSecuritization::get(),
			true,
			1,
		);
		record_recent_argon_transfer(&account_set.vault);

		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(!account.is_operational);
		assert!(account.has_uniswap_transfer);
		assert!(account.vault_created);
		assert!(account.has_treasury_pool_participation);
		assert_eq!(account.bitcoin_accrual, 1);
		assert_eq!(account.mining_seat_accrual, 1);
		assert_eq!(account.available_referrals, 0);

		OperationalAccountsPallet::mining_seat_won(&account_set.mining_funding);
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(!account.is_operational);
		assert_eq!(account.available_referrals, 0);

		activate_account(&account_set);
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_operational);
		assert_eq!(account.available_referrals, 1);
	});
}

#[test]
fn test_registration_lookup_preserves_pre_registration_bitcoin_progress() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(65, 66, 67, 68);
		set_registration_lookup(
			account_set.vault.clone(),
			account_set.mining_funding.clone(),
			BitcoinLockSizeForReferral::get(),
			OperationalMinimumVaultSecuritization::get(),
			true,
			1,
		);
		record_recent_argon_transfer(&account_set.vault);

		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(!account.is_operational);
		assert_eq!(account.available_referrals, 0);
		assert_eq!(account.bitcoin_accrual, BitcoinLockSizeForReferral::get());
		assert_eq!(account.mining_seat_accrual, 1);

		OperationalAccountsPallet::mining_seat_won(&account_set.mining_funding);
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(!account.is_operational);
		assert_eq!(account.available_referrals, 0);

		activate_account(&account_set);
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_operational);
		assert_eq!(account.available_referrals, 2);
		assert_eq!(account.bitcoin_applied_total, BitcoinLockSizeForReferral::get());
		assert_eq!(account.bitcoin_accrual, 0);
		assert_eq!(account.mining_seat_accrual, 2);
	});
}

#[test]
fn test_activate_from_managed_accounts_records_rewards_and_rejects_invalid_calls() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(1, 2, 3, 4);
		register_account(&account_set, None);
		satisfy_operational_requirements(&account_set.mining_funding, &account_set.vault);

		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert!(!operational_account.is_operational);
		assert!(!has_vault_operational_mark(&account_set.vault));
		assert_eq!(operational_account.available_referrals, 0);
		assert_eq!(operational_account.rewards_earned_amount, 0);

		assert_ok!(OperationalAccountsPallet::activate(RuntimeOrigin::signed(
			account_set.mining_funding.clone()
		)));

		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert!(operational_account.is_operational);
		assert!(has_vault_operational_mark(&account_set.vault));
		assert_eq!(operational_account.available_referrals, 1);
		assert_eq!(operational_account.rewards_earned_count, 1);
		assert_eq!(operational_account.rewards_earned_amount, 1_000);
		assert_eq!(pending_rewards_amount(&operational_account), 1_000);

		let activation_cases = [
			make_account_set(10, 11, 12, 13),
			make_account_set(20, 21, 22, 23),
			make_account_set(30, 31, 32, 33),
			make_account_set(40, 41, 42, 43),
		];
		for (index, account_set) in activation_cases.into_iter().enumerate() {
			register_account(&account_set, None);
			satisfy_operational_requirements(&account_set.mining_funding, &account_set.vault);
			let signer = match index {
				0 => account_set.owner.clone(),
				1 => account_set.vault.clone(),
				2 => account_set.mining_funding.clone(),
				_ => account_set.mining_bot.clone(),
			};
			assert_ok!(OperationalAccountsPallet::activate(RuntimeOrigin::signed(signer)));

			let operational_account =
				OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
			assert!(operational_account.is_operational);
			assert!(has_vault_operational_mark(&account_set.vault));
		}
		assert_noop!(
			OperationalAccountsPallet::activate(RuntimeOrigin::signed(account_id_from_seed(54))),
			crate::Error::<Test>::NotOperationalAccount
		);
		assert_noop!(
			OperationalAccountsPallet::activate(RuntimeOrigin::signed(account_set.vault.clone())),
			crate::Error::<Test>::AlreadyOperational
		);
	});
}

#[test]
fn test_genesis_initializes_reward_config() {
	new_test_ext().execute_with(|| {
		assert!(Rewards::<Test>::exists());
		assert_eq!(
			Rewards::<Test>::get(),
			crate::RewardsConfig {
				operational_referral_reward: OperationalReferralReward::get(),
				referral_bonus_reward: OperationalReferralBonusReward::get(),
			}
		);
	});
}

#[test]
fn test_activate_requires_eligibility() {
	new_test_ext().execute_with(|| {
		let missing_bitcoin_set = make_account_set(31, 32, 33, 34);
		register_account(&missing_bitcoin_set, None);
		ensure_registration_lookup(
			missing_bitcoin_set.vault.clone(),
			missing_bitcoin_set.mining_funding.clone(),
		);

		OperationalAccountsPallet::vault_created(&missing_bitcoin_set.vault);
		record_uniswap_transfer(&missing_bitcoin_set.vault, 1_000);
		OperationalAccountsPallet::mining_seat_won(&missing_bitcoin_set.mining_funding);
		OperationalAccountsPallet::mining_seat_won(&missing_bitcoin_set.mining_funding);
		OperationalAccountsPallet::treasury_pool_participated(&missing_bitcoin_set.vault, 1);

		let account = OperationalAccounts::<Test>::get(&missing_bitcoin_set.owner)
			.expect("operational account");
		assert!(!account.is_operational);
		assert!(!has_vault_operational_mark(&missing_bitcoin_set.vault));
		assert_eq!(account.rewards_earned_amount, 0);
		assert_noop!(
			OperationalAccountsPallet::activate(RuntimeOrigin::signed(
				missing_bitcoin_set.owner.clone()
			)),
			crate::Error::<Test>::NotEligibleForActivation
		);

		let insufficient_vault_set = make_account_set(35, 36, 37, 38);
		set_registration_lookup(
			insufficient_vault_set.vault.clone(),
			insufficient_vault_set.mining_funding.clone(),
			0,
			OperationalMinimumVaultSecuritization::get().saturating_sub(1),
			false,
			0,
		);
		register_account(&insufficient_vault_set, None);
		satisfy_operational_requirements(
			&insufficient_vault_set.mining_funding,
			&insufficient_vault_set.vault,
		);

		let account = OperationalAccounts::<Test>::get(&insufficient_vault_set.owner)
			.expect("operational account");
		assert!(!account.is_operational);
		assert!(!has_vault_operational_mark(&insufficient_vault_set.vault));
		assert_eq!(account.rewards_earned_amount, 0);
		assert_noop!(
			OperationalAccountsPallet::activate(RuntimeOrigin::signed(
				insufficient_vault_set.owner.clone()
			)),
			crate::Error::<Test>::NotEligibleForActivation
		);
	});
}

#[test]
fn test_activation_skips_uniswap_transfer_when_it_is_not_required() {
	new_test_ext().execute_with(|| {
		RequiresUniswapTransfer::set(false);
		let account_set = make_account_set(39, 40, 41, 42);
		register_account(&account_set, None);
		ensure_registration_lookup(account_set.vault.clone(), account_set.mining_funding.clone());

		OperationalAccountsPallet::vault_created(&account_set.vault);
		OperationalAccountsPallet::bitcoin_lock_funded(&account_set.vault, 1);
		OperationalAccountsPallet::mining_seat_won(&account_set.mining_funding);
		OperationalAccountsPallet::mining_seat_won(&account_set.mining_funding);
		OperationalAccountsPallet::treasury_pool_participated(&account_set.vault, 1);

		let account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert!(!account.is_operational);
		assert!(!has_vault_operational_mark(&account_set.vault));

		activate_account(&account_set);
		let account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert!(account.is_operational);
		assert!(has_vault_operational_mark(&account_set.vault));
	});
}

#[test]
fn test_referral_bonus_awarded_on_threshold() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(10, 11, 12, 13);
		register_account(&sponsor_set, None);
		satisfy_and_activate(&sponsor_set);
		OperationalAccounts::<Test>::mutate(&sponsor_set.owner, |maybe| {
			let sponsor_account = maybe.as_mut().expect("sponsor account");
			sponsor_account.operational_referrals_count = 4;
			sponsor_account.available_referrals = 1;
			sponsor_account.rewards_earned_count = 0;
			sponsor_account.rewards_earned_amount = 0;
		});

		Rewards::<Test>::put(crate::RewardsConfig {
			operational_referral_reward: 1_000,
			referral_bonus_reward: 250,
		});

		let recruit_set = make_account_set(20, 21, 22, 23);
		let referral_proof = make_referral_proof(&recruit_set.owner, &sponsor_set.owner, 2, 10);

		register_account(&recruit_set, Some(referral_proof));

		satisfy_and_activate(&recruit_set);

		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.operational_referrals_count, 5);
		assert_eq!(sponsor_account.rewards_earned_count, 2);
		assert_eq!(pending_rewards_amount(&sponsor_account), 1_250);

		let recruit_account =
			OperationalAccounts::<Test>::get(&recruit_set.owner).expect("recruit account");
		assert_eq!(recruit_account.rewards_earned_count, 1);
		assert_eq!(pending_rewards_amount(&recruit_account), 1_000);
	});
}

#[test]
fn test_recruit_operational_awards_sponsor_referral() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(10, 11, 12, 13);
		register_account(&sponsor_set, None);
		satisfy_and_activate(&sponsor_set);
		OperationalAccounts::<Test>::mutate(&sponsor_set.owner, |maybe| {
			let sponsor_account = maybe.as_mut().expect("sponsor account");
			sponsor_account.available_referrals = 1;
		});

		let recruit_set = make_account_set(20, 21, 22, 23);
		let referral_proof = make_referral_proof(&recruit_set.owner, &sponsor_set.owner, 3, 10);

		register_account(&recruit_set, Some(referral_proof));

		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.available_referrals, 0);

		satisfy_and_activate(&recruit_set);

		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.available_referrals, 1);
	});
}

#[test]
fn test_pending_referral_materializes_when_referral_consumes_room() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(50, 51, 52, 53);
		register_account(&sponsor_set, None);
		satisfy_and_activate(&sponsor_set);
		OperationalAccounts::<Test>::mutate(&sponsor_set.owner, |maybe| {
			let sponsor_account = maybe.as_mut().expect("sponsor account");
			sponsor_account.available_referrals = MaxAvailableReferrals::get();
		});

		let recruit_set = make_account_set(60, 61, 62, 63);
		let referral_proof = make_referral_proof(&recruit_set.owner, &sponsor_set.owner, 4, 50);
		register_account(&recruit_set, Some(referral_proof));
		satisfy_and_activate(&recruit_set);

		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert!(!sponsor_account.referral_pending);
		assert_eq!(sponsor_account.available_referrals, MaxAvailableReferrals::get());
	});
}

#[test]
fn test_bitcoin_progress_is_single_pending_and_resets_on_issuance() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(1, 2, 3, 4);
		register_account(&account_set, None);
		satisfy_and_activate(&account_set);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.available_referrals, 1);

		let min_lock = 1;
		let referral_threshold = BitcoinLockSizeForReferral::get();
		set_available_referrals(&account_set.owner, MaxAvailableReferrals::get());
		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(7_000),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.available_referrals, MaxAvailableReferrals::get());
		assert!(operational_account.bitcoin_accrual > referral_threshold);
		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(6_000),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert!(operational_account.bitcoin_accrual >= referral_threshold);

		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(2_000),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert!(operational_account.bitcoin_accrual < referral_threshold);

		set_available_referrals(&account_set.owner, 1);
		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(5_000),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.available_referrals, 2);
		assert_eq!(operational_account.bitcoin_accrual, 0);
	});
}

#[test]
fn test_bitcoin_recovery_to_old_total_does_not_reaward_referral() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(70, 71, 72, 73);
		register_account(&account_set, None);
		satisfy_and_activate(&account_set);

		let min_lock = 1;
		let referral_threshold = BitcoinLockSizeForReferral::get();

		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(referral_threshold),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.available_referrals, 2);
		assert_eq!(operational_account.bitcoin_accrual, 0);

		set_available_referrals(&account_set.owner, 1);
		OperationalAccountsPallet::bitcoin_lock_funded(&account_set.vault, min_lock);
		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(referral_threshold),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.available_referrals, 1);
		assert_eq!(operational_account.bitcoin_accrual, 0);

		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(referral_threshold.saturating_mul(2)),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.available_referrals, 2);
		assert_eq!(operational_account.bitcoin_accrual, 0);
	});
}

#[test]
fn test_referrals_awarded_for_mining_seats() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(5, 6, 7, 8);
		register_account(&account_set, None);
		satisfy_and_activate(&account_set);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.available_referrals, 1);

		for _ in 0..3 {
			OperationalAccountsPallet::mining_seat_won(&account_set.mining_funding);
		}

		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.available_referrals, 2);
	});
}

#[test]
fn test_claim_rewards_pays_to_managed_signer_and_decrements_pending() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(80, 81, 82, 83);
		register_account(&account_set, None);
		seed_pending_rewards(&account_set.owner, 3 * MICROGONS_PER_ARGON);
		ClaimableTreasuryBalance::set(2 * MICROGONS_PER_ARGON);

		assert_ok!(OperationalAccountsPallet::claim_rewards(
			RuntimeOrigin::signed(account_set.vault.clone()),
			MICROGONS_PER_ARGON,
		));

		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.rewards_collected_amount, MICROGONS_PER_ARGON);
		assert_eq!(pending_rewards_amount(&operational_account), 2 * MICROGONS_PER_ARGON);
		assert_eq!(
			ClaimedOperationalRewards::get(),
			vec![(account_set.vault, MICROGONS_PER_ARGON)]
		);
		assert_eq!(ClaimableTreasuryBalance::get(), MICROGONS_PER_ARGON);
	});
}

#[test]
fn test_claim_rewards_rejects_invalid_claims() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(80, 81, 82, 83);
		register_account(&account_set, None);
		seed_pending_rewards(&account_set.owner, 2 * MICROGONS_PER_ARGON);
		ClaimableTreasuryBalance::set(2 * MICROGONS_PER_ARGON);

		assert_noop!(
			OperationalAccountsPallet::claim_rewards(
				RuntimeOrigin::signed(account_id_from_seed(84)),
				MICROGONS_PER_ARGON,
			),
			crate::Error::<Test>::NotOperationalAccount
		);
		assert_noop!(
			OperationalAccountsPallet::claim_rewards(
				RuntimeOrigin::signed(account_set.owner.clone()),
				MICROGONS_PER_ARGON.saturating_sub(1),
			),
			crate::Error::<Test>::RewardClaimBelowMinimum
		);
		assert_noop!(
			OperationalAccountsPallet::claim_rewards(
				RuntimeOrigin::signed(account_set.owner.clone()),
				MICROGONS_PER_ARGON.saturating_add(1),
			),
			crate::Error::<Test>::RewardClaimNotWholeArgon
		);

		assert_noop!(
			OperationalAccountsPallet::claim_rewards(
				RuntimeOrigin::signed(account_set.mining_funding.clone()),
				3 * MICROGONS_PER_ARGON,
			),
			crate::Error::<Test>::RewardClaimExceedsPending
		);

		ClaimableTreasuryBalance::set(MICROGONS_PER_ARGON.saturating_sub(1));
		assert_noop!(
			OperationalAccountsPallet::claim_rewards(
				RuntimeOrigin::signed(account_set.mining_funding),
				MICROGONS_PER_ARGON,
			),
			crate::Error::<Test>::TreasuryInsufficientFunds
		);

		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.rewards_collected_amount, 0);
		assert_eq!(pending_rewards_amount(&operational_account), 2 * MICROGONS_PER_ARGON);
		assert!(ClaimedOperationalRewards::get().is_empty());
	});
}

#[test]
fn test_sponsor_can_set_encrypted_server_for_sponsee() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(86, 87, 88, 89);
		let sponsee_set = make_account_set(90, 91, 92, 93);
		register_account(&sponsor_set, None);
		register_account(&sponsee_set, None);
		OperationalAccounts::<Test>::mutate(&sponsee_set.owner, |maybe| {
			let account = maybe.as_mut().expect("sponsee account");
			account.sponsor = Some(sponsor_set.owner.clone());
		});

		let encrypted_server = vec![4u8; 48];
		assert_ok!(OperationalAccountsPallet::set_encrypted_server_for_sponsee(
			RuntimeOrigin::signed(sponsor_set.owner.clone()),
			sponsee_set.owner.clone(),
			encrypted_server.clone(),
		));
		assert_eq!(
			EncryptedServerBySponsee::<Test>::get(&sponsee_set.owner)
				.expect("encrypted server")
				.to_vec(),
			encrypted_server
		);

		let replacement = vec![5u8; 16];
		assert_ok!(OperationalAccountsPallet::set_encrypted_server_for_sponsee(
			RuntimeOrigin::signed(sponsor_set.owner.clone()),
			sponsee_set.owner.clone(),
			replacement.clone(),
		));
		assert_eq!(
			EncryptedServerBySponsee::<Test>::get(&sponsee_set.owner)
				.expect("replacement encrypted server")
				.to_vec(),
			replacement
		);
	});
}

#[test]
fn test_set_encrypted_server_requires_sponsor_relationship() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(94, 95, 96, 97);
		let sponsee_set = make_account_set(98, 99, 100, 101);
		let outsider_set = make_account_set(102, 103, 104, 105);
		register_account(&sponsor_set, None);
		register_account(&sponsee_set, None);
		register_account(&outsider_set, None);
		OperationalAccounts::<Test>::mutate(&sponsee_set.owner, |maybe| {
			let account = maybe.as_mut().expect("sponsee account");
			account.sponsor = Some(sponsor_set.owner.clone());
		});

		assert_noop!(
			OperationalAccountsPallet::set_encrypted_server_for_sponsee(
				RuntimeOrigin::signed(outsider_set.owner.clone()),
				sponsee_set.owner.clone(),
				vec![1u8; 8],
			),
			crate::Error::<Test>::NotSponsorOfSponsee
		);
	});
}

#[test]
fn test_set_encrypted_server_rejects_oversized_payload() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(106, 107, 108, 109);
		let sponsee_set = make_account_set(110, 111, 112, 113);
		register_account(&sponsor_set, None);
		register_account(&sponsee_set, None);
		OperationalAccounts::<Test>::mutate(&sponsee_set.owner, |maybe| {
			let account = maybe.as_mut().expect("sponsee account");
			account.sponsor = Some(sponsor_set.owner.clone());
		});

		assert_noop!(
			OperationalAccountsPallet::set_encrypted_server_for_sponsee(
				RuntimeOrigin::signed(sponsor_set.owner.clone()),
				sponsee_set.owner.clone(),
				vec![9u8; MaxEncryptedServerLen::get() as usize + 1],
			),
			crate::Error::<Test>::EncryptedServerTooLong
		);
	});
}

#[derive(Clone)]
struct AccountSet {
	owner: TestAccountId,
	vault: TestAccountId,
	mining_funding: TestAccountId,
	mining_bot: TestAccountId,
	encryption_pubkey: OpaqueEncryptionPubkey,
	owner_proof: AccountOwnershipProof,
	vault_proof: AccountOwnershipProof,
	mining_funding_proof: AccountOwnershipProof,
	mining_bot_proof: AccountOwnershipProof,
}

impl AccountSet {
	fn registration(
		&self,
		referral_proof: Option<ReferralProof<TestAccountId>>,
	) -> Registration<Test> {
		Registration::V1(RegistrationV1 {
			operational_account: self.owner.clone(),
			encryption_pubkey: self.encryption_pubkey.clone(),
			operational_account_proof: self.owner_proof.clone(),
			vault_account: self.vault.clone(),
			mining_funding_account: self.mining_funding.clone(),
			mining_bot_account: self.mining_bot.clone(),
			vault_account_proof: self.vault_proof.clone(),
			mining_funding_account_proof: self.mining_funding_proof.clone(),
			mining_bot_account_proof: self.mining_bot_proof.clone(),
			referral_proof,
		})
	}
}

fn seed_pending_rewards(owner: &TestAccountId, amount: Balance) {
	OperationalAccounts::<Test>::mutate(owner, |maybe| {
		let account = maybe.as_mut().expect("operational account");
		account.rewards_earned_amount = account.rewards_collected_amount.saturating_add(amount);
	});
}

fn pending_rewards_amount(account: &crate::OperationalAccount<Test>) -> Balance {
	account.rewards_earned_amount.saturating_sub(account.rewards_collected_amount)
}

fn make_referral_proof(
	account: &TestAccountId,
	sponsor: &TestAccountId,
	referral_seed: u8,
	sponsor_seed: u8,
) -> ReferralProof<TestAccountId> {
	make_referral_proof_expiring_at(
		account,
		sponsor,
		referral_seed,
		sponsor_seed,
		CurrentFrameId::get().saturating_add(10),
	)
}

fn make_referral_proof_expiring_at(
	account: &TestAccountId,
	sponsor: &TestAccountId,
	referral_seed: u8,
	sponsor_seed: u8,
	expires_at_frame: FrameId,
) -> ReferralProof<TestAccountId> {
	let pair = sr25519::Pair::from_seed(&[referral_seed; 32]);
	let referral_code = pair.public();
	let message =
		(REFERRAL_CLAIM_PROOF_MESSAGE_KEY, referral_code, account).using_encoded(blake2_256);
	let referral_signature = pair.sign(message.as_slice());
	let sponsor_pair = sr25519::Pair::from_seed(&[sponsor_seed; 32]);
	let sponsor_message =
		(REFERRAL_SPONSOR_GRANT_MESSAGE_KEY, sponsor, referral_code, expires_at_frame)
			.using_encoded(blake2_256);
	let sponsor_signature: Signature = sponsor_pair.sign(sponsor_message.as_slice()).into();
	ReferralProof {
		referral_code,
		referral_signature,
		sponsor: sponsor.clone(),
		expires_at_frame,
		sponsor_signature,
	}
}

fn make_linked_account(
	owner: &TestAccountId,
	seed: u8,
	domain: &[u8],
) -> (TestAccountId, AccountOwnershipProof) {
	let pair = sr25519::Pair::from_seed(&[seed; 32]);
	let account_id = MultiSigner::from(pair.public()).into_account();
	let proof = make_account_proof(owner, &account_id, seed, domain);
	(account_id, proof)
}

fn account_id_from_seed(seed: u8) -> TestAccountId {
	let pair = sr25519::Pair::from_seed(&[seed; 32]);
	MultiSigner::from(pair.public()).into_account()
}

fn make_account_proof(
	owner: &TestAccountId,
	account_id: &TestAccountId,
	seed: u8,
	domain: &[u8],
) -> AccountOwnershipProof {
	let pair = sr25519::Pair::from_seed(&[seed; 32]);
	let account_id =
		AccountId32::decode(&mut account_id.encode().as_slice()).expect("account id32");
	let message = (domain, owner, &account_id).using_encoded(blake2_256);
	let signature: Signature = pair.sign(message.as_slice()).into();
	AccountOwnershipProof { signature }
}

fn make_account_set(owner_seed: u8, vault_seed: u8, funding_seed: u8, bot_seed: u8) -> AccountSet {
	let owner = account_id_from_seed(owner_seed);
	let owner_proof =
		make_account_proof(&owner, &owner, owner_seed, OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY);
	let encryption_pubkey = OpaqueEncryptionPubkey([owner_seed; 32]);
	let (vault, vault_proof) =
		make_linked_account(&owner, vault_seed, VAULT_ACCOUNT_PROOF_MESSAGE_KEY);
	let (mining_funding, mining_funding_proof) =
		make_linked_account(&owner, funding_seed, MINING_FUNDING_ACCOUNT_PROOF_MESSAGE_KEY);
	let (mining_bot, mining_bot_proof) =
		make_linked_account(&owner, bot_seed, MINING_BOT_ACCOUNT_PROOF_MESSAGE_KEY);
	AccountSet {
		owner,
		vault,
		mining_funding,
		mining_bot,
		encryption_pubkey,
		owner_proof,
		vault_proof,
		mining_funding_proof,
		mining_bot_proof,
	}
}

fn register_account(set: &AccountSet, referral_proof: Option<ReferralProof<TestAccountId>>) {
	register_account_with_submitter(set, &set.owner, referral_proof);
}

fn register_account_with_submitter(
	set: &AccountSet,
	submitter: &TestAccountId,
	referral_proof: Option<ReferralProof<TestAccountId>>,
) {
	assert_ok!(OperationalAccountsPallet::register(
		RuntimeOrigin::signed(submitter.clone()),
		set.registration(referral_proof),
	));
}

fn record_uniswap_transfer(vault_account: &TestAccountId, amount: Balance) {
	OperationalAccountsPallet::on_uniswap_transfer(vault_account, amount);
}

fn record_recent_argon_transfer(account_id: &TestAccountId) {
	pallet_inbound_transfer_log::RecentArgonTransfersByAccount::<Test>::insert(account_id, 1);
}

fn satisfy_operational_requirements(mining_account: &TestAccountId, vault_account: &TestAccountId) {
	ensure_registration_lookup(vault_account.clone(), mining_account.clone());
	OperationalAccountsPallet::vault_created(vault_account);
	record_uniswap_transfer(vault_account, 1_000);
	OperationalAccountsPallet::bitcoin_lock_funded(vault_account, 1);
	OperationalAccountsPallet::mining_seat_won(mining_account);
	OperationalAccountsPallet::mining_seat_won(mining_account);
	OperationalAccountsPallet::treasury_pool_participated(vault_account, 1);
}

fn activate_account(set: &AccountSet) {
	assert_ok!(OperationalAccountsPallet::activate(RuntimeOrigin::signed(
		set.mining_funding.clone()
	)));
}

fn satisfy_and_activate(set: &AccountSet) {
	satisfy_operational_requirements(&set.mining_funding, &set.vault);
	activate_account(set);
}

fn set_available_referrals(owner: &TestAccountId, count: u32) {
	OperationalAccounts::<Test>::mutate(owner, |maybe| {
		let account = maybe.as_mut().expect("operational account");
		account.available_referrals = count;
	});
}
