use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::PalletError;
use scale_info::TypeInfo;
use sp_arithmetic::{FixedPointNumber, FixedU128, Permill};
use sp_debug_derive::RuntimeDebug;
use sp_runtime::traits::AtLeast32BitUnsigned;

use crate::{
	bitcoin::{BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinXPub, CompressedBitcoinPubkey},
	tick::Tick,
	ObligationId, VaultId,
};

pub trait MiningBidPoolProvider {
	type Balance: Codec;
	type AccountId: Codec;

	/// Transfer funds to the bid pool and hold
	fn get_bid_pool_account() -> Self::AccountId;
}

pub trait MiningBondFundVaultProvider {
	type Balance: Codec;
	type AccountId: Codec;

	/// Get the total amount of securitization activated for the vault
	fn get_activated_securitization(vault_id: VaultId) -> Self::Balance;

	/// Gets the account id of the vault and the vault share of mining bonds
	fn get_vault_payment_info(vault_id: VaultId) -> Option<(Self::AccountId, Permill)>;

	/// Ensure a vault is accepting mining bonds
	fn is_vault_accepting_mining_bonds(vault_id: VaultId) -> bool;
}

#[derive(RuntimeDebug, Clone, PartialEq, Eq)]
pub struct ReleaseFundsResult<Balance> {
	pub returned_to_beneficiary: Balance,
	pub paid_to_vault: Balance,
}

pub trait BitcoinObligationProvider {
	type Balance: Codec + Copy + TypeInfo + MaxEncodedLen + Default + AtLeast32BitUnsigned;
	type AccountId: Codec;

	fn is_owner(vault_id: VaultId, account_id: &Self::AccountId) -> bool;

	/// Return the obligation to the beneficiary with a prorated refund
	fn cancel_obligation(
		obligation_id: ObligationId,
	) -> Result<ReleaseFundsResult<Self::Balance>, ObligationError>;

	/// Holds the given amount of funds for the given vault. The fee is calculated based on the
	/// amount and the duration of the hold.
	fn create_obligation(
		vault_id: VaultId,
		beneficiary: &Self::AccountId,
		amount: Self::Balance,
		expiration_block: BitcoinHeight,
		ticks: Tick,
	) -> Result<Obligation<Self::AccountId, Self::Balance>, ObligationError>;

	/// Recoup funds from the vault. This will be called if a vault does not move cosigned UTXOs in
	/// the appropriate timeframe. Steps are taken to repay the bitcoin holder at the market rate.
	///
	/// This will make the beneficiary whole via funds from the vault in the following order:
	/// 1. From the obligation funds
	/// 2. From the allocated funds
	/// 3. From the securitized funds
	/// 4. TODO: From the ownership tokens
	///
	/// The funds will be returned to the `beneficiary`
	///
	/// Returns the amount (still owed, repaid)
	fn compensate_lost_bitcoin(
		obligation_id: ObligationId,
		market_rate: Self::Balance,
		unlock_amount_paid: Self::Balance,
	) -> Result<(Self::Balance, Self::Balance), ObligationError>;

	/// Burn the funds from the vault. This will be called if a vault moves a bitcoin utxo outside
	/// the system. It is assumed that the vault is in cahoots with the beneficiary.
	fn burn_vault_bitcoin_obligation(
		obligation_id: ObligationId,
		amount_to_burn: Self::Balance,
	) -> Result<Obligation<Self::AccountId, Self::Balance>, ObligationError>;

	fn create_utxo_script_pubkey(
		vault_id: VaultId,
		owner_pubkey: CompressedBitcoinPubkey,
		vault_claim_height: BitcoinHeight,
		open_claim_height: BitcoinHeight,
		current_height: BitcoinHeight,
	) -> Result<(BitcoinXPub, BitcoinXPub, BitcoinCosignScriptPubkey), ObligationError>;

	fn modify_pending_bitcoin_funds(
		vault_id: VaultId,
		amount: Self::Balance,
		remove_pending: bool,
	) -> Result<(), ObligationError>;
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, PalletError)]
pub enum ObligationError {
	ObligationNotFound,
	NoMoreObligationIds,
	MinimumObligationAmountNotMet,
	VaultClosed,
	/// There are too many obligations expiring in the given expiration block
	ExpirationAtBlockOverflow,
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
	/// An error occurred during completion of an obligation
	ObligationCompletionError,
	/// This vault is not yet active
	VaultNotYetActive,
	/// Too many base fee maturations were inserted per tick
	BaseFeeOverflow,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Vault<
	AccountId: Codec,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
> {
	/// The account assigned to operate this vault
	pub operator_account_id: AccountId,
	/// The securitization in the vault
	#[codec(compact)]
	pub securitization: Balance,
	/// The amount of locked bitcoins
	#[codec(compact)]
	pub bitcoin_locked: Balance,
	/// Bitcoins pending verification (this is "out of" the bitcoin_locked, not in addition to)
	#[codec(compact)]
	pub bitcoin_pending: Balance,
	/// The securitization ratio of "total securitization" to "available for locked bitcoin"
	#[codec(compact)]
	pub securitization_ratio: FixedU128,
	/// If the vault is closed, no new obligations can be issued
	pub is_closed: bool,
	/// The terms for locked bitcoin
	pub terms: VaultTerms<Balance>,
	/// The terms that are pending to be applied to this vault at the given tick
	pub pending_terms: Option<(Tick, VaultTerms<Balance>)>,
	/// A tick at which this vault is active
	#[codec(compact)]
	pub opened_tick: Tick,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct VaultTerms<Balance: Codec + MaxEncodedLen + Clone + TypeInfo + PartialEq + Eq> {
	/// The annual percent rate per argon vaulted for bitcoin locks
	#[codec(compact)]
	pub bitcoin_annual_percent_rate: FixedU128,
	/// The base fee for a bitcoin lock
	#[codec(compact)]
	pub bitcoin_base_fee: Balance,
	/// The percent of mining bonds taken by the vault
	#[codec(compact)]
	pub mining_bond_percent_take: Permill,
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
		let activated_securitization = self.bitcoin_locked.saturating_sub(self.bitcoin_pending);
		// you can increase the max allocation up to an additional 2x over the locked bitcoins
		let ratio = self.securitization_ratio.min(FixedU128::from_u32(2));
		ratio.saturating_mul_int(activated_securitization).min(self.securitization)
	}

	pub fn get_minimum_securitization_needed(&self) -> Balance {
		self.securitization_ratio.saturating_mul_int(self.bitcoin_locked)
	}

	pub fn get_recovery_securitization(&self) -> Balance {
		let reserved = FixedU128::from_u32(1).div(self.securitization_ratio);
		self.securitization
			.saturating_sub(reserved.saturating_mul_int(self.securitization))
	}

	pub fn destroy_funds(&mut self, amount: Balance) -> Result<(), ObligationError> {
		if self.bitcoin_locked < amount {
			return Err(ObligationError::InsufficientFunds);
		}
		self.reduce_securitization(amount);
		self.reduce_locked_bitcoin(amount);
		Ok(())
	}

	pub fn destroy_allocated_funds(&mut self, amount: Balance) -> Result<(), ObligationError> {
		if self.securitization < amount {
			return Err(ObligationError::InsufficientFunds);
		}
		self.reduce_securitization(amount);
		Ok(())
	}

	pub fn reduce_securitization(&mut self, amount: Balance) {
		self.securitization.saturating_reduce(amount);
	}

	pub fn reduce_locked_bitcoin(&mut self, amount: Balance) {
		self.bitcoin_locked.saturating_reduce(amount);
	}

	pub fn free_balance(&self) -> Balance {
		self.securitization
			.saturating_sub(self.get_recovery_securitization())
			.saturating_sub(self.bitcoin_locked)
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Obligation<AccountId: Codec, Balance: Codec> {
	#[codec(compact)]
	pub obligation_id: ObligationId,
	/// The type of funds this obligation drew from
	pub fund_type: FundType,
	#[codec(compact)]
	pub vault_id: VaultId,
	/// The recipient/beneficiary of this obligation activity
	pub beneficiary: AccountId,
	#[codec(compact)]
	pub total_fee: Balance,
	#[codec(compact)]
	pub prepaid_fee: Balance,
	#[codec(compact)]
	pub amount: Balance,
	#[codec(compact)]
	pub start_tick: Tick,
	pub expiration: ObligationExpiration,
	pub bitcoin_annual_percent_rate: Option<FixedU128>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum ObligationExpiration {
	/// The obligation will expire at the given tick
	AtTick(#[codec(compact)] Tick),
	/// The obligation will expire at a bitcoin block height
	BitcoinBlock(#[codec(compact)] BitcoinHeight),
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum FundType {
	LockedBitcoin,
}
