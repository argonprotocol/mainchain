extern crate napi_build;

use std::process::Command;

fn main() {
  let _ = Command::new("sqlx")
    .args(["database", "drop"])
    .output()
    .expect("failed to drop database");
  let output = Command::new("sqlx")
    .args(["database", "setup"])
    .output()
    .expect("failed to build database");
  if !output.status.success() {
    // Convert the output to a String to display the error
    let stderr = String::from_utf8_lossy(&output.stderr);
    panic!("Error: {}", stderr);
  }
  napi_build::setup();
}
