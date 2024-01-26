use jsonrpsee::{core::SubscriptionResult, proc_macros::rpc, types::ErrorObjectOwned};

use ulx_primitives::{
	AccountId, AccountType, BalanceProof, BalanceTip, Notarization, Notebook, NotebookMeta,
	NotebookNumber, SignedNotebookHeader,
};

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

	#[method(name = "getHeader")]
	async fn get_header(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<SignedNotebookHeader, ErrorObjectOwned>;

	#[method(name = "getRawHeaders")]
	async fn get_raw_headers(
		&self,
		since_notebook: NotebookNumber,
	) -> Result<Vec<(NotebookNumber, Vec<u8>)>, ErrorObjectOwned>;

	#[method(name = "get")]
	async fn get(&self, notebook_number: NotebookNumber) -> Result<Notebook, ErrorObjectOwned>;

	#[method(name = "getRawBody")]
	async fn get_raw_body(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<Vec<u8>, ErrorObjectOwned>;

	/// Subscription to notebook completed
	#[subscription(name = "subscribeHeaders" => "notebookHeader", item = SignedNotebookHeader)]
	async fn subscribe_headers(&self) -> SubscriptionResult;
	#[subscription(name = "subscribeRawHeaders" => "notebookRawHeader", item = (NotebookNumber, Vec<u8>))]
	async fn subscribe_raw_headers(&self) -> SubscriptionResult;
}

pub type RawHeadersSubscription = jsonrpsee::core::client::Subscription<(NotebookNumber, Vec<u8>)>;
