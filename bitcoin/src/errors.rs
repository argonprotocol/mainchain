use alloc::{boxed::Box, string::String};
use argon_primitives::bitcoin::BitcoinError;
use bitcoin::{
	bip32, psbt,
	psbt::{ExtractTxError, SignError, SigningErrors},
};
use polkadot_sdk::*;
use sp_runtime::RuntimeDebug;

#[derive(RuntimeDebug, thiserror::Error)]
pub enum Error {
	/// Fees overflowed
	#[error("The fees overflowed.")]
	FeeOverflow,

	/// Insufficient fees
	#[error("Insufficient fees.")]
	FeeTooLow,

	/// An error occurred creating the timelock multisig script
	#[error("An error occurred creating the timelock multisig script. {0:?}")]
	TimelockScriptError(Box<BitcoinError>),

	/// Could not create an address
	#[error("Could not create an address.")]
	AddressError,

	/// Could not sign with derived key
	#[error("Could not sign with derived key.")]
	DerivedKeySignError,

	/// Signature Expected
	#[error("Signature Expected.")]
	SignatureExpected,

	/// Signing Errors
	#[error("Signing Errors {0:?}.")]
	SigningErrors(Box<SigningErrors>),

	/// Sign Error
	#[error("Sign Error {0:?}.")]
	SignError(Box<SignError>),

	/// Bip32 Error
	#[error("Bip32 Error {0}.")]
	Bip32Error(Box<bip32::Error>),

	/// Unknown pubkey hash in partial sigs
	#[error("Unknown pubkey hash in partial sigs")]
	UnknownPubkeyHash,

	/// Could not extract tx
	#[error("Could not extract tx {0:?}")]
	ExtractTxError(Box<ExtractTxError>),

	/// Partially Signed Bitcoin Transaction Error
	#[error("Partially Signed Bitcoin Transaction Error {0:?}")]
	PsbtError(Box<psbt::Error>),

	/// Psbt Finalize Error
	#[error("Psbt Finalize Error")]
	PsbtFinalizeError,

	/// Invalid Signature Bytes
	#[error("Invalid Signature Bytes")]
	InvalidSignatureBytes,

	/// Invalid Compressed Pubkey Bytes
	#[error("Invalid Compressed Pubkey Bytes")]
	InvalidCompressPubkeyBytes,

	/// Broadcast Error
	#[error("Broadcast Error {0:?}")]
	BroadcastError(String),
}

impl From<BitcoinError> for Error {
	fn from(err: BitcoinError) -> Self {
		Error::TimelockScriptError(Box::new(err))
	}
}
impl From<SigningErrors> for Error {
	fn from(err: SigningErrors) -> Self {
		Error::SigningErrors(Box::new(err))
	}
}
impl From<SignError> for Error {
	fn from(err: SignError) -> Self {
		Error::SignError(Box::new(err))
	}
}
impl From<bip32::Error> for Error {
	fn from(err: bip32::Error) -> Self {
		Error::Bip32Error(Box::new(err))
	}
}
impl From<ExtractTxError> for Error {
	fn from(err: ExtractTxError) -> Self {
		Error::ExtractTxError(Box::new(err))
	}
}

impl From<psbt::Error> for Error {
	fn from(err: psbt::Error) -> Self {
		Error::PsbtError(Box::new(err))
	}
}

#[cfg(feature = "std")]
impl From<Error> for String {
	fn from(error: Error) -> String {
		error.to_string()
	}
}
