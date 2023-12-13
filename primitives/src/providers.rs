use codec::{Decode, Encode, FullCodec, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_api::BlockT;
use sp_core::{RuntimeDebug, H256, U256};
use sp_runtime::DispatchResult;

pub use ulx_notary_primitives::NotaryId;
use ulx_notary_primitives::{NotebookHeader, NotebookNumber, VoteMinimum};

use crate::{
	block_seal::MiningAuthority,
	tick::{Tick, Ticker},
};

pub trait NotebookProvider {
	/// Returns a block voting root only if submitted in time for previous block
	fn get_eligible_tick_votes_root(
		notary_id: NotaryId,
		tick: Tick,
	) -> Option<(H256, NotebookNumber)>;
}

pub trait ChainTransferLookup<Nonce, AccountId> {
	fn is_valid_transfer_to_localchain(
		notary_id: NotaryId,
		account_id: &AccountId,
		nonce: Nonce,
	) -> bool;
}

pub trait BlockVotingProvider<Block: BlockT> {
	fn grandparent_vote_minimum() -> Option<VoteMinimum>;
	fn parent_voting_key() -> Option<H256>;
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
pub struct BlockSealerInfo<AccountId: FullCodec> {
	pub miner_rewards_account: AccountId,
	pub block_vote_rewards_account: AccountId,
	pub notaries_included: u32,
}

pub trait BlockSealerProvider<AccountId: FullCodec> {
	fn get_sealer_info() -> BlockSealerInfo<AccountId>;
}

pub trait AuthorityProvider<AuthorityId, Block, AccountId>
where
	Block: BlockT,
{
	fn get_authority(author: AccountId) -> Option<AuthorityId>;
	fn get_rewards_account(author: AccountId) -> Option<AccountId>;
	fn xor_closest_authority(nonce: U256) -> Option<MiningAuthority<AuthorityId, AccountId>>;
}

pub trait TickProvider {
	fn current_tick() -> Tick;
	fn ticker() -> Ticker;
}

/// An event handler to listen for submitted notebook
pub trait NotebookEventHandler {
	fn notebook_submitted(header: &NotebookHeader) -> DispatchResult;
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl NotebookEventHandler for Tuple {
	fn notebook_submitted(header: &NotebookHeader) -> DispatchResult {
		for_tuples!( #( Tuple::notebook_submitted(&header); )* );
		Ok(())
	}
}
