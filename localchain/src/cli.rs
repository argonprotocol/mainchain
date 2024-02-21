use std::ffi::OsString;
use std::fmt::Debug;
use std::{env, path::PathBuf};

use anyhow::anyhow;
use clap::{crate_version, Parser, Subcommand, ValueHint};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Table};
use sc_cli::{utils, KeystoreParams};
use sc_service::config::KeystoreConfig;
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::IdentifyAccount;
use sp_runtime::MultiSigner;
use sqlx::SqliteConnection;

use ulx_primitives::{AccountType, DataDomain, NotaryId};

use crate::signer::Signer;
use crate::{
  to_js_error, AccountStore, BalanceChangeStore, CryptoScheme, DataDomainStore, EscrowCloseOptions,
  LocalAccount, LocalKeystore, Localchain, LocalchainConfig, MainchainClient, TickerConfig,
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
    /// Where is your localchain? Defaults to a project-specific directory based on OS.
    ///    Linux:   /home/alice/.config/localchain
    ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
    ///    macOS:   /Users/Alice/Library/Application Support/org.ulixee.localchain
    #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH", value_hint = ValueHint::DirPath)]
    base_path: Option<PathBuf>,

    /// What mainchain RPC websocket url do you want to reach out use to sync blocks and submit
    /// notebook?
    #[clap(
      short,
      long,
      env = "ULX_MAINCHAIN_URL",
      default_value = "ws://127.0.0.1:9944"
    )]
    mainchain_url: String,

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

    /// Load keys for signrows from the keystore.
    #[clap(flatten)]
    keystore_params: KeystoreParams,
  },

  /// Manage a local keystore
  Keystore {
    #[clap(subcommand)]
    subcommand: KeysSubcommand,
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
enum KeysSubcommand {
  /// Insert a new key into the keystore
  Insert {
    /// The secret key URI.
    /// If the value is a file, the file content is used as URI.
    /// If not given, you will be prompted for the URI.
    #[clap(long)]
    suri: Option<String>,

    /// The crypto scheme to use for the key
    #[clap(long, default_value = "sr25519")]
    scheme: CryptoScheme,

    /// Configuration for the keystore to use
    #[clap(flatten)]
    keystore_params: KeystoreParams,
  },
  /// List all keys in the keystore
  List {
    /// Configuration for the keystore to use
    #[clap(flatten)]
    keystore_params: KeystoreParams,
  },
}

#[derive(Subcommand, Debug)]
enum DataDomainsSubcommand {
  /// List all installed data domains
  List {
    /// Where is your localchain? Defaults to a project-specific directory based on OS.
    ///    Linux:   /home/alice/.config/localchain
    ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
    ///    macOS:   /Users/Alice/Library/Application Support/org.ulixee.localchain
    #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH", value_hint = ValueHint::DirPath)]
    base_path: Option<PathBuf>,
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

    /// Where is your localchain? Defaults to a project-specific directory based on OS.
    ///    Linux:   /home/alice/.config/localchain
    ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
    ///    macOS:   /Users/Alice/Library/Application Support/org.ulixee.localchain
    #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH", value_hint = ValueHint::DirPath)]
    base_path: Option<PathBuf>,

    /// Configuration for the keystore to use
    #[clap(flatten)]
    keystore_params: KeystoreParams,

    /// Which account should be used to fund the registration
    #[arg(short, long, value_name = "SS58_ADDRESS", required = true)]
    funds_address: String,

    /// Which account should be used to place the output tax into. Defaults to Tax account with funds address.
    #[arg(short, long, value_name = "SS58_ADDRESS")]
    tax_address: Option<String>,

    /// Which account should the registration be assigned to. This is the account you'll manage the data domain with on the mainchain.
    #[arg(short, long, value_name = "SS58_ADDRESS", required = true)]
    owner_address: String,
  },
}

#[derive(Subcommand, Debug)]
enum AccountsSubcommand {
  /// List all accounts with balances
  List {
    /// Where is your localchain? Defaults to a project-specific directory based on OS.
    ///    Linux:   /home/alice/.config/localchain
    ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
    ///    macOS:   /Users/Alice/Library/Application Support/org.ulixee.localchain
    #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH", value_hint = ValueHint::DirPath)]
    base_path: Option<PathBuf>,
  },

  /// Show history of an account
  History {
    /// Where is your localchain? Defaults to a project-specific directory based on OS.
    ///    Linux:   /home/alice/.config/localchain
    ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
    ///    macOS:   /Users/Alice/Library/Application Support/org.ulixee.localchain
    #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH", value_hint = ValueHint::DirPath)]
    base_path: Option<PathBuf>,

    /// The account address must be given in SS58 format.
    #[arg(value_name = "SS58_ADDRESS", required = true)]
    address: String,

    /// The type of account
    #[arg(short, long, value_enum, ignore_case = true, default_value = "deposit")]
    account_type: AccountType,

    /// The notary id
    #[arg(short, long, default_value = "1")]
    notary_id: NotaryId,
  },

  /// Add an account to the Localchain
  Add {
    /// Where is your localchain? Defaults to a project-specific directory based on OS.
    ///    Linux:   /home/alice/.config/localchain
    ///    Windows: C:\Users\Alice\AppData\Roaming\ulixee\localchain
    ///    macOS:   /Users/Alice/Library/Application Support/org.ulixee.localchain
    #[clap(long, env = "ULX_LOCALCHAIN_BASE_PATH", value_hint = ValueHint::DirPath)]
    base_path: Option<PathBuf>,

    /// The account address must be given in SS58 format.
    #[arg(value_name = "SS58_ADDRESS", required = true)]
    address: String,

    /// The type of account
    #[arg(short, long, value_enum, ignore_case = true, default_value = "deposit")]
    account_type: AccountType,

    /// The notary id
    #[arg(short, long, default_value = "1")]
    notary_id: NotaryId,
  },
}

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
  result.map_err(to_js_error)?;
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
      base_path,
      mainchain_url,
      vote_address,
      escrows_default_tax_account,
      escrow_claims_send_to_address,
      keystore_params,
      minimum_vote_amount,
    } => {
      let keystore = read_keystore(base_path.clone(), keystore_params)?;

      let db_path = get_db_path(base_path.clone());
      let localchain = Localchain::load(LocalchainConfig {
        db_path,
        mainchain_url,
        ntp_pool_url: None,
      })
      .await?;

      let balance_sync = localchain.balance_sync();
      let sync_options = match escrows_default_tax_account {
        Some(escrow_tax_address) => Some(EscrowCloseOptions {
          escrow_tax_address,
          escrow_claims_send_to_address,
          votes_address: vote_address
            .expect("vote_address is required if escrow_tax_address is set"),
          minimum_vote_amount: minimum_vote_amount.map(|v| v as i64),
        }),
        None => None,
      };

      let signer = Signer::with_keystore(keystore.into());
      let sync = balance_sync
        .sync(sync_options.clone(), Some(&signer))
        .await?;
      println!(
        "Synced {:?} balance changes. Escrows updated: {:?}",
        sync.balance_changes().len(),
        sync.escrow_notarizations().len()
      );
    }
    Commands::Keystore {
      subcommand: KeysSubcommand::List { keystore_params },
    } => {
      let keystore = read_keystore(None, keystore_params)?;
      let mut table = Table::new();

      table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Address", "Type"]);

      for key in keystore.multi_public_keys() {
        let label = match key {
          MultiSigner::Ed25519(_) => "Ed25519",
          MultiSigner::Sr25519(_) => "Sr25519",
          MultiSigner::Ecdsa(_) => "ECDSA",
        };
        let address = key
          .into_account()
          .to_ss58check_with_version(AccountStore::address_format());
        table.add_row(vec![address, label.to_string()]);
      }
      println!("{table}")
    }
    Commands::Keystore {
      subcommand:
        KeysSubcommand::Insert {
          suri,
          scheme,
          keystore_params,
        },
    } => {
      let suri = utils::read_uri(suri.as_ref())?;
      let password = keystore_params.read_password()?;
      let keystore = read_keystore(None, keystore_params)?;
      let multi_signer = keystore
        .insert(&suri, scheme, password)
        .map_err(|_| anyhow!("Could not insert key into the keystore"))?;

      let address = multi_signer
        .into_account()
        .to_ss58check_with_version(AccountStore::address_format());
      println!("Key inserted into keystore. Address {:?}", address);
    }
    Commands::DataDomains { subcommand } => match subcommand {
      DataDomainsSubcommand::List { base_path } => {
        let db = Localchain::create_db(get_db_path(base_path)).await?;
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
          .get_data_domain_registration(domain.domain_name.to_string(), domain.top_level_domain)
          .await?;
        let mut table = Table::new();
        table
          .load_preset(UTF8_FULL)
          .apply_modifier(UTF8_ROUND_CORNERS)
          .set_content_arrangement(ContentArrangement::Dynamic)
          .set_header(vec!["Domain", "Registered?", "Hash"]);
        table.add_row(vec![
          Cell::new(&data_domain),
          Cell::new(&match registration.is_some() {
            true => "Yes",
            false => "No",
          })
          .set_alignment(CellAlignment::Center),
          Cell::new(&hex::encode(domain.hash().0)).set_alignment(CellAlignment::Center),
        ]);
        println!("{table}");
      }
      DataDomainsSubcommand::Lease {
        base_path,
        keystore_params,
        data_domain,
        funds_address,
        tax_address,
        owner_address,
      } => {
        let domain =
          DataDomain::parse(data_domain.clone()).map_err(|_| anyhow!("Not a valid data domain"))?;
        let db_path = get_db_path(base_path.clone());
        let localchain = Localchain::load_without_mainchain(
          db_path,
          TickerConfig {
            ntp_pool_url: None,
            genesis_utc_time: 0,
            tick_duration_millis: 0,
          },
        )
        .await?;

        let keystore = read_keystore(base_path, keystore_params)?;
        let signer = Signer::with_keystore(keystore.into());

        let tax_address = tax_address.unwrap_or_else(|| funds_address.clone());
        let change = localchain.begin_change();
        change
          .lease_data_domain(
            funds_address,
            tax_address,
            data_domain.clone(),
            owner_address,
          )
          .await?;
        change.sign(&signer).await?;
        let tracker = change.notarize().await?;
        let mut table = Table::new();

        table
          .load_preset(UTF8_FULL)
          .apply_modifier(UTF8_ROUND_CORNERS)
          .set_content_arrangement(ContentArrangement::Dynamic)
          .set_header(vec![
            "Address", "Type", "NotaryId", "Change #", "Balance", "Status",
          ]);
        for (account, balance_change) in tracker.get_changed_accounts().await {
          table.add_row(vec![
            account.address,
            account.account_type.as_str().to_string(),
            account.notary_id.to_string(),
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
      AccountsSubcommand::Add {
        account_type,
        notary_id,
        base_path,
        address,
      } => {
        let db_path = get_db_path(base_path.clone());
        let db = Localchain::create_db(db_path).await?;
        let mut conn = db.acquire().await?;
        let account = AccountStore::insert(&mut conn, address, account_type, notary_id).await?;

        let mut table = Table::new();

        table
          .load_preset(UTF8_FULL)
          .apply_modifier(UTF8_ROUND_CORNERS)
          .set_content_arrangement(ContentArrangement::Dynamic)
          .set_header(vec![
            "Address", "Type", "NotaryId", "Change #", "Balance", "Status",
          ]);
        table.add_row(format_account_record(&mut conn, account).await?);

        println!("{table}");
      }
      AccountsSubcommand::History {
        base_path,
        address,
        account_type,
        notary_id,
      } => {
        let db_path = get_db_path(base_path.clone());
        let db = Localchain::create_db(db_path).await?;
        let mut conn = db.acquire().await?;
        let account = AccountStore::get(&mut conn, address, account_type, notary_id).await?;

        let balance_changes = BalanceChangeStore::new(db.clone());
        let history = balance_changes.all_for_account(account.id).await?;

        let mut table = Table::new();

        table
          .load_preset(UTF8_FULL)
          .apply_modifier(UTF8_ROUND_CORNERS)
          .set_content_arrangement(ContentArrangement::Dynamic)
          .set_header(vec![
            "Change #",
            "Balance",
            "Notebook #",
            "Tick",
            "Notes",
            "Status",
          ]);
        for record in history {
          let (notebook_number, tick) = match record.notarization_id {
            Some(notarization_id) => {
              BalanceChangeStore::get_notarization_notebook(&mut conn, notarization_id).await?
            }
            None => (None, None),
          };
          table.add_row(vec![
            record.change_number.to_string(),
            record.balance,
            notebook_number
              .map(|x| x.to_string())
              .unwrap_or("".to_string()),
            tick.map(|x| x.to_string()).unwrap_or("".to_string()),
            record
              .notes_json
              .map(|x| x.to_string())
              .unwrap_or("".to_string()),
            format!("{:?}", record.status),
          ]);
        }
        println!("{table}");
      }
      AccountsSubcommand::List { base_path } => {
        let db_path = get_db_path(base_path.clone());
        let db = Localchain::create_db(db_path).await?;
        let mut conn = db.acquire().await?;
        let account = AccountStore::list(&mut conn).await?;

        let mut table = Table::new();

        table
          .load_preset(UTF8_FULL)
          .apply_modifier(UTF8_ROUND_CORNERS)
          .set_content_arrangement(ContentArrangement::Dynamic)
          .set_header(vec![
            "Address", "Type", "NotaryId", "Change #", "Balance", "Status",
          ]);
        for account in account {
          table.add_row(format_account_record(&mut conn, account).await?);
        }
        println!("{table}");
      }
    },
  }
  Ok(())
}

async fn format_account_record(
  db: &mut SqliteConnection,
  account: LocalAccount,
) -> anyhow::Result<Vec<String>> {
  let balance_change = BalanceChangeStore::get_latest_for_account(db, account.id).await?;
  Ok(vec![
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
  ])
}

fn get_db_path(base_path: Option<PathBuf>) -> String {
  let base_path = base_path.unwrap_or(PathBuf::from(Localchain::get_default_path()));
  base_path
    .join("localchain.db")
    .to_str()
    .expect("Path should convert to a string")
    .to_string()
}

fn read_keystore(
  base_path: Option<PathBuf>,
  keystore_params: KeystoreParams,
) -> anyhow::Result<LocalKeystore> {
  let base_path = base_path.unwrap_or(PathBuf::from(Localchain::get_default_path()));
  let keystore = match keystore_params.keystore_config(base_path.as_path())? {
    KeystoreConfig::Path { path, password } => LocalKeystore::open(path, password)?.into(),
    _ => unreachable!("keystore_config always returns path and password; qed"),
  };
  Ok(keystore)
}
