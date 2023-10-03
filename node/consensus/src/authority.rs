use std::{marker::PhantomData, sync::Arc};

use codec::{Decode, Encode};
use log::warn;
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::RuntimeAppPublic;
use sp_consensus::Error as ConsensusError;
use sp_core::{crypto::AccountId32, ByteArray};
use sp_keystore::KeystorePtr;
use sp_runtime::{app_crypto::AppCrypto, generic::DigestItem, traits::Block as BlockT};

use ulx_primitives::{
	block_seal::{AuthorityApis, BlockProof},
	BlockSealAuthorityId, BlockSealAuthoritySignature, UlxSeal, ULX_ENGINE_ID,
};

use crate::error::Error;

const LOG_TARGET: &str = "node::consensus::authority";

pub struct AuthoritySealer<Block, C> {
	client: Arc<C>,
	keystore: KeystorePtr,
	_phantom: PhantomData<Block>,
}

impl<Block: BlockT, C> AuthoritySealer<Block, C>
where
	C: ProvideRuntimeApi<Block>,
	C::Api: AuthorityApis<Block>,
{
	pub fn new(client: Arc<C>, keystore: KeystorePtr) -> Self {
		Self { client, keystore, _phantom: PhantomData }
	}

	pub fn block_peers(
		&self,
		block_hash: &Block::Hash,
		account_id: AccountId32,
	) -> Result<Vec<(u16, BlockSealAuthorityId)>, Error<Block>> {
		let peers = self
			.client
			.runtime_api()
			.block_peers(*block_hash, account_id)
			.map_err(|e| Error::StringError(e.to_string()))?;
		Ok(peers
			.into_iter()
			.map(|a| (a.authority_index, a.authority_id))
			.collect::<Vec<_>>())
	}

	pub(crate) fn check_if_can_seal(
		&self,
		block_hash: &Block::Hash,
		block_proof: &BlockProof,
		is_final: bool,
	) -> Result<(u16, BlockSealAuthorityId, Vec<(u16, BlockSealAuthorityId)>), Error<Block>> {
		let authorities = match self.block_peers(block_hash, block_proof.author_id.clone()) {
			Ok(x) => x,
			Err(err) =>
				return Err(Error::BlockProposingError(format!(
					"Could not retrieve authorities: {:?}",
					err.to_string()
				))),
		};

		if block_proof.seal_stampers.len() < 1 {
			return Err(Error::InvalidBlockSubmitter.into())
		}

		let seal_submitter_authority_index = block_proof.seal_stampers[0].authority_idx;

		for (authority_index, authority_id) in authorities.clone() {
			if !self.keystore.has_keys(&[(
				RuntimeAppPublic::to_raw_vec(&authority_id),
				ulx_primitives::BLOCK_SEAL_KEY_TYPE,
			)]) {
				continue
			}

			if is_final && authority_index != seal_submitter_authority_index {
				return Err(Error::InvalidBlockSubmitter.into())
			}

			return Ok((authority_index, authority_id.clone(), authorities))
		}

		warn!( target: LOG_TARGET,
				"Unable to propose new block for authoring. No authority found in keystore");
		Err(Error::NoActiveAuthorityInKeystore)
	}

	pub(crate) fn sign_message(
		&self,
		authority_id: &BlockSealAuthorityId,
		message: &[u8],
	) -> Result<BlockSealAuthoritySignature, Error<Block>> {
		let signature = self
			.keystore
			.sign_with(
				<BlockSealAuthorityId as AppCrypto>::ID,
				<BlockSealAuthorityId as AppCrypto>::CRYPTO_ID,
				authority_id.as_slice(),
				message,
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

	pub(crate) fn sign_seal(
		&self,
		authority_index: u16,
		authority_id: &BlockSealAuthorityId,
		header_hash: &Block::Hash,
		seal: &mut UlxSeal,
	) -> Result<DigestItem, Error<Block>> {
		let signature = self.sign_message(authority_id, header_hash.as_ref())?;
		seal.authority = Some((authority_index, signature));

		Ok(DigestItem::Seal(ULX_ENGINE_ID, seal.encode()))
	}

	pub fn fetch_ulx_seal(
		digest: Option<&DigestItem>,
		hash: Block::Hash,
	) -> Result<UlxSeal, Error<Block>> {
		match digest {
			Some(DigestItem::Seal(id, seal)) =>
				if id == &ULX_ENGINE_ID {
					match UlxSeal::decode(&mut &seal[..]) {
						Ok(seal) => Ok(seal),
						Err(_) => Err(Error::<Block>::InvalidSeal),
					}
				} else {
					Err(Error::<Block>::WrongEngine(*id))
				},
			_ => Err(Error::<Block>::HeaderUnsealed(hash)),
		}
	}

	pub(crate) fn verify_seal_signature(
		client: Arc<C>,
		seal: &UlxSeal,
		parent_hash: &Block::Hash,
		pre_hash: &Block::Hash,
	) -> Result<(), Error<Block>> {
		let (authority_index, signature) = match &seal.authority {
			Some((a, s)) => (a, s),
			None => return Err(Error::<Block>::InvalidSealSignature.into()),
		};

		let authority_id = client
			.runtime_api()
			.authorities_by_index(*parent_hash)
			.map_err(|e| Error::<Block>::StringError(format!("Error loading authorities {:?}", e)))?
			.into_iter()
			.find_map(|(index, authority)| {
				if index == *authority_index {
					return Some(authority)
				}
				None
			})
			.ok_or(Error::<Block>::InvalidSealSignerAuthority)?;

		if !authority_id.verify(&pre_hash.as_ref(), &signature) {
			return Err(Error::<Block>::InvalidSealSignature.into())
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;

	use frame_support::{assert_err, assert_ok};
	use sc_network_test::Block;
	use sp_api::{ApiRef, ProvideRuntimeApi};
	use sp_core::{bounded_vec, crypto::AccountId32, OpaquePeerId, H256};
	use sp_keyring::Ed25519Keyring;
	use sp_keystore::{testing::MemoryKeystore, Keystore};

	use ulx_primitives::block_seal::{AuthorityDistance, PeerId, SealStamper, BLOCK_SEAL_KEY_TYPE};

	use crate::tests::setup_logs;

	use super::*;

	#[derive(Default, Clone)]
	struct TestApi {
		pub authorities_by_index: BTreeMap<u16, BlockSealAuthorityId>,
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
		impl AuthorityApis<Block> for RuntimeApi {
			fn authorities() -> Vec<BlockSealAuthorityId>{
				self.inner.authorities_by_index.iter().map(|(_, id)| id.clone()).collect::<Vec<_>>()
			}
			fn authorities_by_index() -> BTreeMap<u16, BlockSealAuthorityId>{
				self.inner.authorities_by_index.clone()
			}
			fn active_authorities() -> u16{
				self.inner.authorities_by_index.len() as u16
			}
			fn block_peers(_: AccountId32) -> Vec<AuthorityDistance<BlockSealAuthorityId>> {
				self.inner.block_peer_order.clone().into_iter().map(|a| {
					let id= self.inner.authorities_by_index.get(&a).unwrap();
					AuthorityDistance {
						authority_id: id.clone(),
						distance: 0u32.into(),
						authority_index: a,
						peer_id: PeerId(OpaquePeerId::default()),
						rpc_hosts: bounded_vec![],
					}
				}).collect::<Vec<_>>()
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
			authorities_by_index: vec![(0, Ed25519Keyring::Alice.public().into())]
				.into_iter()
				.collect(),
			block_peer_order: vec![0],
		};
		let sealer = AuthoritySealer::<Block, _>::new(Arc::new(api), keystore);
		assert_ok!(sealer.check_if_can_seal(
			&H256::zero(),
			&BlockProof {
				seal_stampers: vec![SealStamper { authority_idx: 0, signature: None }],
				tax_amount: 1,
				tax_proof_id: 1,
				author_id: AccountId32::from_slice(&[0; 32]).ok().unwrap(),
			},
			false
		));
	}

	#[test]
	fn cannot_sign_seal_with_second_index() {
		setup_logs();
		let [bob_keystore] = [create_keystore(Ed25519Keyring::Bob)];
		let api = TestApi {
			authorities_by_index: vec![
				(0, Ed25519Keyring::Alice.public().into()),
				(1, Ed25519Keyring::Bob.public().into()),
			]
			.into_iter()
			.collect(),
			block_peer_order: vec![0, 1],
		};
		let bob_as_sealer = AuthoritySealer::<Block, _>::new(Arc::new(api), bob_keystore);
		assert_err!(
			bob_as_sealer.check_if_can_seal(
				&H256::zero(),
				&BlockProof {
					seal_stampers: vec![SealStamper { authority_idx: 0, signature: None }],
					tax_amount: 1,
					tax_proof_id: 1,
					author_id: AccountId32::from_slice(&[0; 32]).ok().unwrap(),
				},
				true
			),
			Error::InvalidBlockSubmitter
		);
	}

	#[test]
	fn can_create_valid_signatures() {
		setup_logs();
		let keystore = create_keystore(Ed25519Keyring::Alice);
		let api = TestApi {
			authorities_by_index: vec![
				(2, Ed25519Keyring::Bob.public().into()),
				(15, Ed25519Keyring::Alice.public().into()),
			]
			.into_iter()
			.collect(),
			block_peer_order: vec![15, 2],
		};
		let sealer = AuthoritySealer::<Block, _>::new(Arc::new(api.clone()), keystore);
		let block_hash = H256::from([31; 32]);
		let proof = BlockProof {
			seal_stampers: vec![SealStamper { authority_idx: 15, signature: None }],
			tax_amount: 1,
			tax_proof_id: 1,
			author_id: AccountId32::from_slice(&[0; 32]).ok().unwrap(),
		};
		let (auth_idx, auth_id, _) =
			sealer.check_if_can_seal(&block_hash, &proof, true).expect("Can seal");
		assert_eq!(auth_idx, 15);
		assert_eq!(auth_id, Ed25519Keyring::Alice.public().into());

		let mut seal = UlxSeal { easing: 0, nonce: [1; 32], authority: None };
		assert_ok!(sealer.sign_seal(auth_idx, &auth_id, &block_hash, &mut seal));
		assert_eq!(&seal.authority.clone().map(|a| a.0), &Some(15u16));

		assert_ok!(AuthoritySealer::<Block, _>::verify_seal_signature(
			Arc::new(api.clone()),
			&seal.clone(),
			&block_hash,
			&block_hash
		));

		seal.authority = Some((2, seal.authority.unwrap().1));
		assert_err!(
			AuthoritySealer::<Block, _>::verify_seal_signature(
				Arc::new(api),
				&seal,
				&block_hash,
				&block_hash
			),
			Error::InvalidSealSignature
		);
	}
}
