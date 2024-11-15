#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use crate::ismp_module::{Asset, EvmChain};
use alloc::{vec, vec::Vec};
pub use argon_notary_audit::VerifyError as NotebookVerifyError;
use argon_primitives::{notary::NotaryId, tick::Tick, TransferToLocalchainId};
use codec::{Decode, Encode};
use core::fmt::Debug;
use ismp::host::StateMachine;
pub use pallet::*;
use scale_info::TypeInfo;
use sp_core::H160;
use sp_runtime::RuntimeDebug;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod ismp_module;
pub mod weights;
pub use ismp_module::{
	ISMP_KUSAMA_PARACHAIN_ID, ISMP_PASEO_PARACHAIN_ID, ISMP_POLKADOT_PARACHAIN_ID,
};

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use argon_primitives::{
		notebook::{ChainTransfer, NotebookHeader},
		tick::Tick,
		BurnEventHandler, ChainTransferLookup, NotebookEventHandler, NotebookNumber,
		NotebookProvider,
	};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, Mutate},
			tokens::{Fortitude, Precision, Preservation},
			Incrementable,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_core::{crypto::AccountId32, H256};
	use sp_runtime::traits::{AccountIdConversion, AtLeast32BitUnsigned};

	use crate::ismp_module::Body;
	use ismp::{dispatcher::IsmpDispatcher, host::StateMachine};
	use sp_core::Get;
	use token_gateway_primitives::{
		GatewayAssetRegistration, GatewayAssetUpdate, RemoteERC6160AssetRegistration,
	};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountId = AccountId32, Hash = H256>
		+ pallet_ismp::Config
		+ pallet_hyperbridge::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		type Argon: Mutate<
			<Self as frame_system::Config>::AccountId,
			Balance = <Self as Config>::Balance,
		>;
		type OwnershipTokens: Mutate<
			<Self as frame_system::Config>::AccountId,
			Balance = <Self as Config>::Balance,
		>;

		/// The [`IsmpDispatcher`] for dispatching cross-chain requests
		type Dispatcher: IsmpDispatcher<
			Account = <Self as frame_system::Config>::AccountId,
			Balance = <Self as Config>::Balance,
		>;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Member
			+ Copy
			+ MaybeSerializeDeserialize
			+ Debug
			+ Default
			+ From<u128>
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// The minimum allowable balance for an account to hold
		type ExistentialDeposit: Get<<Self as Config>::Balance>;

		type NotebookProvider: NotebookProvider;
		type NotebookTick: Get<Tick>;
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
	pub(super) type NextTransferId<T: Config> =
		StorageValue<_, TransferToLocalchainId, OptionQuery>;

	#[pallet::storage]
	pub(super) type PendingTransfersOut<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		TransferToLocalchainId,
		QueuedTransferOut<<T as frame_system::Config>::AccountId, <T as Config>::Balance>,
		OptionQuery,
	>;

	#[pallet::storage]
	pub(super) type ExpiringTransfersOutByNotary<T: Config> = StorageDoubleMap<
		Hasher1 = Twox64Concat,
		Hasher2 = Twox64Concat,
		Key1 = NotaryId,
		Key2 = Tick,
		Value = BoundedVec<TransferToLocalchainId, T::MaxPendingTransfersOutPerBlock>,
		QueryKind = ValueQuery,
	>;

	#[pallet::storage]
	pub(super) type TransfersUsedInBlockNotebooks<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<
			(<T as frame_system::Config>::AccountId, <T as frame_system::Config>::Nonce),
			T::MaxPendingTransfersOutPerBlock,
		>,
		ValueQuery,
	>;

	/// The token gateway addresses on different chains
	#[pallet::storage]
	pub type ActiveEvmDestinations<T: Config> =
		StorageValue<_, BoundedVec<EvmChain, ConstU32<20>>, ValueQuery>;

	/// The token gateway addresses on different chains
	#[pallet::storage]
	pub type TokenGatewayAddresses<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachine, Vec<u8>, OptionQuery>;

	/// Should we use test networks for Polkadot and Ethereum
	#[pallet::storage]
	pub type UseTestNetworks<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// The admin of this side of the token gateway
	#[pallet::storage]
	pub(super) type TokenAdmin<T: Config> =
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
		/// An asset has been sent to an EVM
		TransferToEvm {
			/// Source account on the relaychain
			from: <T as frame_system::Config>::AccountId,
			/// beneficiary account on destination
			to: H160,
			/// Amount transferred
			amount: <T as Config>::Balance,
			/// Destination chain
			evm_chain: EvmChain,
			/// asset
			asset: Asset,
			/// Request commitment
			commitment: H256,
		},
		/// An asset has been refunded and transferred back to the source account
		TransferToEvmExpired {
			/// The original sender
			from: <T as frame_system::Config>::AccountId,
			/// beneficiary account on destination
			to: H160,
			/// Amount transferred
			amount: <T as Config>::Balance,
			/// Destination chain
			evm_chain: EvmChain,
			/// asset
			asset: Asset,
		},
		/// An asset has been received from an EVM chain
		TransferFromEvm {
			/// the source account
			from: H160,
			/// beneficiary account
			to: <T as frame_system::Config>::AccountId,
			/// asset
			asset: Asset,
			/// Amount transferred
			amount: <T as Config>::Balance,
			/// Source chain
			evm_chain: EvmChain,
		},
		/// ERC6160 asset creation request dispatched to hyperbridge
		ERC6160AssetRegistrationDispatched {
			/// Request commitment
			commitment: H256,
			/// asset
			asset: Asset,
			/// added chains
			added_chains: BoundedVec<StateMachine, ConstU32<20>>,
			/// removed chains
			removed_chains: BoundedVec<StateMachine, ConstU32<20>>,
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
		/// An error was encountered trying to send a transfer to an EVM
		FailedToTransferToEvm,
		/// This account is not a token admin
		NotATokenAdmin,
		/// ERC6160 asset registration failed
		Erc6160RegistrationFailed,
		/// ERC6160 asset already registered
		Erc6160AlreadyRegistered,
		/// Coprocessor not configured
		CoprocessorNotConfigured,
		/// Invalid Destination Chain
		InvalidEvmChain,
		/// Evm Chain is not supported yet
		EvmChainNotSupported,
		/// Evm Chain doesn't have the proper configuration setup
		EvmChainNotConfigured,
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub token_admin: Option<<T as frame_system::Config>::AccountId>,
		pub use_evm_test_networks: bool,
		#[serde(skip)]
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if let Some(admin) = self.token_admin.clone() {
				TokenAdmin::<T>::put(admin);
			}
			UseTestNetworks::<T>::put(self.use_evm_test_networks);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		[u8; 32]: From<<T as frame_system::Config>::AccountId>,
		u128: From<<T as pallet_ismp::Config>::Balance>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn send_to_localchain(
			origin: OriginFor<T>,
			#[pallet::compact] amount: <T as Config>::Balance,
			notary_id: NotaryId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

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

			let expiration_tick: Tick = T::NotebookTick::get() + T::TransferExpirationTicks::get();

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

		/// Send argons to a remote EVM based chain. Available destinations are specified in the
		/// `ActiveEvmDestinations` storage item.
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn send_to_evm_chain(
			origin: OriginFor<T>,
			params: TransferToEvm<<T as Config>::Balance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let asset_id = params.asset.asset_id();
			let is_evm_supported = ActiveEvmDestinations::<T>::get().contains(&params.evm_chain);
			ensure!(is_evm_supported, Error::<T>::EvmChainNotSupported);
			let is_testnet = UseTestNetworks::<T>::get();
			let dest = params.evm_chain.get_state_machine(is_testnet);

			// Hold funds in the pallet account
			if params.asset == Asset::Argon {
				T::Argon::transfer(
					&who,
					&Self::pallet_account(),
					params.amount,
					Preservation::Preserve,
				)?;
			} else {
				T::OwnershipTokens::transfer(
					&who,
					&Self::pallet_account(),
					params.amount,
					Preservation::Preserve,
				)?;
			}

			let body = Body::send_to_evm(params.amount, asset_id, who.clone(), params.recipient);

			let commitment = Self::dispatch_request(
				body,
				dest,
				who.clone(),
				params.timeout,
				params.relayer_fee,
			)?;

			Self::deposit_event(Event::<T>::TransferToEvm {
				from: who,
				to: params.recipient,
				evm_chain: params.evm_chain,
				amount: params.amount,
				asset: params.asset,
				commitment,
			});
			Ok(())
		}

		/// One time api to register assets for cross chain transfers
		///
		/// # Arguments
		/// `chains` - Each chain and its corresponding token gateway address
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn register_hyperbridge_assets(
			origin: OriginFor<T>,
			chains: BoundedVec<(StateMachine, Vec<u8>), ConstU32<20>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				who == <TokenAdmin<T>>::get().expect("No token admin. Should be set in genesis"),
				Error::<T>::NotATokenAdmin,
			);

			let is_empty = TokenGatewayAddresses::<T>::iter_keys().collect::<Vec<_>>().is_empty();
			ensure!(is_empty, Error::<T>::Erc6160AlreadyRegistered);

			let mut state_machines = vec![];
			let mut evm_chains = vec![];

			let is_testnet = UseTestNetworks::<T>::get();
			for (sm, gateway) in chains {
				state_machines.push(sm);
				if let StateMachine::Evm(id) = sm {
					let evm_chain =
						EvmChain::try_from(id, is_testnet).ok_or(Error::<T>::InvalidEvmChain)?;
					evm_chains.push(evm_chain);
				}
				TokenGatewayAddresses::<T>::insert(sm, gateway.clone());
			}
			ActiveEvmDestinations::<T>::put(BoundedVec::truncate_from(evm_chains));

			let minimum: u128 = T::ExistentialDeposit::get().into();
			for asset in [Asset::Argon, Asset::OwnershipToken] {
				Self::update_asset_registration(
					RemoteERC6160AssetRegistration::CreateAsset(GatewayAssetRegistration {
						name: asset.name(),
						symbol: asset.symbol(),
						chains: state_machines.clone(),
						minimum_balance: Some(minimum),
					}),
					&who,
					asset,
				)?;
			}
			Ok(())
		}

		/// Set the asset registration for cross chain transfers
		///
		/// # Arguments
		/// `chains` - Each chain and its corresponding token gateway address
		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn update_hyperbridge_assets(
			origin: OriginFor<T>,
			chains: BoundedVec<(StateMachine, Vec<u8>), ConstU32<20>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				who == <TokenAdmin<T>>::get().expect("No token admin. Should be set in genesis"),
				Error::<T>::NotATokenAdmin,
			);

			let is_testnet = UseTestNetworks::<T>::get();
			let mut added_chains = vec![];
			let mut removed_chains = vec![];
			let mut evm_chains = vec![];
			for chain in TokenGatewayAddresses::<T>::iter_keys() {
				if !chains.iter().any(|(sm, _)| sm == &chain) {
					removed_chains.push(chain);
				}
			}
			for (chain, address) in &chains {
				if !TokenGatewayAddresses::<T>::contains_key(chain) {
					added_chains.push(*chain);
					TokenGatewayAddresses::<T>::insert(chain, address);
				}

				if let StateMachine::Evm(id) = chain {
					let evm_chain =
						EvmChain::try_from(*id, is_testnet).ok_or(Error::<T>::InvalidEvmChain)?;
					evm_chains.push(evm_chain);
				}
			}
			ActiveEvmDestinations::<T>::put(BoundedVec::truncate_from(evm_chains));
			for chain in &removed_chains {
				TokenGatewayAddresses::<T>::remove(chain);
			}
			for asset in [Asset::Argon, Asset::OwnershipToken] {
				Self::update_asset_registration(
					RemoteERC6160AssetRegistration::UpdateAsset(GatewayAssetUpdate {
						asset_id: asset.asset_id().0.into(),
						add_chains: BoundedVec::truncate_from(added_chains.clone()),
						remove_chains: BoundedVec::truncate_from(removed_chains.clone()),
						new_admins: Default::default(),
					}),
					&who,
					asset,
				)?;
			}

			Ok(())
		}

		/// This api will re-assign admins for ERC6160 accounts on the TokenGateway.sol contracts
		/// created by Hyperbridge.
		///
		/// This api is only used to disconnect from hyperbridge.
		#[pallet::call_index(4)]
		#[pallet::weight(0)]
		pub fn replace_hyperbridge_admins(
			origin: OriginFor<T>,
			new_admins: BoundedVec<(StateMachine, H160), ConstU32<20>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				who == <TokenAdmin<T>>::get().expect("No token admin. Should be set in genesis"),
				Error::<T>::NotATokenAdmin,
			);

			for asset in [Asset::Argon, Asset::OwnershipToken] {
				Self::update_asset_registration(
					RemoteERC6160AssetRegistration::UpdateAsset(GatewayAssetUpdate {
						asset_id: asset.asset_id().0.into(),
						add_chains: Default::default(),
						remove_chains: Default::default(),
						new_admins: BoundedVec::truncate_from(new_admins.to_vec()),
					}),
					&who,
					asset,
				)?;
			}

			Ok(())
		}
	}

	// Hack for implementing the [`Default`] bound needed for
	// [`IsmpDispatcher`](ismp::dispatcher::IsmpDispatcher) and
	// [`IsmpModule`](ismp::module::IsmpModule)
	impl<T> Default for Pallet<T> {
		fn default() -> Self {
			Self(PhantomData)
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
						if let Some(transfer) = <PendingTransfersOut<T>>::take(transfer_id) {
							<ExpiringTransfersOutByNotary<T>>::mutate(
								transfer.notary_id,
								transfer.expiration_tick,
								|e| {
									if let Some(pos) = e.iter().position(|x| x == transfer_id) {
										e.remove(pos);
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

			let expiring = <ExpiringTransfersOutByNotary<T>>::take(notary_id, header.tick);
			for transfer_id in expiring.into_iter() {
				let Some(transfer) = <PendingTransfersOut<T>>::take(transfer_id) else { continue };
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

	impl<T: Config>
		ChainTransferLookup<<T as frame_system::Config>::AccountId, <T as Config>::Balance>
		for Pallet<T>
	{
		fn is_valid_transfer_to_localchain(
			notary_id: NotaryId,
			transfer_id: TransferToLocalchainId,
			account_id: &<T as frame_system::Config>::AccountId,
			milligons: <T as Config>::Balance,
			at_tick: Tick,
		) -> bool {
			let result = <PendingTransfersOut<T>>::get(transfer_id);
			if let Some(transfer) = result {
				return transfer.notary_id == notary_id &&
					transfer.amount == milligons &&
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

		fn next_transfer_id() -> Result<TransferToLocalchainId, Error<T>> {
			let transfer_id = NextTransferId::<T>::get().unwrap_or(1);
			let next_transfer_id = transfer_id.increment();
			NextTransferId::<T>::set(next_transfer_id);
			Ok(transfer_id)
		}
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[codec(mel_bound(Balance: MaxEncodedLen, BlockNumber: MaxEncodedLen))]
pub struct QueuedTransferOut<AccountId, Balance> {
	pub account_id: AccountId,
	pub amount: Balance,
	pub expiration_tick: Tick,
	pub notary_id: NotaryId,
}

/// Asset transfers to an evm chain via Hyperbridge
#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub struct TransferToEvm<Balance> {
	/// Asset to be sent
	pub asset: Asset,
	/// The evm chain
	pub evm_chain: EvmChain,
	/// Receiving account on destination
	pub recipient: H160,
	/// Amount to be sent
	pub amount: Balance,
	/// Request timeout (seconds). This should be greater than the cumulative time for
	/// finalization on argon AND hyperbridge with some additional buffer.
	pub timeout: u64,
	/// An extra payment to reward the relayer for faster validation
	pub relayer_fee: Balance,
}
