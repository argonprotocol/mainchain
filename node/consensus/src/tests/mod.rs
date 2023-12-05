use std::{collections::BTreeMap, sync::Arc, time::Duration};

use env_logger::{Builder, Env};
use futures::{future, future::BoxFuture, task::Poll, FutureExt, SinkExt, StreamExt};
use log::info;
use parking_lot::Mutex;
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_network_test::{PeersFullClient, TestNetFactory};
use sp_api::ProvideRuntimeApi;
use sp_consensus::{BlockOrigin, NoNetwork as DummyOracle};
use sp_core::{bounded_vec, crypto::AccountId32, H256};
use sp_keyring::sr25519::Keyring;
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystorePtr};
use sp_runtime::traits::Header as HeaderT;
use substrate_test_runtime::AccountId;

use ulx_node_runtime::BlockNumber;
use ulx_primitives::{
	block_seal::{BlockSealAuthorityId, BLOCK_SEAL_KEY_TYPE},
	digests::BlockVoteDigest,
	inherents::BlockSealInherent,
	localchain::{BlockVote, ChannelPass},
	BlockSealMinimumApis, MerkleProof, MiningAuthorityApis,
};

use crate::{
	block_creator::CreateBlockEvent,
	compute_worker::{
		create_compute_miner, create_compute_solver_task, MiningHandle, UntilImportedOrTimeout,
	},
	tests::mock::{Config, DummyFactory, UlxBlockImport, UlxTestNet},
};

use super::tax_block_creator;

pub(crate) mod mock;

fn create_keystore(authority: Keyring) -> KeystorePtr {
	let keystore = MemoryKeystore::new();
	keystore
		.ed25519_generate_new(BLOCK_SEAL_KEY_TYPE, Some(&authority.to_seed()))
		.expect("Creates authority key");
	keystore.into()
}

pub fn setup_logs() {
	let env = Env::new().default_filter_or("node=debug"); //info,sync=debug,sc_=debug,sub-libp2p=debug,node=debug,runtime=debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	sp_tracing::try_init_simple();
}

type TestBlock = substrate_test_runtime::Block;

struct Miner {
	node_identity: NodeIdentity,
	compute_handle: MiningHandle<TestBlock, (), ()>,
	compute_task: BoxFuture<'static, ()>,
	block_import: UlxBlockImport,
	client: Arc<PeersFullClient>,
}

impl Miner {
	fn new(node: NodeIdentity, net: Arc<Mutex<UlxTestNet>>) -> Self {
		let mut net = net.lock();

		let peer = net.peer(node.peer_id);
		let client = peer.client().as_client();
		let select_chain = peer.select_chain().expect("full client has a select chain");
		let data = peer.data.as_ref().expect("ulx test net data");
		let ulx_block_import = data.block_import.clone();
		let environ = DummyFactory(client.clone());
		let api = data.api.clone();
		let (compute_handle, task) = create_compute_miner(
			Box::new(ulx_block_import.clone()),
			api.clone(),
			select_chain.clone(),
			environ,
			DummyOracle,
			node.account_id.clone().into(),
			node.keystore.clone(),
			(),
			// time to wait for a new block before starting to mine a new one
			Duration::from_millis(100),
			// how long to take to actually build the block (i.e. executing extrinsics)
			Duration::from_millis(500),
		);

		Self {
			block_import: ulx_block_import,
			node_identity: node,
			compute_handle,
			client,
			compute_task: task.boxed(),
		}
	}
	async fn wait_for_block(&self, block_number: u64) {
		self.client
			.import_notification_stream()
			.take_while(move |n| future::ready(n.header.number() < &block_number))
			.for_each(move |_| future::ready(()))
			.await;
	}
}

#[derive(Clone)]
struct NodeIdentity {
	peer_id: usize,
	keystore: KeystorePtr,
	authority_id: BlockSealAuthorityId,
	account_id: AccountId,
	keyring: Keyring,
	authority_index: Option<u16>,
}

impl NodeIdentity {
	fn new(keyring: Keyring, peer_id: usize, registered_authorities: bool) -> Self {
		let keystore = create_keystore(keyring.clone());
		let authority_id: BlockSealAuthorityId =
			keystore.ed25519_public_keys(BLOCK_SEAL_KEY_TYPE)[0].into();
		NodeIdentity {
			peer_id,
			authority_index: if registered_authorities {
				Some(peer_id as u16 * 2u16)
			} else {
				None
			},
			keyring,
			keystore,
			authority_id,
			account_id: keyring.public().into(),
		}
	}
	fn as_authority(&self) -> (u16, AccountId, BlockSealAuthorityId) {
		(
			self.authority_index.unwrap_or_default(),
			self.account_id.clone(),
			self.authority_id.clone(),
		)
	}
}

fn authorities(peers: &Vec<NodeIdentity>) -> Vec<(u16, AccountId, BlockSealAuthorityId)> {
	peers.into_iter().map(|p| p.as_authority()).collect()
}

fn create_node_identities(registered_authorities: bool) -> Vec<NodeIdentity> {
	[(Keyring::Alice), (Keyring::Bob), (Keyring::Charlie)]
		.iter()
		.enumerate()
		.map(|(i, keyring)| NodeIdentity::new(keyring.clone(), i, registered_authorities))
		.collect::<Vec<_>>()
}

#[tokio::test]
async fn can_build_compute_blocks() {
	setup_logs();

	let mut import_notifications = Vec::new();
	let mut ulx_futures = Vec::new();

	let node_identities = create_node_identities(true);
	let net = UlxTestNet::new(
		3,
		Config { difficulty: 16u128 ^ 30, tax_minimum: 1 },
		authorities(&node_identities),
	);

	let net = Arc::new(Mutex::new(net));
	let miners = &node_identities
		.iter()
		.map(|id| Miner::new(id.clone(), net.clone()))
		.collect::<Vec<_>>();

	for miner in miners {
		let mut got_own = false;
		let mut got_other = false;

		import_notifications.push(
			miner
				.client
				.import_notification_stream()
				.take_while(move |n| {
					future::ready(
						n.header.number() < &5 || {
							if n.origin == BlockOrigin::Own {
								got_own = true;
							} else {
								got_other = true;
							}

							// continue until we have at least one block of our own
							// and one of another peer.
							!(got_own && got_other)
						},
					)
				})
				.for_each(move |_| future::ready(())),
		);

		ulx_futures.push(miner.compute_task);
		let task2 = create_compute_solver_task(miner.compute_handle.clone());
		ulx_futures.push(task2);
	}

	future::select(
		futures::future::poll_fn(move |cx| {
			let mut net = net.lock();
			net.poll(cx);
			for p in net.peers() {
				for (h, e) in p.failed_verifications() {
					panic!("Verification failed for {:?}: {}", h, e);
				}
			}

			Poll::<()>::Pending
		}),
		future::select(future::join_all(import_notifications), future::join_all(ulx_futures)),
	)
	.await;
}

#[tokio::test]
async fn can_run_proof_of_tax() {
	setup_logs();

	let node_identities = create_node_identities(true);

	let net = UlxTestNet::new(
		node_identities.len(),
		Config { tax_minimum: 5, difficulty: 1 },
		authorities(&node_identities),
	);

	let net = Arc::new(Mutex::new(net));
	let mut ulx_futures = Vec::new();
	let mut sink_by_peer_id = BTreeMap::new();

	let miners = &node_identities
		.iter()
		.map(|id| Miner::new(id.clone(), net.clone()))
		.collect::<Vec<_>>();

	for miner in miners {
		let mut net = net.lock();
		let node_identity = &miner.node_identity;
		let peer_id = node_identity.peer_id;
		let peer = net.peer(peer_id);
		// Channel for the rpc handler to communicate with the authorship task.
		let (block_create_sink, block_create_stream) = futures::channel::mpsc::channel(1000);
		sink_by_peer_id.insert(peer_id, block_create_sink);

		let api = peer.data.as_ref().expect("required").api.clone();
		let environ = DummyFactory(miner.client.clone());

		let task = tax_block_creator(
			Box::new(miner.block_import.clone()),
			api,
			environ,
			(),
			node_identity.account_id.into(),
			Duration::from_millis(100),
			block_create_stream,
			node_identity.keystore.clone(),
		);
		ulx_futures.push(task);
	}

	let (create_block, closest_peer) = {
		let mut net = net.lock();
		let peer = net.peer(0);
		let api = peer.data.as_ref().expect("required").api.clone();

		let client = peer.client().as_client();
		let mut timer = UntilImportedOrTimeout::new(
			client.import_notification_stream(),
			Duration::from_secs(1),
		);
		if timer.next().await.is_none() {
			panic!("No block imported in time")
		}

		let parent_number = client.info().best_number;
		let parent_hash = client.info().best_hash;

		let grandparent_hash = client
			.hash(parent_number - 1)
			.expect("should be able to get grandparent")
			.expect("should be a hash");

		let account_id: AccountId32 = Keyring::Dave.public().into();

		let vote = BlockVote {
			grandparent_block_hash: grandparent_hash,
			account_id: account_id.clone(),
			channel_pass: ChannelPass {
				at_block_height: 1,
				id: 1,
				zone_record_hash: H256::random(),
				miner_index: 0,
			},
			power: 500,
			index: 1,
		};
		let closest_peer = api
			.runtime_api()
			.block_peer(grandparent_hash, account_id.clone())
			.expect("can call the api")
			.unwrap()
			.authority_id;
		let parent_voting_key = api
			.runtime_api()
			.parent_voting_key(parent_hash)
			.expect("can call the api")
			.unwrap();
		let nonce = vote.calculate_block_nonce(1, parent_voting_key);
		let create_block = CreateBlockEvent {
			parent_hash,
			parent_block_number: parent_number.clone() as BlockNumber,
			block_vote_digest: BlockVoteDigest {
				votes_count: 0,
				notebook_numbers: bounded_vec![],
				parent_voting_key: None,
				voting_power: 0,
			},
			block_seal_authority: closest_peer.clone(),
			seal_inherent: BlockSealInherent::Vote {
				nonce,
				source_notebook_proof: MerkleProof {
					proof: bounded_vec![],
					leaf_index: 0,
					number_of_leaves: 0,
				},
				source_notebook_number: 1,
				notary_id: 1,
				block_vote: vote,
			},
			nonce,
			latest_finalized_block_needed: 0,
		};

		(create_block, closest_peer)
	};
	// TODO: send vote to peer
	// TODO: need to submit vote digests to two previous blocks. This needs to be in grandparent
	//	 votes

	let miner = miners.iter().find(|a| a.node_identity.authority_id == closest_peer).unwrap();

	let closest_peer_id = miner.node_identity.peer_id;
	let mut block_hash = Arc::new(Mutex::new(None));
	let next_block = miner
		.client
		.import_notification_stream()
		.take_while(move |n| {
			let is_ready = {
				if n.origin == BlockOrigin::Own {
					let mut block_hash = block_hash.lock();
					*block_hash = Some(n.hash);
					true
				} else {
					false
				}
			};
			future::ready(is_ready)
		})
		.for_each(move |_| future::ready(()));

	let sink = sink_by_peer_id.get_mut(&closest_peer_id).unwrap();

	{
		let mut net = net.lock();
		let _ = &net.run_until_connected().await;

		sink.send(create_block).await.expect("Submitted block");
		info!(
			"Submitted proof of block to {:?} {}",
			closest_peer_id,
			net.peers[closest_peer_id].id()
		);
	}

	future::select(
		futures::future::poll_fn(move |cx| {
			let Some(block_hash) = block_hash.lock().as_ref().cloned() else {
				return Poll::<()>::Pending
			};
			let mut net = net.lock();
			net.poll(cx);
			let mut unsynched = net.peers().len().clone();
			for p in net.peers() {
				if p.has_block(block_hash) {
					unsynched -= 1;
				}
				for (h, e) in p.failed_verifications() {
					panic!("Verification failed for {:?}: {}", h, e);
				}
			}
			if unsynched == 0 {
				return Poll::<()>::Ready(())
			}

			Poll::<()>::Pending
		}),
		future::select(future::join_all(ulx_futures), future::join_all(vec![next_block])),
	)
	.await;
}
