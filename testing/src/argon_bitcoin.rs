use std::{env, process, process::Command};

use crate::{get_target_dir, ArgonTestNode};

pub async fn run_bitcoin_cli(
	node: &ArgonTestNode,
	args: Vec<impl ToString>,
) -> anyhow::Result<String> {
	let rust_log = env::var("RUST_LOG").unwrap_or("info".to_string());

	let target_dir = get_target_dir();

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
	let stdout = String::from_utf8_lossy(&output.stdout);
	println!("{}", stdout);
	if output.status.success() {
		Ok(stdout.to_string())
	} else {
		// Print the error
		let stderr = String::from_utf8_lossy(&output.stderr);
		Err(anyhow::anyhow!("Failed to run argon-bitcoin: {:?}", stderr))
	}
}
