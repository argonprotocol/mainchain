use std::{future::Future, sync::Arc, task::Context, time::Duration};

use env_logger::{Builder, Env};
use futures::{future, future::BoxFuture, task::Poll, FutureExt, SinkExt, StreamExt};
use parking_lot::Mutex;
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_network_test::{PeersFullClient, TestNetFactory};
use sp_api::ProvideRuntimeApi;
use sp_consensus::{BlockOrigin, NoNetwork as DummyOracle};
use sp_core::{bounded_vec, crypto::AccountId32, H256};
use sp_keyring::sr25519::Keyring;
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystorePtr};
use sp_timestamp::Timestamp;
use substrate_test_runtime::AccountId;

use ulx_primitives::{
	block_seal::{BlockSealAuthorityId, BLOCK_SEAL_KEY_TYPE},
	digests::BlockVoteDigest,
	inherents::BlockSealInherent,
	localchain::{BlockVote, ChannelPass},
	tick::Ticker,
	BlockSealSpecApis, MerkleProof, MiningAuthorityApis, TickApis,
};

use crate::{
	block_creator::CreateTaxVoteBlock,
	compute_worker::{create_compute_miner, create_compute_solver_task, MiningHandle},
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
	compute_task: ReusableFuture,
	block_import: UlxBlockImport,
	client: Arc<PeersFullClient>,
}
#[allow(dead_code)]
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
			(),
			// how long to take to actually build the block (i.e. executing extrinsics)
			Duration::from_millis(10),
		);

		let compute_task = ReusableFuture::new(task.boxed());

		Self {
			block_import: ulx_block_import,
			node_identity: node,
			compute_handle,
			client,
			compute_task,
		}
	}

	async fn mine_block(&mut self, net: &Arc<Mutex<UlxTestNet>>) {
		let solver = create_compute_solver_task(self.compute_handle.clone());

		println!("start mining for a block");
		let import_notifications = self
			.client
			.import_notification_stream()
			.take_while(move |n| future::ready(n.origin != BlockOrigin::Own))
			.for_each(move |_| future::ready(()));

		future::select(self.check_state(net), future::select(import_notifications, solver)).await;

		println!("mined a block");
	}

	fn check_state<'a>(
		&'a mut self,
		net: &'a Arc<Mutex<UlxTestNet>>,
	) -> impl Future<Output = ()> + '_ {
		let compute_future = &mut self.compute_task;
		futures::future::poll_fn(move |cx| {
			let mut net = net.lock();
			let _ = compute_future.poll_future(cx);
			net.check_errors(cx)
		})
	}

	async fn wait_for_external_block(&mut self, net: &Arc<Mutex<UlxTestNet>>) {
		let id = self.node_identity.peer_id;
		println!("waiting for external block. Id={}", id);
		let stream = self
			.client
			.import_notification_stream()
			.take_while(move |n| {
				future::ready({
					println!("got a block imported. own? {}", n.origin == BlockOrigin::Own);
					n.origin == BlockOrigin::Own
				})
			})
			.for_each(move |_| future::ready(()));
		future::select(self.check_state(net), stream).await;
		println!("waited for external block. Id={}", id);
	}

	async fn wait_for_block_number(&mut self, net: &Arc<Mutex<UlxTestNet>>, block_number: u64) {
		let id = self.node_identity.peer_id;
		println!("waiting for block #{}. id={}", block_number, id);
		{
			let mut net = net.lock();
			let peer = net.peer(id);
			let info = peer.client().as_client().info();
			println!("current_block #{}. id={}", info.best_number, id);
			if info.best_number >= block_number {
				return
			}
		}
		let stream = self
			.client
			.import_notification_stream()
			.take_while(move |n| future::ready(n.header.number < block_number))
			.for_each(move |_| future::ready(()));
		future::select(self.check_state(net), stream).await;
		println!("waited for a block #{}. id={}", block_number, id);
	}
}

#[derive(Clone)]
struct NodeIdentity {
	peer_id: usize,
	keystore: KeystorePtr,
	authority_id: BlockSealAuthorityId,
	account_id: AccountId,
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

async fn mine_one(
	i: u64,
	miner1: &mut Miner,
	miner2: &mut Miner,
	miner3: &mut Miner,
	net: Arc<Mutex<UlxTestNet>>,
) {
	if i % 1 == 0 {
		miner1.mine_block(&net).await;
	} else if i % 2 == 0 {
		miner2.mine_block(&net).await;
	} else {
		miner3.mine_block(&net).await;
	}
}

async fn wait_for_sync(
	block_number: u64,
	miner1: &mut Miner,
	miner2: &mut Miner,
	miner3: &mut Miner,
	net: Arc<Mutex<UlxTestNet>>,
) {
	future::join_all(vec![
		miner1.wait_for_block_number(&net, block_number),
		miner2.wait_for_block_number(&net, block_number),
		miner3.wait_for_block_number(&net, block_number),
	])
	.await;
}

#[tokio::test]
async fn can_build_compute_blocks() {
	setup_logs();

	let node_identities = create_node_identities(true);
	let tick_duration = Duration::from_millis(1000);
	let net = UlxTestNet::new(
		3,
		Config {
			difficulty: 10_000,
			tax_minimum: 1,
			tick_duration: tick_duration.clone(),
			genesis_utc_time: Ticker::start(tick_duration).genesis_utc_time,
			voting_key: None,
		},
		authorities(&node_identities),
	);

	let net = Arc::new(Mutex::new(net));
	let mut miner1 = Miner::new(node_identities[0].clone(), net.clone());
	let mut miner2 = Miner::new(node_identities[1].clone(), net.clone());
	let mut miner3 = Miner::new(node_identities[2].clone(), net.clone());
	{
		let mut net = net.lock();
		let _ = &net.run_until_connected().await;
	}
	for i in 0..5 {
		mine_one(i, &mut miner1, &mut miner2, &mut miner3, net.clone()).await;
		wait_for_sync(i + 1, &mut miner1, &mut miner2, &mut miner3, net.clone()).await;
	}
}

#[tokio::test]
async fn can_run_proof_of_tax() {
	setup_logs();

	let node_identities = create_node_identities(true);

	let tick_duration = Duration::from_millis(1000);
	let net = UlxTestNet::new(
		node_identities.len(),
		Config {
			tax_minimum: 5,
			difficulty: 1,
			tick_duration: tick_duration.clone(),
			genesis_utc_time: Ticker::start(tick_duration).genesis_utc_time,
			voting_key: Some(H256::random()),
		},
		authorities(&node_identities),
	);

	let net = Arc::new(Mutex::new(net));
	let mut miner1 = Miner::new(node_identities[0].clone(), net.clone());
	let mut miner2 = Miner::new(node_identities[1].clone(), net.clone());
	let mut miner3 = Miner::new(node_identities[2].clone(), net.clone());

	{
		let mut net = net.lock();
		let _ = &net.run_until_connected().await;
	}

	miner1.mine_block(&net).await;
	miner1.mine_block(&net).await;
	miner1.mine_block(&net).await;
	wait_for_sync(3, &mut miner1, &mut miner2, &mut miner3, net.clone()).await;

	let raw_net = net.clone();
	let task = {
		// create a closer so the lock doesn't get stuck
		let mut net = net.lock();
		let peer = net.peer(0);
		let api = peer.data.as_ref().expect("required").api.clone();

		let client = peer.client().as_client();

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
		let parent_voting_key = api
			.runtime_api()
			.parent_voting_key(parent_hash)
			.expect("can call the api")
			.unwrap();
		let vote_proof = vote.vote_proof(1, parent_voting_key);
		let closest_peer = api
			.runtime_api()
			.xor_closest_authority(parent_hash, vote_proof)
			.expect("can call the api")
			.unwrap();
		let current_tick = api.runtime_api().current_tick(parent_hash).expect("can call the api");

		let mut closest_miner_id: NodeIdentity = miner1.node_identity.clone();
		if miner2.node_identity.authority_id == closest_peer.authority_id {
			closest_miner_id = miner2.node_identity.clone();
		} else if miner3.node_identity.authority_id == closest_peer.authority_id {
			closest_miner_id = miner3.node_identity.clone();
		}

		let miner_signature = crate::notebook_watch::sign_vote(
			&closest_miner_id.keystore,
			&closest_miner_id.authority_id,
			&parent_hash,
			vote_proof,
		)
		.expect("should sign");

		let create_block = CreateTaxVoteBlock {
			timestamp_millis: Timestamp::current().as_millis(),
			account_id: closest_peer.account_id,
			parent_hash,
			tick: current_tick + 1,
			block_vote_digest: BlockVoteDigest {
				votes_count: 0,
				tick_notebooks: 0,
				parent_voting_key: None,
				voting_power: 0,
			},
			seal_inherent: BlockSealInherent::Vote {
				vote_proof,
				source_notebook_proof: MerkleProof {
					proof: bounded_vec![],
					leaf_index: 0,
					number_of_leaves: 0,
				},
				source_notebook_number: 1,
				notary_id: 1,
				block_vote: vote,
				miner_signature,
			},
			vote_proof,
			latest_finalized_block_needed: 0,
		};
		let miner = match closest_miner_id.peer_id {
			0 => &miner1,
			1 => &miner2,
			2 => &miner3,
			_ => panic!("invalid peer id"),
		};
		let environ = DummyFactory(miner.client.clone());

		let (mut block_create_sink, block_create_stream) = futures::channel::mpsc::channel(1000);
		let task = tax_block_creator(
			Box::new(miner.block_import.clone()),
			api,
			environ,
			(),
			Duration::from_millis(100),
			block_create_stream,
		);
		block_create_sink.send(create_block).await.expect("Submitted block");

		println!("Submitted proof of block to miner {}", closest_miner_id.peer_id);
		task
	};

	futures::future::select(
		task.boxed(),
		wait_for_sync(4, &mut miner1, &mut miner2, &mut miner3, raw_net).boxed(),
	)
	.await;
}
struct ReusableFuture {
	future: Option<BoxFuture<'static, ()>>,
}

impl ReusableFuture {
	fn new(future: BoxFuture<'static, ()>) -> Self {
		Self { future: Some(future) }
	}

	fn poll_future(&mut self, cx: &mut Context<'_>) -> Poll<()> {
		if let Some(future) = self.future.as_mut() {
			let poll = future.as_mut().poll(cx);
			if let Poll::Ready(_) = poll {
				self.future.take();
			}
			poll
		} else {
			Poll::Pending
		}
	}
}
