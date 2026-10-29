//! Layouts pinned to subscriber v1, v4-periphery 545a5d2a87228167edde48f3b9eda122d1e3c4d6,
//! and v4-core 1.0.2.
//! Tests compare these paths with actual Uniswap deployment storage, not caller-supplied offsets.
use crate::{Config, Error, PoolConfig, PoolKey, PositionProof};
use alloy_primitives::{keccak256, Address, B256, I256, U256 as AlloyU256};
use alloy_sol_types::SolValue;
use pallet_prelude::*;

/// Subscriber's `subscriptions` mapping: beneficiary at the base slot, revision at base + 1.
pub const SUBSCRIPTIONS_SLOT: u64 = 0;
/// PositionManager's canonical `subscriber(tokenId)` mapping.
pub const POSITION_MANAGER_SUBSCRIBER_SLOT: u64 = 7;
/// PositionManager's packed `positionInfo(tokenId)` mapping.
pub const POSITION_MANAGER_POSITION_INFO_SLOT: u64 = 9;
/// PoolManager's mapping from full PoolId to pool state.
pub const POOL_MANAGER_POOLS_SLOT: u64 = 6;
/// Offset of the native positions mapping within a pool's state.
pub const POOL_POSITIONS_OFFSET: u64 = 6;

/// Decoded witness values; the caller must still authenticate all three account proofs.
pub(crate) struct DecodedPosition {
	/// Raw AccountId32 bytes recorded by the subscriber.
	pub argon_account_id: H256,
	/// Permanent subscription revision decoded as an Ethereum uint256.
	pub revision: U256,
	/// Inclusive signed int24 lower tick.
	pub tick_lower: i32,
	/// Exclusive signed int24 upper tick.
	pub tick_upper: i32,
	/// Native uint128 Uniswap liquidity value.
	pub liquidity: u128,
}

pub fn pool_id(key: &PoolKey) -> H256 {
	let encoded = (
		Address::from_slice(key.currency0.as_bytes()),
		Address::from_slice(key.currency1.as_bytes()),
		AlloyU256::from(key.fee),
		I256::try_from(key.tick_spacing).expect("i32 fits int256"),
		Address::from_slice(key.hooks.as_bytes()),
	)
		.abi_encode_params();
	H256::from(keccak256(encoded).0)
}

pub fn mapping_slot(key: U256, seed: u64) -> H256 {
	let encoded =
		(AlloyU256::from_be_bytes(key.to_big_endian()), AlloyU256::from(seed)).abi_encode();
	H256::from(keccak256(encoded).0)
}

pub fn offset_slot(slot: H256, offset: u64) -> H256 {
	H256::from(
		U256::from_big_endian(slot.as_bytes())
			.overflowing_add(U256::from(offset))
			.0
			.to_big_endian(),
	)
}

pub fn liquidity_slot(
	pool_id: H256,
	position_manager: H160,
	token_id: U256,
	lower: i32,
	upper: i32,
) -> H256 {
	// PoolManager keys NFT positions by the PositionManager as owner and tokenId as salt.
	let mut position_key = [0u8; 58];
	position_key[..20].copy_from_slice(position_manager.as_bytes());
	position_key[20..23].copy_from_slice(&lower.to_be_bytes()[1..]);
	position_key[23..26].copy_from_slice(&upper.to_be_bytes()[1..]);
	position_key[26..].copy_from_slice(&token_id.to_big_endian());
	let position_key = keccak256(position_key);

	let pool_state_slot =
		mapping_slot(U256::from_big_endian(pool_id.as_bytes()), POOL_MANAGER_POOLS_SLOT);
	let positions_mapping_slot = offset_slot(pool_state_slot, POOL_POSITIONS_OFFSET);

	let encoded = (position_key, B256::from(positions_mapping_slot.0)).abi_encode();
	H256::from(keccak256(encoded).0)
}

pub(crate) fn decode_position<T: Config>(
	config: &PoolConfig,
	pool_id: H256,
	token_id: U256,
	proof: &PositionProof,
) -> Result<DecodedPosition, DispatchError> {
	let [beneficiary_slot, revision_slot] = proof.subscriber.slots.as_slice() else {
		return Err(Error::<T>::InvalidStorageSlots.into());
	};
	let [subscriber_slot, position_info_slot] = proof.position_manager.slots.as_slice() else {
		return Err(Error::<T>::InvalidStorageSlots.into());
	};
	let [liquidity_proof] = proof.pool_manager.slots.as_slice() else {
		return Err(Error::<T>::InvalidStorageSlots.into());
	};

	let subscription_slot = mapping_slot(token_id, SUBSCRIPTIONS_SLOT);
	ensure!(beneficiary_slot.slot == subscription_slot, Error::<T>::InvalidStorageSlots);
	ensure!(
		revision_slot.slot == offset_slot(subscription_slot, 1),
		Error::<T>::InvalidStorageSlots
	);
	ensure!(
		subscriber_slot.slot == mapping_slot(token_id, POSITION_MANAGER_SUBSCRIBER_SLOT),
		Error::<T>::InvalidStorageSlots
	);
	ensure!(
		position_info_slot.slot == mapping_slot(token_id, POSITION_MANAGER_POSITION_INFO_SLOT),
		Error::<T>::InvalidStorageSlots
	);

	let mut expected_subscriber = [0u8; 32];
	expected_subscriber[12..].copy_from_slice(config.subscriber.as_bytes());
	ensure!(
		subscriber_slot.value == H256::from(expected_subscriber),
		Error::<T>::InvalidSubscription
	);
	ensure!(position_info_slot.value[31] == 1, Error::<T>::InvalidSubscription);
	ensure!(revision_slot.value != H256::zero(), Error::<T>::InvalidSubscription);

	// PositionManager retains only the first 25 bytes of the native PoolId.
	let position_info = position_info_slot.value;
	ensure!(position_info[..25] == pool_id[..25], Error::<T>::InvalidPosition);

	let tick_lower = signed_tick(&position_info[28..31]);
	let tick_upper = signed_tick(&position_info[25..28]);
	ensure!((-887_272..=887_272).contains(&tick_lower), Error::<T>::InvalidPosition);
	ensure!((-887_272..=887_272).contains(&tick_upper), Error::<T>::InvalidPosition);
	ensure!(tick_lower < tick_upper, Error::<T>::InvalidPosition);
	ensure!(tick_lower % config.key.tick_spacing == 0, Error::<T>::InvalidPosition);
	ensure!(tick_upper % config.key.tick_spacing == 0, Error::<T>::InvalidPosition);

	ensure!(
		liquidity_proof.slot ==
			liquidity_slot(pool_id, config.position_manager, token_id, tick_lower, tick_upper),
		Error::<T>::InvalidStorageSlots
	);
	ensure!(liquidity_proof.value[..16] == [0u8; 16], Error::<T>::InvalidPosition);

	Ok(DecodedPosition {
		argon_account_id: beneficiary_slot.value,
		revision: U256::from_big_endian(revision_slot.value.as_bytes()),
		tick_lower,
		tick_upper,
		liquidity: u128::from_be_bytes(
			liquidity_proof.value[16..].try_into().expect("sixteen bytes"),
		),
	})
}

fn signed_tick(bytes: &[u8]) -> i32 {
	let sign_extension = if bytes[0] & 0x80 != 0 { 0xff } else { 0 };
	i32::from_be_bytes([sign_extension, bytes[0], bytes[1], bytes[2]])
}
