use crate::get_header_url;
use argon_primitives::{
	AccountId, AccountType, BalanceProof, BalanceTip, Notarization, NotebookMeta, NotebookNumber,
	SignedNotebookHeader, prelude::Tick,
};
use codec::{Decode, Encode};
use jsonrpsee::{
	core::{Serialize, SubscriptionResult},
	proc_macros::rpc,
	types::ErrorObjectOwned,
};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use serde::Deserialize;
use sp_core::{H256, RuntimeDebug};

#[rpc(server, client, namespace = "notebook")]
pub trait NotebookRpc {
	/// EXPERIMENTAL: Get proofs for a set of accounts. Localchain wallets will normally do this
	/// themselves.
	#[method(name = "getBalanceChangeProof")]
	async fn get_balance_proof(
		&self,
		notebook_number: NotebookNumber,
		balance_tip: BalanceTip,
	) -> Result<BalanceProof, ErrorObjectOwned>;

	#[method(name = "getNotarization")]
	async fn get_notarization(
		&self,
		account_id: AccountId,
		account_type: AccountType,
		notebook_number: NotebookNumber,
		change_number: u32,
	) -> Result<Notarization, ErrorObjectOwned>;

	#[method(name = "metadata")]
	async fn metadata(&self) -> Result<NotebookMeta, ErrorObjectOwned>;

	#[method(name = "getHeaderDownloadUrl")]
	async fn get_header_download_url(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<String, ErrorObjectOwned>;

	#[method(name = "getNotebookDownloadUrl")]
	async fn get_notebook_download_url(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<String, ErrorObjectOwned>;

	/// Subscription to notebook completed
	#[subscription(name = "subscribeHeaders" => "headerDownload", item = NotebookSubscriptionBroadcast)]
	async fn subscribe_headers(&self) -> SubscriptionResult;
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotebookSubscriptionBroadcast {
	pub notebook_number: NotebookNumber,
	pub tick: Tick,
	pub notebook_header_hash: H256,
	pub header_download_url: String,
}

impl NotebookSubscriptionBroadcast {
	pub fn build(
		info: (SignedNotebookHeader, H256),
		archive_host: &str,
	) -> NotebookSubscriptionBroadcast {
		let (header, hash) = info;
		Self {
			notebook_number: header.header.notebook_number,
			tick: header.header.tick,
			header_download_url: get_header_url(
				archive_host,
				header.header.notary_id,
				header.header.notebook_number,
			),
			notebook_header_hash: hash,
		}
	}
}

pub type RawHeadersSubscription =
	jsonrpsee::core::client::Subscription<NotebookSubscriptionBroadcast>;
