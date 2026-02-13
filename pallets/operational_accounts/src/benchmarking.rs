//! Benchmarking setup for pallet-operational-accounts
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as OperationalAccountsPallet;
use argon_primitives::{OperationalAccountsHook, Signature};
use codec::Decode;
use frame_system::RawOrigin;
use pallet_prelude::*;
use polkadot_sdk::{
	frame_benchmarking,
	frame_benchmarking::v2::*,
	sp_core::{crypto::KeyTypeId, sr25519},
	sp_io::{self, hashing::blake2_256},
	sp_runtime::traits::Zero,
};

const USER_SEED: u32 = 0;
const BENCH_KEY_TYPE: KeyTypeId = KeyTypeId(*b"opac");

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn register() {
		let caller: T::AccountId = account("caller", 0, USER_SEED);
		let (vault, vault_proof) =
			make_linked_account::<T>(&caller, 1, VAULT_ACCOUNT_PROOF_MESSAGE_KEY);
		let (mining_funding, mining_funding_proof) =
			make_linked_account::<T>(&caller, 2, MINING_FUNDING_ACCOUNT_PROOF_MESSAGE_KEY);
		let (mining_bot, mining_bot_proof) =
			make_linked_account::<T>(&caller, 3, MINING_BOT_ACCOUNT_PROOF_MESSAGE_KEY);
		whitelist_account!(caller);
		#[extrinsic_call]
		register(
			RawOrigin::Signed(caller.clone()),
			vault.clone(),
			mining_funding.clone(),
			mining_bot.clone(),
			vault_proof,
			mining_funding_proof,
			mining_bot_proof,
			None,
		);

		assert!(OperationalAccounts::<T>::contains_key(caller));
		assert!(OperationalAccountBySubAccount::<T>::contains_key(vault));
		assert!(OperationalAccountBySubAccount::<T>::contains_key(mining_funding));
		assert!(OperationalAccountBySubAccount::<T>::contains_key(mining_bot));
	}

	#[benchmark]
	fn on_vault_created() {
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.has_uniswap_transfer = true;
		account.bitcoin_accrual = T::MinBitcoinLockSizeForOperational::get();
		account.has_treasury_pool_participation = true;
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_vault_to_owner::<T>(&linked);

		#[block]
		{
			let _ = OperationalAccountsPallet::<T>::vault_created(&linked.vault);
		}

		assert!(OperationalAccounts::<T>::get(&linked.owner).is_some());
	}

	#[benchmark]
	fn on_bitcoin_lock_funded() {
		let linked = linked_accounts::<T>();
		let threshold = T::BitcoinLockSizeForAccessCode::get();
		let mut account = default_operational_account::<T>(&linked);
		account.has_uniswap_transfer = true;
		account.vault_created = true;
		account.bitcoin_accrual = T::MinBitcoinLockSizeForOperational::get();
		account.has_treasury_pool_participation = true;
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		account.is_operational = true;
		insert_operational_account::<T>(&linked, account);
		link_vault_to_owner::<T>(&linked);

		#[block]
		{
			let _ = OperationalAccountsPallet::<T>::bitcoin_lock_funded(&linked.vault, threshold);
		}

		assert!(OperationalAccounts::<T>::contains_key(&linked.owner));
	}

	#[benchmark]
	fn on_mining_seat_won() {
		let linked = linked_accounts::<T>();
		let seats_per_code = T::MiningSeatsPerAccessCode::get().max(1);
		let accrual_start = seats_per_code.saturating_sub(1);
		let mut account = default_operational_account::<T>(&linked);
		account.has_uniswap_transfer = true;
		account.vault_created = true;
		account.bitcoin_accrual = T::MinBitcoinLockSizeForOperational::get();
		account.has_treasury_pool_participation = true;
		account.mining_seat_accrual = accrual_start.max(T::MiningSeatsForOperational::get());
		account.is_operational = true;
		insert_operational_account::<T>(&linked, account);
		link_mining_funding_to_owner::<T>(&linked);

		#[block]
		{
			let _ = OperationalAccountsPallet::<T>::mining_seat_won(&linked.mining_funding);
		}

		assert!(OperationalAccounts::<T>::contains_key(&linked.owner));
	}

	#[benchmark]
	fn on_treasury_pool_participated() {
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.has_uniswap_transfer = true;
		account.vault_created = true;
		account.bitcoin_accrual = T::MinBitcoinLockSizeForOperational::get();
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_vault_to_owner::<T>(&linked);

		#[block]
		{
			let _ = OperationalAccountsPallet::<T>::treasury_pool_participated(
				&linked.vault,
				<T::Balance as Zero>::zero(),
			);
		}

		assert!(OperationalAccounts::<T>::contains_key(&linked.owner));
	}

	#[benchmark]
	fn on_uniswap_transfer() {
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.vault_created = true;
		account.bitcoin_accrual = T::MinBitcoinLockSizeForOperational::get();
		account.has_treasury_pool_participation = true;
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_vault_to_owner::<T>(&linked);

		#[block]
		{
			OperationalAccountsPallet::<T>::on_uniswap_transfer(
				&linked.vault,
				<T::Balance as Zero>::zero(),
			);
		}

		assert!(OperationalAccounts::<T>::contains_key(&linked.owner));
	}

	#[benchmark]
	fn issue_access_code() {
		let sponsor: T::AccountId = account("sponsor", 0, USER_SEED);
		let sponsor_links = LinkedAccounts {
			owner: sponsor.clone(),
			vault: sponsor.clone(),
			mining_funding: sponsor.clone(),
			mining_bot: sponsor.clone(),
		};
		let mut sponsor_account = default_operational_account::<T>(&sponsor_links);
		sponsor_account.issuable_access_codes = 1;
		insert_operational_account::<T>(&sponsor_links, sponsor_account);

		let access_code_public = sr25519::Public::from_raw([5u8; 32]);
		whitelist_account!(sponsor);
		#[extrinsic_call]
		issue_access_code(RawOrigin::Signed(sponsor.clone()), access_code_public);

		assert!(AccessCodesByPublic::<T>::contains_key(access_code_public));
	}

	#[benchmark]
	fn set_reward_config() {
		let operational_referral_reward = T::Balance::from(1_000u128);
		let referral_bonus_reward = T::Balance::from(500u128);
		#[extrinsic_call]
		set_reward_config(RawOrigin::Root, operational_referral_reward, referral_bonus_reward);

		let config = Rewards::<T>::get();
		assert_eq!(config.operational_referral_reward, operational_referral_reward);
		assert_eq!(config.referral_bonus_reward, referral_bonus_reward);
	}

	impl_benchmark_test_suite!(
		OperationalAccountsPallet,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);

	struct LinkedAccounts<T: Config> {
		owner: T::AccountId,
		vault: T::AccountId,
		mining_funding: T::AccountId,
		mining_bot: T::AccountId,
	}

	fn linked_accounts<T: Config>() -> LinkedAccounts<T> {
		LinkedAccounts {
			owner: account("owner", 0, USER_SEED),
			vault: account("vault", 0, USER_SEED),
			mining_funding: account("mining_funding", 0, USER_SEED),
			mining_bot: account("mining_bot", 0, USER_SEED),
		}
	}

	fn default_operational_account<T: Config>(linked: &LinkedAccounts<T>) -> OperationalAccount<T> {
		OperationalAccount {
			vault_account: linked.vault.clone(),
			mining_funding_account: linked.mining_funding.clone(),
			mining_bot_account: linked.mining_bot.clone(),
			sponsor: None,
			has_uniswap_transfer: false,
			vault_created: false,
			bitcoin_accrual: <T::Balance as Zero>::zero(),
			bitcoin_high_watermark: <T::Balance as Zero>::zero(),
			has_treasury_pool_participation: false,
			mining_seat_accrual: 0,
			mining_seat_high_watermark: 0,
			operational_referrals_count: 0,
			referral_access_code_pending: false,
			issuable_access_codes: 0,
			unactivated_access_codes: 0,
			rewards_earned_count: 0,
			rewards_earned_amount: <T::Balance as Zero>::zero(),
			rewards_collected_amount: <T::Balance as Zero>::zero(),
			is_operational: false,
		}
	}

	fn insert_operational_account<T: Config>(
		linked: &LinkedAccounts<T>,
		account: OperationalAccount<T>,
	) {
		OperationalAccounts::<T>::insert(&linked.owner, account);
	}

	fn link_vault_to_owner<T: Config>(linked: &LinkedAccounts<T>) {
		OperationalAccountBySubAccount::<T>::insert(&linked.vault, &linked.owner);
	}

	fn link_mining_funding_to_owner<T: Config>(linked: &LinkedAccounts<T>) {
		OperationalAccountBySubAccount::<T>::insert(&linked.mining_funding, &linked.owner);
	}

	fn make_linked_account<T: Config>(
		owner: &T::AccountId,
		seed: u8,
		domain: &[u8],
	) -> (T::AccountId, AccountOwnershipProof) {
		let seed_phrase = match seed {
			1 => "//operational-access-code-1",
			2 => "//operational-access-code-2",
			3 => "//operational-access-code-3",
			_ => "//operational-access-code",
		};
		let public =
			sp_io::crypto::sr25519_generate(BENCH_KEY_TYPE, Some(seed_phrase.as_bytes().to_vec()));
		let account_id = T::AccountId::decode(&mut &public.0[..])
			.expect("sr25519 public key should decode into benchmark AccountId");
		let message = (domain, owner, &account_id).using_encoded(blake2_256);
		let signature: Signature =
			sp_io::crypto::sr25519_sign(BENCH_KEY_TYPE, &public, message.as_slice())
				.expect("benchmark signing key should exist")
				.into();
		(account_id, AccountOwnershipProof { signature })
	}
}
