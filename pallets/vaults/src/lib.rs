#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;
use sp_runtime::BoundedVec;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod migrations;
pub mod weights;

/// The vaults pallet allows a user to offer argons for lease to other users. There are two types of
/// obligations offered in the system, Bitcoin and Mining obligations. Vaults can define the amount
/// of argons available for each type of obligation (bonded bitcoins and bitcoin locks), and the
/// interest rate for each. However, Bonded Bitcoins may only issued up to the amount of bitcoins
/// that are locked.
///
/// ** Additional Bitcoin Securitization **
///
/// A vault may apply added bitcoin securitization to their account up to 2x the locked value of
/// their bitcoin argons. This allows a vault to issue more mining obligations, but the funds are
/// locked up for the duration of the bitcoin locks, and will be taken in the case of bitcoins not
/// being cosigned on unlock.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use alloc::{vec, vec::Vec};
	use core::fmt::Debug;

	use super::*;
	use argon_bitcoin::{CosignScript, CosignScriptArgs};
	use argon_primitives::{
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinNetwork, BitcoinXPub,
			CompressedBitcoinPubkey, OpaqueBitcoinXpub,
		},
		block_seal::CohortId,
		tick::Tick,
		vault::{
			BitcoinObligationProvider, BondedBitcoinsBidPoolProvider, FundType, Obligation,
			ObligationError, ObligationExpiration, ReleaseFundsResult, Vault, VaultArgons,
			VaultTerms,
		},
		MiningSlotProvider, ObligationEvents, ObligationId, TickProvider, VaultId,
	};
	use codec::Codec;
	use frame_support::{
		pallet_prelude::*,
		storage::with_storage_layer,
		traits::{
			fungible::{Inspect, InspectHold, Mutate, MutateHold},
			tokens::{Fortitude, Precision, Preservation, Restriction},
			Incrementable,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use log::warn;
	use sp_runtime::{
		traits::{
			AccountIdConversion, AtLeast32BitUnsigned, CheckedAdd, CheckedDiv, CheckedSub,
			UniqueSaturatedInto, Zero,
		},
		ArithmeticError,
		DispatchError::Token,
		FixedPointNumber, FixedU128, Perbill, Percent, Saturating, TokenError,
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;

		type Currency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ core::fmt::Debug
			+ Default
			+ From<u128>
			+ TryInto<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		/// Minimum amount for an obligation
		#[pallet::constant]
		type MinimumObligationAmount: Get<Self::Balance>;

		/// Argon blocks per day
		#[pallet::constant]
		type TicksPerDay: Get<Tick>;

		/// The max pending vault term changes per block
		#[pallet::constant]
		type MaxPendingTermModificationsPerTick: Get<u32>;

		/// The number of ticks that a funding change will be delayed before it takes effect
		#[pallet::constant]
		type MiningArgonIncreaseTickDelay: Get<Tick>;

		/// A provider of mining slot information
		type MiningSlotProvider: MiningSlotProvider;

		/// Provides the bitcoin network this blockchain is connected to
		type GetBitcoinNetwork: Get<BitcoinNetwork>;
		/// Bitcoin time provider
		type BitcoinBlockHeightChange: Get<(BitcoinHeight, BitcoinHeight)>;

		type TickProvider: TickProvider<Self::Block>;
		/// Pallet storage requires bounds, so we have to set a maximum number that can expire in a
		/// single block
		#[pallet::constant]
		type MaxConcurrentlyExpiringObligations: Get<u32>;

		/// Max entrants allowed in the bid pool. This is only present because substrate prefers
		/// limits
		#[pallet::constant]
		type MaxBidPoolEntrants: Get<u32>;

		/// Callbacks for various vault obligation events
		type EventHandler: ObligationEvents<Self::AccountId, Self::Balance>;

		/// Maturation period for base fees
		type BaseFeeMaturationTicks: Get<Tick>;

		/// A pallet id that is used to hold the bid pool
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The minimum bid pool prorata percent
		type MinBidPoolProrataPercent: Get<Perbill>;

		/// Bid Pool burn percent
		type BidPoolBurnPercent: Get<Percent>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		EnterVault,
		ObligationFee,
	}

	#[pallet::storage]
	pub(super) type NextVaultId<T: Config> = StorageValue<_, VaultId, OptionQuery>;

	/// Vaults by id
	#[pallet::storage]
	pub(super) type VaultsById<T: Config> =
		StorageMap<_, Twox64Concat, VaultId, Vault<T::AccountId, T::Balance>, OptionQuery>;

	/// Vault Bitcoin Xpub and current child counter by VaultId
	#[pallet::storage]
	pub(super) type VaultXPubById<T: Config> =
		StorageMap<_, Twox64Concat, VaultId, (BitcoinXPub, u32), OptionQuery>;

	/// Pending terms that will be committed at the given block number (must be a minimum of 1 slot
	/// change away)
	#[pallet::storage]
	pub(super) type PendingTermsModificationsByTick<T: Config> = StorageMap<
		_,
		Twox64Concat,
		Tick,
		BoundedVec<VaultId, T::MaxPendingTermModificationsPerTick>,
		ValueQuery,
	>;

	/// Pending base fee hold releases
	#[pallet::storage]
	pub(super) type PendingBaseFeeMaturationByTick<T: Config> = StorageMap<
		_,
		Twox64Concat,
		Tick,
		BoundedVec<(T::AccountId, T::Balance, VaultId, ObligationId), ConstU32<1000>>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub(super) type NextObligationId<T: Config> = StorageValue<_, ObligationId, OptionQuery>;

	/// Obligation  by id
	#[pallet::storage]
	pub(super) type ObligationsById<T: Config> = StorageMap<
		_,
		Twox64Concat,
		ObligationId,
		Obligation<T::AccountId, T::Balance>,
		OptionQuery,
	>;
	/// Completion of bonded bitcoin obligation, upon which funds are returned to the vault
	#[pallet::storage]
	pub(super) type BondedBitcoinCompletions<T: Config> = StorageMap<
		_,
		Twox64Concat,
		Tick,
		BoundedVec<ObligationId, T::MaxConcurrentlyExpiringObligations>,
		ValueQuery,
	>;

	/// Completion of bitcoin locks by bitcoin height. Funds are returned to the vault if
	/// unlocked or used as the price of the bitcoin
	#[pallet::storage]
	pub(super) type BitcoinLockCompletions<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinHeight,
		BoundedVec<ObligationId, T::MaxConcurrentlyExpiringObligations>,
		ValueQuery,
	>;

	/// The entrants in the bonded bitcoin pool that will next be paid out. They apply to the next
	/// closed mining slot cohort bid pool.
	#[pallet::storage]
	pub(super) type NextBondedBitcoinPoolEntrants<T: Config> = StorageValue<
		_,
		BoundedVec<BidPoolEntrant<T>, T::MaxConcurrentlyExpiringObligations>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		VaultCreated {
			vault_id: VaultId,
			locked_bitcoin_argons: T::Balance,
			bonded_bitcoin_argons: T::Balance,
			added_securitization_percent: FixedU128,
			operator_account_id: T::AccountId,
			activation_tick: Tick,
		},
		VaultModified {
			vault_id: VaultId,
			locked_bitcoin_argons: T::Balance,
			bonded_bitcoin_argons: T::Balance,
			added_securitization_percent: FixedU128,
		},
		VaultTermsChangeScheduled {
			vault_id: VaultId,
			change_tick: Tick,
		},
		VaultTermsChanged {
			vault_id: VaultId,
		},
		VaultClosed {
			vault_id: VaultId,
			locked_bitcoin_amount_still_reserved: T::Balance,
			bonded_bitcoin_amount_still_reserved: T::Balance,
			securitization_still_reserved: T::Balance,
		},
		VaultBitcoinXpubChange {
			vault_id: VaultId,
		},
		ObligationCreated {
			vault_id: VaultId,
			obligation_id: ObligationId,
			fund_type: FundType,
			beneficiary: T::AccountId,
			amount: T::Balance,
			expiration: ObligationExpiration,
		},
		ObligationCompleted {
			vault_id: VaultId,
			obligation_id: ObligationId,
			returned_fee: T::Balance,
			released_fee: T::Balance,
		},
		ObligationModified {
			vault_id: VaultId,
			obligation_id: ObligationId,
			amount: T::Balance,
		},
		ObligationCanceled {
			vault_id: VaultId,
			obligation_id: ObligationId,
			beneficiary: T::AccountId,
			fund_type: FundType,
			returned_fee: T::Balance,
		},
		/// An error occurred while completing an obligation
		ObligationCompletionError {
			obligation_id: ObligationId,
			error: DispatchError,
		},
		/// An error occurred releasing a base fee hold
		ObligationBaseFeeMaturationError {
			obligation_id: ObligationId,
			base_fee: T::Balance,
			vault_id: VaultId,
			error: DispatchError,
		},
		/// An error occurred distributing a bid pool
		CouldNotDistributeBidPool {
			cohort_id: CohortId,
			vault_id: VaultId,
			amount: T::Balance,
			dispatch_error: DispatchError,
		},
		/// An error occurred burning from the bid pool
		CouldNotBurnBidPool {
			cohort_id: CohortId,
			amount: T::Balance,
			dispatch_error: DispatchError,
		},
		/// An error occurred allocating the next bid pool
		CouldNotAllocateNextBidPool {
			cohort_id: CohortId,
			dispatch_error: DispatchError,
		},
		/// Funds fro the active bid pool have been distributed
		BidPoolDistributed {
			cohort_id: CohortId,
			bid_pool_distributed: T::Balance,
			bid_pool_burned: T::Balance,
			bid_pool_entrants: u32,
		},
		/// The next bid pool has been allocated
		NextBidPoolAllocated {
			bonded_bitcoin_pool: T::Balance,
			bid_pool_entrants: u32,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		ObligationNotFound,
		NoMoreVaultIds,
		NoMoreObligationIds,
		MinimumObligationAmountNotMet,
		/// There are too many obligations expiring in the given expiration block
		ExpirationAtBlockOverflow,
		InsufficientFunds,
		InsufficientVaultFunds,
		/// The proposed transaction would take the account below the minimum (existential) balance
		AccountBelowMinimumBalance,
		VaultClosed,
		/// Funding would result in an overflow of the balance type
		InvalidVaultAmount,
		/// This reduction in obligation funds offered goes below the amount that is already
		/// committed to
		VaultReductionBelowAllocatedFunds,
		/// An invalid securitization percent was provided for the vault. NOTE: it cannot be
		/// decreased (or negative)
		InvalidSecuritization,
		/// The vault bitcoin xpubkey has already been used
		ReusedVaultBitcoinXpub,
		/// The bitcoin script to lock this bitcoin has errors
		InvalidBitcoinScript,
		/// Unable to decode xpubkey
		InvalidXpubkey,
		/// Wrong Xpub Network
		WrongXpubNetwork,
		/// The XPub is unsafe to use in a public blockchain (aka, unhardened)
		UnsafeXpubkey,
		/// Unable to derive xpubkey child
		UnableToDeriveVaultXpubChild,
		/// Bitcoin conversion to compressed pubkey failed
		BitcoinConversionFailed,
		NoPermissions,
		HoldUnexpectedlyModified,
		UnrecoverableHold,
		VaultNotFound,
		/// The vault is not yet active
		VaultNotYetActive,
		/// No Vault public keys are available
		NoVaultBitcoinPubkeysAvailable,
		/// The terms modification list could not handle any more items
		TermsModificationOverflow,
		/// Terms are already scheduled to be changed
		TermsChangeAlreadyScheduled,
		/// An internal processing error occurred
		InternalError,
		/// Unable to generate a new vault bitcoin pubkey
		UnableToGenerateVaultBitcoinPubkey,
		/// A funding change is already scheduled
		FundingChangeAlreadyScheduled,
		/// An error occurred processing an obligation completion
		ObligationCompletionError,
		/// Too many base fee maturations were inserted per tick
		BaseFeeOverflow,
	}

	impl<T> From<ObligationError> for Error<T> {
		fn from(e: ObligationError) -> Error<T> {
			match e {
				ObligationError::ObligationNotFound => Error::<T>::ObligationNotFound,
				ObligationError::NoMoreObligationIds => Error::<T>::NoMoreObligationIds,
				ObligationError::MinimumObligationAmountNotMet =>
					Error::<T>::MinimumObligationAmountNotMet,
				ObligationError::ExpirationAtBlockOverflow => Error::<T>::ExpirationAtBlockOverflow,
				ObligationError::InsufficientFunds => Error::<T>::InsufficientFunds,
				ObligationError::HoldUnexpectedlyModified => Error::<T>::HoldUnexpectedlyModified,
				ObligationError::UnrecoverableHold => Error::<T>::UnrecoverableHold,
				ObligationError::VaultNotFound => Error::<T>::VaultNotFound,
				ObligationError::InsufficientVaultFunds => Error::<T>::InsufficientVaultFunds,
				ObligationError::VaultClosed => Error::<T>::VaultClosed,
				ObligationError::AccountWouldBeBelowMinimum =>
					Error::<T>::AccountBelowMinimumBalance,
				ObligationError::InvalidBitcoinScript => Error::<T>::InvalidBitcoinScript,
				ObligationError::NoVaultBitcoinPubkeysAvailable =>
					Error::<T>::NoVaultBitcoinPubkeysAvailable,
				ObligationError::InternalError => Error::<T>::InternalError,
				ObligationError::UnableToGenerateVaultBitcoinPubkey =>
					Error::<T>::UnableToGenerateVaultBitcoinPubkey,
				ObligationError::ObligationCompletionError => Error::<T>::ObligationCompletionError,
				ObligationError::VaultNotYetActive => Error::<T>::VaultNotYetActive,
				ObligationError::BaseFeeOverflow => Error::<T>::BaseFeeOverflow,
			}
		}
	}

	#[derive(
		Encode,
		Decode,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct VaultConfig<
		Balance: Codec + MaxEncodedLen + Clone + TypeInfo + PartialEq + Eq + Debug,
	> {
		/// Terms of this vault configuration
		pub terms: VaultTerms<Balance>,
		/// The amount of argons to be vaulted for bitcoin locks
		#[codec(compact)]
		pub locked_bitcoin_argons_allocated: Balance,
		/// Bytes for a hardened XPub. Will be used to generate child public keys
		pub bitcoin_xpubkey: OpaqueBitcoinXpub,
		/// The amount of argons to be vaulted for bonded bitcoins
		#[codec(compact)]
		pub bonded_bitcoin_argons_allocated: Balance,
		/// The additional/extra securitization percent for the vault (must be maintained going
		/// forward)
		#[codec(compact)]
		pub added_securitization_percent: FixedU128,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			let previous_tick = T::TickProvider::previous_tick();
			let current_tick = T::TickProvider::current_tick();
			let bonded_bitcoin_completions =
				(previous_tick..=current_tick).flat_map(BondedBitcoinCompletions::<T>::take);
			for obligation_id in bonded_bitcoin_completions {
				let res = with_storage_layer(|| Self::obligation_completed(obligation_id));
				if let Err(e) = res {
					log::error!(
						"Bonded bitcoin obligation id {:?} failed to `complete` {:?}",
						obligation_id,
						e
					);
					Self::deposit_event(Event::<T>::ObligationCompletionError {
						obligation_id,
						error: e,
					});
				}
			}

			let pending_base_hold_releases =
				(previous_tick..=current_tick).flat_map(PendingBaseFeeMaturationByTick::<T>::take);
			for (who, amount, vault_id, obligation_id) in pending_base_hold_releases {
				let res = with_storage_layer(|| {
					T::Currency::release(
						&HoldReason::ObligationFee.into(),
						&who,
						amount,
						Precision::Exact,
					)
				});
				if let Err(e) = res {
					log::error!(
						"Base fee failed to release for vault {:?} {:?}, {:?}",
						vault_id,
						amount,
						e
					);
					Self::deposit_event(Event::<T>::ObligationBaseFeeMaturationError {
						obligation_id,
						vault_id,
						base_fee: amount,
						error: e,
					});
				}
			}

			let (start_bitcoin_height, bitcoin_block_height) = T::BitcoinBlockHeightChange::get();
			let bitcoin_completions = (start_bitcoin_height..=bitcoin_block_height)
				.flat_map(BitcoinLockCompletions::<T>::take);
			for obligation_id in bitcoin_completions {
				let res = with_storage_layer(|| Self::obligation_completed(obligation_id));
				if let Err(e) = res {
					log::error!(
						"Bitcoin obligation id {:?} failed to `complete` {:?}",
						obligation_id,
						e
					);
					Self::deposit_event(Event::<T>::ObligationCompletionError {
						obligation_id,
						error: e,
					});
				}
			}

			T::DbWeight::get().reads_writes(0, 2)
		}

		fn on_finalize(_n: BlockNumberFor<T>) {
			let previous_tick = T::TickProvider::previous_tick();
			let current_tick = T::TickProvider::current_tick();
			let terms =
				(previous_tick..=current_tick).flat_map(PendingTermsModificationsByTick::<T>::take);
			for vault_id in terms {
				VaultsById::<T>::mutate(vault_id, |vault| {
					let Some(vault) = vault else {
						return;
					};
					if let Some((_, terms)) = vault.pending_terms.take() {
						vault.terms = terms;
						Self::deposit_event(Event::VaultTermsChanged { vault_id });
					}
				});
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn create(
			origin: OriginFor<T>,
			vault_config: VaultConfig<T::Balance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let VaultConfig {
				added_securitization_percent,
				terms,
				locked_bitcoin_argons_allocated,
				bonded_bitcoin_argons_allocated,
				bitcoin_xpubkey,
			} = vault_config;

			ensure!(
				locked_bitcoin_argons_allocated
					.checked_add(&bonded_bitcoin_argons_allocated)
					.is_some(),
				Error::<T>::InvalidVaultAmount
			);

			let xpub: BitcoinXPub = bitcoin_xpubkey.try_into().map_err(|e| {
				log::error!("Unable to decode xpubkey: {:?}", e);
				Error::<T>::InvalidXpubkey
			})?;
			ensure!(xpub.is_hardened(), Error::<T>::UnsafeXpubkey);
			ensure!(
				xpub.matches_network(T::GetBitcoinNetwork::get()),
				Error::<T>::WrongXpubNetwork
			);
			// make sure we can derive
			let _xpub =
				xpub.derive_pubkey(0).map_err(|_| Error::<T>::UnableToDeriveVaultXpubChild)?;

			let vault_id = NextVaultId::<T>::get().unwrap_or(1);
			let next_vault_id = vault_id.increment().ok_or(Error::<T>::NoMoreVaultIds)?;
			NextVaultId::<T>::set(Some(next_vault_id));

			let activation_tick = Self::get_terms_activation_tick();

			let mut vault = Vault {
				operator_account_id: who.clone(),
				locked_bitcoin_argons: VaultArgons {
					allocated: locked_bitcoin_argons_allocated,
					reserved: 0u32.into(),
				},
				terms,
				bonded_bitcoin_argons: VaultArgons {
					allocated: bonded_bitcoin_argons_allocated,
					reserved: 0u32.into(),
				},
				added_securitization_percent,
				activation_tick,
				added_securitization_argons: 0u32.into(),
				is_closed: false,
				pending_terms: None,
				pending_bitcoins: 0u32.into(),
			};
			vault.added_securitization_argons = vault.get_added_securitization_needed();
			VaultXPubById::<T>::insert(vault_id, (xpub, 0));

			Self::hold(
				&who,
				locked_bitcoin_argons_allocated +
					bonded_bitcoin_argons_allocated +
					vault.added_securitization_argons,
				HoldReason::EnterVault,
			)
			.map_err(Error::<T>::from)?;

			VaultsById::<T>::insert(vault_id, vault);
			Self::deposit_event(Event::VaultCreated {
				vault_id,
				locked_bitcoin_argons: locked_bitcoin_argons_allocated,
				bonded_bitcoin_argons: bonded_bitcoin_argons_allocated,
				added_securitization_percent,
				operator_account_id: who,
				activation_tick,
			});

			Ok(())
		}

		/// Modify funds offered by the vault. This will not affect issued obligations, but will
		/// affect the amount of funds available for new ones.
		///
		/// The additional securitization percent must be maintained or increased.
		///
		/// The amount offered may not go below the existing reserved amounts, but you can release
		/// funds in this vault as obligations are released. To stop issuing any more obligations,
		/// use the `close` api.
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn modify_funding(
			origin: OriginFor<T>,
			vault_id: VaultId,
			total_bonded_bitcoin_amount_offered: T::Balance,
			total_locked_bitcoin_amount_offered: T::Balance,
			added_securitization_percent: FixedU128,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut vault = VaultsById::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;
			// mutable because if it increases, we need to delay it to keep bidding markets fair.
			ensure!(vault.operator_account_id == who, Error::<T>::NoPermissions);

			let mut amount_to_hold: i128 = 0;
			// NOTE: We're not changing the amount of bonded bitcoins, so nothing needs to be
			// checked about the ratio of mining to bitcoin
			if vault.locked_bitcoin_argons.allocated != total_locked_bitcoin_amount_offered {
				ensure!(
					vault.locked_bitcoin_argons.reserved <= total_locked_bitcoin_amount_offered,
					Error::<T>::VaultReductionBelowAllocatedFunds
				);

				amount_to_hold += balance_to_i128::<T>(total_locked_bitcoin_amount_offered) -
					balance_to_i128::<T>(vault.locked_bitcoin_argons.allocated);
				vault.locked_bitcoin_argons.allocated = total_locked_bitcoin_amount_offered;
			}

			if vault.bonded_bitcoin_argons.allocated != total_bonded_bitcoin_amount_offered {
				ensure!(
					vault.bonded_bitcoin_argons.reserved <= total_bonded_bitcoin_amount_offered,
					Error::<T>::VaultReductionBelowAllocatedFunds
				);

				amount_to_hold += balance_to_i128::<T>(total_bonded_bitcoin_amount_offered) -
					balance_to_i128::<T>(vault.bonded_bitcoin_argons.allocated);
				vault.bonded_bitcoin_argons.allocated = total_bonded_bitcoin_amount_offered;
			}

			ensure!(
				added_securitization_percent >= vault.added_securitization_percent,
				Error::<T>::InvalidSecuritization
			);

			vault.added_securitization_percent = added_securitization_percent;

			let total_securities = vault.get_added_securitization_needed();

			amount_to_hold += balance_to_i128::<T>(total_securities) -
				balance_to_i128::<T>(vault.added_securitization_argons);
			vault.added_securitization_argons = total_securities;

			#[allow(clippy::comparison_chain)]
			if amount_to_hold > 0 {
				Self::hold(&who, (amount_to_hold as u128).into(), HoldReason::EnterVault)
					.map_err(Error::<T>::from)?;
			} else if amount_to_hold < 0 {
				Self::release_hold(
					&who,
					amount_to_hold.unsigned_abs().into(),
					HoldReason::EnterVault,
				)?;
			}

			Self::deposit_event(Event::VaultModified {
				vault_id,
				locked_bitcoin_argons: total_locked_bitcoin_amount_offered,
				bonded_bitcoin_argons: total_bonded_bitcoin_amount_offered,
				added_securitization_percent,
			});
			VaultsById::<T>::insert(vault_id, vault);

			Ok(())
		}

		/// Change the terms of this vault. The change will be applied at the next mining slot
		/// change that is at least `MinTermsModificationBlockDelay` blocks away.
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn modify_terms(
			origin: OriginFor<T>,
			vault_id: VaultId,
			terms: VaultTerms<T::Balance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut vault =
				VaultsById::<T>::get(vault_id).ok_or::<Error<T>>(Error::<T>::VaultNotFound)?;

			ensure!(vault.operator_account_id == who, Error::<T>::NoPermissions);
			ensure!(vault.pending_terms.is_none(), Error::<T>::TermsChangeAlreadyScheduled);

			let terms_change_tick = Self::get_terms_activation_tick();

			PendingTermsModificationsByTick::<T>::mutate(terms_change_tick, |a| {
				if !a.iter().any(|x| *x == vault_id) {
					return a.try_push(vault_id);
				}
				Ok(())
			})
			.map_err(|_| Error::<T>::TermsModificationOverflow)?;
			vault.pending_terms = Some((terms_change_tick, terms));
			VaultsById::<T>::insert(vault_id, vault);

			Self::deposit_event(Event::VaultTermsChangeScheduled {
				vault_id,
				change_tick: terms_change_tick,
			});

			Ok(())
		}

		/// Stop offering additional obligations from this vault. Will not affect existing
		/// obligations. As funds are returned, they will be released to the vault owner.
		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn close(origin: OriginFor<T>, vault_id: VaultId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut vault =
				VaultsById::<T>::get(vault_id).ok_or::<Error<T>>(Error::<T>::VaultNotFound)?;

			ensure!(vault.operator_account_id == who, Error::<T>::NoPermissions);

			vault.is_closed = true;

			let securitization_still_needed = vault.get_added_securitization_needed();
			let free_securitization =
				vault.added_securitization_argons.saturating_sub(securitization_still_needed);

			let return_amount = vault.locked_bitcoin_argons.free_balance() +
				vault.bonded_bitcoin_argons.free_balance() +
				free_securitization;

			ensure!(
				T::Currency::balance_on_hold(&HoldReason::EnterVault.into(), &who) >= return_amount,
				Error::<T>::HoldUnexpectedlyModified
			);

			Self::release_hold(&who, return_amount, HoldReason::EnterVault)?;

			vault.locked_bitcoin_argons.allocated = vault.locked_bitcoin_argons.reserved;
			vault.bonded_bitcoin_argons.allocated = vault.bonded_bitcoin_argons.reserved;
			vault.added_securitization_argons = securitization_still_needed;
			Self::deposit_event(Event::VaultClosed {
				vault_id,
				locked_bitcoin_amount_still_reserved: vault.locked_bitcoin_argons.reserved,
				bonded_bitcoin_amount_still_reserved: vault.bonded_bitcoin_argons.reserved,
				securitization_still_reserved: securitization_still_needed,
			});
			VaultsById::<T>::insert(vault_id, vault);

			Ok(())
		}

		/// Replace the bitcoin xpubkey for this vault. This will not affect existing obligations,
		/// but will be used for any obligations after this point. Will be rejected if already
		/// used.
		#[pallet::call_index(4)]
		#[pallet::weight(0)]
		pub fn replace_bitcoin_xpub(
			origin: OriginFor<T>,
			vault_id: VaultId,
			bitcoin_xpub: OpaqueBitcoinXpub,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let vault =
				VaultsById::<T>::get(vault_id).ok_or::<Error<T>>(Error::<T>::VaultNotFound)?;

			ensure!(vault.operator_account_id == who, Error::<T>::NoPermissions);

			let xpub =
				BitcoinXPub::try_from(bitcoin_xpub).map_err(|_| Error::<T>::InvalidXpubkey)?;
			if let Some(existing) = VaultXPubById::<T>::get(vault_id) {
				ensure!(existing.0 != xpub, Error::<T>::ReusedVaultBitcoinXpub);
			}
			ensure!(xpub.is_hardened(), Error::<T>::UnsafeXpubkey);
			ensure!(
				xpub.matches_network(T::GetBitcoinNetwork::get()),
				Error::<T>::WrongXpubNetwork
			);
			let _try_derive =
				xpub.derive_pubkey(0).map_err(|_| Error::<T>::UnableToDeriveVaultXpubChild)?;
			VaultXPubById::<T>::insert(vault_id, (xpub, 0));
			Self::deposit_event(Event::VaultBitcoinXpubChange { vault_id });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn get_terms_activation_tick() -> Tick {
			if T::MiningSlotProvider::is_slot_bidding_started() {
				return T::MiningSlotProvider::get_next_slot_tick();
			}
			T::TickProvider::current_tick()
		}

		fn hold(
			who: &T::AccountId,
			amount: T::Balance,
			reason: HoldReason,
		) -> Result<(), ObligationError> {
			if amount == T::Balance::zero() {
				return Ok(());
			}

			let needs_providers = T::Currency::balance_on_hold(&reason.into(), who) == 0u128.into();

			T::Currency::hold(&reason.into(), who, amount).map_err(|e| match e {
				Token(TokenError::BelowMinimum) => ObligationError::AccountWouldBeBelowMinimum,
				_ => {
					let balance = T::Currency::balance(who);
					if balance.checked_sub(&amount).is_some() &&
						balance.saturating_sub(amount) < T::Currency::minimum_balance()
					{
						return ObligationError::AccountWouldBeBelowMinimum;
					}

					ObligationError::InsufficientFunds
				},
			})?;
			if needs_providers {
				let _ = frame_system::Pallet::<T>::inc_providers(who);
			}
			Ok(())
		}

		fn release_hold(
			who: &T::AccountId,
			amount: T::Balance,
			reason: HoldReason,
		) -> Result<T::Balance, DispatchError> {
			if amount == T::Balance::zero() {
				return Ok(amount);
			}
			if amount == T::Currency::balance_on_hold(&reason.into(), who) {
				let _ = frame_system::Pallet::<T>::dec_providers(who);
			}
			T::Currency::release(&reason.into(), who, amount, Precision::Exact)
		}

		pub(crate) fn calculate_tick_fees(
			annual_percentage_rate: FixedU128,
			amount: T::Balance,
			ticks: Tick,
		) -> T::Balance {
			let ticks_per_day = FixedU128::saturating_from_integer(T::TicksPerDay::get());

			let ticks_per_year = ticks_per_day * FixedU128::saturating_from_integer(365);
			let ticks = FixedU128::saturating_from_integer(ticks);

			let ratio = ticks.checked_div(&ticks_per_year).unwrap_or_default();

			let amount = FixedU128::saturating_from_integer(amount);

			let fee =
				amount.saturating_mul(annual_percentage_rate).saturating_mul(ratio).into_inner() /
					FixedU128::accuracy();
			fee.unique_saturated_into()
		}

		/// Return bonded funds to the vault and complete the obligation
		fn obligation_completed(obligation_id: ObligationId) -> DispatchResult {
			let obligation =
				ObligationsById::<T>::get(obligation_id).ok_or(Error::<T>::ObligationNotFound)?;
			Self::remove_obligation_completion(obligation_id, obligation.expiration.clone());

			T::EventHandler::on_completed(&obligation)?;
			// reload obligation
			let obligation =
				ObligationsById::<T>::take(obligation_id).ok_or(Error::<T>::ObligationNotFound)?;
			let result = Self::release_reserved_funds(&obligation).map_err(Error::<T>::from)?;
			Self::deposit_event(Event::ObligationCompleted {
				vault_id: obligation.vault_id,
				obligation_id,
				returned_fee: result.returned_to_beneficiary,
				released_fee: result.paid_to_vault,
			});
			Ok(())
		}

		fn remove_obligation_completion(
			obligation_id: ObligationId,
			expiration: ObligationExpiration,
		) {
			match expiration {
				ObligationExpiration::BitcoinBlock(completion_block) => {
					if !BitcoinLockCompletions::<T>::contains_key(completion_block) {
						return;
					}
					BitcoinLockCompletions::<T>::mutate(completion_block, |obligations| {
						if let Some(index) = obligations.iter().position(|b| *b == obligation_id) {
							obligations.remove(index);
						}
					});
				},
				ObligationExpiration::AtTick(completion_tick) => {
					if !BondedBitcoinCompletions::<T>::contains_key(completion_tick) {
						return;
					}
					BondedBitcoinCompletions::<T>::mutate(completion_tick, |obligations| {
						if let Some(index) = obligations.iter().position(|b| *b == obligation_id) {
							obligations.remove(index);
						}
					});
				},
			}
		}

		fn release_reserved_funds(
			obligation: &Obligation<T::AccountId, T::Balance>,
		) -> Result<ReleaseFundsResult<T::Balance>, ObligationError> {
			let vault_id = obligation.vault_id;
			let vault = {
				let mut vault =
					VaultsById::<T>::get(vault_id).ok_or(ObligationError::VaultNotFound)?;
				vault.mut_argons(&obligation.fund_type).reduce_reserved(obligation.amount);
				vault
			};

			// after reducing the bonded, we can check the minimum securitization needed (can't be
			// mut)
			let minimum_securitization = vault.get_added_securitization_needed();
			// working around borrow checker
			let mut vault = vault;
			if vault.is_closed {
				let free_securitization =
					vault.added_securitization_argons.saturating_sub(minimum_securitization);

				Self::release_hold(
					&vault.operator_account_id,
					obligation.amount.saturating_add(free_securitization),
					HoldReason::EnterVault,
				)
				.map_err(|_| ObligationError::UnrecoverableHold)?;

				vault.added_securitization_argons = minimum_securitization;
				vault.mut_argons(&obligation.fund_type).reduce_allocated(obligation.amount);
			}

			let apr = vault.terms.bitcoin_annual_percent_rate;

			let current_tick = T::TickProvider::current_tick();
			let ticks = current_tick.saturating_sub(obligation.start_tick);
			let remaining_fee = Self::calculate_tick_fees(apr, obligation.amount, ticks);
			if remaining_fee > 0u128.into() {
				T::Currency::transfer_on_hold(
					&HoldReason::ObligationFee.into(),
					&obligation.beneficiary,
					&vault.operator_account_id,
					remaining_fee,
					Precision::Exact,
					Restriction::Free,
					Fortitude::Force,
				)
				.map_err(|_| ObligationError::UnrecoverableHold)?;
			}
			let amount_on_hold = obligation.total_fee.saturating_sub(obligation.prepaid_fee);
			let to_return = amount_on_hold.saturating_sub(remaining_fee);

			if to_return > 0u128.into() {
				Self::release_hold(&obligation.beneficiary, to_return, HoldReason::ObligationFee)
					.map_err(|_| ObligationError::UnrecoverableHold)?;
			}

			VaultsById::<T>::insert(vault_id, vault);
			Ok(ReleaseFundsResult {
				returned_to_beneficiary: to_return,
				paid_to_vault: remaining_fee,
			})
		}

		fn get_obligation_id_and_increment() -> Result<ObligationId, ObligationError> {
			let obligation_id = NextObligationId::<T>::get().unwrap_or(1);
			let next_obligation_id =
				obligation_id.increment().ok_or(ObligationError::NoMoreObligationIds)?;
			NextObligationId::<T>::set(Some(next_obligation_id));
			Ok(obligation_id)
		}
	}

	impl<T: Config> BitcoinObligationProvider for Pallet<T> {
		type Balance = T::Balance;
		type AccountId = T::AccountId;

		fn is_owner(vault_id: VaultId, account_id: &T::AccountId) -> bool {
			if let Some(vault) = VaultsById::<T>::get(vault_id) {
				return &vault.operator_account_id == account_id;
			}
			false
		}

		fn burn_vault_bitcoin_obligation(
			obligation_id: ObligationId,
			amount_to_burn: T::Balance,
		) -> Result<Obligation<T::AccountId, T::Balance>, ObligationError> {
			let mut obligation = ObligationsById::<T>::get(obligation_id)
				.ok_or(ObligationError::ObligationNotFound)?;
			let vault_id = obligation.vault_id;
			let mut vault = VaultsById::<T>::get(vault_id).ok_or(ObligationError::VaultNotFound)?;

			vault.locked_bitcoin_argons.destroy_funds(amount_to_burn)?;
			obligation.amount.saturating_reduce(amount_to_burn);

			T::Currency::burn_held(
				&HoldReason::EnterVault.into(),
				&vault.operator_account_id,
				amount_to_burn,
				Precision::Exact,
				Fortitude::Force,
			)
			.map_err(|_| ObligationError::UnrecoverableHold)?;

			VaultsById::<T>::insert(vault_id, vault);
			ObligationsById::<T>::insert(obligation_id, obligation.clone());

			Ok(obligation)
		}

		/// Recoup funds from the vault. This will be called if a vault has performed an illegal
		/// activity, like not moving cosigned UTXOs in the appropriate timeframe.
		///
		/// The recouped funds are market rate capped at securitization rate of the vault.
		///
		/// This will take funds from the vault in the following order:
		/// 1. From the bonded funds
		/// 2. From the allocated funds
		/// 3. From the securitized funds
		/// 4. TODO: From the ownership tokens
		///
		/// The funds will be returned to the owed_to_account_id
		///
		/// Returns the amount (still owed, repaid)
		fn compensate_lost_bitcoin(
			obligation_id: ObligationId,
			market_rate: Self::Balance,
			redemption_rate: Self::Balance,
		) -> Result<(Self::Balance, Self::Balance), ObligationError> {
			let zero = T::Balance::zero();
			let obligation = ObligationsById::<T>::get(obligation_id)
				.ok_or(ObligationError::ObligationNotFound)?;

			let vault_id = obligation.vault_id;
			let beneficiary = &obligation.beneficiary;
			let remaining_fee = obligation.total_fee.saturating_sub(obligation.prepaid_fee);
			let original_bonded_amount = obligation.amount;

			// the remaining fee is not paid
			if remaining_fee > 0u128.into() {
				Self::release_hold(beneficiary, remaining_fee, HoldReason::ObligationFee)
					.map_err(|_| ObligationError::UnrecoverableHold)?;
			}

			// 1. burn redemption rate from the vault (or min of market rate)
			let burn_amount = redemption_rate.min(market_rate);
			Self::burn_vault_bitcoin_obligation(obligation.obligation_id, burn_amount)?;

			// don't load until we've already burned
			let mut vault = VaultsById::<T>::get(vault_id).ok_or(ObligationError::VaultNotFound)?;
			let vault_operator = vault.operator_account_id.clone();

			// the max amount to recoup, which is the market rate capped by securitization
			let securitized_bond_amount = vault
				.added_securitization_percent
				.saturating_mul_int(original_bonded_amount)
				.saturating_add(original_bonded_amount)
				.min(market_rate);

			// Still owed is diff of securitized obligation amount and bonded amount
			let amount_owed = securitized_bond_amount.saturating_sub(original_bonded_amount);
			let mut still_owed = amount_owed;

			// 2: use bitcoin argons
			if still_owed > zero && vault.locked_bitcoin_argons.free_balance() >= zero {
				let amount_to_pull = still_owed.min(vault.locked_bitcoin_argons.free_balance());
				vault.locked_bitcoin_argons.destroy_allocated_funds(amount_to_pull)?;
				still_owed = still_owed
					.checked_sub(&amount_to_pull)
					.ok_or(ObligationError::InternalError)?;
			}

			// 3. Use securitized argons
			if still_owed > zero && vault.added_securitization_argons >= zero {
				let amount_to_pull = still_owed.min(vault.added_securitization_argons);
				vault.added_securitization_argons = vault
					.added_securitization_argons
					.checked_sub(&amount_to_pull)
					.ok_or(ObligationError::InternalError)?;
				still_owed = still_owed
					.checked_sub(&amount_to_pull)
					.ok_or(ObligationError::InternalError)?;
			}

			// TODO: 4. Use ownership tokens at current value

			let recouped = amount_owed.saturating_sub(still_owed);
			T::Currency::transfer_on_hold(
				&HoldReason::EnterVault.into(),
				&vault_operator,
				beneficiary,
				recouped,
				Precision::Exact,
				Restriction::Free,
				Fortitude::Force,
			)
			.map_err(|_| ObligationError::UnrecoverableHold)?;

			VaultsById::<T>::insert(vault_id, vault);

			Ok((still_owed, recouped))
		}

		fn create_utxo_script_pubkey(
			vault_id: VaultId,
			owner_pubkey: CompressedBitcoinPubkey,
			vault_claim_height: BitcoinHeight,
			open_claim_height: BitcoinHeight,
			current_height: BitcoinHeight,
		) -> Result<(BitcoinXPub, BitcoinXPub, BitcoinCosignScriptPubkey), ObligationError> {
			let (vault_xpubkey, vault_claim_pubkey) = VaultXPubById::<T>::mutate(vault_id, |a| {
				let (xpub, counter) =
					a.as_mut().ok_or(ObligationError::NoVaultBitcoinPubkeysAvailable)?;

				let mut next = counter
					.checked_add(1)
					.ok_or(ObligationError::UnableToGenerateVaultBitcoinPubkey)?;
				let pubkey = xpub
					.derive_pubkey(next)
					.map_err(|_| ObligationError::UnableToGenerateVaultBitcoinPubkey)?;

				next = next
					.checked_add(1)
					.ok_or(ObligationError::UnableToGenerateVaultBitcoinPubkey)?;
				let pubkey2 = xpub
					.derive_pubkey(next)
					.map_err(|_| ObligationError::UnableToGenerateVaultBitcoinPubkey)?;

				*a = Some((xpub.clone(), next));
				Ok((pubkey, pubkey2))
			})?;

			let script_args = CosignScriptArgs {
				vault_pubkey: vault_xpubkey.public_key,
				vault_claim_pubkey: vault_claim_pubkey.public_key,
				owner_pubkey,
				vault_claim_height,
				open_claim_height,
				created_at_height: current_height,
			};

			let network = T::GetBitcoinNetwork::get();
			let script_pubkey = CosignScript::new(script_args, network.into())
				.map_err(|_| ObligationError::InvalidBitcoinScript)?;

			Ok((
				vault_xpubkey,
				vault_claim_pubkey,
				script_pubkey
					.script
					.to_p2wsh()
					.try_into()
					.map_err(|_| ObligationError::InvalidBitcoinScript)?,
			))
		}

		fn modify_pending_bitcoin_funds(
			vault_id: VaultId,
			amount: Self::Balance,
			remove_pending: bool,
		) -> Result<(), ObligationError> {
			VaultsById::<T>::try_mutate(vault_id, |vault| {
				let vault = vault.as_mut().ok_or(ObligationError::VaultNotFound)?;
				vault.pending_bitcoins = if remove_pending {
					vault.pending_bitcoins.saturating_sub(amount)
				} else {
					vault.pending_bitcoins.saturating_add(amount)
				};
				Ok(())
			})
		}

		fn create_obligation(
			vault_id: VaultId,
			account_id: &T::AccountId,
			amount: T::Balance,
			expiration_block: BitcoinHeight,
			ticks: Tick,
		) -> Result<Obligation<T::AccountId, T::Balance>, ObligationError> {
			let obligation_id = Self::get_obligation_id_and_increment()?;

			ensure!(
				amount >= T::MinimumObligationAmount::get(),
				ObligationError::MinimumObligationAmountNotMet
			);
			let mut vault = VaultsById::<T>::get(vault_id)
				.ok_or::<ObligationError>(ObligationError::VaultNotFound)?;

			ensure!(
				vault.activation_tick <= T::TickProvider::current_tick(),
				ObligationError::VaultNotYetActive
			);

			ensure!(!vault.is_closed, ObligationError::VaultClosed);

			ensure!(
				vault.locked_bitcoin_argons.free_balance() >= amount,
				ObligationError::InsufficientVaultFunds
			);
			let vault_argons = &mut vault.locked_bitcoin_argons;

			let apr = vault.terms.bitcoin_annual_percent_rate;
			let base_fee = vault.terms.bitcoin_base_fee;

			let total_fee = Self::calculate_tick_fees(apr, amount, ticks).saturating_add(base_fee);
			log::trace!(
				"Vault {vault_id} trying to reserve {:?} for total_fees {:?}",
				amount,
				total_fee
			);

			T::Currency::transfer_and_hold(
				&HoldReason::ObligationFee.into(),
				account_id,
				&vault.operator_account_id,
				base_fee,
				Precision::Exact,
				Preservation::Preserve,
				Fortitude::Force,
			)
			.map_err(|e| match e {
				Token(TokenError::BelowMinimum) => ObligationError::AccountWouldBeBelowMinimum,
				_ => ObligationError::InsufficientFunds,
			})?;

			let base_fee_maturation =
				T::TickProvider::current_tick().saturating_add(T::BaseFeeMaturationTicks::get());
			PendingBaseFeeMaturationByTick::<T>::try_append(
				base_fee_maturation,
				(vault.operator_account_id.clone(), base_fee, vault_id, obligation_id),
			)
			.map_err(|_| ObligationError::BaseFeeOverflow)?;

			if total_fee > base_fee {
				Self::hold(account_id, total_fee - base_fee, HoldReason::ObligationFee)?;
			}

			vault_argons.reserved = vault_argons.reserved.saturating_add(amount);
			VaultsById::<T>::set(vault_id, Some(vault));

			let expiration = ObligationExpiration::BitcoinBlock(expiration_block);

			let obligation = Obligation {
				obligation_id,
				vault_id,
				fund_type: FundType::LockedBitcoin,
				beneficiary: account_id.clone(),
				amount,
				expiration: expiration.clone(),
				total_fee,
				start_tick: T::TickProvider::current_tick(),
				prepaid_fee: base_fee,
			};
			ObligationsById::<T>::set(obligation_id, Some(obligation.clone()));
			BitcoinLockCompletions::<T>::try_mutate(expiration_block, |a| {
				a.try_push(obligation_id)
					.map_err(|_| ObligationError::ExpirationAtBlockOverflow)
			})?;

			Self::deposit_event(Event::ObligationCreated {
				vault_id,
				obligation_id,
				beneficiary: account_id.clone(),
				amount,
				expiration,
				fund_type: FundType::LockedBitcoin,
			});
			Ok(obligation)
		}

		fn cancel_obligation(
			obligation_id: ObligationId,
		) -> Result<ReleaseFundsResult<T::Balance>, ObligationError> {
			let obligation = ObligationsById::<T>::take(obligation_id)
				.ok_or(ObligationError::ObligationNotFound)?;

			let release_funds_result = Self::release_reserved_funds(&obligation)?;

			Self::deposit_event(Event::ObligationCanceled {
				vault_id: obligation.vault_id,
				obligation_id,
				beneficiary: obligation.beneficiary.clone(),
				fund_type: obligation.fund_type.clone(),
				returned_fee: release_funds_result.returned_to_beneficiary,
			});
			Self::remove_obligation_completion(obligation_id, obligation.expiration.clone());
			T::EventHandler::on_canceled(&obligation)
				.map_err(|_| ObligationError::ObligationCompletionError)?;
			Ok(release_funds_result)
		}
	}

	impl<T: Config> BondedBitcoinsBidPoolProvider for Pallet<T> {
		type Balance = T::Balance;
		type AccountId = T::AccountId;

		fn get_bid_pool_account() -> Self::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		fn distribute_and_rotate_bid_pool(cohort_id: CohortId, mining_window_end_tick: Tick) {
			if cohort_id > 0 {
				Self::distribute_open_bid_pool(cohort_id - 1);
			}
			Self::allocate_next_bid_pool(cohort_id, mining_window_end_tick);
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn distribute_open_bid_pool(cohort_id: CohortId) {
			let bid_pool_account = Self::get_bid_pool_account();
			let mut bid_pool_amount = T::Currency::balance(&bid_pool_account);

			let burn_amount = T::BidPoolBurnPercent::get().mul_ceil(bid_pool_amount);
			if let Err(e) = T::Currency::burn_from(
				&bid_pool_account,
				burn_amount,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			) {
				Self::deposit_event(Event::<T>::CouldNotBurnBidPool {
					cohort_id,
					amount: burn_amount,
					dispatch_error: e,
				});
			}

			bid_pool_amount = bid_pool_amount.saturating_sub(burn_amount);
			let mut remaining = bid_pool_amount;
			let bid_pool_list = NextBondedBitcoinPoolEntrants::<T>::take();
			let bid_pool_entrants = bid_pool_list.len();

			for (i, entrant) in bid_pool_list.iter().enumerate() {
				let mut prorata = entrant.prorata.mul_floor(bid_pool_amount);
				remaining = remaining.saturating_sub(prorata);
				if i == bid_pool_list.len() - 1 && remaining > T::Balance::zero() {
					prorata = prorata.saturating_add(remaining);
					remaining = T::Balance::zero();
				}
				if let Err(e) = T::Currency::transfer(
					&bid_pool_account,
					&entrant.operator_account_id,
					prorata,
					Preservation::Expendable,
				) {
					Self::deposit_event(Event::<T>::CouldNotDistributeBidPool {
						cohort_id,
						vault_id: entrant.vault_id,
						amount: prorata,
						dispatch_error: e,
					});
				}
			}

			Self::deposit_event(Event::<T>::BidPoolDistributed {
				cohort_id,
				bid_pool_distributed: bid_pool_amount - remaining,
				bid_pool_burned: burn_amount,
				bid_pool_entrants: bid_pool_entrants as u32,
			});
		}

		pub(crate) fn allocate_next_bid_pool(cohort_id: CohortId, mining_window_end_tick: Tick) {
			let mut first_pass_bonded_bitcoins_pool = T::Balance::zero();
			let vaults = VaultsById::<T>::iter().collect::<Vec<_>>();
			let slots = 10u32;
			for (_, vault) in &vaults {
				if vault.is_closed {
					continue;
				}

				let allocated = vault.bonded_bitcoins_for_pool(slots);
				if let Some(next) = first_pass_bonded_bitcoins_pool.checked_add(&allocated) {
					first_pass_bonded_bitcoins_pool = next;
				} else {
					warn!("First pass bonded bitcoins pool overflowed");
					Self::deposit_event(Event::<T>::CouldNotAllocateNextBidPool {
						cohort_id,
						dispatch_error: ArithmeticError::Overflow.into(),
					});
				}
			}

			let mut entrants = Vec::new();
			let mut bid_pool_entrants = 0;
			let mut bonded_bitcoin_pool = T::Balance::zero();
			let bid_pool_account = Self::get_bid_pool_account();

			for (vault_id, vault) in vaults {
				if vault.is_closed {
					continue;
				}

				let bonded_bitcoins: T::Balance = vault.bonded_bitcoins_for_pool(slots);

				let prorata =
					Perbill::from_rational(bonded_bitcoins, first_pass_bonded_bitcoins_pool);
				if prorata < T::MinBidPoolProrataPercent::get() {
					continue;
				}
				match Self::allocate_bid_pool_entrant(
					&bid_pool_account,
					vault,
					vault_id,
					bonded_bitcoins,
					prorata,
					mining_window_end_tick,
				) {
					Ok(entrant) => {
						bonded_bitcoin_pool = bonded_bitcoin_pool.saturating_add(bonded_bitcoins);
						bid_pool_entrants += 1;
						entrants.push(entrant)
					},
					Err(e) => {
						Self::deposit_event(Event::<T>::CouldNotAllocateNextBidPool {
							cohort_id,
							dispatch_error: Into::<Error<T>>::into(e).into(),
						});
					},
				}
			}
			entrants.sort_by(|a, b| a.prorata.cmp(&b.prorata));
			NextBondedBitcoinPoolEntrants::<T>::put(BoundedVec::truncate_from(entrants));

			Self::deposit_event(Event::<T>::NextBidPoolAllocated {
				bonded_bitcoin_pool,
				bid_pool_entrants,
			});
		}

		pub(crate) fn allocate_bid_pool_entrant(
			pallet_bid_pool_account: &T::AccountId,
			mut vault: Vault<T::AccountId, T::Balance>,
			vault_id: VaultId,
			bonded_bitcoins: T::Balance,
			prorata: Perbill,
			mining_window_end_tick: Tick,
		) -> Result<BidPoolEntrant<T>, ObligationError> {
			let entrant = BidPoolEntrant {
				operator_account_id: vault.operator_account_id.clone(),
				vault_id,
				bonded_bitcoins,
				prorata,
			};

			vault.bonded_bitcoin_argons.reserved =
				vault.bonded_bitcoin_argons.reserved.saturating_add(bonded_bitcoins);
			VaultsById::<T>::set(vault_id, Some(vault));

			let obligation_id = Self::get_obligation_id_and_increment()?;
			let amount = bonded_bitcoins;
			let expiration = ObligationExpiration::AtTick(mining_window_end_tick);

			ObligationsById::<T>::insert(
				obligation_id,
				Obligation {
					obligation_id,
					vault_id,
					fund_type: FundType::BondedBitcoin,
					beneficiary: pallet_bid_pool_account.clone(),
					amount,
					expiration: expiration.clone(),
					total_fee: 0u128.into(),
					start_tick: T::TickProvider::current_tick(),
					prepaid_fee: 0u128.into(),
				},
			);
			BondedBitcoinCompletions::<T>::try_mutate(mining_window_end_tick, |a| {
				a.try_push(obligation_id)
					.map_err(|_| ObligationError::ExpirationAtBlockOverflow)
			})?;

			Self::deposit_event(Event::<T>::ObligationCreated {
				vault_id,
				obligation_id,
				beneficiary: pallet_bid_pool_account.clone(),
				amount,
				expiration,
				fund_type: FundType::BondedBitcoin,
			});

			Ok(entrant)
		}
	}

	fn balance_to_i128<T: Config>(balance: T::Balance) -> i128 {
		UniqueSaturatedInto::<u128>::unique_saturated_into(balance) as i128
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebugNoBound, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct BidPoolEntrant<T: Config> {
		pub operator_account_id: T::AccountId,
		#[codec(compact)]
		pub vault_id: VaultId,
		#[codec(compact)]
		pub bonded_bitcoins: T::Balance,
		pub prorata: Perbill,
	}
}
