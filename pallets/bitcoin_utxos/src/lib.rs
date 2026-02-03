#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;
use pallet_prelude::*;
pub use weights::*;

pub mod migrations;

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
			BitcoinSyncStatus, Satoshis, UtxoId, UtxoRef, UtxoValue,
		},
		inherents::{BitcoinInherentData, BitcoinInherentError, BitcoinUtxoSync},
	};
	use pallet_prelude::argon_primitives::{
		bitcoin::BitcoinRejectedReason, inherents::BitcoinUtxoFunding,
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		type WeightInfo: WeightInfo;

		type EventHandler: BitcoinUtxoEvents<Self::AccountId, BlockNumberFor<Self>>;

		/// The maximum number of UTXOs that can be watched in a block and/or expiring at same block
		#[pallet::constant]
		type MaxPendingConfirmationUtxos: Get<u32>;
		/// Maximum number of candidate UTXOs stored per lock
		#[pallet::constant]
		type MaxCandidateUtxosPerLock: Get<u32>;

		/// Maximum bitcoin blocks to watch a Utxo for confirmation before canceling
		#[pallet::constant]
		type MaxPendingConfirmationBlocks: Get<BitcoinHeight>;

		/// Minimum number of satoshis required to consider a UTXO as a candidate
		type MinimumSatoshisPerCandidateUtxo: Get<Satoshis>;

		/// Maximum number of satoshi difference allowed from expected to consider a UTXO as
		/// "confirmed"
		#[pallet::constant]
		type MaximumSatoshiThresholdFromExpected: Get<Satoshis>;
	}

	/// Locked Bitcoin UTXOs that have been funded with a UtxoRef from the Bitcoin network and
	/// amounts within the MinimumSatoshiThreshold of the expected. If a Bitcoin UTXO is moved
	/// before the expiration block, the funds are burned and the UTXO is unlocked.
	#[pallet::storage]
	pub type LockedUtxos<T: Config> =
		StorageMap<_, Blake2_128Concat, UtxoRef, UtxoValue, OptionQuery>;

	/// A mapping of utxo id to the confirmed utxo reference
	#[pallet::storage]
	pub type UtxoIdToFundingUtxoRef<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, UtxoRef, OptionQuery>;

	/// Bitcoin locks that are pending full funding on the bitcoin network
	#[pallet::storage]
	pub type LocksPendingFunding<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<UtxoId, UtxoValue, T::MaxPendingConfirmationUtxos>,
		ValueQuery,
	>;

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct CandidateUtxo {
		#[codec(compact)]
		pub utxo_id: UtxoId,
		#[codec(compact)]
		pub satoshis: Satoshis,
	}

	/// Candidate UTXOs associated with a lock (mismatches, extra funding, etc.).
	#[pallet::storage]
	pub type CandidateUtxoRefsByUtxoId<T: Config> = StorageMap<
		_,
		Twox64Concat,
		UtxoId,
		BoundedBTreeMap<UtxoRef, Satoshis, T::MaxCandidateUtxosPerLock>,
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
		UtxoVerified {
			utxo_id: UtxoId,
			satoshis_received: Satoshis,
		},
		UtxoRejected {
			utxo_id: UtxoId,
			utxo_ref: UtxoRef,
			rejected_reason: BitcoinRejectedReason,
			satoshis_received: Satoshis,
		},
		UtxoSpent {
			utxo_id: UtxoId,
			block_height: BitcoinHeight,
		},
		UtxoUnwatched {
			utxo_id: UtxoId,
		},

		UtxoSpentError {
			utxo_id: UtxoId,
			error: DispatchError,
		},
		UtxoVerifiedError {
			utxo_id: UtxoId,
			error: DispatchError,
		},
		UtxoRejectedError {
			utxo_id: UtxoId,
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
		/// Locked Utxo Not Found
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
		/// Duplicated UtxoId. Already in use
		DuplicateUtxoId,
		/// Too many candidate UTXOs are being stored for this lock
		MaxCandidateUtxosExceeded,
		/// The UTXO reference does not map to a candidate entry
		UtxoNotCandidate,
		/// This Lock already has an attached funding UTXO
		LockAlreadyFunded,
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
		/// Submitted when a bitcoin UTXO has been moved or confirmed.
		#[pallet::call_index(0)]
		#[pallet::weight((
			T::WeightInfo::sync(
				utxo_sync.spent.len() as u32,
				utxo_sync.funded.len() as u32,
			),
			DispatchClass::Mandatory
		))]
		pub fn sync(origin: OriginFor<T>, utxo_sync: BitcoinUtxoSync) -> DispatchResult {
			ensure_none(origin)?;
			log::info!(
				"Bitcoin UTXO sync submitted (spent: {:?}, funded {})",
				utxo_sync.spent.len(),
				utxo_sync.funded.len(),
			);

			ensure!(!InherentIncluded::<T>::get(), "Inherent already included");
			// this ensures we can be sure the inherent was included in a relay chain
			InherentIncluded::<T>::put(true);

			let BitcoinUtxoSync { sync_to_block, funded, spent } = utxo_sync;

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

			for funding in funded {
				let BitcoinUtxoFunding { utxo_id, utxo_ref, satoshis, bitcoin_height, .. } =
					funding;
				let res = with_storage_layer(|| {
					Self::lock_funding_received(utxo_id, utxo_ref, satoshis, bitcoin_height)
				});
				if let Err(e) = res {
					log::warn!("Failed to verify UTXO {}: {:?}", utxo_id, e);
					Self::deposit_event(Event::UtxoVerifiedError { utxo_id, error: e });
				}
			}

			// process funding first, so any funded + spent received in the same block are handled
			// correctly
			for spend in spent {
				let err =
					with_storage_layer(|| Self::utxo_spent(spend.utxo_id, spend.bitcoin_height));
				if let Err(e) = err {
					log::warn!("Failed to mark UTXO {} as spent: {:?}", spend.utxo_id, e);
					Self::deposit_event(Event::UtxoSpentError { utxo_id: spend.utxo_id, error: e });
				}
			}

			let oldest_pending_bitcoin_submitted_height = current_confirmed
				.block_height
				.saturating_sub(T::MaxPendingConfirmationBlocks::get());
			let locks_pending = LocksPendingFunding::<T>::get();
			for (utxo_id, utxo_value) in locks_pending.into_iter() {
				if utxo_value.submitted_at_height < oldest_pending_bitcoin_submitted_height {
					let res = with_storage_layer(|| Self::lock_timeout_pending_funding(utxo_id));
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
			if let Some(current) = ConfirmedBitcoinBlockTip::<T>::get() {
				if bitcoin_height < current.block_height {
					return Ok(());
				}
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

		/// Bind a candidate UTXO ref as the funding UTXO for its lock.
		/// The locks pallet authorizes the promotion; this pallet binds the ref and begins
		/// tracking.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::fund_with_utxo_candidate())]
		pub fn fund_with_utxo_candidate(
			origin: OriginFor<T>,
			utxo_id: UtxoId,
			utxo_ref: UtxoRef,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				!UtxoIdToFundingUtxoRef::<T>::contains_key(utxo_id),
				Error::<T>::LockAlreadyFunded
			);
			ensure!(!LockedUtxos::<T>::contains_key(&utxo_ref), Error::<T>::LockAlreadyFunded);

			let mut candidates = CandidateUtxoRefsByUtxoId::<T>::take(utxo_id);
			let satoshis = candidates.remove(&utxo_ref).ok_or(Error::<T>::UtxoNotCandidate)?;

			T::EventHandler::funding_promoted_by_account(utxo_id, satoshis, &who, &utxo_ref)?;
			let mut entry = LocksPendingFunding::<T>::mutate(|a| a.remove(&utxo_id))
				.ok_or(Error::<T>::UtxoNotLocked)?;
			entry.satoshis = satoshis;
			LockedUtxos::<T>::insert(utxo_ref.clone(), entry);
			UtxoIdToFundingUtxoRef::<T>::insert(utxo_id, utxo_ref.clone());
			for (candidate, satoshis) in candidates {
				T::EventHandler::orphaned_utxo_detected(utxo_id, satoshis, candidate)?;
			}

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
			ensure!(
				!UtxoIdToFundingUtxoRef::<T>::contains_key(utxo_id),
				Error::<T>::DuplicateUtxoId
			);
			LocksPendingFunding::<T>::try_mutate(|utxo_pending_confirmation| {
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

		fn get_funding_utxo_ref(utxo_id: UtxoId) -> Option<UtxoRef> {
			UtxoIdToFundingUtxoRef::<T>::get(utxo_id)
		}

		fn unwatch(utxo_id: UtxoId) {
			if let Some(utxo_ref) = UtxoIdToFundingUtxoRef::<T>::take(utxo_id) {
				LockedUtxos::<T>::take(utxo_ref);
			}
			LocksPendingFunding::<T>::mutate(|a| a.remove(&utxo_id));
			let _ = CandidateUtxoRefsByUtxoId::<T>::take(utxo_id);
		}

		fn unwatch_candidate(utxo_id: UtxoId, utxo_ref: &UtxoRef) -> Option<(UtxoRef, Satoshis)> {
			let mut result = None;
			CandidateUtxoRefsByUtxoId::<T>::mutate_exists(utxo_id, |refs| {
				if let Some(inner) = refs.as_mut() {
					if let Some(sats) = inner.remove(utxo_ref) {
						result = Some((utxo_ref.clone(), sats));
					}
					let should_clear = inner.is_empty();
					if should_clear {
						*refs = None;
					}
				}
			});
			result
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
				for (_, entry) in LocksPendingFunding::<T>::get() {
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
			for (utxo, entry) in LockedUtxos::<T>::iter() {
				utxos.push((Some(utxo), entry));
			}
			let pending = LocksPendingFunding::<T>::get();
			for (utxo_id, entry) in pending.iter() {
				utxos.push((None, entry.clone()));
				// Make sure to only look at all candidate utxos if a utxo is still pending funding
				let candidate_refs = CandidateUtxoRefsByUtxoId::<T>::get(*utxo_id);
				for (utxo_ref, _) in candidate_refs {
					utxos.push((Some(utxo_ref), entry.clone()));
				}
			}
			utxos
		}

		fn send_candidates_as_orphans(utxo_id: UtxoId) -> DispatchResult {
			let candidate_refs = CandidateUtxoRefsByUtxoId::<T>::take(utxo_id);
			for (utxo_ref, satoshis) in candidate_refs {
				T::EventHandler::orphaned_utxo_detected(utxo_id, satoshis, utxo_ref)?;
			}
			Ok(())
		}

		fn record_candidate(
			utxo_id: UtxoId,
			utxo_ref: UtxoRef,
			satoshis: Satoshis,
		) -> DispatchResult {
			CandidateUtxoRefsByUtxoId::<T>::try_mutate(utxo_id, |refs| {
				if refs.contains_key(&utxo_ref) {
					return Ok::<(), Error<T>>(());
				}
				refs.try_insert(utxo_ref.clone(), satoshis)
					.map_err(|_| Error::<T>::MaxCandidateUtxosExceeded)?;
				Ok::<(), Error<T>>(())
			})?;
			Ok(())
		}

		pub fn lock_funding_received(
			utxo_id: UtxoId,
			utxo_ref: UtxoRef,
			satoshis: Satoshis,
			bitcoin_height: BitcoinHeight,
		) -> DispatchResult {
			if satoshis < T::MinimumSatoshisPerCandidateUtxo::get() {
				tracing::info!(utxo_id = ?utxo_id, satoshis = ?satoshis,
					"UTXO funding below minimum threshold");
				return Ok(())
			}

			if LockedUtxos::<T>::contains_key(&utxo_ref) {
				tracing::info!(utxo_id = ?utxo_id, satoshis = ?satoshis, utxo_ref = ?utxo_ref, bitcoin_height,
					"UTXO duplicate received");
				return Ok(());
			}

			let mut rejected_reason = BitcoinRejectedReason::AlreadyVerified;
			let pending_funding = LocksPendingFunding::<T>::get();
			let Some(entry) = pending_funding.get(&utxo_id) else {
				if UtxoIdToFundingUtxoRef::<T>::contains_key(utxo_id) {
					T::EventHandler::orphaned_utxo_detected(utxo_id, satoshis, utxo_ref.clone())?;
					// If the lock is already funded, this is treated as an unbacked candidate UTXO.
					Self::deposit_event(Event::UtxoRejected {
						utxo_id,
						utxo_ref: utxo_ref.clone(),
						satoshis_received: satoshis,
						rejected_reason,
					});
				} else {
					tracing::info!(utxo_id = ?utxo_id, satoshis = ?satoshis, utxo_ref = ?utxo_ref, bitcoin_height,
						"UTXO not being waited for");
				}
				return Ok(())
			};
			let max_acceptance_height =
				entry.submitted_at_height.saturating_add(T::MaxPendingConfirmationBlocks::get());

			let is_within_threshold = if satoshis >= entry.satoshis {
				satoshis.saturating_sub(entry.satoshis) <=
					T::MaximumSatoshiThresholdFromExpected::get()
			} else {
				entry.satoshis.saturating_sub(satoshis) <=
					T::MaximumSatoshiThresholdFromExpected::get()
			};
			let is_within_allowed_height = bitcoin_height <= max_acceptance_height;
			if !is_within_threshold {
				rejected_reason = BitcoinRejectedReason::SatoshisOutsideAcceptedRange;
			} else if !is_within_allowed_height {
				rejected_reason = BitcoinRejectedReason::VerificationExpired;
			}

			if is_within_allowed_height && is_within_threshold {
				LockedUtxos::<T>::insert(utxo_ref.clone(), entry.clone());
				UtxoIdToFundingUtxoRef::<T>::insert(utxo_id, utxo_ref.clone());
				LocksPendingFunding::<T>::mutate(|a| a.remove(&utxo_id));
				// this shouldn't be possible, but remove just as a defensive measure
				Self::unwatch_candidate(utxo_id, &utxo_ref);
				// at this point, any other candidates are now orphans
				Self::send_candidates_as_orphans(utxo_id)?;

				T::EventHandler::funding_received(utxo_id, satoshis)?;
				Self::deposit_event(Event::UtxoVerified { utxo_id, satoshis_received: satoshis });
			} else {
				Self::record_candidate(utxo_id, utxo_ref.clone(), satoshis)?;
				Self::deposit_event(Event::UtxoRejected {
					utxo_id,
					utxo_ref: utxo_ref.clone(),
					satoshis_received: satoshis,
					rejected_reason,
				});
			}

			Ok(())
		}

		pub fn lock_timeout_pending_funding(utxo_id: UtxoId) -> DispatchResult {
			if LocksPendingFunding::<T>::mutate(|a| a.remove(&utxo_id)).is_some() {
				// send candidates first!
				Self::send_candidates_as_orphans(utxo_id)?;
				<Self as BitcoinUtxoTracker>::unwatch(utxo_id);
				T::EventHandler::timeout_waiting_for_funding(utxo_id)?;
				Self::deposit_event(Event::UtxoUnwatched { utxo_id });
			}
			Ok(())
		}

		pub fn utxo_spent(utxo_id: UtxoId, block_height: BitcoinHeight) -> DispatchResult {
			if let Some(locked_ref) = UtxoIdToFundingUtxoRef::<T>::take(utxo_id) {
				LockedUtxos::<T>::take(locked_ref);
			}
			LocksPendingFunding::<T>::mutate(|a| a.remove(&utxo_id));
			let _ = CandidateUtxoRefsByUtxoId::<T>::take(utxo_id);

			T::EventHandler::spent(utxo_id)?;
			Self::deposit_event(Event::UtxoSpent { utxo_id, block_height });
			Ok(())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = BitcoinInherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier =
			argon_primitives::inherents::BITCOIN_INHERENT_IDENTIFIER_V2;

		fn create_inherent(data: &InherentData) -> Option<Self::Call>
		where
			InherentData: BitcoinInherentData,
		{
			let utxo_sync = data.bitcoin_sync().expect("Could not decode bitcoin inherent data");
			utxo_sync.map(|utxo_sync| Call::sync { utxo_sync })
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let sync = match call {
				Call::sync { utxo_sync } => utxo_sync,
				_ => return Ok(()),
			};

			let Some(data_v2) =
				data.bitcoin_sync().expect("Could not decode bitcoin inherent data")
			else {
				return Err(BitcoinInherentError::InvalidInherentData);
			};
			if data_v2 != *sync {
				return Err(BitcoinInherentError::InvalidInherentData);
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
