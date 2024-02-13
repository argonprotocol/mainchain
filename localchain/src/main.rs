use std::{env, path::PathBuf};

use clap::{crate_version, Parser};
use sc_cli::KeystoreParams;
use sc_keystore::LocalKeystore;
use sc_service::config::KeystoreConfig;
use sc_service::BasePath;
use sp_keystore::KeystorePtr;
use tokio::time;

use ulx_localchain::signer::Signer;
use ulx_localchain::{EscrowCloseOptions, Localchain, LocalchainConfig};
use ulx_primitives::NotaryId;

#[derive(Parser, Debug)]
#[clap(version = crate_version!())]
#[command(author, version, about, arg_required_else_help = true, long_about = None)]
struct Cli {
  /// Where is your localchain? Defaults to a project-specific directory based on OS.
  ///    Linux:   /home/alice/.config/localchain
  ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
  ///    macOS:   /Users/Alice/Library/Application Support/org.ulixee.localchain
  #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH")]
  base_path: Option<PathBuf>,

  /// What mainchain RPC websocket url do you want to reach out use to sync blocks and submit
  /// notebook?
  #[clap(short, long, env, default_value = "ws://127.0.0.1:9944")]
  trusted_rpc_url: String,

  /// What default notary do you want to use?
  #[clap(short, long, env, default_value = "1")]
  notary_id: NotaryId,

  /// What default account id should claimed escrows be sent to?
  #[clap(long, value_name = "SS58_ADDRESS")]
  escrow_claims_send_to_address: Option<String>,

  /// What default account id will be used to create votes, accept claims, etc
  #[clap(long, value_name = "SS58_ADDRESS")]
  escrows_default_tax_account: Option<String>,

  /// What address should be used for votes (only relevant if claiming escrows)
  #[clap(long, value_name = "SS58_ADDRESS")]
  vote_address: Option<String>,

  /// Set a minimum amount of tax to wait for before submitting votes (does not ignore blockchain minimum
  #[clap(long)]
  minimum_vote_amount: Option<u128>,

  /// Load keys for signing escrows from the keystore.
  #[clap(flatten)]
  keystore_params: KeystoreParams,

  #[clap(long, default_value = "ntp.pool.org")]
  ntp_pool_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let cli = Cli::parse();
  tracing_subscriber::FmtSubscriber::builder()
    .with_env_filter(tracing_subscriber::EnvFilter::from_env("DEBUG"))
    .try_init()
    .expect("setting default subscriber failed");

  let base_path = cli
    .base_path
    .map(BasePath::new)
    .unwrap_or(BasePath::new(Localchain::get_default_path()));

  let keystore = read_keystore(&base_path, cli.keystore_params)?;

  let localchain = Localchain::load(LocalchainConfig {
    db_path: base_path.path().to_str().unwrap().to_string(),
    mainchain_url: cli.trusted_rpc_url.clone(),
    ntp_pool_url: Some(cli.ntp_pool_url.clone()),
  })
  .await?;

  let balance_sync = localchain.balance_sync();
  let sync_options = match cli.escrows_default_tax_account {
    Some(escrow_tax_address) => Some(EscrowCloseOptions {
      escrow_tax_address,
      escrow_claims_send_to_address: cli.escrow_claims_send_to_address,
      votes_address: cli
        .vote_address
        .expect("vote_address is required if escrow_tax_address is set"),
      minimum_vote_amount: cli.minimum_vote_amount.map(|v| v as i64),
    }),
    None => None,
  };

  let signer = Signer::with_keystore(keystore);
  loop {
    balance_sync
      .sync(sync_options.clone(), Some(&signer))
      .await?;

    time::sleep(localchain.duration_to_next_tick()).await;
  }
}

fn read_keystore(
  base_path: &BasePath,
  keystore_params: KeystoreParams,
) -> anyhow::Result<KeystorePtr> {
  let keystore: KeystorePtr = match keystore_params.keystore_config(base_path.path())? {
    KeystoreConfig::Path { path, password } => LocalKeystore::open(path, password)?.into(),
    _ => unreachable!("keystore_config always returns path and password; qed"),
  };
  Ok(keystore)
}
