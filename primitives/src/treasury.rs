use crate::VaultId;
use codec::{Codec, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use polkadot_sdk::{
	frame_support::weights::Weight,
	sp_runtime::{DispatchError, DispatchResult},
};
use scale_info::TypeInfo;

/// Whole token units in one bond lot or vault's admission state.
pub type Bonds = u32;
/// Whole token units aggregated across account bonds or stakes.
pub type BondTotal = u128;

/// Absolute Bitcoin collateral amounts in microgons, for one lock or a sum of locks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BitcoinLockPosition<Balance> {
	/// Collateral backing confirmed funding.
	pub activated_securitization: Balance,
	/// Total collateral allocated to the lock: activated collateral plus the unfunded remainder.
	pub allocated_securitization: Balance,
}

/// Live quantities shared by account positions and network totals. Sources retain individual
/// lots and Fissions; Treasury freezes bond/stake totals when preparing a frame.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	Default,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PositionQuantities<Balance: Codec + MaxEncodedLen> {
	/// Whole ARGON bonds: regular principal plus undisplaced operator flexible bonds.
	#[codec(compact)]
	pub bonds: BondTotal,
	/// Whole ARGONOTs in the admitted stake set, excluding releasing lots.
	#[codec(compact)]
	pub stakes: BondTotal,
	/// Active Fissions' full promised ARGON liquidity in microgons, including pending mints.
	#[codec(compact)]
	pub fission_liquidity: Balance,
}

/// The single account and network quantity replaced by a source update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionQuantity {
	Bonds,
	Stakes,
	FissionLiquidity,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	Default,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct UpstreamPosition<Balance: Codec + MaxEncodedLen> {
	#[codec(compact)]
	pub vault_id: VaultId,
	/// Collateral used by confirmed funding.
	#[codec(compact)]
	pub bitcoin_securitization: Balance,
	/// Total collateral allocated to this owner's upstream locks: activated collateral plus
	/// the unfunded remainder. Excludes the vault's unallocated reserved securitization space.
	#[codec(compact)]
	pub bitcoin_allocated_securitization: Balance,
	/// Non-releasing upstream ARGON principal, including flexible bonds.
	#[codec(compact)]
	pub bond_principal: Balance,
}

pub trait TreasuryPositionProviderWeightInfo {
	fn operational_account_registered() -> Weight;
	fn bond_principal() -> Weight;
	fn account_quantities() -> Weight;
	fn network_totals() -> Weight;
	fn account_quantity_updated() -> Weight;
	fn upstream_position() -> Weight;
	fn set_upstream_position() -> Weight;
	fn position_updated() -> Weight;
}
impl TreasuryPositionProviderWeightInfo for () {
	fn operational_account_registered() -> Weight {
		Weight::zero()
	}
	fn bond_principal() -> Weight {
		Weight::zero()
	}
	fn account_quantities() -> Weight {
		Weight::zero()
	}
	fn network_totals() -> Weight {
		Weight::zero()
	}
	fn account_quantity_updated() -> Weight {
		Weight::zero()
	}
	fn upstream_position() -> Weight {
		Weight::zero()
	}
	fn set_upstream_position() -> Weight {
		Weight::zero()
	}
	fn position_updated() -> Weight {
		Weight::zero()
	}
}

/// Source contributions are replaced atomically with their canonical mutation.
pub trait TreasuryPositionProvider<AccountId, Balance: Codec + MaxEncodedLen> {
	type Weights: TreasuryPositionProviderWeightInfo;
	/// Notify positions after the account and its upstream relationship are stored.
	fn operational_account_registered(account: &AccountId) -> DispatchResult;
	fn bond_principal(account: &AccountId) -> Balance;
	/// Maintained quantities across this account's live source positions.
	fn account_quantities(account: &AccountId) -> PositionQuantities<Balance>;
	/// Network totals maintained with the account quantities; no account or source scan.
	fn network_totals() -> PositionQuantities<Balance>;
	/// Replace one source contribution in the selected account and network quantity atomically.
	/// Bonds and stakes are whole units; Fission liquidity is microgons. The source
	/// supplies its absolute before/after amounts in the same storage transaction as its canonical
	/// change.
	fn account_quantity_updated<Amount: Into<u128>>(
		account: &AccountId,
		quantity: PositionQuantity,
		previous: Amount,
		current: Amount,
	) -> DispatchResult;
	fn upstream_position(account: &AccountId) -> Option<UpstreamPosition<Balance>>;
	/// Complete upstream snapshot, installed at registration or during migration.
	fn set_upstream_position(account: &AccountId, upstream: Option<UpstreamPosition<Balance>>);
	/// Replace one lock's contribution; the caller supplies its absolute before/after amounts
	/// within the same storage transaction as the canonical lock mutation.
	fn bitcoin_position_updated(
		account: &AccountId,
		vault: VaultId,
		previous: BitcoinLockPosition<Balance>,
		position: BitcoinLockPosition<Balance>,
	) -> DispatchResult;
	fn bond_position_updated(
		account: &AccountId,
		vault: VaultId,
		previous: Balance,
		principal: Balance,
	) -> DispatchResult;
}
impl<AccountId, Balance: Codec + MaxEncodedLen + Default>
	TreasuryPositionProvider<AccountId, Balance> for ()
{
	type Weights = ();
	fn operational_account_registered(_: &AccountId) -> DispatchResult {
		Ok(())
	}
	fn bond_principal(_: &AccountId) -> Balance {
		Balance::default()
	}
	fn account_quantities(_: &AccountId) -> PositionQuantities<Balance> {
		PositionQuantities::default()
	}
	fn network_totals() -> PositionQuantities<Balance> {
		PositionQuantities::default()
	}
	fn account_quantity_updated<Amount: Into<u128>>(
		_: &AccountId,
		_: PositionQuantity,
		_: Amount,
		_: Amount,
	) -> DispatchResult {
		Ok(())
	}
	fn upstream_position(_: &AccountId) -> Option<UpstreamPosition<Balance>> {
		None
	}
	fn set_upstream_position(_: &AccountId, _: Option<UpstreamPosition<Balance>>) {}
	fn bitcoin_position_updated(
		_: &AccountId,
		_: VaultId,
		_: BitcoinLockPosition<Balance>,
		_: BitcoinLockPosition<Balance>,
	) -> DispatchResult {
		Ok(())
	}
	fn bond_position_updated(_: &AccountId, _: VaultId, _: Balance, _: Balance) -> DispatchResult {
		Ok(())
	}
}

pub trait BitcoinLockPositionProviderWeightInfo {
	fn account_position() -> Weight;
}
impl BitcoinLockPositionProviderWeightInfo for () {
	fn account_position() -> Weight {
		Weight::zero()
	}
}
/// Canonical owner-index query used only when establishing an upstream relationship.
pub trait BitcoinLockPositionProvider<AccountId, Balance> {
	type Weights: BitcoinLockPositionProviderWeightInfo;
	fn account_position(
		account: &AccountId,
		vault: VaultId,
	) -> Result<BitcoinLockPosition<Balance>, DispatchError>;
}
impl<AccountId, Balance: Default> BitcoinLockPositionProvider<AccountId, Balance> for () {
	type Weights = ();
	fn account_position(
		_: &AccountId,
		_: VaultId,
	) -> Result<BitcoinLockPosition<Balance>, DispatchError> {
		Ok(BitcoinLockPosition::default())
	}
}
