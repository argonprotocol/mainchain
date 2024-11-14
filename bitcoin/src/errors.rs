use bitcoin::{
	bip32, psbt,
	psbt::{ExtractTxError, SignError, SigningErrors},
};
use sp_runtime::RuntimeDebug;

use argon_primitives::bitcoin::BitcoinError;

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
	TimelockScriptError(BitcoinError),

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
	SigningErrors(SigningErrors),

	/// Sign Error
	#[error("Sign Error {0:?}.")]
	SignError(SignError),

	/// Bip32 Error
	#[error("Bip32 Error {0}.")]
	Bip32Error(bip32::Error),

	/// Unknown pubkey hash in partial sigs
	#[error("Unknown pubkey hash in partial sigs")]
	UnknownPubkeyHash,

	/// Could not extract tx
	#[error("Could not extract tx {0:?}")]
	ExtractTxError(ExtractTxError),

	/// Partially Signed Bitcoin Transaction Error
	#[error("Partially Signed Bitcoin Transaction Error {0:?}")]
	PsbtError(psbt::Error),

	/// Psbt Finalize Error
	#[error("Psbt Finalize Error")]
	PsbtFinalizeError,

	/// Invalid Signature Bytes
	#[error("Invalid Signature Bytes")]
	InvalidSignatureBytes,

	/// Invalid Compressed Pubkey Bytes
	#[error("Invalid Compressed Pubkey Bytes")]
	InvalidCompressPubkeyBytes,
}

impl From<BitcoinError> for Error {
	fn from(err: BitcoinError) -> Self {
		Error::TimelockScriptError(err)
	}
}
impl From<SigningErrors> for Error {
	fn from(err: SigningErrors) -> Self {
		Error::SigningErrors(err)
	}
}
impl From<bip32::Error> for Error {
	fn from(err: bip32::Error) -> Self {
		Error::Bip32Error(err)
	}
}
impl From<ExtractTxError> for Error {
	fn from(err: ExtractTxError) -> Self {
		Error::ExtractTxError(err)
	}
}

impl From<psbt::Error> for Error {
	fn from(err: psbt::Error) -> Self {
		Error::PsbtError(err)
	}
}

#[cfg(feature = "std")]
impl From<Error> for String {
	fn from(error: Error) -> String {
		error.to_string()
	}
}
