use alloc::{
	format,
	string::{String, ToString},
	vec::Vec,
};

pub fn format_argons(argons: u128) -> String {
	let value = argons;
	let whole_part = value / 1_000_000; // Extract the whole part
	let decimal_part = (value % 1_000_000) / 10_000; // Extract the decimal part, considering only 2 decimal places
	let whole_part_str = insert_commas(whole_part);

	if decimal_part == 0 {
		return format!("₳{}", whole_part_str);
	}
	format!("₳{}.{:02}", whole_part_str, decimal_part)
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
