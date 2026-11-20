#![cfg_attr(not(feature = "std"), no_std)]
//! Claimant-submitted checkpoints for maintained Uniswap v4 positions.
//!
//! Checkpoints establish continuity and maintain registered liquidity by Uniswap range.

extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};
pub use argon_primitives::EthereumProofBlock as ProofBlock;
use argon_primitives::{
	block_seal::FrameId,
	treasury::{
		StabilizerPoolFrameClaim, StabilizerPoolPositionProvider, StabilizerPoolRewardProvider,
		TreasuryPositionProvider,
	},
	Balance, BlockSealAuthorityId, EthereumAccountStorageProof, EthereumAccountStorageWitness,
	EthereumAccountStorageWitnessRef, EthereumExecutionBlockProof, EthereumVerifyProvider,
	MiningFrameTransitionProvider, OnNewSlot, PriceProvider, UniswapNftId, UniswapTick,
};
use frame_support::traits::Time;
pub use pallet::*;
use pallet_prelude::*;
use polkadot_sdk::sp_core::U512;
use sp_runtime::ArithmeticError;
use uniswap::{MAX_REWARD_RANGES, REWARD_BAND_TOLERANCE};
use weights::liquidity_snapshot_weight;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod evm;
pub mod migrations;
#[cfg(any(test, feature = "runtime-benchmarks"))]
mod mock;
mod price_history;
#[cfg(test)]
mod tests;
mod uniswap;
pub mod weights;

pub use price_history::{
	StabilizerPoolFramePrice, StabilizerPoolPriceSamples, StabilizerPoolPriceSampling,
};
pub use uniswap::{PositionAmountError, PositionAmounts, StabilizerPoolRewardBand};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	// FRAME requires this explicit path for the system configuration.
	pub trait Config: polkadot_sdk::frame_system::Config {
		type WeightInfo: weights::WeightInfo;
		/// Current, nonexpired market and target quotes used to admit new earning liquidity.
		type PriceProvider: PriceProvider<Balance>;
		/// Authenticates Ethereum state and identifies the block proved by each witness.
		type EthereumVerifier: EthereumVerifyProvider;
		/// Fixes financial frame rates and settles earned ARGON against Treasury funding.
		type RewardProvider: StabilizerPoolRewardProvider;
		/// Makes source owners discoverable for account collection.
		type TreasuryPositionProvider: TreasuryPositionProvider<Self::AccountId, Balance>;
		/// Share of rewards assigned to USDC principal. ARGON receives the remainder.
		#[pallet::constant]
		type UsdcRewardShare: Get<Percent>;
		/// Maximum beneficiary slots, including reported NFTs held for recovery or collection.
		#[pallet::constant]
		type MaxPositionsPerAccount: Get<u32>;
		/// Maximum Ethereum handoff gap in milliseconds; finality and relay waits are excluded.
		#[pallet::constant]
		type ContinuationGraceMillis: Get<Moment>;
		/// Supplies the mining frame active when a checkpoint is accepted.
		type MiningFrameTransitionProvider: MiningFrameTransitionProvider;
		/// Mainchain clock used to assign observations to frames and reject future prices.
		type CurrentTick: Get<Tick>;
		/// Unix time of the block that activates a mining frame.
		type TimeProvider: Time<Moment = Moment>;
		/// Closed mining frames retained in the sampled Uniswap price history.
		#[pallet::constant]
		type MaxSampledPriceHistoryFrames: Get<u32>;
		/// Frames available to collect after Treasury finalizes the reward rate.
		#[pallet::constant]
		type ClaimWindowFrames: Get<u32>;
		/// Nominal frame duration used to size the price sampling buckets.
		type TicksPerFrame: Get<Tick>;
		/// Lower and upper reward price boundaries, as percentages of the ARGON USD target.
		/// The lower boundary must be positive and strictly below the upper boundary.
		#[pallet::constant]
		type RewardPriceBand: Get<(Percent, Percent)>;
		/// Maximum number of time buckets used to sample one frame. Zero disables sampling.
		#[pallet::constant]
		type MaxPriceBucketsPerFrame: Get<u32>;
		/// Minimum share of Mainchain ticks with a valid price in each bucket. Even at zero,
		/// each bucket needs a sample to calculate its mean.
		#[pallet::constant]
		type MinPriceBucketCoverage: Get<Percent>;
	}

	/// One start for every retained closed frame and one for the active frame.
	pub struct MaxStabilizerPoolFrameStarts<T>(PhantomData<T>);
	impl<T: Config> Get<u32> for MaxStabilizerPoolFrameStarts<T> {
		fn get() -> u32 {
			T::MaxSampledPriceHistoryFrames::get().saturating_add(1)
		}
	}

	/// Opening block timestamp for each retained mining frame, in Unix milliseconds.
	/// The opening of the next frame is the closing of this one.
	#[pallet::storage]
	pub type StabilizerPoolFrameStartMillis<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<FrameId, Moment, MaxStabilizerPoolFrameStarts<T>>,
		ValueQuery,
	>;

	/// Approved deployments on the Ethereum network authenticated by EthereumVerifier.
	/// Configuration is immutable once registered, preserving checkpoint identity and layout.
	#[pallet::storage]
	pub type Pools<T> =
		StorageDoubleMap<_, Blake2_128Concat, H160, Blake2_128Concat, H256, PoolConfig>;

	/// Current sampling buckets and the latest spot observation awaiting a tick boundary.
	#[pallet::storage]
	pub type StabilizerPoolPriceSamplingState<T: Config> =
		StorageValue<_, StabilizerPoolPriceSampling<T::MaxPriceBucketsPerFrame>, ValueQuery>;

	/// Representative Uniswap price and valuation quotes for each well-sampled closed frame.
	#[pallet::storage]
	pub type StabilizerPoolPricesByFrame<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<FrameId, StabilizerPoolFramePrice, T::MaxSampledPriceHistoryFrames>,
		ValueQuery,
	>;

	/// Last accepted checkpoint for an NFT. Keep its source-block watermark across re-enrollment.
	#[pallet::storage]
	pub type PositionCheckpoints<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		H160,
		Blake2_128Concat,
		UniswapNftId,
		PositionCheckpoint<T::AccountId>,
		OptionQuery,
	>;

	/// Current reward enrollment or the latest authenticated removal for each NFT.
	#[pallet::storage]
	pub type StabilizerPoolRegistrations<T> = StorageMap<
		_,
		Blake2_128Concat,
		StabilizerPoolPositionId,
		StabilizerPoolRegistration,
		OptionQuery,
	>;

	/// Beneficiary slots for enrolled NFTs and reported NFTs held for recovery or collection.
	#[pallet::storage]
	pub type PositionIdsByAccount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<StabilizerPoolPositionId, T::MaxPositionsPerAccount>,
		ValueQuery,
	>;

	/// Per-position continuation history and unresolved public report, sharing one owner.
	#[pallet::storage]
	pub type PositionContinuations<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		StabilizerPoolPositionId,
		PositionContinuation<T::MaxSampledPriceHistoryFrames>,
		ValueQuery,
	>;

	/// Sum of registered raw Uniswap liquidity at each exact pool and range.
	#[pallet::storage]
	pub type RegisteredLiquidityByRange<T> = StorageMap<
		_,
		Blake2_128Concat,
		StabilizerPoolLiquidityRange,
		StabilizerPoolRangeLiquidity,
		ValueQuery,
	>;

	/// This frame's capped liquidity for handoffs whose two different bands must qualify at close.
	#[pallet::storage]
	pub type PendingBandChanges<T> = StorageValue<_, PendingBandLiquidity, ValueQuery>;

	/// Eligible pool value fixed at close, awaiting Treasury funding and rate finalization.
	#[pallet::storage]
	pub type PendingFrameRewardValue<T> = StorageValue<_, ClosedFrameRewardValue, OptionQuery>;

	/// USDC reward share fixed for each retained frame. ARGON receives the remainder.
	#[pallet::storage]
	pub type StabilizerPoolRewardSharesByFrame<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<FrameId, Percent, T::MaxSampledPriceHistoryFrames>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PoolRegistered {
			/// Ethereum PoolManager used to scope the native PoolId.
			pool_manager: H160,
			/// ABI hash of the newly approved pool key.
			pool_id: H256,
			/// Immutable deployment bindings and canonical pool key.
			config: PoolConfig,
		},
		PositionCheckpointed {
			/// Ethereum NFT manager used to scope the token ID.
			position_manager: H160,
			/// NFT identifier within the approved PositionManager.
			token_id: UniswapNftId,
			/// Authenticated position state now stored as the latest checkpoint.
			checkpoint: Box<PositionCheckpoint<T::AccountId>>,
			/// Whether this proof registered, extended, or reset continuity.
			outcome: CheckpointOutcome,
		},
		PositionChangeReported {
			/// Position removed from registered liquidity by authenticated Ethereum state.
			position: StabilizerPoolPositionId,
			/// Ethereum state in which the old revision or subscription had changed.
			observed_at: ProofBlock,
		},
		PositionContinued {
			/// Source NFT retired by this handoff.
			position: StabilizerPoolPositionId,
			/// Fresh NFT inheriting reward history and the account's existing position slot.
			replacement: StabilizerPoolPositionId,
			/// First frame using the replacement's daily reward terms.
			first_eligible_frame_id: FrameId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// A deployment address is zero or reused, or the pool key fails ordering or encoding
		/// bounds.
		InvalidPool,
		/// This PoolManager/PoolId pair already has an immutable registration.
		PoolAlreadyRegistered,
		/// No approved configuration exists for the supplied PoolManager/PoolId pair.
		PoolNotRegistered,
		/// The Ethereum verifier is unavailable, or header ancestry/account/storage authentication
		/// fails.
		InvalidProof,
		/// The three account witnesses do not contain identical execution-block proofs.
		InconsistentProofBlocks,
		/// A witness has the wrong slot count or slot addresses for the requested NFT position.
		InvalidStorageSlots,
		/// The NFT's subscriber or subscription flag is invalid, or its permanent revision is
		/// zero.
		InvalidSubscription,
		/// The signer does not match the beneficiary of the position.
		NotBeneficiary,
		/// The pool or PositionManager identity mismatches, ticks are invalid/misaligned,
		/// or liquidity exceeds uint128.
		InvalidPosition,
		/// Ethereum block height or time does not strictly advance past the stored checkpoint.
		StaleCheckpoint,
		/// The subscriber revision is lower than the NFT's last accepted revision.
		RevisionRegressed,
		/// Beneficiary, subscriber, pool, tick range, or liquidity changed without a revision
		/// increase.
		PositionChangedWithoutRevision,
		/// The enrollment, recoverable report, account slot, or required checkpoint is missing.
		PositionNotEnrolled,
		/// The authenticated state still has the registered revision or subscription.
		PositionChangeNotProven,
		/// The registered position cannot be valued at the frame's sampled pool price.
		ClaimValuationFailed,
		/// The beneficiary already has the maximum number of enrolled NFTs.
		TooManyPositions,
		/// Current market or target quotes are unavailable, or the target is zero.
		PriceUnavailable,
		/// New enrollment or increased liquidity would be admitted below the ARGON target price.
		PriceBelowTarget,
		/// The Ethereum gap exceeds the configured allowance, or continuation is disabled.
		ContinuationGraceExceeded,
		/// The source frame is too old, the replacement proof is in the future, or report recovery
		/// has expired.
		ContinuationTooLate,
		/// Another handoff already consumed a frame touched by this transition.
		ContinuationAlreadyUsed,
		/// The replacement NFT has previous Mainchain checkpoint history.
		PositionAlreadyCheckpointed,
		/// The source NFT's packed position slot is not proven zero at the replacement block.
		PositionNotRetired,
		/// Unpaid history cannot fit within the retained frame bound.
		TooManyPositionPeriods,
		/// The active frame is tracking its maximum number of distinct band changes. A proof
		/// needs no pending entry when the Ethereum handoff frame has already closed.
		TooManyPendingBandChanges,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Approve immutable deployment bindings and the canonical pool key.
		/// Root must verify the pinned implementations and their immutable bindings first.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::register_pool())]
		pub fn register_pool(
			origin: OriginFor<T>,
			pool_manager: H160,
			config: PoolConfig,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(pool_manager != H160::zero(), Error::<T>::InvalidPool);
			ensure!(config.position_manager != H160::zero(), Error::<T>::InvalidPool);
			ensure!(config.subscriber != H160::zero(), Error::<T>::InvalidPool);

			ensure!(pool_manager != config.position_manager, Error::<T>::InvalidPool);
			ensure!(config.subscriber != config.position_manager, Error::<T>::InvalidPool);
			ensure!(config.subscriber != pool_manager, Error::<T>::InvalidPool);

			ensure!(config.key.currency0 < config.key.currency1, Error::<T>::InvalidPool);
			ensure!(config.key.fee <= 0xff_ffff, Error::<T>::InvalidPool);
			ensure!((1..=32_767).contains(&config.key.tick_spacing), Error::<T>::InvalidPool);

			let pool_id = evm::pool_id(&config.key);
			ensure!(
				!Pools::<T>::contains_key(pool_manager, pool_id),
				Error::<T>::PoolAlreadyRegistered
			);

			Pools::<T>::insert(pool_manager, pool_id, config);
			Self::deposit_event(Event::PoolRegistered { pool_manager, pool_id, config });
			Ok(())
		}

		/// Register a position or refresh its checkpoint. A new revision forfeits the old
		/// registration's unclaimed rewards and starts earning at the next frame.
		#[pallet::call_index(1)]
		#[pallet::weight(weights::checkpoint_position::<T>(proof))]
		pub fn checkpoint_position(
			origin: OriginFor<T>,
			pool_manager: H160,
			pool_id: H256,
			token_id: UniswapNftId,
			proof: Box<PositionProof>,
		) -> DispatchResult {
			let beneficiary = ensure_signed(origin)?;
			let (position_manager, checkpoint) =
				Self::verify_position_proof(&beneficiary, pool_manager, pool_id, token_id, &proof)?;
			Self::record_checkpoint(position_manager, token_id, checkpoint)
		}

		/// Any signed account may report a newer revision or ended subscription. Proven rewards
		/// remain collectible until an ordinary checkpoint reset starts a fresh enrollment.
		/// A timely continuation preserves the enrollment and its unpaid reward cursor.
		#[pallet::call_index(2)]
		#[pallet::weight(weights::report_position_change::<T>(proof))]
		pub fn report_position_change(
			origin: OriginFor<T>,
			position: StabilizerPoolPositionId,
			proof: Box<PositionChangeProof>,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let Some(StabilizerPoolRegistration::Enrolled(enrollment)) =
				StabilizerPoolRegistrations::<T>::get(position)
			else {
				return Err(Error::<T>::PositionNotEnrolled.into());
			};
			let previous =
				PositionCheckpoints::<T>::get(position.position_manager, position.uniswap_nft_id)
					.ok_or(Error::<T>::PositionNotEnrolled)?;

			let (account, witness) = evm::decode_position_change::<T>(position, &previous, &proof)?;
			let observed_at = T::EthereumVerifier::verify_account_storage_proof(account, witness)
				.map_err(|_| Error::<T>::InvalidProof)?;
			ensure!(
				observed_at.number > previous.proven_through_block.number,
				Error::<T>::StaleCheckpoint
			);
			ensure!(
				observed_at.timestamp_millis > previous.proven_through_block.timestamp_millis,
				Error::<T>::StaleCheckpoint
			);

			Self::remove_reported_position(position, &previous, enrollment, observed_at)
		}

		/// Continue the enrollment with another NFT from the same Uniswap PositionManager.
		/// The replacement's own reward terms start in the frame after its proven Ethereum block.
		#[pallet::call_index(3)]
		#[pallet::weight(weights::continue_enrollment::<T>(proof))]
		pub fn continue_enrollment(
			origin: OriginFor<T>,
			position: StabilizerPoolPositionId,
			replacement_uniswap_nft_id: UniswapNftId,
			proof: Box<PositionContinuationProof>,
		) -> DispatchResult {
			let beneficiary = ensure_signed(origin)?;

			// A public report holds the original enrollment during its recovery window.
			let mut continuation = PositionContinuations::<T>::get(position);
			let reported = continuation.reported_enrollment;
			let mut enrollment = match StabilizerPoolRegistrations::<T>::get(position) {
				Some(StabilizerPoolRegistration::Enrolled(enrollment)) => enrollment,
				_ => reported.ok_or(Error::<T>::PositionNotEnrolled)?.enrollment,
			};

			let mut previous =
				PositionCheckpoints::<T>::get(position.position_manager, position.uniswap_nft_id)
					.ok_or(Error::<T>::PositionNotEnrolled)?;
			ensure!(beneficiary == previous.beneficiary, Error::<T>::NotBeneficiary);

			ensure!(
				!PositionCheckpoints::<T>::contains_key(
					position.position_manager,
					replacement_uniswap_nft_id
				),
				Error::<T>::PositionAlreadyCheckpointed
			);

			let (source_block, replacement_checkpoint) = Self::verify_continuation_proof(
				&beneficiary,
				position,
				&previous,
				replacement_uniswap_nft_id,
				&proof,
			)?;

			let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
			let replacement_block = replacement_checkpoint.proven_through_block;
			let (source_frame, replacement_frame) =
				Self::continuation_frames(source_block, replacement_block, current_frame)?;
			let replacement_start_frame_id = replacement_frame.saturating_add(1);

			if let Some(report) = reported {
				ensure!(
					current_frame < report.recovery_deadline_frame_id,
					Error::<T>::ContinuationTooLate
				);
			}

			// A handoff across a frame boundary uses the one allowed change in each frame.
			let history = &mut continuation.earning_periods;
			if let Some(last) = history.last() {
				ensure!(
					source_frame >= last.replacement_start_frame_id,
					Error::<T>::ContinuationAlreadyUsed
				);
			}

			// Recovery is allowed through the following frame, so at most one frame closed
			// with this position excluded from the pool total.
			let excluded_frame_id = reported
				.filter(|report| report.reported_frame_id < current_frame)
				.map(|report| report.reported_frame_id);
			let oldest_claim_frame = enrollment.next_claim_frame_id.max(
				current_frame
					.saturating_sub(FrameId::from(T::ClaimWindowFrames::get()).saturating_add(1)),
			);
			history.retain(|period| period.replacement_start_frame_id > oldest_claim_frame);
			history
				.try_push(PositionEarningPeriod {
					retired_position_id: position,
					first_eligible_frame_id: enrollment.first_eligible_frame_id,
					replacement_start_frame_id,
					excluded_frame_id,
				})
				.map_err(|_| Error::<T>::TooManyPositionPeriods)?;

			let replacement =
				StabilizerPoolPositionId { uniswap_nft_id: replacement_uniswap_nft_id, ..position };
			let mut account_positions = PositionIdsByAccount::<T>::get(&beneficiary);
			let account_position_index = account_positions
				.iter()
				.position(|id| *id == position)
				.ok_or(Error::<T>::PositionNotEnrolled)?;

			// A report has already removed the source liquidity from the registration totals.
			let previous_registration = if reported.is_none() {
				Some((position, &previous, enrollment.first_eligible_frame_id))
			} else {
				None
			};
			Self::update_registered_liquidity(
				current_frame,
				Some(&replacement_checkpoint),
				previous_registration,
			)?;

			// During the handoff frame both bands must qualify and the smaller liquidity earns.
			// After the Ethereum handoff frame closes, the replacement earns on its own terms.
			if enrollment.first_eligible_frame_id <= current_frame {
				let basis = if current_frame < replacement_start_frame_id {
					FrameLiquidity::new(&previous, Some(&replacement_checkpoint))
				} else {
					FrameLiquidity::new(&replacement_checkpoint, None)
				};
				Self::change_current_liquidity(current_frame, basis, LiquidityChange::Add)?;
			}

			// Transfer the same account slot and claim cursor to the replacement NFT.
			account_positions[account_position_index] = replacement;
			enrollment.first_eligible_frame_id =
				replacement_start_frame_id.max(enrollment.first_eligible_frame_id);
			PositionIdsByAccount::<T>::insert(beneficiary, account_positions);

			previous.proven_through_block = source_block;
			PositionCheckpoints::<T>::insert(
				position.position_manager,
				position.uniswap_nft_id,
				previous,
			);
			PositionCheckpoints::<T>::insert(
				position.position_manager,
				replacement_uniswap_nft_id,
				replacement_checkpoint,
			);

			StabilizerPoolRegistrations::<T>::insert(
				position,
				StabilizerPoolRegistration::Removed { reported_at_block: replacement_block },
			);
			StabilizerPoolRegistrations::<T>::insert(
				replacement,
				StabilizerPoolRegistration::Enrolled(enrollment),
			);

			continuation.reported_enrollment = None;
			PositionContinuations::<T>::remove(position);
			PositionContinuations::<T>::insert(replacement, continuation);

			Self::deposit_event(Event::PositionContinued {
				position,
				replacement,
				first_eligible_frame_id: enrollment.first_eligible_frame_id,
			});
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn record_checkpoint(
			position_manager: H160,
			token_id: UniswapNftId,
			mut checkpoint: PositionCheckpoint<T::AccountId>,
		) -> DispatchResult {
			let block = checkpoint.proven_through_block;
			let outcome;
			let previous = PositionCheckpoints::<T>::get(position_manager, token_id);
			if let Some(previous) = previous.as_ref() {
				ensure!(
					block.number > previous.proven_through_block.number,
					Error::<T>::StaleCheckpoint
				);
				ensure!(
					block.timestamp_millis > previous.proven_through_block.timestamp_millis,
					Error::<T>::StaleCheckpoint
				);
				ensure!(checkpoint.revision >= previous.revision, Error::<T>::RevisionRegressed);

				if checkpoint.revision == previous.revision {
					ensure!(
						checkpoint.beneficiary == previous.beneficiary,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.subscriber == previous.subscriber,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.pool_manager == previous.pool_manager,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.pool_id == previous.pool_id,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.tick_lower == previous.tick_lower,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.tick_upper == previous.tick_upper,
						Error::<T>::PositionChangedWithoutRevision
					);
					ensure!(
						checkpoint.liquidity == previous.liquidity,
						Error::<T>::PositionChangedWithoutRevision
					);

					checkpoint.maintained_since_block = previous.maintained_since_block;
					outcome = CheckpointOutcome::Maintained { from: previous.proven_through_block };
				} else {
					outcome = CheckpointOutcome::Reset {
						previous_revision: previous.revision,
						unproven_from: previous.proven_through_block,
					};
				}
			} else {
				outcome = CheckpointOutcome::Registered;
			}

			let position_id =
				StabilizerPoolPositionId { position_manager, uniswap_nft_id: token_id };
			let enrollment = match StabilizerPoolRegistrations::<T>::get(position_id) {
				Some(StabilizerPoolRegistration::Enrolled(enrollment)) => Some(enrollment),
				Some(StabilizerPoolRegistration::Removed { reported_at_block }) => {
					// A proof can re-enroll from the same authenticated post-change block.
					ensure!(block.number >= reported_at_block.number, Error::<T>::StaleCheckpoint);
					if block.number == reported_at_block.number {
						ensure!(block == reported_at_block, Error::<T>::StaleCheckpoint);
					}
					ensure!(
						block.timestamp_millis >= reported_at_block.timestamp_millis,
						Error::<T>::StaleCheckpoint
					);
					None
				},
				None => None,
			};
			let mut account_positions = PositionIdsByAccount::<T>::get(&checkpoint.beneficiary);
			if !account_positions.contains(&position_id) {
				account_positions
					.try_push(position_id)
					.map_err(|_| Error::<T>::TooManyPositions)?;
			}
			if enrollment.is_none() || !matches!(outcome, CheckpointOutcome::Maintained { .. }) {
				Self::ensure_liquidity_increase_allowed()?;
				let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
				let first_eligible_frame_id =
					current_frame.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				let previous_registration = previous.as_ref().zip(enrollment);
				Self::update_registered_liquidity(
					current_frame,
					Some(&checkpoint),
					previous_registration.map(|(previous, enrollment)| {
						(position_id, previous, enrollment.first_eligible_frame_id)
					}),
				)?;
				StabilizerPoolRegistrations::<T>::insert(
					position_id,
					StabilizerPoolRegistration::Enrolled(StabilizerPoolEnrollment {
						first_eligible_frame_id,
						next_claim_frame_id: first_eligible_frame_id,
					}),
				);
				PositionContinuations::<T>::remove(position_id);
			}

			if let Some(previous) = previous &&
				previous.beneficiary != checkpoint.beneficiary
			{
				PositionIdsByAccount::<T>::mutate_exists(&previous.beneficiary, |positions| {
					if let Some(ids) = positions {
						ids.retain(|id| *id != position_id);
						if ids.is_empty() {
							*positions = None;
						}
					}
				});
			}
			PositionIdsByAccount::<T>::insert(&checkpoint.beneficiary, account_positions);
			T::TreasuryPositionProvider::ensure_account(&checkpoint.beneficiary);
			PositionCheckpoints::<T>::insert(position_manager, token_id, &checkpoint);
			Self::deposit_event(Event::PositionCheckpointed {
				position_manager,
				token_id,
				checkpoint: Box::new(checkpoint),
				outcome,
			});
			Ok(())
		}

		/// Calculate unpaid finalized rewards using the position's retained daily terms.
		pub(crate) fn calculate_position_rewards(
			checkpoint: &PositionCheckpoint<T::AccountId>,
			enrollment: &StabilizerPoolEnrollment,
			continuation: &PositionContinuation<T::MaxSampledPriceHistoryFrames>,
			current_frame: FrameId,
		) -> Result<(FrameId, Vec<StabilizerPoolFrameClaim>), DispatchError> {
			let next_claim_frame_id = enrollment.next_claim_frame_id;
			let first_eligible_frame_id = enrollment.first_eligible_frame_id;

			let retained_earning_frames =
				FrameId::from(T::ClaimWindowFrames::get()).saturating_add(1);
			let oldest_claimable_frame = current_frame.saturating_sub(retained_earning_frames);
			let first_frame = next_claim_frame_id.max(oldest_claimable_frame);
			let first_unfinalized_frame_id = current_frame.saturating_sub(1);
			if first_frame >= first_unfinalized_frame_id {
				return Ok((next_claim_frame_id, Vec::new()));
			}

			let mut history = Vec::new();
			for period in &continuation.earning_periods {
				let terms = PositionCheckpoints::<T>::get(
					period.retired_position_id.position_manager,
					period.retired_position_id.uniswap_nft_id,
				)
				.ok_or(Error::<T>::PositionNotEnrolled)?;
				history.push((period, terms));
			}
			let config = Pools::<T>::get(checkpoint.pool_manager, checkpoint.pool_id)
				.ok_or(Error::<T>::PoolNotRegistered)?;
			let prices = StabilizerPoolPricesByFrame::<T>::get();
			let frame_starts = StabilizerPoolFrameStartMillis::<T>::get();
			let reward_shares = StabilizerPoolRewardSharesByFrame::<T>::get();
			let mut next_unclaimed = next_claim_frame_id;
			let mut claims = Vec::new();

			let closed = continuation
				.reported_enrollment
				.is_some_and(|report| current_frame >= report.recovery_deadline_frame_id);
			for frame in first_frame..first_unfinalized_frame_id {
				let next_frame = frame.saturating_add(1);
				let (Some(start), Some(end)) =
					(frame_starts.get(&frame), frame_starts.get(&next_frame))
				else {
					next_unclaimed = next_frame;
					continue;
				};
				// Frames from the report onward closed without this position.
				if continuation
					.reported_enrollment
					.is_some_and(|report| frame >= report.reported_frame_id)
				{
					next_unclaimed = next_frame;
					continue;
				}
				if history.iter().any(|(period, _)| period.excluded_frame_id == Some(frame)) {
					next_unclaimed = next_frame;
					continue;
				}
				let mut terms = checkpoint;
				let mut coverage = checkpoint.proven_through_block.timestamp_millis;
				let mut replacement = None;
				if let Some(index) =
					history.iter().position(|(period, _)| frame < period.replacement_start_frame_id)
				{
					let (period, previous) = &history[index];
					if frame < period.first_eligible_frame_id {
						next_unclaimed = next_frame;
						continue;
					}
					terms = previous;
					let successor =
						history.get(index + 1).map(|(_, terms)| terms).unwrap_or(checkpoint);
					coverage = terms
						.proven_through_block
						.timestamp_millis
						.max(successor.maintained_since_block.timestamp_millis);
					if next_frame == period.replacement_start_frame_id {
						coverage = successor.proven_through_block.timestamp_millis;
						replacement = Some(successor);
					}
				} else if frame < first_eligible_frame_id {
					next_unclaimed = next_frame;
					continue
				}
				if *end > coverage {
					if closed {
						next_unclaimed = next_frame;
						continue
					}
					break;
				}
				next_unclaimed = next_frame;
				if *start < terms.maintained_since_block.timestamp_millis {
					continue;
				}

				let Some(price) = prices.get(&frame) else { continue };
				let Some(&usdc_share) = reward_shares.get(&frame) else { continue };
				let Some(rate) = T::RewardProvider::frame_reward_rate(frame) else { continue };
				if checkpoint.pool_manager != price.pool.pool_manager {
					continue;
				}
				if checkpoint.pool_id != price.pool.pool_id {
					continue;
				}
				let Ok(band) = Self::reward_band(price, config.key.tick_spacing) else { continue };
				let basis = FrameLiquidity::new(terms, replacement);
				let earning_liquidity = basis
					.liquidity_for_band(&band)
					.map_err(|_| Error::<T>::ClaimValuationFailed)?;

				let amounts = band
					.principal_for_range(
						earning_liquidity,
						terms.tick_lower,
						terms.tick_upper,
						price.sqrt_price_x96,
					)
					.map_err(|_| Error::<T>::ClaimValuationFailed)?;

				let usdc_in_microgons = price.usdc_in_microgons(amounts.usdc)?;
				let rate = U512::from(rate.into_inner());
				let argon_share = Percent::one().saturating_sub(usdc_share);
				let scale = U512::from(FixedU128::DIV) * Percent::one().deconstruct();
				// Two uint128 factors and a percentage can require 263 bits. Round down
				// only after applying both the financial rate and the asset's share.
				let argon_reward =
					U512::from(amounts.argon) * rate * argon_share.deconstruct() / scale;
				let usdc_reward =
					U512::from(usdc_in_microgons) * rate * usdc_share.deconstruct() / scale;
				let amount = Balance::try_from(argon_reward + usdc_reward)
					.map_err(|_| ArithmeticError::Overflow)?;
				if amount.is_zero() {
					continue;
				}
				claims.push(StabilizerPoolFrameClaim { frame_id: frame, amount });
			}

			Ok((next_unclaimed, claims))
		}

		pub(crate) fn remove_reported_position(
			position: StabilizerPoolPositionId,
			previous: &PositionCheckpoint<T::AccountId>,
			enrollment: StabilizerPoolEnrollment,
			observed_at: ProofBlock,
		) -> DispatchResult {
			let frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
			Self::update_registered_liquidity(
				frame_id,
				None,
				Some((position, previous, enrollment.first_eligible_frame_id)),
			)?;

			StabilizerPoolRegistrations::<T>::insert(
				position,
				StabilizerPoolRegistration::Removed { reported_at_block: observed_at },
			);
			let recovery_deadline_frame_id = StabilizerPoolFrameStartMillis::<T>::get()
				.iter()
				.rev()
				.find_map(|(&frame, &start)| {
					(start <= observed_at.timestamp_millis).then_some(frame.saturating_add(2))
				})
				.unwrap_or_default();
			PositionContinuations::<T>::mutate(position, |continuation| {
				continuation.reported_enrollment = Some(ReportedPositionEnrollment {
					enrollment,
					recovery_deadline_frame_id,
					reported_frame_id: frame_id,
				});
			});
			Self::deposit_event(Event::PositionChangeReported { position, observed_at });
			Ok(())
		}

		pub(crate) fn advance_liquidity_snapshots(
			current_frame: FrameId,
			closed_price: Option<StabilizerPoolFramePrice>,
		) {
			let mut frame_starts = StabilizerPoolFrameStartMillis::<T>::get();
			if frame_starts
				.last_key_value()
				.is_some_and(|(latest, _)| *latest >= current_frame)
			{
				return;
			}

			let oldest_frame =
				current_frame.saturating_sub(T::MaxSampledPriceHistoryFrames::get().into());
			frame_starts.retain(|frame, _| *frame >= oldest_frame);
			frame_starts
				.try_insert(current_frame, T::TimeProvider::now())
				.expect("one start per retained closed frame plus the active frame");
			let closed_frame =
				current_frame.checked_sub(1).filter(|frame| frame_starts.contains_key(frame));
			StabilizerPoolFrameStartMillis::<T>::put(frame_starts);

			// Treasury funds the closed frame later in the same hook sequence. Settle the
			// previous frozen value now that its funding is available.
			if let Some(previous) = PendingFrameRewardValue::<T>::take() &&
				previous.frame_id >= oldest_frame
			{
				T::RewardProvider::finalize_frame_rewards(
					previous.frame_id,
					previous.eligible_value_microgons,
				);
			}
			StabilizerPoolRewardSharesByFrame::<T>::mutate(|history| {
				history.retain(|frame, _| *frame >= oldest_frame);
				if let Some(frame_id) = closed_frame {
					history
						.try_insert(frame_id, T::UsdcRewardShare::get())
						.expect("at most one reward share per retained closed frame");
				}
			});

			let pending = PendingBandChanges::<T>::take();
			if let Some(frame_id) = closed_frame {
				let eligible_value_microgons = closed_price
					.and_then(|price| Self::eligible_frame_value(frame_id, &price, pending).ok())
					.unwrap_or_default();
				PendingFrameRewardValue::<T>::put(ClosedFrameRewardValue {
					frame_id,
					eligible_value_microgons,
				});
			}
		}

		fn eligible_frame_value(
			frame_id: FrameId,
			price: &StabilizerPoolFramePrice,
			pending: PendingBandLiquidity,
		) -> Result<Balance, PositionAmountError> {
			let pool = Pools::<T>::get(price.pool.pool_manager, price.pool.pool_id)
				.ok_or(PositionAmountError::InvalidPrice)?;
			let band = Self::reward_band(price, pool.key.tick_spacing)?;
			let mut qualifying: BTreeMap<_, U256> =
				band.reward_ranges()?.into_iter().map(|ticks| (ticks, U256::zero())).collect();

			// Attribute a handoff to its source range only when both ranges qualify.
			for (change, amount) in pending {
				let range = change.source_range;
				if range.pool_manager != price.pool.pool_manager {
					continue;
				}
				if range.pool_id != price.pool.pool_id {
					continue;
				}
				if !qualifying
					.contains_key(&(change.replacement_lower_tick, change.replacement_upper_tick))
				{
					continue;
				}
				if let Some(capped) =
					qualifying.get_mut(&(range.lower_uniswap_tick, range.upper_uniswap_tick))
				{
					*capped =
						capped.checked_add(amount).ok_or(PositionAmountError::AmountOverflow)?;
				}
			}

			let mut total = PositionAmounts::default();
			for ((lower, upper), capped) in qualifying {
				let range = StabilizerPoolLiquidityRange {
					pool_manager: price.pool.pool_manager,
					pool_id: price.pool.pool_id,
					lower_uniswap_tick: lower,
					upper_uniswap_tick: upper,
				};
				let registered = RegisteredLiquidityByRange::<T>::get(range)
					.liquidity_for_frame(frame_id)
					.unwrap_or_default();
				let liquidity =
					registered.checked_add(capped).ok_or(PositionAmountError::AmountOverflow)?;
				if liquidity.is_zero() {
					continue;
				}
				let principal =
					band.principal_for_range(liquidity, lower, upper, price.sqrt_price_x96)?;
				total.argon = total
					.argon
					.checked_add(principal.argon)
					.ok_or(PositionAmountError::AmountOverflow)?;
				total.usdc = total
					.usdc
					.checked_add(principal.usdc)
					.ok_or(PositionAmountError::AmountOverflow)?;
			}
			total
				.argon
				.checked_add(
					price
						.usdc_in_microgons(total.usdc)
						.map_err(|_| PositionAmountError::AmountOverflow)?,
				)
				.ok_or(PositionAmountError::AmountOverflow)
		}

		fn reward_band(
			price: &StabilizerPoolFramePrice,
			uniswap_tick_spacing: i32,
		) -> Result<StabilizerPoolRewardBand, PositionAmountError> {
			StabilizerPoolRewardBand::new(
				price.argon_usd_target_price,
				price.usdc_usd_price,
				price.argon_is_token0,
				uniswap_tick_spacing,
				T::RewardPriceBand::get(),
				REWARD_BAND_TOLERANCE,
			)
		}

		fn update_registered_liquidity(
			frame_id: FrameId,
			replacement: Option<&PositionCheckpoint<T::AccountId>>,
			previous_registration: Option<(
				StabilizerPoolPositionId,
				&PositionCheckpoint<T::AccountId>,
				FrameId,
			)>,
		) -> DispatchResult {
			if let Some((position, previous, first_eligible_frame_id)) = previous_registration &&
				let Some(basis) = Self::registered_basis(
					position,
					previous,
					first_eligible_frame_id,
					frame_id,
				)? {
				Self::change_current_liquidity(frame_id, basis, LiquidityChange::Remove)?;
			}
			let previous = previous_registration.map(|(_, checkpoint, _)| checkpoint);
			for (checkpoint, change) in
				[(previous, LiquidityChange::Remove), (replacement, LiquidityChange::Add)]
			{
				let Some(checkpoint) = checkpoint else { continue };
				let range = StabilizerPoolLiquidityRange::from(checkpoint);
				let mut liquidity = RegisteredLiquidityByRange::<T>::get(range);
				liquidity.advance_to(frame_id);
				liquidity.next_frame_liquidity =
					change.apply(liquidity.next_frame_liquidity, checkpoint.liquidity.into())?;
				Self::store_registered_liquidity(range, liquidity);
			}
			Ok(())
		}

		/// Resolve the open frame's contribution before removing or replacing the enrollment.
		fn registered_basis(
			position: StabilizerPoolPositionId,
			checkpoint: &PositionCheckpoint<T::AccountId>,
			first_eligible_frame_id: FrameId,
			frame: FrameId,
		) -> Result<Option<FrameLiquidity>, DispatchError> {
			let continuation = PositionContinuations::<T>::get(position);
			if let Some(period) = continuation.earning_periods.last() &&
				frame.checked_add(1) == Some(period.replacement_start_frame_id)
			{
				if frame < period.first_eligible_frame_id {
					return Ok(None);
				}
				let retired = period.retired_position_id;
				let old =
					PositionCheckpoints::<T>::get(retired.position_manager, retired.uniswap_nft_id)
						.ok_or(Error::<T>::PositionNotEnrolled)?;
				return Ok(Some(FrameLiquidity::new(&old, Some(checkpoint))));
			}
			Ok((frame >= first_eligible_frame_id).then(|| FrameLiquidity::new(checkpoint, None)))
		}

		fn change_current_liquidity(
			frame: FrameId,
			basis: FrameLiquidity,
			change: LiquidityChange,
		) -> DispatchResult {
			if basis.amount.is_zero() {
				return Ok(());
			}
			if let Some(ticks) = basis.replacement_ticks {
				return PendingBandChanges::<T>::try_mutate(|pending| -> DispatchResult {
					let key = PendingBandChange::new(basis.range, ticks);
					let total = pending.get(&key).copied().unwrap_or_default();
					let updated = change.apply(total, basis.amount)?;
					if updated.is_zero() {
						pending.remove(&key);
					} else {
						pending
							.try_insert(key, updated)
							.map_err(|_| Error::<T>::TooManyPendingBandChanges)?;
					}
					Ok(())
				});
			}
			let mut liquidity = RegisteredLiquidityByRange::<T>::get(basis.range);
			liquidity.advance_to(frame);
			liquidity.current_liquidity =
				change.apply(liquidity.current_liquidity, basis.amount)?;
			Self::store_registered_liquidity(basis.range, liquidity);
			Ok(())
		}

		fn ensure_liquidity_increase_allowed() -> DispatchResult {
			let market = T::PriceProvider::get_latest_argon_price_in_usd()
				.ok_or(Error::<T>::PriceUnavailable)?;
			let target = T::PriceProvider::get_target_argon_price_in_usd()
				.ok_or(Error::<T>::PriceUnavailable)?;
			ensure!(!target.is_zero(), Error::<T>::PriceUnavailable);
			ensure!(market >= target, Error::<T>::PriceBelowTarget);
			Ok(())
		}

		/// Prove the source was enrolled at the first block, then burned by the replacement block.
		fn verify_continuation_proof(
			beneficiary: &T::AccountId,
			position: StabilizerPoolPositionId,
			previous: &PositionCheckpoint<T::AccountId>,
			replacement_uniswap_nft_id: UniswapNftId,
			proof: &PositionContinuationProof,
		) -> Result<(ProofBlock, PositionCheckpoint<T::AccountId>), DispatchError> {
			let pool = (previous.pool_manager, previous.pool_id);
			let config = Pools::<T>::get(previous.pool_manager, previous.pool_id)
				.ok_or(Error::<T>::PoolNotRegistered)?;
			ensure!(
				config.position_manager == position.position_manager,
				Error::<T>::InvalidPosition
			);

			let source_proof = &proof.before;
			let mut before = Self::verify_position_at_block(
				beneficiary,
				pool,
				position.uniswap_nft_id,
				&config,
				&source_proof.execution_block_proof,
				[
					(&source_proof.subscriber).into(),
					(&source_proof.position_manager).into(),
					(&source_proof.pool_manager).into(),
				],
				None,
			)?;

			ensure!(
				before.proven_through_block.number >= previous.proven_through_block.number,
				Error::<T>::StaleCheckpoint
			);
			ensure!(
				before.proven_through_block.timestamp_millis >=
					previous.proven_through_block.timestamp_millis,
				Error::<T>::StaleCheckpoint
			);

			// Compare the terms independently of the blocks at which they were observed.
			let source_block = before.proven_through_block;
			before.maintained_since_block = previous.maintained_since_block;
			before.proven_through_block = previous.proven_through_block;
			ensure!(&before == previous, Error::<T>::PositionChangedWithoutRevision);

			// Both the retirement and the live replacement must be proven at the second block.
			let replacement_proof = &proof.after;
			let [slot] = replacement_proof.retired.slots.as_slice() else {
				return Err(Error::<T>::InvalidStorageSlots.into())
			};
			let position_info_slot = evm::mapping_slot(
				position.uniswap_nft_id,
				evm::POSITION_MANAGER_POSITION_INFO_SLOT,
			);
			ensure!(slot.slot == position_info_slot, Error::<T>::InvalidStorageSlots);
			ensure!(slot.value == H256::zero(), Error::<T>::PositionNotRetired);
			let after = Self::verify_position_at_block(
				beneficiary,
				pool,
				replacement_uniswap_nft_id,
				&config,
				&replacement_proof.execution_block_proof,
				[
					(&replacement_proof.subscriber).into(),
					(&replacement_proof.position_manager).into(),
					(&replacement_proof.pool_manager).into(),
				],
				Some((&replacement_proof.retired).into()),
			)?;
			if after.liquidity > previous.liquidity {
				Self::ensure_liquidity_increase_allowed()?;
			}

			Ok((source_block, after))
		}

		/// Check the Ethereum grace period and return the Argon frame containing each proof.
		fn continuation_frames(
			source_block: ProofBlock,
			replacement_block: ProofBlock,
			current_frame: FrameId,
		) -> Result<(FrameId, FrameId), DispatchError> {
			ensure!(replacement_block.number > source_block.number, Error::<T>::StaleCheckpoint);
			let gap_millis = replacement_block
				.timestamp_millis
				.checked_sub(source_block.timestamp_millis)
				.ok_or(Error::<T>::StaleCheckpoint)?;
			ensure!(gap_millis > 0, Error::<T>::StaleCheckpoint);
			ensure!(
				gap_millis <= T::ContinuationGraceMillis::get(),
				Error::<T>::ContinuationGraceExceeded
			);
			ensure!(
				replacement_block.timestamp_millis <= T::TimeProvider::now(),
				Error::<T>::ContinuationTooLate
			);

			// Ethereum timestamps determine which Argon frames the handoff touches.
			let starts = StabilizerPoolFrameStartMillis::<T>::get();
			let frame_at = |time| {
				starts
					.iter()
					.rev()
					.find_map(|(&frame, &start)| (start <= time).then_some(frame))
			};
			let source_frame =
				frame_at(source_block.timestamp_millis).ok_or(Error::<T>::ContinuationTooLate)?;
			ensure!(
				source_frame >= current_frame.saturating_sub(1),
				Error::<T>::ContinuationTooLate
			);
			let replacement_frame = frame_at(replacement_block.timestamp_millis)
				.ok_or(Error::<T>::ContinuationTooLate)?;

			Ok((source_frame, replacement_frame))
		}

		fn verify_position_proof(
			beneficiary: &T::AccountId,
			pool_manager: H160,
			pool_id: H256,
			token_id: UniswapNftId,
			proof: &PositionProof,
		) -> Result<(H160, PositionCheckpoint<T::AccountId>), DispatchError> {
			let config =
				Pools::<T>::get(pool_manager, pool_id).ok_or(Error::<T>::PoolNotRegistered)?;
			let execution_proof = &proof.subscriber.execution_block_proof;
			ensure!(
				execution_proof == &proof.position_manager.execution_block_proof,
				Error::<T>::InconsistentProofBlocks
			);
			ensure!(
				execution_proof == &proof.pool_manager.execution_block_proof,
				Error::<T>::InconsistentProofBlocks
			);

			let checkpoint = Self::verify_position_at_block(
				beneficiary,
				(pool_manager, pool_id),
				token_id,
				&config,
				execution_proof,
				[
					(&proof.subscriber).into(),
					(&proof.position_manager).into(),
					(&proof.pool_manager).into(),
				],
				None,
			)?;
			Ok((config.position_manager, checkpoint))
		}

		fn verify_position_at_block<'a>(
			beneficiary: &T::AccountId,
			(pool_manager, pool_id): (H160, H256),
			token_id: UniswapNftId,
			config: &PoolConfig,
			execution_proof: &EthereumExecutionBlockProof,
			[subscriber, manager, liquidity]: [EthereumAccountStorageWitnessRef<'a>; 3],
			retired: Option<EthereumAccountStorageWitnessRef<'a>>,
		) -> Result<PositionCheckpoint<T::AccountId>, DispatchError> {
			let position = evm::decode_position::<T>(
				config,
				pool_id,
				token_id,
				subscriber.slots,
				manager.slots,
				liquidity.slots,
			)?;
			ensure!(
				beneficiary.encode().as_slice() == position.argon_account_id.as_bytes(),
				Error::<T>::NotBeneficiary
			);

			let accounts = [
				(config.subscriber, subscriber),
				(config.position_manager, manager),
				(pool_manager, liquidity),
			];
			let retirement = retired.map(|witness| (config.position_manager, witness));
			let block = T::EthereumVerifier::verify_storage_at_block(
				execution_proof,
				accounts.into_iter().chain(retirement),
			)
			.map_err(|_| Error::<T>::InvalidProof)?;

			Ok(PositionCheckpoint {
				beneficiary: beneficiary.clone(),
				pool_manager,
				pool_id,
				subscriber: config.subscriber,
				revision: position.revision,
				tick_lower: position.tick_lower,
				tick_upper: position.tick_upper,
				liquidity: position.liquidity,
				maintained_since_block: block,
				proven_through_block: block,
			})
		}

		fn store_registered_liquidity(
			range: StabilizerPoolLiquidityRange,
			liquidity: StabilizerPoolRangeLiquidity,
		) {
			if liquidity.current_liquidity.is_zero() && liquidity.next_frame_liquidity.is_zero() {
				RegisteredLiquidityByRange::<T>::remove(range);
			} else {
				RegisteredLiquidityByRange::<T>::insert(range, liquidity);
			}
		}
	}

	impl<T: Config> OnNewSlot<T::AccountId> for Pallet<T> {
		type Key = BlockSealAuthorityId;

		fn on_frame_start(frame_id: FrameId) {
			let closed_price =
				Self::start_stabilizer_pool_price_sampling(frame_id, T::CurrentTick::get());
			Self::advance_liquidity_snapshots(frame_id, closed_price);
		}

		fn on_frame_start_weight(_: FrameId) -> Weight {
			T::WeightInfo::start_stabilizer_pool_price_sampling()
				.saturating_add(liquidity_snapshot_weight::<T>())
		}
	}

	impl<T: Config> StabilizerPoolPositionProvider<T::AccountId> for Pallet<T> {
		type Weights = weights::PositionProviderWeights<T>;
		fn settle_account(account: &T::AccountId) -> Result<Balance, DispatchError> {
			let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
			let mut claims_by_frame = BTreeMap::<FrameId, Balance>::new();
			for position in PositionIdsByAccount::<T>::get(account) {
				let mut continuation = PositionContinuations::<T>::get(position);
				let reported = continuation.reported_enrollment;
				let mut enrollment = match StabilizerPoolRegistrations::<T>::get(position) {
					Some(StabilizerPoolRegistration::Enrolled(enrollment)) => enrollment,
					_ => {
						let Some(report) = reported else { continue };
						report.enrollment
					},
				};
				let checkpoint = PositionCheckpoints::<T>::get(
					position.position_manager,
					position.uniswap_nft_id,
				)
				.ok_or(Error::<T>::PositionNotEnrolled)?;
				ensure!(&checkpoint.beneficiary == account, Error::<T>::NotBeneficiary);

				let closed = reported
					.is_some_and(|report| current_frame >= report.recovery_deadline_frame_id);
				let (next_claim_frame_id, claims) = Self::calculate_position_rewards(
					&checkpoint,
					&enrollment,
					&continuation,
					current_frame,
				)?;
				for claim in claims {
					let amount = claims_by_frame.entry(claim.frame_id).or_default();
					*amount = amount.checked_add(claim.amount).ok_or(ArithmeticError::Overflow)?;
				}
				if next_claim_frame_id > enrollment.next_claim_frame_id {
					enrollment.next_claim_frame_id = next_claim_frame_id;
					if let Some(mut report) = reported {
						report.enrollment = enrollment;
						continuation.reported_enrollment = Some(report);
						PositionContinuations::<T>::insert(position, continuation);
					} else {
						StabilizerPoolRegistrations::<T>::insert(
							position,
							StabilizerPoolRegistration::Enrolled(enrollment),
						);
					}
				}
				if closed {
					PositionContinuations::<T>::remove(position);
					PositionIdsByAccount::<T>::mutate_exists(account, |stored| {
						if let Some(ids) = stored {
							ids.retain(|id| *id != position);
							if ids.is_empty() {
								*stored = None;
							}
						}
					});
				}
			}

			if claims_by_frame.is_empty() {
				return Ok(0)
			}
			let claims: Vec<_> = claims_by_frame
				.into_iter()
				.map(|(frame_id, amount)| StabilizerPoolFrameClaim { frame_id, amount })
				.collect();
			T::RewardProvider::claim_rewards(&claims)
		}
	}
}

enum LiquidityChange {
	Add,
	Remove,
}

impl LiquidityChange {
	fn apply(&self, total: U256, amount: U256) -> Result<U256, ArithmeticError> {
		match self {
			Self::Add => total.checked_add(amount).ok_or(ArithmeticError::Overflow),
			Self::Remove => total.checked_sub(amount).ok_or(ArithmeticError::Underflow),
		}
	}
}

type TickRange = (UniswapTick, UniswapTick);
type PendingBandLiquidity = BoundedBTreeMap<PendingBandChange, U256, ConstU32<MAX_REWARD_RANGES>>;

/// Identifies handoffs whose liquidity depends on the same source and replacement ranges.
#[derive(
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PendingBandChange {
	/// Original pool and tick range used to value the handoff.
	pub source_range: StabilizerPoolLiquidityRange,
	/// Inclusive lower tick of the replacement in the source pool.
	pub replacement_lower_tick: UniswapTick,
	/// Exclusive upper tick of the replacement in the source pool.
	pub replacement_upper_tick: UniswapTick,
}

impl PendingBandChange {
	fn new(
		source_range: StabilizerPoolLiquidityRange,
		(replacement_lower_tick, replacement_upper_tick): TickRange,
	) -> Self {
		Self { source_range, replacement_lower_tick, replacement_upper_tick }
	}
}

/// Raw earning liquidity valued under the original range, conditional on a changed band.
struct FrameLiquidity {
	range: StabilizerPoolLiquidityRange,
	replacement_ticks: Option<TickRange>,
	amount: U256,
}

impl FrameLiquidity {
	fn new<AccountId>(
		old: &PositionCheckpoint<AccountId>,
		replacement: Option<&PositionCheckpoint<AccountId>>,
	) -> Self {
		let range = StabilizerPoolLiquidityRange::from(old);
		let mut amount = old.liquidity;
		let mut replacement_ticks = None;
		if let Some(new) = replacement {
			amount = amount.min(new.liquidity);
			let ticks = (new.tick_lower, new.tick_upper);
			if ticks != (old.tick_lower, old.tick_upper) {
				replacement_ticks = Some(ticks);
			}
		}
		Self { range, replacement_ticks, amount: amount.into() }
	}

	fn liquidity_for_band(
		&self,
		band: &StabilizerPoolRewardBand,
	) -> Result<U256, PositionAmountError> {
		if let Some((lower, upper)) = self.replacement_ticks &&
			band.qualifying_range(lower, upper)?.is_none()
		{
			return Ok(U256::zero());
		}
		Ok(self.amount)
	}
}

/// The closed frame's eligible principal, fixed before Treasury computes its reward rate.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub struct ClosedFrameRewardValue {
	/// Mining frame that earned the reward.
	pub frame_id: FrameId,
	/// Combined ARGON and USDC principal valued in microgons at the closing frame price.
	pub eligible_value_microgons: Balance,
}

/// How the latest authenticated state affects this NFT's rewards.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum StabilizerPoolRegistration {
	Enrolled(StabilizerPoolEnrollment),
	Removed {
		/// Ethereum block proving the reported change or the completed NFT handoff. A replacement
		/// cannot use an earlier block.
		reported_at_block: ProofBlock,
	},
}

/// Reward timing and claim progress for the NFT's currently enrolled revision.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct StabilizerPoolEnrollment {
	/// First frame in which this revision contributes liquidity.
	pub first_eligible_frame_id: FrameId,
	/// Earliest frame this enrollment can still submit a claim for.
	pub next_claim_frame_id: FrameId,
}

/// Uniswap v4's canonical pool key. The native PoolId is its ABI-encoded keccak256 hash.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PoolKey {
	/// Lower-sorted Ethereum currency address; zero represents native ETH.
	pub currency0: H160,
	/// Higher-sorted Ethereum currency address.
	pub currency1: H160,
	/// Uniswap's uint24 fee encoding, including its dynamic-fee flag.
	pub fee: u32,
	/// Positive tick interval to which position boundaries must align.
	pub tick_spacing: i32,
	/// Pool hook contract, or zero for a pool without hooks.
	pub hooks: H160,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PoolConfig {
	/// Approved Uniswap NFT manager whose token IDs identify checkpointed positions.
	pub position_manager: H160,
	/// Approved subscriber holding each NFT's Argon beneficiary and permanent revision.
	pub subscriber: H160,
	/// Canonical key whose ABI hash identifies this pool within its PoolManager.
	pub key: PoolKey,
}

/// All three account witnesses must prove the same execution block and state root.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct PositionProof {
	/// Subscriber account witness for the NFT's beneficiary and revision slots.
	pub subscriber: EthereumAccountStorageProof<ConstU32<2>>,
	/// NFT manager witness for canonical subscription membership and packed position metadata.
	pub position_manager: EthereumAccountStorageProof<ConstU32<2>>,
	/// PoolManager witness for the NFT's native Uniswap liquidity value.
	pub pool_manager: EthereumAccountStorageProof<ConstU32<1>>,
}

/// Evidence for the two Ethereum blocks bounding a rollover.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct PositionContinuationProof {
	/// Enrolled source NFT at the start of the grace period.
	pub before: ContinuationSourceProof,
	/// Live replacement and burned source at the end of the grace period.
	pub after: ContinuationReplacementProof,
}

/// Source position at block A.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct ContinuationSourceProof {
	/// Authenticates this snapshot directly or through ancestry to a retained anchor.
	pub execution_block_proof: EthereumExecutionBlockProof,
	/// Subscriber slots identifying the beneficiary and revision.
	pub subscriber: EthereumAccountStorageWitness<ConstU32<2>>,
	/// NFT manager slots for subscription membership and packed position metadata.
	pub position_manager: EthereumAccountStorageWitness<ConstU32<2>>,
	/// PoolManager slot proving the NFT's liquidity.
	pub pool_manager: EthereumAccountStorageWitness<ConstU32<1>>,
}

/// Replacement position and source retirement at block B.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct ContinuationReplacementProof {
	/// Authenticates this snapshot directly or through ancestry to a retained anchor.
	pub execution_block_proof: EthereumExecutionBlockProof,
	/// Subscriber slots identifying the beneficiary and revision.
	pub subscriber: EthereumAccountStorageWitness<ConstU32<2>>,
	/// NFT manager slots for subscription membership and packed position metadata.
	pub position_manager: EthereumAccountStorageWitness<ConstU32<2>>,
	/// PoolManager slot proving the NFT's liquidity.
	pub pool_manager: EthereumAccountStorageWitness<ConstU32<1>>,
	/// Source NFT's zero positionInfo slot in the NFT manager at this same block.
	pub retired: EthereumAccountStorageWitness<ConstU32<1>>,
}

/// Continuation state owned by the currently enrolled NFT, including a pending report.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, DefaultNoBound)]
#[scale_info(skip_type_params(MaxFrames))]
pub struct PositionContinuation<MaxFrames: Get<u32>> {
	/// Earlier positions whose unpaid frames share the current enrollment's cursor.
	pub earning_periods: BoundedVec<PositionEarningPeriod, MaxFrames>,
	/// Original enrollment held while the first public report remains recoverable.
	pub reported_enrollment: Option<ReportedPositionEnrollment>,
}

/// Daily reward terms retained after an NFT handoff.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct PositionEarningPeriod {
	/// Retired NFT whose immutable terms remain in PositionCheckpoints.
	pub retired_position_id: StabilizerPoolPositionId,
	/// First frame in which the retired NFT contributed liquidity.
	pub first_eligible_frame_id: FrameId,
	/// Exclusive end of the old daily terms; the preceding frame uses the minimum old/new
	/// liquidity.
	pub replacement_start_frame_id: FrameId,
	/// Frame that closed after a public report and before this enrollment recovered.
	pub excluded_frame_id: Option<FrameId>,
}

/// Original cursor held until a report's source-time recovery window closes.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct ReportedPositionEnrollment {
	/// Eligibility and claim cursor from before the first report.
	pub enrollment: StabilizerPoolEnrollment,
	/// Exclusive recovery frame, derived from the first report's Ethereum observation.
	pub recovery_deadline_frame_id: FrameId,
	/// Argon frame in which the public report removed the position from the pool total.
	pub reported_frame_id: FrameId,
}

/// One Ethereum storage witness showing why the old checkpoint no longer qualifies.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub enum PositionChangeProof {
	/// The enrolled subscriber's permanent revision increased.
	Revision(EthereumAccountStorageProof<ConstU32<1>>),
	/// PositionManager's canonical subscriber no longer matches the enrolled subscriber.
	Subscription(EthereumAccountStorageProof<ConstU32<1>>),
}

/// The NFT that owns a Uniswap position on the Ethereum chain authenticated by the pallet.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct StabilizerPoolPositionId {
	/// Address of the Uniswap PositionManager that issued the NFT.
	pub position_manager: H160,
	/// NFT token ID within that PositionManager.
	pub uniswap_nft_id: UniswapNftId,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct PositionCheckpoint<AccountId> {
	/// Argon account encoded in the subscription and authorizing this checkpoint.
	pub beneficiary: AccountId,
	/// Ethereum contract owning the pool's underlying Uniswap state.
	pub pool_manager: H160,
	/// Full native PoolId: the ABI hash of the approved canonical pool key.
	pub pool_id: H256,
	/// Subscriber proven to be attached to the NFT at the checkpoint block.
	pub subscriber: H160,
	/// Permanent subscriber counter; any increase starts a new continuity baseline.
	pub revision: U256,
	/// Inclusive lower Uniswap price tick of the NFT's position.
	pub tick_lower: UniswapTick,
	/// Exclusive upper Uniswap price tick of the NFT's position.
	pub tick_upper: UniswapTick,
	/// Native Uniswap liquidity L, not a token balance or reward amount.
	pub liquidity: u128,
	/// Start of the continuously maintained revision proven by this record.
	pub maintained_since_block: ProofBlock,
	/// Latest accepted Ethereum endpoint; proves nothing about later blocks.
	pub proven_through_block: ProofBlock,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub enum CheckpointOutcome {
	/// The first proof establishes a baseline without proving earlier activity.
	Registered,
	Maintained {
		/// Previous verified endpoint from which the unchanged revision is extended.
		from: ProofBlock,
	},
	Reset {
		/// Revision replaced by the new baseline.
		previous_revision: U256,
		/// Last verified endpoint before the unproven gap.
		unproven_from: ProofBlock,
	},
}

#[derive(
	Clone,
	Copy,
	Debug,
	PartialEq,
	PartialOrd,
	Eq,
	Ord,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct StabilizerPoolLiquidityRange {
	/// Ethereum PoolManager authenticated by this runtime's Ethereum verifier.
	pub pool_manager: H160,
	/// Canonical pool ID within the manager.
	pub pool_id: H256,
	/// Inclusive lower Uniswap tick of the registered range.
	pub lower_uniswap_tick: UniswapTick,
	/// Exclusive upper Uniswap tick of the registered range.
	pub upper_uniswap_tick: UniswapTick,
}

#[derive(
	Clone,
	Copy,
	Debug,
	Default,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct StabilizerPoolRangeLiquidity {
	/// Frame at the last update of this range.
	pub frame_id: FrameId,
	/// Raw Uniswap liquidity eligible in `frame_id`, summed across registered NFTs.
	pub current_liquidity: U256,
	/// Raw liquidity eligible from `frame_id + 1` if the registrations stay the same.
	pub next_frame_liquidity: U256,
}

impl<AccountId> From<&PositionCheckpoint<AccountId>> for StabilizerPoolLiquidityRange {
	fn from(checkpoint: &PositionCheckpoint<AccountId>) -> Self {
		Self {
			pool_manager: checkpoint.pool_manager,
			pool_id: checkpoint.pool_id,
			lower_uniswap_tick: checkpoint.tick_lower,
			upper_uniswap_tick: checkpoint.tick_upper,
		}
	}
}

impl StabilizerPoolRangeLiquidity {
	/// Return the registered liquidity for this frame or a later frame with no further changes.
	/// Earlier frames require a snapshot from before the range was updated.
	pub fn liquidity_for_frame(&self, frame_id: FrameId) -> Option<U256> {
		if frame_id < self.frame_id {
			return None;
		}
		if frame_id == self.frame_id {
			return Some(self.current_liquidity);
		}
		Some(self.next_frame_liquidity)
	}

	fn advance_to(&mut self, frame_id: FrameId) {
		if frame_id > self.frame_id {
			self.current_liquidity = self.next_frame_liquidity;
			self.frame_id = frame_id;
		}
	}
}
