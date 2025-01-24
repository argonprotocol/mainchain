#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
mod benchmark;

pub mod configs;
mod migrations;
pub mod weights;

pub use crate::configs::NotaryRecordT;
pub use pallet_notebook::NotebookVerifyError;

use crate::configs::ArgonToken;

use alloc::{collections::BTreeMap, vec, vec::Vec};
use argon_primitives::{
	bitcoin::{BitcoinNetwork, BitcoinSyncStatus, Satoshis, UtxoRef, UtxoValue},
	block_seal::{BlockPayout, ComputePuzzle, MiningAuthority},
	notary::{
		NotaryNotebookAuditSummary, NotaryNotebookDetails, NotaryNotebookRawVotes,
		NotaryNotebookVoteDigestDetails, NotaryRecordWithState,
	},
	prelude::*,
	tick::Ticker,
	BestBlockVoteSeal, BlockHash, BlockSealAuthorityId, BlockSealDigest, BlockSealSpecProvider,
	BlockVoteDigest, NotebookAuditResult, NotebookNumber, PriceProvider, Signature, TickProvider,
	VoteMinimum, VotingKey,
};
use frame_support::{
	genesis_builder_helper::{build_state, get_preset},
	traits::ConstU32,
	weights::Weight,
};
use ismp::{
	consensus::{ConsensusClientId, StateMachineHeight, StateMachineId},
	host::StateMachine,
	router::{Request, Response},
};
use sp_api::{decl_runtime_apis, impl_runtime_apis};
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::KeyTypeId, Get, OpaqueMetadata, H256, U256};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{Block as BlockT, NumberFor},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, BoundedVec, Digest, DispatchError,
};
use sp_version::RuntimeVersion;

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
#[cfg(feature = "std")]
use sp_version::NativeVersion;

pub type BalancesCall = pallet_balances::Call<Runtime, ArgonToken>;
// A few exports that help ease life for downstream crates.
pub type AccountData = pallet_balances::AccountData<Balance>;

impl_opaque_keys! {
	pub struct SessionKeys {
	pub grandpa: Grandpa,
	pub block_seal_authority: MiningSlot,
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("argon"),
	impl_name: create_runtime_str!("argon"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 106,
	impl_version: 3,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeTask
	)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;
	#[runtime::pallet_index(1)]
	pub type Digests = pallet_digests;
	#[runtime::pallet_index(2)]
	pub type Timestamp = pallet_timestamp;
	#[runtime::pallet_index(3)]
	pub type Multisig = pallet_multisig;
	#[runtime::pallet_index(4)]
	pub type Proxy = pallet_proxy;
	#[runtime::pallet_index(5)]
	pub type Ticks = pallet_ticks;
	#[runtime::pallet_index(6)]
	pub type MiningSlot = pallet_mining_slot;
	#[runtime::pallet_index(7)]
	pub type BitcoinUtxos = pallet_bitcoin_utxos;
	#[runtime::pallet_index(8)]
	pub type Vaults = pallet_vaults;
	#[runtime::pallet_index(9)]
	pub type Bonds = pallet_bond;
	#[runtime::pallet_index(10)]
	pub type Notaries = pallet_notaries;
	#[runtime::pallet_index(11)]
	pub type Notebook = pallet_notebook;
	#[runtime::pallet_index(12)]
	pub type ChainTransfer = pallet_chain_transfer;
	#[runtime::pallet_index(13)]
	pub type BlockSealSpec = pallet_block_seal_spec;
	#[runtime::pallet_index(14)]
	pub type Domains = pallet_domains;
	#[runtime::pallet_index(15)]
	pub type PriceIndex = pallet_price_index;
	#[runtime::pallet_index(16)]
	pub type Authorship = pallet_authorship;
	#[runtime::pallet_index(17)]
	pub type Grandpa = pallet_grandpa;
	// Block Seal uses notebooks and ticks
	#[runtime::pallet_index(18)]
	pub type BlockSeal = pallet_block_seal;
	// NOTE: BlockRewards must come after seal (on_finalize uses seal info)
	#[runtime::pallet_index(19)]
	pub type BlockRewards = pallet_block_rewards;
	#[runtime::pallet_index(20)]
	pub type Mint = pallet_mint;
	#[runtime::pallet_index(21)]
	pub type Balances = pallet_balances<Instance1>;
	#[runtime::pallet_index(22)]
	pub type Ownership = pallet_balances<Instance2>;
	#[runtime::pallet_index(23)]
	pub type TxPause = pallet_tx_pause;
	#[runtime::pallet_index(24)]
	pub type TransactionPayment = pallet_transaction_payment;
	#[runtime::pallet_index(25)]
	pub type Utility = pallet_utility;
	#[runtime::pallet_index(26)]
	pub type Sudo = pallet_sudo;

	#[runtime::pallet_index(27)]
	pub type Ismp = pallet_ismp;
	#[runtime::pallet_index(28)]
	pub type IsmpGrandpa = ismp_grandpa;
	#[runtime::pallet_index(29)]
	pub type Hyperbridge = pallet_hyperbridge;
	#[runtime::pallet_index(30)]
	pub type TokenGateway = pallet_token_gateway;
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlockHash>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckMortality<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);
/// All migrations of the runtime, aside from the ones declared in the pallets.
///
/// This can be a tuple of types, each implementing `OnRuntimeUpgrade`.
type Migrations = (pallet_vaults::migrations::MigrateV0ToV1<Runtime>,);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

decl_runtime_apis! {
	/// Configuration items exposed via rpc so they can be confirmed externally
	pub trait ConfigurationApis {
		fn ismp_coprocessor() -> Option<StateMachine>;
	}
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl argon_primitives::MiningApis<Block, AccountId, BlockSealAuthorityId> for Runtime {
		fn get_authority_id(account_id: &AccountId) -> Option<MiningAuthority< BlockSealAuthorityId, AccountId>> {
			MiningSlot::get_mining_authority(account_id)
		}
		fn get_block_payouts() -> Vec<BlockPayout<AccountId, Balance>> {
			BlockRewards::block_payouts()
		}
	}

	impl argon_primitives::BlockSealApis<Block, AccountId, BlockSealAuthorityId> for Runtime {
		fn vote_minimum() -> VoteMinimum {
			BlockSealSpec::vote_minimum()
		}

		fn compute_puzzle() -> ComputePuzzle<Block> {
			ComputePuzzle {
				difficulty: BlockSealSpec::compute_difficulty(),
				randomx_key_block: BlockSealSpec::compute_key_block_hash(),
			}
		}

		fn create_vote_digest(notebook_tick: Tick, included_notebooks: Vec<NotaryNotebookVoteDigestDetails>) -> BlockVoteDigest {
			BlockSealSpec::create_block_vote_digest(notebook_tick, included_notebooks)
		}

		fn find_vote_block_seals(
			votes: Vec<NotaryNotebookRawVotes>,
			with_better_strength: U256,
			expected_notebook_tick: Tick,
		) -> Result<BoundedVec<BestBlockVoteSeal<AccountId, BlockSealAuthorityId>, ConstU32<2>>, DispatchError>{
			Ok(BlockSeal::find_vote_block_seals(votes,with_better_strength, expected_notebook_tick)?)
		}

		fn has_eligible_votes() -> bool {
			BlockSeal::has_eligible_votes()
		}

		fn is_valid_signature(block_hash: <Block as BlockT>::Hash, seal: &BlockSealDigest, digest: &Digest) -> bool {
			BlockSeal::is_valid_miner_signature(block_hash, seal, digest)
		}

		fn is_bootstrap_mining() -> bool {
			!MiningSlot::is_registered_mining_active()
		}
	}

	impl argon_primitives::BlockCreatorApis<Block, AccountId, NotebookVerifyError> for Runtime {
		fn decode_voting_author(digest: &Digest) -> Result<(AccountId, Tick, Option<VotingKey>), DispatchError> {
			Digests::decode_voting_author(digest)
		}

		fn digest_notebooks(
			digests: &Digest,
		) -> Result<Vec<NotebookAuditResult<NotebookVerifyError>>, DispatchError> {
			let digests = Digests::decode(digests)?;
			Ok(digests.notebooks.notebooks.clone())
		}
	}

	impl argon_primitives::NotaryApis<Block, NotaryRecordT> for Runtime {
		fn notary_by_id(notary_id: NotaryId) -> Option<NotaryRecordT> {
			Self::notaries().iter().find(|a| a.notary_id == notary_id).cloned()
		}
		fn notaries() -> Vec<NotaryRecordT> {
			Notaries::notaries().iter().map(|n| {
				let state = Notebook::get_state(n.notary_id);
				NotaryRecordWithState {
					notary_id: n.notary_id,
					operator_account_id: n.operator_account_id.clone(),
					activated_block: n.activated_block,
					meta_updated_block: n.meta_updated_block,
					meta_updated_tick: n.meta_updated_tick,
					meta: n.meta.clone(),
					state,
				}
			}).collect()
		}
	}

	impl pallet_mining_slot::MiningSlotApi<Block, BlockNumber> for Runtime {
		fn next_slot_era() -> (BlockNumber, BlockNumber) {
			MiningSlot::get_slot_era()
		}
	}

	impl argon_primitives::NotebookApis<Block, NotebookVerifyError> for Runtime {
		fn audit_notebook_and_get_votes(
			version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			notebook_tick: Tick,
			header_hash: H256,
			vote_minimums: &BTreeMap<<Block as BlockT>::Hash, VoteMinimum>,
			bytes: &Vec<u8>,
			audit_dependency_summaries: Vec<NotaryNotebookAuditSummary>,
		) -> Result<NotaryNotebookRawVotes, NotebookVerifyError> {
			Notebook::audit_notebook(version, notary_id, notebook_number, notebook_tick, header_hash, vote_minimums, bytes, audit_dependency_summaries)
		}

		fn decode_signed_raw_notebook_header(raw_header: Vec<u8>) -> Result<NotaryNotebookDetails <<Block as BlockT>::Hash>, DispatchError> {
			Notebook::decode_signed_raw_notebook_header(raw_header)
		}

		fn latest_notebook_by_notary() -> BTreeMap<NotaryId, (NotebookNumber, Tick)> {
			Notebook::latest_notebook_by_notary()
		}
	}

	impl argon_primitives::TickApis<Block> for Runtime {
		fn current_tick() -> Tick {
			Ticks::current_tick()
		}
		fn ticker() -> Ticker {
			Ticks::ticker()
		}
		fn blocks_at_tick(tick: Tick) -> Vec<<Block as BlockT>::Hash> {
			Ticks::blocks_at_tick(tick)
		}
	}

	impl argon_primitives::BitcoinApis<Block,Balance> for Runtime {
		fn get_sync_status() -> Option<BitcoinSyncStatus> {
			BitcoinUtxos::get_sync_status()
		}

		fn active_utxos() -> Vec<(Option<UtxoRef>, UtxoValue)>{
			BitcoinUtxos::active_utxos()
		}

		fn redemption_rate(satoshis: Satoshis) -> Option<Balance> {
			Bonds::get_redemption_price(&satoshis).ok()
		}

		fn market_rate(satoshis: Satoshis) -> Option<Balance> {
			PriceIndex::get_bitcoin_argon_price(satoshis)
		}

		fn get_bitcoin_network() -> BitcoinNetwork {
			<BitcoinUtxos as Get<BitcoinNetwork>>::get()
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			_authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			None
		}
	}


	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch};
			use frame_support::traits::TrackedStorageKey;

			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, configs::BlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			vec![]
		}
	}

	impl pallet_ismp_runtime_api::IsmpRuntimeApi<Block, <Block as BlockT>::Hash> for Runtime {
		fn host_state_machine() -> StateMachine {
			<Runtime as pallet_ismp::Config>::HostStateMachine::get()
		}

		fn challenge_period(state_machine_id: StateMachineId) -> Option<u64> {
			Ismp::challenge_period(state_machine_id)
		}

		/// Fetch all ISMP events in the block, should only be called from runtime-api.
		fn block_events() -> Vec<::ismp::events::Event> {
			Ismp::block_events()
		}

		/// Fetch all ISMP events and their extrinsic metadata, should only be called from runtime-api.
		fn block_events_with_metadata() -> Vec<(::ismp::events::Event, Option<u32>)> {
			Ismp::block_events_with_metadata()
		}

		/// Return the scale encoded consensus state
		fn consensus_state(id: ConsensusClientId) -> Option<Vec<u8>> {
			Ismp::consensus_states(id)
		}

		/// Return the timestamp this client was last updated in seconds
		fn state_machine_update_time(height: StateMachineHeight) -> Option<u64> {
			Ismp::state_machine_update_time(height)
		}

		/// Return the latest height of the state machine
		fn latest_state_machine_height(id: StateMachineId) -> Option<u64> {
			Ismp::latest_state_machine_height(id)
		}


		/// Get actual requests
		fn requests(commitments: Vec<H256>) -> Vec<Request> {
			Ismp::requests(commitments)
		}

		/// Get actual requests
		fn responses(commitments: Vec<H256>) -> Vec<Response> {
			Ismp::responses(commitments)
		}
	}

	impl crate::ConfigurationApis<Block> for Runtime {
		fn ismp_coprocessor() -> Option<StateMachine> {
			<Runtime as pallet_ismp::Config>::Coprocessor::get()
		}
	}
}
