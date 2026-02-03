use crate::{Config, FunderState, FunderStateByVaultAndAccount, HoldReason, VaultPoolsByFrame};
use alloc::collections::BTreeMap;
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::{
	argon_primitives::{MiningFrameTransitionProvider, vault::TreasuryVaultProvider},
	log::info,
	*,
};
use tracing::warn;

mod old_storage {
	use crate::{Config, TreasuryCapital};
	use alloc::collections::BTreeMap;
	use frame_support::storage_alias;
	use pallet_prelude::*;
	use polkadot_sdk::frame_support;

	#[derive(
		Encode,
		Decode,
		Clone,
		PartialEqNoBound,
		Eq,
		RuntimeDebugNoBound,
		TypeInfo,
		DefaultNoBound,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct BondHolder<T: Config> {
		/// The starting balance of the bond holder
		#[codec(compact)]
		pub starting_balance: T::Balance,
		/// The profits earned by the bond holder
		#[codec(compact)]
		pub earnings: T::Balance,
		/// Are the earnings maintained in this pool? For some holders (like vault operators),
		/// earnings are exported to other places (eg, the Vault Pallet). Vaults must collect their
		/// earnings there.
		pub keep_earnings_in_pool: bool,
	}

	#[derive(
		Encode,
		Decode,
		Clone,
		PartialEqNoBound,
		Eq,
		RuntimeDebugNoBound,
		TypeInfo,
		DefaultNoBound,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct TreasuryPool<T: Config> {
		/// The amount of argons per bond holder. Sorted with largest first. After bid pool is
		/// distributed, profits are added to this balance
		pub bond_holders: BoundedVec<(T::AccountId, BondHolder<T>), T::MaxTreasuryContributors>,

		/// Legacy list of accounts that opted out of renewing.
		pub do_not_renew: BoundedVec<T::AccountId, T::MaxTreasuryContributors>,

		/// Did this fund already roll over?
		pub is_rolled_over: bool,

		/// The amount of argons that have been distributed to the fund (vault + contributors)
		pub distributed_earnings: Option<T::Balance>,

		/// The vault percent of profits shared
		#[codec(compact)]
		pub vault_sharing_percent: Permill,
	}

	#[derive(PartialEq, Eq, Clone, Debug, TypeInfo, MaxEncodedLen, Encode, Decode)]
	#[scale_info(skip_type_params(T))]
	pub struct PrebondedArgons<T: Config> {
		/// The vault id that the argons are pre-bonded for
		#[codec(compact)]
		pub vault_id: VaultId,
		/// The account that is pre-bonding the argons
		pub account_id: T::AccountId,
		/// The operator principal currently held for auto-bonding (legacy field name).
		#[codec(compact)]
		pub amount_unbonded: T::Balance,
		/// The frame id that the pre-bonding started
		#[codec(compact)]
		pub starting_frame_id: FrameId,
		/// The amount bonded by offset since the starting frame (eg, frame - starting_frame % 10)
		#[deprecated(since = "1.3.6", note = "Use amounts allocated to treasury pools instead")]
		pub bonded_by_start_offset: BoundedVec<T::Balance, ConstU32<10>>,
		/// The max amount of argons that can be bonded per frame offset
		#[codec(compact)]
		pub max_amount_per_frame: T::Balance,
	}

	#[storage_alias]
	pub type PrebondedByVaultId<T: Config> =
		StorageMap<crate::Pallet<T>, Twox64Concat, VaultId, PrebondedArgons<T>, OptionQuery>;

	#[storage_alias]
	pub type CapitalRaising<T: Config> = StorageValue<
		crate::Pallet<T>,
		BoundedVec<TreasuryCapital<T>, <T as Config>::MaxVaultsPerPool>,
		ValueQuery,
	>;

	#[storage_alias]
	pub type VaultPoolsByFrame<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		FrameId,
		BoundedBTreeMap<VaultId, TreasuryPool<T>, <T as Config>::MaxVaultsPerPool>,
		ValueQuery,
	>;

	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub vault_pools_by_frame:
			BTreeMap<FrameId, BoundedBTreeMap<VaultId, TreasuryPool<T>, T::MaxVaultsPerPool>>,
		pub prebonded: BTreeMap<VaultId, PrebondedArgons<T>>,
	}
}
pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use pallet_prelude::Encode;
		let vault_pools_by_frame =
			old_storage::VaultPoolsByFrame::<T>::iter().collect::<BTreeMap<_, _>>();
		let prebonded = old_storage::PrebondedByVaultId::<T>::iter().collect::<BTreeMap<_, _>>();

		Ok(old_storage::Model { vault_pools_by_frame, prebonded }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 1;

		let mut bonded_by_vault = BTreeMap::<VaultId, (Balance, FrameId)>::new();

		let current_frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
		VaultPoolsByFrame::<T>::translate::<
			BoundedBTreeMap<VaultId, old_storage::TreasuryPool<T>, T::MaxVaultsPerPool>,
			_,
		>(|frame_id, pool_by_frame| {
			// Drop legacy prebonded (future) pools so they don't inflate bonded_principal.
			if frame_id > current_frame_id {
				info!("Dropping treasury pool for future frame {frame_id:?}");
				return None;
			}

			info!("Migrating treasury pool for frame {frame_id:?}");
			count += 1;
			let new_pool_by_frame = pool_by_frame
				.into_iter()
				.map(|(vault_id, old_pool)| {
					if !old_pool.is_rolled_over {
						for (_account, bond_holder) in &old_pool.bond_holders {
							let balance = bond_holder.starting_balance;
							bonded_by_vault
								.entry(vault_id)
								.and_modify(|(b, f)| {
									b.saturating_accrue(balance.into());
									if frame_id < *f {
										*f = frame_id;
									}
								})
								.or_insert((balance.into(), frame_id));
						}
					}
					let bond_holders = old_pool
						.bond_holders
						.into_iter()
						.map(|(account, bond_holder)| (account, bond_holder.starting_balance))
						.collect::<Vec<_>>();
					let new_pool = crate::TreasuryPool {
						bond_holders: BoundedVec::truncate_from(bond_holders),
						distributed_earnings: old_pool.distributed_earnings,
						vault_sharing_percent: old_pool.vault_sharing_percent,
					};
					(vault_id, new_pool)
				})
				.collect::<BTreeMap<_, _>>();

			Some(
				BoundedBTreeMap::try_from(new_pool_by_frame)
					.expect("Treasury pools should be valid"),
			)
		});

		let current_frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
		let prebonded = old_storage::PrebondedByVaultId::<T>::drain().collect::<BTreeMap<_, _>>();
		for (vault_id, bond) in &prebonded {
			if !bonded_by_vault.contains_key(vault_id) {
				bonded_by_vault.insert(*vault_id, (0u32.into(), bond.starting_frame_id));
			}
		}
		for (vault_id, (bonded, oldest_frame)) in &bonded_by_vault {
			info!("Migrating prebonded argons for vault ID {:?}", vault_id);
			let vault_operator = T::TreasuryVaultProvider::get_vault_operator(*vault_id)
				.expect("Vault must exist for prebonded argons");
			let target_principal = T::Currency::balance_on_hold(
				&HoldReason::ContributedToTreasury.into(),
				&vault_operator,
			);
			let bonded_principal = *bonded;
			let held_principal = target_principal;

			info!(
				"Setting funder state for vault ID {:?}, operator {:?}, bonded principal {:?}, target principal {:?}, held principal {:?}",
				vault_id, vault_operator, bonded_principal, target_principal, held_principal
			);
			let mut basis_frame_id = *oldest_frame;
			if let Some(old_bonded) = prebonded.get(vault_id) {
				if bonded_principal !=
					target_principal.saturating_sub(old_bonded.amount_unbonded).into()
				{
					warn!(
						"Warning: bonded principal {:?} does not match prebonded amount {:?} for vault ID {:?}",
						bonded_principal, old_bonded.amount_unbonded, vault_id
					);
				}
				basis_frame_id = old_bonded.starting_frame_id;
			}

			let elapsed_frames: T::Balance =
				(current_frame_id.saturating_sub(basis_frame_id) as u128).max(1).into();
			FunderStateByVaultAndAccount::<T>::insert(
				vault_id,
				vault_operator,
				FunderState::<T> {
					bonded_principal: bonded_principal.into(),
					target_principal,
					held_principal,
					lifetime_principal_last_basis_frame: basis_frame_id,
					lifetime_compounded_earnings: 0u32.into(),
					lifetime_principal_deployed: elapsed_frames * held_principal,
				},
			);

			count += 1;
		}
		old_storage::CapitalRaising::<T>::kill();

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use crate::VaultPoolsByFrame;
		use pallet_prelude::Decode;

		let old = <old_storage::Model<T>>::decode(&mut &state[..])
			.map_err(|_| {
				sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
			})?
			.vault_pools_by_frame;

		let new = VaultPoolsByFrame::<T>::iter().collect::<BTreeMap<_, _>>();
		let current_frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
		let old_filtered = old
			.into_iter()
			.filter(|(frame_id, _)| *frame_id <= current_frame_id)
			.collect::<BTreeMap<_, _>>();
		assert_eq!(old_filtered.len(), new.len(), "Storage values do not match after migration");
		for (frame_id, pools) in new {
			log::info!("Frame ID: {}, Pools: {:?}", frame_id, pools);
			let old_pools = old_filtered.get(&frame_id).expect("Vault pools not found");
			assert_eq!(pools.len(), old_pools.len(), "Mismatch in pools for frame ID {}", frame_id);
			for (vault_id, pool) in pools {
				let old_pool = old_pools.get(&vault_id).ok_or_else(|| {
					sp_runtime::TryRuntimeError::Other("Missing vault ID in old storage")
				})?;

				// Verify bond holders are preserved 1:1.
				assert_eq!(
					pool.bond_holders.len(),
					old_pool.bond_holders.len(),
					"Mismatch in bond holder count for vault ID {}",
					vault_id
				);

				for (account, old_bh) in &old_pool.bond_holders {
					let (_, new_bh) =
						pool.bond_holders.iter().find(|(a, _)| a == account).ok_or_else(|| {
							sp_runtime::TryRuntimeError::Other("Missing account in bond holders")
						})?;

					assert_eq!(
						*new_bh, old_bh.starting_balance,
						"Mismatch in starting balance for account {:?} in vault ID {}",
						account, vault_id
					);
				}

				// Verify pool-level fields we migrate.
				assert_eq!(
					pool.distributed_earnings, old_pool.distributed_earnings,
					"Mismatch in distributed_earnings for vault ID {}",
					vault_id
				);
				assert_eq!(
					pool.vault_sharing_percent, old_pool.vault_sharing_percent,
					"Mismatch in vault_sharing_percent for vault ID {}",
					vault_id
				);
			}
		}

		Ok(())
	}
}

pub type PalletMigrate<T> = frame_support::migrations::VersionedMigration<
	1,
	2,
	InnerMigrate<T>,
	crate::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::{
		VaultPoolsByFrame,
		mock::{Test, TestVault, insert_vault, new_test_ext},
	};
	use frame_support::assert_ok;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			insert_vault(
				42,
				TestVault {
					account_id: 42,
					activated: 100_000_000,
					is_closed: false,
					sharing_percent: Permill::from_percent(10),
				},
			);
			insert_vault(
				43,
				TestVault {
					account_id: 2,
					activated: 100_000_000,
					is_closed: false,
					sharing_percent: Permill::from_percent(10),
				},
			);
			old_storage::VaultPoolsByFrame::<Test>::mutate(1, |a| {
				a.try_insert(
					42,
					old_storage::TreasuryPool {
						bond_holders: BoundedVec::truncate_from(vec![
							(
								1u64.into(),
								old_storage::BondHolder {
									starting_balance: 1_000u128.into(),
									earnings: 0u128.into(),
									keep_earnings_in_pool: false,
								},
							),
							(
								2u64.into(),
								old_storage::BondHolder {
									starting_balance: 2_000u128.into(),
									earnings: 0u128.into(),
									keep_earnings_in_pool: false,
								},
							),
						]),
						do_not_renew: Default::default(),
						distributed_earnings: None,
						is_rolled_over: false,
						vault_sharing_percent: Permill::from_percent(10),
					},
				)
				.unwrap()
			});
			old_storage::VaultPoolsByFrame::<Test>::mutate(2, |a| {
				a.try_insert(
					43,
					old_storage::TreasuryPool {
						bond_holders: BoundedVec::truncate_from(vec![
							(
								3u64.into(),
								old_storage::BondHolder {
									starting_balance: 3_000u128.into(),
									earnings: 0u128.into(),
									keep_earnings_in_pool: false,
								},
							),
							(
								4u64.into(),
								old_storage::BondHolder {
									starting_balance: 4_000u128.into(),
									earnings: 0u128.into(),
									keep_earnings_in_pool: false,
								},
							),
						]),
						do_not_renew: Default::default(),
						distributed_earnings: None,
						is_rolled_over: false,
						vault_sharing_percent: Permill::from_percent(20),
					},
				)
				.unwrap()
			});
			old_storage::VaultPoolsByFrame::<Test>::mutate(10_000, |a| {
				a.try_insert(
					42,
					old_storage::TreasuryPool {
						bond_holders: BoundedVec::truncate_from(vec![(
							5u64.into(),
							old_storage::BondHolder {
								starting_balance: 5_000u128.into(),
								earnings: 0u128.into(),
								keep_earnings_in_pool: false,
							},
						)]),
						do_not_renew: Default::default(),
						distributed_earnings: None,
						is_rolled_over: false,
						vault_sharing_percent: Permill::from_percent(10),
					},
				)
				.unwrap()
			});

			// Get the pre_upgrade bytes
			let bytes = match InnerMigrate::<Test>::pre_upgrade() {
				Ok(bytes) => bytes,
				Err(e) => panic!("pre_upgrade failed: {:?}", e),
			};
			// Execute the migration
			let _weight = InnerMigrate::<Test>::on_runtime_upgrade();

			// Verify post_upgrade succeeds
			assert_ok!(InnerMigrate::<Test>::post_upgrade(bytes));

			// Verify the new storage has the expected values
			let new_value_1 = VaultPoolsByFrame::<Test>::get(1);
			let new_value_2 = VaultPoolsByFrame::<Test>::get(2);
			let new_value_10k = VaultPoolsByFrame::<Test>::get(10_000);
			assert_eq!(new_value_1.len(), 1);
			assert_eq!(new_value_2.len(), 1);
			assert_eq!(new_value_1.get(&42).unwrap().bond_holders.len(), 2);
			assert_eq!(new_value_2.get(&43).unwrap().bond_holders.len(), 2);
			assert_eq!(new_value_10k.len(), 0);
		});
	}
}
