#![allow(clippy::await_holding_lock)]

use std::{future::Future, sync::Arc, task::Context, time::Duration};

use env_logger::{Builder, Env};
use futures::{future, future::BoxFuture, task::Poll, FutureExt, SinkExt, StreamExt};
use parking_lot::Mutex;
use sc_client_api::BlockchainEvents;
use sc_network_test::{PeersFullClient, TestNetFactory};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, NoNetwork as DummyOracle};
use sp_core::{bounded_vec, crypto::AccountId32, sr25519::Signature, H256};
use sp_keyring::sr25519::Keyring;
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystorePtr};
use sp_timestamp::Timestamp;
use substrate_test_runtime::AccountId;

use argon_primitives::{
	block_seal::{BlockSealAuthorityId, BLOCK_SEAL_KEY_TYPE},
	localchain::BlockVote,
	tick::Ticker,
	BestBlockVoteSeal, DataDomain, DataTLD, MerkleProof, TickApis,
};

use crate::{
	block_creator::CreateTaxVoteBlock,
	compute_worker::{create_compute_miner, create_compute_solver_task, MiningHandle},
	tests::mock::{ArgonBlockImport, ArgonTestNet, Config, DummyFactory},
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
	block_import: ArgonBlockImport,
	client: Arc<PeersFullClient>,
}
#[allow(dead_code)]
impl Miner {
	fn new(node: NodeIdentity, net: Arc<Mutex<ArgonTestNet>>) -> Self {
		let mut net = net.lock();

		let peer = net.peer(node.peer_id);
		let client = peer.client().as_client();
		let select_chain = peer.select_chain().expect("full client has a select chain");
		let data = peer.data.as_ref().expect("argon test net data");
		let argon_block_import = data.block_import.clone();
		let environ = DummyFactory(client.clone());
		let api = data.api.clone();
		let aux_client = data.aux_client.clone();
		let uxto_tracker = data.utxo_tracker.clone();
		let (compute_handle, task) = create_compute_miner(
			Box::new(argon_block_import.clone()),
			api.clone(),
			aux_client.clone(),
			select_chain.clone(),
			environ,
			DummyOracle,
			node.account_id,
			(),
			uxto_tracker,
			// how long to take to actually build the block (i.e. executing extrinsics)
			Duration::from_millis(10),
		);

		let compute_task = ReusableFuture::new(task.boxed());

		Self {
			block_import: argon_block_import,
			node_identity: node,
			compute_handle,
			client,
			compute_task,
		}
	}

	async fn mine_block(&mut self, net: &Arc<Mutex<ArgonTestNet>>) {
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
		net: &'a Arc<Mutex<ArgonTestNet>>,
	) -> impl Future<Output = ()> + '_ {
		let compute_future = &mut self.compute_task;
		futures::future::poll_fn(move |cx| {
			let mut net = net.lock();
			let _ = compute_future.poll_future(cx);
			net.check_errors(cx)
		})
	}

	async fn wait_for_external_block(&mut self, net: &Arc<Mutex<ArgonTestNet>>) {
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

	async fn wait_for_block_number(&mut self, net: &Arc<Mutex<ArgonTestNet>>, block_number: u64) {
		let id = self.node_identity.peer_id;
		println!("waiting for block #{}. id={}", block_number, id);
		{
			let mut net = net.lock();
			let peer = net.peer(id);
			let info = peer.client().as_client().info();
			println!("current_block #{}. id={}", info.best_number, id);
			if info.best_number >= block_number {
				return;
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
}

impl NodeIdentity {
	fn new(keyring: Keyring, peer_id: usize) -> Self {
		let keystore = create_keystore(keyring);
		let authority_id: BlockSealAuthorityId =
			keystore.ed25519_public_keys(BLOCK_SEAL_KEY_TYPE)[0].into();
		NodeIdentity { peer_id, keystore, authority_id, account_id: keyring.public() }
	}
}

fn create_node_identities() -> Vec<NodeIdentity> {
	[(Keyring::Alice), (Keyring::Bob), (Keyring::Charlie)]
		.iter()
		.enumerate()
		.map(|(i, keyring)| NodeIdentity::new(*keyring, i))
		.collect::<Vec<_>>()
}

async fn mine_one(
	i: u64,
	miner1: &mut Miner,
	miner2: &mut Miner,
	miner3: &mut Miner,
	net: Arc<Mutex<ArgonTestNet>>,
) {
	if i % 3 == 0 {
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
	net: Arc<Mutex<ArgonTestNet>>,
) {
	future::join_all(vec![
		miner1.wait_for_block_number(&net, block_number),
		miner2.wait_for_block_number(&net, block_number),
		miner3.wait_for_block_number(&net, block_number),
	])
	.await;
}

/// This test is only for testing that the compute engine can work cross-miner without any runtime
/// engine involved.
#[tokio::test]
async fn can_build_compute_blocks() {
	setup_logs();

	let node_identities = create_node_identities();
	let tick_duration = Duration::from_millis(1000);
	let net = ArgonTestNet::new(
		3,
		Config {
			difficulty: 200,
			tax_minimum: 1,
			tick_duration,
			genesis_utc_time: Ticker::start(tick_duration, 2).genesis_utc_time,
		},
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

/// Tests that votes can be constructed with a proof of tax, but nothing about the votes themselves
/// is verified
#[tokio::test]
async fn can_run_proof_of_tax() {
	setup_logs();

	let node_identities = create_node_identities();

	let tick_duration = Duration::from_millis(1000);
	let net = ArgonTestNet::new(
		node_identities.len(),
		Config {
			tax_minimum: 5,
			difficulty: 1,
			tick_duration,
			genesis_utc_time: Ticker::start(tick_duration, 2).genesis_utc_time,
		},
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
		let aux_client = peer.data.as_ref().expect("required").aux_client.clone();
		let utxo_tracker = peer.data.as_ref().expect("required").utxo_tracker.clone();

		let client = peer.client().as_client();

		let parent_number = client.info().best_number;
		let parent_hash = client.info().best_hash;

		let grandparent_hash = client
			.hash(parent_number - 1)
			.expect("should be able to get grandparent")
			.expect("should be a hash");

		let account_id: AccountId32 = Keyring::Dave.public().into();

		let vote = BlockVote {
			block_hash: grandparent_hash,
			account_id: account_id.clone(),
			data_domain_hash: DataDomain::new("delta", DataTLD::Flights).hash(),
			data_domain_account: Keyring::Alice.to_account_id(),
			power: 500,
			index: 1,
			signature: Signature::from_raw([0; 64]).into(),
			block_rewards_account_id: account_id.clone(),
		};
		let parent_voting_key = H256::random();
		let seal_strength = vote.get_seal_strength(1, parent_voting_key);

		let current_tick = api.runtime_api().current_tick(parent_hash).expect("can call the api");

		let closest_miner_id: NodeIdentity = miner1.node_identity.clone();

		let miner_signature = crate::notebook_watch::try_sign_vote(
			&closest_miner_id.keystore,
			&parent_hash,
			&closest_miner_id.authority_id,
			seal_strength,
		)
		.expect("should sign");

		let create_block = CreateTaxVoteBlock {
			timestamp_millis: Timestamp::current().as_millis(),
			parent_hash,
			tick: current_tick + 1,
			vote: BestBlockVoteSeal {
				closest_miner: (closest_miner_id.account_id, closest_miner_id.authority_id),
				seal_strength,
				notary_id: 1,
				block_vote_bytes: vec![],
				source_notebook_number: 1,
				source_notebook_proof: MerkleProof {
					proof: bounded_vec![],
					leaf_index: 0,
					number_of_leaves: 0,
				},
			},
			signature: miner_signature,
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
			aux_client,
			environ,
			(),
			Duration::from_millis(100),
			block_create_stream,
			utxo_tracker,
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
			if poll.is_ready() {
				self.future.take();
			}
			poll
		} else {
			Poll::Pending
		}
	}
}
