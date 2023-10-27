use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use jsonrpsee::{
	core::RpcResult,
	proc_macros::rpc,
	types::{error::CallError, ErrorObject},
};
use serde::{Deserialize, Serialize};
use sp_api::{BlockT, ProvideRuntimeApi};
use sp_application_crypto::AppCrypto;
use sp_core::{bytes::to_hex, ByteArray, H256};
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Header;

use pallet_localchain_relay::LocalchainRelayApis;
use ulx_primitives::{
	notary::{NotaryId, NotarySignature},
	notebook::NotebookNumber,
	BlockSealAuthorityId,
};

#[rpc(client, server)]
pub trait NotaryApis<Hash> {
	#[method(name = "notebook_audit")]
	async fn notebook_audit(
		&self,
		at: Hash,
		version: u32,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		notary_signature: NotarySignature,
		header_hash: H256,
		bytes: Vec<u8>,
		auditor_public: BlockSealAuthorityId,
	) -> RpcResult<AuditResponse>;
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AuditResponse {
	pub signature: String,
}

/// A struct that implements the `ProofOfTaxApi`.
pub struct NotaryRpc<Block, C> {
	keystore: KeystorePtr,
	client: Arc<C>,
	_phantom: PhantomData<Block>,
}

impl<Block, C> NotaryRpc<Block, C> {
	/// Create new `ProofOfTax` instance with the given reference to the client.
	pub fn new(client: Arc<C>, keystore: KeystorePtr) -> Self {
		Self { keystore, client, _phantom: PhantomData }
	}
}

#[async_trait]
impl<Block, C> NotaryApisServer<Block::Hash> for NotaryRpc<Block, C>
where
	Block: BlockT,
	C: ProvideRuntimeApi<Block> + Send + Sync + 'static,
	C::Api: LocalchainRelayApis<Block, <Block::Header as Header>::Number>,
{
	async fn notebook_audit(
		&self,
		at: Block::Hash,
		version: u32,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		notary_signature: NotarySignature,
		header_hash: H256,
		bytes: Vec<u8>,
		auditor_public: BlockSealAuthorityId,
	) -> RpcResult<AuditResponse> {
		if !self
			.keystore
			.has_keys(&[(auditor_public.to_raw_vec(), ulx_primitives::BLOCK_SEAL_KEY_TYPE)])
		{
			return Err(CallError::Custom(ErrorObject::owned(
				Error::InvalidAuthorityId.into(),
				"Invalid auditor",
				None::<()>,
			)))?
		}

		let _ = self
			.client
			.runtime_api()
			.audit_notebook(
				at,
				version,
				notary_id,
				notebook_number,
				notary_signature,
				header_hash,
				bytes,
			)
			.map_err(|e| {
				CallError::Custom(ErrorObject::owned(
					Error::FailedNotebookAudit.into(),
					"Failed notebook api call",
					Some(format!("{:?}", e)),
				))
			})?
			.map_err(|e| {
				CallError::Custom(ErrorObject::owned(
					Error::FailedNotebookAudit.into(),
					"Failed notebook audit",
					Some(format!("{:?}", e)),
				))
			})?;

		let signature = self
			.keystore
			.sign_with(
				<BlockSealAuthorityId as AppCrypto>::ID,
				<BlockSealAuthorityId as AppCrypto>::CRYPTO_ID,
				auditor_public.as_slice(),
				header_hash.as_bytes(),
			)
			.map_err(|e| {
				CallError::Custom(ErrorObject::owned(
					Error::SignatureFailed.into(),
					"Failed signature",
					Some(format!("{:?}", e)),
				))
			})?
			.ok_or_else(|| {
				CallError::Custom(ErrorObject::owned(
					Error::SignatureFailed.into(),
					"No signature generated",
					None::<()>,
				))
			})?;

		Ok(AuditResponse { signature: to_hex(&signature[..], true) })
	}
}

/// Error type of this RPC api.
pub enum Error {
	/// Cannot access or sign with the given public key
	InvalidAuthorityId,
	/// The transaction was not decodable.
	FailedNotebookAudit,
	/// The signature failed
	SignatureFailed,
}

impl From<Error> for i32 {
	fn from(e: Error) -> i32 {
		match e {
			Error::FailedNotebookAudit => 1,
			Error::InvalidAuthorityId => 2,
			Error::SignatureFailed => 3,
		}
	}
}
