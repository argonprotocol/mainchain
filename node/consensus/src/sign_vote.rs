use std::sync::Arc;

use sp_api::ProvideRuntimeApi;
use sp_consensus::Error as ConsensusError;
use sp_core::{crypto::AccountId32, ByteArray, U256};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{app_crypto::AppCrypto, traits::Block as BlockT};

use ulx_primitives::{
	localchain::BlockVote, BlockSealAuthorityId, BlockSealAuthoritySignature, MiningAuthorityApis,
	BLOCK_SEAL_KEY_TYPE,
};

use crate::error::Error;

pub fn sign_vote<Block: BlockT, C>(
	client: &Arc<C>,
	keystore: &KeystorePtr,
	block_hash: &Block::Hash,
	vote_proof: U256,
) -> Result<(BlockSealAuthoritySignature, AccountId32), Error<Block>>
where
	C: ProvideRuntimeApi<Block>,
	C::Api: MiningAuthorityApis<Block>,
{
	let (authority_id, account_id) =
		match client.runtime_api().xor_closest_authority(*block_hash, vote_proof) {
			Ok(Some(x)) => (x.authority_id, x.account_id),
			Ok(None) =>
				return Err(Error::BlockProposingError(
					"Could not find authority for vote proof signature".to_string(),
				)),
			Err(err) =>
				return Err(Error::BlockProposingError(format!(
					"Could not retrieve authorities: {:?}",
					err.to_string()
				))),
		};

	if !keystore.has_keys(&[(authority_id.to_raw_vec(), BLOCK_SEAL_KEY_TYPE)]) {
		return Err(Error::NoActiveAuthorityInKeystore)
	}

	let message = BlockVote::vote_proof_signature_message(vote_proof);
	let signature = keystore
		.sign_with(
			<BlockSealAuthorityId as AppCrypto>::ID,
			<BlockSealAuthorityId as AppCrypto>::CRYPTO_ID,
			&authority_id.to_raw_vec(),
			&message,
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
	Ok((signature, account_id))
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

	use ulx_primitives::block_seal::{MiningAuthority, PeerId, BLOCK_SEAL_KEY_TYPE};

	use crate::tests::setup_logs;

	use super::*;

	#[derive(Default, Clone)]
	struct TestApi {
		pub authority_id_by_index: BTreeMap<u16, (AccountId32, BlockSealAuthorityId)>,
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
			fn xor_closest_authority(_: U256) -> Option<MiningAuthority<BlockSealAuthorityId, AccountId32>> {
				self.inner.block_peer_order.first().map(|a| {
					let (account_id, id)= self.inner.authority_id_by_index.get(&a).unwrap();
					MiningAuthority {
						account_id: account_id.clone(),
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
			authority_id_by_index: vec![(
				0,
				(Ed25519Keyring::Alice.public().into(), Ed25519Keyring::Alice.public().into()),
			)]
			.into_iter()
			.collect(),
			block_peer_order: vec![0],
		};
		assert_ok!(sign_vote(&Arc::new(api), &keystore, &H256::zero(), U256::from(1)));
	}

	#[test]
	fn it_fails_if_not_the_right_authority() {
		setup_logs();
		let keystore = create_keystore(Ed25519Keyring::Alice);

		let block_hash = H256::from([31; 32]);
		let nonce = U256::from(1);
		let api = TestApi {
			authority_id_by_index: vec![
				(0, (Ed25519Keyring::Alice.public().into(), Ed25519Keyring::Alice.public().into())),
				(2, (Ed25519Keyring::Bob.public().into(), Ed25519Keyring::Bob.public().into())),
			]
			.into_iter()
			.collect(),
			block_peer_order: vec![2],
		};
		assert_err!(
			sign_vote(&Arc::new(api), &keystore, &block_hash, nonce),
			Error::NoActiveAuthorityInKeystore
		);
	}
}
