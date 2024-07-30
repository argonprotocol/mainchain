use std::sync::Arc;

use bitcoin::{
	bip32::{ChildNumber, DerivationPath, Fingerprint, Xpriv, Xpub},
	hashes::Hash,
	secp256k1::Secp256k1,
	Amount, FeeRate, Network, PrivateKey, Txid,
};
use bitcoind::{
	anyhow,
	bitcoincore_rpc::{RawTx, RpcApi},
};
use rand::{rngs::OsRng, RngCore};
use sp_arithmetic::FixedU128;
use sp_core::{crypto::AccountId32, sr25519, Pair};

use ulixee_client::{
	api,
	api::{
		price_index::calls::types::submit::Index,
		runtime_types::{
			pallet_vaults::pallet::VaultConfig,
			sp_arithmetic::fixed_point::FixedU128 as FixedU128Ext,
			ulx_primitives::bond::VaultTerms,
		},
		storage, tx,
	},
	signer::Sr25519Signer,
	MainchainClient,
};
use ulx_bitcoin::{CosignScript, UnlockStep, UtxoUnlocker};
use ulx_primitives::bitcoin::{
	BitcoinScriptPubkey, BitcoinSignature, CompressedBitcoinPubkey, Satoshis,
};
use ulx_testing::{
	add_blocks, add_wallet_address, fund_script_address, start_ulx_test_node, UlxTestOracle,
};

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
	let utxo_satoshis: Satoshis = Amount::ONE_BTC.to_sat() + 500;
	let alice_sr25519 = sr25519::Pair::from_string("//Alice", None).unwrap();
	let price_index_operator = sr25519::Pair::from_string("//Eve", None).unwrap();
	let bob_sr25519 = sr25519::Pair::from_string("//Bob", None).unwrap();

	let vault_master_xpriv = create_xpriv();
	let client = test_node.client.clone();
	let client = Arc::new(client);

	let vault_child_xpriv =
		vault_master_xpriv.derive_priv(&Secp256k1::new(), &[ChildNumber::from_hardened_idx(0)?])?;

	let xpubkey = Xpub::from_priv(&secp, &vault_child_xpriv);
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
					terms: VaultTerms {
						mining_reward_sharing_percent_take: FixedU128Ext(
							FixedU128::from_u32(0).into_inner(),
						),
						mining_annual_percent_rate: FixedU128Ext(
							FixedU128::from_float(0.01).into_inner(),
						),
						mining_base_fee: 0,
						bitcoin_annual_percent_rate: FixedU128Ext(
							FixedU128::from_float(0.01).into_inner(),
						),
						bitcoin_base_fee: 0,
					},
					mining_amount_allocated: 0,
					bitcoin_amount_allocated: 100_000_000,
					bitcoin_xpubkey: xpubkey.encode().into(),
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
				btc_usd_price: FixedU128Ext(FixedU128::from_float(62_000.0).into_inner()),
				argon_usd_target_price: FixedU128Ext(FixedU128::from_float(1.0).into_inner()),
				argon_usd_price: FixedU128Ext(FixedU128::from_float(1.1).into_inner()),
				tick: ticker.current(),
			}),
			&Sr25519Signer::new(price_index_operator.clone()),
		)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("bitcoin prices submitted");

	let ulx_owner_compressed_pubkey: CompressedBitcoinPubkey = owner_compressed_pubkey.into();
	// 3. User calls bond api to start a bitcoin bond
	let bond_tx = client
		.live
		.tx()
		.sign_and_submit_then_watch_default(
			&tx()
				.bonds()
				.bond_bitcoin(vault_id, utxo_satoshis, ulx_owner_compressed_pubkey.into()),
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

	assert_eq!(utxo.owner_pubkey.0.to_vec(), owner_compressed_pubkey.to_bytes());

	// 3. Owner recreates the script from the details and submits to blockchain
	let script_address = {
		let cosign_script = CosignScript::new(
			utxo.vault_pubkey.clone().into(),
			utxo.owner_pubkey.clone().into(),
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
		utxo.vault_pubkey.clone().into(),
		utxo.owner_pubkey.clone().into(),
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
		let utxo = client
			.fetch_storage(&storage().bonds().utxos_by_id(utxo_id), None)
			.await?
			.expect("utxo");

		let child_number = ChildNumber::from(utxo.vault_xpub_source.1);
		assert_eq!(child_number, ChildNumber::from_normal_idx(1).expect("child number"));
		let fingerprint = Fingerprint::from(utxo.vault_xpub_source.0);
		let derivation_path = DerivationPath::from(vec![child_number]);
		assert_eq!(fingerprint, xpubkey.fingerprint());

		let (vault_signature, vault_pubkey) = unlocker
			.sign_derived(vault_child_xpriv, (fingerprint, derivation_path))
			.expect("sign");
		assert_eq!(vault_pubkey.to_bytes(), utxo.vault_pubkey.0.to_vec());
		let vault_signature: BitcoinSignature = vault_signature.try_into().unwrap();

		let progress = client
			.live
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().bonds().cosign_bitcoin_unlock(unlock_event.bond_id, vault_signature.into()),
				&vault_signer,
			)
			.await?;
		MainchainClient::wait_for_ext_in_block(progress).await.expect("finalized");
	};

	println!("User sees the transaction and cosigns");
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

		let utxo = client
			.fetch_storage(&storage().bonds().utxos_by_id(utxo_id), None)
			.await?
			.expect("utxo");

		while let Some(block) = finalized_sub.next().await {
			let block = block?;
			let utxo_unlock =
				block.events().await?.find_first::<api::bonds::events::BitcoinUtxoCosigned>()?;
			if let Some(utxo_unlock) = utxo_unlock {
				if utxo_unlock.bond_id == bond_event.bond_id {
					unlocker.add_signature(
						CompressedBitcoinPubkey::from(utxo.vault_pubkey).try_into().unwrap(),
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

fn create_xpriv() -> Xpriv {
	let mut seed = [0u8; 32];
	OsRng.fill_bytes(&mut seed);
	Xpriv::new_master(Network::Regtest, &seed).unwrap()
}
