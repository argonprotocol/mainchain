use alloc::vec::Vec;
use alloy_primitives::{keccak256, Address, B256, U256};
use alloy_sol_types::{sol, SolValue};

sol!(
	#[allow(missing_docs)]
	MintingGateway,
	concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../artifacts/contracts/MintingGateway.sol/MintingGateway.json"
	)
);

sol! {
	#[allow(missing_docs)]
	struct GlobalIssuanceCouncilRotatedActivityHash {
		bytes32 tag;
		uint8 version;
		uint256 chainId;
		address gateway;
		bytes32 councilHash;
		bytes32 approvalHash;
		bytes32 relayerArgonAccountId;
		bytes32 gatewayStateHash;
	}

	#[allow(missing_docs)]
	struct MintingAuthorityActivatedActivityHash {
		bytes32 tag;
		uint8 version;
		uint256 chainId;
		address gateway;
		address signingKey;
		uint128 microgonCollateral;
		uint128 micronotCollateral;
		uint32 coactivationCount;
		uint32 sharedSignatureCount;
		bytes32 approvalHash;
		bytes32 relayerArgonAccountId;
		bytes32 gatewayStateHash;
	}

	#[allow(missing_docs)]
	struct MintingAuthorityDeactivatedActivityHash {
		bytes32 tag;
		uint8 version;
		uint256 chainId;
		address gateway;
		address signingKey;
		uint128 microgonCollateral;
		uint128 micronotCollateral;
		bytes32 approvalHash;
		bytes32 relayerArgonAccountId;
		bytes32 gatewayStateHash;
	}

	#[allow(missing_docs)]
	struct TransferToArgonStartedActivityHash {
		bytes32 tag;
		uint8 version;
		uint256 chainId;
		address gateway;
		address from;
		address token;
		uint128 amount;
		bytes32 argonAccountId;
		bytes32 gatewayStateHash;
	}

	#[allow(missing_docs)]
	struct TransferOutOfArgonCanceledActivityHash {
		bytes32 tag;
		uint8 version;
		uint256 chainId;
		address gateway;
		bytes32 transferId;
		bytes32 gatewayStateHash;
	}

	#[allow(missing_docs)]
	struct TransferOutOfArgonFinalizedActivityHash {
		bytes32 tag;
		uint8 version;
		uint256 chainId;
		address gateway;
		bytes32 transferId;
		address token;
		uint128 amount;
		bytes32 mintingCollateralHash;
		bytes32 gatewayStateHash;
	}

	#[allow(missing_docs)]
	struct ActivityBlockLocatorHash {
		uint8 version;
		uint64 blockNumber;
		uint64 startGatewayActivityNonce;
		uint64 endGatewayActivityNonce;
		bytes32 previousLocatorHash;
		bytes32 activityRoot;
	}

	#[allow(missing_docs)]
	struct ActivityRootHash {
		uint8 version;
		bytes32 currentRoot;
		bytes32 activityHash;
	}
}

pub use MintingGateway::{
	GatewayActivityState, GlobalIssuanceCouncilRotated, MintingAuthorityActivated,
	MintingAuthorityCollateral, MintingAuthorityDeactivated, TransferOutOfArgonCanceled,
	TransferOutOfArgonFinalized, TransferOutOfArgonRequest, TransferToArgonStarted,
};

pub const ACTIVITY_HASH_VERSION: u8 = 1;

pub fn hash_global_issuance_council(
	signers: Vec<Address>,
	weights: Vec<U256>,
	epoch_microgons_per_argonot: u128,
) -> B256 {
	keccak256((signers, weights, epoch_microgons_per_argonot).abi_encode_params())
}

#[allow(clippy::too_many_arguments)]
pub fn hash_gateway_update_approval(
	chain_id: u64,
	gateway: Address,
	queue_nonce: u64,
	approving_council_hash: B256,
	kind: u8,
	target_id: B256,
	target_payload_hash: B256,
	previous_approval_hash: B256,
) -> B256 {
	keccak256(
		(
			B256::from(keccak256(b"ARGON_GATEWAY_UPDATE_APPROVAL").0),
			U256::from(chain_id),
			gateway,
			queue_nonce,
			approving_council_hash,
			U256::from(kind),
			target_id,
			target_payload_hash,
			previous_approval_hash,
		)
			.abi_encode_params(),
	)
}

pub fn hash_minting_authority_activation_target(
	microgon_collateral: u128,
	micronot_collateral: u128,
	destination_signing_key: Address,
) -> B256 {
	keccak256(
		(microgon_collateral, micronot_collateral, destination_signing_key).abi_encode_params(),
	)
}

pub fn hash_minting_authority_deactivation_target(destination_signing_key: Address) -> B256 {
	keccak256((destination_signing_key,).abi_encode_params())
}

pub fn hash_activate_minting_authority(
	chain_id: u64,
	gateway: Address,
	microgon_collateral: u128,
	micronot_collateral: u128,
	destination_signing_key: Address,
) -> B256 {
	keccak256(
		(
			B256::from(keccak256(b"ARGON_MINTING_AUTHORITY_ACTIVATION").0),
			U256::from(chain_id),
			gateway,
			hash_minting_authority_activation_target(
				microgon_collateral,
				micronot_collateral,
				destination_signing_key,
			),
		)
			.abi_encode_params(),
	)
}

pub fn destination_signing_key_target_id(destination_signing_key: Address) -> B256 {
	let mut target_id = [0u8; 32];
	target_id[12..].copy_from_slice(destination_signing_key.as_slice());
	B256::from(target_id)
}

#[allow(clippy::too_many_arguments)]
pub fn hash_transfer_out_of_argon_request(
	argon_account_id: [u8; 32],
	argon_transfer_nonce: u64,
	chain_id: u64,
	microgons_per_argonot: u128,
	recipient: Address,
	valid_until_block: u64,
	token: Address,
	amount: u128,
	minting_authority_tip: u128,
) -> B256 {
	keccak256(
		(
			B256::from(argon_account_id),
			argon_transfer_nonce,
			chain_id,
			microgons_per_argonot,
			recipient,
			valid_until_block,
			token,
			amount,
			minting_authority_tip,
		)
			.abi_encode_params(),
	)
}

pub fn hash_minting_authorization(
	chain_id: u64,
	gateway: Address,
	transfer_id: B256,
	microgon_collateral: u128,
	micronot_collateral: u128,
) -> B256 {
	keccak256(
		(
			B256::from(keccak256(b"ARGON_TRANSFER_OUT_OF_ARGON_AUTHORIZATION").0),
			U256::from(chain_id),
			gateway,
			transfer_id,
			microgon_collateral,
			micronot_collateral,
		)
			.abi_encode_params(),
	)
}

pub fn hash_gateway_activity_state(gateway_state: GatewayActivityState) -> B256 {
	keccak256((gateway_state,).abi_encode_params())
}

pub fn append_activity_root(current_root: B256, activity_hash: B256) -> B256 {
	keccak256(
		ActivityRootHash {
			version: ACTIVITY_HASH_VERSION,
			currentRoot: current_root,
			activityHash: activity_hash,
		}
		.abi_encode(),
	)
}

pub fn hash_activity_block_locator(
	block_number: u64,
	start_gateway_activity_nonce: u64,
	end_gateway_activity_nonce: u64,
	previous_locator_hash: B256,
	activity_root: B256,
) -> B256 {
	keccak256(
		ActivityBlockLocatorHash {
			version: ACTIVITY_HASH_VERSION,
			blockNumber: block_number,
			startGatewayActivityNonce: start_gateway_activity_nonce,
			endGatewayActivityNonce: end_gateway_activity_nonce,
			previousLocatorHash: previous_locator_hash,
			activityRoot: activity_root,
		}
		.abi_encode(),
	)
}

pub fn hash_global_issuance_council_rotated_activity(
	chain_id: u64,
	gateway: Address,
	council_hash: B256,
	approval_hash: B256,
	relayer_argon_account_id: B256,
	gateway_state: GatewayActivityState,
) -> B256 {
	keccak256(
		GlobalIssuanceCouncilRotatedActivityHash {
			tag: B256::from(keccak256(b"ARGON_GLOBAL_ISSUANCE_COUNCIL_ROTATED_ACTIVITY").0),
			version: ACTIVITY_HASH_VERSION,
			chainId: U256::from(chain_id),
			gateway,
			councilHash: council_hash,
			approvalHash: approval_hash,
			relayerArgonAccountId: relayer_argon_account_id,
			gatewayStateHash: hash_gateway_activity_state(gateway_state),
		}
		.abi_encode(),
	)
}

#[allow(clippy::too_many_arguments)]
pub fn hash_minting_authority_activated_activity(
	chain_id: u64,
	gateway: Address,
	signing_key: Address,
	microgon_collateral: u128,
	micronot_collateral: u128,
	coactivation_count: u32,
	shared_signature_count: u32,
	approval_hash: B256,
	relayer_argon_account_id: B256,
	gateway_state: GatewayActivityState,
) -> B256 {
	keccak256(
		MintingAuthorityActivatedActivityHash {
			tag: B256::from(keccak256(b"ARGON_MINTING_AUTHORITY_ACTIVATED_ACTIVITY").0),
			version: ACTIVITY_HASH_VERSION,
			chainId: U256::from(chain_id),
			gateway,
			signingKey: signing_key,
			microgonCollateral: microgon_collateral,
			micronotCollateral: micronot_collateral,
			coactivationCount: coactivation_count,
			sharedSignatureCount: shared_signature_count,
			approvalHash: approval_hash,
			relayerArgonAccountId: relayer_argon_account_id,
			gatewayStateHash: hash_gateway_activity_state(gateway_state),
		}
		.abi_encode(),
	)
}

#[allow(clippy::too_many_arguments)]
pub fn hash_minting_authority_deactivated_activity(
	chain_id: u64,
	gateway: Address,
	signing_key: Address,
	microgon_collateral: u128,
	micronot_collateral: u128,
	approval_hash: B256,
	relayer_argon_account_id: B256,
	gateway_state: GatewayActivityState,
) -> B256 {
	keccak256(
		MintingAuthorityDeactivatedActivityHash {
			tag: B256::from(keccak256(b"ARGON_MINTING_AUTHORITY_DEACTIVATED_ACTIVITY").0),
			version: ACTIVITY_HASH_VERSION,
			chainId: U256::from(chain_id),
			gateway,
			signingKey: signing_key,
			microgonCollateral: microgon_collateral,
			micronotCollateral: micronot_collateral,
			approvalHash: approval_hash,
			relayerArgonAccountId: relayer_argon_account_id,
			gatewayStateHash: hash_gateway_activity_state(gateway_state),
		}
		.abi_encode(),
	)
}

pub fn hash_transfer_to_argon_started_activity(
	chain_id: u64,
	gateway: Address,
	from: Address,
	token: Address,
	amount: u128,
	argon_account_id: B256,
	gateway_state: GatewayActivityState,
) -> B256 {
	keccak256(
		TransferToArgonStartedActivityHash {
			tag: B256::from(keccak256(b"ARGON_TRANSFER_TO_ARGON_STARTED_ACTIVITY").0),
			version: ACTIVITY_HASH_VERSION,
			chainId: U256::from(chain_id),
			gateway,
			from,
			token,
			amount,
			argonAccountId: argon_account_id,
			gatewayStateHash: hash_gateway_activity_state(gateway_state),
		}
		.abi_encode(),
	)
}

pub fn hash_transfer_out_of_argon_canceled_activity(
	chain_id: u64,
	gateway: Address,
	transfer_id: B256,
	gateway_state: GatewayActivityState,
) -> B256 {
	keccak256(
		TransferOutOfArgonCanceledActivityHash {
			tag: B256::from(keccak256(b"ARGON_TRANSFER_OUT_OF_ARGON_CANCELED_ACTIVITY").0),
			version: ACTIVITY_HASH_VERSION,
			chainId: U256::from(chain_id),
			gateway,
			transferId: transfer_id,
			gatewayStateHash: hash_gateway_activity_state(gateway_state),
		}
		.abi_encode(),
	)
}

pub fn hash_transfer_out_of_argon_finalized_activity(
	chain_id: u64,
	gateway: Address,
	transfer_id: B256,
	token: Address,
	amount: u128,
	minting_collateral: Vec<MintingAuthorityCollateral>,
	gateway_state: GatewayActivityState,
) -> B256 {
	let minting_collateral_hash = keccak256((minting_collateral,).abi_encode_params());

	keccak256(
		TransferOutOfArgonFinalizedActivityHash {
			tag: B256::from(keccak256(b"ARGON_TRANSFER_OUT_OF_ARGON_FINALIZED_ACTIVITY").0),
			version: ACTIVITY_HASH_VERSION,
			chainId: U256::from(chain_id),
			gateway,
			transferId: transfer_id,
			token,
			amount,
			mintingCollateralHash: minting_collateral_hash,
			gatewayStateHash: hash_gateway_activity_state(gateway_state),
		}
		.abi_encode(),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use alloy_primitives::Bytes;
	use alloy_sol_types::SolCall;
	use core::str::FromStr;

	#[test]
	fn transfer_out_hashes_match_the_known_vector() {
		let transfer_id = hash_transfer_out_of_argon_request(
			[0x22; 32],
			1,
			1,
			7,
			Address::from([0x55; 20]),
			10,
			Address::from([0x44; 20]),
			25,
			1,
		);

		assert_eq!(
			transfer_id,
			B256::from_str("0x15547a58403a407ace23aaafbc0343f0221b1564405e9974f692e5f25b9b08ce")
				.unwrap()
		);
		assert_eq!(
			hash_minting_authorization(1, Address::from([0x11; 20]), transfer_id, 16, 0),
			B256::from_str("0x94fb1ef1202e2f7b0e2943219453e28153d2a97069bdb48ce28a695a6cf3bbb8")
				.unwrap()
		);
	}

	#[test]
	fn finalize_transfer_out_calldata_matches_the_known_vector() {
		let calldata = MintingGateway::finalizeTransferOutOfArgonCall {
			request: TransferOutOfArgonRequest {
				argonAccountId: [0x22; 32].into(),
				argonTransferNonce: 1,
				chainId: 1,
				microgonsPerArgonot: 7,
				recipient: Address::from([0x55; 20]),
				validUntilBlock: 10,
				token: Address::from([0x44; 20]),
				amount: 25,
				mintingAuthorityTip: 1,
			},
			proof: MintingGateway::TransferOutOfArgonProof {
				authorizations: vec![MintingGateway::MintingAuthorization {
					microgonCollateral: 16,
					micronotCollateral: 0,
					signature: Bytes::from(vec![0x33; 65]),
				}],
			},
		}
		.abi_encode();

		assert_eq!(
			Bytes::from(calldata),
			Bytes::from_str(
				"0x138f878122222222222222222222222222222222222222222222222222222222222222220000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000070000000000000000000000005555555555555555555555555555555555555555000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000044444444444444444444444444444444444444440000000000000000000000000000000000000000000000000000000000000019000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000041333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333300000000000000000000000000000000000000000000000000000000000000"
			)
			.unwrap()
			);
	}

	#[test]
	fn activity_hashes_match_known_vectors() {
		let gateway = Address::from_str("0x8A2841380151160BBf88D44D8D9A4DdA05838B76").unwrap();
		let token = Address::from_str("0xa1a1e87614B1f447fD8E65c18DCc41bAd4A7A1E5").unwrap();
		let from = Address::from([0x11; 20]);
		let signing_key = Address::from([0x22; 20]);
		let argon_account_id =
			B256::from_str("0x6172676f6e2d6163636f756e742d310000000000000000000000000000000000")
				.unwrap();
		let relayer_argon_account_id =
			B256::from_str("0x72656c617965722d310000000000000000000000000000000000000000000000")
				.unwrap();
		let approval_hash = B256::from([0x33; 32]);
		let state1 = GatewayActivityState {
			gatewayActivityNonce: 1,
			argonApprovalsNonce: 0,
			argonCirculation: 750,
			argonotCirculation: 2_000,
		};
		let state2 = GatewayActivityState {
			gatewayActivityNonce: 2,
			argonApprovalsNonce: 1,
			argonCirculation: 750,
			argonotCirculation: 2_000,
		};

		let leaf1 = hash_transfer_to_argon_started_activity(
			11_155_111,
			gateway,
			from,
			token,
			250,
			argon_account_id,
			state1,
		);
		let leaf2 = hash_minting_authority_activated_activity(
			11_155_111,
			gateway,
			signing_key,
			1_000,
			100,
			2,
			3,
			approval_hash,
			relayer_argon_account_id,
			state2,
		);

		assert_eq!(
			leaf1,
			B256::from_str("0x5cf4762ca44e4650c2f11040a0d9864cdb84fc0df19f478605ac2e3872032857")
				.unwrap()
		);
		assert_eq!(
			leaf2,
			B256::from_str("0xc7a6582937d2403a52b116f280ba9367acc5f77142527bf06da1177256491868")
				.unwrap()
		);
		assert_eq!(
			append_activity_root(B256::ZERO, leaf1),
			B256::from_str("0x77344b2f6e1a3d658361e8760b7ab1cb824371891488876d4b1126eb645dbc1a")
				.unwrap()
		);
		assert_eq!(
			append_activity_root(append_activity_root(B256::ZERO, leaf1), leaf2),
			B256::from_str("0x9bf620d17225f03ab3cbd4fce6218ff360a324391bb68ad77669c4401a9f23cf")
				.unwrap()
		);
		assert_eq!(
			hash_activity_block_locator(
				10,
				1,
				1,
				B256::ZERO,
				append_activity_root(B256::ZERO, leaf1),
			),
			B256::from_str("0x7572bb6cc2ec2ccec41eaa58a3d0f81e3dcdb06fcb4e3e522d58a08c2054fefd")
				.unwrap()
		);
	}
}
