#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{pallet_prelude::TypeInfo, traits::tokens::Preservation};
use sp_core::RuntimeDebug;

pub use pallet::*;
use ulx_primitives::bitcoin::Satoshis;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
const LOG_TARGET: &str = "runtime::bitcoin_mint";

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, Mutate},
			tokens::{Fortitude, Precision},
		},
	};
	use frame_system::pallet_prelude::*;
	use log::{info, warn};
	use sp_runtime::traits::{AtLeast32BitUnsigned, CheckedDiv, UniqueSaturatedInto, Zero};
	use sp_std::vec;

	use ulx_primitives::{
		bitcoin::{BitcoinUtxoId, H256Le},
		bond::{BondError, BondProvider},
		inherents::{BitcoinInherentData, BitcoinInherentError, BitcoinUtxoSync},
		ArgonPriceProvider, BitcoinPriceProvider, BurnEventHandler, MintCirculationProvider,
	};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Currency: Mutate<Self::AccountId, Balance = Self::Balance>;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ sp_std::fmt::Debug
			+ Default
			+ From<u128>
			+ TryInto<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		type UlixeeMintCirculation: MintCirculationProvider<Self::Balance>;

		type BondId: Parameter
			+ Copy
			+ AtLeast32BitUnsigned
			+ codec::FullCodec
			+ TypeInfo
			+ MaxEncodedLen
			+ MaybeSerializeDeserialize;

		/// The hold reason when reserving bonded funds
		type RuntimeHoldReason: From<HoldReason>;

		type BondProvider: BondProvider<
			Balance = Self::Balance,
			BondId = Self::BondId,
			AccountId = Self::AccountId,
			BlockNumber = BlockNumberFor<Self>,
		>;

		type BitcoinPriceProvider: BitcoinPriceProvider<Self::Balance>;

		type ArgonPriceProvider: ArgonPriceProvider;

		/// The minimum number of satoshis that can be submitted in a single transaction
		#[pallet::constant]
		type MinimumSatoshiAmount: Get<u64>;

		/// The required bond duration for a Bitcoin submission
		#[pallet::constant]
		type BondDurationBlocks: Get<BlockNumberFor<Self>>;

		/// The maximum number of UTXOs that can be waiting for minting
		#[pallet::constant]
		type MaxPendingMintUtxos: Get<u32>;

		/// The maximum number of UTXOs that can be tracked at a given time
		#[pallet::constant]
		type MaxTrackedUtxos: Get<u32>;
	}

	/// Locked Bitcoin UTXOs that have had ownership confirmed. If a Bitcoin UTXO is moved before
	/// the expiration block, the bond is burned and the UTXO is unlocked.
	#[pallet::storage]
	pub(super) type LockedUtxos<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BitcoinUtxoId,
		LockedUtxo<T::AccountId, T::BondId, T::Balance, BlockNumberFor<T>>,
		OptionQuery,
	>;

	/// All tracked bitcoin utxos
	#[pallet::storage]
	pub(super) type TrackedUtxos<T: Config> =
		StorageValue<_, BoundedBTreeSet<BitcoinUtxoId, T::MaxTrackedUtxos>, ValueQuery>;

	/// Bitcoin UTXOs that have been submitted for ownership confirmation
	#[pallet::storage]
	pub(super) type UtxosPendingConfirmation<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<
			BitcoinUtxoId,
			LockedUtxo<T::AccountId, T::BondId, T::Balance, BlockNumberFor<T>>,
			T::MaxPendingMintUtxos,
		>,
		ValueQuery,
	>;

	/// Bitcoin UTXOs that have been submitted for minting. This list is FIFO for minting whenever
	/// a) CPI >= 0 and
	/// b) the aggregate minted Bitcoins <= the aggregate minted Argons from Ulixee Shares
	#[pallet::storage]
	pub(super) type PendingMintUtxos<T: Config> = StorageValue<
		_,
		BoundedVec<(BitcoinUtxoId, T::AccountId, T::Balance), T::MaxPendingMintUtxos>,
		ValueQuery,
	>;

	/// Expiration blocks mapped to Bitcoin UTXOs
	#[pallet::storage]
	pub(super) type LockedUtxoExpirationBlocks<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<BitcoinUtxoId, T::MaxPendingMintUtxos>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub(super) type MintedArgons<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		UtxoLocked {
			utxo: BitcoinUtxoId,
			account_id: T::AccountId,
			bond_id: T::BondId,
			lock_price: T::Balance,
			expiration_block: BlockNumberFor<T>,
		},
		UtxoUnlocked {
			utxo: BitcoinUtxoId,
			account_id: T::AccountId,
			bond_id: T::BondId,
			lock_price: T::Balance,
		},
		UtxoApplicationRejected {
			utxo: BitcoinUtxoId,
			account_id: T::AccountId,
			bond_id: T::BondId,
			lock_price: T::Balance,
			expiration_block: BlockNumberFor<T>,
		},
		UtxoMovedWithBurn {
			utxo: BitcoinUtxoId,
			bond_id: T::BondId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// An invalid bond was submitted
		InvalidBondSubmitted,
		/// Insufficient bitcoin amount
		InsufficientBitcoinAmount,
		/// Not enough argons were bonded
		InsufficientBondAmount,
		/// The bond expires sooner than required
		PrematureBondExpiration,
		/// No prices are available to mint bitcoins
		NoBitcoinPricesAvailable,
		/// This bitcoin utxo is already locked
		BitcoinAlreadyLocked,
		/// No more slots available for bitcoin minting
		MaxPendingMintUtxosExceeded,
		/// Locked Utxo Not Found
		UtxoNotLocked,
		/// Redemptions not currently available
		RedemptionsUnavailable,
		// copied from bond
		BadState,
		BondNotFound,
		NoMoreBondIds,
		BondFundClosed,
		MinimumBondAmountNotMet,
		LeaseUntilBlockTooSoon,
		LeaseUntilPastFundExpiration,
		/// There are too many bond or bond funds expiring in the given expiration block
		ExpirationAtBlockOverflow,
		InsufficientFunds,
		InsufficientBondFunds,
		ExpirationTooSoon,
		NoPermissions,
		NoBondFundFound,
		HoldUnexpectedlyModified,
		BondFundMaximumBondsExceeded,
		UnrecoverableHold,
		BondFundNotFound,
		BondAlreadyClosed,
		BondAlreadyLocked,
		BondLockedCannotModify,
		/// The fee for this bond exceeds the amount of the bond, which is unsafe
		FeeExceedsBondAmount,
		AccountWouldBeBelowMinimum,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			// clear out expired utxos
			let current_block = <frame_system::Pallet<T>>::block_number();
			let expired_utxos = LockedUtxoExpirationBlocks::<T>::take(current_block);
			for utxo in expired_utxos {
				if let Some(entry) = LockedUtxos::<T>::take(utxo.clone()) {
					match T::BondProvider::unlock_bond(entry.bond_id) {
						Ok(_) => {
							TrackedUtxos::<T>::mutate(|a| a.remove(&utxo));
							Self::deposit_event(Event::UtxoUnlocked {
								utxo,
								account_id: entry.account_id,
								bond_id: entry.bond_id,
								lock_price: entry.lock_price,
							});
						},
						Err(e) => {
							warn!(
								target: LOG_TARGET,
								"Bitcoin UTXO {:?} failed to unlock utxo bond {:?}", utxo, e
							);
						},
					}
				}
			}

			let mut bitcoin_mint = MintedArgons::<T>::get();
			let ulixee_mint = T::UlixeeMintCirculation::get_mint_circulation();
			let mut available_to_mint = ulixee_mint - bitcoin_mint;

			// if current cpi is 0, we can't mint any argons
			if T::ArgonPriceProvider::get_argon_cpi_price().unwrap_or_default() <= 0 {
				available_to_mint = T::Balance::zero();
			}

			if available_to_mint > T::Balance::zero() {
				let updated = <PendingMintUtxos<T>>::get().try_mutate(|pending| {
					pending.retain_mut(|(utxo, account_id, remaining_account_mint)| {
						if available_to_mint == T::Balance::zero() {
							return true;
						}

						let amount_to_mint = if available_to_mint >= *remaining_account_mint {
							*remaining_account_mint
						} else {
							available_to_mint
						};

						match T::Currency::mint_into(account_id, amount_to_mint) {
							Ok(_) => {
								available_to_mint -= amount_to_mint;
								*remaining_account_mint -= amount_to_mint;
								bitcoin_mint += amount_to_mint;
							},
							Err(e) => {
								warn!(
									target: LOG_TARGET,
									"Failed to mint {:?} argons for bitcoin UTXO {:?}: {:?}", amount_to_mint, &utxo, e
								);
							},
						};
						*remaining_account_mint > T::Balance::zero()
					});
				});

				match updated {
					Some(pending_mint_utxos) => {
						<PendingMintUtxos<T>>::put(pending_mint_utxos);
					},
					None => {
						warn!(target: LOG_TARGET, "Failed to mint argons for bitcoin UTXOs");
					},
				}
			}
			MintedArgons::<T>::put(bitcoin_mint);

			T::DbWeight::get().reads_writes(1, 1)
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
				"Bitcoin UTXO sync submitted (moved: {:?}, confirmed {})", utxo_sync.moved.len(), utxo_sync.confirmed.len()
			);
			let mut utxo_pending_confirmation = <UtxosPendingConfirmation<T>>::get();
			let mut pending_mint_utxos = <PendingMintUtxos<T>>::get();
			for utxo in utxo_sync.confirmed.into_iter() {
				let Some(entry) = utxo_pending_confirmation.remove(&utxo) else {
					continue;
				};
				if pending_mint_utxos
					.try_push((utxo.clone(), entry.account_id.clone(), entry.lock_price))
					.is_err()
				{
					warn!(
						target: LOG_TARGET,
						"Failed to add bitcoin UTXO {:?} to pending list", utxo
					);
					continue;
				}

				<LockedUtxos<T>>::insert(
					utxo.clone(),
					LockedUtxo {
						account_id: entry.account_id.clone(),
						bond_id: entry.bond_id,
						satoshis: entry.satoshis,
						lock_price: entry.lock_price,
						expiration_block: entry.expiration_block,
					},
				);
				TrackedUtxos::<T>::try_mutate(|a| {
					a.try_insert(utxo.clone()).map_err(|_| Error::<T>::ExpirationAtBlockOverflow)
				})?;

				<LockedUtxoExpirationBlocks<T>>::try_mutate(
					entry.expiration_block,
					|utxos| -> DispatchResult {
						Ok(utxos
							.try_push(utxo.clone())
							.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?)
					},
				)?;
				Self::deposit_event(Event::UtxoLocked {
					utxo: utxo.clone(),
					account_id: entry.account_id,
					bond_id: entry.bond_id,
					lock_price: entry.lock_price,
					expiration_block: entry.expiration_block,
				});
			}

			let current_block = <frame_system::Pallet<T>>::block_number();
			let mut canceled_utxos = vec![];
			for utxo in utxo_sync.moved.into_iter() {
				if let Some(entry) = <LockedUtxos<T>>::take(&utxo) {
					if current_block < entry.expiration_block {
						let redemption_price = Self::get_redemption_price(&entry.satoshis)
							.unwrap_or(entry.lock_price)
							.max(entry.lock_price);
						match T::BondProvider::burn_bond(entry.bond_id, Some(redemption_price)) {
							Ok(_) => {
								TrackedUtxos::<T>::mutate(|a| a.remove(&utxo));
								canceled_utxos.push(utxo.clone());
								Self::deposit_event(Event::UtxoMovedWithBurn {
									utxo: utxo.clone(),
									bond_id: entry.bond_id,
								});
							},
							Err(e) => {
								let bond_id = entry.bond_id;
								// Stop paying out any mints, even if we can't burn the bond
								canceled_utxos.push(utxo.clone());
								// Re-insert it since we failed to burn it
								LockedUtxos::<T>::insert(utxo.clone(), entry);
								warn!(
									target: LOG_TARGET,
									"Failed to burn bond {:?} for bitcoin UTXO {:?}: {:?}", bond_id, &utxo, e
								);
							},
						}
					}
				}
				if let Some(entry) = utxo_pending_confirmation.remove(&utxo) {
					T::BondProvider::unlock_bond(entry.bond_id).map_err(Error::<T>::from)?;
					canceled_utxos.push(utxo.clone());
					Self::deposit_event(Event::UtxoApplicationRejected {
						utxo: utxo.clone(),
						account_id: entry.account_id,
						bond_id: entry.bond_id,
						lock_price: entry.lock_price,
						expiration_block: entry.expiration_block,
					});
				}
			}

			pending_mint_utxos.retain(|(utxo, _, _)| !canceled_utxos.contains(utxo));
			<PendingMintUtxos<T>>::put(pending_mint_utxos);
			<UtxosPendingConfirmation<T>>::put(utxo_pending_confirmation);

			Ok(())
		}

		/// Submit bitcoins to be minted as minting becomes available
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn lock(
			origin: OriginFor<T>,
			bond_id: Option<T::BondId>,
			txid: H256Le,
			output_index: u32,
			satoshis: Satoshis,
		) -> DispatchResult {
			ensure!(
				(<UtxosPendingConfirmation<T>>::get().len() as u32 +
					<PendingMintUtxos<T>>::get().len() as u32) <
					T::MaxPendingMintUtxos::get(),
				Error::<T>::MaxPendingMintUtxosExceeded
			);

			let who = ensure_signed(origin)?;

			// 1. verify ownership
			ensure!(
				satoshis > T::MinimumSatoshiAmount::get(),
				Error::<T>::InsufficientBitcoinAmount
			);

			let expiration_block =
				<frame_system::Pallet<T>>::block_number() + T::BondDurationBlocks::get();
			// 2. Check if enough is bonded, or create a new bond
			let prices = T::BitcoinPriceProvider::get_bitcoin_argon_prices(satoshis);

			let mut lock_price =
				prices.first().cloned().ok_or(Error::<T>::NoBitcoinPricesAvailable)?;
			let bond_id = if let Some(bond_id) = bond_id {
				let bond = T::BondProvider::get_bond(bond_id).map_err(Error::<T>::from)?;
				ensure!(bond.bonded_account_id == who, Error::<T>::InvalidBondSubmitted);
				lock_price = prices
					.into_iter()
					.find(|a| *a <= bond.amount)
					.ok_or(Error::<T>::InsufficientBondAmount)?;
				ensure!(
					bond.completion_block >= expiration_block,
					Error::<T>::PrematureBondExpiration
				);
				ensure!(!bond.is_locked, Error::<T>::BondAlreadyLocked);
				bond_id
			} else {
				T::BondProvider::bond_self(who.clone(), lock_price, expiration_block)
					.map_err(Error::<T>::from)?
			};
			T::BondProvider::lock_bond(bond_id).map_err(Error::<T>::from)?;

			let bitcoin_utxo_id = BitcoinUtxoId { txid, output_index };

			ensure!(
				!<LockedUtxos<T>>::contains_key(&bitcoin_utxo_id),
				Error::<T>::BitcoinAlreadyLocked
			);

			<UtxosPendingConfirmation<T>>::try_mutate(|utxo_pending_confirmation| {
				ensure!(
					!utxo_pending_confirmation.contains_key(&bitcoin_utxo_id),
					Error::<T>::BitcoinAlreadyLocked
				);
				utxo_pending_confirmation
					.try_insert(
						bitcoin_utxo_id,
						LockedUtxo {
							account_id: who.clone(),
							bond_id,
							satoshis,
							lock_price,
							expiration_block,
						},
					)
					.map_err(|_| Error::<T>::MaxPendingMintUtxosExceeded)?;
				Ok::<(), Error<T>>(())
			})?;

			Ok(())
		}

		/// Unlock a bitcoin UTXO that has been confirmed.
		///
		/// NOTE: this call will burn from your account the current argon value of the UTXO maxed at
		/// your buy-in price.
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn unlock(origin: OriginFor<T>, txid: H256Le, output_index: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let bitcoin_utxo_id = BitcoinUtxoId { txid, output_index };
			let locked_utxo =
				LockedUtxos::<T>::take(&bitcoin_utxo_id).ok_or(Error::<T>::UtxoNotLocked)?;
			ensure!(locked_utxo.account_id == who, Error::<T>::NoPermissions);
			let redemption_price = Self::get_redemption_price(&locked_utxo.satoshis)?;
			ensure!(
				T::Currency::reducible_balance(&who, Preservation::Expendable, Fortitude::Force) >=
					redemption_price,
				Error::<T>::InsufficientFunds
			);

			T::Currency::burn_from(&who, redemption_price, Precision::Exact, Fortitude::Force)?;
			LockedUtxoExpirationBlocks::<T>::try_mutate(
				locked_utxo.expiration_block,
				|utxos| -> DispatchResult {
					utxos.retain(|utxo| utxo != &bitcoin_utxo_id);
					Ok(())
				},
			)?;

			Ok(())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = BitcoinInherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier =
			ulx_primitives::inherents::BITCOIN_INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call>
		where
			InherentData: BitcoinInherentData,
		{
			let utxo_sync = data
				.bitcoin_sync()
				.expect("Could not decode bitcoin inherent data")
				.expect("Bitcoin inherent data must be provided");

			Some(Call::sync { utxo_sync })
		}

		fn is_inherent_required(_: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			Ok(Some(BitcoinInherentError::MissingInherent))
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::sync { .. })
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn on_argon_burn(amount: T::Balance) {
			let bitcoin_mint = MintedArgons::<T>::get();
			let total_minted = bitcoin_mint + T::UlixeeMintCirculation::get_mint_circulation();
			let prorata = (amount * bitcoin_mint).checked_div(&total_minted);
			if let Some(milligons) = prorata {
				MintedArgons::<T>::mutate(|mint| *mint -= milligons);
			}
		}

		pub fn get_redemption_price(satoshis: &Satoshis) -> Result<T::Balance, Error<T>> {
			let mut price: u128 = T::BitcoinPriceProvider::get_bitcoin_argon_price(*satoshis)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?
				.unique_saturated_into();
			let cpi = T::ArgonPriceProvider::get_argon_cpi_price().unwrap_or_default();
			if cpi > 0 {
				// The redemption price of the argon allocates fluctuating incentives based on how
				// fast the dip should be incentivized to be capitalized on.
				price = (713 * price + 274) / (price * 1000);
			};

			Ok(price.into())
		}
	}

	impl<T: Config> MintCirculationProvider<T::Balance> for Pallet<T> {
		fn get_mint_circulation() -> T::Balance {
			MintedArgons::<T>::get()
		}
	}

	impl<T: Config> BurnEventHandler<T::Balance> for Pallet<T> {
		fn on_argon_burn(milligons: &T::Balance) -> sp_runtime::DispatchResult {
			Self::on_argon_burn(*milligons);
			Ok(())
		}
	}

	impl<T> From<BondError> for Error<T> {
		fn from(e: BondError) -> Error<T> {
			match e {
				BondError::BadState => Error::<T>::BadState,
				BondError::BondNotFound => Error::<T>::BondNotFound,
				BondError::NoMoreBondIds => Error::<T>::NoMoreBondIds,
				BondError::MinimumBondAmountNotMet => Error::<T>::MinimumBondAmountNotMet,
				BondError::ExpirationAtBlockOverflow => Error::<T>::ExpirationAtBlockOverflow,
				BondError::InsufficientFunds => Error::<T>::InsufficientFunds,
				BondError::InsufficientBondFunds => Error::<T>::InsufficientBondFunds,
				BondError::ExpirationTooSoon => Error::<T>::ExpirationTooSoon,
				BondError::NoPermissions => Error::<T>::NoPermissions,
				BondError::BondFundClosed => Error::<T>::BondFundClosed,
				BondError::NoBondFundFound => Error::<T>::NoBondFundFound,
				BondError::HoldUnexpectedlyModified => Error::<T>::HoldUnexpectedlyModified,
				BondError::BondFundMaximumBondsExceeded => Error::<T>::BondFundMaximumBondsExceeded,
				BondError::UnrecoverableHold => Error::<T>::UnrecoverableHold,
				BondError::BondFundNotFound => Error::<T>::BondFundNotFound,
				BondError::BondAlreadyLocked => Error::<T>::BondAlreadyLocked,
				BondError::BondLockedCannotModify => Error::<T>::BondLockedCannotModify,
				BondError::FeeExceedsBondAmount => Error::<T>::FeeExceedsBondAmount,
				BondError::LeaseUntilBlockTooSoon => Error::<T>::LeaseUntilBlockTooSoon,
				BondError::LeaseUntilPastFundExpiration => Error::<T>::LeaseUntilPastFundExpiration,
				BondError::BondAlreadyClosed => Error::<T>::BondLockedCannotModify,
				BondError::AccountWouldBeBelowMinimum => Error::<T>::AccountWouldBeBelowMinimum,
			}
		}
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[codec(mel_bound(Balance: MaxEncodedLen, BlockNumber: MaxEncodedLen))]
pub struct LockedUtxo<AccountId, BondId, Balance, BlockNumber> {
	pub account_id: AccountId,
	pub bond_id: BondId,
	pub lock_price: Balance,
	pub satoshis: Satoshis,
	pub expiration_block: BlockNumber,
}
