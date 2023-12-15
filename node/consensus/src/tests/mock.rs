use std::{
	sync::Arc,
	task::{Context, Poll},
	time::Duration,
};

use futures::future;
use log::trace;
use sc_block_builder::BlockBuilderBuilder;
use sc_client_api::{
	AuxStore, BlockOf, BlockchainEvents, FinalityNotifications, ImportNotifications,
	StorageEventStream, StorageKey,
};
use sc_consensus::{
	BlockCheckParams, BlockImport, BlockImportParams, BoxJustificationImport, ImportResult,
	LongestChain,
};
use sc_network_test::{
	Block, BlockImportAdapter, Peer, PeersClient, PeersFullClient, TestNetFactory,
};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_blockchain::{BlockStatus, HeaderBackend, Info};
use sp_consensus::{DisableProofRecording, Environment, Proposal, Proposer};
use sp_core::{blake2_256, crypto::AccountId32, ByteArray, OpaquePeerId, H256, U256};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{traits::Block as BlockT, ApplyExtrinsicResult, BoundedVec, Digest};
use substrate_test_runtime::{AccountId, BlockNumber, Executive, Hash, Header};
use substrate_test_runtime_client::Backend;
use ulx_primitives::notary::NotaryNotebookVoteDigestDetails;

use pallet_mining_slot::find_xor_closest;
use ulx_primitives::{
	block_seal::{BlockSealAuthorityId, Host, MiningAuthority, PeerId},
	ComputeDifficulty, VoteMinimum,
};

use crate::import_queue;
use sp_runtime::traits::UniqueSaturatedInto;
use ulx_primitives::{
	digests::BlockVoteDigest,
	tick::{Tick, Ticker},
};

type Error = sp_blockchain::Error;

pub(crate) struct DummyFactory(pub Arc<PeersFullClient>);

pub(crate) struct DummyProposer(BlockNumber, Arc<PeersFullClient>);

impl Environment<Block> for DummyFactory {
	type Proposer = DummyProposer;
	type CreateProposer = future::Ready<Result<DummyProposer, Error>>;
	type Error = Error;

	fn init(&mut self, parent_header: &<Block as BlockT>::Header) -> Self::CreateProposer {
		future::ready(Ok(DummyProposer(parent_header.number + 1, self.0.clone())))
	}
}

impl Proposer<Block> for DummyProposer {
	type Error = Error;
	type Proposal = future::Ready<Result<Proposal<Block, ()>, Error>>;
	type ProofRecording = DisableProofRecording;
	type Proof = ();

	fn propose(
		self,
		_: InherentData,
		digests: Digest,
		_: Duration,
		_: Option<usize>,
	) -> Self::Proposal {
		let r = BlockBuilderBuilder::new(&*self.1)
			.on_parent_block(self.1.chain_info().best_hash)
			.fetch_parent_block_number(&*self.1)
			.unwrap()
			.with_inherent_digests(digests)
			.build()
			.unwrap()
			.build();

		future::ready(r.map(|b| Proposal {
			block: b.block,
			proof: (),
			storage_changes: b.storage_changes,
		}))
	}
}

pub(crate) type UlxVerifier = import_queue::UlxVerifier<Block>;
pub(crate) type UlxBlockImport = PanickingBlockImport<
	import_queue::UlxBlockImport<
		Block,
		BlockImportAdapter<PeersClient>,
		TestApi,
		LongestChain<Backend, Block>,
	>,
>;
pub(crate) struct PeerData {
	pub block_import: UlxBlockImport,
	pub api: Arc<TestApi>,
}
pub(crate) type UlxPeer = Peer<Option<PeerData>, UlxBlockImport>;

#[derive(Default)]
pub(crate) struct UlxTestNet {
	pub peers: Vec<UlxPeer>,
	pub config: Config,
	pub authorities: Vec<(u16, AccountId, BlockSealAuthorityId)>,
}

impl UlxTestNet {
	pub(crate) fn new(
		n_authority: usize,
		config: Config,
		authorities: Vec<(u16, AccountId, BlockSealAuthorityId)>,
	) -> Self {
		trace!("Creating test network");
		let mut net = UlxTestNet { peers: Vec::with_capacity(n_authority), config, authorities };

		for i in 0..n_authority {
			trace!("Adding peer {}", i);
			net.add_full_peer();
		}
		net
	}

	pub(crate) fn check_errors(&mut self, cx: &mut Context) -> Poll<()> {
		self.poll(cx);
		for p in self.peers() {
			for (h, e) in p.failed_verifications() {
				panic!("Verification failed for {:?}: {}", h, e);
			}
		}
		Poll::<()>::Pending
	}
}

#[derive(Clone, Default)]
pub(crate) struct Config {
	pub difficulty: ComputeDifficulty,
	pub tax_minimum: VoteMinimum,
	pub tick_duration: Duration,
	pub genesis_utc_time: u64,

	pub voting_key: Option<H256>,
}

#[derive(Clone, Default)]
pub(crate) struct TestApi {
	config: Config,
	authorities: Vec<(u16, AccountId, BlockSealAuthorityId)>,
	client: Option<Arc<PeersFullClient>>,
}

// compiler gets confused and warns us about unused inner
#[allow(dead_code)]
pub(crate) struct RuntimeApi {
	inner: TestApi,
}

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = RuntimeApi;
	fn runtime_api(&self) -> ApiRef<Self::Api> {
		RuntimeApi { inner: self.clone() }.into()
	}
}

impl HeaderBackend<Block> for TestApi {
	fn header(&self, hash: Hash) -> sc_client_api::blockchain::Result<Option<Header>> {
		self.client.as_ref().unwrap().header(hash)
	}

	fn info(&self) -> Info<Block> {
		self.client.as_ref().unwrap().info()
	}

	fn status(&self, hash: Hash) -> sc_client_api::blockchain::Result<BlockStatus> {
		self.client.as_ref().unwrap().status(hash)
	}

	fn number(&self, hash: Hash) -> sc_client_api::blockchain::Result<Option<BlockNumber>> {
		self.client.as_ref().unwrap().number(hash)
	}

	fn hash(&self, number: BlockNumber) -> sc_client_api::blockchain::Result<Option<Hash>> {
		self.client.as_ref().unwrap().hash(number)
	}
}

impl BlockOf for TestApi {
	type Type = Block;
}

impl AuxStore for TestApi {
	fn insert_aux<
		'a,
		'b: 'a,
		'c: 'a,
		I: IntoIterator<Item = &'a (&'c [u8], &'c [u8])>,
		D: IntoIterator<Item = &'a &'b [u8]>,
	>(
		&self,
		insert: I,
		delete: D,
	) -> sp_blockchain::Result<()> {
		self.client.as_ref().unwrap().insert_aux(insert, delete)
	}
	fn get_aux(&self, key: &[u8]) -> sp_blockchain::Result<Option<Vec<u8>>> {
		self.client.as_ref().unwrap().get_aux(key)
	}
}

impl BlockchainEvents<Block> for TestApi {
	fn import_notification_stream(&self) -> ImportNotifications<Block> {
		self.client.as_ref().unwrap().import_notification_stream()
	}

	fn every_import_notification_stream(&self) -> ImportNotifications<Block> {
		self.client.as_ref().unwrap().every_import_notification_stream()
	}

	fn finality_notification_stream(&self) -> FinalityNotifications<Block> {
		self.client.as_ref().unwrap().finality_notification_stream()
	}

	fn storage_changes_notification_stream(
		&self,
		filter_keys: Option<&[StorageKey]>,
		child_filter_keys: Option<&[(StorageKey, Option<Vec<StorageKey>>)]>,
	) -> sc_client_api::blockchain::Result<StorageEventStream<Hash>> {
		self.client
			.as_ref()
			.unwrap()
			.storage_changes_notification_stream(filter_keys, child_filter_keys)
	}
}

sp_api::mock_impl_runtime_apis! {
	impl ulx_primitives::BlockSealSpecApis<Block> for RuntimeApi {
		fn vote_minimum() -> VoteMinimum {
			self.inner.config.tax_minimum
		}
		fn compute_difficulty() -> u128{
			self.inner.config.difficulty
		}
		fn parent_voting_key() -> Option<H256> {
			self.inner.config.voting_key
		}
		fn create_vote_digest(_tick_notebooks: Vec<NotaryNotebookVoteDigestDetails>) -> BlockVoteDigest {
			BlockVoteDigest {
				tick_notebooks: 0,
				voting_power: 0,
				parent_voting_key: None,
				votes_count: 0
			}
		}
	}

	impl ulx_primitives::TickApis<Block> for RuntimeApi {
		fn current_tick() -> Tick {
			Ticker::new(self.inner.config.tick_duration.as_millis() as u64, self.inner.config.genesis_utc_time).current() -1
		}
		fn ticker() -> Ticker {
			Ticker::new(self.inner.config.tick_duration.as_millis() as u64, self.inner.config.genesis_utc_time)
		}
	}

	impl ulx_primitives::MiningAuthorityApis<Block> for RuntimeApi {

		fn xor_closest_authority(nonce: U256) -> Option<MiningAuthority<BlockSealAuthorityId, AccountId32>> {
			let closest = find_xor_closest(self.inner.authorities.iter().map(|(index, _, id)| {
				((*index).unique_saturated_into(), ( id.clone(), U256::from( blake2_256(&id.as_slice()[..]))))
			}).collect::<Vec<_>>(), nonce);


			closest.map(|(a, index)| {
				let account_id: AccountId32 = self.inner.authorities.iter().find(|(i, _, _)| *i as u32 == index).unwrap().1.clone().into();
				MiningAuthority::<_, _> {
					account_id,
					authority_id: a.clone(),
					authority_index: index.unique_saturated_into(),
					peer_id: PeerId(OpaquePeerId::default()),
					rpc_hosts: BoundedVec::truncate_from(vec![Host{ ip:0, port: 1, is_secure:false}]),
				}
			})
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for RuntimeApi {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(_data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			vec![]
		}

		fn check_inherents(_block: Block, _data: InherentData) -> CheckInherentsResult {
			CheckInherentsResult::new()
		}
	}
}
#[derive(Clone)]
pub struct PanickingBlockImport<B>(B);

#[async_trait::async_trait]
impl<B: BlockImport<Block>> BlockImport<Block> for PanickingBlockImport<B>
where
	B: Send,
{
	type Error = B::Error;

	async fn import_block(
		&mut self,
		block: BlockImportParams<Block>,
	) -> Result<ImportResult, Self::Error> {
		Ok(self.0.import_block(block).await.expect("importing block failed"))
	}

	async fn check_block(
		&mut self,
		block: BlockCheckParams<Block>,
	) -> Result<ImportResult, Self::Error> {
		Ok(self.0.check_block(block).await.expect("checking block failed"))
	}
}

impl TestNetFactory for UlxTestNet {
	type Verifier = UlxVerifier;
	type BlockImport = UlxBlockImport;
	type PeerData = Option<PeerData>;

	fn make_verifier(&self, _client: PeersClient, _peer_data: &Self::PeerData) -> Self::Verifier {
		UlxVerifier::new()
	}

	fn peer(&mut self, i: usize) -> &mut UlxPeer {
		&mut self.peers[i]
	}

	fn peers(&self) -> &Vec<UlxPeer> {
		&self.peers
	}

	fn peers_mut(&mut self) -> &mut Vec<UlxPeer> {
		&mut self.peers
	}

	fn mut_peers<F: FnOnce(&mut Vec<UlxPeer>)>(&mut self, closure: F) {
		closure(&mut self.peers);
	}

	fn make_block_import(
		&self,
		client: PeersClient,
	) -> (
		BlockImportAdapter<Self::BlockImport>,
		Option<BoxJustificationImport<Block>>,
		Self::PeerData,
	) {
		let inner = BlockImportAdapter::new(client.clone());
		let select_chain = LongestChain::new(client.as_backend());
		let api = TestApi {
			client: Some(client.as_client().clone()),
			config: self.config.clone(),
			authorities: self.authorities.clone(),
		};
		let api = Arc::new(api);
		let block_import = PanickingBlockImport(import_queue::UlxBlockImport::new(
			inner,
			api.clone(),
			select_chain,
		));

		(BlockImportAdapter::new(block_import.clone()), None, Some(PeerData { block_import, api }))
	}
}
