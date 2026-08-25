use alloc::collections::BTreeSet;
use codec::{Codec, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::iter::Sum;
use frame_support::{weights::Weight, PalletError};
use polkadot_sdk::{sp_core::ConstU32, sp_runtime::BoundedBTreeMap, *};
use scale_info::TypeInfo;
use sp_arithmetic::{FixedPointNumber, FixedU128, Permill};
use sp_core::blake2_256;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, SaturatedConversion, Saturating, Verify},
	AccountId32,
};

use crate::{
	bitcoin::{
		get_rounded_up_bitcoin_day_height, BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinXPub,
		CompressedBitcoinPubkey, Satoshis, UtxoId, SATOSHIS_PER_BITCOIN,
	},
	ensure,
	prelude::FrameId,
	tick::Tick,
	Signature, VaultId,
};

pub const MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES: u32 = 366;

pub trait BitcoinVaultProviderWeightInfo {
	fn get_registration_vault_data() -> Weight;
	fn get_committed_securitization() -> Weight;
	fn get_committed_argonots() -> Weight;
	fn encumber_argonots() -> Weight;
	fn release_encumbered_argonots() -> Weight;
	fn burn_encumbered_argonots() -> Weight;
	fn account_became_operational() -> Weight;
	fn set_bitcoin_lock_flexible() -> Weight;
	fn resecuritize() -> Weight;
}

impl BitcoinVaultProviderWeightInfo for () {
	fn get_registration_vault_data() -> Weight {
		Weight::zero()
	}

	fn get_committed_securitization() -> Weight {
		Weight::zero()
	}

	fn get_committed_argonots() -> Weight {
		Weight::zero()
	}

	fn encumber_argonots() -> Weight {
		Weight::zero()
	}

	fn release_encumbered_argonots() -> Weight {
		Weight::zero()
	}

	fn burn_encumbered_argonots() -> Weight {
		Weight::zero()
	}

	fn account_became_operational() -> Weight {
		Weight::zero()
	}

	fn set_bitcoin_lock_flexible() -> Weight {
		Weight::zero()
	}

	fn resecuritize() -> Weight {
		Weight::zero()
	}
}

pub const TREASURY_BONUS_APPROVAL_PROOF_MESSAGE_KEY: &[u8] = b"treasury_bonus_approval";

#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct TreasuryBonusApprovalProof {
	#[codec(compact)]
	pub vault_id: VaultId,
	pub beneficiary: AccountId32,
	#[codec(compact)]
	pub bonus_percent: Permill,
	#[codec(compact)]
	pub expires_at_frame: FrameId,
	#[codec(compact)]
	pub bond_space_to_unreserve: u32,
	/// Monotonically increasing value preventing replay for this vault and beneficiary.
	#[codec(compact)]
	pub nonce: u64,
	pub signature: Signature,
}

impl TreasuryBonusApprovalProof {
	pub fn verify(&self, signer: &AccountId32) -> bool {
		let message = (
			TREASURY_BONUS_APPROVAL_PROOF_MESSAGE_KEY,
			self.vault_id,
			&self.beneficiary,
			self.bonus_percent,
			self.expires_at_frame,
			self.bond_space_to_unreserve,
			self.nonce,
		)
			.using_encoded(blake2_256);
		let verified = self.signature.verify(message.as_slice(), signer);
		#[cfg(feature = "runtime-benchmarks")]
		{
			let _ = verified;
			true
		}
		#[cfg(not(feature = "runtime-benchmarks"))]
		{
			verified
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistrationVaultData<Balance> {
	pub vault_id: VaultId,
	pub activated_securitization: Balance,
	pub securitization: Balance,
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
	/// Argonot securitization in the vault at the frame turn.
	pub argonot_securitization: Balance,
	/// Argonots needed for the vault to realize its maximum Treasury earnings.
	pub argonots_for_max_earnings: Balance,
	/// Vault Treasury earnings not realized because of its Argonot securitization.
	pub treasury_unrealized_earnings: Balance,
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Default,
)]
pub struct VaultArgonotCommitment<Balance>
where
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	/// Total committed argonots, denominated in micronots.
	#[codec(compact)]
	pub committed_micronots: Balance,

	/// Amount of argonots held for cross-chain transfer collateral, denominated in micronots.
	#[codec(compact)]
	pub encumbered_micronots: Balance,
}

pub struct VaultBondEarningsSnapshot<Balance> {
	/// Raw securitized satoshis used for the vault's pro-rata Argonot requirement.
	pub securitized_satoshis: Satoshis,
	/// Argonot securitization in the vault, denominated in micronots.
	pub argonot_securitization: Balance,
}

pub trait TreasuryVaultProviderWeightInfo {
	fn get_bond_earnings_snapshot() -> Weight;
}

impl TreasuryVaultProviderWeightInfo for () {
	fn get_bond_earnings_snapshot() -> Weight {
		Weight::zero()
	}
}

pub trait TreasuryVaultProvider {
	type Weights: TreasuryVaultProviderWeightInfo;
	type Balance: Codec;
	type AccountId: Codec;

	/// Get the vault capital and effective Bitcoin-backed Treasury capacity.
	fn get_eligible_capacity(vault_id: VaultId) -> (Self::Balance, Satoshis);

	/// Get the values used to determine the vault's bond earnings eligibility.
	fn get_bond_earnings_snapshot(vault_id: VaultId) -> VaultBondEarningsSnapshot<Self::Balance>;

	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId>;
	fn get_vault_delegate(vault_id: VaultId) -> Option<Self::AccountId>;
	/// Gets the bonder-side percent of lot yield shared to the bond holder.
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

#[derive(Clone, Copy)]
pub struct BitcoinSecuritization<Balance> {
	/// Maximum satoshis covered by this securitization.
	pub securitized_satoshis: Satoshis,
	/// Target value of one BTC in microgons.
	pub microgons_at_target_per_btc: Balance,
	/// Microgon coverage after applying the redemption curve.
	pub securitization_coverage_microgons: Balance,
	/// Vault collateral required per unit of securitization coverage.
	pub securitization_ratio: FixedU128,
}

impl<Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned>
	BitcoinSecuritization<Balance>
{
	pub fn btc_value_in_microgons(&self) -> Balance {
		FixedU128::from_rational(self.securitized_satoshis as u128, SATOSHIS_PER_BITCOIN as u128)
			.saturating_mul_int(self.microgons_at_target_per_btc)
	}

	pub fn collateral_required(&self) -> Balance {
		self.securitization_ratio
			.saturating_mul_int(self.securitization_coverage_microgons)
	}

	/// Ratio-adjusted funded satoshis eligible for vault capacity.
	pub fn eligible_satoshis(&self, funded_satoshis: Satoshis) -> Satoshis {
		self.securitization_ratio
			.saturating_mul_int(funded_satoshis.min(self.securitized_satoshis))
	}
}

pub struct ReserveSecuritizationRequest<Balance> {
	/// Fee coupon value supplied by the Lock owner.
	pub fee_discount: Balance,
	/// Aggregate vault securitization space released for this operation.
	pub securitization_space_to_unreserve: Balance,
}

pub struct BitcoinResecuritization<'a, Balance> {
	/// Existing Lock securitization to replace.
	pub current: &'a BitcoinSecuritization<Balance>,
	/// New Lock securitization.
	pub replacement: &'a BitcoinSecuritization<Balance>,
	/// Confirmed satoshis attached to the Lock, or zero while pending funding.
	pub funded_satoshis: Satoshis,
	/// Fraction of the original Lock term still remaining.
	pub remaining_term: FixedU128,
	/// Existing collateral extensions, updated for the replacement.
	pub lock_extension: &'a mut LockExtension<Balance>,
	/// Whether the Lock uses flexible vault collateral.
	pub is_flexible: bool,
	/// Fee coupon value supplied by the Lock owner.
	pub fee_discount: Balance,
	/// Aggregate vault securitization space released for this operation.
	pub securitization_space_to_unreserve: Balance,
}

pub trait BitcoinVaultProvider {
	type Weights: BitcoinVaultProviderWeightInfo;
	type Balance: Codec + Copy + TypeInfo + MaxEncodedLen + Default + AtLeast32BitUnsigned;
	type AccountId: Codec;

	fn is_owner(vault_id: VaultId, account_id: &Self::AccountId) -> bool;
	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId>;
	fn get_vault_delegate(vault_id: VaultId) -> Option<Self::AccountId>;
	fn get_vault_id(account_id: &Self::AccountId) -> Option<VaultId>;
	fn get_locked_securitization(_vault_id: VaultId) -> Option<Self::Balance> {
		None
	}
	fn get_registration_vault_data(
		account_id: &Self::AccountId,
	) -> Option<RegistrationVaultData<Self::Balance>>;
	fn get_committed_securitization(
		account_id: &Self::AccountId,
		min_frames_remaining: FrameId,
	) -> Option<Self::Balance>;
	fn get_committed_argonots(account_id: &Self::AccountId) -> Option<Self::Balance>;
	fn encumber_argonots(
		account_id: &Self::AccountId,
		amount: Self::Balance,
	) -> Result<(), VaultError>;
	fn release_encumbered_argonots(
		account_id: &Self::AccountId,
		amount: Self::Balance,
	) -> Result<(), VaultError>;
	fn burn_encumbered_argonots(
		account_id: &Self::AccountId,
		amount: Self::Balance,
	) -> Result<(), VaultError>;
	fn account_became_operational(_vault_operator_account: &Self::AccountId) {}

	/// Get the securitization ratio offered by this vault
	fn get_securitization_ratio(vault_id: VaultId) -> Result<FixedU128, VaultError>;

	/// Activate a pending securitization when its Bitcoin funding is detected.
	fn activate_securitization(
		vault_id: VaultId,
		securitization: &BitcoinSecuritization<Self::Balance>,
		funded_satoshis: Satoshis,
	) -> Result<(), VaultError>;

	/// Return projected `(flexible requirement, undisplaced flexible requirement)` after replacing
	/// a funded flexible lock's released securitization with its newly added securitization.
	fn get_projected_flexible_securitization(
		vault_id: VaultId,
		flexible_securitization_released: Self::Balance,
		flexible_securitization_added: Self::Balance,
	) -> Option<(Self::Balance, Self::Balance)>;

	/// Move a funded Bitcoin lock into or out of the vault's flexible totals.
	fn set_bitcoin_lock_flexible(
		vault_id: VaultId,
		securitization: &BitcoinSecuritization<Self::Balance>,
		funded_satoshis: Satoshis,
		is_flexible: bool,
	) -> Result<(), VaultError>;

	/// Reserve vault collateral for a Bitcoin Lock pending funding confirmation.
	fn reserve_securitization(
		vault_id: VaultId,
		locker: &Self::AccountId,
		securitization: &BitcoinSecuritization<Self::Balance>,
		request: ReserveSecuritizationRequest<Self::Balance>,
	) -> Result<(Self::Balance, Self::Balance), VaultError>;

	/// Replace all securitization terms for one Lock while preserving its funded Bitcoin.
	fn resecuritize(
		_vault_id: VaultId,
		_locker: &Self::AccountId,
		_request: BitcoinResecuritization<'_, Self::Balance>,
	) -> Result<(Self::Balance, Self::Balance), VaultError> {
		Err(VaultError::InternalError)
	}

	/// End a funded Lock and schedule its collateral for release.
	fn schedule_securitization_release(
		vault_id: VaultId,
		securitization: &BitcoinSecuritization<Self::Balance>,
		funded_satoshis: Satoshis,
		lock_extension: &LockExtension<Self::Balance>,
		is_flexible: bool,
	) -> Result<(), VaultError>;

	/// Return the collateral reserved by an unfunded Lock.
	fn return_securitization(
		vault_id: VaultId,
		securitization: &BitcoinSecuritization<Self::Balance>,
	) -> Result<(), VaultError>;

	/// Burn the funds from the vault. This will be called if a vault moves a bitcoin utxo outside
	/// the system. It is assumed that the vault is in cahoots with the beneficiary.
	///
	/// Returns the amount of argons that were burned
	fn burn(
		vault_id: VaultId,
		securitization: &BitcoinSecuritization<Self::Balance>,
		funded_satoshis: Satoshis,
		market_rate: Self::Balance,
		lock_extension: &LockExtension<Self::Balance>,
		is_flexible: bool,
	) -> Result<Self::Balance, VaultError>;

	/// Recoup funds from the vault. This will be called if a vault has performed an illegal
	/// activity, like not moving cosigned UTXOs in the appropriate timeframe.
	///
	/// The recouped funds is up to the market rate, but capped at securitization rate of the
	/// vault.
	///
	/// Returns the amounts sent to the beneficiary and burned.
	fn compensate_lost_bitcoin(
		vault_id: VaultId,
		beneficiary: &Self::AccountId,
		securitization: &BitcoinSecuritization<Self::Balance>,
		funded_satoshis: Satoshis,
		market_rate: Self::Balance,
		lock_extension: &LockExtension<Self::Balance>,
		is_flexible: bool,
	) -> Result<LostBitcoinCompensation<Self::Balance>, VaultError>;

	fn create_utxo_script_pubkey(
		vault_id: VaultId,
		owner_pubkey: CompressedBitcoinPubkey,
		vault_claim_height: BitcoinHeight,
		open_claim_height: BitcoinHeight,
		current_height: BitcoinHeight,
	) -> Result<(BitcoinXPub, BitcoinXPub, BitcoinCosignScriptPubkey), VaultError>;

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
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, PalletError,
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
	/// Committed Argonots cannot be reduced below the backing already encumbered elsewhere.
	CommittedArgonotsBelowEncumberedBacking,
}

#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct Vault<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	/// The account assigned to operate this vault
	pub operator_account_id: AccountId,
	/// Optional delegated hot account allowed to act on behalf of this vault.
	pub delegate_account_id: Option<AccountId>,
	/// The securitization in the vault
	#[codec(compact)]
	pub securitization: Balance,
	/// The target securitization to have in the vault (in case of reducing)
	#[codec(compact)]
	pub securitization_target: Balance,
	/// The securitization locked for bitcoin (at the ratio given)
	#[codec(compact)]
	pub securitization_locked: Balance,
	/// The funded flexible portion of `securitization_locked`.
	#[codec(compact)]
	pub flexible_securitization_locked: Balance,
	/// Vault securitization space reserved for future Bitcoin locks.
	#[codec(compact)]
	pub reserved_securitization_space: Balance,
	/// Securitization pending bitcoin funding confirmation (this is "out of" the
	/// securitization_locked, not in addition to)
	#[codec(compact)]
	pub securitization_pending_activation: Balance,
	/// The number of locked satoshis currently tracked by this vault.
	#[codec(compact)]
	pub locked_satoshis: Satoshis,
	/// Funded satoshis adjusted by each Lock's securitization ratio.
	#[codec(compact)]
	pub ratio_adjusted_satoshis: Satoshis,
	/// The funded flexible portion of `ratio_adjusted_satoshis`.
	#[codec(compact)]
	pub flexible_ratio_adjusted_satoshis: Satoshis,
	/// Securitization that will be released at the given block height (NOTE: these are grouped by
	/// next day of bitcoin blocks). This securitization can be relocked
	pub securitization_release_schedule: BoundedBTreeMap<
		BitcoinHeight,
		Balance,
		ConstU32<MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES>,
	>,
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
	/// Optional tick when the temporary operational minimum securitization may be released.
	pub operational_minimum_release_tick: Option<Tick>,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
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
	/// The bonder-side percent of lot yield shared to the bond holder.
	#[codec(compact)]
	pub treasury_profit_sharing: Permill,
}

pub struct BurnResult<Balance> {
	pub burned_amount: Balance,
	pub held_for_release: Balance,
	pub release_heights: BTreeSet<BitcoinHeight>,
}

pub struct LostBitcoinCompensation<Balance> {
	pub to_beneficiary: Balance,
	pub burned: Balance,
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
		let activated = self.get_activated_securitization();
		let regular_securitization_locked = self.regular_securitization_locked();
		debug_assert!(
			self.securitization_pending_activation <= self.securitization_locked,
			"[{where_}] invariant failed: pending securitization ({:?}) > locked ({:?})",
			self.securitization_pending_activation,
			self.securitization_locked
		);
		debug_assert!(
			self.flexible_securitization_locked <= activated,
			"[{where_}] invariant failed: flexible securitization ({:?}) > activated ({:?})",
			self.flexible_securitization_locked,
			activated
		);
		debug_assert!(
			regular_securitization_locked.saturating_add(self.reserved_securitization_space) <=
				self.securitization,
			"[{where_}] invariant failed: regular securitization ({:?}) plus reserved ({:?}) exceeds securitization ({:?})",
			regular_securitization_locked,
			self.reserved_securitization_space,
			self.securitization,
		);
		debug_assert!(
			self.flexible_ratio_adjusted_satoshis <= self.ratio_adjusted_satoshis,
			"[{where_}] invariant failed: flexible securitized satoshis ({:?}) > total ({:?})",
			self.flexible_ratio_adjusted_satoshis,
			self.ratio_adjusted_satoshis
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

	pub fn regular_securitization_locked(&self) -> Balance {
		self.securitization_locked.saturating_sub(self.flexible_securitization_locked)
	}

	pub fn securitization_space(&self) -> Balance {
		self.securitization.saturating_sub(self.regular_securitization_locked())
	}

	pub fn undisplaced_flexible_securitization(&self) -> Balance {
		self.flexible_securitization_locked.min(self.securitization_space())
	}

	pub fn projected_flexible_securitization(
		&self,
		flexible_securitization_released: Balance,
		flexible_securitization_added: Balance,
	) -> (Balance, Balance) {
		let flexible_securitization = self
			.flexible_securitization_locked
			.saturating_sub(flexible_securitization_released)
			.saturating_add(flexible_securitization_added);
		let undisplaced = flexible_securitization.min(self.securitization_space());
		(flexible_securitization, undisplaced)
	}

	pub fn effective_eligible_satoshis(&self) -> Satoshis {
		if self.flexible_securitization_locked.is_zero() {
			return self.ratio_adjusted_satoshis;
		}

		let confirmed_regular_securitization_locked = self
			.get_activated_securitization()
			.saturating_sub(self.flexible_securitization_locked);
		let flexible_securitization_available =
			self.securitization.saturating_sub(confirmed_regular_securitization_locked);
		let undisplaced_flexible =
			self.flexible_securitization_locked.min(flexible_securitization_available);
		let earning_flexible_satoshis = FixedU128::from_rational(
			undisplaced_flexible.saturated_into::<u128>(),
			self.flexible_securitization_locked.saturated_into::<u128>(),
		)
		.saturating_mul_int(self.flexible_ratio_adjusted_satoshis);

		self.ratio_adjusted_satoshis
			.saturating_sub(self.flexible_ratio_adjusted_satoshis)
			.saturating_add(earning_flexible_satoshis)
	}

	pub fn set_reserved_securitization_space(
		&mut self,
		reserved_securitization_space: Balance,
	) -> Result<(), VaultError> {
		ensure!(
			reserved_securitization_space <= self.securitization_space(),
			VaultError::InsufficientVaultFunds
		);
		self.reserved_securitization_space = reserved_securitization_space;
		Ok(())
	}

	pub fn set_bitcoin_lock_flexible(
		&mut self,
		securitization: &BitcoinSecuritization<Balance>,
		funded_satoshis: Satoshis,
		is_flexible: bool,
	) -> Result<(), VaultError> {
		let collateral_required = securitization.collateral_required();
		let eligible_satoshis = securitization.eligible_satoshis(funded_satoshis);
		if is_flexible {
			self.flexible_securitization_locked.saturating_accrue(collateral_required);
			self.flexible_ratio_adjusted_satoshis.saturating_accrue(eligible_satoshis);
		} else {
			// This reclassifies an existing flexible lock as regular, so its amounts must be
			// present in the flexible totals before they are removed below.
			ensure!(
				collateral_required <= self.flexible_securitization_locked &&
					eligible_satoshis <= self.flexible_ratio_adjusted_satoshis,
				VaultError::InternalError
			);
			let regular_securitization_locked =
				self.regular_securitization_locked().saturating_add(collateral_required);
			ensure!(
				regular_securitization_locked.saturating_add(self.reserved_securitization_space) <=
					self.securitization,
				VaultError::InsufficientVaultFunds
			);
			self.flexible_securitization_locked.saturating_reduce(collateral_required);
			self.flexible_ratio_adjusted_satoshis.saturating_reduce(eligible_satoshis);
		}
		self.debug_assert_invariants_at("set_bitcoin_lock_flexible:after");
		Ok(())
	}

	pub fn burn(
		&mut self,
		securitization: &BitcoinSecuritization<Balance>,
		funded_satoshis: Satoshis,
		market_rate: Balance,
		lock_extension: &LockExtension<Balance>,
		is_flexible: bool,
	) -> Result<BurnResult<Balance>, VaultError> {
		let collateral_required = securitization.collateral_required();
		let burn_amount = collateral_required.min(market_rate);
		if burn_amount > self.securitization || burn_amount > self.securitization_locked {
			return Err(VaultError::InsufficientVaultFunds);
		}
		if is_flexible && collateral_required > self.flexible_securitization_locked {
			return Err(VaultError::InternalError);
		}
		if is_flexible {
			let securitization_after_burn = self.securitization.saturating_sub(burn_amount);
			let securitization_space_after_burn =
				securitization_after_burn.saturating_sub(self.regular_securitization_locked());
			self.reserved_securitization_space =
				self.reserved_securitization_space.min(securitization_space_after_burn);
		}
		self.securitization.saturating_reduce(burn_amount);
		self.securitization_locked.saturating_reduce(burn_amount);
		if is_flexible {
			self.flexible_securitization_locked.saturating_reduce(burn_amount);
		}

		let amount_to_future_release = collateral_required.saturating_sub(burn_amount);
		let release_height = self.schedule_release(
			amount_to_future_release,
			funded_satoshis,
			securitization.eligible_satoshis(funded_satoshis),
			lock_extension,
			is_flexible,
		)?;

		self.debug_assert_invariants_at("burn:after");

		Ok(BurnResult {
			burned_amount: burn_amount,
			held_for_release: amount_to_future_release,
			release_heights: release_height,
		})
	}

	/// Reserve collateral for a full term while the underlying Bitcoin is pending.
	pub fn reserve_securitization(
		&mut self,
		securitization: &BitcoinSecuritization<Balance>,
		may_use_flexible_space: bool,
	) -> Result<(), VaultError> {
		let collateral_required = securitization.collateral_required();
		ensure!(
			collateral_required <= self.available_securitization_space(may_use_flexible_space),
			VaultError::InsufficientVaultFunds
		);

		let remaining = self.use_relockable_securitization(collateral_required, None);
		self.securitization_locked.saturating_accrue(remaining);
		self.securitization_pending_activation.saturating_accrue(collateral_required);
		self.debug_assert_invariants_at("reserve_securitization:after");

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
		securitization: &BitcoinSecuritization<Balance>,
		lock_extension: &mut LockExtension<Balance>,
		is_flexible: bool,
		may_use_flexible_space: bool,
	) -> Result<(), VaultError> {
		let collateral_required = securitization.collateral_required();
		// Flexible extensions replace capacity that can already back reservations.
		let available_securitization = if is_flexible {
			self.securitization_space().saturating_sub(self.flexible_securitization_locked)
		} else {
			self.available_securitization_space(may_use_flexible_space)
		};
		ensure!(
			collateral_required <= available_securitization,
			VaultError::InsufficientVaultFunds
		);

		// 1. Use the relockable argons within the max expiration
		let mut remaining = self.use_relockable_securitization(
			collateral_required,
			Some(lock_extension.lock_expiration),
		);

		// 2. Use any available *unlocked* securitization (exclude anything still scheduled for
		//    release).
		// `available_securitization()` includes scheduled-for-release amounts, but step (2) must
		// not consume those, because step (3) handles scheduled funds beyond the max expiration.
		let mut uninhibited_securitization = self.uninhibited_securitization();
		if may_use_flexible_space {
			uninhibited_securitization
				.saturating_accrue(self.undisplaced_flexible_securitization());
		}
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

		if is_flexible {
			self.flexible_securitization_locked.saturating_accrue(collateral_required);
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

	pub fn activate_securitization(
		&mut self,
		securitization: &BitcoinSecuritization<Balance>,
		funded_satoshis: Satoshis,
	) -> Result<(), VaultError> {
		let collateral_required = securitization.collateral_required();
		ensure!(
			collateral_required <= self.securitization_pending_activation,
			VaultError::InternalError
		);
		self.securitization_pending_activation.saturating_reduce(collateral_required);
		self.locked_satoshis.saturating_accrue(funded_satoshis);
		self.ratio_adjusted_satoshis
			.saturating_accrue(securitization.eligible_satoshis(funded_satoshis));
		self.debug_assert_invariants_at("activate_securitization:after");
		Ok(())
	}

	pub fn replace_securitization(
		&mut self,
		current: &BitcoinSecuritization<Balance>,
		replacement: &BitcoinSecuritization<Balance>,
		funded_satoshis: Satoshis,
		lock_extension: &mut LockExtension<Balance>,
		is_flexible: bool,
		may_use_flexible_space: bool,
	) -> Result<BTreeSet<BitcoinHeight>, VaultError> {
		if funded_satoshis == 0 {
			self.return_securitization(current)?;
			self.reserve_securitization(replacement, may_use_flexible_space)?;
			return Ok(BTreeSet::new());
		}

		let release_heights = self.schedule_securitization_release(
			current,
			funded_satoshis,
			lock_extension,
			is_flexible,
		)?;
		let mut replacement_extension = LockExtension::new(lock_extension.lock_expiration);
		self.extend_lock(
			replacement,
			&mut replacement_extension,
			is_flexible,
			may_use_flexible_space,
		)?;
		self.locked_satoshis.saturating_accrue(funded_satoshis);
		self.ratio_adjusted_satoshis
			.saturating_accrue(replacement.eligible_satoshis(funded_satoshis));
		if is_flexible {
			self.flexible_ratio_adjusted_satoshis
				.saturating_accrue(replacement.eligible_satoshis(funded_satoshis));
		}
		lock_extension.extended_expiration_funds = replacement_extension.extended_expiration_funds;
		self.debug_assert_invariants_at("replace_securitization:after");

		Ok(release_heights)
	}

	pub fn return_securitization(
		&mut self,
		securitization: &BitcoinSecuritization<Balance>,
	) -> Result<(), VaultError> {
		let collateral_required = securitization.collateral_required();
		ensure!(
			collateral_required <= self.securitization_pending_activation,
			VaultError::InternalError
		);
		self.securitization_pending_activation.saturating_reduce(collateral_required);
		self.securitization_locked.saturating_reduce(collateral_required);
		self.debug_assert_invariants_at("return_securitization:after");
		Ok(())
	}

	pub fn schedule_securitization_release(
		&mut self,
		securitization: &BitcoinSecuritization<Balance>,
		funded_satoshis: Satoshis,
		lock_extension: &LockExtension<Balance>,
		is_flexible: bool,
	) -> Result<BTreeSet<BitcoinHeight>, VaultError> {
		self.schedule_release(
			securitization.collateral_required(),
			funded_satoshis,
			securitization.eligible_satoshis(funded_satoshis),
			lock_extension,
			is_flexible,
		)
	}

	fn schedule_release(
		&mut self,
		collateral_required: Balance,
		funded_satoshis: Satoshis,
		eligible_satoshis: Satoshis,
		lock_extension: &LockExtension<Balance>,
		is_flexible: bool,
	) -> Result<BTreeSet<BitcoinHeight>, VaultError> {
		let mut release_heights = BTreeSet::new();

		// Reschedule the amounts to be released by any delayed funds first, then any remaining at
		// the expiration of the lock.
		// Under the "count once" model, scheduled-for-release funds are no longer counted as
		// locked, so we must ensure we actually have enough locked collateral to move into the
		// schedule.
		ensure!(
			collateral_required <= self.securitization_locked,
			VaultError::InsufficientVaultFunds
		);
		self.securitization_locked.saturating_reduce(collateral_required);
		ensure!(
			funded_satoshis <= self.locked_satoshis &&
				eligible_satoshis <= self.ratio_adjusted_satoshis,
			VaultError::InternalError
		);
		self.locked_satoshis.saturating_reduce(funded_satoshis);
		self.ratio_adjusted_satoshis.saturating_reduce(eligible_satoshis);
		if is_flexible {
			ensure!(
				collateral_required <= self.flexible_securitization_locked &&
					eligible_satoshis <= self.flexible_ratio_adjusted_satoshis,
				VaultError::InternalError
			);
			self.flexible_securitization_locked.saturating_reduce(collateral_required);
			self.flexible_ratio_adjusted_satoshis.saturating_reduce(eligible_satoshis);
		}
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

		let remaining = collateral_required.saturating_sub(amount_in_lock_extension);
		if remaining > Balance::zero() {
			let expiration = lock_extension.expiration_day();
			release_heights.insert(expiration);
			Self::increment_scheduled_expiration(
				&mut self.securitization_release_schedule,
				remaining,
				&expiration,
			)?;
		}
		ensure!(
			self.regular_securitization_locked()
				.saturating_add(self.reserved_securitization_space) <=
				self.securitization,
			VaultError::InsufficientVaultFunds
		);

		self.debug_assert_invariants_at("schedule_release:after");
		Ok(release_heights)
	}

	pub fn securitized_amount(&self, amount: Balance) -> Balance {
		self.securitization_ratio.saturating_mul_int(amount)
	}

	/// The amount of securitization that can be re-locked (with expiration extended out)
	pub fn get_relock_capacity(&self) -> Balance {
		self.securitization_release_schedule.values().copied().sum()
	}

	/// The amount of release-scheduled securitization that remains committed after a given
	/// bitcoin block height horizon.
	pub fn get_relock_capacity_after(&self, min_release_height: BitcoinHeight) -> Balance {
		self.securitization_release_schedule
			.iter()
			.filter(|(height, _)| **height > min_release_height)
			.map(|(_, amount)| *amount)
			.sum()
	}

	pub fn available_securitization_space(&self, may_use_flexible_space: bool) -> Balance {
		let available =
			self.securitization_space().saturating_sub(self.reserved_securitization_space);
		if !may_use_flexible_space {
			return available.min(self.securitization.saturating_sub(self.securitization_locked));
		}

		available
	}

	pub fn uninhibited_securitization(&self) -> Balance {
		self.securitization
			.saturating_sub(self.securitization_locked)
			.saturating_sub(self.get_relock_capacity())
	}

	fn increment_scheduled_expiration(
		release_schedule: &mut BoundedBTreeMap<
			BitcoinHeight,
			Balance,
			ConstU32<MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES>,
		>,
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
		collateral_required: Balance,
		max_expiration: Option<BitcoinHeight>,
	) -> Balance {
		let mut remaining = collateral_required;
		let max_expiration = max_expiration.map(get_rounded_up_bitcoin_day_height);
		for (height, releasing_amount) in self.securitization_release_schedule.iter_mut() {
			if let Some(max_expiration) = max_expiration &&
				*height > max_expiration
			{
				continue;
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
		prelude::sp_arithmetic::{traits::One, FixedU128},
		Balance,
	};
	use polkadot_sdk::frame_support::assert_err;

	#[test]
	fn bitcoin_securitization_derives_value_collateral_and_eligible_capacity() {
		let securitization = BitcoinSecuritization {
			securitized_satoshis: 50_000_000,
			microgons_at_target_per_btc: 1_000_000u128,
			securitization_coverage_microgons: 500_000,
			securitization_ratio: FixedU128::from_rational(3u128, 2u128),
		};

		assert_eq!(securitization.btc_value_in_microgons(), 500_000);
		assert_eq!(securitization.collateral_required(), 750_000);
		assert_eq!(securitization.eligible_satoshis(25_000_000), 37_500_000);
		assert_eq!(securitization.eligible_satoshis(75_000_000), 75_000_000);
	}

	#[test]
	fn activating_securitization_tracks_funded_and_ratio_adjusted_satoshis() {
		let mut vault = default_vault(100, 1.0);
		let securitization = bitcoin_securitization(80);
		vault.reserve_securitization(&securitization, false).unwrap();

		vault.activate_securitization(&securitization, 100).unwrap();

		assert_eq!(vault.securitization_pending_activation, 0);
		assert_eq!(vault.locked_satoshis, 100);
		assert_eq!(vault.ratio_adjusted_satoshis, 80);
	}

	#[test]
	fn resecuritizing_funded_bitcoin_preserves_funded_satoshis() {
		let mut vault = default_vault(100, 1.0);
		let current = bitcoin_securitization(100);
		let replacement = bitcoin_securitization(80);
		let mut lock_extension = LockExtension::new(100);
		vault.reserve_securitization(&current, false).unwrap();
		vault.activate_securitization(&current, 100).unwrap();

		vault
			.replace_securitization(&current, &replacement, 100, &mut lock_extension, false, false)
			.unwrap();

		assert_eq!(vault.securitization_locked, 80);
		assert_eq!(vault.locked_satoshis, 100);
		assert_eq!(vault.ratio_adjusted_satoshis, 80);
		assert_eq!(vault.get_relock_capacity(), 20);
	}

	#[test]
	fn test_locking_and_releasing_funds() {
		let mut vault = default_vault(100, 1.0);
		vault.reserve_securitization(&securitization(50), true).unwrap();
		assert_eq!(vault.securitization_locked, 50);
		assert_eq!(vault.securitization_pending_activation, 50);

		vault.reserve_securitization(&securitization(30), true).unwrap();
		assert_eq!(vault.securitization_locked, 80);
		assert_eq!(vault.securitization_pending_activation, 80);

		vault.return_securitization(&securitization(20)).unwrap();
		assert_eq!(vault.securitization_locked, 60);
	}

	#[test]
	fn calculates_securitization() {
		let mut vault = default_vault(100, 2.0);
		let requested = BitcoinSecuritization {
			securitized_satoshis: 50,
			microgons_at_target_per_btc: SATOSHIS_PER_BITCOIN as Balance,
			securitization_coverage_microgons: 50,
			securitization_ratio: vault.securitization_ratio,
		};
		let excessive = BitcoinSecuritization {
			securitized_satoshis: 60,
			securitization_coverage_microgons: 60,
			..requested
		};
		assert_eq!(vault.get_activated_securitization(), 0);
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(vault.available_securitization_space(true), 100);
		assert_eq!(vault.securitized_amount(50), 100);

		assert_err!(
			vault.reserve_securitization(&excessive, true),
			VaultError::InsufficientVaultFunds
		);

		vault.reserve_securitization(&requested, true).unwrap();
		assert_eq!(vault.get_activated_securitization(), 0);
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(vault.available_securitization_space(true), 0);

		vault.securitization_pending_activation = 0;
		assert_eq!(vault.get_activated_securitization(), 100);

		let lock_extensions = &mut LockExtension::new(106);
		vault
			.schedule_securitization_release(&requested, 0, lock_extensions, false)
			.unwrap();
		assert_eq!(vault.get_relock_capacity(), 100);
		assert_eq!(vault.get_activated_securitization(), 0);
	}

	#[test]
	fn flexible_space_is_available_to_regular_locks() {
		let mut vault = default_vault(100, 1.0);
		vault.securitization_locked = 100;
		vault.flexible_securitization_locked = 100;

		assert_eq!(vault.available_securitization_space(false), 0);
		assert_eq!(vault.available_securitization_space(true), 100);

		vault.securitization_locked = 120;
		vault.securitization_pending_activation = 20;

		assert_eq!(vault.available_securitization_space(true), 80);
		vault.debug_assert_invariants();

		vault.securitization_locked = 100;
		vault.securitization_pending_activation = 20;
		vault.flexible_securitization_locked = 80;

		assert_eq!(vault.available_securitization_space(true), 80);
	}

	#[test]
	fn pending_regular_lock_does_not_reduce_funded_bitcoin_space() {
		let mut vault = default_vault(100, 1.0);
		vault.securitization_locked = 120;
		vault.securitization_pending_activation = 20;
		vault.flexible_securitization_locked = 100;
		vault.ratio_adjusted_satoshis = 100;
		vault.flexible_ratio_adjusted_satoshis = 100;

		assert_eq!(vault.effective_eligible_satoshis(), 100);

		vault.securitization_pending_activation = 0;
		vault.ratio_adjusted_satoshis = 120;

		assert_eq!(vault.effective_eligible_satoshis(), 100);
	}

	#[test]
	fn undisplaced_flexible_securitization_includes_pending_regular_locks() {
		let mut vault = default_vault(100, 1.0);
		vault.securitization_locked = 120;
		vault.securitization_pending_activation = 20;
		vault.flexible_securitization_locked = 100;

		assert_eq!(vault.undisplaced_flexible_securitization(), 80);

		vault.securitization_locked = 100;
		vault.securitization_pending_activation = 0;

		assert_eq!(vault.undisplaced_flexible_securitization(), 100);
	}

	#[test]
	fn projects_flexible_securitization_for_a_ratchet() {
		let mut vault = default_vault(93, 1.0);
		vault.securitization_locked = 124;
		vault.flexible_securitization_locked = 93;

		assert_eq!(vault.projected_flexible_securitization(62, 52), (83, 62));
	}

	#[test]
	fn operator_cannot_use_flexible_space_for_a_new_lock() {
		let mut vault = default_vault(100, 1.0);
		vault.securitization_locked = 100;
		vault.flexible_securitization_locked = 100;

		assert_err!(
			vault.reserve_securitization(&securitization(1), false),
			VaultError::InsufficientVaultFunds
		);
		vault.reserve_securitization(&securitization(100), true).unwrap();

		assert_eq!(vault.securitization_locked, 200);
		assert_eq!(vault.securitization_pending_activation, 100);
	}

	#[test]
	fn reservation_uses_unoccupied_space_and_blocks_all_locks() {
		let mut vault = default_vault(100, 1.0);
		vault.set_reserved_securitization_space(60).unwrap();

		vault.reserve_securitization(&securitization(40), false).unwrap();
		assert_err!(
			vault.reserve_securitization(&securitization(1), false),
			VaultError::InsufficientVaultFunds
		);

		let mut vault = default_vault(100, 1.0);
		vault.set_reserved_securitization_space(60).unwrap();

		vault.reserve_securitization(&securitization(40), true).unwrap();
		assert_err!(
			vault.reserve_securitization(&securitization(1), true),
			VaultError::InsufficientVaultFunds
		);
	}

	#[test]
	fn reservation_includes_unoccupied_and_flexible_securitization_space() {
		let mut vault = default_vault(100, 1.0);
		vault.securitization_locked = 50;
		vault.flexible_securitization_locked = 50;
		vault.locked_satoshis = 50;
		vault.ratio_adjusted_satoshis = 50;
		vault.flexible_ratio_adjusted_satoshis = 50;
		assert_err!(
			vault.set_reserved_securitization_space(101),
			VaultError::InsufficientVaultFunds
		);
		vault.set_reserved_securitization_space(100).unwrap();

		let mut reclassified_flexible_space = vault.clone();
		assert_err!(
			reclassified_flexible_space.set_bitcoin_lock_flexible(&securitization(50), 50, false,),
			VaultError::InsufficientVaultFunds
		);
		let mut released_flexible_space = vault.clone();
		released_flexible_space
			.schedule_securitization_release(
				&securitization(50),
				50,
				&LockExtension::new(100),
				true,
			)
			.unwrap();
		assert_eq!(released_flexible_space.flexible_securitization_locked, 0);
		assert_eq!(released_flexible_space.reserved_securitization_space, 100);

		vault.set_reserved_securitization_space(50).unwrap();
		vault.set_bitcoin_lock_flexible(&securitization(50), 50, false).unwrap();
		assert_eq!(vault.flexible_securitization_locked, 0);
		assert_eq!(vault.reserved_securitization_space, 50);
		assert_eq!(vault.securitization_space(), 50);
	}

	#[test]
	fn flexible_securitization_is_included_in_securitization_space() {
		let mut vault = default_vault(100, 1.0);
		vault.securitization_locked = 50;
		vault.flexible_securitization_locked = 50;
		vault.set_reserved_securitization_space(100).unwrap();

		assert_eq!(vault.securitization_space(), 100);
		assert_eq!(vault.available_securitization_space(true), 0);
		vault.set_reserved_securitization_space(50).unwrap();
		vault.reserve_securitization(&securitization(50), true).unwrap();

		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.securitization_space(), 50);
		assert_eq!(vault.available_securitization_space(true), 0);
	}

	#[test]
	fn funded_flexible_lifecycle_updates_flexible_totals() {
		let mut vault = default_vault(100, 1.0);
		vault.reserve_securitization(&securitization(100), false).unwrap();
		vault.activate_securitization(&securitization(100), 50).unwrap();
		vault.set_bitcoin_lock_flexible(&securitization(100), 50, true).unwrap();

		assert_eq!(vault.flexible_securitization_locked, 100);
		assert_eq!(vault.flexible_ratio_adjusted_satoshis, 50);
		assert_eq!(vault.ratio_adjusted_satoshis, 50);

		vault
			.schedule_securitization_release(
				&securitization(20),
				10,
				&LockExtension::new(100),
				true,
			)
			.unwrap();

		assert_eq!(vault.flexible_securitization_locked, 80);
		assert_eq!(vault.flexible_ratio_adjusted_satoshis, 40);
		assert_eq!(vault.ratio_adjusted_satoshis, 40);

		vault
			.extend_lock(&securitization(20), &mut LockExtension::new(200), true, true)
			.unwrap();

		assert_eq!(vault.flexible_securitization_locked, 100);
	}

	#[test]
	fn burning_funded_flexible_bitcoin_removes_flexible_totals() {
		let mut vault = default_vault(100, 1.0);
		vault.reserve_securitization(&securitization(100), false).unwrap();
		vault.activate_securitization(&securitization(100), 50).unwrap();
		vault.set_bitcoin_lock_flexible(&securitization(100), 50, true).unwrap();

		vault
			.burn(&securitization(100), 50, 50, &LockExtension::new(100), true)
			.unwrap();

		assert_eq!(vault.flexible_securitization_locked, 0);
		assert_eq!(vault.flexible_ratio_adjusted_satoshis, 0);
		assert_eq!(vault.ratio_adjusted_satoshis, 0);
	}

	#[test]
	fn burning_flexible_securitization_reduces_reserved_space() {
		let mut vault = default_vault(100, 1.0);
		vault.securitization_locked = 100;
		vault.flexible_securitization_locked = 50;
		vault.locked_satoshis = 75;
		vault.ratio_adjusted_satoshis = 75;
		vault.flexible_ratio_adjusted_satoshis = 25;
		vault.set_reserved_securitization_space(50).unwrap();

		vault.burn(&securitization(50), 25, 25, &LockExtension::new(100), true).unwrap();

		assert_eq!(vault.securitization, 75);
		assert_eq!(vault.securitization_locked, 50);
		assert_eq!(vault.flexible_securitization_locked, 0);
		assert_eq!(vault.reserved_securitization_space, 25);
	}

	#[test]
	fn can_burn() {
		let mut vault = default_vault(100, 1.0);

		vault.reserve_securitization(&securitization(100), true).unwrap();
		assert_eq!(vault.securitized_amount(50), 50);
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(vault.available_securitization_space(true), 0);
		assert_eq!(vault.securitization_locked, 100);
		vault.securitization_pending_activation = 0;

		let lock_extensions = &mut LockExtension::new(365);
		let burn_result = vault.burn(&securitization(100), 0, 50, lock_extensions, false).unwrap();
		assert_eq!(burn_result.burned_amount, 50);
		assert_eq!(burn_result.held_for_release, 50);
		assert_eq!(burn_result.release_heights.len(), 1);
		assert_eq!(vault.securitization_locked, 0);
		assert_eq!(vault.securitization_release_schedule.len(), 1);
		assert_eq!(vault.securitization_release_schedule.get(&432).unwrap(), &50);
		assert_eq!(vault.securitization, 50);
		assert_eq!(vault.available_securitization_space(true), 50);
	}

	#[test]
	fn handles_schedule_for_release() {
		let mut vault = default_vault(500, 1.0);
		vault.reserve_securitization(&securitization(100), true).unwrap();
		vault.activate_securitization(&securitization(100), 0).unwrap();

		let lock_extensions = &mut LockExtension::new(100);
		let release_heights = vault
			.schedule_securitization_release(&securitization(100), 0, lock_extensions, false)
			.unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.securitization_release_schedule.len(), 1);
		assert_eq!(vault.securitization_release_schedule.get(&144).unwrap(), &100);
		assert_eq!(vault.securitization_locked, 0);
		assert_eq!(vault.available_securitization_space(true), 500);

		let lock_extensions = &mut LockExtension::new(100);
		vault.extend_lock(&securitization(100), lock_extensions, false, true).unwrap();
		assert_eq!(vault.securitization_release_schedule.len(), 0);
		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.available_securitization_space(true), 400);
		assert_eq!(vault.get_relock_capacity(), 0);

		vault.reserve_securitization(&securitization(100), true).unwrap();
		assert_eq!(vault.securitization_locked, 200);
		assert_eq!(vault.available_securitization_space(true), 300);
		assert_eq!(vault.get_relock_capacity(), 0);
		vault.activate_securitization(&securitization(100), 0).unwrap();

		// schedule multiple releases
		let lock_extensions = &mut LockExtension::new(250);
		let release_heights = vault
			.schedule_securitization_release(&securitization(50), 0, lock_extensions, false)
			.unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.securitization_locked, 150);
		assert_eq!(vault.securitization_release_schedule.len(), 1);
		assert_eq!(vault.securitization_release_schedule.get(&288).unwrap(), &50);
		assert_eq!(vault.get_relock_capacity(), 50);

		let lock_extensions = &mut LockExtension::new(255);
		let release_heights = vault
			.schedule_securitization_release(&securitization(25), 0, lock_extensions, false)
			.unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.securitization_locked, 125);
		assert_eq!(vault.securitization_release_schedule.len(), 1);
		assert_eq!(vault.securitization_release_schedule.get(&288).unwrap(), &75);
		assert_eq!(vault.get_relock_capacity(), 75);

		let lock_extensions = &mut LockExtension::new(300);
		let release_heights = vault
			.schedule_securitization_release(&securitization(25), 0, lock_extensions, false)
			.unwrap();
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
		vault.extend_lock(&securitization(25), lock_extensions, false, true).unwrap();
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
		assert_eq!(vault.available_securitization_space(true), 375);
		assert_eq!(vault.securitization_locked, 125);
		let lock_extensions = &mut LockExtension::new(143);
		vault.extend_lock(&securitization(370), lock_extensions, false, true).unwrap();
		assert_eq!(lock_extensions.len(), 2);
		assert_eq!(lock_extensions.get(&288).unwrap(), &75);
		assert_eq!(lock_extensions.get(&432).unwrap(), &20);
		assert_eq!(vault.get_relock_capacity(), 5);
		assert_eq!(vault.securitization_locked, 495);
		assert_eq!(vault.securitization_release_schedule.len(), 1);
		assert_eq!(vault.securitization_release_schedule.get(&432).unwrap(), &5);

		// now return the 370
		let result = vault
			.schedule_securitization_release(&securitization(370), 0, lock_extensions, false)
			.unwrap();
		assert_eq!(result.len(), 3);
		assert_eq!(result.iter().collect::<Vec<_>>(), vec![&144, &288, &432]);
		assert_eq!(vault.get_relock_capacity(), 375);
		assert_eq!(vault.securitization_locked, 125);
		assert_eq!(vault.securitization_release_schedule.len(), 3);
		assert_eq!(vault.securitization_release_schedule.get(&144).unwrap(), &275);
		assert_eq!(vault.securitization_release_schedule.get(&288).unwrap(), &75);
		assert_eq!(vault.securitization_release_schedule.get(&432).unwrap(), &25);
	}

	fn securitization(amount: Satoshis) -> BitcoinSecuritization<Balance> {
		bitcoin_securitization(amount)
	}

	fn bitcoin_securitization(amount: Satoshis) -> BitcoinSecuritization<Balance> {
		BitcoinSecuritization {
			securitized_satoshis: amount,
			microgons_at_target_per_btc: SATOSHIS_PER_BITCOIN as Balance,
			securitization_coverage_microgons: amount.into(),
			securitization_ratio: FixedU128::one(),
		}
	}

	fn default_vault(securitization: Balance, ratio: f64) -> Vault<u64, Balance> {
		Vault::<u64, Balance> {
			operator_account_id: 0,
			delegate_account_id: None,
			securitization,
			securitization_target: securitization,
			securitization_locked: 0,
			flexible_securitization_locked: 0,
			reserved_securitization_space: 0,
			securitization_pending_activation: 0,
			locked_satoshis: 0,
			ratio_adjusted_satoshis: 0,
			flexible_ratio_adjusted_satoshis: 0,
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
			operational_minimum_release_tick: None,
		}
	}
}
