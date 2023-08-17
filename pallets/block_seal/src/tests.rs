use codec::Encode;
use frame_support::{assert_noop, assert_ok, pallet_prelude::Hooks};
use sp_core::{blake2_256, crypto::AccountId32, H256, U256};
use sp_runtime::{testing::UintAuthorityId, BoundedVec, Digest, DigestItem, RuntimeAppPublic};

use ulx_primitives::{
	inherents::UlxBlockSealInherent, ProofOfWorkType, SealNonceHashMessage, SealStamper,
	SealerSignatureMessage, AUTHOR_ID,
};

use crate::{
	mock::*,
	pallet::{Author, CurrentWorkType, DidSeal},
};

#[test]
#[should_panic(expected = "A Seal must be updated only once in the block")]
fn it_should_only_allow_a_single_seal() {
	new_test_ext(vec![], vec![], 2, 5).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		DidSeal::<Test>::put(true);

		// actually panics
		assert_ok!(BlockSeal::create(
			RuntimeOrigin::none(),
			UlxBlockSealInherent {
				work_type: ProofOfWorkType::Tax,
				tax_nonce: None,
				tax_block_proof: None,
			},
		));
	});
}

#[test]
#[should_panic(expected = "No valid account id provided for block author.")]
fn it_should_panic_if_no_block_author() {
	new_test_ext(vec![], vec![], 2, 5).execute_with(|| BlockSeal::on_initialize(1));
}
#[test]
fn it_should_read_the_block_author() {
	new_test_ext(vec![], vec![], 2, 5).execute_with(|| {
		let pre_digest = Digest { logs: vec![DigestItem::PreRuntime(AUTHOR_ID, 1u64.encode())] };

		System::reset_events();
		System::initialize(&42, &System::parent_hash(), &pre_digest);
		BlockSeal::on_initialize(42);
		assert_eq!(Author::<Test>::get(), Some(1u64));
	});
}

#[test]
fn it_should_transition_to_proof_of_tax() {
	AuthorityCountInitiatingTaxProof::set(5);
	new_test_ext(vec![1, 2, 3, 4, 5], vec![], 2, 5).execute_with(|| {
		CurrentWorkType::<Test>::put(ProofOfWorkType::Compute);
		let pre_digest = Digest { logs: vec![DigestItem::PreRuntime(AUTHOR_ID, 1u64.encode())] };

		System::reset_events();
		System::initialize(&2, &System::parent_hash(), &pre_digest);
		BlockSeal::on_initialize(2);
		assert_eq!(CurrentWorkType::<Test>::get(), ProofOfWorkType::Tax);
	});
}
#[test]
#[should_panic(expected = "Block seal must be processed as an inherent for a proof of tax block")]
fn it_should_panic_if_tax_with_no_seal() {
	AuthorityCountInitiatingTaxProof::set(5);
	new_test_ext(vec![1, 2, 3, 4, 5], vec![], 2, 5).execute_with(|| {
		CurrentWorkType::<Test>::put(ProofOfWorkType::Compute);
		let pre_digest = Digest { logs: vec![DigestItem::PreRuntime(AUTHOR_ID, 1u64.encode())] };

		System::reset_events();
		System::initialize(&2, &System::parent_hash(), &pre_digest);
		BlockSeal::on_initialize(2);
		assert_eq!(CurrentWorkType::<Test>::get(), ProofOfWorkType::Tax);
		BlockSeal::on_finalize(2);
	});
}

#[test]
fn it_should_be_able_to_submit_a_seal() {
	AuthorityCountInitiatingTaxProof::set(5);
	let authorities = vec![1, 2, 3, 4, 5];
	let closest_validators = vec![1, 5, 4, 2, 3];
	new_test_ext(authorities.clone(), closest_validators.clone(), 2, 5).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		CurrentWorkType::<Test>::put(ProofOfWorkType::Tax);
		let pre_digest = Digest { logs: vec![DigestItem::PreRuntime(AUTHOR_ID, 1u64.encode())] };

		System::reset_events();
		System::initialize(&2, &System::parent_hash(), &pre_digest);
		BlockSeal::on_initialize(2);

		let inherent = create_seal(
			authorities,
			closest_validators,
			AccountId32::from([1u8; 32]),
			50,
			1,
			System::parent_hash(),
			vec![],
		);

		assert_ok!(BlockSeal::create(RuntimeOrigin::none(), inherent));

		assert_eq!(DidSeal::<Test>::get(), true);
		BlockSeal::on_finalize(2);
	});
}
#[test]
fn it_should_reject_if_block_proof_out_of_order() {
	AuthorityCountInitiatingTaxProof::set(5);
	let authorities = vec![1, 2, 3, 4, 5];
	let closest_validators = vec![1, 5, 4, 2, 3];
	new_test_ext(authorities.clone(), closest_validators.clone(), 2, 5).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		CurrentWorkType::<Test>::put(ProofOfWorkType::Tax);
		let pre_digest = Digest { logs: vec![DigestItem::PreRuntime(AUTHOR_ID, 1u64.encode())] };

		System::reset_events();
		System::initialize(&2, &System::parent_hash(), &pre_digest);
		BlockSeal::on_initialize(2);

		let inherent = create_seal(
			authorities,
			vec![4, 1, 5, 2, 3],
			AccountId32::from([1u8; 32]),
			50,
			1,
			System::parent_hash(),
			vec![],
		);

		assert_noop!(
			BlockSeal::create(RuntimeOrigin::none(), inherent),
			crate::Error::<Test>::InvalidXorClosestAuthoritiesOrder
		);

		assert_eq!(DidSeal::<Test>::get(), false);
	});
}

#[test]
fn it_should_reject_if_any_validators_not_real() {
	AuthorityCountInitiatingTaxProof::set(5);
	let authorities = vec![1, 2, 3, 4, 5];
	let closest_validators = vec![1, 5, 4, 2, 3];
	new_test_ext(authorities.clone(), closest_validators.clone(), 2, 5).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		CurrentWorkType::<Test>::put(ProofOfWorkType::Tax);
		let pre_digest = Digest { logs: vec![DigestItem::PreRuntime(AUTHOR_ID, 1u64.encode())] };

		System::reset_events();
		System::initialize(&2, &System::parent_hash(), &pre_digest);
		BlockSeal::on_initialize(2);

		let inherent = create_seal(
			authorities,
			vec![1, 5, 4, 2],
			AccountId32::from([1u8; 32]),
			50,
			1,
			System::parent_hash(),
			vec![(UintAuthorityId(6), 6)],
		);

		assert_noop!(
			BlockSeal::create(RuntimeOrigin::none(), inherent),
			crate::Error::<Test>::InvalidSealValidatorsProvided
		);

		assert_eq!(DidSeal::<Test>::get(), false);
	});
}

#[test]
fn it_should_reject_a_block_author_not_in_validators() {
	AuthorityCountInitiatingTaxProof::set(5);
	let authorities = vec![1, 2, 3, 4, 5];
	let closest_validators = vec![1, 5, 4, 2, 3];
	new_test_ext(authorities.clone(), closest_validators.clone(), 2, 5).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		CurrentWorkType::<Test>::put(ProofOfWorkType::Tax);
		let pre_digest = Digest { logs: vec![DigestItem::PreRuntime(AUTHOR_ID, 6u64.encode())] };

		System::reset_events();
		System::initialize(&2, &System::parent_hash(), &pre_digest);
		BlockSeal::on_initialize(2);

		let inherent = create_seal(
			authorities,
			closest_validators,
			AccountId32::from([1u8; 32]),
			50,
			1,
			System::parent_hash(),
			vec![],
		);

		assert_noop!(
			BlockSeal::create(RuntimeOrigin::none(), inherent),
			crate::Error::<Test>::UnregisteredBlockAuthor
		);

		assert_eq!(DidSeal::<Test>::get(), false);
	});
}

#[test]
fn it_should_detect_invalid_signatures() {
	AuthorityCountInitiatingTaxProof::set(5);
	let authorities = vec![1, 2, 3, 4, 5];
	let closest_validators = vec![1, 5, 4, 2, 3];
	new_test_ext(authorities.clone(), closest_validators.clone(), 2, 5).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		CurrentWorkType::<Test>::put(ProofOfWorkType::Tax);
		let pre_digest = Digest { logs: vec![DigestItem::PreRuntime(AUTHOR_ID, 1u64.encode())] };

		System::reset_events();
		System::initialize(&2, &System::parent_hash(), &pre_digest);
		BlockSeal::on_initialize(2);

		let mut inherent = create_seal(
			authorities,
			closest_validators,
			AccountId32::from([1u8; 32]),
			50,
			1,
			System::parent_hash(),
			vec![],
		);

		let proof = inherent.tax_block_proof.as_mut().unwrap();
		proof.seal_stampers[1].signature = Some(BoundedVec::truncate_from([1u8; 64].to_vec()));

		// create new nonce, but don't assign it yet
		let nonce = SealNonceHashMessage {
			tax_proof_id: proof.tax_proof_id,
			tax_amount: proof.tax_amount,
			parent_hash: System::parent_hash(),
			seal_stampers: proof.seal_stampers.clone(),
		}
		.using_encoded(blake2_256);

		assert_noop!(
			BlockSeal::create(RuntimeOrigin::none(), inherent.clone()),
			crate::Error::<Test>::InvalidBlockSealNonce
		);

		// now once nonce is valid, we should fail signatures
		let mut inherent = inherent.clone();
		inherent.tax_nonce = Some(U256::from(nonce));
		assert_noop!(
			BlockSeal::create(RuntimeOrigin::none(), inherent.clone()),
			crate::Error::<Test>::InvalidSealSignature
		);

		assert_eq!(DidSeal::<Test>::get(), false);
	});
}

fn create_seal(
	authorities: Vec<u64>,
	closest_validators: Vec<u64>,
	author_id: AccountId32,
	tax_amount: u128,
	tax_proof_id: u32,
	parent_hash: H256,
	mut extra_stampers: Vec<(UintAuthorityId, u16)>,
) -> UlxBlockSealInherent {
	let mut stampers_in_order = closest_validators
		.clone()
		.iter()
		.map(|v| {
			let authority_id = UintAuthorityId(*v);
			let index = authorities.iter().position(|a| a == v).unwrap_or_default();
			(authority_id, index as u16)
		})
		.collect::<Vec<(UintAuthorityId, u16)>>();
	stampers_in_order.append(&mut extra_stampers);

	let signature_message = SealerSignatureMessage {
		tax_proof_id,
		tax_amount,
		parent_hash,
		author_id: author_id.clone(),
		seal_stampers: stampers_in_order.clone(),
	}
	.using_encoded(blake2_256);

	let signers = stampers_in_order
		.clone()
		.into_iter()
		.take(2)
		.map(|x| x.0.clone())
		.collect::<Vec<_>>();

	let seal_stampers = stampers_in_order
		.clone()
		.iter()
		.map(|(authority_id, index)| {
			let signature = if signers.iter().any(|x| x == authority_id) {
				match authority_id.sign(&signature_message.clone()) {
					Some(sig) => Some(BoundedVec::truncate_from(sig.encode())),
					None => None,
				}
			} else {
				None
			};

			SealStamper { authority_idx: *index, signature }
		})
		.collect::<Vec<_>>();

	let nonce = SealNonceHashMessage {
		tax_proof_id,
		tax_amount,
		parent_hash,
		seal_stampers: seal_stampers.clone(),
	}
	.using_encoded(blake2_256);
	let inherent = UlxBlockSealInherent {
		work_type: ProofOfWorkType::Tax,
		tax_nonce: Some(U256::from(nonce)),
		tax_block_proof: Some(ulx_primitives::BlockProof {
			tax_amount,
			tax_proof_id,
			author_id,
			seal_stampers,
		}),
	};
	inherent
}
