use crate::{
	Config, LockReleaseRequestsById, LockedBitcoin as LockedBitcoinV12, OrphanedUtxo,
	OrphanedUtxoCosignRequest, OrphanedUtxosByAccount, Pallet,
};
use argon_primitives::{
	bitcoin::{BitcoinLockId, BitcoinScriptPubkey, Satoshis},
	vault::{BitcoinSecuritizationBasis, BitcoinVaultProvider, CosignRequest, CosignerGuarantee},
	VaultId,
};
use frame_support::{migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

pub fn is_migration_pending<T: Config>() -> bool {
	StorageVersion::get::<Pallet<T>>() == 11
}

pub fn is_migration_complete<T: Config>() -> bool {
	StorageVersion::get::<Pallet<T>>() == 12
}

/// Record completion after the runtime has converted the Vault, Lock and Fission storage.
pub fn complete_migration<T: Config>() {
	StorageVersion::new(12).put::<Pallet<T>>();
}

/// Rebuild the per-vault signing list from accepted Bitcoin requests.
pub struct InitializePendingCosigns<T>(PhantomData<T>);
impl<T: Config> UncheckedOnRuntimeUpgrade for InitializePendingCosigns<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut requests = 0u64;
		let mut orphans = 0u64;
		for request in LockReleaseRequestsById::<T>::iter_values() {
			requests.saturating_accrue(1);
			T::VaultProvider::update_pending_cosign(
				request.vault_id,
				request
					.cosign_due_frame
					.saturating_sub(T::LockReleaseCosignDeadlineFrames::get())
					.saturating_add(1),
				CosignRequest::Lock(request.lock_id),
				false,
			);
		}
		OrphanedUtxosByAccount::<T>::translate::<OrphanedUtxoV12<T>, _>(|owner, utxo_ref, old| {
			orphans.saturating_accrue(1);
			let cosign_request = old.cosign_request.and_then(|request| {
				if Pallet::<T>::validate_orphan_release(
					old.satoshis,
					&request.to_script_pubkey,
					request.bitcoin_network_fee,
				)
				.is_err()
				{
					return None
				}
				T::VaultProvider::update_pending_cosign(
					old.vault_id,
					0,
					CosignRequest::Orphan { owner, utxo_ref },
					false,
				);
				Some(OrphanedUtxoCosignRequest {
					bitcoin_network_fee: request.bitcoin_network_fee,
					to_script_pubkey: request.to_script_pubkey,
					created_at_argon_block_number: request.created_at_argon_block_number,
					collect_due_frame: 0,
				})
			});
			Some(OrphanedUtxo {
				lock_id: old.lock_id,
				vault_id: old.vault_id,
				satoshis: old.satoshis,
				recorded_argon_block_number: old.recorded_argon_block_number,
				cosign_request,
			})
		});
		T::DbWeight::get().reads_writes(
			requests.saturating_add(orphans).saturating_add(2),
			requests.saturating_add(orphans.saturating_mul(2)),
		)
	}
}

pub type PendingCosignsMigration<T> = VersionedMigration<
	12,
	13,
	InitializePendingCosigns<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[derive(Encode, Decode)]
pub(crate) struct OrphanedUtxoV12<T: Config> {
	#[codec(compact)]
	pub lock_id: BitcoinLockId,
	#[codec(compact)]
	pub vault_id: VaultId,
	#[codec(compact)]
	pub satoshis: Satoshis,
	#[codec(compact)]
	pub recorded_argon_block_number: BlockNumberFor<T>,
	pub cosign_request: Option<OrphanedUtxoCosignRequestV12<BlockNumberFor<T>>>,
}

#[derive(Encode, Decode)]
pub(crate) struct OrphanedUtxoCosignRequestV12<BlockNumber> {
	pub bitcoin_network_fee: Satoshis,
	pub to_script_pubkey: BitcoinScriptPubkey,
	pub created_at_argon_block_number: BlockNumber,
}

/// Frozen runtime-160 Lock storage. Field order and compact encodings must remain frozen.
pub mod v11 {
	use super::*;
	use argon_primitives::{
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinLockId, CompressedBitcoinPubkey,
			Satoshis, UtxoRef, XPubChildNumber, XPubFingerprint,
		},
		tick::Tick,
		VaultId,
	};
	use codec::{Codec, Decode, Encode};
	use frame_support::storage_alias;
	use sp_runtime::BoundedBTreeMap;

	/// Keep the historical SCALE layout independent of the new basis history tick.
	#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq, Eq)]
	pub struct BitcoinSecuritizationBasis<Balance: Codec> {
		#[codec(compact)]
		pub satoshis: Satoshis,
		#[codec(compact)]
		pub microgons_at_target_per_btc: Balance,
	}

	#[derive(Encode, Decode, CloneNoBound)]
	pub struct LockedBitcoin<T: Config> {
		#[codec(compact)]
		pub vault_id: VaultId,
		/// Bitcoin amount and valuation used to price this Lock's securitization.
		pub securitization_basis: BitcoinSecuritizationBasis<T::Balance>,
		/// Microgon coverage purchased after applying the redemption curve.
		#[codec(compact)]
		pub securitization_coverage_microgons: T::Balance,
		/// Tick of the price-history entry used for the current securitization.
		#[codec(compact)]
		pub securitization_tick: Tick,
		/// Satoshis attached to this Lock by confirmed funding outputs. Zero means unfunded.
		#[codec(compact)]
		pub funded_satoshis: Satoshis,
		/// Confirmed Bitcoin outputs assigned to this Lock and their individual amounts.
		pub funding_utxos: BoundedBTreeMap<UtxoRef, Satoshis, T::MaxUtxosPerLock>,
		/// Funded satoshis currently split into active Fissions.
		#[codec(compact)]
		pub fissioned_satoshis: Satoshis,
		/// The owner account
		pub owner_account: T::AccountId,
		/// The guaranteed securitization ratio for this lock
		pub securitization_ratio: FixedU128,
		/// Sum of all lock fees (initial plus any resecuritization)
		#[codec(compact)]
		pub security_fees: T::Balance,
		/// Fees covered by the vault for this lock
		#[codec(compact)]
		pub coupon_paid_fees: T::Balance,
		/// The vault pubkey used in the cosign script to lock (and unlock) the bitcoin
		pub vault_pubkey: CompressedBitcoinPubkey,
		/// The vault pubkey used to claim the bitcoin after the lock expiration
		pub vault_claim_pubkey: CompressedBitcoinPubkey,
		/// The vault xpub sources. First is the cosign number, second is the claim number
		pub vault_xpub_sources: (XPubFingerprint, XPubChildNumber, XPubChildNumber),
		/// The bitcoin pubkey provided by the owner of the bitcoin lock that will be needed to
		/// spend the bitcoin (owner side of cosign)
		pub owner_pubkey: CompressedBitcoinPubkey,
		/// The height where the vault has exclusive rights to claim the bitcoin
		#[codec(compact)]
		pub vault_claim_height: BitcoinHeight,
		/// The height where either owner or vault can claim the bitcoin
		#[codec(compact)]
		pub open_claim_height: BitcoinHeight,
		/// The bitcoin height when this lock was created
		#[codec(compact)]
		pub created_at_height: BitcoinHeight,
		/// The Bitcoin height through which unused securitization remains reserved.
		#[codec(compact)]
		pub securitization_hold_expiration_bitcoin_height: BitcoinHeight,
		/// The script pubkey where funds are sent to fund this bitcoin lock
		pub utxo_script_pubkey: BitcoinCosignScriptPubkey,
		/// Whether this operator-owned lock may be displaced by outside vault capital.
		pub is_flexible: bool,
		/// Funds used by this bitcoin that will need to be held for extended periods when released
		/// back to the vault
		pub fund_hold_extensions: BoundedBTreeMap<BitcoinHeight, T::Balance, ConstU32<366>>,
		/// The argon block when this lock was created
		#[codec(compact)]
		pub created_at_argon_block: BlockNumberFor<T>,
	}

	impl<T: Config> LockedBitcoin<T> {
		/// Runtime 160 rounded the funding fraction before multiplying by securitization.
		pub fn securitization_for_satoshis(&self, satoshis: Satoshis) -> T::Balance {
			let capacity = self.securitization_basis.satoshis;
			FixedU128::from_rational(satoshis.min(capacity) as u128, capacity.max(1) as u128)
				.saturating_mul_int(self.securitization_coverage_microgons)
		}
	}

	#[storage_alias]
	pub type LocksById<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, BitcoinLockId, LockedBitcoin<T>, OptionQuery>;
}

impl<T: Config> From<v11::LockedBitcoin<T>> for LockedBitcoinV12<T> {
	fn from(old: v11::LockedBitcoin<T>) -> Self {
		LockedBitcoinV12 {
			vault_id: old.vault_id,
			cosigner_guarantee: CosignerGuarantee {
				basis: BitcoinSecuritizationBasis {
					satoshis: old.securitization_basis.satoshis,
					microgons_at_target_per_btc: old
						.securitization_basis
						.microgons_at_target_per_btc,
					tick: old.securitization_tick,
				},
				guaranteed_microgons: old.securitization_coverage_microgons,
				fund_hold_extensions: old.fund_hold_extensions,
			},
			securitization_hold_expiration_bitcoin_height: None,
			funded_satoshis: old.funded_satoshis,
			funding_utxos: old.funding_utxos,
			fissioned_satoshis: old.fissioned_satoshis,
			owner_account: old.owner_account,
			security_fees: old.security_fees,
			coupon_paid_fees: old.coupon_paid_fees,
			vault_pubkey: old.vault_pubkey,
			vault_claim_pubkey: old.vault_claim_pubkey,
			vault_xpub_sources: old.vault_xpub_sources,
			owner_pubkey: old.owner_pubkey,
			vault_claim_height: old.vault_claim_height,
			open_claim_height: old.open_claim_height,
			created_at_height: old.created_at_height,
			utxo_script_pubkey: old.utxo_script_pubkey,
			is_flexible: old.is_flexible,
			created_at_argon_block: old.created_at_argon_block,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use argon_primitives::bitcoin::{H256Le, UtxoRef};
	use frame_support::traits::OnRuntimeUpgrade;

	#[test]
	fn migration_preserves_valid_orphan_requests_and_allows_invalid_ones_to_be_corrected() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			let mut script = vec![0; 34];
			script[0] = 0x51;
			script[1] = 0x20;
			let script = BitcoinScriptPubkey::try_from(script).unwrap();
			for (output_index, fee) in [(0, 1_000), (1, 10_001)] {
				let utxo_ref = UtxoRef { txid: H256Le([1; 32]), output_index };
				frame_support::storage::unhashed::put(
					&OrphanedUtxosByAccount::<Test>::hashed_key_for(2, &utxo_ref),
					&OrphanedUtxoV12::<Test> {
						lock_id: 1,
						vault_id: 1,
						satoshis: 10_000,
						recorded_argon_block_number: 5,
						cosign_request: Some(OrphanedUtxoCosignRequestV12 {
							bitcoin_network_fee: fee,
							to_script_pubkey: script.clone(),
							created_at_argon_block_number: 7,
						}),
					},
				);
			}
			StorageVersion::new(12).put::<BitcoinLocks>();
			PendingCosignsMigration::<Test>::on_runtime_upgrade();
			PendingCosignsMigration::<Test>::on_runtime_upgrade();
			assert_eq!(StorageVersion::get::<BitcoinLocks>(), StorageVersion::new(13));
			let valid_ref = UtxoRef { txid: H256Le([1; 32]), output_index: 0 };
			let request = OrphanedUtxosByAccount::<Test>::get(2, valid_ref)
				.unwrap()
				.cosign_request
				.unwrap();
			assert_eq!(request.collect_due_frame, 0);
			assert_eq!(request.created_at_argon_block_number, 7);
			assert_eq!(VaultViewOfOrphanedUtxoCosigns::get()[&1][&2], 1);
			let invalid_ref = UtxoRef { txid: H256Le([1; 32]), output_index: 1 };
			assert!(OrphanedUtxosByAccount::<Test>::get(2, &invalid_ref)
				.unwrap()
				.cosign_request
				.is_none());
			assert_ok!(BitcoinLocks::request_orphaned_utxo_release(
				RuntimeOrigin::signed(2),
				invalid_ref,
				script,
				1_000
			));
			assert_eq!(VaultViewOfOrphanedUtxoCosigns::get()[&1][&2], 2);
		});
	}
}
