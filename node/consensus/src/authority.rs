use std::{marker::PhantomData, sync::Arc};

use sp_api::ProvideRuntimeApi;
use sp_application_crypto::RuntimeAppPublic;
use sp_consensus::Error as ConsensusError;
use sp_core::{crypto::AccountId32, ByteArray, U256};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{app_crypto::AppCrypto, traits::Block as BlockT};

use ulx_primitives::{
	block_seal::MiningAuthority, BlockSealAuthorityId, BlockSealAuthoritySignature,
	BlockSealDigest, MiningAuthorityApis,
};

use crate::error::Error;

pub struct AuthorityClient<Block, C> {
	client: Arc<C>,
	keystore: KeystorePtr,
	_phantom: PhantomData<Block>,
}

impl<Block: BlockT, C> AuthorityClient<Block, C>
where
	C: ProvideRuntimeApi<Block>,
	C::Api: MiningAuthorityApis<Block>,
{
	pub fn new(client: Arc<C>, keystore: KeystorePtr) -> Self {
		Self { client, keystore, _phantom: PhantomData }
	}

	pub fn block_peer(
		&self,
		block_hash: &Block::Hash,
		account_id: AccountId32,
	) -> Result<Option<MiningAuthority<BlockSealAuthorityId>>, Error<Block>> {
		let peer = self
			.client
			.runtime_api()
			.block_peer(*block_hash, account_id)
			.map_err(|e| Error::StringError(e.to_string()))?;
		Ok(peer)
	}

	pub fn is_active_authority(
		&self,
		block_hash: &Block::Hash,
		authority_id: &BlockSealAuthorityId,
	) -> bool {
		self.client
			.runtime_api()
			.is_valid_authority(block_hash.clone(), authority_id.clone())
			.unwrap_or(false)
	}

	pub fn get_preferred_authority(
		&self,
		block_hash: &Block::Hash,
	) -> Option<BlockSealAuthorityId> {
		let keys = self.keystore.ed25519_public_keys(ulx_primitives::BLOCK_SEAL_KEY_TYPE);

		for key in keys.iter() {
			let key: BlockSealAuthorityId = key.clone().into();
			if self.is_active_authority(block_hash, &key) {
				return Some(key)
			}
		}
		keys.first().cloned().map(|k| k.into())
	}

	pub(crate) fn check_if_can_seal_tax_vote(
		&self,
		block_hash: &Block::Hash,
		account_id: &AccountId32,
	) -> Result<BlockSealAuthorityId, Error<Block>> {
		Self::can_seal_tax_vote(&self.client, &self.keystore, block_hash, account_id)
	}

	pub(crate) fn can_seal_tax_vote(
		client: &Arc<C>,
		keystore: &KeystorePtr,
		block_hash: &Block::Hash,
		account_id: &AccountId32,
	) -> Result<BlockSealAuthorityId, Error<Block>> {
		let authority = match client.runtime_api().block_peer(*block_hash, account_id.clone()) {
			Ok(x) => x,
			Err(err) =>
				return Err(Error::BlockProposingError(format!(
					"Could not retrieve authorities: {:?}",
					err.to_string()
				))),
		};

		if let Some(authority) = authority {
			if !keystore.has_keys(&[(
				RuntimeAppPublic::to_raw_vec(&authority.authority_id),
				ulx_primitives::BLOCK_SEAL_KEY_TYPE,
			)]) {
				return Err(Error::NoActiveAuthorityInKeystore)
			}

			return Ok(authority.authority_id)
		}

		Err(Error::NoActiveAuthorityInKeystore)
	}
}

pub(crate) fn verify_seal_signature<Block: BlockT>(
	seal: &BlockSealDigest,
	pre_hash: &Block::Hash,
	authority_id: &BlockSealAuthorityId,
) -> Result<(), Error<Block>> {
	let mut nonce_bytes = [0u8, 32];
	seal.nonce.to_big_endian(&mut nonce_bytes);
	let message = &[&nonce_bytes, pre_hash.as_ref()].concat();
	if !authority_id.verify(&message, &seal.signature) {
		return Err(Error::<Block>::InvalidSealSignature.into())
	}

	Ok(())
}

pub(crate) fn sign_seal(
	keystore: &KeystorePtr,
	authority_id: &BlockSealAuthorityId,
	pre_hash: &[u8],
	nonce: &U256,
) -> Result<BlockSealAuthoritySignature, ConsensusError> {
	let mut nonce_bytes = [0u8, 32];
	nonce.to_big_endian(&mut nonce_bytes);
	let signature = keystore
		.sign_with(
			<BlockSealAuthorityId as AppCrypto>::ID,
			<BlockSealAuthorityId as AppCrypto>::CRYPTO_ID,
			authority_id.as_slice(),
			&[&nonce_bytes, pre_hash].concat(),
		)
		.map_err(|e| ConsensusError::CannotSign(format!("{}. Key: {:?}", e, authority_id)))?
		.ok_or_else(|| {
			ConsensusError::CannotSign(format!(
				"Could not find key in keystore. Key: {:?}",
				authority_id
			))
		})?;

	let signature = signature.try_into().map_err(|_| {
		ConsensusError::CannotSign(format!(
			"Could not create a valid signature. Key: {:?}",
			authority_id
		))
	})?;
	Ok(signature)
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;

	use frame_support::assert_ok;
	use sc_network_test::Block;
	use sp_api::{ApiRef, ProvideRuntimeApi};
	use sp_core::{bounded_vec, crypto::AccountId32, OpaquePeerId, H256};
	use sp_keyring::Ed25519Keyring;
	use sp_keystore::{testing::MemoryKeystore, Keystore};

	use ulx_primitives::{
		block_seal::{MiningAuthority, PeerId, BLOCK_SEAL_KEY_TYPE},
		digests::SealSource,
	};

	use crate::tests::setup_logs;

	use super::*;

	#[derive(Default, Clone)]
	struct TestApi {
		pub authority_id_by_index: BTreeMap<u16, BlockSealAuthorityId>,
		pub block_peer_order: Vec<u16>,
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
		impl MiningAuthorityApis<Block> for RuntimeApi {
			fn is_valid_authority(_: BlockSealAuthorityId) -> bool {
				true
			}

			fn authority_id_for_account(_: AccountId32) -> Option<BlockSealAuthorityId> {
				None
			}

			fn authority_id_by_index() -> BTreeMap<u16, BlockSealAuthorityId>{
				self.inner.authority_id_by_index.clone()
			}
			fn active_authorities() -> u16{
				self.inner.authority_id_by_index.len() as u16
			}
			fn block_peer(_: AccountId32) -> Option<MiningAuthority<BlockSealAuthorityId>> {
				self.inner.block_peer_order.first().map(|a| {
					let id= self.inner.authority_id_by_index.get(&a).unwrap();
					MiningAuthority {
						authority_id: id.clone(),
						authority_index: *a,
						peer_id: PeerId(OpaquePeerId::default()),
						rpc_hosts: bounded_vec![],
					}
				})
			}
		}
	}

	fn create_keystore(authority: Ed25519Keyring) -> KeystorePtr {
		let keystore = MemoryKeystore::new();
		keystore
			.ed25519_generate_new(BLOCK_SEAL_KEY_TYPE, Some(&authority.to_seed()))
			.expect("Creates authority key");
		keystore.into()
	}

	#[test]
	fn it_finds_local_authority_for_seal() {
		setup_logs();
		let keystore = create_keystore(Ed25519Keyring::Alice);
		let api = TestApi {
			authority_id_by_index: vec![(0, Ed25519Keyring::Alice.public().into())]
				.into_iter()
				.collect(),
			block_peer_order: vec![0],
		};
		let sealer = AuthorityClient::<Block, _>::new(Arc::new(api), keystore);
		assert_ok!(sealer.check_if_can_seal_tax_vote(
			&H256::zero(),
			&AccountId32::from_slice(&[0; 32]).ok().unwrap(),
		));
	}

	#[test]
	fn can_create_valid_signatures() {
		setup_logs();
		let keystore = create_keystore(Ed25519Keyring::Alice);

		let block_hash = H256::from([31; 32]);
		let nonce = U256::from(1);
		let signature = sign_seal(
			&keystore,
			&Ed25519Keyring::Alice.public().into(),
			&block_hash.as_bytes(),
			&nonce,
		)
		.expect("Can sign");

		assert_ok!(verify_seal_signature::<Block>(
			&BlockSealDigest { nonce, seal_source: SealSource::Compute, signature },
			&block_hash,
			&Ed25519Keyring::Alice.public().into()
		));
	}
}
