use std::{fmt, io, io::Write, iter::Iterator, string::ToString};

use clap::Args;
use inquire::{
	error::InquireResult,
	validator::{StringValidator, Validation},
	CustomUserError, InquireError, Select, Text,
};

use crate::{
	formatters::{parse_number, Argons, Pct},
	helpers::{read_bitcoin_xpub, read_percent_to_fixed_128},
};
use argon_bitcoin_cli_macros::ReadDocs;
use argon_client::{api, api::runtime_types, conversion::to_api_fixed_u128};
use argon_primitives::bitcoin::BitcoinXPub;

#[derive(Debug, Args, ReadDocs)]
pub struct VaultConfig {
	/// Argons to move to the vault to be available for bitcoin bonds.
	#[clap(long, value_parser=parse_number)]
	bitcoin_argons: Option<f32>,
	/// A serialized xpub string to be uploaded to the vault. Child pubkeys will have a single
	/// incrementing index used for each bond.
	#[clap(long)]
	bitcoin_xpub: Option<String>,

	/// The base fee in argons. Up to 6 decimal points
	#[clap(long, value_parser=parse_number)]
	bitcoin_base_fee: Option<f32>,

	/// The bitcoin bonds annual percent return. A bitcoin bond is 1 year, so returns are the
	/// amount of argons borrowed times this rate.
	#[clap(long, value_parser=parse_number)]
	bitcoin_apr: Option<f32>,
	/// Number of argons to move into the vault for mining. NOTE: mining can only be done at a 1-1
	/// ratio with the amount of bonded bitcoin argons (or securitization up to 2x bitcoin bonds).
	#[clap(long, value_parser=parse_number)]
	bonded_argons: Option<f32>,

	/// The base fee in argons. Up to 6 decimal points
	#[clap(long, value_parser=parse_number)]
	bonded_argons_base_fee: Option<f32>,
	/// The bonded argons annual percent return. NOTE: this will be adjusted down to the mining
	/// slot duration
	#[clap(long, value_parser=parse_number)]
	bonded_argons_apr: Option<f32>,
	/// An optional profit sharing setup where any argons mined or minted (not including fees) are
	/// split between miner and this vault.
	#[clap(long, value_parser=parse_number)]
	mining_reward_sharing_percent_take: Option<f32>,
	/// A percentage of additional argons to add to a securitization pool. These argons are a
	/// guarantee for bitcoin lockers in the case of loss or fraud if the price is above the
	/// original lock price. They may be up to 2x the amount of bitcoin argons.
	#[clap(long, value_parser=parse_number)]
	added_securitization_percent: Option<f32>,
}

const FIELD_TO_LABEL: [(&str, &str); 9] = [
	("bitcoin_argons", "Bitcoin Bond Argons"),
	("bitcoin_xpub", "Bitcoin XPub"),
	("bitcoin_base_fee", "Bitcoin Base Fee Argons"),
	("bitcoin_apr", "Bitcoin APR"),
	("bonded_argons", "Bonded Argons"),
	("bonded_argons_apr", "Bonded Argons APR"),
	("bonded_argons_base_fee", "Bonded Argons Base Fee"),
	("mining_reward_sharing_percent_take", "Mining Reward Sharing %"),
	("added_securitization_percent", "Added Securitization %"),
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
				bonded_argons_base_fee: (self.bonded_argons_base_fee.unwrap_or(0.0) * 1_000_000.0)
					as u128,
				bonded_argons_annual_percent_rate: to_api_fixed_u128(read_percent_to_fixed_128(
					self.bonded_argons_apr.unwrap_or(0.0),
				)),
				mining_reward_sharing_percent_take: to_api_fixed_u128(read_percent_to_fixed_128(
					self.mining_reward_sharing_percent_take.unwrap_or(0.0),
				)),
			},
			added_securitization_percent: to_api_fixed_u128(read_percent_to_fixed_128(
				self.added_securitization_percent.unwrap_or(0.0),
			)),
			bitcoin_amount_allocated: (self.bitcoin_argons.unwrap_or(0.0) * 1_000_000.0) as u128,
			bonded_argons_allocated: (self.bonded_argons.unwrap_or(0.0) * 1_000_000.0) as u128,
		}
	}

	pub fn argons_needed(&self) -> String {
		let mut argons_needed = self.bitcoin_argons.unwrap_or(0.0);
		argons_needed += (self.added_securitization_percent.unwrap_or(0.0) / 100.0) * argons_needed;
		argons_needed += self.bonded_argons.unwrap_or(0.0);
		Argons(argons_needed).to_string()
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
				"bitcoin_argons" => self.prompt_bitcoin_argons()?,
				"bitcoin_xpub" => self.prompt_bitcoin_xpub()?,
				"bitcoin_base_fee" => self.prompt_bitcoin_base_fee()?,
				"bitcoin_apr" => self.prompt_bitcoin_apr()?,
				"bonded_argons_apr" => self.prompt_bonded_argons_apr()?,
				"bonded_argons_base_fee" => self.prompt_bonded_argons_base_fee()?,
				"mining_reward_sharing_percent_take" =>
					self.prompt_mining_reward_sharing_percent_take()?,
				"bonded_argons" => self.prompt_bonded_argons()?,
				"added_securitization_percent" => self.prompt_securitization()?,
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

	fn prompt_mining_reward_sharing_percent_take(&mut self) -> Result<(), String> {
		self.mining_reward_sharing_percent_take = Some(
			self.text_field("mining_reward_sharing_percent_take", "0.0")
				.with_validator(|input: &str| {
					if let Ok(x) = parse_number(input) {
						if (0.0..=100.0).contains(&x) {
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

	fn prompt_bonded_argons_base_fee(&mut self) -> Result<(), String> {
		self.bonded_argons_base_fee = Some(
			self.text_field("bonded_argons_base_fee", "0.00")
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
			self.text_field("bitcoin_argons", "0.00")
				.with_positive_f32()
				.prompt_with_f32()?,
		);
		Ok(())
	}

	fn prompt_bonded_argons(&mut self) -> Result<(), String> {
		self.bonded_argons =
			Some(self.text_field("bonded_argons", "0.00").with_positive_f32().prompt_with_f32()?);
		Ok(())
	}

	fn prompt_securitization(&mut self) -> Result<(), String> {
		self.added_securitization_percent = Some(
			self.text_field("added_securitization_percent", "100.0")
				.with_positive_f32()
				.prompt_with_f32()?,
		);
		Ok(())
	}

	fn prompt_bonded_argons_apr(&mut self) -> Result<(), String> {
		self.bonded_argons_apr = Some(
			self.text_field("bonded_argons_apr", "0.0")
				.with_positive_f32()
				.prompt_with_f32()?,
		);
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
		if let Some(val) = self.bonded_argons {
			if val < 0.0 {
				self.bonded_argons = None;
			}
		}
		if let Some(val) = self.added_securitization_percent {
			if val < 0.0 {
				self.added_securitization_percent = None;
			}
		}

		if let Some(val) = self.bitcoin_apr {
			if val < 0.0 {
				self.bitcoin_apr = None;
			}
		}
		if let Some(val) = self.bonded_argons_apr {
			if val < 0.0 {
				self.bonded_argons_apr = None;
			}
		}

		if let Some(val) = self.bonded_argons_base_fee {
			if val < 0.0 {
				self.bonded_argons_base_fee = None;
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
		FIELD_TO_LABEL
			.iter()
			.map(|(f, _)| *f)
			.find(|&field| self.formatted_value(field).is_none())
	}

	fn formatted_value(&self, field: &str) -> Option<String> {
		match field {
			"bitcoin_argons" => self.format_type(field, &self.bitcoin_argons),
			"bitcoin_xpub" => self.bitcoin_xpub.clone(),
			"bitcoin_base_fee" => self.format_type(field, &self.bitcoin_base_fee),
			"bitcoin_apr" => self.format_type(field, &self.bitcoin_apr),
			"bonded_argons_apr" => self.format_type(field, &self.bonded_argons_apr),
			"bonded_argons_base_fee" => self.format_type(field, &self.bonded_argons_base_fee),
			"mining_reward_sharing_percent_take" =>
				self.format_type(field, &self.mining_reward_sharing_percent_take),
			"bonded_argons" => self.format_type(field, &self.bonded_argons),
			"added_securitization_percent" =>
				self.format_type(field, &self.added_securitization_percent),
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
			"bitcoin_argons" | "bonded_argons" | "bitcoin_base_fee" | "bonded_argons_base_fee" => {
				let argons = parse_number(&value).unwrap();
				Some(Argons(argons).to_string())
			},
			"bitcoin_apr" |
			"bonded_argons_apr" |
			"mining_reward_sharing_percent_take" |
			"added_securitization_percent" => {
				let pct = parse_number(&value).unwrap();
				Some(Pct(pct).to_string())
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
				if x >= 0.0 || !self.0 {
					Ok(Validation::Valid)
				} else {
					Ok(Validation::Invalid("Must not be negative".into()))
				},
			Err(_) => Ok(Validation::Invalid("Invalid number".into())),
		}
	}
}
