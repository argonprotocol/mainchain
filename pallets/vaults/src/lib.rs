#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

/// The vaults pallet allows a user to offer argons for lease to other users. There are two types of
/// bonds offered in the system, Bitcoin and Mining bonds. Vaults can define the amount of argons
/// available for each type of bond, and the interest rate for each. However, mining bonds may only
/// issued up to the amount of bitcoin argons that are locked.
///
/// ** Bitcoin Securitization **
///
/// A vault may apply a securitization bond to their account up to 2x the locked value of their
/// bitcoin argons. This allows a vault to issue more mining bonds, but the funds are locked up for
/// the duration of the bitcoin bonds, and will be taken in the case of bitcoins not being cosiged
/// on unlock.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, InspectHold, Mutate, MutateHold},
			tokens::{Fortitude, Precision, Preservation, Restriction},
			Incrementable,
		},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{
			AtLeast32BitUnsigned, CheckedAdd, CheckedDiv, CheckedSub, UniqueSaturatedInto, Zero,
		},
		DispatchError::Token,
		Saturating, TokenError,
	};
	use sp_std::vec;

	use ulx_primitives::{
		bitcoin::{
			create_timelock_multisig_script, BitcoinCosignScriptPubkey, BitcoinHeight,
			BitcoinPubkeyHash, UtxoId,
		},
		bond::{
			Bond, BondError, BondType, ThreeDecimalPercent, Vault, VaultArgons, VaultProvider,
			WholePercent,
		},
		VaultId,
	};

	use super::*;

	#[pallet::pallet]
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
			+ sp_std::fmt::Debug
			+ Default
			+ From<u128>
			+ TryInto<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		/// Minimum amount for a bond
		#[pallet::constant]
		type MinimumBondAmount: Get<Self::Balance>;

		/// Ulixee blocks per day
		#[pallet::constant]
		type BlocksPerDay: Get<BlockNumberFor<Self>>;

		/// The max amount of pending bitcoin pubkey hashes allowed
		#[pallet::constant]
		type MaxVaultBitcoinPubkeys: Get<u32>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		EnterVault,
		BondFee,
	}

	#[pallet::storage]
	pub(super) type NextVaultId<T: Config> = StorageValue<_, VaultId, OptionQuery>;

	/// Vaults by id
	#[pallet::storage]
	pub(super) type VaultsById<T: Config> =
		StorageMap<_, Twox64Concat, VaultId, Vault<T::AccountId, T::Balance>, OptionQuery>;

	/// Vault Bitcoin Pubkeys by VaultId
	#[pallet::storage]
	pub(super) type VaultPubkeysById<T: Config> = StorageMap<
		_,
		Twox64Concat,
		VaultId,
		BoundedVec<BitcoinPubkeyHash, T::MaxVaultBitcoinPubkeys>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		VaultCreated {
			vault_id: VaultId,
			bitcoin_argons: T::Balance,
			mining_argons: T::Balance,
			securitization_percent: WholePercent,
			operator_account_id: T::AccountId,
		},
		VaultModified {
			vault_id: VaultId,
			bitcoin_argons: T::Balance,
			mining_argons: T::Balance,
			securitization_percent: WholePercent,
		},
		VaultClosed {
			vault_id: VaultId,
			bitcoin_amount_still_bonded: T::Balance,
			mining_amount_still_bonded: T::Balance,
			securitization_still_bonded: T::Balance,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		BondNotFound,
		NoMoreVaultIds,
		NoMoreBondIds,
		MinimumBondAmountNotMet,
		/// There are too many bond or bond funds expiring in the given expiration block
		ExpirationAtBlockOverflow,
		InsufficientFunds,
		InsufficientVaultFunds,
		/// The vault does not have enough bitcoins to cover the mining bond
		InsufficientBitcoinsForMining,
		/// The proposed transaction would take the account below the minimum (existential) balance
		AccountBelowMinimumBalance,
		VaultClosed,
		/// Funding would result in an overflow of the balance type
		InvalidVaultAmount,
		/// This reduction in bond funds offered goes below the amount that is already committed to
		VaultReductionBelowAllocatedFunds,
		/// An invalid securitization percent was provided for the vault. NOTE: it cannot be
		/// decreased
		InvalidSecuritization,
		/// The maximum number of bitcoin pubkeys for a vault has been exceeded
		MaxVaultBitcoinPubkeys,
		/// Securitization percent would exceed the maximum allowed
		MaxSecuritizationPercentExceeded,
		InvalidBondType,
		BitcoinUtxoNotFound,
		InsufficientSatoshisBonded,
		NoBitcoinPricesAvailable,
		/// The bitcoin script to lock this bitcoin has errors
		InvalidBitcoinScript,
		ExpirationTooSoon,
		NoPermissions,
		HoldUnexpectedlyModified,
		UnrecoverableHold,
		VaultNotFound,
		/// The fee for this bond exceeds the amount of the bond, which is unsafe
		FeeExceedsBondAmount,
		/// No Vault public keys are available
		NoVaultBitcoinPubkeysAvailable,
	}

	impl<T> From<BondError> for Error<T> {
		fn from(e: BondError) -> Error<T> {
			match e {
				BondError::BondNotFound => Error::<T>::BondNotFound,
				BondError::NoMoreBondIds => Error::<T>::NoMoreBondIds,
				BondError::MinimumBondAmountNotMet => Error::<T>::MinimumBondAmountNotMet,
				BondError::ExpirationAtBlockOverflow => Error::<T>::ExpirationAtBlockOverflow,
				BondError::InsufficientFunds => Error::<T>::InsufficientFunds,
				BondError::InsufficientBitcoinsForMining =>
					Error::<T>::InsufficientBitcoinsForMining,
				BondError::ExpirationTooSoon => Error::<T>::ExpirationTooSoon,
				BondError::NoPermissions => Error::<T>::NoPermissions,
				BondError::HoldUnexpectedlyModified => Error::<T>::HoldUnexpectedlyModified,
				BondError::UnrecoverableHold => Error::<T>::UnrecoverableHold,
				BondError::VaultNotFound => Error::<T>::VaultNotFound,
				BondError::FeeExceedsBondAmount => Error::<T>::FeeExceedsBondAmount,
				BondError::InsufficientVaultFunds => Error::<T>::InsufficientVaultFunds,
				BondError::VaultClosed => Error::<T>::VaultClosed,
				BondError::AccountWouldBeBelowMinimum => Error::<T>::AccountBelowMinimumBalance,
				BondError::InvalidBitcoinScript => Error::<T>::InvalidBitcoinScript,
				BondError::NoVaultBitcoinPubkeysAvailable =>
					Error::<T>::NoVaultBitcoinPubkeysAvailable,
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn create(
			origin: OriginFor<T>,
			bitcoin_annual_percent_rate: ThreeDecimalPercent,
			mining_annual_percent_rate: ThreeDecimalPercent,
			#[pallet::compact] bitcoin_amount_allocated: T::Balance,
			#[pallet::compact] mining_amount_allocated: T::Balance,
			#[pallet::compact] securitization_percent: WholePercent,
			bitcoin_pubkey_hashes: BoundedVec<BitcoinPubkeyHash, T::MaxVaultBitcoinPubkeys>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				securitization_percent <= 200u16.into(),
				Error::<T>::MaxSecuritizationPercentExceeded
			);
			ensure!(
				bitcoin_amount_allocated.checked_add(&mining_amount_allocated).is_some(),
				Error::<T>::InvalidVaultAmount
			);

			let vault_id = NextVaultId::<T>::get().unwrap_or(1);
			let next_vault_id = vault_id.increment().ok_or(Error::<T>::NoMoreVaultIds)?;
			NextVaultId::<T>::set(Some(next_vault_id));

			let mut vault = Vault {
				operator_account_id: who.clone(),
				bitcoin_argons: VaultArgons {
					annual_percent_rate: bitcoin_annual_percent_rate,
					allocated: bitcoin_amount_allocated,
					bonded: 0u32.into(),
				},
				mining_argons: VaultArgons {
					annual_percent_rate: mining_annual_percent_rate,
					allocated: mining_amount_allocated,
					bonded: 0u32.into(),
				},
				securitization_percent,
				securitized_argons: 0u32.into(),
				is_closed: false,
			};
			VaultPubkeysById::<T>::insert(vault_id, bitcoin_pubkey_hashes);

			vault.securitized_argons = vault.get_minimum_securitization_needed();

			Self::hold(
				&who,
				bitcoin_amount_allocated + mining_amount_allocated + vault.securitized_argons,
				HoldReason::EnterVault,
			)
			.map_err(Error::<T>::from)?;

			VaultsById::<T>::insert(vault_id, vault);
			Self::deposit_event(Event::VaultCreated {
				vault_id,
				bitcoin_argons: bitcoin_amount_allocated,
				mining_argons: mining_amount_allocated,
				securitization_percent,
				operator_account_id: who,
			});

			Ok(())
		}

		/// Add additional funds to the vault
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn modify(
			origin: OriginFor<T>,
			vault_id: VaultId,
			total_mining_amount_offered: T::Balance,
			total_bitcoin_amount_offered: T::Balance,
			securitization_percent: u16,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut vault = VaultsById::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;
			ensure!(vault.operator_account_id == who, Error::<T>::NoPermissions);

			let mut amount_to_hold: i128 = 0;
			// NOTE: We're not changing the amount of bonded argons, so nothing needs to be checked
			// about the ratio of mining to bitcoin
			if vault.bitcoin_argons.allocated != total_bitcoin_amount_offered {
				ensure!(
					vault.bitcoin_argons.bonded <= total_bitcoin_amount_offered,
					Error::<T>::VaultReductionBelowAllocatedFunds
				);

				amount_to_hold += balance_to_i128::<T>(total_bitcoin_amount_offered) -
					balance_to_i128::<T>(vault.bitcoin_argons.allocated);
				vault.bitcoin_argons.allocated = total_bitcoin_amount_offered;
			}

			if vault.mining_argons.allocated != total_mining_amount_offered {
				ensure!(
					vault.mining_argons.bonded <= total_mining_amount_offered,
					Error::<T>::VaultReductionBelowAllocatedFunds
				);

				amount_to_hold += balance_to_i128::<T>(total_mining_amount_offered) -
					balance_to_i128::<T>(vault.mining_argons.allocated);
				vault.mining_argons.allocated = total_mining_amount_offered;
			}

			ensure!(
				securitization_percent >= vault.securitization_percent,
				Error::<T>::InvalidSecuritization
			);

			vault.securitization_percent = securitization_percent;

			let total_securities = vault.get_minimum_securitization_needed();

			amount_to_hold += balance_to_i128::<T>(total_securities) -
				balance_to_i128::<T>(vault.securitized_argons);
			vault.securitized_argons = total_securities;

			if amount_to_hold > 0 {
				Self::hold(&who, (amount_to_hold as u128).into(), HoldReason::EnterVault)
					.map_err(Error::<T>::from)?;
			} else if amount_to_hold < 0 {
				Self::release_hold(
					&who,
					(amount_to_hold.abs() as u128).into(),
					HoldReason::EnterVault,
				)?;
			}

			Self::deposit_event(Event::VaultModified {
				vault_id,
				bitcoin_argons: total_bitcoin_amount_offered,
				mining_argons: total_mining_amount_offered,
				securitization_percent,
			});
			VaultsById::<T>::insert(vault_id, vault);

			Ok(())
		}

		/// Stop offering additional bonds from this vault. Will not affect existing bond.
		/// As funds are returned, they will be released to the vault owner.
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn close(origin: OriginFor<T>, vault_id: VaultId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let mut vault =
				VaultsById::<T>::get(vault_id).ok_or::<Error<T>>(Error::<T>::VaultNotFound)?;

			ensure!(vault.operator_account_id == who, Error::<T>::NoPermissions);

			vault.is_closed = true;

			let securitization_still_needed = vault.get_minimum_securitization_needed();
			let free_securitization =
				vault.securitized_argons.saturating_sub(securitization_still_needed);

			let return_amount = vault.bitcoin_argons.free_balance() +
				vault.mining_argons.free_balance() +
				free_securitization;

			ensure!(
				T::Currency::balance_on_hold(&HoldReason::EnterVault.into(), &who) >= return_amount,
				Error::<T>::HoldUnexpectedlyModified
			);

			Self::release_hold(&who, return_amount, HoldReason::EnterVault)?;

			vault.bitcoin_argons.allocated = vault.bitcoin_argons.bonded;
			vault.mining_argons.allocated = vault.mining_argons.bonded;
			vault.securitized_argons = securitization_still_needed;
			Self::deposit_event(Event::VaultClosed {
				vault_id,
				bitcoin_amount_still_bonded: vault.bitcoin_argons.bonded,
				mining_amount_still_bonded: vault.mining_argons.bonded,
				securitization_still_bonded: securitization_still_needed,
			});
			VaultsById::<T>::insert(vault_id, vault);

			Ok(())
		}

		/// Add public key hashes to the vault. Will be inserted at the beginning of the list.
		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn add_bitcoin_pubkey_hashes(
			origin: OriginFor<T>,
			vault_id: VaultId,
			bitcoin_pubkey_hashes: BoundedVec<BitcoinPubkeyHash, T::MaxVaultBitcoinPubkeys>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let vault =
				VaultsById::<T>::get(vault_id).ok_or::<Error<T>>(Error::<T>::VaultNotFound)?;

			ensure!(vault.operator_account_id == who, Error::<T>::NoPermissions);

			VaultPubkeysById::<T>::try_mutate(vault_id, |x| {
				if let Some(x) = x {
					let mut bitcoin_pubkey_hashes = bitcoin_pubkey_hashes;
					bitcoin_pubkey_hashes
						.try_append(&mut x.to_vec())
						.map_err(|_| Error::<T>::MaxVaultBitcoinPubkeys)?;
					*x = bitcoin_pubkey_hashes;
				} else {
					*x = Some(bitcoin_pubkey_hashes);
				}
				Ok::<(), Error<T>>(())
			})?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn hold(
			who: &T::AccountId,
			amount: T::Balance,
			reason: HoldReason,
		) -> Result<(), BondError> {
			if amount == T::Balance::zero() {
				return Ok(());
			}

			let needs_providers = T::Currency::balance_on_hold(&reason.into(), who) == 0u128.into();

			T::Currency::hold(&reason.into(), who, amount).map_err(|e| match e {
				Token(TokenError::BelowMinimum) => BondError::AccountWouldBeBelowMinimum,
				_ => {
					let balance = T::Currency::balance(who);
					if balance.checked_sub(&amount).is_some() &&
						balance.saturating_sub(amount) < T::Currency::minimum_balance()
					{
						return BondError::AccountWouldBeBelowMinimum;
					}

					BondError::InsufficientFunds
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

		pub(crate) fn calculate_fees(
			annual_percentage_rate: ThreeDecimalPercent,
			amount: T::Balance,
			blocks: BlockNumberFor<T>,
		) -> T::Balance {
			let percent_basis: T::Balance = 100_000u128.into();

			let blocks_per_day: T::Balance = blocks_into_u128::<T>(T::BlocksPerDay::get()).into();

			let blocks_per_year: T::Balance = blocks_per_day * 365u32.into();
			let blocks: T::Balance = blocks_into_u128::<T>(blocks).into();

			// min of 1 day
			let blocks = blocks.max(blocks_per_day);

			let fee: T::Balance = amount
				.saturating_mul(annual_percentage_rate.into())
				.saturating_mul(blocks)
				.checked_div(&blocks_per_year)
				.unwrap_or_default()
				.checked_div(&percent_basis)
				.unwrap_or_default(); // amount is in milligons
			fee
		}
	}

	impl<T: Config> VaultProvider for Pallet<T> {
		type AccountId = T::AccountId;
		type Balance = T::Balance;
		type BlockNumber = BlockNumberFor<T>;

		fn get(vault_id: VaultId) -> Option<Vault<Self::AccountId, Self::Balance>> {
			VaultsById::<T>::get(vault_id)
		}

		fn bond_funds(
			vault_id: VaultId,
			amount: Self::Balance,
			bond_type: BondType,
			blocks: Self::BlockNumber,
			bond_account_id: &Self::AccountId,
		) -> Result<(Self::Balance, Self::Balance), BondError> {
			ensure!(amount >= T::MinimumBondAmount::get(), BondError::MinimumBondAmountNotMet);
			let mut vault =
				VaultsById::<T>::get(vault_id).ok_or::<BondError>(BondError::VaultNotFound)?;

			ensure!(!vault.is_closed, BondError::VaultClosed);

			let vault_argons = match bond_type {
				BondType::Bitcoin => {
					ensure!(
						vault.bitcoin_argons.free_balance() >= amount,
						BondError::InsufficientVaultFunds
					);
					&mut vault.bitcoin_argons
				},
				BondType::Mining => {
					ensure!(
						vault.amount_eligible_for_mining() >= amount,
						BondError::InsufficientVaultFunds
					);
					&mut vault.mining_argons
				},
			};

			let apr = vault_argons.annual_percent_rate;

			let fee = Self::calculate_fees(apr, amount, blocks);
			ensure!(fee <= amount.into(), BondError::FeeExceedsBondAmount);

			let base_fee = Self::calculate_fees(apr, amount, T::BlocksPerDay::get());

			T::Currency::transfer(
				bond_account_id,
				&vault.operator_account_id,
				base_fee,
				Preservation::Preserve,
			)
			.map_err(|e| match e {
				Token(TokenError::BelowMinimum) => BondError::AccountWouldBeBelowMinimum,
				_ => BondError::InsufficientFunds,
			})?;

			if fee > base_fee {
				Self::hold(bond_account_id, fee - base_fee, HoldReason::BondFee)?;
			}

			vault_argons.bonded = vault_argons.bonded.saturating_add(amount);
			VaultsById::<T>::set(vault_id, Some(vault));

			Ok((fee, base_fee))
		}

		fn burn_vault_bitcoin_funds(
			bond: &Bond<T::AccountId, T::Balance, BlockNumberFor<T>>,
			amount_to_burn: T::Balance,
		) -> Result<(), BondError> {
			let vault_id = bond.vault_id;
			let mut vault = VaultsById::<T>::get(vault_id).ok_or(BondError::VaultNotFound)?;

			vault.bitcoin_argons.destroy_bond_funds(amount_to_burn)?;

			T::Currency::burn_held(
				&HoldReason::EnterVault.into(),
				&vault.operator_account_id,
				amount_to_burn,
				Precision::Exact,
				Fortitude::Force,
			)
			.map_err(|_| BondError::UnrecoverableHold)?;

			VaultsById::<T>::insert(vault_id, vault);

			Ok(())
		}

		/// Recoup funds from the vault. This will be called if a vault has performed an illegal
		/// activity, like not moving cosigned UTXOs in the appropriate timeframe.
		///
		/// This will take funds from the vault in the following order:
		/// 1. From the bonded funds
		/// 2. From the allocated funds
		/// 3. From the securitized funds
		/// 4. TODO: From the Ulixee shares
		///
		/// The funds will be returned to the owed_to_account_id
		///
		/// Returns the amount that was recouped
		fn compensate_lost_bitcoin(
			bond: &Bond<T::AccountId, T::Balance, BlockNumberFor<T>>,
			market_rate: Self::Balance,
		) -> Result<Self::Balance, BondError> {
			let vault_id = bond.vault_id;
			let bonded_account_id = &bond.bonded_account_id;
			let remaining_fee = bond.total_fee.saturating_sub(bond.prepaid_fee);
			let bonded_amount = bond.amount;
			let mut vault = VaultsById::<T>::get(vault_id).ok_or(BondError::VaultNotFound)?;

			let vault_operator = vault.operator_account_id.clone();

			// the remaining fee is not paid
			if remaining_fee > 0u128.into() {
				Self::release_hold(&bonded_account_id, remaining_fee, HoldReason::BondFee)
					.map_err(|_| BondError::UnrecoverableHold)?;
			}
			// 1. take away from the vault first
			vault.bitcoin_argons.destroy_bond_funds(bonded_amount.min(market_rate))?;

			let mut still_owed = market_rate.saturating_sub(bonded_amount);
			let zero = T::Balance::zero();

			// 2: use bitcoin argons
			if still_owed > zero && vault.bitcoin_argons.free_balance() >= zero {
				let amount_to_pull = still_owed.min(vault.bitcoin_argons.free_balance());
				vault.bitcoin_argons.destroy_allocated_funds(amount_to_pull)?;
				still_owed -= amount_to_pull;
			}

			// 3. Use securitized argons
			if still_owed > zero && vault.securitized_argons >= zero {
				let amount_to_pull = still_owed.min(vault.securitized_argons);
				vault.securitized_argons -= amount_to_pull;
				still_owed -= amount_to_pull;
			}

			// 3. Use ulixee shares at current value
			// TODO

			T::Currency::transfer_on_hold(
				&HoldReason::EnterVault.into(),
				&vault_operator,
				&bonded_account_id,
				market_rate - still_owed,
				Precision::Exact,
				Restriction::Free,
				Fortitude::Force,
			)
			.map_err(|_| BondError::UnrecoverableHold)?;

			VaultsById::<T>::insert(vault_id, vault);

			Ok(market_rate - still_owed)
		}

		fn release_bonded_funds(
			bond: &Bond<T::AccountId, T::Balance, BlockNumberFor<T>>,
			should_charge_fee: bool,
		) -> Result<T::Balance, BondError> {
			let vault_id = bond.vault_id;
			let vault = {
				let mut vault = VaultsById::<T>::get(vault_id).ok_or(BondError::VaultNotFound)?;
				vault.mut_argons(&bond.bond_type).reduce_bonded(bond.amount);
				vault
			};
			// after reducing the bonded, we can check the minimum securitization needed (can't be
			// mut)
			let minimum_securitization = vault.get_minimum_securitization_needed();
			// working around borrow checker
			let mut vault = vault;
			if vault.is_closed {
				let free_securitization =
					vault.securitized_argons.saturating_sub(minimum_securitization);

				Self::release_hold(
					&vault.operator_account_id,
					bond.amount + free_securitization,
					HoldReason::EnterVault,
				)
				.map_err(|_| BondError::UnrecoverableHold)?;

				vault.securitized_argons = minimum_securitization;
				vault.mut_argons(&bond.bond_type).reduce_allocated(bond.amount);
			}
			let remaining_fee = bond.total_fee.saturating_sub(bond.prepaid_fee);
			if should_charge_fee {
				if remaining_fee > 0u128.into() {
					T::Currency::transfer_on_hold(
						&HoldReason::BondFee.into(),
						&bond.bonded_account_id,
						&vault.operator_account_id,
						remaining_fee,
						Precision::Exact,
						Restriction::Free,
						Fortitude::Force,
					)
					.map_err(|_| BondError::UnrecoverableHold)?;
				}
			} else {
				Self::release_hold(&bond.bonded_account_id, remaining_fee, HoldReason::BondFee)
					.map_err(|_| BondError::UnrecoverableHold)?;
			}
			VaultsById::<T>::insert(vault_id, vault);
			Ok(remaining_fee)
		}

		fn create_utxo_script_pubkey(
			vault_id: VaultId,
			_utxo_id: UtxoId,
			owner_pubkey_hash: BitcoinPubkeyHash,
			vault_claim_height: BitcoinHeight,
			open_claim_height: BitcoinHeight,
		) -> Result<(BitcoinPubkeyHash, BitcoinCosignScriptPubkey), BondError> {
			let vault_pubkey_hash = VaultPubkeysById::<T>::mutate(vault_id, |a| {
				if let Some(a) = a {
					Ok(a.pop())
				} else {
					Err(BondError::VaultNotFound)
				}
			})?
			.ok_or(BondError::NoVaultBitcoinPubkeysAvailable)?;

			let script_pubkey = create_timelock_multisig_script(
				vault_pubkey_hash.clone(),
				owner_pubkey_hash,
				vault_claim_height,
				open_claim_height,
			)
			.map_err(|_| BondError::InvalidBitcoinScript)?;

			Ok((
				vault_pubkey_hash,
				script_pubkey
					.to_p2wsh()
					.try_into()
					.map_err(|_| BondError::InvalidBitcoinScript)?,
			))
		}
	}

	fn blocks_into_u128<T: Config>(blocks: BlockNumberFor<T>) -> u128 {
		UniqueSaturatedInto::<u128>::unique_saturated_into(blocks)
	}

	fn balance_to_i128<T: Config>(balance: T::Balance) -> i128 {
		UniqueSaturatedInto::<u128>::unique_saturated_into(balance) as i128
	}
}
