#[allow(dead_code, unused_imports, non_camel_case_types)]
#[allow(clippy::all)]
#[allow(rustdoc::broken_intra_doc_links)]
pub mod api {
	#[allow(unused_imports)]
	mod root_mod {
		pub use super::*;
	}
	pub static PALLETS: [&str; 28usize] = [
		"System",
		"Timestamp",
		"Multisig",
		"Proxy",
		"Ticks",
		"MiningSlot",
		"BitcoinUtxos",
		"Vaults",
		"Bonds",
		"Notaries",
		"Notebook",
		"ChainTransfer",
		"BlockSealSpec",
		"DataDomain",
		"PriceIndex",
		"Authorship",
		"Historical",
		"Session",
		"BlockSeal",
		"BlockRewards",
		"Grandpa",
		"Offences",
		"Mint",
		"ArgonBalances",
		"UlixeeBalances",
		"TxPause",
		"TransactionPayment",
		"Sudo",
	];
	pub static RUNTIME_APIS: [&str; 18usize] = [
		"Core",
		"Metadata",
		"BlockBuilder",
		"TaggedTransactionQueue",
		"OffchainWorkerApi",
		"AccountNonceApi",
		"SessionKeys",
		"TransactionPaymentApi",
		"TransactionPaymentCallApi",
		"MiningApis",
		"BlockSealApis",
		"NotaryApis",
		"MiningSlotApi",
		"NotebookApis",
		"TickApis",
		"BitcoinApis",
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
		use ::subxt::ext::codec::Encode;
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
			pub fn mining_apis(&self) -> mining_apis::MiningApis {
				mining_apis::MiningApis
			}
			pub fn block_seal_apis(&self) -> block_seal_apis::BlockSealApis {
				block_seal_apis::BlockSealApis
			}
			pub fn notary_apis(&self) -> notary_apis::NotaryApis {
				notary_apis::NotaryApis
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
			pub fn bitcoin_apis(&self) -> bitcoin_apis::BitcoinApis {
				bitcoin_apis::BitcoinApis
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
				) -> ::subxt::runtime_api::Payload<types::Version, types::version::output::Output>
				{
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
					block: types::execute_block::Block,
				) -> ::subxt::runtime_api::Payload<
					types::ExecuteBlock,
					types::execute_block::output::Output,
				> {
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
				#[doc = " Initialize a block with the given header and return the runtime executive mode."]
				pub fn initialize_block(
					&self,
					header: types::initialize_block::Header,
				) -> ::subxt::runtime_api::Payload<
					types::InitializeBlock,
					types::initialize_block::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"Core",
						"initialize_block",
						types::InitializeBlock { header },
						[
							132u8, 169u8, 113u8, 112u8, 80u8, 139u8, 113u8, 35u8, 41u8, 81u8, 36u8,
							35u8, 37u8, 202u8, 29u8, 207u8, 205u8, 229u8, 145u8, 7u8, 133u8, 94u8,
							25u8, 108u8, 233u8, 86u8, 234u8, 29u8, 236u8, 57u8, 56u8, 186u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				pub mod version {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = runtime_types::sp_version::RuntimeVersion;
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
				pub struct Version {}
				pub mod execute_block {
					use super::runtime_types;
					pub type Block = runtime_types :: sp_runtime :: generic :: block :: Block < runtime_types :: sp_runtime :: generic :: header :: Header < :: core :: primitive :: u32 > , :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > > ;
					pub mod output {
						use super::runtime_types;
						pub type Output = ();
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
				pub struct ExecuteBlock {
					pub block: execute_block::Block,
				}
				pub mod initialize_block {
					use super::runtime_types;
					pub type Header =
						runtime_types::sp_runtime::generic::header::Header<::core::primitive::u32>;
					pub mod output {
						use super::runtime_types;
						pub type Output = runtime_types::sp_runtime::ExtrinsicInclusionMode;
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
				pub struct InitializeBlock {
					pub header: initialize_block::Header,
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
				) -> ::subxt::runtime_api::Payload<types::Metadata, types::metadata::output::Output>
				{
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
					version: types::metadata_at_version::Version,
				) -> ::subxt::runtime_api::Payload<
					types::MetadataAtVersion,
					types::metadata_at_version::output::Output,
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
					types::metadata_versions::output::Output,
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
				pub mod metadata {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = runtime_types::sp_core::OpaqueMetadata;
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
				pub struct Metadata {}
				pub mod metadata_at_version {
					use super::runtime_types;
					pub type Version = ::core::primitive::u32;
					pub mod output {
						use super::runtime_types;
						pub type Output =
							::core::option::Option<runtime_types::sp_core::OpaqueMetadata>;
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
				pub struct MetadataAtVersion {
					pub version: metadata_at_version::Version,
				}
				pub mod metadata_versions {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::std::vec::Vec<::core::primitive::u32>;
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
					extrinsic: types::apply_extrinsic::Extrinsic,
				) -> ::subxt::runtime_api::Payload<
					types::ApplyExtrinsic,
					types::apply_extrinsic::output::Output,
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
					types::finalize_block::output::Output,
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
				#[doc = " Generate inherent extrinsics. The inherent data will vary from chain to chain."]
				pub fn inherent_extrinsics(
					&self,
					inherent: types::inherent_extrinsics::Inherent,
				) -> ::subxt::runtime_api::Payload<
					types::InherentExtrinsics,
					types::inherent_extrinsics::output::Output,
				> {
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
					block: types::check_inherents::Block,
					data: types::check_inherents::Data,
				) -> ::subxt::runtime_api::Payload<
					types::CheckInherents,
					types::check_inherents::output::Output,
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
				pub mod apply_extrinsic {
					use super::runtime_types;
					pub type Extrinsic = :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > ;
					pub mod output {
						use super::runtime_types;
						pub type Output = :: core :: result :: Result < :: core :: result :: Result < () , runtime_types :: sp_runtime :: DispatchError > , runtime_types :: sp_runtime :: transaction_validity :: TransactionValidityError > ;
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
				pub struct ApplyExtrinsic {
					pub extrinsic: apply_extrinsic::Extrinsic,
				}
				pub mod finalize_block {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = runtime_types::sp_runtime::generic::header::Header<
							::core::primitive::u32,
						>;
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
				pub struct FinalizeBlock {}
				pub mod inherent_extrinsics {
					use super::runtime_types;
					pub type Inherent = runtime_types::sp_inherents::InherentData;
					pub mod output {
						use super::runtime_types;
						pub type Output = :: std :: vec :: Vec < :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > > ;
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
				pub struct InherentExtrinsics {
					pub inherent: inherent_extrinsics::Inherent,
				}
				pub mod check_inherents {
					use super::runtime_types;
					pub type Block = runtime_types :: sp_runtime :: generic :: block :: Block < runtime_types :: sp_runtime :: generic :: header :: Header < :: core :: primitive :: u32 > , :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > > ;
					pub type Data = runtime_types::sp_inherents::InherentData;
					pub mod output {
						use super::runtime_types;
						pub type Output = runtime_types::sp_inherents::CheckInherentsResult;
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
				pub struct CheckInherents {
					pub block: check_inherents::Block,
					pub data: check_inherents::Data,
				}
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
					source: types::validate_transaction::Source,
					tx: types::validate_transaction::Tx,
					block_hash: types::validate_transaction::BlockHash,
				) -> ::subxt::runtime_api::Payload<
					types::ValidateTransaction,
					types::validate_transaction::output::Output,
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
				pub mod validate_transaction {
					use super::runtime_types;
					pub type Source =
						runtime_types::sp_runtime::transaction_validity::TransactionSource;
					pub type Tx = :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > ;
					pub type BlockHash = ::sp_core::H256;
					pub mod output {
						use super::runtime_types;
						pub type Output = :: core :: result :: Result < runtime_types :: sp_runtime :: transaction_validity :: ValidTransaction , runtime_types :: sp_runtime :: transaction_validity :: TransactionValidityError > ;
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
				pub struct ValidateTransaction {
					pub source: validate_transaction::Source,
					pub tx: validate_transaction::Tx,
					pub block_hash: validate_transaction::BlockHash,
				}
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
					header: types::offchain_worker::Header,
				) -> ::subxt::runtime_api::Payload<
					types::OffchainWorker,
					types::offchain_worker::output::Output,
				> {
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
				pub mod offchain_worker {
					use super::runtime_types;
					pub type Header =
						runtime_types::sp_runtime::generic::header::Header<::core::primitive::u32>;
					pub mod output {
						use super::runtime_types;
						pub type Output = ();
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
				pub struct OffchainWorker {
					pub header: offchain_worker::Header,
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
					account: types::account_nonce::Account,
				) -> ::subxt::runtime_api::Payload<
					types::AccountNonce,
					types::account_nonce::output::Output,
				> {
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
				pub mod account_nonce {
					use super::runtime_types;
					pub type Account = ::subxt::utils::AccountId32;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::primitive::u32;
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
				pub struct AccountNonce {
					pub account: account_nonce::Account,
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
					seed: types::generate_session_keys::Seed,
				) -> ::subxt::runtime_api::Payload<
					types::GenerateSessionKeys,
					types::generate_session_keys::output::Output,
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
					encoded: types::decode_session_keys::Encoded,
				) -> ::subxt::runtime_api::Payload<
					types::DecodeSessionKeys,
					types::decode_session_keys::output::Output,
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
				pub mod generate_session_keys {
					use super::runtime_types;
					pub type Seed = ::core::option::Option<::std::vec::Vec<::core::primitive::u8>>;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::std::vec::Vec<::core::primitive::u8>;
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
				pub struct GenerateSessionKeys {
					pub seed: generate_session_keys::Seed,
				}
				pub mod decode_session_keys {
					use super::runtime_types;
					pub type Encoded = ::std::vec::Vec<::core::primitive::u8>;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::option::Option<
							::std::vec::Vec<(
								::std::vec::Vec<::core::primitive::u8>,
								runtime_types::sp_core::crypto::KeyTypeId,
							)>,
						>;
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
				pub struct DecodeSessionKeys {
					pub encoded: decode_session_keys::Encoded,
				}
			}
		}
		pub mod transaction_payment_api {
			use super::{root_mod, runtime_types};
			pub struct TransactionPaymentApi;
			impl TransactionPaymentApi {
				pub fn query_info(
					&self,
					uxt: types::query_info::Uxt,
					len: types::query_info::Len,
				) -> ::subxt::runtime_api::Payload<
					types::QueryInfo,
					types::query_info::output::Output,
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
					uxt: types::query_fee_details::Uxt,
					len: types::query_fee_details::Len,
				) -> ::subxt::runtime_api::Payload<
					types::QueryFeeDetails,
					types::query_fee_details::output::Output,
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
					weight: types::query_weight_to_fee::Weight,
				) -> ::subxt::runtime_api::Payload<
					types::QueryWeightToFee,
					types::query_weight_to_fee::output::Output,
				> {
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
					length: types::query_length_to_fee::Length,
				) -> ::subxt::runtime_api::Payload<
					types::QueryLengthToFee,
					types::query_length_to_fee::output::Output,
				> {
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
				pub mod query_info {
					use super::runtime_types;
					pub type Uxt = :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > ;
					pub type Len = ::core::primitive::u32;
					pub mod output {
						use super::runtime_types;
						pub type Output =
							runtime_types::pallet_transaction_payment::types::RuntimeDispatchInfo<
								::core::primitive::u128,
								runtime_types::sp_weights::weight_v2::Weight,
							>;
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
				pub struct QueryInfo {
					pub uxt: query_info::Uxt,
					pub len: query_info::Len,
				}
				pub mod query_fee_details {
					use super::runtime_types;
					pub type Uxt = :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: ulx_node_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > ;
					pub type Len = ::core::primitive::u32;
					pub mod output {
						use super::runtime_types;
						pub type Output =
							runtime_types::pallet_transaction_payment::types::FeeDetails<
								::core::primitive::u128,
							>;
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
				pub struct QueryFeeDetails {
					pub uxt: query_fee_details::Uxt,
					pub len: query_fee_details::Len,
				}
				pub mod query_weight_to_fee {
					use super::runtime_types;
					pub type Weight = runtime_types::sp_weights::weight_v2::Weight;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::primitive::u128;
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
				pub struct QueryWeightToFee {
					pub weight: query_weight_to_fee::Weight,
				}
				pub mod query_length_to_fee {
					use super::runtime_types;
					pub type Length = ::core::primitive::u32;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::primitive::u128;
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
				pub struct QueryLengthToFee {
					pub length: query_length_to_fee::Length,
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
					call: types::query_call_info::Call,
					len: types::query_call_info::Len,
				) -> ::subxt::runtime_api::Payload<
					types::QueryCallInfo,
					types::query_call_info::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentCallApi",
						"query_call_info",
						types::QueryCallInfo { call, len },
						[
							76u8, 34u8, 186u8, 69u8, 156u8, 43u8, 133u8, 175u8, 14u8, 38u8, 151u8,
							94u8, 115u8, 42u8, 182u8, 245u8, 144u8, 98u8, 14u8, 68u8, 174u8, 64u8,
							239u8, 47u8, 147u8, 5u8, 20u8, 187u8, 195u8, 177u8, 176u8, 177u8,
						],
					)
				}
				#[doc = " Query fee details of a given encoded `Call`."]
				pub fn query_call_fee_details(
					&self,
					call: types::query_call_fee_details::Call,
					len: types::query_call_fee_details::Len,
				) -> ::subxt::runtime_api::Payload<
					types::QueryCallFeeDetails,
					types::query_call_fee_details::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentCallApi",
						"query_call_fee_details",
						types::QueryCallFeeDetails { call, len },
						[
							153u8, 1u8, 167u8, 207u8, 103u8, 142u8, 128u8, 54u8, 201u8, 198u8,
							227u8, 45u8, 43u8, 136u8, 162u8, 48u8, 239u8, 132u8, 121u8, 171u8,
							169u8, 149u8, 124u8, 88u8, 48u8, 87u8, 18u8, 134u8, 48u8, 96u8, 248u8,
							246u8,
						],
					)
				}
				#[doc = " Query the output of the current `WeightToFee` given some input."]
				pub fn query_weight_to_fee(
					&self,
					weight: types::query_weight_to_fee::Weight,
				) -> ::subxt::runtime_api::Payload<
					types::QueryWeightToFee,
					types::query_weight_to_fee::output::Output,
				> {
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
					length: types::query_length_to_fee::Length,
				) -> ::subxt::runtime_api::Payload<
					types::QueryLengthToFee,
					types::query_length_to_fee::output::Output,
				> {
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
				pub mod query_call_info {
					use super::runtime_types;
					pub type Call = runtime_types::ulx_node_runtime::RuntimeCall;
					pub type Len = ::core::primitive::u32;
					pub mod output {
						use super::runtime_types;
						pub type Output =
							runtime_types::pallet_transaction_payment::types::RuntimeDispatchInfo<
								::core::primitive::u128,
								runtime_types::sp_weights::weight_v2::Weight,
							>;
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
				pub struct QueryCallInfo {
					pub call: query_call_info::Call,
					pub len: query_call_info::Len,
				}
				pub mod query_call_fee_details {
					use super::runtime_types;
					pub type Call = runtime_types::ulx_node_runtime::RuntimeCall;
					pub type Len = ::core::primitive::u32;
					pub mod output {
						use super::runtime_types;
						pub type Output =
							runtime_types::pallet_transaction_payment::types::FeeDetails<
								::core::primitive::u128,
							>;
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
				pub struct QueryCallFeeDetails {
					pub call: query_call_fee_details::Call,
					pub len: query_call_fee_details::Len,
				}
				pub mod query_weight_to_fee {
					use super::runtime_types;
					pub type Weight = runtime_types::sp_weights::weight_v2::Weight;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::primitive::u128;
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
				pub struct QueryWeightToFee {
					pub weight: query_weight_to_fee::Weight,
				}
				pub mod query_length_to_fee {
					use super::runtime_types;
					pub type Length = ::core::primitive::u32;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::primitive::u128;
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
				pub struct QueryLengthToFee {
					pub length: query_length_to_fee::Length,
				}
			}
		}
		pub mod mining_apis {
			use super::{root_mod, runtime_types};
			pub struct MiningApis;
			impl MiningApis {
				pub fn get_authority_id(
					&self,
					account_id: types::get_authority_id::AccountId,
				) -> ::subxt::runtime_api::Payload<
					types::GetAuthorityId,
					types::get_authority_id::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"MiningApis",
						"get_authority_id",
						types::GetAuthorityId { account_id },
						[
							77u8, 76u8, 252u8, 255u8, 70u8, 110u8, 251u8, 108u8, 92u8, 141u8, 6u8,
							122u8, 191u8, 248u8, 214u8, 19u8, 136u8, 46u8, 207u8, 152u8, 27u8,
							241u8, 131u8, 117u8, 28u8, 251u8, 178u8, 207u8, 247u8, 136u8, 204u8,
							164u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				pub mod get_authority_id {
					use super::runtime_types;
					pub type AccountId = ::subxt::utils::AccountId32;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::option::Option<
							runtime_types::ulx_primitives::block_seal::MiningAuthority<
								runtime_types::ulx_primitives::block_seal::app::Public,
								::subxt::utils::AccountId32,
							>,
						>;
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
				pub struct GetAuthorityId {
					pub account_id: get_authority_id::AccountId,
				}
			}
		}
		pub mod block_seal_apis {
			use super::{root_mod, runtime_types};
			pub struct BlockSealApis;
			impl BlockSealApis {
				pub fn vote_minimum(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::VoteMinimum,
					types::vote_minimum::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockSealApis",
						"vote_minimum",
						types::VoteMinimum {},
						[
							34u8, 243u8, 211u8, 48u8, 245u8, 6u8, 82u8, 51u8, 6u8, 166u8, 211u8,
							255u8, 49u8, 101u8, 124u8, 196u8, 54u8, 25u8, 202u8, 165u8, 171u8,
							83u8, 168u8, 132u8, 181u8, 92u8, 125u8, 47u8, 37u8, 172u8, 208u8, 46u8,
						],
					)
				}
				pub fn compute_difficulty(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::ComputeDifficulty,
					types::compute_difficulty::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockSealApis",
						"compute_difficulty",
						types::ComputeDifficulty {},
						[
							149u8, 83u8, 109u8, 227u8, 84u8, 55u8, 195u8, 204u8, 71u8, 92u8, 148u8,
							180u8, 227u8, 192u8, 22u8, 15u8, 33u8, 41u8, 176u8, 238u8, 15u8, 218u8,
							52u8, 183u8, 182u8, 199u8, 174u8, 83u8, 84u8, 180u8, 176u8, 57u8,
						],
					)
				}
				pub fn create_vote_digest(
					&self,
					tick: types::create_vote_digest::Tick,
					included_notebooks: types::create_vote_digest::IncludedNotebooks,
				) -> ::subxt::runtime_api::Payload<
					types::CreateVoteDigest,
					types::create_vote_digest::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockSealApis",
						"create_vote_digest",
						types::CreateVoteDigest { tick, included_notebooks },
						[
							212u8, 58u8, 178u8, 158u8, 47u8, 84u8, 233u8, 9u8, 218u8, 195u8, 151u8,
							229u8, 77u8, 46u8, 81u8, 95u8, 40u8, 152u8, 181u8, 94u8, 27u8, 112u8,
							56u8, 152u8, 11u8, 35u8, 209u8, 138u8, 79u8, 24u8, 30u8, 94u8,
						],
					)
				}
				pub fn find_vote_block_seals(
					&self,
					votes: types::find_vote_block_seals::Votes,
					with_better_strength: types::find_vote_block_seals::WithBetterStrength,
				) -> ::subxt::runtime_api::Payload<
					types::FindVoteBlockSeals,
					types::find_vote_block_seals::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockSealApis",
						"find_vote_block_seals",
						types::FindVoteBlockSeals { votes, with_better_strength },
						[
							11u8, 172u8, 29u8, 248u8, 46u8, 146u8, 69u8, 138u8, 206u8, 18u8, 2u8,
							200u8, 125u8, 106u8, 244u8, 88u8, 44u8, 56u8, 57u8, 91u8, 130u8, 39u8,
							101u8, 78u8, 29u8, 159u8, 40u8, 45u8, 107u8, 180u8, 86u8, 140u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				pub mod vote_minimum {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::primitive::u128;
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
				pub struct VoteMinimum {}
				pub mod compute_difficulty {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::primitive::u128;
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
				pub struct ComputeDifficulty {}
				pub mod create_vote_digest {
					use super::runtime_types;
					pub type Tick = ::core::primitive::u32;
					pub type IncludedNotebooks = ::std::vec::Vec<
						runtime_types::ulx_primitives::notary::NotaryNotebookVoteDigestDetails,
					>;
					pub mod output {
						use super::runtime_types;
						pub type Output = runtime_types::ulx_primitives::digests::BlockVoteDigest;
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
				pub struct CreateVoteDigest {
					pub tick: create_vote_digest::Tick,
					pub included_notebooks: create_vote_digest::IncludedNotebooks,
				}
				pub mod find_vote_block_seals {
					use super::runtime_types;
					pub type Votes =
						::std::vec::Vec<runtime_types::ulx_primitives::apis::NotaryNotebookVotes>;
					pub type WithBetterStrength = runtime_types::primitive_types::U256;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::result::Result<
							runtime_types::bounded_collections::bounded_vec::BoundedVec<
								runtime_types::ulx_primitives::block_vote::BestBlockVoteSeal<
									::subxt::utils::AccountId32,
									runtime_types::ulx_primitives::block_seal::app::Public,
								>,
							>,
							runtime_types::sp_runtime::DispatchError,
						>;
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
				pub struct FindVoteBlockSeals {
					pub votes: find_vote_block_seals::Votes,
					pub with_better_strength: find_vote_block_seals::WithBetterStrength,
				}
			}
		}
		pub mod notary_apis {
			use super::{root_mod, runtime_types};
			pub struct NotaryApis;
			impl NotaryApis {
				pub fn notary_by_id(
					&self,
					notary_id: types::notary_by_id::NotaryId,
				) -> ::subxt::runtime_api::Payload<
					types::NotaryById,
					types::notary_by_id::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"NotaryApis",
						"notary_by_id",
						types::NotaryById { notary_id },
						[
							67u8, 89u8, 113u8, 93u8, 230u8, 187u8, 174u8, 251u8, 11u8, 150u8,
							230u8, 190u8, 82u8, 190u8, 48u8, 170u8, 253u8, 3u8, 254u8, 175u8,
							156u8, 118u8, 196u8, 149u8, 147u8, 90u8, 150u8, 109u8, 123u8, 190u8,
							155u8, 92u8,
						],
					)
				}
				pub fn notaries(
					&self,
				) -> ::subxt::runtime_api::Payload<types::Notaries, types::notaries::output::Output>
				{
					::subxt::runtime_api::Payload::new_static(
						"NotaryApis",
						"notaries",
						types::Notaries {},
						[
							255u8, 199u8, 119u8, 53u8, 165u8, 244u8, 14u8, 63u8, 108u8, 87u8, 40u8,
							202u8, 181u8, 23u8, 18u8, 118u8, 27u8, 128u8, 232u8, 24u8, 124u8, 13u8,
							193u8, 65u8, 121u8, 113u8, 80u8, 54u8, 29u8, 238u8, 191u8, 221u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				pub mod notary_by_id {
					use super::runtime_types;
					pub type NotaryId = ::core::primitive::u32;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::option::Option<
							runtime_types::ulx_primitives::notary::NotaryRecord<
								::subxt::utils::AccountId32,
								::core::primitive::u32,
							>,
						>;
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
				pub struct NotaryById {
					pub notary_id: notary_by_id::NotaryId,
				}
				pub mod notaries {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::std::vec::Vec<
							runtime_types::ulx_primitives::notary::NotaryRecord<
								::subxt::utils::AccountId32,
								::core::primitive::u32,
							>,
						>;
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
				pub struct Notaries {}
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
					types::next_slot_era::output::Output,
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
				pub mod next_slot_era {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = (::core::primitive::u32, ::core::primitive::u32);
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
				pub struct NextSlotEra {}
			}
		}
		pub mod notebook_apis {
			use super::{root_mod, runtime_types};
			pub struct NotebookApis;
			impl NotebookApis {
				pub fn audit_notebook_and_get_votes(
					&self,
					version: types::audit_notebook_and_get_votes::Version,
					notary_id: types::audit_notebook_and_get_votes::NotaryId,
					notebook_number: types::audit_notebook_and_get_votes::NotebookNumber,
					header_hash: types::audit_notebook_and_get_votes::HeaderHash,
					vote_minimums: types::audit_notebook_and_get_votes::VoteMinimums,
					bytes: types::audit_notebook_and_get_votes::Bytes,
					audit_dependency_summaries : types :: audit_notebook_and_get_votes :: AuditDependencySummaries,
				) -> ::subxt::runtime_api::Payload<
					types::AuditNotebookAndGetVotes,
					types::audit_notebook_and_get_votes::output::Output,
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
							audit_dependency_summaries,
						},
						[
							12u8, 0u8, 42u8, 2u8, 96u8, 188u8, 239u8, 75u8, 246u8, 170u8, 18u8,
							213u8, 30u8, 73u8, 197u8, 224u8, 83u8, 70u8, 126u8, 80u8, 28u8, 147u8,
							238u8, 83u8, 25u8, 45u8, 150u8, 240u8, 31u8, 71u8, 241u8, 50u8,
						],
					)
				}
				pub fn decode_signed_raw_notebook_header(
					&self,
					raw_header: types::decode_signed_raw_notebook_header::RawHeader,
				) -> ::subxt::runtime_api::Payload<
					types::DecodeSignedRawNotebookHeader,
					types::decode_signed_raw_notebook_header::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"NotebookApis",
						"decode_signed_raw_notebook_header",
						types::DecodeSignedRawNotebookHeader { raw_header },
						[
							161u8, 122u8, 93u8, 183u8, 54u8, 151u8, 145u8, 232u8, 214u8, 0u8,
							254u8, 196u8, 214u8, 65u8, 221u8, 233u8, 63u8, 175u8, 209u8, 15u8,
							227u8, 20u8, 89u8, 192u8, 50u8, 46u8, 159u8, 34u8, 44u8, 2u8, 125u8,
							57u8,
						],
					)
				}
				pub fn latest_notebook_by_notary(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::LatestNotebookByNotary,
					types::latest_notebook_by_notary::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"NotebookApis",
						"latest_notebook_by_notary",
						types::LatestNotebookByNotary {},
						[
							85u8, 20u8, 202u8, 169u8, 17u8, 113u8, 81u8, 236u8, 115u8, 197u8,
							120u8, 136u8, 102u8, 113u8, 49u8, 102u8, 175u8, 238u8, 64u8, 34u8,
							88u8, 80u8, 194u8, 239u8, 232u8, 40u8, 227u8, 162u8, 135u8, 203u8,
							122u8, 236u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				pub mod audit_notebook_and_get_votes {
					use super::runtime_types;
					pub type Version = ::core::primitive::u32;
					pub type NotaryId = ::core::primitive::u32;
					pub type NotebookNumber = ::core::primitive::u32;
					pub type HeaderHash = ::sp_core::H256;
					pub type VoteMinimums =
						::subxt::utils::KeyedVec<::sp_core::H256, ::core::primitive::u128>;
					pub type Bytes = ::std::vec::Vec<::core::primitive::u8>;
					pub type AuditDependencySummaries =
						::std::vec::Vec<runtime_types::ulx_primitives::apis::NotebookAuditSummary>;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::result::Result<
							runtime_types::ulx_primitives::apis::NotebookAuditResult,
							runtime_types::ulx_notary_audit::error::VerifyError,
						>;
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
				pub struct AuditNotebookAndGetVotes {
					pub version: audit_notebook_and_get_votes::Version,
					pub notary_id: audit_notebook_and_get_votes::NotaryId,
					pub notebook_number: audit_notebook_and_get_votes::NotebookNumber,
					pub header_hash: audit_notebook_and_get_votes::HeaderHash,
					pub vote_minimums: audit_notebook_and_get_votes::VoteMinimums,
					pub bytes: audit_notebook_and_get_votes::Bytes,
					pub audit_dependency_summaries:
						audit_notebook_and_get_votes::AuditDependencySummaries,
				}
				pub mod decode_signed_raw_notebook_header {
					use super::runtime_types;
					pub type RawHeader = ::std::vec::Vec<::core::primitive::u8>;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::result::Result<
							runtime_types::ulx_primitives::notary::NotaryNotebookVoteDetails<
								::sp_core::H256,
							>,
							runtime_types::sp_runtime::DispatchError,
						>;
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
				pub struct DecodeSignedRawNotebookHeader {
					pub raw_header: decode_signed_raw_notebook_header::RawHeader,
				}
				pub mod latest_notebook_by_notary {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::subxt::utils::KeyedVec<
							::core::primitive::u32,
							(::core::primitive::u32, ::core::primitive::u32),
						>;
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
				pub struct LatestNotebookByNotary {}
			}
		}
		pub mod tick_apis {
			use super::{root_mod, runtime_types};
			pub struct TickApis;
			impl TickApis {
				pub fn current_tick(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::CurrentTick,
					types::current_tick::output::Output,
				> {
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
				) -> ::subxt::runtime_api::Payload<types::Ticker, types::ticker::output::Output>
				{
					::subxt::runtime_api::Payload::new_static(
						"TickApis",
						"ticker",
						types::Ticker {},
						[
							242u8, 50u8, 78u8, 194u8, 192u8, 155u8, 42u8, 156u8, 182u8, 142u8, 8u8,
							147u8, 11u8, 233u8, 105u8, 22u8, 191u8, 183u8, 38u8, 35u8, 161u8, 21u8,
							187u8, 143u8, 253u8, 24u8, 219u8, 219u8, 215u8, 48u8, 217u8, 18u8,
						],
					)
				}
				pub fn blocks_at_tick(
					&self,
					tick: types::blocks_at_tick::Tick,
				) -> ::subxt::runtime_api::Payload<
					types::BlocksAtTick,
					types::blocks_at_tick::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TickApis",
						"blocks_at_tick",
						types::BlocksAtTick { tick },
						[
							24u8, 144u8, 142u8, 178u8, 118u8, 93u8, 62u8, 204u8, 18u8, 106u8, 41u8,
							140u8, 137u8, 26u8, 109u8, 47u8, 252u8, 163u8, 76u8, 164u8, 253u8,
							248u8, 114u8, 130u8, 199u8, 246u8, 96u8, 13u8, 96u8, 242u8, 159u8,
							47u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				pub mod current_tick {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::primitive::u32;
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
				pub struct CurrentTick {}
				pub mod ticker {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = runtime_types::ulx_primitives::tick::Ticker;
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
				pub struct Ticker {}
				pub mod blocks_at_tick {
					use super::runtime_types;
					pub type Tick = ::core::primitive::u32;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::std::vec::Vec<::sp_core::H256>;
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
				pub struct BlocksAtTick {
					pub tick: blocks_at_tick::Tick,
				}
			}
		}
		pub mod bitcoin_apis {
			use super::{root_mod, runtime_types};
			pub struct BitcoinApis;
			impl BitcoinApis {
				pub fn get_sync_status(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::GetSyncStatus,
					types::get_sync_status::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BitcoinApis",
						"get_sync_status",
						types::GetSyncStatus {},
						[
							53u8, 173u8, 205u8, 55u8, 220u8, 217u8, 10u8, 179u8, 210u8, 26u8,
							181u8, 247u8, 187u8, 49u8, 14u8, 21u8, 116u8, 96u8, 191u8, 101u8, 1u8,
							236u8, 4u8, 209u8, 136u8, 98u8, 127u8, 123u8, 99u8, 73u8, 122u8, 88u8,
						],
					)
				}
				pub fn active_utxos(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::ActiveUtxos,
					types::active_utxos::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BitcoinApis",
						"active_utxos",
						types::ActiveUtxos {},
						[
							79u8, 203u8, 54u8, 14u8, 237u8, 17u8, 106u8, 8u8, 19u8, 65u8, 183u8,
							52u8, 151u8, 147u8, 249u8, 46u8, 132u8, 58u8, 131u8, 162u8, 243u8,
							43u8, 158u8, 188u8, 213u8, 82u8, 97u8, 222u8, 89u8, 35u8, 249u8, 159u8,
						],
					)
				}
				pub fn redemption_rate(
					&self,
					satoshis: types::redemption_rate::Satoshis,
				) -> ::subxt::runtime_api::Payload<
					types::RedemptionRate,
					types::redemption_rate::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BitcoinApis",
						"redemption_rate",
						types::RedemptionRate { satoshis },
						[
							245u8, 56u8, 160u8, 154u8, 180u8, 3u8, 245u8, 231u8, 157u8, 229u8,
							249u8, 223u8, 96u8, 211u8, 207u8, 170u8, 111u8, 150u8, 177u8, 246u8,
							89u8, 216u8, 135u8, 221u8, 225u8, 238u8, 219u8, 155u8, 149u8, 162u8,
							182u8, 107u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				pub mod get_sync_status {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::option::Option<
							runtime_types::ulx_primitives::bitcoin::BitcoinSyncStatus,
						>;
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
				pub struct GetSyncStatus {}
				pub mod active_utxos {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::std::vec::Vec<(
							::core::option::Option<runtime_types::ulx_primitives::bitcoin::UtxoRef>,
							runtime_types::ulx_primitives::bitcoin::UtxoValue,
						)>;
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
				pub struct ActiveUtxos {}
				pub mod redemption_rate {
					use super::runtime_types;
					pub type Satoshis = ::core::primitive::u64;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::option::Option<::core::primitive::u128>;
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
				pub struct RedemptionRate {
					pub satoshis: redemption_rate::Satoshis,
				}
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
					types::grandpa_authorities::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GrandpaApi",
						"grandpa_authorities",
						types::GrandpaAuthorities {},
						[
							8u8, 1u8, 99u8, 227u8, 52u8, 95u8, 230u8, 139u8, 198u8, 90u8, 159u8,
							146u8, 193u8, 81u8, 37u8, 27u8, 216u8, 227u8, 108u8, 126u8, 12u8, 94u8,
							125u8, 183u8, 143u8, 231u8, 87u8, 101u8, 114u8, 190u8, 193u8, 180u8,
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
					equivocation_proof : types :: submit_report_equivocation_unsigned_extrinsic :: EquivocationProof,
					key_owner_proof : types :: submit_report_equivocation_unsigned_extrinsic :: KeyOwnerProof,
				) -> ::subxt::runtime_api::Payload<
					types::SubmitReportEquivocationUnsignedExtrinsic,
					types::submit_report_equivocation_unsigned_extrinsic::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GrandpaApi",
						"submit_report_equivocation_unsigned_extrinsic",
						types::SubmitReportEquivocationUnsignedExtrinsic {
							equivocation_proof,
							key_owner_proof,
						},
						[
							27u8, 32u8, 16u8, 79u8, 172u8, 124u8, 44u8, 13u8, 176u8, 89u8, 69u8,
							60u8, 45u8, 176u8, 72u8, 151u8, 252u8, 5u8, 243u8, 82u8, 170u8, 51u8,
							179u8, 197u8, 117u8, 177u8, 110u8, 111u8, 97u8, 15u8, 109u8, 169u8,
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
					set_id: types::generate_key_ownership_proof::SetId,
					authority_id: types::generate_key_ownership_proof::AuthorityId,
				) -> ::subxt::runtime_api::Payload<
					types::GenerateKeyOwnershipProof,
					types::generate_key_ownership_proof::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GrandpaApi",
						"generate_key_ownership_proof",
						types::GenerateKeyOwnershipProof { set_id, authority_id },
						[
							13u8, 144u8, 66u8, 235u8, 24u8, 190u8, 39u8, 75u8, 29u8, 157u8, 215u8,
							181u8, 173u8, 145u8, 224u8, 244u8, 189u8, 79u8, 6u8, 116u8, 139u8,
							196u8, 54u8, 16u8, 89u8, 190u8, 121u8, 43u8, 137u8, 150u8, 117u8, 68u8,
						],
					)
				}
				#[doc = " Get current GRANDPA authority set id."]
				pub fn current_set_id(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::CurrentSetId,
					types::current_set_id::output::Output,
				> {
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
				pub mod grandpa_authorities {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::std::vec::Vec<(
							runtime_types::sp_consensus_grandpa::app::Public,
							::core::primitive::u64,
						)>;
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
				pub struct GrandpaAuthorities {}
				pub mod submit_report_equivocation_unsigned_extrinsic {
					use super::runtime_types;
					pub type EquivocationProof =
						runtime_types::sp_consensus_grandpa::EquivocationProof<
							::sp_core::H256,
							::core::primitive::u32,
						>;
					pub type KeyOwnerProof =
						runtime_types::sp_consensus_grandpa::OpaqueKeyOwnershipProof;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::option::Option<()>;
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
				pub struct SubmitReportEquivocationUnsignedExtrinsic {
					pub equivocation_proof:
						submit_report_equivocation_unsigned_extrinsic::EquivocationProof,
					pub key_owner_proof:
						submit_report_equivocation_unsigned_extrinsic::KeyOwnerProof,
				}
				pub mod generate_key_ownership_proof {
					use super::runtime_types;
					pub type SetId = ::core::primitive::u64;
					pub type AuthorityId = runtime_types::sp_consensus_grandpa::app::Public;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::option::Option<
							runtime_types::sp_consensus_grandpa::OpaqueKeyOwnershipProof,
						>;
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
				pub struct GenerateKeyOwnershipProof {
					pub set_id: generate_key_ownership_proof::SetId,
					pub authority_id: generate_key_ownership_proof::AuthorityId,
				}
				pub mod current_set_id {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::primitive::u64;
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
				pub struct CurrentSetId {}
			}
		}
		pub mod genesis_builder {
			use super::{root_mod, runtime_types};
			#[doc = " API to interact with RuntimeGenesisConfig for the runtime"]
			pub struct GenesisBuilder;
			impl GenesisBuilder {
				#[doc = " Build `RuntimeGenesisConfig` from a JSON blob not using any defaults and store it in the"]
				#[doc = " storage."]
				#[doc = ""]
				#[doc = " In the case of a FRAME-based runtime, this function deserializes the full `RuntimeGenesisConfig` from the given JSON blob and"]
				#[doc = " puts it into the storage. If the provided JSON blob is incorrect or incomplete or the"]
				#[doc = " deserialization fails, an error is returned."]
				#[doc = ""]
				#[doc = " Please note that provided JSON blob must contain all `RuntimeGenesisConfig` fields, no"]
				#[doc = " defaults will be used."]
				pub fn build_state(
					&self,
					json: types::build_state::Json,
				) -> ::subxt::runtime_api::Payload<
					types::BuildState,
					types::build_state::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GenesisBuilder",
						"build_state",
						types::BuildState { json },
						[
							203u8, 233u8, 104u8, 116u8, 111u8, 131u8, 201u8, 235u8, 117u8, 116u8,
							140u8, 185u8, 93u8, 25u8, 155u8, 210u8, 56u8, 49u8, 23u8, 32u8, 253u8,
							92u8, 149u8, 241u8, 85u8, 245u8, 137u8, 45u8, 209u8, 189u8, 81u8, 2u8,
						],
					)
				}
				#[doc = " Returns a JSON blob representation of the built-in `RuntimeGenesisConfig` identified by"]
				#[doc = " `id`."]
				#[doc = ""]
				#[doc = " If `id` is `None` the function returns JSON blob representation of the default"]
				#[doc = " `RuntimeGenesisConfig` struct of the runtime. Implementation must provide default"]
				#[doc = " `RuntimeGenesisConfig`."]
				#[doc = ""]
				#[doc = " Otherwise function returns a JSON representation of the built-in, named"]
				#[doc = " `RuntimeGenesisConfig` preset identified by `id`, or `None` if such preset does not"]
				#[doc = " exists. Returned `Vec<u8>` contains bytes of JSON blob (patch) which comprises a list of"]
				#[doc = " (potentially nested) key-value pairs that are intended for customizing the default"]
				#[doc = " runtime genesis config. The patch shall be merged (rfc7386) with the JSON representation"]
				#[doc = " of the default `RuntimeGenesisConfig` to create a comprehensive genesis config that can"]
				#[doc = " be used in `build_state` method."]
				pub fn get_preset(
					&self,
					id: types::get_preset::Id,
				) -> ::subxt::runtime_api::Payload<
					types::GetPreset,
					types::get_preset::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GenesisBuilder",
						"get_preset",
						types::GetPreset { id },
						[
							43u8, 153u8, 23u8, 52u8, 113u8, 161u8, 227u8, 122u8, 169u8, 135u8,
							119u8, 8u8, 128u8, 33u8, 143u8, 235u8, 13u8, 173u8, 58u8, 121u8, 178u8,
							223u8, 66u8, 217u8, 22u8, 244u8, 168u8, 113u8, 202u8, 186u8, 241u8,
							124u8,
						],
					)
				}
				#[doc = " Returns a list of identifiers for available builtin `RuntimeGenesisConfig` presets."]
				#[doc = ""]
				#[doc = " The presets from the list can be queried with [`GenesisBuilder::get_preset`] method. If"]
				#[doc = " no named presets are provided by the runtime the list is empty."]
				pub fn preset_names(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::PresetNames,
					types::preset_names::output::Output,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GenesisBuilder",
						"preset_names",
						types::PresetNames {},
						[
							150u8, 117u8, 54u8, 129u8, 221u8, 130u8, 186u8, 71u8, 13u8, 140u8,
							77u8, 180u8, 141u8, 37u8, 22u8, 219u8, 149u8, 218u8, 186u8, 206u8,
							80u8, 42u8, 165u8, 41u8, 99u8, 184u8, 73u8, 37u8, 125u8, 188u8, 167u8,
							122u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				pub mod build_state {
					use super::runtime_types;
					pub type Json = ::std::vec::Vec<::core::primitive::u8>;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::core::result::Result<(), ::std::string::String>;
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
				pub struct BuildState {
					pub json: build_state::Json,
				}
				pub mod get_preset {
					use super::runtime_types;
					pub type Id = ::core::option::Option<::std::string::String>;
					pub mod output {
						use super::runtime_types;
						pub type Output =
							::core::option::Option<::std::vec::Vec<::core::primitive::u8>>;
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
				pub struct GetPreset {
					pub id: get_preset::Id,
				}
				pub mod preset_names {
					use super::runtime_types;
					pub mod output {
						use super::runtime_types;
						pub type Output = ::std::vec::Vec<::std::string::String>;
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
				pub struct PresetNames {}
			}
		}
	}
	pub fn custom() -> CustomValuesApi {
		CustomValuesApi
	}
	pub struct CustomValuesApi;
	impl CustomValuesApi {}
	pub struct ConstantsApi;
	impl ConstantsApi {
		pub fn system(&self) -> system::constants::ConstantsApi {
			system::constants::ConstantsApi
		}
		pub fn timestamp(&self) -> timestamp::constants::ConstantsApi {
			timestamp::constants::ConstantsApi
		}
		pub fn multisig(&self) -> multisig::constants::ConstantsApi {
			multisig::constants::ConstantsApi
		}
		pub fn proxy(&self) -> proxy::constants::ConstantsApi {
			proxy::constants::ConstantsApi
		}
		pub fn mining_slot(&self) -> mining_slot::constants::ConstantsApi {
			mining_slot::constants::ConstantsApi
		}
		pub fn bitcoin_utxos(&self) -> bitcoin_utxos::constants::ConstantsApi {
			bitcoin_utxos::constants::ConstantsApi
		}
		pub fn vaults(&self) -> vaults::constants::ConstantsApi {
			vaults::constants::ConstantsApi
		}
		pub fn bonds(&self) -> bonds::constants::ConstantsApi {
			bonds::constants::ConstantsApi
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
		pub fn price_index(&self) -> price_index::constants::ConstantsApi {
			price_index::constants::ConstantsApi
		}
		pub fn block_rewards(&self) -> block_rewards::constants::ConstantsApi {
			block_rewards::constants::ConstantsApi
		}
		pub fn grandpa(&self) -> grandpa::constants::ConstantsApi {
			grandpa::constants::ConstantsApi
		}
		pub fn mint(&self) -> mint::constants::ConstantsApi {
			mint::constants::ConstantsApi
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
		pub fn multisig(&self) -> multisig::storage::StorageApi {
			multisig::storage::StorageApi
		}
		pub fn proxy(&self) -> proxy::storage::StorageApi {
			proxy::storage::StorageApi
		}
		pub fn ticks(&self) -> ticks::storage::StorageApi {
			ticks::storage::StorageApi
		}
		pub fn mining_slot(&self) -> mining_slot::storage::StorageApi {
			mining_slot::storage::StorageApi
		}
		pub fn bitcoin_utxos(&self) -> bitcoin_utxos::storage::StorageApi {
			bitcoin_utxos::storage::StorageApi
		}
		pub fn vaults(&self) -> vaults::storage::StorageApi {
			vaults::storage::StorageApi
		}
		pub fn bonds(&self) -> bonds::storage::StorageApi {
			bonds::storage::StorageApi
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
		pub fn data_domain(&self) -> data_domain::storage::StorageApi {
			data_domain::storage::StorageApi
		}
		pub fn price_index(&self) -> price_index::storage::StorageApi {
			price_index::storage::StorageApi
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
		pub fn mint(&self) -> mint::storage::StorageApi {
			mint::storage::StorageApi
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
		pub fn multisig(&self) -> multisig::calls::TransactionApi {
			multisig::calls::TransactionApi
		}
		pub fn proxy(&self) -> proxy::calls::TransactionApi {
			proxy::calls::TransactionApi
		}
		pub fn ticks(&self) -> ticks::calls::TransactionApi {
			ticks::calls::TransactionApi
		}
		pub fn mining_slot(&self) -> mining_slot::calls::TransactionApi {
			mining_slot::calls::TransactionApi
		}
		pub fn bitcoin_utxos(&self) -> bitcoin_utxos::calls::TransactionApi {
			bitcoin_utxos::calls::TransactionApi
		}
		pub fn vaults(&self) -> vaults::calls::TransactionApi {
			vaults::calls::TransactionApi
		}
		pub fn bonds(&self) -> bonds::calls::TransactionApi {
			bonds::calls::TransactionApi
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
		pub fn data_domain(&self) -> data_domain::calls::TransactionApi {
			data_domain::calls::TransactionApi
		}
		pub fn price_index(&self) -> price_index::calls::TransactionApi {
			price_index::calls::TransactionApi
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
		pub fn mint(&self) -> mint::calls::TransactionApi {
			mint::calls::TransactionApi
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
				228u8, 69u8, 196u8, 139u8, 178u8, 40u8, 10u8, 109u8, 91u8, 104u8, 6u8, 104u8, 78u8,
				110u8, 31u8, 205u8, 193u8, 94u8, 91u8, 212u8, 8u8, 196u8, 48u8, 8u8, 90u8, 250u8,
				124u8, 252u8, 252u8, 193u8, 214u8, 108u8,
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
				#[doc = "Make some on-chain remark."]
				#[doc = ""]
				#[doc = "Can be executed by every `origin`."]
				pub struct Remark {
					pub remark: remark::Remark,
				}
				pub mod remark {
					use super::runtime_types;
					pub type Remark = ::std::vec::Vec<::core::primitive::u8>;
				}
				impl ::subxt::blocks::StaticExtrinsic for Remark {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "remark";
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
				#[doc = "Set the number of pages in the WebAssembly environment's heap."]
				pub struct SetHeapPages {
					pub pages: set_heap_pages::Pages,
				}
				pub mod set_heap_pages {
					use super::runtime_types;
					pub type Pages = ::core::primitive::u64;
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
				#[doc = "Set the new runtime code."]
				pub struct SetCode {
					pub code: set_code::Code,
				}
				pub mod set_code {
					use super::runtime_types;
					pub type Code = ::std::vec::Vec<::core::primitive::u8>;
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
				#[doc = "Set the new runtime code without doing any checks of the given `code`."]
				#[doc = ""]
				#[doc = "Note that runtime upgrades will not run if this is called with a not-increasing spec"]
				#[doc = "version!"]
				pub struct SetCodeWithoutChecks {
					pub code: set_code_without_checks::Code,
				}
				pub mod set_code_without_checks {
					use super::runtime_types;
					pub type Code = ::std::vec::Vec<::core::primitive::u8>;
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
				#[doc = "Set some items of storage."]
				pub struct SetStorage {
					pub items: set_storage::Items,
				}
				pub mod set_storage {
					use super::runtime_types;
					pub type Items = ::std::vec::Vec<(
						::std::vec::Vec<::core::primitive::u8>,
						::std::vec::Vec<::core::primitive::u8>,
					)>;
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
				#[doc = "Kill some items from storage."]
				pub struct KillStorage {
					pub keys: kill_storage::Keys,
				}
				pub mod kill_storage {
					use super::runtime_types;
					pub type Keys = ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>;
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
				#[doc = "Kill all storage items with a key that starts with the given prefix."]
				#[doc = ""]
				#[doc = "**NOTE:** We rely on the Root origin to provide us the number of subkeys under"]
				#[doc = "the prefix we are removing to accurately calculate the weight of this function."]
				pub struct KillPrefix {
					pub prefix: kill_prefix::Prefix,
					pub subkeys: kill_prefix::Subkeys,
				}
				pub mod kill_prefix {
					use super::runtime_types;
					pub type Prefix = ::std::vec::Vec<::core::primitive::u8>;
					pub type Subkeys = ::core::primitive::u32;
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
				#[doc = "Make some on-chain remark and emit event."]
				pub struct RemarkWithEvent {
					pub remark: remark_with_event::Remark,
				}
				pub mod remark_with_event {
					use super::runtime_types;
					pub type Remark = ::std::vec::Vec<::core::primitive::u8>;
				}
				impl ::subxt::blocks::StaticExtrinsic for RemarkWithEvent {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "remark_with_event";
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
				#[doc = "Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be supplied"]
				#[doc = "later."]
				#[doc = ""]
				#[doc = "This call requires Root origin."]
				pub struct AuthorizeUpgrade {
					pub code_hash: authorize_upgrade::CodeHash,
				}
				pub mod authorize_upgrade {
					use super::runtime_types;
					pub type CodeHash = ::sp_core::H256;
				}
				impl ::subxt::blocks::StaticExtrinsic for AuthorizeUpgrade {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "authorize_upgrade";
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
				#[doc = "Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be supplied"]
				#[doc = "later."]
				#[doc = ""]
				#[doc = "WARNING: This authorizes an upgrade that will take place without any safety checks, for"]
				#[doc = "example that the spec name remains the same and that the version number increases. Not"]
				#[doc = "recommended for normal use. Use `authorize_upgrade` instead."]
				#[doc = ""]
				#[doc = "This call requires Root origin."]
				pub struct AuthorizeUpgradeWithoutChecks {
					pub code_hash: authorize_upgrade_without_checks::CodeHash,
				}
				pub mod authorize_upgrade_without_checks {
					use super::runtime_types;
					pub type CodeHash = ::sp_core::H256;
				}
				impl ::subxt::blocks::StaticExtrinsic for AuthorizeUpgradeWithoutChecks {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "authorize_upgrade_without_checks";
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
				#[doc = "Provide the preimage (runtime binary) `code` for an upgrade that has been authorized."]
				#[doc = ""]
				#[doc = "If the authorization required a version check, this call will ensure the spec name"]
				#[doc = "remains unchanged and that the spec version has increased."]
				#[doc = ""]
				#[doc = "Depending on the runtime's `OnSetCode` configuration, this function may directly apply"]
				#[doc = "the new `code` in the same block or attempt to schedule the upgrade."]
				#[doc = ""]
				#[doc = "All origins are allowed."]
				pub struct ApplyAuthorizedUpgrade {
					pub code: apply_authorized_upgrade::Code,
				}
				pub mod apply_authorized_upgrade {
					use super::runtime_types;
					pub type Code = ::std::vec::Vec<::core::primitive::u8>;
				}
				impl ::subxt::blocks::StaticExtrinsic for ApplyAuthorizedUpgrade {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "apply_authorized_upgrade";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Make some on-chain remark."]
				#[doc = ""]
				#[doc = "Can be executed by every `origin`."]
				pub fn remark(
					&self,
					remark: types::remark::Remark,
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
				#[doc = "Set the number of pages in the WebAssembly environment's heap."]
				pub fn set_heap_pages(
					&self,
					pages: types::set_heap_pages::Pages,
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
				#[doc = "Set the new runtime code."]
				pub fn set_code(
					&self,
					code: types::set_code::Code,
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
				#[doc = "Set the new runtime code without doing any checks of the given `code`."]
				#[doc = ""]
				#[doc = "Note that runtime upgrades will not run if this is called with a not-increasing spec"]
				#[doc = "version!"]
				pub fn set_code_without_checks(
					&self,
					code: types::set_code_without_checks::Code,
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
				#[doc = "Set some items of storage."]
				pub fn set_storage(
					&self,
					items: types::set_storage::Items,
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
				#[doc = "Kill some items from storage."]
				pub fn kill_storage(
					&self,
					keys: types::kill_storage::Keys,
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
				#[doc = "Kill all storage items with a key that starts with the given prefix."]
				#[doc = ""]
				#[doc = "**NOTE:** We rely on the Root origin to provide us the number of subkeys under"]
				#[doc = "the prefix we are removing to accurately calculate the weight of this function."]
				pub fn kill_prefix(
					&self,
					prefix: types::kill_prefix::Prefix,
					subkeys: types::kill_prefix::Subkeys,
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
				#[doc = "Make some on-chain remark and emit event."]
				pub fn remark_with_event(
					&self,
					remark: types::remark_with_event::Remark,
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
				#[doc = "Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be supplied"]
				#[doc = "later."]
				#[doc = ""]
				#[doc = "This call requires Root origin."]
				pub fn authorize_upgrade(
					&self,
					code_hash: types::authorize_upgrade::CodeHash,
				) -> ::subxt::tx::Payload<types::AuthorizeUpgrade> {
					::subxt::tx::Payload::new_static(
						"System",
						"authorize_upgrade",
						types::AuthorizeUpgrade { code_hash },
						[
							4u8, 14u8, 76u8, 107u8, 209u8, 129u8, 9u8, 39u8, 193u8, 17u8, 84u8,
							254u8, 170u8, 214u8, 24u8, 155u8, 29u8, 184u8, 249u8, 241u8, 109u8,
							58u8, 145u8, 131u8, 109u8, 63u8, 38u8, 165u8, 107u8, 215u8, 217u8,
							172u8,
						],
					)
				}
				#[doc = "Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be supplied"]
				#[doc = "later."]
				#[doc = ""]
				#[doc = "WARNING: This authorizes an upgrade that will take place without any safety checks, for"]
				#[doc = "example that the spec name remains the same and that the version number increases. Not"]
				#[doc = "recommended for normal use. Use `authorize_upgrade` instead."]
				#[doc = ""]
				#[doc = "This call requires Root origin."]
				pub fn authorize_upgrade_without_checks(
					&self,
					code_hash: types::authorize_upgrade_without_checks::CodeHash,
				) -> ::subxt::tx::Payload<types::AuthorizeUpgradeWithoutChecks> {
					::subxt::tx::Payload::new_static(
						"System",
						"authorize_upgrade_without_checks",
						types::AuthorizeUpgradeWithoutChecks { code_hash },
						[
							126u8, 126u8, 55u8, 26u8, 47u8, 55u8, 66u8, 8u8, 167u8, 18u8, 29u8,
							136u8, 146u8, 14u8, 189u8, 117u8, 16u8, 227u8, 162u8, 61u8, 149u8,
							197u8, 104u8, 184u8, 185u8, 161u8, 99u8, 154u8, 80u8, 125u8, 181u8,
							233u8,
						],
					)
				}
				#[doc = "Provide the preimage (runtime binary) `code` for an upgrade that has been authorized."]
				#[doc = ""]
				#[doc = "If the authorization required a version check, this call will ensure the spec name"]
				#[doc = "remains unchanged and that the spec version has increased."]
				#[doc = ""]
				#[doc = "Depending on the runtime's `OnSetCode` configuration, this function may directly apply"]
				#[doc = "the new `code` in the same block or attempt to schedule the upgrade."]
				#[doc = ""]
				#[doc = "All origins are allowed."]
				pub fn apply_authorized_upgrade(
					&self,
					code: types::apply_authorized_upgrade::Code,
				) -> ::subxt::tx::Payload<types::ApplyAuthorizedUpgrade> {
					::subxt::tx::Payload::new_static(
						"System",
						"apply_authorized_upgrade",
						types::ApplyAuthorizedUpgrade { code },
						[
							232u8, 107u8, 127u8, 38u8, 230u8, 29u8, 97u8, 4u8, 160u8, 191u8, 222u8,
							156u8, 245u8, 102u8, 196u8, 141u8, 44u8, 163u8, 98u8, 68u8, 125u8,
							32u8, 124u8, 101u8, 108u8, 93u8, 211u8, 52u8, 0u8, 231u8, 33u8, 227u8,
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
				pub dispatch_info: extrinsic_success::DispatchInfo,
			}
			pub mod extrinsic_success {
				use super::runtime_types;
				pub type DispatchInfo = runtime_types::frame_support::dispatch::DispatchInfo;
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
				pub dispatch_error: extrinsic_failed::DispatchError,
				pub dispatch_info: extrinsic_failed::DispatchInfo,
			}
			pub mod extrinsic_failed {
				use super::runtime_types;
				pub type DispatchError = runtime_types::sp_runtime::DispatchError;
				pub type DispatchInfo = runtime_types::frame_support::dispatch::DispatchInfo;
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
				pub account: new_account::Account,
			}
			pub mod new_account {
				use super::runtime_types;
				pub type Account = ::subxt::utils::AccountId32;
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
				pub account: killed_account::Account,
			}
			pub mod killed_account {
				use super::runtime_types;
				pub type Account = ::subxt::utils::AccountId32;
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
				pub sender: remarked::Sender,
				pub hash: remarked::Hash,
			}
			pub mod remarked {
				use super::runtime_types;
				pub type Sender = ::subxt::utils::AccountId32;
				pub type Hash = ::sp_core::H256;
			}
			impl ::subxt::events::StaticEvent for Remarked {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "Remarked";
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
			#[doc = "An upgrade was authorized."]
			pub struct UpgradeAuthorized {
				pub code_hash: upgrade_authorized::CodeHash,
				pub check_version: upgrade_authorized::CheckVersion,
			}
			pub mod upgrade_authorized {
				use super::runtime_types;
				pub type CodeHash = ::sp_core::H256;
				pub type CheckVersion = ::core::primitive::bool;
			}
			impl ::subxt::events::StaticEvent for UpgradeAuthorized {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "UpgradeAuthorized";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod account {
					use super::runtime_types;
					pub type Account = runtime_types::frame_system::AccountInfo<
						::core::primitive::u32,
						runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>,
					>;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod extrinsic_count {
					use super::runtime_types;
					pub type ExtrinsicCount = ::core::primitive::u32;
				}
				pub mod inherents_applied {
					use super::runtime_types;
					pub type InherentsApplied = ::core::primitive::bool;
				}
				pub mod block_weight {
					use super::runtime_types;
					pub type BlockWeight = runtime_types::frame_support::dispatch::PerDispatchClass<
						runtime_types::sp_weights::weight_v2::Weight,
					>;
				}
				pub mod all_extrinsics_len {
					use super::runtime_types;
					pub type AllExtrinsicsLen = ::core::primitive::u32;
				}
				pub mod block_hash {
					use super::runtime_types;
					pub type BlockHash = ::sp_core::H256;
					pub type Param0 = ::core::primitive::u32;
				}
				pub mod extrinsic_data {
					use super::runtime_types;
					pub type ExtrinsicData = ::std::vec::Vec<::core::primitive::u8>;
					pub type Param0 = ::core::primitive::u32;
				}
				pub mod number {
					use super::runtime_types;
					pub type Number = ::core::primitive::u32;
				}
				pub mod parent_hash {
					use super::runtime_types;
					pub type ParentHash = ::sp_core::H256;
				}
				pub mod digest {
					use super::runtime_types;
					pub type Digest = runtime_types::sp_runtime::generic::digest::Digest;
				}
				pub mod events {
					use super::runtime_types;
					pub type Events = ::std::vec::Vec<
						runtime_types::frame_system::EventRecord<
							runtime_types::ulx_node_runtime::RuntimeEvent,
							::sp_core::H256,
						>,
					>;
				}
				pub mod event_count {
					use super::runtime_types;
					pub type EventCount = ::core::primitive::u32;
				}
				pub mod event_topics {
					use super::runtime_types;
					pub type EventTopics =
						::std::vec::Vec<(::core::primitive::u32, ::core::primitive::u32)>;
					pub type Param0 = ::sp_core::H256;
				}
				pub mod last_runtime_upgrade {
					use super::runtime_types;
					pub type LastRuntimeUpgrade =
						runtime_types::frame_system::LastRuntimeUpgradeInfo;
				}
				pub mod upgraded_to_u32_ref_count {
					use super::runtime_types;
					pub type UpgradedToU32RefCount = ::core::primitive::bool;
				}
				pub mod upgraded_to_triple_ref_count {
					use super::runtime_types;
					pub type UpgradedToTripleRefCount = ::core::primitive::bool;
				}
				pub mod execution_phase {
					use super::runtime_types;
					pub type ExecutionPhase = runtime_types::frame_system::Phase;
				}
				pub mod authorized_upgrade {
					use super::runtime_types;
					pub type AuthorizedUpgrade =
						runtime_types::frame_system::CodeUpgradeAuthorization;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The full account information for a particular account ID."]
				pub fn account_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::account::Account,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"Account",
						(),
						[
							14u8, 233u8, 115u8, 214u8, 0u8, 109u8, 222u8, 121u8, 162u8, 65u8, 60u8,
							175u8, 209u8, 79u8, 222u8, 124u8, 22u8, 235u8, 138u8, 176u8, 133u8,
							124u8, 90u8, 158u8, 85u8, 45u8, 37u8, 174u8, 47u8, 79u8, 47u8, 166u8,
						],
					)
				}
				#[doc = " The full account information for a particular account ID."]
				pub fn account(
					&self,
					_0: impl ::std::borrow::Borrow<types::account::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::account::Param0>,
					types::account::Account,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"Account",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
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
					(),
					types::extrinsic_count::ExtrinsicCount,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"ExtrinsicCount",
						(),
						[
							102u8, 76u8, 236u8, 42u8, 40u8, 231u8, 33u8, 222u8, 123u8, 147u8,
							153u8, 148u8, 234u8, 203u8, 181u8, 119u8, 6u8, 187u8, 177u8, 199u8,
							120u8, 47u8, 137u8, 254u8, 96u8, 100u8, 165u8, 182u8, 249u8, 230u8,
							159u8, 79u8,
						],
					)
				}
				#[doc = " Whether all inherents have been applied."]
				pub fn inherents_applied(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::inherents_applied::InherentsApplied,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"InherentsApplied",
						(),
						[
							132u8, 249u8, 142u8, 252u8, 8u8, 103u8, 80u8, 120u8, 50u8, 6u8, 188u8,
							223u8, 101u8, 55u8, 165u8, 189u8, 172u8, 249u8, 165u8, 230u8, 183u8,
							109u8, 34u8, 65u8, 185u8, 150u8, 29u8, 8u8, 186u8, 129u8, 135u8, 239u8,
						],
					)
				}
				#[doc = " The current weight for the block."]
				pub fn block_weight(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::block_weight::BlockWeight,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"BlockWeight",
						(),
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
					(),
					types::all_extrinsics_len::AllExtrinsicsLen,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"AllExtrinsicsLen",
						(),
						[
							117u8, 86u8, 61u8, 243u8, 41u8, 51u8, 102u8, 214u8, 137u8, 100u8,
							243u8, 185u8, 122u8, 174u8, 187u8, 117u8, 86u8, 189u8, 63u8, 135u8,
							101u8, 218u8, 203u8, 201u8, 237u8, 254u8, 128u8, 183u8, 169u8, 221u8,
							242u8, 65u8,
						],
					)
				}
				#[doc = " Map of block numbers to block hashes."]
				pub fn block_hash_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::block_hash::BlockHash,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"BlockHash",
						(),
						[
							217u8, 32u8, 215u8, 253u8, 24u8, 182u8, 207u8, 178u8, 157u8, 24u8,
							103u8, 100u8, 195u8, 165u8, 69u8, 152u8, 112u8, 181u8, 56u8, 192u8,
							164u8, 16u8, 20u8, 222u8, 28u8, 214u8, 144u8, 142u8, 146u8, 69u8,
							202u8, 118u8,
						],
					)
				}
				#[doc = " Map of block numbers to block hashes."]
				pub fn block_hash(
					&self,
					_0: impl ::std::borrow::Borrow<types::block_hash::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::block_hash::Param0>,
					types::block_hash::BlockHash,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"BlockHash",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							217u8, 32u8, 215u8, 253u8, 24u8, 182u8, 207u8, 178u8, 157u8, 24u8,
							103u8, 100u8, 195u8, 165u8, 69u8, 152u8, 112u8, 181u8, 56u8, 192u8,
							164u8, 16u8, 20u8, 222u8, 28u8, 214u8, 144u8, 142u8, 146u8, 69u8,
							202u8, 118u8,
						],
					)
				}
				#[doc = " Extrinsics data for the current block (maps an extrinsic's index to its data)."]
				pub fn extrinsic_data_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::extrinsic_data::ExtrinsicData,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"ExtrinsicData",
						(),
						[
							160u8, 180u8, 122u8, 18u8, 196u8, 26u8, 2u8, 37u8, 115u8, 232u8, 133u8,
							220u8, 106u8, 245u8, 4u8, 129u8, 42u8, 84u8, 241u8, 45u8, 199u8, 179u8,
							128u8, 61u8, 170u8, 137u8, 231u8, 156u8, 247u8, 57u8, 47u8, 38u8,
						],
					)
				}
				#[doc = " Extrinsics data for the current block (maps an extrinsic's index to its data)."]
				pub fn extrinsic_data(
					&self,
					_0: impl ::std::borrow::Borrow<types::extrinsic_data::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::extrinsic_data::Param0>,
					types::extrinsic_data::ExtrinsicData,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"ExtrinsicData",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
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
					(),
					types::number::Number,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"Number",
						(),
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
					(),
					types::parent_hash::ParentHash,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"ParentHash",
						(),
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
					(),
					types::digest::Digest,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"Digest",
						(),
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
					(),
					types::events::Events,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"Events",
						(),
						[
							135u8, 123u8, 56u8, 141u8, 189u8, 44u8, 135u8, 192u8, 236u8, 87u8,
							255u8, 188u8, 232u8, 178u8, 199u8, 187u8, 19u8, 173u8, 203u8, 157u8,
							32u8, 19u8, 244u8, 251u8, 15u8, 242u8, 176u8, 48u8, 190u8, 151u8,
							223u8, 232u8,
						],
					)
				}
				#[doc = " The number of events in the `Events<T>` list."]
				pub fn event_count(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::event_count::EventCount,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"EventCount",
						(),
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
				pub fn event_topics_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::event_topics::EventTopics,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"EventTopics",
						(),
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
				pub fn event_topics(
					&self,
					_0: impl ::std::borrow::Borrow<types::event_topics::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::event_topics::Param0>,
					types::event_topics::EventTopics,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"EventTopics",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
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
					(),
					types::last_runtime_upgrade::LastRuntimeUpgrade,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"LastRuntimeUpgrade",
						(),
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
					(),
					types::upgraded_to_u32_ref_count::UpgradedToU32RefCount,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"UpgradedToU32RefCount",
						(),
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
					(),
					types::upgraded_to_triple_ref_count::UpgradedToTripleRefCount,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"UpgradedToTripleRefCount",
						(),
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
					(),
					types::execution_phase::ExecutionPhase,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"ExecutionPhase",
						(),
						[
							191u8, 129u8, 100u8, 134u8, 126u8, 116u8, 154u8, 203u8, 220u8, 200u8,
							0u8, 26u8, 161u8, 250u8, 133u8, 205u8, 146u8, 24u8, 5u8, 156u8, 158u8,
							35u8, 36u8, 253u8, 52u8, 235u8, 86u8, 167u8, 35u8, 100u8, 119u8, 27u8,
						],
					)
				}
				#[doc = " `Some` if a code upgrade has been authorized."]
				pub fn authorized_upgrade(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::authorized_upgrade::AuthorizedUpgrade,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"AuthorizedUpgrade",
						(),
						[
							165u8, 97u8, 27u8, 138u8, 2u8, 28u8, 55u8, 92u8, 96u8, 96u8, 168u8,
							169u8, 55u8, 178u8, 44u8, 127u8, 58u8, 140u8, 206u8, 178u8, 1u8, 37u8,
							214u8, 213u8, 251u8, 123u8, 5u8, 111u8, 90u8, 148u8, 217u8, 135u8,
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
				) -> ::subxt::constants::Address<runtime_types::frame_system::limits::BlockLength>
				{
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
				#[doc = " Get the chain's in-code version."]
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
				#[doc = "Set the current time."]
				#[doc = ""]
				#[doc = "This call should be invoked exactly once per block. It will panic at the finalization"]
				#[doc = "phase, if this call hasn't been invoked by that time."]
				#[doc = ""]
				#[doc = "The timestamp should be greater than the previous one by the amount specified by"]
				#[doc = "[`Config::MinimumPeriod`]."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _None_."]
				#[doc = ""]
				#[doc = "This dispatch class is _Mandatory_ to ensure it gets executed in the block. Be aware"]
				#[doc = "that changing the complexity of this call could result exhausting the resources in a"]
				#[doc = "block to execute any other calls."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(1)` (Note that implementations of `OnTimestampSet` must also be `O(1)`)"]
				#[doc = "- 1 storage read and 1 storage mutation (codec `O(1)` because of `DidUpdate::take` in"]
				#[doc = "  `on_finalize`)"]
				#[doc = "- 1 event handler `on_timestamp_set`. Must be `O(1)`."]
				pub struct Set {
					#[codec(compact)]
					pub now: set::Now,
				}
				pub mod set {
					use super::runtime_types;
					pub type Now = ::core::primitive::u64;
				}
				impl ::subxt::blocks::StaticExtrinsic for Set {
					const PALLET: &'static str = "Timestamp";
					const CALL: &'static str = "set";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Set the current time."]
				#[doc = ""]
				#[doc = "This call should be invoked exactly once per block. It will panic at the finalization"]
				#[doc = "phase, if this call hasn't been invoked by that time."]
				#[doc = ""]
				#[doc = "The timestamp should be greater than the previous one by the amount specified by"]
				#[doc = "[`Config::MinimumPeriod`]."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _None_."]
				#[doc = ""]
				#[doc = "This dispatch class is _Mandatory_ to ensure it gets executed in the block. Be aware"]
				#[doc = "that changing the complexity of this call could result exhausting the resources in a"]
				#[doc = "block to execute any other calls."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(1)` (Note that implementations of `OnTimestampSet` must also be `O(1)`)"]
				#[doc = "- 1 storage read and 1 storage mutation (codec `O(1)` because of `DidUpdate::take` in"]
				#[doc = "  `on_finalize`)"]
				#[doc = "- 1 event handler `on_timestamp_set`. Must be `O(1)`."]
				pub fn set(&self, now: types::set::Now) -> ::subxt::tx::Payload<types::Set> {
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
			pub mod types {
				use super::runtime_types;
				pub mod now {
					use super::runtime_types;
					pub type Now = ::core::primitive::u64;
				}
				pub mod did_update {
					use super::runtime_types;
					pub type DidUpdate = ::core::primitive::bool;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The current time for the current block."]
				pub fn now(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::now::Now,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Timestamp",
						"Now",
						(),
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
					(),
					types::did_update::DidUpdate,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Timestamp",
						"DidUpdate",
						(),
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
	pub mod multisig {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_multisig::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_multisig::pallet::Call;
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
				#[doc = "Immediately dispatch a multi-signature call using a single approval from the caller."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "- `other_signatories`: The accounts (other than the sender) who are part of the"]
				#[doc = "multi-signature, but do not participate in the approval process."]
				#[doc = "- `call`: The call to be executed."]
				#[doc = ""]
				#[doc = "Result is equivalent to the dispatched result."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "O(Z + C) where Z is the length of the call and C its execution weight."]
				pub struct AsMultiThreshold1 {
					pub other_signatories: as_multi_threshold1::OtherSignatories,
					pub call: ::std::boxed::Box<as_multi_threshold1::Call>,
				}
				pub mod as_multi_threshold1 {
					use super::runtime_types;
					pub type OtherSignatories = ::std::vec::Vec<::subxt::utils::AccountId32>;
					pub type Call = runtime_types::ulx_node_runtime::RuntimeCall;
				}
				impl ::subxt::blocks::StaticExtrinsic for AsMultiThreshold1 {
					const PALLET: &'static str = "Multisig";
					const CALL: &'static str = "as_multi_threshold_1";
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
				#[doc = "Register approval for a dispatch to be made from a deterministic composite account if"]
				#[doc = "approved by a total of `threshold - 1` of `other_signatories`."]
				#[doc = ""]
				#[doc = "If there are enough, then dispatch the call."]
				#[doc = ""]
				#[doc = "Payment: `DepositBase` will be reserved if this is the first approval, plus"]
				#[doc = "`threshold` times `DepositFactor`. It is returned once this dispatch happens or"]
				#[doc = "is cancelled."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
				#[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
				#[doc = "dispatch. May not be empty."]
				#[doc = "- `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is"]
				#[doc = "not the first approval, then it must be `Some`, with the timepoint (block number and"]
				#[doc = "transaction index) of the first approval transaction."]
				#[doc = "- `call`: The call to be executed."]
				#[doc = ""]
				#[doc = "NOTE: Unless this is the final approval, you will generally want to use"]
				#[doc = "`approve_as_multi` instead, since it only requires a hash of the call."]
				#[doc = ""]
				#[doc = "Result is equivalent to the dispatched result if `threshold` is exactly `1`. Otherwise"]
				#[doc = "on success, result is `Ok` and the result from the interior call, if it was executed,"]
				#[doc = "may be found in the deposited `MultisigExecuted` event."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(S + Z + Call)`."]
				#[doc = "- Up to one balance-reserve or unreserve operation."]
				#[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
				#[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
				#[doc = "- One call encode & hash, both of complexity `O(Z)` where `Z` is tx-len."]
				#[doc = "- One encode & hash, both of complexity `O(S)`."]
				#[doc = "- Up to one binary search and insert (`O(logS + S)`)."]
				#[doc = "- I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove."]
				#[doc = "- One event."]
				#[doc = "- The weight of the `call`."]
				#[doc = "- Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit"]
				#[doc = "  taken for its lifetime of `DepositBase + threshold * DepositFactor`."]
				pub struct AsMulti {
					pub threshold: as_multi::Threshold,
					pub other_signatories: as_multi::OtherSignatories,
					pub maybe_timepoint: as_multi::MaybeTimepoint,
					pub call: ::std::boxed::Box<as_multi::Call>,
					pub max_weight: as_multi::MaxWeight,
				}
				pub mod as_multi {
					use super::runtime_types;
					pub type Threshold = ::core::primitive::u16;
					pub type OtherSignatories = ::std::vec::Vec<::subxt::utils::AccountId32>;
					pub type MaybeTimepoint = ::core::option::Option<
						runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
					>;
					pub type Call = runtime_types::ulx_node_runtime::RuntimeCall;
					pub type MaxWeight = runtime_types::sp_weights::weight_v2::Weight;
				}
				impl ::subxt::blocks::StaticExtrinsic for AsMulti {
					const PALLET: &'static str = "Multisig";
					const CALL: &'static str = "as_multi";
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
				#[doc = "Register approval for a dispatch to be made from a deterministic composite account if"]
				#[doc = "approved by a total of `threshold - 1` of `other_signatories`."]
				#[doc = ""]
				#[doc = "Payment: `DepositBase` will be reserved if this is the first approval, plus"]
				#[doc = "`threshold` times `DepositFactor`. It is returned once this dispatch happens or"]
				#[doc = "is cancelled."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
				#[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
				#[doc = "dispatch. May not be empty."]
				#[doc = "- `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is"]
				#[doc = "not the first approval, then it must be `Some`, with the timepoint (block number and"]
				#[doc = "transaction index) of the first approval transaction."]
				#[doc = "- `call_hash`: The hash of the call to be executed."]
				#[doc = ""]
				#[doc = "NOTE: If this is the final approval, you will want to use `as_multi` instead."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(S)`."]
				#[doc = "- Up to one balance-reserve or unreserve operation."]
				#[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
				#[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
				#[doc = "- One encode & hash, both of complexity `O(S)`."]
				#[doc = "- Up to one binary search and insert (`O(logS + S)`)."]
				#[doc = "- I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove."]
				#[doc = "- One event."]
				#[doc = "- Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit"]
				#[doc = "  taken for its lifetime of `DepositBase + threshold * DepositFactor`."]
				pub struct ApproveAsMulti {
					pub threshold: approve_as_multi::Threshold,
					pub other_signatories: approve_as_multi::OtherSignatories,
					pub maybe_timepoint: approve_as_multi::MaybeTimepoint,
					pub call_hash: approve_as_multi::CallHash,
					pub max_weight: approve_as_multi::MaxWeight,
				}
				pub mod approve_as_multi {
					use super::runtime_types;
					pub type Threshold = ::core::primitive::u16;
					pub type OtherSignatories = ::std::vec::Vec<::subxt::utils::AccountId32>;
					pub type MaybeTimepoint = ::core::option::Option<
						runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
					>;
					pub type CallHash = [::core::primitive::u8; 32usize];
					pub type MaxWeight = runtime_types::sp_weights::weight_v2::Weight;
				}
				impl ::subxt::blocks::StaticExtrinsic for ApproveAsMulti {
					const PALLET: &'static str = "Multisig";
					const CALL: &'static str = "approve_as_multi";
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
				#[doc = "Cancel a pre-existing, on-going multisig transaction. Any deposit reserved previously"]
				#[doc = "for this operation will be unreserved on success."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
				#[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
				#[doc = "dispatch. May not be empty."]
				#[doc = "- `timepoint`: The timepoint (block number and transaction index) of the first approval"]
				#[doc = "transaction for this dispatch."]
				#[doc = "- `call_hash`: The hash of the call to be executed."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(S)`."]
				#[doc = "- Up to one balance-reserve or unreserve operation."]
				#[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
				#[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
				#[doc = "- One encode & hash, both of complexity `O(S)`."]
				#[doc = "- One event."]
				#[doc = "- I/O: 1 read `O(S)`, one remove."]
				#[doc = "- Storage: removes one item."]
				pub struct CancelAsMulti {
					pub threshold: cancel_as_multi::Threshold,
					pub other_signatories: cancel_as_multi::OtherSignatories,
					pub timepoint: cancel_as_multi::Timepoint,
					pub call_hash: cancel_as_multi::CallHash,
				}
				pub mod cancel_as_multi {
					use super::runtime_types;
					pub type Threshold = ::core::primitive::u16;
					pub type OtherSignatories = ::std::vec::Vec<::subxt::utils::AccountId32>;
					pub type Timepoint =
						runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>;
					pub type CallHash = [::core::primitive::u8; 32usize];
				}
				impl ::subxt::blocks::StaticExtrinsic for CancelAsMulti {
					const PALLET: &'static str = "Multisig";
					const CALL: &'static str = "cancel_as_multi";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Immediately dispatch a multi-signature call using a single approval from the caller."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "- `other_signatories`: The accounts (other than the sender) who are part of the"]
				#[doc = "multi-signature, but do not participate in the approval process."]
				#[doc = "- `call`: The call to be executed."]
				#[doc = ""]
				#[doc = "Result is equivalent to the dispatched result."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "O(Z + C) where Z is the length of the call and C its execution weight."]
				pub fn as_multi_threshold_1(
					&self,
					other_signatories: types::as_multi_threshold1::OtherSignatories,
					call: types::as_multi_threshold1::Call,
				) -> ::subxt::tx::Payload<types::AsMultiThreshold1> {
					::subxt::tx::Payload::new_static(
						"Multisig",
						"as_multi_threshold_1",
						types::AsMultiThreshold1 {
							other_signatories,
							call: ::std::boxed::Box::new(call),
						},
						[
							104u8, 178u8, 157u8, 234u8, 74u8, 228u8, 122u8, 90u8, 5u8, 211u8,
							221u8, 232u8, 165u8, 220u8, 246u8, 255u8, 116u8, 174u8, 231u8, 92u8,
							27u8, 83u8, 214u8, 3u8, 218u8, 134u8, 133u8, 13u8, 234u8, 230u8, 122u8,
							177u8,
						],
					)
				}
				#[doc = "Register approval for a dispatch to be made from a deterministic composite account if"]
				#[doc = "approved by a total of `threshold - 1` of `other_signatories`."]
				#[doc = ""]
				#[doc = "If there are enough, then dispatch the call."]
				#[doc = ""]
				#[doc = "Payment: `DepositBase` will be reserved if this is the first approval, plus"]
				#[doc = "`threshold` times `DepositFactor`. It is returned once this dispatch happens or"]
				#[doc = "is cancelled."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
				#[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
				#[doc = "dispatch. May not be empty."]
				#[doc = "- `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is"]
				#[doc = "not the first approval, then it must be `Some`, with the timepoint (block number and"]
				#[doc = "transaction index) of the first approval transaction."]
				#[doc = "- `call`: The call to be executed."]
				#[doc = ""]
				#[doc = "NOTE: Unless this is the final approval, you will generally want to use"]
				#[doc = "`approve_as_multi` instead, since it only requires a hash of the call."]
				#[doc = ""]
				#[doc = "Result is equivalent to the dispatched result if `threshold` is exactly `1`. Otherwise"]
				#[doc = "on success, result is `Ok` and the result from the interior call, if it was executed,"]
				#[doc = "may be found in the deposited `MultisigExecuted` event."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(S + Z + Call)`."]
				#[doc = "- Up to one balance-reserve or unreserve operation."]
				#[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
				#[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
				#[doc = "- One call encode & hash, both of complexity `O(Z)` where `Z` is tx-len."]
				#[doc = "- One encode & hash, both of complexity `O(S)`."]
				#[doc = "- Up to one binary search and insert (`O(logS + S)`)."]
				#[doc = "- I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove."]
				#[doc = "- One event."]
				#[doc = "- The weight of the `call`."]
				#[doc = "- Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit"]
				#[doc = "  taken for its lifetime of `DepositBase + threshold * DepositFactor`."]
				pub fn as_multi(
					&self,
					threshold: types::as_multi::Threshold,
					other_signatories: types::as_multi::OtherSignatories,
					maybe_timepoint: types::as_multi::MaybeTimepoint,
					call: types::as_multi::Call,
					max_weight: types::as_multi::MaxWeight,
				) -> ::subxt::tx::Payload<types::AsMulti> {
					::subxt::tx::Payload::new_static(
						"Multisig",
						"as_multi",
						types::AsMulti {
							threshold,
							other_signatories,
							maybe_timepoint,
							call: ::std::boxed::Box::new(call),
							max_weight,
						},
						[
							195u8, 186u8, 95u8, 254u8, 134u8, 178u8, 216u8, 255u8, 179u8, 116u8,
							243u8, 9u8, 46u8, 210u8, 79u8, 150u8, 195u8, 200u8, 0u8, 94u8, 115u8,
							202u8, 159u8, 34u8, 206u8, 79u8, 205u8, 60u8, 154u8, 242u8, 41u8, 28u8,
						],
					)
				}
				#[doc = "Register approval for a dispatch to be made from a deterministic composite account if"]
				#[doc = "approved by a total of `threshold - 1` of `other_signatories`."]
				#[doc = ""]
				#[doc = "Payment: `DepositBase` will be reserved if this is the first approval, plus"]
				#[doc = "`threshold` times `DepositFactor`. It is returned once this dispatch happens or"]
				#[doc = "is cancelled."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
				#[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
				#[doc = "dispatch. May not be empty."]
				#[doc = "- `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is"]
				#[doc = "not the first approval, then it must be `Some`, with the timepoint (block number and"]
				#[doc = "transaction index) of the first approval transaction."]
				#[doc = "- `call_hash`: The hash of the call to be executed."]
				#[doc = ""]
				#[doc = "NOTE: If this is the final approval, you will want to use `as_multi` instead."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(S)`."]
				#[doc = "- Up to one balance-reserve or unreserve operation."]
				#[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
				#[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
				#[doc = "- One encode & hash, both of complexity `O(S)`."]
				#[doc = "- Up to one binary search and insert (`O(logS + S)`)."]
				#[doc = "- I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove."]
				#[doc = "- One event."]
				#[doc = "- Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit"]
				#[doc = "  taken for its lifetime of `DepositBase + threshold * DepositFactor`."]
				pub fn approve_as_multi(
					&self,
					threshold: types::approve_as_multi::Threshold,
					other_signatories: types::approve_as_multi::OtherSignatories,
					maybe_timepoint: types::approve_as_multi::MaybeTimepoint,
					call_hash: types::approve_as_multi::CallHash,
					max_weight: types::approve_as_multi::MaxWeight,
				) -> ::subxt::tx::Payload<types::ApproveAsMulti> {
					::subxt::tx::Payload::new_static(
						"Multisig",
						"approve_as_multi",
						types::ApproveAsMulti {
							threshold,
							other_signatories,
							maybe_timepoint,
							call_hash,
							max_weight,
						},
						[
							248u8, 46u8, 131u8, 35u8, 204u8, 12u8, 218u8, 150u8, 88u8, 131u8, 89u8,
							13u8, 95u8, 122u8, 87u8, 107u8, 136u8, 154u8, 92u8, 199u8, 108u8, 92u8,
							207u8, 171u8, 113u8, 8u8, 47u8, 248u8, 65u8, 26u8, 203u8, 135u8,
						],
					)
				}
				#[doc = "Cancel a pre-existing, on-going multisig transaction. Any deposit reserved previously"]
				#[doc = "for this operation will be unreserved on success."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
				#[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
				#[doc = "dispatch. May not be empty."]
				#[doc = "- `timepoint`: The timepoint (block number and transaction index) of the first approval"]
				#[doc = "transaction for this dispatch."]
				#[doc = "- `call_hash`: The hash of the call to be executed."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(S)`."]
				#[doc = "- Up to one balance-reserve or unreserve operation."]
				#[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
				#[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
				#[doc = "- One encode & hash, both of complexity `O(S)`."]
				#[doc = "- One event."]
				#[doc = "- I/O: 1 read `O(S)`, one remove."]
				#[doc = "- Storage: removes one item."]
				pub fn cancel_as_multi(
					&self,
					threshold: types::cancel_as_multi::Threshold,
					other_signatories: types::cancel_as_multi::OtherSignatories,
					timepoint: types::cancel_as_multi::Timepoint,
					call_hash: types::cancel_as_multi::CallHash,
				) -> ::subxt::tx::Payload<types::CancelAsMulti> {
					::subxt::tx::Payload::new_static(
						"Multisig",
						"cancel_as_multi",
						types::CancelAsMulti { threshold, other_signatories, timepoint, call_hash },
						[
							212u8, 179u8, 123u8, 40u8, 209u8, 228u8, 181u8, 0u8, 109u8, 28u8, 27u8,
							48u8, 15u8, 47u8, 203u8, 54u8, 106u8, 114u8, 28u8, 118u8, 101u8, 201u8,
							95u8, 187u8, 46u8, 182u8, 4u8, 30u8, 227u8, 105u8, 14u8, 81u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_multisig::pallet::Event;
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
			#[doc = "A new multisig operation has begun."]
			pub struct NewMultisig {
				pub approving: new_multisig::Approving,
				pub multisig: new_multisig::Multisig,
				pub call_hash: new_multisig::CallHash,
			}
			pub mod new_multisig {
				use super::runtime_types;
				pub type Approving = ::subxt::utils::AccountId32;
				pub type Multisig = ::subxt::utils::AccountId32;
				pub type CallHash = [::core::primitive::u8; 32usize];
			}
			impl ::subxt::events::StaticEvent for NewMultisig {
				const PALLET: &'static str = "Multisig";
				const EVENT: &'static str = "NewMultisig";
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
			#[doc = "A multisig operation has been approved by someone."]
			pub struct MultisigApproval {
				pub approving: multisig_approval::Approving,
				pub timepoint: multisig_approval::Timepoint,
				pub multisig: multisig_approval::Multisig,
				pub call_hash: multisig_approval::CallHash,
			}
			pub mod multisig_approval {
				use super::runtime_types;
				pub type Approving = ::subxt::utils::AccountId32;
				pub type Timepoint =
					runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>;
				pub type Multisig = ::subxt::utils::AccountId32;
				pub type CallHash = [::core::primitive::u8; 32usize];
			}
			impl ::subxt::events::StaticEvent for MultisigApproval {
				const PALLET: &'static str = "Multisig";
				const EVENT: &'static str = "MultisigApproval";
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
			#[doc = "A multisig operation has been executed."]
			pub struct MultisigExecuted {
				pub approving: multisig_executed::Approving,
				pub timepoint: multisig_executed::Timepoint,
				pub multisig: multisig_executed::Multisig,
				pub call_hash: multisig_executed::CallHash,
				pub result: multisig_executed::Result,
			}
			pub mod multisig_executed {
				use super::runtime_types;
				pub type Approving = ::subxt::utils::AccountId32;
				pub type Timepoint =
					runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>;
				pub type Multisig = ::subxt::utils::AccountId32;
				pub type CallHash = [::core::primitive::u8; 32usize];
				pub type Result =
					::core::result::Result<(), runtime_types::sp_runtime::DispatchError>;
			}
			impl ::subxt::events::StaticEvent for MultisigExecuted {
				const PALLET: &'static str = "Multisig";
				const EVENT: &'static str = "MultisigExecuted";
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
			#[doc = "A multisig operation has been cancelled."]
			pub struct MultisigCancelled {
				pub cancelling: multisig_cancelled::Cancelling,
				pub timepoint: multisig_cancelled::Timepoint,
				pub multisig: multisig_cancelled::Multisig,
				pub call_hash: multisig_cancelled::CallHash,
			}
			pub mod multisig_cancelled {
				use super::runtime_types;
				pub type Cancelling = ::subxt::utils::AccountId32;
				pub type Timepoint =
					runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>;
				pub type Multisig = ::subxt::utils::AccountId32;
				pub type CallHash = [::core::primitive::u8; 32usize];
			}
			impl ::subxt::events::StaticEvent for MultisigCancelled {
				const PALLET: &'static str = "Multisig";
				const EVENT: &'static str = "MultisigCancelled";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod multisigs {
					use super::runtime_types;
					pub type Multisigs = runtime_types::pallet_multisig::Multisig<
						::core::primitive::u32,
						::core::primitive::u128,
						::subxt::utils::AccountId32,
					>;
					pub type Param0 = ::subxt::utils::AccountId32;
					pub type Param1 = [::core::primitive::u8; 32usize];
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The set of open multisig operations."]
				pub fn multisigs_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::multisigs::Multisigs,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Multisig",
						"Multisigs",
						(),
						[
							154u8, 109u8, 45u8, 18u8, 155u8, 151u8, 81u8, 28u8, 86u8, 127u8, 189u8,
							151u8, 49u8, 61u8, 12u8, 149u8, 84u8, 61u8, 110u8, 197u8, 200u8, 140u8,
							37u8, 100u8, 14u8, 162u8, 158u8, 161u8, 48u8, 117u8, 102u8, 61u8,
						],
					)
				}
				#[doc = " The set of open multisig operations."]
				pub fn multisigs_iter1(
					&self,
					_0: impl ::std::borrow::Borrow<types::multisigs::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::multisigs::Param0>,
					types::multisigs::Multisigs,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Multisig",
						"Multisigs",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							154u8, 109u8, 45u8, 18u8, 155u8, 151u8, 81u8, 28u8, 86u8, 127u8, 189u8,
							151u8, 49u8, 61u8, 12u8, 149u8, 84u8, 61u8, 110u8, 197u8, 200u8, 140u8,
							37u8, 100u8, 14u8, 162u8, 158u8, 161u8, 48u8, 117u8, 102u8, 61u8,
						],
					)
				}
				#[doc = " The set of open multisig operations."]
				pub fn multisigs(
					&self,
					_0: impl ::std::borrow::Borrow<types::multisigs::Param0>,
					_1: impl ::std::borrow::Borrow<types::multisigs::Param1>,
				) -> ::subxt::storage::address::Address<
					(
						::subxt::storage::address::StaticStorageKey<types::multisigs::Param0>,
						::subxt::storage::address::StaticStorageKey<types::multisigs::Param1>,
					),
					types::multisigs::Multisigs,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Multisig",
						"Multisigs",
						(
							::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
							::subxt::storage::address::StaticStorageKey::new(_1.borrow()),
						),
						[
							154u8, 109u8, 45u8, 18u8, 155u8, 151u8, 81u8, 28u8, 86u8, 127u8, 189u8,
							151u8, 49u8, 61u8, 12u8, 149u8, 84u8, 61u8, 110u8, 197u8, 200u8, 140u8,
							37u8, 100u8, 14u8, 162u8, 158u8, 161u8, 48u8, 117u8, 102u8, 61u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The base amount of currency needed to reserve for creating a multisig execution or to"]
				#[doc = " store a dispatch call for later."]
				#[doc = ""]
				#[doc = " This is held for an additional storage item whose value size is"]
				#[doc = " `4 + sizeof((BlockNumber, Balance, AccountId))` bytes and whose key size is"]
				#[doc = " `32 + sizeof(AccountId)` bytes."]
				pub fn deposit_base(&self) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Multisig",
						"DepositBase",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The amount of currency needed per unit threshold when creating a multisig execution."]
				#[doc = ""]
				#[doc = " This is held for adding 32 bytes more into a pre-existing storage value."]
				pub fn deposit_factor(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Multisig",
						"DepositFactor",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The maximum amount of signatories allowed in the multisig."]
				pub fn max_signatories(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Multisig",
						"MaxSignatories",
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
	pub mod proxy {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_proxy::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_proxy::pallet::Call;
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
				#[doc = "Dispatch the given `call` from an account that the sender is authorised for through"]
				#[doc = "`add_proxy`."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
				#[doc = "- `force_proxy_type`: Specify the exact proxy type to be used and checked for this call."]
				#[doc = "- `call`: The call to be made by the `real` account."]
				pub struct Proxy {
					pub real: proxy::Real,
					pub force_proxy_type: proxy::ForceProxyType,
					pub call: ::std::boxed::Box<proxy::Call>,
				}
				pub mod proxy {
					use super::runtime_types;
					pub type Real = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type ForceProxyType =
						::core::option::Option<runtime_types::ulx_node_runtime::ProxyType>;
					pub type Call = runtime_types::ulx_node_runtime::RuntimeCall;
				}
				impl ::subxt::blocks::StaticExtrinsic for Proxy {
					const PALLET: &'static str = "Proxy";
					const CALL: &'static str = "proxy";
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
				#[doc = "Register a proxy account for the sender that is able to make calls on its behalf."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `proxy`: The account that the `caller` would like to make a proxy."]
				#[doc = "- `proxy_type`: The permissions allowed for this proxy account."]
				#[doc = "- `delay`: The announcement period required of the initial proxy. Will generally be"]
				#[doc = "zero."]
				pub struct AddProxy {
					pub delegate: add_proxy::Delegate,
					pub proxy_type: add_proxy::ProxyType,
					pub delay: add_proxy::Delay,
				}
				pub mod add_proxy {
					use super::runtime_types;
					pub type Delegate =
						::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type ProxyType = runtime_types::ulx_node_runtime::ProxyType;
					pub type Delay = ::core::primitive::u32;
				}
				impl ::subxt::blocks::StaticExtrinsic for AddProxy {
					const PALLET: &'static str = "Proxy";
					const CALL: &'static str = "add_proxy";
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
				#[doc = "Unregister a proxy account for the sender."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `proxy`: The account that the `caller` would like to remove as a proxy."]
				#[doc = "- `proxy_type`: The permissions currently enabled for the removed proxy account."]
				pub struct RemoveProxy {
					pub delegate: remove_proxy::Delegate,
					pub proxy_type: remove_proxy::ProxyType,
					pub delay: remove_proxy::Delay,
				}
				pub mod remove_proxy {
					use super::runtime_types;
					pub type Delegate =
						::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type ProxyType = runtime_types::ulx_node_runtime::ProxyType;
					pub type Delay = ::core::primitive::u32;
				}
				impl ::subxt::blocks::StaticExtrinsic for RemoveProxy {
					const PALLET: &'static str = "Proxy";
					const CALL: &'static str = "remove_proxy";
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
				#[doc = "Unregister all proxy accounts for the sender."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "WARNING: This may be called on accounts created by `pure`, however if done, then"]
				#[doc = "the unreserved fees will be inaccessible. **All access to this account will be lost.**"]
				pub struct RemoveProxies;
				impl ::subxt::blocks::StaticExtrinsic for RemoveProxies {
					const PALLET: &'static str = "Proxy";
					const CALL: &'static str = "remove_proxies";
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
				#[doc = "Spawn a fresh new account that is guaranteed to be otherwise inaccessible, and"]
				#[doc = "initialize it with a proxy of `proxy_type` for `origin` sender."]
				#[doc = ""]
				#[doc = "Requires a `Signed` origin."]
				#[doc = ""]
				#[doc = "- `proxy_type`: The type of the proxy that the sender will be registered as over the"]
				#[doc = "new account. This will almost always be the most permissive `ProxyType` possible to"]
				#[doc = "allow for maximum flexibility."]
				#[doc = "- `index`: A disambiguation index, in case this is called multiple times in the same"]
				#[doc = "transaction (e.g. with `utility::batch`). Unless you're using `batch` you probably just"]
				#[doc = "want to use `0`."]
				#[doc = "- `delay`: The announcement period required of the initial proxy. Will generally be"]
				#[doc = "zero."]
				#[doc = ""]
				#[doc = "Fails with `Duplicate` if this has already been called in this transaction, from the"]
				#[doc = "same sender, with the same parameters."]
				#[doc = ""]
				#[doc = "Fails if there are insufficient funds to pay for deposit."]
				pub struct CreatePure {
					pub proxy_type: create_pure::ProxyType,
					pub delay: create_pure::Delay,
					pub index: create_pure::Index,
				}
				pub mod create_pure {
					use super::runtime_types;
					pub type ProxyType = runtime_types::ulx_node_runtime::ProxyType;
					pub type Delay = ::core::primitive::u32;
					pub type Index = ::core::primitive::u16;
				}
				impl ::subxt::blocks::StaticExtrinsic for CreatePure {
					const PALLET: &'static str = "Proxy";
					const CALL: &'static str = "create_pure";
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
				#[doc = "Removes a previously spawned pure proxy."]
				#[doc = ""]
				#[doc = "WARNING: **All access to this account will be lost.** Any funds held in it will be"]
				#[doc = "inaccessible."]
				#[doc = ""]
				#[doc = "Requires a `Signed` origin, and the sender account must have been created by a call to"]
				#[doc = "`pure` with corresponding parameters."]
				#[doc = ""]
				#[doc = "- `spawner`: The account that originally called `pure` to create this account."]
				#[doc = "- `index`: The disambiguation index originally passed to `pure`. Probably `0`."]
				#[doc = "- `proxy_type`: The proxy type originally passed to `pure`."]
				#[doc = "- `height`: The height of the chain when the call to `pure` was processed."]
				#[doc = "- `ext_index`: The extrinsic index in which the call to `pure` was processed."]
				#[doc = ""]
				#[doc = "Fails with `NoPermission` in case the caller is not a previously created pure"]
				#[doc = "account whose `pure` call has corresponding parameters."]
				pub struct KillPure {
					pub spawner: kill_pure::Spawner,
					pub proxy_type: kill_pure::ProxyType,
					pub index: kill_pure::Index,
					#[codec(compact)]
					pub height: kill_pure::Height,
					#[codec(compact)]
					pub ext_index: kill_pure::ExtIndex,
				}
				pub mod kill_pure {
					use super::runtime_types;
					pub type Spawner =
						::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type ProxyType = runtime_types::ulx_node_runtime::ProxyType;
					pub type Index = ::core::primitive::u16;
					pub type Height = ::core::primitive::u32;
					pub type ExtIndex = ::core::primitive::u32;
				}
				impl ::subxt::blocks::StaticExtrinsic for KillPure {
					const PALLET: &'static str = "Proxy";
					const CALL: &'static str = "kill_pure";
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
				#[doc = "Publish the hash of a proxy-call that will be made in the future."]
				#[doc = ""]
				#[doc = "This must be called some number of blocks before the corresponding `proxy` is attempted"]
				#[doc = "if the delay associated with the proxy relationship is greater than zero."]
				#[doc = ""]
				#[doc = "No more than `MaxPending` announcements may be made at any one time."]
				#[doc = ""]
				#[doc = "This will take a deposit of `AnnouncementDepositFactor` as well as"]
				#[doc = "`AnnouncementDepositBase` if there are no other pending announcements."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_ and a proxy of `real`."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
				#[doc = "- `call_hash`: The hash of the call to be made by the `real` account."]
				pub struct Announce {
					pub real: announce::Real,
					pub call_hash: announce::CallHash,
				}
				pub mod announce {
					use super::runtime_types;
					pub type Real = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type CallHash = ::sp_core::H256;
				}
				impl ::subxt::blocks::StaticExtrinsic for Announce {
					const PALLET: &'static str = "Proxy";
					const CALL: &'static str = "announce";
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
				#[doc = "Remove a given announcement."]
				#[doc = ""]
				#[doc = "May be called by a proxy account to remove a call they previously announced and return"]
				#[doc = "the deposit."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
				#[doc = "- `call_hash`: The hash of the call to be made by the `real` account."]
				pub struct RemoveAnnouncement {
					pub real: remove_announcement::Real,
					pub call_hash: remove_announcement::CallHash,
				}
				pub mod remove_announcement {
					use super::runtime_types;
					pub type Real = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type CallHash = ::sp_core::H256;
				}
				impl ::subxt::blocks::StaticExtrinsic for RemoveAnnouncement {
					const PALLET: &'static str = "Proxy";
					const CALL: &'static str = "remove_announcement";
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
				#[doc = "Remove the given announcement of a delegate."]
				#[doc = ""]
				#[doc = "May be called by a target (proxied) account to remove a call that one of their delegates"]
				#[doc = "(`delegate`) has announced they want to execute. The deposit is returned."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `delegate`: The account that previously announced the call."]
				#[doc = "- `call_hash`: The hash of the call to be made."]
				pub struct RejectAnnouncement {
					pub delegate: reject_announcement::Delegate,
					pub call_hash: reject_announcement::CallHash,
				}
				pub mod reject_announcement {
					use super::runtime_types;
					pub type Delegate =
						::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type CallHash = ::sp_core::H256;
				}
				impl ::subxt::blocks::StaticExtrinsic for RejectAnnouncement {
					const PALLET: &'static str = "Proxy";
					const CALL: &'static str = "reject_announcement";
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
				#[doc = "Dispatch the given `call` from an account that the sender is authorized for through"]
				#[doc = "`add_proxy`."]
				#[doc = ""]
				#[doc = "Removes any corresponding announcement(s)."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
				#[doc = "- `force_proxy_type`: Specify the exact proxy type to be used and checked for this call."]
				#[doc = "- `call`: The call to be made by the `real` account."]
				pub struct ProxyAnnounced {
					pub delegate: proxy_announced::Delegate,
					pub real: proxy_announced::Real,
					pub force_proxy_type: proxy_announced::ForceProxyType,
					pub call: ::std::boxed::Box<proxy_announced::Call>,
				}
				pub mod proxy_announced {
					use super::runtime_types;
					pub type Delegate =
						::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Real = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type ForceProxyType =
						::core::option::Option<runtime_types::ulx_node_runtime::ProxyType>;
					pub type Call = runtime_types::ulx_node_runtime::RuntimeCall;
				}
				impl ::subxt::blocks::StaticExtrinsic for ProxyAnnounced {
					const PALLET: &'static str = "Proxy";
					const CALL: &'static str = "proxy_announced";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Dispatch the given `call` from an account that the sender is authorised for through"]
				#[doc = "`add_proxy`."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
				#[doc = "- `force_proxy_type`: Specify the exact proxy type to be used and checked for this call."]
				#[doc = "- `call`: The call to be made by the `real` account."]
				pub fn proxy(
					&self,
					real: types::proxy::Real,
					force_proxy_type: types::proxy::ForceProxyType,
					call: types::proxy::Call,
				) -> ::subxt::tx::Payload<types::Proxy> {
					::subxt::tx::Payload::new_static(
						"Proxy",
						"proxy",
						types::Proxy { real, force_proxy_type, call: ::std::boxed::Box::new(call) },
						[
							6u8, 50u8, 70u8, 143u8, 13u8, 199u8, 21u8, 53u8, 232u8, 135u8, 102u8,
							120u8, 36u8, 181u8, 146u8, 125u8, 33u8, 215u8, 219u8, 26u8, 199u8,
							244u8, 57u8, 222u8, 83u8, 89u8, 136u8, 204u8, 246u8, 31u8, 104u8, 30u8,
						],
					)
				}
				#[doc = "Register a proxy account for the sender that is able to make calls on its behalf."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `proxy`: The account that the `caller` would like to make a proxy."]
				#[doc = "- `proxy_type`: The permissions allowed for this proxy account."]
				#[doc = "- `delay`: The announcement period required of the initial proxy. Will generally be"]
				#[doc = "zero."]
				pub fn add_proxy(
					&self,
					delegate: types::add_proxy::Delegate,
					proxy_type: types::add_proxy::ProxyType,
					delay: types::add_proxy::Delay,
				) -> ::subxt::tx::Payload<types::AddProxy> {
					::subxt::tx::Payload::new_static(
						"Proxy",
						"add_proxy",
						types::AddProxy { delegate, proxy_type, delay },
						[
							30u8, 5u8, 200u8, 92u8, 168u8, 67u8, 155u8, 131u8, 40u8, 91u8, 231u8,
							100u8, 94u8, 217u8, 231u8, 253u8, 244u8, 28u8, 110u8, 177u8, 145u8,
							70u8, 234u8, 94u8, 33u8, 101u8, 251u8, 28u8, 222u8, 24u8, 220u8, 139u8,
						],
					)
				}
				#[doc = "Unregister a proxy account for the sender."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `proxy`: The account that the `caller` would like to remove as a proxy."]
				#[doc = "- `proxy_type`: The permissions currently enabled for the removed proxy account."]
				pub fn remove_proxy(
					&self,
					delegate: types::remove_proxy::Delegate,
					proxy_type: types::remove_proxy::ProxyType,
					delay: types::remove_proxy::Delay,
				) -> ::subxt::tx::Payload<types::RemoveProxy> {
					::subxt::tx::Payload::new_static(
						"Proxy",
						"remove_proxy",
						types::RemoveProxy { delegate, proxy_type, delay },
						[
							108u8, 252u8, 57u8, 123u8, 13u8, 169u8, 85u8, 69u8, 153u8, 216u8, 54u8,
							238u8, 191u8, 104u8, 117u8, 251u8, 233u8, 193u8, 213u8, 190u8, 246u8,
							101u8, 86u8, 17u8, 162u8, 183u8, 209u8, 250u8, 131u8, 42u8, 218u8,
							186u8,
						],
					)
				}
				#[doc = "Unregister all proxy accounts for the sender."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "WARNING: This may be called on accounts created by `pure`, however if done, then"]
				#[doc = "the unreserved fees will be inaccessible. **All access to this account will be lost.**"]
				pub fn remove_proxies(&self) -> ::subxt::tx::Payload<types::RemoveProxies> {
					::subxt::tx::Payload::new_static(
						"Proxy",
						"remove_proxies",
						types::RemoveProxies {},
						[
							1u8, 126u8, 36u8, 227u8, 185u8, 34u8, 218u8, 236u8, 125u8, 231u8, 68u8,
							185u8, 145u8, 63u8, 250u8, 225u8, 103u8, 3u8, 189u8, 37u8, 172u8,
							195u8, 197u8, 216u8, 99u8, 210u8, 240u8, 162u8, 158u8, 132u8, 24u8,
							6u8,
						],
					)
				}
				#[doc = "Spawn a fresh new account that is guaranteed to be otherwise inaccessible, and"]
				#[doc = "initialize it with a proxy of `proxy_type` for `origin` sender."]
				#[doc = ""]
				#[doc = "Requires a `Signed` origin."]
				#[doc = ""]
				#[doc = "- `proxy_type`: The type of the proxy that the sender will be registered as over the"]
				#[doc = "new account. This will almost always be the most permissive `ProxyType` possible to"]
				#[doc = "allow for maximum flexibility."]
				#[doc = "- `index`: A disambiguation index, in case this is called multiple times in the same"]
				#[doc = "transaction (e.g. with `utility::batch`). Unless you're using `batch` you probably just"]
				#[doc = "want to use `0`."]
				#[doc = "- `delay`: The announcement period required of the initial proxy. Will generally be"]
				#[doc = "zero."]
				#[doc = ""]
				#[doc = "Fails with `Duplicate` if this has already been called in this transaction, from the"]
				#[doc = "same sender, with the same parameters."]
				#[doc = ""]
				#[doc = "Fails if there are insufficient funds to pay for deposit."]
				pub fn create_pure(
					&self,
					proxy_type: types::create_pure::ProxyType,
					delay: types::create_pure::Delay,
					index: types::create_pure::Index,
				) -> ::subxt::tx::Payload<types::CreatePure> {
					::subxt::tx::Payload::new_static(
						"Proxy",
						"create_pure",
						types::CreatePure { proxy_type, delay, index },
						[
							185u8, 202u8, 223u8, 190u8, 46u8, 164u8, 170u8, 194u8, 106u8, 39u8,
							83u8, 211u8, 56u8, 152u8, 212u8, 82u8, 126u8, 63u8, 117u8, 94u8, 1u8,
							45u8, 207u8, 69u8, 63u8, 197u8, 122u8, 169u8, 149u8, 26u8, 212u8, 9u8,
						],
					)
				}
				#[doc = "Removes a previously spawned pure proxy."]
				#[doc = ""]
				#[doc = "WARNING: **All access to this account will be lost.** Any funds held in it will be"]
				#[doc = "inaccessible."]
				#[doc = ""]
				#[doc = "Requires a `Signed` origin, and the sender account must have been created by a call to"]
				#[doc = "`pure` with corresponding parameters."]
				#[doc = ""]
				#[doc = "- `spawner`: The account that originally called `pure` to create this account."]
				#[doc = "- `index`: The disambiguation index originally passed to `pure`. Probably `0`."]
				#[doc = "- `proxy_type`: The proxy type originally passed to `pure`."]
				#[doc = "- `height`: The height of the chain when the call to `pure` was processed."]
				#[doc = "- `ext_index`: The extrinsic index in which the call to `pure` was processed."]
				#[doc = ""]
				#[doc = "Fails with `NoPermission` in case the caller is not a previously created pure"]
				#[doc = "account whose `pure` call has corresponding parameters."]
				pub fn kill_pure(
					&self,
					spawner: types::kill_pure::Spawner,
					proxy_type: types::kill_pure::ProxyType,
					index: types::kill_pure::Index,
					height: types::kill_pure::Height,
					ext_index: types::kill_pure::ExtIndex,
				) -> ::subxt::tx::Payload<types::KillPure> {
					::subxt::tx::Payload::new_static(
						"Proxy",
						"kill_pure",
						types::KillPure { spawner, proxy_type, index, height, ext_index },
						[
							149u8, 241u8, 131u8, 170u8, 127u8, 52u8, 48u8, 187u8, 72u8, 221u8,
							196u8, 137u8, 168u8, 43u8, 25u8, 42u8, 225u8, 94u8, 228u8, 168u8, 93u8,
							245u8, 25u8, 33u8, 253u8, 93u8, 186u8, 72u8, 198u8, 108u8, 47u8, 99u8,
						],
					)
				}
				#[doc = "Publish the hash of a proxy-call that will be made in the future."]
				#[doc = ""]
				#[doc = "This must be called some number of blocks before the corresponding `proxy` is attempted"]
				#[doc = "if the delay associated with the proxy relationship is greater than zero."]
				#[doc = ""]
				#[doc = "No more than `MaxPending` announcements may be made at any one time."]
				#[doc = ""]
				#[doc = "This will take a deposit of `AnnouncementDepositFactor` as well as"]
				#[doc = "`AnnouncementDepositBase` if there are no other pending announcements."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_ and a proxy of `real`."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
				#[doc = "- `call_hash`: The hash of the call to be made by the `real` account."]
				pub fn announce(
					&self,
					real: types::announce::Real,
					call_hash: types::announce::CallHash,
				) -> ::subxt::tx::Payload<types::Announce> {
					::subxt::tx::Payload::new_static(
						"Proxy",
						"announce",
						types::Announce { real, call_hash },
						[
							105u8, 218u8, 232u8, 82u8, 80u8, 10u8, 11u8, 1u8, 93u8, 241u8, 121u8,
							198u8, 167u8, 218u8, 95u8, 15u8, 75u8, 122u8, 155u8, 233u8, 10u8,
							175u8, 145u8, 73u8, 214u8, 230u8, 67u8, 107u8, 23u8, 239u8, 69u8,
							240u8,
						],
					)
				}
				#[doc = "Remove a given announcement."]
				#[doc = ""]
				#[doc = "May be called by a proxy account to remove a call they previously announced and return"]
				#[doc = "the deposit."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
				#[doc = "- `call_hash`: The hash of the call to be made by the `real` account."]
				pub fn remove_announcement(
					&self,
					real: types::remove_announcement::Real,
					call_hash: types::remove_announcement::CallHash,
				) -> ::subxt::tx::Payload<types::RemoveAnnouncement> {
					::subxt::tx::Payload::new_static(
						"Proxy",
						"remove_announcement",
						types::RemoveAnnouncement { real, call_hash },
						[
							40u8, 237u8, 179u8, 128u8, 201u8, 183u8, 20u8, 47u8, 99u8, 182u8, 81u8,
							31u8, 27u8, 212u8, 133u8, 36u8, 8u8, 248u8, 57u8, 230u8, 138u8, 80u8,
							241u8, 147u8, 69u8, 236u8, 156u8, 167u8, 205u8, 49u8, 60u8, 16u8,
						],
					)
				}
				#[doc = "Remove the given announcement of a delegate."]
				#[doc = ""]
				#[doc = "May be called by a target (proxied) account to remove a call that one of their delegates"]
				#[doc = "(`delegate`) has announced they want to execute. The deposit is returned."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `delegate`: The account that previously announced the call."]
				#[doc = "- `call_hash`: The hash of the call to be made."]
				pub fn reject_announcement(
					&self,
					delegate: types::reject_announcement::Delegate,
					call_hash: types::reject_announcement::CallHash,
				) -> ::subxt::tx::Payload<types::RejectAnnouncement> {
					::subxt::tx::Payload::new_static(
						"Proxy",
						"reject_announcement",
						types::RejectAnnouncement { delegate, call_hash },
						[
							150u8, 178u8, 49u8, 160u8, 211u8, 75u8, 58u8, 228u8, 121u8, 253u8,
							167u8, 72u8, 68u8, 105u8, 159u8, 52u8, 41u8, 155u8, 92u8, 26u8, 169u8,
							177u8, 102u8, 36u8, 1u8, 47u8, 87u8, 189u8, 223u8, 238u8, 244u8, 110u8,
						],
					)
				}
				#[doc = "Dispatch the given `call` from an account that the sender is authorized for through"]
				#[doc = "`add_proxy`."]
				#[doc = ""]
				#[doc = "Removes any corresponding announcement(s)."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
				#[doc = "- `force_proxy_type`: Specify the exact proxy type to be used and checked for this call."]
				#[doc = "- `call`: The call to be made by the `real` account."]
				pub fn proxy_announced(
					&self,
					delegate: types::proxy_announced::Delegate,
					real: types::proxy_announced::Real,
					force_proxy_type: types::proxy_announced::ForceProxyType,
					call: types::proxy_announced::Call,
				) -> ::subxt::tx::Payload<types::ProxyAnnounced> {
					::subxt::tx::Payload::new_static(
						"Proxy",
						"proxy_announced",
						types::ProxyAnnounced {
							delegate,
							real,
							force_proxy_type,
							call: ::std::boxed::Box::new(call),
						},
						[
							249u8, 198u8, 158u8, 55u8, 114u8, 143u8, 171u8, 218u8, 128u8, 199u8,
							202u8, 22u8, 149u8, 180u8, 76u8, 233u8, 103u8, 1u8, 42u8, 164u8, 206u8,
							41u8, 130u8, 142u8, 107u8, 90u8, 224u8, 172u8, 153u8, 244u8, 215u8,
							188u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_proxy::pallet::Event;
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
			#[doc = "A proxy was executed correctly, with the given."]
			pub struct ProxyExecuted {
				pub result: proxy_executed::Result,
			}
			pub mod proxy_executed {
				use super::runtime_types;
				pub type Result =
					::core::result::Result<(), runtime_types::sp_runtime::DispatchError>;
			}
			impl ::subxt::events::StaticEvent for ProxyExecuted {
				const PALLET: &'static str = "Proxy";
				const EVENT: &'static str = "ProxyExecuted";
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
			#[doc = "A pure account has been created by new proxy with given"]
			#[doc = "disambiguation index and proxy type."]
			pub struct PureCreated {
				pub pure: pure_created::Pure,
				pub who: pure_created::Who,
				pub proxy_type: pure_created::ProxyType,
				pub disambiguation_index: pure_created::DisambiguationIndex,
			}
			pub mod pure_created {
				use super::runtime_types;
				pub type Pure = ::subxt::utils::AccountId32;
				pub type Who = ::subxt::utils::AccountId32;
				pub type ProxyType = runtime_types::ulx_node_runtime::ProxyType;
				pub type DisambiguationIndex = ::core::primitive::u16;
			}
			impl ::subxt::events::StaticEvent for PureCreated {
				const PALLET: &'static str = "Proxy";
				const EVENT: &'static str = "PureCreated";
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
			#[doc = "An announcement was placed to make a call in the future."]
			pub struct Announced {
				pub real: announced::Real,
				pub proxy: announced::Proxy,
				pub call_hash: announced::CallHash,
			}
			pub mod announced {
				use super::runtime_types;
				pub type Real = ::subxt::utils::AccountId32;
				pub type Proxy = ::subxt::utils::AccountId32;
				pub type CallHash = ::sp_core::H256;
			}
			impl ::subxt::events::StaticEvent for Announced {
				const PALLET: &'static str = "Proxy";
				const EVENT: &'static str = "Announced";
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
			#[doc = "A proxy was added."]
			pub struct ProxyAdded {
				pub delegator: proxy_added::Delegator,
				pub delegatee: proxy_added::Delegatee,
				pub proxy_type: proxy_added::ProxyType,
				pub delay: proxy_added::Delay,
			}
			pub mod proxy_added {
				use super::runtime_types;
				pub type Delegator = ::subxt::utils::AccountId32;
				pub type Delegatee = ::subxt::utils::AccountId32;
				pub type ProxyType = runtime_types::ulx_node_runtime::ProxyType;
				pub type Delay = ::core::primitive::u32;
			}
			impl ::subxt::events::StaticEvent for ProxyAdded {
				const PALLET: &'static str = "Proxy";
				const EVENT: &'static str = "ProxyAdded";
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
			#[doc = "A proxy was removed."]
			pub struct ProxyRemoved {
				pub delegator: proxy_removed::Delegator,
				pub delegatee: proxy_removed::Delegatee,
				pub proxy_type: proxy_removed::ProxyType,
				pub delay: proxy_removed::Delay,
			}
			pub mod proxy_removed {
				use super::runtime_types;
				pub type Delegator = ::subxt::utils::AccountId32;
				pub type Delegatee = ::subxt::utils::AccountId32;
				pub type ProxyType = runtime_types::ulx_node_runtime::ProxyType;
				pub type Delay = ::core::primitive::u32;
			}
			impl ::subxt::events::StaticEvent for ProxyRemoved {
				const PALLET: &'static str = "Proxy";
				const EVENT: &'static str = "ProxyRemoved";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod proxies {
					use super::runtime_types;
					pub type Proxies = (
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::pallet_proxy::ProxyDefinition<
								::subxt::utils::AccountId32,
								runtime_types::ulx_node_runtime::ProxyType,
								::core::primitive::u32,
							>,
						>,
						::core::primitive::u128,
					);
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod announcements {
					use super::runtime_types;
					pub type Announcements = (
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::pallet_proxy::Announcement<
								::subxt::utils::AccountId32,
								::sp_core::H256,
								::core::primitive::u32,
							>,
						>,
						::core::primitive::u128,
					);
					pub type Param0 = ::subxt::utils::AccountId32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The set of account proxies. Maps the account which has delegated to the accounts"]
				#[doc = " which are being delegated to, together with the amount held on deposit."]
				pub fn proxies_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::proxies::Proxies,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Proxy",
						"Proxies",
						(),
						[
							251u8, 183u8, 22u8, 123u8, 4u8, 156u8, 182u8, 68u8, 66u8, 31u8, 166u8,
							196u8, 5u8, 225u8, 243u8, 133u8, 91u8, 196u8, 104u8, 27u8, 22u8, 129u8,
							131u8, 129u8, 218u8, 66u8, 195u8, 145u8, 45u8, 37u8, 187u8, 23u8,
						],
					)
				}
				#[doc = " The set of account proxies. Maps the account which has delegated to the accounts"]
				#[doc = " which are being delegated to, together with the amount held on deposit."]
				pub fn proxies(
					&self,
					_0: impl ::std::borrow::Borrow<types::proxies::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::proxies::Param0>,
					types::proxies::Proxies,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Proxy",
						"Proxies",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							251u8, 183u8, 22u8, 123u8, 4u8, 156u8, 182u8, 68u8, 66u8, 31u8, 166u8,
							196u8, 5u8, 225u8, 243u8, 133u8, 91u8, 196u8, 104u8, 27u8, 22u8, 129u8,
							131u8, 129u8, 218u8, 66u8, 195u8, 145u8, 45u8, 37u8, 187u8, 23u8,
						],
					)
				}
				#[doc = " The announcements made by the proxy (key)."]
				pub fn announcements_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::announcements::Announcements,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Proxy",
						"Announcements",
						(),
						[
							129u8, 228u8, 198u8, 210u8, 90u8, 69u8, 151u8, 198u8, 206u8, 174u8,
							148u8, 58u8, 134u8, 14u8, 53u8, 56u8, 234u8, 71u8, 84u8, 247u8, 246u8,
							207u8, 117u8, 221u8, 84u8, 72u8, 254u8, 215u8, 102u8, 49u8, 21u8,
							173u8,
						],
					)
				}
				#[doc = " The announcements made by the proxy (key)."]
				pub fn announcements(
					&self,
					_0: impl ::std::borrow::Borrow<types::announcements::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::announcements::Param0>,
					types::announcements::Announcements,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Proxy",
						"Announcements",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							129u8, 228u8, 198u8, 210u8, 90u8, 69u8, 151u8, 198u8, 206u8, 174u8,
							148u8, 58u8, 134u8, 14u8, 53u8, 56u8, 234u8, 71u8, 84u8, 247u8, 246u8,
							207u8, 117u8, 221u8, 84u8, 72u8, 254u8, 215u8, 102u8, 49u8, 21u8,
							173u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The base amount of currency needed to reserve for creating a proxy."]
				#[doc = ""]
				#[doc = " This is held for an additional storage item whose value size is"]
				#[doc = " `sizeof(Balance)` bytes and whose key size is `sizeof(AccountId)` bytes."]
				pub fn proxy_deposit_base(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Proxy",
						"ProxyDepositBase",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The amount of currency needed per proxy added."]
				#[doc = ""]
				#[doc = " This is held for adding 32 bytes plus an instance of `ProxyType` more into a"]
				#[doc = " pre-existing storage value. Thus, when configuring `ProxyDepositFactor` one should take"]
				#[doc = " into account `32 + proxy_type.encode().len()` bytes of data."]
				pub fn proxy_deposit_factor(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Proxy",
						"ProxyDepositFactor",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The maximum amount of proxies allowed for a single account."]
				pub fn max_proxies(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Proxy",
						"MaxProxies",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum amount of time-delayed announcements that are allowed to be pending."]
				pub fn max_pending(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Proxy",
						"MaxPending",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The base amount of currency needed to reserve for creating an announcement."]
				#[doc = ""]
				#[doc = " This is held when a new storage item holding a `Balance` is created (typically 16"]
				#[doc = " bytes)."]
				pub fn announcement_deposit_base(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Proxy",
						"AnnouncementDepositBase",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The amount of currency needed per announcement made."]
				#[doc = ""]
				#[doc = " This is held for adding an `AccountId`, `Hash` and `BlockNumber` (typically 68 bytes)"]
				#[doc = " into a pre-existing storage value."]
				pub fn announcement_deposit_factor(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Proxy",
						"AnnouncementDepositFactor",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
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
			pub mod types {
				use super::runtime_types;
				pub mod current_tick {
					use super::runtime_types;
					pub type CurrentTick = ::core::primitive::u32;
				}
				pub mod tick_duration {
					use super::runtime_types;
					pub type TickDuration = ::core::primitive::u64;
				}
				pub mod genesis_tick_utc_timestamp {
					use super::runtime_types;
					pub type GenesisTickUtcTimestamp = ::core::primitive::u64;
				}
				pub mod recent_blocks_at_ticks {
					use super::runtime_types;
					pub type RecentBlocksAtTicks =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::sp_core::H256,
						>;
					pub type Param0 = ::core::primitive::u32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				pub fn current_tick(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::current_tick::CurrentTick,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Ticks",
						"CurrentTick",
						(),
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
					(),
					types::tick_duration::TickDuration,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Ticks",
						"TickDuration",
						(),
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
					(),
					types::genesis_tick_utc_timestamp::GenesisTickUtcTimestamp,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Ticks",
						"GenesisTickUtcTimestamp",
						(),
						[
							237u8, 236u8, 104u8, 247u8, 108u8, 221u8, 147u8, 133u8, 46u8, 84u8,
							173u8, 103u8, 141u8, 162u8, 59u8, 108u8, 39u8, 245u8, 68u8, 84u8,
							216u8, 141u8, 150u8, 23u8, 36u8, 174u8, 131u8, 175u8, 249u8, 139u8,
							213u8, 248u8,
						],
					)
				}
				#[doc = " Blocks from the last 100 ticks. Trimmed in on_initialize."]
				#[doc = " NOTE: cannot include the current block hash until next block"]
				pub fn recent_blocks_at_ticks_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::recent_blocks_at_ticks::RecentBlocksAtTicks,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Ticks",
						"RecentBlocksAtTicks",
						(),
						[
							87u8, 146u8, 56u8, 118u8, 8u8, 156u8, 127u8, 97u8, 47u8, 118u8, 176u8,
							69u8, 28u8, 88u8, 208u8, 151u8, 231u8, 136u8, 139u8, 247u8, 240u8,
							171u8, 13u8, 89u8, 145u8, 134u8, 81u8, 194u8, 30u8, 219u8, 126u8, 55u8,
						],
					)
				}
				#[doc = " Blocks from the last 100 ticks. Trimmed in on_initialize."]
				#[doc = " NOTE: cannot include the current block hash until next block"]
				pub fn recent_blocks_at_ticks(
					&self,
					_0: impl ::std::borrow::Borrow<types::recent_blocks_at_ticks::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::recent_blocks_at_ticks::Param0,
					>,
					types::recent_blocks_at_ticks::RecentBlocksAtTicks,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Ticks",
						"RecentBlocksAtTicks",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							87u8, 146u8, 56u8, 118u8, 8u8, 156u8, 127u8, 97u8, 47u8, 118u8, 176u8,
							69u8, 28u8, 88u8, 208u8, 151u8, 231u8, 136u8, 139u8, 247u8, 240u8,
							171u8, 13u8, 89u8, 145u8, 134u8, 81u8, 194u8, 30u8, 219u8, 126u8, 55u8,
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
				#[doc = "Submit a bid for a mining slot in the next cohort. Once all spots are filled in a slot,"]
				#[doc = "a slot can be supplanted by supplying a higher mining bond amount. Bond terms can be"]
				#[doc = "found in the `vaults` pallet. You will supply the bond amount and the vault id to bond"]
				#[doc = "with."]
				#[doc = ""]
				#[doc = "Each slot has `MaxCohortSize` spots available."]
				#[doc = ""]
				#[doc = "To be eligible for a slot, you must have the required ownership tokens in this account."]
				#[doc = "The required amount is calculated as a percentage of the total ownership tokens in the"]
				#[doc = "network. This percentage is adjusted before the beginning of each slot."]
				#[doc = ""]
				#[doc = "If your bid is replaced, a `SlotBidderReplaced` event will be emitted. By monitoring for"]
				#[doc = "this event, you will be able to ensure your bid is accepted."]
				#[doc = ""]
				#[doc = "NOTE: bidding for each slot will be closed at a random block within"]
				#[doc = "`BlocksBeforeBidEndForVrfClose` blocks of the slot end time."]
				#[doc = ""]
				#[doc = "The slot duration can be calculated as `BlocksBetweenSlots * MaxMiners / MaxCohortSize`."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `bond_info`: The bond information to submit for the bid. If `None`, the bid will be"]
				#[doc = " considered a zero-bid."]
				#[doc = "\t- `vault_id`: The vault id to bond with. Terms are taken from the vault at time of bid"]
				#[doc = "   inclusion in the block."]
				#[doc = "  \t- `amount`: The amount to bond with the vault."]
				#[doc = "- `reward_destination`: The account_id for the mining rewards, or `Owner` for the"]
				#[doc = "  submitting user."]
				pub struct Bid {
					pub bond_info: bid::BondInfo,
					pub reward_destination: bid::RewardDestination,
				}
				pub mod bid {
					use super::runtime_types;
					pub type BondInfo = ::core::option::Option<
						runtime_types::pallet_mining_slot::MiningSlotBid<
							::core::primitive::u32,
							::core::primitive::u128,
						>,
					>;
					pub type RewardDestination =
						runtime_types::ulx_primitives::block_seal::RewardDestination<
							::subxt::utils::AccountId32,
						>;
				}
				impl ::subxt::blocks::StaticExtrinsic for Bid {
					const PALLET: &'static str = "MiningSlot";
					const CALL: &'static str = "bid";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Submit a bid for a mining slot in the next cohort. Once all spots are filled in a slot,"]
				#[doc = "a slot can be supplanted by supplying a higher mining bond amount. Bond terms can be"]
				#[doc = "found in the `vaults` pallet. You will supply the bond amount and the vault id to bond"]
				#[doc = "with."]
				#[doc = ""]
				#[doc = "Each slot has `MaxCohortSize` spots available."]
				#[doc = ""]
				#[doc = "To be eligible for a slot, you must have the required ownership tokens in this account."]
				#[doc = "The required amount is calculated as a percentage of the total ownership tokens in the"]
				#[doc = "network. This percentage is adjusted before the beginning of each slot."]
				#[doc = ""]
				#[doc = "If your bid is replaced, a `SlotBidderReplaced` event will be emitted. By monitoring for"]
				#[doc = "this event, you will be able to ensure your bid is accepted."]
				#[doc = ""]
				#[doc = "NOTE: bidding for each slot will be closed at a random block within"]
				#[doc = "`BlocksBeforeBidEndForVrfClose` blocks of the slot end time."]
				#[doc = ""]
				#[doc = "The slot duration can be calculated as `BlocksBetweenSlots * MaxMiners / MaxCohortSize`."]
				#[doc = ""]
				#[doc = "Parameters:"]
				#[doc = "- `bond_info`: The bond information to submit for the bid. If `None`, the bid will be"]
				#[doc = " considered a zero-bid."]
				#[doc = "\t- `vault_id`: The vault id to bond with. Terms are taken from the vault at time of bid"]
				#[doc = "   inclusion in the block."]
				#[doc = "  \t- `amount`: The amount to bond with the vault."]
				#[doc = "- `reward_destination`: The account_id for the mining rewards, or `Owner` for the"]
				#[doc = "  submitting user."]
				pub fn bid(
					&self,
					bond_info: types::bid::BondInfo,
					reward_destination: types::bid::RewardDestination,
				) -> ::subxt::tx::Payload<types::Bid> {
					::subxt::tx::Payload::new_static(
						"MiningSlot",
						"bid",
						types::Bid { bond_info, reward_destination },
						[
							96u8, 245u8, 243u8, 243u8, 160u8, 140u8, 179u8, 184u8, 81u8, 249u8,
							234u8, 42u8, 78u8, 99u8, 33u8, 16u8, 203u8, 122u8, 253u8, 221u8, 81u8,
							26u8, 193u8, 36u8, 12u8, 94u8, 150u8, 159u8, 96u8, 107u8, 135u8, 204u8,
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
				pub start_index: new_miners::StartIndex,
				pub new_miners: new_miners::NewMiners,
			}
			pub mod new_miners {
				use super::runtime_types;
				pub type StartIndex = ::core::primitive::u32;
				pub type NewMiners = runtime_types::bounded_collections::bounded_vec::BoundedVec<
					runtime_types::ulx_primitives::block_seal::MiningRegistration<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
					>,
				>;
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
				pub account_id: slot_bidder_added::AccountId,
				pub bid_amount: slot_bidder_added::BidAmount,
				pub index: slot_bidder_added::Index,
			}
			pub mod slot_bidder_added {
				use super::runtime_types;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type BidAmount = ::core::primitive::u128;
				pub type Index = ::core::primitive::u32;
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
				pub account_id: slot_bidder_replaced::AccountId,
				pub bond_id: slot_bidder_replaced::BondId,
				pub kept_ownership_bond: slot_bidder_replaced::KeptOwnershipBond,
			}
			pub mod slot_bidder_replaced {
				use super::runtime_types;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type BondId = ::core::option::Option<::core::primitive::u64>;
				pub type KeptOwnershipBond = ::core::primitive::bool;
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
				pub account_id: unbonded_miner::AccountId,
				pub bond_id: unbonded_miner::BondId,
				pub kept_ownership_bond: unbonded_miner::KeptOwnershipBond,
			}
			pub mod unbonded_miner {
				use super::runtime_types;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type BondId = ::core::option::Option<::core::primitive::u64>;
				pub type KeptOwnershipBond = ::core::primitive::bool;
			}
			impl ::subxt::events::StaticEvent for UnbondedMiner {
				const PALLET: &'static str = "MiningSlot";
				const EVENT: &'static str = "UnbondedMiner";
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
			pub struct UnbondMinerError {
				pub account_id: unbond_miner_error::AccountId,
				pub bond_id: unbond_miner_error::BondId,
				pub error: unbond_miner_error::Error,
			}
			pub mod unbond_miner_error {
				use super::runtime_types;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type BondId = ::core::option::Option<::core::primitive::u64>;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for UnbondMinerError {
				const PALLET: &'static str = "MiningSlot";
				const EVENT: &'static str = "UnbondMinerError";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod active_miners_by_index {
					use super::runtime_types;
					pub type ActiveMinersByIndex =
						runtime_types::ulx_primitives::block_seal::MiningRegistration<
							::subxt::utils::AccountId32,
							::core::primitive::u128,
						>;
					pub type Param0 = ::core::primitive::u32;
				}
				pub mod active_miners_count {
					use super::runtime_types;
					pub type ActiveMinersCount = ::core::primitive::u16;
				}
				pub mod authorities_by_index {
					use super::runtime_types;
					pub type AuthoritiesByIndex =
						runtime_types::bounded_collections::bounded_btree_map::BoundedBTreeMap<
							::core::primitive::u32,
							(
								runtime_types::ulx_primitives::block_seal::app::Public,
								runtime_types::primitive_types::U256,
							),
						>;
				}
				pub mod ownership_bond_amount {
					use super::runtime_types;
					pub type OwnershipBondAmount = ::core::primitive::u128;
				}
				pub mod last_ownership_percent_adjustment {
					use super::runtime_types;
					pub type LastOwnershipPercentAdjustment =
						runtime_types::sp_arithmetic::fixed_point::FixedU128;
				}
				pub mod account_index_lookup {
					use super::runtime_types;
					pub type AccountIndexLookup = ::core::primitive::u32;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod next_slot_cohort {
					use super::runtime_types;
					pub type NextSlotCohort =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::block_seal::MiningRegistration<
								::subxt::utils::AccountId32,
								::core::primitive::u128,
							>,
						>;
				}
				pub mod is_next_slot_bidding_open {
					use super::runtime_types;
					pub type IsNextSlotBiddingOpen = ::core::primitive::bool;
				}
				pub mod miner_zero {
					use super::runtime_types;
					pub type MinerZero =
						runtime_types::ulx_primitives::block_seal::MiningRegistration<
							::subxt::utils::AccountId32,
							::core::primitive::u128,
						>;
				}
				pub mod historical_bids_per_slot {
					use super::runtime_types;
					pub type HistoricalBidsPerSlot =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u32,
						>;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Miners that are active in the current block (post initialize)"]
				pub fn active_miners_by_index_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::active_miners_by_index::ActiveMinersByIndex,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"ActiveMinersByIndex",
						(),
						[
							229u8, 235u8, 210u8, 136u8, 57u8, 249u8, 160u8, 206u8, 204u8, 199u8,
							217u8, 40u8, 76u8, 152u8, 242u8, 119u8, 175u8, 202u8, 212u8, 6u8,
							143u8, 44u8, 116u8, 84u8, 229u8, 157u8, 75u8, 192u8, 111u8, 69u8,
							165u8, 132u8,
						],
					)
				}
				#[doc = " Miners that are active in the current block (post initialize)"]
				pub fn active_miners_by_index(
					&self,
					_0: impl ::std::borrow::Borrow<types::active_miners_by_index::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::active_miners_by_index::Param0,
					>,
					types::active_miners_by_index::ActiveMinersByIndex,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"ActiveMinersByIndex",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							229u8, 235u8, 210u8, 136u8, 57u8, 249u8, 160u8, 206u8, 204u8, 199u8,
							217u8, 40u8, 76u8, 152u8, 242u8, 119u8, 175u8, 202u8, 212u8, 6u8,
							143u8, 44u8, 116u8, 84u8, 229u8, 157u8, 75u8, 192u8, 111u8, 69u8,
							165u8, 132u8,
						],
					)
				}
				pub fn active_miners_count(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::active_miners_count::ActiveMinersCount,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"ActiveMinersCount",
						(),
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
					(),
					types::authorities_by_index::AuthoritiesByIndex,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"AuthoritiesByIndex",
						(),
						[
							35u8, 35u8, 17u8, 184u8, 250u8, 250u8, 12u8, 2u8, 78u8, 97u8, 114u8,
							1u8, 33u8, 200u8, 71u8, 158u8, 77u8, 136u8, 234u8, 148u8, 193u8, 5u8,
							111u8, 235u8, 252u8, 16u8, 104u8, 203u8, 219u8, 244u8, 1u8, 98u8,
						],
					)
				}
				#[doc = " Tokens that must be bonded to take a Miner role"]
				pub fn ownership_bond_amount(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::ownership_bond_amount::OwnershipBondAmount,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"OwnershipBondAmount",
						(),
						[
							45u8, 244u8, 244u8, 223u8, 190u8, 125u8, 131u8, 55u8, 69u8, 254u8,
							146u8, 237u8, 64u8, 61u8, 35u8, 114u8, 66u8, 95u8, 137u8, 138u8, 50u8,
							128u8, 217u8, 131u8, 10u8, 243u8, 1u8, 238u8, 208u8, 214u8, 106u8,
							235u8,
						],
					)
				}
				#[doc = " The last percentage adjustment to the ownership bond amount"]
				pub fn last_ownership_percent_adjustment(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::last_ownership_percent_adjustment::LastOwnershipPercentAdjustment,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"LastOwnershipPercentAdjustment",
						(),
						[
							22u8, 170u8, 117u8, 90u8, 112u8, 170u8, 183u8, 3u8, 63u8, 34u8, 26u8,
							138u8, 147u8, 135u8, 3u8, 37u8, 74u8, 195u8, 167u8, 141u8, 156u8, 49u8,
							161u8, 236u8, 21u8, 84u8, 10u8, 187u8, 29u8, 162u8, 44u8, 175u8,
						],
					)
				}
				#[doc = " Lookup by account id to the corresponding index in ActiveMinersByIndex and Authorities"]
				pub fn account_index_lookup_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::account_index_lookup::AccountIndexLookup,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"AccountIndexLookup",
						(),
						[
							203u8, 195u8, 115u8, 185u8, 125u8, 36u8, 9u8, 40u8, 68u8, 9u8, 52u8,
							60u8, 181u8, 139u8, 145u8, 41u8, 100u8, 62u8, 237u8, 172u8, 108u8,
							227u8, 106u8, 161u8, 59u8, 110u8, 244u8, 142u8, 80u8, 147u8, 188u8,
							190u8,
						],
					)
				}
				#[doc = " Lookup by account id to the corresponding index in ActiveMinersByIndex and Authorities"]
				pub fn account_index_lookup(
					&self,
					_0: impl ::std::borrow::Borrow<types::account_index_lookup::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::account_index_lookup::Param0,
					>,
					types::account_index_lookup::AccountIndexLookup,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"AccountIndexLookup",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
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
					(),
					types::next_slot_cohort::NextSlotCohort,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"NextSlotCohort",
						(),
						[
							112u8, 33u8, 37u8, 235u8, 0u8, 110u8, 195u8, 34u8, 206u8, 246u8, 2u8,
							113u8, 207u8, 63u8, 55u8, 117u8, 34u8, 26u8, 56u8, 93u8, 207u8, 112u8,
							83u8, 224u8, 245u8, 7u8, 108u8, 190u8, 174u8, 24u8, 159u8, 145u8,
						],
					)
				}
				#[doc = " Is the next slot still open for bids"]
				pub fn is_next_slot_bidding_open(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::is_next_slot_bidding_open::IsNextSlotBiddingOpen,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"IsNextSlotBiddingOpen",
						(),
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
					(),
					types::miner_zero::MinerZero,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"MinerZero",
						(),
						[
							20u8, 123u8, 210u8, 18u8, 232u8, 241u8, 190u8, 77u8, 190u8, 202u8,
							103u8, 129u8, 1u8, 44u8, 116u8, 226u8, 42u8, 227u8, 171u8, 38u8, 133u8,
							107u8, 190u8, 49u8, 175u8, 172u8, 157u8, 154u8, 209u8, 46u8, 47u8,
							83u8,
						],
					)
				}
				#[doc = " The number of bids per slot for the last 10 slots (newest first)"]
				pub fn historical_bids_per_slot(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::historical_bids_per_slot::HistoricalBidsPerSlot,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MiningSlot",
						"HistoricalBidsPerSlot",
						(),
						[
							136u8, 148u8, 91u8, 191u8, 165u8, 59u8, 132u8, 128u8, 139u8, 150u8,
							116u8, 76u8, 241u8, 30u8, 112u8, 65u8, 17u8, 155u8, 66u8, 130u8, 163u8,
							97u8, 253u8, 37u8, 13u8, 91u8, 82u8, 221u8, 123u8, 191u8, 104u8, 7u8,
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
				#[doc = " How many blocks before the end of a slot can the bid close"]
				pub fn blocks_before_bid_end_for_vrf_close(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"MiningSlot",
						"BlocksBeforeBidEndForVrfClose",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The block number when bidding will start (eg, Slot \"1\")"]
				pub fn slot_bidding_start_block(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"MiningSlot",
						"SlotBiddingStartBlock",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The max percent swing for the ownership bond amount per slot (from the last percent"]
				pub fn ownership_percent_adjustment_damper(
					&self,
				) -> ::subxt::constants::Address<runtime_types::sp_arithmetic::fixed_point::FixedU128>
				{
					::subxt::constants::Address::new_static(
						"MiningSlot",
						"OwnershipPercentAdjustmentDamper",
						[
							62u8, 145u8, 102u8, 227u8, 159u8, 92u8, 27u8, 54u8, 159u8, 228u8,
							193u8, 99u8, 75u8, 196u8, 26u8, 250u8, 229u8, 230u8, 88u8, 109u8,
							246u8, 100u8, 152u8, 158u8, 14u8, 25u8, 224u8, 173u8, 224u8, 41u8,
							105u8, 231u8,
						],
					)
				}
				#[doc = " The target number of bids per slot. This will adjust the ownership bond amount up or"]
				#[doc = " down to ensure mining slots are filled."]
				pub fn target_bids_per_slot(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"MiningSlot",
						"TargetBidsPerSlot",
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
	pub mod bitcoin_utxos {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_bitcoin_utxos::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_bitcoin_utxos::pallet::Call;
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
				#[doc = "Submitted when a bitcoin UTXO has been moved or confirmed"]
				pub struct Sync {
					pub utxo_sync: sync::UtxoSync,
				}
				pub mod sync {
					use super::runtime_types;
					pub type UtxoSync = runtime_types::ulx_primitives::inherents::BitcoinUtxoSync;
				}
				impl ::subxt::blocks::StaticExtrinsic for Sync {
					const PALLET: &'static str = "BitcoinUtxos";
					const CALL: &'static str = "sync";
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
				#[doc = "Sets the most recent confirmed bitcoin block height (only executable by the Oracle"]
				#[doc = "Operator account)"]
				#[doc = ""]
				#[doc = "# Arguments"]
				#[doc = "* `bitcoin_height` - the latest bitcoin block height to be confirmed"]
				pub struct SetConfirmedBlock {
					pub bitcoin_height: set_confirmed_block::BitcoinHeight,
					pub bitcoin_block_hash: set_confirmed_block::BitcoinBlockHash,
				}
				pub mod set_confirmed_block {
					use super::runtime_types;
					pub type BitcoinHeight = ::core::primitive::u64;
					pub type BitcoinBlockHash = runtime_types::ulx_primitives::bitcoin::H256Le;
				}
				impl ::subxt::blocks::StaticExtrinsic for SetConfirmedBlock {
					const PALLET: &'static str = "BitcoinUtxos";
					const CALL: &'static str = "set_confirmed_block";
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
				#[doc = "Sets the oracle operator account id (only executable by the Root account)"]
				#[doc = ""]
				#[doc = "# Arguments"]
				#[doc = "* `account_id` - the account id of the operator"]
				pub struct SetOperator {
					pub account_id: set_operator::AccountId,
				}
				pub mod set_operator {
					use super::runtime_types;
					pub type AccountId = ::subxt::utils::AccountId32;
				}
				impl ::subxt::blocks::StaticExtrinsic for SetOperator {
					const PALLET: &'static str = "BitcoinUtxos";
					const CALL: &'static str = "set_operator";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Submitted when a bitcoin UTXO has been moved or confirmed"]
				pub fn sync(
					&self,
					utxo_sync: types::sync::UtxoSync,
				) -> ::subxt::tx::Payload<types::Sync> {
					::subxt::tx::Payload::new_static(
						"BitcoinUtxos",
						"sync",
						types::Sync { utxo_sync },
						[
							234u8, 65u8, 74u8, 196u8, 87u8, 89u8, 1u8, 48u8, 165u8, 144u8, 131u8,
							167u8, 46u8, 250u8, 227u8, 251u8, 8u8, 206u8, 26u8, 89u8, 158u8, 42u8,
							206u8, 16u8, 124u8, 193u8, 225u8, 117u8, 16u8, 143u8, 240u8, 95u8,
						],
					)
				}
				#[doc = "Sets the most recent confirmed bitcoin block height (only executable by the Oracle"]
				#[doc = "Operator account)"]
				#[doc = ""]
				#[doc = "# Arguments"]
				#[doc = "* `bitcoin_height` - the latest bitcoin block height to be confirmed"]
				pub fn set_confirmed_block(
					&self,
					bitcoin_height: types::set_confirmed_block::BitcoinHeight,
					bitcoin_block_hash: types::set_confirmed_block::BitcoinBlockHash,
				) -> ::subxt::tx::Payload<types::SetConfirmedBlock> {
					::subxt::tx::Payload::new_static(
						"BitcoinUtxos",
						"set_confirmed_block",
						types::SetConfirmedBlock { bitcoin_height, bitcoin_block_hash },
						[
							33u8, 160u8, 96u8, 40u8, 80u8, 235u8, 253u8, 54u8, 254u8, 196u8, 157u8,
							145u8, 121u8, 135u8, 90u8, 142u8, 187u8, 18u8, 120u8, 173u8, 13u8,
							40u8, 4u8, 69u8, 191u8, 36u8, 206u8, 106u8, 28u8, 55u8, 31u8, 59u8,
						],
					)
				}
				#[doc = "Sets the oracle operator account id (only executable by the Root account)"]
				#[doc = ""]
				#[doc = "# Arguments"]
				#[doc = "* `account_id` - the account id of the operator"]
				pub fn set_operator(
					&self,
					account_id: types::set_operator::AccountId,
				) -> ::subxt::tx::Payload<types::SetOperator> {
					::subxt::tx::Payload::new_static(
						"BitcoinUtxos",
						"set_operator",
						types::SetOperator { account_id },
						[
							160u8, 195u8, 42u8, 151u8, 18u8, 138u8, 64u8, 248u8, 118u8, 157u8,
							178u8, 120u8, 23u8, 254u8, 8u8, 157u8, 220u8, 244u8, 50u8, 65u8, 219u8,
							177u8, 56u8, 216u8, 58u8, 76u8, 168u8, 143u8, 16u8, 155u8, 250u8, 21u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_bitcoin_utxos::pallet::Event;
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
			pub struct UtxoVerified {
				pub utxo_id: utxo_verified::UtxoId,
			}
			pub mod utxo_verified {
				use super::runtime_types;
				pub type UtxoId = ::core::primitive::u64;
			}
			impl ::subxt::events::StaticEvent for UtxoVerified {
				const PALLET: &'static str = "BitcoinUtxos";
				const EVENT: &'static str = "UtxoVerified";
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
			pub struct UtxoRejected {
				pub utxo_id: utxo_rejected::UtxoId,
				pub rejected_reason: utxo_rejected::RejectedReason,
			}
			pub mod utxo_rejected {
				use super::runtime_types;
				pub type UtxoId = ::core::primitive::u64;
				pub type RejectedReason =
					runtime_types::ulx_primitives::bitcoin::BitcoinRejectedReason;
			}
			impl ::subxt::events::StaticEvent for UtxoRejected {
				const PALLET: &'static str = "BitcoinUtxos";
				const EVENT: &'static str = "UtxoRejected";
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
			pub struct UtxoSpent {
				pub utxo_id: utxo_spent::UtxoId,
				pub block_height: utxo_spent::BlockHeight,
			}
			pub mod utxo_spent {
				use super::runtime_types;
				pub type UtxoId = ::core::primitive::u64;
				pub type BlockHeight = ::core::primitive::u64;
			}
			impl ::subxt::events::StaticEvent for UtxoSpent {
				const PALLET: &'static str = "BitcoinUtxos";
				const EVENT: &'static str = "UtxoSpent";
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
			pub struct UtxoUnwatched {
				pub utxo_id: utxo_unwatched::UtxoId,
			}
			pub mod utxo_unwatched {
				use super::runtime_types;
				pub type UtxoId = ::core::primitive::u64;
			}
			impl ::subxt::events::StaticEvent for UtxoUnwatched {
				const PALLET: &'static str = "BitcoinUtxos";
				const EVENT: &'static str = "UtxoUnwatched";
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
			pub struct UtxoSpentError {
				pub utxo_id: utxo_spent_error::UtxoId,
				pub error: utxo_spent_error::Error,
			}
			pub mod utxo_spent_error {
				use super::runtime_types;
				pub type UtxoId = ::core::primitive::u64;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for UtxoSpentError {
				const PALLET: &'static str = "BitcoinUtxos";
				const EVENT: &'static str = "UtxoSpentError";
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
			pub struct UtxoVerifiedError {
				pub utxo_id: utxo_verified_error::UtxoId,
				pub error: utxo_verified_error::Error,
			}
			pub mod utxo_verified_error {
				use super::runtime_types;
				pub type UtxoId = ::core::primitive::u64;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for UtxoVerifiedError {
				const PALLET: &'static str = "BitcoinUtxos";
				const EVENT: &'static str = "UtxoVerifiedError";
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
			pub struct UtxoRejectedError {
				pub utxo_id: utxo_rejected_error::UtxoId,
				pub error: utxo_rejected_error::Error,
			}
			pub mod utxo_rejected_error {
				use super::runtime_types;
				pub type UtxoId = ::core::primitive::u64;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for UtxoRejectedError {
				const PALLET: &'static str = "BitcoinUtxos";
				const EVENT: &'static str = "UtxoRejectedError";
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
			pub struct UtxoExpiredError {
				pub utxo_ref: utxo_expired_error::UtxoRef,
				pub error: utxo_expired_error::Error,
			}
			pub mod utxo_expired_error {
				use super::runtime_types;
				pub type UtxoRef = runtime_types::ulx_primitives::bitcoin::UtxoRef;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for UtxoExpiredError {
				const PALLET: &'static str = "BitcoinUtxos";
				const EVENT: &'static str = "UtxoExpiredError";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod next_utxo_id {
					use super::runtime_types;
					pub type NextUtxoId = ::core::primitive::u64;
				}
				pub mod locked_utxos {
					use super::runtime_types;
					pub type LockedUtxos = runtime_types::ulx_primitives::bitcoin::UtxoValue;
					pub type Param0 = runtime_types::ulx_primitives::bitcoin::UtxoRef;
				}
				pub mod utxo_id_to_ref {
					use super::runtime_types;
					pub type UtxoIdToRef = runtime_types::ulx_primitives::bitcoin::UtxoRef;
					pub type Param0 = ::core::primitive::u64;
				}
				pub mod utxos_pending_confirmation {
					use super::runtime_types;
					pub type UtxosPendingConfirmation =
						runtime_types::bounded_collections::bounded_btree_map::BoundedBTreeMap<
							::core::primitive::u64,
							runtime_types::ulx_primitives::bitcoin::UtxoValue,
						>;
				}
				pub mod confirmed_bitcoin_block_tip {
					use super::runtime_types;
					pub type ConfirmedBitcoinBlockTip =
						runtime_types::ulx_primitives::bitcoin::BitcoinBlock;
				}
				pub mod synched_bitcoin_block {
					use super::runtime_types;
					pub type SynchedBitcoinBlock =
						runtime_types::ulx_primitives::bitcoin::BitcoinBlock;
				}
				pub mod oracle_operator_account {
					use super::runtime_types;
					pub type OracleOperatorAccount = ::subxt::utils::AccountId32;
				}
				pub mod locked_utxo_expirations_by_block {
					use super::runtime_types;
					pub type LockedUtxoExpirationsByBlock =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::bitcoin::UtxoRef,
						>;
					pub type Param0 = ::core::primitive::u64;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				pub fn next_utxo_id(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::next_utxo_id::NextUtxoId,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BitcoinUtxos",
						"NextUtxoId",
						(),
						[
							152u8, 87u8, 202u8, 169u8, 252u8, 39u8, 159u8, 178u8, 28u8, 105u8,
							169u8, 154u8, 151u8, 212u8, 63u8, 64u8, 68u8, 159u8, 195u8, 109u8, 9u8,
							203u8, 203u8, 236u8, 250u8, 168u8, 142u8, 62u8, 69u8, 158u8, 222u8,
							25u8,
						],
					)
				}
				#[doc = " Locked Bitcoin UTXOs that have had ownership confirmed. If a Bitcoin UTXO is moved before"]
				#[doc = " the expiration block, the bond is burned and the UTXO is unlocked."]
				pub fn locked_utxos_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::locked_utxos::LockedUtxos,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"BitcoinUtxos",
						"LockedUtxos",
						(),
						[
							162u8, 29u8, 77u8, 39u8, 142u8, 235u8, 61u8, 200u8, 6u8, 16u8, 154u8,
							106u8, 0u8, 217u8, 249u8, 95u8, 129u8, 115u8, 234u8, 45u8, 95u8, 32u8,
							23u8, 217u8, 59u8, 253u8, 156u8, 10u8, 175u8, 165u8, 99u8, 235u8,
						],
					)
				}
				#[doc = " Locked Bitcoin UTXOs that have had ownership confirmed. If a Bitcoin UTXO is moved before"]
				#[doc = " the expiration block, the bond is burned and the UTXO is unlocked."]
				pub fn locked_utxos(
					&self,
					_0: impl ::std::borrow::Borrow<types::locked_utxos::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::locked_utxos::Param0>,
					types::locked_utxos::LockedUtxos,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BitcoinUtxos",
						"LockedUtxos",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							162u8, 29u8, 77u8, 39u8, 142u8, 235u8, 61u8, 200u8, 6u8, 16u8, 154u8,
							106u8, 0u8, 217u8, 249u8, 95u8, 129u8, 115u8, 234u8, 45u8, 95u8, 32u8,
							23u8, 217u8, 59u8, 253u8, 156u8, 10u8, 175u8, 165u8, 99u8, 235u8,
						],
					)
				}
				pub fn utxo_id_to_ref_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::utxo_id_to_ref::UtxoIdToRef,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"BitcoinUtxos",
						"UtxoIdToRef",
						(),
						[
							217u8, 13u8, 120u8, 37u8, 57u8, 155u8, 243u8, 201u8, 228u8, 157u8,
							235u8, 124u8, 28u8, 83u8, 58u8, 139u8, 202u8, 236u8, 230u8, 117u8,
							51u8, 142u8, 247u8, 156u8, 239u8, 168u8, 220u8, 180u8, 189u8, 238u8,
							206u8, 250u8,
						],
					)
				}
				pub fn utxo_id_to_ref(
					&self,
					_0: impl ::std::borrow::Borrow<types::utxo_id_to_ref::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::utxo_id_to_ref::Param0>,
					types::utxo_id_to_ref::UtxoIdToRef,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BitcoinUtxos",
						"UtxoIdToRef",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							217u8, 13u8, 120u8, 37u8, 57u8, 155u8, 243u8, 201u8, 228u8, 157u8,
							235u8, 124u8, 28u8, 83u8, 58u8, 139u8, 202u8, 236u8, 230u8, 117u8,
							51u8, 142u8, 247u8, 156u8, 239u8, 168u8, 220u8, 180u8, 189u8, 238u8,
							206u8, 250u8,
						],
					)
				}
				#[doc = " Bitcoin UTXOs that have been submitted for ownership confirmation"]
				pub fn utxos_pending_confirmation(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::utxos_pending_confirmation::UtxosPendingConfirmation,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BitcoinUtxos",
						"UtxosPendingConfirmation",
						(),
						[
							56u8, 157u8, 248u8, 1u8, 201u8, 141u8, 124u8, 101u8, 78u8, 20u8, 68u8,
							40u8, 158u8, 225u8, 216u8, 94u8, 224u8, 34u8, 121u8, 23u8, 107u8, 2u8,
							226u8, 18u8, 251u8, 118u8, 147u8, 155u8, 48u8, 106u8, 193u8, 218u8,
						],
					)
				}
				#[doc = " An oracle-provided confirmed bitcoin block (eg, 6 blocks back)"]
				pub fn confirmed_bitcoin_block_tip(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::confirmed_bitcoin_block_tip::ConfirmedBitcoinBlockTip,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BitcoinUtxos",
						"ConfirmedBitcoinBlockTip",
						(),
						[
							55u8, 43u8, 121u8, 181u8, 157u8, 56u8, 201u8, 220u8, 116u8, 146u8,
							156u8, 65u8, 123u8, 186u8, 11u8, 116u8, 56u8, 185u8, 200u8, 220u8,
							24u8, 35u8, 245u8, 31u8, 138u8, 229u8, 216u8, 238u8, 120u8, 249u8,
							12u8, 122u8,
						],
					)
				}
				#[doc = " The last synched bitcoin block"]
				pub fn synched_bitcoin_block(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::synched_bitcoin_block::SynchedBitcoinBlock,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BitcoinUtxos",
						"SynchedBitcoinBlock",
						(),
						[
							145u8, 66u8, 134u8, 11u8, 48u8, 176u8, 166u8, 251u8, 58u8, 166u8,
							177u8, 97u8, 90u8, 183u8, 251u8, 60u8, 79u8, 103u8, 162u8, 169u8,
							215u8, 148u8, 225u8, 170u8, 23u8, 215u8, 58u8, 10u8, 195u8, 162u8,
							104u8, 52u8,
						],
					)
				}
				#[doc = " Bitcoin Oracle Operator Account"]
				pub fn oracle_operator_account(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::oracle_operator_account::OracleOperatorAccount,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BitcoinUtxos",
						"OracleOperatorAccount",
						(),
						[
							151u8, 70u8, 223u8, 250u8, 5u8, 161u8, 108u8, 102u8, 107u8, 110u8,
							60u8, 134u8, 61u8, 245u8, 222u8, 145u8, 254u8, 101u8, 37u8, 136u8,
							86u8, 228u8, 83u8, 8u8, 106u8, 61u8, 240u8, 220u8, 141u8, 81u8, 212u8,
							143u8,
						],
					)
				}
				#[doc = " Expiration date as a day since unix timestamp mapped to Bitcoin UTXOs"]
				pub fn locked_utxo_expirations_by_block_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::locked_utxo_expirations_by_block::LockedUtxoExpirationsByBlock,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"BitcoinUtxos",
						"LockedUtxoExpirationsByBlock",
						(),
						[
							74u8, 218u8, 105u8, 155u8, 234u8, 81u8, 4u8, 175u8, 46u8, 184u8, 124u8,
							58u8, 140u8, 123u8, 74u8, 1u8, 179u8, 39u8, 105u8, 114u8, 86u8, 2u8,
							190u8, 8u8, 20u8, 57u8, 60u8, 37u8, 215u8, 150u8, 103u8, 222u8,
						],
					)
				}
				#[doc = " Expiration date as a day since unix timestamp mapped to Bitcoin UTXOs"]
				pub fn locked_utxo_expirations_by_block(
					&self,
					_0: impl ::std::borrow::Borrow<types::locked_utxo_expirations_by_block::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::locked_utxo_expirations_by_block::Param0,
					>,
					types::locked_utxo_expirations_by_block::LockedUtxoExpirationsByBlock,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BitcoinUtxos",
						"LockedUtxoExpirationsByBlock",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							74u8, 218u8, 105u8, 155u8, 234u8, 81u8, 4u8, 175u8, 46u8, 184u8, 124u8,
							58u8, 140u8, 123u8, 74u8, 1u8, 179u8, 39u8, 105u8, 114u8, 86u8, 2u8,
							190u8, 8u8, 20u8, 57u8, 60u8, 37u8, 215u8, 150u8, 103u8, 222u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The maximum number of UTXOs that can be tracked in a block and/or expiring at same block"]
				pub fn max_pending_confirmation_utxos(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"BitcoinUtxos",
						"MaxPendingConfirmationUtxos",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " Maximum bitcoin blocks to watch a Utxo for confirmation before canceling"]
				pub fn max_pending_confirmation_blocks(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u64> {
					::subxt::constants::Address::new_static(
						"BitcoinUtxos",
						"MaxPendingConfirmationBlocks",
						[
							128u8, 214u8, 205u8, 242u8, 181u8, 142u8, 124u8, 231u8, 190u8, 146u8,
							59u8, 226u8, 157u8, 101u8, 103u8, 117u8, 249u8, 65u8, 18u8, 191u8,
							103u8, 119u8, 53u8, 85u8, 81u8, 96u8, 220u8, 42u8, 184u8, 239u8, 42u8,
							246u8,
						],
					)
				}
				#[doc = " The number of blocks previous to the tip that a bitcoin UTXO will be allowed to be"]
				#[doc = " locked"]
				pub fn max_utxo_birth_blocks_old(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u64> {
					::subxt::constants::Address::new_static(
						"BitcoinUtxos",
						"MaxUtxoBirthBlocksOld",
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
	pub mod vaults {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_vaults::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_vaults::pallet::Call;
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
				pub struct Create {
					pub vault_config: create::VaultConfig,
				}
				pub mod create {
					use super::runtime_types;
					pub type VaultConfig =
						runtime_types::pallet_vaults::pallet::VaultConfig<::core::primitive::u128>;
				}
				impl ::subxt::blocks::StaticExtrinsic for Create {
					const PALLET: &'static str = "Vaults";
					const CALL: &'static str = "create";
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
				#[doc = "Modify funds offered by the vault. This will not affect existing bonds, but will affect"]
				#[doc = "the amount of funds available for new bonds."]
				#[doc = ""]
				#[doc = "The securitization percent must be maintained or increased."]
				#[doc = ""]
				#[doc = "The amount offered may not go below the existing bonded amounts, but you can release"]
				#[doc = "funds in this vault as bonds are released. To stop issuing any more bonds, use the"]
				#[doc = "`close` api."]
				pub struct ModifyFunding {
					pub vault_id: modify_funding::VaultId,
					pub total_mining_amount_offered: modify_funding::TotalMiningAmountOffered,
					pub total_bitcoin_amount_offered: modify_funding::TotalBitcoinAmountOffered,
					pub securitization_percent: modify_funding::SecuritizationPercent,
				}
				pub mod modify_funding {
					use super::runtime_types;
					pub type VaultId = ::core::primitive::u32;
					pub type TotalMiningAmountOffered = ::core::primitive::u128;
					pub type TotalBitcoinAmountOffered = ::core::primitive::u128;
					pub type SecuritizationPercent =
						runtime_types::sp_arithmetic::fixed_point::FixedU128;
				}
				impl ::subxt::blocks::StaticExtrinsic for ModifyFunding {
					const PALLET: &'static str = "Vaults";
					const CALL: &'static str = "modify_funding";
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
				#[doc = "Change the terms of this vault. The change will be applied at the next mining slot"]
				#[doc = "change that is at least `MinTermsModificationBlockDelay` blocks away."]
				pub struct ModifyTerms {
					pub vault_id: modify_terms::VaultId,
					pub terms: modify_terms::Terms,
				}
				pub mod modify_terms {
					use super::runtime_types;
					pub type VaultId = ::core::primitive::u32;
					pub type Terms =
						runtime_types::ulx_primitives::bond::VaultTerms<::core::primitive::u128>;
				}
				impl ::subxt::blocks::StaticExtrinsic for ModifyTerms {
					const PALLET: &'static str = "Vaults";
					const CALL: &'static str = "modify_terms";
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
				#[doc = "Stop offering additional bonds from this vault. Will not affect existing bond."]
				#[doc = "As funds are returned, they will be released to the vault owner."]
				pub struct Close {
					pub vault_id: close::VaultId,
				}
				pub mod close {
					use super::runtime_types;
					pub type VaultId = ::core::primitive::u32;
				}
				impl ::subxt::blocks::StaticExtrinsic for Close {
					const PALLET: &'static str = "Vaults";
					const CALL: &'static str = "close";
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
				#[doc = "Add public key hashes to the vault. Will be inserted at the beginning of the list."]
				pub struct AddBitcoinPubkeyHashes {
					pub vault_id: add_bitcoin_pubkey_hashes::VaultId,
					pub bitcoin_pubkey_hashes: add_bitcoin_pubkey_hashes::BitcoinPubkeyHashes,
				}
				pub mod add_bitcoin_pubkey_hashes {
					use super::runtime_types;
					pub type VaultId = ::core::primitive::u32;
					pub type BitcoinPubkeyHashes =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash,
						>;
				}
				impl ::subxt::blocks::StaticExtrinsic for AddBitcoinPubkeyHashes {
					const PALLET: &'static str = "Vaults";
					const CALL: &'static str = "add_bitcoin_pubkey_hashes";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				pub fn create(
					&self,
					vault_config: types::create::VaultConfig,
				) -> ::subxt::tx::Payload<types::Create> {
					::subxt::tx::Payload::new_static(
						"Vaults",
						"create",
						types::Create { vault_config },
						[
							235u8, 145u8, 136u8, 106u8, 204u8, 107u8, 30u8, 11u8, 91u8, 240u8,
							119u8, 130u8, 174u8, 29u8, 65u8, 8u8, 60u8, 191u8, 184u8, 116u8, 255u8,
							53u8, 216u8, 120u8, 204u8, 249u8, 129u8, 87u8, 255u8, 186u8, 160u8,
							90u8,
						],
					)
				}
				#[doc = "Modify funds offered by the vault. This will not affect existing bonds, but will affect"]
				#[doc = "the amount of funds available for new bonds."]
				#[doc = ""]
				#[doc = "The securitization percent must be maintained or increased."]
				#[doc = ""]
				#[doc = "The amount offered may not go below the existing bonded amounts, but you can release"]
				#[doc = "funds in this vault as bonds are released. To stop issuing any more bonds, use the"]
				#[doc = "`close` api."]
				pub fn modify_funding(
					&self,
					vault_id: types::modify_funding::VaultId,
					total_mining_amount_offered: types::modify_funding::TotalMiningAmountOffered,
					total_bitcoin_amount_offered: types::modify_funding::TotalBitcoinAmountOffered,
					securitization_percent: types::modify_funding::SecuritizationPercent,
				) -> ::subxt::tx::Payload<types::ModifyFunding> {
					::subxt::tx::Payload::new_static(
						"Vaults",
						"modify_funding",
						types::ModifyFunding {
							vault_id,
							total_mining_amount_offered,
							total_bitcoin_amount_offered,
							securitization_percent,
						},
						[
							162u8, 81u8, 169u8, 67u8, 69u8, 149u8, 161u8, 166u8, 251u8, 205u8,
							45u8, 77u8, 21u8, 206u8, 146u8, 86u8, 254u8, 38u8, 197u8, 49u8, 50u8,
							35u8, 220u8, 134u8, 35u8, 251u8, 56u8, 85u8, 166u8, 226u8, 245u8, 67u8,
						],
					)
				}
				#[doc = "Change the terms of this vault. The change will be applied at the next mining slot"]
				#[doc = "change that is at least `MinTermsModificationBlockDelay` blocks away."]
				pub fn modify_terms(
					&self,
					vault_id: types::modify_terms::VaultId,
					terms: types::modify_terms::Terms,
				) -> ::subxt::tx::Payload<types::ModifyTerms> {
					::subxt::tx::Payload::new_static(
						"Vaults",
						"modify_terms",
						types::ModifyTerms { vault_id, terms },
						[
							2u8, 156u8, 165u8, 142u8, 0u8, 195u8, 81u8, 228u8, 248u8, 221u8, 102u8,
							90u8, 248u8, 139u8, 81u8, 68u8, 135u8, 92u8, 154u8, 64u8, 135u8, 38u8,
							15u8, 251u8, 191u8, 211u8, 194u8, 224u8, 68u8, 253u8, 212u8, 218u8,
						],
					)
				}
				#[doc = "Stop offering additional bonds from this vault. Will not affect existing bond."]
				#[doc = "As funds are returned, they will be released to the vault owner."]
				pub fn close(
					&self,
					vault_id: types::close::VaultId,
				) -> ::subxt::tx::Payload<types::Close> {
					::subxt::tx::Payload::new_static(
						"Vaults",
						"close",
						types::Close { vault_id },
						[
							14u8, 136u8, 33u8, 152u8, 34u8, 3u8, 88u8, 190u8, 184u8, 18u8, 3u8,
							47u8, 130u8, 23u8, 5u8, 95u8, 160u8, 30u8, 23u8, 71u8, 131u8, 115u8,
							54u8, 172u8, 62u8, 22u8, 163u8, 82u8, 49u8, 127u8, 81u8, 136u8,
						],
					)
				}
				#[doc = "Add public key hashes to the vault. Will be inserted at the beginning of the list."]
				pub fn add_bitcoin_pubkey_hashes(
					&self,
					vault_id: types::add_bitcoin_pubkey_hashes::VaultId,
					bitcoin_pubkey_hashes: types::add_bitcoin_pubkey_hashes::BitcoinPubkeyHashes,
				) -> ::subxt::tx::Payload<types::AddBitcoinPubkeyHashes> {
					::subxt::tx::Payload::new_static(
						"Vaults",
						"add_bitcoin_pubkey_hashes",
						types::AddBitcoinPubkeyHashes { vault_id, bitcoin_pubkey_hashes },
						[
							207u8, 241u8, 132u8, 96u8, 1u8, 62u8, 232u8, 135u8, 0u8, 67u8, 180u8,
							11u8, 49u8, 145u8, 211u8, 29u8, 88u8, 103u8, 254u8, 176u8, 83u8, 207u8,
							10u8, 144u8, 71u8, 6u8, 81u8, 40u8, 143u8, 224u8, 85u8, 39u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_vaults::pallet::Event;
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
			pub struct VaultCreated {
				pub vault_id: vault_created::VaultId,
				pub bitcoin_argons: vault_created::BitcoinArgons,
				pub mining_argons: vault_created::MiningArgons,
				pub securitization_percent: vault_created::SecuritizationPercent,
				pub operator_account_id: vault_created::OperatorAccountId,
			}
			pub mod vault_created {
				use super::runtime_types;
				pub type VaultId = ::core::primitive::u32;
				pub type BitcoinArgons = ::core::primitive::u128;
				pub type MiningArgons = ::core::primitive::u128;
				pub type SecuritizationPercent =
					runtime_types::sp_arithmetic::fixed_point::FixedU128;
				pub type OperatorAccountId = ::subxt::utils::AccountId32;
			}
			impl ::subxt::events::StaticEvent for VaultCreated {
				const PALLET: &'static str = "Vaults";
				const EVENT: &'static str = "VaultCreated";
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
			pub struct VaultModified {
				pub vault_id: vault_modified::VaultId,
				pub bitcoin_argons: vault_modified::BitcoinArgons,
				pub mining_argons: vault_modified::MiningArgons,
				pub securitization_percent: vault_modified::SecuritizationPercent,
			}
			pub mod vault_modified {
				use super::runtime_types;
				pub type VaultId = ::core::primitive::u32;
				pub type BitcoinArgons = ::core::primitive::u128;
				pub type MiningArgons = ::core::primitive::u128;
				pub type SecuritizationPercent =
					runtime_types::sp_arithmetic::fixed_point::FixedU128;
			}
			impl ::subxt::events::StaticEvent for VaultModified {
				const PALLET: &'static str = "Vaults";
				const EVENT: &'static str = "VaultModified";
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
			pub struct VaultTermsChangeScheduled {
				pub vault_id: vault_terms_change_scheduled::VaultId,
				pub change_block: vault_terms_change_scheduled::ChangeBlock,
			}
			pub mod vault_terms_change_scheduled {
				use super::runtime_types;
				pub type VaultId = ::core::primitive::u32;
				pub type ChangeBlock = ::core::primitive::u32;
			}
			impl ::subxt::events::StaticEvent for VaultTermsChangeScheduled {
				const PALLET: &'static str = "Vaults";
				const EVENT: &'static str = "VaultTermsChangeScheduled";
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
			pub struct VaultTermsChanged {
				pub vault_id: vault_terms_changed::VaultId,
			}
			pub mod vault_terms_changed {
				use super::runtime_types;
				pub type VaultId = ::core::primitive::u32;
			}
			impl ::subxt::events::StaticEvent for VaultTermsChanged {
				const PALLET: &'static str = "Vaults";
				const EVENT: &'static str = "VaultTermsChanged";
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
			pub struct VaultClosed {
				pub vault_id: vault_closed::VaultId,
				pub bitcoin_amount_still_bonded: vault_closed::BitcoinAmountStillBonded,
				pub mining_amount_still_bonded: vault_closed::MiningAmountStillBonded,
				pub securitization_still_bonded: vault_closed::SecuritizationStillBonded,
			}
			pub mod vault_closed {
				use super::runtime_types;
				pub type VaultId = ::core::primitive::u32;
				pub type BitcoinAmountStillBonded = ::core::primitive::u128;
				pub type MiningAmountStillBonded = ::core::primitive::u128;
				pub type SecuritizationStillBonded = ::core::primitive::u128;
			}
			impl ::subxt::events::StaticEvent for VaultClosed {
				const PALLET: &'static str = "Vaults";
				const EVENT: &'static str = "VaultClosed";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod next_vault_id {
					use super::runtime_types;
					pub type NextVaultId = ::core::primitive::u32;
				}
				pub mod vaults_by_id {
					use super::runtime_types;
					pub type VaultsById = runtime_types::ulx_primitives::bond::Vault<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
						::core::primitive::u32,
					>;
					pub type Param0 = ::core::primitive::u32;
				}
				pub mod vault_pubkeys_by_id {
					use super::runtime_types;
					pub type VaultPubkeysById =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash,
						>;
					pub type Param0 = ::core::primitive::u32;
				}
				pub mod pending_terms_modifications_by_block {
					use super::runtime_types;
					pub type PendingTermsModificationsByBlock =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u32,
						>;
					pub type Param0 = ::core::primitive::u32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				pub fn next_vault_id(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::next_vault_id::NextVaultId,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Vaults",
						"NextVaultId",
						(),
						[
							87u8, 84u8, 241u8, 173u8, 71u8, 222u8, 78u8, 131u8, 238u8, 15u8, 149u8,
							223u8, 92u8, 177u8, 171u8, 86u8, 100u8, 111u8, 89u8, 72u8, 160u8,
							219u8, 106u8, 206u8, 188u8, 177u8, 196u8, 73u8, 161u8, 41u8, 239u8,
							57u8,
						],
					)
				}
				#[doc = " Vaults by id"]
				pub fn vaults_by_id_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::vaults_by_id::VaultsById,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Vaults",
						"VaultsById",
						(),
						[
							220u8, 201u8, 2u8, 230u8, 52u8, 137u8, 3u8, 5u8, 81u8, 69u8, 48u8,
							206u8, 29u8, 50u8, 211u8, 4u8, 97u8, 89u8, 18u8, 115u8, 133u8, 104u8,
							211u8, 111u8, 213u8, 234u8, 46u8, 135u8, 162u8, 235u8, 39u8, 157u8,
						],
					)
				}
				#[doc = " Vaults by id"]
				pub fn vaults_by_id(
					&self,
					_0: impl ::std::borrow::Borrow<types::vaults_by_id::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::vaults_by_id::Param0>,
					types::vaults_by_id::VaultsById,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Vaults",
						"VaultsById",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							220u8, 201u8, 2u8, 230u8, 52u8, 137u8, 3u8, 5u8, 81u8, 69u8, 48u8,
							206u8, 29u8, 50u8, 211u8, 4u8, 97u8, 89u8, 18u8, 115u8, 133u8, 104u8,
							211u8, 111u8, 213u8, 234u8, 46u8, 135u8, 162u8, 235u8, 39u8, 157u8,
						],
					)
				}
				#[doc = " Vault Bitcoin Pubkeys by VaultId"]
				pub fn vault_pubkeys_by_id_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::vault_pubkeys_by_id::VaultPubkeysById,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Vaults",
						"VaultPubkeysById",
						(),
						[
							49u8, 205u8, 205u8, 110u8, 184u8, 91u8, 25u8, 215u8, 178u8, 118u8,
							235u8, 217u8, 79u8, 148u8, 30u8, 112u8, 64u8, 218u8, 237u8, 138u8,
							79u8, 21u8, 17u8, 143u8, 17u8, 230u8, 143u8, 69u8, 16u8, 179u8, 24u8,
							177u8,
						],
					)
				}
				#[doc = " Vault Bitcoin Pubkeys by VaultId"]
				pub fn vault_pubkeys_by_id(
					&self,
					_0: impl ::std::borrow::Borrow<types::vault_pubkeys_by_id::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::vault_pubkeys_by_id::Param0>,
					types::vault_pubkeys_by_id::VaultPubkeysById,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Vaults",
						"VaultPubkeysById",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							49u8, 205u8, 205u8, 110u8, 184u8, 91u8, 25u8, 215u8, 178u8, 118u8,
							235u8, 217u8, 79u8, 148u8, 30u8, 112u8, 64u8, 218u8, 237u8, 138u8,
							79u8, 21u8, 17u8, 143u8, 17u8, 230u8, 143u8, 69u8, 16u8, 179u8, 24u8,
							177u8,
						],
					)
				}
				#[doc = " Pending terms that will be committed at the given block number (must be a minimum of 1 slot"]
				#[doc = " change away)"]
				pub fn pending_terms_modifications_by_block_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::pending_terms_modifications_by_block::PendingTermsModificationsByBlock,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Vaults",
						"PendingTermsModificationsByBlock",
						(),
						[
							73u8, 78u8, 166u8, 219u8, 56u8, 178u8, 50u8, 224u8, 11u8, 41u8, 148u8,
							28u8, 137u8, 75u8, 207u8, 172u8, 199u8, 18u8, 175u8, 112u8, 91u8, 46u8,
							21u8, 226u8, 48u8, 43u8, 135u8, 247u8, 45u8, 100u8, 124u8, 211u8,
						],
					)
				}
				#[doc = " Pending terms that will be committed at the given block number (must be a minimum of 1 slot"]
				#[doc = " change away)"]
				pub fn pending_terms_modifications_by_block(
					&self,
					_0: impl ::std::borrow::Borrow<types::pending_terms_modifications_by_block::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::pending_terms_modifications_by_block::Param0,
					>,
					types::pending_terms_modifications_by_block::PendingTermsModificationsByBlock,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Vaults",
						"PendingTermsModificationsByBlock",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							73u8, 78u8, 166u8, 219u8, 56u8, 178u8, 50u8, 224u8, 11u8, 41u8, 148u8,
							28u8, 137u8, 75u8, 207u8, 172u8, 199u8, 18u8, 175u8, 112u8, 91u8, 46u8,
							21u8, 226u8, 48u8, 43u8, 135u8, 247u8, 45u8, 100u8, 124u8, 211u8,
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
						"Vaults",
						"MinimumBondAmount",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " Ulixee blocks per day"]
				pub fn blocks_per_day(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Vaults",
						"BlocksPerDay",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The max amount of pending bitcoin pubkey hashes allowed"]
				pub fn max_pending_vault_bitcoin_pubkeys(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Vaults",
						"MaxPendingVaultBitcoinPubkeys",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The max pending vault term changes per block"]
				pub fn max_pending_term_modifications_per_block(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Vaults",
						"MaxPendingTermModificationsPerBlock",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The number of blocks that a change in terms will take before applying. Terms only apply"]
				#[doc = " on a slot changeover, so this setting is the minimum blocks that must pass, in"]
				#[doc = " addition to the time to the next slot after that"]
				pub fn min_terms_modification_block_delay(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Vaults",
						"MinTermsModificationBlockDelay",
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
	pub mod bonds {
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
				#[doc = "Bond a bitcoin. This will create a bond for the submitting account and log the Bitcoin"]
				#[doc = "Script hash to Events. A bondee must create the UTXO in order to be added to the Bitcoin"]
				#[doc = "Mint line."]
				#[doc = ""]
				#[doc = "NOTE: The script"]
				pub struct BondBitcoin {
					pub vault_id: bond_bitcoin::VaultId,
					#[codec(compact)]
					pub satoshis: bond_bitcoin::Satoshis,
					pub bitcoin_pubkey_hash: bond_bitcoin::BitcoinPubkeyHash,
				}
				pub mod bond_bitcoin {
					use super::runtime_types;
					pub type VaultId = ::core::primitive::u32;
					pub type Satoshis = ::core::primitive::u64;
					pub type BitcoinPubkeyHash =
						runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash;
				}
				impl ::subxt::blocks::StaticExtrinsic for BondBitcoin {
					const PALLET: &'static str = "Bonds";
					const CALL: &'static str = "bond_bitcoin";
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
				#[doc = "Submitted by a Bitcoin holder to trigger the unlock of their Bitcoin. A transaction"]
				#[doc = "spending the UTXO from the given bond should be pre-created so that the sighash can be"]
				#[doc = "submitted here. The vault operator will have 10 days to counter-sign the transaction. It"]
				#[doc = "will be published with the public key as a BitcoinUtxoCosigned Event."]
				#[doc = ""]
				#[doc = "Owner must submit a script pubkey and also a fee to pay to the bitcoin network."]
				pub struct UnlockBitcoinBond {
					pub bond_id: unlock_bitcoin_bond::BondId,
					pub to_script_pubkey: unlock_bitcoin_bond::ToScriptPubkey,
					pub bitcoin_network_fee: unlock_bitcoin_bond::BitcoinNetworkFee,
				}
				pub mod unlock_bitcoin_bond {
					use super::runtime_types;
					pub type BondId = ::core::primitive::u64;
					pub type ToScriptPubkey =
						runtime_types::ulx_primitives::bitcoin::BitcoinScriptPubkey;
					pub type BitcoinNetworkFee = ::core::primitive::u64;
				}
				impl ::subxt::blocks::StaticExtrinsic for UnlockBitcoinBond {
					const PALLET: &'static str = "Bonds";
					const CALL: &'static str = "unlock_bitcoin_bond";
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
				#[doc = "Submitted by a Vault operator to cosign the unlock of a bitcoin utxo. The Bitcoin owner"]
				#[doc = "unlock fee will be burned, and the bond will be allowed to expire without penalty."]
				#[doc = ""]
				#[doc = "This is submitted as a no-fee transaction off chain to allow keys to remain in cold"]
				#[doc = "wallets."]
				pub struct CosignBitcoinUnlock {
					pub bond_id: cosign_bitcoin_unlock::BondId,
					pub pubkey: cosign_bitcoin_unlock::Pubkey,
					pub signature: cosign_bitcoin_unlock::Signature,
				}
				pub mod cosign_bitcoin_unlock {
					use super::runtime_types;
					pub type BondId = ::core::primitive::u64;
					pub type Pubkey =
						runtime_types::ulx_primitives::bitcoin::CompressedBitcoinPubkey;
					pub type Signature = runtime_types::ulx_primitives::bitcoin::BitcoinSignature;
				}
				impl ::subxt::blocks::StaticExtrinsic for CosignBitcoinUnlock {
					const PALLET: &'static str = "Bonds";
					const CALL: &'static str = "cosign_bitcoin_unlock";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Bond a bitcoin. This will create a bond for the submitting account and log the Bitcoin"]
				#[doc = "Script hash to Events. A bondee must create the UTXO in order to be added to the Bitcoin"]
				#[doc = "Mint line."]
				#[doc = ""]
				#[doc = "NOTE: The script"]
				pub fn bond_bitcoin(
					&self,
					vault_id: types::bond_bitcoin::VaultId,
					satoshis: types::bond_bitcoin::Satoshis,
					bitcoin_pubkey_hash: types::bond_bitcoin::BitcoinPubkeyHash,
				) -> ::subxt::tx::Payload<types::BondBitcoin> {
					::subxt::tx::Payload::new_static(
						"Bonds",
						"bond_bitcoin",
						types::BondBitcoin { vault_id, satoshis, bitcoin_pubkey_hash },
						[
							11u8, 123u8, 223u8, 173u8, 244u8, 36u8, 76u8, 151u8, 235u8, 70u8, 25u8,
							119u8, 13u8, 201u8, 225u8, 4u8, 247u8, 155u8, 238u8, 234u8, 250u8,
							126u8, 248u8, 167u8, 205u8, 65u8, 151u8, 97u8, 31u8, 87u8, 233u8, 89u8,
						],
					)
				}
				#[doc = "Submitted by a Bitcoin holder to trigger the unlock of their Bitcoin. A transaction"]
				#[doc = "spending the UTXO from the given bond should be pre-created so that the sighash can be"]
				#[doc = "submitted here. The vault operator will have 10 days to counter-sign the transaction. It"]
				#[doc = "will be published with the public key as a BitcoinUtxoCosigned Event."]
				#[doc = ""]
				#[doc = "Owner must submit a script pubkey and also a fee to pay to the bitcoin network."]
				pub fn unlock_bitcoin_bond(
					&self,
					bond_id: types::unlock_bitcoin_bond::BondId,
					to_script_pubkey: types::unlock_bitcoin_bond::ToScriptPubkey,
					bitcoin_network_fee: types::unlock_bitcoin_bond::BitcoinNetworkFee,
				) -> ::subxt::tx::Payload<types::UnlockBitcoinBond> {
					::subxt::tx::Payload::new_static(
						"Bonds",
						"unlock_bitcoin_bond",
						types::UnlockBitcoinBond { bond_id, to_script_pubkey, bitcoin_network_fee },
						[
							13u8, 92u8, 1u8, 190u8, 150u8, 197u8, 212u8, 123u8, 61u8, 234u8, 137u8,
							127u8, 150u8, 215u8, 176u8, 73u8, 99u8, 212u8, 68u8, 192u8, 144u8,
							108u8, 159u8, 183u8, 70u8, 31u8, 173u8, 212u8, 236u8, 233u8, 20u8,
							236u8,
						],
					)
				}
				#[doc = "Submitted by a Vault operator to cosign the unlock of a bitcoin utxo. The Bitcoin owner"]
				#[doc = "unlock fee will be burned, and the bond will be allowed to expire without penalty."]
				#[doc = ""]
				#[doc = "This is submitted as a no-fee transaction off chain to allow keys to remain in cold"]
				#[doc = "wallets."]
				pub fn cosign_bitcoin_unlock(
					&self,
					bond_id: types::cosign_bitcoin_unlock::BondId,
					pubkey: types::cosign_bitcoin_unlock::Pubkey,
					signature: types::cosign_bitcoin_unlock::Signature,
				) -> ::subxt::tx::Payload<types::CosignBitcoinUnlock> {
					::subxt::tx::Payload::new_static(
						"Bonds",
						"cosign_bitcoin_unlock",
						types::CosignBitcoinUnlock { bond_id, pubkey, signature },
						[
							78u8, 158u8, 114u8, 130u8, 161u8, 11u8, 26u8, 4u8, 32u8, 206u8, 184u8,
							156u8, 191u8, 87u8, 201u8, 120u8, 53u8, 68u8, 56u8, 231u8, 249u8, 60u8,
							101u8, 110u8, 247u8, 8u8, 172u8, 145u8, 95u8, 223u8, 254u8, 44u8,
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
			pub struct BondCreated {
				pub vault_id: bond_created::VaultId,
				pub bond_id: bond_created::BondId,
				pub bond_type: bond_created::BondType,
				pub bonded_account_id: bond_created::BondedAccountId,
				pub utxo_id: bond_created::UtxoId,
				pub amount: bond_created::Amount,
				pub expiration: bond_created::Expiration,
			}
			pub mod bond_created {
				use super::runtime_types;
				pub type VaultId = ::core::primitive::u32;
				pub type BondId = ::core::primitive::u64;
				pub type BondType = runtime_types::ulx_primitives::bond::BondType;
				pub type BondedAccountId = ::subxt::utils::AccountId32;
				pub type UtxoId = ::core::option::Option<::core::primitive::u64>;
				pub type Amount = ::core::primitive::u128;
				pub type Expiration =
					runtime_types::ulx_primitives::bond::BondExpiration<::core::primitive::u32>;
			}
			impl ::subxt::events::StaticEvent for BondCreated {
				const PALLET: &'static str = "Bonds";
				const EVENT: &'static str = "BondCreated";
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
				pub vault_id: bond_completed::VaultId,
				pub bond_id: bond_completed::BondId,
			}
			pub mod bond_completed {
				use super::runtime_types;
				pub type VaultId = ::core::primitive::u32;
				pub type BondId = ::core::primitive::u64;
			}
			impl ::subxt::events::StaticEvent for BondCompleted {
				const PALLET: &'static str = "Bonds";
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
			pub struct BondCanceled {
				pub vault_id: bond_canceled::VaultId,
				pub bond_id: bond_canceled::BondId,
				pub bonded_account_id: bond_canceled::BondedAccountId,
				pub bond_type: bond_canceled::BondType,
				pub returned_fee: bond_canceled::ReturnedFee,
			}
			pub mod bond_canceled {
				use super::runtime_types;
				pub type VaultId = ::core::primitive::u32;
				pub type BondId = ::core::primitive::u64;
				pub type BondedAccountId = ::subxt::utils::AccountId32;
				pub type BondType = runtime_types::ulx_primitives::bond::BondType;
				pub type ReturnedFee = ::core::primitive::u128;
			}
			impl ::subxt::events::StaticEvent for BondCanceled {
				const PALLET: &'static str = "Bonds";
				const EVENT: &'static str = "BondCanceled";
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
			pub struct BitcoinBondBurned {
				pub vault_id: bitcoin_bond_burned::VaultId,
				pub bond_id: bitcoin_bond_burned::BondId,
				pub utxo_id: bitcoin_bond_burned::UtxoId,
				pub amount_burned: bitcoin_bond_burned::AmountBurned,
				pub amount_held: bitcoin_bond_burned::AmountHeld,
				pub was_utxo_spent: bitcoin_bond_burned::WasUtxoSpent,
			}
			pub mod bitcoin_bond_burned {
				use super::runtime_types;
				pub type VaultId = ::core::primitive::u32;
				pub type BondId = ::core::primitive::u64;
				pub type UtxoId = ::core::primitive::u64;
				pub type AmountBurned = ::core::primitive::u128;
				pub type AmountHeld = ::core::primitive::u128;
				pub type WasUtxoSpent = ::core::primitive::bool;
			}
			impl ::subxt::events::StaticEvent for BitcoinBondBurned {
				const PALLET: &'static str = "Bonds";
				const EVENT: &'static str = "BitcoinBondBurned";
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
			pub struct BitcoinUtxoCosignRequested {
				pub bond_id: bitcoin_utxo_cosign_requested::BondId,
				pub vault_id: bitcoin_utxo_cosign_requested::VaultId,
				pub utxo_id: bitcoin_utxo_cosign_requested::UtxoId,
			}
			pub mod bitcoin_utxo_cosign_requested {
				use super::runtime_types;
				pub type BondId = ::core::primitive::u64;
				pub type VaultId = ::core::primitive::u32;
				pub type UtxoId = ::core::primitive::u64;
			}
			impl ::subxt::events::StaticEvent for BitcoinUtxoCosignRequested {
				const PALLET: &'static str = "Bonds";
				const EVENT: &'static str = "BitcoinUtxoCosignRequested";
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
			pub struct BitcoinUtxoCosigned {
				pub bond_id: bitcoin_utxo_cosigned::BondId,
				pub vault_id: bitcoin_utxo_cosigned::VaultId,
				pub utxo_id: bitcoin_utxo_cosigned::UtxoId,
				pub pubkey: bitcoin_utxo_cosigned::Pubkey,
				pub signature: bitcoin_utxo_cosigned::Signature,
			}
			pub mod bitcoin_utxo_cosigned {
				use super::runtime_types;
				pub type BondId = ::core::primitive::u64;
				pub type VaultId = ::core::primitive::u32;
				pub type UtxoId = ::core::primitive::u64;
				pub type Pubkey = runtime_types::ulx_primitives::bitcoin::CompressedBitcoinPubkey;
				pub type Signature = runtime_types::ulx_primitives::bitcoin::BitcoinSignature;
			}
			impl ::subxt::events::StaticEvent for BitcoinUtxoCosigned {
				const PALLET: &'static str = "Bonds";
				const EVENT: &'static str = "BitcoinUtxoCosigned";
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
			pub struct BitcoinCosignPastDue {
				pub bond_id: bitcoin_cosign_past_due::BondId,
				pub vault_id: bitcoin_cosign_past_due::VaultId,
				pub utxo_id: bitcoin_cosign_past_due::UtxoId,
				pub compensation_amount: bitcoin_cosign_past_due::CompensationAmount,
				pub compensation_still_owed: bitcoin_cosign_past_due::CompensationStillOwed,
				pub compensated_account_id: bitcoin_cosign_past_due::CompensatedAccountId,
			}
			pub mod bitcoin_cosign_past_due {
				use super::runtime_types;
				pub type BondId = ::core::primitive::u64;
				pub type VaultId = ::core::primitive::u32;
				pub type UtxoId = ::core::primitive::u64;
				pub type CompensationAmount = ::core::primitive::u128;
				pub type CompensationStillOwed = ::core::primitive::u128;
				pub type CompensatedAccountId = ::subxt::utils::AccountId32;
			}
			impl ::subxt::events::StaticEvent for BitcoinCosignPastDue {
				const PALLET: &'static str = "Bonds";
				const EVENT: &'static str = "BitcoinCosignPastDue";
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
			#[doc = "An error occurred while completing a bond"]
			pub struct BondCompletionError {
				pub bond_id: bond_completion_error::BondId,
				pub error: bond_completion_error::Error,
			}
			pub mod bond_completion_error {
				use super::runtime_types;
				pub type BondId = ::core::primitive::u64;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for BondCompletionError {
				const PALLET: &'static str = "Bonds";
				const EVENT: &'static str = "BondCompletionError";
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
			#[doc = "An error occurred while refunding an overdue cosigned bitcoin bond"]
			pub struct CosignOverdueError {
				pub utxo_id: cosign_overdue_error::UtxoId,
				pub error: cosign_overdue_error::Error,
			}
			pub mod cosign_overdue_error {
				use super::runtime_types;
				pub type UtxoId = ::core::primitive::u64;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for CosignOverdueError {
				const PALLET: &'static str = "Bonds";
				const EVENT: &'static str = "CosignOverdueError";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod next_bond_id {
					use super::runtime_types;
					pub type NextBondId = ::core::primitive::u64;
				}
				pub mod bonds_by_id {
					use super::runtime_types;
					pub type BondsById = runtime_types::ulx_primitives::bond::Bond<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
						::core::primitive::u32,
					>;
					pub type Param0 = ::core::primitive::u64;
				}
				pub mod mining_bond_completions {
					use super::runtime_types;
					pub type MiningBondCompletions =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u64,
						>;
					pub type Param0 = ::core::primitive::u32;
				}
				pub mod bitcoin_bond_completions {
					use super::runtime_types;
					pub type BitcoinBondCompletions =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u64,
						>;
					pub type Param0 = ::core::primitive::u64;
				}
				pub mod utxos_by_id {
					use super::runtime_types;
					pub type UtxosById = runtime_types::pallet_bond::pallet::UtxoState;
					pub type Param0 = ::core::primitive::u64;
				}
				pub mod owed_utxo_aggrieved {
					use super::runtime_types;
					pub type OwedUtxoAggrieved = (
						::subxt::utils::AccountId32,
						::core::primitive::u32,
						::core::primitive::u128,
						runtime_types::pallet_bond::pallet::UtxoState,
					);
					pub type Param0 = ::core::primitive::u64;
				}
				pub mod utxos_pending_unlock {
					use super::runtime_types;
					pub type UtxosPendingUnlock =
						runtime_types::bounded_collections::bounded_btree_map::BoundedBTreeMap<
							::core::primitive::u64,
							runtime_types::pallet_bond::pallet::UtxoCosignRequest<
								::core::primitive::u128,
							>,
						>;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				pub fn next_bond_id(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::next_bond_id::NextBondId,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"NextBondId",
						(),
						[
							5u8, 229u8, 152u8, 112u8, 204u8, 211u8, 171u8, 9u8, 47u8, 162u8, 31u8,
							88u8, 78u8, 187u8, 161u8, 163u8, 70u8, 216u8, 229u8, 145u8, 188u8,
							250u8, 163u8, 102u8, 207u8, 195u8, 149u8, 21u8, 202u8, 216u8, 11u8,
							181u8,
						],
					)
				}
				#[doc = " Bonds by id"]
				pub fn bonds_by_id_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::bonds_by_id::BondsById,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"BondsById",
						(),
						[
							197u8, 229u8, 52u8, 69u8, 194u8, 104u8, 121u8, 29u8, 29u8, 232u8, 17u8,
							12u8, 16u8, 121u8, 42u8, 233u8, 153u8, 215u8, 78u8, 66u8, 216u8, 237u8,
							51u8, 136u8, 136u8, 85u8, 239u8, 198u8, 58u8, 169u8, 8u8, 238u8,
						],
					)
				}
				#[doc = " Bonds by id"]
				pub fn bonds_by_id(
					&self,
					_0: impl ::std::borrow::Borrow<types::bonds_by_id::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::bonds_by_id::Param0>,
					types::bonds_by_id::BondsById,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"BondsById",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							197u8, 229u8, 52u8, 69u8, 194u8, 104u8, 121u8, 29u8, 29u8, 232u8, 17u8,
							12u8, 16u8, 121u8, 42u8, 233u8, 153u8, 215u8, 78u8, 66u8, 216u8, 237u8,
							51u8, 136u8, 136u8, 85u8, 239u8, 198u8, 58u8, 169u8, 8u8, 238u8,
						],
					)
				}
				#[doc = " Completion of mining bonds, upon which funds are returned to the vault"]
				pub fn mining_bond_completions_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::mining_bond_completions::MiningBondCompletions,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"MiningBondCompletions",
						(),
						[
							153u8, 181u8, 17u8, 224u8, 255u8, 219u8, 239u8, 203u8, 112u8, 13u8,
							124u8, 44u8, 185u8, 1u8, 71u8, 170u8, 230u8, 196u8, 81u8, 84u8, 173u8,
							193u8, 68u8, 17u8, 231u8, 0u8, 26u8, 21u8, 175u8, 242u8, 152u8, 133u8,
						],
					)
				}
				#[doc = " Completion of mining bonds, upon which funds are returned to the vault"]
				pub fn mining_bond_completions(
					&self,
					_0: impl ::std::borrow::Borrow<types::mining_bond_completions::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::mining_bond_completions::Param0,
					>,
					types::mining_bond_completions::MiningBondCompletions,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"MiningBondCompletions",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							153u8, 181u8, 17u8, 224u8, 255u8, 219u8, 239u8, 203u8, 112u8, 13u8,
							124u8, 44u8, 185u8, 1u8, 71u8, 170u8, 230u8, 196u8, 81u8, 84u8, 173u8,
							193u8, 68u8, 17u8, 231u8, 0u8, 26u8, 21u8, 175u8, 242u8, 152u8, 133u8,
						],
					)
				}
				#[doc = " Completion of bitcoin bonds by bitcoin height. Bond funds are returned to the vault if"]
				#[doc = " unlocked or used as the price of the bitcoin"]
				pub fn bitcoin_bond_completions_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::bitcoin_bond_completions::BitcoinBondCompletions,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"BitcoinBondCompletions",
						(),
						[
							144u8, 221u8, 131u8, 44u8, 114u8, 152u8, 35u8, 175u8, 233u8, 76u8,
							243u8, 225u8, 167u8, 73u8, 187u8, 60u8, 188u8, 49u8, 234u8, 26u8, 88u8,
							150u8, 226u8, 66u8, 154u8, 144u8, 19u8, 108u8, 54u8, 204u8, 79u8,
							218u8,
						],
					)
				}
				#[doc = " Completion of bitcoin bonds by bitcoin height. Bond funds are returned to the vault if"]
				#[doc = " unlocked or used as the price of the bitcoin"]
				pub fn bitcoin_bond_completions(
					&self,
					_0: impl ::std::borrow::Borrow<types::bitcoin_bond_completions::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::bitcoin_bond_completions::Param0,
					>,
					types::bitcoin_bond_completions::BitcoinBondCompletions,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"BitcoinBondCompletions",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							144u8, 221u8, 131u8, 44u8, 114u8, 152u8, 35u8, 175u8, 233u8, 76u8,
							243u8, 225u8, 167u8, 73u8, 187u8, 60u8, 188u8, 49u8, 234u8, 26u8, 88u8,
							150u8, 226u8, 66u8, 154u8, 144u8, 19u8, 108u8, 54u8, 204u8, 79u8,
							218u8,
						],
					)
				}
				#[doc = " Stores bitcoin utxos that have requested to be unlocked"]
				pub fn utxos_by_id_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::utxos_by_id::UtxosById,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"UtxosById",
						(),
						[
							17u8, 124u8, 131u8, 220u8, 113u8, 106u8, 219u8, 50u8, 117u8, 84u8,
							148u8, 193u8, 223u8, 88u8, 67u8, 94u8, 51u8, 255u8, 82u8, 40u8, 3u8,
							225u8, 215u8, 55u8, 189u8, 61u8, 254u8, 153u8, 235u8, 147u8, 232u8,
							240u8,
						],
					)
				}
				#[doc = " Stores bitcoin utxos that have requested to be unlocked"]
				pub fn utxos_by_id(
					&self,
					_0: impl ::std::borrow::Borrow<types::utxos_by_id::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::utxos_by_id::Param0>,
					types::utxos_by_id::UtxosById,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"UtxosById",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							17u8, 124u8, 131u8, 220u8, 113u8, 106u8, 219u8, 50u8, 117u8, 84u8,
							148u8, 193u8, 223u8, 88u8, 67u8, 94u8, 51u8, 255u8, 82u8, 40u8, 3u8,
							225u8, 215u8, 55u8, 189u8, 61u8, 254u8, 153u8, 235u8, 147u8, 232u8,
							240u8,
						],
					)
				}
				#[doc = " Stores Utxos that were not paid back in full"]
				#[doc = ""]
				#[doc = " Tuple stores Account, Vault, Still Owed, State"]
				pub fn owed_utxo_aggrieved_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::owed_utxo_aggrieved::OwedUtxoAggrieved,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"OwedUtxoAggrieved",
						(),
						[
							64u8, 170u8, 57u8, 192u8, 109u8, 155u8, 247u8, 215u8, 67u8, 120u8,
							218u8, 195u8, 226u8, 202u8, 4u8, 69u8, 191u8, 150u8, 108u8, 54u8, 11u8,
							173u8, 251u8, 7u8, 92u8, 4u8, 53u8, 234u8, 213u8, 47u8, 41u8, 217u8,
						],
					)
				}
				#[doc = " Stores Utxos that were not paid back in full"]
				#[doc = ""]
				#[doc = " Tuple stores Account, Vault, Still Owed, State"]
				pub fn owed_utxo_aggrieved(
					&self,
					_0: impl ::std::borrow::Borrow<types::owed_utxo_aggrieved::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::owed_utxo_aggrieved::Param0>,
					types::owed_utxo_aggrieved::OwedUtxoAggrieved,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"OwedUtxoAggrieved",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							64u8, 170u8, 57u8, 192u8, 109u8, 155u8, 247u8, 215u8, 67u8, 120u8,
							218u8, 195u8, 226u8, 202u8, 4u8, 69u8, 191u8, 150u8, 108u8, 54u8, 11u8,
							173u8, 251u8, 7u8, 92u8, 4u8, 53u8, 234u8, 213u8, 47u8, 41u8, 217u8,
						],
					)
				}
				#[doc = " Utxos that have been requested to be cosigned for unlocking"]
				pub fn utxos_pending_unlock(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::utxos_pending_unlock::UtxosPendingUnlock,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Bonds",
						"UtxosPendingUnlock",
						(),
						[
							26u8, 163u8, 85u8, 23u8, 237u8, 214u8, 97u8, 34u8, 215u8, 148u8, 76u8,
							76u8, 100u8, 12u8, 220u8, 213u8, 140u8, 170u8, 112u8, 250u8, 181u8,
							222u8, 3u8, 36u8, 182u8, 253u8, 73u8, 45u8, 100u8, 115u8, 175u8, 9u8,
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
						"Bonds",
						"MinimumBondAmount",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " Ulixee blocks per day"]
				pub fn ulixee_blocks_per_day(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Bonds",
						"UlixeeBlocksPerDay",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " Maximum unlocking utxos at a time"]
				pub fn max_unlocking_utxos(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Bonds",
						"MaxUnlockingUtxos",
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
						"Bonds",
						"MaxConcurrentlyExpiringBonds",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The minimum number of satoshis that can be bonded"]
				pub fn minimum_bitcoin_bond_satoshis(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u64> {
					::subxt::constants::Address::new_static(
						"Bonds",
						"MinimumBitcoinBondSatoshis",
						[
							128u8, 214u8, 205u8, 242u8, 181u8, 142u8, 124u8, 231u8, 190u8, 146u8,
							59u8, 226u8, 157u8, 101u8, 103u8, 117u8, 249u8, 65u8, 18u8, 191u8,
							103u8, 119u8, 53u8, 85u8, 81u8, 96u8, 220u8, 42u8, 184u8, 239u8, 42u8,
							246u8,
						],
					)
				}
				#[doc = " The number of blocks a bitcoin bond is locked for"]
				pub fn bitcoin_bond_duration_blocks(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u64> {
					::subxt::constants::Address::new_static(
						"Bonds",
						"BitcoinBondDurationBlocks",
						[
							128u8, 214u8, 205u8, 242u8, 181u8, 142u8, 124u8, 231u8, 190u8, 146u8,
							59u8, 226u8, 157u8, 101u8, 103u8, 117u8, 249u8, 65u8, 18u8, 191u8,
							103u8, 119u8, 53u8, 85u8, 81u8, 96u8, 220u8, 42u8, 184u8, 239u8, 42u8,
							246u8,
						],
					)
				}
				#[doc = " The bitcoin blocks after a bond expires which the vault will be allowed to claim a"]
				#[doc = " bitcoin"]
				pub fn bitcoin_bond_reclamation_blocks(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u64> {
					::subxt::constants::Address::new_static(
						"Bonds",
						"BitcoinBondReclamationBlocks",
						[
							128u8, 214u8, 205u8, 242u8, 181u8, 142u8, 124u8, 231u8, 190u8, 146u8,
							59u8, 226u8, 157u8, 101u8, 103u8, 117u8, 249u8, 65u8, 18u8, 191u8,
							103u8, 119u8, 53u8, 85u8, 81u8, 96u8, 220u8, 42u8, 184u8, 239u8, 42u8,
							246u8,
						],
					)
				}
				#[doc = " Number of bitcoin blocks a vault has to counter-sign a bitcoin unlock"]
				pub fn utxo_unlock_cosign_deadline_blocks(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u64> {
					::subxt::constants::Address::new_static(
						"Bonds",
						"UtxoUnlockCosignDeadlineBlocks",
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
					pub meta: propose::Meta,
				}
				pub mod propose {
					use super::runtime_types;
					pub type Meta = runtime_types::ulx_primitives::notary::NotaryMeta;
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
					pub operator_account: activate::OperatorAccount,
				}
				pub mod activate {
					use super::runtime_types;
					pub type OperatorAccount = ::subxt::utils::AccountId32;
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
				#[doc = "Update the metadata of a notary, to be effective at the given tick height, which must be"]
				#[doc = ">= MetaChangesTickDelay ticks in the future."]
				pub struct Update {
					#[codec(compact)]
					pub notary_id: update::NotaryId,
					pub meta: update::Meta,
					#[codec(compact)]
					pub effective_tick: update::EffectiveTick,
				}
				pub mod update {
					use super::runtime_types;
					pub type NotaryId = ::core::primitive::u32;
					pub type Meta = runtime_types::ulx_primitives::notary::NotaryMeta;
					pub type EffectiveTick = ::core::primitive::u32;
				}
				impl ::subxt::blocks::StaticExtrinsic for Update {
					const PALLET: &'static str = "Notaries";
					const CALL: &'static str = "update";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				pub fn propose(
					&self,
					meta: types::propose::Meta,
				) -> ::subxt::tx::Payload<types::Propose> {
					::subxt::tx::Payload::new_static(
						"Notaries",
						"propose",
						types::Propose { meta },
						[
							43u8, 165u8, 55u8, 140u8, 106u8, 117u8, 25u8, 74u8, 191u8, 110u8,
							115u8, 125u8, 54u8, 34u8, 71u8, 157u8, 52u8, 56u8, 128u8, 11u8, 143u8,
							94u8, 121u8, 195u8, 203u8, 26u8, 128u8, 90u8, 198u8, 33u8, 87u8, 241u8,
						],
					)
				}
				pub fn activate(
					&self,
					operator_account: types::activate::OperatorAccount,
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
				#[doc = "Update the metadata of a notary, to be effective at the given tick height, which must be"]
				#[doc = ">= MetaChangesTickDelay ticks in the future."]
				pub fn update(
					&self,
					notary_id: types::update::NotaryId,
					meta: types::update::Meta,
					effective_tick: types::update::EffectiveTick,
				) -> ::subxt::tx::Payload<types::Update> {
					::subxt::tx::Payload::new_static(
						"Notaries",
						"update",
						types::Update { notary_id, meta, effective_tick },
						[
							120u8, 206u8, 188u8, 143u8, 26u8, 121u8, 25u8, 34u8, 108u8, 154u8,
							209u8, 53u8, 182u8, 242u8, 116u8, 9u8, 180u8, 244u8, 90u8, 79u8, 153u8,
							121u8, 122u8, 172u8, 210u8, 48u8, 70u8, 142u8, 62u8, 150u8, 60u8, 14u8,
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
				pub operator_account: notary_proposed::OperatorAccount,
				pub meta: notary_proposed::Meta,
				pub expires: notary_proposed::Expires,
			}
			pub mod notary_proposed {
				use super::runtime_types;
				pub type OperatorAccount = ::subxt::utils::AccountId32;
				pub type Meta = runtime_types::ulx_primitives::notary::NotaryMeta;
				pub type Expires = ::core::primitive::u32;
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
				pub notary: notary_activated::Notary,
			}
			pub mod notary_activated {
				use super::runtime_types;
				pub type Notary = runtime_types::ulx_primitives::notary::NotaryRecord<
					::subxt::utils::AccountId32,
					::core::primitive::u32,
				>;
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
				pub notary_id: notary_meta_update_queued::NotaryId,
				pub meta: notary_meta_update_queued::Meta,
				pub effective_tick: notary_meta_update_queued::EffectiveTick,
			}
			pub mod notary_meta_update_queued {
				use super::runtime_types;
				pub type NotaryId = ::core::primitive::u32;
				pub type Meta = runtime_types::ulx_primitives::notary::NotaryMeta;
				pub type EffectiveTick = ::core::primitive::u32;
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
				pub notary_id: notary_meta_updated::NotaryId,
				pub meta: notary_meta_updated::Meta,
			}
			pub mod notary_meta_updated {
				use super::runtime_types;
				pub type NotaryId = ::core::primitive::u32;
				pub type Meta = runtime_types::ulx_primitives::notary::NotaryMeta;
			}
			impl ::subxt::events::StaticEvent for NotaryMetaUpdated {
				const PALLET: &'static str = "Notaries";
				const EVENT: &'static str = "NotaryMetaUpdated";
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
			#[doc = "Error updating queued notary info"]
			pub struct NotaryMetaUpdateError {
				pub notary_id: notary_meta_update_error::NotaryId,
				pub error: notary_meta_update_error::Error,
				pub meta: notary_meta_update_error::Meta,
			}
			pub mod notary_meta_update_error {
				use super::runtime_types;
				pub type NotaryId = ::core::primitive::u32;
				pub type Error = runtime_types::sp_runtime::DispatchError;
				pub type Meta = runtime_types::ulx_primitives::notary::NotaryMeta;
			}
			impl ::subxt::events::StaticEvent for NotaryMetaUpdateError {
				const PALLET: &'static str = "Notaries";
				const EVENT: &'static str = "NotaryMetaUpdateError";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod next_notary_id {
					use super::runtime_types;
					pub type NextNotaryId = ::core::primitive::u32;
				}
				pub mod proposed_notaries {
					use super::runtime_types;
					pub type ProposedNotaries =
						(runtime_types::ulx_primitives::notary::NotaryMeta, ::core::primitive::u32);
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod expiring_proposals {
					use super::runtime_types;
					pub type ExpiringProposals =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::subxt::utils::AccountId32,
						>;
					pub type Param0 = ::core::primitive::u32;
				}
				pub mod active_notaries {
					use super::runtime_types;
					pub type ActiveNotaries =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::notary::NotaryRecord<
								::subxt::utils::AccountId32,
								::core::primitive::u32,
							>,
						>;
				}
				pub mod notary_key_history {
					use super::runtime_types;
					pub type NotaryKeyHistory =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<(
							::core::primitive::u32,
							[::core::primitive::u8; 32usize],
						)>;
					pub type Param0 = ::core::primitive::u32;
				}
				pub mod queued_notary_meta_changes {
					use super::runtime_types;
					pub type QueuedNotaryMetaChanges =
						runtime_types::bounded_collections::bounded_btree_map::BoundedBTreeMap<
							::core::primitive::u32,
							runtime_types::ulx_primitives::notary::NotaryMeta,
						>;
					pub type Param0 = ::core::primitive::u32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				pub fn next_notary_id(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::next_notary_id::NextNotaryId,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"NextNotaryId",
						(),
						[
							246u8, 48u8, 149u8, 160u8, 181u8, 5u8, 135u8, 44u8, 164u8, 37u8, 82u8,
							255u8, 240u8, 24u8, 171u8, 176u8, 255u8, 52u8, 54u8, 210u8, 131u8,
							113u8, 102u8, 36u8, 241u8, 251u8, 53u8, 118u8, 13u8, 52u8, 230u8, 7u8,
						],
					)
				}
				pub fn proposed_notaries_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::proposed_notaries::ProposedNotaries,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"ProposedNotaries",
						(),
						[
							89u8, 32u8, 91u8, 88u8, 147u8, 56u8, 8u8, 54u8, 59u8, 182u8, 252u8,
							90u8, 180u8, 110u8, 23u8, 137u8, 248u8, 196u8, 14u8, 158u8, 174u8,
							39u8, 114u8, 123u8, 98u8, 23u8, 167u8, 194u8, 23u8, 159u8, 102u8,
							103u8,
						],
					)
				}
				pub fn proposed_notaries(
					&self,
					_0: impl ::std::borrow::Borrow<types::proposed_notaries::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::proposed_notaries::Param0>,
					types::proposed_notaries::ProposedNotaries,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"ProposedNotaries",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							89u8, 32u8, 91u8, 88u8, 147u8, 56u8, 8u8, 54u8, 59u8, 182u8, 252u8,
							90u8, 180u8, 110u8, 23u8, 137u8, 248u8, 196u8, 14u8, 158u8, 174u8,
							39u8, 114u8, 123u8, 98u8, 23u8, 167u8, 194u8, 23u8, 159u8, 102u8,
							103u8,
						],
					)
				}
				pub fn expiring_proposals_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::expiring_proposals::ExpiringProposals,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"ExpiringProposals",
						(),
						[
							64u8, 68u8, 247u8, 229u8, 147u8, 217u8, 204u8, 231u8, 82u8, 104u8,
							212u8, 163u8, 195u8, 244u8, 63u8, 148u8, 181u8, 120u8, 176u8, 52u8,
							125u8, 39u8, 74u8, 241u8, 126u8, 83u8, 45u8, 96u8, 30u8, 29u8, 155u8,
							108u8,
						],
					)
				}
				pub fn expiring_proposals(
					&self,
					_0: impl ::std::borrow::Borrow<types::expiring_proposals::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::expiring_proposals::Param0>,
					types::expiring_proposals::ExpiringProposals,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"ExpiringProposals",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
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
					(),
					types::active_notaries::ActiveNotaries,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"ActiveNotaries",
						(),
						[
							120u8, 168u8, 144u8, 200u8, 209u8, 168u8, 173u8, 186u8, 99u8, 9u8,
							162u8, 212u8, 120u8, 72u8, 26u8, 114u8, 187u8, 232u8, 177u8, 86u8,
							104u8, 165u8, 110u8, 71u8, 33u8, 204u8, 237u8, 148u8, 233u8, 35u8,
							139u8, 60u8,
						],
					)
				}
				pub fn notary_key_history_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::notary_key_history::NotaryKeyHistory,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"NotaryKeyHistory",
						(),
						[
							59u8, 52u8, 23u8, 225u8, 223u8, 28u8, 225u8, 2u8, 39u8, 170u8, 155u8,
							214u8, 55u8, 134u8, 180u8, 53u8, 230u8, 255u8, 30u8, 165u8, 102u8,
							81u8, 80u8, 26u8, 213u8, 207u8, 158u8, 183u8, 71u8, 77u8, 191u8, 123u8,
						],
					)
				}
				pub fn notary_key_history(
					&self,
					_0: impl ::std::borrow::Borrow<types::notary_key_history::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::notary_key_history::Param0>,
					types::notary_key_history::NotaryKeyHistory,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"NotaryKeyHistory",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							59u8, 52u8, 23u8, 225u8, 223u8, 28u8, 225u8, 2u8, 39u8, 170u8, 155u8,
							214u8, 55u8, 134u8, 180u8, 53u8, 230u8, 255u8, 30u8, 165u8, 102u8,
							81u8, 80u8, 26u8, 213u8, 207u8, 158u8, 183u8, 71u8, 77u8, 191u8, 123u8,
						],
					)
				}
				#[doc = " Metadata changes to be activated at the given tick"]
				pub fn queued_notary_meta_changes_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::queued_notary_meta_changes::QueuedNotaryMetaChanges,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"QueuedNotaryMetaChanges",
						(),
						[
							53u8, 211u8, 146u8, 97u8, 213u8, 139u8, 42u8, 197u8, 253u8, 77u8,
							144u8, 117u8, 91u8, 179u8, 122u8, 83u8, 177u8, 108u8, 91u8, 50u8,
							195u8, 248u8, 70u8, 99u8, 68u8, 254u8, 75u8, 140u8, 195u8, 193u8, 81u8,
							25u8,
						],
					)
				}
				#[doc = " Metadata changes to be activated at the given tick"]
				pub fn queued_notary_meta_changes(
					&self,
					_0: impl ::std::borrow::Borrow<types::queued_notary_meta_changes::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::queued_notary_meta_changes::Param0,
					>,
					types::queued_notary_meta_changes::QueuedNotaryMetaChanges,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notaries",
						"QueuedNotaryMetaChanges",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							53u8, 211u8, 146u8, 97u8, 213u8, 139u8, 42u8, 197u8, 253u8, 77u8,
							144u8, 117u8, 91u8, 179u8, 122u8, 83u8, 177u8, 108u8, 91u8, 50u8,
							195u8, 248u8, 70u8, 99u8, 68u8, 254u8, 75u8, 140u8, 195u8, 193u8, 81u8,
							25u8,
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
				#[doc = " Number of ticks to delay changing a notaries' meta (this is to allow a window for"]
				#[doc = " notaries to switch to new keys after a new key is finalized)"]
				pub fn meta_changes_tick_delay(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Notaries",
						"MetaChangesTickDelay",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " Number of ticks to maintain key history for each notary"]
				#[doc = " NOTE: only pruned when new keys are added"]
				pub fn max_ticks_for_key_history(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Notaries",
						"MaxTicksForKeyHistory",
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
					pub notebooks: submit::Notebooks,
				}
				pub mod submit {
					use super::runtime_types;
					pub type Notebooks = ::std::vec::Vec<
						runtime_types::ulx_primitives::notebook::SignedNotebookHeader,
					>;
				}
				impl ::subxt::blocks::StaticExtrinsic for Submit {
					const PALLET: &'static str = "Notebook";
					const CALL: &'static str = "submit";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				pub fn submit(
					&self,
					notebooks: types::submit::Notebooks,
				) -> ::subxt::tx::Payload<types::Submit> {
					::subxt::tx::Payload::new_static(
						"Notebook",
						"submit",
						types::Submit { notebooks },
						[
							226u8, 89u8, 102u8, 115u8, 24u8, 77u8, 60u8, 95u8, 160u8, 22u8, 110u8,
							135u8, 8u8, 45u8, 166u8, 138u8, 247u8, 23u8, 18u8, 65u8, 247u8, 171u8,
							43u8, 78u8, 118u8, 91u8, 56u8, 174u8, 235u8, 5u8, 87u8, 38u8,
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
				pub notary_id: notebook_submitted::NotaryId,
				pub notebook_number: notebook_submitted::NotebookNumber,
			}
			pub mod notebook_submitted {
				use super::runtime_types;
				pub type NotaryId = ::core::primitive::u32;
				pub type NotebookNumber = ::core::primitive::u32;
			}
			impl ::subxt::events::StaticEvent for NotebookSubmitted {
				const PALLET: &'static str = "Notebook";
				const EVENT: &'static str = "NotebookSubmitted";
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
			pub struct NotebookAuditFailure {
				pub notary_id: notebook_audit_failure::NotaryId,
				pub notebook_number: notebook_audit_failure::NotebookNumber,
				pub first_failure_reason: notebook_audit_failure::FirstFailureReason,
			}
			pub mod notebook_audit_failure {
				use super::runtime_types;
				pub type NotaryId = ::core::primitive::u32;
				pub type NotebookNumber = ::core::primitive::u32;
				pub type FirstFailureReason = runtime_types::ulx_notary_audit::error::VerifyError;
			}
			impl ::subxt::events::StaticEvent for NotebookAuditFailure {
				const PALLET: &'static str = "Notebook";
				const EVENT: &'static str = "NotebookAuditFailure";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod notebook_changed_accounts_root_by_notary {
					use super::runtime_types;
					pub type NotebookChangedAccountsRootByNotary = ::sp_core::H256;
					pub type Param0 = ::core::primitive::u32;
					pub type Param1 = ::core::primitive::u32;
				}
				pub mod account_origin_last_changed_notebook_by_notary {
					use super::runtime_types;
					pub type AccountOriginLastChangedNotebookByNotary = ::core::primitive::u32;
					pub type Param0 = ::core::primitive::u32;
					pub type Param1 = runtime_types::ulx_primitives::balance_change::AccountOrigin;
				}
				pub mod last_notebook_details_by_notary {
					use super::runtime_types;
					pub type LastNotebookDetailsByNotary =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<(
							runtime_types::ulx_primitives::notary::NotaryNotebookKeyDetails,
							::core::primitive::bool,
						)>;
					pub type Param0 = ::core::primitive::u32;
				}
				pub mod block_notebooks {
					use super::runtime_types;
					pub type BlockNotebooks =
						runtime_types::ulx_primitives::digests::NotebookDigest<
							runtime_types::ulx_notary_audit::error::VerifyError,
						>;
				}
				pub mod temp_notebook_digest {
					use super::runtime_types;
					pub type TempNotebookDigest =
						runtime_types::ulx_primitives::digests::NotebookDigest<
							runtime_types::ulx_notary_audit::error::VerifyError,
						>;
				}
				pub mod notaries_locked_for_failed_audit {
					use super::runtime_types;
					pub type NotariesLockedForFailedAudit = (
						::core::primitive::u32,
						::core::primitive::u32,
						runtime_types::ulx_notary_audit::error::VerifyError,
					);
					pub type Param0 = ::core::primitive::u32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Double storage map of notary id + notebook # to the change root"]				pub fn notebook_changed_accounts_root_by_notary_iter (& self ,) -> :: subxt :: storage :: address :: Address :: < () , types :: notebook_changed_accounts_root_by_notary :: NotebookChangedAccountsRootByNotary , () , () , :: subxt :: storage :: address :: Yes >{
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"NotebookChangedAccountsRootByNotary",
						(),
						[
							84u8, 136u8, 124u8, 162u8, 187u8, 104u8, 116u8, 80u8, 119u8, 130u8,
							77u8, 8u8, 34u8, 154u8, 63u8, 59u8, 4u8, 169u8, 227u8, 231u8, 95u8,
							16u8, 2u8, 116u8, 193u8, 76u8, 174u8, 109u8, 254u8, 206u8, 159u8,
							109u8,
						],
					)
				}
				#[doc = " Double storage map of notary id + notebook # to the change root"]				pub fn notebook_changed_accounts_root_by_notary_iter1 (& self , _0 : impl :: std :: borrow :: Borrow < types :: notebook_changed_accounts_root_by_notary :: Param0 > ,) -> :: subxt :: storage :: address :: Address :: < :: subxt :: storage :: address :: StaticStorageKey < types :: notebook_changed_accounts_root_by_notary :: Param0 > , types :: notebook_changed_accounts_root_by_notary :: NotebookChangedAccountsRootByNotary , () , () , :: subxt :: storage :: address :: Yes >{
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"NotebookChangedAccountsRootByNotary",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							84u8, 136u8, 124u8, 162u8, 187u8, 104u8, 116u8, 80u8, 119u8, 130u8,
							77u8, 8u8, 34u8, 154u8, 63u8, 59u8, 4u8, 169u8, 227u8, 231u8, 95u8,
							16u8, 2u8, 116u8, 193u8, 76u8, 174u8, 109u8, 254u8, 206u8, 159u8,
							109u8,
						],
					)
				}
				#[doc = " Double storage map of notary id + notebook # to the change root"]				pub fn notebook_changed_accounts_root_by_notary (& self , _0 : impl :: std :: borrow :: Borrow < types :: notebook_changed_accounts_root_by_notary :: Param0 > , _1 : impl :: std :: borrow :: Borrow < types :: notebook_changed_accounts_root_by_notary :: Param1 > ,) -> :: subxt :: storage :: address :: Address :: < (:: subxt :: storage :: address :: StaticStorageKey < types :: notebook_changed_accounts_root_by_notary :: Param0 > , :: subxt :: storage :: address :: StaticStorageKey < types :: notebook_changed_accounts_root_by_notary :: Param1 > ,) , types :: notebook_changed_accounts_root_by_notary :: NotebookChangedAccountsRootByNotary , :: subxt :: storage :: address :: Yes , () , () >{
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"NotebookChangedAccountsRootByNotary",
						(
							::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
							::subxt::storage::address::StaticStorageKey::new(_1.borrow()),
						),
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
				#[doc = " (NotebookChangedAccountsRootByNotary)"]				pub fn account_origin_last_changed_notebook_by_notary_iter (& self ,) -> :: subxt :: storage :: address :: Address :: < () , types :: account_origin_last_changed_notebook_by_notary :: AccountOriginLastChangedNotebookByNotary , () , () , :: subxt :: storage :: address :: Yes >{
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"AccountOriginLastChangedNotebookByNotary",
						(),
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
				#[doc = " (NotebookChangedAccountsRootByNotary)"]				pub fn account_origin_last_changed_notebook_by_notary_iter1 (& self , _0 : impl :: std :: borrow :: Borrow < types :: account_origin_last_changed_notebook_by_notary :: Param0 > ,) -> :: subxt :: storage :: address :: Address :: < :: subxt :: storage :: address :: StaticStorageKey < types :: account_origin_last_changed_notebook_by_notary :: Param0 > , types :: account_origin_last_changed_notebook_by_notary :: AccountOriginLastChangedNotebookByNotary , () , () , :: subxt :: storage :: address :: Yes >{
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"AccountOriginLastChangedNotebookByNotary",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
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
				#[doc = " (NotebookChangedAccountsRootByNotary)"]				pub fn account_origin_last_changed_notebook_by_notary (& self , _0 : impl :: std :: borrow :: Borrow < types :: account_origin_last_changed_notebook_by_notary :: Param0 > , _1 : impl :: std :: borrow :: Borrow < types :: account_origin_last_changed_notebook_by_notary :: Param1 > ,) -> :: subxt :: storage :: address :: Address :: < (:: subxt :: storage :: address :: StaticStorageKey < types :: account_origin_last_changed_notebook_by_notary :: Param0 > , :: subxt :: storage :: address :: StaticStorageKey < types :: account_origin_last_changed_notebook_by_notary :: Param1 > ,) , types :: account_origin_last_changed_notebook_by_notary :: AccountOriginLastChangedNotebookByNotary , :: subxt :: storage :: address :: Yes , () , () >{
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"AccountOriginLastChangedNotebookByNotary",
						(
							::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
							::subxt::storage::address::StaticStorageKey::new(_1.borrow()),
						),
						[
							233u8, 5u8, 227u8, 113u8, 187u8, 168u8, 114u8, 176u8, 38u8, 129u8,
							116u8, 70u8, 109u8, 153u8, 173u8, 216u8, 216u8, 105u8, 245u8, 249u8,
							164u8, 236u8, 233u8, 205u8, 156u8, 134u8, 105u8, 157u8, 196u8, 182u8,
							144u8, 213u8,
						],
					)
				}
				#[doc = " List of last few notebook details by notary. The bool is whether the notebook is eligible"]
				#[doc = " for votes (received at correct tick and audit passed)"]
				pub fn last_notebook_details_by_notary_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::last_notebook_details_by_notary::LastNotebookDetailsByNotary,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"LastNotebookDetailsByNotary",
						(),
						[
							64u8, 129u8, 238u8, 122u8, 17u8, 221u8, 69u8, 225u8, 72u8, 184u8,
							105u8, 250u8, 99u8, 151u8, 43u8, 252u8, 57u8, 109u8, 163u8, 1u8, 135u8,
							215u8, 78u8, 62u8, 248u8, 161u8, 207u8, 89u8, 136u8, 227u8, 59u8, 78u8,
						],
					)
				}
				#[doc = " List of last few notebook details by notary. The bool is whether the notebook is eligible"]
				#[doc = " for votes (received at correct tick and audit passed)"]
				pub fn last_notebook_details_by_notary(
					&self,
					_0: impl ::std::borrow::Borrow<types::last_notebook_details_by_notary::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::last_notebook_details_by_notary::Param0,
					>,
					types::last_notebook_details_by_notary::LastNotebookDetailsByNotary,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"LastNotebookDetailsByNotary",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							64u8, 129u8, 238u8, 122u8, 17u8, 221u8, 69u8, 225u8, 72u8, 184u8,
							105u8, 250u8, 99u8, 151u8, 43u8, 252u8, 57u8, 109u8, 163u8, 1u8, 135u8,
							215u8, 78u8, 62u8, 248u8, 161u8, 207u8, 89u8, 136u8, 227u8, 59u8, 78u8,
						],
					)
				}
				#[doc = " The notebooks included in this block"]
				pub fn block_notebooks(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::block_notebooks::BlockNotebooks,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"BlockNotebooks",
						(),
						[
							195u8, 186u8, 187u8, 231u8, 93u8, 224u8, 232u8, 100u8, 244u8, 111u8,
							168u8, 152u8, 59u8, 65u8, 156u8, 150u8, 181u8, 118u8, 106u8, 250u8,
							219u8, 224u8, 211u8, 34u8, 236u8, 168u8, 231u8, 190u8, 45u8, 69u8,
							91u8, 136u8,
						],
					)
				}
				#[doc = " Temporary store a copy of the notebook digest in storage"]
				pub fn temp_notebook_digest(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::temp_notebook_digest::TempNotebookDigest,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"TempNotebookDigest",
						(),
						[
							163u8, 174u8, 68u8, 196u8, 172u8, 0u8, 5u8, 180u8, 244u8, 60u8, 114u8,
							38u8, 197u8, 134u8, 94u8, 115u8, 5u8, 240u8, 229u8, 11u8, 21u8, 88u8,
							151u8, 105u8, 216u8, 109u8, 56u8, 236u8, 239u8, 230u8, 183u8, 168u8,
						],
					)
				}
				#[doc = " Notaries locked for failing audits"]
				#[doc = " TODO: we need a mechanism to unlock a notary with \"Fixes\""]
				pub fn notaries_locked_for_failed_audit_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::notaries_locked_for_failed_audit::NotariesLockedForFailedAudit,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"NotariesLockedForFailedAudit",
						(),
						[
							190u8, 65u8, 197u8, 240u8, 114u8, 105u8, 88u8, 238u8, 11u8, 127u8,
							30u8, 196u8, 222u8, 73u8, 184u8, 173u8, 4u8, 54u8, 40u8, 99u8, 92u8,
							181u8, 2u8, 196u8, 235u8, 157u8, 199u8, 153u8, 125u8, 139u8, 223u8,
							203u8,
						],
					)
				}
				#[doc = " Notaries locked for failing audits"]
				#[doc = " TODO: we need a mechanism to unlock a notary with \"Fixes\""]
				pub fn notaries_locked_for_failed_audit(
					&self,
					_0: impl ::std::borrow::Borrow<types::notaries_locked_for_failed_audit::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::notaries_locked_for_failed_audit::Param0,
					>,
					types::notaries_locked_for_failed_audit::NotariesLockedForFailedAudit,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Notebook",
						"NotariesLockedForFailedAudit",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							190u8, 65u8, 197u8, 240u8, 114u8, 105u8, 88u8, 238u8, 11u8, 127u8,
							30u8, 196u8, 222u8, 73u8, 184u8, 173u8, 4u8, 54u8, 40u8, 99u8, 92u8,
							181u8, 2u8, 196u8, 235u8, 157u8, 199u8, 153u8, 125u8, 139u8, 223u8,
							203u8,
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
					pub amount: send_to_localchain::Amount,
					pub notary_id: send_to_localchain::NotaryId,
				}
				pub mod send_to_localchain {
					use super::runtime_types;
					pub type Amount = ::core::primitive::u128;
					pub type NotaryId = ::core::primitive::u32;
				}
				impl ::subxt::blocks::StaticExtrinsic for SendToLocalchain {
					const PALLET: &'static str = "ChainTransfer";
					const CALL: &'static str = "send_to_localchain";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				pub fn send_to_localchain(
					&self,
					amount: types::send_to_localchain::Amount,
					notary_id: types::send_to_localchain::NotaryId,
				) -> ::subxt::tx::Payload<types::SendToLocalchain> {
					::subxt::tx::Payload::new_static(
						"ChainTransfer",
						"send_to_localchain",
						types::SendToLocalchain { amount, notary_id },
						[
							83u8, 216u8, 66u8, 149u8, 234u8, 85u8, 61u8, 45u8, 152u8, 156u8, 153u8,
							118u8, 179u8, 201u8, 255u8, 21u8, 5u8, 117u8, 53u8, 241u8, 173u8, 66u8,
							32u8, 8u8, 26u8, 176u8, 221u8, 245u8, 212u8, 13u8, 86u8, 171u8,
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
				pub account_id: transfer_to_localchain::AccountId,
				pub amount: transfer_to_localchain::Amount,
				pub transfer_id: transfer_to_localchain::TransferId,
				pub notary_id: transfer_to_localchain::NotaryId,
				pub expiration_tick: transfer_to_localchain::ExpirationTick,
			}
			pub mod transfer_to_localchain {
				use super::runtime_types;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
				pub type TransferId = ::core::primitive::u32;
				pub type NotaryId = ::core::primitive::u32;
				pub type ExpirationTick = ::core::primitive::u32;
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
				pub account_id: transfer_to_localchain_expired::AccountId,
				pub transfer_id: transfer_to_localchain_expired::TransferId,
				pub notary_id: transfer_to_localchain_expired::NotaryId,
			}
			pub mod transfer_to_localchain_expired {
				use super::runtime_types;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type TransferId = ::core::primitive::u32;
				pub type NotaryId = ::core::primitive::u32;
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
				pub account_id: transfer_in::AccountId,
				pub amount: transfer_in::Amount,
				pub notary_id: transfer_in::NotaryId,
			}
			pub mod transfer_in {
				use super::runtime_types;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
				pub type NotaryId = ::core::primitive::u32;
			}
			impl ::subxt::events::StaticEvent for TransferIn {
				const PALLET: &'static str = "ChainTransfer";
				const EVENT: &'static str = "TransferIn";
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
			#[doc = "A transfer into the mainchain failed"]
			pub struct TransferInError {
				pub account_id: transfer_in_error::AccountId,
				pub amount: transfer_in_error::Amount,
				pub notary_id: transfer_in_error::NotaryId,
				pub notebook_number: transfer_in_error::NotebookNumber,
				pub error: transfer_in_error::Error,
			}
			pub mod transfer_in_error {
				use super::runtime_types;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
				pub type NotaryId = ::core::primitive::u32;
				pub type NotebookNumber = ::core::primitive::u32;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for TransferInError {
				const PALLET: &'static str = "ChainTransfer";
				const EVENT: &'static str = "TransferInError";
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
			#[doc = "An expired transfer to localchain failed to be refunded"]
			pub struct TransferToLocalchainRefundError {
				pub account_id: transfer_to_localchain_refund_error::AccountId,
				pub transfer_id: transfer_to_localchain_refund_error::TransferId,
				pub notary_id: transfer_to_localchain_refund_error::NotaryId,
				pub notebook_number: transfer_to_localchain_refund_error::NotebookNumber,
				pub error: transfer_to_localchain_refund_error::Error,
			}
			pub mod transfer_to_localchain_refund_error {
				use super::runtime_types;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type TransferId = ::core::primitive::u32;
				pub type NotaryId = ::core::primitive::u32;
				pub type NotebookNumber = ::core::primitive::u32;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for TransferToLocalchainRefundError {
				const PALLET: &'static str = "ChainTransfer";
				const EVENT: &'static str = "TransferToLocalchainRefundError";
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
			#[doc = "A localchain transfer could not be cleaned up properly. Possible invalid transfer"]
			#[doc = "needing investigation."]
			pub struct PossibleInvalidTransferAllowed {
				pub transfer_id: possible_invalid_transfer_allowed::TransferId,
				pub notary_id: possible_invalid_transfer_allowed::NotaryId,
				pub notebook_number: possible_invalid_transfer_allowed::NotebookNumber,
			}
			pub mod possible_invalid_transfer_allowed {
				use super::runtime_types;
				pub type TransferId = ::core::primitive::u32;
				pub type NotaryId = ::core::primitive::u32;
				pub type NotebookNumber = ::core::primitive::u32;
			}
			impl ::subxt::events::StaticEvent for PossibleInvalidTransferAllowed {
				const PALLET: &'static str = "ChainTransfer";
				const EVENT: &'static str = "PossibleInvalidTransferAllowed";
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
			#[doc = "Taxation failed"]
			pub struct TaxationError {
				pub notary_id: taxation_error::NotaryId,
				pub notebook_number: taxation_error::NotebookNumber,
				pub tax: taxation_error::Tax,
				pub error: taxation_error::Error,
			}
			pub mod taxation_error {
				use super::runtime_types;
				pub type NotaryId = ::core::primitive::u32;
				pub type NotebookNumber = ::core::primitive::u32;
				pub type Tax = ::core::primitive::u128;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for TaxationError {
				const PALLET: &'static str = "ChainTransfer";
				const EVENT: &'static str = "TaxationError";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod next_transfer_id {
					use super::runtime_types;
					pub type NextTransferId = ::core::primitive::u32;
				}
				pub mod pending_transfers_out {
					use super::runtime_types;
					pub type PendingTransfersOut =
						runtime_types::pallet_chain_transfer::QueuedTransferOut<
							::subxt::utils::AccountId32,
							::core::primitive::u128,
						>;
					pub type Param0 = ::core::primitive::u32;
				}
				pub mod expiring_transfers_out_by_notary {
					use super::runtime_types;
					pub type ExpiringTransfersOutByNotary =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u32,
						>;
					pub type Param0 = ::core::primitive::u32;
					pub type Param1 = ::core::primitive::u32;
				}
				pub mod transfers_used_in_block_notebooks {
					use super::runtime_types;
					pub type TransfersUsedInBlockNotebooks =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<(
							::subxt::utils::AccountId32,
							::core::primitive::u32,
						)>;
					pub type Param0 = ::core::primitive::u32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				pub fn next_transfer_id(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::next_transfer_id::NextTransferId,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"NextTransferId",
						(),
						[
							24u8, 254u8, 76u8, 107u8, 53u8, 239u8, 9u8, 199u8, 247u8, 44u8, 22u8,
							150u8, 46u8, 130u8, 241u8, 50u8, 36u8, 76u8, 133u8, 78u8, 69u8, 43u8,
							94u8, 241u8, 60u8, 247u8, 91u8, 71u8, 248u8, 43u8, 217u8, 31u8,
						],
					)
				}
				pub fn pending_transfers_out_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::pending_transfers_out::PendingTransfersOut,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"PendingTransfersOut",
						(),
						[
							140u8, 69u8, 183u8, 224u8, 241u8, 38u8, 50u8, 205u8, 168u8, 146u8,
							113u8, 111u8, 247u8, 46u8, 126u8, 64u8, 166u8, 208u8, 70u8, 80u8,
							231u8, 104u8, 230u8, 67u8, 186u8, 147u8, 19u8, 226u8, 117u8, 177u8,
							155u8, 138u8,
						],
					)
				}
				pub fn pending_transfers_out(
					&self,
					_0: impl ::std::borrow::Borrow<types::pending_transfers_out::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::pending_transfers_out::Param0,
					>,
					types::pending_transfers_out::PendingTransfersOut,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"PendingTransfersOut",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							140u8, 69u8, 183u8, 224u8, 241u8, 38u8, 50u8, 205u8, 168u8, 146u8,
							113u8, 111u8, 247u8, 46u8, 126u8, 64u8, 166u8, 208u8, 70u8, 80u8,
							231u8, 104u8, 230u8, 67u8, 186u8, 147u8, 19u8, 226u8, 117u8, 177u8,
							155u8, 138u8,
						],
					)
				}
				pub fn expiring_transfers_out_by_notary_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::expiring_transfers_out_by_notary::ExpiringTransfersOutByNotary,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"ExpiringTransfersOutByNotary",
						(),
						[
							71u8, 202u8, 250u8, 213u8, 80u8, 177u8, 45u8, 234u8, 239u8, 244u8,
							48u8, 170u8, 79u8, 174u8, 219u8, 77u8, 149u8, 123u8, 98u8, 218u8,
							105u8, 105u8, 236u8, 104u8, 144u8, 237u8, 242u8, 209u8, 133u8, 16u8,
							189u8, 36u8,
						],
					)
				}
				pub fn expiring_transfers_out_by_notary_iter1(
					&self,
					_0: impl ::std::borrow::Borrow<types::expiring_transfers_out_by_notary::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::expiring_transfers_out_by_notary::Param0,
					>,
					types::expiring_transfers_out_by_notary::ExpiringTransfersOutByNotary,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"ExpiringTransfersOutByNotary",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							71u8, 202u8, 250u8, 213u8, 80u8, 177u8, 45u8, 234u8, 239u8, 244u8,
							48u8, 170u8, 79u8, 174u8, 219u8, 77u8, 149u8, 123u8, 98u8, 218u8,
							105u8, 105u8, 236u8, 104u8, 144u8, 237u8, 242u8, 209u8, 133u8, 16u8,
							189u8, 36u8,
						],
					)
				}
				pub fn expiring_transfers_out_by_notary(
					&self,
					_0: impl ::std::borrow::Borrow<types::expiring_transfers_out_by_notary::Param0>,
					_1: impl ::std::borrow::Borrow<types::expiring_transfers_out_by_notary::Param1>,
				) -> ::subxt::storage::address::Address<
					(
						::subxt::storage::address::StaticStorageKey<
							types::expiring_transfers_out_by_notary::Param0,
						>,
						::subxt::storage::address::StaticStorageKey<
							types::expiring_transfers_out_by_notary::Param1,
						>,
					),
					types::expiring_transfers_out_by_notary::ExpiringTransfersOutByNotary,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"ExpiringTransfersOutByNotary",
						(
							::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
							::subxt::storage::address::StaticStorageKey::new(_1.borrow()),
						),
						[
							71u8, 202u8, 250u8, 213u8, 80u8, 177u8, 45u8, 234u8, 239u8, 244u8,
							48u8, 170u8, 79u8, 174u8, 219u8, 77u8, 149u8, 123u8, 98u8, 218u8,
							105u8, 105u8, 236u8, 104u8, 144u8, 237u8, 242u8, 209u8, 133u8, 16u8,
							189u8, 36u8,
						],
					)
				}
				pub fn transfers_used_in_block_notebooks_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::transfers_used_in_block_notebooks::TransfersUsedInBlockNotebooks,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"TransfersUsedInBlockNotebooks",
						(),
						[
							56u8, 113u8, 70u8, 50u8, 20u8, 191u8, 57u8, 47u8, 98u8, 209u8, 251u8,
							146u8, 233u8, 41u8, 193u8, 196u8, 198u8, 195u8, 231u8, 184u8, 49u8,
							3u8, 16u8, 180u8, 218u8, 7u8, 51u8, 90u8, 220u8, 111u8, 153u8, 219u8,
						],
					)
				}
				pub fn transfers_used_in_block_notebooks(
					&self,
					_0: impl ::std::borrow::Borrow<types::transfers_used_in_block_notebooks::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::transfers_used_in_block_notebooks::Param0,
					>,
					types::transfers_used_in_block_notebooks::TransfersUsedInBlockNotebooks,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ChainTransfer",
						"TransfersUsedInBlockNotebooks",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							56u8, 113u8, 70u8, 50u8, 20u8, 191u8, 57u8, 47u8, 98u8, 209u8, 251u8,
							146u8, 233u8, 41u8, 193u8, 196u8, 198u8, 195u8, 231u8, 184u8, 49u8,
							3u8, 16u8, 180u8, 218u8, 7u8, 51u8, 90u8, 220u8, 111u8, 153u8, 219u8,
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
				#[doc = " How long a transfer should remain in storage before returning. NOTE: there is a 2 tick"]
				#[doc = " grace period where we will still allow a transfer"]
				pub fn transfer_expiration_ticks(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"ChainTransfer",
						"TransferExpirationTicks",
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
					pub vote_minimum: configure::VoteMinimum,
					pub compute_difficulty: configure::ComputeDifficulty,
				}
				pub mod configure {
					use super::runtime_types;
					pub type VoteMinimum = ::core::option::Option<::core::primitive::u128>;
					pub type ComputeDifficulty = ::core::option::Option<::core::primitive::u128>;
				}
				impl ::subxt::blocks::StaticExtrinsic for Configure {
					const PALLET: &'static str = "BlockSealSpec";
					const CALL: &'static str = "configure";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				pub fn configure(
					&self,
					vote_minimum: types::configure::VoteMinimum,
					compute_difficulty: types::configure::ComputeDifficulty,
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
				pub expected_block_votes: vote_minimum_adjusted::ExpectedBlockVotes,
				pub actual_block_votes: vote_minimum_adjusted::ActualBlockVotes,
				pub start_vote_minimum: vote_minimum_adjusted::StartVoteMinimum,
				pub new_vote_minimum: vote_minimum_adjusted::NewVoteMinimum,
			}
			pub mod vote_minimum_adjusted {
				use super::runtime_types;
				pub type ExpectedBlockVotes = ::core::primitive::u128;
				pub type ActualBlockVotes = ::core::primitive::u128;
				pub type StartVoteMinimum = ::core::primitive::u128;
				pub type NewVoteMinimum = ::core::primitive::u128;
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
				pub expected_block_time: compute_difficulty_adjusted::ExpectedBlockTime,
				pub actual_block_time: compute_difficulty_adjusted::ActualBlockTime,
				pub start_difficulty: compute_difficulty_adjusted::StartDifficulty,
				pub new_difficulty: compute_difficulty_adjusted::NewDifficulty,
			}
			pub mod compute_difficulty_adjusted {
				use super::runtime_types;
				pub type ExpectedBlockTime = ::core::primitive::u64;
				pub type ActualBlockTime = ::core::primitive::u64;
				pub type StartDifficulty = ::core::primitive::u128;
				pub type NewDifficulty = ::core::primitive::u128;
			}
			impl ::subxt::events::StaticEvent for ComputeDifficultyAdjusted {
				const PALLET: &'static str = "BlockSealSpec";
				const EVENT: &'static str = "ComputeDifficultyAdjusted";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod current_vote_minimum {
					use super::runtime_types;
					pub type CurrentVoteMinimum = ::core::primitive::u128;
				}
				pub mod current_compute_difficulty {
					use super::runtime_types;
					pub type CurrentComputeDifficulty = ::core::primitive::u128;
				}
				pub mod past_compute_block_times {
					use super::runtime_types;
					pub type PastComputeBlockTimes =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u64,
						>;
				}
				pub mod previous_block_timestamp {
					use super::runtime_types;
					pub type PreviousBlockTimestamp = ::core::primitive::u64;
				}
				pub mod temp_block_timestamp {
					use super::runtime_types;
					pub type TempBlockTimestamp = ::core::primitive::u64;
				}
				pub mod vote_minimum_history {
					use super::runtime_types;
					pub type VoteMinimumHistory =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u128,
						>;
				}
				pub mod temp_current_tick_notebooks_in_block {
					use super::runtime_types;
					pub type TempCurrentTickNotebooksInBlock =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::notary::NotaryNotebookVoteDigestDetails,
						>;
				}
				pub mod temp_block_vote_digest {
					use super::runtime_types;
					pub type TempBlockVoteDigest =
						runtime_types::ulx_primitives::digests::BlockVoteDigest;
				}
				pub mod past_block_votes {
					use super::runtime_types;
					pub type PastBlockVotes =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<(
							::core::primitive::u32,
							::core::primitive::u32,
							::core::primitive::u128,
						)>;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The current vote minimum of the chain. Block votes use this minimum to determine the"]
				#[doc = " minimum amount of tax or compute needed to create a vote. It is adjusted up or down to"]
				#[doc = " target a max number of votes"]
				pub fn current_vote_minimum(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::current_vote_minimum::CurrentVoteMinimum,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"CurrentVoteMinimum",
						(),
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
					(),
					types::current_compute_difficulty::CurrentComputeDifficulty,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"CurrentComputeDifficulty",
						(),
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
					(),
					types::past_compute_block_times::PastComputeBlockTimes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"PastComputeBlockTimes",
						(),
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
					(),
					types::previous_block_timestamp::PreviousBlockTimestamp,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"PreviousBlockTimestamp",
						(),
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
					(),
					types::temp_block_timestamp::TempBlockTimestamp,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"TempBlockTimestamp",
						(),
						[
							167u8, 201u8, 179u8, 72u8, 25u8, 20u8, 159u8, 162u8, 18u8, 154u8,
							169u8, 53u8, 137u8, 227u8, 96u8, 187u8, 3u8, 133u8, 155u8, 31u8, 92u8,
							145u8, 254u8, 239u8, 86u8, 215u8, 65u8, 223u8, 91u8, 120u8, 79u8, 34u8,
						],
					)
				}
				#[doc = " Keeps the last 3 vote minimums. The first one applies to the current block."]
				pub fn vote_minimum_history(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::vote_minimum_history::VoteMinimumHistory,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"VoteMinimumHistory",
						(),
						[
							197u8, 183u8, 228u8, 59u8, 233u8, 183u8, 83u8, 132u8, 64u8, 76u8,
							112u8, 118u8, 156u8, 127u8, 114u8, 2u8, 189u8, 14u8, 255u8, 83u8,
							185u8, 11u8, 100u8, 71u8, 52u8, 7u8, 102u8, 205u8, 208u8, 103u8, 12u8,
							206u8,
						],
					)
				}
				#[doc = " Temporary store of any current tick notebooks included in this block (vs tick)"]
				pub fn temp_current_tick_notebooks_in_block(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::temp_current_tick_notebooks_in_block::TempCurrentTickNotebooksInBlock,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"TempCurrentTickNotebooksInBlock",
						(),
						[
							44u8, 17u8, 131u8, 64u8, 117u8, 10u8, 84u8, 129u8, 184u8, 227u8, 180u8,
							61u8, 162u8, 160u8, 189u8, 249u8, 202u8, 103u8, 51u8, 254u8, 97u8,
							218u8, 234u8, 192u8, 64u8, 146u8, 10u8, 174u8, 101u8, 110u8, 234u8,
							142u8,
						],
					)
				}
				#[doc = " Temporary store the vote digest"]
				pub fn temp_block_vote_digest(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::temp_block_vote_digest::TempBlockVoteDigest,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"TempBlockVoteDigest",
						(),
						[
							148u8, 195u8, 42u8, 184u8, 217u8, 89u8, 137u8, 65u8, 230u8, 188u8,
							58u8, 194u8, 7u8, 147u8, 16u8, 160u8, 78u8, 186u8, 242u8, 66u8, 159u8,
							43u8, 61u8, 250u8, 181u8, 78u8, 188u8, 53u8, 159u8, 38u8, 210u8, 38u8,
						],
					)
				}
				pub fn past_block_votes(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::past_block_votes::PastBlockVotes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSealSpec",
						"PastBlockVotes",
						(),
						[
							96u8, 31u8, 172u8, 50u8, 227u8, 32u8, 171u8, 95u8, 14u8, 206u8, 31u8,
							192u8, 30u8, 75u8, 199u8, 111u8, 243u8, 142u8, 194u8, 59u8, 101u8, 4u8,
							207u8, 52u8, 6u8, 131u8, 130u8, 83u8, 227u8, 80u8, 149u8, 168u8,
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
						"BlockSealSpec",
						"MaxActiveNotaries",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
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
	pub mod data_domain {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_data_domain::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_data_domain::pallet::Call;
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
				pub struct SetZoneRecord {
					pub domain_hash: set_zone_record::DomainHash,
					pub zone_record: set_zone_record::ZoneRecord,
				}
				pub mod set_zone_record {
					use super::runtime_types;
					pub type DomainHash = ::sp_core::H256;
					pub type ZoneRecord = runtime_types::ulx_primitives::data_domain::ZoneRecord<
						::subxt::utils::AccountId32,
					>;
				}
				impl ::subxt::blocks::StaticExtrinsic for SetZoneRecord {
					const PALLET: &'static str = "DataDomain";
					const CALL: &'static str = "set_zone_record";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				pub fn set_zone_record(
					&self,
					domain_hash: types::set_zone_record::DomainHash,
					zone_record: types::set_zone_record::ZoneRecord,
				) -> ::subxt::tx::Payload<types::SetZoneRecord> {
					::subxt::tx::Payload::new_static(
						"DataDomain",
						"set_zone_record",
						types::SetZoneRecord { domain_hash, zone_record },
						[
							50u8, 248u8, 228u8, 101u8, 86u8, 211u8, 24u8, 101u8, 200u8, 6u8, 34u8,
							171u8, 25u8, 68u8, 62u8, 191u8, 115u8, 156u8, 137u8, 190u8, 64u8,
							140u8, 151u8, 39u8, 251u8, 54u8, 169u8, 124u8, 49u8, 255u8, 131u8,
							81u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_data_domain::pallet::Event;
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
			#[doc = "A data domain zone record was updated"]
			pub struct ZoneRecordUpdated {
				pub domain_hash: zone_record_updated::DomainHash,
				pub zone_record: zone_record_updated::ZoneRecord,
			}
			pub mod zone_record_updated {
				use super::runtime_types;
				pub type DomainHash = ::sp_core::H256;
				pub type ZoneRecord = runtime_types::ulx_primitives::data_domain::ZoneRecord<
					::subxt::utils::AccountId32,
				>;
			}
			impl ::subxt::events::StaticEvent for ZoneRecordUpdated {
				const PALLET: &'static str = "DataDomain";
				const EVENT: &'static str = "ZoneRecordUpdated";
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
			#[doc = "A data domain was registered"]
			pub struct DataDomainRegistered {
				pub domain_hash: data_domain_registered::DomainHash,
				pub registration: data_domain_registered::Registration,
			}
			pub mod data_domain_registered {
				use super::runtime_types;
				pub type DomainHash = ::sp_core::H256;
				pub type Registration = runtime_types::pallet_data_domain::DataDomainRegistration<
					::subxt::utils::AccountId32,
				>;
			}
			impl ::subxt::events::StaticEvent for DataDomainRegistered {
				const PALLET: &'static str = "DataDomain";
				const EVENT: &'static str = "DataDomainRegistered";
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
			#[doc = "A data domain was registered"]
			pub struct DataDomainRenewed {
				pub domain_hash: data_domain_renewed::DomainHash,
			}
			pub mod data_domain_renewed {
				use super::runtime_types;
				pub type DomainHash = ::sp_core::H256;
			}
			impl ::subxt::events::StaticEvent for DataDomainRenewed {
				const PALLET: &'static str = "DataDomain";
				const EVENT: &'static str = "DataDomainRenewed";
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
			#[doc = "A data domain was expired"]
			pub struct DataDomainExpired {
				pub domain_hash: data_domain_expired::DomainHash,
			}
			pub mod data_domain_expired {
				use super::runtime_types;
				pub type DomainHash = ::sp_core::H256;
			}
			impl ::subxt::events::StaticEvent for DataDomainExpired {
				const PALLET: &'static str = "DataDomain";
				const EVENT: &'static str = "DataDomainExpired";
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
			#[doc = "A data domain registration was canceled due to a conflicting registration in the same"]
			#[doc = "tick"]
			pub struct DataDomainRegistrationCanceled {
				pub domain_hash: data_domain_registration_canceled::DomainHash,
				pub registration: data_domain_registration_canceled::Registration,
			}
			pub mod data_domain_registration_canceled {
				use super::runtime_types;
				pub type DomainHash = ::sp_core::H256;
				pub type Registration = runtime_types::pallet_data_domain::DataDomainRegistration<
					::subxt::utils::AccountId32,
				>;
			}
			impl ::subxt::events::StaticEvent for DataDomainRegistrationCanceled {
				const PALLET: &'static str = "DataDomain";
				const EVENT: &'static str = "DataDomainRegistrationCanceled";
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
			#[doc = "A data domain registration failed due to an error"]
			pub struct DataDomainRegistrationError {
				pub domain_hash: data_domain_registration_error::DomainHash,
				pub account_id: data_domain_registration_error::AccountId,
				pub error: data_domain_registration_error::Error,
			}
			pub mod data_domain_registration_error {
				use super::runtime_types;
				pub type DomainHash = ::sp_core::H256;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for DataDomainRegistrationError {
				const PALLET: &'static str = "DataDomain";
				const EVENT: &'static str = "DataDomainRegistrationError";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod registered_data_domains {
					use super::runtime_types;
					pub type RegisteredDataDomains =
						runtime_types::pallet_data_domain::DataDomainRegistration<
							::subxt::utils::AccountId32,
						>;
					pub type Param0 = ::sp_core::H256;
				}
				pub mod zone_records_by_domain {
					use super::runtime_types;
					pub type ZoneRecordsByDomain =
						runtime_types::ulx_primitives::data_domain::ZoneRecord<
							::subxt::utils::AccountId32,
						>;
					pub type Param0 = ::sp_core::H256;
				}
				pub mod domain_payment_address_history {
					use super::runtime_types;
					pub type DomainPaymentAddressHistory =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<(
							::subxt::utils::AccountId32,
							::core::primitive::u32,
						)>;
					pub type Param0 = ::sp_core::H256;
				}
				pub mod expiring_domains_by_block {
					use super::runtime_types;
					pub type ExpiringDomainsByBlock =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::sp_core::H256,
						>;
					pub type Param0 = ::core::primitive::u32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				pub fn registered_data_domains_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::registered_data_domains::RegisteredDataDomains,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"DataDomain",
						"RegisteredDataDomains",
						(),
						[
							170u8, 223u8, 96u8, 217u8, 109u8, 173u8, 60u8, 75u8, 121u8, 255u8,
							183u8, 225u8, 40u8, 141u8, 154u8, 7u8, 236u8, 133u8, 75u8, 191u8,
							127u8, 226u8, 48u8, 207u8, 31u8, 90u8, 194u8, 75u8, 100u8, 165u8,
							146u8, 28u8,
						],
					)
				}
				pub fn registered_data_domains(
					&self,
					_0: impl ::std::borrow::Borrow<types::registered_data_domains::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::registered_data_domains::Param0,
					>,
					types::registered_data_domains::RegisteredDataDomains,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"DataDomain",
						"RegisteredDataDomains",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							170u8, 223u8, 96u8, 217u8, 109u8, 173u8, 60u8, 75u8, 121u8, 255u8,
							183u8, 225u8, 40u8, 141u8, 154u8, 7u8, 236u8, 133u8, 75u8, 191u8,
							127u8, 226u8, 48u8, 207u8, 31u8, 90u8, 194u8, 75u8, 100u8, 165u8,
							146u8, 28u8,
						],
					)
				}
				pub fn zone_records_by_domain_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::zone_records_by_domain::ZoneRecordsByDomain,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"DataDomain",
						"ZoneRecordsByDomain",
						(),
						[
							89u8, 9u8, 74u8, 149u8, 114u8, 123u8, 168u8, 120u8, 229u8, 160u8,
							117u8, 192u8, 175u8, 47u8, 173u8, 145u8, 255u8, 91u8, 208u8, 238u8,
							99u8, 100u8, 38u8, 182u8, 219u8, 113u8, 161u8, 196u8, 186u8, 254u8,
							59u8, 213u8,
						],
					)
				}
				pub fn zone_records_by_domain(
					&self,
					_0: impl ::std::borrow::Borrow<types::zone_records_by_domain::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::zone_records_by_domain::Param0,
					>,
					types::zone_records_by_domain::ZoneRecordsByDomain,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"DataDomain",
						"ZoneRecordsByDomain",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							89u8, 9u8, 74u8, 149u8, 114u8, 123u8, 168u8, 120u8, 229u8, 160u8,
							117u8, 192u8, 175u8, 47u8, 173u8, 145u8, 255u8, 91u8, 208u8, 238u8,
							99u8, 100u8, 38u8, 182u8, 219u8, 113u8, 161u8, 196u8, 186u8, 254u8,
							59u8, 213u8,
						],
					)
				}
				pub fn domain_payment_address_history_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::domain_payment_address_history::DomainPaymentAddressHistory,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"DataDomain",
						"DomainPaymentAddressHistory",
						(),
						[
							139u8, 10u8, 161u8, 228u8, 6u8, 142u8, 223u8, 232u8, 177u8, 237u8,
							179u8, 75u8, 61u8, 224u8, 169u8, 114u8, 150u8, 245u8, 215u8, 214u8,
							246u8, 127u8, 45u8, 51u8, 246u8, 216u8, 217u8, 59u8, 232u8, 104u8,
							124u8, 82u8,
						],
					)
				}
				pub fn domain_payment_address_history(
					&self,
					_0: impl ::std::borrow::Borrow<types::domain_payment_address_history::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::domain_payment_address_history::Param0,
					>,
					types::domain_payment_address_history::DomainPaymentAddressHistory,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"DataDomain",
						"DomainPaymentAddressHistory",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							139u8, 10u8, 161u8, 228u8, 6u8, 142u8, 223u8, 232u8, 177u8, 237u8,
							179u8, 75u8, 61u8, 224u8, 169u8, 114u8, 150u8, 245u8, 215u8, 214u8,
							246u8, 127u8, 45u8, 51u8, 246u8, 216u8, 217u8, 59u8, 232u8, 104u8,
							124u8, 82u8,
						],
					)
				}
				pub fn expiring_domains_by_block_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::expiring_domains_by_block::ExpiringDomainsByBlock,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"DataDomain",
						"ExpiringDomainsByBlock",
						(),
						[
							75u8, 15u8, 133u8, 11u8, 204u8, 248u8, 72u8, 80u8, 4u8, 5u8, 0u8,
							168u8, 130u8, 36u8, 43u8, 246u8, 211u8, 66u8, 249u8, 52u8, 60u8, 67u8,
							113u8, 130u8, 240u8, 148u8, 245u8, 99u8, 98u8, 203u8, 12u8, 116u8,
						],
					)
				}
				pub fn expiring_domains_by_block(
					&self,
					_0: impl ::std::borrow::Borrow<types::expiring_domains_by_block::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::expiring_domains_by_block::Param0,
					>,
					types::expiring_domains_by_block::ExpiringDomainsByBlock,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"DataDomain",
						"ExpiringDomainsByBlock",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							75u8, 15u8, 133u8, 11u8, 204u8, 248u8, 72u8, 80u8, 4u8, 5u8, 0u8,
							168u8, 130u8, 36u8, 43u8, 246u8, 211u8, 66u8, 249u8, 52u8, 60u8, 67u8,
							113u8, 130u8, 240u8, 148u8, 245u8, 99u8, 98u8, 203u8, 12u8, 116u8,
						],
					)
				}
			}
		}
	}
	pub mod price_index {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_price_index::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_price_index::pallet::Call;
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
				#[doc = "Submit the latest price index. Only valid for the configured operator account"]
				pub struct Submit {
					pub index: submit::Index,
				}
				pub mod submit {
					use super::runtime_types;
					pub type Index = runtime_types::pallet_price_index::PriceIndex;
				}
				impl ::subxt::blocks::StaticExtrinsic for Submit {
					const PALLET: &'static str = "PriceIndex";
					const CALL: &'static str = "submit";
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
				#[doc = "Sets the operator account id (only executable by the Root account)"]
				#[doc = ""]
				#[doc = "# Arguments"]
				#[doc = "* `account_id` - the account id of the operator"]
				pub struct SetOperator {
					pub account_id: set_operator::AccountId,
				}
				pub mod set_operator {
					use super::runtime_types;
					pub type AccountId = ::subxt::utils::AccountId32;
				}
				impl ::subxt::blocks::StaticExtrinsic for SetOperator {
					const PALLET: &'static str = "PriceIndex";
					const CALL: &'static str = "set_operator";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Submit the latest price index. Only valid for the configured operator account"]
				pub fn submit(
					&self,
					index: types::submit::Index,
				) -> ::subxt::tx::Payload<types::Submit> {
					::subxt::tx::Payload::new_static(
						"PriceIndex",
						"submit",
						types::Submit { index },
						[
							94u8, 206u8, 248u8, 131u8, 143u8, 26u8, 189u8, 238u8, 151u8, 78u8,
							199u8, 145u8, 127u8, 63u8, 30u8, 175u8, 198u8, 197u8, 104u8, 233u8,
							136u8, 107u8, 51u8, 152u8, 204u8, 139u8, 234u8, 143u8, 37u8, 113u8,
							224u8, 28u8,
						],
					)
				}
				#[doc = "Sets the operator account id (only executable by the Root account)"]
				#[doc = ""]
				#[doc = "# Arguments"]
				#[doc = "* `account_id` - the account id of the operator"]
				pub fn set_operator(
					&self,
					account_id: types::set_operator::AccountId,
				) -> ::subxt::tx::Payload<types::SetOperator> {
					::subxt::tx::Payload::new_static(
						"PriceIndex",
						"set_operator",
						types::SetOperator { account_id },
						[
							160u8, 195u8, 42u8, 151u8, 18u8, 138u8, 64u8, 248u8, 118u8, 157u8,
							178u8, 120u8, 23u8, 254u8, 8u8, 157u8, 220u8, 244u8, 50u8, 65u8, 219u8,
							177u8, 56u8, 216u8, 58u8, 76u8, 168u8, 143u8, 16u8, 155u8, 250u8, 21u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_price_index::pallet::Event;
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
			#[doc = "Event emitted when a new price index is submitted"]
			pub struct NewIndex;
			impl ::subxt::events::StaticEvent for NewIndex {
				const PALLET: &'static str = "PriceIndex";
				const EVENT: &'static str = "NewIndex";
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
			pub struct OperatorChanged {
				pub operator_id: operator_changed::OperatorId,
			}
			pub mod operator_changed {
				use super::runtime_types;
				pub type OperatorId = ::subxt::utils::AccountId32;
			}
			impl ::subxt::events::StaticEvent for OperatorChanged {
				const PALLET: &'static str = "PriceIndex";
				const EVENT: &'static str = "OperatorChanged";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod current {
					use super::runtime_types;
					pub type Current = runtime_types::pallet_price_index::PriceIndex;
				}
				pub mod operator {
					use super::runtime_types;
					pub type Operator = ::subxt::utils::AccountId32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Stores the active price index"]
				pub fn current(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::current::Current,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"PriceIndex",
						"Current",
						(),
						[
							24u8, 100u8, 2u8, 213u8, 216u8, 27u8, 132u8, 28u8, 34u8, 22u8, 106u8,
							108u8, 248u8, 161u8, 103u8, 63u8, 82u8, 230u8, 205u8, 44u8, 159u8,
							38u8, 222u8, 0u8, 8u8, 248u8, 208u8, 161u8, 101u8, 179u8, 132u8, 36u8,
						],
					)
				}
				#[doc = " The price index operator account"]
				pub fn operator(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::operator::Operator,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"PriceIndex",
						"Operator",
						(),
						[
							152u8, 164u8, 78u8, 48u8, 12u8, 43u8, 174u8, 179u8, 169u8, 249u8,
							152u8, 14u8, 233u8, 30u8, 112u8, 171u8, 78u8, 85u8, 4u8, 212u8, 122u8,
							243u8, 113u8, 8u8, 231u8, 105u8, 183u8, 70u8, 147u8, 227u8, 225u8,
							44u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The maximum number of ticks to preserve a price index"]
				pub fn max_downtime_ticks_before_reset(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"PriceIndex",
						"MaxDowntimeTicksBeforeReset",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The oldest history to keep"]
				pub fn max_price_age_in_ticks(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"PriceIndex",
						"MaxPriceAgeInTicks",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The max price difference dropping below target or raising above target per tick. There's"]
				#[doc = " no corresponding constant for time to recovery to target"]
				pub fn max_argon_change_per_tick_away_from_target(
					&self,
				) -> ::subxt::constants::Address<runtime_types::sp_arithmetic::fixed_point::FixedU128>
				{
					::subxt::constants::Address::new_static(
						"PriceIndex",
						"MaxArgonChangePerTickAwayFromTarget",
						[
							62u8, 145u8, 102u8, 227u8, 159u8, 92u8, 27u8, 54u8, 159u8, 228u8,
							193u8, 99u8, 75u8, 196u8, 26u8, 250u8, 229u8, 230u8, 88u8, 109u8,
							246u8, 100u8, 152u8, 158u8, 14u8, 25u8, 224u8, 173u8, 224u8, 41u8,
							105u8, 231u8,
						],
					)
				}
				pub fn max_argon_target_change_per_tick(
					&self,
				) -> ::subxt::constants::Address<runtime_types::sp_arithmetic::fixed_point::FixedU128>
				{
					::subxt::constants::Address::new_static(
						"PriceIndex",
						"MaxArgonTargetChangePerTick",
						[
							62u8, 145u8, 102u8, 227u8, 159u8, 92u8, 27u8, 54u8, 159u8, 228u8,
							193u8, 99u8, 75u8, 196u8, 26u8, 250u8, 229u8, 230u8, 88u8, 109u8,
							246u8, 100u8, 152u8, 158u8, 14u8, 25u8, 224u8, 173u8, 224u8, 41u8,
							105u8, 231u8,
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
			pub mod types {
				use super::runtime_types;
				pub mod author {
					use super::runtime_types;
					pub type Author = ::subxt::utils::AccountId32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Author of current block."]
				pub fn author(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::author::Author,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Authorship",
						"Author",
						(),
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
			pub mod types {
				use super::runtime_types;
				pub mod historical_sessions {
					use super::runtime_types;
					pub type HistoricalSessions = (::sp_core::H256, ::core::primitive::u32);
					pub type Param0 = ::core::primitive::u32;
				}
				pub mod stored_range {
					use super::runtime_types;
					pub type StoredRange = (::core::primitive::u32, ::core::primitive::u32);
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Mapping from historical session indices to session-data root hash and validator count."]
				pub fn historical_sessions_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::historical_sessions::HistoricalSessions,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Historical",
						"HistoricalSessions",
						(),
						[
							9u8, 138u8, 247u8, 141u8, 178u8, 146u8, 124u8, 81u8, 162u8, 211u8,
							205u8, 149u8, 222u8, 254u8, 253u8, 188u8, 170u8, 242u8, 218u8, 41u8,
							124u8, 178u8, 109u8, 209u8, 163u8, 125u8, 225u8, 206u8, 249u8, 175u8,
							117u8, 75u8,
						],
					)
				}
				#[doc = " Mapping from historical session indices to session-data root hash and validator count."]
				pub fn historical_sessions(
					&self,
					_0: impl ::std::borrow::Borrow<types::historical_sessions::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::historical_sessions::Param0>,
					types::historical_sessions::HistoricalSessions,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Historical",
						"HistoricalSessions",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
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
					(),
					types::stored_range::StoredRange,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Historical",
						"StoredRange",
						(),
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
				#[doc = "Sets the session key(s) of the function caller to `keys`."]
				#[doc = "Allows an account to set its session key prior to becoming a validator."]
				#[doc = "This doesn't take effect until the next session."]
				#[doc = ""]
				#[doc = "The dispatch origin of this function must be signed."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(1)`. Actual cost depends on the number of length of `T::Keys::key_ids()` which is"]
				#[doc = "  fixed."]
				pub struct SetKeys {
					pub keys: set_keys::Keys,
					pub proof: set_keys::Proof,
				}
				pub mod set_keys {
					use super::runtime_types;
					pub type Keys = runtime_types::ulx_node_runtime::opaque::SessionKeys;
					pub type Proof = ::std::vec::Vec<::core::primitive::u8>;
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
				#[doc = "Removes any session key(s) of the function caller."]
				#[doc = ""]
				#[doc = "This doesn't take effect until the next session."]
				#[doc = ""]
				#[doc = "The dispatch origin of this function must be Signed and the account must be either be"]
				#[doc = "convertible to a validator ID using the chain's typical addressing system (this usually"]
				#[doc = "means being a controller account) or directly convertible into a validator ID (which"]
				#[doc = "usually means being a stash account)."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(1)` in number of key types. Actual cost depends on the number of length of"]
				#[doc = "  `T::Keys::key_ids()` which is fixed."]
				pub struct PurgeKeys;
				impl ::subxt::blocks::StaticExtrinsic for PurgeKeys {
					const PALLET: &'static str = "Session";
					const CALL: &'static str = "purge_keys";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Sets the session key(s) of the function caller to `keys`."]
				#[doc = "Allows an account to set its session key prior to becoming a validator."]
				#[doc = "This doesn't take effect until the next session."]
				#[doc = ""]
				#[doc = "The dispatch origin of this function must be signed."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(1)`. Actual cost depends on the number of length of `T::Keys::key_ids()` which is"]
				#[doc = "  fixed."]
				pub fn set_keys(
					&self,
					keys: types::set_keys::Keys,
					proof: types::set_keys::Proof,
				) -> ::subxt::tx::Payload<types::SetKeys> {
					::subxt::tx::Payload::new_static(
						"Session",
						"set_keys",
						types::SetKeys { keys, proof },
						[
							152u8, 217u8, 176u8, 109u8, 66u8, 122u8, 90u8, 250u8, 1u8, 124u8,
							215u8, 216u8, 118u8, 182u8, 223u8, 236u8, 96u8, 202u8, 80u8, 211u8,
							36u8, 242u8, 64u8, 65u8, 116u8, 9u8, 178u8, 35u8, 202u8, 98u8, 151u8,
							65u8,
						],
					)
				}
				#[doc = "Removes any session key(s) of the function caller."]
				#[doc = ""]
				#[doc = "This doesn't take effect until the next session."]
				#[doc = ""]
				#[doc = "The dispatch origin of this function must be Signed and the account must be either be"]
				#[doc = "convertible to a validator ID using the chain's typical addressing system (this usually"]
				#[doc = "means being a controller account) or directly convertible into a validator ID (which"]
				#[doc = "usually means being a stash account)."]
				#[doc = ""]
				#[doc = "## Complexity"]
				#[doc = "- `O(1)` in number of key types. Actual cost depends on the number of length of"]
				#[doc = "  `T::Keys::key_ids()` which is fixed."]
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
				:: subxt :: ext :: codec :: Decode,
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
				pub session_index: new_session::SessionIndex,
			}
			pub mod new_session {
				use super::runtime_types;
				pub type SessionIndex = ::core::primitive::u32;
			}
			impl ::subxt::events::StaticEvent for NewSession {
				const PALLET: &'static str = "Session";
				const EVENT: &'static str = "NewSession";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod validators {
					use super::runtime_types;
					pub type Validators = ::std::vec::Vec<::subxt::utils::AccountId32>;
				}
				pub mod current_index {
					use super::runtime_types;
					pub type CurrentIndex = ::core::primitive::u32;
				}
				pub mod queued_changed {
					use super::runtime_types;
					pub type QueuedChanged = ::core::primitive::bool;
				}
				pub mod queued_keys {
					use super::runtime_types;
					pub type QueuedKeys = ::std::vec::Vec<(
						::subxt::utils::AccountId32,
						runtime_types::ulx_node_runtime::opaque::SessionKeys,
					)>;
				}
				pub mod disabled_validators {
					use super::runtime_types;
					pub type DisabledValidators = ::std::vec::Vec<::core::primitive::u32>;
				}
				pub mod next_keys {
					use super::runtime_types;
					pub type NextKeys = runtime_types::ulx_node_runtime::opaque::SessionKeys;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod key_owner {
					use super::runtime_types;
					pub type KeyOwner = ::subxt::utils::AccountId32;
					pub type Param0 = runtime_types::sp_core::crypto::KeyTypeId;
					pub type Param1 = [::core::primitive::u8];
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The current set of validators."]
				pub fn validators(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::validators::Validators,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"Validators",
						(),
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
					(),
					types::current_index::CurrentIndex,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"CurrentIndex",
						(),
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
					(),
					types::queued_changed::QueuedChanged,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"QueuedChanged",
						(),
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
					(),
					types::queued_keys::QueuedKeys,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"QueuedKeys",
						(),
						[
							184u8, 60u8, 226u8, 158u8, 118u8, 58u8, 50u8, 111u8, 207u8, 71u8,
							206u8, 234u8, 200u8, 44u8, 199u8, 184u8, 229u8, 70u8, 32u8, 199u8,
							202u8, 46u8, 136u8, 31u8, 181u8, 146u8, 22u8, 226u8, 216u8, 20u8,
							214u8, 253u8,
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
					(),
					types::disabled_validators::DisabledValidators,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"DisabledValidators",
						(),
						[
							213u8, 19u8, 168u8, 234u8, 187u8, 200u8, 180u8, 97u8, 234u8, 189u8,
							36u8, 233u8, 158u8, 184u8, 45u8, 35u8, 129u8, 213u8, 133u8, 8u8, 104u8,
							183u8, 46u8, 68u8, 154u8, 240u8, 132u8, 22u8, 247u8, 11u8, 54u8, 221u8,
						],
					)
				}
				#[doc = " The next session keys for a validator."]
				pub fn next_keys_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::next_keys::NextKeys,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"NextKeys",
						(),
						[
							78u8, 250u8, 134u8, 66u8, 119u8, 121u8, 215u8, 245u8, 120u8, 34u8,
							46u8, 97u8, 251u8, 104u8, 7u8, 233u8, 153u8, 10u8, 6u8, 169u8, 23u8,
							152u8, 62u8, 22u8, 137u8, 3u8, 222u8, 144u8, 201u8, 56u8, 179u8, 87u8,
						],
					)
				}
				#[doc = " The next session keys for a validator."]
				pub fn next_keys(
					&self,
					_0: impl ::std::borrow::Borrow<types::next_keys::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::next_keys::Param0>,
					types::next_keys::NextKeys,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"NextKeys",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							78u8, 250u8, 134u8, 66u8, 119u8, 121u8, 215u8, 245u8, 120u8, 34u8,
							46u8, 97u8, 251u8, 104u8, 7u8, 233u8, 153u8, 10u8, 6u8, 169u8, 23u8,
							152u8, 62u8, 22u8, 137u8, 3u8, 222u8, 144u8, 201u8, 56u8, 179u8, 87u8,
						],
					)
				}
				#[doc = " The owner of a key. The key is the `KeyTypeId` + the encoded key."]
				pub fn key_owner_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::key_owner::KeyOwner,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"KeyOwner",
						(),
						[
							217u8, 204u8, 21u8, 114u8, 247u8, 129u8, 32u8, 242u8, 93u8, 91u8,
							253u8, 253u8, 248u8, 90u8, 12u8, 202u8, 195u8, 25u8, 18u8, 100u8,
							253u8, 109u8, 88u8, 77u8, 217u8, 140u8, 51u8, 40u8, 118u8, 35u8, 107u8,
							206u8,
						],
					)
				}
				#[doc = " The owner of a key. The key is the `KeyTypeId` + the encoded key."]
				pub fn key_owner_iter1(
					&self,
					_0: impl ::std::borrow::Borrow<types::key_owner::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::key_owner::Param0>,
					types::key_owner::KeyOwner,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"KeyOwner",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							217u8, 204u8, 21u8, 114u8, 247u8, 129u8, 32u8, 242u8, 93u8, 91u8,
							253u8, 253u8, 248u8, 90u8, 12u8, 202u8, 195u8, 25u8, 18u8, 100u8,
							253u8, 109u8, 88u8, 77u8, 217u8, 140u8, 51u8, 40u8, 118u8, 35u8, 107u8,
							206u8,
						],
					)
				}
				#[doc = " The owner of a key. The key is the `KeyTypeId` + the encoded key."]
				pub fn key_owner(
					&self,
					_0: impl ::std::borrow::Borrow<types::key_owner::Param0>,
					_1: impl ::std::borrow::Borrow<types::key_owner::Param1>,
				) -> ::subxt::storage::address::Address<
					(
						::subxt::storage::address::StaticStorageKey<types::key_owner::Param0>,
						::subxt::storage::address::StaticStorageKey<types::key_owner::Param1>,
					),
					types::key_owner::KeyOwner,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"KeyOwner",
						(
							::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
							::subxt::storage::address::StaticStorageKey::new(_1.borrow()),
						),
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
					pub seal: apply::Seal,
				}
				pub mod apply {
					use super::runtime_types;
					pub type Seal = runtime_types::ulx_primitives::inherents::BlockSealInherent;
				}
				impl ::subxt::blocks::StaticExtrinsic for Apply {
					const PALLET: &'static str = "BlockSeal";
					const CALL: &'static str = "apply";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				pub fn apply(
					&self,
					seal: types::apply::Seal,
				) -> ::subxt::tx::Payload<types::Apply> {
					::subxt::tx::Payload::new_static(
						"BlockSeal",
						"apply",
						types::Apply { seal },
						[
							219u8, 28u8, 137u8, 83u8, 119u8, 114u8, 20u8, 229u8, 253u8, 233u8,
							140u8, 58u8, 32u8, 228u8, 139u8, 84u8, 9u8, 93u8, 54u8, 8u8, 96u8, 8u8,
							97u8, 25u8, 73u8, 9u8, 55u8, 175u8, 198u8, 182u8, 196u8, 85u8,
						],
					)
				}
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod last_block_sealer_info {
					use super::runtime_types;
					pub type LastBlockSealerInfo =
						runtime_types::ulx_primitives::providers::BlockSealerInfo<
							::subxt::utils::AccountId32,
						>;
				}
				pub mod parent_voting_key {
					use super::runtime_types;
					pub type ParentVotingKey = ::core::option::Option<::sp_core::H256>;
				}
				pub mod temp_author {
					use super::runtime_types;
					pub type TempAuthor = ::subxt::utils::AccountId32;
				}
				pub mod temp_seal_inherent {
					use super::runtime_types;
					pub type TempSealInherent =
						runtime_types::ulx_primitives::inherents::BlockSealInherent;
				}
				pub mod temp_voting_key_digest {
					use super::runtime_types;
					pub type TempVotingKeyDigest =
						runtime_types::ulx_primitives::digests::ParentVotingKeyDigest;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				pub fn last_block_sealer_info(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::last_block_sealer_info::LastBlockSealerInfo,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSeal",
						"LastBlockSealerInfo",
						(),
						[
							59u8, 15u8, 90u8, 79u8, 226u8, 1u8, 28u8, 174u8, 251u8, 138u8, 15u8,
							104u8, 204u8, 172u8, 75u8, 34u8, 100u8, 24u8, 205u8, 13u8, 24u8, 6u8,
							224u8, 243u8, 41u8, 185u8, 186u8, 164u8, 126u8, 225u8, 20u8, 81u8,
						],
					)
				}
				#[doc = " The calculated parent voting key for a block. Refers to the Notebook BlockVote Revealed"]
				#[doc = " Secret + VotesMerkleRoot of the parent block notebooks."]
				pub fn parent_voting_key(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::parent_voting_key::ParentVotingKey,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSeal",
						"ParentVotingKey",
						(),
						[
							12u8, 73u8, 52u8, 154u8, 15u8, 127u8, 150u8, 214u8, 178u8, 186u8,
							231u8, 204u8, 104u8, 196u8, 141u8, 55u8, 198u8, 11u8, 23u8, 252u8,
							108u8, 65u8, 42u8, 124u8, 77u8, 77u8, 88u8, 35u8, 154u8, 241u8, 50u8,
							216u8,
						],
					)
				}
				#[doc = " Author of current block (temporary storage)."]
				pub fn temp_author(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::temp_author::TempAuthor,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSeal",
						"TempAuthor",
						(),
						[
							29u8, 149u8, 234u8, 74u8, 206u8, 138u8, 152u8, 92u8, 28u8, 103u8, 4u8,
							236u8, 161u8, 51u8, 52u8, 196u8, 28u8, 242u8, 250u8, 210u8, 187u8,
							78u8, 217u8, 251u8, 157u8, 143u8, 91u8, 60u8, 246u8, 218u8, 227u8,
							114u8,
						],
					)
				}
				#[doc = " Ensures only a single inherent is applied"]
				pub fn temp_seal_inherent(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::temp_seal_inherent::TempSealInherent,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSeal",
						"TempSealInherent",
						(),
						[
							155u8, 198u8, 152u8, 193u8, 160u8, 65u8, 79u8, 57u8, 173u8, 165u8,
							225u8, 42u8, 102u8, 64u8, 210u8, 78u8, 243u8, 181u8, 84u8, 156u8,
							139u8, 238u8, 34u8, 129u8, 199u8, 138u8, 84u8, 136u8, 156u8, 115u8,
							10u8, 107u8,
						],
					)
				}
				#[doc = " Temporarily track the parent voting key digest"]
				pub fn temp_voting_key_digest(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::temp_voting_key_digest::TempVotingKeyDigest,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockSeal",
						"TempVotingKeyDigest",
						(),
						[
							62u8, 236u8, 18u8, 31u8, 216u8, 143u8, 7u8, 128u8, 99u8, 129u8, 168u8,
							182u8, 205u8, 207u8, 253u8, 199u8, 82u8, 185u8, 26u8, 190u8, 222u8,
							160u8, 10u8, 186u8, 150u8, 3u8, 192u8, 56u8, 145u8, 106u8, 159u8, 73u8,
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
				pub maturation_block: reward_created::MaturationBlock,
				pub rewards: reward_created::Rewards,
			}
			pub mod reward_created {
				use super::runtime_types;
				pub type MaturationBlock = ::core::primitive::u32;
				pub type Rewards = ::std::vec::Vec<
					runtime_types::ulx_primitives::block_seal::BlockPayout<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
					>,
				>;
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
				pub rewards: reward_unlocked::Rewards,
			}
			pub mod reward_unlocked {
				use super::runtime_types;
				pub type Rewards = ::std::vec::Vec<
					runtime_types::ulx_primitives::block_seal::BlockPayout<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
					>,
				>;
			}
			impl ::subxt::events::StaticEvent for RewardUnlocked {
				const PALLET: &'static str = "BlockRewards";
				const EVENT: &'static str = "RewardUnlocked";
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
			pub struct RewardUnlockError {
				pub account_id: reward_unlock_error::AccountId,
				pub argons: reward_unlock_error::Argons,
				pub ulixees: reward_unlock_error::Ulixees,
				pub error: reward_unlock_error::Error,
			}
			pub mod reward_unlock_error {
				use super::runtime_types;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type Argons = ::core::option::Option<::core::primitive::u128>;
				pub type Ulixees = ::core::option::Option<::core::primitive::u128>;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for RewardUnlockError {
				const PALLET: &'static str = "BlockRewards";
				const EVENT: &'static str = "RewardUnlockError";
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
			pub struct RewardCreateError {
				pub account_id: reward_create_error::AccountId,
				pub argons: reward_create_error::Argons,
				pub ulixees: reward_create_error::Ulixees,
				pub error: reward_create_error::Error,
			}
			pub mod reward_create_error {
				use super::runtime_types;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type Argons = ::core::option::Option<::core::primitive::u128>;
				pub type Ulixees = ::core::option::Option<::core::primitive::u128>;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for RewardCreateError {
				const PALLET: &'static str = "BlockRewards";
				const EVENT: &'static str = "RewardCreateError";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod payouts_by_block {
					use super::runtime_types;
					pub type PayoutsByBlock =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::block_seal::BlockPayout<
								::subxt::utils::AccountId32,
								::core::primitive::u128,
							>,
						>;
					pub type Param0 = ::core::primitive::u32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				pub fn payouts_by_block_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::payouts_by_block::PayoutsByBlock,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"BlockRewards",
						"PayoutsByBlock",
						(),
						[
							187u8, 146u8, 103u8, 131u8, 218u8, 195u8, 255u8, 176u8, 113u8, 61u8,
							112u8, 173u8, 15u8, 21u8, 131u8, 112u8, 96u8, 186u8, 252u8, 118u8,
							26u8, 27u8, 149u8, 237u8, 185u8, 141u8, 172u8, 27u8, 65u8, 127u8,
							206u8, 31u8,
						],
					)
				}
				pub fn payouts_by_block(
					&self,
					_0: impl ::std::borrow::Borrow<types::payouts_by_block::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::payouts_by_block::Param0>,
					types::payouts_by_block::PayoutsByBlock,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"BlockRewards",
						"PayoutsByBlock",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							187u8, 146u8, 103u8, 131u8, 218u8, 195u8, 255u8, 176u8, 113u8, 61u8,
							112u8, 173u8, 15u8, 21u8, 131u8, 112u8, 96u8, 186u8, 252u8, 118u8,
							26u8, 27u8, 149u8, 237u8, 185u8, 141u8, 172u8, 27u8, 65u8, 127u8,
							206u8, 31u8,
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
				#[doc = " Percent as a number out of 100 of the block reward that goes to the miner."]
				pub fn miner_payout_percent(
					&self,
				) -> ::subxt::constants::Address<runtime_types::sp_arithmetic::per_things::Percent>
				{
					::subxt::constants::Address::new_static(
						"BlockRewards",
						"MinerPayoutPercent",
						[
							40u8, 171u8, 69u8, 196u8, 34u8, 184u8, 50u8, 128u8, 139u8, 192u8, 63u8,
							231u8, 249u8, 200u8, 252u8, 73u8, 244u8, 170u8, 51u8, 177u8, 106u8,
							47u8, 114u8, 234u8, 84u8, 104u8, 62u8, 118u8, 227u8, 50u8, 225u8,
							122u8,
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
				#[doc = "Report voter equivocation/misbehavior. This method will verify the"]
				#[doc = "equivocation proof and validate the given key ownership proof"]
				#[doc = "against the extracted offender. If both are valid, the offence"]
				#[doc = "will be reported."]
				pub struct ReportEquivocation {
					pub equivocation_proof:
						::std::boxed::Box<report_equivocation::EquivocationProof>,
					pub key_owner_proof: report_equivocation::KeyOwnerProof,
				}
				pub mod report_equivocation {
					use super::runtime_types;
					pub type EquivocationProof =
						runtime_types::sp_consensus_grandpa::EquivocationProof<
							::sp_core::H256,
							::core::primitive::u32,
						>;
					pub type KeyOwnerProof = runtime_types::sp_session::MembershipProof;
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
				#[doc = "Report voter equivocation/misbehavior. This method will verify the"]
				#[doc = "equivocation proof and validate the given key ownership proof"]
				#[doc = "against the extracted offender. If both are valid, the offence"]
				#[doc = "will be reported."]
				#[doc = ""]
				#[doc = "This extrinsic must be called unsigned and it is expected that only"]
				#[doc = "block authors will call it (validated in `ValidateUnsigned`), as such"]
				#[doc = "if the block author is defined it will be defined as the equivocation"]
				#[doc = "reporter."]
				pub struct ReportEquivocationUnsigned {
					pub equivocation_proof:
						::std::boxed::Box<report_equivocation_unsigned::EquivocationProof>,
					pub key_owner_proof: report_equivocation_unsigned::KeyOwnerProof,
				}
				pub mod report_equivocation_unsigned {
					use super::runtime_types;
					pub type EquivocationProof =
						runtime_types::sp_consensus_grandpa::EquivocationProof<
							::sp_core::H256,
							::core::primitive::u32,
						>;
					pub type KeyOwnerProof = runtime_types::sp_session::MembershipProof;
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
				#[doc = "Note that the current authority set of the GRANDPA finality gadget has stalled."]
				#[doc = ""]
				#[doc = "This will trigger a forced authority set change at the beginning of the next session, to"]
				#[doc = "be enacted `delay` blocks after that. The `delay` should be high enough to safely assume"]
				#[doc = "that the block signalling the forced change will not be re-orged e.g. 1000 blocks."]
				#[doc = "The block production rate (which may be slowed down because of finality lagging) should"]
				#[doc = "be taken into account when choosing the `delay`. The GRANDPA voters based on the new"]
				#[doc = "authority will start voting on top of `best_finalized_block_number` for new finalized"]
				#[doc = "blocks. `best_finalized_block_number` should be the highest of the latest finalized"]
				#[doc = "block of all validators of the new authority set."]
				#[doc = ""]
				#[doc = "Only callable by root."]
				pub struct NoteStalled {
					pub delay: note_stalled::Delay,
					pub best_finalized_block_number: note_stalled::BestFinalizedBlockNumber,
				}
				pub mod note_stalled {
					use super::runtime_types;
					pub type Delay = ::core::primitive::u32;
					pub type BestFinalizedBlockNumber = ::core::primitive::u32;
				}
				impl ::subxt::blocks::StaticExtrinsic for NoteStalled {
					const PALLET: &'static str = "Grandpa";
					const CALL: &'static str = "note_stalled";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Report voter equivocation/misbehavior. This method will verify the"]
				#[doc = "equivocation proof and validate the given key ownership proof"]
				#[doc = "against the extracted offender. If both are valid, the offence"]
				#[doc = "will be reported."]
				pub fn report_equivocation(
					&self,
					equivocation_proof: types::report_equivocation::EquivocationProof,
					key_owner_proof: types::report_equivocation::KeyOwnerProof,
				) -> ::subxt::tx::Payload<types::ReportEquivocation> {
					::subxt::tx::Payload::new_static(
						"Grandpa",
						"report_equivocation",
						types::ReportEquivocation {
							equivocation_proof: ::std::boxed::Box::new(equivocation_proof),
							key_owner_proof,
						},
						[
							197u8, 206u8, 246u8, 26u8, 171u8, 25u8, 214u8, 211u8, 138u8, 132u8,
							148u8, 48u8, 66u8, 12u8, 92u8, 17u8, 190u8, 155u8, 121u8, 222u8, 226u8,
							171u8, 208u8, 123u8, 253u8, 247u8, 253u8, 191u8, 90u8, 4u8, 224u8,
							104u8,
						],
					)
				}
				#[doc = "Report voter equivocation/misbehavior. This method will verify the"]
				#[doc = "equivocation proof and validate the given key ownership proof"]
				#[doc = "against the extracted offender. If both are valid, the offence"]
				#[doc = "will be reported."]
				#[doc = ""]
				#[doc = "This extrinsic must be called unsigned and it is expected that only"]
				#[doc = "block authors will call it (validated in `ValidateUnsigned`), as such"]
				#[doc = "if the block author is defined it will be defined as the equivocation"]
				#[doc = "reporter."]
				pub fn report_equivocation_unsigned(
					&self,
					equivocation_proof: types::report_equivocation_unsigned::EquivocationProof,
					key_owner_proof: types::report_equivocation_unsigned::KeyOwnerProof,
				) -> ::subxt::tx::Payload<types::ReportEquivocationUnsigned> {
					::subxt::tx::Payload::new_static(
						"Grandpa",
						"report_equivocation_unsigned",
						types::ReportEquivocationUnsigned {
							equivocation_proof: ::std::boxed::Box::new(equivocation_proof),
							key_owner_proof,
						},
						[
							109u8, 97u8, 251u8, 184u8, 77u8, 61u8, 95u8, 187u8, 132u8, 146u8, 18u8,
							105u8, 109u8, 124u8, 181u8, 74u8, 143u8, 171u8, 248u8, 188u8, 69u8,
							63u8, 65u8, 92u8, 64u8, 42u8, 104u8, 131u8, 67u8, 202u8, 172u8, 73u8,
						],
					)
				}
				#[doc = "Note that the current authority set of the GRANDPA finality gadget has stalled."]
				#[doc = ""]
				#[doc = "This will trigger a forced authority set change at the beginning of the next session, to"]
				#[doc = "be enacted `delay` blocks after that. The `delay` should be high enough to safely assume"]
				#[doc = "that the block signalling the forced change will not be re-orged e.g. 1000 blocks."]
				#[doc = "The block production rate (which may be slowed down because of finality lagging) should"]
				#[doc = "be taken into account when choosing the `delay`. The GRANDPA voters based on the new"]
				#[doc = "authority will start voting on top of `best_finalized_block_number` for new finalized"]
				#[doc = "blocks. `best_finalized_block_number` should be the highest of the latest finalized"]
				#[doc = "block of all validators of the new authority set."]
				#[doc = ""]
				#[doc = "Only callable by root."]
				pub fn note_stalled(
					&self,
					delay: types::note_stalled::Delay,
					best_finalized_block_number: types::note_stalled::BestFinalizedBlockNumber,
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
				pub authority_set: new_authorities::AuthoritySet,
			}
			pub mod new_authorities {
				use super::runtime_types;
				pub type AuthoritySet = ::std::vec::Vec<(
					runtime_types::sp_consensus_grandpa::app::Public,
					::core::primitive::u64,
				)>;
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
			pub mod types {
				use super::runtime_types;
				pub mod state {
					use super::runtime_types;
					pub type State =
						runtime_types::pallet_grandpa::StoredState<::core::primitive::u32>;
				}
				pub mod pending_change {
					use super::runtime_types;
					pub type PendingChange =
						runtime_types::pallet_grandpa::StoredPendingChange<::core::primitive::u32>;
				}
				pub mod next_forced {
					use super::runtime_types;
					pub type NextForced = ::core::primitive::u32;
				}
				pub mod stalled {
					use super::runtime_types;
					pub type Stalled = (::core::primitive::u32, ::core::primitive::u32);
				}
				pub mod current_set_id {
					use super::runtime_types;
					pub type CurrentSetId = ::core::primitive::u64;
				}
				pub mod set_id_session {
					use super::runtime_types;
					pub type SetIdSession = ::core::primitive::u32;
					pub type Param0 = ::core::primitive::u64;
				}
				pub mod authorities {
					use super::runtime_types;
					pub type Authorities =
						runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<(
							runtime_types::sp_consensus_grandpa::app::Public,
							::core::primitive::u64,
						)>;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " State of the current authority set."]
				pub fn state(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::state::State,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"State",
						(),
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
					(),
					types::pending_change::PendingChange,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"PendingChange",
						(),
						[
							32u8, 165u8, 141u8, 100u8, 109u8, 66u8, 58u8, 22u8, 118u8, 84u8, 92u8,
							164u8, 119u8, 130u8, 104u8, 25u8, 244u8, 111u8, 223u8, 54u8, 184u8,
							95u8, 196u8, 30u8, 244u8, 129u8, 110u8, 127u8, 200u8, 66u8, 226u8,
							26u8,
						],
					)
				}
				#[doc = " next block number where we can force a change."]
				pub fn next_forced(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::next_forced::NextForced,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"NextForced",
						(),
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
					(),
					types::stalled::Stalled,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"Stalled",
						(),
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
					(),
					types::current_set_id::CurrentSetId,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"CurrentSetId",
						(),
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
				pub fn set_id_session_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::set_id_session::SetIdSession,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"SetIdSession",
						(),
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
				pub fn set_id_session(
					&self,
					_0: impl ::std::borrow::Borrow<types::set_id_session::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::set_id_session::Param0>,
					types::set_id_session::SetIdSession,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"SetIdSession",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
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
					(),
					types::authorities::Authorities,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Grandpa",
						"Authorities",
						(),
						[
							192u8, 157u8, 98u8, 244u8, 104u8, 38u8, 195u8, 114u8, 183u8, 62u8,
							247u8, 18u8, 31u8, 152u8, 246u8, 206u8, 97u8, 13u8, 118u8, 211u8,
							104u8, 54u8, 150u8, 152u8, 126u8, 170u8, 228u8, 158u8, 108u8, 129u8,
							134u8, 44u8,
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
				pub kind: offence::Kind,
				pub timeslot: offence::Timeslot,
			}
			pub mod offence {
				use super::runtime_types;
				pub type Kind = [::core::primitive::u8; 16usize];
				pub type Timeslot = ::std::vec::Vec<::core::primitive::u8>;
			}
			impl ::subxt::events::StaticEvent for Offence {
				const PALLET: &'static str = "Offences";
				const EVENT: &'static str = "Offence";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod reports {
					use super::runtime_types;
					pub type Reports = runtime_types::sp_staking::offence::OffenceDetails<
						::subxt::utils::AccountId32,
						(
							::subxt::utils::AccountId32,
							runtime_types::pallet_mining_slot::MinerHistory,
						),
					>;
					pub type Param0 = ::sp_core::H256;
				}
				pub mod concurrent_reports_index {
					use super::runtime_types;
					pub type ConcurrentReportsIndex = ::std::vec::Vec<::sp_core::H256>;
					pub type Param0 = [::core::primitive::u8; 16usize];
					pub type Param1 = [::core::primitive::u8];
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The primary structure that holds all offence records keyed by report identifiers."]
				pub fn reports_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::reports::Reports,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Offences",
						"Reports",
						(),
						[
							205u8, 231u8, 221u8, 1u8, 157u8, 93u8, 122u8, 97u8, 61u8, 216u8, 201u8,
							203u8, 114u8, 249u8, 113u8, 235u8, 82u8, 159u8, 25u8, 19u8, 207u8,
							108u8, 214u8, 122u8, 8u8, 1u8, 110u8, 191u8, 218u8, 248u8, 56u8, 36u8,
						],
					)
				}
				#[doc = " The primary structure that holds all offence records keyed by report identifiers."]
				pub fn reports(
					&self,
					_0: impl ::std::borrow::Borrow<types::reports::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::reports::Param0>,
					types::reports::Reports,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Offences",
						"Reports",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							205u8, 231u8, 221u8, 1u8, 157u8, 93u8, 122u8, 97u8, 61u8, 216u8, 201u8,
							203u8, 114u8, 249u8, 113u8, 235u8, 82u8, 159u8, 25u8, 19u8, 207u8,
							108u8, 214u8, 122u8, 8u8, 1u8, 110u8, 191u8, 218u8, 248u8, 56u8, 36u8,
						],
					)
				}
				#[doc = " A vector of reports of the same kind that happened at the same time slot."]
				pub fn concurrent_reports_index_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::concurrent_reports_index::ConcurrentReportsIndex,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Offences",
						"ConcurrentReportsIndex",
						(),
						[
							170u8, 186u8, 72u8, 29u8, 251u8, 38u8, 193u8, 195u8, 109u8, 86u8, 0u8,
							241u8, 20u8, 235u8, 108u8, 126u8, 215u8, 82u8, 73u8, 113u8, 199u8,
							138u8, 24u8, 58u8, 216u8, 72u8, 221u8, 232u8, 252u8, 244u8, 96u8,
							247u8,
						],
					)
				}
				#[doc = " A vector of reports of the same kind that happened at the same time slot."]
				pub fn concurrent_reports_index_iter1(
					&self,
					_0: impl ::std::borrow::Borrow<types::concurrent_reports_index::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<
						types::concurrent_reports_index::Param0,
					>,
					types::concurrent_reports_index::ConcurrentReportsIndex,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Offences",
						"ConcurrentReportsIndex",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							170u8, 186u8, 72u8, 29u8, 251u8, 38u8, 193u8, 195u8, 109u8, 86u8, 0u8,
							241u8, 20u8, 235u8, 108u8, 126u8, 215u8, 82u8, 73u8, 113u8, 199u8,
							138u8, 24u8, 58u8, 216u8, 72u8, 221u8, 232u8, 252u8, 244u8, 96u8,
							247u8,
						],
					)
				}
				#[doc = " A vector of reports of the same kind that happened at the same time slot."]
				pub fn concurrent_reports_index(
					&self,
					_0: impl ::std::borrow::Borrow<types::concurrent_reports_index::Param0>,
					_1: impl ::std::borrow::Borrow<types::concurrent_reports_index::Param1>,
				) -> ::subxt::storage::address::Address<
					(
						::subxt::storage::address::StaticStorageKey<
							types::concurrent_reports_index::Param0,
						>,
						::subxt::storage::address::StaticStorageKey<
							types::concurrent_reports_index::Param1,
						>,
					),
					types::concurrent_reports_index::ConcurrentReportsIndex,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Offences",
						"ConcurrentReportsIndex",
						(
							::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
							::subxt::storage::address::StaticStorageKey::new(_1.borrow()),
						),
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
	pub mod mint {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_mint::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_mint::pallet::Call;
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
		pub type Event = runtime_types::pallet_mint::pallet::Event;
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
			pub struct ArgonsMinted {
				pub mint_type: argons_minted::MintType,
				pub account_id: argons_minted::AccountId,
				pub utxo_id: argons_minted::UtxoId,
				pub amount: argons_minted::Amount,
			}
			pub mod argons_minted {
				use super::runtime_types;
				pub type MintType = runtime_types::pallet_mint::pallet::MintType;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type UtxoId = ::core::option::Option<::core::primitive::u64>;
				pub type Amount = ::core::primitive::u128;
			}
			impl ::subxt::events::StaticEvent for ArgonsMinted {
				const PALLET: &'static str = "Mint";
				const EVENT: &'static str = "ArgonsMinted";
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
			pub struct MintError {
				pub mint_type: mint_error::MintType,
				pub account_id: mint_error::AccountId,
				pub utxo_id: mint_error::UtxoId,
				pub amount: mint_error::Amount,
				pub error: mint_error::Error,
			}
			pub mod mint_error {
				use super::runtime_types;
				pub type MintType = runtime_types::pallet_mint::pallet::MintType;
				pub type AccountId = ::subxt::utils::AccountId32;
				pub type UtxoId = ::core::option::Option<::core::primitive::u64>;
				pub type Amount = ::core::primitive::u128;
				pub type Error = runtime_types::sp_runtime::DispatchError;
			}
			impl ::subxt::events::StaticEvent for MintError {
				const PALLET: &'static str = "Mint";
				const EVENT: &'static str = "MintError";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod pending_mint_utxos {
					use super::runtime_types;
					pub type PendingMintUtxos =
						runtime_types::bounded_collections::bounded_vec::BoundedVec<(
							::core::primitive::u64,
							::subxt::utils::AccountId32,
							::core::primitive::u128,
						)>;
				}
				pub mod minted_ulixee_argons {
					use super::runtime_types;
					pub type MintedUlixeeArgons = ::core::primitive::u128;
				}
				pub mod minted_bitcoin_argons {
					use super::runtime_types;
					pub type MintedBitcoinArgons = ::core::primitive::u128;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Bitcoin UTXOs that have been submitted for minting. This list is FIFO for minting whenever"]
				#[doc = " a) CPI >= 0 and"]
				#[doc = " b) the aggregate minted Bitcoins <= the aggregate minted Argons from Ulixee Shares"]
				pub fn pending_mint_utxos(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::pending_mint_utxos::PendingMintUtxos,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Mint",
						"PendingMintUtxos",
						(),
						[
							196u8, 160u8, 225u8, 237u8, 86u8, 164u8, 92u8, 231u8, 111u8, 145u8,
							28u8, 0u8, 109u8, 229u8, 72u8, 54u8, 178u8, 120u8, 204u8, 23u8, 98u8,
							40u8, 69u8, 158u8, 175u8, 210u8, 29u8, 172u8, 163u8, 182u8, 128u8,
							108u8,
						],
					)
				}
				pub fn minted_ulixee_argons(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::minted_ulixee_argons::MintedUlixeeArgons,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Mint",
						"MintedUlixeeArgons",
						(),
						[
							174u8, 93u8, 85u8, 65u8, 22u8, 251u8, 187u8, 150u8, 221u8, 159u8, 77u8,
							97u8, 124u8, 13u8, 164u8, 70u8, 7u8, 186u8, 69u8, 152u8, 19u8, 69u8,
							205u8, 216u8, 41u8, 84u8, 54u8, 57u8, 54u8, 146u8, 38u8, 201u8,
						],
					)
				}
				pub fn minted_bitcoin_argons(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::minted_bitcoin_argons::MintedBitcoinArgons,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Mint",
						"MintedBitcoinArgons",
						(),
						[
							120u8, 95u8, 112u8, 229u8, 112u8, 13u8, 155u8, 208u8, 133u8, 219u8,
							23u8, 36u8, 127u8, 221u8, 196u8, 20u8, 155u8, 91u8, 196u8, 50u8, 46u8,
							172u8, 232u8, 31u8, 252u8, 111u8, 111u8, 56u8, 121u8, 38u8, 241u8,
							124u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The maximum number of UTXOs that can be waiting for minting"]
				pub fn max_pending_mint_utxos(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Mint",
						"MaxPendingMintUtxos",
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
				#[doc = "Transfer some liquid free balance to another account."]
				#[doc = ""]
				#[doc = "`transfer_allow_death` will set the `FreeBalance` of the sender and receiver."]
				#[doc = "If the sender's account is below the existential deposit as a result"]
				#[doc = "of the transfer, the account will be reaped."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be `Signed` by the transactor."]
				pub struct TransferAllowDeath {
					pub dest: transfer_allow_death::Dest,
					#[codec(compact)]
					pub value: transfer_allow_death::Value,
				}
				pub mod transfer_allow_death {
					use super::runtime_types;
					pub type Dest = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Value = ::core::primitive::u128;
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
				#[doc = "Exactly as `transfer_allow_death`, except the origin must be root and the source account"]
				#[doc = "may be specified."]
				pub struct ForceTransfer {
					pub source: force_transfer::Source,
					pub dest: force_transfer::Dest,
					#[codec(compact)]
					pub value: force_transfer::Value,
				}
				pub mod force_transfer {
					use super::runtime_types;
					pub type Source = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Dest = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Value = ::core::primitive::u128;
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
				#[doc = "Same as the [`transfer_allow_death`] call, but with a check that the transfer will not"]
				#[doc = "kill the origin account."]
				#[doc = ""]
				#[doc = "99% of the time you want [`transfer_allow_death`] instead."]
				#[doc = ""]
				#[doc = "[`transfer_allow_death`]: struct.Pallet.html#method.transfer"]
				pub struct TransferKeepAlive {
					pub dest: transfer_keep_alive::Dest,
					#[codec(compact)]
					pub value: transfer_keep_alive::Value,
				}
				pub mod transfer_keep_alive {
					use super::runtime_types;
					pub type Dest = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Value = ::core::primitive::u128;
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
				#[doc = "Transfer the entire transferable balance from the caller account."]
				#[doc = ""]
				#[doc = "NOTE: This function only attempts to transfer _transferable_ balances. This means that"]
				#[doc = "any locked, reserved, or existential deposits (when `keep_alive` is `true`), will not be"]
				#[doc = "transferred by this function. To ensure that this function results in a killed account,"]
				#[doc = "you might need to prepare the account by removing any reference counters, storage"]
				#[doc = "deposits, etc..."]
				#[doc = ""]
				#[doc = "The dispatch origin of this call must be Signed."]
				#[doc = ""]
				#[doc = "- `dest`: The recipient of the transfer."]
				#[doc = "- `keep_alive`: A boolean to determine if the `transfer_all` operation should send all"]
				#[doc = "  of the funds the account has, causing the sender account to be killed (false), or"]
				#[doc = "  transfer everything except at least the existential deposit, which will guarantee to"]
				#[doc = "  keep the sender account alive (true)."]
				pub struct TransferAll {
					pub dest: transfer_all::Dest,
					pub keep_alive: transfer_all::KeepAlive,
				}
				pub mod transfer_all {
					use super::runtime_types;
					pub type Dest = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type KeepAlive = ::core::primitive::bool;
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
				#[doc = "Unreserve some balance from a user by force."]
				#[doc = ""]
				#[doc = "Can only be called by ROOT."]
				pub struct ForceUnreserve {
					pub who: force_unreserve::Who,
					pub amount: force_unreserve::Amount,
				}
				pub mod force_unreserve {
					use super::runtime_types;
					pub type Who = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Amount = ::core::primitive::u128;
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
				#[doc = "Upgrade a specified account."]
				#[doc = ""]
				#[doc = "- `origin`: Must be `Signed`."]
				#[doc = "- `who`: The account to be upgraded."]
				#[doc = ""]
				#[doc = "This will waive the transaction fee if at least all but 10% of the accounts needed to"]
				#[doc = "be upgraded. (We let some not have to be upgraded just in order to allow for the"]
				#[doc = "possibility of churn)."]
				pub struct UpgradeAccounts {
					pub who: upgrade_accounts::Who,
				}
				pub mod upgrade_accounts {
					use super::runtime_types;
					pub type Who = ::std::vec::Vec<::subxt::utils::AccountId32>;
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
				#[doc = "Set the regular balance of a given account."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call is `root`."]
				pub struct ForceSetBalance {
					pub who: force_set_balance::Who,
					#[codec(compact)]
					pub new_free: force_set_balance::NewFree,
				}
				pub mod force_set_balance {
					use super::runtime_types;
					pub type Who = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type NewFree = ::core::primitive::u128;
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceSetBalance {
					const PALLET: &'static str = "ArgonBalances";
					const CALL: &'static str = "force_set_balance";
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
				#[doc = "Adjust the total issuance in a saturating way."]
				#[doc = ""]
				#[doc = "Can only be called by root and always needs a positive `delta`."]
				#[doc = ""]
				#[doc = "# Example"]
				pub struct ForceAdjustTotalIssuance {
					pub direction: force_adjust_total_issuance::Direction,
					#[codec(compact)]
					pub delta: force_adjust_total_issuance::Delta,
				}
				pub mod force_adjust_total_issuance {
					use super::runtime_types;
					pub type Direction = runtime_types::pallet_balances::types::AdjustmentDirection;
					pub type Delta = ::core::primitive::u128;
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceAdjustTotalIssuance {
					const PALLET: &'static str = "ArgonBalances";
					const CALL: &'static str = "force_adjust_total_issuance";
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
				#[doc = "Burn the specified liquid free balance from the origin account."]
				#[doc = ""]
				#[doc = "If the origin's account ends up below the existential deposit as a result"]
				#[doc = "of the burn and `keep_alive` is false, the account will be reaped."]
				#[doc = ""]
				#[doc = "Unlike sending funds to a _burn_ address, which merely makes the funds inaccessible,"]
				#[doc = "this `burn` operation will reduce total issuance by the amount _burned_."]
				pub struct Burn {
					#[codec(compact)]
					pub value: burn::Value,
					pub keep_alive: burn::KeepAlive,
				}
				pub mod burn {
					use super::runtime_types;
					pub type Value = ::core::primitive::u128;
					pub type KeepAlive = ::core::primitive::bool;
				}
				impl ::subxt::blocks::StaticExtrinsic for Burn {
					const PALLET: &'static str = "ArgonBalances";
					const CALL: &'static str = "burn";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Transfer some liquid free balance to another account."]
				#[doc = ""]
				#[doc = "`transfer_allow_death` will set the `FreeBalance` of the sender and receiver."]
				#[doc = "If the sender's account is below the existential deposit as a result"]
				#[doc = "of the transfer, the account will be reaped."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be `Signed` by the transactor."]
				pub fn transfer_allow_death(
					&self,
					dest: types::transfer_allow_death::Dest,
					value: types::transfer_allow_death::Value,
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
				#[doc = "Exactly as `transfer_allow_death`, except the origin must be root and the source account"]
				#[doc = "may be specified."]
				pub fn force_transfer(
					&self,
					source: types::force_transfer::Source,
					dest: types::force_transfer::Dest,
					value: types::force_transfer::Value,
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
				#[doc = "Same as the [`transfer_allow_death`] call, but with a check that the transfer will not"]
				#[doc = "kill the origin account."]
				#[doc = ""]
				#[doc = "99% of the time you want [`transfer_allow_death`] instead."]
				#[doc = ""]
				#[doc = "[`transfer_allow_death`]: struct.Pallet.html#method.transfer"]
				pub fn transfer_keep_alive(
					&self,
					dest: types::transfer_keep_alive::Dest,
					value: types::transfer_keep_alive::Value,
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
				#[doc = "Transfer the entire transferable balance from the caller account."]
				#[doc = ""]
				#[doc = "NOTE: This function only attempts to transfer _transferable_ balances. This means that"]
				#[doc = "any locked, reserved, or existential deposits (when `keep_alive` is `true`), will not be"]
				#[doc = "transferred by this function. To ensure that this function results in a killed account,"]
				#[doc = "you might need to prepare the account by removing any reference counters, storage"]
				#[doc = "deposits, etc..."]
				#[doc = ""]
				#[doc = "The dispatch origin of this call must be Signed."]
				#[doc = ""]
				#[doc = "- `dest`: The recipient of the transfer."]
				#[doc = "- `keep_alive`: A boolean to determine if the `transfer_all` operation should send all"]
				#[doc = "  of the funds the account has, causing the sender account to be killed (false), or"]
				#[doc = "  transfer everything except at least the existential deposit, which will guarantee to"]
				#[doc = "  keep the sender account alive (true)."]
				pub fn transfer_all(
					&self,
					dest: types::transfer_all::Dest,
					keep_alive: types::transfer_all::KeepAlive,
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
				#[doc = "Unreserve some balance from a user by force."]
				#[doc = ""]
				#[doc = "Can only be called by ROOT."]
				pub fn force_unreserve(
					&self,
					who: types::force_unreserve::Who,
					amount: types::force_unreserve::Amount,
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
				#[doc = "Upgrade a specified account."]
				#[doc = ""]
				#[doc = "- `origin`: Must be `Signed`."]
				#[doc = "- `who`: The account to be upgraded."]
				#[doc = ""]
				#[doc = "This will waive the transaction fee if at least all but 10% of the accounts needed to"]
				#[doc = "be upgraded. (We let some not have to be upgraded just in order to allow for the"]
				#[doc = "possibility of churn)."]
				pub fn upgrade_accounts(
					&self,
					who: types::upgrade_accounts::Who,
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
				#[doc = "Set the regular balance of a given account."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call is `root`."]
				pub fn force_set_balance(
					&self,
					who: types::force_set_balance::Who,
					new_free: types::force_set_balance::NewFree,
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
				#[doc = "Adjust the total issuance in a saturating way."]
				#[doc = ""]
				#[doc = "Can only be called by root and always needs a positive `delta`."]
				#[doc = ""]
				#[doc = "# Example"]
				pub fn force_adjust_total_issuance(
					&self,
					direction: types::force_adjust_total_issuance::Direction,
					delta: types::force_adjust_total_issuance::Delta,
				) -> ::subxt::tx::Payload<types::ForceAdjustTotalIssuance> {
					::subxt::tx::Payload::new_static(
						"ArgonBalances",
						"force_adjust_total_issuance",
						types::ForceAdjustTotalIssuance { direction, delta },
						[
							208u8, 134u8, 56u8, 133u8, 232u8, 164u8, 10u8, 213u8, 53u8, 193u8,
							190u8, 63u8, 236u8, 186u8, 96u8, 122u8, 104u8, 87u8, 173u8, 38u8, 58u8,
							176u8, 21u8, 78u8, 42u8, 106u8, 46u8, 248u8, 251u8, 190u8, 150u8,
							202u8,
						],
					)
				}
				#[doc = "Burn the specified liquid free balance from the origin account."]
				#[doc = ""]
				#[doc = "If the origin's account ends up below the existential deposit as a result"]
				#[doc = "of the burn and `keep_alive` is false, the account will be reaped."]
				#[doc = ""]
				#[doc = "Unlike sending funds to a _burn_ address, which merely makes the funds inaccessible,"]
				#[doc = "this `burn` operation will reduce total issuance by the amount _burned_."]
				pub fn burn(
					&self,
					value: types::burn::Value,
					keep_alive: types::burn::KeepAlive,
				) -> ::subxt::tx::Payload<types::Burn> {
					::subxt::tx::Payload::new_static(
						"ArgonBalances",
						"burn",
						types::Burn { value, keep_alive },
						[
							176u8, 64u8, 7u8, 109u8, 16u8, 44u8, 145u8, 125u8, 147u8, 152u8, 130u8,
							114u8, 221u8, 201u8, 150u8, 162u8, 118u8, 71u8, 52u8, 92u8, 240u8,
							116u8, 203u8, 98u8, 5u8, 22u8, 43u8, 102u8, 94u8, 208u8, 101u8, 57u8,
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
				pub account: endowed::Account,
				pub free_balance: endowed::FreeBalance,
			}
			pub mod endowed {
				use super::runtime_types;
				pub type Account = ::subxt::utils::AccountId32;
				pub type FreeBalance = ::core::primitive::u128;
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
				pub account: dust_lost::Account,
				pub amount: dust_lost::Amount,
			}
			pub mod dust_lost {
				use super::runtime_types;
				pub type Account = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub from: transfer::From,
				pub to: transfer::To,
				pub amount: transfer::Amount,
			}
			pub mod transfer {
				use super::runtime_types;
				pub type From = ::subxt::utils::AccountId32;
				pub type To = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: balance_set::Who,
				pub free: balance_set::Free,
			}
			pub mod balance_set {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Free = ::core::primitive::u128;
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
				pub who: reserved::Who,
				pub amount: reserved::Amount,
			}
			pub mod reserved {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: unreserved::Who,
				pub amount: unreserved::Amount,
			}
			pub mod unreserved {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub from: reserve_repatriated::From,
				pub to: reserve_repatriated::To,
				pub amount: reserve_repatriated::Amount,
				pub destination_status: reserve_repatriated::DestinationStatus,
			}
			pub mod reserve_repatriated {
				use super::runtime_types;
				pub type From = ::subxt::utils::AccountId32;
				pub type To = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
				pub type DestinationStatus =
					runtime_types::frame_support::traits::tokens::misc::BalanceStatus;
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
				pub who: deposit::Who,
				pub amount: deposit::Amount,
			}
			pub mod deposit {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: withdraw::Who,
				pub amount: withdraw::Amount,
			}
			pub mod withdraw {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: slashed::Who,
				pub amount: slashed::Amount,
			}
			pub mod slashed {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: minted::Who,
				pub amount: minted::Amount,
			}
			pub mod minted {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: burned::Who,
				pub amount: burned::Amount,
			}
			pub mod burned {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: suspended::Who,
				pub amount: suspended::Amount,
			}
			pub mod suspended {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: restored::Who,
				pub amount: restored::Amount,
			}
			pub mod restored {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: upgraded::Who,
			}
			pub mod upgraded {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
			}
			impl ::subxt::events::StaticEvent for Upgraded {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Upgraded";
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
			#[doc = "Total issuance was increased by `amount`, creating a credit to be balanced."]
			pub struct Issued {
				pub amount: issued::Amount,
			}
			pub mod issued {
				use super::runtime_types;
				pub type Amount = ::core::primitive::u128;
			}
			impl ::subxt::events::StaticEvent for Issued {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Issued";
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
			#[doc = "Total issuance was decreased by `amount`, creating a debt to be balanced."]
			pub struct Rescinded {
				pub amount: rescinded::Amount,
			}
			pub mod rescinded {
				use super::runtime_types;
				pub type Amount = ::core::primitive::u128;
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
				pub who: locked::Who,
				pub amount: locked::Amount,
			}
			pub mod locked {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: unlocked::Who,
				pub amount: unlocked::Amount,
			}
			pub mod unlocked {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: frozen::Who,
				pub amount: frozen::Amount,
			}
			pub mod frozen {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: thawed::Who,
				pub amount: thawed::Amount,
			}
			pub mod thawed {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
			}
			impl ::subxt::events::StaticEvent for Thawed {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "Thawed";
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
			#[doc = "The `TotalIssuance` was forcefully changed."]
			pub struct TotalIssuanceForced {
				pub old: total_issuance_forced::Old,
				pub new: total_issuance_forced::New,
			}
			pub mod total_issuance_forced {
				use super::runtime_types;
				pub type Old = ::core::primitive::u128;
				pub type New = ::core::primitive::u128;
			}
			impl ::subxt::events::StaticEvent for TotalIssuanceForced {
				const PALLET: &'static str = "ArgonBalances";
				const EVENT: &'static str = "TotalIssuanceForced";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod total_issuance {
					use super::runtime_types;
					pub type TotalIssuance = ::core::primitive::u128;
				}
				pub mod inactive_issuance {
					use super::runtime_types;
					pub type InactiveIssuance = ::core::primitive::u128;
				}
				pub mod account {
					use super::runtime_types;
					pub type Account =
						runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod locks {
					use super::runtime_types;
					pub type Locks =
						runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
							runtime_types::pallet_balances::types::BalanceLock<
								::core::primitive::u128,
							>,
						>;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod reserves {
					use super::runtime_types;
					pub type Reserves = runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::ReserveData<
							[::core::primitive::u8; 8usize],
							::core::primitive::u128,
						>,
					>;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod holds {
					use super::runtime_types;
					pub type Holds = runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeHoldReason,
							::core::primitive::u128,
						>,
					>;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod freezes {
					use super::runtime_types;
					pub type Freezes = runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeFreezeReason,
							::core::primitive::u128,
						>,
					>;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The total units issued in the system."]
				pub fn total_issuance(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::total_issuance::TotalIssuance,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"TotalIssuance",
						(),
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
					(),
					types::inactive_issuance::InactiveIssuance,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"InactiveIssuance",
						(),
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
				pub fn account_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::account::Account,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Account",
						(),
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
				pub fn account(
					&self,
					_0: impl ::std::borrow::Borrow<types::account::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::account::Param0>,
					types::account::Account,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Account",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							213u8, 38u8, 200u8, 69u8, 218u8, 0u8, 112u8, 181u8, 160u8, 23u8, 96u8,
							90u8, 3u8, 88u8, 126u8, 22u8, 103u8, 74u8, 64u8, 69u8, 29u8, 247u8,
							18u8, 17u8, 234u8, 143u8, 189u8, 22u8, 247u8, 194u8, 154u8, 249u8,
						],
					)
				}
				#[doc = " Any liquidity locks on some account balances."]
				#[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
				#[doc = ""]
				#[doc = " Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`"]
				pub fn locks_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::locks::Locks,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Locks",
						(),
						[
							10u8, 223u8, 55u8, 0u8, 249u8, 69u8, 168u8, 41u8, 75u8, 35u8, 120u8,
							167u8, 18u8, 132u8, 9u8, 20u8, 91u8, 51u8, 27u8, 69u8, 136u8, 187u8,
							13u8, 220u8, 163u8, 122u8, 26u8, 141u8, 174u8, 249u8, 85u8, 37u8,
						],
					)
				}
				#[doc = " Any liquidity locks on some account balances."]
				#[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
				#[doc = ""]
				#[doc = " Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`"]
				pub fn locks(
					&self,
					_0: impl ::std::borrow::Borrow<types::locks::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::locks::Param0>,
					types::locks::Locks,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Locks",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							10u8, 223u8, 55u8, 0u8, 249u8, 69u8, 168u8, 41u8, 75u8, 35u8, 120u8,
							167u8, 18u8, 132u8, 9u8, 20u8, 91u8, 51u8, 27u8, 69u8, 136u8, 187u8,
							13u8, 220u8, 163u8, 122u8, 26u8, 141u8, 174u8, 249u8, 85u8, 37u8,
						],
					)
				}
				#[doc = " Named reserves on some account balances."]
				#[doc = ""]
				#[doc = " Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`"]
				pub fn reserves_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::reserves::Reserves,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Reserves",
						(),
						[
							112u8, 10u8, 241u8, 77u8, 64u8, 187u8, 106u8, 159u8, 13u8, 153u8,
							140u8, 178u8, 182u8, 50u8, 1u8, 55u8, 149u8, 92u8, 196u8, 229u8, 170u8,
							106u8, 193u8, 88u8, 255u8, 244u8, 2u8, 193u8, 62u8, 235u8, 204u8, 91u8,
						],
					)
				}
				#[doc = " Named reserves on some account balances."]
				#[doc = ""]
				#[doc = " Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`"]
				pub fn reserves(
					&self,
					_0: impl ::std::borrow::Borrow<types::reserves::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::reserves::Param0>,
					types::reserves::Reserves,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Reserves",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							112u8, 10u8, 241u8, 77u8, 64u8, 187u8, 106u8, 159u8, 13u8, 153u8,
							140u8, 178u8, 182u8, 50u8, 1u8, 55u8, 149u8, 92u8, 196u8, 229u8, 170u8,
							106u8, 193u8, 88u8, 255u8, 244u8, 2u8, 193u8, 62u8, 235u8, 204u8, 91u8,
						],
					)
				}
				#[doc = " Holds on account balances."]
				pub fn holds_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::holds::Holds,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Holds",
						(),
						[
							234u8, 77u8, 178u8, 184u8, 219u8, 183u8, 175u8, 110u8, 166u8, 58u8,
							98u8, 108u8, 13u8, 11u8, 120u8, 201u8, 217u8, 47u8, 75u8, 12u8, 32u8,
							103u8, 28u8, 137u8, 64u8, 162u8, 164u8, 132u8, 86u8, 183u8, 165u8,
							106u8,
						],
					)
				}
				#[doc = " Holds on account balances."]
				pub fn holds(
					&self,
					_0: impl ::std::borrow::Borrow<types::holds::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::holds::Param0>,
					types::holds::Holds,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Holds",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							234u8, 77u8, 178u8, 184u8, 219u8, 183u8, 175u8, 110u8, 166u8, 58u8,
							98u8, 108u8, 13u8, 11u8, 120u8, 201u8, 217u8, 47u8, 75u8, 12u8, 32u8,
							103u8, 28u8, 137u8, 64u8, 162u8, 164u8, 132u8, 86u8, 183u8, 165u8,
							106u8,
						],
					)
				}
				#[doc = " Freeze locks on account balances."]
				pub fn freezes_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::freezes::Freezes,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Freezes",
						(),
						[
							137u8, 54u8, 103u8, 63u8, 166u8, 153u8, 14u8, 79u8, 7u8, 65u8, 178u8,
							80u8, 204u8, 36u8, 206u8, 69u8, 194u8, 200u8, 174u8, 172u8, 20u8,
							157u8, 156u8, 101u8, 214u8, 98u8, 160u8, 16u8, 102u8, 198u8, 126u8,
							198u8,
						],
					)
				}
				#[doc = " Freeze locks on account balances."]
				pub fn freezes(
					&self,
					_0: impl ::std::borrow::Borrow<types::freezes::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::freezes::Param0>,
					types::freezes::Freezes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ArgonBalances",
						"Freezes",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
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
				#[doc = ""]
				#[doc = " Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`"]
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
				#[doc = ""]
				#[doc = " Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`"]
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
				#[doc = "Transfer some liquid free balance to another account."]
				#[doc = ""]
				#[doc = "`transfer_allow_death` will set the `FreeBalance` of the sender and receiver."]
				#[doc = "If the sender's account is below the existential deposit as a result"]
				#[doc = "of the transfer, the account will be reaped."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be `Signed` by the transactor."]
				pub struct TransferAllowDeath {
					pub dest: transfer_allow_death::Dest,
					#[codec(compact)]
					pub value: transfer_allow_death::Value,
				}
				pub mod transfer_allow_death {
					use super::runtime_types;
					pub type Dest = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Value = ::core::primitive::u128;
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
				#[doc = "Exactly as `transfer_allow_death`, except the origin must be root and the source account"]
				#[doc = "may be specified."]
				pub struct ForceTransfer {
					pub source: force_transfer::Source,
					pub dest: force_transfer::Dest,
					#[codec(compact)]
					pub value: force_transfer::Value,
				}
				pub mod force_transfer {
					use super::runtime_types;
					pub type Source = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Dest = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Value = ::core::primitive::u128;
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
				#[doc = "Same as the [`transfer_allow_death`] call, but with a check that the transfer will not"]
				#[doc = "kill the origin account."]
				#[doc = ""]
				#[doc = "99% of the time you want [`transfer_allow_death`] instead."]
				#[doc = ""]
				#[doc = "[`transfer_allow_death`]: struct.Pallet.html#method.transfer"]
				pub struct TransferKeepAlive {
					pub dest: transfer_keep_alive::Dest,
					#[codec(compact)]
					pub value: transfer_keep_alive::Value,
				}
				pub mod transfer_keep_alive {
					use super::runtime_types;
					pub type Dest = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Value = ::core::primitive::u128;
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
				#[doc = "Transfer the entire transferable balance from the caller account."]
				#[doc = ""]
				#[doc = "NOTE: This function only attempts to transfer _transferable_ balances. This means that"]
				#[doc = "any locked, reserved, or existential deposits (when `keep_alive` is `true`), will not be"]
				#[doc = "transferred by this function. To ensure that this function results in a killed account,"]
				#[doc = "you might need to prepare the account by removing any reference counters, storage"]
				#[doc = "deposits, etc..."]
				#[doc = ""]
				#[doc = "The dispatch origin of this call must be Signed."]
				#[doc = ""]
				#[doc = "- `dest`: The recipient of the transfer."]
				#[doc = "- `keep_alive`: A boolean to determine if the `transfer_all` operation should send all"]
				#[doc = "  of the funds the account has, causing the sender account to be killed (false), or"]
				#[doc = "  transfer everything except at least the existential deposit, which will guarantee to"]
				#[doc = "  keep the sender account alive (true)."]
				pub struct TransferAll {
					pub dest: transfer_all::Dest,
					pub keep_alive: transfer_all::KeepAlive,
				}
				pub mod transfer_all {
					use super::runtime_types;
					pub type Dest = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type KeepAlive = ::core::primitive::bool;
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
				#[doc = "Unreserve some balance from a user by force."]
				#[doc = ""]
				#[doc = "Can only be called by ROOT."]
				pub struct ForceUnreserve {
					pub who: force_unreserve::Who,
					pub amount: force_unreserve::Amount,
				}
				pub mod force_unreserve {
					use super::runtime_types;
					pub type Who = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Amount = ::core::primitive::u128;
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
				#[doc = "Upgrade a specified account."]
				#[doc = ""]
				#[doc = "- `origin`: Must be `Signed`."]
				#[doc = "- `who`: The account to be upgraded."]
				#[doc = ""]
				#[doc = "This will waive the transaction fee if at least all but 10% of the accounts needed to"]
				#[doc = "be upgraded. (We let some not have to be upgraded just in order to allow for the"]
				#[doc = "possibility of churn)."]
				pub struct UpgradeAccounts {
					pub who: upgrade_accounts::Who,
				}
				pub mod upgrade_accounts {
					use super::runtime_types;
					pub type Who = ::std::vec::Vec<::subxt::utils::AccountId32>;
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
				#[doc = "Set the regular balance of a given account."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call is `root`."]
				pub struct ForceSetBalance {
					pub who: force_set_balance::Who,
					#[codec(compact)]
					pub new_free: force_set_balance::NewFree,
				}
				pub mod force_set_balance {
					use super::runtime_types;
					pub type Who = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type NewFree = ::core::primitive::u128;
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceSetBalance {
					const PALLET: &'static str = "UlixeeBalances";
					const CALL: &'static str = "force_set_balance";
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
				#[doc = "Adjust the total issuance in a saturating way."]
				#[doc = ""]
				#[doc = "Can only be called by root and always needs a positive `delta`."]
				#[doc = ""]
				#[doc = "# Example"]
				pub struct ForceAdjustTotalIssuance {
					pub direction: force_adjust_total_issuance::Direction,
					#[codec(compact)]
					pub delta: force_adjust_total_issuance::Delta,
				}
				pub mod force_adjust_total_issuance {
					use super::runtime_types;
					pub type Direction = runtime_types::pallet_balances::types::AdjustmentDirection;
					pub type Delta = ::core::primitive::u128;
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceAdjustTotalIssuance {
					const PALLET: &'static str = "UlixeeBalances";
					const CALL: &'static str = "force_adjust_total_issuance";
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
				#[doc = "Burn the specified liquid free balance from the origin account."]
				#[doc = ""]
				#[doc = "If the origin's account ends up below the existential deposit as a result"]
				#[doc = "of the burn and `keep_alive` is false, the account will be reaped."]
				#[doc = ""]
				#[doc = "Unlike sending funds to a _burn_ address, which merely makes the funds inaccessible,"]
				#[doc = "this `burn` operation will reduce total issuance by the amount _burned_."]
				pub struct Burn {
					#[codec(compact)]
					pub value: burn::Value,
					pub keep_alive: burn::KeepAlive,
				}
				pub mod burn {
					use super::runtime_types;
					pub type Value = ::core::primitive::u128;
					pub type KeepAlive = ::core::primitive::bool;
				}
				impl ::subxt::blocks::StaticExtrinsic for Burn {
					const PALLET: &'static str = "UlixeeBalances";
					const CALL: &'static str = "burn";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Transfer some liquid free balance to another account."]
				#[doc = ""]
				#[doc = "`transfer_allow_death` will set the `FreeBalance` of the sender and receiver."]
				#[doc = "If the sender's account is below the existential deposit as a result"]
				#[doc = "of the transfer, the account will be reaped."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be `Signed` by the transactor."]
				pub fn transfer_allow_death(
					&self,
					dest: types::transfer_allow_death::Dest,
					value: types::transfer_allow_death::Value,
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
				#[doc = "Exactly as `transfer_allow_death`, except the origin must be root and the source account"]
				#[doc = "may be specified."]
				pub fn force_transfer(
					&self,
					source: types::force_transfer::Source,
					dest: types::force_transfer::Dest,
					value: types::force_transfer::Value,
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
				#[doc = "Same as the [`transfer_allow_death`] call, but with a check that the transfer will not"]
				#[doc = "kill the origin account."]
				#[doc = ""]
				#[doc = "99% of the time you want [`transfer_allow_death`] instead."]
				#[doc = ""]
				#[doc = "[`transfer_allow_death`]: struct.Pallet.html#method.transfer"]
				pub fn transfer_keep_alive(
					&self,
					dest: types::transfer_keep_alive::Dest,
					value: types::transfer_keep_alive::Value,
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
				#[doc = "Transfer the entire transferable balance from the caller account."]
				#[doc = ""]
				#[doc = "NOTE: This function only attempts to transfer _transferable_ balances. This means that"]
				#[doc = "any locked, reserved, or existential deposits (when `keep_alive` is `true`), will not be"]
				#[doc = "transferred by this function. To ensure that this function results in a killed account,"]
				#[doc = "you might need to prepare the account by removing any reference counters, storage"]
				#[doc = "deposits, etc..."]
				#[doc = ""]
				#[doc = "The dispatch origin of this call must be Signed."]
				#[doc = ""]
				#[doc = "- `dest`: The recipient of the transfer."]
				#[doc = "- `keep_alive`: A boolean to determine if the `transfer_all` operation should send all"]
				#[doc = "  of the funds the account has, causing the sender account to be killed (false), or"]
				#[doc = "  transfer everything except at least the existential deposit, which will guarantee to"]
				#[doc = "  keep the sender account alive (true)."]
				pub fn transfer_all(
					&self,
					dest: types::transfer_all::Dest,
					keep_alive: types::transfer_all::KeepAlive,
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
				#[doc = "Unreserve some balance from a user by force."]
				#[doc = ""]
				#[doc = "Can only be called by ROOT."]
				pub fn force_unreserve(
					&self,
					who: types::force_unreserve::Who,
					amount: types::force_unreserve::Amount,
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
				#[doc = "Upgrade a specified account."]
				#[doc = ""]
				#[doc = "- `origin`: Must be `Signed`."]
				#[doc = "- `who`: The account to be upgraded."]
				#[doc = ""]
				#[doc = "This will waive the transaction fee if at least all but 10% of the accounts needed to"]
				#[doc = "be upgraded. (We let some not have to be upgraded just in order to allow for the"]
				#[doc = "possibility of churn)."]
				pub fn upgrade_accounts(
					&self,
					who: types::upgrade_accounts::Who,
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
				#[doc = "Set the regular balance of a given account."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call is `root`."]
				pub fn force_set_balance(
					&self,
					who: types::force_set_balance::Who,
					new_free: types::force_set_balance::NewFree,
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
				#[doc = "Adjust the total issuance in a saturating way."]
				#[doc = ""]
				#[doc = "Can only be called by root and always needs a positive `delta`."]
				#[doc = ""]
				#[doc = "# Example"]
				pub fn force_adjust_total_issuance(
					&self,
					direction: types::force_adjust_total_issuance::Direction,
					delta: types::force_adjust_total_issuance::Delta,
				) -> ::subxt::tx::Payload<types::ForceAdjustTotalIssuance> {
					::subxt::tx::Payload::new_static(
						"UlixeeBalances",
						"force_adjust_total_issuance",
						types::ForceAdjustTotalIssuance { direction, delta },
						[
							208u8, 134u8, 56u8, 133u8, 232u8, 164u8, 10u8, 213u8, 53u8, 193u8,
							190u8, 63u8, 236u8, 186u8, 96u8, 122u8, 104u8, 87u8, 173u8, 38u8, 58u8,
							176u8, 21u8, 78u8, 42u8, 106u8, 46u8, 248u8, 251u8, 190u8, 150u8,
							202u8,
						],
					)
				}
				#[doc = "Burn the specified liquid free balance from the origin account."]
				#[doc = ""]
				#[doc = "If the origin's account ends up below the existential deposit as a result"]
				#[doc = "of the burn and `keep_alive` is false, the account will be reaped."]
				#[doc = ""]
				#[doc = "Unlike sending funds to a _burn_ address, which merely makes the funds inaccessible,"]
				#[doc = "this `burn` operation will reduce total issuance by the amount _burned_."]
				pub fn burn(
					&self,
					value: types::burn::Value,
					keep_alive: types::burn::KeepAlive,
				) -> ::subxt::tx::Payload<types::Burn> {
					::subxt::tx::Payload::new_static(
						"UlixeeBalances",
						"burn",
						types::Burn { value, keep_alive },
						[
							176u8, 64u8, 7u8, 109u8, 16u8, 44u8, 145u8, 125u8, 147u8, 152u8, 130u8,
							114u8, 221u8, 201u8, 150u8, 162u8, 118u8, 71u8, 52u8, 92u8, 240u8,
							116u8, 203u8, 98u8, 5u8, 22u8, 43u8, 102u8, 94u8, 208u8, 101u8, 57u8,
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
				pub account: endowed::Account,
				pub free_balance: endowed::FreeBalance,
			}
			pub mod endowed {
				use super::runtime_types;
				pub type Account = ::subxt::utils::AccountId32;
				pub type FreeBalance = ::core::primitive::u128;
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
				pub account: dust_lost::Account,
				pub amount: dust_lost::Amount,
			}
			pub mod dust_lost {
				use super::runtime_types;
				pub type Account = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub from: transfer::From,
				pub to: transfer::To,
				pub amount: transfer::Amount,
			}
			pub mod transfer {
				use super::runtime_types;
				pub type From = ::subxt::utils::AccountId32;
				pub type To = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: balance_set::Who,
				pub free: balance_set::Free,
			}
			pub mod balance_set {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Free = ::core::primitive::u128;
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
				pub who: reserved::Who,
				pub amount: reserved::Amount,
			}
			pub mod reserved {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: unreserved::Who,
				pub amount: unreserved::Amount,
			}
			pub mod unreserved {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub from: reserve_repatriated::From,
				pub to: reserve_repatriated::To,
				pub amount: reserve_repatriated::Amount,
				pub destination_status: reserve_repatriated::DestinationStatus,
			}
			pub mod reserve_repatriated {
				use super::runtime_types;
				pub type From = ::subxt::utils::AccountId32;
				pub type To = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
				pub type DestinationStatus =
					runtime_types::frame_support::traits::tokens::misc::BalanceStatus;
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
				pub who: deposit::Who,
				pub amount: deposit::Amount,
			}
			pub mod deposit {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: withdraw::Who,
				pub amount: withdraw::Amount,
			}
			pub mod withdraw {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: slashed::Who,
				pub amount: slashed::Amount,
			}
			pub mod slashed {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: minted::Who,
				pub amount: minted::Amount,
			}
			pub mod minted {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: burned::Who,
				pub amount: burned::Amount,
			}
			pub mod burned {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: suspended::Who,
				pub amount: suspended::Amount,
			}
			pub mod suspended {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: restored::Who,
				pub amount: restored::Amount,
			}
			pub mod restored {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: upgraded::Who,
			}
			pub mod upgraded {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
			}
			impl ::subxt::events::StaticEvent for Upgraded {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Upgraded";
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
			#[doc = "Total issuance was increased by `amount`, creating a credit to be balanced."]
			pub struct Issued {
				pub amount: issued::Amount,
			}
			pub mod issued {
				use super::runtime_types;
				pub type Amount = ::core::primitive::u128;
			}
			impl ::subxt::events::StaticEvent for Issued {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Issued";
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
			#[doc = "Total issuance was decreased by `amount`, creating a debt to be balanced."]
			pub struct Rescinded {
				pub amount: rescinded::Amount,
			}
			pub mod rescinded {
				use super::runtime_types;
				pub type Amount = ::core::primitive::u128;
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
				pub who: locked::Who,
				pub amount: locked::Amount,
			}
			pub mod locked {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: unlocked::Who,
				pub amount: unlocked::Amount,
			}
			pub mod unlocked {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: frozen::Who,
				pub amount: frozen::Amount,
			}
			pub mod frozen {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
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
				pub who: thawed::Who,
				pub amount: thawed::Amount,
			}
			pub mod thawed {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type Amount = ::core::primitive::u128;
			}
			impl ::subxt::events::StaticEvent for Thawed {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "Thawed";
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
			#[doc = "The `TotalIssuance` was forcefully changed."]
			pub struct TotalIssuanceForced {
				pub old: total_issuance_forced::Old,
				pub new: total_issuance_forced::New,
			}
			pub mod total_issuance_forced {
				use super::runtime_types;
				pub type Old = ::core::primitive::u128;
				pub type New = ::core::primitive::u128;
			}
			impl ::subxt::events::StaticEvent for TotalIssuanceForced {
				const PALLET: &'static str = "UlixeeBalances";
				const EVENT: &'static str = "TotalIssuanceForced";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod total_issuance {
					use super::runtime_types;
					pub type TotalIssuance = ::core::primitive::u128;
				}
				pub mod inactive_issuance {
					use super::runtime_types;
					pub type InactiveIssuance = ::core::primitive::u128;
				}
				pub mod account {
					use super::runtime_types;
					pub type Account =
						runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod locks {
					use super::runtime_types;
					pub type Locks =
						runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
							runtime_types::pallet_balances::types::BalanceLock<
								::core::primitive::u128,
							>,
						>;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod reserves {
					use super::runtime_types;
					pub type Reserves = runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::ReserveData<
							[::core::primitive::u8; 8usize],
							::core::primitive::u128,
						>,
					>;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod holds {
					use super::runtime_types;
					pub type Holds = runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeHoldReason,
							::core::primitive::u128,
						>,
					>;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
				pub mod freezes {
					use super::runtime_types;
					pub type Freezes = runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::ulx_node_runtime::RuntimeFreezeReason,
							::core::primitive::u128,
						>,
					>;
					pub type Param0 = ::subxt::utils::AccountId32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The total units issued in the system."]
				pub fn total_issuance(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::total_issuance::TotalIssuance,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"TotalIssuance",
						(),
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
					(),
					types::inactive_issuance::InactiveIssuance,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"InactiveIssuance",
						(),
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
				pub fn account_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::account::Account,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Account",
						(),
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
				pub fn account(
					&self,
					_0: impl ::std::borrow::Borrow<types::account::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::account::Param0>,
					types::account::Account,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Account",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							213u8, 38u8, 200u8, 69u8, 218u8, 0u8, 112u8, 181u8, 160u8, 23u8, 96u8,
							90u8, 3u8, 88u8, 126u8, 22u8, 103u8, 74u8, 64u8, 69u8, 29u8, 247u8,
							18u8, 17u8, 234u8, 143u8, 189u8, 22u8, 247u8, 194u8, 154u8, 249u8,
						],
					)
				}
				#[doc = " Any liquidity locks on some account balances."]
				#[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
				#[doc = ""]
				#[doc = " Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`"]
				pub fn locks_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::locks::Locks,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Locks",
						(),
						[
							10u8, 223u8, 55u8, 0u8, 249u8, 69u8, 168u8, 41u8, 75u8, 35u8, 120u8,
							167u8, 18u8, 132u8, 9u8, 20u8, 91u8, 51u8, 27u8, 69u8, 136u8, 187u8,
							13u8, 220u8, 163u8, 122u8, 26u8, 141u8, 174u8, 249u8, 85u8, 37u8,
						],
					)
				}
				#[doc = " Any liquidity locks on some account balances."]
				#[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
				#[doc = ""]
				#[doc = " Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`"]
				pub fn locks(
					&self,
					_0: impl ::std::borrow::Borrow<types::locks::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::locks::Param0>,
					types::locks::Locks,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Locks",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							10u8, 223u8, 55u8, 0u8, 249u8, 69u8, 168u8, 41u8, 75u8, 35u8, 120u8,
							167u8, 18u8, 132u8, 9u8, 20u8, 91u8, 51u8, 27u8, 69u8, 136u8, 187u8,
							13u8, 220u8, 163u8, 122u8, 26u8, 141u8, 174u8, 249u8, 85u8, 37u8,
						],
					)
				}
				#[doc = " Named reserves on some account balances."]
				#[doc = ""]
				#[doc = " Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`"]
				pub fn reserves_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::reserves::Reserves,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Reserves",
						(),
						[
							112u8, 10u8, 241u8, 77u8, 64u8, 187u8, 106u8, 159u8, 13u8, 153u8,
							140u8, 178u8, 182u8, 50u8, 1u8, 55u8, 149u8, 92u8, 196u8, 229u8, 170u8,
							106u8, 193u8, 88u8, 255u8, 244u8, 2u8, 193u8, 62u8, 235u8, 204u8, 91u8,
						],
					)
				}
				#[doc = " Named reserves on some account balances."]
				#[doc = ""]
				#[doc = " Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`"]
				pub fn reserves(
					&self,
					_0: impl ::std::borrow::Borrow<types::reserves::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::reserves::Param0>,
					types::reserves::Reserves,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Reserves",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							112u8, 10u8, 241u8, 77u8, 64u8, 187u8, 106u8, 159u8, 13u8, 153u8,
							140u8, 178u8, 182u8, 50u8, 1u8, 55u8, 149u8, 92u8, 196u8, 229u8, 170u8,
							106u8, 193u8, 88u8, 255u8, 244u8, 2u8, 193u8, 62u8, 235u8, 204u8, 91u8,
						],
					)
				}
				#[doc = " Holds on account balances."]
				pub fn holds_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::holds::Holds,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Holds",
						(),
						[
							234u8, 77u8, 178u8, 184u8, 219u8, 183u8, 175u8, 110u8, 166u8, 58u8,
							98u8, 108u8, 13u8, 11u8, 120u8, 201u8, 217u8, 47u8, 75u8, 12u8, 32u8,
							103u8, 28u8, 137u8, 64u8, 162u8, 164u8, 132u8, 86u8, 183u8, 165u8,
							106u8,
						],
					)
				}
				#[doc = " Holds on account balances."]
				pub fn holds(
					&self,
					_0: impl ::std::borrow::Borrow<types::holds::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::holds::Param0>,
					types::holds::Holds,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Holds",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							234u8, 77u8, 178u8, 184u8, 219u8, 183u8, 175u8, 110u8, 166u8, 58u8,
							98u8, 108u8, 13u8, 11u8, 120u8, 201u8, 217u8, 47u8, 75u8, 12u8, 32u8,
							103u8, 28u8, 137u8, 64u8, 162u8, 164u8, 132u8, 86u8, 183u8, 165u8,
							106u8,
						],
					)
				}
				#[doc = " Freeze locks on account balances."]
				pub fn freezes_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::freezes::Freezes,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Freezes",
						(),
						[
							137u8, 54u8, 103u8, 63u8, 166u8, 153u8, 14u8, 79u8, 7u8, 65u8, 178u8,
							80u8, 204u8, 36u8, 206u8, 69u8, 194u8, 200u8, 174u8, 172u8, 20u8,
							157u8, 156u8, 101u8, 214u8, 98u8, 160u8, 16u8, 102u8, 198u8, 126u8,
							198u8,
						],
					)
				}
				#[doc = " Freeze locks on account balances."]
				pub fn freezes(
					&self,
					_0: impl ::std::borrow::Borrow<types::freezes::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::freezes::Param0>,
					types::freezes::Freezes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"UlixeeBalances",
						"Freezes",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
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
				#[doc = ""]
				#[doc = " Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`"]
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
				#[doc = ""]
				#[doc = " Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`"]
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
				#[doc = "Pause a call."]
				#[doc = ""]
				#[doc = "Can only be called by [`Config::PauseOrigin`]."]
				#[doc = "Emits an [`Event::CallPaused`] event on success."]
				pub struct Pause {
					pub full_name: pause::FullName,
				}
				pub mod pause {
					use super::runtime_types;
					pub type FullName = (
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					);
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
				#[doc = "Un-pause a call."]
				#[doc = ""]
				#[doc = "Can only be called by [`Config::UnpauseOrigin`]."]
				#[doc = "Emits an [`Event::CallUnpaused`] event on success."]
				pub struct Unpause {
					pub ident: unpause::Ident,
				}
				pub mod unpause {
					use super::runtime_types;
					pub type Ident = (
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					);
				}
				impl ::subxt::blocks::StaticExtrinsic for Unpause {
					const PALLET: &'static str = "TxPause";
					const CALL: &'static str = "unpause";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Pause a call."]
				#[doc = ""]
				#[doc = "Can only be called by [`Config::PauseOrigin`]."]
				#[doc = "Emits an [`Event::CallPaused`] event on success."]
				pub fn pause(
					&self,
					full_name: types::pause::FullName,
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
				#[doc = "Un-pause a call."]
				#[doc = ""]
				#[doc = "Can only be called by [`Config::UnpauseOrigin`]."]
				#[doc = "Emits an [`Event::CallUnpaused`] event on success."]
				pub fn unpause(
					&self,
					ident: types::unpause::Ident,
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
				pub full_name: call_paused::FullName,
			}
			pub mod call_paused {
				use super::runtime_types;
				pub type FullName = (
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
				);
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
				pub full_name: call_unpaused::FullName,
			}
			pub mod call_unpaused {
				use super::runtime_types;
				pub type FullName = (
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
				);
			}
			impl ::subxt::events::StaticEvent for CallUnpaused {
				const PALLET: &'static str = "TxPause";
				const EVENT: &'static str = "CallUnpaused";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod paused_calls {
					use super::runtime_types;
					pub type PausedCalls = ();
					pub type Param0 = runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>;
					pub type Param1 = runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The set of calls that are explicitly paused."]
				pub fn paused_calls_iter(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::paused_calls::PausedCalls,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TxPause",
						"PausedCalls",
						(),
						[
							36u8, 9u8, 29u8, 154u8, 39u8, 47u8, 237u8, 97u8, 176u8, 241u8, 153u8,
							131u8, 20u8, 16u8, 73u8, 63u8, 27u8, 21u8, 107u8, 5u8, 147u8, 198u8,
							82u8, 212u8, 38u8, 162u8, 1u8, 203u8, 57u8, 187u8, 53u8, 132u8,
						],
					)
				}
				#[doc = " The set of calls that are explicitly paused."]
				pub fn paused_calls_iter1(
					&self,
					_0: impl ::std::borrow::Borrow<types::paused_calls::Param0>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageKey<types::paused_calls::Param0>,
					types::paused_calls::PausedCalls,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TxPause",
						"PausedCalls",
						::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
						[
							36u8, 9u8, 29u8, 154u8, 39u8, 47u8, 237u8, 97u8, 176u8, 241u8, 153u8,
							131u8, 20u8, 16u8, 73u8, 63u8, 27u8, 21u8, 107u8, 5u8, 147u8, 198u8,
							82u8, 212u8, 38u8, 162u8, 1u8, 203u8, 57u8, 187u8, 53u8, 132u8,
						],
					)
				}
				#[doc = " The set of calls that are explicitly paused."]
				pub fn paused_calls(
					&self,
					_0: impl ::std::borrow::Borrow<types::paused_calls::Param0>,
					_1: impl ::std::borrow::Borrow<types::paused_calls::Param1>,
				) -> ::subxt::storage::address::Address<
					(
						::subxt::storage::address::StaticStorageKey<types::paused_calls::Param0>,
						::subxt::storage::address::StaticStorageKey<types::paused_calls::Param1>,
					),
					types::paused_calls::PausedCalls,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"TxPause",
						"PausedCalls",
						(
							::subxt::storage::address::StaticStorageKey::new(_0.borrow()),
							::subxt::storage::address::StaticStorageKey::new(_1.borrow()),
						),
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
				pub who: transaction_fee_paid::Who,
				pub actual_fee: transaction_fee_paid::ActualFee,
				pub tip: transaction_fee_paid::Tip,
			}
			pub mod transaction_fee_paid {
				use super::runtime_types;
				pub type Who = ::subxt::utils::AccountId32;
				pub type ActualFee = ::core::primitive::u128;
				pub type Tip = ::core::primitive::u128;
			}
			impl ::subxt::events::StaticEvent for TransactionFeePaid {
				const PALLET: &'static str = "TransactionPayment";
				const EVENT: &'static str = "TransactionFeePaid";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod next_fee_multiplier {
					use super::runtime_types;
					pub type NextFeeMultiplier =
						runtime_types::sp_arithmetic::fixed_point::FixedU128;
				}
				pub mod storage_version {
					use super::runtime_types;
					pub type StorageVersion = runtime_types::pallet_transaction_payment::Releases;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				pub fn next_fee_multiplier(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::next_fee_multiplier::NextFeeMultiplier,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"TransactionPayment",
						"NextFeeMultiplier",
						(),
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
					(),
					types::storage_version::StorageVersion,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"TransactionPayment",
						"StorageVersion",
						(),
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
				#[doc = "Authenticates the sudo key and dispatches a function call with `Root` origin."]
				pub struct Sudo {
					pub call: ::std::boxed::Box<sudo::Call>,
				}
				pub mod sudo {
					use super::runtime_types;
					pub type Call = runtime_types::ulx_node_runtime::RuntimeCall;
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
				#[doc = "Authenticates the sudo key and dispatches a function call with `Root` origin."]
				#[doc = "This function does not check the weight of the call, and instead allows the"]
				#[doc = "Sudo user to specify the weight of the call."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				pub struct SudoUncheckedWeight {
					pub call: ::std::boxed::Box<sudo_unchecked_weight::Call>,
					pub weight: sudo_unchecked_weight::Weight,
				}
				pub mod sudo_unchecked_weight {
					use super::runtime_types;
					pub type Call = runtime_types::ulx_node_runtime::RuntimeCall;
					pub type Weight = runtime_types::sp_weights::weight_v2::Weight;
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
				#[doc = "Authenticates the current sudo key and sets the given AccountId (`new`) as the new sudo"]
				#[doc = "key."]
				pub struct SetKey {
					pub new: set_key::New,
				}
				pub mod set_key {
					use super::runtime_types;
					pub type New = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
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
				#[doc = "Authenticates the sudo key and dispatches a function call with `Signed` origin from"]
				#[doc = "a given account."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				pub struct SudoAs {
					pub who: sudo_as::Who,
					pub call: ::std::boxed::Box<sudo_as::Call>,
				}
				pub mod sudo_as {
					use super::runtime_types;
					pub type Who = ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>;
					pub type Call = runtime_types::ulx_node_runtime::RuntimeCall;
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
				#[doc = "Permanently removes the sudo key."]
				#[doc = ""]
				#[doc = "**This cannot be un-done.**"]
				pub struct RemoveKey;
				impl ::subxt::blocks::StaticExtrinsic for RemoveKey {
					const PALLET: &'static str = "Sudo";
					const CALL: &'static str = "remove_key";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "Authenticates the sudo key and dispatches a function call with `Root` origin."]
				pub fn sudo(&self, call: types::sudo::Call) -> ::subxt::tx::Payload<types::Sudo> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"sudo",
						types::Sudo { call: ::std::boxed::Box::new(call) },
						[
							55u8, 119u8, 99u8, 57u8, 192u8, 92u8, 231u8, 244u8, 244u8, 237u8, 59u8,
							218u8, 145u8, 106u8, 68u8, 100u8, 74u8, 15u8, 227u8, 213u8, 241u8,
							188u8, 247u8, 0u8, 251u8, 212u8, 55u8, 82u8, 225u8, 183u8, 124u8, 91u8,
						],
					)
				}
				#[doc = "Authenticates the sudo key and dispatches a function call with `Root` origin."]
				#[doc = "This function does not check the weight of the call, and instead allows the"]
				#[doc = "Sudo user to specify the weight of the call."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				pub fn sudo_unchecked_weight(
					&self,
					call: types::sudo_unchecked_weight::Call,
					weight: types::sudo_unchecked_weight::Weight,
				) -> ::subxt::tx::Payload<types::SudoUncheckedWeight> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"sudo_unchecked_weight",
						types::SudoUncheckedWeight { call: ::std::boxed::Box::new(call), weight },
						[
							149u8, 249u8, 27u8, 221u8, 145u8, 150u8, 192u8, 54u8, 211u8, 87u8,
							244u8, 250u8, 116u8, 210u8, 169u8, 46u8, 98u8, 58u8, 65u8, 145u8,
							202u8, 74u8, 236u8, 114u8, 181u8, 12u8, 38u8, 37u8, 161u8, 12u8, 149u8,
							11u8,
						],
					)
				}
				#[doc = "Authenticates the current sudo key and sets the given AccountId (`new`) as the new sudo"]
				#[doc = "key."]
				pub fn set_key(
					&self,
					new: types::set_key::New,
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
				#[doc = "Authenticates the sudo key and dispatches a function call with `Signed` origin from"]
				#[doc = "a given account."]
				#[doc = ""]
				#[doc = "The dispatch origin for this call must be _Signed_."]
				pub fn sudo_as(
					&self,
					who: types::sudo_as::Who,
					call: types::sudo_as::Call,
				) -> ::subxt::tx::Payload<types::SudoAs> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"sudo_as",
						types::SudoAs { who, call: ::std::boxed::Box::new(call) },
						[
							92u8, 21u8, 242u8, 42u8, 8u8, 193u8, 113u8, 165u8, 23u8, 209u8, 102u8,
							126u8, 74u8, 100u8, 9u8, 163u8, 15u8, 56u8, 140u8, 22u8, 46u8, 72u8,
							146u8, 188u8, 107u8, 103u8, 78u8, 69u8, 43u8, 143u8, 101u8, 70u8,
						],
					)
				}
				#[doc = "Permanently removes the sudo key."]
				#[doc = ""]
				#[doc = "**This cannot be un-done.**"]
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
				pub sudo_result: sudid::SudoResult,
			}
			pub mod sudid {
				use super::runtime_types;
				pub type SudoResult =
					::core::result::Result<(), runtime_types::sp_runtime::DispatchError>;
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
				pub old: key_changed::Old,
				pub new: key_changed::New,
			}
			pub mod key_changed {
				use super::runtime_types;
				pub type Old = ::core::option::Option<::subxt::utils::AccountId32>;
				pub type New = ::subxt::utils::AccountId32;
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
				pub sudo_result: sudo_as_done::SudoResult,
			}
			pub mod sudo_as_done {
				use super::runtime_types;
				pub type SudoResult =
					::core::result::Result<(), runtime_types::sp_runtime::DispatchError>;
			}
			impl ::subxt::events::StaticEvent for SudoAsDone {
				const PALLET: &'static str = "Sudo";
				const EVENT: &'static str = "SudoAsDone";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub mod types {
				use super::runtime_types;
				pub mod key {
					use super::runtime_types;
					pub type Key = ::subxt::utils::AccountId32;
				}
			}
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The `AccountId` of the sudo key."]
				pub fn key(
					&self,
				) -> ::subxt::storage::address::Address<
					(),
					types::key::Key,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Sudo",
						"Key",
						(),
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
					#[doc = "Make some on-chain remark."]
					#[doc = ""]
					#[doc = "Can be executed by every `origin`."]
					remark { remark: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 1)]
					#[doc = "Set the number of pages in the WebAssembly environment's heap."]
					set_heap_pages { pages: ::core::primitive::u64 },
					#[codec(index = 2)]
					#[doc = "Set the new runtime code."]
					set_code { code: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 3)]
					#[doc = "Set the new runtime code without doing any checks of the given `code`."]
					#[doc = ""]
					#[doc = "Note that runtime upgrades will not run if this is called with a not-increasing spec"]
					#[doc = "version!"]
					set_code_without_checks { code: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 4)]
					#[doc = "Set some items of storage."]
					set_storage {
						items: ::std::vec::Vec<(
							::std::vec::Vec<::core::primitive::u8>,
							::std::vec::Vec<::core::primitive::u8>,
						)>,
					},
					#[codec(index = 5)]
					#[doc = "Kill some items from storage."]
					kill_storage { keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>> },
					#[codec(index = 6)]
					#[doc = "Kill all storage items with a key that starts with the given prefix."]
					#[doc = ""]
					#[doc = "**NOTE:** We rely on the Root origin to provide us the number of subkeys under"]
					#[doc = "the prefix we are removing to accurately calculate the weight of this function."]
					kill_prefix {
						prefix: ::std::vec::Vec<::core::primitive::u8>,
						subkeys: ::core::primitive::u32,
					},
					#[codec(index = 7)]
					#[doc = "Make some on-chain remark and emit event."]
					remark_with_event { remark: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 9)]
					#[doc = "Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be supplied"]
					#[doc = "later."]
					#[doc = ""]
					#[doc = "This call requires Root origin."]
					authorize_upgrade { code_hash: ::sp_core::H256 },
					#[codec(index = 10)]
					#[doc = "Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be supplied"]
					#[doc = "later."]
					#[doc = ""]
					#[doc = "WARNING: This authorizes an upgrade that will take place without any safety checks, for"]
					#[doc = "example that the spec name remains the same and that the version number increases. Not"]
					#[doc = "recommended for normal use. Use `authorize_upgrade` instead."]
					#[doc = ""]
					#[doc = "This call requires Root origin."]
					authorize_upgrade_without_checks { code_hash: ::sp_core::H256 },
					#[codec(index = 11)]
					#[doc = "Provide the preimage (runtime binary) `code` for an upgrade that has been authorized."]
					#[doc = ""]
					#[doc = "If the authorization required a version check, this call will ensure the spec name"]
					#[doc = "remains unchanged and that the spec version has increased."]
					#[doc = ""]
					#[doc = "Depending on the runtime's `OnSetCode` configuration, this function may directly apply"]
					#[doc = "the new `code` in the same block or attempt to schedule the upgrade."]
					#[doc = ""]
					#[doc = "All origins are allowed."]
					apply_authorized_upgrade { code: ::std::vec::Vec<::core::primitive::u8> },
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
					#[codec(index = 6)]
					#[doc = "A multi-block migration is ongoing and prevents the current code from being replaced."]
					MultiBlockMigrationsOngoing,
					#[codec(index = 7)]
					#[doc = "No upgrade authorized."]
					NothingAuthorized,
					#[codec(index = 8)]
					#[doc = "The submitted code is not authorized."]
					Unauthorized,
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
					Remarked { sender: ::subxt::utils::AccountId32, hash: ::sp_core::H256 },
					#[codec(index = 6)]
					#[doc = "An upgrade was authorized."]
					UpgradeAuthorized {
						code_hash: ::sp_core::H256,
						check_version: ::core::primitive::bool,
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
			pub struct CodeUpgradeAuthorization {
				pub code_hash: ::sp_core::H256,
				pub check_version: ::core::primitive::bool,
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
					#[doc = "Transfer some liquid free balance to another account."]
					#[doc = ""]
					#[doc = "`transfer_allow_death` will set the `FreeBalance` of the sender and receiver."]
					#[doc = "If the sender's account is below the existential deposit as a result"]
					#[doc = "of the transfer, the account will be reaped."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be `Signed` by the transactor."]
					transfer_allow_death {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 2)]
					#[doc = "Exactly as `transfer_allow_death`, except the origin must be root and the source account"]
					#[doc = "may be specified."]
					force_transfer {
						source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 3)]
					#[doc = "Same as the [`transfer_allow_death`] call, but with a check that the transfer will not"]
					#[doc = "kill the origin account."]
					#[doc = ""]
					#[doc = "99% of the time you want [`transfer_allow_death`] instead."]
					#[doc = ""]
					#[doc = "[`transfer_allow_death`]: struct.Pallet.html#method.transfer"]
					transfer_keep_alive {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 4)]
					#[doc = "Transfer the entire transferable balance from the caller account."]
					#[doc = ""]
					#[doc = "NOTE: This function only attempts to transfer _transferable_ balances. This means that"]
					#[doc = "any locked, reserved, or existential deposits (when `keep_alive` is `true`), will not be"]
					#[doc = "transferred by this function. To ensure that this function results in a killed account,"]
					#[doc = "you might need to prepare the account by removing any reference counters, storage"]
					#[doc = "deposits, etc..."]
					#[doc = ""]
					#[doc = "The dispatch origin of this call must be Signed."]
					#[doc = ""]
					#[doc = "- `dest`: The recipient of the transfer."]
					#[doc = "- `keep_alive`: A boolean to determine if the `transfer_all` operation should send all"]
					#[doc = "  of the funds the account has, causing the sender account to be killed (false), or"]
					#[doc = "  transfer everything except at least the existential deposit, which will guarantee to"]
					#[doc = "  keep the sender account alive (true)."]
					transfer_all {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						keep_alive: ::core::primitive::bool,
					},
					#[codec(index = 5)]
					#[doc = "Unreserve some balance from a user by force."]
					#[doc = ""]
					#[doc = "Can only be called by ROOT."]
					force_unreserve {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 6)]
					#[doc = "Upgrade a specified account."]
					#[doc = ""]
					#[doc = "- `origin`: Must be `Signed`."]
					#[doc = "- `who`: The account to be upgraded."]
					#[doc = ""]
					#[doc = "This will waive the transaction fee if at least all but 10% of the accounts needed to"]
					#[doc = "be upgraded. (We let some not have to be upgraded just in order to allow for the"]
					#[doc = "possibility of churn)."]
					upgrade_accounts { who: ::std::vec::Vec<::subxt::utils::AccountId32> },
					#[codec(index = 8)]
					#[doc = "Set the regular balance of a given account."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call is `root`."]
					force_set_balance {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						new_free: ::core::primitive::u128,
					},
					#[codec(index = 9)]
					#[doc = "Adjust the total issuance in a saturating way."]
					#[doc = ""]
					#[doc = "Can only be called by root and always needs a positive `delta`."]
					#[doc = ""]
					#[doc = "# Example"]
					force_adjust_total_issuance {
						direction: runtime_types::pallet_balances::types::AdjustmentDirection,
						#[codec(compact)]
						delta: ::core::primitive::u128,
					},
					#[codec(index = 10)]
					#[doc = "Burn the specified liquid free balance from the origin account."]
					#[doc = ""]
					#[doc = "If the origin's account ends up below the existential deposit as a result"]
					#[doc = "of the burn and `keep_alive` is false, the account will be reaped."]
					#[doc = ""]
					#[doc = "Unlike sending funds to a _burn_ address, which merely makes the funds inaccessible,"]
					#[doc = "this `burn` operation will reduce total issuance by the amount _burned_."]
					burn {
						#[codec(compact)]
						value: ::core::primitive::u128,
						keep_alive: ::core::primitive::bool,
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
					#[doc = "Transfer some liquid free balance to another account."]
					#[doc = ""]
					#[doc = "`transfer_allow_death` will set the `FreeBalance` of the sender and receiver."]
					#[doc = "If the sender's account is below the existential deposit as a result"]
					#[doc = "of the transfer, the account will be reaped."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be `Signed` by the transactor."]
					transfer_allow_death {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 2)]
					#[doc = "Exactly as `transfer_allow_death`, except the origin must be root and the source account"]
					#[doc = "may be specified."]
					force_transfer {
						source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 3)]
					#[doc = "Same as the [`transfer_allow_death`] call, but with a check that the transfer will not"]
					#[doc = "kill the origin account."]
					#[doc = ""]
					#[doc = "99% of the time you want [`transfer_allow_death`] instead."]
					#[doc = ""]
					#[doc = "[`transfer_allow_death`]: struct.Pallet.html#method.transfer"]
					transfer_keep_alive {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 4)]
					#[doc = "Transfer the entire transferable balance from the caller account."]
					#[doc = ""]
					#[doc = "NOTE: This function only attempts to transfer _transferable_ balances. This means that"]
					#[doc = "any locked, reserved, or existential deposits (when `keep_alive` is `true`), will not be"]
					#[doc = "transferred by this function. To ensure that this function results in a killed account,"]
					#[doc = "you might need to prepare the account by removing any reference counters, storage"]
					#[doc = "deposits, etc..."]
					#[doc = ""]
					#[doc = "The dispatch origin of this call must be Signed."]
					#[doc = ""]
					#[doc = "- `dest`: The recipient of the transfer."]
					#[doc = "- `keep_alive`: A boolean to determine if the `transfer_all` operation should send all"]
					#[doc = "  of the funds the account has, causing the sender account to be killed (false), or"]
					#[doc = "  transfer everything except at least the existential deposit, which will guarantee to"]
					#[doc = "  keep the sender account alive (true)."]
					transfer_all {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						keep_alive: ::core::primitive::bool,
					},
					#[codec(index = 5)]
					#[doc = "Unreserve some balance from a user by force."]
					#[doc = ""]
					#[doc = "Can only be called by ROOT."]
					force_unreserve {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 6)]
					#[doc = "Upgrade a specified account."]
					#[doc = ""]
					#[doc = "- `origin`: Must be `Signed`."]
					#[doc = "- `who`: The account to be upgraded."]
					#[doc = ""]
					#[doc = "This will waive the transaction fee if at least all but 10% of the accounts needed to"]
					#[doc = "be upgraded. (We let some not have to be upgraded just in order to allow for the"]
					#[doc = "possibility of churn)."]
					upgrade_accounts { who: ::std::vec::Vec<::subxt::utils::AccountId32> },
					#[codec(index = 8)]
					#[doc = "Set the regular balance of a given account."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call is `root`."]
					force_set_balance {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						new_free: ::core::primitive::u128,
					},
					#[codec(index = 9)]
					#[doc = "Adjust the total issuance in a saturating way."]
					#[doc = ""]
					#[doc = "Can only be called by root and always needs a positive `delta`."]
					#[doc = ""]
					#[doc = "# Example"]
					force_adjust_total_issuance {
						direction: runtime_types::pallet_balances::types::AdjustmentDirection,
						#[codec(compact)]
						delta: ::core::primitive::u128,
					},
					#[codec(index = 10)]
					#[doc = "Burn the specified liquid free balance from the origin account."]
					#[doc = ""]
					#[doc = "If the origin's account ends up below the existential deposit as a result"]
					#[doc = "of the burn and `keep_alive` is false, the account will be reaped."]
					#[doc = ""]
					#[doc = "Unlike sending funds to a _burn_ address, which merely makes the funds inaccessible,"]
					#[doc = "this `burn` operation will reduce total issuance by the amount _burned_."]
					burn {
						#[codec(compact)]
						value: ::core::primitive::u128,
						keep_alive: ::core::primitive::bool,
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
					#[doc = "Number of holds exceed `VariantCountOf<T::RuntimeHoldReason>`."]
					TooManyHolds,
					#[codec(index = 9)]
					#[doc = "Number of freezes exceed `MaxFreezes`."]
					TooManyFreezes,
					#[codec(index = 10)]
					#[doc = "The issuance cannot be modified since it is already deactivated."]
					IssuanceDeactivated,
					#[codec(index = 11)]
					#[doc = "The delta cannot be zero."]
					DeltaZero,
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
					#[doc = "Number of holds exceed `VariantCountOf<T::RuntimeHoldReason>`."]
					TooManyHolds,
					#[codec(index = 9)]
					#[doc = "Number of freezes exceed `MaxFreezes`."]
					TooManyFreezes,
					#[codec(index = 10)]
					#[doc = "The issuance cannot be modified since it is already deactivated."]
					IssuanceDeactivated,
					#[codec(index = 11)]
					#[doc = "The delta cannot be zero."]
					DeltaZero,
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
					#[codec(index = 21)]
					#[doc = "The `TotalIssuance` was forcefully changed."]
					TotalIssuanceForced {
						old: ::core::primitive::u128,
						new: ::core::primitive::u128,
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
					#[codec(index = 21)]
					#[doc = "The `TotalIssuance` was forcefully changed."]
					TotalIssuanceForced {
						old: ::core::primitive::u128,
						new: ::core::primitive::u128,
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
				pub enum AdjustmentDirection {
					#[codec(index = 0)]
					Increase,
					#[codec(index = 1)]
					Decrease,
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
		pub mod pallet_bitcoin_utxos {
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
					#[doc = "Submitted when a bitcoin UTXO has been moved or confirmed"]
					sync { utxo_sync: runtime_types::ulx_primitives::inherents::BitcoinUtxoSync },
					#[codec(index = 1)]
					#[doc = "Sets the most recent confirmed bitcoin block height (only executable by the Oracle"]
					#[doc = "Operator account)"]
					#[doc = ""]
					#[doc = "# Arguments"]
					#[doc = "* `bitcoin_height` - the latest bitcoin block height to be confirmed"]
					set_confirmed_block {
						bitcoin_height: ::core::primitive::u64,
						bitcoin_block_hash: runtime_types::ulx_primitives::bitcoin::H256Le,
					},
					#[codec(index = 2)]
					#[doc = "Sets the oracle operator account id (only executable by the Root account)"]
					#[doc = ""]
					#[doc = "# Arguments"]
					#[doc = "* `account_id` - the account id of the operator"]
					set_operator { account_id: ::subxt::utils::AccountId32 },
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
					#[doc = "Only an Oracle Operator can perform this action"]
					NoPermissions,
					#[codec(index = 1)]
					#[doc = "No Oracle-provided bitcoin block has been provided to the network"]
					NoBitcoinConfirmedBlock,
					#[codec(index = 2)]
					#[doc = "Insufficient bitcoin amount"]
					InsufficientBitcoinAmount,
					#[codec(index = 3)]
					#[doc = "No prices are available to mint bitcoins"]
					NoBitcoinPricesAvailable,
					#[codec(index = 4)]
					#[doc = "ScriptPubKey is already being waited for"]
					ScriptPubkeyConflict,
					#[codec(index = 5)]
					#[doc = "Locked Utxo Not Found"]
					UtxoNotLocked,
					#[codec(index = 6)]
					#[doc = "Redemptions not currently available"]
					RedemptionsUnavailable,
					#[codec(index = 7)]
					#[doc = "Invalid bitcoin sync height attempted"]
					InvalidBitcoinSyncHeight,
					#[codec(index = 8)]
					#[doc = "Bitcoin height not confirmed yet"]
					BitcoinHeightNotConfirmed,
					#[codec(index = 9)]
					#[doc = "Too many UTXOs are being tracked"]
					MaxUtxosExceeded,
					#[codec(index = 10)]
					#[doc = "Locking script has errors"]
					InvalidBitcoinScript,
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
					UtxoVerified { utxo_id: ::core::primitive::u64 },
					#[codec(index = 1)]
					UtxoRejected {
						utxo_id: ::core::primitive::u64,
						rejected_reason:
							runtime_types::ulx_primitives::bitcoin::BitcoinRejectedReason,
					},
					#[codec(index = 2)]
					UtxoSpent {
						utxo_id: ::core::primitive::u64,
						block_height: ::core::primitive::u64,
					},
					#[codec(index = 3)]
					UtxoUnwatched { utxo_id: ::core::primitive::u64 },
					#[codec(index = 4)]
					UtxoSpentError {
						utxo_id: ::core::primitive::u64,
						error: runtime_types::sp_runtime::DispatchError,
					},
					#[codec(index = 5)]
					UtxoVerifiedError {
						utxo_id: ::core::primitive::u64,
						error: runtime_types::sp_runtime::DispatchError,
					},
					#[codec(index = 6)]
					UtxoRejectedError {
						utxo_id: ::core::primitive::u64,
						error: runtime_types::sp_runtime::DispatchError,
					},
					#[codec(index = 7)]
					UtxoExpiredError {
						utxo_ref: runtime_types::ulx_primitives::bitcoin::UtxoRef,
						error: runtime_types::sp_runtime::DispatchError,
					},
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
							runtime_types::ulx_primitives::block_seal::BlockPayout<
								::subxt::utils::AccountId32,
								::core::primitive::u128,
							>,
						>,
					},
					#[codec(index = 1)]
					RewardUnlocked {
						rewards: ::std::vec::Vec<
							runtime_types::ulx_primitives::block_seal::BlockPayout<
								::subxt::utils::AccountId32,
								::core::primitive::u128,
							>,
						>,
					},
					#[codec(index = 2)]
					RewardUnlockError {
						account_id: ::subxt::utils::AccountId32,
						argons: ::core::option::Option<::core::primitive::u128>,
						ulixees: ::core::option::Option<::core::primitive::u128>,
						error: runtime_types::sp_runtime::DispatchError,
					},
					#[codec(index = 3)]
					RewardCreateError {
						account_id: ::subxt::utils::AccountId32,
						argons: ::core::option::Option<::core::primitive::u128>,
						ulixees: ::core::option::Option<::core::primitive::u128>,
						error: runtime_types::sp_runtime::DispatchError,
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
					#[doc = "The strength of the given seal did not match calculations"]
					InvalidVoteSealStrength,
					#[codec(index = 1)]
					#[doc = "Vote not submitted by the right miner"]
					InvalidSubmitter,
					#[codec(index = 2)]
					#[doc = "Could not decode the vote bytes"]
					UnableToDecodeVoteAccount,
					#[codec(index = 3)]
					#[doc = "The block author is not a registered miner"]
					UnregisteredBlockAuthor,
					#[codec(index = 4)]
					#[doc = "The merkle proof of vote inclusion in the notebook is invalid"]
					InvalidBlockVoteProof,
					#[codec(index = 5)]
					#[doc = "No vote minimum found at grandparent height"]
					NoGrandparentVoteMinimum,
					#[codec(index = 6)]
					#[doc = "Too many block seals submitted"]
					DuplicateBlockSealProvided,
					#[codec(index = 7)]
					#[doc = "The block vote did not reach the minimum voting power at time of the grandparent block"]
					InsufficientVotingPower,
					#[codec(index = 8)]
					#[doc = "No registered voting key found for the parent block"]
					ParentVotingKeyNotFound,
					#[codec(index = 9)]
					#[doc = "The block vote was not for a valid block"]
					InvalidVoteGrandparentHash,
					#[codec(index = 10)]
					#[doc = "The notebook for this vote was not eligible to vote"]
					IneligibleNotebookUsed,
					#[codec(index = 11)]
					#[doc = "The lookup to verify a vote's authenticity is not available for the given block"]
					NoEligibleVotingRoot,
					#[codec(index = 12)]
					#[doc = "The data domain was not registered"]
					UnregisteredDataDomain,
					#[codec(index = 13)]
					#[doc = "The data domain account is mismatched with the block reward seeker"]
					InvalidDataDomainAccount,
					#[codec(index = 14)]
					#[doc = "Message was not signed by a registered miner"]
					InvalidAuthoritySignature,
					#[codec(index = 15)]
					#[doc = "Could not decode the scale bytes of the votes"]
					CouldNotDecodeVote,
					#[codec(index = 16)]
					#[doc = "Too many notebooks were submitted for the current tick. Should not be possible"]
					MaxNotebooksAtTickExceeded,
					#[codec(index = 17)]
					#[doc = "No closest miner found for vote"]
					NoClosestMinerFoundForVote,
					#[codec(index = 18)]
					#[doc = "The vote signature was invalid"]
					BlockVoteInvalidSignature,
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
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "The maximum number of notebooks at the current tick has been exceeded"]
					MaxNotebooksAtTickExceeded,
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
					#[doc = "Bond a bitcoin. This will create a bond for the submitting account and log the Bitcoin"]
					#[doc = "Script hash to Events. A bondee must create the UTXO in order to be added to the Bitcoin"]
					#[doc = "Mint line."]
					#[doc = ""]
					#[doc = "NOTE: The script"]
					bond_bitcoin {
						vault_id: ::core::primitive::u32,
						#[codec(compact)]
						satoshis: ::core::primitive::u64,
						bitcoin_pubkey_hash:
							runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash,
					},
					#[codec(index = 4)]
					#[doc = "Submitted by a Bitcoin holder to trigger the unlock of their Bitcoin. A transaction"]
					#[doc = "spending the UTXO from the given bond should be pre-created so that the sighash can be"]
					#[doc = "submitted here. The vault operator will have 10 days to counter-sign the transaction. It"]
					#[doc = "will be published with the public key as a BitcoinUtxoCosigned Event."]
					#[doc = ""]
					#[doc = "Owner must submit a script pubkey and also a fee to pay to the bitcoin network."]
					unlock_bitcoin_bond {
						bond_id: ::core::primitive::u64,
						to_script_pubkey:
							runtime_types::ulx_primitives::bitcoin::BitcoinScriptPubkey,
						bitcoin_network_fee: ::core::primitive::u64,
					},
					#[codec(index = 5)]
					#[doc = "Submitted by a Vault operator to cosign the unlock of a bitcoin utxo. The Bitcoin owner"]
					#[doc = "unlock fee will be burned, and the bond will be allowed to expire without penalty."]
					#[doc = ""]
					#[doc = "This is submitted as a no-fee transaction off chain to allow keys to remain in cold"]
					#[doc = "wallets."]
					cosign_bitcoin_unlock {
						bond_id: ::core::primitive::u64,
						pubkey: runtime_types::ulx_primitives::bitcoin::CompressedBitcoinPubkey,
						signature: runtime_types::ulx_primitives::bitcoin::BitcoinSignature,
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
					BondNotFound,
					#[codec(index = 1)]
					NoMoreBondIds,
					#[codec(index = 2)]
					MinimumBondAmountNotMet,
					#[codec(index = 3)]
					#[doc = "There are too many bond or bond funds expiring in the given expiration block"]
					ExpirationAtBlockOverflow,
					#[codec(index = 4)]
					InsufficientFunds,
					#[codec(index = 5)]
					InsufficientVaultFunds,
					#[codec(index = 6)]
					#[doc = "The vault does not have enough bitcoins to cover the mining bond"]
					InsufficientBitcoinsForMining,
					#[codec(index = 7)]
					#[doc = "The proposed transaction would take the account below the minimum (existential) balance"]
					AccountWouldGoBelowMinimumBalance,
					#[codec(index = 8)]
					VaultClosed,
					#[codec(index = 9)]
					#[doc = "Funding would result in an overflow of the balance type"]
					InvalidVaultAmount,
					#[codec(index = 10)]
					#[doc = "This bitcoin redemption has not been locked in"]
					BondRedemptionNotLocked,
					#[codec(index = 11)]
					#[doc = "The bitcoin has passed the deadline to unlock it"]
					BitcoinUnlockInitiationDeadlinePassed,
					#[codec(index = 12)]
					#[doc = "The fee for this bitcoin unlock is too high"]
					BitcoinFeeTooHigh,
					#[codec(index = 13)]
					InvalidBondType,
					#[codec(index = 14)]
					BitcoinUtxoNotFound,
					#[codec(index = 15)]
					InsufficientSatoshisBonded,
					#[codec(index = 16)]
					NoBitcoinPricesAvailable,
					#[codec(index = 17)]
					#[doc = "The bitcoin script to lock this bitcoin has errors"]
					InvalidBitcoinScript,
					#[codec(index = 18)]
					ExpirationTooSoon,
					#[codec(index = 19)]
					NoPermissions,
					#[codec(index = 20)]
					HoldUnexpectedlyModified,
					#[codec(index = 21)]
					UnrecoverableHold,
					#[codec(index = 22)]
					VaultNotFound,
					#[codec(index = 23)]
					#[doc = "The fee for this bond exceeds the amount of the bond, which is unsafe"]
					FeeExceedsBondAmount,
					#[codec(index = 24)]
					GenericBondError(runtime_types::ulx_primitives::bond::BondError),
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
					BondCreated {
						vault_id: ::core::primitive::u32,
						bond_id: ::core::primitive::u64,
						bond_type: runtime_types::ulx_primitives::bond::BondType,
						bonded_account_id: ::subxt::utils::AccountId32,
						utxo_id: ::core::option::Option<::core::primitive::u64>,
						amount: ::core::primitive::u128,
						expiration: runtime_types::ulx_primitives::bond::BondExpiration<
							::core::primitive::u32,
						>,
					},
					#[codec(index = 1)]
					BondCompleted {
						vault_id: ::core::primitive::u32,
						bond_id: ::core::primitive::u64,
					},
					#[codec(index = 2)]
					BondCanceled {
						vault_id: ::core::primitive::u32,
						bond_id: ::core::primitive::u64,
						bonded_account_id: ::subxt::utils::AccountId32,
						bond_type: runtime_types::ulx_primitives::bond::BondType,
						returned_fee: ::core::primitive::u128,
					},
					#[codec(index = 3)]
					BitcoinBondBurned {
						vault_id: ::core::primitive::u32,
						bond_id: ::core::primitive::u64,
						utxo_id: ::core::primitive::u64,
						amount_burned: ::core::primitive::u128,
						amount_held: ::core::primitive::u128,
						was_utxo_spent: ::core::primitive::bool,
					},
					#[codec(index = 4)]
					BitcoinUtxoCosignRequested {
						bond_id: ::core::primitive::u64,
						vault_id: ::core::primitive::u32,
						utxo_id: ::core::primitive::u64,
					},
					#[codec(index = 5)]
					BitcoinUtxoCosigned {
						bond_id: ::core::primitive::u64,
						vault_id: ::core::primitive::u32,
						utxo_id: ::core::primitive::u64,
						pubkey: runtime_types::ulx_primitives::bitcoin::CompressedBitcoinPubkey,
						signature: runtime_types::ulx_primitives::bitcoin::BitcoinSignature,
					},
					#[codec(index = 6)]
					BitcoinCosignPastDue {
						bond_id: ::core::primitive::u64,
						vault_id: ::core::primitive::u32,
						utxo_id: ::core::primitive::u64,
						compensation_amount: ::core::primitive::u128,
						compensation_still_owed: ::core::primitive::u128,
						compensated_account_id: ::subxt::utils::AccountId32,
					},
					#[codec(index = 7)]
					#[doc = "An error occurred while completing a bond"]
					BondCompletionError {
						bond_id: ::core::primitive::u64,
						error: runtime_types::sp_runtime::DispatchError,
					},
					#[codec(index = 8)]
					#[doc = "An error occurred while refunding an overdue cosigned bitcoin bond"]
					CosignOverdueError {
						utxo_id: ::core::primitive::u64,
						error: runtime_types::sp_runtime::DispatchError,
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
					UnlockingBitcoin,
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
				pub struct UtxoCosignRequest<_0> {
					pub bitcoin_network_fee: ::core::primitive::u64,
					pub cosign_due_block: ::core::primitive::u64,
					pub to_script_pubkey:
						runtime_types::ulx_primitives::bitcoin::BitcoinScriptPubkey,
					pub redemption_price: _0,
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
				pub struct UtxoState {
					pub bond_id: ::core::primitive::u64,
					pub satoshis: ::core::primitive::u64,
					pub vault_pubkey_hash:
						runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash,
					pub owner_pubkey_hash:
						runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash,
					pub vault_claim_height: ::core::primitive::u64,
					pub open_claim_height: ::core::primitive::u64,
					pub register_block: ::core::primitive::u64,
					pub utxo_script_pubkey:
						runtime_types::ulx_primitives::bitcoin::BitcoinCosignScriptPubkey,
					pub is_verified: ::core::primitive::bool,
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
					send_to_localchain {
						#[codec(compact)]
						amount: ::core::primitive::u128,
						notary_id: ::core::primitive::u32,
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
					#[doc = "Insufficient balance to fulfill a mainchain transfer"]
					InsufficientNotarizedFunds,
					#[codec(index = 3)]
					#[doc = "The transfer was already submitted in a previous block"]
					InvalidOrDuplicatedLocalchainTransfer,
					#[codec(index = 4)]
					#[doc = "A transfer was submitted in a previous block but the expiration block has passed"]
					NotebookIncludesExpiredLocalchainTransfer,
					#[codec(index = 5)]
					#[doc = "The notary id is not registered"]
					InvalidNotaryUsedForTransfer,
					#[codec(index = 6)]
					#[doc = "The notary is locked (likey due to failed audits)"]
					NotaryLocked,
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
						transfer_id: ::core::primitive::u32,
						notary_id: ::core::primitive::u32,
						expiration_tick: ::core::primitive::u32,
					},
					#[codec(index = 1)]
					TransferToLocalchainExpired {
						account_id: ::subxt::utils::AccountId32,
						transfer_id: ::core::primitive::u32,
						notary_id: ::core::primitive::u32,
					},
					#[codec(index = 2)]
					TransferIn {
						account_id: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						notary_id: ::core::primitive::u32,
					},
					#[codec(index = 3)]
					#[doc = "A transfer into the mainchain failed"]
					TransferInError {
						account_id: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						notary_id: ::core::primitive::u32,
						notebook_number: ::core::primitive::u32,
						error: runtime_types::sp_runtime::DispatchError,
					},
					#[codec(index = 4)]
					#[doc = "An expired transfer to localchain failed to be refunded"]
					TransferToLocalchainRefundError {
						account_id: ::subxt::utils::AccountId32,
						transfer_id: ::core::primitive::u32,
						notary_id: ::core::primitive::u32,
						notebook_number: ::core::primitive::u32,
						error: runtime_types::sp_runtime::DispatchError,
					},
					#[codec(index = 5)]
					#[doc = "A localchain transfer could not be cleaned up properly. Possible invalid transfer"]
					#[doc = "needing investigation."]
					PossibleInvalidTransferAllowed {
						transfer_id: ::core::primitive::u32,
						notary_id: ::core::primitive::u32,
						notebook_number: ::core::primitive::u32,
					},
					#[codec(index = 6)]
					#[doc = "Taxation failed"]
					TaxationError {
						notary_id: ::core::primitive::u32,
						notebook_number: ::core::primitive::u32,
						tax: ::core::primitive::u128,
						error: runtime_types::sp_runtime::DispatchError,
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
				pub account_id: _0,
				pub amount: _1,
				pub expiration_tick: ::core::primitive::u32,
				pub notary_id: ::core::primitive::u32,
			}
		}
		pub mod pallet_data_domain {
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
					set_zone_record {
						domain_hash: ::sp_core::H256,
						zone_record: runtime_types::ulx_primitives::data_domain::ZoneRecord<
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
					#[doc = "The domain is not registered."]
					DomainNotRegistered,
					#[codec(index = 1)]
					#[doc = "The sender is not the owner of the domain."]
					NotDomainOwner,
					#[codec(index = 2)]
					#[doc = "Failed to add to the address history."]
					FailedToAddToAddressHistory,
					#[codec(index = 3)]
					#[doc = "Failed to add to the expiring domain list"]
					FailedToAddExpiringDomain,
					#[codec(index = 4)]
					#[doc = "Error decoding account from notary"]
					AccountDecodingError,
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
					#[doc = "A data domain zone record was updated"]
					ZoneRecordUpdated {
						domain_hash: ::sp_core::H256,
						zone_record: runtime_types::ulx_primitives::data_domain::ZoneRecord<
							::subxt::utils::AccountId32,
						>,
					},
					#[codec(index = 1)]
					#[doc = "A data domain was registered"]
					DataDomainRegistered {
						domain_hash: ::sp_core::H256,
						registration: runtime_types::pallet_data_domain::DataDomainRegistration<
							::subxt::utils::AccountId32,
						>,
					},
					#[codec(index = 2)]
					#[doc = "A data domain was registered"]
					DataDomainRenewed { domain_hash: ::sp_core::H256 },
					#[codec(index = 3)]
					#[doc = "A data domain was expired"]
					DataDomainExpired { domain_hash: ::sp_core::H256 },
					#[codec(index = 4)]
					#[doc = "A data domain registration was canceled due to a conflicting registration in the same"]
					#[doc = "tick"]
					DataDomainRegistrationCanceled {
						domain_hash: ::sp_core::H256,
						registration: runtime_types::pallet_data_domain::DataDomainRegistration<
							::subxt::utils::AccountId32,
						>,
					},
					#[codec(index = 5)]
					#[doc = "A data domain registration failed due to an error"]
					DataDomainRegistrationError {
						domain_hash: ::sp_core::H256,
						account_id: ::subxt::utils::AccountId32,
						error: runtime_types::sp_runtime::DispatchError,
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
			pub struct DataDomainRegistration<_0> {
				pub account_id: _0,
				pub registered_at_tick: ::core::primitive::u32,
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
					#[doc = "Report voter equivocation/misbehavior. This method will verify the"]
					#[doc = "equivocation proof and validate the given key ownership proof"]
					#[doc = "against the extracted offender. If both are valid, the offence"]
					#[doc = "will be reported."]
					report_equivocation {
						equivocation_proof: ::std::boxed::Box<
							runtime_types::sp_consensus_grandpa::EquivocationProof<
								::sp_core::H256,
								::core::primitive::u32,
							>,
						>,
						key_owner_proof: runtime_types::sp_session::MembershipProof,
					},
					#[codec(index = 1)]
					#[doc = "Report voter equivocation/misbehavior. This method will verify the"]
					#[doc = "equivocation proof and validate the given key ownership proof"]
					#[doc = "against the extracted offender. If both are valid, the offence"]
					#[doc = "will be reported."]
					#[doc = ""]
					#[doc = "This extrinsic must be called unsigned and it is expected that only"]
					#[doc = "block authors will call it (validated in `ValidateUnsigned`), as such"]
					#[doc = "if the block author is defined it will be defined as the equivocation"]
					#[doc = "reporter."]
					report_equivocation_unsigned {
						equivocation_proof: ::std::boxed::Box<
							runtime_types::sp_consensus_grandpa::EquivocationProof<
								::sp_core::H256,
								::core::primitive::u32,
							>,
						>,
						key_owner_proof: runtime_types::sp_session::MembershipProof,
					},
					#[codec(index = 2)]
					#[doc = "Note that the current authority set of the GRANDPA finality gadget has stalled."]
					#[doc = ""]
					#[doc = "This will trigger a forced authority set change at the beginning of the next session, to"]
					#[doc = "be enacted `delay` blocks after that. The `delay` should be high enough to safely assume"]
					#[doc = "that the block signalling the forced change will not be re-orged e.g. 1000 blocks."]
					#[doc = "The block production rate (which may be slowed down because of finality lagging) should"]
					#[doc = "be taken into account when choosing the `delay`. The GRANDPA voters based on the new"]
					#[doc = "authority will start voting on top of `best_finalized_block_number` for new finalized"]
					#[doc = "blocks. `best_finalized_block_number` should be the highest of the latest finalized"]
					#[doc = "block of all validators of the new authority set."]
					#[doc = ""]
					#[doc = "Only callable by root."]
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
					#[doc = "Submit a bid for a mining slot in the next cohort. Once all spots are filled in a slot,"]
					#[doc = "a slot can be supplanted by supplying a higher mining bond amount. Bond terms can be"]
					#[doc = "found in the `vaults` pallet. You will supply the bond amount and the vault id to bond"]
					#[doc = "with."]
					#[doc = ""]
					#[doc = "Each slot has `MaxCohortSize` spots available."]
					#[doc = ""]
					#[doc = "To be eligible for a slot, you must have the required ownership tokens in this account."]
					#[doc = "The required amount is calculated as a percentage of the total ownership tokens in the"]
					#[doc = "network. This percentage is adjusted before the beginning of each slot."]
					#[doc = ""]
					#[doc = "If your bid is replaced, a `SlotBidderReplaced` event will be emitted. By monitoring for"]
					#[doc = "this event, you will be able to ensure your bid is accepted."]
					#[doc = ""]
					#[doc = "NOTE: bidding for each slot will be closed at a random block within"]
					#[doc = "`BlocksBeforeBidEndForVrfClose` blocks of the slot end time."]
					#[doc = ""]
					#[doc = "The slot duration can be calculated as `BlocksBetweenSlots * MaxMiners / MaxCohortSize`."]
					#[doc = ""]
					#[doc = "Parameters:"]
					#[doc = "- `bond_info`: The bond information to submit for the bid. If `None`, the bid will be"]
					#[doc = " considered a zero-bid."]
					#[doc = "\t- `vault_id`: The vault id to bond with. Terms are taken from the vault at time of bid"]
					#[doc = "   inclusion in the block."]
					#[doc = "  \t- `amount`: The amount to bond with the vault."]
					#[doc = "- `reward_destination`: The account_id for the mining rewards, or `Owner` for the"]
					#[doc = "  submitting user."]
					bid {
						bond_info: ::core::option::Option<
							runtime_types::pallet_mining_slot::MiningSlotBid<
								::core::primitive::u32,
								::core::primitive::u128,
							>,
						>,
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
					InsufficientOwnershipTokens,
					#[codec(index = 3)]
					BidTooLow,
					#[codec(index = 4)]
					#[doc = "A Non-Mining bond was submitted as part of a bid"]
					CannotRegisterOverlappingSessions,
					#[codec(index = 5)]
					BondNotFound,
					#[codec(index = 6)]
					NoMoreBondIds,
					#[codec(index = 7)]
					VaultClosed,
					#[codec(index = 8)]
					MinimumBondAmountNotMet,
					#[codec(index = 9)]
					#[doc = "There are too many bond or bond funds expiring in the given expiration block"]
					ExpirationAtBlockOverflow,
					#[codec(index = 10)]
					InsufficientFunds,
					#[codec(index = 11)]
					InsufficientVaultFunds,
					#[codec(index = 12)]
					ExpirationTooSoon,
					#[codec(index = 13)]
					NoPermissions,
					#[codec(index = 14)]
					HoldUnexpectedlyModified,
					#[codec(index = 15)]
					UnrecoverableHold,
					#[codec(index = 16)]
					VaultNotFound,
					#[codec(index = 17)]
					BondAlreadyClosed,
					#[codec(index = 18)]
					#[doc = "The fee for this bond exceeds the amount of the bond, which is unsafe"]
					FeeExceedsBondAmount,
					#[codec(index = 19)]
					AccountWouldBeBelowMinimum,
					#[codec(index = 20)]
					GenericBondError(runtime_types::ulx_primitives::bond::BondError),
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
					#[codec(index = 4)]
					UnbondMinerError {
						account_id: ::subxt::utils::AccountId32,
						bond_id: ::core::option::Option<::core::primitive::u64>,
						error: runtime_types::sp_runtime::DispatchError,
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
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct MiningSlotBid<_0, _1> {
				pub vault_id: _0,
				pub amount: _1,
			}
		}
		pub mod pallet_mint {
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
				pub enum Error {
					#[codec(index = 0)]
					TooManyPendingMints,
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
					ArgonsMinted {
						mint_type: runtime_types::pallet_mint::pallet::MintType,
						account_id: ::subxt::utils::AccountId32,
						utxo_id: ::core::option::Option<::core::primitive::u64>,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 1)]
					MintError {
						mint_type: runtime_types::pallet_mint::pallet::MintType,
						account_id: ::subxt::utils::AccountId32,
						utxo_id: ::core::option::Option<::core::primitive::u64>,
						amount: ::core::primitive::u128,
						error: runtime_types::sp_runtime::DispatchError,
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
				pub enum MintType {
					#[codec(index = 0)]
					Bitcoin,
					#[codec(index = 1)]
					Ulixee,
				}
			}
		}
		pub mod pallet_multisig {
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
					#[doc = "Immediately dispatch a multi-signature call using a single approval from the caller."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					#[doc = ""]
					#[doc = "- `other_signatories`: The accounts (other than the sender) who are part of the"]
					#[doc = "multi-signature, but do not participate in the approval process."]
					#[doc = "- `call`: The call to be executed."]
					#[doc = ""]
					#[doc = "Result is equivalent to the dispatched result."]
					#[doc = ""]
					#[doc = "## Complexity"]
					#[doc = "O(Z + C) where Z is the length of the call and C its execution weight."]
					as_multi_threshold_1 {
						other_signatories: ::std::vec::Vec<::subxt::utils::AccountId32>,
						call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall>,
					},
					#[codec(index = 1)]
					#[doc = "Register approval for a dispatch to be made from a deterministic composite account if"]
					#[doc = "approved by a total of `threshold - 1` of `other_signatories`."]
					#[doc = ""]
					#[doc = "If there are enough, then dispatch the call."]
					#[doc = ""]
					#[doc = "Payment: `DepositBase` will be reserved if this is the first approval, plus"]
					#[doc = "`threshold` times `DepositFactor`. It is returned once this dispatch happens or"]
					#[doc = "is cancelled."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					#[doc = ""]
					#[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
					#[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
					#[doc = "dispatch. May not be empty."]
					#[doc = "- `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is"]
					#[doc = "not the first approval, then it must be `Some`, with the timepoint (block number and"]
					#[doc = "transaction index) of the first approval transaction."]
					#[doc = "- `call`: The call to be executed."]
					#[doc = ""]
					#[doc = "NOTE: Unless this is the final approval, you will generally want to use"]
					#[doc = "`approve_as_multi` instead, since it only requires a hash of the call."]
					#[doc = ""]
					#[doc = "Result is equivalent to the dispatched result if `threshold` is exactly `1`. Otherwise"]
					#[doc = "on success, result is `Ok` and the result from the interior call, if it was executed,"]
					#[doc = "may be found in the deposited `MultisigExecuted` event."]
					#[doc = ""]
					#[doc = "## Complexity"]
					#[doc = "- `O(S + Z + Call)`."]
					#[doc = "- Up to one balance-reserve or unreserve operation."]
					#[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
					#[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
					#[doc = "- One call encode & hash, both of complexity `O(Z)` where `Z` is tx-len."]
					#[doc = "- One encode & hash, both of complexity `O(S)`."]
					#[doc = "- Up to one binary search and insert (`O(logS + S)`)."]
					#[doc = "- I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove."]
					#[doc = "- One event."]
					#[doc = "- The weight of the `call`."]
					#[doc = "- Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit"]
					#[doc = "  taken for its lifetime of `DepositBase + threshold * DepositFactor`."]
					as_multi {
						threshold: ::core::primitive::u16,
						other_signatories: ::std::vec::Vec<::subxt::utils::AccountId32>,
						maybe_timepoint: ::core::option::Option<
							runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
						>,
						call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall>,
						max_weight: runtime_types::sp_weights::weight_v2::Weight,
					},
					#[codec(index = 2)]
					#[doc = "Register approval for a dispatch to be made from a deterministic composite account if"]
					#[doc = "approved by a total of `threshold - 1` of `other_signatories`."]
					#[doc = ""]
					#[doc = "Payment: `DepositBase` will be reserved if this is the first approval, plus"]
					#[doc = "`threshold` times `DepositFactor`. It is returned once this dispatch happens or"]
					#[doc = "is cancelled."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					#[doc = ""]
					#[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
					#[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
					#[doc = "dispatch. May not be empty."]
					#[doc = "- `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is"]
					#[doc = "not the first approval, then it must be `Some`, with the timepoint (block number and"]
					#[doc = "transaction index) of the first approval transaction."]
					#[doc = "- `call_hash`: The hash of the call to be executed."]
					#[doc = ""]
					#[doc = "NOTE: If this is the final approval, you will want to use `as_multi` instead."]
					#[doc = ""]
					#[doc = "## Complexity"]
					#[doc = "- `O(S)`."]
					#[doc = "- Up to one balance-reserve or unreserve operation."]
					#[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
					#[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
					#[doc = "- One encode & hash, both of complexity `O(S)`."]
					#[doc = "- Up to one binary search and insert (`O(logS + S)`)."]
					#[doc = "- I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove."]
					#[doc = "- One event."]
					#[doc = "- Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit"]
					#[doc = "  taken for its lifetime of `DepositBase + threshold * DepositFactor`."]
					approve_as_multi {
						threshold: ::core::primitive::u16,
						other_signatories: ::std::vec::Vec<::subxt::utils::AccountId32>,
						maybe_timepoint: ::core::option::Option<
							runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
						>,
						call_hash: [::core::primitive::u8; 32usize],
						max_weight: runtime_types::sp_weights::weight_v2::Weight,
					},
					#[codec(index = 3)]
					#[doc = "Cancel a pre-existing, on-going multisig transaction. Any deposit reserved previously"]
					#[doc = "for this operation will be unreserved on success."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					#[doc = ""]
					#[doc = "- `threshold`: The total number of approvals for this dispatch before it is executed."]
					#[doc = "- `other_signatories`: The accounts (other than the sender) who can approve this"]
					#[doc = "dispatch. May not be empty."]
					#[doc = "- `timepoint`: The timepoint (block number and transaction index) of the first approval"]
					#[doc = "transaction for this dispatch."]
					#[doc = "- `call_hash`: The hash of the call to be executed."]
					#[doc = ""]
					#[doc = "## Complexity"]
					#[doc = "- `O(S)`."]
					#[doc = "- Up to one balance-reserve or unreserve operation."]
					#[doc = "- One passthrough operation, one insert, both `O(S)` where `S` is the number of"]
					#[doc = "  signatories. `S` is capped by `MaxSignatories`, with weight being proportional."]
					#[doc = "- One encode & hash, both of complexity `O(S)`."]
					#[doc = "- One event."]
					#[doc = "- I/O: 1 read `O(S)`, one remove."]
					#[doc = "- Storage: removes one item."]
					cancel_as_multi {
						threshold: ::core::primitive::u16,
						other_signatories: ::std::vec::Vec<::subxt::utils::AccountId32>,
						timepoint:
							runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
						call_hash: [::core::primitive::u8; 32usize],
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
					#[doc = "Threshold must be 2 or greater."]
					MinimumThreshold,
					#[codec(index = 1)]
					#[doc = "Call is already approved by this signatory."]
					AlreadyApproved,
					#[codec(index = 2)]
					#[doc = "Call doesn't need any (more) approvals."]
					NoApprovalsNeeded,
					#[codec(index = 3)]
					#[doc = "There are too few signatories in the list."]
					TooFewSignatories,
					#[codec(index = 4)]
					#[doc = "There are too many signatories in the list."]
					TooManySignatories,
					#[codec(index = 5)]
					#[doc = "The signatories were provided out of order; they should be ordered."]
					SignatoriesOutOfOrder,
					#[codec(index = 6)]
					#[doc = "The sender was contained in the other signatories; it shouldn't be."]
					SenderInSignatories,
					#[codec(index = 7)]
					#[doc = "Multisig operation not found when attempting to cancel."]
					NotFound,
					#[codec(index = 8)]
					#[doc = "Only the account that originally created the multisig is able to cancel it."]
					NotOwner,
					#[codec(index = 9)]
					#[doc = "No timepoint was given, yet the multisig operation is already underway."]
					NoTimepoint,
					#[codec(index = 10)]
					#[doc = "A different timepoint was given to the multisig operation that is underway."]
					WrongTimepoint,
					#[codec(index = 11)]
					#[doc = "A timepoint was given, yet no multisig operation is underway."]
					UnexpectedTimepoint,
					#[codec(index = 12)]
					#[doc = "The maximum weight information provided was too low."]
					MaxWeightTooLow,
					#[codec(index = 13)]
					#[doc = "The data to be stored is already stored."]
					AlreadyStored,
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
					#[doc = "A new multisig operation has begun."]
					NewMultisig {
						approving: ::subxt::utils::AccountId32,
						multisig: ::subxt::utils::AccountId32,
						call_hash: [::core::primitive::u8; 32usize],
					},
					#[codec(index = 1)]
					#[doc = "A multisig operation has been approved by someone."]
					MultisigApproval {
						approving: ::subxt::utils::AccountId32,
						timepoint:
							runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
						multisig: ::subxt::utils::AccountId32,
						call_hash: [::core::primitive::u8; 32usize],
					},
					#[codec(index = 2)]
					#[doc = "A multisig operation has been executed."]
					MultisigExecuted {
						approving: ::subxt::utils::AccountId32,
						timepoint:
							runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
						multisig: ::subxt::utils::AccountId32,
						call_hash: [::core::primitive::u8; 32usize],
						result:
							::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
					},
					#[codec(index = 3)]
					#[doc = "A multisig operation has been cancelled."]
					MultisigCancelled {
						cancelling: ::subxt::utils::AccountId32,
						timepoint:
							runtime_types::pallet_multisig::Timepoint<::core::primitive::u32>,
						multisig: ::subxt::utils::AccountId32,
						call_hash: [::core::primitive::u8; 32usize],
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
			pub struct Multisig<_0, _1, _2> {
				pub when: runtime_types::pallet_multisig::Timepoint<_0>,
				pub deposit: _1,
				pub depositor: _2,
				pub approvals: runtime_types::bounded_collections::bounded_vec::BoundedVec<_2>,
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
			pub struct Timepoint<_0> {
				pub height: _0,
				pub index: ::core::primitive::u32,
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
					propose { meta: runtime_types::ulx_primitives::notary::NotaryMeta },
					#[codec(index = 1)]
					activate { operator_account: ::subxt::utils::AccountId32 },
					#[codec(index = 2)]
					#[doc = "Update the metadata of a notary, to be effective at the given tick height, which must be"]
					#[doc = ">= MetaChangesTickDelay ticks in the future."]
					update {
						#[codec(compact)]
						notary_id: ::core::primitive::u32,
						meta: runtime_types::ulx_primitives::notary::NotaryMeta,
						#[codec(compact)]
						effective_tick: ::core::primitive::u32,
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
					#[doc = "The proposal to activate was not found"]
					ProposalNotFound,
					#[codec(index = 1)]
					#[doc = "Maximum number of notaries exceeded"]
					MaxNotariesExceeded,
					#[codec(index = 2)]
					#[doc = "Maximum number of proposals per block exceeded"]
					MaxProposalsPerBlockExceeded,
					#[codec(index = 3)]
					#[doc = "This notary is not active, so this change cannot be made yet"]
					NotAnActiveNotary,
					#[codec(index = 4)]
					#[doc = "Invalid notary operator for this operation"]
					InvalidNotaryOperator,
					#[codec(index = 5)]
					#[doc = "An internal error has occurred. The notary ids are exhausted."]
					NoMoreNotaryIds,
					#[codec(index = 6)]
					#[doc = "The proposed effective tick is too soon"]
					EffectiveTickTooSoon,
					#[codec(index = 7)]
					#[doc = "Too many internal keys"]
					TooManyKeys,
					#[codec(index = 8)]
					#[doc = "The notary is invalid"]
					InvalidNotary,
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
						effective_tick: ::core::primitive::u32,
					},
					#[codec(index = 3)]
					#[doc = "Notary metadata updated"]
					NotaryMetaUpdated {
						notary_id: ::core::primitive::u32,
						meta: runtime_types::ulx_primitives::notary::NotaryMeta,
					},
					#[codec(index = 4)]
					#[doc = "Error updating queued notary info"]
					NotaryMetaUpdateError {
						notary_id: ::core::primitive::u32,
						error: runtime_types::sp_runtime::DispatchError,
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
					submit {
						notebooks: ::std::vec::Vec<
							runtime_types::ulx_primitives::notebook::SignedNotebookHeader,
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
					#[doc = "This notebook has already been submitted"]
					DuplicateNotebookNumber,
					#[codec(index = 1)]
					#[doc = "Notebooks received out of order"]
					MissingNotebookNumber,
					#[codec(index = 2)]
					#[doc = "A notebook was already provided at this tick"]
					NotebookTickAlreadyUsed,
					#[codec(index = 3)]
					#[doc = "The signature of the notebook is invalid"]
					InvalidNotebookSignature,
					#[codec(index = 4)]
					#[doc = "The secret or secret hash of the parent notebook do not match"]
					InvalidSecretProvided,
					#[codec(index = 5)]
					#[doc = "Could not decode the scale bytes of the notebook"]
					CouldNotDecodeNotebook,
					#[codec(index = 6)]
					#[doc = "The notebook digest was included more than once"]
					DuplicateNotebookDigest,
					#[codec(index = 7)]
					#[doc = "The notebook digest was not included"]
					MissingNotebookDigest,
					#[codec(index = 8)]
					#[doc = "The notebook digest did not match the included notebooks"]
					InvalidNotebookDigest,
					#[codec(index = 9)]
					#[doc = "Multiple inherents provided"]
					MultipleNotebookInherentsProvided,
					#[codec(index = 10)]
					#[doc = "Unable to track the notebook change list"]
					InternalError,
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
					#[codec(index = 1)]
					NotebookAuditFailure {
						notary_id: ::core::primitive::u32,
						notebook_number: ::core::primitive::u32,
						first_failure_reason: runtime_types::ulx_notary_audit::error::VerifyError,
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
		pub mod pallet_price_index {
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
					#[doc = "Submit the latest price index. Only valid for the configured operator account"]
					submit { index: runtime_types::pallet_price_index::PriceIndex },
					#[codec(index = 1)]
					#[doc = "Sets the operator account id (only executable by the Root account)"]
					#[doc = ""]
					#[doc = "# Arguments"]
					#[doc = "* `account_id` - the account id of the operator"]
					set_operator { account_id: ::subxt::utils::AccountId32 },
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
					#[doc = "Not authorized as an oracle operator"]
					NotAuthorizedOperator,
					#[codec(index = 1)]
					#[doc = "Missing value"]
					MissingValue,
					#[codec(index = 2)]
					#[doc = "The submitted prices are too old"]
					PricesTooOld,
					#[codec(index = 3)]
					#[doc = "Change in argon price is too large"]
					MaxPriceChangePerTickExceeded,
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
					#[doc = "Event emitted when a new price index is submitted"]
					NewIndex,
					#[codec(index = 1)]
					OperatorChanged { operator_id: ::subxt::utils::AccountId32 },
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
			pub struct PriceIndex {
				#[codec(compact)]
				pub btc_usd_price: runtime_types::sp_arithmetic::fixed_point::FixedU128,
				#[codec(compact)]
				pub argon_usd_price: runtime_types::sp_arithmetic::fixed_point::FixedU128,
				pub argon_usd_target_price: runtime_types::sp_arithmetic::fixed_point::FixedU128,
				#[codec(compact)]
				pub tick: ::core::primitive::u32,
			}
		}
		pub mod pallet_proxy {
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
					#[doc = "Dispatch the given `call` from an account that the sender is authorised for through"]
					#[doc = "`add_proxy`."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					#[doc = ""]
					#[doc = "Parameters:"]
					#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
					#[doc = "- `force_proxy_type`: Specify the exact proxy type to be used and checked for this call."]
					#[doc = "- `call`: The call to be made by the `real` account."]
					proxy {
						real: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						force_proxy_type:
							::core::option::Option<runtime_types::ulx_node_runtime::ProxyType>,
						call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall>,
					},
					#[codec(index = 1)]
					#[doc = "Register a proxy account for the sender that is able to make calls on its behalf."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					#[doc = ""]
					#[doc = "Parameters:"]
					#[doc = "- `proxy`: The account that the `caller` would like to make a proxy."]
					#[doc = "- `proxy_type`: The permissions allowed for this proxy account."]
					#[doc = "- `delay`: The announcement period required of the initial proxy. Will generally be"]
					#[doc = "zero."]
					add_proxy {
						delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						proxy_type: runtime_types::ulx_node_runtime::ProxyType,
						delay: ::core::primitive::u32,
					},
					#[codec(index = 2)]
					#[doc = "Unregister a proxy account for the sender."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					#[doc = ""]
					#[doc = "Parameters:"]
					#[doc = "- `proxy`: The account that the `caller` would like to remove as a proxy."]
					#[doc = "- `proxy_type`: The permissions currently enabled for the removed proxy account."]
					remove_proxy {
						delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						proxy_type: runtime_types::ulx_node_runtime::ProxyType,
						delay: ::core::primitive::u32,
					},
					#[codec(index = 3)]
					#[doc = "Unregister all proxy accounts for the sender."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					#[doc = ""]
					#[doc = "WARNING: This may be called on accounts created by `pure`, however if done, then"]
					#[doc = "the unreserved fees will be inaccessible. **All access to this account will be lost.**"]
					remove_proxies,
					#[codec(index = 4)]
					#[doc = "Spawn a fresh new account that is guaranteed to be otherwise inaccessible, and"]
					#[doc = "initialize it with a proxy of `proxy_type` for `origin` sender."]
					#[doc = ""]
					#[doc = "Requires a `Signed` origin."]
					#[doc = ""]
					#[doc = "- `proxy_type`: The type of the proxy that the sender will be registered as over the"]
					#[doc = "new account. This will almost always be the most permissive `ProxyType` possible to"]
					#[doc = "allow for maximum flexibility."]
					#[doc = "- `index`: A disambiguation index, in case this is called multiple times in the same"]
					#[doc = "transaction (e.g. with `utility::batch`). Unless you're using `batch` you probably just"]
					#[doc = "want to use `0`."]
					#[doc = "- `delay`: The announcement period required of the initial proxy. Will generally be"]
					#[doc = "zero."]
					#[doc = ""]
					#[doc = "Fails with `Duplicate` if this has already been called in this transaction, from the"]
					#[doc = "same sender, with the same parameters."]
					#[doc = ""]
					#[doc = "Fails if there are insufficient funds to pay for deposit."]
					create_pure {
						proxy_type: runtime_types::ulx_node_runtime::ProxyType,
						delay: ::core::primitive::u32,
						index: ::core::primitive::u16,
					},
					#[codec(index = 5)]
					#[doc = "Removes a previously spawned pure proxy."]
					#[doc = ""]
					#[doc = "WARNING: **All access to this account will be lost.** Any funds held in it will be"]
					#[doc = "inaccessible."]
					#[doc = ""]
					#[doc = "Requires a `Signed` origin, and the sender account must have been created by a call to"]
					#[doc = "`pure` with corresponding parameters."]
					#[doc = ""]
					#[doc = "- `spawner`: The account that originally called `pure` to create this account."]
					#[doc = "- `index`: The disambiguation index originally passed to `pure`. Probably `0`."]
					#[doc = "- `proxy_type`: The proxy type originally passed to `pure`."]
					#[doc = "- `height`: The height of the chain when the call to `pure` was processed."]
					#[doc = "- `ext_index`: The extrinsic index in which the call to `pure` was processed."]
					#[doc = ""]
					#[doc = "Fails with `NoPermission` in case the caller is not a previously created pure"]
					#[doc = "account whose `pure` call has corresponding parameters."]
					kill_pure {
						spawner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						proxy_type: runtime_types::ulx_node_runtime::ProxyType,
						index: ::core::primitive::u16,
						#[codec(compact)]
						height: ::core::primitive::u32,
						#[codec(compact)]
						ext_index: ::core::primitive::u32,
					},
					#[codec(index = 6)]
					#[doc = "Publish the hash of a proxy-call that will be made in the future."]
					#[doc = ""]
					#[doc = "This must be called some number of blocks before the corresponding `proxy` is attempted"]
					#[doc = "if the delay associated with the proxy relationship is greater than zero."]
					#[doc = ""]
					#[doc = "No more than `MaxPending` announcements may be made at any one time."]
					#[doc = ""]
					#[doc = "This will take a deposit of `AnnouncementDepositFactor` as well as"]
					#[doc = "`AnnouncementDepositBase` if there are no other pending announcements."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_ and a proxy of `real`."]
					#[doc = ""]
					#[doc = "Parameters:"]
					#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
					#[doc = "- `call_hash`: The hash of the call to be made by the `real` account."]
					announce {
						real: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						call_hash: ::sp_core::H256,
					},
					#[codec(index = 7)]
					#[doc = "Remove a given announcement."]
					#[doc = ""]
					#[doc = "May be called by a proxy account to remove a call they previously announced and return"]
					#[doc = "the deposit."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					#[doc = ""]
					#[doc = "Parameters:"]
					#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
					#[doc = "- `call_hash`: The hash of the call to be made by the `real` account."]
					remove_announcement {
						real: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						call_hash: ::sp_core::H256,
					},
					#[codec(index = 8)]
					#[doc = "Remove the given announcement of a delegate."]
					#[doc = ""]
					#[doc = "May be called by a target (proxied) account to remove a call that one of their delegates"]
					#[doc = "(`delegate`) has announced they want to execute. The deposit is returned."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					#[doc = ""]
					#[doc = "Parameters:"]
					#[doc = "- `delegate`: The account that previously announced the call."]
					#[doc = "- `call_hash`: The hash of the call to be made."]
					reject_announcement {
						delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						call_hash: ::sp_core::H256,
					},
					#[codec(index = 9)]
					#[doc = "Dispatch the given `call` from an account that the sender is authorized for through"]
					#[doc = "`add_proxy`."]
					#[doc = ""]
					#[doc = "Removes any corresponding announcement(s)."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					#[doc = ""]
					#[doc = "Parameters:"]
					#[doc = "- `real`: The account that the proxy will make a call on behalf of."]
					#[doc = "- `force_proxy_type`: Specify the exact proxy type to be used and checked for this call."]
					#[doc = "- `call`: The call to be made by the `real` account."]
					proxy_announced {
						delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						real: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						force_proxy_type:
							::core::option::Option<runtime_types::ulx_node_runtime::ProxyType>,
						call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall>,
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
					#[doc = "There are too many proxies registered or too many announcements pending."]
					TooMany,
					#[codec(index = 1)]
					#[doc = "Proxy registration not found."]
					NotFound,
					#[codec(index = 2)]
					#[doc = "Sender is not a proxy of the account to be proxied."]
					NotProxy,
					#[codec(index = 3)]
					#[doc = "A call which is incompatible with the proxy type's filter was attempted."]
					Unproxyable,
					#[codec(index = 4)]
					#[doc = "Account is already a proxy."]
					Duplicate,
					#[codec(index = 5)]
					#[doc = "Call may not be made by proxy because it may escalate its privileges."]
					NoPermission,
					#[codec(index = 6)]
					#[doc = "Announcement, if made at all, was made too recently."]
					Unannounced,
					#[codec(index = 7)]
					#[doc = "Cannot add self as proxy."]
					NoSelfProxy,
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
					#[doc = "A proxy was executed correctly, with the given."]
					ProxyExecuted {
						result:
							::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
					},
					#[codec(index = 1)]
					#[doc = "A pure account has been created by new proxy with given"]
					#[doc = "disambiguation index and proxy type."]
					PureCreated {
						pure: ::subxt::utils::AccountId32,
						who: ::subxt::utils::AccountId32,
						proxy_type: runtime_types::ulx_node_runtime::ProxyType,
						disambiguation_index: ::core::primitive::u16,
					},
					#[codec(index = 2)]
					#[doc = "An announcement was placed to make a call in the future."]
					Announced {
						real: ::subxt::utils::AccountId32,
						proxy: ::subxt::utils::AccountId32,
						call_hash: ::sp_core::H256,
					},
					#[codec(index = 3)]
					#[doc = "A proxy was added."]
					ProxyAdded {
						delegator: ::subxt::utils::AccountId32,
						delegatee: ::subxt::utils::AccountId32,
						proxy_type: runtime_types::ulx_node_runtime::ProxyType,
						delay: ::core::primitive::u32,
					},
					#[codec(index = 4)]
					#[doc = "A proxy was removed."]
					ProxyRemoved {
						delegator: ::subxt::utils::AccountId32,
						delegatee: ::subxt::utils::AccountId32,
						proxy_type: runtime_types::ulx_node_runtime::ProxyType,
						delay: ::core::primitive::u32,
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
			pub struct Announcement<_0, _1, _2> {
				pub real: _0,
				pub call_hash: _1,
				pub height: _2,
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
			pub struct ProxyDefinition<_0, _1, _2> {
				pub delegate: _0,
				pub proxy_type: _1,
				pub delay: _2,
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
					#[doc = "Sets the session key(s) of the function caller to `keys`."]
					#[doc = "Allows an account to set its session key prior to becoming a validator."]
					#[doc = "This doesn't take effect until the next session."]
					#[doc = ""]
					#[doc = "The dispatch origin of this function must be signed."]
					#[doc = ""]
					#[doc = "## Complexity"]
					#[doc = "- `O(1)`. Actual cost depends on the number of length of `T::Keys::key_ids()` which is"]
					#[doc = "  fixed."]
					set_keys {
						keys: runtime_types::ulx_node_runtime::opaque::SessionKeys,
						proof: ::std::vec::Vec<::core::primitive::u8>,
					},
					#[codec(index = 1)]
					#[doc = "Removes any session key(s) of the function caller."]
					#[doc = ""]
					#[doc = "This doesn't take effect until the next session."]
					#[doc = ""]
					#[doc = "The dispatch origin of this function must be Signed and the account must be either be"]
					#[doc = "convertible to a validator ID using the chain's typical addressing system (this usually"]
					#[doc = "means being a controller account) or directly convertible into a validator ID (which"]
					#[doc = "usually means being a stash account)."]
					#[doc = ""]
					#[doc = "## Complexity"]
					#[doc = "- `O(1)` in number of key types. Actual cost depends on the number of length of"]
					#[doc = "  `T::Keys::key_ids()` which is fixed."]
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
					#[doc = "Authenticates the sudo key and dispatches a function call with `Root` origin."]
					sudo { call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall> },
					#[codec(index = 1)]
					#[doc = "Authenticates the sudo key and dispatches a function call with `Root` origin."]
					#[doc = "This function does not check the weight of the call, and instead allows the"]
					#[doc = "Sudo user to specify the weight of the call."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					sudo_unchecked_weight {
						call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall>,
						weight: runtime_types::sp_weights::weight_v2::Weight,
					},
					#[codec(index = 2)]
					#[doc = "Authenticates the current sudo key and sets the given AccountId (`new`) as the new sudo"]
					#[doc = "key."]
					set_key { new: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()> },
					#[codec(index = 3)]
					#[doc = "Authenticates the sudo key and dispatches a function call with `Signed` origin from"]
					#[doc = "a given account."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _Signed_."]
					sudo_as {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						call: ::std::boxed::Box<runtime_types::ulx_node_runtime::RuntimeCall>,
					},
					#[codec(index = 4)]
					#[doc = "Permanently removes the sudo key."]
					#[doc = ""]
					#[doc = "**This cannot be un-done.**"]
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
					#[doc = "Set the current time."]
					#[doc = ""]
					#[doc = "This call should be invoked exactly once per block. It will panic at the finalization"]
					#[doc = "phase, if this call hasn't been invoked by that time."]
					#[doc = ""]
					#[doc = "The timestamp should be greater than the previous one by the amount specified by"]
					#[doc = "[`Config::MinimumPeriod`]."]
					#[doc = ""]
					#[doc = "The dispatch origin for this call must be _None_."]
					#[doc = ""]
					#[doc = "This dispatch class is _Mandatory_ to ensure it gets executed in the block. Be aware"]
					#[doc = "that changing the complexity of this call could result exhausting the resources in a"]
					#[doc = "block to execute any other calls."]
					#[doc = ""]
					#[doc = "## Complexity"]
					#[doc = "- `O(1)` (Note that implementations of `OnTimestampSet` must also be `O(1)`)"]
					#[doc = "- 1 storage read and 1 storage mutation (codec `O(1)` because of `DidUpdate::take` in"]
					#[doc = "  `on_finalize`)"]
					#[doc = "- 1 event handler `on_timestamp_set`. Must be `O(1)`."]
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
					#[doc = "Pause a call."]
					#[doc = ""]
					#[doc = "Can only be called by [`Config::PauseOrigin`]."]
					#[doc = "Emits an [`Event::CallPaused`] event on success."]
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
					#[doc = "Un-pause a call."]
					#[doc = ""]
					#[doc = "Can only be called by [`Config::UnpauseOrigin`]."]
					#[doc = "Emits an [`Event::CallUnpaused`] event on success."]
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
		pub mod pallet_vaults {
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
					create {
						vault_config: runtime_types::pallet_vaults::pallet::VaultConfig<
							::core::primitive::u128,
						>,
					},
					#[codec(index = 1)]
					#[doc = "Modify funds offered by the vault. This will not affect existing bonds, but will affect"]
					#[doc = "the amount of funds available for new bonds."]
					#[doc = ""]
					#[doc = "The securitization percent must be maintained or increased."]
					#[doc = ""]
					#[doc = "The amount offered may not go below the existing bonded amounts, but you can release"]
					#[doc = "funds in this vault as bonds are released. To stop issuing any more bonds, use the"]
					#[doc = "`close` api."]
					modify_funding {
						vault_id: ::core::primitive::u32,
						total_mining_amount_offered: ::core::primitive::u128,
						total_bitcoin_amount_offered: ::core::primitive::u128,
						securitization_percent:
							runtime_types::sp_arithmetic::fixed_point::FixedU128,
					},
					#[codec(index = 2)]
					#[doc = "Change the terms of this vault. The change will be applied at the next mining slot"]
					#[doc = "change that is at least `MinTermsModificationBlockDelay` blocks away."]
					modify_terms {
						vault_id: ::core::primitive::u32,
						terms: runtime_types::ulx_primitives::bond::VaultTerms<
							::core::primitive::u128,
						>,
					},
					#[codec(index = 3)]
					#[doc = "Stop offering additional bonds from this vault. Will not affect existing bond."]
					#[doc = "As funds are returned, they will be released to the vault owner."]
					close { vault_id: ::core::primitive::u32 },
					#[codec(index = 4)]
					#[doc = "Add public key hashes to the vault. Will be inserted at the beginning of the list."]
					add_bitcoin_pubkey_hashes {
						vault_id: ::core::primitive::u32,
						bitcoin_pubkey_hashes:
							runtime_types::bounded_collections::bounded_vec::BoundedVec<
								runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash,
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
					BondNotFound,
					#[codec(index = 1)]
					NoMoreVaultIds,
					#[codec(index = 2)]
					NoMoreBondIds,
					#[codec(index = 3)]
					MinimumBondAmountNotMet,
					#[codec(index = 4)]
					#[doc = "There are too many bond or bond funds expiring in the given expiration block"]
					ExpirationAtBlockOverflow,
					#[codec(index = 5)]
					InsufficientFunds,
					#[codec(index = 6)]
					InsufficientVaultFunds,
					#[codec(index = 7)]
					#[doc = "The vault does not have enough bitcoins to cover the mining bond"]
					InsufficientBitcoinsForMining,
					#[codec(index = 8)]
					#[doc = "The proposed transaction would take the account below the minimum (existential) balance"]
					AccountBelowMinimumBalance,
					#[codec(index = 9)]
					VaultClosed,
					#[codec(index = 10)]
					#[doc = "Funding would result in an overflow of the balance type"]
					InvalidVaultAmount,
					#[codec(index = 11)]
					#[doc = "This reduction in bond funds offered goes below the amount that is already committed to"]
					VaultReductionBelowAllocatedFunds,
					#[codec(index = 12)]
					#[doc = "An invalid securitization percent was provided for the vault. NOTE: it cannot be"]
					#[doc = "decreased"]
					InvalidSecuritization,
					#[codec(index = 13)]
					#[doc = "The maximum number of bitcoin pubkeys for a vault has been exceeded"]
					MaxPendingVaultBitcoinPubkeys,
					#[codec(index = 14)]
					#[doc = "Securitization percent would exceed the maximum allowed"]
					MaxSecuritizationPercentExceeded,
					#[codec(index = 15)]
					InvalidBondType,
					#[codec(index = 16)]
					BitcoinUtxoNotFound,
					#[codec(index = 17)]
					InsufficientSatoshisBonded,
					#[codec(index = 18)]
					NoBitcoinPricesAvailable,
					#[codec(index = 19)]
					#[doc = "The bitcoin script to lock this bitcoin has errors"]
					InvalidBitcoinScript,
					#[codec(index = 20)]
					ExpirationTooSoon,
					#[codec(index = 21)]
					NoPermissions,
					#[codec(index = 22)]
					HoldUnexpectedlyModified,
					#[codec(index = 23)]
					UnrecoverableHold,
					#[codec(index = 24)]
					VaultNotFound,
					#[codec(index = 25)]
					#[doc = "The fee for this bond exceeds the amount of the bond, which is unsafe"]
					FeeExceedsBondAmount,
					#[codec(index = 26)]
					#[doc = "No Vault public keys are available"]
					NoVaultBitcoinPubkeysAvailable,
					#[codec(index = 27)]
					#[doc = "The terms modification list could not handle any more items"]
					TermsModificationOverflow,
					#[codec(index = 28)]
					#[doc = "Terms are already scheduled to be changed"]
					TermsChangeAlreadyScheduled,
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
					VaultCreated {
						vault_id: ::core::primitive::u32,
						bitcoin_argons: ::core::primitive::u128,
						mining_argons: ::core::primitive::u128,
						securitization_percent:
							runtime_types::sp_arithmetic::fixed_point::FixedU128,
						operator_account_id: ::subxt::utils::AccountId32,
					},
					#[codec(index = 1)]
					VaultModified {
						vault_id: ::core::primitive::u32,
						bitcoin_argons: ::core::primitive::u128,
						mining_argons: ::core::primitive::u128,
						securitization_percent:
							runtime_types::sp_arithmetic::fixed_point::FixedU128,
					},
					#[codec(index = 2)]
					VaultTermsChangeScheduled {
						vault_id: ::core::primitive::u32,
						change_block: ::core::primitive::u32,
					},
					#[codec(index = 3)]
					VaultTermsChanged { vault_id: ::core::primitive::u32 },
					#[codec(index = 4)]
					VaultClosed {
						vault_id: ::core::primitive::u32,
						bitcoin_amount_still_bonded: ::core::primitive::u128,
						mining_amount_still_bonded: ::core::primitive::u128,
						securitization_still_bonded: ::core::primitive::u128,
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
					EnterVault,
					#[codec(index = 1)]
					BondFee,
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
				pub struct VaultConfig<_0> {
					pub terms: runtime_types::ulx_primitives::bond::VaultTerms<_0>,
					#[codec(compact)]
					pub bitcoin_amount_allocated: _0,
					pub bitcoin_pubkey_hashes:
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash,
						>,
					#[codec(compact)]
					pub mining_amount_allocated: _0,
					#[codec(compact)]
					pub securitization_percent:
						runtime_types::sp_arithmetic::fixed_point::FixedU128,
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
			pub mod per_things {
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
				pub struct Percent(pub ::core::primitive::u8);
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
			#[derive(
				:: subxt :: ext :: codec :: Decode,
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
			#[derive(
				:: subxt :: ext :: codec :: Decode,
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
						pub parent_hash: ::sp_core::H256,
						#[codec(compact)]
						pub number: _0,
						pub state_root: ::sp_core::H256,
						pub extrinsics_root: ::sp_core::H256,
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
			pub enum ExtrinsicInclusionMode {
				#[codec(index = 0)]
				AllExtrinsics,
				#[codec(index = 1)]
				OnlyInherents,
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
				Ed25519([::core::primitive::u8; 64usize]),
				#[codec(index = 1)]
				Sr25519([::core::primitive::u8; 64usize]),
				#[codec(index = 2)]
				Ecdsa([::core::primitive::u8; 65usize]),
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
			pub enum ProxyType {
				#[codec(index = 0)]
				Any,
				#[codec(index = 1)]
				NonTransfer,
				#[codec(index = 2)]
				PriceIndex,
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
				Multisig(runtime_types::pallet_multisig::pallet::Call),
				#[codec(index = 3)]
				Proxy(runtime_types::pallet_proxy::pallet::Call),
				#[codec(index = 4)]
				Ticks(runtime_types::pallet_ticks::pallet::Call),
				#[codec(index = 5)]
				MiningSlot(runtime_types::pallet_mining_slot::pallet::Call),
				#[codec(index = 6)]
				BitcoinUtxos(runtime_types::pallet_bitcoin_utxos::pallet::Call),
				#[codec(index = 7)]
				Vaults(runtime_types::pallet_vaults::pallet::Call),
				#[codec(index = 8)]
				Bonds(runtime_types::pallet_bond::pallet::Call),
				#[codec(index = 9)]
				Notaries(runtime_types::pallet_notaries::pallet::Call),
				#[codec(index = 10)]
				Notebook(runtime_types::pallet_notebook::pallet::Call),
				#[codec(index = 11)]
				ChainTransfer(runtime_types::pallet_chain_transfer::pallet::Call),
				#[codec(index = 12)]
				BlockSealSpec(runtime_types::pallet_block_seal_spec::pallet::Call),
				#[codec(index = 13)]
				DataDomain(runtime_types::pallet_data_domain::pallet::Call),
				#[codec(index = 14)]
				PriceIndex(runtime_types::pallet_price_index::pallet::Call),
				#[codec(index = 17)]
				Session(runtime_types::pallet_session::pallet::Call),
				#[codec(index = 18)]
				BlockSeal(runtime_types::pallet_block_seal::pallet::Call),
				#[codec(index = 19)]
				BlockRewards(runtime_types::pallet_block_rewards::pallet::Call),
				#[codec(index = 20)]
				Grandpa(runtime_types::pallet_grandpa::pallet::Call),
				#[codec(index = 22)]
				Mint(runtime_types::pallet_mint::pallet::Call),
				#[codec(index = 23)]
				ArgonBalances(runtime_types::pallet_balances::pallet::Call),
				#[codec(index = 24)]
				UlixeeBalances(runtime_types::pallet_balances::pallet::Call2),
				#[codec(index = 25)]
				TxPause(runtime_types::pallet_tx_pause::pallet::Call),
				#[codec(index = 27)]
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
				Multisig(runtime_types::pallet_multisig::pallet::Error),
				#[codec(index = 3)]
				Proxy(runtime_types::pallet_proxy::pallet::Error),
				#[codec(index = 4)]
				Ticks(runtime_types::pallet_ticks::pallet::Error),
				#[codec(index = 5)]
				MiningSlot(runtime_types::pallet_mining_slot::pallet::Error),
				#[codec(index = 6)]
				BitcoinUtxos(runtime_types::pallet_bitcoin_utxos::pallet::Error),
				#[codec(index = 7)]
				Vaults(runtime_types::pallet_vaults::pallet::Error),
				#[codec(index = 8)]
				Bonds(runtime_types::pallet_bond::pallet::Error),
				#[codec(index = 9)]
				Notaries(runtime_types::pallet_notaries::pallet::Error),
				#[codec(index = 10)]
				Notebook(runtime_types::pallet_notebook::pallet::Error),
				#[codec(index = 11)]
				ChainTransfer(runtime_types::pallet_chain_transfer::pallet::Error),
				#[codec(index = 12)]
				BlockSealSpec(runtime_types::pallet_block_seal_spec::pallet::Error),
				#[codec(index = 13)]
				DataDomain(runtime_types::pallet_data_domain::pallet::Error),
				#[codec(index = 14)]
				PriceIndex(runtime_types::pallet_price_index::pallet::Error),
				#[codec(index = 17)]
				Session(runtime_types::pallet_session::pallet::Error),
				#[codec(index = 18)]
				BlockSeal(runtime_types::pallet_block_seal::pallet::Error),
				#[codec(index = 19)]
				BlockRewards(runtime_types::pallet_block_rewards::pallet::Error),
				#[codec(index = 20)]
				Grandpa(runtime_types::pallet_grandpa::pallet::Error),
				#[codec(index = 22)]
				Mint(runtime_types::pallet_mint::pallet::Error),
				#[codec(index = 23)]
				ArgonBalances(runtime_types::pallet_balances::pallet::Error),
				#[codec(index = 24)]
				UlixeeBalances(runtime_types::pallet_balances::pallet::Error2),
				#[codec(index = 25)]
				TxPause(runtime_types::pallet_tx_pause::pallet::Error),
				#[codec(index = 27)]
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
				#[codec(index = 2)]
				Multisig(runtime_types::pallet_multisig::pallet::Event),
				#[codec(index = 3)]
				Proxy(runtime_types::pallet_proxy::pallet::Event),
				#[codec(index = 5)]
				MiningSlot(runtime_types::pallet_mining_slot::pallet::Event),
				#[codec(index = 6)]
				BitcoinUtxos(runtime_types::pallet_bitcoin_utxos::pallet::Event),
				#[codec(index = 7)]
				Vaults(runtime_types::pallet_vaults::pallet::Event),
				#[codec(index = 8)]
				Bonds(runtime_types::pallet_bond::pallet::Event),
				#[codec(index = 9)]
				Notaries(runtime_types::pallet_notaries::pallet::Event),
				#[codec(index = 10)]
				Notebook(runtime_types::pallet_notebook::pallet::Event),
				#[codec(index = 11)]
				ChainTransfer(runtime_types::pallet_chain_transfer::pallet::Event),
				#[codec(index = 12)]
				BlockSealSpec(runtime_types::pallet_block_seal_spec::pallet::Event),
				#[codec(index = 13)]
				DataDomain(runtime_types::pallet_data_domain::pallet::Event),
				#[codec(index = 14)]
				PriceIndex(runtime_types::pallet_price_index::pallet::Event),
				#[codec(index = 17)]
				Session(runtime_types::pallet_session::pallet::Event),
				#[codec(index = 19)]
				BlockRewards(runtime_types::pallet_block_rewards::pallet::Event),
				#[codec(index = 20)]
				Grandpa(runtime_types::pallet_grandpa::pallet::Event),
				#[codec(index = 21)]
				Offences(runtime_types::pallet_offences::pallet::Event),
				#[codec(index = 22)]
				Mint(runtime_types::pallet_mint::pallet::Event),
				#[codec(index = 23)]
				ArgonBalances(runtime_types::pallet_balances::pallet::Event),
				#[codec(index = 24)]
				UlixeeBalances(runtime_types::pallet_balances::pallet::Event2),
				#[codec(index = 25)]
				TxPause(runtime_types::pallet_tx_pause::pallet::Event),
				#[codec(index = 26)]
				TransactionPayment(runtime_types::pallet_transaction_payment::pallet::Event),
				#[codec(index = 27)]
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
				#[codec(index = 19)]
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
				#[codec(index = 5)]
				MiningSlot(runtime_types::pallet_mining_slot::pallet::HoldReason),
				#[codec(index = 7)]
				Vaults(runtime_types::pallet_vaults::pallet::HoldReason),
				#[codec(index = 8)]
				Bonds(runtime_types::pallet_bond::pallet::HoldReason),
				#[codec(index = 19)]
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
						account_type: runtime_types::ulx_primitives::account::AccountType,
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
					InvalidDomainLeaseAllocation,
					#[codec(index = 20)]
					TaxBalanceChangeNotNetZero {
						sent: ::core::primitive::u128,
						claimed: ::core::primitive::u128,
					},
					#[codec(index = 21)]
					MissingBalanceProof,
					#[codec(index = 22)]
					InvalidPreviousBalanceProof,
					#[codec(index = 23)]
					InvalidNotebookHash,
					#[codec(index = 24)]
					InvalidNotebookHeaderHash,
					#[codec(index = 25)]
					DuplicateChainTransfer,
					#[codec(index = 26)]
					DuplicatedAccountOriginUid,
					#[codec(index = 27)]
					InvalidNotarySignature,
					#[codec(index = 28)]
					InvalidSecretProvided,
					#[codec(index = 29)]
					NotebookTooOld,
					#[codec(index = 30)]
					CatchupNotebooksMissing,
					#[codec(index = 31)]
					DecodeError,
					#[codec(index = 32)]
					AccountEscrowHoldDoesntExist,
					#[codec(index = 33)]
					AccountAlreadyHasEscrowHold,
					#[codec(index = 34)]
					EscrowHoldNotReadyForClaim {
						current_tick: ::core::primitive::u32,
						claim_tick: ::core::primitive::u32,
					},
					#[codec(index = 35)]
					AccountLocked,
					#[codec(index = 36)]
					MissingEscrowHoldNote,
					#[codec(index = 37)]
					InvalidEscrowHoldNote,
					#[codec(index = 38)]
					InvalidEscrowClaimers,
					#[codec(index = 39)]
					EscrowNoteBelowMinimum,
					#[codec(index = 40)]
					InvalidTaxNoteAccount,
					#[codec(index = 41)]
					InvalidTaxOperation,
					#[codec(index = 42)]
					InsufficientTaxIncluded {
						tax_sent: ::core::primitive::u128,
						tax_owed: ::core::primitive::u128,
						account_id: ::subxt::utils::AccountId32,
					},
					#[codec(index = 43)]
					InsufficientBlockVoteTax,
					#[codec(index = 44)]
					IneligibleTaxVoter,
					#[codec(index = 45)]
					BlockVoteInvalidSignature,
					#[codec(index = 46)]
					InvalidBlockVoteAllocation,
					#[codec(index = 47)]
					InvalidBlockVoteRoot,
					#[codec(index = 48)]
					InvalidBlockVotesCount,
					#[codec(index = 49)]
					InvalidBlockVotingPower,
					#[codec(index = 50)]
					InvalidBlockVoteList,
					#[codec(index = 51)]
					InvalidComputeProof,
					#[codec(index = 52)]
					InvalidBlockVoteSource,
					#[codec(index = 53)]
					InsufficientBlockVoteMinimum,
					#[codec(index = 54)]
					BlockVoteDataDomainMismatch,
					#[codec(index = 55)]
					BlockVoteEscrowReused,
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
			pub mod account {
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
			}
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
				pub struct NotaryNotebookVotes {
					#[codec(compact)]
					pub notary_id: ::core::primitive::u32,
					#[codec(compact)]
					pub notebook_number: ::core::primitive::u32,
					pub raw_votes: ::std::vec::Vec<(
						::std::vec::Vec<::core::primitive::u8>,
						::core::primitive::u128,
					)>,
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
				pub struct NotebookAuditResult {
					#[codec(compact)]
					pub notary_id: ::core::primitive::u32,
					#[codec(compact)]
					pub notebook_number: ::core::primitive::u32,
					#[codec(compact)]
					pub tick: ::core::primitive::u32,
					pub raw_votes: ::std::vec::Vec<(
						::std::vec::Vec<::core::primitive::u8>,
						::core::primitive::u128,
					)>,
					pub changed_accounts_root: ::sp_core::H256,
					pub account_changelist: ::std::vec::Vec<
						runtime_types::ulx_primitives::balance_change::AccountOrigin,
					>,
					pub used_transfers_to_localchain: ::std::vec::Vec<::core::primitive::u32>,
					pub secret_hash: ::sp_core::H256,
					pub block_votes_root: ::sp_core::H256,
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
				pub struct NotebookAuditSummary {
					#[codec(compact)]
					pub notary_id: ::core::primitive::u32,
					#[codec(compact)]
					pub notebook_number: ::core::primitive::u32,
					#[codec(compact)]
					pub tick: ::core::primitive::u32,
					pub changed_accounts_root: ::sp_core::H256,
					pub account_changelist: ::std::vec::Vec<
						runtime_types::ulx_primitives::balance_change::AccountOrigin,
					>,
					pub used_transfers_to_localchain: ::std::vec::Vec<::core::primitive::u32>,
					pub secret_hash: ::sp_core::H256,
					pub block_votes_root: ::sp_core::H256,
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
						::sp_core::H256,
					>,
					#[codec(compact)]
					pub number_of_leaves: ::core::primitive::u32,
					#[codec(compact)]
					pub leaf_index: ::core::primitive::u32,
				}
			}
			pub mod bitcoin {
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
				pub struct BitcoinBlock {
					#[codec(compact)]
					pub block_height: ::core::primitive::u64,
					pub block_hash: runtime_types::ulx_primitives::bitcoin::H256Le,
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
				pub enum BitcoinCosignScriptPubkey {
					#[codec(index = 0)]
					P2WSH { wscript_hash: ::sp_core::H256 },
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
				pub struct BitcoinPubkeyHash(pub [::core::primitive::u8; 20usize]);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum BitcoinRejectedReason {
					#[codec(index = 0)]
					SatoshisMismatch,
					#[codec(index = 1)]
					Spent,
					#[codec(index = 2)]
					LookupExpired,
					#[codec(index = 3)]
					DuplicateUtxo,
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
				pub struct BitcoinScriptPubkey(
					pub  runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
				);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BitcoinSignature(
					pub  runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
				);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BitcoinSyncStatus {
					pub confirmed_block: runtime_types::ulx_primitives::bitcoin::BitcoinBlock,
					pub synched_block: ::core::option::Option<
						runtime_types::ulx_primitives::bitcoin::BitcoinBlock,
					>,
					pub oldest_allowed_block_height: ::core::primitive::u64,
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
				pub struct CompressedBitcoinPubkey(pub [::core::primitive::u8; 33usize]);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct H256Le(pub [::core::primitive::u8; 32usize]);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UtxoRef {
					pub txid: runtime_types::ulx_primitives::bitcoin::H256Le,
					#[codec(compact)]
					pub output_index: ::core::primitive::u32,
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
				pub struct UtxoValue {
					pub utxo_id: ::core::primitive::u64,
					pub script_pubkey:
						runtime_types::ulx_primitives::bitcoin::BitcoinCosignScriptPubkey,
					#[codec(compact)]
					pub satoshis: ::core::primitive::u64,
					#[codec(compact)]
					pub submitted_at_height: ::core::primitive::u64,
					#[codec(compact)]
					pub watch_for_spent_until_height: ::core::primitive::u64,
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
				#[derive(
					:: subxt :: ext :: codec :: Decode,
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
					#[codec(compact)]
					pub ulixees: _1,
					#[codec(compact)]
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
				pub struct MiningAuthority<_0, _1> {
					#[codec(compact)]
					pub authority_index: ::core::primitive::u32,
					pub authority_id: _0,
					pub account_id: _1,
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
				pub struct MiningRegistration<_0, _1> {
					pub account_id: _0,
					pub reward_destination:
						runtime_types::ulx_primitives::block_seal::RewardDestination<_0>,
					pub bond_id: ::core::option::Option<::core::primitive::u64>,
					#[codec(compact)]
					pub bond_amount: _1,
					#[codec(compact)]
					pub ownership_tokens: _1,
					pub reward_sharing: ::core::option::Option<
						runtime_types::ulx_primitives::block_seal::RewardSharing<_0>,
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
				pub enum RewardDestination<_0> {
					#[codec(index = 0)]
					Owner,
					#[codec(index = 1)]
					Account(_0),
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
				pub struct RewardSharing<_0> {
					pub account_id: _0,
					#[codec(compact)]
					pub percent_take: runtime_types::sp_arithmetic::per_things::Percent,
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
				pub struct BestBlockVoteSeal<_0, _1> {
					pub seal_strength: runtime_types::primitive_types::U256,
					#[codec(compact)]
					pub notary_id: ::core::primitive::u32,
					pub block_vote_bytes: ::std::vec::Vec<::core::primitive::u8>,
					#[codec(compact)]
					pub source_notebook_number: ::core::primitive::u32,
					pub source_notebook_proof:
						runtime_types::ulx_primitives::balance_change::MerkleProof,
					pub closest_miner: (_0, _1),
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
					pub block_hash: _0,
					#[codec(compact)]
					pub index: ::core::primitive::u32,
					#[codec(compact)]
					pub power: ::core::primitive::u128,
					pub data_domain_hash: ::sp_core::H256,
					pub data_domain_account: ::subxt::utils::AccountId32,
					pub signature: runtime_types::sp_runtime::MultiSignature,
					pub block_rewards_account_id: ::subxt::utils::AccountId32,
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
				pub struct Bond<_0, _1, _2> {
					pub bond_type: runtime_types::ulx_primitives::bond::BondType,
					#[codec(compact)]
					pub vault_id: _2,
					pub utxo_id: ::core::option::Option<::core::primitive::u64>,
					pub bonded_account_id: _0,
					#[codec(compact)]
					pub total_fee: _1,
					#[codec(compact)]
					pub prepaid_fee: _1,
					#[codec(compact)]
					pub amount: _1,
					#[codec(compact)]
					pub start_block: _2,
					pub expiration: runtime_types::ulx_primitives::bond::BondExpiration<_2>,
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
				pub enum BondError {
					#[codec(index = 0)]
					BondNotFound,
					#[codec(index = 1)]
					NoMoreBondIds,
					#[codec(index = 2)]
					MinimumBondAmountNotMet,
					#[codec(index = 3)]
					VaultClosed,
					#[codec(index = 4)]
					ExpirationAtBlockOverflow,
					#[codec(index = 5)]
					AccountWouldBeBelowMinimum,
					#[codec(index = 6)]
					InsufficientFunds,
					#[codec(index = 7)]
					InsufficientVaultFunds,
					#[codec(index = 8)]
					InsufficientBitcoinsForMining,
					#[codec(index = 9)]
					ExpirationTooSoon,
					#[codec(index = 10)]
					NoPermissions,
					#[codec(index = 11)]
					HoldUnexpectedlyModified,
					#[codec(index = 12)]
					UnrecoverableHold,
					#[codec(index = 13)]
					VaultNotFound,
					#[codec(index = 14)]
					NoVaultBitcoinPubkeysAvailable,
					#[codec(index = 15)]
					FeeExceedsBondAmount,
					#[codec(index = 16)]
					InvalidBitcoinScript,
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
				pub enum BondExpiration<_0> {
					#[codec(index = 0)]
					UlixeeBlock(#[codec(compact)] _0),
					#[codec(index = 1)]
					BitcoinBlock(#[codec(compact)] ::core::primitive::u64),
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
				pub enum BondType {
					#[codec(index = 0)]
					Mining,
					#[codec(index = 1)]
					Bitcoin,
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
				pub struct Vault<_0, _1, _2> {
					pub operator_account_id: _0,
					pub bitcoin_argons: runtime_types::ulx_primitives::bond::VaultArgons<_1>,
					#[codec(compact)]
					pub securitization_percent:
						runtime_types::sp_arithmetic::fixed_point::FixedU128,
					#[codec(compact)]
					pub securitized_argons: _1,
					pub mining_argons: runtime_types::ulx_primitives::bond::VaultArgons<_1>,
					#[codec(compact)]
					pub mining_reward_sharing_percent_take:
						runtime_types::sp_arithmetic::per_things::Percent,
					pub is_closed: ::core::primitive::bool,
					pub pending_terms: ::core::option::Option<(
						_2,
						runtime_types::ulx_primitives::bond::VaultTerms<_1>,
					)>,
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
				pub struct VaultArgons<_0> {
					#[codec(compact)]
					pub annual_percent_rate: runtime_types::sp_arithmetic::fixed_point::FixedU128,
					#[codec(compact)]
					pub allocated: _0,
					#[codec(compact)]
					pub bonded: _0,
					#[codec(compact)]
					pub base_fee: _0,
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
				pub struct VaultTerms<_0> {
					#[codec(compact)]
					pub bitcoin_annual_percent_rate:
						runtime_types::sp_arithmetic::fixed_point::FixedU128,
					#[codec(compact)]
					pub bitcoin_base_fee: _0,
					#[codec(compact)]
					pub mining_annual_percent_rate:
						runtime_types::sp_arithmetic::fixed_point::FixedU128,
					#[codec(compact)]
					pub mining_base_fee: _0,
					#[codec(compact)]
					pub mining_reward_sharing_percent_take:
						runtime_types::sp_arithmetic::per_things::Percent,
				}
			}
			pub mod data_domain {
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
				pub struct Semver {
					pub major: ::core::primitive::u32,
					pub minor: ::core::primitive::u32,
					pub patch: ::core::primitive::u32,
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
				pub struct VersionHost {
					pub datastore_id: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					pub host: runtime_types::ulx_primitives::host::Host,
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
				pub struct ZoneRecord<_0> {
					pub payment_account: _0,
					pub notary_id: ::core::primitive::u32,
					pub versions: ::subxt::utils::KeyedVec<
						runtime_types::ulx_primitives::data_domain::Semver,
						runtime_types::ulx_primitives::data_domain::VersionHost,
					>,
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
					#[codec(compact)]
					pub voting_power: ::core::primitive::u128,
					#[codec(compact)]
					pub votes_count: ::core::primitive::u32,
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
				pub struct NotebookDigest<_0> {
					pub notebooks: ::std::vec::Vec<
						runtime_types::ulx_primitives::digests::NotebookDigestRecord<_0>,
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
				pub struct NotebookDigestRecord<_0> {
					#[codec(compact)]
					pub notary_id: ::core::primitive::u32,
					#[codec(compact)]
					pub notebook_number: ::core::primitive::u32,
					#[codec(compact)]
					pub tick: ::core::primitive::u32,
					pub audit_first_failure: ::core::option::Option<_0>,
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
				pub struct ParentVotingKeyDigest {
					pub parent_voting_key: ::core::option::Option<::sp_core::H256>,
				}
			}
			pub mod host {
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
				pub struct Host(
					pub  runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
				);
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
				pub struct BitcoinUtxoSync {
					pub spent:
						::subxt::utils::KeyedVec<::core::primitive::u64, ::core::primitive::u64>,
					pub verified: ::subxt::utils::KeyedVec<
						::core::primitive::u64,
						runtime_types::ulx_primitives::bitcoin::UtxoRef,
					>,
					pub invalid: ::subxt::utils::KeyedVec<
						::core::primitive::u64,
						runtime_types::ulx_primitives::bitcoin::BitcoinRejectedReason,
					>,
					pub sync_to_block: runtime_types::ulx_primitives::bitcoin::BitcoinBlock,
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
				pub enum BlockSealInherent {
					#[codec(index = 0)]
					Vote {
						seal_strength: runtime_types::primitive_types::U256,
						#[codec(compact)]
						notary_id: ::core::primitive::u32,
						#[codec(compact)]
						source_notebook_number: ::core::primitive::u32,
						source_notebook_proof:
							runtime_types::ulx_primitives::balance_change::MerkleProof,
						block_vote:
							runtime_types::ulx_primitives::block_vote::BlockVoteT<::sp_core::H256>,
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
					pub name: runtime_types::ulx_primitives::notary::NotaryName,
					pub public: [::core::primitive::u8; 32usize],
					pub hosts: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::ulx_primitives::host::Host,
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
				pub struct NotaryName(
					pub  runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
				);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
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
					#[codec(compact)]
					pub notebook_number: ::core::primitive::u32,
					#[codec(compact)]
					pub tick: ::core::primitive::u32,
					pub block_votes_root: ::sp_core::H256,
					pub secret_hash: ::sp_core::H256,
					pub parent_secret: ::core::option::Option<::sp_core::H256>,
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
					#[codec(compact)]
					pub notary_id: ::core::primitive::u32,
					#[codec(compact)]
					pub version: ::core::primitive::u32,
					#[codec(compact)]
					pub notebook_number: ::core::primitive::u32,
					#[codec(compact)]
					pub tick: ::core::primitive::u32,
					pub header_hash: ::sp_core::H256,
					#[codec(compact)]
					pub block_votes_count: ::core::primitive::u32,
					#[codec(compact)]
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
					#[codec(compact)]
					pub notary_id: ::core::primitive::u32,
					#[codec(compact)]
					pub notebook_number: ::core::primitive::u32,
					#[codec(compact)]
					pub tick: ::core::primitive::u32,
					#[codec(compact)]
					pub block_votes_count: ::core::primitive::u32,
					#[codec(compact)]
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
					#[codec(compact)]
					pub meta_updated_tick: _1,
					pub meta: runtime_types::ulx_primitives::notary::NotaryMeta,
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
						#[codec(compact)]
						transfer_id: ::core::primitive::u32,
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
					pub tax: ::core::primitive::u128,
					#[codec(compact)]
					pub notary_id: ::core::primitive::u32,
					pub chain_transfers:
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::notebook::ChainTransfer,
						>,
					pub changed_accounts_root: ::sp_core::H256,
					pub changed_account_origins:
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							runtime_types::ulx_primitives::balance_change::AccountOrigin,
						>,
					pub block_votes_root: ::sp_core::H256,
					#[codec(compact)]
					pub block_votes_count: ::core::primitive::u32,
					pub blocks_with_votes:
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::sp_core::H256,
						>,
					#[codec(compact)]
					pub block_voting_power: ::core::primitive::u128,
					pub secret_hash: ::sp_core::H256,
					pub parent_secret: ::core::option::Option<::sp_core::H256>,
					pub data_domains: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						(::sp_core::H256, ::subxt::utils::AccountId32),
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
				pub struct SignedNotebookHeader {
					pub header: runtime_types::ulx_primitives::notebook::NotebookHeader,
					pub signature: [::core::primitive::u8; 64usize],
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
					pub block_author_account_id: _0,
					pub block_vote_rewards_account: ::core::option::Option<_0>,
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
					pub ntp_offset_millis: ::core::primitive::i64,
				}
			}
		}
	}
}
