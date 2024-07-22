use codec::{Codec, Decode, Encode, FullCodec, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_arithmetic::{FixedI128, FixedPointNumber, Percent};
use sp_core::{RuntimeDebug, H256, U256};
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Block as BlockT, CheckedDiv, UniqueSaturatedInto},
	DispatchError, DispatchResult, FixedU128, Saturating,
};
use sp_std::vec::Vec;

use crate::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinRejectedReason, Satoshis, UtxoId,
		SATOSHIS_PER_BITCOIN,
	},
	block_seal::{BlockPayout, MiningAuthority, RewardSharing},
	inherents::BlockSealInherent,
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

pub trait PriceProvider<Balance: Codec + AtLeast32BitUnsigned> {
	/// Price of the given satoshis in milligons
	fn get_bitcoin_argon_price(satoshis: Satoshis) -> Option<Balance> {
		let satoshis = FixedU128::saturating_from_integer(satoshis);
		let satoshis_per_bitcoin = FixedU128::saturating_from_integer(SATOSHIS_PER_BITCOIN);
		let milligons_per_argon = FixedU128::saturating_from_integer(1000);

		let btc_usd_price = Self::get_latest_btc_price_in_us_cents()?;
		let argon_usd_price = Self::get_latest_argon_price_in_us_cents()?;

		let satoshi_cents =
			satoshis.saturating_mul(btc_usd_price).checked_div(&satoshis_per_bitcoin)?;

		let milligons = satoshi_cents
			.saturating_mul(milligons_per_argon)
			.checked_div(&argon_usd_price)?;

		Some((milligons.into_inner() / FixedU128::accuracy()).unique_saturated_into())
	}

	/// Prices of a single bitcoin in US cents
	fn get_latest_btc_price_in_us_cents() -> Option<FixedU128>;
	/// Prices of a single argon in US cents
	fn get_latest_argon_price_in_us_cents() -> Option<FixedU128>;

	/// The argon CPI is the US CPI deconstructed by the Argon market price in Dollars.
	/// This value has 3 decimal places of precision in a whole number (eg, 1 = 1_000, -1 = -1_000)
	fn get_argon_cpi_price() -> Option<ArgonCPI>;
}

pub trait BitcoinUtxoTracker {
	fn new_utxo_id() -> UtxoId;
	fn watch_for_utxo(
		utxo_id: UtxoId,
		script_pubkey: BitcoinCosignScriptPubkey,
		satoshis: Satoshis,
		watch_for_spent_until: BitcoinHeight,
	) -> Result<(), DispatchError>;
	fn unwatch(utxo_id: UtxoId);
}

pub trait BitcoinUtxoEvents {
	fn utxo_verified(utxo_id: UtxoId) -> DispatchResult;

	fn utxo_rejected(utxo_id: UtxoId, reason: BitcoinRejectedReason) -> DispatchResult;

	fn utxo_spent(utxo_id: UtxoId) -> DispatchResult;

	fn utxo_expired(utxo_id: UtxoId) -> DispatchResult;
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl BitcoinUtxoEvents for Tuple {
	fn utxo_verified(utxo_id: UtxoId) -> DispatchResult {
		for_tuples!( #( Tuple::utxo_verified(utxo_id)?; )* );
		Ok(())
	}

	fn utxo_rejected(utxo_id: UtxoId, reason: BitcoinRejectedReason) -> DispatchResult {
		for_tuples!( #( Tuple::utxo_rejected(utxo_id, reason.clone())?; )* );
		Ok(())
	}

	fn utxo_spent(utxo_id: UtxoId) -> DispatchResult {
		for_tuples!( #( Tuple::utxo_spent(utxo_id)?; )* );
		Ok(())
	}

	fn utxo_expired(utxo_id: UtxoId) -> DispatchResult {
		for_tuples!( #( Tuple::utxo_expired(utxo_id)?; )* );
		Ok(())
	}
}

pub trait UtxoBondedEvents<AccountId: Codec, Balance: Codec + Copy> {
	fn utxo_bonded(utxo_id: UtxoId, account_id: &AccountId, amount: Balance) -> DispatchResult;
}
#[impl_trait_for_tuples::impl_for_tuples(5)]
impl<AccountId: Codec, Balance: Codec + Copy> UtxoBondedEvents<AccountId, Balance> for Tuple {
	fn utxo_bonded(utxo_id: UtxoId, account_id: &AccountId, amount: Balance) -> DispatchResult {
		for_tuples!( #( Tuple::utxo_bonded(utxo_id, account_id, amount)?; )* );
		Ok(())
	}
}

/// Argon CPI is the US CPI deconstructed by the Argon market price in Dollars
pub type ArgonCPI = FixedI128;

pub trait ChainTransferLookup<AccountId, Balance> {
	fn is_valid_transfer_to_localchain(
		notary_id: NotaryId,
		transfer_to_localchain_id: TransferToLocalchainId,
		account_id: &AccountId,
		milligons: Balance,
		for_notebook_tick: Tick,
	) -> bool;
}

pub trait BlockVotingProvider<Block: BlockT> {
	fn grandparent_vote_minimum() -> Option<VoteMinimum>;
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
pub struct BlockSealerInfo<AccountId: FullCodec> {
	pub block_author_account_id: AccountId,
	/// The voting account, if a block seal
	pub block_vote_rewards_account: Option<AccountId>,
}

pub trait BlockSealerProvider<AccountId: FullCodec> {
	fn get_sealer_info() -> BlockSealerInfo<AccountId>;
}

pub trait BlockRewardAccountsProvider<AccountId: FullCodec> {
	fn get_rewards_account(
		author: &AccountId,
	) -> (Option<AccountId>, Option<RewardSharing<AccountId>>);
	/// Returns all rewards accounts and the share they receive
	fn get_all_rewards_accounts() -> Vec<(AccountId, Option<RewardShare>)>;
}

pub trait MiningSlotProvider<BlockNumber> {
	fn get_next_slot_block_number() -> BlockNumber;
	fn mining_window_blocks() -> BlockNumber;
}

pub type RewardShare = Percent;
pub trait AuthorityProvider<AuthorityId, Block, AccountId>
where
	Block: BlockT,
{
	fn get_authority(author: AccountId) -> Option<AuthorityId>;
	fn xor_closest_authority(nonce: U256) -> Option<MiningAuthority<AuthorityId, AccountId>>;
}

pub trait TickProvider<B: BlockT> {
	fn current_tick() -> Tick;
	fn ticker() -> Ticker;
	fn blocks_at_tick(tick: Tick) -> Vec<B::Hash>;
}

/// An event handler to listen for submitted notebook
pub trait NotebookEventHandler {
	fn notebook_submitted(header: &NotebookHeader);
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl NotebookEventHandler for Tuple {
	fn notebook_submitted(header: &NotebookHeader) {
		for_tuples!( #( Tuple::notebook_submitted(&header); )* );
	}
}

/// An event handler to listen for submitted block seals
pub trait BlockSealEventHandler {
	fn block_seal_read(seal: &BlockSealInherent);
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl BlockSealEventHandler for Tuple {
	fn block_seal_read(seal: &BlockSealInherent) {
		for_tuples!( #( Tuple::block_seal_read(seal); )* );
	}
}

/// An event handler to listen for submitted notebook
pub trait BurnEventHandler<Balance> {
	fn on_argon_burn(milligons: &Balance);
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl<Balance> BurnEventHandler<Balance> for Tuple {
	fn on_argon_burn(milligons: &Balance) {
		for_tuples!( #( Tuple::on_argon_burn(milligons); )* );
	}
}

pub trait BlockRewardsEventHandler<AccountId: Codec, Balance: Codec + MaxEncodedLen> {
	fn rewards_created(payout: &[BlockPayout<AccountId, Balance>]);
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl<AccountId: Codec, Balance: Codec + MaxEncodedLen> BlockRewardsEventHandler<AccountId, Balance>
	for Tuple
{
	fn rewards_created(payout: &[BlockPayout<AccountId, Balance>]) {
		for_tuples!( #( Tuple::rewards_created(&payout); )* );
	}
}
