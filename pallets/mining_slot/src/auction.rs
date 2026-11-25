//! Deterministic auction allocation with one spending limit per funding wallet.
use alloc::{collections::BTreeSet, vec::Vec};
use polkadot_sdk::sp_core::U256;

#[derive(Clone, Debug)]
pub(crate) struct Candidate {
	/// Public bid or already-paid floor, in microgons.
	pub bid: u128,
	/// Stored willingness to pay; it is not a reservation of wallet funds.
	pub max: u128,
	/// Public fixed bids pay their stated bid, rather than the competitive price.
	pub fixed: bool,
	/// Index into the auction's shared wallet budgets.
	pub wallet: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Demand {
	pub max: u128,
	pub fixed: bool,
	pub priority: usize,
	pub wallet: usize,
}

/// Each wallet offers quantities, not independently funded per-miner maximums.
/// Work is bounded by offer count and maximum breakpoints, never bid distance.
pub(crate) fn demand(candidates: &[Candidate], budgets: &[u128], increment: u128) -> Vec<Demand> {
	if increment == 0 {
		return Vec::new();
	}
	let mut demand = Vec::new();
	for (wallet, budget) in budgets.iter().copied().enumerate() {
		let indices = candidates
			.iter()
			.enumerate()
			.filter_map(|(index, candidate)| (candidate.wallet == wallet).then_some(index))
			.collect::<Vec<_>>();
		let limits = quantity_limits(candidates, &indices, budget, increment);
		let mut start = 0;
		while start < limits.len() {
			let max = limits[start];
			let mut end = start + 1;
			while end < limits.len() && limits[end] == max {
				end += 1;
			}
			let below = affordable(candidates, &indices, budget, max);
			let above = max
				.checked_add(increment)
				.map(|price| affordable(candidates, &indices, budget, price))
				.unwrap_or_default()
				.into_iter()
				.collect::<BTreeSet<_>>();
			let mut added =
				below.into_iter().filter(|index| !above.contains(index)).collect::<Vec<_>>();
			added.sort_by(|a, b| {
				let fixed = |index: usize| candidates[index].fixed && candidates[index].max == max;
				fixed(*b).cmp(&fixed(*a)).then(a.cmp(b))
			});
			// A cheaper price can replace expensive paid floors as well as add quantity.
			// Match the higher-price units to the strongest replacement offers first;
			// the additional units inherit the remaining offers' fixed/commit priority.
			let additional = end - start;
			let replacements = added.len().saturating_sub(additional);
			for priority in added.into_iter().skip(replacements) {
				demand.push(Demand {
					max,
					fixed: candidates[priority].fixed && candidates[priority].max == max,
					priority,
					wallet,
				});
			}
			start = end;
		}
	}
	demand.sort_by(|a, b| {
		b.max
			.cmp(&a.max)
			.then_with(|| b.fixed.cmp(&a.fixed))
			.then(a.priority.cmp(&b.priority))
	});
	demand
}

/// Winners from one wallet share a competitive price capped by that quantity's
/// affordability. Clipping each unit separately can spend more than the wallet owns.
pub(crate) fn clear(
	candidates: &[Candidate],
	budgets: &[u128],
	seats: usize,
	increment: u128,
) -> Vec<(usize, u128)> {
	if seats == 0 {
		return Vec::new();
	}
	let offers = demand(candidates, budgets, increment);
	let cutoff = offers.get(seats).map(|offer| offer.max.saturating_add(increment));
	let mut quantities = alloc::vec![0usize; budgets.len()];
	for offer in offers.iter().take(seats) {
		quantities[offer.wallet] += 1;
	}
	let mut winners = Vec::new();
	for (wallet, quantity) in quantities.into_iter().enumerate() {
		if quantity == 0 {
			continue;
		}
		let indices = candidates
			.iter()
			.enumerate()
			.filter_map(|(index, candidate)| (candidate.wallet == wallet).then_some(index))
			.collect::<Vec<_>>();
		let limits = quantity_limits(candidates, &indices, budgets[wallet], increment);
		let price = cutoff.unwrap_or(0).min(limits[quantity - 1]);
		let eligible = indices
			.into_iter()
			.filter(|index| candidates[*index].max >= price)
			.collect::<Vec<_>>();
		for index in select_quantity(candidates, &eligible, budgets[wallet], price, quantity) {
			winners.push((index, candidates[index].bid.max(price)));
		}
	}
	winners
}

/// Highest common price supporting some affordable set of each quantity. At an
/// offer's maximum breakpoint, the cheapest eligible floors minimize total cost.
fn quantity_limits(
	candidates: &[Candidate],
	indices: &[usize],
	budget: u128,
	increment: u128,
) -> Vec<u128> {
	let mut by_max = indices.to_vec();
	by_max.sort_by(|a, b| candidates[*b].max.cmp(&candidates[*a].max));
	let mut active = Vec::new();
	let mut limits = Vec::new();
	let mut next = 0;
	while next < by_max.len() {
		let maximum = candidates[by_max[next]].max;
		while next < by_max.len() && candidates[by_max[next]].max == maximum {
			let floor = candidates[by_max[next]].bid;
			let position = active.binary_search(&floor).unwrap_or_else(|position| position);
			active.insert(position, floor);
			next += 1;
		}
		let mut sums = alloc::vec![0u128];
		let mut leveled = 0;
		for (index, floor) in active.iter().copied().enumerate() {
			let Some(total) = sums[index].checked_add(floor) else {
				break;
			};
			if total > budget {
				break;
			}
			sums.push(total);
			if leveled == index {
				leveled += 1;
			}
			let mut price = (budget - (total - sums[leveled])) / leveled as u128;
			while price < active[leveled - 1] {
				leveled -= 1;
				price = (budget - (total - sums[leveled])) / leveled as u128;
			}
			price = price.min(maximum);
			price -= price % increment;
			if limits.len() <= index {
				limits.push(price);
			} else {
				limits[index] = limits[index].max(price);
			}
		}
	}
	limits
}

fn affordable(
	candidates: &[Candidate],
	indices: &[usize],
	budget: u128,
	price: u128,
) -> Vec<usize> {
	let mut cheapest = indices
		.iter()
		.copied()
		.filter(|index| candidates[*index].max >= price)
		.collect::<Vec<_>>();
	cheapest.sort_by_key(|index| candidates[*index].bid.max(price));
	let mut remaining = budget;
	let mut quantity = 0;
	for index in &cheapest {
		let cost = candidates[*index].bid.max(price);
		if cost > remaining {
			break;
		}
		remaining -= cost;
		quantity += 1;
	}
	select_quantity(candidates, &cheapest, budget, price, quantity)
}

fn select_quantity(
	candidates: &[Candidate],
	eligible: &[usize],
	budget: u128,
	price: u128,
	quantity: usize,
) -> Vec<usize> {
	let mut preferred = eligible.to_vec();
	preferred.sort_by(|a, b| {
		candidates[*b]
			.fixed
			.cmp(&candidates[*a].fixed)
			.then_with(|| candidates[*b].max.cmp(&candidates[*a].max))
			.then(a.cmp(b))
	});

	let mut unallocated = preferred
		.iter()
		.map(|index| (candidates[*index].bid.max(price), *index))
		.collect::<BTreeSet<_>>();
	let mut cheapest: BTreeSet<(u128, usize)> = BTreeSet::new();
	let mut completion_cost = U256::zero();
	let mut remaining = budget;
	let mut selected = Vec::new();
	for index in preferred {
		if selected.len() == quantity {
			break
		}
		let cost = candidates[index].bid.max(price);
		let entry = (cost, index);
		if cheapest.remove(&entry) {
			completion_cost -= U256::from(cost);
		} else {
			unallocated.remove(&entry);
		}
		let needed = quantity - selected.len() - 1;
		while cheapest.len() > needed {
			let entry = cheapest.pop_last().expect("nonempty completion");
			completion_cost -= U256::from(entry.0);
			unallocated.insert(entry);
		}
		while cheapest.len() < needed {
			let Some(entry) = unallocated.pop_first() else { break };
			completion_cost += U256::from(entry.0);
			cheapest.insert(entry);
		}
		if cost > remaining {
			continue
		}
		if cheapest.len() != needed {
			continue
		}
		if completion_cost > U256::from(remaining - cost) {
			continue
		}
		remaining -= cost;
		selected.push(index);
	}
	selected
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn shared_quantity_ties_cannot_spend_more_than_the_wallet_budget() {
		let bids = [
			Candidate { bid: 0, max: 100, fixed: false, wallet: 0 },
			Candidate { bid: 0, max: 100, fixed: false, wallet: 0 },
			Candidate { bid: 0, max: 50, fixed: false, wallet: 1 },
		];
		assert_eq!(clear(&bids, &[100, 50], 2, 1), vec![(0, 50), (1, 50)]);
	}

	#[test]
	fn expensive_paid_floors_can_yield_to_two_affordable_seats() {
		let bids = [
			Candidate { bid: 80, max: 100, fixed: false, wallet: 0 },
			Candidate { bid: 10, max: 90, fixed: false, wallet: 0 },
			Candidate { bid: 10, max: 90, fixed: false, wallet: 0 },
			Candidate { bid: 40, max: 40, fixed: true, wallet: 1 },
			Candidate { bid: 30, max: 30, fixed: true, wallet: 2 },
		];
		assert_eq!(quantity_limits(&bids, &[0, 1, 2], 100, 1), vec![100, 50, 10]);
		assert_eq!(clear(&bids, &[100, 40, 30], 3, 1), vec![(1, 31), (2, 31), (3, 40)]);
	}

	#[test]
	fn fixed_bids_keep_their_payment_and_priority_at_the_marginal_price() {
		let bids = [
			Candidate { bid: 0, max: 100, fixed: false, wallet: 0 },
			Candidate { bid: 50, max: 50, fixed: true, wallet: 0 },
			Candidate { bid: 0, max: 100, fixed: false, wallet: 0 },
			Candidate { bid: 50, max: 50, fixed: true, wallet: 1 },
		];
		assert_eq!(clear(&bids, &[100, 50], 2, 1), vec![(1, 50), (0, 50)]);
		let bids = [
			Candidate { bid: 1, max: 100, fixed: false, wallet: 0 },
			Candidate { bid: 90, max: 90, fixed: true, wallet: 0 },
		];
		assert_eq!(clear(&bids, &[91], 2, 1), vec![(1, 90), (0, 1)]);
	}

	#[test]
	fn independent_wallets_keep_the_existing_cutoff_and_paid_floors() {
		let mut bids = (0..10)
			.map(|wallet| Candidate { bid: 10000, max: 10000, fixed: true, wallet })
			.collect::<Vec<_>>();
		bids.push(Candidate { bid: 9000, max: 15000, fixed: false, wallet: 10 });
		let result = clear(&bids, &[20000; 11], 10, 1);
		assert!(result.contains(&(10, 10001)));
		assert!(!result.iter().any(|(index, _)| *index == 9));
		let bids = [Candidate { bid: 30, max: u128::MAX, fixed: false, wallet: 0 }];
		assert_eq!(clear(&bids, &[u128::MAX], 3, 1), vec![(0, 30)]);
		assert!(clear(&bids, &[u128::MAX], 0, 1).is_empty());
		let bids = [
			Candidate { bid: 0, max: u128::MAX, fixed: false, wallet: 0 },
			Candidate { bid: 0, max: u128::MAX, fixed: false, wallet: 1 },
		];
		assert_eq!(clear(&bids, &[u128::MAX; 2], 1, 1), vec![(0, u128::MAX)]);
	}
	#[test]
	fn wallet_quantities_match_affordable_subsets_and_never_overspend() {
		for seed in 0usize..256 {
			let budgets = [(seed % 23) as u128, ((seed * 3) % 19) as u128];
			let bids = (0..5)
				.map(|index| {
					let bid = ((seed >> index) % 7) as u128;
					let fixed = seed & (1 << index) != 0;
					Candidate {
						bid,
						max: if fixed { bid } else { bid + ((seed * (index + 1)) % 13) as u128 },
						fixed,
						wallet: usize::from(index >= 3),
					}
				})
				.collect::<Vec<_>>();
			let offers = demand(&bids, &budgets, 1);
			for (wallet, budget) in budgets.iter().copied().enumerate() {
				let indices = bids
					.iter()
					.enumerate()
					.filter_map(|(index, bid)| (bid.wallet == wallet).then_some(index))
					.collect::<Vec<_>>();
				for price in 0..=20 {
					let mut affordable_count = 0;
					for subset in 0usize..(1 << indices.len()) {
						let selected = indices
							.iter()
							.enumerate()
							.filter_map(|(position, index)| {
								(subset & (1 << position) != 0).then_some(*index)
							})
							.collect::<Vec<_>>();
						if selected.iter().any(|index| bids[*index].max < price) {
							continue
						}
						let cost =
							selected.iter().map(|index| bids[*index].bid.max(price)).sum::<u128>();
						if cost <= budget {
							affordable_count = affordable_count.max(selected.len());
						}
					}
					assert_eq!(
						offers
							.iter()
							.filter(|offer| offer.wallet == wallet && offer.max >= price)
							.count(),
						affordable_count,
						"seed {seed}, wallet {wallet}, price {price}"
					);
				}
			}
			for seats in 0..=5 {
				let winners = clear(&bids, &budgets, seats, 1);
				assert_eq!(winners.len(), seats.min(offers.len()));
				for (wallet, budget) in budgets.iter().copied().enumerate() {
					let spent = winners
						.iter()
						.filter(|(index, _)| bids[*index].wallet == wallet)
						.map(|(_, price)| *price)
						.sum::<u128>();
					assert!(spent <= budget, "seed {seed}, wallet {wallet}, seats {seats}");
				}
				let mut seen = BTreeSet::new();
				for (index, price) in winners {
					assert!(seen.insert(index));
					assert!(price >= bids[index].bid && price <= bids[index].max);
					if bids[index].fixed {
						assert_eq!(price, bids[index].bid);
					}
				}
			}
		}
	}
}
