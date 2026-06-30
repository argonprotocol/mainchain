use alloc::vec::Vec;
use alloy_primitives::{Address as AlloyAddress, B256};
use argon_ethereum_contracts::minting_gateway as ethereum_contracts;
use argon_primitives::{
	EthereumBlockNumber, EthereumVerifyProvider, TickProvider, MICROGONS_PER_ARGON,
};
use frame_support::{
	ensure,
	storage::{transactional::with_transaction_opaque_err, TransactionOutcome},
	traits::fungible::MutateHold,
};
use pallet_prelude::*;
use polkadot_sdk::sp_core::ecdsa::KeccakSignature;

use super::{
	gateway_activity::GatewayMintingAuthorityCollateral,
	ActiveGlobalIssuanceCouncilByDestinationChain, AssetKind, ChainConfig,
	ChainConfigBySourceChain, Config, CurrentTransferOutMicrogonsPerArgonotByDestinationChain,
	Error, Event, GatewaySyncPauseReason, GlobalIssuanceCouncilByHash, HoldReason,
	MintingAuthoritiesBySigner, MintingAuthorityState, NextTransferOutNonceBySendingAccountId,
	NonTerminalTransferOutCountByDestinationChain, Pallet, PendingCollateralizationRequestsByChain,
	PendingTransferOutCirculationByDestinationChain,
	PreviousTransferOutMicrogonsPerArgonotByDestinationChain, SourceChain, TransferOutById,
	TransferOutQuoteMicrogonsPerArgonotByDestinationChain, TransferOutRequestNonce, H160, H256,
};

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	CloneNoBound,
	PartialEq,
	Eq,
	DebugNoBound,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
/// One authority's current signed reservation row for an outbound transfer request.
pub struct MintingAuthorityTransferReservation<T: Config> {
	/// Microgon backing this authority currently commits to the request.
	pub microgon_collateral: T::Balance,
	/// Micronot backing this authority currently commits to the request.
	pub micronot_collateral: T::Balance,
	/// Normalized contribution used to decide when the request is fully covered and how the tip is
	/// split on finalization.
	pub collateral_share: T::Balance,
	/// Destination-chain signature over the authority's cumulative reservation row.
	pub signature: KeccakSignature,
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
/// Client-facing stage of an outbound transfer request.
pub enum TransferOutState {
	/// The request exists but still needs more authority collateral before it can be finalized on
	/// the destination chain.
	Started,
	/// The request has enough authority collateral and can be finalized on the destination chain.
	Ready,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	CloneNoBound,
	PartialEq,
	Eq,
	DebugNoBound,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
/// Canonical Argon-side record for one outbound transfer request.
pub struct TransferOutOfArgon<T: Config> {
	/// Sending Argon account that opened the transfer and supplied the principal and tip.
	pub argon_account_id: T::AccountId,
	/// Sender-scoped request id salt so two otherwise identical transfers from the same account
	/// stay distinct.
	#[codec(compact)]
	pub argon_transfer_nonce: TransferOutRequestNonce,
	/// Destination chain that will receive and settle the request.
	pub destination_chain: SourceChain,
	/// Exact transfer conversion rate snapshotted for this request, expressed as microgons per 1
	/// whole Argonot.
	#[codec(compact)]
	pub microgons_per_argonot: T::Balance,
	/// Recipient account on the destination chain.
	pub destination_account: H160,
	/// Last verified Ethereum execution block at which this request is still considered timely for
	/// the current gateway implementation.
	#[codec(compact)]
	pub valid_until_ethereum_block: EthereumBlockNumber,
	/// Asset being moved out of Argon.
	pub asset: AssetKind,
	/// Principal amount the user wants released on the destination chain.
	pub amount: T::Balance,
	/// Incentive held on Argon and later paid to the authorities that actually finalize the
	/// request.
	pub minting_authority_tip: T::Balance,
	/// Total normalized collateral currently attached across all authority rows.
	pub total_attached_collateral: T::Balance,
	/// Current authority reservation rows keyed by destination-chain signer.
	pub minting_authority_collateral_by_signer:
		BoundedBTreeMap<H160, MintingAuthorityTransferReservation<T>, T::MaxCouncilMembers>,
	/// Whether the transfer still needs coverage or is ready for gateway finalization.
	pub state: TransferOutState,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	CloneNoBound,
	PartialEq,
	Eq,
	DebugNoBound,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
/// One transfer that can still accept more collateral.
pub struct PendingCollateralizationRequest<T: Config> {
	/// Canonical transfer request id.
	pub transfer_id: H256,
	/// Remaining normalized coverage gap before the transfer can move to `Ready`.
	pub remaining_collateral: T::Balance,
	/// Portion of the minting authority tip that still tracks the uncovered share of the request.
	pub remaining_minting_authority_tip: T::Balance,
}

impl<T: Config> Pallet<T> {
	pub(crate) fn do_transfer_out(
		account_id: T::AccountId,
		destination_chain: SourceChain,
		asset: AssetKind,
		destination_account: H160,
		amount: T::Balance,
	) -> DispatchResult {
		ensure!(amount != T::Balance::default(), Error::<T>::InvalidTransferOutAmount);
		ensure!(destination_account != H160::zero(), Error::<T>::InvalidTransferOutRecipient);

		let microgons_per_argonot =
			Self::transfer_out_quote_microgons_per_argonot(destination_chain)?;
		let (chain_id, _) = Self::evm_gateway_signature_domain(destination_chain)?;
		let token = Self::destination_token(destination_chain, asset)?;
		let latest_execution_block_number = T::EthereumVerifier::latest_execution_block_number()
			.ok_or(Error::<T>::MissingVerifiedExecutionBlock)?;
		let latest_execution_block_timestamp =
			T::EthereumVerifier::latest_execution_block_timestamp()
				.ok_or(Error::<T>::MissingVerifiedExecutionBlock)?;
		let latest_execution_block_tick =
			T::TickProvider::ticker().tick_for_time(latest_execution_block_timestamp);
		ensure!(
			T::CurrentTick::get().saturating_sub(latest_execution_block_tick) <=
				T::MaxVerifiedExecutionBlockAgeTicks::get(),
			Error::<T>::StaleVerifiedExecutionBlock,
		);
		let valid_until_ethereum_block = latest_execution_block_number
			.saturating_add(T::TransferOutValidityEthereumBlocks::get());
		let minting_authority_tip: T::Balance = amount
			.into()
			.saturating_mul(T::TransferOutMintingAuthorityTipBasisPoints::get() as u128)
			.saturating_div(10_000)
			.into();
		ensure!(
			NonTerminalTransferOutCountByDestinationChain::<T>::get(destination_chain) <
				T::MaxPendingTransferOutsPerDestinationChain::get(),
			Error::<T>::TooManyPendingTransferOuts,
		);
		let argon_transfer_nonce =
			NextTransferOutNonceBySendingAccountId::<T>::mutate(&account_id, |next_nonce| {
				next_nonce.saturating_accrue(1);
				*next_nonce
			});
		let transfer_id = H256::from_slice(
			ethereum_contracts::hash_transfer_out_of_argon_request(
				Self::argon_account_id_bytes(&account_id),
				argon_transfer_nonce,
				chain_id,
				microgons_per_argonot.into(),
				AlloyAddress::from_slice(destination_account.as_bytes()),
				valid_until_ethereum_block,
				AlloyAddress::from_slice(token.as_bytes()),
				amount.into(),
				minting_authority_tip.into(),
			)
			.as_slice(),
		);

		Self::hold_transfer_out_minting_authority_tip(asset, &account_id, minting_authority_tip)?;
		match asset {
			AssetKind::Argon => Self::move_to_burn_account::<T::NativeCurrency>(
				destination_chain,
				&account_id,
				amount,
			)?,
			AssetKind::Argonot => Self::move_to_burn_account::<T::OwnershipCurrency>(
				destination_chain,
				&account_id,
				amount,
			)?,
		}
		Self::add_pending_transfer_out_circulation(destination_chain, asset, amount);

		let transfer = TransferOutOfArgon::<T> {
			argon_account_id: account_id.clone(),
			argon_transfer_nonce,
			destination_chain,
			microgons_per_argonot,
			destination_account,
			valid_until_ethereum_block,
			asset,
			amount,
			minting_authority_tip,
			total_attached_collateral: T::Balance::default(),
			minting_authority_collateral_by_signer: BoundedBTreeMap::new(),
			state: TransferOutState::Started,
		};
		NonTerminalTransferOutCountByDestinationChain::<T>::mutate(destination_chain, |count| {
			count.saturating_accrue(1);
		});
		TransferOutById::<T>::insert(transfer_id, transfer.clone());
		Self::sync_pending_collateralization_request(transfer_id, &transfer)?;
		Self::deposit_event(Event::TransferOutStarted {
			destination_chain,
			transfer_id,
			account_id,
			asset,
			amount,
			minting_authority_tip,
		});
		Ok(())
	}

	pub(crate) fn do_collateralize_transfer(
		account_id: T::AccountId,
		transfer_id: H256,
		signature: KeccakSignature,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
	) -> DispatchResult {
		let mut became_ready = false;
		let mut recovered_signer = None;
		let mut finalized_microgon_collateral = T::Balance::default();
		let mut finalized_micronot_collateral = T::Balance::default();

		TransferOutById::<T>::try_mutate_exists(transfer_id, |maybe_transfer| -> DispatchResult {
			let transfer = maybe_transfer.as_mut().ok_or(Error::<T>::TransferOutNotFound)?;
			ensure!(transfer.state != TransferOutState::Ready, Error::<T>::TransferOutAlreadyReady);
			let latest_execution_block_number =
				T::EthereumVerifier::latest_execution_block_number()
					.ok_or(Error::<T>::MissingVerifiedExecutionBlock)?;
			ensure!(
				latest_execution_block_number <= transfer.valid_until_ethereum_block,
				Error::<T>::TransferOutExpired
			);

			let (chain_id, gateway) =
				Self::evm_gateway_signature_domain(transfer.destination_chain)?;
			let authorization_hash = H256::from_slice(
				ethereum_contracts::hash_minting_authorization(
					chain_id,
					AlloyAddress::from_slice(gateway.as_bytes()),
					B256::from(Self::transfer_out_request_id(transfer)?.0),
					microgon_collateral.into(),
					micronot_collateral.into(),
				)
				.as_slice(),
			);
			let destination_signing_key =
				Self::recover_evm_message_signer(authorization_hash, &signature)
					.ok_or(Error::<T>::InvalidTransferCollateralSignature)?;
			recovered_signer = Some(destination_signing_key);

			ensure!(
				!transfer
					.minting_authority_collateral_by_signer
					.contains_key(&destination_signing_key),
				Error::<T>::InvalidTransferCollateralUpdate,
			);

			let collateral_share = Self::normalized_transfer_collateral(
				transfer.asset,
				transfer.microgons_per_argonot,
				microgon_collateral,
				micronot_collateral,
			)?;
			ensure!(
				collateral_share != T::Balance::default(),
				Error::<T>::InvalidTransferCollateralUpdate,
			);

			let next_total_attached_collateral =
				transfer.total_attached_collateral.saturating_add(collateral_share);
			let completes_transfer = next_total_attached_collateral >= transfer.amount;
			ensure!(
				collateral_share >= T::MinTransferCollateralIncrement::get() || completes_transfer,
				Error::<T>::TransferCollateralIncrementTooSmall,
			);

			MintingAuthoritiesBySigner::<T>::try_mutate_exists(
				destination_signing_key,
				|maybe_authority| -> DispatchResult {
					let authority =
						maybe_authority.as_mut().ok_or(Error::<T>::MintingAuthorityNotFound)?;
					ensure!(
						authority.account_id == account_id,
						Error::<T>::InvalidTransferCollateralSignature,
					);
					ensure!(
						authority.destination_chain == transfer.destination_chain,
						Error::<T>::MintingAuthorityMismatch,
					);
					ensure!(
						authority.state == MintingAuthorityState::Active,
						Error::<T>::UnexpectedMintingAuthorityState,
					);

					// if we're funding argons, we need to use up argon collateral before argonots
					// since argons can't fund argonots
					if transfer.asset == AssetKind::Argon &&
						micronot_collateral != T::Balance::default()
					{
						let remaining_collateral =
							transfer.amount.saturating_sub(transfer.total_attached_collateral);
						let available_microgons = authority
							.gateway_remaining_microgon_collateral
							.saturating_sub(authority.pending_reserved_microgon_collateral);
						ensure!(
							available_microgons < remaining_collateral &&
								microgon_collateral == available_microgons,
							Error::<T>::InvalidTransferCollateral,
						);
					}
					ensure!(
						authority
							.pending_reserved_microgon_collateral
							.saturating_add(microgon_collateral) <=
							authority.gateway_remaining_microgon_collateral,
						Error::<T>::InsufficientMintingAuthorityCollateral,
					);
					ensure!(
						authority
							.pending_reserved_micronot_collateral
							.saturating_add(micronot_collateral) <=
							authority.gateway_remaining_micronot_collateral,
						Error::<T>::InsufficientMintingAuthorityCollateral,
					);

					authority
						.pending_reserved_microgon_collateral
						.saturating_accrue(microgon_collateral);
					authority
						.pending_reserved_micronot_collateral
						.saturating_accrue(micronot_collateral);
					authority
						.active_pending_transfer_ids
						.try_push(transfer_id)
						.map_err(|_| Error::<T>::TooManyPendingTransferOuts)?;
					Ok(())
				},
			)?;

			let reservation = MintingAuthorityTransferReservation::<T> {
				microgon_collateral,
				micronot_collateral,
				collateral_share,
				signature,
			};
			let _ = transfer
				.minting_authority_collateral_by_signer
				.try_insert(destination_signing_key, reservation)
				.map_err(|_| Error::<T>::InvalidTransferCollateral)?;
			transfer.total_attached_collateral = next_total_attached_collateral;
			transfer.state = if completes_transfer {
				TransferOutState::Ready
			} else {
				TransferOutState::Started
			};
			became_ready = transfer.state == TransferOutState::Ready;
			finalized_microgon_collateral = microgon_collateral;
			finalized_micronot_collateral = micronot_collateral;
			Ok(())
		})?;

		let transfer =
			TransferOutById::<T>::get(transfer_id).ok_or(Error::<T>::TransferOutNotFound)?;
		Self::sync_pending_collateralization_request(transfer_id, &transfer)?;

		let destination_signing_key =
			recovered_signer.ok_or(Error::<T>::InvalidTransferCollateralSignature)?;
		Self::deposit_event(Event::TransferCollateralized {
			transfer_id,
			destination_signing_key,
			microgon_collateral: finalized_microgon_collateral,
			micronot_collateral: finalized_micronot_collateral,
		});
		if became_ready {
			Self::deposit_event(Event::TransferOutReady { transfer_id });
		}
		Ok(())
	}

	pub(crate) fn finalize_transfer_out_from_gateway(
		source_chain: SourceChain,
		transfer_id: H256,
		asset: AssetKind,
		amount: T::Balance,
		finalized_collateral_rows: Vec<GatewayMintingAuthorityCollateral<T>>,
	) -> Result<(), GatewaySyncPauseReason> {
		let local_transfer = TransferOutById::<T>::take(transfer_id);
		let has_local_transfer = local_transfer.is_some();
		let mut finalized_authority_collateral =
			Vec::with_capacity(finalized_collateral_rows.len());
		if let Some(transfer) = local_transfer.as_ref() {
			Self::remove_pending_collateralization_request(transfer.destination_chain, transfer_id);
			NonTerminalTransferOutCountByDestinationChain::<T>::mutate(
				transfer.destination_chain,
				|count| count.saturating_reduce(1),
			);
			Self::remove_pending_transfer_out_circulation(
				transfer.destination_chain,
				transfer.asset,
				transfer.amount,
			);
			Self::release_transfer_reservations(transfer_id, transfer)
				.map_err(|_| GatewaySyncPauseReason::MintingAuthorityMismatch)?;
		}

		for collateral_row in &finalized_collateral_rows {
			MintingAuthoritiesBySigner::<T>::try_mutate_exists(
				collateral_row.signing_key,
				|maybe_authority| -> Result<(), GatewaySyncPauseReason> {
					let authority = maybe_authority
						.as_mut()
						.ok_or(GatewaySyncPauseReason::MintingAuthorityNotFound)?;
					if authority.destination_chain != source_chain {
						return Err(GatewaySyncPauseReason::MintingAuthorityMismatch);
					}
					finalized_authority_collateral.push((
						authority.account_id.clone(),
						collateral_row.microgon_collateral,
						collateral_row.micronot_collateral,
					));
					authority.gateway_remaining_microgon_collateral = authority
						.gateway_remaining_microgon_collateral
						.checked_sub(&collateral_row.microgon_collateral)
						.ok_or(GatewaySyncPauseReason::MintingAuthorityMismatch)?;
					authority.gateway_remaining_micronot_collateral = authority
						.gateway_remaining_micronot_collateral
						.checked_sub(&collateral_row.micronot_collateral)
						.ok_or(GatewaySyncPauseReason::MintingAuthorityMismatch)?;
					Ok(())
				},
			)?;
		}

		for (account_id, microgon_collateral, micronot_collateral) in finalized_authority_collateral
		{
			if has_local_transfer {
				// When the local transfer exists, the user's source-side principal already stays
				// parked in the burn account as the new gateway backing. Only the authority's
				// temporary encumbrance should be released.
				Self::release_minting_authority_collateral(
					account_id.clone(),
					microgon_collateral,
					micronot_collateral,
				)
				.map_err(|_| GatewaySyncPauseReason::GatewayStateDrift)?;
			} else {
				// Without a matching local transfer, the gateway's remaining collateral is
				// canonical. A proven finalization must be treated as out-of-system collateral
				// consumption and burned locally.
				Self::burn_minting_authority_collateral(
					account_id.clone(),
					microgon_collateral,
					micronot_collateral,
				)
				.map_err(|_| GatewaySyncPauseReason::GatewayStateDrift)?;
			}
		}

		if !has_local_transfer {
			let burn_account = Self::burn_account(source_chain);
			match asset {
				AssetKind::Argon => T::NativeCurrency::mint_into(&burn_account, amount),
				AssetKind::Argonot => T::OwnershipCurrency::mint_into(&burn_account, amount),
			}
			.map_err(|_| GatewaySyncPauseReason::GatewayStateDrift)?;
		}

		for collateral_row in &finalized_collateral_rows {
			Self::invalidate_excess_transfer_reservations(collateral_row.signing_key)?;
			// Proven finalization should still land even if opportunistic local deactivation
			// queueing cannot be scheduled right now.
			let _ = Self::maybe_auto_deactivate_minting_authority(collateral_row.signing_key);
		}

		if let Some(transfer) = local_transfer.as_ref() {
			Self::pay_transfer_out_minting_authority_tip(transfer, &finalized_collateral_rows)?;
			Self::record_transfer_out(&transfer.argon_account_id, asset, amount);
		}
		Self::deposit_event(Event::TransferOutFinalized { source_chain, transfer_id });
		Ok(())
	}

	pub(crate) fn cancel_transfer_out_from_gateway(
		source_chain: SourceChain,
		transfer_id: H256,
	) -> Result<(), GatewaySyncPauseReason> {
		let Some(transfer) = TransferOutById::<T>::take(transfer_id) else {
			Self::deposit_event(Event::TransferOutCanceled { source_chain, transfer_id });
			return Ok(());
		};
		Self::remove_pending_collateralization_request(transfer.destination_chain, transfer_id);
		NonTerminalTransferOutCountByDestinationChain::<T>::mutate(
			transfer.destination_chain,
			|count| count.saturating_reduce(1),
		);
		Self::remove_pending_transfer_out_circulation(
			transfer.destination_chain,
			transfer.asset,
			transfer.amount,
		);
		Self::release_transfer_reservations(transfer_id, &transfer)
			.map_err(|_| GatewaySyncPauseReason::MintingAuthorityMismatch)?;
		match transfer.asset {
			AssetKind::Argon => Self::mint_to::<T::NativeCurrency>(
				transfer.destination_chain,
				transfer.amount,
				&transfer.argon_account_id,
			),
			AssetKind::Argonot => Self::mint_to::<T::OwnershipCurrency>(
				transfer.destination_chain,
				transfer.amount,
				&transfer.argon_account_id,
			),
		}
		.map_err(|_| GatewaySyncPauseReason::GatewayStateDrift)?;
		Self::release_transfer_out_minting_authority_tip(
			transfer.asset,
			&transfer.argon_account_id,
			transfer.minting_authority_tip,
		)
		.map_err(|_| GatewaySyncPauseReason::GatewayStateDrift)?;
		Self::deposit_event(Event::TransferOutCanceled { source_chain, transfer_id });
		Ok(())
	}

	pub(crate) fn expire_transfer_outs_through_block(
		source_chain: SourceChain,
		synced_ethereum_block: EthereumBlockNumber,
	) -> Result<(), GatewaySyncPauseReason> {
		let expired_transfer_ids = TransferOutById::<T>::iter()
			.filter_map(|(transfer_id, transfer)| {
				(transfer.destination_chain == source_chain &&
					transfer.valid_until_ethereum_block < synced_ethereum_block)
					.then_some(transfer_id)
			})
			.collect::<Vec<_>>();

		for transfer_id in expired_transfer_ids {
			with_transaction_opaque_err(|| {
				match Self::cancel_transfer_out_from_gateway(source_chain, transfer_id) {
					Ok(()) => TransactionOutcome::Commit(Ok(())),
					Err(reason) => TransactionOutcome::Rollback(Err(reason)),
				}
			})
			.map_err(|_| GatewaySyncPauseReason::GatewayStateDrift)??;
		}

		Ok(())
	}

	fn sync_pending_collateralization_request(
		transfer_id: H256,
		transfer: &TransferOutOfArgon<T>,
	) -> DispatchResult {
		let remaining_collateral =
			transfer.amount.saturating_sub(transfer.total_attached_collateral);
		if remaining_collateral == T::Balance::default() {
			Self::remove_pending_collateralization_request(transfer.destination_chain, transfer_id);
			return Ok(());
		}

		let minting_authority_tip_u128: u128 = transfer.minting_authority_tip.into();
		let remaining_minting_authority_tip = minting_authority_tip_u128
			.saturating_mul(remaining_collateral.into())
			.saturating_div(transfer.amount.into())
			.into();

		PendingCollateralizationRequestsByChain::<T>::try_mutate(
			transfer.destination_chain,
			|requests| -> DispatchResult {
				let request = PendingCollateralizationRequest::<T> {
					transfer_id,
					remaining_collateral,
					remaining_minting_authority_tip,
				};
				if let Some(existing) =
					requests.iter_mut().find(|existing| existing.transfer_id == transfer_id)
				{
					*existing = request;
					return Ok(());
				}

				requests.try_push(request).map_err(|_| Error::<T>::TooManyPendingTransferOuts)?;
				Ok(())
			},
		)
	}

	fn remove_pending_collateralization_request(destination_chain: SourceChain, transfer_id: H256) {
		PendingCollateralizationRequestsByChain::<T>::mutate(destination_chain, |requests| {
			if let Some(index) =
				requests.iter().position(|request| request.transfer_id == transfer_id)
			{
				requests.remove(index);
			}
		});
	}

	fn release_transfer_reservations(
		transfer_id: H256,
		transfer: &TransferOutOfArgon<T>,
	) -> DispatchResult {
		for (destination_signing_key, reservation) in
			transfer.minting_authority_collateral_by_signer.iter()
		{
			MintingAuthoritiesBySigner::<T>::try_mutate_exists(
				destination_signing_key,
				|maybe_authority| -> DispatchResult {
					let authority =
						maybe_authority.as_mut().ok_or(Error::<T>::MintingAuthorityNotFound)?;
					authority
						.pending_reserved_microgon_collateral
						.saturating_reduce(reservation.microgon_collateral);
					authority
						.pending_reserved_micronot_collateral
						.saturating_reduce(reservation.micronot_collateral);
					let Some(transfer_index) = authority
						.active_pending_transfer_ids
						.iter()
						.position(|active_transfer_id| *active_transfer_id == transfer_id)
					else {
						return Err(Error::<T>::MintingAuthorityMismatch.into());
					};
					authority.active_pending_transfer_ids.remove(transfer_index);
					Ok(())
				},
			)?;
		}
		Ok(())
	}

	fn invalidate_excess_transfer_reservations(
		destination_signing_key: H160,
	) -> Result<(), GatewaySyncPauseReason> {
		loop {
			let Some(authority) = MintingAuthoritiesBySigner::<T>::get(destination_signing_key)
			else {
				return Err(GatewaySyncPauseReason::MintingAuthorityNotFound);
			};
			let microgon_over_reserved = authority.pending_reserved_microgon_collateral >
				authority.gateway_remaining_microgon_collateral;
			let micronot_over_reserved = authority.pending_reserved_micronot_collateral >
				authority.gateway_remaining_micronot_collateral;
			if !microgon_over_reserved && !micronot_over_reserved {
				return Ok(());
			}

			let Some(transfer_id) = authority.active_pending_transfer_ids.last().copied() else {
				return Err(GatewaySyncPauseReason::MintingAuthorityMismatch);
			};
			Self::invalidate_transfer_reservation(destination_signing_key, transfer_id)?;
		}
	}

	pub(crate) fn invalidate_all_transfer_reservations(
		destination_signing_key: H160,
	) -> Result<(), GatewaySyncPauseReason> {
		loop {
			let Some(authority) = MintingAuthoritiesBySigner::<T>::get(destination_signing_key)
			else {
				return Err(GatewaySyncPauseReason::MintingAuthorityNotFound);
			};

			let Some(transfer_id) = authority.active_pending_transfer_ids.last().copied() else {
				if authority.pending_reserved_microgon_collateral == T::Balance::default() &&
					authority.pending_reserved_micronot_collateral == T::Balance::default()
				{
					return Ok(());
				}
				return Err(GatewaySyncPauseReason::MintingAuthorityMismatch);
			};

			Self::invalidate_transfer_reservation(destination_signing_key, transfer_id)?;
		}
	}

	fn invalidate_transfer_reservation(
		destination_signing_key: H160,
		transfer_id: H256,
	) -> Result<(), GatewaySyncPauseReason> {
		let removed_row = TransferOutById::<T>::try_mutate_exists(
			transfer_id,
			|maybe_transfer| -> Result<MintingAuthorityTransferReservation<T>, GatewaySyncPauseReason> {
				let transfer = maybe_transfer
					.as_mut()
					.ok_or(GatewaySyncPauseReason::MintingAuthorityMismatch)?;
					let removed_row = transfer
						.minting_authority_collateral_by_signer
						.remove(&destination_signing_key)
						.ok_or(GatewaySyncPauseReason::MintingAuthorityMismatch)?;
					transfer
						.total_attached_collateral
						.saturating_reduce(removed_row.collateral_share);
					transfer.state = if transfer.total_attached_collateral >= transfer.amount {
						TransferOutState::Ready
					} else {
						TransferOutState::Started
					};
				Ok(removed_row)
			},
		)?;

		MintingAuthoritiesBySigner::<T>::try_mutate_exists(
			destination_signing_key,
			|maybe_authority| -> Result<(), GatewaySyncPauseReason> {
				let authority = maybe_authority
					.as_mut()
					.ok_or(GatewaySyncPauseReason::MintingAuthorityNotFound)?;
				authority
					.pending_reserved_microgon_collateral
					.saturating_reduce(removed_row.microgon_collateral);
				authority
					.pending_reserved_micronot_collateral
					.saturating_reduce(removed_row.micronot_collateral);
				let Some(transfer_index) = authority
					.active_pending_transfer_ids
					.iter()
					.position(|active_transfer_id| *active_transfer_id == transfer_id)
				else {
					return Err(GatewaySyncPauseReason::MintingAuthorityMismatch);
				};
				authority.active_pending_transfer_ids.remove(transfer_index);
				Ok(())
			},
		)?;

		if let Some(transfer) = TransferOutById::<T>::get(transfer_id) {
			Self::sync_pending_collateralization_request(transfer_id, &transfer)
				.map_err(|_| GatewaySyncPauseReason::GatewayStateDrift)?;
		}
		Self::deposit_event(Event::TransferCollateralInvalidated {
			transfer_id,
			destination_signing_key,
		});
		Ok(())
	}

	fn pay_transfer_out_minting_authority_tip(
		transfer: &TransferOutOfArgon<T>,
		finalized_collateral_rows: &[GatewayMintingAuthorityCollateral<T>],
	) -> Result<(), GatewaySyncPauseReason> {
		if transfer.minting_authority_tip == T::Balance::default() {
			return Ok(());
		}

		let mut payouts = finalized_collateral_rows
			.iter()
			.map(|collateral_row| {
				let collateral_share = Self::normalized_transfer_collateral(
					transfer.asset,
					transfer.microgons_per_argonot,
					collateral_row.microgon_collateral,
					collateral_row.micronot_collateral,
				)
				.map_err(|_| GatewaySyncPauseReason::MintingAuthorityMismatch)?;
				Ok((collateral_row.signing_key, collateral_share, T::Balance::default()))
			})
			.collect::<Result<Vec<_>, GatewaySyncPauseReason>>()?;
		let total_finalized_collateral_share = payouts
			.iter()
			.fold(T::Balance::default(), |total, payout| total.saturating_add(payout.1));
		if total_finalized_collateral_share == T::Balance::default() {
			return Err(GatewaySyncPauseReason::MintingAuthorityMismatch);
		}

		let minting_authority_tip_u128: u128 = transfer.minting_authority_tip.into();
		let mut remaining_tip = transfer.minting_authority_tip;

		for payout in &mut payouts {
			let tip = minting_authority_tip_u128
				.saturating_mul(payout.1.into())
				.saturating_div(total_finalized_collateral_share.into())
				.into();
			payout.2 = tip;
			remaining_tip.saturating_reduce(tip);
		}

		payouts.sort_by(|left, right| {
			right.1.cmp(&left.1).then_with(|| left.0.as_bytes().cmp(right.0.as_bytes()))
		});

		let mut payout_index = 0usize;
		while remaining_tip != T::Balance::default() {
			if payouts.is_empty() {
				return Err(GatewaySyncPauseReason::GatewayStateDrift);
			}
			payouts[payout_index].2.saturating_accrue(1u128.into());
			remaining_tip.saturating_reduce(1u128.into());
			payout_index = (payout_index + 1) % payouts.len();
		}

		for (destination_signing_key, _, payout) in payouts {
			if payout == T::Balance::default() {
				continue;
			}
			let authority = MintingAuthoritiesBySigner::<T>::get(destination_signing_key)
				.ok_or(GatewaySyncPauseReason::MintingAuthorityNotFound)?;
			Self::transfer_held_minting_authority_tip(
				transfer.asset,
				&transfer.argon_account_id,
				&authority.account_id,
				payout,
			)
			.map_err(|_| GatewaySyncPauseReason::GatewayStateDrift)?;
		}

		Ok(())
	}

	pub(crate) fn transfer_out_request_id(
		transfer: &TransferOutOfArgon<T>,
	) -> Result<H256, DispatchError> {
		let (chain_id, _) = Self::evm_gateway_signature_domain(transfer.destination_chain)?;
		let token = Self::destination_token(transfer.destination_chain, transfer.asset)?;
		Ok(H256::from_slice(
			ethereum_contracts::hash_transfer_out_of_argon_request(
				Self::argon_account_id_bytes(&transfer.argon_account_id),
				transfer.argon_transfer_nonce,
				chain_id,
				transfer.microgons_per_argonot.into(),
				AlloyAddress::from_slice(transfer.destination_account.as_bytes()),
				transfer.valid_until_ethereum_block,
				AlloyAddress::from_slice(token.as_bytes()),
				transfer.amount.into(),
				transfer.minting_authority_tip.into(),
			)
			.as_slice(),
		))
	}

	pub(crate) fn transfer_out_quote_microgons_per_argonot(
		destination_chain: SourceChain,
	) -> Result<T::Balance, DispatchError> {
		TransferOutQuoteMicrogonsPerArgonotByDestinationChain::<T>::get(destination_chain)
			.filter(|rate| *rate != T::Balance::default())
			.map(Ok)
			.unwrap_or_else(|| {
				Self::base_transfer_out_quote_microgons_per_argonot(destination_chain)
			})
	}

	pub(crate) fn base_transfer_out_quote_microgons_per_argonot(
		destination_chain: SourceChain,
	) -> Result<T::Balance, DispatchError> {
		let mut quoted_microgons_per_argonot =
			CurrentTransferOutMicrogonsPerArgonotByDestinationChain::<T>::get(destination_chain)
				.filter(|rate| *rate != T::Balance::default())
				.or_else(|| {
					ActiveGlobalIssuanceCouncilByDestinationChain::<T>::get(destination_chain)
						.and_then(GlobalIssuanceCouncilByHash::<T>::get)
						.map(|council| council.epoch_microgons_per_argonot)
						.filter(|rate| *rate != T::Balance::default())
				})
				.ok_or(Error::<T>::InvalidMicrogonsPerArgonot)?;
		if let Some(previous_transfer_rate) =
			PreviousTransferOutMicrogonsPerArgonotByDestinationChain::<T>::get(destination_chain)
				.filter(|rate| *rate != T::Balance::default())
		{
			quoted_microgons_per_argonot = quoted_microgons_per_argonot.min(previous_transfer_rate);
		}
		Ok(quoted_microgons_per_argonot)
	}

	fn add_pending_transfer_out_circulation(
		source_chain: SourceChain,
		asset: AssetKind,
		amount: T::Balance,
	) {
		PendingTransferOutCirculationByDestinationChain::<T>::mutate(source_chain, |pending| {
			match asset {
				AssetKind::Argon => pending.argon_circulation.saturating_accrue(amount),
				AssetKind::Argonot => pending.argonot_circulation.saturating_accrue(amount),
			}
		});
	}

	fn remove_pending_transfer_out_circulation(
		source_chain: SourceChain,
		asset: AssetKind,
		amount: T::Balance,
	) {
		PendingTransferOutCirculationByDestinationChain::<T>::mutate(source_chain, |pending| {
			match asset {
				AssetKind::Argon => pending.argon_circulation.saturating_reduce(amount),
				AssetKind::Argonot => pending.argonot_circulation.saturating_reduce(amount),
			}
		});
	}

	fn normalized_transfer_collateral(
		asset: AssetKind,
		epoch_microgons_per_argonot: T::Balance,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
	) -> Result<T::Balance, DispatchError> {
		if asset == AssetKind::Argonot && microgon_collateral != T::Balance::default() {
			return Err(Error::<T>::InvalidTransferCollateral.into());
		}

		let collateral_share = match asset {
			AssetKind::Argon => {
				let epoch_microgons_per_argonot: u128 = epoch_microgons_per_argonot.into();
				microgon_collateral.saturating_add(
					micronot_collateral
						.into()
						.saturating_mul(epoch_microgons_per_argonot)
						.saturating_div(MICROGONS_PER_ARGON)
						.into(),
				)
			},
			AssetKind::Argonot => micronot_collateral,
		};

		Ok(collateral_share)
	}

	fn destination_token(
		destination_chain: SourceChain,
		asset: AssetKind,
	) -> Result<H160, DispatchError> {
		let config = ChainConfigBySourceChain::<T>::get(destination_chain)
			.ok_or(Error::<T>::UnsupportedSource)?;
		match config {
			ChainConfig::Evm { argon_token, argonot_token, .. } => Ok(match asset {
				AssetKind::Argon => argon_token,
				AssetKind::Argonot => argonot_token,
			}),
		}
	}

	fn argon_account_id_bytes(account_id: &T::AccountId) -> [u8; 32] {
		*AsRef::<[u8; 32]>::as_ref(account_id)
	}

	fn move_to_burn_account<C: Mutate<T::AccountId, Balance = T::Balance> + 'static>(
		source_chain: SourceChain,
		from: &T::AccountId,
		amount: T::Balance,
	) -> DispatchResult {
		if amount == T::Balance::default() {
			return Ok(());
		}

		let _ =
			C::burn_from(from, amount, Preservation::Preserve, Precision::Exact, Fortitude::Force)?;
		let _ = C::mint_into(&Self::burn_account(source_chain), amount)?;
		Ok(())
	}

	fn hold_transfer_out_minting_authority_tip(
		asset: AssetKind,
		account_id: &T::AccountId,
		minting_authority_tip: T::Balance,
	) -> DispatchResult {
		match asset {
			AssetKind::Argon =>
				Self::hold_with_reason::<T::NativeCurrency>(account_id, minting_authority_tip),
			AssetKind::Argonot =>
				Self::hold_with_reason::<T::OwnershipCurrency>(account_id, minting_authority_tip),
		}
	}

	fn release_transfer_out_minting_authority_tip(
		asset: AssetKind,
		account_id: &T::AccountId,
		minting_authority_tip: T::Balance,
	) -> DispatchResult {
		match asset {
			AssetKind::Argon => Self::release_hold_with_reason::<T::NativeCurrency>(
				account_id,
				minting_authority_tip,
			),
			AssetKind::Argonot => Self::release_hold_with_reason::<T::OwnershipCurrency>(
				account_id,
				minting_authority_tip,
			),
		}
	}

	fn transfer_held_minting_authority_tip(
		asset: AssetKind,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: T::Balance,
	) -> DispatchResult {
		match asset {
			AssetKind::Argon =>
				Self::transfer_hold_with_reason::<T::NativeCurrency>(from, to, amount),
			AssetKind::Argonot =>
				Self::transfer_hold_with_reason::<T::OwnershipCurrency>(from, to, amount),
		}
	}

	fn hold_with_reason<C>(account_id: &T::AccountId, amount: T::Balance) -> DispatchResult
	where
		C: MutateHold<T::AccountId, Reason = T::RuntimeHoldReason, Balance = T::Balance>,
	{
		if amount == T::Balance::default() {
			return Ok(());
		}
		if C::balance_on_hold(&HoldReason::TransferOutMintingAuthorityTip.into(), account_id) ==
			T::Balance::default()
		{
			frame_system::Pallet::<T>::inc_providers(account_id);
		}
		C::hold(&HoldReason::TransferOutMintingAuthorityTip.into(), account_id, amount)?;
		Ok(())
	}

	fn release_hold_with_reason<C>(account_id: &T::AccountId, amount: T::Balance) -> DispatchResult
	where
		C: MutateHold<T::AccountId, Reason = T::RuntimeHoldReason, Balance = T::Balance>,
	{
		if amount == T::Balance::default() {
			return Ok(());
		}
		let _ = C::release(
			&HoldReason::TransferOutMintingAuthorityTip.into(),
			account_id,
			amount,
			Precision::Exact,
		)?;
		if C::balance_on_hold(&HoldReason::TransferOutMintingAuthorityTip.into(), account_id) ==
			T::Balance::default()
		{
			let _ = frame_system::Pallet::<T>::dec_providers(account_id);
		}
		Ok(())
	}

	fn transfer_hold_with_reason<C>(
		from: &T::AccountId,
		to: &T::AccountId,
		amount: T::Balance,
	) -> DispatchResult
	where
		C: MutateHold<T::AccountId, Reason = T::RuntimeHoldReason, Balance = T::Balance>,
	{
		if amount == T::Balance::default() {
			return Ok(());
		}
		C::transfer_on_hold(
			&HoldReason::TransferOutMintingAuthorityTip.into(),
			from,
			to,
			amount,
			Precision::Exact,
			Restriction::Free,
			Fortitude::Force,
		)?;
		if C::balance_on_hold(&HoldReason::TransferOutMintingAuthorityTip.into(), from) ==
			T::Balance::default()
		{
			let _ = frame_system::Pallet::<T>::dec_providers(from);
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::{gateway_activity::GatewayMintingAuthorityCollateral, tests::*};

	#[test]
	fn transfer_out_moves_principal_to_burn_and_holds_tip() {
		new_test_ext().execute_with(|| {
			let council_account = account(60);
			let council_pair = council_signing_pair(80);
			let user = account(61);

			configure_single_member_ethereum_council(council_account, 40, 20_000, &council_pair);
			assert_ok!(Balances::mint_into(&user, 25_000));

			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x44),
				20_000,
			));

			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			let transfer_id = transfer_out_id(&user, 1);
			assert_eq!(Balances::balance(&burn_account), 20_000);
			assert_eq!(Balances::balance(&user), 4_980);
			assert_eq!(
				Balances::balance_on_hold(
					&RuntimeHoldReason::CrosschainTransfer(
						HoldReason::TransferOutMintingAuthorityTip,
					),
					&user,
				),
				20,
			);
			let transfer =
				TransferOutById::<Test>::get(transfer_id).expect("transfer should be stored");
			assert_eq!(
				transfer.microgons_per_argonot,
				CrosschainTransfer::transfer_out_quote_microgons_per_argonot(
					SourceChain::Ethereum,
				)
				.expect("transfer quote rate should be available"),
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
				TransferTotalsByAccount::<Test>::get(&user),
				AccountTransferTotals::default(),
			);
		});
	}

	#[test]
	fn transfer_out_rejects_when_non_terminal_request_cap_is_reached() {
		new_test_ext().execute_with(|| {
			let council_account = account(160);
			let council_pair = council_signing_pair(180);
			let user = account(161);

			configure_single_member_ethereum_council(council_account, 140, 20_000, &council_pair);
			assert_ok!(Balances::mint_into(&user, 25_000));
			NonTerminalTransferOutCountByDestinationChain::<Test>::insert(
				SourceChain::Ethereum,
				100,
			);

			assert_noop!(
				CrosschainTransfer::transfer_out(
					RuntimeOrigin::signed(user),
					SourceChain::Ethereum,
					AssetKind::Argon,
					h160(0x54),
					20_000,
				),
				Error::<Test>::TooManyPendingTransferOuts,
			);
		});
	}

	#[test]
	fn transfer_out_rejects_stale_verified_execution_anchor() {
		new_test_ext().execute_with(|| {
			let council_account = account(162);
			let council_pair = council_signing_pair(181);
			let user = account(163);

			configure_single_member_ethereum_council(council_account, 141, 20_000, &council_pair);
			assert_ok!(Balances::mint_into(&user, 25_000));
			CurrentTick::set(61);
			LatestExecutionBlockTimestamp::set(Some(0));

			assert_noop!(
				CrosschainTransfer::transfer_out(
					RuntimeOrigin::signed(user),
					SourceChain::Ethereum,
					AssetKind::Argon,
					h160(0x55),
					20_000,
				),
				Error::<Test>::StaleVerifiedExecutionBlock,
			);
		});
	}

	#[test]
	fn collateralize_transfer_marks_ready_tracks_pending_reservations_and_rejects_updates() {
		new_test_ext().execute_with(|| {
			let authority_account = account(62);
			let authority_pair = council_signing_pair(81);
			let signing_key = activate_test_minting_authority(
				authority_account.clone(),
				41,
				20_000,
				&council_signing_pair(82),
				&authority_pair,
				30_000,
				0,
			);
			let user = account(63);

			assert_ok!(Balances::mint_into(&user, 25_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x45),
				20_000,
			));
			let transfer_id = transfer_out_id(&user, 1);
			let signature = transfer_collateral_signature(&authority_pair, transfer_id, 20_000, 0);

			let result = CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account.clone()),
				transfer_id,
				signature,
				20_000,
				0,
			);
			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));

			let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
				.expect("authority should stay registered");
			let transfer =
				TransferOutById::<Test>::get(transfer_id).expect("transfer should remain stored");
			assert_eq!(authority.pending_reserved_microgon_collateral, 20_000);
			assert_eq!(authority.active_pending_transfer_ids, vec![transfer_id]);
			assert_eq!(transfer.state, TransferOutState::Ready);
			assert_eq!(transfer.total_attached_collateral, 20_000);
			assert_eq!(
				transfer
					.minting_authority_collateral_by_signer
					.get(&signing_key)
					.expect("authority row should be stored")
					.microgon_collateral,
				20_000,
			);
			assert!(PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum)
				.is_empty());
			assert_eq!(
				NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
				1,
			);
			assert_noop!(
				CrosschainTransfer::collateralize_transfer(
					RuntimeOrigin::signed(authority_account),
					transfer_id,
					transfer_collateral_signature(&authority_pair, transfer_id, 25_000, 0),
					25_000,
					0,
				),
				Error::<Test>::TransferOutAlreadyReady,
			);
		});
	}

	#[test]
	fn force_set_global_issuance_council_keeps_existing_transfer_collateralization() {
		new_test_ext().execute_with(|| {
			let authority_account = account(164);
			let council_pair = council_signing_pair(183);
			let authority_pair = council_signing_pair(184);
			let signing_key = activate_test_minting_authority(
				authority_account.clone(),
				47,
				20_000,
				&council_pair,
				&authority_pair,
				40_000,
				0,
			);
			let user = account(165);

			assert_ok!(Balances::mint_into(&user, 25_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x56),
				20_000,
			));
			let first_transfer_id = transfer_out_id(&user, 1);
			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account.clone()),
				first_transfer_id,
				transfer_collateral_signature(&authority_pair, first_transfer_id, 20_000, 0),
				20_000,
				0,
			));

			LowestMicrogonsPerArgonot::set(Some(2 * argon_primitives::MICROGONS_PER_ARGON));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum)
					.map_or(0, |state| state.argon_approvals_nonce),
				vec![authority_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));

			let second_user = account(167);
			assert_ok!(Balances::mint_into(&second_user, 25_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(second_user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x57),
				20_000,
			));
			let second_transfer_id = transfer_out_id(&second_user, 1);

			LowestMicrogonsPerArgonot::set(Some(3 * argon_primitives::MICROGONS_PER_ARGON));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				GatewayStateBySourceChain::<Test>::get(SourceChain::Ethereum)
					.map_or(0, |state| state.argon_approvals_nonce),
				vec![authority_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));

			let first_transfer = TransferOutById::<Test>::get(first_transfer_id)
				.expect("orphaned transfer should stay stored until expiry");
			let second_transfer = TransferOutById::<Test>::get(second_transfer_id)
				.expect("new transfer should stay stored");
			assert_eq!(first_transfer.state, TransferOutState::Ready);
			assert_eq!(second_transfer.microgons_per_argonot, first_transfer.microgons_per_argonot);
			assert_eq!(
				PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum),
				vec![PendingCollateralizationRequest::<Test> {
					transfer_id: second_transfer_id,
					remaining_collateral: 20_000,
					remaining_minting_authority_tip: 20,
				}],
			);

			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account),
				second_transfer_id,
				transfer_collateral_signature(&authority_pair, second_transfer_id, 20_000, 0),
				20_000,
				0,
			));

			let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
				.expect("authority should stay registered");
			let finalized_transfer = TransferOutById::<Test>::get(second_transfer_id)
				.expect("previous-window transfer should still accept collateral");
			assert_eq!(authority.pending_reserved_microgon_collateral, 40_000);
			assert_eq!(
				authority.active_pending_transfer_ids,
				vec![first_transfer_id, second_transfer_id]
			);
			assert_eq!(finalized_transfer.state, TransferOutState::Ready);
		});
	}

	#[test]
	fn collateralize_transfer_rejects_duplicate_signer_and_allows_another_authority_to_complete() {
		new_test_ext().execute_with(|| {
			let first_authority_account = account(64);
			let first_authority_pair = council_signing_pair(83);
			let first_signing_key = activate_test_minting_authority(
				first_authority_account.clone(),
				42,
				20_000,
				&council_signing_pair(84),
				&first_authority_pair,
				20_000,
				0,
			);
			let second_authority_account = account(166);
			let second_authority_pair = council_signing_pair(185);
			let second_signing_key = activate_test_minting_authority(
				second_authority_account.clone(),
				46,
				20_000,
				&council_signing_pair(186),
				&second_authority_pair,
				20_000,
				0,
			);
			let user = account(65);

			assert_ok!(Balances::mint_into(&user, 20_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x46),
				15_000,
			));
			let transfer_id = transfer_out_id(&user, 1);

			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(first_authority_account.clone()),
				transfer_id,
				transfer_collateral_signature(&first_authority_pair, transfer_id, 10_000, 0),
				10_000,
				0,
			));
			assert_noop!(
				CrosschainTransfer::collateralize_transfer(
					RuntimeOrigin::signed(first_authority_account),
					transfer_id,
					transfer_collateral_signature(&first_authority_pair, transfer_id, 12_000, 0),
					12_000,
					0,
				),
				Error::<Test>::InvalidTransferCollateralUpdate,
			);
			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(second_authority_account),
				transfer_id,
				transfer_collateral_signature(&second_authority_pair, transfer_id, 5_000, 0),
				5_000,
				0,
			));

			let first_authority = MintingAuthoritiesBySigner::<Test>::get(first_signing_key)
				.expect("first authority should remain registered");
			let second_authority = MintingAuthoritiesBySigner::<Test>::get(second_signing_key)
				.expect("second authority should remain registered");
			let transfer =
				TransferOutById::<Test>::get(transfer_id).expect("transfer should still exist");
			assert_eq!(first_authority.pending_reserved_microgon_collateral, 10_000);
			assert_eq!(first_authority.active_pending_transfer_ids, vec![transfer_id]);
			assert_eq!(second_authority.pending_reserved_microgon_collateral, 5_000);
			assert_eq!(second_authority.active_pending_transfer_ids, vec![transfer_id]);
			assert_eq!(transfer.state, TransferOutState::Ready);
			assert_eq!(transfer.total_attached_collateral, 15_000);
		});
	}

	#[test]
	fn collateralize_transfer_for_argon_requires_exhausting_microgons_before_micronots() {
		new_test_ext().execute_with(|| {
			let authority_account = account(167);
			let authority_pair = council_signing_pair(187);
			activate_test_minting_authority(
				authority_account.clone(),
				47,
				20_000,
				&council_signing_pair(188),
				&authority_pair,
				10_000,
				10_000,
			);
			let user = account(168);

			assert_ok!(Balances::mint_into(&user, 20_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x49),
				15_000,
			));
			let transfer_id = transfer_out_id(&user, 1);

			assert_noop!(
				CrosschainTransfer::collateralize_transfer(
					RuntimeOrigin::signed(authority_account.clone()),
					transfer_id,
					transfer_collateral_signature(&authority_pair, transfer_id, 5_000, 5_000),
					5_000,
					5_000,
				),
				Error::<Test>::InvalidTransferCollateral,
			);

			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account),
				transfer_id,
				transfer_collateral_signature(&authority_pair, transfer_id, 10_000, 5_000),
				10_000,
				5_000,
			));

			let transfer =
				TransferOutById::<Test>::get(transfer_id).expect("transfer should stay stored");
			assert_eq!(transfer.state, TransferOutState::Ready);
			assert_eq!(transfer.total_attached_collateral, 15_000);
		});
	}

	#[test]
	fn finalize_transfer_out_reconciles_external_consumption_and_invalidates_newer_rows() {
		new_test_ext().execute_with(|| {
			let authority_account = account(66);
			let authority_pair = council_signing_pair(85);
			let signing_key = activate_test_minting_authority(
				authority_account.clone(),
				43,
				20_000,
				&council_signing_pair(86),
				&authority_pair,
				10_000,
				0,
			);
			let first_user = account(67);
			let second_user = account(68);

			assert_ok!(Balances::mint_into(&first_user, 6_000));
			assert_ok!(Balances::mint_into(&second_user, 11_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(first_user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x47),
				5_000,
			));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(second_user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x48),
				10_000,
			));
			let first_transfer_id = transfer_out_id(&first_user, 1);
			let second_transfer_id = transfer_out_id(&second_user, 1);

			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account),
				second_transfer_id,
				transfer_collateral_signature(&authority_pair, second_transfer_id, 10_000, 0),
				10_000,
				0,
			));
			assert_eq!(
				TransferOutById::<Test>::get(second_transfer_id)
					.expect("second transfer should stay stored")
					.state,
				TransferOutState::Ready,
			);

			assert_ok!(CrosschainTransfer::finalize_transfer_out_from_gateway(
				SourceChain::Ethereum,
				first_transfer_id,
				AssetKind::Argon,
				5_000,
				vec![GatewayMintingAuthorityCollateral::<Test> {
					signing_key,
					microgon_collateral: 5_000,
					micronot_collateral: 0,
				}],
			));

			let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
				.expect("authority should remain registered");
			let second_transfer = TransferOutById::<Test>::get(second_transfer_id)
				.expect("second transfer should remain");
			assert!(TransferOutById::<Test>::get(first_transfer_id).is_none());
			assert_eq!(authority.gateway_remaining_microgon_collateral, 5_000);
			assert_eq!(authority.pending_reserved_microgon_collateral, 0);
			assert_eq!(encumbered_bond_microgons(&account(66)), 5_000);
			assert_eq!(active_bond_microgons(&account(66)), 10_000);
			assert_eq!(second_transfer.state, TransferOutState::Started);
			assert_eq!(second_transfer.total_attached_collateral, 0);
			assert!(second_transfer.minting_authority_collateral_by_signer.is_empty());
			assert!(authority.active_pending_transfer_ids.is_empty());
			assert_eq!(
				PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum),
				vec![PendingCollateralizationRequest::<Test> {
					transfer_id: second_transfer_id,
					remaining_collateral: 10_000,
					remaining_minting_authority_tip: 10,
				}],
			);
			assert_eq!(
				NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
				1,
			);
			assert_eq!(
				TransferTotalsByAccount::<Test>::get(&first_user),
				AccountTransferTotals {
					microgons_out: 5_000,
					argon_transfers_out_count: 1,
					..Default::default()
				},
			);
			assert_eq!(
				TransferTotalsByAccount::<Test>::get(&second_user),
				AccountTransferTotals::default(),
			);
		});
	}

	#[test]
	fn finalize_transfer_out_unknown_transfer_invalidates_newer_local_reservation() {
		new_test_ext().execute_with(|| {
			let authority_account = account(173);
			let authority_pair = council_signing_pair(90);
			let signing_key = activate_test_minting_authority(
				authority_account.clone(),
				45,
				20_000,
				&council_signing_pair(91),
				&authority_pair,
				10_000,
				0,
			);
			let user = account(174);

			assert_ok!(Balances::mint_into(&user, 11_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x59),
				10_000,
			));
			let local_transfer_id = transfer_out_id(&user, 1);
			let external_transfer_id = H256::repeat_byte(0x92);

			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account.clone()),
				local_transfer_id,
				transfer_collateral_signature(&authority_pair, local_transfer_id, 10_000, 0),
				10_000,
				0,
			));
			assert_eq!(
				TransferOutById::<Test>::get(local_transfer_id)
					.expect("local transfer should stay stored")
					.state,
				TransferOutState::Ready,
			);

			assert_ok!(CrosschainTransfer::finalize_transfer_out_from_gateway(
				SourceChain::Ethereum,
				external_transfer_id,
				AssetKind::Argon,
				4_000,
				vec![GatewayMintingAuthorityCollateral::<Test> {
					signing_key,
					microgon_collateral: 4_000,
					micronot_collateral: 0,
				}],
			));

			let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
				.expect("authority should remain registered");
			let local_transfer = TransferOutById::<Test>::get(local_transfer_id)
				.expect("local transfer should remain");
			assert!(TransferOutById::<Test>::get(external_transfer_id).is_none());
			assert_eq!(authority.gateway_remaining_microgon_collateral, 6_000);
			assert_eq!(authority.pending_reserved_microgon_collateral, 0);
			assert_eq!(encumbered_bond_microgons(&authority_account), 6_000);
			assert_eq!(active_bond_microgons(&authority_account), 6_000);
			assert_eq!(local_transfer.state, TransferOutState::Started);
			assert_eq!(local_transfer.total_attached_collateral, 0);
			assert!(local_transfer.minting_authority_collateral_by_signer.is_empty());
			assert!(authority.active_pending_transfer_ids.is_empty());
			assert_eq!(
				PendingCollateralizationRequestsByChain::<Test>::get(SourceChain::Ethereum),
				vec![PendingCollateralizationRequest::<Test> {
					transfer_id: local_transfer_id,
					remaining_collateral: 10_000,
					remaining_minting_authority_tip: 10,
				}],
			);
			assert_eq!(
				NonTerminalTransferOutCountByDestinationChain::<Test>::get(SourceChain::Ethereum),
				1,
			);
		});
	}

	#[test]
	fn finalize_transfer_out_burns_backing_for_unknown_local_transfer() {
		new_test_ext().execute_with(|| {
			let authority_account = account(169);
			let authority_pair = council_signing_pair(88);
			let signing_key = activate_test_minting_authority(
				authority_account.clone(),
				44,
				20_000,
				&council_signing_pair(89),
				&authority_pair,
				10_000,
				250,
			);
			let transfer_id = H256::repeat_byte(0x91);
			assert_ok!(CrosschainTransfer::finalize_transfer_out_from_gateway(
				SourceChain::Ethereum,
				transfer_id,
				AssetKind::Argon,
				4_000,
				vec![GatewayMintingAuthorityCollateral::<Test> {
					signing_key,
					microgon_collateral: 4_000,
					micronot_collateral: 100,
				}],
			));

			let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
				.expect("authority should remain registered");
			assert!(TransferOutById::<Test>::get(transfer_id).is_none());
			assert_eq!(authority.gateway_remaining_microgon_collateral, 6_000);
			assert_eq!(authority.gateway_remaining_micronot_collateral, 150);
			assert_eq!(encumbered_bond_microgons(&authority_account), 6_000);
			assert_eq!(active_bond_microgons(&authority_account), 6_000);
			assert_eq!(encumbered_argonot_micronots(&authority_account), 150);
			assert_eq!(committed_argonot_micronots(&authority_account), 150);
		});
	}

	#[test]
	fn cancel_transfer_out_refunds_principal_and_tip() {
		new_test_ext().execute_with(|| {
			let council_account = account(69);
			let council_pair = council_signing_pair(87);
			let user = account(70);
			let second_user = account(71);

			configure_single_member_ethereum_council(council_account, 44, 20_000, &council_pair);
			assert_ok!(Balances::mint_into(&user, 25_000));
			assert_ok!(Balances::mint_into(&second_user, 6_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x49),
				20_000,
			));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(second_user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x50),
				5_000,
			));
			let transfer_id = transfer_out_id(&user, 1);
			let second_transfer_id = transfer_out_id(&second_user, 1);

			assert_ok!(CrosschainTransfer::cancel_transfer_out_from_gateway(
				SourceChain::Ethereum,
				transfer_id,
			));

			let burn_account = CrosschainTransfer::burn_account(SourceChain::Ethereum);
			assert!(TransferOutById::<Test>::get(transfer_id).is_none());
			assert_eq!(Balances::balance(&burn_account), 5_000);
			assert_eq!(Balances::balance(&user), 25_000);
			assert_eq!(
				TransferOutById::<Test>::get(second_transfer_id).map(|transfer| transfer.amount),
				Some(5_000)
			);
			assert_eq!(
				TransferTotalsByAccount::<Test>::get(&user),
				AccountTransferTotals::default(),
			);
			assert_eq!(
				TransferTotalsByAccount::<Test>::get(&second_user),
				AccountTransferTotals::default(),
			);
			assert_eq!(
				Balances::balance_on_hold(
					&RuntimeHoldReason::CrosschainTransfer(
						HoldReason::TransferOutMintingAuthorityTip,
					),
					&user,
				),
				0,
			);
		});
	}

	#[test]
	fn cancel_transfer_out_skips_unknown_transfer_ids() {
		new_test_ext().execute_with(|| {
			let transfer_id = H256::repeat_byte(7);
			assert_ok!(CrosschainTransfer::cancel_transfer_out_from_gateway(
				SourceChain::Ethereum,
				transfer_id,
			));
		});
	}
}
