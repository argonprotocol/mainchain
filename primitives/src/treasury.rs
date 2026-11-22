use crate::{block_seal::FrameId, Balance, VaultId};
use codec::{Codec, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use polkadot_sdk::{
	frame_support::weights::Weight,
	sp_runtime::{
		traits::{AtLeast32BitUnsigned, Saturating},
		DispatchError, DispatchResult, FixedPointNumber, FixedU128,
	},
};
use scale_info::TypeInfo;

/// Account collection amounts in ARGON microgons, by source.
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
pub struct CollectAmounts<Balance>
where
	Balance: Codec + MaxEncodedLen,
{
	/// Rewards earned on ARGON bonds.
	#[codec(compact)]
	pub bond_earnings: Balance,
	/// Rewards earned on ARGONOT stakes.
	#[codec(compact)]
	pub stake_earnings: Balance,
	/// Rewards earned on eligible Bitcoin liquidity.
	#[codec(compact)]
	pub bitcoin_earnings: Balance,
	/// Rewards earned for supplying eligible stabilizer pool liquidity.
	#[codec(compact)]
	pub stabilizer_pool_earnings: Balance,
	/// Mature Treasury-funded stabilizer buy rewards.
	#[codec(compact)]
	pub stabilizer_buy_earnings: Balance,
	/// Additional rewards funded by completed Liquid repayment burns.
	#[codec(compact)]
	pub liquid_repayment_earnings: Balance,
	/// Vault revenue awaiting collection, subject to the vault's blockers and expiry policy.
	#[codec(compact)]
	pub vault_earnings: Balance,
}

impl<Balance: AtLeast32BitUnsigned + Copy + Codec + MaxEncodedLen> CollectAmounts<Balance> {
	pub fn total(&self) -> Balance {
		self.bond_earnings
			.saturating_add(self.stake_earnings)
			.saturating_add(self.bitcoin_earnings)
			.saturating_add(self.stabilizer_pool_earnings)
			.saturating_add(self.stabilizer_buy_earnings)
			.saturating_add(self.liquid_repayment_earnings)
			.saturating_add(self.vault_earnings)
	}

	pub fn saturating_accrue(&mut self, other: &Self) {
		self.bond_earnings.saturating_accrue(other.bond_earnings);
		self.stake_earnings.saturating_accrue(other.stake_earnings);
		self.bitcoin_earnings.saturating_accrue(other.bitcoin_earnings);
		self.stabilizer_pool_earnings.saturating_accrue(other.stabilizer_pool_earnings);
		self.stabilizer_buy_earnings.saturating_accrue(other.stabilizer_buy_earnings);
		self.liquid_repayment_earnings
			.saturating_accrue(other.liquid_repayment_earnings);
		self.vault_earnings.saturating_accrue(other.vault_earnings);
	}

	pub fn saturating_reduce(&mut self, other: &Self) {
		self.bond_earnings.saturating_reduce(other.bond_earnings);
		self.stake_earnings.saturating_reduce(other.stake_earnings);
		self.bitcoin_earnings.saturating_reduce(other.bitcoin_earnings);
		self.stabilizer_pool_earnings.saturating_reduce(other.stabilizer_pool_earnings);
		self.stabilizer_buy_earnings.saturating_reduce(other.stabilizer_buy_earnings);
		self.liquid_repayment_earnings
			.saturating_reduce(other.liquid_repayment_earnings);
		self.vault_earnings.saturating_reduce(other.vault_earnings);
	}
}

/// Issued Collect funding and wallet payments, in ARGON microgons by source.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Debug,
	Default,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct CollectTotals<Balance>
where
	Balance: Codec + MaxEncodedLen,
{
	/// Source funding issued into custody, excluding its minimum balance.
	pub funded: CollectAmounts<Balance>,
	/// Backed amounts delivered to wallets since activation.
	pub collected: CollectAmounts<Balance>,
	/// Vault revenue burned after its collection deadline. Other sources never expire.
	pub expired_vault_earnings: Balance,
}

/// Source attribution for custody funding, account accrual and wallet collection.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum CollectSource {
	/// ARGON bond earnings.
	Bonds,
	/// ARGONOT stake earnings, paid in ARGON.
	Stakes,
	/// Earnings on active Fissions' promised ARGON liquidity.
	BitcoinLiquidity,
	/// Accepted stabilizer pool earnings.
	StabilizerPools,
	/// Mature Treasury-funded stabilizer buy rewards.
	StabilizerBuys,
	/// Additional rewards funded by completed Liquid repayment burns.
	LiquidRepayments,
	/// Conditional vault revenue, in ARGON.
	Vaults,
}

impl<Balance: AtLeast32BitUnsigned + Copy + Default + Codec + MaxEncodedLen>
	CollectAmounts<Balance>
{
	pub fn from_source(source: CollectSource, amount: Balance) -> Self {
		let mut amounts = Self::default();
		match source {
			CollectSource::Bonds => amounts.bond_earnings = amount,
			CollectSource::Stakes => amounts.stake_earnings = amount,
			CollectSource::BitcoinLiquidity => amounts.bitcoin_earnings = amount,
			CollectSource::StabilizerPools => amounts.stabilizer_pool_earnings = amount,
			CollectSource::StabilizerBuys => amounts.stabilizer_buy_earnings = amount,
			CollectSource::LiquidRepayments => amounts.liquid_repayment_earnings = amount,
			CollectSource::Vaults => amounts.vault_earnings = amount,
		}
		amounts
	}
}

/// Cumulative ARGON earnings per eligible source quantity, scaled by FixedU128.
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
pub struct EarningsIndexValues {
	/// ARGON microgons per whole eligible ARGON bond, at FixedU128 precision.
	pub bonds: FixedU128,
	/// ARGON microgons per whole admitted ARGONOT stake, at FixedU128 precision.
	pub stakes: FixedU128,
	/// ARGON microgons per promised ARGON microgon of active Bitcoin liquidity.
	pub bitcoin_liquidity: FixedU128,
}

/// Saved cumulative index and fractional microgon carry for one indexed position.
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
pub struct EarningsCheckpoint {
	/// Cumulative source index already settled for this earning quantity.
	pub index: FixedU128,
	/// Fractional microgon numerator, always less than FixedU128::DIV.
	pub microgon_remainder: u128,
}

impl EarningsCheckpoint {
	pub fn settle(&mut self, quantity: u128, index: FixedU128) -> crate::Balance {
		let earnings_rate = index.saturating_sub(self.index);
		let amount = earnings_rate.saturating_mul_int(quantity);

		// Each reduced factor is below 10^18, so the fractional product fits in u128.
		let scale = FixedU128::DIV;
		let remainder = (quantity % scale) * (earnings_rate.into_inner() % scale) % scale +
			self.microgon_remainder;

		self.index = index;
		self.microgon_remainder = remainder % scale;
		amount.saturating_add(remainder / scale)
	}
}

/// Earning quantity and its saved financial progress; never a source lot/NFT identity.
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
pub struct IndexedEarningsPosition {
	/// Eligible whole bonds/stakes or promised Bitcoin liquidity in microgons for this interval.
	pub quantity: u128,
	/// Financial progress and fractional carry belonging to this account/source.
	pub checkpoint: EarningsCheckpoint,
}

impl IndexedEarningsPosition {
	pub fn settle(&mut self, index: FixedU128) -> crate::Balance {
		self.checkpoint.settle(self.quantity, index)
	}
}

/// ARGON earned against one frame's financial reward terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StabilizerPoolFrameClaim {
	/// Mining frame in which the rewards were earned.
	pub frame_id: FrameId,
	/// Earned ARGON in microgons, calculated by the source pallet.
	pub amount: Balance,
}

/// Financial frame terms and funding supplied by Treasury.
pub trait StabilizerPoolRewardProvider {
	type Weights: StabilizerPoolRewardProviderWeightInfo;

	/// Fix a funded frame's rate using the source's finalized eligible value in microgons.
	fn finalize_frame_rewards(frame_id: FrameId, eligible_value: Balance);

	/// Financial rate fixed after the source finalizes participation.
	fn frame_reward_rate(frame_id: FrameId) -> Option<FixedU128>;

	/// Fund finalized earnings in custody. The caller commits the source cursor and account
	/// accrual in the same transaction; failure leaves the earnings available for retry.
	fn claim_rewards(frames: &[StabilizerPoolFrameClaim]) -> Result<Balance, DispatchError>;
}

pub trait StabilizerPoolRewardProviderWeightInfo {
	fn finalize_frame_rewards() -> Weight;
	fn frame_reward_rate() -> Weight;
	fn claim_rewards(frame_count: u32) -> Weight;
}

impl StabilizerPoolRewardProviderWeightInfo for () {
	fn finalize_frame_rewards() -> Weight {
		Weight::zero()
	}
	fn frame_reward_rate() -> Weight {
		Weight::zero()
	}
	fn claim_rewards(_: u32) -> Weight {
		Weight::zero()
	}
}

impl StabilizerPoolRewardProvider for () {
	type Weights = ();

	fn finalize_frame_rewards(_: FrameId, _: Balance) {}

	fn frame_reward_rate(_: FrameId) -> Option<FixedU128> {
		None
	}
	fn claim_rewards(_: &[StabilizerPoolFrameClaim]) -> Result<Balance, DispatchError> {
		Err(DispatchError::Unavailable)
	}
}

/// Source positions that Treasury Positions can collect.
pub trait EarningsSettlementProvider<AccountId> {
	type Weights: EarningsSettlementProviderWeightInfo;
	/// Settle this account's available source earnings and advance its source cursors.
	/// The caller supplies the transaction through delivery and account bookkeeping.
	fn settle_account(account: &AccountId) -> DispatchResult;
}

impl<AccountId> EarningsSettlementProvider<AccountId> for () {
	type Weights = ();
	fn settle_account(_: &AccountId) -> DispatchResult {
		Ok(())
	}
}

pub trait EarningsSettlementProviderWeightInfo {
	fn settle_account() -> Weight;
}

/// Source-owned authorization and bookkeeping for the expiring vault earnings bucket.
pub trait VaultEarningsProvider<AccountId, Balance> {
	type Weights: VaultEarningsProviderWeightInfo;
	/// Clear the vault's source revenue when collection is allowed. A blocked vault returns
	/// false without changing its revenue; the caller supplies the transaction through payment.
	fn collect_earnings(account: &AccountId, amount: Balance) -> Result<bool, DispatchError>;
}

pub trait VaultEarningsProviderWeightInfo {
	fn collect_earnings() -> Weight;
}

impl VaultEarningsProviderWeightInfo for () {
	fn collect_earnings() -> Weight {
		Weight::zero()
	}
}

impl<AccountId, Balance> VaultEarningsProvider<AccountId, Balance> for () {
	type Weights = ();
	fn collect_earnings(_: &AccountId, _: Balance) -> Result<bool, DispatchError> {
		Ok(false)
	}
}

impl EarningsSettlementProviderWeightInfo for () {
	fn settle_account() -> Weight {
		Weight::zero()
	}
}

/// Treasury-owned unissued auction budgets. Reservations and returns share the auction caller's
/// transaction boundary. The auction owns reserved funding until it is earned or returned.
pub trait TreasuryRewardProvider {
	type Balance;
	type Weights: TreasuryRewardProviderWeightInfo;

	/// Available microgons, excluding auction reservations and committed rewards.
	fn available_budget() -> Self::Balance;
	/// Reserve microgons for one auction without issuing currency.
	fn reserve_budget(amount: Self::Balance) -> DispatchResult;
	/// Restore unused reserved microgons to available funding.
	fn return_budget(amount: Self::Balance) -> DispatchResult;
}

pub trait TreasuryRewardProviderWeightInfo {
	fn available_budget() -> Weight;
	fn reserve_budget() -> Weight;
	fn return_budget() -> Weight;
}

impl TreasuryRewardProviderWeightInfo for () {
	fn available_budget() -> Weight {
		Weight::zero()
	}
	fn reserve_budget() -> Weight {
		Weight::zero()
	}
	fn return_budget() -> Weight {
		Weight::zero()
	}
}

/// Canonical identifier of one Treasury bond purchase.
pub type BondLotId = u64;

/// Whole token units in one bond lot or vault's admission state.
pub type Bonds = u32;
/// Whole token units aggregated across account bonds or stakes.
pub type BondTotal = u128;

/// Allocated and activated securitization in microgons, for one position or a total.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Encode, Decode)]
pub struct BitcoinLockPosition<Balance> {
	/// Securitization activated by confirmed Bitcoin funding.
	pub activated_securitization: Balance,
	/// Total allocated securitization: activated securitization plus the unfunded remainder.
	pub allocated_securitization: Balance,
}

/// Live quantities shared by account positions and network totals. Sources retain individual
/// lots and Fissions; Treasury freezes earning totals when preparing a frame.
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
pub struct PositionQuantities<Balance>
where
	Balance: Codec + MaxEncodedLen,
{
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

/// Network sums of account positions and Bitcoin covered by Vaults.
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
pub struct NetworkPositionQuantities<Balance>
where
	Balance: Codec + MaxEncodedLen,
{
	pub account_quantities: PositionQuantities<Balance>,
	/// Confirmed Bitcoin satoshis backed by vault securitization.
	#[codec(compact)]
	pub vault_securitized_satoshis: u128,
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
pub struct UpstreamPosition<Balance>
where
	Balance: Codec + MaxEncodedLen,
{
	#[codec(compact)]
	pub vault_id: VaultId,
	/// Securitization activated by confirmed Bitcoin funding.
	#[codec(compact)]
	pub bitcoin_securitization: Balance,
	/// Total securitization allocated to this owner's upstream Locks and Fissions, including
	/// the unfunded remainder. Excludes the vault's unallocated reserved securitization space.
	#[codec(compact)]
	pub bitcoin_allocated_securitization: Balance,
	/// Non-releasing upstream ARGON principal, including flexible bonds.
	#[codec(compact)]
	pub bond_principal: Balance,
}

pub trait TreasuryPositionProviderWeightInfo {
	fn record_funding() -> Weight {
		Weight::zero()
	}
	fn expire_vault_earnings() -> Weight {
		Weight::zero()
	}
	fn prepare_collect() -> Weight {
		Weight::zero()
	}
	fn allocate_frame_earnings() -> Weight {
		Weight::zero()
	}
	fn indexed_frame() -> Weight {
		Weight::zero()
	}
	fn ensure_account() -> Weight {
		Weight::zero()
	}
	fn fund() -> Weight {
		Weight::zero()
	}
	fn accrue() -> Weight {
		Weight::zero()
	}
	fn operational_account_registered() -> Weight;
	fn bond_principal() -> Weight;
	fn account_quantities() -> Weight;
	fn network_totals() -> Weight;
	fn account_quantity_updated() -> Weight;
	fn network_securitized_satoshis_updated() -> Weight;
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
	fn network_securitized_satoshis_updated() -> Weight {
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
	/// Record issued funds already deposited into Collect custody. The source owns funding
	/// authorization and the transaction joining that deposit to this accounting.
	fn record_funding(_amounts: CollectAmounts<Balance>) {}
	/// Burn expired vault backing and reduce its account bucket in the source's transaction.
	fn expire_vault_earnings(_account: &AccountId, _amount: Balance) -> DispatchResult {
		Err(DispatchError::Unavailable)
	}
	/// Make an account with a source position discoverable for account collection.
	fn ensure_account(_: &AccountId) {}
	/// Address receiving issued backing. Funding callers use the configured native currency.
	fn collect_account() -> Option<AccountId> {
		None
	}
	/// Finish cutover backing and seed the custody minimum before a funding transfer.
	fn prepare_collect() -> DispatchResult {
		Ok(())
	}
	/// Issue authorized backing and record its source; the caller owns budget/grant authorization.
	fn fund(_source: CollectSource, _amount: Balance) -> DispatchResult {
		Err(DispatchError::Unavailable)
	}
	/// Assign already backed money to its beneficiary. Never mint or pay the wallet here.
	fn accrue(_account: &AccountId, _source: CollectSource, _amount: Balance) -> DispatchResult {
		Err(DispatchError::Unavailable)
	}
	/// Accrue pre-upgrade unpaid IOUs and their aggregate cutover backing requirement.
	/// The account storage layout must be initialized before this migration operation.
	fn migrate_accrued(_account: &AccountId, _amounts: CollectAmounts<Balance>) {}
	/// Commit one retained frame's funded allocations, indexes and requested account boundaries.
	fn allocate_frame_earnings(
		_frame: FrameId,
		_quantities: &PositionQuantities<Balance>,
		_amounts: CollectAmounts<Balance>,
	) -> DispatchResult {
		Err(DispatchError::Unavailable)
	}
	fn indexed_frame() -> Option<FrameId> {
		None
	}
	fn earnings_start_frame() -> FrameId {
		1
	}
	/// Notify positions after the account and its upstream relationship are stored.
	fn operational_account_registered(account: &AccountId) -> DispatchResult;
	fn bond_principal(account: &AccountId) -> Balance;
	/// Maintained quantities across this account's live source positions.
	fn account_quantities(account: &AccountId) -> PositionQuantities<Balance>;
	/// Network totals maintained from account and Vault updates; no source scan.
	fn network_totals() -> NetworkPositionQuantities<Balance>;
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
	/// Replace a Vault's covered satoshis in the network total alongside its canonical mutation.
	fn network_securitized_satoshis_updated(previous: u128, current: u128) -> DispatchResult;
	fn upstream_position(account: &AccountId) -> Option<UpstreamPosition<Balance>>;
	/// Complete upstream snapshot, installed at registration or during migration.
	fn set_upstream_position(account: &AccountId, upstream: Option<UpstreamPosition<Balance>>);
	/// Update the account's Bitcoin securitization totals: subtract `previous`, then add
	/// `position`. Include only the Locks and Fissions changed by the operation.
	/// This update and the Lock/Fission changes must succeed or roll back together.
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
	fn network_totals() -> NetworkPositionQuantities<Balance> {
		NetworkPositionQuantities::default()
	}
	fn account_quantity_updated<Amount: Into<u128>>(
		_: &AccountId,
		_: PositionQuantity,
		_: Amount,
		_: Amount,
	) -> DispatchResult {
		Ok(())
	}
	fn network_securitized_satoshis_updated(_: u128, _: u128) -> DispatchResult {
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
