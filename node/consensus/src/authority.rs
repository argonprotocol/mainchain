use std::{marker::PhantomData, sync::Arc};

use codec::{Codec, Decode, Encode};
use log::warn;
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::RuntimeAppPublic;
use sp_consensus::Error as ConsensusError;
use sp_core::ByteArray;
use sp_keystore::KeystorePtr;
use sp_runtime::{app_crypto::AppCrypto, generic::DigestItem, traits::Block as BlockT};

use ulx_primitives::{AuthorityApis, BlockProof, BlockSealAuthorityId, UlxSeal, ULX_ENGINE_ID};

use crate::error::Error;

const LOG_TARGET: &str = "node::consensus::authority";

pub struct AuthoritySealer<Block, C, AccountId> {
	client: Arc<C>,
	keystore: KeystorePtr,
	discovered_local_authority: Option<(u16, BlockSealAuthorityId)>,
	_phantom: PhantomData<(Block, AccountId)>,
}

impl<Block: BlockT, C, AccountId> AuthoritySealer<Block, C, AccountId>
where
	C: ProvideRuntimeApi<Block>,
	C::Api: AuthorityApis<Block>,
	AccountId: Codec,
{
	pub fn new(client: Arc<C>, keystore: KeystorePtr) -> Self {
		Self { client, keystore, discovered_local_authority: None, _phantom: PhantomData }
	}

	pub(crate) fn check_if_can_seal(
		&mut self,
		best_hash: &Block::Hash,
		block_proof: &BlockProof,
	) -> Result<(), Error<Block>> {
		let authorities = match self.client.runtime_api().authorities_by_index(*best_hash) {
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

		let seal_submitter_authority_index =
			block_proof.seal_stampers.get(0).map(|a| a.authority_idx);

		let authority_keys = authorities.into_iter().find(|(_, auth)| {
			self.keystore.has_keys(&[(
				RuntimeAppPublic::to_raw_vec(auth),
				ulx_primitives::BLOCK_SEAL_KEY_TYPE,
			)])
		});

		if authority_keys.is_none() {
			warn!( target: LOG_TARGET,
				"Unable to propose new block for authoring. No authority found in keystore");
			return Err(Error::NoActiveAuthorityInKeystore.into())
		}

		if let Some((keystore_authority_index, authority_id)) = authority_keys {
			if seal_submitter_authority_index.unwrap() != keystore_authority_index {
				return Err(Error::InvalidBlockSubmitter.into())
			}

			self.discovered_local_authority =
				Some((keystore_authority_index, authority_id.clone()));
		}

		Ok(())
	}

	pub(crate) fn sign_seal(
		&self,
		header_hash: &Block::Hash,
		seal: &mut UlxSeal,
	) -> Result<DigestItem, Error<Block>> {
		let (authority_index, authority_id) = match self.discovered_local_authority {
			Some((index, ref id)) => (index, id),
			None => return Err(Error::NoActiveAuthorityInKeystore.into()),
		};

		let signature = self
			.keystore
			.sign_with(
				<BlockSealAuthorityId as AppCrypto>::ID,
				<BlockSealAuthorityId as AppCrypto>::CRYPTO_ID,
				authority_id.as_slice(),
				header_hash.as_ref(),
			)
			.map_err(|e| ConsensusError::CannotSign(format!("{}. Key: {:?}", e, authority_id)))?
			.ok_or_else(|| {
				ConsensusError::CannotSign(format!(
					"Could not find key in keystore. Key: {:?}",
					authority_id
				))
			})?;

		let signature = signature.clone().try_into().map_err(|_| {
			ConsensusError::InvalidSignature(signature, RuntimeAppPublic::to_raw_vec(authority_id))
		})?;

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
	use sp_core::{crypto::AccountId32, H256};
	use sp_keyring::Ed25519Keyring;
	use sp_keystore::{testing::MemoryKeystore, Keystore};

	use ulx_node_runtime::AccountId;
	use ulx_primitives::{AuthorityDistance, SealStamper, BLOCK_SEAL_KEY_TYPE};

	use crate::tests::setup_logs;

	use super::*;

	#[derive(Default, Clone)]
	struct TestApi {
		pub authorities_by_index: BTreeMap<u16, BlockSealAuthorityId>,
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
			fn xor_closest_validators(_hash: Vec<u8>) -> Vec<AuthorityDistance<BlockSealAuthorityId>>{
				vec![]
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
		};
		let mut sealer = AuthoritySealer::<Block, _, AccountId>::new(Arc::new(api), keystore);
		assert_ok!(sealer.check_if_can_seal(
			&H256::zero(),
			&BlockProof {
				seal_stampers: vec![SealStamper { authority_idx: 0, signature: None }],
				tax_amount: 1,
				tax_proof_id: 1,
				author_id: AccountId32::from_slice(&[0; 32]).ok().unwrap(),
			},
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
		};
		let mut bob_as_sealer =
			AuthoritySealer::<Block, _, AccountId>::new(Arc::new(api), bob_keystore);
		assert_err!(
			bob_as_sealer.check_if_can_seal(
				&H256::zero(),
				&BlockProof {
					seal_stampers: vec![SealStamper { authority_idx: 0, signature: None }],
					tax_amount: 1,
					tax_proof_id: 1,
					author_id: AccountId32::from_slice(&[0; 32]).ok().unwrap(),
				},
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
		};
		let mut sealer =
			AuthoritySealer::<Block, _, AccountId>::new(Arc::new(api.clone()), keystore);
		let block_hash = H256::from([31; 32]);
		let proof = BlockProof {
			seal_stampers: vec![SealStamper { authority_idx: 15, signature: None }],
			tax_amount: 1,
			tax_proof_id: 1,
			author_id: AccountId32::from_slice(&[0; 32]).ok().unwrap(),
		};
		assert_ok!(sealer.check_if_can_seal(&block_hash, &proof));

		let mut seal = UlxSeal { easing: 0, nonce: [1; 32], authority: None };
		assert_ok!(sealer.sign_seal(&block_hash, &mut seal));
		assert_eq!(&seal.authority.clone().map(|a| a.0), &Some(15u16));

		assert_ok!(AuthoritySealer::<Block, _, AccountId>::verify_seal_signature(
			Arc::new(api.clone()),
			&seal.clone(),
			&block_hash,
			&block_hash
		));

		seal.authority = Some((2, seal.authority.unwrap().1));
		assert_err!(
			AuthoritySealer::<Block, _, AccountId>::verify_seal_signature(
				Arc::new(api),
				&seal,
				&block_hash,
				&block_hash
			),
			Error::InvalidSealSignature
		);
	}
}
