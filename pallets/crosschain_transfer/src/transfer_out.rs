use alloc::vec::Vec;
use alloy_primitives::{Address as AlloyAddress, B256};
use argon_ethereum_contracts::minting_gateway as ethereum_contracts;
use argon_primitives::{
	EthereumBlockNumber, EthereumVerifyProvider, TickProvider, MICROGONS_PER_ARGON,
};
use frame_support::{ensure, traits::fungible::MutateHold};
use pallet_prelude::*;
use polkadot_sdk::sp_core::ecdsa::KeccakSignature;

use super::{
	gateway_activity::GatewayMintingAuthorityCollateral,
	ActiveGlobalIssuanceCouncilByDestinationChain, AssetKind, ChainConfig,
	ChainConfigBySourceChain, Config, Error, Event, GatewaySyncPauseReason,
	GlobalIssuanceCouncilByHash, HoldReason, MintingAuthoritiesBySigner, MintingAuthorityState,
	NextTransferOutNonceBySendingAccountId, NonTerminalTransferOutCountByDestinationChain, Pallet,
	PendingCollateralizationRequestsByChain, SourceChain, TransferOutById, TransferOutRequestNonce,
	H160, H256,
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
	/// Council snapshot whose hash is baked into the request the gateway will verify.
	pub council_hash: H256,
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
		Self::ensure_source_chain_not_paused(destination_chain)?;
		ensure!(amount != T::Balance::default(), Error::<T>::InvalidTransferOutAmount);
		ensure!(destination_account != H160::zero(), Error::<T>::InvalidTransferOutRecipient);

		let active_council_hash =
			ActiveGlobalIssuanceCouncilByDestinationChain::<T>::get(destination_chain)
				.ok_or(Error::<T>::GlobalIssuanceCouncilNotFound)?;
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
				B256::from(active_council_hash.0),
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
			council_hash: active_council_hash,
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
			let council = GlobalIssuanceCouncilByHash::<T>::get(transfer.council_hash)
				.ok_or(Error::<T>::GlobalIssuanceCouncilNotFound)?;
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
				council.epoch_microgons_per_argonot,
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
		}

		if let Some(transfer) = local_transfer.as_ref() {
			Self::pay_transfer_out_minting_authority_tip(transfer, &finalized_collateral_rows)?;
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

		let council = GlobalIssuanceCouncilByHash::<T>::get(transfer.council_hash)
			.ok_or(GatewaySyncPauseReason::MintingAuthorityMismatch)?;
		let mut payouts = finalized_collateral_rows
			.iter()
			.map(|collateral_row| {
				let collateral_share = Self::normalized_transfer_collateral(
					transfer.asset,
					council.epoch_microgons_per_argonot,
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
				B256::from(transfer.council_hash.0),
				AlloyAddress::from_slice(transfer.destination_account.as_bytes()),
				transfer.valid_until_ethereum_block,
				AlloyAddress::from_slice(token.as_bytes()),
				transfer.amount.into(),
				transfer.minting_authority_tip.into(),
			)
			.as_slice(),
		))
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
