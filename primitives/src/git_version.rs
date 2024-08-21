use std::process::Command;

pub fn generate_git_cli_env_var() {
	let commit_hash = Command::new("git")
		.args(["rev-parse", "--short=11", "HEAD"])
		.output()
		.expect("Failed to execute git command")
		.stdout;

	let commit_hash =
		String::from_utf8(commit_hash).expect("Failed to convert commit hash to string");
	println!("cargo:rustc-env=GIT_COMMIT_HASH={}", commit_hash.trim());
	println!("cargo:rustc-env=IMPL_VERSION={}", get_version(&commit_hash));
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
