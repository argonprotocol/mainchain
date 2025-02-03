use crate::{
	pallet::{BondsById, MiningBondCompletions},
	Config,
};
use alloc::vec::Vec;
use argon_primitives::{
	bond::{Bond, BondExpiration},
	TickProvider,
};
use frame_support::{pallet_prelude::*, traits::UncheckedOnRuntimeUpgrade};
use log::info;
use sp_arithmetic::traits::{Saturating, UniqueSaturatedInto};

mod v0 {
	use crate::Config;
	#[cfg(feature = "try-runtime")]
	use alloc::vec::Vec;
	use argon_primitives::{
		bitcoin::{BitcoinHeight, UtxoId},
		bond::BondType,
		BondId, VaultId,
	};
	use codec::Codec;
	use frame_support::{pallet_prelude::*, storage_alias, BoundedVec};
	use frame_system::pallet_prelude::BlockNumberFor;
	use scale_info::TypeInfo;

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum BondExpiration<BlockNumber: Codec> {
		ArgonBlock(#[codec(compact)] BlockNumber),
		BitcoinBlock(#[codec(compact)] BitcoinHeight),
	}

	#[derive(Clone, Encode, Decode, TypeInfo)]
	pub struct Bond<AccountId: Codec, Balance: Codec, BlockNumber: Codec> {
		pub bond_type: BondType,
		#[codec(compact)]
		pub vault_id: VaultId,
		pub utxo_id: Option<UtxoId>,
		pub bonded_account_id: AccountId,
		#[codec(compact)]
		pub total_fee: Balance,
		#[codec(compact)]
		pub prepaid_fee: Balance,
		#[codec(compact)]
		pub amount: Balance,
		#[codec(compact)]
		pub start_block: BlockNumber,
		pub expiration: BondExpiration<BlockNumber>,
	}

	pub type BondOf<T> =
		Bond<<T as frame_system::Config>::AccountId, <T as Config>::Balance, BlockNumberFor<T>>;

	#[storage_alias]
	pub type BondsById<T: Config> =
		StorageMap<crate::Pallet<T>, Twox64Concat, BondId, BondOf<T>, OptionQuery>;

	#[storage_alias]
	pub type MiningBondCompletions<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<BondId, <T as Config>::MaxConcurrentlyExpiringBonds>,
		ValueQuery,
	>;

	#[cfg(feature = "try-runtime")]
	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct ConversionStructure<T: Config + frame_system::Config> {
		pub bonds_by_id: Vec<(BondId, BondOf<T>)>,
		pub completions: Vec<(
			BlockNumberFor<T>,
			BoundedVec<BondId, <T as Config>::MaxConcurrentlyExpiringBonds>,
		)>,
	}
}

pub struct InnerMigrateV0ToV1<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrateV0ToV1<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let bonds_by_id = v0::BondsById::<T>::iter().collect::<Vec<_>>();
		let completions = v0::MiningBondCompletions::<T>::iter().collect::<Vec<_>>();
		// Return it as an encoded `Vec<u8>`
		Ok(v0::ConversionStructure::<T> { bonds_by_id, completions }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		info!("Migrating Bonds from v0 to v1");
		let current_tick = T::TickProvider::current_tick();
		let current_block = <frame_system::Pallet<T>>::block_number();
		BondsById::<T>::translate::<v0::BondOf<T>, _>(|id, b| {
			info!("Migration: Translating bond with id {:?}", id);
			let block_offset = UniqueSaturatedInto::<u64>::unique_saturated_into(
				current_block.saturating_sub(b.start_block),
			);
			count += 1;
			Some(Bond {
				bond_type: b.bond_type,
				vault_id: b.vault_id,
				utxo_id: b.utxo_id,
				bonded_account_id: b.bonded_account_id,
				total_fee: b.total_fee,
				prepaid_fee: b.prepaid_fee,
				amount: b.amount,
				start_tick: current_tick.saturating_sub(block_offset),
				expiration: match b.expiration {
					v0::BondExpiration::BitcoinBlock(x) => BondExpiration::BitcoinBlock(x),
					v0::BondExpiration::ArgonBlock(exp) => {
						let offset = UniqueSaturatedInto::<u64>::unique_saturated_into(
							exp.saturating_sub(current_block),
						);
						BondExpiration::AtTick(current_tick + offset)
					},
				},
			})
		});
		let bond_completions = v0::MiningBondCompletions::<T>::drain().collect::<Vec<_>>();
		for (block_exp, list) in bond_completions {
			let block_offset = UniqueSaturatedInto::<u64>::unique_saturated_into(
				block_exp.saturating_sub(current_block),
			);
			count += 1;
			MiningBondCompletions::<T>::insert(current_tick + block_offset, list);
		}

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;
		use sp_core::Decode;

		let v0::ConversionStructure { completions: _old_completions, bonds_by_id: old_bonds_by_id } =
			<v0::ConversionStructure<T>>::decode(&mut &state[..]).map_err(|_| {
				sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
			})?;

		let new_bonds = BondsById::<T>::iter().collect::<Vec<_>>();

		ensure!(new_bonds.len() == old_bonds_by_id.len(), "New value not set correctly");
		for x in new_bonds {
			if let BondExpiration::AtTick(exp) = x.1.expiration {
				ensure!(
					MiningBondCompletions::<T>::get(exp).contains(&x.0),
					"Expiration bond not moved"
				);
			}
			let old = old_bonds_by_id.iter().find(|(id, _)| id == &x.0);
			ensure!(old.is_some(), "Bond missing in translation");
			if let v0::BondExpiration::ArgonBlock(exp) = old.unwrap().1.expiration {
				ensure!(
					!v0::MiningBondCompletions::<T>::contains_key(exp),
					"Old value is in the list"
				);
			}
		}

		Ok(())
	}
}

pub type MigrateV0ToV1<T> = frame_support::migrations::VersionedMigration<
	0,
	1,
	InnerMigrateV0ToV1<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrateV0ToV1;
	use super::*;
	use crate::mock::{new_test_ext, CurrentTick, System, Test};
	use argon_primitives::bond::BondType;
	use frame_support::assert_ok;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			CurrentTick::set(5000);
			System::set_block_number(99);
			v0::BondsById::<Test>::insert(
				1,
				v0::Bond {
					start_block: 10,
					expiration: v0::BondExpiration::ArgonBlock(100),
					bond_type: BondType::Mining,
					amount: 10,
					prepaid_fee: 0,
					vault_id: 1,
					utxo_id: None,
					total_fee: 10,
					bonded_account_id: 1,
				},
			);
			v0::MiningBondCompletions::<Test>::insert(100, BoundedVec::truncate_from(vec![1]));

			// Get the pre_upgrade bytes
			let bytes = match InnerMigrateV0ToV1::<Test>::pre_upgrade() {
				Ok(bytes) => bytes,
				Err(e) => panic!("pre_upgrade failed: {:?}", e),
			};

			// Execute the migration
			let weight = InnerMigrateV0ToV1::<Test>::on_runtime_upgrade();

			// Verify post_upgrade succeeds
			assert_ok!(InnerMigrateV0ToV1::<Test>::post_upgrade(bytes));

			// The weight used should be 1 read for the old value, and 1 write for the new
			// value.
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(2, 2));

			// After the migration, the new value should be set as the `current` value.
			assert_eq!(
				crate::MiningBondCompletions::<Test>::iter_keys().map(|a| a).collect::<Vec<_>>(),
				vec![5001]
			);
			let new_value = crate::BondsById::<Test>::get(1).unwrap();
			assert_eq!(new_value.start_tick, 5000 - 89);
			assert_eq!(new_value.expiration, BondExpiration::AtTick(5001));
		})
	}
}
