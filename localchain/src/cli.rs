use std::ffi::OsString;
use std::fmt::Debug;
use std::{env, fs, path::PathBuf};

use anyhow::anyhow;
use clap::{crate_version, Args, Parser, Subcommand, ValueHint};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Table};

use ulx_primitives::DataDomain;

use crate::keystore::Keystore;
use crate::{
  AccountStore, BalanceChangeRow, BalanceChangeStore, CryptoScheme, DataDomainStore,
  EscrowCloseOptions, LocalAccount, Localchain, LocalchainConfig, MainchainClient, TickerConfig,
};

#[derive(Parser, Debug)]
#[clap(version = crate_version!())]
#[command(author, version, about, arg_required_else_help = true, long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
  /// Sync the localchain proofs with the latest notebooks. This will also submit votes and close/claim escrows as needed.
  Sync {
    /// Which localchain are you synching?
    #[clap(default_value = "primary")]
    name: String,

    /// Where is your localchain? Defaults to a project-specific directory based on OS.
    ///    Linux:   /home/alice/.config/ulixee/localchain
    ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
    ///    macOS:   /Users/Alice/Library/Application Support/ulixee/localchain
    #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH", value_hint = ValueHint::DirPath)]
    base_dir: Option<PathBuf>,

    /// What mainchain RPC websocket url do you want to reach out use to sync blocks and submit
    /// notebook?
    #[clap(
      short,
      long,
      env = "ULX_MAINCHAIN_URL",
      default_value = "ws://127.0.0.1:9944"
    )]
    mainchain_url: String,

    /// What address should be used for votes (only relevant if claiming escrows)
    #[clap(long, value_name = "SS58_ADDRESS")]
    vote_address: Option<String>,

    /// Set a minimum amount of tax to wait for before submitting votes (does not ignore blockchain minimum
    #[clap(long)]
    minimum_vote_amount: Option<u128>,

    /// Password to unlock the embedded keystore
    #[clap(flatten)]
    keystore_password: EmbeddedKeyPassword,
  },

  /// Explore and manage data domains
  DataDomains {
    #[clap(subcommand)]
    subcommand: DataDomainsSubcommand,
  },

  /// Manage local accounts
  Accounts {
    #[clap(subcommand)]
    subcommand: AccountsSubcommand,
  },
}

#[derive(Subcommand, Debug)]
enum DataDomainsSubcommand {
  /// List all installed data domains
  List {
    #[clap(long, value_name = "LOCALCHAIN_NAME", default_value = "primary")]
    name: String,

    /// Where is your localchain? Defaults to a project-specific directory based on OS.
    ///    Linux:   /home/alice/.config/ulixee/localchain
    ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
    ///    macOS:   /Users/Alice/Library/Application Support/ulixee/localchain
    #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH", value_hint = ValueHint::DirPath)]
    base_dir: Option<PathBuf>,
  },
  /// Generate the hash for a data domain
  Hash {
    /// The data domain name
    #[clap()]
    data_domain: String,
  },
  /// Check if a data domain is registered
  Check {
    /// The data domain name
    #[clap()]
    data_domain: String,

    /// What mainchain RPC websocket url do you want to use to check registrations
    #[clap(
      short,
      long,
      env = "ULX_MAINCHAIN_URL",
      value_name = "URL",
      default_value = "ws://127.0.0.1:9944"
    )]
    mainchain_url: String,
  },
  /// Lease a data domain
  Lease {
    /// The data domain name
    #[clap()]
    data_domain: String,

    #[clap(long, value_name = "LOCALCHAIN_NAME", default_value = "primary")]
    name: String,

    /// Where is your localchain? Defaults to a project-specific directory based on OS.
    ///    Linux:   /home/alice/.config/ulixee/localchain
    ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
    ///    macOS:   /Users/Alice/Library/Application Support/ulixee/localchain
    #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH", value_hint = ValueHint::DirPath)]
    base_dir: Option<PathBuf>,

    /// Password to unlock the embedded keystore
    #[clap(flatten)]
    keystore_password: EmbeddedKeyPassword,

    /// Which account should the registration be assigned to. This is the account you'll manage the data domain with on the mainchain.
    #[arg(short, long, value_name = "SS58_ADDRESS", required = true)]
    owner_address: String,
  },
}

#[derive(Subcommand, Debug)]
enum AccountsSubcommand {
  /// List all localchains you have access to
  List {
    /// Where is your localchain? Defaults to a project-specific directory based on OS.
    ///    Linux:   /home/alice/.config/ulixee/localchain
    ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
    ///    macOS:   /Users/Alice/Library/Application Support/ulixee/localchain
    #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH", value_hint = ValueHint::DirPath)]
    base_dir: Option<PathBuf>,
  },

  /// Create a new localchain
  Create {
    #[clap(default_value = "primary")]
    name: String,

    /// Where is your localchain? Defaults to a project-specific directory based on OS.
    ///    Linux:   /home/alice/.config/ulixee/localchain
    ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
    ///    macOS:   /Users/Alice/Library/Application Support/ulixee/localchain
    #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH", value_hint = ValueHint::DirPath)]
    base_dir: Option<PathBuf>,

    /// The secret key URI.
    /// If the value is a file, the file content is used as URI.
    /// If not given, a key will be autogenerated
    #[clap(long)]
    suri: Option<String>,

    /// Add a password for this key (can also be embedded in the suri by trailing the suri with ///password)
    /// You will use this to unlock your keystore signing
    #[clap(flatten)]
    keystore_password: EmbeddedKeyPassword,

    /// The crypto scheme to use for the key
    #[clap(long, default_value = "sr25519")]
    scheme: CryptoScheme,
  },
}

#[cfg(feature = "napi")]
#[napi(js_name = "runCli")]
pub async fn run_js() -> napi::Result<()> {
  let _ = tracing_subscriber::FmtSubscriber::builder()
    .with_env_filter(tracing_subscriber::EnvFilter::from_env("DEBUG"))
    .try_init();

  let inner_args = {
    let mut args = env::args_os();
    // lop off the first nodejs arg
    let _ = args.next();
    args.collect::<Vec<OsString>>()
  };
  let result = run(inner_args).await;
  result?;
  Ok(())
}

pub async fn run<I, T>(itr: I) -> anyhow::Result<()>
where
  I: IntoIterator<Item = T>,
  T: Into<OsString> + Clone,
{
  let cli = Cli::parse_from(itr);

  match cli.command {
    Commands::Sync {
      base_dir,
      mainchain_url,
      vote_address,
      keystore_password,
      minimum_vote_amount,
      name,
    } => {
      let path = get_path(base_dir.clone(), name.clone());
      let localchain = Localchain::load(LocalchainConfig {
        path,
        mainchain_url,
        ntp_pool_url: None,
        keystore_password: Some(keystore_password),
      })
      .await?;

      let balance_sync = localchain.balance_sync();
      let sync_options = vote_address.map(|vote_address| EscrowCloseOptions {
        votes_address: Some(vote_address),
        minimum_vote_amount: minimum_vote_amount.map(|v| v as i64),
      });

      let sync = balance_sync.sync(sync_options.clone()).await?;
      println!(
        "Synced {:?} balance changes. Escrows updated: {:?}",
        sync.balance_changes().len(),
        sync.escrow_notarizations().len()
      );
    }
    Commands::DataDomains { subcommand } => match subcommand {
      DataDomainsSubcommand::List { base_dir, name } => {
        let db = Localchain::create_db(get_path(base_dir, name)).await?;
        let domains = DataDomainStore::new(db);
        let data_domains = domains.list().await?;

        let mut table = Table::new();

        table
          .load_preset(UTF8_FULL)
          .apply_modifier(UTF8_ROUND_CORNERS)
          .set_content_arrangement(ContentArrangement::Dynamic)
          .set_header(vec![
            "Top Level",
            "Second Level",
            "Owner",
            "Registration Tick",
            "Hash",
          ]);
        for domain in data_domains {
          table.add_row(vec![
            domain.tld.clone(),
            domain.name.clone(),
            domain.registered_to_address,
            domain.registered_at_tick.to_string(),
            DataDomain::from_string(
              domain.name,
              DataDomainStore::tld_from_string(domain.tld)
                .expect("Should be able to translate a tld"),
            )
            .hash()
            .to_string(),
          ]);
        }
        println!("{table}");
      }
      DataDomainsSubcommand::Hash { data_domain } => {
        let domain =
          DataDomain::parse(data_domain).map_err(|_| anyhow!("Not a valid data domain"))?;
        println!("Hash: {:?}", domain.hash());
      }
      DataDomainsSubcommand::Check {
        data_domain,
        mainchain_url,
      } => {
        let domain =
          DataDomain::parse(data_domain.clone()).map_err(|_| anyhow!("Not a valid data domain"))?;
        let mainchain = MainchainClient::connect(mainchain_url, 5_000).await?;
        let registration = mainchain
          .get_data_domain_registration(
            domain.domain_name.clone().to_string(),
            domain.top_level_domain,
          )
          .await?;
        let mut table = Table::new();
        table
          .load_preset(UTF8_FULL)
          .apply_modifier(UTF8_ROUND_CORNERS)
          .set_content_arrangement(ContentArrangement::Dynamic)
          .set_header(vec!["Domain", "Registered?", "Hash"]);
        table.add_row(vec![
          Cell::new(&data_domain),
          Cell::new(match registration.is_some() {
            true => "Yes",
            false => "No",
          })
          .set_alignment(CellAlignment::Center),
          Cell::new(hex::encode(domain.hash().0)).set_alignment(CellAlignment::Center),
        ]);
        println!("{table}");
      }
      DataDomainsSubcommand::Lease {
        name,
        base_dir,
        keystore_password,
        data_domain,
        owner_address,
      } => {
        let domain =
          DataDomain::parse(data_domain.clone()).map_err(|_| anyhow!("Not a valid data domain"))?;
        let path = get_path(base_dir.clone(), name);
        let localchain = Localchain::load_without_mainchain(
          path,
          TickerConfig {
            ntp_pool_url: None,
            genesis_utc_time: 0,
            tick_duration_millis: 0,
          },
          Some(keystore_password),
        )
        .await?;

        let change = localchain.begin_change();
        change
          .lease_data_domain(data_domain.clone(), owner_address)
          .await?;
        change.sign().await?;
        let tracker = change.notarize().await?;
        let mut table = Table::new();

        table
          .load_preset(UTF8_FULL)
          .apply_modifier(UTF8_ROUND_CORNERS)
          .set_content_arrangement(ContentArrangement::Dynamic)
          .set_header(vec!["Change #", "Balance", "Status"]);
        for (_account, balance_change) in tracker.get_changed_accounts().await {
          table.add_row(vec![
            balance_change.change_number.to_string(),
            balance_change.balance,
            format!("{:?}", balance_change.status),
          ]);
        }
        println!("{} registered at tick {} in notebook {}. Domain hash={:#?} (use this hash for zone record registration on mainchain).\
          \n\nChanged Accounts:\n{table}",
                         data_domain, tracker.tick, tracker.notebook_number, domain.hash());
      }
    },

    Commands::Accounts { subcommand } => match subcommand {
      AccountsSubcommand::Create {
        name,
        scheme,
        base_dir,
        suri,
        keystore_password,
      } => {
        let path = get_path(base_dir.clone(), name.clone());
        if fs::metadata(&path).is_ok() {
          return Err(anyhow!("Localchain already exists at {:?}", path));
        }

        let db = Localchain::create_db(path.clone()).await?;
        let keystore = Keystore::new(db.clone());
        if let Some(suri) = suri {
          keystore
            .import_suri(suri, scheme, Some(keystore_password))
            .await?;
        } else {
          keystore
            .bootstrap(Some(scheme), Some(keystore_password))
            .await?;
        }

        let mut table = Table::new();

        let mut conn = db.acquire().await?;
        table
          .load_preset(UTF8_FULL)
          .apply_modifier(UTF8_ROUND_CORNERS)
          .set_content_arrangement(ContentArrangement::Dynamic)
          .set_header(vec!["Address", "Path", "NotaryId"]);
        let account = AccountStore::db_deposit_account(&mut conn, None).await?;
        table.add_row(vec![account.address, path, account.notary_id.to_string()]);

        println!("Account created at:\n{table}");
      }

      AccountsSubcommand::List { base_dir } => {
        let dir = base_dir.unwrap_or_else(|| PathBuf::from(Localchain::get_default_dir()));

        let mut table = Table::new();

        table
          .load_preset(UTF8_FULL)
          .apply_modifier(UTF8_ROUND_CORNERS)
          .set_content_arrangement(ContentArrangement::Dynamic)
          .set_header(account_columns());

        for entry in fs::read_dir(dir.clone())? {
          let Some(entry) = entry.ok() else {
            continue;
          };
          let Some(file_type) = entry.file_type().ok() else {
            continue;
          };
          if !file_type.is_file() {
            continue;
          }
          let Some(name) = entry.file_name().into_string().ok() else {
            continue;
          };
          if name.ends_with(".db") {
            let path = get_path(Some(dir.clone()), name.clone());
            let db = Localchain::create_db(path).await?;
            let mut conn = db.acquire().await?;
            let accounts = AccountStore::db_list(&mut conn, false).await?;
            for account in accounts {
              let balance_change =
                BalanceChangeStore::db_get_latest_for_account(&mut conn, account.id).await?;
              table.add_row(format_account_record(&name, account, balance_change));
            }
          }
        }

        println!("{table}");
      }
    },
  }
  Ok(())
}

fn account_columns() -> Vec<&'static str> {
  vec![
    "Name", "Address", "Type", "NotaryId", "Change #", "Balance", "Status",
  ]
}
fn format_account_record(
  name: &str,
  account: LocalAccount,
  balance_change: Option<BalanceChangeRow>,
) -> Vec<String> {
  vec![
    name.replace(".db", ""),
    account.address,
    account.account_type.as_str().to_string(),
    account.notary_id.to_string(),
    balance_change
      .clone()
      .map(|x| x.change_number.to_string())
      .unwrap_or("0".to_string()),
    balance_change
      .clone()
      .map(|x| x.balance.to_string())
      .unwrap_or("0".to_string()),
    balance_change
      .map(|x| format!("{:?}", x.status))
      .unwrap_or("-".to_string()),
  ]
}

fn get_path(base_dir: Option<PathBuf>, name: String) -> String {
  let base_dir = base_dir.unwrap_or(PathBuf::from(Localchain::get_default_path()));
  base_dir
    .join(format!("{}.db", name.replace(".db", "")))
    .to_str()
    .expect("Path should convert to a string")
    .to_string()
}

/// Parameters of the keystore
#[derive(Debug, Clone, Args)]
pub struct EmbeddedKeyPassword {
  /// Use interactive shell for entering the password used by the embedded keystore.
  #[arg(long, conflicts_with_all = &["key_password", "key_password_filename"])]
  pub key_password_interactive: bool,

  /// Password used by the embedded keystore.
  ///
  /// This allows appending an extra user-defined secret to the seed.
  #[arg(
    long,
    conflicts_with_all = &["key_password_interactive", "key_password_filename"]
  )]
  pub key_password: Option<String>,

  /// File that contains the password used by the embedded keystore.
  #[arg(
    long,
    value_name = "PATH",
    conflicts_with_all = &["key_password_interactive", "key_password"]
  )]
  pub key_password_filename: Option<String>,
}
