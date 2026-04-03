#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_primitives::{
	OperationalRewardKind, OperationalRewardPayout, TreasuryPoolProvider, VaultId,
	bitcoin::Satoshis,
	vault::{Vault, VaultTerms},
};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet_prelude::{
	argon_primitives::{
		MiningFrameTransitionProvider, OperationalRewardsPayer, OperationalRewardsProvider,
	},
	benchmarking::{
		BenchmarkBitcoinVaultProviderState, BenchmarkOperationalRewardsProviderState,
		BenchmarkPriceProviderState, benchmark_bitcoin_vault_provider_state,
		benchmark_operational_rewards_provider_state, reset_benchmark_bitcoin_vault_provider_state,
		reset_benchmark_operational_rewards_provider_state, reset_benchmark_price_provider_state,
		set_benchmark_bitcoin_vault_provider_state,
		set_benchmark_operational_rewards_provider_state, set_benchmark_price_provider_state,
	},
};
use polkadot_sdk::{
	frame_support::{
		BoundedBTreeMap, BoundedVec,
		traits::fungible::{Inspect, InspectHold, Mutate},
	},
	sp_arithmetic::FixedU128,
	sp_runtime::Permill,
};

const BENCHMARK_FRAME_ID: FrameId = 20;
type TreasuryBalanceOf<T> = <T as Config>::Balance;

#[benchmarks(
	where
		T::AccountId: Ord
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn set_allocation() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("allocation-caller", 0, 0);
		let vault_id: VaultId = 1;
		let current_frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
		let principal = balance::<T>(3_000_000_000);
		let new_commitment = balance::<T>(3_500_000_000);
		let expected_additional_hold = balance::<T>(500_000_000);

		T::Currency::mint_into(&caller, balance::<T>(5_000_000_000))
			.map_err(|_| BenchmarkError::Stop("failed to fund benchmark caller"))?;
		Pallet::<T>::create_hold(&caller, principal)
			.map_err(|_| BenchmarkError::Stop("failed to create existing hold"))?;

		FunderStateByVaultAndAccount::<T>::insert(
			vault_id,
			&caller,
			FunderState {
				held_principal: principal,
				lifetime_principal_last_basis_frame: current_frame_id,
				..Default::default()
			},
		);
		Pallet::<T>::refresh_funder_index(vault_id, &caller, principal);
		seed_competing_funders_for_set_allocation::<T>(vault_id, &caller, current_frame_id)?;
		seed_pending_unlocks_for_set_allocation::<T>(vault_id, &caller, current_frame_id)?;
		whitelist_account!(caller);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_id, new_commitment);

		let state = FunderStateByVaultAndAccount::<T>::get(vault_id, &caller)
			.ok_or(BenchmarkError::Stop("missing funder state after set_allocation"))?;
		assert_eq!(
			state.held_principal,
			principal.saturating_add(expected_additional_hold),
			"expected set_allocation benchmark to add hold after canceling pending unlocks",
		);
		assert_eq!(
			state.pending_unlock_amount,
			TreasuryBalanceOf::<T>::zero(),
			"expected benchmark to cancel the queued unlock",
		);
		assert_eq!(state.pending_unlock_at_frame, None);
		assert_eq!(
			T::Currency::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &caller),
			principal.saturating_add(expected_additional_hold),
			"expected benchmark caller hold to include the additional commitment",
		);
		Ok(())
	}

	#[benchmark]
	fn try_pay_reward() -> Result<(), BenchmarkError> {
		let payout_account: T::AccountId = account("reward-payout", 0, 0);
		let operational_account: T::AccountId = account("reward-operational", 0, 0);
		let reward_amount = balance::<T>(1_000_000_000);
		let reserves_funding = balance::<T>(2_000_000_000);
		let reserves_account = Pallet::<T>::get_treasury_reserves_account();
		let reward = OperationalRewardPayout {
			operational_account,
			payout_account: payout_account.clone(),
			reward_kind: OperationalRewardKind::Activation,
			amount: reward_amount,
		};

		T::Currency::mint_into(&reserves_account, reserves_funding)
			.map_err(|_| BenchmarkError::Stop("failed to fund treasury reserves"))?;

		#[block]
		{
			assert!(
				<Pallet<T> as OperationalRewardsPayer<T::AccountId, TreasuryBalanceOf<T>>>::try_pay_reward(
					&reward,
				)
			);
		}

		assert_eq!(T::Currency::balance(&payout_account), reward_amount);
		Ok(())
	}

	#[benchmark]
	fn pay_operational_rewards() -> Result<(), BenchmarkError> {
		seed_pay_operational_rewards_state::<T>()?;

		#[block]
		{
			let payouts = T::OperationalRewardsProvider::pending_rewards();
			Pallet::<T>::pay_operational_rewards(payouts);
		}

		assert!(
			benchmark_operational_rewards_provider_state::<T::AccountId, TreasuryBalanceOf<T>>()
				.pending_rewards
				.is_empty(),
			"expected all queued rewards to be marked paid",
		);
		Ok(())
	}

	#[benchmark]
	fn provider_has_pool_participation() -> Result<(), BenchmarkError> {
		let account_id: T::AccountId = account("treasury_pool_participant", 0, 0);
		let vault_id: VaultId = 1;
		FunderStateByVaultAndAccount::<T>::insert(
			vault_id,
			&account_id,
			FunderState { held_principal: balance::<T>(1_000_000_000), ..Default::default() },
		);

		#[block]
		{
			assert!(<Pallet<T> as TreasuryPoolProvider<T::AccountId>>::has_pool_participation(
				vault_id,
				&account_id,
			));
		}

		Ok(())
	}

	#[benchmark]
	fn release_pending_unlocks() -> Result<(), BenchmarkError> {
		seed_release_pending_unlocks_state::<T>(BENCHMARK_FRAME_ID)?;

		#[block]
		{
			Pallet::<T>::release_pending_unlocks(BENCHMARK_FRAME_ID);
		}

		let sample_account: T::AccountId = account("pending-unlock", 0, 0);
		assert!(
			PendingUnlocksByFrame::<T>::get(BENCHMARK_FRAME_ID).is_empty(),
			"expected benchmark frame unlock queue to be emptied",
		);
		assert!(
			FunderStateByVaultAndAccount::<T>::get(10_000, &sample_account).is_none(),
			"expected sample pending unlock funder state to be removed after release",
		);
		assert!(
			!VaultPoolsByFrame::<T>::contains_key(
				BENCHMARK_FRAME_ID.saturating_sub(benchmark_exit_delay_frames::<T>()),
			),
			"expected matured historical pool snapshot to be removed",
		);
		Ok(())
	}

	#[benchmark]
	fn distribute_bid_pool() -> Result<(), BenchmarkError> {
		seed_bid_pool_distribution_state::<T>(BENCHMARK_FRAME_ID)?;
		let payout_frame = BENCHMARK_FRAME_ID.saturating_sub(1);

		#[block]
		{
			Pallet::<T>::distribute_bid_pool(payout_frame);
		}

		assert!(
			CapitalActive::<T>::get().is_empty(),
			"expected capital snapshot to be consumed during distribution",
		);
		assert_eq!(
			benchmark_bitcoin_vault_provider_state::<T::AccountId, TreasuryBalanceOf<T>>()
				.treasury_frame_earnings
				.len(),
			T::MaxVaultsPerPool::get() as usize,
			"expected every selected vault to record frame earnings",
		);
		Ok(())
	}

	#[benchmark]
	fn lock_in_vault_capital() -> Result<(), BenchmarkError> {
		seed_lock_in_vault_capital_state::<T>(BENCHMARK_FRAME_ID)?;

		#[block]
		{
			Pallet::<T>::lock_in_vault_capital(BENCHMARK_FRAME_ID);
		}

		assert!(
			VaultPoolsByFrame::<T>::contains_key(BENCHMARK_FRAME_ID),
			"expected current frame vault pools to be stored",
		);
		assert_eq!(
			CapitalActive::<T>::get().len(),
			T::MaxVaultsPerPool::get() as usize,
			"expected benchmark to fill the next frame capital snapshot",
		);
		Ok(())
	}

	#[benchmark]
	fn on_frame_transition() -> Result<(), BenchmarkError> {
		seed_on_frame_transition::<T>(BENCHMARK_FRAME_ID)?;

		#[block]
		{
			Pallet::<T>::run_frame_transition(BENCHMARK_FRAME_ID);
		}

		assert!(
			PendingUnlocksByFrame::<T>::get(BENCHMARK_FRAME_ID).is_empty(),
			"expected benchmark frame unlock queue to be emptied",
		);
		assert!(
			VaultPoolsByFrame::<T>::contains_key(BENCHMARK_FRAME_ID),
			"expected next frame vault pools to be stored",
		);
		assert_eq!(
			CapitalActive::<T>::get().len(),
			T::MaxVaultsPerPool::get() as usize,
			"expected benchmark to fill the next frame capital snapshot",
		);
		Ok(())
	}
}

fn seed_on_frame_transition<T: Config>(frame_id: FrameId) -> Result<(), BenchmarkError>
where
	T::AccountId: Ord,
{
	reset_transition_benchmark_state::<T>();
	seed_transition_vault_state::<T>(frame_id, true, true)?;
	seed_release_pending_unlock_entries::<T>(frame_id)?;

	Ok(())
}

fn seed_release_pending_unlocks_state<T: Config>(frame_id: FrameId) -> Result<(), BenchmarkError>
where
	T::AccountId: Ord,
{
	reset_transition_benchmark_state::<T>();
	seed_historical_pool_frames::<T>(frame_id)?;
	seed_release_pending_unlock_entries::<T>(frame_id)?;

	Ok(())
}

fn seed_bid_pool_distribution_state<T: Config>(frame_id: FrameId) -> Result<(), BenchmarkError>
where
	T::AccountId: Ord,
{
	reset_transition_benchmark_state::<T>();
	seed_transition_vault_state::<T>(frame_id, true, false)?;

	Ok(())
}

fn seed_lock_in_vault_capital_state<T: Config>(frame_id: FrameId) -> Result<(), BenchmarkError>
where
	T::AccountId: Ord,
{
	reset_transition_benchmark_state::<T>();
	seed_transition_vault_state::<T>(frame_id, false, true)?;

	Ok(())
}

fn reset_transition_benchmark_state<T: Config>() {
	reset_benchmark_bitcoin_vault_provider_state();
	reset_benchmark_operational_rewards_provider_state();
	reset_benchmark_price_provider_state();
	set_benchmark_price_provider_state(BenchmarkPriceProviderState {
		btc_price_in_usd: Some(FixedU128::saturating_from_integer(100u128)),
		argon_price_in_usd: Some(FixedU128::one()),
		argon_target_price_in_usd: Some(FixedU128::one()),
		circulation: 1_000_000,
	});
}

fn seed_transition_vault_state<T: Config>(
	frame_id: FrameId,
	include_distribution_state: bool,
	include_historical_state: bool,
) -> Result<(), BenchmarkError>
where
	T::AccountId: Ord,
{
	let vault_count = T::MaxVaultsPerPool::get();
	let contributor_principal = balance::<T>(1_000_000_000);
	let payout_frame = frame_id.saturating_sub(1);

	let mut benchmark_vault_state =
		BenchmarkBitcoinVaultProviderState::<T::AccountId, TreasuryBalanceOf<T>>::default();
	let mut active_capital = include_distribution_state
		.then(BoundedVec::<TreasuryCapital<T>, T::MaxVaultsPerPool>::default);
	let mut payout_pools = include_distribution_state
		.then(BoundedBTreeMap::<VaultId, TreasuryPool<T>, T::MaxVaultsPerPool>::new);

	for vault_index in 0..vault_count {
		let vault_id = vault_index.saturating_add(1);
		let operator: T::AccountId = account("treasury-operator", vault_index, 0);
		let (pool, raised) =
			seed_vault_funders::<T>(vault_id, &operator, vault_index, contributor_principal)?;

		let securitized_satoshis = raised.into().min(u64::MAX as u128) as Satoshis;
		let vault = benchmark_vault::<T>(operator, securitized_satoshis);
		benchmark_vault_state.vaults.insert(vault_id, vault);

		if let Some(active_capital) = active_capital.as_mut() {
			active_capital
				.try_push(TreasuryCapital {
					vault_id,
					activated_capital: raised,
					frame_id: payout_frame,
				})
				.map_err(|_| BenchmarkError::Stop("failed to seed active capital"))?;
		}
		if let Some(payout_pools) = payout_pools.as_mut() {
			payout_pools
				.try_insert(vault_id, pool)
				.map_err(|_| BenchmarkError::Stop("failed to seed payout pools"))?;
		}
	}

	set_benchmark_bitcoin_vault_provider_state(benchmark_vault_state);

	if let Some(payout_pools) = payout_pools {
		VaultPoolsByFrame::<T>::insert(payout_frame, payout_pools);
	}
	if let Some(active_capital) = active_capital {
		CapitalActive::<T>::put(active_capital);

		let bid_pool_account = Pallet::<T>::get_bid_pool_account();
		T::Currency::mint_into(&bid_pool_account, balance::<T>(10_000_000_000_000))
			.map_err(|_| BenchmarkError::Stop("failed to fund bid pool"))?;
	}
	if include_historical_state {
		seed_historical_pool_frames::<T>(frame_id)?;
	}

	Ok(())
}

fn seed_pay_operational_rewards_state<T: Config>() -> Result<(), BenchmarkError> {
	reset_benchmark_operational_rewards_provider_state();
	let max_rewards = T::OperationalRewardsProvider::max_pending_rewards();
	let reward_amount = balance::<T>(1_000_000_000);
	let total_rewards = reward_amount.saturating_mul((max_rewards as u128).into());
	let reserves_account = Pallet::<T>::get_treasury_reserves_account();

	T::Currency::mint_into(&reserves_account, total_rewards)
		.map_err(|_| BenchmarkError::Stop("failed to fund treasury reserves"))?;

	let mut pending_rewards = Vec::with_capacity(max_rewards as usize);

	for reward_index in 0..max_rewards {
		let operational_account: T::AccountId = account("operational-reward", reward_index, 0);
		let payout_account: T::AccountId = account("operational-payout", reward_index, 0);
		pending_rewards.push(OperationalRewardPayout {
			operational_account,
			payout_account,
			reward_kind: OperationalRewardKind::Activation,
			amount: reward_amount,
		});
	}

	set_benchmark_operational_rewards_provider_state(BenchmarkOperationalRewardsProviderState {
		pending_rewards,
		paid_rewards: Vec::new(),
		max_pending_rewards: max_rewards,
	});

	Ok(())
}

fn seed_release_pending_unlock_entries<T: Config>(frame_id: FrameId) -> Result<(), BenchmarkError> {
	let pending_unlock_amount = balance::<T>(100_000_000);

	for unlock_index in 0..T::MaxPendingUnlocksPerFrame::get() {
		let account_id: T::AccountId = account("pending-unlock", unlock_index, 0);
		let vault_id = 10_000u32.saturating_add(unlock_index);

		T::Currency::mint_into(&account_id, pending_unlock_amount)
			.map_err(|_| BenchmarkError::Stop("failed to fund pending unlock account"))?;
		Pallet::<T>::create_hold(&account_id, pending_unlock_amount)
			.map_err(|_| BenchmarkError::Stop("failed to hold pending unlock principal"))?;

		FunderStateByVaultAndAccount::<T>::insert(
			vault_id,
			&account_id,
			FunderState {
				held_principal: pending_unlock_amount,
				pending_unlock_amount,
				pending_unlock_at_frame: Some(frame_id),
				lifetime_principal_last_basis_frame: frame_id.saturating_sub(1),
				..Default::default()
			},
		);
		PendingUnlocksByFrame::<T>::try_mutate(frame_id, |pending_unlocks| {
			pending_unlocks
				.try_push(PendingUnlock { vault_id, account_id: account_id.clone() })
				.map_err(|_| BenchmarkError::Stop("failed to seed pending unlock"))
		})?;
	}

	Ok(())
}

fn seed_historical_pool_frames<T: Config>(frame_id: FrameId) -> Result<(), BenchmarkError>
where
	T::AccountId: Ord,
{
	let vault_count = T::MaxVaultsPerPool::get();
	let contributor_count = T::MaxTreasuryContributors::get();
	let contributor_principal = balance::<T>(1_000_000_000);
	let retained_history_frames = benchmark_retained_history_frames::<T>();
	let first_historical_frame = frame_id.saturating_sub(retained_history_frames + 1);

	for frame_offset in 0..retained_history_frames {
		let history_frame = first_historical_frame.saturating_add(frame_offset);
		let history_frame_index =
			u32::try_from(history_frame).expect("benchmark historical frame fits in u32");
		let mut historical_pools =
			BoundedBTreeMap::<VaultId, TreasuryPool<T>, T::MaxVaultsPerPool>::new();

		for vault_index in 0..vault_count {
			let vault_id = vault_index.saturating_add(1);
			let history_operator: T::AccountId =
				account("historical-operator", history_frame_index.saturating_add(vault_index), 0);
			let mut historical_pool = TreasuryPool::<T> {
				vault_sharing_percent: Permill::from_percent(20),
				..Default::default()
			};

			for contributor_index in 0..contributor_count {
				let account_id = if contributor_index == 0 {
					history_operator.clone()
				} else {
					account(
						"historical-funder",
						history_frame_index
							.saturating_mul(vault_count)
							.saturating_mul(contributor_count)
							.saturating_add(vault_index.saturating_mul(contributor_count))
							.saturating_add(contributor_index),
						0,
					)
				};
				historical_pool
					.try_insert_bond_holder(
						account_id,
						contributor_principal,
						Some(&history_operator),
					)
					.map_err(|_| BenchmarkError::Stop("failed to seed historical pool"))?;
			}

			historical_pools
				.try_insert(vault_id, historical_pool)
				.map_err(|_| BenchmarkError::Stop("failed to insert historical pool"))?;
		}

		VaultPoolsByFrame::<T>::insert(history_frame, historical_pools);
	}

	Ok(())
}

fn seed_vault_funders<T: Config>(
	vault_id: VaultId,
	operator: &T::AccountId,
	vault_seed: u32,
	contributor_principal: TreasuryBalanceOf<T>,
) -> Result<(TreasuryPool<T>, TreasuryBalanceOf<T>), BenchmarkError> {
	let mut pool = TreasuryPool::<T> {
		vault_sharing_percent: Permill::from_percent(20),
		..Default::default()
	};
	let tracked_count = T::MaxTrackedTreasuryFunders::get();

	for contributor_index in 0..tracked_count {
		let account_id = if contributor_index == 0 {
			operator.clone()
		} else {
			account(
				"treasury-funder",
				vault_seed.saturating_mul(tracked_count).saturating_add(contributor_index),
				0,
			)
		};

		FunderStateByVaultAndAccount::<T>::insert(
			vault_id,
			&account_id,
			FunderState { held_principal: contributor_principal, ..Default::default() },
		);
		Pallet::<T>::refresh_funder_index(vault_id, &account_id, contributor_principal);
		pool.try_insert_bond_holder(account_id, contributor_principal, Some(operator))
			.map_err(|_| BenchmarkError::Stop("failed to seed payout pool"))?;
	}

	Ok((pool.clone(), pool.raised_capital()))
}

fn seed_competing_funders_for_set_allocation<T: Config>(
	vault_id: VaultId,
	caller: &T::AccountId,
	current_frame_id: FrameId,
) -> Result<(), BenchmarkError>
where
	T::AccountId: Ord,
{
	let tracked_slots = T::MaxTrackedTreasuryFunders::get();
	let competing_principal = balance::<T>(2_000_000_000);

	for competitor_index in 0..tracked_slots.saturating_sub(1) {
		let account_id: T::AccountId = account("allocation-competitor", competitor_index, 0);
		if account_id == *caller {
			continue;
		}

		T::Currency::mint_into(&account_id, competing_principal)
			.map_err(|_| BenchmarkError::Stop("failed to fund competitor"))?;
		Pallet::<T>::create_hold(&account_id, competing_principal)
			.map_err(|_| BenchmarkError::Stop("failed to hold competitor principal"))?;
		FunderStateByVaultAndAccount::<T>::insert(
			vault_id,
			&account_id,
			FunderState {
				held_principal: competing_principal,
				lifetime_principal_last_basis_frame: current_frame_id,
				..Default::default()
			},
		);
		Pallet::<T>::refresh_funder_index(vault_id, &account_id, competing_principal);
	}

	Ok(())
}

fn seed_pending_unlocks_for_set_allocation<T: Config>(
	vault_id: VaultId,
	account_id: &T::AccountId,
	current_frame_id: FrameId,
) -> Result<(), BenchmarkError> {
	let unlock_frame = current_frame_id.saturating_add(benchmark_exit_delay_frames::<T>());
	let unrelated_account: T::AccountId = account("unrelated-pending", 0, 0);
	let target_amount = balance::<T>(1_000_000_000);

	FunderStateByVaultAndAccount::<T>::mutate(vault_id, account_id, |entry| {
		let mut state = entry.take().unwrap_or_default();
		state.pending_unlock_amount = target_amount;
		state.pending_unlock_at_frame = Some(unlock_frame);
		*entry = Some(state);
	});

	PendingUnlocksByFrame::<T>::try_mutate(unlock_frame, |pending_unlocks| {
		for unrelated_index in 0..T::MaxPendingUnlocksPerFrame::get().saturating_sub(1) {
			pending_unlocks
				.try_push(PendingUnlock {
					vault_id: vault_id.saturating_add(10_000).saturating_add(unrelated_index),
					account_id: unrelated_account.clone(),
				})
				.map_err(|_| BenchmarkError::Stop("failed to seed unrelated pending unlock"))?;
		}
		pending_unlocks
			.try_push(PendingUnlock { vault_id, account_id: account_id.clone() })
			.map_err(|_| BenchmarkError::Stop("failed to seed target pending unlock"))?;
		Ok::<(), BenchmarkError>(())
	})?;

	Ok(())
}

fn benchmark_vault<T: Config>(
	operator: T::AccountId,
	securitized_satoshis: Satoshis,
) -> Vault<T::AccountId, TreasuryBalanceOf<T>> {
	Vault {
		operator_account_id: operator,
		bitcoin_lock_delegate_account: None,
		securitization: TreasuryBalanceOf::<T>::zero(),
		securitization_target: TreasuryBalanceOf::<T>::zero(),
		securitization_locked: TreasuryBalanceOf::<T>::zero(),
		securitization_pending_activation: TreasuryBalanceOf::<T>::zero(),
		locked_satoshis: 0,
		securitized_satoshis,
		securitization_release_schedule: Default::default(),
		securitization_ratio: FixedU128::one(),
		is_closed: false,
		terms: VaultTerms {
			bitcoin_annual_percent_rate: FixedU128::one(),
			bitcoin_base_fee: TreasuryBalanceOf::<T>::zero(),
			treasury_profit_sharing: Permill::from_percent(20),
		},
		pending_terms: None,
		opened_tick: 0,
		operational_minimum_release_tick: None,
	}
}

fn balance<T: Config>(amount: u128) -> TreasuryBalanceOf<T> {
	amount.into()
}

fn benchmark_exit_delay_frames<T: Config>() -> FrameId {
	T::TreasuryExitDelayFrames::get()
}

fn benchmark_retained_history_frames<T: Config>() -> FrameId {
	benchmark_exit_delay_frames::<T>().saturating_sub(1)
}
