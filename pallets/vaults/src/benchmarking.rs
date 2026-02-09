#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_bitcoin::{derive_xpub, xpriv_from_seed};
use argon_primitives::{
	bitcoin::{BitcoinHeight, OpaqueBitcoinXpub},
	vault::VaultTerms,
};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

const MAX_RELEASE_SCHEDULE_ENTRIES: u32 = 366;
// on_frame_start and on_initialize_with_vault_releases are linear in v. Runtime charging uses
// T::MaxVaults in on_frame_start_weight, so a smaller benchmark cap is enough to fit the slope.
const MAX_RELEASE_COMPLETIONS: u32 = 100;

fn benchmark_terms<T: Config>() -> VaultTerms<T::Balance> {
	VaultTerms {
		bitcoin_annual_percent_rate: FixedU128::from_rational(110u128, 100u128),
		bitcoin_base_fee: 1_000u128.into(),
		treasury_profit_sharing: Permill::from_percent(20),
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
) -> VaultConfig<T::Balance> {
	VaultConfig {
		terms: benchmark_terms::<T>(),
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
	T::Currency: frame_support::traits::fungible::Mutate<T::AccountId, Balance = T::Balance>,
{
	let funding = securitization.saturating_add(1_000_000);
	let _ = T::Currency::mint_into(operator, funding.into());
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
	entry_amount: T::Balance,
	locked: T::Balance,
	target: T::Balance,
) -> Result<(), BenchmarkError> {
	VaultsById::<T>::try_mutate(vault_id, |vault| {
		let vault = vault
			.as_mut()
			.ok_or(BenchmarkError::Stop("vault missing while seeding schedule"))?;
		vault.securitization_locked = locked;
		vault.securitization_pending_activation = 0u32.into();
		vault.securitization_target = target;
		vault
			.securitization_release_schedule
			.try_insert(release_at_or_before, entry_amount)
			.map_err(|_| BenchmarkError::Stop("unable to seed release schedule"))?;
		for i in 1..entries {
			let h = release_at_or_before.saturating_add(i.into());
			vault
				.securitization_release_schedule
				.try_insert(h, entry_amount)
				.map_err(|_| BenchmarkError::Stop("unable to seed release schedule"))?;
		}
		Ok(())
	})
}

#[benchmarks(
	where
		T::Currency: frame_support::traits::fungible::Mutate<T::AccountId, Balance = T::Balance>,
		T: pallet_bitcoin_utxos::Config,
)]
mod benchmarks {
	use super::*;
	use argon_primitives::{
		OnNewSlot,
		bitcoin::{BitcoinBlock, BitcoinHeight, BitcoinXPub, H256Le},
		vault::{TreasuryVaultProvider, VaultTreasuryFrameEarnings},
	};
	use frame_support::traits::{Get, Hooks, fungible::InspectHold};

	#[benchmark]
	fn create() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("create_caller", 0, 0);
		let securitization: u128 = 100_000;
		let _ = T::Currency::mint_into(&caller, securitization.saturating_add(1_000_000).into());
		let vault_config = benchmark_vault_config::<T>(1, securitization);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_config);

		assert!(VaultIdByOperator::<T>::contains_key(&caller));
		Ok(())
	}

	#[benchmark]
	fn modify_funding() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("funding_caller", 0, 0);
		let vault_id = create_vault::<T>(&caller, 2, 1_000_000)?;
		seed_release_schedule_for_benchmark::<T>(
			vault_id,
			30_000,
			MAX_RELEASE_SCHEDULE_ENTRIES,
			100u128.into(),
			200_000u128.into(),
			100_000u128.into(),
		)?;
		let securitization: T::Balance = 500_000u128.into();
		let ratio = FixedU128::one();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_id, securitization, ratio);

		let vault = VaultsById::<T>::get(vault_id)
			.ok_or(BenchmarkError::Stop("vault missing after modify"))?;
		assert_eq!(vault.securitization_target, securitization);
		assert_eq!(vault.securitization_ratio, ratio);
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
			treasury_profit_sharing: Permill::from_percent(25),
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
		let vault_id = create_vault::<T>(&caller, 4, 1_000_000)?;
		seed_release_schedule_for_benchmark::<T>(
			vault_id,
			40_000,
			MAX_RELEASE_SCHEDULE_ENTRIES,
			100u128.into(),
			200_000u128.into(),
			100_000u128.into(),
		)?;

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_id);

		let vault = VaultsById::<T>::get(vault_id)
			.ok_or(BenchmarkError::Stop("vault missing after close"))?;
		assert!(vault.is_closed);
		assert_eq!(vault.securitization_target, T::Balance::zero());
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
	fn collect() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("collect_caller", 0, 0);
		let vault_id = create_vault::<T>(&caller, 7, 100_000)?;
		let source: T::AccountId = account("source", 0, 0);
		let earnings_for_vault: T::Balance = 50_000u128.into();
		let _ = T::Currency::mint_into(&source, 1_000_000_000u128.into());

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
			);
		}
		assert_eq!(RevenuePerFrameByVault::<T>::get(vault_id).len(), 12);
		assert!(
			RevenuePerFrameByVault::<T>::get(vault_id)
				.iter()
				.any(|entry| entry.uncollected_revenue > T::Balance::zero()),
			"expected uncollected revenue before collect"
		);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), vault_id);

		assert_eq!(LastCollectFrameByVaultId::<T>::get(vault_id), Some(T::CurrentFrameId::get()));
		assert_eq!(
			T::Currency::balance_on_hold(&HoldReason::PendingCollect.into(), &caller),
			T::Balance::zero()
		);
		assert!(
			RevenuePerFrameByVault::<T>::get(vault_id)
				.iter()
				.all(|entry| entry.uncollected_revenue == T::Balance::zero()),
			"expected all frame revenue to be collected"
		);
		Ok(())
	}

	#[benchmark]
	fn on_frame_start(v: Linear<1, MAX_RELEASE_COMPLETIONS>) -> Result<(), BenchmarkError> {
		let source: T::AccountId = account("source", 99, 0);
		let earnings_for_vault: T::Balance = 1_000u128.into();
		let _ = T::Currency::mint_into(&source, 10_000_000_000u128.into());

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
				);
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

		let mut first_vault_id: Option<VaultId> = None;
		let release_amount: T::Balance = 1_000u128.into();
		for i in 0..v {
			let operator: T::AccountId = account("vault_operator", i, 0);
			let seed = (i % 200) as u8;
			let vault_id = create_vault::<T>(&operator, seed, 1_000_000)?;
			if first_vault_id.is_none() {
				first_vault_id = Some(vault_id);
			}
			let release_height = start_height.saturating_add((i % h).into());
			seed_release_schedule_for_benchmark::<T>(
				vault_id,
				release_height,
				MAX_RELEASE_SCHEDULE_ENTRIES,
				release_amount,
				200_000u128.into(),
				100_000u128.into(),
			)?;
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
			vault.securitization < 1_000_000u128.into(),
			"vault securitization should shrink after releases"
		);
		Ok(())
	}
}
