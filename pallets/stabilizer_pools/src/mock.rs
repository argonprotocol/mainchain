//! Shared setup for tests and runtime benchmarks; excluded from ordinary runtime builds.
//! Ethereum finality anchors are supplied here, while account/storage proofs use the real verifier.
#[cfg(test)]
use crate as pallet_stabilizer_pools;
use crate::*;
#[cfg(test)]
use alloy_primitives::aliases::I24;
use alloy_primitives::{keccak256, B256, U256 as AlloyU256};
use alloy_trie::{
	proof::{ProofNodes, ProofRetainer},
	HashBuilder, Nibbles, TrieAccount,
};
use argon_primitives::{
	EthereumExecutionBlockProof, EthereumStorageSlotProof, StabilizerPoolPriceSource,
	UniswapPoolIdentity,
};
#[cfg(test)]
use argon_primitives::{OnNewSlot, PriceIndex, PriceIndexUpdated, UniswapSqrtPriceX96};
#[cfg(test)]
use core::ops::Range;
#[cfg(test)]
use frame_support::{assert_ok, derive_impl};
use pallet_ethereum_verifier::{types::ExecutionHeaderAnchor, ExecutionHeaderAnchors};
#[cfg(test)]
use pallet_prelude::{new_test_with_genesis, TestState};
#[cfg(test)]
use polkadot_sdk::sp_core::U512;
#[cfg(test)]
use sp_runtime::{traits::IdentityLookup, AccountId32};
#[cfg(test)]
use uniswap_v3_sdk_bigint::utils::{
	encode_sqrt_ratio_x96, get_amount_0_delta, get_amount_1_delta, get_sqrt_ratio_at_tick,
	get_tick_at_sqrt_ratio, ToBig,
};

#[cfg(test)]
type Block = frame_system::mocking::MockBlock<Test>;

#[cfg(test)]
frame_support::construct_runtime!(pub enum Test {
	System: frame_system,
	EthereumVerifier: pallet_ethereum_verifier,
	StabilizerPools: pallet_stabilizer_pools,
});

#[cfg(test)]
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
}

#[cfg(test)]
impl pallet_ethereum_verifier::Config for Test {
	type FreeHeadersInterval = ConstU32<32>;
	type WeightInfo = ();
}

#[cfg(test)]
impl Config for Test {
	type WeightInfo = ();
	type CurrentTick = CurrentTick;
	type MaxSampledPriceHistoryFrames = ConstU32<13>;
	type TicksPerFrame = ConstU64<10>;
	type RewardPriceBand = RewardPriceBand;
	type MaxPriceBucketsPerFrame = MaxPriceBucketsPerFrame;
	type MinPriceBucketCoverage = MinPriceBucketCoverage;
}

#[cfg(test)]
frame_support::parameter_types! {
	pub const RewardPriceBand: (Percent, Percent) = (Percent::from_percent(90), Percent::one());
	pub const MaxPriceBucketsPerFrame: u32 = 24;
	pub const MinPriceBucketCoverage: Percent = Percent::from_percent(75);
	pub static CurrentTick: Tick = 20;
}

#[cfg(test)]
pub fn new_test_ext() -> TestState {
	let mut ext = new_test_with_genesis::<Test>(|_| {});
	ext.execute_with(|| {
		System::set_block_number(1);
		assert_ok!(StabilizerPools::register_pool(RuntimeOrigin::root(), pool_manager(), config()));
	});
	ext
}

#[cfg(test)]
pub(crate) fn start_mining_frame(frame_id: FrameId, tick: Tick) {
	CurrentTick::set(tick);
	StabilizerPools::on_frame_start(frame_id);
}

#[cfg(test)]
pub(crate) fn report_stabilizer_pool_price(tick: Tick, sqrt_price_x96: UniswapSqrtPriceX96) {
	StabilizerPools::on_price_index(
		&stabilizer_pool_price_index(tick, sqrt_price_x96),
		Some(stabilizer_pool_price_source()),
	);
}

pub(crate) fn stabilizer_pool_price_source() -> StabilizerPoolPriceSource {
	StabilizerPoolPriceSource {
		revision: 1,
		pool: UniswapPoolIdentity { chain_id: 1, pool_manager: pool_manager(), pool_id: pool_id() },
	}
}

#[cfg(test)]
pub(crate) fn stabilizer_pool_price_index(
	tick: Tick,
	sqrt_price_x96: UniswapSqrtPriceX96,
) -> PriceIndex {
	PriceIndex {
		tick,
		stabilizer_pool_sqrt_price_x96: Some(sqrt_price_x96),
		btc_usd_price: FixedU128::from_u32(60_000),
		argonot_usd_price: FixedU128::from_u32(2),
		argon_usd_price: FixedU128::one(),
		argon_usd_target_price: FixedU128::one(),
		argon_time_weighted_average_liquidity: 1_000,
		usdc_usd_price: Some(FixedU128::one()),
	}
}

pub(crate) fn pool_manager() -> H160 {
	H160::repeat_byte(0x33)
}

pub(crate) fn config() -> PoolConfig {
	PoolConfig {
		position_manager: H160::repeat_byte(0x22),
		subscriber: H160::repeat_byte(0x11),
		key: PoolKey {
			currency0: H160::repeat_byte(0x44),
			currency1: H160::repeat_byte(0x55),
			fee: 500,
			tick_spacing: 10,
			hooks: H160::zero(),
		},
	}
}

pub(crate) fn pool_id() -> H256 {
	evm::pool_id(&config().key)
}

#[cfg(test)]
pub(crate) fn position_checkpoint(
	liquidity: u128,
	tick_lower: UniswapTick,
	tick_upper: UniswapTick,
) -> PositionCheckpoint<AccountId32> {
	let block =
		ProofBlock { hash: H256::from_low_u64_be(100), number: 100, timestamp_millis: 1_200_000 };
	PositionCheckpoint {
		beneficiary: AccountId32::new([0x42; 32]),
		pool_manager: pool_manager(),
		pool_id: pool_id(),
		subscriber: config().subscriber,
		revision: U256::one(),
		tick_lower,
		tick_upper,
		liquidity,
		maintained_since: block,
		proven_through: block,
	}
}

/// Convert a named Uniswap price tick to the raw sqrt-price format accepted by the pallet.
#[cfg(test)]
pub(crate) fn uniswap_sqrt_price_at_tick(uniswap_tick: UniswapTick) -> UniswapSqrtPriceX96 {
	let sqrt_price = get_sqrt_ratio_at_tick(I24::try_from(uniswap_tick).unwrap()).unwrap();
	U256::from_big_endian(&sqrt_price.to_be_bytes::<20>())
}

/// Independent principal reference from the pinned Uniswap SDK, which accepts one NFT's liquidity.
#[cfg(test)]
pub(crate) fn uniswap_sdk_principal(
	liquidity: u128,
	sqrt_range: Range<UniswapSqrtPriceX96>,
	sqrt_price_x96: UniswapSqrtPriceX96,
	argon_is_token0: bool,
) -> PositionAmounts<U256> {
	let sqrt_lower = AlloyU256::from_be_bytes(sqrt_range.start.to_big_endian());
	let sqrt_upper = AlloyU256::from_be_bytes(sqrt_range.end.to_big_endian());
	let sqrt_price =
		AlloyU256::from_be_bytes(sqrt_price_x96.to_big_endian()).clamp(sqrt_lower, sqrt_upper);
	let token0 = get_amount_0_delta(sqrt_price, sqrt_upper, liquidity, false).unwrap();
	let token1 = get_amount_1_delta(sqrt_lower, sqrt_price, liquidity, false).unwrap();
	let (argon, usdc) = if argon_is_token0 { (token0, token1) } else { (token1, token0) };

	let argon_token_units_per_microgon = U256::from(10u128.pow(12));
	PositionAmounts {
		argon: U256::from_big_endian(&argon.to_be_bytes::<32>()) / argon_token_units_per_microgon,
		usdc: U256::from_big_endian(&usdc.to_be_bytes::<32>()),
	}
}

/// Use the SDK's arbitrary-precision encoder as the reference for USD-to-pool-price conversion.
#[cfg(test)]
pub(crate) fn uniswap_sqrt_price_for_usd_price(
	argon_usd_price: FixedU128,
	usdc_usd_price: FixedU128,
	argon_is_token0: bool,
	round_up: bool,
) -> UniswapSqrtPriceX96 {
	let argon = AlloyU256::from(argon_usd_price.into_inner());
	let usdc = AlloyU256::from(usdc_usd_price.into_inner()) * AlloyU256::from(10u128.pow(12));
	let (numerator, denominator) = if argon_is_token0 { (argon, usdc) } else { (usdc, argon) };
	let sqrt_price: AlloyU256 =
		encode_sqrt_ratio_x96(numerator.to_big_int(), denominator.to_big_int());
	let mut sqrt_price = U256::from_big_endian(&sqrt_price.to_be_bytes::<32>());
	if round_up {
		let numerator = U512::from_big_endian(&numerator.to_be_bytes::<32>()) << 192;
		let denominator = U512::from_big_endian(&denominator.to_be_bytes::<32>());
		if U512::from(sqrt_price) * U512::from(sqrt_price) * denominator < numerator {
			sqrt_price += U256::one();
		}
	}
	sqrt_price
}

/// Round both requested USD boundaries down to usable grid ticks, as an app would when minting.
#[cfg(test)]
pub(crate) fn uniswap_bounds_for_usd_band(
	lower_usd_price: FixedU128,
	upper_usd_price: FixedU128,
	usdc_usd_price: FixedU128,
	argon_is_token0: bool,
	uniswap_tick_spacing: i32,
) -> (UniswapTick, UniswapTick) {
	let mut ticks = [0, 0];
	for (index, usd_price) in [lower_usd_price, upper_usd_price].into_iter().enumerate() {
		let sqrt_price = uniswap_sqrt_price_for_usd_price(
			usd_price,
			usdc_usd_price,
			argon_is_token0,
			!argon_is_token0,
		);
		let raw_uniswap_tick =
			get_tick_at_sqrt_ratio(AlloyU256::from_be_bytes(sqrt_price.to_big_endian()))
				.unwrap()
				.as_i32();
		let mut aligned_tick =
			raw_uniswap_tick.div_euclid(uniswap_tick_spacing) * uniswap_tick_spacing;
		if !argon_is_token0 {
			// USD rises as raw Uniswap ticks fall, so USD round-down uses the next higher grid
			// tick.
			if uniswap_sqrt_price_at_tick(aligned_tick) < sqrt_price {
				aligned_tick += uniswap_tick_spacing;
			}
		}
		ticks[index] = aligned_tick;
	}
	if argon_is_token0 {
		(ticks[0], ticks[1])
	} else {
		(ticks[1], ticks[0])
	}
}

/// Synthetic Ethereum state, real account/storage tries and the real configured verifier.
/// Slot paths are also checked against deployed Uniswap bytecode in the Solidity package tests.
pub(crate) fn evidence<T: Config>(
	block: u64,
	revision: u64,
	beneficiary: H256,
	liquidity: u128,
	subscribed: bool,
) -> PositionProof {
	let pool_config = config();

	let token_id = U256::one();
	let subscription_slot = evm::mapping_slot(token_id, evm::SUBSCRIPTIONS_SLOT);

	// Pack Uniswap's pool prefix, signed tick bounds, and subscription flag.
	let mut position_info = pool_id().0;
	position_info[25..28].copy_from_slice(&100i32.to_be_bytes()[1..]);
	position_info[28..31].copy_from_slice(&(-100i32).to_be_bytes()[1..]);
	position_info[31] = u8::from(subscribed);

	let mut subscriber_value = [0u8; 32];
	if subscribed {
		subscriber_value[12..].copy_from_slice(pool_config.subscriber.as_bytes());
	}

	// Each account contributes its own storage trie to the shared Ethereum state.
	let slots = [
		vec![
			(subscription_slot, beneficiary),
			(
				evm::offset_slot(subscription_slot, 1),
				H256::from(U256::from(revision).to_big_endian()),
			),
		],
		vec![
			(
				evm::mapping_slot(token_id, evm::POSITION_MANAGER_SUBSCRIBER_SLOT),
				H256::from(subscriber_value),
			),
			(
				evm::mapping_slot(token_id, evm::POSITION_MANAGER_POSITION_INFO_SLOT),
				H256::from(position_info),
			),
		],
		vec![(
			evm::liquidity_slot(pool_id(), pool_config.position_manager, token_id, -100, 100),
			H256::from(U256::from(liquidity).to_big_endian()),
		)],
	];

	let addresses = [pool_config.subscriber, pool_config.position_manager, pool_manager()];
	let storage_tries = slots
		.iter()
		.map(|entries| {
			// Ethereum omits zero storage values; their witnesses prove absence.
			let encoded = entries
				.iter()
				.filter(|(_, value)| *value != H256::zero())
				.map(|(slot, value)| {
					(
						slot.as_bytes().to_vec(),
						alloy_rlp::encode(AlloyU256::from_be_slice(value.as_bytes())),
					)
				})
				.collect::<Vec<_>>();
			trie(&encoded)
		})
		.collect::<Vec<_>>();

	let encoded_accounts = addresses
		.iter()
		.zip(&storage_tries)
		.map(|(address, (root, _))| {
			(
				address.as_bytes().to_vec(),
				alloy_rlp::encode(TrieAccount {
					nonce: 1,
					storage_root: *root,
					..Default::default()
				}),
			)
		})
		.collect::<Vec<_>>();

	let (state_root, account_nodes) = trie(&encoded_accounts);

	// Supply finality at the external-chain boundary; trie verification remains real.
	let anchor = ExecutionHeaderAnchor {
		block_hash: H256::from(U256::from(block).to_big_endian()),
		block_number: block,
		timestamp_millis: block * 12_000,
		parent_hash: H256::zero(),
		state_root: H256::from(state_root.0),
		receipts_root: H256::zero(),
	};
	ExecutionHeaderAnchors::<T>::insert(anchor.block_hash, anchor);

	PositionProof {
		subscriber: account_storage_proof(
			addresses[0],
			&account_nodes,
			&storage_tries[0].1,
			&slots[0],
			block,
		),
		position_manager: account_storage_proof(
			addresses[1],
			&account_nodes,
			&storage_tries[1].1,
			&slots[1],
			block,
		),
		pool_manager: account_storage_proof(
			addresses[2],
			&account_nodes,
			&storage_tries[2].1,
			&slots[2],
			block,
		),
	}
}

fn trie(entries: &[(Vec<u8>, Vec<u8>)]) -> (B256, ProofNodes) {
	let mut entries = entries
		.iter()
		.map(|(key, value)| (Nibbles::unpack(keccak256(key)), value))
		.collect::<Vec<_>>();
	entries.sort_by_key(|(key, _)| *key);

	let mut trie = HashBuilder::default()
		.with_proof_retainer(ProofRetainer::new(entries.iter().map(|(key, _)| *key).collect()));
	for (key, value) in entries {
		trie.add_leaf(key, value);
	}

	let root = trie.root();
	(root, trie.take_proof_nodes())
}

fn account_storage_proof<MaxSlots: Get<u32>>(
	address: H160,
	account_nodes: &ProofNodes,
	storage_nodes: &ProofNodes,
	slots: &[(H256, H256)],
	block: u64,
) -> EthereumAccountStorageProof<MaxSlots> {
	let account_key = Nibbles::unpack(keccak256(address));
	let account_proof = account_nodes
		.matching_nodes_sorted(&account_key)
		.into_iter()
		.map(|(_, node)| node.to_vec().try_into().unwrap())
		.collect::<Vec<_>>()
		.try_into()
		.unwrap();

	let sorted_nodes = storage_nodes.nodes_sorted();
	let storage_proof = sorted_nodes
		.iter()
		.map(|(_, node)| node.to_vec().try_into().unwrap())
		.collect::<Vec<_>>()
		.try_into()
		.unwrap();

	let slots = slots
		.iter()
		.map(|(slot, value)| {
			let storage_key = Nibbles::unpack(keccak256(slot));
			let node_indexes = sorted_nodes
				.iter()
				.enumerate()
				.filter(|(_, (path, _))| storage_key.starts_with(path))
				.map(|(index, _)| index as u16)
				.collect::<Vec<_>>()
				.try_into()
				.unwrap();

			EthereumStorageSlotProof { slot: *slot, value: *value, node_indexes }
		})
		.collect::<Vec<_>>()
		.try_into()
		.unwrap();

	EthereumAccountStorageProof {
		execution_block_proof: EthereumExecutionBlockProof {
			anchor_block_hash: H256::from(U256::from(block).to_big_endian()),
			target_to_anchor_header_chain: Default::default(),
		},
		account_proof,
		storage_proof,
		slots,
	}
}
