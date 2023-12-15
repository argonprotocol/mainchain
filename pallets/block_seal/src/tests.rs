use binary_merkle_tree::{merkle_proof, merkle_root};
use codec::Encode;
use frame_support::{
	assert_err, assert_ok,
	inherent::{InherentData, ProvideInherent},
	pallet_prelude::*,
};
use sp_core::{
	bounded_vec,
	ed25519::{Public, Signature},
	OpaquePeerId, H256, U256,
};
use sp_inherents::InherentDataProvider;
use sp_keyring::{ed25519::Keyring, Ed25519Keyring::Alice};
use sp_runtime::{
	traits::{BlakeTwo256, Header},
	BoundedVec, Digest, DigestItem,
};

use ulx_primitives::{
	block_seal::{MiningAuthority, PeerId},
	digests::{BlockVoteDigest, BLOCK_VOTES_DIGEST_ID},
	inherents::{BlockSealInherent, BlockSealInherentDataProvider, InherentError},
	localchain::{BlockVote, ChannelPass},
	BlockSealAuthorityId, BlockSealAuthoritySignature, BlockSealDigest, BlockSealerInfo,
	MerkleProof, AUTHOR_DIGEST_ID,
};

use crate::{
	mock::{BlockSeal, *},
	pallet::{LastBlockSealerInfo, TempAuthor, TempBlockVoteDigest, TempSealInherent},
	Call, Error,
};

fn empty_signature() -> BlockSealAuthoritySignature {
	Signature([0u8; 64]).into()
}

#[test]
#[should_panic(expected = "No valid account id provided for block author.")]
fn it_should_panic_if_no_block_author() {
	new_test_ext().execute_with(|| BlockSeal::on_initialize(1));
}

#[test]
#[should_panic]
fn it_should_panic_if_no_vote_digest() {
	new_test_ext().execute_with(|| {
		System::reset_events();
		System::initialize(&2, &System::parent_hash(), &Digest { logs: vec![author_digest(1)] });
		BlockSeal::on_initialize(2)
	});
}

#[test]
fn it_should_ensure_block_seal_inherents_match() {
	new_test_ext().execute_with(|| {
		let data_provider = BlockSealInherentDataProvider {
			seal: None,
			digest: Some(BlockSealDigest::Compute { nonce: U256::from(1) }),
		};
		let mut inherent_data = InherentData::new();
		assert_ok!(futures::executor::block_on(
			data_provider.provide_inherent_data(&mut inherent_data)
		));
		assert_ok!(BlockSeal::check_inherent(
			&Call::apply { seal: BlockSealInherent::Compute },
			&inherent_data,
		));
	});
}
#[test]
fn it_should_check_vote_proof_inherents() {
	new_test_ext().execute_with(|| {
		let data_provider = BlockSealInherentDataProvider {
			seal: None,
			digest: Some(BlockSealDigest::Vote { vote_proof: U256::from(1) }),
		};
		let mut inherent_data = InherentData::new();
		assert_ok!(futures::executor::block_on(
			data_provider.provide_inherent_data(&mut inherent_data)
		));
		assert_eq!(
			BlockSeal::check_inherent(
				&Call::apply { seal: BlockSealInherent::Compute },
				&inherent_data,
			)
			.unwrap_err()
			.to_string(),
			InherentError::InvalidSeal.to_string()
		);
	});
}

#[test]
fn it_should_read_the_digests() {
	new_test_ext().execute_with(|| {
		let block_vote_digest = get_block_vote_digest(5, 1);
		let pre_digest = Digest {
			logs: vec![
				author_digest(1),
				DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, block_vote_digest.encode()),
			],
		};

		System::reset_events();
		System::initialize(&42, &System::parent_hash(), &pre_digest);
		BlockSeal::on_initialize(42);
		assert_eq!(TempAuthor::<Test>::get(), Some(1u64));
		assert_eq!(TempSealInherent::<Test>::get(), None);
		assert_eq!(TempBlockVoteDigest::<Test>::get(), Some(block_vote_digest));

		TempSealInherent::<Test>::put(BlockSealInherent::Compute);
		LastBlockSealerInfo::<Test>::put(BlockSealerInfo {
			block_vote_rewards_account: 1,
			miner_rewards_account: 1,
			notaries_included: 1,
		});
		BlockSeal::on_finalize(42);

		assert_eq!(TempAuthor::<Test>::get(), None);
		assert_eq!(TempBlockVoteDigest::<Test>::get(), None);
		assert_eq!(TempSealInherent::<Test>::get(), None);
	});
}

#[test]
fn it_should_only_allow_a_single_seal() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		TempSealInherent::<Test>::put(BlockSealInherent::Compute);

		// actually panics
		assert_err!(
			BlockSeal::apply(RuntimeOrigin::none(), BlockSealInherent::Compute),
			Error::<Test>::DuplicateBlockSealProvided
		);
	});
}

#[test]
fn it_should_only_allow_compute_for_first_4() {
	new_test_ext().execute_with(|| {
		setup_blocks(1);
		let inherent = BlockSealInherent::Vote {
			notary_id: 1,
			block_vote: default_vote(),
			vote_proof: 1.into(),
			source_notebook_proof: MerkleProof {
				proof: Default::default(),
				number_of_leaves: 1,
				leaf_index: 0,
			},
			source_notebook_number: 1,
			miner_signature: empty_signature(),
		};

		System::initialize(
			&2,
			&System::parent_hash(),
			&Digest { logs: vec![author_digest(1), vote_digest(1, 1)] },
		);
		BlockSeal::on_initialize(2);

		assert_err!(
			BlockSeal::apply(RuntimeOrigin::none(), inherent),
			Error::<Test>::NoEligibleVotingRoot,
		);
	});
}

#[test]
fn it_requires_the_nonce_to_match() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(4);
		System::reset_events();
		let block_vote = default_vote();
		let parent_voting_key = H256::random();
		ParentVotingKey::set(Some(parent_voting_key.clone()));
		let vote_proof = block_vote.vote_proof(1, parent_voting_key.clone()) + U256::from(1u32);
		System::initialize(
			&4,
			&System::parent_hash(),
			&Digest { logs: vec![author_digest(1), vote_digest(1, 1)] },
		);
		BlockSeal::on_initialize(4);

		assert_err!(
			BlockSeal::apply(
				RuntimeOrigin::none(),
				BlockSealInherent::Vote {
					notary_id: 1,
					block_vote,
					vote_proof,
					source_notebook_proof: Default::default(),
					source_notebook_number: 1,
					miner_signature: empty_signature(),
				}
			),
			Error::<Test>::InvalidVoteProof
		);
	});
}

#[test]
fn it_should_be_able_to_submit_a_seal() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(4);
		System::reset_events();
		AuthorityList::set(vec![(10, default_authority())]);
		XorClosest::set(Some(MiningAuthority {
			account_id: 1,
			authority_id: default_authority(),
			peer_id: empty_peer(),
			authority_index: 0,
			rpc_hosts: bounded_vec![],
		}));

		let parent_voting_key = H256::random();
		ParentVotingKey::set(Some(parent_voting_key.clone()));
		GrandpaVoteMinimum::set(Some(500));

		let block_vote = default_vote();
		let vote_proof = block_vote.vote_proof(1, parent_voting_key.clone());

		let root = merkle_root::<BlakeTwo256, _>(vec![block_vote.encode()]);
		VotingRoots::mutate(|a| a.insert((1, 2), (root, 1)));
		let merkle_proof = merkle_proof::<BlakeTwo256, _, _>(vec![block_vote.encode()], 0).proof;

		let inherent = BlockSealInherent::Vote {
			notary_id: 1,
			block_vote,
			vote_proof,
			source_notebook_proof: MerkleProof {
				proof: BoundedVec::truncate_from(merkle_proof),
				number_of_leaves: 1,
				leaf_index: 0,
			},
			source_notebook_number: 1,
			miner_signature: Alice
				.sign(&BlockVote::vote_proof_signature_message(&System::parent_hash(), vote_proof))
				.into(),
		};

		System::initialize(
			&4,
			&System::parent_hash(),
			&Digest { logs: vec![author_digest(10), vote_digest(1, 1)] },
		);
		BlockSeal::on_initialize(4);

		assert_ok!(BlockSeal::apply(RuntimeOrigin::none(), inherent.clone()));

		assert_eq!(TempSealInherent::<Test>::get(), Some(inherent.clone()));
		assert_eq!(BlockSeal::get(), inherent);
		BlockSeal::on_finalize(4);
	});
}

#[test]
fn it_requires_vote_notebook_proof() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(3);
		System::reset_events();
		AuthorityList::set(vec![(10, BlockSealAuthorityId::from(Public([0; 32])))]);

		let mut block_vote = default_vote();
		let merkle_proof = merkle_proof::<BlakeTwo256, _, _>(vec![block_vote.encode()], 0).proof;
		let source_notebook_proof = MerkleProof {
			proof: BoundedVec::truncate_from(merkle_proof),
			number_of_leaves: 1,
			leaf_index: 0,
		};
		let root = merkle_root::<BlakeTwo256, _>(vec![block_vote.encode()]);
		VotingRoots::mutate(|a| a.insert((1, 1), (root, 2)));

		// set block to 2 - not in the history
		assert_err!(
			BlockSeal::verify_vote_source(1, 2, &block_vote, source_notebook_proof.clone(), 1,),
			Error::<Test>::NoEligibleVotingRoot
		);

		// notebook number i mismatched
		assert_err!(
			BlockSeal::verify_vote_source(1, 1, &block_vote, source_notebook_proof.clone(), 1,),
			Error::<Test>::IneligibleNotebookUsed
		);
		assert_ok!(BlockSeal::verify_vote_source(
			1,
			1,
			&block_vote,
			source_notebook_proof.clone(),
			2,
		),);

		block_vote.power = 100;
		assert_err!(
			BlockSeal::verify_vote_source(1, 1, &block_vote, source_notebook_proof.clone(), 2,),
			Error::<Test>::InvalidBlockVoteProof
		);
	});
}
#[test]
fn it_checks_that_votes_are_for_great_grandpa() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(4);
		let mut vote = default_vote();
		vote.grandparent_block_hash = System::block_hash(2);
		GrandpaVoteMinimum::set(Some(500));
		assert_err!(
			BlockSeal::verify_block_vote(U256::from(1), &vote, &1, 2, empty_signature()),
			Error::<Test>::InvalidVoteGrandparentHash
		);
	});
}

#[test]
fn it_checks_tax_votes() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(4);
		let vote = default_vote();
		let default_authority = default_authority();
		let author = &1;

		GrandpaVoteMinimum::set(Some(501));
		let vote_proof = vote.vote_proof(1, H256::random());
		assert_err!(
			BlockSeal::verify_block_vote(vote_proof, &vote, author, 2, empty_signature()),
			Error::<Test>::InsufficientVotingPower
		);
		GrandpaVoteMinimum::set(Some(500));
		assert_err!(
			BlockSeal::verify_block_vote(vote_proof, &vote, author, 2, empty_signature()),
			Error::<Test>::UnregisteredBlockAuthor
		);
		AuthorityList::mutate(|a| a.push((1, default_authority.clone())));
		assert_err!(
			BlockSeal::verify_block_vote(vote_proof, &vote, author, 2, empty_signature()),
			Error::<Test>::InvalidSubmitter
		);
		XorClosest::set(Some(MiningAuthority {
			account_id: 1,
			peer_id: empty_peer(),
			authority_id: default_authority.clone(),
			rpc_hosts: Default::default(),
			authority_index: 0,
		}));
		assert_err!(
			BlockSeal::verify_block_vote(vote_proof, &vote, author, 2, empty_signature()),
			Error::<Test>::InvalidAuthoritySignature
		);

		let signature = Alice
			.sign(&BlockVote::vote_proof_signature_message(&System::parent_hash(), vote_proof));
		assert_ok!(BlockSeal::verify_block_vote(vote_proof, &vote, author, 2, signature.into()),);
	});
}

fn empty_peer() -> PeerId {
	PeerId(OpaquePeerId::default())
}
fn setup_blocks(blocks: u64) {
	let mut parent_hash = System::parent_hash();

	for i in 1..(blocks + 1) {
		System::reset_events();
		System::initialize(&i, &parent_hash, &Default::default());

		let header = System::finalize();
		parent_hash = header.hash();
		System::set_block_number(*header.number());
	}
}

fn default_authority() -> BlockSealAuthorityId {
	BlockSealAuthorityId::from(Keyring::Alice.public())
}

fn empty_channel_pass() -> ChannelPass {
	ChannelPass { miner_index: 0, zone_record_hash: H256::zero(), id: 0, at_block_height: 0 }
}

fn author_digest(author: u64) -> DigestItem {
	DigestItem::PreRuntime(AUTHOR_DIGEST_ID, author.encode())
}

fn vote_digest(notebooks: u32, votes: u32) -> DigestItem {
	DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, get_block_vote_digest(notebooks, votes).encode())
}

fn get_block_vote_digest(notebooks: u32, votes: u32) -> BlockVoteDigest {
	BlockVoteDigest {
		tick_notebooks: notebooks,
		parent_voting_key: None,
		voting_power: 1,
		votes_count: votes,
	}
}

fn default_vote() -> BlockVote {
	BlockVote {
		grandparent_block_hash: System::block_hash(System::block_number().saturating_sub(4)),
		channel_pass: empty_channel_pass(),
		account_id: Keyring::Alice.into(),
		index: 1,
		power: 500,
	}
}
