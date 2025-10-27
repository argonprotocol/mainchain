#![allow(clippy::crate_in_macro_def)]

#[macro_export]
macro_rules! inject_common_apis {
    () => {
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
        impl frame_support::view_functions::runtime_api::RuntimeViewFunction<Block> for Runtime {
            fn execute_view_function(id: frame_support::view_functions::ViewFunctionId, input: Vec<u8>) -> Result<Vec<u8>, frame_support::view_functions::ViewFunctionDispatchError> {
                Runtime::execute_view_function(id, input)
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

            fn find_better_vote_block_seal(
                notebook_votes: Vec<NotaryNotebookRawVotes>,
                best_strength: U256,
                closest_xor_distance: U256,
                with_signing_key: BlockSealAuthorityId,
                expected_notebook_tick: Tick,
            ) -> Result<Option<BestBlockVoteSeal<AccountId, BlockSealAuthorityId>>, DispatchError> {
                Ok(BlockSeal::find_better_vote_block_seal(
                    notebook_votes,
                    best_strength,
                    closest_xor_distance,
                    with_signing_key,
                    expected_notebook_tick,
                )?)
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
                Ok(digests.notebooks.notebooks.to_vec())
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

        impl pallet_mining_slot::MiningSlotApi<Block, Balance> for Runtime {
            fn next_slot_era() -> (Tick, Tick) {
                MiningSlot::get_next_slot_era()
            }
            fn bid_pool() -> Balance {
                MiningSlot::bid_pool_balance()
            }
        }

        impl argon_primitives::NotebookApis<Block, NotebookVerifyError> for Runtime {
            fn audit_notebook_and_get_votes_v2(
                version: u32,
                notary_id: NotaryId,
                notebook_number: NotebookNumber,
                notebook_tick: Tick,
                header_hash: H256,
                bytes: &Vec<u8>,
                audit_dependency_summaries: Vec<NotaryNotebookAuditSummary>,
            ) -> Result<NotaryNotebookRawVotes, NotebookVerifyError> {
                Notebook::audit_notebook(version, notary_id, notebook_number, notebook_tick, header_hash, bytes, audit_dependency_summaries)
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
            fn tick_for_frame(frame_id: FrameId) -> Tick {
                MiningSlot::tick_for_frame(frame_id)
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
                BitcoinLocks::get_redemption_price(&satoshis, None).ok()
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

		    #[allow(non_local_definitions)]
            fn dispatch_benchmark(
                config: frame_benchmarking::BenchmarkConfig
            ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
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
                (weight, BlockWeights::get().max_block)
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
            fn host_state_machine() -> ismp::host::StateMachine {
                <Runtime as pallet_ismp::Config>::HostStateMachine::get()
            }

            fn challenge_period(state_machine_id: ismp::consensus::StateMachineId) -> Option<u64> {
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
            fn consensus_state(id: ismp::consensus::ConsensusClientId) -> Option<Vec<u8>> {
                Ismp::consensus_states(id)
            }

            /// Return the timestamp this client was last updated in seconds
            fn state_machine_update_time(height: ismp::consensus::StateMachineHeight) -> Option<u64> {
                Ismp::state_machine_update_time(height)
            }

            /// Return the latest height of the state machine
            fn latest_state_machine_height(id: ismp::consensus::StateMachineId) -> Option<u64> {
                Ismp::latest_state_machine_height(id)
            }


            /// Get actual requests
            fn requests(commitments: Vec<H256>) -> Vec<ismp::router::Request> {
                Ismp::requests(commitments)
            }

            /// Get actual requests
            fn responses(commitments: Vec<H256>) -> Vec<ismp::router::Response> {
                Ismp::responses(commitments)
            }
        }

        impl crate::ConfigurationApis<Block> for Runtime {
            fn ismp_coprocessor() -> Option<StateMachine> {
                <Runtime as pallet_ismp::Config>::Coprocessor::get()
            }
        }
    }
}
}
