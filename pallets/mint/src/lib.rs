#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

pub use pallet::*;
use pallet_prelude::*;
pub use weights::*;

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
		ArgonCPI, BlockRewardAccountsProvider, BlockRewardsEventHandler, BurnEventHandler,
		PriceProvider, UtxoLockEvents, bitcoin::UtxoId, block_seal::BlockPayout,
	};
	use pallet_prelude::argon_primitives::MiningFrameProvider;
	use sp_runtime::FixedPointNumber;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		type WeightInfo: WeightInfo;
		type Currency: Mutate<Self::AccountId, Balance = Self::Balance>
			+ Inspect<Self::AccountId, Balance = Self::Balance>;
		type PriceProvider: PriceProvider<Self::Balance>;
		type MiningFrameProvider: MiningFrameProvider;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ DecodeWithMemTracking
			+ core::fmt::Debug
			+ Default
			+ From<u128>
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// The maximum number of UTXOs that can be waiting for minting
		#[pallet::constant]
		type MaxPendingMintUtxos: Get<u32>;

		/// The provider of reward account ids
		type BlockRewardAccountsProvider: BlockRewardAccountsProvider<Self::AccountId>;

		/// The maximum number of mint histories to keep
		#[pallet::constant]
		type MaxMintHistoryToMaintain: Get<u32>;

		#[pallet::constant]
		type MaxPossibleMiners: Get<u32>;
	}

	/// Bitcoin UTXOs that have been submitted for minting. This list is FIFO for minting whenever
	/// a) CPI >= 0 and
	/// b) the aggregate minted Bitcoins <= the aggregate minted microgons from mining
	#[pallet::storage]
	pub type PendingMintUtxos<T: Config> = StorageValue<
		_,
		BoundedVec<(UtxoId, T::AccountId, T::Balance), T::MaxPendingMintUtxos>,
		ValueQuery,
	>;

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
		/// The amount of microgons minted for mining. NOTE: accounts below Existential Deposit will
		/// not be able to mint
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
			if available_bitcoin_to_mint > T::Balance::zero() {
				let updated = <PendingMintUtxos<T>>::get().try_mutate(|pending| {
					pending.retain_mut(|(utxo_id, account_id, remaining_account_mint)| {
						if available_bitcoin_to_mint == T::Balance::zero() {
							return true;
						}

						let amount_to_mint = if available_bitcoin_to_mint >= *remaining_account_mint
						{
							*remaining_account_mint
						} else {
							available_bitcoin_to_mint
						};

						match T::Currency::mint_into(account_id, amount_to_mint) {
							Ok(_) => {
								available_bitcoin_to_mint -= amount_to_mint;
								*remaining_account_mint -= amount_to_mint;
								bitcoin_mint += amount_to_mint;
								block_mint_action.bitcoin_minted += amount_to_mint;

								Self::deposit_event(Event::<T>::BitcoinMint {
									account_id: account_id.clone(),
									utxo_id: Some(*utxo_id),
									amount: amount_to_mint,
								});
							},
							Err(e) => {
								log::warn!(
									"Failed to mint {:?} microgons for bitcoin UTXO {:?}: {:?}",
									amount_to_mint,
									&utxo_id,
									e
								);
								Self::deposit_event(Event::<T>::MintError {
									mint_type: MintType::Bitcoin,
									account_id: account_id.clone(),
									utxo_id: Some(*utxo_id),
									amount: amount_to_mint,
									error: e,
								});
							},
						};
						*remaining_account_mint > T::Balance::zero()
					});
				});
				PendingMintUtxos::<T>::put(updated.expect("cannot fail, but should be handled"));
			}
			MintedBitcoinMicrogons::<T>::put(bitcoin_mint);
			BlockMintAction::<T>::put((n, block_mint_action));
			T::DbWeight::get().reads_writes(1, 1)
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
							mining_mint += amount;
							amount_minted += amount;
							block_mint_action.argon_minted += amount;
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

		pub fn track_block_mint(amount: T::Balance) {
			BlockMintAction::<T>::mutate(|(b, data)| {
				let block = <frame_system::Pallet<T>>::block_number();
				if *b != block {
					*b = block;
					*data = Default::default();
				}
				data.argon_minted += amount;
			});
			MintedMiningMicrogons::<T>::mutate(|mint| *mint += amount);
		}

		pub fn on_argon_burn(amount: T::Balance) {
			let bitcoin_utxos = MintedBitcoinMicrogons::<T>::get();
			BlockMintAction::<T>::mutate(|(b, data)| {
				let block = <frame_system::Pallet<T>>::block_number();
				if *b != block {
					*b = block;
					*data = Default::default();
				}
				data.argon_burned += amount;
			});

			let mining_mint = MintedMiningMicrogons::<T>::get();
			let total_minted = mining_mint + bitcoin_utxos;
			let mining_prorata = (amount * mining_mint).checked_div(&total_minted);
			if let Some(microgons) = mining_prorata {
				MintedMiningMicrogons::<T>::mutate(|mint| *mint -= microgons);
			}

			let bitcoin_prorata = (amount * bitcoin_utxos).checked_div(&total_minted);
			if let Some(microgons) = bitcoin_prorata {
				MintedBitcoinMicrogons::<T>::mutate(|mint| *mint -= microgons);
			}
		}
	}

	impl<T: Config> UtxoLockEvents<T::AccountId, T::Balance> for Pallet<T>
	where
		<T as Config>::Balance: From<u128>,
		<T as Config>::Balance: Into<u128>,
	{
		fn utxo_locked(
			utxo_id: UtxoId,
			account_id: &T::AccountId,
			amount: T::Balance,
		) -> sp_runtime::DispatchResult {
			<PendingMintUtxos<T>>::try_mutate(|x| -> DispatchResult {
				x.try_push((utxo_id, account_id.clone(), amount))
					.map_err(|_| Error::<T>::TooManyPendingMints.into())
			})?;
			Ok(())
		}

		fn utxo_released(
			utxo_id: UtxoId,
			remove_pending_mints: bool,
			amount_burned: T::Balance,
		) -> sp_runtime::DispatchResult {
			if remove_pending_mints {
				<PendingMintUtxos<T>>::mutate(|x| {
					x.retain(|(id, _, _)| id != &utxo_id);
				});
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

	impl<T: Config> BlockRewardsEventHandler<T::AccountId, T::Balance> for Pallet<T>
	where
		<T as Config>::Balance: Into<u128>,
		<T as Config>::Balance: From<u128>,
	{
		fn rewards_created(payout: &[BlockPayout<T::AccountId, T::Balance>]) {
			let mut microgons = T::Balance::zero();
			for reward in payout {
				microgons = microgons.saturating_add(reward.argons);
			}
			if microgons != T::Balance::zero() {
				Self::track_block_mint(microgons);
			}
		}
	}
}
