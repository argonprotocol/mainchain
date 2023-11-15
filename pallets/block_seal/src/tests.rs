use binary_merkle_tree::{merkle_proof, merkle_root};
use codec::Encode;
use frame_support::{assert_err, assert_ok, pallet_prelude::Hooks};
use sp_core::{bounded_vec, ed25519::Public, OpaquePeerId, H256, U256};
use sp_keyring::ed25519::Keyring;
use sp_runtime::{
	traits::{BlakeTwo256, Header},
	BoundedVec, Digest, DigestItem,
};

use ulx_primitives::{
	block_seal::{BlockVoteEligibility, BlockVoteSource, MiningAuthority, PeerId, VoteSource},
	digests::{
		BlockVoteDigest, NotaryNotebookDigest, BLOCK_VOTES_DIGEST_ID, COMPUTE_AUTHORITY_DIGEST_ID,
	},
	inherents::BlockSealInherent,
	localchain::{BlockVote, ChannelPass},
	BlockSealAuthorityId, BlockSealDigest, BlockSealerInfo, MerkleProof, AUTHOR_DIGEST_ID,
	BLOCK_SEAL_DIGEST_ID,
};

use crate::{
	mock::{BlockSeal, *},
	pallet::{
		HasSealInherent, LastBlockSealer, TempAuthor, TempBlockSealerInfo, TempBlockVoteDigest,
		TempComputeAuthorityDigest, TempSealDigest,
	},
	Error,
};

#[test]
#[should_panic(expected = "No valid account id provided for block author.")]
fn it_should_panic_if_no_block_author() {
	new_test_ext().execute_with(|| BlockSeal::on_initialize(1));
}

#[test]
fn it_should_read_the_digests() {
	new_test_ext().execute_with(|| {
		let block_seal_digest = BlockSealDigest { nonce: 1.into() };
		let block_vote_digest = get_block_vote_digest(5, 1);
		let compute_authority = BlockSealAuthorityId::from(Public([0; 32]));
		let pre_digest = Digest {
			logs: vec![
				author_digest(1),
				seal_digest(block_seal_digest.nonce),
				DigestItem::PreRuntime(COMPUTE_AUTHORITY_DIGEST_ID, compute_authority.encode()),
				DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, block_vote_digest.encode()),
			],
		};

		System::reset_events();
		System::initialize(&42, &System::parent_hash(), &pre_digest);
		BlockSeal::on_initialize(42);
		assert_eq!(TempAuthor::<Test>::get(), Some(1u64));
		assert_eq!(TempComputeAuthorityDigest::<Test>::get(), Some(compute_authority));
		assert_eq!(HasSealInherent::<Test>::get(), false);
		assert_eq!(TempBlockVoteDigest::<Test>::get(), Some(block_vote_digest));
		assert_eq!(TempSealDigest::<Test>::get(), Some(block_seal_digest));

		HasSealInherent::<Test>::put(true);
		TempBlockSealerInfo::<Test>::put(BlockSealerInfo {
			block_vote_rewards_account: 1,
			miner_rewards_account: 1,
			notaries_included: 1,
		});
		BlockSeal::on_finalize(42);

		assert_eq!(TempAuthor::<Test>::get(), None);
		assert_eq!(TempComputeAuthorityDigest::<Test>::get(), None);
		assert_eq!(TempBlockVoteDigest::<Test>::get(), None);
		assert_eq!(TempSealDigest::<Test>::get(), None);
	});
}
#[test]
fn it_should_only_allow_a_single_seal() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		HasSealInherent::<Test>::put(true);

		// actually panics
		assert_err!(
			BlockSeal::apply(RuntimeOrigin::none(), BlockSealInherent::Continuation),
			Error::<Test>::DuplicateSealSpecification
		);
	});
}

#[test]
#[should_panic(expected = "The seal digest was not provided")]
fn it_should_panic_if_no_seal() {
	new_test_ext().execute_with(|| {
		System::reset_events();
		System::initialize(
			&2,
			&System::parent_hash(),
			&Digest { logs: vec![author_digest(1), vote_digest(1, 1)] },
		);
		BlockSeal::on_initialize(2);
	});
}

#[test]
fn it_should_only_allow_continuations_for_first_3() {
	new_test_ext().execute_with(|| {
		setup_blocks(1);
		let inherent = BlockSealInherent::ClosestNonce {
			notary_id: 1,
			block_vote: default_vote(),
			nonce: 1.into(),
			source_notebook_proof: MerkleProof {
				proof: Default::default(),
				number_of_leaves: 1,
				leaf_index: 0,
			},
			source_notebook_number: 1,
		};

		System::initialize(
			&2,
			&System::parent_hash(),
			&Digest { logs: vec![author_digest(1), seal_digest(1.into()), vote_digest(1, 1)] },
		);
		BlockSeal::on_initialize(2);

		assert_err!(
			BlockSeal::apply(RuntimeOrigin::none(), inherent),
			Error::<Test>::InvalidBlockSealUsed,
		);
	});
}

#[test]
fn it_should_allow_continuations() {
	new_test_ext().execute_with(|| {
		System::initialize(
			&1,
			&System::parent_hash(),
			&Digest { logs: vec![author_digest(1), seal_digest(1.into()), vote_digest(1, 1)] },
		);
		BlockSeal::on_initialize(1);

		assert_err!(
			BlockSeal::apply(RuntimeOrigin::none(), BlockSealInherent::Continuation),
			Error::<Test>::BlockSealDigestMismatch
		);
	});
	new_test_ext().execute_with(|| {
		System::initialize(
			&1,
			&System::parent_hash(),
			&Digest { logs: vec![author_digest(2), seal_digest(U256::MAX), vote_digest(1, 1)] },
		);
		BlockSeal::on_initialize(1);

		// no miner zero, so it fails
		assert_err!(
			BlockSeal::apply(RuntimeOrigin::none(), BlockSealInherent::Continuation),
			Error::<Test>::InvalidContinuationAuthor
		);

		MinerZero::set(Some((
			2,
			MiningAuthority {
				rpc_hosts: bounded_vec![],
				authority_id: BlockSealAuthorityId::from(Keyring::Alice.public()),
				authority_index: 0,
				peer_id: empty_peer(),
			},
		)));
		assert_ok!(BlockSeal::apply(RuntimeOrigin::none(), BlockSealInherent::Continuation));
	});
}
#[test]
fn it_requires_the_nonce_to_match() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(3);
		System::reset_events();
		let block_vote = default_vote();
		let parent_voting_key = H256::random();
		ParentVotingKey::set(Some(parent_voting_key.clone()));
		let nonce =
			block_vote.calculate_block_nonce(1, parent_voting_key.clone()) + U256::from(1u32);
		System::initialize(
			&3,
			&System::parent_hash(),
			&Digest { logs: vec![author_digest(1), seal_digest(nonce), vote_digest(1, 1)] },
		);
		BlockSeal::on_initialize(3);

		assert_err!(
			BlockSeal::apply(
				RuntimeOrigin::none(),
				BlockSealInherent::ClosestNonce {
					notary_id: 1,
					block_vote,
					nonce,
					source_notebook_proof: Default::default(),
					source_notebook_number: 1,
				}
			),
			Error::<Test>::InvalidNonce
		);
	});
}

#[test]
fn it_should_be_able_to_submit_a_seal() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(3);
		System::reset_events();
		AuthorityList::set(vec![(10, BlockSealAuthorityId::from(Public([0; 32])))]);
		XorClosest::set(Some(MiningAuthority {
			authority_id: BlockSealAuthorityId::from(Public([0; 32])),
			peer_id: empty_peer(),
			authority_index: 0,
			rpc_hosts: bounded_vec![],
		}));

		let parent_voting_key = H256::random();
		ParentVotingKey::set(Some(parent_voting_key.clone()));
		GrandpaVoteEligibility::set(Some(BlockVoteEligibility {
			allowed_sources: BlockVoteSource::Tax,
			minimum: 500,
		}));

		let block_vote = default_vote();
		let nonce = block_vote.calculate_block_nonce(1, parent_voting_key.clone());

		let root = merkle_root::<BlakeTwo256, _>(vec![block_vote.encode()]);
		VotingRoots::mutate(|a| a.insert((1, 1), (root, 1)));
		let merkle_proof = merkle_proof::<BlakeTwo256, _, _>(vec![block_vote.encode()], 0).proof;

		let inherent = BlockSealInherent::ClosestNonce {
			notary_id: 1,
			block_vote,
			nonce,
			source_notebook_proof: MerkleProof {
				proof: BoundedVec::truncate_from(merkle_proof),
				number_of_leaves: 1,
				leaf_index: 0,
			},
			source_notebook_number: 1,
		};

		System::initialize(
			&3,
			&System::parent_hash(),
			&Digest { logs: vec![author_digest(10), seal_digest(nonce), vote_digest(1, 1)] },
		);
		BlockSeal::on_initialize(3);

		assert_ok!(BlockSeal::apply(RuntimeOrigin::none(), inherent));

		assert_eq!(HasSealInherent::<Test>::get(), true);
		BlockSeal::on_finalize(2);
		assert_eq!(LastBlockSealer::<Test>::get().unwrap().miner_rewards_account, 10);
	});
}

#[test]
fn it_requires_vote_proof() {
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
		System::set_block_number(3);
		let mut vote = default_vote();
		vote.block_hash = System::block_hash(2);
		GrandpaVoteEligibility::set(Some(BlockVoteEligibility {
			allowed_sources: BlockVoteSource::Tax,
			minimum: 500,
		}));
		assert_err!(
			BlockSeal::verify_block_vote(&vote, &1, &2, 1),
			Error::<Test>::InvalidBlockHistoryVote
		);
	});
}
#[test]
fn it_checks_compute_votes() {
	new_test_ext().execute_with(|| {
		setup_blocks(2);
		System::set_block_number(3);
		let vote = BlockVote {
			block_hash: System::block_hash(System::block_number().saturating_sub(3)),
			vote_source: VoteSource::Compute { puzzle_proof: U256::from(2).pow(252.into()) },
			account_id: Keyring::Alice.into(),
			index: 1,
			power: 2,
		};
		GrandpaVoteEligibility::set(Some(BlockVoteEligibility {
			allowed_sources: BlockVoteSource::Tax,
			minimum: 500,
		}));
		assert_err!(BlockSeal::verify_block_vote(&vote, &1, &2, 1), Error::<Test>::InvalidPower);

		GrandpaVoteEligibility::set(Some(BlockVoteEligibility {
			allowed_sources: BlockVoteSource::Tax,
			minimum: 0,
		}));

		assert_err!(
			BlockSeal::verify_block_vote(&vote, &1, &2, 1),
			Error::<Test>::InvalidBlockVoteSource
		);
		GrandpaVoteEligibility::set(Some(BlockVoteEligibility {
			allowed_sources: BlockVoteSource::Compute,
			minimum: 0,
		}));
		assert_err!(
			BlockSeal::verify_block_vote(&vote, &1, &2, 1),
			Error::<Test>::InvalidAuthoritySupplied
		);
		let bob_authority = BlockSealAuthorityId::from(Keyring::Bob.public());
		TempComputeAuthorityDigest::<Test>::set(Some(bob_authority));
		assert_err!(
			BlockSeal::verify_block_vote(&vote, &1, &2, 1),
			Error::<Test>::InvalidSubmitter
		);
		assert_err!(BlockSeal::verify_block_vote(&vote, &1, &1, 1), Error::<Test>::InvalidPower);
	});
}
#[test]
fn it_checks_tax_votes() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(3);
		let vote = default_vote();
		GrandpaVoteEligibility::set(Some(BlockVoteEligibility {
			allowed_sources: BlockVoteSource::Compute,
			minimum: 500,
		}));
		assert_err!(
			BlockSeal::verify_block_vote(&vote, &1, &2, 1),
			Error::<Test>::InvalidBlockVoteSource
		);
		GrandpaVoteEligibility::set(Some(BlockVoteEligibility {
			allowed_sources: BlockVoteSource::Tax,
			minimum: 501,
		}));
		assert_err!(BlockSeal::verify_block_vote(&vote, &1, &2, 1), Error::<Test>::InvalidPower);
		GrandpaVoteEligibility::set(Some(BlockVoteEligibility {
			allowed_sources: BlockVoteSource::Tax,
			minimum: 500,
		}));
		assert_err!(
			BlockSeal::verify_block_vote(&vote, &1, &2, 1),
			Error::<Test>::UnregisteredBlockAuthor
		);
		let bob_authority = BlockSealAuthorityId::from(Keyring::Bob.public());
		AuthorityList::mutate(|a| a.push((1, bob_authority.clone())));
		assert_err!(
			BlockSeal::verify_block_vote(&vote, &1, &2, 1),
			Error::<Test>::InvalidSubmitter
		);
		XorClosest::set(Some(MiningAuthority {
			peer_id: empty_peer(),
			authority_id: bob_authority,
			rpc_hosts: Default::default(),
			authority_index: 0,
		}));
		assert_ok!(BlockSeal::verify_block_vote(&vote, &1, &2, 1));
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

fn tax_vote() -> VoteSource {
	VoteSource::Tax { channel_pass: empty_channel_pass() }
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

fn seal_digest(nonce: U256) -> DigestItem {
	DigestItem::PreRuntime(BLOCK_SEAL_DIGEST_ID, BlockSealDigest { nonce }.encode())
}

fn get_block_vote_digest(notebooks: u32, votes: u32) -> BlockVoteDigest {
	let numbers = (0..notebooks)
		.map(|a| NotaryNotebookDigest { notary_id: a, notebook_number: a })
		.collect::<Vec<_>>();
	BlockVoteDigest {
		notebook_numbers: BoundedVec::truncate_from(numbers),
		parent_voting_key: None,
		voting_power: 1,
		votes_count: votes,
	}
}

fn default_vote() -> BlockVote {
	BlockVote {
		block_hash: System::block_hash(System::block_number().saturating_sub(3)),
		vote_source: tax_vote(),
		account_id: Keyring::Alice.into(),
		index: 1,
		power: 500,
	}
}
