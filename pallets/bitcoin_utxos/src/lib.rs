#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
const LOG_TARGET: &str = "runtime::bitcoin_utxos";

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use alloc::{vec, vec::Vec};
	use frame_support::{pallet_prelude::*, storage::with_storage_layer};
	use frame_system::pallet_prelude::*;
	use log::{info, warn};

	use argon_primitives::{
		bitcoin::{
			BitcoinBlock, BitcoinBlockHash, BitcoinCosignScriptPubkey, BitcoinHeight,
			BitcoinRejectedReason, BitcoinSyncStatus, Satoshis, UtxoId, UtxoRef, UtxoValue,
		},
		inherents::{BitcoinInherentData, BitcoinInherentError, BitcoinUtxoSync},
		BitcoinUtxoEvents, BitcoinUtxoTracker,
	};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;

		type EventHandler: BitcoinUtxoEvents;

		/// The maximum number of UTXOs that can be tracked in a block and/or expiring at same block
		#[pallet::constant]
		type MaxPendingConfirmationUtxos: Get<u32>;

		/// Maximum bitcoin blocks to watch a Utxo for confirmation before canceling
		#[pallet::constant]
		type MaxPendingConfirmationBlocks: Get<BitcoinHeight>;

		/// The number of blocks previous to the tip that a bitcoin UTXO will be allowed to be
		/// locked
		#[pallet::constant]
		type MaxUtxoBirthBlocksOld: Get<BitcoinHeight>;
	}

	#[pallet::storage]
	pub(super) type NextUtxoId<T: Config> = StorageValue<_, UtxoId, OptionQuery>;

	/// Locked Bitcoin UTXOs that have had ownership confirmed. If a Bitcoin UTXO is moved before
	/// the expiration block, the bond is burned and the UTXO is unlocked.
	#[pallet::storage]
	pub(super) type LockedUtxos<T: Config> =
		StorageMap<_, Blake2_128Concat, UtxoRef, UtxoValue, OptionQuery>;

	#[pallet::storage]
	pub(super) type UtxoIdToRef<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, UtxoRef, OptionQuery>;

	/// Bitcoin UTXOs that have been submitted for ownership confirmation
	#[pallet::storage]
	pub(super) type UtxosPendingConfirmation<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<UtxoId, UtxoValue, T::MaxPendingConfirmationUtxos>,
		ValueQuery,
	>;

	/// The genesis set bitcoin network that this chain is tied to
	#[pallet::storage]
	pub(super) type BitcoinNetwork<T: Config> =
		StorageValue<_, argon_primitives::bitcoin::BitcoinNetwork, ValueQuery>;

	/// An oracle-provided confirmed bitcoin block (eg, 6 blocks back)
	#[pallet::storage]
	pub(super) type ConfirmedBitcoinBlockTip<T: Config> =
		StorageValue<_, BitcoinBlock, OptionQuery>;

	/// The last synched bitcoin block
	#[pallet::storage]
	pub(super) type SynchedBitcoinBlock<T: Config> = StorageValue<_, BitcoinBlock, OptionQuery>;

	/// Bitcoin Oracle Operator Account
	#[pallet::storage]
	pub(super) type OracleOperatorAccount<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Expiration date as a day since unix timestamp mapped to Bitcoin UTXOs
	#[pallet::storage]
	pub(super) type LockedUtxoExpirationsByBlock<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinHeight,
		BoundedVec<UtxoRef, T::MaxPendingConfirmationUtxos>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		UtxoVerified { utxo_id: UtxoId },
		UtxoRejected { utxo_id: UtxoId, rejected_reason: BitcoinRejectedReason },
		UtxoSpent { utxo_id: UtxoId, block_height: BitcoinHeight },
		UtxoUnwatched { utxo_id: UtxoId },

		UtxoSpentError { utxo_id: UtxoId, error: DispatchError },
		UtxoVerifiedError { utxo_id: UtxoId, error: DispatchError },
		UtxoRejectedError { utxo_id: UtxoId, error: DispatchError },
		UtxoExpiredError { utxo_ref: UtxoRef, error: DispatchError },
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
		/// Locked Utxo Not Found
		UtxoNotLocked,
		/// Redemptions not currently available
		RedemptionsUnavailable,
		/// Invalid bitcoin sync height attempted
		InvalidBitcoinSyncHeight,
		/// Bitcoin height not confirmed yet
		BitcoinHeightNotConfirmed,
		/// Too many UTXOs are being tracked
		MaxUtxosExceeded,
		/// Locking script has errors
		InvalidBitcoinScript,
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
				<OracleOperatorAccount<T>>::put(operator);
			}
			<BitcoinNetwork<T>>::put(self.network.clone());
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submitted when a bitcoin UTXO has been moved or confirmed
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn sync(origin: OriginFor<T>, utxo_sync: BitcoinUtxoSync) -> DispatchResult {
			ensure_none(origin)?;
			info!(
				target: LOG_TARGET,
				"Bitcoin UTXO sync submitted (spent: {:?}, confirmed {}, rejected {})", utxo_sync.spent.len(), utxo_sync.verified.len(), utxo_sync.invalid.len()
			);

			let current_confirmed =
				ConfirmedBitcoinBlockTip::<T>::get().ok_or(Error::<T>::NoBitcoinConfirmedBlock)?;
			ensure!(
				utxo_sync.sync_to_block.block_height <= current_confirmed.block_height,
				Error::<T>::InvalidBitcoinSyncHeight
			);
			if let Some(existing_sync) = SynchedBitcoinBlock::<T>::get() {
				ensure!(
					utxo_sync.sync_to_block.block_height >= existing_sync.block_height,
					Error::<T>::InvalidBitcoinSyncHeight
				);
			}

			// watch for spent first, so we don't verify and then burn
			for (utxo_id, block_height) in utxo_sync.spent.into_iter() {
				let err = with_storage_layer(|| Self::utxo_spent(utxo_id, block_height));
				if let Err(e) = err {
					warn!(target: LOG_TARGET, "Failed to mark UTXO {} as spent: {:?}", utxo_id, e);
					Self::deposit_event(Event::UtxoSpentError { utxo_id, error: e });
				}
			}

			for (utxo_id, utxo_ref) in utxo_sync.verified.into_iter() {
				let res = with_storage_layer(|| Self::utxo_verified(utxo_id, utxo_ref));
				if let Err(e) = res {
					warn!(target: LOG_TARGET, "Failed to verify UTXO {}: {:?}", utxo_id, e);
					Self::deposit_event(Event::UtxoVerifiedError { utxo_id, error: e });
				}
			}

			for (utxo_id, rejected_reason) in utxo_sync.invalid.into_iter() {
				let res = with_storage_layer(|| Self::utxo_rejected(utxo_id, rejected_reason));
				if let Err(e) = res {
					warn!(target: LOG_TARGET, "Failed to reject UTXO {}: {:?}", utxo_id, e);
					Self::deposit_event(Event::UtxoRejectedError { utxo_id, error: e });
				}
			}

			let oldest_allowed_bitcoin_confirmation = current_confirmed
				.block_height
				.saturating_sub(T::MaxPendingConfirmationBlocks::get());
			let pending_confirmation = <UtxosPendingConfirmation<T>>::get();
			for (utxo_id, utxo_value) in pending_confirmation.into_iter() {
				if utxo_value.submitted_at_height < oldest_allowed_bitcoin_confirmation {
					let res = with_storage_layer(|| {
						Self::utxo_rejected(utxo_id, BitcoinRejectedReason::LookupExpired)
					});
					if let Err(e) = res {
						warn!(
							target: LOG_TARGET,
							"Failed to reject UTXO {:?} due to lookup expiration: {:?}", utxo_id, e
						);
						Self::deposit_event(Event::UtxoRejectedError { utxo_id, error: e });
					}
				}
			}

			// median time past is in seconds
			let last_synched_block = <SynchedBitcoinBlock<T>>::get()
				.map(|a| a.block_height)
				.unwrap_or(oldest_allowed_bitcoin_confirmation);

			for i in last_synched_block..=utxo_sync.sync_to_block.block_height {
				let utxos = <LockedUtxoExpirationsByBlock<T>>::take(i);
				for utxo_ref in utxos.into_iter() {
					let res = with_storage_layer(|| Self::utxo_expired(utxo_ref.clone()));
					if let Err(e) = res {
						warn!(
							target: LOG_TARGET,
							"Failed to expire UTXO {:?}: {:?}", utxo_ref, e
						);
						Self::deposit_event(Event::UtxoExpiredError { utxo_ref, error: e });
					}
				}
			}
			<SynchedBitcoinBlock<T>>::set(Some(utxo_sync.sync_to_block));

			Ok(())
		}

		/// Sets the most recent confirmed bitcoin block height (only executable by the Oracle
		/// Operator account)
		///
		/// # Arguments
		/// * `bitcoin_height` - the latest bitcoin block height to be confirmed
		#[pallet::call_index(1)]
		#[pallet::weight((0, DispatchClass::Operational))]
		pub fn set_confirmed_block(
			origin: OriginFor<T>,
			bitcoin_height: BitcoinHeight,
			bitcoin_block_hash: BitcoinBlockHash,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Some(who) == <OracleOperatorAccount<T>>::get(), Error::<T>::NoPermissions);
			if let Some(current) = <ConfirmedBitcoinBlockTip<T>>::get() {
				if bitcoin_height < current.block_height {
					return Ok(Pays::No.into());
				}
			}
			<ConfirmedBitcoinBlockTip<T>>::put(BitcoinBlock {
				block_height: bitcoin_height,
				block_hash: bitcoin_block_hash,
			});
			Ok(Pays::No.into())
		}

		/// Sets the oracle operator account id (only executable by the Root account)
		///
		/// # Arguments
		/// * `account_id` - the account id of the operator
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn set_operator(origin: OriginFor<T>, account_id: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			<OracleOperatorAccount<T>>::put(account_id.clone());
			Ok(())
		}
	}

	impl<T: Config> BitcoinUtxoTracker for Pallet<T> {
		fn new_utxo_id() -> UtxoId {
			let utxo_id = NextUtxoId::<T>::get().unwrap_or(1u32.into());

			NextUtxoId::<T>::set(Some(utxo_id + 1));
			utxo_id
		}

		fn watch_for_utxo(
			utxo_id: UtxoId,
			script_pubkey: BitcoinCosignScriptPubkey,
			satoshis: Satoshis,
			watch_for_spent_until: BitcoinHeight,
		) -> Result<(), DispatchError> {
			ensure!(!<UtxoIdToRef<T>>::contains_key(utxo_id), Error::<T>::ScriptPubkeyConflict);
			<UtxosPendingConfirmation<T>>::try_mutate(|utxo_pending_confirmation| {
				ensure!(
					!utxo_pending_confirmation
						.iter()
						.any(|(_, a)| a.script_pubkey == script_pubkey),
					Error::<T>::ScriptPubkeyConflict
				);
				utxo_pending_confirmation
					.try_insert(
						utxo_id,
						UtxoValue {
							utxo_id,
							satoshis,
							script_pubkey,
							watch_for_spent_until_height: watch_for_spent_until,
							submitted_at_height: <ConfirmedBitcoinBlockTip<T>>::get()
								.map(|a| a.block_height)
								.unwrap_or_default(),
						},
					)
					.map_err(|_| Error::<T>::MaxUtxosExceeded)?;
				Ok::<(), Error<T>>(())
			})?;
			Ok(())
		}

		fn get(utxo_id: UtxoId) -> Option<UtxoRef> {
			<UtxoIdToRef<T>>::get(utxo_id)
		}

		fn unwatch(utxo_id: UtxoId) {
			if let Some(utxo_ref) = <UtxoIdToRef<T>>::take(utxo_id) {
				if let Some(utxo_value) = <LockedUtxos<T>>::take(utxo_ref.clone()) {
					if LockedUtxoExpirationsByBlock::<T>::contains_key(
						utxo_value.watch_for_spent_until_height,
					) {
						LockedUtxoExpirationsByBlock::<T>::mutate(
							utxo_value.watch_for_spent_until_height,
							|utxos| {
								utxos.retain(|a| utxo_ref != *a);
							},
						);
					}
				}
			}
			<UtxosPendingConfirmation<T>>::mutate(|a| a.remove(&utxo_id));
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_sync_status() -> Option<BitcoinSyncStatus> {
			let confirmed_block = ConfirmedBitcoinBlockTip::<T>::get()?;
			let synched_block = SynchedBitcoinBlock::<T>::get();
			let oldest_allowed_block_height =
				confirmed_block.block_height.saturating_sub(T::MaxUtxoBirthBlocksOld::get());
			Some(BitcoinSyncStatus { confirmed_block, synched_block, oldest_allowed_block_height })
		}

		pub fn active_utxos() -> Vec<(Option<UtxoRef>, UtxoValue)> {
			let mut utxos = vec![];
			// TODO: This is not efficient, but we don't have a better way to do this right now
			for (utxo, entry) in <LockedUtxos<T>>::iter() {
				utxos.push((Some(utxo), entry));
			}
			for (_, entry) in <UtxosPendingConfirmation<T>>::get() {
				utxos.push((None, entry));
			}
			utxos
		}

		fn reject_utxo(utxo_id: UtxoId, reason: BitcoinRejectedReason) -> DispatchResult {
			T::EventHandler::utxo_rejected(utxo_id, reason.clone())?;
			Self::deposit_event(Event::UtxoRejected { utxo_id, rejected_reason: reason });
			Ok(())
		}

		pub fn utxo_verified(utxo_id: UtxoId, utxo_ref: UtxoRef) -> DispatchResult {
			let Some(entry) = <UtxosPendingConfirmation<T>>::mutate(|a| a.remove(&utxo_id)) else {
				return Ok(());
			};

			if <LockedUtxos<T>>::contains_key(&utxo_ref) {
				Self::reject_utxo(utxo_id, BitcoinRejectedReason::DuplicateUtxo)?;
				return Ok(());
			}

			Self::deposit_event(Event::UtxoVerified { utxo_id });

			<LockedUtxos<T>>::insert(utxo_ref.clone(), entry.clone());
			<UtxoIdToRef<T>>::insert(utxo_id, utxo_ref.clone());

			<LockedUtxoExpirationsByBlock<T>>::try_mutate(
				entry.watch_for_spent_until_height,
				|utxos| -> DispatchResult {
					Ok(utxos
						.try_push(utxo_ref.clone())
						.map_err(|_| Error::<T>::MaxUtxosExceeded)?)
				},
			)?;
			T::EventHandler::utxo_verified(utxo_id)?;
			Ok(())
		}

		pub fn utxo_rejected(
			utxo_id: UtxoId,
			rejected_reason: BitcoinRejectedReason,
		) -> DispatchResult {
			if <UtxosPendingConfirmation<T>>::mutate(|a| a.remove(&utxo_id)).is_some() {
				Self::reject_utxo(utxo_id, rejected_reason)?;
			}
			Ok(())
		}

		pub fn utxo_spent(utxo_id: UtxoId, block_height: BitcoinHeight) -> DispatchResult {
			if <UtxosPendingConfirmation<T>>::mutate(|a| a.remove(&utxo_id)).is_some() {
				Self::reject_utxo(utxo_id, BitcoinRejectedReason::Spent)?;
			}
			if let Some(utxo_ref) = <UtxoIdToRef<T>>::take(utxo_id) {
				if <LockedUtxos<T>>::take(utxo_ref.clone()).is_some() {
					T::EventHandler::utxo_spent(utxo_id)?;
					Self::deposit_event(Event::UtxoSpent { utxo_id, block_height });
				}
			}
			Ok(())
		}

		pub fn utxo_expired(utxo_ref: UtxoRef) -> DispatchResult {
			if let Some(entry) = LockedUtxos::<T>::take(utxo_ref.clone()) {
				let utxo_id = entry.utxo_id;
				UtxoIdToRef::<T>::remove(utxo_id);
				Self::deposit_event(Event::UtxoUnwatched { utxo_id });
				T::EventHandler::utxo_expired(utxo_id)?;
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
			let utxo_sync = data.bitcoin_sync().expect("Could not decode bitcoin inherent data");

			let utxo_sync = utxo_sync?;

			Some(Call::sync { utxo_sync })
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let sync = match call {
				Call::sync { ref utxo_sync } => utxo_sync,
				_ => return Ok(()),
			};

			if let Some(data) = data.bitcoin_sync().expect("Could not decode bitcoin inherent data")
			{
				if data != *sync {
					return Err(BitcoinInherentError::InvalidInherentData);
				}
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

	impl<T: Config> Get<argon_primitives::bitcoin::BitcoinNetwork> for Pallet<T> {
		fn get() -> argon_primitives::bitcoin::BitcoinNetwork {
			<BitcoinNetwork<T>>::get()
		}
	}

	impl<T: Config> Get<BitcoinHeight> for Pallet<T> {
		fn get() -> BitcoinHeight {
			<ConfirmedBitcoinBlockTip<T>>::get().map(|a| a.block_height).unwrap_or_default()
		}
	}
}
