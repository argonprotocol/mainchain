use crate::utils::{create_active_notary, register_miner_keys, register_miners};
use anyhow::anyhow;
use argon_bitcoin::{CosignScript, CosignScriptArgs};
use argon_client::{
	ArgonConfig, FetchAt, MainchainClient, api,
	api::{
		price_index::calls::types::submit::Index,
		runtime_types::sp_arithmetic::fixed_point::FixedU128 as FixedU128Ext, storage, tx,
	},
	signer::{Signer, Sr25519Signer},
};
use argon_primitives::{
	Balance, VaultId,
	argon_utils::format_argons,
	bitcoin::{BitcoinCosignScriptPubkey, BitcoinNetwork, SATOSHIS_PER_BITCOIN, Satoshis, UtxoId},
	tick::{Tick, Ticker},
};
use argon_testing::{
	ArgonTestNode, ArgonTestOracle, add_blocks, add_wallet_address, fund_script_address,
	run_bitcoin_cli, start_argon_test_node,
};
use bitcoin::{
	Amount, EcdsaSighashType, Network, Psbt, PublicKey, ScriptBuf, Txid,
	bip32::{ChildNumber, DerivationPath, Fingerprint, Xpub},
	hashes::Hash,
	secp256k1::{All, Secp256k1},
};
use bitcoind::{
	BitcoinD, anyhow,
	bitcoincore_rpc::{Auth, RpcApi, bitcoincore_rpc_json::AddressType, jsonrpc::serde_json},
};
use polkadot_sdk::*;
use serial_test::serial;
use sp_arithmetic::FixedU128;
use sp_core::{Pair, crypto::AccountId32, sr25519};
use sp_keyring::Sr25519Keyring::{Alice, Bob, Eve};
use std::{env, str::FromStr, sync::Arc, time::Duration};
use tokio::{fs, time::sleep};
use url::Url;

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_bitcoin_minting_e2e() {
	let test_node = start_argon_test_node().await;
	// need a test notary to get ownership rewards, so we can actually mint.
	let _test_notary = create_active_notary(&test_node).await.expect("Notary registered");
	let bitcoind = test_node.bitcoind.as_ref().expect("bitcoind");
	let block_creator = add_wallet_address(bitcoind);
	bitcoind.client.generate_to_address(101, &block_creator).unwrap();

	// 1. Owner creates a new pubkey and submits to blockchain
	let network: BitcoinNetwork = test_node
		.client
		.fetch_storage(&storage().bitcoin_utxos().bitcoin_network(), FetchAt::Finalized)
		.await
		.unwrap()
		.ok_or(anyhow!("No bitcoin network found"))
		.unwrap()
		.into();
	let network: Network = network.into();
	let secp = Secp256k1::new();

	let owner_address = bitcoind
		.client
		.get_new_address(Some("owner"), None)
		.unwrap()
		.require_network(network)
		.unwrap();
	println!("Owner address: {:#?}", bitcoind.client.get_address_info(&owner_address).unwrap());
	let owner_address_info = bitcoind.client.get_address_info(&owner_address).unwrap();
	let owner_compressed_pubkey = owner_address_info.pubkey.unwrap();
	let owner_hd_key_path = owner_address_info.hd_key_path.unwrap();

	assert!(owner_compressed_pubkey.compressed);
	assert_eq!(owner_compressed_pubkey.to_bytes().len(), 33);
	let owner_hd_fingerprint = get_parent_fingerprint(bitcoind, &owner_hd_key_path);

	let utxo_satoshis: Satoshis = Amount::ONE_BTC.to_sat() + 500;
	let utxo_btc = utxo_satoshis as f64 / SATOSHIS_PER_BITCOIN as f64;
	let alice_sr25519 = Alice.pair();
	let price_index_operator = Eve.pair();
	let bitcoin_owner_pair = Bob.pair();

	let client = test_node.client.clone();
	let client = Arc::new(client);

	let (vault_xpriv_path, vault_xpriv_pwd, vault_xpub, vault_xpub_hd_path) =
		create_xpriv_and_master_xpub(&test_node, "vault0").await.unwrap();
	let vault_signer = Sr25519Signer::new(alice_sr25519.clone());

	let _oracle = ArgonTestOracle::bitcoin_tip(&test_node).await.unwrap();

	add_blocks(bitcoind, 1, &block_creator);

	let vault_owner = alice_sr25519.clone();
	let vault_owner_account_id32: AccountId32 = vault_owner.public().into();

	// 2. A vault is setup with enough funds
	let vault_id = create_vault(&test_node, &vault_xpub, &vault_owner_account_id32, &vault_signer)
		.await
		.unwrap();

	let ticker = client.lookup_ticker().await.expect("ticker");
	let _last_bitcoin_price_tick = submit_price(&ticker, &client, &price_index_operator).await;

	let alice_signer = Sr25519Signer::new(alice_sr25519.clone());

	let _ = run_bitcoin_cli(&test_node, vec!["vault", "list", "--btc", &utxo_btc.to_string()])
		.await
		.unwrap();

	// 3. Owner calls api to request a bitcoin lock
	let utxo_id = request_bitcoin_lock(
		&test_node,
		vault_id,
		utxo_btc,
		&owner_compressed_pubkey,
		&bitcoin_owner_pair,
	)
	.await
	.unwrap();

	let send_to_address = run_bitcoin_cli(
		&test_node,
		vec!["lock", "send-to-address", "--utxo-id", &utxo_id.to_string()],
	)
	.await
	.unwrap();

	let (script_address, lock_amount) = confirm_lock(
		&test_node,
		&secp,
		owner_compressed_pubkey,
		utxo_satoshis,
		&client,
		&vault_xpub,
		network,
		vault_id,
		&utxo_id,
	)
	.await
	.unwrap();

	// 4. Owner funds the LockedBitcoin utxo and submits it
	let scriptbuf: ScriptBuf = script_address.into();
	let scriptaddress = bitcoin::Address::from_script(scriptbuf.as_script(), network).unwrap();
	println!("Checking for {} satoshis to {}", utxo_satoshis, scriptaddress);
	assert!(send_to_address.contains(&format!("{} satoshis to {}", utxo_satoshis, scriptaddress)));

	let (txid, vout, _) =
		fund_script_address(bitcoind, &scriptaddress, utxo_satoshis, &block_creator);

	add_blocks(bitcoind, 5, &block_creator);
	let vote_miner = Eve;
	let vote_miner_account = vote_miner.to_account_id();
	let miner_node = test_node.fork_node("eve", 0).await.unwrap();
	// wait for miner_node to catch up

	let finalized =
		test_node.client.latest_finalized_block_hash().await.expect("should get latest");
	let block_number = test_node
		.client
		.block_number(finalized.hash())
		.await
		.expect("should get number");
	let mut miner_node_catchup_sub =
		miner_node.client.live.blocks().subscribe_finalized().await.unwrap();
	while let Some(next) = miner_node_catchup_sub.next().await {
		let next = next.unwrap();
		println!(
			"Got next finalized catching up to main node {:?}. Waiting for {}",
			next.header().number,
			block_number
		);
		if next.hash().as_ref() == finalized.hash().as_ref() || next.number() >= block_number {
			break;
		}
	}
	let nonce_miner_node = miner_node.client.get_account_nonce(&vote_miner_account).await.unwrap();
	let nonce_test_node = client.get_account_nonce(&vote_miner_account).await.unwrap();
	assert_eq!(nonce_miner_node, nonce_test_node);
	println!("System health of miner node {:?}", miner_node.client.methods.system_health().await);
	let keys = register_miner_keys(&miner_node, vote_miner, 1)
		.await
		.expect("Couldn't register vote miner");

	// Register the miner against the test node since we are having fork issues
	register_miners(&test_node, alice_signer, vec![(vote_miner_account.clone(), keys)])
		.await
		.unwrap();

	wait_for_mint(&bitcoin_owner_pair, &client, &utxo_id, lock_amount, txid, vout)
		.await
		.unwrap();

	let _ = run_bitcoin_cli(&test_node, vec!["lock", "status", "--utxo-id", &utxo_id.to_string()])
		.await
		.unwrap();

	println!("Submitting new bitcoin price");
	submit_price(&ticker, &client, &price_index_operator).await;

	// 5. Ask for the bitcoin to be released
	println!("\nOwner requests release");
	owner_requests_release(
		&test_node,
		bitcoind,
		network,
		&bitcoin_owner_pair,
		&client,
		vault_id,
		utxo_id,
	)
	.await
	.unwrap();

	// 5. vault sees release request (outaddress, fee) and creates a transaction
	println!("\nVault publishes cosign tx");
	vault_cosigns_release(
		&test_node,
		client,
		&vault_signer,
		&vault_id,
		&utxo_id,
		&vault_xpriv_path,
		&vault_xpriv_pwd,
		&vault_xpub_hd_path,
	)
	.await
	.unwrap();

	println!("\nOwner sees the transaction and cosigns");
	// 6. Owner sees the transaction and can submit it
	owner_sees_signature_and_releases(
		&test_node,
		bitcoind,
		&utxo_id,
		&owner_hd_key_path.to_string(),
		&owner_hd_fingerprint.to_string(),
	)
	.await
	.unwrap();
	drop(test_node);
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_bitcoin_xpriv_lock_e2e() {
	let test_node = start_argon_test_node().await;
	let bitcoind = test_node.bitcoind.as_ref().expect("bitcoind");
	let (bitcoin_url, auth) = test_node.get_bitcoin_url();
	let bitcoind_client = bitcoincore_rpc::Client::new(&bitcoin_url, auth).unwrap();
	let block_creator = add_wallet_address(bitcoind);
	bitcoind.client.generate_to_address(101, &block_creator).unwrap();

	// 1. Owner creates a new pubkey and submits to blockchain
	let network: BitcoinNetwork = test_node
		.client
		.fetch_storage(&storage().bitcoin_utxos().bitcoin_network(), FetchAt::Finalized)
		.await
		.unwrap()
		.ok_or(anyhow!("No bitcoin network found"))
		.unwrap()
		.into();
	let network: Network = network.into();
	let secp = Secp256k1::new();

	let (owner_xpriv_path, owner_xpriv_pwd, owner_compressed_pubkey, owner_hd_path) =
		create_xpriv_and_derive(&test_node).await.unwrap();

	let utxo_satoshis: Satoshis = Amount::ONE_BTC.to_sat() + 500;
	let utxo_btc = utxo_satoshis as f64 / SATOSHIS_PER_BITCOIN as f64;
	let alice_sr25519 = Alice.pair();
	let price_index_operator = Eve.pair();
	let bitcoin_owner_pair = Bob.pair();

	let client = test_node.client.clone();
	let client = Arc::new(client);

	let (vault_xpriv_path, vault_xpriv_pwd, vault_xpub, vault_xpub_hd_path) =
		create_xpriv_and_master_xpub(&test_node, "vault1").await.unwrap();
	let vault_signer = Sr25519Signer::new(alice_sr25519.clone());

	let _oracle = ArgonTestOracle::bitcoin_tip(&test_node).await.unwrap();

	add_blocks(bitcoind, 1, &block_creator);

	let vault_owner = alice_sr25519.clone();
	let vault_owner_account_id32: AccountId32 = vault_owner.public().into();

	// 2. A vault is setup with enough funds
	let vault_id = create_vault(&test_node, &vault_xpub, &vault_owner_account_id32, &vault_signer)
		.await
		.unwrap();

	let ticker = client.lookup_ticker().await.expect("ticker");
	let _last_bitcoin_price_tick = submit_price(&ticker, &client, &price_index_operator).await;

	let _ = run_bitcoin_cli(&test_node, vec!["vault", "list", "--btc", &utxo_btc.to_string()])
		.await
		.unwrap();

	// 3. Owner calls api to request a bitcoin lock
	let utxo_id = request_bitcoin_lock(
		&test_node,
		vault_id,
		utxo_btc,
		&owner_compressed_pubkey,
		&bitcoin_owner_pair,
	)
	.await
	.unwrap();

	let send_to_address = run_bitcoin_cli(
		&test_node,
		vec!["lock", "send-to-address", "--utxo-id", &utxo_id.to_string()],
	)
	.await
	.unwrap();

	let (script_address, _lock_amount) = confirm_lock(
		&test_node,
		&secp,
		owner_compressed_pubkey,
		utxo_satoshis,
		&client,
		&vault_xpub,
		network,
		vault_id,
		&utxo_id,
	)
	.await
	.unwrap();

	// 4. Owner funds the LockedBitcoin utxo and submits it
	let scriptbuf: ScriptBuf = script_address.into();
	let scriptaddress = bitcoin::Address::from_script(scriptbuf.as_script(), network).unwrap();
	println!("Checking for {} satoshis to {}", utxo_satoshis, scriptaddress);
	assert!(send_to_address.contains(&format!("{} satoshis to {}", utxo_satoshis, scriptaddress)));

	let _ = fund_script_address(bitcoind, &scriptaddress, utxo_satoshis, &block_creator);

	add_blocks(bitcoind, 5, &block_creator);
	let mut max_blocks = 100;
	let mut block_sub = test_node.client.live.blocks().subscribe_finalized().await.unwrap();
	while let Some(next) = block_sub.next().await {
		let utxo_lock = test_node
			.client
			.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(utxo_id), FetchAt::Finalized)
			.await
			.unwrap()
			.expect("Utxo state should be present");
		if utxo_lock.is_verified {
			println!("Utxo lock is verified in block {:?}", next.unwrap().hash());
			break;
		}
		println!("Waiting for utxo lock to be verified in block {:?}", next.unwrap().hash());
		max_blocks -= 1;
		if max_blocks == 0 {
			panic!("No utxo lock verified after 100 blocks");
		}
	}

	println!("Submitting new bitcoin price");
	submit_price(&ticker, &client, &price_index_operator).await;

	// 5. Ask for the bitcoin to be releaseed
	println!("\nOwner requests release");
	owner_requests_release(
		&test_node,
		bitcoind,
		network,
		&bitcoin_owner_pair,
		&client,
		vault_id,
		utxo_id,
	)
	.await
	.unwrap();

	// 5. vault sees release request (outaddress, fee) and creates a transaction
	println!("\nVault publishes cosign tx");
	vault_cosigns_release(
		&test_node,
		client,
		&vault_signer,
		&vault_id,
		&utxo_id,
		&vault_xpriv_path,
		&vault_xpriv_pwd,
		&vault_xpub_hd_path,
	)
	.await
	.unwrap();

	println!("\nOwner sees the transaction and cosigns");
	let (bitcoin_url, auth) = test_node.get_bitcoin_url();
	let mut bitcoin_url = Url::parse(&bitcoin_url).unwrap();
	if let Auth::UserPass(user, pass) = auth {
		bitcoin_url.set_username(&user).unwrap();
		bitcoin_url.set_password(Some(&pass)).unwrap();
	}
	println!("Owner pubkey is {}", owner_compressed_pubkey);
	tokio::spawn(async move {
		loop {
			bitcoind_client.generate_to_address(1, &block_creator).unwrap();
			println!("Bitcoin block generated");
			sleep(Duration::from_secs(5)).await;
		}
	});
	// 6. Owner sees the transaction and can submit it
	let owner_cosign_cli = run_bitcoin_cli(
		&test_node,
		vec![
			"lock",
			"owner-cosign-release",
			"--utxo-id",
			&utxo_id.to_string(),
			"--hd-path",
			&owner_hd_path,
			"--xpriv-path",
			&owner_xpriv_path,
			"--xpriv-password",
			&owner_xpriv_pwd,
			"--bitcoin-rpc-url",
			bitcoin_url.as_ref(),
			"--wait",
		],
	)
	.await
	.unwrap();
	assert!(
		owner_cosign_cli.contains("confirmations: 6"),
		"Got 6 confirmations for the transaction"
	);
	drop(test_node);
}

async fn submit_price(
	ticker: &Ticker,
	client: &MainchainClient,
	price_index_operator: &sr25519::Pair,
) -> Tick {
	let tick = ticker.current();
	client
		.live
		.tx()
		.sign_and_submit_then_watch_default(
			&tx().price_index().submit(Index {
				btc_usd_price: FixedU128Ext(FixedU128::from_float(62_000.0).into_inner()),
				argon_usd_target_price: FixedU128Ext(FixedU128::from_float(1.0).into_inner()),
				argon_usd_price: FixedU128Ext(FixedU128::from_float(1.6).into_inner()),
				argon_time_weighted_average_liquidity: 500_000_000_000,
				argonot_usd_price: FixedU128Ext(FixedU128::from_float(1.0).into_inner()),
				tick,
			}),
			&Sr25519Signer::new(price_index_operator.clone()),
		)
		.await
		.unwrap()
		.wait_for_finalized_success()
		.await
		.unwrap();
	println!("bitcoin prices submitted at tick {tick}",);
	tick
}

fn get_parent_fingerprint(bitcoind: &BitcoinD, owner_hd_key_path: &DerivationPath) -> Fingerprint {
	let parent_hd_key_path = owner_hd_key_path.to_string();
	let mut parent_hd_key_path = parent_hd_key_path.split('/').collect::<Vec<_>>();
	parent_hd_key_path.pop();
	let parent_part = parent_hd_key_path.pop().unwrap();
	let is_internal_hd = parent_part.ends_with('1');
	let hardened_parent_hd_key_path = parent_hd_key_path.join("/").replace('\'', "h");
	println!("Hardened Parent HD Key Path: {}", hardened_parent_hd_key_path);

	let descriptors = bitcoind.client.call::<serde_json::Value>("listdescriptors", &[]).unwrap();
	println!("Descriptors: {:#?}", descriptors);
	// Step 5: Find the hardened parent xpub in the descriptors
	let master_fingerprint = descriptors["descriptors"]
		.as_array()
		.expect("Invalid descriptors format")
		.iter()
		.find_map(|desc| {
			let desc_str = desc["desc"].as_str().unwrap();
			let is_internal = desc["internal"].as_bool().unwrap();
			if desc_str.contains(&hardened_parent_hd_key_path) && is_internal == is_internal_hd {
				let bracketed = desc_str.split('[').next_back().unwrap();
				let xpub = bracketed.split(']').next().unwrap();
				let fingerprint = xpub.split('/').next().unwrap();
				Some(fingerprint)
			} else {
				None
			}
		})
		.expect("Parent xpub not found in descriptors");
	let master_fingerprint = Fingerprint::from_hex(master_fingerprint).unwrap();
	println!("Master Fingerprint: {}", master_fingerprint);
	master_fingerprint
}

async fn create_xpriv_and_derive(
	test_node: &ArgonTestNode,
) -> anyhow::Result<(String, String, bitcoin::PublicKey, String)> {
	let path = env::temp_dir().join("owner0.xpriv");
	if path.is_file() {
		fs::remove_file(&path).await?;
	}
	let password = "Password123";
	let _ = run_bitcoin_cli(
		test_node,
		vec![
			"xpriv",
			"master",
			"--xpriv-password",
			password,
			"--xpriv-path",
			path.to_str().unwrap(),
			"--bitcoin-network",
			"regtest",
		],
	)
	.await?;
	let derivation_path = "m/84'/0'/0'";

	let result = run_bitcoin_cli(
		test_node,
		vec![
			"xpriv",
			"derive-pubkey",
			"--xpriv-path",
			path.to_str().unwrap(),
			"--xpriv-password",
			password,
			"--hd-path",
			derivation_path,
		],
	)
	.await?;

	println!("Result from derive-pubkey: {}", result.trim());

	let pubkey = PublicKey::from_str(result.trim())
		.map_err(|e| anyhow!("Failed to parse public key: {}", e))?;

	Ok((
		path.to_str().unwrap().to_string(),
		password.to_string(),
		pubkey,
		derivation_path.to_string(),
	))
}

async fn create_xpriv_and_master_xpub(
	test_node: &ArgonTestNode,
	name: &str,
) -> anyhow::Result<(String, String, Xpub, String)> {
	let path = env::temp_dir().join(format!("{}.xpriv", name));
	if path.is_file() {
		fs::remove_file(&path).await?;
	}
	let password = "Password123";
	let _ = run_bitcoin_cli(
		test_node,
		vec![
			"xpriv",
			"master",
			"--xpriv-password",
			password,
			"--xpriv-path",
			path.to_str().unwrap(),
			"--bitcoin-network",
			"regtest",
		],
	)
	.await?;
	let derivation_path = "m/0'";

	let xpub_result = run_bitcoin_cli(
		test_node,
		vec![
			"xpriv",
			"derive-xpub",
			"--xpriv-path",
			path.to_str().unwrap(),
			"--xpriv-password",
			password,
			"--hd-path",
			derivation_path,
		],
	)
	.await?;

	let xpub =
		Xpub::from_str(xpub_result.trim()).map_err(|e| anyhow!("Failed to parse xpub: {}", e))?;

	Ok((
		path.to_str().unwrap().to_string(),
		password.to_string(),
		xpub,
		derivation_path.to_string(),
	))
}

async fn create_vault(
	test_node: &ArgonTestNode,
	xpubkey: &Xpub,
	vault_owner_account_id32: &AccountId32,
	vault_signer: &impl Signer<ArgonConfig>,
) -> anyhow::Result<VaultId> {
	let client = test_node.client.clone();
	// wait for alice to have enough argons
	let mut finalized_sub = client.live.blocks().subscribe_finalized().await?;
	let vault_account = client.api_account(vault_owner_account_id32);
	let lookup = storage().system().account(vault_account);
	while let Some(block) = finalized_sub.next().await {
		println!("Waiting for Alice to have enough argons");
		if let Some(alice_balance) =
			client.fetch_storage(&lookup, FetchAt::Block(block.unwrap().hash())).await?
		{
			println!("Alice argon balance {:#?}", alice_balance.data.free);
			if alice_balance.data.free > 100_001_000_000 {
				println!("Alice can start a vault now!");
				break;
			}
		}
	}

	println!("creating a vault");
	let params = client.params_with_best_nonce(&vault_owner_account_id32.clone()).await?;

	let result = run_bitcoin_cli(
		test_node,
		vec![
			"vault",
			"create",
			"--argons",
			"100,000",
			"--securitization-ratio",
			"1x",
			"--bitcoin-xpub",
			&xpubkey.to_string(),
			"--bitcoin-apr",
			"1.0",
			"--bitcoin-base-fee",
			"0",
			"--liquidity-pool-profit-sharing",
			"50",
		],
	)
	.await?;

	let vault_creation = client
		.submit_from_polkadot_url(&result, vault_signer, Some(params.build()))
		.await?
		.wait_for_finalized_success()
		.await?
		.find_first::<api::vaults::events::VaultCreated>()?
		.expect("vault created");
	println!("vault created {:?}", vault_creation);
	assert_eq!(vault_creation.vault_id, 1);

	Ok(vault_creation.vault_id)
}

async fn request_bitcoin_lock(
	test_node: &ArgonTestNode,
	vault_id: VaultId,
	utxo_btc: f64,
	owner_compressed_pubkey: &bitcoin::PublicKey,
	bitcoin_owner: &sr25519::Pair,
) -> anyhow::Result<UtxoId> {
	// wait for the vault to be open

	loop {
		println!("Waiting for vault to be open");
		let tick = test_node
			.client
			.fetch_storage(&storage().ticks().current_tick(), FetchAt::Best)
			.await?
			.ok_or(anyhow!("No tick found"))?;
		let vault = test_node
			.client
			.fetch_storage(&storage().vaults().vaults_by_id(vault_id), FetchAt::Best)
			.await?
			.ok_or(anyhow!("No vault found"))?;
		if vault.opened_tick <= tick {
			println!("Vault is open");
			break;
		}
		// wait for 1 second
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	}

	let lock_cli_result = run_bitcoin_cli(
		test_node,
		vec![
			"lock",
			"initialize",
			"--vault-id",
			&vault_id.to_string(),
			"--btc",
			&utxo_btc.to_string(),
			"--owner-pubkey",
			&owner_compressed_pubkey.to_string(),
		],
	)
	.await?;

	println!("Owner locked bitcoin with pubkey: {}", owner_compressed_pubkey);

	let lock_tx = test_node
		.client
		.submit_from_polkadot_url(
			&lock_cli_result,
			&Sr25519Signer::new(bitcoin_owner.clone()),
			None,
		)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("bitcoin lock submitted (owner watches for utxo id)");
	let lock_event = lock_tx
		.find_first::<api::bitcoin_locks::events::BitcoinLockCreated>()?
		.expect("lock event");
	let utxo_id = lock_event.utxo_id;
	Ok(utxo_id)
}

#[allow(clippy::too_many_arguments)]
async fn confirm_lock(
	test_node: &ArgonTestNode,
	secp: &Secp256k1<All>,
	owner_compressed_pubkey: PublicKey,
	utxo_satoshis: Satoshis,
	client: &Arc<MainchainClient>,
	xpubkey: &Xpub,
	bitcoin_network: Network,
	vault_id: VaultId,
	utxo_id: &UtxoId,
) -> anyhow::Result<(BitcoinCosignScriptPubkey, Balance)> {
	let lock_cli_get =
		run_bitcoin_cli(test_node, vec!["lock", "status", "--utxo-id", &utxo_id.to_string()])
			.await?;

	let lock_api = client
		.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(*utxo_id), FetchAt::Finalized)
		.await?
		.expect("should be able to retrieve");
	assert_eq!(lock_api.vault_id, vault_id);
	{
		assert_eq!(lock_api.satoshis, utxo_satoshis);
		assert_eq!(lock_api.owner_pubkey.0, owner_compressed_pubkey.inner.serialize());
		assert_eq!(lock_api.vault_xpub_sources.0, xpubkey.fingerprint().to_bytes());
		assert_eq!(
			lock_api.vault_xpub_sources.1,
			Into::<u32>::into(ChildNumber::from_normal_idx(1)?)
		);
		assert_eq!(
			lock_api.vault_pubkey.0,
			xpubkey
				.derive_pub(secp, &DerivationPath::from_str("1")?)?
				.public_key
				.serialize()
		);
		let cosign_script = CosignScript::new(
			CosignScriptArgs {
				vault_pubkey: lock_api.vault_pubkey.clone().into(),
				owner_pubkey: lock_api.owner_pubkey.into(),
				vault_claim_pubkey: lock_api.vault_claim_pubkey.into(),
				vault_claim_height: lock_api.vault_claim_height,
				open_claim_height: lock_api.open_claim_height,
				created_at_height: lock_api.created_at_height,
			},
			bitcoin_network,
		)
		.map_err(|_| anyhow!("Unable to create a script"))?;
		let cosign_key = cosign_script.script.to_p2wsh();
		let cosign_script_pubkey: BitcoinCosignScriptPubkey =
			cosign_key.try_into().map_err(|_| anyhow!("Unable to convert script pubkey"))?;
		assert_eq!(cosign_script_pubkey, lock_api.utxo_script_pubkey.clone().into());
	}

	assert!(
		lock_cli_get
			.lines()
			.find(|line| line.contains("Minted Argons"))
			.unwrap()
			.contains(&format!("₳0 of {}", format_argons(lock_api.liquidity_value)))
	);
	let liquidity_value = lock_api.liquidity_value;
	Ok((lock_api.utxo_script_pubkey.into(), liquidity_value))
}

async fn wait_for_mint(
	bitcoin_owner: &sr25519::Pair,
	client: &Arc<MainchainClient>,
	utxo_id: &UtxoId,
	liquidity_value: Balance,
	txid: Txid,
	vout: u32,
) -> anyhow::Result<()> {
	let mut finalized_sub = client.live.blocks().subscribe_finalized().await?;
	let pending_utxos = client
		.fetch_storage(&storage().bitcoin_utxos().utxos_pending_confirmation(), FetchAt::Finalized)
		.await?
		.unwrap()
		.0;
	if !pending_utxos.is_empty() {
		println!("Waiting for pending utxo to be verified {:?}", pending_utxos);
		let mut counter = 0;
		while let Some(block) = finalized_sub.next().await {
			let block = block?;
			let utxo_verified =
				block.events().await?.find_first::<api::bitcoin_utxos::events::UtxoVerified>()?;
			if let Some(utxo_verified) = utxo_verified {
				if utxo_verified.utxo_id == 1 {
					println!("Utxo verified in Argon mainchain");
					break;
				}
			}
			counter += 1;
			if counter >= 20 {
				panic!("No mint after 100 blocks")
			}
		}
	}
	// load 2 more blocks
	for _ in 0..2 {
		let _ = finalized_sub.next().await;
	}
	let utxo_ref = client
		.fetch_storage(&storage().bitcoin_utxos().utxo_id_to_ref(*utxo_id), FetchAt::Finalized)
		.await?
		.expect("utxo");
	assert_eq!(utxo_ref.txid.0, txid.to_byte_array());
	assert_eq!(utxo_ref.output_index, vout);

	let pending_mint = client
		.fetch_storage(&storage().mint().pending_mint_utxos(), FetchAt::Finalized)
		.await?
		.expect("pending mint");

	let owner_account_id32: AccountId32 = bitcoin_owner.clone().public().into();
	let balance = client.get_argons(&owner_account_id32).await.expect("pending mint balance");
	if pending_mint.0.is_empty() {
		assert!(balance.free >= liquidity_value);
	} else {
		assert_eq!(pending_mint.0.len(), 1);
		assert_eq!(pending_mint.0[0].1, owner_account_id32.into());
		// should have minted some amount
		assert!(pending_mint.0[0].2 < liquidity_value);
		println!(
			"Owner mint pending remaining = {} (balance={})",
			pending_mint.0[0].2, balance.free
		);
		assert!(balance.free > (liquidity_value - pending_mint.0[0].2));

		// 4. Wait for the full payout
		let mut counter = 0;
		while let Some(_block) = finalized_sub.next().await {
			let pending_mint = client
				.fetch_storage(&storage().mint().pending_mint_utxos(), FetchAt::Finalized)
				.await?
				.expect("pending mint");
			println!("Pending mint {:?}", pending_mint.0.first().map(|a| a.2));
			if pending_mint.0.is_empty() {
				break;
			}
			counter += 1;
			if counter >= 30 {
				panic!("Timed out waiting for remaining mint. Last mint was {:?}", pending_mint.0);
			}
		}
		println!("Owner minted full bitcoin")
	}
	Ok(())
}

async fn owner_requests_release(
	test_node: &ArgonTestNode,
	bitcoind: &BitcoinD,
	network: Network,
	bitcoin_owner: &sr25519::Pair,
	client: &Arc<MainchainClient>,
	vault_id: VaultId,
	utxo_id: UtxoId,
) -> anyhow::Result<()> {
	let out_script_pubkey = bitcoind
		.client
		.get_new_address(Some("takeback"), Some(AddressType::Bech32m))
		.unwrap()
		.require_network(network)?;
	let release_cli = run_bitcoin_cli(
		test_node,
		vec![
			"lock",
			"request-release",
			"--utxo-id",
			&utxo_id.to_string(),
			"--dest-pubkey",
			&out_script_pubkey.to_string(),
		],
	)
	.await?;

	let release_tx = client
		.submit_from_polkadot_url(&release_cli, &Sr25519Signer::new(bitcoin_owner.clone()), None)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("bitcoin release request finalized");
	// this is the event that a vault would also monitor
	let release_event = release_tx
		.find_first::<api::bitcoin_locks::events::BitcoinUtxoCosignRequested>()?
		.expect("release event");
	assert_eq!(release_event.utxo_id, utxo_id);
	assert_eq!(release_event.vault_id, vault_id);

	Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn vault_cosigns_release(
	test_node: &ArgonTestNode,
	client: Arc<MainchainClient>,
	vault_signer: &Sr25519Signer,
	vault_id: &VaultId,
	utxo_id: &UtxoId,
	xpriv_path: &str,
	xpriv_pwd: &str,
	uploaded_xpub_hd_path: &str,
) -> anyhow::Result<()> {
	let pending_release = run_bitcoin_cli(
		test_node,
		vec!["vault", "pending-release", "--vault-id", &vault_id.to_string()],
	)
	.await?;

	assert!(pending_release.lines().count() > 3);
	assert!(pending_release.contains('1'));

	let release_fulfill_cli = run_bitcoin_cli(
		test_node,
		vec![
			"lock",
			"vault-cosign-release",
			"--utxo-id",
			&utxo_id.to_string(),
			"--xpriv-path",
			xpriv_path,
			"--xpriv-password",
			xpriv_pwd,
			"--hd-path",
			uploaded_xpub_hd_path,
		],
	)
	.await?;

	let _ = client
		.submit_from_polkadot_url(&release_fulfill_cli, vault_signer, None)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("bitcoin cosign submitted");
	Ok(())
}

async fn owner_sees_signature_and_releases(
	test_node: &ArgonTestNode,
	bitcoind: &BitcoinD,
	utxo_id: &UtxoId,
	hd_path: &str,
	fingerprint: &str,
) -> anyhow::Result<()> {
	let owner_cosign_cli = run_bitcoin_cli(
		test_node,
		vec![
			"lock",
			"owner-cosign-release",
			"--utxo-id",
			&utxo_id.to_string(),
			"--hd-path",
			hd_path,
			"--parent-fingerprint",
			fingerprint,
			"--wait",
		],
	)
	.await?;

	let psbt_text = owner_cosign_cli
		.trim()
		.split('\n')
		.next_back()
		.ok_or(anyhow!("No psbt in text found"))?
		.trim();

	println!("Processing with wallet");
	{
		let psbt = Psbt::from_str(psbt_text).expect("psbt");
		println!("PSBT from cli: {:#?}", psbt);
		let analyzed = bitcoind
			.client
			.call::<serde_json::Value>("analyzepsbt", &[serde_json::to_value(psbt_text).unwrap()])
			.unwrap();
		println!("Analyzed Psbt: {:#?}", analyzed);
	}
	let import = bitcoind.client.wallet_process_psbt(
		psbt_text,
		Some(true),
		Some(EcdsaSighashType::AllPlusAnyoneCanPay.into()),
		None,
	)?;
	println!("Processed with wallet {:?}", import);
	{
		let psbt = Psbt::from_str(import.psbt.as_str()).expect("psbt");
		println!("PSBT after import: {:#?}", psbt);
		let analyzed = bitcoind
			.client
			.call::<serde_json::Value>("analyzepsbt", &[serde_json::to_value(psbt_text).unwrap()])
			.unwrap();
		println!("Analyzed Psbt: {:#?}", analyzed);
	}

	let finalized = bitcoind.client.finalize_psbt(&import.psbt, None)?;
	println!("Finalized psbt! {:?}", finalized);
	let acceptance = bitcoind
		.client
		.test_mempool_accept(&[&finalized.hex.unwrap()])
		.expect("checked");
	let did_accept = acceptance.first().unwrap();
	assert!(did_accept.allowed);

	Ok(())
}
