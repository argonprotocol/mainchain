#![allow(clippy::inconsistent_digit_grouping)]
use std::collections::{BTreeMap, BTreeSet};

use bitcoin::PublicKey;
use pallet_prelude::*;

use crate as pallet_bitcoin_locks;
use crate::BitcoinVerifier;
use argon_bitcoin::CosignReleaser;
use argon_primitives::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinNetwork, BitcoinSignature, BitcoinXPub,
		CompressedBitcoinPubkey, NetworkKind, Satoshis, UtxoId, UtxoRef,
	},
	vault::{
		BitcoinVaultProvider, LockExtension, Securitization, Vault, VaultError, VaultLockRequest,
		VaultTerms,
	},
	ArgonCPI, BitcoinUtxoTracker, PriceProvider, UtxoLockEvents,
};
use frame_support::traits::Currency;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		BitcoinLocks: pallet_bitcoin_locks
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
}

parameter_types! {
	pub static ExistentialDeposit: Balance = 10;
	pub const BlocksPerYear:u32 = 1440*365;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type ReserveIdentifier = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

pub fn set_argons(account_id: u64, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

parameter_types! {
	pub static MaxConcurrentlyReleasingLocks: u32 = 10;
	pub static BitcoinPriceInUsd: Option<FixedU128> = Some(FixedU128::from_rational(62_000_00, 100));
	pub static ArgonPriceInUsd: Option<FixedU128> = Some(FixedU128::from_rational(100, 100));
	pub static ArgonTargetPriceInUsd: Option<FixedU128> = Some(FixedU128::from_rational(100, 100));
	pub static LockReleaseCosignDeadlineFrames: FrameId = 5;
	pub static OrphanedUtxoReleaseExpiryFrames: FrameId = 5;
	pub static LockReclamationBlocks: BitcoinHeight = 30;
	pub static LockDurationBlocks: BitcoinHeight = 144 * 365;
	pub static BitcoinBlockHeightChange: (BitcoinHeight, BitcoinHeight) = (0, 0);
	pub static MinimumLockSatoshis: Satoshis = 10_000_000;
	pub static DefaultVault: Vault<u64, Balance> = Vault {
		operator_account_id: 1,
		delegate_account_id: None,
		securitization:  200_000_000_000,
		securitization_target: 200_000_000_000,
		securitization_locked: 0,
		flexible_securitization_locked: 0,
		reserved_securitization_space: 0,
		locked_satoshis: 0,
		securitized_satoshis: 0,
		flexible_securitized_satoshis: 0,
		terms: VaultTerms {
			bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
			bitcoin_base_fee: 0,
			treasury_profit_sharing: Permill::from_float(0.0),
		},
		opened_tick: 1,
		securitization_ratio: FixedU128::from_float(1.0),
		securitization_release_schedule: BoundedBTreeMap::new(),
		is_closed: false,
		pending_terms: None,
		securitization_pending_activation: 0,
		operational_minimum_release_tick: None,
	};

	pub static NextUtxoId: UtxoId = 1;
	pub static WatchedUtxosById: BTreeMap<UtxoId, (BitcoinCosignScriptPubkey, Satoshis, BitcoinHeight)> = BTreeMap::new();

	pub static GetUtxoRef: Option<UtxoRef> = None;
	pub static CandidateUtxosByRef: BTreeMap<UtxoRef, (UtxoId, Satoshis)> = BTreeMap::new();

	pub static LastLockEvent: Option<(UtxoId, u64, Balance)> = None;
	pub static LastReleaseEvent: Option<(UtxoId, u64, bool, Balance, Balance)> = None;

	pub static GetBitcoinNetwork: BitcoinNetwork = BitcoinNetwork::Regtest;

	pub static DefaultVaultBitcoinPubkey: PublicKey = "02e3af28965693b9ce1228f9d468149b831d6a0540b25e8a9900f71372c11fb277".parse::<PublicKey>().unwrap();
	pub static DefaultVaultReclaimBitcoinPubkey: PublicKey = "026c468be64d22761c30cd2f12cbc7de255d592d7904b1bab07236897cc4c2e766".parse::<PublicKey>().unwrap();

	pub static CurrentFrameId: FrameId = 1;

	pub static CanceledLocks: Vec<(VaultId, Balance)> = Vec::new();

	pub static ChargeFee: bool = false;

	pub static VaultViewOfCosignPendingLocks: BTreeMap<VaultId,  BTreeSet<UtxoId>> = BTreeMap::new();
	pub static VaultViewOfOrphanedUtxoCosigns: BTreeMap<VaultId,  BTreeMap<u64, u32>> = BTreeMap::new();
	pub const TicksPerBitcoinBlock: u64 = 10;
	pub const ArgonTicksPerDay: u64 = 1440;
	pub static CurrentTick: Tick = 1;
	pub static DidStartNewFrame: bool = true;
	pub static UseRealBitcoinVerifier: bool = false;
}

pub struct EventHandler;
impl UtxoLockEvents<u64, Balance> for EventHandler {
	type Weights = ();

	fn utxo_locked(
		utxo_id: UtxoId,
		account_id: &u64,
		amount: Balance,
	) -> Result<(), DispatchError> {
		LastLockEvent::set(Some((utxo_id, *account_id, amount)));
		Ok(())
	}
	fn utxo_released(
		utxo_id: UtxoId,
		account_id: &u64,
		remove_pending_mints: bool,
		amount_burned: Balance,
		original_liquidity_promised: Balance,
	) -> DispatchResult {
		LastReleaseEvent::set(Some((
			utxo_id,
			*account_id,
			remove_pending_mints,
			amount_burned,
			original_liquidity_promised,
		)));

		Ok(())
	}
}

pub struct StaticPriceProvider;
impl PriceProvider<Balance> for StaticPriceProvider {
	type Weights = ();

	fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
		BitcoinPriceInUsd::get()
	}
	fn get_latest_argon_price_in_usd() -> Option<FixedU128> {
		ArgonPriceInUsd::get()
	}
	fn get_argonot_price_in_usd() -> Option<FixedU128> {
		ArgonPriceInUsd::get()
	}
	fn get_target_argon_price_in_usd() -> Option<FixedU128> {
		ArgonTargetPriceInUsd::get()
	}
	fn get_argon_cpi() -> Option<ArgonCPI> {
		let ratio = ArgonTargetPriceInUsd::get()? / ArgonPriceInUsd::get()?;
		let ratio_as_cpi = ArgonCPI::from_inner(ratio.into_inner() as i128);
		Some(ratio_as_cpi - One::one())
	}
	fn get_redemption_r_value() -> Option<FixedU128> {
		Some(ArgonPriceInUsd::get()? / ArgonTargetPriceInUsd::get()?)
	}
	fn get_circulation() -> Balance {
		1000
	}
	fn get_average_cpi_for_ticks(_tick_range: (Tick, Tick)) -> ArgonCPI {
		Self::get_argon_cpi().unwrap_or_default()
	}
}

pub struct StaticVaultProvider;

impl BitcoinVaultProvider for StaticVaultProvider {
	type Weights = ();
	type Balance = Balance;
	type AccountId = u64;

	fn is_owner(vault_id: VaultId, account_id: &Self::AccountId) -> bool {
		if vault_id == 1 {
			return DefaultVault::get().operator_account_id == *account_id;
		}
		false
	}

	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId> {
		if vault_id == 1 {
			return Some(DefaultVault::get().operator_account_id);
		}
		None
	}

	fn get_vault_delegate(vault_id: VaultId) -> Option<Self::AccountId> {
		if vault_id == 1 {
			return DefaultVault::get().delegate_account_id;
		}
		None
	}

	fn get_vault_id(account_id: &Self::AccountId) -> Option<VaultId> {
		if DefaultVault::get().operator_account_id == *account_id {
			return Some(1);
		}
		None
	}

	fn get_locked_securitization(vault_id: VaultId) -> Option<Self::Balance> {
		(vault_id == 1).then(|| DefaultVault::get().securitization_locked)
	}

	fn get_registration_vault_data(
		account_id: &Self::AccountId,
	) -> Option<argon_primitives::vault::RegistrationVaultData<Self::Balance>> {
		Self::get_vault_id(account_id).map(|vault_id| {
			let vault = DefaultVault::get();
			argon_primitives::vault::RegistrationVaultData {
				vault_id,
				activated_securitization: vault.get_activated_securitization(),
				securitization: vault.securitization,
			}
		})
	}

	fn get_committed_securitization(
		account_id: &Self::AccountId,
		_min_frames_remaining: FrameId,
	) -> Option<Self::Balance> {
		Self::get_vault_id(account_id).map(|_| {
			let vault = DefaultVault::get();
			vault.get_activated_securitization().saturating_add(vault.get_relock_capacity())
		})
	}

	fn get_committed_argonots(account_id: &Self::AccountId) -> Option<Self::Balance> {
		Self::get_vault_id(account_id).map(|_| Default::default())
	}

	fn encumber_argonots(
		_account_id: &Self::AccountId,
		_amount: Self::Balance,
	) -> Result<(), argon_primitives::vault::VaultError> {
		Ok(())
	}

	fn release_encumbered_argonots(
		_account_id: &Self::AccountId,
		_amount: Self::Balance,
	) -> Result<(), argon_primitives::vault::VaultError> {
		Ok(())
	}

	fn burn_encumbered_argonots(
		_account_id: &Self::AccountId,
		_amount: Self::Balance,
	) -> Result<(), argon_primitives::vault::VaultError> {
		Ok(())
	}

	fn cancel(
		vault_id: VaultId,
		securitization: &Securitization<Balance>,
	) -> Result<(), VaultError> {
		DefaultVault::mutate(|v| {
			v.release_lock(securitization);
		});
		CanceledLocks::mutate(|a| a.push((vault_id, securitization.liquidity_promised)));
		Ok(())
	}

	fn lock(
		_vault_id: VaultId,
		locker: &Self::AccountId,
		securitization: &Securitization<Balance>,
		request: VaultLockRequest<'_, Self::Balance>,
	) -> Result<(Self::Balance, Self::Balance), VaultError> {
		let VaultLockRequest {
			extension,
			fee_discount,
			is_flexible,
			securitization_space_to_unreserve,
			..
		} = request;
		let is_operator = DefaultVault::get().operator_account_id == *locker;
		let may_use_flexible_space = !is_operator;
		let term = extension.as_ref().map(|(a, _)| *a).unwrap_or(FixedU128::one());
		DefaultVault::mutate(|a| {
			if let Some((_, lock_extension)) = extension {
				a.extend_lock(securitization, lock_extension, is_flexible, may_use_flexible_space)
			} else {
				a.reserved_securitization_space
					.saturating_reduce(securitization_space_to_unreserve);
				a.lock(securitization, may_use_flexible_space)
			}
		})?;
		let terms = DefaultVault::get().terms.clone();
		let total_fee = terms
			.bitcoin_annual_percent_rate
			.saturating_mul(term)
			.saturating_mul_int(securitization.liquidity_promised)
			.saturating_add(terms.bitcoin_base_fee);
		let fee_discount = if is_operator { total_fee } else { fee_discount.min(total_fee) };
		if ChargeFee::get() {
			Balances::burn_from(
				locker,
				total_fee.saturating_sub(fee_discount),
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)
			.map_err(|_| VaultError::InsufficientFunds)?;
		}
		Ok((total_fee, fee_discount))
	}

	fn schedule_for_release(
		_vault_id: VaultId,
		securitization: &Securitization<Balance>,
		satoshis: Satoshis,
		lock_extensions: &LockExtension<Self::Balance>,
		is_flexible: bool,
	) -> Result<(), VaultError> {
		DefaultVault::mutate(|a| {
			a.schedule_for_release(securitization, satoshis, lock_extensions, is_flexible)
		})?;
		Ok(())
	}

	fn compensate_lost_bitcoin(
		_vault_id: VaultId,
		_beneficiary: &Self::AccountId,
		securitization: &Securitization<Balance>,
		satoshis: Satoshis,
		market_rate: Self::Balance,
		lock_extension: &LockExtension<Self::Balance>,
		is_flexible: bool,
	) -> Result<Self::Balance, VaultError> {
		let result = DefaultVault::mutate(|a| {
			a.burn(securitization, satoshis, market_rate, lock_extension, is_flexible)
		})?;
		Ok(result.burned_amount)
	}

	fn burn(
		_vault_id: VaultId,
		securitization: &Securitization<Balance>,
		satoshis: Satoshis,
		redemption_amount: Self::Balance,
		lock_extension: &LockExtension<Self::Balance>,
		is_flexible: bool,
	) -> Result<Self::Balance, VaultError> {
		let result = DefaultVault::mutate(|a| {
			a.burn(securitization, satoshis, redemption_amount, lock_extension, is_flexible)
		})?;
		Ok(result.burned_amount)
	}

	fn create_utxo_script_pubkey(
		_vault_id: VaultId,
		_owner_pubkey: CompressedBitcoinPubkey,
		_vault_claim_height: BitcoinHeight,
		_open_claim_height: BitcoinHeight,
		_current_height: BitcoinHeight,
	) -> Result<(BitcoinXPub, BitcoinXPub, BitcoinCosignScriptPubkey), VaultError> {
		Ok((
			BitcoinXPub {
				public_key: DefaultVaultBitcoinPubkey::get().into(),
				chain_code: [0; 32],
				depth: 0,
				parent_fingerprint: [0; 4],
				child_number: 0,
				network: NetworkKind::Test,
			},
			BitcoinXPub {
				public_key: DefaultVaultReclaimBitcoinPubkey::get().into(),
				chain_code: [0; 32],
				depth: 0,
				parent_fingerprint: [0; 4],
				child_number: 1,
				network: NetworkKind::Test,
			},
			BitcoinCosignScriptPubkey::P2WSH { wscript_hash: H256::from([0; 32]) },
		))
	}

	fn remove_pending(
		_vault_id: VaultId,
		securitization: &Securitization<Balance>,
	) -> Result<(), VaultError> {
		DefaultVault::mutate(|a| {
			a.remove_pending_activation(securitization);
		});
		Ok(())
	}

	fn update_pending_cosign_list(
		vault_id: VaultId,
		utxo_id: UtxoId,
		should_remove: bool,
	) -> Result<(), VaultError> {
		VaultViewOfCosignPendingLocks::mutate(|l| {
			let list = l.entry(vault_id).or_default();
			if should_remove {
				list.remove(&utxo_id);
			} else {
				list.insert(utxo_id);
			}
		});
		Ok(())
	}

	fn update_orphan_cosign_list(
		vault_id: VaultId,
		_utxo_id: UtxoId,
		account_id: &Self::AccountId,
		should_remove: bool,
	) -> Result<(), VaultError> {
		VaultViewOfOrphanedUtxoCosigns::mutate(|x| {
			let vault_map = x.entry(vault_id).or_default();
			let count = vault_map.entry(*account_id).or_default();
			if should_remove {
				*count = count.saturating_sub(1);
				if *count == 0 {
					vault_map.remove(account_id);
				}
			} else {
				*count = count.saturating_add(1);
			}
			if vault_map.is_empty() {
				x.remove(&vault_id);
			}
		});
		Ok(())
	}

	fn get_securitization_ratio(_vault_id: VaultId) -> Result<FixedU128, VaultError> {
		Ok(DefaultVault::get().securitization_ratio)
	}

	fn add_securitized_satoshis(
		_vault_id: VaultId,
		satoshis: Satoshis,
		securitization_ratio: FixedU128,
	) -> Result<(), VaultError> {
		DefaultVault::mutate(|vault| {
			vault.add_securitized_satoshis(satoshis, securitization_ratio);
		});
		Ok(())
	}

	fn get_projected_flexible_securitization(
		_vault_id: VaultId,
		flexible_securitization_released: Self::Balance,
		flexible_securitization_added: Self::Balance,
	) -> Option<(Self::Balance, Self::Balance)> {
		Some(DefaultVault::get().projected_flexible_securitization(
			flexible_securitization_released,
			flexible_securitization_added,
		))
	}

	fn set_bitcoin_lock_flexible(
		_vault_id: VaultId,
		securitization: &Securitization<Self::Balance>,
		satoshis: Satoshis,
		is_flexible: bool,
	) -> Result<(), VaultError> {
		DefaultVault::mutate(|vault| {
			vault.set_bitcoin_lock_flexible(securitization, satoshis, is_flexible)
		})
	}
}

pub struct StaticBitcoinVerifier;
impl BitcoinVerifier<Test> for StaticBitcoinVerifier {
	fn verify_signature(
		utxo_releaseer: CosignReleaser,
		pubkey: CompressedBitcoinPubkey,
		signature: &BitcoinSignature,
	) -> Result<bool, DispatchError> {
		if UseRealBitcoinVerifier::get() {
			return utxo_releaseer.verify_signature_raw(pubkey, signature).map_err(|e| {
				match e {
					argon_bitcoin::Error::InvalidCompressPubkeyBytes =>
						pallet_bitcoin_locks::Error::<Test>::BitcoinPubkeyUnableToBeDecoded,
					argon_bitcoin::Error::InvalidSignatureBytes =>
						pallet_bitcoin_locks::Error::<Test>::BitcoinSignatureUnableToBeDecoded,
					_ => pallet_bitcoin_locks::Error::<Test>::BitcoinInvalidCosignature,
				}
				.into()
			});
		}
		Ok(true)
	}
}

pub struct StaticBitcoinUtxoTracker;
impl BitcoinUtxoTracker for StaticBitcoinUtxoTracker {
	fn get_funding_utxo_ref(_utxo_id: UtxoId) -> Option<UtxoRef> {
		GetUtxoRef::get()
	}

	fn watch_for_utxo(
		utxo_id: UtxoId,
		script_pubkey: BitcoinCosignScriptPubkey,
		satoshis: Satoshis,
		watch_for_spent_until: BitcoinHeight,
	) -> Result<(), DispatchError> {
		WatchedUtxosById::mutate(|watched_utxos| {
			watched_utxos.insert(utxo_id, (script_pubkey, satoshis, watch_for_spent_until));
		});
		Ok(())
	}

	fn unwatch(utxo_id: UtxoId) {
		WatchedUtxosById::mutate(|watched_utxos| {
			watched_utxos.remove(&utxo_id);
		});
		CandidateUtxosByRef::mutate(|candidates| {
			candidates.retain(|_utxo_ref, (id, _)| *id != utxo_id);
		});
	}

	fn unwatch_candidate(utxo_id: UtxoId, utxo_ref: &UtxoRef) -> Option<(UtxoRef, Satoshis)> {
		let mut removed = None;
		CandidateUtxosByRef::mutate(|candidates| {
			if let Some((id, satoshis)) = candidates.remove(utxo_ref) {
				if id == utxo_id {
					removed = Some((utxo_ref.clone(), satoshis));
				} else {
					candidates.insert(utxo_ref.clone(), (id, satoshis));
				}
			}
		});
		removed
	}
}

pub(crate) fn insert_candidate_utxo(utxo_ref: UtxoRef, utxo_id: UtxoId, satoshis: Satoshis) {
	CandidateUtxosByRef::mutate(|candidates| {
		candidates.insert(utxo_ref, (utxo_id, satoshis));
	});
}

pub(crate) fn set_bitcoin_height(height: BitcoinHeight) {
	BitcoinBlockHeightChange::set((height, height));
}

impl pallet_bitcoin_locks::Config for Test {
	type WeightInfo = ();
	type Currency = Balances;
	type Balance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type LockEvents = (EventHandler,);
	type BitcoinUtxoTracker = StaticBitcoinUtxoTracker;
	type PriceProvider = StaticPriceProvider;
	type BitcoinSignatureVerifier = StaticBitcoinVerifier;
	type FeeCouponSigner = polkadot_sdk::sp_runtime::testing::UintAuthorityId;
	type FeeCouponSignature = polkadot_sdk::sp_runtime::testing::TestSignature;
	type GetBitcoinNetwork = GetBitcoinNetwork;
	type VaultProvider = StaticVaultProvider;
	type ArgonTicksPerDay = ArgonTicksPerDay;
	type MaxConcurrentlyReleasingLocks = MaxConcurrentlyReleasingLocks;
	type LockDurationBlocks = LockDurationBlocks;
	type LockReclamationBlocks = LockReclamationBlocks;
	type LockReleaseCosignDeadlineFrames = LockReleaseCosignDeadlineFrames;
	type OrphanedUtxoReleaseExpiryFrames = OrphanedUtxoReleaseExpiryFrames;
	type BitcoinBlockHeightChange = BitcoinBlockHeightChange;
	type MaxConcurrentlyExpiringLocks = ConstU32<100>;
	type CurrentFrameId = CurrentFrameId;
	type TicksPerBitcoinBlock = TicksPerBitcoinBlock;
	type CurrentTick = CurrentTick;
	type MaxBtcPriceTickAge = ConstU32<10>;
	type DidStartNewFrame = DidStartNewFrame;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> TestState {
	DefaultVault::set(Vault {
		operator_account_id: 1,
		delegate_account_id: None,
		securitization: 200_000_000_000,
		securitization_target: 200_000_000_000,
		securitization_locked: 0,
		flexible_securitization_locked: 0,
		reserved_securitization_space: 0,
		locked_satoshis: 0,
		securitized_satoshis: 0,
		flexible_securitized_satoshis: 0,
		terms: VaultTerms {
			bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
			bitcoin_base_fee: 0,
			treasury_profit_sharing: Permill::from_float(0.0),
		},
		opened_tick: 1,
		securitization_ratio: FixedU128::from_float(1.0),
		securitization_release_schedule: BoundedBTreeMap::new(),
		is_closed: false,
		pending_terms: None,
		securitization_pending_activation: 0,
		operational_minimum_release_tick: None,
	});
	new_test_with_genesis::<Test>(|t: &mut Storage| {
		pallet_bitcoin_locks::GenesisConfig::<Test> {
			minimum_bitcoin_lock_satoshis: MinimumLockSatoshis::get(),
			_phantom: Default::default(),
		}
		.assimilate_storage(t)
		.unwrap();
	})
}
