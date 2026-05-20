#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;
pub use weights::{WeightInfo, WithProviderWeights};

use pallet_prelude::*;

#[cfg(any(test, feature = "runtime-benchmarks"))]
pub(crate) const TRANSFER_TO_ARGON_STARTED_EVENT_SIGNATURE: &[u8] =
	b"TransferToArgonStarted(address,address,uint128,bytes32,(uint64,uint64,uint128,uint128))";

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
	use alloy_sol_types::{sol, SolEvent};
	use argon_primitives::{
		ethereum::{
			EthereumReceiptLogProofBatch as BaseEthereumReceiptLogProofBatch,
			EthereumReceiptLogProofBlock as BaseEthereumReceiptLogProofBlock,
			MAX_ETHEREUM_HEADER_CHAIN_LEN,
		},
		CallTxPoolKeyProvider, CallTxValidityProvider, EthereumLog, EthereumVerifyProvider,
		OperationalAccountsHook, UniswapTransferProvider,
	};
	use frame_support::dispatch::Pays;
	use polkadot_sdk::{
		frame_support::traits::IsSubType,
		frame_system::{ensure_root, ensure_signed},
		sp_crypto_hashing::blake2_256,
		sp_runtime::transaction_validity::InvalidTransaction,
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	sol! {
		struct GatewayActivityState {
			uint64 gateway_activity_nonce;
			uint64 argon_approvals_nonce;
			uint128 argon_circulation;
			uint128 argonot_circulation;
		}

		event TransferToArgonStarted(
			address indexed from,
			address indexed token,
			uint128 amount,
			bytes32 argon_destination,
			GatewayActivityState gateway_state
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

		/// Runtime tick provider used for recent-transfer retention checks.
		type CurrentTick: Get<Tick>;

		/// Retention window, in ticks, for recent Argon transfer evidence used by operational
		/// accounts.
		#[pallet::constant]
		type RecentTransferRetentionTicks: Get<Tick>;

		/// Maximum number of ordered gateway activities that may share one receipt proof.
		#[pallet::constant]
		type MaxActivitiesPerReceiptProof: Get<u32>;

		/// Maximum number of proved receipt proofs that may be supplied in one extrinsic.
		#[pallet::constant]
		type MaxReceiptProofsPerExtrinsic: Get<u32>;

		/// Weight implementation for pallet calls and hooks.
		type WeightInfo: WeightInfo;
	}

	#[pallet::extra_constants]
	impl<T: Config> Pallet<T> {
		/// Maximum execution headers carried in one receipt proof's target-to-anchor chain.
		pub fn max_proof_execution_header_depth() -> u32 {
			MAX_ETHEREUM_HEADER_CHAIN_LEN
		}
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
		Encode,
		Decode,
		DecodeWithMemTracking,
		Copy,
		Clone,
		PartialEq,
		Eq,
		Debug,
		TypeInfo,
		MaxEncodedLen,
	)]
	/// Local payout asset selected from a proven inbound burn notice.
	pub enum AssetKind {
		Argon,
		Argonot,
	}

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		DebugNoBound,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	/// Latest proven gateway activity snapshot for one source chain.
	pub struct GatewayState<T: Config> {
		#[codec(compact)]
		pub gateway_activity_nonce: u64,
		#[codec(compact)]
		pub argon_approvals_nonce: u64,
		pub argon_circulation: T::Balance,
		pub argonot_circulation: T::Balance,
	}

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, DebugNoBound, TypeInfo,
	)]
	#[scale_info(skip_type_params(T))]
	/// One proven `TransferToArgonStarted` gateway activity.
	pub struct TransferToArgonActivity<T: Config> {
		#[codec(compact)]
		pub gateway_activity_nonce: u64,
		pub from: H160,
		pub asset: AssetKind,
		pub to: T::AccountId,
		#[codec(compact)]
		pub amount: T::Balance,
	}

	/// One proved contiguous activity slice backed by a combined receipt proof for one execution
	/// block.
	pub type GatewayActivityProofBlock<T> =
		BaseEthereumReceiptLogProofBlock<<T as Config>::MaxActivitiesPerReceiptProof>;

	/// Ordered proof batch supplied to `prove_gateway_activity(...)`.
	pub type GatewayActivityProofBatch<T> = BaseEthereumReceiptLogProofBatch<
		<T as Config>::MaxReceiptProofsPerExtrinsic,
		<T as Config>::MaxActivitiesPerReceiptProof,
	>;

	#[pallet::storage]
	/// Config accepted for each supported source chain.
	pub type ChainConfigBySourceChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, ChainConfig, OptionQuery>;

	#[pallet::storage]
	/// Latest proven gateway activity snapshot for each source chain.
	pub type GatewayStateBySourceChain<T: Config> =
		StorageMap<_, Blake2_128Concat, SourceChain, GatewayState<T>, OptionQuery>;

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
		/// A `TransferToArgonStarted` activity was proved and settled locally.
		TransferToArgonSettled { source_chain: SourceChain, transfer: TransferToArgonActivity<T> },
		/// The stored gateway-state snapshot advanced after a proved contiguous batch.
		GatewayStateAdvanced { source_chain: SourceChain, gateway_state: GatewayState<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The Ethereum event topics or payload do not match `TransferToArgonStarted`.
		InvalidTransferToArgonActivity,
		/// At least one proved gateway-activity block must be supplied.
		NoGatewayProofBlocksProvided,
		/// At least one gateway activity log must be supplied with the receipt proof.
		NoGatewayActivitiesProvided,
		/// The Ethereum verifier rejected the supplied proof.
		InvalidProof,
		/// The destination account bytes could not be decoded into a local account id.
		InvalidRecipient,
		/// The source chain is not configured for inbound claims.
		UnsupportedSource,
		/// The gateway does not match the configured gateway address.
		UnsupportedGateway,
		/// The token is not supported under the configured gateway.
		UnsupportedToken,
		/// The caller's expected already-proven gateway activity nonce is stale or incorrect.
		UnexpectedPreviousGatewayActivityNonce,
		/// The proven gateway activity nonce is not the next contiguous nonce.
		UnexpectedGatewayActivityNonce,
		/// The configured source-chain shape is incomplete or malformed.
		InvalidChainConfig,
		/// The burn account lacks enough balance for the payout.
		InsufficientLiquidity,
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
				ChainConfig::Ethereum { gateway, argon_token, argonot_token } => {
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
				},
			};

			let source_chain = config.source_chain();
			Self::ensure_burn_account_unreapable(&Self::burn_account(source_chain));
			ChainConfigBySourceChain::<T>::insert(source_chain, config);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight({
			let proof_blocks = proof_batch.blocks.len() as u32;
			let activities = proof_batch.blocks.iter().fold(0u32, |total, block| {
				total.saturating_add(block.receipt_logs.len() as u32)
			});
			let extra_activities = activities.saturating_sub(proof_blocks);
			T::WeightInfo::prove_gateway_activity(proof_blocks, extra_activities).saturating_add(
				T::OperationalAccountsHook::uniswap_transfer_confirmed_weight()
					.saturating_mul(activities as u64)
			)
		})]
		pub fn prove_gateway_activity(
			origin: OriginFor<T>,
			source_chain: SourceChain,
			#[pallet::compact] previous_gateway_activity_nonce: u64,
			proof_batch: GatewayActivityProofBatch<T>,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;
			ensure!(!proof_batch.blocks.is_empty(), Error::<T>::NoGatewayProofBlocksProvided);
			let current_gateway_state = GatewayStateBySourceChain::<T>::get(source_chain)
				.unwrap_or(GatewayState::<T> {
					gateway_activity_nonce: 0,
					argon_approvals_nonce: 0,
					argon_circulation: T::Balance::default(),
					argonot_circulation: T::Balance::default(),
				});

			ensure!(
				previous_gateway_activity_nonce == current_gateway_state.gateway_activity_nonce,
				Error::<T>::UnexpectedPreviousGatewayActivityNonce,
			);
			let mut expected_gateway_activity_nonce = previous_gateway_activity_nonce;
			let mut latest_gateway_state = None;
			for proof_block in &proof_batch.blocks {
				ensure!(
					!proof_block.receipt_logs.is_empty(),
					Error::<T>::NoGatewayActivitiesProvided
				);
			}

			T::EthereumVerifier::verify_receipt_logs(&proof_batch)
				.map_err(|_| Error::<T>::InvalidProof)?;

			for proof_block in proof_batch.blocks {
				for receipt_log in proof_block.receipt_logs {
					let activity =
						Self::decode_transfer_to_argon_started_log(&receipt_log.event_log)
							.map_err(|_| Error::<T>::InvalidTransferToArgonActivity)?;
					let (transfer, gateway_state) =
						Self::decode_ethereum_transfer_to_argon_started(
							activity,
							&receipt_log.event_log.address,
						)?;

					ensure!(
						gateway_state.gateway_activity_nonce ==
							expected_gateway_activity_nonce.saturating_add(1),
						Error::<T>::UnexpectedGatewayActivityNonce,
					);

					expected_gateway_activity_nonce = gateway_state.gateway_activity_nonce;

					match transfer.asset {
						AssetKind::Argon => {
							Self::mint_to::<T::NativeCurrency>(
								source_chain,
								transfer.amount,
								&transfer.to,
							)?;

							if transfer.amount != T::Balance::default() {
								Self::retain_recent_argon_transfer(&transfer.to);
								T::OperationalAccountsHook::uniswap_transfer_confirmed(
									&transfer.to,
									transfer.amount,
								);
							}
						},
						AssetKind::Argonot => {
							Self::mint_to::<T::OwnershipCurrency>(
								source_chain,
								transfer.amount,
								&transfer.to,
							)?;
						},
					}

					Self::deposit_event(Event::TransferToArgonSettled { source_chain, transfer });

					latest_gateway_state = Some(gateway_state);
				}
			}

			let latest_gateway_state =
				latest_gateway_state.expect("non-empty proof batch must produce gateway state");
			GatewayStateBySourceChain::<T>::insert(source_chain, latest_gateway_state.clone());
			Self::deposit_event(Event::GatewayStateAdvanced {
				source_chain,
				gateway_state: latest_gateway_state,
			});
			Ok(Pays::No.into())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn burn_account(source_chain: SourceChain) -> T::AccountId {
			match source_chain {
				SourceChain::Ethereum => T::EthereumBurnAccount::get(),
			}
		}

		fn mint_to<C: Mutate<T::AccountId, Balance = T::Balance> + 'static>(
			source_chain: SourceChain,
			amount: T::Balance,
			to: &T::AccountId,
		) -> DispatchResult {
			let burn_account = Self::burn_account(source_chain);
			if amount == 0u128.into() {
				return Ok(());
			}
			ensure!(
				C::reducible_balance(&burn_account, Preservation::Expendable, Fortitude::Force,) >=
					amount,
				Error::<T>::InsufficientLiquidity,
			);

			let _ = C::burn_from(
				&burn_account,
				amount,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)?;
			let _ = C::mint_into(to, amount)?;

			Ok(())
		}

		fn ensure_burn_account_unreapable(account_id: &T::AccountId) {
			let providers = frame_system::Pallet::<T>::providers(account_id);
			for _ in providers..2 {
				frame_system::Pallet::<T>::inc_providers(account_id);
			}
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

			match config {
				ChainConfig::Ethereum { gateway: active_gateway, argon_token, argonot_token } => {
					ensure!(*gateway == active_gateway, Error::<T>::UnsupportedGateway);

					if *token == argon_token {
						return Ok(AssetKind::Argon);
					}
					if *token == argonot_token {
						return Ok(AssetKind::Argonot);
					}

					Err(Error::<T>::UnsupportedToken.into())
				},
			}
		}

		fn decode_ethereum_transfer_to_argon_started(
			event: TransferToArgonStarted,
			gateway: &H160,
		) -> Result<(TransferToArgonActivity<T>, GatewayState<T>), DispatchError> {
			let from = H160::from_slice(event.from.as_slice());
			let token = H160::from_slice(event.token.as_slice());
			let amount = Self::decode_runtime_balance(event.amount)?;
			let mut destination_bytes = event.argon_destination.as_slice();
			let destination = T::AccountId::decode(&mut destination_bytes)
				.map_err(|_| Error::<T>::InvalidRecipient)?;
			let gateway_state = GatewayState::<T> {
				gateway_activity_nonce: event.gateway_state.gateway_activity_nonce,
				argon_approvals_nonce: event.gateway_state.argon_approvals_nonce,
				argon_circulation: Self::decode_runtime_balance(
					event.gateway_state.argon_circulation,
				)?,
				argonot_circulation: Self::decode_runtime_balance(
					event.gateway_state.argonot_circulation,
				)?,
			};
			let asset = Self::resolve_ethereum_asset_kind(SourceChain::Ethereum, gateway, &token)?;
			let transfer = TransferToArgonActivity::<T> {
				gateway_activity_nonce: gateway_state.gateway_activity_nonce,
				from,
				asset,
				to: destination,
				amount,
			};

			Ok((transfer, gateway_state))
		}

		fn decode_runtime_balance(amount: u128) -> Result<T::Balance, DispatchError> {
			Ok(amount.into())
		}

		fn decode_transfer_to_argon_started_log(
			log: &EthereumLog,
		) -> Result<TransferToArgonStarted, alloy_sol_types::Error> {
			TransferToArgonStarted::decode_raw_log_validate(
				log.topics.iter().map(|topic| topic.0),
				&log.data,
			)
		}
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
				Call::prove_gateway_activity { source_chain, proof_batch, .. } => Some(
					(
						b"crosschain_transfer:prove".as_slice(),
						source_chain,
						proof_batch.using_encoded(blake2_256),
					)
						.using_encoded(blake2_256)
						.to_vec(),
				),
				_ => None,
			}
		}
	}

	impl<T: Config> CallTxValidityProvider<RuntimeCallOf<T>, T::AccountId> for Pallet<T>
	where
		RuntimeCallOf<T>: IsSubType<Call<T>>,
	{
		fn validate(
			call: &RuntimeCallOf<T>,
			_signer: Option<&T::AccountId>,
		) -> Result<(), TransactionValidityError> {
			let Some(call) = <RuntimeCallOf<T> as IsSubType<Call<T>>>::is_sub_type(call) else {
				return Ok(());
			};

			if let Call::prove_gateway_activity {
				source_chain,
				previous_gateway_activity_nonce,
				..
			} = call
			{
				let current_nonce = GatewayStateBySourceChain::<T>::get(source_chain)
					.map(|state| state.gateway_activity_nonce)
					.unwrap_or_default();

				if previous_gateway_activity_nonce < &current_nonce {
					return Err(TransactionValidityError::Invalid(InvalidTransaction::Stale));
				}
			}

			Ok(())
		}
	}
}
