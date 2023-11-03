use subxt::{ext::sp_core::Encode, utils::AccountId32};

use crate::api::runtime_types::ulx_primitives::block_seal::{BlockProof, SealStamper};

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
