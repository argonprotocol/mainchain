use std::{env, process::Command};

use dotenv::dotenv;

fn main() {
	let offline = option_env!("SQLX_OFFLINE").unwrap_or("false");
	if offline != "1" && offline != "true" {
		dotenv().ok();
		let project_dir = env::current_dir().unwrap(); // Get the current directory

		let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

		match Command::new("cargo").args(["sqlx", "--version"]).output() {
			Ok(output) if output.status.success() => {
				println!("`sqlx-cli` is already installed.");
			},
			_ => {
				println!("Installing `sqlx-cli`...");
				Command::new("cargo")
					.args(["install", "sqlx-cli@^0.7"])
					.status()
					.expect("Failed to install `sqlx-cli`");
			},
		}

		match Command::new("cargo")
			.args(["sqlx", "database", "setup", "--database-url", &database_url])
			.current_dir(&project_dir) // Set the current directory for the command
			.output()
		{
			Ok(output) => {
				if !output.status.success() {
					// Convert the output to a String to display the error
					let stderr = String::from_utf8_lossy(&output.stderr);
					println!("Error setting up {}: {}", database_url, stderr);
				}
			},
			Err(e) => {
				println!("Error setting up {}: {}", database_url, e);
			},
		}
	}
	println!("cargo:rerun-if-changed=Cargo.toml");
}
