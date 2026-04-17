//! Benchmarking setup for pallet-operational-accounts
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as OperationalAccountsPallet;
use argon_primitives::{MICROGONS_PER_ARGON, OperationalAccountsHook};
use codec::Decode;
#[cfg(test)]
use codec::Encode;
use frame_system::RawOrigin;
use pallet_prelude::{
	benchmarking::{
		BenchmarkOperationalAccountsProviderCallCounters,
		BenchmarkOperationalAccountsProviderState,
		benchmark_operational_accounts_provider_call_counters,
		reset_benchmark_operational_accounts_provider_call_counters,
		reset_benchmark_operational_accounts_provider_state,
		set_benchmark_operational_accounts_provider_state,
	},
	*,
};
use polkadot_sdk::{
	frame_benchmarking,
	frame_benchmarking::v2::*,
	sp_core::sr25519,
	sp_runtime::traits::{SaturatedConversion, Zero},
};

const USER_SEED: u32 = 0;
const BENCH_OPERATIONAL_ACCOUNT: [u8; 32] = [
	106, 16, 190, 2, 157, 30, 210, 131, 68, 101, 135, 20, 90, 79, 136, 82, 37, 72, 155, 73, 4, 36,
	160, 50, 141, 204, 226, 164, 138, 230, 254, 97,
];
const BENCH_VAULT_ACCOUNT: [u8; 32] = [
	24, 157, 172, 41, 41, 109, 49, 129, 77, 200, 197, 108, 243, 211, 106, 5, 67, 55, 43, 186, 117,
	56, 250, 50, 42, 74, 235, 254, 188, 57, 224, 86,
];
const BENCH_MINING_FUNDING_ACCOUNT: [u8; 32] = [
	26, 79, 238, 72, 193, 186, 26, 72, 232, 205, 67, 120, 42, 132, 133, 214, 53, 170, 145, 207,
	184, 44, 187, 71, 127, 12, 28, 87, 107, 196, 3, 28,
];
const BENCH_MINING_BOT_ACCOUNT: [u8; 32] = [
	142, 229, 4, 20, 142, 117, 195, 78, 143, 5, 24, 153, 179, 198, 228, 36, 31, 241, 141, 193, 201,
	33, 18, 96, 182, 166, 164, 52, 190, 219, 72, 95,
];
const BENCH_ACCESS_CODE_PUBLIC: [u8; 32] = [
	194, 226, 189, 113, 224, 74, 106, 242, 137, 124, 52, 20, 214, 253, 64, 52, 119, 36, 80, 96,
	253, 34, 218, 170, 65, 47, 245, 27, 131, 192, 194, 46,
];
const BENCH_OPERATIONAL_SIGNATURE: [u8; 64] = [
	230, 146, 70, 252, 75, 156, 14, 179, 5, 252, 131, 55, 28, 132, 179, 106, 133, 194, 117, 53, 7,
	114, 109, 51, 140, 222, 142, 222, 43, 159, 230, 89, 159, 206, 150, 92, 229, 141, 6, 3, 121,
	207, 240, 213, 196, 141, 121, 5, 161, 224, 2, 153, 56, 45, 192, 84, 130, 92, 152, 65, 16, 0,
	252, 130,
];
const BENCH_VAULT_SIGNATURE: [u8; 64] = [
	208, 37, 32, 57, 145, 169, 149, 246, 23, 137, 200, 189, 56, 8, 210, 234, 138, 80, 93, 177, 220,
	97, 176, 197, 92, 69, 194, 70, 52, 195, 122, 2, 92, 108, 24, 222, 71, 106, 164, 170, 122, 11,
	220, 221, 45, 77, 177, 52, 238, 110, 133, 134, 67, 192, 67, 134, 77, 247, 247, 167, 165, 159,
	248, 133,
];
const BENCH_MINING_FUNDING_SIGNATURE: [u8; 64] = [
	204, 30, 215, 199, 3, 160, 79, 206, 129, 74, 58, 226, 244, 227, 87, 155, 111, 157, 218, 163,
	248, 225, 78, 150, 65, 23, 151, 84, 74, 134, 187, 39, 135, 206, 70, 119, 84, 176, 37, 93, 41,
	112, 113, 184, 124, 65, 108, 129, 200, 202, 188, 23, 26, 47, 167, 239, 63, 181, 237, 102, 180,
	195, 96, 137,
];
const BENCH_MINING_BOT_SIGNATURE: [u8; 64] = [
	198, 19, 229, 142, 158, 206, 161, 89, 52, 138, 239, 238, 61, 243, 178, 227, 34, 135, 47, 31,
	131, 165, 92, 199, 109, 211, 96, 42, 32, 41, 17, 33, 44, 252, 182, 102, 104, 87, 223, 157, 11,
	135, 111, 189, 98, 83, 208, 56, 143, 194, 77, 72, 165, 46, 134, 218, 194, 69, 34, 97, 215, 124,
	158, 129,
];
const BENCH_ACCESS_CODE_SIGNATURE: [u8; 64] = [
	170, 36, 183, 56, 107, 58, 13, 16, 115, 132, 63, 37, 140, 27, 84, 40, 81, 152, 48, 54, 147, 83,
	164, 193, 205, 195, 202, 61, 245, 88, 99, 30, 55, 196, 28, 150, 3, 169, 10, 7, 123, 188, 204,
	42, 23, 162, 235, 99, 46, 221, 130, 45, 74, 84, 84, 251, 236, 101, 89, 190, 23, 115, 251, 139,
];

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn register() {
		reset_benchmark_operational_accounts_provider_state();
		reset_benchmark_operational_accounts_provider_call_counters();
		let caller = benchmark_account_id::<T>(BENCH_OPERATIONAL_ACCOUNT);
		let operational_account_proof = benchmark_account_proof(BENCH_OPERATIONAL_SIGNATURE);
		let sponsor = linked_accounts::<T>();
		let access_code = benchmark_access_code_proof();
		let mut sponsor_account = default_operational_account::<T>(&sponsor);
		sponsor_account.is_operational = true;
		sponsor_account.unactivated_access_codes = 1;
		insert_operational_account::<T>(&sponsor, sponsor_account);
		AccessCodesByPublic::<T>::insert(
			access_code.public,
			AccessCodeMetadata { sponsor: sponsor.owner.clone(), expiration_frame: 1 },
		);
		AccessCodesExpiringByFrame::<T>::mutate(1, |expiring_codes| {
			let _ = expiring_codes.try_push(access_code.public);
		});
		let vault = benchmark_account_id::<T>(BENCH_VAULT_ACCOUNT);
		let mining_funding = benchmark_account_id::<T>(BENCH_MINING_FUNDING_ACCOUNT);
		let mining_bot = benchmark_account_id::<T>(BENCH_MINING_BOT_ACCOUNT);
		let vault_proof = benchmark_account_proof(BENCH_VAULT_SIGNATURE);
		let mining_funding_proof = benchmark_account_proof(BENCH_MINING_FUNDING_SIGNATURE);
		let mining_bot_proof = benchmark_account_proof(BENCH_MINING_BOT_SIGNATURE);
		#[cfg(test)]
		seed_mock_registration_lookup(&LinkedAccounts::<T> {
			owner: caller.clone(),
			vault: vault.clone(),
			mining_funding: mining_funding.clone(),
			mining_bot: mining_bot.clone(),
		});
		set_benchmark_operational_accounts_provider_state(
			BenchmarkOperationalAccountsProviderState {
				vault_registration_data: Some(argon_primitives::vault::RegistrationVaultData {
					vault_id: 1,
					activated_securitization: 1u128,
					securitization: T::OperationalMinimumVaultSecuritization::get()
						.saturated_into(),
				}),
				has_active_rewards_account_seat: true,
				has_bond_participation: true,
				requires_uniswap_transfer: true,
				call_counters: Default::default(),
			},
		);
		let registration = Registration::V1(RegistrationV1 {
			operational_account: caller.clone(),
			encryption_pubkey: OpaqueEncryptionPubkey([7u8; 32]),
			operational_account_proof,
			vault_account: vault.clone(),
			mining_funding_account: mining_funding.clone(),
			mining_bot_account: mining_bot.clone(),
			vault_account_proof: vault_proof,
			mining_funding_account_proof: mining_funding_proof,
			mining_bot_account_proof: mining_bot_proof,
			access_code: Some(access_code.clone()),
		});
		let Registration::V1(RegistrationV1 {
			operational_account,
			operational_account_proof,
			vault_account,
			mining_funding_account,
			mining_bot_account,
			vault_account_proof,
			mining_funding_account_proof,
			mining_bot_account_proof,
			..
		}) = &registration;
		assert!(operational_account_proof.verify(
			operational_account,
			operational_account,
			OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY,
		));
		assert!(vault_account_proof.verify(
			operational_account,
			vault_account,
			VAULT_ACCOUNT_PROOF_MESSAGE_KEY,
		));
		assert!(mining_funding_account_proof.verify(
			operational_account,
			mining_funding_account,
			MINING_FUNDING_ACCOUNT_PROOF_MESSAGE_KEY,
		));
		assert!(mining_bot_account_proof.verify(
			operational_account,
			mining_bot_account,
			MINING_BOT_ACCOUNT_PROOF_MESSAGE_KEY,
		));
		whitelist_account!(caller);
		#[extrinsic_call]
		register(RawOrigin::Signed(caller.clone()), registration);

		let account = OperationalAccounts::<T>::get(&caller).expect("account stored");
		assert_eq!(account.bitcoin_accrual, 1u128.into());
		assert!(account.vault_created);
		assert!(account.has_treasury_pool_participation);
		assert_eq!(account.mining_seat_accrual, 1);
		assert!(!account.is_operational);
		assert!(OperationalAccountBySubAccount::<T>::contains_key(vault));
		assert!(OperationalAccountBySubAccount::<T>::contains_key(mining_funding));
		assert!(OperationalAccountBySubAccount::<T>::contains_key(mining_bot));
		assert!(!AccessCodesByPublic::<T>::contains_key(access_code.public));
		assert_provider_calls(BenchmarkOperationalAccountsProviderCallCounters {
			get_registration_vault_data: 1,
			has_active_rewards_account_seat: 1,
			has_bond_participation: 1,
			requires_uniswap_transfer: 1,
			account_became_operational: 0,
		});
	}

	#[benchmark]
	fn on_vault_created() {
		set_benchmark_operational_accounts_provider_state(
			BenchmarkOperationalAccountsProviderState {
				vault_registration_data: Some(argon_primitives::vault::RegistrationVaultData {
					vault_id: 1,
					activated_securitization: 1u128,
					securitization: T::OperationalMinimumVaultSecuritization::get()
						.saturated_into(),
				}),
				has_active_rewards_account_seat: true,
				has_bond_participation: true,
				requires_uniswap_transfer: true,
				call_counters: Default::default(),
			},
		);
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.has_uniswap_transfer = true;
		account.bitcoin_accrual = 1u128.into();
		account.has_treasury_pool_participation = true;
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_vault_to_owner::<T>(&linked);
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);

		#[block]
		{
			let _ = OperationalAccountsPallet::<T>::vault_created(&linked.vault);
		}

		assert!(
			OperationalAccounts::<T>::get(&linked.owner)
				.expect("account should exist")
				.vault_created
		);
	}

	#[benchmark]
	fn on_bitcoin_lock_funded() {
		set_benchmark_operational_accounts_provider_state(
			BenchmarkOperationalAccountsProviderState {
				vault_registration_data: Some(argon_primitives::vault::RegistrationVaultData {
					vault_id: 1,
					activated_securitization: 1u128,
					securitization: T::OperationalMinimumVaultSecuritization::get()
						.saturated_into(),
				}),
				has_active_rewards_account_seat: true,
				has_bond_participation: true,
				requires_uniswap_transfer: true,
				call_counters: Default::default(),
			},
		);
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.has_uniswap_transfer = true;
		account.vault_created = true;
		account.has_treasury_pool_participation = true;
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_vault_to_owner::<T>(&linked);
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);

		#[block]
		{
			let _ =
				OperationalAccountsPallet::<T>::bitcoin_lock_funded(&linked.vault, 1u128.into());
		}

		assert_eq!(
			OperationalAccounts::<T>::get(&linked.owner)
				.expect("account should exist")
				.bitcoin_accrual,
			1u128.into()
		);
	}

	#[benchmark]
	fn on_mining_seat_won() {
		set_benchmark_operational_accounts_provider_state(
			BenchmarkOperationalAccountsProviderState {
				vault_registration_data: Some(argon_primitives::vault::RegistrationVaultData {
					vault_id: 1,
					activated_securitization: 1u128,
					securitization: T::OperationalMinimumVaultSecuritization::get()
						.saturated_into(),
				}),
				has_active_rewards_account_seat: true,
				has_bond_participation: true,
				requires_uniswap_transfer: true,
				call_counters: Default::default(),
			},
		);
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.has_uniswap_transfer = true;
		account.vault_created = true;
		account.bitcoin_accrual = 1u128.into();
		account.has_treasury_pool_participation = true;
		account.mining_seat_accrual = T::MiningSeatsForOperational::get().saturating_sub(1);
		insert_operational_account::<T>(&linked, account);
		link_mining_funding_to_owner::<T>(&linked);
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);

		#[block]
		{
			let _ = OperationalAccountsPallet::<T>::mining_seat_won(&linked.mining_funding);
		}

		assert_eq!(
			OperationalAccounts::<T>::get(&linked.owner)
				.expect("account should exist")
				.mining_seat_accrual,
			T::MiningSeatsForOperational::get()
		);
	}

	#[benchmark]
	fn on_treasury_pool_participated() {
		set_benchmark_operational_accounts_provider_state(
			BenchmarkOperationalAccountsProviderState {
				vault_registration_data: Some(argon_primitives::vault::RegistrationVaultData {
					vault_id: 1,
					activated_securitization: 1u128,
					securitization: T::OperationalMinimumVaultSecuritization::get()
						.saturated_into(),
				}),
				has_active_rewards_account_seat: true,
				has_bond_participation: true,
				requires_uniswap_transfer: true,
				call_counters: Default::default(),
			},
		);
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.has_uniswap_transfer = true;
		account.vault_created = true;
		account.bitcoin_accrual = 1u128.into();
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_vault_to_owner::<T>(&linked);
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);

		#[block]
		{
			let _ = OperationalAccountsPallet::<T>::treasury_pool_participated(
				&linked.vault,
				<T::Balance as Zero>::zero(),
			);
		}

		assert!(
			OperationalAccounts::<T>::get(&linked.owner)
				.expect("account should exist")
				.has_treasury_pool_participation
		);
	}

	#[benchmark]
	fn on_uniswap_transfer() {
		set_benchmark_operational_accounts_provider_state(
			BenchmarkOperationalAccountsProviderState {
				vault_registration_data: Some(argon_primitives::vault::RegistrationVaultData {
					vault_id: 1,
					activated_securitization: 1u128,
					securitization: T::OperationalMinimumVaultSecuritization::get()
						.saturated_into(),
				}),
				has_active_rewards_account_seat: true,
				has_bond_participation: true,
				requires_uniswap_transfer: true,
				call_counters: Default::default(),
			},
		);
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.vault_created = true;
		account.bitcoin_accrual = 1u128.into();
		account.has_treasury_pool_participation = true;
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_vault_to_owner::<T>(&linked);
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);

		#[block]
		{
			OperationalAccountsPallet::<T>::on_uniswap_transfer(
				&linked.vault,
				<T::Balance as Zero>::zero(),
			);
		}

		assert!(
			OperationalAccounts::<T>::get(&linked.owner)
				.expect("account should exist")
				.has_uniswap_transfer
		);
	}

	#[benchmark]
	fn activate() {
		set_benchmark_operational_accounts_provider_state(
			BenchmarkOperationalAccountsProviderState {
				vault_registration_data: Some(argon_primitives::vault::RegistrationVaultData {
					vault_id: 1,
					activated_securitization: 1u128,
					securitization: T::OperationalMinimumVaultSecuritization::get()
						.saturated_into(),
				}),
				has_active_rewards_account_seat: true,
				has_pool_participation: true,
				requires_uniswap_transfer: true,
				call_counters: Default::default(),
			},
		);
		let linked = linked_accounts::<T>();
		let sponsor = linked_accounts_with_seed::<T>(1);
		let mut sponsor_account = default_operational_account::<T>(&sponsor);
		sponsor_account.is_operational = true;
		sponsor_account.operational_referrals_count =
			T::ReferralBonusEveryXOperationalSponsees::get().saturating_sub(1);
		insert_operational_account::<T>(&sponsor, sponsor_account);
		let mut account = default_operational_account::<T>(&linked);
		account.sponsor = Some(sponsor.owner.clone());
		account.has_uniswap_transfer = true;
		account.vault_created = true;
		account.bitcoin_accrual = 1u128.into();
		account.has_treasury_pool_participation = true;
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_mining_funding_to_owner::<T>(&linked);
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);
		let caller = linked.mining_funding.clone();
		whitelist_account!(caller);

		#[extrinsic_call]
		activate(RawOrigin::Signed(caller.clone()));

		assert!(
			OperationalAccounts::<T>::get(&linked.owner)
				.expect("account should exist")
				.is_operational
		);
		let sponsor_account =
			OperationalAccounts::<T>::get(&sponsor.owner).expect("sponsor account should exist");
		assert!(sponsor_account.operational_referrals_count > 0);
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

	#[benchmark]
	fn claim_rewards() {
		let linked = linked_accounts::<T>();
		let mut operational_account = default_operational_account::<T>(&linked);
		let claim_amount = T::Balance::from(MICROGONS_PER_ARGON);
		operational_account.rewards_earned_amount = claim_amount;
		insert_operational_account::<T>(&linked, operational_account);
		link_mining_funding_to_owner::<T>(&linked);
		let claimant = linked.mining_funding.clone();
		#[cfg(test)]
		crate::mock::ClaimableTreasuryBalance::set(claim_amount.into());
		whitelist_account!(claimant);

		#[extrinsic_call]
		claim_rewards(RawOrigin::Signed(claimant.clone()), claim_amount);

		assert_eq!(
			OperationalAccounts::<T>::get(&linked.owner)
				.expect("operational account should exist")
				.rewards_collected_amount,
			claim_amount,
		);
	}

	#[benchmark]
	fn force_set_progress() {
		set_benchmark_operational_accounts_provider_state(
			BenchmarkOperationalAccountsProviderState {
				vault_registration_data: Some(argon_primitives::vault::RegistrationVaultData {
					vault_id: 1,
					activated_securitization: 1u128,
					securitization: T::OperationalMinimumVaultSecuritization::get()
						.saturated_into(),
				}),
				has_active_rewards_account_seat: true,
				has_bond_participation: true,
				requires_uniswap_transfer: true,
				call_counters: Default::default(),
			},
		);
		let linked = linked_accounts::<T>();
		let account = default_operational_account::<T>(&linked);
		insert_operational_account::<T>(&linked, account);
		let patch = OperationalProgressPatch {
			has_uniswap_transfer: Some(true),
			vault_created: Some(true),
			has_treasury_pool_participation: Some(true),
			observed_bitcoin_total: Some(1u128.into()),
			observed_mining_seat_total: Some(T::MiningSeatsForOperational::get()),
		};
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);

		#[extrinsic_call]
		force_set_progress(RawOrigin::Root, linked.owner.clone(), patch, true);

		let account = OperationalAccounts::<T>::get(&linked.owner).expect("account exists");
		assert!(!account.is_operational);
	}

	#[benchmark]
	fn set_encrypted_server_for_sponsee() {
		let sponsor = linked_accounts::<T>();
		let sponsee = LinkedAccounts {
			owner: account("sponsee_owner", 0, USER_SEED),
			vault: account("sponsee_vault", 0, USER_SEED),
			mining_funding: account("sponsee_mining_funding", 0, USER_SEED),
			mining_bot: account("sponsee_mining_bot", 0, USER_SEED),
		};
		insert_operational_account::<T>(&sponsor, default_operational_account::<T>(&sponsor));
		let mut sponsee_account = default_operational_account::<T>(&sponsee);
		sponsee_account.sponsor = Some(sponsor.owner.clone());
		insert_operational_account::<T>(&sponsee, sponsee_account);
		let encrypted_server = vec![7u8; 32];
		let sponsor_owner = sponsor.owner.clone();
		whitelist_account!(sponsor_owner);

		#[extrinsic_call]
		set_encrypted_server_for_sponsee(
			RawOrigin::Signed(sponsor_owner),
			sponsee.owner.clone(),
			encrypted_server.clone(),
		);

		assert_eq!(
			EncryptedServerBySponsee::<T>::get(&sponsee.owner)
				.expect("payload stored")
				.to_vec(),
			encrypted_server
		);
	}

	impl_benchmark_test_suite!(
		OperationalAccountsPallet,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);

	fn assert_provider_calls(expected: BenchmarkOperationalAccountsProviderCallCounters) {
		if !cfg!(test) {
			assert_eq!(benchmark_operational_accounts_provider_call_counters(), expected);
		}
	}

	struct LinkedAccounts<T: Config> {
		owner: T::AccountId,
		vault: T::AccountId,
		mining_funding: T::AccountId,
		mining_bot: T::AccountId,
	}

	fn linked_accounts<T: Config>() -> LinkedAccounts<T> {
		linked_accounts_with_seed::<T>(0)
	}

	fn linked_accounts_with_seed<T: Config>(seed: u32) -> LinkedAccounts<T> {
		LinkedAccounts {
			owner: account("owner", seed, USER_SEED),
			vault: account("vault", seed, USER_SEED),
			mining_funding: account("mining_funding", seed, USER_SEED),
			mining_bot: account("mining_bot", seed, USER_SEED),
		}
	}

	fn default_operational_account<T: Config>(linked: &LinkedAccounts<T>) -> OperationalAccount<T> {
		OperationalAccount {
			vault_account: linked.vault.clone(),
			mining_funding_account: linked.mining_funding.clone(),
			mining_bot_account: linked.mining_bot.clone(),
			encryption_pubkey: OpaqueEncryptionPubkey([0u8; 32]),
			sponsor: None,
			has_uniswap_transfer: false,
			vault_created: false,
			bitcoin_accrual: <T::Balance as Zero>::zero(),
			bitcoin_applied_total: <T::Balance as Zero>::zero(),
			has_treasury_pool_participation: false,
			mining_seat_accrual: 0,
			mining_seat_applied_total: 0,
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

	fn benchmark_account_id<T: Config>(bytes: [u8; 32]) -> T::AccountId {
		T::AccountId::decode(&mut &bytes[..]).expect("benchmark account should decode")
	}

	fn benchmark_account_proof(signature: [u8; 64]) -> AccountOwnershipProof {
		AccountOwnershipProof { signature: sr25519::Signature::from_raw(signature).into() }
	}

	fn benchmark_access_code_proof() -> AccessCodeProof {
		AccessCodeProof {
			public: sr25519::Public::from_raw(BENCH_ACCESS_CODE_PUBLIC),
			signature: sr25519::Signature::from_raw(BENCH_ACCESS_CODE_SIGNATURE),
		}
	}

	#[cfg(test)]
	fn seed_mock_registration_lookup<T: Config>(linked: &LinkedAccounts<T>) {
		let vault_account = crate::mock::TestAccountId::decode(&mut &linked.vault.encode()[..])
			.expect("benchmark vault account should decode");
		let mining_funding_account =
			crate::mock::TestAccountId::decode(&mut &linked.mining_funding.encode()[..])
				.expect("benchmark mining funding account should decode");
		crate::mock::set_registration_lookup(
			vault_account,
			mining_funding_account,
			1,
			crate::mock::OperationalMinimumVaultSecuritization::get(),
			true,
			1,
		);
	}

	#[cfg(test)]
	mod fixture_output {
		use super::*;
		use argon_primitives::Signature;
		use polkadot_sdk::{
			sp_io::hashing::blake2_256,
			sp_runtime::{AccountId32, traits::Verify},
		};

		#[test]
		fn register_fixture_signatures_verify() {
			let operational_account = AccountId32::new(BENCH_OPERATIONAL_ACCOUNT);
			let vault_account = AccountId32::new(BENCH_VAULT_ACCOUNT);
			let mining_funding_account = AccountId32::new(BENCH_MINING_FUNDING_ACCOUNT);
			let mining_bot_account = AccountId32::new(BENCH_MINING_BOT_ACCOUNT);
			let access_code_public = sr25519::Public::from_raw(BENCH_ACCESS_CODE_PUBLIC);

			let operational_signature: Signature =
				sr25519::Signature::from_raw(BENCH_OPERATIONAL_SIGNATURE).into();
			let vault_signature: Signature =
				sr25519::Signature::from_raw(BENCH_VAULT_SIGNATURE).into();
			let mining_funding_signature: Signature =
				sr25519::Signature::from_raw(BENCH_MINING_FUNDING_SIGNATURE).into();
			let mining_bot_signature: Signature =
				sr25519::Signature::from_raw(BENCH_MINING_BOT_SIGNATURE).into();
			let access_code_signature = sr25519::Signature::from_raw(BENCH_ACCESS_CODE_SIGNATURE);

			assert!(
				operational_signature.verify(
					(
						OPERATIONAL_ACCOUNT_PROOF_MESSAGE_KEY,
						&operational_account,
						&operational_account,
					)
						.using_encoded(blake2_256)
						.as_slice(),
					&operational_account,
				)
			);
			assert!(
				vault_signature.verify(
					(VAULT_ACCOUNT_PROOF_MESSAGE_KEY, &operational_account, &vault_account)
						.using_encoded(blake2_256)
						.as_slice(),
					&vault_account,
				)
			);
			assert!(
				mining_funding_signature.verify(
					(
						MINING_FUNDING_ACCOUNT_PROOF_MESSAGE_KEY,
						&operational_account,
						&mining_funding_account,
					)
						.using_encoded(blake2_256)
						.as_slice(),
					&mining_funding_account,
				)
			);
			assert!(
				mining_bot_signature.verify(
					(
						MINING_BOT_ACCOUNT_PROOF_MESSAGE_KEY,
						&operational_account,
						&mining_bot_account,
					)
						.using_encoded(blake2_256)
						.as_slice(),
					&mining_bot_account,
				)
			);
			assert!(
				access_code_signature.verify(
					(ACCESS_CODE_PROOF_MESSAGE_KEY, access_code_public, &operational_account,)
						.using_encoded(blake2_256)
						.as_slice(),
					&access_code_public,
				)
			);
		}

		#[test]
		#[ignore = "manual helper to print the fixed benchmark proof fixture data"]
		fn print_register_fixture_data() {
			println!("operational_account={:?}", BENCH_OPERATIONAL_ACCOUNT);
			println!("vault_account={:?}", BENCH_VAULT_ACCOUNT);
			println!("mining_funding_account={:?}", BENCH_MINING_FUNDING_ACCOUNT);
			println!("mining_bot_account={:?}", BENCH_MINING_BOT_ACCOUNT);
			println!("access_code_public={:?}", BENCH_ACCESS_CODE_PUBLIC);
			println!("operational_signature={:?}", BENCH_OPERATIONAL_SIGNATURE);
			println!("vault_signature={:?}", BENCH_VAULT_SIGNATURE);
			println!("mining_funding_signature={:?}", BENCH_MINING_FUNDING_SIGNATURE);
			println!("mining_bot_signature={:?}", BENCH_MINING_BOT_SIGNATURE);
			println!("access_code_signature={:?}", BENCH_ACCESS_CODE_SIGNATURE);
		}
	}
}
