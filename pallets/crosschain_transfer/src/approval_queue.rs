use alloc::vec::Vec;

use argon_primitives::block_seal::FrameId;
use frame_support::ensure;
use pallet_prelude::*;
use polkadot_sdk::sp_runtime::BoundedBTreeMap;

use super::{
	ActiveGlobalIssuanceCouncilByDestinationChain, Config,
	CouncilApprovalCursorByDestinationChainAndAccountId,
	CouncilApprovalQueueByDestinationChainAndNonce, CouncilApprovalQueueEntry,
	CouncilApprovalQueueNonce, CouncilApprovalTargetId,
	CurrentTransferOutMicrogonsPerArgonotByDestinationChain, Error, GatewayStateBySourceChain,
	GlobalIssuanceCouncil, GlobalIssuanceCouncilByHash, LatestQueuedCouncilHashByDestinationChain,
	MintingAuthoritiesBySigner, NextCouncilApprovalQueueNonceByDestinationChain, Pallet,
	PreviousTransferOutMicrogonsPerArgonotByDestinationChain, SourceChain,
	TransferOutQuoteMicrogonsPerArgonotByDestinationChain, H256,
};

impl<T: Config> Pallet<T> {
	pub(crate) fn previous_gateway_update_hash(
		destination_chain: SourceChain,
		queue_nonce: CouncilApprovalQueueNonce,
	) -> Result<H256, DispatchError> {
		if queue_nonce <= 1 {
			return Ok(H256::zero());
		}

		let previous_entry = CouncilApprovalQueueByDestinationChainAndNonce::<T>::get(
			destination_chain,
			queue_nonce.saturating_sub(1),
		)
		.ok_or(Error::<T>::CouncilApprovalQueueEntryNotFound)?;

		Ok(previous_entry.approval_hash)
	}

	pub(crate) fn update_destination_chain_queue_tracking_after_insert(
		destination_chain: SourceChain,
		approving_council_hash: H256,
		target: &CouncilApprovalTargetId,
	) -> DispatchResult {
		let mut latest_queued_council_hash =
			LatestQueuedCouncilHashByDestinationChain::<T>::get(destination_chain)
				.unwrap_or(approving_council_hash);
		let mut quoted_microgons_per_argonot =
			TransferOutQuoteMicrogonsPerArgonotByDestinationChain::<T>::get(destination_chain)
				.filter(|rate| *rate != T::Balance::default())
				.unwrap_or(Self::base_transfer_out_quote_microgons_per_argonot(destination_chain)?);
		let approving_council = GlobalIssuanceCouncilByHash::<T>::get(approving_council_hash)
			.ok_or(Error::<T>::GlobalIssuanceCouncilNotFound)?;
		quoted_microgons_per_argonot =
			quoted_microgons_per_argonot.min(approving_council.epoch_microgons_per_argonot);

		if let CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(council_hash) = target {
			let rotation_council = GlobalIssuanceCouncilByHash::<T>::get(*council_hash)
				.ok_or(Error::<T>::GlobalIssuanceCouncilNotFound)?;
			latest_queued_council_hash = *council_hash;
			quoted_microgons_per_argonot =
				quoted_microgons_per_argonot.min(rotation_council.epoch_microgons_per_argonot);
		}

		LatestQueuedCouncilHashByDestinationChain::<T>::insert(
			destination_chain,
			latest_queued_council_hash,
		);
		TransferOutQuoteMicrogonsPerArgonotByDestinationChain::<T>::insert(
			destination_chain,
			quoted_microgons_per_argonot,
		);
		Ok(())
	}

	pub(crate) fn refresh_destination_chain_queue_tracking(
		destination_chain: SourceChain,
	) -> DispatchResult {
		let Some(active_council_hash) =
			ActiveGlobalIssuanceCouncilByDestinationChain::<T>::get(destination_chain)
		else {
			LatestQueuedCouncilHashByDestinationChain::<T>::remove(destination_chain);
			TransferOutQuoteMicrogonsPerArgonotByDestinationChain::<T>::remove(destination_chain);
			return Ok(());
		};
		let mut latest_queued_council_hash = active_council_hash;
		let mut quoted_microgons_per_argonot =
			Self::base_transfer_out_quote_microgons_per_argonot(destination_chain)?;

		let first_unresolved_nonce = GatewayStateBySourceChain::<T>::get(destination_chain)
			.map_or(1, |state| state.argon_approvals_nonce.saturating_add(1));
		let last_queued_nonce =
			NextCouncilApprovalQueueNonceByDestinationChain::<T>::get(destination_chain);
		for queue_nonce in first_unresolved_nonce..=last_queued_nonce {
			let Some(entry) = CouncilApprovalQueueByDestinationChainAndNonce::<T>::get(
				destination_chain,
				queue_nonce,
			) else {
				continue;
			};
			let approving_council =
				GlobalIssuanceCouncilByHash::<T>::get(entry.approving_council_hash)
					.ok_or(Error::<T>::GlobalIssuanceCouncilNotFound)?;
			quoted_microgons_per_argonot =
				quoted_microgons_per_argonot.min(approving_council.epoch_microgons_per_argonot);
			if let CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(council_hash) =
				entry.target
			{
				let council = GlobalIssuanceCouncilByHash::<T>::get(council_hash)
					.ok_or(Error::<T>::GlobalIssuanceCouncilNotFound)?;
				latest_queued_council_hash = council_hash;
				quoted_microgons_per_argonot =
					quoted_microgons_per_argonot.min(council.epoch_microgons_per_argonot);
			}
		}

		LatestQueuedCouncilHashByDestinationChain::<T>::insert(
			destination_chain,
			latest_queued_council_hash,
		);
		TransferOutQuoteMicrogonsPerArgonotByDestinationChain::<T>::insert(
			destination_chain,
			quoted_microgons_per_argonot,
		);
		Ok(())
	}

	pub(crate) fn roll_active_global_issuance_council(
		destination_chain: SourceChain,
		council_hash: H256,
		epoch_microgons_per_argonot: T::Balance,
	) -> Option<H256> {
		let previous_council_hash =
			ActiveGlobalIssuanceCouncilByDestinationChain::<T>::get(destination_chain);
		let previous_transfer_out_rate =
			CurrentTransferOutMicrogonsPerArgonotByDestinationChain::<T>::get(destination_chain);

		ActiveGlobalIssuanceCouncilByDestinationChain::<T>::insert(destination_chain, council_hash);
		CurrentTransferOutMicrogonsPerArgonotByDestinationChain::<T>::insert(
			destination_chain,
			epoch_microgons_per_argonot,
		);

		if let Some(previous_transfer_out_rate) = previous_transfer_out_rate {
			PreviousTransferOutMicrogonsPerArgonotByDestinationChain::<T>::insert(
				destination_chain,
				previous_transfer_out_rate,
			);
		} else {
			PreviousTransferOutMicrogonsPerArgonotByDestinationChain::<T>::remove(
				destination_chain,
			);
		}

		previous_council_hash
	}

	pub(crate) fn prune_global_issuance_council_if_unreferenced(
		destination_chain: SourceChain,
		council_hash: H256,
	) {
		if ActiveGlobalIssuanceCouncilByDestinationChain::<T>::get(destination_chain) ==
			Some(council_hash)
		{
			return;
		}

		let first_unresolved_nonce = GatewayStateBySourceChain::<T>::get(destination_chain)
			.map_or(1, |state| state.argon_approvals_nonce.saturating_add(1));
		let last_queued_nonce =
			NextCouncilApprovalQueueNonceByDestinationChain::<T>::get(destination_chain);
		for queue_nonce in first_unresolved_nonce..=last_queued_nonce {
			let Some(entry) = CouncilApprovalQueueByDestinationChainAndNonce::<T>::get(
				destination_chain,
				queue_nonce,
			) else {
				continue;
			};
			if entry.approving_council_hash == council_hash {
				return;
			}
			if matches!(
				entry.target,
				CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(target_council_hash)
					if target_council_hash == council_hash
			) {
				return;
			}
		}

		GlobalIssuanceCouncilByHash::<T>::remove(council_hash);
	}

	pub(crate) fn next_council_approval_queue_nonce_for_account(
		destination_chain: SourceChain,
		account_id: &T::AccountId,
	) -> Option<CouncilApprovalQueueNonce> {
		let last_synced_nonce = GatewayStateBySourceChain::<T>::get(destination_chain)
			.map(|state| state.argon_approvals_nonce)
			.unwrap_or_default();
		let last_signed_nonce = CouncilApprovalCursorByDestinationChainAndAccountId::<T>::get(
			destination_chain,
			account_id,
		)?;

		Some(last_synced_nonce.max(last_signed_nonce).saturating_add(1))
	}

	pub(crate) fn reset_council_approval_cursor(
		destination_chain: SourceChain,
		after_nonce: CouncilApprovalQueueNonce,
		previous_council: Option<&GlobalIssuanceCouncil<T>>,
		council: &GlobalIssuanceCouncil<T>,
	) -> DispatchResult {
		if let Some(previous_council) = previous_council {
			for member in previous_council.members.values() {
				if council
					.members
					.values()
					.any(|current_member| current_member.account_id == member.account_id)
				{
					continue;
				}
				CouncilApprovalCursorByDestinationChainAndAccountId::<T>::remove(
					destination_chain,
					&member.account_id,
				);
			}
		}

		for member in council.members.values() {
			let next_cursor = CouncilApprovalCursorByDestinationChainAndAccountId::<T>::get(
				destination_chain,
				&member.account_id,
			)
			.map(|last_signed_nonce| last_signed_nonce.min(after_nonce))
			.unwrap_or(after_nonce);
			CouncilApprovalCursorByDestinationChainAndAccountId::<T>::insert(
				destination_chain,
				&member.account_id,
				next_cursor,
			);
		}
		Ok(())
	}

	pub(crate) fn rebase_unresolved_queue_entries(
		destination_chain: SourceChain,
		after_nonce: CouncilApprovalQueueNonce,
		approving_council_hash: H256,
	) -> DispatchResult {
		let last_queued_nonce =
			NextCouncilApprovalQueueNonceByDestinationChain::<T>::get(destination_chain);
		let first_rebased_nonce = after_nonce.saturating_add(1);

		if first_rebased_nonce > last_queued_nonce {
			return Ok(());
		}

		let mut rebased_entries = Vec::new();
		let mut dropped_rotation_hashes = Vec::new();
		for queue_nonce in first_rebased_nonce..=last_queued_nonce {
			let Some(entry) = CouncilApprovalQueueByDestinationChainAndNonce::<T>::get(
				destination_chain,
				queue_nonce,
			) else {
				continue;
			};
			let approving_council =
				GlobalIssuanceCouncilByHash::<T>::get(entry.approving_council_hash)
					.ok_or(Error::<T>::GlobalIssuanceCouncilNotFound)?;
			ensure!(
				!Self::global_issuance_council_has_quorum(&approving_council, &entry),
				Error::<T>::CannotForceSetQuorumApprovedQueueEntry,
			);
			if let CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(council_hash) =
				entry.target
			{
				dropped_rotation_hashes.push(council_hash);
				continue;
			}
			rebased_entries.push((queue_nonce, entry));
		}

		for queue_nonce in first_rebased_nonce..=last_queued_nonce {
			CouncilApprovalQueueByDestinationChainAndNonce::<T>::remove(
				destination_chain,
				queue_nonce,
			);
		}

		let mut previous_approval_hash =
			Self::previous_gateway_update_hash(destination_chain, first_rebased_nonce)?;
		let mut next_queue_nonce = first_rebased_nonce;
		let due_frame_id = Self::queue_entry_due_frame_id();

		for (old_queue_nonce, mut entry) in rebased_entries {
			match entry.target {
				CouncilApprovalTargetId::MintingAuthorityActivation(destination_signing_key) => {
					MintingAuthoritiesBySigner::<T>::try_mutate(
						destination_signing_key,
						|maybe_authority| -> DispatchResult {
							let authority = maybe_authority
								.as_mut()
								.ok_or(Error::<T>::MintingAuthorityNotFound)?;
							ensure!(
								authority.activation_approval_queue_nonce == old_queue_nonce,
								Error::<T>::MintingAuthorityMismatch,
							);
							authority.activation_approval_queue_nonce = next_queue_nonce;
							Ok(())
						},
					)?;
				},
				CouncilApprovalTargetId::MintingAuthorityDeactivation(destination_signing_key) => {
					MintingAuthoritiesBySigner::<T>::try_mutate(
						destination_signing_key,
						|maybe_authority| -> DispatchResult {
							let authority = maybe_authority
								.as_mut()
								.ok_or(Error::<T>::MintingAuthorityNotFound)?;
							ensure!(
								authority.deactivation_approval_queue_nonce ==
									Some(old_queue_nonce),
								Error::<T>::MintingAuthorityMismatch,
							);
							authority.deactivation_approval_queue_nonce = Some(next_queue_nonce);
							Ok(())
						},
					)?;
				},
				CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(_) =>
					unreachable!("pending council rotations are dropped before rebasing"),
			}

			entry.approving_council_hash = approving_council_hash;
			entry.due_frame_id = due_frame_id;
			entry.previous_approval_hash = previous_approval_hash;
			entry.approved_total_weight = T::Balance::default();
			entry.signatures = BoundedBTreeMap::new();
			entry.approval_hash = Self::hash_council_approval_queue_entry(
				destination_chain,
				next_queue_nonce,
				&entry,
			)?;
			previous_approval_hash = entry.approval_hash;

			CouncilApprovalQueueByDestinationChainAndNonce::<T>::insert(
				destination_chain,
				next_queue_nonce,
				entry.clone(),
			);
			next_queue_nonce = next_queue_nonce.saturating_add(1);
		}

		NextCouncilApprovalQueueNonceByDestinationChain::<T>::insert(
			destination_chain,
			next_queue_nonce.saturating_sub(1),
		);
		for council_hash in dropped_rotation_hashes {
			Self::prune_global_issuance_council_if_unreferenced(destination_chain, council_hash);
		}

		Ok(())
	}

	pub(crate) fn global_issuance_council_has_quorum(
		active_council: &GlobalIssuanceCouncil<T>,
		entry: &CouncilApprovalQueueEntry<T>,
	) -> bool {
		let total_weight = active_council.total_weight;
		if total_weight == T::Balance::default() {
			return false;
		}

		let approved_weight = entry.approved_total_weight;
		let unsigned_member_count =
			active_council.members.len().saturating_sub(entry.signatures.len());

		approved_weight.saturating_mul(100u128.into()) >= total_weight.saturating_mul(90u128.into()) ||
			(unsigned_member_count <= 2 &&
				approved_weight.saturating_mul(100u128.into()) >=
					total_weight.saturating_mul(80u128.into()))
	}

	pub(crate) fn queue_entry_due_frame_id() -> FrameId {
		T::CurrentFrameId::get().saturating_add(T::CouncilRotationFrames::get())
	}
}

#[cfg(test)]
mod tests {
	use crate::tests::*;

	#[test]
	fn transfer_out_quote_uses_initialized_floor_and_pending_rotation_rates() {
		new_test_ext().execute_with(|| {
			let council_account = account(38);
			let council_pair = council_signing_pair(53);
			let council_signing_key = council_signer(&council_pair);

			register_vault_operator(council_account.clone(), 12, 8_000);
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(council_account.clone()),
				SourceChain::Ethereum,
				council_signing_key,
				council_signer_registration_signature(&council_pair, &council_account),
			));

			LowestMicrogonsPerArgonot::set(Some(3 * argon_primitives::MICROGONS_PER_ARGON));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));

			LowestMicrogonsPerArgonot::set(Some(5 * argon_primitives::MICROGONS_PER_ARGON));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));

			assert_eq!(
				CurrentTransferOutMicrogonsPerArgonotByDestinationChain::<Test>::get(
					SourceChain::Ethereum,
				),
				Some(3 * argon_primitives::MICROGONS_PER_ARGON),
			);
			assert_eq!(
				PreviousTransferOutMicrogonsPerArgonotByDestinationChain::<Test>::get(
					SourceChain::Ethereum,
				),
				None,
			);
			assert_eq!(
				CrosschainTransfer::transfer_out_quote_microgons_per_argonot(SourceChain::Ethereum,),
				Ok(3 * argon_primitives::MICROGONS_PER_ARGON),
			);

			let active_council_hash =
				ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
					.expect("active council should be stored");
			let mut queued_rotation_council =
				GlobalIssuanceCouncilByHash::<Test>::get(active_council_hash)
					.expect("active council snapshot should be stored");
			queued_rotation_council.epoch_microgons_per_argonot =
				2 * argon_primitives::MICROGONS_PER_ARGON;
			let queued_rotation_hash = CrosschainTransfer::hash_global_issuance_council(
				&queued_rotation_council.members,
				queued_rotation_council.epoch_microgons_per_argonot,
			);
			GlobalIssuanceCouncilByHash::<Test>::insert(
				queued_rotation_hash,
				queued_rotation_council,
			);
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::insert(
				SourceChain::Ethereum,
				1,
				CouncilApprovalQueueEntry::<Test> {
					approving_council_hash: active_council_hash,
					target: CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(
						queued_rotation_hash,
					),
					target_payload_hash: H256::repeat_byte(0x41),
					due_frame_id: 1,
					previous_approval_hash: H256::zero(),
					approval_hash: H256::repeat_byte(0x42),
					approved_total_weight: 0,
					signatures: BoundedBTreeMap::new(),
				},
			);
			NextCouncilApprovalQueueNonceByDestinationChain::<Test>::insert(
				SourceChain::Ethereum,
				1,
			);

			assert_ok!(CrosschainTransfer::refresh_destination_chain_queue_tracking(
				SourceChain::Ethereum
			));
			assert_eq!(
				CrosschainTransfer::transfer_out_quote_microgons_per_argonot(SourceChain::Ethereum,),
				Ok(2 * argon_primitives::MICROGONS_PER_ARGON),
			);
		});
	}

	#[test]
	fn pending_rotation_entries_drive_activation_quotes_and_are_dropped_on_force_set() {
		new_test_ext().execute_with(|| {
			let council_account = account(39);
			let authority_account = account(40);
			let council_pair = council_signing_pair(54);
			let authority_pair = council_signing_pair(55);
			let council_signing_key = council_signer(&council_pair);
			let authority_signing_key = council_signer(&authority_pair);

			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));
			assert_ok!(CrosschainTransfer::set_minting_authority_activation_repayment_pricing(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				MintingAuthorityActivationRepaymentPricing::<Test> {
					activation_gas_cost: 100_000,
					signature_gas_cost: 50_000,
					estimated_wei_per_gas: 1_000_000_000,
					estimated_microgons_per_eth: 1_000_000,
				},
			));
			register_vault_operator(council_account.clone(), 16, 8_000);
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(council_account.clone()),
				SourceChain::Ethereum,
				council_signing_key,
				council_signer_registration_signature(&council_pair, &council_account),
			));
			assert_ok!(CrosschainTransfer::set_minimum_minting_authority_value(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				4_000,
			));

			LowestMicrogonsPerArgonot::set(Some(3 * argon_primitives::MICROGONS_PER_ARGON));
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
					.expect("active council should be stored");
			let mut queued_rotation_council =
				GlobalIssuanceCouncilByHash::<Test>::get(active_council_hash)
					.expect("active council snapshot should be stored");
			queued_rotation_council.epoch_microgons_per_argonot =
				2 * argon_primitives::MICROGONS_PER_ARGON;
			let queued_rotation_hash = CrosschainTransfer::hash_global_issuance_council(
				&queued_rotation_council.members,
				queued_rotation_council.epoch_microgons_per_argonot,
			);
			GlobalIssuanceCouncilByHash::<Test>::insert(
				queued_rotation_hash,
				queued_rotation_council,
			);
			CouncilApprovalQueueByDestinationChainAndNonce::<Test>::insert(
				SourceChain::Ethereum,
				1,
				CouncilApprovalQueueEntry::<Test> {
					approving_council_hash: active_council_hash,
					target: CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(
						queued_rotation_hash,
					),
					target_payload_hash: H256::repeat_byte(0x51),
					due_frame_id: 1,
					previous_approval_hash: H256::zero(),
					approval_hash: H256::repeat_byte(0x52),
					approved_total_weight: 0,
					signatures: BoundedBTreeMap::new(),
				},
			);
			NextCouncilApprovalQueueNonceByDestinationChain::<Test>::insert(
				SourceChain::Ethereum,
				1,
			);

			assert_ok!(CrosschainTransfer::refresh_destination_chain_queue_tracking(
				SourceChain::Ethereum
			));

			register_vault_operator(authority_account.clone(), 17, 6_000);
			assert_ok!(Balances::mint_into(&authority_account, 10_000));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(authority_account.clone()),
				SourceChain::Ethereum,
				authority_signing_key,
				minting_authority_registration_signature(&authority_pair, &authority_account),
				4_000,
				0,
			));

			let authority = MintingAuthoritiesBySigner::<Test>::get(authority_signing_key)
				.expect("authority should be queued");
			assert_eq!(authority.activation_approval_queue_nonce, 2);
			assert_eq!(authority.activation_signature_repayment_quote, 100);
			assert_eq!(
				CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
					SourceChain::Ethereum,
					2,
				)
				.expect("activation queue entry should exist")
				.approving_council_hash,
				queued_rotation_hash,
			);

			LowestMicrogonsPerArgonot::set(Some(5 * argon_primitives::MICROGONS_PER_ARGON));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));

			let replacement_council_hash =
				ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
					.expect("replacement council should be stored");
			assert_eq!(
				MintingAuthoritiesBySigner::<Test>::get(authority_signing_key)
					.expect("authority should stay queued")
					.activation_approval_queue_nonce,
				1,
			);
			assert_eq!(
				CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
					SourceChain::Ethereum,
					1,
				)
				.expect("activation should be rebased into the first unresolved slot")
				.approving_council_hash,
				replacement_council_hash,
			);
			assert!(matches!(
				CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
					SourceChain::Ethereum,
					1,
				)
				.expect("rebased activation should stay queued")
				.target,
				CouncilApprovalTargetId::MintingAuthorityActivation(signing_key)
					if signing_key == authority_signing_key
			));
			assert!(CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
				SourceChain::Ethereum,
				2,
			)
			.is_none(),);
			assert_eq!(
				NextCouncilApprovalQueueNonceByDestinationChain::<Test>::get(SourceChain::Ethereum,),
				1,
			);
			assert!(GlobalIssuanceCouncilByHash::<Test>::get(queued_rotation_hash).is_none());
		});
	}

	#[test]
	fn force_set_global_issuance_council_rebases_unresolved_queue_entries() {
		new_test_ext().execute_with(|| {
			let original_council_account = account(44);
			let replacement_council_account = account(45);
			let original_council_pair = council_signing_pair(71);
			let replacement_council_pair = council_signing_pair(72);
			let first_minting_authority_pair = council_signing_pair(73);
			let second_minting_authority_pair = council_signing_pair(74);
			let original_council_signer = council_signer(&original_council_pair);
			let replacement_council_signer = council_signer(&replacement_council_pair);
			let first_signing_key = council_signer(&first_minting_authority_pair);
			let second_signing_key = council_signer(&second_minting_authority_pair);

			register_vault_operator(original_council_account.clone(), 15, 12_000);
			register_vault_operator(replacement_council_account.clone(), 16, 11_000);
			assert_ok!(Balances::mint_into(&original_council_account, 20_000));
			assert_ok!(Ownership::mint_into(&original_council_account, 2_000));
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));
			assert_ok!(CrosschainTransfer::set_minting_authority_activation_repayment_pricing(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				MintingAuthorityActivationRepaymentPricing::<Test> {
					activation_gas_cost: 100_000,
					signature_gas_cost: 50_000,
					estimated_wei_per_gas: 1_000_000_000,
					estimated_microgons_per_eth: 1_000_000,
				},
			));
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(original_council_account.clone()),
				SourceChain::Ethereum,
				original_council_signer,
				council_signer_registration_signature(
					&original_council_pair,
					&original_council_account,
				),
			));
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(replacement_council_account.clone()),
				SourceChain::Ethereum,
				replacement_council_signer,
				council_signer_registration_signature(
					&replacement_council_pair,
					&replacement_council_account,
				),
			));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![original_council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));
			assert_ok!(CrosschainTransfer::set_minimum_minting_authority_value(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				3_000,
			));
			assert_ok!(set_committed_argonots(original_council_account.clone(), 500));

			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(original_council_account.clone()),
				SourceChain::Ethereum,
				first_signing_key,
				minting_authority_registration_signature(
					&first_minting_authority_pair,
					&original_council_account,
				),
				4_000,
				100,
			));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(original_council_account.clone()),
				SourceChain::Ethereum,
				second_signing_key,
				minting_authority_registration_signature(
					&second_minting_authority_pair,
					&original_council_account,
				),
				3_000,
				50,
			));
			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(original_council_account.clone()),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&original_council_pair, 1,)],
			));

			let first_queue_entry_before =
				CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
					SourceChain::Ethereum,
					1,
				)
				.expect("first queue entry should exist");
			let second_queue_entry_before =
				CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
					SourceChain::Ethereum,
					2,
				)
				.expect("second queue entry should exist");

			assert_noop!(
				CrosschainTransfer::force_set_global_issuance_council(
					RuntimeOrigin::root(),
					SourceChain::Ethereum,
					0,
					vec![replacement_council_account.clone()]
						.try_into()
						.expect("single council member stays within limit"),
				),
				Error::<Test>::CannotForceSetQuorumApprovedQueueEntry,
			);

			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				1,
				vec![replacement_council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));

			let active_council =
				ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
					.expect("replacement council should be active");
			let first_queue_entry_after =
				CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
					SourceChain::Ethereum,
					1,
				)
				.expect("first queue entry should stay queued");
			let second_queue_entry_after =
				CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
					SourceChain::Ethereum,
					2,
				)
				.expect("second queue entry should stay queued");

			assert_eq!(
				first_queue_entry_after.approving_council_hash,
				first_queue_entry_before.approving_council_hash,
			);
			assert_eq!(second_queue_entry_after.approving_council_hash, active_council);
			assert_eq!(
				first_queue_entry_after.previous_approval_hash,
				first_queue_entry_before.previous_approval_hash,
			);
			assert_eq!(
				second_queue_entry_after.previous_approval_hash,
				first_queue_entry_after.approval_hash,
			);
			assert_eq!(
				first_queue_entry_after.approved_total_weight,
				first_queue_entry_before.approved_total_weight,
			);
			assert_eq!(second_queue_entry_after.approved_total_weight, 0);
			assert_eq!(
				first_queue_entry_after.signatures.len(),
				first_queue_entry_before.signatures.len(),
			);
			assert!(second_queue_entry_after.signatures.is_empty());
			assert_eq!(
				first_queue_entry_after.approval_hash,
				first_queue_entry_before.approval_hash,
			);
			assert_ne!(
				second_queue_entry_after.approval_hash,
				second_queue_entry_before.approval_hash,
			);
		});
	}

	#[test]
	fn approve_queue_entries_validates_signature_and_records_council_quorum() {
		new_test_ext().execute_with(|| {
			let owner_vault_operator = account(30);
			let council_pair = council_signing_pair(1);
			let wrong_pair = council_signing_pair(9);
			let council_address = council_signer(&council_pair);
			let minting_authority_pair = council_signing_pair(2);
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
			assert_noop!(
				CrosschainTransfer::approve_queue_entries(
					RuntimeOrigin::signed(owner_vault_operator.clone()),
					SourceChain::Ethereum,
					bounded_vec![minting_authority_approval_signature(&wrong_pair, 1)],
				),
				Error::<Test>::InvalidCouncilApprovalSignature,
			);
			let result = CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&council_pair, 1)],
			);
			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));

			let queue_entry = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
				SourceChain::Ethereum,
				1,
			)
			.expect("queue entry should stay available");
			assert_eq!(queue_entry.approved_total_weight, 10_000);
			assert_eq!(queue_entry.signatures.len(), 1);
			assert!(queue_entry.signatures.contains_key(&council_address));
			assert_eq!(
				CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
					SourceChain::Ethereum,
					owner_vault_operator,
				),
				Some(1),
			);
		});
	}

	#[test]
	fn approve_queue_entries_keeps_non_signers_blocked_until_gateway_sync_advances() {
		new_test_ext().execute_with(|| {
			let first_council_account = account(31);
			let second_council_account = account(32);
			let third_council_account = account(33);
			let first_council_pair = council_signing_pair(11);
			let second_council_pair = council_signing_pair(12);
			let third_council_pair = council_signing_pair(13);
			let minting_authority_pair = council_signing_pair(14);
			let first_council_signer = council_signer(&first_council_pair);
			let second_council_signer = council_signer(&second_council_pair);
			let third_council_signer = council_signer(&third_council_pair);
			let signing_key = council_signer(&minting_authority_pair);

			register_vault_operator(first_council_account.clone(), 21, 50);
			register_vault_operator(second_council_account.clone(), 22, 30);
			register_vault_operator(third_council_account.clone(), 23, 20);
			assert_ok!(Balances::mint_into(&first_council_account, 10_000));
			assert_ok!(Ownership::mint_into(&first_council_account, 900));
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));
			assert_ok!(CrosschainTransfer::set_minting_authority_activation_repayment_pricing(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				MintingAuthorityActivationRepaymentPricing::<Test> {
					activation_gas_cost: 100_000,
					signature_gas_cost: 50_000,
					estimated_wei_per_gas: 1_000_000_000,
					estimated_microgons_per_eth: 1_000_000,
				},
			));
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(first_council_account.clone()),
				SourceChain::Ethereum,
				first_council_signer,
				council_signer_registration_signature(&first_council_pair, &first_council_account),
			));
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(second_council_account.clone()),
				SourceChain::Ethereum,
				second_council_signer,
				council_signer_registration_signature(
					&second_council_pair,
					&second_council_account,
				),
			));
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(third_council_account.clone()),
				SourceChain::Ethereum,
				third_council_signer,
				council_signer_registration_signature(&third_council_pair, &third_council_account),
			));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![
					first_council_account.clone(),
					second_council_account.clone(),
					third_council_account.clone(),
				]
				.try_into()
				.expect("three council members stay within limit"),
			));
			set_active_bond_amount(21, first_council_account.clone(), 10_000);
			assert_ok!(set_committed_argonots(first_council_account.clone(), 500));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(first_council_account.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&first_council_account,
				),
				10_000,
				300,
			));

			let queue_entry = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
				SourceChain::Ethereum,
				1,
			)
			.expect("queue entry should exist");
			assert!(
				!<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
					&first_council_account,
				)
			);
			CurrentFrameId::set(queue_entry.due_frame_id);
			assert!(
				<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
					&first_council_account,
				)
			);
			assert!(
				<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
					&second_council_account,
				)
			);
			assert!(
				<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
					&third_council_account,
				)
			);

			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(first_council_account.clone()),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&first_council_pair, 1)],
			));
			assert!(
				!<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
					&first_council_account,
				)
			);
			assert!(
				<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
					&second_council_account,
				)
			);
			assert!(
				<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
					&third_council_account,
				)
			);

			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(second_council_account.clone()),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&second_council_pair, 1)],
			));
			assert!(
				!<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
					&second_council_account,
				)
			);
			assert!(
				<CrosschainTransfer as CollectBlockerProvider<TestAccountId>>::has_overdue_collect_blocker(
					&third_council_account,
				)
			);
		});
	}

	#[test]
	fn approve_queue_entries_applies_contiguous_batch_fee_free() {
		new_test_ext().execute_with(|| {
			let owner_vault_operator = account(34);
			let council_pair = council_signing_pair(15);
			let council_address = council_signer(&council_pair);
			let first_minting_authority_pair = council_signing_pair(16);
			let second_minting_authority_pair = council_signing_pair(17);
			let first_signing_key = council_signer(&first_minting_authority_pair);
			let second_signing_key = council_signer(&second_minting_authority_pair);

			register_vault_operator(owner_vault_operator.clone(), 24, 20_000);
			assert_ok!(Balances::mint_into(&owner_vault_operator, 20_000));
			assert_ok!(Ownership::mint_into(&owner_vault_operator, 1_000));
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));
			assert_ok!(CrosschainTransfer::set_minting_authority_activation_repayment_pricing(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				MintingAuthorityActivationRepaymentPricing::<Test> {
					activation_gas_cost: 100_000,
					signature_gas_cost: 50_000,
					estimated_wei_per_gas: 1_000_000_000,
					estimated_microgons_per_eth: 1_000_000,
				},
			));
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				council_address,
				council_signer_registration_signature(&council_pair, &owner_vault_operator),
			));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![owner_vault_operator.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));
			assert_ok!(set_committed_argonots(owner_vault_operator.clone(), 600));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				first_signing_key,
				minting_authority_registration_signature(
					&first_minting_authority_pair,
					&owner_vault_operator,
				),
				10_000,
				200,
			));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(owner_vault_operator.clone()),
				SourceChain::Ethereum,
				second_signing_key,
				minting_authority_registration_signature(
					&second_minting_authority_pair,
					&owner_vault_operator,
				),
				10_000,
				100,
			));

			let result = CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(owner_vault_operator),
				SourceChain::Ethereum,
				bounded_vec![
					minting_authority_approval_signature(&council_pair, 1),
					minting_authority_approval_signature(&council_pair, 2)
				],
			);
			assert!(matches!(result, Ok(post_info) if post_info.pays_fee == Pays::No));

			let first_queue_entry = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
				SourceChain::Ethereum,
				1,
			)
			.expect("first queue entry should stay available");
			let second_queue_entry = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
				SourceChain::Ethereum,
				2,
			)
			.expect("second queue entry should stay available");
			assert_eq!(first_queue_entry.approved_total_weight, 20_000);
			assert_eq!(second_queue_entry.approved_total_weight, 20_000);
			assert_eq!(first_queue_entry.signatures.len(), 1);
			assert_eq!(second_queue_entry.signatures.len(), 1);
			assert_eq!(
				CouncilApprovalCursorByDestinationChainAndAccountId::<Test>::get(
					SourceChain::Ethereum,
					account(34),
				),
				Some(2),
			);
		});
	}

	#[test]
	fn force_set_global_issuance_council_rebases_queued_approving_council() {
		new_test_ext().execute_with(|| {
			let queued_council_account = account(42);
			let replacement_council_account = account(43);
			let queued_council_pair = council_signing_pair(61);
			let replacement_council_pair = council_signing_pair(62);
			let minting_authority_pair = council_signing_pair(63);
			let queued_council_signer = council_signer(&queued_council_pair);
			let replacement_council_signer = council_signer(&replacement_council_pair);
			let signing_key = council_signer(&minting_authority_pair);

			register_vault_operator(queued_council_account.clone(), 13, 8_000);
			register_vault_operator(replacement_council_account.clone(), 14, 7_000);
			assert_ok!(Balances::mint_into(&queued_council_account, 10_000));
			assert_ok!(Ownership::mint_into(&queued_council_account, 500));
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));
			assert_ok!(CrosschainTransfer::set_minting_authority_activation_repayment_pricing(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				MintingAuthorityActivationRepaymentPricing::<Test> {
					activation_gas_cost: 100_000,
					signature_gas_cost: 50_000,
					estimated_wei_per_gas: 1_000_000_000,
					estimated_microgons_per_eth: 1_000_000,
				},
			));
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(queued_council_account.clone()),
				SourceChain::Ethereum,
				queued_council_signer,
				council_signer_registration_signature(
					&queued_council_pair,
					&queued_council_account,
				),
			));
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(replacement_council_account.clone()),
				SourceChain::Ethereum,
				replacement_council_signer,
				council_signer_registration_signature(
					&replacement_council_pair,
					&replacement_council_account,
				),
			));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![queued_council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));
			assert_ok!(CrosschainTransfer::set_minimum_minting_authority_value(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				4_000,
			));
			assert_ok!(set_committed_argonots(queued_council_account.clone(), 300));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(queued_council_account.clone()),
				SourceChain::Ethereum,
				signing_key,
				minting_authority_registration_signature(
					&minting_authority_pair,
					&queued_council_account,
				),
				4_000,
				100,
			));

			let queued_approving_council_hash = CouncilApprovalQueueByDestinationChainAndNonce::<
				Test,
			>::get(SourceChain::Ethereum, 1)
			.expect("queued approval should exist")
			.approving_council_hash;
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![replacement_council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));
			assert_ne!(
				ActiveGlobalIssuanceCouncilByDestinationChain::<Test>::get(SourceChain::Ethereum)
					.expect("replacement council should be active"),
				queued_approving_council_hash,
			);

			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(replacement_council_account),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&replacement_council_pair, 1,)],
			));
			assert_noop!(
				CrosschainTransfer::approve_queue_entries(
					RuntimeOrigin::signed(queued_council_account),
					SourceChain::Ethereum,
					bounded_vec![minting_authority_approval_signature(&queued_council_pair, 1,)],
				),
				Error::<Test>::GlobalIssuanceCouncilMemberNotFound,
			);
		});
	}

	#[test]
	fn force_set_global_issuance_council_preserves_approved_front_queue_entries() {
		new_test_ext().execute_with(|| {
			let original_council_account = account(144);
			let replacement_council_account = account(145);
			let original_council_pair = council_signing_pair(171);
			let replacement_council_pair = council_signing_pair(172);
			let first_minting_authority_pair = council_signing_pair(173);
			let second_minting_authority_pair = council_signing_pair(174);
			let original_council_signer = council_signer(&original_council_pair);
			let replacement_council_signer = council_signer(&replacement_council_pair);
			let first_signing_key = council_signer(&first_minting_authority_pair);
			let second_signing_key = council_signer(&second_minting_authority_pair);

			register_vault_operator(original_council_account.clone(), 115, 12_000);
			register_vault_operator(replacement_council_account.clone(), 116, 11_000);
			assert_ok!(Balances::mint_into(&original_council_account, 20_000));
			assert_ok!(Ownership::mint_into(&original_council_account, 2_000));
			assert_ok!(CrosschainTransfer::set_chain_config(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				chain_config(),
			));
			assert_ok!(CrosschainTransfer::set_minting_authority_activation_repayment_pricing(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				MintingAuthorityActivationRepaymentPricing::<Test> {
					activation_gas_cost: 100_000,
					signature_gas_cost: 50_000,
					estimated_wei_per_gas: 1_000_000_000,
					estimated_microgons_per_eth: 1_000_000,
				},
			));
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(original_council_account.clone()),
				SourceChain::Ethereum,
				original_council_signer,
				council_signer_registration_signature(
					&original_council_pair,
					&original_council_account,
				),
			));
			assert_ok!(CrosschainTransfer::register_council_signer(
				RuntimeOrigin::signed(replacement_council_account.clone()),
				SourceChain::Ethereum,
				replacement_council_signer,
				council_signer_registration_signature(
					&replacement_council_pair,
					&replacement_council_account,
				),
			));
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				0,
				vec![original_council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));
			assert_ok!(CrosschainTransfer::set_minimum_minting_authority_value(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				3_000,
			));
			assert_ok!(set_committed_argonots(original_council_account.clone(), 500));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(original_council_account.clone()),
				SourceChain::Ethereum,
				first_signing_key,
				minting_authority_registration_signature(
					&first_minting_authority_pair,
					&original_council_account,
				),
				4_000,
				100,
			));
			assert_ok!(CrosschainTransfer::register_minting_authority(
				RuntimeOrigin::signed(original_council_account.clone()),
				SourceChain::Ethereum,
				second_signing_key,
				minting_authority_registration_signature(
					&second_minting_authority_pair,
					&original_council_account,
				),
				3_000,
				50,
			));
			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(original_council_account.clone()),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&original_council_pair, 1)],
			));

			let first_queue_entry_before_force = CouncilApprovalQueueByDestinationChainAndNonce::<
				Test,
			>::get(SourceChain::Ethereum, 1)
			.expect("front queue entry should stay queued");
			assert_ok!(CrosschainTransfer::force_set_global_issuance_council(
				RuntimeOrigin::root(),
				SourceChain::Ethereum,
				1,
				vec![replacement_council_account.clone()]
					.try_into()
					.expect("single council member stays within limit"),
			));

			let first_queue_entry = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
				SourceChain::Ethereum,
				1,
			)
			.expect("front queue entry should stay queued");
			let second_queue_entry = CouncilApprovalQueueByDestinationChainAndNonce::<Test>::get(
				SourceChain::Ethereum,
				2,
			)
			.expect("rebased queue entry should stay queued");
			assert_ne!(
				first_queue_entry.approving_council_hash,
				second_queue_entry.approving_council_hash,
			);
			assert_eq!(
				first_queue_entry.approving_council_hash,
				first_queue_entry_before_force.approving_council_hash,
			);
			assert_eq!(
				first_queue_entry.previous_approval_hash,
				first_queue_entry_before_force.previous_approval_hash,
			);
			assert_eq!(
				first_queue_entry.approval_hash,
				first_queue_entry_before_force.approval_hash,
			);
			assert_eq!(
				first_queue_entry.approved_total_weight,
				first_queue_entry_before_force.approved_total_weight,
			);
			assert_eq!(first_queue_entry.signatures, first_queue_entry_before_force.signatures,);
			assert_noop!(
				CrosschainTransfer::approve_queue_entries(
					RuntimeOrigin::signed(original_council_account.clone()),
					SourceChain::Ethereum,
					bounded_vec![minting_authority_approval_signature(&original_council_pair, 2,)],
				),
				Error::<Test>::GlobalIssuanceCouncilMemberNotFound,
			);
			assert_noop!(
				CrosschainTransfer::approve_queue_entries(
					RuntimeOrigin::signed(original_council_account),
					SourceChain::Ethereum,
					bounded_vec![minting_authority_approval_signature(&original_council_pair, 1,)],
				),
				Error::<Test>::GlobalIssuanceCouncilMemberNotFound,
			);
			assert_ok!(CrosschainTransfer::approve_queue_entries(
				RuntimeOrigin::signed(replacement_council_account),
				SourceChain::Ethereum,
				bounded_vec![minting_authority_approval_signature(&replacement_council_pair, 2,)],
			));
		});
	}
}
