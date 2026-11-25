use super::*;
use flate2::{write::GzEncoder, Compression};

#[test]
fn unpacks_a_verified_bitcoin_release() {
	let directory = tempfile::tempdir().unwrap();
	let archive_path = directory.path().join("bitcoin.tar.gz");
	let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
	let mut header = tar::Header::new_gnu();
	header.set_size(15);
	header.set_mode(0o755);
	header.set_cksum();
	archive
		.append_data(
			&mut header,
			format!("bitcoin-{VERSION}/bin/bitcoind"),
			b"test executable".as_slice(),
		)
		.unwrap();
	let bytes = archive.into_inner().unwrap().finish().unwrap();
	fs::write(&archive_path, &bytes).unwrap();

	let destination = directory.path().join("unpacked");
	fs::create_dir(&destination).unwrap();
	unpack_release(&archive_path, &sha256::Hash::hash(&bytes).to_string(), &destination).unwrap();
	assert_eq!(
		fs::read(destination.join(format!("bitcoin-{VERSION}/bin/bitcoind"))).unwrap(),
		b"test executable"
	);
}

#[test]
fn rejects_modified_downloads_before_unpacking() {
	let directory = tempfile::tempdir().unwrap();
	let archive_path = directory.path().join("bitcoin.tar.gz");
	fs::write(&archive_path, b"modified download").unwrap();
	let destination = directory.path().join("unpacked");
	fs::create_dir(&destination).unwrap();

	let error = unpack_release(&archive_path, &"0".repeat(64), &destination).unwrap_err();
	assert!(error.to_string().contains("checksum mismatch"));
	assert_eq!(fs::read_dir(destination).unwrap().count(), 0);
}
