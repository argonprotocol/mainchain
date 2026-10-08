use crate::{
	weights::WeightInfo as TreasuryWeightInfo, BondLot, BondLotById, BondLotId, BondLotIdsByVault,
	BondLotSummary, BondLotsByVault, BondProgram, BondReleaseReason, Bonds,
	Config as TreasuryConfig, CurrentFrameArgonotBondParticipants, Pallet as TreasuryPallet,
	TotalArgonBondLots, VaultBondState,
};
use argon_primitives::{
	treasury::{
		BitcoinLockPosition, PositionQuantity, TreasuryPositionProvider,
		TreasuryPositionProviderWeightInfo, UpstreamPosition,
	},
	vault::TreasuryVaultProvider,
	Balance, MiningFrameTransitionProvider, OperationalAccountProvider,
	OperationalAccountProviderWeightInfo, OperationalAccountsHook,
	OperationalAccountsHookWeightInfo, MICROGONS_PER_ARGON,
};
use core::marker::PhantomData;
use frame_support::{
	migrations::VersionedMigration, storage_alias, traits::UncheckedOnRuntimeUpgrade,
};
use pallet_bitcoin_fissions::{Config as BitcoinFissionsConfig, FissionByOwnerAndId};
use pallet_bitcoin_locks::{Config as BitcoinLocksConfig, LocksById};
use pallet_operational_accounts::{
	Config as OperationalAccountsConfig, OperationalAccounts, Pallet as OperationalAccountsPallet,
};
use pallet_prelude::*;
use pallet_treasury_positions::{
	Config as PositionsConfig, Pallet as PositionsPallet, ProviderWeightAdapter as PositionWeights,
};
use polkadot_sdk::sp_runtime::AccountId32 as AccountId;
#[cfg(feature = "try-runtime")]
use {
	crate::CurrentFrameVaultCapital,
	alloc::collections::BTreeMap,
	argon_primitives::treasury::PositionQuantities,
	pallet_treasury_positions::{NetworkTotals, Position, PositionsByAccount},
	polkadot_sdk::sp_runtime::TryRuntimeError,
};

/// The Treasury scalar replaced by the shared position totals in runtime 160.
#[frame_support::storage_alias]
type TotalActiveArgonotBonds<T: TreasuryConfig> =
	StorageValue<TreasuryPallet<T>, Bonds, ValueQuery>;

type TreasuryWeights<T> = <T as TreasuryConfig>::WeightInfo;
type OperationalAccountWeights<T> =
	<OperationalAccountsPallet<T> as OperationalAccountProvider<AccountId>>::Weights;

/// Convert Treasury's bond records, seed account positions, and freeze the current frame.
pub struct SeedTreasuryState<T>(PhantomData<T>);
impl<T> UncheckedOnRuntimeUpgrade for SeedTreasuryState<T>
where
	T: frame_system::Config<AccountId = AccountId>
		+ PositionsConfig<Balance = Balance>
		+ OperationalAccountsConfig<Balance = Balance>
		+ BitcoinLocksConfig<Balance = Balance>
		+ BitcoinFissionsConfig<Balance = Balance>
		+ TreasuryConfig<Balance = Balance>,
{
	fn on_runtime_upgrade() -> Weight {
		let db = T::DbWeight::get();
		// Each source iterator performs a final lookup when it is exhausted.
		// VersionedMigration accounts for Treasury's version read and write separately.
		let mut weight = db.reads(8);

		// Convert the deployed bond layouts and build their vault index.
		let mut reads = 0u64;
		let mut writes = 0u64;
		let mut vault_bond_lot_count = 0u32;
		BondLotById::<T>::translate::<old::BondLot<T>, _>(|bond_lot_id, lot| {
			reads.saturating_accrue(1);
			writes.saturating_accrue(1);
			if let BondProgram::Vault { vault_id, .. } = lot.program {
				BondLotIdsByVault::<T>::insert(vault_id, bond_lot_id, ());
				vault_bond_lot_count.saturating_accrue(1);
				writes.saturating_accrue(1);
			}
			Some(BondLot {
				owner: lot.owner,
				program: lot.program,
				bonds: lot.bonds,
				is_flexible: lot.is_flexible,
				locked_frame_terms: None,
				created_frame_id: lot.created_frame_id,
				participated_frames: lot.participated_frames,
				last_frame_earnings_frame_id: lot.last_frame_earnings_frame_id,
				last_frame_earnings: lot.last_frame_earnings,
				cumulative_earnings: lot.cumulative_earnings,
				release_frame_id: lot.release_frame_id,
				release_reason: lot.release_reason,
			})
		});
		TotalArgonBondLots::<T>::put(vault_bond_lot_count);
		writes.saturating_accrue(1);
		BondLotsByVault::<T>::translate::<old::VaultBondState, _>(|vault_id, bonds| {
			reads.saturating_accrue(2);
			writes.saturating_accrue(1);
			let regular_bonds = bonds
				.regular_bond_lots
				.iter()
				.fold(0u32, |total, lot| total.saturating_add(lot.bonds));
			let securitization =
				<T as TreasuryConfig>::TreasuryVaultProvider::get_vault_securitization(vault_id)
					.unwrap_or_default();
			let capacity = (securitization / MICROGONS_PER_ARGON).min(Bonds::MAX.into()) as Bonds;
			let displaced_flexible_bonds =
				bonds.flexible_bonds.saturating_sub(capacity.saturating_sub(regular_bonds));
			Some(VaultBondState {
				regular_bonds,
				flexible_bonds: bonds.flexible_bonds,
				displaced_flexible_bonds,
				locked_frame_terms: None,
				reserved_bond_space: bonds.reserved_bond_space,
			})
		});
		weight = weight.saturating_add(db.reads_writes(reads, writes));

		// Bind the single upstream before any source quantities are added.
		let bind_account_weight = db
			.reads(2)
			.saturating_add(OperationalAccountWeights::<T>::upstream_vault())
			.saturating_add(PositionWeights::<T>::set_upstream_position());
		for (_, account) in OperationalAccounts::<T>::iter() {
			let upstream = OperationalAccountsPallet::<T>::upstream_vault(&account.vault_account)
				.map(|(_, vault_id)| UpstreamPosition { vault_id, ..Default::default() });
			PositionsPallet::<T>::set_upstream_position(&account.vault_account, upstream);
			weight = weight.saturating_add(bind_account_weight);
		}

		// Replace zero contributions with each canonical live bond/Bitcoin position.
		// position_updated already includes its Operational Accounts provider cost.
		let position_update_weight = PositionWeights::<T>::position_updated();
		for (_, lot) in BondLotById::<T>::iter() {
			if lot.release_reason.is_none() {
				let quantity = match lot.program {
					BondProgram::Vault { .. } if !lot.is_flexible =>
						Some((PositionQuantity::Bonds, u128::from(lot.bonds))),
					BondProgram::Argonot => Some((PositionQuantity::Stakes, u128::from(lot.bonds))),
					_ => None,
				};
				if let Some((quantity, amount)) = quantity {
					PositionsPallet::<T>::account_quantity_updated(&lot.owner, quantity, 0, amount)
						.expect("canonical position quantities fit their types");
					weight =
						weight.saturating_add(PositionWeights::<T>::account_quantity_updated());
				}
			}
			if let BondProgram::Vault { vault_id, .. } = lot.program &&
				lot.release_reason.is_none()
			{
				PositionsPallet::<T>::bond_position_updated(
					&lot.owner,
					vault_id,
					0,
					Balance::from(lot.bonds).saturating_mul(MICROGONS_PER_ARGON),
				)
				.expect("canonical bond principal fits in the balance type");
				weight = weight.saturating_add(position_update_weight);
			}
			weight = weight.saturating_add(db.reads(2));
		}

		// Flexible principal belongs to the operator; only its undisplaced quantity earns.
		for (vault_id, bonds) in BondLotsByVault::<T>::iter() {
			let eligible_flexible = bonds.eligible_flexible_bonds();
			if eligible_flexible > 0 {
				let operator =
					<T as TreasuryConfig>::TreasuryVaultProvider::get_vault_operator(vault_id)
						.expect("a flexible bond position has a vault operator");
				PositionsPallet::<T>::account_quantity_updated(
					&operator,
					PositionQuantity::Bonds,
					0,
					eligible_flexible,
				)
				.expect("canonical flexible quantities fit their types");
				weight = weight
					.saturating_add(PositionWeights::<T>::account_quantity_updated())
					.saturating_add(db.reads(1));
			}
			weight = weight.saturating_add(db.reads(2));
		}

		for (_, lock) in LocksById::<T>::iter() {
			PositionsPallet::<T>::bitcoin_position_updated(
				&lock.owner_account,
				lock.vault_id,
				BitcoinLockPosition::default(),
				lock.upstream_collateral(),
			)
			.expect("canonical Bitcoin collateral fits in the balance type");
			weight = weight.saturating_add(db.reads(2)).saturating_add(position_update_weight);
		}

		// Active Fissions count in full, including liquidity whose mint is still pending.
		for (account, _, fission) in FissionByOwnerAndId::<T>::iter() {
			PositionsPallet::<T>::account_quantity_updated(
				&account,
				PositionQuantity::FissionLiquidity,
				0u128,
				fission.liquidity_promised,
			)
			.expect("canonical Fission liquidity fits in the balance type");
			weight = weight
				.saturating_add(db.reads(2))
				.saturating_add(PositionWeights::<T>::account_quantity_updated());
		}

		// Remove the replaced Treasury stake total; no second authoritative copy remains.
		TotalActiveArgonotBonds::<T>::kill();
		weight = weight.saturating_add(db.writes(1));

		// Bring the existing certification cache into agreement with the new account totals.
		let certification_weight = db
			.reads(2)
			.saturating_add(PositionWeights::<T>::bond_principal())
			.saturating_add(PositionWeights::<T>::account_quantities())
			.saturating_add(OperationalAccountWeights::<T>::account_vault_bond_total_updated())
			.saturating_add(OperationalAccountWeights::<T>::account_bitcoin_amount_changed());
		for (_, account) in OperationalAccounts::<T>::iter() {
			let liquidity =
				PositionsPallet::<T>::account_quantities(&account.vault_account).fission_liquidity;
			let previous = account.account_bitcoin_amount;
			let (change, is_increase) = if liquidity >= previous {
				(liquidity - previous, true)
			} else {
				(previous - liquidity, false)
			};
			OperationalAccountsPallet::<T>::account_bitcoin_amount_changed(
				&account.vault_account,
				change,
				is_increase,
			);
			OperationalAccountsPallet::<T>::account_vault_bond_total_updated(
				&account.vault_account,
				PositionsPallet::<T>::bond_principal(&account.vault_account),
			);
			weight = weight.saturating_add(certification_weight);
		}

		// Freeze the current frame once, using the completed totals and upstream positions.
		// This does not distribute earnings or advance the mining frame.
		let frame_id = <T as TreasuryConfig>::MiningFrameTransitionProvider::get_current_frame_id();
		TreasuryPallet::<T>::lock_in_vault_capital(frame_id);
		let argonot_participants = CurrentFrameArgonotBondParticipants::<T>::get()
			.map(|participants| participants.bond_lots.len() as u32)
			.unwrap_or_default();
		weight = weight.saturating_add(db.reads(2)).saturating_add(
			TreasuryWeights::<T>::on_frame_transition(
				vault_bond_lot_count,
				argonot_participants,
				0,
			),
		);
		StorageVersion::new(1).put::<PositionsPallet<T>>();
		weight.saturating_add(db.writes(1))
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let frame_id = <T as TreasuryConfig>::MiningFrameTransitionProvider::get_current_frame_id();
		let bid_pool_balance =
			<T as TreasuryConfig>::Currency::balance(&T::MiningBidPoolAccount::get());
		let reserves_balance =
			<T as TreasuryConfig>::Currency::balance(&T::TreasuryReservesAccount::get());
		let issuance = <T as TreasuryConfig>::Currency::total_issuance();
		ensure!(
			PositionsByAccount::<T>::iter().next().is_none(),
			TryRuntimeError::Other("new position storage is not empty")
		);
		let accounts = OperationalAccounts::<T>::iter().collect::<Vec<_>>();
		// Describe the expected converted records without mutating deployed storage.
		let lots = old::BondLotById::<T>::iter()
			.map(|(id, lot)| {
				(
					id,
					BondLot::<T> {
						owner: lot.owner,
						program: lot.program,
						bonds: lot.bonds,
						is_flexible: lot.is_flexible,
						locked_frame_terms: None,
						created_frame_id: lot.created_frame_id,
						participated_frames: lot.participated_frames,
						last_frame_earnings_frame_id: lot.last_frame_earnings_frame_id,
						last_frame_earnings: lot.last_frame_earnings,
						cumulative_earnings: lot.cumulative_earnings,
						release_frame_id: lot.release_frame_id,
						release_reason: lot.release_reason,
					},
				)
			})
			.collect::<Vec<_>>();
		let locks = LocksById::<T>::iter().collect::<Vec<_>>();
		let fissions = FissionByOwnerAndId::<T>::iter().collect::<Vec<_>>();
		let vault_bonds = old::BondLotsByVault::<T>::iter()
			.map(|(vault_id, bonds)| {
				let regular_bonds = bonds
					.regular_bond_lots
					.iter()
					.fold(0u32, |total, lot| total.saturating_add(lot.bonds));
				let securitization =
					<T as TreasuryConfig>::TreasuryVaultProvider::get_vault_securitization(
						vault_id,
					)
					.unwrap_or_default();
				let capacity =
					(securitization / MICROGONS_PER_ARGON).min(Bonds::MAX.into()) as Bonds;
				let displaced_flexible_bonds =
					bonds.flexible_bonds.saturating_sub(capacity.saturating_sub(regular_bonds));
				(
					vault_id,
					VaultBondState {
						regular_bonds,
						flexible_bonds: bonds.flexible_bonds,
						displaced_flexible_bonds,
						locked_frame_terms: None,
						reserved_bond_space: bonds.reserved_bond_space,
					},
				)
			})
			.collect::<Vec<_>>();
		let mut positions = BTreeMap::<AccountId, Position<Balance>>::new();
		for (_, account) in &accounts {
			let upstream = OperationalAccountsPallet::<T>::upstream_vault(&account.vault_account)
				.map(|(_, vault_id)| UpstreamPosition { vault_id, ..Default::default() });
			positions.entry(account.vault_account.clone()).or_default().upstream = upstream;
		}
		for (_, lot) in &lots {
			if lot.release_reason.is_none() {
				let position = positions.entry(lot.owner.clone()).or_default();
				match lot.program {
					BondProgram::Vault { vault_id, .. } if lot.is_flexible => {
						ensure!(
							<T as TreasuryConfig>::TreasuryVaultProvider::get_vault_operator(
								vault_id
							)
							.as_ref() == Some(&lot.owner),
							TryRuntimeError::Other(
								"flexible bonds do not belong to their vault operator"
							)
						);
					},
					BondProgram::Vault { .. } =>
						position.quantities.bonds = position
							.quantities
							.bonds
							.checked_add(lot.bonds.into())
							.ok_or(TryRuntimeError::Other("eligible bonds overflow"))?,
					BondProgram::Argonot =>
						position.quantities.stakes = position
							.quantities
							.stakes
							.checked_add(lot.bonds.into())
							.ok_or(TryRuntimeError::Other("stakes overflow"))?,
				}
			}
			if let BondProgram::Vault { vault_id, .. } = lot.program &&
				lot.release_reason.is_none()
			{
				let principal = Balance::from(lot.bonds).saturating_mul(MICROGONS_PER_ARGON);
				let position = positions.entry(lot.owner.clone()).or_default();
				position.bond_principal = position
					.bond_principal
					.checked_add(principal)
					.ok_or(TryRuntimeError::Other("bond principal overflow"))?;
				if let Some(upstream) =
					position.upstream.as_mut().filter(|p| p.vault_id == vault_id)
				{
					upstream.bond_principal = upstream
						.bond_principal
						.checked_add(principal)
						.ok_or(TryRuntimeError::Other("upstream bond principal overflow"))?;
				}
			}
		}
		for (vault_id, bonds) in &vault_bonds {
			if bonds.eligible_flexible_bonds() > 0 {
				let operator =
					<T as TreasuryConfig>::TreasuryVaultProvider::get_vault_operator(*vault_id)
						.ok_or(TryRuntimeError::Other("missing flexible bond operator"))?;
				let position = positions.entry(operator).or_default();
				position.quantities.bonds = position
					.quantities
					.bonds
					.checked_add(bonds.eligible_flexible_bonds().into())
					.ok_or(TryRuntimeError::Other("eligible flexible bonds overflow"))?;
			}
		}
		for (_, lock) in &locks {
			if let Some(upstream) = positions
				.get_mut(&lock.owner_account)
				.and_then(|p| p.upstream.as_mut())
				.filter(|p| p.vault_id == lock.vault_id)
			{
				let bitcoin = lock.upstream_collateral();
				upstream.bitcoin_securitization = upstream
					.bitcoin_securitization
					.checked_add(bitcoin.activated_securitization)
					.ok_or(TryRuntimeError::Other("activated Bitcoin collateral overflow"))?;
				upstream.bitcoin_allocated_securitization = upstream
					.bitcoin_allocated_securitization
					.checked_add(bitcoin.allocated_securitization)
					.ok_or(TryRuntimeError::Other("allocated Bitcoin collateral overflow"))?;
			}
		}
		for (account, _, fission) in &fissions {
			let position = positions.entry(account.clone()).or_default();
			position.quantities.fission_liquidity = position
				.quantities
				.fission_liquidity
				.checked_add(fission.liquidity_promised)
				.ok_or(TryRuntimeError::Other("Fission liquidity overflow"))?;
		}
		// Only the cached source quantities may change in Operational Accounts.
		let expected_accounts = accounts
			.into_iter()
			.map(|(owner, mut account)| {
				account.account_vault_bond_amount = positions
					.get(&account.vault_account)
					.map(|p| p.bond_principal)
					.unwrap_or_default();
				account.account_bitcoin_amount = positions
					.get(&account.vault_account)
					.map(|p| p.quantities.fission_liquidity)
					.unwrap_or_default();
				(owner, account)
			})
			.collect::<Vec<_>>();
		Ok((
			positions,
			T::Hashing::hash_of(&(&lots, &vault_bonds)),
			T::Hashing::hash_of(&locks),
			T::Hashing::hash_of(&fissions),
			T::Hashing::hash_of(&expected_accounts),
			(frame_id, bid_pool_balance, reserves_balance, issuance),
		)
			.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let (positions, lots_hash, locks_hash, fissions_hash, accounts_hash, frame_state) =
			<(
				BTreeMap<AccountId, Position<Balance>>,
				T::Hash,
				T::Hash,
				T::Hash,
				T::Hash,
				(FrameId, Balance, Balance, Balance),
			)>::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("invalid position migration state"))?;
		let (frame_id, bid_pool_balance, reserves_balance, issuance) = frame_state;
		ensure!(
			<T as TreasuryConfig>::MiningFrameTransitionProvider::get_current_frame_id() ==
				frame_id,
			TryRuntimeError::Other("migration rotated the mining frame")
		);
		ensure!(
			<T as TreasuryConfig>::Currency::balance(&T::MiningBidPoolAccount::get()) ==
				bid_pool_balance &&
				<T as TreasuryConfig>::Currency::balance(&T::TreasuryReservesAccount::get()) ==
					reserves_balance &&
				<T as TreasuryConfig>::Currency::total_issuance() == issuance,
			TryRuntimeError::Other("migration distributed or burned frame earnings")
		);
		ensure!(
			StorageVersion::get::<PositionsPallet<T>>() == StorageVersion::new(1),
			TryRuntimeError::Other("position storage version was not advanced")
		);
		ensure!(
			PositionsByAccount::<T>::iter().collect::<BTreeMap<_, _>>() == positions,
			TryRuntimeError::Other("account positions do not match canonical sources")
		);
		let expected_totals = positions.values().try_fold(
			PositionQuantities::<Balance>::default(),
			|mut total, position| -> Result<_, TryRuntimeError> {
				total.bonds = total
					.bonds
					.checked_add(position.quantities.bonds)
					.ok_or(TryRuntimeError::Other("network bonds overflow"))?;
				total.stakes = total
					.stakes
					.checked_add(position.quantities.stakes)
					.ok_or(TryRuntimeError::Other("network stakes overflow"))?;
				total.fission_liquidity = total
					.fission_liquidity
					.checked_add(position.quantities.fission_liquidity)
					.ok_or(TryRuntimeError::Other("network Fission liquidity overflow"))?;
				Ok(total)
			},
		)?;
		ensure!(
			NetworkTotals::<T>::get() == expected_totals,
			TryRuntimeError::Other("network quantities do not match accounts")
		);
		let lots = BondLotById::<T>::iter().collect::<Vec<_>>();
		let vault_bonds = BondLotsByVault::<T>::iter().collect::<Vec<_>>();
		ensure!(
			T::Hashing::hash_of(&(&lots, &vault_bonds)) == lots_hash &&
				T::Hashing::hash_of(&LocksById::<T>::iter().collect::<Vec<_>>()) == locks_hash &&
				T::Hashing::hash_of(&FissionByOwnerAndId::<T>::iter().collect::<Vec<_>>()) ==
					fissions_hash,
			TryRuntimeError::Other("migration changed canonical bonds, locks or Fissions")
		);
		let expected_vault_lots = lots
			.iter()
			.filter(|(_, lot)| matches!(lot.program, BondProgram::Vault { .. }))
			.count() as u32;
		ensure!(
			TotalArgonBondLots::<T>::get() == expected_vault_lots &&
				BondLotIdsByVault::<T>::iter().count() as u32 == expected_vault_lots &&
				lots.iter().all(|(id, lot)| match lot.program {
					BondProgram::Vault { vault_id, .. } =>
						BondLotIdsByVault::<T>::contains_key(vault_id, id),
					BondProgram::Argonot => true,
				}),
			TryRuntimeError::Other("vault bond id index mismatch")
		);
		ensure!(
			T::Hashing::hash_of(&OperationalAccounts::<T>::iter().collect::<Vec<_>>()) ==
				accounts_hash,
			TryRuntimeError::Other("operational accounts changed beyond their quantity caches")
		);
		let capital = CurrentFrameVaultCapital::<T>::get()
			.ok_or(TryRuntimeError::Other("current frame capital was not seeded"))?;
		ensure!(
			capital.frame_id == frame_id && capital.total_active_bonds == expected_totals.bonds,
			TryRuntimeError::Other("current frame inputs do not match migrated quantities")
		);
		for (_, position) in capital.vault_securitization_positions.iter() {
			ensure!(
				position.upstream_participation ==
					TreasuryPallet::<T>::upstream_participation(&position.operator_account_id),
				TryRuntimeError::Other("incorrect frame participation")
			);
		}
		Ok(())
	}
}

// FRAME initializes a newly added pallet's version before this tuple runs. Guard the
// bond conversion and position initialization together with Treasury's existing version.
// Both are part of runtime 160 and skip fresh genesis and subsequent upgrades.
pub type SeedTreasuryStateMigration<T> = VersionedMigration<
	8,
	9,
	SeedTreasuryState<T>,
	TreasuryPallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

/// Deployed Treasury layouts read by the runtime 160 migration.
pub mod old {
	use super::*;
	use crate::{Config, Pallet};

	#[derive(Encode, Decode)]
	pub struct BondLot<T: Config> {
		pub owner: T::AccountId,
		pub program: BondProgram,
		#[codec(compact)]
		pub bonds: Bonds,
		pub is_flexible: bool,
		#[codec(compact)]
		pub created_frame_id: FrameId,
		#[codec(compact)]
		pub participated_frames: u32,
		pub last_frame_earnings_frame_id: Option<FrameId>,
		pub last_frame_earnings: Option<T::Balance>,
		#[codec(compact)]
		pub cumulative_earnings: T::Balance,
		pub release_frame_id: Option<FrameId>,
		pub release_reason: Option<BondReleaseReason>,
	}

	#[storage_alias]
	pub type BondLotById<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, BondLotId, BondLot<T>, OptionQuery>;

	#[derive(Encode, Decode, DefaultNoBound)]
	pub struct VaultBondState {
		pub regular_bond_lots: BoundedVec<BondLotSummary, ConstU32<100>>,
		#[codec(compact)]
		pub flexible_bonds: Bonds,
		#[codec(compact)]
		pub reserved_bond_space: Bonds,
	}

	#[storage_alias]
	pub type BondLotsByVault<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, VaultId, VaultBondState, ValueQuery>;
}
