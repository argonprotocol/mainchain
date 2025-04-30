extern crate napi_build;

use dotenv::dotenv;
use std::env;
use std::process::Command;

fn main() {
  let commit_hash = Command::new("git")
    .args(["rev-parse", "--short=11", "HEAD"])
    .output()
    .expect("Failed to execute git command")
    .stdout;

  let commit_hash =
    String::from_utf8(commit_hash).expect("Failed to convert commit hash to string");
  println!("cargo:rustc-env=GIT_COMMIT_HASH={}", commit_hash.trim());
  println!("cargo:rustc-env=IMPL_VERSION={}", get_version(&commit_hash));

  println!("cargo:rerun-if-changed=Cargo.toml");
  println!("Building localchain... {:?}", env::vars());

  let offline = option_env!("SQLX_OFFLINE").unwrap_or("false");
  if offline != "1" && offline != "true" {
    dotenv().ok();
    let project_dir = env::current_dir().unwrap(); // Get the current directory

    match Command::new("cargo").args(["sqlx", "--version"]).output() {
      Ok(output) if output.status.success() => {
        println!("Noop `sqlx-cli` is already installed.");
      }
      _ => {
        println!("Installing `sqlx-cli`...");
        Command::new("cargo")
          .args(["install", "sqlx-cli@^0.8"])
          .status()
          .expect("Failed to install `sqlx-cli`");
      }
    }

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let output = Command::new("cargo")
      .args([
        "sqlx",
        "database",
        "reset",
        "-y",
        "--database-url",
        &database_url,
      ])
      .current_dir(&project_dir) // Set the current directory for the command
      .output()
      .unwrap_or_else(|_| panic!("failed to build database at {}", database_url.clone()));
    if !output.status.success() {
      // Convert the output to a String to display the error
      let stderr = String::from_utf8_lossy(&output.stderr);
      panic!("Error setting up {}: {}", database_url, stderr);
    }
  }

  if env::var("CARGO_FEATURE_NAPI").is_ok() {
    println!("setting up napi build...");
    napi_build::setup();
  }
}

fn get_version(impl_commit: &str) -> String {
  let commit_dash = if impl_commit.is_empty() { "" } else { "-" };
  format!(
    "{}{}{}",
    std::env::var("CARGO_PKG_VERSION").unwrap_or_default(),
    commit_dash,
    impl_commit
  )
}
