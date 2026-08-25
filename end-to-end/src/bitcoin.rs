use crate::utils::{create_active_notary, sudo};
use anyhow::anyhow;
use argon_bitcoin::{
	derive_pubkey, derive_xpub, xpriv_from_seed, CosignReleaser, CosignScript, CosignScriptArgs,
};
use argon_client::{
	api,
	api::{
		price_index::calls::types::submit::Index,
		runtime_types::{
			argon_runtime::{RuntimeCall, RuntimeError},
			pallet_bitcoin_fissions::pallet::Error as BitcoinFissionsError,
			pallet_bitcoin_locks::pallet::{
				Error as BitcoinLocksError, LockOptions as BitcoinLockOptions,
			},
			sp_arithmetic::fixed_point::FixedU128 as FixedU128Ext,
		},
		storage, tx,
	},
	conversion::{to_api_fixed_u128, to_api_per_mill},
	signer::{Signer, Sr25519Signer},
	subxt_error, ArgonConfig, FetchAt, MainchainClient,
};
use argon_primitives::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinNetwork, BitcoinScriptPubkey, BitcoinSignature,
		CompressedBitcoinPubkey, FissionId, H256Le, LiquidId, Satoshis, UtxoId,
	},
	block_seal::MiningSlotConfig,
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
use subxt::ext::scale_encode::EncodeAsType;
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

	println!("\n1. Create the Bitcoin owner key");
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
	let bitcoin_owner_account_id: AccountId32 = bitcoin_owner_pair.public().into();
	let bitcoin_owner_signer = Sr25519Signer::new(bitcoin_owner_pair.clone());

	let client = test_node.client.clone();
	let client = Arc::new(client);

	let (vault_xpriv, vault_xpub, vault_xpub_hd_path) = create_xpriv_and_master_xpub().unwrap();
	let vault_signer = Sr25519Signer::new(alice_sr25519.clone());

	let _oracle = ArgonTestOracle::bitcoin_tip(&test_node).await.unwrap();

	add_blocks(bitcoind, 1, &block_creator);

	let vault_owner = alice_sr25519.clone();
	let vault_owner_account_id32: AccountId32 = vault_owner.public().into();

	println!("\n2. Create a funded Vault");
	let vault_id = create_vault(&test_node, &vault_xpub, &vault_owner_account_id32, &vault_signer)
		.await
		.unwrap();

	let ticker = client.lookup_ticker().await.expect("ticker");
	let mut last_bitcoin_price_tick =
		submit_price(&ticker, &client, &price_index_operator, 62_000.0).await;

	println!("\n3. Create a Bitcoin receive address backed by a Lock");
	let utxo_id = create_receive_address(
		&test_node,
		vault_id,
		utxo_satoshis,
		&owner_compressed_pubkey,
		&bitcoin_owner_pair,
	)
	.await
	.unwrap();

	let (cosign_script_pubkey, microgons_at_target_per_btc) = verify_lock(
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

	println!("\n4. Fund the Lock's Bitcoin address");
	let funding_script_pubkey: ScriptBuf = cosign_script_pubkey.into();
	let funding_address =
		bitcoin::Address::from_script(funding_script_pubkey.as_script(), network).unwrap();
	println!("Checking for {utxo_satoshis} satoshis to {funding_address}");

	let (txid, vout, _) =
		fund_script_address(bitcoind, &funding_address, utxo_satoshis, &block_creator);

	add_blocks(bitcoind, 5, &block_creator);
	wait_for_lock_funding(&client, utxo_id, utxo_satoshis, txid, vout)
		.await
		.unwrap();

	let fission_id: FissionId = 1;
	let second_fission_id: FissionId = 3;
	let liquid_id: LiquidId = 1;
	let fission_satoshis = utxo_satoshis / 2;
	let second_fission_satoshis = utxo_satoshis - fission_satoshis;
	println!("\n5. Create the first Fission and check its active constraints");
	let liquidity_promised = create_first_fission_and_check_constraints(
		bitcoind,
		network,
		&client,
		&bitcoin_owner_signer,
		&bitcoin_owner_account_id,
		utxo_id,
		fission_id,
		liquid_id,
		fission_satoshis,
		microgons_at_target_per_btc,
	)
	.await;

	let original_mining_cadence = speed_up_minting(&test_node, &client).await;

	println!("\n6. Pay the first Fission's mint entitlement");
	wait_for_mint(
		&bitcoin_owner_pair,
		&client,
		&utxo_id,
		fission_id,
		liquidity_promised,
		&ticker,
		&price_index_operator,
		&mut last_bitcoin_price_tick,
	)
	.await
	.unwrap();

	println!("\n7. Allocate the remaining Lock satoshis to a second Fission");
	create_second_fission_and_check_allocation(
		&bitcoin_owner_pair,
		&client,
		&bitcoin_owner_signer,
		utxo_id,
		fission_id,
		second_fission_id,
		liquid_id,
		second_fission_satoshis,
		microgons_at_target_per_btc,
		utxo_satoshis,
		&ticker,
		&price_index_operator,
		&mut last_bitcoin_price_tick,
	)
	.await
	.unwrap();

	println!("\n8. Ratchet the first Fission and pay its replacement entitlement");
	ratchet_first_fission(
		&client,
		&ticker,
		&price_index_operator,
		&bitcoin_owner_signer,
		&bitcoin_owner_pair,
		&bitcoin_owner_account_id,
		utxo_id,
		fission_id,
		utxo_satoshis,
		microgons_at_target_per_btc,
		liquidity_promised,
		&mut last_bitcoin_price_tick,
	)
	.await
	.unwrap();

	println!("\n9. Close both Fissions and return their Lock allocations");
	close_fissions(
		&client,
		&bitcoin_owner_signer,
		&bitcoin_owner_account_id,
		utxo_id,
		fission_id,
		second_fission_id,
		second_fission_satoshis,
	)
	.await;
	restore_mining_cadence(&test_node, original_mining_cadence).await;

	submit_price_if_needed(&ticker, &client, &price_index_operator, &mut last_bitcoin_price_tick)
		.await;

	println!("\n10. Request the Lock's Bitcoin release");
	owner_requests_release(bitcoind, network, &bitcoin_owner_pair, &client, vault_id, utxo_id)
		.await
		.unwrap();

	println!("\n11. Vault cosigns the release request");
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

	println!("\n12. Owner cosigns and broadcasts the Bitcoin transaction");
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
async fn test_bitcoin_xpriv_release_e2e() {
	let test_node = start_argon_test_node().await;
	let bitcoind = test_node.bitcoind.as_ref().expect("bitcoind");
	let block_creator = add_wallet_address(bitcoind);
	bitcoind.client.generate_to_address(101, &block_creator).unwrap();

	println!("\n1. Set up a Lock with an xpriv-derived owner key");
	let network: BitcoinNetwork = test_node
		.client
		.fetch_storage(&storage().bitcoin_utxos().bitcoin_network(), FetchAt::Finalized)
		.await
		.unwrap()
		.ok_or(anyhow!("No bitcoin network found"))
		.unwrap()
		.into();
	let network: Network = network.into();

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

	let vault_id = create_vault(&test_node, &vault_xpub, &vault_owner_account_id32, &vault_signer)
		.await
		.unwrap();

	let ticker = client.lookup_ticker().await.expect("ticker");
	submit_price(&ticker, &client, &price_index_operator, 62_000.0).await;

	let utxo_id = create_receive_address(
		&test_node,
		vault_id,
		utxo_satoshis,
		&owner_compressed_pubkey,
		&bitcoin_owner_pair,
	)
	.await
	.unwrap();

	let lock = client
		.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(utxo_id), FetchAt::Finalized)
		.await
		.unwrap()
		.expect("xpriv-owned Lock");
	let cosign_script_pubkey: BitcoinCosignScriptPubkey = lock.utxo_script_pubkey.into();

	println!("\n2. Fund the xpriv-owned Lock");
	let funding_script_pubkey: ScriptBuf = cosign_script_pubkey.into();
	let funding_address =
		bitcoin::Address::from_script(funding_script_pubkey.as_script(), network).unwrap();
	println!("Checking for {utxo_satoshis} satoshis to {funding_address}");

	let (txid, vout, _) =
		fund_script_address(bitcoind, &funding_address, utxo_satoshis, &block_creator);

	add_blocks(bitcoind, 5, &block_creator);
	wait_for_lock_funding(&client, utxo_id, utxo_satoshis, txid, vout)
		.await
		.unwrap();

	println!("\n3. Request the Lock's Bitcoin release");
	owner_requests_release(bitcoind, network, &bitcoin_owner_pair, &client, vault_id, utxo_id)
		.await
		.unwrap();

	println!("\n4. Vault cosigns the release request");
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

	println!("\n5. Owner signs the release with the xpriv and broadcasts it");
	let (bitcoin_url, auth) = test_node.get_bitcoin_url();
	let bitcoind_client = bitcoincore_rpc::Client::new(&bitcoin_url, auth.clone()).unwrap();
	let mut authenticated_bitcoin_url = Url::parse(&bitcoin_url).unwrap();
	if let Auth::UserPass(user, pass) = auth {
		authenticated_bitcoin_url.set_username(&user).unwrap();
		authenticated_bitcoin_url.set_password(Some(&pass)).unwrap();
	}
	println!("Owner pubkey is {owner_compressed_pubkey}");
	tokio::spawn(async move {
		loop {
			bitcoind_client.generate_to_address(1, &block_creator).unwrap();
			println!("Bitcoin block generated");
			sleep(Duration::from_secs(5)).await;
		}
	});
	owner_signs_and_releases(
		client.as_ref(),
		&utxo_id,
		&owner_xpriv,
		&owner_hd_path,
		authenticated_bitcoin_url.as_ref(),
	)
	.await
	.unwrap();
	drop(test_node);
}

#[allow(clippy::too_many_arguments)]
async fn create_first_fission_and_check_constraints(
	bitcoind: &BitcoinD,
	network: Network,
	client: &Arc<MainchainClient>,
	bitcoin_owner_signer: &Sr25519Signer,
	bitcoin_owner_account_id: &AccountId32,
	utxo_id: UtxoId,
	fission_id: FissionId,
	liquid_id: LiquidId,
	fission_satoshis: Satoshis,
	microgons_at_target_per_btc: Balance,
) -> Balance {
	let liquidity_promised = create_fission(
		client,
		bitcoin_owner_signer,
		utxo_id,
		fission_id,
		liquid_id,
		fission_satoshis,
		microgons_at_target_per_btc,
	)
	.await
	.unwrap();

	let release_address = bitcoind
		.client
		.get_new_address(Some("blocked-release"), Some(AddressType::Bech32m))
		.unwrap()
		.require_network(network)
		.unwrap();
	let release_script: BitcoinScriptPubkey = release_address.script_pubkey().into();
	submit_rejected_bitcoin_call(
		client,
		bitcoin_owner_signer,
		RuntimeCall::BitcoinLocks(
			api::runtime_types::pallet_bitcoin_locks::pallet::Call::request_release {
				utxo_id,
				to_script_pubkey: release_script.into(),
				bitcoin_network_fee: 0,
			},
		),
		RuntimeError::BitcoinLocks(BitcoinLocksError::LockHasActiveFissions),
	)
	.await;
	let release_request = client
		.fetch_storage(
			&storage().bitcoin_locks().lock_release_requests_by_utxo_id(utxo_id),
			FetchAt::Best,
		)
		.await
		.unwrap();
	assert!(release_request.is_none());

	submit_rejected_bitcoin_call(
		client,
		bitcoin_owner_signer,
		RuntimeCall::BitcoinFissions(
			api::runtime_types::pallet_bitcoin_fissions::pallet::Call::ratchet {
				fission_id,
				microgons_at_target_per_btc,
			},
		),
		RuntimeError::BitcoinFissions(BitcoinFissionsError::NoRatchetingAvailable),
	)
	.await;
	let unchanged_fission = client
		.fetch_storage(
			&storage()
				.bitcoin_fissions()
				.fission_by_owner_and_id(bitcoin_owner_account_id.clone().into(), fission_id),
			FetchAt::Best,
		)
		.await
		.unwrap()
		.expect("active Fission");
	assert_eq!(unchanged_fission.ratchet_number, 0);
	assert_eq!(unchanged_fission.microgons_at_target_per_btc, microgons_at_target_per_btc);

	liquidity_promised
}

#[allow(clippy::too_many_arguments)]
async fn create_second_fission_and_check_allocation(
	bitcoin_owner: &sr25519::Pair,
	client: &Arc<MainchainClient>,
	bitcoin_owner_signer: &Sr25519Signer,
	utxo_id: UtxoId,
	first_fission_id: FissionId,
	fission_id: FissionId,
	liquid_id: LiquidId,
	fission_satoshis: Satoshis,
	microgons_at_target_per_btc: Balance,
	total_fissioned_satoshis: Satoshis,
	ticker: &Ticker,
	price_index_operator: &sr25519::Pair,
	last_submitted_tick: &mut Tick,
) -> anyhow::Result<()> {
	let liquidity_promised = create_fission(
		client,
		bitcoin_owner_signer,
		utxo_id,
		fission_id,
		liquid_id,
		fission_satoshis,
		microgons_at_target_per_btc,
	)
	.await?;

	wait_for_mint(
		bitcoin_owner,
		client,
		&utxo_id,
		fission_id,
		liquidity_promised,
		ticker,
		price_index_operator,
		last_submitted_tick,
	)
	.await?;

	let active_fission_ids = client
		.fetch_storage(&storage().bitcoin_fissions().fission_ids_by_lock_id(utxo_id), FetchAt::Best)
		.await?
		.expect("active Fission IDs");
	assert_eq!(
		active_fission_ids.0.into_iter().collect::<Vec<_>>(),
		vec![first_fission_id, fission_id]
	);

	let lock = client
		.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(utxo_id), FetchAt::Best)
		.await?
		.expect("funded Lock");
	assert_eq!(lock.fissioned_satoshis, total_fissioned_satoshis);

	Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn ratchet_first_fission(
	client: &Arc<MainchainClient>,
	ticker: &Ticker,
	price_index_operator: &sr25519::Pair,
	bitcoin_owner_signer: &Sr25519Signer,
	bitcoin_owner: &sr25519::Pair,
	bitcoin_owner_account_id: &AccountId32,
	utxo_id: UtxoId,
	fission_id: FissionId,
	securitized_satoshis: Satoshis,
	microgons_at_target_per_btc: Balance,
	initial_liquidity_promised: Balance,
	last_submitted_tick: &mut Tick,
) -> anyhow::Result<()> {
	// The mint payout loop can submit its 62k price during the current tick. PriceIndex ignores
	// another price at that tick, so advance before recording the older ratchet rate used below.
	while current_chain_tick(client, ticker).await <= *last_submitted_tick {
		sleep(Duration::from_millis(100)).await;
	}
	let submitted_older_ratchet_tick =
		submit_price(ticker, client, price_index_operator, 60_000.0).await;
	let rate_history = client
		.fetch_storage(&storage().bitcoin_locks().microgon_per_btc_history(), FetchAt::Best)
		.await?
		.expect("Bitcoin rate history");
	let (older_ratchet_tick, older_ratchet_rate) =
		*rate_history.0.last().expect("older Bitcoin rate");
	assert!(older_ratchet_tick >= submitted_older_ratchet_tick);
	assert!(older_ratchet_rate < microgons_at_target_per_btc);

	while current_chain_tick(client, ticker).await <= older_ratchet_tick {
		sleep(Duration::from_millis(100)).await;
	}

	*last_submitted_tick = submit_price(ticker, client, price_index_operator, 55_000.0).await;
	let rate_history = client
		.fetch_storage(&storage().bitcoin_locks().microgon_per_btc_history(), FetchAt::Best)
		.await?
		.expect("Bitcoin rate history");
	let (ratchet_tick, ratchet_rate) = *rate_history.0.last().expect("latest Bitcoin rate");
	assert!(ratchet_tick > older_ratchet_tick);
	assert!(ratchet_rate < older_ratchet_rate);

	submit_rejected_bitcoin_call(
		client,
		bitcoin_owner_signer,
		RuntimeCall::BitcoinLocks(
			api::runtime_types::pallet_bitcoin_locks::pallet::Call::resecuritize {
				utxo_id,
				satoshis: securitized_satoshis,
				options: Some(BitcoinLockOptions {
					microgons_at_target_per_btc: ratchet_rate,
					fee_coupon: None,
				}),
			},
		),
		RuntimeError::BitcoinLocks(BitcoinLocksError::InsufficientSecuritizationForFissions),
	)
	.await;
	let unchanged_lock = client
		.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(utxo_id), FetchAt::Best)
		.await?
		.expect("funded Lock");
	assert_eq!(unchanged_lock.securitized_satoshis, securitized_satoshis);
	assert_eq!(unchanged_lock.microgons_at_target_per_btc, microgons_at_target_per_btc);

	let params = client
		.params_with_best_nonce(&bitcoin_owner_signer.account_id())
		.await?
		.immortal()
		.build();
	let fission_ratchet_tx = client
		.submit_tx(
			&tx().bitcoin_fissions().ratchet(fission_id, ratchet_rate),
			bitcoin_owner_signer,
			Some(params),
			false,
		)
		.await?;
	let fission_ratcheted = fission_ratchet_tx
		.events
		.iter()
		.find_map(|event| {
			event.as_event::<api::bitcoin_fissions::events::FissionRatcheted>().transpose()
		})
		.transpose()?
		.expect("fission ratcheted event");
	assert_eq!(fission_ratcheted.fission_id, fission_id);
	assert_eq!(fission_ratcheted.ratchet_number, 1);

	let ratcheted_fission = client
		.fetch_storage(
			&storage()
				.bitcoin_fissions()
				.fission_by_owner_and_id(bitcoin_owner_account_id.clone().into(), fission_id),
			FetchAt::Best,
		)
		.await?
		.expect("active Fission");
	assert_eq!(ratcheted_fission.ratchet_number, 1);
	assert!(ratcheted_fission.last_ratchet_tick >= ratchet_tick);
	assert_eq!(ratcheted_fission.microgons_at_target_per_btc, ratchet_rate);
	assert!(ratcheted_fission.liquidity_promised < initial_liquidity_promised);

	submit_rejected_bitcoin_call(
		client,
		bitcoin_owner_signer,
		RuntimeCall::BitcoinFissions(
			api::runtime_types::pallet_bitcoin_fissions::pallet::Call::ratchet {
				fission_id,
				microgons_at_target_per_btc: older_ratchet_rate,
			},
		),
		RuntimeError::BitcoinFissions(
			BitcoinFissionsError::MicrogonsAtTargetPerBtcTickOlderThanCurrent,
		),
	)
	.await;
	let unchanged_fission = client
		.fetch_storage(
			&storage()
				.bitcoin_fissions()
				.fission_by_owner_and_id(bitcoin_owner_account_id.clone().into(), fission_id),
			FetchAt::Best,
		)
		.await?
		.expect("active Fission");
	assert_eq!(unchanged_fission.ratchet_number, 1);
	assert_eq!(unchanged_fission.last_ratchet_tick, ratcheted_fission.last_ratchet_tick);
	assert_eq!(unchanged_fission.microgons_at_target_per_btc, ratchet_rate);

	let pending_mint_indices = client
		.fetch_storage(&storage().mint().pending_mint_utxo_id_lookup(utxo_id), FetchAt::Best)
		.await?
		.expect("ratchet pending mint lookup");
	let pending_mint_index = *pending_mint_indices.0.last().expect("ratchet pending mint index");
	let pending_mint = client
		.fetch_storage(
			&storage().mint().pending_mint_utxos_by_index(pending_mint_index),
			FetchAt::Best,
		)
		.await?
		.expect("ratchet pending mint");
	assert_eq!(pending_mint.fission_id, fission_id);
	assert_eq!(pending_mint.account_id, bitcoin_owner_account_id.clone().into());
	assert!(pending_mint.remaining_amount > 0);
	assert!(pending_mint.remaining_amount <= ratcheted_fission.liquidity_promised);

	wait_for_mint(
		bitcoin_owner,
		client,
		&utxo_id,
		fission_id,
		ratcheted_fission.liquidity_promised,
		ticker,
		price_index_operator,
		last_submitted_tick,
	)
	.await
}

#[allow(clippy::too_many_arguments)]
async fn close_fissions(
	client: &Arc<MainchainClient>,
	bitcoin_owner_signer: &Sr25519Signer,
	bitcoin_owner_account_id: &AccountId32,
	utxo_id: UtxoId,
	fission_id: FissionId,
	second_fission_id: FissionId,
	second_fission_satoshis: Satoshis,
) {
	let params = client
		.params_with_best_nonce(&bitcoin_owner_signer.account_id())
		.await
		.unwrap()
		.immortal()
		.build();
	let fission_close_tx = client
		.submit_tx(
			&tx().bitcoin_fissions().close(fission_id),
			bitcoin_owner_signer,
			Some(params),
			false,
		)
		.await
		.unwrap();
	let fission_closed = fission_close_tx
		.events
		.iter()
		.find_map(|event| {
			event.as_event::<api::bitcoin_fissions::events::FissionClosed>().transpose()
		})
		.transpose()
		.unwrap()
		.expect("fission closed event");
	assert_eq!(fission_closed.fission_id, fission_id);

	let closed_fission = client
		.fetch_storage(
			&storage()
				.bitcoin_fissions()
				.fission_by_owner_and_id(bitcoin_owner_account_id.clone().into(), fission_id),
			FetchAt::Best,
		)
		.await
		.unwrap();
	assert!(closed_fission.is_none());
	let fission_ids = client
		.fetch_storage(&storage().bitcoin_fissions().fission_ids_by_lock_id(utxo_id), FetchAt::Best)
		.await
		.unwrap()
		.expect("second Fission remains active");
	assert_eq!(fission_ids.0.into_iter().collect::<Vec<_>>(), vec![second_fission_id]);
	let lock = client
		.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(utxo_id), FetchAt::Best)
		.await
		.unwrap()
		.expect("Lock remains active after its Fission closes");
	assert_eq!(lock.fissioned_satoshis, second_fission_satoshis);

	submit_rejected_bitcoin_call(
		client,
		bitcoin_owner_signer,
		RuntimeCall::BitcoinFissions(
			api::runtime_types::pallet_bitcoin_fissions::pallet::Call::close { fission_id },
		),
		RuntimeError::BitcoinFissions(BitcoinFissionsError::FissionNotFound),
	)
	.await;

	let params = client
		.params_with_best_nonce(&bitcoin_owner_signer.account_id())
		.await
		.unwrap()
		.immortal()
		.build();
	client
		.submit_tx(
			&tx().bitcoin_fissions().close(second_fission_id),
			bitcoin_owner_signer,
			Some(params),
			false,
		)
		.await
		.unwrap();
	let second_closed_fission = client
		.fetch_storage(
			&storage().bitcoin_fissions().fission_by_owner_and_id(
				bitcoin_owner_account_id.clone().into(),
				second_fission_id,
			),
			FetchAt::Best,
		)
		.await
		.unwrap();
	assert!(second_closed_fission.is_none());
	let fission_ids = client
		.fetch_storage(&storage().bitcoin_fissions().fission_ids_by_lock_id(utxo_id), FetchAt::Best)
		.await
		.unwrap();
	assert!(fission_ids.map(|ids| ids.0.is_empty()).unwrap_or(true));
	let lock = client
		.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(utxo_id), FetchAt::Best)
		.await
		.unwrap()
		.expect("Lock remains active after all Fissions close");
	assert_eq!(lock.fissioned_satoshis, 0);
}

#[allow(clippy::too_many_arguments)]
async fn create_fission(
	client: &MainchainClient,
	bitcoin_owner_signer: &Sr25519Signer,
	utxo_id: UtxoId,
	fission_id: FissionId,
	liquid_id: LiquidId,
	satoshis: Satoshis,
	microgons_at_target_per_btc: Balance,
) -> anyhow::Result<Balance> {
	let params = client
		.params_with_best_nonce(&bitcoin_owner_signer.account_id())
		.await?
		.immortal()
		.build();
	let result = client
		.submit_tx(
			&tx().bitcoin_fissions().create(
				fission_id,
				liquid_id,
				utxo_id,
				satoshis,
				microgons_at_target_per_btc,
			),
			bitcoin_owner_signer,
			Some(params),
			false,
		)
		.await?;
	let created = result
		.events
		.iter()
		.find_map(|event| {
			event.as_event::<api::bitcoin_fissions::events::FissionCreated>().transpose()
		})
		.transpose()?
		.expect("fission created event");
	assert_eq!(created.fission_id, fission_id);
	assert_eq!(created.liquid_id, liquid_id);

	Ok(created.liquidity_promised)
}

async fn speed_up_minting(
	test_node: &ArgonTestNode,
	client: &MainchainClient,
) -> (MiningSlotConfig, Tick) {
	let mining_config = client
		.fetch_storage(&storage().mining_slot().mining_config(), FetchAt::Best)
		.await
		.unwrap()
		.expect("mining config");
	let original_mining_config = MiningSlotConfig {
		ticks_between_slots: mining_config.ticks_between_slots,
		ticks_before_bid_end_for_vrf_close: mining_config.ticks_before_bid_end_for_vrf_close,
		slot_bidding_start_after_ticks: mining_config.slot_bidding_start_after_ticks,
	};
	let original_frame_reward_ticks_remaining = client
		.fetch_storage(&storage().mining_slot().frame_reward_ticks_remaining(), FetchAt::Best)
		.await
		.unwrap()
		.expect("frame reward ticks remaining");
	let accelerated_mining_config =
		MiningSlotConfig { ticks_between_slots: 1, ..original_mining_config.clone() };

	sudo(
		test_node,
		RuntimeCall::System(
			argon_client::api::runtime_types::frame_system::pallet::Call::set_storage {
				items: vec![
					(
						storage().mint().minted_mining_microgons().to_root_bytes(),
						Balance::from(100_000_000_000u64).encode(),
					),
					(
						storage().mining_slot().mining_config().to_root_bytes(),
						accelerated_mining_config.encode(),
					),
					(
						storage().mining_slot().frame_reward_ticks_remaining().to_root_bytes(),
						1u64.encode(),
					),
				],
			},
		),
		false,
	)
	.await
	.unwrap();

	(original_mining_config, original_frame_reward_ticks_remaining)
}

async fn restore_mining_cadence(
	test_node: &ArgonTestNode,
	(original_mining_config, original_frame_reward_ticks_remaining): (MiningSlotConfig, Tick),
) {
	// Release and cosign deadlines are frame-based, so restore normal cadence before handing the
	// request to the Vault and wait until the restoration is finalized.
	sudo(
		test_node,
		RuntimeCall::System(
			argon_client::api::runtime_types::frame_system::pallet::Call::set_storage {
				items: vec![
					(
						storage().mining_slot().mining_config().to_root_bytes(),
						original_mining_config.encode(),
					),
					(
						storage().mining_slot().frame_reward_ticks_remaining().to_root_bytes(),
						original_frame_reward_ticks_remaining.encode(),
					),
				],
			},
		),
		true,
	)
	.await
	.unwrap();
}

async fn submit_rejected_bitcoin_call(
	client: &MainchainClient,
	signer: &Sr25519Signer,
	call: RuntimeCall,
	expected_error: RuntimeError,
) {
	let params = client
		.params_with_best_nonce(&signer.account_id())
		.await
		.unwrap()
		.immortal()
		.build();
	let result = client
		.submit_tx(&tx().utility().force_batch(vec![call]), signer, Some(params), false)
		.await
		.unwrap();
	let failed = result
		.events
		.iter()
		.find_map(|event| event.as_event::<api::utility::events::ItemFailed>().transpose())
		.transpose()
		.unwrap()
		.expect("Bitcoin call unexpectedly succeeded");

	let metadata = client.live.metadata();
	let dispatch_error_type = metadata.dispatch_error_ty().expect("DispatchError metadata");
	let encoded_dispatch_error = failed
		.error
		.encode_as_type(dispatch_error_type, metadata.types())
		.expect("DispatchError should encode against runtime metadata");
	let actual_error =
		subxt_error::DispatchError::decode_from(encoded_dispatch_error, metadata.clone())
			.expect("ItemFailed should contain a valid DispatchError");
	let subxt_error::DispatchError::Module(actual_error) = actual_error else {
		panic!("expected {expected_error:?}, got {actual_error:?}");
	};
	let actual_error = actual_error
		.as_root_error::<RuntimeError>()
		.expect("module error should decode as a runtime error");
	let runtime_error_type = metadata.outer_enums().error_enum_ty();
	let actual_error_bytes = actual_error
		.encode_as_type(runtime_error_type, metadata.types())
		.expect("actual runtime error should encode");
	let expected_error_bytes = expected_error
		.encode_as_type(runtime_error_type, metadata.types())
		.expect("expected runtime error should encode");

	assert_eq!(
		actual_error_bytes, expected_error_bytes,
		"expected {expected_error:?}, got {actual_error:?}"
	);
}

async fn submit_price(
	ticker: &Ticker,
	client: &MainchainClient,
	price_index_operator: &sr25519::Pair,
	btc_usd_price: f64,
) -> Tick {
	let signer = Sr25519Signer::new(price_index_operator.clone());
	let account_id = signer.account_id();

	// The two-node mining flow can invalidate a price transaction while switching forks.
	// Refresh both the tick and nonce once before failing the test.
	for attempt in 0..2 {
		let tick = current_chain_tick(client, ticker).await;
		let nonce = client.get_account_nonce(&account_id).await.unwrap();
		let params = MainchainClient::ext_params_builder().nonce(nonce.into()).immortal().build();
		let progress = client
			.live
			.tx()
			.sign_and_submit_then_watch(
				&tx().price_index().submit(
					Index {
						btc_usd_price: FixedU128Ext(
							FixedU128::from_float(btc_usd_price).into_inner(),
						),
						argon_usd_target_price: FixedU128Ext(
							FixedU128::from_float(1.0).into_inner(),
						),
						argon_usd_price: FixedU128Ext(FixedU128::from_float(1.6).into_inner()),
						argon_time_weighted_average_liquidity: 500_000_000_000,
						argonot_usd_price: FixedU128Ext(FixedU128::from_float(1.0).into_inner()),
						tick,
					},
					None,
				),
				&signer,
				params,
			)
			.await
			.unwrap();

		match MainchainClient::wait_for_ext_in_block(progress, false).await {
			Ok(_) => {
				println!("bitcoin prices submitted at tick {tick}");
				return tick;
			},
			Err(subxt_error::Error::Other(message))
				if attempt == 0 && message.contains("Transaction is invalid") =>
			{
				println!("Bitcoin price transaction became invalid; retrying with a fresh nonce");
			},
			Err(error) => panic!("Could not submit bitcoin price: {error}"),
		}
	}

	unreachable!("bitcoin price submission either succeeds or panics")
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

	*last_submitted_tick = submit_price(ticker, client, price_index_operator, 62_000.0).await;
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
		bitcoin_xpubkey: xpubkey.encode().into(),
		terms: api::runtime_types::argon_primitives::vault::VaultTerms::<u128> {
			bitcoin_base_fee: 0,
			bitcoin_annual_percent_rate: to_api_fixed_u128(FixedU128::from_float(0.01)),
			treasury_profit_sharing: to_api_per_mill(Permill::from_percent(50)),
		},
		delegate_account_id: None,
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

async fn create_receive_address(
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

	println!("Owner creates a Bitcoin receive address with pubkey: {owner_compressed_pubkey}");
	let owner_pubkey: CompressedBitcoinPubkey = (*owner_compressed_pubkey).into();

	let receive_address_tx = test_node
		.client
		.submit_tx(
			&tx().bitcoin_locks().create_receive_address(
				vault_id,
				satoshis,
				owner_pubkey.into(),
				None,
			),
			&Sr25519Signer::new(bitcoin_owner.clone()),
			None,
			true,
		)
		.await?;
	println!("Bitcoin receive address created for Lock");
	let lock_created = receive_address_tx
		.events
		.iter()
		.find_map(|event| {
			event.as_event::<api::bitcoin_locks::events::BitcoinLockCreated>().transpose()
		})
		.transpose()?
		.expect("lock event");
	let utxo_id = lock_created.utxo_id;
	Ok(utxo_id)
}

#[allow(clippy::too_many_arguments)]
async fn verify_lock(
	secp: &Secp256k1<All>,
	owner_compressed_pubkey: PublicKey,
	utxo_satoshis: Satoshis,
	client: &Arc<MainchainClient>,
	xpubkey: &Xpub,
	bitcoin_network: Network,
	vault_id: VaultId,
	utxo_id: &UtxoId,
) -> anyhow::Result<(BitcoinCosignScriptPubkey, Balance)> {
	let lock = client
		.fetch_storage(&storage().bitcoin_locks().locks_by_utxo_id(*utxo_id), FetchAt::Finalized)
		.await?
		.expect("should be able to retrieve");
	assert_eq!(lock.vault_id, vault_id);
	{
		assert_eq!(lock.securitized_satoshis, utxo_satoshis);
		assert_eq!(lock.owner_pubkey.0, owner_compressed_pubkey.inner.serialize());
		assert_eq!(lock.vault_xpub_sources.0, xpubkey.fingerprint().to_bytes());
		assert_eq!(lock.vault_xpub_sources.1, Into::<u32>::into(ChildNumber::from_normal_idx(1)?));
		assert_eq!(
			lock.vault_pubkey.0,
			xpubkey
				.derive_pub(secp, &DerivationPath::from_str("1")?)?
				.public_key
				.serialize()
		);
		let cosign_script = CosignScript::new(
			CosignScriptArgs {
				vault_pubkey: lock.vault_pubkey.clone().into(),
				owner_pubkey: lock.owner_pubkey.into(),
				vault_claim_pubkey: lock.vault_claim_pubkey.into(),
				vault_claim_height: lock.vault_claim_height,
				open_claim_height: lock.open_claim_height,
				created_at_height: lock.created_at_height,
			},
			bitcoin_network,
		)
		.map_err(|_| anyhow!("Unable to create a script"))?;
		let cosign_key = cosign_script.script.to_p2wsh();
		let cosign_script_pubkey: BitcoinCosignScriptPubkey =
			cosign_key.try_into().map_err(|_| anyhow!("Unable to convert script pubkey"))?;
		assert_eq!(cosign_script_pubkey, lock.utxo_script_pubkey.clone().into());
	}

	assert_eq!(lock.funded_satoshis, 0);
	assert_eq!(lock.fissioned_satoshis, 0);
	Ok((lock.utxo_script_pubkey.into(), lock.microgons_at_target_per_btc))
}

async fn wait_for_lock_funding(
	client: &Arc<MainchainClient>,
	utxo_id: UtxoId,
	funded_satoshis: Satoshis,
	txid: Txid,
	vout: u32,
) -> anyhow::Result<()> {
	let mut finalized_sub = client.live.blocks().subscribe_finalized().await?;
	let mut finalized_hash = client.latest_finalized_block_hash().await?.hash();

	for remaining_blocks in (1..=100).rev() {
		let lock = client
			.fetch_storage(
				&storage().bitcoin_locks().locks_by_utxo_id(utxo_id),
				FetchAt::Block(finalized_hash),
			)
			.await?
			.expect("Lock state should be present");
		if lock.funded_satoshis == funded_satoshis {
			break;
		}

		let block = finalized_sub
			.next()
			.await
			.ok_or_else(|| anyhow!("Stopped waiting for Lock {utxo_id} funding"))??;
		finalized_hash = block.hash();
		println!("Waiting for Lock {utxo_id} funding in block {:?}", block.hash());
		if remaining_blocks == 1 {
			panic!("Lock {utxo_id} was not funded after 100 blocks");
		}
	}

	let utxo_ref = client
		.fetch_storage(
			&storage().bitcoin_locks().utxo_id_to_funding_utxo_ref(utxo_id),
			FetchAt::Block(finalized_hash),
		)
		.await?
		.expect("funding UTXO");
	assert_eq!(utxo_ref.txid.0, txid.to_byte_array());
	assert_eq!(utxo_ref.output_index, vout);

	Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn wait_for_mint(
	bitcoin_owner: &sr25519::Pair,
	client: &Arc<MainchainClient>,
	utxo_id: &UtxoId,
	fission_id: FissionId,
	liquidity_promised: Balance,
	ticker: &Ticker,
	price_index_operator: &sr25519::Pair,
	last_submitted_tick: &mut Tick,
) -> anyhow::Result<()> {
	let mut best_block_sub = client.live.blocks().subscribe_best().await?;
	let pending_mint_index = client
		.fetch_storage(&storage().mint().pending_mint_utxo_id_lookup(*utxo_id), FetchAt::Best)
		.await?
		.and_then(|lookup| lookup.0.first().copied());
	let pending_mint = if let Some(pending_mint_index) = pending_mint_index {
		client
			.fetch_storage(
				&storage().mint().pending_mint_utxos_by_index(pending_mint_index),
				FetchAt::Best,
			)
			.await?
	} else {
		None
	};

	let owner_account_id32: AccountId32 = bitcoin_owner.clone().public().into();
	let owner_account_id = owner_account_id32.clone().into();
	let Some(mut pending_mint) = pending_mint else {
		let balance = client.get_argons(&owner_account_id32).await.expect("pending mint balance");
		assert!(balance.free >= liquidity_promised);
		return Ok(())
	};
	assert_eq!(pending_mint.account_id, owner_account_id);
	assert_eq!(pending_mint.fission_id, fission_id);

	let mut last_remaining_amount = pending_mint.remaining_amount;
	let mut highest_frame_id = client
		.fetch_storage(&storage().mining_slot().next_frame_id(), FetchAt::Best)
		.await?
		.unwrap_or_default();
	let mut frames_waited = 0u64;
	let deadline = tokio::time::Instant::now() + Duration::from_secs(90);

	loop {
		let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
		let block = tokio::time::timeout(remaining, best_block_sub.next())
			.await
			.map_err(|_| anyhow!("Timed out waiting for pending mint payout"))?
			.ok_or_else(|| anyhow!("Best block subscription ended during pending mint payout"))??;
		let fetch_at = FetchAt::Block(block.hash());

		let frame_id = client
			.fetch_storage(&storage().mining_slot().next_frame_id(), fetch_at)
			.await?
			.unwrap_or_default();
		if frame_id > highest_frame_id {
			frames_waited = frames_waited.saturating_add(frame_id - highest_frame_id);
			highest_frame_id = frame_id;
		}

		let pending_mint_index = client
			.fetch_storage(&storage().mint().pending_mint_utxo_id_lookup(*utxo_id), fetch_at)
			.await?
			.and_then(|lookup| lookup.0.first().copied());
		let Some(pending_mint_index) = pending_mint_index else { break };
		pending_mint = client
			.fetch_storage(
				&storage().mint().pending_mint_utxos_by_index(pending_mint_index),
				fetch_at,
			)
			.await?
			.ok_or_else(|| anyhow!("Pending mint lookup referenced a missing mint"))?;
		assert_eq!(pending_mint.account_id, owner_account_id);
		assert_eq!(pending_mint.fission_id, fission_id);

		if pending_mint.remaining_amount < last_remaining_amount {
			last_remaining_amount = pending_mint.remaining_amount;
			println!("Owner mint pending remaining = {last_remaining_amount}");
		}
		submit_price_if_needed(ticker, client, price_index_operator, last_submitted_tick).await;

		if frames_waited >= 15 {
			let bitcoin_minted = client
				.fetch_storage(&storage().mint().minted_bitcoin_microgons(), FetchAt::Best)
				.await?
				.expect("Bitcoin minted amount");
			let mining_minted = client
				.fetch_storage(&storage().mint().minted_mining_microgons(), FetchAt::Best)
				.await?
				.expect("mining minted amount");
			anyhow::bail!(
				"Pending mint did not complete after {frames_waited} frames. Last mint: {pending_mint:?}. Mining minted: {mining_minted}. Bitcoin minted: {bitcoin_minted}"
			);
		}
	}

	let balance = client.get_argons(&owner_account_id32).await.expect("completed mint balance");
	assert!(balance.free >= liquidity_promised);
	println!("Owner minted full Fission liquidity");
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
	// The runtime derives each lock pubkey from the uploaded vault xpub, so we need to sign from
	// that xpub root and then derive the per-lock child number stored on the lock.
	let uploaded_vault_xpriv = vault_xpriv
		.derive_priv(&Secp256k1::new(), &DerivationPath::from_str(uploaded_xpub_hd_path)?)?;
	let vault_hd_path = DerivationPath::from(vec![ChildNumber::from(lock.vault_xpub_sources.1)]);
	let (signature, _) = releaser.sign_derived(uploaded_vault_xpriv, vault_hd_path)?;
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
		.fetch_storage(&storage().bitcoin_locks().utxo_id_to_funding_utxo_ref(utxo_id), at_block)
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
		lock.funded_satoshis,
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
