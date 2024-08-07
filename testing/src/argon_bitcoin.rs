use std::{env, path::PathBuf, process, process::Command};

use crate::ArgonTestNode;

pub async fn run_bitcoin_cli(
	node: &ArgonTestNode,
	args: Vec<impl ToString>,
) -> anyhow::Result<String> {
	let rust_log = env::var("RUST_LOG").unwrap_or("info".to_string());

	let target_dir = {
		let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
		let workspace_cargo_path = project_dir.join("..");
		let workspace_cargo_path =
			workspace_cargo_path.canonicalize().expect("Failed to canonicalize path");
		let workspace_cargo_path = workspace_cargo_path.as_path().join("target/debug");
		workspace_cargo_path
	};

	let output = Command::new("./argon-bitcoin-cli")
		.current_dir(&target_dir)
		.env("RUST_LOG", rust_log)
		.stdout(process::Stdio::piped())
		.args(
			[
				&args.into_iter().map(|a| a.to_string()).collect::<Vec<String>>()[..],
				&["--trusted-rpc-url".to_string(), node.client.url.to_string()][..],
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
		Err(anyhow::anyhow!("Failed to run argon-bitcoin: {:?}", stderr))
	}
}
