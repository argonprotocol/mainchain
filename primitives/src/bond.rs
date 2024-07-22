use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::PalletError;
use scale_info::TypeInfo;
use sp_arithmetic::{traits::UniqueSaturatedInto, FixedPointNumber, FixedU128, Percent};
use sp_debug_derive::RuntimeDebug;
use sp_runtime::traits::AtLeast32BitUnsigned;

use crate::{
	bitcoin::{BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinPubkeyHash, UtxoId},
	block_seal::RewardSharing,
	BondId, RewardShare, VaultId,
};

pub trait BondProvider {
	type Balance: Codec;
	type AccountId: Codec;
	type BlockNumber: Codec;

	/// Create a mining bond
	fn bond_mining_slot(
		vault_id: VaultId,
		account_id: Self::AccountId,
		amount: Self::Balance,
		bond_until_block: Self::BlockNumber,
	) -> Result<(BondId, Option<RewardSharing<Self::AccountId>>), BondError>;

	/// Return the bond to the originator with a prorated refund
	fn cancel_bond(bond_id: BondId) -> Result<(), BondError>;
}

pub trait VaultProvider {
	type Balance: Codec + Copy + TypeInfo + MaxEncodedLen + Default + AtLeast32BitUnsigned;
	type AccountId: Codec;
	type BlockNumber: Codec + MaxEncodedLen + Clone + TypeInfo + PartialEq + Eq;

	fn get(vault_id: VaultId) -> Option<Vault<Self::AccountId, Self::Balance, Self::BlockNumber>>;

	/// Recoup funds from the vault. This will be called if a vault does not move cosigned UTXOs in
	/// the appropriate timeframe. Steps are taken to repay the bitcoin holder at the market rate.
	///
	/// This will make the bonded account whole via funds from the vault in the following order:
	/// 1. From the bonded funds
	/// 2. From the allocated funds
	/// 3. From the securitized funds
	/// 4. TODO: From the Ulixee shares
	///
	/// The funds will be returned to the bond.bonded_account_id
	///
	/// Returns the amount that was recouped
	fn compensate_lost_bitcoin(
		bond: &Bond<Self::AccountId, Self::Balance, Self::BlockNumber>,
		market_rate: Self::Balance,
	) -> Result<Self::Balance, BondError>;

	/// Burn the funds from the vault. This will be called if a vault moves a bitcoin utxo outside
	/// the system. It is assumed that the vault is in cahoots with the bonded account.
	fn burn_vault_bitcoin_funds(
		bond: &Bond<Self::AccountId, Self::Balance, Self::BlockNumber>,
		amount_to_burn: Self::Balance,
	) -> Result<(), BondError>;

	/// Bonds the given amount of funds for the given vault. The fee is calculated based on the
	/// amount and the duration of the bond.
	fn bond_funds(
		vault_id: VaultId,
		amount: Self::Balance,
		bond_type: BondType,
		blocks: Self::BlockNumber,
		bond_account_id: &Self::AccountId,
	) -> Result<(Self::Balance, Self::Balance), BondError>;

	/// Release the bonded funds for the given bond. This will be called when the bond is completed
	/// or canceled. The remaining fee will be charged/returned based on the pro-rata owed
	fn release_bonded_funds(
		bond: &Bond<Self::AccountId, Self::Balance, Self::BlockNumber>,
	) -> Result<Self::Balance, BondError>;

	fn create_utxo_script_pubkey(
		vault_id: VaultId,
		utxo_id: UtxoId,
		owner_pubkey_hash: BitcoinPubkeyHash,
		vault_claim_height: BitcoinHeight,
		open_claim_height: BitcoinHeight,
	) -> Result<(BitcoinPubkeyHash, BitcoinCosignScriptPubkey), BondError>;
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, PalletError)]
pub enum BondError {
	BondNotFound,
	NoMoreBondIds,
	MinimumBondAmountNotMet,
	VaultClosed,
	/// There are too many bond or bond funds expiring in the given expiration block
	ExpirationAtBlockOverflow,
	AccountWouldBeBelowMinimum,
	InsufficientFunds,
	InsufficientVaultFunds,
	/// The vault does not have enough bitcoins to cover the mining bond
	InsufficientBitcoinsForMining,
	ExpirationTooSoon,
	NoPermissions,
	HoldUnexpectedlyModified,
	/// The hold could not be removed - it must have been modified
	UnrecoverableHold,
	VaultNotFound,
	/// No Vault public keys are available
	NoVaultBitcoinPubkeysAvailable,
	/// The fee for this bond exceeds the amount of the bond, which is unsafe
	FeeExceedsBondAmount,
	/// Scripting for a bitcoin UTXO failed
	InvalidBitcoinScript,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Vault<
	AccountId: Codec,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
	BlockNumber: Codec + MaxEncodedLen + Clone + TypeInfo + PartialEq + Eq,
> {
	/// The account assigned to operate this vault
	pub operator_account_id: AccountId,
	/// The assignment and allocation of bitcoin bonds
	pub bitcoin_argons: VaultArgons<Balance>,
	/// The additional securitization percent that has been added to the vault (recoverable by
	/// bonder in case of fraud or theft)
	#[codec(compact)]
	pub securitization_percent: FixedU128,
	/// The amount of argons that have been securitized
	#[codec(compact)]
	pub securitized_argons: Balance,
	/// The assignment and allocation of mining bonds
	pub mining_argons: VaultArgons<Balance>,
	/// The percent of argon mining rewards (minted and mined, not including fees) that this vault
	/// "charges"
	#[codec(compact)]
	pub mining_reward_sharing_percent_take: RewardShare,
	/// If the vault is closed, no new bonds can be issued
	pub is_closed: bool,
	/// The terms that are pending to be applied to this vault at the given block number
	pub pending_terms: Option<(BlockNumber, VaultTerms<Balance>)>,
}
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct VaultTerms<Balance: Codec + MaxEncodedLen + Clone + TypeInfo + PartialEq + Eq> {
	/// The annual percent rate per argon vaulted for bitcoin bonds
	#[codec(compact)]
	pub bitcoin_annual_percent_rate: FixedU128,
	/// The base fee for a bitcoin bond
	#[codec(compact)]
	pub bitcoin_base_fee: Balance,
	/// The annual percent rate per argon vaulted for mining bonds
	#[codec(compact)]
	pub mining_annual_percent_rate: FixedU128,
	/// A base fee for mining bonds
	#[codec(compact)]
	pub mining_base_fee: Balance,
	/// The optional sharing of any argons minted for stabilization or mined from blocks
	#[codec(compact)]
	pub mining_reward_sharing_percent_take: Percent, // max 100, actual percent
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
			+ PartialEq
			+ Eq,
		BlockNumber: Codec + MaxEncodedLen + Clone + TypeInfo + PartialEq + Eq,
	> Vault<AccountId, Balance, BlockNumber>
{
	pub fn bonded(&self) -> Balance {
		self.bitcoin_argons.bonded.saturating_add(self.mining_argons.bonded)
	}

	pub fn allocated(&self) -> Balance {
		self.bitcoin_argons.allocated.saturating_add(self.mining_argons.allocated)
	}

	pub fn amount_eligible_for_mining(&self) -> Balance {
		let allocated = self.mining_argons.free_balance();
		let mut bitcoins_bonded = self.bitcoin_argons.bonded;
		if self.securitized_argons > Balance::zero() {
			let allowed_securities =
				bitcoins_bonded.saturating_mul(2u32.into()).min(self.securitized_argons);
			bitcoins_bonded = bitcoins_bonded.saturating_add(allowed_securities);
		}
		allocated.min(bitcoins_bonded)
	}

	pub fn get_minimum_securitization_needed(&self) -> Balance {
		let argons =
			if self.is_closed { self.bitcoin_argons.bonded } else { self.bitcoin_argons.allocated };

		let argons = self
			.securitization_percent
			.saturating_mul_int::<u128>(argons.unique_saturated_into());

		argons.unique_saturated_into()
	}

	pub fn mut_argons(&mut self, bond_type: &BondType) -> &mut VaultArgons<Balance> {
		match *bond_type {
			BondType::Mining => &mut self.mining_argons,
			BondType::Bitcoin => &mut self.bitcoin_argons,
		}
	}

	pub fn argons(&self, bond_type: &BondType) -> &VaultArgons<Balance> {
		match *bond_type {
			BondType::Mining => &self.mining_argons,
			BondType::Bitcoin => &self.bitcoin_argons,
		}
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
pub struct VaultArgons<Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned> {
	#[codec(compact)]
	pub annual_percent_rate: FixedU128,
	#[codec(compact)]
	pub allocated: Balance,
	#[codec(compact)]
	pub bonded: Balance,
	#[codec(compact)]
	pub base_fee: Balance,
}

impl<Balance> VaultArgons<Balance>
where
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned,
{
	pub fn destroy_bond_funds(&mut self, amount: Balance) -> Result<(), BondError> {
		if self.bonded < amount {
			return Err(BondError::InsufficientFunds);
		}
		self.reduce_allocated(amount);
		self.reduce_bonded(amount);
		Ok(())
	}

	pub fn destroy_allocated_funds(&mut self, amount: Balance) -> Result<(), BondError> {
		if self.allocated < amount {
			return Err(BondError::InsufficientFunds);
		}
		self.reduce_allocated(amount);
		Ok(())
	}

	pub fn reduce_allocated(&mut self, amount: Balance) {
		self.allocated = self.allocated.saturating_sub(amount);
	}
	pub fn reduce_bonded(&mut self, amount: Balance) {
		self.bonded = self.bonded.saturating_sub(amount);
	}
}

impl<Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned> VaultArgons<Balance> {
	pub fn free_balance(&self) -> Balance {
		self.allocated.saturating_sub(self.bonded)
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Bond<AccountId: Codec, Balance: Codec, BlockNumber: Codec> {
	pub bond_type: BondType,
	#[codec(compact)]
	pub vault_id: VaultId,
	pub utxo_id: Option<UtxoId>,
	pub bonded_account_id: AccountId,
	#[codec(compact)]
	pub total_fee: Balance,
	#[codec(compact)]
	pub prepaid_fee: Balance,
	#[codec(compact)]
	pub amount: Balance,
	#[codec(compact)]
	pub start_block: BlockNumber,
	pub expiration: BondExpiration<BlockNumber>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BondExpiration<BlockNumber: Codec> {
	/// The bond will expire at the given block number
	UlixeeBlock(#[codec(compact)] BlockNumber),
	/// The bond will expire at a bitcoin block height
	BitcoinBlock(#[codec(compact)] BitcoinHeight),
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BondType {
	Mining,
	Bitcoin,
}
