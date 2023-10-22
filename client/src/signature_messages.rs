use subxt::{ext::sp_core::Encode, utils::AccountId32};

use crate::api::runtime_types::{
	bounded_collections::bounded_vec::BoundedVec,
	ulx_primitives::{
		block_seal,
		block_seal::{BlockProof, SealStamper},
		notebook::{ChainTransfer, Notebook},
	},
};

const NOTEBOOK_HASH_PREFIX: [u8; 13] = *b"notebook_hash";

#[derive(Encode)]
pub struct NotebookHashMessage<AccountId, Balance> {
	pub prefix: [u8; 13],
	#[codec(compact)]
	pub pinned_to_block_number: u32,
	#[codec(compact)]
	pub notary_id: u32,
	pub transfers: BoundedVec<ChainTransfer<AccountId, Balance, u32>>,
	pub auditors: BoundedVec<(block_seal::app::Public, block_seal::app::Signature)>,
}

pub fn to_notebook_post_hash<AccountId: Clone, Balance: Clone>(
	notebook: &Notebook<AccountId, Balance, u32>,
) -> NotebookHashMessage<AccountId, Balance> {
	NotebookHashMessage {
		prefix: NOTEBOOK_HASH_PREFIX,
		pinned_to_block_number: notebook.pinned_to_block_number,
		notary_id: notebook.notary_id,
		transfers: BoundedVec(notebook.transfers.0.iter().map(|a| a.clone()).collect::<Vec<_>>()),
		auditors: BoundedVec(
			notebook
				.auditors
				.0
				.iter()
				.map(|(public, sig)| {
					(
						block_seal::app::Public(public.0.clone()),
						block_seal::app::Signature(sig.0.clone()),
					)
				})
				.collect::<Vec<_>>(),
		),
	}
}

const SEAL_NONCE_PREFIX: [u8; 14] = *b"ulx_block_seal";

#[derive(Encode)]
pub struct SealNonceHashMessage<Hash> {
	pub prefix: [u8; 14],
	pub tax_proof_id: u32,
	pub tax_amount: u128,
	pub parent_hash: Hash,
	pub seal_stampers: Vec<SealStamper>,
	pub author_id: AccountId32,
}

pub fn to_seal_nonce_hash_message<Hash>(
	block_proof: BlockProof,
	parent_hash: Hash,
) -> SealNonceHashMessage<Hash> {
	SealNonceHashMessage {
		prefix: SEAL_NONCE_PREFIX,
		tax_proof_id: block_proof.tax_proof_id,
		tax_amount: block_proof.tax_amount,
		author_id: block_proof.author_id,
		parent_hash,
		seal_stampers: block_proof.seal_stampers,
	}
}
