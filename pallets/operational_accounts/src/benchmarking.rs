//! Benchmarking setup for pallet-operational-accounts
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[cfg(test)]
use crate::mock::{
	new_test_ext, record_funded_bitcoin_amount, record_microgons_in, set_registration_lookup,
	ClaimableTreasuryBalance, MinimumBitcoin, MinimumBonds, MinimumUniswapTransfer,
	OperationalMinimumVaultSecuritization, Test, TestAccountId,
};
#[allow(unused)]
use crate::Pallet as OperationalAccountsPallet;
use argon_primitives::{OperationalAccountsHook, UtxoLockEvents, MICROGONS_PER_ARGON};
use codec::Decode;
#[cfg(test)]
use codec::Encode;
use frame_system::RawOrigin;
use pallet_prelude::{
	benchmarking::{
		benchmark_operational_accounts_provider_call_counters,
		reset_benchmark_operational_accounts_provider_call_counters,
		reset_benchmark_operational_accounts_provider_state,
		set_benchmark_operational_accounts_provider_state,
		BenchmarkOperationalAccountsProviderCallCounters,
		BenchmarkOperationalAccountsProviderState,
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
const BENCH_MINING_ACCOUNT: [u8; 32] = [
	142, 229, 4, 20, 142, 117, 195, 78, 143, 5, 24, 153, 179, 198, 228, 36, 31, 241, 141, 193, 201,
	33, 18, 96, 182, 166, 164, 52, 190, 219, 72, 95,
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
const BENCH_MINING_ACCOUNT_SIGNATURE: [u8; 64] = [
	198, 19, 229, 142, 158, 206, 161, 89, 52, 138, 239, 238, 61, 243, 178, 227, 34, 135, 47, 31,
	131, 165, 92, 199, 109, 211, 96, 42, 32, 41, 17, 33, 44, 252, 182, 102, 104, 87, 223, 157, 11,
	135, 111, 189, 98, 83, 208, 56, 143, 194, 77, 72, 165, 46, 134, 218, 194, 69, 34, 97, 215, 124,
	158, 129,
];

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn register() {
		reset_benchmark_operational_accounts_provider_state();
		reset_benchmark_operational_accounts_provider_call_counters();
		IsOperationalAccountInviteOnly::<T>::put(false);
		let caller = benchmark_account_id::<T>(BENCH_OPERATIONAL_ACCOUNT);
		let operational_account_proof = benchmark_account_proof(BENCH_OPERATIONAL_SIGNATURE);
		let vault = benchmark_account_id::<T>(BENCH_VAULT_ACCOUNT);
		let mining = benchmark_account_id::<T>(BENCH_MINING_ACCOUNT);
		let vault_proof = benchmark_account_proof(BENCH_VAULT_SIGNATURE);
		let mining_proof = benchmark_account_proof(BENCH_MINING_ACCOUNT_SIGNATURE);
		#[cfg(test)]
		{
			let linked = LinkedAccounts::<T> {
				owner: caller.clone(),
				vault: vault.clone(),
				mining: mining.clone(),
			};
			seed_mock_registration_lookup(&linked);
			seed_mock_linked_uniswap_argon_transfers_in(&linked);
			record_funded_bitcoin_amount(
				&mock_account_id::<T>(&linked.vault),
				MinimumBitcoin::get(),
			);
		}
		set_benchmark_operational_accounts_provider_state(default_provider_state::<T>());
		let registration = Registration::V1(RegistrationV1 {
			operational_account: caller.clone(),
			encryption_pubkey: OpaqueEncryptionPubkey([7u8; 32]),
			operational_account_proof,
			vault_account: vault.clone(),
			mining_account: mining.clone(),
			vault_account_proof: vault_proof,
			mining_account_proof: mining_proof,
			access_proof: None,
		});
		whitelist_account!(caller);
		#[extrinsic_call]
		register(RawOrigin::Signed(caller.clone()), registration);

		let expected_uniswap_argon_transfers_in_amount = T::MinimumUniswapTransfer::get()
			.saturating_add(T::MinimumUniswapTransfer::get())
			.saturating_add(T::MinimumUniswapTransfer::get());
		let account = OperationalAccounts::<T>::get(&caller).expect("account stored");
		assert_eq!(
			account.uniswap_argon_transfers_in_amount,
			expected_uniswap_argon_transfers_in_amount
		);
		assert_eq!(account.vault_bitcoin_accrual, T::MinimumBitcoin::get());
		assert!(account.vault_created);
		assert_eq!(account.mining_seat_accrual, 1);
		assert!(!account.is_operationally_certified);
		assert!(OperationalAccountBySubAccount::<T>::contains_key(vault));
		assert!(OperationalAccountBySubAccount::<T>::contains_key(mining));
		assert_provider_calls(BenchmarkOperationalAccountsProviderCallCounters {
			get_registration_vault_data: 1,
			get_account_funded_bitcoin_amount: 1,
			is_eligible: 0,
			has_active_rewards_account_seat: 1,
			has_vault_bond_participation: 0,
			active_vault_bond_amount: 0,
			active_account_vault_bond_amount: 1,
			is_crosschain_activated: 1,
			account_uniswap_argon_transfers_in_amount: 3,
			account_became_operational: 0,
		});
	}

	#[benchmark]
	fn on_vault_created() {
		set_benchmark_operational_accounts_provider_state(default_provider_state::<T>());
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.uniswap_argon_transfers_in_amount = T::OperationalMinimumUniswapTransfer::get();
		account.vault_bitcoin_accrual = T::MinimumBitcoin::get();
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_vault_to_owner::<T>(&linked);
		#[cfg(test)]
		{
			seed_mock_registration_lookup(&linked);
			seed_mock_linked_uniswap_argon_transfers_in(&linked);
		}

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
	fn on_vault_bitcoin_lock_funded() {
		set_benchmark_operational_accounts_provider_state(default_provider_state::<T>());
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.uniswap_argon_transfers_in_amount = T::MinimumUniswapTransfer::get();
		account.vault_created = true;
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_vault_to_owner::<T>(&linked);
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);

		#[block]
		{
			let _ = OperationalAccountsPallet::<T>::vault_bitcoin_lock_funded(
				&linked.vault,
				T::MinimumBitcoin::get(),
			);
		}

		let account = OperationalAccounts::<T>::get(&linked.owner).expect("account should exist");
		assert_eq!(account.vault_bitcoin_accrual, T::MinimumBitcoin::get());
	}

	#[benchmark]
	fn on_mining_seat_won() {
		set_benchmark_operational_accounts_provider_state(default_provider_state::<T>());
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.uniswap_argon_transfers_in_amount = T::OperationalMinimumUniswapTransfer::get();
		account.vault_created = true;
		account.vault_bitcoin_accrual = T::MinimumBitcoin::get();
		account.mining_seat_accrual = T::MiningSeatsForOperational::get().saturating_sub(1);
		insert_operational_account::<T>(&linked, account);
		link_mining_to_owner::<T>(&linked);
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);

		#[block]
		{
			let _ = OperationalAccountsPallet::<T>::mining_seat_won(&linked.mining);
		}

		assert_eq!(
			OperationalAccounts::<T>::get(&linked.owner)
				.expect("account should exist")
				.mining_seat_accrual,
			T::MiningSeatsForOperational::get()
		);
	}

	#[benchmark]
	fn on_account_bitcoin_amount_updated() {
		set_benchmark_operational_accounts_provider_state(default_provider_state::<T>());
		let linked = linked_accounts::<T>();
		insert_operational_account::<T>(&linked, default_operational_account::<T>(&linked));

		#[block]
		{
			let _ = <OperationalAccountsPallet<T> as UtxoLockEvents<T::AccountId, T::Balance>>::utxo_locked(
				1u64,
				&linked.vault,
				T::MinimumBitcoin::get(),
			);
		}

		assert_eq!(
			OperationalAccounts::<T>::get(&linked.owner)
				.expect("account should exist")
				.account_bitcoin_amount,
			T::MinimumBitcoin::get()
		);
	}

	#[benchmark]
	fn on_account_vault_bond_total_updated() {
		set_benchmark_operational_accounts_provider_state(default_provider_state::<T>());
		let linked = linked_accounts::<T>();
		insert_operational_account::<T>(&linked, default_operational_account::<T>(&linked));
		link_vault_to_owner::<T>(&linked);
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);

		#[block]
		{
			OperationalAccountsPallet::<T>::account_vault_bond_total_updated(
				&linked.vault,
				T::MinimumBonds::get(),
			);
		}

		assert_eq!(
			OperationalAccounts::<T>::get(&linked.owner)
				.expect("account should exist")
				.account_vault_bond_amount,
			T::MinimumBonds::get()
		);
	}

	#[benchmark]
	fn on_account_uniswap_argon_transfers_in_updated() {
		set_benchmark_operational_accounts_provider_state(default_provider_state::<T>());
		let linked = linked_accounts::<T>();
		let mut account = default_operational_account::<T>(&linked);
		account.account_bitcoin_amount = T::MinimumBitcoin::get();
		account.account_vault_bond_amount = T::MinimumBonds::get();
		account.vault_created = true;
		account.vault_bitcoin_accrual = T::MinimumBitcoin::get();
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_vault_to_owner::<T>(&linked);
		#[cfg(test)]
		{
			seed_mock_registration_lookup(&linked);
			seed_mock_linked_uniswap_argon_transfers_in(&linked);
		}

		#[block]
		{
			OperationalAccountsPallet::<T>::refresh_account_uniswap_argon_transfers_in_amount(
				&linked.vault,
			);
		}

		let account = OperationalAccounts::<T>::get(&linked.owner).expect("account should exist");
		let expected_uniswap_argon_transfers_in_amount = T::MinimumUniswapTransfer::get()
			.saturating_add(T::MinimumUniswapTransfer::get())
			.saturating_add(T::MinimumUniswapTransfer::get());
		assert_eq!(
			account.uniswap_argon_transfers_in_amount,
			expected_uniswap_argon_transfers_in_amount
		);
		assert!(meets_minimums::<T>(&account));
	}

	#[benchmark]
	fn activate() {
		set_benchmark_operational_accounts_provider_state(default_provider_state::<T>());
		let linked = linked_accounts::<T>();
		let upstream_account = linked_accounts_with_seed::<T>(1);
		let mut upstream_account_data = default_operational_account::<T>(&upstream_account);
		upstream_account_data.is_operationally_certified = true;
		upstream_account_data.operational_certifications_count =
			T::OperationalCertificationsPerBonusReward::get().saturating_sub(1);
		insert_operational_account::<T>(&upstream_account, upstream_account_data);
		let mut account = default_operational_account::<T>(&linked);
		account.upstream_account = Some(upstream_account.owner.clone());
		account.account_bitcoin_amount = T::MinimumBitcoin::get();
		account.account_vault_bond_amount = T::MinimumBonds::get();
		account.uniswap_argon_transfers_in_amount = T::OperationalMinimumUniswapTransfer::get();
		account.vault_created = true;
		account.vault_bitcoin_accrual = T::MinimumBitcoin::get();
		account.mining_seat_accrual = T::MiningSeatsForOperational::get();
		insert_operational_account::<T>(&linked, account);
		link_mining_to_owner::<T>(&linked);
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);
		let caller = linked.mining.clone();
		whitelist_account!(caller);

		#[extrinsic_call]
		activate(RawOrigin::Signed(caller.clone()));

		assert!(
			OperationalAccounts::<T>::get(&linked.owner)
				.expect("account should exist")
				.is_operationally_certified
		);
		let upstream_account_data = OperationalAccounts::<T>::get(&upstream_account.owner)
			.expect("upstream account should exist");
		assert!(upstream_account_data.operational_certifications_count > 0);
	}

	#[benchmark]
	fn set_reward_config() {
		let operational_certification_reward = T::Balance::from(1_000u128);
		let operational_certification_bonus_reward = T::Balance::from(500u128);
		#[extrinsic_call]
		set_reward_config(
			RawOrigin::Root,
			operational_certification_reward,
			operational_certification_bonus_reward,
		);

		let config = Rewards::<T>::get();
		assert_eq!(config.operational_certification_reward, operational_certification_reward);
		assert_eq!(
			config.operational_certification_bonus_reward,
			operational_certification_bonus_reward
		);
	}

	#[benchmark]
	fn claim_rewards() {
		let linked = linked_accounts::<T>();
		let mut operational_account = default_operational_account::<T>(&linked);
		let claim_amount = T::Balance::from(MICROGONS_PER_ARGON);
		operational_account.rewards_earned_amount = claim_amount;
		insert_operational_account::<T>(&linked, operational_account);
		link_mining_to_owner::<T>(&linked);
		let claimant = linked.mining.clone();
		#[cfg(test)]
		ClaimableTreasuryBalance::set(claim_amount.into());
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
		set_benchmark_operational_accounts_provider_state(default_provider_state::<T>());
		let linked = linked_accounts::<T>();
		let account = default_operational_account::<T>(&linked);
		insert_operational_account::<T>(&linked, account);
		let patch = OperationalProgressPatch {
			uniswap_argon_transfers_in_amount: Some(T::OperationalMinimumUniswapTransfer::get()),
			account_bitcoin_amount: Some(T::MinimumBitcoin::get()),
			account_vault_bond_amount: Some(T::MinimumBonds::get()),
			vault_created: Some(true),
			vault_bitcoin_amount: Some(T::MinimumBitcoin::get()),
			mining_seat_count: Some(T::MiningSeatsForOperational::get()),
		};
		#[cfg(test)]
		seed_mock_registration_lookup(&linked);

		#[extrinsic_call]
		force_set_progress(RawOrigin::Root, linked.owner.clone(), patch, true);

		let account = OperationalAccounts::<T>::get(&linked.owner).expect("account exists");
		assert!(meets_minimums::<T>(&account));
		assert!(!account.is_operationally_certified);
	}

	#[benchmark]
	fn set_encrypted_server_for_downstream_account() {
		let upstream_account = linked_accounts::<T>();
		let downstream_account = LinkedAccounts {
			owner: account("downstream_account_owner", 0, USER_SEED),
			vault: account("downstream_account_vault", 0, USER_SEED),
			mining: account("downstream_account_mining", 0, USER_SEED),
		};
		insert_operational_account::<T>(
			&upstream_account,
			default_operational_account::<T>(&upstream_account),
		);
		let mut downstream_account_data = default_operational_account::<T>(&downstream_account);
		downstream_account_data.upstream_account = Some(upstream_account.owner.clone());
		insert_operational_account::<T>(&downstream_account, downstream_account_data);
		let encrypted_server = vec![7u8; 32];
		let upstream_account_owner = upstream_account.owner.clone();
		whitelist_account!(upstream_account_owner);

		#[extrinsic_call]
		set_encrypted_server_for_downstream_account(
			RawOrigin::Signed(upstream_account_owner),
			downstream_account.owner.clone(),
			encrypted_server.clone(),
		);

		assert_eq!(
			EncryptedServerByDownstreamAccount::<T>::get(&downstream_account.owner)
				.expect("payload stored")
				.to_vec(),
			encrypted_server
		);
	}

	impl_benchmark_test_suite!(OperationalAccountsPallet, new_test_ext(), Test);

	fn assert_provider_calls(expected: BenchmarkOperationalAccountsProviderCallCounters) {
		if !cfg!(test) {
			assert_eq!(benchmark_operational_accounts_provider_call_counters(), expected);
		}
	}

	struct LinkedAccounts<T: Config> {
		owner: T::AccountId,
		vault: T::AccountId,
		mining: T::AccountId,
	}

	fn linked_accounts<T: Config>() -> LinkedAccounts<T> {
		linked_accounts_with_seed::<T>(0)
	}

	fn linked_accounts_with_seed<T: Config>(seed: u32) -> LinkedAccounts<T> {
		LinkedAccounts {
			owner: account("owner", seed, USER_SEED),
			vault: account("vault", seed, USER_SEED),
			mining: account("mining", seed, USER_SEED),
		}
	}

	fn default_operational_account<T: Config>(linked: &LinkedAccounts<T>) -> OperationalAccount<T> {
		OperationalAccount {
			vault_account: linked.vault.clone(),
			mining_account: linked.mining.clone(),
			encryption_pubkey: OpaqueEncryptionPubkey([0u8; 32]),
			upstream_account: None,
			uniswap_argon_transfers_in_amount: <T::Balance as Zero>::zero(),
			account_bitcoin_amount: <T::Balance as Zero>::zero(),
			account_vault_bond_amount: <T::Balance as Zero>::zero(),
			vault_created: false,
			vault_bitcoin_accrual: <T::Balance as Zero>::zero(),
			vault_bitcoin_applied_total: <T::Balance as Zero>::zero(),
			mining_seat_accrual: 0,
			mining_seat_applied_total: 0,
			operational_certifications_count: 0,
			access_code_pending: false,
			available_access_codes: 0,
			rewards_earned_count: 0,
			rewards_earned_amount: <T::Balance as Zero>::zero(),
			rewards_collected_amount: <T::Balance as Zero>::zero(),
			is_operationally_certified: false,
		}
	}

	fn meets_minimums<T: Config>(account: &OperationalAccount<T>) -> bool {
		OperationalAccountsPallet::<T>::meets_minimums(account)
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

	fn link_mining_to_owner<T: Config>(linked: &LinkedAccounts<T>) {
		OperationalAccountBySubAccount::<T>::insert(&linked.mining, &linked.owner);
	}

	fn benchmark_account_id<T: Config>(bytes: [u8; 32]) -> T::AccountId {
		T::AccountId::decode(&mut &bytes[..]).expect("benchmark account should decode")
	}

	fn benchmark_account_proof(signature: [u8; 64]) -> AccountOwnershipProof {
		AccountOwnershipProof { signature: sr25519::Signature::from_raw(signature).into() }
	}

	#[cfg(test)]
	fn seed_mock_registration_lookup<T: Config>(linked: &LinkedAccounts<T>) {
		let vault_account = mock_account_id::<T>(&linked.vault);
		let mining_account = mock_account_id::<T>(&linked.mining);
		set_registration_lookup(
			vault_account,
			mining_account,
			MinimumBitcoin::get(),
			OperationalMinimumVaultSecuritization::get(),
			MinimumBonds::get(),
			1,
		);
	}

	#[cfg(test)]
	fn seed_mock_linked_uniswap_argon_transfers_in<T: Config>(linked: &LinkedAccounts<T>) {
		record_microgons_in(&mock_account_id::<T>(&linked.owner), MinimumUniswapTransfer::get());
		record_microgons_in(&mock_account_id::<T>(&linked.vault), MinimumUniswapTransfer::get());
		record_microgons_in(&mock_account_id::<T>(&linked.mining), MinimumUniswapTransfer::get());
	}

	#[cfg(test)]
	fn mock_account_id<T: Config>(account_id: &T::AccountId) -> TestAccountId {
		TestAccountId::decode(&mut &account_id.encode()[..])
			.expect("benchmark mock account should decode")
	}

	fn default_provider_state<T: Config>() -> BenchmarkOperationalAccountsProviderState {
		BenchmarkOperationalAccountsProviderState {
			vault_registration_data: Some(argon_primitives::vault::RegistrationVaultData {
				vault_id: 1,
				activated_securitization: T::MinimumBitcoin::get().saturated_into(),
				securitization: T::OperationalMinimumVaultSecuritization::get().saturated_into(),
			}),
			account_bitcoin_amount: T::MinimumBitcoin::get().saturated_into(),
			is_eligible: true,
			has_active_rewards_account_seat: true,
			active_account_vault_bond_amount: T::MinimumBonds::get().saturated_into(),
			is_crosschain_activated: true,
			account_uniswap_argon_transfers_in_amount: T::MinimumUniswapTransfer::get()
				.saturated_into(),
			call_counters: Default::default(),
		}
	}
}
