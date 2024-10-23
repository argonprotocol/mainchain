use std::{env, str::FromStr, sync::Arc};

use bitcoin::{
	bip32::{ChildNumber, DerivationPath, Fingerprint, Xpub},
	hashes::Hash,
	secp256k1::{All, Secp256k1},
	Amount, EcdsaSighashType, Network, Psbt, PublicKey, ScriptBuf, Txid,
};
use bitcoind::{
	anyhow,
	bitcoincore_rpc::{bitcoincore_rpc_json::AddressType, jsonrpc::serde_json, RpcApi},
	BitcoinD,
};
use sp_arithmetic::FixedU128;
use sp_core::{crypto::AccountId32, sr25519, Pair};

use anyhow::anyhow;
use argon_bitcoin::{CosignScript, CosignScriptArgs};
use argon_client::{
	api,
	api::{
		price_index::calls::types::submit::Index,
		runtime_types::sp_arithmetic::fixed_point::FixedU128 as FixedU128Ext, storage, tx,
	},
	signer::{Signer, Sr25519Signer},
	ArgonConfig, MainchainClient,
};
use argon_primitives::{
	argon_utils::format_argons,
	bitcoin::{BitcoinCosignScriptPubkey, BitcoinNetwork, Satoshis, UtxoId, SATOSHIS_PER_BITCOIN},
	Balance, BondId, VaultId,
};
use argon_testing::{
	add_blocks, add_wallet_address, fund_script_address, run_bitcoin_cli, start_argon_test_node,
	ArgonTestNode, ArgonTestOracle,
};
use tokio::fs;

#[tokio::test]
async fn test_bitcoin_minting_e2e() {
	let test_node = start_argon_test_node().await;
	let bitcoind = test_node.bitcoind.as_ref().expect("bitcoind");
	let block_creator = add_wallet_address(bitcoind);
	bitcoind.client.generate_to_address(101, &block_creator).unwrap();

	// 1. Owner creates a new pubkey and submits to blockchain
	let network: BitcoinNetwork = test_node
		.client
		.fetch_storage(&storage().bitcoin_utxos().bitcoin_network(), None)
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
	let alice_sr25519 = sr25519::Pair::from_string("//Alice", None).unwrap();
	let price_index_operator = sr25519::Pair::from_string("//Eve", None).unwrap();
	let bob_sr25519 = sr25519::Pair::from_string("//Bob", None).unwrap();

	let client = test_node.client.clone();
	let client = Arc::new(client);

	let (vault_xpriv_path, vault_xpriv_pwd, vault_xpub, vault_xpub_hd_path) =
		create_xpriv(&test_node).await.unwrap();
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
	let tick = ticker.current();
	client
		.live
		.tx()
		.sign_and_submit_then_watch_default(
			&tx().price_index().submit(Index {
				btc_usd_price: FixedU128Ext(FixedU128::from_float(62_000.0).into_inner()),
				argon_usd_target_price: FixedU128Ext(FixedU128::from_float(1.0).into_inner()),
				argon_usd_price: FixedU128Ext(FixedU128::from_float(1.1).into_inner()),
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

	let _ = run_bitcoin_cli(&test_node, vec!["vault", "list", "--btc", &utxo_btc.to_string()])
		.await
		.unwrap();

	// 3. Owner calls bond api to start a bitcoin bond
	let (utxo_id, bond_id) =
		create_bond(&test_node, vault_id, utxo_btc, &owner_compressed_pubkey, &bob_sr25519)
			.await
			.unwrap();

	let send_to_address = run_bitcoin_cli(
		&test_node,
		vec!["bond", "send-to-address", "--bond-id", &bond_id.to_string()],
	)
	.await
	.unwrap();

	let (script_address, bond_amount) = confirm_bond(
		&test_node,
		&secp,
		owner_compressed_pubkey,
		utxo_satoshis,
		&client,
		vault_xpub,
		network,
		vault_id,
		&bond_id,
	)
	.await
	.unwrap();

	// 4. Owner funds the bond utxo and submits it
	let scriptbuf: ScriptBuf = script_address.into();
	let scriptaddress = bitcoin::Address::from_script(scriptbuf.as_script(), network).unwrap();
	println!("Checking for {} satoshis to {}", utxo_satoshis, scriptaddress);
	assert!(send_to_address.contains(&format!("{} satoshis to {}", utxo_satoshis, scriptaddress)));

	let (txid, vout, _) =
		fund_script_address(bitcoind, &scriptaddress, utxo_satoshis, &block_creator);

	add_blocks(bitcoind, 5, &block_creator);

	wait_for_mint(&bob_sr25519, &client, &utxo_id, bond_amount, txid, vout)
		.await
		.unwrap();

	let _ = run_bitcoin_cli(&test_node, vec!["bond", "get", "--bond-id", &bond_id.to_string()])
		.await
		.unwrap();

	// 5. Ask for the bitcoin to be unlocked
	println!("\nOwner requests unlock");
	owner_requests_unlock(&test_node, bitcoind, network, &bob_sr25519, &client, vault_id, bond_id)
		.await
		.unwrap();

	// 5. vault sees unlock request (outaddress, fee) and creates a transaction
	println!("\nVault publishes cosign tx");
	vault_cosigns_unlock(
		&test_node,
		client,
		&vault_signer,
		&vault_id,
		&bond_id,
		&vault_xpriv_path,
		&vault_xpriv_pwd,
		&vault_xpub_hd_path,
	)
	.await
	.unwrap();

	println!("\nOwner sees the transaction and cosigns");
	// 6. Owner sees the transaction and can submit it
	owner_sees_signature_and_unlocks(
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
				let bracketed = desc_str.split('[').last().unwrap();
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

async fn create_xpriv(test_node: &ArgonTestNode) -> anyhow::Result<(String, String, Xpub, String)> {
	let path = env::temp_dir().join("vault0.xpriv");
	if path.is_file() {
		fs::remove_file(&path).await?;
	}
	let password = "Password123";
	let _ = run_bitcoin_cli(
		test_node,
		vec!["xpriv", "master", "--password", password, "--xpriv-path", path.to_str().unwrap()],
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
			"--password",
			password,
			"--hd-path",
			derivation_path,
		],
	)
	.await?;
	let xpub_hex = hex::decode(xpub_result.split("0x").last().unwrap().trim())?;

	let xpub = Xpub::decode(xpub_hex.as_ref())?;
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
	while let Some(_block) = finalized_sub.next().await {
		if let Some(alice_balance) =
			client.fetch_storage(&storage().system().account(&vault_account), None).await?
		{
			println!("Alice argon balance {:#?}", alice_balance.data.free);
			if alice_balance.data.free > 100_001_000 {
				println!("Alice can start a vault now!");
				break;
			}
		}
	}

	println!("creating a vault");
	let params = client.params_with_best_nonce(vault_owner_account_id32.clone()).await?;

	let result = run_bitcoin_cli(
		test_node,
		vec![
			"vault",
			"create",
			"--bitcoin-argons",
			"100000",
			"--bitcoin-xpub",
			xpubkey.to_string().as_str(),
			"--bitcoin-apr",
			"1.0",
			"--bitcoin-base-fee",
			"0",
			"--mining-argons",
			"0",
			"--mining-apr",
			"1",
			"--mining-base-fee",
			"0",
			"--mining-reward-sharing-percent-take",
			"0",
			"--securitization-percent",
			"0",
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

async fn create_bond(
	test_node: &ArgonTestNode,
	vault_id: VaultId,
	utxo_btc: f64,
	owner_compressed_pubkey: &bitcoin::PublicKey,
	bob_sr25519: &sr25519::Pair,
) -> anyhow::Result<(UtxoId, BondId)> {
	let bond_cli_result = run_bitcoin_cli(
		test_node,
		vec![
			"bond",
			"apply",
			"--vault-id",
			&vault_id.to_string(),
			"--btc",
			&utxo_btc.to_string(),
			"--owner-pubkey",
			&owner_compressed_pubkey.to_string(),
		],
	)
	.await?;

	let bond_tx = test_node
		.client
		.submit_from_polkadot_url(&bond_cli_result, &Sr25519Signer::new(bob_sr25519.clone()), None)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("bitcoin bond submitted (owner watches for bond id)");
	let bond_event = bond_tx.find_first::<api::bonds::events::BondCreated>()?.expect("bond event");
	let utxo_id = bond_event.utxo_id.unwrap();
	let bond_id = bond_event.bond_id;
	Ok((utxo_id, bond_id))
}

#[allow(clippy::too_many_arguments)]
async fn confirm_bond(
	test_node: &ArgonTestNode,
	secp: &Secp256k1<All>,
	owner_compressed_pubkey: PublicKey,
	utxo_satoshis: Satoshis,
	client: &Arc<MainchainClient>,
	xpubkey: Xpub,
	bitcoin_network: Network,
	vault_id: VaultId,
	bond_id: &BondId,
) -> anyhow::Result<(BitcoinCosignScriptPubkey, Balance)> {
	let bond_cli_get =
		run_bitcoin_cli(test_node, vec!["bond", "get", "--bond-id", &bond_id.to_string()]).await?;

	let bond_api = client
		.fetch_storage(&storage().bonds().bonds_by_id(bond_id), None)
		.await?
		.expect("should be able to retrieve");
	assert_eq!(bond_api.vault_id, vault_id);
	let utxo_api = client
		.fetch_storage(&storage().bonds().utxos_by_id(bond_api.utxo_id.expect("")), None)
		.await?
		.unwrap();
	{
		assert_eq!(utxo_api.satoshis, utxo_satoshis);
		assert_eq!(utxo_api.owner_pubkey.0, owner_compressed_pubkey.inner.serialize());
		assert_eq!(utxo_api.vault_xpub_sources.0, xpubkey.fingerprint().to_bytes());
		assert_eq!(
			utxo_api.vault_xpub_sources.1,
			Into::<u32>::into(ChildNumber::from_normal_idx(1)?)
		);
		assert_eq!(
			utxo_api.vault_pubkey.0,
			xpubkey
				.derive_pub(secp, &DerivationPath::from_str("1")?)?
				.public_key
				.serialize()
		);
		let cosign_script = CosignScript::new(
			CosignScriptArgs {
				vault_pubkey: utxo_api.vault_pubkey.clone().into(),
				owner_pubkey: utxo_api.owner_pubkey.into(),
				vault_claim_pubkey: utxo_api.vault_claim_pubkey.into(),
				vault_claim_height: utxo_api.vault_claim_height,
				open_claim_height: utxo_api.open_claim_height,
				created_at_height: utxo_api.created_at_height,
			},
			bitcoin_network,
		)
		.map_err(|_| anyhow!("Unable to create a script"))?;
		let cosign_key = cosign_script.script.to_p2wsh();
		let cosign_script_pubkey: BitcoinCosignScriptPubkey =
			cosign_key.try_into().map_err(|_| anyhow!("Unable to convert script pubkey"))?;
		assert_eq!(cosign_script_pubkey, utxo_api.utxo_script_pubkey.clone().into());
	}

	assert!(bond_cli_get
		.lines()
		.find(|line| line.contains("Minted Argons"))
		.unwrap()
		.contains(&format!("₳0 of {}", format_argons(bond_api.amount))));
	let bond_amount = bond_api.amount;
	Ok((utxo_api.utxo_script_pubkey.into(), bond_amount))
}

async fn wait_for_mint(
	bob_sr25519: &sr25519::Pair,
	client: &Arc<MainchainClient>,
	utxo_id: &UtxoId,
	bond_amount: Balance,
	txid: Txid,
	vout: u32,
) -> anyhow::Result<()> {
	let mut finalized_sub = client.live.blocks().subscribe_finalized().await?;
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
	}
	// load 2 more blocks
	for _ in 0..2 {
		let _ = finalized_sub.next().await;
	}
	let utxo_ref = client
		.fetch_storage(&storage().bitcoin_utxos().utxo_id_to_ref(utxo_id), None)
		.await?
		.expect("utxo");
	assert_eq!(utxo_ref.txid.0, txid.to_byte_array());
	assert_eq!(utxo_ref.output_index, vout);

	let pending_mint = client
		.fetch_storage(&storage().mint().pending_mint_utxos(), None)
		.await?
		.expect("pending mint");

	let owner_account_id32: AccountId32 = bob_sr25519.clone().public().into();
	let balance = client.get_argons(&owner_account_id32).await.expect("pending mint balance");
	if pending_mint.0.is_empty() {
		assert!(balance.free >= bond_amount);
	} else {
		assert_eq!(pending_mint.0.len(), 1);
		assert_eq!(pending_mint.0[0].1, owner_account_id32.into());
		// should have minted some amount
		assert!(pending_mint.0[0].2 < bond_amount);
		println!(
			"Owner mint pending remaining = {} (balance={})",
			pending_mint.0[0].2, balance.free
		);
		assert!(balance.free > (bond_amount - pending_mint.0[0].2));

		// 4. Wait for the full payout
		while let Some(_block) = finalized_sub.next().await {
			let pending_mint = client
				.fetch_storage(&storage().mint().pending_mint_utxos(), None)
				.await?
				.expect("pending mint");
			if pending_mint.0.is_empty() {
				break;
			}
		}
	}
	Ok(())
}

async fn owner_requests_unlock(
	test_node: &ArgonTestNode,
	bitcoind: &BitcoinD,
	network: Network,
	bob_sr25519: &sr25519::Pair,
	client: &Arc<MainchainClient>,
	vault_id: VaultId,
	bond_id: BondId,
) -> anyhow::Result<()> {
	let out_script_pubkey = bitcoind
		.client
		.get_new_address(Some("takeback"), Some(AddressType::Bech32m))
		.unwrap()
		.require_network(network)?;
	let unlock_request_cli = run_bitcoin_cli(
		test_node,
		vec![
			"bond",
			"request-unlock",
			"--bond-id",
			&bond_id.to_string(),
			"--dest-pubkey",
			&out_script_pubkey.to_string(),
		],
	)
	.await?;

	let unlock_request_tx = client
		.submit_from_polkadot_url(
			&unlock_request_cli,
			&Sr25519Signer::new(bob_sr25519.clone()),
			None,
		)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("bitcoin unlock request finalized");
	// this is the event that a vault would also monitor
	let unlock_event = unlock_request_tx
		.find_first::<api::bonds::events::BitcoinUtxoCosignRequested>()?
		.expect("unlock event");
	assert_eq!(unlock_event.bond_id, bond_id);
	assert_eq!(unlock_event.vault_id, vault_id);

	Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn vault_cosigns_unlock(
	test_node: &ArgonTestNode,
	client: Arc<MainchainClient>,
	vault_signer: &Sr25519Signer,
	vault_id: &VaultId,
	bond_id: &BondId,
	xpriv_path: &str,
	xpriv_pwd: &str,
	uploaded_xpub_hd_path: &str,
) -> anyhow::Result<()> {
	let pending_unlock = run_bitcoin_cli(
		test_node,
		vec!["vault", "pending-unlock", "--vault-id", &vault_id.to_string()],
	)
	.await?;

	assert!(pending_unlock.lines().count() > 3);
	assert!(pending_unlock.contains('1'));

	let unlock_fulfill_cli = run_bitcoin_cli(
		test_node,
		vec![
			"bond",
			"vault-cosign",
			"--bond-id",
			&bond_id.to_string(),
			"--xpriv-path",
			xpriv_path,
			"--password",
			xpriv_pwd,
			"--master-xpub-hd-path",
			uploaded_xpub_hd_path,
		],
	)
	.await?;

	let _ = client
		.submit_from_polkadot_url(&unlock_fulfill_cli, vault_signer, None)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("bitcoin cosign submitted");
	Ok(())
}

async fn owner_sees_signature_and_unlocks(
	test_node: &ArgonTestNode,
	bitcoind: &BitcoinD,
	utxo_id: &UtxoId,
	hd_path: &str,
	fingerprint: &str,
) -> anyhow::Result<()> {
	let owner_cosign_cli = run_bitcoin_cli(
		test_node,
		vec![
			"bond",
			"owner-cosign-psbt",
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
		.last()
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
