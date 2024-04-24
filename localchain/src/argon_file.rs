use crate::Result;
use clap::crate_version;
use serde::{Deserialize, Serialize};
use ulx_primitives::{BalanceChange, Notarization};
/// The version of the Argon file format.
#[cfg_attr(feature = "napi", napi)]
pub const ARGON_FILE_VERSION: &str = crate_version!();

#[derive(Debug, Serialize, Deserialize)]
pub struct ArgonFile {
  pub version: String,
  pub send: Option<Vec<BalanceChange>>,
  pub request: Option<Vec<BalanceChange>>,
}

impl ArgonFile {
  pub fn to_json(&self) -> Result<String> {
    Ok(serde_json::to_string(self)?)
  }

  pub fn from_json(json: &str) -> Result<Self> {
    Ok(serde_json::from_str(json)?)
  }

  pub fn from_notarization(notarization: &Notarization, file_type: ArgonFileType) -> Self {
    Self::create(notarization.balance_changes.to_vec(), file_type)
  }

  pub fn create(balance_changes: Vec<BalanceChange>, file_type: ArgonFileType) -> Self {
    match file_type {
      ArgonFileType::Send => Self {
        version: ARGON_FILE_VERSION.to_string(),
        send: Some(balance_changes),
        request: None,
      },
      ArgonFileType::Request => Self {
        version: ARGON_FILE_VERSION.to_string(),
        send: None,
        request: Some(balance_changes),
      },
    }
  }
}

#[cfg_attr(feature = "napi", napi)]
pub enum ArgonFileType {
  Send,
  Request,
}
