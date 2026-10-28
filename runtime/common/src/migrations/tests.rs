use super::*;
use alloc::collections::{BTreeMap, BTreeSet};
use argon_primitives::{
	bitcoin::{BitcoinBlock, BitcoinCosignScriptPubkey, CompressedBitcoinPubkey, H256Le, UtxoRef},
	treasury::{PositionQuantities, UpstreamPosition},
	vault::{SecuritizationScheduleEntry, VaultTerms},
};
use argon_runtime::{BitcoinLocks, Runtime, TreasuryPositions};
#[cfg(feature = "try-runtime")]
use frame_support::assert_ok;
use pallet_bitcoin_fissions::Pallet as FissionsPallet;
use pallet_bitcoin_locks::Pallet as LocksPallet;
use pallet_bitcoin_utxos::{ConfirmedBitcoinBlockTip, SynchedBitcoinBlock};
use pallet_treasury_positions::{NetworkTotals, PositionsByAccount};
use pallet_vaults::{Pallet as VaultsPallet, RevenuePerFrameByVault, VaultFrameRevenue};
use polkadot_sdk::sp_io::{self, TestExternalities};
use sp_runtime::{bounded_btree_map, AccountId32, BuildStorage, StateVersion};

const VAULT_OPERATOR: AccountId32 = AccountId32::new([1; 32]);
const LOCK_OWNER: AccountId32 = AccountId32::new([2; 32]);
const OPERATIONAL_OWNER: AccountId32 = AccountId32::new([9; 32]);

fn new_test_ext() -> TestExternalities {
	argon_runtime::RuntimeGenesisConfig::default().build_storage().unwrap().into()
}

fn stored_vault() -> v19::Vault<Runtime> {
	v19::Vault {
		operator_account_id: VAULT_OPERATOR,
		delegate_account_id: Some(LOCK_OWNER),
		securitization: 1_000,
		securitization_target: 950,
		securitization_locked: 110,
		flexible_securitization_locked: 79,
		reserved_securitization_space: 50,
		securitization_pending_activation: 31,
		securitized_satoshis: 13,
		total_satoshis: 13,
		ratio_adjusted_satoshis: 8,
		flexible_ratio_adjusted_satoshis: 8,
		securitization_release_schedule: bounded_btree_map! {
			144 => SecuritizationScheduleEntry { locked_commitments: 101, ..Default::default() },
			432 => SecuritizationScheduleEntry { locked_commitments: 3, ..Default::default() },
			576 => SecuritizationScheduleEntry { locked_commitments: 6, relockable_commitments: 5, argon_withdrawals: 7, argonot_withdrawals: 11 },
		},
		committed_microgons: 500,
		securitization_ratio: FixedU128::from_u32(2),
		is_closed: true,
		terms: VaultTerms {
			bitcoin_annual_percent_rate: FixedU128::from_rational(1, 10),
			bitcoin_base_fee: 7,
		},
		pending_terms: Some((
			23,
			VaultTerms { bitcoin_annual_percent_rate: FixedU128::zero(), bitcoin_base_fee: 11 },
		)),
		opened_tick: 3,
	}
}

// Two historical Locks exercise retirement and partially funded rounding in one real Vault.
fn source_state() {
	StorageVersion::new(19).put::<VaultsPallet<Runtime>>();
	StorageVersion::new(11).put::<pallet_bitcoin_locks::Pallet<Runtime>>();
	StorageVersion::new(1).put::<pallet_bitcoin_fissions::Pallet<Runtime>>();
	frame_system::LastRuntimeUpgrade::<Runtime>::put(frame_system::LastRuntimeUpgradeInfo {
		spec_version: 160.into(),
		spec_name: "argon".into(),
	});
	let mut account = pallet_operational_accounts::OperationalAccount::<Runtime>::decode(
		&mut TrailingZeroInput::zeroes(),
	)
	.unwrap();
	account.vault_account = VAULT_OPERATOR;
	account.mining_account = AccountId32::new([3; 32]);
	account.vault_created = true;
	account.vault_bitcoin_accrual = 79;
	pallet_operational_accounts::OperationalAccounts::<Runtime>::insert(OPERATIONAL_OWNER, account);
	pallet_operational_accounts::OperationalAccountBySubAccount::<Runtime>::insert(
		VAULT_OPERATOR,
		OPERATIONAL_OWNER,
	);
	ConfirmedBitcoinBlockTip::<Runtime>::put(BitcoinBlock::new(12, H256Le([0; 32])));
	let mut old = v11::LockedBitcoin::<Runtime> {
		vault_id: 1,
		securitization_basis: v11::BitcoinSecuritizationBasis {
			satoshis: 10,
			microgons_at_target_per_btc: 123_000_000,
		},
		securitization_coverage_microgons: 9,
		securitization_tick: 7,
		funded_satoshis: 8,
		funding_utxos: bounded_btree_map! { UtxoRef { txid: H256Le([4; 32]), output_index: 2 } => 8 },
		fissioned_satoshis: 6,
		owner_account: LOCK_OWNER,
		securitization_ratio: FixedU128::one(),
		security_fees: 17,
		coupon_paid_fees: 5,
		vault_pubkey: CompressedBitcoinPubkey([2; 33]),
		vault_claim_pubkey: CompressedBitcoinPubkey([3; 33]),
		vault_xpub_sources: ([6; 4], 2, 3),
		owner_pubkey: CompressedBitcoinPubkey([4; 33]),
		vault_claim_height: 100,
		open_claim_height: 1_000,
		created_at_height: 1,
		securitization_hold_expiration_bitcoin_height: 70,
		utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH { wscript_hash: H256::repeat_byte(8) },
		is_flexible: true,
		// Runtime 160 could shrink coverage at expiry without trimming this map.
		fund_hold_extensions: bounded_btree_map! { 432 => 3, 576 => 20 },
		created_at_argon_block: 9,
	};
	v11::LocksById::<Runtime>::insert(1, &old);
	old.funded_satoshis = 5;
	old.fissioned_satoshis = 2;
	old.securitization_basis.satoshis = 7;
	old.securitization_coverage_microgons = 101;
	old.fund_hold_extensions.clear();
	v11::LocksById::<Runtime>::insert(2, old);
	LockIdsByVaultId::<Runtime>::insert(1, 1, ());
	LockIdsByVaultId::<Runtime>::insert(1, 2, ());
	SecuritizationHoldExpirationsByBitcoinHeight::<Runtime>::insert(
		70,
		BoundedBTreeSet::<_, <Runtime as LocksConfig>::MaxConcurrentlyExpiringLocks>::try_from(
			BTreeSet::from([2]),
		)
		.unwrap(),
	);
	pallet_bitcoin_locks::LastProcessedSecuritizationHoldBitcoinHeight::<Runtime>::put(60);
	for (lock_id, id, satoshis) in [(1, 0, 2), (1, 1, 4), (2, 2, 2)] {
		v1::FissionByOwnerAndId::<Runtime>::insert(
			LOCK_OWNER,
			id,
			v1::Fission::<Runtime> {
				liquid_id: id + 77,
				lock_id,
				satoshis,
				microgons_at_target_per_btc: 90_000_000,
				liquidity_promised: 2,
				created_at_argon_block: 3,
				ratchet_number: 1,
				last_ratchet_tick: 8,
				last_updated_argon_block: 4,
			},
		);
		FissionIdsByLockId::<Runtime>::mutate(lock_id, |ids| {
			ids.try_insert(id).unwrap();
		});
	}
	v19::VaultsById::<Runtime>::insert(1, stored_vault());
	TreasuryPositions::set_upstream_position(
		&LOCK_OWNER,
		Some(UpstreamPosition {
			vault_id: 1,
			bitcoin_allocated_securitization: 110,
			bitcoin_securitization: 79,
			..Default::default()
		}),
	);
	let mut revenue = VaultFrameRevenue::<Runtime>::new(3);
	revenue.bitcoin_lock_fee_revenue = 17;
	revenue.uncollected_revenue = 12;
	RevenuePerFrameByVault::<Runtime>::insert(1, BoundedVec::try_from(vec![revenue]).unwrap());
}

#[test]
fn migration_conserves_backing_and_reconciles_real_vault_and_treasury_storage() {
	new_test_ext().execute_with(|| {
		source_state();
		let revenue = RevenuePerFrameByVault::<Runtime>::get(1).encode();

		// Runtime 160 stored account totals without the Vault coverage field.
		let source_totals = PositionQuantities { fission_liquidity: 6, ..Default::default() };
		PositionsByAccount::<Runtime>::mutate(LOCK_OWNER, |position| {
			position.as_mut().unwrap().quantities = source_totals;
		});
		sp_io::storage::set(&NetworkTotals::<Runtime>::hashed_key(), &source_totals.encode());
		StorageVersion::new(1).put::<pallet_treasury_positions::Pallet<Runtime>>();

		#[cfg(feature = "try-runtime")]
		assert_ok!(argon_runtime::Executive::try_runtime_upgrade(
			polkadot_sdk::frame_try_runtime::UpgradeCheckSelect::PreAndPost,
		));
		#[cfg(not(feature = "try-runtime"))]
		argon_runtime::Executive::execute_on_runtime_upgrade();

		let network = NetworkTotals::<Runtime>::get();
		assert_eq!(network.account_quantities, source_totals);
		assert_eq!(network.vault_securitized_satoshis, 13);
		let first = LocksById::<Runtime>::get(1).unwrap();
		assert_eq!(
			(
				first.cosigner_guarantee.basis.satoshis,
				first.cosigner_guarantee.guaranteed_microgons
			),
			(2, 2)
		);
		assert_eq!(
			first.cosigner_guarantee.fund_hold_extensions.into_inner(),
			BTreeMap::from([(576, 2)])
		);
		assert_eq!(first.securitization_hold_expiration_bitcoin_height, None);
		let second = LocksById::<Runtime>::get(2).unwrap();
		assert_eq!(
			(
				second.cosigner_guarantee.basis.satoshis,
				second.cosigner_guarantee.guaranteed_microgons
			),
			(5, 73)
		);
		assert_eq!(second.securitization_hold_expiration_bitcoin_height, Some(70));
		assert_eq!(second.lock_held_securitization().activated_securitization, 43);
		for (id, amount) in [(0, 1), (1, 4), (2, 28)] {
			let fission = FissionByOwnerAndId::<Runtime>::get(LOCK_OWNER, id).unwrap();
			assert_eq!(fission.cosigner_guarantee.guaranteed_microgons, amount);
			assert_eq!(
				(fission.liquidity_promised, fission.last_ratchet_tick, fission.ratchet_number),
				(2, 8, 1)
			);
		}
		let vault = VaultsById::<Runtime>::get(1).unwrap();
		assert_eq!(
			(
				vault.securitization_locked,
				vault.securitization_pending_activation,
				vault.flexible_securitization_locked
			),
			(108, 30, 78)
		);
		assert_eq!((vault.total_satoshis, vault.securitized_satoshis), (13, 13));
		assert_eq!(
			pallet_operational_accounts::OperationalAccounts::<Runtime>::get(OPERATIONAL_OWNER)
				.unwrap()
				.vault_bitcoin_accrual,
			78
		);
		assert_eq!(
			vault.securitization_release_schedule[&576],
			SecuritizationScheduleEntry {
				locked_commitments: 4,
				relockable_commitments: 7,
				argon_withdrawals: 7,
				argonot_withdrawals: 11
			}
		);
		assert!(VaultFundsReleasingByHeight::<Runtime>::get(576).contains(&1));
		assert_eq!(RevenuePerFrameByVault::<Runtime>::get(1).encode(), revenue);
		let upstream = TreasuryPositions::upstream_position(&LOCK_OWNER).unwrap();
		assert_eq!(
			(upstream.bitcoin_allocated_securitization, upstream.bitcoin_securitization),
			(108, 78)
		);
		assert_eq!(StorageVersion::get::<VaultsPallet<Runtime>>(), StorageVersion::new(20));
		assert_eq!(
			StorageVersion::get::<pallet_bitcoin_locks::Pallet<Runtime>>(),
			StorageVersion::new(12)
		);
		assert_eq!(
			StorageVersion::get::<pallet_bitcoin_fissions::Pallet<Runtime>>(),
			StorageVersion::new(2)
		);
		frame_system::LastRuntimeUpgrade::<Runtime>::put(frame_system::LastRuntimeUpgradeInfo {
			spec_version: argon_runtime::VERSION.spec_version.into(),
			spec_name: "argon".into(),
		});
		let storage_before = sp_io::storage::root(StateVersion::V1);
		#[cfg(feature = "try-runtime")]
		assert_ok!(argon_runtime::Executive::try_runtime_upgrade(
			polkadot_sdk::frame_try_runtime::UpgradeCheckSelect::PreAndPost,
		));
		#[cfg(not(feature = "try-runtime"))]
		argon_runtime::Executive::execute_on_runtime_upgrade();
		assert_eq!(sp_io::storage::root(StateVersion::V1), storage_before);
		assert_eq!(VaultsById::<Runtime>::get(1).unwrap(), vault);
		assert_eq!(LocksById::<Runtime>::get(2).unwrap().encode(), second.encode());
		assert_eq!(TreasuryPositions::upstream_position(&LOCK_OWNER).unwrap(), upstream);
	});
}

#[test]
fn migration_uses_historical_rounding_for_source_securitization() {
	new_test_ext().execute_with(|| {
		source_state();
		// Runtime 160 assigned 3 of these 14 microgons to the Fission holding 2 of 7 sats.
		// Multiplying before division would assign 4 and change its historical guarantee.
		v11::LocksById::<Runtime>::mutate(2, |lock| {
			lock.as_mut().unwrap().securitization_coverage_microgons = 14;
		});
		v19::VaultsById::<Runtime>::mutate(1, |vault| {
			let vault = vault.as_mut().unwrap();
			vault.securitization_locked = 23;
			vault.securitization_pending_activation = 6;
			vault.flexible_securitization_locked = 17;
			vault.securitization_release_schedule.get_mut(&144).unwrap().locked_commitments = 14;
		});
		let mut position = TreasuryPositions::upstream_position(&LOCK_OWNER).unwrap();
		position.bitcoin_allocated_securitization = 23;
		position.bitcoin_securitization = 17;
		TreasuryPositions::set_upstream_position(&LOCK_OWNER, Some(position));
		pallet_operational_accounts::OperationalAccounts::<Runtime>::mutate(
			OPERATIONAL_OWNER,
			|account| {
				account.as_mut().unwrap().vault_bitcoin_accrual = 17;
			},
		);

		#[cfg(feature = "try-runtime")]
		let state = CosignerGuaranteesMigration::<Runtime>::pre_upgrade().unwrap();
		CosignerGuaranteesMigration::<Runtime>::on_runtime_upgrade();
		#[cfg(feature = "try-runtime")]
		assert_ok!(CosignerGuaranteesMigration::<Runtime>::post_upgrade(state));
		assert_eq!(
			FissionByOwnerAndId::<Runtime>::get(LOCK_OWNER, 2)
				.unwrap()
				.cosigner_guarantee
				.guaranteed_microgons,
			3
		);
		let vault = VaultsById::<Runtime>::get(1).unwrap();
		assert_eq!(vault.securitization_locked, 21);
		assert_eq!(vault.get_activated_securitization(), 16);
		assert_eq!(
			TreasuryPositions::upstream_position(&LOCK_OWNER)
				.unwrap()
				.bitcoin_securitization,
			16
		);
	});
}

#[test]
fn migration_leaves_the_funding_window_for_normal_expiry() {
	use frame_support::traits::Hooks;
	new_test_ext().execute_with(|| {
		source_state();
		CosignerGuaranteesMigration::<Runtime>::on_runtime_upgrade();
		// Expiry only runs through Bitcoin's synchronized height.
		ConfirmedBitcoinBlockTip::<Runtime>::put(BitcoinBlock::new(71, H256Le([0; 32])));
		SynchedBitcoinBlock::<Runtime>::put(BitcoinBlock::new(71, H256Le([0; 32])));
		BitcoinLocks::on_initialize(20);
		let lock = LocksById::<Runtime>::get(2).unwrap();
		assert_eq!(lock.securitization_hold_expiration_bitcoin_height, None);
		assert_eq!(
			(lock.cosigner_guarantee.basis.satoshis, lock.cosigner_guarantee.guaranteed_microgons),
			(3, 43)
		);
		let vault = VaultsById::<Runtime>::get(1).unwrap();
		assert_eq!(
			(
				vault.securitization_locked,
				vault.securitization_pending_activation,
				vault.flexible_securitization_locked
			),
			(78, 0, 78)
		);
		let upstream = TreasuryPositions::upstream_position(&LOCK_OWNER).unwrap();
		assert_eq!(
			(upstream.bitcoin_allocated_securitization, upstream.bitcoin_securitization),
			(78, 78)
		);
	});
}

#[cfg(feature = "try-runtime")]
#[test]
fn preflight_rejects_inconsistent_source_and_postflight_checks_downstream_state() {
	new_test_ext().execute_with(|| {
		source_state();
		v19::VaultsById::<Runtime>::mutate(1, |vault| {
			vault.as_mut().unwrap().securitization_pending_activation += 1
		});
		assert!(CosignerGuaranteesMigration::<Runtime>::pre_upgrade().is_err());
		v19::VaultsById::<Runtime>::mutate(1, |vault| {
			vault.as_mut().unwrap().securitization_pending_activation -= 1
		});
		let state = CosignerGuaranteesMigration::<Runtime>::pre_upgrade().unwrap();
		CosignerGuaranteesMigration::<Runtime>::on_runtime_upgrade();
		assert_ok!(CosignerGuaranteesMigration::<Runtime>::post_upgrade(state.clone()));
		VaultsById::<Runtime>::mutate(1, |vault| {
			vault.as_mut().unwrap().securitization_pending_activation += 1
		});
		assert!(CosignerGuaranteesMigration::<Runtime>::post_upgrade(state.clone()).is_err());
		VaultsById::<Runtime>::mutate(1, |vault| {
			vault.as_mut().unwrap().securitization_pending_activation -= 1
		});
		let position = TreasuryPositions::upstream_position(&LOCK_OWNER).unwrap();
		TreasuryPositions::set_upstream_position(
			&LOCK_OWNER,
			Some(UpstreamPosition { bitcoin_securitization: 77, ..position }),
		);
		assert!(CosignerGuaranteesMigration::<Runtime>::post_upgrade(state.clone()).is_err());
		TreasuryPositions::set_upstream_position(&LOCK_OWNER, Some(position));
		VaultFundsReleasingByHeight::<Runtime>::remove(576);
		assert!(CosignerGuaranteesMigration::<Runtime>::post_upgrade(state).is_err());
	});
}

#[test]
fn migration_leaves_storage_untouched_unless_all_three_versions_match() {
	for (vaults, locks, fissions) in [
		(18, 11, 1),
		(20, 11, 1),
		(21, 11, 1),
		(19, 10, 1),
		(19, 12, 1),
		(19, 13, 1),
		(19, 11, 0),
		(19, 11, 2),
		(19, 11, 3),
	] {
		new_test_ext().execute_with(|| {
			source_state();
			StorageVersion::new(vaults).put::<VaultsPallet<Runtime>>();
			StorageVersion::new(locks).put::<LocksPallet<Runtime>>();
			StorageVersion::new(fissions).put::<FissionsPallet<Runtime>>();
			let storage_before = sp_io::storage::root(StateVersion::V1);

			#[cfg(feature = "try-runtime")]
			assert!(CosignerGuaranteesMigration::<Runtime>::try_on_runtime_upgrade(true).is_err());
			let weight = CosignerGuaranteesMigration::<Runtime>::on_runtime_upgrade();
			assert_eq!(weight, <Runtime as frame_system::Config>::DbWeight::get().reads(3));
			assert_eq!(sp_io::storage::root(StateVersion::V1), storage_before);
		});
	}
}
