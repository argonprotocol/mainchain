#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

const LOG_TARGET: &str = "runtime::block_rewards";
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

	use argon_primitives::{
		block_seal::BlockPayout, notary::NotaryProvider, tick::Tick, BlockRewardAccountsProvider,
		BlockRewardsEventHandler, BlockSealerProvider, NotebookProvider,
	};
	use sp_arithmetic::per_things::SignedRounding;

	use super::*;

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
		type SharesCurrency: MutateFreeze<Self::AccountId, Balance = Self::Balance>
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
		type NotaryProvider: NotaryProvider<Self::Block>;
		type NotebookProvider: NotebookProvider;
		type CurrentTick: Get<Tick>;
		/// Number of argons minted per block
		#[pallet::constant]
		type ArgonsPerBlock: Get<Self::Balance>;

		/// Number of shares minted per block
		#[pallet::constant]
		type StartingSharesPerBlock: Get<Self::Balance>;

		/// Number of blocks for halving of ownership share rewards
		#[pallet::constant]
		type HalvingBlocks: Get<u32>;

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
			shares: Option<T::Balance>,
			error: DispatchError,
		},
		RewardCreateError {
			account_id: T::AccountId,
			argons: Option<T::Balance>,
			shares: Option<T::Balance>,
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
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			// Unlock any rewards
			let unlocks = <PayoutsByBlock<T>>::take(n);
			for reward in unlocks.iter() {
				if let Err(e) =
					Self::unfreeze_amount::<T::ArgonCurrency>(&reward.account_id, reward.argons)
				{
					log::error!(target: LOG_TARGET, "Failed to unfreeze argons for reward: {:?}, {:?}", reward, e);
					Self::deposit_event(Event::RewardUnlockError {
						account_id: reward.account_id.clone(),
						argons: Some(reward.argons),
						shares: None,
						error: e,
					});
				}

				if let Err(e) =
					Self::unfreeze_amount::<T::SharesCurrency>(&reward.account_id, reward.shares)
				{
					log::error!(target: LOG_TARGET, "Failed to unfreeze shares for reward: {:?}, {:?}", reward, e);
					Self::deposit_event(Event::RewardUnlockError {
						account_id: reward.account_id.clone(),
						argons: None,
						shares: Some(reward.shares),
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

			let block_number = UniqueSaturatedInto::<u32>::unique_saturated_into(n);
			let halvings: u128 = block_number.saturating_div(T::HalvingBlocks::get()).into();

			let mut block_argons =
				UniqueSaturatedInto::<u128>::unique_saturated_into(T::ArgonsPerBlock::get());
			let block_shares = UniqueSaturatedInto::<u128>::unique_saturated_into(
				T::StartingSharesPerBlock::get(),
			);

			let mut block_shares = block_shares.saturating_div(halvings + 1u128);
			let active_notaries = T::NotaryProvider::active_notaries().len() as u128;
			let block_notebooks = T::NotebookProvider::notebooks_in_block();
			let current_tick = T::CurrentTick::get();
			let tick_notebooks = block_notebooks.iter().fold(0u128, |acc, (_, _, tick)| {
				if *tick == current_tick {
					acc + 1u128
				} else {
					acc
				}
			});

			if active_notaries > tick_notebooks {
				if tick_notebooks == 0 {
					block_shares = 1u128;
					block_argons = 1u128;
				} else {
					block_shares = block_shares.saturating_mul(tick_notebooks) / active_notaries;
					block_argons = block_argons.saturating_mul(tick_notebooks) / active_notaries;
				}
			}

			let block_shares: T::Balance = block_shares.into();
			let block_argons: T::Balance = block_argons.into();

			let miner_percent = T::MinerPayoutPercent::get();

			let miner_shares: T::Balance = Self::saturating_mul_ceil(miner_percent, block_shares);
			let miner_argons: T::Balance = Self::saturating_mul_ceil(miner_percent, block_argons);

			let (assigned_rewards_account, reward_sharing) =
				T::BlockRewardAccountsProvider::get_rewards_account(
					&authors.block_author_account_id,
				);
			let miner_reward_account =
				assigned_rewards_account.unwrap_or(authors.block_author_account_id.clone());

			let mut rewards: Vec<BlockPayout<T::AccountId, T::Balance>> = vec![BlockPayout {
				account_id: miner_reward_account.clone(),
				shares: miner_shares,
				argons: miner_argons,
			}];
			if let Some(sharing) = reward_sharing {
				let sharing_amount: T::Balance =
					Self::saturating_mul_ceil(sharing.percent_take, miner_argons);
				rewards[0].argons = miner_argons.saturating_sub(sharing_amount);
				rewards.push(BlockPayout {
					account_id: sharing.account_id,
					shares: 0u128.into(),
					argons: sharing_amount,
				});
			}

			rewards.push(BlockPayout {
				// block vote rewards account is the miner if not set
				account_id: authors
					.block_vote_rewards_account
					.unwrap_or(authors.block_author_account_id.clone())
					.clone(),
				shares: block_shares.saturating_sub(miner_shares),
				argons: block_argons.saturating_sub(miner_argons),
			});

			let reward_height = n.saturating_add(T::MaturationBlocks::get().into());
			for reward in rewards.iter_mut() {
				let start_argons = reward.argons.clone();
				let start_shares = reward.shares.clone();
				if let Err(e) = Self::mint_and_freeze::<T::ArgonCurrency>(reward) {
					log::error!(target: LOG_TARGET, "Failed to mint argons for reward: {:?}, {:?}", reward, e);
					Self::deposit_event(Event::RewardCreateError {
						account_id: reward.account_id.clone(),
						argons: Some(start_argons),
						shares: None,
						error: e,
					});
				}
				if let Err(e) = Self::mint_and_freeze::<T::SharesCurrency>(reward) {
					log::error!(target: LOG_TARGET, "Failed to mint shares for reward: {:?}, {:?}", reward, e);
					Self::deposit_event(Event::RewardCreateError {
						account_id: reward.account_id.clone(),
						argons: None,
						shares: Some(start_shares),
						error: e,
					});
				}
			}

			Self::deposit_event(Event::RewardCreated {
				maturation_block: reward_height,
				rewards: rewards.clone(),
			});
			T::EventHandler::rewards_created(&rewards);
			<PayoutsByBlock<T>>::insert(reward_height, BoundedVec::truncate_from(rewards));
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		pub fn mint_and_freeze<
			C: MutateFreeze<T::AccountId, Balance = T::Balance>
				+ Mutate<T::AccountId, Balance = T::Balance>
				+ InspectFreeze<T::AccountId, Balance = T::Balance, Id = T::RuntimeFreezeReason>
				+ 'static,
		>(
			reward: &mut BlockPayout<T::AccountId, T::Balance>,
		) -> DispatchResult {
			let freeze_id = FreezeReason::MaturationPeriod.into();
			let is_shares = TypeId::of::<C>() == TypeId::of::<T::SharesCurrency>();
			let amount = if is_shares { reward.shares } else { reward.argons };
			if amount == 0u128.into() {
				return Ok(());
			}

			C::mint_into(&reward.account_id, amount).map_err(|e| {
				if is_shares {
					reward.shares = 0u128.into();
				} else {
					reward.argons = 0u128.into();
				}
				e
			})?;

			let frozen = C::balance_frozen(&freeze_id, &reward.account_id);
			let _ = C::set_freeze(&freeze_id, &reward.account_id, amount + frozen)?;
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
	}
}
