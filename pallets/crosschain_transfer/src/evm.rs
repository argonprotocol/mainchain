use alloc::{format, vec::Vec};
use alloy_primitives::{keccak256, Address as AlloyAddress, B256, U256};
use alloy_sol_types::SolEvent;
use argon_ethereum_contracts::minting_gateway::{
	self as ethereum_contracts, GatewayActivityState as ContractGatewayActivityState,
	GlobalIssuanceCouncilRotated as ContractGlobalIssuanceCouncilRotated,
	MintingAuthorityActivated as ContractMintingAuthorityActivated,
	MintingAuthorityDeactivated as ContractMintingAuthorityDeactivated,
	TransferOutOfArgonCanceled as ContractTransferOutOfArgonCanceled,
	TransferOutOfArgonFinalized as ContractTransferOutOfArgonFinalized,
	TransferToArgonStarted as ContractTransferToArgonStarted,
};
use argon_primitives::EthereumLog;
use codec::Encode;
use k256::{elliptic_curve::sec1::ToEncodedPoint, PublicKey as Secp256k1PublicKey};
use polkadot_sdk::{
	frame_support::ensure,
	sp_core::ecdsa::{KeccakPublic, KeccakSignature},
	sp_runtime::{BoundedBTreeMap, DispatchError},
};

use crate::{
	gateway_activity::{
		DecodedGatewayActivity, GatewayActivityBlock, GatewayMintingAuthorityCollateral,
	},
	ChainConfig, ChainConfigBySourceChain, Config, CouncilApprovalQueueEntry,
	CouncilApprovalQueueNonce, CouncilApprovalTargetId, Error, GatewayActivityNonce,
	GatewayActivityStorageProof, GatewayState, GlobalIssuanceCouncil, GlobalIssuanceCouncilMember,
	Pallet, SourceChain, H160, H256,
};

impl<T: Config> DecodedGatewayActivity<T> {
	pub(crate) fn leaf_hash(&self, chain_config: &ChainConfig) -> H256 {
		let ChainConfig::Evm { chain_id, gateway, argon_token, argonot_token } = chain_config;
		let gateway = AlloyAddress::from_slice(gateway.as_bytes());
		let gateway_state = ContractGatewayActivityState {
			gatewayActivityNonce: self.gateway_state().gateway_activity_nonce,
			argonApprovalsNonce: self.gateway_state().argon_approvals_nonce,
			argonCirculation: self.gateway_state().argon_circulation.into(),
			argonotCirculation: self.gateway_state().argonot_circulation.into(),
		};

		let leaf_hash = match self {
			Self::TransferToArgon { from, token, to, amount, .. } =>
				ethereum_contracts::hash_transfer_to_argon_started_activity(
					*chain_id,
					gateway,
					AlloyAddress::from_slice(from.as_bytes()),
					AlloyAddress::from_slice(token.as_bytes()),
					(*amount).into(),
					B256::from(*to),
					gateway_state,
				),
			Self::MintingAuthorityActivated {
				destination_signing_key,
				microgon_collateral,
				micronot_collateral,
				coactivation_count,
				shared_signature_count,
				relayer_argon_account_id,
				approval_hash,
				..
			} => ethereum_contracts::hash_minting_authority_activated_activity(
				*chain_id,
				gateway,
				AlloyAddress::from_slice(destination_signing_key.as_bytes()),
				(*microgon_collateral).into(),
				(*micronot_collateral).into(),
				*coactivation_count,
				*shared_signature_count,
				B256::from(approval_hash.0),
				B256::from(*relayer_argon_account_id),
				gateway_state,
			),
			Self::GlobalIssuanceCouncilRotated {
				council_hash,
				approval_hash,
				relayer_argon_account_id,
				..
			} => ethereum_contracts::hash_global_issuance_council_rotated_activity(
				*chain_id,
				gateway,
				B256::from(council_hash.0),
				B256::from(approval_hash.0),
				B256::from(*relayer_argon_account_id),
				gateway_state,
			),
			Self::MintingAuthorityDeactivated {
				destination_signing_key,
				microgon_collateral,
				micronot_collateral,
				approval_hash,
				relayer_argon_account_id,
				..
			} => ethereum_contracts::hash_minting_authority_deactivated_activity(
				*chain_id,
				gateway,
				AlloyAddress::from_slice(destination_signing_key.as_bytes()),
				(*microgon_collateral).into(),
				(*micronot_collateral).into(),
				B256::from(approval_hash.0),
				B256::from(*relayer_argon_account_id),
				gateway_state,
			),
			Self::TransferOutOfArgonFinalized {
				transfer_id,
				asset,
				amount,
				minting_authority_collateral,
				..
			} => {
				let token = match asset {
					crate::AssetKind::Argon => AlloyAddress::from_slice(argon_token.as_bytes()),
					crate::AssetKind::Argonot => AlloyAddress::from_slice(argonot_token.as_bytes()),
				};

				ethereum_contracts::hash_transfer_out_of_argon_finalized_activity(
					*chain_id,
					gateway,
					B256::from(transfer_id.0),
					token,
					(*amount).into(),
					minting_authority_collateral
						.iter()
						.map(|row| ethereum_contracts::MintingAuthorityCollateral {
							signingKey: AlloyAddress::from_slice(row.signing_key.as_bytes()),
							microgonCollateral: row.microgon_collateral.into(),
							micronotCollateral: row.micronot_collateral.into(),
						})
						.collect(),
					gateway_state,
				)
			},
			Self::TransferOutOfArgonCanceled { transfer_id, .. } =>
				ethereum_contracts::hash_transfer_out_of_argon_canceled_activity(
					*chain_id,
					gateway,
					B256::from(transfer_id.0),
					gateway_state,
				),
		};

		H256::from_slice(leaf_hash.as_slice())
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn hash_global_issuance_council(
		members: &BoundedBTreeMap<H160, GlobalIssuanceCouncilMember<T>, T::MaxCouncilMembers>,
		epoch_microgons_per_argonot: T::Balance,
	) -> H256 {
		let epoch_microgons_per_argonot: u128 = epoch_microgons_per_argonot.into();
		let signers: Vec<AlloyAddress> = members
			.keys()
			.map(|signer| AlloyAddress::from_slice(signer.as_bytes()))
			.collect();
		let weights: Vec<U256> = members
			.values()
			.map(|member| {
				let weight: u128 = member.weight.into();
				U256::from(weight)
			})
			.collect();

		H256::from_slice(
			ethereum_contracts::hash_global_issuance_council(
				signers,
				weights,
				epoch_microgons_per_argonot,
			)
			.as_slice(),
		)
	}

	pub(crate) fn hash_rotate_global_issuance_council(
		destination_chain: SourceChain,
		council: &GlobalIssuanceCouncil<T>,
	) -> Result<H256, DispatchError> {
		let (chain_id, gateway) = Self::evm_gateway_signature_domain(destination_chain)?;
		let council_hash = Self::hash_global_issuance_council(
			&council.members,
			council.epoch_microgons_per_argonot,
		);

		Ok(H256::from_slice(
			ethereum_contracts::hash_rotate_global_issuance_council(
				chain_id,
				AlloyAddress::from_slice(gateway.as_bytes()),
				B256::from(council_hash.0),
				council.epoch_microgons_per_argonot.into(),
			)
			.as_slice(),
		))
	}

	pub(crate) fn hash_activate_minting_authority(
		destination_chain: SourceChain,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
		destination_signing_key: H160,
	) -> Result<H256, DispatchError> {
		let (chain_id, gateway) = Self::evm_gateway_signature_domain(destination_chain)?;
		let microgon_collateral: u128 = microgon_collateral.into();
		let micronot_collateral: u128 = micronot_collateral.into();

		Ok(H256::from_slice(
			ethereum_contracts::hash_activate_minting_authority(
				chain_id,
				AlloyAddress::from_slice(gateway.as_bytes()),
				microgon_collateral,
				micronot_collateral,
				AlloyAddress::from_slice(destination_signing_key.as_bytes()),
			)
			.as_slice(),
		))
	}

	pub(crate) fn hash_deactivate_minting_authority_target(destination_signing_key: H160) -> H256 {
		H256::from_slice(
			ethereum_contracts::hash_minting_authority_deactivation_target(
				AlloyAddress::from_slice(destination_signing_key.as_bytes()),
			)
			.as_slice(),
		)
	}

	pub(crate) fn evm_gateway_signature_domain(
		destination_chain: SourceChain,
	) -> Result<(u64, H160), DispatchError> {
		let config = ChainConfigBySourceChain::<T>::get(destination_chain)
			.ok_or(Error::<T>::InvalidChainConfig)?;

		match config {
			ChainConfig::Evm { chain_id, gateway, .. } => Ok((chain_id, gateway)),
		}
	}

	pub(crate) fn hash_council_approval_queue_entry(
		destination_chain: SourceChain,
		queue_nonce: CouncilApprovalQueueNonce,
		entry: &CouncilApprovalQueueEntry<T>,
	) -> Result<H256, DispatchError> {
		let (chain_id, gateway) = Self::evm_gateway_signature_domain(destination_chain)?;

		match entry.target {
			CouncilApprovalTargetId::GlobalIssuanceCouncilRotation(council_hash) =>
				Ok(H256::from_slice(
					ethereum_contracts::hash_gateway_update_approval(
						chain_id,
						AlloyAddress::from_slice(gateway.as_bytes()),
						queue_nonce,
						B256::from(entry.approving_council_hash.0),
						0u8,
						B256::from(council_hash.0),
						B256::from(entry.target_payload_hash.0),
						B256::from(entry.previous_approval_hash.0),
					)
					.as_slice(),
				)),
			CouncilApprovalTargetId::MintingAuthorityActivation(destination_signing_key) =>
				Ok(H256::from_slice(
					ethereum_contracts::hash_gateway_update_approval(
						chain_id,
						AlloyAddress::from_slice(gateway.as_bytes()),
						queue_nonce,
						B256::from(entry.approving_council_hash.0),
						1u8,
						ethereum_contracts::destination_signing_key_target_id(
							AlloyAddress::from_slice(destination_signing_key.as_bytes()),
						),
						B256::from(entry.target_payload_hash.0),
						B256::from(entry.previous_approval_hash.0),
					)
					.as_slice(),
				)),
			CouncilApprovalTargetId::MintingAuthorityDeactivation(destination_signing_key) =>
				Ok(H256::from_slice(
					ethereum_contracts::hash_gateway_update_approval(
						chain_id,
						AlloyAddress::from_slice(gateway.as_bytes()),
						queue_nonce,
						B256::from(entry.approving_council_hash.0),
						2u8,
						ethereum_contracts::destination_signing_key_target_id(
							AlloyAddress::from_slice(destination_signing_key.as_bytes()),
						),
						B256::from(entry.target_payload_hash.0),
						B256::from(entry.previous_approval_hash.0),
					)
					.as_slice(),
				)),
		}
	}

	pub(crate) fn council_signer_registration_message(
		destination_chain: SourceChain,
		account_id: &T::AccountId,
	) -> Vec<u8> {
		(b"argon/council-signer/v2".as_slice(), destination_chain, account_id).encode()
	}

	pub(crate) fn minting_authority_signer_registration_message(
		destination_chain: SourceChain,
		account_id: &T::AccountId,
	) -> Vec<u8> {
		(b"argon/minting-authority-signer/v2".as_slice(), destination_chain, account_id).encode()
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub(crate) fn recover_evm_personal_signer(
		_message: &[u8],
		signature: &KeccakSignature,
	) -> Option<H160> {
		Some(H160::from_slice(&signature[..20]))
	}

	#[cfg(not(feature = "runtime-benchmarks"))]
	pub(crate) fn recover_evm_personal_signer(
		message: &[u8],
		signature: &KeccakSignature,
	) -> Option<H160> {
		let public_key = signature.recover(Self::evm_personal_message(message))?;
		Self::evm_address_from_public_key(&public_key)
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub(crate) fn recover_evm_message_signer(
		_approval_hash: H256,
		signature: &KeccakSignature,
	) -> Option<H160> {
		Some(H160::from_slice(&signature[..20]))
	}

	#[cfg(not(feature = "runtime-benchmarks"))]
	pub(crate) fn recover_evm_message_signer(
		approval_hash: H256,
		signature: &KeccakSignature,
	) -> Option<H160> {
		let public_key = signature.recover(Self::evm_signed_message(approval_hash))?;
		Self::evm_address_from_public_key(&public_key)
	}

	#[cfg_attr(feature = "runtime-benchmarks", allow(dead_code))]
	pub(crate) fn evm_personal_message(message: &[u8]) -> Vec<u8> {
		let mut signable_message =
			format!("\x19Ethereum Signed Message:\n{}", message.len()).into_bytes();
		signable_message.extend_from_slice(message);
		signable_message
	}

	#[cfg_attr(feature = "runtime-benchmarks", allow(dead_code))]
	pub(crate) fn evm_signed_message(message_hash: H256) -> Vec<u8> {
		Self::evm_personal_message(message_hash.as_bytes())
	}

	#[cfg_attr(feature = "runtime-benchmarks", allow(dead_code))]
	pub(crate) fn evm_address_from_public_key(public_key: &KeccakPublic) -> Option<H160> {
		let public_key = Secp256k1PublicKey::from_sec1_bytes(public_key.as_ref()).ok()?;
		let uncompressed = public_key.to_encoded_point(false);
		let hash = keccak256(&uncompressed.as_bytes()[1..]);
		Some(H160::from_slice(&hash.as_slice()[12..]))
	}

	fn gateway_state_from_contract(value: ContractGatewayActivityState) -> GatewayState<T> {
		GatewayState {
			gateway_activity_nonce: value.gatewayActivityNonce,
			argon_approvals_nonce: value.argonApprovalsNonce,
			argon_circulation: value.argonCirculation.into(),
			argonot_circulation: value.argonotCirculation.into(),
		}
	}

	pub(crate) fn validate_gateway_activity_locator(
		chain_config: &ChainConfig,
		previous_gateway_activity_nonce: GatewayActivityNonce,
		proof: &GatewayActivityStorageProof<T::MaxActivitiesPerGatewayProof>,
		mut activities: Vec<Result<DecodedGatewayActivity<T>, DispatchError>>,
	) -> Result<GatewayActivityBlock<T>, DispatchError> {
		let [range_slot, root_slot] = proof.storage_proof.slots.as_slice() else {
			return Err(Error::<T>::InvalidProof.into());
		};

		let (expected_range_slot, expected_root_slot) =
			Self::gateway_activity_locator_slots(proof.locator_index);
		ensure!(
			range_slot.slot == expected_range_slot && root_slot.slot == expected_root_slot,
			Error::<T>::InvalidProof,
		);

		let (block_number, start_gateway_activity_nonce, end_gateway_activity_nonce) =
			Self::decode_gateway_activity_locator_range_slot(range_slot.value);
		ensure!(
			start_gateway_activity_nonce <= previous_gateway_activity_nonce.saturating_add(1) &&
				previous_gateway_activity_nonce < end_gateway_activity_nonce,
			Error::<T>::InvalidProof,
		);
		let activity_count = end_gateway_activity_nonce
			.checked_sub(start_gateway_activity_nonce)
			.and_then(|range| range.checked_add(1))
			.ok_or(Error::<T>::InvalidProof)?;
		ensure!(activities.len() as u64 == activity_count, Error::<T>::InvalidProof);

		let mut activity_root = B256::from(proof.activity_root_seed.0);
		for (index, activity) in activities.iter().enumerate() {
			let activity = activity.as_ref().map_err(|_| Error::<T>::InvalidProof)?;
			// The checked range length above bounds this addition by the proven end nonce.
			let gateway_activity_nonce = start_gateway_activity_nonce + index as u64;
			ensure!(
				activity.gateway_activity_nonce() == gateway_activity_nonce,
				Error::<T>::InvalidProof
			);
			activity_root = ethereum_contracts::append_activity_root(
				activity_root,
				B256::from(activity.leaf_hash(chain_config).0),
			);
		}

		let activity_root = H256::from_slice(activity_root.as_slice());
		ensure!(activity_root == root_slot.value, Error::<T>::InvalidProof);

		// Authenticate the entire locator before dropping its already-settled prefix.
		// The checked range and length above keep this index within the activity vector.
		let consumed =
			(previous_gateway_activity_nonce + 1 - start_gateway_activity_nonce) as usize;
		drop(activities.drain(..consumed));
		Ok(GatewayActivityBlock { block_number, activities })
	}

	pub(crate) fn gateway_activity_locator_slots(locator_index: u64) -> (H256, H256) {
		let mut encoded_slot = [0u8; 64];
		encoded_slot[..32].copy_from_slice(&U256::from(locator_index).to_be_bytes::<32>());
		encoded_slot[32..].copy_from_slice(
			&U256::from(ethereum_contracts::ACTIVITY_BLOCK_LOCATORS_MAPPING_SLOT)
				.to_be_bytes::<32>(),
		);
		let base_slot = U256::from_be_slice(keccak256(encoded_slot).as_slice());
		let root_slot = base_slot.saturating_add(U256::from(1u8));

		(
			H256::from_slice(&base_slot.to_be_bytes::<32>()),
			H256::from_slice(&root_slot.to_be_bytes::<32>()),
		)
	}

	#[cfg(any(test, feature = "runtime-benchmarks"))]
	pub(crate) fn gateway_activity_locator_range_slot_value(
		block_number: u64,
		start_gateway_activity_nonce: GatewayActivityNonce,
		end_gateway_activity_nonce: GatewayActivityNonce,
	) -> H256 {
		let mut encoded_slot = [0u8; 32];
		encoded_slot[24..].copy_from_slice(&block_number.to_be_bytes());
		encoded_slot[16..24].copy_from_slice(&start_gateway_activity_nonce.to_be_bytes());
		encoded_slot[8..16].copy_from_slice(&end_gateway_activity_nonce.to_be_bytes());
		H256::from(encoded_slot)
	}

	fn decode_gateway_activity_locator_range_slot(
		value: H256,
	) -> (u64, GatewayActivityNonce, GatewayActivityNonce) {
		let bytes = value.as_bytes();

		(
			u64::from_be_bytes(bytes[24..32].try_into().expect("slice length is fixed")),
			u64::from_be_bytes(bytes[16..24].try_into().expect("slice length is fixed")),
			u64::from_be_bytes(bytes[8..16].try_into().expect("slice length is fixed")),
		)
	}

	pub(crate) fn decode_evm_gateway_activity(
		chain_config: &ChainConfig,
		log: &EthereumLog,
	) -> Result<DecodedGatewayActivity<T>, DispatchError> {
		let ChainConfig::Evm { gateway, .. } = chain_config;
		ensure!(log.address == *gateway, Error::<T>::UnsupportedGateway);
		let Some(first_topic) = log.topics.first() else {
			return Err(Error::<T>::InvalidGatewayActivity.into());
		};

		if first_topic.0 == ContractTransferToArgonStarted::SIGNATURE_HASH.0 {
			let activity = ContractTransferToArgonStarted::decode_raw_log_validate(
				log.topics.iter().map(|topic| topic.0),
				&log.data,
			)
			.map_err(|_| Error::<T>::InvalidTransferToArgonActivity)?;
			let gateway_state = Self::gateway_state_from_contract(activity.gatewayState);
			return Ok(DecodedGatewayActivity::TransferToArgon {
				from: H160::from_slice(activity.from.as_slice()),
				gateway: log.address,
				token: H160::from_slice(activity.token.as_slice()),
				to: activity.argonAccountId.0,
				amount: activity.amount.into(),
				gateway_state,
			});
		}

		if first_topic.0 == ContractMintingAuthorityActivated::SIGNATURE_HASH.0 {
			let activity = ContractMintingAuthorityActivated::decode_raw_log_validate(
				log.topics.iter().map(|topic| topic.0),
				&log.data,
			)
			.map_err(|_| Error::<T>::InvalidGatewayActivity)?;
			return Ok(DecodedGatewayActivity::MintingAuthorityActivated {
				destination_signing_key: H160::from_slice(activity.signingKey.as_slice()),
				microgon_collateral: activity.microgonCollateral.into(),
				micronot_collateral: activity.micronotCollateral.into(),
				coactivation_count: activity.coactivationCount,
				shared_signature_count: activity.sharedSignatureCount,
				approval_hash: H256::from_slice(activity.approvalHash.as_slice()),
				relayer_argon_account_id: activity.relayerArgonAccountId.0,
				gateway_state: Self::gateway_state_from_contract(activity.gatewayState),
			});
		}

		if first_topic.0 == ContractGlobalIssuanceCouncilRotated::SIGNATURE_HASH.0 {
			let activity = ContractGlobalIssuanceCouncilRotated::decode_raw_log_validate(
				log.topics.iter().map(|topic| topic.0),
				&log.data,
			)
			.map_err(|_| Error::<T>::InvalidGatewayActivity)?;
			return Ok(DecodedGatewayActivity::GlobalIssuanceCouncilRotated {
				council_hash: H256::from_slice(activity.councilHash.as_slice()),
				approval_hash: H256::from_slice(activity.approvalHash.as_slice()),
				relayer_argon_account_id: activity.relayerArgonAccountId.0,
				gateway_state: Self::gateway_state_from_contract(activity.gatewayState),
			});
		}

		if first_topic.0 == ContractMintingAuthorityDeactivated::SIGNATURE_HASH.0 {
			let activity = ContractMintingAuthorityDeactivated::decode_raw_log_validate(
				log.topics.iter().map(|topic| topic.0),
				&log.data,
			)
			.map_err(|_| Error::<T>::InvalidGatewayActivity)?;
			return Ok(DecodedGatewayActivity::MintingAuthorityDeactivated {
				destination_signing_key: H160::from_slice(activity.signingKey.as_slice()),
				microgon_collateral: activity.microgonCollateral.into(),
				micronot_collateral: activity.micronotCollateral.into(),
				approval_hash: H256::from_slice(activity.approvalHash.as_slice()),
				relayer_argon_account_id: activity.relayerArgonAccountId.0,
				gateway_state: Self::gateway_state_from_contract(activity.gatewayState),
			});
		}

		if first_topic.0 == ContractTransferOutOfArgonFinalized::SIGNATURE_HASH.0 {
			let activity = ContractTransferOutOfArgonFinalized::decode_raw_log_validate(
				log.topics.iter().map(|topic| topic.0),
				&log.data,
			)
			.map_err(|_| Error::<T>::InvalidGatewayActivity)?;
			let token = H160::from_slice(activity.token.as_slice());
			let ChainConfig::Evm { gateway, argon_token, argonot_token, .. } = chain_config;
			let asset = if log.address != *gateway {
				return Err(Error::<T>::InvalidGatewayActivity.into())
			} else if token == *argon_token {
				crate::AssetKind::Argon
			} else if token == *argonot_token {
				crate::AssetKind::Argonot
			} else {
				return Err(Error::<T>::InvalidGatewayActivity.into())
			};
			return Ok(DecodedGatewayActivity::TransferOutOfArgonFinalized {
				transfer_id: H256::from_slice(activity.transferId.as_slice()),
				asset,
				amount: activity.amount.into(),
				minting_authority_collateral: activity
					.mintingCollateral
					.into_iter()
					.map(|row| GatewayMintingAuthorityCollateral::<T> {
						signing_key: H160::from_slice(row.signingKey.as_slice()),
						microgon_collateral: row.microgonCollateral.into(),
						micronot_collateral: row.micronotCollateral.into(),
					})
					.collect(),
				gateway_state: Self::gateway_state_from_contract(activity.gatewayState),
			});
		}

		if first_topic.0 == ContractTransferOutOfArgonCanceled::SIGNATURE_HASH.0 {
			let activity = ContractTransferOutOfArgonCanceled::decode_raw_log_validate(
				log.topics.iter().map(|topic| topic.0),
				&log.data,
			)
			.map_err(|_| Error::<T>::InvalidGatewayActivity)?;
			return Ok(DecodedGatewayActivity::TransferOutOfArgonCanceled {
				transfer_id: H256::from_slice(activity.transferId.as_slice()),
				gateway_state: Self::gateway_state_from_contract(activity.gatewayState),
			});
		}

		Err(Error::<T>::InvalidGatewayActivity.into())
	}
}
