use argon_primitives::{argon_utils::format_argons, bitcoin::SATOSHIS_PER_BITCOIN};
use std::fmt;

pub fn parse_number(s: &str) -> Result<f32, String> {
	// Remove commas from the string
	let cleaned: String = s.chars().filter(|&c| c.is_ascii_digit() || c == '.').collect();

	// Ensure there's a decimal point for integer numbers by appending ".0" if needed
	let cleaned = if !cleaned.contains('.') { format!("{}.0", cleaned) } else { cleaned };

	// Parse the cleaned string as an f32
	let number: f32 = cleaned.parse().map_err(|_| "Invalid number".to_string())?;
	Ok(number)
}

pub struct Argons(pub f32);

impl fmt::Display for Argons {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.0;
		let whole_part = value.floor(); // Extract the whole part
		let whole_part_str = insert_commas(whole_part as u128);

		let decimal_part = (value.fract() * 100.0).round(); // Extract and round the decimal part

		if decimal_part == 0.0 {
			write!(f, "₳{}", whole_part_str)
		} else {
			write!(f, "₳{}.{:02}", whole_part_str, decimal_part as u32)
		}
	}
}

pub struct ArgonFormatter(pub u128);

impl fmt::Display for ArgonFormatter {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.0;
		format_argons(value).fmt(f)
	}
}

pub struct Pct(pub f32);

impl fmt::Display for Pct {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.0;
		let mut value_str = &*format!("{}", value);

		if value_str.contains('.') {
			value_str = value_str.trim_end_matches('0').trim_end_matches('.');
		}

		write!(f, "{}%", value_str)
	}
}

pub struct Pct64(pub f64);

impl fmt::Display for Pct64 {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.0;
		let mut value_str = &*format!("{}", value);

		if value_str.contains('.') {
			value_str = value_str.trim_end_matches('0').trim_end_matches('.');
		}

		write!(f, "{}%", value_str)
	}
}

fn insert_commas(n: u128) -> String {
	let whole_part = n.to_string();
	let chars: Vec<char> = whole_part.chars().rev().collect();
	let mut result = String::new();

	for (i, c) in chars.iter().enumerate() {
		if i > 0 && i % 3 == 0 {
			result.push(',');
		}
		result.push(*c);
	}

	result.chars().rev().collect()
}

#[allow(dead_code)]
struct BTCFormatter(u64, u8);

impl fmt::Display for BTCFormatter {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.0;
		let whole_part = value / SATOSHIS_PER_BITCOIN; // Extract the whole part
		let decimal_part = value % SATOSHIS_PER_BITCOIN; // Extract the decimal part

		// Scale the decimal part according to the requested number of decimals
		let scaled_decimal_part = decimal_part / 10u64.pow(8 - self.1 as u32);

		write!(f, "₿ {}.{:0width$}", whole_part, scaled_decimal_part, width = self.1 as usize)
	}
}
