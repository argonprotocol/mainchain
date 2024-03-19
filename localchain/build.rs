extern crate napi_build;

use std::env;
use dotenv::dotenv;
use std::process::Command;

fn main() {
  let offline = option_env!("SQLX_OFFLINE").unwrap_or("false");
  if offline != "1" && offline != "true" {
    dotenv().ok();
    let project_dir = env::current_dir().unwrap(); // Get the current directory

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let _ = Command::new("sqlx")
        .args(["database", "drop", "--database-url", &database_url])
        .current_dir(&project_dir) // Set the current directory for the command
        .output();

    let output = Command::new("sqlx")
        .args(["database", "setup", "--database-url", &database_url])
        .current_dir(&project_dir) // Set the current directory for the command
        .output()
        .expect(&format!("failed to build database at {}", database_url.clone()));
    if !output.status.success() {
      // Convert the output to a String to display the error
      let stderr = String::from_utf8_lossy(&output.stderr);
      panic!("Error setting up {}: {}", database_url, stderr);
    }
  }
  napi_build::setup();
}
