//! GRANDPA authority derivation from mining-slot activity.
//!
//! Policy:
//! - Operators are eligible when they have recent mining activity inside the configured recency
//!   window (`GrandpaRotationBlocks * GrandpaRecentActivityWindowInRotations`).
//! - The derived GRANDPA set is bounded to `MaxGrandpaAuthorities` highest-weight keys.
//! - Eligible operators are weighted by recent activity and normalized into a fixed vote budget
//!   (`GrandpaTotalVoteWeight`) so authority weights are deterministic each rotation.
//! - Per-operator influence is bounded by `MaxGrandpaAuthorityWeightPercent`.
//! - If active authorities are below the minimum needed for the configured max share (`ceil(100 /
//!   MaxGrandpaAuthorityWeightPercent)`), all eligible authorities are included with equal weight
//!   so finality can continue.
//!
//! Allocation flow:
//! 1. Gather raw operator weights from recent activity.
//! 2. If the set is too small, retry once with a longer recency window.
//! 3. Keep the highest-weight authorities up to `MaxGrandpaAuthorities`.
//! 4. If the set is still below the minimum feasible size, assign equal weights.
//! 5. Otherwise, scale proportionally into the fixed vote budget and enforce per-authority max.
use crate::sp_runtime::{
	Percent, RuntimeAppPublic,
	traits::{OpaqueKeys, UniqueSaturatedInto},
};
use alloc::{collections::BTreeMap, vec::Vec};
use codec::Decode;

use crate::{Config, Get, MinerNonceScoringByCohort, MinersByCohort, Pallet};

pub type AuthorityWeight = u64;
pub type AuthoritySet<K> = Vec<(K, AuthorityWeight)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeriveAuthoritiesError {
	RegisteredMiningInactive,
	NoEligibleWeights,
}

#[derive(Clone)]
struct AuthorityEntry<K> {
	key: K,
	raw_weight: AuthorityWeight,
	allocated_weight: AuthorityWeight,
}

impl<K> AuthorityEntry<K> {
	fn new(key: K, raw_weight: AuthorityWeight) -> Self {
		Self { key, raw_weight, allocated_weight: 0 }
	}
}

pub fn derive_authorities_from_recent_mining<T, K>()
-> Result<AuthoritySet<K>, DeriveAuthoritiesError>
where
	T: Config,
	K: Decode + RuntimeAppPublic + Ord,
{
	if !Pallet::<T>::is_registered_mining_active() {
		return Err(DeriveAuthoritiesError::RegisteredMiningInactive);
	}

	let max_operator_share_percent = T::MaxGrandpaAuthorityWeightPercent::get();
	let minimum_authorities_needed = minimum_authority_count_for_share(max_operator_share_percent);
	let recency_window_blocks =
		UniqueSaturatedInto::<u32>::unique_saturated_into(T::GrandpaRotationBlocks::get())
			.saturating_mul(T::GrandpaRecentActivityWindowInRotations::get());
	let current_block: u32 =
		crate::frame_system::Pallet::<T>::block_number().unique_saturated_into();
	let mut operator_weight_by_grandpa_key =
		collect_recent_operator_weights::<T, K>(current_block, recency_window_blocks);

	if operator_weight_by_grandpa_key.len() < minimum_authorities_needed {
		let fallback_window_blocks =
			recency_window_blocks.saturating_mul(T::GrandpaRecencyWindowFallbackMultiplier::get());
		if fallback_window_blocks > recency_window_blocks {
			let fallback_operator_weight_by_grandpa_key =
				collect_recent_operator_weights::<T, K>(current_block, fallback_window_blocks);
			if fallback_operator_weight_by_grandpa_key.len() > operator_weight_by_grandpa_key.len()
			{
				operator_weight_by_grandpa_key = fallback_operator_weight_by_grandpa_key;
			}
		}
	}

	derive_authorities_from_weights::<T, K>(
		operator_weight_by_grandpa_key,
		max_operator_share_percent,
		T::MaxGrandpaAuthorities::get(),
	)
}

pub fn derive_authorities_from_weights<T, K>(
	operator_weight_by_grandpa_key: BTreeMap<K, AuthorityWeight>,
	max_operator_share_percent: Percent,
	max_grandpa_authorities: u32,
) -> Result<AuthoritySet<K>, DeriveAuthoritiesError>
where
	T: Config,
	K: Ord,
{
	let total_grandpa_vote_weight = T::GrandpaTotalVoteWeight::get();
	let operator_weight_by_grandpa_key =
		select_top_authority_candidates(operator_weight_by_grandpa_key, max_grandpa_authorities);
	if operator_weight_by_grandpa_key.is_empty() {
		return Err(DeriveAuthoritiesError::NoEligibleWeights);
	}

	// Explicitly disable share limits when configured to zero.
	if max_operator_share_percent == Percent::from_percent(0) {
		return Ok(canonical_authority_order(
			operator_weight_by_grandpa_key
				.into_iter()
				.map(|entry| (entry.key, entry.raw_weight))
				.collect(),
		));
	}

	let minimum_authorities_needed = minimum_authority_count_for_share(max_operator_share_percent);
	if operator_weight_by_grandpa_key.len() < minimum_authorities_needed {
		return Ok(equal_weight_authorities(
			operator_weight_by_grandpa_key,
			total_grandpa_vote_weight,
		));
	}

	Ok(allocate_weighted_authorities(
		operator_weight_by_grandpa_key,
		max_operator_share_percent,
		total_grandpa_vote_weight,
	))
}

fn collect_recent_operator_weights<T, K>(
	current_block: u32,
	recency_window_blocks: u32,
) -> BTreeMap<K, AuthorityWeight>
where
	T: Config,
	K: Decode + RuntimeAppPublic + Ord,
{
	let scoring_by_cohort = MinerNonceScoringByCohort::<T>::get();
	let mut operator_weight_by_grandpa_key = BTreeMap::<K, AuthorityWeight>::new();

	for (frame_id, cohort) in MinersByCohort::<T>::iter() {
		let Some(scores) = scoring_by_cohort.get(&frame_id) else {
			continue;
		};

		for (index, miner) in cohort.iter().enumerate() {
			let Some(scoring) = scores.get(index) else {
				continue;
			};
			let Some(last_win_block) = scoring.last_win_block else {
				continue;
			};
			let Some(key) = miner.authority_keys.get::<K>(K::ID) else {
				continue;
			};
			let last_win_block: u32 = last_win_block.unique_saturated_into();
			if current_block.saturating_sub(last_win_block) > recency_window_blocks {
				continue;
			}

			operator_weight_by_grandpa_key
				.entry(key)
				.and_modify(|weight| *weight = weight.saturating_add(1))
				.or_insert(1);
		}
	}

	operator_weight_by_grandpa_key
}

fn select_top_authority_candidates<K: Ord>(
	operator_weight_by_grandpa_key: BTreeMap<K, AuthorityWeight>,
	max_grandpa_authorities: u32,
) -> Vec<AuthorityEntry<K>> {
	let mut operator_weight_by_grandpa_key = operator_weight_by_grandpa_key
		.into_iter()
		.filter(|(_, raw_weight)| *raw_weight > 0)
		.map(|(key, raw_weight)| AuthorityEntry::new(key, raw_weight))
		.collect::<Vec<_>>();
	operator_weight_by_grandpa_key.sort_by(|left, right| {
		right.raw_weight.cmp(&left.raw_weight).then_with(|| left.key.cmp(&right.key))
	});
	operator_weight_by_grandpa_key.truncate(max_grandpa_authorities.max(1) as usize);
	operator_weight_by_grandpa_key
}

fn minimum_authority_count_for_share(max_operator_share_percent: Percent) -> usize {
	if max_operator_share_percent == Percent::from_percent(0) {
		return 1;
	}
	let share_percent = max_operator_share_percent.deconstruct().max(1) as u64;
	100u64.div_ceil(share_percent) as usize
}

fn equal_weight_authorities<K: Ord>(
	mut operator_weight_by_grandpa_key: Vec<AuthorityEntry<K>>,
	total_grandpa_vote_weight: AuthorityWeight,
) -> AuthoritySet<K> {
	if operator_weight_by_grandpa_key.is_empty() {
		return Vec::new();
	}

	operator_weight_by_grandpa_key.sort_by(|left, right| left.key.cmp(&right.key));
	let operator_count = operator_weight_by_grandpa_key.len() as AuthorityWeight;
	let base_weight = total_grandpa_vote_weight.checked_div(operator_count).unwrap_or_default();
	let remainder = total_grandpa_vote_weight.saturating_sub(base_weight * operator_count);

	operator_weight_by_grandpa_key
		.into_iter()
		.enumerate()
		.map(|(index, entry)| {
			let extra = u64::from((index as AuthorityWeight) < remainder);
			(entry.key, base_weight.saturating_add(extra))
		})
		.collect()
}

fn allocate_weighted_authorities<K: Ord>(
	mut operator_weight_by_grandpa_key: Vec<AuthorityEntry<K>>,
	max_operator_share_percent: Percent,
	total_grandpa_vote_weight: AuthorityWeight,
) -> AuthoritySet<K> {
	let max_operator_weight =
		max_operator_share_percent.mul_floor(total_grandpa_vote_weight).max(1);
	for entry in operator_weight_by_grandpa_key.iter_mut() {
		// Seed each selected operator with one vote unit so no selected authority is dropped.
		entry.allocated_weight = 1;
	}

	let total_raw_weight = operator_weight_by_grandpa_key
		.iter()
		.map(|entry| entry.raw_weight)
		.sum::<AuthorityWeight>();
	if total_raw_weight == 0 {
		return equal_weight_authorities(operator_weight_by_grandpa_key, total_grandpa_vote_weight);
	}

	let operator_count = operator_weight_by_grandpa_key.len() as AuthorityWeight;
	let distributable_weight = total_grandpa_vote_weight.saturating_sub(operator_count);
	for entry in operator_weight_by_grandpa_key.iter_mut() {
		let proportional_weight = entry
			.raw_weight
			.saturating_mul(distributable_weight)
			.checked_div(total_raw_weight)
			.unwrap_or_default();
		let headroom = max_operator_weight.saturating_sub(entry.allocated_weight);
		let additional_weight = proportional_weight.min(headroom);
		entry.allocated_weight = entry.allocated_weight.saturating_add(additional_weight);
	}

	let allocated_weight_total = operator_weight_by_grandpa_key
		.iter()
		.map(|entry| entry.allocated_weight)
		.sum::<AuthorityWeight>();
	let mut remaining_weight = total_grandpa_vote_weight.saturating_sub(allocated_weight_total);

	// Top up once after max enforcement so the authority set stays at the target vote budget.
	if remaining_weight > 0 {
		let mut redistribution_order = operator_weight_by_grandpa_key
			.iter()
			.enumerate()
			.filter(|(_, entry)| entry.allocated_weight < max_operator_weight)
			.map(|(index, _)| index)
			.collect::<Vec<_>>();
		redistribution_order.sort_by(|left, right| {
			let left_entry = &operator_weight_by_grandpa_key[*left];
			let right_entry = &operator_weight_by_grandpa_key[*right];
			right_entry
				.raw_weight
				.cmp(&left_entry.raw_weight)
				.then_with(|| left_entry.key.cmp(&right_entry.key))
		});

		let mut operators_with_headroom = redistribution_order.len() as u64;
		for index in redistribution_order {
			if remaining_weight == 0 || operators_with_headroom == 0 {
				break;
			}

			let entry = &mut operator_weight_by_grandpa_key[index];
			let headroom = max_operator_weight.saturating_sub(entry.allocated_weight);
			let even_share = remaining_weight.div_ceil(operators_with_headroom);
			let additional_weight = headroom.min(even_share);
			entry.allocated_weight = entry.allocated_weight.saturating_add(additional_weight);
			remaining_weight = remaining_weight.saturating_sub(additional_weight);
			operators_with_headroom = operators_with_headroom.saturating_sub(1);
		}
	}

	canonical_authority_order(
		operator_weight_by_grandpa_key
			.into_iter()
			.map(|entry| (entry.key, entry.allocated_weight))
			.collect(),
	)
}

fn canonical_authority_order<K: Ord>(
	mut operator_weight_by_grandpa_key: Vec<(K, AuthorityWeight)>,
) -> AuthoritySet<K> {
	operator_weight_by_grandpa_key.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));
	operator_weight_by_grandpa_key
}
