use alloc::vec::Vec;
use argon_primitives::{AccountId as ArgonAccountId, EthereumReceiptLog, OperationalAccountsHook};
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	traits::fungible::{Inspect, Mutate, MutateHold},
};
use pallet_prelude::*;

use super::{
	AssetKind, ChainConfig, ChainConfigBySourceChain, Config,
	CouncilApprovalQueueByDestinationChainAndNonce, Error, Event, GatewayActivityNonce,
	GatewayState, GatewaySyncPauseReason, GlobalIssuanceCouncilByHash, HoldReason,
	MintingAuthoritiesBySigner, MintingAuthorityActivationRepaymentPricingByDestinationChain,
	MintingAuthorityState, Pallet, SourceChain, TransferOutById, TransferToArgonActivity, H160,
	H256,
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
		approval_hash: H256,
		gateway_state: GatewayState<T>,
	},
	GlobalIssuanceCouncilRotated {
		council_hash: H256,
		approval_hash: H256,
		gateway_state: GatewayState<T>,
	},
	MintingAuthorityDeactivated {
		destination_signing_key: H160,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
		approval_hash: H256,
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
			Self::GlobalIssuanceCouncilRotated { gateway_state, .. } |
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

	fn ensure_local_approval_hash(
		&self,
		approval_hash: H256,
	) -> Result<(), GatewayActivityApplyError> {
		let Some(queue_entry) = CouncilApprovalQueueByDestinationChainAndNonce::<T>::get(
			self.source_chain,
			self.gateway_state.argon_approvals_nonce,
		) else {
			return Err(self.pause(GatewaySyncPauseReason::GatewayStateDrift));
		};

		if queue_entry.approval_hash != approval_hash {
			return Err(self.pause(GatewaySyncPauseReason::GatewayStateDrift));
		}

		Ok(())
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
				GatewayCirculationAdjustment::DecreaseArgon(amount) => {
					if let Some(x) = expected_argon_circulation.checked_sub(&amount) {
						expected_argon_circulation = x
					} else {
						return false;
					}
				},
				GatewayCirculationAdjustment::DecreaseArgonot(amount) => {
					if let Some(x) = expected_argonot_circulation.checked_sub(&amount) {
						expected_argonot_circulation = x
					} else {
						return false;
					}
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
				let activation_base_repayment_quote = authority.activation_base_repayment_quote;
				let activation_signature_repayment_quote =
					authority.activation_signature_repayment_quote;
				let held_repayment_amount = activation_base_repayment_quote
					.saturating_add(activation_signature_repayment_quote);
				if held_repayment_amount != T::Balance::default() {
					let refund_amount;
					if authority.account_id == relayer_argon_account_id {
						repayment_amount = held_repayment_amount;
						refund_amount = T::Balance::default();
					} else {
						let actual_signature_repayment_quote = if shared_signature_count == 0 {
							T::Balance::default()
						} else {
							let pricing =
									MintingAuthorityActivationRepaymentPricingByDestinationChain::<T>::get(
										authority.destination_chain,
									)
									.ok_or(
										GatewaySyncPauseReason::MintingAuthorityActivationRepaymentMismatch,
									)?;
							let single_signature_repayment_quote =
								Self::minting_authority_activation_gas_repayment_due(
									&pricing,
									pricing.signature_gas_cost,
								)
								.map_err(|_| {
									GatewaySyncPauseReason::MintingAuthorityActivationRepaymentMismatch
								})?;
							single_signature_repayment_quote
								.saturating_mul(shared_signature_count.into())
						};
						let coactivation_count: u128 = coactivation_count.max(1).into();
						let shared_signature_repayment_quote = actual_signature_repayment_quote
							.into()
							.saturating_add(coactivation_count.saturating_sub(1))
							.saturating_div(coactivation_count);
						let requested_repayment_amount = activation_base_repayment_quote
							.saturating_add(shared_signature_repayment_quote.into());
						repayment_amount = requested_repayment_amount.min(held_repayment_amount);
						refund_amount = held_repayment_amount.saturating_sub(repayment_amount);
					}

					T::NativeCurrency::burn_held(
						&HoldReason::MintingAuthorityActivationRepayment.into(),
						&authority.account_id,
						held_repayment_amount,
						Precision::Exact,
						Fortitude::Force,
					)
					.map_err(|_| {
						GatewaySyncPauseReason::MintingAuthorityActivationRepaymentMismatch
					})?;

					if repayment_amount != T::Balance::default() &&
						T::NativeCurrency::mint_into(&relayer_argon_account_id, repayment_amount)
							.is_err()
					{
						repayment_amount = T::Balance::default();
					}
					if refund_amount != T::Balance::default() {
						let _ = T::NativeCurrency::mint_into(&authority.account_id, refund_amount);
					}
					if T::NativeCurrency::balance_on_hold(
						&HoldReason::MintingAuthorityActivationRepayment.into(),
						&authority.account_id,
					) == T::Balance::default()
					{
						let _ = frame_system::Pallet::<T>::dec_providers(&authority.account_id);
					}
				}
				authority.state = MintingAuthorityState::Active;
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

	fn apply_global_issuance_council_rotation(
		context: GatewayActivityContext<T>,
		council_hash: H256,
	) -> GatewayActivityApplyResult<T> {
		let council = GlobalIssuanceCouncilByHash::<T>::get(council_hash)
			.ok_or_else(|| context.pause(GatewaySyncPauseReason::GlobalIssuanceCouncilNotFound))?;
		let prunable_council_hash = Self::roll_active_global_issuance_council(
			context.source_chain,
			council_hash,
			council.epoch_microgons_per_argonot,
		);

		if let Some(prunable_council_hash) = prunable_council_hash {
			Self::prune_global_issuance_council_if_unreferenced(
				context.source_chain,
				prunable_council_hash,
			);
		}

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
				approval_hash,
				..
			} => {
				context.ensure_gateway_circulation_matches_local_state(None)?;
				context.ensure_local_approval_hash(approval_hash)?;
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
			DecodedGatewayActivity::GlobalIssuanceCouncilRotated {
				council_hash,
				approval_hash,
				..
			} => {
				context.ensure_gateway_circulation_matches_local_state(None)?;
				context.ensure_local_approval_hash(approval_hash)?;
				Self::apply_global_issuance_council_rotation(context, council_hash)
			},
			DecodedGatewayActivity::MintingAuthorityDeactivated {
				destination_signing_key,
				microgon_collateral,
				micronot_collateral,
				approval_hash,
				..
			} => {
				context.ensure_gateway_circulation_matches_local_state(None)?;
				context.ensure_local_approval_hash(approval_hash)?;
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

#[cfg(test)]
mod test {
	use crate::tests::*;
	use alloy_sol_types::SolEvent;
	use argon_ethereum_contracts::minting_gateway::{
		GatewayActivityState as ContractGatewayActivityState, GlobalIssuanceCouncilRotated,
		MintingAuthorityActivated,
		MintingAuthorityCollateral as ContractMintingAuthorityCollateral,
		MintingAuthorityDeactivated,
		TransferOutOfArgonCanceled as ContractTransferOutOfArgonCanceled,
		TransferOutOfArgonFinalized as ContractTransferOutOfArgonFinalized, TransferToArgonStarted,
	};
	use argon_primitives::{
		ethereum::{
			EthereumCombinedReceiptProof, EthereumExecutionBlockProof, EthereumReceiptProofReceipt,
		},
		EthereumLog, EthereumReceiptLog,
	};

	#[test]
	fn prove_gateway_activity_pays_argon_marks_recent_transfer_and_ignores_zero_amounts() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));
			assert_eq!(
				ChainConfigBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(chain_config()),
			);

			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert!(System::providers(&burn_account) > 0);
			assert_ok!(Balances::mint_into(&burn_account, 10_000));

			let recipient = account(2);
			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![argon_activity_log(
					recipient.clone(),
					1,
					1_250,
				)])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(Balances::balance(&recipient), 1_250);
			assert_eq!(
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewayState::<Test> {
					gateway_activity_nonce: 1,
					argon_approvals_nonce: 0,
					argon_circulation: 8_750,
					argonot_circulation: 0,
				}),
			);
			assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&recipient), 1);
			assert_eq!(ConfirmedTransfers::get(), vec![(recipient.clone(), 1_250)]);
			assert!(System::events().iter().any(|record| match &record.event {
				RuntimeEvent::CrosschainTransfer(Event::TransferToArgonSettled {
					source_chain: SourceChain::Ethereum,
					transfer,
				}) => {
					transfer ==
						&TransferToArgonActivity::<Test> {
							gateway_activity_nonce: 1,
							from: h160(0x11),
							asset: AssetKind::Argon,
							to: recipient.clone(),
							amount: 1_250,
						}
				},
				_ => false,
			}));
			assert!(System::events().iter().any(|record| match &record.event {
				RuntimeEvent::CrosschainTransfer(Event::GatewayStateAdvanced {
					source_chain: SourceChain::Ethereum,
					gateway_state,
				}) => {
					gateway_state ==
						&GatewayState::<Test> {
							gateway_activity_nonce: 1,
							argon_approvals_nonce: 0,
							argon_circulation: 8_750,
							argonot_circulation: 0,
						}
				},
				_ => false,
			}));
			let zero_amount_recipient = account(9);
			let zero_result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				1,
				proof_batch(vec![activity_logs(vec![argon_activity_log(
					zero_amount_recipient.clone(),
					2,
					0,
				)])]),
			);

			assert!(matches!(zero_result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(Balances::balance(&zero_amount_recipient), 0);
			assert_eq!(
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewayState::<Test> {
					gateway_activity_nonce: 2,
					argon_approvals_nonce: 0,
					argon_circulation: 8_750,
					argonot_circulation: 0,
				}),
			);
			assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&zero_amount_recipient), 0);
			assert_eq!(ConfirmedTransfers::get(), vec![(recipient, 1_250)]);
		});
	}

	#[test]
	fn prove_gateway_activity_pays_argonot_from_burn_account() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));

			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert_ok!(Ownership::mint_into(&burn_account, 777));

			let recipient = account(3);
			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![argonot_activity_log(
					recipient.clone(),
					1,
					777,
				)])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(Ownership::balance(&recipient), 777);
			assert_eq!(Ownership::balance(&burn_account), 0);
			assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&recipient), 0);
			assert!(ConfirmedTransfers::get().is_empty());
		});
	}

	#[test]
	fn prove_gateway_activity_records_minting_authority_activation_and_prunes_older_synced_queue_entries(
	) {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			let owner_vault_operator = account(31);
			let council_pair = council_signing_pair(3);
			let minting_authority_pair = council_signing_pair(4);
			let signing_key = council_signer(&minting_authority_pair);

			configure_single_member_ethereum_council(
				owner_vault_operator.clone(),
				8,
				10_000,
				&council_pair,
			);
			assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
			assert_ok!(Ownership::mint_into(&owner_vault_operator, 500));
			assert_ok!(set_committed_argonots(owner_vault_operator.clone(), 200));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				10_000,
				0,
			));
			let approval_signature = minting_authority_approval_signature(&council_pair, 1);
			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				bounded_vec![approval_signature],
			));

			assert_ok!(CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![minting_authority_activated_log(
					10_000,
					0,
					signing_key,
					owner_vault_operator.clone(),
					1,
					1,
					approval_hash_for_queue_nonce(1),
					1,
					1,
				)])]),
			));

			assert_eq!(
				MintingAuthoritiesBySigner::<Test>::get(signing_key),
				Some(MintingAuthority::<Test> {
					account_id: owner_vault_operator.clone(),
					destination_chain: SourceChain::Ethereum,
					destination_signing_key: signing_key,
					state: MintingAuthorityState::Active,
					gateway_remaining_microgon_collateral: 10_000,
					gateway_remaining_micronot_collateral: 0,
					pending_reserved_microgon_collateral: 0,
					pending_reserved_micronot_collateral: 0,
					active_pending_transfer_ids: bounded_vec![],
					activation_approval_queue_nonce: 1,
					activation_base_repayment_quote: 100,
					activation_signature_repayment_quote: 50,
					deactivation_approval_queue_nonce: None,
				}),
			);
			let activation_entry = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
				SourceChain::Ethereum,
				1,
			)
			.expect("last local synced queue entry should remain as previous-hash anchor");
			assert_eq!(
				CrosschainTransfer::previous_gateway_update_hash(SourceChain::Ethereum, 2),
				Ok(activation_entry.approval_hash),
			);
			assert_eq!(
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewayState::<Test> {
					gateway_activity_nonce: 1,
					argon_approvals_nonce: 1,
					argon_circulation: 0,
					argonot_circulation: 0,
				}),
			);
			assert_ok!(CrosschainTransfer::deactivate_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				signing_key,
			));

			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert_ok!(Balances::mint_into(&burn_account, 500));
			let recipient = account(33);
			let (argon_circulation, argonot_circulation) =
				current_gateway_circulation_after(Some((AssetKind::Argon, 5)), None);
			assert_ok!(CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				1,
				proof_batch(vec![activity_logs(vec![transfer_to_argon_started_log(
					h160(0x21),
					h160(0x11),
					h160(0x31),
					5,
					destination_bytes(&recipient),
					2,
					2,
					argon_circulation,
					argonot_circulation,
				)])]),
			));

			assert!(
				CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
					SourceChain::Ethereum,
					1
				)
				.is_none(),
				"older fully-synced queue entries should be pruned",
			);
			let retained_entry = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
				SourceChain::Ethereum,
				2,
			)
			.expect("latest local synced queue entry should be retained");
			assert_eq!(
				CrosschainTransfer::previous_gateway_update_hash(SourceChain::Ethereum, 3),
				Ok(retained_entry.approval_hash),
			);
		});
	}

	#[test]
	fn prove_gateway_activity_rotates_council_and_prunes_the_displaced_previous_snapshot() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			let council_account = account(45);
			let council_pair = council_signing_pair(13);
			let council_signer = council_signer(&council_pair);

			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));
			register_vault_operator(council_account.clone(), 18, 10_000);
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(council_account.clone()),
				SourceChain::Ethereum,
				council_signer,
				council_signer_registration_signature(&council_pair, &council_account),
			));

			LowestMicrogonsPerArgonot::set(Some(4 * argon_primitives::MICROGONS_PER_ARGON));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));
			let oldest_council_hash =
				ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
					.expect("first council should be stored");

			LowestMicrogonsPerArgonot::set(Some(5 * argon_primitives::MICROGONS_PER_ARGON));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));
			let active_council_hash =
				ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
					.expect("second council should be active");
			let mut next_council = GlobalIssuanceCouncilByHash::<Test>::get(active_council_hash)
				.expect("active council snapshot should be stored");
			next_council.epoch_microgons_per_argonot = 6 * argon_primitives::MICROGONS_PER_ARGON;
			let next_council_hash = CrosschainTransfer::hash_global_issuance_council(
				&next_council.members,
				next_council.epoch_microgons_per_argonot,
			);
			GlobalIssuanceCouncilByHash::<Test>::insert(next_council_hash, next_council);
			let mut queued_rotation = CouncilApprovalQueueEntry::<Test> {
				approving_council_hash: active_council_hash,
				target: CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(next_council_hash),
				target_payload_hash: H256::repeat_byte(0x31),
				due_frame_id: 0,
				previous_approval_hash: H256::zero(),
				approval_hash: H256::zero(),
				approved_total_weight: Default::default(),
				signatures: Default::default(),
			};
			queued_rotation.approval_hash = CrosschainTransfer::hash_council_approval_queue_entry(
				SourceChain::Ethereum,
				1,
				&queued_rotation,
			)
			.expect("rotation queue entry should hash with the configured gateway domain");
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::insert(
				SourceChain::Ethereum,
				1,
				queued_rotation,
			);

			assert_ok!(CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![global_issuance_council_rotated_log(
					next_council_hash,
					approval_hash_for_queue_nonce(1),
					1,
					1,
				)])]),
			));

			assert_eq!(
				ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum),
				Some(next_council_hash),
			);
			assert_eq!(
				CurrentTransferOutMicrogonsPerArgonotByDestinationChain::<Test>::get(
					SourceChain::Ethereum,
				),
				Some(6 * argon_primitives::MICROGONS_PER_ARGON),
			);
			assert_eq!(
				PreviousTransferOutMicrogonsPerArgonotByDestinationChain::<Test>::get(
					SourceChain::Ethereum,
				),
				Some(4 * argon_primitives::MICROGONS_PER_ARGON),
			);
			assert!(GlobalIssuanceCouncilByHash::<Test>::get(oldest_council_hash).is_none());
		});
	}

	#[test]
	fn third_party_relayed_activation_pays_relayer_and_activates_immediately() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			let owner_vault_operator = account(34);
			let relayer = account(35);
			let council_pair = council_signing_pair(5);
			let minting_authority_pair = council_signing_pair(6);
			let signing_key = council_signer(&minting_authority_pair);

			configure_single_member_ethereum_council(
				owner_vault_operator.clone(),
				10,
				10_000,
				&council_pair,
			);
			assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				10_000,
				0,
			));
			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&council_pair, 1)],
			));

			assert_noop!(
				CrosschainTransfer::deactivate_minting_authority(
					RuntimeOrigin::signed(owner_vault_operator.clone()),
					signing_key,
				),
				Error::<Test>::UnexpectedMintingAuthorityState,
			);

			assert_ok!(CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![minting_authority_activated_log(
					10_000,
					0,
					signing_key,
					relayer.clone(),
					1,
					1,
					approval_hash_for_queue_nonce(1),
					1,
					1,
				)])]),
			));

			assert_eq!(
				MintingAuthoritiesBySigner::<Test>::get(signing_key),
				Some(MintingAuthority::<Test> {
					account_id: owner_vault_operator.clone(),
					destination_chain: SourceChain::Ethereum,
					destination_signing_key: signing_key,
					state: MintingAuthorityState::Active,
					gateway_remaining_microgon_collateral: 10_000,
					gateway_remaining_micronot_collateral: 0,
					pending_reserved_microgon_collateral: 0,
					pending_reserved_micronot_collateral: 0,
					active_pending_transfer_ids: bounded_vec![],
					activation_approval_queue_nonce: 1,
					activation_base_repayment_quote: 100,
					activation_signature_repayment_quote: 50,
					deactivation_approval_queue_nonce: None,
				}),
			);
			assert_eq!(Balances::balance(&relayer), 150);

			assert_ok!(CrosschainTransfer::deactivate_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator),
				signing_key,
			));
			assert_eq!(
				MintingAuthoritiesBySigner::<Test>::get(signing_key)
					.expect("authority should stay available until proof-backed deactivation")
					.state,
				MintingAuthorityState::Deactivating,
			);
		});
	}

	#[test]
	fn third_party_relayed_activation_discards_below_ed_repayment_without_pausing() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			ExistentialDeposit::set(10_000);

			let owner_vault_operator = account(36);
			let relayer = account(37);
			let council_pair = council_signing_pair(7);
			let minting_authority_pair = council_signing_pair(8);
			let signing_key = council_signer(&minting_authority_pair);

			configure_single_member_ethereum_council(
				owner_vault_operator.clone(),
				11,
				10_000,
				&council_pair,
			);
			assert_ok!(Balances::mint_into(&owner_vault_operator, 20_000));
			assert_eq!(Balances::balance(&relayer), 0);
			assert_eq!(System::providers(&relayer), 0);

			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				10_000,
				0,
			));
			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&council_pair, 1)],
			));
			assert_eq!(
				Balances::balance_on_hold(
					&RuntimeHoldReason::CrosschainTransfer(
						HoldReason::MintingAuthorityActivationRepayment,
					),
					&owner_vault_operator,
				),
				150,
			);

			assert_ok!(CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![minting_authority_activated_log(
					10_000,
					0,
					signing_key,
					relayer.clone(),
					1,
					1,
					approval_hash_for_queue_nonce(1),
					1,
					1,
				)])]),
			));

			assert_eq!(GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum), None);
			assert_eq!(Balances::balance(&relayer), 0);
			assert_eq!(Balances::balance(&owner_vault_operator), 19_850);
			assert_eq!(System::providers(&relayer), 0);
			assert_eq!(
				Balances::balance_on_hold(
					&RuntimeHoldReason::CrosschainTransfer(
						HoldReason::MintingAuthorityActivationRepayment,
					),
					&owner_vault_operator,
				),
				0,
			);
			assert_eq!(
				MintingAuthoritiesBySigner::<Test>::get(signing_key)
					.expect("authority should activate even when the relayer payout is too small")
					.state,
				MintingAuthorityState::Active,
			);
			assert!(System::events().iter().any(|record| match &record.event {
				RuntimeEvent::CrosschainTransfer(Event::MintingAuthorityActivationCompleted {
					destination_chain,
					destination_signing_key,
					relayer_argon_account_id,
					repayment_amount,
				}) => {
					*destination_chain == SourceChain::Ethereum &&
						*destination_signing_key == signing_key &&
						relayer_argon_account_id == &relayer &&
						*repayment_amount == 0
				},
				_ => false,
			}));
		});
	}

	#[test]
	fn prove_gateway_activity_pauses_when_activation_repayment_hold_is_missing() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			let owner_vault_operator = account(40);
			let relayer = account(41);
			let council_pair = council_signing_pair(11);
			let minting_authority_pair = council_signing_pair(12);
			let signing_key = council_signer(&minting_authority_pair);

			configure_single_member_ethereum_council(
				owner_vault_operator.clone(),
				13,
				10_000,
				&council_pair,
			);
			assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				10_000,
				0,
			));
			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&council_pair, 1)],
			));
			assert_ok!(Balances::burn_held(
				&RuntimeHoldReason::CrosschainTransfer(
					HoldReason::MintingAuthorityActivationRepayment,
				),
				&owner_vault_operator,
				150,
				Precision::Exact,
				Fortitude::Force,
			));

			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![minting_authority_activated_log(
					10_000,
					0,
					signing_key,
					relayer,
					1,
					1,
					approval_hash_for_queue_nonce(1),
					1,
					1,
				)])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
			assert_eq!(
				GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewaySyncPause {
					last_good_gateway_activity_nonce: 0,
					failed_gateway_activity_nonce: 1,
					reason: GatewaySyncPauseReason::MintingAuthorityActivationRepaymentMismatch,
				}),
			);
		});
	}

	#[test]
	fn shared_activation_signature_refunds_excess_hold_back_to_authorities() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			let council_account = account(41);
			let first_authority_account = account(42);
			let second_authority_account = account(43);
			let relayer = account(44);
			let council_pair = council_signing_pair(41);
			let first_authority_pair = council_signing_pair(42);
			let second_authority_pair = council_signing_pair(43);
			let first_signing_key = council_signer(&first_authority_pair);
			let second_signing_key = council_signer(&second_authority_pair);

			configure_single_member_ethereum_council(
				council_account.clone(),
				41,
				10_000,
				&council_pair,
			);
			register_vault_operator(first_authority_account.clone(), 42, 10_000);
			register_vault_operator(second_authority_account.clone(), 43, 10_000);
			assert_ok!(Balances::mint_into(&first_authority_account, 10_000));
			assert_ok!(Balances::mint_into(&second_authority_account, 10_000));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(first_authority_account.clone()),
				SourceChain::Ethereum,
				first_signing_key,
				minting_authority_registration_signature(
					&first_authority_pair,
					&first_authority_account,
				),
				10_000,
				0,
			));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(second_authority_account.clone()),
				SourceChain::Ethereum,
				second_signing_key,
				minting_authority_registration_signature(
					&second_authority_pair,
					&second_authority_account,
				),
				10_000,
				0,
			));
			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(council_account),
				SourceChain::Ethereum,
				bounded_vec![
					minting_authority_approval_signature(&council_pair, 1),
					minting_authority_approval_signature(&council_pair, 2),
				],
			));

			assert_ok!(CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![
					minting_authority_activated_log(
						10_000,
						0,
						first_signing_key,
						relayer.clone(),
						2,
						1,
						approval_hash_for_queue_nonce(1),
						1,
						1,
					),
					minting_authority_activated_log(
						10_000,
						0,
						second_signing_key,
						relayer.clone(),
						2,
						1,
						approval_hash_for_queue_nonce(2),
						2,
						2,
					),
				])]),
			));

			assert_eq!(Balances::balance(&relayer), 250);
			assert_eq!(
				Balances::balance_on_hold(
					&RuntimeHoldReason::CrosschainTransfer(
						HoldReason::MintingAuthorityActivationRepayment,
					),
					&first_authority_account,
				),
				0,
			);
			assert_eq!(
				Balances::balance_on_hold(
					&RuntimeHoldReason::CrosschainTransfer(
						HoldReason::MintingAuthorityActivationRepayment,
					),
					&second_authority_account,
				),
				0,
			);
			assert_eq!(
				MintingAuthoritiesBySigner::<Test>::get(first_signing_key)
					.expect("first authority should activate")
					.state,
				MintingAuthorityState::Active,
			);
			assert_eq!(
				MintingAuthoritiesBySigner::<Test>::get(second_signing_key)
					.expect("second authority should activate")
					.state,
				MintingAuthorityState::Active,
			);
		});
	}

	#[test]
	fn activation_settlement_uses_emitted_shared_signature_count() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			let owner_vault_operator = account(45);
			let relayer = account(46);
			let council_pair = council_signing_pair(44);
			let minting_authority_pair = council_signing_pair(45);
			let signing_key = council_signer(&minting_authority_pair);

			configure_single_member_ethereum_council(
				owner_vault_operator.clone(),
				44,
				10_000,
				&council_pair,
			);
			assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				10_000,
				0,
			));
			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&council_pair, 1)],
			));
			assert_ok!(Balances::hold(
				&RuntimeHoldReason::CrosschainTransfer(
					HoldReason::MintingAuthorityActivationRepayment,
				),
				&owner_vault_operator,
				50,
			));
			MintingAuthoritiesBySigner::<Test>::mutate(signing_key, |authority| {
				authority
					.as_mut()
					.expect("pending activation should exist")
					.activation_signature_repayment_quote = 100;
			});

			assert_ok!(CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![minting_authority_activated_log(
					10_000,
					0,
					signing_key,
					relayer.clone(),
					1,
					1,
					approval_hash_for_queue_nonce(1),
					1,
					1,
				)])]),
			));

			assert_eq!(Balances::balance(&relayer), 150);
			assert_eq!(Balances::balance(&owner_vault_operator), 9_850);
			assert_eq!(
				Balances::balance_on_hold(
					&RuntimeHoldReason::CrosschainTransfer(
						HoldReason::MintingAuthorityActivationRepayment,
					),
					&owner_vault_operator,
				),
				0,
			);
		});
	}

	#[test]
	fn prove_gateway_activity_uses_gateway_deactivation_collateral_without_pausing() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			let owner_vault_operator = account(33);
			let council_pair = council_signing_pair(31);
			let minting_authority_pair = council_signing_pair(32);
			let signing_key = council_signer(&minting_authority_pair);

			configure_single_member_ethereum_council(
				owner_vault_operator.clone(),
				9,
				4_000,
				&council_pair,
			);
			assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
			assert_ok!(Ownership::mint_into(&owner_vault_operator, 500));
			assert_ok!(CrosschainTransfer::set_minimum_minting_authority_value(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				3_000,
			));
			assert_ok!(set_committed_argonots(owner_vault_operator.clone(), 250));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				4_000,
				250,
			));
			MintingAuthoritiesBySigner::<Test>::insert(
				signing_key,
				MintingAuthority::<Test> {
					account_id: owner_vault_operator.clone(),
					destination_chain: SourceChain::Ethereum,
					destination_signing_key: signing_key,
					state: MintingAuthorityState::Active,
					gateway_remaining_microgon_collateral: 4_000,
					gateway_remaining_micronot_collateral: 250,
					pending_reserved_microgon_collateral: 0,
					pending_reserved_micronot_collateral: 0,
					active_pending_transfer_ids: bounded_vec![],
					activation_approval_queue_nonce: 1,
					activation_base_repayment_quote: 0,
					activation_signature_repayment_quote: 0,
					deactivation_approval_queue_nonce: None,
				},
			);

			assert_ok!(CrosschainTransfer::deactivate_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				signing_key,
			));

			assert_ok!(CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![minting_authority_deactivated_log(
					3_000,
					200,
					signing_key,
					account(35),
					approval_hash_for_queue_nonce(2),
					1,
					2,
				)])]),
			));

			assert_eq!(MintingAuthoritiesBySigner::<Test>::get(signing_key), None);
			assert_eq!(encumbered_bond_microgons(&owner_vault_operator), 0);
			assert_eq!(encumbered_argonot_micronots(&owner_vault_operator), 0);
			assert_eq!(active_bond_microgons(&owner_vault_operator), 3_000);
			assert_eq!(committed_argonot_micronots(&owner_vault_operator), 200);
			assert_eq!(GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum), None,);
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				3_000,
				200,
			));
			assert_eq!(
				MintingAuthoritiesBySigner::<Test>::get(signing_key),
				Some(MintingAuthority::<Test> {
					account_id: owner_vault_operator,
					destination_chain: SourceChain::Ethereum,
					destination_signing_key: signing_key,
					state: MintingAuthorityState::PendingActivation,
					gateway_remaining_microgon_collateral: 3_000,
					gateway_remaining_micronot_collateral: 200,
					pending_reserved_microgon_collateral: 0,
					pending_reserved_micronot_collateral: 0,
					active_pending_transfer_ids: bounded_vec![],
					activation_approval_queue_nonce: 3,
					activation_base_repayment_quote: 100,
					activation_signature_repayment_quote: 50,
					deactivation_approval_queue_nonce: None,
				}),
			);
			assert_eq!(
				encumbered_bond_microgons(
					&MintingAuthoritiesBySigner::<Test>::get(signing_key)
						.expect("minting authority should be re-registered")
						.account_id,
				),
				3_000,
			);
			assert_eq!(
				encumbered_argonot_micronots(
					&MintingAuthoritiesBySigner::<Test>::get(signing_key)
						.expect("minting authority should be re-registered")
						.account_id,
				),
				200,
			);
		});
	}

	#[test]
	fn prove_gateway_activity_deactivation_invalidates_all_pending_signer_reservations() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			let authority_account = account(36);
			let authority_pair = council_signing_pair(37);
			let signing_key = activate_test_minting_authority(
				authority_account.clone(),
				48,
				20_000,
				&council_signing_pair(38),
				&authority_pair,
				12_000,
				0,
			);
			let first_user = account(39);
			let second_user = account(40);

			assert_ok!(Balances::mint_into(&first_user, 5_000));
			assert_ok!(Balances::mint_into(&second_user, 4_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(first_user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x61),
				4_000,
			));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(second_user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x62),
				3_000,
			));

			let first_transfer_id = transfer_out_id(&first_user, 1);
			let second_transfer_id = transfer_out_id(&second_user, 1);
			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account.clone()),
				first_transfer_id,
				transfer_collateral_signature(&authority_pair, first_transfer_id, 4_000, 0),
				4_000,
				0,
			));
			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account),
				second_transfer_id,
				transfer_collateral_signature(&authority_pair, second_transfer_id, 3_000, 0),
				3_000,
				0,
			));

			assert_ok!(CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				1,
				proof_batch(vec![activity_logs(vec![minting_authority_deactivated_log(
					12_000,
					0,
					signing_key,
					account(41),
					approval_hash_for_queue_nonce(1),
					2,
					1,
				)])]),
			));

			assert_eq!(MintingAuthoritiesBySigner::<Test>::get(signing_key), None);
			let first_transfer = TransferOutById::<Test>::get(first_transfer_id)
				.expect("first transfer should remain");
			let second_transfer = TransferOutById::<Test>::get(second_transfer_id)
				.expect("second transfer should remain");
			assert_eq!(first_transfer.state, TransferOutState::Started);
			assert_eq!(second_transfer.state, TransferOutState::Started);
			assert_eq!(first_transfer.total_attached_collateral, 0);
			assert_eq!(second_transfer.total_attached_collateral, 0);
			assert!(first_transfer
				.minting_authority_collateral_by_signer
				.get(&signing_key)
				.is_none());
			assert!(second_transfer
				.minting_authority_collateral_by_signer
				.get(&signing_key)
				.is_none());
			let pending_requests =
				PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum);
			assert_eq!(pending_requests.len(), 2);
			assert!(pending_requests.contains(&PendingCollateralizationRequest::<Test> {
				transfer_id: first_transfer_id,
				remaining_collateral: 4_000,
				remaining_minting_authority_tip: 4,
			}));
			assert!(pending_requests.contains(&PendingCollateralizationRequest::<Test> {
				transfer_id: second_transfer_id,
				remaining_collateral: 3_000,
				remaining_minting_authority_tip: 3,
			}));
			assert_eq!(GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum), None);
			assert!(System::events().iter().any(|record| match &record.event {
				RuntimeEvent::CrosschainTransfer(Event::TransferCollateralInvalidated {
					transfer_id,
					destination_signing_key,
				}) => *transfer_id == first_transfer_id && *destination_signing_key == signing_key,
				_ => false,
			}));
			assert!(System::events().iter().any(|record| match &record.event {
				RuntimeEvent::CrosschainTransfer(Event::TransferCollateralInvalidated {
					transfer_id,
					destination_signing_key,
				}) => *transfer_id == second_transfer_id && *destination_signing_key == signing_key,
				_ => false,
			}));
		});
	}

	#[test]
	fn prove_gateway_activity_rolls_back_failed_finalized_activity_before_pausing() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			let council_account = account(170);
			let council_pair = council_signing_pair(171);
			let user = account(172);

			configure_single_member_ethereum_council(
				council_account.clone(),
				145,
				20_000,
				&council_pair,
			);
			assert_ok!(Balances::mint_into(&user, 25_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x58),
				20_000,
			));

			let transfer_id = transfer_out_id(&user, 1);
			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![transfer_out_of_argon_finalized_log(
					transfer_id,
					vec![(h160(0xad), 20_000, 0)],
					1,
					0,
				)])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
			assert_eq!(
				GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewaySyncPause {
					last_good_gateway_activity_nonce: 0,
					failed_gateway_activity_nonce: 1,
					reason: GatewaySyncPauseReason::MintingAuthorityNotFound,
				}),
			);
			assert_eq!(
				TransferOutById::<Test>::get(transfer_id).map(|transfer| transfer.amount),
				Some(20_000),
			);
			assert_eq!(
				PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum),
				vec![PendingCollateralizationRequest::<Test> {
					transfer_id,
					remaining_collateral: 20_000,
					remaining_minting_authority_tip: 20,
				}],
			);
			assert_eq!(
				NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
				1,
			);
			assert_eq!(
				PendingTransferOutCirculationByDestinationChain::<Test>::get(SourceChain::Ethereum)
					.argon_circulation,
				20_000,
			);
		});
	}

	#[test]
	fn prove_gateway_activity_rolls_back_finalized_activity_when_collateral_exceeds_remaining() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			let authority_account = account(175);
			let authority_pair = council_signing_pair(92);
			let signing_key = activate_test_minting_authority(
				authority_account.clone(),
				46,
				20_000,
				&council_signing_pair(93),
				&authority_pair,
				10_000,
				0,
			);
			let user = account(176);

			assert_ok!(Balances::mint_into(&user, 6_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x5a),
				5_000,
			));
			let transfer_id = transfer_out_id(&user, 1);
			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account),
				transfer_id,
				transfer_collateral_signature(&authority_pair, transfer_id, 5_000, 0),
				5_000,
				0,
			));

			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				1,
				proof_batch(vec![activity_logs(vec![transfer_out_of_argon_finalized_log(
					transfer_id,
					vec![(signing_key, 11_000, 0)],
					2,
					1,
				)])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewayState::<Test> {
					gateway_activity_nonce: 1,
					argon_approvals_nonce: 1,
					argon_circulation: 0,
					argonot_circulation: 0,
				}),
			);
			assert_eq!(
				GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewaySyncPause {
					last_good_gateway_activity_nonce: 1,
					failed_gateway_activity_nonce: 2,
					reason: GatewaySyncPauseReason::MintingAuthorityMismatch,
				}),
			);

			let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
				.expect("authority should remain registered");
			let transfer =
				TransferOutById::<Test>::get(transfer_id).expect("transfer should remain stored");
			assert_eq!(authority.gateway_remaining_microgon_collateral, 10_000);
			assert_eq!(authority.pending_reserved_microgon_collateral, 5_000);
			assert_eq!(transfer.state, TransferOutState::Ready);
			assert_eq!(transfer.total_attached_collateral, 5_000);
			assert_eq!(
				transfer
					.minting_authority_collateral_by_signer
					.get(&signing_key)
					.map(|row| row.microgon_collateral),
				Some(5_000),
			);
			assert_eq!(authority.active_pending_transfer_ids, vec![transfer_id],);
			assert!(PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum)
				.is_empty());
			assert_eq!(
				NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
				1,
			);
			assert_eq!(
				PendingTransferOutCirculationByDestinationChain::<Test>::get(SourceChain::Ethereum)
					.argon_circulation,
				5_000,
			);
		});
	}

	#[test]
	fn prove_gateway_activity_mints_unknown_finalized_principal_into_burn_account() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			let authority_account = account(179);
			let authority_pair = council_signing_pair(96);
			let signing_key = activate_test_minting_authority(
				authority_account.clone(),
				47,
				20_000,
				&council_signing_pair(97),
				&authority_pair,
				10_000,
				0,
			);
			let user = account(180);

			assert_ok!(Balances::mint_into(&user, 6_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x5c),
				5_000,
			));
			let local_transfer_id = transfer_out_id(&user, 1);
			let external_transfer_id = H256::repeat_byte(0x93);
			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account.clone()),
				local_transfer_id,
				transfer_collateral_signature(&authority_pair, local_transfer_id, 5_000, 0),
				5_000,
				0,
			));
			let (argon_circulation, argonot_circulation) =
				current_gateway_circulation_after(None, Some((AssetKind::Argon, 5_000)));

			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				1,
				proof_batch(vec![activity_logs(vec![
					transfer_out_of_argon_finalized_log_with_circulation(
						external_transfer_id,
						AssetKind::Argon,
						5_000,
						vec![(signing_key, 5_000, 0)],
						2,
						1,
						argon_circulation,
						argonot_circulation,
					),
				])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum), None);
			assert_eq!(
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewayState::<Test> {
					gateway_activity_nonce: 2,
					argon_approvals_nonce: 1,
					argon_circulation: 5_000,
					argonot_circulation: 0,
				}),
			);
			assert_eq!(
				Balances::balance(&CrosschainTransfer::burn_account(SourceChain::Ethereum)),
				10_000,
			);
			let local_transfer = TransferOutById::<Test>::get(local_transfer_id)
				.expect("local transfer should remain after unrelated unknown finalization");
			assert!(TransferOutById::<Test>::get(external_transfer_id).is_none());
			assert_eq!(local_transfer.state, TransferOutState::Ready);
			assert_eq!(local_transfer.total_attached_collateral, 5_000);
			assert_eq!(local_transfer.minting_authority_collateral_by_signer.len(), 1);
			assert_eq!(encumbered_bond_microgons(&authority_account), 5_000);
		});
	}

	#[test]
	fn prove_gateway_activity_pauses_when_unknown_finalized_burn_cleanup_fails() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			let authority_account = account(179);
			let authority_pair = council_signing_pair(96);
			let signing_key = activate_test_minting_authority(
				authority_account.clone(),
				47,
				20_000,
				&council_signing_pair(97),
				&authority_pair,
				10_000,
				0,
			);
			let user = account(180);

			assert_ok!(Balances::mint_into(&user, 6_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x5c),
				5_000,
			));
			let local_transfer_id = transfer_out_id(&user, 1);
			let external_transfer_id = H256::repeat_byte(0x93);
			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account.clone()),
				local_transfer_id,
				transfer_collateral_signature(&authority_pair, local_transfer_id, 5_000, 0),
				5_000,
				0,
			));
			crate::mock::EncumberedBondMicrogonsByAccount::mutate(|entries| {
				entries.insert(authority_account.clone(), 1_000);
			});
			let (argon_circulation, argonot_circulation) =
				current_gateway_circulation_after(None, Some((AssetKind::Argon, 5_000)));

			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				1,
				proof_batch(vec![activity_logs(vec![
					transfer_out_of_argon_finalized_log_with_circulation(
						external_transfer_id,
						AssetKind::Argon,
						5_000,
						vec![(signing_key, 5_000, 0)],
						2,
						1,
						argon_circulation,
						argonot_circulation,
					),
				])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewayState::<Test> {
					gateway_activity_nonce: 1,
					argon_approvals_nonce: 1,
					argon_circulation: 0,
					argonot_circulation: 0,
				}),
			);
			assert_eq!(
				GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewaySyncPause {
					last_good_gateway_activity_nonce: 1,
					failed_gateway_activity_nonce: 2,
					reason: GatewaySyncPauseReason::GatewayStateDrift,
				}),
			);
			assert_eq!(
				Balances::balance(&CrosschainTransfer::burn_account(SourceChain::Ethereum)),
				5_000,
			);
			let local_transfer = TransferOutById::<Test>::get(local_transfer_id)
				.expect("local transfer should remain after unknown finalization");
			assert!(TransferOutById::<Test>::get(external_transfer_id).is_none());
			assert_eq!(local_transfer.state, TransferOutState::Ready);
			assert_eq!(local_transfer.total_attached_collateral, 5_000);
			assert_eq!(local_transfer.minting_authority_collateral_by_signer.len(), 1);
			assert_eq!(encumbered_bond_microgons(&authority_account), 1_000);
		});
	}

	#[test]
	fn prove_gateway_activity_rolls_back_failed_canceled_activity_before_pausing() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			let council_account = account(177);
			let council_pair = council_signing_pair(94);
			let user = account(178);

			configure_single_member_ethereum_council(
				council_account.clone(),
				146,
				20_000,
				&council_pair,
			);
			assert_ok!(Balances::mint_into(&user, 25_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x5b),
				20_000,
			));

			let transfer_id = transfer_out_id(&user, 1);
			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			let _ = Balances::burn_from(
				&burn_account,
				20_000,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)
			.expect("draining the burn account should succeed in the mock");

			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![transfer_out_of_argon_canceled_log(
					transfer_id,
					1,
					0,
				)])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
			assert_eq!(
				GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewaySyncPause {
					last_good_gateway_activity_nonce: 0,
					failed_gateway_activity_nonce: 1,
					reason: GatewaySyncPauseReason::GatewayStateDrift,
				}),
			);
			assert_eq!(
				TransferOutById::<Test>::get(transfer_id).map(|transfer| transfer.amount),
				Some(20_000),
			);
			assert_eq!(
				PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum),
				vec![PendingCollateralizationRequest::<Test> {
					transfer_id,
					remaining_collateral: 20_000,
					remaining_minting_authority_tip: 20,
				}],
			);
			assert_eq!(
				NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
				1,
			);
			assert_eq!(
				PendingTransferOutCirculationByDestinationChain::<Test>::get(SourceChain::Ethereum)
					.argon_circulation,
				20_000,
			);
			assert_eq!(Balances::balance(&burn_account), 0);
			assert_eq!(
				Balances::balance_on_hold(
					&RuntimeHoldReason::CrosschainTransfer(
						HoldReason::TransferOutMintingAuthorityTip,
					),
					&user,
				),
				20,
			);
		});
	}

	#[test]
	fn prove_gateway_activity_pauses_without_advancing_and_refunds_submitter() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));

			let signing_key = council_signer(&council_signing_pair(55));
			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![minting_authority_activated_log(
					9_000,
					0,
					signing_key,
					account(56),
					1,
					1,
					H256::zero(),
					1,
					4,
				)])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
			assert_eq!(
				GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewaySyncPause {
					last_good_gateway_activity_nonce: 0,
					failed_gateway_activity_nonce: 1,
					reason: GatewaySyncPauseReason::GatewayStateDrift,
				}),
			);
			assert!(System::events().iter().any(|record| match &record.event {
				RuntimeEvent::CrosschainTransfer(Event::GatewaySyncPaused {
					source_chain: SourceChain::Ethereum,
					pause,
				}) => {
					*pause ==
						GatewaySyncPause {
							last_good_gateway_activity_nonce: 0,
							failed_gateway_activity_nonce: 1,
							reason: GatewaySyncPauseReason::GatewayStateDrift,
						}
				},
				_ => false,
			}));
		});

		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));
			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert_ok!(Balances::mint_into(&burn_account, 10_000));

			GatewayStateBySourceChain::<Test>::insert(
				SourceChain::Ethereum,
				GatewayState::<Test> {
					gateway_activity_nonce: 0,
					argon_approvals_nonce: 0,
					argon_circulation: 10_000,
					argonot_circulation: 0,
				},
			);

			let recipient = account(57);
			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![transfer_to_argon_started_log(
					h160(0x21),
					h160(0x11),
					h160(0x31),
					1_250,
					destination_bytes(&recipient),
					1,
					0,
					0,
					0,
				)])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(Balances::balance(&recipient), 0);
			assert_eq!(
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewayState::<Test> {
					gateway_activity_nonce: 0,
					argon_approvals_nonce: 0,
					argon_circulation: 10_000,
					argonot_circulation: 0,
				}),
			);
			assert_eq!(
				GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewaySyncPause {
					last_good_gateway_activity_nonce: 0,
					failed_gateway_activity_nonce: 1,
					reason: GatewaySyncPauseReason::GatewayStateDrift,
				}),
			);
		});
	}

	#[test]
	fn prove_gateway_activity_rejects_empty_proof_inputs() {
		new_test_ext().execute_with(|| {
			assert_noop!(
				CrosschainTransfer::prove_gateway_activity(
					RuntimeOrigin::signed(account(1)),
					SourceChain::Ethereum,
					0,
					GatewayActivityProofBatch::<Test> {
						execution_block_proof: dummy_execution_block_proof(),
						blocks: Vec::new()
							.try_into()
							.expect("empty proof batch stays within pallet block bound"),
					},
				),
				Error::<Test>::NoGatewayProofBlocksProvided,
			);
			assert_noop!(
				CrosschainTransfer::prove_gateway_activity(
					RuntimeOrigin::signed(account(1)),
					SourceChain::Ethereum,
					0,
					proof_batch(vec![activity_logs(vec![])]),
				),
				Error::<Test>::NoGatewayActivitiesProvided,
			);
		});
	}

	#[test]
	fn prove_gateway_activity_accepts_anchor_target_block_number_for_anchor_path() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));
			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert_ok!(Balances::mint_into(&burn_account, 10_000));
			let recipient = account(2);

			assert_ok!(CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				GatewayActivityProofBatch::<Test> {
					execution_block_proof: dummy_execution_block_proof(),
					blocks: vec![GatewayActivityProofBlock::<Test> {
						target_block_number: 0,
						receipt_proof: dummy_receipt_proof(),
						receipt_logs: activity_logs(vec![argon_activity_log(
							recipient.clone(),
							1,
							10
						)]),
					}]
					.try_into()
					.expect("single proof block stays within pallet block bound"),
				},
			),);
			assert_eq!(Balances::balance(&recipient), 10);
		});
	}

	#[test]
	fn prove_gateway_activity_pool_key_uses_payload_not_signer() {
		new_test_ext().execute_with(|| {
			let first_call =
				RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
					source_chain: SourceChain::Ethereum,
					previous_gateway_activity_nonce: 0,
					proof_batch: proof_batch(vec![activity_logs(vec![argon_activity_log(
						account(2),
						1,
						1_250,
					)])]),
				});
			let retry_call =
				RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
					source_chain: SourceChain::Ethereum,
					previous_gateway_activity_nonce: 0,
					proof_batch: proof_batch(vec![activity_logs(vec![argon_activity_log(
						account(2),
						1,
						1_250,
					)])]),
				});
			let different_sender_same_nonce_call =
				RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
					source_chain: SourceChain::Ethereum,
					previous_gateway_activity_nonce: 0,
					proof_batch: proof_batch(vec![activity_logs(vec![
						transfer_to_argon_started_log(
							h160(0x21),
							h160(0x99),
							h160(0x31),
							1_250,
							destination_bytes(&account(2)),
							1,
							0,
							8_750,
							0,
						),
					])]),
				});
			let different_nonce_call =
				RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
					source_chain: SourceChain::Ethereum,
					previous_gateway_activity_nonce: 1,
					proof_batch: proof_batch(vec![activity_logs(vec![argon_activity_log(
						account(2),
						2,
						1_250,
					)])]),
				});

			let first_key = <CrosschainTransfer as CallTxPoolKeyProvider<
				RuntimeCall,
				TestAccountId,
			>>::key_for(&first_call, Some(&account(1)))
			.expect("prove_gateway_activity should publish a pool key");
			let retry_key = <CrosschainTransfer as CallTxPoolKeyProvider<
				RuntimeCall,
				TestAccountId,
			>>::key_for(&retry_call, Some(&account(9)))
			.expect("prove_gateway_activity retry should publish a pool key");
			let different_sender_same_nonce_key =
				<CrosschainTransfer as CallTxPoolKeyProvider<RuntimeCall, TestAccountId>>::key_for(
					&different_sender_same_nonce_call,
					Some(&account(9)),
				)
				.expect("prove_gateway_activity with same first nonce should publish a pool key");
			let different_nonce_key = <CrosschainTransfer as CallTxPoolKeyProvider<
				RuntimeCall,
				TestAccountId,
			>>::key_for(&different_nonce_call, Some(&account(1)))
			.expect("prove_gateway_activity with different nonce should publish a pool key");

			assert_eq!(first_key, retry_key);
			assert_ne!(first_key, different_sender_same_nonce_key);
			assert_ne!(first_key, different_nonce_key);
		});
	}

	#[test]
	fn prove_gateway_activity_validate_marks_stale_cases() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));

			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert_ok!(Balances::mint_into(&burn_account, 10_000));

			assert_ok!(CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![argon_activity_log(account(2), 1, 500)])]),
			));

			let stale_call =
				RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
					source_chain: SourceChain::Ethereum,
					previous_gateway_activity_nonce: 0,
					proof_batch: proof_batch(vec![activity_logs(vec![argon_activity_log(
						account(3),
						2,
						750,
					)])]),
				});

			assert!(matches!(
			<CrosschainTransfer as CallTxValidityProvider<RuntimeCall, TestAccountId>>::validate(
				&stale_call,
				Some(&account(1)),
			),
			Err(TransactionValidityError::Invalid(InvalidTransaction::Stale))
		));
		});

		new_test_ext().execute_with(|| {
			GatewaySyncPauseBySourceChain::<Test>::insert(
				SourceChain::Ethereum,
				GatewaySyncPause {
					last_good_gateway_activity_nonce: 4,
					failed_gateway_activity_nonce: 5,
					reason: GatewaySyncPauseReason::GatewayStateDrift,
				},
			);

			let paused_call =
				RuntimeCall::CrosschainTransfer(Call::<Test>::prove_gateway_activity {
					source_chain: SourceChain::Ethereum,
					previous_gateway_activity_nonce: 4,
					proof_batch: proof_batch(vec![activity_logs(vec![argon_activity_log(
						account(3),
						5,
						750,
					)])]),
				});

			assert!(matches!(
			<CrosschainTransfer as CallTxValidityProvider<RuntimeCall, TestAccountId>>::validate(
				&paused_call,
				Some(&account(1)),
			),
			Err(TransactionValidityError::Invalid(InvalidTransaction::Stale))
		));
		});
	}

	#[test]
	fn prove_gateway_activity_refunds_batch_that_advances_before_pausing() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));

			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert_ok!(Balances::mint_into(&burn_account, 10_000));
			let recipient = account(2);
			let signing_key = council_signer(&council_signing_pair(55));

			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![
					argon_activity_log(recipient.clone(), 1, 500),
					minting_authority_activated_log(
						9_000,
						0,
						signing_key,
						account(56),
						1,
						1,
						H256::zero(),
						2,
						4,
					),
				])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(Balances::balance(&recipient), 500);
			assert_eq!(
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewayState::<Test> {
					gateway_activity_nonce: 1,
					argon_approvals_nonce: 0,
					argon_circulation: 9_500,
					argonot_circulation: 0,
				}),
			);
			assert_eq!(
				GatewaySyncPauseBySourceChain::<Test>::get(SourceChain::Ethereum),
				Some(GatewaySyncPause {
					last_good_gateway_activity_nonce: 1,
					failed_gateway_activity_nonce: 2,
					reason: GatewaySyncPauseReason::GatewayStateDrift,
				}),
			);
		});
	}

	#[test]
	fn prove_gateway_activity_accepts_contiguous_batch_across_proof_blocks() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));

			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert_ok!(Balances::mint_into(&burn_account, 10_000));

			let first_recipient = account(2);
			let second_recipient = account(3);

			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![
					activity_logs(vec![argon_activity_log(first_recipient.clone(), 1, 500)]),
					activity_logs(vec![argon_activity_log_with_circulation(
						second_recipient.clone(),
						2,
						750,
						8_750,
						0,
					)]),
				]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(Balances::balance(&first_recipient), 500);
			assert_eq!(Balances::balance(&second_recipient), 750);
			assert_eq!(
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum)
					.expect("gateway state should be written")
					.gateway_activity_nonce,
				2,
			);
			assert_eq!(
				ConfirmedTransfers::get(),
				vec![(first_recipient.clone(), 500), (second_recipient.clone(), 750)],
			);
		});
	}

	#[test]
	fn prove_gateway_activity_accepts_multiple_logs_from_one_receipt_in_order() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));

			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert_ok!(Balances::mint_into(&burn_account, 10_000));

			let first_recipient = account(10);
			let second_recipient = account(11);

			let result = CrosschainTransfer::prove_gateway_activity(
				RuntimeOrigin::signed(account(1)),
				SourceChain::Ethereum,
				0,
				proof_batch(vec![activity_logs(vec![
					argon_activity_log(first_recipient.clone(), 1, 400),
					argon_activity_log_with_circulation(second_recipient.clone(), 2, 600, 9_000, 0),
				])]),
			);

			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));
			assert_eq!(Balances::balance(&first_recipient), 400);
			assert_eq!(Balances::balance(&second_recipient), 600);
			assert_eq!(
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum)
					.expect("gateway state should be written")
					.gateway_activity_nonce,
				2,
			);
			assert_eq!(
				ConfirmedTransfers::get(),
				vec![(first_recipient.clone(), 400), (second_recipient.clone(), 600)],
			);
		});
	}

	#[test]
	fn prove_gateway_activity_rejects_invalid_batches_without_partial_settlement() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));

			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert_ok!(Balances::mint_into(&burn_account, 10_000));

			let first_recipient = account(12);
			let second_recipient = account(13);

			assert_noop!(
				CrosschainTransfer::prove_gateway_activity(
					RuntimeOrigin::signed(account(1)),
					SourceChain::Ethereum,
					0,
					proof_batch(vec![activity_logs(vec![
						argon_activity_log(first_recipient.clone(), 2, 400),
						argon_activity_log(second_recipient.clone(), 1, 600),
					])]),
				),
				Error::<Test>::UnexpectedGatewayActivityNonce,
			);

			assert_eq!(Balances::balance(&first_recipient), 0);
			assert_eq!(Balances::balance(&second_recipient), 0);
			assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
			assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&first_recipient), 0);
			assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&second_recipient), 0);
			assert!(
				!System::events()
					.iter()
					.any(|record| { matches!(record.event, RuntimeEvent::CrosschainTransfer(_)) }),
				"transactional batch failure should roll back crosschain events",
			);
			ProofVerificationRejectedTransactionIndexes::set(vec![1]);

			let first_recipient = account(14);
			let second_recipient = account(15);

			assert_noop!(
				CrosschainTransfer::prove_gateway_activity(
					RuntimeOrigin::signed(account(1)),
					SourceChain::Ethereum,
					0,
					proof_batch(vec![
						activity_logs_for_transaction_index(
							vec![argon_activity_log(first_recipient.clone(), 1, 500)],
							0,
						),
						activity_logs_for_transaction_index(
							vec![argon_activity_log_with_circulation(
								second_recipient.clone(),
								2,
								750,
								8_750,
								0,
							)],
							1,
						),
					]),
				),
				Error::<Test>::InvalidProof,
			);

			assert_eq!(Balances::balance(&first_recipient), 0);
			assert_eq!(Balances::balance(&second_recipient), 0);
			assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
			assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&first_recipient), 0);
			assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&second_recipient), 0);
			assert!(
				!System::events()
					.iter()
					.any(|record| { matches!(record.event, RuntimeEvent::CrosschainTransfer(_)) }),
				"invalid later proof block should not leave partial crosschain events behind",
			);
			ProofVerificationRejectedTransactionIndexes::set(vec![]);

			let first_recipient = account(16);
			let second_recipient = account(17);

			assert_noop!(
				CrosschainTransfer::prove_gateway_activity(
					RuntimeOrigin::signed(account(1)),
					SourceChain::Ethereum,
					0,
					proof_batch(vec![activity_logs(vec![
						argon_activity_log(first_recipient.clone(), 1, 500),
						argon_activity_log_with_circulation(
							second_recipient.clone(),
							3,
							750,
							8_750,
							0
						),
					])]),
				),
				Error::<Test>::UnexpectedGatewayActivityNonce,
			);

			assert_eq!(Balances::balance(&first_recipient), 0);
			assert_eq!(Balances::balance(&second_recipient), 0);
			assert_eq!(GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum), None);
			assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&first_recipient), 0);
			assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&second_recipient), 0);
			assert!(
				!System::events()
					.iter()
					.any(|record| { matches!(record.event, RuntimeEvent::CrosschainTransfer(_)) }),
				"invalid later activity should not leave partial crosschain events behind",
			);
		});
	}

	#[test]
	fn prove_gateway_activity_rejects_unsupported_gateway() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));

			let recipient = account(4);
			assert_noop!(
				CrosschainTransfer::prove_gateway_activity(
					RuntimeOrigin::signed(account(1)),
					SourceChain::Ethereum,
					0,
					proof_batch(vec![activity_logs(vec![transfer_to_argon_started_log(
						h160(0x44),
						h160(0x11),
						h160(0x31),
						5,
						destination_bytes(&recipient),
						1,
						0,
						0,
						0,
					)])]),
				),
				Error::<Test>::UnsupportedGateway,
			);
		});
	}

	#[test]
	fn prove_gateway_activity_rejects_invalid_gateway_activity_progression() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));

			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert_ok!(Balances::mint_into(&burn_account, 10_000));

			let recipient = account(5);
			assert_noop!(
				CrosschainTransfer::prove_gateway_activity(
					RuntimeOrigin::signed(account(1)),
					SourceChain::Ethereum,
					1,
					proof_batch(vec![activity_logs(vec![argon_activity_log(
						recipient.clone(),
						1,
						10,
					)])]),
				),
				Error::<Test>::UnexpectedPreviousGatewayActivityNonce,
			);
			assert_noop!(
				CrosschainTransfer::prove_gateway_activity(
					RuntimeOrigin::signed(account(1)),
					SourceChain::Ethereum,
					0,
					proof_batch(vec![activity_logs(vec![argon_activity_log(
						recipient.clone(),
						2,
						10,
					)])]),
				),
				Error::<Test>::UnexpectedGatewayActivityNonce,
			);
		});
	}

	#[test]
	fn invalid_proof_from_provider_is_rejected() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));

			ProofVerificationAllowed::set(false);

			assert_noop!(
				CrosschainTransfer::prove_gateway_activity(
					RuntimeOrigin::signed(account(1)),
					SourceChain::Ethereum,
					0,
					proof_batch(vec![activity_logs(vec![argon_activity_log(account(8), 1, 100)])]),
				),
				Error::<Test>::InvalidProof,
			);
		});
	}

	fn argon_activity_log(
		recipient: TestAccountId,
		gateway_activity_nonce: GatewayActivityNonce,
		amount: Balance,
	) -> EthereumLog {
		let (argon_circulation, argonot_circulation) =
			current_gateway_circulation_after(Some((AssetKind::Argon, amount)), None);
		argon_activity_log_with_circulation(
			recipient,
			gateway_activity_nonce,
			amount,
			argon_circulation,
			argonot_circulation,
		)
	}

	fn argon_activity_log_with_circulation(
		recipient: TestAccountId,
		gateway_activity_nonce: GatewayActivityNonce,
		amount: Balance,
		argon_circulation: Balance,
		argonot_circulation: Balance,
	) -> EthereumLog {
		transfer_to_argon_started_log(
			h160(0x21),
			h160(0x11),
			h160(0x31),
			amount,
			destination_bytes(&recipient),
			gateway_activity_nonce,
			0,
			argon_circulation,
			argonot_circulation,
		)
	}

	fn argonot_activity_log(
		recipient: TestAccountId,
		gateway_activity_nonce: GatewayActivityNonce,
		amount: Balance,
	) -> EthereumLog {
		let (argon_circulation, argonot_circulation) =
			current_gateway_circulation_after(Some((AssetKind::Argonot, amount)), None);
		transfer_to_argon_started_log(
			h160(0x21),
			h160(0x11),
			h160(0x32),
			amount,
			destination_bytes(&recipient),
			gateway_activity_nonce,
			0,
			argon_circulation,
			argonot_circulation,
		)
	}

	fn approval_hash_for_queue_nonce(queue_nonce: ArgonApprovalsNonce) -> H256 {
		CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
			SourceChain::Ethereum,
			queue_nonce,
		)
		.expect("queued council approval should exist for the proven gateway activity")
		.approval_hash
	}

	fn global_issuance_council_rotated_log(
		council_hash: H256,
		approval_hash: H256,
		gateway_activity_nonce: GatewayActivityNonce,
		argon_approvals_nonce: ArgonApprovalsNonce,
	) -> EthereumLog {
		let (argon_circulation, argonot_circulation) =
			current_gateway_circulation_after(None, None);
		let event = GlobalIssuanceCouncilRotated {
			councilHash: alloy_primitives::B256::from(council_hash.0),
			approvalHash: alloy_primitives::B256::from(approval_hash.0),
			relayerArgonAccountId: destination_bytes(&account(46)).into(),
			gatewayState: contract_gateway_activity_state(
				gateway_activity_nonce,
				argon_approvals_nonce,
				argon_circulation,
				argonot_circulation,
			),
		};

		EthereumLog {
			address: h160(0x21),
			topics: vec![H256::from_slice(GlobalIssuanceCouncilRotated::SIGNATURE_HASH.as_slice())]
				.try_into()
				.expect("topics stay within Ethereum log topic bounds"),
			data: event
				.encode_data()
				.try_into()
				.expect("council rotation data stays within bounded log payload"),
		}
	}

	#[allow(clippy::too_many_arguments)]
	fn minting_authority_activated_log(
		microgon_collateral: Balance,
		micronot_collateral: Balance,
		destination_signing_key: H160,
		relayer_argon_account_id: TestAccountId,
		coactivation_count: u32,
		shared_signature_count: u32,
		approval_hash: H256,
		gateway_activity_nonce: GatewayActivityNonce,
		argon_approvals_nonce: ArgonApprovalsNonce,
	) -> EthereumLog {
		let (argon_circulation, argonot_circulation) =
			current_gateway_circulation_after(None, None);
		let mut data = Vec::with_capacity(320);
		data.extend_from_slice(&u128_word(microgon_collateral));
		data.extend_from_slice(&u128_word(micronot_collateral));
		data.extend_from_slice(&u64_word(coactivation_count as u64));
		data.extend_from_slice(&u64_word(shared_signature_count as u64));
		data.extend_from_slice(approval_hash.as_bytes());
		data.extend_from_slice(&destination_bytes(&relayer_argon_account_id));
		data.extend_from_slice(&u64_word(gateway_activity_nonce));
		data.extend_from_slice(&u64_word(argon_approvals_nonce));
		data.extend_from_slice(&u128_word(argon_circulation));
		data.extend_from_slice(&u128_word(argonot_circulation));

		EthereumLog {
			address: h160(0x21),
			topics: vec![
				H256::from_slice(MintingAuthorityActivated::SIGNATURE_HASH.as_slice()),
				indexed_address_word(destination_signing_key),
			]
			.try_into()
			.expect("topics stay within Ethereum log topic bounds"),
			data: data
				.try_into()
				.expect("minting-authority activation data stays within bounded log payload"),
		}
	}

	fn minting_authority_deactivated_log(
		microgon_collateral: Balance,
		micronot_collateral: Balance,
		destination_signing_key: H160,
		relayer_argon_account_id: TestAccountId,
		approval_hash: H256,
		gateway_activity_nonce: GatewayActivityNonce,
		argon_approvals_nonce: ArgonApprovalsNonce,
	) -> EthereumLog {
		let (argon_circulation, argonot_circulation) =
			current_gateway_circulation_after(None, None);
		let mut data = Vec::with_capacity(224);
		data.extend_from_slice(&u128_word(microgon_collateral));
		data.extend_from_slice(&u128_word(micronot_collateral));
		data.extend_from_slice(approval_hash.as_bytes());
		data.extend_from_slice(&destination_bytes(&relayer_argon_account_id));
		data.extend_from_slice(&u64_word(gateway_activity_nonce));
		data.extend_from_slice(&u64_word(argon_approvals_nonce));
		data.extend_from_slice(&u128_word(argon_circulation));
		data.extend_from_slice(&u128_word(argonot_circulation));

		EthereumLog {
			address: h160(0x21),
			topics: vec![
				H256::from_slice(MintingAuthorityDeactivated::SIGNATURE_HASH.as_slice()),
				indexed_address_word(destination_signing_key),
			]
			.try_into()
			.expect("topics stay within Ethereum log topic bounds"),
			data: data
				.try_into()
				.expect("minting-authority deactivation data stays within bounded log payload"),
		}
	}

	fn transfer_out_of_argon_finalized_log(
		transfer_id: H256,
		minting_collateral: Vec<(H160, Balance, Balance)>,
		gateway_activity_nonce: GatewayActivityNonce,
		argon_approvals_nonce: ArgonApprovalsNonce,
	) -> EthereumLog {
		let finalized_transfer = TransferOutById::<Test>::get(transfer_id)
			.expect("finalized transfer should still be present when building event");
		let (argon_circulation, argonot_circulation) = current_gateway_circulation_after(
			None,
			Some((finalized_transfer.asset, finalized_transfer.amount)),
		);
		transfer_out_of_argon_finalized_log_with_circulation(
			transfer_id,
			finalized_transfer.asset,
			finalized_transfer.amount,
			minting_collateral,
			gateway_activity_nonce,
			argon_approvals_nonce,
			argon_circulation,
			argonot_circulation,
		)
	}

	#[allow(clippy::too_many_arguments)]
	fn transfer_out_of_argon_finalized_log_with_circulation(
		transfer_id: H256,
		asset: AssetKind,
		amount: Balance,
		minting_collateral: Vec<(H160, Balance, Balance)>,
		gateway_activity_nonce: GatewayActivityNonce,
		argon_approvals_nonce: ArgonApprovalsNonce,
		argon_circulation: Balance,
		argonot_circulation: Balance,
	) -> EthereumLog {
		let event = ContractTransferOutOfArgonFinalized {
			transferId: alloy_primitives::B256::from(transfer_id.0),
			token: AlloyAddress::from_slice(
				match asset {
					AssetKind::Argon => h160(0x31),
					AssetKind::Argonot => h160(0x32),
				}
				.as_bytes(),
			),
			amount,
			mintingCollateral: minting_collateral
				.into_iter()
				.map(|(signing_key, microgon_collateral, micronot_collateral)| {
					ContractMintingAuthorityCollateral {
						signingKey: AlloyAddress::from_slice(signing_key.as_bytes()),
						microgonCollateral: microgon_collateral,
						micronotCollateral: micronot_collateral,
					}
				})
				.collect(),
			gatewayState: contract_gateway_activity_state(
				gateway_activity_nonce,
				argon_approvals_nonce,
				argon_circulation,
				argonot_circulation,
			),
		};

		EthereumLog {
			address: h160(0x21),
			topics: vec![H256::from_slice(
				ContractTransferOutOfArgonFinalized::SIGNATURE_HASH.as_slice(),
			)]
			.try_into()
			.expect("topics stay within Ethereum log topic bounds"),
			data: event
				.encode_data()
				.try_into()
				.expect("transfer-out finalized data stays within bounded log payload"),
		}
	}

	fn transfer_out_of_argon_canceled_log(
		transfer_id: H256,
		gateway_activity_nonce: GatewayActivityNonce,
		argon_approvals_nonce: ArgonApprovalsNonce,
	) -> EthereumLog {
		let (argon_circulation, argonot_circulation) =
			current_gateway_circulation_after(None, None);
		let event = ContractTransferOutOfArgonCanceled {
			transferId: alloy_primitives::B256::from(transfer_id.0),
			gatewayState: contract_gateway_activity_state(
				gateway_activity_nonce,
				argon_approvals_nonce,
				argon_circulation,
				argonot_circulation,
			),
		};

		EthereumLog {
			address: h160(0x21),
			topics: vec![H256::from_slice(
				ContractTransferOutOfArgonCanceled::SIGNATURE_HASH.as_slice(),
			)]
			.try_into()
			.expect("topics stay within Ethereum log topic bounds"),
			data: event
				.encode_data()
				.try_into()
				.expect("transfer-out canceled data stays within bounded log payload"),
		}
	}

	#[allow(clippy::too_many_arguments)]
	fn transfer_to_argon_started_log(
		gateway: H160,
		from: H160,
		token: H160,
		amount: Balance,
		destination: [u8; 32],
		gateway_activity_nonce: GatewayActivityNonce,
		argon_approvals_nonce: ArgonApprovalsNonce,
		argon_circulation: Balance,
		argonot_circulation: Balance,
	) -> EthereumLog {
		let mut data = Vec::with_capacity(192);
		data.extend_from_slice(&u128_word(amount));
		data.extend_from_slice(&destination);
		data.extend_from_slice(&u64_word(gateway_activity_nonce));
		data.extend_from_slice(&u64_word(argon_approvals_nonce));
		data.extend_from_slice(&u128_word(argon_circulation));
		data.extend_from_slice(&u128_word(argonot_circulation));

		EthereumLog {
			address: gateway,
			topics: vec![
				H256::from_slice(TransferToArgonStarted::SIGNATURE_HASH.as_slice()),
				indexed_address_word(from),
				indexed_address_word(token),
			]
			.try_into()
			.expect("topics stay within Ethereum log topic bounds"),
			data: data
				.try_into()
				.expect("transfer-to-argon event data stays within bounded log payload"),
		}
	}

	fn current_gateway_circulation_after(
		decrease: Option<(AssetKind, Balance)>,
		increase: Option<(AssetKind, Balance)>,
	) -> (Balance, Balance) {
		let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
		let pending =
			PendingTransferOutCirculationByDestinationChain::<Test>::get(SourceChain::Ethereum);
		let mut argon_circulation =
			Balances::balance(&burn_account).saturating_sub(pending.argon_circulation);
		let mut argonot_circulation =
			Ownership::balance(&burn_account).saturating_sub(pending.argonot_circulation);

		if let Some((asset, amount)) = decrease {
			match asset {
				AssetKind::Argon => argon_circulation = argon_circulation.saturating_sub(amount),
				AssetKind::Argonot =>
					argonot_circulation = argonot_circulation.saturating_sub(amount),
			}
		}

		if let Some((asset, amount)) = increase {
			match asset {
				AssetKind::Argon => argon_circulation = argon_circulation.saturating_add(amount),
				AssetKind::Argonot =>
					argonot_circulation = argonot_circulation.saturating_add(amount),
			}
		}

		(argon_circulation, argonot_circulation)
	}

	fn contract_gateway_activity_state(
		gateway_activity_nonce: GatewayActivityNonce,
		argon_approvals_nonce: ArgonApprovalsNonce,
		argon_circulation: Balance,
		argonot_circulation: Balance,
	) -> ContractGatewayActivityState {
		ContractGatewayActivityState {
			gatewayActivityNonce: gateway_activity_nonce,
			argonApprovalsNonce: argon_approvals_nonce,
			argonCirculation: argon_circulation,
			argonotCirculation: argonot_circulation,
		}
	}

	fn activity_logs(
		logs: Vec<EthereumLog>,
	) -> BoundedVec<EthereumReceiptLog, MaxActivitiesPerReceiptProof> {
		activity_logs_for_transaction_index(logs, 0)
	}

	fn activity_logs_for_transaction_index(
		logs: Vec<EthereumLog>,
		transaction_index: u64,
	) -> BoundedVec<EthereumReceiptLog, MaxActivitiesPerReceiptProof> {
		logs.into_iter()
			.map(|event_log| EthereumReceiptLog { transaction_index, event_log })
			.collect::<Vec<_>>()
			.try_into()
			.expect("test gateway activity logs stay within pallet bound")
	}

	fn proof_batch(
		log_blocks: Vec<BoundedVec<EthereumReceiptLog, MaxActivitiesPerReceiptProof>>,
	) -> GatewayActivityProofBatch<Test> {
		GatewayActivityProofBatch::<Test> {
			execution_block_proof: dummy_execution_block_proof(),
			blocks: log_blocks
				.into_iter()
				.map(|receipt_logs| GatewayActivityProofBlock::<Test> {
					target_block_number: 0,
					receipt_proof: dummy_receipt_proof(),
					receipt_logs,
				})
				.collect::<Vec<_>>()
				.try_into()
				.expect("test gateway proof blocks stay within pallet bound"),
		}
	}

	fn dummy_execution_block_proof() -> EthereumExecutionBlockProof {
		EthereumExecutionBlockProof {
			anchor_block_hash: H256::repeat_byte(1),
			target_to_anchor_header_chain: Vec::new()
				.try_into()
				.expect("empty header chain stays within bounds"),
		}
	}

	fn dummy_receipt_proof() -> EthereumCombinedReceiptProof {
		EthereumCombinedReceiptProof {
			nodes: vec![vec![1u8].try_into().expect("tiny receipt proof node stays within bounds")]
				.try_into()
				.expect("single-node receipt proof stays within bounds"),
			receipts: vec![EthereumReceiptProofReceipt {
				transaction_index: 0,
				node_indexes: vec![0]
					.try_into()
					.expect("single node index stays within bounded receipt proof refs"),
			}]
			.try_into()
			.expect("single receipt reference stays within bounded receipt proof count"),
		}
	}
}
