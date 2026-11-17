use crate::{
	mock::{evidence as build_evidence, *},
	uniswap::principal_for_liquidity_range,
	*,
};
use alloy_consensus::Header as EthereumHeader;
use alloy_primitives::B256;
use argon_primitives::{EthereumExecutionBlockProof, EthereumExecutionHeader, MICROGONS_PER_ARGON};
use frame_support::{assert_noop, assert_ok};
use pallet_ethereum_verifier::{types::ExecutionHeaderAnchor, ExecutionHeaderAnchors};
use rstest::rstest;
use sp_runtime::AccountId32;
use uniswap_v3_sdk_bigint::utils::{
	MAX_TICK_I32 as MAX_UNISWAP_TICK, MIN_TICK_I32 as MIN_UNISWAP_TICK,
};

#[test]
fn calculates_both_principals_inside_and_outside_the_range_for_either_token_order() {
	// TickMath(-100), TickMath(0) and TickMath(100). Expected principal uses floor division
	// of L * Q96 * (upper - price) / (price * upper) and L * (price - lower) / Q96.
	let sqrt_lower = 78_833_030_112_140_176_575_862_854_579u128;
	let sqrt_upper = 79_625_275_426_524_748_796_330_556_128u128;
	let cases = [
		(sqrt_lower - 1, 9_999_541_693_800_299u128, 0u128),
		(sqrt_lower, 9_999_541_693_800_299, 0),
		(1u128 << 96, 4_987_272_070_749_096, 4_987_272_070_749_096),
		(79_327_254_682_584_544_153_556_411_592, 3_738_115_413_094_571, 6_237_991_072_371_352),
		(sqrt_upper, 0, 9_999_541_693_800_299),
		(sqrt_upper + 1, 0, 9_999_541_693_800_299),
	];

	for (sqrt_price, amount0, amount1) in cases {
		let argon_first = position_checkpoint(1_000_000_000_000_000_000, -100, 100)
			.amounts_at_price(U256::from(sqrt_price), true)
			.unwrap();
		assert_eq!(argon_first.argon, amount0 / 1_000_000_000_000);
		assert_eq!(argon_first.usdc, amount1);

		let usdc_first = position_checkpoint(1_000_000_000_000_000_000, -100, 100)
			.amounts_at_price(U256::from(sqrt_price), false)
			.unwrap();
		assert_eq!(usdc_first.usdc, amount0);
		assert_eq!(usdc_first.argon, amount1 / 1_000_000_000_000);
	}
}

#[test]
fn empty_and_fractional_positions_do_not_invent_principal() {
	for liquidity in [0, 1] {
		let amounts = position_checkpoint(liquidity, -100, 100)
			.amounts_at_price(U256::one() << 96, true)
			.unwrap();
		assert_eq!(amounts, PositionAmounts { argon: 0, usdc: 0 });
	}
}

#[test]
fn converts_argon_to_microgons_and_drops_sub_microgon_dust() {
	let cases = [
		(200_510_416_479_002_803_287u128, 999_999u128, 999_999_999_999_999_999u128),
		(200_510_416_479_002_803_288, MICROGONS_PER_ARGON, 1_000_000_000_000_000_000),
	];
	for (liquidity, microgons, usdc) in cases {
		let amounts = position_checkpoint(liquidity, -100, 100)
			.amounts_at_price(U256::one() << 96, true)
			.unwrap();
		assert_eq!(amounts, PositionAmounts { argon: microgons, usdc });
	}
}

#[test]
fn scales_argon_before_checking_whether_it_fits_balance() {
	// The raw 18-decimal amount exceeds u128, but its native microgon balance fits.
	let amounts = position_checkpoint(1_000_000_000_000_000_000_000_000, -887_272, 887_272)
		.amounts_at_price(U256::from(4_295_128_739u64), true)
		.unwrap();
	assert_eq!(amounts.argon, 18_446_050_707_367_246_063_248_664_439_159);
	assert_eq!(amounts.usdc, 0);
}

#[test]
fn rejects_principal_that_exceeds_its_balance_representation() {
	for argon_is_token0 in [true, false] {
		assert_eq!(
			position_checkpoint(u128::MAX, -887_272, 887_272)
				.amounts_at_price(U256::from(4_295_128_739u64), argon_is_token0),
			Err(PositionAmountError::AmountOverflow)
		);
	}
}

#[test]
fn rejects_invalid_uniswap_ranges_and_prices_without_truncating_them() {
	for (tick_lower, tick_upper) in [
		(100, 100),
		(100, -100),
		(-887_273, 100),
		(-100, 887_273),
		(i32::MIN, 100),
		(-100, i32::MAX),
	] {
		assert!(position_checkpoint(1_000, tick_lower, tick_upper)
			.amounts_at_price(U256::one() << 96, true)
			.is_err());
	}

	let max_sqrt_price =
		U256::from_dec_str("1461446703485210103287273052203988822378723970342").unwrap();
	for sqrt_price in [
		U256::zero(),
		U256::from(4_295_128_738u64),
		max_sqrt_price,
		(U256::one() << 160) + (U256::one() << 96),
		U256::MAX,
	] {
		assert!(position_checkpoint(1_000, -100, 100)
			.amounts_at_price(sqrt_price, true)
			.is_err());
	}
}

#[test]
fn combined_liquidity_and_principal_can_exceed_one_nfts_limits() {
	let largest_nft = position_checkpoint(u128::MAX, 0, 20_000);
	let smallest_nft = position_checkpoint(1, 0, 20_000);
	let combined_liquidity = U256::from(largest_nft.liquidity) + U256::from(smallest_nft.liquidity);

	// The lower sqrt price is 1 and the current sqrt price is 2. Both are inside the range,
	// so token1 principal equals liquidity exactly: L * (2 - 1) = L.
	let sqrt_price = uniswap_sqrt_price_at_tick(0) * U256::from(2);
	let largest_nft_amounts = largest_nft.amounts_at_price(sqrt_price, true).unwrap();
	let smallest_nft_amounts = smallest_nft.amounts_at_price(sqrt_price, true).unwrap();
	assert_eq!(largest_nft_amounts.usdc, u128::MAX);
	assert_eq!(smallest_nft_amounts.usdc, 1);

	let combined =
		principal_for_liquidity_range(combined_liquidity, 0, 20_000, sqrt_price, true).unwrap();
	assert_eq!(combined.usdc, U256::from(u128::MAX) + U256::one());

	// Reversing token order makes the same token1 amount ARGON, converted to microgons.
	let reversed =
		principal_for_liquidity_range(combined_liquidity, 0, 20_000, sqrt_price, false).unwrap();
	let argon_token_units_per_microgon = U256::from(10u128.pow(12));
	assert_eq!(reversed.argon, combined_liquidity / argon_token_units_per_microgon);
}

#[test]
fn rejects_combined_principal_that_exceeds_u256() {
	let minimum_price = uniswap_sqrt_price_at_tick(MIN_UNISWAP_TICK);
	for argon_is_token0 in [true, false] {
		let result = principal_for_liquidity_range(
			U256::MAX,
			MIN_UNISWAP_TICK,
			MAX_UNISWAP_TICK,
			minimum_price,
			argon_is_token0,
		);
		assert_eq!(result, Err(PositionAmountError::AmountOverflow));
	}
}

#[test]
fn shared_range_principal_matches_the_uniswap_sdk() {
	let ordinary_liquidity = 10u128.pow(18);
	let scenarios = [
		("below the range", position_checkpoint(ordinary_liquidity, -100, 100), -200),
		("at the lower boundary", position_checkpoint(ordinary_liquidity, -100, 100), -100),
		("inside the range", position_checkpoint(ordinary_liquidity, -100, 100), 0),
		("at the upper boundary", position_checkpoint(ordinary_liquidity, -100, 100), 100),
		("above the range", position_checkpoint(ordinary_liquidity, -100, 100), 200),
		(
			"at the lowest supported price",
			position_checkpoint(u128::MAX, MIN_UNISWAP_TICK, MIN_UNISWAP_TICK + 1),
			MIN_UNISWAP_TICK,
		),
		(
			"near the highest supported price",
			position_checkpoint(u128::MAX, MAX_UNISWAP_TICK - 1, MAX_UNISWAP_TICK),
			MAX_UNISWAP_TICK - 1,
		),
		(
			"across the widest supported range",
			position_checkpoint(u128::MAX, MIN_UNISWAP_TICK, MAX_UNISWAP_TICK),
			0,
		),
		(
			"with a small token-unit price ratio",
			position_checkpoint(ordinary_liquidity, -276_400, -276_200),
			-276_300,
		),
		(
			"with a large token-unit price ratio",
			position_checkpoint(ordinary_liquidity, 276_200, 276_400),
			276_300,
		),
	];

	for (scenario, position, uniswap_tick) in scenarios {
		let sqrt_price = uniswap_sqrt_price_at_tick(uniswap_tick);
		for argon_is_token0 in [true, false] {
			let sqrt_range = uniswap_sqrt_price_at_tick(position.tick_lower)..
				uniswap_sqrt_price_at_tick(position.tick_upper);
			let expected =
				uniswap_sdk_principal(position.liquidity, sqrt_range, sqrt_price, argon_is_token0);
			let actual = principal_for_liquidity_range(
				U256::from(position.liquidity),
				position.tick_lower,
				position.tick_upper,
				sqrt_price,
				argon_is_token0,
			)
			.unwrap();
			assert_eq!(actual, expected, "{scenario}; ARGON is token0: {argon_is_token0}");
		}
	}
}

#[test]
fn withdrawing_liquidity_can_remove_a_rounding_unit() {
	let liquidity_per_position = 100u128;
	let sqrt_price = uniswap_sqrt_price_at_tick(0);

	// Each NFT rounds down to zero USDC base units, but three together have one unit.
	// After withdrawing one NFT, the remaining two round down to zero again.
	for argon_is_token0 in [true, false] {
		let one_position = position_checkpoint(liquidity_per_position, -100, 100)
			.amounts_at_price(sqrt_price, argon_is_token0)
			.unwrap();
		let all_three = principal_for_liquidity_range(
			U256::from(liquidity_per_position * 3),
			-100,
			100,
			sqrt_price,
			argon_is_token0,
		)
		.unwrap();
		let after_withdrawal = principal_for_liquidity_range(
			U256::from(liquidity_per_position * 2),
			-100,
			100,
			sqrt_price,
			argon_is_token0,
		)
		.unwrap();

		assert_eq!(one_position.usdc, 0);
		assert_eq!(all_three.usdc, U256::one());
		assert_eq!(after_withdrawal.usdc, U256::zero());
	}
}

#[rstest]
#[case::argon_first_spacing_1(true, 1)]
#[case::argon_second_spacing_1(false, 1)]
#[case::argon_first_spacing_10(true, 10)]
#[case::argon_second_spacing_10(false, 10)]
#[case::argon_first_spacing_60(true, 60)]
#[case::argon_second_spacing_60(false, 60)]
fn reward_bands_allow_a_shared_shift_but_not_independent_boundary_changes(
	#[case] argon_is_token0: bool,
	#[case] spacing: i32,
) {
	let target = FixedU128::one();
	let usdc = FixedU128::one();
	let liquidity = U256::from(10u128.pow(24));
	let band = StabilizerPoolRewardBand::new(
		target,
		usdc,
		argon_is_token0,
		spacing,
		RewardPriceBand::get(),
		Percent::from_percent(1),
	)
	.unwrap();
	let pool_price = uniswap_sqrt_price_for_usd_price(
		FixedU128::from_rational(95, 100),
		usdc,
		argon_is_token0,
		false,
	);

	for reference in [FixedU128::from_rational(99, 100), target, FixedU128::from_rational(101, 100)]
	{
		let (lower, upper) = uniswap_bounds_for_usd_band(
			FixedU128::from_rational(90, 100) * reference,
			reference,
			usdc,
			argon_is_token0,
			spacing,
		);
		let principal = band.principal_for_range(liquidity, lower, upper, pool_price).unwrap();
		assert!(principal.argon > U256::zero(), "reference {reference:?}");
		assert!(principal.usdc > U256::zero(), "reference {reference:?}");
	}

	// A standard full-range NFT has valid grid boundaries, but is too wide to earn.
	// Looking for its next grid boundary must not turn that into a malformed-range error.
	let lowest_usable_tick = -(-MIN_UNISWAP_TICK).div_euclid(spacing) * spacing;
	let highest_usable_tick = MAX_UNISWAP_TICK.div_euclid(spacing) * spacing;
	assert_eq!(
		band.principal_for_range(liquidity, lowest_usable_tick, highest_usable_tick, pool_price),
		Ok(PositionAmounts { argon: U256::zero(), usdc: U256::zero() }),
		"valid full-range position"
	);

	for (invalid_band, lower_usd, upper_usd) in [
		("shortened: 91%-99%", 9_100, 9_900),
		("widened: 89%-101%", 8_900, 10_100),
		("only the upper edge shifted: 90%-101%", 9_000, 10_100),
		("whole band too far up: 92.7%-103%", 9_270, 10_300),
		("whole band too far down: 87.3%-97%", 8_730, 9_700),
	] {
		let (lower, upper) = uniswap_bounds_for_usd_band(
			FixedU128::from_rational(lower_usd, 10_000),
			FixedU128::from_rational(upper_usd, 10_000),
			usdc,
			argon_is_token0,
			spacing,
		);
		let principal = band.principal_for_range(liquidity, lower, upper, pool_price).unwrap();
		assert_eq!(
			principal,
			PositionAmounts { argon: U256::zero(), usdc: U256::zero() },
			"{invalid_band}"
		);
	}
}

#[rstest]
#[case::argon_first(true, 100, 100, (90, 100))]
#[case::argon_second(false, 100, 100, (90, 100))]
#[case::argon_first_changed_quotes(true, 120, 98, (90, 100))]
#[case::argon_second_changed_quotes(false, 120, 98, (90, 100))]
#[case::argon_first_custom_band(true, 120, 98, (85, 98))]
#[case::argon_second_custom_band(false, 120, 98, (85, 98))]
fn shifted_bands_earn_only_on_their_overlap_with_the_current_reward_band(
	#[case] argon_is_token0: bool,
	#[case] argon_target_cents: u128,
	#[case] usdc_price_cents: u128,
	#[case] reward_price_band: (u8, u8),
	#[values(99, 101)] reference_percent: u128,
	#[values(88, 95, 102)] pool_price_percent: u128,
) {
	let target = FixedU128::from_rational(argon_target_cents, 100);
	let usdc = FixedU128::from_rational(usdc_price_cents, 100);
	let liquidity = 10u128.pow(24);
	let (lower_percent, upper_percent) = reward_price_band;
	let band = StabilizerPoolRewardBand::new(
		target,
		usdc,
		argon_is_token0,
		10,
		(Percent::from_percent(lower_percent), Percent::from_percent(upper_percent)),
		Percent::from_percent(1),
	)
	.unwrap();

	let lower_fraction = FixedU128::from_rational(u128::from(lower_percent), 100);
	let upper_fraction = FixedU128::from_rational(u128::from(upper_percent), 100);
	let reference = FixedU128::from_rational(reference_percent, 100) * target;
	let (lower, upper) = uniswap_bounds_for_usd_band(
		lower_fraction * reference,
		upper_fraction * reference,
		usdc,
		argon_is_token0,
		10,
	);

	let lower_reward_price = uniswap_sqrt_price_for_usd_price(
		lower_fraction * target,
		usdc,
		argon_is_token0,
		argon_is_token0,
	);
	let upper_reward_price = uniswap_sqrt_price_for_usd_price(
		upper_fraction * target,
		usdc,
		argon_is_token0,
		!argon_is_token0,
	);
	let reward_sqrt_lower = lower_reward_price.min(upper_reward_price);
	let reward_sqrt_upper = lower_reward_price.max(upper_reward_price);
	let eligible_sqrt_range = uniswap_sqrt_price_at_tick(lower).max(reward_sqrt_lower)..
		uniswap_sqrt_price_at_tick(upper).min(reward_sqrt_upper);

	let pool_price = uniswap_sqrt_price_for_usd_price(
		FixedU128::from_rational(pool_price_percent, 100) * target,
		usdc,
		argon_is_token0,
		false,
	);
	let expected =
		uniswap_sdk_principal(liquidity, eligible_sqrt_range, pool_price, argon_is_token0);
	let actual = band
		.principal_for_range(U256::from(liquidity), lower, upper, pool_price)
		.unwrap();
	assert_eq!(actual, expected);

	let whole_position = position_checkpoint(liquidity, lower, upper)
		.amounts_at_price(pool_price, argon_is_token0)
		.unwrap();
	assert!(actual.argon <= U256::from(whole_position.argon));
	assert!(actual.usdc <= U256::from(whole_position.usdc));
	assert!(
		actual.argon < U256::from(whole_position.argon) ||
			actual.usdc < U256::from(whole_position.usdc)
	);
}

#[rstest]
#[case::zero_lower(0, 100)]
#[case::same_endpoints(90, 90)]
#[case::reversed_endpoints(100, 90)]
fn rejects_invalid_reward_price_bands(#[case] lower_percent: u8, #[case] upper_percent: u8) {
	let result = StabilizerPoolRewardBand::new(
		FixedU128::one(),
		FixedU128::one(),
		true,
		10,
		(Percent::from_percent(lower_percent), Percent::from_percent(upper_percent)),
		Percent::from_percent(1),
	);
	assert_eq!(result.unwrap_err(), PositionAmountError::InvalidRewardPriceBand);
}

#[test]
fn reward_bands_reject_unusable_quotes_and_malformed_uniswap_ranges() {
	let one_dollar = FixedU128::one();
	for argon_is_token0 in [true, false] {
		for (scenario, target, usdc) in [
			("zero ARGON target", FixedU128::zero(), one_dollar),
			("zero USDC quote", one_dollar, FixedU128::zero()),
			(
				"USD ratio outside Uniswap's supported prices",
				FixedU128::from_inner(1),
				FixedU128::from_inner(u128::MAX),
			),
		] {
			let error = StabilizerPoolRewardBand::new(
				target,
				usdc,
				argon_is_token0,
				10,
				RewardPriceBand::get(),
				Percent::from_percent(1),
			)
			.unwrap_err();
			assert_eq!(error, PositionAmountError::InvalidPrice, "{scenario}");
		}

		let band = StabilizerPoolRewardBand::new(
			one_dollar,
			one_dollar,
			argon_is_token0,
			10,
			RewardPriceBand::get(),
			Percent::from_percent(1),
		)
		.unwrap();
		let pool_price =
			uniswap_sqrt_price_for_usd_price(one_dollar, one_dollar, argon_is_token0, false);
		let (lower, upper) = uniswap_bounds_for_usd_band(
			FixedU128::from_rational(90, 100),
			one_dollar,
			one_dollar,
			argon_is_token0,
			10,
		);
		for (scenario, invalid_lower, invalid_upper) in [
			("equal boundaries", lower, lower),
			("reversed boundaries", upper, lower),
			("lower boundary off the grid", lower + 1, upper),
			("upper boundary off the grid", lower, upper - 1),
			("boundary outside Uniswap's domain", MIN_UNISWAP_TICK.div_euclid(10) * 10, upper),
		] {
			assert_eq!(
				band.principal_for_range(U256::one(), invalid_lower, invalid_upper, pool_price),
				Err(PositionAmountError::InvalidTickRange),
				"{scenario}"
			);
		}
		assert_eq!(
			band.principal_for_range(U256::one(), lower, upper, U256::zero()),
			Err(PositionAmountError::InvalidPrice)
		);
	}
}

#[test]
fn opening_prices_are_fixed_and_history_expires_by_frame_age() {
	new_test_ext().execute_with(|| {
		let mut index = PriceIndex {
			btc_usd_price: FixedU128::from_u32(60_000),
			argonot_usd_price: FixedU128::from_u32(2),
			argon_usd_price: FixedU128::one(),
			argon_usd_target_price: FixedU128::one(),
			argon_time_weighted_average_liquidity: 1_000,
			tick: 20,
			usdc_usd_price: Some(FixedU128::one()),
			stabilizer_pool_sqrt_price_x96: Some(U256::one() << 96),
		};
		StabilizerPools::on_price_index(&index);
		assert_eq!(FrameStartPriceIndices::<Test>::get().get(&1), Some(&index));

		// Subsequent swaps and target changes must not revalue the opening reference.
		let opening = index;
		CurrentTick::set(21);
		index.tick = 21;
		index.stabilizer_pool_sqrt_price_x96 = Some(U256::from(2) << 96);
		index.argon_usd_target_price = FixedU128::from_rational(101, 100);
		StabilizerPools::on_price_index(&index);
		assert_eq!(FrameStartPriceIndices::<Test>::get().get(&1), Some(&opening));

		// The transition flag selects the new frame before the ordinary frame ID advances.
		NewlyStartedFrame::set(Some(2));
		index.stabilizer_pool_sqrt_price_x96 = None;
		StabilizerPools::on_price_index(&index);
		assert_eq!(FrameStartPriceIndices::<Test>::get().get(&2), Some(&index));

		CurrentFrame::set(2);
		NewlyStartedFrame::set(None);
		index.stabilizer_pool_sqrt_price_x96 = opening.stabilizer_pool_sqrt_price_x96;
		StabilizerPools::on_price_index(&index);
		assert_eq!(
			FrameStartPriceIndices::<Test>::get()
				.get(&2)
				.unwrap()
				.stabilizer_pool_sqrt_price_x96,
			None
		);

		// A missed frame remains absent. An older index cannot fill the next frame either.
		CurrentFrame::set(4);
		CurrentTick::set(40);
		StabilizerPools::on_price_index(&index);
		assert!(!FrameStartPriceIndices::<Test>::get().contains_key(&3));
		assert!(!FrameStartPriceIndices::<Test>::get().contains_key(&4));

		// Retention counts financial frames, not observations or the network's wall-clock speed.
		for frame_id in 4..=33 {
			CurrentFrame::set(frame_id);
			CurrentTick::set(frame_id * 10);
			index.tick = frame_id * 10;
			StabilizerPools::on_price_index(&index);
		}
		assert_eq!(FrameStartPriceIndices::<Test>::get().get(&1), Some(&opening));

		// Frame 34 expires frame 1 even though the missing frame 3 leaves unused capacity.
		CurrentFrame::set(34);
		CurrentTick::set(340);
		index.tick = 340;
		StabilizerPools::on_price_index(&index);
		let history = FrameStartPriceIndices::<Test>::get();
		assert!(!history.contains_key(&1));
		assert!(history.contains_key(&2));
		assert!(!history.contains_key(&3));
		assert_eq!(history.get(&34), Some(&index));
		assert!(history.len() <= 33);

		for frame_id in 35..=37 {
			CurrentFrame::set(frame_id);
			CurrentTick::set(frame_id * 10);
			index.tick = frame_id * 10;
			StabilizerPools::on_price_index(&index);
		}
		let history = FrameStartPriceIndices::<Test>::get();
		assert_eq!(history.keys().next(), Some(&5));
		assert_eq!(history.len(), 33);
		assert!(history.encoded_size() < 5_000);

		// Returning after an oracle gap removes all expired references in the same update.
		CurrentFrame::set(100);
		CurrentTick::set(1_000);
		index.tick = 1_000;
		StabilizerPools::on_price_index(&index);
		let history = FrameStartPriceIndices::<Test>::get();
		assert_eq!(history.keys().copied().collect::<Vec<_>>(), vec![100]);
		assert_eq!(history.get(&100), Some(&index));
	});
}

#[test]
fn registers_and_proves_maintained_intervals_without_sync_or_rewards() {
	new_test_ext().execute_with(|| {
		let proof = evidence(100, 1, beneficiary_account(), 1_000, true);
		let proof = PositionProof::decode(&mut proof.encode().as_slice()).unwrap();
		assert_ok!(submit(proof));
		assert_eq!(checkpoint().maintained_since.number, 100);

		let later = evidence(200, 1, beneficiary_account(), 1_000, true);
		assert_ok!(submit(later));
		let record = checkpoint();
		assert_eq!(record.proven_through.timestamp_millis, 2_400_000);
		assert_eq!(record.maintained_since.number, 100);
		System::assert_last_event(
			Event::PositionCheckpointed {
				position_manager: config().position_manager,
				token_id: U256::one(),
				checkpoint: Box::new(record),
				outcome: CheckpointOutcome::Maintained {
					from: ProofBlock {
						hash: H256::from_low_u64_be(100),
						number: 100,
						timestamp_millis: 1_200_000,
					},
				},
			}
			.into(),
		);
	});
}

#[test]
fn revision_change_discards_unverified_gap_even_when_liquidity_returns_to_same_value() {
	new_test_ext().execute_with(|| {
		let first = evidence(100, 1, beneficiary_account(), 1_000, true);
		assert_ok!(submit(first));

		let changed = evidence(200, 3, beneficiary_account(), 1_000, true);
		assert_ok!(submit(changed));
		assert_eq!(checkpoint().maintained_since.number, 200);
		assert_eq!(checkpoint().revision, U256::from(3));

		let later = evidence(300, 3, beneficiary_account(), 1_000, true);
		assert_ok!(submit(later));
		assert_eq!(checkpoint().maintained_since.number, 200);
	});
}

#[test]
fn unchanged_counter_cannot_hide_an_unsubscribe_and_failed_proofs_preserve_checkpoint() {
	new_test_ext().execute_with(|| {
		let first = evidence(100, 1, beneficiary_account(), 1_000, true);
		assert_ok!(submit(first));

		let before = checkpoint();
		let unsubscribed = evidence(200, 1, beneficiary_account(), 1_000, false);
		assert_noop!(submit(unsubscribed), Error::<Test>::InvalidSubscription);
		assert_eq!(checkpoint(), before);
	});
}

#[test]
fn new_beneficiary_must_authorize_reenrollment_and_keeps_global_replay_watermark() {
	new_test_ext().execute_with(|| {
		let first = evidence(100, 1, beneficiary_account(), 1_000, true);
		assert_ok!(submit(first));

		let next = AccountId32::new([0x24; 32]);

		let changed = evidence(200, 2, next.clone(), 1_000, true);
		assert_noop!(submit(changed.clone()), Error::<Test>::NotBeneficiary);
		assert_ok!(StabilizerPools::checkpoint_position(
			RuntimeOrigin::signed(next.clone()),
			pool_manager(),
			pool_id(),
			U256::one(),
			Box::new(changed)
		));
		assert_eq!(checkpoint().beneficiary, next);
		assert_eq!(checkpoint().maintained_since.number, 200);

		let old = evidence(100, 1, beneficiary_account(), 1_000, true);
		assert_noop!(submit(old), Error::<Test>::StaleCheckpoint);
	});
}

#[test]
fn rejects_replays_regressed_revisions_and_unchanged_revision_state_changes() {
	new_test_ext().execute_with(|| {
		let first = evidence(100, 3, beneficiary_account(), 1_000, true);
		assert_ok!(submit(first.clone()));
		assert_noop!(submit(first), Error::<Test>::StaleCheckpoint);

		let changed = evidence(200, 3, beneficiary_account(), 900, true);
		assert_noop!(submit(changed), Error::<Test>::PositionChangedWithoutRevision);

		let regressed = evidence(300, 2, beneficiary_account(), 1_000, true);
		assert_noop!(submit(regressed), Error::<Test>::RevisionRegressed);
	});
}

#[test]
fn rejects_forged_values_and_wrong_slots_or_token_ids() {
	new_test_ext().execute_with(|| {
		let proof = evidence(100, 1, beneficiary_account(), 1_000, true);

		let mut tampered = proof.clone();
		tampered.pool_manager.slots[0].value = H256::from_low_u64_be(10_000);
		assert_noop!(submit(tampered), Error::<Test>::InvalidProof);

		let mut tampered = proof.clone();
		tampered.subscriber.slots[0].slot = H256::zero();
		assert_noop!(submit(tampered), Error::<Test>::InvalidStorageSlots);
		assert_noop!(
			StabilizerPools::checkpoint_position(
				RuntimeOrigin::signed(beneficiary_account()),
				pool_manager(),
				pool_id(),
				U256::from(2),
				Box::new(proof)
			),
			Error::<Test>::InvalidStorageSlots
		);
		assert!(PositionCheckpoints::<Test>::get(config().position_manager, U256::one()).is_none());
	});
}

#[test]
fn rejects_different_source_blocks_and_uninitialized_subscriptions() {
	new_test_ext().execute_with(|| {
		let mut proof = evidence(100, 1, beneficiary_account(), 1_000, true);
		proof.pool_manager.execution_block_proof.anchor_block_hash = H256::repeat_byte(9);
		assert_noop!(submit(proof), Error::<Test>::InconsistentProofBlocks);
		let proof = evidence(200, 0, beneficiary_account(), 1_000, true);
		assert_noop!(submit(proof), Error::<Test>::InvalidSubscription);
	});
}

#[test]
fn uses_authenticated_target_timestamp_when_proof_is_an_ancestor_of_anchor() {
	new_test_ext().execute_with(|| {
		let mut proof = evidence(100, 1, beneficiary_account(), 1_000, true);
		let state = ExecutionHeaderAnchors::<Test>::get(H256::from_low_u64_be(100)).unwrap();
		let target = EthereumHeader {
			number: 100,
			timestamp: 123,
			state_root: B256::from(state.state_root.0),
			..Default::default()
		};
		let anchor = ExecutionHeaderAnchor {
			block_hash: H256::repeat_byte(0xaa),
			parent_hash: H256::from(target.hash_slow().0),
			block_number: 101,
			timestamp_millis: 135_000,
			..state
		};
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		let execution = EthereumExecutionBlockProof {
			anchor_block_hash: anchor.block_hash,
			target_to_anchor_header_chain: vec![EthereumExecutionHeader {
				rlp: alloy_rlp::encode(&target).try_into().unwrap(),
			}]
			.try_into()
			.unwrap(),
		};
		proof.subscriber.execution_block_proof = execution.clone();
		proof.position_manager.execution_block_proof = execution.clone();
		proof.pool_manager.execution_block_proof = execution;
		assert_ok!(submit(proof));
		assert_eq!(checkpoint().proven_through.hash, H256::from(target.hash_slow().0));
		assert_eq!(checkpoint().proven_through.timestamp_millis, 123_000);
	});
}

#[test]
fn approvals_are_root_only_and_cannot_reconfigure_existing_pool_history() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			StabilizerPools::register_pool(
				RuntimeOrigin::signed(beneficiary_account()),
				pool_manager(),
				config()
			),
			DispatchError::BadOrigin
		);
		assert_noop!(
			StabilizerPools::register_pool(RuntimeOrigin::root(), pool_manager(), config()),
			Error::<Test>::PoolAlreadyRegistered
		);

		let mut changed = config();
		changed.subscriber = H160::repeat_byte(0x99);
		assert_noop!(
			StabilizerPools::register_pool(RuntimeOrigin::root(), pool_manager(), changed),
			Error::<Test>::PoolAlreadyRegistered
		);

		let mut invalid = config();
		invalid.key.tick_spacing = 0;
		assert_noop!(
			StabilizerPools::register_pool(RuntimeOrigin::root(), pool_manager(), invalid),
			Error::<Test>::InvalidPool
		);
	});
}

fn beneficiary_account() -> AccountId32 {
	AccountId32::new([0x42; 32])
}

fn evidence(
	block: u64,
	revision: u64,
	beneficiary: AccountId32,
	liquidity: u128,
	subscribed: bool,
) -> PositionProof {
	build_evidence::<Test>(
		block,
		revision,
		H256::from_slice(beneficiary.as_ref()),
		liquidity,
		subscribed,
	)
}

fn submit(proof: PositionProof) -> DispatchResult {
	StabilizerPools::checkpoint_position(
		RuntimeOrigin::signed(beneficiary_account()),
		pool_manager(),
		pool_id(),
		U256::one(),
		Box::new(proof),
	)
}

fn checkpoint() -> PositionCheckpoint<AccountId32> {
	PositionCheckpoints::<Test>::get(config().position_manager, U256::one()).unwrap()
}
