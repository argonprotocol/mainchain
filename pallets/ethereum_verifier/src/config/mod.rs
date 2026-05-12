// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>

pub mod altair;
pub mod electra;

/// Sizes related to SSZ encoding.
pub const MAX_EXTRA_DATA_BYTES: usize = 32;
pub const MAX_LOGS_BLOOM_SIZE: usize = 256;
pub const MAX_FEE_RECIPIENT_SIZE: usize = 20;

/// Sanity value to constrain the max size of a merkle branch proof.
pub const MAX_BRANCH_PROOF_SIZE: usize = 20;
pub const MAX_BRANCH_PROOF_SIZE_U32: u32 = MAX_BRANCH_PROOF_SIZE as u32;

/// DomainType('0x07000000')
/// <https://github.com/ethereum/consensus-specs/blob/master/specs/altair/beacon-chain.md#domains>
pub const DOMAIN_SYNC_COMMITTEE: [u8; 4] = [7, 0, 0, 0];

/// Validators public keys are 48 bytes.
pub const PUBKEY_SIZE: usize = 48;

/// Signatures produced by validators are 96 bytes.
pub const SIGNATURE_SIZE: usize = 96;

/// The index depth of the `block_roots` field in the beacon state tree.
pub const BLOCK_ROOT_AT_INDEX_DEPTH: usize = 13;
