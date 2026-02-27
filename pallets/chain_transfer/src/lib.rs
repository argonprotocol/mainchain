#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use argon_notary_audit::VerifyError as NotebookVerifyError;
pub use pallet::*;
use pallet_prelude::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use argon_primitives::{
		BurnEventHandler, ChainTransferLookup, NotebookEventHandler, NotebookProvider,
		TickProvider, TransferToLocalchainId,
		notary::NotaryProvider,
		notebook::{ChainTransfer, NotebookHeader},
	};

	use sp_core::crypto::AccountId32;
	use sp_runtime::traits::AccountIdConversion;

	use sp_core::Get;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config<AccountId = AccountId32, Hash = H256>
	{
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		type Argon: Mutate<<Self as frame_system::Config>::AccountId, Balance = <Self as Config>::Balance>;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Member
			+ Copy
			+ MaybeSerializeDeserialize
			+ DecodeWithMemTracking
			+ Debug
			+ Default
			+ From<u128>
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		type NotebookProvider: NotebookProvider;
		type NotaryProvider: NotaryProvider<Self::Block, <Self as frame_system::Config>::AccountId>;
		type TickProvider: TickProvider<Self::Block>;
		type EventHandler: BurnEventHandler<<Self as Config>::Balance>;
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// How long a transfer should remain in storage before returning. NOTE: there is a 2 tick
		/// grace period where we will still allow a transfer
		#[pallet::constant]
		type TransferExpirationTicks: Get<Tick>;

		/// How many transfers out can be queued per block
		#[pallet::constant]
		type MaxPendingTransfersOutPerBlock: Get<u32>;
	}

	#[pallet::storage]
	pub type NextTransferId<T: Config> = StorageValue<_, TransferToLocalchainId, OptionQuery>;

	#[pallet::storage]
	pub type PendingTransfersOut<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		TransferToLocalchainId,
		QueuedTransferOut<<T as frame_system::Config>::AccountId, <T as Config>::Balance>,
		OptionQuery,
	>;

	#[pallet::storage]
	/// Expiration index for outgoing transfers keyed by `(notary_id, expiration_tick)`.
	///
	/// NOTE: Expiration processing follows notebook progression (`header.tick`) for each notary,
	/// not wall/runtime tick. If a notary stops submitting notebooks indefinitely, pending
	/// transfers for that notary remain frozen by design until a notary-switch recovery path is
	/// executed.
	pub type ExpiringTransfersOutByNotary<T: Config> = StorageDoubleMap<
		Hasher1 = Twox64Concat,
		Hasher2 = Twox64Concat,
		Key1 = NotaryId,
		Key2 = Tick,
		Value = BoundedVec<TransferToLocalchainId, T::MaxPendingTransfersOutPerBlock>,
		QueryKind = ValueQuery,
	>;

	#[pallet::storage]
	pub type TransfersUsedInBlockNotebooks<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<
			(<T as frame_system::Config>::AccountId, <T as frame_system::Config>::Nonce),
			T::MaxPendingTransfersOutPerBlock,
		>,
		ValueQuery,
	>;

	/// The admin of the hyperbridge token gateway
	#[pallet::storage]
	pub type HyperbridgeTokenAdmin<T: Config> =
		StorageValue<_, <T as frame_system::Config>::AccountId, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Funds sent to a localchain
		TransferToLocalchain {
			account_id: <T as frame_system::Config>::AccountId,
			amount: <T as Config>::Balance,
			transfer_id: TransferToLocalchainId,
			notary_id: NotaryId,
			expiration_tick: Tick,
		},
		/// Transfer to localchain expired and rolled back
		TransferToLocalchainExpired {
			account_id: <T as frame_system::Config>::AccountId,
			transfer_id: TransferToLocalchainId,
			notary_id: NotaryId,
		},
		/// Transfer from Localchain to Mainchain
		TransferFromLocalchain {
			account_id: <T as frame_system::Config>::AccountId,
			amount: <T as Config>::Balance,
			notary_id: NotaryId,
		},
		/// A transfer into the mainchain failed
		TransferFromLocalchainError {
			account_id: <T as frame_system::Config>::AccountId,
			amount: <T as Config>::Balance,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			error: DispatchError,
		},
		/// An expired transfer to localchain failed to be refunded
		TransferToLocalchainRefundError {
			account_id: <T as frame_system::Config>::AccountId,
			transfer_id: TransferToLocalchainId,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			error: DispatchError,
		},
		/// A localchain transfer could not be cleaned up properly. Possible invalid transfer
		/// needing investigation.
		PossibleInvalidLocalchainTransferAllowed {
			transfer_id: TransferToLocalchainId,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
		},
		/// Taxation failed
		TaxationError {
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			tax: <T as Config>::Balance,
			error: DispatchError,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		MaxBlockTransfersExceeded,
		/// Insufficient balance to create this transfer
		InsufficientFunds,
		/// Insufficient balance to fulfill a mainchain transfer
		InsufficientNotarizedFunds,
		/// The transfer was already submitted in a previous block
		InvalidOrDuplicatedLocalchainTransfer,
		/// A transfer was submitted in a previous block but the expiration block has passed
		NotebookIncludesExpiredLocalchainTransfer,
		/// The notary id is not registered
		InvalidNotaryUsedForTransfer,
		/// The notary is currently locked and cannot process transfers
		NotaryLockedForTransfer,
		/// No transfer IDs are currently available
		NoAvailableTransferId,
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub hyperbridge_token_admin: Option<<T as frame_system::Config>::AccountId>,
		#[serde(skip)]
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if let Some(admin) = self.hyperbridge_token_admin.clone() {
				HyperbridgeTokenAdmin::<T>::put(admin);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		[u8; 32]: From<<T as frame_system::Config>::AccountId>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::send_to_localchain())]
		pub fn send_to_localchain(
			origin: OriginFor<T>,
			#[pallet::compact] amount: <T as Config>::Balance,
			notary_id: NotaryId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let current_tick = T::TickProvider::current_tick();
			ensure!(
				T::NotaryProvider::active_notaries().contains(&notary_id),
				Error::<T>::InvalidNotaryUsedForTransfer,
			);
			ensure!(
				!T::NotebookProvider::is_notary_locked_at_tick(notary_id, current_tick),
				Error::<T>::NotaryLockedForTransfer,
			);

			ensure!(
				T::Argon::reducible_balance(&who, Preservation::Expendable, Fortitude::Force) >=
					amount,
				Error::<T>::InsufficientFunds,
			);

			// the nonce is incremented pre-dispatch. we want the nonce for the transaction
			let transfer_id = Pallet::<T>::next_transfer_id()?;

			T::Argon::transfer(
				&who,
				&Self::notary_account_id(notary_id),
				amount,
				Preservation::Expendable,
			)?;

			let expiration_tick: Tick = current_tick + T::TransferExpirationTicks::get();

			PendingTransfersOut::<T>::insert(
				transfer_id,
				QueuedTransferOut { account_id: who.clone(), amount, expiration_tick, notary_id },
			);
			ExpiringTransfersOutByNotary::<T>::try_append(notary_id, expiration_tick, transfer_id)
				.map_err(|_| Error::<T>::MaxBlockTransfersExceeded)?;

			Self::deposit_event(Event::TransferToLocalchain {
				account_id: who,
				amount,
				transfer_id,
				notary_id,
				expiration_tick,
			});
			Ok(())
		}
	}

	impl<T: Config> NotebookEventHandler for Pallet<T> {
		fn notebook_submitted(header: &NotebookHeader) {
			let notary_id = header.notary_id;

			// un-spendable notary account
			let notary_pallet_account_id = Self::notary_account_id(notary_id);
			for transfer in header.chain_transfers.iter() {
				match transfer {
					ChainTransfer::ToMainchain { account_id, amount } => {
						let amount = (*amount).into();
						if let Err(e) = Self::transfer_funds_to_mainchain(
							&notary_pallet_account_id,
							account_id,
							amount,
						) {
							Self::deposit_event(Event::TransferFromLocalchainError {
								notary_id,
								notebook_number: header.notebook_number,
								account_id: account_id.clone(),
								amount,
								error: e,
							});
						} else {
							Self::deposit_event(Event::TransferFromLocalchain {
								notary_id,
								account_id: account_id.clone(),
								amount,
							});
						}
					},
					ChainTransfer::ToLocalchain { transfer_id } => {
						if let Some(transfer) = PendingTransfersOut::<T>::take(transfer_id) {
							ExpiringTransfersOutByNotary::<T>::mutate_exists(
								transfer.notary_id,
								transfer.expiration_tick,
								|maybe_expiring| {
									let Some(expiring) = maybe_expiring else {
										return;
									};
									if let Some(pos) =
										expiring.iter().position(|x| x == transfer_id)
									{
										expiring.remove(pos);
									}
									if expiring.is_empty() {
										*maybe_expiring = None;
									}
								},
							);
						} else {
							Self::deposit_event(Event::PossibleInvalidLocalchainTransferAllowed {
								transfer_id: *transfer_id,
								notebook_number: header.notebook_number,
								notary_id,
							});
						}
					},
				}
			}

			if header.tax > 0 {
				if let Err(e) = T::Argon::burn_from(
					&notary_pallet_account_id,
					header.tax.into(),
					Preservation::Preserve,
					Precision::Exact,
					Fortitude::Force,
				) {
					Self::deposit_event(Event::TaxationError {
						notary_id,
						notebook_number: header.notebook_number,
						tax: header.tax.into(),
						error: e,
					});
				}
				T::EventHandler::on_argon_burn(&header.tax.into());
			}

			// Use notebook tick progression as the expiry boundary so delayed-but-valid notebooks
			// can still consume transfers before they are treated as stale.
			let expiring_ticks: Vec<Tick> =
				ExpiringTransfersOutByNotary::<T>::iter_key_prefix(notary_id)
					.filter(|tick| *tick < header.tick)
					.collect();

			for tick in expiring_ticks.into_iter() {
				let expiring = ExpiringTransfersOutByNotary::<T>::take(notary_id, tick);
				for transfer_id in expiring.into_iter() {
					let Some(transfer) = PendingTransfersOut::<T>::take(transfer_id) else {
						continue;
					};
					match T::Argon::transfer(
						&Self::notary_account_id(transfer.notary_id),
						&transfer.account_id,
						transfer.amount,
						Preservation::Expendable,
					) {
						Ok(_) => {
							Self::deposit_event(Event::TransferToLocalchainExpired {
								account_id: transfer.account_id,
								transfer_id,
								notary_id: transfer.notary_id,
							});
						},
						Err(e) => {
							// can't panic here or chain will get stuck
							log::warn!(
								"Failed to return pending Localchain transfer to account {:?} (amount={:?}): {:?}",
								&transfer.account_id,
								transfer.amount,
								e
							);
							Self::deposit_event(Event::TransferToLocalchainRefundError {
								account_id: transfer.account_id,
								notebook_number: header.notebook_number,
								transfer_id,
								notary_id: transfer.notary_id,
								error: e,
							});
						},
					}
				}
			}
		}
	}

	impl<T: Config>
		ChainTransferLookup<<T as frame_system::Config>::AccountId, <T as Config>::Balance>
		for Pallet<T>
	{
		type Weights = crate::weights::ProviderWeightAdapter<T>;

		fn is_valid_transfer_to_localchain(
			notary_id: NotaryId,
			transfer_id: TransferToLocalchainId,
			account_id: &<T as frame_system::Config>::AccountId,
			microgons: <T as Config>::Balance,
			at_tick: Tick,
		) -> bool {
			let result = PendingTransfersOut::<T>::get(transfer_id);
			if let Some(transfer) = result {
				return transfer.notary_id == notary_id &&
					transfer.amount == microgons &&
					transfer.account_id == *account_id &&
					transfer.expiration_tick >= at_tick;
			}

			false
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn transfer_funds_to_mainchain(
			notary_pallet_account_id: &<T as frame_system::Config>::AccountId,
			account_id: &<T as frame_system::Config>::AccountId,
			amount: <T as Config>::Balance,
		) -> DispatchResult {
			ensure!(
				T::Argon::reducible_balance(
					notary_pallet_account_id,
					Preservation::Expendable,
					Fortitude::Force,
				) >= amount,
				Error::<T>::InsufficientNotarizedFunds
			);
			T::Argon::transfer(
				notary_pallet_account_id,
				account_id,
				amount,
				Preservation::Expendable,
			)?;
			Ok(())
		}

		pub fn notary_account_id(notary_id: NotaryId) -> <T as frame_system::Config>::AccountId {
			T::PalletId::get().into_sub_account_truncating(notary_id)
		}

		pub fn hyperbridge_token_admin() -> <T as frame_system::Config>::AccountId {
			HyperbridgeTokenAdmin::<T>::get().expect("Should have been initialized in genesis")
		}

		fn next_transfer_id() -> Result<TransferToLocalchainId, Error<T>> {
			let mut next_transfer_id = NextTransferId::<T>::get().unwrap_or(1);
			if next_transfer_id == 0 {
				next_transfer_id = 1;
			}
			ensure!(
				!PendingTransfersOut::<T>::contains_key(next_transfer_id),
				Error::<T>::NoAvailableTransferId
			);
			let transfer_id = next_transfer_id;

			next_transfer_id = transfer_id.wrapping_add(1);
			if next_transfer_id == 0 {
				next_transfer_id = 1;
			}
			NextTransferId::<T>::set(Some(next_transfer_id));
			Ok(transfer_id)
		}
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound(AccountId: MaxEncodedLen, Balance: MaxEncodedLen))]
pub struct QueuedTransferOut<AccountId, Balance> {
	pub account_id: AccountId,
	pub amount: Balance,
	pub expiration_tick: Tick,
	pub notary_id: NotaryId,
}
