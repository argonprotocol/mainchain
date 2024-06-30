#![cfg_attr(not(feature = "std"), no_std)]

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
	use frame_support::{
		pallet_prelude::*,
		traits::fungible::{InspectFreeze, Mutate, MutateFreeze},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, UniqueSaturatedInto},
		Saturating,
	};
	use sp_std::{vec, vec::Vec};

	use ulx_primitives::{
		block_seal::BlockPayout, notary::NotaryProvider, tick::Tick, BlockRewardsEventHandler,
		BlockSealerProvider, NotebookProvider,
	};

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
		type UlixeeCurrency: MutateFreeze<Self::AccountId, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>
			+ InspectFreeze<Self::AccountId, Balance = Self::Balance, Id = Self::RuntimeFreezeReason>;

		/// The balance of an account.
		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ sp_std::fmt::Debug
			+ Default
			+ From<u128>
			+ TryInto<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		type BlockSealerProvider: BlockSealerProvider<Self::AccountId>;
		type NotaryProvider: NotaryProvider<Self::Block>;
		type NotebookProvider: NotebookProvider;
		type CurrentTick: Get<Tick>;
		/// Number of argons minted per block
		#[pallet::constant]
		type ArgonsPerBlock: Get<Self::Balance>;

		/// Number of ulixees minted per block
		#[pallet::constant]
		type StartingUlixeesPerBlock: Get<Self::Balance>;

		/// Number of blocks for halving of ulixee rewards
		#[pallet::constant]
		type HalvingBlocks: Get<u32>;

		/// Percent as a number out of 100 of the block reward that goes to the miner.
		#[pallet::constant]
		type MinerPayoutPercent: Get<u32>;

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
		BoundedVec<BlockPayout<T::AccountId, T::Balance>, ConstU32<2>>,
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
			let freeze_id = FreezeReason::MaturationPeriod.into();
			// Unlock any rewards
			let unlocks = <PayoutsByBlock<T>>::take(n);
			for reward in unlocks.iter() {
				let argons_frozen =
					T::ArgonCurrency::balance_frozen(&freeze_id, &reward.account_id);
				let _ = T::ArgonCurrency::set_freeze(
					&freeze_id,
					&reward.account_id,
					argons_frozen.saturating_sub(reward.argons),
				)
				.map_err(|e| {
					log::error!(target: LOG_TARGET, "Failed to unfreeze argons for reward: {:?}, {:?}", reward, e);
				});

				let ulixees_frozen =
					T::ArgonCurrency::balance_frozen(&freeze_id, &reward.account_id);
				let _ = T::UlixeeCurrency::set_freeze(
					&freeze_id,
					&reward.account_id,
					ulixees_frozen.saturating_sub(reward.ulixees),
				)
				.map_err(|e| {
					log::error!(target: LOG_TARGET, "Failed to unfreeze ulixees for reward: {:?}, {:?}", reward, e);
				});
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
				TryInto::<u128>::try_into(T::ArgonsPerBlock::get()).unwrap_or(0u128);
			let Some(block_ulixees) =
				TryInto::<u128>::try_into(T::StartingUlixeesPerBlock::get()).ok()
			else {
				log::error!(target: LOG_TARGET, "Failed to convert ulixees per block to u128");
				return;
			};

			let mut block_ulixees = block_ulixees.saturating_div(halvings + 1u128);
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
					block_ulixees = 1u128;
					block_argons = 1u128;
				} else {
					block_ulixees = block_ulixees.saturating_mul(tick_notebooks) / active_notaries;
					block_argons = block_argons.saturating_mul(tick_notebooks) / active_notaries;
				}
			}

			let miner_percent: u128 = T::MinerPayoutPercent::get().into();

			let miner_ulixees = round_up(block_ulixees, miner_percent);
			let miner_argons = round_up(block_argons, miner_percent);

			let miner_reward = BlockPayout {
				account_id: authors.miner_rewards_account.clone(),
				ulixees: miner_ulixees.into(),
				argons: miner_argons.into(),
			};

			let block_vote_reward = BlockPayout {
				// block vote rewards account is the miner if not set
				account_id: authors
					.block_vote_rewards_account
					.unwrap_or(authors.miner_rewards_account.clone())
					.clone(),
				ulixees: block_ulixees.saturating_sub(miner_ulixees).into(),
				argons: block_argons.saturating_sub(miner_argons).into(),
			};
			let mut rewards = vec![miner_reward, block_vote_reward];

			let freeze_id = FreezeReason::MaturationPeriod.into();
			let reward_height = n.saturating_add(T::MaturationBlocks::get().into());
			for reward in rewards.iter_mut() {
				if let Err(e) = T::ArgonCurrency::mint_into(&reward.account_id, reward.argons) {
					reward.argons = 0u128.into();
					log::error!(target: LOG_TARGET, "Failed to mint argons for reward: {:?}, {:?}", reward, e);
				} else {
					let frozen = T::ArgonCurrency::balance_frozen(&freeze_id, &reward.account_id);
					let _ = T::ArgonCurrency::set_freeze(
						&freeze_id,
						&reward.account_id,
						reward.argons + frozen,
					)
					.map_err(|e| {
						log::error!(target: LOG_TARGET, "Failed to freeze argons for reward: {:?}, {:?}", reward, e);
					});
				}
				if let Err(e) = T::UlixeeCurrency::mint_into(&reward.account_id, reward.ulixees) {
					reward.ulixees = 0u128.into();
					log::error!(target: LOG_TARGET, "Failed to mint ulixees for reward: {:?}, {:?}", reward, e);
				} else {
					let frozen = T::UlixeeCurrency::balance_frozen(&freeze_id, &reward.account_id);
					let _ = T::UlixeeCurrency::set_freeze(
						&freeze_id,
						&reward.account_id,
						reward.ulixees + frozen,
					)
					.map_err(|e| {
						log::error!(target: LOG_TARGET, "Failed to hold ulixees for reward: {:?}, {:?}", reward, e);
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
}

fn round_up(value: u128, percentage: u128) -> u128 {
	let numerator = value * percentage;

	let round = if numerator % 100 == 0 { 0 } else { 1 };

	numerator.saturating_div(100) + round
}
