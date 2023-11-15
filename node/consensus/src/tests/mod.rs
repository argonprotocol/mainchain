use codec::Encode;
use env_logger::{Builder, Env};
use futures::{channel::oneshot, future, task::Poll, SinkExt, StreamExt};
use log::info;
use parking_lot::Mutex;
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_network_test::TestNetFactory;
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::AppCrypto;
use sp_consensus::{BlockOrigin, NoNetwork as DummyOracle};
use sp_core::{blake2_256, crypto::AccountId32, ByteArray, U256};
use sp_keyring::sr25519::Keyring;
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystorePtr};
use sp_runtime::{traits::Header as HeaderT, BoundedVec};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use ulx_primitives::{
	block_seal::{
		BlockProof, BlockSealAuthorityId, SealNonceHashMessage, SealStamper,
		SealerSignatureMessage, BLOCK_SEAL_KEY_TYPE, SEAL_NONCE_PREFIX,
	},
	BlockSealDigest, MiningAuthorityApis, ProofOfWorkType,
};
use ulx_voter::compute_worker::{run_miner_thread, UntilImportedOrTimeout};

use crate::{
	block_creator,
	rpc_block_votes::SealNewBlock,
	tests::mock::{Config, DummyFactory, UlxTestNet},
};

use super::{
	create_compute_miner, inherents::UlxCreateInherentDataProviders, nonce_verify::UlxNonce,
};

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

#[tokio::test]
async fn can_run_proof_of_tax() {
	setup_logs();

	let peers = &[(Keyring::Alice), (Keyring::Bob), (Keyring::Charlie)]
		.into_iter()
		.map(|keyring| {
			let keystore = create_keystore(keyring.clone());
			let authority_id: BlockSealAuthorityId =
				keystore.ed25519_public_keys(BLOCK_SEAL_KEY_TYPE)[0].into();
			(keyring, keystore, authority_id)
		})
		.collect::<Vec<(Keyring, KeystorePtr, BlockSealAuthorityId)>>();

	let mut counter = 0u16;
	let net = UlxTestNet::new(
		peers.len(),
		Config {
			closest_xor: 2,
			difficulty: 1,
			work_type: ProofOfWorkType::Tax,
			easing: 0,
			min_seal_signers: 2,
		},
		peers
			.iter()
			.map(|(keyring, _, authority_id)| {
				counter += 2;
				(counter, keyring.public().into(), authority_id.clone())
			})
			.collect::<Vec<_>>(),
	);

	let net = Arc::new(Mutex::new(net));
	let mut ulx_futures = Vec::new();
	let mut sink_by_peer_id = BTreeMap::new();

	for (peer_id, (key, keystore, _)) in peers.into_iter().enumerate() {
		let mut net = net.lock();
		let peer = net.peer(peer_id);
		let client = peer.client().as_client();
		let select_chain = peer.select_chain().expect("full client has a select chain");

		let data = peer.data.as_ref().expect("peer data set up during initialization");
		let ulx_block_import = data
			.block_import
			.lock()
			.take()
			.expect("block import set up during initialization");
		let environ = DummyFactory(client.clone());

		// Channel for the rpc handler to communicate with the authorship task.
		let (block_proof_sink, block_proof_stream) = futures::channel::mpsc::channel(1000);
		sink_by_peer_id.insert(peer_id, block_proof_sink);
		let api = data.api.clone();
		let algorithm = UlxNonce::new(api.clone());
		let task = block_creator(
			ulx_block_import,
			api.clone(),
			select_chain.clone(),
			algorithm.clone(),
			environ,
			DummyOracle,
			(),
			key.to_account_id().clone(),
			UlxCreateInherentDataProviders::new(),
			// time to wait for a new block before starting to mine a new one
			Duration::from_secs(0),
			// how long to take to actually build the block (i.e. executing extrinsics)
			block_proof_stream,
			keystore.clone(),
		);
		let future = tokio::spawn(async move {
			task.await;
		});
		ulx_futures.push(future);
	}

	let (nonce, parent_hash, block_proof, closest_peer_id) = {
		let mut net = net.lock();
		let peer = &net.peer(0);

		let client = peer.client().as_client();
		let mut timer = UntilImportedOrTimeout::new(
			client.import_notification_stream(),
			Duration::from_secs(1),
		);
		if timer.next().await.is_none() {
			panic!("No block imported in time")
		}

		let parent_hash = client.info().best_hash;

		let author_id: AccountId32 = Keyring::Dave.public().into();

		let xor_closest_peers = peer
			.data
			.as_ref()
			.unwrap()
			.api
			.runtime_api()
			.block_peers(parent_hash, author_id.clone())
			.expect("Got closest");

		let xor_closest_authority_ids = xor_closest_peers
			.clone()
			.into_iter()
			.map(|a| a.authority_id)
			.collect::<Vec<_>>();

		let mut block_proof = BlockProof {
			tax_proof_id: 22343,
			author_id: author_id.clone(),
			seal_stampers: xor_closest_peers
				.clone()
				.iter()
				.map(|a| SealStamper { authority_idx: a.authority_index, signature: None })
				.collect::<Vec<_>>(),
			tax_amount: 1,
		};

		let peer_signature_message = blake2_256(
			SealerSignatureMessage {
				tax_proof_id: block_proof.tax_proof_id,
				parent_hash,
				author_id: author_id.clone(),
				tax_amount: block_proof.tax_amount,
				seal_stampers: xor_closest_authority_ids.clone(),
				prefix: SEAL_NONCE_PREFIX,
			}
			.encode()
			.as_slice(),
		);

		for (i, xor_peer) in xor_closest_peers.iter().enumerate() {
			let (_, keystore, authority_id) =
				peers.iter().find(|(_, _, id)| id == &xor_peer.authority_id).unwrap();
			block_proof.seal_stampers[i].signature = Some(BoundedVec::truncate_from(
				keystore
					.sign_with(
						<BlockSealAuthorityId as AppCrypto>::ID,
						<BlockSealAuthorityId as AppCrypto>::CRYPTO_ID,
						authority_id.as_slice(),
						peer_signature_message.as_ref(),
					)
					.ok()
					.unwrap()
					.unwrap(),
			));
		}

		let nonce = SealNonceHashMessage {
			prefix: SEAL_NONCE_PREFIX,
			tax_proof_id: block_proof.tax_proof_id,
			tax_amount: block_proof.tax_amount,
			parent_hash,
			author_id,
			seal_stampers: block_proof.seal_stampers.clone(),
		}
		.using_encoded(blake2_256);

		let closest_peer_id = peers
			.iter()
			.position(|(_, _, authority_id)| xor_closest_peers[0].authority_id == *authority_id)
			.expect("Should find the closest xor peer");

		(nonce, parent_hash, block_proof, closest_peer_id)
	};

	let sink = sink_by_peer_id.get_mut(&closest_peer_id).unwrap();
	let (sender, receiver) = oneshot::channel();
	let command = SealNewBlock::Submit {
		block_proof,
		sender: Some(sender),
		nonce: U256::from(nonce),
		parent_hash,
	};

	{
		let mut net = net.lock();
		let _ = &net.run_until_connected().await;

		sink.send(command).await.expect("Submitted seal");
		info!(
			"Submitted proof of block to {:?} {}",
			closest_peer_id,
			net.peers[closest_peer_id].id()
		);
	}
	let block_hash = match receiver.await {
		Ok(Ok(rx)) => rx,
		Ok(Err(e)) => panic!("Error receiving block hash: {}", e),
		Err(e) => panic!("Error receiving block hash: {}", e),
	};

	future::select(
		futures::future::poll_fn(move |cx| {
			let mut net = net.lock();
			net.poll(cx);
			let mut unsynched = net.peers().len().clone();
			for p in net.peers() {
				if p.has_block(block_hash.hash.clone()) {
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
		future::join_all(ulx_futures),
	)
	.await;
}
