use anyhow::{bail, ensure, Context};
use bitcoin::hashes::{sha256, Hash};
use flate2::read::GzDecoder;
use std::{
	env,
	fs::{self, File},
	io::{self, Read},
	path::{Path, PathBuf},
};

const VERSION: &str = "31.2";
const MAX_ARCHIVE_BYTES: u64 = 128 * 1024 * 1024;

/// Find the Bitcoin Core test binary, downloading the pinned release when needed.
pub fn bitcoin_executable() -> anyhow::Result<PathBuf> {
	if let Some(executable) = env::var_os("BITCOIND_EXE") {
		return Ok(executable.into());
	}

	// https://github.com/bitcoin-core/guix.sigs/blob/main/31.2/fanquake/all.SHA256SUMS
	let (platform, checksum) = match (env::consts::OS, env::consts::ARCH) {
		("macos", "aarch64") => (
			"arm64-apple-darwin",
			"debfc5375b8aac1bcc30029407de2386e576a57cb88a8163fa77e506dc416b92",
		),
		("macos", "x86_64") => (
			"x86_64-apple-darwin",
			"98e7b744f71d174583b04c7b41b1438c7c0b5bdfde7d72bce5e4b70ab7fcb8e7",
		),
		("linux", "aarch64") => (
			"aarch64-linux-gnu",
			"1d7d8916719ed8de0f49937f8702ec5895b0af24d936e9c4d72b58f15afe3731",
		),
		("linux", "x86_64") =>
			("x86_64-linux-gnu", "2f38996afaa74dfa469a23bf4727887133dfc09784df1f21ddf1c74d0dc82300"),
		(os, arch) =>
			bail!("No Bitcoin Core {VERSION} test download for {os}/{arch}; set BITCOIND_EXE"),
	};

	let target_dir = env::var_os("CARGO_TARGET_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"));
	let cache_root = target_dir.join("bitcoin").join(VERSION);
	let install_dir = cache_root.join(format!("{}-{}", env::consts::OS, env::consts::ARCH));
	let executable = install_dir.join(format!("bitcoin-{VERSION}/bin/bitcoind"));
	if executable.is_file() {
		return Ok(executable.canonicalize()?);
	}

	fs::create_dir_all(&cache_root)?;
	let staging = tempfile::tempdir_in(&cache_root)?;
	let archive_name = format!("bitcoin-{VERSION}-{platform}.tar.gz");
	let url = format!("https://bitcoincore.org/bin/bitcoin-core-{VERSION}/{archive_name}");
	let response = minreq::get(&url)
		.with_timeout(120)
		.send_lazy()
		.context("Download Bitcoin Core")?;
	ensure!(
		response.status_code == 200,
		"Bitcoin Core download returned HTTP {}",
		response.status_code
	);

	let archive_path = staging.path().join(&archive_name);
	let mut archive = File::create(&archive_path)?;
	let bytes = io::copy(&mut Read::take(response, MAX_ARCHIVE_BYTES + 1), &mut archive)?;
	ensure!(bytes <= MAX_ARCHIVE_BYTES, "Bitcoin Core download exceeds 128 MiB");
	drop(archive);

	let unpacked = staging.path().join("unpacked");
	fs::create_dir(&unpacked)?;
	unpack_release(&archive_path, checksum, &unpacked)?;
	ensure!(
		unpacked.join(format!("bitcoin-{VERSION}/bin/bitcoind")).is_file(),
		"Bitcoin Core archive has no bitcoind binary"
	);
	if let Err(error) = fs::rename(&unpacked, &install_dir) {
		if !executable.is_file() {
			return Err(error.into());
		}
	}
	Ok(executable.canonicalize()?)
}

fn unpack_release(archive_path: &Path, checksum: &str, destination: &Path) -> anyhow::Result<()> {
	let bytes = fs::read(archive_path)?;
	ensure!(
		sha256::Hash::hash(&bytes).to_string() == checksum,
		"Bitcoin Core archive checksum mismatch"
	);
	tar::Archive::new(GzDecoder::new(bytes.as_slice())).unpack(destination)?;
	Ok(())
}

#[cfg(test)]
mod tests;
