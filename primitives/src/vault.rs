use alloc::{collections::BTreeSet, vec};
use codec::{Codec, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
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
		get_rounded_up_bitcoin_day_height,
	},
	ensure,
	tick::Tick,
};

pub trait MiningBidPoolProvider {
	type Balance: Codec;
	type AccountId: Codec;

	/// Transfer funds to the bid pool and hold
	fn get_bid_pool_account() -> Self::AccountId;
}

pub trait LiquidityPoolVaultProvider {
	type Balance: Codec;
	type AccountId: Codec;

	/// Get the total amount of securitization activated for the vault
	fn get_activated_securitization(vault_id: VaultId) -> Self::Balance;

	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId>;
	/// Gets the percent of profit sharing
	fn get_vault_profit_sharing_percent(vault_id: VaultId) -> Option<Permill>;

	/// Ensure a vault is open
	fn is_vault_open(vault_id: VaultId) -> bool;
}

pub struct LockExtension<Balance> {
	pub extended_expiration_funds: BoundedBTreeMap<BitcoinHeight, Balance, ConstU32<366>>,
	pub lock_expiration: BitcoinHeight,
}

impl<Balance: Codec + MaxEncodedLen> LockExtension<Balance> {
	pub fn new(lock_expiration: BitcoinHeight) -> Self {
		Self { extended_expiration_funds: Default::default(), lock_expiration }
	}

	pub fn day(&self) -> BitcoinHeight {
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

pub trait BitcoinVaultProvider {
	type Balance: Codec + Copy + TypeInfo + MaxEncodedLen + Default + AtLeast32BitUnsigned;
	type AccountId: Codec;

	fn is_owner(vault_id: VaultId, account_id: &Self::AccountId) -> bool;

	/// Holds the given "lock_price" from the vault. Returns the fee amount
	fn lock(
		vault_id: VaultId,
		locker: &Self::AccountId,
		locked_argons: Self::Balance,
		satoshis: Satoshis,
		extension: Option<(FixedU128, &mut LockExtension<Self::Balance>)>,
	) -> Result<Self::Balance, VaultError>;

	/// Release the lock and move into holding, eligible for relock
	fn schedule_for_release(
		vault_id: VaultId,
		locked_argons: Self::Balance,
		satoshis: Satoshis,
		lock_extension: &LockExtension<Self::Balance>,
	) -> Result<(), VaultError>;

	/// The lock is complete and remaining funds can be returned to the vault
	fn cancel(vault_id: VaultId, amount: Self::Balance) -> Result<(), VaultError>;

	/// Burn the funds from the vault. This will be called if a vault moves a bitcoin utxo outside
	/// the system. It is assumed that the vault is in cahoots with the beneficiary.
	///
	/// Returns the amount of argons that were burned
	fn burn(
		vault_id: VaultId,
		lock_amount: Self::Balance,
		redemption_rate: Self::Balance,
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
		lock_amount: Self::Balance,
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

	/// Argons no longer in a "pending state" - eg, verified bitcoin or canceled
	fn remove_pending(vault_id: VaultId, amount: Self::Balance) -> Result<(), VaultError>;
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
	/// The argons locked for bitcoin
	#[codec(compact)]
	pub argons_locked: Balance,
	/// Argons for bitcoin pending verification (this is "out of" the bitcoin_locked, not in
	/// addition to)
	#[codec(compact)]
	pub argons_pending_activation: Balance,
	/// Argons that will be released at the given block height (NOTE: these are grouped by next day
	/// of bitcoin blocks). These argons can be re-locked
	pub argons_scheduled_for_release: BoundedBTreeMap<BitcoinHeight, Balance, ConstU32<366>>,
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
	pub liquidity_pool_profit_sharing: Permill,
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
		+ Eq,
> Vault<AccountId, Balance>
{
	pub fn get_activated_securitization(&self) -> Balance {
		let activated_securitization =
			self.argons_locked.saturating_sub(self.argons_pending_activation);
		// you can increase the max allocation up to an additional 2x over the locked bitcoins
		let ratio = self.securitization_ratio.min(FixedU128::from_u32(2));
		ratio.saturating_mul_int(activated_securitization).min(self.securitization)
	}

	pub fn get_minimum_securitization_needed(&self) -> Balance {
		self.securitization_ratio.saturating_mul_int(self.argons_locked)
	}

	pub fn get_recovery_securitization(&self) -> Balance {
		let reserved = FixedU128::from_u32(1).div(self.securitization_ratio);
		self.securitization
			.saturating_sub(reserved.saturating_mul_int(self.securitization))
	}

	pub fn burn(
		&mut self,
		amount: Balance,
		market_rate: Balance,
		lock_extension: &LockExtension<Balance>,
	) -> Result<BurnResult<Balance>, VaultError> {
		let burn_amount = self.securitized_amount(amount).min(market_rate);

		self.destroy_securitization(burn_amount)?;

		let lock_to_destroy = amount.min(market_rate);
		let amount_to_future_release = amount.saturating_sub(lock_to_destroy);

		let release_height = self.schedule_for_release(amount_to_future_release, lock_extension)?;

		self.argons_locked = self
			.argons_locked
			.checked_sub(&lock_to_destroy)
			.ok_or(VaultError::InsufficientVaultFunds)?;
		Ok(BurnResult {
			burned_amount: burn_amount,
			held_for_release: amount_to_future_release,
			release_heights: release_height,
		})
	}

	/// Lock funds for a full term (assume underlying bitcoin is pending)
	pub fn lock(&mut self, amount: Balance) -> Result<(), VaultError> {
		ensure!(amount <= self.available_for_lock(), VaultError::InsufficientVaultFunds);

		let remaining = self.use_relockable_argons(amount, None);
		self.argons_locked.saturating_accrue(remaining);
		self.argons_pending_activation.saturating_accrue(amount);
		Ok(())
	}

	/// Extends an existing lock for a given number of argons. This will prioritize using the
	/// relockable argons that are scheduled for release within the max expiration. If there are
	/// remaining argons, they will be used from "free" to unlock argons. Lastly, if there are still
	/// remaining argons, they will be used from argons scheduled for release and the release
	/// extensions will be returned.
	///
	/// Modifies lock extensions for the argons locked beyond the max expiration
	pub fn extend_lock(
		&mut self,
		amount: Balance,
		lock_extension: &mut LockExtension<Balance>,
	) -> Result<(), VaultError> {
		ensure!(amount <= self.available_for_lock(), VaultError::InsufficientVaultFunds);

		// 1. Use the relockable argons within the max expiration
		let mut remaining =
			self.use_relockable_argons(amount, Some(lock_extension.lock_expiration));

		let max_lockable = self
			.securitization
			.saturating_sub(self.get_recovery_securitization())
			.saturating_sub(self.argons_locked);

		let amount_to_lock = remaining.min(max_lockable);
		self.argons_locked.saturating_accrue(amount_to_lock);
		ensure!(
			self.securitized_amount(self.argons_locked) <= self.securitization,
			VaultError::InsufficientVaultFunds
		);
		remaining.saturating_reduce(amount_to_lock);

		if !remaining.is_zero() {
			let max_expiration = lock_extension.day();
			for (height, release_amount) in self.argons_scheduled_for_release.iter_mut() {
				let amount_to_use = remaining.min(*release_amount);
				release_amount.saturating_reduce(amount_to_use);
				remaining.saturating_reduce(amount_to_use);

				if *height > max_expiration {
					if !lock_extension.extended_expiration_funds.contains_key(height) {
						lock_extension
							.extended_expiration_funds
							.try_insert(*height, Balance::zero())
							.map_err(|_| VaultError::InternalError)?;
					}
					lock_extension
						.extended_expiration_funds
						.get_mut(height)
						.expect("Just inserted this, must exist")
						.saturating_accrue(amount_to_use);
				}
				if remaining.is_zero() {
					break;
				}
			}
			self.argons_scheduled_for_release.retain(|_, v| !v.is_zero());
		}

		Ok(())
	}

	pub fn sweep_released(&mut self, block_height: BitcoinHeight) -> Balance {
		let mut amount = Balance::zero();
		self.argons_scheduled_for_release.retain(|height, released_amount| {
			if *height <= block_height {
				amount.saturating_accrue(*released_amount);
				return false
			}
			true
		});
		self.argons_locked.saturating_reduce(amount);
		amount
	}

	pub fn release_locked_funds(&mut self, amount: Balance) {
		self.argons_locked.saturating_reduce(amount);
	}

	pub fn schedule_for_release(
		&mut self,
		amount: Balance,
		lock_extension: &LockExtension<Balance>,
	) -> Result<BTreeSet<BitcoinHeight>, VaultError> {
		let mut release_heights = BTreeSet::new();

		// reschedule the amounts to be released by any delayed funds first, then any remaining at
		// the expiration of the lock
		let mut to_release = vec![];
		let mut remaining = amount;
		for (height, release_at_height) in lock_extension.extended_expiration_funds.iter() {
			let to_use = remaining.min(*release_at_height);
			remaining.saturating_reduce(to_use);
			to_release.push((*height, to_use));
			if remaining.is_zero() {
				break;
			}
		}
		if remaining > Balance::zero() {
			to_release.push((lock_extension.day(), remaining));
		}

		for (height, amount) in to_release {
			release_heights.insert(height);
			if !self.argons_scheduled_for_release.contains_key(&height) {
				self.argons_scheduled_for_release
					.try_insert(height, Balance::zero())
					.map_err(|_| VaultError::InternalError)?;
			}
			self.argons_scheduled_for_release
				.get_mut(&height)
				.expect("Just inserted this, must exist")
				.saturating_accrue(amount);
		}
		Ok(release_heights)
	}

	pub fn securitized_amount(&self, amount: Balance) -> Balance {
		self.securitization_ratio.saturating_mul_int(amount)
	}

	/// The number of argons that can be re-locked (with expiration extended out)
	pub fn get_relock_capacity(&self) -> Balance {
		let mut total = Balance::zero();
		for (_height, releasing) in self.argons_scheduled_for_release.iter() {
			total.saturating_accrue(*releasing);
		}
		total
	}

	pub fn available_for_lock(&self) -> Balance {
		self.securitization
			.saturating_sub(self.get_recovery_securitization())
			.saturating_sub(self.argons_locked)
			// relocks should be counted as free balance
			.saturating_add(self.get_relock_capacity())
	}

	fn destroy_securitization(&mut self, amount: Balance) -> Result<(), VaultError> {
		ensure!(amount <= self.securitization, VaultError::InsufficientVaultFunds);
		self.securitization.saturating_reduce(amount);
		Ok(())
	}

	fn use_relockable_argons(
		&mut self,
		amount: Balance,
		max_expiration: Option<BitcoinHeight>,
	) -> Balance {
		let mut remaining = amount;
		let max_expiration = max_expiration.map(get_rounded_up_bitcoin_day_height);
		for (height, releasing_amount) in self.argons_scheduled_for_release.iter_mut() {
			if let Some(max_expiration) = max_expiration {
				if *height > max_expiration {
					continue;
				}
			}
			let amount_to_use = remaining.min(*releasing_amount);
			releasing_amount.saturating_reduce(amount_to_use);
			remaining.saturating_reduce(amount_to_use);
			if remaining.is_zero() {
				break;
			}
		}
		self.argons_scheduled_for_release.retain(|_, v| !v.is_zero());
		remaining
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::{Balance, prelude::sp_arithmetic::FixedU128};
	use polkadot_sdk::frame_support::assert_err;

	#[test]
	fn test_locking_and_releasing_funds() {
		let mut vault = default_vault(100, 1.0);
		vault.lock(50).unwrap();
		assert_eq!(vault.argons_locked, 50);
		assert_eq!(vault.argons_pending_activation, 50);

		vault.lock(30).unwrap();
		assert_eq!(vault.argons_locked, 80);
		assert_eq!(vault.argons_pending_activation, 80);

		vault.release_locked_funds(20);
		assert_eq!(vault.argons_locked, 60);
	}

	#[test]
	fn calculates_securitization() {
		let mut vault = default_vault(100, 2.0);
		assert_eq!(vault.get_activated_securitization(), 0);
		assert_eq!(vault.get_minimum_securitization_needed(), 0);
		assert_eq!(vault.get_recovery_securitization(), 50);
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(vault.available_for_lock(), 50);
		assert_eq!(vault.securitized_amount(50), 100);

		assert_err!(vault.lock(60), VaultError::InsufficientVaultFunds);

		vault.lock(50).unwrap();
		assert_eq!(vault.get_activated_securitization(), 0);
		assert_eq!(vault.get_minimum_securitization_needed(), 100);
		assert_eq!(vault.get_recovery_securitization(), 50);
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(vault.available_for_lock(), 0);

		vault.argons_pending_activation = 0;
		assert_eq!(vault.get_activated_securitization(), 100);

		let lock_extensions = &mut LockExtension::new(100);
		vault.schedule_for_release(100, lock_extensions).unwrap();
		assert_eq!(vault.get_relock_capacity(), 100);
		assert_eq!(vault.get_activated_securitization(), 100);
	}

	#[test]
	fn can_burn() {
		let mut vault = default_vault(100, 1.0);

		vault.lock(100).unwrap();

		let lock_extensions = &mut LockExtension::new(100);
		let burn_result = vault.burn(100, 50, lock_extensions).unwrap();
		assert_eq!(burn_result.burned_amount, 50);
		assert_eq!(burn_result.held_for_release, 50);
		assert_eq!(burn_result.release_heights.len(), 1);
		assert_eq!(vault.argons_locked, 50);
		assert_eq!(vault.argons_scheduled_for_release.len(), 1);
		assert_eq!(vault.argons_scheduled_for_release.get(&144).unwrap(), &50);
		assert_eq!(vault.securitization, 50);
		assert_eq!(vault.available_for_lock(), 50);
	}

	#[test]
	fn handles_schedule_for_release() {
		let mut vault = default_vault(500, 1.0);
		vault.lock(100).unwrap();

		let lock_extensions = &mut LockExtension::new(100);
		let release_heights = vault.schedule_for_release(50, lock_extensions).unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.argons_scheduled_for_release.len(), 1);
		assert_eq!(vault.argons_scheduled_for_release.get(&144).unwrap(), &50);

		let lock_extensions = &mut LockExtension::new(100);
		vault.extend_lock(50, lock_extensions).unwrap();
		assert_eq!(vault.argons_scheduled_for_release.len(), 0);
		assert_eq!(vault.argons_locked, 100);

		vault.lock(100).unwrap();
		assert_eq!(vault.argons_locked, 200);
		let lock_extensions = &mut LockExtension::new(250);
		let release_heights = vault.schedule_for_release(50, lock_extensions).unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.argons_locked, 200);
		assert_eq!(vault.get_relock_capacity(), 50);
		assert_eq!(vault.argons_scheduled_for_release.len(), 1);
		assert_eq!(vault.argons_scheduled_for_release.get(&288).unwrap(), &50);

		let lock_extensions = &mut LockExtension::new(255);
		let release_heights = vault.schedule_for_release(25, lock_extensions).unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.argons_locked, 200);
		assert_eq!(vault.get_relock_capacity(), 75);
		assert_eq!(vault.argons_scheduled_for_release.len(), 1);
		assert_eq!(vault.argons_scheduled_for_release.get(&288).unwrap(), &75);

		let lock_extensions = &mut LockExtension::new(300);
		let release_heights = vault.schedule_for_release(25, lock_extensions).unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.argons_locked, 200);
		assert_eq!(vault.get_relock_capacity(), 100);
		assert_eq!(vault.argons_scheduled_for_release.len(), 2);
		assert_eq!(
			vault.argons_scheduled_for_release.get(&288).unwrap(),
			&75,
			"shouldn't need to touch this"
		);
		assert_eq!(vault.argons_scheduled_for_release.get(&432).unwrap(), &25);

		// if the expiration is within the other expirations, it will prefer the argons locked
		let lock_extensions = &mut LockExtension::new(143);
		vault.extend_lock(25, lock_extensions).unwrap();
		assert_eq!(lock_extensions.len(), 0);
		assert_eq!(vault.argons_locked, 225);
		assert_eq!(vault.get_relock_capacity(), 100);
		assert_eq!(vault.argons_scheduled_for_release.len(), 2);
		assert_eq!(
			vault.argons_scheduled_for_release.get(&288).unwrap(),
			&75,
			"shouldn't need to touch this"
		);
		assert_eq!(vault.argons_scheduled_for_release.get(&432).unwrap(), &25);
		assert_eq!(vault.argons_scheduled_for_release.keys().collect::<Vec<_>>(), vec![&288, &432]);

		// take more than the argons_locked amount
		assert_eq!(vault.available_for_lock(), 375);
		let lock_extensions = &mut LockExtension::new(143);
		vault.extend_lock(370, lock_extensions).unwrap();
		assert_eq!(lock_extensions.len(), 2);
		assert_eq!(lock_extensions.get(&288).unwrap(), &75);
		assert_eq!(lock_extensions.get(&432).unwrap(), &20);
		assert_eq!(vault.get_relock_capacity(), 5);
		assert_eq!(vault.argons_locked, 500);
		assert_eq!(vault.argons_scheduled_for_release.len(), 1);
		assert_eq!(vault.argons_scheduled_for_release.get(&432).unwrap(), &5);

		// now return the 370
		let result = vault.schedule_for_release(370, lock_extensions).unwrap();
		assert_eq!(result.len(), 3);
		assert_eq!(result.iter().collect::<Vec<_>>(), vec![&144, &288, &432]);
		assert_eq!(vault.get_relock_capacity(), 375);
		assert_eq!(vault.argons_locked, 500);
		assert_eq!(vault.argons_scheduled_for_release.len(), 3);
		assert_eq!(vault.argons_scheduled_for_release.get(&144).unwrap(), &275);
		assert_eq!(vault.argons_scheduled_for_release.get(&288).unwrap(), &75);
		assert_eq!(vault.argons_scheduled_for_release.get(&432).unwrap(), &25);
	}

	fn default_vault(securitization: Balance, ratio: f64) -> Vault<u64, Balance> {
		Vault::<u64, Balance> {
			operator_account_id: 0,
			securitization,
			argons_locked: 0,
			argons_pending_activation: 0,
			argons_scheduled_for_release: Default::default(),
			securitization_ratio: FixedU128::from_float(ratio),
			is_closed: false,
			terms: VaultTerms {
				bitcoin_annual_percent_rate: 0.into(),
				bitcoin_base_fee: 0,
				liquidity_pool_profit_sharing: Default::default(),
			},
			pending_terms: None,
			opened_tick: 0,
		}
	}
}
