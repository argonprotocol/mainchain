use std::{env, path::PathBuf, process, process::Command};

use crate::{ArgonTestNode, get_target_dir};

pub struct LocalchainCli {
	pub tmp_dir: PathBuf,
}

impl LocalchainCli {
	pub fn new(test_id: &str) -> Self {
		let tmp_dir = env::temp_dir().join(test_id).join("localchain");
		if tmp_dir.exists() {
			std::fs::remove_dir_all(&tmp_dir).expect("Failed to remove test dir");
		}
		Self { tmp_dir }
	}

	pub async fn run(
		&self,
		node: &ArgonTestNode,
		args: Vec<impl ToString>,
	) -> anyhow::Result<String> {
		let rust_log = env::var("RUST_LOG").unwrap_or("warn".to_string());

		let target_dir = get_target_dir();

		let tmp_dir = self.tmp_dir.clone();

		let output = Command::new("./argon-localchain")
			.current_dir(&target_dir)
			.env("RUST_LOG", rust_log)
			.stdout(process::Stdio::piped())
			.args(
				[
					&args.into_iter().map(|a| a.to_string()).collect::<Vec<String>>()[..],
					&[
						"--mainchain-url".to_string(),
						node.client.url.to_string(),
						"--base-dir".to_string(),
						tmp_dir.to_string_lossy().to_string(),
					][..],
				]
				.concat()
				.into_iter(),
			)
			.output()?;

		if output.status.success() {
			// Convert the output to a string and print it
			let stdout = String::from_utf8_lossy(&output.stdout);
			Ok(stdout.to_string())
		} else {
			// Print the error
			let stderr = String::from_utf8_lossy(&output.stderr);
			Err(anyhow::anyhow!("Failed to run argon-localchain: {:?}", stderr))
		}
	}
}
