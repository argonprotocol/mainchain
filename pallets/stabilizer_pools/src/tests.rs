use crate::{
	mock::{evidence as build_evidence, *},
	uniswap::{principal_for_liquidity_range, MAX_REWARD_RANGES, REWARD_BAND_TOLERANCE},
	*,
};
use alloy_consensus::Header as EthereumHeader;
use alloy_primitives::B256;
use argon_primitives::{
	EthereumExecutionBlockProof, EthereumExecutionHeader, PriceIndexUpdated, MICROGONS_PER_ARGON,
};
use frame_support::{assert_noop, assert_ok, traits::OnRuntimeUpgrade};
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
fn raw_liquidity_can_exceed_u128_while_principal_must_fit() {
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

	assert_eq!(
		principal_for_liquidity_range(combined_liquidity, 0, 20_000, sqrt_price, true),
		Err(PositionAmountError::AmountOverflow)
	);

	// Reversing token order makes the same token1 amount ARGON, converted to microgons.
	let reversed =
		principal_for_liquidity_range(combined_liquidity, 0, 20_000, sqrt_price, false).unwrap();
	let argon_token_units_per_microgon = U256::from(10u128.pow(12));
	let expected_microgons =
		Balance::try_from(combined_liquidity / argon_token_units_per_microgon).unwrap();
	assert_eq!(reversed.argon, expected_microgons);
}

#[test]
fn rejects_out_of_range_principal_from_max_raw_liquidity() {
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
			);
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
		assert_eq!(all_three.usdc, 1);
		assert_eq!(after_withdrawal.usdc, 0);
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
		assert!(principal.argon > 0, "reference {reference:?}");
		assert!(principal.usdc > 0, "reference {reference:?}");
	}

	// A standard full-range NFT has valid grid boundaries, but is too wide to earn.
	// Looking for its next grid boundary must not turn that into a malformed-range error.
	let lowest_usable_tick = -(-MIN_UNISWAP_TICK).div_euclid(spacing) * spacing;
	let highest_usable_tick = MAX_UNISWAP_TICK.div_euclid(spacing) * spacing;
	assert_eq!(
		band.principal_for_range(liquidity, lowest_usable_tick, highest_usable_tick, pool_price),
		Ok(PositionAmounts::default()),
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
		assert_eq!(principal, PositionAmounts::default(), "{invalid_band}");
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
		uniswap_sdk_principal(liquidity, eligible_sqrt_range, pool_price, argon_is_token0).unwrap();
	let actual = band
		.principal_for_range(U256::from(liquidity), lower, upper, pool_price)
		.unwrap();
	assert_eq!(actual, expected);

	let whole_position = position_checkpoint(liquidity, lower, upper)
		.amounts_at_price(pool_price, argon_is_token0)
		.unwrap();
	assert!(actual.argon <= whole_position.argon);
	assert!(actual.usdc <= whole_position.usdc);
	assert!(actual.argon < whole_position.argon || actual.usdc < whole_position.usdc);
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

type FramePriceSamples = StabilizerPoolPriceSamples<MaxPriceBucketsPerFrame>;

#[test]
fn frame_price_is_the_median_of_bucket_means() {
	let frame_start = 20;
	let ticks_per_bucket = 4;
	let frame_ticks = 24 * ticks_per_bucket;
	let mut samples =
		FramePriceSamples::new(frame_start, frame_ticks, MinPriceBucketCoverage::get()).unwrap();

	let cheaper_pool_price = uniswap_sqrt_price_at_tick(-300);
	let higher_pool_price = uniswap_sqrt_price_at_tick(300);
	let price_spike = uniswap_sqrt_price_at_tick(18_000);
	for elapsed_ticks in 0..frame_ticks {
		let bucket = elapsed_ticks / ticks_per_bucket;
		let tick_in_bucket = elapsed_ticks % ticks_per_bucket;
		let price = match bucket {
			0..=10 => cheaper_pool_price,
			// The two middle buckets average to -100 and +200 Uniswap ticks.
			11 if tick_in_bucket < 2 => cheaper_pool_price,
			11 if tick_in_bucket == 2 => higher_pool_price,
			11 => continue,
			12 if tick_in_bucket == 0 => uniswap_sqrt_price_at_tick(-100),
			12..=22 => higher_pool_price,
			_ => price_spike,
		};
		samples.record(frame_start + elapsed_ticks, price);
	}

	assert_eq!(samples.finish(frame_start + frame_ticks), Some(uniswap_sqrt_price_at_tick(50)));
}

#[rstest]
#[case::empty_bucket(0, false)]
#[case::half_coverage(2, false)]
#[case::minimum_coverage(3, true)]
fn frame_price_requires_coverage_in_every_bucket(
	#[case] samples_in_first_bucket: Tick,
	#[case] has_frame_price: bool,
) {
	let frame_start = 20;
	let frame_ticks = 96;
	let pool_price = uniswap_sqrt_price_at_tick(100);
	let price_wider_than_uniswap_slot = U256::one() << 160;
	let mut samples =
		FramePriceSamples::new(frame_start, frame_ticks, MinPriceBucketCoverage::get()).unwrap();
	samples.record(frame_start - 1, pool_price);
	for elapsed_ticks in 0..4 {
		let tick = frame_start + elapsed_ticks;
		if elapsed_ticks < samples_in_first_bucket {
			samples.record(tick, pool_price);
			samples.record(tick, pool_price);
		} else {
			samples.record(tick, U256::zero());
			samples.record(tick, price_wider_than_uniswap_slot);
		}
	}
	for elapsed_ticks in 4..frame_ticks {
		samples.record(frame_start + elapsed_ticks, pool_price);
	}

	assert_eq!(samples.finish(frame_start + frame_ticks), has_frame_price.then_some(pool_price));
}

#[rstest]
#[case::short_frame(5, 5, 5, true)]
#[case::ten_tick_frame_missing_one_price(10, 10, 9, false)]
#[case::delayed_frame(96, 100, 100, true)]
#[case::delayed_frame_missing_its_last_bucket(96, 100, 96, false)]
#[case::delayed_frame_with_two_samples_across_three_ticks(96, 99, 98, false)]
fn frame_price_uses_the_actual_frame_end(
	#[case] configured_frame_ticks: Tick,
	#[case] actual_frame_ticks: Tick,
	#[case] sampled_ticks: Tick,
	#[case] has_frame_price: bool,
) {
	let frame_start = 20;
	let pool_price = uniswap_sqrt_price_at_tick(100);
	let mut samples =
		FramePriceSamples::new(frame_start, configured_frame_ticks, MinPriceBucketCoverage::get())
			.unwrap();
	for elapsed_ticks in 0..sampled_ticks {
		samples.record(frame_start + elapsed_ticks, pool_price);
	}

	assert_eq!(
		samples.finish(frame_start + actual_frame_ticks),
		has_frame_price.then_some(pool_price)
	);
}

#[test]
fn sparse_frame_with_a_large_tick_gap_has_no_price() {
	let pool_price = uniswap_sqrt_price_at_tick(100);
	let mut samples = FramePriceSamples::new(0, 24, MinPriceBucketCoverage::get()).unwrap();
	samples.record(0, pool_price);
	samples.record(Tick::MAX - 1, pool_price);

	assert_eq!(samples.finish(Tick::MAX), None);
}

#[rstest]
#[case::half_coverage_is_enough(Percent::from_percent(50), true)]
#[case::needs_three_quarters(Percent::from_percent(75), false)]
fn frame_price_uses_the_configured_bucket_count_and_coverage(
	#[case] min_coverage: Percent,
	#[case] has_frame_price: bool,
) {
	let mut samples = StabilizerPoolPriceSamples::<ConstU32<2>>::new(0, 8, min_coverage).unwrap();
	let first_bucket_price = uniswap_sqrt_price_at_tick(100);
	let second_bucket_price = uniswap_sqrt_price_at_tick(300);
	samples.record(0, first_bucket_price);
	samples.record(1, first_bucket_price);
	samples.record(4, second_bucket_price);
	samples.record(5, second_bucket_price);

	assert_eq!(samples.finish(8), has_frame_price.then(|| uniswap_sqrt_price_at_tick(200)));
}

#[test]
fn one_price_bucket_can_cover_the_full_tick_range() {
	let mut samples =
		StabilizerPoolPriceSamples::<ConstU32<1>>::new(0, 1, Percent::zero()).unwrap();
	let pool_price = uniswap_sqrt_price_at_tick(100);
	samples.record(0, pool_price);

	assert_eq!(samples.finish(Tick::MAX), Some(pool_price));
}

#[test]
fn sampled_prices_follow_frame_boundaries_even_when_blocks_share_a_clock_tick() {
	new_test_ext().execute_with(|| {
		let first_frame_price = uniswap_sqrt_price_at_tick(100);
		let second_frame_price = uniswap_sqrt_price_at_tick(300);
		start_mining_frame(1, 20);
		let mut opening = stabilizer_pool_price_index(20, uniswap_sqrt_price_at_tick(18_000));
		opening.usdc_usd_price = None;
		StabilizerPools::on_price_index(&opening, Some(stabilizer_pool_price_source()));
		CurrentTick::set(21);
		report_stabilizer_pool_price(21, first_frame_price);
		for tick in 22..30 {
			CurrentTick::set(tick);
			let mut index = stabilizer_pool_price_index(tick, first_frame_price);
			index.argon_usd_target_price = FixedU128::from_rational(101, 100);
			index.usdc_usd_price = (tick != 22).then_some(FixedU128::from_rational(99, 100));
			StabilizerPools::on_price_index(&index, Some(stabilizer_pool_price_source()));
		}
		assert!(StabilizerPoolPricesByFrame::<Test>::get().is_empty());

		// The oracle reports tick 30 in the block before frame 2 opens, at the same tick.
		CurrentTick::set(30);
		report_stabilizer_pool_price(30, second_frame_price);
		start_mining_frame(2, 30);
		assert_eq!(
			StabilizerPoolPricesByFrame::<Test>::get().get(&1),
			Some(&StabilizerPoolFramePrice {
				sqrt_price_x96: first_frame_price,
				argon_usd_target_price: FixedU128::one(),
				usdc_usd_price: FixedU128::one(),
				pool: stabilizer_pool_price_source().pool,
				argon_is_token0: stabilizer_pool_price_source().argon_is_token0,
			})
		);
		for tick in 31..40 {
			CurrentTick::set(tick);
			report_stabilizer_pool_price(tick, second_frame_price);
		}

		// Even without an oracle submission in the opening block, frame 2 closes normally.
		start_mining_frame(3, 40);
		assert_eq!(
			StabilizerPoolPricesByFrame::<Test>::get()
				.get(&2)
				.map(|price| price.sqrt_price_x96),
			Some(second_frame_price)
		);
	});
}

#[test]
fn missing_prices_do_not_delay_the_next_frame_or_history_expiry() {
	new_test_ext().execute_with(|| {
		let price = uniswap_sqrt_price_at_tick(100);
		start_mining_frame(1, 20);
		for tick in 20..29 {
			CurrentTick::set(tick);
			report_stabilizer_pool_price(tick, price);
		}

		start_mining_frame(2, 30);
		// This would complete frame 1, but it arrived after that frame closed.
		report_stabilizer_pool_price(29, price);
		for tick in 31..40 {
			CurrentTick::set(tick);
			report_stabilizer_pool_price(tick - 1, price);
		}
		// A late observation delivered before activation can still complete the closing frame.
		CurrentTick::set(40);
		report_stabilizer_pool_price(39, price);
		// The opening block can land partway through a Mainchain tick.
		CurrentTime::set(2_401_234);
		CurrentFrame::set(3);
		StabilizerPools::on_frame_start(3);

		let finalized = StabilizerPoolRewardSharesByFrame::<Test>::get();
		assert_eq!(finalized[&1], Percent::from_percent(90));
		assert_eq!(FinalizedRewardValues::get(), vec![(1, 0)]);
		// The frame with a missing price still has its own opening and the next opening.
		// Reprocessing the frame boundary cannot overwrite its recorded timestamp.
		start_mining_frame(3, 41);
		let starts = StabilizerPoolFrameStartMillis::<Test>::get();
		assert_eq!(starts[&1], 1_200_000);
		assert_eq!(starts[&2], 1_800_000);
		assert_eq!(starts[&3], 2_401_234);

		let history = StabilizerPoolPricesByFrame::<Test>::get();
		assert!(!history.contains_key(&1));
		assert_eq!(history.get(&2).map(|frame_price| frame_price.sqrt_price_x96), Some(price));
		for frame in 4..=15 {
			start_mining_frame(frame, 10 + frame * 10);
		}
		assert_eq!(StabilizerPoolPricesByFrame::<Test>::get(), history);
		start_mining_frame(16, 170);
		assert!(StabilizerPoolPricesByFrame::<Test>::get().is_empty());
		let starts = StabilizerPoolFrameStartMillis::<Test>::get();
		assert_eq!(starts.keys().copied().collect::<Vec<_>>(), (3..=16).collect::<Vec<_>>());
		assert_eq!(starts[&16], 10_200_000);

		// A skipped frame has no boundary and cannot become a review frame.
		start_mining_frame(18, 190);
		let starts = StabilizerPoolFrameStartMillis::<Test>::get();
		assert!(!starts.contains_key(&17));
		assert_eq!(starts[&18], 11_400_000);
		assert!(!StabilizerPoolReviewFrame::<Test>::exists());
	});
}

#[test]
fn frame_price_requires_spot_prices_from_a_single_approved_revision() {
	new_test_ext().execute_with(|| {
		let old_pool = stabilizer_pool_price_source();
		let mut new_pool = old_pool;
		new_pool.revision += 1;
		new_pool.pool.pool_id = H256::repeat_byte(2);
		let sqrt_price = uniswap_sqrt_price_at_tick(100);

		start_mining_frame(1, 20);
		for tick in 20..30 {
			CurrentTick::set(tick);
			let source = if tick < 25 { old_pool } else { new_pool };
			StabilizerPools::on_price_index(
				&stabilizer_pool_price_index(tick, sqrt_price),
				Some(source),
			);
		}
		start_mining_frame(2, 30);
		assert!(StabilizerPoolPricesByFrame::<Test>::get().is_empty());

		for tick in 30..40 {
			CurrentTick::set(tick);
			StabilizerPools::on_price_index(
				&stabilizer_pool_price_index(tick, sqrt_price),
				Some(new_pool),
			);
		}
		start_mining_frame(3, 40);
		let prices = StabilizerPoolPricesByFrame::<Test>::get();
		assert_eq!(prices.get(&2).unwrap().pool, new_pool.pool);
	});
}

#[test]
fn upgrade_retires_opening_price_indices_and_preserves_checkpoints() {
	use crate::migrations::{v1::FrameStartPriceIndices, RemoveOpeningPriceIndicesMigration};

	new_test_ext().execute_with(|| {
		let opening_index = stabilizer_pool_price_index(20, U256::one() << 96);
		StorageVersion::new(1).put::<StabilizerPools>();
		FrameStartPriceIndices::<Test>::mutate(|history| {
			history.try_insert(1, opening_index).unwrap()
		});
		let token_id = U256::one();
		let checkpoint = position_checkpoint(1_000, -100, 100);
		PositionCheckpoints::<Test>::insert(config().position_manager, token_id, &checkpoint);

		RemoveOpeningPriceIndicesMigration::<Test>::on_runtime_upgrade();

		assert!(!FrameStartPriceIndices::<Test>::exists());
		assert!(StabilizerPoolPricesByFrame::<Test>::get().is_empty());
		assert_eq!(
			PositionCheckpoints::<Test>::get(config().position_manager, token_id),
			Some(checkpoint.clone())
		);
		assert_eq!(StorageVersion::get::<StabilizerPools>(), StorageVersion::new(2));

		// An existing checkpoint becomes eligible only after a fresh proof. Its earlier continuity
		// can still be recorded without treating that time as registered participation.
		assert_ok!(submit(evidence(200, 1, beneficiary_account(), 1_000, true)));
		assert_eq!(
			StabilizerPoolRegistrations::<Test>::get(StabilizerPoolPositionId {
				position_manager: config().position_manager,
				token_id,
			}),
			Some(StabilizerPoolRegistration::Enrolled(StabilizerPoolEnrollment {
				eligible_from: 2,
				next_claim_frame: 2
			}))
		);
		let range = StabilizerPoolLiquidityRange::from(&checkpoint);
		assert_eq!(
			RegisteredLiquidityByRange::<Test>::get(range).liquidity_for_frame(1),
			Some(U256::zero())
		);
		assert_eq!(
			RegisteredLiquidityByRange::<Test>::get(range).liquidity_for_frame(2),
			Some(U256::from(1_000))
		);
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
		let range = StabilizerPoolLiquidityRange::from(&checkpoint());
		CurrentFrame::set(2);

		let changed = evidence(200, 3, beneficiary_account(), 1_000, true);
		assert_ok!(submit(changed));
		assert_eq!(checkpoint().maintained_since.number, 200);
		assert_eq!(checkpoint().revision, U256::from(3));
		let registered = RegisteredLiquidityByRange::<Test>::get(range);
		assert_eq!(registered.liquidity_for_frame(2), Some(U256::zero()));
		assert_eq!(registered.liquidity_for_frame(3), Some(U256::from(1_000)));

		let later = evidence(300, 3, beneficiary_account(), 1_000, true);
		assert_ok!(submit(later));
		assert_eq!(checkpoint().maintained_since.number, 200);
		assert_eq!(RegisteredLiquidityByRange::<Test>::get(range), registered);
		assert_eq!(
			StabilizerPoolRegistrations::<Test>::get(StabilizerPoolPositionId {
				position_manager: config().position_manager,
				token_id: U256::one(),
			}),
			Some(StabilizerPoolRegistration::Enrolled(StabilizerPoolEnrollment {
				eligible_from: 3,
				next_claim_frame: 3
			}))
		);

		let emptied = evidence(400, 4, beneficiary_account(), 0, true);
		assert_ok!(submit(emptied));
		assert!(!RegisteredLiquidityByRange::<Test>::contains_key(range));
	});
}

#[test]
fn independent_nfts_share_their_range_without_narrowing_its_liquidity() {
	new_test_ext().execute_with(|| {
		let first = evidence(100, 1, beneficiary_account(), u128::MAX, true);
		assert_ok!(submit(first));

		let second_checkpoint = checkpoint();
		let range = StabilizerPoolLiquidityRange::from(&second_checkpoint);
		assert_ok!(StabilizerPools::record_checkpoint(
			config().position_manager,
			U256::from(2),
			second_checkpoint
		));

		let one_nft = U256::from(u128::MAX);
		let registered = RegisteredLiquidityByRange::<Test>::get(range);
		assert_eq!(registered.liquidity_for_frame(1), Some(U256::zero()));
		assert_eq!(registered.liquidity_for_frame(2), Some(one_nft + one_nft));

		CurrentFrame::set(5);
		let changed = evidence(200, 2, beneficiary_account(), 100, true);
		assert_ok!(submit(changed));
		let registered = RegisteredLiquidityByRange::<Test>::get(range);
		assert_eq!(registered.liquidity_for_frame(5), Some(one_nft));
		assert_eq!(registered.liquidity_for_frame(6), Some(one_nft + U256::from(100)));
	});
}

#[test]
fn account_position_limit_allows_refreshes_and_tracks_owner_changes() {
	new_test_ext().execute_with(|| {
		assert_ok!(submit(evidence(100, 1, beneficiary_account(), 1_000, true)));
		let mut record = checkpoint();
		let manager = config().position_manager;
		for token_id in 2..=5 {
			assert_ok!(StabilizerPools::record_checkpoint(
				manager,
				token_id.into(),
				record.clone()
			));
		}
		assert_eq!(PositionIdsByAccount::<Test>::get(beneficiary_account()).len(), 5);
		assert_noop!(
			StabilizerPools::record_checkpoint(manager, 6.into(), record.clone()),
			Error::<Test>::TooManyPositions
		);
		assert_ok!(submit(evidence(150, 1, beneficiary_account(), 1_000, true)));
		assert_eq!(PositionIdsByAccount::<Test>::get(beneficiary_account()).len(), 5);

		let next_owner = AccountId32::new([0x24; 32]);
		let changed = evidence(200, 2, next_owner.clone(), 1_000, true);
		assert_ok!(StabilizerPools::checkpoint_position(
			RuntimeOrigin::signed(next_owner.clone()),
			pool_manager(),
			pool_id(),
			1.into(),
			Box::new(changed),
		));
		assert_eq!(PositionIdsByAccount::<Test>::get(beneficiary_account()).len(), 4);
		assert_eq!(
			PositionIdsByAccount::<Test>::get(&next_owner).as_slice(),
			&[StabilizerPoolPositionId { position_manager: manager, token_id: 1.into() },]
		);
		assert_ok!(StabilizerPools::record_checkpoint(manager, 6.into(), record.clone()));
		assert_eq!(PositionIdsByAccount::<Test>::get(beneficiary_account()).len(), 5);

		let position = StabilizerPoolPositionId { position_manager: manager, token_id: 2.into() };
		let Some(StabilizerPoolRegistration::Enrolled(enrollment)) =
			StabilizerPoolRegistrations::<Test>::get(position)
		else {
			panic!("expected an enrolled position")
		};
		record.proven_through.number += 1;
		record.proven_through.timestamp_millis += 12_000;
		assert_ok!(StabilizerPools::remove_reported_position(
			position,
			&record,
			enrollment,
			record.proven_through,
		));
		assert_eq!(PositionIdsByAccount::<Test>::get(beneficiary_account()).len(), 4);
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
	build_evidence(block, revision, H256::from_slice(beneficiary.as_ref()), liquidity, subscribed)
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

#[rstest]
#[case(true, 1)]
#[case(false, 1)]
#[case(true, 10)]
#[case(false, 60)]
fn closed_frame_looks_up_every_eligible_range_on_the_pool_grid(
	#[case] argon_is_token0: bool,
	#[case] spacing: i32,
) {
	let target = FixedU128::from_rational(120, 100);
	let usdc = FixedU128::from_rational(98, 100);
	let band = StabilizerPoolRewardBand::new(
		target,
		usdc,
		argon_is_token0,
		spacing,
		RewardPriceBand::get(),
		REWARD_BAND_TOLERANCE,
	)
	.unwrap();
	let ranges = band.reward_ranges().unwrap();
	assert!(!ranges.is_empty());
	assert!(ranges.len() <= MAX_REWARD_RANGES as usize);
	let spot = uniswap_sqrt_price_for_usd_price(
		FixedU128::from_rational(95, 100) * target,
		usdc,
		argon_is_token0,
		false,
	);
	let liquidity = U256::from(10u128.pow(24));
	for &(lower, upper) in &ranges {
		let principal = band.principal_for_range(liquidity, lower, upper, spot).unwrap();
		assert_ne!(principal, PositionAmounts::default());
	}

	let (minimum_lower, minimum_upper) = uniswap_bounds_for_usd_band(
		FixedU128::from_rational(891, 1000) * target,
		FixedU128::from_rational(99, 100) * target,
		usdc,
		argon_is_token0,
		spacing,
	);
	let (maximum_lower, maximum_upper) = uniswap_bounds_for_usd_band(
		FixedU128::from_rational(909, 1000) * target,
		FixedU128::from_rational(101, 100) * target,
		usdc,
		argon_is_token0,
		spacing,
	);
	// Check the full small grid rectangle against independent per-range valuation.
	let mut eligible_ranges = 0;
	for lower in (minimum_lower.min(maximum_lower)..=minimum_lower.max(maximum_lower))
		.step_by(spacing as usize)
	{
		for upper in (minimum_upper.min(maximum_upper)..=minimum_upper.max(maximum_upper))
			.step_by(spacing as usize)
		{
			if band.principal_for_range(liquidity, lower, upper, spot).unwrap() ==
				PositionAmounts::default()
			{
				continue;
			}
			assert!(ranges.contains(&(lower, upper)), "missing ({lower}, {upper})");
			eligible_ranges += 1;
		}
	}
	assert_eq!(ranges.len(), eligible_ranges);
}

#[test]
fn a_coarse_grid_range_entirely_below_the_reward_band_is_skipped() {
	let target = FixedU128::one();
	let spacing = 1101;
	let band = StabilizerPoolRewardBand::new(
		target,
		target,
		true,
		spacing,
		RewardPriceBand::get(),
		REWARD_BAND_TOLERANCE,
	)
	.unwrap();
	let shifted_range = uniswap_bounds_for_usd_band(
		FixedU128::from_rational(891, 1000),
		FixedU128::from_rational(99, 100),
		target,
		true,
		spacing,
	);
	// The allowed shift and coarse grid can leave a valid NFT entirely below $0.90.
	assert_eq!(shifted_range, (-278_553, -277_452));
	assert!(!band.reward_ranges().unwrap().contains(&shifted_range));
}

enum PositionCorrection {
	Unsubscribe,
	RevisionAfterCheckpoint,
	LateUnsubscribe,
	Replacement,
}

#[rstest]
#[case(PositionCorrection::Unsubscribe)]
#[case(PositionCorrection::RevisionAfterCheckpoint)]
#[case(PositionCorrection::LateUnsubscribe)]
#[case(PositionCorrection::Replacement)]
fn invalidating_a_position_forfeits_unclaimed_rewards(#[case] correction: PositionCorrection) {
	let revision_report = matches!(correction, PositionCorrection::RevisionAfterCheckpoint);
	let late = matches!(correction, PositionCorrection::LateUnsubscribe);
	let owner_replaces = matches!(correction, PositionCorrection::Replacement);
	new_test_ext().execute_with(|| {
		let owner = AccountId32::new([0x42; 32]);
		let reporter = AccountId32::new([0x77; 32]);
		let position = StabilizerPoolPositionId {
			position_manager: config().position_manager,
			token_id: U256::one(),
		};
		let (lower, upper) = uniswap_bounds_for_usd_band(
			FixedU128::from_rational(90, 100),
			FixedU128::one(),
			FixedU128::one(),
			true,
			config().key.tick_spacing,
		);
		let liquidity = 10u128.pow(24);
		let mut checkpoint = position_checkpoint(liquidity, lower, upper);
		let enrollment_block = 95;
		checkpoint.proven_through = ProofBlock {
			hash: H256::from_low_u64_be(enrollment_block),
			number: enrollment_block,
			timestamp_millis: enrollment_block * 12_000,
		};
		checkpoint.maintained_since = checkpoint.proven_through;
		let enrollment = position_evidence(
			enrollment_block,
			1,
			H256::from_slice(owner.as_ref()),
			liquidity,
			true,
			(lower, upper),
		);
		assert_ok!(StabilizerPools::checkpoint_position(
			RuntimeOrigin::signed(owner.clone()),
			pool_manager(),
			pool_id(),
			position.token_id,
			Box::new(enrollment),
		));
		// Another NFT shares the same range. The correction must subtract only one contribution.
		assert_ok!(StabilizerPools::record_checkpoint(
			position.position_manager,
			2.into(),
			checkpoint.clone()
		));

		let spot = uniswap_sqrt_price_for_usd_price(
			FixedU128::from_rational(95, 100),
			FixedU128::one(),
			true,
			false,
		);
		start_mining_frame(2, 20);
		for tick in 20..30 {
			CurrentTick::set(tick);
			report_stabilizer_pool_price(tick, spot);
		}
		start_mining_frame(3, 30);
		let review = StabilizerPoolReviewFrame::<Test>::get().unwrap();
		assert_eq!(review.frame_id, 2);
		assert_eq!(
			review.liquidity_by_range.get(&(lower, upper)),
			Some(&U256::from(liquidity * 2))
		);

		if revision_report || late {
			// Refreshing an unchanged revision does not claim the frame or protect it from a
			// report.
			let continuity = position_evidence(
				151,
				1,
				H256::from_slice(owner.as_ref()),
				liquidity,
				true,
				(lower, upper),
			);
			assert_ok!(StabilizerPools::checkpoint_position(
				RuntimeOrigin::signed(owner.clone()),
				pool_manager(),
				pool_id(),
				position.token_id,
				Box::new(continuity),
			));
		}
		assert_ok!(StabilizerPools::settle_account(&owner), 0);
		assert!(PaidClaims::get().is_empty());
		if late {
			start_mining_frame(4, 40);
			// A failed automatic payout leaves rewards unclaimed for the late report to forfeit.
			RejectClaims::set(true);
			assert_noop!(
				with_storage_layer(|| StabilizerPools::settle_account(&owner)),
				DispatchError::Unavailable
			);
			RejectClaims::set(false);
		}

		if owner_replaces {
			let replacement_owner = AccountId32::new([0x17; 32]);
			let replacement = position_evidence(
				170,
				2,
				H256::from_slice(replacement_owner.as_ref()),
				liquidity * 2,
				true,
				(lower, upper),
			);
			assert_ok!(StabilizerPools::checkpoint_position(
				RuntimeOrigin::signed(replacement_owner),
				pool_manager(),
				pool_id(),
				position.token_id,
				Box::new(replacement),
			));
		} else {
			let unchanged = position_change_evidence(160, 1, liquidity, true, (lower, upper));
			assert_noop!(
				StabilizerPools::report_position_change(
					RuntimeOrigin::signed(reporter.clone()),
					position,
					Box::new(unchanged)
				),
				Error::<Test>::PositionChangeNotProven
			);
			let reported_liquidity = if revision_report { liquidity * 2 } else { liquidity };
			let changed = position_change_evidence(
				170,
				2,
				reported_liquidity,
				revision_report,
				(lower, upper),
			);
			assert_ok!(StabilizerPools::report_position_change(
				RuntimeOrigin::signed(reporter.clone()),
				position,
				Box::new(changed.clone())
			));
			assert_noop!(
				StabilizerPools::report_position_change(
					RuntimeOrigin::signed(reporter.clone()),
					position,
					Box::new(changed)
				),
				Error::<Test>::PositionNotEnrolled
			);
		}
		let expected_next_liquidity = if owner_replaces { liquidity * 3 } else { liquidity };
		assert_eq!(
			RegisteredLiquidityByRange::<Test>::get(StabilizerPoolLiquidityRange::from(
				&checkpoint
			))
			.next_frame_liquidity,
			U256::from(expected_next_liquidity)
		);

		let reviewed_liquidity = if late { liquidity * 2 } else { liquidity };
		if !late {
			assert_eq!(
				StabilizerPoolReviewFrame::<Test>::get()
					.unwrap()
					.liquidity_by_range
					.get(&(lower, upper)),
				Some(&U256::from(reviewed_liquidity))
			);
			start_mining_frame(4, 40);
		}
		let price = StabilizerPoolPricesByFrame::<Test>::get()[&2];
		let band = StabilizerPoolRewardBand::new(
			price.argon_usd_target_price,
			price.usdc_usd_price,
			price.argon_is_token0,
			config().key.tick_spacing,
			RewardPriceBand::get(),
			REWARD_BAND_TOLERANCE,
		)
		.unwrap();
		let expected = band
			.principal_for_range(reviewed_liquidity.into(), lower, upper, price.sqrt_price_x96)
			.unwrap();
		assert_ne!(expected, PositionAmounts::default());
		assert_eq!(
			FinalizedRewardValues::get().into_iter().find(|(frame, _)| *frame == 2),
			Some((2, expected.argon + expected.usdc)),
		);

		assert!(PaidClaims::get().is_empty());

		if owner_replaces {
			assert_eq!(
				StabilizerPoolRegistrations::<Test>::get(position),
				Some(StabilizerPoolRegistration::Enrolled(StabilizerPoolEnrollment {
					eligible_from: 4,
					next_claim_frame: 4
				}))
			);
			return;
		}

		assert_ok!(StabilizerPools::settle_account(&owner), 0);
		assert_eq!(
			StabilizerPoolRegistrations::<Test>::get(position),
			Some(StabilizerPoolRegistration::Removed {
				reported_at: ProofBlock {
					hash: H256::from_low_u64_be(170),
					number: 170,
					timestamp_millis: 2_040_000,
				},
			}),
		);

		checkpoint.proven_through = ProofBlock {
			hash: H256::from_low_u64_be(165),
			number: 165,
			timestamp_millis: 1_980_000,
		};
		assert_noop!(
			StabilizerPools::record_checkpoint(
				position.position_manager,
				position.token_id,
				checkpoint.clone()
			),
			Error::<Test>::StaleCheckpoint
		);
		// A revision report can be followed by the owner's proof from that same Ethereum block.
		let (block, revision, replacement_liquidity) =
			if revision_report { (170, 2, liquidity * 2) } else { (180, 3, liquidity) };
		let replacement = position_evidence(
			block,
			revision,
			H256::from_slice(owner.as_ref()),
			replacement_liquidity,
			true,
			(lower, upper),
		);
		assert_ok!(StabilizerPools::checkpoint_position(
			RuntimeOrigin::signed(owner.clone()),
			pool_manager(),
			pool_id(),
			position.token_id,
			Box::new(replacement),
		));
		assert_eq!(
			StabilizerPoolRegistrations::<Test>::get(position),
			Some(StabilizerPoolRegistration::Enrolled(StabilizerPoolEnrollment {
				eligible_from: 5,
				next_claim_frame: 5
			}))
		);
		// Re-enrollment starts a new cursor. The forfeited frame cannot be claimed again.
		start_mining_frame(5, 50);
		assert_ok!(StabilizerPools::settle_account(&owner), 0);
		assert!(PaidClaims::get().is_empty());
	});
}

#[test]
fn collect_pays_finalized_frames_covered_by_the_latest_checkpoint_once() {
	new_test_ext().execute_with(|| {
		let owner = AccountId32::new([0x42; 32]);
		let position = StabilizerPoolPositionId {
			position_manager: config().position_manager,
			token_id: U256::one(),
		};
		let argon_target = FixedU128::one();
		let usdc_quote = FixedU128::from_u32(2);
		let tick_spacing = config().key.tick_spacing;
		let (lower, upper) = uniswap_bounds_for_usd_band(
			FixedU128::from_rational(90, 100),
			argon_target,
			usdc_quote,
			true,
			tick_spacing,
		);
		let liquidity = 10u128.pow(24);
		let spot = uniswap_sqrt_price_for_usd_price(
			FixedU128::from_rational(95, 100),
			usdc_quote,
			true,
			false,
		);
		let build_proof = |ethereum_block| {
			position_evidence(
				ethereum_block,
				1,
				H256::from_slice(owner.as_ref()),
				liquidity,
				true,
				(lower, upper),
			)
		};
		let submit_proof = |proof| {
			StabilizerPools::checkpoint_position(
				RuntimeOrigin::signed(owner.clone()),
				pool_manager(),
				pool_id(),
				position.token_id,
				Box::new(proof),
			)
		};
		let sample_frame = |frame: FrameId| {
			for tick in (frame * 10)..((frame + 1) * 10) {
				CurrentTick::set(tick);
				let mut price_index = stabilizer_pool_price_index(tick, spot);
				price_index.usdc_usd_price = Some(usdc_quote);
				StabilizerPools::on_price_index(&price_index, Some(stabilizer_pool_price_source()));
			}
		};

		start_mining_frame(1, 10);
		assert_ok!(submit_proof(build_proof(45)));
		for frame in 1..=11 {
			// The oracle missed frame 5. Later frames can still have valid claims.
			if frame != 5 {
				sample_frame(frame);
			}
			start_mining_frame(frame + 1, (frame + 1) * 10);
		}
		assert_ok!(StabilizerPools::settle_account(&owner), 0);
		assert!(PaidClaims::get().is_empty());
		assert_ok!(submit_proof(build_proof(600)));
		assert!(PaidClaims::get().is_empty());
		assert_eq!(
			StabilizerPoolRegistrations::<Test>::get(position),
			Some(StabilizerPoolRegistration::Enrolled(StabilizerPoolEnrollment {
				eligible_from: 2,
				next_claim_frame: 2,
			})),
		);
		// Frame 2 has its tenth collectible frame at frame 13. Frame 12 is still under review.
		sample_frame(12);
		start_mining_frame(13, 130);
		let (_, last_day_claims) =
			StabilizerPools::value_position_claim(&checkpoint(), 2, 13).unwrap();
		assert_eq!(
			last_day_claims.iter().map(|claim| claim.frame_id).collect::<Vec<_>>(),
			vec![2, 3, 4, 6, 7, 8, 9, 10, 11],
		);
		// The next frame expires that claim even though its valuation price is retained.
		sample_frame(13);
		start_mining_frame(14, 140);
		assert!(StabilizerPoolPricesByFrame::<Test>::get().contains_key(&2));
		// A later runtime split cannot change the weighting retained for past frames.
		UsdcRewardShare::set(Percent::from_percent(50));
		assert_ok!(StabilizerPools::settle_account(&owner), 1);
		let claims = PaidClaims::get();
		let [claim] = claims.as_slice() else { panic!("expected one accepted claim") };
		assert_eq!(
			claim.frames.iter().map(|frame| frame.frame_id).collect::<Vec<_>>(),
			vec![3, 4, 6, 7, 8, 9, 10, 11],
		);
		assert_eq!(
			StabilizerPoolRegistrations::<Test>::get(position),
			Some(StabilizerPoolRegistration::Enrolled(StabilizerPoolEnrollment {
				eligible_from: 2,
				next_claim_frame: 12
			})),
		);

		let sampled_price = StabilizerPoolPricesByFrame::<Test>::get()[&3].sqrt_price_x96;
		let band = StabilizerPoolRewardBand::new(
			argon_target,
			usdc_quote,
			true,
			tick_spacing,
			RewardPriceBand::get(),
			REWARD_BAND_TOLERANCE,
		)
		.unwrap();
		let principal =
			band.principal_for_range(liquidity.into(), lower, upper, sampled_price).unwrap();
		assert!(!principal.usdc.is_zero());
		assert_eq!(
			claim.frames[0],
			StabilizerPoolFrameClaim {
				frame_id: 3,
				amount: Percent::from_percent(10).mul_floor(principal.argon) +
					Percent::from_percent(90).mul_floor(principal.usdc * 2),
			},
		);
		assert_eq!(
			FinalizedRewardValues::get().into_iter().find(|(frame, _)| *frame == 3),
			Some((3, principal.argon + principal.usdc * 2)),
		);

		assert_ok!(StabilizerPools::settle_account(&owner), 0);
		assert_eq!(PaidClaims::get().len(), 1);
		assert_ok!(submit_proof(build_proof(650)));
		RejectClaims::set(true);
		assert_noop!(
			with_storage_layer(|| StabilizerPools::settle_account(&owner)),
			DispatchError::Unavailable
		);
		assert_eq!(PaidClaims::get().len(), 1);
		RejectClaims::set(false);
		start_mining_frame(15, 150);
		assert_ok!(StabilizerPools::settle_account(&owner), 1);
		start_mining_frame(16, 160);
		assert_ok!(StabilizerPools::settle_account(&owner), 0);
		let claims = PaidClaims::get();
		let [_, last_claim] = claims.as_slice() else { panic!("expected one additional claim") };
		let [last_frame] = last_claim.frames.as_slice() else { panic!("expected one new frame") };
		assert_eq!(last_frame.frame_id, 12);
		assert_eq!(checkpoint().proven_through.number, 650);
	});
}
