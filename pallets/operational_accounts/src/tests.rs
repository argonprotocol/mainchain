use crate::{
	AccessCodeReadyAccounts, AccountOwnershipProof, Error, IsOperationalAccountInviteOnly,
	OpaqueEncryptionPubkey, OperationalAccount, OperationalAccountBySubAccount,
	OperationalAccountName, OperationalAccounts, OperationalProgressPatch, Registration,
	RegistrationV1, UpstreamAccessProof, MINING_ACCOUNT_PROOF_MESSAGE_KEY,
	OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY, UPSTREAM_ACCESS_PROOF_MESSAGE_KEY,
	VAULT_ACCOUNT_PROOF_MESSAGE_KEY,
};
use argon_primitives::{
	OnNewSlot, OperationalAccountProvider, OperationalAccountsHook, Signature, UtxoLockEvents,
	MICROGONS_PER_ARGON,
};
use frame_support::{assert_noop, assert_ok};
use pallet_prelude::*;
use sp_core::{sr25519, Pair};
use sp_io::hashing::blake2_256;
use sp_runtime::{traits::IdentifyAccount, AccountId32, MultiSigner};

use crate::mock::{
	funded_bitcoin_amount, has_vault_operational_mark, new_test_ext,
	record_active_vault_bond_amount, record_funded_bitcoin_amount, record_microgons_in,
	record_microgons_out, set_argon_balance, set_crosschain_activated, set_registration_lookup,
	BitcoinLockSizeForAccessCode, ClaimableTreasuryBalance, ClaimedOperationalRewards, CurrentTick,
	MaxAvailableAccessCodes, MinimumBitcoin, MinimumBonds, MinimumUniswapTransfer,
	MiningSeatsPerAccessCode, OperationalAccounts as OperationalAccountsPallet,
	OperationalCertificationReward, OperationalMinimumUniswapTransfer,
	OperationalMinimumVaultSecuritization, RuntimeOrigin, Test, TestAccountId,
};

#[test]
fn test_register_creates_operational_account() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(1, 2, 3);
		register_account(&account_set, None);

		let operational_account = OperationalAccounts::<Test>::get(&account_set.owner)
			.expect("operational account created");
		assert_eq!(AccessCodeReadyAccounts::<Test>::count(), 0);
		assert_eq!(operational_account.vault_account, account_set.vault);
		assert_eq!(operational_account.mining_account, account_set.mining);
		assert_eq!(operational_account.encryption_pubkey, account_set.encryption_pubkey);
		assert!(operational_account.upstream_account.is_none());
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
fn test_operational_account_can_update_profile_name() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(4, 5, 6);
		register_account(&account_set, None);
		let first_name = operational_account_name("VaultAlpha1");

		CurrentTick::set(10);
		assert_ok!(OperationalAccountsPallet::set_name(
			RuntimeOrigin::signed(account_set.vault.clone()),
			Some(first_name.clone()),
		));
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.name, Some(first_name.clone()));
		assert_eq!(account.last_name_change_tick, Some(10));

		CurrentTick::set(11);
		assert_ok!(OperationalAccountsPallet::set_name(
			RuntimeOrigin::signed(account_set.mining.clone()),
			Some(first_name),
		));
		assert_eq!(
			OperationalAccounts::<Test>::get(&account_set.owner)
				.expect("account")
				.last_name_change_tick,
			Some(10)
		);

		CurrentTick::set(12);
		assert_ok!(OperationalAccountsPallet::set_name(
			RuntimeOrigin::signed(account_set.owner.clone()),
			None,
		));
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.name, None);
		assert_eq!(account.last_name_change_tick, Some(12));

		assert_noop!(
			OperationalAccountsPallet::set_name(
				RuntimeOrigin::signed(account_id_from_seed(99)),
				Some(operational_account_name("VaultBeta2")),
			),
			Error::<Test>::NotOperationalAccount
		);

		assert_noop!(
			OperationalAccountsPallet::set_name(
				RuntimeOrigin::signed(account_set.owner.clone()),
				Some(operational_account_name("")),
			),
			Error::<Test>::InvalidName
		);
		assert_noop!(
			OperationalAccountsPallet::set_name(
				RuntimeOrigin::signed(account_set.owner.clone()),
				Some(operational_account_name("vault")),
			),
			Error::<Test>::InvalidName
		);
		assert_noop!(
			OperationalAccountsPallet::set_name(
				RuntimeOrigin::signed(account_set.owner.clone()),
				Some(operational_account_name("Vault-1")),
			),
			Error::<Test>::InvalidName
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
fn test_register_requires_minimums() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(8, 9, 10);

		assert_noop!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(account_set.owner.clone()),
				account_set.registration(None),
			),
			Error::<Test>::MinimumsNotMet
		);
	});
}

#[test]
fn test_register_allows_one_argon_bitcoin_shortfall_but_not_more() {
	new_test_ext().execute_with(|| {
		let within_tolerance = make_account_set(201, 202, 203);
		record_microgons_in(&within_tolerance.owner, MinimumUniswapTransfer::get());
		record_account_bitcoin(
			&within_tolerance.vault,
			MinimumBitcoin::get().saturating_sub(Balance::from(MICROGONS_PER_ARGON)),
		);
		record_account_vault_bond_amount(&within_tolerance.vault, MinimumBonds::get());

		assert_ok!(OperationalAccountsPallet::register(
			RuntimeOrigin::signed(within_tolerance.owner.clone()),
			within_tolerance.registration(None),
		));

		let beyond_tolerance = make_account_set(205, 206, 207);
		record_microgons_in(&beyond_tolerance.owner, MinimumUniswapTransfer::get());
		record_account_bitcoin(
			&beyond_tolerance.vault,
			MinimumBitcoin::get()
				.saturating_sub(Balance::from(MICROGONS_PER_ARGON))
				.saturating_sub(1),
		);
		record_account_vault_bond_amount(&beyond_tolerance.vault, MinimumBonds::get());

		assert_noop!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(beyond_tolerance.owner.clone()),
				beyond_tolerance.registration(None),
			),
			Error::<Test>::MinimumsNotMet
		);
	});
}

#[test]
fn test_referred_operational_account_replenishes_upstream_access_code() {
	new_test_ext().execute_with(|| {
		let upstream_set = make_account_set(9, 10, 11);
		let downstream_set = make_account_set(13, 14, 15);

		seed_vault_registration(&upstream_set);
		register_account(&upstream_set, None);
		satisfy_and_activate(&upstream_set);
		IsOperationalAccountInviteOnly::<Test>::put(true);
		let access_proof = make_access_proof(&upstream_set.owner, 9, &downstream_set.owner);

		register_account(&downstream_set, Some(access_proof));

		let upstream_account =
			OperationalAccounts::<Test>::get(&upstream_set.owner).expect("upstream account");
		assert_eq!(upstream_account.available_access_codes, 0);

		satisfy_and_activate(&downstream_set);

		let downstream_account =
			OperationalAccounts::<Test>::get(&downstream_set.owner).expect("downstream account");
		assert_eq!(downstream_account.upstream_account, Some(upstream_set.owner.clone()));
		assert_eq!(downstream_account.available_access_codes, 1);
		let upstream_account =
			OperationalAccounts::<Test>::get(&upstream_set.owner).expect("upstream account");
		assert_eq!(upstream_account.available_access_codes, 1);
	});
}

#[test]
fn test_register_requires_available_access_code_when_invite_only() {
	new_test_ext().execute_with(|| {
		let upstream_set = make_account_set(16, 17, 18);
		let downstream_set = make_account_set(19, 20, 21);

		seed_vault_registration(&upstream_set);
		register_account(&upstream_set, None);
		IsOperationalAccountInviteOnly::<Test>::put(true);
		let access_proof = make_access_proof(&upstream_set.owner, 16, &downstream_set.owner);

		assert_noop!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(downstream_set.owner.clone()),
				downstream_set.registration(Some(access_proof)),
			),
			Error::<Test>::UpstreamHasNoAvailableAccessCodes
		);
		assert!(!OperationalAccounts::<Test>::contains_key(&downstream_set.owner));
	});
}

#[test]
fn test_invite_only_eligibility_requires_registration() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(17, 18, 19);
		register_account(&account_set, None);
		IsOperationalAccountInviteOnly::<Test>::put(true);

		assert!(
			<OperationalAccountsPallet as OperationalAccountProvider<TestAccountId>>::is_eligible(
				&account_set.owner,
			)
		);
		assert!(
			<OperationalAccountsPallet as OperationalAccountProvider<TestAccountId>>::is_eligible(
				&account_set.vault,
			)
		);
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
fn test_register_rejects_unknown_upstream_account() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(31, 32, 33);
		seed_registration_minimums(&account_set);
		set_argon_balance(
			&account_set.owner,
			OperationalMinimumUniswapTransfer::get().saturating_add(MinimumBitcoin::get()),
		);
		let unknown_upstream = account_id_from_seed(99);
		let access_proof = make_access_proof(&unknown_upstream, 99, &account_set.owner);

		assert_noop!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(account_set.owner.clone()),
				account_set.registration(Some(access_proof)),
			),
			Error::<Test>::UpstreamNotOperationalAccount
		);
	});
}

#[test]
fn test_register_records_upstream_account() {
	new_test_ext().execute_with(|| {
		let upstream_set = make_account_set(41, 42, 43);
		let downstream_set = make_account_set(45, 46, 47);
		seed_vault_registration(&upstream_set);
		register_account(&upstream_set, None);
		satisfy_and_activate(&upstream_set);
		register_account(
			&downstream_set,
			Some(make_access_proof(&upstream_set.owner, 41, &downstream_set.owner)),
		);

		let downstream_account =
			OperationalAccounts::<Test>::get(&downstream_set.owner).expect("downstream");
		assert_eq!(downstream_account.upstream_account, Some(upstream_set.owner));
	});
}

#[test]
fn test_access_registration_queues_ready_accrual_access_code() {
	new_test_ext().execute_with(|| {
		let upstream_set = make_account_set(48, 49, 50);
		let downstream_set = make_account_set(51, 52, 53);
		seed_vault_registration(&upstream_set);
		register_account(&upstream_set, None);
		satisfy_and_activate(&upstream_set);
		let bitcoin_threshold = BitcoinLockSizeForAccessCode::get();
		OperationalAccounts::<Test>::mutate(&upstream_set.owner, |maybe| {
			let upstream_account = maybe.as_mut().expect("upstream account");
			upstream_account.available_access_codes = MaxAvailableAccessCodes::get();
			upstream_account.mining_seat_applied_total = mining_seat_count(upstream_account);
			upstream_account.mining_seat_accrual = 0;
		});
		for _ in 0..MiningSeatsPerAccessCode::get() {
			OperationalAccountsPallet::mining_seat_won(&upstream_set.mining);
		}
		OperationalAccountsPallet::vault_bitcoin_lock_funded(
			&upstream_set.vault,
			bitcoin_threshold,
		);
		assert!(!AccessCodeReadyAccounts::<Test>::contains_key(&upstream_set.owner));

		register_account(
			&downstream_set,
			Some(make_access_proof(&upstream_set.owner, 48, &downstream_set.owner)),
		);

		let downstream_account =
			OperationalAccounts::<Test>::get(&downstream_set.owner).expect("downstream account");
		assert_eq!(downstream_account.upstream_account, Some(upstream_set.owner.clone()));
		let upstream_account =
			OperationalAccounts::<Test>::get(&upstream_set.owner).expect("upstream account");
		assert_eq!(upstream_account.available_access_codes, 1);
		assert_eq!(upstream_account.vault_bitcoin_accrual, bitcoin_threshold);
		assert_eq!(upstream_account.mining_seat_accrual, MiningSeatsPerAccessCode::get());
		assert!(AccessCodeReadyAccounts::<Test>::contains_key(&upstream_set.owner));

		<OperationalAccountsPallet as OnNewSlot<TestAccountId>>::on_frame_start(1);
		let upstream_account =
			OperationalAccounts::<Test>::get(&upstream_set.owner).expect("upstream account");
		assert_eq!(upstream_account.available_access_codes, MaxAvailableAccessCodes::get());
		assert_eq!(upstream_account.vault_bitcoin_accrual, 0);
		assert_eq!(upstream_account.mining_seat_accrual, 0);
	});
}

#[test]
fn test_register_preserves_pre_registration_uniswap_argon_transfers_in() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(51, 52, 53);
		let pre_registration_amount = MinimumUniswapTransfer::get();

		record_microgons_in(&account_set.vault, MinimumUniswapTransfer::get());
		record_microgons_out(&account_set.vault, 1);
		record_account_bitcoin(&account_set.vault, MinimumBitcoin::get());
		record_account_vault_bond_amount(&account_set.vault, MinimumBonds::get());
		set_argon_balance(
			&account_set.owner,
			OperationalMinimumUniswapTransfer::get().saturating_add(MinimumBitcoin::get()),
		);
		assert_ok!(OperationalAccountsPallet::register(
			RuntimeOrigin::signed(account_set.owner.clone()),
			account_set.registration(None),
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.uniswap_argon_transfers_in_amount, pre_registration_amount);
	});
}

#[test]
fn test_register_preserves_pre_registration_minimum_amounts() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(55, 56, 57);

		record_microgons_in(&account_set.owner, MinimumUniswapTransfer::get());
		record_account_bitcoin(&account_set.vault, MinimumBitcoin::get());
		record_account_vault_bond_amount(&account_set.vault, MinimumBonds::get());
		set_argon_balance(
			&account_set.owner,
			OperationalMinimumUniswapTransfer::get().saturating_add(MinimumBitcoin::get()),
		);
		assert_ok!(OperationalAccountsPallet::register(
			RuntimeOrigin::signed(account_set.owner.clone()),
			account_set.registration(None),
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.uniswap_argon_transfers_in_amount, MinimumUniswapTransfer::get());
		assert_eq!(account.account_bitcoin_amount, MinimumBitcoin::get());
		assert_eq!(account.account_vault_bond_amount, MinimumBonds::get());
		assert!(meets_minimums(&account));
		assert_eq!(current_vault_bitcoin_amount(&account), 0);
	});
}

#[test]
fn test_register_ignores_non_vault_bitcoin_and_bonds() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(58, 59, 60);

		record_microgons_in(&account_set.owner, MinimumUniswapTransfer::get());
		record_account_bitcoin(&account_set.owner, MinimumBitcoin::get());
		record_account_vault_bond_amount(&account_set.owner, MinimumBonds::get());
		set_argon_balance(
			&account_set.owner,
			OperationalMinimumUniswapTransfer::get().saturating_add(MinimumBitcoin::get()),
		);

		assert_noop!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(account_set.owner.clone()),
				account_set.registration(None),
			),
			Error::<Test>::MinimumsNotMet
		);
	});
}

#[test]
fn test_register_preloads_existing_vault_bitcoin_progress() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(61, 62, 63);
		set_registration_lookup(
			account_set.vault.clone(),
			account_set.mining.clone(),
			MinimumBitcoin::get(),
			OperationalMinimumVaultSecuritization::get(),
			0,
			0,
		);
		record_microgons_in(&account_set.owner, MinimumUniswapTransfer::get());
		record_account_bitcoin(&account_set.vault, MinimumBitcoin::get());
		record_account_vault_bond_amount(&account_set.vault, MinimumBonds::get());
		set_argon_balance(
			&account_set.owner,
			OperationalMinimumUniswapTransfer::get().saturating_add(MinimumBitcoin::get()),
		);

		assert_ok!(OperationalAccountsPallet::register(
			RuntimeOrigin::signed(account_set.owner.clone()),
			account_set.registration(None),
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(current_vault_bitcoin_amount(&account), MinimumBitcoin::get());
	});
}

#[test]
fn test_register_loads_linked_account_uniswap_argon_transfers_in() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(64, 65, 66);

		record_microgons_in(&account_set.vault, MinimumUniswapTransfer::get());
		record_account_bitcoin(&account_set.vault, MinimumBitcoin::get());
		record_account_vault_bond_amount(&account_set.vault, MinimumBonds::get());
		set_argon_balance(
			&account_set.owner,
			OperationalMinimumUniswapTransfer::get().saturating_add(MinimumBitcoin::get()),
		);
		assert_ok!(OperationalAccountsPallet::register(
			RuntimeOrigin::signed(account_set.owner.clone()),
			account_set.registration(None),
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.uniswap_argon_transfers_in_amount, MinimumUniswapTransfer::get());
	});
}

#[test]
fn test_minimums_ignore_uniswap_argon_transfers_in_when_crosschain_is_not_activated() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(74, 75, 76);
		set_crosschain_activated(false);
		record_account_bitcoin(&account_set.vault, MinimumBitcoin::get());
		record_account_vault_bond_amount(&account_set.vault, MinimumBonds::get());
		set_argon_balance(
			&account_set.owner,
			OperationalMinimumUniswapTransfer::get().saturating_add(MinimumBitcoin::get()),
		);
		assert_ok!(OperationalAccountsPallet::register(
			RuntimeOrigin::signed(account_set.owner.clone()),
			account_set.registration(None),
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.uniswap_argon_transfers_in_amount, 0);
		assert!(meets_minimums(&account));
	});
}

#[test]
fn test_minimums_use_account_bitcoin_not_vault_bitcoin_progress() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(78, 79, 80);
		register_account(&account_set, None);
		release_account_bitcoin(&account_set.vault);

		OperationalAccountsPallet::vault_bitcoin_lock_funded(
			&account_set.vault,
			BitcoinLockSizeForAccessCode::get(),
		);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(!meets_minimums(&account));
		assert_eq!(account.account_bitcoin_amount, 0);
		assert_eq!(current_vault_bitcoin_amount(&account), BitcoinLockSizeForAccessCode::get(),);

		record_account_bitcoin(&account_set.vault, MinimumBitcoin::get());

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(meets_minimums(&account));
		assert_eq!(account.account_bitcoin_amount, MinimumBitcoin::get());
	});
}

#[test]
fn test_minimums_clear_when_account_bitcoin_is_released() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(81, 82, 83);
		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(meets_minimums(&account));

		release_account_bitcoin(&account_set.vault);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.account_bitcoin_amount, 0);
		assert!(!meets_minimums(&account));

		record_account_bitcoin(&account_set.vault, MinimumBitcoin::get());

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.account_bitcoin_amount, MinimumBitcoin::get());
		assert!(meets_minimums(&account));
	});
}

#[test]
fn test_minimums_clear_when_account_bonds_drop() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(84, 85, 86);
		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(meets_minimums(&account));

		record_account_vault_bond_amount(&account_set.vault, 0);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.account_vault_bond_amount, 0);
		assert!(!meets_minimums(&account));

		record_account_vault_bond_amount(&account_set.vault, MinimumBonds::get());

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.account_vault_bond_amount, MinimumBonds::get());
		assert!(meets_minimums(&account));
	});
}

#[test]
fn test_minimums_ignore_uniswap_argon_transfer_outs() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(87, 88, 89);
		register_account(&account_set, None);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(meets_minimums(&account));

		record_microgons_out(&account_set.owner, MinimumUniswapTransfer::get());
		OperationalAccountsPallet::refresh_account_uniswap_argon_transfers_in_amount(
			&account_set.owner,
		);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.uniswap_argon_transfers_in_amount, MinimumUniswapTransfer::get(),);
		assert!(meets_minimums(&account));
	});
}

#[test]
fn test_minimums_ignore_operational_owner_holdings_after_registration() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(90, 91, 92);
		register_account(&account_set, None);
		release_account_bitcoin(&account_set.vault);
		record_account_vault_bond_amount(&account_set.vault, 0);

		record_account_bitcoin(&account_set.owner, MinimumBitcoin::get());
		record_account_vault_bond_amount(&account_set.owner, MinimumBonds::get());

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.account_bitcoin_amount, 0);
		assert_eq!(account.account_vault_bond_amount, 0);
		assert!(!meets_minimums(&account));
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
			MinimumBonds::get(),
			0,
		);

		assert_ok!(OperationalAccountsPallet::force_set_progress(
			RuntimeOrigin::root(),
			account_set.owner.clone(),
			OperationalProgressPatch {
				uniswap_argon_transfers_in_amount: Some(OperationalMinimumUniswapTransfer::get()),
				account_bitcoin_amount: Some(MinimumBitcoin::get()),
				account_vault_bond_amount: Some(MinimumBonds::get()),
				vault_created: Some(true),
				vault_bitcoin_amount: Some(BitcoinLockSizeForAccessCode::get()),
				mining_seat_count: Some(2),
			},
			true,
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert!(meets_minimums(&account));
		assert_eq!(
			account.uniswap_argon_transfers_in_amount,
			OperationalMinimumUniswapTransfer::get()
		);
		assert_eq!(account.account_bitcoin_amount, MinimumBitcoin::get());
		assert_eq!(account.account_vault_bond_amount, MinimumBonds::get());
		assert_eq!(current_vault_bitcoin_amount(&account), BitcoinLockSizeForAccessCode::get(),);
		assert_eq!(mining_seat_count(&account), 2);
		assert_ok!(OperationalAccountsPallet::activate(RuntimeOrigin::signed(
			account_set.owner.clone(),
		)));

		assert_ok!(OperationalAccountsPallet::force_set_progress(
			RuntimeOrigin::root(),
			account_set.owner.clone(),
			OperationalProgressPatch {
				uniswap_argon_transfers_in_amount: None,
				account_bitcoin_amount: None,
				account_vault_bond_amount: None,
				vault_created: None,
				vault_bitcoin_amount: None,
				mining_seat_count: Some(MiningSeatsPerAccessCode::get()),
			},
			false,
		));
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.available_access_codes, 1);
		assert!(!AccessCodeReadyAccounts::<Test>::contains_key(&account_set.owner));

		assert_ok!(OperationalAccountsPallet::force_set_progress(
			RuntimeOrigin::root(),
			account_set.owner.clone(),
			OperationalProgressPatch {
				uniswap_argon_transfers_in_amount: None,
				account_bitcoin_amount: None,
				account_vault_bond_amount: None,
				vault_created: None,
				vault_bitcoin_amount: None,
				mining_seat_count: Some(MiningSeatsPerAccessCode::get()),
			},
			true,
		));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.available_access_codes, 2);
		assert_eq!(account.vault_bitcoin_accrual, 0);
		assert_eq!(account.mining_seat_accrual, 0);
	});
}

#[test]
fn test_register_accepts_signed_access_proof() {
	new_test_ext().execute_with(|| {
		let upstream_set = make_account_set(101, 102, 103);
		let downstream_set = make_account_set(105, 106, 107);
		seed_vault_registration(&upstream_set);
		register_account(&upstream_set, None);
		satisfy_and_activate(&upstream_set);
		let access_proof = make_access_proof(&upstream_set.owner, 101, &downstream_set.owner);

		register_account_with_submitter(&downstream_set, &downstream_set.vault, Some(access_proof));

		let downstream =
			OperationalAccounts::<Test>::get(&downstream_set.owner).expect("downstream");
		assert_eq!(downstream.upstream_account, Some(upstream_set.owner));
	});
}

#[test]
fn test_register_rejects_invalid_access_proof() {
	new_test_ext().execute_with(|| {
		let upstream_set = make_account_set(111, 112, 113);
		let downstream_set = make_account_set(119, 120, 121);
		seed_vault_registration(&upstream_set);
		register_account(&upstream_set, None);
		let access_proof = make_access_proof(&upstream_set.owner, 99, &downstream_set.owner);

		assert_noop!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(downstream_set.owner.clone()),
				downstream_set.registration(Some(access_proof)),
			),
			Error::<Test>::InvalidAccessProof
		);
	});
}

#[test]
fn test_register_allows_upstream_without_vault_when_access_available() {
	new_test_ext().execute_with(|| {
		let upstream_set = make_account_set(131, 132, 133);
		let downstream_set = make_account_set(135, 136, 137);
		register_account(&upstream_set, None);
		set_available_access_codes(&upstream_set.owner, 1);
		IsOperationalAccountInviteOnly::<Test>::put(true);
		let access_proof = make_access_proof(&upstream_set.owner, 131, &downstream_set.owner);

		register_account(&downstream_set, Some(access_proof));

		let downstream_account =
			OperationalAccounts::<Test>::get(&downstream_set.owner).expect("downstream account");
		assert_eq!(downstream_account.upstream_account, Some(upstream_set.owner));
	});
}

#[test]
fn test_register_rejects_when_upstream_is_missing() {
	new_test_ext().execute_with(|| {
		let downstream_set = make_account_set(145, 146, 147);
		let missing_upstream = account_id_from_seed(149);
		let missing_access_proof = make_access_proof(&missing_upstream, 149, &downstream_set.owner);

		assert_noop!(
			OperationalAccountsPallet::register(
				RuntimeOrigin::signed(downstream_set.owner.clone()),
				downstream_set.registration(Some(missing_access_proof)),
			),
			Error::<Test>::UpstreamNotOperationalAccount
		);
	});
}

#[test]
fn test_activate_requires_current_minimums() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(149, 150, 151);
		register_account(&account_set, None);
		satisfy_operational_requirements(&account_set.mining, &account_set.vault);

		release_account_bitcoin(&account_set.vault);

		assert_noop!(
			OperationalAccountsPallet::activate(RuntimeOrigin::signed(account_set.mining.clone())),
			Error::<Test>::MinimumsNotMet
		);
	});
}

#[test]
fn test_activate_allows_one_argon_vault_shortfall_but_not_more() {
	new_test_ext().execute_with(|| {
		let within_tolerance = make_account_set(209, 210, 211);
		register_account(&within_tolerance, None);
		set_registration_lookup(
			within_tolerance.vault.clone(),
			within_tolerance.mining.clone(),
			MinimumBitcoin::get(),
			OperationalMinimumVaultSecuritization::get()
				.saturating_sub(Balance::from(MICROGONS_PER_ARGON)),
			MinimumBonds::get(),
			0,
		);
		OperationalAccountsPallet::vault_created(&within_tolerance.vault);
		set_linked_account_uniswap_argon_transfers_in_amount(
			&within_tolerance.vault,
			OperationalMinimumUniswapTransfer::get().saturating_sub(MinimumUniswapTransfer::get()),
		);
		OperationalAccountsPallet::mining_seat_won(&within_tolerance.mining);
		OperationalAccountsPallet::mining_seat_won(&within_tolerance.mining);

		assert_ok!(OperationalAccountsPallet::activate(RuntimeOrigin::signed(
			within_tolerance.owner.clone(),
		)));

		let beyond_tolerance = make_account_set(213, 214, 215);
		register_account(&beyond_tolerance, None);
		set_registration_lookup(
			beyond_tolerance.vault.clone(),
			beyond_tolerance.mining.clone(),
			MinimumBitcoin::get(),
			OperationalMinimumVaultSecuritization::get()
				.saturating_sub(Balance::from(MICROGONS_PER_ARGON))
				.saturating_sub(1),
			MinimumBonds::get(),
			0,
		);
		OperationalAccountsPallet::vault_created(&beyond_tolerance.vault);
		set_linked_account_uniswap_argon_transfers_in_amount(
			&beyond_tolerance.vault,
			OperationalMinimumUniswapTransfer::get().saturating_sub(MinimumUniswapTransfer::get()),
		);
		OperationalAccountsPallet::mining_seat_won(&beyond_tolerance.mining);
		OperationalAccountsPallet::mining_seat_won(&beyond_tolerance.mining);

		assert_noop!(
			OperationalAccountsPallet::activate(RuntimeOrigin::signed(
				beyond_tolerance.owner.clone(),
			)),
			Error::<Test>::NotEligibleForActivation
		);
	});
}

#[test]
fn test_activate_requires_minimum_uniswap_argon_transfers_in() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(151, 152, 153);
		register_account(&account_set, None);
		set_registration_lookup(
			account_set.vault.clone(),
			account_set.mining.clone(),
			MinimumBitcoin::get(),
			OperationalMinimumVaultSecuritization::get(),
			MinimumBonds::get(),
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
			OperationalMinimumUniswapTransfer::get().saturating_sub(MinimumUniswapTransfer::get()),
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
fn test_follow_on_access_codes_only_materialize_once_on_mining_frame_turn() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(151, 152, 153);
		register_account(&account_set, None);
		satisfy_and_activate(&account_set);
		OperationalAccounts::<Test>::mutate(&account_set.owner, |maybe| {
			let account = maybe.as_mut().expect("account");
			account.available_access_codes = 0;
			account.mining_seat_applied_total = mining_seat_count(account);
			account.mining_seat_accrual = 0;
		});

		for _ in 0..MiningSeatsPerAccessCode::get() {
			OperationalAccountsPallet::mining_seat_won(&account_set.mining);
		}

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.available_access_codes, 0);
		assert_eq!(account.mining_seat_accrual, MiningSeatsPerAccessCode::get());

		OperationalAccountsPallet::vault_bitcoin_lock_funded(
			&account_set.vault,
			BitcoinLockSizeForAccessCode::get(),
		);
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.available_access_codes, 0);
		assert_eq!(account.vault_bitcoin_accrual, BitcoinLockSizeForAccessCode::get());
		assert_eq!(account.mining_seat_accrual, MiningSeatsPerAccessCode::get());

		<OperationalAccountsPallet as OnNewSlot<TestAccountId>>::on_frame_start(1);
		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.available_access_codes, 1);
		assert_eq!(account.vault_bitcoin_accrual, 0);
		assert_eq!(account.mining_seat_accrual, 0);
		assert_eq!(current_vault_bitcoin_amount(&account), BitcoinLockSizeForAccessCode::get());

		OperationalAccounts::<Test>::mutate(&account_set.owner, |maybe| {
			let account = maybe.as_mut().expect("account");
			account.available_access_codes = 0;
			account.vault_bitcoin_accrual = 2 * BitcoinLockSizeForAccessCode::get();
			account.mining_seat_accrual = 2 * MiningSeatsPerAccessCode::get() - 1;
		});
		AccessCodeReadyAccounts::<Test>::insert(&account_set.owner, ());
		<OperationalAccountsPallet as OnNewSlot<TestAccountId>>::on_frame_start(2);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.available_access_codes, 1);
		assert_eq!(account.vault_bitcoin_accrual, 0);
		assert_eq!(account.mining_seat_accrual, 0);
	});
}

#[test]
fn test_frame_rechecks_access_code_thresholds_before_awarding() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(181, 182, 183);
		register_account(&account_set, None);
		OperationalAccounts::<Test>::mutate(&account_set.owner, |maybe| {
			let account = maybe.as_mut().expect("account");
			account.is_operationally_certified = true;
			account.vault_bitcoin_accrual = BitcoinLockSizeForAccessCode::get();
			account.mining_seat_accrual = MiningSeatsPerAccessCode::get();
		});
		AccessCodeReadyAccounts::<Test>::insert(&account_set.owner, ());

		OperationalAccounts::<Test>::mutate(&account_set.owner, |maybe| {
			let account = maybe.as_mut().expect("account");
			account.mining_seat_accrual = MiningSeatsPerAccessCode::get() - 1;
		});

		<OperationalAccountsPallet as OnNewSlot<TestAccountId>>::on_frame_start(1);

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(account.available_access_codes, 0);
		assert_eq!(account.vault_bitcoin_accrual, BitcoinLockSizeForAccessCode::get());
		assert_eq!(account.mining_seat_accrual, MiningSeatsPerAccessCode::get() - 1);
		assert!(!AccessCodeReadyAccounts::<Test>::contains_key(&account_set.owner));
	});
}

#[test]
fn test_activation_grants_single_code_without_materializing_progress() {
	new_test_ext().execute_with(|| {
		let account_set = make_account_set(201, 202, 203);
		register_account(&account_set, None);
		satisfy_operational_requirements(&account_set.mining, &account_set.vault);
		for _ in 2..MiningSeatsPerAccessCode::get() {
			OperationalAccountsPallet::mining_seat_won(&account_set.mining);
		}
		OperationalAccountsPallet::vault_bitcoin_lock_funded(
			&account_set.vault,
			BitcoinLockSizeForAccessCode::get(),
		);

		assert_ok!(OperationalAccountsPallet::activate(RuntimeOrigin::signed(
			account_set.mining.clone()
		)));

		let account = OperationalAccounts::<Test>::get(&account_set.owner).expect("account");
		assert_eq!(pending_rewards_amount(&account), OperationalCertificationReward::get());
		assert_eq!(account.available_access_codes, 1);
		assert_eq!(account.vault_bitcoin_accrual, BitcoinLockSizeForAccessCode::get());
		assert_eq!(account.mining_seat_accrual, MiningSeatsPerAccessCode::get());
		assert!(AccessCodeReadyAccounts::<Test>::contains_key(&account_set.owner));
	});
}

#[test]
fn test_frame_awards_are_bounded_and_remaining_accounts_stay_ready() {
	new_test_ext().execute_with(|| {
		let account_sets = [
			make_account_set(211, 212, 213),
			make_account_set(221, 222, 223),
			make_account_set(231, 232, 233),
		];

		for account_set in &account_sets {
			register_account(account_set, None);
			OperationalAccounts::<Test>::mutate(&account_set.owner, |maybe| {
				let account = maybe.as_mut().expect("account");
				account.is_operationally_certified = true;
				account.vault_bitcoin_accrual = BitcoinLockSizeForAccessCode::get();
				account.mining_seat_accrual = MiningSeatsPerAccessCode::get();
			});
			AccessCodeReadyAccounts::<Test>::insert(&account_set.owner, ());
		}

		<OperationalAccountsPallet as OnNewSlot<TestAccountId>>::on_frame_start(1);

		let awarded = account_sets
			.iter()
			.filter(|account_set| {
				OperationalAccounts::<Test>::get(&account_set.owner)
					.expect("account")
					.available_access_codes ==
					1
			})
			.count();
		assert_eq!(awarded, 2);
		assert_eq!(AccessCodeReadyAccounts::<Test>::count(), 1);

		<OperationalAccountsPallet as OnNewSlot<TestAccountId>>::on_frame_start(2);
		assert!(account_sets.iter().all(|account_set| {
			OperationalAccounts::<Test>::get(&account_set.owner)
				.expect("account")
				.available_access_codes ==
				1
		}));
		assert_eq!(AccessCodeReadyAccounts::<Test>::count(), 0);
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
	fn registration(
		&self,
		access_proof: Option<UpstreamAccessProof<TestAccountId>>,
	) -> Registration<Test> {
		Registration::V1(RegistrationV1 {
			operational_account: self.owner.clone(),
			encryption_pubkey: self.encryption_pubkey.clone(),
			operational_account_proof: self.owner_proof.clone(),
			vault_account: self.vault.clone(),
			mining_account: self.mining.clone(),
			vault_account_proof: self.vault_proof.clone(),
			mining_account_proof: self.mining_proof.clone(),
			access_proof,
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

fn operational_account_name(name: &str) -> OperationalAccountName {
	BoundedVec::truncate_from(name.as_bytes().to_vec())
}

fn meets_minimums(account: &OperationalAccount<Test>) -> bool {
	OperationalAccountsPallet::meets_minimums(account)
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

fn register_account(set: &AccountSet, access_proof: Option<UpstreamAccessProof<TestAccountId>>) {
	register_account_with_submitter(set, &set.owner, access_proof);
}

fn register_account_with_submitter(
	set: &AccountSet,
	submitter: &TestAccountId,
	access_proof: Option<UpstreamAccessProof<TestAccountId>>,
) {
	seed_registration_minimums(set);
	set_argon_balance(
		&set.owner,
		OperationalMinimumUniswapTransfer::get().saturating_add(MinimumBitcoin::get()),
	);
	assert_ok!(OperationalAccountsPallet::register(
		RuntimeOrigin::signed(submitter.clone()),
		set.registration(access_proof),
	));
}

fn make_access_proof(
	upstream_account: &TestAccountId,
	upstream_seed: u8,
	downstream_account: &TestAccountId,
) -> UpstreamAccessProof<TestAccountId> {
	let upstream_pair = sr25519::Pair::from_seed(&[upstream_seed; 32]);
	let upstream_message =
		(UPSTREAM_ACCESS_PROOF_MESSAGE_KEY, upstream_account, downstream_account)
			.using_encoded(blake2_256);
	let signature: Signature = upstream_pair.sign(upstream_message.as_slice()).into();

	UpstreamAccessProof { upstream_account: upstream_account.clone(), signature }
}

fn seed_vault_registration(set: &AccountSet) {
	set_registration_lookup(
		set.vault.clone(),
		set.mining.clone(),
		MinimumBitcoin::get(),
		OperationalMinimumVaultSecuritization::get(),
		MinimumBonds::get(),
		0,
	);
}

fn seed_registration_minimums(set: &AccountSet) {
	record_microgons_in(&set.owner, MinimumUniswapTransfer::get());
	record_account_bitcoin(&set.vault, MinimumBitcoin::get());
	record_account_vault_bond_amount(&set.vault, MinimumBonds::get());
}

fn set_linked_account_uniswap_argon_transfers_in_amount(
	account_id: &TestAccountId,
	amount: Balance,
) {
	record_microgons_in(account_id, amount);
	OperationalAccountsPallet::refresh_account_uniswap_argon_transfers_in_amount(account_id);
}

fn account_utxo_id(account_id: &TestAccountId) -> u64 {
	let account_bytes: &[u8] = account_id.as_ref();
	u64::from_le_bytes(account_bytes[0..8].try_into().expect("utxo id bytes"))
}

fn source_vault_id(account_id: &TestAccountId) -> u32 {
	let account_bytes: &[u8] = account_id.as_ref();
	u32::from_le_bytes(account_bytes[0..4].try_into().expect("vault id bytes"))
}

fn record_account_bitcoin(account_id: &TestAccountId, amount: Balance) {
	record_funded_bitcoin_amount(
		account_id,
		funded_bitcoin_amount(account_id).saturating_add(amount),
	);
	assert_ok!(OperationalAccountsPallet::utxo_locked(
		account_utxo_id(account_id),
		account_id,
		amount,
	));
}

fn release_account_bitcoin(account_id: &TestAccountId) {
	let amount = funded_bitcoin_amount(account_id);
	record_funded_bitcoin_amount(account_id, 0);
	assert_ok!(OperationalAccountsPallet::utxo_released(
		account_utxo_id(account_id),
		account_id,
		false,
		amount,
		amount,
	));
}

fn record_account_vault_bond_amount(account_id: &TestAccountId, amount: Balance) {
	record_active_vault_bond_amount(source_vault_id(account_id), account_id, amount);
	OperationalAccountsPallet::account_vault_bond_total_updated(account_id, amount);
}

fn satisfy_operational_requirements(mining_account: &TestAccountId, vault_account: &TestAccountId) {
	set_registration_lookup(
		vault_account.clone(),
		mining_account.clone(),
		MinimumBitcoin::get(),
		OperationalMinimumVaultSecuritization::get(),
		MinimumBonds::get(),
		0,
	);
	OperationalAccountsPallet::vault_created(vault_account);
	set_linked_account_uniswap_argon_transfers_in_amount(
		vault_account,
		OperationalMinimumUniswapTransfer::get().saturating_sub(MinimumUniswapTransfer::get()),
	);
	OperationalAccountsPallet::mining_seat_won(mining_account);
	OperationalAccountsPallet::mining_seat_won(mining_account);
}

fn satisfy_and_activate(set: &AccountSet) {
	satisfy_operational_requirements(&set.mining, &set.vault);
	assert_ok!(OperationalAccountsPallet::activate(RuntimeOrigin::signed(set.mining.clone())));
}

fn set_available_access_codes(owner: &TestAccountId, count: u32) {
	OperationalAccounts::<Test>::mutate(owner, |maybe| {
		let account = maybe.as_mut().expect("operational account");
		account.available_access_codes = count;
	});
}
