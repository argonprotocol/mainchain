use std::{
	collections::BTreeMap,
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
use sp_core::{ConstU32, H256, U256};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
	traits::Block as BlockT, ApplyExtrinsicResult, BoundedVec, Digest, DispatchError,
};
use substrate_test_runtime::{AccountId, BlockNumber, Executive, Hash, Header};
use substrate_test_runtime_client::Backend;

use ulx_node_runtime::{NotaryRecordT, NotebookVerifyError};
use ulx_primitives::{
	block_seal::BlockSealAuthorityId,
	digests::BlockVoteDigest,
	notary::{NotaryNotebookVoteDetails, NotaryNotebookVoteDigestDetails},
	tick::{Tick, Ticker},
	BestBlockVoteSeal, ComputeDifficulty, HashOutput, NotaryId, NotaryNotebookVotes,
	NotebookAuditSummary,NotebookAuditResult, NotebookNumber, VoteMinimum,
};

use crate::{aux_client::UlxAux, import_queue};

type Error = sp_blockchain::Error;

pub(crate) struct DummyFactory(pub Arc<PeersFullClient>);

#[allow(dead_code)]
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

		future::ready(r.map(|b| {
			let block = b.block;
			Proposal { block, proof: (), storage_changes: b.storage_changes }
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
		AccountId,
	>,
>;
pub(crate) struct PeerData {
	pub block_import: UlxBlockImport,
	pub api: Arc<TestApi>,
	pub aux_client: UlxAux<Block, TestApi>,
}
pub(crate) type UlxPeer = Peer<Option<PeerData>, UlxBlockImport>;

#[derive(Default)]
pub(crate) struct UlxTestNet {
	pub peers: Vec<UlxPeer>,
	pub config: Config,
}

impl UlxTestNet {
	pub(crate) fn new(n_authority: usize, config: Config) -> Self {
		trace!("Creating test network");
		let mut net = UlxTestNet { peers: Vec::with_capacity(n_authority), config };

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
}

#[derive(Clone, Default)]
pub(crate) struct TestApi {
	config: Config,
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
	impl ulx_primitives::BlockSealApis<Block, AccountId, BlockSealAuthorityId> for RuntimeApi {
		fn vote_minimum() -> VoteMinimum {
			self.inner.config.tax_minimum
		}
		fn compute_difficulty() -> u128{
			self.inner.config.difficulty
		}
		fn create_vote_digest(_tick: Tick, included_notebooks: Vec<NotaryNotebookVoteDigestDetails>) -> BlockVoteDigest {
			let mut digest = BlockVoteDigest {
				voting_power: 0,
				votes_count: 0,
			};

			for notebook in included_notebooks {
				digest.voting_power += notebook.block_voting_power;
				digest.votes_count += notebook.block_votes_count;
			}

			digest
		}
		fn find_vote_block_seals(
			_votes: Vec<NotaryNotebookVotes>,
			_: U256
		) -> Result<BoundedVec<BestBlockVoteSeal<AccountId, BlockSealAuthorityId>, ConstU32<2>>, DispatchError> {
			Ok(BoundedVec::truncate_from(vec![]))
		}
	}

	impl ulx_primitives::NotaryApis<Block, NotaryRecordT> for RuntimeApi {
		fn notary_by_id(_notary_id: NotaryId) -> Option<NotaryRecordT> {
			None
		}
		fn notaries() -> Vec<NotaryRecordT> {
			Vec::new()
		}
	}

	impl ulx_primitives::NotebookApis<Block, NotebookVerifyError> for RuntimeApi {
		fn audit_notebook_and_get_votes(
			_version: u32,
			_notary_id: NotaryId,
			_notebook_number: NotebookNumber,
			_header_hash: H256,
			_vote_minimums: &BTreeMap<HashOutput, VoteMinimum>,
			_bytes: &Vec<u8>,
			_audit_dependency_summaries: Vec<NotebookAuditSummary>,
		) -> Result<NotebookAuditResult, NotebookVerifyError> {
			todo!("implement audit_notebook_and_get_votes")
		}

		fn decode_signed_raw_notebook_header(_raw_header: Vec<u8>) -> Result<NotaryNotebookVoteDetails<HashOutput>, DispatchError> {
			todo!()
		}

		fn latest_notebook_by_notary() -> BTreeMap<NotaryId, (NotebookNumber, Tick)> {
			BTreeMap::new()
		}
	}

	impl ulx_primitives::TickApis<Block> for RuntimeApi {
		fn current_tick() -> Tick {
			Ticker::new(self.inner.config.tick_duration.as_millis() as u64, self.inner.config.genesis_utc_time).current() -1
		}
		fn ticker() -> Ticker {
			Ticker::new(self.inner.config.tick_duration.as_millis() as u64, self.inner.config.genesis_utc_time)
		}
		fn blocks_at_tick(_: Tick) -> Vec<<Block as BlockT>::Hash> {
			vec![]
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
		let api = TestApi { client: Some(client.as_client().clone()), config: self.config.clone() };
		let api = Arc::new(api);
		let aux_client = UlxAux::new(api.clone());
		let block_import = PanickingBlockImport(import_queue::UlxBlockImport::new(
			inner,
			api.clone(),
			aux_client.clone(),
			select_chain,
		));

		(
			BlockImportAdapter::new(block_import.clone()),
			None,
			Some(PeerData { block_import, api, aux_client }),
		)
	}
}
