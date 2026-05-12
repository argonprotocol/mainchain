// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use alloc::vec::Vec;
use argon_primitives::EthereumBeaconPreset;
use snowbridge_beacon_primitives::decompress_sync_committee_bits as decompress_raw_sync_committee_bits;

const MAINNET_SYNC_COMMITTEE_SIZE: usize = EthereumBeaconPreset::Mainnet.sync_committee_size();
const MAINNET_SYNC_COMMITTEE_BITS_SIZE: usize =
	EthereumBeaconPreset::Mainnet.sync_committee_bits_size();
const MINIMAL_SYNC_COMMITTEE_SIZE: usize = EthereumBeaconPreset::Minimal.sync_committee_size();
const MINIMAL_SYNC_COMMITTEE_BITS_SIZE: usize =
	EthereumBeaconPreset::Minimal.sync_committee_bits_size();

/// Decompress packed bitvector into byte vector according to SSZ deserialization rules. Each byte
/// in the decompressed vector is either 0 or 1.
pub fn decompress_sync_committee_bits(input: &[u8]) -> Option<Vec<u8>> {
	match input.len() {
		MAINNET_SYNC_COMMITTEE_BITS_SIZE => Some(
			decompress_raw_sync_committee_bits::<
				MAINNET_SYNC_COMMITTEE_SIZE,
				MAINNET_SYNC_COMMITTEE_BITS_SIZE,
			>(input.try_into().ok()?)
			.to_vec(),
		),
		MINIMAL_SYNC_COMMITTEE_BITS_SIZE => Some(
			decompress_raw_sync_committee_bits::<
				MINIMAL_SYNC_COMMITTEE_SIZE,
				MINIMAL_SYNC_COMMITTEE_BITS_SIZE,
			>(input.try_into().ok()?)
			.to_vec(),
		),
		_ => None,
	}
}

/// Compute the sync committee period in which a slot is contained.
pub fn compute_period(
	slot: u64,
	slots_per_epoch: u64,
	epochs_per_sync_committee_period: u64,
) -> u64 {
	slot / slots_per_epoch / epochs_per_sync_committee_period
}

/// Compute epoch in which a slot is contained.
pub fn compute_epoch(slot: u64, slots_per_epoch: u64) -> u64 {
	slot / slots_per_epoch
}

/// Sums the bit vector of sync committee participation.
pub fn sync_committee_sum(sync_committee_bits: &[u8]) -> u32 {
	sync_committee_bits.iter().fold(0, |acc: u32, x| acc + *x as u32)
}
