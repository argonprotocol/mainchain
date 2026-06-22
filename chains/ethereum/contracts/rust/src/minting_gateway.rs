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
/// Storage slot seed for `MintingGateway.activityBlockLocators`.
pub const ACTIVITY_BLOCK_LOCATORS_MAPPING_SLOT: u64 = 11;

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
	activity_root: B256,
) -> B256 {
	keccak256(
		ActivityBlockLocatorHash {
			version: ACTIVITY_HASH_VERSION,
			blockNumber: block_number,
			startGatewayActivityNonce: start_gateway_activity_nonce,
			endGatewayActivityNonce: end_gateway_activity_nonce,
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
	use serde_json::Value;

	fn fixture() -> Value {
		serde_json::from_str(include_str!(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/../test/fixtures/gateway-activity-hashing.json"
		)))
		.unwrap()
	}

	fn json<'a>(value: &'a Value, key: &str) -> &'a Value {
		value.get(key).unwrap()
	}

	fn string(value: &Value) -> &str {
		value.as_str().unwrap()
	}

	fn address(value: &Value) -> Address {
		Address::from_str(string(value)).unwrap()
	}

	fn b256(value: &Value) -> B256 {
		B256::from_str(string(value)).unwrap()
	}

	fn u64_value(value: &Value) -> u64 {
		string(value).parse().unwrap()
	}

	fn u128_value(value: &Value) -> u128 {
		string(value).parse().unwrap()
	}

	fn u32_value(value: &Value) -> u32 {
		value.as_u64().unwrap().try_into().unwrap()
	}

	fn gateway_state(value: &Value) -> GatewayActivityState {
		GatewayActivityState {
			gatewayActivityNonce: u64_value(json(value, "gatewayActivityNonce")),
			argonApprovalsNonce: u64_value(json(value, "argonApprovalsNonce")),
			argonCirculation: u128_value(json(value, "argonCirculation")),
			argonotCirculation: u128_value(json(value, "argonotCirculation")),
		}
	}

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
	fn activity_hashes_match_shared_fixture() {
		let fixture = fixture();
		let context = json(&fixture, "context");
		let activities = json(&fixture, "activities");
		let leaf_hashes = json(&fixture, "leafHashes");
		let transfer_to_argon_started_activity = json(activities, "transferToArgonStarted");
		let minting_authority_activated_activity = json(activities, "mintingAuthorityActivated");
		let global_issuance_council_rotated_activity =
			json(activities, "globalIssuanceCouncilRotated");
		let minting_authority_deactivated_activity =
			json(activities, "mintingAuthorityDeactivated");
		let transfer_out_of_argon_canceled_activity =
			json(activities, "transferOutOfArgonCanceled");
		let transfer_out_of_argon_finalized_activity =
			json(activities, "transferOutOfArgonFinalized");
		let block_number = u64_value(json(&fixture, "blockNumber"));
		let start_gateway_activity_nonce = u64_value(json(&fixture, "startGatewayActivityNonce"));
		let end_gateway_activity_nonce = u64_value(json(&fixture, "endGatewayActivityNonce"));
		let mapping_slot = u64_value(json(&fixture, "mappingSlot"));
		let locator_index = u64_value(json(&fixture, "locatorIndex"));
		let gateway = address(json(context, "gatewayAddress"));
		let chain_id = u64_value(json(context, "chainId"));

		let transfer_to_argon_started = hash_transfer_to_argon_started_activity(
			chain_id,
			gateway,
			address(json(transfer_to_argon_started_activity, "from")),
			address(json(transfer_to_argon_started_activity, "token")),
			u128_value(json(transfer_to_argon_started_activity, "amount")),
			b256(json(transfer_to_argon_started_activity, "argonAccountId")),
			gateway_state(json(transfer_to_argon_started_activity, "gatewayState")),
		);
		assert_eq!(transfer_to_argon_started, b256(json(leaf_hashes, "transferToArgonStarted")));

		let minting_authority_activated = hash_minting_authority_activated_activity(
			chain_id,
			gateway,
			address(json(minting_authority_activated_activity, "signingKey")),
			u128_value(json(minting_authority_activated_activity, "microgonCollateral")),
			u128_value(json(minting_authority_activated_activity, "micronotCollateral")),
			u32_value(json(minting_authority_activated_activity, "coactivationCount")),
			u32_value(json(minting_authority_activated_activity, "sharedSignatureCount")),
			b256(json(minting_authority_activated_activity, "approvalHash")),
			b256(json(minting_authority_activated_activity, "relayerArgonAccountId")),
			gateway_state(json(minting_authority_activated_activity, "gatewayState")),
		);
		assert_eq!(
			minting_authority_activated,
			b256(json(leaf_hashes, "mintingAuthorityActivated"))
		);

		let global_issuance_council_rotated = hash_global_issuance_council_rotated_activity(
			chain_id,
			gateway,
			b256(json(global_issuance_council_rotated_activity, "councilHash")),
			b256(json(global_issuance_council_rotated_activity, "approvalHash")),
			b256(json(global_issuance_council_rotated_activity, "relayerArgonAccountId")),
			gateway_state(json(global_issuance_council_rotated_activity, "gatewayState")),
		);
		assert_eq!(
			global_issuance_council_rotated,
			b256(json(leaf_hashes, "globalIssuanceCouncilRotated"))
		);

		let minting_authority_deactivated = hash_minting_authority_deactivated_activity(
			chain_id,
			gateway,
			address(json(minting_authority_deactivated_activity, "signingKey")),
			u128_value(json(minting_authority_deactivated_activity, "microgonCollateral")),
			u128_value(json(minting_authority_deactivated_activity, "micronotCollateral")),
			b256(json(minting_authority_deactivated_activity, "approvalHash")),
			b256(json(minting_authority_deactivated_activity, "relayerArgonAccountId")),
			gateway_state(json(minting_authority_deactivated_activity, "gatewayState")),
		);
		assert_eq!(
			minting_authority_deactivated,
			b256(json(leaf_hashes, "mintingAuthorityDeactivated"))
		);

		let transfer_out_of_argon_canceled = hash_transfer_out_of_argon_canceled_activity(
			chain_id,
			gateway,
			b256(json(transfer_out_of_argon_canceled_activity, "transferId")),
			gateway_state(json(transfer_out_of_argon_canceled_activity, "gatewayState")),
		);
		assert_eq!(
			transfer_out_of_argon_canceled,
			b256(json(leaf_hashes, "transferOutOfArgonCanceled"))
		);

		let transfer_out_of_argon_finalized = hash_transfer_out_of_argon_finalized_activity(
			chain_id,
			gateway,
			b256(json(transfer_out_of_argon_finalized_activity, "transferId")),
			address(json(transfer_out_of_argon_finalized_activity, "token")),
			u128_value(json(transfer_out_of_argon_finalized_activity, "amount")),
			json(transfer_out_of_argon_finalized_activity, "mintingCollateral")
				.as_array()
				.unwrap()
				.iter()
				.map(|row| MintingAuthorityCollateral {
					signingKey: address(json(row, "signingKey")),
					microgonCollateral: u128_value(json(row, "microgonCollateral")),
					micronotCollateral: u128_value(json(row, "micronotCollateral")),
				})
				.collect(),
			gateway_state(json(transfer_out_of_argon_finalized_activity, "gatewayState")),
		);
		assert_eq!(
			transfer_out_of_argon_finalized,
			b256(json(leaf_hashes, "transferOutOfArgonFinalized"))
		);

		let mut activity_root = b256(json(&fixture, "activityRootSeed"));
		for root in json(&fixture, "roots").as_array().unwrap() {
			let leaf_hash = match string(json(root, "name")) {
				"transferToArgonStarted" => transfer_to_argon_started,
				"mintingAuthorityActivated" => minting_authority_activated,
				"globalIssuanceCouncilRotated" => global_issuance_council_rotated,
				"mintingAuthorityDeactivated" => minting_authority_deactivated,
				"transferOutOfArgonCanceled" => transfer_out_of_argon_canceled,
				"transferOutOfArgonFinalized" => transfer_out_of_argon_finalized,
				_ => panic!("unexpected root fixture name"),
			};
			activity_root = append_activity_root(activity_root, leaf_hash);
			assert_eq!(activity_root, b256(json(root, "root")));
		}
		assert_eq!(activity_root, b256(json(&fixture, "finalRoot")));
		assert_eq!(
			hash_activity_block_locator(
				block_number,
				start_gateway_activity_nonce,
				end_gateway_activity_nonce,
				activity_root,
			),
			b256(json(&fixture, "locatorHash"))
		);
		assert_eq!(ACTIVITY_BLOCK_LOCATORS_MAPPING_SLOT, mapping_slot);

		let mut locator_slot_key = [0u8; 64];
		locator_slot_key[..32].copy_from_slice(&U256::from(locator_index).to_be_bytes::<32>());
		locator_slot_key[32..]
			.copy_from_slice(&U256::from(ACTIVITY_BLOCK_LOCATORS_MAPPING_SLOT).to_be_bytes::<32>());
		let range_slot_key = keccak256(locator_slot_key);
		assert_eq!(range_slot_key, b256(json(&fixture, "rangeSlotKey")));

		let root_slot_key = B256::from(
			U256::from_be_slice(range_slot_key.as_slice())
				.saturating_add(U256::from(1u8))
				.to_be_bytes::<32>(),
		);
		assert_eq!(root_slot_key, b256(json(&fixture, "rootSlotKey")));

		let mut range_slot_value = [0u8; 32];
		range_slot_value[24..].copy_from_slice(&block_number.to_be_bytes());
		range_slot_value[16..24].copy_from_slice(&start_gateway_activity_nonce.to_be_bytes());
		range_slot_value[8..16].copy_from_slice(&end_gateway_activity_nonce.to_be_bytes());
		assert_eq!(B256::from(range_slot_value), b256(json(&fixture, "rangeSlotValue")));
	}
}
