use crate::{Config, Pallet};
use argon_primitives::{
	bitcoin::{BitcoinHeight, Satoshis},
	tick::Tick,
	vault::{SecuritizationScheduleEntry, VaultTerms},
	VaultId,
};
use codec::{Decode, Encode};
use frame_support::storage_alias;
use pallet_prelude::*;
use sp_runtime::BoundedBTreeMap;

/// Record completion after the runtime has converted the Vault, Lock and Fission storage.
pub fn complete_migration<T: Config>() {
	StorageVersion::new(20).put::<Pallet<T>>();
}

/// Exact runtime 160 Vault layout; runtime 159 upgrades must finish before runtime 161.
pub mod v19 {
	use super::*;

	#[derive(Encode, Decode, CloneNoBound)]
	pub struct Vault<T: Config> {
		/// The account assigned to operate this vault
		pub operator_account_id: T::AccountId,
		/// Optional delegated hot account allowed to act on behalf of this vault.
		pub delegate_account_id: Option<T::AccountId>,
		/// The securitization in the vault
		#[codec(compact)]
		pub securitization: T::Balance,
		/// The target securitization to have in the vault (in case of reducing)
		#[codec(compact)]
		pub securitization_target: T::Balance,
		/// The securitization locked for bitcoin (at the ratio given)
		#[codec(compact)]
		pub securitization_locked: T::Balance,
		/// The funded flexible portion of `securitization_locked`.
		#[codec(compact)]
		pub flexible_securitization_locked: T::Balance,
		/// Vault securitization space reserved for future Bitcoin locks.
		#[codec(compact)]
		pub reserved_securitization_space: T::Balance,
		/// Securitization pending bitcoin funding confirmation (this is "out of" the
		/// securitization_locked, not in addition to)
		#[codec(compact)]
		pub securitization_pending_activation: T::Balance,
		/// Confirmed satoshis currently backed by activated securitization.
		#[codec(compact)]
		pub securitized_satoshis: Satoshis,
		/// Total confirmed satoshis currently held across this Vault's Locks.
		#[codec(compact)]
		pub total_satoshis: Satoshis,
		/// Funded satoshis adjusted by each Lock's securitization ratio.
		#[codec(compact)]
		pub ratio_adjusted_satoshis: Satoshis,
		/// The funded flexible portion of `ratio_adjusted_satoshis`.
		#[codec(compact)]
		pub flexible_ratio_adjusted_satoshis: Satoshis,
		/// Commitments and whole withdrawals grouped by their next Bitcoin day release boundary.
		pub securitization_release_schedule:
			BoundedBTreeMap<BitcoinHeight, SecuritizationScheduleEntry<T::Balance>, ConstU32<366>>,
		/// Microgons used in reward snapshots that require one-year withdrawal notice.
		#[codec(compact)]
		pub committed_microgons: T::Balance,
		/// The securitization ratio of "total securitization" to "available for locked bitcoin"
		#[codec(compact)]
		pub securitization_ratio: FixedU128,
		/// If the vault is closed, no new bitcoin locks can be issued
		pub is_closed: bool,
		/// The terms for locked bitcoin
		pub terms: VaultTerms<T::Balance>,
		/// The terms that are pending to be applied to this vault at the given tick
		pub pending_terms: Option<(Tick, VaultTerms<T::Balance>)>,
		/// A tick at which this vault is active
		#[codec(compact)]
		pub opened_tick: Tick,
	}

	#[storage_alias]
	pub type VaultsById<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, VaultId, Vault<T>, OptionQuery>;

	impl<T: Config> Vault<T> {
		pub fn into_current(self) -> argon_primitives::vault::Vault<T::AccountId, T::Balance> {
			argon_primitives::vault::Vault {
				operator_account_id: self.operator_account_id,
				delegate_account_id: self.delegate_account_id,
				securitization: self.securitization,
				securitization_target: self.securitization_target,
				securitization_locked: self.securitization_locked,
				flexible_securitization_locked: self.flexible_securitization_locked,
				reserved_securitization_space: self.reserved_securitization_space,
				securitization_pending_activation: self.securitization_pending_activation,
				securitized_satoshis: self.securitized_satoshis,
				total_satoshis: self.total_satoshis,
				securitization_release_schedule: self.securitization_release_schedule,
				committed_microgons: self.committed_microgons,
				is_closed: self.is_closed,
				terms: self.terms,
				pending_terms: self.pending_terms,
				opened_tick: self.opened_tick,
			}
		}
	}
}
