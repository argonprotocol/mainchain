use codec::{Decode, Encode};
use jsonrpsee::{proc_macros::rpc, types::ErrorObjectOwned};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{H256, RuntimeDebug};

use argon_primitives::{
	AccountId, AccountOrigin, AccountType, NewAccountOrigin, NotarizationBalanceChangeset,
	NotarizationBlockVotes, NotarizationDomains, NotebookNumber, tick::Tick,
};

#[rpc(server, client, namespace = "localchain")]
pub trait LocalchainRpc {
	#[method(name = "notarize")]
	async fn notarize(
		&self,
		balance_changeset: NotarizationBalanceChangeset,
		block_votes: NotarizationBlockVotes,
		domains: NotarizationDomains,
	) -> Result<BalanceChangeResult, ErrorObjectOwned>;

	#[method(name = "getTip")]
	async fn get_tip(
		&self,
		account_id: AccountId,
		account_type: AccountType,
	) -> Result<BalanceTipResult, ErrorObjectOwned>;

	#[method(name = "getAccountOrigin")]
	async fn get_origin(
		&self,
		account_id: AccountId,
		account_type: AccountType,
	) -> Result<AccountOrigin, ErrorObjectOwned>;
}
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceChangeResult {
	pub notebook_number: NotebookNumber,
	pub tick: Tick,
	pub new_account_origins: Vec<NewAccountOrigin>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceTipResult {
	pub notebook_number: NotebookNumber,
	pub tick: Tick,
	pub balance_tip: H256,
}
