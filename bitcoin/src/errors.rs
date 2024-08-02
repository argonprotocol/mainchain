use bitcoin::{
	bip32, psbt,
	psbt::{ExtractTxError, SignError, SigningErrors},
};
use sp_runtime::RuntimeDebug;

use ulx_primitives::bitcoin::BitcoinError;

#[derive(RuntimeDebug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum Error {
	/// Fees overflowed
	#[cfg_attr(feature = "std", error("The fees overflowed."))]
	FeeOverflow,

	/// Insufficient fees
	#[cfg_attr(feature = "std", error("Insufficient fees."))]
	FeeTooLow,

	/// An error occurred creating the timelock multisig script
	#[cfg_attr(
		feature = "std",
		error("An error occurred creating the timelock multisig script. {0:?}")
	)]
	TimelockScriptError(BitcoinError),

	/// Could not create an address
	#[cfg_attr(feature = "std", error("Could not create an address."))]
	AddressError,

	/// Could not sign with derived key
	#[cfg_attr(feature = "std", error("Could not sign with derived key."))]
	DerivedKeySignError,

	/// Signature Expected
	#[cfg_attr(feature = "std", error("Signature Expected."))]
	SignatureExpected,

	/// Signing Errors
	#[cfg_attr(feature = "std", error("Signing Errors {0:?}."))]
	SigningErrors(SigningErrors),

	/// Sign Error
	#[cfg_attr(feature = "std", error("Sign Error {0:?}."))]
	SignError(SignError),

	/// Bip32 Error
	#[cfg_attr(feature = "std", error("Bip32 Error {0}."))]
	Bip32Error(bip32::Error),

	/// Unknown pubkey hash in partial sigs
	#[cfg_attr(feature = "std", error("Unknown pubkey hash in partial sigs"))]
	UnknownPubkeyHash,

	/// Could not extract tx
	#[cfg_attr(feature = "std", error("Could not extract tx {0:?}"))]
	ExtractTxError(ExtractTxError),

	/// Partially Signed Bitcoin Transaction Error
	#[cfg_attr(feature = "std", error("Partially Signed Bitcoin Transaction Error {0:?}"))]
	PsbtError(psbt::Error),

	/// Psbt Finalize Error
	#[cfg_attr(feature = "std", error("Psbt Finalize Error"))]
	PsbtFinalizeError,

	/// Invalid Signature Bytes
	#[cfg_attr(feature = "std", error("Invalid Signature Bytes"))]
	InvalidSignatureBytes,

	/// Invalid Compressed Pubkey Bytes
	#[cfg_attr(feature = "std", error("Invalid Compressed Pubkey Bytes"))]
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
