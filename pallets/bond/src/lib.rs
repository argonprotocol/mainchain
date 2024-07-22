#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate core;

pub use pallet::*;
use sp_runtime::DispatchError;
use ulx_bitcoin::UtxoUnlocker;
use ulx_primitives::bitcoin::{BitcoinSignature, CompressedBitcoinPubkey};
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
const LOG_TARGET: &str = "runtime::bond";

/// The bond pallet allows users to manage the lifecycle of Bitcoin bonds, and stores the state for
/// Mining Bonds. Bonds lock up argons for a pre-defined period of time for a fee. A vault issuer
/// can determine that fee on their own.
///
/// ** Vaults: **
/// Vaults are managed in the vault pallet, but determine the amount of funding eligible for
/// bonding.
///
/// ** Bitcoin Bonds: **
///
/// Bitcoin bonds allow a user to mint new argons equal to the current market price of the bonded
/// UTXO's satoshis. The bond must lock up the equivalent argons for a year's time. At any time
/// during the bonded year, a Bitcoin holder is eligible to cancel their bond and unlock their
/// bitcoin. To unlock a bitcoin, a user must pay back the current market price of bitcoin (capped
/// at their bonded price). Should they move their UTXO via the bitcoin network, the current value
/// of the UTXO will be burned from the bond and vault funds.
///
/// _Bitcoin multisig/ownership_
/// A bitcoin holder retains ownership of their UTXO via a pubkey script that is pre-agreed by the
/// vault user and the bitcoin holder. The vault's hashed public key can be obtained in this pallet,
/// and will be combined with a hashed pubkey provided by the user. The pre-agreed script will be
/// such that both signatures are required to unlock the bitcoin before 370 days of blocks. After
/// 370 days, only the Vault's signature will be required to unlock the bitcoin for 30 days. After
/// 400 days, either party will be able to unlock.
///
/// NOTE: the bond will end on day 365, which gives a 5-day grace period for a bitcoin owner to buy
/// back their bitcoin before the vault can claim it.
///
/// _Unlocking a Bitcoin_
/// A bitcoin owner will pre-create a transaction to unlock their UTXO and submit the sighash to
/// this pallet. The vault operator has 10 days to publish a counter signature along with the public
/// key. If the vault operator fails to do so, they will lose their Ulixee shares and all underlying
/// Bitcoin bonds. A user will be made whole via a governance vote.
///
/// _Penalties_
/// 1. If a UTXO is found to have moved before a bond expiration via the bitcoin network, the vault
///    will be penalized by the amount of the UTXOs' current value.
/// 2. If a vault operator fails to counter-sign a transaction within 10 days, they will lose their
///    Ulixee shares and all underlying Bitcoin bonds.
///
/// ** Mining Bonds: **
///
/// A mining bond allows a user to jump the line for mining slots. A bond with a higher number of
/// locked argons will take the place of a non-bonded or lower bonded miner. In such cases, the bond
/// will be canceled and refunded.
///
/// Mining bonds can be offered by a vault operator up to a maximum of the number of bitcoin argons
/// they have bonded. Eg, they must be bonded 1-to-1 with bitcoin argons.
///
/// Mining bonds last the duration of a slot window, which is 40 days.
///
/// NOTE: a bond has a minimum 1-day cost which will not be refunded. This cost is required to make
/// it uneconomical to try to eat up all bonds to win a mining slot.
///
///
/// ** Bitcoin Securitization **
///
/// A vault may apply a securitization bond to their account up to 2x the locked value of their
/// bitcoin argons. This allows a vault to issue more mining bonds, but the funds are locked up for
/// the duration of the bitcoin bonds, and will be taken in the case of bitcoins not being returned.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use alloc::vec;
	use codec::Codec;
	use frame_support::{
		pallet_prelude::*,
		storage::with_storage_layer,
		traits::{
			fungible::{Inspect, Mutate, MutateHold},
			tokens::{Fortitude, Precision},
			Incrementable,
		},
		BoundedVec,
	};
	use frame_system::pallet_prelude::*;
	use log::warn;
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, UniqueSaturatedInto},
		DispatchError::Token,
		FixedPointNumber, Saturating, TokenError,
	};

	use super::*;
	use sp_arithmetic::FixedU128;
	use ulx_bitcoin::{Amount, UnlockStep, UtxoUnlocker};
	use ulx_primitives::{
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinRejectedReason, BitcoinScriptPubkey,
			BitcoinSignature, CompressedBitcoinPubkey, Satoshis, UtxoId, XPubChildNumber,
			XPubFingerprint,
		},
		block_seal::RewardSharing,
		bond::{Bond, BondError, BondExpiration, BondProvider, BondType, VaultProvider},
		BitcoinUtxoEvents, BitcoinUtxoTracker, BondId, PriceProvider, RewardShare,
		UtxoBondedEvents, VaultId,
	};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
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

		type BondEvents: UtxoBondedEvents<Self::AccountId, Self::Balance>;

		/// Utxo tracker for bitcoin
		type BitcoinUtxoTracker: BitcoinUtxoTracker;

		type PriceProvider: PriceProvider<Self::Balance>;

		type BitcoinSignatureVerifier: BitcoinVerifier<Self>;

		/// Bitcoin time provider
		type BitcoinBlockHeight: Get<BitcoinHeight>;

		type VaultProvider: VaultProvider<
			AccountId = Self::AccountId,
			Balance = Self::Balance,
			BlockNumber = BlockNumberFor<Self>,
		>;

		/// Minimum amount for a bond
		#[pallet::constant]
		type MinimumBondAmount: Get<Self::Balance>;

		/// Ulixee blocks per day
		#[pallet::constant]
		type UlixeeBlocksPerDay: Get<BlockNumberFor<Self>>;

		/// Maximum unlocking utxos at a time
		#[pallet::constant]
		type MaxUnlockingUtxos: Get<u32>;
		/// Pallet storage requires bounds, so we have to set a maximum number that can expire in a
		/// single block
		#[pallet::constant]
		type MaxConcurrentlyExpiringBonds: Get<u32>;

		/// The minimum number of satoshis that can be bonded
		#[pallet::constant]
		type MinimumBitcoinBondSatoshis: Get<Satoshis>;

		/// The number of bitcoin blocks a bitcoin bond is locked for
		#[pallet::constant]
		type BitcoinBondDurationBlocks: Get<BitcoinHeight>;

		/// The bitcoin blocks after a bond expires which the vault will be allowed to claim a
		/// bitcoin
		#[pallet::constant]
		type BitcoinBondReclamationBlocks: Get<BitcoinHeight>;

		/// Number of bitcoin blocks a vault has to counter-sign a bitcoin unlock
		#[pallet::constant]
		type UtxoUnlockCosignDeadlineBlocks: Get<BitcoinHeight>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		UnlockingBitcoin,
	}

	#[pallet::storage]
	pub(super) type NextBondId<T: Config> = StorageValue<_, BondId, OptionQuery>;

	/// Bonds by id
	#[pallet::storage]
	pub(super) type BondsById<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BondId,
		Bond<T::AccountId, T::Balance, BlockNumberFor<T>>,
		OptionQuery,
	>;
	/// Completion of mining bonds, upon which funds are returned to the vault
	#[pallet::storage]
	pub(super) type MiningBondCompletions<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<BondId, T::MaxConcurrentlyExpiringBonds>,
		ValueQuery,
	>;

	/// Completion of bitcoin bonds by bitcoin height. Bond funds are returned to the vault if
	/// unlocked or used as the price of the bitcoin
	#[pallet::storage]
	pub(super) type BitcoinBondCompletions<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BitcoinHeight,
		BoundedVec<BondId, T::MaxConcurrentlyExpiringBonds>,
		ValueQuery,
	>;

	/// Stores bitcoin utxos that have requested to be unlocked
	#[pallet::storage]
	pub(super) type UtxosById<T: Config> =
		StorageMap<_, Twox64Concat, UtxoId, UtxoState, OptionQuery>;

	/// Stores Utxos that were not paid back in full
	///
	/// Tuple stores Account, Vault, Still Owed, State
	#[pallet::storage]
	pub(super) type OwedUtxoAggrieved<T: Config> = StorageMap<
		_,
		Twox64Concat,
		UtxoId,
		(T::AccountId, VaultId, T::Balance, UtxoState),
		OptionQuery,
	>;

	/// Utxos that have been requested to be cosigned for unlocking
	#[pallet::storage]
	pub(super) type UtxosPendingUnlock<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<UtxoId, UtxoCosignRequest<T::Balance>, T::MaxUnlockingUtxos>,
		ValueQuery,
	>;

	#[derive(Decode, Encode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	pub struct UtxoState {
		pub bond_id: BondId,
		pub satoshis: Satoshis,
		pub vault_pubkey: CompressedBitcoinPubkey,
		pub vault_xpub_source: (XPubFingerprint, XPubChildNumber),
		pub owner_pubkey: CompressedBitcoinPubkey,
		pub vault_claim_height: BitcoinHeight,
		pub open_claim_height: BitcoinHeight,
		pub register_block: BitcoinHeight,
		pub utxo_script_pubkey: BitcoinCosignScriptPubkey,
		pub is_verified: bool,
	}

	#[derive(Decode, Encode, CloneNoBound, PartialEqNoBound, EqNoBound, RuntimeDebug, TypeInfo)]
	pub struct UtxoCosignRequest<Balance: Clone + Eq + PartialEq + TypeInfo + Codec> {
		pub bitcoin_network_fee: Satoshis,
		pub cosign_due_block: BitcoinHeight,
		pub to_script_pubkey: BitcoinScriptPubkey,
		pub redemption_price: Balance,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		BondCreated {
			vault_id: VaultId,
			bond_id: BondId,
			bond_type: BondType,
			bonded_account_id: T::AccountId,
			utxo_id: Option<UtxoId>,
			amount: T::Balance,
			expiration: BondExpiration<BlockNumberFor<T>>,
		},
		BondCompleted {
			vault_id: VaultId,
			bond_id: BondId,
		},
		BondCanceled {
			vault_id: VaultId,
			bond_id: BondId,
			bonded_account_id: T::AccountId,
			bond_type: BondType,
			returned_fee: T::Balance,
		},
		BitcoinBondBurned {
			vault_id: VaultId,
			bond_id: BondId,
			utxo_id: UtxoId,
			amount_burned: T::Balance,
			amount_held: T::Balance,
			was_utxo_spent: bool,
		},
		BitcoinUtxoCosignRequested {
			bond_id: BondId,
			vault_id: VaultId,
			utxo_id: UtxoId,
		},
		BitcoinUtxoCosigned {
			bond_id: BondId,
			vault_id: VaultId,
			utxo_id: UtxoId,
			signature: BitcoinSignature,
		},
		BitcoinCosignPastDue {
			bond_id: BondId,
			vault_id: VaultId,
			utxo_id: UtxoId,
			compensation_amount: T::Balance,
			compensation_still_owed: T::Balance,
			compensated_account_id: T::AccountId,
		},
		/// An error occurred while completing a bond
		BondCompletionError {
			bond_id: BondId,
			error: DispatchError,
		},
		/// An error occurred while refunding an overdue cosigned bitcoin bond
		CosignOverdueError {
			utxo_id: UtxoId,
			error: DispatchError,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		BondNotFound,
		NoMoreBondIds,
		MinimumBondAmountNotMet,
		/// There are too many bond or bond funds expiring in the given expiration block
		ExpirationAtBlockOverflow,
		InsufficientFunds,
		InsufficientVaultFunds,
		/// The vault does not have enough bitcoins to cover the mining bond
		InsufficientBitcoinsForMining,
		/// The proposed transaction would take the account below the minimum (existential) balance
		AccountWouldGoBelowMinimumBalance,
		VaultClosed,
		/// Funding would result in an overflow of the balance type
		InvalidVaultAmount,
		/// This bitcoin redemption has not been locked in
		BondRedemptionNotLocked,
		/// The bitcoin has passed the deadline to unlock it
		BitcoinUnlockInitiationDeadlinePassed,
		/// The fee for this bitcoin unlock is too high
		BitcoinFeeTooHigh,
		InvalidBondType,
		BitcoinUtxoNotFound,
		/// This bitcoin cosign script couldn't be decoded for unlock
		BitcoinUnableToBeDecodedForUnlock,
		/// This bitcoin signature couldn't be decoded for unlock
		BitcoinSignatureUnableToBeDecoded,
		/// This bitcoin pubkey couldn't be decoded for unlock
		BitcoinPubkeyUnableToBeDecoded,
		/// The cosign signature is not valid for the bitcoin unlock
		BitcoinInvalidCosignature,
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
		GenericBondError(BondError),
	}

	impl<T> From<BondError> for Error<T> {
		fn from(e: BondError) -> Error<T> {
			match e {
				BondError::BondNotFound => Error::<T>::BondNotFound,
				BondError::NoMoreBondIds => Error::<T>::NoMoreBondIds,
				BondError::MinimumBondAmountNotMet => Error::<T>::MinimumBondAmountNotMet,
				BondError::ExpirationAtBlockOverflow => Error::<T>::ExpirationAtBlockOverflow,
				BondError::InsufficientFunds => Error::<T>::InsufficientFunds,
				BondError::ExpirationTooSoon => Error::<T>::ExpirationTooSoon,
				BondError::NoPermissions => Error::<T>::NoPermissions,
				BondError::HoldUnexpectedlyModified => Error::<T>::HoldUnexpectedlyModified,
				BondError::UnrecoverableHold => Error::<T>::UnrecoverableHold,
				BondError::VaultNotFound => Error::<T>::VaultNotFound,
				BondError::FeeExceedsBondAmount => Error::<T>::FeeExceedsBondAmount,
				BondError::InsufficientVaultFunds => Error::<T>::InsufficientVaultFunds,
				BondError::InsufficientBitcoinsForMining =>
					Error::<T>::InsufficientBitcoinsForMining,
				BondError::VaultClosed => Error::<T>::VaultClosed,
				BondError::AccountWouldBeBelowMinimum =>
					Error::<T>::AccountWouldGoBelowMinimumBalance,
				BondError::InvalidBitcoinScript => Error::<T>::InvalidBitcoinScript,
				_ => Error::<T>::GenericBondError(e),
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			let bond_completions = MiningBondCompletions::<T>::take(block_number);
			for bond_id in bond_completions {
				let res = with_storage_layer(|| Self::bond_completed(bond_id));
				if let Err(e) = res {
					log::error!( target: LOG_TARGET, "Mining bond id {:?} failed to `complete` {:?}", bond_id, e);
					Self::deposit_event(Event::<T>::BondCompletionError {
						bond_id,
						error: e.into(),
					});
				}
			}

			let mut overdue = vec![];
			let bitcoin_block_height = T::BitcoinBlockHeight::get();
			<UtxosPendingUnlock<T>>::mutate(|pending| {
				pending.retain(|id, x| {
					if x.cosign_due_block > bitcoin_block_height {
						return true;
					}
					overdue.push((*id, x.redemption_price));
					false
				});
			});

			for (utxo_id, redemption_amount) in overdue {
				let res =
					with_storage_layer(|| Self::cosign_bitcoin_overdue(utxo_id, redemption_amount));
				if let Err(e) = res {
					log::error!( target: LOG_TARGET, "Bitcoin utxo id {:?} failed to `cosign` {:?}", utxo_id, e);
					Self::deposit_event(Event::<T>::CosignOverdueError {
						utxo_id,
						error: e.into(),
					});
				}
			}

			let bitcoin_bond_completions = BitcoinBondCompletions::<T>::take(bitcoin_block_height);
			for bond_id in bitcoin_bond_completions {
				let res = with_storage_layer(|| Self::bond_completed(bond_id));
				if let Err(e) = res {
					log::error!( target: LOG_TARGET, "Bitcoin bond id {:?} failed to `complete` {:?}", bond_id, e);
					Self::deposit_event(Event::<T>::BondCompletionError {
						bond_id,
						error: e.into(),
					});
				}
			}
			T::DbWeight::get().reads_writes(2, 1)
		}

		fn on_finalize(_: BlockNumberFor<T>) {}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Bond a bitcoin. This will create a bond for the submitting account and log the Bitcoin
		/// Script hash to Events. A bondee must create the UTXO in order to be added to the Bitcoin
		/// Mint line.
		///
		/// The pubkey submitted here will be used to create a script pubkey that will be used in a
		/// timelock multisig script to lock the bitcoin.
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn bond_bitcoin(
			origin: OriginFor<T>,
			vault_id: VaultId,
			#[pallet::compact] satoshis: Satoshis,
			bitcoin_pubkey: CompressedBitcoinPubkey,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;

			ensure!(
				satoshis >= T::MinimumBitcoinBondSatoshis::get(),
				Error::<T>::InsufficientSatoshisBonded
			);

			let vault_claim_height =
				T::BitcoinBlockHeight::get() + T::BitcoinBondDurationBlocks::get();
			let open_claim_height = vault_claim_height + T::BitcoinBondReclamationBlocks::get();

			let amount = T::PriceProvider::get_bitcoin_argon_price(satoshis)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?;

			let (total_fee, prepaid_fee) = T::VaultProvider::bond_funds(
				vault_id,
				amount,
				BondType::Bitcoin,
				// charge in 1 year of blocks (even though we'll expire off bitcoin time)
				T::UlixeeBlocksPerDay::get() * 365u32.into(),
				&account_id,
			)
			.map_err(Error::<T>::from)?;
			ensure!(total_fee <= amount, Error::<T>::FeeExceedsBondAmount);

			let utxo_id = T::BitcoinUtxoTracker::new_utxo_id();

			let (vault_xpub, script_pubkey) = T::VaultProvider::create_utxo_script_pubkey(
				vault_id,
				utxo_id,
				bitcoin_pubkey,
				vault_claim_height,
				open_claim_height,
			)
			.map_err(|_| Error::<T>::InvalidBitcoinScript)?;

			let vault_pubkey = vault_xpub.public_key;
			let vault_xpub_source = (vault_xpub.parent_fingerprint, vault_xpub.child_number);

			T::BitcoinUtxoTracker::watch_for_utxo(
				utxo_id,
				script_pubkey,
				satoshis,
				// translate back into a time with millis
				vault_claim_height,
			)?;

			let bond_id = Self::create_bond(
				vault_id,
				account_id,
				BondType::Bitcoin,
				amount,
				BondExpiration::BitcoinBlock(vault_claim_height),
				total_fee,
				prepaid_fee,
				Some(utxo_id),
			)
			.map_err(Error::<T>::from)?;

			UtxosById::<T>::insert(
				utxo_id,
				UtxoState {
					bond_id,
					satoshis,
					vault_pubkey,
					vault_xpub_source,
					owner_pubkey: bitcoin_pubkey,
					vault_claim_height,
					open_claim_height,
					register_block: T::BitcoinBlockHeight::get(),
					utxo_script_pubkey: script_pubkey,
					is_verified: false,
				},
			);

			Ok(())
		}

		/// Submitted by a Bitcoin holder to trigger the unlock of their Bitcoin. A transaction
		/// spending the UTXO from the given bond should be pre-created so that the sighash can be
		/// submitted here. The vault operator will have 10 days to counter-sign the transaction. It
		/// will be published with the public key as a BitcoinUtxoCosigned Event.
		///
		/// Owner must submit a script pubkey and also a fee to pay to the bitcoin network.
		#[pallet::call_index(4)]
		#[pallet::weight(0)]
		pub fn unlock_bitcoin_bond(
			origin: OriginFor<T>,
			bond_id: BondId,
			to_script_pubkey: BitcoinScriptPubkey,
			bitcoin_network_fee: Satoshis,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let bond = BondsById::<T>::get(bond_id).ok_or(Error::<T>::BondNotFound)?;
			ensure!(bond.bond_type == BondType::Bitcoin, Error::<T>::NoPermissions);
			ensure!(bond.bonded_account_id == who, Error::<T>::NoPermissions);
			let expiration = match bond.expiration {
				BondExpiration::BitcoinBlock(vault_claim_height) => vault_claim_height,
				_ => return Err(Error::<T>::InvalidBondType.into()),
			};
			let unlock_due_date =
				expiration.saturating_sub(T::UtxoUnlockCosignDeadlineBlocks::get());
			ensure!(
				T::BitcoinBlockHeight::get() <= unlock_due_date,
				Error::<T>::BitcoinUnlockInitiationDeadlinePassed
			);

			let utxo_id = bond.utxo_id.ok_or(Error::<T>::InvalidBondType)?;

			let utxo = <UtxosById<T>>::get(utxo_id).ok_or(Error::<T>::BitcoinUtxoNotFound)?;
			// If this is a confirmed utxo, we require the unlock price to be paid
			if utxo.is_verified {
				ensure!(bitcoin_network_fee < utxo.satoshis, Error::<T>::BitcoinFeeTooHigh);
				let redemption_price = Self::get_redemption_price(&utxo.satoshis)?.min(bond.amount);
				let cosign_due_block =
					T::UtxoUnlockCosignDeadlineBlocks::get() + T::BitcoinBlockHeight::get();

				// hold funds until the utxo is seen in the chain
				let balance = T::Currency::balance(&who);
				ensure!(
					balance.saturating_sub(redemption_price) >= T::Currency::minimum_balance(),
					Error::<T>::AccountWouldGoBelowMinimumBalance
				);

				T::Currency::hold(&HoldReason::UnlockingBitcoin.into(), &who, redemption_price)
					.map_err(|e| match e {
						Token(TokenError::BelowMinimum) =>
							Error::<T>::AccountWouldGoBelowMinimumBalance,
						_ => Error::<T>::InsufficientFunds,
					})?;
				frame_system::Pallet::<T>::inc_providers(&who);

				<UtxosPendingUnlock<T>>::try_mutate(|a| {
					a.try_insert(
						utxo_id,
						UtxoCosignRequest {
							bitcoin_network_fee,
							cosign_due_block,
							to_script_pubkey,
							redemption_price,
						},
					)
				})
				.map_err(|_| Error::<T>::ExpirationAtBlockOverflow)?;

				Self::deposit_event(Event::<T>::BitcoinUtxoCosignRequested {
					bond_id,
					vault_id: bond.vault_id,
					utxo_id,
				});
			} else {
				<Self as BondProvider>::cancel_bond(bond_id).map_err(Error::<T>::from)?;
			}
			Ok(())
		}

		/// Submitted by a Vault operator to cosign the unlock of a bitcoin utxo. The Bitcoin owner
		/// unlock fee will be burned, and the bond will be allowed to expire without penalty.
		///
		/// This is submitted as a no-fee transaction off chain to allow keys to remain in cold
		/// wallets.
		#[pallet::call_index(5)]
		#[pallet::weight((0, DispatchClass::Operational))]
		pub fn cosign_bitcoin_unlock(
			origin: OriginFor<T>,
			bond_id: BondId,
			signature: BitcoinSignature,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let bond = BondsById::<T>::get(bond_id).ok_or(Error::<T>::BondNotFound)?;
			ensure!(bond.bond_type == BondType::Bitcoin, Error::<T>::NoPermissions);

			let utxo_id = bond.utxo_id.ok_or(Error::<T>::InvalidBondType)?;
			let vault_id = bond.vault_id;

			let vault = T::VaultProvider::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;
			ensure!(vault.operator_account_id == who, Error::<T>::NoPermissions);
			let request = UtxosPendingUnlock::<T>::mutate(|a| a.remove(&utxo_id))
				.ok_or(Error::<T>::BondRedemptionNotLocked)?;

			let utxo_state = <UtxosById<T>>::get(utxo_id).ok_or(Error::<T>::BitcoinUtxoNotFound)?;
			let utxo_ref =
				T::BitcoinUtxoTracker::get(utxo_id).ok_or(Error::<T>::BitcoinUtxoNotFound)?;

			let unlocker = UtxoUnlocker::new(
				utxo_state
					.vault_pubkey
					.try_into()
					.map_err(|_| Error::<T>::BitcoinPubkeyUnableToBeDecoded)?,
				utxo_state
					.owner_pubkey
					.try_into()
					.map_err(|_| Error::<T>::BitcoinPubkeyUnableToBeDecoded)?,
				utxo_state.register_block,
				utxo_state.vault_claim_height,
				utxo_state.open_claim_height,
				utxo_state.satoshis,
				utxo_ref.txid.into(),
				utxo_ref.output_index,
				UnlockStep::VaultCosign,
				Amount::from_sat(request.bitcoin_network_fee),
				request.to_script_pubkey.into(),
			)
			.map_err(|_| Error::<T>::BitcoinUnableToBeDecodedForUnlock)?;

			T::BitcoinSignatureVerifier::verify_signature(
				unlocker,
				utxo_state.vault_pubkey,
				&signature,
			)?;

			// burn the owner's held funds
			let _ = T::Currency::burn_held(
				&HoldReason::UnlockingBitcoin.into(),
				&bond.bonded_account_id,
				request.redemption_price,
				Precision::Exact,
				Fortitude::Force,
			)?;
			frame_system::Pallet::<T>::dec_providers(&who)?;

			T::BitcoinUtxoTracker::unwatch(utxo_id);
			<UtxosById<T>>::take(utxo_id);

			Self::deposit_event(Event::BitcoinUtxoCosigned {
				bond_id,
				vault_id,
				utxo_id,
				signature,
			});

			// no fee for cosigning
			Ok(Pays::No.into())
		}
	}

	impl<T: Config> BitcoinUtxoEvents for Pallet<T> {
		fn utxo_verified(utxo_id: UtxoId) -> DispatchResult {
			UtxosById::<T>::mutate(utxo_id, |a| {
				if let Some(utxo_state) = a {
					utxo_state.is_verified = true;
					let bond =
						BondsById::<T>::get(utxo_state.bond_id).ok_or(Error::<T>::BondNotFound)?;
					T::BondEvents::utxo_bonded(utxo_id, &bond.bonded_account_id, bond.amount)?;
				} else {
					warn!( target: LOG_TARGET, "Verified utxo_id {:?} not found", utxo_id);
				}
				Ok::<(), DispatchError>(())
			})
		}

		fn utxo_rejected(utxo_id: UtxoId, _reason: BitcoinRejectedReason) -> DispatchResult {
			if let Some(utxo_state) = UtxosById::<T>::take(utxo_id) {
				Self::cancel_bond(utxo_state.bond_id).map_err(Error::<T>::from)?;
			}
			Ok(())
		}

		fn utxo_spent(utxo_id: UtxoId) -> DispatchResult {
			if let Some(utxo) = UtxosById::<T>::take(utxo_id) {
				Self::burn_bitcoin_bond(utxo_id, utxo, true)
			} else {
				Ok(())
			}
		}

		fn utxo_expired(utxo_id: UtxoId) -> DispatchResult {
			if let Some(utxo) = UtxosById::<T>::take(utxo_id) {
				Self::burn_bitcoin_bond(utxo_id, utxo, false)
			} else {
				Ok(())
			}
		}
	}

	impl<T: Config> Pallet<T> {
		#[allow(clippy::too_many_arguments)]
		fn create_bond(
			vault_id: VaultId,
			account_id: T::AccountId,
			bond_type: BondType,
			amount: T::Balance,
			expiration: BondExpiration<BlockNumberFor<T>>,
			total_fee: T::Balance,
			prepaid_fee: T::Balance,
			utxo_id: Option<UtxoId>,
		) -> Result<BondId, BondError> {
			let bond_id = NextBondId::<T>::get().unwrap_or(1);

			let next_bond_id = bond_id.increment().ok_or(BondError::NoMoreBondIds)?;
			NextBondId::<T>::set(Some(next_bond_id));

			let bond = Bond {
				vault_id,
				utxo_id,
				bond_type: bond_type.clone(),
				bonded_account_id: account_id.clone(),
				amount,
				expiration: expiration.clone(),
				total_fee,
				start_block: frame_system::Pallet::<T>::block_number(),
				prepaid_fee,
			};
			BondsById::<T>::set(bond_id, Some(bond));
			match expiration {
				BondExpiration::UlixeeBlock(block) => {
					MiningBondCompletions::<T>::try_mutate(block, |a| {
						a.try_push(bond_id).map_err(|_| BondError::ExpirationAtBlockOverflow)
					})?;
				},
				BondExpiration::BitcoinBlock(block) => {
					BitcoinBondCompletions::<T>::try_mutate(block, |a| {
						a.try_push(bond_id).map_err(|_| BondError::ExpirationAtBlockOverflow)
					})?;
				},
			}

			Self::deposit_event(Event::BondCreated {
				vault_id,
				bond_id,
				bonded_account_id: account_id,
				utxo_id,
				amount,
				expiration,
				bond_type,
			});
			Ok(bond_id)
		}

		/// Return bonded funds to the vault and complete the bond
		fn bond_completed(bond_id: BondId) -> DispatchResult {
			let bond = BondsById::<T>::get(bond_id).ok_or(Error::<T>::BondNotFound)?;
			Self::remove_bond_completion(bond_id, bond.expiration.clone());

			if bond.bond_type == BondType::Bitcoin {
				let utxo_id = bond.utxo_id.ok_or(Error::<T>::InvalidBondType)?;
				if let Some(utxo) = <UtxosById<T>>::take(utxo_id) {
					Self::burn_bitcoin_bond(utxo_id, utxo, false)?;
					BondsById::<T>::remove(bond_id);
					return Ok(());
				}
			}
			// reload bond
			let bond = BondsById::<T>::get(bond_id).ok_or(Error::<T>::BondNotFound)?;
			T::VaultProvider::release_bonded_funds(&bond).map_err(Error::<T>::from)?;
			Self::deposit_event(Event::BondCompleted { vault_id: bond.vault_id, bond_id });
			BondsById::<T>::remove(bond_id);
			Ok(())
		}

		fn burn_bitcoin_bond(utxo_id: UtxoId, utxo: UtxoState, is_spent: bool) -> DispatchResult {
			let bond_id = utxo.bond_id;

			if !utxo.is_verified {
				Self::cancel_bond(bond_id).map_err(Error::<T>::from)?;
				return Ok(());
			}
			let mut bond = BondsById::<T>::get(bond_id).ok_or(Error::<T>::BondNotFound)?;

			// burn the current redemption price from the bond
			let amount_to_burn = Self::get_redemption_price(&utxo.satoshis)
				.unwrap_or(bond.amount)
				.min(bond.amount);

			T::VaultProvider::burn_vault_bitcoin_funds(&bond, amount_to_burn)
				.map_err(Error::<T>::from)?;
			let vault_id = bond.vault_id;
			bond.amount = bond.amount.saturating_sub(amount_to_burn);

			Self::deposit_event(Event::BitcoinBondBurned {
				vault_id,
				bond_id,
				utxo_id,
				amount_burned: amount_to_burn,
				amount_held: bond.amount,
				was_utxo_spent: is_spent,
			});
			BondsById::<T>::insert(bond_id, bond);

			T::BitcoinUtxoTracker::unwatch(utxo_id);

			Ok(())
		}

		/// Call made during the on_initialize to implement cosign overdue penalties.
		pub(crate) fn cosign_bitcoin_overdue(
			utxo_id: UtxoId,
			redemption_amount_held: T::Balance,
		) -> DispatchResult {
			let utxo = <UtxosById<T>>::take(utxo_id).ok_or(Error::<T>::BitcoinUtxoNotFound)?;
			let bond_id = utxo.bond_id;
			let mut bond = BondsById::<T>::get(bond_id).ok_or(Error::<T>::BondNotFound)?;
			let vault_id = bond.vault_id;

			let market_price = T::PriceProvider::get_bitcoin_argon_price(utxo.satoshis)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?;

			let repaid = T::VaultProvider::compensate_lost_bitcoin(&bond, market_price)
				.map_err(Error::<T>::from)?;

			bond.amount = bond.amount.saturating_sub(market_price);
			let still_owed = market_price.saturating_sub(repaid);
			if still_owed > 0u128.into() {
				<OwedUtxoAggrieved<T>>::insert(
					utxo_id,
					(bond.bonded_account_id.clone(), vault_id, still_owed, utxo),
				);
			}

			T::Currency::release(
				&HoldReason::UnlockingBitcoin.into(),
				&bond.bonded_account_id,
				redemption_amount_held,
				Precision::Exact,
			)?;
			frame_system::Pallet::<T>::dec_providers(&bond.bonded_account_id)?;

			Self::deposit_event(Event::BitcoinCosignPastDue {
				vault_id,
				bond_id,
				utxo_id,
				compensation_amount: repaid,
				compensation_still_owed: still_owed,
				compensated_account_id: bond.bonded_account_id.clone(),
			});
			T::BitcoinUtxoTracker::unwatch(utxo_id);
			BondsById::<T>::insert(bond_id, bond);

			Ok(())
		}
		fn remove_bond_completion(bond_id: BondId, expiration: BondExpiration<BlockNumberFor<T>>) {
			match expiration {
				BondExpiration::BitcoinBlock(completion_block) => {
					if !BitcoinBondCompletions::<T>::contains_key(completion_block) {
						return;
					}
					BitcoinBondCompletions::<T>::mutate(completion_block, |bonds| {
						if let Some(index) = bonds.iter().position(|b| *b == bond_id) {
							bonds.remove(index);
						}
					});
				},
				BondExpiration::UlixeeBlock(completion_block) => {
					if !MiningBondCompletions::<T>::contains_key(completion_block) {
						return;
					}
					MiningBondCompletions::<T>::mutate(completion_block, |bonds| {
						if let Some(index) = bonds.iter().position(|b| *b == bond_id) {
							bonds.remove(index);
						}
					});
				},
			}
		}

		pub fn get_redemption_price(satoshis: &Satoshis) -> Result<T::Balance, Error<T>> {
			const REDEMPTION_MULTIPLIER: FixedU128 = FixedU128::from_rational(713, 1000);
			const REDEMPTION_ADDON: FixedU128 = FixedU128::from_rational(274, 1000);
			let mut price: u128 = T::PriceProvider::get_bitcoin_argon_price(*satoshis)
				.ok_or(Error::<T>::NoBitcoinPricesAvailable)?
				.unique_saturated_into();
			let cpi = T::PriceProvider::get_argon_cpi().unwrap_or_default();

			if cpi.is_positive() {
				let argon_price =
					T::PriceProvider::get_latest_argon_price_in_us_cents().unwrap_or_default();

				let multiplier = REDEMPTION_MULTIPLIER
					.saturating_mul(argon_price)
					.saturating_add(REDEMPTION_ADDON);

				// Apply the formula: R = (Pb / Pa) * (0.713 * Pa + 0.274)
				// Pa should be in a float value (1.01)
				// `price` is already (Pb / Pa)
				// The redemption price of the argon allocates fluctuating incentives based on how
				// fast the dip should be incentivized to be capitalized on.
				let adjusted_price = FixedU128::saturating_from_integer(price);
				let fixed_price =
					multiplier.saturating_mul(adjusted_price).into_inner() / FixedU128::accuracy();
				price = fixed_price.unique_saturated_into();
			};

			Ok(price.into())
		}
	}

	impl<T: Config> BondProvider for Pallet<T> {
		type Balance = T::Balance;
		type AccountId = T::AccountId;
		type BlockNumber = BlockNumberFor<T>;

		fn bond_mining_slot(
			vault_id: VaultId,
			account_id: Self::AccountId,
			amount: Self::Balance,
			bond_until_block: Self::BlockNumber,
		) -> Result<(BondId, Option<RewardSharing<Self::AccountId>>), BondError> {
			ensure!(amount >= T::MinimumBondAmount::get(), BondError::MinimumBondAmountNotMet);

			let block_number = frame_system::Pallet::<T>::block_number();
			ensure!(bond_until_block > block_number, BondError::ExpirationTooSoon);

			let (total_fee, prepaid_fee) = T::VaultProvider::bond_funds(
				vault_id,
				amount,
				BondType::Mining,
				bond_until_block - block_number,
				&account_id,
			)?;

			let bond_id = Self::create_bond(
				vault_id,
				account_id,
				BondType::Mining,
				amount,
				BondExpiration::UlixeeBlock(bond_until_block),
				total_fee,
				prepaid_fee,
				None,
			)?;
			let vault = T::VaultProvider::get(vault_id).ok_or(BondError::VaultNotFound)?;
			Ok((
				bond_id,
				if vault.mining_reward_sharing_percent_take > RewardShare::zero() {
					Some(RewardSharing {
						percent_take: vault.mining_reward_sharing_percent_take,
						account_id: vault.operator_account_id.clone(),
					})
				} else {
					None
				},
			))
		}

		fn cancel_bond(bond_id: BondId) -> Result<(), BondError> {
			let bond = BondsById::<T>::take(bond_id).ok_or(BondError::BondNotFound)?;

			let returned_fee = T::VaultProvider::release_bonded_funds(&bond)?;

			Self::deposit_event(Event::BondCanceled {
				vault_id: bond.vault_id,
				bond_id,
				bonded_account_id: bond.bonded_account_id.clone(),
				bond_type: bond.bond_type,
				returned_fee,
			});
			Self::remove_bond_completion(bond_id, bond.expiration.clone());
			if let Some(utxo_id) = bond.utxo_id {
				UtxosById::<T>::take(utxo_id);
				T::BitcoinUtxoTracker::unwatch(utxo_id);
			}

			Ok(())
		}
	}
}

pub trait BitcoinVerifier<T: Config> {
	fn verify_signature(
		utxo_unlocker: UtxoUnlocker,
		pubkey: CompressedBitcoinPubkey,
		signature: &BitcoinSignature,
	) -> Result<bool, DispatchError> {
		let is_ok = utxo_unlocker.verify_signature_raw(pubkey, signature).map_err(|e| match e {
			ulx_bitcoin::Error::InvalidCompressPubkeyBytes =>
				Error::<T>::BitcoinPubkeyUnableToBeDecoded,
			ulx_bitcoin::Error::InvalidSignatureBytes =>
				Error::<T>::BitcoinSignatureUnableToBeDecoded,
			_ => Error::<T>::BitcoinInvalidCosignature,
		})?;
		Ok(is_ok)
	}
}
