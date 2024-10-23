use argon_localchain::cli;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  cli::run(env::args_os()).await
}
