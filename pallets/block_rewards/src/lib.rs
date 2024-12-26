#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use frame_system::pallet_prelude::BlockNumberFor;
pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

/// (Incremental increase per block, blocks between increments, max value)
pub type GrowthPath<T> = (<T as Config>::Balance, BlockNumberFor<T>, <T as Config>::Balance);

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use alloc::{vec, vec::Vec};
	use core::any::TypeId;
	use frame_support::{
		pallet_prelude::*,
		traits::fungible::{InspectFreeze, Mutate, MutateFreeze},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, One, UniqueSaturatedInto},
		FixedPointNumber, FixedU128, Saturating,
	};

	use super::*;
	use argon_primitives::{
		block_seal::{BlockPayout, BlockRewardType},
		notary::NotaryProvider,
		tick::Tick,
		BlockRewardAccountsProvider, BlockRewardsEventHandler, BlockSealerProvider,
		NotebookProvider,
	};
	use sp_arithmetic::per_things::SignedRounding;

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		MaturationPeriod,
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
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
			+ TryInto<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		type BlockSealerProvider: BlockSealerProvider<Self::AccountId>;
		type BlockRewardAccountsProvider: BlockRewardAccountsProvider<Self::AccountId>;
		type NotaryProvider: NotaryProvider<Self::Block, Self::AccountId>;
		type NotebookProvider: NotebookProvider;
		type NotebookTick: Get<Tick>;
		/// Number of argons minted per block
		#[pallet::constant]
		type StartingArgonsPerBlock: Get<Self::Balance>;

		/// Number of ownership tokens minted per block
		#[pallet::constant]
		type StartingOwnershipTokensPerBlock: Get<Self::Balance>;

		/// The growth path for both ownership and argons before halving
		#[pallet::constant]
		type IncrementalGrowth: Get<GrowthPath<Self>>;

		/// Number of blocks for halving of ownership share rewards
		#[pallet::constant]
		type HalvingBlocks: Get<u32>;

		/// The block number at which the halving begins for ownership shares
		#[pallet::constant]
		type HalvingBeginBlock: Get<BlockNumberFor<Self>>;

		/// Percent as a number out of 100 of the block reward that goes to the miner.
		#[pallet::constant]
		type MinerPayoutPercent: Get<FixedU128>;

		/// Blocks until a block reward is mature
		#[pallet::constant]
		type MaturationBlocks: Get<u32>;
		/// The overarching freeze reason.
		type RuntimeFreezeReason: From<FreezeReason>;
		type EventHandler: BlockRewardsEventHandler<Self::AccountId, Self::Balance>;
	}

	#[pallet::storage]
	pub(super) type PayoutsByBlock<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		BoundedVec<BlockPayout<T::AccountId, T::Balance>, ConstU32<3>>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RewardCreated {
			maturation_block: BlockNumberFor<T>,
			rewards: Vec<BlockPayout<T::AccountId, T::Balance>>,
		},
		RewardUnlocked {
			rewards: Vec<BlockPayout<T::AccountId, T::Balance>>,
		},

		RewardUnlockError {
			account_id: T::AccountId,
			argons: Option<T::Balance>,
			ownership: Option<T::Balance>,
			error: DispatchError,
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
			let unlocks = <PayoutsByBlock<T>>::take(n);
			for reward in unlocks.iter() {
				if let Err(e) =
					Self::unfreeze_amount::<T::ArgonCurrency>(&reward.account_id, reward.argons)
				{
					log::error!("Failed to unfreeze argons for reward: {:?}, {:?}", reward, e);
					Self::deposit_event(Event::RewardUnlockError {
						account_id: reward.account_id.clone(),
						argons: Some(reward.argons),
						ownership: None,
						error: e,
					});
				}

				if let Err(e) = Self::unfreeze_amount::<T::OwnershipCurrency>(
					&reward.account_id,
					reward.ownership,
				) {
					log::error!("Failed to unfreeze ownership for reward: {:?}, {:?}", reward, e);
					Self::deposit_event(Event::RewardUnlockError {
						account_id: reward.account_id.clone(),
						argons: None,
						ownership: Some(reward.ownership),
						error: e,
					});
				}
			}
			if unlocks.len() > 0 {
				Self::deposit_event(Event::RewardUnlocked { rewards: unlocks.to_vec() });
			}
			T::DbWeight::get().reads_writes(0, 0)
		}

		fn on_finalize(n: BlockNumberFor<T>) {
			let authors = T::BlockSealerProvider::get_sealer_info();

			let RewardAmounts { argons, ownership } = Self::get_reward_amounts(n);

			let mut block_ownership = ownership.into();
			let mut block_argons = argons.into();

			let active_notaries = T::NotaryProvider::active_notaries().len() as u128;
			let block_notebooks = T::NotebookProvider::notebooks_in_block();
			let current_tick = T::NotebookTick::get();
			let tick_notebooks = block_notebooks.iter().fold(0u128, |acc, (_, _, tick)| {
				if *tick == current_tick {
					acc + 1u128
				} else {
					acc
				}
			});

			if active_notaries > tick_notebooks {
				if tick_notebooks == 0 {
					block_ownership = 1u128;
					block_argons = 1u128;
				} else {
					block_ownership =
						block_ownership.saturating_mul(tick_notebooks) / active_notaries;
					block_argons = block_argons.saturating_mul(tick_notebooks) / active_notaries;
				}
			}

			let block_ownership: T::Balance = block_ownership.into();
			let block_argons: T::Balance = block_argons.into();

			let miner_percent = T::MinerPayoutPercent::get();

			let miner_ownership: T::Balance =
				Self::saturating_mul_ceil(miner_percent, block_ownership);
			let miner_argons: T::Balance = Self::saturating_mul_ceil(miner_percent, block_argons);

			let (assigned_rewards_account, reward_sharing) =
				T::BlockRewardAccountsProvider::get_rewards_account(
					&authors.block_author_account_id,
				);
			let miner_reward_account =
				assigned_rewards_account.unwrap_or(authors.block_author_account_id.clone());

			let mut rewards: Vec<BlockPayout<T::AccountId, T::Balance>> = vec![BlockPayout {
				account_id: miner_reward_account.clone(),
				reward_type: BlockRewardType::Miner,
				block_seal_authority: authors.block_seal_authority.clone(),
				ownership: miner_ownership,
				argons: miner_argons,
			}];
			if let Some(sharing) = reward_sharing {
				let sharing_amount: T::Balance =
					Self::saturating_mul_ceil(sharing.percent_take, miner_argons);
				rewards[0].argons = miner_argons.saturating_sub(sharing_amount);
				rewards.push(BlockPayout {
					account_id: sharing.account_id,
					reward_type: BlockRewardType::ProfitShare,
					block_seal_authority: None,
					ownership: 0u128.into(),
					argons: sharing_amount,
				});
			}

			if let Some(ref block_vote_rewards_account) = authors.block_vote_rewards_account {
				rewards.push(BlockPayout {
					account_id: block_vote_rewards_account.clone(),
					ownership: block_ownership.saturating_sub(miner_ownership),
					argons: block_argons.saturating_sub(miner_argons),
					reward_type: BlockRewardType::Voter,
					block_seal_authority: None,
				});
			}

			let reward_height = n.saturating_add(T::MaturationBlocks::get().into());
			for reward in rewards.iter_mut() {
				let start_argons = reward.argons;
				let start_ownership = reward.ownership;
				if let Err(e) = Self::mint_and_freeze::<T::ArgonCurrency>(reward) {
					log::error!("Failed to mint argons for reward: {:?}, {:?}", reward, e);
					Self::deposit_event(Event::RewardCreateError {
						account_id: reward.account_id.clone(),
						argons: Some(start_argons),
						ownership: None,
						error: e,
					});
				}
				if let Err(e) = Self::mint_and_freeze::<T::OwnershipCurrency>(reward) {
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
				Self::deposit_event(Event::RewardCreated {
					maturation_block: reward_height,
					rewards: rewards.clone(),
				});
				T::EventHandler::rewards_created(&rewards);
				<PayoutsByBlock<T>>::insert(reward_height, BoundedVec::truncate_from(rewards));
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T>
	where
		T::Balance: From<u128>,
		T::Balance: Into<u128>,
	{
		/// This is expected to be called in the context of an api (eg, after a block is put
		/// together)
		pub fn block_payouts() -> Vec<BlockPayout<T::AccountId, T::Balance>> {
			let n = <frame_system::Pallet<T>>::block_number();
			let reward_height = n.saturating_add(T::MaturationBlocks::get().into());
			let rewards = <PayoutsByBlock<T>>::get(reward_height);
			rewards.to_vec()
		}

		pub fn mint_and_freeze<
			C: MutateFreeze<T::AccountId, Balance = T::Balance>
				+ Mutate<T::AccountId, Balance = T::Balance>
				+ InspectFreeze<T::AccountId, Balance = T::Balance, Id = T::RuntimeFreezeReason>
				+ 'static,
		>(
			reward: &mut BlockPayout<T::AccountId, T::Balance>,
		) -> DispatchResult {
			let freeze_id = FreezeReason::MaturationPeriod.into();
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

			let frozen = C::balance_frozen(&freeze_id, &reward.account_id);
			C::set_freeze(&freeze_id, &reward.account_id, amount + frozen)?;
			Ok(())
		}

		pub fn unfreeze_amount<
			C: MutateFreeze<T::AccountId, Balance = T::Balance>
				+ InspectFreeze<T::AccountId, Balance = T::Balance, Id = T::RuntimeFreezeReason>,
		>(
			account: &T::AccountId,
			amount: T::Balance,
		) -> DispatchResult {
			let freeze_id = FreezeReason::MaturationPeriod.into();
			let frozen = C::balance_frozen(&freeze_id, account);
			C::set_freeze(&freeze_id, account, frozen.saturating_sub(amount))?;
			Ok(())
		}

		fn saturating_mul_ceil(percent: FixedU128, balance: T::Balance) -> T::Balance {
			let other =
				FixedU128::from_u32(UniqueSaturatedInto::<u32>::unique_saturated_into(balance));

			percent
				.const_checked_mul_with_rounding(other, SignedRounding::High)
				.unwrap_or_default()
				.saturating_mul_int(T::Balance::one())
		}

		pub(crate) fn get_reward_amounts(block_number: BlockNumberFor<T>) -> RewardAmounts<T> {
			let block_number = block_as_u32::<T>(block_number);
			let (increment, blocks_between_increments, final_starting_amount) =
				T::IncrementalGrowth::get();

			let final_starting_amount: u128 = final_starting_amount.into();
			let halving_being_block = block_as_u32::<T>(T::HalvingBeginBlock::get());
			if block_number >= halving_being_block {
				let blocks_after_halving = block_number.saturating_sub(halving_being_block);
				let halvings: u128 =
					blocks_after_halving.saturating_div(T::HalvingBlocks::get()).into();
				return RewardAmounts {
					ownership: final_starting_amount.saturating_div(halvings + 1).into(),
					argons: final_starting_amount.into(),
				}
			}

			let start_block_argons = T::StartingArgonsPerBlock::get().into();
			let start_block_ownership = T::StartingOwnershipTokensPerBlock::get().into();
			let blocks_between_increments = block_as_u32::<T>(blocks_between_increments);
			let increments = block_number.saturating_div(blocks_between_increments) as u128;
			let increment_sum = increments.saturating_mul(increment.into());

			RewardAmounts {
				argons: (start_block_argons + increment_sum).into(),
				ownership: (start_block_ownership + increment_sum).into(),
			}
		}
	}

	fn block_as_u32<T: Config>(n: BlockNumberFor<T>) -> u32 {
		UniqueSaturatedInto::<u32>::unique_saturated_into(n)
	}
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct RewardAmounts<T: Config> {
	pub argons: T::Balance,
	pub ownership: T::Balance,
}
