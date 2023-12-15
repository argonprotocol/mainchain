use codec::{Decode, Encode};
use jsonrpsee::{proc_macros::rpc, types::ErrorObjectOwned};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::RuntimeDebug;

use ulx_primitives::{
	NewAccountOrigin, NotarizationBalanceChangeset, NotarizationBlockVotes, NotebookNumber,
};

#[rpc(server, client, namespace = "localchain")]
pub trait LocalchainRpc {
	#[method(name = "notarize")]
	async fn notarize(
		&self,
		balance_changeset: NotarizationBalanceChangeset,
		block_votes: NotarizationBlockVotes,
	) -> Result<BalanceChangeResult, ErrorObjectOwned>;
}
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
pub struct BalanceChangeResult {
	pub notebook_number: NotebookNumber,
	pub new_account_origins: Vec<NewAccountOrigin>,
}
