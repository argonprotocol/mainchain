#![allow(clippy::type_complexity)]
use crate::{
	aux_data::AuxData, error::Error, metrics::BlockMetrics, notary_client::VotingPowerInfo,
};
use argon_primitives::{
	AccountId, NotaryId, NotebookAuditResult, NotebookHeaderData, NotebookNumber, VotingSchedule,
	notary::{
		NotaryNotebookAuditSummary, NotaryNotebookDetails, NotaryNotebookRawVotes,
		NotaryNotebookTickState, NotaryNotebookVoteDigestDetails, SignedHeaderBytes,
	},
	tick::Tick,
};
use argon_runtime::NotebookVerifyError;
use codec::{Codec, Decode, Encode};
use frame_support::{Deserialize, Serialize};
use log::{trace, warn};
use parking_lot::RwLock;
use polkadot_sdk::*;
use sc_client_api::{self, backend::AuxStore};
use schnellru::{ByLength, LruMap};
use sp_core::{H256, RuntimeDebug};
use sp_runtime::traits::Block as BlockT;
use std::{
	any::Any,
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
	sync::Arc,
};
use tracing::info;

pub enum AuxState<C: AuxStore> {
	NotaryStateAtTick(Arc<AuxData<NotaryNotebookTickState, C>>),
	AuthorsAtTick(Arc<AuxData<BTreeMap<H256, BTreeSet<AccountId>>, C>>),
	NotaryNotebooks(
		Arc<AuxData<BTreeMap<NotebookNumber, NotebookAuditAndRawHeader<NotebookVerifyError>>, C>>,
	),
	NotaryAuditSummaries(Arc<AuxData<Vec<NotaryNotebookAuditSummary>, C>>),
	VotesAtTick(Arc<AuxData<Vec<NotaryNotebookRawVotes>, C>>),
	BlockMetrics(Arc<AuxData<BlockMetrics, C>>),
	Version(Arc<AuxData<u32, C>>),
}

trait AuxStateData {
	fn as_any(&self) -> &dyn Any;
}

impl<C: AuxStore + 'static> AuxStateData for AuxState<C> {
	fn as_any(&self) -> &dyn Any {
		match self {
			AuxState::NotaryStateAtTick(a) => a,
			AuxState::AuthorsAtTick(a) => a,
			AuxState::NotaryNotebooks(a) => a,
			AuxState::VotesAtTick(a) => a,
			AuxState::NotaryAuditSummaries(a) => a,
			AuxState::BlockMetrics(a) => a,
			AuxState::Version(a) => a,
		}
	}
}
#[derive(Clone, Encode, Decode, Debug, Hash, Eq, PartialEq)]
pub enum AuxKey {
	NotaryStateAtTick(Tick),
	AuthorsAtTick(Tick),
	NotaryNotebooks(NotaryId),
	VotesAtTick(Tick),
	NotaryAuditSummaries(NotaryId),
	BlockMetrics,
	Version,
}

impl AuxKey {
	pub fn default_state<C: AuxStore>(&self, client: Arc<C>) -> AuxState<C> {
		match self {
			AuxKey::NotaryStateAtTick(_) =>
				AuxState::NotaryStateAtTick(AuxData::new(client, self.clone()).into()),
			AuxKey::AuthorsAtTick(_) =>
				AuxState::AuthorsAtTick(AuxData::new(client, self.clone()).into()),
			AuxKey::NotaryNotebooks(_) =>
				AuxState::NotaryNotebooks(AuxData::new(client, self.clone()).into()),
			AuxKey::VotesAtTick(_) =>
				AuxState::VotesAtTick(AuxData::new(client, self.clone()).into()),
			AuxKey::NotaryAuditSummaries(_) =>
				AuxState::NotaryAuditSummaries(AuxData::new(client, self.clone()).into()),
			AuxKey::BlockMetrics =>
				AuxState::BlockMetrics(AuxData::new(client, self.clone()).into()),
			AuxKey::Version => AuxState::Version(AuxData::new(client, self.clone()).into()),
		}
	}
}

pub struct ArgonAux<B: BlockT, C: AuxStore> {
	pub lock: Arc<RwLock<()>>,
	client: Arc<C>,
	state: Arc<RwLock<LruMap<AuxKey, AuxState<C>>>>,
	_block: std::marker::PhantomData<B>,
}

impl<B: BlockT, C: AuxStore> Clone for ArgonAux<B, C> {
	fn clone(&self) -> Self {
		Self {
			client: self.client.clone(),
			state: self.state.clone(),
			lock: self.lock.clone(),
			_block: Default::default(),
		}
	}
}

impl<B: BlockT, C: AuxStore> ArgonAux<B, C> {
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			state: Arc::new(RwLock::new(LruMap::new(ByLength::new(500)))),
			lock: Default::default(),
			_block: Default::default(),
		}
	}
}

pub const OLDEST_TICK_STATE: Tick = 256;
const MAX_FINALIZED_AUDIT_HISTORY: usize = 100;
const MAX_EXTRA_SUMMARY_HISTORY: usize = 100;

///
/// Stores auxiliary data for argon consensus (eg - cross block data)
///
/// We store several types of data
/// - `BlockVotes` - all block votes submitted (voting for a block hash)
/// - `StrongestVoteAtHeight` - the strongest vote at a given height - helps determine if we should
///   create a block
/// - `AuthorsAtHeight` - the authors at a given height for every voting key. A block will only be
///   accepted once per author per key
impl<B: BlockT, C: AuxStore + 'static> ArgonAux<B, C> {
	pub fn migrate(&self, runtime_tick: Tick) -> Result<(), Error> {
		let _lock = self.lock.write();
		let version = self.get_or_insert_state::<u32>(AuxKey::Version)?;

		const VERSION: u32 = 1;
		if version.get() == VERSION {
			return Ok(());
		}

		if runtime_tick == 0 {
			version.mutate(|a| *a = VERSION)?;
			return Ok(());
		}

		info!("Migrating argon aux data from version {} to {}", version.get(), VERSION);

		let mut to_delete = vec![];
		// check a year of ticks
		let oldest_tick = runtime_tick.saturating_sub(1440 * 365);

		// Force a resync of headers
		for i in oldest_tick..runtime_tick.saturating_sub(OLDEST_TICK_STATE) {
			let key = AuxKey::NotaryStateAtTick(i).encode();
			if i % 1440 == 0 {
				info!("Migrating tick state progress {} of {}", i, runtime_tick);
			}
			if self.client.get_aux(&key)?.is_some() {
				to_delete.push(key);
			};
		}

		for i in 0..5 {
			let key = AuxKey::NotaryNotebooks(i).encode();
			if self.client.get_aux(&key)?.is_some() {
				to_delete.push(key);
			}
		}

		self.client
			.insert_aux(vec![], &to_delete.iter().map(|a| a.as_slice()).collect::<Vec<_>>())?;

		version.mutate(|a| *a = VERSION)?;
		info!("Migration of aux data complete {}", version.get());

		Ok(())
	}

	pub fn check_duplicate_block_at_tick<AC: Codec>(
		&self,
		author: AC,
		block_key: H256,
		tick: Tick,
		is_vote: bool,
	) -> Result<(), Error> {
		let _lock = self.lock.write();
		self.authors_at_tick(tick)?.mutate(|authors_at_height| {
			let account_id = AccountId::decode(&mut &author.encode()[..])
				.map_err(|e| Error::StringError(format!("Failed to decode author: {:?}", e)))?;
			if !authors_at_height.entry(block_key).or_default().insert(account_id.clone()) {
				let block_type = if is_vote { "vote" } else { "compute" };
				return Err(Error::DuplicateAuthoredBlock(account_id, block_type.to_string()));
			}
			Ok::<(), Error>(())
		})??;

		Ok(())
	}

	pub fn clean_state_history(
		&self,
		auxiliary_changes: &mut Vec<(Vec<u8>, Option<Vec<u8>>)>,
		tick: Tick,
	) -> Result<(), Error> {
		// cleanup old votes (None deletes)
		if tick >= 10 {
			let cleanup_height = tick - 10;
			auxiliary_changes.push((AuxKey::VotesAtTick(cleanup_height).encode(), None));
			auxiliary_changes.push((AuxKey::AuthorsAtTick(cleanup_height).encode(), None));
			self.state.write().remove(&AuxKey::VotesAtTick(cleanup_height));
			self.state.write().remove(&AuxKey::AuthorsAtTick(cleanup_height));
		}
		// Cleanup old notary state. We keep this longer because we might need to catchup on
		// notebooks
		if tick >= OLDEST_TICK_STATE {
			let oldest = tick.saturating_sub(OLDEST_TICK_STATE);
			// cleanup 10 ticks at a time, just in case
			for tick in oldest.saturating_sub(10)..=oldest {
				auxiliary_changes.push((AuxKey::NotaryStateAtTick(tick).encode(), None));
				self.state.write().remove(&AuxKey::NotaryStateAtTick(tick));
			}
		}
		Ok(())
	}

	pub fn get_tick_voting_power(&self, tick: Tick) -> Result<Option<VotingPowerInfo>, Error> {
		let state = self.get_notebook_tick_state(tick)?.get();
		let mut voting_power = 0u128;
		let mut notebooks = 0u32;
		for digest in state.notebook_key_details_by_notary.values() {
			voting_power += digest.block_voting_power;
			notebooks += 1;
		}
		Ok(Some((tick, voting_power, notebooks)))
	}

	pub fn get_notary_notebooks_for_header(
		&self,
		notary_id: NotaryId,
		latest_runtime_notebook_number: NotebookNumber,
		voting_schedule: &VotingSchedule,
		max_notebooks: u32,
	) -> Result<
		(NotebookHeaderData<NotebookVerifyError>, Option<NotaryNotebookVoteDigestDetails>),
		Error,
	> {
		let mut headers = NotebookHeaderData::default();
		let mut tick_notebook = None;
		let audit_results = self.get_notary_audit_history(notary_id)?.get();
		let notebook_tick = voting_schedule.notebook_tick();

		let mut notebook_count = 0;
		for (notebook_number, notebook) in audit_results {
			if notebook_number <= latest_runtime_notebook_number || notebook.tick > notebook_tick {
				continue;
			}
			if notebook.audit_first_failure.is_some() {
				warn!(
					"Not adding additional notebooks for notary {} due to audit failure {}",
					notary_id,
					notebook.audit_first_failure.unwrap()
				);
				break;
			}
			let tick = notebook.tick;
			if tick == notebook_tick {
				let state = self.get_notebook_tick_state(tick)?.get();
				tick_notebook = state.notebook_key_details_by_notary.get(&notary_id).cloned();
			}

			notebook_count += 1;
			headers.signed_headers.push(notebook.raw_signed_header.clone());
			headers.notebook_digest.notebooks.push(NotebookAuditResult {
				notary_id,
				notebook_number: notebook.notebook_number,
				tick,
				audit_first_failure: notebook.audit_first_failure.clone(),
			});

			if headers.signed_headers.len() >= max_notebooks as usize {
				break;
			}
		}

		let mut expected_next_number = latest_runtime_notebook_number + 1;
		for notebook in &headers.notebook_digest.notebooks {
			if notebook.notary_id == notary_id && notebook.notebook_number != expected_next_number {
				return Err(Error::StringError(format!(
					"Missing notebook {} for notary {} (stopped here, might be more)",
					expected_next_number, notary_id
				)));
			}
			expected_next_number += 1;
		}
		tracing::trace!(
			notebook_tick,
			notary_id,
			notebook_count,
			"Building notebook inherent for notary.",
		);
		Ok((headers, tick_notebook))
	}

	pub fn has_successful_audit(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
	) -> bool {
		self.get_notary_audit_history(notary_id)
			.ok()
			.map(|a| a.get())
			.unwrap_or_default()
			.get(&notebook_number)
			.map(|n| n.audit_first_failure.is_none())
			.unwrap_or_default()
	}

	/// Keeps a manually truncated vec of the last 2000 notary audit results
	pub fn get_notary_audit_history(
		&self,
		notary_id: NotaryId,
	) -> Result<
		Arc<AuxData<BTreeMap<NotebookNumber, NotebookAuditAndRawHeader<NotebookVerifyError>>, C>>,
		Error,
	> {
		let key = AuxKey::NotaryNotebooks(notary_id);
		self.get_or_insert_state(key)
	}

	pub fn authors_at_tick(
		&self,
		tick: Tick,
	) -> Result<Arc<AuxData<BTreeMap<H256, BTreeSet<AccountId>>, C>>, Error> {
		let key = AuxKey::AuthorsAtTick(tick);
		self.get_or_insert_state(key)
	}

	pub fn store_votes(&self, tick: Tick, votes: NotaryNotebookRawVotes) -> Result<(), Error> {
		self.get_votes(tick)?.mutate(|existing| {
			if !existing.iter().any(|x| {
				x.notary_id == votes.notary_id && x.notebook_number == votes.notebook_number
			}) {
				existing.push(votes);
			}
		})?;
		Ok(())
	}

	pub fn get_votes(
		&self,
		tick: Tick,
	) -> Result<Arc<AuxData<Vec<NotaryNotebookRawVotes>, C>>, Error> {
		let key = AuxKey::VotesAtTick(tick);
		self.get_or_insert_state(key)
	}

	pub fn get_audit_summaries(
		&self,
		notary_id: NotaryId,
	) -> Result<Arc<AuxData<Vec<NotaryNotebookAuditSummary>, C>>, Error> {
		let key = AuxKey::NotaryAuditSummaries(notary_id);
		self.get_or_insert_state(key)
	}

	/// clear out state for a failed notebook
	pub fn reprocess_notebook(
		&self,
		notary_id: NotaryId,
		reprocess_notebook_number: NotebookNumber,
	) -> Result<(), Error> {
		let current_audits = self.get_notary_audit_history(notary_id)?.get();
		let Some(existing) = current_audits.get(&reprocess_notebook_number) else { return Ok(()) };

		// if this audit is valid, assume we're good
		if existing.audit_first_failure.is_none() {
			return Ok(());
		}

		let tick = existing.tick;
		self.get_notary_audit_history(notary_id)?
			.mutate(|a| a.retain(|n, _| n < &reprocess_notebook_number))?;
		self.get_audit_summaries(notary_id)?
			.mutate(|a| a.retain(|n| n.notebook_number < reprocess_notebook_number))?;

		self.get_notebook_tick_state(tick)?.mutate(|state| {
			state.notebook_key_details_by_notary.remove(&notary_id);
		})?;

		Ok(())
	}

	/// Stores notebook details and audit results by tick
	/// Returns total block votes at tick and the number of notebooks stored
	pub fn store_notebook_result(
		&self,
		audit_result: NotebookAuditResult<NotebookVerifyError>,
		raw_signed_header: SignedHeaderBytes,
		notebook_details: NotaryNotebookDetails<B::Hash>,
		finalized_notebook_number: NotebookNumber,
		runtime_tick: Tick,
	) -> Result<VotingPowerInfo, Error> {
		let tick = notebook_details.tick;
		let notary_id = notebook_details.notary_id;
		let notebook_number = notebook_details.notebook_number;

		trace!(
			"Storing vote details for tick {} and notary {} at notebook #{}",
			tick, notary_id, notebook_number
		);

		let mut voting_power = 0u128;
		let mut notebooks = 0u32;

		let (summary, vote_details) = notebook_details.into();
		// only update tick state if this is in the last OLDEST_TICK_STATE ticks
		if tick > runtime_tick || runtime_tick.saturating_sub(tick) < OLDEST_TICK_STATE {
			self.get_notebook_tick_state(tick)?.mutate(|state| {
				state.notebook_key_details_by_notary.insert(notary_id, vote_details);
				for digest in state.notebook_key_details_by_notary.values() {
					voting_power += digest.block_voting_power;
					notebooks += 1;
				}
			})?;
		} else {
			let state = self.get_notebook_tick_state(tick)?.get();
			for digest in state.notebook_key_details_by_notary.values() {
				voting_power += digest.block_voting_power;
				notebooks += 1;
			}
		}

		self.get_audit_summaries(notary_id)?.mutate(|summaries| {
			let mut insert_index = 0;
			for (i, s) in summaries.iter().enumerate().rev() {
				if s.notebook_number == notebook_number {
					return;
				}
				if s.notebook_number < notebook_number {
					insert_index = i + 1;
					break;
				}
			}
			summaries.insert(insert_index, summary);
			// keep history for a little while
			let oldest_to_retain =
				finalized_notebook_number.saturating_sub(MAX_EXTRA_SUMMARY_HISTORY as u32);
			summaries.retain(|s| s.notebook_number > oldest_to_retain);
		})?;

		// keep history for a little while
		let oldest_to_retain =
			finalized_notebook_number.saturating_sub(MAX_FINALIZED_AUDIT_HISTORY as u32);
		self.get_notary_audit_history(notary_id)?.mutate(|notebooks| {
			notebooks.insert(notebook_number, (raw_signed_header, audit_result).into());
			if notebooks.len() > MAX_FINALIZED_AUDIT_HISTORY {
				// remove oldest notebooks
				notebooks.retain(|n, _| *n > oldest_to_retain);
			}
		})?;
		Ok((tick, voting_power, notebooks))
	}

	fn get_notebook_tick_state(
		&self,
		tick: Tick,
	) -> Result<Arc<AuxData<NotaryNotebookTickState, C>>, Error> {
		let key = AuxKey::NotaryStateAtTick(tick);
		self.get_or_insert_state(key)
	}

	fn get_or_insert_state<T: 'static + Clone>(
		&self,
		key: AuxKey,
	) -> Result<Arc<AuxData<T, C>>, Error> {
		let mut state = self.state.write();
		let entry = state
			.get_or_insert(key.clone(), || key.default_state(self.client.clone()))
			.ok_or(Error::StringError(format!("Error unlocking notary state for {key:?}")))?;
		if let Some(data) = entry.as_any().downcast_ref::<Arc<AuxData<T, C>>>() {
			Ok(data.clone())
		} else {
			Err(format!("Could not downcast AuxState for {key:?}").into())
		}
	}
}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug, Serialize, Deserialize, Default)]
pub struct NotebookAuditAndRawHeader<E: Codec> {
	#[codec(compact)]
	pub notary_id: NotaryId,
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	#[codec(compact)]
	pub tick: Tick,
	pub audit_first_failure: Option<E>,
	pub raw_signed_header: SignedHeaderBytes,
}

impl<E: Codec> From<(SignedHeaderBytes, NotebookAuditResult<E>)> for NotebookAuditAndRawHeader<E> {
	fn from(value: (SignedHeaderBytes, NotebookAuditResult<E>)) -> Self {
		NotebookAuditAndRawHeader {
			notary_id: value.1.notary_id,
			notebook_number: value.1.notebook_number,
			tick: value.1.tick,
			audit_first_failure: value.1.audit_first_failure,
			raw_signed_header: value.0,
		}
	}
}
impl<E: Codec> From<NotebookAuditAndRawHeader<E>> for NotebookAuditResult<E> {
	fn from(value: NotebookAuditAndRawHeader<E>) -> Self {
		NotebookAuditResult {
			notary_id: value.notary_id,
			notebook_number: value.notebook_number,
			tick: value.tick,
			audit_first_failure: value.audit_first_failure,
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use argon_runtime::Block;
	use sc_client_api::AuxStore;
	use std::{collections::BTreeMap, sync::Arc};

	#[derive(Clone, Default)]
	struct MockAux {
		pub aux: Arc<parking_lot::Mutex<BTreeMap<Vec<u8>, Vec<u8>>>>,
	}

	impl AuxStore for MockAux {
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
			let mut aux = self.aux.lock();
			for (k, v) in insert {
				aux.insert(k.to_vec(), v.to_vec());
			}
			for k in delete {
				aux.remove(*k);
			}
			Ok(())
		}

		fn get_aux(&self, key: &[u8]) -> sc_client_api::blockchain::Result<Option<Vec<u8>>> {
			let aux = self.aux.lock();
			Ok(aux.get(key).cloned())
		}
	}

	#[test]
	fn it_should_store_notebook_results() {
		let aux = Arc::new(MockAux::default());
		let argon_aux = ArgonAux::<Block, _>::new(aux.clone());
		let empty_header: SignedHeaderBytes = Default::default();
		let audit_10 = NotebookAuditResult {
			notebook_number: 10,
			tick: 1,
			notary_id: 1,
			audit_first_failure: None,
		};
		let details_10 = NotaryNotebookDetails {
			notary_id: 1,
			block_voting_power: 0,
			tick: 1,
			notebook_number: 10,
			raw_audit_summary: vec![],
			version: 1,
			block_votes_count: 0,
			blocks_with_votes: vec![],
			header_hash: H256::zero(),
		};
		let (summary_10, _vote_details_10) = details_10.clone().into();

		let result = argon_aux
			.store_notebook_result(audit_10.clone(), empty_header.clone(), details_10.clone(), 3, 1)
			.expect("store notebook result");
		assert_eq!(result, (1, 0u128, 1));
		let stored = argon_aux.get_notary_audit_history(1).expect("get notary audit history").get();
		assert_eq!(stored, BTreeMap::from([(10, (empty_header.clone(), audit_10.clone()).into())]));
		assert_eq!(
			argon_aux.get_audit_summaries(1).expect("get audit summaries").get(),
			vec![summary_10.clone()]
		);
		assert_eq!(
			argon_aux
				.get_notebook_tick_state(1)
				.expect("get notebook tick state")
				.get()
				.notebook_key_details_by_notary
				.len(),
			1
		);

		let audit_9 = NotebookAuditResult {
			notebook_number: 9,
			tick: 1,
			notary_id: 1,
			audit_first_failure: None,
		};
		let details_9 = NotaryNotebookDetails {
			notary_id: 1,
			block_voting_power: 0,
			tick: 1,
			notebook_number: 9,
			raw_audit_summary: vec![],
			version: 1,
			block_votes_count: 0,
			blocks_with_votes: vec![],
			header_hash: H256::zero(),
		};
		let (summary_9, _vote_details_9) = details_9.clone().into();
		let result = argon_aux
			.store_notebook_result(audit_9.clone(), empty_header.clone(), details_9.clone(), 3, 1)
			.expect("store notebook result");
		assert_eq!(result, (1, 0u128, 1));
		assert_eq!(
			argon_aux.get_notary_audit_history(1).expect("get notary audit history").get(),
			BTreeMap::from([
				(9, (empty_header.clone(), audit_9.clone()).into()),
				(10, (empty_header.clone(), audit_10.clone()).into())
			])
		);

		assert_eq!(
			argon_aux.get_audit_summaries(1).expect("get audit summaries").get(),
			vec![summary_9.clone(), summary_10.clone(),]
		);

		let audit_11 = NotebookAuditResult {
			notebook_number: 11,
			tick: 2,
			notary_id: 1,
			audit_first_failure: None,
		};
		let details_11 = NotaryNotebookDetails {
			notary_id: 1,
			block_voting_power: 0,
			tick: 2,
			notebook_number: 11,
			raw_audit_summary: vec![],
			version: 1,
			block_votes_count: 0,
			blocks_with_votes: vec![],
			header_hash: H256::zero(),
		};
		let (summary_11, _vote_details_11) = details_11.clone().into();
		argon_aux
			.store_notebook_result(audit_11.clone(), Default::default(), details_11.clone(), 9, 1)
			.expect("store notebook result");
		assert_eq!(
			argon_aux.get_notary_audit_history(1).expect("get notary audit history").get(),
			BTreeMap::from([
				(9, (empty_header.clone(), audit_9.clone()).into()),
				(10, (empty_header.clone(), audit_10.clone()).into()),
				(11, (empty_header.clone(), audit_11.clone()).into()),
			])
		);
		assert_eq!(
			argon_aux.get_audit_summaries(1).expect("get audit summaries").get(),
			vec![summary_9.clone(), summary_10.clone(), summary_11.clone(),]
		);

		let mut audit_10_mod = audit_10.clone();
		audit_10_mod.tick = 2;
		let mut details_10_mod = details_10.clone();
		details_10_mod.tick = 2;
		argon_aux
			.store_notebook_result(audit_10_mod.clone(), Default::default(), details_10, 9, 1)
			.expect("store notebook result");

		assert_eq!(
			argon_aux.get_notary_audit_history(1).expect("get notary audit history").get(),
			BTreeMap::from([
				(9, (empty_header.clone(), audit_9.clone()).into()),
				(10, (empty_header.clone(), audit_10_mod.clone()).into()),
				(11, (empty_header.clone(), audit_11.clone()).into()),
			]),
			"should not add duplicate notebook"
		);
		assert_eq!(
			argon_aux.get_audit_summaries(1).expect("get audit summaries").get(),
			vec![summary_9.clone(), summary_10.clone(), summary_11.clone(),],
			"should not add duplicate notebook"
		);
	}

	#[test]
	fn it_should_not_add_tick_state_for_old_ticks() {
		let aux = Arc::new(MockAux::default());
		let argon_aux = ArgonAux::<Block, _>::new(aux.clone());
		for i in 1..OLDEST_TICK_STATE + 10 {
			let notebook_number = i as NotebookNumber;
			let tick = i as Tick;
			argon_aux
				.get_audit_summaries(1)
				.unwrap()
				.mutate(|a| {
					a.push(NotaryNotebookAuditSummary {
						notary_id: 1,
						notebook_number,
						tick,
						version: 0,
						raw_data: vec![],
					});
				})
				.unwrap();
			let audit = NotebookAuditResult {
				notebook_number,
				tick,
				notary_id: 1,
				audit_first_failure: None,
			};
			let details = NotaryNotebookDetails {
				notary_id: 1,
				block_voting_power: 0,
				tick,
				notebook_number,
				raw_audit_summary: vec![],
				version: 1,
				block_votes_count: 0,
				blocks_with_votes: vec![],
				header_hash: H256::zero(),
			};

			let mut changes = vec![];
			argon_aux.clean_state_history(&mut changes, i as Tick).unwrap();
			argon_aux
				.store_notebook_result(
					audit,
					Default::default(),
					details,
					notebook_number.saturating_sub(1),
					tick.saturating_sub(1),
				)
				.expect("store notebook result");

			for (key, value) in &changes {
				if value.is_some() {
					aux.insert_aux(vec![&(&key[..], &value.clone().unwrap()[..])], &[]).unwrap();
				} else {
					aux.insert_aux(vec![], &[&key[..]]).unwrap();
				}
			}

			let state =
				argon_aux.get_notebook_tick_state(tick).expect("get notebook tick state").get();
			assert_eq!(state.notebook_key_details_by_notary.len(), 1);

			if i > OLDEST_TICK_STATE {
				let cleaned_tick = tick.saturating_sub(OLDEST_TICK_STATE);
				let oldest_state = argon_aux.get_notebook_tick_state(cleaned_tick).unwrap();
				assert!(
					changes
						.iter()
						.any(|a| a.0 == AuxKey::NotaryStateAtTick(cleaned_tick).encode() &&
							a.1.is_none())
				);
				assert_eq!(oldest_state.get().notebook_key_details_by_notary.len(), 0);
				assert_eq!(
					aux.get_aux(&AuxKey::NotaryStateAtTick(cleaned_tick).encode()[..]).unwrap(),
					None
				);
			}
		}
	}

	#[test]
	fn it_should_clean_old_summaries() {
		let aux = Arc::new(MockAux::default());
		let argon_aux = ArgonAux::<Block, _>::new(aux.clone());
		let starting_notebook_number = 500 as NotebookNumber;
		argon_aux
			.get_audit_summaries(1)
			.unwrap()
			.mutate(|a| {
				for i in 1..=(MAX_EXTRA_SUMMARY_HISTORY as NotebookNumber) + 4 {
					a.push(NotaryNotebookAuditSummary {
						notary_id: 1,
						notebook_number: starting_notebook_number + i as NotebookNumber,
						tick: 1,
						version: 0,
						raw_data: vec![],
					});
				}
			})
			.unwrap();
		let next_id = starting_notebook_number + MAX_EXTRA_SUMMARY_HISTORY as u32 + 5;
		let audit_10 = NotebookAuditResult {
			notebook_number: next_id,
			tick: 1,
			notary_id: 1,
			audit_first_failure: None,
		};
		let details_10 = NotaryNotebookDetails {
			notary_id: 1,
			block_voting_power: 0,
			tick: 1,
			notebook_number: next_id,
			raw_audit_summary: vec![],
			version: 1,
			block_votes_count: 0,
			blocks_with_votes: vec![],
			header_hash: H256::zero(),
		};
		let finalized_notebook_number = next_id - 2;

		argon_aux
			.store_notebook_result(
				audit_10.clone(),
				Default::default(),
				details_10.clone(),
				finalized_notebook_number,
				1,
			)
			.expect("store notebook result");
		assert_eq!(
			argon_aux.get_audit_summaries(1).expect("get audit summaries").get().len() as u32,
			(next_id - finalized_notebook_number) + MAX_EXTRA_SUMMARY_HISTORY as u32
		);
		assert_eq!(
			argon_aux
				.get_audit_summaries(1)
				.expect("get audit summaries")
				.get()
				.last()
				.unwrap()
				.notebook_number,
			next_id
		);
	}

	#[test]
	fn it_returns_if_audits_successful() {
		let aux = Arc::new(MockAux::default());
		let argon_aux = ArgonAux::<Block, _>::new(aux.clone());
		let audit_10 = NotebookAuditResult {
			notebook_number: 10,
			tick: 1,
			notary_id: 1,
			audit_first_failure: None,
		};
		let details_10 = NotaryNotebookDetails {
			notary_id: 1,
			block_voting_power: 0,
			tick: 1,
			notebook_number: 10,
			raw_audit_summary: vec![],
			version: 1,
			block_votes_count: 0,
			blocks_with_votes: vec![],
			header_hash: H256::zero(),
		};

		argon_aux
			.store_notebook_result(audit_10.clone(), Default::default(), details_10.clone(), 3, 1)
			.expect("store notebook result");
		assert!(argon_aux.has_successful_audit(1, 10));
		assert!(!argon_aux.has_successful_audit(1, 9));
		argon_aux
			.get_notary_audit_history(1)
			.expect("get audit summaries")
			.mutate(|a| {
				a.get_mut(&10).unwrap().audit_first_failure =
					Some(NotebookVerifyError::InvalidSecretProvided);
			})
			.expect("mutate");
		assert!(!argon_aux.has_successful_audit(1, 10));
	}

	#[test]
	fn can_reprocess_a_notebook() {
		let aux = Arc::new(MockAux::default());
		let argon_aux = ArgonAux::<Block, _>::new(aux.clone());
		for i in 1..=10 {
			let audit_first_failure =
				if i == 10 { Some(NotebookVerifyError::InvalidSecretProvided) } else { None };
			let audit = NotebookAuditResult {
				notebook_number: i,
				tick: 1,
				notary_id: 1,
				audit_first_failure,
			};
			let details = NotaryNotebookDetails {
				notary_id: 1,
				block_voting_power: 0,
				tick: i as Tick,
				notebook_number: i,
				raw_audit_summary: vec![],
				version: 1,
				block_votes_count: 0,
				blocks_with_votes: vec![],
				header_hash: H256::zero(),
			};
			argon_aux
				.store_notebook_result(audit, Default::default(), details, 0, 0)
				.expect("store notebook result");
		}
		assert_eq!(
			argon_aux
				.get_notary_audit_history(1)
				.expect("get notary audit history")
				.get()
				.len(),
			10
		);

		assert_eq!(
			argon_aux.get_notary_audit_history(1).expect("get notary audit history").get()[&10]
				.audit_first_failure,
			Some(NotebookVerifyError::InvalidSecretProvided)
		);
		argon_aux.reprocess_notebook(1, 10).expect("reprocess notebook");

		assert_eq!(
			argon_aux
				.get_notary_audit_history(1)
				.expect("get notary audit history")
				.get()
				.len(),
			9
		);
		assert_eq!(
			argon_aux.get_notary_audit_history(1).expect("get notary audit history").get()[&9]
				.notebook_number,
			9
		);
		assert_eq!(argon_aux.get_audit_summaries(1).expect("get audit summaries").get().len(), 9);
		assert_eq!(
			argon_aux
				.get_notebook_tick_state(1)
				.expect("get notebook tick state")
				.get()
				.notebook_key_details_by_notary
				.get(&10),
			None
		);

		argon_aux
			.store_notebook_result(
				NotebookAuditResult {
					notebook_number: 10,
					tick: 1,
					notary_id: 1,
					audit_first_failure: None,
				},
				Default::default(),
				NotaryNotebookDetails {
					notary_id: 1,
					block_voting_power: 0,
					tick: 10,
					notebook_number: 10,
					raw_audit_summary: vec![],
					version: 1,
					block_votes_count: 0,
					blocks_with_votes: vec![],
					header_hash: H256::zero(),
				},
				0,
				0,
			)
			.expect("store notebook result");
		assert_eq!(
			argon_aux
				.get_notary_audit_history(1)
				.expect("get notary audit history")
				.get()
				.len(),
			10
		);
		assert_eq!(
			argon_aux.get_notary_audit_history(1).expect("get notary audit history").get()[&10]
				.audit_first_failure,
			None
		);
	}
}
