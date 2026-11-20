//! Shared setup for tests and runtime benchmarks; excluded from ordinary runtime builds.
//! Tests supply Ethereum anchors here so that the real verifier can check the storage proofs.
#[cfg(test)]
use crate as pallet_stabilizer_pools;
use crate::*;
#[cfg(test)]
use alloy_consensus::Header as EthereumHeader;
#[cfg(test)]
use alloy_primitives::aliases::I24;
use alloy_primitives::{keccak256, B256, U256 as AlloyU256};
use alloy_trie::{
	proof::{ProofNodes, ProofRetainer},
	HashBuilder, Nibbles, TrieAccount,
};
#[cfg(test)]
use argon_primitives::{ArgonCPI, OnNewSlot, PriceIndex, PriceIndexUpdated, UniswapSqrtPriceX96};
use argon_primitives::{
	EthereumExecutionBlockProof, EthereumStorageSlotProof, StabilizerPoolPriceSource,
	UniswapPoolIdentity,
};
#[cfg(test)]
use core::ops::Range;
#[cfg(test)]
use frame_support::{assert_ok, derive_impl, traits::Time};
#[cfg(test)]
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
	type EthereumVerifier = EthereumVerifier;
	type PriceProvider = MockPriceProvider;
	type RewardProvider = MockRewardProvider;
	type TreasuryPositionProvider = ();
	type MaxPositionsPerAccount = ConstU32<5>;
	type UsdcRewardShare = UsdcRewardShare;
	type WeightInfo = ();
	type MiningFrameTransitionProvider = CurrentFrame;
	type CurrentTick = CurrentTick;
	type TimeProvider = CurrentTime;
	type MaxSampledPriceHistoryFrames = ConstU32<13>;
	type ClaimWindowFrames = ConstU32<10>;
	type ContinuationGraceMillis = ConstU64<300_000>;
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
	pub static ArgonMarketPrice: Option<FixedU128> = Some(FixedU128::one());
	pub static ArgonTargetPrice: Option<FixedU128> = Some(FixedU128::one());
	pub static CurrentTick: Tick = 20;
	pub static CurrentTime: Moment = 0;
	pub static CurrentFrame: FrameId = 1;
	pub static PaidClaims: Vec<PaidClaim> = Vec::new();
	pub static RejectClaims: bool = false;
	pub static UsdcRewardShare: Percent = Percent::from_percent(90);
	pub static FinalizedRewardValues: Vec<(FrameId, Balance)> = Vec::new();
}

#[cfg(test)]
#[derive(Clone)]
pub struct PaidClaim {
	pub frames: Vec<StabilizerPoolFrameClaim>,
}

#[cfg(test)]
pub struct MockPriceProvider;

#[cfg(test)]
impl PriceProvider<Balance> for MockPriceProvider {
	type Weights = ();
	fn get_latest_argon_price_in_usd() -> Option<FixedU128> {
		ArgonMarketPrice::get()
	}
	fn get_target_argon_price_in_usd() -> Option<FixedU128> {
		ArgonTargetPrice::get()
	}
	fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
		None
	}
	fn get_argonot_price_in_usd() -> Option<FixedU128> {
		None
	}
	fn get_argon_cpi() -> Option<ArgonCPI> {
		None
	}
	fn get_average_cpi_for_ticks(_: (Tick, Tick)) -> ArgonCPI {
		ArgonCPI::zero()
	}
	fn get_circulation() -> Balance {
		0
	}
	fn get_redemption_r_value() -> Option<FixedU128> {
		None
	}
}

#[cfg(test)]
pub struct MockRewardProvider;

#[cfg(test)]
impl StabilizerPoolRewardProvider for MockRewardProvider {
	type Weights = ();

	fn finalize_frame_rewards(frame_id: FrameId, eligible_value: Balance) {
		FinalizedRewardValues::mutate(|frames| frames.push((frame_id, eligible_value)));
	}

	fn frame_reward_rate(frame_id: FrameId) -> Option<FixedU128> {
		FinalizedRewardValues::get()
			.iter()
			.any(|(frame, _)| *frame == frame_id)
			.then_some(FixedU128::one())
	}
	fn claim_rewards(frames: &[StabilizerPoolFrameClaim]) -> Result<Balance, DispatchError> {
		if RejectClaims::get() {
			return Err(DispatchError::Unavailable);
		}
		PaidClaims::mutate(|claims| {
			claims.push(PaidClaim { frames: frames.to_vec() });
		});
		Ok(1)
	}
}

#[cfg(test)]
impl Time for CurrentTime {
	type Moment = Moment;
	fn now() -> Moment {
		Self::get()
	}
}

#[cfg(test)]
impl MiningFrameTransitionProvider for CurrentFrame {
	type Weights = ();
	fn get_current_frame_id() -> FrameId {
		Self::get()
	}
	fn is_new_frame_started() -> Option<FrameId> {
		None
	}
}

#[cfg(test)]
pub fn new_test_ext() -> TestState {
	let mut ext = new_test_with_genesis::<Test>(|_| {});
	ext.execute_with(|| {
		System::set_block_number(1);
		ArgonMarketPrice::set(Some(FixedU128::one()));
		ArgonTargetPrice::set(Some(FixedU128::one()));
		CurrentFrame::set(1);
		PaidClaims::set(Vec::new());
		RejectClaims::set(false);
		UsdcRewardShare::set(Percent::from_percent(90));
		FinalizedRewardValues::set(Vec::new());
		assert_ok!(StabilizerPools::register_pool(RuntimeOrigin::root(), pool_manager(), config()));
	});
	ext
}

#[cfg(test)]
pub(crate) fn start_mining_frame(frame_id: FrameId, tick: Tick) {
	CurrentTick::set(tick);
	CurrentTime::set(tick.saturating_mul(60_000));
	CurrentFrame::set(frame_id);
	StabilizerPools::on_frame_start(frame_id);
}

#[cfg(test)]
pub(crate) fn report_stabilizer_pool_price(tick: Tick, sqrt_price_x96: UniswapSqrtPriceX96) {
	StabilizerPools::on_price_index(
		&stabilizer_pool_price_index(tick, sqrt_price_x96),
		Some(stabilizer_pool_price_source()),
	);
}

#[cfg(test)]
pub(crate) fn sample_reward_ticks(ticks: Range<Tick>) {
	let spot = uniswap_sqrt_price_for_usd_price(
		FixedU128::from_rational(95, 100),
		FixedU128::one(),
		true,
		false,
	);
	for tick in ticks {
		CurrentTick::set(tick);
		report_stabilizer_pool_price(tick, spot);
	}
}

#[cfg(test)]
pub(crate) fn finalized_reward_value(frame_id: FrameId) -> Balance {
	FinalizedRewardValues::get()
		.into_iter()
		.find(|(frame, _)| *frame == frame_id)
		.expect("frame has finalized")
		.1
}

#[cfg(test)]
pub(crate) fn enroll_reward_position() -> (StabilizerPoolPositionId, PositionCheckpoint<AccountId32>)
{
	let (lower, upper) = uniswap_bounds_for_usd_band(
		FixedU128::from_rational(90, 100),
		FixedU128::one(),
		FixedU128::one(),
		true,
		10,
	);
	let source = StabilizerPoolPositionId {
		position_manager: config().position_manager,
		uniswap_nft_id: U256::one(),
	};
	let mut checkpoint = position_checkpoint(10u128.pow(24), lower, upper);
	checkpoint.maintained_since_block = ethereum_proof_block(95);
	checkpoint.proven_through_block = checkpoint.maintained_since_block;
	assert_ok!(checkpoint_nft(source.uniswap_nft_id, &checkpoint));
	(source, checkpoint)
}

#[cfg(test)]
pub(crate) fn checkpoint_nft(
	token_id: UniswapNftId,
	checkpoint: &PositionCheckpoint<AccountId32>,
) -> DispatchResult {
	StabilizerPools::checkpoint_position(
		RuntimeOrigin::signed(checkpoint.beneficiary.clone()),
		checkpoint.pool_manager,
		checkpoint.pool_id,
		token_id,
		Box::new(position_evidence_for_token(token_id, checkpoint)),
	)
}

#[cfg(test)]
pub(crate) fn continue_with_proof(
	source: StabilizerPoolPositionId,
	replacement: UniswapNftId,
	proof: PositionContinuationProof,
) -> DispatchResult {
	StabilizerPools::continue_enrollment(
		RuntimeOrigin::signed(AccountId32::new([0x42; 32])),
		source,
		replacement,
		Box::new(proof),
	)
}

pub(crate) fn stabilizer_pool_price_source() -> StabilizerPoolPriceSource {
	StabilizerPoolPriceSource {
		revision: 1,
		argon_is_token0: true,
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
	let block = ethereum_proof_block(100);
	PositionCheckpoint {
		beneficiary: AccountId32::new([0x42; 32]),
		pool_manager: pool_manager(),
		pool_id: pool_id(),
		subscriber: config().subscriber,
		revision: U256::one(),
		tick_lower,
		tick_upper,
		liquidity,
		maintained_since_block: block,
		proven_through_block: block,
	}
}

#[cfg(test)]
pub(crate) fn ethereum_proof_block(block: u64) -> ProofBlock {
	ProofBlock {
		hash: U256::from(block).to_big_endian().into(),
		number: block,
		timestamp_millis: block * 12_000,
	}
}

#[cfg(test)]
pub(crate) fn prove_from_later_anchor(
	number: u64,
	timestamp: u64,
	anchor_delay_secs: u64,
) -> (EthereumExecutionBlockProof, H256) {
	let state = ExecutionHeaderAnchors::<Test>::get(H256::from_low_u64_be(number)).unwrap();
	let target = EthereumHeader {
		number,
		timestamp,
		state_root: B256::from(state.state_root.0),
		..Default::default()
	};
	let anchor = ExecutionHeaderAnchor {
		block_hash: H256::from_low_u64_be(10_000 + number),
		parent_hash: H256::from(target.hash_slow().0),
		block_number: number + 1,
		timestamp_millis: (timestamp + anchor_delay_secs) * 1_000,
		..state
	};
	ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
	let execution = EthereumExecutionBlockProof {
		anchor_block_hash: anchor.block_hash,
		target_to_anchor_header_chain: vec![argon_primitives::EthereumExecutionHeader {
			rlp: alloy_rlp::encode(&target).try_into().unwrap(),
		}]
		.try_into()
		.unwrap(),
	};
	(execution, H256::from(target.hash_slow().0))
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
) -> Result<PositionAmounts, PositionAmountError> {
	let sqrt_lower = AlloyU256::from_be_bytes(sqrt_range.start.to_big_endian());
	let sqrt_upper = AlloyU256::from_be_bytes(sqrt_range.end.to_big_endian());
	let sqrt_price =
		AlloyU256::from_be_bytes(sqrt_price_x96.to_big_endian()).clamp(sqrt_lower, sqrt_upper);
	let token0 = get_amount_0_delta(sqrt_price, sqrt_upper, liquidity, false).unwrap();
	let token1 = get_amount_1_delta(sqrt_lower, sqrt_price, liquidity, false).unwrap();
	let (argon, usdc) = if argon_is_token0 { (token0, token1) } else { (token1, token0) };

	let argon_token_units_per_microgon = AlloyU256::from(10u128.pow(12));
	let argon = Balance::try_from(argon / argon_token_units_per_microgon)
		.map_err(|_| PositionAmountError::AmountOverflow)?;
	let usdc = Balance::try_from(usdc).map_err(|_| PositionAmountError::AmountOverflow)?;
	Ok(PositionAmounts { argon, usdc })
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
#[cfg(test)]
pub(crate) fn evidence(
	block: u64,
	revision: u64,
	beneficiary: H256,
	liquidity: u128,
	subscribed: bool,
) -> PositionProof {
	position_evidence(block, revision, beneficiary, liquidity, subscribed, (-100, 100))
}

pub(crate) fn position_evidence(
	block: u64,
	revision: u64,
	beneficiary: H256,
	liquidity: u128,
	subscribed: bool,
	bounds: (UniswapTick, UniswapTick),
) -> PositionProof {
	let block = ProofBlock {
		hash: U256::from(block).to_big_endian().into(),
		number: block,
		timestamp_millis: block * 12_000,
	};
	let checkpoint = PositionCheckpoint {
		beneficiary,
		pool_manager: pool_manager(),
		pool_id: pool_id(),
		subscriber: config().subscriber,
		revision: revision.into(),
		tick_lower: bounds.0,
		tick_upper: bounds.1,
		liquidity,
		maintained_since_block: block,
		proven_through_block: block,
	};
	position_evidence_with_retirement(U256::one(), &checkpoint, subscribed, None).0
}

#[cfg(test)]
pub(crate) fn position_evidence_for_token(
	token_id: UniswapNftId,
	checkpoint: &PositionCheckpoint<AccountId32>,
) -> PositionProof {
	position_evidence_with_retirement(token_id, checkpoint, true, None).0
}

/// Prove a live source at A and a burned source plus live successor in one state at B.
pub(crate) fn continuation_evidence<AccountId: Encode>(
	source_token_id: UniswapNftId,
	before: &PositionCheckpoint<AccountId>,
	successor_token_id: UniswapNftId,
	after: &PositionCheckpoint<AccountId>,
) -> PositionContinuationProof {
	let before = position_evidence_with_retirement(source_token_id, before, true, None).0;
	let (after, retirement) =
		position_evidence_with_retirement(successor_token_id, after, true, Some(source_token_id));
	let retired = retirement.expect("a source token requests its retirement witness");
	fn witness<S: Get<u32>>(
		proof: EthereumAccountStorageProof<S>,
	) -> EthereumAccountStorageWitness<S> {
		EthereumAccountStorageWitness {
			account_proof: proof.account_proof,
			storage_proof: proof.storage_proof,
			slots: proof.slots,
		}
	}
	PositionContinuationProof {
		before: ContinuationSourceProof {
			execution_block_proof: before.subscriber.execution_block_proof.clone(),
			subscriber: witness(before.subscriber),
			position_manager: witness(before.position_manager),
			pool_manager: witness(before.pool_manager),
		},
		after: ContinuationReplacementProof {
			execution_block_proof: after.subscriber.execution_block_proof.clone(),
			subscriber: witness(after.subscriber),
			position_manager: witness(after.position_manager),
			pool_manager: witness(after.pool_manager),
			retired: witness(retired),
		},
	}
}

fn position_evidence_with_retirement<AccountId: Encode>(
	token_id: UniswapNftId,
	checkpoint: &PositionCheckpoint<AccountId>,
	subscribed: bool,
	retired_token_id: Option<UniswapNftId>,
) -> (PositionProof, Option<EthereumAccountStorageProof<ConstU32<1>>>) {
	let block = checkpoint.proven_through_block.number;
	let bounds = (checkpoint.tick_lower, checkpoint.tick_upper);
	let beneficiary = H256::from_slice(&checkpoint.beneficiary.encode());
	let revision = checkpoint.revision;
	let liquidity = checkpoint.liquidity;

	let pool_config = config();

	let subscription_slot = evm::mapping_slot(token_id, evm::SUBSCRIPTIONS_SLOT);

	// Pack Uniswap's pool prefix, signed tick bounds, and subscription flag.
	let mut position_info = pool_id().0;
	position_info[25..28].copy_from_slice(&bounds.1.to_be_bytes()[1..]);
	position_info[28..31].copy_from_slice(&bounds.0.to_be_bytes()[1..]);
	position_info[31] = u8::from(subscribed);

	let mut subscriber_value = [0u8; 32];
	if subscribed {
		subscriber_value[12..].copy_from_slice(pool_config.subscriber.as_bytes());
	}

	// Each account contributes its own storage trie to the shared Ethereum state.
	let mut slots = [
		vec![
			(subscription_slot, beneficiary),
			(evm::offset_slot(subscription_slot, 1), H256::from(revision.to_big_endian())),
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
			evm::liquidity_slot(
				pool_id(),
				pool_config.position_manager,
				token_id,
				bounds.0,
				bounds.1,
			),
			H256::from(U256::from(liquidity).to_big_endian()),
		)],
	];
	if let Some(retired_token_id) = retired_token_id {
		slots[1].push((
			evm::mapping_slot(retired_token_id, evm::POSITION_MANAGER_POSITION_INFO_SLOT),
			H256::zero(),
		));
	}

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

	let (_state_root, account_nodes) = trie(&encoded_accounts);

	#[cfg(test)]
	{
		// Supply finality at the external-chain boundary; trie verification remains real.
		let anchor = ExecutionHeaderAnchor {
			block_hash: H256::from(U256::from(block).to_big_endian()),
			block_number: block,
			timestamp_millis: block * 12_000,
			parent_hash: H256::zero(),
			state_root: H256::from(_state_root.0),
			receipts_root: H256::zero(),
		};
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
	}

	let position = PositionProof {
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
			&slots[1][..2],
			block,
		),
		pool_manager: account_storage_proof(
			addresses[2],
			&account_nodes,
			&storage_tries[2].1,
			&slots[2],
			block,
		),
	};
	let retirement = retired_token_id.map(|_| {
		account_storage_proof(
			addresses[1],
			&account_nodes,
			&storage_tries[1].1,
			&slots[1][2..3],
			block,
		)
	});
	(position, retirement)
}

pub(crate) fn position_change_evidence(
	block: u64,
	revision: u64,
	liquidity: u128,
	subscribed: bool,
	bounds: (UniswapTick, UniswapTick),
) -> PositionChangeProof {
	let proof =
		position_evidence(block, revision, H256::repeat_byte(0x42), liquidity, subscribed, bounds);
	position_change_proof(proof, subscribed)
}

pub(crate) fn position_change_proof(proof: PositionProof, subscribed: bool) -> PositionChangeProof {
	let (account, slot_index) =
		if subscribed { (proof.subscriber, 1) } else { (proof.position_manager, 0) };
	let witness = EthereumAccountStorageProof {
		execution_block_proof: account.execution_block_proof,
		account_proof: account.account_proof,
		storage_proof: account.storage_proof,
		slots: [account.slots[slot_index].clone()].to_vec().try_into().unwrap(),
	};
	if subscribed {
		PositionChangeProof::Revision(witness)
	} else {
		PositionChangeProof::Subscription(witness)
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
