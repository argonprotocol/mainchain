#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;
use pallet_prelude::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod migrations;
pub mod weights;

/// The Vaults pallet allows a user to fund BitcoinLocks for bitcoin holders. This allows them to
/// participate in liquidity pools. Vaults can define the number of Argons available for bitcoin
/// locks and the terms of both bitcoin locks and liquidity pools. However, bonded argons for
/// LiquidityPool may only be issued up to the amount of locked bitcoin.
///
/// ** Activated Securitization **
/// A vault can create liquidity pools up to 2x the locked securitization used for Bitcoin. This
/// added securitization is locked up for the duration of the bitcoin locks, and will be taken in
/// the case of bitcoins not being cosigned on release.
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::collections::BTreeSet;
	use argon_bitcoin::{CosignScript, CosignScriptArgs};
	use argon_primitives::{
		MiningSlotProvider, TickProvider,
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinNetwork, BitcoinXPub,
			CompressedBitcoinPubkey, OpaqueBitcoinXpub, Satoshis,
		},
		vault::{BitcoinVaultProvider, LiquidityPoolVaultProvider, Vault, VaultError, VaultTerms},
	};
	use frame_support::traits::Incrementable;
	use pallet_prelude::argon_primitives::vault::LockExtension;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(6);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;

		type Currency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ DecodeWithMemTracking
			+ core::fmt::Debug
			+ Default
			+ From<u128>
			+ TryInto<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		/// The max pending vault term changes per block
		#[pallet::constant]
		type MaxPendingTermModificationsPerTick: Get<u32>;

		/// A provider of mining slot information
		type MiningSlotProvider: MiningSlotProvider;

		/// Provides the bitcoin network this blockchain is connected to
		type GetBitcoinNetwork: Get<BitcoinNetwork>;
		/// Bitcoin time provider
		type BitcoinBlockHeightChange: Get<(BitcoinHeight, BitcoinHeight)>;

		type TickProvider: TickProvider<Self::Block>;

		type CurrentFrameId: Get<FrameId>;

		/// The max number of vaults that can be created
		#[pallet::constant]
		type MaxVaults: Get<u32>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		EnterVault,
		#[deprecated]
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

	/// The vaults that have funds releasing at a given bitcoin height
	#[pallet::storage]
	pub(super) type VaultFundsReleasingByHeight<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinHeight,
		BoundedBTreeSet<VaultId, T::MaxVaults>,
		ValueQuery,
	>;

	/// Tracks fee revenue from Bitcoin Locks for the last 10 Frames for each vault (a frame is a
	/// "mining day" in Argon). The total revenue for a vault includes Liquidity Pools, of which the
	/// associated data can be found in that pallet.
	#[pallet::storage]
	pub(super) type PerFrameFeeRevenueByVault<T: Config> = StorageMap<
		_,
		Twox64Concat,
		VaultId,
		BoundedVec<VaultFrameFeeRevenue<T>, ConstU32<10>>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		VaultCreated {
			vault_id: VaultId,
			securitization: T::Balance,
			securitization_ratio: FixedU128,
			operator_account_id: T::AccountId,
			opened_tick: Tick,
		},
		VaultModified {
			vault_id: VaultId,
			securitization: T::Balance,
			securitization_ratio: FixedU128,
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
			securitization_remaining: T::Balance,
			securitization_released: T::Balance,
		},
		VaultBitcoinXpubChange {
			vault_id: VaultId,
		},
		FundsLocked {
			vault_id: VaultId,
			locker: T::AccountId,
			amount: T::Balance,
			is_ratchet: bool,
		},
		FundLockCanceled {
			vault_id: VaultId,
			amount: T::Balance,
		},
		FundsScheduledForRelease {
			vault_id: VaultId,
			amount: T::Balance,
			release_height: BitcoinHeight,
		},
		LostBitcoinCompensated {
			vault_id: VaultId,
			beneficiary: T::AccountId,
			to_beneficiary: T::Balance,
			burned: T::Balance,
		},
		FundsReleased {
			vault_id: VaultId,
			amount: T::Balance,
		},
		FundsReleasedError {
			vault_id: VaultId,
			error: DispatchError,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Internally, the vault ids are maxed out
		NoMoreVaultIds,
		/// The user doesn't have enough funds to complete this request
		InsufficientFunds,
		/// There aren't enough funds in the vault to cover the requested bitcoin lock
		InsufficientVaultFunds,
		/// The proposed transaction would take the account below the minimum (existential) balance
		AccountBelowMinimumBalance,
		/// This vault is closed
		VaultClosed,
		/// Funding would result in an overflow of the balance type
		InvalidVaultAmount,
		/// This reduction in vault securitization goes below the amount already committed
		VaultReductionBelowSecuritization,
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
	}

	impl<T> From<VaultError> for Error<T> {
		fn from(e: VaultError) -> Error<T> {
			match e {
				VaultError::InsufficientFunds => Error::<T>::InsufficientFunds,
				VaultError::HoldUnexpectedlyModified => Error::<T>::HoldUnexpectedlyModified,
				VaultError::UnrecoverableHold => Error::<T>::UnrecoverableHold,
				VaultError::VaultNotFound => Error::<T>::VaultNotFound,
				VaultError::InsufficientVaultFunds => Error::<T>::InsufficientVaultFunds,
				VaultError::VaultClosed => Error::<T>::VaultClosed,
				VaultError::AccountWouldBeBelowMinimum => Error::<T>::AccountBelowMinimumBalance,
				VaultError::InvalidBitcoinScript => Error::<T>::InvalidBitcoinScript,
				VaultError::NoVaultBitcoinPubkeysAvailable =>
					Error::<T>::NoVaultBitcoinPubkeysAvailable,
				VaultError::InternalError => Error::<T>::InternalError,
				VaultError::UnableToGenerateVaultBitcoinPubkey =>
					Error::<T>::UnableToGenerateVaultBitcoinPubkey,
				VaultError::VaultNotYetActive => Error::<T>::VaultNotYetActive,
			}
		}
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct VaultConfig<Balance>
	where
		Balance: Codec + MaxEncodedLen + Clone + TypeInfo + PartialEq + Eq + Debug,
	{
		/// Terms of this vault configuration
		pub terms: VaultTerms<Balance>,
		/// The amount of argons to be vaulted for bitcoin locks
		#[codec(compact)]
		pub securitization: Balance,
		/// Bytes for a hardened XPub. Will be used to generate child public keys
		pub bitcoin_xpubkey: OpaqueBitcoinXpub,
		/// The securitization percent for the vault (must be maintained going forward)
		#[codec(compact)]
		pub securitization_ratio: FixedU128,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			let (start_bitcoin_height, bitcoin_block_height) = T::BitcoinBlockHeightChange::get();
			let bitcoin_completions = (start_bitcoin_height..=bitcoin_block_height)
				.flat_map(VaultFundsReleasingByHeight::<T>::take);
			for vault_id in bitcoin_completions {
				let res = with_storage_layer(|| {
					Self::release_funds(vault_id, bitcoin_block_height)
						.map_err(Error::<T>::from)
						.map_err(DispatchError::from)
				});
				if let Err(e) = res {
					log::error!("Vault `{}` unable to recoupd released funds {:?}", vault_id, e);
					Self::deposit_event(Event::<T>::FundsReleasedError { vault_id, error: e });
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
		#[pallet::weight(T::WeightInfo::create())]
		pub fn create(
			origin: OriginFor<T>,
			vault_config: VaultConfig<T::Balance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let VaultConfig { securitization_ratio, securitization, terms, bitcoin_xpubkey } =
				vault_config;

			ensure!(securitization_ratio >= FixedU128::one(), Error::<T>::InvalidSecuritization);

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

			let opened_tick = T::TickProvider::current_tick();

			ensure!(securitization > T::Balance::zero(), Error::<T>::InvalidVaultAmount);

			let vault = Vault {
				operator_account_id: who.clone(),
				securitization,
				argons_locked: 0u32.into(),
				terms,
				securitization_ratio,
				opened_tick,
				argons_scheduled_for_release: Default::default(),
				is_closed: false,
				pending_terms: None,
				argons_pending_activation: 0u32.into(),
			};
			VaultXPubById::<T>::insert(vault_id, (xpub, 0));

			Self::hold(&who, securitization, HoldReason::EnterVault).map_err(Error::<T>::from)?;

			VaultsById::<T>::insert(vault_id, vault);
			Self::deposit_event(Event::VaultCreated {
				vault_id,
				securitization,
				securitization_ratio,
				operator_account_id: who,
				opened_tick,
			});

			Ok(())
		}

		/// Modify funds allocated by the vault. This will not affect issued bitcoin locks, but will
		/// affect the amount of funds available for new ones.
		///
		/// The securitization percent must be maintained or increased.
		///
		/// The amount allocated may not go below the existing reserved amounts, but you can release
		/// funds in this vault as bitcoin locks are released. To stop issuing any more bitcoin
		/// locks, use the `close` api.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::modify_funding())]
		pub fn modify_funding(
			origin: OriginFor<T>,
			vault_id: VaultId,
			securitization: T::Balance,
			securitization_ratio: FixedU128,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut vault = VaultsById::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;
			// mutable because if it increases, we need to delay it to keep bidding markets fair.
			ensure!(vault.operator_account_id == who, Error::<T>::NoPermissions);

			ensure!(
				securitization_ratio >= vault.securitization_ratio,
				Error::<T>::InvalidSecuritization
			);

			let amount_to_hold =
				balance_to_i128::<T>(securitization) - balance_to_i128::<T>(vault.securitization);

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

			vault.securitization = securitization;
			vault.securitization_ratio = securitization_ratio;
			ensure!(
				securitization >= vault.get_minimum_securitization_needed(),
				Error::<T>::VaultReductionBelowSecuritization
			);

			Self::deposit_event(Event::VaultModified {
				vault_id,
				securitization,
				securitization_ratio,
			});
			VaultsById::<T>::insert(vault_id, vault);

			Ok(())
		}

		/// Change the terms of this vault. The change will be applied at the next mining slot
		/// change that is at least `MinTermsModificationBlockDelay` blocks away.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::modify_terms())]
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

			let terms_change_tick = Self::get_terms_active_tick();

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

		/// Stop offering additional bitcoin locks from this vault. Will not affect existing
		/// locks. As funds are returned, they will be released to the vault owner.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::close())]
		pub fn close(origin: OriginFor<T>, vault_id: VaultId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut vault =
				VaultsById::<T>::get(vault_id).ok_or::<Error<T>>(Error::<T>::VaultNotFound)?;

			ensure!(vault.operator_account_id == who, Error::<T>::NoPermissions);

			vault.is_closed = true;
			let start_securitization = vault.securitization;
			Self::shrink_vault_securitization(&mut vault).map_err(Error::<T>::from)?;
			let securitization_remaining = vault.securitization;

			Self::deposit_event(Event::VaultClosed {
				vault_id,
				securitization_remaining,
				securitization_released: start_securitization
					.saturating_sub(securitization_remaining),
			});
			VaultsById::<T>::insert(vault_id, vault);

			Ok(())
		}

		/// Replace the bitcoin xpubkey for this vault. This will not affect existing bitcoin locks,
		/// but will be used for any locks after this point. Will be rejected if already
		/// used.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::replace_bitcoin_xpub())]
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
		pub(crate) fn get_terms_active_tick() -> Tick {
			if T::MiningSlotProvider::is_slot_bidding_started() {
				return T::MiningSlotProvider::get_next_slot_tick();
			}
			T::TickProvider::current_tick()
		}

		fn hold(
			who: &T::AccountId,
			amount: T::Balance,
			reason: HoldReason,
		) -> Result<(), VaultError> {
			if amount == T::Balance::zero() {
				return Ok(());
			}

			if T::Currency::balance_on_hold(&reason.into(), who) == 0u128.into() {
				let _ = frame_system::Pallet::<T>::inc_providers(who);
			}

			T::Currency::hold(&reason.into(), who, amount).map_err(|e| match e {
				Token(TokenError::BelowMinimum) => VaultError::AccountWouldBeBelowMinimum,
				_ => {
					let balance = T::Currency::balance(who);
					if balance.checked_sub(&amount).is_some() &&
						balance.saturating_sub(amount) < T::Currency::minimum_balance()
					{
						return VaultError::AccountWouldBeBelowMinimum;
					}

					VaultError::InsufficientFunds
				},
			})?;
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
			let balance = T::Currency::release(&reason.into(), who, amount, Precision::Exact)
				.inspect_err(|e| {
					log::warn!(
						"Error releasing {:?} hold for {:?}. Amount {:?}. {:?}",
						reason,
						who,
						amount,
						e
					);
				})?;

			if T::Currency::balance_on_hold(&reason.into(), who) == 0u128.into() {
				let _ = frame_system::Pallet::<T>::dec_providers(who);
			}
			Ok(balance)
		}

		fn release_funds(vault_id: VaultId, block_height: BitcoinHeight) -> Result<(), VaultError> {
			let mut vault = VaultsById::<T>::get(vault_id).ok_or(VaultError::VaultNotFound)?;

			let swept = vault.sweep_released(block_height);
			Self::deposit_event(Event::FundsReleased { vault_id, amount: swept });
			if vault.is_closed {
				Self::shrink_vault_securitization(&mut vault)?;
			}
			VaultsById::<T>::insert(vault_id, vault);
			Ok(())
		}

		fn shrink_vault_securitization(
			vault: &mut Vault<T::AccountId, T::Balance>,
		) -> Result<(), VaultError> {
			let minimum_securitization = vault.get_minimum_securitization_needed();
			let free_securitization = vault.securitization.saturating_sub(minimum_securitization);

			vault.securitization = minimum_securitization;

			ensure!(
				T::Currency::balance_on_hold(
					&HoldReason::EnterVault.into(),
					&vault.operator_account_id
				) >= free_securitization,
				VaultError::HoldUnexpectedlyModified
			);

			Self::release_hold(
				&vault.operator_account_id,
				free_securitization,
				HoldReason::EnterVault,
			)
			.map_err(|_| VaultError::UnrecoverableHold)?;
			Ok(())
		}

		pub(crate) fn update_vault_metrics(
			metrics: VaultMetricUpdate<T>,
		) -> Result<(), VaultError> {
			let VaultMetricUpdate {
				vault_id,
				locks_created,
				total_fee,
				securitization_locked,
				satoshis_locked,
				satoshis_released,
				..
			} = metrics;
			let current_frame_id = T::CurrentFrameId::get();
			PerFrameFeeRevenueByVault::<T>::mutate(vault_id, |x| {
				let mut needs_insert = false;
				if let Some(existing) = x.first() {
					if existing.frame_id != current_frame_id {
						if x.is_full() {
							x.pop();
						}
						needs_insert = true;
					}
				} else {
					needs_insert = true;
				}
				if needs_insert {
					x.try_insert(
						0,
						VaultFrameFeeRevenue {
							frame_id: current_frame_id,
							fee_revenue: Zero::zero(),
							bitcoin_locks_created: 0,
							bitcoin_locks_total_satoshis: 0,
							bitcoin_locks_market_value: 0u32.into(),
							satoshis_released: 0,
						},
					)
					.map_err(|_| {
						tracing::error!("Unable to push new vault revenue");
						VaultError::InternalError
					})?;
				}
				if let Some(existing) = x.get_mut(0) {
					existing.fee_revenue.saturating_accrue(total_fee);
					existing.bitcoin_locks_created += locks_created;
					existing.bitcoin_locks_total_satoshis.saturating_accrue(satoshis_locked);
					existing.bitcoin_locks_market_value.saturating_accrue(securitization_locked);
					existing.satoshis_released.saturating_accrue(satoshis_released);
				}
				Ok(())
			})
		}

		fn track_vault_release_schedule(
			vault_id: VaultId,
			vault: &mut Vault<T::AccountId, T::Balance>,
			release_heights: BTreeSet<BitcoinHeight>,
		) -> Result<(), VaultError> {
			let current_height = T::BitcoinBlockHeightChange::get().1;
			for height in release_heights {
				if height <= current_height {
					continue;
				}
				VaultFundsReleasingByHeight::<T>::mutate(height, |x| x.try_insert(vault_id))
					.map_err(|_| VaultError::InternalError)?;
			}
			let swept = vault.sweep_released(current_height);
			if !swept.is_zero() {
				Self::deposit_event(Event::FundsReleased { vault_id, amount: swept });
			}
			Ok(())
		}
	}

	#[allow(dead_code)]
	pub(crate) struct VaultMetricUpdate<T: Config> {
		pub vault_id: VaultId,
		pub locks_created: u32,
		pub total_fee: T::Balance,
		pub securitization_locked: T::Balance,
		pub securitization_released: T::Balance,
		pub satoshis_locked: Satoshis,
		pub satoshis_released: Satoshis,
	}

	impl<T: Config> LiquidityPoolVaultProvider for Pallet<T> {
		type Balance = T::Balance;
		type AccountId = T::AccountId;

		fn get_activated_securitization(vault_id: VaultId) -> Self::Balance {
			VaultsById::<T>::get(vault_id)
				.map(|a| a.get_activated_securitization())
				.unwrap_or_default()
		}

		fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId> {
			VaultsById::<T>::get(vault_id).map(|a| a.operator_account_id)
		}

		fn get_vault_profit_sharing_percent(vault_id: VaultId) -> Option<Permill> {
			VaultsById::<T>::get(vault_id).map(|a| a.terms.liquidity_pool_profit_sharing)
		}

		fn is_vault_open(vault_id: VaultId) -> bool {
			VaultsById::<T>::get(vault_id).map(|a| !a.is_closed).unwrap_or_default()
		}
	}

	impl<T: Config> BitcoinVaultProvider for Pallet<T> {
		type Balance = T::Balance;
		type AccountId = T::AccountId;

		fn is_owner(vault_id: VaultId, account_id: &T::AccountId) -> bool {
			if let Some(vault) = VaultsById::<T>::get(vault_id) {
				return &vault.operator_account_id == account_id;
			}
			false
		}

		fn lock(
			vault_id: VaultId,
			account_id: &T::AccountId,
			lock_price: T::Balance,
			satoshis: Satoshis,
			extension: Option<(FixedU128, &mut LockExtension<T::Balance>)>,
		) -> Result<T::Balance, VaultError> {
			let mut vault =
				VaultsById::<T>::get(vault_id).ok_or::<VaultError>(VaultError::VaultNotFound)?;

			ensure!(
				vault.opened_tick <= T::TickProvider::current_tick(),
				VaultError::VaultNotYetActive
			);

			ensure!(!vault.is_closed, VaultError::VaultClosed);

			ensure!(vault.available_for_lock() >= lock_price, VaultError::InsufficientVaultFunds);

			let apr = vault.terms.bitcoin_annual_percent_rate;
			let base_fee = vault.terms.bitcoin_base_fee;
			let is_ratchet = extension.is_some();
			let term = extension.as_ref().map(|(term, _)| *term).unwrap_or(FixedU128::one());

			let total_fee =
				apr.saturating_mul(term).saturating_mul_int(lock_price).saturating_add(base_fee);
			log::trace!(
				"Vault {vault_id} trying to reserve {:?} for total_fees {:?}",
				lock_price,
				total_fee
			);

			Self::update_vault_metrics(VaultMetricUpdate {
				vault_id,
				locks_created: if is_ratchet { 0 } else { 1 },
				total_fee,
				securitization_locked: lock_price,
				securitization_released: 0u32.into(),
				satoshis_locked: satoshis,
				satoshis_released: 0,
			})?;

			// do this second so the 'provider' is already on the account
			T::Currency::transfer(
				account_id,
				&vault.operator_account_id,
				total_fee,
				Preservation::Expendable,
			)
			.map_err(|e| match e {
				Token(TokenError::BelowMinimum) => VaultError::AccountWouldBeBelowMinimum,
				_ => VaultError::InsufficientFunds,
			})?;

			if let Some((_, extension)) = extension {
				// locks must be held for a minimum of a year, so when we are looking to re-use
				// locked funds, they must be getting a new expiration of > 1 year from their
				// original date
				vault.extend_lock(lock_price, extension)?;
			} else {
				vault.lock(lock_price)?;
			}

			Self::deposit_event(Event::FundsLocked {
				vault_id,
				locker: account_id.clone(),
				amount: lock_price,
				is_ratchet,
			});
			VaultsById::<T>::insert(vault_id, vault);
			Ok(total_fee)
		}

		fn schedule_for_release(
			vault_id: VaultId,
			locked_argons: T::Balance,
			satoshis: Satoshis,
			lock_extension: &LockExtension<T::Balance>,
		) -> Result<(), VaultError> {
			Self::update_vault_metrics(VaultMetricUpdate {
				vault_id,
				locks_created: 0,
				total_fee: 0u32.into(),
				securitization_locked: 0u32.into(),
				securitization_released: locked_argons,
				satoshis_locked: 0,
				satoshis_released: satoshis,
			})?;

			let mut vault = VaultsById::<T>::get(vault_id).ok_or(VaultError::VaultNotFound)?;
			let release_heights = vault.schedule_for_release(locked_argons, lock_extension)?;
			Self::track_vault_release_schedule(vault_id, &mut vault, release_heights)?;
			VaultsById::<T>::insert(vault_id, vault);
			Self::deposit_event(Event::FundsScheduledForRelease {
				vault_id,
				amount: locked_argons,
				release_height: lock_extension.lock_expiration,
			});

			Ok(())
		}

		/// Recoup funds from the vault. This will be called if a vault has performed an illegal
		/// activity, like not moving cosigned UTXOs in the appropriate timeframe.
		///
		/// The compensation is up to the market rate but capped at the securitization rate of the
		/// vault.
		///
		/// Returns the amount sent to the beneficiary.
		fn compensate_lost_bitcoin(
			vault_id: VaultId,
			beneficiary: &T::AccountId,
			original_lock_amount: Self::Balance,
			market_rate: Self::Balance,
			lock_extension: &LockExtension<T::Balance>,
		) -> Result<Self::Balance, VaultError> {
			let mut vault = VaultsById::<T>::get(vault_id).ok_or(VaultError::VaultNotFound)?;

			let burn_result = vault.burn(original_lock_amount, market_rate, lock_extension)?;

			let securitized_amount = burn_result.burned_amount;
			Self::track_vault_release_schedule(vault_id, &mut vault, burn_result.release_heights)?;

			let to_beneficiary = securitized_amount.saturating_sub(original_lock_amount);
			if !to_beneficiary.is_zero() {
				T::Currency::transfer_on_hold(
					&HoldReason::EnterVault.into(),
					&vault.operator_account_id,
					beneficiary,
					to_beneficiary,
					Precision::Exact,
					Restriction::Free,
					Fortitude::Force,
				)
				.map_err(|_| VaultError::UnrecoverableHold)?;
			}
			let to_burn = securitized_amount.saturating_sub(to_beneficiary);
			if !to_burn.is_zero() {
				T::Currency::burn_held(
					&HoldReason::EnterVault.into(),
					&vault.operator_account_id,
					to_burn,
					Precision::Exact,
					Fortitude::Force,
				)
				.map_err(|_| VaultError::UnrecoverableHold)?;
			}

			Self::deposit_event(Event::LostBitcoinCompensated {
				vault_id,
				beneficiary: beneficiary.clone(),
				to_beneficiary,
				burned: to_burn,
			});
			VaultsById::<T>::insert(vault_id, vault);

			Ok(to_beneficiary)
		}

		/// Burn the funds from the vault.
		fn burn(
			vault_id: VaultId,
			lock_amount: T::Balance,
			market_rate: T::Balance,
			lock_extension: &LockExtension<T::Balance>,
		) -> Result<T::Balance, VaultError> {
			let mut vault = VaultsById::<T>::get(vault_id).ok_or(VaultError::VaultNotFound)?;

			let burn_result = vault.burn(lock_amount, market_rate, lock_extension)?;

			let burn_amount = burn_result.burned_amount;
			Self::track_vault_release_schedule(vault_id, &mut vault, burn_result.release_heights)?;

			T::Currency::burn_held(
				&HoldReason::EnterVault.into(),
				&vault.operator_account_id,
				burn_amount,
				Precision::Exact,
				Fortitude::Force,
			)
			.map_err(|_| VaultError::UnrecoverableHold)?;

			VaultsById::<T>::insert(vault_id, vault);

			Ok(burn_amount)
		}

		fn create_utxo_script_pubkey(
			vault_id: VaultId,
			owner_pubkey: CompressedBitcoinPubkey,
			vault_claim_height: BitcoinHeight,
			open_claim_height: BitcoinHeight,
			current_height: BitcoinHeight,
		) -> Result<(BitcoinXPub, BitcoinXPub, BitcoinCosignScriptPubkey), VaultError> {
			let (vault_xpubkey, vault_claim_pubkey) = VaultXPubById::<T>::mutate(vault_id, |a| {
				let (xpub, counter) =
					a.as_mut().ok_or(VaultError::NoVaultBitcoinPubkeysAvailable)?;

				let mut next =
					counter.checked_add(1).ok_or(VaultError::UnableToGenerateVaultBitcoinPubkey)?;
				let pubkey = xpub
					.derive_pubkey(next)
					.map_err(|_| VaultError::UnableToGenerateVaultBitcoinPubkey)?;

				next = next.checked_add(1).ok_or(VaultError::UnableToGenerateVaultBitcoinPubkey)?;
				let pubkey2 = xpub
					.derive_pubkey(next)
					.map_err(|_| VaultError::UnableToGenerateVaultBitcoinPubkey)?;

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
				.map_err(|_| VaultError::InvalidBitcoinScript)?;

			Ok((
				vault_xpubkey,
				vault_claim_pubkey,
				script_pubkey
					.script
					.to_p2wsh()
					.try_into()
					.map_err(|_| VaultError::InvalidBitcoinScript)?,
			))
		}

		fn remove_pending(vault_id: VaultId, amount: Self::Balance) -> Result<(), VaultError> {
			VaultsById::<T>::try_mutate(vault_id, |vault| {
				let vault = vault.as_mut().ok_or(VaultError::VaultNotFound)?;
				vault.argons_pending_activation.saturating_reduce(amount);
				Ok(())
			})
		}

		fn cancel(vault_id: VaultId, amount: Self::Balance) -> Result<(), VaultError> {
			VaultsById::<T>::mutate(vault_id, |vault| {
				let vault = vault.as_mut().ok_or(VaultError::VaultNotFound)?;
				vault.release_locked_funds(amount);

				// after reducing the bonded, we can check the minimum securitization needed
				if vault.is_closed {
					Self::shrink_vault_securitization(vault)?;
				}
				Ok::<(), VaultError>(())
			})?;
			Self::deposit_event(Event::FundLockCanceled { vault_id, amount });

			Ok(())
		}
	}

	fn balance_to_i128<T: Config>(balance: T::Balance) -> i128 {
		UniqueSaturatedInto::<u128>::unique_saturated_into(balance) as i128
	}

	/// Tracks the fee revenue for a Vault for a single Frame (mining day). Includes the associated
	/// amount of bitcoin locked up in Satoshi and Argon values.
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
	#[scale_info(skip_type_params(T))]
	pub struct VaultFrameFeeRevenue<T: Config> {
		/// The frame id in question
		#[codec(compact)]
		pub frame_id: FrameId,
		/// The fee revenue for the value
		#[codec(compact)]
		pub fee_revenue: T::Balance,
		/// The number of bitcoin locks created
		#[codec(compact)]
		pub bitcoin_locks_created: u32,
		/// The argon market value of the locked satoshis
		#[codec(compact)]
		pub bitcoin_locks_market_value: T::Balance,
		/// The number of satoshis locked into the vault
		#[codec(compact)]
		pub bitcoin_locks_total_satoshis: Satoshis,
		/// The number of satoshis released during this period
		#[codec(compact)]
		pub satoshis_released: Satoshis,
	}
}
