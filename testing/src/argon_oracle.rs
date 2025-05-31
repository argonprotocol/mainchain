use std::{
	env,
	io::{BufRead, BufReader},
	process,
	process::Command,
	sync::{
		mpsc,
		mpsc::{Receiver, Sender},
	},
};

use strip_ansi_escapes::strip;
use tokio::task::spawn_blocking;

use crate::{ArgonTestNode, get_target_dir};

pub struct ArgonTestOracle {
	// Keep a handle to the node; once it's dropped the node is killed.
	proc: Option<process::Child>,
}

impl Drop for ArgonTestOracle {
	fn drop(&mut self) {
		if let Some(mut proc) = self.proc.take() {
			let _ = proc.kill();
		}
	}
}

impl ArgonTestOracle {
	pub async fn bitcoin_tip(node: &ArgonTestNode) -> anyhow::Result<Self> {
		let mut rust_log = env::var("RUST_LOG").unwrap_or("info".to_string());

		if !rust_log.contains("argon") {
			rust_log.push_str(",argon=info");
		}

		let target_dir = get_target_dir();

		let mut proc = Command::new("./argon-oracle")
			.current_dir(&target_dir)
			.env("RUST_LOG", rust_log)
			.stdout(process::Stdio::piped())
			.args(vec![
				"--dev",
				"-t",
				&node.client.url,
				"bitcoin",
				"--bitcoin-rpc-url",
				&node.start_args.bitcoin_rpc_url().unwrap().as_str(),
			])
			.spawn()?;

		// Wait for RPC port to be logged (it's logged to stderr).
		let stdout = proc.stdout.take().unwrap();
		let stdout_reader = BufReader::new(stdout);
		let (tx, rx): (Sender<()>, Receiver<()>) = mpsc::channel();

		let tx_clone = tx.clone();

		spawn_blocking(move || {
			for line in stdout_reader.lines() {
				let line = line.expect("failed to obtain next line from stdout");
				let cleaned_log = strip(&line);
				println!("ORACLE>> {}", String::from_utf8_lossy(&cleaned_log));

				if line.contains("Oracle Started.") {
					tx_clone.send(()).unwrap();
				}
			}
		});

		rx.recv().expect("Failed to start oracle");

		Ok(Self { proc: Some(proc) })
	}
}
