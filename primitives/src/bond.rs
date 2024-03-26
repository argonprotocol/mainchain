use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::PalletError;
use scale_info::TypeInfo;
use sp_debug_derive::RuntimeDebug;

pub trait BondProvider {
	type BondFundId: Codec + MaxEncodedLen;
	type BondId: Codec + MaxEncodedLen;
	type Balance: Codec;
	type AccountId: Codec;
	type BlockNumber: Codec;

	/// Access the details of a bond
	fn get_bond(
		bond_id: Self::BondId,
	) -> Result<Bond<Self::AccountId, Self::Balance, Self::BlockNumber, Self::BondFundId>, BondError>;

	/// Create a bone from one's own funds
	fn bond_self(
		account_id: Self::AccountId,
		amount: Self::Balance,
		bond_until_block: Self::BlockNumber,
	) -> Result<Self::BondId, BondError>;

	/// Create a time-locked bond from a bond fund
	fn lease(
		bond_fund_id: Self::BondFundId,
		account_id: Self::AccountId,
		amount: Self::Balance,
		lease_until_block: Self::BlockNumber,
	) -> Result<Self::BondId, BondError>;

	/// Add funding or time to a bond
	fn extend_bond(
		bond_id: Self::BondId,
		account_id: Self::AccountId,
		total_amount: Self::Balance,
		lease_until: Self::BlockNumber,
	) -> Result<(), BondError>;

	/// Return the bond to the originator with a prorated refund
	fn return_bond(bond_id: Self::BondId, account_id: Self::AccountId) -> Result<(), BondError>;

	/// Lock the bond so that it cannot be modified (performed by other pallets)
	fn lock_bond(bond_id: Self::BondId) -> Result<(), BondError>;

	/// Free the bond to be used for other things (or extended)
	fn unlock_bond(bond_id: Self::BondId) -> Result<(), BondError>;
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, PalletError)]
pub enum BondError {
	BadState,
	BondNotFound,
	NoMoreBondIds,
	MinimumBondAmountNotMet,
	LeaseUntilBlockTooSoon,
	BondFundClosed,
	LeaseUntilPastFundExpiration,
	/// There are too many bond or bond funds expiring in the given expiration block
	ExpirationAtBlockOverflow,
	AccountWouldBeBelowMinimum,
	InsufficientFunds,
	InsufficientBondFunds,
	ExpirationTooSoon,
	NoPermissions,
	NoBondFundFound,
	HoldUnexpectedlyModified,
	BondFundMaximumBondsExceeded,
	UnrecoverableHold,
	BondFundNotFound,
	BondAlreadyClosed,
	BondAlreadyLocked,
	BondLockedCannotModify,
	/// The fee for this bond exceeds the amount of the bond, which is unsafe
	FeeExceedsBondAmount,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Fee<Balance: From<u32>> {
	pub total_fee: Balance,
	pub base_fee: Balance,
	pub annual_percent_rate: u32,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct BondFund<
	AccountId: Codec,
	Balance: Codec + MaxEncodedLen,
	BlockNumber: Codec + MaxEncodedLen,
> {
	#[codec(compact)]
	pub lease_annual_percent_rate: u32,
	#[codec(compact)]
	pub lease_base_fee: Balance,
	pub offer_account_id: AccountId,
	#[codec(compact)]
	pub amount_reserved: Balance,
	pub offer_expiration_block: BlockNumber,
	#[codec(compact)]
	pub amount_bonded: Balance,
	pub is_ended: bool,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Bond<
	AccountId: Codec,
	Balance: Codec,
	BlockNumber: Codec,
	BondFundId: Codec + MaxEncodedLen,
> {
	pub bond_fund_id: Option<BondFundId>,
	pub bonded_account_id: AccountId,
	#[codec(compact)]
	pub annual_percent_rate: u32,
	#[codec(compact)]
	pub base_fee: Balance,
	/// Full fee paid
	#[codec(compact)]
	pub fee: Balance,
	#[codec(compact)]
	pub amount: Balance,
	#[codec(compact)]
	pub start_block: BlockNumber,
	#[codec(compact)]
	pub completion_block: BlockNumber,
	pub is_locked: bool,
}
