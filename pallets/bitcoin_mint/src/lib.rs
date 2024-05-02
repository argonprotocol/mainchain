#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::tokens::Preservation;

pub use pallet::*;
use ulx_primitives::bitcoin::{LockedUtxo, Satoshis};
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
	use sp_std::{collections::btree_map::BTreeMap, vec};

	use ulx_primitives::{
		bitcoin::{
			BitcoinBlock, BitcoinBlockHash, BitcoinHeight, BitcoinRejectedReason,
			BitcoinScriptPubkey, BitcoinSyncStatus, BitcoinUtxo, H256Le, UtxoLookup,
		},
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

		/// The number of blocks previous to the tip that a bitcoin UTXO will be allowed to be
		/// locked
		#[pallet::constant]
		type MaxBitcoinBirthBlocksOld: Get<u64>;
	}

	/// Locked Bitcoin UTXOs that have had ownership confirmed. If a Bitcoin UTXO is moved before
	/// the expiration block, the bond is burned and the UTXO is unlocked.
	#[pallet::storage]
	pub(super) type LockedUtxos<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BitcoinUtxo,
		LockedUtxo<T::AccountId, T::BondId, T::Balance, BlockNumberFor<T>>,
		OptionQuery,
	>;

	/// Bitcoin UTXOs that have been submitted for ownership confirmation
	#[pallet::storage]
	pub(super) type UtxosPendingConfirmation<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<
			BitcoinUtxo,
			LockedUtxo<T::AccountId, T::BondId, T::Balance, BlockNumberFor<T>>,
			T::MaxPendingMintUtxos,
		>,
		ValueQuery,
	>;

	/// An oracle provided confirmed bitcoin block (eg, 6 blocks back)
	#[pallet::storage]
	pub(super) type ConfirmedBitcoinBlock<T: Config> = StorageValue<_, BitcoinBlock, OptionQuery>;

	/// The last synched bitcoin block
	#[pallet::storage]
	pub(super) type SynchedBitcoinBlock<T: Config> = StorageValue<_, BitcoinBlock, OptionQuery>;

	/// Bitcoin UTXOs that have been submitted for minting. This list is FIFO for minting whenever
	/// a) CPI >= 0 and
	/// b) the aggregate minted Bitcoins <= the aggregate minted Argons from Ulixee Shares
	#[pallet::storage]
	pub(super) type PendingMintUtxos<T: Config> = StorageValue<
		_,
		BoundedVec<(BitcoinUtxo, T::AccountId, T::Balance), T::MaxPendingMintUtxos>,
		ValueQuery,
	>;

	/// Bitcoin Oracle Operator Account
	#[pallet::storage]
	pub(super) type OracleOperatorAccount<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Expiration blocks mapped to Bitcoin UTXOs
	#[pallet::storage]
	pub(super) type LockedUtxoExpirationBlocks<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<BitcoinUtxo, T::MaxPendingMintUtxos>,
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
			utxo: BitcoinUtxo,
			account_id: T::AccountId,
			bond_id: T::BondId,
			lock_price: T::Balance,
			expiration_block: BlockNumberFor<T>,
		},
		UtxoUnlocked {
			utxo: BitcoinUtxo,
			account_id: T::AccountId,
			bond_id: T::BondId,
			lock_price: T::Balance,
		},
		UtxoApplicationRejected {
			utxo: BitcoinUtxo,
			account_id: T::AccountId,
			bond_id: T::BondId,
			lock_price: T::Balance,
			expiration_block: BlockNumberFor<T>,
			rejected_reason: BitcoinRejectedReason,
		},
		UtxoMovedWithBurn {
			utxo: BitcoinUtxo,
			bond_id: T::BondId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No Oraclized bitcoin block has been provided to the network
		NoBitcoinConfirmedBlock,
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
		/// Invalid bitcoin sync height attempted
		InvalidBitcoinSyncHeight,
		/// Bitcoin height not confirmed yet
		BitcoinHeightNotConfirmed,
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

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub operator: Option<T::AccountId>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if let Some(operator) = &self.operator {
				<OracleOperatorAccount<T>>::put(operator);
			}
		}
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

			Self::mint_bitcoin_argons();

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
				"Bitcoin UTXO sync submitted (spent: {:?}, confirmed {}, rejected {})", utxo_sync.spent.len(), utxo_sync.verified.len(), utxo_sync.invalid.len()
			);

			let current_confirmed =
				ConfirmedBitcoinBlock::<T>::get().ok_or(Error::<T>::NoBitcoinConfirmedBlock)?;
			ensure!(
				utxo_sync.sync_to_block.block_height <= current_confirmed.block_height,
				Error::<T>::InvalidBitcoinSyncHeight
			);
			if let Some(existing_sync) = SynchedBitcoinBlock::<T>::get() {
				ensure!(
					utxo_sync.sync_to_block.block_height > existing_sync.block_height,
					Error::<T>::InvalidBitcoinSyncHeight
				);
			}

			let mut utxo_pending_confirmation = <UtxosPendingConfirmation<T>>::get();
			let mut pending_mint_utxos = <PendingMintUtxos<T>>::get();
			for (utxo, confirmed_height) in utxo_sync.verified.into_iter() {
				let Some(mut entry) = utxo_pending_confirmation.remove(&utxo) else {
					continue;
				};

				entry.confirmed_height = confirmed_height;

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

				<LockedUtxos<T>>::insert(utxo.clone(), entry.clone());

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

			for (utxo, reason) in utxo_sync.invalid.into_iter() {
				if let Some(entry) = utxo_pending_confirmation.remove(&utxo) {
					Self::reject_application(utxo.clone(), entry, reason)?;
				}
			}

			let mut canceled_utxos = vec![];
			for (utxo, _) in utxo_sync.spent.into_iter() {
				// if the UTXO is locked, it was redeemed without an unlock, so we need to burn it
				if let Some(entry) = <LockedUtxos<T>>::take(&utxo) {
					let redemption_price = Self::get_redemption_price(&entry.satoshis)
						.unwrap_or(entry.lock_price)
						.max(entry.lock_price);
					match T::BondProvider::burn_bond(entry.bond_id, Some(redemption_price)) {
						Ok(_) => {
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
				if let Some(entry) = utxo_pending_confirmation.remove(&utxo) {
					Self::reject_application(utxo.clone(), entry, BitcoinRejectedReason::Spent)?;
					canceled_utxos.push(utxo.clone());
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
			#[pallet::compact] output_index: u32,
			#[pallet::compact] satoshis: Satoshis,
			bitcoin_script_pubkey: BitcoinScriptPubkey,
			#[pallet::compact] confirmed_height: BitcoinHeight,
		) -> DispatchResult {
			ensure!(
				(<UtxosPendingConfirmation<T>>::get().len() as u32 +
					<PendingMintUtxos<T>>::get().len() as u32) <
					T::MaxPendingMintUtxos::get(),
				Error::<T>::MaxPendingMintUtxosExceeded
			);

			let current_confirmed =
				ConfirmedBitcoinBlock::<T>::get().ok_or(Error::<T>::NoBitcoinConfirmedBlock)?;
			ensure!(
				confirmed_height <= current_confirmed.block_height,
				Error::<T>::BitcoinHeightNotConfirmed
			);
			// Convenience checks for user. These will also be checked in the bitcoin sync
			ensure!(
				confirmed_height >=
					current_confirmed
						.block_height
						.saturating_sub(T::MaxBitcoinBirthBlocksOld::get()),
				Error::<T>::RedemptionsUnavailable
			);

			ensure!(
				satoshis >= T::MinimumSatoshiAmount::get(),
				Error::<T>::InsufficientBitcoinAmount
			);

			let who = ensure_signed(origin)?;

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

			let bitcoin_utxo = BitcoinUtxo { txid, output_index };

			ensure!(
				!<LockedUtxos<T>>::contains_key(&bitcoin_utxo),
				Error::<T>::BitcoinAlreadyLocked
			);

			<UtxosPendingConfirmation<T>>::try_mutate(|utxo_pending_confirmation| {
				ensure!(
					!utxo_pending_confirmation.contains_key(&bitcoin_utxo),
					Error::<T>::BitcoinAlreadyLocked
				);
				utxo_pending_confirmation
					.try_insert(
						bitcoin_utxo,
						LockedUtxo {
							account_id: who.clone(),
							bond_id,
							satoshis,
							lock_price,
							expiration_block,
							script_pubkey: bitcoin_script_pubkey,
							confirmed_height,
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
		pub fn unlock(
			origin: OriginFor<T>,
			txid: H256Le,
			#[pallet::compact] output_index: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let bitcoin_utxo = BitcoinUtxo { txid, output_index };
			let locked_utxo =
				LockedUtxos::<T>::take(&bitcoin_utxo).ok_or(Error::<T>::UtxoNotLocked)?;
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
					utxos.retain(|utxo| utxo != &bitcoin_utxo);
					Ok(())
				},
			)?;

			Ok(())
		}

		/// Sets the most recent confirmed bitcoin block height (only executable by the Oracle
		/// Operator account)
		///
		/// # Arguments
		/// * `bitcoin_height` - the latest bitcoin block height to be confirmed
		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn set_confirmed_block(
			origin: OriginFor<T>,
			bitcoin_height: BitcoinHeight,
			bitcoin_block_hash: BitcoinBlockHash,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Some(who) == <OracleOperatorAccount<T>>::get(), Error::<T>::NoPermissions);
			<ConfirmedBitcoinBlock<T>>::put(BitcoinBlock {
				block_height: bitcoin_height,
				block_hash: bitcoin_block_hash,
			});
			Ok(())
		}

		/// Sets the oracle operator account id (only executable by the Root account)
		///
		/// # Arguments
		/// * `account_id` - the account id of the operator
		#[pallet::call_index(4)]
		#[pallet::weight(0)]
		pub fn set_operator(origin: OriginFor<T>, account_id: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			<OracleOperatorAccount<T>>::put(account_id.clone());
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

	impl<T: Config> Pallet<T> {
		pub fn on_argon_burn(amount: T::Balance) {
			let bitcoin_mint = MintedArgons::<T>::get();
			let total_minted = bitcoin_mint + T::UlixeeMintCirculation::get_mint_circulation();
			let prorata = (amount * bitcoin_mint).checked_div(&total_minted);
			if let Some(milligons) = prorata {
				MintedArgons::<T>::mutate(|mint| *mint -= milligons);
			}
		}
		pub fn get_sync_status() -> Option<BitcoinSyncStatus> {
			let confirmed_block = ConfirmedBitcoinBlock::<T>::get()?;
			let synched_block = SynchedBitcoinBlock::<T>::get();
			let oldest_allowed_block_height =
				confirmed_block.block_height.saturating_sub(T::MaxBitcoinBirthBlocksOld::get());
			Some(BitcoinSyncStatus { confirmed_block, synched_block, oldest_allowed_block_height })
		}
		pub fn active_utxos() -> BTreeMap<BitcoinUtxo, UtxoLookup> {
			let mut utxos = BTreeMap::new();
			// TODO: This is not efficient, but we don't have a better way to do this right now
			for (utxo, entry) in <LockedUtxos<T>>::iter() {
				utxos.insert(
					utxo,
					UtxoLookup { script_pubkey: entry.script_pubkey, pending_confirmation: None },
				);
			}
			for (utxo, entry) in <UtxosPendingConfirmation<T>>::get() {
				utxos.insert(
					utxo,
					UtxoLookup {
						script_pubkey: entry.script_pubkey,
						pending_confirmation: Some((entry.satoshis, entry.confirmed_height)),
					},
				);
			}
			utxos
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

		fn reject_application(
			utxo: BitcoinUtxo,
			entry: LockedUtxo<T::AccountId, T::BondId, T::Balance, BlockNumberFor<T>>,
			rejected_reason: BitcoinRejectedReason,
		) -> DispatchResult {
			T::BondProvider::unlock_bond(entry.bond_id).map_err(Error::<T>::from)?;
			Self::deposit_event(Event::UtxoApplicationRejected {
				utxo,
				account_id: entry.account_id,
				bond_id: entry.bond_id,
				lock_price: entry.lock_price,
				expiration_block: entry.expiration_block,
				rejected_reason,
			});
			Ok(())
		}

		pub(crate) fn mint_bitcoin_argons() {
			let mut bitcoin_mint = crate::pallet::MintedArgons::<T>::get();
			let ulixee_mint = T::UlixeeMintCirculation::get_mint_circulation();
			let mut available_to_mint = ulixee_mint - bitcoin_mint;

			// if current cpi is 0, we can't mint any argons
			if T::ArgonPriceProvider::get_argon_cpi_price().unwrap_or_default() <= 0 {
				available_to_mint = T::Balance::zero();
			}

			if available_to_mint > T::Balance::zero() {
				let updated = <crate::pallet::PendingMintUtxos<T>>::get().try_mutate(|pending| {
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
