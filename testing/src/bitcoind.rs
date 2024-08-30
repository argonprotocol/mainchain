use std::{env, fs::File, str::FromStr};

use anyhow::anyhow;
use bitcoind::{downloaded_exe_path, BitcoinD, Conf};
use fs2::FileExt;
use lazy_static::lazy_static;
use rand::{rngs::OsRng, RngCore};

use argon_primitives::bitcoin::Satoshis;
use bitcoin::{
	bip32::{DerivationPath, Fingerprint, Xpriv, Xpub},
	secp256k1::Secp256k1,
	Address, Amount, CompressedPublicKey, Network, Txid,
};
use bitcoincore_rpc::{json::GetRawTransactionResult, RpcApi};

lazy_static! {
	static ref BITCOIND_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}

pub fn start_bitcoind() -> anyhow::Result<(BitcoinD, url::Url, bitcoin::Network)> {
	let lock = BITCOIND_LOCK.lock().unwrap();
	let path = env::temp_dir().join("argon_bitcoind_testing.lock");
	let file = File::create_new(&path).or_else(|_| File::open(&path))?;
	// Acquire the lock
	file.lock_exclusive().expect("Failed to acquire lock");

	let _ = env_logger::builder().is_test(true).try_init();

	let mut conf = Conf::default();
	conf.args.push("-blockfilterindex");
	conf.args.push("-txindex");

	println!("Bitcoin path {}", downloaded_exe_path().unwrap());
	let bitcoind = match BitcoinD::with_conf(downloaded_exe_path().unwrap(), &conf) {
		Ok(bitcoind) => bitcoind,
		Err(e) => {
			file.unlock().expect("Failed to unlock file");
			eprintln!("Failed to start bitcoind: {:#?}", e);
			return Err(anyhow!("Failed to start bitcoind: {:#?}", e));
		},
	};
	let url = read_rpc_url(&bitcoind)?;
	file.unlock().expect("Failed to unlock file");
	drop(lock);
	let network = bitcoind.client.get_blockchain_info().unwrap().chain;
	Ok((bitcoind, url, network))
}

pub fn read_rpc_url(bitcoind: &BitcoinD) -> anyhow::Result<url::Url> {
	let rpc_url = bitcoind.params.rpc_socket.to_string();
	let cookie = bitcoind.params.get_cookie_values().expect("cookie");

	let mut url = url::Url::parse(&format!("http://{rpc_url}"))?;
	if let Some(cookie) = cookie {
		url.set_username(&cookie.user).expect("username");
		url.set_password(Some(&cookie.password)).expect("password");
	}
	Ok(url)
}

pub fn wait_for_txid(
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

pub fn create_xpriv(network: Network) -> (Xpriv, Fingerprint) {
	let secp = Secp256k1::new();
	let mut seed = [0u8; 32];
	OsRng.fill_bytes(&mut seed);
	let master_xpriv = Xpriv::new_master(network, &seed).unwrap();
	let master_xpub = Xpub::from_priv(&secp, &master_xpriv);
	let fingerprint = master_xpub.fingerprint();
	(master_xpriv, fingerprint)
}

pub fn derive(master_xpriv: &Xpriv, path: &str) -> (CompressedPublicKey, DerivationPath) {
	let secp = Secp256k1::new();
	let path = DerivationPath::from_str(path).expect("Invalid derivation path");
	let child_xpriv = master_xpriv.derive_priv(&secp, &path).expect("Unable to derive child key");

	let vault_privkey = child_xpriv.to_priv();
	let vault_compressed_pubkey = CompressedPublicKey::from_private_key(&secp, &vault_privkey)
		.expect("Unable to derive pubkey");
	(vault_compressed_pubkey, path)
}

pub fn add_wallet_address(bitcoind: &BitcoinD) -> Address {
	let address = bitcoind.client.get_new_address(None, None).unwrap();
	let network = bitcoind.client.get_blockchain_info().unwrap().chain;
	address.require_network(network).unwrap()
}

pub fn add_blocks(bitcoind: &BitcoinD, count: u64, grant_to_address: &Address) {
	// NOTE: if you get errors here on a Mac M1/M*, you may need to run `brew install sqlite`.
	// There's a weird history of sqlite having massive slowdowns with bitcoind. It will use
	// built-in homebrew version if available
	bitcoind.client.generate_to_address(count, grant_to_address).unwrap();
}

pub fn fund_script_address(
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
