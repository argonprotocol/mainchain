use pallet_prelude::*;
use pallet_treasury::{Config as TreasuryConfig, Pallet as TreasuryPallet};

/// Deployed stake scalar removed by Treasury's runtime 160 migration.
#[frame_support::storage_alias]
type TotalActiveArgonotBonds<T: TreasuryConfig> =
	StorageValue<TreasuryPallet<T>, pallet_treasury::Bonds, ValueQuery>;

#[cfg(test)]
mod tests {
	use super::TotalActiveArgonotBonds;
	use argon_primitives::{
		bitcoin::{BitcoinCosignScriptPubkey, CompressedBitcoinPubkey},
		treasury::{
			BitcoinLockPosition, PositionQuantities, TreasuryPositionProvider, UpstreamPosition,
		},
		vault::BitcoinSecuritizationBasis,
		MICROGONS_PER_ARGON,
	};
	use argon_runtime::{
		Balances, Executive, Mint, Runtime, RuntimeEvent, RuntimeGenesisConfig, System, Treasury,
		TreasuryPositions, Vaults,
	};
	use frame_support::traits::StorageVersion;
	#[cfg(feature = "try-runtime")]
	use frame_support::traits::UpgradeCheckSelect;
	use pallet_bitcoin_fissions::{
		Fission, FissionByOwnerAndId, FissionIdsByLockId, NextFissionIdByOwner,
	};
	use pallet_bitcoin_locks::LocksById;
	use pallet_operational_accounts::{
		OperationalAccount, OperationalAccountBySubAccount, OperationalAccounts as Accounts,
	};
	use pallet_prelude::*;
	use pallet_treasury::{
		migrations::old as old_treasury, ArgonotBondLots, BondLotById, BondLotIdsByAccount,
		BondLotIdsByVault, BondLotSummary, BondLotsByVault, BondProgram, BondReleaseReason,
		CurrentFrameVaultCapital,
	};
	use pallet_vaults::migrations::old as old_vaults;
	use polkadot_sdk::{
		sp_io,
		sp_runtime::{self, AccountId32 as AccountId, BuildStorage},
	};

	#[frame_support::storage_alias]
	type MintedMiningMicrogons<T: pallet_mint::Config> =
		StorageValue<pallet_mint::Pallet<T>, <T as pallet_mint::Config>::Balance, ValueQuery>;

	#[test]
	fn runtime_upgrade_converts_sources_and_seeds_treasury_positions_once() {
		polkadot_sdk::sp_tracing::try_init_simple();
		sp_io::TestExternalities::new(RuntimeGenesisConfig::default().build_storage().unwrap())
			.execute_with(|| {
				System::set_block_number(1);
				let upstream_owner = AccountId::new([1; 32]);
				let upstream_vault = AccountId::new([2; 32]);
				let downstream_owner = AccountId::new([3; 32]);
				let downstream_vault = AccountId::new([4; 32]);
				let unrelated = AccountId::new([5; 32]);
				let upstream = OperationalAccount::<Runtime> {
					vault_account: upstream_vault.clone(),
					mining_account: AccountId::new([6; 32]),
					encryption_pubkey: Default::default(),
					upstream_account: None,
					name: None,
					last_name_change_tick: None,
					uniswap_argon_transfers_in_amount: 0,
					account_bitcoin_amount: 999,
					account_vault_bond_amount: 999,
					vault_created: true,
					vault_bitcoin_accrual: 0,
					vault_bitcoin_applied_total: 0,
					mining_seat_accrual: 0,
					mining_seat_applied_total: 0,
					operational_certifications_count: 0,
					available_access_codes: 0,
					rewards_earned_count: 0,
					rewards_earned_amount: 0,
					rewards_collected_amount: 0,
					is_operationally_certified: false,
				};
				let mut downstream = upstream.clone();
				downstream.vault_account = downstream_vault.clone();
				downstream.mining_account = AccountId::new([7; 32]);
				downstream.upstream_account = Some(upstream_owner.clone());
				Accounts::<Runtime>::insert(&upstream_owner, upstream);
				Accounts::<Runtime>::insert(&downstream_owner, downstream);
				OperationalAccountBySubAccount::<Runtime>::insert(&upstream_vault, &upstream_owner);
				OperationalAccountBySubAccount::<Runtime>::insert(
					&downstream_vault,
					&downstream_owner,
				);
				for (vault_id, operator, bonds, locked, pending) in [
					(1, upstream_vault.clone(), 15, 200, 100),
					(2, downstream_vault.clone(), 20, 0, 0),
					(9, unrelated.clone(), 7, 0, 0),
				] {
					pallet_vaults::VaultIdByOperator::<Runtime>::insert(&operator, vault_id);
					old_vaults::VaultsById::<Runtime>::insert(
						vault_id,
						old_vaults::Vault {
							operator_account_id: operator,
							delegate_account_id: None,
							securitization: bonds * MICROGONS_PER_ARGON,
							securitization_target: bonds * MICROGONS_PER_ARGON,
							securitization_locked: locked,
							flexible_securitization_locked: 0,
							reserved_securitization_space: 0,
							securitization_pending_activation: pending,
							securitized_satoshis: if vault_id == 1 { 50 } else { 0 },
							total_satoshis: if vault_id == 1 { 50 } else { 0 },
							ratio_adjusted_satoshis: if vault_id == 1 { 100 } else { 0 },
							flexible_ratio_adjusted_satoshis: 0,
							securitization_release_schedule: Default::default(),
							securitization_ratio: FixedU128::from_u32(2),
							is_closed: false,
							terms: old_vaults::VaultTerms {
								bitcoin_annual_percent_rate: FixedU128::zero(),
								bitcoin_base_fee: 0,
								treasury_profit_sharing: sp_runtime::Permill::from_percent(30),
							},
							pending_terms: None,
							opened_tick: 0,
							operational_minimum_release_tick: None,
						},
					);
				}
				let upstream_bonds = BondProgram::Vault {
					vault_id: 1,
					sharing_percent: sp_runtime::Permill::zero(),
					bonus_percent: sp_runtime::Permill::zero(),
				};
				let other_bonds = BondProgram::Vault {
					vault_id: 9,
					sharing_percent: sp_runtime::Permill::zero(),
					bonus_percent: sp_runtime::Permill::zero(),
				};
				for (id, owner, program, bonds, flexible, releasing) in [
					(1, downstream_vault.clone(), upstream_bonds, 10, false, false),
					(2, upstream_vault.clone(), upstream_bonds, 5, true, false),
					(3, downstream_vault.clone(), other_bonds, 7, false, false),
					(4, downstream_vault.clone(), upstream_bonds, 3, false, true),
					(5, downstream_vault.clone(), BondProgram::Argonot, 20, false, false),
					(6, unrelated.clone(), upstream_bonds, 3, false, false),
				] {
					BondLotIdsByAccount::<Runtime>::insert(&owner, id, ());
					old_treasury::BondLotById::<Runtime>::insert(
						id,
						old_treasury::BondLot {
							owner,
							program,
							bonds,
							is_flexible: flexible,
							created_frame_id: 0,
							participated_frames: 0,
							last_frame_earnings_frame_id: None,
							last_frame_earnings: None,
							cumulative_earnings: 0,
							release_frame_id: releasing.then_some(10),
							release_reason: releasing.then_some(BondReleaseReason::UserLiquidation),
						},
					);
				}
				pallet_bitcoin_locks::LocksById::<Runtime>::insert(
					1,
					pallet_bitcoin_locks::LockedBitcoin {
						vault_id: 1,
						owner_account: downstream_vault.clone(),
						securitization_basis: BitcoinSecuritizationBasis {
							satoshis: 100,
							microgons_at_target_per_btc: 100_000_000,
						},
						securitization_coverage_microgons: 100,
						securitization_tick: 0,
						funded_satoshis: 50,
						funding_utxos: Default::default(),
						fissioned_satoshis: 30,
						securitization_ratio: FixedU128::from_u32(2),
						security_fees: 0,
						coupon_paid_fees: 0,
						vault_pubkey: CompressedBitcoinPubkey([0; 33]),
						vault_claim_pubkey: CompressedBitcoinPubkey([0; 33]),
						vault_xpub_sources: ([0; 4], 0, 0),
						owner_pubkey: CompressedBitcoinPubkey([0; 33]),
						vault_claim_height: 100,
						open_claim_height: 110,
						created_at_height: 0,
						securitization_hold_expiration_bitcoin_height: 5,
						utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
							wscript_hash: Default::default(),
						},
						is_flexible: false,
						fund_hold_extensions: Default::default(),
						created_at_argon_block: 1,
					},
				);
				pallet_bitcoin_locks::LockIdsByOwnerAccount::<Runtime>::insert(
					&downstream_vault,
					1,
					(),
				);
				old_treasury::BondLotsByVault::<Runtime>::insert(
					1,
					old_treasury::VaultBondState {
						regular_bond_lots: vec![
							BondLotSummary { bond_lot_id: 1, bonds: 10 },
							BondLotSummary { bond_lot_id: 6, bonds: 3 },
						]
						.try_into()
						.unwrap(),
						flexible_bonds: 5,
						reserved_bond_space: 0,
					},
				);
				old_treasury::BondLotsByVault::<Runtime>::insert(
					9,
					old_treasury::VaultBondState {
						regular_bond_lots: vec![BondLotSummary { bond_lot_id: 3, bonds: 7 }]
							.try_into()
							.unwrap(),
						..Default::default()
					},
				);
				ArgonotBondLots::<Runtime>::put(
					BoundedVec::try_from(vec![BondLotSummary { bond_lot_id: 5, bonds: 20 }])
						.unwrap(),
				);
				TotalActiveArgonotBonds::<Runtime>::put(20);
				for (id, satoshis) in [(0, 20u64), (1, 10u64)] {
					FissionByOwnerAndId::<Runtime>::insert(
						&downstream_vault,
						id,
						Fission {
							liquid_id: 1,
							lock_id: 1,
							satoshis,
							microgons_at_target_per_btc: 100_000_000,
							liquidity_promised: satoshis.into(),
							last_ratchet_tick: 0,
							ratchet_number: 0,
							created_at_argon_block: 1,
							last_updated_argon_block: 1,
						},
					);
					FissionIdsByLockId::<Runtime>::mutate(1, |ids| {
						ids.try_insert(id).unwrap();
					});
				}
				NextFissionIdByOwner::<Runtime>::insert(&downstream_vault, 2);
				let bid_pool = <Runtime as pallet_treasury::Config>::MiningBidPoolAccount::get();
				let reserves = <Runtime as pallet_treasury::Config>::TreasuryReservesAccount::get();
				frame_support::assert_ok!(Balances::mint_into(
					&bid_pool,
					100 * MICROGONS_PER_ARGON
				));
				frame_support::assert_ok!(Balances::mint_into(&reserves, 5 * MICROGONS_PER_ARGON));
				let issuance = Balances::total_issuance();
				// Run the configured runtime tuple: Mint 3->4, Vaults 18->19, and Treasury 8->9
				// including bond conversion and position initialization under the same guard.
				pallet_mining_slot::NextFrameId::<Runtime>::put(8);
				StorageVersion::new(3).put::<Mint>();
				StorageVersion::new(18).put::<Vaults>();
				StorageVersion::new(8).put::<Treasury>();
				MintedMiningMicrogons::<Runtime>::put(123);
				// A newly added pallet has no stored version; FRAME reads it as zero.
				sp_io::storage::clear(&StorageVersion::storage_key::<TreasuryPositions>());
				assert_eq!(StorageVersion::get::<TreasuryPositions>(), StorageVersion::new(0));
				#[cfg(feature = "try-runtime")]
				Executive::try_runtime_upgrade(UpgradeCheckSelect::PreAndPost).unwrap();
				#[cfg(not(feature = "try-runtime"))]
				Executive::execute_on_runtime_upgrade();
				assert_eq!(StorageVersion::get::<Mint>(), StorageVersion::new(4));
				assert_eq!(StorageVersion::get::<Vaults>(), StorageVersion::new(19));
				assert_eq!(StorageVersion::get::<Treasury>(), StorageVersion::new(9));
				assert_eq!(Balances::free_balance(&bid_pool), 100 * MICROGONS_PER_ARGON);
				assert_eq!(Balances::free_balance(&reserves), 5 * MICROGONS_PER_ARGON);
				assert_eq!(Balances::total_issuance(), issuance);
				assert!(!System::events().iter().any(|record| matches!(
					record.event,
					RuntimeEvent::Treasury(pallet_treasury::Event::FrameEarningsDistributed { .. })
				)));
				assert!(!MintedMiningMicrogons::<Runtime>::exists());
				assert_eq!(BondLotById::<Runtime>::iter().count(), 6);
				assert_eq!(BondLotsByVault::<Runtime>::get(1).regular_bonds, 13);
				assert_eq!(BondLotsByVault::<Runtime>::get(1).flexible_bonds, 5);
				assert_eq!(BondLotsByVault::<Runtime>::get(1).displaced_flexible_bonds, 3);
				let flexible_lot = BondLotById::<Runtime>::get(2).unwrap();
				assert!(flexible_lot.is_flexible);
				assert_eq!(flexible_lot.bonds, 5);
				assert!(BondLotIdsByVault::<Runtime>::contains_key(1, 4));
				assert_eq!(
					pallet_vaults::TotalVaultSecuritization::<Runtime>::get(),
					42 * MICROGONS_PER_ARGON
				);
				assert_eq!(LocksById::<Runtime>::get(1).unwrap().funded_satoshis, 50);
				assert_eq!(StorageVersion::get::<TreasuryPositions>(), StorageVersion::new(1));
				assert!(!TotalActiveArgonotBonds::<Runtime>::exists());
				assert_eq!(
					TreasuryPositions::account_quantities(&downstream_vault),
					PositionQuantities { bonds: 17, stakes: 20, fission_liquidity: 30 }
				);
				assert_eq!(TreasuryPositions::account_quantities(&upstream_vault).bonds, 2);
				assert_eq!(
					TreasuryPositions::network_totals(),
					PositionQuantities { bonds: 22, stakes: 20, fission_liquidity: 30 }
				);
				assert_eq!(
					Accounts::<Runtime>::get(&downstream_owner).unwrap().account_bitcoin_amount,
					30
				);
				assert_eq!(
					Accounts::<Runtime>::get(&upstream_owner).unwrap().account_bitcoin_amount,
					0
				);
				let capital = CurrentFrameVaultCapital::<Runtime>::get().unwrap();
				assert_eq!(capital.frame_id, 7);
				assert_eq!(capital.total_active_bonds, 22);
				assert_eq!(
					capital.vault_securitization_positions[&2].upstream_participation,
					FixedU128::from_rational(100, 15 * MICROGONS_PER_ARGON)
						.saturating_mul(FixedU128::from_rational(1, 2))
						.saturating_add(FixedU128::from_rational(1, 2))
				);
				assert_eq!(
					TreasuryPositions::bond_principal(&downstream_vault),
					17 * MICROGONS_PER_ARGON
				);
				assert_eq!(TreasuryPositions::bond_principal(&unrelated), 3 * MICROGONS_PER_ARGON);
				assert_eq!(
					TreasuryPositions::upstream_position(&downstream_vault),
					Some(UpstreamPosition {
						vault_id: 1,
						bitcoin_securitization: 100,
						bitcoin_allocated_securitization: 200,
						bond_principal: 10 * MICROGONS_PER_ARGON,
					})
				);
				assert_eq!(TreasuryPositions::upstream_position(&upstream_vault), None);
				assert_eq!(TreasuryPositions::upstream_position(&unrelated), None);
				assert_eq!(
					Accounts::<Runtime>::get(&downstream_owner).unwrap().account_vault_bond_amount,
					17 * MICROGONS_PER_ARGON
				);
				assert_eq!(
					Accounts::<Runtime>::get(&upstream_owner).unwrap().account_vault_bond_amount,
					5 * MICROGONS_PER_ARGON
				);
				// Registration initialization rebuilds the upstream subset from canonical owner
				// indexes.
				let upstream_position = TreasuryPositions::upstream_position(&downstream_vault);
				TreasuryPositions::set_upstream_position(&downstream_vault, None);
				frame_support::assert_ok!(TreasuryPositions::operational_account_registered(
					&downstream_vault
				));
				assert_eq!(
					TreasuryPositions::upstream_position(&downstream_vault),
					upstream_position
				);
				assert_eq!(
					TreasuryPositions::bond_principal(&downstream_vault),
					17 * MICROGONS_PER_ARGON
				);
				#[cfg(feature = "try-runtime")]
				Executive::try_runtime_upgrade(UpgradeCheckSelect::PreAndPost).unwrap();
				#[cfg(not(feature = "try-runtime"))]
				Executive::execute_on_runtime_upgrade();
				assert_eq!(
					TreasuryPositions::bond_principal(&downstream_vault),
					17 * MICROGONS_PER_ARGON
				);
				assert_eq!(
					TreasuryPositions::upstream_position(&downstream_vault)
						.unwrap()
						.bitcoin_securitization,
					100
				);
				assert_eq!(
					TreasuryPositions::network_totals(),
					PositionQuantities { bonds: 22, stakes: 20, fission_liquidity: 30 }
				);
				frame_support::assert_ok!(TreasuryPositions::bond_position_updated(
					&downstream_vault,
					1,
					10 * MICROGONS_PER_ARGON,
					0
				));
				frame_support::assert_ok!(TreasuryPositions::bitcoin_position_updated(
					&downstream_vault,
					1,
					BitcoinLockPosition {
						activated_securitization: 100,
						allocated_securitization: 200
					},
					BitcoinLockPosition::default(),
				));
				assert_eq!(
					TreasuryPositions::bond_principal(&downstream_vault),
					7 * MICROGONS_PER_ARGON
				);
				assert_eq!(
					TreasuryPositions::upstream_position(&downstream_vault).unwrap().bond_principal,
					0 * MICROGONS_PER_ARGON
				);
			});
	}
}
