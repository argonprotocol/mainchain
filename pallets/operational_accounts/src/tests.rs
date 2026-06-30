use crate::{
	AccountOwnershipProof, EncryptedServerByDownstreamAccount, Error,
	IsOperationalAccountInviteOnly, OpaqueEncryptionPubkey, OperationalAccount,
	OperationalAccountBySubAccount, OperationalAccounts, OperationalProgressPatch, Registration,
	RegistrationV1, MINING_ACCOUNT_PROOF_MESSAGE_KEY, OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY,
	VAULT_ACCOUNT_PROOF_MESSAGE_KEY,
};
use argon_primitives::{OperationalAccountsHook, Signature, UtxoLockEvents, MICROGONS_PER_ARGON};
use frame_support::{assert_noop, assert_ok};
use pallet_prelude::*;
use sp_core::{sr25519, Pair};
use sp_io::hashing::blake2_256;
use sp_runtime::{traits::IdentifyAccount, AccountId32, MultiSigner};

use crate::mock::{
	funded_bitcoin_amount, has_vault_operational_mark, new_test_ext,
	record_active_vault_bond_amount, record_funded_bitcoin_amount, record_microgons_in,
	record_microgons_out, set_argon_balance, set_crosschain_activated, set_registration_lookup,
	BitcoinLockSizeForUpgradeCode, ClaimableTreasuryBalance, ClaimedOperationalRewards,
	MaxEncryptedServerLen, OperationalAccounts as OperationalAccountsPallet,
	OperationalActivationReward, OperationalMinimumUniswapTransfer,
	OperationalMinimumVaultSecuritization, RuntimeOrigin, Test, TestAccountId,
	TreasuryMinimumBitcoin, TreasuryMinimumBonds, TreasuryMinimumUniswapTransfer,
};

#[test]
fn test_register_creates_operational_account() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(1, 2, 3);
		register_account(&account_set, None);

		let operational_account = OperationalAccounts::<Test>::get(&account_set.owner)
			.expect("operational account created");
		assert_eq!(operational_account.vault_account, account_set.vault);
		assert_eq!(operational_account.mining_account, account_set.mining);
		assert_eq!(operational_account.encryption_pubkey, account_set.encryption_pubkey);
		assert!(operational_account.referrer.is_none());
		assert!(!operational_account.is_upgraded_to_operations);
		assert!(!operational_account.is_operationally_certified);

		assert_eq!(
			OperationalAccountBySubAccount::<Test>::get(&account_set.vault),
			Some(account_set.owner.clone())
		);
		assert_eq!(
			OperationalAccountBySubAccount::<Test>::get(&account_set.mining),
			Some(account_set.owner.clone())
		);
	});
}

#[test]
fn test_register_requires_invite_when_invite_only() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(5, 6, 7);
		IsOperationalAccountInviteOnly::<Test>::put(true);

		assert_noop!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(account_set.owner.clone()),
				account_set.registration(None),
			),
			Error::<Test>::RegistrationInviteRequired
		);
	});
}

#[test]
fn test_register_allows_referrer_when_invite_only() {
	new_test_ext().execute_with(|| {
		let referrer_set = make_account_set(9, 10, 11);
		let recruit_set = make_account_set(13, 14, 15);

		register_account(&referrer_set, None);
		IsOperationalAccountInviteOnly::<Test>::put(true);

		assert_ok!(OperationalAccountsPallet::register(
			RuntimeOrigin::signed(recruit_set.owner.clone()),
			recruit_set.registration(Some(referrer_set.owner.clone())),
		));

		let recruit_account =
			OperationalAccounts::<Test>::get(&recruit_set.owner).expect("recruit account");
		assert_eq!(recruit_account.referrer, Some(referrer_set.owner));
	});
}

#[test]
fn test_register_allows_linked_account_submitter() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(11, 12, 13);

		register_account_with_submitter(&account_set, &account_set.vault, None);
		assert!(OperationalAccounts::<Test>::contains_key(&account_set.owner));
	});
}

#[test]
fn test_register_allows_vault_and_mining_account_to_alias() {
	new_test_ext().execute_with(|| {
		let mut account_set = make_account_set(21, 22, 23);
		account_set.mining = account_set.vault.clone();
		account_set.mining_proof = make_account_proof(
			&account_set.owner,
			&account_set.mining,
			22,
			MINING_ACCOUNT_PROOF_MESSAGE_KEY,
		);

		register_account(&account_set, None);
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.vault_account, account_set.vault);
		assert_eq!(account.mining_account, account_set.vault);
	});
}

#[test]
fn test_register_rejects_unknown_referrer() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(31, 32, 33);

		assert_noop!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(account_set.owner.clone()),
				account_set.registration(Some(account_id_from_seed(99))),
			),
			Error::<Test>::ReferrerNotOperationalAccount
		);
	});
}

#[test]
fn test_register_records_referrer_without_granting_operational_upgrade() {
	new_test_ext().execute_with(|| {
		let referrer_set = make_account_set(41, 42, 43);
		let recruit_set = make_account_set(45, 46, 47);
		register_account(&referrer_set, None);
		register_account(&recruit_set, Some(referrer_set.owner.clone()));

		let recruit_account =
			OperationalAccounts::<Test>::get(&recruit_set.owner).expect("recruit");
		assert_eq!(recruit_account.referrer, Some(referrer_set.owner));
		assert!(!recruit_account.is_upgraded_to_operations);
	});
}

#[test]
fn test_register_preserves_pre_registration_uniswap_argon_transfers_in() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(51, 52, 53);
		let pre_registration_amount = TreasuryMinimumUniswapTransfer::get();

		record_microgons_in(&account_set.vault, TreasuryMinimumUniswapTransfer::get());
		record_microgons_out(&account_set.vault, 1);
		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.uniswap_argon_transfers_in_amount, pre_registration_amount);
	});
}

#[test]
fn test_register_preserves_pre_registration_treasury_certification_amounts() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(55, 56, 57);

		record_microgons_in(&account_set.owner, TreasuryMinimumUniswapTransfer::get());
		record_account_bitcoin(&account_set.owner, TreasuryMinimumBitcoin::get());
		record_account_vault_bond_amount(&account_set.owner, TreasuryMinimumBonds::get());

		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(
			account.uniswap_argon_transfers_in_amount,
			TreasuryMinimumUniswapTransfer::get()
		);
		assert_eq!(account.account_bitcoin_amount, TreasuryMinimumBitcoin::get());
		assert_eq!(account.account_vault_bond_amount, TreasuryMinimumBonds::get());
		assert!(account.is_treasury_certified);
		assert_eq!(current_vault_bitcoin_amount(&account), 0);
	});
}

#[test]
fn test_register_preloads_existing_vault_bitcoin_progress() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(58, 59, 60);
		set_registration_lookup(
			account_set.vault.clone(),
			account_set.mining.clone(),
			TreasuryMinimumBitcoin::get(),
			OperationalMinimumVaultSecuritization::get(),
			0,
			0,
		);

		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(current_vault_bitcoin_amount(&account), TreasuryMinimumBitcoin::get());
	});
}

#[test]
fn test_register_loads_linked_account_uniswap_argon_transfers_in() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(61, 62, 63);

		record_microgons_in(&account_set.vault, TreasuryMinimumUniswapTransfer::get());
		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(
			account.uniswap_argon_transfers_in_amount,
			TreasuryMinimumUniswapTransfer::get()
		);
	});
}

#[test]
fn test_treasury_certification_ignores_uniswap_argon_transfers_in_when_crosschain_is_not_activated()
{
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(71, 72, 73);
		set_crosschain_activated(false);
		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.uniswap_argon_transfers_in_amount, 0);

		record_account_bitcoin(&account_set.owner, TreasuryMinimumBitcoin::get());
		record_account_vault_bond_amount(&account_set.owner, TreasuryMinimumBonds::get());

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_treasury_certified);
	});
}

#[test]
fn test_treasury_certification_uses_account_positions_in_any_vault() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(75, 76, 77);
		register_account(&account_set, None);
		set_linked_account_uniswap_argon_transfers_in_amount(
			&account_set.owner,
			TreasuryMinimumUniswapTransfer::get(),
		);

		OperationalAccountsPallet::vault_bitcoin_lock_funded(
			&account_set.vault,
			BitcoinLockSizeForUpgradeCode::get(),
		);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(!account.is_treasury_certified);
		assert_eq!(account.account_bitcoin_amount, 0);
		assert_eq!(current_vault_bitcoin_amount(&account), BitcoinLockSizeForUpgradeCode::get(),);

		record_account_bitcoin(&account_set.owner, TreasuryMinimumBitcoin::get());
		record_account_vault_bond_amount(&account_set.owner, TreasuryMinimumBonds::get());

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_treasury_certified);
		assert_eq!(account.account_bitcoin_amount, TreasuryMinimumBitcoin::get());
		assert_eq!(account.account_vault_bond_amount, TreasuryMinimumBonds::get());
	});
}

#[test]
fn test_treasury_certification_clears_when_account_bitcoin_is_released() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(78, 79, 80);
		register_account(&account_set, None);
		satisfy_treasury_requirements(&account_set.vault, &account_set.mining);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_treasury_certified);

		release_account_bitcoin(&account_set.owner);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.account_bitcoin_amount, 0);
		assert!(!account.is_treasury_certified);

		record_account_bitcoin(&account_set.owner, TreasuryMinimumBitcoin::get());

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.account_bitcoin_amount, TreasuryMinimumBitcoin::get());
		assert!(account.is_treasury_certified);
	});
}

#[test]
fn test_treasury_certification_clears_when_account_bonds_drop() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(81, 82, 83);
		register_account(&account_set, None);
		satisfy_treasury_requirements(&account_set.vault, &account_set.mining);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_treasury_certified);

		record_account_vault_bond_amount(&account_set.owner, 0);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.account_vault_bond_amount, 0);
		assert!(!account.is_treasury_certified);

		record_account_vault_bond_amount(&account_set.owner, TreasuryMinimumBonds::get());

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.account_vault_bond_amount, TreasuryMinimumBonds::get());
		assert!(account.is_treasury_certified);
	});
}

#[test]
fn test_treasury_certification_ignores_uniswap_argon_transfer_outs() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(84, 85, 86);
		register_account(&account_set, None);
		satisfy_treasury_requirements(&account_set.vault, &account_set.mining);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_treasury_certified);

		record_microgons_out(&account_set.owner, TreasuryMinimumUniswapTransfer::get());
		OperationalAccountsPallet::refresh_account_uniswap_argon_transfers_in_amount(
			&account_set.owner,
		);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(
			account.uniswap_argon_transfers_in_amount,
			TreasuryMinimumUniswapTransfer::get(),
		);
		assert!(account.is_treasury_certified);
	});
}

#[test]
fn test_force_set_progress_guardrails() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(81, 82, 83);
		register_account(&account_set, None);

		assert_noop!(
			OperationalAccountsPallet::force_set_progress(
				RuntimeOrigin::root(),
				account_set.owner.clone(),
				OperationalProgressPatch {
					uniswap_argon_transfers_in_amount: None,
					account_bitcoin_amount: None,
					account_vault_bond_amount: None,
					vault_created: None,
					vault_bitcoin_amount: None,
					mining_seat_count: None,
				},
				true,
			),
			Error::<Test>::NoProgressUpdateProvided
		);
	});
}

#[test]
fn test_force_set_progress_applies_patch_and_reconciles_totals() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(91, 92, 93);
		register_account(&account_set, None);
		set_registration_lookup(
			account_set.vault.clone(),
			account_set.mining.clone(),
			0,
			OperationalMinimumVaultSecuritization::get(),
			TreasuryMinimumBonds::get(),
			0,
		);

		assert_ok!(OperationalAccountsPallet::force_set_progress(
			RuntimeOrigin::root(),
			account_set.owner.clone(),
			OperationalProgressPatch {
				uniswap_argon_transfers_in_amount: Some(OperationalMinimumUniswapTransfer::get()),
				account_bitcoin_amount: Some(TreasuryMinimumBitcoin::get()),
				account_vault_bond_amount: Some(TreasuryMinimumBonds::get()),
				vault_created: Some(true),
				vault_bitcoin_amount: Some(BitcoinLockSizeForUpgradeCode::get()),
				mining_seat_count: Some(2),
			},
			true,
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_treasury_certified);
		assert_eq!(
			account.uniswap_argon_transfers_in_amount,
			OperationalMinimumUniswapTransfer::get()
		);
		assert_eq!(account.account_bitcoin_amount, TreasuryMinimumBitcoin::get());
		assert_eq!(account.account_vault_bond_amount, TreasuryMinimumBonds::get());
		assert_eq!(current_vault_bitcoin_amount(&account), BitcoinLockSizeForUpgradeCode::get(),);
		assert_eq!(mining_seat_count(&account), 2);
	});
}

#[test]
fn test_upgrade_account_allows_managed_account_submitter() {
	new_test_ext().execute_with(|| {
		let referrer_set = make_account_set(101, 102, 103);
		let recruit_set = make_account_set(105, 106, 107);
		register_account(&referrer_set, None);
		register_account(&recruit_set, None);
		satisfy_and_activate(&referrer_set);
		satisfy_treasury_requirements(&recruit_set.vault, &recruit_set.mining);
		set_available_upgrade_codes(&referrer_set.owner, 1);

		assert_ok!(OperationalAccountsPallet::upgrade_account(
			RuntimeOrigin::signed(referrer_set.vault.clone()),
			recruit_set.owner.clone(),
		));

		let recruit = OperationalAccounts::<Test>::get(&recruit_set.owner).expect("recruit");
		assert!(recruit.is_upgraded_to_operations);
		assert_eq!(recruit.referrer, Some(referrer_set.owner));
	});
}

#[test]
fn test_upgrade_account_rejects_wrong_registered_referrer() {
	new_test_ext().execute_with(|| {
		let referrer_set = make_account_set(111, 112, 113);
		let other_referrer_set = make_account_set(115, 116, 117);
		let recruit_set = make_account_set(119, 120, 121);
		register_account(&referrer_set, None);
		register_account(&other_referrer_set, None);
		register_account(&recruit_set, Some(referrer_set.owner.clone()));
		satisfy_and_activate(&referrer_set);
		satisfy_and_activate(&other_referrer_set);
		satisfy_treasury_requirements(&recruit_set.vault, &recruit_set.mining);
		set_available_upgrade_codes(&referrer_set.owner, 1);
		set_available_upgrade_codes(&other_referrer_set.owner, 1);

		assert_noop!(
			OperationalAccountsPallet::upgrade_account(
				RuntimeOrigin::signed(other_referrer_set.owner.clone()),
				recruit_set.owner.clone(),
			),
			Error::<Test>::RegisteredReferrerMismatch
		);
	});
}

#[test]
fn test_upgrade_account_requires_current_treasury_certification() {
	new_test_ext().execute_with(|| {
		let referrer_set = make_account_set(131, 132, 133);
		let recruit_set = make_account_set(135, 136, 137);
		register_account(&referrer_set, None);
		register_account(&recruit_set, None);
		satisfy_and_activate(&referrer_set);
		set_available_upgrade_codes(&referrer_set.owner, 1);

		assert_noop!(
			OperationalAccountsPallet::upgrade_account(
				RuntimeOrigin::signed(referrer_set.owner.clone()),
				recruit_set.owner.clone(),
			),
			Error::<Test>::NotTreasuryCertified
		);

		satisfy_treasury_requirements(&recruit_set.vault, &recruit_set.mining);
		let recruit_account =
			OperationalAccounts::<Test>::get(&recruit_set.owner).expect("recruit account");
		assert!(recruit_account.is_treasury_certified);

		release_account_bitcoin(&recruit_set.owner);
		let recruit_account =
			OperationalAccounts::<Test>::get(&recruit_set.owner).expect("recruit account");
		assert!(!recruit_account.is_treasury_certified);

		assert_noop!(
			OperationalAccountsPallet::upgrade_account(
				RuntimeOrigin::signed(referrer_set.owner.clone()),
				recruit_set.owner.clone(),
			),
			Error::<Test>::NotTreasuryCertified
		);
	});
}

#[test]
fn test_upgrade_account_rejects_when_referrer_cannot_spend_upgrade() {
	new_test_ext().execute_with(|| {
		let referrer_set = make_account_set(141, 142, 143);
		let recruit_set = make_account_set(145, 146, 147);
		let missing_referrer = account_id_from_seed(149);
		register_account(&referrer_set, None);
		register_account(&recruit_set, None);
		satisfy_and_activate(&referrer_set);
		satisfy_treasury_requirements(&recruit_set.vault, &recruit_set.mining);
		set_available_upgrade_codes(&referrer_set.owner, 0);

		assert_noop!(
			OperationalAccountsPallet::upgrade_account(
				RuntimeOrigin::signed(referrer_set.owner.clone()),
				recruit_set.owner.clone(),
			),
			Error::<Test>::NoAvailableUpgrades
		);

		assert_noop!(
			OperationalAccountsPallet::upgrade_account(
				RuntimeOrigin::signed(missing_referrer),
				recruit_set.owner.clone(),
			),
			Error::<Test>::ReferrerNotOperationalAccount
		);
	});
}

#[test]
fn test_activate_requires_current_treasury_certification() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(149, 150, 151);
		register_account(&account_set, None);
		satisfy_treasury_requirements(&account_set.vault, &account_set.mining);
		mark_upgraded_to_operational(&account_set.owner);
		satisfy_operational_requirements(&account_set.mining, &account_set.vault);

		release_account_bitcoin(&account_set.owner);

		assert_noop!(
			OperationalAccountsPallet::activate(RuntimeOrigin::signed(account_set.mining.clone())),
			Error::<Test>::TreasuryCertificationNoLongerMet
		);
	});
}

#[test]
fn test_activate_requires_minimum_uniswap_argon_transfers_in() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(151, 152, 153);
		register_account(&account_set, None);
		satisfy_treasury_requirements(&account_set.vault, &account_set.mining);
		mark_upgraded_to_operational(&account_set.owner);
		set_registration_lookup(
			account_set.vault.clone(),
			account_set.mining.clone(),
			TreasuryMinimumBitcoin::get(),
			OperationalMinimumVaultSecuritization::get(),
			TreasuryMinimumBonds::get(),
			0,
		);
		OperationalAccountsPallet::vault_created(&account_set.vault);
		OperationalAccountsPallet::mining_seat_won(&account_set.mining);
		OperationalAccountsPallet::mining_seat_won(&account_set.mining);

		assert_noop!(
			OperationalAccountsPallet::activate(RuntimeOrigin::signed(account_set.mining.clone(),)),
			Error::<Test>::NotEligibleForActivation
		);

		set_linked_account_uniswap_argon_transfers_in_amount(
			&account_set.vault,
			OperationalMinimumUniswapTransfer::get()
				.saturating_sub(TreasuryMinimumUniswapTransfer::get()),
		);

		assert_ok!(OperationalAccountsPallet::activate(RuntimeOrigin::signed(
			account_set.mining.clone(),
		)));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(account.is_operationally_certified);
		assert!(has_vault_operational_mark(&account_set.vault));
	});
}

#[test]
fn test_claim_rewards_rejects_invalid_claims() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(161, 162, 163);
		register_account(&account_set, None);
		seed_pending_rewards(&account_set.owner, 2 * MICROGONS_PER_ARGON);

		assert_noop!(
			OperationalAccountsPallet::claim_rewards(
				RuntimeOrigin::signed(account_set.vault.clone()),
				0,
			),
			Error::<Test>::RewardClaimBelowMinimum
		);
		assert_noop!(
			OperationalAccountsPallet::claim_rewards(
				RuntimeOrigin::signed(account_set.vault.clone()),
				MICROGONS_PER_ARGON + 1,
			),
			Error::<Test>::RewardClaimNotWholeArgon
		);
		assert_noop!(
			OperationalAccountsPallet::claim_rewards(
				RuntimeOrigin::signed(account_set.vault.clone()),
				3 * MICROGONS_PER_ARGON,
			),
			Error::<Test>::RewardClaimExceedsPending
		);
	});
}

#[test]
fn test_claim_rewards_pays_to_managed_signer_and_decrements_pending() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(171, 172, 173);
		register_account(&account_set, None);
		seed_pending_rewards(&account_set.owner, 2 * MICROGONS_PER_ARGON);
		ClaimableTreasuryBalance::set(2 * MICROGONS_PER_ARGON);

		assert_ok!(OperationalAccountsPallet::claim_rewards(
			RuntimeOrigin::signed(account_set.vault.clone()),
			MICROGONS_PER_ARGON,
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(pending_rewards_amount(&account), MICROGONS_PER_ARGON);
		assert_eq!(
			ClaimedOperationalRewards::get(),
			vec![(account_set.vault, MICROGONS_PER_ARGON)]
		);
	});
}

#[test]
fn test_referrer_can_set_encrypted_server_for_downstream_account() {
	new_test_ext().execute_with(|| {
		let referrer_set = make_account_set(181, 182, 183);
		let recruit_set = make_account_set(185, 186, 187);
		register_account(&referrer_set, None);
		register_account(&recruit_set, Some(referrer_set.owner.clone()));

		let encrypted_server = vec![7u8; 32];
		assert_ok!(OperationalAccountsPallet::set_encrypted_server_for_downstream_account(
			RuntimeOrigin::signed(referrer_set.owner.clone()),
			recruit_set.owner.clone(),
			encrypted_server.clone(),
		));

		assert_eq!(
			EncryptedServerByDownstreamAccount::<Test>::get(&recruit_set.owner)
				.expect("payload stored")
				.to_vec(),
			encrypted_server
		);
	});
}

#[test]
fn test_set_encrypted_server_requires_referrer_relationship() {
	new_test_ext().execute_with(|| {
		let referrer_set = make_account_set(191, 192, 193);
		let recruit_set = make_account_set(195, 196, 197);
		register_account(&referrer_set, None);
		register_account(&recruit_set, None);

		assert_noop!(
			OperationalAccountsPallet::set_encrypted_server_for_downstream_account(
				RuntimeOrigin::signed(referrer_set.owner.clone()),
				recruit_set.owner.clone(),
				vec![1u8; 32],
			),
			Error::<Test>::NotReferrerOfAccount
		);

		assert_noop!(
			OperationalAccountsPallet::set_encrypted_server_for_downstream_account(
				RuntimeOrigin::signed(referrer_set.owner.clone()),
				recruit_set.owner.clone(),
				vec![0u8; MaxEncryptedServerLen::get() as usize + 1],
			),
			Error::<Test>::NotReferrerOfAccount
		);
	});
}

#[test]
fn test_follow_on_upgrade_codes_only_count_vault_bitcoin() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(151, 152, 153);
		register_account(&account_set, None);
		satisfy_and_activate(&account_set);
		set_available_upgrade_codes(&account_set.owner, 0);
		let prior_account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		let prior_account_bitcoin_amount = prior_account.account_bitcoin_amount;

		record_account_bitcoin(&account_set.vault, BitcoinLockSizeForUpgradeCode::get());
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		let prior_vault_bitcoin_amount = current_vault_bitcoin_amount(&account);
		assert_eq!(account.available_upgrade_codes, 0);
		assert_eq!(
			account.account_bitcoin_amount,
			prior_account_bitcoin_amount.saturating_add(BitcoinLockSizeForUpgradeCode::get()),
		);

		OperationalAccountsPallet::vault_bitcoin_lock_funded(
			&account_set.vault,
			BitcoinLockSizeForUpgradeCode::get(),
		);
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.available_upgrade_codes, 1);
		assert_eq!(
			current_vault_bitcoin_amount(&account),
			prior_vault_bitcoin_amount.saturating_add(BitcoinLockSizeForUpgradeCode::get()),
		);
	});
}

#[test]
fn test_activate_records_activation_reward() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(201, 202, 203);
		register_account(&account_set, None);
		satisfy_and_activate(&account_set);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(pending_rewards_amount(&account), OperationalActivationReward::get());
	});
}

#[derive(Clone)]
struct AccountSet {
	owner: TestAccountId,
	vault: TestAccountId,
	mining: TestAccountId,
	encryption_pubkey: OpaqueEncryptionPubkey,
	owner_proof: AccountOwnershipProof,
	vault_proof: AccountOwnershipProof,
	mining_proof: AccountOwnershipProof,
}

impl AccountSet {
	fn registration(&self, referrer: Option<TestAccountId>) -> Registration<Test> {
		Registration::V1(RegistrationV1 {
			operational_account: self.owner.clone(),
			encryption_pubkey: self.encryption_pubkey.clone(),
			operational_account_proof: self.owner_proof.clone(),
			vault_account: self.vault.clone(),
			mining_account: self.mining.clone(),
			vault_account_proof: self.vault_proof.clone(),
			mining_account_proof: self.mining_proof.clone(),
			referrer,
		})
	}
}

fn seed_pending_rewards(owner: &TestAccountId, amount: Balance) {
	OperationalAccounts::<Test>::mutate(owner, |maybe| {
		let account = maybe.as_mut().expect("operational account");
		account.rewards_earned_amount = account.rewards_collected_amount.saturating_add(amount);
	});
}

fn pending_rewards_amount(account: &OperationalAccount<Test>) -> Balance {
	account.rewards_earned_amount.saturating_sub(account.rewards_collected_amount)
}

fn current_vault_bitcoin_amount(account: &OperationalAccount<Test>) -> Balance {
	account
		.vault_bitcoin_applied_total
		.saturating_add(account.vault_bitcoin_accrual)
}

fn mining_seat_count(account: &OperationalAccount<Test>) -> u32 {
	account.mining_seat_applied_total.saturating_add(account.mining_seat_accrual)
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

fn make_account_set(owner_seed: u8, vault_seed: u8, mining_seed: u8) -> AccountSet {
	let owner = account_id_from_seed(owner_seed);
	let owner_proof =
		make_account_proof(&owner, &owner, owner_seed, OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY);
	let encryption_pubkey = OpaqueEncryptionPubkey([owner_seed; 32]);
	let (vault, vault_proof) =
		make_linked_account(&owner, vault_seed, VAULT_ACCOUNT_PROOF_MESSAGE_KEY);
	let (mining, mining_proof) =
		make_linked_account(&owner, mining_seed, MINING_ACCOUNT_PROOF_MESSAGE_KEY);
	AccountSet { owner, vault, mining, encryption_pubkey, owner_proof, vault_proof, mining_proof }
}

fn register_account(set: &AccountSet, referrer: Option<TestAccountId>) {
	register_account_with_submitter(set, &set.owner, referrer);
}

fn register_account_with_submitter(
	set: &AccountSet,
	submitter: &TestAccountId,
	referrer: Option<TestAccountId>,
) {
	set_argon_balance(
		&set.owner,
		OperationalMinimumUniswapTransfer::get().saturating_add(TreasuryMinimumBitcoin::get()),
	);
	assert_ok!(OperationalAccountsPallet::register(
		RuntimeOrigin::signed(submitter.clone()),
		set.registration(referrer),
	));
}

fn set_linked_account_uniswap_argon_transfers_in_amount(
	account_id: &TestAccountId,
	amount: Balance,
) {
	record_microgons_in(account_id, amount);
	OperationalAccountsPallet::refresh_account_uniswap_argon_transfers_in_amount(account_id);
}

fn treasury_utxo_id(account_id: &TestAccountId) -> u64 {
	let account_bytes: &[u8] = account_id.as_ref();
	u64::from_le_bytes(account_bytes[0..8].try_into().expect("utxo id bytes"))
}

fn treasury_source_vault_id(account_id: &TestAccountId) -> u32 {
	let account_bytes: &[u8] = account_id.as_ref();
	u32::from_le_bytes(account_bytes[0..4].try_into().expect("vault id bytes"))
}

fn record_account_bitcoin(account_id: &TestAccountId, amount: Balance) {
	record_funded_bitcoin_amount(
		account_id,
		funded_bitcoin_amount(account_id).saturating_add(amount),
	);
	assert_ok!(OperationalAccountsPallet::utxo_locked(
		treasury_utxo_id(account_id),
		account_id,
		amount,
	));
}

fn release_account_bitcoin(account_id: &TestAccountId) {
	let amount = funded_bitcoin_amount(account_id);
	record_funded_bitcoin_amount(account_id, 0);
	assert_ok!(OperationalAccountsPallet::utxo_released(
		treasury_utxo_id(account_id),
		account_id,
		false,
		amount,
		amount,
	));
}

fn record_account_vault_bond_amount(account_id: &TestAccountId, amount: Balance) {
	record_active_vault_bond_amount(treasury_source_vault_id(account_id), account_id, amount);
	OperationalAccountsPallet::account_vault_bond_total_updated(account_id, amount);
}

fn mark_upgraded_to_operational(owner: &TestAccountId) {
	OperationalAccounts::<Test>::mutate(owner, |maybe| {
		let account = maybe.as_mut().expect("operational account");
		account.is_upgraded_to_operations = true;
	});
}

fn satisfy_treasury_requirements(vault_account: &TestAccountId, mining_account: &TestAccountId) {
	set_registration_lookup(
		vault_account.clone(),
		mining_account.clone(),
		0,
		OperationalMinimumVaultSecuritization::get(),
		0,
		0,
	);
	let owner =
		OperationalAccountBySubAccount::<Test>::get(vault_account).expect("operational owner");
	set_linked_account_uniswap_argon_transfers_in_amount(
		&owner,
		TreasuryMinimumUniswapTransfer::get(),
	);
	record_account_bitcoin(&owner, TreasuryMinimumBitcoin::get());
	record_account_vault_bond_amount(&owner, TreasuryMinimumBonds::get());
}

fn satisfy_operational_requirements(mining_account: &TestAccountId, vault_account: &TestAccountId) {
	set_registration_lookup(
		vault_account.clone(),
		mining_account.clone(),
		TreasuryMinimumBitcoin::get(),
		OperationalMinimumVaultSecuritization::get(),
		TreasuryMinimumBonds::get(),
		0,
	);
	OperationalAccountsPallet::vault_created(vault_account);
	set_linked_account_uniswap_argon_transfers_in_amount(
		vault_account,
		OperationalMinimumUniswapTransfer::get()
			.saturating_sub(TreasuryMinimumUniswapTransfer::get()),
	);
	OperationalAccountsPallet::mining_seat_won(mining_account);
	OperationalAccountsPallet::mining_seat_won(mining_account);
}

fn satisfy_and_activate(set: &AccountSet) {
	satisfy_treasury_requirements(&set.vault, &set.mining);
	mark_upgraded_to_operational(&set.owner);
	satisfy_operational_requirements(&set.mining, &set.vault);
	assert_ok!(OperationalAccountsPallet::activate(RuntimeOrigin::signed(set.mining.clone())));
}

fn set_available_upgrade_codes(owner: &TestAccountId, count: u32) {
	OperationalAccounts::<Test>::mutate(owner, |maybe| {
		let account = maybe.as_mut().expect("operational account");
		account.available_upgrade_codes = count;
	});
}
