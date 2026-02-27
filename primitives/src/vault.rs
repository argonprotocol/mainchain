use alloc::collections::BTreeSet;
use codec::{Codec, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::iter::Sum;
use frame_support::PalletError;
use polkadot_sdk::{sp_core::ConstU32, sp_runtime::BoundedBTreeMap, *};
use scale_info::TypeInfo;
use sp_arithmetic::{FixedPointNumber, FixedU128, Permill};
use sp_debug_derive::RuntimeDebug;
use sp_runtime::traits::AtLeast32BitUnsigned;

use crate::{
	VaultId,
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinXPub, CompressedBitcoinPubkey, Satoshis,
		UtxoId, get_rounded_up_bitcoin_day_height,
	},
	ensure,
	prelude::FrameId,
	tick::Tick,
};

pub trait MiningBidPoolProvider {
	type Balance: Codec;
	type AccountId: Codec;

	/// Transfer funds to the bid pool and hold
	fn get_bid_pool_account() -> Self::AccountId;
}

#[derive(Clone, Debug, PartialEq)]
pub struct VaultTreasuryFrameEarnings<Balance, AccountId> {
	pub vault_id: VaultId,
	pub vault_operator_account_id: AccountId,
	pub frame_id: FrameId,
	/// Frame earnings for all contributors
	pub earnings: Balance,
	/// Contributed capital by all contributors
	pub capital_contributed: Balance,
	/// Vault earnings from the frame
	pub earnings_for_vault: Balance,
	/// Contributed capital by the vault
	pub capital_contributed_by_vault: Balance,
}

pub trait TreasuryVaultProvider {
	type Balance: Codec;
	type AccountId: Codec;

	/// Get the number of securitized satoshis tracked by the vault.
	fn get_securitized_satoshis(vault_id: VaultId) -> Satoshis;

	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId>;
	/// Gets the percent of profit sharing
	fn get_vault_profit_sharing_percent(vault_id: VaultId) -> Option<Permill>;

	/// Ensure a vault is open
	fn is_vault_open(vault_id: VaultId) -> bool;

	/// Records the earnings for a vault frame
	fn record_vault_frame_earnings(
		source_account_id: &Self::AccountId,
		profit: VaultTreasuryFrameEarnings<Self::Balance, Self::AccountId>,
	);
}

pub struct LockExtension<Balance> {
	pub extended_expiration_funds: BoundedBTreeMap<BitcoinHeight, Balance, ConstU32<366>>,
	pub lock_expiration: BitcoinHeight,
}

impl<Balance: Codec + MaxEncodedLen> LockExtension<Balance> {
	pub fn new(lock_expiration: BitcoinHeight) -> Self {
		Self { extended_expiration_funds: Default::default(), lock_expiration }
	}

	pub fn expiration_day(&self) -> BitcoinHeight {
		get_rounded_up_bitcoin_day_height(self.lock_expiration)
	}

	pub fn len(&self) -> usize {
		self.extended_expiration_funds.len()
	}

	pub fn is_empty(&self) -> bool {
		self.extended_expiration_funds.is_empty()
	}

	pub fn contains_key(&self, key: &BitcoinHeight) -> bool {
		self.extended_expiration_funds.contains_key(key)
	}

	pub fn get(&self, key: &BitcoinHeight) -> Option<&Balance> {
		self.extended_expiration_funds.get(key)
	}
}

pub struct Securitization<Balance> {
	pub collateral_required: Balance,
	pub securitization_ratio: FixedU128,
	pub liquidity_promised: Balance,
}

impl<Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned>
	Securitization<Balance>
{
	pub fn new(liquidity_promised: Balance, securitization_ratio: FixedU128) -> Self {
		Self {
			collateral_required: securitization_ratio.saturating_mul_int(liquidity_promised),
			securitization_ratio,
			liquidity_promised,
		}
	}
}

pub trait BitcoinVaultProvider {
	type Balance: Codec + Copy + TypeInfo + MaxEncodedLen + Default + AtLeast32BitUnsigned;
	type AccountId: Codec;

	fn is_owner(vault_id: VaultId, account_id: &Self::AccountId) -> bool;
	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId>;
	fn get_vault_id(account_id: &Self::AccountId) -> Option<VaultId>;

	/// Get the securitization ratio offered by this vault
	fn get_securitization_ratio(vault_id: VaultId) -> Result<FixedU128, VaultError>;

	/// Add satoshis from a funded lock to this vault's satoshi totals.
	fn add_securitized_satoshis(
		vault_id: VaultId,
		satoshis: Satoshis,
		securitization_ratio: FixedU128,
	) -> Result<(), VaultError>;

	/// Reduce satoshis from this vault's satoshi totals.
	fn reduce_securitized_satoshis(
		vault_id: VaultId,
		satoshis: Satoshis,
		securitization_ratio: FixedU128,
	) -> Result<(), VaultError>;

	/// Holds the given "securitization" from the vault. Returns the fee amount
	fn lock(
		vault_id: VaultId,
		locker: &Self::AccountId,
		securitization: &Securitization<Self::Balance>,
		satoshis: Satoshis,
		extension: Option<(FixedU128, &mut LockExtension<Self::Balance>)>,
		has_fee_coupon: bool,
	) -> Result<Self::Balance, VaultError>;

	/// Release the lock and move into holding, eligible for relock
	fn schedule_for_release(
		vault_id: VaultId,
		securitization: &Securitization<Self::Balance>,
		satoshis: Satoshis,
		lock_extension: &LockExtension<Self::Balance>,
	) -> Result<(), VaultError>;

	/// The lock is complete and remaining funds can be returned to the vault
	fn cancel(
		vault_id: VaultId,
		securitization: &Securitization<Self::Balance>,
	) -> Result<(), VaultError>;

	/// Burn the funds from the vault. This will be called if a vault moves a bitcoin utxo outside
	/// the system. It is assumed that the vault is in cahoots with the beneficiary.
	///
	/// Returns the amount of argons that were burned
	fn burn(
		vault_id: VaultId,
		securitization: &Securitization<Self::Balance>,
		market_rate: Self::Balance,
		lock_extension: &LockExtension<Self::Balance>,
	) -> Result<Self::Balance, VaultError>;

	/// Recoup funds from the vault. This will be called if a vault has performed an illegal
	/// activity, like not moving cosigned UTXOs in the appropriate timeframe.
	///
	/// The recouped funds is up to the market rate, but capped at securitization rate of the
	/// vault.
	///
	/// Returns the amount sent to the beneficiary.
	fn compensate_lost_bitcoin(
		vault_id: VaultId,
		beneficiary: &Self::AccountId,
		securitization: &Securitization<Self::Balance>,
		market_rate: Self::Balance,
		lock_extension: &LockExtension<Self::Balance>,
	) -> Result<Self::Balance, VaultError>;

	fn create_utxo_script_pubkey(
		vault_id: VaultId,
		owner_pubkey: CompressedBitcoinPubkey,
		vault_claim_height: BitcoinHeight,
		open_claim_height: BitcoinHeight,
		current_height: BitcoinHeight,
	) -> Result<(BitcoinXPub, BitcoinXPub, BitcoinCosignScriptPubkey), VaultError>;

	/// Argons no longer in a "pending state" - eg, funded bitcoin or canceled
	fn remove_pending(
		vault_id: VaultId,
		securitization: &Securitization<Self::Balance>,
	) -> Result<(), VaultError>;

	/// Track a pending cosign for a UTXO.
	fn update_pending_cosign_list(
		vault_id: VaultId,
		utxo_id: UtxoId,
		should_remove: bool,
	) -> Result<(), VaultError>;

	/// Track an orphaned cosign request for a UTXO.
	fn update_orphan_cosign_list(
		vault_id: VaultId,
		utxo_id: UtxoId,
		account_id: &Self::AccountId,
		should_remove: bool,
	) -> Result<(), VaultError>;
}

#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo, PalletError,
)]
pub enum VaultError {
	VaultClosed,
	AccountWouldBeBelowMinimum,
	InsufficientFunds,
	InsufficientVaultFunds,
	HoldUnexpectedlyModified,
	/// The hold could not be removed - it must have been modified
	UnrecoverableHold,
	VaultNotFound,
	/// No Vault public keys are available
	NoVaultBitcoinPubkeysAvailable,
	/// Unable to generate a new vault public key
	UnableToGenerateVaultBitcoinPubkey,
	/// Scripting for a bitcoin UTXO failed
	InvalidBitcoinScript,
	/// An internal processing error occurred that is too technical to be useful to the user
	InternalError,
	/// This vault is not yet active
	VaultNotYetActive,
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct Vault<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	/// The account assigned to operate this vault
	pub operator_account_id: AccountId,
	/// The securitization in the vault
	#[codec(compact)]
	pub securitization: Balance,
	/// The target securitization to have in the vault (in case of reducing)
	#[codec(compact)]
	pub securitization_target: Balance,
	/// The securitization locked for bitcoin (at the ratio given)
	#[codec(compact)]
	pub securitization_locked: Balance,
	/// Securitization pending bitcoin funding confirmation (this is "out of" the
	/// securitization_locked, not in addition to)
	#[codec(compact)]
	pub securitization_pending_activation: Balance,
	/// The number of locked satoshis currently tracked by this vault.
	#[codec(compact)]
	pub locked_satoshis: Satoshis,
	/// The number of securitized satoshis (`sats * securitization ratio`).
	#[codec(compact)]
	pub securitized_satoshis: Satoshis,
	/// Securitization that will be released at the given block height (NOTE: these are grouped by
	/// next day of bitcoin blocks). This securitization can be relocked
	pub securitization_release_schedule: BoundedBTreeMap<BitcoinHeight, Balance, ConstU32<366>>,
	/// The securitization ratio of "total securitization" to "available for locked bitcoin"
	#[codec(compact)]
	pub securitization_ratio: FixedU128,
	/// If the vault is closed, no new bitcoin locks can be issued
	pub is_closed: bool,
	/// The terms for locked bitcoin
	pub terms: VaultTerms<Balance>,
	/// The terms that are pending to be applied to this vault at the given tick
	pub pending_terms: Option<(Tick, VaultTerms<Balance>)>,
	/// A tick at which this vault is active
	#[codec(compact)]
	pub opened_tick: Tick,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct VaultTerms<Balance>
where
	Balance: Codec + MaxEncodedLen + Clone + TypeInfo + PartialEq + Eq,
{
	/// The annual percent rate per argon vaulted for bitcoin locks
	#[codec(compact)]
	pub bitcoin_annual_percent_rate: FixedU128,
	/// The base fee for a bitcoin lock
	#[codec(compact)]
	pub bitcoin_base_fee: Balance,
	/// The percent of mining bonds taken by the vault
	#[codec(compact)]
	pub treasury_profit_sharing: Permill,
}

pub struct BurnResult<Balance> {
	pub burned_amount: Balance,
	pub held_for_release: Balance,
	pub release_heights: BTreeSet<BitcoinHeight>,
}

impl<
	AccountId: Codec,
	Balance: Codec
		+ Copy
		+ MaxEncodedLen
		+ Default
		+ AtLeast32BitUnsigned
		+ MaxEncodedLen
		+ Clone
		+ TypeInfo
		+ core::fmt::Debug
		+ PartialEq
		+ Eq
		+ Sum,
> Vault<AccountId, Balance>
{
	#[cfg(debug_assertions)]
	#[inline(always)]
	pub fn debug_assert_invariants_at(&self, where_: &'static str) {
		let relock = self.get_relock_capacity();
		debug_assert!(
			self.securitization_locked <= self.securitization,
			"[{where_}] invariant failed: securitization_locked ({:?}) > securitization ({:?})",
			self.securitization_locked,
			self.securitization
		);
		debug_assert!(
			relock <= self.securitization,
			"[{where_}] invariant failed: relock_capacity ({:?}) > securitization ({:?})",
			relock,
			self.securitization
		);
		debug_assert!(
			self.securitization_locked.saturating_add(relock) <= self.securitization,
			"[{where_}] invariant failed: locked+relock ({:?}) > securitization ({:?}); locked={:?} relock={:?} scheduled_keys={:?}",
			self.securitization_locked.saturating_add(relock),
			self.securitization,
			self.securitization_locked,
			relock,
			self.securitization_release_schedule.keys().collect::<alloc::vec::Vec<_>>()
		);
	}

	#[cfg(debug_assertions)]
	#[inline(always)]
	pub fn debug_assert_invariants(&self) {
		self.debug_assert_invariants_at("Vault");
	}

	#[cfg(not(debug_assertions))]
	#[inline(always)]
	pub fn debug_assert_invariants_at(&self, _where_: &'static str) {
		// no-op in release builds
	}

	#[cfg(not(debug_assertions))]
	#[inline(always)]
	pub fn debug_assert_invariants(&self) {
		// no-op in release builds
	}

	pub fn get_activated_securitization(&self) -> Balance {
		self.securitization_locked
			.saturating_sub(self.securitization_pending_activation)
	}

	pub fn burn(
		&mut self,
		securitization: &Securitization<Balance>,
		market_rate: Balance,
		lock_extension: &LockExtension<Balance>,
	) -> Result<BurnResult<Balance>, VaultError> {
		let burn_amount = securitization.collateral_required.min(market_rate);
		if burn_amount > self.securitization || burn_amount > self.securitization_locked {
			return Err(VaultError::InsufficientVaultFunds);
		}
		self.securitization.saturating_reduce(burn_amount);
		self.securitization_locked.saturating_reduce(burn_amount);

		let amount_to_future_release =
			securitization.collateral_required.saturating_sub(burn_amount);

		let release_height = self.schedule_for_release(
			&Securitization::new(amount_to_future_release, securitization.securitization_ratio),
			lock_extension,
		)?;

		self.debug_assert_invariants_at("burn:after");

		Ok(BurnResult {
			burned_amount: burn_amount,
			held_for_release: amount_to_future_release,
			release_heights: release_height,
		})
	}

	/// Lock funds for a full term (assume underlying bitcoin is pending)
	pub fn lock(&mut self, securitization: &Securitization<Balance>) -> Result<(), VaultError> {
		ensure!(
			securitization.collateral_required <= self.available_for_lock(),
			VaultError::InsufficientVaultFunds
		);

		let remaining = self.use_relockable_securitization(securitization, None);
		self.securitization_locked.saturating_accrue(remaining);
		self.securitization_pending_activation
			.saturating_accrue(securitization.collateral_required);
		self.debug_assert_invariants_at("lock:after");

		Ok(())
	}

	/// Extends an existing lock for a given amount of securitization. This will prioritize using:
	/// 1. relockable securitization scheduled for release within the max expiration.
	/// 2. available unused + lock-free securitization
	/// 3. any remaining scheduled for release (recording extensions for heights beyond the max
	///    expiration)
	///
	/// Modifies lock extensions for the securitization locked beyond the max expiration
	pub fn extend_lock(
		&mut self,
		securitization: &Securitization<Balance>,
		lock_extension: &mut LockExtension<Balance>,
	) -> Result<(), VaultError> {
		ensure!(
			securitization.collateral_required <= self.available_for_lock(),
			VaultError::InsufficientVaultFunds
		);

		// 1. Use the relockable argons within the max expiration
		let mut remaining = self
			.use_relockable_securitization(securitization, Some(lock_extension.lock_expiration));

		// 2. Use any available *unlocked* securitization (exclude anything still scheduled for
		//    release).
		// `available_securitization()` includes scheduled-for-release amounts, but step (2) must
		// not consume those, because step (3) handles scheduled funds beyond the max expiration.
		let uninhibited_securitization = self.uninhibited_securitization();
		let amount_to_lock = remaining.min(uninhibited_securitization);
		self.securitization_locked.saturating_accrue(amount_to_lock);
		remaining.saturating_reduce(amount_to_lock);

		// 3. Use any remaining scheduled for release beyond the max expiration
		if !remaining.is_zero() {
			let max_expiration = lock_extension.expiration_day();
			for (height, release_amount) in self.securitization_release_schedule.iter_mut() {
				let amount_to_use = remaining.min(*release_amount);
				release_amount.saturating_reduce(amount_to_use);
				remaining.saturating_reduce(amount_to_use);
				self.securitization_locked.saturating_accrue(amount_to_use);

				if *height > max_expiration {
					Self::increment_scheduled_expiration(
						&mut lock_extension.extended_expiration_funds,
						amount_to_use,
						height,
					)?;
				}
				if remaining.is_zero() {
					break;
				}
			}
			if !remaining.is_zero() {
				return Err(VaultError::InsufficientVaultFunds);
			}
			self.securitization_release_schedule.retain(|_, v| !v.is_zero());
		}

		self.debug_assert_invariants_at("extend_lock:after");
		Ok(())
	}

	pub fn sweep_released(&mut self, block_height: BitcoinHeight) -> Balance {
		let mut released_securitization = Balance::zero();
		self.securitization_release_schedule.retain(|height, released_amount| {
			if *height <= block_height {
				released_securitization.saturating_accrue(*released_amount);
				return false;
			}
			true
		});

		released_securitization
	}

	pub fn did_confirm_pending_activation(&mut self, securitization: &Securitization<Balance>) {
		self.securitization_pending_activation
			.saturating_reduce(securitization.collateral_required);
	}

	pub fn release_lock(&mut self, securitization: &Securitization<Balance>) {
		self.securitization_locked.saturating_reduce(securitization.collateral_required);
	}

	pub fn schedule_for_release(
		&mut self,
		securitization: &Securitization<Balance>,
		lock_extension: &LockExtension<Balance>,
	) -> Result<BTreeSet<BitcoinHeight>, VaultError> {
		let mut release_heights = BTreeSet::new();

		// Reschedule the amounts to be released by any delayed funds first, then any remaining at
		// the expiration of the lock.
		// Under the "count once" model, scheduled-for-release funds are no longer counted as
		// locked, so we must ensure we actually have enough locked collateral to move into the
		// schedule.
		ensure!(
			securitization.collateral_required <= self.securitization_locked,
			VaultError::InsufficientVaultFunds
		);
		self.securitization_locked.saturating_reduce(securitization.collateral_required);
		let mut amount_in_lock_extension = Balance::zero();
		for (height, amount) in &lock_extension.extended_expiration_funds {
			release_heights.insert(*height);
			amount_in_lock_extension.saturating_accrue(*amount);
			Self::increment_scheduled_expiration(
				&mut self.securitization_release_schedule,
				*amount,
				height,
			)?;
		}

		let remaining = securitization.collateral_required.saturating_sub(amount_in_lock_extension);
		if remaining > Balance::zero() {
			let expiration = lock_extension.expiration_day();
			release_heights.insert(expiration);
			Self::increment_scheduled_expiration(
				&mut self.securitization_release_schedule,
				remaining,
				&expiration,
			)?;
		}

		self.debug_assert_invariants_at("schedule_for_release:after");
		Ok(release_heights)
	}

	pub fn create_securitization_bundle(
		&self,
		liquidity_promised: Balance,
	) -> Securitization<Balance> {
		Securitization::new(liquidity_promised, self.securitization_ratio)
	}

	pub fn securitized_amount(&self, amount: Balance) -> Balance {
		self.securitization_ratio.saturating_mul_int(amount)
	}

	/// The amount of securitization that can be re-locked (with expiration extended out)
	pub fn get_relock_capacity(&self) -> Balance {
		self.securitization_release_schedule.values().copied().sum()
	}

	pub fn available_for_lock(&self) -> Balance {
		self.securitization.saturating_sub(self.securitization_locked)
	}

	pub fn uninhibited_securitization(&self) -> Balance {
		self.securitization
			.saturating_sub(self.securitization_locked)
			.saturating_sub(self.get_relock_capacity())
	}

	fn increment_scheduled_expiration(
		release_schedule: &mut BoundedBTreeMap<BitcoinHeight, Balance, ConstU32<366>>,
		amount_to_use: Balance,
		height: &BitcoinHeight,
	) -> Result<(), VaultError> {
		if !release_schedule.contains_key(height) {
			release_schedule
				.try_insert(*height, Balance::zero())
				.map_err(|_| VaultError::InternalError)?;
		}

		let Some(x) = release_schedule.get_mut(height) else {
			return Err(VaultError::InternalError);
		};
		x.saturating_accrue(amount_to_use);
		Ok(())
	}

	fn use_relockable_securitization(
		&mut self,
		securitization: &Securitization<Balance>,
		max_expiration: Option<BitcoinHeight>,
	) -> Balance {
		let mut remaining = securitization.collateral_required;
		let max_expiration = max_expiration.map(get_rounded_up_bitcoin_day_height);
		for (height, releasing_amount) in self.securitization_release_schedule.iter_mut() {
			if let Some(max_expiration) = max_expiration {
				if *height > max_expiration {
					continue;
				}
			}
			let amount_to_use = remaining.min(*releasing_amount);
			releasing_amount.saturating_reduce(amount_to_use);
			remaining.saturating_reduce(amount_to_use);
			self.securitization_locked.saturating_accrue(amount_to_use);
			if remaining.is_zero() {
				break;
			}
		}
		self.securitization_release_schedule.retain(|_, v| !v.is_zero());
		remaining
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::{
		Balance,
		prelude::sp_arithmetic::{FixedU128, traits::One},
	};
	use polkadot_sdk::frame_support::assert_err;

	#[test]
	fn test_locking_and_releasing_funds() {
		let mut vault = default_vault(100, 1.0);
		vault.lock(&securitization(50)).unwrap();
		assert_eq!(vault.securitization_locked, 50);
		assert_eq!(vault.securitization_pending_activation, 50);

		vault.lock(&securitization(30)).unwrap();
		assert_eq!(vault.securitization_locked, 80);
		assert_eq!(vault.securitization_pending_activation, 80);

		vault.release_lock(&securitization(20));
		assert_eq!(vault.securitization_locked, 60);
	}

	#[test]
	fn calculates_securitization() {
		let mut vault = default_vault(100, 2.0);
		assert_eq!(vault.get_activated_securitization(), 0);
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(vault.available_for_lock(), 100);
		assert_eq!(vault.securitized_amount(50), 100);

		assert_err!(
			vault.lock(&vault.create_securitization_bundle(60)),
			VaultError::InsufficientVaultFunds
		);

		vault.lock(&vault.create_securitization_bundle(50)).unwrap();
		assert_eq!(vault.get_activated_securitization(), 0);
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(vault.available_for_lock(), 0);

		vault.securitization_pending_activation = 0;
		assert_eq!(vault.get_activated_securitization(), 100);

		let lock_extensions = &mut LockExtension::new(106);
		vault
			.schedule_for_release(&vault.create_securitization_bundle(50), lock_extensions)
			.unwrap();
		assert_eq!(vault.get_relock_capacity(), 100);
		assert_eq!(vault.get_activated_securitization(), 0);
	}

	#[test]
	fn can_burn() {
		let mut vault = default_vault(100, 1.0);

		vault.lock(&securitization(100)).unwrap();
		assert_eq!(vault.securitized_amount(50), 50);
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(vault.available_for_lock(), 0);
		assert_eq!(vault.securitization_locked, 100);
		vault.securitization_pending_activation = 0;

		let lock_extensions = &mut LockExtension::new(365);
		let burn_result = vault.burn(&securitization(100), 50, lock_extensions).unwrap();
		assert_eq!(burn_result.burned_amount, 50);
		assert_eq!(burn_result.held_for_release, 50);
		assert_eq!(burn_result.release_heights.len(), 1);
		assert_eq!(vault.securitization_locked, 0);
		assert_eq!(vault.securitization_release_schedule.len(), 1);
		assert_eq!(vault.securitization_release_schedule.get(&432).unwrap(), &50);
		assert_eq!(vault.securitization, 50);
		assert_eq!(vault.available_for_lock(), 50);
	}

	#[test]
	fn handles_schedule_for_release() {
		let mut vault = default_vault(500, 1.0);
		vault.lock(&securitization(100)).unwrap();
		vault.did_confirm_pending_activation(&securitization(100));

		let lock_extensions = &mut LockExtension::new(100);
		let release_heights =
			vault.schedule_for_release(&securitization(100), lock_extensions).unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.securitization_release_schedule.len(), 1);
		assert_eq!(vault.securitization_release_schedule.get(&144).unwrap(), &100);
		assert_eq!(vault.securitization_locked, 0);
		assert_eq!(vault.available_for_lock(), 500);

		let lock_extensions = &mut LockExtension::new(100);
		vault.extend_lock(&securitization(100), lock_extensions).unwrap();
		assert_eq!(vault.securitization_release_schedule.len(), 0);
		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.available_for_lock(), 400);
		assert_eq!(vault.get_relock_capacity(), 0);

		vault.lock(&securitization(100)).unwrap();
		assert_eq!(vault.securitization_locked, 200);
		assert_eq!(vault.available_for_lock(), 300);
		assert_eq!(vault.get_relock_capacity(), 0);
		vault.did_confirm_pending_activation(&securitization(100));

		// schedule multiple releases
		let lock_extensions = &mut LockExtension::new(250);
		let release_heights =
			vault.schedule_for_release(&securitization(50), lock_extensions).unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.securitization_locked, 150);
		assert_eq!(vault.securitization_release_schedule.len(), 1);
		assert_eq!(vault.securitization_release_schedule.get(&288).unwrap(), &50);
		assert_eq!(vault.get_relock_capacity(), 50);

		let lock_extensions = &mut LockExtension::new(255);
		let release_heights =
			vault.schedule_for_release(&securitization(25), lock_extensions).unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.securitization_locked, 125);
		assert_eq!(vault.securitization_release_schedule.len(), 1);
		assert_eq!(vault.securitization_release_schedule.get(&288).unwrap(), &75);
		assert_eq!(vault.get_relock_capacity(), 75);

		let lock_extensions = &mut LockExtension::new(300);
		let release_heights =
			vault.schedule_for_release(&securitization(25), lock_extensions).unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.securitization_release_schedule.len(), 2);
		assert_eq!(
			vault.securitization_release_schedule.get(&288).unwrap(),
			&75,
			"shouldn't need to touch this"
		);
		assert_eq!(vault.securitization_release_schedule.get(&432).unwrap(), &25);
		assert_eq!(vault.get_relock_capacity(), 100);

		// if the expiration is within the other expirations, it will prefer the securitization
		// already scheduled for release
		let lock_extensions = &mut LockExtension::new(143);
		vault.extend_lock(&securitization(25), lock_extensions).unwrap();
		assert_eq!(lock_extensions.len(), 0);
		assert_eq!(vault.securitization_locked, 125);
		assert_eq!(vault.securitization_release_schedule.len(), 2);
		assert_eq!(
			vault.securitization_release_schedule.get(&288).unwrap(),
			&75,
			"shouldn't need to touch this"
		);
		assert_eq!(vault.securitization_release_schedule.get(&432).unwrap(), &25);
		assert_eq!(
			vault.securitization_release_schedule.keys().collect::<Vec<_>>(),
			vec![&288, &432]
		);
		assert_eq!(vault.get_relock_capacity(), 100);

		// extend the lock beyond the available unlocked securitization, using scheduled-for-release
		// funds
		assert_eq!(vault.available_for_lock(), 375);
		assert_eq!(vault.securitization_locked, 125);
		let lock_extensions = &mut LockExtension::new(143);
		vault.extend_lock(&securitization(370), lock_extensions).unwrap();
		assert_eq!(lock_extensions.len(), 2);
		assert_eq!(lock_extensions.get(&288).unwrap(), &75);
		assert_eq!(lock_extensions.get(&432).unwrap(), &20);
		assert_eq!(vault.get_relock_capacity(), 5);
		assert_eq!(vault.securitization_locked, 495);
		assert_eq!(vault.securitization_release_schedule.len(), 1);
		assert_eq!(vault.securitization_release_schedule.get(&432).unwrap(), &5);

		// now return the 370
		let result = vault.schedule_for_release(&securitization(370), lock_extensions).unwrap();
		assert_eq!(result.len(), 3);
		assert_eq!(result.iter().collect::<Vec<_>>(), vec![&144, &288, &432]);
		assert_eq!(vault.get_relock_capacity(), 375);
		assert_eq!(vault.securitization_locked, 125);
		assert_eq!(vault.securitization_release_schedule.len(), 3);
		assert_eq!(vault.securitization_release_schedule.get(&144).unwrap(), &275);
		assert_eq!(vault.securitization_release_schedule.get(&288).unwrap(), &75);
		assert_eq!(vault.securitization_release_schedule.get(&432).unwrap(), &25);
	}

	fn securitization(amount: Balance) -> Securitization<Balance> {
		Securitization::new(amount, FixedU128::one())
	}

	fn default_vault(securitization: Balance, ratio: f64) -> Vault<u64, Balance> {
		Vault::<u64, Balance> {
			operator_account_id: 0,
			securitization,
			securitization_target: securitization,
			securitization_locked: 0,
			securitization_pending_activation: 0,
			locked_satoshis: 0,
			securitized_satoshis: 0,
			securitization_release_schedule: Default::default(),
			securitization_ratio: FixedU128::from_float(ratio),
			is_closed: false,
			terms: VaultTerms {
				bitcoin_annual_percent_rate: 0.into(),
				bitcoin_base_fee: 0,
				treasury_profit_sharing: Default::default(),
			},
			pending_terms: None,
			opened_tick: 0,
		}
	}
}
