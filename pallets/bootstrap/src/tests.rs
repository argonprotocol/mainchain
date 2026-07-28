use crate::{
	mock::{new_test_ext, Bootstrap, MaxEncryptedPayloadLen, RuntimeOrigin, Test},
	EncryptedEndpointByPubkey, EncryptedRecoveryPayloadByPubkey, EndpointPubkey, Error,
	RecoveryProof, RecoveryPubkey, RECOVERY_PROOF_MESSAGE_KEY,
};
use frame_support::{assert_noop, assert_ok};
use pallet_prelude::*;
use sp_core::{ed25519, sr25519, Pair};
use sp_io::hashing::blake2_256;
use sp_runtime::{traits::IdentifyAccount, AccountId32, MultiSigner};

#[test]
fn recovery_payload_requires_matching_key_and_writer() {
	new_test_ext().execute_with(|| {
		let writer = account_id_from_seed(1);
		let other_writer = account_id_from_seed(2);
		let recovery_pair = ed25519::Pair::from_seed(&[3u8; 32]);
		let other_recovery_pair = ed25519::Pair::from_seed(&[4u8; 32]);
		let recovery_pubkey = RecoveryPubkey(recovery_pair.public().0);
		let encrypted_recovery_payload = vec![7u8; 60];

		assert_noop!(
			Bootstrap::set_recovery_payload(
				RuntimeOrigin::signed(writer.clone()),
				recovery_pubkey.clone(),
				recovery_proof(
					&writer,
					&recovery_pubkey,
					&other_recovery_pair,
					&encrypted_recovery_payload
				),
				encrypted_recovery_payload.clone(),
			),
			Error::<Test>::InvalidRecoveryProof
		);
		assert_ok!(Bootstrap::set_recovery_payload(
			RuntimeOrigin::signed(writer.clone()),
			recovery_pubkey.clone(),
			recovery_proof(&writer, &recovery_pubkey, &recovery_pair, &encrypted_recovery_payload),
			encrypted_recovery_payload.clone(),
		));
		assert_eq!(
			EncryptedRecoveryPayloadByPubkey::<Test>::get(&recovery_pubkey)
				.expect("recovery payload stored")
				.to_vec(),
			encrypted_recovery_payload
		);

		let replacement = vec![8u8; 60];
		assert_noop!(
			Bootstrap::set_recovery_payload(
				RuntimeOrigin::signed(other_writer.clone()),
				recovery_pubkey.clone(),
				recovery_proof(&writer, &recovery_pubkey, &recovery_pair, &replacement),
				replacement.clone(),
			),
			Error::<Test>::InvalidRecoveryProof
		);
		assert_ok!(Bootstrap::set_recovery_payload(
			RuntimeOrigin::signed(other_writer.clone()),
			recovery_pubkey.clone(),
			recovery_proof(&other_writer, &recovery_pubkey, &recovery_pair, &replacement),
			replacement.clone(),
		));
		assert_eq!(
			EncryptedRecoveryPayloadByPubkey::<Test>::get(&recovery_pubkey)
				.expect("recovery payload replaced")
				.to_vec(),
			replacement
		);

		let oversized = vec![0u8; MaxEncryptedPayloadLen::get() as usize + 1];
		assert_noop!(
			Bootstrap::set_recovery_payload(
				RuntimeOrigin::signed(other_writer.clone()),
				recovery_pubkey.clone(),
				recovery_proof(&other_writer, &recovery_pubkey, &recovery_pair, &oversized),
				oversized,
			),
			Error::<Test>::EncryptedPayloadTooLong
		);
	});
}

#[test]
fn endpoint_updates_remain_owned_by_the_initial_owner() {
	new_test_ext().execute_with(|| {
		let endpoint_owner = account_id_from_seed(4);
		let other_account = account_id_from_seed(5);
		let endpoint_pubkey = EndpointPubkey([6u8; 32]);
		let encrypted_endpoint = vec![7u8; 96];

		assert_ok!(Bootstrap::set_endpoint(
			RuntimeOrigin::signed(endpoint_owner.clone()),
			endpoint_pubkey.clone(),
			encrypted_endpoint.clone(),
		));
		assert_eq!(
			EncryptedEndpointByPubkey::<Test>::get(&endpoint_pubkey)
				.expect("endpoint stored")
				.to_vec(),
			encrypted_endpoint
		);
		assert_noop!(
			Bootstrap::set_endpoint(
				RuntimeOrigin::signed(other_account),
				endpoint_pubkey.clone(),
				vec![8u8; 96],
			),
			Error::<Test>::EndpointOwnedByAnotherAccount
		);
		assert_noop!(
			Bootstrap::set_endpoint(
				RuntimeOrigin::signed(endpoint_owner),
				endpoint_pubkey,
				vec![0u8; MaxEncryptedPayloadLen::get() as usize + 1],
			),
			Error::<Test>::EncryptedPayloadTooLong
		);
	});
}

fn account_id_from_seed(seed: u8) -> AccountId32 {
	let pair = sr25519::Pair::from_seed(&[seed; 32]);
	MultiSigner::from(pair.public()).into_account()
}

fn recovery_proof(
	writer: &AccountId32,
	recovery_pubkey: &RecoveryPubkey,
	recovery_pair: &ed25519::Pair,
	encrypted_recovery_payload: &[u8],
) -> RecoveryProof {
	let message = (
		RECOVERY_PROOF_MESSAGE_KEY,
		writer,
		recovery_pubkey,
		blake2_256(encrypted_recovery_payload),
	)
		.using_encoded(blake2_256);
	RecoveryProof { signature: recovery_pair.sign(message.as_slice()) }
}
