use std::{
	env,
	io::{BufRead, BufReader},
	path::PathBuf,
	process,
	process::Command,
	sync::{
		mpsc,
		mpsc::{Receiver, Sender},
	},
	thread,
};

use crate::UlxTestNode;

pub struct UlxTestOracle {
	// Keep a handle to the node; once it's dropped the node is killed.
	proc: Option<process::Child>,
}

impl Drop for UlxTestOracle {
	fn drop(&mut self) {
		if let Some(mut proc) = self.proc.take() {
			let _ = proc.kill();
		}
	}
}

impl UlxTestOracle {
	pub async fn bitcoin_tip(node: &UlxTestNode) -> anyhow::Result<Self> {
		let rust_log = env::var("RUST_LOG").unwrap_or("info".to_string());

		let target_dir = {
			let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
			let workspace_cargo_path = project_dir.join("..");
			let workspace_cargo_path =
				workspace_cargo_path.canonicalize().expect("Failed to canonicalize path");
			let workspace_cargo_path = workspace_cargo_path.as_path().join("target/debug");
			workspace_cargo_path
		};
		println!("run from {}", target_dir.to_str().unwrap_or(""));

		let mut proc = Command::new("./ulx-oracle")
			.current_dir(&target_dir)
			.env("RUST_LOG", rust_log)
			.stdout(process::Stdio::piped())
			.args(vec![
				"--dev",
				"-t",
				&node.client.url,
				"bitcoin",
				"--bitcoin-rpc-url",
				&node.bitcoin_rpc_url.clone().unwrap().as_str(),
			])
			.spawn()?;

		// Wait for RPC port to be logged (it's logged to stderr).
		let stdout = proc.stdout.take().unwrap();
		let stdout_reader = BufReader::new(stdout);
		let (tx, rx): (Sender<()>, Receiver<()>) = mpsc::channel();

		let tx_clone = tx.clone();

		thread::spawn(move || {
			for line in stdout_reader.lines() {
				let line = line.expect("failed to obtain next line from stdout");
				println!("{}", line);
				if line.contains("Oracle Started.") {
					tx_clone.send(()).unwrap();
				}
			}
		});

		rx.recv().expect("Failed to start oracle");

		Ok(Self { proc: Some(proc) })
	}
}
