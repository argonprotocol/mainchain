use crate::utils::{create_active_notary, register_miner_keys, register_miners, sudo};
use anyhow::anyhow;
use argon_bitcoin::{
	derive_pubkey, derive_xpub, xpriv_from_seed, CosignReleaser, CosignScript, CosignScriptArgs,
};
use argon_client::{
	api,
	api::{
		price_index::calls::types::submit::Index,
		runtime_types::{
			argon_runtime::RuntimeCall, sp_arithmetic::fixed_point::FixedU128 as FixedU128Ext,
		},
		storage, tx,
	},
	conversion::{to_api_fixed_u128, to_api_per_mill},
	signer::{Signer, Sr25519Signer},
	ArgonConfig, FetchAt, MainchainClient,
};
use argon_primitives::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinNetwork, BitcoinScriptPubkey, BitcoinSignature,
		CompressedBitcoinPubkey, H256Le, Satoshis, UtxoId,
	},
	prelude::sp_core::Encode,
	tick::{Tick, Ticker},
	Balance, VaultId,
};
use argon_testing::{
	add_blocks, add_wallet_address, fund_script_address, start_argon_test_node, ArgonTestNode,
	ArgonTestOracle,
};
use base64::{engine::general_purpose, Engine as _};
use bitcoin::{
	bip32::{ChildNumber, DerivationPath, Fingerprint, Xpub},
	hashes::Hash,
	secp256k1::{All, Secp256k1},
	Amount, EcdsaSighashType, Network, Psbt, PublicKey, ScriptBuf, Txid,
};
use bitcoind::{
	anyhow,
	bitcoincore_rpc::{bitcoincore_rpc_json::AddressType, jsonrpc::serde_json, Auth, RpcApi},
	BitcoinD,
};
use polkadot_sdk::*;
use serial_test::serial;
use sp_arithmetic::FixedU128;
use sp_core::{crypto::AccountId32, sr25519, Pair};
use sp_keyring::Sr25519Keyring::{Alice, Bob, Eve};
use sp_runtime::Permill;
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::time::sleep;
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
	let alice_sr25519 = Alice.pair();
	let price_index_operator = Eve.pair();
	let bitcoin_owner_pair = Bob.pair();

	let client = test_node.client.clone();
	let client = Arc::new(client);

	let (vault_xpriv, vault_xpub, vault_xpub_hd_path) = create_xpriv_and_master_xpub().unwrap();
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
	let mut last_bitcoin_price_tick = submit_price(&ticker, &client, &price_index_operator).await;

	let alice_signer = Sr25519Signer::new(alice_sr25519.clone());

	// 3. Owner calls api to request a bitcoin lock
	let utxo_id = request_bitcoin_lock(
		&test_node,
		vault_id,
		utxo_satoshis,
		&owner_compressed_pubkey,
		&bitcoin_owner_pair,
	)
	.await
	.unwrap();

	let (script_address, lock_amount) = confirm_lock(
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
	println!("Checking for {utxo_satoshis} satoshis to {scriptaddress}");

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
	register_miners(&test_node, alice_signer, vec![(vote_miner_account.clone(), keys)], None)
		.await
		.unwrap();

	// increase the mining mint so we can match it without waiting
	sudo(
		&test_node,
		RuntimeCall::System(
			argon_client::api::runtime_types::frame_system::pallet::Call::set_storage {
				items: vec![(
					storage().mint().minted_mining_microgons().to_root_bytes(),
					Balance::from(100_000_000_000u64).encode(),
				)],
			},
		),
		false,
	)
	.await
	.unwrap();

	wait_for_mint(
		&bitcoin_owner_pair,
		&client,
		&utxo_id,
		lock_amount,
		txid,
		vout,
		&ticker,
		&price_index_operator,
		&mut last_bitcoin_price_tick,
	)
	.await
	.unwrap();

	println!("Submitting new bitcoin price");
	submit_price_if_needed(&ticker, &client, &price_index_operator, &mut last_bitcoin_price_tick)
		.await;

	// 5. Ask for the bitcoin to be released
	println!("\nOwner requests release");
	owner_requests_release(bitcoind, network, &bitcoin_owner_pair, &client, vault_id, utxo_id)
		.await
		.unwrap();

	// 5. vault sees release request (outaddress, fee) and creates a transaction
	println!("\nVault publishes cosign tx");
	vault_cosigns_release(
		client.as_ref(),
		&vault_signer,
		&vault_id,
		&utxo_id,
		&vault_xpriv,
		&vault_xpub_hd_path,
	)
	.await
	.unwrap();

	println!("\nOwner sees the transaction and cosigns");
	// 6. Owner sees the transaction and can submit it
	owner_sees_signature_and_releases(
		client.as_ref(),
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

	let (owner_xpriv, owner_compressed_pubkey, owner_hd_path) = create_xpriv_and_derive().unwrap();

	let utxo_satoshis: Satoshis = Amount::ONE_BTC.to_sat() + 500;
	let alice_sr25519 = Alice.pair();
	let price_index_operator = Eve.pair();
	let bitcoin_owner_pair = Bob.pair();

	let client = test_node.client.clone();
	let client = Arc::new(client);

	let (vault_xpriv, vault_xpub, vault_xpub_hd_path) = create_xpriv_and_master_xpub().unwrap();
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
	let mut last_bitcoin_price_tick = submit_price(&ticker, &client, &price_index_operator).await;

	// 3. Owner calls api to request a bitcoin lock
	let utxo_id = request_bitcoin_lock(
		&test_node,
		vault_id,
		utxo_satoshis,
		&owner_compressed_pubkey,
		&bitcoin_owner_pair,
	)
	.await
	.unwrap();

	let (script_address, _lock_amount) = confirm_lock(
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
	println!("Checking for {utxo_satoshis} satoshis to {scriptaddress}");

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
		if utxo_lock.is_funded {
			println!("Utxo lock is funded in block {:?}", next.unwrap().hash());
			break;
		}
		println!("Waiting for utxo lock to be funded in block {:?}", next.unwrap().hash());
		max_blocks -= 1;
		if max_blocks == 0 {
			panic!("No utxo lock funded after 100 blocks");
		}
	}

	println!("Submitting new bitcoin price");
	submit_price_if_needed(&ticker, &client, &price_index_operator, &mut last_bitcoin_price_tick)
		.await;

	// 5. Ask for the bitcoin to be releaseed
	println!("\nOwner requests release");
	owner_requests_release(bitcoind, network, &bitcoin_owner_pair, &client, vault_id, utxo_id)
		.await
		.unwrap();

	// 5. vault sees release request (outaddress, fee) and creates a transaction
	println!("\nVault publishes cosign tx");
	vault_cosigns_release(
		client.as_ref(),
		&vault_signer,
		&vault_id,
		&utxo_id,
		&vault_xpriv,
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
	println!("Owner pubkey is {owner_compressed_pubkey}");
	tokio::spawn(async move {
		loop {
			bitcoind_client.generate_to_address(1, &block_creator).unwrap();
			println!("Bitcoin block generated");
			sleep(Duration::from_secs(5)).await;
		}
	});
	// 6. Owner sees the transaction and can submit it
	owner_signs_and_releases(
		client.as_ref(),
		&utxo_id,
		&owner_xpriv,
		&owner_hd_path,
		bitcoin_url.as_ref(),
	)
	.await
	.unwrap();
	drop(test_node);
}

async fn submit_price(
	ticker: &Ticker,
	client: &MainchainClient,
	price_index_operator: &sr25519::Pair,
) -> Tick {
	let signer = Sr25519Signer::new(price_index_operator.clone());
	let account_id = signer.account_id();
	let tick = current_chain_tick(client, ticker).await;
	let nonce = client.get_account_nonce(&account_id).await.unwrap();
	let params = MainchainClient::ext_params_builder().nonce(nonce.into()).build();
	let progress = client
		.live
		.tx()
		.sign_and_submit_then_watch(
			&tx().price_index().submit(Index {
				btc_usd_price: FixedU128Ext(FixedU128::from_float(62_000.0).into_inner()),
				argon_usd_target_price: FixedU128Ext(FixedU128::from_float(1.0).into_inner()),
				argon_usd_price: FixedU128Ext(FixedU128::from_float(1.6).into_inner()),
				argon_time_weighted_average_liquidity: 500_000_000_000,
				argonot_usd_price: FixedU128Ext(FixedU128::from_float(1.0).into_inner()),
				tick,
			}),
			&signer,
			params,
		)
		.await
		.unwrap();
	MainchainClient::wait_for_ext_in_block(progress, true).await.unwrap();
	println!("bitcoin prices submitted at tick {tick}",);
	tick
}

async fn submit_price_if_needed(
	ticker: &Ticker,
	client: &MainchainClient,
	price_index_operator: &sr25519::Pair,
	last_submitted_tick: &mut Tick,
) {
	let current_tick = current_chain_tick(client, ticker).await;
	if current_tick <= *last_submitted_tick {
		return;
	}

	*last_submitted_tick = submit_price(ticker, client, price_index_operator).await;
}

async fn current_chain_tick(client: &MainchainClient, ticker: &Ticker) -> Tick {
	client
		.fetch_storage(&storage().ticks().current_tick(), FetchAt::Best)
		.await
		.unwrap()
		.unwrap_or_else(|| ticker.current())
}

fn get_parent_fingerprint(bitcoind: &BitcoinD, owner_hd_key_path: &DerivationPath) -> Fingerprint {
	let parent_hd_key_path = owner_hd_key_path.to_string();
	let mut parent_hd_key_path = parent_hd_key_path.split('/').collect::<Vec<_>>();
	parent_hd_key_path.pop();
	let parent_part = parent_hd_key_path.pop().unwrap();
	let is_internal_hd = parent_part.ends_with('1');
	let hardened_parent_hd_key_path = parent_hd_key_path.join("/").replace('\'', "h");
	println!("Hardened Parent HD Key Path: {hardened_parent_hd_key_path}");

	let descriptors = bitcoind.client.call::<serde_json::Value>("listdescriptors", &[]).unwrap();
	println!("Descriptors: {descriptors:#?}");
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
	println!("Master Fingerprint: {master_fingerprint}");
	master_fingerprint
}

fn create_xpriv_and_derive() -> anyhow::Result<(bitcoin::bip32::Xpriv, bitcoin::PublicKey, String)>
{
	let seed: [u8; 32] = rand::random();
	let xpriv = xpriv_from_seed(&seed, BitcoinNetwork::Regtest)?;
	let derivation_path = "m/84'/0'/0'";
	let pubkey = derive_pubkey(&xpriv, derivation_path)?;
	let pubkey = PublicKey::from_slice(&pubkey.serialize())
		.map_err(|e| anyhow!("Failed to create bitcoin public key: {e}"))?;

	Ok((xpriv, pubkey, derivation_path.to_string()))
}

fn create_xpriv_and_master_xpub() -> anyhow::Result<(bitcoin::bip32::Xpriv, Xpub, String)> {
	let seed: [u8; 32] = rand::random();
	let xpriv = xpriv_from_seed(&seed, BitcoinNetwork::Regtest)?;
	let derivation_path = "m/0'";
	let xpub = derive_xpub(&xpriv, derivation_path)?;

	Ok((xpriv, xpub, derivation_path.to_string()))
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
	let params = client.params_with_best_nonce(&vault_owner_account_id32.clone()).await?.build();
	let vault_config = api::vaults::calls::types::create::VaultConfig {
		name: None,
		bitcoin_xpubkey: xpubkey.encode().into(),
		terms: api::runtime_types::argon_primitives::vault::VaultTerms::<u128> {
			bitcoin_base_fee: 0,
			bitcoin_annual_percent_rate: to_api_fixed_u128(FixedU128::from_float(0.01)),
			treasury_profit_sharing: to_api_per_mill(Permill::from_percent(50)),
		},
		securitization_ratio: to_api_fixed_u128(FixedU128::from_u32(1)),
		securitization: 100_000_000_000,
	};

	let vault_creation_tx = client
		.submit_tx(&tx().vaults().create(vault_config), vault_signer, Some(params), true)
		.await?;
	let vault_creation = vault_creation_tx
		.events
		.iter()
		.find_map(|event| event.as_event::<api::vaults::events::VaultCreated>().transpose())
		.transpose()?
		.expect("vault created");
	println!("vault created {vault_creation:?}");
	assert_eq!(vault_creation.vault_id, 1);

	Ok(vault_creation.vault_id)
}

async fn request_bitcoin_lock(
	test_node: &ArgonTestNode,
	vault_id: VaultId,
	satoshis: Satoshis,
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

	println!("Owner locked bitcoin with pubkey: {owner_compressed_pubkey}");
	let owner_pubkey: CompressedBitcoinPubkey = (*owner_compressed_pubkey).into();

	let lock_tx = test_node
		.client
		.submit_tx(
			&tx().bitcoin_locks().initialize(vault_id, satoshis, owner_pubkey.into(), None),
			&Sr25519Signer::new(bitcoin_owner.clone()),
			None,
			true,
		)
		.await?;
	println!("bitcoin lock submitted (owner watches for utxo id)");
	let lock_event = lock_tx
		.events
		.iter()
		.find_map(|event| {
			event.as_event::<api::bitcoin_locks::events::BitcoinLockCreated>().transpose()
		})
		.transpose()?
		.expect("lock event");
	let utxo_id = lock_event.utxo_id;
	Ok(utxo_id)
}

#[allow(clippy::too_many_arguments)]
async fn confirm_lock(
	secp: &Secp256k1<All>,
	owner_compressed_pubkey: PublicKey,
	utxo_satoshis: Satoshis,
	client: &Arc<MainchainClient>,
	xpubkey: &Xpub,
	bitcoin_network: Network,
	vault_id: VaultId,
	utxo_id: &UtxoId,
) -> anyhow::Result<(BitcoinCosignScriptPubkey, Balance)> {
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

	assert!(!lock_api.is_funded);
	let liquidity_promised = lock_api.liquidity_promised;
	Ok((lock_api.utxo_script_pubkey.into(), liquidity_promised))
}

#[allow(clippy::too_many_arguments)]
async fn wait_for_mint(
	bitcoin_owner: &sr25519::Pair,
	client: &Arc<MainchainClient>,
	utxo_id: &UtxoId,
	liquidity_promised: Balance,
	txid: Txid,
	vout: u32,
	ticker: &Ticker,
	price_index_operator: &sr25519::Pair,
	last_submitted_tick: &mut Tick,
) -> anyhow::Result<()> {
	let mut finalized_sub = client.live.blocks().subscribe_finalized().await?;
	let pending_utxos = client
		.fetch_storage(&storage().bitcoin_utxos().locks_pending_funding(), FetchAt::Finalized)
		.await?
		.unwrap()
		.0;
	if !pending_utxos.is_empty() {
		println!("Waiting for pending utxo to be verified {pending_utxos:?}");
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
		.fetch_storage(
			&storage().bitcoin_utxos().utxo_id_to_funding_utxo_ref(*utxo_id),
			FetchAt::Finalized,
		)
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
		assert!(balance.free >= liquidity_promised);
	} else {
		assert_eq!(pending_mint.0.len(), 1);
		assert_eq!(pending_mint.0[0].1, owner_account_id32.into());
		// should have minted some amount
		assert!(pending_mint.0[0].2 < liquidity_promised);
		println!(
			"Owner mint pending remaining = {} (balance={})",
			pending_mint.0[0].2, balance.free
		);
		assert!(balance.free > (liquidity_promised - pending_mint.0[0].2));

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

			submit_price_if_needed(ticker, client, price_index_operator, last_submitted_tick).await;
			if counter >= 30 {
				let registered_miners = client
					.fetch_storage(
						&storage().mining_slot().active_miners_count(),
						FetchAt::Finalized,
					)
					.await?
					.expect("registered miners");
				let bitcoin_minted = client
					.fetch_storage(&storage().mint().minted_bitcoin_microgons(), FetchAt::Finalized)
					.await?
					.expect("mining mint");
				let mining_minted = client
					.fetch_storage(&storage().mint().minted_mining_microgons(), FetchAt::Finalized)
					.await?
					.expect("mining mint");
				panic!(
					"Timed out waiting for remaining mint. Last mint was {:?}. Miners registered {:?}. Mining Minted {} microgons, Bitcoin Minted {} microgons",
					pending_mint.0, registered_miners, mining_minted, bitcoin_minted
				);
			}
		}
		println!("Owner minted full bitcoin")
	}
	Ok(())
}

async fn owner_requests_release(
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
	let lock = client
		.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(utxo_id), FetchAt::Finalized)
		.await?
		.ok_or_else(|| anyhow!("No finalized lock found for utxo {utxo_id}"))?;
	let cosign = get_cosign_script(&lock, network)?;
	let bitcoin_network_fee = cosign
		.calculate_fee(
			true,
			out_script_pubkey.script_pubkey(),
			bitcoin::FeeRate::from_sat_per_vb(5).ok_or_else(|| anyhow!("Invalid fee rate"))?,
		)?
		.to_sat();
	let to_script_pubkey: BitcoinScriptPubkey = out_script_pubkey.script_pubkey().into();

	let release_tx = client
		.submit_tx(
			&tx().bitcoin_locks().request_release(
				utxo_id,
				to_script_pubkey.into(),
				bitcoin_network_fee,
			),
			&Sr25519Signer::new(bitcoin_owner.clone()),
			None,
			true,
		)
		.await?;
	println!("bitcoin release request finalized");
	// this is the event that a vault would also monitor
	let release_event = release_tx
		.events
		.iter()
		.find_map(|event| {
			event
				.as_event::<api::bitcoin_locks::events::BitcoinUtxoCosignRequested>()
				.transpose()
		})
		.transpose()?
		.expect("release event");
	assert_eq!(release_event.utxo_id, utxo_id);
	assert_eq!(release_event.vault_id, vault_id);

	Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn vault_cosigns_release(
	client: &MainchainClient,
	vault_signer: &Sr25519Signer,
	vault_id: &VaultId,
	utxo_id: &UtxoId,
	vault_xpriv: &bitcoin::bip32::Xpriv,
	uploaded_xpub_hd_path: &str,
) -> anyhow::Result<()> {
	let pending_cosigns = client
		.fetch_storage(
			&storage().vaults().pending_cosign_by_vault_id(*vault_id),
			FetchAt::Finalized,
		)
		.await?
		.ok_or_else(|| anyhow!("No pending cosign requests found for vault {vault_id}"))?;
	assert!(
		pending_cosigns.0.contains(utxo_id),
		"Missing utxo {utxo_id} from pending cosign requests for vault {vault_id}: {:?}",
		pending_cosigns.0
	);

	let pending_request = client
		.fetch_storage(
			&storage().bitcoin_locks().lock_release_requests_by_utxo_id(*utxo_id),
			FetchAt::Finalized,
		)
		.await?;
	assert!(pending_request.is_some(), "Missing finalized release request for utxo {utxo_id}");

	let lock = client
		.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(*utxo_id), FetchAt::Finalized)
		.await?
		.ok_or_else(|| anyhow!("No finalized lock found for utxo {utxo_id}"))?;
	let mut releaser = load_cosign_releaser(client, *utxo_id, &lock, FetchAt::Finalized).await?;
	let (signature, _) = releaser
		.sign_derived(vault_xpriv.clone(), DerivationPath::from_str(uploaded_xpub_hd_path)?)?;
	let signature: BitcoinSignature = signature
		.try_into()
		.map_err(|_| anyhow!("Unable to translate signature to bytes"))?;

	let _ = client
		.submit_tx(
			&tx().bitcoin_locks().cosign_release(*utxo_id, signature.into()),
			vault_signer,
			None,
			true,
		)
		.await?;
	println!("bitcoin cosign submitted");
	Ok(())
}

async fn owner_sees_signature_and_releases(
	client: &MainchainClient,
	bitcoind: &BitcoinD,
	utxo_id: &UtxoId,
	hd_path: &str,
	fingerprint: &str,
) -> anyhow::Result<()> {
	let mut releaser = load_owner_release_releaser(client, *utxo_id).await?;
	let owner_pubkey = releaser
		.cosign_script
		.script_args
		.bitcoin_owner_pubkey()
		.map_err(|e| anyhow!("Could not convert owner pubkey {e:?}"))?;
	releaser.psbt.inputs[0].bip32_derivation.insert(
		bitcoin::secp256k1::PublicKey::from_slice(&owner_pubkey.to_bytes())?,
		(Fingerprint::from_str(fingerprint)?, DerivationPath::from_str(hd_path)?),
	);
	let psbt_text = general_purpose::STANDARD.encode(&releaser.psbt.serialize()[..]);

	println!("Processing with wallet");
	{
		let psbt = Psbt::from_str(&psbt_text).expect("psbt");
		println!("PSBT from cli: {psbt:#?}");
		let analyzed = bitcoind
			.client
			.call::<serde_json::Value>("analyzepsbt", &[serde_json::to_value(&psbt_text).unwrap()])
			.unwrap();
		println!("Analyzed Psbt: {analyzed:#?}");
	}
	let import = bitcoind.client.wallet_process_psbt(
		&psbt_text,
		Some(true),
		Some(EcdsaSighashType::AllPlusAnyoneCanPay.into()),
		None,
	)?;
	println!("Processed with wallet {import:?}");
	{
		let psbt = Psbt::from_str(import.psbt.as_str()).expect("psbt");
		println!("PSBT after import: {psbt:#?}");
		let analyzed = bitcoind
			.client
			.call::<serde_json::Value>(
				"analyzepsbt",
				&[serde_json::to_value(&import.psbt).unwrap()],
			)
			.unwrap();
		println!("Analyzed Psbt: {analyzed:#?}");
	}

	let finalized = bitcoind.client.finalize_psbt(&import.psbt, None)?;
	println!("Finalized psbt! {finalized:?}");
	let acceptance = bitcoind
		.client
		.test_mempool_accept(&[&finalized.hex.unwrap()])
		.expect("checked");
	let did_accept = acceptance.first().unwrap();
	assert!(did_accept.allowed);

	Ok(())
}

async fn owner_signs_and_releases(
	client: &MainchainClient,
	utxo_id: &UtxoId,
	owner_xpriv: &bitcoin::bip32::Xpriv,
	owner_hd_path: &str,
	bitcoin_rpc_url: &str,
) -> anyhow::Result<()> {
	let mut releaser = load_owner_release_releaser(client, *utxo_id).await?;
	releaser.sign_derived(owner_xpriv.clone(), DerivationPath::from_str(owner_hd_path)?)?;
	let confirmations = Arc::new(std::sync::Mutex::new(0));

	releaser
		.broadcast(bitcoin_rpc_url, Duration::from_secs(10), move |status| {
			let next_confirmations = status.confirmations.unwrap_or(0);
			let mut confirmations = confirmations.lock().unwrap();
			if next_confirmations > *confirmations {
				*confirmations = next_confirmations;
				println!("Transaction confirmations: {confirmations}/6");
				if *confirmations >= 6 {
					return true;
				}
			}
			false
		})
		.await
		.map_err(|e| anyhow!("Failed to broadcast release transaction: {e:?}"))?;
	Ok(())
}

async fn load_cosign_releaser(
	client: &MainchainClient,
	utxo_id: UtxoId,
	lock: &api::runtime_types::pallet_bitcoin_locks::pallet::LockedBitcoin,
	at_block: FetchAt,
) -> anyhow::Result<CosignReleaser> {
	let utxo_ref = client
		.fetch_storage(&storage().bitcoin_utxos().utxo_id_to_funding_utxo_ref(utxo_id), at_block)
		.await?
		.ok_or_else(|| anyhow!("No funding utxo found for lock {utxo_id}"))?;
	let release_request = client
		.fetch_storage(
			&storage().bitcoin_locks().lock_release_requests_by_utxo_id(utxo_id),
			at_block,
		)
		.await?
		.ok_or_else(|| anyhow!("No release request found for lock {utxo_id}"))?;
	let txid: Txid = H256Le(utxo_ref.txid.0).into();
	let to_script_pubkey: BitcoinScriptPubkey = release_request
		.to_script_pubkey
		.try_into()
		.map_err(|_| anyhow!("Unable to decode destination pubkey"))?;
	let bitcoin_network: BitcoinNetwork = client
		.fetch_storage(&storage().bitcoin_utxos().bitcoin_network(), at_block)
		.await?
		.ok_or_else(|| anyhow!("No bitcoin network found"))?
		.into();

	Ok(CosignReleaser::from_script(
		get_cosign_script(lock, bitcoin_network.into())?,
		lock.satoshis,
		txid,
		utxo_ref.output_index,
		argon_bitcoin::ReleaseStep::VaultCosign,
		argon_bitcoin::Amount::from_sat(release_request.bitcoin_network_fee),
		to_script_pubkey.into(),
	)?)
}

async fn load_owner_release_releaser(
	client: &MainchainClient,
	utxo_id: UtxoId,
) -> anyhow::Result<CosignReleaser> {
	let release_height = client
		.fetch_storage(
			&storage().bitcoin_locks().lock_release_cosign_height_by_id(utxo_id),
			FetchAt::Finalized,
		)
		.await?
		.ok_or_else(|| anyhow!("No release cosign height found for utxo {utxo_id}"))?;
	let release_block = client
		.block_at_height(release_height)
		.await?
		.ok_or_else(|| anyhow!("No block found for release height {release_height}"))?;
	let release_event = client
		.live
		.blocks()
		.at(release_block)
		.await?
		.events()
		.await?
		.find_first::<api::bitcoin_locks::events::BitcoinUtxoCosigned>()?
		.ok_or_else(|| anyhow!("No corresponding cosign event found for utxo {utxo_id}"))?;
	let active_height = client.block_at_height(release_height.saturating_sub(1)).await?;
	let fetch_at = active_height.map(Into::into).unwrap_or_default();
	let lock = client
		.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(utxo_id), fetch_at)
		.await?
		.ok_or_else(|| anyhow!("No lock found for utxo {utxo_id}"))?;
	let mut releaser = load_cosign_releaser(client, utxo_id, &lock, fetch_at).await?;
	let vault_signature: BitcoinSignature = release_event
		.signature
		.try_into()
		.map_err(|_| anyhow!("Unable to decode vault signature"))?;

	releaser.add_signature(
		releaser
			.cosign_script
			.script_args
			.bitcoin_vault_pubkey()
			.map_err(|e| anyhow!("Could not convert vault pubkey {e:?}"))?,
		vault_signature.try_into()?,
	);

	let vault_pubkey: CompressedBitcoinPubkey = lock.vault_pubkey.into();
	let vault_pubkey: bitcoin::CompressedPublicKey = vault_pubkey.try_into()?;
	let vault_hd_path = DerivationPath::from(vec![ChildNumber::from(lock.vault_xpub_sources.1)]);
	releaser.psbt.inputs[0]
		.bip32_derivation
		.insert(vault_pubkey.0, (Fingerprint::from(lock.vault_xpub_sources.0), vault_hd_path));

	Ok(releaser)
}

fn get_cosign_script(
	lock: &api::runtime_types::pallet_bitcoin_locks::pallet::LockedBitcoin,
	network: Network,
) -> anyhow::Result<CosignScript> {
	CosignScript::new(
		CosignScriptArgs {
			vault_pubkey: lock.vault_pubkey.clone().into(),
			vault_claim_pubkey: lock.vault_claim_pubkey.clone().into(),
			owner_pubkey: lock.owner_pubkey.clone().into(),
			vault_claim_height: lock.vault_claim_height,
			open_claim_height: lock.open_claim_height,
			created_at_height: lock.created_at_height,
		},
		network,
	)
	.map_err(|e| anyhow!("Unable to create cosign script: {e:?}"))
}
