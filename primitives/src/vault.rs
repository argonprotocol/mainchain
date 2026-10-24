use alloc::{
	collections::{BTreeMap, BTreeSet},
	vec::Vec,
};
use codec::{Codec, Decode, DecodeWithMemTracking, Encode, HasCompact, MaxEncodedLen};
use core::iter::Sum;
use frame_support::{weights::Weight, PalletError};
use polkadot_sdk::{sp_core::ConstU32, sp_runtime::BoundedBTreeMap, *};
use scale_info::TypeInfo;
use sp_arithmetic::{
	helpers_128bit::multiply_by_rational_with_rounding, FixedPointNumber, FixedU128, Permill,
	Rounding,
};
use sp_core::blake2_256;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, SaturatedConversion, Saturating, Verify, Zero},
	AccountId32,
};

use crate::{
	bitcoin::{
		get_rounded_up_bitcoin_day_height, BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinLockId,
		BitcoinXPub, CompressedBitcoinPubkey, Satoshis, SATOSHIS_PER_BITCOIN,
	},
	ensure,
	prelude::FrameId,
	tick::Tick,
	Signature, VaultId,
};

pub const MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES: u32 = 366;

pub trait BitcoinVaultProviderWeightInfo {
	fn is_owner() -> Weight;
	fn get_registration_vault_data() -> Weight;
	fn get_committed_securitization() -> Weight;
	fn get_held_argonots() -> Weight;
	fn encumber_argonots() -> Weight;
	fn release_encumbered_argonots() -> Weight;
	fn burn_encumbered_argonots() -> Weight;
	fn account_became_operational() -> Weight;
	fn set_bitcoin_lock_flexible() -> Weight;
	fn reserve_securitization() -> Weight;
	fn resecuritize() -> Weight;
	fn burn() -> Weight;
}

impl BitcoinVaultProviderWeightInfo for () {
	fn is_owner() -> Weight {
		Weight::zero()
	}
	fn get_registration_vault_data() -> Weight {
		Weight::zero()
	}

	fn get_committed_securitization() -> Weight {
		Weight::zero()
	}

	fn get_held_argonots() -> Weight {
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

	fn reserve_securitization() -> Weight {
		Weight::zero()
	}

	fn resecuritize() -> Weight {
		Weight::zero()
	}

	fn burn() -> Weight {
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultSecuritization<Balance, AccountId> {
	pub vault_id: VaultId,
	pub operator_account_id: AccountId,
	pub securitization: Balance,
	/// Securitization activated by confirmed Bitcoin funding.
	pub activated_securitization: Balance,
	/// Confirmed Bitcoin locked in this vault.
	pub bitcoin_locked_satoshis: Satoshis,
	/// Confirmed Bitcoin backed by activated securitization.
	pub securitized_satoshis: Satoshis,
	/// Argonot securitization held for this vault, denominated in micronots.
	pub securitization_micronots: Balance,
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
pub struct VaultArgonotSecuritization<Balance>
where
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	/// Total Argonots held as vault securitization, denominated in micronots.
	#[codec(compact)]
	pub held_micronots: Balance,

	/// Argonots used in rewards that require one-year withdrawal notice, including pending exits.
	#[codec(compact)]
	pub committed_micronots: Balance,

	/// Amount of argonots held for cross-chain transfer collateral, denominated in micronots.
	#[codec(compact)]
	pub encumbered_micronots: Balance,
}

pub type VaultSecuritizationRanking<Balance, AccountId> =
	(Vec<VaultSecuritization<Balance, AccountId>>, Balance);

/// Public participation capacity in an open vault, denominated in microgons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultParticipationCapacity<Balance> {
	/// Securitization available for a new regular Bitcoin Lock, including replaceable flexible
	/// securitization and respecting reserved space and withdrawal deadlines.
	pub available_securitization_space: Balance,
	/// Total regular-bond admission capacity after withdrawal notices. Treasury subtracts
	/// existing regular bonds and reserved space. Flexible bonds can be displaced by new
	/// purchases.
	pub regular_bond_capacity: Balance,
}

pub trait TreasuryVaultProviderWeightInfo {
	fn get_vault_operator() -> Weight {
		Weight::zero()
	}
	fn get_participation_capacity() -> Weight {
		Weight::zero()
	}
	fn get_top_vaults_by_securitization(vaults: u32) -> Weight;
	fn commit_securitization_for_bonds() -> Weight;
	fn commit_securitization_for_rewards() -> Weight;
	fn record_vault_frame_earnings() -> Weight;
}

impl TreasuryVaultProviderWeightInfo for () {
	fn get_top_vaults_by_securitization(_vaults: u32) -> Weight {
		Weight::zero()
	}

	fn commit_securitization_for_bonds() -> Weight {
		Weight::zero()
	}

	fn commit_securitization_for_rewards() -> Weight {
		Weight::zero()
	}

	fn record_vault_frame_earnings() -> Weight {
		Weight::zero()
	}
}

pub trait TreasuryVaultProvider {
	type Weights: TreasuryVaultProviderWeightInfo;
	type Balance: Codec;
	type AccountId: Codec;

	/// Public admission capacity for upstream participation; unavailable vaults return `None`.
	fn get_participation_capacity(
		_vault_id: VaultId,
	) -> Option<VaultParticipationCapacity<Self::Balance>> {
		None
	}
	/// Get raw Argon securitization for an open vault.
	fn get_vault_securitization(vault_id: VaultId) -> Option<Self::Balance>;
	/// Put securitization backing regular bonds into the normal withdrawal-notice flow.
	fn commit_securitization_for_bonds(
		vault_id: VaultId,
		regular_bond_microgons: Self::Balance,
	) -> Result<(), VaultError>;
	/// Get the largest open vault positions and the total across every open vault.
	fn get_top_vaults_by_securitization(
		max_vaults: u32,
	) -> VaultSecuritizationRanking<Self::Balance, Self::AccountId>;

	/// Apply the withdrawal-notice requirement to securitization used in a reward snapshot.
	fn commit_securitization_for_rewards(vault_id: VaultId, micronots: Self::Balance);

	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId>;
	fn get_vault_delegate(vault_id: VaultId) -> Option<Self::AccountId>;

	/// Ensure a vault is open
	fn is_vault_open(vault_id: VaultId) -> bool;

	/// Records the earnings for a vault frame
	fn record_vault_frame_earnings(
		source_account_id: &Self::AccountId,
		profit: VaultTreasuryFrameEarnings<Self::Balance, Self::AccountId>,
	) -> sp_runtime::DispatchResult;
}

#[derive(
	Clone,
	Copy,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct BitcoinSecuritizationBasis<Balance>
where
	Balance: Codec + MaxEncodedLen,
{
	/// Satoshis used to size the securitization.
	#[codec(compact)]
	pub satoshis: Satoshis,
	/// Microgon value per BTC if Argon were trading at its target price.
	#[codec(compact)]
	pub microgons_at_target_per_btc: Balance,
	/// Tick of the validated BTC basis in price history.
	#[codec(compact)]
	pub tick: Tick,
}

impl<Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned>
	BitcoinSecuritizationBasis<Balance>
{
	pub fn btc_value_in_microgons(&self) -> Balance {
		FixedU128::from_rational(self.satoshis as u128, SATOSHIS_PER_BITCOIN as u128)
			.saturating_mul_int(self.microgons_at_target_per_btc)
	}
}

/// Purchased cosigner coverage at a recorded Bitcoin basis, owned by one Lock or Fission.
#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct CosignerGuarantee<Balance>
where
	Balance: Codec + MaxEncodedLen,
{
	pub basis: BitcoinSecuritizationBasis<Balance>,
	/// Paid coverage in microgons, preserved when guarantees are split or merged.
	#[codec(compact)]
	pub guaranteed_microgons: Balance,
	/// Allocated securitization amounts and their release dates beyond the Bitcoin Lock's expiry.
	pub fund_hold_extensions: BoundedBTreeMap<
		BitcoinHeight,
		Balance,
		ConstU32<MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES>,
	>,
}

impl<Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned>
	CosignerGuarantee<Balance>
{
	pub fn securitized_satoshis(&self, funded_satoshis: Satoshis) -> Satoshis {
		funded_satoshis.min(self.basis.satoshis)
	}

	/// Securitization allocated to the given satoshis, capped at the guarantee and rounded down.
	pub fn securitization_for_satoshis(&self, satoshis: Satoshis) -> Balance {
		if self.basis.satoshis == 0 {
			return Balance::zero();
		}
		if satoshis >= self.basis.satoshis {
			return self.guaranteed_microgons;
		}

		let amount = multiply_by_rational_with_rounding(
			self.guaranteed_microgons.saturated_into::<u128>(),
			satoshis.into(),
			self.basis.satoshis.into(),
			Rounding::Down,
		)
		.expect("a proportional portion cannot exceed its original guarantee");
		Balance::saturated_from(amount)
	}

	pub fn unactivated_securitization(&self, funded_satoshis: Satoshis) -> Balance {
		self.guaranteed_microgons
			.saturating_sub(self.securitization_for_satoshis(funded_satoshis))
	}

	/// Assign securitization to its hold extensions, then the Bitcoin Lock's own expiry.
	pub fn collateral_expirations(
		&self,
		lock_expiration: BitcoinHeight,
		collateral: Balance,
	) -> impl Iterator<Item = (BitcoinHeight, Balance)> + '_ {
		let mut remaining = collateral;
		self.fund_hold_extensions
			.iter()
			.map(|(height, amount)| (*height, *amount))
			.chain(core::iter::once((
				get_rounded_up_bitcoin_day_height(lock_expiration),
				collateral,
			)))
			.filter_map(move |(height, amount)| {
				let amount = remaining.min(amount);
				remaining.saturating_reduce(amount);
				(!amount.is_zero()).then_some((height, amount))
			})
	}

	/// Take part of this guarantee, carrying its securitization hold extensions.
	pub fn take_portion(
		&mut self,
		satoshis: Satoshis,
		guaranteed_microgons: Balance,
	) -> Option<Self> {
		let remaining_satoshis = self.basis.satoshis.checked_sub(satoshis)?;
		let remaining_microgons = self.guaranteed_microgons.checked_sub(&guaranteed_microgons)?;

		// The taken amount carries the earliest release holds; the remainder keeps the rest.
		let mut extensions = self.fund_hold_extensions.clone();
		let mut to_assign = guaranteed_microgons;
		for ((_, remaining), (_, assigned)) in
			self.fund_hold_extensions.iter_mut().zip(extensions.iter_mut())
		{
			*assigned = (*remaining).min(to_assign);
			remaining.saturating_reduce(*assigned);
			to_assign.saturating_reduce(*assigned);
		}

		extensions.retain(|_, amount| !amount.is_zero());
		self.fund_hold_extensions.retain(|_, amount| !amount.is_zero());

		self.guaranteed_microgons = remaining_microgons;
		self.basis.satoshis = remaining_satoshis;

		Some(Self {
			basis: BitcoinSecuritizationBasis { satoshis, ..self.basis },
			guaranteed_microgons,
			fund_hold_extensions: extensions,
		})
	}
}

/// Absolute funding and activated securitization contributed to a Vault.
/// Replacements supply both the previous and new contribution; these are not deltas.
#[derive(Clone, Copy, Default)]
pub struct BitcoinLockFunding<Balance> {
	/// Confirmed Bitcoin in the affected contribution.
	pub funded_satoshis: Satoshis,
	/// Confirmed Bitcoin covered by the affected guarantee(s).
	pub securitized_satoshis: Satoshis,
	/// Activated securitization in the affected contribution.
	pub activated_securitization: Balance,
	/// Whether the Lock uses flexible vault securitization.
	pub is_flexible: bool,
}

pub struct ReserveSecuritizationRequest<Balance> {
	pub remaining_term: FixedU128,
	pub is_flexible: bool,
	/// Fee coupon value supplied by the Lock owner.
	pub fee_discount: Balance,
	/// Full Bitcoin commitment expiry for this reservation.
	pub lock_expiration: BitcoinHeight,
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
	fn get_held_argonots(account_id: &Self::AccountId) -> Option<Self::Balance>;
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

	/// Replace the affected funding and coverage contribution. Unchanged positions are omitted.
	fn record_bitcoin_lock_funding(
		vault_id: VaultId,
		previous_funding: BitcoinLockFunding<Self::Balance>,
		updated_funding: BitcoinLockFunding<Self::Balance>,
	) -> Result<(), VaultError>;

	/// Reclassify the activated securitization of a Lock and its Fissions in the Vault's flexible
	/// total.
	fn set_bitcoin_lock_flexible(
		vault_id: VaultId,
		activated_securitization: Self::Balance,
		is_flexible: bool,
	) -> Result<(), VaultError>;

	/// Allocate vault securitization to a Bitcoin Lock, pending confirmation of Bitcoin funding.
	fn reserve_securitization(
		vault_id: VaultId,
		locker: &Self::AccountId,
		securitization: &CosignerGuarantee<Self::Balance>,
		request: ReserveSecuritizationRequest<Self::Balance>,
	) -> Result<(Self::Balance, Self::Balance), VaultError>;

	/// Replace the cosigner guarantee; Bitcoin stays in the Lock.
	fn resecuritize(
		_vault_id: VaultId,
		_locker: &Self::AccountId,
		_existing_guarantee: &CosignerGuarantee<Self::Balance>,
		_replacement_guarantee: &mut CosignerGuarantee<Self::Balance>,
		_funded_satoshis: Satoshis,
		_securitization_request: ReserveSecuritizationRequest<Self::Balance>,
	) -> Result<(Self::Balance, Self::Balance), VaultError> {
		Err(VaultError::InternalError)
	}

	/// Release excess securitization after its funding is reconciled. With
	/// `release_unused_immediately`, only hold extensions remain scheduled; otherwise all release
	/// dates are preserved.
	fn shrink_securitization(
		_vault_id: VaultId,
		_released_guarantee: &CosignerGuarantee<Self::Balance>,
		_lock_expiration: BitcoinHeight,
		_release_unused_immediately: bool,
	) -> Result<(), VaultError> {
		Err(VaultError::InternalError)
	}

	/// Release a Bitcoin Lock's allocated securitization, preserving its release restrictions.
	/// Unactivated securitization without hold extensions is released immediately.
	fn release_bitcoin_lock_securitization(
		vault_id: VaultId,
		securitization: &CosignerGuarantee<Self::Balance>,
		funded_satoshis: Satoshis,
		lock_expiration: BitcoinHeight,
		is_flexible: bool,
	) -> Result<(), VaultError>;

	/// Burn vault securitization if a vault moves a Bitcoin UTXO outside the system.
	/// It is assumed that the vault is in cahoots with the beneficiary.
	///
	/// Settle all terminating guarantees together. Each entry includes the confirmed Bitcoin
	/// represented by that position; the sum includes the whole Lock's funded Bitcoin once.
	/// Returns the amount of argons that were burned.
	fn burn(
		vault_id: VaultId,
		guarantees: &[(&CosignerGuarantee<Self::Balance>, Satoshis)],
		lock_expiration: BitcoinHeight,
		is_flexible: bool,
		market_rate: Self::Balance,
	) -> Result<Self::Balance, VaultError>;

	/// Pay the Bitcoin owner the lesser of the funded Lock's insurance and redemption amount
	/// when a full release is not cosigned.
	///
	/// Returns the amounts sent to the beneficiary and burned.
	fn compensate_lost_bitcoin(
		vault_id: VaultId,
		beneficiary: &Self::AccountId,
		securitization: &CosignerGuarantee<Self::Balance>,
		funded_satoshis: Satoshis,
		lock_expiration: BitcoinHeight,
		is_flexible: bool,
		redemption_amount: Self::Balance,
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
		lock_id: BitcoinLockId,
		should_remove: bool,
	) -> Result<(), VaultError>;

	/// Track an orphaned cosign request for a UTXO.
	fn update_orphan_cosign_list(
		vault_id: VaultId,
		lock_id: BitcoinLockId,
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
	/// Held Argonots cannot be reduced below the backing already encumbered elsewhere.
	ArgonotsBelowEncumberedBacking,
}

/// Daily commitment releases and operator withdrawals. Withdrawals can overlap either commitment.
#[derive(
	Clone,
	Copy,
	Default,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct SecuritizationScheduleEntry<Balance>
where
	Balance: HasCompact + MaxEncodedLen,
{
	/// Securitization allocated to Bitcoin guarantees until this entry's release height.
	#[codec(compact)]
	pub locked_commitments: Balance,
	/// Securitization released from guarantees but still held until this height; it can be reused.
	#[codec(compact)]
	pub relockable_commitments: Balance,
	/// Whole Argon withdrawal due at this height, denominated in microgons.
	#[codec(compact)]
	pub argon_withdrawals: Balance,
	/// Whole Argonot withdrawal due at this height, denominated in micronots.
	#[codec(compact)]
	pub argonot_withdrawals: Balance,
}

impl<Balance: HasCompact + MaxEncodedLen + Zero> SecuritizationScheduleEntry<Balance> {
	pub fn is_empty(&self) -> bool {
		self.locked_commitments.is_zero() &&
			self.relockable_commitments.is_zero() &&
			self.argon_withdrawals.is_zero() &&
			self.argonot_withdrawals.is_zero()
	}
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
	/// The securitization locked for bitcoin
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
	/// Confirmed satoshis currently backed by activated securitization.
	#[codec(compact)]
	pub securitized_satoshis: Satoshis,
	/// Total confirmed satoshis currently held across this Vault's Locks.
	#[codec(compact)]
	pub total_satoshis: Satoshis,
	/// Commitments and whole withdrawals grouped by their next Bitcoin day release boundary.
	pub securitization_release_schedule: BoundedBTreeMap<
		BitcoinHeight,
		SecuritizationScheduleEntry<Balance>,
		ConstU32<MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES>,
	>,
	/// Microgons used in reward snapshots that require one-year withdrawal notice.
	#[codec(compact)]
	pub committed_microgons: Balance,
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
			self.securitized_satoshis <= self.total_satoshis,
			"[{where_}] invariant failed: securitized satoshis ({:?}) exceed total satoshis ({:?})",
			self.securitized_satoshis,
			self.total_satoshis,
		);
		debug_assert!(
			self.exit_notice_amount() <= self.securitization,
			"[{where_}] exit requests exceed vault securitization",
		);
		debug_assert!(
			self.committed_microgons <= self.securitization,
			"[{where_}] reward commitment exceeds vault securitization",
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

	pub fn exit_notice_amount(&self) -> Balance {
		self.securitization_release_schedule
			.values()
			.map(|entry| entry.argon_withdrawals)
			.sum()
	}

	/// Check each withdrawal deadline crossed by the new securitization commitment.
	pub fn ensure_withdrawal_capacity(&self) -> Result<(), VaultError> {
		let mut committed = Balance::zero();
		let mut withdrawals = Balance::zero();
		for entry in self.securitization_release_schedule.values() {
			committed.saturating_accrue(entry.locked_commitments);
			committed.saturating_accrue(entry.relockable_commitments);
			withdrawals.saturating_accrue(entry.argon_withdrawals);
		}
		if withdrawals.is_zero() {
			return Ok(());
		}
		withdrawals = Balance::zero();
		for entry in self.securitization_release_schedule.values() {
			committed.saturating_reduce(entry.locked_commitments);
			committed.saturating_reduce(entry.relockable_commitments);
			if entry.argon_withdrawals.is_zero() {
				continue;
			}
			withdrawals.saturating_accrue(entry.argon_withdrawals);
			ensure!(
				committed <= self.securitization.saturating_sub(withdrawals),
				VaultError::InsufficientVaultFunds
			);
		}
		Ok(())
	}

	/// Update a portion of the Lock's securitization commitments, preserving later release dates.
	pub fn update_locked_commitments(
		&mut self,
		guarantee: &CosignerGuarantee<Balance>,
		lock_expiration: BitcoinHeight,
		mut offset: Balance,
		mut amount: Balance,
		accrue: bool,
	) -> Result<(), VaultError> {
		for (height, extended) in
			guarantee.collateral_expirations(lock_expiration, amount.saturating_add(offset))
		{
			let skipped = offset.min(extended);
			offset.saturating_reduce(skipped);
			let change = amount.min(extended.saturating_sub(skipped));
			if change.is_zero() {
				continue;
			}
			if accrue {
				self.scheduled_release(height)?.locked_commitments.saturating_accrue(change);
			} else if let Some(entry) = self.securitization_release_schedule.get_mut(&height) {
				entry.locked_commitments.saturating_reduce(change);
			}
			amount.saturating_reduce(change);
			if amount.is_zero() {
				break;
			}
		}
		self.securitization_release_schedule.retain(|_, entry| !entry.is_empty());
		Ok(())
	}

	pub fn undisplaced_flexible_securitization(&self) -> Balance {
		self.flexible_securitization_locked.min(self.securitization_space())
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
		activated_securitization: Balance,
		is_flexible: bool,
	) -> Result<(), VaultError> {
		if is_flexible {
			self.flexible_securitization_locked.saturating_accrue(activated_securitization);
		} else {
			// This reclassifies an existing flexible lock as regular, so its amounts must be
			// present in the flexible totals before they are removed below.
			ensure!(
				activated_securitization <= self.flexible_securitization_locked,
				VaultError::InternalError
			);
			let regular_securitization_locked =
				self.regular_securitization_locked().saturating_add(activated_securitization);
			ensure!(
				regular_securitization_locked.saturating_add(self.reserved_securitization_space) <=
					self.securitization,
				VaultError::InsufficientVaultFunds
			);
			self.flexible_securitization_locked.saturating_reduce(activated_securitization);
		}
		self.debug_assert_invariants_at("set_bitcoin_lock_flexible:after");
		Ok(())
	}

	pub fn burn(
		&mut self,
		guarantees: &[(&CosignerGuarantee<Balance>, Satoshis)],
		market_rate: Balance,
		lock_expiration: BitcoinHeight,
		is_flexible: bool,
	) -> Result<BurnResult<Balance>, VaultError> {
		let mut funded_satoshis = 0u64;
		let mut securitized_satoshis = 0u64;
		let mut securitization_microgons = Balance::zero();
		let mut allocated_microgons = Balance::zero();
		let mut hold_extensions = BTreeMap::<BitcoinHeight, Balance>::new();
		for (guarantee, guarantee_funded_satoshis) in guarantees {
			funded_satoshis.saturating_accrue(*guarantee_funded_satoshis);
			securitized_satoshis
				.saturating_accrue(guarantee.securitized_satoshis(*guarantee_funded_satoshis));
			securitization_microgons.saturating_accrue(
				guarantee.securitization_for_satoshis(*guarantee_funded_satoshis),
			);
			allocated_microgons.saturating_accrue(guarantee.guaranteed_microgons);
			for (height, amount) in &guarantee.fund_hold_extensions {
				hold_extensions.entry(*height).or_default().saturating_accrue(*amount);
			}
		}
		ensure!(funded_satoshis <= self.total_satoshis, VaultError::InternalError);
		ensure!(securitized_satoshis <= self.securitized_satoshis, VaultError::InternalError);

		// Retire the whole allocation before burning, just as when it was stored on the Lock.
		for (guarantee, _) in guarantees {
			self.update_locked_commitments(
				guarantee,
				lock_expiration,
				Balance::zero(),
				guarantee.guaranteed_microgons,
				false,
			)?;
		}
		let unactivated = allocated_microgons.saturating_sub(securitization_microgons);
		ensure!(unactivated <= self.securitization_pending_activation, VaultError::InternalError);

		// Return unfunded inherited holds to the existing release schedule before burning.
		// They stay available for settlement, while retaining their dates if not burned.
		let mut unactivated_hold_extensions = hold_extensions.clone();
		let mut activated_holds_to_skip = securitization_microgons;
		let mut held_unactivated = Balance::zero();
		unactivated_hold_extensions.retain(|_, amount| {
			let activated = (*amount).min(activated_holds_to_skip);
			activated_holds_to_skip.saturating_reduce(activated);
			amount.saturating_reduce(activated);
			held_unactivated.saturating_accrue(*amount);
			!amount.is_zero()
		});
		self.securitization_pending_activation.saturating_reduce(held_unactivated);
		let mut release_heights = self.schedule_release(
			held_unactivated,
			unactivated_hold_extensions.iter().map(|(height, amount)| (*height, *amount)),
			lock_expiration,
			false,
		)?;
		self.release_unactivated_securitization(unactivated.saturating_sub(held_unactivated))?;

		let burn_from_lock = securitization_microgons.min(market_rate);
		// A terminal Bitcoin spend can owe more than this Lock's securitization. Draw unused
		// vault securitization without taking amounts allocated to other Locks or reservations.
		let additional_available = self
			.securitization
			.saturating_sub(self.securitization_locked)
			.saturating_sub(self.reserved_securitization_space);
		let additional_burn = market_rate.saturating_sub(burn_from_lock).min(additional_available);
		let burn_amount = burn_from_lock.saturating_add(additional_burn);
		if burn_amount > self.securitization || burn_from_lock > self.securitization_locked {
			return Err(VaultError::InsufficientVaultFunds);
		}
		if is_flexible && securitization_microgons > self.flexible_securitization_locked {
			return Err(VaultError::InternalError);
		}
		let unscheduled_available = self
			.uninhibited_securitization()
			.saturating_sub(self.reserved_securitization_space);
		// Once unscheduled funds are exhausted, consume scheduled funds as well so they cannot
		// later be released or committed a second time.
		let mut scheduled_burn = additional_burn.saturating_sub(unscheduled_available);
		if !scheduled_burn.is_zero() {
			for (height, entry) in self.securitization_release_schedule.iter_mut() {
				let amount_to_burn = scheduled_burn.min(entry.relockable_commitments);
				if let Some(returned_hold) = unactivated_hold_extensions.get(height) {
					let previously_relockable =
						entry.relockable_commitments.saturating_sub(*returned_hold);
					held_unactivated
						.saturating_reduce(amount_to_burn.saturating_sub(previously_relockable));
				}
				entry.relockable_commitments.saturating_reduce(amount_to_burn);
				scheduled_burn.saturating_reduce(amount_to_burn);
				if scheduled_burn.is_zero() {
					break;
				}
			}
			ensure!(scheduled_burn.is_zero(), VaultError::InternalError);
			self.securitization_release_schedule.retain(|_, entry| !entry.is_empty());
		}
		if is_flexible {
			let securitization_after_burn = self.securitization.saturating_sub(burn_amount);
			let securitization_space_after_burn =
				securitization_after_burn.saturating_sub(self.regular_securitization_locked());
			self.reserved_securitization_space =
				self.reserved_securitization_space.min(securitization_space_after_burn);
		}
		self.securitization.saturating_reduce(burn_amount);
		self.committed_microgons = self.committed_microgons.min(self.securitization);
		self.cancel_securitization_exits(burn_amount);
		self.securitization_target = self.securitization_target.min(self.securitization);
		self.securitization_locked.saturating_reduce(burn_from_lock);
		if is_flexible {
			self.flexible_securitization_locked.saturating_reduce(securitization_microgons);
		}

		let amount_to_future_release = securitization_microgons.saturating_sub(burn_from_lock);
		self.securitized_satoshis.saturating_reduce(securitized_satoshis);
		release_heights.extend(self.schedule_release(
			amount_to_future_release,
			hold_extensions,
			lock_expiration,
			false,
		)?);
		self.total_satoshis.saturating_reduce(funded_satoshis);

		self.debug_assert_invariants_at("burn:after");

		Ok(BurnResult {
			burned_amount: burn_amount,
			held_for_release: amount_to_future_release.saturating_add(held_unactivated),
			release_heights,
		})
	}

	/// Reserve securitization for a full term while the Bitcoin is pending.
	pub fn reserve_securitization(
		&mut self,
		securitization: &CosignerGuarantee<Balance>,
		may_use_flexible_space: bool,
		lock_expiration: BitcoinHeight,
	) -> Result<(), VaultError> {
		let securitization_microgons = securitization.guaranteed_microgons;
		let available =
			self.available_securitization_space(may_use_flexible_space, Some(lock_expiration));
		ensure!(securitization_microgons <= available, VaultError::InsufficientVaultFunds);

		let remaining = self.use_relockable_securitization(securitization_microgons, None);
		self.securitization_locked.saturating_accrue(remaining);
		self.securitization_pending_activation
			.saturating_accrue(securitization_microgons);
		if !securitization_microgons.is_zero() {
			self.scheduled_release(get_rounded_up_bitcoin_day_height(lock_expiration))?
				.locked_commitments
				.saturating_accrue(securitization_microgons);
		}
		self.securitization_release_schedule.retain(|_, entry| !entry.is_empty());
		self.ensure_withdrawal_capacity()?;
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
		guarantee: &mut CosignerGuarantee<Balance>,
		lock_expiration: BitcoinHeight,
		is_flexible: bool,
		may_use_flexible_space: bool,
	) -> Result<(), VaultError> {
		let securitization_microgons = guarantee.guaranteed_microgons;
		// Flexible extensions replace capacity that can already back reservations.
		let available_securitization = if is_flexible {
			self.securitization_space().saturating_sub(self.flexible_securitization_locked)
		} else {
			self.available_securitization_space(may_use_flexible_space, None)
		};
		ensure!(
			securitization_microgons <= available_securitization,
			VaultError::InsufficientVaultFunds
		);

		// 1. Use the relockable argons within the max expiration
		let mut remaining =
			self.use_relockable_securitization(securitization_microgons, Some(lock_expiration));

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
			let max_expiration = get_rounded_up_bitcoin_day_height(lock_expiration);
			for (height, entry) in self.securitization_release_schedule.iter_mut() {
				let amount_to_use = remaining.min(entry.relockable_commitments);
				if amount_to_use.is_zero() {
					continue;
				}
				entry.relockable_commitments.saturating_reduce(amount_to_use);
				remaining.saturating_reduce(amount_to_use);
				self.securitization_locked.saturating_accrue(amount_to_use);

				if *height > max_expiration {
					Self::increment_scheduled_expiration(
						&mut guarantee.fund_hold_extensions,
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
			self.securitization_release_schedule.retain(|_, entry| !entry.is_empty());
		}

		if is_flexible {
			self.flexible_securitization_locked.saturating_accrue(securitization_microgons);
		}
		self.update_locked_commitments(
			guarantee,
			lock_expiration,
			Balance::zero(),
			securitization_microgons,
			true,
		)?;
		self.ensure_withdrawal_capacity()?;
		self.debug_assert_invariants_at("extend_lock:after");
		Ok(())
	}

	pub fn sweep_released(&mut self, block_height: BitcoinHeight) -> Balance {
		let mut released_securitization = Balance::zero();
		self.securitization_release_schedule.retain(|height, entry| {
			if *height <= block_height {
				released_securitization.saturating_accrue(entry.relockable_commitments);
				entry.locked_commitments = Balance::zero();
				entry.relockable_commitments = Balance::zero();
			}
			!entry.is_empty()
		});

		released_securitization
	}

	pub fn record_bitcoin_lock_funding(
		&mut self,
		previous_funding: BitcoinLockFunding<Balance>,
		updated_funding: BitcoinLockFunding<Balance>,
	) -> Result<(), VaultError> {
		ensure!(previous_funding.funded_satoshis <= self.total_satoshis, VaultError::InternalError);
		ensure!(
			previous_funding.securitized_satoshis <= self.securitized_satoshis,
			VaultError::InternalError
		);
		ensure!(
			previous_funding.activated_securitization <= self.get_activated_securitization(),
			VaultError::InternalError
		);

		let total_satoshis = self
			.total_satoshis
			.saturating_sub(previous_funding.funded_satoshis)
			.saturating_add(updated_funding.funded_satoshis);
		let securitized_satoshis = self
			.securitized_satoshis
			.saturating_sub(previous_funding.securitized_satoshis)
			.saturating_add(updated_funding.securitized_satoshis);
		let activated_securitization = self
			.get_activated_securitization()
			.saturating_sub(previous_funding.activated_securitization)
			.saturating_add(updated_funding.activated_securitization);

		ensure!(securitized_satoshis <= total_satoshis, VaultError::InternalError);
		ensure!(activated_securitization <= self.securitization_locked, VaultError::InternalError);
		if updated_funding.is_flexible {
			ensure!(
				previous_funding.activated_securitization <= self.flexible_securitization_locked,
				VaultError::InternalError
			);
		}

		self.total_satoshis = total_satoshis;
		self.securitized_satoshis = securitized_satoshis;
		self.securitization_pending_activation =
			self.securitization_locked.saturating_sub(activated_securitization);
		if updated_funding.is_flexible {
			self.flexible_securitization_locked = self
				.flexible_securitization_locked
				.saturating_sub(previous_funding.activated_securitization)
				.saturating_add(updated_funding.activated_securitization);
		}

		self.debug_assert_invariants_at("record_bitcoin_lock_funding:after");

		Ok(())
	}

	pub fn release_unactivated_securitization(
		&mut self,
		amount: Balance,
	) -> Result<(), VaultError> {
		ensure!(amount <= self.securitization_pending_activation, VaultError::InternalError);
		self.securitization_pending_activation.saturating_reduce(amount);
		self.securitization_locked.saturating_reduce(amount);
		self.debug_assert_invariants_at("release_unactivated_securitization:after");
		Ok(())
	}

	pub fn release_bitcoin_lock_securitization(
		&mut self,
		existing_guarantee: &CosignerGuarantee<Balance>,
		lock_funded_satoshis: Satoshis,
		lock_expiration: BitcoinHeight,
		is_flexible: bool,
	) -> Result<BTreeSet<BitcoinHeight>, VaultError> {
		ensure!(lock_funded_satoshis <= self.total_satoshis, VaultError::InternalError);

		let covered_satoshis = existing_guarantee.securitized_satoshis(lock_funded_satoshis);
		ensure!(covered_satoshis <= self.securitized_satoshis, VaultError::InternalError);

		self.securitized_satoshis.saturating_reduce(covered_satoshis);
		let release_heights = self.unwind_bitcoin_lock_securitization(
			existing_guarantee,
			existing_guarantee.unactivated_securitization(lock_funded_satoshis),
			lock_expiration,
			is_flexible,
		)?;

		self.total_satoshis.saturating_reduce(lock_funded_satoshis);
		self.debug_assert_invariants_at("release_bitcoin_lock_securitization:after");

		Ok(release_heights)
	}

	pub fn unwind_bitcoin_lock_securitization(
		&mut self,
		existing_guarantee: &CosignerGuarantee<Balance>,
		unactivated_microgons: Balance,
		lock_expiration: BitcoinHeight,
		is_flexible: bool,
	) -> Result<BTreeSet<BitcoinHeight>, VaultError> {
		ensure!(
			unactivated_microgons <= existing_guarantee.guaranteed_microgons,
			VaultError::InternalError
		);

		let activated_microgons =
			existing_guarantee.guaranteed_microgons.saturating_sub(unactivated_microgons);
		self.update_locked_commitments(
			existing_guarantee,
			lock_expiration,
			Balance::zero(),
			existing_guarantee.guaranteed_microgons,
			false,
		)?;

		// Existing release holds remain committed even when this guarantee was never funded.
		let extended_hold_microgons = existing_guarantee
			.fund_hold_extensions
			.values()
			.copied()
			.fold(Balance::zero(), |total, amount| total.saturating_add(amount));
		let held_unactivated_microgons = extended_hold_microgons
			.saturating_sub(activated_microgons)
			.min(unactivated_microgons);
		self.release_unactivated_securitization(
			unactivated_microgons.saturating_sub(held_unactivated_microgons),
		)?;
		self.securitization_pending_activation
			.saturating_reduce(held_unactivated_microgons);

		if is_flexible {
			ensure!(
				activated_microgons <= self.flexible_securitization_locked,
				VaultError::InternalError
			);
			self.flexible_securitization_locked.saturating_reduce(activated_microgons);
		}

		self.schedule_release(
			activated_microgons.saturating_add(held_unactivated_microgons),
			existing_guarantee
				.fund_hold_extensions
				.iter()
				.map(|(height, amount)| (*height, *amount)),
			lock_expiration,
			false,
		)
	}

	pub fn schedule_release(
		&mut self,
		securitization_microgons: Balance,
		hold_extensions: impl IntoIterator<Item = (BitcoinHeight, Balance)>,
		lock_expiration: BitcoinHeight,
		is_flexible: bool,
	) -> Result<BTreeSet<BitcoinHeight>, VaultError> {
		let mut release_heights = BTreeSet::new();

		// Move released securitization into reusable commitments, preserving hold extensions
		// before assigning the rest to the Lock's own expiry.
		ensure!(
			securitization_microgons <= self.securitization_locked,
			VaultError::InsufficientVaultFunds
		);
		self.securitization_locked.saturating_reduce(securitization_microgons);

		if is_flexible {
			ensure!(
				securitization_microgons <= self.flexible_securitization_locked,
				VaultError::InternalError
			);
			self.flexible_securitization_locked.saturating_reduce(securitization_microgons);
		}

		let mut remaining = securitization_microgons;
		for (height, amount) in hold_extensions.into_iter().chain(core::iter::once((
			get_rounded_up_bitcoin_day_height(lock_expiration),
			securitization_microgons,
		))) {
			let released = amount.min(remaining);
			if released.is_zero() {
				continue;
			}
			remaining.saturating_reduce(released);
			release_heights.insert(height);
			self.scheduled_release(height)?
				.relockable_commitments
				.saturating_accrue(released);
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

	/// The amount of securitization that can be re-locked (with expiration extended out)
	pub fn get_relock_capacity(&self) -> Balance {
		self.securitization_release_schedule
			.values()
			.map(|entry| entry.relockable_commitments)
			.sum()
	}

	/// Schedule a whole withdrawal. Allocated securitization keeps its Bitcoin maturity.
	pub fn request_securitization_exit(
		&mut self,
		amount: Balance,
		notice_height: BitcoinHeight,
	) -> Result<(), VaultError> {
		ensure!(
			amount <= self.securitization.saturating_sub(self.exit_notice_amount()),
			VaultError::InsufficientVaultFunds
		);
		let notice_height = get_rounded_up_bitcoin_day_height(notice_height);
		self.scheduled_release(notice_height)?
			.argon_withdrawals
			.saturating_accrue(amount);
		Ok(())
	}

	/// Cancel newest withdrawals first when the operator increases funding.
	pub fn cancel_securitization_exits(&mut self, amount: Balance) {
		let mut remaining = amount;
		for (_, entry) in self.securitization_release_schedule.iter_mut().rev() {
			let cancelled = remaining.min(entry.argon_withdrawals);
			entry.argon_withdrawals.saturating_reduce(cancelled);
			remaining.saturating_reduce(cancelled);
		}
		self.securitization_release_schedule.retain(|_, entry| !entry.is_empty());
	}

	/// Cancel newest Argonot withdrawals first when funding increases or backing is burned.
	pub fn cancel_argonot_exits(&mut self, amount: Balance) {
		let mut remaining = amount;
		for (_, entry) in self.securitization_release_schedule.iter_mut().rev() {
			let cancelled = remaining.min(entry.argonot_withdrawals);
			entry.argonot_withdrawals.saturating_reduce(cancelled);
			remaining.saturating_reduce(cancelled);
		}
		self.securitization_release_schedule.retain(|_, entry| !entry.is_empty());
	}

	/// Release only complete due entries, oldest first. A blocked entry remains unchanged.
	pub fn release_matured_securitization_exits(
		&mut self,
		current_height: BitcoinHeight,
		max_release: Balance,
	) -> Balance {
		let mut remaining = max_release;
		for (height, entry) in self.securitization_release_schedule.iter_mut() {
			if *height > current_height || entry.argon_withdrawals > remaining {
				break;
			}
			remaining.saturating_reduce(entry.argon_withdrawals);
			entry.argon_withdrawals = Balance::zero();
		}
		self.securitization_release_schedule.retain(|_, entry| !entry.is_empty());
		let released = max_release.saturating_sub(remaining);
		self.securitization.saturating_reduce(released);
		self.committed_microgons.saturating_reduce(released);
		released
	}

	/// The amount of release-scheduled securitization that remains committed after a given
	/// bitcoin block height horizon.
	pub fn get_relock_capacity_after(&self, min_release_height: BitcoinHeight) -> Balance {
		self.securitization_release_schedule
			.iter()
			.filter(|(height, _)| **height > min_release_height)
			.map(|(_, entry)| entry.relockable_commitments)
			.sum()
	}

	/// Collateral available after existing locks and reserved space. With a new lock's maturity,
	/// also limit it to the amount that can remain committed through scheduled withdrawals.
	/// Reservation and upstream participation use the same maturity-aware admission limit.
	pub fn available_securitization_space(
		&self,
		may_use_flexible_space: bool,
		lock_expiration: Option<BitcoinHeight>,
	) -> Balance {
		let mut available = self
			.securitization
			.saturating_sub(self.regular_securitization_locked())
			.saturating_sub(self.reserved_securitization_space);
		if !may_use_flexible_space {
			available =
				available.min(self.securitization.saturating_sub(self.securitization_locked));
		}
		let Some(lock_expiration) = lock_expiration else {
			return available;
		};

		let expiration_day = get_rounded_up_bitcoin_day_height(lock_expiration);
		let mut committed = Balance::zero();
		let mut relockable = Balance::zero();
		for entry in self.securitization_release_schedule.values() {
			committed.saturating_accrue(entry.locked_commitments);
			committed.saturating_accrue(entry.relockable_commitments);
			relockable.saturating_accrue(entry.relockable_commitments);
		}

		let total_relockable = relockable;
		let mut withdrawals = Balance::zero();
		for (height, entry) in &self.securitization_release_schedule {
			if *height >= expiration_day {
				break;
			}
			committed.saturating_reduce(entry.locked_commitments);
			committed.saturating_reduce(entry.relockable_commitments);
			relockable.saturating_reduce(entry.relockable_commitments);
			withdrawals.saturating_accrue(entry.argon_withdrawals);
			if entry.argon_withdrawals.is_zero() {
				continue;
			}

			let headroom =
				self.securitization.saturating_sub(withdrawals).saturating_sub(committed);
			let earlier_relockable = total_relockable.saturating_sub(relockable);
			// Earlier relockable funds must be reused before later ones. If they fit, the
			// later commitments can also be moved to this lock without increasing retention.
			let admissible = if headroom < earlier_relockable {
				headroom
			} else {
				headroom.saturating_add(relockable)
			};
			available = available.min(admissible);
		}

		// A full schedule must either already contain the new maturity or lose an entry
		// when the reservation consumes relockable securitization.
		if self.securitization_release_schedule.len() ==
			MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES as usize &&
			!self.securitization_release_schedule.contains_key(&expiration_day)
		{
			let mut remaining = available;
			let frees_entry = self.securitization_release_schedule.values().any(|entry| {
				let consumed = remaining.min(entry.relockable_commitments);
				remaining.saturating_reduce(consumed);
				!consumed.is_zero() &&
					consumed == entry.relockable_commitments &&
					entry.locked_commitments.is_zero() &&
					entry.argon_withdrawals.is_zero() &&
					entry.argonot_withdrawals.is_zero()
			});
			if !frees_entry {
				return Balance::zero();
			}
		}
		available
	}

	pub fn uninhibited_securitization(&self) -> Balance {
		self.securitization
			.saturating_sub(self.securitization_locked)
			.saturating_sub(self.get_relock_capacity())
	}

	pub fn increment_scheduled_expiration(
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

	/// Get or add a daily entry without exceeding the bounded schedule.
	pub fn scheduled_release(
		&mut self,
		height: BitcoinHeight,
	) -> Result<&mut SecuritizationScheduleEntry<Balance>, VaultError> {
		if !self.securitization_release_schedule.contains_key(&height) {
			self.securitization_release_schedule
				.try_insert(height, SecuritizationScheduleEntry::default())
				.map_err(|_| VaultError::InternalError)?;
		}
		self.securitization_release_schedule
			.get_mut(&height)
			.ok_or(VaultError::InternalError)
	}

	fn use_relockable_securitization(
		&mut self,
		securitization_microgons: Balance,
		max_expiration: Option<BitcoinHeight>,
	) -> Balance {
		let mut remaining = securitization_microgons;
		let max_expiration = max_expiration.map(get_rounded_up_bitcoin_day_height);
		for (height, entry) in self.securitization_release_schedule.iter_mut() {
			if let Some(max_expiration) = max_expiration &&
				*height > max_expiration
			{
				break;
			}
			let amount_to_use = remaining.min(entry.relockable_commitments);
			entry.relockable_commitments.saturating_reduce(amount_to_use);
			remaining.saturating_reduce(amount_to_use);
			self.securitization_locked.saturating_accrue(amount_to_use);
			if remaining.is_zero() {
				break;
			}
		}
		self.securitization_release_schedule.retain(|_, entry| !entry.is_empty());
		remaining
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::Balance;
	use alloc::collections::BTreeMap;
	use polkadot_sdk::frame_support::assert_err;

	#[test]
	fn guarantee_portions_preserve_microgons_and_release_maturities() {
		let mut guarantee = CosignerGuarantee {
			basis: BitcoinSecuritizationBasis {
				satoshis: 3,
				microgons_at_target_per_btc: 99u128,
				tick: 7,
			},
			guaranteed_microgons: 7,
			fund_hold_extensions: BoundedBTreeMap::try_from(BTreeMap::from([(144, 5), (288, 2)]))
				.unwrap(),
		};
		let original = guarantee.clone();
		assert_eq!(guarantee.take_portion(4, 2), None);
		assert_eq!(guarantee, original);
		assert_eq!(guarantee.take_portion(1, 8), None);
		assert_eq!(guarantee, original);

		let portion = guarantee.take_portion(1, guarantee.securitization_for_satoshis(1)).unwrap();
		assert_eq!((portion.basis.satoshis, portion.guaranteed_microgons), (1, 2));
		assert_eq!((guarantee.basis.satoshis, guarantee.guaranteed_microgons), (2, 5));
		assert_eq!(portion.basis.tick, 7);
		assert_eq!(portion.fund_hold_extensions[&144], 2);
		assert_eq!(guarantee.fund_hold_extensions[&144], 3);
		assert_eq!(guarantee.fund_hold_extensions[&288], 2);
		let rest = guarantee.take_portion(2, guarantee.securitization_for_satoshis(2)).unwrap();
		assert_eq!(rest.guaranteed_microgons + portion.guaranteed_microgons, 7);
		assert_eq!(guarantee.basis.satoshis, 0);
		assert_eq!(guarantee.guaranteed_microgons, 0);
	}

	#[test]
	fn guarantee_portions_round_once_after_multiplication() {
		let mut guarantee = CosignerGuarantee {
			basis: BitcoinSecuritizationBasis {
				satoshis: 75_000_000,
				microgons_at_target_per_btc: 62_000_000_000u128,
				tick: 1,
			},
			guaranteed_microgons: 46_500_000_000,
			fund_hold_extensions: Default::default(),
		};

		let created = guarantee
			.take_portion(25_000_000, guarantee.securitization_for_satoshis(25_000_000))
			.unwrap();
		assert_eq!(created.guaranteed_microgons, 15_500_000_000);
		assert_eq!(guarantee.guaranteed_microgons, 31_000_000_000);
	}

	#[test]
	fn bitcoin_securitization_derives_value_and_required_backing() {
		let securitization = CosignerGuarantee {
			basis: BitcoinSecuritizationBasis {
				satoshis: 50_000_000,
				microgons_at_target_per_btc: 1_000_000u128,
				tick: 0,
			},
			guaranteed_microgons: 500_000,
			fund_hold_extensions: Default::default(),
		};

		assert_eq!(securitization.basis.btc_value_in_microgons(), 500_000);
		assert_eq!(securitization.guaranteed_microgons, 500_000);
		assert_eq!(securitization.securitization_for_satoshis(25_000_000), 250_000);
		assert_eq!(securitization.securitization_for_satoshis(75_000_000), 500_000);
	}

	#[test]
	fn activating_securitization_tracks_only_covered_satoshis() {
		let mut vault = default_vault(100);
		let securitization = securitization(80);
		vault.reserve_securitization(&securitization, false, 100).unwrap();

		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(80, 80, false))
			.unwrap();

		assert_eq!(vault.securitization_pending_activation, 0);
		assert_eq!(vault.total_satoshis, 80);
		assert_eq!(vault.securitized_satoshis, 80);
	}

	#[test]
	fn funding_reduction_rejects_pending_overflow_without_mutation() {
		let mut vault = default_vault(u128::MAX);
		vault.securitization_locked = u128::MAX;
		vault.securitization_pending_activation = u128::MAX;
		let before = vault.clone();

		assert_err!(
			vault.record_bitcoin_lock_funding(funding_update(0, 1, false), Default::default()),
			VaultError::InternalError
		);
		assert_eq!(vault, before);
	}

	#[test]
	fn fractional_backing_remains_exact_across_partial_and_full_release() {
		let mut vault = default_vault(10);
		let securitization = CosignerGuarantee {
			basis: BitcoinSecuritizationBasis {
				satoshis: 3,
				microgons_at_target_per_btc: 2,
				tick: 0,
			},
			guaranteed_microgons: 2,
			fund_hold_extensions: Default::default(),
		};
		assert_eq!(securitization.securitization_for_satoshis(1), 0);
		assert_eq!(
			securitization.securitization_for_satoshis(3) -
				securitization.securitization_for_satoshis(1),
			2
		);
		vault.reserve_securitization(&securitization, false, 100).unwrap();
		let partial = funding_update(1, securitization.securitization_for_satoshis(1), false);
		let full = funding_update(3, securitization.securitization_for_satoshis(3), false);
		vault.record_bitcoin_lock_funding(Default::default(), partial).unwrap();
		vault.record_bitcoin_lock_funding(partial, full).unwrap();
		vault.record_bitcoin_lock_funding(full, partial).unwrap();
		assert_eq!(vault.securitization_pending_activation, 2);
		assert_eq!(vault.total_satoshis, 1);
		assert_eq!(vault.securitized_satoshis, 1);

		vault
			.release_bitcoin_lock_securitization(&securitization, 1, 100, false)
			.unwrap();

		assert_eq!(vault.total_satoshis, 0);
		assert_eq!(vault.securitized_satoshis, 0);
	}

	#[test]
	fn reducing_funding_preserves_the_contract_and_reclassifies_only_backed_satoshis() {
		let mut vault = default_vault(100);
		let securitization = securitization(100);
		vault.reserve_securitization(&securitization, false, 100).unwrap();
		let original =
			BitcoinLockFunding { funded_satoshis: 150, ..funding_update(100, 100, false) };
		let excess_withdrawn = BitcoinLockFunding { funded_satoshis: 120, ..original };
		let partially_funded = funding_update(50, 50, false);
		vault.record_bitcoin_lock_funding(Default::default(), original).unwrap();

		vault.record_bitcoin_lock_funding(original, excess_withdrawn).unwrap();
		assert_eq!(vault.total_satoshis, 120);
		assert_eq!(vault.securitized_satoshis, 100);
		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.securitization_pending_activation, 0);

		vault.record_bitcoin_lock_funding(excess_withdrawn, partially_funded).unwrap();
		assert_eq!(vault.total_satoshis, 50);
		assert_eq!(vault.securitized_satoshis, 50);
		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.securitization_pending_activation, 50);
	}

	#[test]
	fn withdrawals_release_whole_entries_in_order_and_cancel_without_moving_collateral() {
		let mut vault = default_vault(100);
		vault.scheduled_release(144).unwrap().relockable_commitments = 50;
		vault.request_securitization_exit(10, 100).unwrap();
		vault.request_securitization_exit(20, 120).unwrap();
		vault.request_securitization_exit(30, 200).unwrap();
		assert_eq!(vault.securitization_release_schedule[&144].argon_withdrawals, 30);
		assert_eq!(vault.securitization_release_schedule[&288].argon_withdrawals, 30);
		assert_eq!(vault.release_matured_securitization_exits(288, 29), 0);
		assert_eq!(vault.exit_notice_amount(), 60);
		assert_eq!(vault.release_matured_securitization_exits(288, 50), 30);
		assert_eq!(vault.securitization_release_schedule[&288].argon_withdrawals, 30);
		vault.cancel_securitization_exits(10);
		assert_eq!(vault.securitization_release_schedule[&288].argon_withdrawals, 20);
		assert_eq!(vault.get_relock_capacity(), 50);
		assert_eq!(vault.release_matured_securitization_exits(288, 20), 20);
		assert_eq!(vault.securitization, 50);
	}

	#[test]
	fn withdrawals_limit_only_commitments_that_outlive_each_deadline() {
		let mut vault = default_vault(100);
		vault.request_securitization_exit(50, 288).unwrap();
		vault.reserve_securitization(&securitization(25), false, 100).unwrap();
		vault.reserve_securitization(&securitization(25), false, 120).unwrap();
		vault.reserve_securitization(&securitization(50), false, 432).unwrap();
		assert_eq!(vault.securitization_release_schedule[&144].locked_commitments, 50);
		assert_eq!(vault.securitization_release_schedule[&432].locked_commitments, 50);
		assert_eq!(vault.securitization_locked, 100);
		let mut excessive = default_vault(100);
		excessive.request_securitization_exit(50, 288).unwrap();
		assert_err!(
			excessive.reserve_securitization(&securitization(51), false, 432),
			VaultError::InsufficientVaultFunds
		);

		let mut successive = default_vault(100);
		successive.request_securitization_exit(25, 288).unwrap();
		successive.request_securitization_exit(25, 432).unwrap();
		successive.scheduled_release(432).unwrap().relockable_commitments = 20;
		successive.reserve_securitization(&securitization(50), false, 576).unwrap();
		assert_err!(
			successive.reserve_securitization(&securitization(1), false, 576),
			VaultError::InsufficientVaultFunds
		);
	}

	#[test]
	fn new_lock_capacity_matches_admission_across_withdrawals_and_relockable_collateral() {
		for (early_relockable, later_relockable, withdrawal, expected) in
			[(0, 0, 100, 0), (0, 40, 60, 40), (40, 0, 60, 40), (20, 40, 40, 60)]
		{
			let mut vault = default_vault(100);
			vault.scheduled_release(144).unwrap().relockable_commitments = early_relockable;
			vault.scheduled_release(288).unwrap().argon_withdrawals = withdrawal;
			vault.scheduled_release(576).unwrap().relockable_commitments = later_relockable;
			assert_eq!(vault.available_securitization_space(true, Some(432)), expected);
			for amount in 1..=100 {
				let admitted =
					vault.clone().reserve_securitization(&securitization(amount), true, 432);
				assert_eq!(admitted.is_ok(), u128::from(amount) <= expected, "collateral {amount}");
			}
			// A lock maturing at the withdrawal deadline releases its commitment in time.
			assert_eq!(vault.available_securitization_space(true, Some(288)), 100);
		}
	}

	#[test]
	fn new_lock_capacity_requires_a_schedule_slot_or_reusable_entry() {
		let mut vault = default_vault(1_000);
		for day in 1..=MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES {
			vault.scheduled_release(u64::from(day) * 144).unwrap().argon_withdrawals = 1;
		}
		let expiration = u64::from(MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES + 1) * 144;
		assert_eq!(vault.available_securitization_space(true, Some(expiration)), 0);
		let original = vault.clone();
		assert_err!(
			vault.reserve_securitization(&securitization(1), true, expiration),
			VaultError::InsufficientVaultFunds
		);
		assert_eq!(vault, original);

		let first = vault.scheduled_release(144).unwrap();
		first.argon_withdrawals = 0;
		first.relockable_commitments = 10;
		let available = vault.available_securitization_space(true, Some(expiration));
		assert_eq!(available, 635);
		let original = vault.clone();
		assert_err!(
			vault.reserve_securitization(&securitization(636), true, expiration),
			VaultError::InsufficientVaultFunds
		);
		assert_eq!(vault, original);
		assert!(vault
			.reserve_securitization(&securitization(available as u64), true, expiration)
			.is_ok());
		assert!(vault.ensure_withdrawal_capacity().is_ok());
	}

	#[test]
	fn daily_schedule_keeps_commitments_and_withdrawals_independent() {
		let mut vault = default_vault(100);
		vault.reserve_securitization(&securitization(25), false, 100).unwrap();
		vault.reserve_securitization(&securitization(25), false, 100).unwrap();
		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(50, 50, false))
			.unwrap();
		vault.request_securitization_exit(20, 100).unwrap();
		vault
			.release_bitcoin_lock_securitization(&securitization(25), 25, 100, false)
			.unwrap();
		assert_eq!(
			vault.securitization_release_schedule[&144],
			SecuritizationScheduleEntry {
				locked_commitments: 25,
				relockable_commitments: 25,
				argon_withdrawals: 20,
				argonot_withdrawals: 0,
			}
		);
		let mut releasing = vault.clone();
		assert_eq!(releasing.sweep_released(144), 25);
		assert_eq!(
			releasing.securitization_release_schedule[&144],
			SecuritizationScheduleEntry { argon_withdrawals: 20, ..Default::default() }
		);

		vault.cancel_securitization_exits(10);
		vault.extend_lock(&mut securitization(25), 100, false, false).unwrap();
		assert_eq!(
			vault.securitization_release_schedule[&144],
			SecuritizationScheduleEntry {
				locked_commitments: 50,
				relockable_commitments: 0,
				argon_withdrawals: 10,
				argonot_withdrawals: 0,
			}
		);

		assert_eq!(vault.sweep_released(144), 0);
		assert_eq!(vault.release_matured_securitization_exits(144, 9), 0);
		assert_eq!(
			vault.securitization_release_schedule[&144],
			SecuritizationScheduleEntry {
				locked_commitments: 0,
				relockable_commitments: 0,
				argon_withdrawals: 10,
				argonot_withdrawals: 0,
			}
		);
		assert_eq!(vault.release_matured_securitization_exits(144, 10), 10);
		assert_eq!(vault.securitization, 90);
		assert!(vault.securitization_release_schedule.is_empty());
	}

	#[test]
	fn full_schedule_accepts_existing_days_but_rejects_new_days() {
		let mut vault =
			default_vault(Balance::from(MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES) + 1);
		for day in 1..=MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES {
			vault
				.scheduled_release(BitcoinHeight::from(day) * 144)
				.unwrap()
				.argon_withdrawals = 1;
		}
		assert_err!(
			vault.request_securitization_exit(
				1,
				BitcoinHeight::from(MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES + 1) * 144
			),
			VaultError::InternalError
		);
		vault.request_securitization_exit(1, 144).unwrap();
		assert_eq!(vault.securitization_release_schedule[&144].argon_withdrawals, 2);
		assert_eq!(
			vault.exit_notice_amount(),
			Balance::from(MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES) + 1
		);
	}

	#[test]
	fn test_locking_and_releasing_funds() {
		let mut vault = default_vault(100);
		vault.reserve_securitization(&securitization(50), true, 100).unwrap();
		assert_eq!(vault.securitization_locked, 50);
		assert_eq!(vault.securitization_pending_activation, 50);

		vault.reserve_securitization(&securitization(30), true, 100).unwrap();
		assert_eq!(vault.securitization_locked, 80);
		assert_eq!(vault.securitization_pending_activation, 80);

		vault.release_unactivated_securitization(20).unwrap();
		assert_eq!(vault.securitization_locked, 60);
	}

	#[test]
	fn calculates_securitization() {
		let mut vault = default_vault(100);
		let requested = CosignerGuarantee {
			basis: BitcoinSecuritizationBasis {
				satoshis: 50,
				microgons_at_target_per_btc: SATOSHIS_PER_BITCOIN as Balance,
				tick: 0,
			},
			guaranteed_microgons: 100,
			fund_hold_extensions: Default::default(),
		};
		let excessive = CosignerGuarantee {
			basis: BitcoinSecuritizationBasis { satoshis: 60, ..requested.basis },
			guaranteed_microgons: 120,
			fund_hold_extensions: Default::default(),
		};
		assert_eq!(vault.get_activated_securitization(), 0);
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(vault.available_securitization_space(true, None), 100);

		assert_err!(
			vault.reserve_securitization(&excessive, true, 100),
			VaultError::InsufficientVaultFunds
		);

		vault.reserve_securitization(&requested, true, 100).unwrap();
		assert_eq!(vault.get_activated_securitization(), 0);
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(vault.available_securitization_space(true, None), 0);

		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(50, 100, false))
			.unwrap();
		assert_eq!(vault.get_activated_securitization(), 100);

		vault.release_bitcoin_lock_securitization(&requested, 50, 106, false).unwrap();
		assert_eq!(vault.get_relock_capacity(), 100);
		assert_eq!(vault.get_activated_securitization(), 0);
	}

	#[test]
	fn flexible_space_is_available_to_regular_locks() {
		let mut vault = default_vault(100);
		vault.securitization_locked = 100;
		vault.flexible_securitization_locked = 100;

		assert_eq!(vault.available_securitization_space(false, None), 0);
		assert_eq!(vault.available_securitization_space(true, None), 100);

		vault.securitization_locked = 120;
		vault.securitization_pending_activation = 20;

		assert_eq!(vault.available_securitization_space(true, None), 80);
		vault.debug_assert_invariants();

		vault.securitization_locked = 100;
		vault.securitization_pending_activation = 20;
		vault.flexible_securitization_locked = 80;

		assert_eq!(vault.available_securitization_space(true, None), 80);
	}

	#[test]
	fn undisplaced_flexible_securitization_includes_pending_regular_locks() {
		let mut vault = default_vault(100);
		vault.securitization_locked = 120;
		vault.securitization_pending_activation = 20;
		vault.flexible_securitization_locked = 100;

		assert_eq!(vault.undisplaced_flexible_securitization(), 80);

		vault.securitization_locked = 100;
		vault.securitization_pending_activation = 0;

		assert_eq!(vault.undisplaced_flexible_securitization(), 100);
	}

	#[test]
	fn operator_cannot_use_flexible_space_for_a_new_lock() {
		let mut vault = default_vault(100);
		vault.securitization_locked = 100;
		vault.flexible_securitization_locked = 100;

		assert_err!(
			vault.reserve_securitization(&securitization(1), false, 100),
			VaultError::InsufficientVaultFunds
		);
		vault.reserve_securitization(&securitization(100), true, 100).unwrap();

		assert_eq!(vault.securitization_locked, 200);
		assert_eq!(vault.securitization_pending_activation, 100);
	}

	#[test]
	fn reservation_uses_unoccupied_space_and_blocks_all_locks() {
		let mut vault = default_vault(100);
		vault.set_reserved_securitization_space(60).unwrap();

		vault.reserve_securitization(&securitization(40), false, 100).unwrap();
		assert_err!(
			vault.reserve_securitization(&securitization(1), false, 100),
			VaultError::InsufficientVaultFunds
		);

		let mut vault = default_vault(100);
		vault.set_reserved_securitization_space(60).unwrap();

		vault.reserve_securitization(&securitization(40), true, 100).unwrap();
		assert_err!(
			vault.reserve_securitization(&securitization(1), true, 100),
			VaultError::InsufficientVaultFunds
		);
	}

	#[test]
	fn reservation_includes_unoccupied_and_flexible_securitization_space() {
		let mut vault = default_vault(100);
		vault.securitization_locked = 50;
		vault.flexible_securitization_locked = 50;
		vault.total_satoshis = 50;
		vault.securitized_satoshis = 50;
		assert_err!(
			vault.set_reserved_securitization_space(101),
			VaultError::InsufficientVaultFunds
		);
		vault.set_reserved_securitization_space(100).unwrap();

		let mut reclassified_flexible_space = vault.clone();
		assert_err!(
			reclassified_flexible_space.set_bitcoin_lock_flexible(50, false),
			VaultError::InsufficientVaultFunds
		);
		let mut released_flexible_space = vault.clone();
		released_flexible_space
			.release_bitcoin_lock_securitization(&securitization(50), 50, 100, true)
			.unwrap();
		assert_eq!(released_flexible_space.flexible_securitization_locked, 0);
		assert_eq!(released_flexible_space.reserved_securitization_space, 100);

		vault.set_reserved_securitization_space(50).unwrap();
		vault.set_bitcoin_lock_flexible(50, false).unwrap();
		assert_eq!(vault.flexible_securitization_locked, 0);
		assert_eq!(vault.reserved_securitization_space, 50);
		assert_eq!(vault.securitization_space(), 50);
	}

	#[test]
	fn flexible_securitization_is_included_in_securitization_space() {
		let mut vault = default_vault(100);
		vault.securitization_locked = 50;
		vault.flexible_securitization_locked = 50;
		vault.set_reserved_securitization_space(100).unwrap();

		assert_eq!(vault.securitization_space(), 100);
		assert_eq!(vault.available_securitization_space(true, None), 0);
		vault.set_reserved_securitization_space(50).unwrap();
		vault.reserve_securitization(&securitization(50), true, 100).unwrap();

		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.securitization_space(), 50);
		assert_eq!(vault.available_securitization_space(true, None), 0);
	}

	#[test]
	fn funded_flexible_lifecycle_updates_flexible_totals() {
		let mut vault = default_vault(100);
		vault.reserve_securitization(&securitization(100), false, 100).unwrap();
		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(50, 50, false))
			.unwrap();
		vault.set_bitcoin_lock_flexible(50, true).unwrap();

		assert_eq!(vault.flexible_securitization_locked, 50);

		vault
			.release_bitcoin_lock_securitization(&securitization(20), 10, 100, true)
			.unwrap();

		assert_eq!(vault.flexible_securitization_locked, 40);

		vault.extend_lock(&mut securitization(20), 200, true, true).unwrap();

		assert_eq!(vault.flexible_securitization_locked, 60);
	}

	#[test]
	fn burning_funded_flexible_bitcoin_removes_flexible_totals() {
		let mut vault = default_vault(100);
		vault.reserve_securitization(&securitization(100), false, 100).unwrap();
		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(50, 50, false))
			.unwrap();
		vault.set_bitcoin_lock_flexible(50, true).unwrap();

		vault.burn(&[(&securitization(100), 50)], 50, 100, true).unwrap();

		assert_eq!(vault.flexible_securitization_locked, 0);
	}

	#[test]
	fn terminal_burn_retires_reservations_before_settling_displaced_flexible_coverage() {
		let mut vault = default_vault(200);
		vault.reserve_securitization(&securitization(195), false, 365).unwrap();
		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(100, 100, false))
			.unwrap();
		vault.set_bitcoin_lock_flexible(100, true).unwrap();

		// Another customer's reservation displaces the flexible coverage.
		vault.reserve_securitization(&securitization(100), true, 365).unwrap();
		assert_eq!(vault.securitization_locked, 295);
		assert_eq!(vault.regular_securitization_locked(), 195);

		// The terminating Lock has an unfunded reservation of 95 and a funded Liquid of 100.
		let result = vault
			.burn(&[(&securitization(95), 0), (&securitization(100), 100)], 100, 365, true)
			.unwrap();

		assert_eq!(result.burned_amount, 100);
		assert_eq!(vault.securitization, 100);
		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.securitization_pending_activation, 100);
		assert_eq!(vault.flexible_securitization_locked, 0);
		assert_eq!(vault.total_satoshis, 0);
		assert_eq!(vault.securitized_satoshis, 0);
	}

	#[test]
	fn burning_flexible_securitization_reduces_reserved_space() {
		let mut vault = default_vault(100);
		vault.securitization_locked = 100;
		vault.flexible_securitization_locked = 50;
		vault.total_satoshis = 75;
		vault.securitized_satoshis = 75;
		vault.set_reserved_securitization_space(50).unwrap();

		vault.burn(&[(&securitization(50), 50)], 25, 100, true).unwrap();

		assert_eq!(vault.securitization, 75);
		assert_eq!(vault.securitization_locked, 50);
		assert_eq!(vault.flexible_securitization_locked, 0);
		assert_eq!(vault.reserved_securitization_space, 25);
	}

	#[test]
	fn can_burn() {
		let mut vault = default_vault(100);

		vault.reserve_securitization(&securitization(100), true, 365).unwrap();
		assert_eq!(vault.get_relock_capacity(), 0);
		assert_eq!(vault.available_securitization_space(true, None), 0);
		assert_eq!(vault.securitization_locked, 100);
		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(100, 100, false))
			.unwrap();
		vault.request_securitization_exit(80, 432).unwrap();
		vault.securitization_target = 20;

		let burn_result = vault.burn(&[(&securitization(100), 100)], 50, 365, false).unwrap();
		assert_eq!(burn_result.burned_amount, 50);
		assert_eq!(burn_result.held_for_release, 50);
		assert_eq!(burn_result.release_heights.len(), 1);
		assert_eq!(vault.securitization_locked, 0);
		assert_eq!(vault.securitization_release_schedule[&432].relockable_commitments, 50);
		assert_eq!(vault.get_relock_capacity(), 50);
		assert_eq!(vault.securitization, 50);
		assert_eq!(vault.exit_notice_amount(), 30);
		assert_eq!(vault.securitization_target, 20);
		assert_eq!(vault.available_securitization_space(true, None), 50);
	}

	#[test]
	fn burn_uses_uncommitted_securitization_above_the_lock_collateral() {
		let mut vault = default_vault(300);
		vault.committed_microgons = 300;
		for _ in 0..2 {
			vault.reserve_securitization(&securitization(100), false, 365).unwrap();
			vault
				.record_bitcoin_lock_funding(Default::default(), funding_update(100, 100, false))
				.unwrap();
		}

		let result = vault.burn(&[(&securitization(100), 100)], 150, 365, false).unwrap();
		assert_eq!(result.burned_amount, 150);
		assert_eq!(result.held_for_release, 0);
		assert_eq!(vault.securitization, 150);
		assert_eq!(vault.committed_microgons, 150);
		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.total_satoshis, 100);
	}

	#[test]
	fn burn_does_not_use_collateral_committed_to_another_lock() {
		let mut vault = default_vault(200);
		for _ in 0..2 {
			vault.reserve_securitization(&securitization(100), false, 365).unwrap();
			vault
				.record_bitcoin_lock_funding(Default::default(), funding_update(100, 100, false))
				.unwrap();
		}

		let result = vault.burn(&[(&securitization(100), 100)], 150, 365, false).unwrap();
		assert_eq!(result.burned_amount, 100);
		assert_eq!(vault.securitization, 100);
		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.total_satoshis, 100);
	}

	#[test]
	fn burn_reduces_scheduled_collateral_when_extra_collateral_is_needed() {
		let mut vault = default_vault(300);
		for _ in 0..2 {
			vault.reserve_securitization(&securitization(100), false, 365).unwrap();
			vault
				.record_bitcoin_lock_funding(Default::default(), funding_update(100, 100, false))
				.unwrap();
		}
		vault
			.release_bitcoin_lock_securitization(&securitization(100), 100, 365, false)
			.unwrap();

		let result = vault.burn(&[(&securitization(100), 100)], 250, 365, false).unwrap();

		assert_eq!(result.burned_amount, 250);
		assert_eq!(vault.securitization, 50);
		assert_eq!(vault.securitization_locked, 0);
		assert_eq!(vault.get_relock_capacity(), 50);
		assert_eq!(vault.securitization_release_schedule[&432].relockable_commitments, 50);
	}

	#[test]
	fn handles_schedule_for_release() {
		let mut vault = default_vault(500);
		vault.reserve_securitization(&securitization(100), true, 100).unwrap();
		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(100, 100, false))
			.unwrap();

		let release_heights = vault
			.release_bitcoin_lock_securitization(&securitization(100), 100, 100, false)
			.unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.securitization_release_schedule[&144].relockable_commitments, 100);
		assert_eq!(vault.get_relock_capacity(), 100);
		assert_eq!(vault.securitization_locked, 0);
		assert_eq!(vault.available_securitization_space(true, None), 500);

		let guarantee = &mut securitization(100);
		vault.extend_lock(guarantee, 100, false, true).unwrap();
		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(100, 0, false))
			.unwrap();
		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.available_securitization_space(true, None), 400);
		assert_eq!(vault.get_relock_capacity(), 0);

		vault.reserve_securitization(&securitization(100), true, 100).unwrap();
		assert_eq!(vault.securitization_locked, 200);
		assert_eq!(vault.available_securitization_space(true, None), 300);
		assert_eq!(vault.get_relock_capacity(), 0);
		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(0, 0, false))
			.unwrap();

		// schedule multiple releases

		let release_heights = vault
			.release_bitcoin_lock_securitization(&securitization(50), 50, 250, false)
			.unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.securitization_locked, 150);
		assert_eq!(vault.securitization_release_schedule[&288].relockable_commitments, 50);
		assert_eq!(vault.get_relock_capacity(), 50);

		let release_heights = vault
			.release_bitcoin_lock_securitization(&securitization(25), 25, 255, false)
			.unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.securitization_locked, 125);
		assert_eq!(vault.securitization_release_schedule[&288].relockable_commitments, 75);
		assert_eq!(vault.get_relock_capacity(), 75);

		let release_heights = vault
			.release_bitcoin_lock_securitization(&securitization(25), 25, 300, false)
			.unwrap();
		assert_eq!(release_heights.len(), 1);
		assert_eq!(vault.securitization_locked, 100);
		assert_eq!(vault.securitization_release_schedule[&288].relockable_commitments, 75);
		assert_eq!(vault.securitization_release_schedule[&432].relockable_commitments, 25);
		assert_eq!(vault.get_relock_capacity(), 100);

		// if the expiration is within the other expirations, it will prefer the securitization
		// already scheduled for release
		let guarantee = &mut securitization(25);
		vault.extend_lock(guarantee, 143, false, true).unwrap();
		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(25, 0, false))
			.unwrap();
		assert_eq!(guarantee.fund_hold_extensions.len(), 0);
		assert_eq!(vault.securitization_locked, 125);
		assert_eq!(vault.securitization_release_schedule[&288].relockable_commitments, 75);
		assert_eq!(vault.securitization_release_schedule[&432].relockable_commitments, 25);
		assert_eq!(vault.get_relock_capacity(), 100);

		// extend the lock beyond the available unlocked securitization, using scheduled-for-release
		// funds
		assert_eq!(vault.available_securitization_space(true, None), 375);
		assert_eq!(vault.securitization_locked, 125);
		let guarantee = &mut securitization(370);
		vault.extend_lock(guarantee, 143, false, true).unwrap();
		vault
			.record_bitcoin_lock_funding(Default::default(), funding_update(370, 0, false))
			.unwrap();
		assert_eq!(guarantee.fund_hold_extensions.len(), 2);
		assert_eq!(guarantee.fund_hold_extensions.get(&288).unwrap(), &75);
		assert_eq!(guarantee.fund_hold_extensions.get(&432).unwrap(), &20);
		assert_eq!(vault.get_relock_capacity(), 5);
		assert_eq!(vault.securitization_locked, 495);
		assert_eq!(vault.securitization_release_schedule[&432].relockable_commitments, 5);

		// now return the 370
		let result = vault.release_bitcoin_lock_securitization(guarantee, 370, 143, false).unwrap();
		assert_eq!(result.len(), 3);
		assert_eq!(result.iter().collect::<Vec<_>>(), vec![&144, &288, &432]);
		assert_eq!(vault.get_relock_capacity(), 375);
		assert_eq!(vault.securitization_locked, 125);
		assert_eq!(vault.securitization_release_schedule[&144].relockable_commitments, 275);
		assert_eq!(vault.securitization_release_schedule[&288].relockable_commitments, 75);
		assert_eq!(vault.securitization_release_schedule[&432].relockable_commitments, 25);
	}

	fn securitization(amount: Satoshis) -> CosignerGuarantee<Balance> {
		CosignerGuarantee {
			basis: BitcoinSecuritizationBasis {
				satoshis: amount,
				microgons_at_target_per_btc: SATOSHIS_PER_BITCOIN as Balance,
				tick: 0,
			},
			guaranteed_microgons: amount.into(),
			fund_hold_extensions: Default::default(),
		}
	}

	fn funding_update(
		securitized_satoshis: Satoshis,
		activated_securitization: Balance,
		is_flexible: bool,
	) -> BitcoinLockFunding<Balance> {
		BitcoinLockFunding {
			funded_satoshis: securitized_satoshis,
			securitized_satoshis,
			activated_securitization,
			is_flexible,
		}
	}

	fn default_vault(securitization: Balance) -> Vault<u64, Balance> {
		Vault::<u64, Balance> {
			operator_account_id: 0,
			delegate_account_id: None,
			securitization,
			securitization_target: securitization,
			securitization_locked: 0,
			flexible_securitization_locked: 0,
			reserved_securitization_space: 0,
			securitization_pending_activation: 0,
			total_satoshis: 0,
			securitized_satoshis: 0,
			securitization_release_schedule: Default::default(),
			committed_microgons: 0,
			is_closed: false,
			terms: VaultTerms { bitcoin_annual_percent_rate: 0.into(), bitcoin_base_fee: 0 },
			pending_terms: None,
			opened_tick: 0,
		}
	}
}
