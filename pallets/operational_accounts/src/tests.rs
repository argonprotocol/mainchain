use crate::{
	ACCESS_CODE_PROOF_MESSAGE_KEY, AccessCodeMetadata, AccessCodeProof, AccessCodesByPublic,
	AccessCodesExpiringByFrame, AccountOwnershipProof, EncryptedServerBySponsee,
	MINING_BOT_ACCOUNT_PROOF_MESSAGE_KEY, MINING_FUNDING_ACCOUNT_PROOF_MESSAGE_KEY,
	OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY, OpaqueEncryptionPubkey, OperationalAccountBySubAccount,
	OperationalAccounts, OperationalProgressPatch, OperationalRewardsQueue, Registration,
	RegistrationV1, Rewards, VAULT_ACCOUNT_PROOF_MESSAGE_KEY,
};
use argon_primitives::{
	OperationalAccountsHook, OperationalRewardKind, OperationalRewardPayout,
	OperationalRewardsProvider, Signature,
};
use frame_support::{assert_err, assert_noop, assert_ok};
use pallet_prelude::*;
use sp_core::{Pair, sr25519};
use sp_io::hashing::blake2_256;
use sp_runtime::{AccountId32, DispatchError, MultiSigner, traits::IdentifyAccount};

use crate::mock::{
	BitcoinLockSizeForAccessCode, CurrentFrameId, MaxAccessCodesExpiringPerFrame,
	MaxEncryptedServerLen, MaxIssuableAccessCodes, MaxOperationalRewardsQueued,
	OperationalAccounts as OperationalAccountsPallet, OperationalMinimumVaultSecuritization,
	OperationalReferralBonusReward, OperationalReferralReward, RuntimeOrigin, System, Test,
	TestAccountId, ensure_registration_lookup, has_vault_operational_mark, new_test_ext,
	set_registration_lookup,
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
fn test_register_with_access_code_sets_sponsor_and_decrements_unactivated() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(10, 11, 12, 13);
		register_account(&sponsor_set, None);
		OperationalAccounts::<Test>::mutate(&sponsor_set.owner, |maybe| {
			let sponsor_account = maybe.as_mut().expect("sponsor account");
			sponsor_account.unactivated_access_codes = 1;
		});

		let recruit_set = make_account_set(20, 21, 22, 23);
		let access_code = make_access_code_proof(&recruit_set.owner, 1);
		AccessCodesByPublic::<Test>::insert(
			access_code.public,
			AccessCodeMetadata { sponsor: sponsor_set.owner.clone(), expiration_frame: 5 },
		);
		AccessCodesExpiringByFrame::<Test>::mutate(5, |expiring_codes| {
			assert!(expiring_codes.try_push(access_code.public).is_ok());
		});

		register_account_with_submitter(&recruit_set, &recruit_set.vault, Some(access_code));

		let recruit_account =
			OperationalAccounts::<Test>::get(&recruit_set.owner).expect("recruit account");
		assert_eq!(recruit_account.sponsor, Some(sponsor_set.owner.clone()));
		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.unactivated_access_codes, 0);
		assert_eq!(sponsor_account.issuable_access_codes, 0);
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
		assert_eq!(no_recompute.issuable_access_codes, 0);
		assert!(OperationalRewardsQueue::<Test>::get().is_empty());

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
		assert!(recompute.is_operational);
		assert_eq!(recompute.issuable_access_codes, 1);
		let queue = OperationalRewardsQueue::<Test>::get();
		assert_eq!(queue.len(), 1);
		assert_eq!(queue[0].operational_account, recompute_set.owner);
	});
}

#[test]
fn test_access_code_activation_materializes_pending_sponsor_issuance() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(30, 31, 32, 33);
		register_account(&sponsor_set, None);
		satisfy_operational_requirements(&sponsor_set.mining_funding, &sponsor_set.vault);
		let bitcoin_threshold = BitcoinLockSizeForAccessCode::get();
		OperationalAccounts::<Test>::mutate(&sponsor_set.owner, |maybe| {
			let sponsor_account = maybe.as_mut().expect("sponsor account");
			sponsor_account.issuable_access_codes = 1;
			sponsor_account.unactivated_access_codes = 1;
			sponsor_account.bitcoin_accrual = bitcoin_threshold;
		});

		let recruit_set = make_account_set(40, 41, 42, 43);
		let access_code = make_access_code_proof(&recruit_set.owner, 8);
		AccessCodesByPublic::<Test>::insert(
			access_code.public,
			AccessCodeMetadata { sponsor: sponsor_set.owner.clone(), expiration_frame: 5 },
		);
		AccessCodesExpiringByFrame::<Test>::mutate(5, |expiring_codes| {
			assert!(expiring_codes.try_push(access_code.public).is_ok());
		});

		register_account(&recruit_set, Some(access_code));

		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.unactivated_access_codes, 0);
		assert_eq!(sponsor_account.issuable_access_codes, 2);
		assert_eq!(sponsor_account.bitcoin_accrual, 0);
	});
}

#[test]
fn test_issue_access_code_tracks_expiration_and_counts() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(1, 2, 3, 4);
		register_account(&sponsor_set, None);
		set_issuable_access_codes(&sponsor_set.owner, 1);

		CurrentFrameId::set(10);
		let access_code = sr25519::Public::from_raw([7u8; 32]);
		assert_ok!(OperationalAccountsPallet::issue_access_code(
			RuntimeOrigin::signed(sponsor_set.owner.clone()),
			access_code,
		));

		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.issuable_access_codes, 0);
		assert_eq!(sponsor_account.unactivated_access_codes, 1);

		let code_metadata = AccessCodesByPublic::<Test>::get(access_code).expect("code");
		assert_eq!(code_metadata.sponsor, sponsor_set.owner);
		assert_eq!(code_metadata.expiration_frame, 12);
		let expiring_codes = AccessCodesExpiringByFrame::<Test>::get(12);
		assert_eq!(expiring_codes.len(), 1);
		assert_eq!(expiring_codes[0], access_code);
	});
}

#[test]
fn test_issue_access_code_rejects_full_expiration_frame() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(1, 2, 3, 4);
		register_account(&sponsor_set, None);
		set_issuable_access_codes(&sponsor_set.owner, 1);

		CurrentFrameId::set(10);
		let expiration_frame = 12;
		AccessCodesExpiringByFrame::<Test>::mutate(expiration_frame, |expiring_codes| {
			for seed in 0..MaxAccessCodesExpiringPerFrame::get() {
				let mut bytes = [0u8; 32];
				bytes[0] = seed as u8;
				assert!(expiring_codes.try_push(sr25519::Public::from_raw(bytes)).is_ok());
			}
		});

		assert_noop!(
			OperationalAccountsPallet::issue_access_code(
				RuntimeOrigin::signed(sponsor_set.owner.clone()),
				sr25519::Public::from_raw([8u8; 32]),
			),
			crate::Error::<Test>::MaxAccessCodesExpiringPerFrameReached
		);
	});
}

#[test]
fn test_access_code_expiration_cleanup() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(1, 2, 3, 4);
		register_account(&sponsor_set, None);
		OperationalAccounts::<Test>::mutate(&sponsor_set.owner, |maybe| {
			let sponsor_account = maybe.as_mut().expect("sponsor account");
			sponsor_account.unactivated_access_codes = 1;
			sponsor_account.is_operational = true;
			sponsor_account.issuable_access_codes = 0;
		});

		CurrentFrameId::set(5);
		let access_code = sr25519::Public::from_raw([9u8; 32]);
		AccessCodesByPublic::<Test>::insert(
			access_code,
			AccessCodeMetadata { sponsor: sponsor_set.owner.clone(), expiration_frame: 5 },
		);
		AccessCodesExpiringByFrame::<Test>::mutate(5, |expiring_codes| {
			assert!(expiring_codes.try_push(access_code).is_ok());
		});

		OperationalAccountsPallet::on_initialize(1u32.into());

		assert!(AccessCodesByPublic::<Test>::get(access_code).is_none());
		assert!(AccessCodesExpiringByFrame::<Test>::get(5).is_empty());
		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.unactivated_access_codes, 0);
		assert_eq!(sponsor_account.issuable_access_codes, 1);
	});
}

#[test]
fn test_runtime_setup_runs_only_once() {
	new_test_ext().execute_with(|| {
		Rewards::<Test>::kill();
		OperationalAccountsPallet::on_runtime_upgrade();
		let rewards = Rewards::<Test>::get();
		assert_eq!(rewards.operational_referral_reward, OperationalReferralReward::get());
		assert_eq!(rewards.referral_bonus_reward, OperationalReferralBonusReward::get());
		Rewards::<Test>::put(crate::RewardsConfig {
			operational_referral_reward: 123,
			referral_bonus_reward: 45,
		});
		OperationalAccountsPallet::on_runtime_upgrade();
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
		assert_eq!(account.issuable_access_codes, 0);

		OperationalAccountsPallet::mining_seat_won(&account_set.mining_funding);
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_operational);
		assert_eq!(account.issuable_access_codes, 1);
	});
}

#[test]
fn test_registration_lookup_preserves_pre_registration_bitcoin_progress() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(65, 66, 67, 68);
		set_registration_lookup(
			account_set.vault.clone(),
			account_set.mining_funding.clone(),
			BitcoinLockSizeForAccessCode::get(),
			OperationalMinimumVaultSecuritization::get(),
			true,
			1,
		);
		record_recent_argon_transfer(&account_set.vault);

		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(!account.is_operational);
		assert_eq!(account.issuable_access_codes, 0);
		assert_eq!(account.bitcoin_accrual, BitcoinLockSizeForAccessCode::get());
		assert_eq!(account.mining_seat_accrual, 1);

		OperationalAccountsPallet::mining_seat_won(&account_set.mining_funding);
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_operational);
		assert_eq!(account.issuable_access_codes, 2);
		assert_eq!(account.bitcoin_applied_total, BitcoinLockSizeForAccessCode::get());
		assert_eq!(account.bitcoin_accrual, 0);
		assert_eq!(account.mining_seat_accrual, 2);
	});
}

#[test]
fn test_activation_queues_reward_when_requirements_met() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(1, 2, 3, 4);
		register_account(&account_set, None);
		satisfy_operational_requirements(&account_set.mining_funding, &account_set.vault);

		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert!(operational_account.is_operational);
		assert!(has_vault_operational_mark(&account_set.vault));
		assert_eq!(operational_account.issuable_access_codes, 1);
		let queue = OperationalRewardsQueue::<Test>::get();
		assert_eq!(queue.len(), 1);
		let reward = queue[0].clone();
		assert_eq!(reward.operational_account, account_set.owner);
		assert_eq!(reward.payout_account, account_set.mining_funding);
		assert_eq!(reward.reward_kind, OperationalRewardKind::Activation);
		assert_eq!(reward.amount, 1_000);

		assert_eq!(operational_account.rewards_earned_count, 1);
		assert_eq!(operational_account.rewards_earned_amount, 1_000);

		OperationalAccountsPallet::mark_reward_paid(&reward, reward.amount);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.rewards_collected_amount, 1_000);
		assert!(OperationalRewardsQueue::<Test>::get().is_empty());
	});
}

#[test]
fn test_activation_requires_positive_bitcoin() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(31, 32, 33, 34);
		register_account(&account_set, None);
		ensure_registration_lookup(account_set.vault.clone(), account_set.mining_funding.clone());

		OperationalAccountsPallet::vault_created(&account_set.vault);
		record_uniswap_transfer(&account_set.vault, 1_000);
		OperationalAccountsPallet::mining_seat_won(&account_set.mining_funding);
		OperationalAccountsPallet::mining_seat_won(&account_set.mining_funding);
		OperationalAccountsPallet::treasury_pool_participated(&account_set.vault, 1);

		let account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert!(!account.is_operational);
		assert!(!has_vault_operational_mark(&account_set.vault));
		assert!(OperationalRewardsQueue::<Test>::get().is_empty());
	});
}

#[test]
fn test_activation_requires_minimum_vault_securitization() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(35, 36, 37, 38);
		set_registration_lookup(
			account_set.vault.clone(),
			account_set.mining_funding.clone(),
			0,
			OperationalMinimumVaultSecuritization::get().saturating_sub(1),
			false,
			0,
		);
		register_account(&account_set, None);
		satisfy_operational_requirements(&account_set.mining_funding, &account_set.vault);

		let account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert!(!account.is_operational);
		assert!(!has_vault_operational_mark(&account_set.vault));
		assert!(OperationalRewardsQueue::<Test>::get().is_empty());
	});
}

#[test]
fn test_mark_reward_paid_consumes_queue_on_partial_payment() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(61, 62, 63, 64);
		register_account(&account_set, None);
		satisfy_operational_requirements(&account_set.mining_funding, &account_set.vault);

		let reward = OperationalRewardsQueue::<Test>::get()[0].clone();
		OperationalAccountsPallet::mark_reward_paid(&reward, 250);

		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.rewards_collected_amount, 250);

		let queue = OperationalRewardsQueue::<Test>::get();
		assert!(queue.is_empty());
	});
}

#[test]
fn test_operational_referral_reward_enqueue_failed_emits_event() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(101, 102, 103, 104);
		register_account(&account_set, None);
		System::set_block_number(1);

		let filler = make_account_set(111, 112, 113, 114);
		let reward = OperationalRewardPayout {
			operational_account: filler.owner,
			payout_account: filler.mining_funding,
			reward_kind: OperationalRewardKind::Activation,
			amount: 1,
		};
		OperationalRewardsQueue::<Test>::mutate(|queue| {
			for _ in 0..MaxOperationalRewardsQueued::get() {
				assert!(queue.try_push(reward.clone()).is_ok());
			}
		});

		System::reset_events();
		satisfy_operational_requirements(&account_set.mining_funding, &account_set.vault);

		System::assert_has_event(
			crate::Event::<Test>::OperationalRewardEnqueueFailed {
				account: account_set.owner.clone(),
				reward_kind: OperationalRewardKind::Activation,
				amount: 1_000,
			}
			.into(),
		);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_operational);
		assert_eq!(account.rewards_earned_count, 0);
		assert_eq!(account.rewards_earned_amount, 0);
		assert_eq!(
			OperationalRewardsQueue::<Test>::get().len() as u32,
			MaxOperationalRewardsQueued::get()
		);
	});
}

#[test]
fn test_referral_bonus_awarded_on_threshold() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(10, 11, 12, 13);
		register_account(&sponsor_set, None);
		satisfy_operational_requirements(&sponsor_set.mining_funding, &sponsor_set.vault);
		OperationalAccounts::<Test>::mutate(&sponsor_set.owner, |maybe| {
			let sponsor_account = maybe.as_mut().expect("sponsor account");
			sponsor_account.operational_referrals_count = 4;
			sponsor_account.unactivated_access_codes = 1;
		});
		OperationalRewardsQueue::<Test>::kill();

		Rewards::<Test>::put(crate::RewardsConfig {
			operational_referral_reward: 1_000,
			referral_bonus_reward: 250,
		});

		let recruit_set = make_account_set(20, 21, 22, 23);
		let access_code = make_access_code_proof(&recruit_set.owner, 2);
		AccessCodesByPublic::<Test>::insert(
			access_code.public,
			AccessCodeMetadata { sponsor: sponsor_set.owner.clone(), expiration_frame: 5 },
		);

		register_account(&recruit_set, Some(access_code));

		satisfy_operational_requirements(&recruit_set.mining_funding, &recruit_set.vault);

		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.operational_referrals_count, 5);
		let queue = OperationalRewardsQueue::<Test>::get();
		assert_eq!(queue.len(), 3);
		assert_eq!(queue[0].operational_account, recruit_set.owner);
		assert_eq!(queue[0].reward_kind, OperationalRewardKind::Activation);
		assert_eq!(queue[1].operational_account, sponsor_set.owner);
		assert_eq!(queue[1].reward_kind, OperationalRewardKind::Activation);
		assert_eq!(queue[1].amount, 1_000);
		assert_eq!(queue[2].operational_account, sponsor_set.owner);
		assert_eq!(queue[2].reward_kind, OperationalRewardKind::ReferralBonus);
		assert_eq!(queue[2].amount, 250);
	});
}

#[test]
fn test_recruit_operational_awards_sponsor_access_code() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(10, 11, 12, 13);
		register_account(&sponsor_set, None);
		satisfy_operational_requirements(&sponsor_set.mining_funding, &sponsor_set.vault);
		OperationalAccounts::<Test>::mutate(&sponsor_set.owner, |maybe| {
			let sponsor_account = maybe.as_mut().expect("sponsor account");
			sponsor_account.unactivated_access_codes = 1;
			sponsor_account.issuable_access_codes = 0;
		});

		let recruit_set = make_account_set(20, 21, 22, 23);
		let access_code = make_access_code_proof(&recruit_set.owner, 3);
		AccessCodesByPublic::<Test>::insert(
			access_code.public,
			AccessCodeMetadata { sponsor: sponsor_set.owner.clone(), expiration_frame: 5 },
		);
		AccessCodesExpiringByFrame::<Test>::mutate(5, |expiring_codes| {
			assert!(expiring_codes.try_push(access_code.public).is_ok());
		});

		register_account(&recruit_set, Some(access_code));

		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.issuable_access_codes, 0);

		satisfy_operational_requirements(&recruit_set.mining_funding, &recruit_set.vault);

		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert_eq!(sponsor_account.issuable_access_codes, 1);
	});
}

#[test]
fn test_pending_referral_access_code_materializes_when_issuance_room_opens() {
	new_test_ext().execute_with(|| {
		let sponsor_set = make_account_set(50, 51, 52, 53);
		register_account(&sponsor_set, None);
		satisfy_operational_requirements(&sponsor_set.mining_funding, &sponsor_set.vault);
		OperationalAccounts::<Test>::mutate(&sponsor_set.owner, |maybe| {
			let sponsor_account = maybe.as_mut().expect("sponsor account");
			sponsor_account.issuable_access_codes = MaxIssuableAccessCodes::get();
			sponsor_account.unactivated_access_codes = 1;
		});

		let recruit_set = make_account_set(60, 61, 62, 63);
		let access_code = make_access_code_proof(&recruit_set.owner, 4);
		AccessCodesByPublic::<Test>::insert(
			access_code.public,
			AccessCodeMetadata { sponsor: sponsor_set.owner.clone(), expiration_frame: 5 },
		);
		AccessCodesExpiringByFrame::<Test>::mutate(5, |expiring_codes| {
			assert!(expiring_codes.try_push(access_code.public).is_ok());
		});
		register_account(&recruit_set, Some(access_code));
		satisfy_operational_requirements(&recruit_set.mining_funding, &recruit_set.vault);

		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert!(sponsor_account.referral_access_code_pending);
		assert_eq!(sponsor_account.issuable_access_codes, MaxIssuableAccessCodes::get());

		assert_ok!(OperationalAccountsPallet::issue_access_code(
			RuntimeOrigin::signed(sponsor_set.owner.clone()),
			sr25519::Public::from_raw([44u8; 32]),
		));
		let sponsor_account =
			OperationalAccounts::<Test>::get(&sponsor_set.owner).expect("sponsor account");
		assert!(!sponsor_account.referral_access_code_pending);
		assert_eq!(sponsor_account.issuable_access_codes, MaxIssuableAccessCodes::get());
	});
}

#[test]
fn test_bitcoin_progress_is_single_pending_and_resets_on_issuance() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(1, 2, 3, 4);
		register_account(&account_set, None);
		satisfy_operational_requirements(&account_set.mining_funding, &account_set.vault);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.issuable_access_codes, 1);

		let min_lock = 1;
		let access_code_threshold = BitcoinLockSizeForAccessCode::get();
		set_issuable_access_codes(&account_set.owner, MaxIssuableAccessCodes::get());
		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(7_000),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.issuable_access_codes, MaxIssuableAccessCodes::get());
		assert!(operational_account.bitcoin_accrual > access_code_threshold);
		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(6_000),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert!(operational_account.bitcoin_accrual >= access_code_threshold);

		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(2_000),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert!(operational_account.bitcoin_accrual < access_code_threshold);

		set_issuable_access_codes(&account_set.owner, 1);
		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(5_000),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.issuable_access_codes, 2);
		assert_eq!(operational_account.bitcoin_accrual, 0);
	});
}

#[test]
fn test_bitcoin_recovery_to_old_total_does_not_reaward_access_code() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(70, 71, 72, 73);
		register_account(&account_set, None);
		satisfy_operational_requirements(&account_set.mining_funding, &account_set.vault);

		let min_lock = 1;
		let access_code_threshold = BitcoinLockSizeForAccessCode::get();

		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(access_code_threshold),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.issuable_access_codes, 2);
		assert_eq!(operational_account.bitcoin_accrual, 0);

		set_issuable_access_codes(&account_set.owner, 1);
		OperationalAccountsPallet::bitcoin_lock_funded(&account_set.vault, min_lock);
		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(access_code_threshold),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.issuable_access_codes, 1);
		assert_eq!(operational_account.bitcoin_accrual, 0);

		OperationalAccountsPallet::bitcoin_lock_funded(
			&account_set.vault,
			min_lock.saturating_add(access_code_threshold.saturating_mul(2)),
		);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.issuable_access_codes, 2);
		assert_eq!(operational_account.bitcoin_accrual, 0);
	});
}

#[test]
fn test_access_codes_awarded_for_mining_seats() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(5, 6, 7, 8);
		register_account(&account_set, None);
		satisfy_operational_requirements(&account_set.mining_funding, &account_set.vault);
		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.issuable_access_codes, 1);

		for _ in 0..3 {
			OperationalAccountsPallet::mining_seat_won(&account_set.mining_funding);
		}

		let operational_account =
			OperationalAccounts::<Test>::get(&account_set.owner).expect("operational account");
		assert_eq!(operational_account.issuable_access_codes, 2);
	});
}

#[test]
fn test_pending_rewards_returns_all_queued_rewards() {
	new_test_ext().execute_with(|| {
		OperationalRewardsQueue::<Test>::mutate(|queue| {
			for i in 0..5u8 {
				assert!(
					queue
						.try_push(OperationalRewardPayout {
							operational_account: account_id_from_seed(i.saturating_add(1)),
							payout_account: account_id_from_seed(i.saturating_add(100)),
							reward_kind: OperationalRewardKind::Activation,
							amount: 1,
						})
						.is_ok()
				);
			}
		});

		let rewards = <OperationalAccountsPallet as OperationalRewardsProvider<
			TestAccountId,
			Balance,
		>>::pending_rewards();
		assert_eq!(rewards.len(), 5);
		assert_eq!(rewards[0].operational_account, account_id_from_seed(1));
		assert_eq!(rewards[4].operational_account, account_id_from_seed(5));
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
	fn registration(&self, access_code: Option<AccessCodeProof>) -> Registration<Test> {
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
			access_code,
		})
	}
}

fn make_access_code_proof(account: &TestAccountId, seed: u8) -> AccessCodeProof {
	let pair = sr25519::Pair::from_seed(&[seed; 32]);
	let public = pair.public();
	let message = (ACCESS_CODE_PROOF_MESSAGE_KEY, public, account).using_encoded(blake2_256);
	let signature = pair.sign(message.as_slice());
	AccessCodeProof { public, signature }
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

fn register_account(set: &AccountSet, access_code: Option<AccessCodeProof>) {
	register_account_with_submitter(set, &set.owner, access_code);
}

fn register_account_with_submitter(
	set: &AccountSet,
	submitter: &TestAccountId,
	access_code: Option<AccessCodeProof>,
) {
	assert_ok!(OperationalAccountsPallet::register(
		RuntimeOrigin::signed(submitter.clone()),
		set.registration(access_code),
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

fn set_issuable_access_codes(owner: &TestAccountId, count: u32) {
	OperationalAccounts::<Test>::mutate(owner, |maybe| {
		let account = maybe.as_mut().expect("operational account");
		account.issuable_access_codes = count;
	});
}
