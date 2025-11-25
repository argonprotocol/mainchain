use crate::{Config, MintedBitcoinMicrogons, MintedMiningMicrogons, Pallet};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

mod old_storage {
	use super::*;

	#[storage_alias]
	pub type MintedMiningArgons<T: Config> = StorageValue<Pallet<T>, U256, ValueQuery>;

	#[storage_alias]
	pub type MintedBitcoinArgons<T: Config> = StorageValue<Pallet<T>, U256, ValueQuery>;

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	pub struct Model {
		pub minted_mining_argons: U256,
		pub minted_bitcoin_argons: U256,
	}
}

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let minted_mining_argons = old_storage::MintedMiningArgons::<T>::get();
		let minted_bitcoin_argons = old_storage::MintedBitcoinArgons::<T>::get();
		let state = old_storage::Model { minted_mining_argons, minted_bitcoin_argons };
		let encoded = state.encode();
		Ok(encoded)
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		log::info!("Migrating mint");
		let modify_count = 2;
		let old_mining: T::Balance = old_storage::MintedMiningArgons::<T>::take().as_u128().into();
		MintedMiningMicrogons::<T>::put(old_mining);
		let old_bitcoin: T::Balance =
			old_storage::MintedBitcoinArgons::<T>::take().as_u128().into();
		MintedBitcoinMicrogons::<T>::put(old_bitcoin);

		T::DbWeight::get().reads_writes(modify_count as u64, modify_count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let model = old_storage::Model::decode(&mut &state[..])
			.expect("Failed to decode pre-upgrade state");
		assert_eq!(
			model.minted_bitcoin_argons.as_u128(),
			MintedBitcoinMicrogons::<T>::get().into()
		);
		assert_eq!(model.minted_mining_argons.as_u128(), MintedMiningMicrogons::<T>::get().into());

		Ok(())
	}
}

pub type MintToBalance<T> = frame_support::migrations::VersionedMigration<
	0,
	1,
	InnerMigrate<T>,
	crate::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
