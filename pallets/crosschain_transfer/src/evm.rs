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
use polkadot_sdk::sp_core::ecdsa::KeccakSignature;

use crate::{
	gateway_activity::{DecodedGatewayActivity, GatewayMintingAuthorityCollateral},
	ChainConfig, ChainConfigBySourceChain, Config, CouncilApprovalQueueEntry,
	CouncilApprovalQueueNonce, CouncilApprovalTargetId, Error, GatewayState,
	GlobalIssuanceCouncilMember, Pallet, SourceChain, H160, H256,
};
use polkadot_sdk::sp_runtime::BoundedBTreeMap;

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

	pub(crate) fn hash_activate_minting_authority(
		destination_chain: SourceChain,
		microgon_collateral: T::Balance,
		micronot_collateral: T::Balance,
		destination_signing_key: H160,
	) -> Result<H256, polkadot_sdk::sp_runtime::DispatchError> {
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
	) -> Result<(u64, H160), polkadot_sdk::sp_runtime::DispatchError> {
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
	) -> Result<H256, polkadot_sdk::sp_runtime::DispatchError> {
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
					ethereum_contracts::hash_minting_authority_deactivation(
						chain_id,
						AlloyAddress::from_slice(gateway.as_bytes()),
						queue_nonce,
						AlloyAddress::from_slice(destination_signing_key.as_bytes()),
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
	pub(crate) fn evm_address_from_public_key(
		public_key: &polkadot_sdk::sp_core::ecdsa::KeccakPublic,
	) -> Option<H160> {
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

	pub(crate) fn decode_evm_gateway_activity(
		source_chain: SourceChain,
		log: &EthereumLog,
	) -> Result<DecodedGatewayActivity<T>, polkadot_sdk::sp_runtime::DispatchError> {
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
			let asset = match Self::resolve_source_asset_kind(source_chain, &log.address, &token) {
				Ok(asset) => asset,
				Err(_) => return Err(Error::<T>::InvalidGatewayActivity.into()),
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
