use crate::{Config, Fission as FissionWithGuarantee, Pallet};
use argon_primitives::vault::CosignerGuarantee;
use frame_support::storage_alias;
use frame_system::Config as SystemConfig;
use pallet_prelude::*;

pub fn is_migration_pending<T: Config>() -> bool {
	StorageVersion::get::<Pallet<T>>() == 1
}

pub fn is_migration_complete<T: Config>() -> bool {
	StorageVersion::get::<Pallet<T>>() == 2
}

/// Record completion after the runtime has converted the Vault, Lock and Fission storage.
pub fn complete_migration<T: Config>() {
	StorageVersion::new(2).put::<Pallet<T>>();
}

/// Fissions storage version 1, used by runtime 160 before guarantees were stored on Fissions.
pub mod v1 {
	use super::*;
	use argon_primitives::bitcoin::{BitcoinLockId, FissionId, LiquidId, Satoshis};

	#[derive(Encode, Decode, CloneNoBound)]
	pub struct Fission<T: Config> {
		#[codec(compact)]
		pub liquid_id: LiquidId,
		#[codec(compact)]
		pub lock_id: BitcoinLockId,
		#[codec(compact)]
		pub satoshis: Satoshis,
		#[codec(compact)]
		pub microgons_at_target_per_btc: T::Balance,
		#[codec(compact)]
		pub liquidity_promised: T::Balance,
		#[codec(compact)]
		pub created_at_argon_block: BlockNumberFor<T>,
		#[codec(compact)]
		pub ratchet_number: u32,
		#[codec(compact)]
		pub last_ratchet_tick: Tick,
		#[codec(compact)]
		pub last_updated_argon_block: BlockNumberFor<T>,
	}

	impl<T: Config> Fission<T> {
		pub fn into_current(
			self,
			cosigner_guarantee: CosignerGuarantee<T::Balance>,
		) -> FissionWithGuarantee<T> {
			FissionWithGuarantee {
				liquid_id: self.liquid_id,
				lock_id: self.lock_id,
				satoshis: self.satoshis,
				microgons_at_target_per_btc: self.microgons_at_target_per_btc,
				liquidity_promised: self.liquidity_promised,
				cosigner_guarantee,
				created_at_argon_block: self.created_at_argon_block,
				ratchet_number: self.ratchet_number,
				last_ratchet_tick: self.last_ratchet_tick,
				last_updated_argon_block: self.last_updated_argon_block,
			}
		}
	}

	#[storage_alias]
	pub type FissionByOwnerAndId<T: Config> = StorageDoubleMap<
		Pallet<T>,
		Blake2_128Concat,
		<T as SystemConfig>::AccountId,
		Twox64Concat,
		FissionId,
		Fission<T>,
		OptionQuery,
	>;
}
