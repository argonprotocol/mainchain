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

pub use MintingGateway::{
	GatewayActivityState, GlobalIssuanceCouncilRotated, MintingAuthorityActivated,
	MintingAuthorityCollateral, MintingAuthorityDeactivated, TransferOutOfArgonCanceled,
	TransferOutOfArgonFinalized, TransferOutOfArgonRequest, TransferToArgonStarted,
};

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

pub fn hash_minting_authority_deactivation(
	chain_id: u64,
	gateway: Address,
	queue_nonce: u64,
	destination_signing_key: Address,
	previous_update_hash: B256,
) -> B256 {
	keccak256(
		(
			B256::from(keccak256(b"ARGON_MINTING_AUTHORITY_DEACTIVATION").0),
			U256::from(chain_id),
			gateway,
			queue_nonce,
			destination_signing_key,
			previous_update_hash,
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
}
