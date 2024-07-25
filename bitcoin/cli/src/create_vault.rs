use std::{fmt, io, io::Write, iter::Iterator, str::FromStr, string::ToString};

use bitcoin::bip32::Xpub;
use clap::Args;
use inquire::{
	error::InquireResult,
	validator::{StringValidator, Validation},
	CustomUserError, Select, Text,
};
use sp_runtime::FixedU128;
use ulixee_client::{api, api::runtime_types, conversion::to_api_fixed_u128};

use ulx_bitcoin_cli_macros::ReadDocs;

#[derive(Debug, Args, ReadDocs)]
pub struct VaultConfig {
	/// Argons to move to the vault to be available for bitcoin bonds.
	#[clap(long, value_parser=parse_number)]
	bitcoin_argons: Option<f32>,
	/// A serialized xpub string to be uploaded to the vault. Child pubkeys will have a single
	/// incrementing index used for each bond.
	#[clap(long)]
	bitcoin_xpub: Option<String>,

	/// The base fee in argons. Up to 3 decimal points
	#[clap(long, value_parser=parse_number)]
	bitcoin_base_fee: Option<f32>,

	/// The bitcoin bonds annual percent return. A bitcoin bond is 1 year, so returns are the
	/// amount of argons borrowed times this rate.
	#[clap(long, value_parser=parse_number)]
	bitcoin_apr: Option<f32>,

	/// The mining bond annual percent return. NOTE: this will be adjusted down to the mining slot
	/// duration
	#[clap(long, value_parser=parse_number)]
	mining_apr: Option<f32>,
	/// The base fee in argons. Up to 3 decimal points
	#[clap(long, value_parser=parse_number)]
	mining_base_fee: Option<f32>,
	/// An optional profit sharing setup where any argons mined or minted (not including fees) are
	/// split between miner and this vault.
	#[clap(long, value_parser=parse_number)]
	mining_reward_sharing_percent_take: Option<f32>,
	/// Number of argons to move into the vault for mining. NOTE: mining can only be done at a 1-1
	/// ratio with the amount of bonded bitcoin argons (or securitization up to 2x bitcoin bonds).
	#[clap(long, value_parser=parse_number)]
	mining_argons: Option<f32>,
	/// A percentage of additional argons to add to a securitization pool. These argons are a
	/// guarantee for bitcoin bonders in the case of loss or fraud. They maybe be up to 2x the
	/// amount of bitcoin argons.
	#[clap(long, value_parser=parse_number)]
	securitization_percent: Option<f32>,
}

const FIELD_TO_LABEL: [(&str, &str); 9] = [
	("bitcoin_argons", "Bitcoin Bond Argons"),
	("bitcoin_xpub", "Bitcoin XPub"),
	("bitcoin_base_fee", "Bitcoin Base Fee Argons"),
	("bitcoin_apr", "Bitcoin APR"),
	("mining_apr", "Mining APR"),
	("mining_base_fee", "Mining Base Fee Argons"),
	("mining_reward_sharing_percent_take", "Mining Reward Sharing %"),
	("mining_argons", "Mining Bond Argons"),
	("securitization_percent", "Securitization %"),
];

fn label(field: &str) -> &str {
	FIELD_TO_LABEL.into_iter().find(|(f, _)| *f == field).unwrap().1
}

impl VaultConfig {
	pub async fn complete_prompt(&mut self) -> bool {
		self.sanitize_bad_values();

		if self.next_incomplete_field().is_none() {
			return true;
		}

		loop {
			match self.update_state() {
				Ok(false) => continue,
				Ok(true) => return true,
				Err(_) => return false,
			}
		}
	}

	pub fn as_call_data(&self) -> api::vaults::calls::types::create::VaultConfig {
		let xpub = Xpub::from_str(&self.bitcoin_xpub.clone().unwrap()).expect("Invalid xpub");

		api::vaults::calls::types::create::VaultConfig {
			bitcoin_xpubkey: runtime_types::ulx_primitives::bitcoin::OpaqueBitcoinXpub(
				xpub.encode(),
			),
			terms: runtime_types::ulx_primitives::bond::VaultTerms::<u128> {
				bitcoin_base_fee: (self.bitcoin_base_fee.unwrap_or(0.0) * 1000.0) as u128,
				bitcoin_annual_percent_rate: to_api_fixed_u128(FixedU128::from_rational(
					(self.bitcoin_apr.unwrap_or(0.0) as f64 * 1000.0) as u128,
					100 * 1000,
				)),
				mining_base_fee: (self.mining_base_fee.unwrap_or(0.0) * 1000.0) as u128,
				mining_annual_percent_rate: to_api_fixed_u128(FixedU128::from_rational(
					(self.mining_apr.unwrap_or(0.0) as f64 * 1000.0) as u128,
					100 * 1000,
				)),
				mining_reward_sharing_percent_take: to_api_fixed_u128(FixedU128::from_rational(
					(self.mining_reward_sharing_percent_take.unwrap_or(0.0) as f64 * 1000.0)
						as u128,
					100 * 1000,
				)),
			},
			securitization_percent: to_api_fixed_u128(FixedU128::from_float(
				self.securitization_percent.unwrap_or(0.0) as f64,
			)),
			bitcoin_amount_allocated: (self.bitcoin_argons.unwrap_or(0.0) * 1000.0) as u128,
			mining_amount_allocated: (self.mining_argons.unwrap_or(0.0) * 1000.0) as u128,
		}
	}

	fn update_state(&mut self) -> Result<bool, String> {
		print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
		io::stdout().flush().unwrap();

		let mut next_field = self.next_incomplete_field();

		if next_field.is_none() {
			let mut fields = FIELD_TO_LABEL
				.iter()
				.map(|(f, _)| {
					format!("{}: {}", label(f), self.formatted_value(f).unwrap_or("-".to_string()))
				})
				.collect::<Vec<_>>();

			fields.push("Submit".to_string());

			let choice = Select::new("Review your Configuration:", fields.clone())
				.with_starting_cursor(fields.len() - 1)
				.with_page_size(fields.len())
				.prompt()
				.unwrap();

			let idx = fields.iter().position(|f| f == &choice).unwrap();
			if idx == fields.len() - 1 {
				return Ok(true);
			}
			next_field = Some(FIELD_TO_LABEL[idx].0);
		}

		if let Some(field) = next_field {
			println!("Your Vault\n");

			for field in FIELD_TO_LABEL.iter().map(|(f, _)| *f) {
				self.print_field(field);
			}

			println!("\n\n");
			match field {
				"bitcoin_argons" => self.prompt_bitcoin_argons()?,
				"bitcoin_xpub" => self.prompt_bitcoin_xpub()?,
				"bitcoin_base_fee" => self.prompt_bitcoin_base_fee()?,
				"bitcoin_apr" => self.prompt_bitcoin_apr()?,
				"mining_apr" => self.prompt_mining_apr()?,
				"mining_base_fee" => self.prompt_mining_base_fee()?,
				"mining_reward_sharing_percent_take" =>
					self.prompt_mining_reward_sharing_percent_take()?,
				"mining_argons" => self.prompt_mining_argons()?,
				"securitization_percent" => self.prompt_securitization()?,
				_ => unreachable!(),
			}

			Ok(false)
		} else {
			Ok(true)
		}
	}

	fn prompt_bitcoin_xpub(&mut self) -> Result<(), String> {
		self.bitcoin_xpub = Some(
			self.text_field("bitcoin_xpub", "")
				.with_placeholder("xpub...")
				.with_validator(|input: &str| {
					if input.len() != 111 {
						return Ok(Validation::Invalid("xpub must be 111 characters long".into()))
					}
					if !input.starts_with("xpub") {
						return Ok(Validation::Invalid("xpub must start with 'xpub'".into()))
					}
					match Xpub::from_str(input) {
						Ok(_) => Ok(Validation::Valid),
						Err(e) => Ok(Validation::Invalid(format!("Invalid xpub: {}", e).into())),
					}
				})
				.prompt()
				.map_err(|e| e.to_string())?,
		);
		Ok(())
	}

	fn prompt_mining_reward_sharing_percent_take(&mut self) -> Result<(), String> {
		self.mining_reward_sharing_percent_take = Some(
			self.text_field("mining_reward_sharing_percent_take", "0.0")
				.with_validator(|input: &str| {
					if let Ok(x) = parse_number(input) {
						if x <= 100.0 && x >= 0.0 {
							Ok(Validation::Valid)
						} else {
							Ok(Validation::Invalid("Must be 0-100".into()))
						}
					} else {
						Ok(Validation::Invalid("Invalid number".into()))
					}
				})
				.prompt_with_f32()?,
		);
		Ok(())
	}

	fn prompt_mining_base_fee(&mut self) -> Result<(), String> {
		self.mining_base_fee = Some(
			self.text_field("mining_base_fee", "0.00")
				.with_positive_f32()
				.prompt_with_f32()?,
		);
		Ok(())
	}

	fn prompt_bitcoin_base_fee(&mut self) -> Result<(), String> {
		self.bitcoin_base_fee = Some(
			self.text_field("bitcoin_base_fee", "0.0")
				.with_positive_f32()
				.prompt_with_f32()?,
		);
		Ok(())
	}

	fn prompt_bitcoin_argons(&mut self) -> Result<(), String> {
		self.bitcoin_argons = Some(
			self.text_field("bitcoin_argons", "100,000")
				.with_positive_f32()
				.prompt_with_f32()?,
		);
		Ok(())
	}

	fn prompt_mining_argons(&mut self) -> Result<(), String> {
		self.mining_argons = Some(
			self.text_field("mining_argons", "100,000")
				.with_positive_f32()
				.prompt_with_f32()?,
		);
		Ok(())
	}

	fn prompt_securitization(&mut self) -> Result<(), String> {
		self.securitization_percent = Some(
			self.text_field("securitization_percent", "100.0")
				.with_positive_f32()
				.prompt_with_f32()?,
		);
		Ok(())
	}

	fn prompt_mining_apr(&mut self) -> Result<(), String> {
		self.mining_apr =
			Some(self.text_field("mining_apr", "1.0").with_positive_f32().prompt_with_f32()?);
		Ok(())
	}

	fn prompt_bitcoin_apr(&mut self) -> Result<(), String> {
		self.bitcoin_apr =
			Some(self.text_field("bitcoin_apr", "0.0").with_positive_f32().prompt_with_f32()?);
		Ok(())
	}

	fn sanitize_bad_values(&mut self) {
		if let Some(val) = self.bitcoin_argons {
			if val < 0.0 {
				self.bitcoin_argons = None;
			}
		}
		if let Some(val) = self.mining_argons {
			if val < 0.0 {
				self.mining_argons = None;
			}
		}
		if let Some(val) = self.securitization_percent {
			if val < 0.0 {
				self.securitization_percent = None;
			}
		}

		if let Some(val) = self.bitcoin_apr {
			if val < 0.0 {
				self.bitcoin_apr = None;
			}
		}
		if let Some(val) = self.mining_apr {
			if val < 0.0 {
				self.mining_apr = None;
			}
		}

		if let Some(val) = self.mining_base_fee {
			if val < 0.0 {
				self.mining_base_fee = None;
			}
		}
		if let Some(val) = self.bitcoin_base_fee {
			if val < 0.0 {
				self.bitcoin_base_fee = None;
			}
		}
		if let Some(val) = self.mining_reward_sharing_percent_take {
			if val < 0.0 {
				self.mining_reward_sharing_percent_take = None;
			}
			if val > 100.0 {
				self.mining_reward_sharing_percent_take = None;
			}
		}
	}

	fn print_field(&self, field: &str) {
		println!("{}: {}", label(field), self.formatted_value(field).unwrap_or("-".to_string()));
	}

	fn next_incomplete_field(&self) -> Option<&'static str> {
		for field in FIELD_TO_LABEL.iter().map(|(f, _)| *f) {
			if self.formatted_value(field).is_none() {
				return Some(field);
			}
		}
		None
	}

	fn formatted_value(&self, field: &str) -> Option<String> {
		match field {
			"bitcoin_argons" => self.bitcoin_argons.map(|a| Argons(a).to_string()),
			"bitcoin_xpub" => self.bitcoin_xpub.clone(),
			"bitcoin_base_fee" => self.bitcoin_base_fee.map(|a| Argons(a).to_string()),
			"bitcoin_apr" => self.bitcoin_apr.map(|a| Pct(a).to_string()),
			"mining_apr" => self.mining_apr.map(|a| Pct(a).to_string()),
			"mining_base_fee" => self.mining_base_fee.map(|a| Argons(a).to_string()),
			"mining_reward_sharing_percent_take" =>
				self.mining_reward_sharing_percent_take.map(|a| Pct(a).to_string()),
			"mining_argons" => self.mining_argons.map(|a| Argons(a).to_string()),
			"securitization_percent" => self.securitization_percent.map(|a| Pct(a).to_string()),
			_ => None,
		}
	}

	fn text_field(&self, field: &'static str, default: &'static str) -> TextField {
		let text = label(field);
		let docs = VaultConfig::get_docs(field).unwrap();
		let existing_value = self.formatted_value(field);
		TextField::new(text, docs, default, existing_value)
	}
}

struct TextField<'a> {
	existing_value: Option<String>,
	default: String,
	text: Text<'a>,
}

impl<'a> TextField<'a> {
	fn new(
		label: &'a str,
		docs: &'a str,
		default: &'a str,
		existing_value: Option<String>,
	) -> Self {
		let text_field = Text::new(label).with_help_message(docs);

		Self {
			existing_value: existing_value.clone(),
			default: default.to_string(),
			text: text_field,
		}
	}

	fn with_positive_f32(mut self) -> Self {
		self.text = self.text.with_validator(F32Validator(true));
		self
	}

	fn with_validator<V: StringValidator + 'static>(mut self, validator: V) -> Self {
		self.text = self.text.with_validator(validator);
		self
	}

	fn with_placeholder(mut self, placeholder: &'a str) -> Self {
		self.text = self.text.with_placeholder(placeholder);
		self
	}

	fn prompt(self) -> InquireResult<String> {
		let text_field = self.text;
		if let Some(existing) = self.existing_value {
			text_field.with_initial_value(&existing).prompt()
		} else if !self.default.is_empty() {
			text_field.with_default(&self.default).prompt()
		} else {
			text_field.prompt()
		}
	}

	fn prompt_with_f32(self) -> Result<f32, String> {
		let text = self.prompt().map_err(|e| e.to_string())?;
		parse_number(&text)
	}
}

/// Validates that a string can be converted to an f32 and is non-negative (if the .0 is true)
#[derive(Clone)]
struct F32Validator(bool);
impl StringValidator for F32Validator {
	fn validate(&self, input: &str) -> Result<Validation, CustomUserError> {
		match parse_number(input) {
			Ok(x) =>
				if self.0 == false {
					Ok(Validation::Valid)
				} else if x >= 0.0 {
					Ok(Validation::Valid)
				} else {
					Ok(Validation::Invalid("Must not be negative".into()))
				},
			Err(_) => Ok(Validation::Invalid("Invalid number".into())),
		}
	}
}
fn parse_number(s: &str) -> Result<f32, String> {
	// Remove commas from the string
	let cleaned: String = s.chars().filter(|&c| c.is_digit(10) || c == '.').collect();

	// Ensure there's a decimal point for integer numbers by appending ".0" if needed
	let cleaned = if !cleaned.contains('.') { format!("{}.0", cleaned) } else { cleaned };

	// Parse the cleaned string as an f32
	let number: f32 = cleaned.parse().map_err(|_| "Invalid number".to_string())?;
	Ok(number)
}

struct Argons(f32);

impl fmt::Display for Argons {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.0;
		let whole_part = value.floor(); // Extract the whole part
		let whole_part_str = insert_commas(whole_part as u128);

		let decimal_part = (value.fract() * 100.0).round(); // Extract and round the decimal part

		if decimal_part == 0.0 {
			write!(f, "₳ {}", whole_part_str)
		} else {
			write!(f, "₳ {}.{:02}", whole_part_str, decimal_part as u32)
		}
	}
}

struct Pct(f32);

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
