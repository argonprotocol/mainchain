#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

pub use pallet::*;
use pallet_prelude::*;
pub use weights::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod migrations;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use argon_primitives::{
		bitcoin::UtxoId,
		block_seal::{BlockPayout, FrameId},
		ArgonCPI, BlockRewardAccountsProvider, BlockRewardsEventHandler, BurnEventHandler,
		PriceProvider, UtxoLockEvents,
	};
	use pallet_prelude::argon_primitives::{MiningFrameProvider, MiningFrameTransitionProvider};
	use sp_runtime::FixedPointNumber;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);
	pub type MintIndex = u64;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		type WeightInfo: WeightInfo;
		type Currency: Mutate<Self::AccountId, Balance = Self::Balance>
			+ Inspect<Self::AccountId, Balance = Self::Balance>;
		type PriceProvider: PriceProvider<Self::Balance>;
		type MiningFrameProvider: MiningFrameProvider + MiningFrameTransitionProvider;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ codec::HasCompact
			+ Copy
			+ MaybeSerializeDeserialize
			+ DecodeWithMemTracking
			+ core::fmt::Debug
			+ Default
			+ From<u128>
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// The maximum number of queued mint entries a single bitcoin UTXO may accumulate.
		#[pallet::constant]
		type MaxPendingMintsPerUtxo: Get<u32>;

		/// The maximum number of queued bitcoin mints that may receive payouts in a frame.
		#[pallet::constant]
		type MaxPendingMintPayoutWindowSize: Get<u32>;

		/// The provider of reward account ids
		type BlockRewardAccountsProvider: BlockRewardAccountsProvider<Self::AccountId>;

		/// The maximum number of mint histories to keep
		#[pallet::constant]
		type MaxMintHistoryToMaintain: Get<u32>;

		/// The maximum number of miners that can be paid in a frame.
		#[pallet::constant]
		type MaxPossibleMiners: Get<u32>;

		/// The maximum share of a queued bitcoin mint that may be paid in a single frame.
		#[pallet::constant]
		type BitcoinMintPayoutPercentPerFrame: Get<Percent>;
	}

	/// Bitcoin UTXOs that have been submitted for minting, keyed by a monotonic queue index so
	/// payouts can preserve FIFO order while each frame works through a fixed payout cohort.
	#[pallet::storage]
	pub type PendingMintUtxosByIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, MintIndex, PendingMintUtxo<T>, OptionQuery>;

	/// Reverse lookup from bitcoin UTXO id to all queued mint indices for direct removal and
	/// client lookup.
	#[pallet::storage]
	pub type PendingMintUtxoIdLookup<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		UtxoId,
		BoundedVec<MintIndex, T::MaxPendingMintsPerUtxo>,
		ValueQuery,
	>;

	/// The next monotonic queue index to assign to a pending bitcoin mint.
	#[pallet::storage]
	pub type NextPendingMintUtxoIndex<T: Config> = StorageValue<_, MintIndex, ValueQuery>;

	/// Queue bookkeeping for pending bitcoin mints, including the bounded payout start and the
	/// current frame scan cursor.
	#[pallet::storage]
	pub type PendingMintQueueState<T: Config> = StorageValue<_, MintQueueCursor, ValueQuery>;

	/// The total amount of microgons minted for mining
	#[pallet::storage]
	pub type MintedMiningMicrogons<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	/// The total amount of Bitcoin microgons minted. Cannot exceed `MintedMiningMicrogons`.
	#[pallet::storage]
	pub type MintedBitcoinMicrogons<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	/// The amount of argons minted per mining cohort (ie, grouped by starting frame id)
	#[pallet::storage]
	pub type MiningMintPerCohort<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<FrameId, T::Balance, T::MaxMintHistoryToMaintain>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type BlockMintAction<T> =
		StorageValue<_, (BlockNumberFor<T>, MintAction<<T as Config>::Balance>), ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Any bitcoins minted
		BitcoinMint { account_id: T::AccountId, utxo_id: Option<UtxoId>, amount: T::Balance },
		/// The amount of microgons minted for mining. NOTE: accounts below Existential Deposit
		/// will not be able to mint
		MiningMint {
			amount: T::Balance,
			per_miner: T::Balance,
			argon_cpi: ArgonCPI,
			liquidity: T::Balance,
		},
		/// Errors encountered while minting. Most often due to mint amount still below Existential
		/// Deposit
		MintError {
			mint_type: MintType,
			account_id: T::AccountId,
			utxo_id: Option<UtxoId>,
			amount: T::Balance,
			error: DispatchError,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		TooManyPendingMints,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		<T as Config>::Balance: Into<u128>,
		<T as Config>::Balance: From<u128>,
	{
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut block_mint_action = <BlockMintAction<T>>::mutate(|(b, data)| {
				// if this is for a different block, clear it first (this can be set from other
				// pallets via event handlers)
				if *b != n {
					*b = n;
					*data = Default::default();
				}
				data.clone()
			});
			let argon_cpi = T::PriceProvider::get_argon_cpi().unwrap_or_default();
			// only mint when cpi is negative or 0
			if argon_cpi.is_positive() {
				log::trace!("Argon cpi is positive. Nothing to mint.");
				return T::DbWeight::get().reads(2);
			}

			let mut bitcoin_mint = MintedBitcoinMicrogons::<T>::get();
			let mining_mint = MintedMiningMicrogons::<T>::get();
			let mut available_bitcoin_to_mint = mining_mint.saturating_sub(bitcoin_mint);
			let mut payout_window_utxo_count = 0;
			if available_bitcoin_to_mint > T::Balance::zero() {
				let current_frame_id = T::MiningFrameProvider::get_current_frame_id();
				let next_utxo_index = NextPendingMintUtxoIndex::<T>::get();
				let mut queue_cursor = PendingMintQueueState::<T>::get();

				if queue_cursor.payout_cursor_frame_id != Some(current_frame_id) {
					queue_cursor.payout_cursor_frame_id = Some(current_frame_id);
					queue_cursor.payout_cursor_index = queue_cursor.payout_start_index;
				}

				let frame_limit = queue_cursor
					.payout_start_index
					.saturating_add(MintIndex::from(T::MaxPendingMintPayoutWindowSize::get()))
					.min(next_utxo_index);
				payout_window_utxo_count = frame_limit
					.saturating_sub(queue_cursor.payout_start_index)
					.try_into()
					.unwrap_or(u32::MAX);

				while available_bitcoin_to_mint > T::Balance::zero() &&
					queue_cursor.payout_cursor_index < frame_limit
				{
					let pending_index = queue_cursor.payout_cursor_index;

					let Some(mut mint) = PendingMintUtxosByIndex::<T>::get(pending_index) else {
						if pending_index == queue_cursor.payout_start_index {
							queue_cursor.payout_start_index.saturating_accrue(1);
						}
						queue_cursor.payout_cursor_index.saturating_accrue(1);
						continue;
					};

					let amount_to_mint = mint.remaining_amount.min(mint.max_amount_per_frame);
					if available_bitcoin_to_mint < amount_to_mint {
						break;
					}

					match T::Currency::mint_into(&mint.account_id, amount_to_mint) {
						Ok(_) => {
							available_bitcoin_to_mint.saturating_reduce(amount_to_mint);
							mint.remaining_amount.saturating_reduce(amount_to_mint);
							bitcoin_mint.saturating_accrue(amount_to_mint);
							block_mint_action.bitcoin_minted.saturating_accrue(amount_to_mint);

							Self::deposit_event(Event::<T>::BitcoinMint {
								account_id: mint.account_id.clone(),
								utxo_id: Some(mint.utxo_id),
								amount: amount_to_mint,
							});

							if mint.remaining_amount > T::Balance::zero() {
								PendingMintUtxosByIndex::<T>::insert(pending_index, mint);
							} else {
								PendingMintUtxosByIndex::<T>::remove(pending_index);
								let mut pending_indices =
									PendingMintUtxoIdLookup::<T>::take(mint.utxo_id);
								pending_indices.retain(|index| *index != pending_index);
								if !pending_indices.is_empty() {
									PendingMintUtxoIdLookup::<T>::insert(
										mint.utxo_id,
										pending_indices,
									);
								}

								if pending_index == queue_cursor.payout_start_index {
									queue_cursor.payout_start_index.saturating_accrue(1);
								}
							}

							queue_cursor.payout_cursor_index.saturating_accrue(1);
						},
						Err(e) => {
							queue_cursor.payout_cursor_index.saturating_accrue(1);
							log::warn!(
								"Failed to mint {:?} microgons for bitcoin UTXO {:?}: {:?}",
								amount_to_mint,
								&mint.utxo_id,
								e
							);
							Self::deposit_event(Event::<T>::MintError {
								mint_type: MintType::Bitcoin,
								account_id: mint.account_id.clone(),
								utxo_id: Some(mint.utxo_id),
								amount: amount_to_mint,
								error: e,
							});
						},
					};
				}

				PendingMintQueueState::<T>::put(queue_cursor);
			}
			MintedBitcoinMicrogons::<T>::put(bitcoin_mint);
			BlockMintAction::<T>::put((n, block_mint_action));
			T::WeightInfo::on_initialize(payout_window_utxo_count)
		}

		fn on_finalize(n: BlockNumberFor<T>) {
			let Some(frame_id) = T::MiningFrameProvider::is_new_frame_started() else {
				return;
			};

			let last_frame = frame_id.saturating_sub(1);
			let Some(tick_range) = T::MiningFrameProvider::get_tick_range_for_frame(last_frame)
			else {
				return;
			};
			// if there are no miners registered, we can't mint
			let reward_accounts = T::BlockRewardAccountsProvider::get_mint_rewards_accounts();
			let mut argon_cpi = T::PriceProvider::get_average_cpi_for_ticks(tick_range);
			let current_cpi = T::PriceProvider::get_argon_cpi().unwrap_or_default();

			// if cpi is coming back to zero (from average), take the current cpi instead
			if argon_cpi.is_negative() {
				argon_cpi = argon_cpi.max(current_cpi);
			}

			// only mint when cpi is negative or 0
			if !argon_cpi.is_negative() {
				log::trace!("Argon cpi is non-negative. Nothing to mint.");
				return;
			}

			let starting_liquidity = T::Currency::total_issuance();
			let microgons_to_print_per_miner =
				Self::get_microgons_to_print_per_miner(reward_accounts.len() as u128);

			let mut mining_mint = MintedMiningMicrogons::<T>::get();
			let mut block_mint_action = BlockMintAction::<T>::get().1;
			if microgons_to_print_per_miner > T::Balance::zero() {
				let mut mining_mint_history = MiningMintPerCohort::<T>::get().into_inner();
				let mut amount_minted = T::Balance::zero();
				for (miner, starting_frame_id) in reward_accounts {
					let amount = microgons_to_print_per_miner;
					match T::Currency::mint_into(&miner, amount) {
						Ok(_) => {
							mining_mint.saturating_accrue(amount);
							amount_minted.saturating_accrue(amount);
							block_mint_action.argon_minted.saturating_accrue(amount);
							if !mining_mint_history.contains_key(&starting_frame_id) &&
								mining_mint_history.len() >=
									T::MaxMintHistoryToMaintain::get() as usize
							{
								mining_mint_history.pop_first();
							}
							mining_mint_history
								.entry(starting_frame_id)
								.or_default()
								.saturating_accrue(amount);
						},
						Err(e) => {
							log::warn!(
								"Failed to mint {:?} microgons for miner {:?}: {:?}",
								amount,
								&miner,
								e
							);
							Self::deposit_event(Event::<T>::MintError {
								mint_type: MintType::Mining,
								account_id: miner.clone(),
								utxo_id: None,
								amount,
								error: e,
							});
						},
					};
				}

				MintedMiningMicrogons::<T>::put(mining_mint);

				if let Ok(result) = BoundedBTreeMap::try_from(mining_mint_history) {
					MiningMintPerCohort::<T>::put(result);
				}

				if !amount_minted.is_zero() {
					Self::deposit_event(Event::<T>::MiningMint {
						amount: amount_minted,
						per_miner: microgons_to_print_per_miner,
						argon_cpi,
						liquidity: starting_liquidity,
					});
				}
			}

			BlockMintAction::<T>::put((n, block_mint_action));
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T>
	where
		<T as Config>::Balance: Into<u128>,
		<T as Config>::Balance: From<u128>,
	{
		pub fn get_microgons_to_print_per_miner(active_miners: u128) -> T::Balance {
			if active_miners == 0 {
				return T::Balance::zero();
			}

			let microgons_to_print =
				T::PriceProvider::get_liquidity_change_needed().unwrap_or_default();
			if microgons_to_print <= 0i128 {
				return T::Balance::zero();
			}

			let per_miner = microgons_to_print as u128 / active_miners;
			per_miner.into()
		}

		pub fn get_bitcoin_mint_payout_cap(amount: T::Balance) -> T::Balance {
			T::BitcoinMintPayoutPercentPerFrame::get().mul_ceil(amount)
		}

		pub fn track_block_mint(amount: T::Balance) {
			BlockMintAction::<T>::mutate(|(b, data)| {
				let block = <frame_system::Pallet<T>>::block_number();
				if *b != block {
					*b = block;
					*data = Default::default();
				}
				data.argon_minted.saturating_accrue(amount);
			});
			MintedMiningMicrogons::<T>::mutate(|mint| mint.saturating_accrue(amount));
		}

		pub fn on_argon_burn(amount: T::Balance) {
			let bitcoin_utxos = MintedBitcoinMicrogons::<T>::get();
			BlockMintAction::<T>::mutate(|(b, data)| {
				let block = <frame_system::Pallet<T>>::block_number();
				if *b != block {
					*b = block;
					*data = Default::default();
				}
				data.argon_burned.saturating_accrue(amount);
			});

			let mining_mint = MintedMiningMicrogons::<T>::get();
			let total_minted = mining_mint + bitcoin_utxos;
			let mining_prorata = (amount * mining_mint).checked_div(&total_minted);
			if let Some(microgons) = mining_prorata {
				MintedMiningMicrogons::<T>::mutate(|mint| mint.saturating_reduce(microgons));
			}

			let bitcoin_prorata = (amount * bitcoin_utxos).checked_div(&total_minted);
			if let Some(microgons) = bitcoin_prorata {
				MintedBitcoinMicrogons::<T>::mutate(|mint| mint.saturating_reduce(microgons));
			}
		}
	}

	impl<T: Config> UtxoLockEvents<T::AccountId, T::Balance> for Pallet<T>
	where
		<T as Config>::Balance: From<u128>,
		<T as Config>::Balance: Into<u128>,
	{
		type Weights = ProviderWeightAdapter<T>;

		fn utxo_locked(
			utxo_id: UtxoId,
			account_id: &T::AccountId,
			amount: T::Balance,
		) -> sp_runtime::DispatchResult {
			if amount.is_zero() {
				return Ok(());
			}

			let pending_index = NextPendingMintUtxoIndex::<T>::get();
			PendingMintUtxoIdLookup::<T>::try_mutate(utxo_id, |pending_indices| {
				ensure!(
					pending_indices.len() < T::MaxPendingMintsPerUtxo::get() as usize,
					Error::<T>::TooManyPendingMints
				);
				pending_indices
					.try_push(pending_index)
					.map_err(|_| Error::<T>::TooManyPendingMints)
			})?;

			PendingMintUtxosByIndex::<T>::insert(
				pending_index,
				PendingMintUtxo {
					utxo_id,
					account_id: account_id.clone(),
					remaining_amount: amount,
					max_amount_per_frame: Self::get_bitcoin_mint_payout_cap(amount),
				},
			);
			NextPendingMintUtxoIndex::<T>::put(pending_index.saturating_add(1));
			Ok(())
		}

		fn utxo_released(
			utxo_id: UtxoId,
			_account_id: &T::AccountId,
			remove_pending_mints: bool,
			amount_burned: T::Balance,
			_original_liquidity_promised: T::Balance,
		) -> sp_runtime::DispatchResult {
			if remove_pending_mints {
				let pending_indices = PendingMintUtxoIdLookup::<T>::take(utxo_id);
				for pending_index in pending_indices {
					let _ = PendingMintUtxosByIndex::<T>::take(pending_index);
				}
			}

			MintedBitcoinMicrogons::<T>::mutate(|mint| *mint = mint.saturating_sub(amount_burned));
			Ok(())
		}
	}

	impl<T: Config> BurnEventHandler<T::Balance> for Pallet<T>
	where
		<T as Config>::Balance: Into<u128>,
		<T as Config>::Balance: From<u128>,
	{
		fn on_argon_burn(microgons: &T::Balance) {
			Self::on_argon_burn(*microgons);
		}
	}

	#[derive(
		Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
	)]
	pub enum MintType {
		Bitcoin,
		Mining,
	}

	#[derive(
		Debug,
		Clone,
		PartialEq,
		Eq,
		Encode,
		Default,
		Decode,
		DecodeWithMemTracking,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct MintAction<B>
	where
		B: Codec + MaxEncodedLen,
	{
		pub argon_burned: B,
		pub argon_minted: B,
		pub bitcoin_minted: B,
	}

	#[derive(
		Debug,
		Clone,
		PartialEq,
		Eq,
		Encode,
		Default,
		Decode,
		DecodeWithMemTracking,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct MintQueueCursor {
		/// Queue index where the frame's bounded payout cohort begins.
		#[codec(compact)]
		pub payout_start_index: MintIndex,
		/// Next queue index to resume scanning within the current frame's payout cohort.
		#[codec(compact)]
		pub payout_cursor_index: MintIndex,
		pub payout_cursor_frame_id: Option<FrameId>,
	}

	#[derive(
		Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct PendingMintUtxo<T: Config>
	where
		T::AccountId: Codec + MaxEncodedLen,
		T::Balance: Codec + MaxEncodedLen,
	{
		#[codec(compact)]
		pub utxo_id: UtxoId,
		pub account_id: T::AccountId,
		#[codec(compact)]
		pub remaining_amount: T::Balance,
		#[codec(compact)]
		pub max_amount_per_frame: T::Balance,
	}

	impl<T: Config> BlockRewardsEventHandler<T::AccountId, T::Balance> for Pallet<T>
	where
		<T as Config>::Balance: Into<u128>,
		<T as Config>::Balance: From<u128>,
	{
		fn rewards_created(payout: &[BlockPayout<T::AccountId, T::Balance>]) {
			let mut microgons = T::Balance::zero();
			for reward in payout {
				microgons.saturating_accrue(reward.argons);
			}
			if microgons != T::Balance::zero() {
				Self::track_block_mint(microgons);
			}
		}
	}
}
