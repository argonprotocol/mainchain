#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_bitcoin::{derive_xpub, xpriv_from_seed};
use argon_primitives::{
	bitcoin::{get_rounded_up_bitcoin_day_height, BitcoinHeight, OpaqueBitcoinXpub},
	treasury::{PositionQuantity, TreasuryPositionProvider},
	vault::{
		BitcoinLockFundingUpdate, BitcoinResecuritization, BitcoinSecuritization,
		BitcoinSecuritizationBasis, LockExtension, ReserveSecuritizationRequest,
		SecuritizationScheduleEntry, VaultArgonotSecuritization, VaultError, VaultTerms,
		MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES,
	},
	MiningFrameTransitionProvider, MICROGONS_PER_ARGON,
};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet_prelude::benchmarking::{
	benchmark_bitcoin_vault_provider_state, reset_benchmark_bitcoin_vault_provider_state,
	reset_benchmark_treasury_positions, set_benchmark_bitcoin_vault_provider_state,
};
use pallet_treasury::{
	BondLotsByVault, CurrentFrameVaultCapital, FrameVaultCapital, VaultBondState,
	VaultSecuritizationPosition,
};

// Frame revenue and release completion costs are linear in the number of vaults.
// Measure up to 100 vaults to fit the slope; hooks charge the actual storage count.
const MAX_RELEASE_COMPLETIONS: u32 = 100;

type BalanceOf<T> = <T as Config>::Balance;
type CurrencyOf<T> = <T as Config>::Currency;
type OwnershipCurrencyOf<T> = <T as Config>::OwnershipCurrency;

fn benchmark_terms<T: Config>() -> VaultTerms<BalanceOf<T>> {
	VaultTerms {
		bitcoin_annual_percent_rate: FixedU128::from_rational(110u128, 100u128),
		bitcoin_base_fee: 1_000u128.into(),
	}
}

fn benchmark_xpub<T: Config>(seed_hint: u8) -> OpaqueBitcoinXpub {
	let mut seed = [0u8; 32];
	seed[0] = seed_hint;
	let xpriv = xpriv_from_seed(&seed, T::GetBitcoinNetwork::get())
		.expect("xpriv generation should work for benchmarks");
	let xpub =
		derive_xpub(&xpriv, "m/84'/0'/0'").expect("xpub derivation should work for benchmarks");
	OpaqueBitcoinXpub::from(xpub)
}

fn benchmark_vault_config<T: Config>(
	seed_hint: u8,
	securitization: u128,
) -> VaultConfig<T::AccountId, BalanceOf<T>> {
	VaultConfig {
		terms: benchmark_terms::<T>(),
		delegate_account_id: None,
		securitization: securitization.into(),
		bitcoin_xpubkey: benchmark_xpub::<T>(seed_hint),
		securitization_ratio: FixedU128::one(),
	}
}

fn create_vault<T: Config>(
	operator: &T::AccountId,
	seed_hint: u8,
	securitization: u128,
) -> Result<VaultId, BenchmarkError>
where
	CurrencyOf<T>: frame_support::traits::fungible::Mutate<T::AccountId, Balance = BalanceOf<T>>,
{
	let funding = securitization.saturating_add(1_000_000);
	let _ = CurrencyOf::<T>::mint_into(operator, funding.into());
	Pallet::<T>::create(
		RawOrigin::Signed(operator.clone()).into(),
		benchmark_vault_config::<T>(seed_hint, securitization),
	)
	.map_err(|_| BenchmarkError::Stop("vault create failed"))?;
	VaultIdByOperator::<T>::get(operator).ok_or(BenchmarkError::Stop("vault id missing"))
}

fn seed_release_schedule_for_benchmark<T: Config>(
	vault_id: VaultId,
	release_at_or_before: BitcoinHeight,
	entries: u32,
	entry_amount: BalanceOf<T>,
	locked: BalanceOf<T>,
	target: BalanceOf<T>,
) -> Result<(), BenchmarkError> {
	VaultsById::<T>::try_mutate(vault_id, |vault| {
		let vault = vault
			.as_mut()
			.ok_or(BenchmarkError::Stop("vault missing while seeding schedule"))?;
		vault.securitization_locked = locked;
		vault.securitization_pending_activation = 0u32.into();
		vault.securitization_target = target;
		vault
			.scheduled_release(release_at_or_before)
			.map_err(|_| BenchmarkError::Stop("unable to seed release schedule"))?
			.relockable_commitments = entry_amount;
		for i in 1..entries {
			let h = release_at_or_before.saturating_add(u64::from(i) * 144);
			vault
				.scheduled_release(h)
				.map_err(|_| BenchmarkError::Stop("unable to seed release schedule"))?
				.relockable_commitments = entry_amount;
		}
		assert_eq!(vault.securitization_release_schedule.len(), entries as usize);
		Ok(())
	})
}

/// Reuse the full 5,000-ARGON requirement across distinct daily buckets. Withdrawals keep
/// the earlier rows occupied, so a full schedule only frees its last row during admission.
fn seed_relockable_admission<T: Config>(
	vault_id: VaultId,
	entries: u32,
) -> Result<(), BenchmarkError> {
	VaultsById::<T>::try_mutate(vault_id, |vault| {
		let vault = vault.as_mut().ok_or(BenchmarkError::Stop("admission vault missing"))?;
		vault.reserved_securitization_space = (100 * MICROGONS_PER_ARGON).into();
		let collateral = 5_000 * MICROGONS_PER_ARGON;
		let mut remaining = collateral;
		for index in 0..entries {
			let amount =
				if index + 1 == entries { remaining } else { collateral / u128::from(entries) };
			remaining -= amount;
			let entry = vault
				.scheduled_release(u64::from(index + 1) * 144)
				.map_err(|_| BenchmarkError::Stop("admission schedule overflow"))?;
			entry.relockable_commitments = amount.into();
			if index + 1 < entries {
				entry.argon_withdrawals = MICROGONS_PER_ARGON.into();
			}
		}
		assert_eq!(vault.securitization_release_schedule.len(), entries as usize);
		Ok(())
	})
}

/// Exercise Treasury's populated callback and the complete bounded frame snapshot. The
/// position provider is isolated during measurement; its cost is added by the weight adapter.
fn seed_treasury_bonds_for_benchmark<T>(vault_id: VaultId) -> Result<(), BenchmarkError>
where
	T: Config + pallet_treasury::Config<Balance = BalanceOf<T>>,
	BalanceOf<T>: Into<u128>,
{
	let mut vault =
		VaultsById::<T>::get(vault_id).ok_or(BenchmarkError::Stop("callback vault missing"))?;
	if !CurrentFrameVaultCapital::<T>::exists() {
		reset_benchmark_treasury_positions();
		reset_benchmark_bitcoin_vault_provider_state();
	}
	let flexible_bonds = (vault.securitization.into() / MICROGONS_PER_ARGON)
		.min(pallet_treasury::Bonds::MAX.into()) as pallet_treasury::Bonds;
	BondLotsByVault::<T>::insert(vault_id, VaultBondState { flexible_bonds, ..Default::default() });
	<T as pallet_treasury::Config>::PositionProvider::account_quantity_updated(
		&vault.operator_account_id,
		PositionQuantity::Bonds,
		0u32,
		flexible_bonds,
	)
	.map_err(|_| BenchmarkError::Stop("callback position setup failed"))?;
	let mut benchmark_vaults =
		benchmark_bitcoin_vault_provider_state::<T::AccountId, BalanceOf<T>>();
	// This stub only supplies the operator. The real query's full schedule is measured
	// separately, so avoid decoding it repeatedly inside the callback benchmark.
	vault.securitization_release_schedule.clear();
	benchmark_vaults.vaults.insert(vault_id, vault);
	set_benchmark_bitcoin_vault_provider_state(benchmark_vaults);

	if CurrentFrameVaultCapital::<T>::exists() {
		return Ok(());
	}
	let amount: BalanceOf<T> = (5_000 * MICROGONS_PER_ARGON).into();
	let mut positions = BoundedBTreeMap::new();
	for index in 0..<T as pallet_treasury::Config>::MaxVaultsPerPool::get() {
		positions
			.try_insert(
				index + 1,
				VaultSecuritizationPosition::<T> {
					operator_account_id: account("callback-frame-operator", index, 0),
					securitization: amount,
					activated_securitization: amount,
					bitcoin_locked_microgons: amount,
					argonot_securitization_in_microgons: amount,
					active_bond_microgons: amount,
					upstream_participation: FixedU128::one(),
				},
			)
			.map_err(|_| BenchmarkError::Stop("callback frame snapshot overflow"))?;
	}
	CurrentFrameVaultCapital::<T>::put(FrameVaultCapital::<T> {
		frame_id:
			<T as pallet_treasury::Config>::MiningFrameTransitionProvider::get_current_frame_id(),
		total_active_bonds: 5_000 *
			u128::from(<T as pallet_treasury::Config>::MaxVaultsPerPool::get()),
		target_securitization: amount,
		total_securitization: amount,
		vault_securitization_positions: positions,
	});
	Ok(())
}

#[benchmarks(
	where
		CurrencyOf<T>: frame_support::traits::fungible::Mutate<T::AccountId, Balance = BalanceOf<T>>,
		T: pallet_bitcoin_utxos::Config + pallet_treasury::Config<Balance = BalanceOf<T>>,
		BalanceOf<T>: Into<u128>,
)]
mod benchmarks {
	use super::*;
	use argon_primitives::{
		bitcoin::{BitcoinBlock, BitcoinHeight, BitcoinXPub, H256Le},
		vault::{BitcoinVaultProvider, TreasuryVaultProvider, VaultTreasuryFrameEarnings},
		OnNewSlot,
	};
	use frame_support::traits::{fungible::InspectHold, Get, Hooks};

	#[benchmark]
	fn provider_get_vault_operator() -> Result<(), BenchmarkError> {
		let operator: T::AccountId = account("vault-operator-query", 0, 0);
		let vault_id = create_vault::<T>(&operator, 8, 5_000 * MICROGONS_PER_ARGON)?;
		seed_release_schedule_for_benchmark::<T>(
			vault_id,
			144,
			MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES,
			1u128.into(),
			0u128.into(),
			(5_000 * MICROGONS_PER_ARGON).into(),
		)?;
		#[block]
		{
			assert_eq!(
				<Pallet<T> as TreasuryVaultProvider>::get_vault_operator(vault_id),
				Some(operator)
			);
		}
		Ok(())
	}

	#[benchmark]
	fn provider_get_participation_capacity() -> Result<(), BenchmarkError> {
		let operator: T::AccountId = account("upstream-capacity", 0, 0);
		let vault_id = create_vault::<T>(&operator, 8, 8_650 * MICROGONS_PER_ARGON)?;
		pallet_bitcoin_utxos::ConfirmedBitcoinBlockTip::<T>::put(BitcoinBlock::new(
			145,
			H256Le([1; 32]),
		));
		pallet_bitcoin_utxos::PreviousBitcoinBlockTip::<T>::put(BitcoinBlock::new(
			144,
			H256Le([2; 32]),
		));
		let first_height = <T as Config>::BitcoinBlockHeightChange::get().1 / 144 * 144;
		VaultsById::<T>::mutate(vault_id, |vault| {
			let vault = vault.as_mut().unwrap();
			vault.opened_tick = 0;
			vault.securitization_locked = (3_650 * MICROGONS_PER_ARGON).into();
			for index in 0..MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES {
				// The last row can be fully reused; the other daily rows retain withdrawals.
				let entry = if index + 1 == MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES {
					SecuritizationScheduleEntry {
						relockable_commitments: (10 * MICROGONS_PER_ARGON).into(),
						..Default::default()
					}
				} else {
					SecuritizationScheduleEntry {
						locked_commitments: (10 * MICROGONS_PER_ARGON).into(),
						argon_withdrawals: MICROGONS_PER_ARGON.into(),
						..Default::default()
					}
				};
				vault
					.securitization_release_schedule
					.try_insert(first_height + u64::from(index) * 144, entry)
					.unwrap();
			}
		});
		#[block]
		{
			let capacity =
				<Pallet<T> as TreasuryVaultProvider>::get_participation_capacity(vault_id)
					.expect("upstream is open");
			assert_eq!(
				capacity.available_securitization_space,
				(5_000 * MICROGONS_PER_ARGON).into()
			);
			assert_eq!(capacity.regular_bond_capacity, (8_285 * MICROGONS_PER_ARGON).into());
		}
		Ok(())
	}

	#[benchmark]
	fn create() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("create_caller", 0, 0);
		let securitization: u128 = 100_000;
		let _ =
			CurrencyOf::<T>::mint_into(&caller, securitization.saturating_add(1_000_000).into());
		let vault_config = benchmark_vault_config::<T>(1, securitization);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_config);

		assert!(VaultIdByOperator::<T>::contains_key(&caller));
		Ok(())
	}

	#[benchmark]
	fn modify_funding() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("funding_caller", 0, 0);
		let capital = 5_000 * MICROGONS_PER_ARGON;
		let vault_id = create_vault::<T>(&caller, 2, capital)?;
		let notice_height = get_rounded_up_bitcoin_day_height(
			<T as Config>::BitcoinBlockHeightChange::get()
				.1
				.saturating_add(T::SecuritizationExitNoticeBlocks::get()),
		);
		seed_release_schedule_for_benchmark::<T>(
			vault_id,
			notice_height
				.saturating_sub(u64::from(MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES - 1) * 144),
			MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES,
			100u128.into(),
			(2_000 * MICROGONS_PER_ARGON).into(),
			capital.into(),
		)?;
		seed_treasury_bonds_for_benchmark::<T>(vault_id)?;
		let securitization: BalanceOf<T> = (1_000 * MICROGONS_PER_ARGON).into();
		let ratio = FixedU128::one();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_id, securitization, ratio);

		let vault = VaultsById::<T>::get(vault_id)
			.ok_or(BenchmarkError::Stop("vault missing after modify"))?;
		assert_eq!(vault.securitization_target, securitization);
		assert_eq!(vault.securitization_ratio, ratio);
		let bonds = BondLotsByVault::<T>::get(vault_id);
		assert!(bonds.locked_frame_terms.is_some());
		assert!(bonds.displaced_flexible_bonds > 0);
		Ok(())
	}

	#[benchmark]
	fn modify_terms() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("terms_caller", 0, 0);
		let vault_id = create_vault::<T>(&caller, 3, 100_000)?;
		let terms_change_tick = Pallet::<T>::get_terms_active_tick();
		let max_pending = T::MaxPendingTermModificationsPerTick::get().saturating_sub(1);
		PendingTermsModificationsByTick::<T>::mutate(terms_change_tick, |pending| {
			for i in 0..max_pending {
				let dummy_vault_id = 100_000u32.saturating_add(i);
				if dummy_vault_id == vault_id {
					continue;
				}
				let _ = pending.try_push(dummy_vault_id);
			}
		});
		let new_terms = VaultTerms {
			bitcoin_annual_percent_rate: FixedU128::from_rational(120u128, 100u128),
			bitcoin_base_fee: 2_000u128.into(),
		};

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_id, new_terms.clone());

		let vault = VaultsById::<T>::get(vault_id)
			.ok_or(BenchmarkError::Stop("vault missing after terms"))?;
		assert!(vault.pending_terms.is_some());
		assert_eq!(vault.pending_terms.map(|(_, terms)| terms), Some(new_terms));
		Ok(())
	}

	#[benchmark]
	fn close() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("close_caller", 0, 0);
		let capital = 5_000 * MICROGONS_PER_ARGON;
		let vault_id = create_vault::<T>(&caller, 4, capital)?;
		let notice_height = get_rounded_up_bitcoin_day_height(
			<T as Config>::BitcoinBlockHeightChange::get()
				.1
				.saturating_add(T::SecuritizationExitNoticeBlocks::get()),
		);
		seed_release_schedule_for_benchmark::<T>(
			vault_id,
			notice_height
				.saturating_sub(u64::from(MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES - 1) * 144),
			MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES,
			100u128.into(),
			(2_000 * MICROGONS_PER_ARGON).into(),
			capital.into(),
		)?;
		seed_treasury_bonds_for_benchmark::<T>(vault_id)?;

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_id);

		let vault = VaultsById::<T>::get(vault_id)
			.ok_or(BenchmarkError::Stop("vault missing after close"))?;
		assert!(vault.is_closed);
		assert_eq!(vault.securitization_target, BalanceOf::<T>::zero());
		let bonds = BondLotsByVault::<T>::get(vault_id);
		assert!(bonds.locked_frame_terms.is_some());
		assert_eq!(bonds.displaced_flexible_bonds, bonds.flexible_bonds);
		Ok(())
	}

	#[benchmark]
	fn replace_bitcoin_xpub() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("xpub_caller", 0, 0);
		let vault_id = create_vault::<T>(&caller, 5, 100_000)?;
		let new_xpub = benchmark_xpub::<T>(6);
		let expected_xpub: BitcoinXPub = new_xpub
			.try_into()
			.map_err(|_| BenchmarkError::Stop("benchmark xpub decode failed"))?;

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_id, new_xpub);

		let stored =
			VaultXPubById::<T>::get(vault_id).ok_or(BenchmarkError::Stop("vault xpub missing"))?;
		assert_eq!(stored.0, expected_xpub);
		Ok(())
	}

	#[benchmark]
	fn set_delegate_account() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("set_delegate_caller", 0, 0);
		let delegate: T::AccountId = account("set_delegate_target", 0, 0);
		let vault_id = create_vault::<T>(&caller, 6, 100_000)?;

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), Some(delegate.clone()));

		let vault = VaultsById::<T>::get(vault_id).ok_or(BenchmarkError::Stop("vault missing"))?;
		assert_eq!(vault.delegate_account_id, Some(delegate));
		Ok(())
	}

	#[benchmark]
	fn set_reserved_securitization_space() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("set_capacity_reserved_caller", 0, 0);
		let vault_id = create_vault::<T>(&caller, 7, 100_000)?;
		VaultsById::<T>::mutate(vault_id, |vault| {
			let vault = vault.as_mut().expect("benchmark vault");
			vault.securitization_locked = 100_000u32.into();
			vault.flexible_securitization_locked = 100_000u32.into();
		});

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), 100_000u32.into());

		let vault = VaultsById::<T>::get(vault_id).ok_or(BenchmarkError::Stop("vault missing"))?;
		assert_eq!(vault.reserved_securitization_space, 100_000u32.into());
		Ok(())
	}

	#[benchmark]
	fn set_argonot_securitization() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("set_argonot_securitization_caller", 0, 0);
		let vault_id = create_vault::<T>(&caller, 8, 100_000)?;
		let amount: BalanceOf<T> = 25_000u128.into();
		let _ = OwnershipCurrencyOf::<T>::mint_into(&caller, 1_000_000u128.into());

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), amount);

		assert_eq!(
			OwnershipCurrencyOf::<T>::balance_on_hold(&HoldReason::EnterVault.into(), &caller),
			amount,
		);
		assert_eq!(<Pallet<T> as BitcoinVaultProvider>::get_held_argonots(&caller), Some(amount),);
		assert_eq!(<Pallet<T> as BitcoinVaultProvider>::get_vault_id(&caller), Some(vault_id),);
		Ok(())
	}

	#[benchmark]
	fn collect() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("collect_caller", 0, 0);
		let vault_id = create_vault::<T>(&caller, 8, 100_000)?;
		let source: T::AccountId = account("source", 0, 0);
		let earnings_for_vault: BalanceOf<T> = 50_000u128.into();
		let _ = CurrencyOf::<T>::mint_into(&source, 1_000_000_000u128.into());

		for i in 0..12u32 {
			<Pallet<T> as TreasuryVaultProvider>::record_vault_frame_earnings(
				&source,
				VaultTreasuryFrameEarnings {
					vault_id,
					vault_operator_account_id: caller.clone(),
					frame_id: T::CurrentFrameId::get().saturating_add(i.into()),
					earnings: earnings_for_vault,
					capital_contributed: earnings_for_vault,
					earnings_for_vault,
					capital_contributed_by_vault: earnings_for_vault,
				},
			)?;
		}
		assert_eq!(RevenuePerFrameByVault::<T>::get(vault_id).len(), 12);
		assert!(
			RevenuePerFrameByVault::<T>::get(vault_id)
				.iter()
				.any(|entry| entry.uncollected_revenue > BalanceOf::<T>::zero()),
			"expected uncollected revenue before collect"
		);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_id);

		assert_eq!(LastCollectFrameByVaultId::<T>::get(vault_id), Some(T::CurrentFrameId::get()));
		assert_eq!(
			CurrencyOf::<T>::balance_on_hold(&HoldReason::PendingCollect.into(), &caller),
			BalanceOf::<T>::zero()
		);
		assert!(
			RevenuePerFrameByVault::<T>::get(vault_id)
				.iter()
				.all(|entry| entry.uncollected_revenue == BalanceOf::<T>::zero()),
			"expected all frame revenue to be collected"
		);
		Ok(())
	}

	#[benchmark]
	fn provider_get_registration_vault_data() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("provider_vault_caller", 0, 0);
		let vault_id = create_vault::<T>(&caller, 8, 100_000)?;

		#[block]
		{
			let registration =
				<Pallet<T> as BitcoinVaultProvider>::get_registration_vault_data(&caller);
			assert_eq!(
				registration.clone().map(|entry| entry.vault_id),
				Some(vault_id),
				"expected provider lookup to resolve the created vault"
			);
			assert_eq!(
				registration.map(|entry| entry.securitization),
				Some(100_000u128.into()),
				"expected provider lookup to return the vault securitization"
			);
		}

		Ok(())
	}

	#[benchmark]
	fn provider_get_committed_securitization() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("provider_committed_securitization", 0, 0);
		let vault_id = create_vault::<T>(&caller, 8, 100_000)?;
		let vault = VaultsById::<T>::get(vault_id).ok_or(BenchmarkError::Stop("vault missing"))?;
		let expected =
			vault.get_activated_securitization().saturating_add(vault.get_relock_capacity());

		#[block]
		{
			assert_eq!(
				<Pallet<T> as BitcoinVaultProvider>::get_committed_securitization(&caller, 10),
				Some(expected),
			);
		}

		Ok(())
	}

	#[benchmark]
	fn provider_get_held_argonots() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("provider_committed_argonots", 0, 0);
		create_vault::<T>(&caller, 9, 100_000)?;
		let amount: BalanceOf<T> = 40_000u128.into();
		let _ = OwnershipCurrencyOf::<T>::mint_into(&caller, 1_000_000u128.into());
		Pallet::<T>::set_argonot_securitization(RawOrigin::Signed(caller.clone()).into(), amount)
			.map_err(|_| BenchmarkError::Stop("failed to set committed argonots"))?;

		#[block]
		{
			assert_eq!(
				<Pallet<T> as BitcoinVaultProvider>::get_held_argonots(&caller),
				Some(amount),
			);
		}

		Ok(())
	}

	#[benchmark]
	fn provider_get_top_vaults_by_securitization(
		v: Linear<1, MAX_RELEASE_COMPLETIONS>,
	) -> Result<(), BenchmarkError> {
		for index in 0..v {
			let operator: T::AccountId = account("ranked_vault_operator", index, 0);
			create_vault::<T>(&operator, index as u8, 100_000)?;
		}

		#[block]
		{
			let (vaults, _) =
				<Pallet<T> as TreasuryVaultProvider>::get_top_vaults_by_securitization(v);
			assert_eq!(vaults.len(), v as usize);
		}

		Ok(())
	}

	#[benchmark]
	fn provider_commit_securitization_for_rewards() -> Result<(), BenchmarkError> {
		let operator: T::AccountId = account("reward_operator", 0, 0);
		let vault_id = create_vault::<T>(&operator, 8, 100_000)?;
		let amount: BalanceOf<T> = 25_000u128.into();
		ArgonotSecuritizationByVaultId::<T>::insert(
			vault_id,
			VaultArgonotSecuritization {
				held_micronots: amount,
				committed_micronots: BalanceOf::<T>::zero(),
				encumbered_micronots: BalanceOf::<T>::zero(),
			},
		);

		#[block]
		{
			<Pallet<T> as TreasuryVaultProvider>::commit_securitization_for_rewards(
				vault_id, amount,
			);
		}

		assert_eq!(
			ArgonotSecuritizationByVaultId::<T>::get(vault_id)
				.ok_or(BenchmarkError::Stop("Argonot securitization missing"))?
				.committed_micronots,
			amount,
		);
		assert_eq!(
			VaultsById::<T>::get(vault_id)
				.ok_or(BenchmarkError::Stop("vault missing"))?
				.committed_microgons,
			100_000u128.into(),
		);
		Ok(())
	}

	#[benchmark]
	fn provider_record_vault_frame_earnings() -> Result<(), BenchmarkError> {
		let operator: T::AccountId = account("reward_vault_operator", 0, 0);
		let vault_id = create_vault::<T>(&operator, 8, 100_000)?;
		let source: T::AccountId = account("reward_source", 0, 0);
		let earnings: BalanceOf<T> = 50_000u128.into();
		CurrencyOf::<T>::mint_into(&source, 1_000_000u128.into())
			.map_err(|_| BenchmarkError::Stop("failed to fund reward source"))?;
		let frame_id = T::CurrentFrameId::get();

		#[block]
		{
			<Pallet<T> as TreasuryVaultProvider>::record_vault_frame_earnings(
				&source,
				VaultTreasuryFrameEarnings {
					vault_id,
					vault_operator_account_id: operator,
					frame_id,
					earnings_for_vault: earnings,
					capital_contributed: BalanceOf::<T>::zero(),
					capital_contributed_by_vault: BalanceOf::<T>::zero(),
					earnings,
				},
			)
			.expect("benchmark reward should be recorded");
		}

		assert_eq!(RevenuePerFrameByVault::<T>::get(vault_id).len(), 1);
		Ok(())
	}

	#[benchmark]
	fn provider_set_bitcoin_lock_flexible() -> Result<(), BenchmarkError> {
		let operator: T::AccountId = account("provider_flexible_lock", 0, 0);
		let vault_id = create_vault::<T>(&operator, 10, 100_000)?;
		let collateral_required: BalanceOf<T> = 10_000u128.into();
		let satoshis = 10_000;
		VaultsById::<T>::mutate(vault_id, |vault| {
			let vault = vault.as_mut().expect("benchmark vault should exist");
			vault.securitization_locked = collateral_required;
			vault.securitized_satoshis = satoshis;
			vault.ratio_adjusted_satoshis = satoshis;
		});
		let securitization = BitcoinSecuritization {
			basis: BitcoinSecuritizationBasis {
				satoshis,
				microgons_at_target_per_btc: 100_000_000u128.into(),
			},
			securitization_coverage_microgons: collateral_required,
			securitization_ratio: FixedU128::one(),
		};

		#[block]
		{
			<Pallet<T> as BitcoinVaultProvider>::set_bitcoin_lock_flexible(
				vault_id,
				&securitization,
				satoshis,
				true,
			)
			.map_err(|_| BenchmarkError::Stop("failed to mark Bitcoin lock flexible"))?;
		}

		let vault = VaultsById::<T>::get(vault_id)
			.ok_or(BenchmarkError::Stop("vault missing after flexible lock update"))?;
		assert_eq!(vault.flexible_securitization_locked, collateral_required);
		assert_eq!(vault.flexible_ratio_adjusted_satoshis, satoshis);
		Ok(())
	}

	#[benchmark]
	fn provider_reserve_securitization(
		e: Linear<1, MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES>,
	) -> Result<(), BenchmarkError> {
		let operator: T::AccountId = account("admission-operator", 0, 0);
		let locker: T::AccountId = account("admission-locker", 0, 0);
		let vault_id = create_vault::<T>(&operator, 12, 10_000 * MICROGONS_PER_ARGON)?;
		CurrencyOf::<T>::mint_into(&locker, (10_000 * MICROGONS_PER_ARGON).into())?;
		seed_relockable_admission::<T>(vault_id, e)?;
		let securitization = BitcoinSecuritization {
			basis: BitcoinSecuritizationBasis {
				satoshis: 5_000_000,
				microgons_at_target_per_btc: (100_000 * MICROGONS_PER_ARGON).into(),
			},
			securitization_coverage_microgons: (5_000 * MICROGONS_PER_ARGON).into(),
			securitization_ratio: FixedU128::one(),
		};
		let request = ReserveSecuritizationRequest {
			lock_expiration: u64::from(e + 1) * 144,
			fee_discount: (250 * MICROGONS_PER_ARGON).into(),
			securitization_space_to_unreserve: (100 * MICROGONS_PER_ARGON).into(),
		};

		#[block]
		{
			<Pallet<T> as BitcoinVaultProvider>::reserve_securitization(
				vault_id,
				&locker,
				&securitization,
				request,
			)
			.map_err(|_| BenchmarkError::Stop("public admission failed"))?;
		}

		let vault = VaultsById::<T>::get(vault_id).expect("admission vault exists");
		assert_eq!(vault.securitization_pending_activation, securitization.collateral_required());
		assert_eq!(vault.reserved_securitization_space, BalanceOf::<T>::zero());
		assert_eq!(vault.get_relock_capacity(), BalanceOf::<T>::zero());
		assert!(
			CurrencyOf::<T>::balance_on_hold(&HoldReason::PendingCollect.into(), &operator) >
				BalanceOf::<T>::zero()
		);
		Ok(())
	}

	#[benchmark]
	fn provider_resecuritize_unfunded(
		e: Linear<1, MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES>,
	) -> Result<(), BenchmarkError> {
		let operator: T::AccountId = account("unfunded-operator", 0, 0);
		let locker: T::AccountId = account("unfunded-locker", 0, 0);
		let vault_id = create_vault::<T>(&operator, 13, 10_000 * MICROGONS_PER_ARGON)?;
		CurrencyOf::<T>::mint_into(&locker, (10_000 * MICROGONS_PER_ARGON).into())?;
		let current = BitcoinSecuritization {
			basis: BitcoinSecuritizationBasis {
				satoshis: 2_500_000,
				microgons_at_target_per_btc: (100_000 * MICROGONS_PER_ARGON).into(),
			},
			securitization_coverage_microgons: (2_500 * MICROGONS_PER_ARGON).into(),
			securitization_ratio: FixedU128::one(),
		};
		let replacement = BitcoinSecuritization {
			basis: BitcoinSecuritizationBasis { satoshis: 5_000_000, ..current.basis },
			securitization_coverage_microgons: (5_000 * MICROGONS_PER_ARGON).into(),
			..current
		};
		let mut extension = LockExtension::new(u64::from(e) * 144);
		VaultsById::<T>::try_mutate(vault_id, |vault| {
			vault
				.as_mut()
				.expect("unfunded vault exists")
				.reserve_securitization(&current, true, extension.lock_expiration)
				.map_err(|_| BenchmarkError::Stop("unfunded reservation setup failed"))
		})?;
		seed_relockable_admission::<T>(vault_id, e)?;

		#[block]
		{
			<Pallet<T> as BitcoinVaultProvider>::resecuritize(
				vault_id,
				&locker,
				BitcoinResecuritization {
					current: &current,
					replacement: &replacement,
					funded_satoshis: 0,
					remaining_term: FixedU128::one(),
					lock_extension: &mut extension,
					is_flexible: false,
					fee_discount: (250 * MICROGONS_PER_ARGON).into(),
					securitization_space_to_unreserve: (100 * MICROGONS_PER_ARGON).into(),
				},
			)
			.map_err(|_| BenchmarkError::Stop("unfunded replacement failed"))?;
		}

		let vault = VaultsById::<T>::get(vault_id).expect("unfunded vault exists");
		assert_eq!(vault.securitization_pending_activation, replacement.collateral_required());
		assert_eq!(vault.reserved_securitization_space, BalanceOf::<T>::zero());
		assert_eq!(vault.get_relock_capacity(), BalanceOf::<T>::zero());
		Ok(())
	}

	#[benchmark]
	fn provider_resecuritize(
		e: Linear<1, MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES>,
	) -> Result<(), BenchmarkError> {
		let operator: T::AccountId = account("provider_resecuritize_operator", 0, 0);
		let locker: T::AccountId = account("provider_resecuritize_locker", 0, 0);
		let collateral = 2_500 * MICROGONS_PER_ARGON;
		let current = BitcoinSecuritization {
			basis: BitcoinSecuritizationBasis {
				satoshis: 2_500_000,
				microgons_at_target_per_btc: (100_000 * MICROGONS_PER_ARGON).into(),
			},
			securitization_coverage_microgons: collateral.into(),
			securitization_ratio: FixedU128::one(),
		};
		let replacement = BitcoinSecuritization {
			basis: BitcoinSecuritizationBasis { satoshis: 5_000_000, ..current.basis },
			securitization_coverage_microgons: (5_000 * MICROGONS_PER_ARGON).into(),
			..current
		};
		let vault_id = create_vault::<T>(&operator, 11, 5_000 * MICROGONS_PER_ARGON)?;
		CurrencyOf::<T>::mint_into(&locker, (10_000 * MICROGONS_PER_ARGON).into())?;
		let funded_satoshis = replacement.basis.satoshis;
		let original_extension = LockExtension::new(144);
		let mut lock_extension = LockExtension::new(144);
		let mut remaining = collateral;
		for index in 0..e {
			let amount = if index + 1 == e { remaining } else { collateral / u128::from(e) };
			remaining -= amount;
			let height = u64::from(index + 2) * 144;
			lock_extension
				.extended_expiration_funds
				.try_insert(height, amount.into())
				.map_err(|_| BenchmarkError::Stop("inherited maturity setup failed"))?;
		}
		VaultsById::<T>::try_mutate(vault_id, |vault| {
			let vault = vault.as_mut().expect("funded replacement vault exists");
			vault.reserve_securitization(&current, true, original_extension.lock_expiration)?;
			vault.record_bitcoin_lock_funding(BitcoinLockFundingUpdate {
				funded_satoshis,
				securitized_satoshis: current.securitized_satoshis(funded_satoshis),
				collateral_required: current.collateral_between(0, funded_satoshis),
				eligible_satoshis: current
					.eligible_satoshis(current.securitized_satoshis(funded_satoshis)),
				is_flexible: false,
			})?;

			// Match the funded Lock's inherited commitments. The other half of the vault's
			// collateral is relockable at those same maturities, forcing reuse of every row.
			vault.update_locked_commitments(
				&original_extension,
				0u128.into(),
				collateral.into(),
				false,
			)?;
			vault.update_locked_commitments(
				&lock_extension,
				0u128.into(),
				collateral.into(),
				true,
			)?;
			for (height, amount) in &lock_extension.extended_expiration_funds {
				let entry = vault.scheduled_release(*height)?;
				entry.relockable_commitments = *amount;
				entry.argon_withdrawals = MICROGONS_PER_ARGON.into();
			}
			vault.reserved_securitization_space = (100 * MICROGONS_PER_ARGON).into();
			assert_eq!(vault.securitization_release_schedule.len(), e as usize);
			Ok::<_, VaultError>(())
		})
		.map_err(|_| BenchmarkError::Stop("funded replacement setup failed"))?;

		#[block]
		{
			<Pallet<T> as BitcoinVaultProvider>::resecuritize(
				vault_id,
				&locker,
				BitcoinResecuritization {
					current: &current,
					replacement: &replacement,
					funded_satoshis,
					remaining_term: FixedU128::one(),
					lock_extension: &mut lock_extension,
					is_flexible: false,
					fee_discount: (250 * MICROGONS_PER_ARGON).into(),
					securitization_space_to_unreserve: (100 * MICROGONS_PER_ARGON).into(),
				},
			)
			.map_err(|_| BenchmarkError::Stop("failed to resecuritize Bitcoin lock"))?;
		}

		let vault = VaultsById::<T>::get(vault_id)
			.ok_or(BenchmarkError::Stop("vault missing after resecuritization"))?;
		assert_eq!(vault.securitized_satoshis, replacement.securitized_satoshis(funded_satoshis));
		assert_eq!(vault.ratio_adjusted_satoshis, replacement.eligible_satoshis(funded_satoshis));
		assert_eq!(vault.securitization_locked, replacement.collateral_required());
		assert_eq!(lock_extension.len(), e as usize);
		assert_eq!(VaultFundsReleasingByHeight::<T>::iter().count(), e as usize);
		assert_eq!(vault.reserved_securitization_space, BalanceOf::<T>::zero());
		assert_eq!(vault.get_relock_capacity(), BalanceOf::<T>::zero());
		Ok(())
	}

	#[benchmark]
	fn provider_burn(
		e: Linear<1, MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES>,
	) -> Result<(), BenchmarkError> {
		let operator: T::AccountId = account("provider_burn_operator", 0, 0);
		let collateral = 2_500 * MICROGONS_PER_ARGON;
		let securitization = BitcoinSecuritization {
			basis: BitcoinSecuritizationBasis {
				satoshis: 100_000_000,
				microgons_at_target_per_btc: collateral.into(),
			},
			securitization_coverage_microgons: collateral.into(),
			securitization_ratio: FixedU128::one(),
		};
		let vault_id = create_vault::<T>(&operator, 13, 5_000 * MICROGONS_PER_ARGON)?;
		let original_extension = LockExtension::new(10_000);
		let mut lock_extension = LockExtension::new(10_000);
		let mut remaining = collateral;
		for index in 0..e {
			let amount = if index == e - 1 { remaining } else { collateral / u128::from(e) };
			remaining -= amount;
			let height = original_extension.expiration_day() + 144 * u64::from(index + 1);
			lock_extension
				.extended_expiration_funds
				.try_insert(height, amount.into())
				.map_err(|_| BenchmarkError::Stop("unable to seed Lock extension"))?;
		}
		VaultsById::<T>::try_mutate(vault_id, |vault| {
			let vault =
				vault.as_mut().ok_or(BenchmarkError::Stop("benchmark vault should exist"))?;
			vault
				.reserve_securitization(&securitization, true, original_extension.lock_expiration)
				.map_err(|_| BenchmarkError::Stop("failed to reserve securitization"))?;
			// The vault's commitment schedule must describe the same inherited maturities as
			// the Lock being burned. Replace the original commitment before adding the extensions.
			vault
				.update_locked_commitments(
					&original_extension,
					0u128.into(),
					collateral.into(),
					false,
				)
				.map_err(|_| BenchmarkError::Stop("failed to remove original commitment"))?;
			vault
				.update_locked_commitments(&lock_extension, 0u128.into(), collateral.into(), true)
				.map_err(|_| BenchmarkError::Stop("failed to seed extended commitments"))?;
			vault
				.record_bitcoin_lock_funding(BitcoinLockFundingUpdate {
					funded_satoshis: securitization.basis.satoshis,
					securitized_satoshis: securitization.basis.satoshis,
					collateral_required: securitization.collateral_required(),
					eligible_satoshis: securitization
						.eligible_satoshis(securitization.basis.satoshis),
					is_flexible: false,
				})
				.map_err(|_| BenchmarkError::Stop("failed to activate securitization"))?;
			Ok::<_, BenchmarkError>(())
		})?;
		seed_treasury_bonds_for_benchmark::<T>(vault_id)?;

		#[block]
		{
			<Pallet<T> as BitcoinVaultProvider>::burn(
				vault_id,
				&securitization,
				securitization.basis.satoshis,
				MICROGONS_PER_ARGON.into(),
				&lock_extension,
				false,
			)
			.map_err(|_| BenchmarkError::Stop("failed to burn Bitcoin Lock securitization"))?;
		}

		let vault = VaultsById::<T>::get(vault_id)
			.ok_or(BenchmarkError::Stop("vault missing after burn"))?;
		assert_eq!(vault.securitization_locked, BalanceOf::<T>::zero());
		assert_eq!(vault.securitized_satoshis, 0);
		let bonds = BondLotsByVault::<T>::get(vault_id);
		assert!(bonds.locked_frame_terms.is_some());
		assert!(bonds.displaced_flexible_bonds > 0);
		Ok(())
	}

	#[benchmark]
	fn provider_encumber_argonots() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("provider_encumber_argonots", 0, 0);
		let vault_id = create_vault::<T>(&caller, 10, 100_000)?;
		let amount: BalanceOf<T> = 40_000u128.into();
		let _ = OwnershipCurrencyOf::<T>::mint_into(&caller, 1_000_000u128.into());
		Pallet::<T>::set_argonot_securitization(RawOrigin::Signed(caller.clone()).into(), amount)
			.map_err(|_| BenchmarkError::Stop("failed to set committed argonots"))?;

		#[block]
		{
			assert!(<Pallet<T> as BitcoinVaultProvider>::encumber_argonots(&caller, amount).is_ok());
		}

		assert_eq!(
			ArgonotSecuritizationByVaultId::<T>::get(vault_id)
				.map(|commitment| commitment.encumbered_micronots),
			Some(amount),
		);
		assert_eq!(
			OwnershipCurrencyOf::<T>::balance_on_hold(&HoldReason::EnterVault.into(), &caller),
			amount,
		);
		Ok(())
	}

	#[benchmark]
	fn provider_release_encumbered_argonots() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("provider_release_encumbered_argonots", 0, 0);
		let vault_id = create_vault::<T>(&caller, 10, 100_000)?;
		let amount: BalanceOf<T> = 40_000u128.into();
		let _ = OwnershipCurrencyOf::<T>::mint_into(&caller, 1_000_000u128.into());
		Pallet::<T>::set_argonot_securitization(RawOrigin::Signed(caller.clone()).into(), amount)
			.map_err(|_| BenchmarkError::Stop("failed to set committed argonots"))?;
		<Pallet<T> as BitcoinVaultProvider>::encumber_argonots(&caller, amount)
			.map_err(|_| BenchmarkError::Stop("failed to encumber argonots"))?;

		#[block]
		{
			assert!(<Pallet<T> as BitcoinVaultProvider>::release_encumbered_argonots(
				&caller, amount
			)
			.is_ok());
		}

		assert_eq!(
			ArgonotSecuritizationByVaultId::<T>::get(vault_id)
				.map(|commitment| commitment.encumbered_micronots),
			Some(BalanceOf::<T>::zero()),
		);
		assert_eq!(
			OwnershipCurrencyOf::<T>::balance_on_hold(&HoldReason::EnterVault.into(), &caller),
			amount,
		);
		Ok(())
	}

	#[benchmark]
	fn provider_burn_encumbered_argonots() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("provider_burn_encumbered_argonots", 0, 0);
		let vault_id = create_vault::<T>(&caller, 10, 100_000)?;
		let amount: BalanceOf<T> = 40_000u128.into();
		let _ = OwnershipCurrencyOf::<T>::mint_into(&caller, 1_000_000u128.into());
		Pallet::<T>::set_argonot_securitization(RawOrigin::Signed(caller.clone()).into(), amount)
			.map_err(|_| BenchmarkError::Stop("failed to set committed argonots"))?;
		<Pallet<T> as BitcoinVaultProvider>::encumber_argonots(&caller, amount)
			.map_err(|_| BenchmarkError::Stop("failed to encumber argonots"))?;

		#[block]
		{
			assert!(<Pallet<T> as BitcoinVaultProvider>::burn_encumbered_argonots(&caller, amount)
				.is_ok());
		}

		assert_eq!(
			<Pallet<T> as BitcoinVaultProvider>::get_held_argonots(&caller),
			Some(BalanceOf::<T>::zero()),
		);
		assert_eq!(
			ArgonotSecuritizationByVaultId::<T>::get(vault_id)
				.map(|commitment| commitment.encumbered_micronots),
			Some(BalanceOf::<T>::zero()),
		);
		assert_eq!(
			OwnershipCurrencyOf::<T>::balance_on_hold(&HoldReason::EnterVault.into(), &caller),
			BalanceOf::<T>::zero(),
		);
		Ok(())
	}

	#[benchmark]
	fn provider_account_became_operational() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("provider_became_operational", 0, 0);
		let vault_id = create_vault::<T>(&caller, 9, 100_000)?;

		#[block]
		{
			<Pallet<T> as BitcoinVaultProvider>::account_became_operational(&caller);
		}

		let vault = VaultsById::<T>::get(vault_id)
			.ok_or(BenchmarkError::Stop("vault missing after operational callback"))?;
		assert_eq!(
			vault.committed_microgons,
			T::OperationalMinimumVaultSecuritization::get().min(vault.securitization)
		);

		Ok(())
	}

	#[benchmark]
	fn on_frame_start(v: Linear<1, MAX_RELEASE_COMPLETIONS>) -> Result<(), BenchmarkError> {
		let source: T::AccountId = account("source", 99, 0);
		let earnings_for_vault: BalanceOf<T> = 1_000u128.into();
		let _ = CurrencyOf::<T>::mint_into(&source, 10_000_000_000u128.into());

		let frame_id = T::RevenueCollectionExpirationFrames::get().saturating_add(100u32.into());
		let collect_expired_frame =
			frame_id.saturating_sub(T::RevenueCollectionExpirationFrames::get());
		let first_expired_frame = collect_expired_frame.saturating_sub(11u32.into());

		for i in 0..v {
			let operator: T::AccountId = account("frame_start_operator", i, 0);
			let vault_id = create_vault::<T>(&operator, (i % 200) as u8, 100_000)?;
			for frame_offset in 0..12u32 {
				let expired_frame = first_expired_frame.saturating_add(frame_offset.into());
				<Pallet<T> as TreasuryVaultProvider>::record_vault_frame_earnings(
					&source,
					VaultTreasuryFrameEarnings {
						vault_id,
						vault_operator_account_id: operator.clone(),
						frame_id: expired_frame,
						earnings: earnings_for_vault,
						capital_contributed: earnings_for_vault,
						earnings_for_vault,
						capital_contributed_by_vault: earnings_for_vault,
					},
				)?;
			}
		}
		assert_eq!(RevenuePerFrameByVault::<T>::iter_keys().count(), v as usize);

		#[block]
		{
			let _ = <Pallet<T> as OnNewSlot<T::AccountId>>::on_frame_start(frame_id);
		}

		assert!(
			RevenuePerFrameByVault::<T>::iter_keys().next().is_none(),
			"expected on_frame_start to clear expired frame revenue for all vaults"
		);
		Ok(())
	}

	#[benchmark]
	fn on_initialize_with_vault_releases(
		h: Linear<1, 366>,
		v: Linear<1, MAX_RELEASE_COMPLETIONS>,
	) -> Result<(), BenchmarkError> {
		let start_height: BitcoinHeight = 10_000;
		let end_height = start_height.saturating_add((h.saturating_sub(1)).into());
		let previous_tip = BitcoinBlock::new(start_height, H256Le([1u8; 32]));
		let current_tip = BitcoinBlock::new(end_height, H256Le([2u8; 32]));
		pallet_bitcoin_utxos::PreviousBitcoinBlockTip::<T>::put(previous_tip);
		pallet_bitcoin_utxos::ConfirmedBitcoinBlockTip::<T>::put(current_tip);
		frame_system::Pallet::<T>::set_block_number(1u32.into());

		let mut first_vault_id: Option<VaultId> = None;
		let capital = 5_000 * MICROGONS_PER_ARGON;
		let release_amount: BalanceOf<T> = MICROGONS_PER_ARGON.into();
		for i in 0..v {
			let operator: T::AccountId = account("vault_operator", i, 0);
			let seed = (i % 200) as u8;
			let vault_id = create_vault::<T>(&operator, seed, capital)?;
			if first_vault_id.is_none() {
				first_vault_id = Some(vault_id);
			}
			let release_height = start_height.saturating_add((i % h).into());
			seed_release_schedule_for_benchmark::<T>(
				vault_id,
				release_height,
				MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES - 1,
				100u128.into(),
				(2_000 * MICROGONS_PER_ARGON).into(),
				capital.into(),
			)?;
			VaultsById::<T>::mutate(vault_id, |vault| {
				let vault = vault.as_mut().expect("benchmark vault should exist");
				vault.securitization_target = (capital - MICROGONS_PER_ARGON - 200_000).into();
				vault
					.scheduled_release(
						end_height.saturating_add(T::SecuritizationExitNoticeBlocks::get()),
					)
					.expect("one exit notice fits")
					.argon_withdrawals = 200_000u128.into();
				vault
					.scheduled_release(release_height)
					.expect("one due exit fits")
					.argon_withdrawals = release_amount;
			});
			seed_treasury_bonds_for_benchmark::<T>(vault_id)?;
			VaultFundsReleasingByHeight::<T>::mutate(release_height, |vaults| {
				vaults
					.try_insert(vault_id)
					.map_err(|_| BenchmarkError::Stop("vault release set overflow"))
			})?;
		}
		#[block]
		{
			let _ = Pallet::<T>::on_initialize(1u32.into());
		}

		for height in start_height..=end_height {
			assert!(
				VaultFundsReleasingByHeight::<T>::get(height).is_empty(),
				"release queue should be drained for each processed height"
			);
		}
		let first = first_vault_id.ok_or(BenchmarkError::Stop("missing benchmark vault"))?;
		let vault =
			VaultsById::<T>::get(first).ok_or(BenchmarkError::Stop("missing first vault"))?;
		assert!(
			vault.securitization < capital.into(),
			"vault securitization should shrink after releases"
		);
		let bonds = BondLotsByVault::<T>::get(first);
		assert!(bonds.locked_frame_terms.is_some());
		assert!(bonds.displaced_flexible_bonds > 0);
		Ok(())
	}
}
