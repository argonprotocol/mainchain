extern crate napi_build;

use std::process::Command;

fn main() {
  let output = Command::new("sqlx")
    .args(["database", "setup"])
    .output()
    .expect("failed to build database");
  if !output.status.success() {
    // Convert the output to a String to display the error
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("Error: {}", stderr);
  }
  napi_build::setup();
}
