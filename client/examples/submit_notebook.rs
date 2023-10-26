use codec::Encode;
use sp_core::crypto::{AccountId32, KeyTypeId};
use sp_keystore::{testing::MemoryKeystore, Keystore};
use std::{convert::Into, net::Ipv4Addr};
use subxt::{ext::sp_core::blake2_256, utils::H256, OnlineClient};
use subxt_signer::{sr25519, sr25519::dev};

use ulixee_client::{
	api,
	api::runtime_types::{
		bounded_collections::bounded_vec::BoundedVec,
		sp_core::ed25519::Signature,
		ulx_primitives::{block_seal::Host, notary::NotaryMeta, notebook::Notebook},
	},
	local_client,
	signature_messages::to_notebook_post_hash,
	UlxConfig,
};

const NOTARY: KeyTypeId = KeyTypeId(*b"not_");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Create a client to use:
	let client = local_client().await?;

	let notary_operator = dev::bob();
	let keystore = MemoryKeystore::new();
	let public = keystore.ed25519_generate_new(NOTARY, None)?;

	let register_tx = api::tx().notaries().propose(NotaryMeta {
		hosts: BoundedVec(
			vec![Host { ip: Ipv4Addr::new(127, 0, 0, 1).into(), port: 1668, is_secure: false }]
				.into(),
		),
		public: api::runtime_types::sp_core::ed25519::Public(public.0),
	});

	let events = client
		.tx()
		.sign_and_submit_then_watch_default(&register_tx, &notary_operator)
		.await?
		.wait_for_in_block()
		.await?
		.wait_for_success()
		.await?;

	let propose_event = events.find_first::<api::notaries::events::NotaryProposed>()?.unwrap();
	let account_id = AccountId32::from(notary_operator.public_key().0);
	println!("Proposed self as notary (self ={:?}),{propose_event:?}", account_id.to_string());

	// wait for the proposal to be accepted
	let notary_id = wait_for_notary_activated(&client, &notary_operator).await?;
	let finalized_query = api::storage().localchain_relay().finalized_block_number();
	let finalized_height =
		client.storage().at_latest().await?.fetch(&finalized_query).await?.unwrap();

	println!("Creating notebook for notary_id {notary_id} at block {finalized_height})",);
	let notebook = Notebook {
		notebook_number: 1,
		notary_id,
		auditors: BoundedVec(vec![].into()),
		transfers: BoundedVec(vec![].into()),
		pinned_to_block_number: finalized_height,
	};

	let notebook_hash = H256(to_notebook_post_hash(&notebook).using_encoded(blake2_256));
	let signature =
		Signature(keystore.ed25519_sign(NOTARY, &public, &notebook_hash[..])?.unwrap().0);

	let tx = api::tx().localchain_relay().submit_notebook(
		notebook_hash.clone(),
		notebook.clone(),
		signature,
	);

	let ext = client.tx().create_unsigned(&tx)?;
	println!(
		"Submitting notebook (notebook_hash = {:?})\n{}",
		notebook_hash,
		hex::encode(ext.encoded())
	);

	let message = ext
		.submit_and_watch()
		.await?
		.wait_for_in_block()
		.await?
		.wait_for_success()
		.await?;

	let notebook_event = message
		.find_first::<api::localchain_relay::events::NotebookSubmitted>()?
		.unwrap();
	println!("Notebook submitted event {:?}", notebook_event);

	Ok(())
}

async fn wait_for_notary_activated(
	client: &OnlineClient<UlxConfig>,
	operator: &sr25519::Keypair,
) -> Result<u32, subxt::Error> {
	let mut blocks_sub = client.blocks().subscribe_best().await?;

	println!("Waiting for notary activation",);
	while let Some(block) = blocks_sub.next().await {
		let block = block?;
		if let Some(approved_event) =
			block.events().await?.find_first::<api::notaries::events::NotaryActivated>()?
		{
			if approved_event.notary.operator_account_id == operator.public_key().into() {
				println!("Activated as notary {approved_event:?}",);
				return Ok(approved_event.notary.notary_id)
			}
		}
	}
	Ok(0)
}
