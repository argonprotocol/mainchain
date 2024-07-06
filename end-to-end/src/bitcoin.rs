use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use bitcoin::{
	bip32::{DerivationPath, Fingerprint, Xpriv, Xpub},
	hashes::Hash,
	secp256k1::Secp256k1,
	Address, Amount, CompressedPublicKey, FeeRate, Network, PrivateKey, Txid,
};
use bitcoind::{
	anyhow,
	bitcoincore_rpc::{json::GetRawTransactionResult, RawTx, RpcApi},
	BitcoinD,
};
use rand::{rngs::OsRng, RngCore};
use sp_arithmetic::{per_things::Percent, FixedU128};
use sp_core::{crypto::AccountId32, sr25519, Pair};

use ulixee_client::{
	api,
	api::{
		price_index::calls::types::submit::Index,
		runtime_types::{
			pallet_vaults::pallet::VaultConfig,
			sp_arithmetic::{
				fixed_point::FixedU128 as FixedU128Ext, per_things::Percent as PercentExt,
			},
		},
		storage, tx,
	},
	signer::Sr25519Signer,
};
use ulx_bitcoin_utxo_tracker::{CosignScript, UnlockStep, UtxoUnlocker};
use ulx_primitives::bitcoin::{
	BitcoinPubkeyHash, BitcoinScriptPubkey, BitcoinSignature, CompressedBitcoinPubkey, Satoshis,
};
use ulx_testing::{start_ulx_test_node, UlxTestOracle};

#[tokio::test]
async fn test_bitcoin_minting_e2e() -> anyhow::Result<()> {
	let test_node = start_ulx_test_node().await;
	let bitcoind = test_node.bitcoind.as_ref().expect("bitcoind");
	let block_creator = add_wallet_address(bitcoind);
	bitcoind.client.generate_to_address(101, &block_creator).unwrap();

	// 1. Owner creates a new pubkey and submits to blockchain
	let secp = Secp256k1::new();
	let owner_keypair = PrivateKey::generate(Network::Regtest);
	let owner_compressed_pubkey = owner_keypair.public_key(&secp);
	let owner_pubkey_hash: BitcoinPubkeyHash = owner_compressed_pubkey.pubkey_hash().into();
	let utxo_satoshis: Satoshis = Amount::ONE_BTC.to_sat() + 500;
	let alice_sr25519 = sr25519::Pair::from_string("//Alice", None).unwrap();
	let bob_sr25519 = sr25519::Pair::from_string("//Bob", None).unwrap();

	let (vault_master_xpriv, vault_fingerprint) = create_xpriv();
	let client = test_node.client.clone();
	let client = Arc::new(client);
	let mut pubkey_hashes = vec![];
	let mut vault_mappings = BTreeMap::new();
	for i in 0..100 {
		let (vault_compressed_pubkey, vault_hd_path) =
			derive(&vault_master_xpriv, format!("m/48'/0'/0'/0/{i}").as_str());
		let vault_pubkey_hash: BitcoinPubkeyHash = vault_compressed_pubkey.pubkey_hash().into();
		pubkey_hashes.push(vault_pubkey_hash);
		vault_mappings.insert(vault_pubkey_hash, vault_hd_path);
	}
	let vault_signer = Sr25519Signer::new(alice_sr25519.clone());

	let _oracle = UlxTestOracle::bitcoin_tip(&test_node).await?;

	add_blocks(bitcoind, 1, &block_creator);

	let mut finalized_sub = client.live.blocks().subscribe_finalized().await?;
	let vault_owner = sr25519::Pair::from_string("//Alice", None).unwrap();
	let vault_owner_account_id32: AccountId32 = vault_owner.public().into();
	while let Some(_block) = finalized_sub.next().await {
		if let Some(alice_balance) = client
			.fetch_storage(
				&storage()
					.system()
					.account(subxt::utils::AccountId32(*vault_owner_account_id32.as_ref())),
				None,
			)
			.await?
		{
			println!("Alice argon balance {:#?}", alice_balance.data.free);
			if alice_balance.data.free > 100_001_000 {
				println!("Alice can start a vault now!");
				break;
			}
		}
	}

	// 2. A vault is setup with enough funds
	let vault_id = {
		println!("creating a vault");
		let params = client.params_with_best_nonce(vault_owner_account_id32.clone()).await?;
		let vault_tx = client
			.live
			.tx()
			.sign_and_submit_then_watch(
				&tx().vaults().create(VaultConfig {
					mining_reward_sharing_percent_take: PercentExt(
						Percent::from_percent(0).deconstruct(),
					),
					mining_amount_allocated: 0,
					mining_annual_percent_rate: FixedU128Ext(
						FixedU128::from_float(0.01).into_inner(),
					),
					mining_base_fee: 0,
					bitcoin_amount_allocated: 100_000_000,
					bitcoin_annual_percent_rate: FixedU128Ext(
						FixedU128::from_float(0.01).into_inner(),
					),
					bitcoin_base_fee: 0,
					bitcoin_pubkey_hashes: pubkey_hashes
						.into_iter()
						.map(Into::into)
						.collect::<Vec<_>>()
						.into(),
					securitization_percent: FixedU128Ext(FixedU128::from_float(0.0).into_inner()),
				}),
				&vault_signer,
				params.build(),
			)
			.await?
			.wait_for_finalized_success()
			.await?;
		let creation = vault_tx
			.find_first::<api::vaults::events::VaultCreated>()?
			.expect("vault created");
		println!("vault created {:?}", creation);
		assert_eq!(creation.vault_id, 1);
		creation.vault_id
	};

	let ticker = client.lookup_ticker().await.expect("ticker");
	client
		.live
		.tx()
		.sign_and_submit_then_watch_default(
			&tx().price_index().submit(Index {
				btc_usd_price: FixedU128Ext(FixedU128::from_rational(6_200_000, 1_00).into_inner()),
				argon_usd_target_price: FixedU128Ext(FixedU128::from_float(0.99).into_inner()),
				argon_usd_price: FixedU128Ext(FixedU128::from_rational(1_00, 1_00).into_inner()),
				tick: ticker.current(),
			}),
			&Sr25519Signer::new(alice_sr25519.clone()),
		)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("bitcoin prices submitted");

	// 3. User calls bond api to start a bitcoin bond
	let bond_tx = client
		.live
		.tx()
		.sign_and_submit_then_watch_default(
			&tx().bonds().bond_bitcoin(vault_id, utxo_satoshis, owner_pubkey_hash.into()),
			&Sr25519Signer::new(bob_sr25519.clone()),
		)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("bitcoin bond submitted");
	let bond_event = bond_tx.find_first::<api::bonds::events::BondCreated>()?.expect("bond event");
	let utxo_id = bond_event.utxo_id.unwrap();
	let bond_amount = bond_event.amount;

	let utxo = client
		.fetch_storage(&storage().bonds().utxos_by_id(utxo_id), None)
		.await?
		.expect("utxo");

	assert_eq!(utxo.owner_pubkey_hash.0, owner_pubkey_hash.0);

	// 3. Owner recreates the script from the details and submits to blockchain
	let script_address = {
		let cosign_script = CosignScript::new(
			BitcoinPubkeyHash(utxo.vault_pubkey_hash.0),
			BitcoinPubkeyHash(utxo.owner_pubkey_hash.0),
			utxo.vault_claim_height,
			utxo.open_claim_height,
			utxo.register_block,
		)
		.expect("script address");
		cosign_script.get_script_address(Network::Regtest)
	};

	let block_creator = add_wallet_address(bitcoind);
	let (txid, vout, _raw_tx) =
		fund_script_address(bitcoind, &script_address, utxo_satoshis, &block_creator);

	add_blocks(bitcoind, 5, &block_creator);

	while let Some(block) = finalized_sub.next().await {
		let block = block?;
		let utxo_verified =
			block.events().await?.find_first::<api::bitcoin_utxos::events::UtxoVerified>()?;
		if let Some(utxo_verified) = utxo_verified {
			if utxo_verified.utxo_id == 1 {
				println!("Utxo verified in Ulixee mainchain");
				break;
			}
		}
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
	let balance = client
		.get_argons(owner_account_id32.clone())
		.await
		.expect("pending mint balance");
	if pending_mint.0.is_empty() {
		assert!(balance.free >= bond_amount as u128);
	} else {
		assert_eq!(pending_mint.0.len(), 1);
		let subxt_owner_account_id = subxt::utils::AccountId32(*owner_account_id32.as_ref());
		assert_eq!(pending_mint.0[0].1, subxt_owner_account_id.clone());
		// should have minted some amount
		assert!(pending_mint.0[0].2 < bond_amount as u128);
		println!(
			"Owner mint pending remaining = {} (balance={})",
			pending_mint.0[0].2, balance.free
		);
		assert!(balance.free > (bond_amount as u128 - pending_mint.0[0].2));

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
	println!("Owner has been minted {} (balance={})", bond_amount, balance.free);

	// 5. Ask for the bitcoin to be unlocked
	let out_script_pubkey: BitcoinScriptPubkey =
		owner_compressed_pubkey.p2wpkh_script_code().unwrap().try_into().unwrap();
	let feerate = FeeRate::from_sat_per_vb(15).expect("cant translate fee");

	let user_cosign_script = CosignScript::new(
		BitcoinPubkeyHash(utxo.vault_pubkey_hash.0),
		BitcoinPubkeyHash(utxo.owner_pubkey_hash.0),
		utxo.vault_claim_height,
		utxo.open_claim_height,
		utxo.register_block,
	)
	.unwrap();
	let fee = user_cosign_script
		.calculate_fee(true, out_script_pubkey.clone().into(), feerate)
		.unwrap();

	println!("Owner requests bitcoin bond unlock");
	let unlock_tx = client
		.live
		.tx()
		.sign_and_submit_then_watch_default(
			&tx().bonds().unlock_bitcoin_bond(
				bond_event.bond_id,
				out_script_pubkey.clone().into(),
				fee.to_sat(),
			),
			&Sr25519Signer::new(bob_sr25519.clone()),
		)
		.await?
		.wait_for_finalized_success()
		.await?;

	// this is the event that a vault would also monitor
	let unlock_event = unlock_tx
		.find_first::<api::bonds::events::BitcoinUtxoCosignRequested>()?
		.expect("unlock event");
	assert_eq!(unlock_event.bond_id, bond_event.bond_id);
	assert_eq!(unlock_event.vault_id, vault_id);

	// 5. vault sees unlock request (outaddress, fee) and creates a transaction
	{
		println!("Vault publishes cosign tx");
		let mut unlocker = UtxoUnlocker::from_script(
			user_cosign_script.clone(),
			utxo_satoshis,
			Txid::from_slice(utxo_ref.txid.clone().0.as_slice()).unwrap(),
			utxo_ref.output_index,
			UnlockStep::VaultCosign,
			fee,
			out_script_pubkey.clone().into(),
		)
		.expect("unlocker");

		let vault_pubkey_hash = BitcoinPubkeyHash(utxo.vault_pubkey_hash.0);

		let vault_hd_path = vault_mappings.get(&vault_pubkey_hash).unwrap();
		let (vault_signature, vault_pubkey) = unlocker
			.sign_derived(vault_master_xpriv, (vault_fingerprint, vault_hd_path.clone()))
			.expect("sign");
		let vault_signature: BitcoinSignature = vault_signature.try_into().unwrap();
		let vault_pubkey: CompressedBitcoinPubkey = vault_pubkey.into();

		client
			.live
			.tx()
			.sign_and_submit_default(
				&tx().bonds().cosign_bitcoin_unlock(
					unlock_event.bond_id,
					vault_pubkey.into(),
					vault_signature.into(),
				),
				&vault_signer,
			)
			.await?;
	};

	// 6. User sees the transaction and cosigns
	let tx = {
		let mut unlocker = UtxoUnlocker::from_script(
			user_cosign_script.clone(),
			utxo_satoshis,
			Txid::from_slice(utxo_ref.txid.0.as_slice()).expect("deserialize txid"),
			utxo_ref.output_index,
			UnlockStep::VaultCosign,
			fee,
			out_script_pubkey.clone().into(),
		)
		.unwrap();

		while let Some(block) = finalized_sub.next().await {
			let block = block?;
			let utxo_unlock =
				block.events().await?.find_first::<api::bonds::events::BitcoinUtxoCosigned>()?;
			if let Some(utxo_unlock) = utxo_unlock {
				if utxo_unlock.bond_id == bond_event.bond_id {
					unlocker.add_signature(
						CompressedBitcoinPubkey::from(utxo_unlock.pubkey).try_into().unwrap(),
						BitcoinSignature::try_from(utxo_unlock.signature)
							.unwrap()
							.try_into()
							.unwrap(),
					);
					break;
				}
			}
		}
		unlocker.sign(owner_keypair).expect("sign");
		unlocker.extract_tx().expect("tx")
	};

	println!("Bitcoin cosigned unlock script created");
	let tx_hex = tx.raw_hex();

	let acceptance = bitcoind.client.test_mempool_accept(&[tx_hex.clone()]).expect("checked");
	let did_accept = acceptance.first().unwrap();
	assert!(did_accept.allowed);
	Ok(())
}

fn fund_script_address(
	bitcoind: &BitcoinD,
	script_address: &Address,
	amount: Satoshis,
	block_address: &Address,
) -> (Txid, u32, GetRawTransactionResult) {
	let txid = bitcoind
		.client
		.send_to_address(
			script_address,
			Amount::from_sat(amount),
			None,
			None,
			None,
			None,
			None,
			None,
		)
		.unwrap();
	let tx = wait_for_txid(bitcoind, &txid, block_address);
	let vout = tx
		.vout
		.iter()
		.position(|o| o.script_pub_key.script().unwrap() == script_address.script_pubkey())
		.unwrap() as u32;
	(txid, vout, tx)
}

fn wait_for_txid(
	bitcoind: &BitcoinD,
	txid: &Txid,
	block_address: &Address,
) -> GetRawTransactionResult {
	loop {
		// Attempt to get the raw transaction with verbose output
		let result = bitcoind.client.call::<GetRawTransactionResult>(
			"getrawtransaction",
			&[txid.to_string().into(), 1.into()],
		);

		if let Ok(tx) = result {
			if tx.confirmations.unwrap_or_default() > 1 {
				return tx;
			}
		}
		std::thread::sleep(std::time::Duration::from_secs(1));
		add_blocks(bitcoind, 1, block_address);
	}
}

fn create_xpriv() -> (Xpriv, Fingerprint) {
	let secp = Secp256k1::new();
	let mut seed = [0u8; 32];
	OsRng.fill_bytes(&mut seed);
	let master_xpriv = Xpriv::new_master(Network::Regtest, &seed).unwrap();
	let master_xpub = Xpub::from_priv(&secp, &master_xpriv);
	let fingerprint = master_xpub.fingerprint();
	(master_xpriv, fingerprint)
}

fn derive(master_xpriv: &Xpriv, path: &str) -> (CompressedPublicKey, DerivationPath) {
	let secp = Secp256k1::new();
	let path = DerivationPath::from_str(path).expect("Invalid derivation path");
	let child_xpriv = master_xpriv.derive_priv(&secp, &path).expect("Unable to derive child key");

	let vault_privkey = child_xpriv.to_priv();
	let vault_compressed_pubkey = CompressedPublicKey::from_private_key(&secp, &vault_privkey)
		.expect("Unable to derive pubkey");
	(vault_compressed_pubkey, path)
}

fn add_wallet_address(bitcoind: &BitcoinD) -> Address {
	let address = bitcoind.client.get_new_address(None, None).unwrap();
	address.require_network(Network::Regtest).unwrap()
}

fn add_blocks(bitcoind: &BitcoinD, count: u64, grant_to_address: &Address) {
	bitcoind.client.generate_to_address(count, grant_to_address).unwrap();
}
