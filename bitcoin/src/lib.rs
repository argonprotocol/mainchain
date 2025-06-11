#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

pub use cosign_releaser::CosignReleaser;
pub use cosign_script::{Amount, CosignScript, CosignScriptArgs, ReleaseStep};
pub use errors::Error;
#[cfg(feature = "std")]
pub use utxo_spend_filter::{BlockFilter, UtxoSpendFilter};
#[cfg(feature = "std")]
pub use xpriv::*;

pub mod primitives {
	pub use argon_primitives::bitcoin::*;
	pub use bitcoin::bip32::DerivationPath;
}

mod cosign_releaser;
mod cosign_script;

#[cfg(feature = "std")]
pub mod client;
mod errors;
#[cfg(feature = "std")]
mod utxo_spend_filter;
mod xpriv;
