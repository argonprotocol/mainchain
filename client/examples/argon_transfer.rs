use polkadot_sdk::*;
use sp_core::Pair;
use sp_keyring::Sr25519Keyring::{Alice, Bob};
use subxt::{Config, OnlineClient, tx::TxStatus};

use argon_client::{
	ArgonConfig, ArgonExtrinsicParamsBuilder, ArgonOnlineClient, api, signer::Sr25519Signer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Create a client to use:
	let client = ArgonOnlineClient::new().await?;

	let bob = Bob.pair();
	let bob_signer = Sr25519Signer::new(bob.clone());

	let account = bob.public().into();

	// NOTE: argon balances are stored in system.. not the pallet itself
	let balance_query = api::storage().system().account(&account);
	let result = client.storage().at_latest().await?.fetch(&balance_query).await?;
	println!("Bob has free balance: {:?}, {}", result.unwrap().data.free, account);

	let alice_account = argon_client::types::AccountId32::from(Alice.to_account_id());
	let transfer_query = api::tx().balances().transfer_allow_death(alice_account.into(), 1_000_000);

	// transaction to live for 32 blocks from the `latest_block` above.
	let tx_params = ArgonExtrinsicParamsBuilder::<ArgonConfig>::new().tip(100).mortal(32).build();

	let mut sub = client
		.tx()
		.sign_and_submit_then_watch(&transfer_query, &bob_signer, tx_params)
		.await?;

	while let Some(status) = sub.next().await {
		match status? {
			TxStatus::InBestBlock(in_block) => {
				// Find a Transfer event and print it.
				let events = in_block.fetch_events().await?;
				let transfer_event = events.find_first::<api::balances::events::Transfer>()?;
				if let Some(event) = transfer_event {
					println!(
						"Transaction {:?} is in best block {} ({:?}).\n{event:?}",
						in_block.extrinsic_hash(),
						get_block_number(client.clone(), in_block.block_hash()).await?,
						in_block.block_hash()
					);
				}
			},
			// Finalized or otherwise in a block! Return.
			TxStatus::InFinalizedBlock(in_block) => {
				println!(
					"Transaction is finalized in block {} ({:?})",
					get_block_number(client.clone(), in_block.block_hash()).await?,
					in_block.block_hash()
				);
			},
			TxStatus::Dropped { message } |
			TxStatus::Error { message } |
			TxStatus::Invalid { message } => {
				println!("Error submitting transaction: {message:?}");
			},
			// Just log any other status we encounter:
			other => {
				println!("status: {other:?}");
			},
		}
	}

	Ok(())
}

async fn get_block_number(
	client: OnlineClient<ArgonConfig>,
	hash: <ArgonConfig as Config>::Hash,
) -> Result<u32, subxt::Error> {
	client
		.backend()
		.block_header(hash)
		.await?
		.map(|a| a.number)
		.ok_or_else(|| subxt::Error::Other("Block header not found for block hash".to_string()))
}
