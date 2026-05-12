#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;
pub use weights::{WeightInfo, WithProviderWeights};

use pallet_prelude::*;

#[cfg(any(test, feature = "runtime-benchmarks"))]
pub(crate) const BURN_FOR_TRANSFER_EVENT_SIGNATURE: &[u8] =
	b"BurnForTransfer(address,address,uint256,bytes32,uint64)";

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloy_primitives::U256;
	use alloy_sol_types::{sol, SolEvent};
	use argon_primitives::{
		verify_and_decode_event, CallTxPoolKeyProvider, CurrentTransactionFeeProvider,
		EthereumEventDecoder, EthereumLog, EthereumProof, EthereumVerifyAndDecodeError,
		EthereumVerifyProvider, OperationalAccountsHook, UniswapTransferProvider,
	};
	use polkadot_sdk::{
		frame_support::traits::IsSubType,
		frame_system::{ensure_root, ensure_signed},
		sp_crypto_hashing::blake2_256,
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	sol! {
		event BurnForTransfer(
			address indexed from,
			address indexed token,
			uint256 amount_base_units,
			bytes32 argon_destination,
			uint64 account_nonce
		);
	}

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: polkadot_sdk::frame_system::Config<RuntimeEvent: From<Event<Self>>> {
		/// Balance type used for inbound payouts and recent-transfer tracking.
		type Balance: AtLeast32BitUnsigned
			+ Member
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ DecodeWithMemTracking
			+ core::fmt::Debug
			+ Default
			+ From<u128>
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// Canonical Ethereum burn-accounting account representing funds moved to Ethereum.
		#[pallet::constant]
		type EthereumBurnAccount: Get<Self::AccountId>;

		/// Native Argon currency implementation
		type NativeCurrency: Mutate<Self::AccountId, Balance = Self::Balance>;

		/// Ownership-token currency implementation
		type OwnershipCurrency: Mutate<Self::AccountId, Balance = Self::Balance>;

		/// Ethereum proof verifier for receipt and header-chain validation.
		type EthereumVerifier: EthereumVerifyProvider;

		/// Existing operational-accounts hook for qualifying inbound Argon transfers.
		type OperationalAccountsHook: OperationalAccountsHook<Self::AccountId, Self::Balance>;

		/// Reimbursable transaction fee captured for the currently executing prove-transfer call.
		type CurrentTransactionFeeProvider: CurrentTransactionFeeProvider<Self::Balance>;

		/// Runtime tick provider used for previous-release cutover checks.
		type CurrentTick: Get<Tick>;

		/// Retention window, in ticks, for recent Argon transfer evidence used by operational
		/// accounts.
		#[pallet::constant]
		type RecentTransferRetentionTicks: Get<Tick>;

		/// Weight implementation for pallet calls and hooks.
		type WeightInfo: WeightInfo;
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Copy,
		PartialEq,
		Eq,
		Debug,
		TypeInfo,
		MaxEncodedLen,
	)]
	/// Concrete source chains supported by this pallet.
	pub enum SourceChain {
		Ethereum,
	}

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
	)]
	/// Source-chain config accepted by this deployment.
	pub enum ChainConfig {
		Ethereum {
			/// Active gateway contract address for Ethereum.
			gateway: H160,
			/// Active Ethereum Argon token address.
			argon_token: H160,
			/// Active Ethereum Argonot token address.
			argonot_token: H160,
			/// Previously accepted gateway during a bounded cutover window.
			previous_gateway: Option<H160>,
			/// Last runtime tick where the previous release remains accepted.
			previous_release_expiration: Option<Tick>,
		},
	}

	impl ChainConfig {
		pub fn source_chain(&self) -> SourceChain {
			match self {
				Self::Ethereum { .. } => SourceChain::Ethereum,
			}
		}
	}

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
	)]
	/// Local payout asset selected from a proven inbound burn notice.
	pub enum AssetKind {
		Argon,
		Argonot,
	}

	#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
	/// Supported inbound proof variants.
	pub enum TransferProof {
		Ethereum { source_chain: SourceChain, event_log: EthereumLog, proof: EthereumProof },
	}

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, DebugNoBound, TypeInfo,
	)]
	#[scale_info(skip_type_params(T))]
	/// Burn notice accepted after a successful inbound proof verification.
	pub struct BurnNotice<T: Config> {
		/// Source-chain account that burned tokens.
		pub from: H160,
		/// Local payout recipient.
		pub to: T::AccountId,
		/// Local asset paid out after claim verification.
		pub asset_kind: AssetKind,
		/// Gross local amount proven burned on the source chain.
		pub amount: T::Balance,
		/// Monotonic nonce tracked for this source-chain account.
		#[codec(compact)]
		pub account_nonce: u64,
	}

	#[pallet::storage]
	/// Config accepted for each supported source chain.
	pub type ChainConfigBySourceChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, ChainConfig, OptionQuery>;

	#[pallet::storage]
	/// Latest accepted nonce for each `(source_chain, from)` pair.
	pub type NonceBySourceAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, (SourceChain, H160), u64, OptionQuery>;

	#[pallet::storage]
	/// Count of still-retained qualifying Argon transfers for each local account.
	pub type RecentArgonTransfersByAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	#[pallet::storage]
	/// Accounts whose recent-transfer evidence expires at a given tick.
	#[pallet::unbounded]
	pub type InboundTransfersExpiringAt<T: Config> =
		StorageMap<_, Twox64Concat, Tick, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	/// Latest tick whose recent-transfer expiration bucket was cleaned up.
	pub type LastTransferExpiryCleanupTick<T: Config> = StorageValue<_, Tick, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An inbound burn notice was accepted and settled locally.
		BurnNoticeAccepted { source_chain: SourceChain, notice: BurnNotice<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The Ethereum event topics or payload do not match `BurnForTransfer`.
		InvalidEthereumEvent,
		/// The Ethereum verifier rejected the supplied proof.
		InvalidProof,
		/// The destination account bytes could not be decoded into a local account id.
		InvalidRecipient,
		/// The claimed amount was zero or too large for the local balance type.
		InvalidAmount,
		/// The source chain is not configured for inbound claims.
		UnsupportedSource,
		/// The gateway does not match the active or still-accepted previous release.
		UnsupportedGateway,
		/// The token is not supported under the matched gateway release.
		UnsupportedToken,
		/// The claim nonce is not exactly the next accepted nonce for the source account.
		UnexpectedNonce,
		/// The configured source-chain shape is incomplete or malformed.
		InvalidChainConfig,
		/// The burn account lacks enough balance for the payout.
		InsufficientLiquidity,
		/// The captured reimbursable fee is greater than or equal to the burned Argon amount.
		InsufficientBurnAmountForFee,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let current_tick = T::CurrentTick::get();
			let last_cleanup_tick = LastTransferExpiryCleanupTick::<T>::get();
			let first_tick_to_cleanup = if last_cleanup_tick == 0 {
				current_tick
			} else {
				last_cleanup_tick.saturating_add(1)
			};
			let mut expiring_len = 0u32;

			for tick in first_tick_to_cleanup..=current_tick {
				let expiring = InboundTransfersExpiringAt::<T>::take(tick);
				expiring_len = expiring_len.saturating_add(expiring.len() as u32);

				for account_id in expiring {
					Self::decrement_recent_argon_transfer(&account_id);
				}
			}

			LastTransferExpiryCleanupTick::<T>::put(current_tick);
			T::WeightInfo::on_initialize_cleanup(expiring_len)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_chain_config())]
		pub fn set_chain_config(origin: OriginFor<T>, config: ChainConfig) -> DispatchResult {
			ensure_root(origin)?;
			match config {
				ChainConfig::Ethereum {
					gateway,
					argon_token,
					argonot_token,
					previous_gateway,
					previous_release_expiration,
				} => {
					ensure!(
						gateway != H160::zero() &&
							argon_token != H160::zero() &&
							argonot_token != H160::zero(),
						Error::<T>::InvalidChainConfig,
					);

					if let Some(previous_config) =
						ChainConfigBySourceChain::<T>::get(SourceChain::Ethereum)
					{
						match previous_config {
							ChainConfig::Ethereum {
								argon_token: prev_argon_token,
								argonot_token: prev_argonot_token,
								..
							} => {
								ensure!(
									argon_token == prev_argon_token &&
										argonot_token == prev_argonot_token,
									Error::<T>::InvalidChainConfig,
								);
							},
						}
					}

					let has_any_previous =
						previous_gateway.is_some() || previous_release_expiration.is_some();
					let has_full_previous =
						previous_gateway.is_some() && previous_release_expiration.is_some();

					ensure!(!has_any_previous || has_full_previous, Error::<T>::InvalidChainConfig);
				},
			};

			ChainConfigBySourceChain::<T>::insert(config.source_chain(), config);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(
			T::WeightInfo::prove_transfer()
				.saturating_add(T::OperationalAccountsHook::uniswap_transfer_confirmed_weight())
		)]
		pub fn prove_transfer(origin: OriginFor<T>, proof: TransferProof) -> DispatchResult {
			let submitter = ensure_signed(origin)?;

			match proof {
				TransferProof::Ethereum { source_chain, event_log, proof } => {
					let burn_notice =
						verify_and_decode_event::<T::EthereumVerifier, BurnForTransfer>(
							&event_log, &proof,
						)
						.map_err(|error| match error {
							EthereumVerifyAndDecodeError::Verify(_) => Error::<T>::InvalidProof,
							EthereumVerifyAndDecodeError::Decode(_) =>
								Error::<T>::InvalidEthereumEvent,
						})?;
					let claim = Self::decode_ethereum_burn_notice(burn_notice)?;
					let asset_kind = Self::resolve_ethereum_asset_kind(
						source_chain,
						&event_log.address,
						&claim.token,
					)?;

					Self::enact_burn_transfer(&submitter, source_chain, claim, asset_kind)
				},
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn burn_account(source_chain: SourceChain) -> T::AccountId {
			match source_chain {
				SourceChain::Ethereum => T::EthereumBurnAccount::get(),
			}
		}

		fn enact_burn_transfer(
			submitter: &T::AccountId,
			source_chain: SourceChain,
			claim: DecodedEthereumBurnNotice<T>,
			asset_kind: AssetKind,
		) -> DispatchResult {
			let source_key = (source_chain, claim.from);
			let latest_nonce = NonceBySourceAccount::<T>::get(source_key).unwrap_or_default();
			ensure!(
				claim.account_nonce == latest_nonce.saturating_add(1),
				Error::<T>::UnexpectedNonce,
			);

			match asset_kind {
				AssetKind::Argon => {
					let burn_account = Self::burn_account(source_chain);
					let reimbursable_fee = if claim.to == *submitter {
						T::Balance::default()
					} else {
						T::CurrentTransactionFeeProvider::reimbursable_fee().unwrap_or_default()
					};
					let recipient_amount =
						Self::argon_recipient_amount(claim.amount, reimbursable_fee)?;

					ensure!(
						T::NativeCurrency::reducible_balance(
							&burn_account,
							Preservation::Expendable,
							Fortitude::Force,
						) >= claim.amount,
						Error::<T>::InsufficientLiquidity,
					);

					let _ = T::NativeCurrency::transfer(
						&burn_account,
						&claim.to,
						recipient_amount,
						Preservation::Expendable,
					)?;

					if reimbursable_fee != T::Balance::default() {
						let _ = T::NativeCurrency::transfer(
							&burn_account,
							submitter,
							reimbursable_fee,
							Preservation::Expendable,
						)?;
					}

					Self::retain_recent_argon_transfer(&claim.to);
					T::OperationalAccountsHook::uniswap_transfer_confirmed(&claim.to, claim.amount);
				},
				AssetKind::Argonot => {
					let burn_account = Self::burn_account(source_chain);
					ensure!(
						T::OwnershipCurrency::reducible_balance(
							&burn_account,
							Preservation::Expendable,
							Fortitude::Force,
						) >= claim.amount,
						Error::<T>::InsufficientLiquidity,
					);
					let _ = T::OwnershipCurrency::transfer(
						&burn_account,
						&claim.to,
						claim.amount,
						Preservation::Expendable,
					)?;
				},
			}

			let notice = BurnNotice::<T> {
				from: claim.from,
				to: claim.to,
				asset_kind,
				amount: claim.amount,
				account_nonce: claim.account_nonce,
			};

			NonceBySourceAccount::<T>::insert(source_key, notice.account_nonce);
			Self::deposit_event(Event::BurnNoticeAccepted { source_chain, notice });

			Ok(())
		}

		fn argon_recipient_amount(
			amount: T::Balance,
			reimbursable_fee: T::Balance,
		) -> Result<T::Balance, DispatchError> {
			let reimbursable_fee: u128 = reimbursable_fee.into();
			if reimbursable_fee == 0 {
				return Ok(amount);
			}

			let amount: u128 = amount.into();
			ensure!(amount > reimbursable_fee, Error::<T>::InsufficientBurnAmountForFee);

			Ok(amount.saturating_sub(reimbursable_fee).into())
		}

		fn retain_recent_argon_transfer(account_id: &T::AccountId) {
			RecentArgonTransfersByAccount::<T>::mutate(account_id, |count| {
				*count = count.saturating_add(1);
			});

			let expires_at =
				T::CurrentTick::get().saturating_add(T::RecentTransferRetentionTicks::get());
			InboundTransfersExpiringAt::<T>::mutate(expires_at, |accounts| {
				accounts.push(account_id.clone());
			});
		}

		fn decrement_recent_argon_transfer(account_id: &T::AccountId) {
			RecentArgonTransfersByAccount::<T>::mutate_exists(account_id, |count| {
				let Some(existing) = count.as_mut() else {
					return;
				};

				if *existing <= 1 {
					*count = None;
				} else {
					*existing = existing.saturating_sub(1);
				}
			});
		}

		fn resolve_ethereum_asset_kind(
			source_chain: SourceChain,
			gateway: &H160,
			token: &H160,
		) -> Result<AssetKind, DispatchError> {
			let config = ChainConfigBySourceChain::<T>::get(source_chain)
				.ok_or(Error::<T>::UnsupportedSource)?;
			let now = T::CurrentTick::get();

			match config {
				ChainConfig::Ethereum {
					gateway: active_gateway,
					argon_token,
					argonot_token,
					previous_gateway,
					previous_release_expiration,
				} => {
					let mut is_valid_gateway = *gateway == active_gateway;

					let previous_release_is_open =
						previous_release_expiration.is_some_and(|expiration| now <= expiration);
					if previous_release_is_open && previous_gateway == Some(*gateway) {
						is_valid_gateway = true;
					}

					if is_valid_gateway {
						if *token == argon_token {
							return Ok(AssetKind::Argon);
						}
						if *token == argonot_token {
							return Ok(AssetKind::Argonot);
						}
					}

					Err(Error::<T>::UnsupportedGateway.into())
				},
			}
		}

		fn decode_ethereum_burn_notice(
			event: BurnForTransfer,
		) -> Result<DecodedEthereumBurnNotice<T>, DispatchError> {
			let from = H160::from_slice(event.from.as_slice());
			let token = H160::from_slice(event.token.as_slice());
			let amount = Self::decode_amount(event.amount_base_units)?;
			let mut destination_bytes = event.argon_destination.as_slice();
			let destination = T::AccountId::decode(&mut destination_bytes)
				.map_err(|_| Error::<T>::InvalidRecipient)?;

			Ok(DecodedEthereumBurnNotice {
				from,
				token,
				to: destination,
				amount,
				account_nonce: event.account_nonce,
			})
		}

		fn decode_amount(amount: U256) -> Result<T::Balance, DispatchError> {
			let amount_u128 = u128::try_from(amount).map_err(|_| Error::<T>::InvalidAmount)?;
			ensure!(amount_u128 > 0, Error::<T>::InvalidAmount);
			Ok(amount_u128.into())
		}
	}

	impl EthereumEventDecoder for BurnForTransfer {
		type Error = alloy_sol_types::Error;

		fn decode_ethereum_log(log: &EthereumLog) -> Result<Self, Self::Error> {
			Self::decode_raw_log_validate(log.topics.iter().map(|topic| topic.0), &log.data)
		}
	}

	struct DecodedEthereumBurnNotice<T: Config> {
		from: H160,
		token: H160,
		to: T::AccountId,
		amount: T::Balance,
		account_nonce: u64,
	}

	impl<T: Config> UniswapTransferProvider<T::AccountId> for Pallet<T> {
		type Weights = weights::ProviderWeightAdapter<T>;

		fn is_crosschain_activated() -> bool {
			ChainConfigBySourceChain::<T>::contains_key(SourceChain::Ethereum)
		}

		fn has_recent_argon_transfer(account_id: &T::AccountId) -> bool {
			RecentArgonTransfersByAccount::<T>::get(account_id) > 0
		}
	}

	type RuntimeCallOf<T> = <T as frame_system::Config>::RuntimeCall;

	impl<T: Config> CallTxPoolKeyProvider<RuntimeCallOf<T>, T::AccountId> for Pallet<T>
	where
		RuntimeCallOf<T>: IsSubType<Call<T>>,
	{
		fn key_for(call: &RuntimeCallOf<T>, _signer: Option<&T::AccountId>) -> Option<Vec<u8>> {
			let call = <RuntimeCallOf<T> as IsSubType<Call<T>>>::is_sub_type(call)?;

			match call {
				Call::prove_transfer {
					proof: TransferProof::Ethereum { source_chain, event_log, .. },
				} => {
					let event = BurnForTransfer::decode_ethereum_log(event_log).ok()?;
					let from = H160::from_slice(event.from.as_slice());

					Some(
						(
							b"crosschain_transfer:prove".as_slice(),
							source_chain,
							from,
							event.account_nonce,
						)
							.using_encoded(blake2_256)
							.to_vec(),
					)
				},
				_ => None,
			}
		}
	}
}
