#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;
use pallet_prelude::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use argon_primitives::{
		BitcoinUtxoEvents, BitcoinUtxoTracker,
		bitcoin::{
			BitcoinBlock, BitcoinBlockHash, BitcoinCosignScriptPubkey, BitcoinHeight,
			BitcoinRejectedReason, BitcoinSyncStatus, Satoshis, UtxoId, UtxoRef, UtxoValue,
		},
		inherents::{BitcoinInherentData, BitcoinInherentError, BitcoinUtxoSync},
	};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;

		type EventHandler: BitcoinUtxoEvents;

		/// The maximum number of UTXOs that can be tracked in a block and/or expiring at same block
		#[pallet::constant]
		type MaxPendingConfirmationUtxos: Get<u32>;

		/// Maximum bitcoin blocks to watch a Utxo for confirmation before canceling
		#[pallet::constant]
		type MaxPendingConfirmationBlocks: Get<BitcoinHeight>;
	}

	/// Locked Bitcoin UTXOs that have had ownership confirmed. If a Bitcoin UTXO is moved before
	/// the expiration block, the funds are burned and the UTXO is unlocked.
	#[pallet::storage]
	pub type LockedUtxos<T: Config> =
		StorageMap<_, Blake2_128Concat, UtxoRef, UtxoValue, OptionQuery>;

	#[pallet::storage]
	pub type UtxoIdToRef<T: Config> = StorageMap<_, Twox64Concat, UtxoId, UtxoRef, OptionQuery>;

	/// Bitcoin UTXOs that have been submitted for ownership confirmation
	#[pallet::storage]
	pub type UtxosPendingConfirmation<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<UtxoId, UtxoValue, T::MaxPendingConfirmationUtxos>,
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

	/// Expiration date as a day since unix timestamp mapped to Bitcoin UTXOs
	#[pallet::storage]
	pub type LockedUtxoExpirationsByBlock<T: Config> = StorageMap<
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
		/// Duplicated UtxoId. Already in use
		DuplicateUtxoId,
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
			<BitcoinNetwork<T>>::put(self.network);
		}
	}
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			TempParentHasSyncState::<T>::put(ConfirmedBitcoinBlockTip::<T>::get().is_some());
			PreviousBitcoinBlockTip::<T>::set(ConfirmedBitcoinBlockTip::<T>::get());
			// 1 write is temporary
			T::DbWeight::get().reads_writes(2, 1)
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
		/// Submitted when a bitcoin UTXO has been moved or confirmed
		#[pallet::call_index(0)]
		#[pallet::weight((
			T::WeightInfo::sync(
				utxo_sync.spent.len() as u32,
				utxo_sync.verified.len() as u32,
				utxo_sync.invalid.len() as u32
			),
			DispatchClass::Mandatory
		))]
		pub fn sync(origin: OriginFor<T>, utxo_sync: BitcoinUtxoSync) -> DispatchResult {
			ensure_none(origin)?;
			log::info!(
				"Bitcoin UTXO sync submitted (spent: {:?}, confirmed {}, rejected {})",
				utxo_sync.spent.len(),
				utxo_sync.verified.len(),
				utxo_sync.invalid.len()
			);

			ensure!(!InherentIncluded::<T>::get(), "Inherent already included");
			// this ensures we can be sure the inherent was included in a relay chain
			InherentIncluded::<T>::put(true);

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
					log::warn!("Failed to mark UTXO {} as spent: {:?}", utxo_id, e);
					Self::deposit_event(Event::UtxoSpentError { utxo_id, error: e });
				}
			}

			for (utxo_id, utxo_ref) in utxo_sync.verified.into_iter() {
				let res = with_storage_layer(|| Self::utxo_verified(utxo_id, utxo_ref));
				if let Err(e) = res {
					log::warn!("Failed to verify UTXO {}: {:?}", utxo_id, e);
					Self::deposit_event(Event::UtxoVerifiedError { utxo_id, error: e });
				}
			}

			for (utxo_id, rejected_reason) in utxo_sync.invalid.into_iter() {
				let res = with_storage_layer(|| Self::utxo_rejected(utxo_id, rejected_reason));
				if let Err(e) = res {
					log::warn!("Failed to reject UTXO {}: {:?}", utxo_id, e);
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
						log::warn!(
							"Failed to reject UTXO {:?} due to lookup expiration: {:?}",
							utxo_id,
							e
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
						log::warn!("Failed to expire UTXO {:?}: {:?}", utxo_ref, e);
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
		#[pallet::weight((T::WeightInfo::set_confirmed_block(), DispatchClass::Operational))]
		#[pallet::feeless_if(|origin: &OriginFor<T>, _height: &BitcoinHeight, _hash: &BitcoinBlockHash, | -> bool {
			let Ok(who) = ensure_signed(origin.clone()) else {
				return false;
			};
			Some(who) == <OracleOperatorAccount<T>>::get()
		})]
		pub fn set_confirmed_block(
			origin: OriginFor<T>,
			bitcoin_height: BitcoinHeight,
			bitcoin_block_hash: BitcoinBlockHash,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Some(who) == <OracleOperatorAccount<T>>::get(), Error::<T>::NoPermissions);
			if let Some(current) = <ConfirmedBitcoinBlockTip<T>>::get() {
				if bitcoin_height < current.block_height {
					return Ok(());
				}
			}
			<ConfirmedBitcoinBlockTip<T>>::put(BitcoinBlock {
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
			<OracleOperatorAccount<T>>::put(account_id.clone());
			Ok(())
		}
	}

	impl<T: Config> BitcoinUtxoTracker for Pallet<T> {
		fn watch_for_utxo(
			utxo_id: UtxoId,
			script_pubkey: BitcoinCosignScriptPubkey,
			satoshis: Satoshis,
			watch_for_spent_until: BitcoinHeight,
		) -> Result<(), DispatchError> {
			ensure!(!<UtxoIdToRef<T>>::contains_key(utxo_id), Error::<T>::DuplicateUtxoId);
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
			// We have full visibility into everything up to the synched point
			let oldest_allowed_block_height = if let Some(ref x) = synched_block {
				x.block_height
			} else {
				let mut oldest = confirmed_block.block_height;
				for (_, entry) in <UtxosPendingConfirmation<T>>::get() {
					if entry.submitted_at_height < oldest {
						oldest = entry.submitted_at_height;
					}
				}
				oldest
			};
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
				Call::sync { utxo_sync } => utxo_sync,
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
