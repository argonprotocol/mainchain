use sp_core::blake2_256;
use std::time::Instant;

fn main() {
	let data = [0u8; 256]; // Change this to the size of your data
	let start = Instant::now();
	let mut count = 0u64;

	while start.elapsed().as_secs() < 10 {
		blake2_256(&data);
		count += 1;
	}

	println!("Hashes per second: {}", count);
}
