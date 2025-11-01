use binary_merkle_tree::{merkle_proof, merkle_root};
use frame_support::inherent::{InherentData, ProvideInherent};
use pallet_prelude::*;
use sp_core::ed25519::{Public, Signature};
use sp_inherents::InherentDataProvider;
use sp_keyring::{
	Ed25519Keyring,
	Ed25519Keyring::Alice,
	Sr25519Keyring::{Bob, Ferdie},
	ed25519::Keyring,
};
use sp_runtime::MultiSignature;
use std::{ops::Add, panic::catch_unwind};

use crate::{
	Call, Error,
	mock::{BlockSeal, *},
	pallet::{
		BlockForkPower, IsBlockFromVoteSeal, LastBlockSealerInfo, LastTickWithVoteSeal,
		ParentVotingKey, TempSealInherent, VotesInPast3Ticks,
	},
};
use argon_primitives::{
	BlockSealAuthoritySignature, BlockSealDigest, BlockVoteT, BlockVotingKey, Domain,
	DomainTopLevel, FORK_POWER_DIGEST, MerkleProof, PARENT_VOTING_KEY_DIGEST,
	ParentVotingKeyDigest, VotingSchedule,
	block_seal::MiningAuthority,
	digests::BlockVoteDigest,
	fork_power::ForkPower,
	inherents::{BlockSealInherent, BlockSealInherentDataProvider, SealInherentError},
	localchain::BlockVote,
	notary::NotaryNotebookRawVotes,
};

fn empty_signature() -> BlockSealAuthoritySignature {
	Signature::from_raw([0u8; 64]).into()
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
fn it_should_check_vote_seal_inherents() {
	new_test_ext().execute_with(|| {
		let data_provider = BlockSealInherentDataProvider {
			seal: None,
			digest: Some(BlockSealDigest::Vote {
				seal_strength: U256::from(1),
				signature: empty_signature(),
				miner_nonce_score: Some(U256::one()),
			}),
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
			SealInherentError::InvalidSeal.to_string()
		);
	});
}

#[test]
fn it_does_not_allow_a_compute_block_in_same_tick_as_vote() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		CurrentTick::set(100);

		LastTickWithVoteSeal::<Test>::put(99);
		assert_ok!(BlockSeal::apply(RuntimeOrigin::none(), BlockSealInherent::Compute),);
		assert_eq!(TempSealInherent::<Test>::get(), Some(BlockSealInherent::Compute));
		assert!(!IsBlockFromVoteSeal::<Test>::get());
		assert_eq!(LastTickWithVoteSeal::<Test>::get(), 99);
		BlockSeal::on_finalize(1);

		System::set_block_number(2);
		assert_ok!(BlockSeal::apply(RuntimeOrigin::none(), BlockSealInherent::Compute),);
		assert!(!IsBlockFromVoteSeal::<Test>::get());
		assert_eq!(LastTickWithVoteSeal::<Test>::get(), 99);
		BlockSeal::on_finalize(2);

		// now if this was a tick block, it shouldn't allow it
		System::set_block_number(3);
		LastTickWithVoteSeal::<Test>::put(100);

		// actually panics
		assert_err!(
			BlockSeal::apply(RuntimeOrigin::none(), BlockSealInherent::Compute),
			Error::<Test>::InvalidComputeBlockTick
		);
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
fn it_requires_the_nonce_to_match() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(4);
		CurrentTick::set(5);
		System::reset_events();
		let block_vote = default_vote();
		let parent_voting_key = H256::random();
		ParentVotingKey::<Test>::set(Some(parent_voting_key));
		let seal_strength = block_vote.get_seal_strength(1, parent_voting_key) + U256::from(1u32);

		BlockSeal::on_initialize(4);

		assert_err!(
			BlockSeal::apply(
				RuntimeOrigin::none(),
				BlockSealInherent::Vote {
					notary_id: 1,
					block_vote,
					seal_strength,
					source_notebook_proof: Default::default(),
					source_notebook_number: 1,
					miner_nonce_score: Some(U256::one())
				}
			),
			Error::<Test>::InvalidVoteSealStrength
		);
	});
}

#[test]
fn it_can_validate_miner_signatures() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(4);
		CurrentTick::set(5);
		System::reset_events();
		let hash = System::parent_hash();
		let err = catch_unwind(|| {
			let signature =
				Ed25519Keyring::Alice.sign(&BlockVote::seal_signature_message(hash)).into();

			assert!(BlockSeal::is_valid_miner_signature(
				hash,
				&BlockSealDigest::Vote {
					seal_strength: 1.into(),
					signature,
					miner_nonce_score: Some(U256::one())
				},
				&Digest { logs: vec![] }
			));
		});
		assert!(err.is_err());

		AuthorityList::set(vec![(Alice.into(), Ed25519Keyring::Alice.public().into())]);
		let signature = Ed25519Keyring::Alice.sign(&BlockVote::seal_signature_message(hash)).into();

		assert!(BlockSeal::is_valid_miner_signature(
			hash,
			&BlockSealDigest::Vote {
				seal_strength: 1.into(),
				signature,
				miner_nonce_score: Some(U256::one())
			},
			&Digest { logs: vec![] }
		));
	});
}

#[test]
fn it_should_be_able_to_submit_a_seal() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(6);
		System::set_block_number(6);
		System::reset_events();
		AuthorityList::set(vec![(Bob.into(), default_authority())]);
		BestMinerNonce::set(Some((
			MiningAuthority {
				account_id: Alice.into(),
				authority_id: default_authority(),
				authority_index: (1, 0),
			},
			U256::from(100),
		)));

		let parent_voting_key = H256::random();
		ParentVotingKey::<Test>::put(Some(parent_voting_key));
		GrandpaVoteMinimum::set(Some(500));
		CurrentTick::set(6);
		let voting_schedule = VotingSchedule::when_evaluating_runtime_seals(6);
		assert_eq!(voting_schedule.grandparent_votes_tick(), 2);
		assert_eq!(voting_schedule.eligible_votes_tick(), 4);
		assert!(!BlockSeal::has_eligible_votes());
		BlocksAtTick::mutate(|a| {
			a.insert(voting_schedule.grandparent_votes_tick(), vec![System::block_hash(2)]);
		});
		assert!(!BlockSeal::has_eligible_votes());
		RegisteredDomains::mutate(|a| {
			a.insert(Domain::new("test", DomainTopLevel::Bikes).hash());
		});
		NotebooksAtTick::mutate(|a| {
			a.insert(voting_schedule.notebook_tick(), vec![(1, 2, None)]);
		});

		let block_vote = default_vote();
		let seal_strength = block_vote.get_seal_strength(1, parent_voting_key);

		let root = merkle_root::<BlakeTwo256, _>(vec![block_vote.encode()]);
		VotingRoots::mutate(|a| a.insert((1, voting_schedule.eligible_votes_tick()), (root, 1)));
		let _ = VotesInPast3Ticks::<Test>::try_append((voting_schedule.eligible_votes_tick(), 1));
		assert!(!BlockSeal::has_eligible_votes());
		let merkle_proof = merkle_proof::<BlakeTwo256, _, _>(vec![block_vote.encode()], 0).proof;

		let inherent = BlockSealInherent::Vote {
			notary_id: 1,
			block_vote,
			seal_strength,
			source_notebook_proof: MerkleProof {
				proof: BoundedVec::truncate_from(merkle_proof),
				number_of_leaves: 1,
				leaf_index: 0,
			},
			source_notebook_number: 1,
			miner_nonce_score: Some(U256::from(100)),
		};

		BlockSeal::on_initialize(4);

		Digests::mutate(|a| {
			a.block_vote = BlockVoteDigest { voting_power: 1, votes_count: 1 };
			a.author = Bob.into();
		});

		assert_ok!(BlockSeal::apply(RuntimeOrigin::none(), inherent.clone()));
		// only after block seal is applied is this true
		assert!(BlockSeal::has_eligible_votes());
		assert!(IsBlockFromVoteSeal::<Test>::get());

		assert_eq!(LastBlockSealerInfo::<Test>::get().unwrap().block_author_account_id, Bob.into());

		let new_notebook_voting_schedule =
			VotingSchedule::from_runtime_current_tick(CurrentTick::get());
		assert_eq!(
			VotesInPast3Ticks::<Test>::get().into_inner(),
			vec![
				(voting_schedule.eligible_votes_tick(), 1),
				(new_notebook_voting_schedule.notebook_tick(), 1)
			]
		);

		// the vote sealer will be a u64 conversion of an account32
		assert_eq!(TempSealInherent::<Test>::get(), Some(inherent.clone()));
		assert_eq!(BlockSeal::get(), inherent);
		println!("{:?}", BlockForkPower::<Test>::get());
		assert_eq!(BlockForkPower::<Test>::get().seal_strength, seal_strength);
		assert_eq!(BlockForkPower::<Test>::get().voting_power, U256::from(1));
		assert_eq!(BlockForkPower::<Test>::get().notebooks, 1);
		assert_eq!(BlockForkPower::<Test>::get().vote_created_blocks, 1);
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
		AuthorityList::set(vec![(
			Bob.into(),
			BlockSealAuthorityId::from(Public::from_raw([0; 32])),
		)]);

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
			BlockSeal::verify_vote_source(
				1,
				&VotingSchedule::when_evaluating_runtime_seals(4),
				&block_vote,
				&source_notebook_proof,
				1,
			),
			Error::<Test>::NoEligibleVotingRoot
		);

		let voting_schedule = VotingSchedule::when_evaluating_runtime_seals(3);
		// notebook number i mismatched
		assert_err!(
			BlockSeal::verify_vote_source(
				1,
				&voting_schedule,
				&block_vote,
				&source_notebook_proof,
				1,
			),
			Error::<Test>::IneligibleNotebookUsed
		);
		assert_ok!(BlockSeal::verify_vote_source(
			1,
			&voting_schedule,
			&block_vote,
			&source_notebook_proof,
			2,
		),);

		block_vote.power = 100;
		assert_err!(
			BlockSeal::verify_vote_source(
				1,
				&voting_schedule,
				&block_vote,
				&source_notebook_proof,
				2,
			),
			Error::<Test>::InvalidBlockVoteProof
		);
	});
}
#[test]
fn it_checks_that_votes_are_for_great_grandpa_tick() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(10);
		System::set_block_number(10);
		let mut vote = default_vote();
		vote.block_hash = System::block_hash(8);
		let voting_schedule = VotingSchedule::when_evaluating_runtime_seals(10);
		assert_eq!(voting_schedule.grandparent_votes_tick(), 6);
		assert_eq!(voting_schedule.eligible_votes_tick(), 8);

		GrandpaVoteMinimum::set(Some(500));

		BlocksAtTick::mutate(|a| {
			a.insert(voting_schedule.grandparent_votes_tick() - 1, vec![vote.block_hash]);
		});
		assert_err!(
			BlockSeal::verify_block_vote(
				U256::from(1),
				&vote,
				&Alice.into(),
				&voting_schedule,
				U256::one()
			),
			Error::<Test>::InvalidVoteGrandparentHash
		);
		BlocksAtTick::mutate(|a| {
			a.insert(voting_schedule.grandparent_votes_tick(), vec![vote.block_hash]);
		});
		// still errors, but moves past the invalid vote hash
		assert_err!(
			BlockSeal::verify_block_vote(
				U256::from(1),
				&vote,
				&Alice.into(),
				&voting_schedule,
				U256::one()
			),
			Error::<Test>::BlockVoteInvalidSignature
		);
	});
}

#[test]
fn it_creates_the_next_parent_key() {
	new_test_ext().execute_with(|| {
		let old_root1 = H256::random();
		let old_root2 = H256::random();
		// notary 1/2 at tick 2
		VotingRoots::mutate(|a| {
			a.insert((1, 2), (old_root1, 1));
			a.insert((2, 2), (old_root2, 1));
		});

		let book1_secret = H256::from_slice(&[1u8; 32]);
		let book2_secret = H256::from_slice(&[2u8; 32]);

		let parent_key = BlockVotingKey::create_key(vec![
			BlockVotingKey { parent_secret: book1_secret, parent_vote_root: old_root1 },
			BlockVotingKey { parent_secret: book2_secret, parent_vote_root: old_root2 },
		]);

		CurrentTick::set(4);
		TempSealInherent::<Test>::put(BlockSealInherent::Compute);
		BlockSeal::on_initialize(4);

		// add notebook 2/2 at tick 3
		NotebooksAtTick::mutate(|a| {
			a.insert(3, vec![(1, 2, Some(book1_secret)), (2, 2, Some(book2_secret))]);
		});

		BlockSeal::on_finalize(4);
		assert_eq!(ParentVotingKey::<Test>::get(), Some(parent_key));
		assert_eq!(
			System::digest()
				.logs
				.iter()
				.find(|a| matches!(a, DigestItem::Consensus(PARENT_VOTING_KEY_DIGEST, _))),
			Some(&DigestItem::Consensus(PARENT_VOTING_KEY_DIGEST, Some(parent_key).encode()))
		);
	});
}

#[test]
#[should_panic]
fn it_should_panic_if_voting_key_digest_is_wrong() {
	new_test_ext().execute_with(|| {
		let old_root1 = H256::random();
		VotingRoots::mutate(|a| {
			a.insert((1, 2), (old_root1, 1));
		});

		let parent_key = BlockVotingKey::create_key(vec![]);
		Digests::mutate(|a| {
			a.voting_key = Some(ParentVotingKeyDigest { parent_voting_key: Some(parent_key) });
		});

		CurrentTick::set(3);
		TempSealInherent::<Test>::put(BlockSealInherent::Compute);
		BlockSeal::on_initialize(3);

		NotebooksAtTick::mutate(|a| {
			a.insert(3, vec![(1, 2, Some(H256::random()))]);
		});

		BlockSeal::on_finalize(3);
	});
}

#[test]
fn it_creates_the_fork_power_digest() {
	new_test_ext().execute_with(|| {
		let fork_power = ForkPower::default();
		BlockForkPower::<Test>::put(fork_power.clone());

		let mut next_fork_power = fork_power.clone();
		next_fork_power.add(1u128, 1u32, BlockSealDigest::Compute { nonce: 1.into() }, 1);
		CurrentTick::set(4);
		TempSealInherent::<Test>::put(BlockSealInherent::Compute);
		BlockSeal::on_initialize(4);
		Digests::mutate(|a| {
			a.fork_power = Some(next_fork_power.clone());
		});

		BlockForkPower::<Test>::put(next_fork_power.clone());
		BlockSeal::on_finalize(4);
		assert_eq!(
			System::digest()
				.logs
				.iter()
				.find(|a| matches!(a, DigestItem::Consensus(FORK_POWER_DIGEST, _))),
			None
		);

		// now it should add a log
		TempSealInherent::<Test>::put(BlockSealInherent::Compute);
		Digests::mutate(|a| {
			a.fork_power = None;
		});
		BlockSeal::on_finalize(5);
		assert_eq!(
			System::digest()
				.logs
				.iter()
				.find(|a| matches!(a, DigestItem::Consensus(FORK_POWER_DIGEST, _))),
			Some(&DigestItem::Consensus(FORK_POWER_DIGEST, next_fork_power.encode()))
		);
	});
}

#[test]
#[should_panic(expected = "does not match")]
fn it_should_panic_if_the_fork_power_mismatches() {
	new_test_ext().execute_with(|| {
		let fork_power = ForkPower::default();
		BlockForkPower::<Test>::put(fork_power.clone());

		let mut next_fork_power = fork_power.clone();
		next_fork_power.add(1u128, 1u32, BlockSealDigest::Compute { nonce: 1.into() }, 1);
		CurrentTick::set(4);
		TempSealInherent::<Test>::put(BlockSealInherent::Compute);
		BlockSeal::on_initialize(4);
		Digests::mutate(|a| {
			a.fork_power = Some(next_fork_power.clone());
		});
		BlockSeal::on_finalize(4);
	});
}

#[test]
fn it_skips_ineligible_voting_roots() {
	new_test_ext().execute_with(|| {
		let old_root2 = H256::random();
		// no voting root for notary 1
		VotingRoots::mutate(|a| {
			a.insert((2, 2), (old_root2, 1));
		});

		let book1_secret = H256::from_slice(&[1u8; 32]);
		let book2_secret = H256::from_slice(&[2u8; 32]);

		let parent_key = BlockVotingKey::create_key(vec![BlockVotingKey {
			parent_secret: book2_secret,
			parent_vote_root: old_root2,
		}]);

		System::initialize(
			&3,
			&System::parent_hash(),
			&Digest {
				logs: vec![DigestItem::Consensus(
					PARENT_VOTING_KEY_DIGEST,
					ParentVotingKeyDigest { parent_voting_key: Some(parent_key) }.encode(),
				)],
			},
		);
		CurrentTick::set(4);
		TempSealInherent::<Test>::put(BlockSealInherent::Compute);

		// still add both notebooks
		NotebooksAtTick::mutate(|a| {
			a.insert(3, vec![(1, 2, Some(book1_secret))]);
			a.insert(3, vec![(2, 2, Some(book2_secret))]);
		});

		BlockSeal::on_finalize(3);
		assert_eq!(ParentVotingKey::<Test>::get(), Some(parent_key));
	});
}

#[test]
fn it_can_find_best_vote_seal_v2() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		let mut parent_hash = System::parent_hash();
		let authority = default_authority();

		assert_eq!(
			BlockSeal::find_better_vote_block_seal(
				vec![],
				U256::MAX,
				U256::MAX,
				authority.clone(),
				0
			)
			.unwrap(),
			None
		);
		let mut first_vote = BlockVoteT {
			account_id: Bob.public().into(),
			index: 0,
			tick: 1,
			block_hash: parent_hash,
			power: 500,
			block_rewards_account_id: Alice.to_account_id(),
			signature: empty_vote_signature(),
		};
		BestMinerNonce::set(Some((
			MiningAuthority {
				account_id: Alice.into(),
				authority_id: authority.clone(),
				authority_index: (1, 0),
			},
			U256::from(100),
		)));
		AuthorityList::set(vec![(Alice.into(), authority.clone())]);

		let mut vote = NotaryNotebookRawVotes {
			notary_id: 1,
			notebook_number: 1,
			raw_votes: vec![(first_vote.encode(), 500)],
		};
		assert_eq!(
			BlockSeal::find_better_vote_block_seal(
				vec![vote.clone()],
				U256::MAX,
				U256::MAX,
				authority.clone(),
				0
			)
			.unwrap(),
			None,
			"there's not a valid grandpa block or voting key yet"
		);

		for i in 1..=5 {
			System::reset_events();
			System::initialize(&i, &parent_hash, &Default::default());

			let header = System::finalize();
			parent_hash = header.hash();
			System::set_block_number(*header.number());
		}
		CurrentTick::set(5);
		// This api assumes you are building the next block, so the runtime tick will already be -1
		let voting_schedule = VotingSchedule::when_evaluating_runtime_votes(5);
		BlocksAtTick::mutate(|a| {
			for i in 1..5 {
				a.insert(i as Tick, vec![System::block_hash(i)]);
			}
		});

		first_vote.block_hash = System::block_hash(voting_schedule.eligible_votes_tick());

		vote.raw_votes = vec![(first_vote.encode(), 500)];

		ParentVotingKey::<Test>::put(Some(H256::random()));
		assert!(!first_vote.is_proxy_vote());
		assert_eq!(
			BlockSeal::find_better_vote_block_seal(
				vec![vote.clone()],
				U256::MAX,
				U256::MAX,
				authority.clone(),
				voting_schedule.notebook_tick()
			)
			.unwrap(),
			None,
			"vote is for grandparent, but should be for great grandparent"
		);

		first_vote.block_hash = System::block_hash(voting_schedule.grandparent_votes_tick());
		vote.raw_votes = vec![(first_vote.encode(), 500)];
		let best = BlockSeal::find_better_vote_block_seal(
			vec![vote.clone()],
			U256::MAX,
			U256::MAX,
			authority.clone(),
			voting_schedule.notebook_tick(),
		)
		.expect("should return");
		assert!(best.is_some());
		assert_eq!(best.unwrap().block_vote_bytes, first_vote.encode());

		let mut votes = vec![];
		// insert 200 votes
		for i in 2..200 {
			let mut vote = first_vote.clone();
			vote.index = i;
			votes.push(NotaryNotebookRawVotes {
				notary_id: i,
				notebook_number: 1,
				raw_votes: vec![(vote.encode(), 500)],
			});
		}
		let best = BlockSeal::find_better_vote_block_seal(
			votes.clone(),
			U256::MAX,
			U256::MAX,
			authority.clone(),
			voting_schedule.notebook_tick(),
		)
		.expect("should return");
		assert!(best.is_some());
		let strongest = best.clone().unwrap().seal_strength;
		let miner_nonce_score = best.clone().unwrap().miner_nonce_score;
		assert_eq!(best.clone().unwrap().closest_miner.0, Alice.into());
		assert_eq!(best.clone().unwrap().closest_miner.1, authority);
		let voting_key = ParentVotingKey::<Test>::get().unwrap();
		for notebook_vote in &votes {
			for (vote, _) in &notebook_vote.raw_votes {
				let block_vote = BlockVoteT::<H256>::decode(&mut vote.as_slice()).unwrap();
				let calculated = block_vote.get_seal_strength(notebook_vote.notary_id, voting_key);
				println!(
					"{:?}. Strongest {:?}, calculated {:?}",
					block_vote, strongest, calculated
				);
				assert!(calculated >= strongest);
			}
		}
		assert_eq!(
			BlockSeal::find_better_vote_block_seal(
				votes.clone(),
				strongest,
				miner_nonce_score.unwrap().0.add(U256::one()),
				authority.clone(),
				voting_schedule.notebook_tick()
			)
			.expect("")
			.unwrap()
			.miner_nonce_score,
			miner_nonce_score,
			"should return a better nonce score at current best"
		);
		assert_eq!(
			BlockSeal::find_better_vote_block_seal(
				votes.clone(),
				strongest.add(U256::one()),
				miner_nonce_score.unwrap().0,
				authority.clone(),
				voting_schedule.notebook_tick()
			)
			.expect("")
			.unwrap()
			.seal_strength,
			strongest,
			"should return a better strength at current best"
		);
	})
}

#[test]
fn it_allows_any_block_with_default_votes() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		let mut parent_hash = System::parent_hash();

		let authority = default_authority();

		BestMinerNonce::set(Some((
			MiningAuthority {
				account_id: Alice.into(),
				authority_id: authority.clone(),
				authority_index: (1, 0),
			},
			U256::from(100),
		)));
		AuthorityList::set(vec![(Alice.into(), authority.clone())]);

		for i in 1..=5 {
			System::reset_events();
			System::initialize(&i, &parent_hash, &Default::default());

			let header = System::finalize();
			parent_hash = header.hash();
			System::set_block_number(*header.number());
		}
		CurrentTick::set(5);
		// This api assumes you are building the next block, so the runtime tick will already be -1
		let voting_schedule = VotingSchedule::when_evaluating_runtime_votes(5);
		BlocksAtTick::mutate(|a| {
			for i in 1..5 {
				a.insert(i as Tick, vec![System::block_hash(i)]);
			}
		});

		let first_vote = BlockVote::create_default_vote(Bob.public().into(), 5);
		assert!(first_vote.is_proxy_vote());
		assert!(first_vote.is_default_vote());

		ParentVotingKey::<Test>::put(Some(H256::random()));

		let best = BlockSeal::find_better_vote_block_seal(
			vec![NotaryNotebookRawVotes {
				notary_id: 1,
				notebook_number: 1,
				raw_votes: vec![(first_vote.encode(), 0)],
			}],
			U256::MAX,
			U256::MAX,
			authority.clone(),
			voting_schedule.notebook_tick(),
		)
		.expect("should return");
		assert!(best.is_some());
		assert_eq!(best.unwrap().block_vote_bytes, first_vote.encode());
	})
}

#[test]
fn it_checks_v2_tax_votes() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(4);
		let mut vote = BlockVote {
			block_hash: System::block_hash(System::block_number().saturating_sub(4)),
			account_id: Keyring::Alice.into(),
			tick: 1,
			index: 1,
			power: 500,
			block_rewards_account_id: Ferdie.to_account_id(),
			signature: empty_vote_signature(),
		};

		let default_authority = default_authority();
		let author = Alice.to_account_id();
		let tick = 6;
		CurrentTick::set(tick);
		let voting_schedule = VotingSchedule::when_evaluating_runtime_seals(tick);

		BlocksAtTick::mutate(|a| {
			a.insert(voting_schedule.grandparent_votes_tick(), vec![vote.block_hash]);
		});
		GrandpaVoteMinimum::set(Some(501));
		let seal_strength = vote.get_seal_strength(1, H256::random());
		let miner_nonce_score = U256::from(100);
		assert_err!(
			BlockSeal::verify_block_vote(
				seal_strength,
				&vote,
				&author,
				&voting_schedule,
				miner_nonce_score
			),
			Error::<Test>::InsufficientVotingPower
		);
		GrandpaVoteMinimum::set(Some(500));
		assert_err!(
			BlockSeal::verify_block_vote(
				seal_strength,
				&vote,
				&author,
				&voting_schedule,
				miner_nonce_score
			),
			Error::<Test>::BlockVoteInvalidSignature
		);
		vote.sign(Alice.pair());

		assert_err!(
			BlockSeal::verify_block_vote(
				seal_strength,
				&vote,
				&author,
				&voting_schedule,
				miner_nonce_score
			),
			Error::<Test>::UnregisteredBlockAuthor
		);
		AuthorityList::mutate(|a| a.push((Alice.into(), default_authority.clone())));
		assert_err!(
			BlockSeal::verify_block_vote(
				seal_strength,
				&vote,
				&author,
				&voting_schedule,
				miner_nonce_score
			),
			Error::<Test>::NoClosestMinerFoundForVote
		);
		BestMinerNonce::set(Some((
			MiningAuthority {
				account_id: Alice.into(),
				authority_id: default_authority.clone(),
				authority_index: (1, 0),
			},
			U256::from(90),
		)));
		assert_err!(
			BlockSeal::verify_block_vote(
				seal_strength,
				&vote,
				&author,
				&voting_schedule,
				miner_nonce_score
			),
			Error::<Test>::InvalidMinerNonceScore
		);

		BestMinerNonce::set(Some((
			MiningAuthority {
				account_id: Alice.into(),
				authority_id: default_authority.clone(),
				authority_index: (1, 0),
			},
			miner_nonce_score,
		)));
		assert_ok!(BlockSeal::verify_block_vote(
			seal_strength,
			&vote,
			&author,
			&voting_schedule,
			miner_nonce_score
		),);
	});
}

#[test]
fn it_checks_default_votes() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		setup_blocks(2);
		System::set_block_number(4);

		let default_authority = default_authority();
		let author = Alice.to_account_id();
		let tick = 6;
		CurrentTick::set(tick);
		let voting_schedule = VotingSchedule::when_evaluating_runtime_seals(tick);
		let vote =
			BlockVote::create_default_vote(Alice.into(), voting_schedule.eligible_votes_tick());

		let seal_strength = vote.get_seal_strength(1, H256::random());
		// first thing default votes check is the block authority
		assert_err!(
			BlockSeal::verify_block_vote(
				seal_strength,
				&vote,
				&author,
				&voting_schedule,
				U256::one()
			),
			Error::<Test>::UnregisteredBlockAuthor
		);
		AuthorityList::mutate(|a| a.push((Alice.into(), default_authority.clone())));
		BestMinerNonce::set(Some((
			MiningAuthority {
				account_id: Alice.into(),
				authority_id: default_authority.clone(),
				authority_index: (1, 0),
			},
			U256::from(100),
		)));

		assert_ok!(BlockSeal::verify_block_vote(
			seal_strength,
			&vote,
			&author,
			&voting_schedule,
			U256::from(100)
		),);
	});
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
	authority_of(Alice.public())
}

fn authority_of(author: Public) -> BlockSealAuthorityId {
	BlockSealAuthorityId::from(author)
}

fn default_vote() -> BlockVote {
	BlockVote {
		block_hash: System::block_hash(System::block_number().saturating_sub(4)),
		account_id: Keyring::Alice.into(),
		index: 1,
		tick: 1,
		power: 500,
		block_rewards_account_id: Alice.to_account_id(),
		signature: empty_vote_signature(),
	}
	.sign(Alice.pair())
	.clone()
}
fn empty_vote_signature() -> MultiSignature {
	sp_core::sr25519::Signature::from_raw([0u8; 64]).into()
}
