use alloc::vec::Vec;
use argon_primitives::{AccountId as ArgonAccountId, EthereumReceiptLog, OperationalAccountsHook};
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	traits::fungible::{Inspect, MutateHold},
};
use pallet_prelude::*;

use super::{
	AssetKind, ChainConfig, ChainConfigBySourceChain, Config, Error, Event, GatewayActivityNonce,
	GatewayState, GatewaySyncPauseReason, MintingAuthoritiesBySigner, MintingAuthorityState,
	Pallet, SourceChain, TransferOutById, TransferToArgonActivity, H160, H256,
};

pub(crate) enum DecodedGatewayActivity<T: Config> {
	TransferToArgon {
		from: H160,
		token: H160,
		to: [u8; 32],
		amount: T::Balance,
		gateway_state: GatewayState<T>,
	},
	MintingAuthorityActivated {
		destination_signing_key: H160,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
		coactivation_count: u32,
		shared_signature_count: u32,
		relayer_argon_account_id: [u8; 32],
		gateway_state: GatewayState<T>,
	},
	MintingAuthorityDeactivated {
		destination_signing_key: H160,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
		gateway_state: GatewayState<T>,
	},
	TransferOutOfArgonFinalized {
		transfer_id: H256,
		asset: AssetKind,
		amount: T::Balance,
		minting_authority_collateral: Vec<GatewayMintingAuthorityCollateral<T>>,
		gateway_state: GatewayState<T>,
	},
	TransferOutOfArgonCanceled {
		transfer_id: H256,
		gateway_state: GatewayState<T>,
	},
}

impl<T: Config> DecodedGatewayActivity<T> {
	pub(crate) fn gateway_state(&self) -> &GatewayState<T> {
		match self {
			Self::TransferToArgon { gateway_state, .. } |
			Self::MintingAuthorityActivated { gateway_state, .. } |
			Self::MintingAuthorityDeactivated { gateway_state, .. } |
			Self::TransferOutOfArgonFinalized { gateway_state, .. } |
			Self::TransferOutOfArgonCanceled { gateway_state, .. } => gateway_state,
		}
	}
}

/// One collateral participation row emitted by Ethereum when a transfer-out reaches finality.
pub(crate) struct GatewayMintingAuthorityCollateral<T: Config> {
	/// Authority signer that Ethereum treated as responsible for this finalized slice.
	pub signing_key: H160,
	/// Argon-denominated backing that was actually consumed by finalization.
	pub microgon_collateral: T::Balance,
	/// Argonot-denominated backing that was actually consumed by finalization.
	pub micronot_collateral: T::Balance,
}

pub(crate) enum GatewayActivityApplyError {
	Pause { failed_gateway_activity_nonce: GatewayActivityNonce, reason: GatewaySyncPauseReason },
	Reject(DispatchError),
}

pub(crate) type GatewayActivityApplyResult<T> = Result<GatewayState<T>, GatewayActivityApplyError>;

impl From<DispatchError> for GatewayActivityApplyError {
	fn from(error: DispatchError) -> Self {
		Self::Reject(error)
	}
}

struct GatewayActivityContext<T: Config> {
	source_chain: SourceChain,
	gateway_state: GatewayState<T>,
}

enum GatewayCirculationAdjustment<Balance> {
	DecreaseArgon(Balance),
	DecreaseArgonot(Balance),
	IncreaseArgon(Balance),
	IncreaseArgonot(Balance),
}

impl<T: Config> GatewayActivityContext<T> {
	fn new(
		source_chain: SourceChain,
		gateway_state: GatewayState<T>,
		expected_gateway_activity_nonce: GatewayActivityNonce,
	) -> Result<Self, DispatchError> {
		ensure!(
			gateway_state.gateway_activity_nonce ==
				expected_gateway_activity_nonce.saturating_add(1),
			Error::<T>::UnexpectedGatewayActivityNonce,
		);
		Ok(Self { source_chain, gateway_state })
	}

	fn applied(self) -> GatewayState<T> {
		self.gateway_state
	}

	fn pause(&self, reason: GatewaySyncPauseReason) -> GatewayActivityApplyError {
		GatewayActivityApplyError::Pause {
			failed_gateway_activity_nonce: self.gateway_state.gateway_activity_nonce,
			reason,
		}
	}

	fn pause_result<R>(
		&self,
		reason: GatewaySyncPauseReason,
	) -> Result<R, GatewayActivityApplyError> {
		Err(self.pause(reason))
	}

	fn gateway_circulation_matches_local_state(
		&self,
		adjustment: Option<GatewayCirculationAdjustment<T::Balance>>,
	) -> bool {
		let burn_account = Pallet::<T>::burn_account(self.source_chain);
		let pending_transfer_out_circulation =
			Pallet::<T>::pending_transfer_out_circulation(self.source_chain);
		let mut expected_argon_circulation = T::NativeCurrency::balance(&burn_account)
			.saturating_sub(pending_transfer_out_circulation.argon_circulation);
		let mut expected_argonot_circulation = T::OwnershipCurrency::balance(&burn_account)
			.saturating_sub(pending_transfer_out_circulation.argonot_circulation);

		if let Some(adjustment) = adjustment {
			match adjustment {
				GatewayCirculationAdjustment::DecreaseArgon(amount) =>
					if let Some(x) = expected_argon_circulation.checked_sub(&amount) {
						expected_argon_circulation = x
					} else {
						return false;
					},
				GatewayCirculationAdjustment::DecreaseArgonot(amount) =>
					if let Some(x) = expected_argonot_circulation.checked_sub(&amount) {
						expected_argonot_circulation = x
					} else {
						return false;
					},
				GatewayCirculationAdjustment::IncreaseArgon(amount) => {
					expected_argon_circulation = expected_argon_circulation.saturating_add(amount);
				},
				GatewayCirculationAdjustment::IncreaseArgonot(amount) => {
					expected_argonot_circulation =
						expected_argonot_circulation.saturating_add(amount);
				},
			}
		}

		self.gateway_state.argon_circulation == expected_argon_circulation &&
			self.gateway_state.argonot_circulation == expected_argonot_circulation
	}

	fn ensure_gateway_circulation_matches_local_state(
		&self,
		adjustment: Option<GatewayCirculationAdjustment<T::Balance>>,
	) -> Result<(), GatewayActivityApplyError> {
		if self.gateway_circulation_matches_local_state(adjustment) {
			return Ok(());
		}

		Err(self.pause(GatewaySyncPauseReason::GatewayStateDrift))
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn resolve_source_asset_kind(
		source_chain: SourceChain,
		gateway: &H160,
		token: &H160,
	) -> Result<AssetKind, DispatchError> {
		let config = ChainConfigBySourceChain::<T>::get(source_chain)
			.ok_or(Error::<T>::UnsupportedSource)?;

		match config {
			ChainConfig::Evm { gateway: active_gateway, argon_token, argonot_token, .. } => {
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

	fn ensure_supported_gateway(source_chain: SourceChain, gateway: &H160) -> DispatchResult {
		let config = ChainConfigBySourceChain::<T>::get(source_chain)
			.ok_or(Error::<T>::UnsupportedSource)?;

		match config {
			ChainConfig::Evm { gateway: active_gateway, .. } => {
				ensure!(*gateway == active_gateway, Error::<T>::UnsupportedGateway);
				Ok(())
			},
		}
	}

	fn argon_account_id(bytes: [u8; 32]) -> T::AccountId {
		ArgonAccountId::new(bytes)
	}

	fn apply_transfer_to_argon_activity(
		context: GatewayActivityContext<T>,
		from: H160,
		asset: AssetKind,
		to: [u8; 32],
		amount: T::Balance,
	) -> GatewayActivityApplyResult<T> {
		let to = Self::argon_account_id(to);

		let transfer = TransferToArgonActivity::<T> {
			gateway_activity_nonce: context.gateway_state.gateway_activity_nonce,
			from,
			asset,
			to,
			amount,
		};

		if transfer.amount != T::Balance::default() {
			let mint_result = match transfer.asset {
				AssetKind::Argon => {
					let result = Self::mint_to::<T::NativeCurrency>(
						context.source_chain,
						transfer.amount,
						&transfer.to,
					);
					if result.is_ok() {
						Self::retain_recent_argon_transfer(&transfer.to);
						T::OperationalAccountsHook::uniswap_transfer_confirmed(
							&transfer.to,
							transfer.amount,
						);
					}
					result
				},
				AssetKind::Argonot => Self::mint_to::<T::OwnershipCurrency>(
					context.source_chain,
					transfer.amount,
					&transfer.to,
				),
			};
			if let Err(error) = mint_result {
				if error == Error::<T>::InsufficientLiquidity.into() {
					return context.pause_result(GatewaySyncPauseReason::GatewayStateDrift);
				}
				return Err(error.into());
			}
		}
		Self::deposit_event(Event::TransferToArgonSettled {
			source_chain: context.source_chain,
			transfer,
		});
		Ok(context.applied())
	}

	fn apply_minting_authority_activation(
		context: GatewayActivityContext<T>,
		destination_signing_key: H160,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
		coactivation_count: u32,
		shared_signature_count: u32,
		relayer_argon_account_id: [u8; 32],
	) -> GatewayActivityApplyResult<T> {
		let relayer_argon_account_id = Self::argon_account_id(relayer_argon_account_id);
		let mut repayment_amount = T::Balance::default();
		let mut destination_chain = None;

		MintingAuthoritiesBySigner::<T>::try_mutate_exists(
			destination_signing_key,
			|authority| -> Result<(), GatewaySyncPauseReason> {
				let Some(authority) = authority.as_mut() else {
					return Err(GatewaySyncPauseReason::MintingAuthorityNotFound);
				};
				if authority.destination_chain != context.source_chain ||
					authority.destination_signing_key != destination_signing_key ||
					authority.gateway_remaining_microgon_collateral != microgon_collateral ||
					authority.gateway_remaining_micronot_collateral != micronot_collateral
				{
					return Err(GatewaySyncPauseReason::MintingAuthorityMismatch);
				}
				if authority.state != MintingAuthorityState::PendingActivation {
					return Err(GatewaySyncPauseReason::UnexpectedMintingAuthorityState);
				}

				destination_chain = Some(authority.destination_chain);
				let held_repayment_amount = authority.activation_repayment_due.ok_or(
					GatewaySyncPauseReason::MissingMintingAuthorityActivationRepaymentPricing,
				)?;
				let activation_base_repayment_due = authority.activation_base_repayment_due.ok_or(
					GatewaySyncPauseReason::MissingMintingAuthorityActivationRepaymentPricing,
				)?;
				let activation_signature_repayment_due =
					authority.activation_signature_repayment_due.ok_or(
						GatewaySyncPauseReason::MissingMintingAuthorityActivationRepaymentPricing,
					)?;
				if held_repayment_amount != T::Balance::default() {
					if authority.account_id == relayer_argon_account_id {
						repayment_amount = held_repayment_amount;
						T::NativeCurrency::release(
							&super::HoldReason::MintingAuthorityActivationRepayment.into(),
							&authority.account_id,
							held_repayment_amount,
							Precision::Exact,
						)
						.map_err(|_| GatewaySyncPauseReason::GatewayStateDrift)?;
					} else {
						let coactivation_count: u128 = coactivation_count.max(1).into();
						let shared_signature_repayment_due: u128 =
							activation_signature_repayment_due
								.into()
								.saturating_mul(shared_signature_count.max(1).into())
								.saturating_add(coactivation_count.saturating_sub(1))
								.saturating_div(coactivation_count);
						repayment_amount = activation_base_repayment_due
							.saturating_add(shared_signature_repayment_due.into());
						repayment_amount = repayment_amount.min(held_repayment_amount);
						let refund_amount = held_repayment_amount.saturating_sub(repayment_amount);
						T::NativeCurrency::transfer_on_hold(
							&super::HoldReason::MintingAuthorityActivationRepayment.into(),
							&authority.account_id,
							&relayer_argon_account_id,
							repayment_amount,
							Precision::Exact,
							Restriction::Free,
							Fortitude::Force,
						)
						.map_err(|_| GatewaySyncPauseReason::GatewayStateDrift)?;
						if refund_amount != T::Balance::default() {
							T::NativeCurrency::release(
								&super::HoldReason::MintingAuthorityActivationRepayment.into(),
								&authority.account_id,
								refund_amount,
								Precision::Exact,
							)
							.map_err(|_| GatewaySyncPauseReason::GatewayStateDrift)?;
						}
					}
					if T::NativeCurrency::balance_on_hold(
						&super::HoldReason::MintingAuthorityActivationRepayment.into(),
						&authority.account_id,
					) == T::Balance::default()
					{
						let _ = frame_system::Pallet::<T>::dec_providers(&authority.account_id);
					}
				}
				authority.state = MintingAuthorityState::Active;
				authority.activation_base_repayment_due = None;
				authority.activation_signature_repayment_due = None;
				authority.activation_repayment_due = None;
				Ok(())
			},
		)
		.map_err(|reason| context.pause(reason))?;

		Self::deposit_event(Event::MintingAuthorityActivationFinalized {
			source_chain: context.source_chain,
			destination_signing_key,
		});
		Self::deposit_event(Event::MintingAuthorityActivationCompleted {
			destination_chain: destination_chain
				.expect("mutated activation authority must expose destination chain"),
			destination_signing_key,
			relayer_argon_account_id,
			repayment_amount,
		});
		Ok(context.applied())
	}

	fn apply_minting_authority_deactivation(
		context: GatewayActivityContext<T>,
		destination_signing_key: H160,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
	) -> GatewayActivityApplyResult<T> {
		Self::invalidate_all_transfer_reservations(destination_signing_key)
			.map_err(|reason| context.pause(reason))?;

		let mut authority_account_id = None;
		let mut local_microgon_collateral = T::Balance::default();
		let mut local_micronot_collateral = T::Balance::default();
		MintingAuthoritiesBySigner::<T>::try_mutate_exists(
			destination_signing_key,
			|maybe_authority| -> Result<(), GatewaySyncPauseReason> {
				let Some(authority) = maybe_authority.as_mut() else {
					return Err(GatewaySyncPauseReason::MintingAuthorityNotFound);
				};
				if authority.destination_chain != context.source_chain {
					return Err(GatewaySyncPauseReason::MintingAuthorityMismatch);
				}

				authority_account_id = Some(authority.account_id.clone());
				local_microgon_collateral = authority.gateway_remaining_microgon_collateral;
				local_micronot_collateral = authority.gateway_remaining_micronot_collateral;
				*maybe_authority = None;
				Ok(())
			},
		)
		.map_err(|reason| context.pause(reason))?;

		let burn_microgon_collateral = local_microgon_collateral
			.checked_sub(&microgon_collateral)
			.ok_or_else(|| context.pause(GatewaySyncPauseReason::MintingAuthorityMismatch))?;
		let burn_micronot_collateral = local_micronot_collateral
			.checked_sub(&micronot_collateral)
			.ok_or_else(|| context.pause(GatewaySyncPauseReason::MintingAuthorityMismatch))?;
		let authority_account_id =
			authority_account_id.expect("mutated authority must expose authority account");

		// The gateway's remaining collateral is canonical here. Any local excess means collateral
		// was consumed off-chain and must be burned before the proven remainder is released.
		Self::burn_minting_authority_collateral(
			authority_account_id.clone(),
			burn_microgon_collateral,
			burn_micronot_collateral,
		)
		.map_err(|_| context.pause(GatewaySyncPauseReason::GatewayStateDrift))?;
		Self::release_minting_authority_collateral(
			authority_account_id.clone(),
			microgon_collateral,
			micronot_collateral,
		)
		.map_err(|_| context.pause(GatewaySyncPauseReason::GatewayStateDrift))?;
		Self::deposit_event(Event::MintingAuthorityDeactivationFinalized {
			source_chain: context.source_chain,
			destination_signing_key,
		});
		Ok(context.applied())
	}

	pub(crate) fn apply_proved_gateway_activity_log(
		source_chain: SourceChain,
		expected_gateway_activity_nonce: GatewayActivityNonce,
		receipt_log: EthereumReceiptLog,
	) -> GatewayActivityApplyResult<T> {
		let gateway = receipt_log.event_log.address;
		Self::ensure_supported_gateway(source_chain, &gateway)
			.map_err(GatewayActivityApplyError::from)?;
		let decoded_activity = Self::decode_evm_gateway_activity(
			source_chain,
			&receipt_log.event_log,
		)
		.map_err(|_| GatewayActivityApplyError::Pause {
			failed_gateway_activity_nonce: expected_gateway_activity_nonce.saturating_add(1),
			reason: GatewaySyncPauseReason::MalformedGatewayActivity,
		})?;
		let context = GatewayActivityContext::new(
			source_chain,
			decoded_activity.gateway_state().clone(),
			expected_gateway_activity_nonce,
		)
		.map_err(GatewayActivityApplyError::from)?;

		match decoded_activity {
			DecodedGatewayActivity::TransferToArgon { from, token, to, amount, .. } => {
				let asset = match Self::resolve_source_asset_kind(source_chain, &gateway, &token) {
					Ok(asset) => asset,
					Err(_) => return context.pause_result(GatewaySyncPauseReason::UnsupportedToken),
				};
				let transfer_adjustment = match asset {
					AssetKind::Argon => GatewayCirculationAdjustment::DecreaseArgon(amount),
					AssetKind::Argonot => GatewayCirculationAdjustment::DecreaseArgonot(amount),
				};
				context
					.ensure_gateway_circulation_matches_local_state(Some(transfer_adjustment))?;

				Self::apply_transfer_to_argon_activity(context, from, asset, to, amount)
			},
			DecodedGatewayActivity::MintingAuthorityActivated {
				destination_signing_key,
				microgon_collateral,
				micronot_collateral,
				coactivation_count,
				shared_signature_count,
				relayer_argon_account_id,
				..
			} => {
				context.ensure_gateway_circulation_matches_local_state(None)?;
				Self::apply_minting_authority_activation(
					context,
					destination_signing_key,
					microgon_collateral,
					micronot_collateral,
					coactivation_count,
					shared_signature_count,
					relayer_argon_account_id,
				)
			},
			DecodedGatewayActivity::MintingAuthorityDeactivated {
				destination_signing_key,
				microgon_collateral,
				micronot_collateral,
				..
			} => {
				context.ensure_gateway_circulation_matches_local_state(None)?;
				Self::apply_minting_authority_deactivation(
					context,
					destination_signing_key,
					microgon_collateral,
					micronot_collateral,
				)
			},
			DecodedGatewayActivity::TransferOutOfArgonFinalized {
				transfer_id,
				asset,
				amount,
				minting_authority_collateral,
				..
			} => {
				let finalized_transfer = if let Some(transfer) =
					TransferOutById::<T>::get(transfer_id)
				{
					if transfer.asset != asset || transfer.amount != amount {
						return context.pause_result(GatewaySyncPauseReason::GatewayStateDrift);
					}
					Some(match transfer.asset {
						AssetKind::Argon =>
							GatewayCirculationAdjustment::IncreaseArgon(transfer.amount),
						AssetKind::Argonot =>
							GatewayCirculationAdjustment::IncreaseArgonot(transfer.amount),
					})
				} else {
					Some(match asset {
						AssetKind::Argon => GatewayCirculationAdjustment::IncreaseArgon(amount),
						AssetKind::Argonot => GatewayCirculationAdjustment::IncreaseArgonot(amount),
					})
				};
				context.ensure_gateway_circulation_matches_local_state(finalized_transfer)?;
				Self::finalize_transfer_out_from_gateway(
					context.source_chain,
					transfer_id,
					asset,
					amount,
					minting_authority_collateral,
				)
				.map_err(|reason| context.pause(reason))?;
				Ok(context.applied())
			},
			DecodedGatewayActivity::TransferOutOfArgonCanceled { transfer_id, .. } => {
				context.ensure_gateway_circulation_matches_local_state(None)?;
				Self::cancel_transfer_out_from_gateway(context.source_chain, transfer_id)
					.map_err(|reason| context.pause(reason))?;
				Ok(context.applied())
			},
		}
	}
}
