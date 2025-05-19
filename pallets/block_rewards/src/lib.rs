#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;
use pallet_prelude::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

/// (Incremental increase per block, blocks between increments, max value)
pub type GrowthPath<Balance> = (Balance, Tick, Balance);

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use core::any::TypeId;
	use frame_support::traits::fungible::{InspectFreeze, Mutate, MutateFreeze};
	use sp_runtime::FixedPointNumber;

	use super::*;
	use argon_primitives::{
		block_seal::{BlockPayout, BlockRewardType, FrameId},
		notary::NotaryProvider,
		BlockRewardAccountsProvider, BlockRewardsEventHandler, BlockSealAuthorityId,
		BlockSealerProvider, NotebookProvider, OnNewSlot, PriceProvider, TickProvider,
	};

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		MaturationPeriod,
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		type ArgonCurrency: MutateFreeze<Self::AccountId, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>
			+ InspectFreeze<Self::AccountId, Balance = Self::Balance, Id = Self::RuntimeFreezeReason>;
		type OwnershipCurrency: MutateFreeze<Self::AccountId, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>
			+ InspectFreeze<Self::AccountId, Balance = Self::Balance, Id = Self::RuntimeFreezeReason>;

		/// The balance of an account.
		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ core::fmt::Debug
			+ Default
			+ From<u128>
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		type BlockSealerProvider: BlockSealerProvider<Self::AccountId>;
		type BlockRewardAccountsProvider: BlockRewardAccountsProvider<Self::AccountId>;
		type NotaryProvider: NotaryProvider<Self::Block, Self::AccountId>;
		type NotebookProvider: NotebookProvider;
		type PriceProvider: PriceProvider<Self::Balance>;
		/// Get a tick provider
		type TickProvider: TickProvider<Self::Block>;

		/// Number of argons minted per block
		#[pallet::constant]
		type StartingArgonsPerBlock: Get<Self::Balance>;

		/// Number of ownership tokens minted per block
		#[pallet::constant]
		type StartingOwnershipTokensPerBlock: Get<Self::Balance>;

		/// The growth path for both ownership and argons before halving
		#[pallet::constant]
		type IncrementalGrowth: Get<GrowthPath<Self::Balance>>;

		/// Number of ticks for halving of ownership share rewards
		#[pallet::constant]
		type HalvingTicks: Get<Tick>;

		/// The tick number at which the halving begins for ownership tokens
		#[pallet::constant]
		type HalvingBeginTick: Get<Tick>;

		/// Percent as a number out of 100 of the block reward that goes to the miner.
		#[pallet::constant]
		type MinerPayoutPercent: Get<FixedU128>;

		/// The overarching freeze reason.
		type RuntimeFreezeReason: From<FreezeReason>;
		type EventHandler: BlockRewardsEventHandler<Self::AccountId, Self::Balance>;

		/// The number of "argons by cohort" entries to keep in history
		type CohortBlockRewardsToKeep: Get<u32>;

		/// The number of blocks to keep payouts in history
		type PayoutHistoryBlocks: Get<u32>;

		/// Slot full duration of a slot
		type SlotWindowTicks: Get<Tick>;

		/// A percent reduction in mining argons vs the mint amount
		type PerBlockArgonReducerPercent: Get<FixedU128>;
	}

	/// Historical payouts by block number
	#[pallet::storage]
	pub(super) type PayoutsByBlock<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		BoundedVec<BlockPayout<T::AccountId, T::Balance>, ConstU32<3>>,
		ValueQuery,
	>;

	/// Bool if block rewards are paused
	#[pallet::storage]
	pub(super) type BlockRewardsPaused<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// The cohort block rewards by mining cohort (ie, with the same starting frame id)
	#[pallet::storage]
	pub(super) type BlockRewardsByCohort<T: Config> =
		StorageValue<_, BoundedVec<(FrameId, T::Balance), T::CohortBlockRewardsToKeep>, ValueQuery>;

	/// The current scaled block rewards. It will adjust based on the argon movement away from price
	/// target
	#[pallet::storage]
	pub(super) type ArgonsPerBlock<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RewardCreated {
			rewards: Vec<BlockPayout<T::AccountId, T::Balance>>,
		},
		RewardCreateError {
			account_id: T::AccountId,
			argons: Option<T::Balance>,
			ownership: Option<T::Balance>,
			error: DispatchError,
		},
	}

	/// A reason for freezing funds.
	#[pallet::composite_enum]
	pub enum FreezeReason {
		/// Pending reward maturation
		#[codec(index = 0)]
		MaturationPeriod,
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		<T as Config>::Balance: From<u128>,
		<T as Config>::Balance: Into<u128>,
	{
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			// Unlock any rewards
			let cleanup_block = n.saturating_sub(T::PayoutHistoryBlocks::get().into());
			<PayoutsByBlock<T>>::take(cleanup_block);

			T::DbWeight::get().reads_writes(1, 1)
		}

		fn on_finalize(n: BlockNumberFor<T>) {
			let elapsed_ticks = T::TickProvider::elapsed_ticks();
			let minimums = Self::get_minimum_reward_amounts(elapsed_ticks);

			// adjust the block rewards
			Self::adjust_block_argons(minimums.argons);

			if <BlockRewardsPaused<T>>::get() {
				return;
			}

			if !T::BlockSealerProvider::is_block_vote_seal() &&
				!T::BlockRewardAccountsProvider::is_compute_block_eligible_for_rewards()
			{
				log::info!("Compute block is not eligible for rewards");
				return;
			}

			let authors = T::BlockSealerProvider::get_sealer_info();
			let assigned_rewards_account =
				T::BlockRewardAccountsProvider::get_block_rewards_account(
					&authors.block_author_account_id,
				);
			let (miner_reward_account, starting_frame_id) =
				assigned_rewards_account.unwrap_or((authors.block_author_account_id.clone(), 0));

			let miner_percent = T::MinerPayoutPercent::get();

			let RewardAmounts { argons: block_argons, ownership: block_ownership } =
				Self::calculate_reward_amounts(starting_frame_id, minimums);

			let miner_ownership = miner_percent.saturating_mul_int(block_ownership);
			let miner_argons = miner_percent.saturating_mul_int(block_argons);

			let mut rewards: Vec<BlockPayout<T::AccountId, T::Balance>> = vec![BlockPayout {
				account_id: miner_reward_account.clone(),
				reward_type: BlockRewardType::Miner,
				block_seal_authority: authors.block_seal_authority.clone(),
				ownership: miner_ownership,
				argons: miner_argons,
			}];

			if let Some(ref block_vote_rewards_account) = authors.block_vote_rewards_account {
				rewards.push(BlockPayout {
					account_id: block_vote_rewards_account.clone(),
					ownership: block_ownership.saturating_sub(miner_ownership),
					argons: block_argons.saturating_sub(miner_argons),
					reward_type: BlockRewardType::Voter,
					block_seal_authority: None,
				});
			}

			for reward in rewards.iter_mut() {
				let start_argons = reward.argons;
				let start_ownership = reward.ownership;
				if let Err(e) = Self::mint::<T::ArgonCurrency>(reward) {
					log::error!("Failed to mint argons for reward: {:?}, {:?}", reward, e);
					Self::deposit_event(Event::RewardCreateError {
						account_id: reward.account_id.clone(),
						argons: Some(start_argons),
						ownership: None,
						error: e,
					});
				}
				if let Err(e) = Self::mint::<T::OwnershipCurrency>(reward) {
					log::error!("Failed to mint ownership for reward: {:?}, {:?}", reward, e);
					Self::deposit_event(Event::RewardCreateError {
						account_id: reward.account_id.clone(),
						argons: None,
						ownership: Some(start_ownership),
						error: e,
					});
				}
			}

			let rewards = rewards
				.iter()
				.filter(|r| r.argons > 0u128.into() || r.ownership > 0u128.into())
				.cloned()
				.collect::<Vec<_>>();

			if !rewards.is_empty() {
				Self::deposit_event(Event::RewardCreated { rewards: rewards.clone() });
				T::EventHandler::rewards_created(&rewards);
				<PayoutsByBlock<T>>::insert(n, BoundedVec::truncate_from(rewards));
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn set_block_rewards_paused(origin: OriginFor<T>, paused: bool) -> DispatchResult {
			ensure_root(origin)?;
			<BlockRewardsPaused<T>>::set(paused);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		T::Balance: From<u128>,
		T::Balance: Into<u128>,
	{
		/// This is expected to be called in the context of an api (eg, after a block is put
		/// together)
		pub fn block_payouts() -> Vec<BlockPayout<T::AccountId, T::Balance>> {
			let n = <frame_system::Pallet<T>>::block_number();
			<PayoutsByBlock<T>>::get(n).to_vec()
		}

		pub fn mint<C: Mutate<T::AccountId, Balance = T::Balance> + 'static>(
			reward: &mut BlockPayout<T::AccountId, T::Balance>,
		) -> DispatchResult {
			let is_ownership = TypeId::of::<C>() == TypeId::of::<T::OwnershipCurrency>();
			let amount = if is_ownership { reward.ownership } else { reward.argons };
			if amount == 0u128.into() {
				return Ok(());
			}

			C::mint_into(&reward.account_id, amount).inspect_err(|_| {
				if is_ownership {
					reward.ownership = 0u128.into();
				} else {
					reward.argons = 0u128.into();
				}
			})?;

			Ok(())
		}

		pub(crate) fn get_minimum_reward_amounts(elapsed_ticks: Tick) -> RewardAmounts<T> {
			let (increment, ticks_between_increments, final_starting_amount) =
				T::IncrementalGrowth::get();

			let final_starting_amount: u128 = final_starting_amount.into();
			let halving_begin_tick = T::HalvingBeginTick::get();
			if elapsed_ticks >= halving_begin_tick {
				let ticks_after_halving = elapsed_ticks.saturating_sub(halving_begin_tick);
				let halvings: u128 =
					ticks_after_halving.saturating_div(T::HalvingTicks::get()).into();
				return RewardAmounts {
					ownership: final_starting_amount.saturating_div(halvings + 1).into(),
					argons: final_starting_amount.into(),
				}
			}

			let start_block_argons = T::StartingArgonsPerBlock::get().into();
			let start_block_ownership = T::StartingOwnershipTokensPerBlock::get().into();

			let increments = elapsed_ticks.saturating_div(ticks_between_increments) as u128;
			let increment_sum = increments.saturating_mul(increment.into());

			RewardAmounts {
				argons: (start_block_argons + increment_sum).into(),
				ownership: (start_block_ownership + increment_sum).into(),
			}
		}

		pub(crate) fn adjust_block_argons(argon_minimum: T::Balance) {
			let liquidity_change =
				T::PriceProvider::get_liquidity_change_needed().unwrap_or_default();
			let mut block_argons = ArgonsPerBlock::<T>::get();

			let mining_window = T::SlotWindowTicks::get();
			let rewards_change = T::PerBlockArgonReducerPercent::get().saturating_mul_int(
				liquidity_change.saturating_div(mining_window.unique_saturated_into()),
			);
			let change_amount: T::Balance = rewards_change.unsigned_abs().into();
			if rewards_change.is_negative() {
				block_argons = block_argons.saturating_sub(change_amount);
			} else {
				block_argons = block_argons.saturating_add(change_amount);
			}

			if !rewards_change.is_zero() {
				log::trace!(
					"Argons per block adjusted to {:?} (change: {:?})",
					block_argons,
					rewards_change
				);
			}

			ArgonsPerBlock::<T>::put(block_argons.max(argon_minimum));
		}

		pub(crate) fn calculate_reward_amounts(
			starting_frame_id: FrameId,
			minimums: RewardAmounts<T>,
		) -> RewardAmounts<T> {
			let mut block_ownership = minimums.ownership;
			let mut block_argons = BlockRewardsByCohort::<T>::get()
				.iter()
				.find(|x| x.0 == starting_frame_id)
				.map(|x| x.1)
				.unwrap_or(minimums.argons);

			let active_notaries: T::Balance =
				T::NotaryProvider::active_notaries().len().unique_saturated_into();
			let block_notebooks = T::NotebookProvider::notebooks_in_block();
			let notebook_tick = T::TickProvider::voting_schedule().notebook_tick();
			let tick_notebooks =
				block_notebooks.iter().fold(T::Balance::zero(), |acc, (_, _, tick)| {
					if *tick == notebook_tick {
						acc + T::Balance::one()
					} else {
						acc
					}
				});

			if active_notaries > tick_notebooks {
				if tick_notebooks == Zero::zero() {
					block_ownership = T::Balance::one();
					block_argons = T::Balance::one();
				} else {
					block_ownership =
						block_ownership.saturating_mul(tick_notebooks) / active_notaries;
					block_argons = block_argons.saturating_mul(tick_notebooks) / active_notaries;
				}
			}

			RewardAmounts { ownership: block_ownership, argons: block_argons }
		}
	}

	impl<T: Config> OnNewSlot<T::AccountId> for Pallet<T> {
		type Key = BlockSealAuthorityId;
		fn on_frame_start(frame_id: FrameId) {
			let _ = BlockRewardsByCohort::<T>::mutate(|rewards| {
				if rewards.is_full() {
					rewards.remove(0);
				}
				rewards.try_push((frame_id + 1, ArgonsPerBlock::<T>::get()))
			});
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct RewardAmounts<T: Config> {
	pub argons: T::Balance,
	pub ownership: T::Balance,
}
