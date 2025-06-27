#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

pub use cosign_releaser::CosignReleaser;
pub use cosign_script::{Amount, CosignScript, CosignScriptArgs, ReleaseStep};
pub use errors::Error;
#[cfg(feature = "std")]
pub use utxo_spend_filter::{BlockFilter, UtxoSpendFilter};
pub use xpriv::*;

pub mod primitives {
	pub use argon_primitives::bitcoin::*;
	pub use bitcoin::{
		CompressedPublicKey, FeeRate, Network, PrivateKey, Psbt, ScriptBuf, Txid,
		bip32::{DerivationPath, Xpriv, Xpub},
	};
}

mod cosign_releaser;
mod cosign_script;

#[cfg(feature = "std")]
pub mod client;
mod errors;
pub mod psbt_utils;
#[cfg(feature = "std")]
mod utxo_spend_filter;
mod xpriv;
