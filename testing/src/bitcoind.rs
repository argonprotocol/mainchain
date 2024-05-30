use std::{env, fs::File};

use anyhow::anyhow;
use bitcoind::{downloaded_exe_path, BitcoinD, Conf};
use fs2::FileExt;
use lazy_static::lazy_static;

lazy_static! {
	static ref BITCOIND_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}

pub fn start_bitcoind() -> anyhow::Result<(BitcoinD, url::Url)> {
	let path = env::temp_dir().join("ulx_bitcoind_testing.lock");
	let file = File::create_new(&path).or_else(|_| File::open(&path))?;
	// Acquire the lock
	file.lock_exclusive().expect("Failed to acquire lock");

	let _ = env_logger::builder().is_test(true).try_init();

	let mut conf = Conf::default();
	conf.args.push("-blockfilterindex");
	conf.args.push("-txindex");

	let lock = BITCOIND_LOCK.lock().unwrap();
	println!("Bitcoin path {}", downloaded_exe_path().unwrap());
	let bitcoind = match BitcoinD::with_conf(downloaded_exe_path().unwrap(), &conf) {
		Ok(bitcoind) => bitcoind,
		Err(e) => {
			file.unlock().expect("Failed to unlock file");
			eprintln!("Failed to start bitcoind: {:#?}", e);
			return Err(anyhow!("Failed to start bitcoind: {:#?}", e));
		},
	};
	drop(lock);
	file.unlock().expect("Failed to unlock file");
	let rpc_url = bitcoind.params.rpc_socket.to_string();
	let cookie = bitcoind.params.get_cookie_values().expect("cookie");

	let mut url = url::Url::parse(&format!("http://{rpc_url}"))?;
	if let Some(cookie) = cookie {
		url.set_username(&cookie.user).expect("username");
		url.set_password(Some(&cookie.password)).expect("password");
	}
	Ok((bitcoind, url))
}
