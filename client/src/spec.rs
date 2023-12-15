#[allow(dead_code, unused_imports, non_camel_case_types)]
#[allow(clippy::all)]
#[allow(rustdoc::broken_intra_doc_links)]
pub mod api {
	#[allow(unused_imports)]
	mod root_mod {
		pub use super::*;
	}
	pub static PALLETS: [&str; 21usize] = [
		"System",
		"Timestamp",
		"Ticks",
		"MiningSlot",
		"Bond",
		"Notaries",
		"Notebook",
		"ChainTransfer",
		"BlockSealSpec",
		"Authorship",
		"Historical",
		"Session",
		"BlockSeal",
		"BlockRewards",
		"Grandpa",
		"Offences",
		"ArgonBalances",
		"UlixeeBalances",
		"TxPause",
		"TransactionPayment",
		"Sudo",
	];
	pub static RUNTIME_APIS: [&str; 17usize] = [
		"Core",
		"Metadata",
		"BlockBuilder",
		"TaggedTransactionQueue",
		"OffchainWorkerApi",
		"AccountNonceApi",
		"SessionKeys",
		"TransactionPaymentApi",
		"TransactionPaymentCallApi",
		"BlockSealSpecApis",
		"NotaryApis",
		"MiningAuthorityApis",
		"MiningSlotApi",
		"NotebookApis",
		"TickApis",
		"GrandpaApi",
		"GenesisBuilder",
	];
	#[doc = r" The error type returned when there is a runtime issue."]
	pub type DispatchError = runtime_types::sp_runtime::DispatchError;
	#[doc = r" The outer event enum."]
	pub type Event = runtime_types::ulx_node_runtime::RuntimeEvent;
	#[doc = r" The outer extrinsic enum."]
	pub type Call = runtime_types::ulx_node_runtime::RuntimeCall;
	#[doc = r" The outer error enum representing the DispatchError's Module variant."]
	pub type Error = runtime_types::ulx_node_runtime::RuntimeError;
	pub fn constants() -> ConstantsApi {
		ConstantsApi
	}
	pub fn storage() -> StorageApi {
		StorageApi
	}
	pub fn tx() -> TransactionApi {
		TransactionApi
	}
	pub fn apis() -> runtime_apis::RuntimeApi {
		runtime_apis::RuntimeApi
	}
	pub mod runtime_apis {
		use super::{root_mod, runtime_types};
		use subxt::ext::codec::Encode;
		pub struct RuntimeApi;
		impl RuntimeApi {
			pub fn core(&self) -> core::Core {
				core::Core
			}
			pub fn metadata(&self) -> metadata::Metadata {
				metadata::Metadata
			}
			pub fn block_builder(&self) -> block_builder::BlockBuilder {
				block_builder::BlockBuilder
			}
			pub fn tagged_transaction_queue(
				&self,
			) -> tagged_transaction_queue::TaggedTransactionQueue {
				tagged_transaction_queue::TaggedTransactionQueue
			}
			pub fn offchain_worker_api(&self) -> offchain_worker_api::OffchainWorkerApi {
				offchain_worker_api::OffchainWorkerApi
			}
			pub fn account_nonce_api(&self) -> account_nonce_api::AccountNonceApi {
				account_nonce_api::AccountNonceApi
			}
			pub fn session_keys(&self) -> session_keys::SessionKeys {
				session_keys::SessionKeys
			}
			pub fn transaction_payment_api(
				&self,
			) -> transaction_payment_api::TransactionPaymentApi {
				transaction_payment_api::TransactionPaymentApi
			}
			pub fn transaction_payment_call_api(
				&self,
			) -> transaction_payment_call_api::TransactionPaymentCallApi {
				transaction_payment_call_api::TransactionPaymentCallApi
			}
			pub fn block_seal_spec_apis(&self) -> block_seal_spec_apis::BlockSealSpecApis {
				block_seal_spec_apis::BlockSealSpecApis
			}
			pub fn notary_apis(&self) -> notary_apis::NotaryApis {
				notary_apis::NotaryApis
			}
			pub fn mining_authority_apis(&self) -> mining_authority_apis::MiningAuthorityApis {
				mining_authority_apis::MiningAuthorityApis
			}
			pub fn mining_slot_api(&self) -> mining_slot_api::MiningSlotApi {
				mining_slot_api::MiningSlotApi
			}
			pub fn notebook_apis(&self) -> notebook_apis::NotebookApis {
				notebook_apis::NotebookApis
			}
			pub fn tick_apis(&self) -> tick_apis::TickApis {
				tick_apis::TickApis
			}
			pub fn grandpa_api(&self) -> grandpa_api::GrandpaApi {
				grandpa_api::GrandpaApi
			}
			pub fn genesis_builder(&self) -> genesis_builder::GenesisBuilder {
				genesis_builder::GenesisBuilder
			}
		}
		pub mod core {
			use super::{root_mod, runtime_types};
			#[doc = " The `Core` runtime api that every Substrate runtime needs to implement."]
			pub struct Core;
			impl Core {
				#[doc = " Returns the version of the runtime."]
				pub fn version(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::Version,
					runtime_types::sp_version::RuntimeVersion,
				> {
					::subxt::runtime_api::Payload::new_static(
						"Core",
						"version",
						types::Version {},
						[
							76u8, 202u8, 17u8, 117u8, 189u8, 237u8, 239u8, 237u8, 151u8, 17u8,
							125u8, 159u8, 218u8, 92u8, 57u8, 238u8, 64u8, 147u8, 40u8, 72u8, 157u8,
							116u8, 37u8, 195u8, 156u8, 27u8, 123u8, 173u8, 178u8, 102u8, 136u8,
							6u8,
						],
					)
				}
				#[doc = " Execute the given block."]
				pub fn execute_block(
					&self,
					block : runtime_types :: sp_runtime :: generic :: block :: Block < runtime_types :: sp_runtime :: generic :: header :: Header < :: core :: primitive :: u32 > , :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > >,
				) -> ::subxt::runtime_api::Payload<types::ExecuteBlock, ()> {
					::subxt::runtime_api::Payload::new_static(
						"Core",
						"execute_block",
						types::ExecuteBlock { block },
						[
							133u8, 135u8, 228u8, 65u8, 106u8, 27u8, 85u8, 158u8, 112u8, 254u8,
							93u8, 26u8, 102u8, 201u8, 118u8, 216u8, 249u8, 247u8, 91u8, 74u8, 56u8,
							208u8, 231u8, 115u8, 131u8, 29u8, 209u8, 6u8, 65u8, 57u8, 214u8, 125u8,
						],
					)
				}
				#[doc = " Initialize a block with the given header."]
				pub fn initialize_block(
					&self,
					header: runtime_types::sp_runtime::generic::header::Header<
						::core::primitive::u32,
					>,
				) -> ::subxt::runtime_api::Payload<types::InitializeBlock, ()> {
					::subxt::runtime_api::Payload::new_static(
						"Core",
						"initialize_block",
						types::InitializeBlock { header },
						[
							146u8, 138u8, 72u8, 240u8, 63u8, 96u8, 110u8, 189u8, 77u8, 92u8, 96u8,
							232u8, 41u8, 217u8, 105u8, 148u8, 83u8, 190u8, 152u8, 219u8, 19u8,
							87u8, 163u8, 1u8, 232u8, 25u8, 221u8, 74u8, 224u8, 67u8, 223u8, 34u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Version {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ExecuteBlock { pub block : runtime_types :: sp_runtime :: generic :: block :: Block < runtime_types :: sp_runtime :: generic :: header :: Header < :: core :: primitive :: u32 > , :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > > , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct InitializeBlock {
					pub header:
						runtime_types::sp_runtime::generic::header::Header<::core::primitive::u32>,
				}
			}
		}
		pub mod metadata {
			use super::{root_mod, runtime_types};
			#[doc = " The `Metadata` api trait that returns metadata for the runtime."]
			pub struct Metadata;
			impl Metadata {
				#[doc = " Returns the metadata of a runtime."]
				pub fn metadata(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::Metadata,
					runtime_types::sp_core::OpaqueMetadata,
				> {
					::subxt::runtime_api::Payload::new_static(
						"Metadata",
						"metadata",
						types::Metadata {},
						[
							231u8, 24u8, 67u8, 152u8, 23u8, 26u8, 188u8, 82u8, 229u8, 6u8, 185u8,
							27u8, 175u8, 68u8, 83u8, 122u8, 69u8, 89u8, 185u8, 74u8, 248u8, 87u8,
							217u8, 124u8, 193u8, 252u8, 199u8, 186u8, 196u8, 179u8, 179u8, 96u8,
						],
					)
				}
				#[doc = " Returns the metadata at a given version."]
				#[doc = ""]
				#[doc = " If the given `version` isn't supported, this will return `None`."]
				#[doc = " Use [`Self::metadata_versions`] to find out about supported metadata version of the runtime."]
				pub fn metadata_at_version(
					&self,
					version: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<
					types::MetadataAtVersion,
					::core::option::Option<runtime_types::sp_core::OpaqueMetadata>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"Metadata",
						"metadata_at_version",
						types::MetadataAtVersion { version },
						[
							131u8, 53u8, 212u8, 234u8, 16u8, 25u8, 120u8, 252u8, 153u8, 153u8,
							216u8, 28u8, 54u8, 113u8, 52u8, 236u8, 146u8, 68u8, 142u8, 8u8, 10u8,
							169u8, 131u8, 142u8, 204u8, 38u8, 48u8, 108u8, 134u8, 86u8, 226u8,
							61u8,
						],
					)
				}
				#[doc = " Returns the supported metadata versions."]
				#[doc = ""]
				#[doc = " This can be used to call `metadata_at_version`."]
				pub fn metadata_versions(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::MetadataVersions,
					::std::vec::Vec<::core::primitive::u32>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"Metadata",
						"metadata_versions",
						types::MetadataVersions {},
						[
							23u8, 144u8, 137u8, 91u8, 188u8, 39u8, 231u8, 208u8, 252u8, 218u8,
							224u8, 176u8, 77u8, 32u8, 130u8, 212u8, 223u8, 76u8, 100u8, 190u8,
							82u8, 94u8, 190u8, 8u8, 82u8, 244u8, 225u8, 179u8, 85u8, 176u8, 56u8,
							16u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Metadata {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct MetadataAtVersion {
					pub version: ::core::primitive::u32,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct MetadataVersions {}
			}
		}
		pub mod block_builder {
			use super::{root_mod, runtime_types};
			#[doc = " The `BlockBuilder` api trait that provides the required functionality for building a block."]
			pub struct BlockBuilder;
			impl BlockBuilder {
				#[doc = " Apply the given extrinsic."]
				#[doc = ""]
				#[doc = " Returns an inclusion outcome which specifies if this extrinsic is included in"]
				#[doc = " this block or not."]
				pub fn apply_extrinsic(
					&self,
					extrinsic : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) >,
				) -> ::subxt::runtime_api::Payload<
					types::ApplyExtrinsic,
					::core::result::Result<
						::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
						runtime_types::sp_runtime::transaction_validity::TransactionValidityError,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockBuilder",
						"apply_extrinsic",
						types::ApplyExtrinsic { extrinsic },
						[
							72u8, 54u8, 139u8, 3u8, 118u8, 136u8, 65u8, 47u8, 6u8, 105u8, 125u8,
							223u8, 160u8, 29u8, 103u8, 74u8, 79u8, 149u8, 48u8, 90u8, 237u8, 2u8,
							97u8, 201u8, 123u8, 34u8, 167u8, 37u8, 187u8, 35u8, 176u8, 97u8,
						],
					)
				}
				#[doc = " Finish the current block."]
				pub fn finalize_block(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::FinalizeBlock,
					runtime_types::sp_runtime::generic::header::Header<::core::primitive::u32>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockBuilder",
						"finalize_block",
						types::FinalizeBlock {},
						[
							244u8, 207u8, 24u8, 33u8, 13u8, 69u8, 9u8, 249u8, 145u8, 143u8, 122u8,
							96u8, 197u8, 55u8, 64u8, 111u8, 238u8, 224u8, 34u8, 201u8, 27u8, 146u8,
							232u8, 99u8, 191u8, 30u8, 114u8, 16u8, 32u8, 220u8, 58u8, 62u8,
						],
					)
				}
				#[doc = " Generate inherent extrinsics. The inherent data will vary from chain to chain."]				pub fn inherent_extrinsics (& self , inherent : runtime_types :: sp_inherents :: InherentData ,) -> :: subxt :: runtime_api :: Payload < types :: InherentExtrinsics , :: std :: vec :: Vec < :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > > >{
					::subxt::runtime_api::Payload::new_static(
						"BlockBuilder",
						"inherent_extrinsics",
						types::InherentExtrinsics { inherent },
						[
							254u8, 110u8, 245u8, 201u8, 250u8, 192u8, 27u8, 228u8, 151u8, 213u8,
							166u8, 89u8, 94u8, 81u8, 189u8, 234u8, 64u8, 18u8, 245u8, 80u8, 29u8,
							18u8, 140u8, 129u8, 113u8, 236u8, 135u8, 55u8, 79u8, 159u8, 175u8,
							183u8,
						],
					)
				}
				#[doc = " Check that the inherents are valid. The inherent data will vary from chain to chain."]
				pub fn check_inherents(
					&self,
					block : runtime_types :: sp_runtime :: generic :: block :: Block < runtime_types :: sp_runtime :: generic :: header :: Header < :: core :: primitive :: u32 > , :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > >,
					data: runtime_types::sp_inherents::InherentData,
				) -> ::subxt::runtime_api::Payload<
					types::CheckInherents,
					runtime_types::sp_inherents::CheckInherentsResult,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockBuilder",
						"check_inherents",
						types::CheckInherents { block, data },
						[
							153u8, 134u8, 1u8, 215u8, 139u8, 11u8, 53u8, 51u8, 210u8, 175u8, 197u8,
							28u8, 38u8, 209u8, 175u8, 247u8, 142u8, 157u8, 50u8, 151u8, 164u8,
							191u8, 181u8, 118u8, 80u8, 97u8, 160u8, 248u8, 110u8, 217u8, 181u8,
							234u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ApplyExtrinsic { pub extrinsic : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct FinalizeBlock {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct InherentExtrinsics {
					pub inherent: runtime_types::sp_inherents::InherentData,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CheckInherents { pub block : runtime_types :: sp_runtime :: generic :: block :: Block < runtime_types :: sp_runtime :: generic :: header :: Header < :: core :: primitive :: u32 > , :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > > , pub data : runtime_types :: sp_inherents :: InherentData , }
			}
		}
		pub mod tagged_transaction_queue {
			use super::{root_mod, runtime_types};
			#[doc = " The `TaggedTransactionQueue` api trait for interfering with the transaction queue."]
			pub struct TaggedTransactionQueue;
			impl TaggedTransactionQueue {
				#[doc = " Validate the transaction."]
				#[doc = ""]
				#[doc = " This method is invoked by the transaction pool to learn details about given transaction."]
				#[doc = " The implementation should make sure to verify the correctness of the transaction"]
				#[doc = " against current state. The given `block_hash` corresponds to the hash of the block"]
				#[doc = " that is used as current state."]
				#[doc = ""]
				#[doc = " Note that this call may be performed by the pool multiple times and transactions"]
				#[doc = " might be verified in any possible order."]
				pub fn validate_transaction(
					&self,
					source: runtime_types::sp_runtime::transaction_validity::TransactionSource,
					tx : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) >,
					block_hash: ::subxt::utils::H256,
				) -> ::subxt::runtime_api::Payload<
					types::ValidateTransaction,
					::core::result::Result<
						runtime_types::sp_runtime::transaction_validity::ValidTransaction,
						runtime_types::sp_runtime::transaction_validity::TransactionValidityError,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TaggedTransactionQueue",
						"validate_transaction",
						types::ValidateTransaction { source, tx, block_hash },
						[
							196u8, 50u8, 90u8, 49u8, 109u8, 251u8, 200u8, 35u8, 23u8, 150u8, 140u8,
							143u8, 232u8, 164u8, 133u8, 89u8, 32u8, 240u8, 115u8, 39u8, 95u8, 70u8,
							162u8, 76u8, 122u8, 73u8, 151u8, 144u8, 234u8, 120u8, 100u8, 29u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ValidateTransaction { pub source : runtime_types :: sp_runtime :: transaction_validity :: TransactionSource , pub tx : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > , pub block_hash : :: subxt :: utils :: H256 , }
			}
		}
		pub mod offchain_worker_api {
			use super::{root_mod, runtime_types};
			#[doc = " The offchain worker api."]
			pub struct OffchainWorkerApi;
			impl OffchainWorkerApi {
				#[doc = " Starts the off-chain task for given block header."]
				pub fn offchain_worker(
					&self,
					header: runtime_types::sp_runtime::generic::header::Header<
						::core::primitive::u32,
					>,
				) -> ::subxt::runtime_api::Payload<types::OffchainWorker, ()> {
					::subxt::runtime_api::Payload::new_static(
						"OffchainWorkerApi",
						"offchain_worker",
						types::OffchainWorker { header },
						[
							10u8, 135u8, 19u8, 153u8, 33u8, 216u8, 18u8, 242u8, 33u8, 140u8, 4u8,
							223u8, 200u8, 130u8, 103u8, 118u8, 137u8, 24u8, 19u8, 127u8, 161u8,
							29u8, 184u8, 111u8, 222u8, 111u8, 253u8, 73u8, 45u8, 31u8, 79u8, 60u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct OffchainWorker {
					pub header:
						runtime_types::sp_runtime::generic::header::Header<::core::primitive::u32>,
				}
			}
		}
		pub mod account_nonce_api {
			use super::{root_mod, runtime_types};
			#[doc = " The API to query account nonce."]
			pub struct AccountNonceApi;
			impl AccountNonceApi {
				#[doc = " Get current account nonce of given `AccountId`."]
				pub fn account_nonce(
					&self,
					account: ::subxt::utils::AccountId32,
				) -> ::subxt::runtime_api::Payload<types::AccountNonce, ::core::primitive::u32> {
					::subxt::runtime_api::Payload::new_static(
						"AccountNonceApi",
						"account_nonce",
						types::AccountNonce { account },
						[
							231u8, 82u8, 7u8, 227u8, 131u8, 2u8, 215u8, 252u8, 173u8, 82u8, 11u8,
							103u8, 200u8, 25u8, 114u8, 116u8, 79u8, 229u8, 152u8, 150u8, 236u8,
							37u8, 101u8, 26u8, 220u8, 146u8, 182u8, 101u8, 73u8, 55u8, 191u8,
							171u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AccountNonce {
					pub account: ::subxt::utils::AccountId32,
				}
			}
		}
		pub mod session_keys {
			use super::{root_mod, runtime_types};
			#[doc = " Session keys runtime api."]
			pub struct SessionKeys;
			impl SessionKeys {
				#[doc = " Generate a set of session keys with optionally using the given seed."]
				#[doc = " The keys should be stored within the keystore exposed via runtime"]
				#[doc = " externalities."]
				#[doc = ""]
				#[doc = " The seed needs to be a valid `utf8` string."]
				#[doc = ""]
				#[doc = " Returns the concatenated SCALE encoded public keys."]
				pub fn generate_session_keys(
					&self,
					seed: ::core::option::Option<::std::vec::Vec<::core::primitive::u8>>,
				) -> ::subxt::runtime_api::Payload<
					types::GenerateSessionKeys,
					::std::vec::Vec<::core::primitive::u8>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"SessionKeys",
						"generate_session_keys",
						types::GenerateSessionKeys { seed },
						[
							96u8, 171u8, 164u8, 166u8, 175u8, 102u8, 101u8, 47u8, 133u8, 95u8,
							102u8, 202u8, 83u8, 26u8, 238u8, 47u8, 126u8, 132u8, 22u8, 11u8, 33u8,
							190u8, 175u8, 94u8, 58u8, 245u8, 46u8, 80u8, 195u8, 184u8, 107u8, 65u8,
						],
					)
				}
				#[doc = " Decode the given public session keys."]
				#[doc = ""]
				#[doc = " Returns the list of public raw public keys + key type."]
				pub fn decode_session_keys(
					&self,
					encoded: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::runtime_api::Payload<
					types::DecodeSessionKeys,
					::core::option::Option<
						::std::vec::Vec<(
							::std::vec::Vec<::core::primitive::u8>,
							runtime_types::sp_core::crypto::KeyTypeId,
						)>,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"SessionKeys",
						"decode_session_keys",
						types::DecodeSessionKeys { encoded },
						[
							57u8, 242u8, 18u8, 51u8, 132u8, 110u8, 238u8, 255u8, 39u8, 194u8, 8u8,
							54u8, 198u8, 178u8, 75u8, 151u8, 148u8, 176u8, 144u8, 197u8, 87u8,
							29u8, 179u8, 235u8, 176u8, 78u8, 252u8, 103u8, 72u8, 203u8, 151u8,
							248u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct GenerateSessionKeys {
					pub seed: ::core::option::Option<::std::vec::Vec<::core::primitive::u8>>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DecodeSessionKeys {
					pub encoded: ::std::vec::Vec<::core::primitive::u8>,
				}
			}
		}
		pub mod transaction_payment_api {
			use super::{root_mod, runtime_types};
			pub struct TransactionPaymentApi;
			impl TransactionPaymentApi {
				pub fn query_info(
					&self,
					uxt : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) >,
					len: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<
					types::QueryInfo,
					runtime_types::pallet_transaction_payment::types::RuntimeDispatchInfo<
						::core::primitive::u128,
						runtime_types::sp_weights::weight_v2::Weight,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentApi",
						"query_info",
						types::QueryInfo { uxt, len },
						[
							56u8, 30u8, 174u8, 34u8, 202u8, 24u8, 177u8, 189u8, 145u8, 36u8, 1u8,
							156u8, 98u8, 209u8, 178u8, 49u8, 198u8, 23u8, 150u8, 173u8, 35u8,
							205u8, 147u8, 129u8, 42u8, 22u8, 69u8, 3u8, 129u8, 8u8, 196u8, 139u8,
						],
					)
				}
				pub fn query_fee_details(
					&self,
					uxt : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) >,
					len: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<
					types::QueryFeeDetails,
					runtime_types::pallet_transaction_payment::types::FeeDetails<
						::core::primitive::u128,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentApi",
						"query_fee_details",
						types::QueryFeeDetails { uxt, len },
						[
							117u8, 60u8, 137u8, 159u8, 237u8, 252u8, 216u8, 238u8, 232u8, 1u8,
							100u8, 152u8, 26u8, 185u8, 145u8, 125u8, 68u8, 189u8, 4u8, 30u8, 125u8,
							7u8, 196u8, 153u8, 235u8, 51u8, 219u8, 108u8, 185u8, 254u8, 100u8,
							201u8,
						],
					)
				}
				pub fn query_weight_to_fee(
					&self,
					weight: runtime_types::sp_weights::weight_v2::Weight,
				) -> ::subxt::runtime_api::Payload<types::QueryWeightToFee, ::core::primitive::u128>
				{
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentApi",
						"query_weight_to_fee",
						types::QueryWeightToFee { weight },
						[
							206u8, 243u8, 189u8, 83u8, 231u8, 244u8, 247u8, 52u8, 126u8, 208u8,
							224u8, 5u8, 163u8, 108u8, 254u8, 114u8, 214u8, 156u8, 227u8, 217u8,
							211u8, 198u8, 121u8, 164u8, 110u8, 54u8, 181u8, 146u8, 50u8, 146u8,
							146u8, 23u8,
						],
					)
				}
				pub fn query_length_to_fee(
					&self,
					length: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<types::QueryLengthToFee, ::core::primitive::u128>
				{
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentApi",
						"query_length_to_fee",
						types::QueryLengthToFee { length },
						[
							92u8, 132u8, 29u8, 119u8, 66u8, 11u8, 196u8, 224u8, 129u8, 23u8, 249u8,
							12u8, 32u8, 28u8, 92u8, 50u8, 188u8, 101u8, 203u8, 229u8, 248u8, 216u8,
							130u8, 150u8, 212u8, 161u8, 81u8, 254u8, 116u8, 89u8, 162u8, 48u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryInfo { pub uxt : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > , pub len : :: core :: primitive :: u32 , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryFeeDetails { pub uxt : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > , pub len : :: core :: primitive :: u32 , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryWeightToFee {
					pub weight: runtime_types::sp_weights::weight_v2::Weight,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryLengthToFee {
					pub length: ::core::primitive::u32,
				}
			}
		}
		pub mod transaction_payment_call_api {
			use super::{root_mod, runtime_types};
			pub struct TransactionPaymentCallApi;
			impl TransactionPaymentCallApi {
				#[doc = " Query information of a dispatch class, weight, and fee of a given encoded `Call`."]
				pub fn query_call_info(
					&self,
					call: runtime_types::ulx_node_runtime::RuntimeCall,
					len: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<
					types::QueryCallInfo,
					runtime_types::pallet_transaction_payment::types::RuntimeDispatchInfo<
						::core::primitive::u128,
						runtime_types::sp_weights::weight_v2::Weight,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentCallApi",
						"query_call_info",
						types::QueryCallInfo { call, len },
						[
							72u8, 250u8, 7u8, 237u8, 73u8, 150u8, 110u8, 134u8, 5u8, 106u8, 165u8,
							135u8, 16u8, 214u8, 96u8, 190u8, 48u8, 44u8, 146u8, 227u8, 2u8, 46u8,
							185u8, 86u8, 80u8, 229u8, 226u8, 70u8, 174u8, 180u8, 250u8, 214u8,
						],
					)
				}
				#[doc = " Query fee details of a given encoded `Call`."]
				pub fn query_call_fee_details(
					&self,
					call: runtime_types::ulx_node_runtime::RuntimeCall,
					len: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<
					types::QueryCallFeeDetails,
					runtime_types::pallet_transaction_payment::types::FeeDetails<
						::core::primitive::u128,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentCallApi",
						"query_call_fee_details",
						types::QueryCallFeeDetails { call, len },
						[
							239u8, 225u8, 25u8, 118u8, 83u8, 169u8, 255u8, 94u8, 231u8, 219u8,
							149u8, 22u8, 135u8, 199u8, 253u8, 229u8, 185u8, 231u8, 74u8, 168u8,
							35u8, 120u8, 136u8, 144u8, 127u8, 178u8, 229u8, 73u8, 212u8, 113u8,
							184u8, 149u8,
						],
					)
				}
				#[doc = " Query the output of the current `WeightToFee` given some input."]
				pub fn query_weight_to_fee(
					&self,
					weight: runtime_types::sp_weights::weight_v2::Weight,
				) -> ::subxt::runtime_api::Payload<types::QueryWeightToFee, ::core::primitive::u128>
				{
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentCallApi",
						"query_weight_to_fee",
						types::QueryWeightToFee { weight },
						[
							117u8, 91u8, 94u8, 22u8, 248u8, 212u8, 15u8, 23u8, 97u8, 116u8, 64u8,
							228u8, 83u8, 123u8, 87u8, 77u8, 97u8, 7u8, 98u8, 181u8, 6u8, 165u8,
							114u8, 141u8, 164u8, 113u8, 126u8, 88u8, 174u8, 171u8, 224u8, 35u8,
						],
					)
				}
				#[doc = " Query the output of the current `LengthToFee` given some input."]
				pub fn query_length_to_fee(
					&self,
					length: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<types::QueryLengthToFee, ::core::primitive::u128>
				{
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentCallApi",
						"query_length_to_fee",
						types::QueryLengthToFee { length },
						[
							246u8, 40u8, 4u8, 160u8, 152u8, 94u8, 170u8, 53u8, 205u8, 122u8, 5u8,
							69u8, 70u8, 25u8, 128u8, 156u8, 119u8, 134u8, 116u8, 147u8, 14u8,
							164u8, 65u8, 140u8, 86u8, 13u8, 250u8, 218u8, 89u8, 95u8, 234u8, 228u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryCallInfo {
					pub call: runtime_types::ulx_node_runtime::RuntimeCall,
					pub len: ::core::primitive::u32,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryCallFeeDetails {
					pub call: runtime_types::ulx_node_runtime::RuntimeCall,
					pub len: ::core::primitive::u32,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryWeightToFee {
					pub weight: runtime_types::sp_weights::weight_v2::Weight,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryLengthToFee {
					pub length: ::core::primitive::u32,
				}
			}
		}
		pub mod block_seal_spec_apis {
			use super::{root_mod, runtime_types};
			pub struct BlockSealSpecApis;
			impl BlockSealSpecApis {
				pub fn vote_minimum(
					&self,
				) -> ::subxt::runtime_api::Payload<types::VoteMinimum, ::core::primitive::u128> {
					::subxt::runtime_api::Payload::new_static(
						"BlockSealSpecApis",
						"vote_minimum",
						types::VoteMinimum {},
						[
							171u8, 251u8, 91u8, 35u8, 28u8, 225u8, 151u8, 169u8, 48u8, 158u8, 67u8,
							122u8, 183u8, 104u8, 49u8, 70u8, 125u8, 100u8, 126u8, 25u8, 51u8,
							185u8, 74u8, 159u8, 180u8, 76u8, 255u8, 112u8, 250u8, 243u8, 228u8,
							127u8,
						],
					)
				}
				pub fn compute_difficulty(
					&self,
				) -> ::subxt::runtime_api::Payload<types::ComputeDifficulty, ::core::primitive::u128>
				{
					::subxt::runtime_api::Payload::new_static(
						"BlockSealSpecApis",
						"compute_difficulty",
						types::ComputeDifficulty {},
						[
							107u8, 85u8, 138u8, 118u8, 50u8, 163u8, 93u8, 174u8, 116u8, 181u8,
							93u8, 89u8, 224u8, 218u8, 243u8, 57u8, 18u8, 214u8, 116u8, 246u8,
							182u8, 178u8, 27u8, 75u8, 147u8, 6u8, 25u8, 218u8, 138u8, 33u8, 138u8,
							9u8,
						],
					)
				}
				pub fn parent_voting_key(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::ParentVotingKey,
					::core::option::Option<::subxt::utils::H256>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockSealSpecApis",
						"parent_voting_key",
						types::ParentVotingKey {},
						[
							143u8, 101u8, 87u8, 16u8, 236u8, 115u8, 2u8, 140u8, 177u8, 185u8,
							174u8, 136u8, 174u8, 189u8, 191u8, 24u8, 43u8, 155u8, 200u8, 131u8,
							225u8, 70u8, 254u8, 143u8, 49u8, 223u8, 41u8, 244u8, 206u8, 26u8,
							222u8, 188u8,
						],
					)
				}
				pub fn create_vote_digest(
					&self,
					tick_notebooks: ::std::vec::Vec<
						runtime_types::ulx_primitives::notary::NotaryNotebookVoteDigestDetails,
					>,
				) -> ::subxt::runtime_api::Payload<
					types::CreateVoteDigest,
					runtime_types::ulx_primitives::digests::BlockVoteDigest,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockSealSpecApis",
						"create_vote_digest",
						types::CreateVoteDigest { tick_notebooks },
						[
							145u8, 246u8, 241u8, 116u8, 198u8, 250u8, 78u8, 192u8, 109u8, 159u8,
							49u8, 16u8, 1u8, 235u8, 226u8, 40u8, 193u8, 84u8, 238u8, 104u8, 198u8,
							152u8, 40u8, 27u8, 18u8, 176u8, 254u8, 65u8, 194u8, 81u8, 228u8, 194u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct VoteMinimum {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ComputeDifficulty {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ParentVotingKey {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CreateVoteDigest {
					pub tick_notebooks: ::std::vec::Vec<
						runtime_types::ulx_primitives::notary::NotaryNotebookVoteDigestDetails,
					>,
				}
			}
		}
		pub mod notary_apis {
			use super::{root_mod, runtime_types};
			pub struct NotaryApis;
			impl NotaryApis {
				pub fn notary_by_id(
					&self,
					notary_id: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<
					types::NotaryById,
					::core::option::Option<
						runtime_types::ulx_primitives::notary::NotaryRecord<
							::subxt::utils::AccountId32,
							::core::primitive::u32,
						>,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"NotaryApis",
						"notary_by_id",
						types::NotaryById { notary_id },
						[
							9u8, 208u8, 40u8, 114u8, 57u8, 225u8, 169u8, 220u8, 8u8, 57u8, 24u8,
							124u8, 230u8, 220u8, 160u8, 115u8, 252u8, 103u8, 50u8, 105u8, 212u8,
							242u8, 118u8, 14u8, 88u8, 240u8, 247u8, 153u8, 188u8, 244u8, 58u8,
							63u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct NotaryById {
					pub notary_id: ::core::primitive::u32,
				}
			}
		}
		pub mod mining_authority_apis {
			use super::{root_mod, runtime_types};
			pub struct MiningAuthorityApis;
			impl MiningAuthorityApis {
				pub fn xor_closest_authority(
					&self,
					nonce: runtime_types::primitive_types::U256,
				) -> ::subxt::runtime_api::Payload<
					types::XorClosestAuthority,
					::core::option::Option<
						runtime_types::ulx_primitives::block_seal::MiningAuthority<
							runtime_types::ulx_primitives::block_seal::app::Public,
							::subxt::utils::AccountId32,
						>,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"MiningAuthorityApis",
						"xor_closest_authority",
						types::XorClosestAuthority { nonce },
						[
							120u8, 144u8, 182u8, 242u8, 31u8, 95u8, 158u8, 254u8, 215u8, 170u8,
							106u8, 242u8, 23u8, 23u8, 111u8, 6u8, 176u8, 52u8, 184u8, 109u8, 42u8,
							138u8, 42u8, 80u8, 202u8, 172u8, 255u8, 52u8, 111u8, 3u8, 33u8, 102u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct XorClosestAuthority {
					pub nonce: runtime_types::primitive_types::U256,
				}
			}
		}
		pub mod mining_slot_api {
			use super::{root_mod, runtime_types};
			#[doc = " This runtime api allows people to query the upcoming mining_slot"]
			pub struct MiningSlotApi;
			impl MiningSlotApi {
				pub fn next_slot_era(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::NextSlotEra,
					(::core::primitive::u32, ::core::primitive::u32),
				> {
					::subxt::runtime_api::Payload::new_static(
						"MiningSlotApi",
						"next_slot_era",
						types::NextSlotEra {},
						[
							101u8, 27u8, 62u8, 52u8, 219u8, 204u8, 173u8, 178u8, 17u8, 87u8, 198u8,
							186u8, 162u8, 215u8, 162u8, 64u8, 214u8, 87u8, 101u8, 112u8, 128u8,
							160u8, 134u8, 183u8, 46u8, 141u8, 167u8, 2u8, 15u8, 77u8, 5u8, 224u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct NextSlotEra {}
			}
		}
		pub mod notebook_apis {
			use super::{root_mod, runtime_types};
			pub struct NotebookApis;
			impl NotebookApis {
				pub fn audit_notebook_and_get_votes(
					&self,
					version: ::core::primitive::u32,
					notary_id: ::core::primitive::u32,
					notebook_number: ::core::primitive::u32,
					header_hash: ::subxt::utils::H256,
					vote_minimums: ::subxt::utils::KeyedVec<
						::subxt::utils::H256,
						::core::primitive::u128,
					>,
					bytes: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::runtime_api::Payload<
					types::AuditNotebookAndGetVotes,
					::core::result::Result<
						runtime_types::ulx_primitives::apis::NotebookVotes,
						runtime_types::ulx_notary_audit::error::VerifyError,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"NotebookApis",
						"audit_notebook_and_get_votes",
						types::AuditNotebookAndGetVotes {
							version,
							notary_id,
							notebook_number,
							header_hash,
							vote_minimums,
							bytes,
						},
						[
							149u8, 109u8, 199u8, 101u8, 201u8, 103u8, 187u8, 5u8, 82u8, 236u8,
							248u8, 152u8, 177u8, 251u8, 15u8, 208u8, 60u8, 158u8, 194u8, 216u8,
							141u8, 93u8, 77u8, 30u8, 44u8, 122u8, 246u8, 5u8, 205u8, 227u8, 68u8,
							21u8,
						],
					)
				}
				pub fn get_best_vote_proofs(
					&self,
					votes: ::subxt::utils::KeyedVec<
						::core::primitive::u32,
						runtime_types::ulx_primitives::apis::NotebookVotes,
					>,
				) -> ::subxt::runtime_api::Payload<
					types::GetBestVoteProofs,
					::core::result::Result<
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::block_vote::BestBlockVoteProofT<
								::subxt::utils::H256,
							>,
						>,
						runtime_types::sp_runtime::DispatchError,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"NotebookApis",
						"get_best_vote_proofs",
						types::GetBestVoteProofs { votes },
						[
							85u8, 106u8, 32u8, 136u8, 17u8, 31u8, 98u8, 33u8, 29u8, 107u8, 211u8,
							9u8, 96u8, 60u8, 139u8, 216u8, 109u8, 219u8, 125u8, 229u8, 15u8, 44u8,
							143u8, 38u8, 210u8, 174u8, 199u8, 155u8, 49u8, 158u8, 218u8, 254u8,
						],
					)
				}
				pub fn decode_notebook_vote_details(
					&self,
					extrinsic : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) >,
				) -> ::subxt::runtime_api::Payload<
					types::DecodeNotebookVoteDetails,
					::core::option::Option<
						runtime_types::ulx_primitives::notary::NotaryNotebookVoteDetails<
							::subxt::utils::H256,
						>,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"NotebookApis",
						"decode_notebook_vote_details",
						types::DecodeNotebookVoteDetails { extrinsic },
						[
							195u8, 31u8, 236u8, 230u8, 149u8, 165u8, 90u8, 96u8, 253u8, 214u8,
							249u8, 73u8, 121u8, 135u8, 219u8, 107u8, 126u8, 168u8, 116u8, 16u8,
							155u8, 236u8, 76u8, 27u8, 166u8, 62u8, 66u8, 63u8, 78u8, 40u8, 228u8,
							136u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AuditNotebookAndGetVotes {
					pub version: ::core::primitive::u32,
					pub notary_id: ::core::primitive::u32,
					pub notebook_number: ::core::primitive::u32,
					pub header_hash: ::subxt::utils::H256,
					pub vote_minimums:
						::subxt::utils::KeyedVec<::subxt::utils::H256, ::core::primitive::u128>,
					pub bytes: ::std::vec::Vec<::core::primitive::u8>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct GetBestVoteProofs {
					pub votes: ::subxt::utils::KeyedVec<
						::core::primitive::u32,
						runtime_types::ulx_primitives::apis::NotebookVotes,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DecodeNotebookVoteDetails { pub extrinsic : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > , }
			}
		}
		pub mod tick_apis {
			use super::{root_mod, runtime_types};
			pub struct TickApis;
			impl TickApis {
				pub fn current_tick(
					&self,
				) -> ::subxt::runtime_api::Payload<types::CurrentTick, ::core::primitive::u32> {
					::subxt::runtime_api::Payload::new_static(
						"TickApis",
						"current_tick",
						types::CurrentTick {},
						[
							14u8, 164u8, 187u8, 5u8, 165u8, 232u8, 115u8, 62u8, 28u8, 152u8, 59u8,
							125u8, 52u8, 220u8, 63u8, 169u8, 198u8, 88u8, 58u8, 185u8, 7u8, 214u8,
							232u8, 65u8, 163u8, 38u8, 161u8, 233u8, 164u8, 129u8, 67u8, 193u8,
						],
					)
				}
				pub fn ticker(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::Ticker,
					runtime_types::ulx_primitives::tick::Ticker,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TickApis",
						"ticker",
						types::Ticker {},
						[
							184u8, 118u8, 210u8, 100u8, 79u8, 7u8, 156u8, 224u8, 30u8, 166u8, 97u8,
							142u8, 164u8, 179u8, 42u8, 92u8, 38u8, 182u8, 105u8, 52u8, 126u8,
							229u8, 95u8, 143u8, 255u8, 13u8, 107u8, 105u8, 245u8, 155u8, 69u8,
							36u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CurrentTick {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Ticker {}
			}
		}
		pub mod grandpa_api {
			use super::{root_mod, runtime_types};
			#[doc = " APIs for integrating the GRANDPA finality gadget into runtimes."]
			#[doc = " This should be implemented on the runtime side."]
			#[doc = ""]
			#[doc = " This is primarily used for negotiating authority-set changes for the"]
			#[doc = " gadget. GRANDPA uses a signaling model of changing authority sets:"]
			#[doc = " changes should be signaled with a delay of N blocks, and then automatically"]
			#[doc = " applied in the runtime after those N blocks have passed."]
			#[doc = ""]
			#[doc = " The consensus protocol will coordinate the handoff externally."]
			pub struct GrandpaApi;
			impl GrandpaApi {
				#[doc = " Get the current GRANDPA authorities and weights. This should not change except"]
				#[doc = " for when changes are scheduled and the corresponding delay has passed."]
				#[doc = ""]
				#[doc = " When called at block B, it will return the set of authorities that should be"]
				#[doc = " used to finalize descendants of this block (B+1, B+2, ...). The block B itself"]
				#[doc = " is finalized by the authorities from block B-1."]
				pub fn grandpa_authorities(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::GrandpaAuthorities,
					::std::vec::Vec<(
						runtime_types::sp_consensus_grandpa::app::Public,
						::core::primitive::u64,
					)>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GrandpaApi",
						"grandpa_authorities",
						types::GrandpaAuthorities {},
						[
							166u8, 76u8, 160u8, 101u8, 242u8, 145u8, 213u8, 10u8, 16u8, 130u8,
							230u8, 196u8, 125u8, 152u8, 92u8, 143u8, 119u8, 223u8, 140u8, 189u8,
							203u8, 95u8, 52u8, 105u8, 147u8, 107u8, 135u8, 228u8, 62u8, 178u8,
							128u8, 33u8,
						],
					)
				}
				#[doc = " Submits an unsigned extrinsic to report an equivocation. The caller"]
				#[doc = " must provide the equivocation proof and a key ownership proof"]
				#[doc = " (should be obtained using `generate_key_ownership_proof`). The"]
				#[doc = " extrinsic will be unsigned and should only be accepted for local"]
				#[doc = " authorship (not to be broadcast to the network). This method returns"]
				#[doc = " `None` when creation of the extrinsic fails, e.g. if equivocation"]
				#[doc = " reporting is disabled for the given runtime (i.e. this method is"]
				#[doc = " hardcoded to return `None`). Only useful in an offchain context."]
				pub fn submit_report_equivocation_unsigned_extrinsic(
					&self,
					equivocation_proof: runtime_types::sp_consensus_grandpa::EquivocationProof<
						::subxt::utils::H256,
						::core::primitive::u32,
					>,
					key_owner_proof: runtime_types::sp_consensus_grandpa::OpaqueKeyOwnershipProof,
				) -> ::subxt::runtime_api::Payload<
					types::SubmitReportEquivocationUnsignedExtrinsic,
					::core::option::Option<()>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GrandpaApi",
						"submit_report_equivocation_unsigned_extrinsic",
						types::SubmitReportEquivocationUnsignedExtrinsic {
							equivocation_proof,
							key_owner_proof,
						},
						[
							112u8, 94u8, 150u8, 250u8, 132u8, 127u8, 185u8, 24u8, 113u8, 62u8,
							28u8, 171u8, 83u8, 9u8, 41u8, 228u8, 92u8, 137u8, 29u8, 190u8, 214u8,
							232u8, 100u8, 66u8, 100u8, 168u8, 149u8, 122u8, 93u8, 17u8, 236u8,
							104u8,
						],
					)
				}
				#[doc = " Generates a proof of key ownership for the given authority in the"]
				#[doc = " given set. An example usage of this module is coupled with the"]
				#[doc = " session historical module to prove that a given authority key is"]
				#[doc = " tied to a given staking identity during a specific session. Proofs"]
				#[doc = " of key ownership are necessary for submitting equivocation reports."]
				#[doc = " NOTE: even though the API takes a `set_id` as parameter the current"]
				#[doc = " implementations ignore this parameter and instead rely on this"]
				#[doc = " method being called at the correct block height, i.e. any point at"]
				#[doc = " which the given set id is live on-chain. Future implementations will"]
				#[doc = " instead use indexed data through an offchain worker, not requiring"]
				#[doc = " older states to be available."]
				pub fn generate_key_ownership_proof(
					&self,
					set_id: ::core::primitive::u64,
					authority_id: runtime_types::sp_consensus_grandpa::app::Public,
				) -> ::subxt::runtime_api::Payload<
					types::GenerateKeyOwnershipProof,
					::core::option::Option<
						runtime_types::sp_consensus_grandpa::OpaqueKeyOwnershipProof,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GrandpaApi",
						"generate_key_ownership_proof",
						types::GenerateKeyOwnershipProof { set_id, authority_id },
						[
							40u8, 126u8, 113u8, 27u8, 245u8, 45u8, 123u8, 138u8, 12u8, 3u8, 125u8,
							186u8, 151u8, 53u8, 186u8, 93u8, 13u8, 150u8, 163u8, 176u8, 206u8,
							89u8, 244u8, 127u8, 182u8, 85u8, 203u8, 41u8, 101u8, 183u8, 209u8,
							179u8,
						],
					)
				}
				#[doc = " Get current GRANDPA authority set id."]
				pub fn current_set_id(
					&self,
				) -> ::subxt::runtime_api::Payload<types::CurrentSetId, ::core::primitive::u64> {
					::subxt::runtime_api::Payload::new_static(
						"GrandpaApi",
						"current_set_id",
						types::CurrentSetId {},
						[
							42u8, 230u8, 120u8, 211u8, 156u8, 245u8, 109u8, 86u8, 100u8, 146u8,
							234u8, 205u8, 41u8, 183u8, 109u8, 42u8, 17u8, 33u8, 156u8, 25u8, 139u8,
							84u8, 101u8, 75u8, 232u8, 198u8, 87u8, 136u8, 218u8, 233u8, 103u8,
							156u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct GrandpaAuthorities {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SubmitReportEquivocationUnsignedExtrinsic {
					pub equivocation_proof: runtime_types::sp_consensus_grandpa::EquivocationProof<
						::subxt::utils::H256,
						::core::primitive::u32,
					>,
					pub key_owner_proof:
						runtime_types::sp_consensus_grandpa::OpaqueKeyOwnershipProof,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct GenerateKeyOwnershipProof {
					pub set_id: ::core::primitive::u64,
					pub authority_id: runtime_types::sp_consensus_grandpa::app::Public,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CurrentSetId {}
			}
		}
		pub mod genesis_builder {
			use super::{root_mod, runtime_types};
			#[doc = " API to interact with GenesisConfig for the runtime"]
			pub struct GenesisBuilder;
			impl GenesisBuilder {
				#[doc = " Creates the default `GenesisConfig` and returns it as a JSON blob."]
				#[doc = ""]
				#[doc = " This function instantiates the default `GenesisConfig` struct for the runtime and serializes it into a JSON"]
				#[doc = " blob. It returns a `Vec<u8>` containing the JSON representation of the default `GenesisConfig`."]
				pub fn create_default_config(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::CreateDefaultConfig,
					::std::vec::Vec<::core::primitive::u8>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GenesisBuilder",
						"create_default_config",
						types::CreateDefaultConfig {},
						[
							238u8, 5u8, 139u8, 81u8, 184u8, 155u8, 221u8, 118u8, 190u8, 76u8,
							229u8, 67u8, 132u8, 89u8, 83u8, 80u8, 56u8, 171u8, 169u8, 64u8, 123u8,
							20u8, 129u8, 159u8, 28u8, 135u8, 84u8, 52u8, 192u8, 98u8, 104u8, 214u8,
						],
					)
				}
				#[doc = " Build `GenesisConfig` from a JSON blob not using any defaults and store it in the storage."]
				#[doc = ""]
				#[doc = " This function deserializes the full `GenesisConfig` from the given JSON blob and puts it into the storage."]
				#[doc = " If the provided JSON blob is incorrect or incomplete or the deserialization fails, an error is returned."]
				#[doc = " It is recommended to log any errors encountered during the process."]
				#[doc = ""]
				#[doc = " Please note that provided json blob must contain all `GenesisConfig` fields, no defaults will be used."]
				pub fn build_config(
					&self,
					json: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::runtime_api::Payload<
					types::BuildConfig,
					::core::result::Result<(), ::std::string::String>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GenesisBuilder",
						"build_config",
						types::BuildConfig { json },
						[
							6u8, 98u8, 68u8, 125u8, 157u8, 26u8, 107u8, 86u8, 213u8, 227u8, 26u8,
							229u8, 122u8, 161u8, 229u8, 114u8, 123u8, 192u8, 66u8, 231u8, 148u8,
							175u8, 5u8, 185u8, 248u8, 88u8, 40u8, 122u8, 230u8, 209u8, 170u8,
							254u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CreateDefaultConfig {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BuildConfig {
					pub json: ::std::vec::Vec<::core::primitive::u8>,
				}
			}
		}
	}
	pub struct ConstantsApi;
	impl ConstantsApi {
		pub fn system(&self) -> system::constants::ConstantsApi {
			system::constants::ConstantsApi
		}
		pub fn timestamp(&self) -> timestamp::constants::ConstantsApi {
			timestamp::constants::ConstantsApi
		}
		pub fn mining_slot(&self) -> mining_slot::constants::ConstantsApi {
			mining_slot::constants::ConstantsApi
		}
		pub fn bond(&self) -> bond::constants::ConstantsApi {
			bond::constants::ConstantsApi
		}
		pub fn notaries(&self) -> notaries::constants::ConstantsApi {
			notaries::constants::ConstantsApi
		}
		pub fn chain_transfer(&self) -> chain_transfer::constants::ConstantsApi {
			chain_transfer::constants::ConstantsApi
		}
		pub fn block_seal_spec(&self) -> block_seal_spec::constants::ConstantsApi {
			block_seal_spec::constants::ConstantsApi
		}
		pub fn block_rewards(&self) -> block_rewards::constants::ConstantsApi {
			block_rewards::constants::ConstantsApi
		}
		pub fn grandpa(&self) -> grandpa::constants::ConstantsApi {
			grandpa::constants::ConstantsApi
		}
		pub fn argon_balances(&self) -> argon_balances::constants::ConstantsApi {
			argon_balances::constants::ConstantsApi
		}
		pub fn ulixee_balances(&self) -> ulixee_balances::constants::ConstantsApi {
			ulixee_balances::constants::ConstantsApi
		}
		pub fn tx_pause(&self) -> tx_pause::constants::ConstantsApi {
			tx_pause::constants::ConstantsApi
		}
		pub fn transaction_payment(&self) -> transaction_payment::constants::ConstantsApi {
			transaction_payment::constants::ConstantsApi
		}
	}
	pub struct StorageApi;
	impl StorageApi {
		pub fn system(&self) -> system::storage::StorageApi {
			system::storage::StorageApi
		}
		pub fn timestamp(&self) -> timestamp::storage::StorageApi {
			timestamp::storage::StorageApi
		}
		pub fn ticks(&self) -> ticks::storage::StorageApi {
			ticks::storage::StorageApi
		}
		pub fn mining_slot(&self) -> mining_slot::storage::StorageApi {
			mining_slot::storage::StorageApi
		}
		pub fn bond(&self) -> bond::storage::StorageApi {
			bond::storage::StorageApi
		}
		pub fn notaries(&self) -> notaries::storage::StorageApi {
			notaries::storage::StorageApi
		}
		pub fn notebook(&self) -> notebook::storage::StorageApi {
			notebook::storage::StorageApi
		}
		pub fn chain_transfer(&self) -> chain_transfer::storage::StorageApi {
			chain_transfer::storage::StorageApi
		}
		pub fn block_seal_spec(&self) -> block_seal_spec::storage::StorageApi {
			block_seal_spec::storage::StorageApi
		}
		pub fn authorship(&self) -> authorship::storage::StorageApi {
			authorship::storage::StorageApi
		}
		pub fn historical(&self) -> historical::storage::StorageApi {
			historical::storage::StorageApi
		}
		pub fn session(&self) -> session::storage::StorageApi {
			session::storage::StorageApi
		}
		pub fn block_seal(&self) -> block_seal::storage::StorageApi {
			block_seal::storage::StorageApi
		}
		pub fn block_rewards(&self) -> block_rewards::storage::StorageApi {
			block_rewards::storage::StorageApi
		}
		pub fn grandpa(&self) -> grandpa::storage::StorageApi {
			grandpa::storage::StorageApi
		}
		pub fn offences(&self) -> offences::storage::StorageApi {
			offences::storage::StorageApi
		}
		pub fn argon_balances(&self) -> argon_balances::storage::StorageApi {
			argon_balances::storage::StorageApi
		}
		pub fn ulixee_balances(&self) -> ulixee_balances::storage::StorageApi {
			ulixee_balances::storage::StorageApi
		}
		pub fn tx_pause(&self) -> tx_pause::storage::StorageApi {
			tx_pause::storage::StorageApi
		}
		pub fn transaction_payment(&self) -> transaction_payment::storage::StorageApi {
			transaction_payment::storage::StorageApi
		}
		pub fn sudo(&self) -> sudo::storage::StorageApi {
			sudo::storage::StorageApi
		}
	}
	pub struct TransactionApi;
	impl TransactionApi {
		pub fn system(&self) -> system::calls::TransactionApi {
			system::calls::TransactionApi
		}
		pub fn timestamp(&self) -> timestamp::calls::TransactionApi {
			timestamp::calls::TransactionApi
		}
		pub fn ticks(&self) -> ticks::calls::TransactionApi {
			ticks::calls::TransactionApi
		}
		pub fn mining_slot(&self) -> mining_slot::calls::TransactionApi {
			mining_slot::calls::TransactionApi
		}
		pub fn bond(&self) -> bond::calls::TransactionApi {
			bond::calls::TransactionApi
		}
		pub fn notaries(&self) -> notaries::calls::TransactionApi {
			notaries::calls::TransactionApi
		}
		pub fn notebook(&self) -> notebook::calls::TransactionApi {
			notebook::calls::TransactionApi
		}
		pub fn chain_transfer(&self) -> chain_transfer::calls::TransactionApi {
			chain_transfer::calls::TransactionApi
		}
		pub fn block_seal_spec(&self) -> block_seal_spec::calls::TransactionApi {
			block_seal_spec::calls::TransactionApi
		}
		pub fn session(&self) -> session::calls::TransactionApi {
			session::calls::TransactionApi
		}
		pub fn block_seal(&self) -> block_seal::calls::TransactionApi {
			block_seal::calls::TransactionApi
		}
		pub fn block_rewards(&self) -> block_rewards::calls::TransactionApi {
			block_rewards::calls::TransactionApi
		}
		pub fn grandpa(&self) -> grandpa::calls::TransactionApi {
			grandpa::calls::TransactionApi
		}
		pub fn argon_balances(&self) -> argon_balances::calls::TransactionApi {
			argon_balances::calls::TransactionApi
		}
		pub fn ulixee_balances(&self) -> ulixee_balances::calls::TransactionApi {
			ulixee_balances::calls::TransactionApi
		}
		pub fn tx_pause(&self) -> tx_pause::calls::TransactionApi {
			tx_pause::calls::TransactionApi
		}
		pub fn sudo(&self) -> sudo::calls::TransactionApi {
			sudo::calls::TransactionApi
		}
	}
	#[doc = r" check whether the metadata provided is aligned with this statically generated code."]
	pub fn is_codegen_valid_for(metadata: &::subxt::Metadata) -> bool {
		let runtime_metadata_hash = metadata
			.hasher()
			.only_these_pallets(&PALLETS)
			.only_these_runtime_apis(&RUNTIME_APIS)
			.hash();
		runtime_metadata_hash ==
			[
				44u8, 153u8, 57u8, 119u8, 8u8, 33u8, 108u8, 7u8, 175u8, 74u8, 99u8, 50u8, 189u8,
				213u8, 177u8, 245u8, 172u8, 174u8, 249u8, 99u8, 116u8, 18u8, 247u8, 232u8, 237u8,
				103u8, 50u8, 129u8, 49u8, 149u8, 197u8, 108u8,
			]
	}
	pub mod system {
		use super::{root_mod, runtime_types};
		#[doc = "Error for the System pallet"]
		pub type Error = runtime_types::frame_system::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::frame_system::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Remark {
					pub remark: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Remark {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "remark";
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetHeapPages {
					pub pages: ::core::primitive::u64,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetHeapPages {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "set_heap_pages";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetCode {
					pub code: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetCode {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "set_code";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetCodeWithoutChecks {
					pub code: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetCodeWithoutChecks {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "set_code_without_checks";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetStorage {
					pub items: ::std::vec::Vec<(
						::std::vec::Vec<::core::primitive::u8>,
						::std::vec::Vec<::core::primitive::u8>,
					)>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetStorage {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "set_storage";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct KillStorage {
					pub keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
				}
				impl ::subxt::blocks::StaticExtrinsic for KillStorage {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "kill_storage";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct KillPrefix {
					pub prefix: ::std::vec::Vec<::core::primitive::u8>,
					pub subkeys: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for KillPrefix {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "kill_prefix";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RemarkWithEvent {
					pub remark: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for RemarkWithEvent {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "remark_with_event";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::remark`]."]
				pub fn remark(
					&self,
					remark: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::Remark> {
					::subxt::tx::Payload::new_static(
						"System",
						"remark",
						types::Remark { remark },
						[
							43u8, 126u8, 180u8, 174u8, 141u8, 48u8, 52u8, 125u8, 166u8, 212u8,
							216u8, 98u8, 100u8, 24u8, 132u8, 71u8, 101u8, 64u8, 246u8, 169u8, 33u8,
							250u8, 147u8, 208u8, 2u8, 40u8, 129u8, 209u8, 232u8, 207u8, 207u8,
							13u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_heap_pages`]."]
				pub fn set_heap_pages(
					&self,
					pages: ::core::primitive::u64,
				) -> ::subxt::tx::Payload<types::SetHeapPages> {
					::subxt::tx::Payload::new_static(
						"System",
						"set_heap_pages",
						types::SetHeapPages { pages },
						[
							188u8, 191u8, 99u8, 216u8, 219u8, 109u8, 141u8, 50u8, 78u8, 235u8,
							215u8, 242u8, 195u8, 24u8, 111u8, 76u8, 229u8, 64u8, 99u8, 225u8,
							134u8, 121u8, 81u8, 209u8, 127u8, 223u8, 98u8, 215u8, 150u8, 70u8,
							57u8, 147u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_code`]."]
				pub fn set_code(
					&self,
					code: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::SetCode> {
					::subxt::tx::Payload::new_static(
						"System",
						"set_code",
						types::SetCode { code },
						[
							233u8, 248u8, 88u8, 245u8, 28u8, 65u8, 25u8, 169u8, 35u8, 237u8, 19u8,
							203u8, 136u8, 160u8, 18u8, 3u8, 20u8, 197u8, 81u8, 169u8, 244u8, 188u8,
							27u8, 147u8, 147u8, 236u8, 65u8, 25u8, 3u8, 143u8, 182u8, 22u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_code_without_checks`]."]
				pub fn set_code_without_checks(
					&self,
					code: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::SetCodeWithoutChecks> {
					::subxt::tx::Payload::new_static(
						"System",
						"set_code_without_checks",
						types::SetCodeWithoutChecks { code },
						[
							82u8, 212u8, 157u8, 44u8, 70u8, 0u8, 143u8, 15u8, 109u8, 109u8, 107u8,
							157u8, 141u8, 42u8, 169u8, 11u8, 15u8, 186u8, 252u8, 138u8, 10u8,
							147u8, 15u8, 178u8, 247u8, 229u8, 213u8, 98u8, 207u8, 231u8, 119u8,
							115u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_storage`]."]
				pub fn set_storage(
					&self,
					items: ::std::vec::Vec<(
						::std::vec::Vec<::core::primitive::u8>,
						::std::vec::Vec<::core::primitive::u8>,
					)>,
				) -> ::subxt::tx::Payload<types::SetStorage> {
					::subxt::tx::Payload::new_static(
						"System",
						"set_storage",
						types::SetStorage { items },
						[
							141u8, 216u8, 52u8, 222u8, 223u8, 136u8, 123u8, 181u8, 19u8, 75u8,
							163u8, 102u8, 229u8, 189u8, 158u8, 142u8, 95u8, 235u8, 240u8, 49u8,
							150u8, 76u8, 78u8, 137u8, 126u8, 88u8, 183u8, 88u8, 231u8, 146u8,
							234u8, 43u8,
						],
					)
				}
				#[doc = "See [`Pallet::kill_storage`]."]
				pub fn kill_storage(
					&self,
					keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
				) -> ::subxt::tx::Payload<types::KillStorage> {
					::subxt::tx::Payload::new_static(
						"System",
						"kill_storage",
						types::KillStorage { keys },
						[
							73u8, 63u8, 196u8, 36u8, 144u8, 114u8, 34u8, 213u8, 108u8, 93u8, 209u8,
							234u8, 153u8, 185u8, 33u8, 91u8, 187u8, 195u8, 223u8, 130u8, 58u8,
							156u8, 63u8, 47u8, 228u8, 249u8, 216u8, 139u8, 143u8, 177u8, 41u8,
							35u8,
						],
					)
				}
				#[doc = "See [`Pallet::kill_prefix`]."]
				pub fn kill_prefix(
					&self,
					prefix: ::std::vec::Vec<::core::primitive::u8>,
					subkeys: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::KillPrefix> {
					::subxt::tx::Payload::new_static(
						"System",
						"kill_prefix",
						types::KillPrefix { prefix, subkeys },
						[
							184u8, 57u8, 139u8, 24u8, 208u8, 87u8, 108u8, 215u8, 198u8, 189u8,
							175u8, 242u8, 167u8, 215u8, 97u8, 63u8, 110u8, 166u8, 238u8, 98u8,
							67u8, 236u8, 111u8, 110u8, 234u8, 81u8, 102u8, 5u8, 182u8, 5u8, 214u8,
							85u8,
						],
					)
				}
				#[doc = "See [`Pallet::remark_with_event`]."]
				pub fn remark_with_event(
					&self,
					remark: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::RemarkWithEvent> {
					::subxt::tx::Payload::new_static(
						"System",
						"remark_with_event",
						types::RemarkWithEvent { remark },
						[
							120u8, 120u8, 153u8, 92u8, 184u8, 85u8, 34u8, 2u8, 174u8, 206u8, 105u8,
							228u8, 233u8, 130u8, 80u8, 246u8, 228u8, 59u8, 234u8, 240u8, 4u8, 49u8,
							147u8, 170u8, 115u8, 91u8, 149u8, 200u8, 228u8, 181u8, 8u8, 154u8,
						],
					)
				}
			}
		}
		#[doc = "Event for the System pallet."]
		pub type Event = runtime_types::frame_system::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An extrinsic completed successfully."]
			pub struct ExtrinsicSuccess {
				pub dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
			}
			impl ::subxt::events::StaticEvent for ExtrinsicSuccess {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "ExtrinsicSuccess";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An extrinsic failed."]
			pub struct ExtrinsicFailed {
				pub dispatch_error: runtime_types::sp_runtime::DispatchError,
				pub dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
			}
			impl ::subxt::events::StaticEvent for ExtrinsicFailed {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "ExtrinsicFailed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "`:code` was updated."]
			pub struct CodeUpdated;
			impl ::subxt::events::StaticEvent for CodeUpdated {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "CodeUpdated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A new account was created."]
			pub struct NewAccount {
				pub account: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for NewAccount {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "NewAccount";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was reaped."]
			pub struct KilledAccount {
				pub account: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for KilledAccount {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "KilledAccount";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "On on-chain remark happened."]
			pub struct Remarked {
				pub sender: ::subxt::utils::AccountId32,
				pub hash: ::subxt::utils::H256,
			}
			impl ::subxt::events::StaticEvent for Remarked {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "Remarked";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The full account information for a particular account ID."]
				pub fn account(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::frame_system::AccountInfo<
						::core::primitive::u32,
						runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"Account",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							14u8, 233u8, 115u8, 214u8, 0u8, 109u8, 222u8, 121u8, 162u8, 65u8, 60u8,
							175u8, 209u8, 79u8, 222u8, 124u8, 22u8, 235u8, 138u8, 176u8, 133u8,
							124u8, 90u8, 158u8, 85u8, 45u8, 37u8, 174u8, 47u8, 79u8, 47u8, 166u8,
						],
					)
				}
				#[doc = " The full account information for a particular account ID."]
				pub fn account_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::frame_system::AccountInfo<
						::core::primitive::u32,
						runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"Account",
						Vec::new(),
						[
							14u8, 233u8, 115u8, 214u8, 0u8, 109u8, 222u8, 121u8, 162u8, 65u8, 60u8,
							175u8, 209u8, 79u8, 222u8, 124u8, 22u8, 235u8, 138u8, 176u8, 133u8,
							124u8, 90u8, 158u8, 85u8, 45u8, 37u8, 174u8, 47u8, 79u8, 47u8, 166u8,
						],
					)
				}
				#[doc = " Total extrinsics count for the current block."]
				pub fn extrinsic_count(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"ExtrinsicCount",
						vec![],
						[
							102u8, 76u8, 236u8, 42u8, 40u8, 231u8, 33u8, 222u8, 123u8, 147u8,
							153u8, 148u8, 234u8, 203u8, 181u8, 119u8, 6u8, 187u8, 177u8, 199u8,
							120u8, 47u8, 137u8, 254u8, 96u8, 100u8, 165u8, 182u8, 249u8, 230u8,
							159u8, 79u8,
						],
					)
				}
				#[doc = " The current weight for the block."]
				pub fn block_weight(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::frame_support::dispatch::PerDispatchClass<
						runtime_types::sp_weights::weight_v2::Weight,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"BlockWeight",
						vec![],
						[
							158u8, 46u8, 228u8, 89u8, 210u8, 214u8, 84u8, 154u8, 50u8, 68u8, 63u8,
							62u8, 43u8, 42u8, 99u8, 27u8, 54u8, 42u8, 146u8, 44u8, 241u8, 216u8,
							229u8, 30u8, 216u8, 255u8, 165u8, 238u8, 181u8, 130u8, 36u8, 102u8,
						],
					)
				}
				#[doc = " Total length (in bytes) for all extrinsics put together, for the current block."]
				pub fn all_extrinsics_len(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"AllExtrinsicsLen",
						vec![],
						[
							117u8, 86u8, 61u8, 243u8, 41u8, 51u8, 102u8, 214u8, 137u8, 100u8,
							243u8, 185u8, 122u8, 174u8, 187u8, 117u8, 86u8, 189u8, 63u8, 135u8,
							101u8, 218u8, 203u8, 201u8, 237u8, 254u8, 128u8, 183u8, 169u8, 221u8,
							242u8, 65u8,
						],
					)
				}
				#[doc = " Map of block numbers to block hashes."]
				pub fn block_hash(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::H256,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"BlockHash",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							217u8, 32u8, 215u8, 253u8, 24u8, 182u8, 207u8, 178u8, 157u8, 24u8,
							103u8, 100u8, 195u8, 165u8, 69u8, 152u8, 112u8, 181u8, 56u8, 192u8,
							164u8, 16u8, 20u8, 222u8, 28u8, 214u8, 144u8, 142u8, 146u8, 69u8,
							202u8, 118u8,
						],
					)
				}
				#[doc = " Map of block numbers to block hashes."]
				pub fn block_hash_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::H256,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"BlockHash",
						Vec::new(),
						[
							217u8, 32u8, 215u8, 253u8, 24u8, 182u8, 207u8, 178u8, 157u8, 24u8,
							103u8, 100u8, 195u8, 165u8, 69u8, 152u8, 112u8, 181u8, 56u8, 192u8,
							164u8, 16u8, 20u8, 222u8, 28u8, 214u8, 144u8, 142u8, 146u8, 69u8,
							202u8, 118u8,
						],
					)
				}
				#[doc = " Extrinsics data for the current block (maps an extrinsic's index to its data)."]
				pub fn extrinsic_data(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<::core::primitive::u8>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"ExtrinsicData",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							160u8, 180u8, 122u8, 18u8, 196u8, 26u8, 2u8, 37u8, 115u8, 232u8, 133u8,
							220u8, 106u8, 245u8, 4u8, 129u8, 42u8, 84u8, 241u8, 45u8, 199u8, 179u8,
							128u8, 61u8, 170u8, 137u8, 231u8, 156u8, 247u8, 57u8, 47u8, 38u8,
						],
					)
				}
				#[doc = " Extrinsics data for the current block (maps an extrinsic's index to its data)."]
				pub fn extrinsic_data_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<::core::primitive::u8>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"ExtrinsicData",
						Vec::new(),
						[
							160u8, 180u8, 122u8, 18u8, 196u8, 26u8, 2u8, 37u8, 115u8, 232u8, 133u8,
							220u8, 106u8, 245u8, 4u8, 129u8, 42u8, 84u8, 241u8, 45u8, 199u8, 179u8,
							128u8, 61u8, 170u8, 137u8, 231u8, 156u8, 247u8, 57u8, 47u8, 38u8,
						],
					)
				}
				#[doc = " The current block number being processed. Set by `execute_block`."]
				pub fn number(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"Number",
						vec![],
						[
							30u8, 194u8, 177u8, 90u8, 194u8, 232u8, 46u8, 180u8, 85u8, 129u8, 14u8,
							9u8, 8u8, 8u8, 23u8, 95u8, 230u8, 5u8, 13u8, 105u8, 125u8, 2u8, 22u8,
							200u8, 78u8, 93u8, 115u8, 28u8, 150u8, 113u8, 48u8, 53u8,
						],
					)
				}
				#[doc = " Hash of the previous block."]
				pub fn parent_hash(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::H256,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"ParentHash",
						vec![],
						[
							26u8, 130u8, 11u8, 216u8, 155u8, 71u8, 128u8, 170u8, 30u8, 153u8, 21u8,
							192u8, 62u8, 93u8, 137u8, 80u8, 120u8, 81u8, 202u8, 94u8, 248u8, 125u8,
							71u8, 82u8, 141u8, 229u8, 32u8, 56u8, 73u8, 50u8, 101u8, 78u8,
						],
					)
				}
				#[doc = " Digest of the current block, also part of the block header."]
				pub fn digest(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::sp_runtime::generic::digest::Digest,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"Digest",
						vec![],
						[
							61u8, 64u8, 237u8, 91u8, 145u8, 232u8, 17u8, 254u8, 181u8, 16u8, 234u8,
							91u8, 51u8, 140u8, 254u8, 131u8, 98u8, 135u8, 21u8, 37u8, 251u8, 20u8,
							58u8, 92u8, 123u8, 141u8, 14u8, 227u8, 146u8, 46u8, 222u8, 117u8,
						],
					)
				}
				#[doc = " Events deposited for the current block."]
				#[doc = ""]
				#[doc = " NOTE: The item is unbound and should therefore never be read on chain."]
				#[doc = " It could otherwise inflate the PoV size of a block."]
				#[doc = ""]
				#[doc = " Events have a large in-memory size. Box the events to not go out-of-memory"]
				#[doc = " just in case someone still reads them from within the runtime."]
				pub fn events(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<
						runtime_types::frame_system::EventRecord<
							runtime_types::ulx_node_runtime::RuntimeEvent,
							::subxt::utils::H256,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"Events",
						vec![],
						[
							26u8, 19u8, 171u8, 93u8, 64u8, 164u8, 147u8, 177u8, 78u8, 200u8, 166u8,
							71u8, 131u8, 248u8, 124u8, 236u8, 108u8, 50u8, 113u8, 36u8, 7u8, 123u8,
							83u8, 80u8, 98u8, 232u8, 21u8, 23u8, 202u8, 93u8, 151u8, 133u8,
						],
					)
				}
				#[doc = " The number of events in the `Events<T>` list."]
				pub fn event_count(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"EventCount",
						vec![],
						[
							175u8, 24u8, 252u8, 184u8, 210u8, 167u8, 146u8, 143u8, 164u8, 80u8,
							151u8, 205u8, 189u8, 189u8, 55u8, 220u8, 47u8, 101u8, 181u8, 33u8,
							254u8, 131u8, 13u8, 143u8, 3u8, 244u8, 245u8, 45u8, 2u8, 210u8, 79u8,
							133u8,
						],
					)
				}
				#[doc = " Mapping between a topic (represented by T::Hash) and a vector of indexes"]
				#[doc = " of events in the `<Events<T>>` list."]
				#[doc = ""]
				#[doc = " All topic vectors have deterministic storage locations depending on the topic. This"]
				#[doc = " allows light-clients to leverage the changes trie storage tracking mechanism and"]
				#[doc = " in case of changes fetch the list of events of interest."]
				#[doc = ""]
				#[doc = " The value has the type `(BlockNumberFor<T>, EventIndex)` because if we used only just"]
				#[doc = " the `EventIndex` then in case if the topic has the same contents on the next block"]
				#[doc = " no notification will be triggered thus the event might be lost."]
				pub fn event_topics(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::H256>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<(::core::primitive::u32, ::core::primitive::u32)>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"EventTopics",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							40u8, 225u8, 14u8, 75u8, 44u8, 176u8, 76u8, 34u8, 143u8, 107u8, 69u8,
							133u8, 114u8, 13u8, 172u8, 250u8, 141u8, 73u8, 12u8, 65u8, 217u8, 63u8,
							120u8, 241u8, 48u8, 106u8, 143u8, 161u8, 128u8, 100u8, 166u8, 59u8,
						],
					)
				}
				#[doc = " Mapping between a topic (represented by T::Hash) and a vector of indexes"]
				#[doc = " of events in the `<Events<T>>` list."]
				#[doc = ""]
				#[doc = " All topic vectors have deterministic storage locations depending on the topic. This"]
				#[doc = " allows light-clients to leverage the changes trie storage tracking mechanism and"]
				#[doc = " in case of changes fetch the list of events of interest."]
				#[doc = ""]
				#[doc = " The value has the type `(BlockNumberFor<T>, EventIndex)` because if we used only just"]
				#[doc = " the `EventIndex` then in case if the topic has the same contents on the next block"]
				#[doc = " no notification will be triggered thus the event might be lost."]
				pub fn event_topics_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<(::core::primitive::u32, ::core::primitive::u32)>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"EventTopics",
						Vec::new(),
						[
							40u8, 225u8, 14u8, 75u8, 44u8, 176u8, 76u8, 34u8, 143u8, 107u8, 69u8,
							133u8, 114u8, 13u8, 172u8, 250u8, 141u8, 73u8, 12u8, 65u8, 217u8, 63u8,
							120u8, 241u8, 48u8, 106u8, 143u8, 161u8, 128u8, 100u8, 166u8, 59u8,
						],
					)
				}
				#[doc = " Stores the `spec_version` and `spec_name` of when the last runtime upgrade happened."]
				pub fn last_runtime_upgrade(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::frame_system::LastRuntimeUpgradeInfo,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"LastRuntimeUpgrade",
						vec![],
						[
							137u8, 29u8, 175u8, 75u8, 197u8, 208u8, 91u8, 207u8, 156u8, 87u8,
							148u8, 68u8, 91u8, 140u8, 22u8, 233u8, 1u8, 229u8, 56u8, 34u8, 40u8,
							194u8, 253u8, 30u8, 163u8, 39u8, 54u8, 209u8, 13u8, 27u8, 139u8, 184u8,
						],
					)
				}
				#[doc = " True if we have upgraded so that `type RefCount` is `u32`. False (default) if not."]
				pub fn upgraded_to_u32_ref_count(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"UpgradedToU32RefCount",
						vec![],
						[
							229u8, 73u8, 9u8, 132u8, 186u8, 116u8, 151u8, 171u8, 145u8, 29u8, 34u8,
							130u8, 52u8, 146u8, 124u8, 175u8, 79u8, 189u8, 147u8, 230u8, 234u8,
							107u8, 124u8, 31u8, 2u8, 22u8, 86u8, 190u8, 4u8, 147u8, 50u8, 245u8,
						],
					)
				}
				#[doc = " True if we have upgraded so that AccountInfo contains three types of `RefCount`. False"]
				#[doc = " (default) if not."]
				pub fn upgraded_to_triple_ref_count(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"UpgradedToTripleRefCount",
						vec![],
						[
							97u8, 66u8, 124u8, 243u8, 27u8, 167u8, 147u8, 81u8, 254u8, 201u8,
							101u8, 24u8, 40u8, 231u8, 14u8, 179u8, 154u8, 163u8, 71u8, 81u8, 185u8,
							167u8, 82u8, 254u8, 189u8, 3u8, 101u8, 207u8, 206u8, 194u8, 155u8,
							151u8,
						],
					)
				}
				#[doc = " The execution phase of the block."]
				pub fn execution_phase(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::frame_system::Phase,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"ExecutionPhase",
						vec![],
						[
							191u8, 129u8, 100u8, 134u8, 126u8, 116u8, 154u8, 203u8, 220u8, 200u8,
							0u8, 26u8, 161u8, 250u8, 133u8, 205u8, 146u8, 24u8, 5u8, 156u8, 158u8,
							35u8, 36u8, 253u8, 52u8, 235u8, 86u8, 167u8, 35u8, 100u8, 119u8, 27u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " Block & extrinsics weights: base values and limits."]
				pub fn block_weights(
					&self,
				) -> ::subxt::constants::Address<runtime_types::frame_system::limits::BlockWeights>
				{
					::subxt::constants::Address::new_static(
						"System",
						"BlockWeights",
						[
							176u8, 124u8, 225u8, 136u8, 25u8, 73u8, 247u8, 33u8, 82u8, 206u8, 85u8,
							190u8, 127u8, 102u8, 71u8, 11u8, 185u8, 8u8, 58u8, 0u8, 94u8, 55u8,
							163u8, 177u8, 104u8, 59u8, 60u8, 136u8, 246u8, 116u8, 0u8, 239u8,
						],
					)
				}
				#[doc = " The maximum length of a block (in bytes)."]
				pub fn block_length(
					&self,
				) -> ::subxt::constants::Address<runtime_types::frame_system::limits::BlockLength> {
					::subxt::constants::Address::new_static(
						"System",
						"BlockLength",
						[
							23u8, 242u8, 225u8, 39u8, 225u8, 67u8, 152u8, 41u8, 155u8, 104u8, 68u8,
							229u8, 185u8, 133u8, 10u8, 143u8, 184u8, 152u8, 234u8, 44u8, 140u8,
							96u8, 166u8, 235u8, 162u8, 160u8, 72u8, 7u8, 35u8, 194u8, 3u8, 37u8,
						],
					)
				}
				#[doc = " Maximum number of block number to block hash mappings to keep (oldest pruned first)."]
				pub fn block_hash_count(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"System",
						"BlockHashCount",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The weight of runtime database operations the runtime can invoke."]
				pub fn db_weight(
					&self,
				) -> ::subxt::constants::Address<runtime_types::sp_weights::RuntimeDbWeight> {
					::subxt::constants::Address::new_static(
						"System",
						"DbWeight",
						[
							42u8, 43u8, 178u8, 142u8, 243u8, 203u8, 60u8, 173u8, 118u8, 111u8,
							200u8, 170u8, 102u8, 70u8, 237u8, 187u8, 198u8, 120u8, 153u8, 232u8,
							183u8, 76u8, 74u8, 10u8, 70u8, 243u8, 14u8, 218u8, 213u8, 126u8, 29u8,
							177u8,
						],
					)
				}
				#[doc = " Get the chain's current version."]
				pub fn version(
					&self,
				) -> ::subxt::constants::Address<runtime_types::sp_version::RuntimeVersion> {
					::subxt::constants::Address::new_static(
						"System",
						"Version",
						[
							219u8, 45u8, 162u8, 245u8, 177u8, 246u8, 48u8, 126u8, 191u8, 157u8,
							228u8, 83u8, 111u8, 133u8, 183u8, 13u8, 148u8, 108u8, 92u8, 102u8,
							72u8, 205u8, 74u8, 242u8, 233u8, 79u8, 20u8, 170u8, 72u8, 202u8, 158u8,
							165u8,
						],
					)
				}
				#[doc = " The designated SS58 prefix of this chain."]
				#[doc = ""]
				#[doc = " This replaces the \"ss58Format\" property declared in the chain spec. Reason is"]
				#[doc = " that the runtime should know about the prefix in order to make use of it as"]
				#[doc = " an identifier of the chain."]
				pub fn ss58_prefix(&self) -> ::subxt::constants::Address<::core::primitive::u16> {
					::subxt::constants::Address::new_static(
						"System",
						"SS58Prefix",
						[
							116u8, 33u8, 2u8, 170u8, 181u8, 147u8, 171u8, 169u8, 167u8, 227u8,
							41u8, 144u8, 11u8, 236u8, 82u8, 100u8, 74u8, 60u8, 184u8, 72u8, 169u8,
							90u8, 208u8, 135u8, 15u8, 117u8, 10u8, 123u8, 128u8, 193u8, 29u8, 70u8,
						],
					)
				}
			}
		}
	}
	pub mod timestamp {
		use super::{root_mod, runtime_types};
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_timestamp::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Set {
					#[codec(compact)]
					pub now: ::core::primitive::u64,
				}
				impl ::subxt::blocks::StaticExtrinsic for Set {
					const PALLET: &'static str = "Timestamp";
					const CALL: &'static str = "set";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::set`]."]
				pub fn set(&self, now: ::core::primitive::u64) -> ::subxt::tx::Payload<types::Set> {
					::subxt::tx::Payload::new_static(
						"Timestamp",
						"set",
						types::Set { now },
						[
							37u8, 95u8, 49u8, 218u8, 24u8, 22u8, 0u8, 95u8, 72u8, 35u8, 155u8,
							199u8, 213u8, 54u8, 207u8, 22u8, 185u8, 193u8, 221u8, 70u8, 18u8,
							200u8, 4u8, 231u8, 195u8, 173u8, 6u8, 122u8, 11u8, 203u8, 231u8, 227u8,
						],
					)
				}
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The current time for the current block."]
				pub fn now(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Timestamp",
						"Now",
						vec![],
						[
							44u8, 50u8, 80u8, 30u8, 195u8, 146u8, 123u8, 238u8, 8u8, 163u8, 187u8,
							92u8, 61u8, 39u8, 51u8, 29u8, 173u8, 169u8, 217u8, 158u8, 85u8, 187u8,
							141u8, 26u8, 12u8, 115u8, 51u8, 11u8, 200u8, 244u8, 138u8, 152u8,
						],
					)
				}
				#[doc = " Whether the timestamp has been updated in this block."]
				#[doc = ""]
				#[doc = " This value is updated to `true` upon successful submission of a timestamp by a node."]
				#[doc = " It is then checked at the end of each block execution in the `on_finalize` hook."]
				pub fn did_update(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Timestamp",
						"DidUpdate",
						vec![],
						[
							229u8, 175u8, 246u8, 102u8, 237u8, 158u8, 212u8, 229u8, 238u8, 214u8,
							205u8, 160u8, 164u8, 252u8, 195u8, 75u8, 139u8, 110u8, 22u8, 34u8,
							248u8, 204u8, 107u8, 46u8, 20u8, 200u8, 238u8, 167u8, 71u8, 41u8,
							214u8, 140u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The minimum period between blocks."]
				#[doc = ""]
				#[doc = " Be aware that this is different to the *expected* period that the block production"]
				#[doc = " apparatus provides. Your chosen consensus system will generally work with this to"]
				#[doc = " determine a sensible block time. For example, in the Aura pallet it will be double this"]
				#[doc = " period on default settings."]
				pub fn minimum_period(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u64> {
					::subxt::constants::Address::new_static(
						"Timestamp",
						"MinimumPeriod",
						[
							128u8, 214u8, 205u8, 242u8, 181u8, 142u8, 124u8, 231u8, 190u8, 146u8,
							59u8, 226u8, 157u8, 101u8, 103u8, 117u8, 249u8, 65u8, 18u8, 191u8,
							103u8, 119u8, 53u8, 85u8, 81u8, 96u8, 220u8, 42u8, 184u8, 239u8, 42u8,
							246u8,
						],
					)
				}
			}
		}
	}
	pub mod ticks {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_ticks::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_ticks::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
			}
			pub struct TransactionApi;
			impl TransactionApi {}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				pub fn current_tick(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Ticks",
						"CurrentTick",
						vec![],
						[
							22u8, 7u8, 231u8, 159u8, 250u8, 169u8, 243u8, 224u8, 215u8, 82u8, 83u8,
							88u8, 83u8, 90u8, 150u8, 36u8, 157u8, 90u8, 223u8, 33u8, 128u8, 179u8,
							239u8, 41u8, 14u8, 89u8, 96u8, 146u8, 165u8, 37u8, 38u8, 232u8,
						],
					)
				}
				pub fn tick_duration(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Ticks",
						"TickDuration",
						vec![],
						[
							55u8, 52u8, 212u8, 174u8, 70u8, 95u8, 106u8, 159u8, 188u8, 126u8,
							123u8, 167u8, 200u8, 162u8, 123u8, 151u8, 208u8, 141u8, 238u8, 30u8,
							2u8, 249u8, 59u8, 144u8, 88u8, 239u8, 82u8, 32u8, 171u8, 142u8, 241u8,
							130u8,
						],
					)
				}
				pub fn genesis_tick_utc_timestamp(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Ticks",
						"GenesisTickUtcTimestamp",
						vec![],
						[
							237u8, 236u8, 104u8, 247u8, 108u8, 221u8, 147u8, 133u8, 46u8, 84u8,
							173u8, 103u8, 141u8, 162u8, 59u8, 108u8, 39u8, 245u8, 68u8, 84u8,
							216u8, 141u8, 150u8, 23u8, 36u8, 174u8, 131u8, 175u8, 249u8, 139u8,
							213u8, 248u8,
						],
					)
				}
			}
		}
	}
	pub mod mining_slot {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_mining_slot::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_mining_slot::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Bid {
					pub peer_id: runtime_types::sp_core::OpaquePeerId,
					pub rpc_hosts: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::ulx_primitives::block_seal::Host,
					>,
					pub bond_id: ::core::option::Option<::core::primitive::u64>,
					pub reward_destination:
						runtime_types::ulx_primitives::block_seal::RewardDestination<
							::subxt::utils::AccountId32,
						>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Bid {
					const PALLET: &'static str = "MiningSlot";
					const CALL: &'static str = "bid";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See `Pallet::bid`."]
				pub fn bid(
					&self,
					peer_id: runtime_types::sp_core::OpaquePeerId,
					rpc_hosts: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::ulx_primitives::block_seal::Host,
					>,
					bond_id: ::core::option::Option<::core::primitive::u64>,
					reward_destination : runtime_types :: ulx_primitives :: block_seal :: RewardDestination < :: subxt :: utils :: AccountId32 >,
				) -> ::subxt::tx::Payload<types::Bid> {
					::subxt::tx::Payload::new_static(
						"MiningSlot",
						"bid",
						types::Bid { peer_id, rpc_hosts, bond_id, reward_destination },
						[
							57u8, 45u8, 204u8, 242u8, 222u8, 170u8, 73u8, 175u8, 121u8, 81u8,
							107u8, 234u8, 180u8, 58u8, 195u8, 30u8, 183u8, 107u8, 232u8, 139u8,
							18u8, 131u8, 13u8, 24u8, 81u8, 82u8, 42u8, 150u8, 110u8, 16u8, 144u8,
							66u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_mining_slot::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct NewMiners {
				pub start_index: ::core::primitive::u32,
				pub new_miners: runtime_types::bounded_collections::bounded_vec::BoundedVec<
					runtime_types::ulx_primitives::block_seal::MiningRegistration<
						::subxt::utils::AccountId32,
						::core::primitive::u64,
						::core::primitive::u128,
					>,
				>,
			}
			impl ::subxt::events::StaticEvent for NewMiners {
				const PALLET: &'static str = "MiningSlot";
				const EVENT: &'static str = "NewMiners";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct SlotBidderAdded {
				pub account_id: ::subxt::utils::AccountId32,
				pub bid_amount: ::core::primitive::u128,
				pub index: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for SlotBidderAdded {
				const PALLET: &'static str = "MiningSlot";
				const EVENT: &'static str = "SlotBidderAdded";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct SlotBidderReplaced {
				pub account_id: ::subxt::utils::AccountId32,
				pub bond_id: ::core::option::Option<::core::primitive::u64>,
				pub kept_ownership_bond: ::core::primitive::bool,
			}
			impl ::subxt::events::StaticEvent for SlotBidderReplaced {
				const PALLET: &'static str = "MiningSlot";
				const EVENT: &'static str = "SlotBidderReplaced";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct UnbondedMiner {
				pub account_id: ::subxt::utils::AccountId32,
				pub bond_id: ::core::option::Option<::core::primitive::u64>,
				pub kept_ownership_bond: ::core::primitive::bool,
			}
			impl ::subxt::events::StaticEvent for UnbondedMiner {
				const PALLET: &'static str = "MiningSlot";
				const EVENT: &'static str = "UnbondedMiner";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Miners that are active in the current block (post initialize)"]
				pub fn active_miners_by_index(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_primitives::block_seal::MiningRegistration<
						::subxt::utils::AccountId32,
						::core::primitive::u64,
						::core::primitive::u128,
					>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"ActiveMinersByIndex",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							21u8, 133u8, 169u8, 134u8, 154u8, 130u8, 5u8, 39u8, 239u8, 192u8, 25u8,
							174u8, 42u8, 87u8, 19u8, 177u8, 12u8, 223u8, 85u8, 177u8, 239u8, 238u8,
							32u8, 52u8, 21u8, 208u8, 255u8, 6u8, 255u8, 172u8, 112u8, 27u8,
						],
					)
				}
				#[doc = " Miners that are active in the current block (post initialize)"]
				pub fn active_miners_by_index_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_primitives::block_seal::MiningRegistration<
						::subxt::utils::AccountId32,
						::core::primitive::u64,
						::core::primitive::u128,
					>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"ActiveMinersByIndex",
						Vec::new(),
						[
							21u8, 133u8, 169u8, 134u8, 154u8, 130u8, 5u8, 39u8, 239u8, 192u8, 25u8,
							174u8, 42u8, 87u8, 19u8, 177u8, 12u8, 223u8, 85u8, 177u8, 239u8, 238u8,
							32u8, 52u8, 21u8, 208u8, 255u8, 6u8, 255u8, 172u8, 112u8, 27u8,
						],
					)
				}
				pub fn active_miners_count(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u16,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"ActiveMinersCount",
						vec![],
						[
							20u8, 227u8, 98u8, 196u8, 103u8, 165u8, 188u8, 15u8, 246u8, 164u8,
							26u8, 233u8, 247u8, 78u8, 25u8, 118u8, 152u8, 76u8, 206u8, 87u8, 147u8,
							226u8, 101u8, 252u8, 77u8, 171u8, 75u8, 4u8, 74u8, 30u8, 72u8, 214u8,
						],
					)
				}
				#[doc = " Authorities are the session keys that are actively participating in the network."]
				#[doc = " The tuple is the authority, and the blake2 256 hash of the authority used for xor lookups"]
				pub fn authorities_by_index(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_btree_map::BoundedBTreeMap<
						::core::primitive::u32,
						(
							runtime_types::ulx_primitives::block_seal::app::Public,
							runtime_types::primitive_types::U256,
						),
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"AuthoritiesByIndex",
						vec![],
						[
							44u8, 145u8, 62u8, 168u8, 74u8, 162u8, 56u8, 32u8, 145u8, 183u8, 228u8,
							230u8, 148u8, 129u8, 172u8, 60u8, 187u8, 45u8, 251u8, 221u8, 251u8,
							209u8, 140u8, 97u8, 195u8, 91u8, 6u8, 230u8, 242u8, 200u8, 154u8, 69u8,
						],
					)
				}
				#[doc = " Tokens that must be bonded to take a Miner role"]
				pub fn ownership_bond_amount(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u128,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"OwnershipBondAmount",
						vec![],
						[
							45u8, 244u8, 244u8, 223u8, 190u8, 125u8, 131u8, 55u8, 69u8, 254u8,
							146u8, 237u8, 64u8, 61u8, 35u8, 114u8, 66u8, 95u8, 137u8, 138u8, 50u8,
							128u8, 217u8, 131u8, 10u8, 243u8, 1u8, 238u8, 208u8, 214u8, 106u8,
							235u8,
						],
					)
				}
				#[doc = " Lookup by account id to the corresponding index in ActiveMinersByIndex and Authorities"]
				pub fn account_index_lookup(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"AccountIndexLookup",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							203u8, 195u8, 115u8, 185u8, 125u8, 36u8, 9u8, 40u8, 68u8, 9u8, 52u8,
							60u8, 181u8, 139u8, 145u8, 41u8, 100u8, 62u8, 237u8, 172u8, 108u8,
							227u8, 106u8, 161u8, 59u8, 110u8, 244u8, 142u8, 80u8, 147u8, 188u8,
							190u8,
						],
					)
				}
				#[doc = " Lookup by account id to the corresponding index in ActiveMinersByIndex and Authorities"]
				pub fn account_index_lookup_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"AccountIndexLookup",
						Vec::new(),
						[
							203u8, 195u8, 115u8, 185u8, 125u8, 36u8, 9u8, 40u8, 68u8, 9u8, 52u8,
							60u8, 181u8, 139u8, 145u8, 41u8, 100u8, 62u8, 237u8, 172u8, 108u8,
							227u8, 106u8, 161u8, 59u8, 110u8, 244u8, 142u8, 80u8, 147u8, 188u8,
							190u8,
						],
					)
				}
				#[doc = " The cohort set to go into effect in the next slot. The Vec has all"]
				#[doc = " registrants with their bid amount"]
				pub fn next_slot_cohort(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::ulx_primitives::block_seal::MiningRegistration<
							::subxt::utils::AccountId32,
							::core::primitive::u64,
							::core::primitive::u128,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"NextSlotCohort",
						vec![],
						[
							220u8, 82u8, 73u8, 147u8, 145u8, 36u8, 171u8, 100u8, 123u8, 72u8,
							159u8, 110u8, 90u8, 168u8, 202u8, 11u8, 192u8, 211u8, 61u8, 5u8, 56u8,
							246u8, 208u8, 34u8, 238u8, 100u8, 49u8, 75u8, 182u8, 59u8, 53u8, 27u8,
						],
					)
				}
				#[doc = " Is the next slot still open for bids"]
				pub fn is_next_slot_bidding_open(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"IsNextSlotBiddingOpen",
						vec![],
						[
							55u8, 45u8, 220u8, 192u8, 255u8, 253u8, 247u8, 75u8, 156u8, 4u8, 133u8,
							132u8, 109u8, 242u8, 64u8, 251u8, 149u8, 180u8, 69u8, 54u8, 150u8, 3u8,
							249u8, 2u8, 167u8, 148u8, 133u8, 221u8, 27u8, 227u8, 85u8, 32u8,
						],
					)
				}
				#[doc = " The configuration for a miner to supply if there are no registered miners"]
				pub fn miner_zero(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_primitives::block_seal::MiningRegistration<
						::subxt::utils::AccountId32,
						::core::primitive::u64,
						::core::primitive::u128,
					>,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"MinerZero",
						vec![],
						[
							130u8, 202u8, 88u8, 111u8, 194u8, 214u8, 98u8, 153u8, 119u8, 42u8,
							248u8, 92u8, 209u8, 224u8, 131u8, 169u8, 251u8, 108u8, 209u8, 188u8,
							55u8, 125u8, 53u8, 134u8, 109u8, 6u8, 220u8, 122u8, 83u8, 24u8, 110u8,
							194u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The maximum number of Miners that the pallet can hold."]
				pub fn max_miners(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"MiningSlot",
						"MaxMiners",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " How many new miners can be in the cohort for each slot"]
				pub fn max_cohort_size(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"MiningSlot",
						"MaxCohortSize",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " How many blocks transpire between slots"]
				pub fn blocks_between_slots(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"MiningSlot",
						"BlocksBetweenSlots",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " How many session indexes to keep session history"]
				pub fn session_indices_to_keep_in_history(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"MiningSlot",
						"SessionIndicesToKeepInHistory",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " How many blocks buffer shall we use to stop accepting bids for the next period"]
				pub fn blocks_buffer_to_stop_accepting_bids(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"MiningSlot",
						"BlocksBufferToStopAcceptingBids",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The reduction in percent of ownership currency required to secure a slot"]
				pub fn ownership_percent_damper(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"MiningSlot",
						"OwnershipPercentDamper",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
			}
		}
	}
	pub mod bond {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_bond::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_bond::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct OfferFund {
					#[codec(compact)]
					pub lease_annual_percent_rate: ::core::primitive::u32,
					#[codec(compact)]
					pub lease_base_fee: ::core::primitive::u128,
					#[codec(compact)]
					pub amount_offered: ::core::primitive::u128,
					pub expiration_block: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for OfferFund {
					const PALLET: &'static str = "Bond";
					const CALL: &'static str = "offer_fund";
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct EndFund {
					pub bond_fund_id: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for EndFund {
					const PALLET: &'static str = "Bond";
					const CALL: &'static str = "end_fund";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ExtendFund {
					pub bond_fund_id: ::core::primitive::u32,
					pub total_amount_offered: ::core::primitive::u128,
					pub expiration_block: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for ExtendFund {
					const PALLET: &'static str = "Bond";
					const CALL: &'static str = "extend_fund";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BondSelf {
					pub amount: ::core::primitive::u128,
					pub bond_until_block: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for BondSelf {
					const PALLET: &'static str = "Bond";
					const CALL: &'static str = "bond_self";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Lease {
					pub bond_fund_id: ::core::primitive::u32,
					pub amount: ::core::primitive::u128,
					pub lease_until_block: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for Lease {
					const PALLET: &'static str = "Bond";
					const CALL: &'static str = "lease";
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ReturnBond {
					pub bond_id: ::core::primitive::u64,
				}
				impl ::subxt::blocks::StaticExtrinsic for ReturnBond {
					const PALLET: &'static str = "Bond";
					const CALL: &'static str = "return_bond";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ExtendBond {
					pub bond_id: ::core::primitive::u64,
					pub total_amount: ::core::primitive::u128,
					pub bond_until_block: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for ExtendBond {
					const PALLET: &'static str = "Bond";
					const CALL: &'static str = "extend_bond";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See `Pallet::offer_fund`."]
				pub fn offer_fund(
					&self,
					lease_annual_percent_rate: ::core::primitive::u32,
					lease_base_fee: ::core::primitive::u128,
					amount_offered: ::core::primitive::u128,
					expiration_block: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::OfferFund> {
					::subxt::tx::Payload::new_static(
						"Bond",
						"offer_fund",
						types::OfferFund {
							lease_annual_percent_rate,
							lease_base_fee,
							amount_offered,
							expiration_block,
						},
						[
							125u8, 9u8, 52u8, 63u8, 83u8, 189u8, 114u8, 102u8, 223u8, 44u8, 206u8,
							249u8, 14u8, 217u8, 244u8, 238u8, 128u8, 0u8, 119u8, 53u8, 135u8,
							218u8, 23u8, 76u8, 125u8, 138u8, 135u8, 251u8, 145u8, 65u8, 75u8,
							144u8,
						],
					)
				}
				#[doc = "See `Pallet::end_fund`."]
				pub fn end_fund(
					&self,
					bond_fund_id: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::EndFund> {
					::subxt::tx::Payload::new_static(
						"Bond",
						"end_fund",
						types::EndFund { bond_fund_id },
						[
							131u8, 140u8, 43u8, 83u8, 124u8, 77u8, 95u8, 84u8, 250u8, 118u8, 21u8,
							237u8, 240u8, 20u8, 83u8, 167u8, 60u8, 191u8, 90u8, 241u8, 100u8, 39u8,
							7u8, 24u8, 79u8, 149u8, 182u8, 219u8, 240u8, 243u8, 3u8, 70u8,
						],
					)
				}
				#[doc = "See `Pallet::extend_fund`."]
				pub fn extend_fund(
					&self,
					bond_fund_id: ::core::primitive::u32,
					total_amount_offered: ::core::primitive::u128,
					expiration_block: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::ExtendFund> {
					::subxt::tx::Payload::new_static(
						"Bond",
						"extend_fund",
						types::ExtendFund { bond_fund_id, total_amount_offered, expiration_block },
						[
							8u8, 174u8, 241u8, 111u8, 94u8, 45u8, 104u8, 19u8, 98u8, 42u8, 125u8,
							136u8, 192u8, 84u8, 198u8, 19u8, 30u8, 59u8, 176u8, 154u8, 238u8, 99u8,
							2u8, 35u8, 31u8, 209u8, 248u8, 175u8, 110u8, 156u8, 139u8, 205u8,
						],
					)
				}
				#[doc = "See `Pallet::bond_self`."]
				pub fn bond_self(
					&self,
					amount: ::core::primitive::u128,
					bond_until_block: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::BondSelf> {
					::subxt::tx::Payload::new_static(
						"Bond",
						"bond_self",
						types::BondSelf { amount, bond_until_block },
						[
							162u8, 47u8, 161u8, 45u8, 175u8, 122u8, 229u8, 198u8, 84u8, 116u8,
							72u8, 15u8, 173u8, 186u8, 40u8, 5u8, 127u8, 189u8, 39u8, 40u8, 108u8,
							71u8, 18u8, 16u8, 21u8, 139u8, 135u8, 101u8, 152u8, 202u8, 180u8,
							160u8,
						],
					)
				}
				#[doc = "See `Pallet::lease`."]
				pub fn lease(
					&self,
					bond_fund_id: ::core::primitive::u32,
					amount: ::core::primitive::u128,
					lease_until_block: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::Lease> {
					::subxt::tx::Payload::new_static(
						"Bond",
						"lease",
						types::Lease { bond_fund_id, amount, lease_until_block },
						[
							12u8, 180u8, 215u8, 175u8, 212u8, 46u8, 23u8, 132u8, 251u8, 15u8,
							239u8, 80u8, 170u8, 146u8, 89u8, 25u8, 38u8, 219u8, 91u8, 243u8, 137u8,
							233u8, 134u8, 5u8, 100u8, 14u8, 89u8, 161u8, 162u8, 99u8, 40u8, 212u8,
						],
					)
				}
				#[doc = "See `Pallet::return_bond`."]
				pub fn return_bond(
					&self,
					bond_id: ::core::primitive::u64,
				) -> ::subxt::tx::Payload<types::ReturnBond> {
					::subxt::tx::Payload::new_static(
						"Bond",
						"return_bond",
						types::ReturnBond { bond_id },
						[
							19u8, 164u8, 101u8, 219u8, 211u8, 192u8, 51u8, 188u8, 23u8, 171u8,
							188u8, 55u8, 230u8, 117u8, 110u8, 223u8, 129u8, 12u8, 187u8, 178u8,
							152u8, 23u8, 120u8, 20u8, 147u8, 227u8, 96u8, 109u8, 45u8, 28u8, 225u8,
							111u8,
						],
					)
				}
				#[doc = "See `Pallet::extend_bond`."]
				pub fn extend_bond(
					&self,
					bond_id: ::core::primitive::u64,
					total_amount: ::core::primitive::u128,
					bond_until_block: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::ExtendBond> {
					::subxt::tx::Payload::new_static(
						"Bond",
						"extend_bond",
						types::ExtendBond { bond_id, total_amount, bond_until_block },
						[
							0u8, 53u8, 253u8, 106u8, 104u8, 68u8, 176u8, 190u8, 93u8, 28u8, 180u8,
							75u8, 95u8, 132u8, 138u8, 16u8, 88u8, 225u8, 149u8, 29u8, 185u8, 211u8,
							38u8, 49u8, 65u8, 14u8, 198u8, 200u8, 192u8, 109u8, 48u8, 145u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_bond::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BondFundOffered {
				pub bond_fund_id: ::core::primitive::u32,
				pub amount_offered: ::core::primitive::u128,
				pub expiration_block: ::core::primitive::u32,
				pub offer_account_id: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for BondFundOffered {
				const PALLET: &'static str = "Bond";
				const EVENT: &'static str = "BondFundOffered";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BondFundExtended {
				pub bond_fund_id: ::core::primitive::u32,
				pub amount_offered: ::core::primitive::u128,
				pub expiration_block: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for BondFundExtended {
				const PALLET: &'static str = "Bond";
				const EVENT: &'static str = "BondFundExtended";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BondFundEnded {
				pub bond_fund_id: ::core::primitive::u32,
				pub amount_still_bonded: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for BondFundEnded {
				const PALLET: &'static str = "Bond";
				const EVENT: &'static str = "BondFundEnded";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BondFundExpired {
				pub bond_fund_id: ::core::primitive::u32,
				pub offer_account_id: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for BondFundExpired {
				const PALLET: &'static str = "Bond";
				const EVENT: &'static str = "BondFundExpired";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BondedSelf {
				pub bond_id: ::core::primitive::u64,
				pub bonded_account_id: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
				pub completion_block: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for BondedSelf {
				const PALLET: &'static str = "Bond";
				const EVENT: &'static str = "BondedSelf";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BondLeased {
				pub bond_fund_id: ::core::primitive::u32,
				pub bond_id: ::core::primitive::u64,
				pub bonded_account_id: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
				pub total_fee: ::core::primitive::u128,
				pub annual_percent_rate: ::core::primitive::u32,
				pub completion_block: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for BondLeased {
				const PALLET: &'static str = "Bond";
				const EVENT: &'static str = "BondLeased";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BondExtended {
				pub bond_fund_id: ::core::option::Option<::core::primitive::u32>,
				pub bond_id: ::core::primitive::u64,
				pub amount: ::core::primitive::u128,
				pub completion_block: ::core::primitive::u32,
				pub fee_change: ::core::primitive::u128,
				pub annual_percent_rate: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for BondExtended {
				const PALLET: &'static str = "Bond";
				const EVENT: &'static str = "BondExtended";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BondCompleted {
				pub bond_fund_id: ::core::option::Option<::core::primitive::u32>,
				pub bond_id: ::core::primitive::u64,
			}
			impl ::subxt::events::StaticEvent for BondCompleted {
				const PALLET: &'static str = "Bond";
				const EVENT: &'static str = "BondCompleted";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BondFeeRefund {
				pub bond_fund_id: ::core::primitive::u32,
				pub bond_id: ::core::primitive::u64,
				pub bonded_account_id: ::subxt::utils::AccountId32,
				pub bond_fund_reduction_for_payment: ::core::primitive::u128,
				pub final_fee: ::core::primitive::u128,
				pub refund_amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for BondFeeRefund {
				const PALLET: &'static str = "Bond";
				const EVENT: &'static str = "BondFeeRefund";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BondLocked {
				pub bond_id: ::core::primitive::u64,
				pub bonded_account_id: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for BondLocked {
				const PALLET: &'static str = "Bond";
				const EVENT: &'static str = "BondLocked";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BondUnlocked {
				pub bond_id: ::core::primitive::u64,
				pub bonded_account_id: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for BondUnlocked {
				const PALLET: &'static str = "Bond";
				const EVENT: &'static str = "BondUnlocked";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				pub fn next_bond_id(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Bond",
						"NextBondId",
						vec![],
						[
							5u8, 229u8, 152u8, 112u8, 204u8, 211u8, 171u8, 9u8, 47u8, 162u8, 31u8,
							88u8, 78u8, 187u8, 161u8, 163u8, 70u8, 216u8, 229u8, 145u8, 188u8,
							250u8, 163u8, 102u8, 207u8, 195u8, 149u8, 21u8, 202u8, 216u8, 11u8,
							181u8,
						],
					)
				}
				pub fn next_bond_fund_id(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Bond",
						"NextBondFundId",
						vec![],
						[
							194u8, 59u8, 237u8, 245u8, 182u8, 7u8, 180u8, 225u8, 13u8, 94u8, 214u8,
							166u8, 215u8, 116u8, 117u8, 79u8, 103u8, 219u8, 89u8, 99u8, 37u8, 8u8,
							30u8, 160u8, 24u8, 48u8, 43u8, 81u8, 44u8, 178u8, 93u8, 46u8,
						],
					)
				}
				#[doc = " BondFunds by id"]
				pub fn bond_funds(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_primitives::bond::BondFund<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
						::core::primitive::u32,
					>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bond",
						"BondFunds",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							201u8, 175u8, 22u8, 184u8, 52u8, 18u8, 133u8, 107u8, 222u8, 91u8,
							177u8, 22u8, 45u8, 39u8, 73u8, 235u8, 89u8, 138u8, 41u8, 250u8, 31u8,
							245u8, 212u8, 105u8, 79u8, 147u8, 126u8, 235u8, 37u8, 21u8, 57u8, 58u8,
						],
					)
				}
				#[doc = " BondFunds by id"]
				pub fn bond_funds_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_primitives::bond::BondFund<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
						::core::primitive::u32,
					>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bond",
						"BondFunds",
						Vec::new(),
						[
							201u8, 175u8, 22u8, 184u8, 52u8, 18u8, 133u8, 107u8, 222u8, 91u8,
							177u8, 22u8, 45u8, 39u8, 73u8, 235u8, 89u8, 138u8, 41u8, 250u8, 31u8,
							245u8, 212u8, 105u8, 79u8, 147u8, 126u8, 235u8, 37u8, 21u8, 57u8, 58u8,
						],
					)
				}
				#[doc = " Expiration block number for each bond fund"]
				pub fn bond_fund_expirations(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u32,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bond",
						"BondFundExpirations",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							22u8, 162u8, 77u8, 110u8, 141u8, 47u8, 65u8, 85u8, 143u8, 125u8, 17u8,
							130u8, 155u8, 207u8, 222u8, 161u8, 192u8, 51u8, 172u8, 40u8, 239u8,
							126u8, 0u8, 56u8, 130u8, 215u8, 137u8, 234u8, 176u8, 12u8, 98u8, 110u8,
						],
					)
				}
				#[doc = " Expiration block number for each bond fund"]
				pub fn bond_fund_expirations_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u32,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bond",
						"BondFundExpirations",
						Vec::new(),
						[
							22u8, 162u8, 77u8, 110u8, 141u8, 47u8, 65u8, 85u8, 143u8, 125u8, 17u8,
							130u8, 155u8, 207u8, 222u8, 161u8, 192u8, 51u8, 172u8, 40u8, 239u8,
							126u8, 0u8, 56u8, 130u8, 215u8, 137u8, 234u8, 176u8, 12u8, 98u8, 110u8,
						],
					)
				}
				#[doc = " Bonds by id"]
				pub fn bonds(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u64>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_primitives::bond::Bond<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
						::core::primitive::u32,
						::core::primitive::u32,
					>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bond",
						"Bonds",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							44u8, 199u8, 6u8, 94u8, 193u8, 79u8, 161u8, 213u8, 219u8, 172u8, 86u8,
							10u8, 38u8, 27u8, 205u8, 190u8, 112u8, 215u8, 75u8, 30u8, 127u8, 147u8,
							74u8, 217u8, 101u8, 84u8, 16u8, 37u8, 46u8, 63u8, 99u8, 225u8,
						],
					)
				}
				#[doc = " Bonds by id"]
				pub fn bonds_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_primitives::bond::Bond<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
						::core::primitive::u32,
						::core::primitive::u32,
					>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bond",
						"Bonds",
						Vec::new(),
						[
							44u8, 199u8, 6u8, 94u8, 193u8, 79u8, 161u8, 213u8, 219u8, 172u8, 86u8,
							10u8, 38u8, 27u8, 205u8, 190u8, 112u8, 215u8, 75u8, 30u8, 127u8, 147u8,
							74u8, 217u8, 101u8, 84u8, 16u8, 37u8, 46u8, 63u8, 99u8, 225u8,
						],
					)
				}
				#[doc = " Completion of each bond, upon which date funds are returned to the bond fund or self-bonder"]
				pub fn bond_completions(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u64,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bond",
						"BondCompletions",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							90u8, 38u8, 131u8, 102u8, 147u8, 157u8, 59u8, 220u8, 177u8, 233u8,
							31u8, 230u8, 22u8, 137u8, 226u8, 105u8, 41u8, 12u8, 91u8, 40u8, 109u8,
							230u8, 195u8, 27u8, 115u8, 217u8, 17u8, 209u8, 135u8, 113u8, 9u8,
							126u8,
						],
					)
				}
				#[doc = " Completion of each bond, upon which date funds are returned to the bond fund or self-bonder"]
				pub fn bond_completions_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u64,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bond",
						"BondCompletions",
						Vec::new(),
						[
							90u8, 38u8, 131u8, 102u8, 147u8, 157u8, 59u8, 220u8, 177u8, 233u8,
							31u8, 230u8, 22u8, 137u8, 226u8, 105u8, 41u8, 12u8, 91u8, 40u8, 109u8,
							230u8, 195u8, 27u8, 115u8, 217u8, 17u8, 209u8, 135u8, 113u8, 9u8,
							126u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " Minimum amount for a bond"]
				pub fn minimum_bond_amount(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Bond",
						"MinimumBondAmount",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " Blocks per year used for APR calculations"]
				pub fn blocks_per_year(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Bond",
						"BlocksPerYear",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " Pallet storage requires bounds, so we have to set a maximum number that can expire in a"]
				#[doc = " single block"]
				pub fn max_concurrently_expiring_bond_funds(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Bond",
						"MaxConcurrentlyExpiringBondFunds",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " Pallet storage requires bounds, so we have to set a maximum number that can expire in a"]
				#[doc = " single block"]
				pub fn max_concurrently_expiring_bonds(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Bond",
						"MaxConcurrentlyExpiringBonds",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
			}
		}
	}
	pub mod notaries {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_notaries::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_notaries::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Propose {
					pub meta: runtime_types::ulx_primitives::notary::NotaryMeta,
				}
				impl ::subxt::blocks::StaticExtrinsic for Propose {
					const PALLET: &'static str = "Notaries";
					const CALL: &'static str = "propose";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Activate {
					pub operator_account: ::subxt::utils::AccountId32,
				}
				impl ::subxt::blocks::StaticExtrinsic for Activate {
					const PALLET: &'static str = "Notaries";
					const CALL: &'static str = "activate";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Update {
					#[codec(compact)]
					pub notary_id: ::core::primitive::u32,
					pub meta: runtime_types::ulx_primitives::notary::NotaryMeta,
				}
				impl ::subxt::blocks::StaticExtrinsic for Update {
					const PALLET: &'static str = "Notaries";
					const CALL: &'static str = "update";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See `Pallet::propose`."]
				pub fn propose(
					&self,
					meta: runtime_types::ulx_primitives::notary::NotaryMeta,
				) -> ::subxt::tx::Payload<types::Propose> {
					::subxt::tx::Payload::new_static(
						"Notaries",
						"propose",
						types::Propose { meta },
						[
							159u8, 154u8, 52u8, 133u8, 56u8, 190u8, 217u8, 33u8, 111u8, 9u8, 192u8,
							74u8, 29u8, 116u8, 14u8, 152u8, 116u8, 171u8, 61u8, 255u8, 218u8, 44u8,
							197u8, 222u8, 243u8, 52u8, 236u8, 68u8, 95u8, 230u8, 74u8, 220u8,
						],
					)
				}
				#[doc = "See `Pallet::activate`."]
				pub fn activate(
					&self,
					operator_account: ::subxt::utils::AccountId32,
				) -> ::subxt::tx::Payload<types::Activate> {
					::subxt::tx::Payload::new_static(
						"Notaries",
						"activate",
						types::Activate { operator_account },
						[
							135u8, 95u8, 212u8, 221u8, 190u8, 66u8, 114u8, 221u8, 200u8, 32u8,
							94u8, 119u8, 117u8, 207u8, 216u8, 78u8, 123u8, 114u8, 127u8, 0u8,
							209u8, 4u8, 39u8, 108u8, 188u8, 206u8, 192u8, 165u8, 17u8, 176u8, 0u8,
							203u8,
						],
					)
				}
				#[doc = "See `Pallet::update`."]
				pub fn update(
					&self,
					notary_id: ::core::primitive::u32,
					meta: runtime_types::ulx_primitives::notary::NotaryMeta,
				) -> ::subxt::tx::Payload<types::Update> {
					::subxt::tx::Payload::new_static(
						"Notaries",
						"update",
						types::Update { notary_id, meta },
						[
							248u8, 13u8, 108u8, 145u8, 50u8, 185u8, 155u8, 145u8, 134u8, 94u8,
							70u8, 202u8, 129u8, 23u8, 244u8, 8u8, 208u8, 137u8, 242u8, 230u8,
							212u8, 132u8, 148u8, 254u8, 63u8, 209u8, 88u8, 255u8, 95u8, 203u8,
							143u8, 18u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_notaries::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A user has proposed operating as a notary"]
			pub struct NotaryProposed {
				pub operator_account: ::subxt::utils::AccountId32,
				pub meta: runtime_types::ulx_primitives::notary::NotaryMeta,
				pub expires: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for NotaryProposed {
				const PALLET: &'static str = "Notaries";
				const EVENT: &'static str = "NotaryProposed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A notary proposal has been accepted"]
			pub struct NotaryActivated {
				pub notary: runtime_types::ulx_primitives::notary::NotaryRecord<
					::subxt::utils::AccountId32,
					::core::primitive::u32,
				>,
			}
			impl ::subxt::events::StaticEvent for NotaryActivated {
				const PALLET: &'static str = "Notaries";
				const EVENT: &'static str = "NotaryActivated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Notary metadata queued for update"]
			pub struct NotaryMetaUpdateQueued {
				pub notary_id: ::core::primitive::u32,
				pub meta: runtime_types::ulx_primitives::notary::NotaryMeta,
				pub effective_block: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for NotaryMetaUpdateQueued {
				const PALLET: &'static str = "Notaries";
				const EVENT: &'static str = "NotaryMetaUpdateQueued";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Notary metadata updated"]
			pub struct NotaryMetaUpdated {
				pub notary_id: ::core::primitive::u32,
				pub meta: runtime_types::ulx_primitives::notary::NotaryMeta,
			}
			impl ::subxt::events::StaticEvent for NotaryMetaUpdated {
				const PALLET: &'static str = "Notaries";
				const EVENT: &'static str = "NotaryMetaUpdated";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				pub fn next_notary_id(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"NextNotaryId",
						vec![],
						[
							246u8, 48u8, 149u8, 160u8, 181u8, 5u8, 135u8, 44u8, 164u8, 37u8, 82u8,
							255u8, 240u8, 24u8, 171u8, 176u8, 255u8, 52u8, 54u8, 210u8, 131u8,
							113u8, 102u8, 36u8, 241u8, 251u8, 53u8, 118u8, 13u8, 52u8, 230u8, 7u8,
						],
					)
				}
				pub fn proposed_notaries(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					(runtime_types::ulx_primitives::notary::NotaryMeta, ::core::primitive::u32),
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"ProposedNotaries",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							4u8, 53u8, 212u8, 112u8, 239u8, 86u8, 180u8, 223u8, 181u8, 166u8, 51u8,
							250u8, 170u8, 104u8, 23u8, 27u8, 204u8, 149u8, 202u8, 234u8, 219u8,
							254u8, 35u8, 72u8, 20u8, 145u8, 161u8, 83u8, 61u8, 106u8, 150u8, 157u8,
						],
					)
				}
				pub fn proposed_notaries_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					(runtime_types::ulx_primitives::notary::NotaryMeta, ::core::primitive::u32),
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"ProposedNotaries",
						Vec::new(),
						[
							4u8, 53u8, 212u8, 112u8, 239u8, 86u8, 180u8, 223u8, 181u8, 166u8, 51u8,
							250u8, 170u8, 104u8, 23u8, 27u8, 204u8, 149u8, 202u8, 234u8, 219u8,
							254u8, 35u8, 72u8, 20u8, 145u8, 161u8, 83u8, 61u8, 106u8, 150u8, 157u8,
						],
					)
				}
				pub fn expiring_proposals(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::subxt::utils::AccountId32,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"ExpiringProposals",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							64u8, 68u8, 247u8, 229u8, 147u8, 217u8, 204u8, 231u8, 82u8, 104u8,
							212u8, 163u8, 195u8, 244u8, 63u8, 148u8, 181u8, 120u8, 176u8, 52u8,
							125u8, 39u8, 74u8, 241u8, 126u8, 83u8, 45u8, 96u8, 30u8, 29u8, 155u8,
							108u8,
						],
					)
				}
				pub fn expiring_proposals_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::subxt::utils::AccountId32,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"ExpiringProposals",
						Vec::new(),
						[
							64u8, 68u8, 247u8, 229u8, 147u8, 217u8, 204u8, 231u8, 82u8, 104u8,
							212u8, 163u8, 195u8, 244u8, 63u8, 148u8, 181u8, 120u8, 176u8, 52u8,
							125u8, 39u8, 74u8, 241u8, 126u8, 83u8, 45u8, 96u8, 30u8, 29u8, 155u8,
							108u8,
						],
					)
				}
				pub fn active_notaries(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::ulx_primitives::notary::NotaryRecord<
							::subxt::utils::AccountId32,
							::core::primitive::u32,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"ActiveNotaries",
						vec![],
						[
							3u8, 215u8, 55u8, 52u8, 137u8, 40u8, 253u8, 206u8, 46u8, 63u8, 199u8,
							38u8, 90u8, 2u8, 249u8, 139u8, 228u8, 114u8, 158u8, 67u8, 63u8, 176u8,
							18u8, 242u8, 188u8, 74u8, 221u8, 40u8, 75u8, 228u8, 118u8, 138u8,
						],
					)
				}
				pub fn notary_key_history(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						::core::primitive::u32,
						runtime_types::sp_core::ed25519::Public,
					)>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"NotaryKeyHistory",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							71u8, 175u8, 193u8, 253u8, 234u8, 40u8, 253u8, 120u8, 166u8, 216u8,
							37u8, 202u8, 122u8, 54u8, 134u8, 79u8, 227u8, 135u8, 241u8, 51u8, 98u8,
							97u8, 176u8, 124u8, 49u8, 97u8, 68u8, 52u8, 66u8, 20u8, 197u8, 105u8,
						],
					)
				}
				pub fn notary_key_history_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						::core::primitive::u32,
						runtime_types::sp_core::ed25519::Public,
					)>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"NotaryKeyHistory",
						Vec::new(),
						[
							71u8, 175u8, 193u8, 253u8, 234u8, 40u8, 253u8, 120u8, 166u8, 216u8,
							37u8, 202u8, 122u8, 54u8, 134u8, 79u8, 227u8, 135u8, 241u8, 51u8, 98u8,
							97u8, 176u8, 124u8, 49u8, 97u8, 68u8, 52u8, 66u8, 20u8, 197u8, 105u8,
						],
					)
				}
				pub fn queued_notary_meta_changes(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_btree_map::BoundedBTreeMap<
						::core::primitive::u32,
						runtime_types::ulx_primitives::notary::NotaryMeta,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"QueuedNotaryMetaChanges",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							38u8, 141u8, 235u8, 204u8, 119u8, 37u8, 151u8, 26u8, 84u8, 219u8, 10u8,
							121u8, 194u8, 154u8, 189u8, 45u8, 54u8, 199u8, 122u8, 247u8, 199u8,
							248u8, 219u8, 77u8, 133u8, 230u8, 138u8, 12u8, 99u8, 162u8, 192u8,
							64u8,
						],
					)
				}
				pub fn queued_notary_meta_changes_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_btree_map::BoundedBTreeMap<
						::core::primitive::u32,
						runtime_types::ulx_primitives::notary::NotaryMeta,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"QueuedNotaryMetaChanges",
						Vec::new(),
						[
							38u8, 141u8, 235u8, 204u8, 119u8, 37u8, 151u8, 26u8, 84u8, 219u8, 10u8,
							121u8, 194u8, 154u8, 189u8, 45u8, 54u8, 199u8, 122u8, 247u8, 199u8,
							248u8, 219u8, 77u8, 133u8, 230u8, 138u8, 12u8, 99u8, 162u8, 192u8,
							64u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The maximum active notaries allowed"]
				pub fn max_active_notaries(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Notaries",
						"MaxActiveNotaries",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum blocks a proposal can sit unapproved"]
				pub fn max_proposal_hold_blocks(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Notaries",
						"MaxProposalHoldBlocks",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				pub fn max_proposals_per_block(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Notaries",
						"MaxProposalsPerBlock",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " Number of blocks to delay changing a notaries' meta"]
				pub fn meta_changes_block_delay(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Notaries",
						"MetaChangesBlockDelay",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " Number of blocks to maintain key history for each notary"]
				#[doc = " NOTE: only pruned when new keys are added"]
				pub fn max_blocks_for_key_history(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Notaries",
						"MaxBlocksForKeyHistory",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " Maximum hosts a notary can supply"]
				pub fn max_notary_hosts(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Notaries",
						"MaxNotaryHosts",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
			}
		}
	}
	pub mod notebook {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_notebook::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_notebook::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Submit {
					pub header: runtime_types::ulx_primitives::notebook::NotebookHeader,
					pub hash: ::subxt::utils::H256,
					pub signature: runtime_types::sp_core::ed25519::Signature,
				}
				impl ::subxt::blocks::StaticExtrinsic for Submit {
					const PALLET: &'static str = "Notebook";
					const CALL: &'static str = "submit";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See `Pallet::submit`."]
				pub fn submit(
					&self,
					header: runtime_types::ulx_primitives::notebook::NotebookHeader,
					hash: ::subxt::utils::H256,
					signature: runtime_types::sp_core::ed25519::Signature,
				) -> ::subxt::tx::Payload<types::Submit> {
					::subxt::tx::Payload::new_static(
						"Notebook",
						"submit",
						types::Submit { header, hash, signature },
						[
							18u8, 40u8, 201u8, 82u8, 83u8, 71u8, 194u8, 165u8, 56u8, 106u8, 221u8,
							102u8, 232u8, 18u8, 0u8, 46u8, 172u8, 76u8, 144u8, 130u8, 21u8, 89u8,
							110u8, 176u8, 254u8, 191u8, 82u8, 137u8, 221u8, 188u8, 82u8, 72u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_notebook::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct NotebookSubmitted {
				pub notary_id: ::core::primitive::u32,
				pub notebook_number: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for NotebookSubmitted {
				const PALLET: &'static str = "Notebook";
				const EVENT: &'static str = "NotebookSubmitted";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Double storage map of notary id + notebook # to the change root"]
				pub fn notebook_changed_accounts_root_by_notary(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
					_1: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::H256,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"NotebookChangedAccountsRootByNotary",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							84u8, 136u8, 124u8, 162u8, 187u8, 104u8, 116u8, 80u8, 119u8, 130u8,
							77u8, 8u8, 34u8, 154u8, 63u8, 59u8, 4u8, 169u8, 227u8, 231u8, 95u8,
							16u8, 2u8, 116u8, 193u8, 76u8, 174u8, 109u8, 254u8, 206u8, 159u8,
							109u8,
						],
					)
				}
				#[doc = " Double storage map of notary id + notebook # to the change root"]
				pub fn notebook_changed_accounts_root_by_notary_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::H256,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"NotebookChangedAccountsRootByNotary",
						Vec::new(),
						[
							84u8, 136u8, 124u8, 162u8, 187u8, 104u8, 116u8, 80u8, 119u8, 130u8,
							77u8, 8u8, 34u8, 154u8, 63u8, 59u8, 4u8, 169u8, 227u8, 231u8, 95u8,
							16u8, 2u8, 116u8, 193u8, 76u8, 174u8, 109u8, 254u8, 206u8, 159u8,
							109u8,
						],
					)
				}
				#[doc = " Storage map of account origin (notary_id, notebook, account_uid) to the last"]
				#[doc = " notebook containing this account in the changed accounts merkle root"]
				#[doc = " (NotebookChangedAccountsRootByNotary)"]
				pub fn account_origin_last_changed_notebook_by_notary(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
					_1: impl ::std::borrow::Borrow<
						runtime_types::ulx_primitives::balance_change::AccountOrigin,
					>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"AccountOriginLastChangedNotebookByNotary",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							233u8, 5u8, 227u8, 113u8, 187u8, 168u8, 114u8, 176u8, 38u8, 129u8,
							116u8, 70u8, 109u8, 153u8, 173u8, 216u8, 216u8, 105u8, 245u8, 249u8,
							164u8, 236u8, 233u8, 205u8, 156u8, 134u8, 105u8, 157u8, 196u8, 182u8,
							144u8, 213u8,
						],
					)
				}
				#[doc = " Storage map of account origin (notary_id, notebook, account_uid) to the last"]
				#[doc = " notebook containing this account in the changed accounts merkle root"]
				#[doc = " (NotebookChangedAccountsRootByNotary)"]
				pub fn account_origin_last_changed_notebook_by_notary_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"AccountOriginLastChangedNotebookByNotary",
						Vec::new(),
						[
							233u8, 5u8, 227u8, 113u8, 187u8, 168u8, 114u8, 176u8, 38u8, 129u8,
							116u8, 70u8, 109u8, 153u8, 173u8, 216u8, 216u8, 105u8, 245u8, 249u8,
							164u8, 236u8, 233u8, 205u8, 156u8, 134u8, 105u8, 157u8, 196u8, 182u8,
							144u8, 213u8,
						],
					)
				}
				#[doc = " List of last few notebook details by notary. The bool is whether the notebook was received"]
				#[doc = " in the appropriate \"tick\""]
				pub fn last_notebook_details_by_notary(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						runtime_types::ulx_primitives::notary::NotaryNotebookKeyDetails,
						::core::primitive::bool,
					)>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"LastNotebookDetailsByNotary",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							25u8, 149u8, 39u8, 159u8, 116u8, 139u8, 140u8, 29u8, 31u8, 189u8,
							216u8, 28u8, 214u8, 147u8, 72u8, 14u8, 76u8, 68u8, 166u8, 214u8, 166u8,
							122u8, 166u8, 169u8, 85u8, 202u8, 196u8, 241u8, 47u8, 84u8, 56u8,
							231u8,
						],
					)
				}
				#[doc = " List of last few notebook details by notary. The bool is whether the notebook was received"]
				#[doc = " in the appropriate \"tick\""]
				pub fn last_notebook_details_by_notary_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						runtime_types::ulx_primitives::notary::NotaryNotebookKeyDetails,
						::core::primitive::bool,
					)>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"LastNotebookDetailsByNotary",
						Vec::new(),
						[
							25u8, 149u8, 39u8, 159u8, 116u8, 139u8, 140u8, 29u8, 31u8, 189u8,
							216u8, 28u8, 214u8, 147u8, 72u8, 14u8, 76u8, 68u8, 166u8, 214u8, 166u8,
							122u8, 166u8, 169u8, 85u8, 202u8, 196u8, 241u8, 47u8, 84u8, 56u8,
							231u8,
						],
					)
				}
			}
		}
	}
	pub mod chain_transfer {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_chain_transfer::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_chain_transfer::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SendToLocalchain {
					#[codec(compact)]
					pub amount: ::core::primitive::u128,
					pub notary_id: ::core::primitive::u32,
					#[codec(compact)]
					pub account_nonce: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for SendToLocalchain {
					const PALLET: &'static str = "ChainTransfer";
					const CALL: &'static str = "send_to_localchain";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See `Pallet::send_to_localchain`."]
				pub fn send_to_localchain(
					&self,
					amount: ::core::primitive::u128,
					notary_id: ::core::primitive::u32,
					account_nonce: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::SendToLocalchain> {
					::subxt::tx::Payload::new_static(
						"ChainTransfer",
						"send_to_localchain",
						types::SendToLocalchain { amount, notary_id, account_nonce },
						[
							235u8, 60u8, 252u8, 71u8, 57u8, 103u8, 83u8, 70u8, 168u8, 14u8, 138u8,
							217u8, 144u8, 173u8, 93u8, 242u8, 97u8, 193u8, 176u8, 163u8, 124u8,
							162u8, 173u8, 250u8, 52u8, 242u8, 168u8, 25u8, 247u8, 208u8, 159u8,
							117u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_chain_transfer::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct TransferToLocalchain {
				pub account_id: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
				pub account_nonce: ::core::primitive::u32,
				pub notary_id: ::core::primitive::u32,
				pub expiration_block: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for TransferToLocalchain {
				const PALLET: &'static str = "ChainTransfer";
				const EVENT: &'static str = "TransferToLocalchain";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct TransferToLocalchainExpired {
				pub account_id: ::subxt::utils::AccountId32,
				pub account_nonce: ::core::primitive::u32,
				pub notary_id: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for TransferToLocalchainExpired {
				const PALLET: &'static str = "ChainTransfer";
				const EVENT: &'static str = "TransferToLocalchainExpired";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct TransferIn {
				pub account_id: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
				pub notary_id: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for TransferIn {
				const PALLET: &'static str = "ChainTransfer";
				const EVENT: &'static str = "TransferIn";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				pub fn pending_transfers_out(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
					_1: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_chain_transfer::QueuedTransferOut<
						::core::primitive::u128,
						::core::primitive::u32,
					>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"PendingTransfersOut",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							199u8, 145u8, 42u8, 111u8, 78u8, 179u8, 9u8, 117u8, 229u8, 120u8, 33u8,
							244u8, 159u8, 127u8, 196u8, 193u8, 210u8, 158u8, 252u8, 190u8, 79u8,
							111u8, 40u8, 234u8, 159u8, 59u8, 230u8, 96u8, 88u8, 91u8, 221u8, 251u8,
						],
					)
				}
				pub fn pending_transfers_out_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_chain_transfer::QueuedTransferOut<
						::core::primitive::u128,
						::core::primitive::u32,
					>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"PendingTransfersOut",
						Vec::new(),
						[
							199u8, 145u8, 42u8, 111u8, 78u8, 179u8, 9u8, 117u8, 229u8, 120u8, 33u8,
							244u8, 159u8, 127u8, 196u8, 193u8, 210u8, 158u8, 252u8, 190u8, 79u8,
							111u8, 40u8, 234u8, 159u8, 59u8, 230u8, 96u8, 88u8, 91u8, 221u8, 251u8,
						],
					)
				}
				pub fn expiring_transfers_out(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						::subxt::utils::AccountId32,
						::core::primitive::u32,
					)>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"ExpiringTransfersOut",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							2u8, 14u8, 179u8, 191u8, 84u8, 44u8, 30u8, 129u8, 244u8, 166u8, 193u8,
							61u8, 114u8, 176u8, 242u8, 179u8, 153u8, 198u8, 242u8, 204u8, 198u8,
							123u8, 156u8, 26u8, 65u8, 181u8, 236u8, 222u8, 226u8, 29u8, 179u8,
							99u8,
						],
					)
				}
				pub fn expiring_transfers_out_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						::subxt::utils::AccountId32,
						::core::primitive::u32,
					)>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"ExpiringTransfersOut",
						Vec::new(),
						[
							2u8, 14u8, 179u8, 191u8, 84u8, 44u8, 30u8, 129u8, 244u8, 166u8, 193u8,
							61u8, 114u8, 176u8, 242u8, 179u8, 153u8, 198u8, 242u8, 204u8, 198u8,
							123u8, 156u8, 26u8, 65u8, 181u8, 236u8, 222u8, 226u8, 29u8, 179u8,
							99u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				pub fn pallet_id(
					&self,
				) -> ::subxt::constants::Address<runtime_types::frame_support::PalletId> {
					::subxt::constants::Address::new_static(
						"ChainTransfer",
						"PalletId",
						[
							56u8, 243u8, 53u8, 83u8, 154u8, 179u8, 170u8, 80u8, 133u8, 173u8, 61u8,
							161u8, 47u8, 225u8, 146u8, 21u8, 50u8, 229u8, 248u8, 27u8, 104u8, 58u8,
							129u8, 197u8, 102u8, 160u8, 168u8, 205u8, 154u8, 42u8, 217u8, 53u8,
						],
					)
				}
				#[doc = " How long a transfer should remain in storage before returning."]
				pub fn transfer_expiration_blocks(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"ChainTransfer",
						"TransferExpirationBlocks",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " How many transfers out can be queued per block"]
				pub fn max_pending_transfers_out_per_block(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"ChainTransfer",
						"MaxPendingTransfersOutPerBlock",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
			}
		}
	}
	pub mod block_seal_spec {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_block_seal_spec::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_block_seal_spec::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Configure {
					pub vote_minimum: ::core::option::Option<::core::primitive::u128>,
					pub compute_difficulty: ::core::option::Option<::core::primitive::u128>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Configure {
					const PALLET: &'static str = "BlockSealSpec";
					const CALL: &'static str = "configure";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See `Pallet::configure`."]
				pub fn configure(
					&self,
					vote_minimum: ::core::option::Option<::core::primitive::u128>,
					compute_difficulty: ::core::option::Option<::core::primitive::u128>,
				) -> ::subxt::tx::Payload<types::Configure> {
					::subxt::tx::Payload::new_static(
						"BlockSealSpec",
						"configure",
						types::Configure { vote_minimum, compute_difficulty },
						[
							211u8, 110u8, 104u8, 239u8, 141u8, 253u8, 92u8, 219u8, 37u8, 63u8,
							59u8, 202u8, 96u8, 3u8, 132u8, 207u8, 219u8, 42u8, 253u8, 70u8, 152u8,
							29u8, 72u8, 104u8, 182u8, 58u8, 23u8, 133u8, 41u8, 223u8, 62u8, 28u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_block_seal_spec::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct VoteMinimumAdjusted {
				pub expected_block_votes: ::core::primitive::u128,
				pub actual_block_votes: ::core::primitive::u128,
				pub start_vote_minimum: ::core::primitive::u128,
				pub new_vote_minimum: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for VoteMinimumAdjusted {
				const PALLET: &'static str = "BlockSealSpec";
				const EVENT: &'static str = "VoteMinimumAdjusted";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct ComputeDifficultyAdjusted {
				pub expected_block_time: ::core::primitive::u64,
				pub actual_block_time: ::core::primitive::u64,
				pub start_difficulty: ::core::primitive::u128,
				pub new_difficulty: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for ComputeDifficultyAdjusted {
				const PALLET: &'static str = "BlockSealSpec";
				const EVENT: &'static str = "ComputeDifficultyAdjusted";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The current vote minimum of the chain. Block votes use this minimum to determine the"]
				#[doc = " minimum amount of tax or compute needed to create a vote. It is adjusted up or down to"]
				#[doc = " target a max number of votes"]
				pub fn current_vote_minimum(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u128,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"CurrentVoteMinimum",
						vec![],
						[
							12u8, 224u8, 174u8, 92u8, 253u8, 174u8, 51u8, 35u8, 165u8, 155u8,
							173u8, 118u8, 154u8, 150u8, 251u8, 57u8, 233u8, 6u8, 228u8, 92u8,
							186u8, 127u8, 187u8, 158u8, 160u8, 60u8, 117u8, 155u8, 93u8, 1u8,
							160u8, 27u8,
						],
					)
				}
				#[doc = " The current vote minimum of the chain. Block votes use this minimum to determine the"]
				#[doc = " minimum amount of tax or compute needed to create a vote. It is adjusted up or down to"]
				#[doc = " target a max number of votes"]
				pub fn current_compute_difficulty(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u128,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"CurrentComputeDifficulty",
						vec![],
						[
							65u8, 189u8, 189u8, 218u8, 13u8, 81u8, 240u8, 153u8, 77u8, 3u8, 71u8,
							26u8, 76u8, 244u8, 180u8, 15u8, 215u8, 66u8, 20u8, 70u8, 23u8, 133u8,
							136u8, 235u8, 193u8, 90u8, 222u8, 97u8, 139u8, 166u8, 94u8, 0u8,
						],
					)
				}
				pub fn past_compute_block_times(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u64,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"PastComputeBlockTimes",
						vec![],
						[
							210u8, 23u8, 204u8, 23u8, 189u8, 60u8, 128u8, 197u8, 199u8, 181u8,
							117u8, 11u8, 70u8, 235u8, 69u8, 110u8, 215u8, 114u8, 95u8, 198u8,
							131u8, 156u8, 166u8, 24u8, 128u8, 145u8, 205u8, 220u8, 107u8, 28u8,
							134u8, 72u8,
						],
					)
				}
				pub fn previous_block_timestamp(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"PreviousBlockTimestamp",
						vec![],
						[
							47u8, 107u8, 36u8, 17u8, 213u8, 89u8, 21u8, 145u8, 60u8, 224u8, 86u8,
							101u8, 51u8, 209u8, 56u8, 127u8, 186u8, 80u8, 27u8, 41u8, 23u8, 207u8,
							198u8, 42u8, 58u8, 103u8, 8u8, 226u8, 82u8, 191u8, 89u8, 193u8,
						],
					)
				}
				pub fn temp_block_timestamp(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"TempBlockTimestamp",
						vec![],
						[
							167u8, 201u8, 179u8, 72u8, 25u8, 20u8, 159u8, 162u8, 18u8, 154u8,
							169u8, 53u8, 137u8, 227u8, 96u8, 187u8, 3u8, 133u8, 155u8, 31u8, 92u8,
							145u8, 254u8, 239u8, 86u8, 215u8, 65u8, 223u8, 91u8, 120u8, 79u8, 34u8,
						],
					)
				}
				#[doc = " The calculated parent voting key for a block. Refers to the Notebook BlockVote Revealed"]
				#[doc = " Secret + VotesMerkleRoot of the parent block notebooks."]
				pub fn parent_voting_key(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::option::Option<::subxt::utils::H256>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"ParentVotingKey",
						vec![],
						[
							12u8, 73u8, 52u8, 154u8, 15u8, 127u8, 150u8, 214u8, 178u8, 186u8,
							231u8, 204u8, 104u8, 196u8, 141u8, 55u8, 198u8, 11u8, 23u8, 252u8,
							108u8, 65u8, 42u8, 124u8, 77u8, 77u8, 88u8, 35u8, 154u8, 241u8, 50u8,
							216u8,
						],
					)
				}
				#[doc = " Keeps the last 3 vote minimums. The first one applies to the current block."]
				pub fn vote_minimum_history(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u128,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"VoteMinimumHistory",
						vec![],
						[
							197u8, 183u8, 228u8, 59u8, 233u8, 183u8, 83u8, 132u8, 64u8, 76u8,
							112u8, 118u8, 156u8, 127u8, 114u8, 2u8, 189u8, 14u8, 255u8, 83u8,
							185u8, 11u8, 100u8, 71u8, 52u8, 7u8, 102u8, 205u8, 208u8, 103u8, 12u8,
							206u8,
						],
					)
				}
				#[doc = " Temporary store of the number of votes in the current block."]
				pub fn temp_notebooks_by_notary(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_btree_map::BoundedBTreeMap<
						::core::primitive::u32,
						runtime_types::ulx_primitives::notebook::NotebookHeader,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"TempNotebooksByNotary",
						vec![],
						[
							206u8, 180u8, 40u8, 28u8, 98u8, 144u8, 192u8, 222u8, 73u8, 177u8,
							108u8, 52u8, 169u8, 90u8, 152u8, 137u8, 7u8, 142u8, 12u8, 150u8, 158u8,
							18u8, 178u8, 38u8, 202u8, 253u8, 201u8, 166u8, 121u8, 183u8, 132u8,
							68u8,
						],
					)
				}
				#[doc = " Temporary store the vote digest"]
				pub fn temp_block_vote_digest(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_primitives::digests::BlockVoteDigest,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"TempBlockVoteDigest",
						vec![],
						[
							185u8, 155u8, 144u8, 104u8, 185u8, 198u8, 77u8, 36u8, 251u8, 81u8,
							240u8, 74u8, 126u8, 162u8, 113u8, 238u8, 145u8, 39u8, 1u8, 90u8, 221u8,
							205u8, 87u8, 154u8, 246u8, 188u8, 76u8, 103u8, 134u8, 23u8, 69u8,
							147u8,
						],
					)
				}
				pub fn past_block_votes(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						::core::primitive::u32,
						::core::primitive::u128,
					)>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"PastBlockVotes",
						vec![],
						[
							107u8, 45u8, 128u8, 167u8, 36u8, 64u8, 144u8, 19u8, 220u8, 71u8, 170u8,
							170u8, 116u8, 34u8, 17u8, 75u8, 111u8, 68u8, 152u8, 121u8, 139u8, 99u8,
							180u8, 73u8, 219u8, 28u8, 15u8, 98u8, 57u8, 94u8, 115u8, 88u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The desired votes per block"]
				pub fn target_block_votes(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"BlockSealSpec",
						"TargetBlockVotes",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The frequency for changing the minimum"]
				pub fn change_period(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"BlockSealSpec",
						"ChangePeriod",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
			}
		}
	}
	pub mod authorship {
		use super::{root_mod, runtime_types};
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Author of current block."]
				pub fn author(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::AccountId32,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Authorship",
						"Author",
						vec![],
						[
							247u8, 192u8, 118u8, 227u8, 47u8, 20u8, 203u8, 199u8, 216u8, 87u8,
							220u8, 50u8, 166u8, 61u8, 168u8, 213u8, 253u8, 62u8, 202u8, 199u8,
							61u8, 192u8, 237u8, 53u8, 22u8, 148u8, 164u8, 245u8, 99u8, 24u8, 146u8,
							18u8,
						],
					)
				}
			}
		}
	}
	pub mod historical {
		use super::{root_mod, runtime_types};
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Mapping from historical session indices to session-data root hash and validator count."]
				pub fn historical_sessions(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					(::subxt::utils::H256, ::core::primitive::u32),
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Historical",
						"HistoricalSessions",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							9u8, 138u8, 247u8, 141u8, 178u8, 146u8, 124u8, 81u8, 162u8, 211u8,
							205u8, 149u8, 222u8, 254u8, 253u8, 188u8, 170u8, 242u8, 218u8, 41u8,
							124u8, 178u8, 109u8, 209u8, 163u8, 125u8, 225u8, 206u8, 249u8, 175u8,
							117u8, 75u8,
						],
					)
				}
				#[doc = " Mapping from historical session indices to session-data root hash and validator count."]
				pub fn historical_sessions_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					(::subxt::utils::H256, ::core::primitive::u32),
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Historical",
						"HistoricalSessions",
						Vec::new(),
						[
							9u8, 138u8, 247u8, 141u8, 178u8, 146u8, 124u8, 81u8, 162u8, 211u8,
							205u8, 149u8, 222u8, 254u8, 253u8, 188u8, 170u8, 242u8, 218u8, 41u8,
							124u8, 178u8, 109u8, 209u8, 163u8, 125u8, 225u8, 206u8, 249u8, 175u8,
							117u8, 75u8,
						],
					)
				}
				#[doc = " The range of historical sessions we store. [first, last)"]
				pub fn stored_range(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					(::core::primitive::u32, ::core::primitive::u32),
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Historical",
						"StoredRange",
						vec![],
						[
							134u8, 32u8, 250u8, 13u8, 201u8, 25u8, 54u8, 243u8, 231u8, 81u8, 252u8,
							231u8, 68u8, 217u8, 235u8, 43u8, 22u8, 223u8, 220u8, 133u8, 198u8,
							218u8, 95u8, 152u8, 189u8, 87u8, 6u8, 228u8, 242u8, 59u8, 232u8, 59u8,
						],
					)
				}
			}
		}
	}
	pub mod session {
		use super::{root_mod, runtime_types};
		#[doc = "Error for the session pallet."]
		pub type Error = runtime_types::pallet_session::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_session::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetKeys {
					pub keys: runtime_types::ulx_node_runtime::opaque::SessionKeys,
					pub proof: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetKeys {
					const PALLET: &'static str = "Session";
					const CALL: &'static str = "set_keys";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct PurgeKeys;
				impl ::subxt::blocks::StaticExtrinsic for PurgeKeys {
					const PALLET: &'static str = "Session";
					const CALL: &'static str = "purge_keys";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::set_keys`]."]
				pub fn set_keys(
					&self,
					keys: runtime_types::ulx_node_runtime::opaque::SessionKeys,
					proof: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::SetKeys> {
					::subxt::tx::Payload::new_static(
						"Session",
						"set_keys",
						types::SetKeys { keys, proof },
						[
							236u8, 25u8, 135u8, 232u8, 99u8, 88u8, 192u8, 190u8, 245u8, 173u8,
							196u8, 17u8, 61u8, 103u8, 72u8, 41u8, 119u8, 156u8, 202u8, 221u8, 17u8,
							155u8, 41u8, 44u8, 238u8, 152u8, 118u8, 74u8, 35u8, 168u8, 240u8, 98u8,
						],
					)
				}
				#[doc = "See [`Pallet::purge_keys`]."]
				pub fn purge_keys(&self) -> ::subxt::tx::Payload<types::PurgeKeys> {
					::subxt::tx::Payload::new_static(
						"Session",
						"purge_keys",
						types::PurgeKeys {},
						[
							215u8, 204u8, 146u8, 236u8, 32u8, 78u8, 198u8, 79u8, 85u8, 214u8, 15u8,
							151u8, 158u8, 31u8, 146u8, 119u8, 119u8, 204u8, 151u8, 169u8, 226u8,
							67u8, 217u8, 39u8, 241u8, 245u8, 203u8, 240u8, 203u8, 172u8, 16u8,
							209u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_session::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "New session has happened. Note that the argument is the session index, not the"]
			#[doc = "block number as the type might suggest."]
			pub struct NewSession {
				pub session_index: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for NewSession {
				const PALLET: &'static str = "Session";
				const EVENT: &'static str = "NewSession";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The current set of validators."]
				pub fn validators(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<::subxt::utils::AccountId32>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"Validators",
						vec![],
						[
							50u8, 86u8, 154u8, 222u8, 249u8, 209u8, 156u8, 22u8, 155u8, 25u8,
							133u8, 194u8, 210u8, 50u8, 38u8, 28u8, 139u8, 201u8, 90u8, 139u8,
							115u8, 12u8, 12u8, 141u8, 4u8, 178u8, 201u8, 241u8, 223u8, 234u8, 6u8,
							86u8,
						],
					)
				}
				#[doc = " Current index of the session."]
				pub fn current_index(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"CurrentIndex",
						vec![],
						[
							167u8, 151u8, 125u8, 150u8, 159u8, 21u8, 78u8, 217u8, 237u8, 183u8,
							135u8, 65u8, 187u8, 114u8, 188u8, 206u8, 16u8, 32u8, 69u8, 208u8,
							134u8, 159u8, 232u8, 224u8, 243u8, 27u8, 31u8, 166u8, 145u8, 44u8,
							221u8, 230u8,
						],
					)
				}
				#[doc = " True if the underlying economic identities or weighting behind the validators"]
				#[doc = " has changed in the queued validator set."]
				pub fn queued_changed(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"QueuedChanged",
						vec![],
						[
							184u8, 137u8, 224u8, 137u8, 31u8, 236u8, 95u8, 164u8, 102u8, 225u8,
							198u8, 227u8, 140u8, 37u8, 113u8, 57u8, 59u8, 4u8, 202u8, 102u8, 117u8,
							36u8, 226u8, 64u8, 113u8, 141u8, 199u8, 111u8, 99u8, 144u8, 198u8,
							153u8,
						],
					)
				}
				#[doc = " The queued keys for the next session. When the next session begins, these keys"]
				#[doc = " will be used to determine the validator's session keys."]
				pub fn queued_keys(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<(
						::subxt::utils::AccountId32,
						runtime_types::ulx_node_runtime::opaque::SessionKeys,
					)>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"QueuedKeys",
						vec![],
						[
							217u8, 110u8, 17u8, 82u8, 156u8, 134u8, 163u8, 105u8, 251u8, 3u8, 75u8,
							109u8, 155u8, 252u8, 102u8, 172u8, 193u8, 92u8, 42u8, 101u8, 69u8,
							134u8, 75u8, 28u8, 178u8, 7u8, 140u8, 238u8, 147u8, 49u8, 27u8, 216u8,
						],
					)
				}
				#[doc = " Indices of disabled validators."]
				#[doc = ""]
				#[doc = " The vec is always kept sorted so that we can find whether a given validator is"]
				#[doc = " disabled using binary search. It gets cleared when `on_session_ending` returns"]
				#[doc = " a new set of identities."]
				pub fn disabled_validators(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<::core::primitive::u32>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"DisabledValidators",
						vec![],
						[
							213u8, 19u8, 168u8, 234u8, 187u8, 200u8, 180u8, 97u8, 234u8, 189u8,
							36u8, 233u8, 158u8, 184u8, 45u8, 35u8, 129u8, 213u8, 133u8, 8u8, 104u8,
							183u8, 46u8, 68u8, 154u8, 240u8, 132u8, 22u8, 247u8, 11u8, 54u8, 221u8,
						],
					)
				}
				#[doc = " The next session keys for a validator."]
				pub fn next_keys(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_node_runtime::opaque::SessionKeys,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"NextKeys",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							25u8, 1u8, 69u8, 59u8, 90u8, 33u8, 59u8, 39u8, 221u8, 183u8, 101u8,
							140u8, 35u8, 166u8, 124u8, 245u8, 26u8, 245u8, 99u8, 105u8, 96u8,
							239u8, 46u8, 104u8, 158u8, 21u8, 76u8, 45u8, 164u8, 150u8, 134u8,
							201u8,
						],
					)
				}
				#[doc = " The next session keys for a validator."]
				pub fn next_keys_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_node_runtime::opaque::SessionKeys,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"NextKeys",
						Vec::new(),
						[
							25u8, 1u8, 69u8, 59u8, 90u8, 33u8, 59u8, 39u8, 221u8, 183u8, 101u8,
							140u8, 35u8, 166u8, 124u8, 245u8, 26u8, 245u8, 99u8, 105u8, 96u8,
							239u8, 46u8, 104u8, 158u8, 21u8, 76u8, 45u8, 164u8, 150u8, 134u8,
							201u8,
						],
					)
				}
				#[doc = " The owner of a key. The key is the `KeyTypeId` + the encoded key."]
				pub fn key_owner(
					&self,
					_0: impl ::std::borrow::Borrow<runtime_types::sp_core::crypto::KeyTypeId>,
					_1: impl ::std::borrow::Borrow<[::core::primitive::u8]>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::AccountId32,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"KeyOwner",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							217u8, 204u8, 21u8, 114u8, 247u8, 129u8, 32u8, 242u8, 93u8, 91u8,
							253u8, 253u8, 248u8, 90u8, 12u8, 202u8, 195u8, 25u8, 18u8, 100u8,
							253u8, 109u8, 88u8, 77u8, 217u8, 140u8, 51u8, 40u8, 118u8, 35u8, 107u8,
							206u8,
						],
					)
				}
				#[doc = " The owner of a key. The key is the `KeyTypeId` + the encoded key."]
				pub fn key_owner_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::AccountId32,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"KeyOwner",
						Vec::new(),
						[
							217u8, 204u8, 21u8, 114u8, 247u8, 129u8, 32u8, 242u8, 93u8, 91u8,
							253u8, 253u8, 248u8, 90u8, 12u8, 202u8, 195u8, 25u8, 18u8, 100u8,
							253u8, 109u8, 88u8, 77u8, 217u8, 140u8, 51u8, 40u8, 118u8, 35u8, 107u8,
							206u8,
						],
					)
				}
			}
		}
	}
	pub mod block_seal {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_block_seal::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_block_seal::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Apply {
					pub seal: runtime_types::ulx_primitives::inherents::BlockSealInherent,
				}
				impl ::subxt::blocks::StaticExtrinsic for Apply {
					const PALLET: &'static str = "BlockSeal";
					const CALL: &'static str = "apply";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See `Pallet::apply`."]
				pub fn apply(
					&self,
					seal: runtime_types::ulx_primitives::inherents::BlockSealInherent,
				) -> ::subxt::tx::Payload<types::Apply> {
					::subxt::tx::Payload::new_static(
						"BlockSeal",
						"apply",
						types::Apply { seal },
						[
							38u8, 189u8, 107u8, 225u8, 124u8, 204u8, 221u8, 6u8, 55u8, 3u8, 54u8,
							183u8, 247u8, 85u8, 234u8, 131u8, 108u8, 77u8, 170u8, 149u8, 91u8,
							235u8, 32u8, 25u8, 132u8, 78u8, 186u8, 44u8, 91u8, 145u8, 123u8, 95u8,
						],
					)
				}
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				pub fn last_block_sealer_info(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_primitives::providers::BlockSealerInfo<
						::subxt::utils::AccountId32,
					>,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSeal",
						"LastBlockSealerInfo",
						vec![],
						[
							170u8, 198u8, 254u8, 213u8, 119u8, 214u8, 253u8, 36u8, 138u8, 37u8,
							130u8, 217u8, 221u8, 124u8, 83u8, 194u8, 2u8, 73u8, 53u8, 216u8, 178u8,
							51u8, 59u8, 51u8, 209u8, 182u8, 102u8, 209u8, 88u8, 56u8, 96u8, 231u8,
						],
					)
				}
				#[doc = " Author of current block (temporary storage)."]
				pub fn temp_author(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::AccountId32,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSeal",
						"TempAuthor",
						vec![],
						[
							29u8, 149u8, 234u8, 74u8, 206u8, 138u8, 152u8, 92u8, 28u8, 103u8, 4u8,
							236u8, 161u8, 51u8, 52u8, 196u8, 28u8, 242u8, 250u8, 210u8, 187u8,
							78u8, 217u8, 251u8, 157u8, 143u8, 91u8, 60u8, 246u8, 218u8, 227u8,
							114u8,
						],
					)
				}
				pub fn temp_block_vote_digest(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_primitives::digests::BlockVoteDigest,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSeal",
						"TempBlockVoteDigest",
						vec![],
						[
							185u8, 155u8, 144u8, 104u8, 185u8, 198u8, 77u8, 36u8, 251u8, 81u8,
							240u8, 74u8, 126u8, 162u8, 113u8, 238u8, 145u8, 39u8, 1u8, 90u8, 221u8,
							205u8, 87u8, 154u8, 246u8, 188u8, 76u8, 103u8, 134u8, 23u8, 69u8,
							147u8,
						],
					)
				}
				#[doc = " Ensures only a single inherent is applied"]
				pub fn temp_seal_inherent(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ulx_primitives::inherents::BlockSealInherent,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSeal",
						"TempSealInherent",
						vec![],
						[
							71u8, 139u8, 103u8, 110u8, 229u8, 89u8, 241u8, 248u8, 35u8, 28u8,
							109u8, 64u8, 141u8, 77u8, 19u8, 190u8, 6u8, 48u8, 183u8, 176u8, 113u8,
							16u8, 22u8, 103u8, 196u8, 138u8, 40u8, 78u8, 124u8, 86u8, 107u8, 159u8,
						],
					)
				}
			}
		}
	}
	pub mod block_rewards {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_block_rewards::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_block_rewards::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
			}
			pub struct TransactionApi;
			impl TransactionApi {}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_block_rewards::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct RewardCreated {
				pub maturation_block: ::core::primitive::u32,
				pub rewards: ::std::vec::Vec<
					runtime_types::pallet_block_rewards::pallet::BlockPayout<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
					>,
				>,
			}
			impl ::subxt::events::StaticEvent for RewardCreated {
				const PALLET: &'static str = "BlockRewards";
				const EVENT: &'static str = "RewardCreated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct RewardUnlocked {
				pub rewards: ::std::vec::Vec<
					runtime_types::pallet_block_rewards::pallet::BlockPayout<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
					>,
				>,
			}
			impl ::subxt::events::StaticEvent for RewardUnlocked {
				const PALLET: &'static str = "BlockRewards";
				const EVENT: &'static str = "RewardUnlocked";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				pub fn payouts_by_block(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_block_rewards::pallet::BlockPayout<
							::subxt::utils::AccountId32,
							::core::primitive::u128,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"BlockRewards",
						"PayoutsByBlock",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							69u8, 232u8, 99u8, 71u8, 94u8, 15u8, 227u8, 177u8, 153u8, 246u8, 17u8,
							248u8, 101u8, 239u8, 203u8, 172u8, 253u8, 130u8, 39u8, 90u8, 48u8,
							233u8, 49u8, 36u8, 118u8, 214u8, 241u8, 47u8, 136u8, 190u8, 36u8, 91u8,
						],
					)
				}
				pub fn payouts_by_block_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_block_rewards::pallet::BlockPayout<
							::subxt::utils::AccountId32,
							::core::primitive::u128,
						>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"BlockRewards",
						"PayoutsByBlock",
						Vec::new(),
						[
							69u8, 232u8, 99u8, 71u8, 94u8, 15u8, 227u8, 177u8, 153u8, 246u8, 17u8,
							248u8, 101u8, 239u8, 203u8, 172u8, 253u8, 130u8, 39u8, 90u8, 48u8,
							233u8, 49u8, 36u8, 118u8, 214u8, 241u8, 47u8, 136u8, 190u8, 36u8, 91u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " Number of argons minted per block"]
				pub fn argons_per_block(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"BlockRewards",
						"ArgonsPerBlock",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " Number of ulixees minted per block"]
				pub fn starting_ulixees_per_block(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"BlockRewards",
						"StartingUlixeesPerBlock",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " Number of blocks for halving of ulixee rewards"]
				pub fn halving_blocks(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"BlockRewards",
						"HalvingBlocks",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " Percent as a number out of 100 of the block reward that goes to the miner"]
				pub fn miner_payout_percent(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"BlockRewards",
						"MinerPayoutPercent",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " Blocks until a block reward is mature"]
				pub fn maturation_blocks(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"BlockRewards",
						"MaturationBlocks",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
			}
		}
	}
	pub mod grandpa {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_grandpa::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_grandpa::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ReportEquivocation {
					pub equivocation_proof: ::std::boxed::Box<
						runtime_types::sp_consensus_grandpa::EquivocationProof<
							::subxt::utils::H256,
							::core::primitive::u32,
						>,
					>,
					pub key_owner_proof: runtime_types::sp_session::MembershipProof,
				}
				impl ::subxt::blocks::StaticExtrinsic for ReportEquivocation {
					const PALLET: &'static str = "Grandpa";
					const CALL: &'static str = "report_equivocation";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ReportEquivocationUnsigned {
					pub equivocation_proof: ::std::boxed::Box<
						runtime_types::sp_consensus_grandpa::EquivocationProof<
							::subxt::utils::H256,
							::core::primitive::u32,
						>,
					>,
					pub key_owner_proof: runtime_types::sp_session::MembershipProof,
				}
				impl ::subxt::blocks::StaticExtrinsic for ReportEquivocationUnsigned {
					const PALLET: &'static str = "Grandpa";
					const CALL: &'static str = "report_equivocation_unsigned";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct NoteStalled {
					pub delay: ::core::primitive::u32,
					pub best_finalized_block_number: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for NoteStalled {
					const PALLET: &'static str = "Grandpa";
					const CALL: &'static str = "note_stalled";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::report_equivocation`]."]
				pub fn report_equivocation(
					&self,
					equivocation_proof: runtime_types::sp_consensus_grandpa::EquivocationProof<
						::subxt::utils::H256,
						::core::primitive::u32,
					>,
					key_owner_proof: runtime_types::sp_session::MembershipProof,
				) -> ::subxt::tx::Payload<types::ReportEquivocation> {
					::subxt::tx::Payload::new_static(
						"Grandpa",
						"report_equivocation",
						types::ReportEquivocation {
							equivocation_proof: ::std::boxed::Box::new(equivocation_proof),
							key_owner_proof,
						},
						[
							11u8, 183u8, 81u8, 93u8, 41u8, 7u8, 70u8, 155u8, 8u8, 57u8, 177u8,
							245u8, 131u8, 79u8, 236u8, 118u8, 147u8, 114u8, 40u8, 204u8, 177u8,
							2u8, 43u8, 42u8, 2u8, 201u8, 202u8, 120u8, 150u8, 109u8, 108u8, 156u8,
						],
					)
				}
				#[doc = "See [`Pallet::report_equivocation_unsigned`]."]
				pub fn report_equivocation_unsigned(
					&self,
					equivocation_proof: runtime_types::sp_consensus_grandpa::EquivocationProof<
						::subxt::utils::H256,
						::core::primitive::u32,
					>,
					key_owner_proof: runtime_types::sp_session::MembershipProof,
				) -> ::subxt::tx::Payload<types::ReportEquivocationUnsigned> {
					::subxt::tx::Payload::new_static(
						"Grandpa",
						"report_equivocation_unsigned",
						types::ReportEquivocationUnsigned {
							equivocation_proof: ::std::boxed::Box::new(equivocation_proof),
							key_owner_proof,
						},
						[
							141u8, 133u8, 227u8, 65u8, 22u8, 181u8, 108u8, 9u8, 157u8, 27u8, 124u8,
							53u8, 177u8, 27u8, 5u8, 16u8, 193u8, 66u8, 59u8, 87u8, 143u8, 238u8,
							251u8, 167u8, 117u8, 138u8, 246u8, 236u8, 65u8, 148u8, 20u8, 131u8,
						],
					)
				}
				#[doc = "See [`Pallet::note_stalled`]."]
				pub fn note_stalled(
					&self,
					delay: ::core::primitive::u32,
					best_finalized_block_number: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::NoteStalled> {
					::subxt::tx::Payload::new_static(
						"Grandpa",
						"note_stalled",
						types::NoteStalled { delay, best_finalized_block_number },
						[
							158u8, 25u8, 64u8, 114u8, 131u8, 139u8, 227u8, 132u8, 42u8, 107u8,
							40u8, 249u8, 18u8, 93u8, 254u8, 86u8, 37u8, 67u8, 250u8, 35u8, 241u8,
							194u8, 209u8, 20u8, 39u8, 75u8, 186u8, 21u8, 48u8, 124u8, 151u8, 31u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_grandpa::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "New authority set has been applied."]
			pub struct NewAuthorities {
				pub authority_set: ::std::vec::Vec<(
					runtime_types::sp_consensus_grandpa::app::Public,
					::core::primitive::u64,
				)>,
			}
			impl ::subxt::events::StaticEvent for NewAuthorities {
				const PALLET: &'static str = "Grandpa";
				const EVENT: &'static str = "NewAuthorities";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Current authority set has been paused."]
			pub struct Paused;
			impl ::subxt::events::StaticEvent for Paused {
				const PALLET: &'static str = "Grandpa";
				const EVENT: &'static str = "Paused";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Current authority set has been resumed."]
			pub struct Resumed;
			impl ::subxt::events::StaticEvent for Resumed {
				const PALLET: &'static str = "Grandpa";
				const EVENT: &'static str = "Resumed";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " State of the current authority set."]
				pub fn state(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_grandpa::StoredState<::core::primitive::u32>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"State",
						vec![],
						[
							73u8, 71u8, 112u8, 83u8, 238u8, 75u8, 44u8, 9u8, 180u8, 33u8, 30u8,
							121u8, 98u8, 96u8, 61u8, 133u8, 16u8, 70u8, 30u8, 249u8, 34u8, 148u8,
							15u8, 239u8, 164u8, 157u8, 52u8, 27u8, 144u8, 52u8, 223u8, 109u8,
						],
					)
				}
				#[doc = " Pending change: (signaled at, scheduled change)."]
				pub fn pending_change(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_grandpa::StoredPendingChange<::core::primitive::u32>,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"PendingChange",
						vec![],
						[
							150u8, 194u8, 185u8, 248u8, 239u8, 43u8, 141u8, 253u8, 61u8, 106u8,
							74u8, 164u8, 209u8, 204u8, 206u8, 200u8, 32u8, 38u8, 11u8, 78u8, 84u8,
							243u8, 181u8, 142u8, 179u8, 151u8, 81u8, 204u8, 244u8, 150u8, 137u8,
							250u8,
						],
					)
				}
				#[doc = " next block number where we can force a change."]
				pub fn next_forced(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"NextForced",
						vec![],
						[
							3u8, 231u8, 56u8, 18u8, 87u8, 112u8, 227u8, 126u8, 180u8, 131u8, 255u8,
							141u8, 82u8, 34u8, 61u8, 47u8, 234u8, 37u8, 95u8, 62u8, 33u8, 235u8,
							231u8, 122u8, 125u8, 8u8, 223u8, 95u8, 255u8, 204u8, 40u8, 97u8,
						],
					)
				}
				#[doc = " `true` if we are currently stalled."]
				pub fn stalled(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					(::core::primitive::u32, ::core::primitive::u32),
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"Stalled",
						vec![],
						[
							6u8, 81u8, 205u8, 142u8, 195u8, 48u8, 0u8, 247u8, 108u8, 170u8, 10u8,
							249u8, 72u8, 206u8, 32u8, 103u8, 109u8, 57u8, 51u8, 21u8, 144u8, 204u8,
							79u8, 8u8, 191u8, 185u8, 38u8, 34u8, 118u8, 223u8, 75u8, 241u8,
						],
					)
				}
				#[doc = " The number of changes (both in terms of keys and underlying economic responsibilities)"]
				#[doc = " in the \"set\" of Grandpa validators from genesis."]
				pub fn current_set_id(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"CurrentSetId",
						vec![],
						[
							234u8, 215u8, 218u8, 42u8, 30u8, 76u8, 129u8, 40u8, 125u8, 137u8,
							207u8, 47u8, 46u8, 213u8, 159u8, 50u8, 175u8, 81u8, 155u8, 123u8,
							246u8, 175u8, 156u8, 68u8, 22u8, 113u8, 135u8, 137u8, 163u8, 18u8,
							115u8, 73u8,
						],
					)
				}
				#[doc = " A mapping from grandpa set ID to the index of the *most recent* session for which its"]
				#[doc = " members were responsible."]
				#[doc = ""]
				#[doc = " This is only used for validating equivocation proofs. An equivocation proof must"]
				#[doc = " contains a key-ownership proof for a given session, therefore we need a way to tie"]
				#[doc = " together sessions and GRANDPA set ids, i.e. we need to validate that a validator"]
				#[doc = " was the owner of a given key on a given session, and what the active set ID was"]
				#[doc = " during that session."]
				#[doc = ""]
				#[doc = " TWOX-NOTE: `SetId` is not under user control."]
				pub fn set_id_session(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u64>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"SetIdSession",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							47u8, 0u8, 239u8, 121u8, 187u8, 213u8, 254u8, 50u8, 238u8, 10u8, 162u8,
							65u8, 189u8, 166u8, 37u8, 74u8, 82u8, 81u8, 160u8, 20u8, 180u8, 253u8,
							238u8, 18u8, 209u8, 203u8, 38u8, 148u8, 16u8, 105u8, 72u8, 169u8,
						],
					)
				}
				#[doc = " A mapping from grandpa set ID to the index of the *most recent* session for which its"]
				#[doc = " members were responsible."]
				#[doc = ""]
				#[doc = " This is only used for validating equivocation proofs. An equivocation proof must"]
				#[doc = " contains a key-ownership proof for a given session, therefore we need a way to tie"]
				#[doc = " together sessions and GRANDPA set ids, i.e. we need to validate that a validator"]
				#[doc = " was the owner of a given key on a given session, and what the active set ID was"]
				#[doc = " during that session."]
				#[doc = ""]
				#[doc = " TWOX-NOTE: `SetId` is not under user control."]
				pub fn set_id_session_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"SetIdSession",
						Vec::new(),
						[
							47u8, 0u8, 239u8, 121u8, 187u8, 213u8, 254u8, 50u8, 238u8, 10u8, 162u8,
							65u8, 189u8, 166u8, 37u8, 74u8, 82u8, 81u8, 160u8, 20u8, 180u8, 253u8,
							238u8, 18u8, 209u8, 203u8, 38u8, 148u8, 16u8, 105u8, 72u8, 169u8,
						],
					)
				}
				#[doc = " The current list of authorities."]
				pub fn authorities(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<(
						runtime_types::sp_consensus_grandpa::app::Public,
						::core::primitive::u64,
					)>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"Authorities",
						vec![],
						[
							67u8, 196u8, 244u8, 13u8, 246u8, 245u8, 198u8, 98u8, 81u8, 55u8, 182u8,
							187u8, 214u8, 5u8, 181u8, 76u8, 251u8, 213u8, 144u8, 166u8, 36u8,
							153u8, 234u8, 181u8, 252u8, 55u8, 198u8, 175u8, 55u8, 211u8, 105u8,
							85u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " Max Authorities in use"]
				pub fn max_authorities(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Grandpa",
						"MaxAuthorities",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum number of nominators for each validator."]
				pub fn max_nominators(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Grandpa",
						"MaxNominators",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum number of entries to keep in the set id to session index mapping."]
				#[doc = ""]
				#[doc = " Since the `SetIdSession` map is only used for validating equivocations this"]
				#[doc = " value should relate to the bonding duration of whatever staking system is"]
				#[doc = " being used (if any). If equivocation handling is not enabled then this value"]
				#[doc = " can be zero."]
				pub fn max_set_id_session_entries(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u64> {
					::subxt::constants::Address::new_static(
						"Grandpa",
						"MaxSetIdSessionEntries",
						[
							128u8, 214u8, 205u8, 242u8, 181u8, 142u8, 124u8, 231u8, 190u8, 146u8,
							59u8, 226u8, 157u8, 101u8, 103u8, 117u8, 249u8, 65u8, 18u8, 191u8,
							103u8, 119u8, 53u8, 85u8, 81u8, 96u8, 220u8, 42u8, 184u8, 239u8, 42u8,
							246u8,
						],
					)
				}
			}
		}
	}
	pub mod offences {
		use super::{root_mod, runtime_types};
		#[doc = "Events type."]
		pub type Event = runtime_types::pallet_offences::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "There is an offence reported of the given `kind` happened at the `session_index` and"]
			#[doc = "(kind-specific) time slot. This event is not deposited for duplicate slashes."]
			#[doc = "\\[kind, timeslot\\]."]
			pub struct Offence {
				pub kind: [::core::primitive::u8; 16usize],
				pub timeslot: ::std::vec::Vec<::core::primitive::u8>,
			}
			impl ::subxt::events::StaticEvent for Offence {
				const PALLET: &'static str = "Offences";
				const EVENT: &'static str = "Offence";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The primary structure that holds all offence records keyed by report identifiers."]
				pub fn reports(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::H256>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::sp_staking::offence::OffenceDetails<
						::subxt::utils::AccountId32,
						(
							::subxt::utils::AccountId32,
							runtime_types::pallet_mining_slot::MinerHistory,
						),
					>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Offences",
						"Reports",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							205u8, 231u8, 221u8, 1u8, 157u8, 93u8, 122u8, 97u8, 61u8, 216u8, 201u8,
							203u8, 114u8, 249u8, 113u8, 235u8, 82u8, 159u8, 25u8, 19u8, 207u8,
							108u8, 214u8, 122u8, 8u8, 1u8, 110u8, 191u8, 218u8, 248u8, 56u8, 36u8,
						],
					)
				}
				#[doc = " The primary structure that holds all offence records keyed by report identifiers."]
				pub fn reports_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::sp_staking::offence::OffenceDetails<
						::subxt::utils::AccountId32,
						(
							::subxt::utils::AccountId32,
							runtime_types::pallet_mining_slot::MinerHistory,
						),
					>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Offences",
						"Reports",
						Vec::new(),
						[
							205u8, 231u8, 221u8, 1u8, 157u8, 93u8, 122u8, 97u8, 61u8, 216u8, 201u8,
							203u8, 114u8, 249u8, 113u8, 235u8, 82u8, 159u8, 25u8, 19u8, 207u8,
							108u8, 214u8, 122u8, 8u8, 1u8, 110u8, 191u8, 218u8, 248u8, 56u8, 36u8,
						],
					)
				}
				#[doc = " A vector of reports of the same kind that happened at the same time slot."]
				pub fn concurrent_reports_index(
					&self,
					_0: impl ::std::borrow::Borrow<[::core::primitive::u8; 16usize]>,
					_1: impl ::std::borrow::Borrow<[::core::primitive::u8]>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<::subxt::utils::H256>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Offences",
						"ConcurrentReportsIndex",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							170u8, 186u8, 72u8, 29u8, 251u8, 38u8, 193u8, 195u8, 109u8, 86u8, 0u8,
							241u8, 20u8, 235u8, 108u8, 126u8, 215u8, 82u8, 73u8, 113u8, 199u8,
							138u8, 24u8, 58u8, 216u8, 72u8, 221u8, 232u8, 252u8, 244u8, 96u8,
							247u8,
						],
					)
				}
				#[doc = " A vector of reports of the same kind that happened at the same time slot."]
				pub fn concurrent_reports_index_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<::subxt::utils::H256>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Offences",
						"ConcurrentReportsIndex",
						Vec::new(),
						[
							170u8, 186u8, 72u8, 29u8, 251u8, 38u8, 193u8, 195u8, 109u8, 86u8, 0u8,
							241u8, 20u8, 235u8, 108u8, 126u8, 215u8, 82u8, 73u8, 113u8, 199u8,
							138u8, 24u8, 58u8, 216u8, 72u8, 221u8, 232u8, 252u8, 244u8, 96u8,
							247u8,
						],
					)
				}
			}
		}
	}
	pub mod argon_balances {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_balances::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_balances::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferAllowDeath {
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub value: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferAllowDeath {
					const PALLET: &'static str = "ArgonBalances";
					const CALL: &'static str = "transfer_allow_death";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceTransfer {
					pub source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub value: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceTransfer {
					const PALLET: &'static str = "ArgonBalances";
					const CALL: &'static str = "force_transfer";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferKeepAlive {
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub value: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferKeepAlive {
					const PALLET: &'static str = "ArgonBalances";
					const CALL: &'static str = "transfer_keep_alive";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferAll {
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub keep_alive: ::core::primitive::bool,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferAll {
					const PALLET: &'static str = "ArgonBalances";
					const CALL: &'static str = "transfer_all";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceUnreserve {
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub amount: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceUnreserve {
					const PALLET: &'static str = "ArgonBalances";
					const CALL: &'static str = "force_unreserve";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpgradeAccounts {
					pub who: ::std::vec::Vec<::subxt::utils::AccountId32>,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpgradeAccounts {
					const PALLET: &'static str = "ArgonBalances";
					const CALL: &'static str = "upgrade_accounts";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceSetBalance {
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub new_free: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceSetBalance {
					const PALLET: &'static str = "ArgonBalances";
					const CALL: &'static str = "force_set_balance";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::transfer_allow_death`]."]
				pub fn transfer_allow_death(
					&self,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					value: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::TransferAllowDeath> {
					::subxt::tx::Payload::new_static(
						"ArgonBalances",
						"transfer_allow_death",
						types::TransferAllowDeath { dest, value },
						[
							51u8, 166u8, 195u8, 10u8, 139u8, 218u8, 55u8, 130u8, 6u8, 194u8, 35u8,
							140u8, 27u8, 205u8, 214u8, 222u8, 102u8, 43u8, 143u8, 145u8, 86u8,
							219u8, 210u8, 147u8, 13u8, 39u8, 51u8, 21u8, 237u8, 179u8, 132u8,
							130u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_transfer`]."]
				pub fn force_transfer(
					&self,
					source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					value: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ForceTransfer> {
					::subxt::tx::Payload::new_static(
						"ArgonBalances",
						"force_transfer",
						types::ForceTransfer { source, dest, value },
						[
							154u8, 93u8, 222u8, 27u8, 12u8, 248u8, 63u8, 213u8, 224u8, 86u8, 250u8,
							153u8, 249u8, 102u8, 83u8, 160u8, 79u8, 125u8, 105u8, 222u8, 77u8,
							180u8, 90u8, 105u8, 81u8, 217u8, 60u8, 25u8, 213u8, 51u8, 185u8, 96u8,
						],
					)
				}
				#[doc = "See [`Pallet::transfer_keep_alive`]."]
				pub fn transfer_keep_alive(
					&self,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					value: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::TransferKeepAlive> {
					::subxt::tx::Payload::new_static(
						"ArgonBalances",
						"transfer_keep_alive",
						types::TransferKeepAlive { dest, value },
						[
							245u8, 14u8, 190u8, 193u8, 32u8, 210u8, 74u8, 92u8, 25u8, 182u8, 76u8,
							55u8, 247u8, 83u8, 114u8, 75u8, 143u8, 236u8, 117u8, 25u8, 54u8, 157u8,
							208u8, 207u8, 233u8, 89u8, 70u8, 161u8, 235u8, 242u8, 222u8, 59u8,
						],
					)
				}
				#[doc = "See [`Pallet::transfer_all`]."]
				pub fn transfer_all(
					&self,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					keep_alive: ::core::primitive::bool,
				) -> ::subxt::tx::Payload<types::TransferAll> {
					::subxt::tx::Payload::new_static(
						"ArgonBalances",
						"transfer_all",
						types::TransferAll { dest, keep_alive },
						[
							105u8, 132u8, 49u8, 144u8, 195u8, 250u8, 34u8, 46u8, 213u8, 248u8,
							112u8, 188u8, 81u8, 228u8, 136u8, 18u8, 67u8, 172u8, 37u8, 38u8, 238u8,
							9u8, 34u8, 15u8, 67u8, 34u8, 148u8, 195u8, 223u8, 29u8, 154u8, 6u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_unreserve`]."]
				pub fn force_unreserve(
					&self,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					amount: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ForceUnreserve> {
					::subxt::tx::Payload::new_static(
						"ArgonBalances",
						"force_unreserve",
						types::ForceUnreserve { who, amount },
						[
							142u8, 151u8, 64u8, 205u8, 46u8, 64u8, 62u8, 122u8, 108u8, 49u8, 223u8,
							140u8, 120u8, 153u8, 35u8, 165u8, 187u8, 38u8, 157u8, 200u8, 123u8,
							199u8, 198u8, 168u8, 208u8, 159u8, 39u8, 134u8, 92u8, 103u8, 84u8,
							171u8,
						],
					)
				}
				#[doc = "See [`Pallet::upgrade_accounts`]."]
				pub fn upgrade_accounts(
					&self,
					who: ::std::vec::Vec<::subxt::utils::AccountId32>,
				) -> ::subxt::tx::Payload<types::UpgradeAccounts> {
					::subxt::tx::Payload::new_static(
						"ArgonBalances",
						"upgrade_accounts",
						types::UpgradeAccounts { who },
						[
							66u8, 200u8, 179u8, 104u8, 65u8, 2u8, 101u8, 56u8, 130u8, 161u8, 224u8,
							233u8, 255u8, 124u8, 70u8, 122u8, 8u8, 49u8, 103u8, 178u8, 68u8, 47u8,
							214u8, 166u8, 217u8, 116u8, 178u8, 50u8, 212u8, 164u8, 98u8, 226u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_set_balance`]."]
				pub fn force_set_balance(
					&self,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					new_free: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ForceSetBalance> {
					::subxt::tx::Payload::new_static(
						"ArgonBalances",
						"force_set_balance",
						types::ForceSetBalance { who, new_free },
						[
							114u8, 229u8, 59u8, 204u8, 180u8, 83u8, 17u8, 4u8, 59u8, 4u8, 55u8,
							39u8, 151u8, 196u8, 124u8, 60u8, 209u8, 65u8, 193u8, 11u8, 44u8, 164u8,
							116u8, 93u8, 169u8, 30u8, 199u8, 165u8, 55u8, 231u8, 223u8, 43u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_balances::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was created with some free balance."]
			pub struct Endowed {
				pub account: ::subxt::utils::AccountId32,
				pub free_balance: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Endowed {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Endowed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was removed whose balance was non-zero but below ExistentialDeposit,"]
			#[doc = "resulting in an outright loss."]
			pub struct DustLost {
				pub account: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for DustLost {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "DustLost";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Transfer succeeded."]
			pub struct Transfer {
				pub from: ::subxt::utils::AccountId32,
				pub to: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Transfer {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Transfer";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A balance was set by root."]
			pub struct BalanceSet {
				pub who: ::subxt::utils::AccountId32,
				pub free: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for BalanceSet {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "BalanceSet";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was reserved (moved from free to reserved)."]
			pub struct Reserved {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Reserved {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Reserved";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was unreserved (moved from reserved to free)."]
			pub struct Unreserved {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Unreserved {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Unreserved";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was moved from the reserve of the first account to the second account."]
			#[doc = "Final argument indicates the destination balance type."]
			pub struct ReserveRepatriated {
				pub from: ::subxt::utils::AccountId32,
				pub to: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
				pub destination_status:
					runtime_types::frame_support::traits::tokens::misc::BalanceStatus,
			}
			impl ::subxt::events::StaticEvent for ReserveRepatriated {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "ReserveRepatriated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was deposited (e.g. for transaction fees)."]
			pub struct Deposit {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Deposit {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Deposit";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was withdrawn from the account (e.g. for transaction fees)."]
			pub struct Withdraw {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Withdraw {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Withdraw";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was removed from the account (e.g. for misbehavior)."]
			pub struct Slashed {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Slashed {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Slashed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was minted into an account."]
			pub struct Minted {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Minted {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Minted";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was burned from an account."]
			pub struct Burned {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Burned {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Burned";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was suspended from an account (it can be restored later)."]
			pub struct Suspended {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Suspended {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Suspended";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was restored into an account."]
			pub struct Restored {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Restored {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Restored";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was upgraded."]
			pub struct Upgraded {
				pub who: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for Upgraded {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Upgraded";
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Total issuance was increased by `amount`, creating a credit to be balanced."]
			pub struct Issued {
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Issued {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Issued";
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Total issuance was decreased by `amount`, creating a debt to be balanced."]
			pub struct Rescinded {
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Rescinded {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Rescinded";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was locked."]
			pub struct Locked {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Locked {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Locked";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was unlocked."]
			pub struct Unlocked {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Unlocked {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Unlocked";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was frozen."]
			pub struct Frozen {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Frozen {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Frozen";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was thawed."]
			pub struct Thawed {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Thawed {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Thawed";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The total units issued in the system."]
				pub fn total_issuance(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u128,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"TotalIssuance",
						vec![],
						[
							116u8, 70u8, 119u8, 194u8, 69u8, 37u8, 116u8, 206u8, 171u8, 70u8,
							171u8, 210u8, 226u8, 111u8, 184u8, 204u8, 206u8, 11u8, 68u8, 72u8,
							255u8, 19u8, 194u8, 11u8, 27u8, 194u8, 81u8, 204u8, 59u8, 224u8, 202u8,
							185u8,
						],
					)
				}
				#[doc = " The total units of outstanding deactivated balance in the system."]
				pub fn inactive_issuance(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u128,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"InactiveIssuance",
						vec![],
						[
							212u8, 185u8, 19u8, 50u8, 250u8, 72u8, 173u8, 50u8, 4u8, 104u8, 161u8,
							249u8, 77u8, 247u8, 204u8, 248u8, 11u8, 18u8, 57u8, 4u8, 82u8, 110u8,
							30u8, 216u8, 16u8, 37u8, 87u8, 67u8, 189u8, 235u8, 214u8, 155u8,
						],
					)
				}
				#[doc = " The Balances pallet example of storing the balance of an account."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " You can also store the balance of an account in the `System` pallet."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "   type AccountStore = System"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " But this comes with tradeoffs, storing account balances in the system pallet stores"]
				#[doc = " `frame_system` data alongside the account data contrary to storing account balances in the"]
				#[doc = " `Balances` pallet, which uses a `StorageMap` to store balances data only."]
				#[doc = " NOTE: This is only used in the case that this pallet is used to store balances."]
				pub fn account(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Account",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							213u8, 38u8, 200u8, 69u8, 218u8, 0u8, 112u8, 181u8, 160u8, 23u8, 96u8,
							90u8, 3u8, 88u8, 126u8, 22u8, 103u8, 74u8, 64u8, 69u8, 29u8, 247u8,
							18u8, 17u8, 234u8, 143u8, 189u8, 22u8, 247u8, 194u8, 154u8, 249u8,
						],
					)
				}
				#[doc = " The Balances pallet example of storing the balance of an account."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " You can also store the balance of an account in the `System` pallet."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "   type AccountStore = System"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " But this comes with tradeoffs, storing account balances in the system pallet stores"]
				#[doc = " `frame_system` data alongside the account data contrary to storing account balances in the"]
				#[doc = " `Balances` pallet, which uses a `StorageMap` to store balances data only."]
				#[doc = " NOTE: This is only used in the case that this pallet is used to store balances."]
				pub fn account_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Account",
						Vec::new(),
						[
							213u8, 38u8, 200u8, 69u8, 218u8, 0u8, 112u8, 181u8, 160u8, 23u8, 96u8,
							90u8, 3u8, 88u8, 126u8, 22u8, 103u8, 74u8, 64u8, 69u8, 29u8, 247u8,
							18u8, 17u8, 234u8, 143u8, 189u8, 22u8, 247u8, 194u8, 154u8, 249u8,
						],
					)
				}
				#[doc = " Any liquidity locks on some account balances."]
				#[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
				pub fn locks(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
						runtime_types::pallet_balances::types::BalanceLock<::core::primitive::u128>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Locks",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							10u8, 223u8, 55u8, 0u8, 249u8, 69u8, 168u8, 41u8, 75u8, 35u8, 120u8,
							167u8, 18u8, 132u8, 9u8, 20u8, 91u8, 51u8, 27u8, 69u8, 136u8, 187u8,
							13u8, 220u8, 163u8, 122u8, 26u8, 141u8, 174u8, 249u8, 85u8, 37u8,
						],
					)
				}
				#[doc = " Any liquidity locks on some account balances."]
				#[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
				pub fn locks_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
						runtime_types::pallet_balances::types::BalanceLock<::core::primitive::u128>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Locks",
						Vec::new(),
						[
							10u8, 223u8, 55u8, 0u8, 249u8, 69u8, 168u8, 41u8, 75u8, 35u8, 120u8,
							167u8, 18u8, 132u8, 9u8, 20u8, 91u8, 51u8, 27u8, 69u8, 136u8, 187u8,
							13u8, 220u8, 163u8, 122u8, 26u8, 141u8, 174u8, 249u8, 85u8, 37u8,
						],
					)
				}
				#[doc = " Named reserves on some account balances."]
				pub fn reserves(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::ReserveData<
							[::core::primitive::u8; 8usize],
							::core::primitive::u128,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Reserves",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							112u8, 10u8, 241u8, 77u8, 64u8, 187u8, 106u8, 159u8, 13u8, 153u8,
							140u8, 178u8, 182u8, 50u8, 1u8, 55u8, 149u8, 92u8, 196u8, 229u8, 170u8,
							106u8, 193u8, 88u8, 255u8, 244u8, 2u8, 193u8, 62u8, 235u8, 204u8, 91u8,
						],
					)
				}
				#[doc = " Named reserves on some account balances."]
				pub fn reserves_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::ReserveData<
							[::core::primitive::u8; 8usize],
							::core::primitive::u128,
						>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Reserves",
						Vec::new(),
						[
							112u8, 10u8, 241u8, 77u8, 64u8, 187u8, 106u8, 159u8, 13u8, 153u8,
							140u8, 178u8, 182u8, 50u8, 1u8, 55u8, 149u8, 92u8, 196u8, 229u8, 170u8,
							106u8, 193u8, 88u8, 255u8, 244u8, 2u8, 193u8, 62u8, 235u8, 204u8, 91u8,
						],
					)
				}
				#[doc = " Holds on account balances."]
				pub fn holds(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeHoldReason,
							::core::primitive::u128,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Holds",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							186u8, 246u8, 216u8, 4u8, 148u8, 97u8, 53u8, 23u8, 148u8, 240u8, 105u8,
							137u8, 57u8, 191u8, 145u8, 247u8, 11u8, 244u8, 53u8, 73u8, 87u8, 118u8,
							242u8, 126u8, 250u8, 2u8, 154u8, 81u8, 206u8, 166u8, 103u8, 21u8,
						],
					)
				}
				#[doc = " Holds on account balances."]
				pub fn holds_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeHoldReason,
							::core::primitive::u128,
						>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Holds",
						Vec::new(),
						[
							186u8, 246u8, 216u8, 4u8, 148u8, 97u8, 53u8, 23u8, 148u8, 240u8, 105u8,
							137u8, 57u8, 191u8, 145u8, 247u8, 11u8, 244u8, 53u8, 73u8, 87u8, 118u8,
							242u8, 126u8, 250u8, 2u8, 154u8, 81u8, 206u8, 166u8, 103u8, 21u8,
						],
					)
				}
				#[doc = " Freeze locks on account balances."]
				pub fn freezes(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeFreezeReason,
							::core::primitive::u128,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Freezes",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							137u8, 54u8, 103u8, 63u8, 166u8, 153u8, 14u8, 79u8, 7u8, 65u8, 178u8,
							80u8, 204u8, 36u8, 206u8, 69u8, 194u8, 200u8, 174u8, 172u8, 20u8,
							157u8, 156u8, 101u8, 214u8, 98u8, 160u8, 16u8, 102u8, 198u8, 126u8,
							198u8,
						],
					)
				}
				#[doc = " Freeze locks on account balances."]
				pub fn freezes_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeFreezeReason,
							::core::primitive::u128,
						>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Freezes",
						Vec::new(),
						[
							137u8, 54u8, 103u8, 63u8, 166u8, 153u8, 14u8, 79u8, 7u8, 65u8, 178u8,
							80u8, 204u8, 36u8, 206u8, 69u8, 194u8, 200u8, 174u8, 172u8, 20u8,
							157u8, 156u8, 101u8, 214u8, 98u8, 160u8, 16u8, 102u8, 198u8, 126u8,
							198u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The minimum amount required to keep an account open. MUST BE GREATER THAN ZERO!"]
				#[doc = ""]
				#[doc = " If you *really* need it to be zero, you can enable the feature `insecure_zero_ed` for"]
				#[doc = " this pallet. However, you do so at your own risk: this will open up a major DoS vector."]
				#[doc = " In case you have multiple sources of provider references, you may also get unexpected"]
				#[doc = " behaviour if you set this to zero."]
				#[doc = ""]
				#[doc = " Bottom line: Do yourself a favour and make it at least one!"]
				pub fn existential_deposit(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"ArgonBalances",
						"ExistentialDeposit",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The maximum number of locks that should exist on an account."]
				#[doc = " Not strictly enforced, but used for weight estimation."]
				pub fn max_locks(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"ArgonBalances",
						"MaxLocks",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum number of named reserves that can exist on an account."]
				pub fn max_reserves(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"ArgonBalances",
						"MaxReserves",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum number of holds that can exist on an account at any time."]
				pub fn max_holds(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"ArgonBalances",
						"MaxHolds",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum number of individual freeze locks that can exist on an account at any time."]
				pub fn max_freezes(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"ArgonBalances",
						"MaxFreezes",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
			}
		}
	}
	pub mod ulixee_balances {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_balances::pallet::Error2;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_balances::pallet::Call2;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferAllowDeath {
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub value: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferAllowDeath {
					const PALLET: &'static str = "UlixeeBalances";
					const CALL: &'static str = "transfer_allow_death";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceTransfer {
					pub source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub value: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceTransfer {
					const PALLET: &'static str = "UlixeeBalances";
					const CALL: &'static str = "force_transfer";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferKeepAlive {
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub value: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferKeepAlive {
					const PALLET: &'static str = "UlixeeBalances";
					const CALL: &'static str = "transfer_keep_alive";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferAll {
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub keep_alive: ::core::primitive::bool,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferAll {
					const PALLET: &'static str = "UlixeeBalances";
					const CALL: &'static str = "transfer_all";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceUnreserve {
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub amount: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceUnreserve {
					const PALLET: &'static str = "UlixeeBalances";
					const CALL: &'static str = "force_unreserve";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpgradeAccounts {
					pub who: ::std::vec::Vec<::subxt::utils::AccountId32>,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpgradeAccounts {
					const PALLET: &'static str = "UlixeeBalances";
					const CALL: &'static str = "upgrade_accounts";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceSetBalance {
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub new_free: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceSetBalance {
					const PALLET: &'static str = "UlixeeBalances";
					const CALL: &'static str = "force_set_balance";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::transfer_allow_death`]."]
				pub fn transfer_allow_death(
					&self,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					value: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::TransferAllowDeath> {
					::subxt::tx::Payload::new_static(
						"UlixeeBalances",
						"transfer_allow_death",
						types::TransferAllowDeath { dest, value },
						[
							51u8, 166u8, 195u8, 10u8, 139u8, 218u8, 55u8, 130u8, 6u8, 194u8, 35u8,
							140u8, 27u8, 205u8, 214u8, 222u8, 102u8, 43u8, 143u8, 145u8, 86u8,
							219u8, 210u8, 147u8, 13u8, 39u8, 51u8, 21u8, 237u8, 179u8, 132u8,
							130u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_transfer`]."]
				pub fn force_transfer(
					&self,
					source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					value: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ForceTransfer> {
					::subxt::tx::Payload::new_static(
						"UlixeeBalances",
						"force_transfer",
						types::ForceTransfer { source, dest, value },
						[
							154u8, 93u8, 222u8, 27u8, 12u8, 248u8, 63u8, 213u8, 224u8, 86u8, 250u8,
							153u8, 249u8, 102u8, 83u8, 160u8, 79u8, 125u8, 105u8, 222u8, 77u8,
							180u8, 90u8, 105u8, 81u8, 217u8, 60u8, 25u8, 213u8, 51u8, 185u8, 96u8,
						],
					)
				}
				#[doc = "See [`Pallet::transfer_keep_alive`]."]
				pub fn transfer_keep_alive(
					&self,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					value: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::TransferKeepAlive> {
					::subxt::tx::Payload::new_static(
						"UlixeeBalances",
						"transfer_keep_alive",
						types::TransferKeepAlive { dest, value },
						[
							245u8, 14u8, 190u8, 193u8, 32u8, 210u8, 74u8, 92u8, 25u8, 182u8, 76u8,
							55u8, 247u8, 83u8, 114u8, 75u8, 143u8, 236u8, 117u8, 25u8, 54u8, 157u8,
							208u8, 207u8, 233u8, 89u8, 70u8, 161u8, 235u8, 242u8, 222u8, 59u8,
						],
					)
				}
				#[doc = "See [`Pallet::transfer_all`]."]
				pub fn transfer_all(
					&self,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					keep_alive: ::core::primitive::bool,
				) -> ::subxt::tx::Payload<types::TransferAll> {
					::subxt::tx::Payload::new_static(
						"UlixeeBalances",
						"transfer_all",
						types::TransferAll { dest, keep_alive },
						[
							105u8, 132u8, 49u8, 144u8, 195u8, 250u8, 34u8, 46u8, 213u8, 248u8,
							112u8, 188u8, 81u8, 228u8, 136u8, 18u8, 67u8, 172u8, 37u8, 38u8, 238u8,
							9u8, 34u8, 15u8, 67u8, 34u8, 148u8, 195u8, 223u8, 29u8, 154u8, 6u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_unreserve`]."]
				pub fn force_unreserve(
					&self,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					amount: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ForceUnreserve> {
					::subxt::tx::Payload::new_static(
						"UlixeeBalances",
						"force_unreserve",
						types::ForceUnreserve { who, amount },
						[
							142u8, 151u8, 64u8, 205u8, 46u8, 64u8, 62u8, 122u8, 108u8, 49u8, 223u8,
							140u8, 120u8, 153u8, 35u8, 165u8, 187u8, 38u8, 157u8, 200u8, 123u8,
							199u8, 198u8, 168u8, 208u8, 159u8, 39u8, 134u8, 92u8, 103u8, 84u8,
							171u8,
						],
					)
				}
				#[doc = "See [`Pallet::upgrade_accounts`]."]
				pub fn upgrade_accounts(
					&self,
					who: ::std::vec::Vec<::subxt::utils::AccountId32>,
				) -> ::subxt::tx::Payload<types::UpgradeAccounts> {
					::subxt::tx::Payload::new_static(
						"UlixeeBalances",
						"upgrade_accounts",
						types::UpgradeAccounts { who },
						[
							66u8, 200u8, 179u8, 104u8, 65u8, 2u8, 101u8, 56u8, 130u8, 161u8, 224u8,
							233u8, 255u8, 124u8, 70u8, 122u8, 8u8, 49u8, 103u8, 178u8, 68u8, 47u8,
							214u8, 166u8, 217u8, 116u8, 178u8, 50u8, 212u8, 164u8, 98u8, 226u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_set_balance`]."]
				pub fn force_set_balance(
					&self,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					new_free: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ForceSetBalance> {
					::subxt::tx::Payload::new_static(
						"UlixeeBalances",
						"force_set_balance",
						types::ForceSetBalance { who, new_free },
						[
							114u8, 229u8, 59u8, 204u8, 180u8, 83u8, 17u8, 4u8, 59u8, 4u8, 55u8,
							39u8, 151u8, 196u8, 124u8, 60u8, 209u8, 65u8, 193u8, 11u8, 44u8, 164u8,
							116u8, 93u8, 169u8, 30u8, 199u8, 165u8, 55u8, 231u8, 223u8, 43u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_balances::pallet::Event2;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was created with some free balance."]
			pub struct Endowed {
				pub account: ::subxt::utils::AccountId32,
				pub free_balance: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Endowed {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Endowed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was removed whose balance was non-zero but below ExistentialDeposit,"]
			#[doc = "resulting in an outright loss."]
			pub struct DustLost {
				pub account: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for DustLost {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "DustLost";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Transfer succeeded."]
			pub struct Transfer {
				pub from: ::subxt::utils::AccountId32,
				pub to: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Transfer {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Transfer";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A balance was set by root."]
			pub struct BalanceSet {
				pub who: ::subxt::utils::AccountId32,
				pub free: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for BalanceSet {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "BalanceSet";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was reserved (moved from free to reserved)."]
			pub struct Reserved {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Reserved {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Reserved";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was unreserved (moved from reserved to free)."]
			pub struct Unreserved {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Unreserved {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Unreserved";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was moved from the reserve of the first account to the second account."]
			#[doc = "Final argument indicates the destination balance type."]
			pub struct ReserveRepatriated {
				pub from: ::subxt::utils::AccountId32,
				pub to: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
				pub destination_status:
					runtime_types::frame_support::traits::tokens::misc::BalanceStatus,
			}
			impl ::subxt::events::StaticEvent for ReserveRepatriated {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "ReserveRepatriated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was deposited (e.g. for transaction fees)."]
			pub struct Deposit {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Deposit {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Deposit";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was withdrawn from the account (e.g. for transaction fees)."]
			pub struct Withdraw {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Withdraw {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Withdraw";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was removed from the account (e.g. for misbehavior)."]
			pub struct Slashed {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Slashed {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Slashed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was minted into an account."]
			pub struct Minted {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Minted {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Minted";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was burned from an account."]
			pub struct Burned {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Burned {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Burned";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was suspended from an account (it can be restored later)."]
			pub struct Suspended {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Suspended {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Suspended";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was restored into an account."]
			pub struct Restored {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Restored {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Restored";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was upgraded."]
			pub struct Upgraded {
				pub who: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for Upgraded {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Upgraded";
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Total issuance was increased by `amount`, creating a credit to be balanced."]
			pub struct Issued {
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Issued {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Issued";
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Total issuance was decreased by `amount`, creating a debt to be balanced."]
			pub struct Rescinded {
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Rescinded {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Rescinded";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was locked."]
			pub struct Locked {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Locked {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Locked";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was unlocked."]
			pub struct Unlocked {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Unlocked {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Unlocked";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was frozen."]
			pub struct Frozen {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Frozen {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Frozen";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was thawed."]
			pub struct Thawed {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Thawed {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Thawed";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The total units issued in the system."]
				pub fn total_issuance(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u128,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"TotalIssuance",
						vec![],
						[
							116u8, 70u8, 119u8, 194u8, 69u8, 37u8, 116u8, 206u8, 171u8, 70u8,
							171u8, 210u8, 226u8, 111u8, 184u8, 204u8, 206u8, 11u8, 68u8, 72u8,
							255u8, 19u8, 194u8, 11u8, 27u8, 194u8, 81u8, 204u8, 59u8, 224u8, 202u8,
							185u8,
						],
					)
				}
				#[doc = " The total units of outstanding deactivated balance in the system."]
				pub fn inactive_issuance(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u128,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"InactiveIssuance",
						vec![],
						[
							212u8, 185u8, 19u8, 50u8, 250u8, 72u8, 173u8, 50u8, 4u8, 104u8, 161u8,
							249u8, 77u8, 247u8, 204u8, 248u8, 11u8, 18u8, 57u8, 4u8, 82u8, 110u8,
							30u8, 216u8, 16u8, 37u8, 87u8, 67u8, 189u8, 235u8, 214u8, 155u8,
						],
					)
				}
				#[doc = " The Balances pallet example of storing the balance of an account."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " You can also store the balance of an account in the `System` pallet."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "   type AccountStore = System"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " But this comes with tradeoffs, storing account balances in the system pallet stores"]
				#[doc = " `frame_system` data alongside the account data contrary to storing account balances in the"]
				#[doc = " `Balances` pallet, which uses a `StorageMap` to store balances data only."]
				#[doc = " NOTE: This is only used in the case that this pallet is used to store balances."]
				pub fn account(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Account",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							213u8, 38u8, 200u8, 69u8, 218u8, 0u8, 112u8, 181u8, 160u8, 23u8, 96u8,
							90u8, 3u8, 88u8, 126u8, 22u8, 103u8, 74u8, 64u8, 69u8, 29u8, 247u8,
							18u8, 17u8, 234u8, 143u8, 189u8, 22u8, 247u8, 194u8, 154u8, 249u8,
						],
					)
				}
				#[doc = " The Balances pallet example of storing the balance of an account."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " You can also store the balance of an account in the `System` pallet."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "   type AccountStore = System"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " But this comes with tradeoffs, storing account balances in the system pallet stores"]
				#[doc = " `frame_system` data alongside the account data contrary to storing account balances in the"]
				#[doc = " `Balances` pallet, which uses a `StorageMap` to store balances data only."]
				#[doc = " NOTE: This is only used in the case that this pallet is used to store balances."]
				pub fn account_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Account",
						Vec::new(),
						[
							213u8, 38u8, 200u8, 69u8, 218u8, 0u8, 112u8, 181u8, 160u8, 23u8, 96u8,
							90u8, 3u8, 88u8, 126u8, 22u8, 103u8, 74u8, 64u8, 69u8, 29u8, 247u8,
							18u8, 17u8, 234u8, 143u8, 189u8, 22u8, 247u8, 194u8, 154u8, 249u8,
						],
					)
				}
				#[doc = " Any liquidity locks on some account balances."]
				#[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
				pub fn locks(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
						runtime_types::pallet_balances::types::BalanceLock<::core::primitive::u128>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Locks",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							10u8, 223u8, 55u8, 0u8, 249u8, 69u8, 168u8, 41u8, 75u8, 35u8, 120u8,
							167u8, 18u8, 132u8, 9u8, 20u8, 91u8, 51u8, 27u8, 69u8, 136u8, 187u8,
							13u8, 220u8, 163u8, 122u8, 26u8, 141u8, 174u8, 249u8, 85u8, 37u8,
						],
					)
				}
				#[doc = " Any liquidity locks on some account balances."]
				#[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
				pub fn locks_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
						runtime_types::pallet_balances::types::BalanceLock<::core::primitive::u128>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Locks",
						Vec::new(),
						[
							10u8, 223u8, 55u8, 0u8, 249u8, 69u8, 168u8, 41u8, 75u8, 35u8, 120u8,
							167u8, 18u8, 132u8, 9u8, 20u8, 91u8, 51u8, 27u8, 69u8, 136u8, 187u8,
							13u8, 220u8, 163u8, 122u8, 26u8, 141u8, 174u8, 249u8, 85u8, 37u8,
						],
					)
				}
				#[doc = " Named reserves on some account balances."]
				pub fn reserves(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::ReserveData<
							[::core::primitive::u8; 8usize],
							::core::primitive::u128,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Reserves",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							112u8, 10u8, 241u8, 77u8, 64u8, 187u8, 106u8, 159u8, 13u8, 153u8,
							140u8, 178u8, 182u8, 50u8, 1u8, 55u8, 149u8, 92u8, 196u8, 229u8, 170u8,
							106u8, 193u8, 88u8, 255u8, 244u8, 2u8, 193u8, 62u8, 235u8, 204u8, 91u8,
						],
					)
				}
				#[doc = " Named reserves on some account balances."]
				pub fn reserves_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::ReserveData<
							[::core::primitive::u8; 8usize],
							::core::primitive::u128,
						>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Reserves",
						Vec::new(),
						[
							112u8, 10u8, 241u8, 77u8, 64u8, 187u8, 106u8, 159u8, 13u8, 153u8,
							140u8, 178u8, 182u8, 50u8, 1u8, 55u8, 149u8, 92u8, 196u8, 229u8, 170u8,
							106u8, 193u8, 88u8, 255u8, 244u8, 2u8, 193u8, 62u8, 235u8, 204u8, 91u8,
						],
					)
				}
				#[doc = " Holds on account balances."]
				pub fn holds(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeHoldReason,
							::core::primitive::u128,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Holds",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							186u8, 246u8, 216u8, 4u8, 148u8, 97u8, 53u8, 23u8, 148u8, 240u8, 105u8,
							137u8, 57u8, 191u8, 145u8, 247u8, 11u8, 244u8, 53u8, 73u8, 87u8, 118u8,
							242u8, 126u8, 250u8, 2u8, 154u8, 81u8, 206u8, 166u8, 103u8, 21u8,
						],
					)
				}
				#[doc = " Holds on account balances."]
				pub fn holds_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeHoldReason,
							::core::primitive::u128,
						>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Holds",
						Vec::new(),
						[
							186u8, 246u8, 216u8, 4u8, 148u8, 97u8, 53u8, 23u8, 148u8, 240u8, 105u8,
							137u8, 57u8, 191u8, 145u8, 247u8, 11u8, 244u8, 53u8, 73u8, 87u8, 118u8,
							242u8, 126u8, 250u8, 2u8, 154u8, 81u8, 206u8, 166u8, 103u8, 21u8,
						],
					)
				}
				#[doc = " Freeze locks on account balances."]
				pub fn freezes(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeFreezeReason,
							::core::primitive::u128,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Freezes",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							137u8, 54u8, 103u8, 63u8, 166u8, 153u8, 14u8, 79u8, 7u8, 65u8, 178u8,
							80u8, 204u8, 36u8, 206u8, 69u8, 194u8, 200u8, 174u8, 172u8, 20u8,
							157u8, 156u8, 101u8, 214u8, 98u8, 160u8, 16u8, 102u8, 198u8, 126u8,
							198u8,
						],
					)
				}
				#[doc = " Freeze locks on account balances."]
				pub fn freezes_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeFreezeReason,
							::core::primitive::u128,
						>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Freezes",
						Vec::new(),
						[
							137u8, 54u8, 103u8, 63u8, 166u8, 153u8, 14u8, 79u8, 7u8, 65u8, 178u8,
							80u8, 204u8, 36u8, 206u8, 69u8, 194u8, 200u8, 174u8, 172u8, 20u8,
							157u8, 156u8, 101u8, 214u8, 98u8, 160u8, 16u8, 102u8, 198u8, 126u8,
							198u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The minimum amount required to keep an account open. MUST BE GREATER THAN ZERO!"]
				#[doc = ""]
				#[doc = " If you *really* need it to be zero, you can enable the feature `insecure_zero_ed` for"]
				#[doc = " this pallet. However, you do so at your own risk: this will open up a major DoS vector."]
				#[doc = " In case you have multiple sources of provider references, you may also get unexpected"]
				#[doc = " behaviour if you set this to zero."]
				#[doc = ""]
				#[doc = " Bottom line: Do yourself a favour and make it at least one!"]
				pub fn existential_deposit(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"UlixeeBalances",
						"ExistentialDeposit",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The maximum number of locks that should exist on an account."]
				#[doc = " Not strictly enforced, but used for weight estimation."]
				pub fn max_locks(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"UlixeeBalances",
						"MaxLocks",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum number of named reserves that can exist on an account."]
				pub fn max_reserves(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"UlixeeBalances",
						"MaxReserves",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum number of holds that can exist on an account at any time."]
				pub fn max_holds(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"UlixeeBalances",
						"MaxHolds",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum number of individual freeze locks that can exist on an account at any time."]
				pub fn max_freezes(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"UlixeeBalances",
						"MaxFreezes",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
			}
		}
	}
	pub mod tx_pause {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_tx_pause::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_tx_pause::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Pause {
					pub full_name: (
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					),
				}
				impl ::subxt::blocks::StaticExtrinsic for Pause {
					const PALLET: &'static str = "TxPause";
					const CALL: &'static str = "pause";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Unpause {
					pub ident: (
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					),
				}
				impl ::subxt::blocks::StaticExtrinsic for Unpause {
					const PALLET: &'static str = "TxPause";
					const CALL: &'static str = "unpause";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::pause`]."]
				pub fn pause(
					&self,
					full_name: (
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					),
				) -> ::subxt::tx::Payload<types::Pause> {
					::subxt::tx::Payload::new_static(
						"TxPause",
						"pause",
						types::Pause { full_name },
						[
							244u8, 112u8, 104u8, 148u8, 17u8, 164u8, 228u8, 229u8, 103u8, 212u8,
							137u8, 16u8, 194u8, 167u8, 150u8, 148u8, 151u8, 233u8, 15u8, 2u8, 54u8,
							96u8, 158u8, 43u8, 222u8, 128u8, 199u8, 87u8, 74u8, 38u8, 6u8, 215u8,
						],
					)
				}
				#[doc = "See [`Pallet::unpause`]."]
				pub fn unpause(
					&self,
					ident: (
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					),
				) -> ::subxt::tx::Payload<types::Unpause> {
					::subxt::tx::Payload::new_static(
						"TxPause",
						"unpause",
						types::Unpause { ident },
						[
							213u8, 245u8, 75u8, 131u8, 24u8, 188u8, 101u8, 168u8, 39u8, 246u8,
							228u8, 155u8, 255u8, 146u8, 245u8, 218u8, 68u8, 102u8, 75u8, 133u8,
							54u8, 142u8, 191u8, 87u8, 148u8, 59u8, 99u8, 11u8, 33u8, 184u8, 24u8,
							179u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_tx_pause::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "This pallet, or a specific call is now paused."]
			pub struct CallPaused {
				pub full_name: (
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
				),
			}
			impl ::subxt::events::StaticEvent for CallPaused {
				const PALLET: &'static str = "TxPause";
				const EVENT: &'static str = "CallPaused";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "This pallet, or a specific call is now unpaused."]
			pub struct CallUnpaused {
				pub full_name: (
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
				),
			}
			impl ::subxt::events::StaticEvent for CallUnpaused {
				const PALLET: &'static str = "TxPause";
				const EVENT: &'static str = "CallUnpaused";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The set of calls that are explicitly paused."]
				pub fn paused_calls(
					&self,
					_0: impl ::std::borrow::Borrow<
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					>,
					_1: impl ::std::borrow::Borrow<
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					(),
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TxPause",
						"PausedCalls",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							36u8, 9u8, 29u8, 154u8, 39u8, 47u8, 237u8, 97u8, 176u8, 241u8, 153u8,
							131u8, 20u8, 16u8, 73u8, 63u8, 27u8, 21u8, 107u8, 5u8, 147u8, 198u8,
							82u8, 212u8, 38u8, 162u8, 1u8, 203u8, 57u8, 187u8, 53u8, 132u8,
						],
					)
				}
				#[doc = " The set of calls that are explicitly paused."]
				pub fn paused_calls_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					(),
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TxPause",
						"PausedCalls",
						Vec::new(),
						[
							36u8, 9u8, 29u8, 154u8, 39u8, 47u8, 237u8, 97u8, 176u8, 241u8, 153u8,
							131u8, 20u8, 16u8, 73u8, 63u8, 27u8, 21u8, 107u8, 5u8, 147u8, 198u8,
							82u8, 212u8, 38u8, 162u8, 1u8, 203u8, 57u8, 187u8, 53u8, 132u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " Maximum length for pallet name and call name SCALE encoded string names."]
				#[doc = ""]
				#[doc = " TOO LONG NAMES WILL BE TREATED AS PAUSED."]
				pub fn max_name_len(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"TxPause",
						"MaxNameLen",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
			}
		}
	}
	pub mod transaction_payment {
		use super::{root_mod, runtime_types};
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_transaction_payment::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,"]
			#[doc = "has been paid by `who`."]
			pub struct TransactionFeePaid {
				pub who: ::subxt::utils::AccountId32,
				pub actual_fee: ::core::primitive::u128,
				pub tip: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for TransactionFeePaid {
				const PALLET: &'static str = "TransactionPayment";
				const EVENT: &'static str = "TransactionFeePaid";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				pub fn next_fee_multiplier(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::sp_arithmetic::fixed_point::FixedU128,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"TransactionPayment",
						"NextFeeMultiplier",
						vec![],
						[
							247u8, 39u8, 81u8, 170u8, 225u8, 226u8, 82u8, 147u8, 34u8, 113u8,
							147u8, 213u8, 59u8, 80u8, 139u8, 35u8, 36u8, 196u8, 152u8, 19u8, 9u8,
							159u8, 176u8, 79u8, 249u8, 201u8, 170u8, 1u8, 129u8, 79u8, 146u8,
							197u8,
						],
					)
				}
				pub fn storage_version(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_transaction_payment::Releases,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"TransactionPayment",
						"StorageVersion",
						vec![],
						[
							105u8, 243u8, 158u8, 241u8, 159u8, 231u8, 253u8, 6u8, 4u8, 32u8, 85u8,
							178u8, 126u8, 31u8, 203u8, 134u8, 154u8, 38u8, 122u8, 155u8, 150u8,
							251u8, 174u8, 15u8, 74u8, 134u8, 216u8, 244u8, 168u8, 175u8, 158u8,
							144u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " A fee multiplier for `Operational` extrinsics to compute \"virtual tip\" to boost their"]
				#[doc = " `priority`"]
				#[doc = ""]
				#[doc = " This value is multiplied by the `final_fee` to obtain a \"virtual tip\" that is later"]
				#[doc = " added to a tip component in regular `priority` calculations."]
				#[doc = " It means that a `Normal` transaction can front-run a similarly-sized `Operational`"]
				#[doc = " extrinsic (with no tip), by including a tip value greater than the virtual tip."]
				#[doc = ""]
				#[doc = " ```rust,ignore"]
				#[doc = " // For `Normal`"]
				#[doc = " let priority = priority_calc(tip);"]
				#[doc = ""]
				#[doc = " // For `Operational`"]
				#[doc = " let virtual_tip = (inclusion_fee + tip) * OperationalFeeMultiplier;"]
				#[doc = " let priority = priority_calc(tip + virtual_tip);"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " Note that since we use `final_fee` the multiplier applies also to the regular `tip`"]
				#[doc = " sent with the transaction. So, not only does the transaction get a priority bump based"]
				#[doc = " on the `inclusion_fee`, but we also amplify the impact of tips applied to `Operational`"]
				#[doc = " transactions."]
				pub fn operational_fee_multiplier(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u8> {
					::subxt::constants::Address::new_static(
						"TransactionPayment",
						"OperationalFeeMultiplier",
						[
							141u8, 130u8, 11u8, 35u8, 226u8, 114u8, 92u8, 179u8, 168u8, 110u8,
							28u8, 91u8, 221u8, 64u8, 4u8, 148u8, 201u8, 193u8, 185u8, 66u8, 226u8,
							114u8, 97u8, 79u8, 62u8, 212u8, 202u8, 114u8, 237u8, 228u8, 183u8,
							165u8,
						],
					)
				}
			}
		}
	}
	pub mod sudo {
		use super::{root_mod, runtime_types};
		#[doc = "Error for the Sudo pallet."]
		pub type Error = runtime_types::pallet_sudo::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_sudo::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Sudo {
					pub call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Sudo {
					const PALLET: &'static str = "Sudo";
					const CALL: &'static str = "sudo";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SudoUncheckedWeight {
					pub call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall>,
					pub weight: runtime_types::sp_weights::weight_v2::Weight,
				}
				impl ::subxt::blocks::StaticExtrinsic for SudoUncheckedWeight {
					const PALLET: &'static str = "Sudo";
					const CALL: &'static str = "sudo_unchecked_weight";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetKey {
					pub new: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetKey {
					const PALLET: &'static str = "Sudo";
					const CALL: &'static str = "set_key";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SudoAs {
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SudoAs {
					const PALLET: &'static str = "Sudo";
					const CALL: &'static str = "sudo_as";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RemoveKey;
				impl ::subxt::blocks::StaticExtrinsic for RemoveKey {
					const PALLET: &'static str = "Sudo";
					const CALL: &'static str = "remove_key";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::sudo`]."]
				pub fn sudo(
					&self,
					call: runtime_types::ulx_node_runtime::RuntimeCall,
				) -> ::subxt::tx::Payload<types::Sudo> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"sudo",
						types::Sudo { call: ::std::boxed::Box::new(call) },
						[
							195u8, 146u8, 194u8, 245u8, 197u8, 24u8, 146u8, 65u8, 23u8, 104u8,
							217u8, 10u8, 158u8, 158u8, 153u8, 88u8, 157u8, 114u8, 88u8, 173u8,
							171u8, 155u8, 201u8, 196u8, 88u8, 207u8, 56u8, 249u8, 133u8, 32u8,
							225u8, 49u8,
						],
					)
				}
				#[doc = "See [`Pallet::sudo_unchecked_weight`]."]
				pub fn sudo_unchecked_weight(
					&self,
					call: runtime_types::ulx_node_runtime::RuntimeCall,
					weight: runtime_types::sp_weights::weight_v2::Weight,
				) -> ::subxt::tx::Payload<types::SudoUncheckedWeight> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"sudo_unchecked_weight",
						types::SudoUncheckedWeight { call: ::std::boxed::Box::new(call), weight },
						[
							38u8, 117u8, 170u8, 136u8, 86u8, 228u8, 112u8, 86u8, 51u8, 210u8,
							190u8, 212u8, 169u8, 96u8, 215u8, 9u8, 178u8, 215u8, 229u8, 116u8,
							70u8, 96u8, 167u8, 197u8, 19u8, 34u8, 101u8, 141u8, 115u8, 220u8, 50u8,
							42u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_key`]."]
				pub fn set_key(
					&self,
					new: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				) -> ::subxt::tx::Payload<types::SetKey> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"set_key",
						types::SetKey { new },
						[
							9u8, 73u8, 39u8, 205u8, 188u8, 127u8, 143u8, 54u8, 128u8, 94u8, 8u8,
							227u8, 197u8, 44u8, 70u8, 93u8, 228u8, 196u8, 64u8, 165u8, 226u8,
							158u8, 101u8, 192u8, 22u8, 193u8, 102u8, 84u8, 21u8, 35u8, 92u8, 198u8,
						],
					)
				}
				#[doc = "See [`Pallet::sudo_as`]."]
				pub fn sudo_as(
					&self,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					call: runtime_types::ulx_node_runtime::RuntimeCall,
				) -> ::subxt::tx::Payload<types::SudoAs> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"sudo_as",
						types::SudoAs { who, call: ::std::boxed::Box::new(call) },
						[
							155u8, 246u8, 195u8, 168u8, 138u8, 162u8, 189u8, 59u8, 183u8, 207u8,
							160u8, 67u8, 147u8, 138u8, 12u8, 250u8, 64u8, 60u8, 141u8, 22u8, 145u8,
							190u8, 7u8, 54u8, 9u8, 197u8, 202u8, 6u8, 207u8, 72u8, 215u8, 42u8,
						],
					)
				}
				#[doc = "See [`Pallet::remove_key`]."]
				pub fn remove_key(&self) -> ::subxt::tx::Payload<types::RemoveKey> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"remove_key",
						types::RemoveKey {},
						[
							133u8, 253u8, 54u8, 175u8, 202u8, 239u8, 5u8, 198u8, 180u8, 138u8,
							25u8, 28u8, 109u8, 40u8, 30u8, 56u8, 126u8, 100u8, 52u8, 205u8, 250u8,
							191u8, 61u8, 195u8, 172u8, 142u8, 184u8, 239u8, 247u8, 10u8, 211u8,
							79u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_sudo::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A sudo call just took place."]
			pub struct Sudid {
				pub sudo_result:
					::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
			}
			impl ::subxt::events::StaticEvent for Sudid {
				const PALLET: &'static str = "Sudo";
				const EVENT: &'static str = "Sudid";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The sudo key has been updated."]
			pub struct KeyChanged {
				pub old: ::core::option::Option<::subxt::utils::AccountId32>,
				pub new: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for KeyChanged {
				const PALLET: &'static str = "Sudo";
				const EVENT: &'static str = "KeyChanged";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The key was permanently removed."]
			pub struct KeyRemoved;
			impl ::subxt::events::StaticEvent for KeyRemoved {
				const PALLET: &'static str = "Sudo";
				const EVENT: &'static str = "KeyRemoved";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A [sudo_as](Pallet::sudo_as) call just took place."]
			pub struct SudoAsDone {
				pub sudo_result:
					::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
			}
			impl ::subxt::events::StaticEvent for SudoAsDone {
				const PALLET: &'static str = "Sudo";
				const EVENT: &'static str = "SudoAsDone";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The `AccountId` of the sudo key."]
				pub fn key(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::AccountId32,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Sudo",
						"Key",
						vec![],
						[
							72u8, 14u8, 225u8, 162u8, 205u8, 247u8, 227u8, 105u8, 116u8, 57u8, 4u8,
							31u8, 84u8, 137u8, 227u8, 228u8, 133u8, 245u8, 206u8, 227u8, 117u8,
							36u8, 252u8, 151u8, 107u8, 15u8, 180u8, 4u8, 4u8, 152u8, 195u8, 144u8,
						],
					)
				}
			}
		}
	}
	pub mod runtime_types {
		use super::runtime_types;
		pub mod bounded_collections {
			use super::runtime_types;
			pub mod bounded_btree_map {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BoundedBTreeMap<_0, _1>(pub ::subxt::utils::KeyedVec<_0, _1>);
			}
			pub mod bounded_vec {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					serde :: Serialize,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[serde(transparent)]
				pub struct BoundedVec<_0>(pub ::std::vec::Vec<_0>);
			}
			pub mod weak_bounded_vec {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct WeakBoundedVec<_0>(pub ::std::vec::Vec<_0>);
			}
		}
		pub mod finality_grandpa {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct Equivocation<_0, _1, _2> {
				pub round_number: ::core::primitive::u64,
				pub identity: _0,
				pub first: (_1, _2),
				pub second: (_1, _2),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct Precommit<_0, _1> {
				pub target_hash: _0,
				pub target_number: _1,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct Prevote<_0, _1> {
				pub target_hash: _0,
				pub target_number: _1,
			}
		}
		pub mod frame_support {
			use super::runtime_types;
			pub mod dispatch {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum DispatchClass {
					#[codec(index = 0)]
					Normal,
					#[codec(index = 1)]
					Operational,
					#[codec(index = 2)]
					Mandatory,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DispatchInfo {
					pub weight: runtime_types::sp_weights::weight_v2::Weight,
					pub class: runtime_types::frame_support::dispatch::DispatchClass,
					pub pays_fee: runtime_types::frame_support::dispatch::Pays,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Pays {
					#[codec(index = 0)]
					Yes,
					#[codec(index = 1)]
					No,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct PerDispatchClass<_0> {
					pub normal: _0,
					pub operational: _0,
					pub mandatory: _0,
				}
			}
			pub mod traits {
				use super::runtime_types;
				pub mod tokens {
					use super::runtime_types;
					pub mod misc {
						use super::runtime_types;
						#[derive(
							:: subxt :: ext :: codec :: Decode,
							:: subxt :: ext :: codec :: Encode,
							:: subxt :: ext :: scale_decode :: DecodeAsType,
							:: subxt :: ext :: scale_encode :: EncodeAsType,
							Clone,
							Debug,
						)]
						# [codec (crate = :: subxt :: ext :: codec)]
						#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
						#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
						pub enum BalanceStatus {
							#[codec(index = 0)]
							Free,
							#[codec(index = 1)]
							Reserved,
						}
					}
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct PalletId(pub [::core::primitive::u8; 8usize]);
		}
		pub mod frame_system {
			use super::runtime_types;
			pub mod extensions {
				use super::runtime_types;
				pub mod check_genesis {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckGenesis;
				}
				pub mod check_mortality {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckMortality(pub runtime_types::sp_runtime::generic::era::Era);
				}
				pub mod check_non_zero_sender {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckNonZeroSender;
				}
				pub mod check_nonce {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckNonce(#[codec(compact)] pub ::core::primitive::u32);
				}
				pub mod check_spec_version {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckSpecVersion;
				}
				pub mod check_tx_version {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckTxVersion;
				}
				pub mod check_weight {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckWeight;
				}
			}
			pub mod limits {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BlockLength {
					pub max: runtime_types::frame_support::dispatch::PerDispatchClass<
						::core::primitive::u32,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BlockWeights {
					pub base_block: runtime_types::sp_weights::weight_v2::Weight,
					pub max_block: runtime_types::sp_weights::weight_v2::Weight,
					pub per_class: runtime_types::frame_support::dispatch::PerDispatchClass<
						runtime_types::frame_system::limits::WeightsPerClass,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct WeightsPerClass {
					pub base_extrinsic: runtime_types::sp_weights::weight_v2::Weight,
					pub max_extrinsic:
						::core::option::Option<runtime_types::sp_weights::weight_v2::Weight>,
					pub max_total:
						::core::option::Option<runtime_types::sp_weights::weight_v2::Weight>,
					pub reserved:
						::core::option::Option<runtime_types::sp_weights::weight_v2::Weight>,
				}
			}
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::remark`]."]
					remark { remark: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 1)]
					#[doc = "See [`Pallet::set_heap_pages`]."]
					set_heap_pages { pages: ::core::primitive::u64 },
					#[codec(index = 2)]
					#[doc = "See [`Pallet::set_code`]."]
					set_code { code: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 3)]
					#[doc = "See [`Pallet::set_code_without_checks`]."]
					set_code_without_checks { code: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 4)]
					#[doc = "See [`Pallet::set_storage`]."]
					set_storage {
						items: ::std::vec::Vec<(
							::std::vec::Vec<::core::primitive::u8>,
							::std::vec::Vec<::core::primitive::u8>,
						)>,
					},
					#[codec(index = 5)]
					#[doc = "See [`Pallet::kill_storage`]."]
					kill_storage { keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>> },
					#[codec(index = 6)]
					#[doc = "See [`Pallet::kill_prefix`]."]
					kill_prefix {
						prefix: ::std::vec::Vec<::core::primitive::u8>,
						subkeys: ::core::primitive::u32,
					},
					#[codec(index = 7)]
					#[doc = "See [`Pallet::remark_with_event`]."]
					remark_with_event { remark: ::std::vec::Vec<::core::primitive::u8> },
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Error for the System pallet"]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "The name of specification does not match between the current runtime"]
					#[doc = "and the new runtime."]
					InvalidSpecName,
					#[codec(index = 1)]
					#[doc = "The specification version is not allowed to decrease between the current runtime"]
					#[doc = "and the new runtime."]
					SpecVersionNeedsToIncrease,
					#[codec(index = 2)]
					#[doc = "Failed to extract the runtime version from the new runtime."]
					#[doc = ""]
					#[doc = "Either calling `Core_version` or decoding `RuntimeVersion` failed."]
					FailedToExtractRuntimeVersion,
					#[codec(index = 3)]
					#[doc = "Suicide called when the account has non-default composite data."]
					NonDefaultComposite,
					#[codec(index = 4)]
					#[doc = "There is a non-zero reference count preventing the account from being purged."]
					NonZeroRefCount,
					#[codec(index = 5)]
					#[doc = "The origin filter prevent the call to be dispatched."]
					CallFiltered,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Event for the System pallet."]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "An extrinsic completed successfully."]
					ExtrinsicSuccess {
						dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
					},
					#[codec(index = 1)]
					#[doc = "An extrinsic failed."]
					ExtrinsicFailed {
						dispatch_error: runtime_types::sp_runtime::DispatchError,
						dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
					},
					#[codec(index = 2)]
					#[doc = "`:code` was updated."]
					CodeUpdated,
					#[codec(index = 3)]
					#[doc = "A new account was created."]
					NewAccount { account: ::subxt::utils::AccountId32 },
					#[codec(index = 4)]
					#[doc = "An account was reaped."]
					KilledAccount { account: ::subxt::utils::AccountId32 },
					#[codec(index = 5)]
					#[doc = "On on-chain remark happened."]
					Remarked { sender: ::subxt::utils::AccountId32, hash: ::subxt::utils::H256 },
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct AccountInfo<_0, _1> {
				pub nonce: _0,
				pub consumers: ::core::primitive::u32,
				pub providers: ::core::primitive::u32,
				pub sufficients: ::core::primitive::u32,
				pub data: _1,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct EventRecord<_0, _1> {
				pub phase: runtime_types::frame_system::Phase,
				pub event: _0,
				pub topics: ::std::vec::Vec<_1>,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct LastRuntimeUpgradeInfo {
				#[codec(compact)]
				pub spec_version: ::core::primitive::u32,
				pub spec_name: ::std::string::String,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum Phase {
				#[codec(index = 0)]
				ApplyExtrinsic(::core::primitive::u32),
				#[codec(index = 1)]
				Finalization,
				#[codec(index = 2)]
				Initialization,
			}
		}
		pub mod pallet_balances {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::transfer_allow_death`]."]
					transfer_allow_death {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::force_transfer`]."]
					force_transfer {
						source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 3)]
					#[doc = "See [`Pallet::transfer_keep_alive`]."]
					transfer_keep_alive {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 4)]
					#[doc = "See [`Pallet::transfer_all`]."]
					transfer_all {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						keep_alive: ::core::primitive::bool,
					},
					#[codec(index = 5)]
					#[doc = "See [`Pallet::force_unreserve`]."]
					force_unreserve {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 6)]
					#[doc = "See [`Pallet::upgrade_accounts`]."]
					upgrade_accounts { who: ::std::vec::Vec<::subxt::utils::AccountId32> },
					#[codec(index = 8)]
					#[doc = "See [`Pallet::force_set_balance`]."]
					force_set_balance {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						new_free: ::core::primitive::u128,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call2 {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::transfer_allow_death`]."]
					transfer_allow_death {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::force_transfer`]."]
					force_transfer {
						source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 3)]
					#[doc = "See [`Pallet::transfer_keep_alive`]."]
					transfer_keep_alive {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 4)]
					#[doc = "See [`Pallet::transfer_all`]."]
					transfer_all {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						keep_alive: ::core::primitive::bool,
					},
					#[codec(index = 5)]
					#[doc = "See [`Pallet::force_unreserve`]."]
					force_unreserve {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 6)]
					#[doc = "See [`Pallet::upgrade_accounts`]."]
					upgrade_accounts { who: ::std::vec::Vec<::subxt::utils::AccountId32> },
					#[codec(index = 8)]
					#[doc = "See [`Pallet::force_set_balance`]."]
					force_set_balance {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						new_free: ::core::primitive::u128,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Vesting balance too high to send value."]
					VestingBalance,
					#[codec(index = 1)]
					#[doc = "Account liquidity restrictions prevent withdrawal."]
					LiquidityRestrictions,
					#[codec(index = 2)]
					#[doc = "Balance too low to send value."]
					InsufficientBalance,
					#[codec(index = 3)]
					#[doc = "Value too low to create account due to existential deposit."]
					ExistentialDeposit,
					#[codec(index = 4)]
					#[doc = "Transfer/payment would kill account."]
					Expendability,
					#[codec(index = 5)]
					#[doc = "A vesting schedule already exists for this account."]
					ExistingVestingSchedule,
					#[codec(index = 6)]
					#[doc = "Beneficiary account must pre-exist."]
					DeadAccount,
					#[codec(index = 7)]
					#[doc = "Number of named reserves exceed `MaxReserves`."]
					TooManyReserves,
					#[codec(index = 8)]
					#[doc = "Number of holds exceed `MaxHolds`."]
					TooManyHolds,
					#[codec(index = 9)]
					#[doc = "Number of freezes exceed `MaxFreezes`."]
					TooManyFreezes,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error2 {
					#[codec(index = 0)]
					#[doc = "Vesting balance too high to send value."]
					VestingBalance,
					#[codec(index = 1)]
					#[doc = "Account liquidity restrictions prevent withdrawal."]
					LiquidityRestrictions,
					#[codec(index = 2)]
					#[doc = "Balance too low to send value."]
					InsufficientBalance,
					#[codec(index = 3)]
					#[doc = "Value too low to create account due to existential deposit."]
					ExistentialDeposit,
					#[codec(index = 4)]
					#[doc = "Transfer/payment would kill account."]
					Expendability,
					#[codec(index = 5)]
					#[doc = "A vesting schedule already exists for this account."]
					ExistingVestingSchedule,
					#[codec(index = 6)]
					#[doc = "Beneficiary account must pre-exist."]
					DeadAccount,
					#[codec(index = 7)]
					#[doc = "Number of named reserves exceed `MaxReserves`."]
					TooManyReserves,
					#[codec(index = 8)]
					#[doc = "Number of holds exceed `MaxHolds`."]
					TooManyHolds,
					#[codec(index = 9)]
					#[doc = "Number of freezes exceed `MaxFreezes`."]
					TooManyFreezes,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "An account was created with some free balance."]
					Endowed {
						account: ::subxt::utils::AccountId32,
						free_balance: ::core::primitive::u128,
					},
					#[codec(index = 1)]
					#[doc = "An account was removed whose balance was non-zero but below ExistentialDeposit,"]
					#[doc = "resulting in an outright loss."]
					DustLost {
						account: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 2)]
					#[doc = "Transfer succeeded."]
					Transfer {
						from: ::subxt::utils::AccountId32,
						to: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 3)]
					#[doc = "A balance was set by root."]
					BalanceSet { who: ::subxt::utils::AccountId32, free: ::core::primitive::u128 },
					#[codec(index = 4)]
					#[doc = "Some balance was reserved (moved from free to reserved)."]
					Reserved { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 5)]
					#[doc = "Some balance was unreserved (moved from reserved to free)."]
					Unreserved { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 6)]
					#[doc = "Some balance was moved from the reserve of the first account to the second account."]
					#[doc = "Final argument indicates the destination balance type."]
					ReserveRepatriated {
						from: ::subxt::utils::AccountId32,
						to: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						destination_status:
							runtime_types::frame_support::traits::tokens::misc::BalanceStatus,
					},
					#[codec(index = 7)]
					#[doc = "Some amount was deposited (e.g. for transaction fees)."]
					Deposit { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 8)]
					#[doc = "Some amount was withdrawn from the account (e.g. for transaction fees)."]
					Withdraw { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 9)]
					#[doc = "Some amount was removed from the account (e.g. for misbehavior)."]
					Slashed { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 10)]
					#[doc = "Some amount was minted into an account."]
					Minted { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 11)]
					#[doc = "Some amount was burned from an account."]
					Burned { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 12)]
					#[doc = "Some amount was suspended from an account (it can be restored later)."]
					Suspended { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 13)]
					#[doc = "Some amount was restored into an account."]
					Restored { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 14)]
					#[doc = "An account was upgraded."]
					Upgraded { who: ::subxt::utils::AccountId32 },
					#[codec(index = 15)]
					#[doc = "Total issuance was increased by `amount`, creating a credit to be balanced."]
					Issued { amount: ::core::primitive::u128 },
					#[codec(index = 16)]
					#[doc = "Total issuance was decreased by `amount`, creating a debt to be balanced."]
					Rescinded { amount: ::core::primitive::u128 },
					#[codec(index = 17)]
					#[doc = "Some balance was locked."]
					Locked { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 18)]
					#[doc = "Some balance was unlocked."]
					Unlocked { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 19)]
					#[doc = "Some balance was frozen."]
					Frozen { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 20)]
					#[doc = "Some balance was thawed."]
					Thawed { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event2 {
					#[codec(index = 0)]
					#[doc = "An account was created with some free balance."]
					Endowed {
						account: ::subxt::utils::AccountId32,
						free_balance: ::core::primitive::u128,
					},
					#[codec(index = 1)]
					#[doc = "An account was removed whose balance was non-zero but below ExistentialDeposit,"]
					#[doc = "resulting in an outright loss."]
					DustLost {
						account: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 2)]
					#[doc = "Transfer succeeded."]
					Transfer {
						from: ::subxt::utils::AccountId32,
						to: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 3)]
					#[doc = "A balance was set by root."]
					BalanceSet { who: ::subxt::utils::AccountId32, free: ::core::primitive::u128 },
					#[codec(index = 4)]
					#[doc = "Some balance was reserved (moved from free to reserved)."]
					Reserved { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 5)]
					#[doc = "Some balance was unreserved (moved from reserved to free)."]
					Unreserved { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 6)]
					#[doc = "Some balance was moved from the reserve of the first account to the second account."]
					#[doc = "Final argument indicates the destination balance type."]
					ReserveRepatriated {
						from: ::subxt::utils::AccountId32,
						to: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						destination_status:
							runtime_types::frame_support::traits::tokens::misc::BalanceStatus,
					},
					#[codec(index = 7)]
					#[doc = "Some amount was deposited (e.g. for transaction fees)."]
					Deposit { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 8)]
					#[doc = "Some amount was withdrawn from the account (e.g. for transaction fees)."]
					Withdraw { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 9)]
					#[doc = "Some amount was removed from the account (e.g. for misbehavior)."]
					Slashed { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 10)]
					#[doc = "Some amount was minted into an account."]
					Minted { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 11)]
					#[doc = "Some amount was burned from an account."]
					Burned { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 12)]
					#[doc = "Some amount was suspended from an account (it can be restored later)."]
					Suspended { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 13)]
					#[doc = "Some amount was restored into an account."]
					Restored { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 14)]
					#[doc = "An account was upgraded."]
					Upgraded { who: ::subxt::utils::AccountId32 },
					#[codec(index = 15)]
					#[doc = "Total issuance was increased by `amount`, creating a credit to be balanced."]
					Issued { amount: ::core::primitive::u128 },
					#[codec(index = 16)]
					#[doc = "Total issuance was decreased by `amount`, creating a debt to be balanced."]
					Rescinded { amount: ::core::primitive::u128 },
					#[codec(index = 17)]
					#[doc = "Some balance was locked."]
					Locked { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 18)]
					#[doc = "Some balance was unlocked."]
					Unlocked { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 19)]
					#[doc = "Some balance was frozen."]
					Frozen { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 20)]
					#[doc = "Some balance was thawed."]
					Thawed { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AccountData<_0> {
					pub free: _0,
					pub reserved: _0,
					pub frozen: _0,
					pub flags: runtime_types::pallet_balances::types::ExtraFlags,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BalanceLock<_0> {
					pub id: [::core::primitive::u8; 8usize],
					pub amount: _0,
					pub reasons: runtime_types::pallet_balances::types::Reasons,
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ExtraFlags(pub ::core::primitive::u128);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct IdAmount<_0, _1> {
					pub id: _0,
					pub amount: _1,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Reasons {
					#[codec(index = 0)]
					Fee,
					#[codec(index = 1)]
					Misc,
					#[codec(index = 2)]
					All,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ReserveData<_0, _1> {
					pub id: _0,
					pub amount: _1,
				}
			}
		}
		pub mod pallet_block_rewards {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BlockPayout<_0, _1> {
					pub account_id: _0,
					pub ulixees: _1,
					pub argons: _1,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					RewardCreated {
						maturation_block: ::core::primitive::u32,
						rewards: ::std::vec::Vec<
							runtime_types::pallet_block_rewards::pallet::BlockPayout<
								::subxt::utils::AccountId32,
								::core::primitive::u128,
							>,
						>,
					},
					#[codec(index = 1)]
					RewardUnlocked {
						rewards: ::std::vec::Vec<
							runtime_types::pallet_block_rewards::pallet::BlockPayout<
								::subxt::utils::AccountId32,
								::core::primitive::u128,
							>,
						>,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum FreezeReason {
					#[codec(index = 0)]
					MaturationPeriod,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum HoldReason {
					#[codec(index = 0)]
					MaturationPeriod,
				}
			}
		}
		pub mod pallet_block_seal {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See `Pallet::apply`."]
					apply { seal: runtime_types::ulx_primitives::inherents::BlockSealInherent },
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					InvalidVoteProof,
					#[codec(index = 1)]
					InvalidSubmitter,
					#[codec(index = 2)]
					UnableToDecodeVoteAccount,
					#[codec(index = 3)]
					UnregisteredBlockAuthor,
					#[codec(index = 4)]
					InvalidBlockVoteProof,
					#[codec(index = 5)]
					NoGrandparentVoteMinimum,
					#[codec(index = 6)]
					DuplicateBlockSealProvided,
					#[codec(index = 7)]
					InsufficientVotingPower,
					#[codec(index = 8)]
					ParentVotingKeyNotFound,
					#[codec(index = 9)]
					InvalidVoteGrandparentHash,
					#[codec(index = 10)]
					BlockVoteDigestMissing,
					#[codec(index = 11)]
					IneligibleNotebookUsed,
					#[codec(index = 12)]
					NoEligibleVotingRoot,
					#[codec(index = 13)]
					InvalidAuthoritySupplied,
					#[codec(index = 14)]
					InvalidAuthoritySignature,
				}
			}
		}
		pub mod pallet_block_seal_spec {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See `Pallet::configure`."]
					configure {
						vote_minimum: ::core::option::Option<::core::primitive::u128>,
						compute_difficulty: ::core::option::Option<::core::primitive::u128>,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					VoteMinimumAdjusted {
						expected_block_votes: ::core::primitive::u128,
						actual_block_votes: ::core::primitive::u128,
						start_vote_minimum: ::core::primitive::u128,
						new_vote_minimum: ::core::primitive::u128,
					},
					#[codec(index = 1)]
					ComputeDifficultyAdjusted {
						expected_block_time: ::core::primitive::u64,
						actual_block_time: ::core::primitive::u64,
						start_difficulty: ::core::primitive::u128,
						new_difficulty: ::core::primitive::u128,
					},
				}
			}
		}
		pub mod pallet_bond {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See `Pallet::offer_fund`."]
					offer_fund {
						#[codec(compact)]
						lease_annual_percent_rate: ::core::primitive::u32,
						#[codec(compact)]
						lease_base_fee: ::core::primitive::u128,
						#[codec(compact)]
						amount_offered: ::core::primitive::u128,
						expiration_block: ::core::primitive::u32,
					},
					#[codec(index = 1)]
					#[doc = "See `Pallet::end_fund`."]
					end_fund { bond_fund_id: ::core::primitive::u32 },
					#[codec(index = 2)]
					#[doc = "See `Pallet::extend_fund`."]
					extend_fund {
						bond_fund_id: ::core::primitive::u32,
						total_amount_offered: ::core::primitive::u128,
						expiration_block: ::core::primitive::u32,
					},
					#[codec(index = 3)]
					#[doc = "See `Pallet::bond_self`."]
					bond_self {
						amount: ::core::primitive::u128,
						bond_until_block: ::core::primitive::u32,
					},
					#[codec(index = 4)]
					#[doc = "See `Pallet::lease`."]
					lease {
						bond_fund_id: ::core::primitive::u32,
						amount: ::core::primitive::u128,
						lease_until_block: ::core::primitive::u32,
					},
					#[codec(index = 5)]
					#[doc = "See `Pallet::return_bond`."]
					return_bond { bond_id: ::core::primitive::u64 },
					#[codec(index = 6)]
					#[doc = "See `Pallet::extend_bond`."]
					extend_bond {
						bond_id: ::core::primitive::u64,
						total_amount: ::core::primitive::u128,
						bond_until_block: ::core::primitive::u32,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					BadState,
					#[codec(index = 1)]
					BondNotFound,
					#[codec(index = 2)]
					NoMoreBondFundIds,
					#[codec(index = 3)]
					NoMoreBondIds,
					#[codec(index = 4)]
					MinimumBondAmountNotMet,
					#[codec(index = 5)]
					#[doc = "There are too many bond or bond funds expiring in the given expiration block"]
					ExpirationAtBlockOverflow,
					#[codec(index = 6)]
					InsufficientFunds,
					#[codec(index = 7)]
					InsufficientBondFunds,
					#[codec(index = 8)]
					TransactionWouldTakeAccountBelowMinimumBalance,
					#[codec(index = 9)]
					BondFundClosed,
					#[codec(index = 10)]
					#[doc = "This reduction in bond funds offered goes below the amount that is already committed to"]
					#[doc = "bond"]
					BondFundReductionExceedsAllocatedFunds,
					#[codec(index = 11)]
					ExpirationTooSoon,
					#[codec(index = 12)]
					LeaseUntilBlockTooSoon,
					#[codec(index = 13)]
					LeaseUntilPastFundExpiration,
					#[codec(index = 14)]
					NoPermissions,
					#[codec(index = 15)]
					NoBondFundFound,
					#[codec(index = 16)]
					FundExtensionMustBeLater,
					#[codec(index = 17)]
					HoldUnexpectedlyModified,
					#[codec(index = 18)]
					BondFundMaximumBondsExceeded,
					#[codec(index = 19)]
					UnrecoverableHold,
					#[codec(index = 20)]
					BondFundNotFound,
					#[codec(index = 21)]
					BondAlreadyLocked,
					#[codec(index = 22)]
					BondLockedCannotModify,
					#[codec(index = 23)]
					#[doc = "The fee for this bond exceeds the amount of the bond, which is unsafe"]
					FeeExceedsBondAmount,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					BondFundOffered {
						bond_fund_id: ::core::primitive::u32,
						amount_offered: ::core::primitive::u128,
						expiration_block: ::core::primitive::u32,
						offer_account_id: ::subxt::utils::AccountId32,
					},
					#[codec(index = 1)]
					BondFundExtended {
						bond_fund_id: ::core::primitive::u32,
						amount_offered: ::core::primitive::u128,
						expiration_block: ::core::primitive::u32,
					},
					#[codec(index = 2)]
					BondFundEnded {
						bond_fund_id: ::core::primitive::u32,
						amount_still_bonded: ::core::primitive::u128,
					},
					#[codec(index = 3)]
					BondFundExpired {
						bond_fund_id: ::core::primitive::u32,
						offer_account_id: ::subxt::utils::AccountId32,
					},
					#[codec(index = 4)]
					BondedSelf {
						bond_id: ::core::primitive::u64,
						bonded_account_id: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						completion_block: ::core::primitive::u32,
					},
					#[codec(index = 5)]
					BondLeased {
						bond_fund_id: ::core::primitive::u32,
						bond_id: ::core::primitive::u64,
						bonded_account_id: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						total_fee: ::core::primitive::u128,
						annual_percent_rate: ::core::primitive::u32,
						completion_block: ::core::primitive::u32,
					},
					#[codec(index = 6)]
					BondExtended {
						bond_fund_id: ::core::option::Option<::core::primitive::u32>,
						bond_id: ::core::primitive::u64,
						amount: ::core::primitive::u128,
						completion_block: ::core::primitive::u32,
						fee_change: ::core::primitive::u128,
						annual_percent_rate: ::core::primitive::u32,
					},
					#[codec(index = 7)]
					BondCompleted {
						bond_fund_id: ::core::option::Option<::core::primitive::u32>,
						bond_id: ::core::primitive::u64,
					},
					#[codec(index = 8)]
					BondFeeRefund {
						bond_fund_id: ::core::primitive::u32,
						bond_id: ::core::primitive::u64,
						bonded_account_id: ::subxt::utils::AccountId32,
						bond_fund_reduction_for_payment: ::core::primitive::u128,
						final_fee: ::core::primitive::u128,
						refund_amount: ::core::primitive::u128,
					},
					#[codec(index = 9)]
					BondLocked {
						bond_id: ::core::primitive::u64,
						bonded_account_id: ::subxt::utils::AccountId32,
					},
					#[codec(index = 10)]
					BondUnlocked {
						bond_id: ::core::primitive::u64,
						bonded_account_id: ::subxt::utils::AccountId32,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum HoldReason {
					#[codec(index = 0)]
					EnterBondFund,
				}
			}
		}
		pub mod pallet_chain_transfer {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See `Pallet::send_to_localchain`."]
					send_to_localchain {
						#[codec(compact)]
						amount: ::core::primitive::u128,
						notary_id: ::core::primitive::u32,
						#[codec(compact)]
						account_nonce: ::core::primitive::u32,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					MaxBlockTransfersExceeded,
					#[codec(index = 1)]
					#[doc = "Insufficient balance to create this transfer"]
					InsufficientFunds,
					#[codec(index = 2)]
					#[doc = "The account nonce used for this transfer is no longer valid"]
					InvalidAccountNonce,
					#[codec(index = 3)]
					#[doc = "Insufficient balance to fulfill a mainchain transfer"]
					InsufficientNotarizedFunds,
					#[codec(index = 4)]
					#[doc = "The transfer was already submitted in a previous block"]
					InvalidOrDuplicatedLocalchainTransfer,
					#[codec(index = 5)]
					#[doc = "A transfer was submitted in a previous block but the expiration block has passed"]
					NotebookIncludesExpiredLocalchainTransfer,
					#[codec(index = 6)]
					#[doc = "The notary id is not registered"]
					InvalidNotaryUsedForTransfer,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					TransferToLocalchain {
						account_id: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						account_nonce: ::core::primitive::u32,
						notary_id: ::core::primitive::u32,
						expiration_block: ::core::primitive::u32,
					},
					#[codec(index = 1)]
					TransferToLocalchainExpired {
						account_id: ::subxt::utils::AccountId32,
						account_nonce: ::core::primitive::u32,
						notary_id: ::core::primitive::u32,
					},
					#[codec(index = 2)]
					TransferIn {
						account_id: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						notary_id: ::core::primitive::u32,
					},
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct QueuedTransferOut<_0, _1> {
				pub amount: _0,
				pub expiration_block: _1,
				pub notary_id: ::core::primitive::u32,
			}
		}
		pub mod pallet_grandpa {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::report_equivocation`]."]
					report_equivocation {
						equivocation_proof: ::std::boxed::Box<
							runtime_types::sp_consensus_grandpa::EquivocationProof<
								::subxt::utils::H256,
								::core::primitive::u32,
							>,
						>,
						key_owner_proof: runtime_types::sp_session::MembershipProof,
					},
					#[codec(index = 1)]
					#[doc = "See [`Pallet::report_equivocation_unsigned`]."]
					report_equivocation_unsigned {
						equivocation_proof: ::std::boxed::Box<
							runtime_types::sp_consensus_grandpa::EquivocationProof<
								::subxt::utils::H256,
								::core::primitive::u32,
							>,
						>,
						key_owner_proof: runtime_types::sp_session::MembershipProof,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::note_stalled`]."]
					note_stalled {
						delay: ::core::primitive::u32,
						best_finalized_block_number: ::core::primitive::u32,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Attempt to signal GRANDPA pause when the authority set isn't live"]
					#[doc = "(either paused or already pending pause)."]
					PauseFailed,
					#[codec(index = 1)]
					#[doc = "Attempt to signal GRANDPA resume when the authority set isn't paused"]
					#[doc = "(either live or already pending resume)."]
					ResumeFailed,
					#[codec(index = 2)]
					#[doc = "Attempt to signal GRANDPA change with one already pending."]
					ChangePending,
					#[codec(index = 3)]
					#[doc = "Cannot signal forced change so soon after last."]
					TooSoon,
					#[codec(index = 4)]
					#[doc = "A key ownership proof provided as part of an equivocation report is invalid."]
					InvalidKeyOwnershipProof,
					#[codec(index = 5)]
					#[doc = "An equivocation proof provided as part of an equivocation report is invalid."]
					InvalidEquivocationProof,
					#[codec(index = 6)]
					#[doc = "A given equivocation report is valid but already previously reported."]
					DuplicateOffenceReport,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "New authority set has been applied."]
					NewAuthorities {
						authority_set: ::std::vec::Vec<(
							runtime_types::sp_consensus_grandpa::app::Public,
							::core::primitive::u64,
						)>,
					},
					#[codec(index = 1)]
					#[doc = "Current authority set has been paused."]
					Paused,
					#[codec(index = 2)]
					#[doc = "Current authority set has been resumed."]
					Resumed,
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct StoredPendingChange<_0> {
				pub scheduled_at: _0,
				pub delay: _0,
				pub next_authorities:
					runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<(
						runtime_types::sp_consensus_grandpa::app::Public,
						::core::primitive::u64,
					)>,
				pub forced: ::core::option::Option<_0>,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum StoredState<_0> {
				#[codec(index = 0)]
				Live,
				#[codec(index = 1)]
				PendingPause { scheduled_at: _0, delay: _0 },
				#[codec(index = 2)]
				Paused,
				#[codec(index = 3)]
				PendingResume { scheduled_at: _0, delay: _0 },
			}
		}
		pub mod pallet_mining_slot {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See `Pallet::bid`."]
					bid {
						peer_id: runtime_types::sp_core::OpaquePeerId,
						rpc_hosts: runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::block_seal::Host,
						>,
						bond_id: ::core::option::Option<::core::primitive::u64>,
						reward_destination:
							runtime_types::ulx_primitives::block_seal::RewardDestination<
								::subxt::utils::AccountId32,
							>,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					SlotNotTakingBids,
					#[codec(index = 1)]
					TooManyBlockRegistrants,
					#[codec(index = 2)]
					UnableToRotateAuthority,
					#[codec(index = 3)]
					InsufficientOwnershipTokens,
					#[codec(index = 4)]
					InsufficientBalanceForBid,
					#[codec(index = 5)]
					BidTooLow,
					#[codec(index = 6)]
					#[doc = "Internal state has become somehow corrupted and the operation cannot continue."]
					BadInternalState,
					#[codec(index = 7)]
					#[doc = "You must register with rpc hosts so that your miner can be reached for block seal"]
					#[doc = "auditing"]
					RpcHostsAreRequired,
					#[codec(index = 8)]
					BidBondDurationTooShort,
					#[codec(index = 9)]
					CannotRegisteredOverlappingSessions,
					#[codec(index = 10)]
					BadState,
					#[codec(index = 11)]
					BondNotFound,
					#[codec(index = 12)]
					NoMoreBondIds,
					#[codec(index = 13)]
					BondFundClosed,
					#[codec(index = 14)]
					MinimumBondAmountNotMet,
					#[codec(index = 15)]
					LeaseUntilBlockTooSoon,
					#[codec(index = 16)]
					LeaseUntilPastFundExpiration,
					#[codec(index = 17)]
					#[doc = "There are too many bond or bond funds expiring in the given expiration block"]
					ExpirationAtBlockOverflow,
					#[codec(index = 18)]
					InsufficientFunds,
					#[codec(index = 19)]
					InsufficientBondFunds,
					#[codec(index = 20)]
					ExpirationTooSoon,
					#[codec(index = 21)]
					NoPermissions,
					#[codec(index = 22)]
					NoBondFundFound,
					#[codec(index = 23)]
					HoldUnexpectedlyModified,
					#[codec(index = 24)]
					BondFundMaximumBondsExceeded,
					#[codec(index = 25)]
					UnrecoverableHold,
					#[codec(index = 26)]
					BondFundNotFound,
					#[codec(index = 27)]
					BondAlreadyClosed,
					#[codec(index = 28)]
					BondAlreadyLocked,
					#[codec(index = 29)]
					BondLockedCannotModify,
					#[codec(index = 30)]
					#[doc = "The fee for this bond exceeds the amount of the bond, which is unsafe"]
					FeeExceedsBondAmount,
					#[codec(index = 31)]
					AccountWouldBeBelowMinimum,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					NewMiners {
						start_index: ::core::primitive::u32,
						new_miners: runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::block_seal::MiningRegistration<
								::subxt::utils::AccountId32,
								::core::primitive::u64,
								::core::primitive::u128,
							>,
						>,
					},
					#[codec(index = 1)]
					SlotBidderAdded {
						account_id: ::subxt::utils::AccountId32,
						bid_amount: ::core::primitive::u128,
						index: ::core::primitive::u32,
					},
					#[codec(index = 2)]
					SlotBidderReplaced {
						account_id: ::subxt::utils::AccountId32,
						bond_id: ::core::option::Option<::core::primitive::u64>,
						kept_ownership_bond: ::core::primitive::bool,
					},
					#[codec(index = 3)]
					UnbondedMiner {
						account_id: ::subxt::utils::AccountId32,
						bond_id: ::core::option::Option<::core::primitive::u64>,
						kept_ownership_bond: ::core::primitive::bool,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum HoldReason {
					#[codec(index = 0)]
					RegisterAsMiner,
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct MinerHistory {
				pub authority_index: ::core::primitive::u32,
			}
		}
		pub mod pallet_notaries {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See `Pallet::propose`."]
					propose { meta: runtime_types::ulx_primitives::notary::NotaryMeta },
					#[codec(index = 1)]
					#[doc = "See `Pallet::activate`."]
					activate { operator_account: ::subxt::utils::AccountId32 },
					#[codec(index = 2)]
					#[doc = "See `Pallet::update`."]
					update {
						#[codec(compact)]
						notary_id: ::core::primitive::u32,
						meta: runtime_types::ulx_primitives::notary::NotaryMeta,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					ProposalNotFound,
					#[codec(index = 1)]
					MaxNotariesExceeded,
					#[codec(index = 2)]
					MaxProposalsPerBlockExceeded,
					#[codec(index = 3)]
					NotAnActiveNotary,
					#[codec(index = 4)]
					InvalidNotaryOperator,
					#[codec(index = 5)]
					NoMoreNotaryIds,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "A user has proposed operating as a notary"]
					NotaryProposed {
						operator_account: ::subxt::utils::AccountId32,
						meta: runtime_types::ulx_primitives::notary::NotaryMeta,
						expires: ::core::primitive::u32,
					},
					#[codec(index = 1)]
					#[doc = "A notary proposal has been accepted"]
					NotaryActivated {
						notary: runtime_types::ulx_primitives::notary::NotaryRecord<
							::subxt::utils::AccountId32,
							::core::primitive::u32,
						>,
					},
					#[codec(index = 2)]
					#[doc = "Notary metadata queued for update"]
					NotaryMetaUpdateQueued {
						notary_id: ::core::primitive::u32,
						meta: runtime_types::ulx_primitives::notary::NotaryMeta,
						effective_block: ::core::primitive::u32,
					},
					#[codec(index = 3)]
					#[doc = "Notary metadata updated"]
					NotaryMetaUpdated {
						notary_id: ::core::primitive::u32,
						meta: runtime_types::ulx_primitives::notary::NotaryMeta,
					},
				}
			}
		}
		pub mod pallet_notebook {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See `Pallet::submit`."]
					submit {
						header: runtime_types::ulx_primitives::notebook::NotebookHeader,
						hash: ::subxt::utils::H256,
						signature: runtime_types::sp_core::ed25519::Signature,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					UnapprovedNotary,
					#[codec(index = 1)]
					MissingNotebookNumber,
					#[codec(index = 2)]
					NotebookTickAlreadyUsed,
					#[codec(index = 3)]
					IncorrectBlockHeight,
					#[codec(index = 4)]
					NoTickDigestFound,
					#[codec(index = 5)]
					#[doc = "The secret or secret hash of the parent notebook do not match"]
					InvalidSecretProvided,
					#[codec(index = 6)]
					CouldNotDecodeVote,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					NotebookSubmitted {
						notary_id: ::core::primitive::u32,
						notebook_number: ::core::primitive::u32,
					},
				}
			}
		}
		pub mod pallet_offences {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Events type."]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "There is an offence reported of the given `kind` happened at the `session_index` and"]
					#[doc = "(kind-specific) time slot. This event is not deposited for duplicate slashes."]
					#[doc = "\\[kind, timeslot\\]."]
					Offence {
						kind: [::core::primitive::u8; 16usize],
						timeslot: ::std::vec::Vec<::core::primitive::u8>,
					},
				}
			}
		}
		pub mod pallet_session {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::set_keys`]."]
					set_keys {
						keys: runtime_types::ulx_node_runtime::opaque::SessionKeys,
						proof: ::std::vec::Vec<::core::primitive::u8>,
					},
					#[codec(index = 1)]
					#[doc = "See [`Pallet::purge_keys`]."]
					purge_keys,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Error for the session pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Invalid ownership proof."]
					InvalidProof,
					#[codec(index = 1)]
					#[doc = "No associated validator ID for account."]
					NoAssociatedValidatorId,
					#[codec(index = 2)]
					#[doc = "Registered duplicate key."]
					DuplicatedKey,
					#[codec(index = 3)]
					#[doc = "No keys are associated with this account."]
					NoKeys,
					#[codec(index = 4)]
					#[doc = "Key setting account is not live, so it's impossible to associate keys."]
					NoAccount,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "New session has happened. Note that the argument is the session index, not the"]
					#[doc = "block number as the type might suggest."]
					NewSession { session_index: ::core::primitive::u32 },
				}
			}
		}
		pub mod pallet_sudo {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::sudo`]."]
					sudo { call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall> },
					#[codec(index = 1)]
					#[doc = "See [`Pallet::sudo_unchecked_weight`]."]
					sudo_unchecked_weight {
						call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall>,
						weight: runtime_types::sp_weights::weight_v2::Weight,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::set_key`]."]
					set_key { new: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()> },
					#[codec(index = 3)]
					#[doc = "See [`Pallet::sudo_as`]."]
					sudo_as {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall>,
					},
					#[codec(index = 4)]
					#[doc = "See [`Pallet::remove_key`]."]
					remove_key,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Error for the Sudo pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Sender must be the Sudo account."]
					RequireSudo,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "A sudo call just took place."]
					Sudid {
						sudo_result:
							::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
					},
					#[codec(index = 1)]
					#[doc = "The sudo key has been updated."]
					KeyChanged {
						old: ::core::option::Option<::subxt::utils::AccountId32>,
						new: ::subxt::utils::AccountId32,
					},
					#[codec(index = 2)]
					#[doc = "The key was permanently removed."]
					KeyRemoved,
					#[codec(index = 3)]
					#[doc = "A [sudo_as](Pallet::sudo_as) call just took place."]
					SudoAsDone {
						sudo_result:
							::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
					},
				}
			}
		}
		pub mod pallet_ticks {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {}
			}
		}
		pub mod pallet_timestamp {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::set`]."]
					set {
						#[codec(compact)]
						now: ::core::primitive::u64,
					},
				}
			}
		}
		pub mod pallet_transaction_payment {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,"]
					#[doc = "has been paid by `who`."]
					TransactionFeePaid {
						who: ::subxt::utils::AccountId32,
						actual_fee: ::core::primitive::u128,
						tip: ::core::primitive::u128,
					},
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct FeeDetails<_0> {
					pub inclusion_fee: ::core::option::Option<
						runtime_types::pallet_transaction_payment::types::InclusionFee<_0>,
					>,
					pub tip: _0,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct InclusionFee<_0> {
					pub base_fee: _0,
					pub len_fee: _0,
					pub adjusted_weight_fee: _0,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RuntimeDispatchInfo<_0, _1> {
					pub weight: _1,
					pub class: runtime_types::frame_support::dispatch::DispatchClass,
					pub partial_fee: _0,
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct ChargeTransactionPayment(#[codec(compact)] pub ::core::primitive::u128);
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum Releases {
				#[codec(index = 0)]
				V1Ancient,
				#[codec(index = 1)]
				V2,
			}
		}
		pub mod pallet_tx_pause {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::pause`]."]
					pause {
						full_name: (
							runtime_types::bounded_collections::bounded_vec::BoundedVec<
								::core::primitive::u8,
							>,
							runtime_types::bounded_collections::bounded_vec::BoundedVec<
								::core::primitive::u8,
							>,
						),
					},
					#[codec(index = 1)]
					#[doc = "See [`Pallet::unpause`]."]
					unpause {
						ident: (
							runtime_types::bounded_collections::bounded_vec::BoundedVec<
								::core::primitive::u8,
							>,
							runtime_types::bounded_collections::bounded_vec::BoundedVec<
								::core::primitive::u8,
							>,
						),
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "The call is paused."]
					IsPaused,
					#[codec(index = 1)]
					#[doc = "The call is unpaused."]
					IsUnpaused,
					#[codec(index = 2)]
					#[doc = "The call is whitelisted and cannot be paused."]
					Unpausable,
					#[codec(index = 3)]
					NotFound,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "This pallet, or a specific call is now paused."]
					CallPaused {
						full_name: (
							runtime_types::bounded_collections::bounded_vec::BoundedVec<
								::core::primitive::u8,
							>,
							runtime_types::bounded_collections::bounded_vec::BoundedVec<
								::core::primitive::u8,
							>,
						),
					},
					#[codec(index = 1)]
					#[doc = "This pallet, or a specific call is now unpaused."]
					CallUnpaused {
						full_name: (
							runtime_types::bounded_collections::bounded_vec::BoundedVec<
								::core::primitive::u8,
							>,
							runtime_types::bounded_collections::bounded_vec::BoundedVec<
								::core::primitive::u8,
							>,
						),
					},
				}
			}
		}
		pub mod primitive_types {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct U256(pub [::core::primitive::u64; 4usize]);
		}
		pub mod sp_arithmetic {
			use super::runtime_types;
			pub mod fixed_point {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct FixedU128(pub ::core::primitive::u128);
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum ArithmeticError {
				#[codec(index = 0)]
				Underflow,
				#[codec(index = 1)]
				Overflow,
				#[codec(index = 2)]
				DivisionByZero,
			}
		}
		pub mod sp_consensus_grandpa {
			use super::runtime_types;
			pub mod app {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Public(pub runtime_types::sp_core::ed25519::Public);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Signature(pub runtime_types::sp_core::ed25519::Signature);
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum Equivocation<_0, _1> {
				#[codec(index = 0)]
				Prevote(
					runtime_types::finality_grandpa::Equivocation<
						runtime_types::sp_consensus_grandpa::app::Public,
						runtime_types::finality_grandpa::Prevote<_0, _1>,
						runtime_types::sp_consensus_grandpa::app::Signature,
					>,
				),
				#[codec(index = 1)]
				Precommit(
					runtime_types::finality_grandpa::Equivocation<
						runtime_types::sp_consensus_grandpa::app::Public,
						runtime_types::finality_grandpa::Precommit<_0, _1>,
						runtime_types::sp_consensus_grandpa::app::Signature,
					>,
				),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct EquivocationProof<_0, _1> {
				pub set_id: ::core::primitive::u64,
				pub equivocation: runtime_types::sp_consensus_grandpa::Equivocation<_0, _1>,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct OpaqueKeyOwnershipProof(pub ::std::vec::Vec<::core::primitive::u8>);
		}
		pub mod sp_core {
			use super::runtime_types;
			pub mod crypto {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct KeyTypeId(pub [::core::primitive::u8; 4usize]);
			}
			pub mod ecdsa {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Signature(pub [::core::primitive::u8; 65usize]);
			}
			pub mod ed25519 {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Public(pub [::core::primitive::u8; 32usize]);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Signature(pub [::core::primitive::u8; 64usize]);
			}
			pub mod sr25519 {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Signature(pub [::core::primitive::u8; 64usize]);
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct OpaqueMetadata(pub ::std::vec::Vec<::core::primitive::u8>);
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct OpaquePeerId(pub ::std::vec::Vec<::core::primitive::u8>);
		}
		pub mod sp_inherents {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct CheckInherentsResult {
				pub okay: ::core::primitive::bool,
				pub fatal_error: ::core::primitive::bool,
				pub errors: runtime_types::sp_inherents::InherentData,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct InherentData {
				pub data: ::subxt::utils::KeyedVec<
					[::core::primitive::u8; 8usize],
					::std::vec::Vec<::core::primitive::u8>,
				>,
			}
		}
		pub mod sp_runtime {
			use super::runtime_types;
			pub mod generic {
				use super::runtime_types;
				pub mod block {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct Block<_0, _1> {
						pub header: _0,
						pub extrinsics: ::std::vec::Vec<_1>,
					}
				}
				pub mod digest {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct Digest {
						pub logs:
							::std::vec::Vec<runtime_types::sp_runtime::generic::digest::DigestItem>,
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub enum DigestItem {
						#[codec(index = 6)]
						PreRuntime(
							[::core::primitive::u8; 4usize],
							::std::vec::Vec<::core::primitive::u8>,
						),
						#[codec(index = 4)]
						Consensus(
							[::core::primitive::u8; 4usize],
							::std::vec::Vec<::core::primitive::u8>,
						),
						#[codec(index = 5)]
						Seal(
							[::core::primitive::u8; 4usize],
							::std::vec::Vec<::core::primitive::u8>,
						),
						#[codec(index = 0)]
						Other(::std::vec::Vec<::core::primitive::u8>),
						#[codec(index = 8)]
						RuntimeEnvironmentUpdated,
					}
				}
				pub mod era {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub enum Era {
						#[codec(index = 0)]
						Immortal,
						#[codec(index = 1)]
						Mortal1(::core::primitive::u8),
						#[codec(index = 2)]
						Mortal2(::core::primitive::u8),
						#[codec(index = 3)]
						Mortal3(::core::primitive::u8),
						#[codec(index = 4)]
						Mortal4(::core::primitive::u8),
						#[codec(index = 5)]
						Mortal5(::core::primitive::u8),
						#[codec(index = 6)]
						Mortal6(::core::primitive::u8),
						#[codec(index = 7)]
						Mortal7(::core::primitive::u8),
						#[codec(index = 8)]
						Mortal8(::core::primitive::u8),
						#[codec(index = 9)]
						Mortal9(::core::primitive::u8),
						#[codec(index = 10)]
						Mortal10(::core::primitive::u8),
						#[codec(index = 11)]
						Mortal11(::core::primitive::u8),
						#[codec(index = 12)]
						Mortal12(::core::primitive::u8),
						#[codec(index = 13)]
						Mortal13(::core::primitive::u8),
						#[codec(index = 14)]
						Mortal14(::core::primitive::u8),
						#[codec(index = 15)]
						Mortal15(::core::primitive::u8),
						#[codec(index = 16)]
						Mortal16(::core::primitive::u8),
						#[codec(index = 17)]
						Mortal17(::core::primitive::u8),
						#[codec(index = 18)]
						Mortal18(::core::primitive::u8),
						#[codec(index = 19)]
						Mortal19(::core::primitive::u8),
						#[codec(index = 20)]
						Mortal20(::core::primitive::u8),
						#[codec(index = 21)]
						Mortal21(::core::primitive::u8),
						#[codec(index = 22)]
						Mortal22(::core::primitive::u8),
						#[codec(index = 23)]
						Mortal23(::core::primitive::u8),
						#[codec(index = 24)]
						Mortal24(::core::primitive::u8),
						#[codec(index = 25)]
						Mortal25(::core::primitive::u8),
						#[codec(index = 26)]
						Mortal26(::core::primitive::u8),
						#[codec(index = 27)]
						Mortal27(::core::primitive::u8),
						#[codec(index = 28)]
						Mortal28(::core::primitive::u8),
						#[codec(index = 29)]
						Mortal29(::core::primitive::u8),
						#[codec(index = 30)]
						Mortal30(::core::primitive::u8),
						#[codec(index = 31)]
						Mortal31(::core::primitive::u8),
						#[codec(index = 32)]
						Mortal32(::core::primitive::u8),
						#[codec(index = 33)]
						Mortal33(::core::primitive::u8),
						#[codec(index = 34)]
						Mortal34(::core::primitive::u8),
						#[codec(index = 35)]
						Mortal35(::core::primitive::u8),
						#[codec(index = 36)]
						Mortal36(::core::primitive::u8),
						#[codec(index = 37)]
						Mortal37(::core::primitive::u8),
						#[codec(index = 38)]
						Mortal38(::core::primitive::u8),
						#[codec(index = 39)]
						Mortal39(::core::primitive::u8),
						#[codec(index = 40)]
						Mortal40(::core::primitive::u8),
						#[codec(index = 41)]
						Mortal41(::core::primitive::u8),
						#[codec(index = 42)]
						Mortal42(::core::primitive::u8),
						#[codec(index = 43)]
						Mortal43(::core::primitive::u8),
						#[codec(index = 44)]
						Mortal44(::core::primitive::u8),
						#[codec(index = 45)]
						Mortal45(::core::primitive::u8),
						#[codec(index = 46)]
						Mortal46(::core::primitive::u8),
						#[codec(index = 47)]
						Mortal47(::core::primitive::u8),
						#[codec(index = 48)]
						Mortal48(::core::primitive::u8),
						#[codec(index = 49)]
						Mortal49(::core::primitive::u8),
						#[codec(index = 50)]
						Mortal50(::core::primitive::u8),
						#[codec(index = 51)]
						Mortal51(::core::primitive::u8),
						#[codec(index = 52)]
						Mortal52(::core::primitive::u8),
						#[codec(index = 53)]
						Mortal53(::core::primitive::u8),
						#[codec(index = 54)]
						Mortal54(::core::primitive::u8),
						#[codec(index = 55)]
						Mortal55(::core::primitive::u8),
						#[codec(index = 56)]
						Mortal56(::core::primitive::u8),
						#[codec(index = 57)]
						Mortal57(::core::primitive::u8),
						#[codec(index = 58)]
						Mortal58(::core::primitive::u8),
						#[codec(index = 59)]
						Mortal59(::core::primitive::u8),
						#[codec(index = 60)]
						Mortal60(::core::primitive::u8),
						#[codec(index = 61)]
						Mortal61(::core::primitive::u8),
						#[codec(index = 62)]
						Mortal62(::core::primitive::u8),
						#[codec(index = 63)]
						Mortal63(::core::primitive::u8),
						#[codec(index = 64)]
						Mortal64(::core::primitive::u8),
						#[codec(index = 65)]
						Mortal65(::core::primitive::u8),
						#[codec(index = 66)]
						Mortal66(::core::primitive::u8),
						#[codec(index = 67)]
						Mortal67(::core::primitive::u8),
						#[codec(index = 68)]
						Mortal68(::core::primitive::u8),
						#[codec(index = 69)]
						Mortal69(::core::primitive::u8),
						#[codec(index = 70)]
						Mortal70(::core::primitive::u8),
						#[codec(index = 71)]
						Mortal71(::core::primitive::u8),
						#[codec(index = 72)]
						Mortal72(::core::primitive::u8),
						#[codec(index = 73)]
						Mortal73(::core::primitive::u8),
						#[codec(index = 74)]
						Mortal74(::core::primitive::u8),
						#[codec(index = 75)]
						Mortal75(::core::primitive::u8),
						#[codec(index = 76)]
						Mortal76(::core::primitive::u8),
						#[codec(index = 77)]
						Mortal77(::core::primitive::u8),
						#[codec(index = 78)]
						Mortal78(::core::primitive::u8),
						#[codec(index = 79)]
						Mortal79(::core::primitive::u8),
						#[codec(index = 80)]
						Mortal80(::core::primitive::u8),
						#[codec(index = 81)]
						Mortal81(::core::primitive::u8),
						#[codec(index = 82)]
						Mortal82(::core::primitive::u8),
						#[codec(index = 83)]
						Mortal83(::core::primitive::u8),
						#[codec(index = 84)]
						Mortal84(::core::primitive::u8),
						#[codec(index = 85)]
						Mortal85(::core::primitive::u8),
						#[codec(index = 86)]
						Mortal86(::core::primitive::u8),
						#[codec(index = 87)]
						Mortal87(::core::primitive::u8),
						#[codec(index = 88)]
						Mortal88(::core::primitive::u8),
						#[codec(index = 89)]
						Mortal89(::core::primitive::u8),
						#[codec(index = 90)]
						Mortal90(::core::primitive::u8),
						#[codec(index = 91)]
						Mortal91(::core::primitive::u8),
						#[codec(index = 92)]
						Mortal92(::core::primitive::u8),
						#[codec(index = 93)]
						Mortal93(::core::primitive::u8),
						#[codec(index = 94)]
						Mortal94(::core::primitive::u8),
						#[codec(index = 95)]
						Mortal95(::core::primitive::u8),
						#[codec(index = 96)]
						Mortal96(::core::primitive::u8),
						#[codec(index = 97)]
						Mortal97(::core::primitive::u8),
						#[codec(index = 98)]
						Mortal98(::core::primitive::u8),
						#[codec(index = 99)]
						Mortal99(::core::primitive::u8),
						#[codec(index = 100)]
						Mortal100(::core::primitive::u8),
						#[codec(index = 101)]
						Mortal101(::core::primitive::u8),
						#[codec(index = 102)]
						Mortal102(::core::primitive::u8),
						#[codec(index = 103)]
						Mortal103(::core::primitive::u8),
						#[codec(index = 104)]
						Mortal104(::core::primitive::u8),
						#[codec(index = 105)]
						Mortal105(::core::primitive::u8),
						#[codec(index = 106)]
						Mortal106(::core::primitive::u8),
						#[codec(index = 107)]
						Mortal107(::core::primitive::u8),
						#[codec(index = 108)]
						Mortal108(::core::primitive::u8),
						#[codec(index = 109)]
						Mortal109(::core::primitive::u8),
						#[codec(index = 110)]
						Mortal110(::core::primitive::u8),
						#[codec(index = 111)]
						Mortal111(::core::primitive::u8),
						#[codec(index = 112)]
						Mortal112(::core::primitive::u8),
						#[codec(index = 113)]
						Mortal113(::core::primitive::u8),
						#[codec(index = 114)]
						Mortal114(::core::primitive::u8),
						#[codec(index = 115)]
						Mortal115(::core::primitive::u8),
						#[codec(index = 116)]
						Mortal116(::core::primitive::u8),
						#[codec(index = 117)]
						Mortal117(::core::primitive::u8),
						#[codec(index = 118)]
						Mortal118(::core::primitive::u8),
						#[codec(index = 119)]
						Mortal119(::core::primitive::u8),
						#[codec(index = 120)]
						Mortal120(::core::primitive::u8),
						#[codec(index = 121)]
						Mortal121(::core::primitive::u8),
						#[codec(index = 122)]
						Mortal122(::core::primitive::u8),
						#[codec(index = 123)]
						Mortal123(::core::primitive::u8),
						#[codec(index = 124)]
						Mortal124(::core::primitive::u8),
						#[codec(index = 125)]
						Mortal125(::core::primitive::u8),
						#[codec(index = 126)]
						Mortal126(::core::primitive::u8),
						#[codec(index = 127)]
						Mortal127(::core::primitive::u8),
						#[codec(index = 128)]
						Mortal128(::core::primitive::u8),
						#[codec(index = 129)]
						Mortal129(::core::primitive::u8),
						#[codec(index = 130)]
						Mortal130(::core::primitive::u8),
						#[codec(index = 131)]
						Mortal131(::core::primitive::u8),
						#[codec(index = 132)]
						Mortal132(::core::primitive::u8),
						#[codec(index = 133)]
						Mortal133(::core::primitive::u8),
						#[codec(index = 134)]
						Mortal134(::core::primitive::u8),
						#[codec(index = 135)]
						Mortal135(::core::primitive::u8),
						#[codec(index = 136)]
						Mortal136(::core::primitive::u8),
						#[codec(index = 137)]
						Mortal137(::core::primitive::u8),
						#[codec(index = 138)]
						Mortal138(::core::primitive::u8),
						#[codec(index = 139)]
						Mortal139(::core::primitive::u8),
						#[codec(index = 140)]
						Mortal140(::core::primitive::u8),
						#[codec(index = 141)]
						Mortal141(::core::primitive::u8),
						#[codec(index = 142)]
						Mortal142(::core::primitive::u8),
						#[codec(index = 143)]
						Mortal143(::core::primitive::u8),
						#[codec(index = 144)]
						Mortal144(::core::primitive::u8),
						#[codec(index = 145)]
						Mortal145(::core::primitive::u8),
						#[codec(index = 146)]
						Mortal146(::core::primitive::u8),
						#[codec(index = 147)]
						Mortal147(::core::primitive::u8),
						#[codec(index = 148)]
						Mortal148(::core::primitive::u8),
						#[codec(index = 149)]
						Mortal149(::core::primitive::u8),
						#[codec(index = 150)]
						Mortal150(::core::primitive::u8),
						#[codec(index = 151)]
						Mortal151(::core::primitive::u8),
						#[codec(index = 152)]
						Mortal152(::core::primitive::u8),
						#[codec(index = 153)]
						Mortal153(::core::primitive::u8),
						#[codec(index = 154)]
						Mortal154(::core::primitive::u8),
						#[codec(index = 155)]
						Mortal155(::core::primitive::u8),
						#[codec(index = 156)]
						Mortal156(::core::primitive::u8),
						#[codec(index = 157)]
						Mortal157(::core::primitive::u8),
						#[codec(index = 158)]
						Mortal158(::core::primitive::u8),
						#[codec(index = 159)]
						Mortal159(::core::primitive::u8),
						#[codec(index = 160)]
						Mortal160(::core::primitive::u8),
						#[codec(index = 161)]
						Mortal161(::core::primitive::u8),
						#[codec(index = 162)]
						Mortal162(::core::primitive::u8),
						#[codec(index = 163)]
						Mortal163(::core::primitive::u8),
						#[codec(index = 164)]
						Mortal164(::core::primitive::u8),
						#[codec(index = 165)]
						Mortal165(::core::primitive::u8),
						#[codec(index = 166)]
						Mortal166(::core::primitive::u8),
						#[codec(index = 167)]
						Mortal167(::core::primitive::u8),
						#[codec(index = 168)]
						Mortal168(::core::primitive::u8),
						#[codec(index = 169)]
						Mortal169(::core::primitive::u8),
						#[codec(index = 170)]
						Mortal170(::core::primitive::u8),
						#[codec(index = 171)]
						Mortal171(::core::primitive::u8),
						#[codec(index = 172)]
						Mortal172(::core::primitive::u8),
						#[codec(index = 173)]
						Mortal173(::core::primitive::u8),
						#[codec(index = 174)]
						Mortal174(::core::primitive::u8),
						#[codec(index = 175)]
						Mortal175(::core::primitive::u8),
						#[codec(index = 176)]
						Mortal176(::core::primitive::u8),
						#[codec(index = 177)]
						Mortal177(::core::primitive::u8),
						#[codec(index = 178)]
						Mortal178(::core::primitive::u8),
						#[codec(index = 179)]
						Mortal179(::core::primitive::u8),
						#[codec(index = 180)]
						Mortal180(::core::primitive::u8),
						#[codec(index = 181)]
						Mortal181(::core::primitive::u8),
						#[codec(index = 182)]
						Mortal182(::core::primitive::u8),
						#[codec(index = 183)]
						Mortal183(::core::primitive::u8),
						#[codec(index = 184)]
						Mortal184(::core::primitive::u8),
						#[codec(index = 185)]
						Mortal185(::core::primitive::u8),
						#[codec(index = 186)]
						Mortal186(::core::primitive::u8),
						#[codec(index = 187)]
						Mortal187(::core::primitive::u8),
						#[codec(index = 188)]
						Mortal188(::core::primitive::u8),
						#[codec(index = 189)]
						Mortal189(::core::primitive::u8),
						#[codec(index = 190)]
						Mortal190(::core::primitive::u8),
						#[codec(index = 191)]
						Mortal191(::core::primitive::u8),
						#[codec(index = 192)]
						Mortal192(::core::primitive::u8),
						#[codec(index = 193)]
						Mortal193(::core::primitive::u8),
						#[codec(index = 194)]
						Mortal194(::core::primitive::u8),
						#[codec(index = 195)]
						Mortal195(::core::primitive::u8),
						#[codec(index = 196)]
						Mortal196(::core::primitive::u8),
						#[codec(index = 197)]
						Mortal197(::core::primitive::u8),
						#[codec(index = 198)]
						Mortal198(::core::primitive::u8),
						#[codec(index = 199)]
						Mortal199(::core::primitive::u8),
						#[codec(index = 200)]
						Mortal200(::core::primitive::u8),
						#[codec(index = 201)]
						Mortal201(::core::primitive::u8),
						#[codec(index = 202)]
						Mortal202(::core::primitive::u8),
						#[codec(index = 203)]
						Mortal203(::core::primitive::u8),
						#[codec(index = 204)]
						Mortal204(::core::primitive::u8),
						#[codec(index = 205)]
						Mortal205(::core::primitive::u8),
						#[codec(index = 206)]
						Mortal206(::core::primitive::u8),
						#[codec(index = 207)]
						Mortal207(::core::primitive::u8),
						#[codec(index = 208)]
						Mortal208(::core::primitive::u8),
						#[codec(index = 209)]
						Mortal209(::core::primitive::u8),
						#[codec(index = 210)]
						Mortal210(::core::primitive::u8),
						#[codec(index = 211)]
						Mortal211(::core::primitive::u8),
						#[codec(index = 212)]
						Mortal212(::core::primitive::u8),
						#[codec(index = 213)]
						Mortal213(::core::primitive::u8),
						#[codec(index = 214)]
						Mortal214(::core::primitive::u8),
						#[codec(index = 215)]
						Mortal215(::core::primitive::u8),
						#[codec(index = 216)]
						Mortal216(::core::primitive::u8),
						#[codec(index = 217)]
						Mortal217(::core::primitive::u8),
						#[codec(index = 218)]
						Mortal218(::core::primitive::u8),
						#[codec(index = 219)]
						Mortal219(::core::primitive::u8),
						#[codec(index = 220)]
						Mortal220(::core::primitive::u8),
						#[codec(index = 221)]
						Mortal221(::core::primitive::u8),
						#[codec(index = 222)]
						Mortal222(::core::primitive::u8),
						#[codec(index = 223)]
						Mortal223(::core::primitive::u8),
						#[codec(index = 224)]
						Mortal224(::core::primitive::u8),
						#[codec(index = 225)]
						Mortal225(::core::primitive::u8),
						#[codec(index = 226)]
						Mortal226(::core::primitive::u8),
						#[codec(index = 227)]
						Mortal227(::core::primitive::u8),
						#[codec(index = 228)]
						Mortal228(::core::primitive::u8),
						#[codec(index = 229)]
						Mortal229(::core::primitive::u8),
						#[codec(index = 230)]
						Mortal230(::core::primitive::u8),
						#[codec(index = 231)]
						Mortal231(::core::primitive::u8),
						#[codec(index = 232)]
						Mortal232(::core::primitive::u8),
						#[codec(index = 233)]
						Mortal233(::core::primitive::u8),
						#[codec(index = 234)]
						Mortal234(::core::primitive::u8),
						#[codec(index = 235)]
						Mortal235(::core::primitive::u8),
						#[codec(index = 236)]
						Mortal236(::core::primitive::u8),
						#[codec(index = 237)]
						Mortal237(::core::primitive::u8),
						#[codec(index = 238)]
						Mortal238(::core::primitive::u8),
						#[codec(index = 239)]
						Mortal239(::core::primitive::u8),
						#[codec(index = 240)]
						Mortal240(::core::primitive::u8),
						#[codec(index = 241)]
						Mortal241(::core::primitive::u8),
						#[codec(index = 242)]
						Mortal242(::core::primitive::u8),
						#[codec(index = 243)]
						Mortal243(::core::primitive::u8),
						#[codec(index = 244)]
						Mortal244(::core::primitive::u8),
						#[codec(index = 245)]
						Mortal245(::core::primitive::u8),
						#[codec(index = 246)]
						Mortal246(::core::primitive::u8),
						#[codec(index = 247)]
						Mortal247(::core::primitive::u8),
						#[codec(index = 248)]
						Mortal248(::core::primitive::u8),
						#[codec(index = 249)]
						Mortal249(::core::primitive::u8),
						#[codec(index = 250)]
						Mortal250(::core::primitive::u8),
						#[codec(index = 251)]
						Mortal251(::core::primitive::u8),
						#[codec(index = 252)]
						Mortal252(::core::primitive::u8),
						#[codec(index = 253)]
						Mortal253(::core::primitive::u8),
						#[codec(index = 254)]
						Mortal254(::core::primitive::u8),
						#[codec(index = 255)]
						Mortal255(::core::primitive::u8),
					}
				}
				pub mod header {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct Header<_0> {
						pub parent_hash: ::subxt::utils::H256,
						#[codec(compact)]
						pub number: _0,
						pub state_root: ::subxt::utils::H256,
						pub extrinsics_root: ::subxt::utils::H256,
						pub digest: runtime_types::sp_runtime::generic::digest::Digest,
					}
				}
			}
			pub mod transaction_validity {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum InvalidTransaction {
					#[codec(index = 0)]
					Call,
					#[codec(index = 1)]
					Payment,
					#[codec(index = 2)]
					Future,
					#[codec(index = 3)]
					Stale,
					#[codec(index = 4)]
					BadProof,
					#[codec(index = 5)]
					AncientBirthBlock,
					#[codec(index = 6)]
					ExhaustsResources,
					#[codec(index = 7)]
					Custom(::core::primitive::u8),
					#[codec(index = 8)]
					BadMandatory,
					#[codec(index = 9)]
					MandatoryValidation,
					#[codec(index = 10)]
					BadSigner,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum TransactionSource {
					#[codec(index = 0)]
					InBlock,
					#[codec(index = 1)]
					Local,
					#[codec(index = 2)]
					External,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum TransactionValidityError {
					#[codec(index = 0)]
					Invalid(runtime_types::sp_runtime::transaction_validity::InvalidTransaction),
					#[codec(index = 1)]
					Unknown(runtime_types::sp_runtime::transaction_validity::UnknownTransaction),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum UnknownTransaction {
					#[codec(index = 0)]
					CannotLookup,
					#[codec(index = 1)]
					NoUnsignedValidator,
					#[codec(index = 2)]
					Custom(::core::primitive::u8),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ValidTransaction {
					pub priority: ::core::primitive::u64,
					pub requires: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
					pub provides: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
					pub longevity: ::core::primitive::u64,
					pub propagate: ::core::primitive::bool,
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum DispatchError {
				#[codec(index = 0)]
				Other,
				#[codec(index = 1)]
				CannotLookup,
				#[codec(index = 2)]
				BadOrigin,
				#[codec(index = 3)]
				Module(runtime_types::sp_runtime::ModuleError),
				#[codec(index = 4)]
				ConsumerRemaining,
				#[codec(index = 5)]
				NoProviders,
				#[codec(index = 6)]
				TooManyConsumers,
				#[codec(index = 7)]
				Token(runtime_types::sp_runtime::TokenError),
				#[codec(index = 8)]
				Arithmetic(runtime_types::sp_arithmetic::ArithmeticError),
				#[codec(index = 9)]
				Transactional(runtime_types::sp_runtime::TransactionalError),
				#[codec(index = 10)]
				Exhausted,
				#[codec(index = 11)]
				Corruption,
				#[codec(index = 12)]
				Unavailable,
				#[codec(index = 13)]
				RootNotAllowed,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct ModuleError {
				pub index: ::core::primitive::u8,
				pub error: [::core::primitive::u8; 4usize],
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum MultiSignature {
				#[codec(index = 0)]
				Ed25519(runtime_types::sp_core::ed25519::Signature),
				#[codec(index = 1)]
				Sr25519(runtime_types::sp_core::sr25519::Signature),
				#[codec(index = 2)]
				Ecdsa(runtime_types::sp_core::ecdsa::Signature),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum TokenError {
				#[codec(index = 0)]
				FundsUnavailable,
				#[codec(index = 1)]
				OnlyProvider,
				#[codec(index = 2)]
				BelowMinimum,
				#[codec(index = 3)]
				CannotCreate,
				#[codec(index = 4)]
				UnknownAsset,
				#[codec(index = 5)]
				Frozen,
				#[codec(index = 6)]
				Unsupported,
				#[codec(index = 7)]
				CannotCreateHold,
				#[codec(index = 8)]
				NotExpendable,
				#[codec(index = 9)]
				Blocked,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum TransactionalError {
				#[codec(index = 0)]
				LimitReached,
				#[codec(index = 1)]
				NoLayer,
			}
		}
		pub mod sp_session {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct MembershipProof {
				pub session: ::core::primitive::u32,
				pub trie_nodes: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
				pub validator_count: ::core::primitive::u32,
			}
		}
		pub mod sp_staking {
			use super::runtime_types;
			pub mod offence {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct OffenceDetails<_0, _1> {
					pub offender: _1,
					pub reporters: ::std::vec::Vec<_0>,
				}
			}
		}
		pub mod sp_version {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct RuntimeVersion {
				pub spec_name: ::std::string::String,
				pub impl_name: ::std::string::String,
				pub authoring_version: ::core::primitive::u32,
				pub spec_version: ::core::primitive::u32,
				pub impl_version: ::core::primitive::u32,
				pub apis:
					::std::vec::Vec<([::core::primitive::u8; 8usize], ::core::primitive::u32)>,
				pub transaction_version: ::core::primitive::u32,
				pub state_version: ::core::primitive::u8,
			}
		}
		pub mod sp_weights {
			use super::runtime_types;
			pub mod weight_v2 {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Weight {
					#[codec(compact)]
					pub ref_time: ::core::primitive::u64,
					#[codec(compact)]
					pub proof_size: ::core::primitive::u64,
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct RuntimeDbWeight {
				pub read: ::core::primitive::u64,
				pub write: ::core::primitive::u64,
			}
		}
		pub mod ulx_node_runtime {
			use super::runtime_types;
			pub mod opaque {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SessionKeys {
					pub grandpa: runtime_types::sp_consensus_grandpa::app::Public,
					pub block_seal_authority:
						runtime_types::ulx_primitives::block_seal::app::Public,
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct Runtime;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum RuntimeCall {
				#[codec(index = 0)]
				System(runtime_types::frame_system::pallet::Call),
				#[codec(index = 1)]
				Timestamp(runtime_types::pallet_timestamp::pallet::Call),
				#[codec(index = 2)]
				Ticks(runtime_types::pallet_ticks::pallet::Call),
				#[codec(index = 3)]
				MiningSlot(runtime_types::pallet_mining_slot::pallet::Call),
				#[codec(index = 4)]
				Bond(runtime_types::pallet_bond::pallet::Call),
				#[codec(index = 5)]
				Notaries(runtime_types::pallet_notaries::pallet::Call),
				#[codec(index = 6)]
				Notebook(runtime_types::pallet_notebook::pallet::Call),
				#[codec(index = 7)]
				ChainTransfer(runtime_types::pallet_chain_transfer::pallet::Call),
				#[codec(index = 8)]
				BlockSealSpec(runtime_types::pallet_block_seal_spec::pallet::Call),
				#[codec(index = 11)]
				Session(runtime_types::pallet_session::pallet::Call),
				#[codec(index = 12)]
				BlockSeal(runtime_types::pallet_block_seal::pallet::Call),
				#[codec(index = 13)]
				BlockRewards(runtime_types::pallet_block_rewards::pallet::Call),
				#[codec(index = 14)]
				Grandpa(runtime_types::pallet_grandpa::pallet::Call),
				#[codec(index = 16)]
				ArgonBalances(runtime_types::pallet_balances::pallet::Call),
				#[codec(index = 17)]
				UlixeeBalances(runtime_types::pallet_balances::pallet::Call2),
				#[codec(index = 18)]
				TxPause(runtime_types::pallet_tx_pause::pallet::Call),
				#[codec(index = 20)]
				Sudo(runtime_types::pallet_sudo::pallet::Call),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum RuntimeError {
				#[codec(index = 0)]
				System(runtime_types::frame_system::pallet::Error),
				#[codec(index = 2)]
				Ticks(runtime_types::pallet_ticks::pallet::Error),
				#[codec(index = 3)]
				MiningSlot(runtime_types::pallet_mining_slot::pallet::Error),
				#[codec(index = 4)]
				Bond(runtime_types::pallet_bond::pallet::Error),
				#[codec(index = 5)]
				Notaries(runtime_types::pallet_notaries::pallet::Error),
				#[codec(index = 6)]
				Notebook(runtime_types::pallet_notebook::pallet::Error),
				#[codec(index = 7)]
				ChainTransfer(runtime_types::pallet_chain_transfer::pallet::Error),
				#[codec(index = 8)]
				BlockSealSpec(runtime_types::pallet_block_seal_spec::pallet::Error),
				#[codec(index = 11)]
				Session(runtime_types::pallet_session::pallet::Error),
				#[codec(index = 12)]
				BlockSeal(runtime_types::pallet_block_seal::pallet::Error),
				#[codec(index = 13)]
				BlockRewards(runtime_types::pallet_block_rewards::pallet::Error),
				#[codec(index = 14)]
				Grandpa(runtime_types::pallet_grandpa::pallet::Error),
				#[codec(index = 16)]
				ArgonBalances(runtime_types::pallet_balances::pallet::Error),
				#[codec(index = 17)]
				UlixeeBalances(runtime_types::pallet_balances::pallet::Error2),
				#[codec(index = 18)]
				TxPause(runtime_types::pallet_tx_pause::pallet::Error),
				#[codec(index = 20)]
				Sudo(runtime_types::pallet_sudo::pallet::Error),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum RuntimeEvent {
				#[codec(index = 0)]
				System(runtime_types::frame_system::pallet::Event),
				#[codec(index = 3)]
				MiningSlot(runtime_types::pallet_mining_slot::pallet::Event),
				#[codec(index = 4)]
				Bond(runtime_types::pallet_bond::pallet::Event),
				#[codec(index = 5)]
				Notaries(runtime_types::pallet_notaries::pallet::Event),
				#[codec(index = 6)]
				Notebook(runtime_types::pallet_notebook::pallet::Event),
				#[codec(index = 7)]
				ChainTransfer(runtime_types::pallet_chain_transfer::pallet::Event),
				#[codec(index = 8)]
				BlockSealSpec(runtime_types::pallet_block_seal_spec::pallet::Event),
				#[codec(index = 11)]
				Session(runtime_types::pallet_session::pallet::Event),
				#[codec(index = 13)]
				BlockRewards(runtime_types::pallet_block_rewards::pallet::Event),
				#[codec(index = 14)]
				Grandpa(runtime_types::pallet_grandpa::pallet::Event),
				#[codec(index = 15)]
				Offences(runtime_types::pallet_offences::pallet::Event),
				#[codec(index = 16)]
				ArgonBalances(runtime_types::pallet_balances::pallet::Event),
				#[codec(index = 17)]
				UlixeeBalances(runtime_types::pallet_balances::pallet::Event2),
				#[codec(index = 18)]
				TxPause(runtime_types::pallet_tx_pause::pallet::Event),
				#[codec(index = 19)]
				TransactionPayment(runtime_types::pallet_transaction_payment::pallet::Event),
				#[codec(index = 20)]
				Sudo(runtime_types::pallet_sudo::pallet::Event),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum RuntimeFreezeReason {
				#[codec(index = 13)]
				BlockRewards(runtime_types::pallet_block_rewards::pallet::FreezeReason),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum RuntimeHoldReason {
				#[codec(index = 3)]
				MiningSlot(runtime_types::pallet_mining_slot::pallet::HoldReason),
				#[codec(index = 4)]
				Bond(runtime_types::pallet_bond::pallet::HoldReason),
				#[codec(index = 13)]
				BlockRewards(runtime_types::pallet_block_rewards::pallet::HoldReason),
			}
		}
		pub mod ulx_notary_audit {
			use super::runtime_types;
			pub mod error {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum VerifyError {
					#[codec(index = 0)]
					MissingAccountOrigin {
						account_id: ::subxt::utils::AccountId32,
						account_type: runtime_types::ulx_primitives::note::AccountType,
					},
					#[codec(index = 1)]
					HistoryLookupError {
						source: runtime_types::ulx_notary_audit::AccountHistoryLookupError,
					},
					#[codec(index = 2)]
					InvalidAccountChangelist,
					#[codec(index = 3)]
					InvalidChainTransfersList,
					#[codec(index = 4)]
					InvalidBalanceChangeRoot,
					#[codec(index = 5)]
					InvalidHeaderTaxRecorded,
					#[codec(index = 6)]
					InvalidPreviousNonce,
					#[codec(index = 7)]
					InvalidPreviousBalance,
					#[codec(index = 8)]
					InvalidPreviousAccountOrigin,
					#[codec(index = 9)]
					InvalidPreviousBalanceChangeNotebook,
					#[codec(index = 10)]
					InvalidBalanceChange,
					#[codec(index = 11)]
					InvalidBalanceChangeSignature { change_index: ::core::primitive::u16 },
					#[codec(index = 12)]
					InvalidNoteRecipients,
					#[codec(index = 13)]
					BalanceChangeError {
						change_index: ::core::primitive::u16,
						note_index: ::core::primitive::u16,
						message: ::std::string::String,
					},
					#[codec(index = 14)]
					InvalidNetBalanceChangeset,
					#[codec(index = 15)]
					InsufficientBalance {
						balance: ::core::primitive::u128,
						amount: ::core::primitive::u128,
						note_index: ::core::primitive::u16,
						change_index: ::core::primitive::u16,
					},
					#[codec(index = 16)]
					ExceededMaxBalance {
						balance: ::core::primitive::u128,
						amount: ::core::primitive::u128,
						note_index: ::core::primitive::u16,
						change_index: ::core::primitive::u16,
					},
					#[codec(index = 17)]
					BalanceChangeMismatch {
						change_index: ::core::primitive::u16,
						provided_balance: ::core::primitive::u128,
						calculated_balance: ::core::primitive::i128,
					},
					#[codec(index = 18)]
					BalanceChangeNotNetZero {
						sent: ::core::primitive::u128,
						claimed: ::core::primitive::u128,
					},
					#[codec(index = 19)]
					TaxBalanceChangeNotNetZero {
						sent: ::core::primitive::u128,
						claimed: ::core::primitive::u128,
					},
					#[codec(index = 20)]
					MissingBalanceProof,
					#[codec(index = 21)]
					InvalidPreviousBalanceProof,
					#[codec(index = 22)]
					InvalidNotebookHash,
					#[codec(index = 23)]
					InvalidNotebookHeaderHash,
					#[codec(index = 24)]
					DuplicateChainTransfer,
					#[codec(index = 25)]
					DuplicatedAccountOriginUid,
					#[codec(index = 26)]
					InvalidNotarySignature,
					#[codec(index = 27)]
					NotebookTooOld,
					#[codec(index = 28)]
					DecodeError,
					#[codec(index = 29)]
					AccountChannelHoldDoesntExist,
					#[codec(index = 30)]
					AccountAlreadyHasChannelHold,
					#[codec(index = 31)]
					ChannelHoldNotReadyForClaim,
					#[codec(index = 32)]
					AccountLocked,
					#[codec(index = 33)]
					MissingChannelHoldNote,
					#[codec(index = 34)]
					InvalidChannelHoldNote,
					#[codec(index = 35)]
					InvalidChannelClaimers,
					#[codec(index = 36)]
					ChannelNoteBelowMinimum,
					#[codec(index = 37)]
					InvalidTaxNoteAccount,
					#[codec(index = 38)]
					InvalidTaxOperation,
					#[codec(index = 39)]
					InsufficientTaxIncluded {
						tax_sent: ::core::primitive::u128,
						tax_owed: ::core::primitive::u128,
						account_id: ::subxt::utils::AccountId32,
					},
					#[codec(index = 40)]
					InsufficientBlockVoteTax,
					#[codec(index = 41)]
					InvalidChannelPassSignature,
					#[codec(index = 42)]
					DuplicateChannelPassSettled,
					#[codec(index = 43)]
					InvalidBlockVoteAllocation,
					#[codec(index = 44)]
					InvalidBlockVoteRoot,
					#[codec(index = 45)]
					InvalidBlockVotesCount,
					#[codec(index = 46)]
					InvalidBlockVotingPower,
					#[codec(index = 47)]
					InvalidBlockVoteList,
					#[codec(index = 48)]
					InvalidComputeProof,
					#[codec(index = 49)]
					InvalidBlockVoteSource,
					#[codec(index = 50)]
					InsufficientBlockVoteMinimum,
					#[codec(index = 51)]
					InvalidBlockVoteChannelPass,
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum AccountHistoryLookupError {
				#[codec(index = 0)]
				RootNotFound,
				#[codec(index = 1)]
				LastChangeNotFound,
				#[codec(index = 2)]
				InvalidTransferToLocalchain,
				#[codec(index = 3)]
				BlockSpecificationNotFound,
			}
		}
		pub mod ulx_primitives {
			use super::runtime_types;
			pub mod apis {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct NotebookVotes {
					pub raw_votes: ::std::vec::Vec<(
						::std::vec::Vec<::core::primitive::u8>,
						::core::primitive::u128,
					)>,
				}
			}
			pub mod balance_change {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AccountOrigin {
					#[codec(compact)]
					pub notebook_number: ::core::primitive::u32,
					#[codec(compact)]
					pub account_uid: ::core::primitive::u32,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct MerkleProof {
					pub proof: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::subxt::utils::H256,
					>,
					#[codec(compact)]
					pub number_of_leaves: ::core::primitive::u32,
					#[codec(compact)]
					pub leaf_index: ::core::primitive::u32,
				}
			}
			pub mod block_seal {
				use super::runtime_types;
				pub mod app {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct Public(pub runtime_types::sp_core::ed25519::Public);
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct Signature(pub runtime_types::sp_core::ed25519::Signature);
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Host {
					#[codec(compact)]
					pub ip: ::core::primitive::u32,
					#[codec(compact)]
					pub port: ::core::primitive::u16,
					pub is_secure: ::core::primitive::bool,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct MiningAuthority<_0, _1> {
					#[codec(compact)]
					pub authority_index: ::core::primitive::u16,
					pub authority_id: _0,
					pub account_id: _1,
					pub peer_id: runtime_types::ulx_primitives::block_seal::PeerId,
					pub rpc_hosts: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::ulx_primitives::block_seal::Host,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct MiningRegistration<_0, _1, _2> {
					pub account_id: _0,
					pub peer_id: runtime_types::ulx_primitives::block_seal::PeerId,
					pub rpc_hosts: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::ulx_primitives::block_seal::Host,
					>,
					pub reward_destination:
						runtime_types::ulx_primitives::block_seal::RewardDestination<_0>,
					pub bond_id: ::core::option::Option<_1>,
					#[codec(compact)]
					pub bond_amount: _2,
					#[codec(compact)]
					pub ownership_tokens: _2,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct PeerId(pub runtime_types::sp_core::OpaquePeerId);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum RewardDestination<_0> {
					#[codec(index = 0)]
					Owner,
					#[codec(index = 1)]
					Account(_0),
				}
			}
			pub mod block_vote {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BestBlockVoteProofT<_0> {
					pub vote_proof: runtime_types::primitive_types::U256,
					pub notary_id: ::core::primitive::u32,
					pub block_vote: runtime_types::ulx_primitives::block_vote::BlockVoteT<_0>,
					pub source_notebook_proof:
						runtime_types::ulx_primitives::balance_change::MerkleProof,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BlockVoteT<_0> {
					pub account_id: ::subxt::utils::AccountId32,
					pub grandparent_block_hash: _0,
					#[codec(compact)]
					pub index: ::core::primitive::u32,
					#[codec(compact)]
					pub power: ::core::primitive::u128,
					pub channel_pass: runtime_types::ulx_primitives::note::ChannelPass,
				}
			}
			pub mod bond {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Bond<_0, _1, _2, _3> {
					pub bond_fund_id: ::core::option::Option<_2>,
					pub bonded_account_id: _0,
					#[codec(compact)]
					pub annual_percent_rate: _2,
					#[codec(compact)]
					pub base_fee: _1,
					#[codec(compact)]
					pub fee: _1,
					#[codec(compact)]
					pub amount: _1,
					#[codec(compact)]
					pub start_block: _2,
					#[codec(compact)]
					pub completion_block: _2,
					pub is_locked: ::core::primitive::bool,
					#[codec(skip)]
					pub __subxt_unused_type_params: ::core::marker::PhantomData<_3>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BondFund<_0, _1, _2> {
					#[codec(compact)]
					pub lease_annual_percent_rate: _2,
					#[codec(compact)]
					pub lease_base_fee: _1,
					pub offer_account_id: _0,
					#[codec(compact)]
					pub amount_reserved: _1,
					pub offer_expiration_block: _2,
					#[codec(compact)]
					pub amount_bonded: _1,
					pub is_ended: ::core::primitive::bool,
				}
			}
			pub mod digests {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BlockVoteDigest {
					pub parent_voting_key: ::core::option::Option<::subxt::utils::H256>,
					pub voting_power: ::core::primitive::u128,
					pub votes_count: ::core::primitive::u32,
					pub tick_notebooks: ::core::primitive::u32,
				}
			}
			pub mod inherents {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum BlockSealInherent {
					#[codec(index = 0)]
					Vote {
						vote_proof: runtime_types::primitive_types::U256,
						notary_id: ::core::primitive::u32,
						block_vote: runtime_types::ulx_primitives::block_vote::BlockVoteT<
							::subxt::utils::H256,
						>,
						source_notebook_number: ::core::primitive::u32,
						source_notebook_proof:
							runtime_types::ulx_primitives::balance_change::MerkleProof,
						miner_signature: runtime_types::ulx_primitives::block_seal::app::Signature,
					},
					#[codec(index = 1)]
					Compute,
				}
			}
			pub mod notary {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct NotaryMeta {
					pub public: runtime_types::sp_core::ed25519::Public,
					pub hosts: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::ulx_primitives::block_seal::Host,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct NotaryNotebookKeyDetails {
					pub notebook_number: ::core::primitive::u32,
					pub block_votes_root: ::subxt::utils::H256,
					pub tick: ::core::primitive::u32,
					pub secret_hash: ::subxt::utils::H256,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct NotaryNotebookVoteDetails<_0> {
					pub notary_id: ::core::primitive::u32,
					pub version: ::core::primitive::u32,
					pub notebook_number: ::core::primitive::u32,
					pub tick: ::core::primitive::u32,
					pub secret_hash: ::subxt::utils::H256,
					pub parent_secret: ::core::option::Option<_0>,
					pub finalized_block_number: ::core::primitive::u32,
					pub header_hash: ::subxt::utils::H256,
					pub block_votes_count: ::core::primitive::u32,
					pub block_votes_root: ::subxt::utils::H256,
					pub block_voting_power: ::core::primitive::u128,
					pub blocks_with_votes: ::std::vec::Vec<_0>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct NotaryNotebookVoteDigestDetails {
					pub notary_id: ::core::primitive::u32,
					pub notebook_number: ::core::primitive::u32,
					pub tick: ::core::primitive::u32,
					pub parent_secret: ::core::option::Option<::subxt::utils::H256>,
					pub block_votes_count: ::core::primitive::u32,
					pub block_votes_root: ::subxt::utils::H256,
					pub block_voting_power: ::core::primitive::u128,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct NotaryRecord<_0, _1> {
					#[codec(compact)]
					pub notary_id: _1,
					pub operator_account_id: _0,
					#[codec(compact)]
					pub activated_block: _1,
					#[codec(compact)]
					pub meta_updated_block: _1,
					pub meta: runtime_types::ulx_primitives::notary::NotaryMeta,
				}
			}
			pub mod note {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum AccountType {
					#[codec(index = 0)]
					Tax,
					#[codec(index = 1)]
					Deposit,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ChannelPass {
					#[codec(compact)]
					pub id: ::core::primitive::u64,
					#[codec(compact)]
					pub miner_index: ::core::primitive::u16,
					#[codec(compact)]
					pub at_block_height: ::core::primitive::u32,
					pub zone_record_hash: ::subxt::utils::H256,
				}
			}
			pub mod notebook {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum ChainTransfer {
					#[codec(index = 0)]
					ToMainchain {
						account_id: ::subxt::utils::AccountId32,
						#[codec(compact)]
						amount: ::core::primitive::u128,
					},
					#[codec(index = 1)]
					ToLocalchain {
						account_id: ::subxt::utils::AccountId32,
						#[codec(compact)]
						account_nonce: ::core::primitive::u32,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct NotebookHeader {
					#[codec(compact)]
					pub version: ::core::primitive::u16,
					#[codec(compact)]
					pub notebook_number: ::core::primitive::u32,
					#[codec(compact)]
					pub tick: ::core::primitive::u32,
					#[codec(compact)]
					pub finalized_block_number: ::core::primitive::u32,
					#[codec(compact)]
					pub tax: ::core::primitive::u128,
					#[codec(compact)]
					pub notary_id: ::core::primitive::u32,
					pub chain_transfers:
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::notebook::ChainTransfer,
						>,
					pub changed_accounts_root: ::subxt::utils::H256,
					pub changed_account_origins:
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::balance_change::AccountOrigin,
						>,
					pub block_votes_root: ::subxt::utils::H256,
					#[codec(compact)]
					pub block_votes_count: ::core::primitive::u32,
					pub blocks_with_votes:
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::subxt::utils::H256,
						>,
					#[codec(compact)]
					pub block_voting_power: ::core::primitive::u128,
					pub secret_hash: ::subxt::utils::H256,
					pub parent_secret: ::core::option::Option<::subxt::utils::H256>,
				}
			}
			pub mod providers {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BlockSealerInfo<_0> {
					pub miner_rewards_account: _0,
					pub block_vote_rewards_account: _0,
					pub notaries_included: ::core::primitive::u32,
				}
			}
			pub mod tick {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Ticker {
					pub tick_duration_millis: ::core::primitive::u64,
					pub genesis_utc_time: ::core::primitive::u64,
				}
			}
		}
	}
}
