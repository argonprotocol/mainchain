use std::{env, fs};

fn main() {
	let version = env::args().nth(2).map(|version| version.parse().expect("metadata version")).unwrap_or(15);
	let metadata = argon_runtime::Runtime::metadata_at_version(version).expect("supported metadata version");
	fs::write(env::args().nth(1).expect("output metadata path"), &*metadata)
		.expect("write runtime metadata");
}
