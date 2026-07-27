#![cfg(feature = "runtime-benchmarks")]

use super::*;
#[cfg(test)]
use crate::mock::{new_test_ext, Test};
use frame_system::RawOrigin;
use polkadot_sdk::{frame_benchmarking, frame_benchmarking::v2::*, sp_core::crypto::KeyTypeId};

const BENCH_RECOVERY_KEY_TYPE: KeyTypeId = KeyTypeId(*b"brcv");

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn set_recovery_payload() {
		let writer: T::AccountId = account("writer", 0, 0);
		let recovery_public = sp_io::crypto::ed25519_generate(
			BENCH_RECOVERY_KEY_TYPE,
			Some(b"//bootstrap-recovery-benchmark".to_vec()),
		);
		let recovery_pubkey = RecoveryPubkey(recovery_public.0);
		let encrypted_recovery_payload = vec![7u8; T::MaxEncryptedPayloadLen::get() as usize];
		let message = (
			RECOVERY_PROOF_MESSAGE_KEY,
			&writer,
			&recovery_pubkey,
			blake2_256(&encrypted_recovery_payload),
		)
			.using_encoded(blake2_256);
		let signature = sp_io::crypto::ed25519_sign(
			BENCH_RECOVERY_KEY_TYPE,
			&recovery_public,
			message.as_slice(),
		)
		.expect("benchmark signing key should exist");
		let recovery_proof = RecoveryProof { signature };
		whitelist_account!(writer);

		#[extrinsic_call]
		set_recovery_payload(
			RawOrigin::Signed(writer),
			recovery_pubkey.clone(),
			recovery_proof,
			encrypted_recovery_payload.clone(),
		);

		assert_eq!(
			EncryptedRecoveryPayloadByPubkey::<T>::get(recovery_pubkey)
				.expect("recovery payload stored")
				.to_vec(),
			encrypted_recovery_payload
		);
	}

	#[benchmark]
	fn set_endpoint() {
		let endpoint_owner: T::AccountId = account("endpoint_owner", 0, 0);
		let endpoint_pubkey = EndpointPubkey([7u8; 32]);
		let encrypted_endpoint = vec![8u8; T::MaxEncryptedPayloadLen::get() as usize];
		whitelist_account!(endpoint_owner);

		#[extrinsic_call]
		set_endpoint(
			RawOrigin::Signed(endpoint_owner),
			endpoint_pubkey.clone(),
			encrypted_endpoint.clone(),
		);

		assert_eq!(
			EncryptedEndpointByPubkey::<T>::get(endpoint_pubkey)
				.expect("endpoint stored")
				.to_vec(),
			encrypted_endpoint
		);
	}

	impl_benchmark_test_suite!(Pallet, new_test_ext(), Test);
}
