use argon_testing_bitcoin::bitcoin_executable;

fn main() -> anyhow::Result<()> {
	println!("{}", bitcoin_executable()?.display());
	Ok(())
}
