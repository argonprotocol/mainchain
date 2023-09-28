use std::{marker::PhantomData, sync::Arc};

use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::UniqueSaturatedInto;
use sp_core::{blake2_256, U256};
use sp_runtime::traits::Block as BlockT;

use ulx_primitives::{
	block_seal::BlockProof, Difficulty, NextWork, ProofOfWorkType, UlxConsensusApi, UlxPreDigest,
	UlxSeal,
};

use crate::{Error, NonceAlgorithm};

pub struct UlxNonce<B: BlockT, C> {
	client: Arc<C>,
	_block: PhantomData<B>,
}

impl<B: BlockT, C> UlxNonce<B, C> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _block: PhantomData }
	}
}

impl<B: BlockT, C> Clone for UlxNonce<B, C> {
	fn clone(&self) -> Self {
		Self::new(self.client.clone())
	}
}

impl<B: BlockT, C> NonceAlgorithm<B> for UlxNonce<B, C>
where
	C: ProvideRuntimeApi<B>,
	C::Api: UlxConsensusApi<B>,
{
	type Difficulty = Difficulty;

	fn easing(&self, parent: &B::Hash, block_proof: &BlockProof) -> Result<u128, Error<B>> {
		self.easing(
			*parent,
			block_proof.tax_amount,
			block_proof
				.seal_stampers
				.iter()
				.filter(|x| x.signature.is_some())
				.count()
				.unique_saturated_into(),
		)
	}
	// Get the next block's difficulty
	fn next_digest(&self, parent: &B::Hash) -> Result<UlxPreDigest, Error<B>> {
		let next_work = self.next_work(*parent)?;
		Ok(UlxPreDigest { work_type: next_work.work_type, difficulty: next_work.difficulty })
	}

	// Verify that the difficulty is valid against given UlxSeal
	fn verify(
		&self,
		parent: &B::Hash,
		pre_hash: &B::Hash,
		pre_digest: &UlxPreDigest,
		seal: &UlxSeal,
	) -> Result<bool, Error<B>> {
		// if the work type is tax, we'll match our nonce against the parent hash
		let hash: &B::Hash = match pre_digest.work_type {
			ProofOfWorkType::Tax => parent,
			ProofOfWorkType::Compute => pre_hash,
		};

		let mut verifier = NonceVerifier::<B>::new(hash, pre_digest);
		Ok(verifier.is_nonce_valid(&seal.nonce))
	}
}

impl<B: BlockT, C> UlxNonce<B, C>
where
	C: ProvideRuntimeApi<B>,
	C::Api: UlxConsensusApi<B>,
{
	fn next_work(&self, block_hash: B::Hash) -> Result<NextWork, Error<B>> {
		self.client
			.runtime_api()
			.next_work(block_hash)
			.map_err(|_a| Error::CantGetNextWork.into())
	}
	fn easing(
		&self,
		block_hash: B::Hash,
		tax_amount: u128,
		validators: u8,
	) -> Result<u128, Error<B>> {
		self.client
			.runtime_api()
			.calculate_easing(block_hash, tax_amount, validators)
			.map_err(|_a| Error::CantGetNextWork.into())
	}
}

pub struct NonceVerifier<B: BlockT> {
	payload: Vec<u8>,
	threshold: U256,
	_block: PhantomData<B>,
}

impl<B: BlockT> Clone for NonceVerifier<B> {
	fn clone(&self) -> Self {
		NonceVerifier {
			payload: self.payload.clone(),
			threshold: self.threshold.clone(),
			_block: PhantomData,
		}
	}
}

/// The difficulty is an average number of hashes that need to be checked in order mine a block.
///
/// In order to check if the data hash passes the difficulty test, it must be interpreted as
/// a big-endian 256-bit number. If it's smaller than or equal to the threshold, it passes.
/// The threshold is calculated from difficulty as `U256::max_value / difficulty`.
impl<B: BlockT> NonceVerifier<B> {
	pub fn new(pre_hash: &B::Hash, pre_digest: &UlxPreDigest) -> Self {
		NonceVerifier {
			payload: pre_hash.as_ref().to_vec(),
			threshold: U256::MAX / U256::from(pre_digest.difficulty).max(U256::from(1)),
			_block: PhantomData,
		}
	}

	pub fn is_nonce_valid(&mut self, nonce: &[u8]) -> bool {
		let original_payload_len = self.payload.len();
		self.payload.extend_from_slice(nonce);
		let hash = blake2_256(&self.payload);
		self.payload.truncate(original_payload_len);

		let hash_value = U256::from_big_endian(&hash[..]);
		hash_value <= self.threshold
	}
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;
	use sc_network_test::Block;
	use sp_api::{ApiRef, ProvideRuntimeApi};
	use sp_core::H256;

	use crate::tests::setup_logs;

	use super::*;

	#[derive(Default, Clone)]
	struct TestApi {
		pub work_type: ProofOfWorkType,
		pub difficulty: u128,
		pub easing: u64,
	}

	struct RuntimeApi {
		inner: TestApi,
	}

	impl ProvideRuntimeApi<Block> for TestApi {
		type Api = RuntimeApi;

		fn runtime_api(&self) -> ApiRef<'_, Self::Api> {
			RuntimeApi { inner: self.clone() }.into()
		}
	}

	sp_api::mock_impl_runtime_apis! {
		impl UlxConsensusApi<Block> for RuntimeApi {
			fn next_work(&self) -> NextWork {
				NextWork {
					work_type: self.inner.work_type,
					difficulty: self.inner.difficulty,
					min_seal_signers: 3,
					closest_x_authorities_required: 1
				}
			}
			fn calculate_easing(
				&self,
				_tax_amount: u128,
				_validators: u8,
			) -> u128 {
				self.inner.easing.into()
			}
		}
	}

	#[test]
	fn nonce_verify_compute() {
		setup_logs();

		let client = TestApi { work_type: ProofOfWorkType::Compute, difficulty: 2, easing: 1 };
		let nonce = UlxNonce::<Block, TestApi>::new(Arc::new(client.clone()));
		let mut bytes = [0u8; 32];
		bytes[31] = 1;
		let parent_hash = H256::from(bytes.clone());

		bytes[31] = 1;
		let nonce_bytes = bytes.clone();

		let next_work = nonce.next_work(parent_hash).unwrap();
		let pre_digest =
			UlxPreDigest { work_type: next_work.work_type, difficulty: next_work.difficulty };

		let seal = UlxSeal { easing: 0, nonce: nonce_bytes, authority: None };
		bytes[31] = 2;
		let pre_hash = H256::from(bytes.clone());
		assert_eq!(nonce.verify(&parent_hash, &pre_hash, &pre_digest, &seal).ok(), Some(true));
	}

	#[test]
	fn nonce_verify_tax() {
		setup_logs();

		let client = TestApi { work_type: ProofOfWorkType::Tax, difficulty: 2, easing: 1 };
		let nonce = UlxNonce::<Block, TestApi>::new(Arc::new(client.clone()));
		let mut bytes = [0u8; 32];
		bytes[31] = 2;
		let parent_hash = H256::from(bytes.clone());
		bytes[31] = 1;
		let nonce_bytes = bytes.clone();

		let next_work = nonce.next_work(parent_hash).unwrap();
		let pre_digest =
			UlxPreDigest { work_type: next_work.work_type, difficulty: next_work.difficulty };

		let seal = UlxSeal { easing: 0, nonce: nonce_bytes, authority: None };
		// pre hash will not hash now... confirm it uses parent
		bytes[31] = 1;
		let pre_hash = H256::from(bytes.clone());
		assert_eq!(nonce.verify(&parent_hash, &pre_hash, &pre_digest, &seal).ok(), Some(true));
	}

	#[test]
	fn it_can_reuse_a_nonce_algorithm_multiple_times() {
		setup_logs();

		let client = TestApi { work_type: ProofOfWorkType::Tax, difficulty: 2, easing: 1 };
		let nonce = UlxNonce::<Block, TestApi>::new(Arc::new(client.clone()));
		let mut bytes = [0u8; 32];
		bytes[31] = 2;
		let parent_hash = H256::from(bytes.clone());
		let pre_hash = H256::from(bytes.clone());

		let next_work = nonce.next_work(parent_hash).unwrap();
		let pre_digest =
			UlxPreDigest { work_type: next_work.work_type, difficulty: next_work.difficulty };

		for i in 0..100 {
			let mut nonce_bytes = [0u8; 32];
			nonce_bytes[31] = i % 32;
			let seal = UlxSeal { easing: 0, nonce: nonce_bytes, authority: None };
			assert_ok!(nonce.verify(&parent_hash, &pre_hash, &pre_digest, &seal));
		}
	}
}
