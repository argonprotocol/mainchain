#![cfg_attr(not(feature = "std"), no_std)]
//! Maintained account and network quantities used by Treasury frame calculations and bond backing.
//! Source pallets own individual vaults, locks, lots, Fissions and custody. This pallet maintains
//! their aggregates and account earnings. Treasury owns frame terms and reward funding.

extern crate alloc;

use alloc::vec::Vec;
use argon_primitives::{
	providers::MiningFrameTransitionProviderWeightInfo,
	treasury::{
		BitcoinLockPosition, BitcoinLockPositionProvider, NetworkPositionQuantities,
		PositionQuantities, PositionQuantity, StabilizerPoolRewardPayer,
		StabilizerPoolRewardPayerWeightInfo, TreasuryPositionProvider, UpstreamPosition,
	},
	MiningFrameTransitionProvider, OperationalAccountProvider, TreasuryPoolProvider, VaultId,
};
pub use pallet::*;
use pallet_prelude::*;
use polkadot_sdk::sp_runtime::ArithmeticError;
use weights::{MiningFrameWeights, StabilizerPoolPayoutWeights};
pub use weights::{ProviderWeightAdapter, WeightInfo};
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
#[cfg(feature = "try-runtime")]
mod try_state;
pub mod weights;

#[cfg(feature = "try-runtime")]
const EXPECTED_AUTO_COLLECT_POSITIONS_PER_FRAME: u32 = 500;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	pub(super) const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// Argon balance units for bond principal and Bitcoin collateral (microgons in the
		/// runtime).
		type Balance: AtLeast32BitUnsigned
			+ Member
			+ codec::FullCodec
			+ Copy
			+ Default
			+ MaybeSerializeDeserialize
			+ core::fmt::Debug
			+ From<u128>
			+ Into<u128>
			+ codec::HasCompact
			+ DecodeWithMemTracking
			+ TypeInfo
			+ MaxEncodedLen;
		/// Resolves the actual vault account and its single upstream vault from Operational
		/// Accounts.
		type OperationalAccountProvider: OperationalAccountProvider<Self::AccountId>;
		/// Reads canonical owner-indexed Bitcoin locks when initializing a registration snapshot.
		type BitcoinPositionProvider: BitcoinLockPositionProvider<Self::AccountId, Self::Balance>;
		/// Reads canonical upstream bond principal when initializing a registration snapshot.
		type TreasuryPoolProvider: TreasuryPoolProvider<Self::AccountId, Balance = Self::Balance>;
		/// Reports the frame boundary after MiningSlot initializes the block.
		type MiningFrameTransitionProvider: MiningFrameTransitionProvider;
		/// Issues ARGON backed by rewards that Treasury has already committed.
		type StabilizerPoolRewardPayer: StabilizerPoolRewardPayer<Self::AccountId, Self::Balance>;
		/// Costs of reading and maintaining account aggregates.
		type WeightInfo: WeightInfo;
	}

	/// Account-wide positions and earnings, with an optional subset in the upstream vault.
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
	pub struct Position<Balance>
	where
		Balance: Codec + MaxEncodedLen,
	{
		/// Live principal for certification/backing, including displaced flexible bonds.
		#[codec(compact)]
		pub bond_principal: Balance,
		/// Live admitted bond/stake quantities and active Fission promises.
		pub quantities: PositionQuantities<Balance>,
		/// Participation in the single vault that onboarded this account. Absent when no upstream
		/// vault is resolvable, including accounts whose upstream has yet to create a vault.
		pub upstream: Option<UpstreamPosition<Balance>>,
		/// Committed amounts available to collect, by source.
		pub collect_accrued: CollectAmounts<Balance>,
		/// Lifetime amounts delivered through collection, by source.
		pub collected: CollectAmounts<Balance>,
	}

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
		/// ARGON from Bitcoin mint grants.
		#[codec(compact)]
		pub bitcoin_mints: Balance,
		/// Rewards earned for supplying eligible stabilizer pool liquidity.
		#[codec(compact)]
		pub stabilizer_pool_earnings: Balance,
	}

	/// Source quantities and earnings keyed by the position owner. Upstream quantities belong to
	/// the linked vault account; primary and mining accounts have separate positions.
	#[pallet::storage]
	pub type PositionsByAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Position<T::Balance>, OptionQuery>;

	/// Account contribution totals and Vault Bitcoin coverage, maintained as sources change.
	/// Treasury freezes these network quantities at each frame boundary.
	#[pallet::storage]
	pub type NetworkTotals<T: Config> =
		StorageValue<_, NetworkPositionQuantities<T::Balance>, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let frame_check = MiningFrameWeights::<T>::is_new_frame_started();
			if T::MiningFrameTransitionProvider::is_new_frame_started().is_none() {
				return frame_check;
			}

			let weight = frame_check.saturating_add(
				StabilizerPoolPayoutWeights::<T>::has_outstanding_stabilizer_pool_rewards(),
			);
			if !T::StabilizerPoolRewardPayer::has_outstanding_stabilizer_pool_rewards() {
				return weight;
			}

			weight.saturating_add(Self::auto_collect_stabilizer_pool_rewards())
		}

		#[cfg(feature = "try-runtime")]
		fn try_state(_: BlockNumberFor<T>) -> Result<(), sp_runtime::TryRuntimeError> {
			crate::try_state::check::<T>()
		}
	}

	impl<T: Config> TreasuryPositionProvider<T::AccountId, T::Balance> for Pallet<T> {
		type Weights = ProviderWeightAdapter<T>;

		/// Seed the upstream subset after registration records the relationship. Locks and bonds
		/// can predate registration, so query their canonical owner indexes once. Preserve the
		/// account-wide bond principal already maintained by live source updates.
		fn operational_account_registered(account: &T::AccountId) -> DispatchResult {
			let upstream = if let Some((_, vault_id)) =
				T::OperationalAccountProvider::upstream_vault(account)
			{
				let bitcoin = T::BitcoinPositionProvider::account_position(account, vault_id)?;
				Some(UpstreamPosition {
					vault_id,
					bitcoin_securitization: bitcoin.activated_securitization,
					bitcoin_allocated_securitization: bitcoin.allocated_securitization,
					bond_principal: T::TreasuryPoolProvider::active_vault_bond_amount(
						vault_id, account,
					),
				})
			} else {
				None
			};
			Self::set_upstream_position(account, upstream);
			Ok(())
		}

		/// Live principal across all vaults, including displaced flexible bonds and excluding
		/// releases.
		fn bond_principal(account: &T::AccountId) -> T::Balance {
			PositionsByAccount::<T>::get(account)
				.map(|p| p.bond_principal)
				.unwrap_or_default()
		}

		fn account_quantities(account: &T::AccountId) -> PositionQuantities<T::Balance> {
			PositionsByAccount::<T>::get(account).map(|p| p.quantities).unwrap_or_default()
		}

		fn network_totals() -> NetworkPositionQuantities<T::Balance> {
			NetworkTotals::<T>::get()
		}

		/// Replace a source's contribution before or after account registration. Account and
		/// network quantities commit together; Treasury retains the current frame's frozen terms.
		fn account_quantity_updated<Amount: Into<u128>>(
			account: &T::AccountId,
			quantity: PositionQuantity,
			previous: Amount,
			current: Amount,
		) -> DispatchResult {
			let previous = previous.into();
			let current = current.into();
			if previous == current {
				return Ok(());
			}
			PositionsByAccount::<T>::try_mutate(account, |record| -> DispatchResult {
				let account_position = record.get_or_insert_with(Position::default);
				let replace = |quantities: &mut PositionQuantities<T::Balance>| -> DispatchResult {
					let total = match quantity {
						PositionQuantity::Bonds => quantities.bonds,
						PositionQuantity::Stakes => quantities.stakes,
						PositionQuantity::FissionLiquidity => quantities.fission_liquidity.into(),
					};
					let updated = total
						.checked_sub(previous)
						.ok_or(ArithmeticError::Underflow)?
						.checked_add(current)
						.ok_or(ArithmeticError::Overflow)?;
					match quantity {
						PositionQuantity::Bonds => quantities.bonds = updated,
						PositionQuantity::Stakes => quantities.stakes = updated,
						PositionQuantity::FissionLiquidity =>
							quantities.fission_liquidity = updated.into(),
					}
					Ok(())
				};
				replace(&mut account_position.quantities)?;
				NetworkTotals::<T>::try_mutate(|totals| replace(&mut totals.account_quantities))
			})
		}

		fn network_securitized_satoshis_updated(previous: u128, current: u128) -> DispatchResult {
			if previous == current {
				return Ok(());
			}
			NetworkTotals::<T>::try_mutate(|totals| -> DispatchResult {
				totals.vault_securitized_satoshis = totals
					.vault_securitized_satoshis
					.checked_sub(previous)
					.ok_or(ArithmeticError::Underflow)?
					.checked_add(current)
					.ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})
		}

		/// Read the maintained upstream subset without scanning canonical locks or bond lots.
		fn upstream_position(account: &T::AccountId) -> Option<UpstreamPosition<T::Balance>> {
			PositionsByAccount::<T>::get(account).and_then(|p| p.upstream)
		}

		/// Install a complete upstream snapshot at registration or upgrade, preserving total
		/// principal.
		fn set_upstream_position(
			account: &T::AccountId,
			upstream: Option<UpstreamPosition<T::Balance>>,
		) {
			PositionsByAccount::<T>::mutate(account, |p| {
				p.get_or_insert_with(Position::default).upstream = upstream
			});
		}

		/// Replace an affected Bitcoin securitization contribution in this owner's upstream.
		/// Checked arithmetic prevents drift; a failed replacement leaves the aggregate unchanged.
		fn bitcoin_position_updated(
			account: &T::AccountId,
			vault: VaultId,
			previous: BitcoinLockPosition<T::Balance>,
			position: BitcoinLockPosition<T::Balance>,
		) -> DispatchResult {
			PositionsByAccount::<T>::try_mutate_exists(account, |record| -> DispatchResult {
				Self::bind_upstream_if_created(account, vault, record);
				let Some(upstream) = record
					.as_mut()
					.and_then(|p| p.upstream.as_mut())
					.filter(|p| p.vault_id == vault)
				else {
					return Ok(());
				};
				upstream.bitcoin_securitization = upstream
					.bitcoin_securitization
					.checked_sub(&previous.activated_securitization)
					.ok_or(ArithmeticError::Underflow)?
					.checked_add(&position.activated_securitization)
					.ok_or(ArithmeticError::Overflow)?;
				upstream.bitcoin_allocated_securitization = upstream
					.bitcoin_allocated_securitization
					.checked_sub(&previous.allocated_securitization)
					.ok_or(ArithmeticError::Underflow)?
					.checked_add(&position.allocated_securitization)
					.ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})
		}

		/// Replace one active vault lot's principal in the account total and, when applicable, its
		/// upstream subset. Releasing lots contribute zero; flexible displacement retains
		/// principal.
		fn bond_position_updated(
			account: &T::AccountId,
			vault: VaultId,
			previous: T::Balance,
			principal: T::Balance,
		) -> DispatchResult {
			PositionsByAccount::<T>::try_mutate(account, |record| -> DispatchResult {
				Self::bind_upstream_if_created(account, vault, record);
				let p = record.get_or_insert_with(Position::default);
				p.bond_principal = p
					.bond_principal
					.checked_sub(&previous)
					.ok_or(ArithmeticError::Underflow)?
					.checked_add(&principal)
					.ok_or(ArithmeticError::Overflow)?;
				if let Some(upstream) = p.upstream.as_mut().filter(|p| p.vault_id == vault) {
					upstream.bond_principal = upstream
						.bond_principal
						.checked_sub(&previous)
						.ok_or(ArithmeticError::Underflow)?
						.checked_add(&principal)
						.ok_or(ArithmeticError::Overflow)?;
				}
				Ok(())
			})
		}

		fn credit_stabilizer_pool_rewards(
			account: &T::AccountId,
			amount: T::Balance,
		) -> DispatchResult {
			if amount.is_zero() {
				return Ok(());
			}
			PositionsByAccount::<T>::try_mutate(account, |record| {
				let position = record.get_or_insert_with(Position::default);
				let accrued = &mut position.collect_accrued.stabilizer_pool_earnings;
				*accrued = accrued.checked_add(&amount).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})
		}
	}

	impl<T: Config> Pallet<T> {
		/// Temporary frame payout until accounts can use manual Collect.
		fn auto_collect_stabilizer_pool_rewards() -> Weight {
			let db = T::DbWeight::get();
			let read_position = T::WeightInfo::account_quantities().saturating_add(db.reads(1));
			let pay_reward = StabilizerPoolPayoutWeights::<T>::pay_stabilizer_pool_rewards();
			let mut weight = db.reads(1); // The final storage key lookup ends the walk.

			// Finish reading the map before updating any of its records.
			let positions_with_earnings: Vec<_> = PositionsByAccount::<T>::iter()
				.filter(|(_, position)| {
					weight.saturating_accrue(read_position);
					!position.collect_accrued.stabilizer_pool_earnings.is_zero()
				})
				.collect();

			for (account, position) in positions_with_earnings {
				weight.saturating_accrue(pay_reward);
				match Self::pay_stabilizer_pool_position(&account, position) {
					Ok(_) => weight.saturating_accrue(db.writes(1)),
					Err(error) => log::warn!(
						"Stabilizer rewards remain uncollected for {account:?}: {error:?}"
					),
				}
			}
			weight
		}

		/// Collect the account's committed rewards. A failed payment leaves them available.
		pub fn collect_stabilizer_pool_rewards(
			account: &T::AccountId,
		) -> Result<T::Balance, DispatchError> {
			let Some(position) = PositionsByAccount::<T>::get(account) else {
				return Ok(T::Balance::zero());
			};
			Self::pay_stabilizer_pool_position(account, position)
		}

		fn pay_stabilizer_pool_position(
			account: &T::AccountId,
			mut position: Position<T::Balance>,
		) -> Result<T::Balance, DispatchError> {
			let amount = position.collect_accrued.stabilizer_pool_earnings;
			if amount.is_zero() {
				return Ok(amount);
			}

			let collected = position
				.collected
				.stabilizer_pool_earnings
				.checked_add(&amount)
				.ok_or(ArithmeticError::Overflow)?;
			T::StabilizerPoolRewardPayer::pay_stabilizer_pool_rewards(account, amount)?;

			position.collect_accrued.stabilizer_pool_earnings = T::Balance::zero();
			position.collected.stabilizer_pool_earnings = collected;
			PositionsByAccount::<T>::insert(account, position);
			Ok(amount)
		}

		/// An invite can predate its upstream's vault. Bind when that vault first gets a position.
		/// Vault IDs are never reused, so there are no earlier contributions in this newly created
		/// vault to scan. Only the actual linked vault account may own the upstream subset.
		fn bind_upstream_if_created(
			account: &T::AccountId,
			vault: VaultId,
			record: &mut Option<Position<T::Balance>>,
		) {
			if record.as_ref().is_some_and(|p| p.upstream.is_some()) {
				return;
			}
			if let Some((owner, upstream_vault)) =
				T::OperationalAccountProvider::upstream_vault(account) &&
				owner == *account &&
				upstream_vault == vault
			{
				record.get_or_insert_with(Position::default).upstream =
					Some(UpstreamPosition { vault_id: vault, ..Default::default() });
			}
		}
	}
}
