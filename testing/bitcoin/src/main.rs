use bitcoind::downloaded_exe_path;

fn main() {
	println!("{}", downloaded_exe_path().unwrap());
}
