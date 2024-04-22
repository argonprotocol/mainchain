use std::env;
use ulx_localchain::cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::FmtSubscriber::builder()
    .with_env_filter(tracing_subscriber::EnvFilter::from_env("DEBUG"))
    .try_init()
    .expect("setting default subscriber failed");

  cli::run(env::args_os()).await
}
