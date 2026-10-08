#![cfg_attr(not(feature = "std"), no_std)]
//! Maintained account and network quantities used by Treasury frame calculations and bond backing.
//! Source pallets own individual locks, lots, Fissions and custody. This pallet maintains their
//! aggregates; Treasury owns frame snapshots and payouts.

use argon_primitives::{
	treasury::{
		BitcoinLockPosition, BitcoinLockPositionProvider, PositionQuantities, PositionQuantity,
		TreasuryPositionProvider, UpstreamPosition,
	},
	OperationalAccountProvider, TreasuryPoolProvider, VaultId,
};
pub use pallet::*;
use pallet_prelude::*;
use polkadot_sdk::sp_runtime::ArithmeticError;
pub use weights::{ProviderWeightAdapter, WeightInfo};
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod tests;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

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
		/// Costs of reading and maintaining account aggregates.
		type WeightInfo: WeightInfo;
	}

	/// Account-wide principal and quantities, with an optional subset in the upstream vault.
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
	pub struct Position<Balance: codec::Codec + MaxEncodedLen> {
		/// Live principal for certification/backing, including displaced flexible bonds.
		#[codec(compact)]
		pub bond_principal: Balance,
		/// Live admitted bond/stake quantities and active Fission promises.
		pub quantities: PositionQuantities<Balance>,
		/// Participation in the single vault that onboarded this account. Absent when no upstream
		/// vault is resolvable, including accounts whose upstream has yet to create a vault.
		pub upstream: Option<UpstreamPosition<Balance>>,
	}
	/// Maintained source quantities keyed by the actual position owner. Upstream quantities belong
	/// to the linked vault account; primary and mining account balances are separate owners.
	#[pallet::storage]
	pub type PositionsByAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Position<T::Balance>, OptionQuery>;

	/// Maintained network quantities, updated together with their account contributions.
	/// Treasury reads these when freezing frame inputs. This replaces its live Argonot stake total.
	#[pallet::storage]
	pub type NetworkTotals<T: Config> = StorageValue<_, PositionQuantities<T::Balance>, ValueQuery>;

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

		fn network_totals() -> PositionQuantities<T::Balance> {
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
				NetworkTotals::<T>::try_mutate(replace)
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

		/// Replace one canonical lock's contribution when it belongs to this owner's upstream.
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
	}

	impl<T: Config> Pallet<T> {
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
