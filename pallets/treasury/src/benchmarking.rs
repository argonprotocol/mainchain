#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_primitives::{
	MICROGONS_PER_ARGON, TreasuryPoolProvider, VaultId,
	bitcoin::Satoshis,
	vault::{Vault, VaultTerms},
};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet_prelude::{
	argon_primitives::OperationalRewardsPayer,
	benchmarking::{
		BenchmarkBitcoinVaultProviderState, BenchmarkPriceProviderState,
		benchmark_bitcoin_vault_provider_state, reset_benchmark_bitcoin_vault_provider_state,
		reset_benchmark_price_provider_state, set_benchmark_bitcoin_vault_provider_state,
		set_benchmark_price_provider_state,
	},
};
use polkadot_sdk::{
	frame_support::{
		BoundedVec,
		traits::fungible::{InspectHold, Mutate},
	},
	sp_arithmetic::FixedU128,
	sp_runtime::Permill,
};

const BENCHMARK_FRAME_ID: FrameId = 20;

type TreasuryBalanceOf<T> = <T as Config>::Balance;

#[benchmarks(
	where
		T::AccountId: Ord,
		T::Currency: Mutate<T::AccountId, Balance = T::Balance>
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn buy_bonds() -> Result<(), BenchmarkError> {
		reset_benchmark_state::<T>();

		let caller = account("buy-bonds-caller", 0, 0);
		let vault_id = 1;
		let lot_bonds = minimum_purchase_bonds::<T>();
		let purchase_bonds = lot_bonds.saturating_add(1);
		let security_bonds = scaled_bonds(lot_bonds, T::MaxTreasuryContributors::get())
			.saturating_add(purchase_bonds.saturating_sub(lot_bonds));
		let next_bond_lot_id = seed_accepted_vault_state::<T>(
			1,
			T::MaxTreasuryContributors::get(),
			lot_bonds,
			security_bonds,
			BENCHMARK_FRAME_ID.saturating_sub(1),
		)?;
		let evicted_bond_lot_id = next_bond_lot_id.saturating_sub(1);
		let purchase_amount = bonds_to_balance::<T>(purchase_bonds.saturating_mul(2));

		T::Currency::mint_into(&caller, purchase_amount)
			.map_err(|_| BenchmarkError::Stop("failed to fund benchmark buyer"))?;
		whitelist_account!(caller);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_id, purchase_bonds);

		assert!(BondLotById::<T>::contains_key(next_bond_lot_id));
		assert_eq!(
			BondLotsByVault::<T>::get(vault_id).len(),
			T::MaxTreasuryContributors::get() as usize,
			"expected accepted bond-lot list to stay full after purchase",
		);
		assert_eq!(
			BondLotById::<T>::get(evicted_bond_lot_id).and_then(|bond_lot| bond_lot.release_reason),
			Some(BondReleaseReason::Bumped),
		);
		Ok(())
	}

	#[benchmark]
	fn liquidate_bond_lot() -> Result<(), BenchmarkError> {
		reset_benchmark_state::<T>();

		let lot_bonds = minimum_purchase_bonds::<T>();
		let _ = seed_accepted_vault_state::<T>(
			1,
			1,
			lot_bonds,
			lot_bonds,
			BENCHMARK_FRAME_ID.saturating_sub(1),
		)?;
		let caller = benchmark_operator::<T>(0);
		let bond_lot_id = 0;

		whitelist_account!(caller);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), bond_lot_id);

		assert!(BondLotsByVault::<T>::get(1).is_empty());
		assert_eq!(
			BondLotById::<T>::get(bond_lot_id).and_then(|bond_lot| bond_lot.release_reason),
			Some(BondReleaseReason::UserLiquidation),
		);
		Ok(())
	}

	#[benchmark]
	fn claim_reward() -> Result<(), BenchmarkError> {
		let payout_account: T::AccountId = account("reward-payout", 0, 0);
		let reward_amount = balance::<T>(1_000_000_000);
		let minimum_balance = T::Currency::minimum_balance();
		let reserves_funding = reward_amount.saturating_add(minimum_balance);
		let reserves_account = T::TreasuryReservesAccount::get();

		T::Currency::mint_into(&reserves_account, reserves_funding)
			.map_err(|_| BenchmarkError::Stop("failed to fund treasury reserves"))?;

		#[block]
		{
			assert!(
				<Pallet<T> as OperationalRewardsPayer<T::AccountId, TreasuryBalanceOf<T>>>::claim_reward(
					&payout_account,
					reward_amount,
				)
				.is_ok()
			);
		}

		assert_eq!(T::Currency::balance(&payout_account), reward_amount);
		assert_eq!(T::Currency::balance(&reserves_account), minimum_balance);
		Ok(())
	}

	#[benchmark]
	fn provider_has_bond_participation() -> Result<(), BenchmarkError> {
		reset_benchmark_state::<T>();

		let lot_bonds = minimum_purchase_bonds::<T>();
		let _ = seed_accepted_vault_state::<T>(
			1,
			T::MaxTreasuryContributors::get(),
			lot_bonds,
			scaled_bonds(lot_bonds, T::MaxTreasuryContributors::get()),
			BENCHMARK_FRAME_ID.saturating_sub(1),
		)?;
		let account_id = account("missing-bond-holder", 0, 0);

		#[block]
		{
			assert!(!<Pallet<T> as TreasuryPoolProvider<T::AccountId>>::has_bond_participation(
				1,
				&account_id,
			));
		}

		Ok(())
	}

	#[benchmark]
	fn release_pending_bond_lots() -> Result<(), BenchmarkError> {
		reset_benchmark_state::<T>();
		seed_pending_bond_releases::<T>(BENCHMARK_FRAME_ID)?;

		#[block]
		{
			Pallet::<T>::release_pending_bond_lots(BENCHMARK_FRAME_ID);
		}

		let sample_account = account("pending-liquidation", 0, 0);
		assert!(
			PendingBondReleasesByFrame::<T>::get(BENCHMARK_FRAME_ID).is_empty(),
			"expected benchmark frame release queue to be emptied",
		);
		assert!(
			!BondLotById::<T>::contains_key(0),
			"expected sample pending bond lot to be removed after release",
		);
		assert_eq!(
			T::Currency::balance_on_hold(
				&HoldReason::ContributedToTreasury.into(),
				&sample_account
			),
			T::Balance::zero(),
			"expected sample pending bond lot hold to be released",
		);
		Ok(())
	}

	#[benchmark]
	fn distribute_bid_pool() -> Result<(), BenchmarkError> {
		reset_benchmark_state::<T>();
		seed_distribution_state::<T>(BENCHMARK_FRAME_ID)?;

		#[block]
		{
			Pallet::<T>::distribute_bid_pool(BENCHMARK_FRAME_ID);
		}

		assert!(
			CurrentFrameVaultCapital::<T>::get().is_none(),
			"expected current frame capital to be consumed during distribution",
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
		reset_benchmark_state::<T>();
		seed_lock_in_vault_capital_state::<T>(BENCHMARK_FRAME_ID)?;

		#[block]
		{
			Pallet::<T>::lock_in_vault_capital(BENCHMARK_FRAME_ID);
		}

		let current_frame_capital = CurrentFrameVaultCapital::<T>::get()
			.ok_or(BenchmarkError::Stop("missing current frame capital"))?;
		assert_eq!(current_frame_capital.frame_id, BENCHMARK_FRAME_ID);
		assert_eq!(
			current_frame_capital.vaults.len(),
			T::MaxVaultsPerPool::get() as usize,
			"expected benchmark to fill the current frame capital snapshot",
		);
		Ok(())
	}

	#[benchmark]
	fn on_frame_transition() -> Result<(), BenchmarkError> {
		reset_benchmark_state::<T>();
		seed_on_frame_transition_state::<T>(BENCHMARK_FRAME_ID)?;

		#[block]
		{
			Pallet::<T>::run_frame_transition(BENCHMARK_FRAME_ID);
		}

		assert!(
			PendingBondReleasesByFrame::<T>::get(BENCHMARK_FRAME_ID).is_empty(),
			"expected benchmark frame release queue to be emptied",
		);
		assert_eq!(
			CurrentFrameVaultCapital::<T>::get()
				.ok_or(BenchmarkError::Stop("missing current frame capital"))?
				.frame_id,
			BENCHMARK_FRAME_ID,
			"expected next frame capital to be locked in",
		);
		Ok(())
	}
}

fn reset_benchmark_state<T: Config>() {
	reset_benchmark_bitcoin_vault_provider_state();
	reset_benchmark_price_provider_state();
	set_benchmark_price_provider_state(BenchmarkPriceProviderState {
		btc_price_in_usd: Some(FixedU128::saturating_from_integer(100u128)),
		argon_price_in_usd: Some(FixedU128::one()),
		argon_target_price_in_usd: Some(FixedU128::one()),
		circulation: 1_000_000,
	});
}

fn seed_lock_in_vault_capital_state<T: Config>(frame_id: FrameId) -> Result<(), BenchmarkError>
where
	T::AccountId: Ord,
	T::Currency: Mutate<T::AccountId, Balance = T::Balance>,
{
	let lot_bonds = minimum_purchase_bonds::<T>();
	let security_bonds =
		scaled_bonds(lot_bonds, T::MaxTreasuryContributors::get().saturating_mul(2));

	let _ = seed_accepted_vault_state::<T>(
		T::MaxVaultsPerPool::get().saturating_add(1),
		T::MaxTreasuryContributors::get(),
		lot_bonds,
		security_bonds,
		frame_id.saturating_sub(1),
	)?;

	Ok(())
}

fn seed_distribution_state<T: Config>(frame_id: FrameId) -> Result<(), BenchmarkError>
where
	T::AccountId: Ord,
	T::Currency: Mutate<T::AccountId, Balance = T::Balance>,
{
	seed_lock_in_vault_capital_state::<T>(frame_id)?;
	Pallet::<T>::lock_in_vault_capital(frame_id);

	let bid_pool_account = T::MiningBidPoolAccount::get();
	T::Currency::mint_into(&bid_pool_account, balance::<T>(10_000_000_000_000))
		.map_err(|_| BenchmarkError::Stop("failed to fund bid pool"))?;

	Ok(())
}

fn seed_on_frame_transition_state<T: Config>(frame_id: FrameId) -> Result<(), BenchmarkError>
where
	T::AccountId: Ord,
	T::Currency: Mutate<T::AccountId, Balance = T::Balance>,
{
	seed_distribution_state::<T>(frame_id.saturating_sub(1))?;
	seed_pending_bond_releases::<T>(frame_id)?;

	Ok(())
}

fn seed_pending_bond_releases<T: Config>(frame_id: FrameId) -> Result<(), BenchmarkError>
where
	T::Currency: Mutate<T::AccountId, Balance = T::Balance>,
{
	let lot_bonds = minimum_purchase_bonds::<T>();
	let mut pending_releases = BoundedVec::default();

	for liquidation_index in 0..T::MaxPendingUnlocksPerFrame::get() {
		let owner: T::AccountId = account("pending-liquidation", liquidation_index, 0);
		let bond_lot_id = liquidation_index as BondLotId;
		let vault_id = 10_000u32.saturating_add(liquidation_index);
		insert_bond_lot::<T>(
			bond_lot_id,
			&owner,
			vault_id,
			lot_bonds,
			frame_id.saturating_sub(1),
			Some(frame_id),
			Some(BondReleaseReason::UserLiquidation),
			true,
		)?;
		pending_releases
			.try_push(bond_lot_id)
			.map_err(|_| BenchmarkError::Stop("failed to seed pending bond release"))?;
	}

	PendingBondReleasesByFrame::<T>::insert(frame_id, pending_releases);
	NextBondLotId::<T>::put(T::MaxPendingUnlocksPerFrame::get() as BondLotId);

	Ok(())
}

fn seed_accepted_vault_state<T: Config>(
	vault_count: u32,
	contributor_count: u32,
	lot_bonds: Bonds,
	security_bonds: Bonds,
	created_frame_id: FrameId,
) -> Result<BondLotId, BenchmarkError>
where
	T::AccountId: Ord,
{
	let mut benchmark_vault_state =
		BenchmarkBitcoinVaultProviderState::<T::AccountId, TreasuryBalanceOf<T>>::default();
	let mut next_bond_lot_id = 0u64;

	for vault_index in 0..vault_count {
		let vault_id = vault_index.saturating_add(1);
		let operator = benchmark_operator::<T>(vault_index);
		benchmark_vault_state
			.vaults
			.insert(vault_id, benchmark_vault::<T>(operator.clone(), security_bonds));

		let mut accepted_lots = BoundedVec::default();

		for contributor_index in 0..contributor_count {
			let owner = if contributor_index == 0 {
				operator.clone()
			} else {
				benchmark_bond_holder::<T>(vault_index, contributor_index)
			};

			insert_bond_lot::<T>(
				next_bond_lot_id,
				&owner,
				vault_id,
				lot_bonds,
				created_frame_id,
				None,
				None,
				false,
			)?;

			accepted_lots
				.try_push(BondLotSummary { bond_lot_id: next_bond_lot_id, bonds: lot_bonds })
				.map_err(|_| BenchmarkError::Stop("failed to seed accepted bond-lot list"))?;
			next_bond_lot_id = next_bond_lot_id.saturating_add(1);
		}

		BondLotsByVault::<T>::insert(vault_id, accepted_lots);
	}

	NextBondLotId::<T>::put(next_bond_lot_id);
	set_benchmark_bitcoin_vault_provider_state(benchmark_vault_state);

	Ok(next_bond_lot_id)
}

fn insert_bond_lot<T: Config>(
	bond_lot_id: BondLotId,
	owner: &T::AccountId,
	vault_id: VaultId,
	bonds: Bonds,
	created_frame_id: FrameId,
	release_frame_id: Option<FrameId>,
	release_reason: Option<BondReleaseReason>,
	hold_funds: bool,
) -> Result<(), BenchmarkError>
where
	T::Currency: Mutate<T::AccountId, Balance = T::Balance>,
{
	if hold_funds {
		let held_amount = bonds_to_balance::<T>(bonds);
		T::Currency::mint_into(owner, held_amount)
			.map_err(|_| BenchmarkError::Stop("failed to fund held bond lot"))?;
		Pallet::<T>::create_hold(owner, held_amount)
			.map_err(|_| BenchmarkError::Stop("failed to create bond lot hold"))?;
	}

	BondLotById::<T>::insert(
		bond_lot_id,
		BondLot {
			owner: owner.clone(),
			vault_id,
			bonds,
			created_frame_id,
			participated_frames: 0,
			last_frame_earnings_frame_id: None,
			last_frame_earnings: None,
			cumulative_earnings: T::Balance::zero(),
			release_frame_id,
			release_reason,
		},
	);
	BondLotIdsByAccount::<T>::insert(owner, bond_lot_id, ());

	Ok(())
}

fn benchmark_operator<T: Config>(vault_index: u32) -> T::AccountId {
	account("treasury-operator", vault_index, 0)
}

fn benchmark_bond_holder<T: Config>(vault_index: u32, contributor_index: u32) -> T::AccountId {
	account(
		"bond-holder",
		vault_index
			.saturating_mul(T::MaxTreasuryContributors::get())
			.saturating_add(contributor_index),
		0,
	)
}

fn benchmark_vault<T: Config>(
	operator_account_id: T::AccountId,
	securitized_bonds: Bonds,
) -> Vault<T::AccountId, T::Balance> {
	let securitized_satoshis = ((securitized_bonds as u128).saturating_mul(MICROGONS_PER_ARGON))
		.min(u64::MAX as u128) as Satoshis;

	Vault {
		operator_account_id,
		bitcoin_lock_delegate_account: None,
		name: None,
		last_name_change_tick: None,
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

fn scaled_bonds(base_bonds: Bonds, multiplier: u32) -> Bonds {
	((base_bonds as u128).saturating_mul(multiplier as u128)).min(Bonds::MAX as u128) as Bonds
}

fn minimum_purchase_bonds<T: Config>() -> Bonds {
	let minimum = T::MinimumArgonsPerContributor::get().into();
	let minimum_bonds = minimum.div_ceil(MICROGONS_PER_ARGON).max(1);
	minimum_bonds.min(Bonds::MAX as u128) as Bonds
}

fn bonds_to_balance<T: Config>(bonds: Bonds) -> TreasuryBalanceOf<T> {
	(bonds as u128).saturating_mul(MICROGONS_PER_ARGON).into()
}

fn balance<T: Config>(amount: u128) -> TreasuryBalanceOf<T> {
	amount.into()
}
