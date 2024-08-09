use jsonrpsee::core::client::error::Error as JsonrpseeError;
use sp_core::crypto::{DeriveError, SecretStringError};

use argon_notary_audit::VerifyError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
  #[error(transparent)]
  Other(#[from] anyhow::Error),

  #[error("Something has gone wrong {0}")]
  Generic(String),

  #[error("IO Error: {0}")]
  IoError(#[from] std::io::Error),

  #[error("SQL Error: {0}")]
  Database(#[from] sqlx::Error),

  #[error("Mainchain API Error: {0}")]
  MainchainApiError(#[from] subxt::Error),

  #[error(transparent)]
  ParseIntError(#[from] std::num::ParseIntError),

  #[error("Error deserializing JSON: {0}")]
  SerializationError(#[from] serde_json::Error),

  #[error(transparent)]
  AuditVerifyError(#[from] VerifyError),

  #[error(transparent)]
  KeystoreSecretStringError(#[from] SecretStringError),

  #[error(transparent)]
  KeystoreDeriveError(#[from] DeriveError),

  #[error(transparent)]
  CallbackError(#[from] Box<dyn std::error::Error + Sync + Send>),

  #[error(transparent)]
  DecodingError(#[from] codec::Error),

  #[error("Notary API Error {0}")]
  NotaryApiError(argon_notary_apis::Error),

  #[error("Notary Internal API Error {0}")]
  NotaryInternalApiError(JsonrpseeError),

  #[cfg(feature = "napi")]
  #[error(transparent)]
  NapiError(#[from] napi::Error),
}

#[cfg(feature = "uniffi")]
#[derive(thiserror::Error, Debug, uniffi::Error)]
#[uniffi(flat_error)]
pub enum UniffiError {
  #[error("{0}")]
  Generic(String),
}

#[cfg(feature = "uniffi")]
pub type UniffiResult<T> = uniffi::Result<T, UniffiError>;

#[cfg(feature = "uniffi")]
impl From<Error> for UniffiError {
  fn from(e: Error) -> Self {
    UniffiError::Generic(format!("{:#?}", e))
  }
}
#[cfg(feature = "uniffi")]
impl From<anyhow::Error> for UniffiError {
  fn from(e: anyhow::Error) -> Self {
    UniffiError::Generic(format!("{:#?}", e))
  }
}

#[cfg(feature = "napi")]
pub trait NapiOk<T> {
  fn napi_ok(self) -> Result<T, napi::Error>;
}
#[cfg(feature = "napi")]
impl<T> NapiOk<T> for Result<T, Error> {
  fn napi_ok(self) -> Result<T, napi::Error> {
    self.map_err(|e| napi::Error::from_reason(format!("{:#?}", e)))
  }
}

impl From<String> for Error {
  fn from(e: String) -> Self {
    Error::Generic(e)
  }
}
impl From<JsonrpseeError> for Error {
  fn from(e: JsonrpseeError) -> Self {
    return match e {
      JsonrpseeError::Call(ref err) => {
        if let Ok(e) = err.clone().try_into() {
          return Error::NotaryApiError(e);
        }

        Error::NotaryInternalApiError(e)
      }
      _ => Error::NotaryInternalApiError(e),
    };
  }
}
