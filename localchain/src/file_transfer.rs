use clap::crate_version;
use serde::{Deserialize, Serialize};
use ulx_primitives::{BalanceChange, Notarization};
use crate::to_js_error;

/// The version of the Argon file format.
#[napi]
pub const VERSION: &str = crate_version!();

#[derive(Debug, Serialize, Deserialize)]
pub struct ArgonFile {
  pub version: String,
  pub send: Option<Vec<BalanceChange>>,
  pub request: Option<Vec<BalanceChange>>,
}

impl ArgonFile {
  pub fn to_json(&self) -> napi::Result<String> {
    serde_json::to_string(self).map_err(to_js_error)
  }

  pub fn from_json(json: &str) -> napi::Result<Self> {
    serde_json::from_str(json).map_err(to_js_error)
  }

  pub fn from_notarization(notarization: &Notarization, file_type: ArgonFileType) -> Self {
    return Self::create(notarization.balance_changes.to_vec(), file_type);
  }

  pub fn create(balance_changes: Vec<BalanceChange>, file_type: ArgonFileType) -> Self {
    return match file_type {
      ArgonFileType::Send => Self {
        version: VERSION.to_string(),
        send: Some(balance_changes),
        request: None,
      },
      ArgonFileType::Request => Self {
        version: VERSION.to_string(),
        send: None,
        request: Some(balance_changes),
      },
    };
  }
}

#[napi]
pub enum ArgonFileType {
  Send,
  Request,
}
