use crate::{
	aux_client::ArgonAux,
	import_queue::{ArgonBlockImport, ImportApisExt},
};
use argon_primitives::{
	BlockSealAuthoritySignature, BlockSealDigest, ComputeDifficulty, Digestset, FORK_POWER_DIGEST,
	HashOutput as BlockHash, NotebookDigest, PARENT_VOTING_KEY_DIGEST, ParentVotingKeyDigest,
	VotingKey,
	fork_power::ForkPower,
	prelude::{
		sp_runtime::{generic::SignedBlock, traits::BlakeTwo256},
		*,
	},
};
use argon_runtime::NotebookVerifyError;
use async_trait::async_trait;
use codec::Encode;
use polkadot_sdk::{
	frame_support::BoundedVec,
	sp_core::{ByteArray, H256, U256},
	sp_runtime::DigestItem,
};
use sc_client_api::{BlockBackend, backend::AuxStore};
use sc_consensus::{
	BlockCheckParams, BlockImportParams, ImportResult, ImportedAux, StateAction, StateAction::*,
};
use sp_blockchain::{BlockGap, BlockStatus, Error as BlockchainError, HeaderBackend};
use sp_consensus::{BlockOrigin, Error as ConsensusError};
use sp_runtime::{
	Digest, OpaqueExtrinsic as UncheckedExtrinsic, generic,
	traits::{Block as BlockT, Header as HeaderT, NumberFor},
};
use std::{
	collections::{BTreeMap, HashMap},
	sync::{Arc, Mutex},
};
// -------------------------------------------
// Tiny in–memory client & mini importer
// -------------------------------------------

#[derive(Clone)]
pub(crate) struct MemChain {
	headers: Arc<Mutex<HashMap<BlockHash, Header>>>,
	block_state: Arc<Mutex<HashMap<BlockHash, sp_consensus::BlockStatus>>>,
	block_gap: Arc<Mutex<Option<BlockGap<BlockNumber>>>>,
	genesis_hash: BlockHash,
	best: Arc<Mutex<(BlockNumber, BlockHash)>>,
	finalized: Arc<Mutex<(BlockNumber, BlockHash)>>,
	aux: Arc<Mutex<BTreeMap<Vec<u8>, Vec<u8>>>>,
}
impl MemChain {
	pub(crate) fn new(genesis: Header) -> Self {
		let h = genesis.hash();
		Self {
			headers: Arc::new(Mutex::new([(h, genesis)].into())),
			block_state: Arc::new(Mutex::new(
				[(h, sp_consensus::BlockStatus::InChainWithState)].into(),
			)),
			block_gap: Arc::new(Mutex::new(None)),
			genesis_hash: h,
			best: Arc::new(Mutex::new((0u32, h))),
			finalized: Arc::new(Mutex::new((0u32, h))),
			aux: Arc::new(Mutex::new(BTreeMap::new())),
		}
	}
	pub(crate) fn insert(&self, hdr: Header) {
		let h = hdr.hash();
		self.block_state
			.lock()
			.unwrap()
			.insert(h, sp_consensus::BlockStatus::InChainPruned); // header only, no state yet
		self.headers.lock().unwrap().insert(h, hdr);
	}

	pub(crate) fn set_state(&self, h: BlockHash, state: sp_consensus::BlockStatus) {
		self.block_state.lock().unwrap().insert(h, state);
	}
	pub(crate) fn force_best(&self, best_number: BlockNumber, best_hash: BlockHash) {
		*self.best.lock().unwrap() = (best_number, best_hash);
	}

	pub(crate) fn set_block_gap(&self, gap: Option<BlockGap<BlockNumber>>) {
		*self.block_gap.lock().unwrap() = gap;
	}
}
impl HeaderBackend<Block> for MemChain {
	fn header(&self, h: BlockHash) -> Result<Option<Header>, BlockchainError> {
		Ok(self.headers.lock().unwrap().get(&h).cloned())
	}
	fn info(&self) -> sp_blockchain::Info<Block> {
		let best = *self.best.lock().unwrap();
		let fin = *self.finalized.lock().unwrap();
		let block_gap = *self.block_gap.lock().unwrap();
		sp_blockchain::Info {
			finalized_hash: fin.1,
			finalized_number: fin.0,
			finalized_state: None,
			best_hash: best.1,
			best_number: best.0,
			block_gap,
			genesis_hash: self.genesis_hash,
			number_leaves: 0,
		}
	}
	fn status(&self, h: BlockHash) -> Result<BlockStatus, BlockchainError> {
		Ok(if self.headers.lock().unwrap().contains_key(&h) {
			BlockStatus::InChain
		} else {
			BlockStatus::Unknown
		})
	}

	fn number(&self, hash: BlockHash) -> sp_blockchain::Result<Option<BlockNumber>> {
		Ok(self.headers.lock().unwrap().get(&hash).map(|h| *h.number()))
	}

	fn hash(&self, number: NumberFor<Block>) -> sp_blockchain::Result<Option<BlockHash>> {
		Ok(self
			.headers
			.lock()
			.unwrap()
			.values()
			.find(|h| *h.number() == number)
			.map(|h| h.hash()))
	}
}

impl ImportApisExt<Block> for MemChain {
	fn has_new_bitcoin_tip(&self, _hash: BlockHash) -> bool {
		false
	}

	fn has_new_price_index(&self, _hash: BlockHash) -> bool {
		false
	}
}

impl BlockBackend<Block> for MemChain {
	fn block_body(
		&self,
		_hash: BlockHash,
	) -> sc_client_api::blockchain::Result<Option<Vec<UncheckedExtrinsic>>> {
		Ok(Some(Vec::new()))
	}
	fn block(&self, hash: BlockHash) -> sp_blockchain::Result<Option<SignedBlock<Block>>> {
		if let Some(header) = self.headers.lock().unwrap().get(&hash) {
			Ok(Some(SignedBlock {
				block: Block::new(header.clone(), Vec::new()),
				justifications: None,
			}))
		} else {
			Ok(None)
		}
	}
	fn block_status(
		&self,
		hash: BlockHash,
	) -> sc_client_api::blockchain::Result<sp_consensus::BlockStatus> {
		let map = self.block_state.lock().unwrap();
		Ok(map.get(&hash).cloned().unwrap_or(sp_consensus::BlockStatus::Unknown))
	}

	fn justifications(
		&self,
		_hash: BlockHash,
	) -> sc_client_api::blockchain::Result<Option<sp_runtime::Justifications>> {
		Ok(None)
	}

	fn block_indexed_body(
		&self,
		_hash: BlockHash,
	) -> sc_client_api::blockchain::Result<Option<Vec<Vec<u8>>>> {
		Ok(Some(Vec::new()))
	}

	fn requires_full_sync(&self) -> bool {
		false
	}

	fn block_hash(&self, number: NumberFor<Block>) -> sp_blockchain::Result<Option<BlockHash>> {
		Ok(self
			.headers
			.lock()
			.unwrap()
			.values()
			.find(|h| *h.number() == number)
			.map(|h| h.hash()))
	}

	fn indexed_transaction(&self, _hash: BlockHash) -> sp_blockchain::Result<Option<Vec<u8>>> {
		Ok(None)
	}
}

impl AuxStore for MemChain {
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
	) -> sc_client_api::blockchain::Result<()> {
		let mut aux = self.aux.lock().expect("aux is poisoned");
		for (k, v) in insert {
			aux.insert(k.to_vec(), v.to_vec());
		}
		for k in delete {
			aux.remove(*k);
		}
		Ok(())
	}

	fn get_aux(&self, key: &[u8]) -> sc_client_api::blockchain::Result<Option<Vec<u8>>> {
		let aux = self.aux.lock().unwrap();
		Ok(aux.get(key).cloned())
	}
}

/// Opaque block header type.
type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Opaque block type.
pub(crate) type Block = generic::Block<Header, UncheckedExtrinsic>;

#[async_trait]
impl sc_consensus::BlockImport<Block> for MemChain {
	type Error = ConsensusError;

	async fn check_block(&self, _: BlockCheckParams<Block>) -> Result<ImportResult, Self::Error> {
		Ok(ImportResult::Imported(ImportedAux::default()))
	}

	async fn import_block(
		&self,
		params: BlockImportParams<Block>,
	) -> Result<ImportResult, Self::Error> {
		let num = *params.header.number();
		let hash = params.header.hash();
		// store/overwrite header so later calls see it in MemChain
		self.insert(params.header.clone());
		match params.state_action {
			ApplyChanges(_) | Execute =>
				self.set_state(hash, sp_consensus::BlockStatus::InChainWithState),
			ExecuteIfPossible => {
				// If the parent already has full state we can execute immediately.
				let p_hash = *params.header.parent_hash();
				let parent_has_state = matches!(
					self.block_state.lock().unwrap().get(&p_hash),
					Some(sp_consensus::BlockStatus::InChainWithState)
				);
				if parent_has_state {
					self.set_state(hash, sp_consensus::BlockStatus::InChainWithState);
				} else {
					// header‑only until the parent’s state arrives
					// (behaves like real client which retries later)
				}
			},
			Skip => {}, // header only
		}
		// minimal: just mark best/finalized if fork_choice says so
		if let Some(fork_choice) = params.fork_choice {
			match fork_choice {
				sc_consensus::ForkChoiceStrategy::Custom(set_best) if set_best => {
					let mut best = self.best.lock().unwrap();
					if num > best.0 || (num == best.0 && hash != best.1) {
						*best = (num, hash);
					}
				},
				_ => {},
			}
		}
		if params.finalized {
			let mut fin = self.finalized.lock().unwrap();
			if num > fin.0 || (num == fin.0 && hash != fin.1) {
				*fin = (num, hash);
			}
		}
		Ok(ImportResult::Imported(ImportedAux::default()))
	}
}

fn create_header(
	number: BlockNumber,
	parent: H256,
	compute_difficulty: ComputeDifficulty,
	voting_key: Option<VotingKey>,
	author: AccountId,
) -> (Header, DigestItem) {
	let (digest, block_digest) = create_digest(compute_difficulty, voting_key, author);
	(Header::new(number, H256::zero(), H256::zero(), parent, digest), block_digest)
}
pub(crate) fn has_state(db: &MemChain, h: H256) -> bool {
	matches!(
		db.block_state.lock().unwrap().get(&h),
		Some(sp_consensus::BlockStatus::InChainWithState)
	)
}

fn create_digest(
	compute_difficulty: ComputeDifficulty,
	voting_key: Option<VotingKey>,
	author: AccountId,
) -> (Digest, DigestItem) {
	let mut power = ForkPower::default();
	let block_seal = if compute_difficulty > 0 {
		let block_seal = BlockSealDigest::Compute { nonce: U256::zero() };
		power.add(0, 0, block_seal.clone(), compute_difficulty);
		block_seal
	} else {
		let block_seal = BlockSealDigest::Vote {
			seal_strength: U256::one(),
			miner_nonce_score: Some(U256::one()),
			signature: BlockSealAuthoritySignature::from_slice(&[0u8; 64])
				.expect("serialization of block seal strength failed"),
		};
		power.add(500, 1, block_seal.clone(), 0);
		block_seal
	};
	let digests = Digestset {
		author,
		block_vote: Default::default(),
		voting_key: None, // runtime digest
		fork_power: None, // runtime digest
		frame_info: None, // runtime digest
		tick: Default::default(),
		notebooks: NotebookDigest::<NotebookVerifyError> { notebooks: BoundedVec::new() },
	};
	let mut digest = digests.create_pre_runtime_digest();
	digest.push(DigestItem::Consensus(FORK_POWER_DIGEST, power.encode()));
	digest.push(DigestItem::Consensus(
		PARENT_VOTING_KEY_DIGEST,
		ParentVotingKeyDigest { parent_voting_key: voting_key }.encode(),
	));
	(digest, block_seal.to_digest())
}

pub(crate) fn new_importer() -> (ArgonBlockImport<Block, MemChain, MemChain, H256>, MemChain) {
	let genesis = Header::new(0, H256::zero(), H256::zero(), H256::zero(), Digest::default());
	let client = MemChain::new(genesis.clone());
	let db_arc = Arc::new(client.clone());
	let importer = ArgonBlockImport::<Block, _, _, _>::new_for_tests(
		client.clone(),
		db_arc.clone(),
		ArgonAux::new(db_arc.clone()),
	);
	(importer, client)
}

pub(crate) fn create_params(
	block_number: BlockNumber,
	parent_hash: H256,
	compute_difficulty: ComputeDifficulty,
	voting_key: Option<VotingKey>,
	origin: BlockOrigin,
	state_action: StateAction<Block>,
	author: Option<AccountId>,
) -> BlockImportParams<Block> {
	let author = author.unwrap_or_else(|| AccountId::from([0u8; 32]));
	let (header, post_digest) =
		create_header(block_number, parent_hash, compute_difficulty, voting_key, author);
	let mut params = BlockImportParams::new(origin, header.clone());
	params.state_action = state_action;
	params.post_digests.push(post_digest);
	params.post_hash = Some(header.hash());
	params
}
