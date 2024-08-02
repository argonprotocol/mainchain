#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

pub use cosign_script::{Amount, CosignScript, CosignScriptArgs, UnlockStep};
pub use errors::Error;
#[cfg(feature = "std")]
pub use utxo_spend_filter::{BlockFilter, UtxoSpendFilter};
pub use utxo_unlocker::UtxoUnlocker;

mod cosign_script;
mod utxo_unlocker;

mod errors;
#[cfg(feature = "std")]
mod utxo_spend_filter;
