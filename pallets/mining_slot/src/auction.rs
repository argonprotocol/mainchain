//! Deterministic batch allocation. Work depends on candidate count, never bid distance.
use alloc::vec::Vec;

#[derive(Clone, Debug)]
pub(crate) struct Candidate {
	pub low: u128,
	pub high: u128,
	pub fixed: bool,
}

/// Input order is incumbent order followed by commitment order. Stable sorts retain ties.
pub(crate) fn clear(candidates: &[Candidate], seats: usize, increment: u128) -> Vec<(usize, u128)> {
	let mut order: Vec<usize> = (0..candidates.len()).collect();
	order.sort_by(|a, b| {
		candidates[*b]
			.high
			.cmp(&candidates[*a].high)
			.then_with(|| candidates[*b].fixed.cmp(&candidates[*a].fixed))
	});
	let cutoff = order.get(seats).map(|i| candidates[*i].high.saturating_add(increment));
	order
		.into_iter()
		.take(seats)
		.map(|i| {
			let c = &candidates[i];
			let price =
				if c.fixed { c.high } else { c.low.max(cutoff.unwrap_or(c.low).min(c.high)) };
			(i, price)
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn agreed_examples_and_ties() {
		let mut bids = vec![Candidate { low: 10000, high: 10000, fixed: true }; 10];
		bids.push(Candidate { low: 9000, high: 15000, fixed: false });
		assert_eq!(clear(&bids, 10, 1)[0], (10, 10001));
		let mut bids = vec![Candidate { low: 9500, high: 12500, fixed: false }; 10];
		bids.insert(0, Candidate { low: 9000, high: 15000, fixed: false });
		let result = clear(&bids, 10, 1);
		assert_eq!(result[0], (0, 12501));
		assert!(result[1..].iter().all(|(_, p)| *p == 12500));
		assert!(!result.iter().any(|(i, _)| *i == 10));
	}
	#[test]
	fn floors_underfilled_zero_and_overflow() {
		let bids = vec![
			Candidate { low: 0, high: 0, fixed: false },
			Candidate { low: 30, high: u128::MAX, fixed: false },
		];
		assert_eq!(clear(&bids, 3, 1), vec![(1, 30), (0, 0)]);
		assert!(clear(&bids, 0, 1).is_empty());
		let bids = vec![Candidate { low: 0, high: u128::MAX, fixed: false }; 2];
		assert_eq!(clear(&bids, 1, 1), vec![(0, u128::MAX)]);
	}
}
