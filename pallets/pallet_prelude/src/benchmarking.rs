#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use alloc::{
	collections::{BTreeMap, BTreeSet},
	vec,
	vec::Vec,
};
use argon_primitives::{
	ArgonCPI, NotaryId, NotebookNumber, NotebookSecret, OperationalRewardPayout, PriceProvider,
	UtxoLockEvents, VaultId, VotingSchedule,
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinNetwork, BitcoinXPub,
		CompressedBitcoinPubkey, Satoshis, UtxoId, UtxoRef,
	},
	block_seal::MiningAuthority,
	digests::{
		AUTHOR_DIGEST_ID, BLOCK_VOTES_DIGEST_ID, BlockVoteDigest, NOTEBOOKS_DIGEST_ID,
		NotebookDigest, TICK_DIGEST_ID,
	},
	notary::{NotaryProvider, NotarySignature},
	providers::{
		AuthorityProvider, BitcoinUtxoTracker, BlockRewardAccountsProvider, BlockSealerInfo,
		BlockSealerProvider, NotebookProvider, OperationalRewardsPayer, OperationalRewardsProvider,
	},
	tick::{Tick, TickDigest, Ticker},
	vault::{
		BitcoinVaultProvider, LockExtension, RegistrationVaultData, Securitization,
		TreasuryVaultProvider, Vault, VaultError, VaultTreasuryFrameEarnings,
	},
};
use codec::{Decode, Encode, FullCodec, HasCompact};
use core::{iter::Sum, marker::PhantomData};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BenchmarkNotebookProviderCallCounters {
	pub notebooks_in_block: u32,
	pub vote_eligible_notebook_count: u32,
	pub eligible_notebooks_for_vote: u32,
	pub get_eligible_tick_votes_root: u32,
	pub is_notary_locked_at_tick: u32,
}

#[derive(Clone, Default)]
pub struct BenchmarkNotebookProviderState {
	pub notebooks_in_block: Vec<(NotaryId, NotebookNumber, Tick)>,
	pub notebooks_by_tick: BTreeMap<Tick, Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)>>,
	pub eligible_vote_roots: BTreeMap<(NotaryId, Tick), (H256, NotebookNumber)>,
	pub notary_locked_from_tick: BTreeMap<NotaryId, Tick>,
	pub call_counters: BenchmarkNotebookProviderCallCounters,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BenchmarkOperationalAccountsProviderCallCounters {
	pub get_registration_vault_data: u32,
	pub has_active_rewards_account_seat: u32,
	pub has_bond_participation: u32,
	pub requires_uniswap_transfer: u32,
	pub account_became_operational: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BenchmarkOperationalAccountsProviderState {
	pub vault_registration_data: Option<RegistrationVaultData<u128>>,
	pub has_active_rewards_account_seat: bool,
	pub has_bond_participation: bool,
	pub requires_uniswap_transfer: bool,
	pub call_counters: BenchmarkOperationalAccountsProviderCallCounters,
}

#[derive(Clone, Encode, Decode, PartialEq)]
pub struct BenchmarkOperationalRewardsProviderState<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + Copy,
{
	pub pending_rewards: Vec<OperationalRewardPayout<AccountId, Balance>>,
	pub paid_rewards: Vec<OperationalRewardPayout<AccountId, Balance>>,
	pub max_pending_rewards: u32,
}

impl<AccountId, Balance> Default for BenchmarkOperationalRewardsProviderState<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + Copy,
{
	fn default() -> Self {
		Self { pending_rewards: Vec::new(), paid_rewards: Vec::new(), max_pending_rewards: 0 }
	}
}

pub fn set_all_digests<T, VerifyError>(
	author: T::AccountId,
	tick: Tick,
	block_vote: BlockVoteDigest,
	notebooks: NotebookDigest<VerifyError>,
) where
	T: frame_system::Config,
	T::AccountId: Codec,
	VerifyError: Codec + MaxEncodedLen,
{
	let digest = frame_system::Pallet::<T>::digest();
	let has_author = digest
		.logs
		.iter()
		.any(|entry| matches!(entry, DigestItem::PreRuntime(id, _) if *id == AUTHOR_DIGEST_ID));
	let has_tick = digest
		.logs
		.iter()
		.any(|entry| matches!(entry, DigestItem::PreRuntime(id, _) if *id == TICK_DIGEST_ID));
	let has_votes = digest.logs.iter().any(
		|entry| matches!(entry, DigestItem::PreRuntime(id, _) if *id == BLOCK_VOTES_DIGEST_ID),
	);
	let has_notebooks = digest
		.logs
		.iter()
		.any(|entry| matches!(entry, DigestItem::PreRuntime(id, _) if *id == NOTEBOOKS_DIGEST_ID));

	if !has_author {
		frame_system::Pallet::<T>::deposit_log(DigestItem::PreRuntime(
			AUTHOR_DIGEST_ID,
			author.encode(),
		));
	}
	if !has_tick {
		frame_system::Pallet::<T>::deposit_log(DigestItem::PreRuntime(
			TICK_DIGEST_ID,
			TickDigest(tick).encode(),
		));
	}
	if !has_votes {
		frame_system::Pallet::<T>::deposit_log(DigestItem::PreRuntime(
			BLOCK_VOTES_DIGEST_ID,
			block_vote.encode(),
		));
	}
	if !has_notebooks {
		frame_system::Pallet::<T>::deposit_log(DigestItem::PreRuntime(
			NOTEBOOKS_DIGEST_ID,
			notebooks.encode(),
		));
	}
}

pub struct BenchmarkNotaryProvider;
impl<B: BlockT, AccountId: Decode> NotaryProvider<B, AccountId> for BenchmarkNotaryProvider {
	type Weights = ();

	fn verify_signature(
		_notary_id: NotaryId,
		_at_tick: Tick,
		_message: &H256,
		_signature: &NotarySignature,
	) -> bool {
		// RFC 8032 test vector #1 (empty message): real ed25519 verify cost without runtime state
		// dependencies.
		let public = polkadot_sdk::sp_core::ed25519::Public::from_raw([
			0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
			0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
			0xf7, 0x07, 0x51, 0x1a,
		]);
		let signature = NotarySignature::from_raw([
			0xe5, 0x56, 0x43, 0x00, 0xc3, 0x60, 0xac, 0x72, 0x90, 0x86, 0xe2, 0xcc, 0x80, 0x6e,
			0x82, 0x8a, 0x84, 0x87, 0x7f, 0x1e, 0xb8, 0xe5, 0xd9, 0x74, 0xd8, 0x73, 0xe0, 0x65,
			0x22, 0x49, 0x01, 0x55, 0x5f, 0xb8, 0x82, 0x15, 0x90, 0xa3, 0x3b, 0xac, 0xc6, 0x1e,
			0x39, 0x70, 0x1c, 0xf9, 0xb4, 0x6b, 0xd2, 0x5b, 0xf5, 0xf0, 0x59, 0x5b, 0xbe, 0x24,
			0x65, 0x51, 0x41, 0x43, 0x8e, 0x7a, 0x10, 0x0b,
		]);
		polkadot_sdk::sp_io::crypto::ed25519_verify(&signature, &[], &public)
	}

	fn active_notaries() -> Vec<NotaryId> {
		vec![1]
	}

	fn notary_operator_account_id(_notary_id: NotaryId) -> Option<AccountId> {
		let bytes = [0u8; 32];
		AccountId::decode(&mut &bytes[..]).ok()
	}
}

fn benchmark_authority_account_id<AccountId: Decode>() -> AccountId {
	let bytes = [1u8; 32];
	AccountId::decode(&mut &bytes[..]).expect("benchmark authority account id must decode")
}

fn benchmark_authority_id<AuthorityId: Decode>() -> AuthorityId {
	let bytes = [2u8; 32];
	AuthorityId::decode(&mut &bytes[..]).expect("benchmark authority id must decode")
}

pub struct BenchmarkAuthorityProvider<T, AuthorityId>(PhantomData<(T, AuthorityId)>);
impl<T, AuthorityId> AuthorityProvider<AuthorityId, T::Block, T::AccountId>
	for BenchmarkAuthorityProvider<T, AuthorityId>
where
	T: frame_system::Config,
	T::AccountId: Decode + PartialEq,
	AuthorityId: Decode + PartialEq,
{
	fn authority_count() -> u32 {
		1
	}

	fn get_authority(author: T::AccountId) -> Option<AuthorityId> {
		if author == benchmark_authority_account_id::<T::AccountId>() {
			Some(benchmark_authority_id::<AuthorityId>())
		} else {
			None
		}
	}

	fn get_winning_managed_authority(
		seal_proof: U256,
		signing_key: Option<AuthorityId>,
		best_miner_nonce_score: Option<U256>,
	) -> Option<(MiningAuthority<AuthorityId, T::AccountId>, U256, Permill)> {
		let authority_id = benchmark_authority_id::<AuthorityId>();
		if let Some(expected_authority) = signing_key {
			if expected_authority != authority_id {
				return None;
			}
		}
		let score = seal_proof;
		if let Some(best_score) = best_miner_nonce_score {
			if best_score <= score {
				return None;
			}
		}
		Some((
			MiningAuthority {
				authority_index: (1, 0),
				authority_id,
				account_id: benchmark_authority_account_id::<T::AccountId>(),
			},
			score,
			Permill::zero(),
		))
	}

	fn get_authority_score(
		seal_proof: U256,
		authority_id: &AuthorityId,
		account: &T::AccountId,
		_ref_block_number: NumberFor<T::Block>,
	) -> Option<U256> {
		if *authority_id == benchmark_authority_id::<AuthorityId>() &&
			*account == benchmark_authority_account_id::<T::AccountId>()
		{
			Some(seal_proof)
		} else {
			None
		}
	}
}

fn notebook_entries_from_digests<T: frame_system::Config>() -> Vec<(NotaryId, NotebookNumber, Tick)>
{
	let digest = frame_system::Pallet::<T>::digest();
	for entry in digest.logs.iter() {
		if let DigestItem::PreRuntime(id, data) = entry {
			if *id == NOTEBOOKS_DIGEST_ID {
				if let Ok(notebook_digest) = NotebookDigest::<()>::decode(&mut &data[..]) {
					return notebook_digest
						.notebooks
						.into_inner()
						.into_iter()
						.map(|notebook| {
							(notebook.notary_id, notebook.notebook_number, notebook.tick)
						})
						.collect();
				}
			}
		}
	}
	Vec::new()
}

pub struct BenchmarkNotebookProvider<T>(PhantomData<T>);
impl<T: frame_system::Config> NotebookProvider for BenchmarkNotebookProvider<T> {
	type Weights = ();

	fn get_eligible_tick_votes_root(
		notary_id: NotaryId,
		tick: Tick,
	) -> Option<(H256, NotebookNumber)> {
		let mut state = benchmark_notebook_provider_state();
		state.call_counters.get_eligible_tick_votes_root =
			state.call_counters.get_eligible_tick_votes_root.saturating_add(1);
		let mut result = state.eligible_vote_roots.get(&(notary_id, tick)).copied();
		if result.is_none() {
			result = notebook_entries_from_digests::<T>()
				.into_iter()
				.find(|(digest_notary_id, _, digest_tick)| {
					*digest_notary_id == notary_id && *digest_tick == tick
				})
				.map(|(_, notebook_number, _)| {
					(synthetic_benchmark_votes_root(notary_id, tick), notebook_number)
				});
		}
		set_benchmark_notebook_provider_state(state);
		result
	}

	fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)> {
		let mut state = benchmark_notebook_provider_state();
		state.call_counters.notebooks_in_block =
			state.call_counters.notebooks_in_block.saturating_add(1);
		let notebooks = if state.notebooks_in_block.is_empty() {
			notebook_entries_from_digests::<T>()
		} else {
			state.notebooks_in_block.clone()
		};
		set_benchmark_notebook_provider_state(state);
		notebooks
	}

	fn vote_eligible_notebook_count(voting_schedule: &VotingSchedule) -> u32 {
		let mut state = benchmark_notebook_provider_state();
		state.call_counters.vote_eligible_notebook_count =
			state.call_counters.vote_eligible_notebook_count.saturating_add(1);
		let notebook_tick = voting_schedule.notebook_tick();
		let count = if state.notebooks_by_tick.contains_key(&notebook_tick) {
			state
				.notebooks_by_tick
				.get(&notebook_tick)
				.map(|entries| entries.len() as u32)
				.unwrap_or_default()
		} else {
			notebook_entries_from_digests::<T>()
				.into_iter()
				.filter(|(_, _, tick)| *tick == notebook_tick)
				.count() as u32
		};
		set_benchmark_notebook_provider_state(state);
		count
	}

	fn eligible_notebooks_for_vote(
		voting_schedule: &VotingSchedule,
	) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)> {
		let mut state = benchmark_notebook_provider_state();
		state.call_counters.eligible_notebooks_for_vote =
			state.call_counters.eligible_notebooks_for_vote.saturating_add(1);
		let notebook_tick = voting_schedule.notebook_tick();
		let notebooks = if let Some(notebooks) = state.notebooks_by_tick.get(&notebook_tick) {
			notebooks.clone()
		} else {
			notebook_entries_from_digests::<T>()
				.into_iter()
				.filter(|(_, _, tick)| *tick == notebook_tick)
				.map(|(notary_id, notebook_number, _)| {
					(notary_id, notebook_number, Some(synthetic_benchmark_parent_secret(notary_id)))
				})
				.collect()
		};
		set_benchmark_notebook_provider_state(state);
		notebooks
	}

	fn is_notary_locked_at_tick(notary_id: NotaryId, tick: Tick) -> bool {
		let mut state = benchmark_notebook_provider_state();
		state.call_counters.is_notary_locked_at_tick =
			state.call_counters.is_notary_locked_at_tick.saturating_add(1);
		let is_locked = state
			.notary_locked_from_tick
			.get(&notary_id)
			.map(|locked_from_tick| *locked_from_tick <= tick)
			.unwrap_or(false);
		set_benchmark_notebook_provider_state(state);
		is_locked
	}
}

pub struct BenchmarkTickProvider;
impl<B: BlockT> argon_primitives::TickProvider<B> for BenchmarkTickProvider {
	type Weights = ();

	fn previous_tick() -> Tick {
		14
	}

	fn current_tick() -> Tick {
		15
	}

	fn elapsed_ticks() -> Tick {
		10
	}

	fn voting_schedule() -> VotingSchedule {
		VotingSchedule::from_runtime_current_tick(
			<Self as argon_primitives::TickProvider<B>>::current_tick(),
		)
	}

	fn ticker() -> Ticker {
		Ticker::new(1000, 10)
	}

	fn blocks_at_tick(_tick: Tick) -> Vec<B::Hash> {
		Vec::new()
	}
}

pub struct BenchmarkBlockSealerProvider<AccountId>(PhantomData<AccountId>);
impl<AccountId: FullCodec> BlockSealerProvider<AccountId>
	for BenchmarkBlockSealerProvider<AccountId>
{
	type Weights = ();

	fn get_sealer_info() -> BlockSealerInfo<AccountId> {
		BlockSealerInfo {
			block_author_account_id: benchmark_authority_account_id::<AccountId>(),
			block_vote_rewards_account: None,
			block_seal_authority: None,
		}
	}

	fn is_block_vote_seal() -> bool {
		false
	}
}

pub struct BenchmarkBlockRewardAccountsProvider<AccountId>(PhantomData<AccountId>);
impl<AccountId: FullCodec> BlockRewardAccountsProvider<AccountId>
	for BenchmarkBlockRewardAccountsProvider<AccountId>
{
	fn get_block_rewards_account(_author: &AccountId) -> Option<(AccountId, FrameId)> {
		None
	}

	fn get_mint_rewards_accounts() -> Vec<(AccountId, FrameId)> {
		Vec::new()
	}

	fn is_compute_block_eligible_for_rewards() -> bool {
		true
	}
}

pub fn set_benchmark_notebook_provider_state(state: BenchmarkNotebookProviderState) {
	state_backend::set(state);
}

pub fn reset_benchmark_notebook_provider_state() {
	state_backend::reset();
}

pub fn benchmark_notebook_provider_state() -> BenchmarkNotebookProviderState {
	state_backend::get()
}

pub fn benchmark_notebook_provider_call_counters() -> BenchmarkNotebookProviderCallCounters {
	benchmark_notebook_provider_state().call_counters
}

pub fn reset_benchmark_notebook_provider_call_counters() {
	let mut state = benchmark_notebook_provider_state();
	state.call_counters = BenchmarkNotebookProviderCallCounters::default();
	set_benchmark_notebook_provider_state(state);
}

pub fn set_benchmark_operational_accounts_provider_state(
	state: BenchmarkOperationalAccountsProviderState,
) {
	operational_accounts_state_backend::set(state);
}

pub fn reset_benchmark_operational_accounts_provider_state() {
	operational_accounts_state_backend::reset();
}

pub fn benchmark_operational_accounts_provider_state() -> BenchmarkOperationalAccountsProviderState
{
	operational_accounts_state_backend::get()
}

pub fn benchmark_operational_accounts_provider_call_counters()
-> BenchmarkOperationalAccountsProviderCallCounters {
	benchmark_operational_accounts_provider_state().call_counters
}

pub fn reset_benchmark_operational_accounts_provider_call_counters() {
	let mut state = benchmark_operational_accounts_provider_state();
	state.call_counters = BenchmarkOperationalAccountsProviderCallCounters::default();
	set_benchmark_operational_accounts_provider_state(state);
}

pub fn set_benchmark_operational_rewards_provider_state<AccountId, Balance>(
	state: BenchmarkOperationalRewardsProviderState<AccountId, Balance>,
) where
	AccountId: Codec,
	Balance: Codec + Copy,
{
	operational_rewards_provider_state_backend::set(state.encode());
}

pub fn reset_benchmark_operational_rewards_provider_state() {
	operational_rewards_provider_state_backend::reset();
}

pub fn benchmark_operational_rewards_provider_state<AccountId, Balance>()
-> BenchmarkOperationalRewardsProviderState<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + Copy,
{
	decode_benchmark_state(operational_rewards_provider_state_backend::get())
}

fn mutate_benchmark_operational_rewards_provider_state<AccountId, Balance, ResultT>(
	f: impl FnOnce(&mut BenchmarkOperationalRewardsProviderState<AccountId, Balance>) -> ResultT,
) -> ResultT
where
	AccountId: Codec + Clone + PartialEq,
	Balance: Codec + Copy + Ord,
{
	let mut state = benchmark_operational_rewards_provider_state::<AccountId, Balance>();
	let result = f(&mut state);
	set_benchmark_operational_rewards_provider_state(state);
	result
}

pub struct BenchmarkOperationalRewardsProvider<AccountId, Balance, MaxPendingRewards = ConstU32<0>>(
	PhantomData<(AccountId, Balance, MaxPendingRewards)>,
);

impl<AccountId, Balance, MaxPendingRewards> OperationalRewardsProvider<AccountId, Balance>
	for BenchmarkOperationalRewardsProvider<AccountId, Balance, MaxPendingRewards>
where
	AccountId: FullCodec + Clone + PartialEq,
	Balance: FullCodec + Copy + Ord,
	MaxPendingRewards: Get<u32>,
{
	type Weights = ();

	fn pending_rewards() -> Vec<OperationalRewardPayout<AccountId, Balance>> {
		benchmark_operational_rewards_provider_state::<AccountId, Balance>().pending_rewards
	}

	fn max_pending_rewards() -> u32 {
		let configured_max = benchmark_operational_rewards_provider_state::<AccountId, Balance>()
			.max_pending_rewards;
		if configured_max > 0 { configured_max } else { MaxPendingRewards::get() }
	}

	fn mark_reward_paid(
		reward: &OperationalRewardPayout<AccountId, Balance>,
		amount_paid: Balance,
	) {
		mutate_benchmark_operational_rewards_provider_state::<AccountId, Balance, _>(|state| {
			let Some(index) = state.pending_rewards.iter().position(|entry| entry == reward) else {
				return;
			};
			let mut paid_reward = state.pending_rewards.remove(index);
			paid_reward.amount = paid_reward.amount.min(amount_paid);
			state.paid_rewards.push(paid_reward);
		});
	}
}

pub struct BenchmarkOperationalRewardsPayer;

impl<AccountId, Balance> OperationalRewardsPayer<AccountId, Balance>
	for BenchmarkOperationalRewardsPayer
where
	AccountId: FullCodec,
	Balance: FullCodec,
{
	fn claim_reward(_account_id: &AccountId, _amount: Balance) -> DispatchResult {
		Ok(())
	}
}

pub fn synthetic_benchmark_votes_root(notary_id: NotaryId, tick: Tick) -> H256 {
	let mut bytes = [0u8; 32];
	bytes[..4].copy_from_slice(&notary_id.to_be_bytes());
	let tick_bytes = tick.to_be_bytes();
	bytes[4..(4 + tick_bytes.len())].copy_from_slice(&tick_bytes);
	H256::from(bytes)
}

pub fn synthetic_benchmark_parent_secret(notary_id: NotaryId) -> NotebookSecret {
	H256::repeat_byte((notary_id % 251) as u8 + 1)
}

#[cfg(feature = "std")]
mod state_backend {
	use super::*;
	use frame_support::parameter_types;
	parameter_types! {
		pub static BenchmarkNotebookProviderStateHolder: BenchmarkNotebookProviderState =
			BenchmarkNotebookProviderState::default();
	}

	pub(super) fn set(state: BenchmarkNotebookProviderState) {
		BenchmarkNotebookProviderStateHolder::set(state);
	}

	pub(super) fn reset() {
		BenchmarkNotebookProviderStateHolder::reset();
	}

	pub(super) fn get() -> BenchmarkNotebookProviderState {
		BenchmarkNotebookProviderStateHolder::get()
	}
}

#[cfg(feature = "std")]
mod operational_accounts_state_backend {
	use super::*;
	use frame_support::parameter_types;

	parameter_types! {
		pub static BenchmarkOperationalAccountsProviderStateHolder:
			BenchmarkOperationalAccountsProviderState =
				BenchmarkOperationalAccountsProviderState::default();
	}

	pub(super) fn set(state: BenchmarkOperationalAccountsProviderState) {
		BenchmarkOperationalAccountsProviderStateHolder::set(state);
	}

	pub(super) fn reset() {
		BenchmarkOperationalAccountsProviderStateHolder::reset();
	}

	pub(super) fn get() -> BenchmarkOperationalAccountsProviderState {
		BenchmarkOperationalAccountsProviderStateHolder::get()
	}
}

#[cfg(feature = "std")]
mod operational_rewards_provider_state_backend {
	use super::*;
	use frame_support::parameter_types;

	parameter_types! {
		pub static BenchmarkOperationalRewardsProviderStateHolder: Vec<u8> = Vec::new();
	}

	pub(super) fn set(state: Vec<u8>) {
		BenchmarkOperationalRewardsProviderStateHolder::set(state);
	}

	pub(super) fn reset() {
		BenchmarkOperationalRewardsProviderStateHolder::reset();
	}

	pub(super) fn get() -> Vec<u8> {
		BenchmarkOperationalRewardsProviderStateHolder::get()
	}
}

#[cfg(not(feature = "std"))]
mod state_backend {
	use super::*;
	use core::cell::UnsafeCell;

	// Runtime-benchmarks-only in-memory state for WASM execution (no `std`, so no
	// `parameter_types! { static ... }`/`thread_local!`). This is never enabled in production
	// runtime builds.
	struct BenchmarkStateCell(UnsafeCell<Option<BenchmarkNotebookProviderState>>);
	unsafe impl Sync for BenchmarkStateCell {}

	static BENCHMARK_NOTEBOOK_PROVIDER_STATE: BenchmarkStateCell =
		BenchmarkStateCell(UnsafeCell::new(None));

	pub(super) fn set(state: BenchmarkNotebookProviderState) {
		unsafe {
			*BENCHMARK_NOTEBOOK_PROVIDER_STATE.0.get() = Some(state);
		}
	}

	pub(super) fn reset() {
		set(BenchmarkNotebookProviderState::default());
	}

	pub(super) fn get() -> BenchmarkNotebookProviderState {
		unsafe { (*BENCHMARK_NOTEBOOK_PROVIDER_STATE.0.get()).clone().unwrap_or_default() }
	}
}

#[cfg(not(feature = "std"))]
mod operational_accounts_state_backend {
	use super::*;
	use core::cell::UnsafeCell;

	struct BenchmarkStateCell(UnsafeCell<Option<BenchmarkOperationalAccountsProviderState>>);
	unsafe impl Sync for BenchmarkStateCell {}

	static BENCHMARK_OPERATIONAL_ACCOUNTS_PROVIDER_STATE: BenchmarkStateCell =
		BenchmarkStateCell(UnsafeCell::new(None));

	pub(super) fn set(state: BenchmarkOperationalAccountsProviderState) {
		unsafe {
			*BENCHMARK_OPERATIONAL_ACCOUNTS_PROVIDER_STATE.0.get() = Some(state);
		}
	}

	pub(super) fn reset() {
		set(BenchmarkOperationalAccountsProviderState::default());
	}

	pub(super) fn get() -> BenchmarkOperationalAccountsProviderState {
		unsafe {
			(*BENCHMARK_OPERATIONAL_ACCOUNTS_PROVIDER_STATE.0.get())
				.clone()
				.unwrap_or_default()
		}
	}
}

#[cfg(not(feature = "std"))]
mod operational_rewards_provider_state_backend {
	use super::*;
	use core::cell::UnsafeCell;

	struct BenchmarkStateCell(UnsafeCell<Option<Vec<u8>>>);
	unsafe impl Sync for BenchmarkStateCell {}

	static BENCHMARK_OPERATIONAL_REWARDS_PROVIDER_STATE: BenchmarkStateCell =
		BenchmarkStateCell(UnsafeCell::new(None));

	pub(super) fn set(state: Vec<u8>) {
		unsafe {
			*BENCHMARK_OPERATIONAL_REWARDS_PROVIDER_STATE.0.get() = Some(state);
		}
	}

	pub(super) fn reset() {
		unsafe {
			*BENCHMARK_OPERATIONAL_REWARDS_PROVIDER_STATE.0.get() = None;
		}
	}

	pub(super) fn get() -> Vec<u8> {
		unsafe {
			(*BENCHMARK_OPERATIONAL_REWARDS_PROVIDER_STATE.0.get())
				.clone()
				.unwrap_or_default()
		}
	}
}

fn decode_benchmark_state<State: Decode + Default>(encoded: Vec<u8>) -> State {
	if encoded.is_empty() {
		State::default()
	} else {
		State::decode(&mut &encoded[..]).unwrap_or_default()
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
pub struct BenchmarkPriceProviderState {
	pub btc_price_in_usd: Option<FixedU128>,
	pub argon_price_in_usd: Option<FixedU128>,
	pub argon_target_price_in_usd: Option<FixedU128>,
	pub circulation: u128,
}

impl Default for BenchmarkPriceProviderState {
	fn default() -> Self {
		Self {
			btc_price_in_usd: Some(FixedU128::from_rational(62_000_00u128, 100u128)),
			argon_price_in_usd: Some(FixedU128::one()),
			argon_target_price_in_usd: Some(FixedU128::one()),
			circulation: 1_000,
		}
	}
}

pub fn set_benchmark_price_provider_state(state: BenchmarkPriceProviderState) {
	price_state_backend::set(state.encode());
}

pub fn reset_benchmark_price_provider_state() {
	price_state_backend::reset();
}

pub fn benchmark_price_provider_state() -> BenchmarkPriceProviderState {
	decode_benchmark_state(price_state_backend::get())
}

pub struct BenchmarkPriceProvider<Balance>(PhantomData<Balance>);
impl<Balance> PriceProvider<Balance> for BenchmarkPriceProvider<Balance>
where
	Balance:
		Codec + Copy + AtLeast32BitUnsigned + Into<u128> + From<u128> + HasCompact + MaxEncodedLen,
{
	fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
		benchmark_price_provider_state().btc_price_in_usd
	}

	fn get_latest_argon_price_in_usd() -> Option<FixedU128> {
		benchmark_price_provider_state().argon_price_in_usd
	}

	fn get_argon_cpi() -> Option<ArgonCPI> {
		let state = benchmark_price_provider_state();
		let ratio = state.argon_target_price_in_usd? / state.argon_price_in_usd?;
		let ratio_as_cpi = ArgonCPI::from_inner(ratio.into_inner() as i128);
		Some(ratio_as_cpi - One::one())
	}

	fn get_average_cpi_for_ticks(_tick_range: (Tick, Tick)) -> ArgonCPI {
		Self::get_argon_cpi().unwrap_or_default()
	}

	fn get_circulation() -> Balance {
		benchmark_price_provider_state().circulation.into()
	}

	fn get_redemption_r_value() -> Option<FixedU128> {
		let state = benchmark_price_provider_state();
		Some(state.argon_price_in_usd? / state.argon_target_price_in_usd?)
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
pub struct BenchmarkBitcoinUtxoTrackerState {
	pub watched_utxos_by_id: BTreeMap<UtxoId, (BitcoinCosignScriptPubkey, Satoshis, BitcoinHeight)>,
	pub funding_utxo_refs_by_id: BTreeMap<UtxoId, UtxoRef>,
	pub candidate_utxos_by_ref: BTreeMap<UtxoRef, (UtxoId, Satoshis)>,
	pub bitcoin_network: BitcoinNetwork,
	pub bitcoin_block_height_change: (BitcoinHeight, BitcoinHeight),
}

impl Default for BenchmarkBitcoinUtxoTrackerState {
	fn default() -> Self {
		Self {
			watched_utxos_by_id: BTreeMap::new(),
			funding_utxo_refs_by_id: BTreeMap::new(),
			candidate_utxos_by_ref: BTreeMap::new(),
			bitcoin_network: BitcoinNetwork::Regtest,
			bitcoin_block_height_change: (0, 0),
		}
	}
}

pub fn set_benchmark_bitcoin_utxo_tracker_state(state: BenchmarkBitcoinUtxoTrackerState) {
	bitcoin_utxo_tracker_state_backend::set(state.encode());
}

pub fn reset_benchmark_bitcoin_utxo_tracker_state() {
	bitcoin_utxo_tracker_state_backend::reset();
}

pub fn benchmark_bitcoin_utxo_tracker_state() -> BenchmarkBitcoinUtxoTrackerState {
	decode_benchmark_state(bitcoin_utxo_tracker_state_backend::get())
}

pub struct BenchmarkBitcoinUtxoTracker;
impl BitcoinUtxoTracker for BenchmarkBitcoinUtxoTracker {
	fn watch_for_utxo(
		utxo_id: UtxoId,
		script_pubkey: BitcoinCosignScriptPubkey,
		satoshis: Satoshis,
		watch_for_spent_until: BitcoinHeight,
	) -> Result<(), DispatchError> {
		let mut state = benchmark_bitcoin_utxo_tracker_state();
		state
			.watched_utxos_by_id
			.insert(utxo_id, (script_pubkey, satoshis, watch_for_spent_until));
		set_benchmark_bitcoin_utxo_tracker_state(state);
		Ok(())
	}

	fn get_funding_utxo_ref(utxo_id: UtxoId) -> Option<UtxoRef> {
		benchmark_bitcoin_utxo_tracker_state()
			.funding_utxo_refs_by_id
			.get(&utxo_id)
			.cloned()
	}

	fn unwatch(utxo_id: UtxoId) {
		let mut state = benchmark_bitcoin_utxo_tracker_state();
		state.watched_utxos_by_id.remove(&utxo_id);
		state.funding_utxo_refs_by_id.remove(&utxo_id);
		state.candidate_utxos_by_ref.retain(|_, (id, _)| *id != utxo_id);
		set_benchmark_bitcoin_utxo_tracker_state(state);
	}

	fn unwatch_candidate(utxo_id: UtxoId, utxo_ref: &UtxoRef) -> Option<(UtxoRef, Satoshis)> {
		let mut state = benchmark_bitcoin_utxo_tracker_state();
		let mut removed = None;
		if let Some((candidate_utxo_id, satoshis)) = state.candidate_utxos_by_ref.remove(utxo_ref) {
			if candidate_utxo_id == utxo_id {
				removed = Some((utxo_ref.clone(), satoshis));
			} else {
				state
					.candidate_utxos_by_ref
					.insert(utxo_ref.clone(), (candidate_utxo_id, satoshis));
			}
		}
		set_benchmark_bitcoin_utxo_tracker_state(state);
		removed
	}
}

pub struct BenchmarkBitcoinBlockHeightChange;
impl Get<(BitcoinHeight, BitcoinHeight)> for BenchmarkBitcoinBlockHeightChange {
	fn get() -> (BitcoinHeight, BitcoinHeight) {
		benchmark_bitcoin_utxo_tracker_state().bitcoin_block_height_change
	}
}

pub struct BenchmarkBitcoinNetwork;
impl Get<BitcoinNetwork> for BenchmarkBitcoinNetwork {
	fn get() -> BitcoinNetwork {
		benchmark_bitcoin_utxo_tracker_state().bitcoin_network
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
pub struct BenchmarkBitcoinLocksRuntimeState {
	pub current_frame_id: FrameId,
	pub current_tick: Tick,
	pub did_start_new_frame: bool,
}

impl Default for BenchmarkBitcoinLocksRuntimeState {
	fn default() -> Self {
		Self { current_frame_id: 1, current_tick: 1, did_start_new_frame: true }
	}
}

pub fn set_benchmark_bitcoin_locks_runtime_state(state: BenchmarkBitcoinLocksRuntimeState) {
	bitcoin_locks_runtime_state_backend::set(state.encode());
}

pub fn reset_benchmark_bitcoin_locks_runtime_state() {
	bitcoin_locks_runtime_state_backend::reset();
}

pub fn benchmark_bitcoin_locks_runtime_state() -> BenchmarkBitcoinLocksRuntimeState {
	decode_benchmark_state(bitcoin_locks_runtime_state_backend::get())
}

pub struct BenchmarkCurrentFrameId;
impl Get<FrameId> for BenchmarkCurrentFrameId {
	fn get() -> FrameId {
		benchmark_bitcoin_locks_runtime_state().current_frame_id
	}
}

pub struct BenchmarkCurrentTick;
impl Get<Tick> for BenchmarkCurrentTick {
	fn get() -> Tick {
		benchmark_bitcoin_locks_runtime_state().current_tick
	}
}

pub struct BenchmarkDidStartNewFrame;
impl Get<bool> for BenchmarkDidStartNewFrame {
	fn get() -> bool {
		benchmark_bitcoin_locks_runtime_state().did_start_new_frame
	}
}

#[derive(Clone, Encode, Decode, PartialEq)]
pub struct BenchmarkBitcoinVaultProviderState<AccountId, Balance>
where
	AccountId: Codec + Ord,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	pub vaults: BTreeMap<VaultId, Vault<AccountId, Balance>>,
	pub vault_xpubs_by_id: BTreeMap<VaultId, (BitcoinXPub, BitcoinXPub)>,
	pub canceled_locks: Vec<(VaultId, Balance)>,
	pub treasury_frame_earnings: Vec<(VaultId, Balance)>,
	pub charge_fee: bool,
	pub pending_cosigns: BTreeMap<VaultId, BTreeSet<UtxoId>>,
	pub orphaned_utxo_cosigns: BTreeMap<VaultId, BTreeMap<AccountId, u32>>,
}

impl<AccountId, Balance> Default for BenchmarkBitcoinVaultProviderState<AccountId, Balance>
where
	AccountId: Codec + Ord,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	fn default() -> Self {
		Self {
			vaults: BTreeMap::new(),
			vault_xpubs_by_id: BTreeMap::new(),
			canceled_locks: Vec::new(),
			treasury_frame_earnings: Vec::new(),
			charge_fee: false,
			pending_cosigns: BTreeMap::new(),
			orphaned_utxo_cosigns: BTreeMap::new(),
		}
	}
}

pub fn set_benchmark_bitcoin_vault_provider_state<AccountId, Balance>(
	state: BenchmarkBitcoinVaultProviderState<AccountId, Balance>,
) where
	AccountId: Codec + Ord,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	bitcoin_vault_provider_state_backend::set(state.encode());
}

pub fn reset_benchmark_bitcoin_vault_provider_state() {
	bitcoin_vault_provider_state_backend::reset();
}

pub fn benchmark_bitcoin_vault_provider_state<AccountId, Balance>()
-> BenchmarkBitcoinVaultProviderState<AccountId, Balance>
where
	AccountId: Codec + Ord,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	decode_benchmark_state(bitcoin_vault_provider_state_backend::get())
}

fn mutate_benchmark_bitcoin_vault_provider_state<AccountId, Balance, ResultT>(
	f: impl FnOnce(&mut BenchmarkBitcoinVaultProviderState<AccountId, Balance>) -> ResultT,
) -> ResultT
where
	AccountId: Codec + Ord,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	let mut state = benchmark_bitcoin_vault_provider_state::<AccountId, Balance>();
	let result = f(&mut state);
	set_benchmark_bitcoin_vault_provider_state(state);
	result
}

pub struct BenchmarkBitcoinVaultProvider<Currency, AccountId, Balance>(
	PhantomData<(Currency, AccountId, Balance)>,
);
impl<Currency, AccountId, Balance> BitcoinVaultProvider
	for BenchmarkBitcoinVaultProvider<Currency, AccountId, Balance>
where
	Currency: Mutate<AccountId, Balance = Balance>,
	AccountId: Codec + Clone + Ord + PartialEq,
	Balance: Codec
		+ Copy
		+ MaxEncodedLen
		+ Default
		+ AtLeast32BitUnsigned
		+ TypeInfo
		+ Clone
		+ core::fmt::Debug
		+ PartialEq
		+ Eq
		+ Sum,
{
	type Weights = ();
	type Balance = Balance;
	type AccountId = AccountId;

	fn is_owner(vault_id: VaultId, account_id: &Self::AccountId) -> bool {
		benchmark_bitcoin_vault_provider_state::<AccountId, Balance>()
			.vaults
			.get(&vault_id)
			.map(|vault| vault.operator_account_id == *account_id)
			.unwrap_or(false)
	}

	fn can_initialize_bitcoin_locks(vault_id: VaultId, account_id: &Self::AccountId) -> bool {
		benchmark_bitcoin_vault_provider_state::<AccountId, Balance>()
			.vaults
			.get(&vault_id)
			.map(|vault| {
				vault.operator_account_id == *account_id ||
					vault.bitcoin_lock_delegate_account.as_ref() == Some(account_id)
			})
			.unwrap_or(false)
	}

	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId> {
		benchmark_bitcoin_vault_provider_state::<AccountId, Balance>()
			.vaults
			.get(&vault_id)
			.map(|vault| vault.operator_account_id.clone())
	}

	fn get_vault_id(account_id: &Self::AccountId) -> Option<VaultId> {
		benchmark_bitcoin_vault_provider_state::<AccountId, Balance>()
			.vaults
			.iter()
			.find_map(
				|(vault_id, vault)| {
					if vault.operator_account_id == *account_id { Some(*vault_id) } else { None }
				},
			)
	}

	fn get_registration_vault_data(
		account_id: &Self::AccountId,
	) -> Option<RegistrationVaultData<Self::Balance>> {
		benchmark_bitcoin_vault_provider_state::<AccountId, Balance>()
			.vaults
			.iter()
			.find_map(|(vault_id, vault)| {
				(vault.operator_account_id == *account_id).then_some(RegistrationVaultData {
					vault_id: *vault_id,
					activated_securitization: vault.get_activated_securitization(),
					securitization: vault.securitization,
				})
			})
	}

	fn get_securitization_ratio(vault_id: VaultId) -> Result<FixedU128, VaultError> {
		benchmark_bitcoin_vault_provider_state::<AccountId, Balance>()
			.vaults
			.get(&vault_id)
			.map(|vault| vault.securitization_ratio)
			.ok_or(VaultError::VaultNotFound)
	}

	fn add_securitized_satoshis(
		vault_id: VaultId,
		satoshis: Satoshis,
		securitization_ratio: FixedU128,
	) -> Result<(), VaultError> {
		mutate_benchmark_bitcoin_vault_provider_state::<AccountId, Balance, _>(|state| {
			let vault = state.vaults.get_mut(&vault_id).ok_or(VaultError::VaultNotFound)?;
			let securitized_satoshis = securitization_ratio.saturating_mul_int(satoshis);
			vault.locked_satoshis.saturating_accrue(satoshis);
			vault.securitized_satoshis.saturating_accrue(securitized_satoshis);
			Ok(())
		})
	}

	fn reduce_securitized_satoshis(
		vault_id: VaultId,
		satoshis: Satoshis,
		securitization_ratio: FixedU128,
	) -> Result<(), VaultError> {
		mutate_benchmark_bitcoin_vault_provider_state::<AccountId, Balance, _>(|state| {
			let vault = state.vaults.get_mut(&vault_id).ok_or(VaultError::VaultNotFound)?;
			let securitized_satoshis = securitization_ratio.saturating_mul_int(satoshis);
			if vault.locked_satoshis < satoshis || vault.securitized_satoshis < securitized_satoshis
			{
				return Err(VaultError::InternalError);
			}
			vault.locked_satoshis.saturating_reduce(satoshis);
			vault.securitized_satoshis.saturating_reduce(securitized_satoshis);
			Ok(())
		})
	}

	fn lock(
		vault_id: VaultId,
		locker: &Self::AccountId,
		securitization: &Securitization<Self::Balance>,
		_satoshis: Satoshis,
		extension: Option<(FixedU128, &mut LockExtension<Self::Balance>)>,
		_has_fee_coupon: bool,
	) -> Result<Self::Balance, VaultError> {
		let (total_fee, charge_fee) =
			mutate_benchmark_bitcoin_vault_provider_state::<AccountId, Balance, _>(|state| {
				let charge_fee = state.charge_fee;
				let vault = state.vaults.get_mut(&vault_id).ok_or(VaultError::VaultNotFound)?;
				ensure!(
					vault.available_for_lock() >= securitization.collateral_required,
					VaultError::InsufficientVaultFunds
				);
				let term =
					extension.as_ref().map(|(duration, _)| *duration).unwrap_or(FixedU128::one());
				if let Some((_, lock_extension)) = extension {
					vault.extend_lock(securitization, lock_extension)?;
				} else {
					vault.lock(securitization)?;
				}
				let total_fee = vault
					.terms
					.bitcoin_annual_percent_rate
					.saturating_mul(term)
					.saturating_mul_int(securitization.liquidity_promised)
					.saturating_add(vault.terms.bitcoin_base_fee);
				Ok::<_, VaultError>((total_fee, charge_fee))
			})?;

		if charge_fee {
			Currency::burn_from(
				locker,
				total_fee,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)
			.map_err(|_| VaultError::InsufficientFunds)?;
		}

		Ok(total_fee)
	}

	fn schedule_for_release(
		vault_id: VaultId,
		securitization: &Securitization<Self::Balance>,
		_satoshis: Satoshis,
		lock_extension: &LockExtension<Self::Balance>,
	) -> Result<(), VaultError> {
		mutate_benchmark_bitcoin_vault_provider_state::<AccountId, Balance, _>(|state| {
			let vault = state.vaults.get_mut(&vault_id).ok_or(VaultError::VaultNotFound)?;
			vault.schedule_for_release(securitization, lock_extension)?;
			Ok(())
		})
	}

	fn cancel(
		vault_id: VaultId,
		securitization: &Securitization<Self::Balance>,
	) -> Result<(), VaultError> {
		mutate_benchmark_bitcoin_vault_provider_state::<AccountId, Balance, _>(|state| {
			let vault = state.vaults.get_mut(&vault_id).ok_or(VaultError::VaultNotFound)?;
			vault.release_lock(securitization);
			state.canceled_locks.push((vault_id, securitization.liquidity_promised));
			Ok(())
		})
	}

	fn burn(
		vault_id: VaultId,
		securitization: &Securitization<Self::Balance>,
		market_rate: Self::Balance,
		lock_extension: &LockExtension<Self::Balance>,
	) -> Result<Self::Balance, VaultError> {
		mutate_benchmark_bitcoin_vault_provider_state::<AccountId, Balance, _>(|state| {
			let vault = state.vaults.get_mut(&vault_id).ok_or(VaultError::VaultNotFound)?;
			Ok(vault.burn(securitization, market_rate, lock_extension)?.burned_amount)
		})
	}

	fn compensate_lost_bitcoin(
		vault_id: VaultId,
		_beneficiary: &Self::AccountId,
		securitization: &Securitization<Self::Balance>,
		market_rate: Self::Balance,
		lock_extension: &LockExtension<Self::Balance>,
	) -> Result<Self::Balance, VaultError> {
		Self::burn(vault_id, securitization, market_rate, lock_extension)
	}

	fn create_utxo_script_pubkey(
		vault_id: VaultId,
		_owner_pubkey: CompressedBitcoinPubkey,
		_vault_claim_height: BitcoinHeight,
		_open_claim_height: BitcoinHeight,
		_current_height: BitcoinHeight,
	) -> Result<(BitcoinXPub, BitcoinXPub, BitcoinCosignScriptPubkey), VaultError> {
		let state = benchmark_bitcoin_vault_provider_state::<AccountId, Balance>();
		let (vault_xpub, vault_claim_xpub) = state
			.vault_xpubs_by_id
			.get(&vault_id)
			.cloned()
			.ok_or(VaultError::NoVaultBitcoinPubkeysAvailable)?;
		Ok((
			vault_xpub,
			vault_claim_xpub,
			BitcoinCosignScriptPubkey::P2WSH { wscript_hash: H256::repeat_byte(vault_id as u8) },
		))
	}

	fn remove_pending(
		vault_id: VaultId,
		securitization: &Securitization<Self::Balance>,
	) -> Result<(), VaultError> {
		mutate_benchmark_bitcoin_vault_provider_state::<AccountId, Balance, _>(|state| {
			let vault = state.vaults.get_mut(&vault_id).ok_or(VaultError::VaultNotFound)?;
			vault
				.securitization_pending_activation
				.saturating_reduce(securitization.collateral_required);
			Ok(())
		})
	}

	fn update_pending_cosign_list(
		vault_id: VaultId,
		utxo_id: UtxoId,
		should_remove: bool,
	) -> Result<(), VaultError> {
		mutate_benchmark_bitcoin_vault_provider_state::<AccountId, Balance, _>(|state| {
			let entries = state.pending_cosigns.entry(vault_id).or_default();
			if should_remove {
				entries.remove(&utxo_id);
				if entries.is_empty() {
					state.pending_cosigns.remove(&vault_id);
				}
			} else {
				entries.insert(utxo_id);
			}
			Ok(())
		})
	}

	fn update_orphan_cosign_list(
		vault_id: VaultId,
		_utxo_id: UtxoId,
		account_id: &Self::AccountId,
		should_remove: bool,
	) -> Result<(), VaultError> {
		mutate_benchmark_bitcoin_vault_provider_state::<AccountId, Balance, _>(|state| {
			let vault_entries = state.orphaned_utxo_cosigns.entry(vault_id).or_default();
			let count = vault_entries.entry(account_id.clone()).or_default();
			if should_remove {
				*count = count.saturating_sub(1);
				if *count == 0 {
					vault_entries.remove(account_id);
				}
			} else {
				*count = count.saturating_add(1);
			}
			if vault_entries.is_empty() {
				state.orphaned_utxo_cosigns.remove(&vault_id);
			}
			Ok(())
		})
	}

	fn consume_recent_capacity_drop_budget(
		_vault_id: VaultId,
		_required_collateral: Self::Balance,
	) -> Result<bool, VaultError> {
		Ok(false)
	}
}

impl<Currency, AccountId, Balance> TreasuryVaultProvider
	for BenchmarkBitcoinVaultProvider<Currency, AccountId, Balance>
where
	Currency: Mutate<AccountId, Balance = Balance>,
	AccountId: Codec + Clone + Ord + PartialEq,
	Balance: Codec
		+ Copy
		+ MaxEncodedLen
		+ Default
		+ AtLeast32BitUnsigned
		+ TypeInfo
		+ Clone
		+ core::fmt::Debug
		+ PartialEq
		+ Eq
		+ Sum,
{
	type Balance = Balance;
	type AccountId = AccountId;

	fn get_securitized_satoshis(vault_id: VaultId) -> Satoshis {
		benchmark_bitcoin_vault_provider_state::<AccountId, Balance>()
			.vaults
			.get(&vault_id)
			.map(|vault| vault.securitized_satoshis)
			.unwrap_or_default()
	}

	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId> {
		benchmark_bitcoin_vault_provider_state::<AccountId, Balance>()
			.vaults
			.get(&vault_id)
			.map(|vault| vault.operator_account_id.clone())
	}

	fn get_vault_profit_sharing_percent(vault_id: VaultId) -> Option<Permill> {
		benchmark_bitcoin_vault_provider_state::<AccountId, Balance>()
			.vaults
			.get(&vault_id)
			.map(|vault| vault.terms.treasury_profit_sharing)
	}

	fn is_vault_open(vault_id: VaultId) -> bool {
		benchmark_bitcoin_vault_provider_state::<AccountId, Balance>()
			.vaults
			.get(&vault_id)
			.map(|vault| !vault.is_closed)
			.unwrap_or(false)
	}

	fn record_vault_frame_earnings(
		_source_account_id: &Self::AccountId,
		profit: VaultTreasuryFrameEarnings<Self::Balance, Self::AccountId>,
	) {
		mutate_benchmark_bitcoin_vault_provider_state::<AccountId, Balance, _>(|state| {
			state.treasury_frame_earnings.push((profit.vault_id, profit.earnings_for_vault));
		});
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
pub struct BenchmarkUtxoLockEventsState<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + Copy,
{
	pub last_lock_event: Option<(UtxoId, AccountId, Balance)>,
	pub last_release_event: Option<(UtxoId, bool, Balance)>,
}

impl<AccountId, Balance> Default for BenchmarkUtxoLockEventsState<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + Copy,
{
	fn default() -> Self {
		Self { last_lock_event: None, last_release_event: None }
	}
}

pub fn set_benchmark_utxo_lock_events_state<AccountId, Balance>(
	state: BenchmarkUtxoLockEventsState<AccountId, Balance>,
) where
	AccountId: Codec,
	Balance: Codec + Copy,
{
	utxo_lock_events_state_backend::set(state.encode());
}

pub fn reset_benchmark_utxo_lock_events_state() {
	utxo_lock_events_state_backend::reset();
}

pub fn benchmark_utxo_lock_events_state<AccountId, Balance>()
-> BenchmarkUtxoLockEventsState<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + Copy,
{
	decode_benchmark_state(utxo_lock_events_state_backend::get())
}

pub struct BenchmarkUtxoLockEvents<AccountId, Balance>(PhantomData<(AccountId, Balance)>);
impl<AccountId, Balance> UtxoLockEvents<AccountId, Balance>
	for BenchmarkUtxoLockEvents<AccountId, Balance>
where
	AccountId: Codec + Clone,
	Balance: Codec + Copy,
{
	fn utxo_locked(utxo_id: UtxoId, account_id: &AccountId, amount: Balance) -> DispatchResult {
		let mut state = benchmark_utxo_lock_events_state::<AccountId, Balance>();
		state.last_lock_event = Some((utxo_id, account_id.clone(), amount));
		set_benchmark_utxo_lock_events_state(state);
		Ok(())
	}

	fn utxo_released(
		utxo_id: UtxoId,
		remove_pending_mints: bool,
		burned_argons: Balance,
	) -> DispatchResult {
		let mut state = benchmark_utxo_lock_events_state::<AccountId, Balance>();
		state.last_release_event = Some((utxo_id, remove_pending_mints, burned_argons));
		set_benchmark_utxo_lock_events_state(state);
		Ok(())
	}
}

#[cfg(feature = "std")]
mod price_state_backend {
	use super::*;
	use frame_support::parameter_types;

	parameter_types! {
		pub static BenchmarkPriceProviderStateHolder: Vec<u8> = Vec::new();
	}

	pub(super) fn set(state: Vec<u8>) {
		BenchmarkPriceProviderStateHolder::set(state);
	}

	pub(super) fn reset() {
		BenchmarkPriceProviderStateHolder::reset();
	}

	pub(super) fn get() -> Vec<u8> {
		BenchmarkPriceProviderStateHolder::get()
	}
}

#[cfg(not(feature = "std"))]
mod price_state_backend {
	use super::*;
	use core::cell::UnsafeCell;

	struct BenchmarkStateCell(UnsafeCell<Option<Vec<u8>>>);
	unsafe impl Sync for BenchmarkStateCell {}

	static BENCHMARK_PRICE_PROVIDER_STATE: BenchmarkStateCell =
		BenchmarkStateCell(UnsafeCell::new(None));

	pub(super) fn set(state: Vec<u8>) {
		unsafe {
			*BENCHMARK_PRICE_PROVIDER_STATE.0.get() = Some(state);
		}
	}

	pub(super) fn reset() {
		unsafe {
			*BENCHMARK_PRICE_PROVIDER_STATE.0.get() = None;
		}
	}

	pub(super) fn get() -> Vec<u8> {
		unsafe { (*BENCHMARK_PRICE_PROVIDER_STATE.0.get()).clone().unwrap_or_default() }
	}
}

#[cfg(feature = "std")]
mod bitcoin_utxo_tracker_state_backend {
	use super::*;
	use frame_support::parameter_types;

	parameter_types! {
		pub static BenchmarkBitcoinUtxoTrackerStateHolder: Vec<u8> = Vec::new();
	}

	pub(super) fn set(state: Vec<u8>) {
		BenchmarkBitcoinUtxoTrackerStateHolder::set(state);
	}

	pub(super) fn reset() {
		BenchmarkBitcoinUtxoTrackerStateHolder::reset();
	}

	pub(super) fn get() -> Vec<u8> {
		BenchmarkBitcoinUtxoTrackerStateHolder::get()
	}
}

#[cfg(not(feature = "std"))]
mod bitcoin_utxo_tracker_state_backend {
	use super::*;
	use core::cell::UnsafeCell;

	struct BenchmarkStateCell(UnsafeCell<Option<Vec<u8>>>);
	unsafe impl Sync for BenchmarkStateCell {}

	static BENCHMARK_BITCOIN_UTXO_TRACKER_STATE: BenchmarkStateCell =
		BenchmarkStateCell(UnsafeCell::new(None));

	pub(super) fn set(state: Vec<u8>) {
		unsafe {
			*BENCHMARK_BITCOIN_UTXO_TRACKER_STATE.0.get() = Some(state);
		}
	}

	pub(super) fn reset() {
		unsafe {
			*BENCHMARK_BITCOIN_UTXO_TRACKER_STATE.0.get() = None;
		}
	}

	pub(super) fn get() -> Vec<u8> {
		unsafe { (*BENCHMARK_BITCOIN_UTXO_TRACKER_STATE.0.get()).clone().unwrap_or_default() }
	}
}

#[cfg(feature = "std")]
mod bitcoin_locks_runtime_state_backend {
	use super::*;
	use frame_support::parameter_types;

	parameter_types! {
		pub static BenchmarkBitcoinLocksRuntimeStateHolder: Vec<u8> = Vec::new();
	}

	pub(super) fn set(state: Vec<u8>) {
		BenchmarkBitcoinLocksRuntimeStateHolder::set(state);
	}

	pub(super) fn reset() {
		BenchmarkBitcoinLocksRuntimeStateHolder::reset();
	}

	pub(super) fn get() -> Vec<u8> {
		BenchmarkBitcoinLocksRuntimeStateHolder::get()
	}
}

#[cfg(not(feature = "std"))]
mod bitcoin_locks_runtime_state_backend {
	use super::*;
	use core::cell::UnsafeCell;

	struct BenchmarkStateCell(UnsafeCell<Option<Vec<u8>>>);
	unsafe impl Sync for BenchmarkStateCell {}

	static BENCHMARK_BITCOIN_LOCKS_RUNTIME_STATE: BenchmarkStateCell =
		BenchmarkStateCell(UnsafeCell::new(None));

	pub(super) fn set(state: Vec<u8>) {
		unsafe {
			*BENCHMARK_BITCOIN_LOCKS_RUNTIME_STATE.0.get() = Some(state);
		}
	}

	pub(super) fn reset() {
		unsafe {
			*BENCHMARK_BITCOIN_LOCKS_RUNTIME_STATE.0.get() = None;
		}
	}

	pub(super) fn get() -> Vec<u8> {
		unsafe { (*BENCHMARK_BITCOIN_LOCKS_RUNTIME_STATE.0.get()).clone().unwrap_or_default() }
	}
}

#[cfg(feature = "std")]
mod bitcoin_vault_provider_state_backend {
	use super::*;
	use frame_support::parameter_types;

	parameter_types! {
		pub static BenchmarkBitcoinVaultProviderStateHolder: Vec<u8> = Vec::new();
	}

	pub(super) fn set(state: Vec<u8>) {
		BenchmarkBitcoinVaultProviderStateHolder::set(state);
	}

	pub(super) fn reset() {
		BenchmarkBitcoinVaultProviderStateHolder::reset();
	}

	pub(super) fn get() -> Vec<u8> {
		BenchmarkBitcoinVaultProviderStateHolder::get()
	}
}

#[cfg(not(feature = "std"))]
mod bitcoin_vault_provider_state_backend {
	use super::*;
	use core::cell::UnsafeCell;

	struct BenchmarkStateCell(UnsafeCell<Option<Vec<u8>>>);
	unsafe impl Sync for BenchmarkStateCell {}

	static BENCHMARK_BITCOIN_VAULT_PROVIDER_STATE: BenchmarkStateCell =
		BenchmarkStateCell(UnsafeCell::new(None));

	pub(super) fn set(state: Vec<u8>) {
		unsafe {
			*BENCHMARK_BITCOIN_VAULT_PROVIDER_STATE.0.get() = Some(state);
		}
	}

	pub(super) fn reset() {
		unsafe {
			*BENCHMARK_BITCOIN_VAULT_PROVIDER_STATE.0.get() = None;
		}
	}

	pub(super) fn get() -> Vec<u8> {
		unsafe { (*BENCHMARK_BITCOIN_VAULT_PROVIDER_STATE.0.get()).clone().unwrap_or_default() }
	}
}

#[cfg(feature = "std")]
mod utxo_lock_events_state_backend {
	use super::*;
	use frame_support::parameter_types;

	parameter_types! {
		pub static BenchmarkUtxoLockEventsStateHolder: Vec<u8> = Vec::new();
	}

	pub(super) fn set(state: Vec<u8>) {
		BenchmarkUtxoLockEventsStateHolder::set(state);
	}

	pub(super) fn reset() {
		BenchmarkUtxoLockEventsStateHolder::reset();
	}

	pub(super) fn get() -> Vec<u8> {
		BenchmarkUtxoLockEventsStateHolder::get()
	}
}

#[cfg(not(feature = "std"))]
mod utxo_lock_events_state_backend {
	use super::*;
	use core::cell::UnsafeCell;

	struct BenchmarkStateCell(UnsafeCell<Option<Vec<u8>>>);
	unsafe impl Sync for BenchmarkStateCell {}

	static BENCHMARK_UTXO_LOCK_EVENTS_STATE: BenchmarkStateCell =
		BenchmarkStateCell(UnsafeCell::new(None));

	pub(super) fn set(state: Vec<u8>) {
		unsafe {
			*BENCHMARK_UTXO_LOCK_EVENTS_STATE.0.get() = Some(state);
		}
	}

	pub(super) fn reset() {
		unsafe {
			*BENCHMARK_UTXO_LOCK_EVENTS_STATE.0.get() = None;
		}
	}

	pub(super) fn get() -> Vec<u8> {
		unsafe { (*BENCHMARK_UTXO_LOCK_EVENTS_STATE.0.get()).clone().unwrap_or_default() }
	}
}
