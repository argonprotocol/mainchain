#![cfg_attr(not(feature = "std"), no_std)]
//! Claimant-submitted checkpoints for maintained Uniswap v4 positions.
//!
//! Checkpoints establish continuity and maintain registered liquidity by Uniswap range.

extern crate alloc;

use alloc::boxed::Box;
use argon_primitives::{
	block_seal::FrameId, EthereumAccountStorageProof, EthereumVerifyProvider,
	MiningFrameTransitionProvider, UniswapTick,
};
pub use pallet::*;
use pallet_ethereum_verifier::Pallet as EthereumVerifier;
use pallet_prelude::*;
use sp_runtime::ArithmeticError;
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

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	// FRAME requires this explicit path when checking the inherited system configuration.
	pub trait Config:
		polkadot_sdk::frame_system::Config + pallet_ethereum_verifier::Config
	{
		type WeightInfo: weights::WeightInfo;
		/// Supplies the mining frame active when a checkpoint is accepted.
		type MiningFrameTransitionProvider: MiningFrameTransitionProvider;
		/// Mainchain clock used to assign observations to frames and reject future prices.
		type CurrentTick: Get<Tick>;
		/// Closed mining frames retained in the sampled Uniswap price history.
		#[pallet::constant]
		type MaxSampledPriceHistoryFrames: Get<u32>;
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

	/// First eligible frame for the current revision of each PositionManager NFT. A checkpoint
	/// from before reward registration needs a fresh proof to enter the range totals.
	#[pallet::storage]
	pub type EligibleFromFrameByPosition<T> =
		StorageMap<_, Blake2_128Concat, StabilizerPoolPositionId, FrameId, OptionQuery>;

	/// Sum of registered raw Uniswap liquidity at each exact pool and range.
	#[pallet::storage]
	pub type RegisteredLiquidityByRange<T> = StorageMap<
		_,
		Blake2_128Concat,
		StabilizerPoolLiquidityRange,
		StabilizerPoolRangeLiquidity,
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

		/// Register a baseline, extend a maintained revision, or reset after an unproven change.
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

			let header = EthereumVerifier::<T>::verified_execution_header(execution_proof)
				.map_err(|_| Error::<T>::InvalidProof)?;
			EthereumVerifier::<T>::verify_account_storage_proof(
				config.subscriber,
				&proof.subscriber,
			)
			.map_err(|_| Error::<T>::InvalidProof)?;
			EthereumVerifier::<T>::verify_account_storage_proof(
				config.position_manager,
				&proof.position_manager,
			)
			.map_err(|_| Error::<T>::InvalidProof)?;
			EthereumVerifier::<T>::verify_account_storage_proof(pool_manager, &proof.pool_manager)
				.map_err(|_| Error::<T>::InvalidProof)?;

			let block = ProofBlock {
				hash: header.block_hash,
				number: header.block_number,
				timestamp_millis: header.timestamp_millis,
			};
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
			let eligible_from = EligibleFromFrameByPosition::<T>::get(position_id);
			if eligible_from.is_none() || !matches!(outcome, CheckpointOutcome::Maintained { .. }) {
				let previous_registration = previous.as_ref().zip(eligible_from);
				let eligible_from =
					Self::register_position_liquidity(&checkpoint, previous_registration)?;
				EligibleFromFrameByPosition::<T>::insert(position_id, eligible_from);
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

		fn register_position_liquidity(
			checkpoint: &PositionCheckpoint<T::AccountId>,
			previous_registration: Option<(&PositionCheckpoint<T::AccountId>, FrameId)>,
		) -> Result<FrameId, ArithmeticError> {
			// A transition block still dispatches in the closing frame. MiningSlot activates the
			// next frame during finalization, after these extrinsics have run.
			let frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
			let eligible_from = frame_id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
			let range = StabilizerPoolLiquidityRange::from(checkpoint);
			let mut liquidity = RegisteredLiquidityByRange::<T>::get(range);
			liquidity.advance_to(frame_id);

			let mut previous_range_update = None;
			if let Some((previous, previous_eligible_from)) = previous_registration {
				let previous_range = StabilizerPoolLiquidityRange::from(previous);
				let was_eligible = previous_eligible_from <= frame_id;
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
			Ok(eligible_from)
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
pub struct ProofBlock {
	/// Authenticated Ethereum execution-block hash, not an Argon block hash.
	pub hash: H256,
	/// Ethereum execution-block height used for the position's replay watermark.
	pub number: u64,
	/// Ethereum block time in Unix milliseconds, authenticated by the execution header.
	pub timestamp_millis: u64,
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
