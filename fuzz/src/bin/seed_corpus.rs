use argon_runtime_fuzz::{
	LifecycleStats, CLOSE, CREATE, OPERATIONS, RATCHET, REPRICE, SET_GUARANTEE,
};
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let directory = PathBuf::from(
		env::args_os().nth(1).unwrap_or_else(|| "fuzz/corpus/bitcoin_lifecycle".into()),
	);
	if env::args_os().nth(2).is_some_and(|arg| arg == "--report") {
		let mut inputs = Vec::new();
		for entry in fs::read_dir(&directory)? {
			let entry = entry?;
			if entry.file_type()?.is_file() {
				inputs.push(entry.path());
			}
		}
		inputs.sort();
		assert!(!inputs.is_empty(), "Bitcoin corpus must contain inputs");

		let mut successful = [0u64; 5];
		let mut rejected = [0u64; 5];
		for path in &inputs {
			let stats = argon_runtime_fuzz::run(&fs::read(path)?);
			for operation in 0..OPERATIONS.len() {
				successful[operation] += u64::from(stats.successful[operation]);
				rejected[operation] += u64::from(stats.rejected[operation]);
			}
		}
		println!(
			"Completed {} corpus lifecycles; each starts with one created Fission",
			inputs.len()
		);
		println!("operation\tsuccessful\trejected");
		for (operation, name) in OPERATIONS.iter().enumerate() {
			println!("{name}\t{}\t{}", successful[operation], rejected[operation]);
		}
		return Ok(());
	}

	fs::create_dir_all(&directory)?;
	let mut repayment_exhaustion = vec![2];
	repayment_exhaustion.extend([CLOSE, 0, 0, CREATE, 0, 3].repeat(16));
	let scenarios: [(&str, &[u8], LifecycleStats); 4] = [
		(
			"ratchet-close",
			&[0, REPRICE, 0, 3, CREATE, 0, 1, RATCHET, 0, 4, CLOSE, 0, 0, CLOSE, 0, 0],
			LifecycleStats { successful: [2, 1, 2, 0, 1], rejected: [0; 5] },
		),
		(
			"outside-spend",
			&[0, REPRICE, 0, 3, CREATE, 0, 1, RATCHET, 0, 4, SET_GUARANTEE, 0, 3],
			LifecycleStats { successful: [2, 1, 0, 1, 1], rejected: [0; 5] },
		),
		(
			"rejected-operations",
			&[1, CREATE, 0, 5, CREATE, 128, 1, CLOSE, 128, 0, RATCHET, 0, 2],
			LifecycleStats { successful: [1, 0, 0, 0, 0], rejected: [2, 1, 1, 0, 0] },
		),
		(
			"repayment-exhaustion",
			repayment_exhaustion.as_slice(),
			LifecycleStats { successful: [12, 0, 11, 0, 0], rejected: [5, 0, 6, 0, 0] },
		),
	];
	for (name, input, expected) in scenarios {
		let actual = argon_runtime_fuzz::run(input);
		assert_eq!(actual, expected, "Bitcoin seed {name} must reach its lifecycle transitions");
		println!("{name}: {actual:?}; source Lock settled");
		fs::write(directory.join(name), input)?;
	}
	Ok(())
}
