use codec::{Codec, Decode, Encode, FullCodec, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{RuntimeDebug, H256, U256};
use sp_runtime::{traits::Block as BlockT, DispatchResult};
use sp_std::vec::Vec;

use crate::{
	bitcoin::Satoshis,
	block_seal::{BlockPayout, MiningAuthority},
	tick::{Tick, Ticker},
	DataDomainHash, NotaryId, NotebookHeader, NotebookNumber, NotebookSecret,
	TransferToLocalchainId, VoteMinimum,
};

pub trait DataDomainProvider<AccountId> {
	fn is_registered_payment_account(
		data_domain_hash: &DataDomainHash,
		account_id: &AccountId,
		tick_range: (Tick, Tick),
	) -> bool;
}

pub trait MintCirculationProvider<Balance> {
	fn get_mint_circulation() -> Balance;
}

pub trait NotebookProvider {
	/// Returns a block voting root only if submitted in time for previous block
	fn get_eligible_tick_votes_root(
		notary_id: NotaryId,
		tick: Tick,
	) -> Option<(H256, NotebookNumber)>;

	fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)>;

	/// Returns notebooks by notary with their parent secret
	fn notebooks_at_tick(tick: Tick) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)>;

	fn is_notary_locked_at_tick(notary_id: NotaryId, tick: Tick) -> bool;
}

pub trait BitcoinPriceProvider<Balance: Codec> {
	/// Prices of the given satoshis in milligons
	fn get_bitcoin_argon_prices(satoshis: Satoshis) -> Vec<Balance>;
	/// Price of the given satoshis in milligons
	fn get_bitcoin_argon_price(satoshis: Satoshis) -> Option<Balance>;
	/// Prices of a single bitcoin in US cents
	fn get_latest_price_in_us_cents() -> Option<u64>;
}

/// Argon CPI is the US CPI deconstructed by the Argon market price in Dollars with 3 decimal places
/// of precision in a whole number (eg, 1 = 1_000, -1 = -1_000)
pub type ArgonCPI = i16;
pub trait ArgonPriceProvider {
	/// The argon CPI is the US CPI deconstructed by the Argon market price in Dollars.
	/// This value has 3 decimal places of precision in a whole number (eg, 1 = 1_000, -1 = -1_000)
	fn get_argon_cpi_price() -> Option<ArgonCPI>;
	/// Prices of a single argon in US cents
	fn get_latest_price_in_us_cents() -> Option<u64>;
}

pub trait ChainTransferLookup<AccountId, Balance> {
	fn is_valid_transfer_to_localchain(
		notary_id: NotaryId,
		transfer_to_localchain_id: TransferToLocalchainId,
		account_id: &AccountId,
		milligons: Balance,
	) -> bool;
}

pub trait BlockVotingProvider<Block: BlockT> {
	fn grandparent_vote_minimum() -> Option<VoteMinimum>;
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
pub struct BlockSealerInfo<AccountId: FullCodec> {
	pub miner_rewards_account: AccountId,
	pub block_vote_rewards_account: AccountId,
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

pub trait TickProvider<B: BlockT> {
	fn current_tick() -> Tick;
	fn ticker() -> Ticker;
	fn blocks_at_tick(tick: Tick) -> Vec<B::Hash>;
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

/// An event handler to listen for submitted notebook
pub trait BurnEventHandler<Balance> {
	fn on_argon_burn(milligons: &Balance) -> DispatchResult;
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl<Balance> BurnEventHandler<Balance> for Tuple {
	fn on_argon_burn(milligons: &Balance) -> DispatchResult {
		for_tuples!( #( Tuple::on_argon_burn(milligons); )* );
		Ok(())
	}
}

pub trait BlockRewardsEventHandler<AccountId: Codec, Balance: Codec> {
	fn rewards_created(payout: &Vec<BlockPayout<AccountId, Balance>>);
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl<AccountId: Codec, Balance: Codec> BlockRewardsEventHandler<AccountId, Balance> for Tuple {
	fn rewards_created(payout: &Vec<BlockPayout<AccountId, Balance>>) {
		for_tuples!( #( Tuple::rewards_created(&payout); )* );
	}
}
