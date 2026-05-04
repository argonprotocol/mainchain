// SPDX-License-Identifier: Apache-2.0
//! Imported upstream verification helpers.
//!
//! Code in this module should stay close to its recorded upstream source. Argon-specific provider
//! glue belongs in the parent pallet modules that call into these helpers.

pub(crate) mod receipt;
pub(crate) mod verification;
