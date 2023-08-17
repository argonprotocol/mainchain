use std::{collections::BTreeMap, sync::Arc, time::Duration};

use futures::future;
use log::trace;
use parking_lot::Mutex;
use sc_block_builder::BlockBuilderProvider;
use sc_client_api::{
	AuxStore, BlockOf, BlockchainEvents, FinalityNotifications, ImportNotifications,
	StorageEventStream, StorageKey,
};
use sc_consensus::{
	BlockCheckParams, BlockImport, BlockImportParams, BoxBlockImport, BoxJustificationImport,
	ImportResult, LongestChain,
};
use sc_network_test::{
	Block, BlockImportAdapter, Peer, PeersClient, PeersFullClient, TestNetFactory,
};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_arithmetic::traits::UniqueSaturatedInto;
use sp_blockchain::{BlockStatus, HeaderBackend, Info};
use sp_consensus::{DisableProofRecording, Environment, Proposal, Proposer};
use sp_core::{blake2_256, ByteArray, OpaquePeerId, U256};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{traits::Block as BlockT, ApplyExtrinsicResult, Digest};
use substrate_test_runtime::{AccountId, BlockNumber, Executive, Hash, Header};
use substrate_test_runtime_client::Backend;

use pallet_validator_cohorts::find_xor_closest;
use ulx_primitives::{AuthorityDistance, BlockSealAuthorityId, NextWork, PeerId, ProofOfWorkType};

use crate::{import_queue, inherents::UlxCreateInherentDataProviders, nonce_verify::UlxNonce};
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
		let r = self.1.new_block(digests).unwrap().build().map_err(|e| e.into());

		future::ready(r.map(|b| Proposal {
			block: b.block,
			proof: (),
			storage_changes: b.storage_changes,
		}))
	}
}

pub(crate) type UlxNonceAlgorithm = UlxNonce<Block, TestApi>;
pub(crate) type UlxVerifier = import_queue::UlxVerifier<Block, UlxNonceAlgorithm>;
pub(crate) type UlxBlockImport = PanickingBlockImport<
	import_queue::UlxBlockImport<
		Block,
		BlockImportAdapter<PeersClient>,
		TestApi,
		LongestChain<Backend, Block>,
		UlxNonceAlgorithm,
		UlxCreateInherentDataProviders<Block>,
	>,
>;
pub(crate) struct PeerData {
	pub block_import: Mutex<Option<BoxBlockImport<Block>>>,
	pub api: Arc<TestApi>,
}
pub(crate) type UlxPeer = Peer<Option<PeerData>, UlxBlockImport>;

#[derive(Default)]
pub(crate) struct UlxTestNet {
	pub peers: Vec<UlxPeer>,
	pub config: Config,
	pub authorities: Vec<(u16, AccountId, BlockSealAuthorityId)>,
}
#[derive(Clone, Default)]
pub(crate) struct Config {
	pub difficulty: u128,
	pub min_seal_signers: u32,
	pub closest_xor: u32,
	pub easing: u128,
	pub work_type: ProofOfWorkType,
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
	impl ulx_primitives::UlxConsensusApi<Block> for RuntimeApi {
		fn next_work() -> NextWork {
			NextWork {
				work_type: self.inner.config.work_type,
				difficulty: self.inner.config.difficulty,
				min_seal_signers: self.inner.config.min_seal_signers,
				closest_x_authorities_required: self.inner.config.closest_xor,
			}
		}

		fn calculate_easing(_tax_amount: u128, _validators: u8) -> u128 {
			 self.inner.config.easing.into()
		}
	}

	impl ulx_primitives::AuthorityApis<Block> for RuntimeApi {
		fn authorities() -> Vec<BlockSealAuthorityId> {
			self.inner.authorities.iter().map(|(_,_, id)| id.clone()).collect()
		}
		fn authorities_by_index() -> BTreeMap<u16, BlockSealAuthorityId> {
			self.inner.authorities.iter().map(|(i,_, id)| (*i, id.clone())).collect()
		}
		fn xor_closest_validators(hash: Vec<u8>) -> Vec<AuthorityDistance<BlockSealAuthorityId>> {
			let closest = find_xor_closest(self.inner.authorities.iter().map(|(index, _, id)| {
				((*index).unique_saturated_into(), ( id.clone(), U256::from( blake2_256(&id.to_raw_vec()[..]))))
			}).collect::<Vec<_>>(), U256::from(&hash[..]), self.inner.config.closest_xor.unique_saturated_into());


			closest.into_iter().map(|(a, distance, index)| {
				AuthorityDistance::<_> {
					authority_id: a.clone(),
					authority_index: index.unique_saturated_into(),
					peer_id: PeerId(OpaquePeerId::default()),
					distance,
				}
			})
			.collect::<Vec<_>>()
		}
		fn active_authorities() -> u16 {
			self.inner.authorities.len() as u16
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

	fn make_verifier(&self, client: PeersClient, peer_data: &Self::PeerData) -> Self::Verifier {
		let api = match peer_data {
			None => Arc::new(TestApi {
				client: Some(client.as_client().clone()),
				config: self.config.clone(),
				authorities: self.authorities.clone(),
			}),
			Some(x) => x.api.clone(),
		};
		UlxVerifier::new(UlxNonce::new(api))
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
		let algorithm = UlxNonce::new(api.clone());
		let block_import = PanickingBlockImport(import_queue::UlxBlockImport::new(
			inner,
			api.clone(),
			algorithm,
			select_chain,
			UlxCreateInherentDataProviders::new(),
		));

		let data_block_import =
			Mutex::new(Some(Box::new(block_import.clone()) as BoxBlockImport<Block>));

		(
			BlockImportAdapter::new(block_import),
			None,
			Some(PeerData { block_import: data_block_import, api }),
		)
	}
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
}
