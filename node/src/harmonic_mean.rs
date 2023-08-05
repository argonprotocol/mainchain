// Radicle Registry
// Copyright (C) 2019 Monadic GmbH <radicle@monadic.xyz>
// Copyright 2023 Ulixee
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Calculates harmonic mean of a series of values
//!
//! The algorithm is designed to use only integers. It also makes no rounding except for the final
//! step, when the final result is rounded down to to an integer.
//!
//! # How it works
//! The harmonic mean of `count` items `a`, `b`, `c` ... is defined as:
//! ```raw
//! count / (1/a + 1/b + 1/c + ... )
//! ```
//! This is equivalent to:
//! ```raw
//! count * a * b * c * ... / (b * c... + a * c... + a * b... + ...)
//! ```
//!
//! ## Adding values
//! Let's take a look at an example of harmonic mean of `a`, `b` and `c`:
//! ```raw
//! 3 * a * b * c / (b * c + a * c + a * b)
//! ```
//! After adding value `d` it becomes:
//! ```raw
//! 4 * a * b * c * d / (b * c * d + a * c * d + a * b * d + a * b * c)
//! ```
//! Which is equivalent to:
//! ```raw
//! (3 + 1) * (a * b * c) * d / ((b * c + a * c + a * b) * d + (a * b * c))
//! ```
//! Which can be described as:
//! ```raw
//! (count + 1) * nominator * value / (denominator * value + nominator)
//! ```
//! Which can be broken up into updates of individual variables:
//! ```raw
//! count = count + 1
//! denominator = denominator * value + nominator
//! nominator = nominator * value
//! mean = count * nominator / denominator
//! ```
//!
//! ## The initial state
//! The only exception is addition of the first item, which should go from the initial state:
//! ```raw
//! 0 * 1 / 1 = 0
//! ```
//! Which can be broken up into individual variables:
//! ```raw
//! count = 0
//! denominator = 1
//! nominator = 1
//! mean = count * nominator / denominator = 0
//! ```
//! To:
//! ```raw
//! 1 * a / 1 = a
//! ```
//! Which can be described as:
//! ```raw
//! (count + 1) * nominator * value / denominator
//! ```
//! Which can be broken up into updates of individual variables:
//! ```raw
//! count = count + 1
//! denominator = denominator
//! nominator = nominator * value
//! mean = count * nominator / denominator = value
//! ```
//! The only difference from a normal variables update is skipping of the update of `denominator`.
//! The updates of `count` `nominator` and `mean` are all unchanged.

use std::ops::{AddAssign, Div, Mul, MulAssign};

use sp_core::U256;

#[derive(Debug)]
pub struct HarmonicMean {
	nominator: U256,
	denominator: U256,
	count: u32,
}

impl HarmonicMean {
	pub fn new() -> Self {
		HarmonicMean { nominator: U256::from(1), denominator: U256::from(1), count: 0 }
	}

	pub fn push(&mut self, value: U256) {
		if self.count > 0 {
			self.denominator.mul_assign(value);
			self.denominator.add_assign(self.nominator)
		}
		self.nominator.mul_assign(value);
		self.count += 1;
	}

	pub fn calculate(self) -> U256 {
		self.nominator.mul(self.count).div(self.denominator)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn harmonic_mean() {
		let max = U256::MAX;
		assert_mean(0, &[]);
		assert_mean(0, &[0]);
		assert_mean(1, &[1]);
		assert_mean(1, &[1, 1]);
		assert_mean(2, &[1, 4, 4]);
		assert_mean(max, &[max]);
		assert_mean(max, &[max, max]);
	}

	fn assert_mean<I>(expected: I, inputs: &[I])
	where
		I: Clone + std::fmt::Debug + Into<U256>,
	{
		let mut mean = HarmonicMean::new();
		for input in inputs {
			mean.push(input.clone().into());
		}
		assert_eq!(expected.into(), mean.calculate(), "Invalid result for inputs {:?}", inputs);
	}
}
