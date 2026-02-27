//! Benchmark-only runtime stubs and helpers.
#![cfg(feature = "runtime-benchmarks")]

use polkadot_sdk::sp_runtime::DispatchError;

use argon_bitcoin::CosignReleaser;
use argon_primitives::bitcoin::{BitcoinSignature, CompressedBitcoinPubkey};
use pallet_bitcoin_locks::BitcoinVerifier;

pub use pallet_prelude::benchmarking::{
	BenchmarkAuthorityProvider, BenchmarkNotaryProvider, BenchmarkNotebookProvider,
	BenchmarkTickProvider,
};

pub struct BenchmarkBitcoinSignatureVerifier;
impl<T: pallet_bitcoin_locks::Config> BitcoinVerifier<T> for BenchmarkBitcoinSignatureVerifier {
	fn verify_signature(
		_utxo_releaser: CosignReleaser,
		_pubkey: CompressedBitcoinPubkey,
		_signature: &BitcoinSignature,
	) -> Result<bool, DispatchError> {
		Ok(true)
	}
}
