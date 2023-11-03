use jsonrpsee::{core::SubscriptionResult, proc_macros::rpc, types::ErrorObjectOwned};
use ulx_notary_primitives::{BalanceProof, BalanceTip, NotebookHeader, NotebookNumber};

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
	/// Subscription to notebooks completed
	#[subscription(name = "subscribeHeaders" => "notebookHeader", item = NotebookHeader)]
	async fn subscribe_headers(&self) -> SubscriptionResult;
}
