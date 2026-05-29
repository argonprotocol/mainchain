use crate::{get_target_dir, ArgonTestNode};
use std::{env, process::Stdio};
use tokio::{
	io::{AsyncBufReadExt, BufReader},
	process::Command,
};

pub async fn run_bitcoin_cli(
	node: &ArgonTestNode,
	args: Vec<impl ToString>,
) -> anyhow::Result<String> {
	let rust_log = env::var("RUST_LOG").unwrap_or("info".to_string());

	let target_dir = get_target_dir();
	println!("CLI      {}", args.iter().map(|a| a.to_string()).collect::<Vec<String>>().join(" "));

	let mut child = Command::new("./argon-bitcoin-cli")
		.current_dir(&target_dir)
		.env("RUST_LOG", rust_log)
		.args(
			[
				&args.into_iter().map(|a| a.to_string()).collect::<Vec<String>>()[..],
				&["--trusted-rpc-url".to_string(), node.client.url.to_string()][..],
			]
			.concat()
			.into_iter(),
		)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?;

	let stdout = child.stdout.take().unwrap();
	let stderr = child.stderr.take().unwrap();

	let mut stdout_reader = BufReader::new(stdout).lines();
	let mut stderr_reader = BufReader::new(stderr).lines();

	let mut out_log = String::new();
	let mut err_log = String::new();
	let mut stdout_done = false;
	let mut stderr_done = false;

	while !stdout_done || !stderr_done {
		tokio::select! {
			line = stdout_reader.next_line(), if !stdout_done => {
				match line? {
					Some(line) => {
						println!("CLI      {line}");
						out_log.push_str(&line);
						out_log.push('\n');
					},
					None => stdout_done = true,
				}
			},
			line = stderr_reader.next_line(), if !stderr_done => {
				match line? {
					Some(line) => {
						eprintln!("CLI      {line}");
						err_log.push_str(&line);
						err_log.push('\n');
					},
					None => stderr_done = true,
				}
			},
		}
	}

	let status = child.wait().await?;

	if status.success() {
		Ok(out_log)
	} else {
		Err(anyhow::anyhow!("Failed to run argon-bitcoin: {err_log:?}"))
	}
}
