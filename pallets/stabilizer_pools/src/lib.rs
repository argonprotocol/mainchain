#![cfg_attr(not(feature = "std"), no_std)]
//! Claimant-submitted checkpoints for maintained Uniswap v4 positions.
//!
//! Checkpoints establish continuity and maintain registered liquidity by Uniswap range.

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
pub use argon_primitives::EthereumProofBlock as ProofBlock;
use argon_primitives::{
	block_seal::FrameId,
	treasury::{StabilizerPoolClaimProvider, StabilizerPoolFrameClaim},
	BlockSealAuthorityId, EthereumAccountStorageProof, EthereumVerifyProvider,
	MiningFrameTransitionProvider, OnNewSlot, UniswapTick,
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
mod benchmarking;
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
		/// Authenticates Ethereum state and identifies the block proved by each witness.
		type EthereumVerifier: EthereumVerifyProvider;
		/// Accepts the principal proved for newly claimed frames.
		type ClaimProvider: StabilizerPoolClaimProvider<Self::AccountId>;
		/// Supplies the mining frame active when a checkpoint is accepted.
		type MiningFrameTransitionProvider: MiningFrameTransitionProvider;
		/// Mainchain clock used to assign observations to frames and reject future prices.
		type CurrentTick: Get<Tick>;
		/// Unix time of the block that activates a mining frame.
		type TimeProvider: Time<Moment = u64>;
		/// Closed mining frames retained in the sampled Uniswap price history.
		#[pallet::constant]
		type MaxSampledPriceHistoryFrames: Get<u32>;
		/// Number of closed frames for which an Ethereum proof can establish a new claim.
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
	pub type StabilizerPoolFrameStartMillis<T: Config> =
		StorageValue<_, BoundedBTreeMap<FrameId, u64, MaxStabilizerPoolFrameStarts<T>>, ValueQuery>;

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
		U256,
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

	/// Sum of registered raw Uniswap liquidity at each exact pool and range.
	#[pallet::storage]
	pub type RegisteredLiquidityByRange<T> = StorageMap<
		_,
		Blake2_128Concat,
		StabilizerPoolLiquidityRange,
		StabilizerPoolRangeLiquidity,
		ValueQuery,
	>;

	/// Raw liquidity for the most recently closed frame, while public corrections are allowed.
	#[pallet::storage]
	pub type StabilizerPoolReviewFrame<T> =
		StorageValue<_, StabilizerPoolFrameLiquidity, OptionQuery>;

	/// Corrected ARGON and USDC principal for each retained earning frame.
	#[pallet::storage]
	pub type FinalizedStabilizerPoolPrincipalByFrame<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<FrameId, PositionAmounts<U256>, T::MaxSampledPriceHistoryFrames>,
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
			token_id: U256,
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
		/// The signer does not match the beneficiary encoded in the subscription witness.
		NotBeneficiary,
		/// The packed PoolId mismatches, ticks are invalid/misaligned, or liquidity exceeds
		/// uint128.
		InvalidPosition,
		/// Ethereum block height or time does not strictly advance past the stored checkpoint.
		StaleCheckpoint,
		/// The subscriber revision is lower than the NFT's last accepted revision.
		RevisionRegressed,
		/// Beneficiary, subscriber, pool, tick range, or liquidity changed without a revision
		/// increase.
		PositionChangedWithoutRevision,
		/// The NFT has no registration to remove.
		PositionNotEnrolled,
		/// The authenticated state still has the registered revision or subscription.
		PositionChangeNotProven,
		/// An opening timestamp is missing for the reviewed frame or its successor.
		FrameStartTimeUnavailable,
		/// The verified position cannot be valued at the frame's sampled pool price.
		ClaimValuationFailed,
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

		/// Register a baseline, submit claims for newly proved full frames of the maintained
		/// revision, or reset after an unproven change.
		#[pallet::call_index(1)]
		#[pallet::weight(weights::checkpoint_position::<T>(proof))]
		pub fn checkpoint_position(
			origin: OriginFor<T>,
			pool_manager: H160,
			pool_id: H256,
			token_id: U256,
			proof: Box<PositionProof>,
		) -> DispatchResult {
			let beneficiary = ensure_signed(origin)?;
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

			// Decode only the expected slots; callers cannot choose which position the witness
			// describes.
			let position = evm::decode_position::<T>(&config, pool_id, token_id, &proof)?;
			ensure!(
				beneficiary.encode().as_slice() == position.argon_account_id.as_bytes(),
				Error::<T>::NotBeneficiary
			);

			let block = T::EthereumVerifier::verify_account_storage_proof(
				config.subscriber,
				&proof.subscriber,
			)
			.map_err(|_| Error::<T>::InvalidProof)?;
			T::EthereumVerifier::verify_account_storage_proof(
				config.position_manager,
				&proof.position_manager,
			)
			.map_err(|_| Error::<T>::InvalidProof)?;
			T::EthereumVerifier::verify_account_storage_proof(pool_manager, &proof.pool_manager)
				.map_err(|_| Error::<T>::InvalidProof)?;

			let checkpoint = PositionCheckpoint {
				beneficiary,
				pool_manager,
				pool_id,
				subscriber: config.subscriber,
				revision: position.revision,
				tick_lower: position.tick_lower,
				tick_upper: position.tick_upper,
				liquidity: position.liquidity,
				maintained_since: block,
				proven_through: block,
			};
			Self::record_checkpoint(config.position_manager, token_id, checkpoint)
		}

		/// Remove reward participation using proof of a newer revision or an ended subscription.
		/// Any signed account can report a change. During review, only unproved participation
		/// is removed from the closed frame.
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
				PositionCheckpoints::<T>::get(position.position_manager, position.token_id)
					.ok_or(Error::<T>::PositionNotEnrolled)?;

			let (account, witness) = evm::decode_position_change::<T>(position, &previous, &proof)?;
			let observed_at = T::EthereumVerifier::verify_account_storage_proof(account, witness)
				.map_err(|_| Error::<T>::InvalidProof)?;
			ensure!(
				observed_at.number > previous.proven_through.number,
				Error::<T>::StaleCheckpoint
			);
			ensure!(
				observed_at.timestamp_millis > previous.proven_through.timestamp_millis,
				Error::<T>::StaleCheckpoint
			);

			Self::remove_reported_position(position, &previous, enrollment, observed_at)
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn record_checkpoint(
			position_manager: H160,
			token_id: U256,
			mut checkpoint: PositionCheckpoint<T::AccountId>,
		) -> DispatchResult {
			let block = checkpoint.proven_through;
			let outcome;
			let previous = PositionCheckpoints::<T>::get(position_manager, token_id);
			if let Some(previous) = previous.as_ref() {
				ensure!(block.number > previous.proven_through.number, Error::<T>::StaleCheckpoint);
				ensure!(
					block.timestamp_millis > previous.proven_through.timestamp_millis,
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

					checkpoint.maintained_since = previous.maintained_since;
					outcome = CheckpointOutcome::Maintained { from: previous.proven_through };
				} else {
					outcome = CheckpointOutcome::Reset {
						previous_revision: previous.revision,
						unproven_from: previous.proven_through,
					};
				}
			} else {
				outcome = CheckpointOutcome::Registered;
			}

			let position_id = StabilizerPoolPositionId { position_manager, token_id };
			let enrollment = match StabilizerPoolRegistrations::<T>::get(position_id) {
				Some(StabilizerPoolRegistration::Enrolled(enrollment)) => Some(enrollment),
				Some(StabilizerPoolRegistration::Removed { reported_at }) => {
					// A proof can re-enroll from the same authenticated post-change block.
					ensure!(block.number >= reported_at.number, Error::<T>::StaleCheckpoint);
					if block.number == reported_at.number {
						ensure!(block == reported_at, Error::<T>::StaleCheckpoint);
					}
					ensure!(
						block.timestamp_millis >= reported_at.timestamp_millis,
						Error::<T>::StaleCheckpoint
					);
					None
				},
				None => None,
			};
			let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
			match (enrollment, &outcome) {
				(Some(mut enrollment), CheckpointOutcome::Maintained { .. }) => {
					let next_claim_frame = Self::submit_position_claim(
						&checkpoint,
						enrollment.next_claim_frame,
						current_frame,
					)?;
					if next_claim_frame != enrollment.next_claim_frame {
						enrollment.next_claim_frame = next_claim_frame;
						StabilizerPoolRegistrations::<T>::insert(
							position_id,
							StabilizerPoolRegistration::Enrolled(enrollment),
						);
					}
				},
				_ => {
					let previous_registration = previous.as_ref().zip(enrollment);
					let eligible_from =
						current_frame.checked_add(1).ok_or(ArithmeticError::Overflow)?;
					let review = if let Some((previous, enrollment)) = previous_registration {
						let review = Self::review_frame_after_position_change(
							current_frame,
							previous,
							enrollment.eligible_from,
						)?;
						Self::submit_position_claim(
							previous,
							enrollment.next_claim_frame,
							current_frame,
						)?;
						review
					} else {
						None
					};

					Self::register_position_liquidity(
						current_frame,
						&checkpoint,
						previous_registration
							.map(|(previous, enrollment)| (previous, enrollment.eligible_from)),
					)?;
					if let Some(review) = review {
						StabilizerPoolReviewFrame::<T>::put(review);
					}
					StabilizerPoolRegistrations::<T>::insert(
						position_id,
						StabilizerPoolRegistration::Enrolled(StabilizerPoolEnrollment {
							eligible_from,
							next_claim_frame: eligible_from,
						}),
					);
				},
			}

			PositionCheckpoints::<T>::insert(position_manager, token_id, &checkpoint);
			Self::deposit_event(Event::PositionCheckpointed {
				position_manager,
				token_id,
				checkpoint: Box::new(checkpoint),
				outcome,
			});
			Ok(())
		}

		/// Submit the proof's eligible frames before advancing the enrollment's claim cursor.
		fn submit_position_claim(
			checkpoint: &PositionCheckpoint<T::AccountId>,
			next_claim_frame: FrameId,
			current_frame: FrameId,
		) -> Result<FrameId, DispatchError> {
			if !T::ClaimProvider::ENABLED {
				return Ok(next_claim_frame);
			}
			let (next_claim_frame, frames) =
				Self::value_position_claim(checkpoint, next_claim_frame, current_frame)?;
			if !frames.is_empty() {
				T::ClaimProvider::submit_claim(&checkpoint.beneficiary, &frames)?;
			}
			Ok(next_claim_frame)
		}

		/// Value the complete frames covered by an authenticated checkpoint. A proof accepted
		/// before frame close can cover a frame that closed since.
		pub(crate) fn value_position_claim(
			checkpoint: &PositionCheckpoint<T::AccountId>,
			next_claim_frame: FrameId,
			current_frame: FrameId,
		) -> Result<(FrameId, Vec<StabilizerPoolFrameClaim>), DispatchError> {
			let oldest_claimable_frame =
				current_frame.saturating_sub(T::ClaimWindowFrames::get().into());
			let first_frame = next_claim_frame.max(oldest_claimable_frame);
			if first_frame >= current_frame {
				return Ok((next_claim_frame, Vec::new()));
			}

			let config = Pools::<T>::get(checkpoint.pool_manager, checkpoint.pool_id)
				.ok_or(Error::<T>::PoolNotRegistered)?;
			let frame_starts = StabilizerPoolFrameStartMillis::<T>::get();
			let prices = StabilizerPoolPricesByFrame::<T>::get();
			let mut next_unclaimed = first_frame;
			let mut claims = Vec::new();

			for frame in first_frame..current_frame {
				let next_frame = frame.saturating_add(1);
				let (Some(start), Some(end)) =
					(frame_starts.get(&frame), frame_starts.get(&next_frame))
				else {
					// Missing boundaries cannot be recovered by a newer Ethereum proof.
					next_unclaimed = next_frame;
					continue;
				};
				if *end > checkpoint.proven_through.timestamp_millis {
					break;
				}
				next_unclaimed = next_frame;
				if *start < checkpoint.maintained_since.timestamp_millis {
					continue;
				}
				let Some(price) = prices.get(&frame) else { continue };
				if checkpoint.pool_manager != price.pool.pool_manager {
					continue;
				}
				if checkpoint.pool_id != price.pool.pool_id {
					continue;
				}
				let Ok(band) = Self::reward_band(price, config.key.tick_spacing) else { continue };
				let amounts = band
					.principal_for_range(
						checkpoint.liquidity.into(),
						checkpoint.tick_lower,
						checkpoint.tick_upper,
						price.sqrt_price_x96,
					)
					.map_err(|_| Error::<T>::ClaimValuationFailed)?;

				// USDC base units and microgons share six decimals. A uint256 amount times
				// the uint128 USD quote fits in 384 bits, and the band validated the divisor.
				let usdc_in_microgons = U256::try_from(
					U512::from(amounts.usdc) * U512::from(price.usdc_usd_price.into_inner()) /
						U512::from(price.argon_usd_target_price.into_inner()),
				)
				.map_err(|_| ArithmeticError::Overflow)?;
				if amounts.argon.is_zero() && usdc_in_microgons.is_zero() {
					continue;
				}
				claims.push(StabilizerPoolFrameClaim {
					frame_id: frame,
					argon_microgons: amounts.argon,
					usdc_in_microgons,
				});
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
			let range = StabilizerPoolLiquidityRange::from(previous);
			let mut liquidity = RegisteredLiquidityByRange::<T>::get(range);
			liquidity.advance_to(frame_id);
			liquidity
				.remove_registration(previous.liquidity, enrollment.eligible_from <= frame_id)?;
			let review = Self::review_frame_after_position_change(
				frame_id,
				previous,
				enrollment.eligible_from,
			)?;
			Self::submit_position_claim(previous, enrollment.next_claim_frame, frame_id)?;

			Self::store_registered_liquidity(range, liquidity);
			if let Some(review) = review {
				StabilizerPoolReviewFrame::<T>::put(review);
			}
			StabilizerPoolRegistrations::<T>::insert(
				position,
				StabilizerPoolRegistration::Removed { reported_at: observed_at },
			);
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

			FinalizedStabilizerPoolPrincipalByFrame::<T>::mutate(|history| {
				history.retain(|frame, _| *frame >= oldest_frame);
				if let Some(review) = StabilizerPoolReviewFrame::<T>::get()
					.filter(|review| review.frame_id >= oldest_frame)
				{
					let principal = Self::finalize_frame_principal(&review).unwrap_or_default();
					// The review frame precedes the just-closed frame, within the history window.
					let _ = history.try_insert(review.frame_id, principal);
				}
			});

			let Some(frame_id) = closed_frame else {
				StabilizerPoolReviewFrame::<T>::kill();
				return;
			};
			let liquidity_by_range = closed_price
				.and_then(|price| Self::snapshot_registered_liquidity(frame_id, &price).ok())
				.unwrap_or_default();
			StabilizerPoolReviewFrame::<T>::put(StabilizerPoolFrameLiquidity {
				frame_id,
				liquidity_by_range,
			});
		}

		/// Compute the replacement snapshot before writing the registration change.
		fn review_frame_after_position_change(
			current_frame: FrameId,
			previous: &PositionCheckpoint<T::AccountId>,
			eligible_from: FrameId,
		) -> Result<Option<StabilizerPoolFrameLiquidity>, DispatchError> {
			let Some(mut review) = StabilizerPoolReviewFrame::<T>::get() else { return Ok(None) };
			if review.frame_id.checked_add(1) != Some(current_frame) {
				return Ok(None);
			}
			if review.frame_id < eligible_from {
				return Ok(None);
			}

			let ticks = (previous.tick_lower, previous.tick_upper);
			let Some(liquidity) = review.liquidity_by_range.get_mut(&ticks) else {
				return Ok(None)
			};

			let prices = StabilizerPoolPricesByFrame::<T>::get();
			let Some(price) = prices.get(&review.frame_id) else { return Ok(None) };
			if previous.pool_manager != price.pool.pool_manager {
				return Ok(None);
			}
			if previous.pool_id != price.pool.pool_id {
				return Ok(None);
			}

			let frame_starts = StabilizerPoolFrameStartMillis::<T>::get();
			let began_at = frame_starts
				.get(&review.frame_id)
				.ok_or(Error::<T>::FrameStartTimeUnavailable)?;
			let ended_at =
				frame_starts.get(&current_frame).ok_or(Error::<T>::FrameStartTimeUnavailable)?;
			if previous.maintained_since.timestamp_millis <= *began_at &&
				previous.proven_through.timestamp_millis >= *ended_at
			{
				// Only a proof spanning the whole frame protects this contribution.
				return Ok(None);
			}

			*liquidity = liquidity
				.checked_sub(previous.liquidity.into())
				.ok_or(ArithmeticError::Underflow)?;
			if liquidity.is_zero() {
				review.liquidity_by_range.remove(&ticks);
			}
			Ok(Some(review))
		}

		fn snapshot_registered_liquidity(
			frame_id: FrameId,
			price: &StabilizerPoolFramePrice,
		) -> Result<LiquidityByUniswapRange, PositionAmountError> {
			let Some(pool) = Pools::<T>::get(price.pool.pool_manager, price.pool.pool_id) else {
				return Ok(LiquidityByUniswapRange::new());
			};
			let band = Self::reward_band(price, pool.key.tick_spacing)?;
			let mut snapshot = LiquidityByUniswapRange::new();
			for (lower_uniswap_tick, upper_uniswap_tick) in band.reward_ranges()? {
				let range = StabilizerPoolLiquidityRange {
					pool_manager: price.pool.pool_manager,
					pool_id: price.pool.pool_id,
					lower_uniswap_tick,
					upper_uniswap_tick,
				};
				let liquidity = RegisteredLiquidityByRange::<T>::get(range)
					.liquidity_for_frame(frame_id)
					.unwrap_or_default();
				if !liquidity.is_zero() {
					snapshot
						.try_insert((lower_uniswap_tick, upper_uniswap_tick), liquidity)
						.map_err(|_| PositionAmountError::InvalidRewardPriceBand)?;
				}
			}
			Ok(snapshot)
		}

		fn finalize_frame_principal(
			review: &StabilizerPoolFrameLiquidity,
		) -> Result<PositionAmounts<U256>, PositionAmountError> {
			let mut total = PositionAmounts::default();
			if review.liquidity_by_range.is_empty() {
				return Ok(total);
			}

			let prices = StabilizerPoolPricesByFrame::<T>::get();
			let price = prices.get(&review.frame_id).ok_or(PositionAmountError::InvalidPrice)?;
			let pool = Pools::<T>::get(price.pool.pool_manager, price.pool.pool_id)
				.ok_or(PositionAmountError::InvalidPrice)?;
			let band = Self::reward_band(price, pool.key.tick_spacing)?;
			for (&(lower, upper), &liquidity) in &review.liquidity_by_range {
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
			Ok(total)
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

		fn register_position_liquidity(
			frame_id: FrameId,
			checkpoint: &PositionCheckpoint<T::AccountId>,
			previous_registration: Option<(&PositionCheckpoint<T::AccountId>, FrameId)>,
		) -> Result<(), ArithmeticError> {
			let range = StabilizerPoolLiquidityRange::from(checkpoint);
			let mut liquidity = RegisteredLiquidityByRange::<T>::get(range);
			liquidity.advance_to(frame_id);

			let mut previous_range_update = None;
			if let Some((previous, eligible_from)) = previous_registration {
				let previous_range = StabilizerPoolLiquidityRange::from(previous);
				let was_eligible = eligible_from <= frame_id;
				if previous_range == range {
					liquidity.remove_registration(previous.liquidity, was_eligible)?;
				} else {
					let mut previous_liquidity =
						RegisteredLiquidityByRange::<T>::get(previous_range);
					previous_liquidity.advance_to(frame_id);
					previous_liquidity.remove_registration(previous.liquidity, was_eligible)?;
					previous_range_update = Some((previous_range, previous_liquidity));
				}
			}

			liquidity.next_frame_liquidity = liquidity
				.next_frame_liquidity
				.checked_add(U256::from(checkpoint.liquidity))
				.ok_or(ArithmeticError::Overflow)?;

			// Both ranges are ready before any write. A failed total leaves the checkpoint intact.
			if let Some((previous_range, previous_liquidity)) = previous_range_update {
				Self::store_registered_liquidity(previous_range, previous_liquidity);
			}
			Self::store_registered_liquidity(range, liquidity);
			Ok(())
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
}

type LiquidityByUniswapRange =
	BoundedBTreeMap<(UniswapTick, UniswapTick), U256, ConstU32<MAX_REWARD_RANGES>>;

/// Liquidity totals for one closed frame while public reports can still correct them.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub struct StabilizerPoolFrameLiquidity {
	/// Earning frame under review.
	pub frame_id: FrameId,
	/// Sum of raw Uniswap liquidity by (lower price tick, upper price tick).
	pub liquidity_by_range: LiquidityByUniswapRange,
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
		/// Ethereum block proving the change. A replacement cannot use an earlier block.
		reported_at: ProofBlock,
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
	pub eligible_from: FrameId,
	/// Earliest frame this enrollment can still submit a claim for.
	pub next_claim_frame: FrameId,
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
	pub token_id: U256,
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
	pub maintained_since: ProofBlock,
	/// Latest accepted Ethereum endpoint; proves nothing about later blocks.
	pub proven_through: ProofBlock,
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
	Eq,
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
	/// Raw liquidity eligible in `frame_id`.
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

	fn remove_registration(
		&mut self,
		liquidity: u128,
		was_eligible: bool,
	) -> Result<(), ArithmeticError> {
		let liquidity = U256::from(liquidity);
		if was_eligible {
			self.current_liquidity = self
				.current_liquidity
				.checked_sub(liquidity)
				.ok_or(ArithmeticError::Underflow)?;
		}
		self.next_frame_liquidity = self
			.next_frame_liquidity
			.checked_sub(liquidity)
			.ok_or(ArithmeticError::Underflow)?;
		Ok(())
	}
}
