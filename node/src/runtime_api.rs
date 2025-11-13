use argon_primitives::{
	AccountId, Balance, BitcoinApis, BlockCreatorApis, BlockImportApis, BlockSealApis,
	BlockSealAuthorityId, MiningApis, Nonce, NotaryApis, NotebookApis, TickApis, prelude::*,
};
use argon_runtime::{NotaryRecordT, NotebookVerifyError};
use pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi;
use sp_api::{ApiExt, Metadata};
use sp_block_builder::BlockBuilder;
use sp_consensus_grandpa::GrandpaApi;
use sp_core::H256;
use sp_session::SessionKeys;
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;
use substrate_frame_rpc_system::AccountNonceApi;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use argon_primitives::{BlockHash, BlockNumber};
	use polkadot_sdk::*;
	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
	use sp_runtime::{
		generic,
		traits::{BlakeTwo256, Hash as HashT},
	};

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlockHash>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block hash type.
	pub type Hash = <BlakeTwo256 as HashT>::Output;
}

pub trait BaseHostRuntimeApis:
	ApiExt<opaque::Block>
	+ TaggedTransactionQueue<opaque::Block>
	+ BlockBuilder<opaque::Block>
	+ AccountNonceApi<opaque::Block, AccountId, Nonce>
	+ Metadata<opaque::Block>
	+ SessionKeys<opaque::Block>
	+ TransactionPaymentRuntimeApi<opaque::Block, Balance>
	+ GrandpaApi<opaque::Block>
	+ pallet_ismp_runtime_api::IsmpRuntimeApi<opaque::Block, H256>
	+ TickApis<opaque::Block>
	+ NotebookApis<opaque::Block, NotebookVerifyError>
	+ BlockSealApis<opaque::Block, AccountId, BlockSealAuthorityId>
	+ BlockCreatorApis<opaque::Block, AccountId, NotebookVerifyError>
	+ BlockImportApis<opaque::Block>
	+ BitcoinApis<opaque::Block, Balance>
	+ NotaryApis<opaque::Block, NotaryRecordT>
	+ MiningApis<opaque::Block, AccountId, BlockSealAuthorityId>
{
}

impl<Api> BaseHostRuntimeApis for Api where
	Api: TaggedTransactionQueue<opaque::Block>
		+ ApiExt<opaque::Block>
		+ BlockBuilder<opaque::Block>
		+ AccountNonceApi<opaque::Block, AccountId, Nonce>
		+ Metadata<opaque::Block>
		+ SessionKeys<opaque::Block>
		+ TransactionPaymentRuntimeApi<opaque::Block, Balance>
		+ GrandpaApi<opaque::Block>
		+ pallet_ismp_runtime_api::IsmpRuntimeApi<opaque::Block, H256>
		+ TickApis<opaque::Block>
		+ NotebookApis<opaque::Block, NotebookVerifyError>
		+ BlockSealApis<opaque::Block, AccountId, BlockSealAuthorityId>
		+ BlockCreatorApis<opaque::Block, AccountId, NotebookVerifyError>
		+ BlockImportApis<opaque::Block>
		+ BitcoinApis<opaque::Block, Balance>
		+ NotaryApis<opaque::Block, NotaryRecordT>
		+ MiningApis<opaque::Block, AccountId, BlockSealAuthorityId>
{
}
