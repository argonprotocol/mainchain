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
