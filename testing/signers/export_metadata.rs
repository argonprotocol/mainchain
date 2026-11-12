use std::{env, fs};

fn main() {
	let metadata = argon_runtime::Runtime::metadata_at_version(15).expect("runtime supports V15");
	fs::write(env::args().nth(1).expect("output metadata path"), &*metadata)
		.expect("write runtime metadata");
}
