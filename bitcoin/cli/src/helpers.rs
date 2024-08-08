use base58::FromBase58;
use sp_runtime::FixedU128;

use argon_primitives::bitcoin::OpaqueBitcoinXpub;

pub fn read_bitcoin_xpub(xpub: &str) -> Result<OpaqueBitcoinXpub, String> {
	let mut vpub_bytes = xpub.from_base58().map_err(|_| "Invalid Base58 string")?;
	if vpub_bytes.len() == 82 {
		vpub_bytes = vpub_bytes[0..78].to_vec();
	}
	if vpub_bytes.len() != 78 {
		return Err(format!("Invalid byte length ({} should be 78)", vpub_bytes.len()))
	}
	let raw_bytes: [u8; 78] = vpub_bytes.try_into().unwrap();
	Ok(OpaqueBitcoinXpub(raw_bytes))
}

/// Translate a percent out of 100 to a fixed 128-bit number
pub fn read_percent_to_fixed_128(percent: f32) -> FixedU128 {
	FixedU128::from_float(percent as f64).div(FixedU128::from_u32(100))
}
