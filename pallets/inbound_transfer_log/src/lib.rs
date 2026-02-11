#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;
pub use weights::WeightInfo;

use core::marker::PhantomData;
use ismp::{
	module::IsmpModule,
	router::{PostRequest, Response, Timeout},
};
use pallet_prelude::*;
use pallet_token_gateway::types::AssetId as TokenGatewayAssetId;
use polkadot_sdk::frame_support::{
	traits::{Currency, Get, fungibles},
	weights::Weight,
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod weights;

/// ISMP hook that records inbound TokenGateway transfers after delegating to the TokenGateway
/// pallet.
pub struct TokenGatewayHook<T>(PhantomData<T>);

impl<T> Default for TokenGatewayHook<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T> IsmpModule for TokenGatewayHook<T>
where
	T: pallet::Config + pallet_token_gateway::Config,
	T::AccountId: From<[u8; 32]>,
	TokenGatewayAssetId<T>: From<u32>,
	<<T as pallet_token_gateway::Config>::NativeCurrency as Currency<T::AccountId>>::Balance:
		From<u128>,
	<<T as pallet_token_gateway::Config>::Assets as fungibles::Inspect<T::AccountId>>::Balance:
		From<u128>,
{
	fn on_accept(&self, request: PostRequest) -> Result<Weight, anyhow::Error> {
		let max_body_len = <T as pallet::Config>::MaxInboundTransferBytes::get();
		if max_body_len > 0 && request.body.len() as u32 > max_body_len {
			// Accepted short-term risk: we only log here (no hard reject) to avoid breaking
			// chain transfers while validating upstream Hyperbridge payload limits/behavior.
			log::error!(
				"TokenGateway request body too large: {} vs {}",
				request.body.len(),
				max_body_len
			);
		}
		let token_gateway_weight =
			pallet_token_gateway::Pallet::<T>::default().on_accept(request.clone())?;
		let inbound_weight = pallet::Pallet::<T>::on_token_gateway_request(&request);
		Ok(token_gateway_weight.saturating_add(inbound_weight))
	}

	fn on_response(&self, response: Response) -> Result<Weight, anyhow::Error> {
		pallet_token_gateway::Pallet::<T>::default().on_response(response)
	}

	fn on_timeout(&self, timeout: Timeout) -> Result<Weight, anyhow::Error> {
		pallet_token_gateway::Pallet::<T>::default().on_timeout(timeout)
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	use sp_runtime::traits::Saturating;

	use alloy_sol_types::SolValue;
	use argon_primitives::OperationalAccountsHook;
	use ismp::{host::StateMachine, router::PostRequest};
	use pallet_token_gateway::{
		impls::convert_to_balance,
		types::{AssetId as TokenGatewayAssetId, Body, BodyWithCall, RequestBody},
	};

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// How many blocks to retain inbound transfer records.
		#[pallet::constant]
		type InboundTransfersRetentionBlocks: Get<BlockNumberFor<Self>>;

		/// Maximum number of records retained from a single block.
		#[pallet::constant]
		type MaxTransfersToRetainPerBlock: Get<u32>;

		/// Maximum number of bytes allowed in a TokenGateway request body (0 disables the cap).
		#[pallet::constant]
		type MaxInboundTransferBytes: Get<u32>;

		/// Minimum amount (in base units) to record an inbound transfer.
		#[pallet::constant]
		type MinimumTransferMicrogonsToRecord: Get<Balance>;

		/// Ownership token asset id (Argonot).
		#[pallet::constant]
		type OwnershipAssetId: Get<u32>;

		/// Weight information for this pallet.
		type WeightInfo: WeightInfo;

		/// Hook called after a qualifying inbound transfer is recorded.
		type OperationalAccountsHook: OperationalAccountsHook<Self::AccountId, Balance>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Copy,
		PartialEq,
		Eq,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub enum AssetKind {
		Argon,
		Argonot,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Copy,
		PartialEq,
		Eq,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub enum InboundTransferDropReason {
		BodyTooLarge,
		AbiDecodeFailed,
		NonEvmSource,
		UnknownAsset,
		UnsupportedAsset,
		UnknownPrecision,
		InvalidAmount,
		AmountBelowMinimum,
		ExpirationQueueFull,
	}

	#[pallet::storage]
	#[pallet::unbounded]
	/// Inbound EVM transfers recorded by their request commitment key.
	pub type InboundEvmTransfers<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, InboundEvmTransfer<T>, OptionQuery>;

	#[pallet::storage]
	/// Index of inbound transfer record keys expiring at a given block.
	pub type InboundTransfersExpiringAt<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<H256, T::MaxTransfersToRetainPerBlock>,
		ValueQuery,
	>;

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, TypeInfo, RuntimeDebugNoBound,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct InboundEvmTransfer<T: Config> {
		pub source: StateMachine,
		pub nonce: u64,
		pub evm_from: H160,
		pub to: T::AccountId,
		pub asset_kind: AssetKind,
		pub amount: Balance,
		pub expires_at_block: BlockNumberFor<T>,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A qualifying inbound TokenGateway request was recorded.
		InboundEvmTransferRecorded { transfer: InboundEvmTransfer<T> },
		/// A TokenGateway request was ignored or dropped.
		InboundEvmTransferDropped {
			source: StateMachine,
			nonce: u64,
			reason: InboundTransferDropReason,
		},
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let retention_blocks: BlockNumberFor<T> = T::InboundTransfersRetentionBlocks::get();
			let expiring =
				InboundTransfersExpiringAt::<T>::take(n.saturating_sub(retention_blocks));
			let expiring_len = expiring.len() as u32;
			for key in expiring {
				InboundEvmTransfers::<T>::remove(key);
			}
			T::WeightInfo::on_initialize_cleanup(expiring_len)
		}
	}

	impl<T: Config> Pallet<T> {
		fn drop_inbound_request(
			source: &StateMachine,
			nonce: u64,
			reason: InboundTransferDropReason,
		) -> Weight {
			Self::deposit_event(Event::InboundEvmTransferDropped {
				source: *source,
				nonce,
				reason,
			});
			<T as Config>::WeightInfo::on_token_gateway_request_dropped()
		}

		fn decode_request_body(encoded_body: &[u8]) -> Option<RequestBody> {
			if let Ok(body) = Body::abi_decode(encoded_body) {
				return Some(body.into());
			}
			if let Ok(body) = BodyWithCall::abi_decode(encoded_body) {
				return Some(body.into());
			}
			None
		}

		/// Best-effort hook for inbound TokenGateway ISMP requests.
		pub fn on_token_gateway_request(request: &PostRequest) -> Weight
		where
			T: pallet_token_gateway::Config,
			T::AccountId: From<[u8; 32]>,
			TokenGatewayAssetId<T>: From<u32>,
		{
			let source = request.source;
			let nonce = request.nonce;
			let Some(body) = request.body.get(1..).and_then(Self::decode_request_body) else {
				return Self::drop_inbound_request(
					&source,
					nonce,
					InboundTransferDropReason::AbiDecodeFailed,
				);
			};

			let from_bytes = body.from.0;
			let to_bytes = body.to.0;
			let asset_bytes = body.asset_id.0;
			let amount_u256 = U256::from_big_endian(&body.amount.to_be_bytes::<32>());
			if !request.source.is_evm() {
				return Self::drop_inbound_request(
					&source,
					nonce,
					InboundTransferDropReason::NonEvmSource,
				);
			}
			let evm_from = H160::from_slice(&from_bytes[12..]);

			let asset_id = H256::from(asset_bytes);
			let Some(local_asset_id) = pallet_token_gateway::LocalAssets::<T>::get(asset_id) else {
				return Self::drop_inbound_request(
					&source,
					nonce,
					InboundTransferDropReason::UnknownAsset,
				);
			};

			let ownership_asset_id: TokenGatewayAssetId<T> = T::OwnershipAssetId::get().into();
			let asset_kind = if local_asset_id == T::NativeAssetId::get() {
				AssetKind::Argon
			} else if local_asset_id == ownership_asset_id {
				AssetKind::Argonot
			} else {
				return Self::drop_inbound_request(
					&source,
					nonce,
					InboundTransferDropReason::UnsupportedAsset,
				);
			};

			let Some(erc_decimals) =
				pallet_token_gateway::Precisions::<T>::get(local_asset_id, request.source)
			else {
				return Self::drop_inbound_request(
					&source,
					nonce,
					InboundTransferDropReason::UnknownPrecision,
				);
			};

			let amount = if let Ok(amount) =
				convert_to_balance(amount_u256, erc_decimals, T::Decimals::get())
			{
				amount
			} else {
				return Self::drop_inbound_request(
					&source,
					nonce,
					InboundTransferDropReason::InvalidAmount,
				);
			};

			let min_amount = T::MinimumTransferMicrogonsToRecord::get();
			if amount < min_amount {
				return Self::drop_inbound_request(
					&source,
					nonce,
					InboundTransferDropReason::AmountBelowMinimum,
				);
			}

			let record_key = H256::from(blake2_256(&(source, nonce).encode()));
			let current_block = frame_system::Pallet::<T>::block_number();
			let retention_blocks: BlockNumberFor<T> = T::InboundTransfersRetentionBlocks::get();
			let expires_at_block = current_block.saturating_add(retention_blocks);

			let to_account: T::AccountId = to_bytes.into();

			let transfer = InboundEvmTransfer::<T> {
				source,
				nonce,
				evm_from,
				to: to_account,
				asset_kind,
				amount,
				expires_at_block,
			};

			InboundEvmTransfers::<T>::insert(record_key, transfer.clone());

			if InboundTransfersExpiringAt::<T>::try_mutate(expires_at_block, |keys| {
				keys.try_push(record_key)
			})
			.is_err()
			{
				InboundEvmTransfers::<T>::remove(record_key);
				return Self::drop_inbound_request(
					&source,
					nonce,
					InboundTransferDropReason::ExpirationQueueFull,
				);
			}

			let hook_weight = if transfer.asset_kind == AssetKind::Argon {
				T::OperationalAccountsHook::uniswap_transfer_confirmed(
					&transfer.to,
					transfer.amount,
				);
				T::OperationalAccountsHook::uniswap_transfer_confirmed_weight()
			} else {
				Weight::zero()
			};
			Self::deposit_event(Event::InboundEvmTransferRecorded { transfer });
			<T as Config>::WeightInfo::on_token_gateway_request_recorded()
				.saturating_add(hook_weight)
		}
	}
}
