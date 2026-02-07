use anyhow::bail;
use argon_bitcoin::client::Client;
use argon_client::{
	ArgonConfig, FetchAt, ReconnectingClient,
	api::{runtime_types::argon_primitives::bitcoin as bitcoin_primitives_subxt, storage, tx},
	signer::Signer,
	subxt_error,
};
use argon_primitives::bitcoin::{BitcoinNetwork, H256Le};
use bitcoincore_rpc::{Auth, RpcApi};
use std::time::Duration;
use tokio::time::sleep;

const CONFIRMATIONS: u64 = 6;

pub async fn bitcoin_loop(
	bitcoin_rpc_url: String,
	bitcoin_rpc_auth: Option<(String, String)>,
	mainchain_rpc_url: String,
	signer: impl Signer<ArgonConfig> + Send + Sync + 'static,
) -> anyhow::Result<()> {
	let mut mainchain_client = ReconnectingClient::new(vec![mainchain_rpc_url.clone()]);
	let auth = if let Some((username, password)) = bitcoin_rpc_auth {
		Auth::UserPass(username, password)
	} else {
		Auth::None
	};
	let mut bitcoin_client = Client::new(&bitcoin_rpc_url, auth)?;
	bitcoin_client.timeout = Duration::from_secs(60);
	tracing::info!("Oracle Started. Connected to bitcoin at {}", bitcoin_rpc_url);

	let required_bitcoin_network: BitcoinNetwork = mainchain_client
		.get()
		.await?
		.fetch_storage(&storage().bitcoin_utxos().bitcoin_network(), FetchAt::Finalized)
		.await?
		.expect("Expected network")
		.into();
	let connected_bitcoin_network = bitcoin_client.get_blockchain_info()?.chain.into();
	if required_bitcoin_network != connected_bitcoin_network {
		bail!(
			"Connected to incorrect bitcoin network. Expected {required_bitcoin_network:?}, but connected to {connected_bitcoin_network:?}"
		);
	}

	let mut last_confirmed_tip = None;
	let account_id = signer.account_id();
	loop {
		let blockchain_info = bitcoin_client.get_block_count()?;
		let bitcoin_confirmed_height = blockchain_info.saturating_sub(CONFIRMATIONS);

		let bitcoin_tip = bitcoin_client.get_block_hash(bitcoin_confirmed_height)?;
		if Some(bitcoin_tip) == last_confirmed_tip {
			sleep(Duration::from_secs(10)).await;
			continue;
		}
		last_confirmed_tip = Some(bitcoin_tip);

		let bitcoin_tip: H256Le = bitcoin_tip.into();

		let latest_block = tx().bitcoin_utxos().set_confirmed_block(
			bitcoin_confirmed_height,
			bitcoin_primitives_subxt::H256Le(bitcoin_tip.0),
		);

		let client = mainchain_client.get().await?;
		let params = client.params_with_best_nonce(&account_id).await?.build();
		match client.submit_tx(&latest_block, &signer, Some(params), false).await {
			Ok(_) => {
				tracing::info!(bitcoin_confirmed_height, ?bitcoin_tip, "Submitted bitcoin tip",);
			},
			Err(e) => {
				// Try to detect the common "Invalid Transaction (1010)" failure mode.
				// Depending on the RPC stack, this may surface either as a typed `TransactionError`
				// somewhere in the error chain *or* only as an RPC string message.
				let is_invalid_1010 = e.chain().any(|cause| {
					cause
						.downcast_ref::<subxt_error::TransactionError>()
						.and_then(|tx_err| match tx_err {
							subxt_error::TransactionError::Invalid(ev) => Some(ev),
							_ => None,
						})
						.is_some()
				}) || format!("{e:#}").contains("Invalid Transaction (1010)");

				if is_invalid_1010 {
					return Err(anyhow::anyhow!(
						"Failed to submit bitcoin tip due to invalid transaction. \
						This may be due to insufficient funds in the oracle account or a bad nonce. Throwing to kick off a restart. \
						Error: {e}"
					));
				}

				tracing::error!(
					?e,
					bitcoin_confirmed_height,
					?bitcoin_tip,
					"Failed to submit bitcoin tip",
				);
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use argon_client::{
		MainchainClient,
		api::{
			bitcoin_utxos::storage::types::confirmed_bitcoin_block_tip::ConfirmedBitcoinBlockTip,
			storage,
		},
		signer::Sr25519Signer,
	};
	use argon_primitives::bitcoin::BitcoinNetwork;
	use argon_testing::start_argon_test_node;
	use bitcoin::{Network, hashes::Hash};
	use polkadot_sdk::*;
	use sp_core::{Pair, sr25519};

	use super::*;

	#[tokio::test]
	async fn test_bitcoin_loop() {
		let _ = env_logger::builder().is_test(true).try_init();

		let signer_pair = sr25519::Pair::from_string("//Dave", None).unwrap();

		let signer = Sr25519Signer::new(signer_pair);
		let argon_node = start_argon_test_node().await;
		let bitcoind = argon_node.bitcoind.as_ref().expect("Bitcoind not started");
		let network: BitcoinNetwork = argon_node
			.client
			.fetch_storage(&storage().bitcoin_utxos().bitcoin_network(), FetchAt::Finalized)
			.await
			.unwrap()
			.expect("Expected network")
			.into();
		let network: Network = network.into();

		let address = bitcoind
			.client
			.get_new_address(None, None)
			.unwrap()
			.require_network(network)
			.unwrap();
		bitcoind.client.generate_to_address(5, &address).unwrap();

		let (rpc_url, auth) = argon_node.get_bitcoin_url();
		let auth = match auth {
			Auth::None => None,
			Auth::UserPass(u, p) => Some((u, p)),
			Auth::CookieFile(_) => None,
		};
		assert!(get_confirmed_block(&argon_node.client).await.is_none());
		let task = bitcoin_loop(rpc_url.to_string(), auth, argon_node.client.url.clone(), signer);
		let handle = tokio::spawn(task);

		let mut block_watch = argon_node.client.live.blocks().subscribe_best().await.unwrap();
		while let Some(Ok(_block)) = block_watch.next().await {
			if bitcoind.client.get_blockchain_info().unwrap().blocks == 10 {
				assert!(get_confirmed_block(&argon_node.client).await.is_some());
				break;
			}
			bitcoind.client.generate_to_address(1, &address).unwrap();
		}

		let block = get_confirmed_block(&argon_node.client).await.unwrap();
		assert!(block.block_height >= 1);
		assert_eq!(
			block.block_hash.0,
			*bitcoind
				.client
				.get_block_hash(block.block_height)
				.unwrap()
				.as_raw_hash()
				.as_byte_array()
		);

		handle.abort();
	}

	async fn get_confirmed_block(client: &MainchainClient) -> Option<ConfirmedBitcoinBlockTip> {
		client
			.fetch_storage(&storage().bitcoin_utxos().confirmed_bitcoin_block_tip(), FetchAt::Best)
			.await
			.expect("Expected tip")
	}
}
