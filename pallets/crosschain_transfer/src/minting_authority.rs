use argon_primitives::{
	vault::BitcoinVaultProvider, PriceProvider, TreasuryPoolProvider, MICROGONS_PER_ARGON,
};
use pallet_prelude::*;
use polkadot_sdk::sp_runtime::BoundedBTreeMap;
use sp_core::ecdsa::KeccakSignature;

use super::{
	ActiveGlobalIssuanceCouncilByDestinationChain, Config,
	CouncilApprovalQueueByDestinationChainAndNonce, CouncilApprovalQueueEntry,
	CouncilApprovalQueueNonce, CouncilApprovalTargetId, Error, Event, GatewayStateBySourceChain,
	GlobalIssuanceCouncilByHash, HoldReason, LatestQueuedCouncilHashByDestinationChain,
	MinimumMintingAuthorityValueByDestinationChain, MintingAuthoritiesBySigner, MintingAuthority,
	MintingAuthorityActivationRepaymentPricing,
	MintingAuthorityActivationRepaymentPricingByDestinationChain, MintingAuthorityState,
	NextCouncilApprovalQueueNonceByDestinationChain, Pallet, SourceChain, H160, H256, WEI_PER_ETH,
};

impl<T: Config> Pallet<T> {
	pub(crate) fn do_register_minting_authority(
		vault_operator_account_id: T::AccountId,
		destination_chain: SourceChain,
		destination_signing_key: H160,
		signature: KeccakSignature,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
	) -> DispatchResult {
		Self::ensure_source_chain_not_paused(destination_chain)?;
		let _ = Self::evm_gateway_signature_domain(destination_chain)?;
		ensure!(
			destination_signing_key != H160::zero(),
			Error::<T>::InvalidMintingAuthoritySigningKey,
		);
		ensure!(
			microgon_collateral != T::Balance::default() ||
				micronot_collateral != T::Balance::default(),
			Error::<T>::InvalidMintingAuthorityCollateral,
		);

		T::VaultProvider::get_vault_id(&vault_operator_account_id)
			.ok_or(Error::<T>::UnknownOwnerVault)?;
		ensure!(
			Self::recover_evm_personal_signer(
				&Self::minting_authority_signer_registration_message(
					destination_chain,
					&vault_operator_account_id,
				),
				&signature,
			) == Some(destination_signing_key),
			Error::<T>::InvalidMintingAuthoritySigningKeyProof,
		);
		ensure!(
			!MintingAuthoritiesBySigner::<T>::contains_key(destination_signing_key),
			Error::<T>::MintingAuthorityAlreadyRegistered,
		);
		let minimum_value =
			MinimumMintingAuthorityValueByDestinationChain::<T>::get(destination_chain);
		let micronot_value = if micronot_collateral == T::Balance::default() {
			T::Balance::default()
		} else {
			let microgons_per_argonot =
				T::PriceProvider::get_lowest_microgons_per_argonot(T::CouncilRotationFrames::get())
					.filter(|price| *price != T::Balance::default())
					.ok_or(Error::<T>::InvalidMicrogonsPerArgonot)?;
			let microgons_per_argonot: u128 = microgons_per_argonot.into();
			micronot_collateral
				.into()
				.saturating_mul(microgons_per_argonot)
				.saturating_div(MICROGONS_PER_ARGON)
				.into()
		};
		let total_collateral_value = microgon_collateral.saturating_add(micronot_value);
		ensure!(
			total_collateral_value >= minimum_value,
			Error::<T>::MintingAuthorityCollateralBelowMinimum,
		);

		let approval_queue_nonce =
			NextCouncilApprovalQueueNonceByDestinationChain::<T>::get(destination_chain)
				.saturating_add(1);
		let previous_approval_hash =
			Self::previous_gateway_update_hash(destination_chain, approval_queue_nonce)?;
		let approving_council_hash =
			LatestQueuedCouncilHashByDestinationChain::<T>::get(destination_chain)
				.or_else(|| {
					ActiveGlobalIssuanceCouncilByDestinationChain::<T>::get(destination_chain)
				})
				.ok_or(Error::<T>::GlobalIssuanceCouncilNotFound)?;
		let (activation_base_repayment_quote, activation_signature_repayment_quote) =
			Self::minting_authority_activation_repayment_quote(
				destination_chain,
				approval_queue_nonce,
				approving_council_hash,
			)?;
		let activation_repayment_hold_amount =
			activation_base_repayment_quote.saturating_add(activation_signature_repayment_quote);
		let target_payload_hash = Self::hash_activate_minting_authority(
			destination_chain,
			microgon_collateral,
			micronot_collateral,
			destination_signing_key,
		)?;
		let mut queue_entry = CouncilApprovalQueueEntry::<T> {
			approving_council_hash,
			target: CouncilApprovalTargetId::MintingAuthorityActivation(destination_signing_key),
			target_payload_hash,
			due_frame_id: Self::queue_entry_due_frame_id(),
			previous_approval_hash,
			approval_hash: H256::zero(),
			approved_total_weight: T::Balance::default(),
			signatures: BoundedBTreeMap::new(),
		};
		queue_entry.approval_hash = Self::hash_council_approval_queue_entry(
			destination_chain,
			approval_queue_nonce,
			&queue_entry,
		)?;

		if microgon_collateral != T::Balance::default() {
			T::TreasuryPoolProvider::encumber_bond_microgons(
				&vault_operator_account_id,
				microgon_collateral,
			)
			.map_err(|_| Error::<T>::InsufficientCommittedMicrogonCollateral)?;
		}
		if micronot_collateral != T::Balance::default() {
			T::VaultProvider::encumber_argonots(&vault_operator_account_id, micronot_collateral)
				.map_err(|_| Error::<T>::InsufficientCommittedArgonotCollateral)?;
		}
		if activation_repayment_hold_amount != T::Balance::default() {
			if T::NativeCurrency::balance_on_hold(
				&HoldReason::MintingAuthorityActivationRepayment.into(),
				&vault_operator_account_id,
			) == T::Balance::default()
			{
				frame_system::Pallet::<T>::inc_providers(&vault_operator_account_id);
			}
			T::NativeCurrency::hold(
				&HoldReason::MintingAuthorityActivationRepayment.into(),
				&vault_operator_account_id,
				activation_repayment_hold_amount,
			)?;
		}

		MintingAuthoritiesBySigner::<T>::insert(
			destination_signing_key,
			MintingAuthority::<T> {
				account_id: vault_operator_account_id.clone(),
				destination_chain,
				destination_signing_key,
				state: MintingAuthorityState::PendingActivation,
				gateway_remaining_microgon_collateral: microgon_collateral,
				gateway_remaining_micronot_collateral: micronot_collateral,
				pending_reserved_microgon_collateral: T::Balance::default(),
				pending_reserved_micronot_collateral: T::Balance::default(),
				active_pending_transfer_ids: BoundedVec::default(),
				activation_approval_queue_nonce: approval_queue_nonce,
				activation_base_repayment_quote,
				activation_signature_repayment_quote,
				deactivation_approval_queue_nonce: None,
			},
		);
		CouncilApprovalQueueByDestinationChainAndNonce::<T>::insert(
			destination_chain,
			approval_queue_nonce,
			queue_entry,
		);
		Self::update_destination_chain_queue_tracking_after_insert(
			destination_chain,
			approving_council_hash,
			&CouncilApprovalTargetId::MintingAuthorityActivation(destination_signing_key),
		)?;
		NextCouncilApprovalQueueNonceByDestinationChain::<T>::insert(
			destination_chain,
			approval_queue_nonce,
		);
		Self::deposit_event(Event::MintingAuthorityRegistered {
			destination_chain,
			destination_signing_key,
			account_id: vault_operator_account_id,
			approval_queue_nonce,
		});
		Ok(())
	}

	pub(crate) fn do_deactivate_minting_authority(
		account_id: T::AccountId,
		destination_signing_key: H160,
		signature: KeccakSignature,
	) -> DispatchResultWithPostInfo {
		let mut destination_chain = None;
		let mut queued_nonce = None;

		MintingAuthoritiesBySigner::<T>::try_mutate(
			destination_signing_key,
			|maybe_authority| -> DispatchResult {
				let authority =
					maybe_authority.as_mut().ok_or(Error::<T>::MintingAuthorityNotFound)?;
				ensure!(authority.account_id == account_id, Error::<T>::MintingAuthorityMismatch,);
				Self::ensure_source_chain_not_paused(authority.destination_chain)?;
				ensure!(
					matches!(
						authority.state,
						MintingAuthorityState::Active | MintingAuthorityState::Deactivating
					),
					Error::<T>::UnexpectedMintingAuthorityState,
				);

				destination_chain = Some(authority.destination_chain);
				authority.state = MintingAuthorityState::Deactivating;
				let queue_nonce =
					if let Some(existing_nonce) = authority.deactivation_approval_queue_nonce {
						existing_nonce
					} else {
						let approving_council_hash =
							LatestQueuedCouncilHashByDestinationChain::<T>::get(
								authority.destination_chain,
							)
							.or_else(|| {
								ActiveGlobalIssuanceCouncilByDestinationChain::<T>::get(
									authority.destination_chain,
								)
							})
							.ok_or(Error::<T>::GlobalIssuanceCouncilNotFound)?;
						let next_queue_nonce =
							NextCouncilApprovalQueueNonceByDestinationChain::<T>::mutate(
								authority.destination_chain,
								|next_nonce| {
									*next_nonce = next_nonce.saturating_add(1);
									*next_nonce
								},
							);
						let previous_approval_hash = Self::previous_gateway_update_hash(
							authority.destination_chain,
							next_queue_nonce,
						)?;
						let mut queue_entry = CouncilApprovalQueueEntry::<T> {
							approving_council_hash,
							target: CouncilApprovalTargetId::MintingAuthorityDeactivation(
								destination_signing_key,
							),
							target_payload_hash: Self::hash_deactivate_minting_authority_target(
								destination_signing_key,
							),
							due_frame_id: Self::queue_entry_due_frame_id(),
							previous_approval_hash,
							approval_hash: H256::zero(),
							approved_total_weight: T::Balance::default(),
							signatures: BoundedBTreeMap::new(),
						};
						queue_entry.approval_hash = Self::hash_council_approval_queue_entry(
							authority.destination_chain,
							next_queue_nonce,
							&queue_entry,
						)?;
						CouncilApprovalQueueByDestinationChainAndNonce::<T>::insert(
							authority.destination_chain,
							next_queue_nonce,
							queue_entry,
						);
						Self::update_destination_chain_queue_tracking_after_insert(
							authority.destination_chain,
							approving_council_hash,
							&CouncilApprovalTargetId::MintingAuthorityDeactivation(
								destination_signing_key,
							),
						)?;
						authority.deactivation_approval_queue_nonce = Some(next_queue_nonce);
						queued_nonce = Some(next_queue_nonce);
						next_queue_nonce
					};
				let queue_entry = CouncilApprovalQueueByDestinationChainAndNonce::<T>::get(
					authority.destination_chain,
					queue_nonce,
				)
				.ok_or(Error::<T>::CouncilApprovalQueueEntryNotFound)?;
				ensure!(
					Self::recover_evm_message_signer(queue_entry.approval_hash, &signature) ==
						Some(destination_signing_key),
					Error::<T>::InvalidMintingAuthorityDeactivationSignature,
				);

				CouncilApprovalQueueByDestinationChainAndNonce::<T>::try_mutate(
					authority.destination_chain,
					queue_nonce,
					|entry| -> DispatchResult {
						let entry =
							entry.as_mut().ok_or(Error::<T>::CouncilApprovalQueueEntryNotFound)?;
						ensure!(
							entry.target ==
								CouncilApprovalTargetId::MintingAuthorityDeactivation(
									destination_signing_key,
								),
							Error::<T>::MintingAuthorityMismatch,
						);
						entry.signatures.remove(&destination_signing_key);
						let _ = entry
							.signatures
							.try_insert(destination_signing_key, signature)
							.map_err(|_| Error::<T>::InvalidGlobalIssuanceCouncil)?;
						Ok(())
					},
				)?;
				Ok(())
			},
		)?;

		if let (Some(destination_chain), Some(approval_queue_nonce)) =
			(destination_chain, queued_nonce)
		{
			Self::deposit_event(Event::MintingAuthorityDeactivationQueued {
				destination_chain,
				destination_signing_key,
				approval_queue_nonce,
			});
		}

		Ok(Pays::No.into())
	}

	#[allow(clippy::type_complexity)]
	pub(crate) fn minting_authority_activation_repayment_quote(
		destination_chain: SourceChain,
		approval_queue_nonce: CouncilApprovalQueueNonce,
		approving_council_hash: H256,
	) -> Result<(T::Balance, T::Balance), DispatchError> {
		let pricing = MintingAuthorityActivationRepaymentPricingByDestinationChain::<T>::get(
			destination_chain,
		)
		.ok_or(Error::<T>::MissingMintingAuthorityActivationRepaymentPricing)?;
		let activation_base_repayment_quote = Self::minting_authority_activation_gas_repayment_due(
			&pricing,
			pricing.activation_gas_cost,
		)?;
		let activation_signature_repayment_quote =
			Self::minting_authority_activation_gas_repayment_due(
				&pricing,
				pricing.signature_gas_cost,
			)?;
		let mut activation_signature_repayment_quote_count =
			Self::council_signature_quote(approving_council_hash)?;
		let first_unresolved_nonce = GatewayStateBySourceChain::<T>::get(destination_chain)
			.map_or(1, |state| state.argon_approvals_nonce.saturating_add(1));
		for queue_nonce in first_unresolved_nonce..approval_queue_nonce {
			let Some(entry) = CouncilApprovalQueueByDestinationChainAndNonce::<T>::get(
				destination_chain,
				queue_nonce,
			) else {
				continue;
			};
			if !matches!(entry.target, CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(_)) {
				continue;
			}
			activation_signature_repayment_quote_count = activation_signature_repayment_quote_count
				.saturating_add(Self::council_signature_quote(entry.approving_council_hash)?);
		}
		let activation_signature_repayment_quote = activation_signature_repayment_quote
			.saturating_mul(activation_signature_repayment_quote_count.into());
		let activation_repayment_hold_amount =
			activation_base_repayment_quote.saturating_add(activation_signature_repayment_quote);
		if activation_repayment_hold_amount == T::Balance::default() {
			return Err(Error::<T>::MissingMintingAuthorityActivationRepaymentPricing.into());
		}
		Ok((activation_base_repayment_quote, activation_signature_repayment_quote))
	}

	pub(crate) fn minting_authority_activation_gas_repayment_due(
		pricing: &MintingAuthorityActivationRepaymentPricing<T>,
		gas_units: u128,
	) -> Result<T::Balance, DispatchError> {
		let microgons_per_eth: u128 = pricing.estimated_microgons_per_eth.into();
		let wei_cost = gas_units.saturating_mul(pricing.estimated_wei_per_gas);
		let total_microgons =
			microgons_per_eth.saturating_mul(wei_cost).saturating_div(WEI_PER_ETH);
		if total_microgons == 0 {
			return Err(Error::<T>::MissingMintingAuthorityActivationRepaymentPricing.into());
		}
		Ok(total_microgons.into())
	}

	pub(crate) fn council_signature_quote(council_hash: H256) -> Result<u32, DispatchError> {
		let council = GlobalIssuanceCouncilByHash::<T>::get(council_hash)
			.ok_or(Error::<T>::GlobalIssuanceCouncilNotFound)?;
		Ok(council.members.len() as u32)
	}

	pub(crate) fn release_minting_authority_collateral(
		account_id: T::AccountId,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
	) -> DispatchResult {
		if microgon_collateral != T::Balance::default() {
			T::TreasuryPoolProvider::release_encumbered_bond_microgons(
				&account_id,
				microgon_collateral,
			)
			.map_err(|_| Error::<T>::InsufficientCommittedMicrogonCollateral)?;
		}
		if micronot_collateral != T::Balance::default() {
			T::VaultProvider::release_encumbered_argonots(&account_id, micronot_collateral)
				.map_err(|_| Error::<T>::UnknownOwnerVault)?;
		}
		Ok(())
	}

	pub(crate) fn burn_minting_authority_collateral(
		account_id: T::AccountId,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
	) -> DispatchResult {
		if microgon_collateral != T::Balance::default() {
			T::TreasuryPoolProvider::burn_encumbered_bond_microgons(
				&account_id,
				microgon_collateral,
			)
			.map_err(|_| Error::<T>::InsufficientCommittedMicrogonCollateral)?;
		}
		if micronot_collateral != T::Balance::default() {
			T::VaultProvider::burn_encumbered_argonots(&account_id, micronot_collateral)
				.map_err(|_| Error::<T>::UnknownOwnerVault)?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		tests::{
			account, activate_test_minting_authority, assert_noop, assert_ok, bounded_vec,
			encumbered_argonot_micronots, encumbered_bond_microgons, h160,
			minting_authority_deactivation_signature, minting_authority_registration_signature,
			new_test_ext, set_active_bond_amount, set_committed_argonots,
			transfer_collateral_signature, Balances, CrosschainTransfer, Mutate, Ownership,
			RuntimeOrigin, TokenError,
		},
		ActiveGlobalIssuanceCouncilByDestinationChain,
		CouncilApprovalCursorByDestinationChainAndAccountId,
		CouncilApprovalQueueByDestinationChainAndNonce, CouncilApprovalTargetId, Error,
		MinimumMintingAuthorityValueByDestinationChain, MintingAuthoritiesBySigner,
		MintingAuthority, MintingAuthorityState, SourceChain,
	};
	use argon_ethereum_contracts::minting_gateway as ethereum_contracts;

	use crate::tests::{
		configure_single_member_ethereum_council, council_signer, council_signing_pair,
		transfer_out_id, AssetKind, Test, H256,
	};

	#[test]
	fn register_minting_authority_validates_inputs_encumbers_collateral_and_queues_approval() {
		new_test_ext().execute_with(|| {
			let owner_vault_operator = account(30);
			let council_pair = council_signing_pair(11);
			let minting_authority_pair = council_signing_pair(12);
			let wrong_pair = council_signing_pair(13);
			let signing_key = council_signer(&minting_authority_pair);

			configure_single_member_ethereum_council(
				owner_vault_operator.clone(),
				7,
				10_000,
				&council_pair,
			);
			assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
			assert_ok!(Ownership::mint_into(&owner_vault_operator, 900));
			assert_ok!(set_committed_argonots(owner_vault_operator.clone(), 500));
			assert_noop!(
				CrosschainTransfer::register_minting_authority(
					RuntimeOrigin::signed(owner_vault_operator.clone()),
					SourceChain::Ethereum,
					signing_key,
					minting_authority_registration_signature(&wrong_pair, &owner_vault_operator),
					10_000,
					300,
				),
				Error::<Test>::InvalidMintingAuthoritySigningKeyProof,
			);
			set_active_bond_amount(7, owner_vault_operator.clone(), 500);
			assert_noop!(
				CrosschainTransfer::register_minting_authority(
					RuntimeOrigin::signed(owner_vault_operator.clone()),
					SourceChain::Ethereum,
					signing_key,
					minting_authority_registration_signature(
						&minting_authority_pair,
						&owner_vault_operator,
					),
					10_000,
					300,
				),
				Error::<Test>::InsufficientCommittedMicrogonCollateral,
			);
			set_active_bond_amount(7, owner_vault_operator.clone(), 10_000);

			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				10_000,
				300,
			));
			assert_eq!(
				MintingAuthoritiesBySigner::<Test>::get(signing_key),
				Some(MintingAuthority::<Test> {
					account_id: owner_vault_operator.clone(),
					destination_chain: SourceChain::Ethereum,
					destination_signing_key: signing_key,
					state: MintingAuthorityState::PendingActivation,
					gateway_remaining_microgon_collateral: 10_000,
					gateway_remaining_micronot_collateral: 300,
					pending_reserved_microgon_collateral: 0,
					pending_reserved_micronot_collateral: 0,
					active_pending_transfer_ids: bounded_vec![],
					activation_approval_queue_nonce: 1,
					activation_base_repayment_quote: 100,
					activation_signature_repayment_quote: 50,
					deactivation_approval_queue_nonce: None,
				}),
			);
			assert_eq!(encumbered_bond_microgons(&owner_vault_operator), 10_000);
			assert_eq!(encumbered_argonot_micronots(&owner_vault_operator), 300);
			assert_noop!(
				set_committed_argonots(owner_vault_operator.clone(), 299),
				TokenError::Frozen,
			);

			let queue_entry = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
				SourceChain::Ethereum,
				1,
			)
			.expect("queue entry should be stored");
			assert_eq!(
				queue_entry.approving_council_hash,
				ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum,)
					.expect("council should stay active"),
			);
			assert_eq!(
				queue_entry.target,
				CouncilApprovalTargetId::MintingAuthorityActivation(signing_key),
			);
			assert_eq!(
				queue_entry.target_payload_hash,
				H256::from_slice(
					ethereum_contracts::hash_activate_minting_authority(
						1,
						alloy_primitives::Address::from_slice(h160(0x21).as_bytes()),
						10_000,
						300,
						alloy_primitives::Address::from_slice(signing_key.as_bytes()),
					)
					.as_slice(),
				),
			);
			assert_eq!(queue_entry.previous_approval_hash, H256::zero());
			assert_ne!(queue_entry.approval_hash, H256::zero());
			assert_eq!(
				CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
					SourceChain::Ethereum,
					owner_vault_operator.clone(),
				),
				Some(0),
			);
			assert_noop!(
				CrosschainTransfer::register_minting_authority(
					RuntimeOrigin::signed(owner_vault_operator.clone()),
					SourceChain::Ethereum,
					signing_key,
					minting_authority_registration_signature(
						&minting_authority_pair,
						&owner_vault_operator,
					),
					2_000,
					100,
				),
				Error::<Test>::MintingAuthorityAlreadyRegistered,
			);
		});
	}

	#[test]
	fn register_minting_authority_uses_per_chain_minimum_override() {
		new_test_ext().execute_with(|| {
			let owner_vault_operator = account(31);
			let council_pair = council_signing_pair(14);
			let minting_authority_pair = council_signing_pair(15);
			let signing_key = council_signer(&minting_authority_pair);

			configure_single_member_ethereum_council(
				owner_vault_operator.clone(),
				17,
				10_000,
				&council_pair,
			);
			assert_ok!(Balances::mint_into(&owner_vault_operator, 10_000));
			assert_ok!(Ownership::mint_into(&owner_vault_operator, 900));
			assert_ok!(set_committed_argonots(owner_vault_operator.clone(), 300));

			assert_noop!(
				CrosschainTransfer::register_minting_authority(
					RuntimeOrigin::signed(owner_vault_operator.clone()),
					SourceChain::Ethereum,
					signing_key,
					minting_authority_registration_signature(
						&minting_authority_pair,
						&owner_vault_operator,
					),
					8_000,
					300,
				),
				Error::<Test>::MintingAuthorityCollateralBelowMinimum,
			);
			assert_ok!(CrosschainTransfer::set_minimum_minting_authority_value(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				8_300,
			));
			assert_eq!(
				MinimumMintingAuthorityValueByDestinationChain::<Test>::get(SourceChain::Ethereum,),
				8_300,
			);
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&owner_vault_operator,
				),
				10_000,
				300,
			));
		});
	}

	#[test]
	fn deactivate_minting_authority_moves_authority_into_deactivating_and_queues_entry() {
		new_test_ext().execute_with(|| {
			let authority_account = account(34);
			let authority_pair = council_signing_pair(24);
			let signing_key = activate_test_minting_authority(
				authority_account.clone(),
				24,
				20_000,
				&council_signing_pair(23),
				&authority_pair,
				10_000,
				0,
			);
			let deactivation_signature = minting_authority_deactivation_signature(
				&authority_pair,
				2,
				signing_key,
				CrosschainTransfer::previous_gateway_update_hash(SourceChain::Ethereum, 2)
					.expect("activation queue entry should anchor deactivation"),
			);

			assert_ok!(CrosschainTransfer::deactivate_minting_authority(
				RuntimeOrigin::signed(authority_account.clone()),
				signing_key,
				deactivation_signature,
			));

			let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
				.expect("authority should remain until proof-backed deactivation");
			assert_eq!(authority.account_id, authority_account);
			assert_eq!(authority.state, MintingAuthorityState::Deactivating);
			assert_eq!(authority.deactivation_approval_queue_nonce, Some(2));

			let deactivation_entry = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
				SourceChain::Ethereum,
				2,
			)
			.expect("deactivation queue entry should be stored");
			assert_eq!(
				deactivation_entry.target,
				CouncilApprovalTargetId::MintingAuthorityDeactivation(signing_key),
			);
			assert_eq!(
				deactivation_entry.target_payload_hash,
				CrosschainTransfer::hash_deactivate_minting_authority_target(signing_key),
			);
			assert_eq!(
				deactivation_entry.signatures.get(&signing_key),
				Some(&deactivation_signature),
			);
		});
	}

	#[test]
	fn deactivate_minting_authority_skips_council_cursor_blocking() {
		new_test_ext().execute_with(|| {
			let authority_account = account(35);
			let council_pair = council_signing_pair(22);
			let authority_pair = council_signing_pair(25);
			let second_authority_pair = council_signing_pair(26);
			let signing_key = activate_test_minting_authority(
				authority_account.clone(),
				25,
				30_000,
				&council_pair,
				&authority_pair,
				10_000,
				0,
			);
			let user = account(36);

			assert_ok!(Balances::mint_into(&user, 6_000));
			assert_ok!(CrosschainTransfer::transfer_out(
				RuntimeOrigin::signed(user.clone()),
				SourceChain::Ethereum,
				AssetKind::Argon,
				h160(0x72),
				5_000,
			));
			let transfer_id = transfer_out_id(&user, 1);
			assert_ok!(CrosschainTransfer::collateralize_transfer(
				RuntimeOrigin::signed(authority_account.clone()),
				transfer_id,
				transfer_collateral_signature(&authority_pair, transfer_id, 5_000, 0),
				5_000,
				0,
			));
			let previous_approval_hash =
				CrosschainTransfer::previous_gateway_update_hash(SourceChain::Ethereum, 2)
					.expect("activation queue entry should anchor deactivation");
			let deactivation_signature = minting_authority_deactivation_signature(
				&authority_pair,
				2,
				signing_key,
				previous_approval_hash,
			);
			assert_ok!(CrosschainTransfer::deactivate_minting_authority(
				RuntimeOrigin::signed(authority_account.clone()),
				signing_key,
				deactivation_signature,
			));

			let authority = MintingAuthoritiesBySigner::<Test>::get(signing_key)
				.expect("authority should remain until proof-backed deactivation");
			assert_eq!(authority.state, MintingAuthorityState::Deactivating);
			assert_eq!(authority.deactivation_approval_queue_nonce, Some(2));
			assert_eq!(authority.active_pending_transfer_ids, vec![transfer_id]);

			let second_signing_key = council_signer(&second_authority_pair);
			set_active_bond_amount(25, authority_account.clone(), 20_000);
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(authority_account.clone()),
				SourceChain::Ethereum,
				second_signing_key,
				minting_authority_registration_signature(
					&second_authority_pair,
					&authority_account,
				),
				10_000,
				0,
			));
			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(authority_account.clone()),
				SourceChain::Ethereum,
				bounded_vec![crate::tests::minting_authority_approval_signature(&council_pair, 3,)],
			));
			assert_eq!(
				CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
					SourceChain::Ethereum,
					authority_account,
				),
				Some(3),
			);
		});
	}
}
