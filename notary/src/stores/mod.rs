use crate::error::Error;
use std::{future::Future, pin::Pin};

pub mod balance_tip;
pub mod blocks;
pub mod chain_transfer;
pub mod notarizations;
pub mod notebook;
pub mod notebook_header;
pub mod notebook_new_accounts;
pub mod notebook_status;
pub mod registered_key;

pub type BoxFutureResult<'a, T> =
	Pin<Box<dyn Future<Output = anyhow::Result<T, Error>> + Send + 'a>>;
