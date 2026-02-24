use anyhow::anyhow;
use argon_client::{FetchAt, MainchainClient, api::storage};
use argon_primitives::bitcoin::{BitcoinNetwork, OpaqueBitcoinXpub};
use bitcoin::Network;
use polkadot_sdk::*;
use sp_runtime::FixedU128;

pub fn read_bitcoin_xpub(xpub: &str) -> Result<OpaqueBitcoinXpub, String> {
	let vpub_bytes = bitcoin::base58::decode_check(xpub)
		.map_err(|e| format!("Invalid Base58Check string: {e}"))?;
	if vpub_bytes.len() != 78 {
		return Err(format!("Invalid byte length ({} should be 78)", vpub_bytes.len()));
	}
	let raw_bytes: [u8; 78] =
		vpub_bytes.try_into().map_err(|_| "Invalid xpub bytes".to_string())?;
	Ok(OpaqueBitcoinXpub(raw_bytes))
}

/// Translate a percent out of 100 to a fixed 128-bit number
pub fn read_percent_to_fixed_128(percent: f32) -> FixedU128 {
	FixedU128::from_float(percent as f64).div(FixedU128::from_u32(100))
}

pub async fn get_bitcoin_network(
	client: &MainchainClient,
	at_block: FetchAt,
) -> anyhow::Result<Network> {
	let network: BitcoinNetwork = client
		.fetch_storage(&storage().bitcoin_utxos().bitcoin_network(), at_block)
		.await?
		.ok_or(anyhow!("No bitcoin network found"))?
		.into();
	Ok(network.into())
}

#[cfg(test)]
mod tests {
	use super::read_bitcoin_xpub;

	#[test]
	fn read_bitcoin_xpub_requires_valid_base58check() {
		let bytes = [42u8; 78];
		let encoded = bitcoin::base58::encode_check(&bytes);
		let decoded = read_bitcoin_xpub(&encoded).unwrap();
		assert_eq!(decoded.0, bytes);

		let mut invalid_checksum = encoded.clone();
		let last = invalid_checksum.pop().unwrap();
		invalid_checksum.push(if last == '1' { '2' } else { '1' });
		assert!(read_bitcoin_xpub(&invalid_checksum).is_err());
	}

	#[test]
	fn read_bitcoin_xpub_requires_expected_payload_length() {
		let short = bitcoin::base58::encode_check(&[7u8; 77]);
		assert!(read_bitcoin_xpub(&short).is_err());
	}
}
