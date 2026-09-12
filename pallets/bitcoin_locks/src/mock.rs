#![allow(clippy::inconsistent_digit_grouping)]
use std::collections::{BTreeMap, BTreeSet};

use bitcoin::PublicKey;
use pallet_prelude::*;

use crate as pallet_bitcoin_locks;
use crate::BitcoinVerifier;
use argon_bitcoin::CosignReleaser;
use argon_primitives::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinLockId, BitcoinNetwork, BitcoinSignature,
		BitcoinXPub, CompressedBitcoinPubkey, NetworkKind, Satoshis, UtxoRef,
	},
	vault::{
		BitcoinLockFundingUpdate, BitcoinResecuritization, BitcoinSecuritization,
		BitcoinVaultProvider, LockExtension, LostBitcoinCompensation, ReserveSecuritizationRequest,
		Vault, VaultError, VaultTerms,
	},
	ArgonCPI, BitcoinUtxoTracker, BlockRewardAccountsProvider, MiningFrameProvider,
	MiningFrameTransitionProvider, OperationalAccountsHook, PriceProvider,
};
use frame_support::traits::Currency;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		BitcoinUtxos: pallet_bitcoin_utxos,
		BitcoinLocks: pallet_bitcoin_locks,
		BitcoinFissions: pallet_bitcoin_fissions,
		Mint: pallet_mint,
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
	pub static SecuritizationHoldBlocks: BitcoinHeight = 144;
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
		total_satoshis: 0,
		securitized_satoshis: 0,
		ratio_adjusted_satoshis: 0,
		flexible_ratio_adjusted_satoshis: 0,
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

	pub static NextBitcoinLockId: BitcoinLockId = 1;
	pub static WatchedUtxosById: BTreeMap<BitcoinLockId, BitcoinCosignScriptPubkey> = BTreeMap::new();

	pub static GetBitcoinNetwork: BitcoinNetwork = BitcoinNetwork::Regtest;

	pub static DefaultVaultBitcoinPubkey: PublicKey = "02e3af28965693b9ce1228f9d468149b831d6a0540b25e8a9900f71372c11fb277".parse::<PublicKey>().unwrap();
	pub static DefaultVaultReclaimBitcoinPubkey: PublicKey = "026c468be64d22761c30cd2f12cbc7de255d592d7904b1bab07236897cc4c2e766".parse::<PublicKey>().unwrap();

	pub static CurrentFrameId: FrameId = 1;

	pub static CanceledLocks: Vec<(VaultId, Balance)> = Vec::new();

	pub static ChargeFee: bool = false;
	pub static FailReturnSecuritization: bool = false;

	pub static VaultViewOfCosignPendingLocks: BTreeMap<VaultId,  BTreeSet<BitcoinLockId>> = BTreeMap::new();
	pub static VaultViewOfOrphanedUtxoCosigns: BTreeMap<VaultId,  BTreeMap<u64, u32>> = BTreeMap::new();
	pub const TicksPerBitcoinBlock: u64 = 10;
	pub const ArgonTicksPerDay: u64 = 1440;
	pub static CurrentTick: Tick = 1;
	pub static DidStartNewFrame: bool = true;
	pub static UseRealBitcoinVerifier: bool = false;
	pub static MinimumRatchetPercent: Percent = Percent::from_percent(10);
	pub static MaxPendingMintsPerUtxo: u32 = 50;
	pub static MaxPendingMintPayoutWindowSize: u32 = 1_000;
	pub static BitcoinMintPayoutPercentPerFrame: Percent = Percent::from_percent(10);
	pub static AccountBitcoinChanges: Vec<(u64, Balance, bool)> = Vec::new();
}

pub struct StaticMiningFrameProvider;
impl MiningFrameTransitionProvider for StaticMiningFrameProvider {
	fn is_new_frame_started() -> Option<FrameId> {
		None
	}

	fn get_current_frame_id() -> FrameId {
		CurrentFrameId::get()
	}
}

impl MiningFrameProvider for StaticMiningFrameProvider {
	fn get_next_frame_tick() -> Tick {
		CurrentTick::get().saturating_add(1)
	}

	fn is_seat_bidding_started() -> bool {
		true
	}

	fn get_tick_range_for_frame(_frame_id: FrameId) -> Option<(Tick, Tick)> {
		Some((0, CurrentTick::get()))
	}
}

pub struct StaticBlockRewardAccountsProvider;
impl BlockRewardAccountsProvider<u64> for StaticBlockRewardAccountsProvider {
	type Weights = ();

	fn get_block_rewards_account(_author: &u64) -> Option<(u64, FrameId)> {
		None
	}

	fn get_mint_rewards_accounts() -> Vec<(u64, FrameId)> {
		Vec::new()
	}

	fn is_compute_block_eligible_for_rewards() -> bool {
		false
	}
}

pub struct MockOperationalAccounts;
impl OperationalAccountsHook<u64, Balance> for MockOperationalAccounts {
	fn vault_created_weight() -> Weight {
		Weight::zero()
	}

	fn vault_bitcoin_lock_funded_weight() -> Weight {
		Weight::zero()
	}

	fn mining_seat_won_weight() -> Weight {
		Weight::zero()
	}

	fn account_bitcoin_amount_changed_weight() -> Weight {
		Weight::zero()
	}

	fn account_bitcoin_amount_changed(account_id: &u64, amount: Balance, is_increase: bool) {
		AccountBitcoinChanges::mutate(|changes| changes.push((*account_id, amount, is_increase)));
	}

	fn account_vault_bond_total_updated_weight() -> Weight {
		Weight::zero()
	}

	fn account_uniswap_argon_transfers_in_updated_weight() -> Weight {
		Weight::zero()
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

	fn release_unactivated_securitization(
		vault_id: VaultId,
		amount: Balance,
	) -> Result<(), VaultError> {
		if FailReturnSecuritization::get() {
			return Err(VaultError::InternalError);
		}
		DefaultVault::mutate(|vault| vault.release_unactivated_securitization(amount))?;
		CanceledLocks::mutate(|locks| {
			locks.push((vault_id, amount));
		});
		Ok(())
	}

	fn reserve_securitization(
		_vault_id: VaultId,
		locker: &Self::AccountId,
		securitization: &BitcoinSecuritization<Balance>,
		request: ReserveSecuritizationRequest<Self::Balance>,
	) -> Result<(Self::Balance, Self::Balance), VaultError> {
		let ReserveSecuritizationRequest { fee_discount, securitization_space_to_unreserve } =
			request;
		let is_operator = DefaultVault::get().operator_account_id == *locker;
		let may_use_flexible_space = !is_operator;
		DefaultVault::mutate(|vault| {
			vault
				.reserved_securitization_space
				.saturating_reduce(securitization_space_to_unreserve);
			vault.reserve_securitization(securitization, may_use_flexible_space)
		})?;
		let terms = DefaultVault::get().terms.clone();
		let total_fee = terms
			.bitcoin_annual_percent_rate
			.saturating_mul_int(securitization.securitization_coverage_microgons)
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

	fn resecuritize(
		_vault_id: VaultId,
		locker: &Self::AccountId,
		request: BitcoinResecuritization<'_, Self::Balance>,
	) -> Result<(Self::Balance, Self::Balance), VaultError> {
		let BitcoinResecuritization {
			current,
			replacement,
			funded_satoshis,
			remaining_term,
			lock_extension,
			is_flexible,
			fee_discount,
			securitization_space_to_unreserve,
		} = request;
		let vault = DefaultVault::get();
		let is_operator = vault.operator_account_id == *locker;
		let additional_securitization_coverage_microgons = replacement
			.securitization_coverage_microgons
			.saturating_sub(current.securitization_coverage_microgons);
		let total_fee = if additional_securitization_coverage_microgons.is_zero() {
			Balance::zero()
		} else {
			vault
				.terms
				.bitcoin_annual_percent_rate
				.saturating_mul(remaining_term)
				.saturating_mul_int(additional_securitization_coverage_microgons)
				.saturating_add(vault.terms.bitcoin_base_fee)
		};
		let fee_discount = if is_operator { total_fee } else { fee_discount.min(total_fee) };
		DefaultVault::mutate(|vault| {
			vault
				.reserved_securitization_space
				.saturating_reduce(securitization_space_to_unreserve);
			vault.replace_securitization(
				current,
				replacement,
				funded_satoshis,
				lock_extension,
				is_flexible,
				!is_operator,
			)
		})?;
		if ChargeFee::get() && !is_operator {
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

	fn release_bitcoin_lock_securitization(
		_vault_id: VaultId,
		current_securitization: &BitcoinSecuritization<Balance>,
		lock_funded_satoshis: Satoshis,
		lock_extensions: &LockExtension<Self::Balance>,
		is_flexible: bool,
	) -> Result<(), VaultError> {
		DefaultVault::mutate(|vault| {
			vault.release_bitcoin_lock_securitization(
				current_securitization,
				lock_funded_satoshis,
				lock_extensions,
				is_flexible,
			)
		})?;
		Ok(())
	}

	fn compensate_lost_bitcoin(
		_vault_id: VaultId,
		_beneficiary: &Self::AccountId,
		securitization: &BitcoinSecuritization<Balance>,
		satoshis: Satoshis,
		market_rate: Self::Balance,
		lock_extension: &LockExtension<Self::Balance>,
		is_flexible: bool,
	) -> Result<LostBitcoinCompensation<Self::Balance>, VaultError> {
		let result = DefaultVault::mutate(|a| {
			a.burn(securitization, satoshis, market_rate, lock_extension, is_flexible)
		})?;
		let to_beneficiary = result
			.burned_amount
			.saturating_sub(securitization.securitization_coverage_microgons);
		let burned = result.burned_amount.saturating_sub(to_beneficiary);
		Ok(LostBitcoinCompensation { to_beneficiary, burned })
	}

	fn burn(
		_vault_id: VaultId,
		securitization: &BitcoinSecuritization<Balance>,
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

	fn update_pending_cosign_list(
		vault_id: VaultId,
		lock_id: BitcoinLockId,
		should_remove: bool,
	) -> Result<(), VaultError> {
		VaultViewOfCosignPendingLocks::mutate(|l| {
			let list = l.entry(vault_id).or_default();
			if should_remove {
				list.remove(&lock_id);
			} else {
				list.insert(lock_id);
			}
		});
		Ok(())
	}

	fn update_orphan_cosign_list(
		vault_id: VaultId,
		_lock_id: BitcoinLockId,
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

	fn record_bitcoin_lock_funding(
		_vault_id: VaultId,
		update: BitcoinLockFundingUpdate<Self::Balance>,
	) -> Result<(), VaultError> {
		DefaultVault::mutate(|vault| vault.record_bitcoin_lock_funding(update))
	}

	fn record_bitcoin_lock_funding_reduction(
		_vault_id: VaultId,
		update: BitcoinLockFundingUpdate<Self::Balance>,
	) -> Result<(), VaultError> {
		DefaultVault::mutate(|vault| vault.record_bitcoin_lock_funding_reduction(update))
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
		securitization: &BitcoinSecuritization<Self::Balance>,
		securitized_satoshis: Satoshis,
		is_flexible: bool,
	) -> Result<(), VaultError> {
		DefaultVault::mutate(|vault| {
			vault.set_bitcoin_lock_flexible(securitization, securitized_satoshis, is_flexible)
		})
	}
}

pub struct StaticBitcoinVerifier;
impl BitcoinVerifier<Test> for StaticBitcoinVerifier {
	fn verify_signatures(
		utxo_releaseer: CosignReleaser,
		pubkey: CompressedBitcoinPubkey,
		signatures: &[BitcoinSignature],
	) -> Result<bool, DispatchError> {
		if UseRealBitcoinVerifier::get() {
			return utxo_releaseer.verify_signatures_raw(pubkey, signatures).map_err(|e| {
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
	fn get_synched_height() -> BitcoinHeight {
		pallet_bitcoin_utxos::SynchedBitcoinBlock::<Test>::get()
			.map(|block| block.block_height)
			.unwrap_or_else(|| BitcoinBlockHeightChange::get().1)
	}

	fn unwatch_utxo(_lock_id: BitcoinLockId, utxo_ref: &UtxoRef) {
		let _ = utxo_ref;
	}

	fn watch_for_utxo(
		lock_id: BitcoinLockId,
		script_pubkey: BitcoinCosignScriptPubkey,
	) -> Result<(), DispatchError> {
		WatchedUtxosById::mutate(|watched_utxos| {
			watched_utxos.insert(lock_id, script_pubkey);
		});
		Ok(())
	}

	fn unwatch(lock_id: BitcoinLockId) {
		WatchedUtxosById::mutate(|watched_utxos| {
			watched_utxos.remove(&lock_id);
		});
	}
}

impl pallet_bitcoin_utxos::Config for Test {
	type WeightInfo = ();
	type MaxUtxosPerLock = ConstU32<10>;
	type EventHandler = BitcoinLocks;
	type MinimumSatoshisPerUtxo = MinimumLockSatoshis;
}

pub(crate) fn set_bitcoin_height(height: BitcoinHeight) {
	BitcoinBlockHeightChange::set((height, height));
}

impl pallet_bitcoin_locks::Config for Test {
	type WeightInfo = ();
	type Balance = Balance;
	type FissionsProvider = BitcoinFissions;
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
	type SecuritizationHoldBlocks = SecuritizationHoldBlocks;
	type MaxUtxosPerLock = ConstU32<10>;
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

impl pallet_bitcoin_fissions::Config for Test {
	type WeightInfo = ();
	type Balance = Balance;
	type LockProvider = BitcoinLocks;
	type Minting = Mint;
	type OperationalAccountsHook = MockOperationalAccounts;
	type Currency = Balances;
	type MaxFissionsPerLock = ConstU32<10>;
	type MinimumRatchetPercent = MinimumRatchetPercent;
}

impl pallet_mint::Config for Test {
	type WeightInfo = ();
	type Currency = Balances;
	type Balance = Balance;
	type MaxPendingMintsPerUtxo = MaxPendingMintsPerUtxo;
	type MaxPendingMintPayoutWindowSize = MaxPendingMintPayoutWindowSize;
	type PriceProvider = StaticPriceProvider;
	type BlockRewardAccountsProvider = StaticBlockRewardAccountsProvider;
	type MaxMintHistoryToMaintain = ConstU32<10>;
	type MaxPossibleMiners = ConstU32<100>;
	type MiningFrameProvider = StaticMiningFrameProvider;
	type BitcoinMintPayoutPercentPerFrame = BitcoinMintPayoutPercentPerFrame;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> TestState {
	FailReturnSecuritization::set(false);
	AccountBitcoinChanges::set(Vec::new());
	DefaultVault::set(Vault {
		operator_account_id: 1,
		delegate_account_id: None,
		securitization: 200_000_000_000,
		securitization_target: 200_000_000_000,
		securitization_locked: 0,
		flexible_securitization_locked: 0,
		reserved_securitization_space: 0,
		total_satoshis: 0,
		securitized_satoshis: 0,
		ratio_adjusted_satoshis: 0,
		flexible_ratio_adjusted_satoshis: 0,
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
