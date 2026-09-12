#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;
use pallet_prelude::*;
pub use weights::*;

pub mod migrations;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use argon_primitives::{
		bitcoin::{
			BitcoinBlock, BitcoinBlockHash, BitcoinCosignScriptPubkey, BitcoinHeight,
			BitcoinLockId, BitcoinSyncStatus, Satoshis, UtxoAddress, UtxoRef, UtxoValue,
		},
		inherents::{
			BitcoinInherentData, BitcoinInherentError, BitcoinUtxoFunding, BitcoinUtxoSyncV2,
		},
		BitcoinUtxoEvents, BitcoinUtxoTracker,
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		type WeightInfo: WeightInfo;

		type EventHandler: BitcoinUtxoEvents<Self::AccountId>;

		/// Maximum number of outputs tracked for a single Lock address.
		#[pallet::constant]
		type MaxUtxosPerLock: Get<u32>;

		/// Minimum output size tracked and reported for a watched Lock address.
		type MinimumSatoshisPerUtxo: Get<Satoshis>;
	}

	/// The Lock ID identified by each watched script pubkey.
	#[pallet::storage]
	pub type LockIdByScriptPubkey<T: Config> =
		StorageMap<_, Blake2_128Concat, BitcoinCosignScriptPubkey, BitcoinLockId, OptionQuery>;

	/// Watched Lock addresses and the scan height needed to observe them.
	#[pallet::storage]
	pub type UtxoAddressByLockId<T: Config> =
		StorageMap<_, Twox64Concat, BitcoinLockId, UtxoAddress, OptionQuery>;

	/// Every output observed at a watched Lock address, retained until explicitly removed.
	#[pallet::storage]
	pub type UtxoRefsByLockId<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinLockId,
		BoundedBTreeSet<UtxoRef, T::MaxUtxosPerLock>,
		ValueQuery,
	>;

	/// The genesis set bitcoin network that this chain is tied to
	#[pallet::storage]
	pub type BitcoinNetwork<T: Config> =
		StorageValue<_, argon_primitives::bitcoin::BitcoinNetwork, ValueQuery>;

	/// An oracle-provided confirmed bitcoin block (eg, 6 blocks back)
	#[pallet::storage]
	pub type ConfirmedBitcoinBlockTip<T: Config> = StorageValue<_, BitcoinBlock, OptionQuery>;

	#[pallet::storage]
	pub type PreviousBitcoinBlockTip<T: Config> = StorageValue<_, BitcoinBlock, OptionQuery>;

	/// Stores if parent block had a confirmed bitcoin block
	#[pallet::storage]
	pub type TempParentHasSyncState<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// The last synched bitcoin block
	#[pallet::storage]
	pub type SynchedBitcoinBlock<T: Config> = StorageValue<_, BitcoinBlock, OptionQuery>;

	/// Bitcoin Oracle Operator Account
	#[pallet::storage]
	pub type OracleOperatorAccount<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Check if the inherent was included
	#[pallet::storage]
	pub type InherentIncluded<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		UtxoDetected {
			lock_id: BitcoinLockId,
			utxo_ref: UtxoRef,
			satoshis_received: Satoshis,
			bitcoin_height: BitcoinHeight,
		},
		UtxoSpent {
			lock_id: BitcoinLockId,
			utxo_ref: UtxoRef,
			block_height: BitcoinHeight,
		},
		UtxoUnwatched {
			lock_id: BitcoinLockId,
		},

		UtxoSpentError {
			lock_id: BitcoinLockId,
			error: DispatchError,
		},
		UtxoDetectedError {
			lock_id: BitcoinLockId,
			error: DispatchError,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Only an Oracle Operator can perform this action
		NoPermissions,
		/// No Oracle-provided bitcoin block has been provided to the network
		NoBitcoinConfirmedBlock,
		/// Insufficient bitcoin amount
		InsufficientBitcoinAmount,
		/// No prices are available to mint bitcoins
		NoBitcoinPricesAvailable,
		/// ScriptPubKey is already being waited for
		ScriptPubkeyConflict,
		/// Watched Lock or attached UTXO not found.
		UtxoNotLocked,
		/// Redemptions not currently available
		RedemptionsUnavailable,
		/// Invalid bitcoin sync height attempted
		InvalidBitcoinSyncHeight,
		/// Bitcoin height not confirmed yet
		BitcoinHeightNotConfirmed,
		/// Too many UTXOs are being watched
		MaxUtxosExceeded,
		/// Locking script has errors
		InvalidBitcoinScript,
		/// Duplicated BitcoinLockId. Already in use
		DuplicateLockId,
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub tip_oracle_operator: Option<T::AccountId>,
		pub network: argon_primitives::bitcoin::BitcoinNetwork,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if let Some(operator) = &self.tip_oracle_operator {
				OracleOperatorAccount::<T>::put(operator);
			}
			BitcoinNetwork::<T>::put(self.network);
		}
	}
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let confirmed_tip = ConfirmedBitcoinBlockTip::<T>::get();
			TempParentHasSyncState::<T>::put(confirmed_tip.is_some());
			PreviousBitcoinBlockTip::<T>::set(confirmed_tip);
			T::WeightInfo::on_initialize()
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			// If we have started synching bitcoin blocks, inherent must be included
			if TempParentHasSyncState::<T>::get() {
				// According to parity, the only way to ensure that a mandatory inherent is included
				// is by checking on block finalization that the inherent set a particular storage
				// item: https://github.com/paritytech/polkadot-sdk/issues/2841#issuecomment-1876040854
				assert!(
					InherentIncluded::<T>::take(),
					"Block invalid, missing inherent `bitcoin_utxos::sync`"
				);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submitted when a bitcoin UTXO has been moved or confirmed.
		#[pallet::call_index(0)]
		#[pallet::weight((
				T::WeightInfo::sync(
					utxo_sync.spent.len() as u32,
					utxo_sync.funded.len() as u32,
				),
				DispatchClass::Mandatory
			))]
		pub fn sync(origin: OriginFor<T>, utxo_sync: BitcoinUtxoSyncV2) -> DispatchResult {
			ensure_none(origin)?;
			log::info!(
				"Bitcoin UTXO sync submitted (spent: {:?}, funded {})",
				utxo_sync.spent.len(),
				utxo_sync.funded.len(),
			);

			ensure!(!InherentIncluded::<T>::get(), "Inherent already included");
			InherentIncluded::<T>::put(true);

			let BitcoinUtxoSyncV2 { sync_to_block, funded, spent } = utxo_sync;
			let current_confirmed =
				ConfirmedBitcoinBlockTip::<T>::get().ok_or(Error::<T>::NoBitcoinConfirmedBlock)?;
			ensure!(
				sync_to_block.block_height <= current_confirmed.block_height,
				Error::<T>::InvalidBitcoinSyncHeight
			);
			if let Some(existing_sync) = SynchedBitcoinBlock::<T>::get() {
				ensure!(
					sync_to_block.block_height >= existing_sync.block_height,
					Error::<T>::InvalidBitcoinSyncHeight
				);
			}

			for BitcoinUtxoFunding { lock_id, utxo_ref, satoshis, bitcoin_height, .. } in funded {
				let result = with_storage_layer(|| {
					Self::utxo_detected(lock_id, utxo_ref, satoshis, bitcoin_height)
				});
				if let Err(error) = result {
					log::warn!("Failed to process UTXO {lock_id}: {error:?}");
					Self::deposit_event(Event::UtxoDetectedError { lock_id, error });
				}
			}

			for spend in spent {
				let result = with_storage_layer(|| {
					Self::utxo_spent(spend.lock_id, spend.utxo_ref, spend.bitcoin_height)
				});
				if let Err(error) = result {
					log::warn!("Failed to mark UTXO {} as spent: {error:?}", spend.lock_id);
					Self::deposit_event(Event::UtxoSpentError { lock_id: spend.lock_id, error });
				}
			}

			SynchedBitcoinBlock::<T>::set(Some(sync_to_block));
			Ok(())
		}

		/// Sets the most recent confirmed bitcoin block height (only executable by the Oracle
		/// Operator account)
		///
		/// # Arguments
		/// * `bitcoin_height` - the latest bitcoin block height to be confirmed
		#[pallet::call_index(1)]
		#[pallet::weight((T::WeightInfo::set_confirmed_block(), DispatchClass::Operational))]
		#[pallet::feeless_if(|origin: &OriginFor<T>, _height: &BitcoinHeight, _hash: &BitcoinBlockHash, | -> bool {
			let Ok(who) = ensure_signed(origin.clone()) else {
				return false;
			};
			Some(who) == OracleOperatorAccount::<T>::get()
		})]
		pub fn set_confirmed_block(
			origin: OriginFor<T>,
			bitcoin_height: BitcoinHeight,
			bitcoin_block_hash: BitcoinBlockHash,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Some(who) == OracleOperatorAccount::<T>::get(), Error::<T>::NoPermissions);
			if let Some(current) = ConfirmedBitcoinBlockTip::<T>::get() &&
				bitcoin_height < current.block_height
			{
				return Ok(());
			}
			ConfirmedBitcoinBlockTip::<T>::put(BitcoinBlock {
				block_height: bitcoin_height,
				block_hash: bitcoin_block_hash,
			});
			Ok(())
		}

		/// Sets the oracle operator account id (only executable by the Root account)
		///
		/// # Arguments
		/// * `account_id` - the account id of the operator
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_operator())]
		pub fn set_operator(origin: OriginFor<T>, account_id: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			OracleOperatorAccount::<T>::put(account_id.clone());
			Ok(())
		}
	}

	impl<T: Config> BitcoinUtxoTracker for Pallet<T> {
		fn get_synched_height() -> BitcoinHeight {
			SynchedBitcoinBlock::<T>::get()
				.map(|block| block.block_height)
				.unwrap_or_default()
		}

		fn watch_for_utxo(
			lock_id: BitcoinLockId,
			script_pubkey: BitcoinCosignScriptPubkey,
		) -> Result<(), DispatchError> {
			ensure!(!UtxoAddressByLockId::<T>::contains_key(lock_id), Error::<T>::DuplicateLockId);
			ensure!(
				!LockIdByScriptPubkey::<T>::contains_key(script_pubkey),
				Error::<T>::ScriptPubkeyConflict
			);
			let address = UtxoAddress {
				lock_id,
				script_pubkey,
				submitted_at_height: ConfirmedBitcoinBlockTip::<T>::get()
					.map(|block| block.block_height)
					.unwrap_or_default(),
			};
			LockIdByScriptPubkey::<T>::insert(script_pubkey, lock_id);
			UtxoAddressByLockId::<T>::insert(lock_id, address);
			Ok(())
		}

		fn unwatch_utxo(lock_id: BitcoinLockId, utxo_ref: &UtxoRef) {
			UtxoRefsByLockId::<T>::mutate_exists(lock_id, |maybe_refs| {
				if let Some(refs) = maybe_refs.as_mut() {
					refs.remove(utxo_ref);
					if refs.is_empty() {
						*maybe_refs = None;
					}
				}
			});
		}

		fn unwatch(lock_id: BitcoinLockId) {
			if let Some(address) = UtxoAddressByLockId::<T>::get(lock_id) {
				LockIdByScriptPubkey::<T>::remove(address.script_pubkey);
			}
			UtxoAddressByLockId::<T>::remove(lock_id);
			UtxoRefsByLockId::<T>::remove(lock_id);
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn has_new_bitcoin_tip() -> bool {
			let Some(current) = ConfirmedBitcoinBlockTip::<T>::get() else {
				return false;
			};
			let Some(previous) = PreviousBitcoinBlockTip::<T>::get() else {
				return true;
			};
			previous.block_hash != current.block_hash
		}

		pub fn get_sync_status() -> Option<BitcoinSyncStatus> {
			let confirmed_block = ConfirmedBitcoinBlockTip::<T>::get()?;
			let synched_block = SynchedBitcoinBlock::<T>::get();
			// We have full visibility into everything up to the synched point
			let oldest_allowed_block_height = if let Some(ref x) = synched_block {
				x.block_height
			} else {
				let mut oldest = confirmed_block.block_height;
				for entry in UtxoAddressByLockId::<T>::iter_values() {
					if entry.submitted_at_height < oldest {
						oldest = entry.submitted_at_height;
					}
				}
				oldest
			};
			Some(BitcoinSyncStatus { confirmed_block, synched_block, oldest_allowed_block_height })
		}

		pub fn active_utxo_addresses() -> Vec<(Option<UtxoRef>, UtxoAddress)> {
			let mut utxos = vec![];
			for (lock_id, watch) in UtxoAddressByLockId::<T>::iter() {
				let refs = UtxoRefsByLockId::<T>::get(lock_id);
				if refs.is_empty() {
					utxos.push((None, watch));
				} else {
					utxos.extend(refs.into_iter().map(|utxo_ref| (Some(utxo_ref), watch.clone())));
				}
			}
			utxos
		}

		/// Legacy API projection retained for nodes that have not adopted the address-only API.
		pub fn active_utxos() -> Vec<(Option<UtxoRef>, UtxoValue)> {
			Self::active_utxo_addresses()
				.into_iter()
				.map(|(utxo_ref, address)| {
					(
						utxo_ref,
						UtxoValue {
							lock_id: address.lock_id,
							script_pubkey: address.script_pubkey,
							satoshis: 0,
							submitted_at_height: address.submitted_at_height,
							watch_for_spent_until_height: BitcoinHeight::MAX,
						},
					)
				})
				.collect()
		}

		pub fn utxo_detected(
			lock_id: BitcoinLockId,
			utxo_ref: UtxoRef,
			satoshis: Satoshis,
			bitcoin_height: BitcoinHeight,
		) -> DispatchResult {
			if satoshis < T::MinimumSatoshisPerUtxo::get() {
				tracing::info!(lock_id = ?lock_id, satoshis = ?satoshis,
					"UTXO below minimum tracking threshold");
				return Ok(())
			}

			let Some(address) = UtxoAddressByLockId::<T>::get(lock_id) else { return Ok(()) };
			if LockIdByScriptPubkey::<T>::get(address.script_pubkey) != Some(lock_id) {
				tracing::info!(lock_id = ?lock_id, "UTXO address is not being watched");
				return Ok(())
			}

			if UtxoRefsByLockId::<T>::get(lock_id).contains(&utxo_ref) {
				tracing::info!(lock_id = ?lock_id, satoshis = ?satoshis, utxo_ref = ?utxo_ref, bitcoin_height,
					"UTXO duplicate received");
				return Ok(());
			}

			// Let the consumer classify the output before bounded tracker retention. Bitcoin Locks
			// records outputs beyond its Lock limit as orphans.
			T::EventHandler::utxo_detected(lock_id, utxo_ref.clone(), satoshis, bitcoin_height)?;
			let is_tracked = UtxoRefsByLockId::<T>::try_mutate(lock_id, |refs| {
				refs.try_insert(utxo_ref.clone()).map_err(|_| ())
			});
			if is_tracked.is_err() {
				tracing::info!(lock_id = ?lock_id, satoshis = ?satoshis, utxo_ref = ?utxo_ref, bitcoin_height,
					"UTXO handled but not retained because the lock tracking limit was reached");
			}
			Self::deposit_event(Event::UtxoDetected {
				lock_id,
				utxo_ref,
				satoshis_received: satoshis,
				bitcoin_height,
			});
			Ok(())
		}

		pub fn utxo_spent(
			lock_id: BitcoinLockId,
			utxo_ref: Option<UtxoRef>,
			block_height: BitcoinHeight,
		) -> DispatchResult {
			let refs = match utxo_ref {
				Some(utxo_ref) => alloc::vec![utxo_ref],
				None => UtxoRefsByLockId::<T>::get(lock_id).into_iter().collect(),
			};
			for utxo_ref in refs {
				if !UtxoRefsByLockId::<T>::get(lock_id).contains(&utxo_ref) {
					continue
				}
				T::EventHandler::spent(lock_id, utxo_ref.clone(), block_height)?;

				Self::deposit_event(Event::UtxoSpent { lock_id, utxo_ref, block_height });
			}
			Ok(())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = BitcoinInherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier =
			argon_primitives::inherents::BITCOIN_INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call>
		where
			InherentData: BitcoinInherentData,
		{
			let utxo_sync = Self::bitcoin_sync_from_inherent(data)
				.expect("Could not decode bitcoin inherent data");
			utxo_sync.map(|utxo_sync| Call::sync { utxo_sync })
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			match call {
				Call::sync { utxo_sync } => {
					let Some(data_sync) = Self::bitcoin_sync_from_inherent(data)
						.expect("Could not decode bitcoin inherent data")
					else {
						return Err(BitcoinInherentError::InvalidInherentData);
					};
					if data_sync != *utxo_sync {
						return Err(BitcoinInherentError::InvalidInherentData);
					}
				},
				_ => return Ok(()),
			}

			Ok(())
		}

		fn is_inherent_required(_: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			Ok(None)
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::sync { .. })
		}
	}

	impl<T: Config> Pallet<T> {
		fn bitcoin_sync_from_inherent(
			data: &InherentData,
		) -> Result<Option<BitcoinUtxoSyncV2>, sp_inherents::Error> {
			if let Some(sync) = data.bitcoin_sync()? {
				return Ok(Some(sync.into()));
			}
			data.bitcoin_sync_v2()
		}
	}

	impl<T: Config> Get<argon_primitives::bitcoin::BitcoinNetwork> for Pallet<T> {
		fn get() -> argon_primitives::bitcoin::BitcoinNetwork {
			BitcoinNetwork::<T>::get()
		}
	}

	impl<T: Config> Get<BitcoinHeight> for Pallet<T> {
		fn get() -> BitcoinHeight {
			ConfirmedBitcoinBlockTip::<T>::get().map(|a| a.block_height).unwrap_or_default()
		}
	}

	impl<T: Config> Get<(BitcoinHeight, BitcoinHeight)> for Pallet<T> {
		fn get() -> (BitcoinHeight, BitcoinHeight) {
			let current =
				ConfirmedBitcoinBlockTip::<T>::get().map(|a| a.block_height).unwrap_or_default();
			let previous =
				PreviousBitcoinBlockTip::<T>::get().map(|a| a.block_height).unwrap_or(current);
			(previous, current)
		}
	}
}
