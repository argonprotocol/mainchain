use std::{fmt, io, io::Write, iter::Iterator, string::ToString};

use crate::{
	formatters::{Argons, Pct, parse_number},
	helpers::{read_bitcoin_xpub, read_percent_to_fixed_128},
};
use argon_bitcoin_cli_macros::ReadDocs;
use argon_client::{
	api,
	api::runtime_types,
	conversion::{to_api_fixed_u128, to_api_per_mill},
};
use argon_primitives::bitcoin::BitcoinXPub;
use clap::Args;
use inquire::{
	CustomUserError, InquireError, Select, Text,
	error::InquireResult,
	validator::{StringValidator, Validation},
};
use polkadot_sdk::*;
use sp_runtime::{FixedU128, Permill};

#[derive(Debug, Args, ReadDocs)]
pub struct VaultConfig {
	/// Argons to allocate to the vault for bitcoin locks and securitization
	#[clap(long, value_parser=parse_number)]
	argons: Option<f32>,

	/// The securitization ratio for bitcoin argons in your vault
	#[clap(long, value_parser=parse_number)]
	securitization_ratio: Option<f32>,

	/// A serialized xpub string to be uploaded to the vault. Child pubkeys will have a single
	/// incrementing index used for each bitcoin lock.
	#[clap(long)]
	bitcoin_xpub: Option<String>,

	/// The base fee in argons. Up to 6 decimal points
	#[clap(long, value_parser=parse_number)]
	bitcoin_base_fee: Option<f32>,

	/// The bitcoin locks annual percent return. A bitcoin lock is 1 year, so returns are the
	/// amount of argons borrowed times this rate.
	#[clap(long, value_parser=parse_number)]
	bitcoin_apr: Option<f32>,

	/// The vault sharing percent for liquidity pool profits
	#[clap(long, value_parser=parse_number)]
	liquidity_pool_profit_sharing: Option<f32>,
}

const FIELD_TO_LABEL: [(&str, &str); 6] = [
	("argons", "Vault Argons"),
	("securitization_ratio", "Securitization Ratio"),
	("bitcoin_xpub", "Bitcoin XPub"),
	("bitcoin_base_fee", "Bitcoin Base Fee Argons"),
	("bitcoin_apr", "Bitcoin APR"),
	("liquidity_pool_profit_sharing", "Liquidity Pool Profit Sharing %"),
];

fn label(field: &str) -> &str {
	FIELD_TO_LABEL.into_iter().find(|(f, _)| *f == field).unwrap().1
}

impl VaultConfig {
	pub async fn complete_prompt(&mut self, has_keypair: bool) -> bool {
		self.sanitize_bad_values();

		if self.next_incomplete_field().is_none() {
			return true;
		}

		loop {
			match self.update_state(has_keypair) {
				Ok(false) => continue,
				Ok(true) => return true,
				Err(_) => return false,
			}
		}
	}

	pub fn as_call_data(&self) -> api::vaults::calls::types::create::VaultConfig {
		let opaque_xpub = read_bitcoin_xpub(&self.bitcoin_xpub.clone().unwrap_or_default())
			.expect("Invalid xpub");

		api::vaults::calls::types::create::VaultConfig {
			bitcoin_xpubkey: opaque_xpub.0.into(),
			terms: runtime_types::argon_primitives::vault::VaultTerms::<u128> {
				bitcoin_base_fee: (self.bitcoin_base_fee.unwrap_or(0.0) * 1_000_000.0) as u128,
				bitcoin_annual_percent_rate: to_api_fixed_u128(read_percent_to_fixed_128(
					self.bitcoin_apr.unwrap_or(0.0),
				)),
				liquidity_pool_profit_sharing: to_api_per_mill(Permill::from_float(
					(self.liquidity_pool_profit_sharing.unwrap_or(0.0) / 100.0) as f64,
				)),
			},
			securitization_ratio: to_api_fixed_u128(FixedU128::from_float(
				self.securitization_ratio.unwrap_or(1.0) as f64,
			)),
			securitization: (self.argons.unwrap_or(0.0) * 1_000_000.0) as u128,
		}
	}

	pub fn argons_needed(&self) -> String {
		Argons(self.argons.unwrap_or_default()).to_string()
	}

	fn update_state(&mut self, has_keypair: bool) -> Result<bool, String> {
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

			if has_keypair {
				fields.push("Submit".to_string());
			} else {
				fields.push("Generate".to_string());
			}

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
				"argons" => self.prompt_argons()?,
				"bitcoin_xpub" => self.prompt_bitcoin_xpub()?,
				"bitcoin_base_fee" => self.prompt_bitcoin_base_fee()?,
				"bitcoin_apr" => self.prompt_bitcoin_apr()?,
				"liquidity_pool_profit_sharing" => self.prompt_liquidity_pool_profit_sharing()?,
				"securitization_ratio" => self.prompt_securitization()?,
				_ => unreachable!(),
			}

			Ok(false)
		} else {
			Ok(true)
		}
	}

	fn prompt_bitcoin_xpub(&mut self) -> Result<(), String> {
		let result = self
			.text_field("bitcoin_xpub", "")
			.with_placeholder("xpub...")
			.with_validator(|input: &str| {
				let data = match read_bitcoin_xpub(input) {
					Ok(x) => x,
					Err(e) => return Ok(Validation::Invalid(e.into())),
				};

				match TryInto::<BitcoinXPub>::try_into(data) {
					Ok(x) => {
						if !x.is_hardened() {
							return Ok(Validation::Invalid("xpub must be hardened".into()));
						}
						Ok(Validation::Valid)
					},
					Err(e) => Ok(Validation::Invalid(format!("Invalid xpub: {:?}", e).into())),
				}
			})
			.prompt();
		if let Err(err) = result {
			if matches!(err, InquireError::OperationCanceled) {
				self.bitcoin_xpub = None;
				self.prompt_bitcoin_xpub()?;
				return Ok(());
			}
			return Err(err.to_string());
		}
		self.bitcoin_xpub = result.ok();
		Ok(())
	}

	fn prompt_bitcoin_base_fee(&mut self) -> Result<(), String> {
		self.bitcoin_base_fee =
			Some(self.text_field("bitcoin_base_fee", "0.0").with_min_f32(0.0).prompt_with_f32()?);
		Ok(())
	}

	fn prompt_argons(&mut self) -> Result<(), String> {
		self.argons = Some(self.text_field("argons", "0.00").with_min_f32(0.0).prompt_with_f32()?);
		Ok(())
	}

	fn prompt_liquidity_pool_profit_sharing(&mut self) -> Result<(), String> {
		self.liquidity_pool_profit_sharing = Some(
			self.text_field("liquidity_pool_profit_sharing", "0.00")
				.with_min_f32(0.0)
				.prompt_with_f32()?,
		);
		Ok(())
	}

	fn prompt_securitization(&mut self) -> Result<(), String> {
		self.securitization_ratio = Some(
			self.text_field("securitization_ratio", "1.0")
				.with_min_f32(1.0)
				.prompt_with_f32()?,
		);
		Ok(())
	}

	fn prompt_bitcoin_apr(&mut self) -> Result<(), String> {
		self.bitcoin_apr =
			Some(self.text_field("bitcoin_apr", "0.0").with_min_f32(0.0).prompt_with_f32()?);
		Ok(())
	}

	fn sanitize_bad_values(&mut self) {
		if let Some(val) = self.argons {
			if val < 0.0 {
				self.argons = None;
			}
		}
		if let Some(val) = self.liquidity_pool_profit_sharing {
			if val < 0.0 {
				self.liquidity_pool_profit_sharing = None;
			}
		}
		if let Some(val) = self.securitization_ratio {
			if val < 1.0 {
				self.securitization_ratio = None;
			}
		}

		if let Some(val) = self.bitcoin_apr {
			if val < 0.0 {
				self.bitcoin_apr = None;
			}
		}

		if let Some(val) = self.bitcoin_base_fee {
			if val < 0.0 {
				self.bitcoin_base_fee = None;
			}
		}
	}

	fn print_field(&self, field: &str) {
		println!("{}: {}", label(field), self.formatted_value(field).unwrap_or("-".to_string()));
	}

	fn next_incomplete_field(&self) -> Option<&'static str> {
		FIELD_TO_LABEL
			.iter()
			.map(|(f, _)| *f)
			.find(|&field| self.formatted_value(field).is_none())
	}

	fn formatted_value(&self, field: &str) -> Option<String> {
		match field {
			"argons" => self.format_type(field, &self.argons),
			"bitcoin_xpub" => self.bitcoin_xpub.clone(),
			"bitcoin_base_fee" => self.format_type(field, &self.bitcoin_base_fee),
			"bitcoin_apr" => self.format_type(field, &self.bitcoin_apr),
			"liquidity_pool_profit_sharing" =>
				self.format_type(field, &self.liquidity_pool_profit_sharing),
			"securitization_ratio" => self.format_type(field, &self.securitization_ratio),
			_ => None,
		}
	}

	fn format_type(&self, field: &str, value: &Option<impl fmt::Display>) -> Option<String> {
		let Some(value) = value else { return None };
		let value = value.to_string();
		if value.is_empty() {
			return None;
		}

		match field {
			"argons" | "bitcoin_base_fee" => {
				let argons = parse_number(&value).unwrap();
				Some(Argons(argons).to_string())
			},
			"bitcoin_apr" | "liquidity_pool_profit_sharing" => {
				let pct = parse_number(&value).unwrap();
				Some(Pct(pct).to_string())
			},
			"securitization_ratio" => {
				let ratio = parse_number(&value).unwrap();
				Some(format!("{:.2}x", ratio))
			},
			_ => None,
		}
	}

	fn text_field(&self, field: &'static str, default: &'static str) -> TextField {
		let text = label(field);
		let docs = VaultConfig::get_docs(field).unwrap();

		let formatted_default = self.format_type(field, &Some(default)).unwrap_or_default();
		let existing_value = self.formatted_value(field);
		TextField::new(text, docs, formatted_default, existing_value)
	}
}

struct TextField<'a> {
	existing_value: Option<String>,
	default: String,
	text: Text<'a>,
}

impl<'a> TextField<'a> {
	fn new(label: &'a str, docs: &'a str, default: String, existing_value: Option<String>) -> Self {
		let text_field = Text::new(label).with_help_message(docs);

		Self { existing_value: existing_value.clone(), default, text: text_field }
	}

	fn with_min_f32(mut self, min_value: f32) -> Self {
		self.text = self.text.with_validator(F32Validator { min_value });
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
struct F32Validator {
	min_value: f32,
}
impl StringValidator for F32Validator {
	fn validate(&self, input: &str) -> Result<Validation, CustomUserError> {
		match parse_number(input) {
			Ok(x) =>
				if x >= self.min_value {
					Ok(Validation::Valid)
				} else {
					Ok(Validation::Invalid(
						format!("Must be greater than {}", self.min_value).into(),
					))
				},
			Err(_) => Ok(Validation::Invalid("Invalid number".into())),
		}
	}
}
