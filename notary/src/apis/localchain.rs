use codec::{Decode, Encode};
use jsonrpsee::{proc_macros::rpc, types::ErrorObjectOwned};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{bounded::BoundedVec, ConstU32, RuntimeDebug};

use ulx_notary_primitives::{
	BalanceChange, NewAccountOrigin, NotebookNumber, MAX_BALANCESET_CHANGES,
};

#[rpc(server, client, namespace = "localchain")]
pub trait LocalchainRpc {
	#[method(name = "notarize")]
	async fn notarize(
		&self,
		balance_changeset: BoundedVec<BalanceChange, ConstU32<MAX_BALANCESET_CHANGES>>,
	) -> Result<BalanceChangeResult, ErrorObjectOwned>;
}
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
pub struct BalanceChangeResult {
	pub notebook_number: NotebookNumber,
	pub finalized_block_number: u32,
	pub new_account_origins: Vec<NewAccountOrigin>,
}